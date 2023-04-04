use serenity::model::prelude::{
    interaction::application_command::{CommandDataOption, CommandDataOptionValue},
    GuildId,
};

use crate::{database::Databases, Result};

pub fn maximum_content_output_chars(
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


    Ok(format!("Set maximum_content_output_chars to \"{count}\""))
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
        let mut self_assignable_roles = super::super::core::get_role_ids(databases, guild_id)?;
        let name = role.name.clone();
        if remove {
            if !self_assignable_roles.remove(role.id.as_u64()) {
                return Err(crate::Error::CommandMisuse(format!(
                    "Role \"{name}\" is **not** part of the *Self-assignable roles* list"
                )));
            }
        }
        else {
            self_assignable_roles.insert(*role.id.as_u64());
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
