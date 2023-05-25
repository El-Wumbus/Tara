use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use lazy_static::lazy_static;
use serenity::{
    all::{CommandInteraction, Guild},
    builder::{CreateCommand, CreateInteractionResponse, CreateInteractionResponseMessage},
    http::Http,
    prelude::Context,
};

use crate::{config, database, Error, Result};

mod conversions;
mod define;
mod random;
mod role;
mod search;
mod settings;
mod wiki;

macro_rules! discord_command {
    ($cmd:expr) => {
        Box::new($cmd) as Box<dyn DiscordCommand + Sync + Send>
    };
}

crate::commands::lazy_static! {

    /// All callable commands
    pub static ref COMMANDS: Arc<Vec<Box<dyn DiscordCommand + Sync+ Send>>> =
       Arc::new(vec![
            discord_command!(random::COMMAND),
            discord_command!(define::COMMAND),
            discord_command!(wiki::COMMAND),
            discord_command!(settings::COMMAND),
            discord_command!(conversions::COMMAND),
            discord_command!(search::COMMAND),
            discord_command!(role::COMMAND),
        ]);

    /// Command name to `COMMANDS` index value.
    /// Every name corresponds to the index of that command.
    static ref COMMAND_MAP: Arc<HashMap<String, usize>> = Arc::new({
        let mut map = HashMap::new();
        for (i, cmd) in COMMANDS.iter().enumerate() {
            let name = cmd.name();
            map.insert(name, i);
        }

        map
    });
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
    async fn run(&self, args: CommandArguments) -> Result<String>;

    /// The name of the command
    fn name(&self) -> String;
}

#[must_use]
pub fn get_command_name(command: &CommandInteraction) -> String { command.data.name.to_string() }

/// Run a command specified by its name.
pub async fn run_command(
    context: Context,
    command: CommandInteraction,
    guild: Option<Guild>,
    config: Arc<config::Configuration>,
    guild_preferences: database::Guilds,
    error_messages: Arc<config::ErrorMessages>,
) {
    let command_name = get_command_name(&command);
    if let Some(cmd) = COMMAND_MAP.get(&command_name) {
        let cmd = &COMMANDS[*cmd];
        let context = Arc::new(context);
        let command = Arc::new(command);
        match cmd
            .run(CommandArguments {
                context: context.clone(),
                command: command.clone(),
                guild,
                config: config.clone(),
                guild_preferences,
            })
            .await
        {
            Err(e) => notify_user_of_error(e, &context.http, &command, error_messages.clone()).await,
            Ok(x) if !x.is_empty() => give_user_results(x, &context.http, &command).await,
            _ => (),
        }
    }
    else {
        // Respond with an ephemeral error message, this means that only the user who
        // started the interaction can see the error.
        let response = CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
                .content(format!("Command \"{command_name}\" doesn't exist."))
                .ephemeral(true),
        );
        // Respond with an ephemeral error message, this means that only the user who
        // started the interaction can see the error, and it's dismissable.
        if let Err(e) = command.create_response(&context.http, response).await {
            log::error!("Couldn't respond to command: {e}");
        }
    }
}

pub async fn notify_user_of_error(
    e: Error,
    http: &Http,
    command: &CommandInteraction,
    error_messages: Arc<config::ErrorMessages>,
) {
    let error_message = pick_error_message(&error_messages);
    let msg = format!(
        "{}: *[{}] {}.*\n{}",
        error_message.0,
        e.code(),
        e,
        error_message.1
    );
    let response = CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new()
            .content(msg)
            .ephemeral(true),
    );

    if let Err(e) = command.create_response(http, response).await {
        log::error!("Couldn't respond to command: {e}");
    }
}

async fn give_user_results(results: String, http: &Http, command: &CommandInteraction) {
    let response =
        CreateInteractionResponse::Message(CreateInteractionResponseMessage::new().content(results));
    if let Err(e) = command.create_response(http, response).await {
        log::error!("Couldn't respond to command: {e}");
    }
}

/// Randomly select an error message pre/postfix
fn pick_error_message(error_messages: &config::ErrorMessages) -> (String, String) {
    use rand::seq::SliceRandom;
    error_messages
        .messages
        .choose(&mut rand::thread_rng())
        .unwrap()
        .clone()
}

pub mod core {

    use serenity::all::{CommandDataOption, CommandDataOptionValue, GuildId};

    use super::Result;
    use crate::{
        database::{self, GuildPreferences},
        defaults,
    };

    pub fn suboptions(option: &CommandDataOption) -> &Vec<CommandDataOption> {
        let mut val = None;
        match &option.value {
            CommandDataOptionValue::SubCommand(options)
            | CommandDataOptionValue::SubCommandGroup(options) => val = Some(options),
            _ => (),
        }
        val.unwrap()
    }

    pub async fn get_content_character_limit(
        guild_id: Option<GuildId>,
        guild_prefs: &database::Guilds,
    ) -> Result<usize> {
        // Get the max from the guild's configuration. If we're not in a guild then we
        // use the default.
        if let Some(guild_id) = guild_id {
            if !guild_prefs.contains(guild_id).await {
                // Insert default data
                guild_prefs.insert(GuildPreferences::default(guild_id)).await;
            }

            Ok(guild_prefs.get(guild_id).await.unwrap().content_character_limit)
        }
        else {
            Ok(defaults::content_character_limit_default())
        }
    }

    #[must_use]
    /// Remove the first of any suffixes found in `suffixes` from the input string.
    pub fn strip_suffixes(input: String, suffixes: &[&str]) -> String {
        let input_bytes = input.as_bytes();
        let mut _suffix_bytes: &[u8];

        for suffix in suffixes {
            if let Some(input_without_suffix) = input_bytes.strip_suffix(suffix.as_bytes()) {
                return String::from_utf8_lossy(input_without_suffix).into_owned();
            }
        }

        input.to_owned()
    }
}
