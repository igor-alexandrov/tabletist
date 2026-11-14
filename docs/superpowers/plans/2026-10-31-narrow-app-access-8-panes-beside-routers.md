# Narrowing what the views see of `App`, run 8: the panes beside the routers

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `sidebar`, `agents`, `start` and `quick_open` take `ui::Paint`, the workspace or prompt they show and what they read beside it, instead of `&App`/`&mut App`, as runs 1 to 7 did (`2026-10-31-narrow-app-access.md`, "The pattern"). `picker` turns out to be a router and is left for the routers' plan. Behaviour does not change. This is the last view run.

**Files:**
- Modify: `src/ui/sidebar.rs` (`show`)
- Modify: `src/ui/agents.rs` (`show`; `Asked`, what the tab asks of the app; `Elsewhere::of` takes the saved connections)
- Modify: `src/ui/start.rs` (`show`, `status_line`, `keys`, `shown`; `Asked`)
- Modify: `src/ui/quick_open.rs` (`show`; `Asked`; `Window` made public for its caller)
- Modify: `src/ui/picker.rs` (`visible` takes the saved connections; `show` stays)
- Modify: `src/ui/workspace.rs` (one private function per view finds its workspace and calls it: `sidebar`, `start`, `agents_tab`; `start_asked` and `agents_asked` build the answers), `src/ui/mod.rs` (`quick_open_asked`, and the call), `src/ui/keys.rs` (`Elsewhere::of`), `src/app.rs` and `src/shots.rs` (`picker::visible`)

## Survey (2026-11-01, run 7's branch at `284f823c`)

