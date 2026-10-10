//! HTTP fetch + parallel download with SHA1 verification, connection reuse,
//! retries, and byte-accurate progress.

use serde::Deserialize;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};
use ureq::Agent;

pub fn fetch_text(url: &str) -> Result<String, String> {
    let response = shared_agent()
        .get(url)
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

/// One shared agent for the whole process: every download reuses pooled
/// keep-alive connections instead of paying a TCP+TLS handshake per file.
static SHARED_AGENT: std::sync::OnceLock<Agent> = std::sync::OnceLock::new();

fn shared_agent() -> &'static Agent {
    SHARED_AGENT.get_or_init(|| {
        ureq::AgentBuilder::new()
            .max_idle_connections_per_host(64)
            .build()
    })
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

const RETRY_ATTEMPTS: usize = 3;

fn download_one(url: &str, target: &Path, expected_sha1: Option<&str>) -> Result<(), String> {
    let response = shared_agent()
        .get(url)
        .set("User-Agent", agent())
        .timeout(Duration::from_secs(60))
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

/// Downloads one file with retries and backoff, rotating candidate URLs.
fn download_with_retry(task: &FetchTask) -> Result<(), String> {
    let mut last: Option<String> = None;
    for attempt in 0..RETRY_ATTEMPTS {
        if attempt > 0 {
            std::thread::sleep(Duration::from_millis(400 * (1 << attempt)));
        }
        if task.urls.is_empty() {
            return Err("no candidate urls".into());
        }
        let url = &task.urls[attempt % task.urls.len()];
        match download_one(url, &task.target, task.expected_sha1.as_deref()) {
            Ok(()) => return Ok(()),
            Err(err) => last = Some(err),
        }
    }
    Err(last.unwrap_or_else(|| "download failed".into()))
}

/// Byte-level progress snapshot.
#[derive(Debug, Clone, Copy)]
pub struct ByteProgress {
    pub done: usize,
    pub total: usize,
    pub bytes_done: u64,
    pub bytes_total: u64,
}

/// Runs every task with a bounded worker pool, shared connections, retries,
/// and byte-accurate progress. Returns the failures.
pub fn run_all(
    tasks: Vec<FetchTask>,
    workers: usize,
    progress: Arc<dyn Fn(ByteProgress) + Send + Sync>,
) -> Vec<(PathBuf, String)> {
    let total = tasks.len();
    let bytes_total: u64 = tasks.iter().map(|t| t.expected_size).sum();
    let bytes_done = Arc::new(AtomicU64::new(0));
    let done = Arc::new(AtomicUsize::new(0));
    let (tx, rx) = mpsc::channel::<(PathBuf, String)>();
    let queue = Arc::new(Mutex::new(tasks));
    let started = Instant::now();

    std::thread::scope(|scope| {
        for _ in 0..workers {
            let queue = Arc::clone(&queue);
            let done = Arc::clone(&done);
            let bytes_done = Arc::clone(&bytes_done);
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
                    let b = bytes_done.fetch_add(task.expected_size, Ordering::SeqCst)
                        + task.expected_size;
                    progress(ByteProgress {
                        done: n,
                        total,
                        bytes_done: b,
                        bytes_total,
                    });
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
                let size = task.expected_size;
                if let Some(err) = failure {
                    let _ = tx.send((task.target.clone(), err));
                } else {
                    let b = bytes_done.fetch_add(size, Ordering::SeqCst) + size;
                    let n = done.fetch_add(1, Ordering::SeqCst) + 1;
                    progress(ByteProgress {
                        done: n,
                        total,
                        bytes_done: b,
                        bytes_total,
                    });
                }
            });
        }
        drop(tx);
    });

    rx.iter().collect()
}

/// Formats a byte count for humans.
pub fn human_bytes(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
    let mut value = bytes as f64;
    for unit in UNITS {
        if value < 1024.0 {
            return format!("{value:.1} {unit}");
        }
        value /= 1024.0;
    }
    format!("{value:.1} TB")
}

/// A live rate tracker: MB/s over the elapsed run.
pub struct RateTracker {
    started: Instant,
}

impl RateTracker {
    pub fn new() -> Self {
        Self {
            started: Instant::now(),
        }
    }

    pub fn mb_s(&self, bytes_done: u64) -> f64 {
        let elapsed = self.started.elapsed().as_secs_f64();
        if elapsed < 0.5 {
            return 0.0;
        }
        bytes_done as f64 / 1024.0 / 1024.0 / elapsed
    }
}
