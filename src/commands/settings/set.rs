use serenity::model::prelude::{
    interaction::application_command::{CommandDataOption, CommandDataOptionValue},
    GuildId,
};

use crate::{database::Databases, Result};

pub fn maximum_wikipedia_output_chars(
    databases: &Databases,
    option: &CommandDataOption,
    guild_id: GuildId,
) -> Result<String>
{
    let option = &option.options[0];

    // Get the first option (there's only one, and it's required), then get an
    // integer from it. We clamp this so that the command output won't be less than
    // 80 chars and so it's far from discord's character limit.
    let count = option
        .value
        .as_ref()
        .unwrap()
        .as_i64()
        .unwrap_or_default()
        .clamp(80, 1900) as u32;

    if !databases.contains("guilds", guild_id)? {
        // Insert default data
        databases.guilds_insert_default(guild_id)?;
    }

    let _statment = databases
        .guilds
        .get()
        .map_err(crate::Error::DatabaseAccessTimeout)?
        .execute(
            &format!(
                "UPDATE guilds SET max_content_chars={count} WHERE GuildID={}",
                guild_id.as_u64()
            ),
            [],
        )
        .map_err(crate::Error::from)?;


    Ok(format!("Set maximum_wikipedia_output_chars to \"{count}\""))
}

pub fn update_self_assignable_role(
    databases: &Databases,
    option: &CommandDataOption,
    guild_id: GuildId,
    remove: bool,
) -> Result<String>
{
    let option = &option.options[0];
    if let Some(CommandDataOptionValue::Role(role)) = option.resolved.clone() {
        if !databases.contains("guilds", guild_id)? {
            // Insert default data
            databases.guilds_insert_default(guild_id)?;
        }

        // Get the current roles and push to it.
        let mut self_assignable_roles = get_self_assignable_roles(databases, guild_id)?;
        let name = role.name.clone();
        if remove {
            if let Some(i) = self_assignable_roles.iter().position(|x| *x == role.name) {
                self_assignable_roles.remove(i);
            }
            else {
                return Err(crate::Error::CommandMisuse(format!(
                    "Role \"{name}\" is **not** part of the *self assignable roles* list"
                )));
            }
        }
        else {
            self_assignable_roles.push(role.name);
        }

        // Insert data
        databases
            .guilds
            .get()
            .map_err(crate::Error::DatabaseAccessTimeout)?
            .execute(
                &format!(
                    "UPDATE guilds SET assignable_roles=X'{}' WHERE GuildID={}",
                    {
                        use hex_string::HexString;
                        let b: Vec<u8> = bincode::serialize(&self_assignable_roles).unwrap();
                        let hex = HexString::from_bytes(&b);
                        hex.as_string()
                    },
                    guild_id.as_u64()
                ),
                [],
            )
            .map_err(crate::Error::DatabaseAccess)?;

        Ok(format!("Added *self assignable role* \"{name}\""))
    }
    else {
        Err(crate::Error::InternalLogic)
    }
}

fn get_self_assignable_roles(databases: &Databases, guild_id: GuildId) -> Result<Vec<String>>
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
        Ok(self_assignable_roles)
    }
    else {
        Err(crate::Error::NoDatabaseRecord)
    }
}
