use std::sync::Arc;

use async_trait::async_trait;
use serenity::{
    builder::CreateApplicationCommand,
    model::prelude::{
        command::CommandOptionType,
        interaction::application_command::{ApplicationCommandInteraction, CommandDataOptionValue},
        Role, RoleId,
    },
    prelude::Context,
};

use super::DiscordCommand;
use crate::{Error, Result};

pub const COMMAND: RoleCMD = RoleCMD;

pub struct RoleCMD;

#[async_trait]
impl DiscordCommand for RoleCMD
{
    /// Register the discord command.
    fn register<'a>(&'a self, command: &'a mut CreateApplicationCommand) -> &mut CreateApplicationCommand
    {
        command
            .name(self.name())
            .description("Self-manage your roles")
            .dm_permission(false)
            .create_option(|option| {
                option
                    .name("add")
                    .kind(CommandOptionType::SubCommand)
                    .description("Give yourself a role")
                    .create_sub_option(|option| {
                        option
                            .name("role")
                            .description("The role to add")
                            .kind(CommandOptionType::Role)
                            .required(true)
                    })
            })
            .create_option(|option| {
                option
                    .name("remove")
                    .kind(CommandOptionType::SubCommand)
                    .description("Remove a role from yourself")
                    .create_sub_option(|option| {
                        option
                            .name("role")
                            .description("The role to remove")
                            .kind(CommandOptionType::Role)
                            .required(true)
                    })
            })
            .create_option(|option| {
                option
                    .name("list")
                    .kind(CommandOptionType::SubCommand)
                    .description("List all self-assignable roles")
            })
    }

    /// Run the discord command
    async fn run(
        &self,
        context: &Context,
        command: &ApplicationCommandInteraction,
        _config: Arc<crate::config::Configuration>,
        databases: Arc<crate::database::Databases>,
    ) -> Result<String>
    {
        let option = &command.data.options[0];
        // We can unwrap because this command cannot run in DMs
        let guild_id = command.guild_id.unwrap();
        let roles = super::core::get_role_ids(&databases, guild_id)?;

        match &*option.name {
            "list" => {
                // We can unwrap because this command cannot run in DMs
                let guild = guild_id.to_partial_guild(&context.http).await.unwrap();
                let guild_roles = guild.roles;
                let roles: String = roles
                    .into_iter()
                    .filter_map(|role_id| guild_roles.get(&RoleId(role_id)))
                    .map(|role| &*role.name)
                    .collect::<Vec<&str>>()
                    .join(",");

                return Ok(format!("Self-assignable roles:\n> {roles}"));
            }

            "add" => {
                let role = get_role(&option.options[0].resolved);
                if !roles.contains(role.id.as_u64()) {
                    return Err(Error::RoleNotAssignable(role.name.clone()));
                }

                // We can unwrap because this command only runs in DM
                let mut member = command.member.to_owned().unwrap();

                // Add role
                member
                    .add_role(&context.http, role.id)
                    .await
                    .map_err(Error::UnableToSetUserRole)?;

                return Ok(format!("Added {}", role.name));
            }

            "remove" => {
                let role = get_role(&option.options[0].resolved);
                if !roles.contains(role.id.as_u64()) {
                    return Err(Error::RoleNotAssignable(role.name.clone()));
                }

                // We can unwrap because this command only runs in DM
                let mut member = command.member.to_owned().unwrap();

                // Remove role
                member
                    .remove_role(&context.http, role.id)
                    .await
                    .map_err(Error::UnableToSetUserRole)?;

                return Ok(format!("Added {}", role.name));
            }

            _ => return Err(Error::InternalLogic),
        }

        fn get_role(option: &Option<CommandDataOptionValue>) -> Role
        {
            // Get the role argument
            let mut role = None;
            if let Some(CommandDataOptionValue::Role(input)) = option {
                role = Some(input);
            }
            role.unwrap().to_owned()
        }
    }

    /// The name of the command
    fn name(&self) -> String { String::from("role") }
}
