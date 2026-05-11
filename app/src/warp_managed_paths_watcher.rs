use dirs::home_dir;
use std::path::{Path, PathBuf};
#[cfg(not(target_family = "wasm"))]
use std::{fs, sync::Arc, time::Duration};

#[cfg(not(target_family = "wasm"))]
use notify_debouncer_full::notify::{RecursiveMode, WatchFilter};
use repo_metadata::RepositoryUpdate;
#[cfg(any(not(target_family = "wasm"), test))]
use repo_metadata::TargetFile;
#[cfg(not(target_family = "wasm"))]
use warpui::ModelHandle;
use warpui::{Entity, ModelContext, SingletonEntity};
#[cfg(not(target_family = "wasm"))]
use watcher::{BulkFilesystemWatcher, BulkFilesystemWatcherEvent};

/// Duration between filesystem watch events for the Warp managed paths watcher, in milliseconds.
#[cfg(not(target_family = "wasm"))]
const WARP_MANAGED_PATHS_WATCHER_DEBOUNCE_MILLI_SECS: u64 = 500;

pub(crate) fn warp_data_dir() -> PathBuf {
    warp_core::paths::data_dir()
}

#[cfg(target_family = "wasm")]
pub(crate) fn ensure_warp_watch_roots_exist() {}

#[cfg(not(target_family = "wasm"))]
pub(crate) fn ensure_warp_watch_roots_exist() {
    let data_dir = warp_data_dir();
    if let Err(err) = fs::create_dir_all(&data_dir) {
        log::warn!(
            "Failed to create Warp data directory {}: {err}",
            data_dir.display()
        );
    }

    let config_local_dir = warp_core::paths::config_local_dir();
    if config_local_dir != data_dir {
        if let Err(err) = fs::create_dir_all(&config_local_dir) {
            log::warn!(
                "Failed to create Warp config directory {}: {err}",
                config_local_dir.display()
            );
        }
    }
}

#[cfg_attr(target_family = "wasm", allow(dead_code))]
pub(crate) fn warp_home_config_dir() -> Option<PathBuf> {
    warp_core::paths::warp_home_config_dir()
}

pub(crate) fn warp_home_skills_dir() -> Option<PathBuf> {
    warp_core::paths::warp_home_skills_dir()
}

/// Returns the root directory for Claude Code plugins: `~/.claude/plugins/`.
pub(crate) fn claude_plugins_root() -> Option<PathBuf> {
    home_dir().map(|h| h.join(".claude").join("plugins"))
}

/// Discover all `skills/` directories inside `~/.claude/plugins/` that contain
/// at least one `<skill-name>/SKILL.md` entry.  The search walks up to 6 levels
/// deep to cover paths like `cache/bazaar/godmode/0.6.0/skills/`.
///
/// Gated behind `FeatureFlag::OzClaudePluginSkills`.  When enabled, applies
/// include/exclude filtering from `~/.warp/claude-skill-sources.toml`.
pub(crate) fn claude_plugin_skill_dirs() -> Vec<PathBuf> {
    if !warp_core::features::FeatureFlag::OzClaudePluginSkills.is_enabled() {
        return Vec::new();
    }
    let Some(root) = claude_plugins_root() else {
        return Vec::new();
    };
    if !root.is_dir() {
        return Vec::new();
    }
    let mut dirs = Vec::new();
    collect_skill_dirs(&root, 0, 6, &mut dirs);

    let filter = ClaudeSkillSourceFilter::load();
    dirs.into_iter()
        .filter(|dir| filter.is_allowed(dir, &root))
        .collect()
}

/// Include/exclude filter for Claude plugin skill directories.
/// Loaded from `~/.warp/claude-skill-sources.toml`.
///
/// ```toml
/// include = ["cache/bazaar/godmode", "cache/bazaar/atelier", "hand"]
/// exclude = ["cache/toptal-maestro-playbooks", "marketplaces/langchain-skills"]
/// ```
///
/// If `include` is empty or missing, all directories are included.
/// `exclude` takes precedence over `include`.
#[derive(Debug, Default)]
pub(crate) struct ClaudeSkillSourceFilter {
    include: Vec<String>,
    exclude: Vec<String>,
}

impl ClaudeSkillSourceFilter {
    fn load() -> Self {
        let Some(config_path) =
            warp_core::paths::warp_home_config_dir().map(|d| d.join("claude-skill-sources.toml"))
        else {
            return Self::default();
        };
        let Ok(contents) = std::fs::read_to_string(&config_path) else {
            return Self::default();
        };
        Self::parse(&contents)
    }

