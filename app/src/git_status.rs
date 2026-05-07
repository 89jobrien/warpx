//! Rich git status parsing for the prompt integration.
//!
//! Parses `git status --porcelain=v2 --branch` output into structured data.
//! Used by the shell indicator when `OzRichGitPrompt` is enabled.

#![allow(dead_code)]

/// A component of the git status shown in the prompt.
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
    settings_value::SettingsValue,
)]
#[serde(rename_all = "snake_case")]
pub enum PromptGitComponent {
    /// The current branch name (or detached HEAD SHA prefix).
    Branch,
    /// Dirty indicator — shown when there are uncommitted changes.
    Dirty,
    /// Stash count, shown as `s<N>` (e.g. `s3`).
    Stash,
    /// Ahead/behind remote, shown as `↑<N>↓<M>` (omits zero side).
    AheadBehind,
}

/// Parsed git status information extracted from porcelain v2 output.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GitStatusInfo {
    pub branch: Option<String>,
    pub is_dirty: bool,
    pub stash_count: u32,
    pub ahead: u32,
    pub behind: u32,
}

impl GitStatusInfo {
    /// Parse the output of `git status --porcelain=v2 --branch` and
    /// optionally a stash count line `stash: N` appended by the shell hook.
    pub fn parse(output: &str) -> Self {
        let mut info = GitStatusInfo::default();

        for line in output.lines() {
            if let Some(rest) = line.strip_prefix("# branch.head ") {
                info.branch = Some(rest.trim().to_owned());
            } else if let Some(rest) = line.strip_prefix("# branch.ab ") {
                parse_ahead_behind(rest, &mut info);
            } else if line.starts_with("stash: ") {
                if let Some(n) = line.strip_prefix("stash: ") {
                    info.stash_count = n.trim().parse().unwrap_or(0);
                }
            } else if !line.starts_with('#') && !line.is_empty() {
                info.is_dirty = true;
            }
        }

        info
    }

    /// Render the components in the order given, separated by spaces.
    pub fn render(&self, components: &[PromptGitComponent]) -> String {
        let parts: Vec<String> = components
            .iter()
            .filter_map(|c| self.render_component(*c))
            .collect();
        parts.join(" ")
    }

    fn render_component(&self, component: PromptGitComponent) -> Option<String> {
        match component {
            PromptGitComponent::Branch => self.branch.clone(),
            PromptGitComponent::Dirty => {
                if self.is_dirty {
                    Some("*".to_owned())
                } else {
                    None
                }
            }
            PromptGitComponent::Stash => {
                if self.stash_count > 0 {
                    Some(format!("s{}", self.stash_count))
                } else {
                    None
                }
            }
            PromptGitComponent::AheadBehind => {
                let ahead = self.ahead;
                let behind = self.behind;
                match (ahead, behind) {
                    (0, 0) => None,
                    (a, 0) => Some(format!("↑{a}")),
                    (0, b) => Some(format!("↓{b}")),
                    (a, b) => Some(format!("↑{a}↓{b}")),
                }
            }
        }
    }
}

