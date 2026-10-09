//! HTTP fetch + parallel download with SHA1 verification.

use serde::Deserialize;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex};

pub fn fetch_text(url: &str) -> Result<String, String> {
    let response = ureq::get(url)
        .set("User-Agent", agent())
        .call()
        .map_err(|e| format!("{url}: {e}"))?;
    let mut body = String::new();
    response
        .into_reader()
        .read_to_string(&mut body)
        .map_err(|e| e.to_string())?;
    Ok(body)
}

pub fn fetch_json<T: for<'de> Deserialize<'de>>(url: &str) -> Result<T, String> {
    let text = fetch_text(url)?;
    serde_json::from_str(&text).map_err(|e| format!("parse {url}: {e}"))
}

fn agent() -> &'static str {
    concat!("Basalt/", env!("CARGO_PKG_VERSION"), " (Linux)")
}

/// One file to fetch: remote candidates (tried in order) and the target path.
pub struct FetchTask {
    pub urls: Vec<String>,
    pub target: PathBuf,
    pub expected_sha1: Option<String>,
    pub expected_size: u64,
}

impl FetchTask {
    /// True when the target already exists with the expected size (and hash
    /// when declared): lets reinstall runs skip finished work.
    pub fn already_satisfied(&self) -> bool {
        let Ok(meta) = fs::metadata(&self.target) else {
            return false;
        };
        if self.expected_size > 0 && meta.len() != self.expected_size {
            return false;
        }
        match &self.expected_sha1 {
            Some(sha1) => file_sha1(&self.target)
                .map(|got| got.eq_ignore_ascii_case(sha1))
                .unwrap_or(false),
            None => true,
        }
    }
}

fn file_sha1(path: &Path) -> Option<String> {
    use sha1_smol::Sha1;
    let mut file = fs::File::open(path).ok()?;
    let mut hasher = Sha1::new();
    let mut buffer = [0u8; 65536];
    loop {
        let read = file.read(&mut buffer).ok()?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Some(hasher.digest().to_string())
}

fn download_one(url: &str, target: &Path, expected_sha1: Option<&str>) -> Result<(), String> {
    let response = ureq::get(url)
        .set("User-Agent", agent())
        .call()
        .map_err(|e| format!("{url}: {e}"))?;
    if response.status() != 200 {
        return Err(format!("{url}: HTTP {}", response.status()));
    }
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let tmp = target.with_extension("basalt-part");
    let mut file = fs::File::create(&tmp).map_err(|e| e.to_string())?;
    let mut reader = response.into_reader();
    std::io::copy(&mut reader, &mut file).map_err(|e| e.to_string())?;
    drop(file);

    if let Some(sha1) = expected_sha1 {
        let got = file_sha1(&tmp).ok_or_else(|| "hash read failed".to_string())?;
        if !got.eq_ignore_ascii_case(sha1) {
            let _ = fs::remove_file(&tmp);
            return Err(format!("{url}: sha1 mismatch"));
        }
    }
    fs::rename(&tmp, target).map_err(|e| e.to_string())?;
    Ok(())
}

/// Runs every task with a bounded worker pool; returns the failures.
pub fn run_all(
    tasks: Vec<FetchTask>,
    workers: usize,
    progress: impl Fn(usize, usize) + Send + Sync + 'static,
) -> Vec<(PathBuf, String)> {
    let total = tasks.len();
    let done = Arc::new(AtomicUsize::new(0));
    let progress = Arc::new(progress);
    let (tx, rx) = mpsc::channel::<(PathBuf, String)>();
    let queue = Arc::new(Mutex::new(tasks));

    std::thread::scope(|scope| {
        for _ in 0..workers {
            let queue = Arc::clone(&queue);
            let done = Arc::clone(&done);
            let progress = Arc::clone(&progress);
            let tx = tx.clone();
            scope.spawn(move || loop {
                let task = {
                    let mut guard = queue.lock().unwrap();
                    guard.pop()
                };
                let Some(task) = task else { break };
                if task.already_satisfied() {
                    let n = done.fetch_add(1, Ordering::SeqCst) + 1;
                    progress(n, total);
                    continue;
                }
                let mut failure: Option<String> = None;
                for url in &task.urls {
                    match download_one(url, &task.target, task.expected_sha1.as_deref()) {
                        Ok(()) => {
                            failure = None;
                            break;
                        }
                        Err(err) => failure = Some(err),
                    }
                }
                if let Some(err) = failure {
                    let _ = tx.send((task.target.clone(), err));
                }
                let n = done.fetch_add(1, Ordering::SeqCst) + 1;
                progress(n, total);
            });
        }
        drop(tx);
    });

    rx.iter().collect()
}
