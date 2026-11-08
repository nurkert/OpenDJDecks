//! The platform data directory (library, config, stem cache, models,
//! update downloads).
//!
//! | Platform | Directory |
//! |---|---|
//! | macOS | `~/Library/Application Support/OpenDJDecks` |
//! | Windows | `%LOCALAPPDATA%\\OpenDJDecks` |
//! | Linux & others | `$XDG_DATA_HOME/opendjdecks` (`~/.local/share/…`) |
//!
//! `OPENDJDECKS_DATA_DIR` overrides the directory.

use std::path::{Path, PathBuf};

/// Folder name on macOS and Windows.
pub const DIR_NAME: &str = "OpenDJDecks";
/// Folder name under `$XDG_DATA_HOME`.
pub const XDG_DIR_NAME: &str = "opendjdecks";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Platform {
    MacOs,
    Windows,
    /// Linux and other Unix systems (XDG).
    Xdg,
}

impl Platform {
    pub fn current() -> Self {
        if cfg!(target_os = "macos") {
            Platform::MacOs
        } else if cfg!(windows) {
            Platform::Windows
        } else {
            Platform::Xdg
        }
    }
}

/// The data directory for `platform` (pure: the caller supplies `$HOME`,
/// `$XDG_DATA_HOME` and `%LOCALAPPDATA%`).
pub fn dir_for_platform(
    platform: Platform,
    home: &Path,
    xdg_data_home: Option<&Path>,
    local_app_data: Option<&Path>,
) -> PathBuf {
    match platform {
        Platform::MacOs => home.join("Library/Application Support").join(DIR_NAME),
        // Local (not roaming): the stem cache and analysis data are large.
        Platform::Windows => local_app_data
            .map(Path::to_path_buf)
            .unwrap_or_else(|| home.join("AppData").join("Local"))
            .join(DIR_NAME),
        Platform::Xdg => xdg_data_home
            .map(Path::to_path_buf)
            .unwrap_or_else(|| home.join(".local/share"))
            .join(XDG_DIR_NAME),
    }
}

/// Platform data directory of OpenDJDecks.
pub fn data_dir() -> PathBuf {
    if let Some(dir) = od_core::env::var_os("DATA_DIR") {
        return PathBuf::from(dir);
    }
    let home = std::env::home_dir().unwrap_or_else(|| PathBuf::from("."));
    let xdg = std::env::var_os("XDG_DATA_HOME").map(PathBuf::from);
    let local = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
    dir_for_platform(Platform::current(), &home, xdg.as_deref(), local.as_deref())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> PathBuf {
        PathBuf::from(s)
    }

    #[test]
    fn platform_directories() {
        assert_eq!(
            dir_for_platform(Platform::MacOs, &p("/Users/dj"), None, None),
            p("/Users/dj/Library/Application Support/OpenDJDecks")
        );
        assert_eq!(
            dir_for_platform(Platform::Xdg, &p("/home/dj"), None, None),
            p("/home/dj/.local/share/opendjdecks")
        );
        assert_eq!(
            dir_for_platform(Platform::Xdg, &p("/home/dj"), Some(&p("/data")), None),
            p("/data/opendjdecks")
        );
        assert_eq!(
            dir_for_platform(
                Platform::Windows,
                &p("C:/Users/dj"),
                None,
                Some(&p("C:/Users/dj/AppData/Local")),
            ),
            p("C:/Users/dj/AppData/Local").join("OpenDJDecks")
        );
    }
}
