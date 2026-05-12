//! Context provider trait and orchestrator.
//!
//! Each [`ContextProvider`] owns one section of the Context Window panel.
//! The [`ContextOrchestrator`] manages all providers, tracks timestamped
//! state, and exposes a unified snapshot for the UI layer.
//!
//! Providers have two paths:
//! - **`collect`** (fast): read a sidecar JSON from `context_dir()`.
//! - **`generate`** (slow): shell out / compute, write sidecar, return data.
//!
//! File-watching and async scheduling live in the app layer (warpui model),
//! not here — this crate stays free of async/notify deps.

use instant::Instant;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Section types
// ---------------------------------------------------------------------------

/// A single key-value row displayed in a section.
#[derive(Clone, Debug, PartialEq)]
pub struct KvRow {
    pub key: String,
    pub value: String,
}

/// An item with a priority badge.
#[derive(Clone, Debug, PartialEq)]
pub struct PriorityItem {
    pub priority: String,
    pub text: String,
    pub status: String,
}

/// Visual hint for a status line.
#[derive(Clone, Debug, PartialEq)]
pub enum StatusIcon {
    Ok,
    Warn,
    Error,
    Unknown,
}

/// The data shape a provider returns. The panel renders each variant
/// differently.
#[derive(Clone, Debug, PartialEq)]
pub enum ContextSection {
    /// Key-value pairs with optional sub-items (git, AI context).
    KeyValue {
        rows: Vec<KvRow>,
        sub_items: Vec<String>,
    },
    /// Priority-badged list (handoff, todos).
    PriorityList(Vec<PriorityItem>),
    /// Single status line with icon (CI, cargo, env health).
    StatusLine { icon: StatusIcon, text: String },
    /// Provider has no data yet or the tool is unavailable.
    Unavailable(String),
}

/// A section with a timestamp recording when it was last refreshed.
#[derive(Clone, Debug)]
pub struct TimestampedSection {
    pub provider_id: &'static str,
    pub label: String,
    pub data: ContextSection,
    pub updated_at: Instant,
}

// ---------------------------------------------------------------------------
// Provider trait
// ---------------------------------------------------------------------------

/// A single data source for the Context Window panel.
///
/// Implementations live in [`crate::providers`]. Each provider is
/// self-contained: it knows where its sidecar file lives, how to read it,
/// and how to generate fresh data when the sidecar is missing.
pub trait ContextProvider: Send + Sync {
    /// Unique key used for ordering and dedup.
    fn id(&self) -> &'static str;

    /// Display name for the section header in the panel.
    fn label(&self) -> &str;

    /// Paths the file-watcher should monitor. When any of these change,
    /// the orchestrator re-calls [`collect`](ContextProvider::collect).
    fn watch_paths(&self, cwd: &Path) -> Vec<PathBuf>;

    /// Fast path: read current state, typically from a sidecar JSON.
    /// Returns `None` when no cached data exists (triggers `generate`).
    fn collect(&self, cwd: &Path) -> Option<ContextSection>;

    /// Slow path: compute fresh data, write sidecar as side-effect,
    /// return the section. Called on a blocking thread.
    fn generate(&self, cwd: &Path) -> ContextSection;
}

// ---------------------------------------------------------------------------
// Orchestrator
// ---------------------------------------------------------------------------

/// Owns all providers and their most recent output.
///
/// The orchestrator is **not** async-aware. The app layer (warpui model)
/// calls `init`, `refresh`, and `refresh_one` from appropriate contexts
/// (sync for `collect`, `spawn_blocking` for `generate`).
pub struct ContextOrchestrator {
    providers: Vec<Box<dyn ContextProvider>>,
    sections: Vec<TimestampedSection>,
    cwd: PathBuf,
}

impl ContextOrchestrator {
    /// Create an orchestrator with the given providers and working directory.
    pub fn new(providers: Vec<Box<dyn ContextProvider>>, cwd: PathBuf) -> Self {
        let sections = providers
            .iter()
            .map(|p| TimestampedSection {
                provider_id: p.id(),
                label: p.label().to_string(),
                data: ContextSection::Unavailable("not loaded".into()),
                updated_at: Instant::now(),
            })
            .collect();
        Self {
            providers,
            sections,
            cwd,
        }
    }

