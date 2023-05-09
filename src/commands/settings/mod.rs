use async_trait::async_trait;
use serenity::{
    all::CommandOptionType,
    builder::{CreateCommand, CreateCommandOption},
    model::Permissions,
};

use super::{CommandArguments, DiscordCommand};


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

    async fn run(&self, args: CommandArguments) -> crate::Result<String> {
        let option = &args.command.data.options[0];
        let guild = args.guild.unwrap();
        match &*option.name {
            "set" => {
                let option = &super::core::suboptions(option)[0];
                match &*option.name {
                    "maximum_content_output_chars" => {
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
                let option = &super::core::suboptions(option)[0];
                match &*option.name {
                    "maximum_content_output_chars" => {
                        return view::content_character_limit(args.command.guild_id, &args.guild_preferences)
                            .await;
                    }
                    _ => return Err(crate::Error::InternalLogic),
                }
            }
            _ => return Err(crate::Error::InternalLogic),
        }
    }

    fn name(&self) -> String { String::from("settings") }
}
