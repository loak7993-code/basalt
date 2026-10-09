//! Basalt's filesystem layout: XDG data dir for installs and accounts,
//! XDG config dir for overrides.

use std::path::PathBuf;

pub fn data_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("BASALT_DATA") {
        return PathBuf::from(dir);
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join(".local/share/basalt")
}

pub fn config_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("BASALT_CONFIG") {
        return PathBuf::from(dir);
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join(".config/basalt")
}

pub fn versions_dir() -> PathBuf {
    data_dir().join("versions")
}

pub fn assets_dir() -> PathBuf {
    data_dir().join("assets")
}

pub fn libraries_dir() -> PathBuf {
    data_dir().join("libraries")
}

pub fn natives_dir() -> PathBuf {
    data_dir().join("natives")
}

pub fn accounts_file() -> PathBuf {
    data_dir().join("accounts.json")
}

pub fn version_dir(id: &str) -> PathBuf {
    versions_dir().join(id)
}

pub fn ensure_dir(dir: &PathBuf) {
    let _ = std::fs::create_dir_all(dir);
}
