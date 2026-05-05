use super::{FileBasedMCPManager, FileBasedMCPManagerEvent, MCPProvider};
use crate::ai::mcp::FileMCPWatcher;
use crate::ai::mcp::ParsedTemplatableMCPServerResult;
use crate::auth::AuthStateProvider;
use crate::settings::{AISettings, FocusedTerminalInfo};
use crate::warp_managed_paths_watcher::{warp_data_dir, WarpManagedPathsWatcher};
use crate::workspaces::user_workspaces::UserWorkspaces;
use repo_metadata::{
    repositories::DetectedRepositories, watcher::DirectoryWatcher, RepoMetadataModel,
};
use std::path::PathBuf;
use warp_core::features::FeatureFlag;
use warpui::{App, Entity, ModelHandle};
use watcher::HomeDirectoryWatcher;

fn setup_app(app: &mut App) -> ModelHandle<FileBasedMCPManager> {
    app.add_singleton_model(DirectoryWatcher::new);
    app.add_singleton_model(|_| DetectedRepositories::default());
    app.add_singleton_model(RepoMetadataModel::new);
    app.add_singleton_model(HomeDirectoryWatcher::new_for_test);
    app.add_singleton_model(WarpManagedPathsWatcher::new_for_testing);
    app.add_singleton_model(FileMCPWatcher::new);
    app.add_singleton_model(AISettings::new_with_defaults);
    app.add_singleton_model(|_| AuthStateProvider::new_for_test());
    app.add_singleton_model(UserWorkspaces::default_mock);
    app.add_singleton_model(FocusedTerminalInfo::new);
    app.add_singleton_model(FileBasedMCPManager::new)
}

fn parse_mcp_json(json: &str) -> Vec<ParsedTemplatableMCPServerResult> {
    ParsedTemplatableMCPServerResult::from_user_json(json).unwrap_or_default()
}

/// Collects `McpConfigReloaded` events emitted by `FileBasedMCPManager`.
#[derive(Default)]
struct HotReloadEvents {
    reloaded: Vec<usize>,
}

impl Entity for HotReloadEvents {
    type Event = ();
}

fn subscribe_hot_reload_events(
    app: &mut App,
    manager: &ModelHandle<FileBasedMCPManager>,
) -> ModelHandle<HotReloadEvents> {
    let events = app.add_model(|_| HotReloadEvents::default());
    events.update(app, |_, ctx| {
        ctx.subscribe_to_model(manager, |me, event, _| {
            if let FileBasedMCPManagerEvent::McpConfigReloaded { restarted } = event {
                me.reloaded.push(*restarted);
            }
        });
    });
    events
}

const SERVER_A_JSON: &str = r#"{"mcpServers":{"server-a":{"command":"npx","args":["server-a"]}}}"#;
const SERVER_B_JSON: &str = r#"{"mcpServers":{"server-b":{"command":"npx","args":["server-b"]}}}"#;
const EMPTY_JSON: &str = r#"{"mcpServers":{}}"#;

/// Initial load must NOT emit `McpConfigReloaded` even with the flag enabled.
#[test]
fn initial_load_does_not_emit_hot_reload_event() {
    let _flag = FeatureFlag::OzMcpHotReload.override_enabled(true);
    let _file_based = FeatureFlag::FileBasedMcp.override_enabled(true);

    App::test((), |mut app| async move {
        let manager = setup_app(&mut app);
        let events = subscribe_hot_reload_events(&mut app, &manager);

        let root = warp_data_dir();
        manager.update(&mut app, |me, ctx| {
            me.apply_parsed_servers(root, MCPProvider::Warp, parse_mcp_json(SERVER_A_JSON), ctx);
        });

        events.read(&app, |e, _| {
            assert!(
                e.reloaded.is_empty(),
                "initial load must not emit McpConfigReloaded"
            );
        });
    });
}

