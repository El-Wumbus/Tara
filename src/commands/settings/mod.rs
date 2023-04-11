use std::sync::Arc;

use async_trait::async_trait;
use serenity::{
    all::{CommandInteraction, CommandOptionType},
    builder::{CreateCommand, CreateCommandOption},
    model::{prelude::Guild, Permissions},
    prelude::Context,
};

use super::DiscordCommand;
use crate::database::Databases;


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
                    "maximum_content_output_chars",
                    "The maximum length, in characters, of content from APIs.",
                )
                .add_sub_option(
                    CreateCommandOption::new(
                        CommandOptionType::Integer,
                        "chars",
                        "Length in chars (80 MIN, 1900 MAX)",
                    )
                    .add_int_choice("default", super::wiki::Wiki::DEFAULT_MAX_WIKI_LEN as i32)
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
                "maximum_content_output_chars",
                "The maximum length, in characters, of content from APIs.",
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
        _context: &Context,
        command: &CommandInteraction,
        guild: Option<Guild>,
        _config: Arc<crate::config::Configuration>,
        _databases: Arc<crate::database::Databases>,
    ) -> crate::Result<String> {
        let option = &command.data.options[0];
        let databases = Databases::open().await?;
        let guild = guild.unwrap();
        match &*option.name {
            "set" => {
                let option = &super::core::suboptions(option)[0];
                match &*option.name {
                    "maximum_content_output_chars" => {
                        return set::maximum_content_output_chars(&databases, option, guild.id)
                    }
                    "add_self_assignable_role" => {
                        return set::update_self_assignable_role(&databases, option, guild, false)
                    }

                    "remove_self_assignable_role" => {
                        return set::update_self_assignable_role(&databases, option, guild, true)
                    }
                    _ => unreachable!(),
                }
            }
            "view" => {
                let option = &super::core::suboptions(option)[0];
                match &*option.name {
                    "maximum_content_output_chars" => {
                        return view::maximum_content_output_chars(command, &databases);
                    }
                    _ => return Err(crate::Error::InternalLogic),
                }
            }
            _ => return Err(crate::Error::InternalLogic),
        }
    }

    fn name(&self) -> String { String::from("settings") }
}
