//! Plan building: classpath, JVM args, game args, and the full launch plan.

use crate::manifest::{DownloadArtifact, Platform, VersionManifest};
use crate::policy::GcPolicy;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Paths the plan is built against.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallLayout {
    pub client_jar: PathBuf,
    pub libraries_dir: PathBuf,
    pub natives_dir: PathBuf,
    pub game_dir: PathBuf,
    pub assets_dir: PathBuf,
    pub asset_index_id: String,
    pub runtime_home: PathBuf,
}

/// Account context for game argument substitution.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthContext {
    pub player_name: String,
    pub uuid: String,
    #[serde(default = "default_token")]
    pub access_token: String,
    #[serde(default = "default_user_type")]
    pub user_type: String,
    #[serde(default)]
    pub client_id: String,
    #[serde(default)]
    pub xuid: String,
    #[serde(default)]
    pub session_id: String,
}

fn default_token() -> String {
    "0".into()
}

fn default_user_type() -> String {
    "msa".into()
}

impl Default for AuthContext {
    fn default() -> Self {
        AuthContext {
            player_name: String::new(),
            uuid: String::new(),
            access_token: default_token(),
            user_type: default_user_type(),
            client_id: String::new(),
            xuid: String::new(),
            session_id: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum QuickPlay {
    Singleplayer { save_name: String },
    Server { address: String, port: Option<i64> },
}

/// The immutable launch plan: everything needed to start the game JVM.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchPlan {
    pub jvm_args: Vec<String>,
    pub game_args: Vec<String>,
    pub main_class: String,
    pub classpath: Vec<String>,
    pub classpath_string: String,
    pub wrapper_class: Option<String>,
    pub required_java_major: Option<i64>,
    pub log_config_sha1: Option<String>,
}

impl LaunchPlan {
    /// Full exec list: java binary + jvm args + (wrapper, main class) + game args.
    pub fn to_command_list(&self, java_binary: &str) -> Vec<String> {
        let mut command = Vec::with_capacity(4 + self.jvm_args.len() + self.game_args.len());
        command.push(java_binary.to_string());
        command.extend(self.jvm_args.iter().cloned());
        match &self.wrapper_class {
            Some(wrapper) if !wrapper.is_empty() => {
                command.push(wrapper.clone());
                command.push(self.main_class.clone());
            }
            _ => command.push(self.main_class.clone()),
        }
        command.extend(self.game_args.iter().cloned());
        command
    }
}

/// Caller-controlled knobs. None to take manifest defaults untouched, or a
/// policy that replaces heap and collector selection precisely.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineOptions {
    #[serde(default)]
    pub memory: Option<MemoryOptions>,
    /// "tunedG1" | "none" | absent (manifest defaults pass through)
    #[serde(default)]
    pub gc: Option<String>,
    #[serde(default)]
    pub extra_jvm_args: Vec<String>,
    #[serde(default)]
    pub extra_game_args: Vec<String>,
    #[serde(default)]
    pub active_processors: Option<i64>,
    #[serde(default = "default_true")]
    pub strip_always_pre_touch: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryOptions {
    pub heap_mb: i64,
    #[serde(default = "default_min")]
    pub min_mb: i64,
    #[serde(default = "default_max")]
    pub max_mb: i64,
}

fn default_min() -> i64 {
    512
}

fn default_max() -> i64 {
    8192
}

const WRAPPER_CLASS: &str = "mio.Wrapper";
const ALWAYS_PRE_TOUCH_MIN_HEAP_MB: i64 = 4096;

/// The plan builder. Holds the manifest so substitution can render the
/// classpath in a single pass over libraries.
pub struct PlanBuilder<'a> {
    pub manifest: &'a VersionManifest,
    pub platform: Platform,
    pub layout: &'a InstallLayout,
    pub features: crate::rules::Features,
    pub options: EngineOptions,
    pub auth: AuthContext,
    pub version_name: String,
    pub version_type: String,
    pub quick_play: Option<QuickPlay>,
    pub log_config_path: Option<PathBuf>,
}

impl<'a> PlanBuilder<'a> {
    pub fn build(&self) -> LaunchPlan {
        let mut jvm_args: Vec<String> = Vec::new();

        if let Some(args) = &self.manifest.arguments {
            self.evaluate_into(&args.jvm, &mut jvm_args);
            self.evaluate_into(&args.default_user_jvm, &mut jvm_args);
        }

        if let (Some(path), Some(logging)) = (&self.log_config_path, &self.manifest.logging) {
            if let Some(client) = logging.get("client") {
                jvm_args.push(client.argument.replace("${path}", &path.to_string_lossy()));
            }
        }

        jvm_args.extend(self.options.extra_jvm_args.iter().cloned());

        let mut heap_mb: Option<i64> = None;
        if let Some(mem) = &self.options.memory {
            let policy = crate::policy::MemoryPolicy {
                heap_mb: mem.heap_mb,
                min_mb: mem.min_mb,
                max_mb: mem.max_mb,
            };
            heap_mb = Some(policy.effective_heap_mb());
            policy.apply_heap_to(&mut jvm_args);
        }

        if self.options.strip_always_pre_touch
            && heap_mb.map(|h| h < ALWAYS_PRE_TOUCH_MIN_HEAP_MB).unwrap_or(false)
        {
            jvm_args.retain(|arg| arg != "-XX:+AlwaysPreTouch");
        }

        match self.options.gc.as_deref() {
            Some("tunedG1") => GcPolicy::TunedG1.apply_to(&mut jvm_args),
            Some("none") => GcPolicy::None.apply_to(&mut jvm_args),
            _ => {}
        }

        if let Some(count) = self.options.active_processors {
            jvm_args.push(format!("-XX:ActiveProcessorCount={}", count));
        }

        let mut game_args: Vec<String> = Vec::new();
        if let Some(flat) = &self.manifest.minecraft_arguments {
            game_args.extend(
                flat.split(' ')
                    .filter(|s| !s.is_empty())
                    .map(|s| self.substitute(s)),
            );
        } else if let Some(args) = &self.manifest.arguments {
            self.evaluate_into(&args.game, &mut game_args);
        }

        if let Some(qp) = &self.quick_play {
            match qp {
                QuickPlay::Singleplayer { save_name } => {
                    game_args.push("--quickPlaySingleplayer".into());
                    game_args.push(save_name.clone());
                }
                QuickPlay::Server { address, port } => {
                    let port = port.unwrap_or(25565);
                    game_args.push("--quickPlayMultiplayer".into());
                    game_args.push(format!("{}:{}", address, port));
                }
            }
        }
        game_args.extend(self.options.extra_game_args.iter().cloned());

        let classpath = self.build_classpath();
        LaunchPlan {
            jvm_args,
            game_args,
            main_class: self.manifest.main_class.clone(),
            classpath: classpath.clone(),
            classpath_string: classpath.join(":"),
            wrapper_class: Some(WRAPPER_CLASS.to_string()),
            required_java_major: self.manifest.java_version.as_ref().map(|j| j.major_version),
            log_config_sha1: self
                .manifest
                .logging
                .as_ref()
                .and_then(|l| l.get("client"))
                .and_then(|c| c.file.as_ref())
                .and_then(|f| f.sha1.clone()),
        }
    }

