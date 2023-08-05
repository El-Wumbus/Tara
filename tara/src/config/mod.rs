use std::{env, path::Path};

use serde::{Deserialize, Serialize};
use tara_util::paths;
use tokio::fs;

use crate::{Error, Result};

pub mod ai;
pub mod music;

/// Configurations required to host the bot
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Configuration {
    pub secrets:              ConfigurationSecrets,
    pub random_error_message: ConfigurationRandomErrorMessages,
    pub music:                Option<music::Music>,
    pub ai:                   Option<ai::Ai>,
}

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
/// API keys and other secrets
pub struct ConfigurationSecrets {
    /// Discord bot token (overridden at runtime by the `TARA_TOKEN` env variable if
    /// present).
    pub token:            Option<String>,
    /// Postgres Database URL (overridden at runtime by the `TARA_POSTGRES` env variable
    /// if present).
    pub postgres:         Option<String>,
    /// API key for access to `currencyapi.com` (overridden at runtime by the
    /// `TARA_CURRENCY_KEY` env variable if present).
    pub currency_api_key: Option<String>,
    /// API key for access to OMDb (overridden at runtime by the
    /// `TARA_OMDB_KEY` env variable if present), this is completely optional, if
    /// it's not provided builtin ones will be used instead.
    pub omdb_api_key:     Option<String>,
    /// API key for access to Unsplash (overridden at runtime by the
    /// `TARA_UNSPLASH_KEY` env variable if present).
    pub unsplash_key:     Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
/// If, and where, to find error messages to randomly select from.
pub enum ConfigurationRandomErrorMessages {
    Boolean(bool),
    Path(std::path::PathBuf),
}

impl Configuration {
    /// Read a `Configuration` from toml located at `path`, anything not found in the file
    /// will be grabbed from the corresponding environment variables.
    ///
    /// # Usage
    ///
    /// ```no_run
    /// # use std::path::PathBuf;
    /// # use tara::config::Configuration;
    /// # tokio_test::block_on(async {
    /// let file = PathBuf::from("config.toml");
    /// let config = Configuration::parse(file).await.unwrap();
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
    pub async fn parse(path: Option<impl AsRef<Path>>) -> anyhow::Result<Self> {
        // Get the configuration file path and read the configuration from it.
        let path = path
            .as_ref()
            .map(|x| x.as_ref())
            .or_else(|| paths::TARA_CONFIGURATION_FILE.as_ref().map(|x| x.as_path()));

        let Self {
            secrets:
                ConfigurationSecrets {
                    token,
                    postgres,
                    currency_api_key,
                    omdb_api_key,
                    unsplash_key,
                },
            random_error_message,
            music,
            ai,
        } = if let Some(path) = path {
            let file_contents = fs::read_to_string(path).await.map_err(Error::Io)?;
            tracing::info!("Loaded configuration from \"{}\"", path.display());
            let parsed: Self = toml::from_str(&file_contents).map_err(|e| {
                Error::ConfigurationParse {
                    path:  path.to_path_buf(),
                    error: Box::new(e),
                }
            })?;
            parsed
        } else {
            Self::default()
        };

        let config = Self {
            secrets: ConfigurationSecrets {
                token:            env::var("TARA_TOKEN").ok().or(token),
                postgres:         env::var("TARA_POSTGRES").ok().or(postgres),
                currency_api_key: env::var("TARA_CURRENCY_KEY").ok().or(currency_api_key),
                omdb_api_key:     env::var("TARA_OMDB_KEY").ok().or(omdb_api_key),
                unsplash_key:     env::var("TARA_UNSPLASH_KEY").ok().or(unsplash_key),
            },
            random_error_message,
            music,
            ai,
        };

        tracing::debug!("Parsed config: {config:#?}");
        Ok(config)
    }
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            secrets:              ConfigurationSecrets::default(),
            random_error_message: ConfigurationRandomErrorMessages::Boolean(false),
            music:                Some(music::Music::default()),
            ai:                   None,
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
