# Narrowing what the views see of `App`, run 6: the workspace and SQL panes

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `object_tabs`, `sql_editor`, `sql_text`, `sql_results` and `history` take `ui::Paint`, the workspace or SQL tab they show and what they read beside it, instead of `&App`/`&mut App`, as runs 1 to 5 did (`2026-10-31-narrow-app-access.md`, "The pattern"). Behaviour does not change.

**Files:**
- Modify: `src/ui/object_tabs.rs`, `src/ui/history.rs`, `src/ui/sql_editor.rs`, `src/ui/sql_text.rs`, `src/ui/sql_results.rs`
- Modify: `src/ui/workspace.rs` (one private function per view finds its workspace and calls it: `object_tabs`, `sql_editor`, `history_tab`; the status line asks `status_summary` and `vim_mode` of the workspace it holds)
- Modify: `src/ui/keys.rs` (asks `card_key` and `link_keys` of the workspace), `src/ui/data_view.rs` (`Fit::of` with the settings and layout)
- Modify: `src/app.rs` (`workspace_in_mut`, which `App::workspace_mut` now calls)

## Survey (2026-10-31, `main` at `ddc4286e`)

| View | What it read of `App` | Now takes |
|---|---|---|
| `object_tabs::show` | Paint, `workspace(tab)`, `keymap` and `layout()` for New SQL tab's key | `ui, paint, tab, &Workspace, &Keymap` |
| `history::show` | Paint, `workspace(tab)` (its History tab, `driver`, `environment`, `spec`), `keymap`, `layout()`, `history_name`, `settings.history_keep` | `ui, paint, tab, &Workspace, &Keymap, name: &str, keep: u32` |
| `sql_editor::show` | Paint; calls its toolbar, footer, `sql_text::show` and `sql_results::show` with the `App` | `ui, paint, tab, id, &mut Workspace, &Settings, &Keymap` |
| `sql_editor::toolbar` | Paint, `workspace(tab)` (its SQL tab, `run_mode`, `sql_writes`), `settings.run_scope` | `ui, paint, tab, id, &Workspace, &Settings`, returns its actions |
| `sql_editor::footer` | Paint, `workspace(tab)` (its SQL tab, `server_version`), `vim_mode` | `ui, paint, tab, id, &Workspace, &Settings` |
| `sql_editor::vim_mode` | `settings`, `look`, `workspace(tab)`'s SQL tab in front | `ctx, &Look, tab, &Workspace, &Settings` |
| `sql_editor::status_summary` | `locale`, `workspace(tab)`'s SQL tab in front | `&Workspace, Locale` |
| `sql_text::show` | Paint, `settings`, `workspace_mut(tab)`: `driver`, `pane`, the SQL tab (its text and cursor edited) | `ui, paint, tab, &mut SqlTab, &Settings, Dialect, tree_has_arrows: bool` |
| `sql_results::show` | Paint, `keymap`, `settings`, `workspace(tab)` (`zone_shown`, `driver`, `pane`, `full_precision`, its SQL tab), `data_view::Fit::of(workspace, app)` | `ui, paint, tab, &Workspace, &SqlTab, &Settings, &Keymap` |
| `sql_results::card_key`, `link_keys` | `look`, `workspace(tab)` | `&Look, tab, &Workspace` |
| `data_view::Fit::of` | `settings`, `layout()` | `&Workspace, &Settings, Layout` |

The main plan's survey had the five right, but for three things it did not list: `sql_results` reads the `keymap` and `history` the `layout()` (both for the keys they name), and `sql_editor::show` draws `sql_text` and `sql_results` itself, so it takes what both take. The queries the routers ask of these views (`status_summary` and `vim_mode` for the status line, `card_key` and `link_keys` for the keys) moved with them, and `data_view::Fit::of`, which `sql_results` and `data_view` share, takes the two fields of `App` it read.

No view was more tangled than the plan assumed; nothing is deferred. `sql_results.rs` and `sql_text.rs` are large for their drawing and tests; only `show`'s head and tail moved, and the inner functions already took a `Place` and an `Env` (`sql_results`) or a `Shown` (`sql_text`) built from the fields.

## The slice of a SQL editor

What runs 7 and 8 and the routers' plan reuse:

