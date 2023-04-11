use std::{num::NonZeroU64, sync::Arc};

use async_trait::async_trait;
use serenity::{
    all::{CommandDataOptionValue, CommandInteraction, CommandOptionType, RoleId},
    builder::{CreateCommand, CreateCommandOption},
    model::prelude::Guild,
    prelude::Context,
};

use super::DiscordCommand;
use crate::{Error, Result};

pub const COMMAND: RoleCMD = RoleCMD;

pub struct RoleCMD;

#[async_trait]
impl DiscordCommand for RoleCMD {
    fn register(&self) -> CreateCommand {
        let options = vec![
            CreateCommandOption::new(CommandOptionType::SubCommand, "add", "Give yourself a role")
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::Role, "role", "The role to add")
                        .required(true),
                ),
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "remove",
                "Remove a role from yourself",
            )
            .add_sub_option(
                CreateCommandOption::new(CommandOptionType::Role, "role", "The role to remove")
                    .required(true),
            ),
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "list",
                "List all self-assignable roles",
            ),
        ];

        CreateCommand::new(self.name())
            .description("Self-manage your roles")
            .dm_permission(false)
            .set_options(options)
    }

    /// Run the discord command
    async fn run(
        &self,
        context: &Context,
        command: &CommandInteraction,
        guild: Option<Guild>,
        _config: Arc<crate::config::Configuration>,
        databases: Arc<crate::database::Databases>,
    ) -> Result<String> {
        let option = &command.data.options[0];
        let guild = guild.unwrap();
        let roles = super::core::get_role_ids(&databases, guild.id)?;

        match &*option.name {
            "list" => {
                // We can unwrap because this command cannot run in DMs
                let guild = guild.id.to_partial_guild(&context.http).await.unwrap();
                let guild_roles = guild.roles;
                let roles: String = roles
                    .into_iter()
                    .filter_map(|role_id| guild_roles.get(&RoleId(NonZeroU64::new(role_id).unwrap())))
                    .map(|role| &*role.name)
                    .collect::<Vec<&str>>()
                    .join(",");

                return Ok(format!("Self-assignable roles:\n> {roles}"));
            }

            "add" => {
                let role = {
                    let CommandDataOptionValue::Role(role_id) = super::core::suboptions(option)[0].value else {return Err(crate::Error::InternalLogic)};
                    guild.roles.get(&role_id).unwrap()
                };
                if !roles.contains(&u64::from(*role.id.as_inner())) {
                    return Err(Error::RoleNotAssignable(role.name.clone()));
                }

                // We can unwrap because this command only runs in DM
                let mut member = command.member.to_owned().unwrap();

                // Add role
                member
                    .add_role(&context.http, role.id)
                    .await
                    .map_err(Error::UserRole)?;

                return Ok(format!("Added {}", role.name));
            }

            "remove" => {
                let role = {
                    let CommandDataOptionValue::Role(role_id) = super::core::suboptions(option)[0].value else {return Err(crate::Error::InternalLogic)};
                    guild.roles.get(&role_id).unwrap()
                };

                if !roles.contains(&u64::from(*role.id.as_inner())) {
                    return Err(Error::RoleNotAssignable(role.name.clone()));
                }

                // We can unwrap because this command only runs in DM
                let mut member = command.member.to_owned().unwrap();

                // Remove role
                member
                    .remove_role(&context.http, role.id)
                    .await
                    .map_err(Error::UserRole)?;

                return Ok(format!("Added {}", role.name));
            }

            _ => return Err(Error::InternalLogic),
        }
    }

    /// The name of the command
    fn name(&self) -> String { String::from("role") }
}
