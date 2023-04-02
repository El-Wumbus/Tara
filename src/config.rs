use serde::{Deserialize, Serialize};
use tokio::fs;

use crate::{database, Error, Result};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Configuration
{
    pub secrets:              ConfigurationSecrets,
    pub random_error_message: ConfigurationRandomErrorMessages,
    pub databases_path:       Option<std::path::PathBuf>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ConfigurationSecrets
{
    /// Discord bot token
    pub token: String,

    /// API key for access to `currencyapi.com`
    pub currency_api_key: Option<String>,

    /// API key for access to YouTube
    pub youtube_api_key: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
#[serde(untagged)]
pub enum ConfigurationRandomErrorMessages
{
    Boolean(bool),
    Path(std::path::PathBuf),
}

impl Configuration
{
    /// Read a `Configuration` from toml located at `path`.
    ///
    /// # Usage
    ///
    /// ```Rust
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
            secrets:              ConfigurationSecrets::default(),
            random_error_message: ConfigurationRandomErrorMessages::Boolean(true),
            databases_path:       Some(std::path::PathBuf::from(database::DEFAULT_DATABASE_DIRECTORY)),
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
            youtube_api_key:  None,
        }
    }
}

pub struct ErrorMessages
{
    pub messages: Vec<(String, String)>,
}

impl ErrorMessages
{
    pub const DEFAULT_FILE: &str = "/etc/tara.d/error_messages.json";

    pub async fn from_json(path: impl Into<std::path::PathBuf>) -> Result<Self>
    {
        pub type Root = Vec<Vec<String>>;
        let path = path.into();
        let file_contents = tokio::fs::read_to_string(&path).await.map_err(Error::Io)?;
        let parsed: Root =
            serde_json::from_str(&file_contents).map_err(|e| Error::MessageParse { path, error: e })?;

        let mut messages = Vec::new();

        for message in parsed {
            let message_parts = (message[0].clone(), message[1].clone());
            messages.push(message_parts);
        }

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
