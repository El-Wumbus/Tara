use std::{fmt::Write, sync::Arc};

use async_trait::async_trait;
use serenity::{
    all::{CommandDataOptionValue, CommandInteraction, CommandOptionType, RoleId},
    builder::{CreateCommand, CreateCommandOption, CreateEmbed},
};

use super::{common::ExistingRole, CommandArguments, CommandResponse, DiscordCommand};
use crate::{Error, IdUtil, Result};

pub const COMMAND: Role = Role;

pub struct Role;

#[async_trait]
impl DiscordCommand for Role {
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
    async fn run(&self, command: Arc<CommandInteraction>, args: CommandArguments) -> Result<CommandResponse> {
        let option = &command.data.options[0];
        let guild = args.guild.ok_or_else(|| Error::InternalLogic)?;

        let ids = sqlx::query_as!(
            ExistingRole,
            "SELECT id FROM roles WHERE guild_id = $1",
            guild.id.toint(),
        )
        .fetch_all(&args.database)
        .await?
        .into_iter()
        .map(ExistingRole::id)
        .collect::<Vec<RoleId>>();

        match &*option.name {
            "list" => {
                let mut description = String::new();
                for (i, id) in ids.iter().copied().enumerate() {
                    if let Some(role) = guild.roles.get(&id) {
                        let emoji = role.unicode_emoji.clone().map_or_else(String::new, |e| e + " ");
                        write!(&mut description, "{emoji}{}", role.name).unwrap();

                        if i != ids.len() - 1 {
                            write!(&mut description, ", ").unwrap();
                        }
                    }
                }

                let roles = CreateEmbed::new().title("Roles").description(description);
                Ok(CommandResponse::Embed(roles.into()))
            }

            "add" | "remove" => {
                let role = {
                    let CommandDataOptionValue::Role(role_id) = super::common::suboptions(option)[0].value
                    else {
                        return Err(crate::Error::InternalLogic);
                    };
                    guild.roles.get(&role_id).unwrap()
                };

                if !ids.into_iter().any(|x| x == role.id) {
                    return Err(Error::RoleNotAssignable(role.name.clone()));
                }

                // We can unwrap because this command only runs in guilds.
                let mut member = command.member.clone().unwrap();

                match &*option.name {
                    "add" => {
                        member
                            .add_role(&args.context.http, role.id)
                            .await
                            .map_err(|e| Error::UserRole(Box::new(e)))?;

                        Ok(format!("Added {}", role.name).into())
                    }
                    "remove" => {
                        member
                            .remove_role(&args.context.http, role.id)
                            .await
                            .map_err(|e| Error::UserRole(Box::new(e)))?;

                        Ok(format!("Removed {}", role.name).into())
                    }
                    _ => unreachable!(),
                }
            }

            _ => return Err(Error::InternalLogic),
        }
    }

    /// The name of the command
    fn name(&self) -> &'static str { "role" }
}
