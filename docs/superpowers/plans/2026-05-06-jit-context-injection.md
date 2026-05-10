# Wave 2 — JIT Context Injection for Agent Prompts

> **Status: OPEN**

> **For agentic workers:** Use `/godmode:task-driven-development` for each implementation
> step. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Before an agent prompt is dispatched in warpx, parse the user's input for file
references (globs, paths, keywords), search the cwd for matching files within a token
budget, and inject relevant file snippets as additional context — mirroring the Claude Code
`UserPromptSubmit` Python hook pattern.

**Architecture:** Pure logic in `crates/warpx/src/jit_context.rs`. App wrapper in
`app/src/joe/jit_context.rs`. Config loaded from `~/.warp/jit_context.toml`. Wired into
the agent input submit path, gated by `FeatureFlag::OzJitContext`.

**Tech Stack:** Rust, `glob` crate, `tiktoken-rs` or character-count heuristic for token
estimation, `toml` for config.

**Repo:** `/Users/joe/dev/warpx`

**GitHub issue:** #5

**Reference implementation:**
`$HOME/.claude/hooks/python/jit-context.py` (Claude Code `UserPromptSubmit` hook)

---

## Reference Hook Summary

The Python hook does the following on every user prompt submission:

1. Parse the prompt for: glob patterns (`*.rs`, `**/*.toml`), explicit file paths, and
   keyword hints (e.g. `feature_name`, `ModuleName`).
2. Walk the cwd (excluding `.git`, `target`, `node_modules`) for files matching any
   extracted pattern.
3. For each candidate file, estimate token cost (chars / 4). Skip if adding would exceed
   `TOKEN_BUDGET` (default 500).
4. Build `additionalContext` containing file path + relevant snippet.
5. Return `{"additionalContext": "..."}` in stdout for Claude Code to prepend.

The warpx equivalent injects context into the agent `AiPrompt` before the HTTP request is
sent to the AI backend.

---

## Task 1: Pure logic — `crates/warpx/src/jit_context.rs`

**Files:**

- `crates/warpx/src/lib.rs` — add `pub mod jit_context;`
- `crates/warpx/src/jit_context.rs` (new)

### Data types

```rust
pub struct JitConfig {
    /// Max tokens to inject across all files. Default: 500.
    pub token_budget: usize,
    /// Glob patterns always excluded (in addition to defaults).
    pub exclude: Vec<String>,
    /// Max file size in bytes to consider. Default: 32 KiB.
    pub max_file_bytes: usize,
}

impl Default for JitConfig {
    fn default() -> Self {
        Self {
            token_budget: 500,
            exclude: Vec::new(),
            max_file_bytes: 32 * 1024,
        }
    }
}

pub struct JitSnippet {
    pub path: String,
    pub content: String,
    pub tokens: usize,
}

pub struct JitResult {
    pub snippets: Vec<JitSnippet>,
    pub total_tokens: usize,
}
```

### Functions

- [ ] **Step 1: `extract_patterns(prompt: &str) -> Vec<String>`**

  Returns candidate file patterns from the prompt:
  - Glob tokens: any word containing `*`, `?`, or `{}`
  - Explicit paths: tokens matching `[./\w-]+\.\w{1,6}` (look like file paths)
  - Keywords: CamelCase or snake_case words > 4 chars (heuristic for module/type names)

- [ ] **Step 2: `estimate_tokens(content: &str) -> usize`**

  Returns `content.len() / 4` (char-count heuristic). No external dependency.

- [ ] **Step 3: `find_candidates(cwd: &Path, patterns: &[String], config: &JitConfig) -> Vec<PathBuf>`**

  Walks `cwd` (non-recursively at depth 3 max), excludes:
  - `.git/`, `target/`, `node_modules/`, `.ctx/`
  - Any pattern in `config.exclude`

  Returns files whose name or contents (first 512 bytes) match any pattern. Limit to 20
  candidates max.

- [ ] **Step 4: `build_jit_context(cwd: &Path, prompt: &str, config: &JitConfig) -> JitResult`**

  Orchestrates the above:
  1. `extract_patterns(prompt)`
  2. `find_candidates(cwd, patterns, config)`
  3. For each candidate (sorted by path):
     - Read file, check size ≤ `config.max_file_bytes`
     - `estimate_tokens(content)`
     - If `total_tokens + file_tokens <= config.token_budget`: add to result
     - Otherwise: skip
  4. Return `JitResult`

- [ ] **Step 5: Unit tests** (`crates/warpx/src/jit_context_tests.rs`)
  - `extract_patterns_finds_glob` — `"show me **/*.toml"` → `["**/*.toml"]`
  - `extract_patterns_finds_path` — `"edit src/main.rs"` → `["src/main.rs"]`
  - `estimate_tokens_char_heuristic` — 400-char string → 100 tokens
  - `build_jit_context_respects_budget` — temp dir with 3 files, budget only fits 2

- [ ] **Step 6: `cargo nextest run -p warpx -- jit_context`**

---

## Task 2: Config loader

**Files:**

- `crates/warpx/src/jit_context.rs` — add `load_config`