    /// Client jar first, then one entry per allowed library with an artifact.
    pub fn build_classpath(&self) -> Vec<String> {
        let mut entries = vec![self.layout.client_jar.to_string_lossy().to_string()];
        for library in &self.manifest.libraries {
            if let Some(artifact) = self.classpath_artifact(library) {
                entries.push(
                    self.layout
                        .libraries_dir
                        .join(artifact.path.as_deref().unwrap_or_default())
                        .to_string_lossy()
                        .to_string(),
                );
            }
        }
        entries
    }

    pub fn classpath_artifact<'b>(
        &'b self,
        library: &'b crate::manifest::LibraryEntry,
    ) -> Option<&'b DownloadArtifact> {
        let context = self.rule_context();
        if !context.is_allowed(library) {
            return None;
        }
        library
            .downloads
            .as_ref()
            .and_then(|d| d.artifact.as_ref())
            .filter(|a| a.path.as_deref().map(|p| !p.trim().is_empty()).unwrap_or(false))
    }

    fn rule_context(&self) -> crate::rules::RuleContext {
        crate::rules::RuleContext {
            platform: self.platform,
            features: self.features.clone(),
        }
    }

    fn evaluate_into(&self, entries: &[serde_json::Value], out: &mut Vec<String>) {
        for entry in entries {
            match entry {
                serde_json::Value::String(s) => out.push(self.substitute(s)),
                serde_json::Value::Object(obj) => {
                    if let Some(rules) = obj.get("rules").and_then(|r| {
                        serde_json::from_value::<Vec<crate::manifest::LibraryRule>>(r.clone()).ok()
                    }) {
                        let context = self.rule_context();
                        let mut any_allow = false;
                        let mut denied = false;
                        for rule in &rules {
                            let matches = context.rule_matches(rule);
                            if matches && rule.action.eq_ignore_ascii_case("allow") {
                                any_allow = true;
                            }
                            if matches && rule.action.eq_ignore_ascii_case("disallow") {
                                denied = true;
                            }
                        }
                        if denied || (!rules.is_empty() && !any_allow) {
                            continue;
                        }
                    }
                    match obj.get("value") {
                        Some(serde_json::Value::String(s)) => out.push(self.substitute(s)),
                        Some(serde_json::Value::Array(arr)) => {
                            for el in arr {
                                if let Some(s) = el.as_str() {
                                    out.push(self.substitute(s));
                                }
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }

    fn substitute(&self, raw: &str) -> String {
        let classpath = self.build_classpath().join(":");
        let width = self
            .features
            .resolution_width
            .map(|w| w.to_string())
            .unwrap_or_else(|| "854".into());
        let height = self
            .features
            .resolution_height
            .map(|h| h.to_string())
            .unwrap_or_else(|| "480".into());

        let mut out = raw.to_string();
        for (key, value) in [
            ("auth_player_name", self.auth.player_name.as_str()),
            ("auth_uuid", self.auth.uuid.as_str()),
            ("auth_access_token", self.auth.access_token.as_str()),
            ("user_type", self.auth.user_type.as_str()),
            ("clientid", self.auth.client_id.as_str()),
            ("auth_xuid", self.auth.xuid.as_str()),
            ("auth_session_id", self.auth.session_id.as_str()),
            ("version_name", self.version_name.as_str()),
            ("version_type", self.version_type.as_str()),
            ("game_directory", &self.layout.game_dir.to_string_lossy()),
            ("assets_root", &self.layout.assets_dir.to_string_lossy()),
            ("assets_index_id", &self.layout.asset_index_id),
            ("assets_index_name", &self.layout.asset_index_id),
            ("natives_directory", &self.layout.natives_dir.to_string_lossy()),
            ("library_directory", &self.layout.libraries_dir.to_string_lossy()),
            ("classpath", classpath.as_str()),
            ("classpath_separator", ":"),
            ("resolution_width", width.as_str()),
            ("resolution_height", height.as_str()),
            ("os_name", self.platform.os_name()),
            ("os_arch", self.platform.arch_name()),
            ("user_home", &self.layout.game_dir.to_string_lossy()),
        ] {
            let pattern = std::format!("${{{}}}", key);
            if out.contains(&pattern) {
                out = out.replace(&pattern, value);
            }
        }
        out
    }
}
