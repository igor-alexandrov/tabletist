# Narrowing what the views see of `App`, run 4: the settings panes

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** the panes of the settings window that read `&App` (the file pane, Updates in both looks, Keyboard in both looks with the helpers they share, and the sheet's Agents pane) take what they read instead of the whole `App`, as runs 1 to 3 did (`2026-10-31-narrow-app-access.md`, "The pattern"). Behaviour does not change.

**Files:**
- Modify: `src/ui/settings/file_pane.rs`, `updates.rs`, `updates_terminal.rs`, `keyboard.rs`, `keyboard_sheet.rs`, `keyboard_terminal.rs`, `agents.rs`
- Modify: `src/ui/settings/mod.rs` (`update_status` and `server` build two panes' slices; `show` and `keys` call the panes' keys with the keymap and the update status)
- Modify: `src/ui/settings/sheet.rs`, `src/ui/settings/terminal.rs` (find each pane's slice and call it)

## Survey (2026-10-31, `main` at `1e06b2f5`)

| Pane | What it read of `App` | Now takes |
|---|---|---|
| `file_pane::show` | `settings_file`, `dirs` (through `path_shown`), `settings.keys.binds` | `ui, rect, &SettingsFile, path: &str, &[Bind], Shown, &Skin` |
| `keyboard::listed` | `keymap`, `layout()`, `locale` | `&Keymap, Layout, Locale, &KeyboardPage` |
| `keyboard::changed_count` | `keymap`, `layout()` | `&Keymap, Layout` |
| `keyboard::spoken_keys`, `spoken_defaults` | `keymap` (the first), `layout()`, `locale` | `&Keymap, Layout, Locale, Row`; `Layout, Locale, Row` |
| `keyboard::name`, `said`, `refusal` | `look`, `locale`, `keymap` (`said`) | `&Look, Locale, …`; `said` takes `&Keymap` first |
| `keyboard::record` | `keymap`, `layout()` | `ctx, &Keymap, Layout, (recording, on_buttons), actions` |
| `keyboard_sheet::content` | as `keyboard`'s helpers | `ui, &Keymap, &KeyboardPage, limit, &Skin, actions`; its parts take the keymap where they read it, the layout from the `Skin`'s look |
| `keyboard_terminal::rows` | as `keyboard`'s helpers | `ui, &Keymap, &KeyboardPage, &Skin, actions` |
| `keyboard_terminal::cursor`, `keys` | as `keyboard::listed` | `&Keymap, Layout, Locale, &KeyboardPage`; `keys` takes `ctx` first, `dialog` and `actions` after |
| `updates::Card::of`, `offered`, `installs`, `skips`, `install_action`, `managed_command` | `updates`, `settings.update_skip` | `updates::Status` |
| `updates::ready_said` | `updates` | `&Updates` |
| `updates::checked_text` | `updates`, `now_secs()`, `look`, `locale` | `Status, &Look, Locale, terminal` |
| `updates::content` | `updates`, `settings.update_skip`, `now_secs()`, `work_in_progress()`, `shown(app)` | `ui, Status, &Settings, interrupted: impl FnOnce() -> Vec<Work>, limit, &Skin, actions` |
| `updates_terminal::rows` | as `updates::content` | `ui, Status, interrupted, &Settings, cursor, &Skin, actions` |
| `agents::content` (the sheet's) | `shown(app)`, `agents_on()`, `agents_window()`, `agents.sessions` | `ui, &Settings, agents::Server, Agent, limit, &Skin, actions` |

The main plan's survey listed the panes' drawing functions. Their queries the routers call (`updates::offered`, `skips`, `install_action`, `managed_command`, `checked_text`, `keyboard::changed_count`, `keyboard_terminal::cursor`) and the keys they take (`keyboard::record`, `keyboard_terminal::keys`) took `&App` too, and moved with them. The Agents pane also read `agents.sessions`; the file pane, `dirs` and `settings.keys.binds`.

No pane was more tangled than the plan assumed; nothing is deferred.

## The slice of a settings pane

What the routers' plan inherits:

- **A settings pane keeps its window's `Skin`** (`sheet::Skin`, `terminal::Skin`), not `Paint`. The Skin is the settings window's own Paint: it holds the look, palette and locale by reference, with what the window's rows say beside them (the zones, the connections, the fonts). Building a `Paint` from it in each pane would copy the palette for nothing.
- **It keeps pushing into the window's `actions: &mut Vec<Action>`.** That is a local of `settings::show`, not `app.actions`, and every settings pane reads `App` through shared borrows, so it can be lent beside them: the reason the main plan has a view return its actions does not hold. The order of a frame's actions is unchanged because nothing about where they are pushed moved.
- **The slice is the field the pane reads** (`&Keymap`, `&SettingsFile`, `&[Bind]`), never `&App`. The layout comes from the Skin's look (`Layout::of(look)`) where a pane draws, and as an argument where a router asks a query (`cursor`, `changed_count`, `record`, `keys`), as `App::layout()` gave it.
- **The settings as the window shows them** (`settings::shown(app)`, which joins the settings with `agents_on`, the look and the layout) are answered by the router and passed as `&Settings`, as the panes without `&App` (General, Appearance, Editor) already took them.
- **Several answers that join a field with the rest of `App`** go in a struct of the pane's own, built by a function of `settings/mod.rs` beside `shown` and `path_shown`: `updates::Status { updates, skip, now }` (`update_status(app)`), `agents::Server { on, connected, window }` (`server(app)`). The pane's queries take the same struct, so the routers ask them with it.
- **A list asked only in one state** comes as a closure the pane calls then: `interrupted`, `App::work_in_progress`, called only while the card is Held, as before (the `review::show` `whole` of run 3).

Two small differences in how often an answer is asked, neither seen: `agents_window()` is asked once a frame for the Agents pane where it was asked twice, and the Updates card and its Last checked row share one `now_secs()` where each read the clock. The tests stop the clock.

## Tasks

### Task 1: the Keyboard pane

- [x] `keyboard.rs`'s helpers take the keymap, layout, look and locale they read.
- [x] `keyboard_sheet::content` and `keyboard_terminal::rows`, `cursor` and `keys` take `&Keymap`. `keyboard_terminal::keys` borrows the keymap where it cloned the `Arc`.
- [x] `settings::show` calls `record` and `keys` with `&app.keymap` and `app.layout()`; `sheet.rs` and `terminal.rs` call the rest and `changed_count` the same way.

### Task 2: the Updates pane

- [x] `updates::Status` and `settings::update_status`; `Card::of` and the pane's queries take it.
- [x] `updates::content` and `updates_terminal::rows` take the status, the settings shown and `interrupted`.
- [x] `settings::keys`, the sheet's and the screen's nav, and the screen's `checked` call the queries with `update_status(app)`.

### Task 3: the Agents pane and the file pane

- [x] `agents::Server` and `settings::server`; `agents::content` takes the settings shown and the server.
- [x] `file_pane::show` takes the file, its path as the window writes it and the settings' binds; `terminal.rs` passes `&app.settings_file`, `path_shown(app)` and `&app.settings.keys.binds`.

### Task 4: CI's checks and the PR

- [x] `cargo fmt --all --check`
- [x] `cargo clippy --locked --workspace --all-targets -- -D warnings`
- [x] `cargo test --locked --workspace --all-targets`
- [x] Push and open the PR.

The net is the existing tests, unchanged: `src/ui/keyboard_tests.rs`, `update_settings_tests.rs`, `agent_settings_tests.rs`, the other `*_settings_tests.rs` (which draw the terminal look's screen too, the file pane beside its rows: `editor_settings_tests::the_file_pane_marks_the_old_keys_line_for_the_row_it_sets` reads it), and `updates::tests`. No test called a function whose signature moved.

## For the runs after

- Runs 5 to 8 are unaffected: no view of theirs calls a settings pane.
- The routers' plan inherits `settings/mod.rs`'s `shown`, `path_shown`, `update_status` and `server`, and the two Skins. Still on `&App` in the settings window: `sheet.rs`, `terminal.rs` (with the terminal look's Agents rows, `agent_rows`, which `terminal.rs` draws itself) and `mod.rs`.
