#![feature(const_trait_impl)]

use std::{collections::HashMap, path::PathBuf, sync::Arc};

use chrono::Utc;
use rustyline::{history::FileHistory, Editor};
use serenity::{
    async_trait, client,
    model::prelude::{
        command::Command,
        interaction::{Interaction, InteractionResponseType},
        Ready, UserId,
    },
    prelude::*,
    Client,
};
use structopt::{clap::AppSettings::*, StructOpt};
use tokio::fs;


mod error;
pub use error::*;

mod commands;
mod config;
mod database;
mod paths;

const NAME: &str = "Tara";
const DESCRIPTION: &str = "A modern self-hostable Discord bot.";

#[derive(StructOpt, Debug, Clone)]
#[structopt(name = NAME, about = DESCRIPTION)]
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
    },
}

#[derive(StructOpt, Debug, Clone)]
enum SubOptionConfig {
    /// Create configuration files with a user-provided configuration.
    Init,
}

#[tokio::main]
async fn main() -> std::result::Result<(), anyhow::Error> {
    match Options::from_args() {
        Options::Daemon { config } => daemon(config).await?,
        Options::Config(option) => {
            match option {
                SubOptionConfig::Init => init().await?,
            }
        }
    }

    Ok(())
}

async fn daemon(config_path: Option<PathBuf>) -> Result<()> {
    // Setup logging
    env_logger::init();
    log::info!("Initialized Logging");

    // Get the configuration file path and read the configuration from it.
    let config_path = match config_path {
        Some(x) => x,
        None => paths::config_file_path()?,
    };
    let config = Arc::new(config::Configuration::from_toml(&config_path).await?);
    log::info!("Loaded configuration from \"{}\"", config_path.display());

    // Get error messsages
    let error_messages = load_error_messages(config.clone());

    // Setup intents
    let intents =
        GatewayIntents::GUILD_MESSAGES | GatewayIntents::DIRECT_MESSAGES | GatewayIntents::MESSAGE_CONTENT;

    // Initialize && start client
    let mut client = Client::builder(config.secrets.token.clone(), intents)
        .event_handler(EventHandler {
            databases: database::Databases::open().await?,
            config,
            error_messages: error_messages.await,
            dm_cooldown_counter: Arc::new(Mutex::new(HashMap::new())),
        })
        .await
        .map_err(Error::ClientInitialization)?;

    if let Err(why) = client.start().await {
        log::error!("Error: {:?}", why);
    }

    Ok(())
}

