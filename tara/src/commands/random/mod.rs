//! Produce pseudo-random outcomes


use async_trait::async_trait;
use rand::Rng;
use serenity::{
    all::CommandOptionType,
    builder::{
        CreateAttachment, CreateCommand, CreateCommandOption, CreateInteractionResponse,
        CreateInteractionResponseMessage,
    },
};

use self::images::Image;
use super::{CommandArguments, DiscordCommand};
use crate::{Error, Result};

mod images;
mod quote;

pub static COMMAND: Random = Random;

#[derive(Clone, Copy, Debug)]
pub struct Random;

#[async_trait]
impl DiscordCommand for Random {
    fn register(&self) -> CreateCommand {
        let options = vec![
            CreateCommandOption::new(CommandOptionType::SubCommand, "coin", "Flip a coin"),
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "quote",
                "Request a random quote from the internet",
            ),
            CreateCommandOption::new(CommandOptionType::SubCommand, "dog", "Get a random dog photo"),
            CreateCommandOption::new(CommandOptionType::SubCommand, "cat", "Get a random cat photo"),
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
                ),
        ];

        CreateCommand::new(self.name())
            .description("Define an english word")
            .dm_permission(true)
            .set_options(options)
    }

    async fn run(&self, args: CommandArguments) -> Result<String> {
        let option = &args.command.data.options[0];
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

                let response = CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new().add_file(attachment),
                );
                if let Err(e) = args.command.create_response(&args.context.http, response).await {
                    log::error!("Couldn't respond to command: {e}");
                }

                Ok(String::new())
            }
            "number" => {
                let mut low = 0.0;
                let mut high = 1_000_000.0;
                let mut integer = false;

                for option in super::core::suboptions(option) {
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
            _ => Err(Error::InternalLogic),
        }
    }

    fn name(&self) -> String { String::from("random") }
}

/// Flip a coin
///
/// # Usage
///
/// ```Rust
/// dbg!(coin_flip());
/// ```
fn coin_flip() -> String {
    let mut rng = rand::thread_rng();

    if rng.gen_bool(1.0 / 2.0) {
        String::from("Heads")
    }
    else {
        String::from("Tails")
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
fn random_number(low: f64, high: f64, integer: bool) -> String {
    let mut rng = rand::thread_rng();

    if integer {
        rng.gen_range(low as i64..=high as i64).to_string()
    }
    else {
        rng.gen_range(low..=high).to_string()
    }
}
