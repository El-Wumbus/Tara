// I apologize to anyone reading this; this is a mess.
use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use once_cell::sync::Lazy;
use serenity::{
    all::{ChannelId, CommandInteraction, CommandOptionType, Guild, GuildId, MessageId},
    builder::{Builder, CreateCommand, CreateCommandOption, CreateEmbed, EditMessage},
    http::Http,
    prelude::Context,
};
use songbird::{
    events::EventHandler as VoiceEventHandler, input::YoutubeDl, tracks::TrackHandle, Event, EventContext,
    Songbird, TrackEvent,
};
use tokio::sync::Mutex;
use tracing::error;
use uuid::Uuid;

use self::youtube::TrackInfo;
use super::{common::CommandResponse, CommandArguments, DiscordCommand};
use crate::{commands::common, Error, HttpKey, Result};

mod youtube;

static YOUTUBE_CLIENT_CONFIG: Lazy<Arc<youtubei_rs::types::client::ClientConfig>> =
    Lazy::new(|| Arc::new(youtubei_rs::utils::default_client_config()));

#[allow(clippy::type_complexity)]
static CURRENTLY_PLAYING: Lazy<Arc<Mutex<HashMap<Uuid, (TrackInfo, TrackHandle)>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

static GUILD_TO_TRACK_MAP: Lazy<Arc<Mutex<HashMap<GuildId, Uuid>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

static GUILD_CHANNEL_MAP: Lazy<Arc<Mutex<HashMap<Uuid, MessageId>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

pub const COMMAND: Music = Music;

#[derive(Clone, Copy, Debug)]
pub struct Music;

#[async_trait]
impl DiscordCommand for Music {
    fn register(&self) -> serenity::builder::CreateCommand {
        let play = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "play",
            "Join your voice channel and play a song",
        )
        .add_sub_option(
            CreateCommandOption::new(
                CommandOptionType::String,
                "url",
                "The YouTube URL of the track to be played",
            )
            .required(true),
        );
        let stop = CreateCommandOption::new(CommandOptionType::SubCommand, "stop", "Stop playback");
        let pause = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "pause",
            "Pause the currently playing track",
        );
        let unpause = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "unpause",
            "Resume a currently paused track",
        );
        let leave =
            CreateCommandOption::new(CommandOptionType::SubCommand, "leave", "Leave your voice channel");
        let options = vec![leave, play, pause, unpause, stop];

        CreateCommand::new(self.name())
            .dm_permission(false)
            .description("(Alpha) Listen to your favorate songs from YouTube with your friends")
            .set_options(options)
    }

    async fn run(
        &self,
        command: Arc<CommandInteraction>,
        args: CommandArguments,
    ) -> Result<common::CommandResponse> {
        let config = args.config.music.clone().unwrap_or_default();
        if !config.enabled {
            return Err(Error::FeatureDisabled(
                "Music playback is disabled on this instance. Contact the host to enable this feature."
                    .to_string(),
            ));
        }

        let Some(guild) = args.guild else {
            return Err(Error::InternalLogic);
        };
        let option = &command.data.options[0];
        let manager = songbird::get(&args.context).await.unwrap();
        match &*option.name {
            "play" => {
                // Get the url
                let mut options = common::suboptions(option).iter();
                let Some(url_option) = options.next() else {
                    return Err(Error::InternalLogic);
                };
                let Some(url) = url_option.value.as_str() else {
                    return Err(Error::InternalLogic);
                };

                // Check the url is a youtube url
                if !youtube::YOUTUBE_REGEX.is_match(url) {
                    return Err(Error::CommandMisuse(
                        "Must provide a valid YouTube video/audio URL!".to_string(),
                    ));
                }

                play(url, args.context.clone(), &manager, &guild, command.clone()).await
            }
            "stop" => stop(guild.id).await,
            "leave" => leave(&manager, guild.id).await,
            "pause" => pause(guild.id).await,
            "unpause" => unpause(guild.id).await,
            _ => return Err(Error::InternalLogic),
        }
    }

    fn name(&self) -> &'static str { "music" }
}