fn parse_ahead_behind(s: &str, info: &mut GitStatusInfo) {
    // Format: "+<ahead> -<behind>"
    for part in s.split_whitespace() {
        if let Some(n) = part.strip_prefix('+') {
            info.ahead = n.parse().unwrap_or(0);
        } else if let Some(n) = part.strip_prefix('-') {
            info.behind = n.parse().unwrap_or(0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- RED: these tests were written before any implementation ---

    const SAMPLE_PORCELAIN: &str = "\
# branch.oid abc1234
# branch.head main
# branch.upstream origin/main
# branch.ab +2 -1
1 .M N... 100644 100644 100644 aaa bbb file.rs
";

    #[test]
    fn parses_branch_name() {
        let info = GitStatusInfo::parse(SAMPLE_PORCELAIN);
        assert_eq!(
            info.branch.as_deref(),
            Some("main"),
            "branch should be main"
        );
    }

    #[test]
    fn parses_ahead_behind() {
        let info = GitStatusInfo::parse(SAMPLE_PORCELAIN);
        assert_eq!(info.ahead, 2, "ahead should be 2");
        assert_eq!(info.behind, 1, "behind should be 1");
    }

    #[test]
    fn parses_dirty_flag() {
        let info = GitStatusInfo::parse(SAMPLE_PORCELAIN);
        assert!(
            info.is_dirty,
            "should be dirty when there are changed files"
        );
    }

    #[test]
    fn clean_repo_not_dirty() {
        let clean = "# branch.oid abc\n# branch.head main\n# branch.ab +0 -0\n";
        let info = GitStatusInfo::parse(clean);
        assert!(!info.is_dirty, "clean repo should not be dirty");
    }

    #[test]
    fn parses_stash_count() {
        let with_stash = format!("{}\nstash: 3", SAMPLE_PORCELAIN);
        let info = GitStatusInfo::parse(&with_stash);
        assert_eq!(info.stash_count, 3, "stash_count should be 3");
    }

    #[test]
    fn zero_stash_when_absent() {
        let info = GitStatusInfo::parse(SAMPLE_PORCELAIN);
        assert_eq!(info.stash_count, 0, "stash_count should default to 0");
    }

    #[test]
    fn render_follows_component_order() {
        let info = GitStatusInfo {
            branch: Some("feat".to_owned()),
            is_dirty: true,
            stash_count: 2,
            ahead: 1,
            behind: 0,
        };

        // Default order: branch, dirty
        let default_order = info.render(&[PromptGitComponent::Branch, PromptGitComponent::Dirty]);
        assert_eq!(default_order, "feat *", "default order: branch then dirty");

        // Reversed order
        let reversed = info.render(&[PromptGitComponent::Dirty, PromptGitComponent::Branch]);
        assert_eq!(reversed, "* feat", "reversed order: dirty then branch");
    }

    #[test]
    fn render_stash_shown_as_s_n() {
        let info = GitStatusInfo {
            stash_count: 3,
            ..Default::default()
        };
        let out = info.render(&[PromptGitComponent::Stash]);
        assert_eq!(out, "s3");
    }

    #[test]
    fn render_ahead_behind_both_nonzero() {
        let info = GitStatusInfo {
            ahead: 2,
            behind: 1,
            ..Default::default()
        };
        let out = info.render(&[PromptGitComponent::AheadBehind]);
        assert_eq!(out, "↑2↓1");
    }

    #[test]
    fn render_ahead_only() {
        let info = GitStatusInfo {
            ahead: 3,
            behind: 0,
            ..Default::default()
        };
        let out = info.render(&[PromptGitComponent::AheadBehind]);
        assert_eq!(out, "↑3");
    }

    #[test]
    fn render_behind_only() {
        let info = GitStatusInfo {
            ahead: 0,
            behind: 2,
            ..Default::default()
        };
        let out = info.render(&[PromptGitComponent::AheadBehind]);
        assert_eq!(out, "↓2");
    }

    #[test]
    fn render_ahead_behind_both_zero_is_empty() {
        let info = GitStatusInfo {
            ahead: 0,
            behind: 0,
            ..Default::default()
        };
        let out = info.render(&[PromptGitComponent::AheadBehind]);
        assert_eq!(out, "");
    }

    #[test]
    fn render_omits_zero_stash() {
        let info = GitStatusInfo {
            stash_count: 0,
            ..Default::default()
        };
        let out = info.render(&[PromptGitComponent::Stash]);
        assert_eq!(out, "");
    }

    #[test]
    fn detached_head_parsed() {
        let detached = "# branch.oid abc1234\n# branch.head (detached)\n# branch.ab +0 -0\n";
        let info = GitStatusInfo::parse(detached);
        assert_eq!(
            info.branch.as_deref(),
            Some("(detached)"),
            "detached HEAD should be preserved"
        );
    }
}
