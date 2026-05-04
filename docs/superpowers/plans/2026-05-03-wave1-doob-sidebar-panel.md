# Wave 1 — Doob Sidebar Panel in warpx

> **For agentic workers:** Run `/godmode:tackle-issues` to dispatch parallel
> subagents per task group. Use `/godmode:test-driven-development` for each
> implementation step. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a read-only Doob task panel to the warpx left sidebar showing
pending/in-progress/done todos sourced from `doob todo list --json`.

**Architecture:** Mirror the `HandoffPanel` pattern exactly —
`app/src/doob/model.rs` shells out to `doob`, `app/src/doob/panel.rs`
renders the list, `left_panel.rs` registers it under a dogfood feature flag.

**Tech Stack:** Rust, WarpUI Entity-Component-Handle, `std::process::Command`

**Repo:** `/Users/joe/dev/warpx`

**GitHub issue:** #2

**Dependencies:** `doob` on PATH; Phase 1 MCP tools live (for future
enhancement); `HandoffPanel` committed (reference impl).

---

## Task 1: Feature flag

**Files:**

- `crates/warp_features/src/lib.rs`

- [ ] **Step 1: Read feature flag file**

Read `crates/warp_features/src/lib.rs`. Identify the `DOGFOOD_FLAGS` array
and the pattern used for other oz-prefixed flags.

- [ ] **Step 2: Add `OzDoobPanel` flag**

Add `FeatureFlag::OzDoobPanel` to `DOGFOOD_FLAGS`. Follow exact style of
adjacent entries.

- [ ] **Step 3: Cargo check**

```bash
cargo check --workspace --manifest-path /Users/joe/dev/warpx/Cargo.toml
```

- [ ] **Step 4: Commit**

```bash
git -C /Users/joe/dev/warpx add crates/warp_features/src/lib.rs
git -C /Users/joe/dev/warpx commit -m "feat: add OzDoobPanel feature flag to DOGFOOD_FLAGS"
```

---

## Task 2: DoobModel — shell out to doob CLI

**Files:**

- `app/src/doob/mod.rs` (new)
- `app/src/doob/model.rs` (new)

- [ ] **Step 1: Create module**

Create `app/src/doob/mod.rs`:

```rust
pub mod model;
pub mod panel;
```

Register in `app/src/lib.rs` (or equivalent module root) following the
pattern used for `handoff`.

- [ ] **Step 2: Write DoobItem and DoobModel**

Create `app/src/doob/model.rs`:

```rust
use std::process::Command;
use warpui::{Entity, ModelContext};

#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
pub struct DoobItem {
    pub id: String,
    pub title: String,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub project: Option<String>,
    pub due: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum LoadState {
    #[default]
    NotLoaded,
    Loading,
    Loaded,
    Error(String),
}

pub struct DoobModel {
    pub items: Vec<DoobItem>,
    pub load_state: LoadState,
}

impl DoobModel {
    pub fn new(_ctx: &mut ModelContext<Self>) -> Self {
        Self { items: Vec::new(), load_state: LoadState::NotLoaded }
    }

    pub fn load_from_cli() -> Result<Vec<DoobItem>, String> {
        let out = Command::new("doob")
            .args(["todo", "list", "--json"])
            .output()
            .map_err(|e| format!("doob not found: {e}"))?;
        if !out.status.success() {
            return Err(String::from_utf8_lossy(&out.stderr).into_owned());
        }
        let items: Vec<DoobItem> = serde_json::from_slice(&out.stdout)
            .map_err(|e| format!("parse error: {e}"))?;
        Ok(items)
    }

    pub fn load(&mut self, ctx: &mut ModelContext<Self>) {
        self.load_state = LoadState::Loading;
        ctx.notify();
        ctx.spawn(
            async move { DoobModel::load_from_cli() },
            |me, result, ctx| match result {
                Ok(items) => {
                    me.items = items;
                    me.load_state = LoadState::Loaded;
                    ctx.emit(DoobModelEvent::Loaded);
                    ctx.notify();
                }
                Err(e) => {
                    me.load_state = LoadState::Error(e.clone());
                    ctx.emit(DoobModelEvent::Error(e));
                    ctx.notify();
                }
            },
        );
    }
}

pub struct LoadResult {
    pub items: Vec<DoobItem>,
}

impl Entity for DoobModel {
    type Event = DoobModelEvent;
}

#[derive(Clone, Debug)]
pub enum DoobModelEvent {
    Loaded,
    #[allow(dead_code)]
    Error(String),
}
```

- [ ] **Step 3: Unit tests**

Add `#[cfg(test)] mod tests` in `model.rs`. At minimum:

- `doob_item_derives_clone_debug_partialeq` — construct a `DoobItem`,
  clone it, assert eq.

- [ ] **Step 4: Cargo check + nextest**

```bash
cargo check --workspace --manifest-path /Users/joe/dev/warpx/Cargo.toml
cargo nextest run -p app -- doob 2>/dev/null || true
```

---

## Task 3: DoobPanel — render the list

**Files:**

- `app/src/doob/panel.rs` (new)

- [ ] **Step 1: Write DoobPanel**

Model after `HandoffPanel`. Key differences:

- Groups: `pending` / `in_progress` / `done` (map doob status values)
- No priority badge if `priority` is absent
- Project sub-header uses `item.project` directly (no filename parsing)

Structure:

```rust
pub struct DoobPanel {
    model: warpui::ModelHandle<DoobModel>,
    expanded: HashSet<String>,
    refresh_mouse_state: MouseStateHandle,
    scroll_state: ClippedScrollStateHandle,
}

pub enum DoobPanelAction {
    Refresh,
    ToggleExpand(String),
}
```

Render layout: fixed header ("Doob" + item count + refresh btn), body
wrapped in `ClippedScrollable::vertical`.

- [ ] **Step 2: Cargo check**

```bash
cargo check --workspace --manifest-path /Users/joe/dev/warpx/Cargo.toml
```

---

## Task 4: Wire into left_panel.rs

**Files:**

- `app/src/workspace/view/left_panel.rs`

- [ ] **Step 1: Read left_panel.rs**

Search for `HandoffPanel` usage to find the exact registration pattern —
how the view is created, stored, and rendered behind its feature flag.

- [ ] **Step 2: Add DoobPanel registration**

Mirror the HandoffPanel registration:

- Import `crate::doob::panel::DoobPanel`
- Add `doob_panel: Option<ViewHandle<DoobPanel>>` field
- Initialize under `FeatureFlag::OzDoobPanel.is_enabled()`
- Render in sidebar tab list

- [ ] **Step 3: Cargo check**

```bash
cargo check --workspace --manifest-path /Users/joe/dev/warpx/Cargo.toml
```

- [ ] **Step 4: Commit**

```bash
git -C /Users/joe/dev/warpx add app/src/doob/ \
  app/src/workspace/view/left_panel.rs
git -C /Users/joe/dev/warpx commit -m \
  "feat: add Doob sidebar panel (OzDoobPanel dogfood flag) closes #2"
```

---

## Verification Checklist

- [ ] `cargo run` — warpx launches, Doob tab visible in left sidebar
- [ ] Panel shows pending items grouped by project
- [ ] Refresh button re-runs `doob todo list --json`
- [ ] Expanding an item shows its description
- [ ] Panel scrolls when item count overflows
- [ ] `cargo nextest run -p app -- doob` — all tests green
- [ ] `cargo clippy --workspace -- -D warnings` — zero warnings
