use std::sync::Arc;

use async_trait::async_trait;
use serenity::{
    all::{CommandInteraction, CommandOptionType},
    builder::{
        CreateCommand, CreateCommandOption, CreateInteractionResponse, CreateInteractionResponseMessage,
        EditInteractionResponse,
    },
};
use truncrate::TruncateToBoundary;

use super::{common::unsplash, CommandArguments, CommandResponse, DiscordCommand};
use crate::{
    componet::{CleanupFn, ComponentFn},
    Error, Result,
};

mod ddg;
mod image;

pub const COMMAND: Search = Search;

#[derive(Clone, Copy, Debug)]
pub struct Search;

#[async_trait]
impl DiscordCommand for Search {
    fn register(&self) -> CreateCommand {
        let options = vec![
            CreateCommandOption::new(
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
            ),
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "image",
                "Search for an image from the internet",
            )
            .add_sub_option(
                CreateCommandOption::new(CommandOptionType::String, "query", "The search query")
                    .required(true),
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "orientation",
                    "Filter by photo orientation. (Valid values: landscape, portrait, squarish)",
                )
                .add_string_choice("Landscape", "landscape")
                .add_string_choice("Portrait", "portrait")
                .add_string_choice("Squarish", "squarish"),
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "color",
                    "Filter results by color. See `/help` for valid values",
                )
                .add_string_choice("Black & White", "black_and_white")
                .add_string_choice("Black", "black")
                .add_string_choice("White", "white")
                .add_string_choice("Yellow", "yellow")
                .add_string_choice("Orange", "orange")
                .add_string_choice("Red", "red")
                .add_string_choice("Purple", "purple")
                .add_string_choice("Magenta", "magenta")
                .add_string_choice("Green", "green")
                .add_string_choice("Teal", "teal")
                .add_string_choice("Blue", "blue"),
            ),
        ];

        CreateCommand::new(self.name())
            .description("Search the internet")
            .dm_permission(true)
            .set_options(options)
    }

    #[allow(clippy::cast_possible_truncation)]
    async fn run(&self, command: Arc<CommandInteraction>, args: CommandArguments) -> Result<CommandResponse> {
        let option = &command.data.options[0];
        match &*option.name {
            "image" => {
                let (query, color, orientation) = {
                    // Get the role argument
                    let mut query = "";
                    let mut color = None;
                    let mut orientation = None;
                    for option in super::common::suboptions(option) {
                        match &*option.name {
                            "query" => query = option.value.as_str().unwrap(),
                            "color" => color = option.value.as_str().map(ToString::to_string),
                            "orientation" => orientation = option.value.as_str().map(ToString::to_string),
                            _ => unreachable!("How did {} get given to my bot?!", &option.name),
                        }
                    }

                    (query, color, orientation)
                };

                let Some(api_key) = args.config.secrets.unsplash_key.as_ref()
                    else {return Err(Error::FeatureDisabled("Unsplash images have been disabled".to_string()))};
                let images = unsplash::UnsplashImage::search(api_key, query, color, orientation).await?;

                let image = images
                    .get(0)
                    .ok_or(Error::NoSearchResults(format!("No search results for {query}!")))?;

                // Initially create the response because we need the MessageId for a unique identifier.
                command
                    .create_response(
                        &args.context.http,
                        CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::new().embed(image.into()),
                        ),
                    )
                    .await?;

                let message = command.get_response(&args.context.http).await?;
                let umid = (command.channel_id, message.id);
                let id = format!("{}-{}", command.channel_id, message.id);
                let components = image::button_components(&id, 0, images.len(), false);

                // Finally send the buttons
                command
                    .edit_response(
                        &args.context.http,
                        EditInteractionResponse::new().components(components.clone()),
                    )
                    .await?;

                image::IMAGE_RESULTS
                    .lock()
                    .await
                    .insert((command.channel_id, message.id), (images, 0, command.clone()));

                args.component_map
                    .insert(
                        format!("{id}-next"),
                        ComponentFn::new(|args| image::button_handler(args, |x| x + 1)),
                        Some(CleanupFn::new(image::buttons_cleanup_handler)),
                    )
                    .await;
                args.component_map
                    .insert(
                        format!("{id}-prev"),
                        ComponentFn::new(|args| image::button_handler(args, |x| x - 1)),
                        Some(CleanupFn::new(image::buttons_cleanup_handler)),
                    )
                    .await;

                let mut users_lock = image::USERS.lock().await;
                if let Some((previous_channel_id, previous_message_id)) =
                    users_lock.insert(command.user.id, umid)
                {
                    let id = format!("{previous_channel_id}-{previous_message_id}");
                    args.component_map.timeout(format!("{id}-next")).await;
                    args.component_map.timeout(format!("{id}-prev")).await;
                }


                Ok(CommandResponse::None)
            }

            "duckduckgo" => {
                let mut search_term = None;
                let mut result_count = 2;

                for option in super::common::suboptions(option) {
                    match &*option.name {
                        "search_term" => search_term = Some(option.value.as_str().unwrap().to_string()),
                        "result_count" => result_count = option.value.as_i64().unwrap().max(0) as usize,
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
                let max =
                    super::common::get_content_character_limit(command.guild_id, &args.guild_preferences)
                        .await?;
                // Truncate content.
                if content.len() >= max {
                    content = format!("{}…\n{url}", content.truncate_to_boundary(max));
                }
                return Ok(content.into());
            }
            _ => unreachable!(),
        }
    }

    fn name(&self) -> &'static str { "search" }

    fn help(&self) -> Option<String> {
        let s = r#" **Search images**
Valid arguments for Color filtering:
- `black_and_white`
- `black`
- `white`
- `yellow`
- `orange`
- `red`
- `purple`
- `magenta`
- `green`
- `teal`
- `blue`"#;
        Some(String::from(s))
    }
}