| View | What it read of `App` | Now takes |
|---|---|---|
| `sidebar::show` | Paint, `settings.show_system_schemas`, `keymap` and `layout()` (the hints, New SQL query's, History's and Agents' keys), `agents_here`, `workspace(tab)` (the tree, the databases, the pane, the open tab), then `workspace_mut(tab)` for the terminal look's filter line and to take the tree's `reveal_cursor` | `ui, paint, tab, &mut Workspace, &Keymap, show_system: bool, agents: Option<AgentsHere>` |
| `agents::show` | Paint, `agents_here`, `workspace(tab)` (its Agents tab and log, `conn_id`, `environment`, `name`), `agents_name`, `connections` (the environment, whether saved, the logged call's connection), `agents_access`, `agents` (sessions here and elsewhere, calls), `agents_selected`, `keymap` and `layout()`, `settings.mcp_log_days` | `ui, paint, tab, &Workspace, &SavedConnections, &Keymap, Asked` |
| `agents::Elsewhere::of` | `connections.connections` | `&[SavedConnection], &SessionInfo` |
| `start::show` | Paint, `workspace(tab)` (driver, spec, tree, server version, pane), `bar_info`, `start_schema`, `start_recents`, `start_largest`, `agents_here`, `connections`, `keymap` and `layout()`, `start_matches`, `workspace_mut(tab)` (the search's two one-shot flags taken, the search written back), `updates.install`, `settings.grouped` | `ui, paint, tab, &mut Workspace, Asked, &Keymap` |
| `start::status_line` | Paint, `keymap` and `layout()` | `ui, paint, tab, &Keymap` |
| `quick_open::show` | Paint, `dialog` (its `QuickOpen`, edited), `workspace(here)` (whether it is open), `quick_open_list`, `quick_open_picked`, `keymap`, `layout()`, `open_connections` and `bar_info` (each window's name, environment and database), `agents_here`, `settings.grouped`, `quick_open_commands_query` | `ctx, paint, &mut QuickOpen, &Keymap, Option<Asked>` |
| `picker::visible` | `connections` | `&SavedConnections, search` |
| `picker::show` | Paint, `active_tab_id`, `active`, `tabs[active]` (its search edited), `connections`, `titlebar.inset`, `keymap`, `layout()`, `tab_showing` (every tab) and `agents_access` per row; draws `ui::notices(app, …)` between its header and its list | left on `&mut App` |

The main plan's survey had the five right, but for two things it did not list: `sidebar` writes the workspace (the terminal filter line edits the tree's filter, and the sidebar takes `reveal_cursor`), and `picker` calls `ui::notices`, which takes `&mut App`.

## What these views teach

What the routers' plan reuses:

- **A view that joins several of `App`'s parts takes one struct of answers, `Asked`**, built by its caller from `&App` before the slice is lent `&mut`: `start::Asked` (the bar's info, the schema, recents, largest, Agents, whether saved, the matches, the ready update, grouping), `agents::Asked` (Agents' state, the name an agent sees, the access in force, the sessions, calls and selection, the log's days), `quick_open::Asked` (the list, the pick, the windows, Agents, grouping, the commands' query). The builder is a private function of the router beside the call (`ui::workspace::start_asked`, `agents_asked`; `ui::quick_open_asked`), named for the view.
- **The answers a view joins with its own slice stay in the view.** The struct holds what needs `App`. What the view reads of the workspace beside it (the start screen's database name and object count, the Agents tab's environment and log) it still works out itself from its `&Workspace`. So `start::Asked` carries the schema (`start_schema`), and the view counts its objects in the tree.
- **A field of `App` that only lists data (`connections`) is lent as a slice of its own** (`agents::show`'s `&SavedConnections`), not answered, where the view looks up more than one thing in it. A helper that read only that field (`Elsewhere::of`, `picker::visible`) takes it.
- **A view whose window is gone answers that itself where it did** (quick open closes): its caller passes `Option<Asked>`, none where `workspace(here)` is none.
- **A struct whose field is a router's type** (`start::Asked`'s `BarInfo`, which is `pub(super)` in `ui::workspace`) gives that field `pub(super)` visibility, so the view's `show` stays `pub` without a private type in its interface.
- **A view that draws a router function between its own parts is a router.** `picker::show` draws its header, then `ui::notices(app, …)` (the held-agents bar and the notice, which take `&mut App`), then the search, footer and list beside them. Its paint order (backdrop, header, notices over the backdrop) rules out drawing the notices before it. It also asks `tab_showing` across every tab while it edits its own tab's search. So it stays on `&mut App`, as the root of a picker tab, the way `ui::workspace::show` is the root of a workspace.

The order of a frame's actions is unchanged: each view returns what it pushed, in order, and its caller extends `app.actions` where the view did. Quick open's close where its window is gone is the one action it returns then, as it pushed.

## Tasks

### Task 1: `sidebar`

- [x] `show` takes `ui, paint, tab, &mut Workspace, &Keymap, show_system, agents` and returns its actions; the terminal filter line edits `workspace.tree.filter`, and the view takes `reveal_cursor` at its end.
- [x] `ui::workspace::sidebar` asks `agents_here` and the setting, and lends the workspace through `workspace_in_mut` beside `app.keymap`.

### Task 2: `start`

- [x] `start::Asked`; `show` takes `ui, paint, tab, &mut Workspace, Asked, &Keymap` and returns its actions; `shown` takes the workspace and the answers; `keys` takes `&Keymap` and the `Layout`.
- [x] `status_line` takes `ui, paint, tab, &Keymap`.
- [x] `ui::workspace::start` builds the answers with `start_asked` and lends the workspace through `workspace_in_mut`.

### Task 3: `agents`

- [x] `agents::Asked`; `show` takes `ui, paint, tab, &Workspace, &SavedConnections, &Keymap, Asked` and returns its actions.
- [x] `Elsewhere::of` takes `&[SavedConnection]`; `keys` passes `&app.connections.connections`.
- [x] `ui::workspace::agents_tab` builds the answers with `agents_asked`.

### Task 4: `quick_open`

- [x] `quick_open::Asked`; `show` takes `ctx, paint, &mut QuickOpen, &Keymap, Option<Asked>` and returns its actions, `CloseDialog` alone where the window is gone.
- [x] `ui::show` builds the answers with `quick_open_asked` before it borrows the prompt.

### Task 5: `picker`

- [x] `visible` takes `&SavedConnections`; `App` and the shots pass `connections`.
- [x] `show` left on `&mut App` for the routers' plan (above).

### Task 6: CI's checks and the PR

- [x] `cargo fmt --all --check`
- [x] `cargo clippy --locked --workspace --all-targets -- -D warnings`
- [x] `cargo test --locked --workspace --all-targets`
- [ ] Push and open the PR.

The net is the existing tests: `src/ui/start_tests.rs`, `agents_tests.rs`, `quick_open_tests.rs`, `picker::tests`, `agents::tests`, `keyboard_tests.rs`, `pane_keys_tests.rs`, `vim_tests.rs`, and the sidebar's, tree's, picker's and quick open's tests in `src/ui/mod.rs` and `src/app.rs`. One call moved with its signature outside the tests: `src/shots.rs` calls `picker::visible` with `&harness.app.connections`. No test called a function whose signature moved.

## For the routers' plan

It inherits `ui::workspace`'s `sidebar`, `start` and `agents_tab` and their answer builders, `ui::quick_open_asked`, and `picker::show` itself. The main plan's closing section lists what still takes `App`.
