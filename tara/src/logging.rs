use chrono::Utc;
use serenity::{all::CommandInteraction, client::Cache};
use tara_util::logging::LoggedCommandEvent;

pub fn logged_command_event_from_interaction(
    cache: &impl AsRef<Cache>,
    command: &CommandInteraction,
) -> LoggedCommandEvent {
    let time = Utc::now();
    let guild_info = command
        .guild_id
        .and_then(|id| id.to_guild_cached(cache))
        .map(|guild| (guild.name.clone(), guild.id.0));
    let name = command.data.name.clone();
    let user = (command.user.name.clone(), command.user.id.0);
    LoggedCommandEvent {
        name,
        time,
        channel_id: command.channel_id.0,
        user,
        called_from_guild: guild_info.is_some(),
        guild_info,
    }
}
