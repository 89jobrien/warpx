use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
#[cfg(not(test))]
use std::env::var_os;
use vec1::{vec1, Vec1};
use warpui::keymap::Keystroke;
#[cfg(not(test))]
use warpui::keymap::Trigger;
use warpui::AppContext;

use anyhow::Context;

/// Environment variable to disable saving keybindings to file (used in integration tests)
pub const DISABLE_SAVE_ENV_VAR: &str = "WARP_TEST_DISABLE_KEYBINDING_SAVE";

/// A keybinding conflict: the same chord string is bound to more than one action name.
#[derive(Debug, PartialEq)]
pub struct KeybindConflict {
    /// The normalized chord (e.g. `"ctrl-p"`, `"cmd-shift-k"`).
    pub chord: String,
    /// The action names that share this chord (at least two).
    pub actions: Vec<String>,
}

/// Detect conflicts within a set of user-defined keybindings.
///
/// A conflict occurs when two or more action names map to an identical chord string.
/// Only non-removed bindings are considered.
///
/// Returns a `Vec<KeybindConflict>` — one entry per conflicting chord, sorted by chord for
/// deterministic output.
pub fn detect_conflicts(bindings: &HashMap<String, String>) -> Vec<KeybindConflict> {
    // chord → list of action names that share it
    let mut chord_to_actions: HashMap<String, Vec<String>> = HashMap::new();

    for (action, chord) in bindings {
        chord_to_actions
            .entry(chord.clone())
            .or_default()
            .push(action.clone());
    }

    let mut conflicts: Vec<KeybindConflict> = chord_to_actions
        .into_iter()
        .filter(|(_, actions)| actions.len() > 1)
        .map(|(chord, mut actions)| {
            actions.sort();
            KeybindConflict { chord, actions }
        })
        .collect();

    conflicts.sort_by(|a, b| a.chord.cmp(&b.chord));
    conflicts
}
const REMOVED_KEYBINDING_SERIALIZATION: &str = "none";

#[derive(PartialEq, Debug)]
/// A type to encapsulate the valid states of a keybinding
/// provided by a user in their keybindings.yaml file
pub enum UserDefinedKeybinding {
    /// Keybinding we can normalize/parse and will be recognized
    Keystrokes(Vec1<Keystroke>),
    /// User chose to remove the keybinding for an action
    Removed,
}

impl UserDefinedKeybinding {
    pub fn keystroke(value: Keystroke) -> Self {
        UserDefinedKeybinding::Keystrokes(vec1![value])
    }
}

#[cfg(not(test))]
const KEYBINDINGS_FILE_NAME: &str = "keybindings.yaml";

/// Load all stored custom keybindings into the UI framework so that they are used.
///
/// Returns any detected keybinding conflicts so the caller can surface them after
/// windows are available.
#[cfg(not(test))]
pub fn load_custom_keybindings(app: &mut AppContext) -> Vec<KeybindConflict> {
    if let Some(keybindings) = read_custom_keybindings() {
        // Build a chord→action map from non-removed, parseable entries for conflict detection.
        let mut chord_map: HashMap<String, String> = HashMap::new();

        for (name, trigger) in &keybindings.0 {
            let keybinding_type = UserDefinedKeybinding::try_from(trigger.clone());
            if let Ok(UserDefinedKeybinding::Keystrokes(keystrokes)) = keybinding_type {
                let chord = keystrokes.iter().map(Keystroke::normalized).join(" ");
                chord_map.insert(name.clone(), chord);
            }
        }

        let conflicts = detect_conflicts(&chord_map);

        if !conflicts.is_empty() {
            let count = conflicts.len();
            let detail = conflicts
                .iter()
                .map(|c| format!("  {} -> {}", c.chord, c.actions.join(", ")))
                .join("\n");
            log::warn!(
                "{count} keybinding conflict{} detected:\n{detail}",
                if count == 1 { "" } else { "s" }
            );
        }

        for (name, trigger) in keybindings.0 {
            let keybinding_type = UserDefinedKeybinding::try_from(trigger.clone());

            match keybinding_type {
                Ok(UserDefinedKeybinding::Removed) => {
                    app.set_custom_trigger(name, Trigger::Empty);
                }
                Ok(UserDefinedKeybinding::Keystrokes(keystrokes)) => {
                    app.set_custom_trigger(name, Trigger::Keystrokes(keystrokes.to_vec()));
                }
                Err(e) => {
                    log::warn!(
                        "Tried to load an unparsable keybinding of {trigger:?} for action: {name}. error: {e}"
                    );
                }
            }
        }

        conflicts
    } else {
        Vec::new()
    }
}

/// Write a new custom keybinding to disk
/// using the name of the editable binding and the new keystrokes
/// if keystrokes is UserDefinedKeybinding::Removed
/// we write a special value to disk to save that state
#[cfg(not(test))]
pub fn write_custom_keybinding(name: String, keybinding: UserDefinedKeybinding) {
    // In tests, we don't want to write the actual keybindings file, since that could clobber the
    // user's current settings, so we no-op
    if var_os(DISABLE_SAVE_ENV_VAR).is_some() {
        return;
    }

    let mut map = read_custom_keybindings().unwrap_or_default();

    map.0.insert(name, keybinding.into());
    save_custom_keybindings(map);
}

