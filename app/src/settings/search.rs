/// Settings full-text search index.
///
/// Gated on `FeatureFlag::OzSettingsSearch`.
///
/// At init time (or on first use), call `SettingsSearchIndex::build` to create
/// a flat list of all public setting storage keys. Subsequent `filter` calls
/// are O(n) substring matches over the pre-built index — no tree traversal on
/// each keystroke.
use settings::SettingsManager;
use warpui::{AppContext, SingletonEntity};

/// A single entry in the flat settings index.
#[derive(Clone, Debug)]
pub struct SettingsEntry {
    /// The TOML-visible storage key (e.g. `"editor.cursor_blink"`).
    pub key: String,
    /// The section derived from the key prefix (e.g. `"editor"`).
    pub section: String,
}

impl SettingsEntry {
    fn new(key: &str) -> Self {
        let section = key.split('.').next().unwrap_or(key).to_owned();
        Self {
            key: key.to_owned(),
            section,
        }
    }
}

/// Pre-built flat index of all public settings.
#[derive(Clone, Default, Debug)]
pub struct SettingsSearchIndex {
    entries: Vec<SettingsEntry>,
}

impl SettingsSearchIndex {
    /// Build the index from the currently registered settings.
    pub fn build(ctx: &AppContext) -> Self {
        let manager = SettingsManager::as_ref(ctx);
        let mut entries: Vec<SettingsEntry> = manager
            .public_storage_keys()
            .map(SettingsEntry::new)
            .collect();
        entries.sort_by(|a, b| a.key.cmp(&b.key));
        Self { entries }
    }

    /// Return entries whose key or section contain `query` (case-insensitive).
    /// Returns all entries when `query` is empty.
    pub fn filter<'a>(&'a self, query: &str) -> Vec<&'a SettingsEntry> {
        if query.is_empty() {
            return self.entries.iter().collect();
        }
        let q = query.to_lowercase();
        self.entries
            .iter()
            .filter(|e| e.key.to_lowercase().contains(&q) || e.section.to_lowercase().contains(&q))
            .collect()
    }

    /// Number of indexed entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// True when no settings are indexed.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// UI state for the settings search bar.
#[derive(Clone, Default, Debug)]
pub struct SettingsSearchState {
    /// Current user query string.
    pub query: String,
    /// Lazily-built index.
    index: Option<SettingsSearchIndex>,
}

impl SettingsSearchState {
    /// Ensure the index is built (idempotent).
    pub fn ensure_index(&mut self, ctx: &AppContext) {
        if self.index.is_none() {
            self.index = Some(SettingsSearchIndex::build(ctx));
        }
    }

    /// Update the query. Returns filtered results. Builds index on first call.
    pub fn set_query(&mut self, query: impl Into<String>, ctx: &AppContext) -> Vec<SettingsEntry> {
        self.ensure_index(ctx);
        self.query = query.into();
        self.results()
    }

    /// Clear the current query.
    pub fn clear(&mut self) {
        self.query.clear();
    }

    /// Current filtered results. Requires index to have been built.
    pub fn results(&self) -> Vec<SettingsEntry> {
        match &self.index {
            Some(idx) => idx.filter(&self.query).into_iter().cloned().collect(),
            None => Vec::new(),
        }
    }

    /// Whether a search is currently active (non-empty query).
    pub fn is_active(&self) -> bool {
        !self.query.is_empty()
    }
}

#[cfg(test)]
mod search_tests {
    use super::*;

    fn make_entry(key: &str) -> SettingsEntry {
        SettingsEntry::new(key)
    }

    fn make_index(keys: &[&str]) -> SettingsSearchIndex {
        SettingsSearchIndex {
            entries: keys.iter().map(|k| make_entry(k)).collect(),
        }
    }

    #[test]
    fn empty_query_returns_all() {
        let idx = make_index(&["editor.cursor_blink", "theme.background", "font.size"]);
        assert_eq!(idx.filter("").len(), 3);
    }

    #[test]
    fn query_filters_by_key() {
        let idx = make_index(&["editor.cursor_blink", "theme.background", "font.size"]);
        let results = idx.filter("cursor");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].key, "editor.cursor_blink");
    }

    #[test]
    fn query_filters_by_section() {
        let idx = make_index(&["editor.cursor_blink", "editor.tab_size", "font.size"]);
        let results = idx.filter("editor");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn query_is_case_insensitive() {
        let idx = make_index(&["editor.cursor_blink", "theme.background"]);
        let results = idx.filter("CURSOR");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn no_match_returns_empty() {
        let idx = make_index(&["editor.cursor_blink", "theme.background"]);
        let results = idx.filter("zzznomatch");
        assert!(results.is_empty());
    }

    #[test]
    fn section_is_derived_from_key_prefix() {
        let e = make_entry("editor.cursor_blink");
        assert_eq!(e.section, "editor");
        let e2 = make_entry("font");
        assert_eq!(e2.section, "font");
    }

    #[test]
    fn state_clear_resets_query() {
        let mut state = SettingsSearchState {
            query: "cursor".to_owned(),
            index: None,
        };
        state.clear();
        assert!(state.query.is_empty());
        assert!(!state.is_active());
    }
}
