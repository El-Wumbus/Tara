use std::sync::Arc;

use async_trait::async_trait;
use serenity::{
    all::{CommandInteraction, CommandOptionType},
    builder::{CreateCommand, CreateCommandOption},
    model::prelude::Guild,
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
impl DiscordCommand for Search {
    fn register(&self) -> CreateCommand {
        let options = vec![CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "duckduckgo",
            "Search DuckDuckGo (duckduckgo.com/html)",
        )
        .add_sub_option(
            CreateCommandOption::new(CommandOptionType::String, "search_term", "The search term")
                .required(true),
        )
        .add_sub_option(
            CreateCommandOption::new(
                CommandOptionType::Integer,
                "result_count",
                "The number of results to return (MIN: 1, MAX: 8)",
            )
            .required(false),
        )];

        CreateCommand::new(self.name())
            .description("Search the internet")
            .dm_permission(true)
            .set_options(options)
    }

    #[allow(clippy::cast_possible_truncation)]
    async fn run(
        &self,
        _context: &Context,
        command: &CommandInteraction,
        _guild: Option<Guild>,
        _config: Arc<crate::config::Configuration>,
        databases: Arc<crate::database::Databases>,
    ) -> crate::Result<String> {
        let option = &command.data.options[0];
        match &*option.name {
            "duckduckgo" => {
                let mut search_term = None;
                let mut result_count = 2;

                for option in super::core::suboptions(option) {
                    match &*option.name {
                        "search_term" => search_term = Some(option.value.as_str().unwrap().to_string()),
                        "result_count" => result_count = option.value.as_i64().unwrap() as usize,
                        _ => (),
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