/// A second call (reload) for the same slot must emit `McpConfigReloaded` with the
/// correct count when the flag is enabled.
#[test]
fn reload_emits_event_with_changed_count() {
    let _flag = FeatureFlag::OzMcpHotReload.override_enabled(true);
    let _file_based = FeatureFlag::FileBasedMcp.override_enabled(true);

    App::test((), |mut app| async move {
        let manager = setup_app(&mut app);
        let events = subscribe_hot_reload_events(&mut app, &manager);

        let root = warp_data_dir();

        // Initial load — no event expected.
        manager.update(&mut app, |me, ctx| {
            me.apply_parsed_servers(
                root.clone(),
                MCPProvider::Warp,
                parse_mcp_json(SERVER_A_JSON),
                ctx,
            );
        });

        // Reload with a different server — server-a is removed, server-b is added → restarted = 2.
        manager.update(&mut app, |me, ctx| {
            me.apply_parsed_servers(
                root.clone(),
                MCPProvider::Warp,
                parse_mcp_json(SERVER_B_JSON),
                ctx,
            );
        });

        events.read(&app, |e, _| {
            assert_eq!(
                e.reloaded.len(),
                1,
                "expected exactly one McpConfigReloaded event"
            );
            // 1 removed (server-a) + 1 added (server-b) = 2
            assert_eq!(
                e.reloaded[0], 2,
                "restarted count should be 2 (1 removed + 1 added)"
            );
        });
    });
}

/// When the flag is disabled, reloads must NOT emit `McpConfigReloaded`.
#[test]
fn reload_does_not_emit_event_when_flag_disabled() {
    let _flag = FeatureFlag::OzMcpHotReload.override_enabled(false);
    let _file_based = FeatureFlag::FileBasedMcp.override_enabled(true);

    App::test((), |mut app| async move {
        let manager = setup_app(&mut app);
        let events = subscribe_hot_reload_events(&mut app, &manager);

        let root = warp_data_dir();

        manager.update(&mut app, |me, ctx| {
            me.apply_parsed_servers(
                root.clone(),
                MCPProvider::Warp,
                parse_mcp_json(SERVER_A_JSON),
                ctx,
            );
        });

        // Reload — flag disabled, so no event.
        manager.update(&mut app, |me, ctx| {
            me.apply_parsed_servers(
                root.clone(),
                MCPProvider::Warp,
                parse_mcp_json(SERVER_B_JSON),
                ctx,
            );
        });

        events.read(&app, |e, _| {
            assert!(
                e.reloaded.is_empty(),
                "McpConfigReloaded must not emit when OzMcpHotReload flag is off"
            );
        });
    });
}

/// Reloading with no actual changes emits `McpConfigReloaded { restarted: 0 }`.
#[test]
fn reload_with_no_changes_emits_restarted_zero() {
    let _flag = FeatureFlag::OzMcpHotReload.override_enabled(true);
    let _file_based = FeatureFlag::FileBasedMcp.override_enabled(true);

    App::test((), |mut app| async move {
        let manager = setup_app(&mut app);
        let events = subscribe_hot_reload_events(&mut app, &manager);

        let root = warp_data_dir();

        // Initial load.
        manager.update(&mut app, |me, ctx| {
            me.apply_parsed_servers(
                root.clone(),
                MCPProvider::Warp,
                parse_mcp_json(SERVER_A_JSON),
                ctx,
            );
        });

        // Reload with identical content — server-a is already tracked, nothing changes.
        manager.update(&mut app, |me, ctx| {
            me.apply_parsed_servers(
                root.clone(),
                MCPProvider::Warp,
                parse_mcp_json(SERVER_A_JSON),
                ctx,
            );
        });

        events.read(&app, |e, _| {
            assert_eq!(e.reloaded.len(), 1, "expected one McpConfigReloaded event");
            assert_eq!(
                e.reloaded[0], 0,
                "restarted count should be 0 when no servers changed"
            );
        });
    });
}

/// Reloading with an empty config emits `McpConfigReloaded` with count equal to
/// number of servers that were removed.
#[test]
fn reload_with_empty_config_reports_removed_count() {
    let _flag = FeatureFlag::OzMcpHotReload.override_enabled(true);
    let _file_based = FeatureFlag::FileBasedMcp.override_enabled(true);

    App::test((), |mut app| async move {
        let manager = setup_app(&mut app);
        let events = subscribe_hot_reload_events(&mut app, &manager);

        let root: PathBuf = warp_data_dir();

        // Initial load with one server.
        manager.update(&mut app, |me, ctx| {
            me.apply_parsed_servers(
                root.clone(),
                MCPProvider::Warp,
                parse_mcp_json(SERVER_A_JSON),
                ctx,
            );
        });

        // Reload with empty config — server-a should be removed.
        manager.update(&mut app, |me, ctx| {
            me.apply_parsed_servers(
                root.clone(),
                MCPProvider::Warp,
                parse_mcp_json(EMPTY_JSON),
                ctx,
            );
        });

        events.read(&app, |e, _| {
            assert_eq!(e.reloaded.len(), 1, "expected one McpConfigReloaded event");
            assert_eq!(
                e.reloaded[0], 1,
                "restarted count should be 1 (server-a removed)"
            );
        });
    });
}
