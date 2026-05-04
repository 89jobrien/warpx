/// Portable settings export / import.
///
/// Gated on `FeatureFlag::OzSettingsPortable`.
///
/// Export copies the user's public settings (via the `SettingsManager`
/// default-value registry) to `~/.warp/settings-export.toml`.  Import reads
/// that file back, validates the TOML structure, and applies each recognised
/// public key via `update_setting_with_storage_key`.
///
/// # Security contract
/// - Private settings (flagged `is_private` in the manager) are never written.
/// - No auth tokens, API keys, team identifiers, or cloud-sync preferences are
///   included.  The known exclusion list is enforced via `EXCLUDED_KEY_PREFIXES`.
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use settings::SettingsManager;
use warpui::{AppContext, SingletonEntity};

/// Key prefixes / substrings that are unconditionally excluded from export,
/// regardless of the `is_private` flag, because they may contain credentials,
/// auth tokens, or machine-specific cloud state.
const EXCLUDED_KEY_PREFIXES: &[&str] = &[
    "auth.",
    "cloud.",
    "account.",
    "team.",
    "session_sharing.",
    "api_key",
    "token",
    "secret",
    "credential",
    "license",
    "warp_id",
    "user_id",
];

fn is_excluded(key: &str) -> bool {
    let lower = key.to_lowercase();
    EXCLUDED_KEY_PREFIXES
        .iter()
        .any(|prefix| lower.starts_with(prefix) || lower.contains(prefix))
}

/// Returns the canonical path for the portable export file.
pub fn export_file_path() -> PathBuf {
    warp_core::paths::config_local_dir().join("settings-export.toml")
}

/// Summary returned after a successful import.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportSummary {
    /// Keys that were successfully applied.
    pub applied: Vec<String>,
    /// Keys present in the file but not recognised by the current build.
    pub skipped_unknown: Vec<String>,
    /// Keys that were excluded for security reasons (auth/cloud/etc.).
    pub skipped_excluded: Vec<String>,
}

impl ImportSummary {
    /// True when every recognised key was applied without issues.
    pub fn is_clean(&self) -> bool {
        self.skipped_unknown.is_empty() && self.skipped_excluded.is_empty()
    }
}

/// Collect all exportable (public, non-excluded) settings from the manager and
/// write them to `~/.warp/settings-export.toml`.
///
/// The output is a flat TOML `[settings]` table mapping storage keys to their
/// current default/serialised values.
pub fn export_settings(ctx: &AppContext) -> Result<PathBuf> {
    let manager = SettingsManager::as_ref(ctx);

    let mut map: toml::map::Map<String, toml::Value> = toml::map::Map::new();

    for (key, value) in manager.default_values() {
        if is_excluded(&key) {
            continue;
        }
        // `value` is JSON-serialised.  Parse and convert to TOML so the
        // output file is human-readable.
        let parsed: serde_json::Value =
            serde_json::from_str(&value).unwrap_or(serde_json::Value::String(value));
        if let Some(tv) = json_to_toml(parsed) {
            map.insert(key, tv);
        }
    }

    let mut root = toml::map::Map::new();
    root.insert("settings".to_owned(), toml::Value::Table(map));
    let serialized = toml::to_string_pretty(&toml::Value::Table(root))
        .context("serializing portable settings to TOML")?;

    let dest = export_file_path();
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating directory {}", parent.display()))?;
    }
    std::fs::write(&dest, serialized)
        .with_context(|| format!("writing export file {}", dest.display()))?;

    Ok(dest)
}

/// Read a portable export file and apply each recognised setting.
///
/// Unknown keys are collected in [`ImportSummary::skipped_unknown`].
/// Security-excluded keys are collected in [`ImportSummary::skipped_excluded`].
pub fn import_settings(path: &Path, ctx: &mut AppContext) -> Result<ImportSummary> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("reading import file {}", path.display()))?;

    let root: toml::Value = toml::from_str(&content)
        .with_context(|| format!("parsing TOML from {}", path.display()))?;

    let table = root
        .get("settings")
        .and_then(|v| v.as_table())
        .context("import file must have a [settings] table")?
        .clone();

    // Snapshot public keys while holding a read reference.
    let known_keys: HashSet<String> = {
        let manager = SettingsManager::as_ref(ctx);
        manager.public_storage_keys().map(str::to_owned).collect()
    };

    let mut summary = ImportSummary::default();
    let mut to_apply: Vec<(String, String)> = Vec::new();

    for (key, value) in &table {
        if is_excluded(key) {
            summary.skipped_excluded.push(key.clone());
            continue;
        }
        if !known_keys.contains(key.as_str()) {
            summary.skipped_unknown.push(key.clone());
            continue;
        }
        // Convert TOML value → JSON string for the settings manager.
        let json_val = toml_to_json(value.clone());
        match serde_json::to_string(&json_val) {
            Ok(serialized) => to_apply.push((key.clone(), serialized)),
            Err(e) => {
                summary
                    .skipped_unknown
                    .push(format!("{key} (serialize error: {e})"));
            }
        }
    }

    // Apply settings through the manager using the correct warpui update pattern.
    SettingsManager::handle(ctx).update(ctx, |manager, ctx| {
        for (key, value) in to_apply {
            match manager.update_setting_with_storage_key(&key, value, false, ctx) {
                Ok(()) => summary.applied.push(key),
                Err(e) => summary
                    .skipped_unknown
                    .push(format!("{key} (apply error: {e})")),
            }
        }
    });

    Ok(summary)
}

