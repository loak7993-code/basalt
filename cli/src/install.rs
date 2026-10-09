//! `basalt install`: manifest -> client jar + libraries + assets, verified.

use crate::download::{fetch_json, run_all, FetchTask};
use crate::paths;
use basalt_engine::manifest::VersionManifest;
use basalt_engine::rules::{Features, RuleContext};
use std::fs;
use std::path::PathBuf;

const MANIFEST_URL: &str =
    "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";

#[derive(serde::Deserialize)]
struct VersionEntry {
    id: String,
    #[serde(rename = "type")]
    kind: String,
    url: String,
}

#[derive(serde::Deserialize)]
struct VersionList {
    latest: Latest,
    versions: Vec<VersionEntry>,
}

#[derive(serde::Deserialize)]
struct Latest {
    release: String,
}

struct AssetObject {
    hash: String,
    size: u64,
}

fn pick_version(explicit: Option<&str>) -> Result<(String, String), String> {
    let list: VersionList = fetch_json(MANIFEST_URL)?;
    match explicit {
        Some(id) => {
            let entry = list
                .versions
                .iter()
                .find(|v| v.id == id)
                .ok_or_else(|| format!("version {id} not found in the official manifest"))?;
            Ok((entry.id.clone(), entry.url.clone()))
        }
        None => {
            let entry = list
                .versions
                .iter()
                .find(|v| v.id == list.latest.release)
                .ok_or("latest release missing from the manifest")?;
            Ok((entry.id.clone(), entry.url.clone()))
        }
    }
}

fn asset_objects(index_url: &str) -> Result<Vec<(String, AssetObject)>, String> {
    #[derive(serde::Deserialize)]
    struct AssetIndex {
        objects: std::collections::BTreeMap<String, AssetObjectDef>,
    }
    #[derive(serde::Deserialize)]
    struct AssetObjectDef {
        hash: String,
        size: u64,
    }
    let index: AssetIndex = fetch_json(index_url)?;
    Ok(index
        .objects
        .into_iter()
        .map(|(name, def)| (name, AssetObject { hash: def.hash, size: def.size }))
        .collect())
}

/// Unpacks natives-linux jars into the natives directory.
fn extract_natives(manifest: &VersionManifest, context: &RuleContext) -> Result<(), String> {
    let natives_dir = paths::natives_dir();
    paths::ensure_dir(&natives_dir);
    for library in &manifest.libraries {
        if !context.is_allowed(library) {
            continue;
        }
        let Some(artifact) = library.downloads.as_ref().and_then(|d| d.artifact.as_ref()) else {
            continue;
        };
        let Some(path) = &artifact.path else { continue };
        if !path.contains("natives-linux") {
            continue;
        }
        let jar = paths::libraries_dir().join(path);
        let file = fs::File::open(&jar).map_err(|e| format!("open {path}: {e}"))?;
        let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("{path}: {e}"))?;
        for i in 0..archive.len() {
            let mut entry = archive.by_index(i).map_err(|e| format!("{path}#{i}: {e}"))?;
            let Some(name) = entry.enclosed_name().map(|p| p.to_path_buf()) else {
                continue;
            };
            // Skip directories, signatures, and meta files.
            if name.extension().map(|e| e != "so").unwrap_or(true) {
                continue;
            }
            let target = natives_dir.join(&name);
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("mkdir {}: {e}", parent.display()))?;
            }
            let mut out =
                fs::File::create(&target).map_err(|e| format!("create {}: {e}", target.display()))?;
            std::io::copy(&mut entry, &mut out)
                .map_err(|e| format!("copy {path}#{i}: {e}"))?;
        }
    }
    Ok(())
}

