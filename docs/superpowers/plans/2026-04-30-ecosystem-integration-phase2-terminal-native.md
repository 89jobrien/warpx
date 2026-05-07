# Ecosystem Integration Phase 2: Terminal-Native

> **Status: DONE** — Implemented 2026-04-30. Commits: `461bee6`, `03a55a9`,
> `a90938d` in `dotfiles` and `doob` repos.

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps
> use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire ecosystem awareness into the terminal shell layer — doob
status cache, Starship prompt module, Nu directory-change hook, and
session lifecycle hooks (handon on shell open).

**Architecture:** Three independent deliverables that compose:

1. `doob` CLI writes `~/.cache/doob/status.json` on every mutation —
   lightweight JSON file read by Starship and Nu hooks without shelling
   out.
2. Starship custom module reads the cache and renders `[doob: N overdue]`
   in yellow (1–3) or red (>3) when count > 0.
3. Nu `env_change.PWD` hook reads the cache on `cd` and prints overdue
   count for the current repo.
4. Nu `env.nu` fires `handoff-detect` on shell start as a handon banner.

**Dependencies:** Phase 1 MCP tools are live (verified 2026-04-30).
`doob`, `handoff-detect`, `starship`, and `nu` must be on PATH.

**Repos touched:**

- `~/dev/doob` — status cache write on mutation
- `~/dev/dotfiles` — Starship config + Nu hooks + env.nu

---

## Task 1: doob Status Cache

**Goal:** `doob` CLI writes `~/.cache/doob/status.json` after every
mutation so downstream consumers (Starship, Nu hooks) can read it without
shelling out.

**Files:**

- Create: `src/cache.rs` in the doob crate
- Modify: mutation command handlers (complete, add, set-due, set-priority)

**Schema:**

```json
{
  "updated_at": "2026-04-30T22:00:00Z",
  "pending_total": 12,
  "overdue_total": 3,
  "overdue_by_repo": {
    "warpx": 2,
    "minibox": 1
  }
}
```

- [ ] **Step 1: Read doob mutation entry points**

Read `~/dev/doob/src/` to identify which commands mutate state (complete,
add, set-due, set-priority). List the files that handle each before
editing.

- [ ] **Step 2: Create cache module**

Create `src/cache.rs`:

```rust
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use serde::Serialize;

#[derive(Serialize)]
pub struct StatusCache {
    pub updated_at: String,
    pub pending_total: usize,
    pub overdue_total: usize,
    pub overdue_by_repo: HashMap<String, usize>,
}

pub fn cache_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(home).join(".cache/doob/status.json")
}

pub fn write_status_cache(cache: &StatusCache) -> anyhow::Result<()> {
    let path = cache_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(cache)?;
    fs::write(&path, json)?;
    Ok(())
}
```

- [ ] **Step 3: Wire cache write after mutations**

After each successful mutation (complete, add), call:

```rust
let cache = build_status_cache(&db)?; // query pending/overdue counts
let _ = cache::write_status_cache(&cache); // best-effort, never fatal
```

Cache writes must be non-fatal — a failure must never fail the CLI
command.

- [ ] **Step 4: Manual smoke test**

```bash
doob todo add "test cache write" --repo warpx
cat ~/.cache/doob/status.json
```

Expected: valid JSON with `updated_at`, `pending_total`, `overdue_total`.

- [ ] **Step 5: Cargo check + test**

```bash
cargo check -p doob && cargo nextest run -p doob cache
```

- [ ] **Step 6: Commit**

```bash
git -C ~/dev/doob add src/cache.rs src/
git -C ~/dev/doob commit -m "feat: write ~/.cache/doob/status.json on every mutation"
```

---

## Task 2: Starship Custom Module

**Goal:** Render `[doob: N overdue]` in yellow (1–3) or red (>3). Hidden
when overdue count is 0.

**Files:**

- Modify: `~/dotfiles/starship.toml`

**Note:** Starship does not support dynamic style from command output, so
two modules are used — one for the amber range, one for red.

- [ ] **Step 1: Locate starship.toml**

Verify `~/dotfiles/starship.toml` exists and check its current `format`
line.

- [ ] **Step 2: Add custom doob modules**

Append to `~/dotfiles/starship.toml`:

```toml
[custom.doob]
description = "Show overdue doob todos (1-3, yellow)"
command = """
file="$HOME/.cache/doob/status.json"
[ -f "$file" ] || exit 0
count=$(python3 -c "
import json, sys
d = json.load(open('$file'))
c = d.get('overdue_total', 0)
print(c) if 0 < c <= 3 else sys.exit(1)
" 2>/dev/null) || exit 0
echo "doob: $count overdue"
"""
when = "test -f $HOME/.cache/doob/status.json"
format = "[$output]($style) "
style = "yellow"
shell = ["sh", "-c"]

[custom.doob_urgent]
description = "Show urgent overdue doob todos (>3, red)"
command = """
file="$HOME/.cache/doob/status.json"
[ -f "$file" ] || exit 0
python3 -c "
import json, sys
d = json.load(open('$file'))
c = d.get('overdue_total', 0)
print(f'doob: {c} overdue') if c > 3 else sys.exit(1)
" 2>/dev/null
"""
when = "test -f $HOME/.cache/doob/status.json"
format = "[$output]($style) "
style = "red"
shell = ["sh", "-c"]
```

