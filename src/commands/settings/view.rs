use serenity::model::prelude::interaction::application_command::ApplicationCommandInteraction;

use crate::Result;


pub fn maximum_content_output_chars(
    command: &ApplicationCommandInteraction,
    databases: &crate::database::Databases,
) -> Result<String> {
    let max = super::super::core::get_max_content_len(command, databases)?;
    Ok(format!("maximum_content_output_chars = {max}"))
}
