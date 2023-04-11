use serenity::all::CommandInteraction;

use crate::Result;


pub fn maximum_content_output_chars(
    command: &CommandInteraction,
    databases: &crate::database::Databases,
) -> Result<String> {
    let max = super::super::core::get_max_content_len(command, databases)?;
    Ok(format!("maximum_content_output_chars = {max}"))
}
