//! The Rust download engine: shared agent with keep-alive pooling, bounded
//! worker threads, retries, timeouts, SHA1 verification, and byte-accurate
//! progress. Job-based so hosts can poll without JNI callbacks.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;
use ureq::Agent;

fn agent() -> &'static Agent {
    static AGENT: OnceLock<Agent> = OnceLock::new();
    AGENT.get_or_init(|| {
        ureq::AgentBuilder::new()
            .max_idle_connections_per_host(64)
            .build()
    })
}

#[derive(Debug, Clone, Deserialize)]
pub struct DownloadSpec {
    pub urls: Vec<String>,
    pub target: String,
    #[serde(default)]
    pub sha1: Option<String>,
    #[serde(default)]
    pub size: u64,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressSnapshot {
    pub done: usize,
    pub total: usize,
    pub bytes_done: u64,
    pub bytes_total: u64,
    pub finished: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadFailure {
    pub path: String,
    pub error: String,
}

const RETRY_ATTEMPTS: usize = 3;
const REQUEST_TIMEOUT_SECS: u64 = 60;

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

fn already_satisfied(spec: &DownloadSpec) -> bool {
    let target = Path::new(&spec.target);
    let Ok(meta) = fs::metadata(target) else {
        return false;
    };
    if spec.size > 0 && meta.len() != spec.size {
        return false;
    }
    match &spec.sha1 {
        Some(sha1) => file_sha1(target)
            .map(|got| got.eq_ignore_ascii_case(sha1))
            .unwrap_or(false),
        None => true,
    }
}

fn download_one(url: &str, target: &str, expected_sha1: Option<&str>) -> Result<(), String> {
    let response = agent()
        .get(url)
        .set("User-Agent", "Basalt")
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .call()
        .map_err(|e| format!("{url}: {e}"))?;
    if response.status() != 200 {
        return Err(format!("{url}: HTTP {}", response.status()));
    }
    let target_path = Path::new(target);
    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let tmp = target_path.with_extension("basalt-part");
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
    fs::rename(&tmp, target_path).map_err(|e| e.to_string())?;
    Ok(())
}

fn download_with_retry(spec: &DownloadSpec) -> Result<(), String> {
    if spec.urls.is_empty() {
        return Err("no candidate urls".into());
    }
    let mut last: Option<String> = None;
    for attempt in 0..RETRY_ATTEMPTS {
        if attempt > 0 {
            std::thread::sleep(Duration::from_millis(400 * (1 << attempt)));
        }
        let url = &spec.urls[attempt % spec.urls.len()];
        match download_one(url, &spec.target, spec.sha1.as_deref()) {
            Ok(()) => return Ok(()),
            Err(err) => last = Some(err),
        }
    }
    Err(last.unwrap_or_else(|| "download failed".into()))
}

struct Job {
    progress: Mutex<ProgressSnapshot>,
    failures: Mutex<Vec<DownloadFailure>>,
    finished: AtomicBool,
    worker: Mutex<Option<std::thread::JoinHandle<()>>>,
}

static JOBS: OnceLock<Mutex<HashMap<u32, Arc<Job>>>> = OnceLock::new();
static NEXT_JOB: AtomicU64 = AtomicU64::new(1);

fn jobs() -> &'static Mutex<HashMap<u32, Arc<Job>>> {
    JOBS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Starts a download job: spawns the worker threads and returns the job id.
pub fn download_start(specs: Vec<DownloadSpec>, workers: usize) -> u32 {
    let id = NEXT_JOB.fetch_add(1, Ordering::SeqCst) as u32;
    let bytes_total: u64 = specs.iter().map(|s| s.size).sum();
    let job = Arc::new(Job {
        progress: Mutex::new(ProgressSnapshot {
            done: 0,
            total: specs.len(),
            bytes_done: 0,
            bytes_total,
            finished: false,
        }),
        failures: Mutex::new(Vec::new()),
        finished: AtomicBool::new(false),
        worker: Mutex::new(None),
    });

    let queue = Arc::new(Mutex::new(specs));
    let job_for_workers = Arc::clone(&job);
    let job_main = Arc::clone(&job);
    let handle = std::thread::spawn(move || {
        let done = AtomicUsize::new(0);
        let bytes_done = AtomicU64::new(0);
        let failures: Mutex<Vec<DownloadFailure>> = Mutex::new(Vec::new());

        std::thread::scope(|scope| {
            let mut guards = Vec::new();
            for _ in 0..workers.max(1) {
                let queue = Arc::clone(&queue);
                let done = &done;
                let bytes_done = &bytes_done;
                let failures = &failures;
                let job = Arc::clone(&job_for_workers);
                guards.push(scope.spawn(move || loop {
                    let spec = {
                        let mut guard = queue.lock().unwrap();
                        guard.pop()
                    };
                    let Some(spec) = spec else { break };

                    if already_satisfied(&spec) {
                        let b = bytes_done.fetch_add(spec.size, Ordering::SeqCst) + spec.size;
                        let d = done.fetch_add(1, Ordering::SeqCst) + 1;
                        let mut snap = job.progress.lock().unwrap();
                        snap.done = d;
                        snap.bytes_done = b;
                        continue;
                    }

                    match download_with_retry(&spec) {
                        Ok(()) => {
                            let b = bytes_done.fetch_add(spec.size, Ordering::SeqCst) + spec.size;
                            let d = done.fetch_add(1, Ordering::SeqCst) + 1;
                            let mut snap = job.progress.lock().unwrap();
                            snap.done = d;
                            snap.bytes_done = b;
                        }
                        Err(error) => {
                            failures.lock().unwrap().push(DownloadFailure {
                                path: spec.target.clone(),
                                error,
                            });
                            let d = done.fetch_add(1, Ordering::SeqCst) + 1;
                            let mut snap = job.progress.lock().unwrap();
                            snap.done = d;
                        }
                    }
                }));
            }
            for guard in guards {
                let _ = guard.join();
            }
        });

        let failure_list = failures.into_inner().unwrap();
        {
            let mut snap = job.progress.lock().unwrap();
            snap.finished = true;
        }
        *job.failures.lock().unwrap() = failure_list;
        job.finished.store(true, Ordering::SeqCst);
    });

    job_main.worker.lock().unwrap().replace(handle);
    jobs().lock().unwrap().insert(id, job_main);
    id
}

/// Snapshot for a running or finished job; None for unknown ids.
pub fn download_progress(id: u32) -> Option<ProgressSnapshot> {
    let jobs = jobs().lock().unwrap();
    jobs.get(&id)
        .map(|job| job.progress.lock().unwrap().clone())
}

/// Blocks until the job finishes and returns its failures.
pub fn download_wait(id: u32) -> Option<Vec<DownloadFailure>> {
    let job = {
        let mut jobs = jobs().lock().unwrap();
        jobs.remove(&id)?
    };
    if let Some(handle) = job.worker.lock().unwrap().take() {
        let _ = handle.join();
    }
    let failures = job.failures.lock().unwrap().clone();
    Some(failures)
}