    /// Try the fast path (`collect`) for every provider. Returns the ids
    /// of providers that returned `None` (needing `generate`).
    pub fn init(&mut self) -> Vec<&'static str> {
        let mut needs_generate = Vec::new();
        for (i, provider) in self.providers.iter().enumerate() {
            match provider.collect(&self.cwd) {
                Some(data) => {
                    self.sections[i].data = data;
                    self.sections[i].updated_at = Instant::now();
                }
                None => {
                    needs_generate.push(provider.id());
                }
            }
        }
        needs_generate
    }

    /// Run `generate` for a single provider by id. Returns the new section
    /// data (already stored internally).
    ///
    /// Intended to be called from a blocking thread.
    pub fn generate_one(&mut self, id: &str) -> Option<&TimestampedSection> {
        let idx = self.providers.iter().position(|p| p.id() == id)?;
        let data = self.providers[idx].generate(&self.cwd);
        self.sections[idx].data = data;
        self.sections[idx].updated_at = Instant::now();
        Some(&self.sections[idx])
    }

    /// Re-collect a single provider (fast path only). Used when the
    /// file-watcher detects a change on one of the provider's watch paths.
    pub fn refresh_one(&mut self, id: &str) -> Option<&TimestampedSection> {
        let idx = self.providers.iter().position(|p| p.id() == id)?;
        let data = self.providers[idx]
            .collect(&self.cwd)
            .unwrap_or_else(|| self.providers[idx].generate(&self.cwd));
        self.sections[idx].data = data;
        self.sections[idx].updated_at = Instant::now();
        Some(&self.sections[idx])
    }

    /// Re-generate all providers. Intended for the manual refresh button.
    pub fn refresh_all(&mut self) {
        for (i, provider) in self.providers.iter().enumerate() {
            let data = provider
                .collect(&self.cwd)
                .unwrap_or_else(|| provider.generate(&self.cwd));
            self.sections[i].data = data;
            self.sections[i].updated_at = Instant::now();
        }
    }

    /// Current snapshot of all sections, in provider registration order.
    pub fn sections(&self) -> &[TimestampedSection] {
        &self.sections
    }

    /// All watch paths across all providers, paired with provider id.
    pub fn all_watch_paths(&self) -> Vec<(PathBuf, &'static str)> {
        self.providers
            .iter()
            .flat_map(|p| {
                let id = p.id();
                p.watch_paths(&self.cwd)
                    .into_iter()
                    .map(move |path| (path, id))
            })
            .collect()
    }

    /// The working directory this orchestrator was created for.
    pub fn cwd(&self) -> &Path {
        &self.cwd
    }

    /// Look up a provider index by id.
    pub fn provider_index(&self, id: &str) -> Option<usize> {
        self.providers.iter().position(|p| p.id() == id)
    }
}

// ---------------------------------------------------------------------------
// Sidecar helpers
// ---------------------------------------------------------------------------

/// The directory where providers write/read sidecar JSON files.
/// `~/.warp/context.d/`
pub fn context_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    PathBuf::from(home).join(".warp").join("context.d")
}

/// Ensure the context.d directory exists.
pub fn ensure_context_dir() -> std::io::Result<()> {
    std::fs::create_dir_all(context_dir())
}

/// Read and deserialize a sidecar JSON file from context.d.
pub fn read_sidecar<T: serde::de::DeserializeOwned>(filename: &str) -> Option<T> {
    let path = context_dir().join(filename);
    let content = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&content).ok()
}

