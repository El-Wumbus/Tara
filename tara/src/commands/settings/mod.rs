use std::sync::Arc;

use async_trait::async_trait;
use serenity::{
    all::{CommandInteraction, CommandOptionType},
    builder::{CreateCommand, CreateCommandOption},
    model::Permissions,
};

use super::{CommandArguments, CommandResponse, DiscordCommand};


mod set;
pub use set::*;
mod view;
pub use view::*;

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
                    "content_character_limit",
                    "The charater limit on content retrived from external sources",
                )
                .add_sub_option(
                    CreateCommandOption::new(
                        CommandOptionType::Integer,
                        "chars",
                        "Length in chars (80 MIN, 1900 MAX)",
                    )
                    .required(true),
                ),
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
            )
            .add_sub_option(CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "content_character_limit",
                "The charater limit on content retrived from external sources",
            )),
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
                match &*option.name {
                    "content_character_limit" => {
                        return set::content_character_limit(&args.guild_preferences, option, guild.id).await
                    }
                    "add_self_assignable_role" => {
                        return set::update_self_assignable_roles(
                            &args.guild_preferences,
                            option,
                            guild,
                            false,
                        )
                        .await
                    }

                    "remove_self_assignable_role" => {
                        return set::update_self_assignable_roles(
                            &args.guild_preferences,
                            option,
                            guild,
                            true,
                        )
                        .await
                    }
                    _ => unreachable!(),
                }
            }
            "view" => {
                let option = &super::common::suboptions(option)[0];
                match &*option.name {
                    "content_character_limit" => {
                        return view::content_character_limit(command.guild_id, &args.guild_preferences)
                            .await;
                    }
                    _ => return Err(crate::Error::InternalLogic),
                }
            }
            _ => return Err(crate::Error::InternalLogic),
        }
    }

    fn name(&self) -> &'static str { "settings" }
}
