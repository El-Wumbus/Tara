//! Produce pseudo-random outcomes
use rand::Rng;
use serenity::model::prelude::command::CommandOptionType;

use super::{
    async_trait, config, ApplicationCommandInteraction, Arc, Context, CreateApplicationCommand,
    DiscordCommand, Error,
};
use crate::Result;

mod images;
mod quote;

pub static COMMAND: Random = Random;

#[derive(Clone, Copy, Debug)]
pub struct Random;

#[async_trait]
impl DiscordCommand for Random {
    fn register<'a>(&'a self, command: &'a mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
        command
            .name(self.name())
            .dm_permission(true)
            .description("Pseudo-Randomness")
            .create_option(|option| {
                option
                    .name("coin")
                    .kind(CommandOptionType::SubCommand)
                    .description("Flip a coin")
            })
            .create_option(|option| {
                option
                    .name("number")
                    .kind(CommandOptionType::SubCommand)
                    .description("Random Number Generator")
                    .create_sub_option(|option| {
                        option
                            .name("low")
                            .kind(CommandOptionType::Number)
                            .description("The low bound, inclusive")
                            .required(false)
                    })
                    .create_sub_option(|option| {
                        option
                            .name("high")
                            .kind(CommandOptionType::Number)
                            .description("The high bound, inclusive")
                            .required(false)
                    })
                    .create_sub_option(|option| {
                        option
                            .name("integer")
                            .kind(CommandOptionType::Boolean)
                            .description("Generate an integer (whole number) instead of a float (decimal)")
                            .required(false)
                    })
            })
            .create_option(|option| {
                option
                    .name("quote")
                    .description("Request a random quote from the internet")
                    .kind(CommandOptionType::SubCommand)
            })
            .create_option(|option| {
                option
                    .name("dog")
                    .description("Get a random dog photo")
                    .kind(CommandOptionType::SubCommand)
            })
            .create_option(|option| {
                option
                    .name("cat")
                    .description("Get a random cat photo")
                    .kind(CommandOptionType::SubCommand)
            })
    }

    async fn run(
        &self,
        _context: &Context,
        command: &ApplicationCommandInteraction,
        _config: Arc<config::Configuration>,
        _databases: Arc<crate::database::Databases>,
    ) -> Result<String> {
        for option in &command.data.options {
            if matches!(option.kind, CommandOptionType::SubCommand) {
                match &*option.name {
                    "coin" => return Ok(coin_flip()),
                    "quote" => return quote::random().await,
                    "cat" => return images::random_cat().await,
                    "dog" => return images::random_dog().await,
                    "number" => {
                        let mut low = 0.0;
                        let mut high = 1_000_000.0;
                        let mut integer = false;

                        for option in &option.options {
                            match &*option.name {
                                "low" => {
                                    low = option.value.clone().unwrap_or_default().as_f64().unwrap_or(low);
                                }
                                "high" => {
                                    high = option.value.clone().unwrap_or_default().as_f64().unwrap_or(high);
                                }
                                "integer" => integer = true,
                                _ => return Err(Error::InternalLogic),
                            }
                        }
                        return Ok(random_number(low, high, integer));
                    }
                    _ => return Err(Error::InternalLogic),
                }
            }
        }

        Ok(String::from("WHAT? HOW DID THIS EVEN EXECUTE?"))
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
