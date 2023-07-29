use std::path::PathBuf;

use chrono::Utc;
use directories::ProjectDirs;
use lazy_static::lazy_static;

#[cfg(target_os = "linux")]
mod defaults {
    pub const FALLBACK_CONFIG_FILE: &str = "/etc/tara.d/tara.toml";
    pub const FALLBACK_ERROR_MESSAGES_FILE: &str = "/etc/tara.d/error_messages.json";
    pub const FALLBACK_SOCKET_DIRECTORY: &str = "/var";
}

#[cfg(not(target_os = "linux"))]
mod defaults {
    pub const FALLBACK_CONFIG_FILE: &str = "";
    pub const FALLBACK_DATABASE_DIRECTORY: &str = "";
    pub const FALLBACK_ERROR_MESSAGES_FILE: &str = "";
    pub const FALLBACK_SOCKET_DIRECTORY: &str = "";
}

lazy_static! {
pub static ref TARA_PROJECT_DIR: Option<directories::ProjectDirs> = directories::ProjectDirs::from("com.github", "El-Wumbus", "Tara");

/// An existing configuration file.
///
/// # File Locations
///
/// ## Linux
///
/// 1. `$XDG_CONFIG_HOME/Tara/tara.toml` or `$HOME/.config/Tara/tara.toml`
/// 2. `/etc/tara.d/tara.toml`
///
/// ## macOS
///
/// 1. `$HOME/Library/Application Support/com.github.El-Wumbus.Tara/tara.toml`
///
/// ## Windows
///
/// 1. `%APPDATA%\Tara\config\tara.toml`
pub static ref TARA_CONFIGURATION_FILE: Option<PathBuf> = {
    let mut paths = Vec::with_capacity(2);
    if let Some(project_dirs) = TARA_PROJECT_DIR.as_ref() {
        paths.push(project_dirs.config_dir().join("tara.toml"));
    }
    if !defaults::FALLBACK_CONFIG_FILE.is_empty() {
        paths.push(PathBuf::from(defaults::FALLBACK_CONFIG_FILE));
    }

    paths.into_iter().find(|path| path.is_file())
};

/// # File Locations
///
/// ## Linux
///
/// 1. `$XDG_CONFIG_HOME/Tara/error_messages.json` or
/// `$HOME/.config/Tara/error_messages.json`
/// 2. `/etc/tara.d/error_messages.json`
///
/// ## macOS
///
/// 1. `$HOME/Library/Application Support/com.github.El-Wumbus.Tara/error_messages.json`
///
/// ## Windows
///
/// 1. `%APPDATA%\Tara\config\error_messages.json`
pub static ref ERROR_MESSAGES_FILE: Option<PathBuf> = {
    let mut paths = Vec::with_capacity(2);
    if let Some(project_dirs) = TARA_PROJECT_DIR.as_ref() {
        paths.push(project_dirs.config_dir().join("error_messages.json"));
    }
    if !defaults::FALLBACK_ERROR_MESSAGES_FILE.is_empty() {
        paths.push(PathBuf::from(defaults::FALLBACK_ERROR_MESSAGES_FILE));
    }

    paths.into_iter().find(|path| path.is_file())
};

pub static ref TARA_IPC_SOCKET_FILE: String = {
    use interprocess::local_socket::NameTypeSupport;

    const SOCKET_NAME: &str = "tara_bot.sock";
    let create_namespaced = {
        use NameTypeSupport::{Both, OnlyNamespaced, OnlyPaths};
        let nts = NameTypeSupport::query();
        match (nts, false) {
            (OnlyNamespaced, _) | (Both, true) => true,
            (OnlyPaths, _) | (Both, false) => false,
        }
    };

    let mut paths = Vec::with_capacity(2);

    if create_namespaced {
        #[cfg(target_family = "windows")]
        unimplemented!("Please host on Linux, macOS, or some other UNIX!");
        #[cfg(not(target_family = "windows"))]
        unreachable!();
    } else if cfg!(target_family = "windows") { // This is unlikely to happen
        if let Some(socket) = TARA_PROJECT_DIR
            .as_ref()
            .map(|x| x.data_dir().join(SOCKET_NAME).to_string_lossy().to_string())
        {
            paths.push(socket);
        }
    } else if let Some(socket_dir) = TARA_PROJECT_DIR.as_ref().and_then(ProjectDirs::runtime_dir) {
        paths.push(socket_dir.join(SOCKET_NAME).to_string_lossy().to_string());
    } else if !defaults::FALLBACK_ERROR_MESSAGES_FILE.is_empty() {
        paths.push(
            PathBuf::from(defaults::FALLBACK_SOCKET_DIRECTORY)
                .join(SOCKET_NAME)
                .to_string_lossy()
                .to_string(),
        );
    }

    paths.into_iter().next().unwrap()
};

pub static ref TARA_COMMAND_LOG_PATH: PathBuf = {
    TARA_PROJECT_DIR.as_ref().unwrap().data_dir().join(format!(
        "command-log_{}.csv",
        Utc::now().format("%Y-%m")
    ))
};
}