    fn parse(toml_str: &str) -> Self {
        #[derive(serde::Deserialize, Default)]
        struct RawConfig {
            #[serde(default)]
            include: Vec<String>,
            #[serde(default)]
            exclude: Vec<String>,
        }
        let raw: RawConfig = toml::from_str(toml_str).unwrap_or_default();
        Self {
            include: raw.include,
            exclude: raw.exclude,
        }
    }

    /// Check if a skill directory is allowed by this filter.
    /// `skill_dir` is the full path; `plugins_root` is `~/.claude/plugins/`.
    fn is_allowed(&self, skill_dir: &Path, plugins_root: &Path) -> bool {
        let rel = match skill_dir.strip_prefix(plugins_root) {
            Ok(r) => r.to_string_lossy(),
            Err(_) => return true,
        };

        // Exclude takes precedence
        if self
            .exclude
            .iter()
            .any(|pattern| rel.starts_with(pattern.as_str()))
        {
            return false;
        }

        // If include is empty, everything passes
        if self.include.is_empty() {
            return true;
        }

        self.include
            .iter()
            .any(|pattern| rel.starts_with(pattern.as_str()))
    }
}

fn collect_skill_dirs(dir: &Path, depth: u32, max_depth: u32, out: &mut Vec<PathBuf>) {
    if depth > max_depth {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if path.file_name().is_some_and(|n| n == "skills") {
            if has_skill_subdirectory(&path) {
                out.push(path);
            }
        } else {
            collect_skill_dirs(&path, depth + 1, max_depth, out);
        }
    }
}

fn has_skill_subdirectory(skills_dir: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(skills_dir) else {
        return false;
    };
    entries
        .flatten()
        .any(|e| e.path().is_dir() && e.path().join("SKILL.md").exists())
}

#[cfg_attr(target_family = "wasm", allow(dead_code))]
pub(crate) fn warp_home_mcp_config_file_path() -> Option<PathBuf> {
    warp_core::paths::warp_home_mcp_config_file_path()
}

#[cfg_attr(target_family = "wasm", allow(dead_code))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WarpMcpConfigPath {
    pub(crate) root_path: PathBuf,
    pub(crate) config_path: PathBuf,
}

pub(crate) fn warp_managed_skill_dirs() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = warp_home_skills_dir().into_iter().collect();
    dirs.extend(claude_plugin_skill_dirs());
    dirs
}

#[cfg_attr(target_family = "wasm", allow(dead_code))]
pub(crate) fn warp_managed_mcp_config_path() -> Option<WarpMcpConfigPath> {
    Some(WarpMcpConfigPath {
        root_path: home_dir()?,
        config_path: warp_home_mcp_config_file_path()?,
    })
}

#[cfg_attr(target_family = "wasm", allow(dead_code))]
pub(crate) fn repository_update_touches_path(update: &RepositoryUpdate, path: &Path) -> bool {
    repository_update_paths(update).any(|candidate| candidate == path)
}

#[cfg_attr(target_family = "wasm", allow(dead_code))]
pub(crate) fn repository_update_touches_prefix(update: &RepositoryUpdate, prefix: &Path) -> bool {
    repository_update_paths(update).any(|candidate| candidate.starts_with(prefix))
}

#[cfg_attr(target_family = "wasm", allow(dead_code))]
pub(crate) fn filter_repository_update_by_prefix(
    update: &RepositoryUpdate,
    prefix: &Path,
) -> Option<RepositoryUpdate> {
    filter_repository_update(update, |path| path.starts_with(prefix))
}

#[cfg_attr(target_family = "wasm", allow(dead_code))]
fn repository_update_paths(update: &RepositoryUpdate) -> impl Iterator<Item = &Path> {
    update
        .added
        .iter()
        .map(|target| target.path.as_path())
        .chain(update.modified.iter().map(|target| target.path.as_path()))
        .chain(update.deleted.iter().map(|target| target.path.as_path()))
        .chain(update.moved.iter().flat_map(|(to_target, from_target)| {
            [to_target.path.as_path(), from_target.path.as_path()]
        }))
}

