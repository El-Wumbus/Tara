use std::num::NonZeroU64;

use serenity::{
    all::{CommandDataOption, CommandDataOptionValue, CommandInteraction, RoleId},
    builder::{CreateActionRow, CreateEmbed, CreateInteractionResponse, CreateInteractionResponseMessage},
    http::Http,
};
use tracing::{event, Level};

#[must_use]
/// Gets the suboptions of a subcommand or subcommandgroup.
///
/// # Panics
///
/// Panics if `option.value` isn't a [`CommandDataOptionValue::SubCommand`] or
/// [`CommandDataOptionValue::SubCommandGroup`]
pub fn suboptions(option: &CommandDataOption) -> &Vec<CommandDataOption> {
    let mut val = None;
    match &option.value {
        CommandDataOptionValue::SubCommand(options) | CommandDataOptionValue::SubCommandGroup(options) => {
            val = Some(options);
        }
        _ => (),
    }
    val.unwrap()
}

#[must_use]
/// Remove the first of any suffixes found in `suffixes` from the input string.
pub fn strip_suffixes(input: &str, suffixes: &[&str]) -> String {
    let input_bytes = input.as_bytes();
    let mut _suffix_bytes: &[u8];

    for suffix in suffixes {
        if let Some(input_without_suffix) = input_bytes.strip_suffix(suffix.as_bytes()) {
            return String::from_utf8_lossy(input_without_suffix).into_owned();
        }
    }

    input.to_string()
}

pub fn ends_with_any<'a>(s: &str, possible_suffixes: &'a [&'a str]) -> bool {
    possible_suffixes.iter().any(|x| s.ends_with(x))
}

