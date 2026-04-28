# CLAUDE.md — warpx

Warp terminal fork for personal customization. AGPL-3.0 upstream, custom
branch `joe/main` for modifications.

## Build & Run

```bash
cargo run                    # build and run locally
./script/bootstrap           # platform-specific setup (macOS)
./script/install_cargo_build_deps
./script/install_cargo_test_deps
```

## Testing

```bash
cargo nextest run --no-fail-fast --workspace --exclude command-signatures-v2
cargo test --doc
```

## Presubmit

```bash
./script/presubmit           # fmt + clippy + tests — run before any PR
cargo fmt --check --all
cargo clippy --workspace --all-targets --all-features --tests -- -D warnings
```

## Architecture

- **WarpUI** (`ui/`): custom UI framework, Entity-Component-Handle pattern
- **App** (`app/`): terminal emulation, AI/agent mode, settings, auth
- **warp_core** (`crates/warp_core/`): core utilities, platform abstractions
- **warp_features** (`crates/warp_features/`): compile-time feature flags
- Cargo workspace with 34+ member crates
- Cross-platform: macOS, Windows, Linux, WASM

## Feature Flags

Defined in `crates/warp_features/src/lib.rs`. Tiers: `DOGFOOD_FLAGS`,
`PREVIEW_FLAGS`, `RELEASE_FLAGS`. Gate code with
`FeatureFlag::YourFlag.is_enabled()`. Prefer runtime checks over `#[cfg]`.

## Coding Conventions

- Context params (`AppContext`, `ViewContext`, `ModelContext`) named `ctx`,
  placed last (except when a closure param follows)
- Remove unused params entirely (no `_` prefix)
- Inline format args: `eprintln!("{message}")` not `eprintln!("{}", message)`
- Exhaustive match — avoid wildcard `_` patterns
- Unit tests in separate `${filename}_tests.rs` files
- Do not remove existing comments when making unrelated changes

## Terminal Model Locking

Be extremely careful with `model.lock()` on `TerminalModel`. Multiple locks
from different call sites cause deadlocks (UI freeze). Pass already-locked
refs down the call stack. Keep lock scope minimal.

## Key Paths

| Component | Path |
|-----------|------|
| Feature flags | `crates/warp_features/src/lib.rs` |
| Main binary | `app/` |
| UI framework | `ui/` |
| Settings | `app/src/settings/` |
| AI/Agent | `app/src/ai/` |
| Skills (bundled) | `resources/bundled/skills/` |
| MCP config | `~/.warp/.mcp.json` |
| Skills (user) | `~/.warp/skills/`, `~/.agents/skills/` |

## Branch Strategy

- `main` — tracks upstream warp
- `joe/main` — personal customizations
- Feature branches off `joe/main`

## Upstream Reference

See `WARP.md` for full upstream development guidance.