#[cfg_attr(target_family = "wasm", allow(dead_code))]
fn filter_repository_update(
    update: &RepositoryUpdate,
    keep_path: impl Fn(&Path) -> bool,
) -> Option<RepositoryUpdate> {
    let mut filtered = RepositoryUpdate {
        commit_updated: update.commit_updated,
        index_lock_detected: update.index_lock_detected,
        ..Default::default()
    };

    for target in &update.added {
        if keep_path(&target.path) {
            filtered.added.insert(target.clone());
        }
    }

    for target in &update.modified {
        if keep_path(&target.path) {
            filtered.modified.insert(target.clone());
        }
    }

    for target in &update.deleted {
        if keep_path(&target.path) {
            filtered.deleted.insert(target.clone());
        }
    }

    for (to_target, from_target) in &update.moved {
        let keep_to = keep_path(&to_target.path);
        let keep_from = keep_path(&from_target.path);

        match (keep_to, keep_from) {
            (true, true) => {
                filtered
                    .moved
                    .insert(to_target.clone(), from_target.clone());
            }
            (true, false) => {
                filtered.added.insert(to_target.clone());
            }
            (false, true) => {
                filtered.deleted.insert(from_target.clone());
            }
            (false, false) => {}
        }
    }

    (!filtered.is_empty()).then_some(filtered)
}

#[cfg(not(target_family = "wasm"))]
fn filesystem_event_to_repository_update(event: &BulkFilesystemWatcherEvent) -> RepositoryUpdate {
    RepositoryUpdate {
        added: event
            .added
            .iter()
            .cloned()
            .map(|path| TargetFile::new(path, false))
            .collect(),
        modified: event
            .modified
            .iter()
            .cloned()
            .map(|path| TargetFile::new(path, false))
            .collect(),
        deleted: event
            .deleted
            .iter()
            .cloned()
            .map(|path| TargetFile::new(path, false))
            .collect(),
        moved: event
            .moved
            .iter()
            .map(|(to_path, from_path)| {
                (
                    TargetFile::new(to_path.clone(), false),
                    TargetFile::new(from_path.clone(), false),
                )
            })
            .collect(),
        commit_updated: false,
        index_lock_detected: false,
    }
}

#[cfg(target_family = "wasm")]
#[allow(dead_code)]
pub(crate) enum WarpManagedPathsWatcherEvent {}

#[cfg(not(target_family = "wasm"))]
pub(crate) enum WarpManagedPathsWatcherEvent {
    FilesChanged(RepositoryUpdate),
}

#[cfg(not(target_family = "wasm"))]
pub(crate) struct WarpManagedPathsWatcher {
    _watcher: ModelHandle<BulkFilesystemWatcher>,
}

#[cfg(target_family = "wasm")]
pub(crate) struct WarpManagedPathsWatcher;

#[cfg(not(target_family = "wasm"))]
impl WarpManagedPathsWatcher {
    pub(crate) fn new(ctx: &mut ModelContext<Self>) -> Self {
        Self::new_internal(ctx, true)
    }

    #[cfg(test)]
    pub(crate) fn new_for_testing(ctx: &mut ModelContext<Self>) -> Self {
        Self::new_internal(ctx, false)
    }

