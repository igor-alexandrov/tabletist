# Narrowing what the routers see of `App` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The chrome the routers draw themselves (the notices bar, the picker's header and list, a workspace's connecting screens, connection bar and status line, the settings window's two screens, the keys' helpers) takes `ui::Paint` and the state it shows instead of `&App`/`&mut App`, one part at a time, as the views did. What stays on `App` is the routing itself: the root, each router's dispatch to the views and the reducer, and the functions that build a view's answers from `App`. Behaviour does not change.

**Architecture:** The pattern is the views' plan's (`2026-10-31-narrow-app-access.md`, "The pattern"), unchanged. A part of a router's chrome becomes a function that takes `Paint`, the ids it names, the slice it reads and the answers that join that slice with more of `App`, and returns its actions. The router function that drew it keeps `&mut App`, finds the slice, builds the answers, calls the part and extends `app.actions` where the part did.

**Tech Stack:** Rust, egui (crmne fork), headless UI tests through `src/testing.rs`.

**Scope:** inside the `tabletist` crate only, as before. Splitting crates is not part of this work. `tests/layers.rs` already keeps the lower layer from naming `crate::ui` and `crate::app`. Nothing here changes what it checks.

---

## Survey (2026-11-01, `main` at `282438b6`)

Non-test code under `src/ui/` that still takes `&App` or `&mut App`. "Paint" stands for `app.locale`, `app.palette` and `app.look`; "actions" for `app.actions`; `layout()` for `App::layout`.

| Function | Reads or writes besides Paint and actions | App methods it calls | Kind |
|---|---|---|---|
| `mod.rs` `show` | `dialog` (each prompt lent), `keymap`, `tabs` | `active_tab`, `active_tab_id`, `loaded_tables`, `agents_access` | root |
| `mod.rs` `Paint::of` | | | builder |
| `mod.rs` `quick_open_asked` | `settings` | `workspace`, `quick_open_*`, `open_connections`, `agents_here`, `layout`; `workspace::bar_info` | builder |
| `mod.rs` `notices` → `agents_held`, `notice`, `notice_line` | `notice`, `agents_held`, `connections` | | chrome (two routers draw it) |
| `mod.rs` `agents_held_said` | `agents_held`, `connections` | | chrome (tests ask it) |
| `mod.rs` `window_buttons_line` | | `active_tab`; `workspace::bar_line` | root's query |
| `picker.rs` `show` | `tabs[active]` (its search edited), `connections`, `titlebar`, `keymap` | `active_tab_id`, `tab_showing`, `agents_access`, `layout`; draws `notices` | router + chrome |
| `workspace.rs` `show` | | `workspace`, `workspace_mut`; draws `notices` | router |
| `workspace.rs` `sidebar`, `start`, `agents_tab`, `object_tabs`, `sql_editor`, `history_tab`, `header`, `toolbar`, `footer`, `data_grid`, `inspector`, `ddl_footer`, `pending_bar`, `review`, `structure`, `ddl` | the view's slice, `keymap`, `settings`, `dialog` | the view's answers | dispatch |
| `workspace.rs` `start_asked`, `agents_asked`, `bar_info` | `connections`, `settings`, `updates`, `agents`, `keymap` | `start_*`, `agents_*`, `workspace`, `layout` | builder |
| `workspace.rs` `chips` | `keymap` | `open_connections`, `layout` | builder (of `bar_info`) |
| `workspace.rs` `top_bar_height`, `bar_line` | `look`, `titlebar` | | chrome |
| `workspace.rs` `opening`, `connecting`, `failure`, `banner` | one workspace | `workspace` | chrome |
| `workspace.rs` `top_bar` | `settings.env_bar`, `titlebar`; takes `database_menu_asked` | `workspace_mut`; `bar_info` | chrome |
| `workspace.rs` `editing_status`, `command_line` | one workspace; takes `focus_command`, edits `command` | `workspace`, `workspace_mut`, `lacking`, `save_blocked` | chrome |
| `workspace.rs` `status_line` (927 lines) | `agents`, `connections`, `keymap`, `settings`, one workspace | `agents_here`, `structure_label`, `layout`, `workspace` | chrome |
| `settings/mod.rs` `show` | `dialog`, `keymap` | `layout` | router |
| `settings/mod.rs` `keys` | `connections`, `keymap`, `named_zone`, `updates` | `shown`, `set_shown`, `update_status`, `opened` | router (the window's keys) |
| `settings/mod.rs` `shown`, `set_shown`, `path_shown`, `update_status`, `server`, `opened` | `settings`, `desktop_theme`, `dirs`, `updates`, `agents` | `agents_on`, `agents_window`, `now_secs`, `layout` | builder |
| `settings/sheet.rs` `show` | `connections`, `installed_fonts`, `keymap`, `settings_file`, `system_zone`, `zones` | `agents_window`, `data_font_missing`, `work_in_progress`; `shown`, `server`, `update_status` | chrome |
| `settings/sheet.rs` `footer`, `keyboard_footer` | `dialog`, `keymap` | `layout`; `path_shown` | chrome |
| `settings/terminal.rs` `show` | `connections`, `dialog`, `keymap`, `named_zone`, `settings`, `settings_file`, `system_zone` | `agents_window`, `layout`, `work_in_progress`; `shown`, `opened`, `path_shown`, `update_status` | chrome |
| `settings/terminal.rs` `notice` | `notice` | | chrome |
| `settings/terminal.rs` `agent_rows`, `keyboard_footer`, `FontShown::of`, `Desktop::of` | `agents`, `keymap`, `desktop_font`, `desktop_theme`, `themes`, `settings` | `agents_on`, `pin_state`, `layout` | chrome / builder |
| `keys.rs` `handle`, `letters`, `editing_keys`, `text_size` | `dialog`, `keymap`, `agents`, `connections` | `active_*`, `workspace`, `copy_text`, `flips`, `is_busy`, `agents_*`, `review_whole`, `layout` | router |
| `keys.rs` `switchable`, `selection_row`, `on_last_row`, `history_shown`, `agents_log`, `agents_view`, `agents_here`, `agents_shows`, `agents_lists` | one workspace, `agents` | `workspace`, `agents_here` | helper |
| `keys.rs` `delete_row`, `delete_selected`, `delete_count`, `history_command`, `history_chords`, `window_letter` | one workspace, `keymap` | `workspace`, `layout` | helper |

"Kind" sorts them:

- **root:** `ui::show`, the frame's root. It holds `&mut App` by design.
- **router:** it decides what is drawn, or which command a key is, and dispatches to the views or the reducer. `workspace::show`, `picker::show` (once its chrome is out), `settings::show` and `settings::keys`, `keys::handle` with `letters`, `editing_keys` and `text_size`.
- **dispatch:** a router's private function that finds a view's slice, builds its answers, calls it and extends `app.actions`. Run 8 of the views' plan made one per view on purpose.
- **builder:** it builds a view's answers from `&App` (`Paint::of`, the `*_asked` functions, `bar_info`, the settings' `shown`, `update_status`, `server`, `opened`). That is the glue the pattern asks for.
- **chrome:** a router draws it itself. This is what this plan narrows.
- **helper:** a function of the keys' router that reads one workspace. This plan narrows it where that makes the router read plainer.

