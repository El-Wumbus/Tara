#![feature(const_trait_impl, stmt_expr_attributes, type_alias_impl_trait, async_closure)]
use std::{num::NonZeroU64, path::PathBuf, str::FromStr, sync::Arc};

use serenity::{all::*, async_trait, client, gateway::ActivityData, prelude::Context, Client};
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use structopt::{
    clap::AppSettings::{ColorAuto, ColoredHelp, VersionlessSubcommands},
    StructOpt,
};
use tara_util::{ipc as ipcutil, logging as logutil, paths};
use tokio::task;
use tracing::{debug, error, info, metadata::LevelFilter};
use tracing_subscriber::{prelude::*, util::SubscriberInitExt, EnvFilter, Layer};

mod error;
pub use error::{Error, Result};

use crate::ipc::ActionReceiver;
mod commands;
mod componet;
mod config;
mod defaults;
mod ipc;
mod logging;

const NAME: &str = "Tara";
const REPO_URL: &str = env!("CARGO_PKG_REPOSITORY");

/// Discord gateway intents
const INTENTS: GatewayIntents = GatewayIntents::GUILD_MESSAGES
    .union(GatewayIntents::non_privileged())
    .union(GatewayIntents::DIRECT_MESSAGES)
    .union(GatewayIntents::MESSAGE_CONTENT)
    .union(GatewayIntents::GUILDS)
    .union(GatewayIntents::GUILD_VOICE_STATES);

#[derive(StructOpt, Debug, Clone)]
#[structopt(name = NAME, about, author)]
#[structopt(
    global_setting(ColorAuto),
    global_setting(ColoredHelp),
    global_setting(VersionlessSubcommands)
)]
struct Options {
    #[structopt(long)]
    /// Specify a configuration file to use instead of the default.
    config: Option<PathBuf>,

    #[structopt(short, long, name = "LOGLEVEL")]
    log_level: Option<LogLevel>,
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

#[cfg(feature = "music")]
pub use reqwest::Client as HttpClient;

#[cfg(feature = "music")]
/// Used to insert a [`reqwest::Client`] into the [`serenity::prelude::Context`].
pub struct HttpKey;

#[cfg(feature = "music")]
impl serenity::prelude::TypeMapKey for HttpKey {
    type Value = HttpClient;
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> anyhow::Result<()> {
    let Options { config, log_level } = Options::from_args();

    // Setup logging
    let log_level: LevelFilter = log_level.unwrap_or_default().into();
    let filter = EnvFilter::builder()
        .with_default_directive(log_level.into())
        .parse("")?;
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_filter(filter))
        .init();

    // Get the configuration file path and read the configuration from it.
    let config_path = match config {
        Some(x) => x,
        None => {
            paths::TARA_CONFIGURATION_FILE
                .clone()
                .ok_or(Error::MissingConfigurationFile)?
        }
    };
    let config = Arc::new(config::Configuration::parse(&config_path).await?);
    debug!("Loaded configuration from \"{}\"", config_path.display());

    let database = PgPoolOptions::new()
        .connect("postgres://postgres@localhost/TaraTest")
        .await?;
    if let Err(e) = sqlx::migrate!("./migrations").run(&database).await {
        error!("Couldn't run database migrations: {e}");
    }

    let logger = logutil::CommandLogger::new();
    task::spawn({
        let logger = logger.clone();
        async move {
            if let Err(e) = logger.log_to_file(paths::TARA_COMMAND_LOG_PATH.as_path()).await {
                error!("LOGGING: {e}");
            };
        }
    });
    info!("Initialized command logger");

    let receiver = Arc::new(ActionReceiver {});
    task::spawn(async move {
        let receiver = receiver.clone();
        if let Err(e) = ipcutil::start_server(receiver.as_ref()).await {
            error!("IPC: {e}");
        };
    });
    info!("Initialized IPC server");

    let mut client = build_client(
        config.secrets.token.clone(),
        EventHandler {
            config: config.clone(),
            logger: logger.clone(),
            error_messages: load_error_messages(config).await,
            component_map: componet::ComponentMap::new(),
            database,
        },
    )
    .await?;

    let _ = client.start().await.map_err(|why| error!("Error: {:?}", why));

