use settings::{macros::define_settings_group, SupportedPlatforms, SyncToCloud};

pub use crate::git_status::PromptGitComponent;

define_settings_group!(PromptGitSettings, settings: [
    prompt_git_components: PromptGitComponentList {
        type: Vec<PromptGitComponent>,
        default: vec![PromptGitComponent::Branch, PromptGitComponent::Dirty],
        supported_platforms: SupportedPlatforms::ALL,
        sync_to_cloud: SyncToCloud::Never,
        private: false,
        toml_path: "general.prompt_git_components",
        description: "Which git status components appear in the prompt and in what order. \
                       Enabled by default: branch, dirty. Opt-in: stash, ahead_behind.",
    },
]);
