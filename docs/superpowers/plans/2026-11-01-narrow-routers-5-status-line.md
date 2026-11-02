# Narrowing what the routers see of `App`, run 5: the status line and the command line

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The terminal look's status line and its `:` prompt take `ui::Paint`, the tab, the workspace and what they ask of the app instead of `&mut App`. `editing_status`, which builds the `Editing` both read, takes `Paint`, the workspace and the answers to `lacking` and `save_blocked`. A new dispatcher of `workspace::show`, `bottom_line`, builds the answers, draws the prompt or the line and extends `app.actions` after the prompt. This follows the routers' plan (`2026-11-01-narrow-routers.md`, run 5). Behaviour does not change.

**Files:**
- Modify: `src/ui/workspace.rs` (`show`, `editing_status`, `command_line`, `status_line`; new `bottom_line`, `status_asked`, `StatusAsked`, `editing_asked`, `EditingAsked`, `shown_log`)
- Modify: `docs/superpowers/plans/2026-11-01-narrow-routers.md` (run 5)

## Survey (2026-11-01, `main` at `0eb617ef`)

| Function | What it read of `App` | Now takes |
|---|---|---|
| `workspace::status_line` | Paint; `keymap` with `layout()` (the long `key(Command::…)` head, `key_in`, `together`, `pair`); `workspace(tab)` six times; `agents_here(tab)` while the Agents tab is in front; `connections.get` for the workspace's connection and for the selected log call's; `agents.listed`; `structure_label(tab, id)` on a Structure view; `settings` for `vim_mode` and the columns' `Fit`; called `command_line(app, …)` while the prompt is open; called `editing_status(app, tab)`. Pushed no action. | `ui, paint, tab, &Workspace, &Keymap, &Settings, StatusAsked`, returns nothing |
| `workspace::command_line` | Paint; `editing_status(app, tab)`; `workspace_mut(tab)` (took `focus_command`, edited `command`); extended `app.actions` | `ui, paint, tab, &mut Workspace, &Editing`, returns its actions |
| `workspace::editing_status` | Paint; `workspace(tab)`; `lacking(tab, id)`; `save_blocked(tab, id)` while anything is pending | `paint, &Workspace, EditingAsked` |
| `start::status_line` | nothing of `App`: it took `Paint` and `&Keymap` already | unchanged |

`status_line` is long (about 930 lines) but almost all of it draws from what its head computes. What it read of `App` sits in that head and in five places below it, so the run is one PR: the keys' head did not need a run of its own.

Where the survey differs from the plan:

- **The line takes `&Settings` too.** Whether the editor's keys are vim's (`vim_mode`) and how the columns in view are fitted (`data_view::Fit::of`) come from the settings. Both take `&Settings` already, so the line takes it as the seventh argument instead of copying its fields into the answers.
- **A dispatcher, `bottom_line`, chooses between the prompt and the line.** `status_line` drew `command_line` itself while the prompt was open. The prompt edits its line and takes `focus_command`, so it needs `&mut Workspace`, and the line takes `&Workspace`. The router finds the workspace once, and draws one or the other, as `status_line` did. `show` calls `bottom_line` where it called `status_line`.
- **`StatusAsked` holds the editing answers as well.** The line builds its `Editing` itself, after the start screen's early return, so it is handed `EditingAsked` inside its answers. The prompt is handed the `Editing` built beside it.
- **What the line reads of the connections is two answers:** whether the workspace's connection is saved (`saved`), and whether the selected log call's connection is (`call_saved`). What it reads of `agents` is `listed`, and `agents_here` comes as `agents`. `structure_label` comes as `labels`, the one thing the line asked of it. A helper, `shown_log`, finds the Agents tab's Log view and its selected call for the builder and for the line both.

## What did not change

- **The order of actions.** The status line pushed none. The prompt pushes `RunCommand`, `EscapeCommand` or `CloseCommand` into its own `Vec` as before, and `bottom_line` extends `app.actions` right after it, where `command_line` did. Nothing else is drawn between.
- **What is asked, and when.** `save_blocked` is asked only while something is pending, as before. `agents_here` only while the Agents tab is in front, and `structure_label` only on a Structure view. `lacking` was asked for every frame with a table in front, and still is. Each answer is built before the line is drawn now, where it was asked in the middle; none of them reads egui's state, so the frame is the same.
- **`focus_command`** is taken by the prompt from its `&mut Workspace`, the view that meets it, as AGENTS.md has it, and only in the frames the prompt is drawn.

## What this run teaches

- **A router's chrome that drew another part of itself hands that choice to the router** where the two parts borrow the workspace differently: the line `&`, the prompt `&mut`.
- **Answers that are filters of the line's own reads** (`agents_here` while the Agents tab is in front, `structure_label` on a Structure view) are built by the builder with the same filters, so the app is asked exactly what it was.
- **A helper both the builder and the chrome need** (`shown_log`) takes `&Workspace` and returns borrows of it: the builder answers what joins it with the connections, and the line reads the rest itself.

## Tasks

### Task 1: the prompt and the line

- [x] `editing_status` takes `paint, &Workspace, EditingAsked`; `editing_asked` builds the answers to `lacking` and `save_blocked` for the table in front.
- [x] `command_line` takes `ui, paint, tab, &mut Workspace, &Editing` and returns its actions.
- [x] `status_line` takes `ui, paint, tab, &Workspace, &Keymap, &Settings, StatusAsked`; `status_asked` builds the answers; `shown_log` finds the Log view's call.
- [x] `bottom_line` draws the prompt while it is open and the line else, and `workspace::show` calls it.

### Task 2: CI's checks and the PR

- [x] `cargo fmt --all --check`
- [x] `cargo clippy --locked --workspace --all-targets -- -D warnings`
- [x] `cargo test --locked --workspace --all-targets`
- [ ] Push and open the PR.

The net is the existing tests, which drive the whole `App` in the terminal look: the mode line's and the line's in `src/ui/mod.rs` (`a_written_save_and_a_conflict_are_said_in_the_status_line`, `a_save_that_cannot_be_made_says_why_in_the_status_line`, `a_locked_cell_says_why_in_the_mode_line`), the foreign key picker's in `src/ui/key_picker_tests.rs` (`a_narrow_line_keeps_esc_close_when_the_keys_do_not_fit`, `a_new_rows_warning_is_said_by_the_line_the_bar_and_the_form`), the label chooser's in `src/ui/label_tests.rs` (`the_terminal_look_says_a_gone_column_in_its_status_line`, `the_terminal_look_confirms_a_choice_in_its_status_line`), the prompt's, which type `:` and a line into it (`an_unknown_command_says_so` in `src/ui/mod.rs`), the Agents tab's (`omarchy_shows_the_log_with_l_and_the_agents_log_line` in `src/ui/agents_tests.rs`), and the terminal look's shots in `src/shots.rs`. No test changed: no signature a test calls moved.

## For run 6

`workspace::show` now draws nothing with `&mut App` but its dispatchers and `bottom_line`, which is one of them. Runs 6 and 7 narrow the settings window's two screens, `sheet` and `terminal`.
