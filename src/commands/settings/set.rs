use serenity::{
    all::{CommandDataOption, CommandDataOptionValue, GuildId},
    model::prelude::Guild,
};

use crate::{database::Databases, Result};

pub fn maximum_content_output_chars(
    databases: &Databases,
    option: &CommandDataOption,
    guild_id: GuildId,
) -> Result<String> {
    let option = &super::super::core::suboptions(option)[0];

    // Get the first option (there's only one, and it's required), then get an
    // integer from it. We clamp this so that the command output won't be less than
    // 80 chars and so it's far from discord's character limit.
    let count = option.value.as_i64().unwrap_or_default().clamp(80, 1900) as u32;

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
                u64::from(*guild_id.as_inner())
            ),
            [],
        )
        .map_err(crate::Error::from)?;


    Ok(format!("Set maximum_content_output_chars to \"{count}\""))
}

pub fn update_self_assignable_role(
    databases: &Databases,
    option: &CommandDataOption,
    guild: Guild,
    remove: bool,
) -> Result<String> {
    let option = &super::super::core::suboptions(option)[0];
    let CommandDataOptionValue::Role(role_id) = option.value else {return Err(crate::Error::InternalLogic)};

    let role = { guild.roles.get(&role_id).unwrap() };
    if !databases.contains("guilds", guild.id)? {
        // Insert default data
        databases.guilds_insert_default(guild.id)?;
    }

    // Get the current roles and push to it.
    let mut self_assignable_roles = super::super::core::get_role_ids(databases, guild.id)?;
    let name = role.name.clone();
    if remove {
        if !self_assignable_roles.remove(&u64::from(*role_id.as_inner())) {
            return Err(crate::Error::CommandMisuse(format!(
                "Role \"{name}\" is **not** part of the *Self-assignable roles* list"
            )));
        }
    }
    else {
        self_assignable_roles.insert(u64::from(*role_id.as_inner()));
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
                u64::from(*guild.id.as_inner())
            ),
            [],
        )
        .map_err(crate::Error::DatabaseAccess)?;

    Ok(format!("Added *self assignable role* \"{name}\""))
}
