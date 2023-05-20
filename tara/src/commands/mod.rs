use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use lazy_static::lazy_static;
use serenity::{
    all::{CommandInteraction, Guild},
    builder::{CreateCommand, CreateEmbed, CreateInteractionResponse, CreateInteractionResponseMessage},
    http::Http,
    prelude::Context,
};

use crate::{config, database, Error, Result};

mod conversions;
mod core;
mod define;
mod random;
mod role;
mod search;
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
        ];

        let mut map = HashMap::with_capacity(COMMANDS.len());
        for cmd in COMMANDS {
            map.insert(cmd.name(), *cmd);
        }
        map
    };
}

#[derive(Debug, Clone)]
pub struct CommandArguments {
    context:           Arc<Context>,
    command:           Arc<CommandInteraction>,
    guild:             Option<Guild>,
    config:            Arc<config::Configuration>,
    guild_preferences: database::Guilds,
}


#[async_trait]
pub trait DiscordCommand {
    /// Register the discord command.
    fn register(&self) -> CreateCommand;

    /// Run the discord command
    async fn run(&self, args: CommandArguments) -> Result<CommandResponse>;

    /// The name of the command
    fn name(&self) -> &'static str;
}

#[derive(Debug, Clone)]
pub enum CommandResponse {
    String(String),
    EphemeralString(String),
    Embed(CreateEmbed),
    Message(CreateInteractionResponseMessage),
    None,
}

impl CommandResponse {
    pub fn new_string(s: impl Into<String>) -> Self { Self::from(s.into()) }

    pub fn is_none(&self) -> bool { matches!(self, Self::None) }

    pub async fn send(self, command: &CommandInteraction, http: &Http) {
        let message = CreateInteractionResponseMessage::new();
        let response_message = match self {
            CommandResponse::String(s) => message.content(s),
            CommandResponse::EphemeralString(s) => message.content(s).ephemeral(true),
            CommandResponse::Embed(embed) => message.embed(embed),
            CommandResponse::Message(message) => message,
            CommandResponse::None => return,
        };
        let response = CreateInteractionResponse::Message(response_message);

        if let Err(e) = command.create_response(http, response).await {
            log::error!(
                "Couldn't respond to command ({}): {e}",
                command.data.name.as_str()
            );
        }
    }
}

impl From<String> for CommandResponse {
    fn from(value: String) -> Self { Self::String(value) }
}

/// Run a command specified by its name.
pub async fn run_command(
    context: Context,
    command: CommandInteraction,
    guild: Option<Guild>,
    config: Arc<config::Configuration>,
    guild_preferences: database::Guilds,
    error_messages: Arc<config::ErrorMessages>,
) {
    let command_name = command.data.name.as_str();

    // Search the command name in the HashMap of commands (`COMMANDS`)
    if let Some(cmd) = COMMANDS.get(command_name) {
        let context = Arc::new(context);
        let command = Arc::new(command);
        let command_arguments = CommandArguments {
            context: context.clone(),
            command: command.clone(),
            guild,
            config: config.clone(),
            guild_preferences,
        };

        // Run the command.
        match cmd.run(command_arguments).await {
            Err(e) => notify_user_of_error(e, &context.http, &command, &error_messages).await,
            Ok(response) if !response.is_none() => response.send(&command, &context.http).await,
            Ok(content) if content.is_none() => {
                // Do nothing. content should only be empty if the command we called
                // already responded.
            }
            _ => unreachable!(),
        }

        return;
    }

    CommandResponse::EphemeralString(format!("Command \"{command_name}\" doesn't exist."))
        .send(&command, &context.http)
        .await;
}

pub async fn notify_user_of_error(
    e: Error,
    http: &Http,
    command: &CommandInteraction,
    error_messages: &config::ErrorMessages,
) {
    let error_message = pick_error_message(error_messages);
    let msg = format!(
        "{}: *[{}] {}.*\n{}",
        error_message.0,
        e.code(),
        e,
        error_message.1
    );

    CommandResponse::EphemeralString(msg).send(command, http).await;
}

/// Randomly select an error message pre/postfix
fn pick_error_message(error_messages: &config::ErrorMessages) -> &(String, String) {
    use rand::seq::SliceRandom;
    // TODO: Don't panic here!
    error_messages.messages.choose(&mut rand::thread_rng()).unwrap()
}