async fn play(
    url: &str,
    context: Arc<Context>,
    manager: &Songbird,
    guild: &Guild,
    command: Arc<CommandInteraction>,
) -> Result<CommandResponse> {
    let track_info = youtube::TrackInfo::from_youtube_url(YOUTUBE_CLIENT_CONFIG.clone(), url).await?;
    let mut embed = CreateEmbed::from(track_info.clone());

    let (handler_lock, mut message) = match manager.get(guild.id) {
        Some(x) => {
            // Create inital response message
            CommandResponse::Embed(Box::new(embed.clone()))
                .send(&command, &context.http)
                .await;
            let response = command.get_response(&context.http).await?;
            (x, response)
        }
        None => {
            // Join
            let Some(voice_channel_id) = guild
                .voice_states
                .get(&command.user.id)
                .and_then(|voice_state| voice_state.channel_id)
            else {
                return Err(Error::CommandMisuse("You're not in a voice channel!".to_string()));
            };

            // We send a progress message then edit it later because discord only gives us 3 seconds
            // to reply to a slash command.
            CommandResponse::Embed(Box::new(embed.clone().description("Joining voice channel...")))
                .send(&command, &context.http)
                .await;

            join(&context, manager, guild.id, command.channel_id, voice_channel_id).await;

            let response = command.get_response(&context.http).await?;
            (manager.get(guild.id).unwrap(), response)
        }
    };

    let http_client = {
        let data = context.data.read().await;
        data.get::<HttpKey>().cloned().expect("to exist in the typemap")
    };
    let mut handler = handler_lock.lock().await;
    let source = YoutubeDl::new(http_client, url.to_string());
    let handle = handler.play_only_input(source.into());
    let uuid = handle.uuid();

    embed = embed.description("").field("Status", "Playing", false);

    // Insert the currently playing track into `CURRENTLY_PLAYING`
    let mut currently_playing = CURRENTLY_PLAYING.lock().await;
    currently_playing.insert(uuid, (track_info, handle.clone()));
    GUILD_TO_TRACK_MAP.lock().await.insert(guild.id, uuid);
    GUILD_CHANNEL_MAP.lock().await.insert(uuid, message.id);

    message
        .edit(&context.http, EditMessage::new().embed(embed))
        .await?;

    Ok(CommandResponse::None)
}

/// Join the voice channel specified in `voice_channel_id` and add global event handlers.
async fn join(
    context: &Context,
    manager: &Songbird,
    guild_id: GuildId,
    channel_id: ChannelId,
    voice_channel_id: ChannelId,
) {
    if let Ok(lock) = manager.join(guild_id, voice_channel_id).await {
        let mut handler = lock.lock().await;
        handler.add_global_event(TrackEvent::Error.into(), TrackErrorNotifier);
        handler.add_global_event(
            TrackEvent::End.into(),
            TrackEndNotifier {
                channel_id,
                http: context.http.clone(),
            },
        );
        handler.add_global_event(
            TrackEvent::Pause.into(),
            TrackPauseNotifier {
                channel_id,
                http: context.http.clone(),
            },
        );
        handler.add_global_event(
            TrackEvent::Play.into(),
            TrackPlayNotifier {
                channel_id,
                http: context.http.clone(),
            },
        )
    }
}

async fn leave(manager: &Songbird, guild_id: GuildId) -> Result<CommandResponse> {
    manager
        .get(guild_id)
        .ok_or_else(|| Error::CommandMisuse("I'm not in a voice channel!".to_string()))?;

    manager.remove(guild_id).await?;
    let mut guild_track_map = GUILD_TO_TRACK_MAP.lock().await;
    if let Some(uuid) = guild_track_map.remove(&guild_id) {
        CURRENTLY_PLAYING.lock().await.remove(&uuid);
        GUILD_CHANNEL_MAP.lock().await.remove(&uuid);
    }

    Ok(CommandResponse::EphemeralString(
        "I left your voice channel!".to_string(),
    ))
}

async fn stop(guild_id: GuildId) -> Result<CommandResponse> {
    let guild_track_map = GUILD_TO_TRACK_MAP.lock().await;
    let uuid = guild_track_map
        .get(&guild_id)
        .ok_or_else(|| Error::InternalLogic)?;
    let currently_playing = CURRENTLY_PLAYING.lock().await;
    let (track, track_handle) = currently_playing.get(uuid).ok_or_else(|| Error::InternalLogic)?;
    let _ = track_handle.stop();
    Ok(CommandResponse::EphemeralString(format!(
        "*{}* is now stopped.",
        track.title
    )))
}

