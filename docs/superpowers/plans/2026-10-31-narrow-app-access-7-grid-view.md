# Narrowing what the views see of `App`, run 7: the grid's view and the row panel

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `data_view` (its header, toolbar, footer and grid) and `row_panel` take `ui::Paint`, the workspace or object tab they show and what they read beside it, instead of `&App`/`&mut App`, as runs 1 to 6 did (`2026-10-31-narrow-app-access.md`, "The pattern"). Behaviour does not change.

**Files:**
- Modify: `src/ui/data_view.rs` (`header`, `toolbar` and its `where_line`, `footer`, `show` and its `error_line`; `Around`, what the grid's and the row panel's editors read of the rest)
- Modify: `src/ui/row_panel.rs` (`show` and `draw`)
- Modify: `src/ui/workspace.rs` (one private function per view finds its workspace and calls it: `header`, `toolbar`, `footer`, `data_grid`, `inspector`)

## Survey (2026-11-01, run 6's branch at `9e0cb729`)

| View | What it read of `App` | Now takes |
|---|---|---|
| `data_view::header` | Paint, `workspace(tab)` (`environment`, `Table::of` for Add row's lock), its object tab | `ui, paint, tab, &Workspace, &ObjectTab` |
| `data_view::toolbar` | Paint, `workspace(tab)` (`full_precision`, its object tab); the terminal's WHERE line takes `focus_where` and edits the tab's raw clause | `ui, paint, tab, object_tab, &mut Workspace` |
| `data_view::footer` | Paint, `keymap` and `layout()` for Filter's key, `workspace(tab)` (`access`, `selection_note`, `Fit::of`), `settings` | `ui, paint, tab, &Workspace, &ObjectTab, &Settings, &Keymap` |
| `data_view::show` | Paint, `settings` (the grid's look, `Fit::of`), `dialog.is_none()`, `keymap` and `layout()` (the editor's keys, Cancel's), `calendar()`, `workspace(tab)` and then its object tab `&mut` (the cell's field) | `ui, paint, tab, object_tab, &mut Workspace, Around { settings, keymap, hold, calendar }` |
| `data_view::error_line` | Paint, `workspace(tab)`, its object tab, `key_picker::unmatched` | `ui, paint, tab, object_tab, &Workspace` |
| `row_panel::show` | Paint, `settings.value_tags`, `dialog.is_none()`, `keymap` and `layout()`, `calendar()`, `workspace_mut(tab)` (the editor taken out of the tab and put back, `fold_documents` taken), then `workspace(tab)` | `ui, paint, tab, id, &mut Workspace, data_view::Around` |

The main plan's survey had both right. It guessed that `row_panel` would take the `SqlTab` as `sql_results` does. It does not: the panel shows the row of the tab `id`, which is an object tab or a SQL editor (its own `source(workspace, id)` chooses), and writes the workspace around both (the editor it takes out of the object tab and puts back, the `za` fold it takes), so it takes `&mut Workspace` and the id, as `sql_editor` does.

Neither view was more tangled than the plan assumed, and nothing is deferred. `data_view.rs` and `row_panel.rs` are large because of their drawing and tests. Only the public functions' heads and tails moved, plus the two private functions that read `App` (`where_line`, `error_line`). The inner functions already took the workspace and the tab (`editor_target`, `search_of`, `Form::of`, `source`).

## What the grid teaches

What run 8 and the routers' plan reuse:

- **A view that edits one object tab in place and reads its workspace around it** (the grid's field, the WHERE line) takes `&mut Workspace` and the tab's id, as `sql_editor` does, and finds the tab itself: first `&` for what it reads of the workspace and the tab together, then `object_tab_mut` for the field. A `&mut ObjectTab` beside a `&Workspace` cannot be lent, since the workspace holds the tab.
- **A one-time request the workspace holds for a view** (`focus_where`, `fold_documents`) is taken by the view that meets it, as it was. Its caller does not take it, because it is taken only where the view meets it (the WHERE line only in the terminal look).
- **Two views that draw the same editor take the same struct** (`data_view::Around`: the settings, the keymap, whether a dialog has the keyboard, the calendar), built by each caller from `app.dialog.is_none()` and `app.calendar()` before the workspace is borrowed `&mut` through `workspace_in_mut`, with `&app.settings` and `&app.keymap` lent beside it.
- **A view whose workspace is gone is not called.** The terminal grid's error line used to pass over an id (`skip_ahead_auto_ids`) before the grid found its workspace gone. Now the caller does not call the grid, so nothing is drawn after the skipped id in that frame, and the change cannot be seen.
- **A local of the router may have the view's name** (`ui::workspace::show` keeps the panel's tab in `row_panel`), so the router's function for that view is named for what it draws (`inspector`, `data_grid`, which is not `ui::grid`).

The order of a frame's actions is unchanged: each view returns what it pushed, in order, and its caller extends `app.actions` where the view did. The row panel still queues what ends its field ahead of the rest.

## Tasks

### Task 1: `data_view`

- [x] `header` takes `ui, paint, tab, &Workspace, &ObjectTab` and returns its actions; Copy on the DDL view copies the tab it was handed.
- [x] `toolbar` takes `ui, paint, tab, object_tab, &mut Workspace` and returns its actions; `where_line` takes the workspace and the pair of ids.
- [x] `footer` takes `ui, paint, tab, &Workspace, &ObjectTab, &Settings, &Keymap` and returns its actions.
- [x] `show` takes `ui, paint, tab, object_tab, &mut Workspace, Around` and returns its actions; `error_line` takes the workspace.
- [x] `ui::workspace` gets `header`, `toolbar`, `footer` and `data_grid`, which take `Paint::of(app)` first, find the workspace, call the view and extend `app.actions`.

### Task 2: `row_panel`

- [x] `show` takes `ui, paint, tab, id, &mut Workspace, data_view::Around` and returns its actions, the ending first; it takes the editor out of the tab and puts it back, and `draw` takes the fold before it reads the workspace.
- [x] `ui::workspace::inspector` builds the `Around` and lends the workspace through `workspace_in_mut`, in both places the panel was drawn.

### Task 3: CI's checks and the PR

- [x] `cargo fmt --all --check`
- [x] `cargo clippy --locked --workspace --all-targets -- -D warnings`
- [x] `cargo test --locked --workspace --all-targets`
- [ ] Push and open the PR.

The net is the existing tests, none of which changed: `data_view::tests`, `row_panel::tests`, `src/ui/grid_keys_tests.rs`, `calendar_tests.rs`, `date_tests.rs`, `key_picker_tests.rs`, `delete_tests.rs`, `delete_many_tests.rs`, `duplicate_tests.rs`, `insert_audit_tests.rs`, `zone_tests.rs`, `format_settings_tests.rs`, `preview_settings_tests.rs`, `pane_keys_tests.rs`, `vim_tests.rs`, and the grid's, filter's, footer's and row panel's tests in `src/ui/mod.rs` and `src/app.rs`. No test called a function whose signature moved: the tests reach these views through `ui::show`, and they call only `row_panel`'s `field_stop`, `focused_field` and `field_box` and `data_view`'s `cell`, `plain_cell` and `Fit::of`, none of which took `App`.

## For the runs after

- Run 8 (`sidebar`, `agents`, `start`, `picker`, `quick_open`) is unaffected: none of them draws these views, and `keys` calls only `row_panel`'s `field_stop` and `focused_field`, which never took `App`.
- The routers' plan inherits `ui::workspace`'s five new functions, and the two `Around`s it builds from `app.dialog` and `app.calendar()`.
