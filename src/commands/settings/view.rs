use serenity::model::prelude::{interaction::application_command::ApplicationCommandInteraction, GuildId};

use crate::{database::Databases, Result};


pub fn maximum_content_output_chars(
    command: &ApplicationCommandInteraction,
    databases: &crate::database::Databases,
) -> Result<String>
{
    let max = super::super::core::get_max_content_len(command, databases)?;
    Ok(format!("maximum_content_output_chars = {max}"))
}

pub fn self_assignable_roles(databases: &Databases, guild_id: GuildId) -> Result<String>
{
    let connection = databases
        .guilds
        .get()
        .map_err(crate::Error::DatabaseAccessTimeout)?;

    let mut statment = connection
        .prepare(&format!(
            "SELECT assignable_roles FROM guilds WHERE GuildID={}",
            guild_id.as_u64()
        ))
        .map_err(crate::Error::from)?;

    let mut self_assignable_roles = statment
        .query_map([], |row| row.get(0))
        .map_err(crate::Error::from)?;

    if let Some(self_assignable_roles) = self_assignable_roles.next() {
        // `self_assignable_roles` is stored in a BLOB so we get that and deserialize
        // it.
        let self_assignable_roles: Vec<u8> = self_assignable_roles.map_err(crate::Error::from)?;
        let self_assignable_roles: Vec<String> = bincode::deserialize(&self_assignable_roles).unwrap();
        Ok(format!("self_assignable_roles = {self_assignable_roles:#?}"))
    }
    else {
        Err(crate::Error::NoDatabaseRecord)
    }
}
