use std::sync::Arc;

use async_trait::async_trait;
use serenity::{
    all::{CommandDataOptionValue, CommandInteraction, CommandOptionType},
    builder::{CreateCommand, CreateCommandOption, CreateEmbed},
};
use truncrate::TruncateToBoundary;

use super::{CommandArguments, DiscordCommand};
use crate::{commands::CommandResponse, defaults, Result};

mod api;

pub const COMMAND: Wiki = Wiki;

#[derive(Clone, Copy, Debug)]
pub struct Wiki;

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
        command: Arc<CommandInteraction>,
        _args: CommandArguments,
    ) -> Result<CommandResponse> {
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
        let url = page.url.clone();
        let title = page.title.clone();
        let mut content = page.get_summary().await?;

        let max = defaults::content_character_limit_default();
        // Truncate wiki content.
        if content.len() >= max {
            content = format!("{}…", content.truncate_to_boundary(max));
        }

        // Create an embed from everything
        let embed = CreateEmbed::new()
            .title(title.to_string())
            .description(content)
            .url(url.to_string());

        Ok(CommandResponse::Embed(Box::new(embed)))
    }

    fn name(&self) -> &'static str { "wikipedia" }
}
