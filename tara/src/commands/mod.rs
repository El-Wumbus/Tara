use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use lazy_static::lazy_static;
use serenity::{
    all::{CommandInteraction, Guild},
    builder::CreateCommand,
    prelude::Context,
};
use tara_util::logging::CommandLogger;
use tracing::info;

use crate::{commands::common::CommandResponse, componet, config, database, logging, Result};

mod common;
mod conversions;
mod define;
mod help;
mod movie;
#[cfg(feature = "music")]
mod music;
mod random;
mod role;
mod search;
mod series;
mod settings;
mod wiki;

type Command = &'static (dyn DiscordCommand + Sync + Send);

macro_rules! cmd {
    ($cmd:expr) => {
        &$cmd as Command
    };
}

lazy_static! {
    /// The command map. It corralates command names and commands. It's a  [`HashMap<&'static str, &'static (dyn DiscordCommand + Sync + Send)>`].
    pub static ref COMMANDS: HashMap<&'static str, Command> = {
        /// All the commands that get registered and are searched for.
        const COMMANDS: &[Command] = &[
            cmd!(random::COMMAND),
            cmd!(define::COMMAND),
            cmd!(wiki::COMMAND),
            cmd!(settings::COMMAND),
            cmd!(conversions::COMMAND),
            cmd!(search::COMMAND),
            cmd!(role::COMMAND),
            cmd!(help::COMMAND),
            cmd!(movie::COMMAND),
            cmd!(series::COMMAND),
            #[cfg(feature = "music")]
            cmd!(music::COMMAND),
        ];

        let mut map = HashMap::with_capacity(COMMANDS.len());
        for cmd in COMMANDS {
            map.insert(cmd.name(), *cmd);
        }
        map
    };
}

#[derive(Clone)]
pub struct CommandArguments {
    pub(super) context:           Arc<Context>,
    pub(super) guild:             Option<Guild>,
    pub(super) config:            Arc<config::Configuration>,
    pub(super) guild_preferences: database::Guilds,
    pub(super) component_map:     componet::ComponentMap,
}


#[async_trait]
pub trait DiscordCommand {
    /// Register the discord command.
    fn register(&self) -> CreateCommand;

    /// Run the discord command
    async fn run(&self, command: Arc<CommandInteraction>, args: CommandArguments) -> Result<CommandResponse>;

    /// The name of the command
    fn name(&self) -> &'static str;

    /// Additonal helpful information pertaining to usage to be displayed by the `/help`
    /// command.
    fn help(&self) -> Option<String> { None }
}

/// Run a command specified by its name.
#[allow(clippy::too_many_arguments)]
pub async fn run_command(
    context: Context,
    command: CommandInteraction,
    guild: Option<Guild>,
    config: Arc<config::Configuration>,
    guild_preferences: database::Guilds,
    error_messages: Arc<config::ErrorMessages>,
    logger: CommandLogger,
    component_map: componet::ComponentMap,
) {
    let command_event = logging::logged_command_event_from_interaction(&context.cache, &command);
    logger.enqueue(command_event).await;
    let command_name = command.data.name.as_str();

    // Search the command name in the HashMap of commands (`COMMANDS`)
    let Some(cmd) = COMMANDS.get(command_name) else {
        CommandResponse::EphemeralString(format!("Command \"{command_name}\" doesn't exist."))
        .send(&command, &context.http)
        .await;

        return;
    };

    let context = Arc::new(context);
    let command = Arc::new(command);
    let command_arguments = CommandArguments {
        context: context.clone(),
        guild,
        config: config.clone(),
        guild_preferences,
        component_map,
    };

    // Run the command.
    let user = &command.user;
    let dm_or_server = match command_arguments.guild.as_ref() {
        Some(x) => format!("server \"{}\" (id: {})", x.name, x.id),
        None => "DM".to_string(),
    };

    info!(
        "Running \"{}\" (id: {}) on behalf of user \"{}\" (id: {}) running in {dm_or_server}",
        command.data.name, command.data.id, user.name, user.id,
    );

    match cmd.run(command.clone(), command_arguments).await {
        Ok(response) => response.send(&command, &context.http).await,
        Err(e) => {
            let error_message = pick_error_message(&error_messages);

            CommandResponse::EphemeralString(format!(
                "{}: *[{}] {}.*\n{}",
                error_message.0,
                e.code(),
                e,
                error_message.1
            ))
            .send(&command, &context.http)
            .await;
        }
    }
}

/// Randomly select an error message pre/postfix
fn pick_error_message(error_messages: &config::ErrorMessages) -> &(String, String) {
    use rand::seq::SliceRandom;
    error_messages.messages.choose(&mut rand::thread_rng()).unwrap()
}
