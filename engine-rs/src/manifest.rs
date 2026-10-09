//! The slice of Mojang's version manifest the engine needs to plan a launch.

use serde::Deserialize;
use std::collections::HashMap;

/// Top-level version manifest. Unknown fields are ignored so new manifest
/// versions keep parsing.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionManifest {
    pub id: String,
    pub main_class: String,
    #[serde(default)]
    pub assets: Option<String>,
    #[serde(default)]
    pub asset_index: Option<AssetIndex>,
    #[serde(default)]
    pub java_version: Option<JavaRequirement>,
    #[serde(default)]
    pub downloads: HashMap<String, DownloadEntry>,
    #[serde(default)]
    pub libraries: Vec<LibraryEntry>,
    #[serde(default)]
    pub arguments: Option<ManifestArguments>,
    #[serde(default)]
    pub logging: Option<HashMap<String, LoggingConfig>>,
    /// Pre-1.13 versions carry a flat argument string instead of `arguments`.
    #[serde(default)]
    pub minecraft_arguments: Option<String>,
    #[serde(default)]
    pub r#type: Option<String>,
    #[serde(default)]
    pub release_time: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetIndex {
    pub id: String,
    pub url: String,
    #[serde(default)]
    pub total_size: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JavaRequirement {
    pub component: String,
    pub major_version: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadEntry {
    pub url: String,
    #[serde(default)]
    pub sha1: Option<String>,
    #[serde(default)]
    pub size: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoggingConfig {
    pub argument: String,
    #[serde(default)]
    pub file: Option<DownloadEntry>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestArguments {
    #[serde(default)]
    pub game: Vec<serde_json::Value>,
    #[serde(default)]
    pub jvm: Vec<serde_json::Value>,
    /// Mojang's recommended JVM defaults for this version, rule-gated.
    #[serde(default, rename = "default-user-jvm")]
    pub default_user_jvm: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryEntry {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub downloads: Option<LibraryDownloads>,
    #[serde(default)]
    pub rules: Vec<LibraryRule>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub natives: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryDownloads {
    #[serde(default)]
    pub artifact: Option<DownloadArtifact>,
    #[serde(default)]
    pub classifiers: HashMap<String, DownloadArtifact>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadArtifact {
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub sha1: Option<String>,
    #[serde(default)]
    pub size: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryRule {
    pub action: String,
    #[serde(default)]
    pub os: Option<RuleOs>,
    #[serde(default)]
    pub features: Option<HashMap<String, bool>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleOs {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub arch: Option<String>,
}

/// The platform the engine plans for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    LinuxArm64,
    LinuxX8664,
}

impl Platform {
    pub fn os_name(&self) -> &'static str {
        "linux"
    }

    pub fn arch_name(&self) -> &'static str {
        match self {
            Platform::LinuxArm64 => "arm64",
            Platform::LinuxX8664 => "x86_64",
        }
    }

    /// Maps raw system properties to a platform; None when unsupported.
    pub fn from(os: &str, arch: &str) -> Option<Platform> {
        if os.eq_ignore_ascii_case("linux") {
            match arch {
                "aarch64" | "arm64" => Some(Platform::LinuxArm64),
                "amd64" | "x86_64" => Some(Platform::LinuxX8664),
                _ => None,
            }
        } else {
            None
        }
    }
}
