use std::path;

use crate::{Error, Result};

#[cfg(target_os = "linux")]
pub const FALLBACK_DATABASE_DIRECTORY: &str = "/var/db/tara/";

#[cfg(target_os = "linux")]
pub const FALLBACK_CONFIG_FILE: &str = "/etc/tara.d/tara.toml";

#[cfg(not(target_os = "linux"))]
pub const FALLBACK_CONFIG_FILE: &str = "";

#[cfg(not(target_os = "linux"))]
pub const FALLBACK_DATABASE_DIRECTORY: &str = "";

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
pub fn config_file_path() -> Result<path::PathBuf>
{
    use directories::ProjectDirs;
    let file = if let Some(project_dirs) = ProjectDirs::from("com.github", "El-Wumbus", "Tara") {
        let x = project_dirs.config_dir().join("tara.toml");
        if !x.is_file() {
            path::PathBuf::from(FALLBACK_CONFIG_FILE)
        }
        else {
            x
        }
    }
    else if !FALLBACK_CONFIG_FILE.is_empty() {
        path::PathBuf::from(FALLBACK_CONFIG_FILE)
    }
    else {
        return Err(Error::MissingConfigurationFile);
    };

    if !file.is_file() {
        return Err(Error::MissingConfigurationFile);
    }
    Ok(file)
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
pub fn database_directory() -> Result<path::PathBuf>
{
    use directories::ProjectDirs;
    let dir = if let Some(project_dirs) = ProjectDirs::from("com.github", "El-Wumbus", "Tara") {
        path::PathBuf::from(project_dirs.data_dir())
    }
    else if !FALLBACK_DATABASE_DIRECTORY.is_empty() {
        path::PathBuf::from(FALLBACK_DATABASE_DIRECTORY)
    }
    else {
        return Err(Error::DatabaseFile);
    };

    Ok(dir)
}
