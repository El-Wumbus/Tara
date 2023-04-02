use std::sync::Arc;

use async_trait::async_trait;
use serenity::{
    builder::CreateApplicationCommand,
    model::prelude::{
        command::CommandOptionType, interaction::application_command::ApplicationCommandInteraction,
    },
    prelude::Context,
};
use truncrate::TruncateToBoundary;

use super::DiscordCommand;
use crate::{config, Error, Result};

mod api;

pub static COMMAND: Wiki = Wiki;

#[derive(Clone, Copy, Debug)]
pub struct Wiki;

impl Wiki
{
    pub const DEFAULT_MAX_WIKI_LEN: u32 = 800;
}

#[async_trait]
impl DiscordCommand for Wiki
{
    fn register<'a>(&'a self, command: &'a mut CreateApplicationCommand) -> &mut CreateApplicationCommand
    {
        command
            .name(self.name())
            .description("Get a summary of a topic from wikipedia.org")
            .dm_permission(true)
            .create_option(|option| {
                option
                    .name("title")
                    .description("The title to search wikipedia.org for")
                    .kind(CommandOptionType::String)
                    .required(true)
            })
    }

    async fn run(
        &self,
        _context: &Context,
        command: &ApplicationCommandInteraction,
        _config: Arc<config::Configuration>,
        databases: Arc<crate::database::Databases>,
    ) -> Result<String>
    {
        use api::Page;
        let mut title = String::new();

        for option in &command.data.options {
            match &*option.name {
                "title" => {
                    title = option
                        .value
                        .clone()
                        .unwrap_or_default()
                        .as_str()
                        .unwrap_or_default()
                        .to_string();
                }
                _ => return Err(Error::InternalLogic),
            }
        }

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
