//! Tests that do not depend on a specific shell and are not purely UI tests.
//!
//! Add a test to this module if:
//! * It exercises shell-agnostic application behavior (e.g. clear, focus,
//!   secrets, rendering) that does not belong in `shell_integration_tests`
//!   or `ui_tests`.

use super::integration_tests;

integration_tests! {
    // Clearing the terminal block list.
    test_clear,
    // New session should auto-focus the input.
    test_new_session_focuses_input,
    // Clicking a detected secret shows a tooltip.
    test_secret_tooltip_shows_on_click,
    // Long-line rendering in the terminal.
    test_with_long_line,
    // Block filtering with a long-running command (macOS-only at runtime).
    test_block_filtering_keybinding_with_long_running_command,
}
