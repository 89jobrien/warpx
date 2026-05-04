//! Doob task data layer — types and IO.
//!
//! WarpUI model/panel wrappers live in `app/src/joe/doob/`.

use serde::Deserialize;

/// A single task item returned by `doob todo list --json`.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct DoobItem {
    pub id: String,
    pub title: Option<String>,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub description: Option<String>,
    pub project: Option<String>,
}

/// Shell out to `doob todo list --json` and parse the result.
/// Runs synchronously; intended to be called from a spawn closure.
pub fn fetch_items() -> Result<Vec<DoobItem>, String> {
    let output = command::blocking::Command::new("doob")
        .args(["todo", "list", "--json"])
        .output()
        .map_err(|e| format!("failed to run doob: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("doob exited with error: {stderr}"));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let items: Vec<DoobItem> =
        serde_json::from_str(&stdout).map_err(|e| format!("failed to parse doob output: {e}"))?;

    log::info!("[doob] loaded {} items", items.len());
    Ok(items)
}
