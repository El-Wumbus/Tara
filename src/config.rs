use serde::{Deserialize, Serialize};
use tokio::fs;

use crate::{database, Error, Result};

/// Configurations required to host the bot
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde_with::serde_as]
#[serde(rename_all = "camelCase")]
pub struct Configuration
{
    pub secrets:                 ConfigurationSecrets,
    #[serde_as(as = "serde_with::DurationSeconds<i64>")]
    pub direct_message_cooldown: Option<std::time::Duration>,
    pub random_error_message:    ConfigurationRandomErrorMessages,
    pub databases_path:          Option<std::path::PathBuf>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
/// API keys and other secrets
pub struct ConfigurationSecrets
{
    /// Discord bot token
    pub token: String,

    /// API key for access to `currencyapi.com`
    pub currency_api_key: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
#[serde(untagged)]
/// If, and where, to find error messages to randomly select from.
pub enum ConfigurationRandomErrorMessages
{
    Boolean(bool),
    Path(std::path::PathBuf),
}

impl Configuration
{
    const DEFAULT_DM_COOLDOWN_LEN: u64 = 3;

    /// Read a `Configuration` from toml located at `path`.
    ///
    /// # Usage
    ///
    /// ```rust
    /// let file = PathBuf::from("config.toml");
    /// let config = Configuration::from_toml(file).unwrap();
    /// dbg!(config);
    /// ```
    ///
    /// # Errors
    ///
    /// Will throw and `Error` when:
    ///
    /// - `Path` cannoth be read from successfully
    /// - `Path`'s contents cannot be parsed into a `Configuration`
    pub async fn from_toml(path: impl Into<std::path::PathBuf>) -> Result<Self>
    {
        let path = path.into();
        let file_contents = fs::read_to_string(&path).await.map_err(Error::Io)?;
        let parsed =
            toml::from_str(&file_contents).map_err(|e| Error::ConfigurationParse { path, error: e })?;
        Ok(parsed)
    }
}

impl Default for Configuration
{
    fn default() -> Self
    {
        Self {
            secrets:                 ConfigurationSecrets::default(),
            databases_path:          Some(std::path::PathBuf::from(database::DEFAULT_DATABASE_DIRECTORY)),
            random_error_message:    ConfigurationRandomErrorMessages::Boolean(true),
            direct_message_cooldown: Some(std::time::Duration::from_secs(Self::DEFAULT_DM_COOLDOWN_LEN)),
        }
    }
}

impl ConfigurationSecrets
{
    const DEFAULT_DISCORD_TOKEN: &str = "<DISCORD_TOKEN>";
}

impl Default for ConfigurationSecrets
{
    fn default() -> Self
    {
        Self {
            token:            Self::DEFAULT_DISCORD_TOKEN.to_string(),
            currency_api_key: None,
        }
    }
}

/// Error messages parsed from the file provided in the `Configuration`.
pub struct ErrorMessages
{
    pub messages: Vec<(String, String)>,
}

impl ErrorMessages
{
    pub const DEFAULT_FILE: &str = "/etc/tara.d/error_messages.json";

    /// Read an `ErrorMessages` from JSON located at `path`.
    ///
    /// # Usage
    ///
    /// ```rust
    /// let file = PathBuf::from("config.toml");
    /// let messages = ErrorMessages::from_json(file).unwrap();
    /// dbg!(config);
    /// ```
    ///
    /// # Errors
    ///
    /// Will throw and `Error` when:
    ///
    /// - `Path` cannoth be read from successfully
    /// - `Path`'s contents cannot be parsed into `ErrorMessages`
    pub async fn from_json(path: impl Into<std::path::PathBuf>) -> Result<Self>
    {
        pub type Root = Vec<[String; 2]>;
        let path = path.into();
        let file_contents = tokio::fs::read_to_string(&path).await.map_err(Error::Io)?;
        let parsed: Root =
            serde_json::from_str(&file_contents).map_err(|e| Error::MessageParse { path, error: e })?;

        let messages = parsed
            .into_iter()
            .map(|mut x| (std::mem::take(&mut x[0]), std::mem::take(&mut x[1])))
            .collect();

        Ok(ErrorMessages { messages })
    }
}

impl Default for ErrorMessages
{
    fn default() -> Self
    {
        Self {
            messages: vec![("There was an error".to_string(), "Please try again.".to_string())],
        }
    }
}
