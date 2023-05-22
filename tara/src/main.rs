#![feature(const_trait_impl)]

use std::{path::PathBuf, str::FromStr, sync::Arc};

use once_cell::sync::Lazy;
use rustyline::{history::FileHistory, Editor};
use serenity::{
    all::{Command, Guild, Interaction, Ready},
    async_trait, client,
    gateway::ActivityData,
    prelude::*,
    Client,
};
use structopt::{
    clap::AppSettings::{ColorAuto, ColoredHelp, VersionlessSubcommands},
    StructOpt,
};
use tara::{
    commands, config,
    database::{self, GuildPreferences},
    error::{Error, Result},
    logging, paths,
};
use tokio::{fs, task};
use tracing::{debug, error, info, metadata::LevelFilter};
use tracing_subscriber::{prelude::*, util::SubscriberInitExt, EnvFilter, Layer};

const NAME: &str = "Tara";

static COMMAND_LOG_PATH: Lazy<PathBuf> = Lazy::new(|| {
    paths::project_dir()
        .unwrap()
        .data_dir()
        .join(format!("command-log_{}.csv", chrono::Utc::now()))
});

#[derive(StructOpt, Debug, Clone)]
#[structopt(name = NAME, about, author)]
#[structopt(
    global_setting(ColorAuto),
    global_setting(ColoredHelp),
    global_setting(VersionlessSubcommands)
)]
enum Options {
    /// Manage Tara's configuration
    Config(SubOptionConfig),

    /// Start Tara.
    Daemon {
        #[structopt(long)]
        /// Specify a configuration file to use instead of the default.
        config: Option<PathBuf>,

        #[structopt(short, long, name = "LOGLEVEL")]
        log_level: Option<LogLevel>,
    },
}

#[derive(StructOpt, Debug, Clone)]
enum SubOptionConfig {
    /// Create configuration files with a user-provided configuration.
    Init,
}

#[derive(Debug, Clone, Copy, Default)]
enum LogLevel {
    Error,
    Warn,
    #[default]
    Info,
    Debug,
    Trace,
    Off,
}

impl FromStr for LogLevel {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match &*s.to_lowercase() {
            "error" => Ok(Self::Error),
            "warn" => Ok(Self::Warn),
            "info" => Ok(Self::Info),
            "debug" => Ok(Self::Debug),
            "trace" => Ok(Self::Trace),
            "off" => Ok(Self::Off),
            _ => {
                Err(format!(
                    "\"{s}\" isn't a LogLevel variant. They are as follows: Error, Warn, Info, Debug, \
                     Trace, Off"
                ))
            }
        }
    }
}

impl From<LogLevel> for LevelFilter {
    fn from(value: LogLevel) -> Self {
        match value {
            LogLevel::Error => Self::ERROR,
            LogLevel::Warn => Self::WARN,
            LogLevel::Info => Self::INFO,
            LogLevel::Debug => Self::DEBUG,
            LogLevel::Trace => Self::TRACE,
            LogLevel::Off => Self::OFF,
        }
    }
}

#[tokio::main]
async fn main() -> std::result::Result<(), anyhow::Error> {
    match Options::from_args() {
        Options::Daemon { config, log_level } => daemon(config, log_level).await?,
        Options::Config(option) => {
            match option {
                SubOptionConfig::Init => init().await?,
            }
        }
    }

    Ok(())
}

async fn daemon(
    config_path: Option<PathBuf>,
    log_level: Option<LogLevel>,
) -> std::result::Result<(), anyhow::Error> {
    // Setup logging
    let log_level: LevelFilter = log_level.unwrap_or_default().into();
    let filter = EnvFilter::builder()
        .with_default_directive(log_level.into())
        .parse("")?;
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_filter(filter))
        .init();
    info!("Initialized Logging");

    // Get the configuration file path and read the configuration from it.
    let config_path = match config_path {
        Some(x) => x,
        None => paths::config_file_path()?,
    };
    let config = Arc::new(config::Configuration::from_toml(&config_path).await?);
    debug!("Loaded configuration from \"{}\"", config_path.display());

    // Get error messsages
    let error_messages = load_error_messages(config.clone());

    // Setup intents
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILDS;

    let guilds = match database::Guilds::load().await {
        Err(_) => database::Guilds::create().await?,
        Ok(x) => x,
    };

    let logger = logging::CommandLogger::new();

    // Initialize && start client
    let mut client = Client::builder(config.secrets.token.clone(), intents)
        .event_handler(EventHandler {
            guilds: guilds.clone(),
            config,
            logger: logger.clone(),
            error_messages: error_messages.await,
        })
        .await
        .map_err(|e| Error::ClientInitialization(Box::new(e)))?;

    task::spawn(async move {
        let logger = logger.clone();
        if let Err(e) = logger.log_to_file(COMMAND_LOG_PATH.as_path()).await {
            error!("LOGGING ERORR: {e}");
        };
    });
    if let Err(why) = client.start().await {
        error!("Error: {:?}", why);
    }

    Ok(())
}

