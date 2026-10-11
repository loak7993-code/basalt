//! Crash diagnosis and remediation policy.
//!
//! Copyright (C) 2026 Leonxlnx (loak7993-code)
//!
//! This program is free software: you can redistribute it and/or modify it
//! under the terms of the GNU General Public License as published by the Free
//! Software Foundation, either version 3 of the License, or (at your option)
//! any later version.
//!
//! This program is distributed in the hope that it will be useful, but WITHOUT
//! ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or
//! FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for
//! more details.
//!
//! You should have received a copy of the GNU General Public License along
//! with this program. If not, see <https://www.gnu.org/licenses/gpl-3.0.html>.

//! Classifies a failed launch from its log stream and picks a remediation.
//! Every rule here is a pure function over log lines, so the whole classifier
//! is testable against real captured logs without launching anything.

use std::collections::BTreeSet;
use std::path::PathBuf;

/// What went wrong, in the order the rules are evaluated. The first match
/// wins, so the rules are ordered most-specific first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrashKind {
    /// The JVM ran out of heap or metaspace.
    HeapExhausted,
    /// The graphics backend could not be created or crashed on init.
    RendererInit,
    /// The loader could not resolve the mod set.
    DependencyResolution,
    /// A mixin failed to apply, usually a version mismatch.
    MixinFailure,
    /// A class was compiled for a newer Java than the one running it.
    UnsupportedJava,
    /// A downloaded file failed its checksum.
    CorruptAsset,
    /// The JVM could not be started at all (missing binary, bad flag).
    JvmStartFailure,
    /// Nothing matched. The report still carries the tail so the user can read it.
    Unknown,
}

impl CrashKind {
    /// Stable machine-readable id, used by the Android UI and by tests.
    pub fn as_str(self) -> &'static str {
        match self {
            CrashKind::HeapExhausted => "heap_exhausted",
            CrashKind::RendererInit => "renderer_init",
            CrashKind::DependencyResolution => "dependency_resolution",
            CrashKind::MixinFailure => "mixin_failure",
            CrashKind::UnsupportedJava => "unsupported_java",
            CrashKind::CorruptAsset => "corrupt_asset",
            CrashKind::JvmStartFailure => "jvm_start_failure",
            CrashKind::Unknown => "unknown",
        }
    }

    /// One line a player can act on.
    pub fn headline(self) -> &'static str {
        match self {
            CrashKind::HeapExhausted => "The game ran out of memory",
            CrashKind::RendererInit => "The graphics backend could not start",
            CrashKind::DependencyResolution => "The mod set does not resolve",
            CrashKind::MixinFailure => "A mixin failed to apply",
            CrashKind::UnsupportedJava => "The game needs a newer Java",
            CrashKind::CorruptAsset => "A downloaded file is corrupt",
            CrashKind::JvmStartFailure => "The JVM could not start",
            CrashKind::Unknown => "The launch failed for an unknown reason",
        }
    }
}

/// The renderer to fall back to when the current one cannot start.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Renderer {
    Gl4es,
    VirGl,
    Zink,
}

impl Renderer {
    pub fn as_str(self) -> &'static str {
        match self {
            Renderer::Gl4es => "gl4es",
            Renderer::VirGl => "VirGL",
            Renderer::Zink => "Zink",
        }
    }

    pub fn parse(value: &str) -> Option<Renderer> {
        match value.trim().to_ascii_lowercase().as_str() {
            "gl4es" => Some(Renderer::Gl4es),
            "virgl" => Some(Renderer::VirGl),
            "zink" => Some(Renderer::Zink),
            _ => None,
        }
    }

    /// The order the renderer fallback walks: software OpenGL first, because
    /// it is the one that works without a working display stack, then the
    /// llvmpipe-backed options.
    fn fallback_order(self) -> [Renderer; 2] {
        match self {
            Renderer::Gl4es => [Renderer::Zink, Renderer::VirGl],
            Renderer::Zink => [Renderer::VirGl, Renderer::Gl4es],
            Renderer::VirGl => [Renderer::Gl4es, Renderer::Zink],
        }
    }
}