- **A SQL tab is a slice as an object tab is**: `&mut SqlTab` where its field edits the text and cursor in place (`sql_text`), `&SqlTab` otherwise. The pane names `sql.id` where it was given the id. One or two values of the workspace come as their own arguments (`sql_text`: the `Dialect`, whether the tree has the arrows); a pane that reads more takes `&Workspace` beside the `&SqlTab` (`sql_results`, as `structure`).
- **A view that draws others takes what they take**, and calls them in the order it did, extending its own `actions` with what each returns (`sql_editor::show`: toolbar, editor, results, splitter). Where one of them edits in place it takes `&mut Workspace` and lends the children `&mut SqlTab` and `&Workspace` one after the other.
- **`&mut Workspace` beside another field of `App`** (the settings and keymap the editor reads) is lent through `crate::app::workspace_in_mut(&mut app.tabs, tab)`, the `&mut` of run 5's `workspace_in`, which `App::workspace_mut` now calls.
- **A query a router asks of a view's state** (the status line's `status_summary` and `vim_mode`, the keys' `card_key` and `link_keys`) takes the workspace and the fields it reads; the router finds the workspace, and a missing one answers as before (`None`, no links).
- **A helper two views share that read `App`** (`data_view::Fit::of`) takes the fields it read (`&Settings` and the `Layout`), so a narrowed view and one that is not yet narrow both call it; the narrow one takes the layout from `Paint::layout`.
- **More than seven arguments to an inner function** go in a struct of the view's own (`sql_results::Results`: the workspace, SQL tab, settings and keymap `draw` reads), as the plan says of a view.

The order of a frame's actions is unchanged: each view returns what it pushed, in order, and its caller extends `app.actions` where the view did. The SQL editor's results still read the tab after its editor wrote to it in the frame.

## Tasks

### Task 1: `object_tabs`

- [x] `show` takes `ui, paint, tab, &Workspace, &Keymap` and returns its actions. `ui::workspace::object_tabs` finds the workspace, in both places the strip was drawn.

### Task 2: `history`

- [x] `show` takes `ui, paint, tab, &Workspace, &Keymap, name, keep` and returns the actions it asked; copying and its toast stay in the view. `ui::workspace::history_tab` asks `app.history_name(tab)` and the setting first.

### Task 3: the SQL editor

- [x] `app::workspace_in_mut(&mut [ConnTab], ConnTabId)`; `App::workspace_mut` calls it.
- [x] `sql_text::show` takes `ui, paint, tab, &mut SqlTab, &Settings, Dialect, tree_has_arrows` and returns its actions.
- [x] `sql_results::show` takes `ui, paint, tab, &Workspace, &SqlTab, &Settings, &Keymap` and returns its actions; `draw` takes them in `Results`. `card_key` and `link_keys` take `&Look, tab, &Workspace`; `keys` finds the workspace.
- [x] `data_view::Fit::of` takes `&Workspace, &Settings, Layout`; `data_view` and `workspace` pass `&app.settings, app.layout()`.
- [x] `sql_editor::show` takes `ui, paint, tab, id, &mut Workspace, &Settings, &Keymap` and returns the toolbar's, editor's, results' and splitter's actions in that order; `toolbar` and `footer` take the workspace and settings. `vim_mode` and `status_summary` take the workspace; the status line passes the one it holds. `ui::workspace::sql_editor` lends the workspace through `workspace_in_mut` beside `app.settings` and `app.keymap`.

### Task 4: CI's checks and the PR

- [x] `cargo fmt --all --check`
- [x] `cargo clippy --locked --workspace --all-targets -- -D warnings`
- [x] `cargo test --locked --workspace --all-targets`
- [x] Push and open the PR.

The net is the existing tests: `object_tabs::tests`, `sql_editor::tests`, `sql_text::tests`, `sql_results::tests`, `sql_complete::tests`, `src/ui/history_tests.rs`, `editor_results_tests.rs`, `editor_settings_tests.rs`, `complete_tests.rs`, `vim_tests.rs`, `pane_keys_tests.rs`, and the editor's, status line's and tab strip's tests in `src/ui/mod.rs` and `src/app.rs`. Three test calls moved with their signatures: `sql_results::tests` calls `card_key` twice and `Fit::of` once, `sql_editor::tests` `status_summary` once, each now with the workspace.

## For the runs after

- Run 7 (`data_view`, `row_panel`): `data_view` already calls `Fit::of` with the fields; it moves to `Paint::layout` with the view. `row_panel` shows a row of a SQL editor's result too (`Workspace::row_panel_tab`), so it takes the `SqlTab` as `sql_results` does, and lends a `&mut` workspace beside `app.settings`, `app.keymap` and `app.dialog` through `workspace_in_mut` where it edits.
- Run 8 (`sidebar`, `agents`, `start`, `picker`, `quick_open`) is unaffected: none draws these views, and `agents` calls only `sql_text::highlighted`, which never took `App`.
- The routers' plan inherits `ui::workspace`'s three new functions, the status line's `status_summary` and `vim_mode` calls, and `keys`' `card_key` and `link_keys` lookups.