async fn pause(guild_id: GuildId) -> Result<CommandResponse> {
    let guild_track_map = GUILD_TO_TRACK_MAP.lock().await;
    let uuid = guild_track_map
        .get(&guild_id)
        .ok_or_else(|| Error::InternalLogic)?;
    let currently_playing = CURRENTLY_PLAYING.lock().await;
    let (track, track_handle) = currently_playing.get(uuid).ok_or_else(|| Error::InternalLogic)?;
    let _ = track_handle.pause();
    Ok(CommandResponse::EphemeralString(format!(
        "*{}* is now paused.",
        track.title
    )))
}

async fn unpause(guild_id: GuildId) -> Result<CommandResponse> {
    let guild_track_map = GUILD_TO_TRACK_MAP.lock().await;
    let uuid = guild_track_map
        .get(&guild_id)
        .ok_or_else(|| Error::InternalLogic)?;
    let currently_playing = CURRENTLY_PLAYING.lock().await;
    let (track, track_handle) = currently_playing.get(uuid).ok_or_else(|| Error::InternalLogic)?;
    let _ = track_handle.play();
    Ok(CommandResponse::EphemeralString(format!(
        "*{}* is now unpaused.",
        track.title
    )))
}

struct TrackErrorNotifier;

#[async_trait]
impl VoiceEventHandler for TrackErrorNotifier {
    async fn act(&self, context: &EventContext<'_>) -> Option<Event> {
        if let EventContext::Track(track_list) = context {
            for (state, handle) in *track_list {
                error!(
                    "Track {:?} encountered an error: {:?}",
                    handle.uuid(),
                    state.playing
                );
            }
        }

        None
    }
}

/// Deletes the message related to the track that just ended
struct TrackEndNotifier {
    channel_id: ChannelId,
    http:       Arc<Http>,
}

#[async_trait]
impl VoiceEventHandler for TrackEndNotifier {
    async fn act(&self, context: &EventContext<'_>) -> Option<Event> {
        if let EventContext::Track(track_list) = context {
            for (_state, handle) in *track_list {
                let uuid = handle.uuid();
                if let Some(message_id) = GUILD_CHANNEL_MAP.lock().await.remove(&uuid) {
                    if let Ok(message) = self.channel_id.message(&self.http, message_id).await {
                        let _ = message
                            .delete(&self.http)
                            .await
                            .map_err(|e| error!("Error deleting message: {e}"));
                    }
                }
            }
        }

        None
    }
}

struct TrackPauseNotifier {
    channel_id: ChannelId,
    http:       Arc<Http>,
}

#[async_trait]
impl VoiceEventHandler for TrackPauseNotifier {
    async fn act(&self, context: &EventContext<'_>) -> Option<Event> {
        if let EventContext::Track(track_list) = context {
            for (_state, handle) in *track_list {
                let uuid = handle.uuid();
                let currently_playing = CURRENTLY_PLAYING.lock().await;
                if let Some((track, _)) = currently_playing.get(&uuid).cloned() {
                    if let Some(message_id) = GUILD_CHANNEL_MAP.lock().await.get(&uuid).cloned() {
                        let embed = CreateEmbed::from(track).field("Status", "Paused", false);
                        let _ = EditMessage::new()
                            .add_embed(embed)
                            .execute(&self.http, (self.channel_id, message_id))
                            .await
                            .map_err(|e| error!("Error sending message to channel {}: {e}", self.channel_id));
                    }
                }
            }
        }

        None
    }
}

struct TrackPlayNotifier {
    channel_id: ChannelId,
    http:       Arc<Http>,
}

#[async_trait]
impl VoiceEventHandler for TrackPlayNotifier {
    async fn act(&self, context: &EventContext<'_>) -> Option<Event> {
        if let EventContext::Track(track_list) = context {
            for (_state, handle) in *track_list {
                let uuid = handle.uuid();
                let currently_playing = CURRENTLY_PLAYING.lock().await;
                if let Some((track, _)) = currently_playing.get(&uuid).cloned() {
                    if let Some(message_id) = GUILD_CHANNEL_MAP.lock().await.get(&uuid).cloned() {
                        let embed = CreateEmbed::from(track).field("Status", "Playing", false);
                        let _ = EditMessage::new()
                            .add_embed(embed)
                            .execute(&self.http, (self.channel_id, message_id))
                            .await
                            .map_err(|e| error!("Error sending message to channel {}: {e}", self.channel_id));
                    }
                }
            }
        }

        None
    }
}
