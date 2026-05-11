# Handup Panel — Tech Spec

## Context

The Handoff panel (`app/src/joe/handoff/`) reads `.ctx/HANDOFF.*.yaml` files
and renders them in the left sidebar. The Handup panel replicates this
pattern but reads from a SQLite database (`~/.ctx/handoffs/handup.db`) that
stores cross-project checkpoint snapshots produced by the `handup` skill.

Database schema (single table):

```sql
CREATE TABLE checkpoints (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  project TEXT NOT NULL,
  cwd TEXT NOT NULL,
  generated TEXT NOT NULL,       -- date string, e.g. "2026-05-11"
  recommendation TEXT,           -- freeform text summary
  json_path TEXT NOT NULL,       -- path to source HANDUP.json
  created_at TEXT DEFAULT (datetime('now'))
);
```

`rusqlite` (bundled) is already in `app/Cargo.toml:162`.

### Relevant files

- `app/src/joe/handoff/model.rs` — `HandoffModel`, `HandoffItem`,
  `LoadState`, spawn-based async load pattern
- `app/src/joe/handoff/panel.rs` — `HandoffPanel`, `View` impl,
  section/project grouping, expand/collapse
- `app/src/joe/mod.rs:6-11` — joe module declarations
- `app/src/workspace/view/left_panel.rs:107-115` —
  `ToolPanelView` enum (tab routing)
- `app/src/workspace/view/left_panel.rs:175-191` —
  `LeftPanelView` struct (view handles)
- `app/src/workspace/view/left_panel.rs:226-230` —
  panel construction in `new()`
- `app/src/workspace/view/left_panel.rs:1232-1238` —
  panel rendering by active view
- `app/src/app_state.rs:298-306` — `LeftPanelDisplayedTab` enum
- `app/src/workspace/view.rs:3768-3776` — tab-to-view mapping

See `PRODUCT.md` for all user-facing behavior (invariants 1-17).

## Proposed changes

### 1. New module: `app/src/joe/handup/`

Three files mirroring the handoff module structure:

**`mod.rs`**

```rust
pub mod model;
pub mod panel;
```

**`model.rs`** — `HandupModel`

```rust
pub struct HandupCheckpoint {
    pub id: i64,
    pub project: String,
    pub cwd: String,
    pub generated: String,        // date string
    pub recommendation: Option<String>,
    pub json_path: String,
    pub created_at: String,
}

pub struct HandupModel {
    pub checkpoints: Vec<HandupCheckpoint>,
    pub load_state: LoadState,    // reuse from handoff::model
}
```

`load()` follows the same `ctx.spawn()` pattern as `HandoffModel::load`:

- Resolve DB path: `dirs::home_dir()?.join(".ctx/handoffs/handup.db")`
- Open with `rusqlite::Connection::open_with_flags(READ_ONLY)`
- `SELECT id, project, cwd, generated, recommendation, json_path,
created_at FROM checkpoints ORDER BY created_at DESC`
- Map rows to `Vec<HandupCheckpoint>`
- On missing file or schema error, return `Err(message)` (invariants 4, 17)

No `LoadResult` wrapper needed; the model stores checkpoints directly.
Reuse `LoadState` from `handoff::model` (or duplicate the 4-variant enum
locally to avoid coupling).

**`panel.rs`** — `HandupPanel`

Struct fields:

```rust
pub struct HandupPanel {
    model: ModelHandle<HandupModel>,
    expanded_projects: HashSet<String>,  // project-level toggle
    expanded_rows: HashSet<i64>,         // row-level toggle
    refresh_mouse_state: MouseStateHandle,
    scroll_state: ClippedScrollStateHandle,
}
```

Rendering (maps to PRODUCT.md invariants):

- Header: "Handup" + count subtitle + refresh button (inv. 1, 2)
- Loading/error/empty states (inv. 3, 4, 5)
- Group by `project`, sorted alphabetically (inv. 6, 7)
- Default: show only latest checkpoint per project (inv. 8)
- Date badge in priority-badge position + truncated recommendation
  (inv. 9)
- Click row: expand to full recommendation + cwd (inv. 10)
- Click project header: toggle group expansion (inv. 11)
- Scrollable body (inv. 12)
- Theme-derived styling (inv. 13)

Actions:

```rust
pub enum HandupPanelAction {
    Refresh,
    ToggleProject(String),
    ToggleRow(i64),
}
```

### 2. Wire into joe module

`app/src/joe/mod.rs` — add `pub mod handup;`

### 3. Wire into left panel

**`app/src/workspace/view/left_panel.rs`:**

- Add `use crate::joe::handup::panel::HandupPanel;`
- Add `Handup` variant to `ToolPanelView` enum (line 114)
- Add `handup_panel: ViewHandle<HandupPanel>` to `LeftPanelView`
- Construct in `new()` alongside other panels
- Add render arm matching `ToolPanelView::Handup`
- Add focus arm

**`app/src/app_state.rs`:**

- Add `Handup` variant to `LeftPanelDisplayedTab` (line 305)
- Add conversion arm in `From<ToolPanelView>`

**`app/src/workspace/view.rs`:**

- Add `LeftPanelDisplayedTab::Handup => ToolPanelView::Handup` mapping
  (around line 3776)
- Add display name `"Handup"` in the two tooltip/label sites

### 4. Toolbelt button (optional, deferred)

Adding a toolbelt icon for the Handup tab can be done as a follow-up.
The panel is accessible via the `LeftPanelDisplayedTab` mechanism even
without a dedicated button.

## Testing and validation

### Unit tests (`app/src/joe/handup/model_tests.rs`)

| Test                     | Covers                                                                                                        |
| ------------------------ | ------------------------------------------------------------------------------------------------------------- | --------- |
| `load_from_populated_db` | Create temp DB, insert 3 rows across 2 projects, verify `load()` returns all rows sorted by `created_at` DESC | inv. 3, 7 |
| `load_empty_db`          | Create temp DB with empty `checkpoints` table, verify empty vec                                               | inv. 5    |
| `load_missing_db`        | Point at non-existent path, verify `Err` with message                                                         | inv. 4    |
| `load_bad_schema`        | Create temp DB without `checkpoints` table, verify `Err`                                                      | inv. 17   |
| `null_recommendation`    | Insert row with NULL recommendation, verify `None`                                                            | inv. 9    |

### Manual verification

- Build and run `cargo run`
- Switch left panel to Handup tab
- Verify checkpoint list grouped by project
- Click a project header to expand/collapse group
- Click a row to see full recommendation + cwd
- Click refresh button to reload
- Rename `handup.db` to verify error state
- Verify scrolling with many checkpoints

### Not tested

- Panel rendering (requires full UI test harness, not available for
  joe-mode panels)
- Theme color correctness (visual inspection only)
