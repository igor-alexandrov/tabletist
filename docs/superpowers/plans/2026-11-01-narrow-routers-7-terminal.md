# Narrowing what the routers see of `App`, run 7: the settings terminal

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The settings window's terminal screen, which the terminal look draws over the whole window, takes its `Skin`, the dialog and what it asks of the app instead of `&App`. `settings::show` builds the skin with `terminal_skin` (with `font_shown` and `desktop_shown`, which were `FontShown::of` and `Desktop::of`) and the answers with `terminal_asked`, and lends the screen its window's `actions` as before. The Agents pane's rows take the server as the window says it, and the Keyboard pane's footer the keymap. This follows the routers' plan (`2026-11-01-narrow-routers.md`, run 7). Behaviour does not change.

**Files:**
- Modify: `src/ui/settings/terminal.rs` (`show`, `agent_rows`, `keyboard_footer`; new `Asked`; `FontShown::of` and `Desktop::of` go)
- Modify: `src/ui/settings/mod.rs` (`show`; new `terminal_skin`, `font_shown`, `desktop_shown`, `terminal_asked`)
- Modify: `docs/superpowers/plans/2026-11-01-narrow-routers.md` (run 7)

## Survey (2026-11-02, `main` at `7f8b97be`)

| Function | What it read of `App` | Now takes |
|---|---|---|
| `terminal::show` | built its `Skin` from `look`, `palette`, `locale`, `agents_window()`, `system_zone`, `named_zone`, `connections`, `launch_choices`, `checked_text(update_status(app))`, `Desktop::of`, `FontShown::of`; `shown(app)`; `notice`; `update_status(app)` again for the nav's dot and for the Updates pane; `keymap` for the Keyboard pane, with `layout()` and `locale` for the file's pane's row; `path_shown(app)`, `settings_file` and `settings.keys.binds` for the file's pane; `opened(app, cursor)` for the footer's Enter; `app.dialog` for the history's and `:update`'s questions; `work_in_progress()` in a closure the Updates pane calls while its card is held | `ctx, &Skin, &SettingsDialog, Asked, actions` |
| `terminal::agent_rows` | `agents_on()` and `agents.sessions.len()` for the status row | `ui, Server, &Settings, cursor, &Skin, actions` |
| `terminal::keyboard_footer` | `keymap` with `layout()` and `locale` for the cursor's row | `ui, &Keymap, &KeyboardPage, &Skin, actions` |
| `FontShown::of`, `Desktop::of` | `desktop_font`; `settings.theme`, `pin_state()`, `desktop_theme`, `themes` | `settings::font_shown`, `settings::desktop_shown` (builders of `settings/mod.rs`) |
| `terminal::notice` | nothing of `App` since run 1 | unchanged |
| the rest of `terminal.rs` (`header`, `nav`, the rows, `footer`, the questions) | nothing of `App`: they took the `Skin` and what they draw already | unchanged |

Where the survey differs from the plan:

- **The answers come in one struct, `terminal::Asked`,** as the sheet's do: `notice`, `keymap`, `file` (`settings_file`), `path` (`path_shown`), `binds`, `shown`, `server`, `status` (`update_status`), `opened` and `interrupted` (`work_in_progress`, a closure asked only while the Updates card is held). As arguments `show` would take fourteen.
- **`show` takes `Asked` by value.** What Enter on the cursor's row does (`opened`) is a `Vec<Action>`, which the footer takes whole for its button, and `Action` is not `Clone`. The sheet's `Asked` is lent; the terminal's is handed over, and the footer moves `opened` out of it.
- **`opened` is answered by `terminal_asked` for the cursor's row,** which it finds from the dialog as `show` does (`dialog.pane.rows().get(dialog.row)`). It was asked once a frame after the rows were drawn; it reads `agents_window()` alone, which nothing in the frame changes.
- **The `Skin` is built in `settings/mod.rs` by `terminal_skin`,** which borrows what On launch steps through and when updates were last checked. `settings::show` makes those two beside it, as `terminal::show` did, from the same `update_status` the answers hold.
- **The status row reads `server.on` and `server.connected`,** which `settings::server` builds from `agents_on()` and the sessions' count: the answer the pane asked itself. `server` is built in every frame now, where the Agents pane asked it alone.
- **The questions read the dialog `show` is handed,** where they read `app.dialog`. `settings::show` lends it from there, so the `_ => None` arms never matched, and go. `:update`'s version is lent as `&str`, where it was cloned.
- **The Keyboard pane's row and footer find the layout from the skin's look,** `Layout::of(look)`, which is what `App::layout` returns, and the locale from the skin.
- **`FontShown::of` and `Desktop::of` become `font_shown` and `desktop_shown`,** free builders beside `terminal_skin`, as the plan says: `terminal.rs` keeps the two structs and names `App` nowhere.

