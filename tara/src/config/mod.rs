use serde::{Deserialize, Serialize};
use tokio::fs;

use crate::{Error, Result};

pub mod music;

/// Configurations required to host the bot
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Configuration {
    pub secrets:              ConfigurationSecrets,
    pub random_error_message: ConfigurationRandomErrorMessages,
    pub music:                Option<music::Music>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
/// API keys and other secrets
pub struct ConfigurationSecrets {
    /// Discord bot token
    pub token: String,

    /// API key for access to `currencyapi.com`
    pub currency_api_key: Option<String>,
    pub omdb_api_key:     Option<String>,
    pub unsplash_key:     Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
#[serde(untagged)]
/// If, and where, to find error messages to randomly select from.
pub enum ConfigurationRandomErrorMessages {
    Boolean(bool),
    Path(std::path::PathBuf),
}

impl Configuration {
    /// Read a `Configuration` from toml located at `path`.
    ///
    /// # Usage
    ///
    /// ```no_run
    /// # use std::path::PathBuf;
    /// # use tara::config::Configuration;
    /// # tokio_test::block_on(async {
    /// let file = PathBuf::from("config.toml");
    /// let config = Configuration::from_toml(file).await.unwrap();
    /// dbg!(config);
    /// # });
    /// ```
    ///
    /// # Errors
    ///
    /// Will error when:
    ///
    /// - `Path` cannoth be read from successfully
    /// - `Path`'s contents cannot be parsed into a `Configuration`
    pub async fn from_toml(path: impl Into<std::path::PathBuf>) -> Result<Self> {
        let path = path.into();
        let file_contents = fs::read_to_string(&path).await.map_err(Error::Io)?;
        let parsed = toml::from_str(&file_contents).map_err(|e| {
            Error::ConfigurationParse {
                path,
                error: Box::new(e),
            }
        })?;
        Ok(parsed)
    }
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            secrets:              ConfigurationSecrets::default(),
            random_error_message: ConfigurationRandomErrorMessages::Boolean(true),
            music:                Some(music::Music::default()),
        }
    }
}

impl ConfigurationSecrets {
    const DEFAULT_DISCORD_TOKEN: &str = "<DISCORD_TOKEN>";
}

impl Default for ConfigurationSecrets {
    fn default() -> Self {
        Self {
            token:            Self::DEFAULT_DISCORD_TOKEN.to_string(),
            currency_api_key: None,
            omdb_api_key:     None,
            unsplash_key:     None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
/// Error messages parsed from the file provided in the `Configuration`
pub struct ErrorMessages {
    pub(crate) messages: Vec<(String, String)>,
}

impl ErrorMessages {
    /// Read an `ErrorMessages` from JSON located at `path`.
    ///
    /// # Usage
    ///
    /// ```no_run
    /// # use std::path::PathBuf;
    /// # use tara::config::ErrorMessages;
    /// # tokio_test::block_on(async {
    /// let file = PathBuf::from("config.toml");
    /// let messages = ErrorMessages::from_json(file).await.unwrap();
    /// dbg!(messages);
    /// # });
    /// ```
    ///
    /// # Errors
    ///
    /// Will error when:
    ///
    /// - `Path` cannoth be read from successfully
    /// - `Path`'s contents cannot be parsed into `ErrorMessages`
    pub async fn from_json(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let path = path.as_ref();
        let file_contents = tokio::fs::read_to_string(&path).await.map_err(Error::Io)?;
        let parsed: Vec<[String; 2]> = serde_json::from_str(&file_contents).map_err(|e| {
            Error::MessageParse {
                path:  path.into(),
                error: e,
            }
        })?;

        let messages = parsed
            .into_iter()
            .map(|mut x| (std::mem::take(&mut x[0]), std::mem::take(&mut x[1])))
            .collect();

        Ok(ErrorMessages { messages })
    }
}

impl Default for ErrorMessages {
    fn default() -> Self {
        Self {
            messages: vec![("There was an error".to_string(), "Please try again.".to_string())],
        }
    }
}
