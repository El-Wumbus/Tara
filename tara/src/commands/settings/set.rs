use serenity::{
    all::{CommandDataOption, CommandDataOptionValue, GuildId},
    model::prelude::Guild,
};

use crate::{
    commands::CommandResponse,
    database::{self, GuildPreferences, SelfAssignableRole},
    Result,
};

pub async fn content_character_limit(
    guilds: &database::Guilds,
    option: &CommandDataOption,
    guild_id: GuildId,
) -> Result<CommandResponse> {
    let option = &super::super::common::suboptions(option)[0];

    // Get the first option (there's only one, and it's required), then get an
    // integer from it. We clamp this so that the command output won't be less than
    // 80 chars and so it's far from discord's character limit.
    let count = option.value.as_i64().unwrap_or_default().clamp(80, 1900) as usize;

    if !guilds.contains(guild_id).await {
        // Insert default data
        guilds.insert(GuildPreferences::default(guild_id)).await;
    }

    guilds
        .modify(guild_id, |preferences| {
            let preferences = preferences.unwrap();
            preferences.content_character_limit = count;
        })
        .await;

    // Save changes
    guilds.save().await?;
    Ok(format!("Set `content_character_limit` to \"{count}\"").into())
}

pub async fn update_self_assignable_roles(
    guilds: &database::Guilds,
    option: &CommandDataOption,
    guild: Guild,
    remove: bool,
) -> Result<CommandResponse> {
    let option = &super::super::common::suboptions(option)[0];
    let CommandDataOptionValue::Role(role_id) = option.value else {return Err(crate::Error::InternalLogic)};

    let role = { guild.roles.get(&role_id).unwrap() };
    if !guilds.contains(guild.id).await {
        // Insert default data
        guilds.insert(GuildPreferences::default(guild.id)).await;
    }

    // Get the current roles and push to it.
    let name = role.name.clone();
    if remove {
        let was_present = guilds
            .modify(guild.id, |preferences| {
                let preferences = preferences.unwrap();
                let assignable_roles = preferences.get_assignable_roles_mut();
                assignable_roles.remove(&SelfAssignableRole::new(role.id))
            })
            .await;
        if !was_present {
            return Err(crate::Error::CommandMisuse(format!(
                "Role \"{name}\" is **not** part of the *Self-assignable roles* list"
            )));
        }
    } else {
        guilds
            .modify(guild.id, |preferences| {
                let preferences = preferences.unwrap();
                let assignable_role = SelfAssignableRole::new(role.id);
                let assignable_roles = preferences.get_assignable_roles_mut();
                assignable_roles.insert(assignable_role);
            })
            .await;
    }

    // Save changes
    guilds.save().await?;
    Ok(format!("Added *self assignable role* \"{name}\"").into())
}
