use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serenity::{
    all::{CommandInteraction, CommandOptionType, Guild},
    builder::{
        CreateAttachment, CreateCommand, CreateCommandOption, CreateEmbed, CreateInteractionResponse,
        CreateInteractionResponseMessage,
    },
    json::Value,
    prelude::Context,
};
use tokio::task;
use truncrate::TruncateToBoundary;

use super::DiscordCommand;
use crate::{Error, Result};

pub const COMMAND: Define = Define;

const FIELD_NAME_MAX: usize = 256;
const FIELD_VALUE_MAX: usize = 1024;

#[derive(Clone, Copy, Debug)]
pub struct Define;

#[async_trait]
impl DiscordCommand for Define {
    fn register(&self) -> CreateCommand {
        let options = vec![
            CreateCommandOption::new(CommandOptionType::String, "word", "The word to define").required(true),
            CreateCommandOption::new(
                CommandOptionType::Boolean,
                "audio",
                "Enable phonetic audio, may not always be available",
            )
            .required(false),
        ];

        CreateCommand::new(self.name())
            .description("Define an english word")
            .dm_permission(true)
            .set_options(options)
    }

    async fn run(
        &self,
        context: &Context,
        command: &CommandInteraction,
        _guild: Option<Guild>,
        _config: Arc<crate::config::Configuration>,
        databases: Arc<crate::database::Databases>,
    ) -> crate::Result<String> {
        let (word, audio) = {
            // Get the role argument
            let mut word = String::new();
            let mut audio = false;
            for option in &command.data.options {
                match &*option.name {
                    "word" => word = option.value.as_str().unwrap_or_default().trim().to_string(),
                    "audio" => audio = option.value.as_bool().unwrap_or(audio),
                    _ => return Err(Error::InternalLogic),
                }
            }

            (word, audio)
        };

        let words = get_word_definition(word.to_string()).await?;
        let max_content_length = super::core::get_max_content_len(command, &databases)?;

        // Create an embed from everything
        let mut total_length = 0usize;
        let word = words[0].to_owned();
        let phonetic_text = word
            .phonetics
            .iter()
            .filter_map(|phonetic| phonetic.text.clone())
            .next();

        // let phonetic_audio = word.phonetics.iter().filter_map(|phonetic| phonetic.audio)
        let title = if let Some(phonetic_text) = phonetic_text {
            format!("{} ({phonetic_text})", &word.word)
        }
        else {
            word.word.clone()
        };
        let mut embed_builder: CreateEmbed = CreateEmbed::new().title(title);

        'escape: for meaning in word.meanings {
            let mut word_field = format!("[{}] {}", meaning.part_of_speech, word.word);
            if word_field.len() > FIELD_NAME_MAX {
                word_field = word_field.truncate_to_boundary(FIELD_NAME_MAX - 1).to_string();
                word_field.push('…');
            }
            total_length += word_field.len();
            for definition in meaning.definitions {
                let mut value = definition.definition;
                if let Some(example) = definition.example {
                    value = format!("{value}\n> {example}");
                }

                // Truncate if it's too long.
                if value.len() > FIELD_VALUE_MAX {
                    value = value.truncate_to_boundary(FIELD_VALUE_MAX - 1).to_string();
                    value.push('…');
                }

                total_length += value.len();
                if total_length > max_content_length {
                    break 'escape; // we're done
                }
                embed_builder = embed_builder.field(&word_field, value, false);
            }
        }

        let mut attachments = Vec::with_capacity(3);
        if audio {
            let phonetic_audio = word
                .phonetics
                .iter()
                .filter(|phonetic| !phonetic.audio.is_empty())
                .map(|phonetic| &phonetic.audio);

            // We will only ever attach, at most, three audio files.
            for (i, audio_url) in phonetic_audio.enumerate() {
                if i > 3 {
                    break;
                }
                attachments.push(
                    CreateAttachment::url(&context.http, audio_url)
                        .await
                        .map_err(Error::SerenityHttpRequest)?,
                );
            }
        }

        let response = CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
                .add_embed(embed_builder)
                .add_files(attachments),
        );
        if let Err(e) = command.create_response(&context.http, response).await {
            log::error!("Couldn't respond to command: {e}");
        }

        Ok("".into())
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

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
struct NoWord {
    title:      String,
    message:    String,
    resolution: String,
}

/// Get the definition(s) for the provided `word`.
///
/// # Errors
///
/// Will return errors when
///
/// - An HTTP request fails
/// - An API returns invalid or unexpected JSON
async fn get_word_definition(word: String) -> Result<Words> {
    let word_ = urlencoding::encode(word.to_lowercase().trim()).to_string();
    let request_url = format!("https://api.dictionaryapi.dev/api/v2/entries/en/{word_}");

    // Make the API call, parse the json to a `Page`.
    let http_response = reqwest::get(&request_url).await.map_err(Error::HttpRequest)?;
    let http_body = Arc::new(http_response.text().await?);
    match task::spawn_blocking({
        let http_body = http_body.clone();
        move || serde_json::from_str::<Words>(&http_body)
    })
    .await?
    {
        Ok(x) => Ok(x),
        Err(e) => {
            let missing_word =
                task::spawn_blocking(move || serde_json::from_str::<NoWord>(&http_body)).await?;
            if missing_word.is_err() {
                Err(Error::JsonParse(e.to_string()))
            }
            else {
                Err(Error::UndefinedWord(format!(
                    "\"{}\" was not found: {}",
                    &word,
                    missing_word.unwrap().resolution
                )))
            }
        }
    }
}
