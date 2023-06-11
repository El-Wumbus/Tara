use serenity::all::GuildId;

use crate::{commands::CommandResponse, database, Result};


pub async fn content_character_limit(
    guild_id: Option<GuildId>,
    guilds: &database::Guilds,
) -> Result<CommandResponse> {
    let max = crate::commands::common::get_content_character_limit(guild_id, guilds).await?;
    Ok(format!("content_character_limit = {max}").into())
}
