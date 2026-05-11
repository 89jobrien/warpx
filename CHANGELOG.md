# Changelog

All notable changes to warpx are documented here.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
Versions follow [Semantic Versioning](https://semver.org/).

---

## [Unreleased]

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
