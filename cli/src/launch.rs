//! `basalt play`: build the plan with the Rust engine and exec the JVM.

use crate::{accounts, java, paths};
use basalt_engine::exec::{ChildProcessExec, ExecStrategy};
use basalt_engine::manifest::VersionManifest;
use basalt_engine::plan::{AuthContext, EngineOptions, InstallLayout, MemoryOptions, PlanBuilder};
use std::fs;
use std::path::PathBuf;

pub fn run(
    version: Option<&str>,
    name: Option<&str>,
    server: Option<&str>,
    jvm_args: &[String],
    ram: Option<i64>,
) -> i32 {
    let id: String = match version {
        Some(id) => id.to_string(),
        None => match latest_installed() {
            Some(id) => id,
            None => {
                eprintln!("error: no versions installed; run `basalt install` first");
                return 1;
            }
        },
    };
    let version_json = paths::version_dir(&id).join(format!("{id}.json"));
    let client_jar = paths::version_dir(&id).join(format!("{id}.jar"));
    if !version_json.exists() || !client_jar.exists() {
        eprintln!("error: version {} is not installed", id);
        return 1;
    }

    let manifest: VersionManifest = match fs::read_to_string(&version_json)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
    {
        Some(manifest) => manifest,
        None => {
            eprintln!("error: failed to parse {}", version_json.display());
            return 1;
        }
    };
    let Some(account) = accounts::for_name(name) else {
        eprintln!("error: no account; run `basalt account <name>` or pass --name");
        return 1;
    };

    let required_major = manifest
        .java_version
        .as_ref()
        .map(|j| j.major_version)
        .unwrap_or(8);
    let Some(runtime) = java::find(required_major) else {
        eprintln!(
            "error: no Java {}+ found; install it (e.g. `sudo pacman -S jre25-openjdk`) or set JAVA_HOME",
            required_major
        );
        return 1;
    };
    println!("java: {} (major {})", runtime.java_bin.display(), runtime.major);

    let asset_index_id = manifest
        .asset_index
        .as_ref()
        .map(|a| a.id.clone())
        .unwrap_or_else(|| "legacy".into());
    let layout = InstallLayout {
        client_jar: client_jar,
        libraries_dir: paths::libraries_dir(),
        natives_dir: paths::natives_dir(),
        game_dir: paths::version_dir(&id).join(".minecraft"),
        assets_dir: paths::assets_dir(),
        asset_index_id,
        runtime_home: runtime
            .java_bin
            .parent()
            .and_then(|p| p.parent())
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("/")),
    };

    let heap_mb = ram.unwrap_or_else(|| {
        let total: f64 = total_ram_mb();
        let is_32 = cfg!(target_pointer_width = "32");
        basalt_engine::policy::default_ram_allocation(total, is_32)
    });
    let options = EngineOptions {
        memory: Some(MemoryOptions {
            heap_mb,
            min_mb: 512,
            max_mb: 8192,
        }),
        gc: Some("tunedG1".into()),
        extra_jvm_args: jvm_args.to_vec(),
        extra_game_args: vec![],
        active_processors: Some(
            std::thread::available_parallelism()
                .map(|n| n.get() as i64)
                .unwrap_or(4),
        ),
        strip_always_pre_touch: true,
    };
    let quick_play = server.map(|address| basalt_engine::plan::QuickPlay::Server {
        address: address.to_string(),
        port: None,
    });

    let mut plan = PlanBuilder {
        manifest: &manifest,
        platform: accounts::platform(),
        layout: &layout,
        features: basalt_engine::rules::Features::default(),
        options,
        auth: AuthContext {
            player_name: account.player_name.clone(),
            uuid: account.uuid.clone(),
            ..AuthContext::default()
        },
        version_name: id.clone(),
        version_type: manifest.r#type.clone().unwrap_or_else(|| "release".into()),
        quick_play,
        log_config_path: None,
    }
    .build();

    // Desktop launches the game directly: no Android wrapper class.
    plan.wrapper_class = None;

    fs::create_dir_all(&layout.game_dir).ok();

    println!(
        "launching {} with {} ({}M heap, {} args)",
        id,
        account.player_name,
        heap_mb,
        plan.jvm_args.len()
    );

    let exec = ChildProcessExec;
    exec.exec(&plan, &runtime.java_bin.to_string_lossy(), &mut |line| {
        println!("{}", line);
    })
}

fn latest_installed() -> Option<String> {
    let mut best: Option<(String, String)> = None;
    let entries = fs::read_dir(paths::versions_dir()).ok()?;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if entry.path().join(format!("{name}.jar")).exists() {
            let sort_key = version_sort_key(&name);
            if best.as_ref().map(|b| sort_key > b.1).unwrap_or(true) {
                best = Some((name.clone(), sort_key));
            }
        }
    }
    best.map(|b| b.0)
}

/// Sorts 26.3 above 26.2 above 1.21.11: split into dot-separated numeric
/// chunks and compare lexicographically with zero padding.
fn version_sort_key(version: &str) -> String {
    version
        .replace(['-', '_'], ".")
        .split('.')
        .map(|part| {
            let digits: String = part.chars().filter(|c| c.is_ascii_digit()).collect();
            format!("{:0>6}", if digits.is_empty() { "0" } else { &digits })
        })
        .collect::<Vec<_>>()
        .join(".")
}

fn total_ram_mb() -> f64 {
    let Ok(meminfo) = fs::read_to_string("/proc/meminfo") else {
        return 4096.0;
    };
    for line in meminfo.lines() {
        if let Some(rest) = line.strip_prefix("MemTotal:") {
            let kb: f64 = rest
                .trim()
                .trim_end_matches(" kB")
                .trim()
                .parse()
                .unwrap_or(4096.0 * 1024.0);
            return kb / 1024.0;
        }
    }
    4096.0
}
