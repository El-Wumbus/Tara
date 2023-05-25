use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use cached::proc_macro::once;
use lazy_static::lazy_static;
use serenity::{
    builder::CreateApplicationCommand,
    http::Http,
    model::prelude::interaction::{
        application_command::ApplicationCommandInteraction, InteractionResponseType,
    },
    prelude::Context,
};

use crate::{config, Error, Result};

mod conversions;
mod define;
mod random;
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
pub trait DiscordCommand
{
    /// Register the discord command.
    fn register<'a>(&'a self, command: &'a mut CreateApplicationCommand) -> &mut CreateApplicationCommand;

    /// Run the discord command
    async fn run(
        &self,
        context: &Context,
        command: &ApplicationCommandInteraction,
        config: Arc<config::Configuration>,
        databases: Arc<crate::database::Databases>,
    ) -> Result<String>;

    /// The name of the command
    fn name(&self) -> String;
}

#[must_use]
pub fn get_command_name(command: &ApplicationCommandInteraction) -> String { command.data.name.to_string() }

/// Run a command specified by its name.
pub async fn run_command(
    context: Context,
    command: ApplicationCommandInteraction,
    config: Arc<config::Configuration>,
    databases: Arc<crate::database::Databases>,
    error_messages: Arc<config::ErrorMessages>,
)
{
    let command_name = get_command_name(&command);
    if let Some(cmd) = COMMAND_MAP.get(&command_name) {
        let cmd = &COMMANDS[*cmd];
        match cmd.run(&context, &command, config.clone(), databases).await {
            Err(e) => notify_user_of_error(e, &context.http, &command, error_messages.clone()).await,
            Ok(x) => give_user_results(x, &context.http, &command).await,
        }
    }
    else {
        // Respond with an ephemeral error message, this means that only the user who
        // started the interaction can see the error.
        if let Err(e) = command
            .create_interaction_response(&context.http, |response| {
                response
                    .kind(InteractionResponseType::ChannelMessageWithSource)
                    .interaction_response_data(|message| {
                        message.ephemeral(true);
                        message.content(format!("Command \"{command_name}\" doesn't exist."))
                    })
            })
            .await
        {
            log::error!("Couldn't respond to command: {e}");
        }
    }
}

async fn notify_user_of_error(
    e: Error,
    http: &Http,
    command: &ApplicationCommandInteraction,
    error_messages: Arc<config::ErrorMessages>,
)
{
    let error_message = pick_error_message(&error_messages);
    let msg = format!(
        "{}: *[{}] {}.*\n{}",
        error_message.0,
        e.code(),
        e,
        error_message.1
    );
    if let Err(e) = command
        .create_interaction_response(http, |response| {
            response
                .kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|message| {
                    message.ephemeral(true);
                    message.content(msg)
                })
        })
        .await
    {
        log::error!("Couldn't respond to command: {e}");
    }
}

async fn give_user_results(results: String, http: &Http, command: &ApplicationCommandInteraction)
{
    if let Err(e) = command
        .create_interaction_response(http, |response| {
            response
                .kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|message| message.content(results))
        })
        .await
    {
        log::error!("Couldn't respond to command: {e}");
    }
}

/// Randomly select an error message pre/postfix
fn pick_error_message(error_messages: &config::ErrorMessages) -> (String, String)
{
    use rand::seq::SliceRandom;
    error_messages
        .messages
        .choose(&mut rand::thread_rng())
        .unwrap()
        .clone()
}

pub mod core
{
    use super::{once, ApplicationCommandInteraction, Result};

    #[once(time = 15, result = true)]
    pub fn get_max_content_len(
        command: &ApplicationCommandInteraction,
        databases: &crate::database::Databases,
    ) -> Result<usize>
    {
        // Get the max from the guild's configuration. If we're not in a guild then we
        // use the default.
        let max = {
            if let Some(guild_id) = command.guild_id {
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

    #[must_use]
    pub fn strip_suffixes(mut input: String, suffixes: &[&str]) -> String
    {
        for suffix in suffixes {
            input = match input.strip_suffix(suffix) {
                Some(input) => input,
                None => &input,
            }
            .to_string();
        }
        input
    }
}
