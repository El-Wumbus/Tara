use std::sync::Arc;

use async_trait::async_trait;
use serenity::{
    all::{CommandDataOptionValue, CommandInteraction, CommandOptionType},
    builder::{CreateCommand, CreateCommandOption},
    model::prelude::Guild,
    prelude::Context,
};
use tokio::sync::Mutex;

use super::DiscordCommand;
use crate::{Error, Result};

mod currency;
mod temperature;

pub static COMMAND: Conversions = Conversions;

lazy_static::lazy_static! {
    pub static ref CURRENCY_CONVERTER: Mutex<Option<currency::Converter>> = Mutex::new(None);
}

#[derive(Clone, Copy, Debug)]
pub struct Conversions;

#[async_trait]
impl DiscordCommand for Conversions {
    fn register(&self) -> CreateCommand {
        let options = vec![
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "currency",
                "Convert one currency to another, see GitHub for the supported currencies.",
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "input",
                    "The input including the currency (e.g. \"$45\" or \"8000 JPY\")",
                )
                .required(true),
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "output",
                    "The output currency (e.g. \"USD\" or \"CAD\")",
                )
                .required(true),
            ),
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "temperature",
                "Convert from one temperature unit to another. Supports Kelvin, Fahrenheit, and Celcius.",
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "value",
                    "Original value (e.g. '65F' [Fahrenheit], '18.33C' [Celsius].",
                )
                .required(true),
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "target",
                    "The unit to target. (e.g 'F' [Fahrenheit], 'K' [kelvin]).",
                )
                .required(true),
            ),
        ];

        CreateCommand::new(self.name())
            .description("Convert one unit to another")
            .dm_permission(true)
            .set_options(options)
    }

    async fn run(
        &self,
        _context: &Context,
        command: &CommandInteraction,
        _guild: Option<Guild>,
        config: Arc<crate::config::Configuration>,
        _databases: Arc<crate::database::Databases>,
    ) -> Result<String> {
        use super::core::suboptions;
        let option = &command.data.options[0];
        match &*option.name {
            "temperature" => {
                let options = suboptions(option);
                // Get the options
                let(
                    CommandDataOptionValue::String(input),
                    CommandDataOptionValue::String(output),
                ) = (&options[0].value, &options[1].value) else { return Err(Error::InternalLogic) };
                let input = input.trim().to_lowercase();
                let output = output.trim().to_lowercase();

                // Convert and return
                temperature::convert(&input, &output)
            }
            "currency" => {
                let api_key = match config.secrets.currency_api_key.clone() {
                    None => {
                        return Err(Error::FeatureDisabled(
                            "Currency conversion is disabled on this instance. Contact the host to enable \
                             this feature."
                                .to_string(),
                        ));
                    }
                    Some(x) => x,
                };

                let options = suboptions(option);
                // Get the options
                let(
                    CommandDataOptionValue::String(input),
                    CommandDataOptionValue::String(output),
                ) = (&options[0].value, &options[1].value) else { return Err(Error::InternalLogic) };
                let input = input.trim().to_lowercase();
                let output = output.trim().to_lowercase();

                let converter = match CURRENCY_CONVERTER.lock().await.clone() {
                    Some(x) => x,
                    None => currency::Converter::new(api_key, chrono::Duration::hours(6)).await?,
                };

                let (r, c) = currency::run(converter, input, output).await?;

                // Update the currency converter
                *CURRENCY_CONVERTER.lock().await = Some(c);

                Ok(r)
            }
            _ => Err(Error::InternalLogic),
        }
    }

    fn name(&self) -> std::string::String { String::from("conversions") }
}