pub fn equals_any<'a>(s: &str, possible_matches: &'a [&'a str]) -> bool {
    possible_matches.iter().any(|x| *x == s)
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum CommandResponse {
    String(String),
    EphemeralString(String),
    Embed(Box<CreateEmbed>),
    EmbedWithComponents(Box<CreateEmbed>, Vec<CreateActionRow>),
    Message(CreateInteractionResponseMessage),
    None,
}

impl CommandResponse {
    pub fn new_string(s: impl Into<String>) -> Self { Self::from(s.into()) }

    pub async fn send(self, command: &CommandInteraction, http: &Http) {
        let message = CreateInteractionResponseMessage::new();
        let response_message = match self {
            CommandResponse::String(s) => message.content(s),
            CommandResponse::EphemeralString(s) => message.content(s).ephemeral(true),
            CommandResponse::Embed(embed) => message.embed(*embed),
            CommandResponse::EmbedWithComponents(embed, components) => {
                message.embed(*embed).components(components)
            }
            CommandResponse::Message(message) => message,
            CommandResponse::None => return,
        };
        let response = CreateInteractionResponse::Message(response_message);
        if let Err(e) = command.create_response(http, response).await {
            event!(
                Level::ERROR,
                "Couldn't respond to command ({}): {e}",
                command.data.name.as_str()
            );
        }
    }
}

impl From<String> for CommandResponse {
    fn from(value: String) -> Self { Self::String(value) }
}

pub fn hex_color_code_to_rgb(color_code: &str) -> Option<(u8, u8, u8)> {
    if color_code.len() != 7 || &color_code[..1] != "#" {
        return None;
    }

    let value = u32::from_str_radix(&color_code[1..], 16).ok()?;

    let red = ((value >> 16) & 255) as u8;
    let green = ((value >> 8) & 255) as u8;
    let blue = (value & 255) as u8;

    Some((red, green, blue))
}

pub mod unsplash {
    use std::str::FromStr;

    use serde::{Deserialize, Serialize};
    use serenity::{
        builder::{CreateEmbed, CreateEmbedAuthor, CreateEmbedFooter},
        model::Color,
    };
    use url::Url;

    use super::hex_color_code_to_rgb;
    use crate::{Error, Result};

    const UNSPLASH_REFFERAL_QUERY: &str = "utm_source=Tara&utm_medium=referral";

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    #[non_exhaustive]
    struct Urls {
        raw:     Option<String>,
        full:    String,
        regular: String,
        small:   String,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    #[non_exhaustive]
    struct User {
        id:            String,
        username:      String,
        name:          Option<String>,
        #[serde(rename = "profile_image")]
        profile_image: ProfileImage,
    }


    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct Links {
        #[serde(rename = "self")]
        self_field:        String,
        html:              String,
        // Don't use
        download:          String,
        // Use
        download_location: String,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    #[non_exhaustive]
    struct ProfileImage {
        full:    Option<String>,
        large:   Option<String>,
        regular: Option<String>,
        medium:  Option<String>,
        small:   Option<String>,
        thumb:   Option<String>,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    #[non_exhaustive]
    pub struct UnsplashImage {
        id:            String,
        width:         i64,
        height:        i64,
        links:         Links,
        color:         String,
        description:   Option<String>,
        urls:          Urls,
        user:          User,
        #[serde(rename = "public_domain")]
        public_domain: Option<bool>,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct UnsplashSearchResult {
        total:       u64,
        total_pages: u64,
        results:     Vec<UnsplashImage>,
    }

    impl UnsplashImage {
        pub async fn random(client_id: &str) -> Result<Self> {
            let response = reqwest::get(format!(
                "https://api.unsplash.com/photos/random?client_id={client_id}"
            ))
            .await?
            .text()
            .await?;
            let image = serde_json::from_str(&response).map_err(|e| Error::JsonParse(e.to_string()))?;

            Ok(image)
        }

        pub async fn search(
            client_id: &str,
            query: &str,
            color: Option<String>,
            orientation: Option<String>,
        ) -> Result<Vec<Self>> {
            let color = color.map_or_else(String::new, |x| format!("&color={x}"));
            let orientation = orientation.map_or_else(String::new, |x| format!("&orientation={x}"));
            let response = reqwest::get(format!(
                "https://api.unsplash.com/search/photos?client_id={client_id}&query={query}{color}{orientation}"
            ))
            .await?
            .text()
            .await?;
            let images: UnsplashSearchResult =
                serde_json::from_str(&response).map_err(|e| Error::JsonParse(e.to_string()))?;

            Ok(images.results)
        }
    }

    impl From<&UnsplashImage> for CreateEmbed {
        fn from(value: &UnsplashImage) -> Self {
            let image = {
                let mut image = Url::from_str(&value.urls.regular).unwrap();
                let image_query = image.query().map_or(String::from(UNSPLASH_REFFERAL_QUERY), |x| {
                    format!("{x}&{UNSPLASH_REFFERAL_QUERY}")
                });
                image.set_query(Some(&image_query));
                image
            };

            let color = {
                let (r, g, b) = hex_color_code_to_rgb(&value.color).unwrap();
                Color::from_rgb(r, g, b)
            };

            let username = value.user.username.as_str();
            let user_url = format!("https://unsplash.com/@{username}?{UNSPLASH_REFFERAL_QUERY}");

            let author_icon = {
                let profile_image = value.user.profile_image.clone();
                let icons = vec![
                    profile_image.thumb,
                    profile_image.small,
                    profile_image.medium,
                    profile_image.regular,
                    profile_image.large,
                    profile_image.full,
                ];

                icons.into_iter().find_map(|icon| icon).map(|icon_url| {
                    let mut url = Url::from_str(&icon_url).unwrap();
                    let query = url.query().map_or(String::from(UNSPLASH_REFFERAL_QUERY), |x| {
                        format!("{x}&{UNSPLASH_REFFERAL_QUERY}")
                    });
                    url.set_query(Some(&query));
                    url.as_str().to_string()
                })
            };

            let mut embed_author =
                CreateEmbedAuthor::new(value.user.name.as_ref().unwrap_or(&value.user.username))
                    .url(user_url);

            if let Some(author_icon) = author_icon {
                embed_author = embed_author.icon_url(author_icon);
            }

            let public_domain = if value.public_domain.unwrap_or_default() {
                " Work is in the public domain."
            } else {
                ""
            };

            let mut embed = CreateEmbed::new()
                .author(embed_author)
                .image(image.as_str())
                .color(color)
                .footer(CreateEmbedFooter::new(format!("From Unsplash.{public_domain}")));

            if let Some(desc) = &value.description {
                embed = embed.description(desc);
            }

            embed
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct ExistingRole {
    pub(super) id: i64,
}

impl ExistingRole {
    #[inline]
    pub(super) fn id(self) -> RoleId { RoleId(NonZeroU64::new(self.id as u64).unwrap()) }
}
