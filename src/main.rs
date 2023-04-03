#![feature(const_trait_impl)]

use std::{collections::HashMap, sync::Arc};

use chrono::Utc;
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

mod error;
pub use error::*;

mod commands;
mod config;
mod database;

#[tokio::main]
async fn main() -> std::result::Result<(), anyhow::Error>
{
    // Setup logging
    env_logger::init();
    log::info!("Initialized Logger");

    // Get the configuration file path and read the configuration from it.
    let config_path = get_config_dir()?;
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
            databases: database::Databases::open(config.clone()).await?,
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

struct EventHandler
{
    config:              Arc<config::Configuration>,
    error_messages:      Arc<config::ErrorMessages>,
    databases:           Arc<database::Databases>,
    dm_cooldown_counter: Arc<Mutex<HashMap<UserId, chrono::DateTime<Utc>>>>,
}


#[async_trait]
impl client::EventHandler for EventHandler
{
    async fn interaction_create(&self, context: Context, interaction: Interaction)
    {
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

    async fn ready(&self, context: Context, ready: Ready)
    {
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


/// Returns a configuration file after checking some of the default locations.
///
/// # Usage
///
/// ```Rust
/// let path = get_config_dir().unwrap();
/// ```
///
/// # Errors
///
/// Returns an `Error` when:
///
/// - No configuration file is found
fn get_config_dir() -> Result<std::path::PathBuf>
{
    const DEFAULT_CONFIG_FILE: &str = "/etc/tara.d/tara.toml";

    let file = std::path::PathBuf::from(DEFAULT_CONFIG_FILE);
    if file.is_file() {
        Ok(file)
    }
    else {
        Err(Error::MissingConfigurationFile(format!(
            "No configuration file found. Checked the following:\n{}",
            file.display()
        )))
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
async fn load_error_messages(config: Arc<config::Configuration>) -> Arc<config::ErrorMessages>
{
    Arc::new(match &config.random_error_message {
        config::ConfigurationRandomErrorMessages::Boolean(x) => {
            if *x {
                // Load from the default location, if not possible fall back to the default
                // messages.
                match config::ErrorMessages::from_json(config::ErrorMessages::DEFAULT_FILE).await {
                    Ok(x) => {
                        log::info!(
                            "Loaded Error Messages from {}",
                            config::ErrorMessages::DEFAULT_FILE
                        );
                        x
                    }
                    Err(e) => {
                        log::warn!(
                            "Couldn't load error messages from \"{}\": {e}. Using defaults.",
                            config::ErrorMessages::DEFAULT_FILE
                        );
                        config::ErrorMessages::default()
                    }
                }
            }
            else {
                config::ErrorMessages::default()
            }
        }
        config::ConfigurationRandomErrorMessages::Path(path) => {
            match config::ErrorMessages::from_json(&path).await {
                Ok(x) => {
                    log::info!("Loaded Error Messages from {}", path.display());
                    x
                }
                Err(e) => {
                    log::warn!(
                        "Couldn't load error messages from \"{}\": {e}. Using defaults.",
                        path.display()
                    );
                    config::ErrorMessages::default()
                }
            }
        }
    })
}
