use crate::keyboard::{
    detect_conflicts, KeybindConflict, PersistedTrigger, UserDefinedKeybinding,
    REMOVED_KEYBINDING_SERIALIZATION,
};
use anyhow::{Ok, Result};
use std::collections::HashMap;
use vec1::vec1;

use warpui::keymap::Keystroke;

#[test]
fn test_short_user_defined_keybinding_to_persisted_trigger() {
    let keystroke = Keystroke::parse("ctrl-p").unwrap();
    let keybinding = UserDefinedKeybinding::Keystrokes(vec1![keystroke]);
    let persisted_trigger: PersistedTrigger = keybinding.into();

    assert_eq!(persisted_trigger, PersistedTrigger("ctrl-p".to_string()));
}

#[test]
fn test_long_user_defined_keybinding_to_persisted_trigger() {
    let keystroke = Keystroke::parse("ctrl-p").unwrap();
    let other_keystroke = Keystroke::parse("1").unwrap();

    let keybinding = UserDefinedKeybinding::Keystrokes(vec1![keystroke, other_keystroke]);
    let persisted_trigger: PersistedTrigger = keybinding.into();

    assert_eq!(persisted_trigger, PersistedTrigger("ctrl-p 1".to_string()));
}

#[test]
fn test_short_persisted_trigger_to_user_defined_keybinding() -> Result<()> {
    let persisted_trigger = PersistedTrigger("ctrl-x".to_string());
    let keybinding = UserDefinedKeybinding::try_from(persisted_trigger)?;

    let correct_keybinding =
        UserDefinedKeybinding::Keystrokes(vec1![Keystroke::parse("ctrl-x").unwrap()]);

    assert_eq!(keybinding, correct_keybinding);
    Ok(())
}

#[test]
fn test_long_persisted_trigger_to_user_defined_keybinding() -> Result<()> {
    let persisted_trigger = PersistedTrigger("ctrl-x 8".to_string());
    let keybinding = UserDefinedKeybinding::try_from(persisted_trigger)?;

    let correct_keybinding = UserDefinedKeybinding::Keystrokes(vec1![
        Keystroke::parse("ctrl-x").unwrap(),
        Keystroke::parse("8").unwrap()
    ]);

    assert_eq!(keybinding, correct_keybinding);
    Ok(())
}

#[test]
fn test_persisted_trigger_to_removed_user_keybinding() -> Result<()> {
    let persisted_trigger = PersistedTrigger(REMOVED_KEYBINDING_SERIALIZATION.to_string());
    let keybinding = UserDefinedKeybinding::try_from(persisted_trigger)?;

    assert_eq!(keybinding, UserDefinedKeybinding::Removed);
    Ok(())
}

#[test]
fn test_removed_user_keybinding_to_persisted_trigger() {
    let keybinding = UserDefinedKeybinding::Removed;
    let persisted_trigger: PersistedTrigger = keybinding.into();

    assert_eq!(
        persisted_trigger,
        PersistedTrigger(REMOVED_KEYBINDING_SERIALIZATION.to_string())
    );
}

#[test]
fn test_unparsable_persisted_trigger() {
    let persisted_trigger = PersistedTrigger("".to_string());
    let keybinding = UserDefinedKeybinding::try_from(persisted_trigger);

    assert!(keybinding.is_err());
}

#[test]
fn test_detect_conflicts_no_conflicts() {
    let mut bindings = HashMap::new();
    bindings.insert("action:one".to_string(), "ctrl-a".to_string());
    bindings.insert("action:two".to_string(), "ctrl-b".to_string());
    bindings.insert("action:three".to_string(), "ctrl-c".to_string());

    let conflicts = detect_conflicts(&bindings);
    assert!(conflicts.is_empty());
}

#[test]
fn test_detect_conflicts_single_conflict() {
    let mut bindings = HashMap::new();
    bindings.insert("action:alpha".to_string(), "ctrl-p".to_string());
    bindings.insert("action:beta".to_string(), "ctrl-p".to_string());
    bindings.insert("action:gamma".to_string(), "ctrl-z".to_string());

    let conflicts = detect_conflicts(&bindings);
    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0].chord, "ctrl-p");
    assert_eq!(conflicts[0].actions, vec!["action:alpha", "action:beta"]);
}

#[test]
fn test_detect_conflicts_multiple_conflicts() {
    let mut bindings = HashMap::new();
    bindings.insert("editor:cut".to_string(), "ctrl-x".to_string());
    bindings.insert("terminal:close".to_string(), "ctrl-x".to_string());
    bindings.insert("editor:paste".to_string(), "ctrl-v".to_string());
    bindings.insert("workspace:quit".to_string(), "ctrl-v".to_string());
    bindings.insert("editor:undo".to_string(), "ctrl-z".to_string());

    let conflicts = detect_conflicts(&bindings);
    assert_eq!(conflicts.len(), 2);
    // sorted by chord: ctrl-v < ctrl-x
    assert_eq!(conflicts[0].chord, "ctrl-v");
    assert_eq!(conflicts[1].chord, "ctrl-x");
}

#[test]
fn test_detect_conflicts_three_way() {
    let mut bindings = HashMap::new();
    bindings.insert("action:a".to_string(), "cmd-shift-k".to_string());
    bindings.insert("action:b".to_string(), "cmd-shift-k".to_string());
    bindings.insert("action:c".to_string(), "cmd-shift-k".to_string());

    let conflicts = detect_conflicts(&bindings);
    assert_eq!(conflicts.len(), 1);
    assert_eq!(
        conflicts[0],
        KeybindConflict {
            chord: "cmd-shift-k".to_string(),
            actions: vec![
                "action:a".to_string(),
                "action:b".to_string(),
                "action:c".to_string()
            ],
        }
    );
}

#[test]
fn test_detect_conflicts_empty_bindings() {
    let bindings = HashMap::new();
    let conflicts = detect_conflicts(&bindings);
    assert!(conflicts.is_empty());
}