async fn init() -> Result<()> {
    fn get_optional_value(rl: &mut Editor<(), FileHistory>, prompt: &str) -> Result<Option<String>> {
        let value = rl.readline(prompt).map_err(Error::ReadLine)?.trim().to_owned();
        if value.is_empty() {
            Ok(None)
        }
        else {
            Ok(Some(value))
        }
    }

    // Collect all configuration values
    let mut rl = rustyline::DefaultEditor::new().unwrap();

    let token = {
        let mut token = String::new();
        while token.is_empty() {
            token = rl
                .readline("Enter Discord token [Required]: ")
                .map_err(Error::ReadLine)?
                .trim()
                .to_owned();
        }
        token
    };

    let currency_api_key = get_optional_value(&mut rl, "Enter API key for currencyapi.com [Optional]: ")?;
    let direct_message_cooldown = get_optional_value(
        &mut rl,
        "Enter cooldown, in seconds, for direct message commands [Optional]: ",
    )?;
    let direct_message_cooldown = match direct_message_cooldown {
        Some(x) => {
            Some(std::time::Duration::from_secs(
                x.parse::<u64>()
                    .map_err(|e| Error::ParseNumber(format!("\"{x}\": {e}")))?,
            ))
        }
        None => None,
    };

    let random_error_message = get_optional_value(
        &mut rl,
        "Enter path to randomErrorMessage file (Type \"default\" to use the default path) [Optional]: ",
    )?;
    let random_error_message =
        random_error_message.map_or(config::ConfigurationRandomErrorMessages::Boolean(false), |x| {
            if x == "default" {
                config::ConfigurationRandomErrorMessages::Boolean(true)
            }
            else {
                config::ConfigurationRandomErrorMessages::Path(PathBuf::from(x))
            }
        });


    let config_file_path = get_optional_value(
        &mut rl,
        "Enter where to save generated config file (Press Enter to use default) [Optional]: ",
    )?;
    let config_file_path = match config_file_path {
        Some(x) => PathBuf::from(x),
        None => {
            if let Some(project_dirs) = paths::project_dir() {
                project_dirs.config_dir().join("tara.toml")
            }
            else {
                eprintln!("Couldn't get default config file location!");
                return Err(Error::MissingConfigurationFile);
            }
        }
    };


    let config = config::Configuration {
        secrets:              config::ConfigurationSecrets {
            token:            token.clone(),
            currency_api_key: currency_api_key.clone(),
        },
        random_error_message: random_error_message.clone(),
    };

    let config = toml::to_string_pretty(&config).map_err(|e| {
        Error::ConfigurationSave {
            error: Box::new(e),
            path:  config_file_path.clone(),
        }
    })?;

    println!(
        "Selected Configuration:\n\ttoken = '{token}' \n\tcurrencyApiKey = {currency_api_key:?} \
         \n\tdirectMessageCooldown = {direct_message_cooldown:?} \n\trandomErrorMessage = \
         {random_error_message:?}"
    );

    // If we should continue, save, otherwise we exit.
    let cont = get_optional_value(&mut rl, "Is this okay? [y/N]: ")?.map_or(false, |mut x| {
        x = x.to_lowercase();
        x == "y" || x == "yes"
    });
    if cont {
        fs::create_dir_all(&config_file_path.parent().unwrap())
            .await
            .map_err(Error::Io)?;
        fs::write(&config_file_path, config).await.map_err(Error::Io)?;
        println!("Saved config to \"{}\"", config_file_path.display());
    }
    else {
        println!("Quitting...");
    }

    Ok(())
}

struct EventHandler {
    config:         Arc<config::Configuration>,
    error_messages: Arc<config::ErrorMessages>,
    logger:         logging::CommandLogger,
    guilds:         database::Guilds,
}


#[async_trait]
impl client::EventHandler for EventHandler {
    async fn interaction_create(&self, context: Context, interaction: Interaction) {
        if let Interaction::Command(command) = interaction {
            let guild: Option<Guild> = command
                .guild_id
                .and_then(|guild_id| guild_id.to_guild_cached(&context.cache).map(|x| x.to_owned()));

            commands::run_command(
                context,
                command,
                guild,
                self.config.clone(),
                self.guilds.clone(),
                self.error_messages.clone(),
                self.logger.clone(),
            )
            .await;
        }
    }

    async fn ready(&self, context: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);
        context.set_activity(Some(ActivityData::watching("El-Wumbus/Tara on GitHub")));

        info!("Registering commands...");
        // For each command in the map, run `.register()` on it.
        let global_commands = commands::COMMANDS
            .values()
            .map(|command| command.register())
            .collect::<Vec<_>>();

        Command::set_global_commands(&context.http, global_commands)
            .await
            .expect("Unable to register commands.");
        info!("Commands registered.");

        // On startup we check, for each guild we're in, if the guild if present in the
        // database. If it's not, we add it with the default configuration.
        let guilds = ready.guilds.iter().map(|x| x.id);
        for guild_id in guilds {
            if self.guilds.contains(guild_id).await {
                // Insert the guild
                self.guilds.insert(GuildPreferences::default(guild_id)).await;
            }
        }
        if let Err(e) = self.guilds.save().await {
            error!("Couldn't save guild preferences to database: {e}");
        }
    }
}


/// Returns a structure of error message responses from and `error_message` file
/// possibly specified in `config`.
///
/// # Usage
///
/// ```Rust
/// let config = Arc::new(config::Configuration::default());
/// let error_messages = load_error_messages(config.clone());
/// dbg!(error_messages);
/// ```
async fn load_error_messages(config: Arc<config::Configuration>) -> Arc<config::ErrorMessages> {
    Arc::new(match &config.random_error_message {
        config::ConfigurationRandomErrorMessages::Boolean(x) => {
            if *x {
                // Load from the default location, if not possible fall back to the default
                // messages.
                match paths::error_messages_file_path() {
                    Some(file) => config::ErrorMessages::from_json(file).await.unwrap_or_default(),
                    None => config::ErrorMessages::default(),
                }
            }
            else {
                config::ErrorMessages::default()
            }
        }
        config::ConfigurationRandomErrorMessages::Path(path) => {
            config::ErrorMessages::from_json(&path).await.unwrap_or_default()
        }
    })
}
