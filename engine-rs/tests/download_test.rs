//! Network tests for the Rust download engine, against real Mojang endpoints.

use basalt_engine::download::{download_progress, download_start, download_wait, DownloadSpec};

/// The 26.3 logging config file: tiny (1073 bytes) with a known SHA1 from the
/// real manifest — ideal for verifying the whole pipeline against production.
const LOG_URL: &str = "https://piston-data.mojang.com/v1/objects/39384bd14c0606d812afec88d8aff595b2587dd9/client-1.21.2.xml";
const LOG_SHA1: &str = "39384bd14c0606d812afec88d8aff595b2587dd9";

#[test]
fn downloads_and_verifies_a_real_file() {
    let target = std::env::temp_dir().join("basalt-dl-test/client-1.21.2.xml");
    let _ = std::fs::remove_file(&target);
    std::fs::create_dir_all(target.parent().unwrap()).unwrap();

    let specs = vec![DownloadSpec {
        urls: vec![LOG_URL.into()],
        target: target.to_string_lossy().into(),
        sha1: Some(LOG_SHA1.into()),
        size: 1073,
    }];
    let id = download_start(specs, 1);
    loop {
        let snap = download_progress(id).unwrap();
        if snap.finished {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    let failures = download_wait(id).unwrap();
    assert!(failures.is_empty(), "failures: {:?}", failures);
    let content = std::fs::read_to_string(&target).unwrap();
    assert!(content.contains("<Configuration"), "not a log4j config");
}

#[test]
fn sha1_mismatch_is_a_failure() {
    let target = std::env::temp_dir().join("basalt-dl-test/bad-sha.xml");
    let _ = std::fs::remove_file(&target);
    let specs = vec![DownloadSpec {
        urls: vec![LOG_URL.into()],
        target: target.to_string_lossy().into(),
        sha1: Some("0000000000000000000000000000000000000000".into()),
        size: 0,
    }];
    let id = download_start(specs, 1);
    let failures = download_wait(id).unwrap();
    assert_eq!(failures.len(), 1);
    assert!(failures[0].error.contains("sha1 mismatch"));
    assert!(!target.exists(), "mismatched file must not be kept");
}

#[test]
fn retry_rotates_candidates_and_succeeds() {
    let target = std::env::temp_dir().join("basalt-dl-test/retry-rot.xml");
    let _ = std::fs::remove_file(&target);
    // First candidate is a guaranteed-failing URL; the retry rotation lands on
    // the real one by the second attempt.
    let specs = vec![DownloadSpec {
        urls: vec![
            "https://piston-data.mojang.com/v1/objects/deadbeef/nonexistent.xml".into(),
            LOG_URL.into(),
        ],
        target: target.to_string_lossy().into(),
        sha1: Some(LOG_SHA1.into()),
        size: 0,
    }];
    let id = download_start(specs, 1);
    let failures = download_wait(id).unwrap();
    assert!(
        failures.is_empty(),
        "rotation+retry should recover: {:?}",
        failures
    );
    assert!(target.exists());
}