// ── TOML ↔ JSON conversion helpers ──────────────────────────────────────────

fn json_to_toml(v: serde_json::Value) -> Option<toml::Value> {
    match v {
        serde_json::Value::Null => None,
        serde_json::Value::Bool(b) => Some(toml::Value::Boolean(b)),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Some(toml::Value::Integer(i))
            } else {
                n.as_f64().map(toml::Value::Float)
            }
        }
        serde_json::Value::String(s) => Some(toml::Value::String(s)),
        serde_json::Value::Array(arr) => {
            let items: Vec<toml::Value> = arr.into_iter().filter_map(json_to_toml).collect();
            Some(toml::Value::Array(items))
        }
        serde_json::Value::Object(obj) => {
            let map: toml::map::Map<String, toml::Value> = obj
                .into_iter()
                .filter_map(|(k, v)| json_to_toml(v).map(|tv| (k, tv)))
                .collect();
            Some(toml::Value::Table(map))
        }
    }
}

fn toml_to_json(v: toml::Value) -> serde_json::Value {
    match v {
        toml::Value::Boolean(b) => serde_json::Value::Bool(b),
        toml::Value::Integer(i) => serde_json::Value::Number(i.into()),
        toml::Value::Float(f) => serde_json::Number::from_f64(f)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        toml::Value::String(s) => serde_json::Value::String(s),
        toml::Value::Datetime(dt) => serde_json::Value::String(dt.to_string()),
        toml::Value::Array(arr) => {
            serde_json::Value::Array(arr.into_iter().map(toml_to_json).collect())
        }
        toml::Value::Table(table) => {
            let obj: serde_json::Map<String, serde_json::Value> = table
                .into_iter()
                .map(|(k, v)| (k, toml_to_json(v)))
                .collect();
            serde_json::Value::Object(obj)
        }
    }
}

#[cfg(test)]
mod portable_tests {
    use super::*;

    #[test]
    fn is_excluded_matches_auth_prefix() {
        assert!(is_excluded("auth.token"));
        assert!(is_excluded("cloud.sync_enabled"));
        assert!(is_excluded("team.id"));
    }

    #[test]
    fn is_excluded_ignores_normal_keys() {
        assert!(!is_excluded("editor.cursor_blink"));
        assert!(!is_excluded("font.size"));
        assert!(!is_excluded("theme.background"));
    }

    #[test]
    fn import_summary_is_clean_when_no_skips() {
        let s = ImportSummary {
            applied: vec!["editor.cursor_blink".to_owned()],
            skipped_unknown: vec![],
            skipped_excluded: vec![],
        };
        assert!(s.is_clean());
    }

    #[test]
    fn import_summary_not_clean_with_unknown() {
        let s = ImportSummary {
            applied: vec![],
            skipped_unknown: vec!["future_key".to_owned()],
            skipped_excluded: vec![],
        };
        assert!(!s.is_clean());
    }

    #[test]
    fn json_to_toml_bool() {
        let tv = json_to_toml(serde_json::Value::Bool(true)).unwrap();
        assert_eq!(tv, toml::Value::Boolean(true));
    }

    #[test]
    fn json_to_toml_string() {
        let tv = json_to_toml(serde_json::Value::String("bar".into())).unwrap();
        assert_eq!(tv, toml::Value::String("bar".into()));
    }

    #[test]
    fn json_to_toml_null_returns_none() {
        assert!(json_to_toml(serde_json::Value::Null).is_none());
    }

    #[test]
    fn toml_to_json_bool() {
        let jv = toml_to_json(toml::Value::Boolean(false));
        assert_eq!(jv, serde_json::Value::Bool(false));
    }

    #[test]
    fn toml_to_json_string() {
        let jv = toml_to_json(toml::Value::String("hello".into()));
        assert_eq!(jv, serde_json::Value::String("hello".into()));
    }

    #[test]
    fn export_file_path_ends_with_export_toml() {
        let p = export_file_path();
        assert!(p.to_string_lossy().ends_with("settings-export.toml"));
    }

    #[test]
    fn round_trip_string_value() {
        let original = serde_json::Value::String("enabled".into());
        let tv = json_to_toml(original.clone()).unwrap();
        let back = toml_to_json(tv);
        assert_eq!(back, original);
    }

    #[test]
    fn round_trip_bool_value() {
        let original = serde_json::Value::Bool(true);
        let tv = json_to_toml(original.clone()).unwrap();
        let back = toml_to_json(tv);
        assert_eq!(back, original);
    }

    #[test]
    fn round_trip_integer_value() {
        let original = serde_json::Value::Number(42.into());
        let tv = json_to_toml(original.clone()).unwrap();
        let back = toml_to_json(tv);
        assert_eq!(back, original);
    }
}
