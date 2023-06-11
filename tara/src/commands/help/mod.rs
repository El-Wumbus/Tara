use std::sync::Arc;

use async_trait::async_trait;
use convert_case::{Case, Casing};
use serenity::{
    all::{CommandInteraction, CommandOption, CommandOptionType},
    builder::{CreateCommand, CreateCommandOption, CreateEmbed, CreateEmbedFooter},
};
use tokio::sync::RwLock;
use truncrate::TruncateToBoundary;

use super::{common::CommandResponse, CommandArguments, DiscordCommand, COMMANDS};
use crate::{Error, Result};

pub const COMMAND: Help = Help;

lazy_static::lazy_static! {
    static ref GLOBAL_COMMANDS: Arc<RwLock<Vec<serenity::all::Command>>> = Arc::new(RwLock::new(Vec::new()));
}

#[derive(Clone, Copy, Debug)]
pub struct Help;

#[async_trait]
impl DiscordCommand for Help {
    fn register(&self) -> CreateCommand {
        let mut command_name_option = CreateCommandOption::new(
            CommandOptionType::String,
            "command",
            "The name of the command. The command name, not subcommand! (e.g. \"music\", not \"music play\")",
        )
        .required(true);

        for (i, command_name) in super::COMMANDS.values().map(|x| x.name()).enumerate() {
            // There can be no more than 25 choices...
            if i > 25 {
                break;
            }

            command_name_option = command_name_option.add_string_choice(command_name, command_name);
        }

        let options = vec![command_name_option];
        CreateCommand::new(self.name())
            .description("Get help with a command")
            .dm_permission(true)
            .set_options(options)
    }

    async fn run(&self, command: Arc<CommandInteraction>, args: CommandArguments) -> Result<CommandResponse> {
        let command_name = command.data.options[0]
            .value
            .as_str()
            .ok_or(Error::InternalLogic)?
            .to_lowercase();

        if GLOBAL_COMMANDS.read().await.is_empty() {
            let global_commands = serenity::all::Command::get_global_commands(&args.context.http).await?;
            let mut lock = GLOBAL_COMMANDS.write().await;
            *lock = global_commands;
        }

        let all_commands = GLOBAL_COMMANDS.read().await;

        let command = all_commands
            .iter()
            .find(|x| x.name == command_name)
            .ok_or_else(|| Error::CommandMisuse(format!("\"{command_name}\" is not a command!")))?;

        let mut embed = CreateEmbed::new()
            .title(command.name.to_case(Case::Title))
            .url(format!(
                "{}/tree/master/tara/src/commands/{command_name}",
                crate::REPO_URL
            ))
            .description(command.description.clone())
            .footer(CreateEmbedFooter::new(format!("NSFW: {}", command.nsfw)));

        let options_fields = command.options.iter().map(option_to_field);
        embed = embed.fields(options_fields);

        let Some(command) = COMMANDS.get(command_name.as_str()) else {return Err(Error::CommandMisuse(format!("\"{command_name}\" is not a command!")))};
        if let Some(help) = command.help() {
            embed = embed.field("Additional Help", help, false);
        }

        Ok(CommandResponse::Embed(Box::new(embed)))
    }

    fn name(&self) -> &'static str { "help" }
}

fn option_to_field(option: &CommandOption) -> (String, String, bool) {
    let (name, mut description, z, _) = _option_to_field(option, 0);

    if description.len() > 1024 {
        description = description.truncate_to_boundary(1024 - 1).to_string();
        description.push('…');
    }
    (name, description, z)
}

fn _option_to_field(option: &CommandOption, suboption_depth: usize) -> (String, String, bool, usize) {
    let mut description = option.description.clone();
    if matches!(
        option.kind,
        CommandOptionType::SubCommandGroup | CommandOptionType::SubCommand
    ) {
        let name = format!("**`{}`**", option.name);

        for suboption in &option.options {
            let (sub_name, sub_description, _, suboption_depth) =
                _option_to_field(suboption, suboption_depth + 1);
            let indent = "  ".repeat(suboption_depth);
            description.push_str(&format!("\n{indent}{sub_name}\n{indent}{sub_description}"));
        }

        return (name, description, false, suboption_depth);
    }

    let name = if option.required {
        format!("*`{}`\\**", option.name)
    } else {
        format!("*`{}`*", option.name)
    };

    if !option.choices.is_empty() {
        let choices = option
            .choices
            .iter()
            .map(|x| format!("{}: `{}`", x.name, x.value))
            .collect::<Vec<_>>()
            .join("\n");
        description.push_str(&format!("\n\t**Choices**:\n{choices}"));
    }
    (name, description, false, suboption_depth)
}
