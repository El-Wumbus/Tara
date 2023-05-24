pub mod error;
pub mod paths;
pub(crate) use error::{Error, Result};
pub mod commands;
pub mod config;
pub mod database;
pub(crate) mod defaults;
pub mod logging;

#[cfg(feature = "music")]
pub use reqwest::Client as HttpClient;

#[cfg(feature = "music")]
/// Used to insert a [`reqwest::Client`] into the [`serenity::prelude::Context`].
pub struct HttpKey;

#[cfg(feature = "music")]
impl serenity::prelude::TypeMapKey for HttpKey {
    type Value = HttpClient;
}
