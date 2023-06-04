use serenity::{
    all::{CommandDataOption, CommandDataOptionValue, CommandInteraction, GuildId},
    builder::{CreateEmbed, CreateInteractionResponse, CreateInteractionResponseMessage},
    http::Http,
};
use tracing::{event, Level};

use super::Result;
use crate::{
    database::{self, GuildPreferences},
    defaults,
};

#[must_use]
/// Gets the suboptions of a subcommand or subcommandgroup.
///
/// # Panics
///
/// Panics if `option.value` isn't a [`CommandDataOptionValue::SubCommand`] or
/// [`CommandDataOptionValue::SubCommandGroup`]
pub fn suboptions(option: &CommandDataOption) -> &Vec<CommandDataOption> {
    let mut val = None;
    match &option.value {
        CommandDataOptionValue::SubCommand(options) | CommandDataOptionValue::SubCommandGroup(options) => {
            val = Some(options);
        }
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
    } else {
        Ok(defaults::content_character_limit_default())
    }
}

#[must_use]
/// Remove the first of any suffixes found in `suffixes` from the input string.
pub fn strip_suffixes(input: &str, suffixes: &[&str]) -> String {
    let input_bytes = input.as_bytes();
    let mut _suffix_bytes: &[u8];

    for suffix in suffixes {
        if let Some(input_without_suffix) = input_bytes.strip_suffix(suffix.as_bytes()) {
            return String::from_utf8_lossy(input_without_suffix).into_owned();
        }
    }

    input.to_string()
}

pub fn ends_with_any<'a>(s: &str, possible_suffixes: &'a [&'a str]) -> bool {
    possible_suffixes.iter().any(|x| s.ends_with(x))
}

pub fn equals_any<'a>(s: &str, possible_matches: &'a [&'a str]) -> bool {
    possible_matches.iter().any(|x| *x == s)
}

#[derive(Debug, Clone)]
pub enum CommandResponse {
    String(String),
    EphemeralString(String),
    Embed(Box<CreateEmbed>),
    Message(CreateInteractionResponseMessage),
}

impl CommandResponse {
    pub fn new_string(s: impl Into<String>) -> Self { Self::from(s.into()) }

    pub async fn send(self, command: &CommandInteraction, http: &Http) {
        let message = CreateInteractionResponseMessage::new();
        let response_message = match self {
            CommandResponse::String(s) => message.content(s),
            CommandResponse::EphemeralString(s) => message.content(s).ephemeral(true),
            CommandResponse::Embed(embed) => message.embed(*embed),
            CommandResponse::Message(message) => message,
        };
        let response = CreateInteractionResponse::Message(response_message);
        if let Err(e) = command.create_response(http, response).await {
            event!(
                Level::ERROR,
                "Couldn't respond to command ({}): {e}",
                command.data.name.as_str()
            );
        }
    }
}

impl From<String> for CommandResponse {
    fn from(value: String) -> Self { Self::String(value) }
}
