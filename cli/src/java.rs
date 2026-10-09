//! Java runtime discovery: find a `java` at least as new as the version needs.

use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct JavaRuntime {
    pub java_bin: PathBuf,
    pub major: i64,
}

fn major_from_version_output(output: &str) -> Option<i64> {
    // openjdk version "25.0.4.1" ... / openjdk 25 ...
    for token in output.split_whitespace() {
        if let Some(rest) = token.strip_prefix('"') {
            let version = rest.trim_end_matches('"');
            let major = version.split('.').next()?.parse::<i64>().ok()?;
            return Some(if version.starts_with("1.") { major_of_legacy(version) } else { major });
        }
    }
    // OpenJDK 25 format without quotes
    let number: String = output
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    let first = number.split('.').next()?.parse::<i64>().ok()?;
    Some(first)
}

fn major_of_legacy(version: &str) -> i64 {
    version
        .trim_start_matches("1.")
        .split('.')
        .next()
        .and_then(|n| n.parse().ok())
        .unwrap_or(8)
}

fn probe(java_bin: &PathBuf) -> Option<i64> {
    let output = std::process::Command::new(java_bin)
        .arg("-version")
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&output.stderr);
    major_from_version_output(&text)
}

/// Finds a suitable runtime: JAVA_HOME first, then PATH, then common install
/// roots. The first `java` meeting the required major version wins.
pub fn find(required_major: i64) -> Option<JavaRuntime> {
    let mut candidates: Vec<PathBuf> = Vec::new();

    if let Ok(home) = std::env::var("JAVA_HOME") {
        candidates.push(PathBuf::from(home).join("bin/java"));
    }
    if let Ok(path_var) = std::env::var("PATH") {
        for dir in path_var.split(':') {
            candidates.push(PathBuf::from(dir).join("java"));
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        let roots = [
            format!("{home}/.local/share/basalt/runtimes"),
            "/usr/lib/jvm".to_string(),
        ];
        for root in roots {
            if let Ok(entries) = std::fs::read_dir(&root) {
                for entry in entries.flatten() {
                    let bin = entry.path().join("bin/java");
                    if bin.exists() {
                        candidates.push(bin);
                    }
                    if let Ok(subs) = std::fs::read_dir(entry.path()) {
                        for sub in subs.flatten() {
                            let bin = sub.path().join("bin/java");
                            if bin.exists() {
                                candidates.push(bin);
                            }
                        }
                    }
                }
            }
        }
    }

    for candidate in candidates {
        if !candidate.exists() {
            continue;
        }
        if let Some(major) = probe(&candidate) {
            if major >= required_major {
                return Some(JavaRuntime {
                    java_bin: candidate,
                    major,
                });
            }
        }
    }
    None
}