/// Write a sidecar JSON file to context.d.
pub fn write_sidecar<T: serde::Serialize>(filename: &str, data: &T) -> Result<(), String> {
    ensure_context_dir().map_err(|e| format!("failed to create context.d: {e}"))?;
    let path = context_dir().join(filename);
    let json = serde_json::to_string_pretty(data)
        .map_err(|e| format!("failed to serialize {filename}: {e}"))?;
    std::fs::write(&path, json).map_err(|e| format!("failed to write {path:?}: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// A trivial provider for testing the orchestrator.
    struct StubProvider {
        id: &'static str,
        collect_returns: Option<ContextSection>,
        generate_returns: ContextSection,
    }

    impl ContextProvider for StubProvider {
        fn id(&self) -> &'static str {
            self.id
        }
        fn label(&self) -> &str {
            self.id
        }
        fn watch_paths(&self, _cwd: &Path) -> Vec<PathBuf> {
            vec![]
        }
        fn collect(&self, _cwd: &Path) -> Option<ContextSection> {
            self.collect_returns.clone()
        }
        fn generate(&self, _cwd: &Path) -> ContextSection {
            self.generate_returns.clone()
        }
    }

    fn stub(
        id: &'static str,
        collect: Option<ContextSection>,
        generate: ContextSection,
    ) -> Box<dyn ContextProvider> {
        Box::new(StubProvider {
            id,
            collect_returns: collect,
            generate_returns: generate,
        })
    }

    #[test]
    fn init_collects_available_and_flags_missing() {
        let git_data = ContextSection::KeyValue {
            rows: vec![KvRow {
                key: "branch".into(),
                value: "main".into(),
            }],
            sub_items: vec![],
        };
        let ci_gen = ContextSection::StatusLine {
            icon: StatusIcon::Ok,
            text: "passed".into(),
        };

        let providers: Vec<Box<dyn ContextProvider>> = vec![
            stub("git", Some(git_data.clone()), git_data.clone()),
            stub("ci", None, ci_gen),
        ];

        let mut orch = ContextOrchestrator::new(providers, PathBuf::from("/tmp"));
        let needs = orch.init();

        assert_eq!(needs, vec!["ci"]);
        assert_eq!(orch.sections()[0].data, git_data);
        assert!(matches!(
            orch.sections()[1].data,
            ContextSection::Unavailable(_)
        ));
    }

    #[test]
    fn generate_one_updates_section() {
        let generated = ContextSection::StatusLine {
            icon: StatusIcon::Warn,
            text: "2 warnings".into(),
        };
        let providers: Vec<Box<dyn ContextProvider>> = vec![stub("cargo", None, generated.clone())];

        let mut orch = ContextOrchestrator::new(providers, PathBuf::from("/tmp"));
        orch.init();

        let section = orch.generate_one("cargo").unwrap();
        assert_eq!(section.data, generated);
    }

    #[test]
    fn refresh_all_updates_every_section() {
        let b = ContextSection::StatusLine {
            icon: StatusIcon::Ok,
            text: "ok".into(),
        };
        let providers: Vec<Box<dyn ContextProvider>> = vec![
            stub("a", Some(b.clone()), b.clone()),
            stub("b", Some(b.clone()), b.clone()),
        ];

        let mut orch = ContextOrchestrator::new(providers, PathBuf::from("/tmp"));
        // Sections start as Unavailable
        assert!(matches!(
            orch.sections()[0].data,
            ContextSection::Unavailable(_)
        ));

        orch.refresh_all();
        assert_eq!(orch.sections()[0].data, b);
        assert_eq!(orch.sections()[1].data, b);
    }

    #[test]
    fn all_watch_paths_aggregates_across_providers() {
        struct WatchyProvider;
        impl ContextProvider for WatchyProvider {
            fn id(&self) -> &'static str {
                "watchy"
            }
            fn label(&self) -> &str {
                "Watchy"
            }
            fn watch_paths(&self, cwd: &Path) -> Vec<PathBuf> {
                vec![cwd.join(".git/HEAD"), cwd.join(".git/index")]
            }
            fn collect(&self, _cwd: &Path) -> Option<ContextSection> {
                None
            }
            fn generate(&self, _cwd: &Path) -> ContextSection {
                ContextSection::Unavailable("n/a".into())
            }
        }

        let providers: Vec<Box<dyn ContextProvider>> = vec![Box::new(WatchyProvider)];
        let orch = ContextOrchestrator::new(providers, PathBuf::from("/repo"));
        let paths = orch.all_watch_paths();
        assert_eq!(paths.len(), 2);
        assert_eq!(paths[0].1, "watchy");
    }

    #[test]
    fn generate_one_returns_none_for_unknown_id() {
        let providers: Vec<Box<dyn ContextProvider>> = vec![];
        let mut orch = ContextOrchestrator::new(providers, PathBuf::from("/tmp"));
        assert!(orch.generate_one("nonexistent").is_none());
    }
}