## What did not change

- **The order of actions.** Every part pushes into the window's `actions`, a local of `settings::show`, where it pushed, in the order it is drawn. `settings::show` extends `app.actions` after the screen as before.
- **What is asked, and when, but for three answers built once a frame:** `update_status` was built up to three times (the last check's words, the nav's dot, the Updates pane) and is built once, so `now_secs` is read once a frame; the tests stop the clock. `server` and `path_shown` are built in every frame where the Agents pane and a window wide enough for the file's pane asked them. None of the three reads egui's state, so the frame is the same.
- **The focus order.** Nothing that is drawn moved.

## What this run teaches

- **An answer that is not `Clone` and that a part takes whole** comes in the answers' struct handed by value. A closure that moves one field out of a struct still lends the others, so the parts keep reading them by reference.
- **A skin that borrows what its builder makes** (the launch choices, the last check's words) is built by a function that takes those borrows from its caller, which keeps them alive for the frame.

## Tasks

### Task 1: the terminal screen

- [x] `terminal::Asked` holds the notice, the keymap, the settings file, its path, the binds, `shown`, `server`, `status`, `opened` and `interrupted`.
- [x] `terminal::show` takes `ctx, &Skin, &SettingsDialog, Asked, actions`; `agent_rows` takes the server; `keyboard_footer` takes the keymap.
- [x] `settings::show` builds the answers with `terminal_asked`, with `work_in_progress` in a closure, and the skin with `terminal_skin`, `font_shown` and `desktop_shown`, and calls the screen.

### Task 2: CI's checks and the PR

- [x] `cargo fmt --all --check`
- [x] `cargo clippy --locked --workspace --all-targets -- -D warnings`
- [x] `cargo test --locked --workspace --all-targets`
- [x] `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps`
- [ ] Push and open the PR.

The net is the existing tests, which drive the whole `App` in the terminal look: the settings' tests in `src/ui/` (`agent_settings_tests.rs` for the Agents pane's status, Enter and access rows; `theme_settings_tests.rs` for the theme's rows and a pin's state; `text_settings_tests.rs` for the font's row; `update_settings_tests.rs` for the Updates pane, its last check and `:update`'s question; `keyboard_tests.rs` for the Keyboard pane's rows, its footer and the file's pane; `editor_settings_tests.rs` and `history_tests.rs` for the history's question; `appearance_settings_tests.rs`, `format_settings_tests.rs`, `typing_settings_tests.rs`, `indent_settings_tests.rs`, `preview_settings_tests.rs` and `settings_access_tests.rs`), the notice's on the settings screen in `src/ui/mod.rs`, and the terminal screen's shots in `src/shots.rs`. No test changed: no signature a test calls moved.

## For run 8

The keys' helpers in `src/ui/keys.rs` are what run 8 narrows. `settings::keys`, the settings window's router of keys, keeps `&App` by the plan's design, as does `settings::show`, which now only finds the dialog, routes the keys and builds what each screen is handed.