    fn new_internal(ctx: &mut ModelContext<Self>, should_register_watcher: bool) -> Self {
        let watcher = if should_register_watcher {
            ctx.add_model(|ctx| {
                BulkFilesystemWatcher::new(
                    Duration::from_millis(WARP_MANAGED_PATHS_WATCHER_DEBOUNCE_MILLI_SECS),
                    ctx,
                )
            })
        } else {
            ctx.add_model(|_| BulkFilesystemWatcher::new_for_test())
        };
        ctx.subscribe_to_model(&watcher, Self::handle_fs_event);

        if should_register_watcher {
            let data_dir = warp_data_dir();
            let config_local_dir = warp_core::paths::config_local_dir();
            let should_register_config_local_dir = config_local_dir != data_dir;
            let worktrees_dir = data_dir.join("worktrees");
            Self::register_path(
                ctx,
                &watcher,
                data_dir.clone(),
                WatchFilter::with_filter(Arc::new(move |path| !path.starts_with(&worktrees_dir))),
                RecursiveMode::Recursive,
                "Warp data directory",
            );
            if should_register_config_local_dir {
                Self::register_path(
                    ctx,
                    &watcher,
                    config_local_dir.clone(),
                    WatchFilter::accept_all(),
                    RecursiveMode::Recursive,
                    "Warp config directory",
                );
            }
            if let Some(warp_home_skills_dir) = warp_home_skills_dir() {
                if warp_home_skills_dir.exists()
                    && !warp_home_skills_dir.starts_with(&data_dir)
                    && (!should_register_config_local_dir
                        || !warp_home_skills_dir.starts_with(&config_local_dir))
                {
                    Self::register_path(
                        ctx,
                        &watcher,
                        warp_home_skills_dir,
                        WatchFilter::accept_all(),
                        RecursiveMode::Recursive,
                        "Warp home skills directory",
                    );
                }
            }
            if let Some(claude_plugins) = claude_plugins_root() {
                if claude_plugins.exists()
                    && !claude_plugins.starts_with(&data_dir)
                    && (!should_register_config_local_dir
                        || !claude_plugins.starts_with(&config_local_dir))
                {
                    Self::register_path(
                        ctx,
                        &watcher,
                        claude_plugins,
                        WatchFilter::accept_all(),
                        RecursiveMode::Recursive,
                        "Claude plugins skills directory",
                    );
                }
            }
            if let (Some(warp_home_config_dir), Some(warp_home_mcp_config_path)) =
                (warp_home_config_dir(), warp_home_mcp_config_file_path())
            {
                if warp_home_config_dir.exists()
                    && !warp_home_config_dir.starts_with(&data_dir)
                    && (!should_register_config_local_dir
                        || !warp_home_config_dir.starts_with(&config_local_dir))
                {
                    Self::register_path(
                        ctx,
                        &watcher,
                        warp_home_config_dir,
                        WatchFilter::with_filter(Arc::new(move |path| {
                            path == warp_home_mcp_config_path
                        })),
                        RecursiveMode::NonRecursive,
                        "Warp home MCP config directory",
                    );
                }
            }
        }

        Self { _watcher: watcher }
    }

    fn register_path(
        ctx: &mut ModelContext<Self>,
        watcher: &ModelHandle<BulkFilesystemWatcher>,
        directory_path: PathBuf,
        watch_filter: WatchFilter,
        recursive_mode: RecursiveMode,
        description: &'static str,
    ) {
        let registration_path = directory_path.clone();
        let registration = watcher.update(ctx, |watcher, _ctx| {
            watcher.register_path(&registration_path, watch_filter, recursive_mode)
        });

        ctx.spawn(registration, move |_, result, _ctx| {
            if let Err(err) = result {
                log::warn!(
                    "Failed to start watching {description} {}: {err}",
                    directory_path.display()
                );
            }
        });
    }

    fn handle_fs_event(
        &mut self,
        event: &BulkFilesystemWatcherEvent,
        ctx: &mut ModelContext<Self>,
    ) {
        let update = filesystem_event_to_repository_update(event);
        if !update.is_empty() {
            ctx.emit(WarpManagedPathsWatcherEvent::FilesChanged(update));
        }
    }
}

#[cfg(target_family = "wasm")]
impl WarpManagedPathsWatcher {
    pub(crate) fn new(_ctx: &mut ModelContext<Self>) -> Self {
        Self
    }

    #[cfg(test)]
    pub(crate) fn new_for_testing(_ctx: &mut ModelContext<Self>) -> Self {
        Self
    }
}

impl Entity for WarpManagedPathsWatcher {
    type Event = WarpManagedPathsWatcherEvent;
}

impl SingletonEntity for WarpManagedPathsWatcher {}

#[cfg(test)]
mod tests {
    use dirs::home_dir;
    use std::collections::{HashMap, HashSet};
    use std::path::PathBuf;

    use repo_metadata::{RepositoryUpdate, TargetFile};

    use super::{
        claude_plugin_skill_dirs, filter_repository_update_by_prefix,
        warp_home_mcp_config_file_path, warp_home_skills_dir, warp_managed_mcp_config_path,
        warp_managed_skill_dirs, ClaudeSkillSourceFilter,
    };

    #[test]
    fn warp_managed_skill_dirs_contains_warp_home_and_claude_plugins() {
        let dirs = warp_managed_skill_dirs();
        if let Some(warp_home) = warp_home_skills_dir() {
            assert!(dirs.contains(&warp_home));
        }
        for claude_dir in claude_plugin_skill_dirs() {
            assert!(dirs.contains(&claude_dir));
        }
    }

    #[test]
    fn claude_plugin_skill_dirs_returns_valid_skill_directories() {
        for dir in claude_plugin_skill_dirs() {
            assert!(dir.is_dir(), "{} is not a directory", dir.display());
            assert_eq!(
                dir.file_name().and_then(|n| n.to_str()),
                Some("skills"),
                "{} should be named 'skills'",
                dir.display()
            );
        }
    }

