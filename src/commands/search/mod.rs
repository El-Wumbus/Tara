use std::sync::Arc;

use async_trait::async_trait;
use serenity::{
    model::prelude::{
        command::CommandOptionType,
        interaction::application_command::{ApplicationCommandInteraction, CommandDataOptionValue},
    },
    prelude::Context,
};
use truncrate::TruncateToBoundary;

use super::DiscordCommand;
use crate::Error;

mod ddg;

pub static COMMAND: Search = Search;

#[derive(Clone, Copy, Debug)]
pub struct Search;

#[async_trait]
impl DiscordCommand for Search
{
    fn register<'a>(
        &'a self,
        command: &'a mut serenity::builder::CreateApplicationCommand,
    ) -> &mut serenity::builder::CreateApplicationCommand
    {
        command
            .name(self.name())
            .description("Search the internet")
            .dm_permission(true)
            .create_option(|option| {
                option
                    .name("duckduckgo")
                    .description("Search DuckDuckGo (safe.duckduckgo.com)")
                    .kind(CommandOptionType::SubCommand)
                    .create_sub_option(|option| {
                        option
                            .name("search_term")
                            .description("The search term")
                            .kind(CommandOptionType::String)
                            .required(true)
                    })
                    .create_sub_option(|option| {
                        option
                            .name("result_count")
                            .description("The number of results to return (MIN: 1, MAX: 8)")
                            .kind(CommandOptionType::Integer)
                            .required(false)
                    })
            })
    }

    #[allow(clippy::cast_possible_truncation)]
    async fn run(
        &self,
        _context: &Context,
        command: &ApplicationCommandInteraction,
        _config: Arc<crate::config::Configuration>,
        databases: Arc<crate::database::Databases>,
    ) -> crate::Result<String>
    {
        let option = &command.data.options[0];
        match &*option.name {
            "duckduckgo" => {
                let mut search_term = None;
                let mut result_count = 2;

                if let Some(CommandDataOptionValue::String(input)) = &option.options[0].resolved {
                    search_term = Some(input.trim().to_lowercase());
                }
                if let Some(x) = option.options.get(1) {
                    if let Some(CommandDataOptionValue::Integer(count)) = x.resolved {
                        result_count = count as usize;
                    }
                }

                let Some(search_term) = search_term else { return Err(Error::InternalLogic) };
                let (results, url) = ddg::scrape(&search_term, result_count).await?;

                // Get `result_count` number of results, create a string from it, then append a
                // newline to the end.
                let mut content = results
                    .into_iter()
                    .map(|x| {
                        let mut x = x.to_string();
                        x.push('\n');
                        x
                    })
                    .collect::<String>();

                if content.is_empty() {
                    return Err(Error::NoSearchResults(search_term));
                }
                let max = super::core::get_max_content_len(command, &databases)?;
                // Truncate content.
                if content.len() >= max {
                    content = format!("{}…\n{url}", content.truncate_to_boundary(max));
                }
                return Ok(content);
            }
            _ => return Err(Error::InternalLogic),
        }
    }

    fn name(&self) -> String { String::from("search") }
}
