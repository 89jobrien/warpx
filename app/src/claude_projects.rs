/// Utilities for reading `~/.claude/projects/` — the Claude Code per-project
/// memory directory. Each entry is a directory whose name encodes the absolute
/// project path by replacing `/` with `-` (leading `/` becomes the first `-`).
///
/// Example: `-Users-joe-dev-warpx` → `/Users/joe/dev/warpx` → project name `warpx`
use std::path::PathBuf;

/// Decode a `~/.claude/projects/` directory entry name into the absolute path
/// it represents.
///
/// Encoding rules (observed from Claude Code):
/// - Each `/` in the path becomes `-`
/// - A path component starting with `.` is encoded as an extra leading `-`,
///   so `/.claude` → `--claude` (the dot is dropped, replaced by double dash)
///
/// Decoding: replace `--` with `/.` first, then replace remaining `-` with `/`.
pub(crate) fn decode_project_path(entry: &str) -> PathBuf {
    let with_hidden = entry.replace("--", "/.");
    let with_slashes = with_hidden.replace('-', "/");
    PathBuf::from(with_slashes)
}

/// Extract a short human-readable project name from a `~/.claude/projects/`
/// entry name. Returns the last non-empty path segment of the decoded path.
///
/// `-Users-joe-dev-warpx` → `warpx`
/// `-Users-joe--claude`   → `.claude`  (double-dash = hidden dir)
pub(crate) fn project_name_from_entry(entry: &str) -> String {
    // Double `-` in the entry encodes a path component that starts with `.`
    // e.g. `-Users-joe--claude` → `/Users/joe/.claude`
    // We handle this by splitting on `-` carefully.
    let decoded = decode_project_path(entry);
    decoded
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(entry)
        .to_string()
}

/// Returns the `~/.claude/projects/` directory path.
pub(crate) fn claude_projects_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(home).join(".claude").join("projects")
}

/// Reads `~/.claude/projects/` and returns an ordered list of `(entry_name,
/// project_name, decoded_path)` tuples in filesystem iteration order.
///
/// Returns an empty vec if the directory cannot be read.
pub(crate) fn read_project_entries() -> Vec<ProjectEntry> {
    let dir = claude_projects_dir();
    let read_dir = match std::fs::read_dir(&dir) {
        Ok(r) => r,
        Err(e) => {
            log::warn!("[claude_projects] cannot read {dir:?}: {e}");
            return Vec::new();
        }
    };

    let mut entries: Vec<ProjectEntry> = read_dir
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().into_string().ok()?;
            // Skip hidden files / .DS_Store etc — only process dirs starting with `-`
            if !name.starts_with('-') {
                return None;
            }
            if !e.path().is_dir() {
                return None;
            }
            let project_name = project_name_from_entry(&name);
            let decoded_path = decode_project_path(&name);
            Some(ProjectEntry {
                entry_name: name,
                project_name,
                decoded_path,
            })
        })
        .collect();

    // Sort alphabetically by entry name for a stable, reproducible order.
    // The `~/.claude/projects/` dir has no intrinsic ordering guarantee across
    // filesystems, so we impose lexicographic order on the encoded name which
    // preserves the path depth structure.
    entries.sort_by(|a, b| a.entry_name.cmp(&b.entry_name));
    entries
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ProjectEntry {
    /// Raw directory name under `~/.claude/projects/`.
    pub entry_name: String,
    /// Short human-readable name (last path segment).
    pub project_name: String,
    /// Decoded absolute path this entry represents.
    pub decoded_path: PathBuf,
}

/// Reads project entries from a custom directory — used in tests to avoid
/// depending on the real `~/.claude/projects/` path.
#[cfg(test)]
pub(crate) fn read_entries_from_dir(dir: &std::path::Path) -> Vec<ProjectEntry> {
    let read_dir = match std::fs::read_dir(dir) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    let mut entries: Vec<ProjectEntry> = read_dir
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().into_string().ok()?;
            if !name.starts_with('-') || !e.path().is_dir() {
                return None;
            }
            let project_name = project_name_from_entry(&name);
            let decoded_path = decode_project_path(&name);
            Some(ProjectEntry {
                entry_name: name,
                project_name,
                decoded_path,
            })
        })
        .collect();
    entries.sort_by(|a, b| a.entry_name.cmp(&b.entry_name));
    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_simple_project_path() {
        assert_eq!(
            decode_project_path("-Users-joe-dev-warpx"),
            PathBuf::from("/Users/joe/dev/warpx")
        );
    }

    #[test]
    fn decode_hidden_dir_path() {
        // Double dash encodes a dot-prefixed component: `--claude` → `/.claude`
        assert_eq!(
            decode_project_path("-Users-joe--claude"),
            PathBuf::from("/Users/joe/.claude")
        );
    }

    #[test]
    fn decode_home_dir_path() {
        assert_eq!(
            decode_project_path("-Users-joe"),
            PathBuf::from("/Users/joe")
        );
    }

    #[test]
    fn project_name_last_segment() {
        assert_eq!(project_name_from_entry("-Users-joe-dev-warpx"), "warpx");
        assert_eq!(project_name_from_entry("-Users-joe-dev-minibox"), "minibox");
        assert_eq!(project_name_from_entry("-Users-joe-dev-doob"), "doob");
    }

    #[test]
    fn project_name_hidden_dir() {
        assert_eq!(project_name_from_entry("-Users-joe--claude"), ".claude");
    }

    #[test]
    fn project_name_shallow() {
        assert_eq!(project_name_from_entry("-Users-joe"), "joe");
    }

    #[test]
    fn read_project_entries_from_temp_dir() {
        let tmp = tempfile::tempdir().unwrap();
        // Create fake entry dirs
        std::fs::create_dir(tmp.path().join("-Users-joe-dev-warpx")).unwrap();
        std::fs::create_dir(tmp.path().join("-Users-joe-dev-minibox")).unwrap();
        // A non-dir file — should be skipped
        std::fs::write(tmp.path().join(".DS_Store"), "").unwrap();
        // A dir not starting with `-` — should be skipped
        std::fs::create_dir(tmp.path().join("other")).unwrap();

        // Override HOME to point at a predictable structure by calling the
        // low-level helper directly with a custom dir.
        let entries = read_entries_from_dir(tmp.path());
        let names: Vec<&str> = entries.iter().map(|e| e.project_name.as_str()).collect();
        assert_eq!(names, vec!["minibox", "warpx"]);
    }

    #[test]
    fn read_project_entries_sorted_lexicographically() {
        let tmp = tempfile::tempdir().unwrap();
        // Entry names sort: aa < mm < zz — decoded last segment is the part after final `/`
        std::fs::create_dir(tmp.path().join("-Users-joe-dev-zz")).unwrap();
        std::fs::create_dir(tmp.path().join("-Users-joe-dev-aa")).unwrap();
        std::fs::create_dir(tmp.path().join("-Users-joe-dev-mm")).unwrap();

        let entries = read_entries_from_dir(tmp.path());
        let names: Vec<&str> = entries.iter().map(|e| e.project_name.as_str()).collect();
        assert_eq!(names, vec!["aa", "mm", "zz"]);
    }
}
