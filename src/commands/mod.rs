use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use cached::proc_macro::once;
use lazy_static::lazy_static;
use serenity::{
    all::{CommandInteraction, Guild},
    builder::{CreateCommand, CreateInteractionResponse, CreateInteractionResponseMessage},
    http::Http,
    prelude::Context,
};

use crate::{config, Error, Result};

mod conversions;
mod define;
mod random;
mod role;
mod search;
mod settings;
pub mod wiki;

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
    pub static ref COMMAND_MAP: Arc<HashMap<String, usize>> = Arc::new({
        let mut map = HashMap::new();
        for (i, cmd) in COMMANDS.iter().enumerate() {
            let name = cmd.name();
            map.insert(name, i);
        }

        map
    });
}

#[async_trait]
pub trait DiscordCommand {
    /// Register the discord command.
    fn register(&self) -> CreateCommand;

    /// Run the discord command
    async fn run(
        &self,
        context: &Context,
        command: &CommandInteraction,
        guild: Option<Guild>,
        config: Arc<config::Configuration>,
        databases: Arc<crate::database::Databases>,
    ) -> Result<String>;

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
    databases: Arc<crate::database::Databases>,
    error_messages: Arc<config::ErrorMessages>,
) {
    let command_name = get_command_name(&command);
    if let Some(cmd) = COMMAND_MAP.get(&command_name) {
        let cmd = &COMMANDS[*cmd];
        match cmd
            .run(&context, &command, guild, config.clone(), databases)
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
    use std::collections::HashSet;

    use serenity::{
        all::{CommandDataOption, CommandDataOptionValue},
        model::prelude::GuildId,
    };

    use super::{once, CommandInteraction, Result};

    pub fn suboptions(option: &CommandDataOption) -> &Vec<CommandDataOption> {
        let mut val = None;
        match &option.value {
            CommandDataOptionValue::SubCommand(options)
            | CommandDataOptionValue::SubCommandGroup(options) => val = Some(options),
            _ => (),
        }
        val.unwrap()
    }

    #[once(time = 15, result = true)]
    pub fn get_max_content_len(
        command: &CommandInteraction,
        databases: &crate::database::Databases,
    ) -> Result<usize> {
        // Get the max from the guild's configuration. If we're not in a guild then we
        // use the default.
        let max = {
            if let Some(guild_id) = command.guild_id {
                if !databases.contains("guilds", guild_id)? {
                    // Insert default data
                    databases.guilds_insert_default(guild_id)?;
                }

                let connection = databases
                    .guilds
                    .get()
                    .map_err(crate::Error::DatabaseAccessTimeout)?;

                let mut statement = connection
                    .prepare(&format!(
                        "SELECT max_content_chars FROM guilds WHERE GuildID={guild_id}"
                    ))
                    .map_err(crate::Error::from)?;
                let max: u32 = statement
                    .query_row([], |row| {
                        let value: u32 = row.get(0).unwrap_or(super::wiki::Wiki::DEFAULT_MAX_WIKI_LEN);
                        Ok(value)
                    })
                    .map_err(crate::Error::from)?;
                max
            }
            else {
                super::wiki::Wiki::DEFAULT_MAX_WIKI_LEN
            }
        } as usize;
        Ok(max)
    }

    /// Remove any suffixes found in `suffixes` from the input string.
    #[must_use]
    pub fn strip_suffixes(input: String, suffixes: &[&str]) -> String {
        let input_bytes = input.as_bytes();
        let mut suffix_bytes: &[u8];

        for suffix in suffixes {
            suffix_bytes = suffix.as_bytes();
            if let Some(input_without_suffix) = input_bytes.strip_suffix(suffix_bytes) {
                return String::from_utf8_lossy(input_without_suffix).into_owned();
            }
        }

        input.to_owned()
    }

    pub fn get_role_ids(databases: &crate::database::Databases, guild_id: GuildId) -> Result<HashSet<u64>> {
        let connection = databases
            .guilds
            .get()
            .map_err(crate::Error::DatabaseAccessTimeout)?;

        let mut statement = connection
            .prepare(&format!(
                "SELECT assignable_roles FROM guilds WHERE GuildID={guild_id}"
            ))
            .map_err(crate::Error::from)?;
        let role_names = statement
            .query_row([], |row| {
                let bytes: Vec<u8> = row.get(0).unwrap();
                let value: HashSet<u64> = bincode::deserialize(&bytes).unwrap();
                Ok(value)
            })
            .map_err(crate::Error::from)?;

        Ok(role_names)
    }
}
