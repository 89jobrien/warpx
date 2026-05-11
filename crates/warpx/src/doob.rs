//! Doob task data layer — types and IO.
//!
//! WarpUI model/panel wrappers live in `app/src/joe/doob/`.

use serde::Deserialize;
use serde_json::Value;

/// A single task item returned by `doob todo list --json`.
#[derive(Clone, Debug, PartialEq)]
pub struct DoobItem {
    pub id: String,
    pub title: Option<String>,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub description: Option<String>,
    pub project: Option<String>,
}

#[derive(Deserialize)]
struct DoobEnvelope {
    #[serde(default)]
    todos: Vec<RawDoobItem>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum DoobOutput {
    Envelope(DoobEnvelope),
    Items(Vec<RawDoobItem>),
}

#[derive(Deserialize)]
struct RawDoobItem {
    id: Option<Value>,
    uuid: Option<String>,
    title: Option<String>,
    content: Option<String>,
    status: Option<String>,
    priority: Option<Value>,
    description: Option<String>,
    project: Option<String>,
}

impl RawDoobItem {
    fn into_item(self) -> DoobItem {
        let id = self
            .id
            .as_ref()
            .and_then(extract_doob_id)
            .or(self.uuid)
            .unwrap_or_default();

        DoobItem {
            id,
            title: self.title.or(self.content),
            status: self.status.map(normalize_status),
            priority: self.priority.and_then(format_priority),
            description: self.description,
            project: self.project,
        }
    }
}

fn normalize_status(status: String) -> String {
    status.replace('-', "_")
}

fn extract_doob_id(value: &Value) -> Option<String> {
    match value {
        Value::String(value) => Some(value.clone()),
        Value::Object(map) => map
            .get("id")
            .and_then(|id| id.get("String").or_else(|| id.get("Number")))
            .and_then(|value| match value {
                Value::String(value) => Some(value.clone()),
                Value::Number(value) => Some(value.to_string()),
                _ => None,
            }),
        _ => None,
    }
}

fn format_priority(value: Value) -> Option<String> {
    match value {
        Value::String(priority) if priority.starts_with('P') => Some(priority),
        Value::String(priority) if !priority.is_empty() => Some(format!("P{priority}")),
        Value::Number(priority) => Some(format!("P{priority}")),
        _ => None,
    }
}

pub fn parse_items_from_value(value: Value) -> Result<Vec<DoobItem>, String> {
    let output: DoobOutput =
        serde_json::from_value(value).map_err(|e| format!("failed to parse doob output: {e}"))?;
    let items = match output {
        DoobOutput::Envelope(envelope) => envelope.todos,
        DoobOutput::Items(items) => items,
    };
    Ok(items.into_iter().map(RawDoobItem::into_item).collect())
}

pub fn parse_items_from_str(stdout: &str) -> Result<Vec<DoobItem>, String> {
    let value: Value =
        serde_json::from_str(stdout).map_err(|e| format!("failed to parse doob output: {e}"))?;
    parse_items_from_value(value)
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
    let items = parse_items_from_str(&stdout)?;

    log::info!("[doob] loaded {} items", items.len());
    Ok(items)
}

#[cfg(test)]
mod tests {
    use super::{fetch_items, parse_items_from_str};
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn fetch_items_parses_current_doob_json_envelope() {
        let _guard = crate::test_env::ENV_LOCK.lock().unwrap();
        let temp = tempfile::tempdir().unwrap();
        let bin_dir = temp.path().join("bin");
        fs::create_dir(&bin_dir).unwrap();
        let doob = bin_dir.join("doob");
        fs::write(
            &doob,
            r#"#!/bin/sh
cat <<'JSON'
{
  "count": 1,
  "todos": [
    {
      "id": {"tb": "todo", "id": {"String": "abc123"}},
      "uuid": "f6ee5a3c-b69a-44b3-ac9f-62ead47a2584",
      "content": "Triage TODO comments",
      "status": "pending",
      "priority": 1,
      "due_date": null,
      "project": "warpx",
      "metadata": null
    }
  ]
}
JSON
"#,
        )
        .unwrap();
        let mut perms = fs::metadata(&doob).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&doob, perms).unwrap();

        let old_path = std::env::var_os("PATH");
        let new_path = match old_path.as_ref() {
            Some(path) => {
                let mut paths = vec![bin_dir.clone()];
                paths.extend(std::env::split_paths(path));
                std::env::join_paths(paths).unwrap()
            }
            None => bin_dir.clone().into_os_string(),
        };
        unsafe { std::env::set_var("PATH", new_path) };

        let items = fetch_items().unwrap();

        if let Some(path) = old_path {
            unsafe { std::env::set_var("PATH", path) };
        } else {
            unsafe { std::env::remove_var("PATH") };
        }

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, "abc123");
        assert_eq!(items[0].title.as_deref(), Some("Triage TODO comments"));
        assert_eq!(items[0].status.as_deref(), Some("pending"));
        assert_eq!(items[0].priority.as_deref(), Some("P1"));
        assert_eq!(items[0].project.as_deref(), Some("warpx"));
    }

    #[test]
    fn parse_items_normalizes_in_progress_status() {
        let items = parse_items_from_str(
            r#"{
  "count": 1,
  "todos": [
    {
      "id": "abc123",
      "title": "Wire MCP add",
      "status": "in-progress",
      "priority": "P2",
      "project": "warpx"
    }
  ]
}"#,
        )
        .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].status.as_deref(), Some("in_progress"));
    }

    #[test]
    fn parse_items_accepts_array_output() {
        let items = parse_items_from_str(
            r#"[
  {
    "uuid": "abc123",
    "content": "Array task",
    "status": "pending",
    "priority": 3,
    "project": "warpx"
  }
]"#,
        )
        .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, "abc123");
        assert_eq!(items[0].title.as_deref(), Some("Array task"));
        assert_eq!(items[0].priority.as_deref(), Some("P3"));
    }
}
