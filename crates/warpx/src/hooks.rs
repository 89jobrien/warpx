//! JoeMode session lifecycle hooks.
//!
//! Fires `hj detect` on session open and `godmode handoff` on session close.
//! All operations are best-effort — failures are logged to stderr, never fatal.
//! Gated at call sites behind FeatureFlag::JoeMode / FeatureFlag::OzSessionHooks.

use std::path::Path;

use command::blocking::Command;

/// Run on terminal tab/session open. Detects handoff items for the cwd.
/// Output is returned as a string to be displayed as a banner (or ignored).
pub fn on_session_open(cwd: &Path) -> Option<String> {
    let output = Command::new("hj")
        .arg("detect")
        .current_dir(cwd)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() { None } else { Some(text) }
}

/// Run on session close. Triggers godmode handoff validation (best-effort).
pub fn on_session_close(cwd: &Path) {
    let _ = Command::new("godmode")
        .arg("handoff")
        .current_dir(cwd)
        .output();
}

/// Fire-and-forget: runs `godmode handon` in background on tab open.
/// Never blocks the UI thread.
pub fn spawn_handon() {
    let _ = Command::new("godmode")
        .arg("handon")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
}

/// Fire-and-forget: runs `godmode handoff` in background on tab close.
/// Never blocks the UI thread.
pub fn spawn_handoff() {
    let _ = Command::new("godmode")
        .arg("handoff")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
}

/// Run on directory change. Returns doob overdue count for the new directory's repo.
pub fn on_cwd_change(new_cwd: &Path) -> Option<String> {
    let cache = crate::joe::read_doob_cache()?;
    let repo = new_cwd.file_name()?.to_string_lossy().to_string();
    let repo_count = cache.overdue_by_repo.get(&repo).copied().unwrap_or(0);

    if repo_count > 0 {
        Some(format!("doob: {repo_count} overdue in {repo}"))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::on_session_open;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn on_session_open_runs_hj_detect_from_cwd_without_path_argument() {
        let _guard = crate::test_env::ENV_LOCK.lock().unwrap();
        let temp = tempfile::tempdir().unwrap();
        let repo = temp.path().join("repo");
        let bin_dir = temp.path().join("bin");
        fs::create_dir(&repo).unwrap();
        fs::create_dir(&bin_dir).unwrap();
        let hj = bin_dir.join("hj");
        fs::write(
            &hj,
            r#"#!/bin/sh
if [ "$1" != "detect" ]; then
  echo "expected detect" >&2
  exit 2
fi
if [ "$#" -ne 1 ]; then
  echo "unexpected positional path" >&2
  exit 2
fi
if [ "$(pwd)" != "$EXPECTED_CWD" ]; then
  echo "wrong cwd: $(pwd)" >&2
  exit 3
fi
printf 'handoff ok\n'
"#,
        )
        .unwrap();
        let mut perms = fs::metadata(&hj).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&hj, perms).unwrap();

        let old_path = std::env::var_os("PATH");
        let old_expected_cwd = std::env::var_os("EXPECTED_CWD");
        let new_path = match old_path.as_ref() {
            Some(path) => {
                let mut paths = vec![bin_dir.clone()];
                paths.extend(std::env::split_paths(path));
                std::env::join_paths(paths).unwrap()
            }
            None => bin_dir.clone().into_os_string(),
        };
        let expected_cwd = fs::canonicalize(&repo).unwrap();
        unsafe {
            std::env::set_var("PATH", new_path);
            std::env::set_var("EXPECTED_CWD", expected_cwd);
        }

        let output = on_session_open(&repo);

        if let Some(path) = old_path {
            unsafe { std::env::set_var("PATH", path) };
        } else {
            unsafe { std::env::remove_var("PATH") };
        }
        if let Some(expected_cwd) = old_expected_cwd {
            unsafe { std::env::set_var("EXPECTED_CWD", expected_cwd) };
        } else {
            unsafe { std::env::remove_var("EXPECTED_CWD") };
        }

        assert_eq!(output.as_deref(), Some("handoff ok"));
    }
}
