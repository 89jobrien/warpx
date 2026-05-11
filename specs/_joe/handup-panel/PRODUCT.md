# Handup Panel

## Summary

A left-panel section in Warp that surfaces project checkpoint data from the
handup SQLite database (`~/.ctx/handoffs/handup.db`), giving the user an
at-a-glance view of recent handup survey results across all projects. The
panel mirrors the structure and interaction model of the existing Handoff
panel.

## Behavior

1. The panel appears in the left sidebar under the same region as the
   Handoff panel. Its header reads "Handup" with a subtitle showing the
   total checkpoint count (e.g. "71 items").

2. A refresh button in the header reloads the data from disk. The button
   uses the same refresh icon as the Handoff panel.

3. On load, the panel queries `~/.ctx/handoffs/handup.db`, table
   `checkpoints`, and retrieves all rows. The query runs on a background
   thread; the panel shows "Loading..." until complete.

4. If the database file does not exist or cannot be opened, the panel
   shows an error state: "Error: <message>". It does not crash or
   prevent other panels from rendering.

5. If the database exists but contains zero rows, the panel shows
   "No handup checkpoints found."

6. Checkpoints are grouped by `project` (the `project` column). Each
   project group has a sub-header showing the project name, styled
   identically to the Handoff panel's project sub-headers.

7. Project groups are sorted alphabetically by project name.

8. Within each project group, checkpoints are sorted by `created_at`
   descending (most recent first). Only the latest checkpoint per
   project is shown by default (collapsed view). When a project group
   is expanded, all checkpoints for that project are visible.

9. Each checkpoint row displays:
   - A date badge showing the `generated` date (formatted as-is from
     the database, e.g. "2026-05-11"), in the position where the
     Handoff panel shows a priority badge.
   - The `recommendation` text, truncated to a single line in
     collapsed state.

10. Clicking a checkpoint row toggles its expanded state. When expanded,
    the full `recommendation` text is shown (word-wrapped, not
    truncated), along with the `cwd` path on a second line in subdued
    text.

11. Clicking a project sub-header toggles visibility of all checkpoints
    in that group (expand/collapse the group). This is independent of
    individual row expansion.

12. The panel is scrollable when content exceeds the available height.
    Scroll behavior (scrollbar style, colors) matches the Handoff panel.

13. All text colors, fonts, and spacing derive from the active theme and
    `Appearance`, consistent with the Handoff panel. No hard-coded
    colors.

14. The panel loads automatically when first rendered (same as Handoff
    panel: triggers load in `new()`).

15. The database path is resolved as `$HOME/.ctx/handoffs/handup.db`
    using `dirs::home_dir()`. It is not configurable.

16. The panel does not write to or modify the database. It is read-only.

17. If the database schema is unexpected (missing `checkpoints` table,
    missing columns), the panel shows an error state rather than
    panicking.