/// Remove a custom keybinding from disk.
#[cfg(not(test))]
pub fn remove_custom_keybinding<N>(name: N)
where
    N: AsRef<str>,
{
    // In tests, we don't want to write the actual keybindings file, since that could clobber the
    // users current settings, so we no-op
    if var_os(DISABLE_SAVE_ENV_VAR).is_some() {
        return;
    }

    let mut map = read_custom_keybindings().unwrap_or_default();

    map.0.remove(name.as_ref());
    save_custom_keybindings(map);
}

#[cfg(not(test))]
pub fn keybinding_file_path() -> std::path::PathBuf {
    warp_core::paths::config_local_dir().join(KEYBINDINGS_FILE_NAME)
}

/// Save the custom keybindings map to disk.
#[cfg(not(test))]
// Allow unused variables when no local filesystem exists as the arg is unused.
#[cfg_attr(not(feature = "local_fs"), allow(unused_variables))]
fn save_custom_keybindings(map: CustomKeybindings) {
    cfg_if::cfg_if! {
        if #[cfg(feature = "local_fs")] {
            let file = match crate::util::file::create_file(keybinding_file_path()) {
                Ok(f) => f,
                Err(e) => {
                    log::warn!("Unable to open file for storing custom keybindings: {e}");
                    return;
                }
            };
            let writer = std::io::BufWriter::new(file);

            if let Err(e) = serde_yaml::to_writer(writer, &map) {
                log::warn!("Unable to serialize custom keybindings to file: {e}");
            }
        } else {
            log::warn!("TODO(wasm): need to implement keybindings support");
        }
    }
}

/// Read the stored custom keybindings from disk into a map of Editable Binding Name -> Trigger
///
/// Returns `None` if the file can't be read or the deserialization fails
#[cfg(not(test))]
fn read_custom_keybindings() -> Option<CustomKeybindings> {
    let file = std::fs::File::open(keybinding_file_path()).ok()?;
    let reader = std::io::BufReader::new(file);

    match serde_yaml::from_reader(reader) {
        Ok(map) => Some(map),
        Err(e) => {
            log::warn!("Unable to deserialize stored keybindings: {e}");
            None
        }
    }
}

// For tests, we don't want to read or write from the filesystem.
//
// Unit tests are run with #[cfg(test)] enabled, so we can define custom no-op implementations
#[cfg(test)]
pub fn load_custom_keybindings(_: &mut AppContext) -> Vec<KeybindConflict> {
    Vec::new()
}
#[cfg(test)]
pub fn write_custom_keybinding(_: String, _: UserDefinedKeybinding) {}
#[cfg(test)]
pub fn remove_custom_keybinding<N>(_: N)
where
    N: AsRef<str>,
{
}

/// Struct that represents the full custom keybindings file for (de-)serialization
///
/// The file format is a top-level YAML map of (Editable Binding Name) -> Keybinding
/// Since many of the editable bindings have a `:` character in their name, the name will need to
/// be quoted in most cases.
/// The format of the keybinding is the normalized version that we use internally, with multiple
/// keystrokes separated by whitespace, if necessary.
///
/// For example:
/// ---
/// "editor:delete_all_left": cmd-shift-A
/// "editor:delete_all_right": cmd-shift-D escape
#[derive(Serialize, Deserialize, Default)]
#[cfg(not(test))]
struct CustomKeybindings(std::collections::HashMap<String, PersistedTrigger>);

/// The normalized version of a keystroke or series of keystrokes that is written into the
/// keybindings file. If there are multiple keystrokes, each is separated by a space
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
struct PersistedTrigger(String);

impl From<UserDefinedKeybinding> for PersistedTrigger {
    fn from(keybinding: UserDefinedKeybinding) -> Self {
        match keybinding {
            UserDefinedKeybinding::Keystrokes(keystrokes) => {
                PersistedTrigger(keystrokes.iter().map(Keystroke::normalized).join(" "))
            }
            UserDefinedKeybinding::Removed => {
                PersistedTrigger(REMOVED_KEYBINDING_SERIALIZATION.to_string())
            }
        }
    }
}

impl TryFrom<PersistedTrigger> for UserDefinedKeybinding {
    type Error = anyhow::Error;
    fn try_from(trigger: PersistedTrigger) -> anyhow::Result<Self> {
        if trigger.0 == REMOVED_KEYBINDING_SERIALIZATION {
            return Ok(UserDefinedKeybinding::Removed);
        }

        let mut keystrokes: Vec<Keystroke> = Vec::new();

        for keystroke in trigger.0.split_whitespace() {
            let parsed_keystroke: Keystroke = Keystroke::parse(keystroke).context(format!(
                "Failed to parse keystroke \"{}\" in trigger \"{}\"",
                keystroke, trigger.0,
            ))?;
            keystrokes.push(parsed_keystroke);
        }

        let parsed_keystrokes: Vec1<Keystroke> = Vec1::try_from(keystrokes).context(format!(
            "No valid keystrokes were found in trigger: {}",
            trigger.0
        ))?;

        Ok(UserDefinedKeybinding::Keystrokes(parsed_keystrokes))
    }
}

#[cfg(test)]
#[path = "keyboard_test.rs"]
mod tests;