## What stays on `App`, by design

Narrowing these would move `App` one call down and gain nothing. Each runs once a frame and touches most of `App` because its job is to:

- **`ui::show`**, the root: the picker or a workspace, then each dialog lent its prompt, then the toast.
- **Each router's dispatch to the views and the reducer:** `workspace::show` and its per-view dispatchers, `picker::show` once its parts are out, `settings::show` and `settings::keys`, `keys::handle`, `letters`, `editing_keys` and `text_size`. A key's meaning hangs on the dialog, the active tab, its view, what is busy and what is selected. The keys' router reads all of that and only pushes actions. A struct of every answer it asks would be `App` under another name.
- **The builders:** `Paint::of`, `quick_open_asked`, `start_asked`, `agents_asked`, `bar_info` and `chips`, the settings' `shown`, `set_shown`, `path_shown`, `update_status`, `server` and `opened`, and the builders the runs below add. They turn `&App` into what a view is handed.
- **`window_buttons_line`**, which the entrypoint asks of the root: which bar leads the window depends on the active tab.

When every run is done, `App` appears in `src/ui/` only in these.

## The pattern for a router's chrome

The views' pattern holds. Four points it leaves open are set here:

- **Chrome two routers draw is a function of `ui`, not of one router** (`ui::notices`). Each router passes what it reads beside the call and extends `app.actions` where the chrome used to: between its own parts, so a frame's actions keep their order.
- **Chrome that returns a measure returns it with its actions,** `(f32, Vec<Action>)`. The measure comes first, as the function's own answer did.
- **A helper that chrome of `ui` and a settings screen share** (`notice_line`) takes `Paint`. A settings screen's `Skin` holds references, so the screen builds a `Paint` from its skin's three fields where it calls the helper. Its other panes keep the skin.
- **The answers a router's chrome needs about every tab** (`picker`'s `tab_showing` for each row) are built by the router before the chrome's slice is lent `&mut`, as `quick_open::Asked`'s windows are.
- **A part keeps its place in the focus order.** Splitting a router into parts may move what only paints, and a widget that is neither focusable nor overlapped (egui gives such a widget no focus and no AccessKit node), but a focusable widget stays where it was among the others: Tab moves in the order they are made.

