use std::{path::Path, sync::Arc};

use chrono::Utc;
use crossbeam_queue::SegQueue;
use csv_async::AsyncWriterBuilder;
use serde::{Deserialize, Serialize};
use serenity::{
    all::{ChannelId, CommandInteraction, GuildId, UserId},
    client::Cache,
};
use tokio::{
    fs::{self, File},
    sync::Mutex,
    time,
};

use crate::Result;


#[derive(Debug, Clone)]
pub struct CommandLogger {
    queue: Arc<Mutex<SegQueue<LoggedCommandEvent>>>,
}

impl Default for CommandLogger {
    fn default() -> Self { Self::new() }
}

impl CommandLogger {
    pub fn new() -> Self {
        // let (transmit_stop, mut receive_stop) = mpsc::channel::<bool>(64);
        Self {
            queue: Arc::new(Mutex::new(SegQueue::default())),
        }
    }

    /// Push an item to the queue to be logged at next run time.
    pub async fn enqueue(&self, command_event: LoggedCommandEvent) {
        self.queue.lock().await.push(command_event);
    }

    async fn dequeue(&self) -> Option<LoggedCommandEvent> { self.queue.lock().await.pop() }

    pub async fn len(&self) -> usize { self.queue.lock().await.len() }

    /// Continuously logs items present in the queue. **This function never returns**
    /// unless an error occurrs.
    ///
    /// # Errors
    ///
    /// Errors may occurr if:
    /// - writing to the provided file raises an IO error
    /// - serilization raises an error
    pub async fn log_to_file(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();

        let parent_path = path.parent().unwrap();
        if !parent_path.exists() {
            fs::create_dir_all(parent_path).await?;
        }

        let mut csv_file = AsyncWriterBuilder::new()
            .has_headers(false)
            .create_serializer(File::create(path).await?);
        loop {
            while self.len().await == 0 {
                // Asyncronously Sleep for some seconds to let the bot work.
                time::sleep(time::Duration::from_secs(6)).await;
            }

            while self.len().await != 0 {
                // Dequeue the command event
                let command_event = self.dequeue().await.unwrap();
                csv_file.serialize(command_event).await?;
            }
        }
    }

    // pub fn switch_log_file(&self, path: impl Into<pathBuf>) {

    // }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoggedCommandEvent {
    /// Name of the command that was called
    pub name:              String,
    /// Time the command was called
    pub time:              chrono::DateTime<Utc>,
    /// The channel the command was called in
    pub channel_id:        ChannelId,
    /// User that called the command
    pub user:              (String, UserId),
    /// Was the commmand called from a guild
    pub called_from_guild: bool,
    /// The guild that called the command
    pub guild_info:        Option<(String, GuildId)>,
}

impl LoggedCommandEvent {
    pub fn from_command_interaction(cache: &impl AsRef<Cache>, command: &CommandInteraction) -> Self {
        let time = Utc::now();
        let guild_info = command
            .guild_id
            .and_then(|id| id.to_guild_cached(cache))
            .map(|guild| (guild.name.clone(), guild.id));
        let name = command.data.name.clone();
        let user = (command.user.name.clone(), command.user.id);
        Self {
            name,
            time,
            channel_id: command.channel_id,
            user,
            called_from_guild: guild_info.is_some(),
            guild_info,
        }
    }
}