async fn init() -> Result<()> {
    use rustyline::DefaultEditor;

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
    let mut rl = DefaultEditor::new().unwrap();

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
    let direct_message_cooldown = {
        let direct_message_cooldown = get_optional_value(
            &mut rl,
            "Enter cooldown, in seconds, for direct message commands [Optional]: ",
        )?;
        match direct_message_cooldown {
            Some(x) => {
                Some(std::time::Duration::from_secs(
                    x.parse::<u64>()
                        .map_err(|e| Error::ParseNumber(format!("\"{x}\": {e}")))?,
                ))
            }
            None => None,
        }
    };
    let random_error_message = {
        let random_error_message = get_optional_value(
            &mut rl,
            "Enter path to randomErrorMessage file (Type \"default\" to use the default path) [Optional]: ",
        )?;
        random_error_message.map_or(config::ConfigurationRandomErrorMessages::Boolean(false), |x| {
            match &*x {
                "default" => config::ConfigurationRandomErrorMessages::Boolean(true),
                _ => config::ConfigurationRandomErrorMessages::Path(PathBuf::from(x)),
            }
        })
    };

    let config_file_path = {
        let config_file_path = get_optional_value(
            &mut rl,
            "Enter where to save generated config file (Press Enter to use default) [Optional]: ",
        )?;
        match config_file_path {
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
        }
    };

    let config = config::Configuration {
        secrets: config::ConfigurationSecrets {
            token:            token.clone(),
            currency_api_key: currency_api_key.clone(),
        },
        direct_message_cooldown,
        random_error_message: random_error_message.clone(),
    };

    let config = toml::to_string_pretty(&config).map_err(|e| {
        Error::ConfigurationSave {
            error: e,
            path:  config_file_path.clone(),
        }
    })?;

    println!(
        "Selected Configuration:\n\ttoken = '{token}' \n\tcurrencyApiKey = {currency_api_key:?} \
         \n\tdirectMessageCooldown = {direct_message_cooldown:?} \n\trandomErrorMessage = \
         {random_error_message:?}"
    );

    // If we should continue, save, otherwise we exit.
    if get_optional_value(&mut rl, "Is this okay? [y/N]: ")?.map_or(false, |mut x| {
        x = x.to_lowercase();
        x == "y" || x == "yes"
    }) {
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
    config:              Arc<config::Configuration>,
    error_messages:      Arc<config::ErrorMessages>,
    databases:           Arc<database::Databases>,
    dm_cooldown_counter: Arc<Mutex<HashMap<UserId, chrono::DateTime<Utc>>>>,
}


#[async_trait]
impl client::EventHandler for EventHandler {
    async fn interaction_create(&self, context: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(command) = interaction {
            // Assume we're in a DM
            if command.guild_id.is_none() {
                use chrono::{Duration, Utc};
                let uid = command.user.id;
                let now = Utc::now();
                let mut counter = self.dm_cooldown_counter.lock().await;

                // If cooldown counter contains this User ID
                if let Some(end) = counter.get(&uid) {
                    // If the ending time is in the future
                    if now < *end {
                        // Report error & return
                        if let Err(e) = command
                            .create_interaction_response(&context.http, |response| {
                                response
                                    .kind(InteractionResponseType::ChannelMessageWithSource)
                                    .interaction_response_data(|message| {
                                        message.ephemeral(true);
                                        message.content(Error::DirectMessageCooldown(*end - now).to_string())
                                    })
                            })
                            .await
                        {
                            log::error!("Couldn't respond to command: {e}");
                        }
                        return;
                    }
                    else {
                        // Remove the cooldown entry from the hashmap
                        let _ = counter.remove(&uid);
                    }
                }
                else if let Some(cooldown_len) = self.config.direct_message_cooldown {
                    let cooldown_len = Duration::from_std(cooldown_len).unwrap();
                    // Calculate the ending time and add it to the counter.
                    counter.insert(uid, now + cooldown_len);
                }
            }

            commands::run_command(
                context,
                command,
                self.config.clone(),
                self.databases.clone(),
                self.error_messages.clone(),
            )
            .await;
        }
    }

    async fn ready(&self, context: Context, ready: Ready) {
        log::info!("{} is connected!", ready.user.name);
        log::info!("Registering commands...");
        Command::set_global_application_commands(&context.http, |commands| {
            // For each command, register the command.
            for command in commands::COMMANDS.iter() {
                commands.create_application_command(|cmd| command.register(cmd));
            }
            commands
        })
        .await
        .expect("Unable to register commands.");
        log::info!("Commands registered.");

        // On startup we check, for each guild we're in, if the guild if present in the
        // database. If it's not, we add it with the default configuration.
        let guilds = ready.guilds.iter().map(|x| x.id);
        for guild_id in guilds {
            match self.databases.contains("guilds", guild_id) {
                Ok(x) => {
                    if !x {
                        if let Err(e) = self.databases.guilds_insert_default(guild_id) {
                            log::error!("Couldn't add guild to database (\"{guild_id}\"): {e}");
                        }
                        else {
                            log::info!("Added guild to database (\"{guild_id}\")");
                        }
                    }
                }
                Err(e) => {
                    log::error!("Couldn't add guild to database (\"{guild_id}\"): {e}");
                }
            }
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