## Runs

Each run is one PR from `main`, and passes these:

- `cargo fmt --all --check`
- `cargo clippy --locked --workspace --all-targets -- -D warnings`
- `cargo test --locked --workspace --all-targets`

`tests/layers.rs` runs with the rest. Each run after the first writes its record beside this plan as `2026-…-narrow-routers-N-<name>.md`, re-surveying its functions first, since `main` moves. No run changes behaviour, so none adds a behaviour test. A test changes only where it calls a function whose signature moved.

1. **The notices bar** (this plan, below): `ui::notices`, `notice`, `agents_held`, `agents_held_said`, `notice_line`, and the settings terminal's `notice`. Two routers and a settings screen draw it, so it goes first: run 2 needs it narrow.
2. **The picker's chrome:** `picker::show` splits into its header (title, count, New connection) and its body (search, footer, list), both drawn after the notices. Only the header's frame (its fill, its rule and the area that drags the window) is drawn before them, with the backdrop: New connection is focusable and follows the notices' buttons in the focus order, so drawing it first would change the order Tab moves in. The body takes `&mut` the picker tab's state, `&SavedConnections`, `&Keymap` and an `Asked` holding each row's `Opening` (`tab_showing`, `agents_access`), built before the tab is lent. `picker::show` stays as a picker tab's router, as `workspace::show` is a workspace's: backdrop and header's frame, notices, header, body, in that order. (Run 2's survey found this; the plan first had the whole header before the notices.)
3. **A workspace's connecting screens:** `opening`, `connecting`, `failure` and `banner` take `Paint` and `&Workspace` (or `Option<&Workspace>` where they answer a closed one) and return their actions. `top_bar_height` and `bar_line` take the look and the title bar's height. `window_buttons_line` keeps `&App` and passes those two.
4. **The connection bar:** `top_bar` takes `Paint`, the tab, `BarInfo` (built by `bar_info` as now), `settings.env_bar` and the title bar's inset and height. Its one-shot `database_menu_asked` is taken by `workspace::show` where the bar is drawn, or by the bar from a `&mut Workspace`. The run chooses by the views' rule, that a one-time request is taken by the view that meets it, and says which it chose.
5. **The status line and the command line:** `status_line` takes `Paint`, the tab, `&Workspace`, `&Keymap` and a struct of answers built beside it by a builder of `ui::workspace`, `status_asked` (`agents_here`, `structure_label`, and what it reads of `agents` and `connections`). `command_line` takes `&mut Workspace` beside an `Editing` built by `editing_status`. `editing_status` takes `&Workspace` and the answers to `lacking` and `save_blocked`. The largest run. If its survey finds it too large for one PR, the status line's keys (the long `key(Command::…)` head) go first, as the help a `&Keymap` and a `Layout` give.
6. **The settings sheet:** `sheet::show`, `footer` and `keyboard_footer` take the sheet's `Skin` (built in `settings/mod.rs`), `&SettingsDialog` and the answers they read (`shown`, `server`, `update_status`, `path_shown`, `work_in_progress`, `data_font_missing`), in one struct where they pass seven arguments. They keep pushing into the window's `&mut Vec<Action>`, as the panes do.
7. **The settings terminal:** `terminal::show`, `agent_rows` and `keyboard_footer` the same way. `FontShown::of` and `Desktop::of` move into `settings/mod.rs` as builders.
8. **The keys' helpers:** the ones that read one workspace (`switchable`, `selection_row`, `on_last_row`, `history_shown`, `agents_log`, `agents_view`, `delete_row`, `delete_selected`, `delete_count`) take `&Workspace` or the tab's slice. `history_chords`, `history_command` and `window_letter` take `&Keymap` and a `Layout`, or the locale. `handle`, `letters`, `editing_keys` and `text_size` keep `&App`/`&mut App` (above). If the re-survey finds a helper that `handle` calls once and that reads more than one workspace, the helper stays.

---

## Run 1: the notices bar

**Files:**
- Modify: `src/ui/mod.rs` (`notices`, `notice`, `agents_held`, `agents_held_said`, `notice_line`)
- Modify: `src/ui/workspace.rs:47-58` (`show` calls `notices` with what it reads)
- Modify: `src/ui/picker.rs:201` (`show` calls `notices` the same way)
- Modify: `src/ui/settings/terminal.rs` (`notice` takes the skin and the notice; builds a `Paint` for `notice_line`)
- Modify: `src/app.rs` (five tests) and `src/ui/agents_tests.rs` (one test) that call `agents_held_said`

The net is the existing tests, which drive the whole `App`:

- the held bar's: `the_held_bar_says_what_changed_and_its_buttons_answer_it` in `src/ui/agents_tests.rs`; `a_change_made_outside_the_app_is_held_until_the_person_keeps_it` and the other held tests in `src/app.rs`; `agent_settings_tests.rs`;
- the notice's: `a_notice_stands_under_the_bar_that_leads_the_window`, `a_notice_is_solid_so_nothing_under_it_shows_through`, `a_notice_shows_until_dismissed` and `the_settings_screen_shows_a_notice_until_it_is_dismissed` in `src/ui/mod.rs`;
- the picker's and the connection bar's tests, which draw `notices` on every frame.

### Task 1: `notices` and what it draws

- [ ] **Step 1: Run the net before the change**

Run: `cargo test --locked -p tabletist notice` and `cargo test --locked -p tabletist held`
Expected: PASS.

- [ ] **Step 2: `notices` takes what it reads, and returns its height and actions**

```rust
pub(crate) fn notices(
    ui: &mut egui::Ui,
    paint: Paint,
    notice: Option<&str>,
    held: &[crate::agents_shown::Raised],
    saved: &[crate::connections::SavedConnection],
) -> (f32, Vec<Action>) {
    let mut actions = Vec::new();
    let height = agents_held(ui, paint, held, saved, &mut actions)
        + self::notice(ui, paint, notice, &mut actions);
    (height, actions)
}
```

The held bar's actions come before the notice's, as they reached `app.actions` before.

- [ ] **Step 3: The private parts push into the bar's `actions`**

`notice(ui, paint, notice: Option<&str>, actions) -> f32` no longer clones `app.notice`. `agents_held(ui, paint, held, saved, actions) -> f32` takes `Paint` apart on its first line and clones `held` with `held.to_vec()` for its buttons, where it cloned `app.agents_held`. Neither extends `app.actions`. `notice_line(ui, paint, message, actions)` reads `paint` where it read `app`.

- [ ] **Step 4: `agents_held_said` takes the locale, what is held and the saved connections**

```rust
pub fn agents_held_said(
    locale: crate::i18n::Locale,
    held: &[crate::agents_shown::Raised],
    saved: &[crate::connections::SavedConnection],
) -> Option<String> {
```

The body reads `held` where it read `app.agents_held`, and `saved` where it read `&app.connections.connections`.

### Task 2: the callers

- [ ] **Step 1: `workspace::show` and `picker::show` pass what the bar reads, and extend `app.actions` there**

```rust
    let (height, asked) = super::notices(
        &mut under,
        super::Paint::of(app),
        app.notice.as_deref(),
        &app.agents_held,
        &app.connections.connections,
    );
    app.actions.extend(asked);
    let below = header.bottom() + height;
```

In `workspace::show` the height is dropped (`let (_, asked) = …`). The picker extends `app.actions` where `notices` did: its own actions, gathered in a local, are extended at its end, after.

- [ ] **Step 2: The settings terminal's `notice` takes its skin and the notice**

```rust
fn notice(ui: &mut Ui, skin: &Skin, notice: Option<&str>, actions: &mut Vec<Action>) -> f32 {
```

It reads `skin.palette` where it read `app.palette`. It builds the `Paint` for `notice_line` from the skin's look, palette and locale. `terminal::show` calls it with `&skin` and `app.notice.as_deref()`.

- [ ] **Step 3: The six tests that call `agents_held_said` pass the three fields**

```rust
crate::ui::agents_held_said(app.locale, &app.agents_held, &app.connections.connections)
```

- [ ] **Step 4: Run the net again**

Run: `cargo test --locked -p tabletist notice` and `cargo test --locked -p tabletist held`
Expected: PASS, the same counts as Task 1's Step 1.

### Task 3: CI's checks and the PR

- [ ] **Step 1:** `cargo fmt --all --check`
- [ ] **Step 2:** `cargo clippy --locked --workspace --all-targets -- -D warnings`
- [ ] **Step 3:** `cargo test --locked --workspace --all-targets`
- [ ] **Step 4:** Write the run's record, `2026-11-01-narrow-routers-1-notices.md`. Commit the plan, the record and the change together, push, and open the PR against `main`.
