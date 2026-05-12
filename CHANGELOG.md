# Changelog

All notable changes to warpx are documented here.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
Versions follow [Semantic Versioning](https://semver.org/).

---

## [Unreleased]

### Added

- `cargo xtask run [--release] [-- args]` — build and launch WarpX.app
- `cargo xtask install [--release]` — build and install to /Applications
- macOS self-hosted test runner and xvfb for Linux integration tests

### Changed

- Justfile `run` and `install` targets now use `cargo xtask`

### Fixed

- Retryable `async_assert` in block index test assertions
- Clippy: replace `let...else` with `?` in doob panel

---

## [v0.4.0] - 2026-05-11

### Added

- Inline context generation in `warpx::context::generate_context()` — no
  external script dependency; Context Window panel now auto-generates on
  open/refresh
- `examples/context_gen.rs` CLI entrypoint and `.warp/workflows/refresh_context_window.yaml`
- Claude plugin skill discovery (`SkillProvider::ClaudePlugin`,
  `OzClaudePluginSkills` flag) with hot-reload from `~/.claude/plugins/`
- Handup panel and generated project context panel
- Shell-agnostic integration test module

### Fixed

- `protect-main.yml` now rejects PRs to main instead of silently
  redirecting pushes (main is upstream-only)
- Linux CI: allow zero integration tests, cover `ShellType::Nu` in
  autoupdate match arms

### Changed

- HANDOFF YAML files (`.ctx/HANDOFF.*.yaml`) removed from git tracking;
  managed by doob locally

---

## [v0.3.0] - 2026-05-10

### Added

- Nushell (`nu`) as a fully supported shell type: discovery, selection,
  session spawning, bootstrap, and prompt hooks (Precmd/Preexec/CommandFinished)
- `cargo xtask bump` — semver version management for the warpx crate
- `CHANGELOG.md` — version history tracking

---

## [v0.1.0] - 2026-05-10

Initial personal fork baseline.

### Added

- `ShellType::Nu` variant with full metadata (name, history, rc paths, combiners)
- JIT context injection for agent prompts (`OzJitContext` feature flag)
- Session lifecycle hooks (handon/handoff on tab open/close)
- macOS self-hosted CI runner (`warpx-mac`)
- Handoff panel in left sidebar
- MCP integration (personal-mcp, pieces)
- Bundled skills from `~/.warp/skills/`
