#![allow(dead_code)]
//! Persist failing test names across sessions.
//!
//! Each repo is identified by a SHA-256 hash of its canonical path.
//! Records are stored as JSON in `~/.warp/last-failures/<repo-hash>.json`.
//! Gated behind `FeatureFlag::OzFailurePersistence`.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FailureRecord {
    pub test_names: Vec<String>,
    pub rerun_command: String,
    pub timestamp_secs: u64,
}

// ---------------------------------------------------------------------------
// Store
// ---------------------------------------------------------------------------

pub struct FailureStore {
    dir: PathBuf,
}

impl FailureStore {
    /// Use `~/.warp/last-failures/` as the backing directory.
    pub fn default_dir() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join(".warp")
            .join("last-failures")
    }

    /// Production constructor — uses `~/.warp/last-failures/`.
    pub fn new() -> Self {
        Self {
            dir: Self::default_dir(),
        }
    }

    /// Test constructor — uses an arbitrary directory.
    pub fn with_dir(dir: PathBuf) -> Self {
        Self { dir }
    }

    /// Stable hex key for a repo path (first 16 bytes of SHA-256).
    pub fn repo_key(&self, repo_path: &Path) -> String {
        use sha2::{Digest, Sha256};
        let canonical = repo_path
            .canonicalize()
            .unwrap_or_else(|_| repo_path.to_path_buf());
        let hash = Sha256::digest(canonical.to_string_lossy().as_bytes());
        hex::encode(&hash[..16])
    }

    fn record_path(&self, repo_path: &Path) -> PathBuf {
        self.dir.join(format!("{}.json", self.repo_key(repo_path)))
    }

    /// Persist a failure record for `repo_path`.
    pub fn save(&self, repo_path: &Path, record: &FailureRecord) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.dir)?;
        let json = serde_json::to_string_pretty(record)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(self.record_path(repo_path), json)
    }

    /// Load an existing failure record, or `None` if no file exists.
    pub fn load(&self, repo_path: &Path) -> std::io::Result<Option<FailureRecord>> {
        let path = self.record_path(repo_path);
        match std::fs::read_to_string(&path) {
            Ok(contents) => {
                let record = serde_json::from_str(&contents)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
                Ok(Some(record))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Remove the failure record for `repo_path` (called when all tests pass).
    pub fn clear(&self, repo_path: &Path) -> std::io::Result<()> {
        let path = self.record_path(repo_path);
        match std::fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e),
        }
    }
}

impl Default for FailureStore {
    fn default() -> Self {
        Self::new()
    }
}
