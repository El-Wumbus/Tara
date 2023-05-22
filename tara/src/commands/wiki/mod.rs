use async_trait::async_trait;
use serenity::{
    all::{CommandDataOptionValue, CommandOptionType},
    builder::{CreateCommand, CreateCommandOption, CreateEmbed},
};
use truncrate::TruncateToBoundary;

use super::{CommandArguments, DiscordCommand};
use crate::{commands::CommandResponse, Result};

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

    async fn run(&self, args: CommandArguments) -> Result<CommandResponse> {
        use api::Page;

        let title = {
            // Get the role argument
            let mut title = None;
            if let CommandDataOptionValue::String(input) = &args.command.data.options[0].value {
                title = Some(input);
            }
            title.unwrap().trim().to_owned()
        };

        let page = Page::search(&title).await?;
        let url = page.url.clone();
        let title = page.title.clone();
        let mut content = page.get_summary().await?;

        let max =
            super::core::get_content_character_limit(args.command.guild_id, &args.guild_preferences).await?;
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