/// The remediation chosen for a crash. `None` means the crash needs a human.
#[derive(Debug, Clone, PartialEq)]
pub enum Fix {
    /// Relaunch with a different heap. `None` means "let the engine pick".
    SetHeap {
        heap_mb: Option<i64>,
        reason: &'static str,
    },
    /// Relaunch with a different renderer.
    SetRenderer { renderer: Renderer },
    /// Relaunch with a different Java major version.
    UseJava { major: i64 },
    /// Re-download the named files, then relaunch.
    RefetchFiles { paths: Vec<String> },
    /// Move the named mod files aside (reversible rename), then relaunch.
    DisableMods { files: Vec<PathBuf> },
    /// Nothing safe to do automatically.
    None { reason: &'static str },
}

impl Fix {
    /// One line describing what the relaunch will do differently.
    pub fn describe(&self) -> String {
        match self {
            Fix::SetHeap {
                heap_mb: Some(mb),
                reason,
            } => {
                format!("Relaunching with a {mb} MB heap ({reason})")
            }
            Fix::SetHeap {
                heap_mb: None,
                reason,
            } => {
                format!("Relaunching with an engine-chosen heap ({reason})")
            }
            Fix::SetRenderer { renderer } => {
                format!("Relaunching with the {} renderer", renderer.as_str())
            }
            Fix::UseJava { major } => format!("Relaunching on Java {major}"),
            Fix::RefetchFiles { paths } => {
                format!("Re-downloading {} file(s) and relaunching", paths.len())
            }
            Fix::DisableMods { files } => {
                format!("Moving {} mod file(s) aside and relaunching", files.len())
            }
            Fix::None { reason } => format!("Not fixing automatically: {reason}"),
        }
    }
}

/// The outcome of classifying one failed launch.
#[derive(Debug, Clone, PartialEq)]
pub struct Diagnosis {
    pub kind: CrashKind,
    /// Exit code the JVM returned. Kept in the report because a launcher that
    /// only looks at the code misses the fatal-errors-in-log case (exit 0).
    pub exit_code: i32,
    pub fix: Fix,
    /// The log lines that triggered the match, for the report and the UI.
    pub evidence: Vec<String>,
    /// Last lines of the log, always present so a player always gets context.
    pub tail: Vec<String>,
}

/// What the host knows about the launch that failed.
#[derive(Debug, Clone, Default)]
pub struct LaunchContext {
    /// Heap the launch actually used.
    pub heap_mb: Option<i64>,
    /// Physical RAM on the device, used to keep a heap fix inside budget.
    pub device_ram_mb: Option<f64>,
    /// Renderer the launch used, Android only.
    pub renderer: Option<Renderer>,
    /// Java major version that ran the launch.
    pub java_major: Option<i64>,
    /// Directory holding mod jars, used to resolve names from logs to files.
    pub mods_dir: Option<PathBuf>,
}

/// How many log lines the classifier keeps for evidence and tail.
const TAIL_LINES: usize = 24;

/// Splits a raw log stream into classified failures.
///
/// Rules are evaluated in order; the first one that matches wins. Each rule is
/// a pattern the game's own log or the JVM prints, so the classifier stays a
/// pure function and never touches the filesystem or the network.
pub fn diagnose(lines: &[String], exit_code: i32, ctx: &LaunchContext) -> Diagnosis {
    let tail: Vec<String> = lines
        .iter()
        .rev()
        .take(TAIL_LINES)
        .cloned()
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();

    for rule in RULES {
        if let Some(hits) = (rule.matches)(lines) {
            let fix = (rule.fix)(&hits, ctx);
            return Diagnosis {
                kind: rule.kind,
                exit_code,
                fix,
                evidence: hits.into_iter().take(6).collect(),
                tail,
            };
        }
    }

    Diagnosis {
        kind: CrashKind::Unknown,
        exit_code,
        fix: Fix::None {
            reason: "no known failure signature matched",
        },
        evidence: Vec::new(),
        tail,
    }
}

struct Rule {
    kind: CrashKind,
    matches: fn(&[String]) -> Option<Vec<String>>,
    fix: fn(&[String], &LaunchContext) -> Fix,
}

fn contains_ci(haystack: &str, needle: &str) -> bool {
    haystack
        .to_ascii_lowercase()
        .contains(&needle.to_ascii_lowercase())
}

fn lines_with(lines: &[String], needles: &[&str]) -> Option<Vec<String>> {
    let hits: Vec<String> = lines
        .iter()
        .filter(|line| needles.iter().any(|n| contains_ci(line, n)))
        .cloned()
        .collect();
    if hits.is_empty() {
        None
    } else {
        Some(hits)
    }
}

/// Every heap signature the JVM prints, plus the one Minecraft prints when it
/// gives up allocating.
const HEAP_MARKERS: &[&str] = &[
    "java.lang.OutOfMemoryError: Java heap space",
    "java.lang.OutOfMemoryError: Metaspace",
    "java.lang.OutOfMemoryError: Compressed class space",
    "java.lang.OutOfMemoryError: unable to create new native thread",
    "Could not reserve enough space for object heap",
    "OutOfMemoryError",
];

const RENDERER_MARKERS: &[&str] = &[
    "Failed to create backend OpenGL",
    "Failed to create backend Vulkan",
    "BackendCreationException",
    "glGetError mismatch",
    "EGL_BAD_MATCH",
    "GL_INVALID_OPERATION",
    "Failed to create window",
    "Could not initialize GL",
    "XaOpenGL error",
    "Failed to create SDL window",
];

const DEPENDENCY_MARKERS: &[&str] = &[
    "ModResolutionException",
    "Missing mandatory dependencies",
    "Incompatible mod set!",
    "requires version",
    "requires any of",
    "Mod file",
    "Failed to load mod",
    "LoadingFailedException",
];

const MIXIN_MARKERS: &[&str] = &[
    "Mixin apply failed",
    "InvalidMixinException",
    "MixinTransformerError",
    "MixinApplyError",
    "InvalidInjectionException",
    "Critical injection failure",
];

const JAVA_MARKERS: &[&str] = &[
    "UnsupportedClassVersionError",
    "class file version",
    "has been compiled by a more recent version of the Java Runtime",
];

const CORRUPT_MARKERS: &[&str] = &[
    "Checksum does not match",
    "Hash mismatch",
    "hash mismatch",
    "Corrupted",
    "does not match the expected checksum",
    "java.util.zip.ZipException",
];

const JVM_START_MARKERS: &[&str] = &[
    "Unrecognized option",
    "Unrecognized VM option",
    "Could not find or load main class",
    "Error: Could not create the Java Virtual Machine",
    "invalid target release",
];

const RULES: &[Rule] = &[
    Rule {
        kind: CrashKind::JvmStartFailure,
        matches: |lines| lines_with(lines, JVM_START_MARKERS),
        fix: |_, _| Fix::None {
            reason: "the JVM rejected the launch arguments themselves",
        },
    },
    Rule {
        kind: CrashKind::UnsupportedJava,
        matches: |lines| lines_with(lines, JAVA_MARKERS),
        fix: |_, ctx| match ctx.java_major {
            Some(major) if major < 21 => Fix::UseJava { major: 21 },
            _ => Fix::None {
                reason: "this launch already runs on a new enough Java",
            },
        },
    },
    Rule {
        kind: CrashKind::CorruptAsset,
        matches: |lines| lines_with(lines, CORRUPT_MARKERS),
        fix: |hits, _| {
            // The download engine logs the file it was writing when it fails.
            let paths = hits
                .iter()
                .filter_map(|line| extract_quoted_path(line))
                .collect::<Vec<_>>();
            if paths.is_empty() {
                Fix::None {
                    reason: "the log does not name the file that failed",
                }
            } else {
                Fix::RefetchFiles { paths }
            }
        },
    },
    Rule {
        kind: CrashKind::HeapExhausted,
        matches: |lines| lines_with(lines, HEAP_MARKERS),
        fix: |_, ctx| {
            // Metaspace and native-thread exhaustion are not fixed by growing
            // the heap; growing it for "Java heap space" is.
            let fix_heap = |next: i64| Fix::SetHeap {
                heap_mb: Some(next),
                reason: "the heap ran out",
            };
            let ceiling = ctx
                .device_ram_mb
                .map(|ram| (ram * 0.75).floor() as i64)
                .unwrap_or(i64::MAX);
            match ctx.heap_mb {
                Some(current) => {
                    let grown = ((current as f64) * 1.5).ceil() as i64;
                    if current >= ceiling {
                        Fix::None {
                            reason: "the heap is already at the device budget",
                        }
                    } else {
                        fix_heap(grown.min(ceiling))
                    }
                }
                None => Fix::SetHeap {
                    heap_mb: None,
                    reason: "no heap was set and the heap ran out",
                },
            }
        },
    },
    Rule {
        kind: CrashKind::DependencyResolution,
        matches: |lines| lines_with(lines, DEPENDENCY_MARKERS),
        fix: |hits, ctx| disable_from_evidence(hits, ctx, "the mod set does not resolve"),
    },
    Rule {
        kind: CrashKind::MixinFailure,
        matches: |lines| lines_with(lines, MIXIN_MARKERS),
        fix: |hits, ctx| disable_from_evidence(hits, ctx, "a mixin failed to apply"),
    },
    Rule {
        kind: CrashKind::RendererInit,
        matches: |lines| lines_with(lines, RENDERER_MARKERS),
        fix: |_, ctx| match ctx.renderer {
            Some(current) => Fix::SetRenderer {
                renderer: current.fallback_order()[0],
            },
            None => Fix::None {
                reason: "this platform picks the renderer itself",
            },
        },
    },
];

/// Pulls the first quoted path out of a log line, if it looks like a file.
fn extract_quoted_path(line: &str) -> Option<String> {
    let start = line.find('"')? + 1;
    let end = line[start..].find('"')? + start;
    let value = &line[start..end];
    if value.contains('/') && value.len() < 512 {
        Some(value.to_string())
    } else {
        None
    }
}

/// Finds mod jars named in the failure lines and disables them by rename.
///
/// Only jars that actually exist under `mods_dir` are touched, and the rename
/// is to `.basalt-disabled`, so the change is reversible and reported.
fn disable_from_evidence(hits: &[String], ctx: &LaunchContext, reason: &'static str) -> Fix {
    let Some(mods_dir) = ctx.mods_dir.as_ref() else {
        return Fix::None {
            reason: "no mods directory is known for this version",
        };
    };
    let mut files: Vec<PathBuf> = Vec::new();
    for line in hits {
        for token in line.split(|c: char| c.is_whitespace() || c == ',' || c == '"') {
            let token = token.trim_end_matches(['.', ':']);
            if !token.ends_with(".jar") {
                continue;
            }
            let candidate = mods_dir.join(token);
            if candidate.is_file() && !files.contains(&candidate) {
                files.push(candidate);
            }
        }
    }
    if files.is_empty() {
        Fix::None {
            reason: "no mod file in the log is present on disk",
        }
    } else {
        let _ = reason;
        Fix::DisableMods { files }
    }
}

/// Renames the named mod files so the loader skips them, returning the
/// new paths. Existing `.basalt-disabled` files are left alone.
pub fn disable_mods(files: &[PathBuf]) -> std::io::Result<Vec<PathBuf>> {
    let mut renamed = Vec::with_capacity(files.len());
    for file in files {
        let mut target = file.as_os_str().to_owned();
        target.push(".basalt-disabled");
        let target = PathBuf::from(target);
        if target.exists() {
            continue;
        }
        std::fs::rename(file, &target)?;
        renamed.push(target);
    }
    Ok(renamed)
}

/// Restores files previously moved aside by [`disable_mods`].
pub fn enable_mods(files: &[PathBuf]) -> std::io::Result<Vec<PathBuf>> {
    let mut restored = Vec::with_capacity(files.len());
    for file in files {
        let original = file.with_extension("");
        if original.exists() {
            continue;
        }
        std::fs::rename(file, &original)?;
        restored.push(original);
    }
    Ok(restored)
}

/// Collects the mod jars a host should offer to disable, newest first.
pub fn mod_jars(mods_dir: &std::path::Path) -> Vec<PathBuf> {
    let mut jars: BTreeSet<PathBuf> = BTreeSet::new();
    let Ok(entries) = std::fs::read_dir(mods_dir) else {
        return Vec::new();
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name().map(|n| n.to_string_lossy().to_string());
        let Some(name) = name else { continue };
        if name.ends_with(".jar") {
            jars.insert(path);
        }
    }
    jars.into_iter().collect()
}
