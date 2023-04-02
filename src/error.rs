use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error
{
    #[error("ClientInitializationError: {0}")]
    ClientInitialization(serenity::Error),

    #[error("IOError: {0}")]
    Io(tokio::io::Error),

    #[error("MissingConfigurationFile: {0}")]
    MissingConfigurationFile(String),

    #[error("ConfigurationParseError: \"{}\": {error}", path.display())]
    ConfigurationParse
    {
        path:  std::path::PathBuf,
        error: toml::de::Error,
    },

    #[error("MessageParseError: \"{}\": {error}", path.display())]
    MessageParse
    {
        path:  std::path::PathBuf,
        error: serde_json::Error,
    },

    #[error("ExpectedSuboptionError: A suboption was exepcted but discord didn't provide one.")]
    ExpectedSuboption,

    #[error("HTTPRequestError: {0}")]
    HttpRequest(reqwest::Error),

    #[error("CommandMisuseError: {0}")]
    CommandMisuse(String),

    #[error("JSONParseError: {0}")]
    JsonParse(String),

    #[error("WikipedaSearch: Page not found for \"{0}\"")]
    WikipedaSearch(String),

    #[error("DatabaseOpenError: {0}")]
    DatabaseOpen(r2d2::Error),

    #[error("DatabaseAccessError: {0}")]
    DatabaseAccess(rusqlite::Error),

    #[error("DatabaseAccessTimeoutError: {0}")]
    DatabaseAccessTimeout(r2d2::Error),

    #[error("NoDatabaseRecordError")]
    NoDatabaseRecord,

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
}

impl Error
{
    pub fn report(&self) -> &Self
    {
        log::error!("{self}");
        self
    }

    pub fn code(&self) -> String
    {
        let n = match self {
            Error::NoDatabaseRecord => 0,
            Error::ClientInitialization(_) => 1,
            Error::Io(_) => 2,
            Error::MissingConfigurationFile(_) => 3,
            Error::ConfigurationParse { .. } => 4,
            Error::MessageParse { .. } => 5,
            Error::ExpectedSuboption => 6,
            Error::HttpRequest(_) => 7,
            Error::CommandMisuse(_) => 8,
            Error::JsonParse(_) => 9,
            Error::WikipedaSearch(_) => 10,
            Error::DatabaseOpen(_) => 11,
            Error::DatabaseAccess(_) => 12,
            Error::DatabaseAccessTimeout(_) => 13,
            Error::InternalLogic => 14,
            Error::ParseNumber(_) => 15,
            Error::FeatureDisabled(_) => 16,
            Error::NoSearchResults(_) => 17,
            Error::InappropriateSearch(_) => 18,
        };

        format!("0x{n:02X}")
    }
}

impl From<rusqlite::Error> for Error
{
    fn from(value: rusqlite::Error) -> Self { Self::DatabaseAccess(value) }
}
