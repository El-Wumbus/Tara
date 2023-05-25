use serenity::all::{CommandDataOption, CommandDataOptionValue, GuildId};

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
            val = Some(options)
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
    }
    else {
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