Add `$custom.doob$custom.doob_urgent` to the prompt `format` string.

- [ ] **Step 3: Test modules**

```bash
# yellow (2 overdue)
echo '{"updated_at":"2026-04-30T22:00:00Z","pending_total":5,
"overdue_total":2,"overdue_by_repo":{"warpx":2}}' > ~/.cache/doob/status.json
starship prompt

# red (5 overdue)
echo '{"updated_at":"2026-04-30T22:00:00Z","pending_total":8,
"overdue_total":5,"overdue_by_repo":{"warpx":5}}' > ~/.cache/doob/status.json
starship prompt
```

- [ ] **Step 4: Commit**

```bash
git -C ~/dev/dotfiles add starship.toml
git -C ~/dev/dotfiles commit -m "feat: add doob overdue count to Starship prompt"
```

---

## Task 3: Nu Directory-Change Hook

**Goal:** On `cd`, print overdue count for the current repo if > 0.

**Files:**

- Modify: `~/dotfiles/nushell/hooks.nu` (or equivalent PWD hook file)

- [ ] **Step 1: Locate Nu hook file**

Run `ls ~/dotfiles/nushell/` to find where `env_change` hooks are
defined.

- [ ] **Step 2: Add PWD hook**

```nu
$env.config.hooks.env_change.PWD = (
    $env.config.hooks.env_change.PWD? | default []
) ++ [{|before, after|
    let cache = ($env.HOME | path join ".cache/doob/status.json")
    if not ($cache | path exists) { return }
    let data = (open $cache | from json)
    let overdue = ($data.overdue_total? | default 0)
    if $overdue == 0 { return }
    let repo = ($after | path basename)
    let repo_count = ($data.overdue_by_repo? | default {} | get -i $repo | default 0)
    if $repo_count > 0 {
        print $"(ansi yellow)doob: ($repo_count) overdue in ($repo)(ansi reset)"
    } else {
        print $"(ansi dim)doob: ($overdue) overdue total(ansi reset)"
    }
}]
```

- [ ] **Step 3: Test hook**

With a populated `~/.cache/doob/status.json`, open a Nu shell and
`cd ~/dev/warpx`. Expected: `doob: N overdue in warpx` in yellow.

- [ ] **Step 4: Commit**

```bash
git -C ~/dev/dotfiles add nushell/
git -C ~/dev/dotfiles commit -m "feat: show doob overdue count on directory change in Nu"
```

---

## Task 4: Handon Banner on Shell Start

**Goal:** On shell start, run `handoff-detect $PWD` and surface top
priority items as a styled banner.

**Files:**

- Modify: `~/dotfiles/nushell/env.nu`

**Note:** Nu exit hooks are unreliable as of Nu 0.100. The open hook
(`env.nu`) is reliable. For exit — first check
`nu -c '$env.config.hooks | columns'` before attempting; if no clean
exit hook exists, defer to Phase 4 (Warp session event wiring).

- [ ] **Step 1: Add handon banner to env.nu**

Append to `~/dotfiles/nushell/env.nu`:

```nu
# Handon banner — show top handoff items on shell start
let _handoff_bin = (which handoff-detect | get path | first? | default "")
if ($_handoff_bin | is-not-empty) {
    let _result = (do { run-external "handoff-detect" $env.PWD } | complete)
    if $_result.exit_code == 0 and ($_result.stdout | str trim | is-not-empty) {
        print $"(ansi cyan)--- handoff ---"
        print $_result.stdout
        print (ansi reset)
    }
}
```

- [ ] **Step 2: Test banner**

Open a new terminal tab in a repo with a HANDOFF file.
Expected: handoff items printed as a banner.

- [ ] **Step 3: Investigate exit hook**

```bash
nu -c '$env.config.hooks | columns'
```

Document available hook types. If `pre_exit` or equivalent exists, wire
handoff snapshot. Otherwise note as Phase 4 item.

- [ ] **Step 4: Commit**

```bash
git -C ~/dev/dotfiles add nushell/
git -C ~/dev/dotfiles commit -m "feat: handon banner on shell start"
```

---

## Verification Checklist

- [ ] Open new shell tab — handoff banner appears (if HANDOFF in cwd)
- [ ] `cd ~/dev/warpx` — overdue doob count shown if > 0
- [ ] `doob todo add "verify phase 2" --repo warpx` — status.json updated
- [ ] `starship prompt` with 2 overdue → yellow; 5 overdue → red

## Phase 3 Note

Phase 3 (Doob sidebar panel in warpx) depends only on Phase 1 MCP tools
and can proceed in parallel. The status.json cache is a nice-to-have for
the panel's initial render but not a blocker.
