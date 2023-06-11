use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use once_cell::sync::Lazy;
use serenity::{
    all::{ChannelId, CommandInteraction, CommandOptionType, ComponentInteraction, MessageId, ReactionType},
    builder::{
        CreateActionRow, CreateButton, CreateCommand, CreateCommandOption, CreateEmbed,
        CreateInteractionResponse, CreateInteractionResponseMessage, EditInteractionResponse,
    },
    client::Cache,
    http::Http,
};
use tokio::sync::Mutex;
use truncrate::TruncateToBoundary;

use super::{common::unsplash, CommandArguments, CommandResponse, DiscordCommand};
use crate::{
    componet::{CleanupFn, ComponentFn},
    Error, Result,
};

mod ddg;

pub const COMMAND: Search = Search;

#[allow(clippy::type_complexity)]
static IMAGE_RESULTS: Lazy<
    Arc<
        Mutex<
            HashMap<(ChannelId, MessageId), (Vec<unsplash::UnsplashImage>, usize, Arc<CommandInteraction>)>,
        >,
    >,
> = Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));


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
                            "color" => color = option.value.as_str().map(|x| x.to_string()),
                            "orientation" => orientation = option.value.as_str().map(|x| x.to_string()),
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
                let id = format!("{}/{}", command.channel_id, message.id);


                let components = vec![CreateActionRow::Buttons(vec![
                    CreateButton::new(format!("{id}-prev")).emoji(ReactionType::Unicode(String::from("⬅️"))),
                    CreateButton::new(format!("{id}-next"))
                        .emoji(ReactionType::Unicode(String::from("➡️")))
                        .label(format!("Next (1/{})", images.len())),
                ])];

                // Finally send the buttons
                command
                    .edit_response(
                        &args.context.http,
                        EditInteractionResponse::new().components(components),
                    )
                    .await?;

                IMAGE_RESULTS
                    .lock()
                    .await
                    .insert((command.channel_id, message.id), (images, 0, command.clone()));

                args.component_map
                    .insert(
                        format!("{id}-next"),
                        ComponentFn::new(next_handler),
                        Some(CleanupFn::new(buttons_cleanup_handler)),
                    )
                    .await;
                args.component_map
                    .insert(
                        format!("{id}-prev"),
                        ComponentFn::new(prev_handler),
                        Some(CleanupFn::new(buttons_cleanup_handler)),
                    )
                    .await;


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

async fn next_handler(args: (ComponentInteraction, CommandArguments)) -> Result<()> {
    button_handler(args, |x| x + 1).await
}

async fn prev_handler(args: (ComponentInteraction, CommandArguments)) -> Result<()> {
    button_handler(args, |x| x - 1).await
}

async fn button_handler(args: (ComponentInteraction, CommandArguments), f: fn(isize) -> isize) -> Result<()> {
    let (component, args) = args;
    let mut lock = IMAGE_RESULTS.lock().await;
    let (imgs, mut i, _) = lock.get(&(component.channel_id, component.message.id)).unwrap();
    let mut x = f(i as isize);

    if x >= imgs.len() as isize {
        x = 0;
    } else if x < 0 {
        x = imgs.len() as isize - 1 as isize;
    }
    i = x as usize;

    let id = format!("{}/{}", component.channel_id, component.message.id);
    let components = vec![CreateActionRow::Buttons(vec![
        CreateButton::new(format!("{id}-prev")).emoji(ReactionType::Unicode(String::from("⬅️"))),
        CreateButton::new(format!("{id}-next"))
            .emoji(ReactionType::Unicode(String::from("➡️")))
            .label(format!("Next ({}/{})", i + 1, imgs.len())),
    ])];

    let image = imgs.get(i).unwrap();
    let embed: CreateEmbed = image.into();
    component
        .create_response(
            &args.context.http,
            CreateInteractionResponse::UpdateMessage(
                CreateInteractionResponseMessage::new()
                    .embed(embed)
                    .components(components),
            ),
        )
        .await?;

    let (_, ref mut n, _) = lock
        .get_mut(&(component.channel_id, component.message.id))
        .unwrap();
    *n = i;

    Ok(())
}

async fn buttons_cleanup_handler(args: (String, Arc<Http>, Arc<Cache>)) -> Result<()> {
    let x = args
        .0
        .split('/')
        .map(|x| x.parse::<u64>().unwrap())
        .collect::<Vec<_>>();

    if let Some((imgs, i, command)) = IMAGE_RESULTS
        .lock()
        .await
        .remove(&(ChannelId::new(x[0]), MessageId::new(x[1])))
    {
        let message = command.get_response(&args.1).await?;
        let id = format!("{}/{}", command.channel_id, message.id);
        let components = vec![CreateActionRow::Buttons(vec![
            CreateButton::new(format!("{id}-prev"))
                .emoji(ReactionType::Unicode(String::from("⬅️")))
                .disabled(true),
            CreateButton::new(format!("{id}-next"))
                .emoji(ReactionType::Unicode(String::from("➡️")))
                .disabled(true)
                .label(format!("Next ({}/{})", i + 1, imgs.len())),
        ])];

        command
            .edit_response(&args.1, EditInteractionResponse::new().components(components))
            .await?;
    }
    Ok(())
}
