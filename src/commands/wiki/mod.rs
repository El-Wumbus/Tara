use std::sync::Arc;

use async_trait::async_trait;
use serenity::{
    all::{CommandDataOptionValue, CommandInteraction, CommandOptionType, Guild},
    builder::{CreateCommand, CreateCommandOption},
    prelude::Context,
};
use truncrate::TruncateToBoundary;

use super::DiscordCommand;
use crate::{config, Result};

mod api;

pub const COMMAND: Wiki = Wiki;

#[derive(Clone, Copy, Debug)]
pub struct Wiki;

impl Wiki {
    pub const DEFAULT_MAX_WIKI_LEN: u32 = 800;
}

#[async_trait]
impl DiscordCommand for Wiki {
    fn register(&self) -> CreateCommand {
        let options = vec![CreateCommandOption::new(
            CommandOptionType::String,
            "title",
            "The title to search wikipedia.org for",
        )
        .required(true)];

        CreateCommand::new(self.name())
            .description("Get a summary of a topic from wikipedia.org")
            .dm_permission(true)
            .set_options(options)
    }

    async fn run(
        &self,
        _context: &Context,
        command: &CommandInteraction,
        _guild: Option<Guild>,
        _config: Arc<config::Configuration>,
        databases: Arc<crate::database::Databases>,
    ) -> Result<String> {
        use api::Page;

        let title = {
            // Get the role argument
            let mut title = None;
            if let CommandDataOptionValue::String(input) = &command.data.options[0].value {
                title = Some(input);
            }
            title.unwrap().trim().to_owned()
        };

        let page = Page::search(&title).await?;
        let url = &page.url.clone();
        let summary = page.get_summary();
        let mut content = summary.await?;

        let max = super::core::get_max_content_len(command, &databases)?;
        // Truncate wiki content.
        if content.len() >= max {
            content = format!("{}…", content.truncate_to_boundary(max));
        }

        Ok(format!("{content}\n{url}"))
    }

    fn name(&self) -> String { String::from("wikipedia") }
}
