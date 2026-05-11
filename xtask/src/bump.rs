//! bump — bump the warpx crate version in crates/warpx/Cargo.toml.
//!
//! Usage: cargo xtask bump [patch|minor|major]
//!
//! Minor bumps are rate-limited to once per calendar day. If a minor bump
//! has already occurred today, the request is silently downgraded to patch.
//! The last minor bump date is tracked in `.warpx-bump-state` (gitignored).

use anyhow::{bail, Result};
use std::fs;
use std::path::Path;

pub fn bump(root: &Path, level: &str) -> Result<()> {
    let manifest_path = root.join("crates/warpx/Cargo.toml");
    let content = fs::read_to_string(&manifest_path)?;

    let current = parse_crate_version(&content)
        .ok_or_else(|| anyhow::anyhow!("could not find version in crates/warpx/Cargo.toml"))?;

    let effective_level = if level == "minor" && minor_bumped_today(root) {
        eprintln!("[warpx] minor bump already applied today — downgrading to patch");
        "patch"
    } else {
        level
    };

    let (major, minor, patch) = parse_semver(&current)?;
    let next = match effective_level {
        "patch" => format!("{major}.{minor}.{}", patch + 1),
        "minor" => format!("{major}.{}.0", minor + 1),
        "major" => format!("{}.0.0", major + 1),
        other => bail!("unknown bump level: {other} (expected patch, minor, or major)"),
    };

    if effective_level == "minor" {
        record_minor_bump(root);
    }

    let updated = content.replacen(
        &format!("version = \"{current}\""),
        &format!("version = \"{next}\""),
        1,
    );

    if updated == content {
        bail!("version string not found in Cargo.toml — nothing changed");
    }

    fs::write(&manifest_path, updated)?;
    println!("[warpx] version bumped {current} → {next}");
    Ok(())
}

fn parse_crate_version(content: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(v) = trimmed.strip_prefix("version = \"") {
            if let Some(v) = v.strip_suffix('"') {
                return Some(v.to_string());
            }
        }
    }
    None
}

fn parse_semver(v: &str) -> Result<(u64, u64, u64)> {
    let parts: Vec<&str> = v.split('.').collect();
    if parts.len() != 3 {
        bail!("version {v:?} is not semver (expected X.Y.Z)");
    }
    Ok((parts[0].parse()?, parts[1].parse()?, parts[2].parse()?))
}

const BUMP_STATE_FILE: &str = ".warpx-bump-state";

fn today() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

fn minor_bumped_today(root: &Path) -> bool {
    let path = root.join(BUMP_STATE_FILE);
    fs::read_to_string(path)
        .ok()
        .map(|content| content.trim() == today())
        .unwrap_or(false)
}

fn record_minor_bump(root: &Path) {
    let path = root.join(BUMP_STATE_FILE);
    let _ = fs::write(path, today());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_semver() {
        assert_eq!(parse_semver("0.1.0").unwrap(), (0, 1, 0));
        assert_eq!(parse_semver("1.23.456").unwrap(), (1, 23, 456));
        assert!(parse_semver("bad").is_err());
    }

    #[test]
    fn test_parse_crate_version() {
        let toml = r#"
[package]
name = "warpx"
version = "0.1.0"
edition = "2024"
"#;
        assert_eq!(parse_crate_version(toml), Some("0.1.0".to_string()));
    }

    #[test]
    fn test_parse_crate_version_missing() {
        assert_eq!(parse_crate_version("[package]\nname = \"x\""), None);
    }
}
