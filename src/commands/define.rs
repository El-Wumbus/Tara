use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serenity::{
    json::Value,
    model::prelude::{
        command::CommandOptionType, interaction::application_command::ApplicationCommandInteraction,
    },
    prelude::Context,
};
use truncrate::TruncateToBoundary;

use super::DiscordCommand;
use crate::{Error, Result};

pub const COMMAND: Define = Define;

#[derive(Clone, Copy, Debug)]
pub struct Define;

#[async_trait]
impl DiscordCommand for Define
{
    fn register<'a>(
        &'a self,
        command: &'a mut serenity::builder::CreateApplicationCommand,
    ) -> &mut serenity::builder::CreateApplicationCommand
    {
        command
            .name("define")
            .description("Define an english word")
            .dm_permission(true)
            .create_option(|option| {
                option
                    .name("word")
                    .description("The word to define")
                    .kind(CommandOptionType::String)
                    .required(true)
            })
    }

    async fn run(
        &self,
        _context: &Context,
        command: &ApplicationCommandInteraction,
        _config: Arc<crate::config::Configuration>,
        databases: Arc<crate::database::Databases>,
    ) -> crate::Result<String>
    {
        let mut word = Value::Null;
        for option in &command.data.options {
            match &*option.name {
                "word" => word = option.value.clone().unwrap_or_default(),
                _ => return Err(Error::InternalLogic),
            }
        }

        let mut content = get_word_definition(word.as_str().unwrap_or_default().to_string()).await?;

        let max = super::core::get_max_content_len(command, &databases)?;
        // Truncate wiki content.
        if content.len() >= max {
            content = format!("{}…", content.truncate_to_boundary(max));
        }

        Ok(content)
    }

    fn name(&self) -> String { String::from("define") }
}

type Words = Vec<Word>;

structstruck::strike! {
    #[strikethrough[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]]
    #[strikethrough[serde(rename_all = "camelCase")]]
    struct Word {
        word: String,
        phonetics: Vec<pub struct Phonetic {
             audio: String,
            source_url: Option<String>,
             license: Option<pub struct License {
                name: String,
                url: String,
            }>,
            text: Option<String>,
        }>,
        meanings: Vec<pub struct Meaning {
            part_of_speech: String,
            definitions: Vec<pub struct Definition {
                definition: String,
                synonyms: Vec<Value>,
                antonyms: Vec<Value>,
                example: Option<String>,
            }
            >,
            synonyms: Vec<String>,
            antonyms: Vec<String>,
        }>,
        license: License,
        source_urls: Vec<String>,
    }
}

/// Get the definition(s) for the provided `word`.
///
/// # Errors
///
/// Will return errors when
///
/// - An HTTP request fails
/// - An API returns invalid or unexpected JSON
async fn get_word_definition(word: String) -> Result<String>
{
    let word_ = urlencoding::encode(word.to_lowercase().trim()).to_string();
    let request_url = format!("https://api.dictionaryapi.dev/api/v2/entries/en/{word_}");

    // Make the API call, parse the json to a `Page`.
    if let Ok(words) = {
        reqwest::get(&request_url)
            .await
            .map_err(Error::HttpRequest)?
            .json::<Words>()
            .await
    } {
        let mut buf = String::new();
        for meaning in &words[0].meanings {
            buf.push_str(&format!(
                "({}) {}\n",
                meaning.part_of_speech, meaning.definitions[0].definition
            ));
            if let Some(example) = &meaning.definitions[0].example {
                buf.push_str(&format!("    Example: '{example}'\n"));
            }
        }
        Ok(format!("Definitions for {word_}:\n{buf}"))
    }
    else {
        Ok(format!("Couldn't define \"{word}\""))
    }
}