pub fn run(explicit: Option<&str>) -> i32 {
    let (id, version_url) = match pick_version(explicit) {
        Ok(found) => found,
        Err(err) => {
            eprintln!("error: {err}");
            return 1;
        }
    };
    println!("installing {} ...", id);

    let manifest: VersionManifest = match fetch_json(&version_url) {
        Ok(manifest) => manifest,
        Err(err) => {
            eprintln!("error: {err}");
            return 1;
        }
    };
    let context = RuleContext {
        platform: crate::accounts::platform(),
        features: Features::default(),
    };

    let version_dir = paths::version_dir(&id);
    paths::ensure_dir(&version_dir);
    paths::ensure_dir(&paths::libraries_dir());
    paths::ensure_dir(&paths::assets_dir());

    // Persist the manifest: play rebuilds the plan from this exact file, so a
    // launch uses the same data the install verified.
    let manifest_text = serde_json::to_string_pretty(&manifest).expect("manifest serializes");
    if let Err(err) = fs::write(version_dir.join(format!("{id}.json")), manifest_text) {
        eprintln!("error: write version json: {err}");
        return 1;
    }

    // 1. client jar
    let client = manifest.downloads.get("client");
    let mut tasks = Vec::new();
    if let Some(client) = client {
        tasks.push(FetchTask {
            urls: vec![client.url.clone()],
            target: version_dir.join(format!("{}.jar", id)),
            expected_sha1: client.sha1.clone(),
            expected_size: client.size,
        });
    }

    // 2. allowed libraries (linux rules) + every other library that is allowed
    //    for at least the launcher's platform set (linux x86_64/arm64)
    let platform = crate::accounts::platform();
    let all_platforms = [platform, basalt_engine::manifest::Platform::LinuxArm64];
    for library in &manifest.libraries {
        let Some(artifact) = library.downloads.as_ref().and_then(|d| d.artifact.as_ref()) else {
            continue;
        };
        let Some(path) = &artifact.path else { continue };
        // A library is fetched when it is allowed for the running platform.
        // (Cross-arch jars are skipped; natives-linux artifacts cover both.)
        if !all_platforms
            .iter()
            .any(|p| basalt_engine::rules::is_allowed_on(library, *p, &context.features))
        {
            continue;
        }
        let mut urls = Vec::new();
        if let Some(base) = &artifact.url {
            urls.push(base.clone());
        }
        if let Some(repo) = &library.url {
            urls.push(format!("{}{}", repo.trim_end_matches('/'), path));
        }
        if urls.is_empty() {
            continue;
        }
        tasks.push(FetchTask {
            urls,
            target: paths::libraries_dir().join(path),
            expected_sha1: artifact.sha1.clone(),
            expected_size: artifact.size,
        });
    }

    // 3. assets
    if let Some(asset_index) = &manifest.asset_index {
        tasks.push(FetchTask {
            urls: vec![asset_index.url.clone()],
            target: paths::assets_dir().join("indexes").join(format!("{}.json", asset_index.id)),
            expected_sha1: asset_index.sha1.clone(),
            expected_size: asset_index.total_size,
        });
    }

    println!("downloading {} files ...", tasks.len());
    let failures = run_all(tasks, 16, |done, total| {
        if done % 250 == 0 || done == total {
            println!("  {}/{}", done, total);
        }
    });
    if !failures.is_empty() {
        eprintln!("error: {} file(s) failed:", failures.len());
        for (path, err) in failures.iter().take(10) {
            eprintln!("  {}: {}", path.display(), err);
        }
        return 1;
    }

    // 4. asset objects (content-addressed CDN; fan out after the index)
    if let Some(asset_index) = &manifest.asset_index {
        let objects = match asset_objects(&asset_index.url) {
            Ok(objects) => objects,
            Err(err) => {
                eprintln!("error: asset index objects: {err}");
                return 1;
            }
        };
        println!("downloading {} asset objects ...", objects.len());
        let base = "https://resources.download.minecraft.net";
        let asset_tasks: Vec<FetchTask> = objects
            .into_iter()
            .map(|(name, obj)| FetchTask {
                urls: vec![format!("{}/{}/{}", base, &obj.hash[..2], obj.hash)],
                target: paths::assets_dir().join("objects").join(&obj.hash[..2]).join(&obj.hash),
                expected_sha1: Some(obj.hash.clone()),
                expected_size: obj.size,
            })
            .collect();
        let failures = run_all(asset_tasks, 24, |done, total| {
            if done % 500 == 0 || done == total {
                println!("  {}/{}", done, total);
            }
        });
        if !failures.is_empty() {
            eprintln!("error: {} asset object(s) failed:", failures.len());
            for (path, err) in failures.iter().take(10) {
                eprintln!("  {}: {}", path.display(), err);
            }
            return 1;
        }
    }

    // 5. natives
    if let Err(err) = extract_natives(&manifest, &context) {
        eprintln!("error: natives: {err}");
        return 1;
    }

    println!("installed: {}", version_dir.display());
    0
}

pub fn list() -> i32 {
    let dir = paths::versions_dir();
    let Ok(entries) = fs::read_dir(&dir) else {
        println!("no versions installed ({})", dir.display());
        return 0;
    };
    let mut found = false;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if entry.path().join(format!("{name}.jar")).exists() {
            println!("{}", name);
            found = true;
        }
    }
    if !found {
        println!("no versions installed ({})", dir.display());
    }
    0
}

/// The asset index id the engine layout needs.
pub fn asset_index_id_for(id: &str) -> Option<String> {
    let path: PathBuf = paths::version_dir(id).join(format!("{id}.json"));
    let raw = fs::read_to_string(path).ok()?;
    let manifest: VersionManifest = serde_json::from_str(&raw).ok()?;
    Some(manifest.asset_index?.id)
}
