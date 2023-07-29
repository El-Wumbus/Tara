use std::sync::Arc;

use async_trait::async_trait;
use serenity::{
    all::{CommandDataOptionValue, CommandInteraction, CommandOptionType},
    builder::{CreateCommand, CreateCommandOption},
    model::Permissions,
};

use super::{CommandArguments, CommandResponse, DiscordCommand};
use crate::{commands::common::ExistingRole, Error, IdUtil};

pub const COMMAND: Settings = Settings;

#[derive(Clone, Copy, Debug)]
pub struct Settings;

#[async_trait]
impl DiscordCommand for Settings {
    fn register(&self) -> CreateCommand {
        let options = vec![
            CreateCommandOption::new(
                CommandOptionType::SubCommandGroup,
                "set",
                "Set Tara's settings for this guild",
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::SubCommand,
                    "add_self_assignable_role",
                    "Add a role to the list of roles that users can self-assign",
                )
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::Role, "role", "The role to add")
                        .required(true),
                ),
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::SubCommand,
                    "remove_self_assignable_role",
                    "Remove a role from the list of roles that users can self-assign",
                )
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::Role, "role", "The role to remove")
                        .required(true),
                ),
            ),
            CreateCommandOption::new(
                CommandOptionType::SubCommandGroup,
                "view",
                "View a setting's value",
            ),
        ];

        CreateCommand::new(self.name())
            .description("View or modify Tara's settings for this guild")
            .default_member_permissions(Permissions::MANAGE_GUILD)
            .dm_permission(true)
            .set_options(options)
    }

    async fn run(
        &self,
        command: Arc<CommandInteraction>,
        args: CommandArguments,
    ) -> crate::Result<CommandResponse> {
        let option = &command.data.options[0];
        let guild = args.guild.unwrap();
        match &*option.name {
            "set" => {
                let option = &super::common::suboptions(option)[0];
                let option_name = option.name.clone();
                let option = &super::common::suboptions(option)[0];
                let CommandDataOptionValue::Role(role_id) = option.value else {
                    return Err(crate::Error::InternalLogic);
                };

                match &*option_name {
                    "add_self_assignable_role" => {
                        let role = { guild.roles.get(&role_id).unwrap() };
                        let inserted = sqlx::query_as!(
                            ExistingRole,
                            "INSERT INTO roles (id, guild_id) VALUES ($1, $2)
                            ON CONFLICT DO NOTHING
                            returning id",
                            role.id.toint(),
                            guild.id.toint(),
                        )
                        .fetch_optional(&args.database)
                        .await?
                        .map(ExistingRole::id);

                        // For the message
                        if let Some(id) = inserted {
                            Ok(format!("Added '{}' ({id}) to self-assignable roles!", role.name).into())
                        } else {
                            Ok(format!(
                                "'{}' ({}) is already part of the guild's self-assingable roles.",
                                role.name, role.id
                            )
                            .into())
                        }
                    }

                    "remove_self_assignable_role" => {
                        let role = { guild.roles.get(&role_id).unwrap() };
                        let removed = sqlx::query_as!(
                            ExistingRole,
                            "DELETE FROM roles WHERE id = $1 RETURNING id",
                            role.id.toint(),
                        )
                        .fetch_optional(&args.database)
                        .await?
                        .map(ExistingRole::id);

                        if let Some(id) = removed {
                            Ok(format!(
                                "Removed '{}' ({id}) from the guild's self-assignable roles.",
                                role.name
                            )
                            .into())
                        } else {
                            Err(Error::CommandMisuse(format!(
                                "'{}' ({}) wasn't part of the self-assignable roles and couldn't be removed!",
                                role.name, role.id
                            )))
                        }
                    }
                    _ => unreachable!(),
                }
            }
            "view" => {
                let _option = &super::common::suboptions(option)[0];
                return Err(crate::Error::InternalLogic);
            }
            _ => return Err(crate::Error::InternalLogic),
        }
    }

    fn name(&self) -> &'static str { "settings" }
}
