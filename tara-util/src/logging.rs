use std::{num::NonZeroU64, path::Path, sync::Arc};

use chrono::Utc;
use crossbeam_queue::SegQueue;
use csv_async::AsyncWriterBuilder;
use serde::{Deserialize, Serialize};
use tokio::{
    fs::{self, File},
    sync::Mutex,
    time,
};

use crate::error::LoggingError;


#[derive(Debug, Clone)]
pub struct CommandLogger {
    queue: Arc<Mutex<SegQueue<LoggedCommandEvent>>>,
}

impl Default for CommandLogger {
    fn default() -> Self { Self::new() }
}

impl CommandLogger {
    #[must_use]
    #[inline]
    pub fn new() -> Self {
        Self {
            queue: Arc::new(Mutex::new(SegQueue::default())),
        }
    }

    #[inline]
    /// Push an item to the queue to be logged at next run time.
    ///
    /// ```
    /// # use tara_util::logging::*;
    /// # use std::num::NonZeroU64;
    /// # use chrono::Utc;
    /// # tokio_test::block_on(async {
    /// # let one = NonZeroU64::new(1).unwrap();
    /// # let command_event = LoggedCommandEvent {
    /// #   name: String::new(),
    /// #   time: Utc::now(),
    /// #   channel_id: one,
    /// #   user: (String::new(), one),
    /// #   called_from_guild: false,
    /// #   guild_info: Some((String::new(), one)),
    /// # };
    /// # let logger = CommandLogger::new();
    /// let starting_len = logger.len().await;
    /// logger.enqueue(command_event).await;
    /// assert_eq!(logger.len().await, starting_len + 1);
    /// # });
    /// ```
    pub async fn enqueue(&self, command_event: LoggedCommandEvent) {
        self.queue.lock().await.push(command_event);
    }

    #[inline]
    async fn dequeue(&self) -> Option<LoggedCommandEvent> { self.queue.lock().await.pop() }

    #[inline]
    pub async fn len(&self) -> usize { self.queue.lock().await.len() }

    #[inline]
    pub async fn is_empty(&self) -> bool { self.len().await == 0 }

    /// Continuously logs items present in the queue. **This function never returns**
    /// unless an error occurrs.
    ///
    /// # Errors
    ///
    /// Errors may occurr if:
    /// - writing to the provided file raises an IO error
    /// - serilization raises an error
    pub async fn log_to_file(&self, path: impl AsRef<Path>) -> Result<(), LoggingError> {
        let path = path.as_ref();

        let parent_path = path.parent().unwrap();
        if !parent_path.exists() {
            fs::create_dir_all(parent_path).await?;
        }

        let mut csv_file = AsyncWriterBuilder::new()
            .has_headers(false)
            .create_serializer(File::create(path).await?);
        loop {
            while self.is_empty().await {
                // Asyncronously Sleep for some seconds to let the bot work.
                time::sleep(time::Duration::from_secs(6)).await;
            }

            while !self.is_empty().await {
                // Dequeue the command event
                let command_event = self.dequeue().await.unwrap();
                #[cfg(debug_assertions)]
                tracing::trace!(
                    "Serializing and writing {command_event:?} to \"{}\"",
                    path.display()
                );
                csv_file.serialize(command_event).await?;
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LoggedCommandEvent {
    /// Name of the command that was called
    pub name:              String,
    /// Time the command was called
    pub time:              chrono::DateTime<Utc>,
    /// The channel the command was called in
    pub channel_id:        NonZeroU64,
    /// User that called the command
    pub user:              (String, NonZeroU64),
    /// Was the commmand called from a guild
    pub called_from_guild: bool,
    /// The guild that called the command
    pub guild_info:        Option<(String, NonZeroU64)>,
}
