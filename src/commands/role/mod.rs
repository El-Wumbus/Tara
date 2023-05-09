use async_trait::async_trait;
use serenity::{
    all::{CommandDataOptionValue, CommandOptionType},
    builder::{CreateCommand, CreateCommandOption},
};

use super::{CommandArguments, DiscordCommand};
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
    async fn run(&self, args: CommandArguments) -> Result<String> {
        let option = &args.command.data.options[0];
        let guild = args.guild.ok_or_else(|| Error::InternalLogic)?;
        let prefs = args
            .guild_preferences
            .get(guild.id)
            .await
            .ok_or_else(|| Error::InternalLogic)?;
        let roles = prefs
            .all_assignable_discord_roles(&args.context.http)
            .await
            .unwrap();

        match &*option.name {
            "list" => {
                let roles = roles
                    .iter()
                    .map(|role| &*role.name)
                    .collect::<Vec<&str>>()
                    .join(", ");

                return Ok(format!("Self-assignable roles:\n> {roles}"));
            }

            "add" => {
                let role = {
                    let CommandDataOptionValue::Role(role_id) = super::core::suboptions(option)[0].value else {return Err(crate::Error::InternalLogic)};
                    guild.roles.get(&role_id).unwrap()
                };
                if !roles.iter().any(|x| x.id.eq(&role.id)) {
                    return Err(Error::RoleNotAssignable(role.name.clone()));
                }

                // We can unwrap because this command only runs in DM
                let mut member = args.command.member.to_owned().unwrap();

                // Add role
                member
                    .add_role(&args.context.http, role.id)
                    .await
                    .map_err(Error::UserRole)?;

                return Ok(format!("Added {}", role.name));
            }

            "remove" => {
                let role = {
                    let CommandDataOptionValue::Role(role_id) = super::core::suboptions(option)[0].value else {return Err(crate::Error::InternalLogic)};
                    guild.roles.get(&role_id).unwrap()
                };

                if !roles.iter().any(|x| x.id.eq(&role.id)) {
                    return Err(Error::RoleNotAssignable(role.name.clone()));
                }

                // We can unwrap because this command only runs in DM
                let mut member = args.command.member.to_owned().unwrap();

                // Remove role
                member
                    .remove_role(&args.context.http, role.id)
                    .await
                    .map_err(Error::UserRole)?;

                return Ok(format!("Removed {}", role.name));
            }

            _ => return Err(Error::InternalLogic),
        }
    }

    /// The name of the command
    fn name(&self) -> String { String::from("role") }
}
