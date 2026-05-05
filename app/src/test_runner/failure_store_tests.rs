use std::path::{Path, PathBuf};

use super::failure_store::{FailureRecord, FailureStore};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "warpx-failure-store-tests-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time after epoch")
            .subsec_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("create tmp dir");
    dir
}

fn make_record() -> FailureRecord {
    FailureRecord {
        test_names: vec!["foo::bar".to_string(), "baz::qux".to_string()],
        rerun_command: "cargo nextest run --test-name foo::bar".to_string(),
        timestamp_secs: 1_700_000_000,
    }
}

// ---------------------------------------------------------------------------
// Round-trip serialization
// ---------------------------------------------------------------------------

#[test]
fn test_round_trip_serialization() {
    let dir = tmp_dir();
    let store = FailureStore::with_dir(dir.clone());
    let repo_path = Path::new("/Users/joe/dev/warpx");

    let record = make_record();
    store.save(repo_path, &record).expect("save should succeed");

    let loaded = store
        .load(repo_path)
        .expect("load should succeed")
        .expect("record should exist");

    assert_eq!(loaded.test_names, record.test_names);
    assert_eq!(loaded.rerun_command, record.rerun_command);
    assert_eq!(loaded.timestamp_secs, record.timestamp_secs);
}

// ---------------------------------------------------------------------------
// Clear on all-pass
// ---------------------------------------------------------------------------

#[test]
fn test_all_pass_clears_file() {
    let dir = tmp_dir();
    let store = FailureStore::with_dir(dir.clone());
    let repo_path = Path::new("/Users/joe/dev/warpx");

    // Save a record first.
    store
        .save(repo_path, &make_record())
        .expect("save should succeed");
    assert!(store.load(repo_path).expect("load ok").is_some());

    // Clearing should remove it.
    store.clear(repo_path).expect("clear should succeed");
    assert!(
        store
            .load(repo_path)
            .expect("load after clear ok")
            .is_none(),
        "record should be gone after clear"
    );
}

// ---------------------------------------------------------------------------
// Repo hash stability
// ---------------------------------------------------------------------------

#[test]
fn test_repo_hash_is_stable() {
    let dir = tmp_dir();
    let store = FailureStore::with_dir(dir.clone());

    let path = Path::new("/Users/joe/dev/warpx");
    let h1 = store.repo_key(path);
    let h2 = store.repo_key(path);
    assert_eq!(h1, h2, "hash must be deterministic");
}

#[test]
fn test_different_repos_have_different_keys() {
    let dir = tmp_dir();
    let store = FailureStore::with_dir(dir.clone());

    let a = store.repo_key(Path::new("/Users/joe/dev/warpx"));
    let b = store.repo_key(Path::new("/Users/joe/dev/minibox"));
    assert_ne!(a, b, "different repos must produce different keys");
}

// ---------------------------------------------------------------------------
// Missing file returns None (not an error)
// ---------------------------------------------------------------------------

#[test]
fn test_load_nonexistent_returns_none() {
    let dir = tmp_dir();
    let store = FailureStore::with_dir(dir);
    let result = store
        .load(Path::new("/no/such/repo"))
        .expect("should not error on missing file");
    assert!(result.is_none());
}