- [ ] **Step 1: Add `load_config(config_path: &Path) -> JitConfig`**

  Reads `~/.warp/jit_context.toml` if it exists. Falls back to `JitConfig::default()`.

  TOML schema:

  ```toml
  token_budget = 800
  max_file_bytes = 65536
  exclude = ["vendor/**", "*.lock"]
  ```

  Uses `toml` crate (already in workspace or add to `crates/warpx/Cargo.toml`).

- [ ] **Step 2: `cargo check -p warpx`**

---

## Task 3: App wrapper — `app/src/joe/jit_context.rs`

**Files:**

- `app/src/joe/mod.rs` — add `pub mod jit_context;`
- `app/src/joe/jit_context.rs` (new)

- [ ] **Step 1: Write `prepare_jit_context(prompt: &str, cwd: &Path) -> Option<String>`**

  ```rust
  use warpx::jit_context::{build_jit_context, load_config, JitConfig};
  use warp_features::FeatureFlag;

  pub fn prepare_jit_context(prompt: &str, cwd: &Path) -> Option<String> {
      if !FeatureFlag::OzJitContext.is_enabled() {
          return None;
      }
      let config_path = dirs::home_dir()?.join(".warp/jit_context.toml");
      let config = load_config(&config_path);
      let result = build_jit_context(cwd, prompt, &config);
      if result.snippets.is_empty() {
          return None;
      }
      let mut context = String::new();
      for snippet in &result.snippets {
          context.push_str(&format!("// {}\n{}\n\n", snippet.path, snippet.content));
      }
      Some(context)
  }
  ```

- [ ] **Step 2: `cargo check -p warp`**

---

## Task 4: Feature flag

**Files:**

- `crates/warp_features/src/lib.rs`

- [ ] **Step 1: Add `OzJitContext` to `DOGFOOD_FLAGS`**

  Follow exact style of adjacent `OzSessionHooks`, `OzDoobPanel` entries.

- [ ] **Step 2: `cargo check -p warp_features`**

---

## Task 5: Wire into agent input submit path

**Files:**

- Identify the agent input submit call site by grepping for `AiPrompt`, `submit_prompt`,
  `agent_input`, or similar in `app/src/ai/`.

- [ ] **Step 1: Locate agent prompt dispatch**

  ```
  Grep: AiPrompt|submit_prompt|agent.*prompt|on_submit
  Path: app/src/ai/
  ```

  Read the relevant file(s) to identify the exact struct/method that constructs the prompt
  payload before the HTTP call.

- [ ] **Step 2: Inject context before dispatch**

  At the prompt construction site, add:

  ```rust
  let cwd = /* read from terminal session / env */ std::env::current_dir()
      .unwrap_or_else(|_| PathBuf::from("/"));
  if let Some(ctx) = crate::joe::jit_context::prepare_jit_context(&user_input, &cwd) {
      prompt = format!("{ctx}\n\n{prompt}");
  }
  ```

  The injection must be non-blocking — `build_jit_context` reads files synchronously but
  is fast (limited depth + file count). If latency is a concern, move to `ctx.spawn`.

- [ ] **Step 3: `cargo check -p warp`**

---

## Task 6: Commit and verify

- [ ] **Step 1: Run clippy**

  ```bash
  cargo clippy --workspace -- -D warnings
  ```

- [ ] **Step 2: Run tests**

  ```bash
  cargo nextest run -p warpx -- jit_context
  ```

- [ ] **Step 3: Commit**

  ```bash
  git add -A
  git commit -m "feat: JIT context injection for agent prompts (OzJitContext flag) closes #5"
  git push
  ```

- [ ] **Step 4: Manual verification**

  Build and run warpx:

  ```bash
  cargo run --bin warpx
  ```

  In a repo with Rust source files, open agent mode and type:
  `"explain what BackingView does"` — verify that `pane_impl.rs` or relevant files are
  injected as context before the response.

---

## Verification Checklist

- [ ] `OzJitContext` present in `DOGFOOD_FLAGS`
- [ ] `extract_patterns` correctly identifies globs, paths, and keywords from prompt text
- [ ] `build_jit_context` respects `token_budget` — never exceeds it
- [ ] Files from `.git/`, `target/`, `node_modules/` are never injected
- [ ] `prepare_jit_context` returns `None` when flag is disabled
- [ ] Agent prompt receives injected context prepended before user input
- [ ] No UI freeze — file walking is fast (depth 3, max 20 candidates)
- [ ] `cargo nextest run -p warpx -- jit_context` — all tests green
- [ ] `cargo clippy --workspace -- -D warnings` — zero warnings

---

## Config Reference (`~/.warp/jit_context.toml`)

```toml
# Maximum tokens to inject (default: 500)
token_budget = 500

# Maximum file size to read, in bytes (default: 32768)
max_file_bytes = 32768

# Glob patterns to exclude beyond defaults (.git, target, node_modules, .ctx)
exclude = []
```

---

## Phase Note

If the agent input submit path proves difficult to locate or intercept without touching
upstream Warp internals, an alternative is to inject context at the `joe/` input
pre-processing layer and surface it via a UI affordance (e.g. a "context: N files" badge
in the agent input bar). Defer the UI affordance to Wave 3; ship the silent injection
first.
