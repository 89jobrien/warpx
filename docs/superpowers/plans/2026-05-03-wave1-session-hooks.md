# Wave 1 — Session Lifecycle Hooks (handon / handoff)

> **Status: DONE** — `spawn_handon()` called from `BackingView::set_focus_handle`
> (tab open) and `spawn_handoff()` from `BackingView::close` (tab close) in
> `app/src/terminal/view/pane_impl.rs`. Both gated by `FeatureFlag::OzSessionHooks`.

> **For agentic workers:** Run `/godmode:tackle-issues` to dispatch parallel
> subagents per task group. Use `/godmode:test-driven-development` for each
> implementation step. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire warpx tab/session lifecycle events so that opening a tab
runs `godmode handon` and closing a tab runs `godmode handoff` — surfacing
outstanding work at session start and writing a handoff on exit.

**Architecture:** Find the existing session/tab lifecycle event hooks in
`app/src/`. Add fire-and-forget async spawns for `godmode handon` (open)
and `godmode handoff` (close). Gate under `FeatureFlag::OzSessionHooks` so
it only runs in dogfood/local builds.

**Tech Stack:** Rust, WarpUI ModelContext/spawn, std::process::Command

**Repo:** `/Users/joe/dev/warpx`

**GitHub issue:** #4

**Dependencies:** `godmode` CLI on PATH; `OzSessionHooks` feature flag.

---

## Task 1: Discover existing session lifecycle wiring

**Files:**

- `app/src/` — search for tab open/close event handling

- [ ] **Step 1: Find session/tab event hooks**

Search for terms like `session_start`, `tab_open`, `tab_close`, `on_open`,
`workspace_created`, `new_tab` across `app/src/` to find where tab
lifecycle events fire.

```
Grep pattern: session|tab_open|tab_close|WorkspaceCreated|new_tab
Path: app/src/
```

- [ ] **Step 2: Find the shell spawn pattern**

Search for existing `std::process::Command` or `ctx.spawn` usages that
shell out to external CLIs (e.g. how `HandoffModel::load` works) — use
that pattern for hook spawning.

- [ ] **Step 3: Document findings**

Note the exact event name(s), file(s), and line(s) where tab open and
close events fire. This determines where hooks are wired in Task 3.

---

## Task 2: Feature flag

**Files:**

- `crates/warp_features/src/lib.rs`

- [ ] **Step 1: Add `OzSessionHooks` flag**

Read `crates/warp_features/src/lib.rs`. Add `FeatureFlag::OzSessionHooks`
to `DOGFOOD_FLAGS` following the exact style of adjacent entries.

- [ ] **Step 2: Cargo check**

```bash
cargo check --workspace --manifest-path /Users/joe/dev/warpx/Cargo.toml
```

- [ ] **Step 3: Commit**

```bash
git -C /Users/joe/dev/warpx add crates/warp_features/src/lib.rs
git -C /Users/joe/dev/warpx commit -m "feat: add OzSessionHooks feature flag"
```

---

## Task 3: Implement handon hook (tab open)

**Files:**

- File(s) identified in Task 1 Step 3

- [ ] **Step 1: Add handon spawn on tab open**

At the tab-open event site, add (gated on the flag):

```rust
if FeatureFlag::OzSessionHooks.is_enabled() {
    ctx.spawn(
        async move {
            let _ = std::process::Command::new("godmode")
                .arg("handon")
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn();
        },
        |_, _, _| {},
    );
}
```

The spawn is fire-and-forget — never block the UI thread. The `handon`
output will appear in the terminal tab's shell naturally via the session
startup.

- [ ] **Step 2: Cargo check**

```bash
cargo check --workspace --manifest-path /Users/joe/dev/warpx/Cargo.toml
```

---

## Task 4: Implement handoff hook (tab close)

**Files:**

- File(s) identified in Task 1 Step 3

- [ ] **Step 1: Add handoff spawn on tab close**

At the tab-close event site, add (gated on the flag):

```rust
if FeatureFlag::OzSessionHooks.is_enabled() {
    // Best-effort — do not block close
    let _ = std::process::Command::new("godmode")
        .arg("handoff")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
}
```

Note: tab close may not have an async context. Use synchronous
`Command::spawn()` (non-blocking) rather than `ctx.spawn`. Confirm the
event site's context type in Task 1 before choosing the pattern.

- [ ] **Step 2: Cargo check**

```bash
cargo check --workspace --manifest-path /Users/joe/dev/warpx/Cargo.toml
```

---

## Task 5: Commit and verify

- [ ] **Step 1: Run clippy**

```bash
cargo clippy --workspace --manifest-path /Users/joe/dev/warpx/Cargo.toml \
  -- -D warnings
```

Fix all warnings before committing.

- [ ] **Step 2: Commit**

```bash
git -C /Users/joe/dev/warpx add -A
git -C /Users/joe/dev/warpx commit -m \
  "feat: session lifecycle hooks — handon on tab open, handoff on close closes #4"
```

- [ ] **Step 3: Manual verification**

Build and run warpx:

```bash
cargo run --manifest-path /Users/joe/dev/warpx/Cargo.toml --bin warpx
```

- Open a new tab in a repo with a HANDOFF file — verify `godmode handon`
  fires (check `~/.ctx/chats/` or godmode log for evidence).
- Close the tab — verify `godmode handoff` fires and a HANDOFF entry is
  written or updated.

---

## Verification Checklist

- [ ] `OzSessionHooks` flag present in `DOGFOOD_FLAGS`
- [ ] Tab open → `godmode handon` fires (non-blocking, best-effort)
- [ ] Tab close → `godmode handoff` fires (non-blocking, best-effort)
- [ ] No UI freeze or delay on open/close
- [ ] `cargo clippy --workspace -- -D warnings` — zero warnings
- [ ] `cargo nextest run --workspace` — no regressions

## Phase Note

If no clean tab-close event exists (similar to Nu's missing pre_exit hook),
document the blocker and defer the handoff hook to a follow-up. The handon
hook (open) is always feasible and should ship regardless.
