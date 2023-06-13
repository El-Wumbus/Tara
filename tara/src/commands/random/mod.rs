//! Produce pseudo-random outcomes


use std::sync::Arc;

use async_trait::async_trait;
use rand::Rng;
use serde::{Deserialize, Serialize};
use serenity::{
    all::{CommandInteraction, CommandOptionType},
    builder::{
        CreateAttachment, CreateCommand, CreateCommandOption, CreateEmbed, CreateInteractionResponseMessage,
    },
};

use self::images::Image;
use super::{common::unsplash, CommandArguments, CommandResponse, DiscordCommand};
use crate::{Error, Result};

mod emoji;
mod images;
mod quote;

pub const COMMAND: Random = Random;

#[derive(Clone, Copy, Debug)]
pub struct Random;

#[async_trait]
impl DiscordCommand for Random {
    fn register(&self) -> CreateCommand {
        let image = CreateCommandOption::new(CommandOptionType::SubCommand, "image", "Get a random image");
        let coin = CreateCommandOption::new(CommandOptionType::SubCommand, "coin", "Flip a coin");
        let quote = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "quote",
            "Request a random quote from the internet",
        );
        let emoji = CreateCommandOption::new(CommandOptionType::SubCommand, "emoji", "Get a random Emoji");
        let dog = CreateCommandOption::new(CommandOptionType::SubCommand, "dog", "Get a random dog photo");
        let cat = CreateCommandOption::new(CommandOptionType::SubCommand, "cat", "Get a random cat photo");
        let fact = CreateCommandOption::new(CommandOptionType::SubCommand, "fact", "Get a random fun fact");
        let number =
            CreateCommandOption::new(CommandOptionType::SubCommand, "number", "Random Number Generator")
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::Number, "low", "The low bound, inclusive")
                        .required(false),
                )
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::Number, "high", "The high bound, inclusive")
                        .required(false),
                )
                .add_sub_option(
                    CreateCommandOption::new(
                        CommandOptionType::Boolean,
                        "integer",
                        "Generate an integer (whole number) instead of a float (decimal)",
                    )
                    .required(false),
                );

        let options = vec![image, coin, quote, dog, cat, number, fact, emoji];

        CreateCommand::new(self.name())
            .description("Define an english word")
            .dm_permission(true)
            .set_options(options)
    }

    async fn run(&self, command: Arc<CommandInteraction>, args: CommandArguments) -> Result<CommandResponse> {
        let option = &command.data.options[0];
        match &*option.name {
            "coin" => Ok(coin_flip()),
            "quote" => quote::random().await,
            "cat" | "dog" => {
                // Get the image url
                let url = match &*option.name {
                    "cat" => Image::from(images::CatImage::random().await?).link,
                    "dog" => Image::from(images::DogImage::random().await?).link,
                    _ => unreachable!(),
                };

                // Create attachment from image and respond to command. We're downloading the image just
                // to upload it again to discord because discord began to have issues embeding the links.
                let attachment = CreateAttachment::url(&args.context.http, &url)
                    .await
                    .map_err(|e| Error::SerenityHttpRequest(Box::new(e)))?;

                Ok(CommandResponse::Message(
                    CreateInteractionResponseMessage::new().add_file(attachment),
                ))
            }
            "number" => {
                let mut low = 0.0;
                let mut high = 1_000_000.0;
                let mut integer = false;

                for option in super::common::suboptions(option) {
                    match &*option.name {
                        "low" => {
                            low = option.value.as_f64().unwrap_or(low);
                        }
                        "high" => {
                            high = option.value.as_f64().unwrap_or(high);
                        }
                        "integer" => integer = true,
                        _ => return Err(Error::InternalLogic),
                    }
                }
                Ok(random_number(low, high, integer))
            }
            "image" => {
                let Some(api_key) = args.config.secrets.unsplash_key.as_ref()
                    else {return Err(Error::FeatureDisabled("Unsplash images have been disabled".to_string()))};
                let image = &unsplash::UnsplashImage::random(api_key).await?;
                let embed: CreateEmbed = image.into();

                Ok(CommandResponse::Embed(Box::new(embed)))
            }
            "emoji" => Ok(CommandResponse::String(emoji::random_emoji().await?.to_string())),
            "fact" => random_fact().await,
            _ => Err(Error::InternalLogic),
        }
    }

    fn name(&self) -> &'static str { "random" }
}

/// Flip a coin
///
/// # Usage
///
/// ```Rust
/// dbg!(coin_flip());
/// ```
fn coin_flip() -> CommandResponse {
    let mut rng = rand::thread_rng();

    if rng.gen_bool(1.0 / 2.0) {
        CommandResponse::new_string("Heads")
    } else {
        CommandResponse::new_string("Tails")
    }
}

/// Generate number between low and high, inclusive. If `integer` is true it
/// generates an integer instead of a float.
///
/// # Usage
///
/// ```Rust
/// let low = 30.0;
/// let high = 50.0;
///
/// dbg!(random_number(low, high, false));
/// ```
#[allow(clippy::cast_possible_truncation)]
fn random_number(low: f64, high: f64, integer: bool) -> CommandResponse {
    let mut rng = rand::thread_rng();

    let x = if integer {
        rng.gen_range(low as i64..=high as i64).to_string()
    } else {
        rng.gen_range(low..=high).to_string()
    };

    CommandResponse::String(x)
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Fact {
    id:         String,
    text:       String,
    source:     String,
    source_url: String,
    language:   String,
    permalink:  String,
}

impl Fact {
    async fn random() -> Result<Self> {
        const URL: &str = "https://uselessfacts.jsph.pl/api/v2/facts/random";
        Ok(reqwest::get(URL).await?.json::<Self>().await?)
    }
}

async fn random_fact() -> Result<CommandResponse> {
    let fact = Fact::random().await?;
    Ok(fact.text.into())
}
