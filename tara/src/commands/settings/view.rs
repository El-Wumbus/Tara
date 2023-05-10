use serenity::all::GuildId;

use crate::{database, Result};


pub async fn content_character_limit(guild_id: Option<GuildId>, guilds: &database::Guilds) -> Result<String> {
    let max = crate::commands::core::get_content_character_limit(guild_id, guilds).await?;
    Ok(format!("content_character_limit = {max}"))
}
