use std::sync::Arc;

use async_trait::async_trait;
use serenity::{
    model::prelude::{
        command::CommandOptionType,
        interaction::application_command::{ApplicationCommandInteraction, CommandDataOptionValue},
    },
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
    fn register<'a>(
        &'a self,
        command: &'a mut serenity::builder::CreateApplicationCommand,
    ) -> &mut serenity::builder::CreateApplicationCommand {
        command
            .name(self.name())
            .description("Convert one unit to another")
            .dm_permission(true)
            .create_option(|option| {
                option
                    .name("currency")
                    .description("Convert one currency to another, see GitHub for the supported currencies.")
                    .kind(CommandOptionType::SubCommand)
                    .create_sub_option(|option| {
                        option
                            .name("input")
                            .description("The input including the currency (e.g. \"$45\" or \"8000 JPY\")")
                            .kind(CommandOptionType::String)
                            .required(true)
                    })
                    .create_sub_option(|option| {
                        option
                            .name("output")
                            .description("The output currency (e.g. \"USD\" or \"CAD\")")
                            .kind(CommandOptionType::String)
                            .required(true)
                    })
            })
            .create_option(|option| {
                option
                    .name("temperature")
                    .kind(CommandOptionType::SubCommand)
                    .description(
                        "Convert from one temperature unit to another. Supports Kelvin, Fahrenheit, and \
                         Celcius.",
                    )
                    .create_sub_option(|option| {
                        option
                            .name("value")
                            .description("Original value (e.g. '65F' [Fahrenheit], '18.33C' [Celsius].")
                            .kind(CommandOptionType::String)
                            .required(true)
                    })
                    .create_sub_option(|option| {
                        option
                            .name("target")
                            .description("The unit to target. (e.g 'F' [Fahrenheit], 'K' [kelvin]).")
                            .kind(CommandOptionType::String)
                            .required(true)
                    })
            })
    }

    async fn run(
        &self,
        _context: &Context,
        command: &ApplicationCommandInteraction,
        config: Arc<crate::config::Configuration>,
        _databases: Arc<crate::database::Databases>,
    ) -> Result<String> {
        let option = &command.data.options[0];
        match &*option.name {
            "temperature" => {
                let mut input = None;
                let mut output = None;

                // Get the options
                if let (
                    Some(CommandDataOptionValue::String(inp)),
                    Some(CommandDataOptionValue::String(out)),
                ) = (&option.options[0].resolved, &option.options[1].resolved)
                {
                    input = Some(inp.trim().to_lowercase());
                    output = Some(out.trim().to_lowercase());
                }
                let (Some(input), Some(output)) = (input, output) else { return Err(Error::InternalLogic) };

                // Convert and return
                return temperature::convert(&input, &output);
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

                let mut input = None;
                let mut output = None;

                // Get the options
                if let (
                    Some(CommandDataOptionValue::String(inp)),
                    Some(CommandDataOptionValue::String(out)),
                ) = (&option.options[0].resolved, &option.options[1].resolved)
                {
                    input = Some(inp.trim().to_lowercase());
                    output = Some(out.trim().to_lowercase());
                }
                let (Some(input), Some(output)) = (input, output) else { return Err(Error::InternalLogic) };

                let converter = match CURRENCY_CONVERTER.lock().await.clone() {
                    Some(x) => x,
                    None => currency::Converter::new(api_key, chrono::Duration::hours(6)).await?,
                };

                let (r, c) = currency::run(converter, input, output).await?;

                // Update the currency converter
                *CURRENCY_CONVERTER.lock().await = Some(c);

                return Ok(r);
            }
            _ => return Err(Error::InternalLogic),
        }
    }

    fn name(&self) -> std::string::String { String::from("conversions") }
}