    Ok(())
}

async fn build_client(
    token: impl AsRef<str>,
    event_handler: EventHandler,
) -> std::result::Result<Client, anyhow::Error> {
    #[cfg(feature = "music")]
    use songbird::SerenityInit;

    // Initialize && start client
    let client_builder = Client::builder(token, INTENTS).event_handler(event_handler);

    #[cfg(feature = "music")]
    let client = client_builder
        .register_songbird()
        .type_map_insert::<HttpKey>(HttpClient::new())
        .await
        .map_err(|e| Error::ClientInitialization(Box::new(e)))?;

    #[cfg(not(feature = "music"))]
    let client = client_builder
        .await
        .map_err(|e| Error::ClientInitialization(Box::new(e)))?;

    Ok(client)
}

struct EventHandler {
    config:         Arc<config::Configuration>,
    error_messages: Arc<config::ErrorMessages>,
    database:       Pool<Postgres>,
    logger:         logutil::CommandLogger,
    component_map:  componet::ComponentMap,
}

#[async_trait]
impl client::EventHandler for EventHandler {
    async fn interaction_create(&self, context: Context, interaction: Interaction) {
        match interaction {
            Interaction::Component(component) => {
                let guild: Option<Guild> = component
                    .guild_id
                    .and_then(|guild_id| guild_id.to_guild_cached(&context.cache).map(|x| x.to_owned()));

                let args = commands::CommandArguments {
                    context: Arc::new(context),
                    guild,
                    config: self.config.clone(),
                    component_map: self.component_map.clone(),
                    database: self.database.clone(),
                };

                let id = component.data.custom_id.clone();
                if let Some(Err(e)) = self.component_map.run(&id, (component, args)).await {
                    error!("Error running component handler: {e}");
                };
            }
            Interaction::Command(command) => {
                let guild: Option<Guild> = command
                    .guild_id
                    .and_then(|guild_id| guild_id.to_guild_cached(&context.cache).map(|x| x.to_owned()));

                commands::run_command(
                    context,
                    command,
                    guild,
                    self.config.clone(),
                    self.error_messages.clone(),
                    self.logger.clone(),
                    self.component_map.clone(),
                    self.database.clone(),
                )
                .await;
            }
            _ => (),
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
            let guild_name = context.cache.guild(guild_id).map(|x| x.name.clone());
            let insert = if let Some(guild_name) = guild_name {
                sqlx::query!(
                    "INSERT INTO guilds (id, name) VALUES ($1, $2)
                    ON CONFLICT DO NOTHING",
                    guild_id.toint(),
                    guild_name
                )
                .execute(&self.database)
                .await
            } else {
                sqlx::query!(
                    "INSERT INTO guilds (id) VALUES ($1)
                    ON CONFLICT DO NOTHING",
                    guild_id.0.toint()
                )
                .execute(&self.database)
                .await
            };

            if let Err(e) = insert {
                error!("DATABASE: {e}");
            };
        }


        let component_map = self.component_map.clone();
        let http = context.http.clone();
        let cache = context.cache.clone();
        task::spawn(async move {
            if let Err(e) = component_map.timeout_watcher(http, cache).await {
                error!("{e}");
            }
        });
    }
}

trait IdUtil: Copy {
    fn touint(self) -> u64;
    fn toint(self) -> i64;
}

impl IdUtil for NonZeroU64 {
    #[inline]
    fn touint(self) -> u64 { u64::from(self) }

    #[inline]
    fn toint(self) -> i64 { self.touint() as i64 }
}

macro_rules! impl_id_trait {
    ($($t: ident), *) => {
        $(impl IdUtil for $t {
            #[inline]
            fn touint(self) -> u64 { self.0.touint() }

            #[inline]
            fn toint(self) -> i64 { self.0.toint() }
        })*

    };
}

impl_id_trait!(GuildId, RoleId, ChannelId);

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
                match paths::ERROR_MESSAGES_FILE.as_ref() {
                    Some(file) => config::ErrorMessages::from_json(file).await.unwrap_or_default(),
                    None => config::ErrorMessages::default(),
                }
            } else {
                config::ErrorMessages::default()
            }
        }
        config::ConfigurationRandomErrorMessages::Path(path) => {
            config::ErrorMessages::from_json(&path).await.unwrap_or_default()
        }
    })
}
