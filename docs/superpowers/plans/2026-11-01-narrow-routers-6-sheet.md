# Narrowing what the routers see of `App`, run 6: the settings sheet

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The settings window's sheet, which every look but the terminal's draws, takes its `Skin`, the dialog and what it asks of the app instead of `&App`. `settings::show` builds the skin with `sheet_skin` and the answers with `sheet_asked`, beside `shown`, `path_shown`, `update_status` and `server`, and lends the sheet its window's `actions` as before. The footer and the Keyboard pane's footer read the dialog they are handed where they read `app.dialog`. This follows the routers' plan (`2026-11-01-narrow-routers.md`, run 6). Behaviour does not change.

**Files:**
- Modify: `src/ui/settings/sheet.rs` (`show`, `footer`, `keyboard_footer`; new `Asked`)
- Modify: `src/ui/settings/mod.rs` (`show`; new `sheet_skin`, `sheet_asked`)
- Modify: `docs/superpowers/plans/2026-11-01-narrow-routers.md` (run 6, and the rustdoc check in the runs' checks)

## Survey (2026-11-01, `main` at `ceee9ec6`)

| Function | What it read of `App` | Now takes |
|---|---|---|
| `sheet::show` | built its `Skin` from `look`, `palette`, `locale`, `system_zone`, `zones`, `connections`, `installed_fonts` and `data_font_missing()`; `update_status(app)` for the nav's dot and again for the Updates pane; `settings_file.invalid`; `shown(app)` for the pane shown; `agents_window()` for Editor's Show History; `keymap` for the Keyboard pane; `server(app)` for the Agents pane; `work_in_progress()` in a closure the Updates pane calls while its card is held | `ctx, &Skin, &SettingsDialog, &Asked, actions` |
| `sheet::footer` | `app.dialog` for the history's question and the pane shown; `path_shown(app)`; took `asking` from `show` | `ui, rect, &SettingsDialog, &Asked, ignored, &Skin, actions` |
| `sheet::keyboard_footer` | `app.dialog` for `keyboard.resetting`; `keymap` with `layout()` for `changed_count` | `ui, line, &SettingsDialog, &Keymap, &Skin, actions` |
| `sheet::nav`, `content`, `ignored_lines`, `footer_height`, the rows' controls | nothing of `App`: they took the `Skin` and what they draw already | unchanged |

Where the survey differs from the plan:

- **The `Skin` is built in `settings/mod.rs` by `sheet_skin`.** `data_font_missing` is one of its fields already (`DataFonts::missing`), so it is answered there and not in the answers' struct.
- **The answers come in one struct, `sheet::Asked`,** with the two slices the sheet reads beside them: the keymap and the settings file's ignored lines. As arguments `show` would take eleven, and the footer, which reads the keymap (for the Keyboard pane's count of changes) and the path, nine. `Asked` holds `keymap`, `ignored`, `shown`, `server`, `status` (`update_status`), `path` (`path_shown`) and `interrupted` (`work_in_progress`).
- **`work_in_progress` stays asked only while the Updates card is held.** It comes as `interrupted: &dyn Fn() -> Vec<Work>`, a closure over `&App` made by `settings::show`, as the Updates pane took it since the views' run 4.
- **Show History's "no window" asks `server.window`,** which is `agents_window().is_some()`, the answer the Editor pane asked itself. `server` is built in every frame now, where it was built for the Agents pane alone; it asks `agents_on`, the sessions' count and `agents_window`, none of which reads egui's state.
- **The footer takes the dialog instead of `asking`.** It reads the dialog for the history's question and the pane shown, so it finds the pane that asks to reset (`dialog.resetting.then_some(dialog.pane)`) itself, as `show` did. `app.dialog` was the dialog `show` was handed (`settings::show` lends it from there), so its `_ => (None, General)` arm and `keyboard_footer`'s `_ => false` never matched, and go.
- **The Keyboard pane's footer finds the layout from the skin's look,** `Layout::of(look)`, which is what `App::layout` returns, as the Keyboard pane does since the views' run 4.

## What did not change

- **The order of actions.** Every part pushes into the window's `actions`, a local of `settings::show`, where it pushed, in the order it is drawn. `settings::show` extends `app.actions` after the sheet as before.
- **What is asked, and when, but for three answers built once a frame:** `update_status` was built twice on the Updates pane (the nav's dot, then the pane) and is built once, so `now_secs` is read once a frame where it was read twice; the tests stop the clock. `server` and `path_shown` are built in every frame where the Agents pane and the footer's path asked them. None of the three reads egui's state, so the frame is the same.
- **The focus order.** Nothing that is drawn moved.

## What this run teaches

- **A settings screen's answers and the slices it reads beside them can share one struct** where the screen and its footer would each pass more than clippy allows. The `Skin` keeps what every pane draws with; the struct holds what the router asks of the app for this frame.
- **An answer asked only in one state stays lazy inside the struct** as `&dyn Fn`, which the pane that takes `impl FnOnce` is handed as it is.
- **A part that read `app.dialog` to find the dialog its router was handed** takes that dialog, and the arms for another dialog go: they were unreachable.

## Tasks

### Task 1: the sheet

- [x] `sheet::Asked` holds the keymap, the ignored lines, `shown`, `server`, `status`, `path` and `interrupted`.
- [x] `sheet::show` takes `ctx, &Skin, &SettingsDialog, &Asked, actions`; `footer` takes the dialog and the answers; `keyboard_footer` takes the dialog and the keymap.
- [x] `settings::show` builds the skin with `sheet_skin` and the answers with `sheet_asked`, with `work_in_progress` in a closure, and calls the sheet.

### Task 2: CI's checks and the PR

- [x] `cargo fmt --all --check`
- [x] `cargo clippy --locked --workspace --all-targets -- -D warnings`
- [x] `cargo test --locked --workspace --all-targets`
- [x] `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps`
- [ ] Push and open the PR.

The net is the existing tests, which drive the whole `App` in the looks that draw the sheet: its footer's in `src/ui/mod.rs` (`the_settings_sheet_says_where_the_file_is_and_what_can_be_done_with_it`, `the_settings_sheet_counts_the_lines_the_app_ignores`, `the_settings_sheet_writes_the_files_path_from_the_home_directory`), the Keyboard pane's footer in `src/ui/keyboard_tests.rs` (`the_footer_links_to_the_vim_keys_and_exports_the_cheat_sheet`, `a_rows_reset_takes_its_lines_and_reset_all_every_table`), each pane's Reset to defaults (`theme_settings_tests.rs`, `appearance_settings_tests.rs`, `text_settings_tests.rs`, `editor_settings_tests.rs`), the history's questions in `src/ui/history_tests.rs` (`clear_history_clears_this_connection_and_settings_clears_every_one`, `while_clear_history_asks_only_its_answers_are_read`), the Updates pane's and its Work in progress in `src/ui/update_settings_tests.rs`, the Agents pane's in `agent_settings_tests.rs`, and the sheet's shots in `src/shots.rs`. No test changed: no signature a test calls moved.

## For run 7

The settings window's terminal screen, `terminal::show` with `agent_rows` and `keyboard_footer`, is what run 7 narrows. Its `Skin` is built inside `terminal::show` from `&App`, as the sheet's was; `settings/mod.rs` now has `sheet_skin` and `sheet_asked` beside the builders it will add.
