use thiserror::Error;
use tokio::{io, task};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Error: {0}")]
    Unexpected(&'static str),

    #[error("ClientInitializationError: {0}")]
    ClientInitialization(Box<serenity::Error>),

    #[error("IOError: {0}")]
    Io(io::Error),

    #[error("InternalError/JoinError: {0}")]
    JoinError(Box<task::JoinError>),

    /// There's no configuration file to parse.
    #[error("MissingConfigurationFile: No configuration file found.")]
    MissingConfigurationFile,

    /// Configuration parsing failed
    #[error("ConfigurationParseError: \"{}\": {error}", path.display())]
    ConfigurationParse {
        path:  std::path::PathBuf,
        error: Box<toml::de::Error>,
    },

    #[error("ConfigurationSaveError: \"{}\": {error}", path.display())]
    ConfigurationSave {
        path:  std::path::PathBuf,
        error: Box<toml::ser::Error>,
    },

    #[error("MessageParseError: \"{}\": {error}", path.display())]
    MessageParse {
        path:  std::path::PathBuf,
        error: serde_json::Error,
    },

    #[error("ExpectedSuboptionError: A suboption was exepcted but discord didn't provide one.")]
    ExpectedSuboption,

    #[error("HTTPRequestError: {0}")]
    HttpRequest(reqwest::Error),

    #[error("HTTPRequestError: {0}")]
    SerenityHttpRequest(Box<serenity::Error>),

    #[error("CommandMisuseError: {0}")]
    CommandMisuse(String),

    #[error("JSONParseError: {0}")]
    JsonParse(String),

    #[error("WikipedaSearch: Page not found for \"{0}\"")]
    WikipedaSearch(String),

    #[error("RedisError: {0}")]
    RedisError(String),

    #[error("DatabaseError: {0}")]
    Database(Box<sqlx::Error>),

    #[error("InternalLogicError: Something's wrong on this end! Sorry.")]
    InternalLogic,

    #[error("ParseNumberError: {0}")]
    ParseNumber(String),

    #[error("FeatureDisabled: {0}")]
    FeatureDisabled(String),

    #[error("NoSearchResultsError: No search results found for \"{0}\"")]
    NoSearchResults(String),

    #[error(
        "InappropriateSearchError: Attempted to search for sexual or profane content using term \"{0}\""
    )]
    InappropriateSearch(String),

    #[error("DirectMessageCooldownError: Cooldown should end in {0}")]
    DirectMessageCooldown(chrono::Duration),

    #[error("RoleNotAssignableError: \"{0}\" isn't an assignable role")]
    RoleNotAssignable(String),

    #[error("UserRoleError: \"{0}\"")]
    UserRole(Box<serenity::Error>),

    #[error("DatabaseFileError: Couldn't find anywhere to open or create a database.")]
    DatabaseFile,

    #[error("ReadLineError: {0}")]
    ReadLine(rustyline::error::ReadlineError),

    #[error("UndefinedWordError: {0}")]
    UndefinedWord(String),

    #[cfg(feature = "music")]
    #[error("Error joining voice channel: {0}")]
    JoinVoiceChannel(Box<songbird::error::JoinError>),

    #[cfg(feature = "music")]
    #[error("YouTubeInfoError: {0}")]
    YoutubeInfo(String),

    #[error("SerenityError(backend framework): {0}")]
    SerenityErr(Box<serenity::Error>),
}

impl From<io::Error> for Error {
    fn from(value: io::Error) -> Self { Self::Io(value) }
}

impl From<sqlx::Error> for Error {
    fn from(value: sqlx::Error) -> Self { Self::Database(Box::new(value)) }
}

impl Error {
    const fn _code(&self) -> u8 {
        match self {
            Error::Database(_) => 0,
            Error::ClientInitialization(_) => 1,
            Error::Io(_) => 2,
            Error::MissingConfigurationFile => 3,
            Error::ConfigurationParse { .. } => 4,
            Error::MessageParse { .. } => 5,
            Error::ExpectedSuboption => 6,
            Error::HttpRequest(_) => 7,
            Error::CommandMisuse(_) => 8,
            Error::JsonParse(_) => 9,
            Error::WikipedaSearch(_) => 10,
            Error::Unexpected(_) => 12,
            Error::RedisError(_) => 13,
            Error::InternalLogic => 14,
            Error::ParseNumber(_) => 15,
            Error::FeatureDisabled(_) => 16,
            Error::NoSearchResults(_) => 17,
            Error::InappropriateSearch(_) => 18,
            Error::DirectMessageCooldown(_) => 19,
            Error::RoleNotAssignable(_) => 20,
            Error::UserRole(_) => 21,
            Error::DatabaseFile => 22,
            Error::ReadLine(_) => 23,
            Error::ConfigurationSave { .. } => 24,
            Error::SerenityHttpRequest(_) => 25,
            Error::JoinError(_) => 26,
            Error::UndefinedWord(_) => 27,
            #[cfg(feature = "music")]
            Error::JoinVoiceChannel(_) => 29,
            #[cfg(feature = "music")]
            Error::YoutubeInfo(_) => 30,
            Error::SerenityErr(_) => 31,
        }
    }

    /// Return a hex-formatted error code associated with the error
    #[must_use]
    pub fn code(&self) -> String { format!("0x{:02X}", self._code()) }
}


impl From<task::JoinError> for Error {
    fn from(value: task::JoinError) -> Self { Self::JoinError(Box::new(value)) }
}

impl From<reqwest::Error> for Error {
    fn from(value: reqwest::Error) -> Self { Self::HttpRequest(value) }
}

impl From<serenity::Error> for Error {
    fn from(value: serenity::Error) -> Self { Self::SerenityErr(Box::new(value)) }
}


#[cfg(feature = "music")]
impl From<youtubei_rs::types::error::Errors> for Error {
    fn from(value: youtubei_rs::types::error::Errors) -> Self { Self::YoutubeInfo(format!("{value:?}")) }
}
#[cfg(feature = "music")]
impl From<songbird::error::JoinError> for Error {
    fn from(value: songbird::error::JoinError) -> Self { Self::JoinVoiceChannel(Box::new(value)) }
}
