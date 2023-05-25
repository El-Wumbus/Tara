use std::path;

use crate::{Error, Result};

#[cfg(target_os = "linux")]
mod defaults {
    pub const FALLBACK_DATABASE_DIRECTORY: &str = "/var/db/tara/";
    pub const FALLBACK_CONFIG_FILE: &str = "/etc/tara.d/tara.toml";
    pub const FALLBACK_ERROR_MESSAGES_FILE: &str = "/etc/tara.d/error_messages.json";
}

#[cfg(not(target_os = "linux"))]
mod defaults {
    pub const FALLBACK_CONFIG_FILE: &str = "";
    pub const FALLBACK_DATABASE_DIRECTORY: &str = "";
    pub const FALLBACK_ERROR_MESSAGES_FILE: &str = "";
}

#[inline]
pub fn project_dir() -> Option<directories::ProjectDirs> {
    directories::ProjectDirs::from("com.github", "El-Wumbus", "Tara")
}

/// Returns a configuration file after checking some of the default locations.
///
/// # File Locations
///
/// The lower the number, before it checks.
///
/// ## Linux
///
/// 1. `$XDG_CONFIG_HOME/Tara/tara.toml` or `$HOME/.config/Tara/tara.toml`
/// 2. `/etc/tara.d/tara.toml`
///
/// ## MacOS
///
/// 1. `$HOME/Library/Application Support/com.github.El-Wumbus.Tara/tara.toml`
///
/// ## Windows
///
/// 1. `%APPDATA%\Tara\config\tara.toml`
///
/// # Errors
///
/// Returns an `Error` when:
///
/// - No configuration file is found
pub fn config_file_path() -> Result<path::PathBuf> {
    let mut paths = Vec::with_capacity(2);
    if let Some(project_dirs) = project_dir() {
        paths.push(project_dirs.config_dir().join("tara.toml"));
    }
    if !defaults::FALLBACK_CONFIG_FILE.is_empty() {
        paths.push(path::PathBuf::from(defaults::FALLBACK_CONFIG_FILE))
    }

    match paths.into_iter().find(|path| path.is_file()) {
        None => Err(Error::MissingConfigurationFile),
        Some(x) => Ok(x),
    }
}

/// Returns a configuration file after checking some of the default locations.
///
/// # File Locations
///
/// The lower the number, before it checks.
///
/// ## Linux
///
/// 1. `$XDG_DATA_HOME/Tara` or `$HOME/.local/share/Tara/`
/// 2. `/var/db/tara/`
///
/// ## MacOS
///
/// 1. `$HOME/Library/Application Support/com.github.El-Wumbus.Tara/`
///
/// ## Windows
///
/// 1. `%APPDATA%\Tara\data`
///
/// # Errors
///
/// Returns an `Error` when:
///
/// - No configuration file is found
pub fn database_directory() -> Result<path::PathBuf> {
    let mut paths = Vec::with_capacity(2);
    if let Some(project_dirs) = project_dir() {
        paths.push(path::PathBuf::from(project_dirs.data_dir()));
    }
    if !defaults::FALLBACK_DATABASE_DIRECTORY.is_empty() {
        paths.push(path::PathBuf::from(defaults::FALLBACK_DATABASE_DIRECTORY))
    }

    match paths.into_iter().find(|path| path.is_dir()) {
        None => Err(Error::DatabaseFile),
        Some(x) => Ok(x),
    }
}

/// Returns a configuration file after checking some of the default locations.
///
/// # File Locations
///
/// The lower the number, before it checks.
///
/// ## Linux
///
/// 1. `$XDG_CONFIG_HOME/Tara/error_messages.json` or
/// `$HOME/.config/Tara/error_messages.json` 2. `/etc/tara.d/error_messages.json`
///
/// ## MacOS
///
/// 1. `$HOME/Library/Application Support/com.github.El-Wumbus.Tara/error_messages.json`
///
/// ## Windows
///
/// 1. `%APPDATA%\Tara\config\error_messages.json`
pub fn error_messages_file_path() -> Option<path::PathBuf> {
    let mut paths = Vec::with_capacity(2);
    if let Some(project_dirs) = project_dir() {
        paths.push(project_dirs.config_dir().join("error_messages.json"));
    }
    if !defaults::FALLBACK_ERROR_MESSAGES_FILE.is_empty() {
        paths.push(path::PathBuf::from(defaults::FALLBACK_ERROR_MESSAGES_FILE))
    }

    paths.into_iter().find(|path| path.is_file())
}
