use std::sync::Arc;

use async_trait::async_trait;
use serenity::{
    model::{
        prelude::{
            command::CommandOptionType, interaction::application_command::ApplicationCommandInteraction,
        },
        Permissions,
    },
    prelude::Context,
};

use super::DiscordCommand;


mod set;
pub use set::*;
mod view;
pub use view::*;

pub const COMMAND: Settings = Settings;

#[derive(Clone, Copy, Debug)]
pub struct Settings;

#[async_trait]
impl DiscordCommand for Settings {
    fn register<'a>(
        &'a self,
        command: &'a mut serenity::builder::CreateApplicationCommand,
    ) -> &mut serenity::builder::CreateApplicationCommand {
        command
            .name(self.name())
            .dm_permission(false)
            .description("View or modify bot settings for this guild")
            .default_member_permissions(Permissions::MANAGE_GUILD)
            .create_option(|option| {
                option
                    .name("set")
                    .description("Set bot settings for this guild")
                    .kind(CommandOptionType::SubCommandGroup)
                    .create_sub_option(|option| {
                        option
                            .name("maximum_content_output_chars")
                            .description("The maximum length, in characters, of content from APIs.")
                            .kind(CommandOptionType::SubCommand)
                            .create_sub_option(|option| {
                                option
                                    .kind(CommandOptionType::Integer)
                                    .name("chars")
                                    .description("Length in chars (80 MIN, 1900 MAX)")
                                    .required(true)
                            })
                    })
                    .create_sub_option(|option| {
                        option
                            .name("add_self_assignable_role")
                            .description("Add a role to the list of roles that users can self-assign")
                            .kind(CommandOptionType::SubCommand)
                            .create_sub_option(|option| {
                                option
                                    .kind(CommandOptionType::Role)
                                    .name("role")
                                    .description("The role to add")
                                    .required(true)
                            })
                    })
                    .create_sub_option(|option| {
                        option
                            .name("remove_self_assignable_role")
                            .description("Remove a role from the list of roles that users can self-assign")
                            .kind(CommandOptionType::SubCommand)
                            .create_sub_option(|option| {
                                option
                                    .kind(CommandOptionType::Role)
                                    .name("role")
                                    .description("The role to remove")
                                    .required(true)
                            })
                    })
            })
            .create_option(|option| {
                option
                    .name("view")
                    .description("View a setting's value.")
                    .kind(CommandOptionType::SubCommandGroup)
                    .create_sub_option(|option| {
                        option
                            .name("maximum_content_output_chars")
                            .description("The maximum length, in characters, of content from APIs.")
                            .kind(CommandOptionType::SubCommand)
                    })
            })
    }

    async fn run(
        &self,
        _context: &Context,
        command: &ApplicationCommandInteraction,
        _config: Arc<crate::config::Configuration>,
        databases: Arc<crate::database::Databases>,
    ) -> crate::Result<String> {
        let option = &command.data.options[0];
        match &*option.name {
            "set" => {
                let option = &option.options[0];
                match &*option.name {
                    "maximum_content_output_chars" => {
                        return set::maximum_content_output_chars(
                            &databases,
                            option,
                            command.guild_id.unwrap(),
                        )
                    }
                    "add_self_assignable_role" => {
                        return set::update_self_assignable_role(
                            &databases,
                            option,
                            command.guild_id.unwrap(),
                            false,
                        )
                    }

                    "remove_self_assignable_role" => {
                        return set::update_self_assignable_role(
                            &databases,
                            option,
                            command.guild_id.unwrap(),
                            true,
                        )
                    }
                    _ => unreachable!(),
                }
            }
            "view" => {
                let option = &option.options[0];
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