    #[test]
    fn filter_empty_config_allows_all() {
        let filter = ClaudeSkillSourceFilter::parse("");
        let root = PathBuf::from("/home/user/.claude/plugins");
        let dir = root.join("cache/bazaar/godmode/0.6.0/skills");
        assert!(filter.is_allowed(&dir, &root));
    }

    #[test]
    fn filter_include_only_allows_matching() {
        let filter =
            ClaudeSkillSourceFilter::parse(r#"include = ["cache/bazaar/godmode", "hand"]"#);
        let root = PathBuf::from("/home/user/.claude/plugins");
        assert!(filter.is_allowed(&root.join("cache/bazaar/godmode/0.6.0/skills"), &root));
        assert!(filter.is_allowed(&root.join("hand/skills"), &root));
        assert!(!filter.is_allowed(&root.join("cache/bazaar/atelier/1.0/skills"), &root));
    }

    #[test]
    fn filter_exclude_takes_precedence() {
        let filter = ClaudeSkillSourceFilter::parse(
            r#"
include = ["cache/bazaar"]
exclude = ["cache/bazaar/godmode"]
"#,
        );
        let root = PathBuf::from("/home/user/.claude/plugins");
        assert!(filter.is_allowed(&root.join("cache/bazaar/atelier/1.0/skills"), &root));
        assert!(!filter.is_allowed(&root.join("cache/bazaar/godmode/0.6.0/skills"), &root));
    }

    #[test]
    fn filter_exclude_only_blocks_matching() {
        let filter =
            ClaudeSkillSourceFilter::parse(r#"exclude = ["marketplaces/langchain-skills"]"#);
        let root = PathBuf::from("/home/user/.claude/plugins");
        assert!(filter.is_allowed(&root.join("cache/bazaar/godmode/0.6.0/skills"), &root));
        assert!(!filter.is_allowed(&root.join("marketplaces/langchain-skills/skills"), &root));
    }

    #[test]
    fn warp_managed_mcp_config_path_contains_only_warp_home_path() {
        match (
            home_dir(),
            warp_home_mcp_config_file_path(),
            warp_managed_mcp_config_path(),
        ) {
            (Some(home_dir), Some(warp_home_mcp_config_path), Some(path)) => {
                assert_eq!(path.root_path, home_dir);
                assert_eq!(path.config_path, warp_home_mcp_config_path);
            }
            (_, _, None) => {}
            _ => panic!("Expected Warp MCP path when home directory is available"),
        }
    }

    #[test]
    fn filter_repository_update_by_prefix_keeps_only_matching_paths() {
        let skills_dir = PathBuf::from("/tmp/.warp-local/skills");
        let other_dir = PathBuf::from("/tmp/.warp-local/worktrees/repo");
        let skill_file = skills_dir.join("deploy").join("SKILL.md");
        let other_file = other_dir.join("README.md");

        let update = RepositoryUpdate {
            added: HashSet::from([
                TargetFile::new(skill_file.clone(), false),
                TargetFile::new(other_file.clone(), false),
            ]),
            modified: HashSet::new(),
            deleted: HashSet::new(),
            moved: HashMap::new(),
            commit_updated: false,
            index_lock_detected: false,
        };

        let filtered =
            filter_repository_update_by_prefix(&update, &skills_dir).expect("expected update");

        assert!(filtered.contains_added_or_modified(&TargetFile::new(skill_file, false)));
        assert!(!filtered.contains_added_or_modified(&TargetFile::new(other_file, false)));
    }

    #[test]
    fn filter_repository_update_by_prefix_converts_cross_boundary_moves() {
        let skills_dir = PathBuf::from("/tmp/.warp-local/skills");
        let skill_file = skills_dir.join("deploy").join("SKILL.md");
        let ignored_file = PathBuf::from("/tmp/.warp-local/worktrees/repo/SKILL.md");

        let update = RepositoryUpdate {
            added: HashSet::new(),
            modified: HashSet::new(),
            deleted: HashSet::new(),
            moved: HashMap::from([(
                TargetFile::new(skill_file.clone(), false),
                TargetFile::new(ignored_file, false),
            )]),
            commit_updated: false,
            index_lock_detected: false,
        };

        let filtered =
            filter_repository_update_by_prefix(&update, &skills_dir).expect("expected update");

        assert!(filtered.contains_added_or_modified(&TargetFile::new(skill_file, false)));
        assert!(filtered.moved.is_empty());
        assert!(filtered.deleted.is_empty());
    }
}
