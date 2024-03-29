#![feature(stmt_expr_attributes, type_alias_impl_trait, result_flattening, let_chains)]
use std::{num::NonZeroU64, path::PathBuf, str::FromStr, sync::Arc};

use anyhow::Context as AnyhowContextWtfRust;
use serenity::{all::*, async_trait, client, gateway::ActivityData, prelude::Context, Client};
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use structopt::{
    clap::AppSettings::{ColorAuto, ColoredHelp, VersionlessSubcommands},
    StructOpt,
};
use tara_util::{ipc as ipcutil, logging as logutil, paths};
use tokio::task;
use tracing::{error, info, Level};
use tracing_subscriber::{filter, prelude::*, Layer};

mod error;
pub use error::{Error, Result};

use crate::ipc::ActionReceiver;
mod commands;
mod componet;
mod config;
mod defaults;
mod ipc;
#[cfg(feature = "ai")]
mod llm;
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
            "e" | "error" => Ok(Self::Error),
            "w" | "warn" => Ok(Self::Warn),
            "i" | "info" => Ok(Self::Info),
            "d" | "debug" => Ok(Self::Debug),
            "t" | "trace" => Ok(Self::Trace),
            "o" | "off" => Ok(Self::Off),
            _ => {
                Err(format!(
                    "\"{s}\" isn't a LogLevel variant. They are as follows: Error, Warn, Info, Debug, \
                     Trace, Off"
                ))
            }
        }
    }
}

impl From<LogLevel> for filter::LevelFilter {
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
    let log_level = log_level.unwrap_or(LogLevel::Info);
    let stdout = tracing_subscriber::fmt::layer()
        .pretty()
        .with_file(false)
        .with_line_number(false);
    tracing_subscriber::registry()
        .with(
            stdout
                .with_filter(filter::LevelFilter::from(log_level))
                .with_filter(filter::filter_fn(|metadata| {
                    metadata.target().starts_with("tara") || {
                        let level: Level = *metadata.level();
                        (level == Level::WARN || level == Level::ERROR)
                            && metadata.target().starts_with("serenity")
                    }
                })),
        )
        .init();

    tokio::task::spawn_blocking(|| {
        match dotenvy::dotenv() {
            // This is stupid.
            Ok(_) => {}
            Err(dotenvy::Error::Io(e)) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e).context("Failed reading .env file"),
        }
        anyhow::Ok(())
    })
    .await??;
    let config = Arc::new(config::Configuration::parse(config).await?);

    let postgres = config
        .secrets
        .postgres
        .as_deref()
        .context("Gimme a postgres database please!")?;
    let database = PgPoolOptions::new().connect(postgres).await?;
    sqlx::migrate!("./migrations")
        .run(&database)
        .await
        .context("Couldn't run database migrations!")?;

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


    #[cfg(feature = "ai")]
    let llm_channel = if let Some(llm_config) = config.ai.as_ref().and_then(|x| x.llm.clone()) {
        tracing::debug!("LLM configuration: {llm_config:#?}");
        let (mut llm, discord_message_tx) = llm::Llm::new(llm_config).await?;
        tokio::spawn(async move {
            if let Err(e) = llm.spawn().await {
                error!("LLM erorr: {e}");
            };
        });
        info!("Initialized LLM");
        Some(discord_message_tx.clone())
    } else {
        tracing::debug!("Not initializing LLM");
        None
    };

    let event_handler = EventHandler {
        config: config.clone(),
        logger: logger.clone(),
        error_messages: load_error_messages(config.clone()).await,
        component_map: componet::ComponentMap::new(),
        database,
        #[cfg(feature = "ai")]
        llm_channel,
    };
    let mut client = build_client(
        config
            .secrets
            .token
            .clone()
            .context("Why didn't you provide a discord token?")?,
        event_handler,
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
    #[cfg(feature = "ai")]
    llm_channel:    Option<flume::Sender<llm::LlmMessage>>,
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
                match self.component_map.run(&id, component, args).await {
                    Some(Err(e)) => {
                        tracing::error!(
                            "Error running component handler registered for component '{id}': {e}"
                        );
                    }
                    Some(Ok(_)) => tracing::trace!("Ran component handler registered for component '{id}'"),
                    None => tracing::warn!("No component handler regestered for component '{id}'"),
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

    #[cfg(feature = "ai")]
    async fn message(&self, context: Context, message: Message) {
        match message.mentions_me(&context.http).await {
            Ok(true) if message.kind == MessageType::InlineReply => {
                // TODO: allow configuration...
                if let Some(tx) = self.llm_channel.clone() {
                    let content = message.content_safe(&context.cache);
                    let message = llm::LlmMessage::new(
                        &content,
                        context.http.clone(),
                        self.component_map.clone(),
                        &message,
                    );
                    if let Err(e) = tx.send_async(message.clone()).await {
                        error!("Couldn't send message to LLM task via sender: {e}");
                    }
                    tracing::trace!("Sent '{message:?}' to LLM");
                }
            }
            Err(e) => error!("Couldn't check if the message mentions me: {e}"),
            _ => {}
        }
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
