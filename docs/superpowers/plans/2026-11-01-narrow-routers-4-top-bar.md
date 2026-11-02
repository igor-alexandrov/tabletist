# Narrowing what the routers see of `App`, run 4: the connection bar

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The connection bar takes `ui::Paint`, the tab, the workspace, what it shows of it (`BarInfo`), how the settings tint it and the title bar's inset and height instead of `&mut App`, and returns its actions. `workspace::show` builds the bar's `BarInfo` with `bar_info`, as before, lends the bar the workspace and extends `app.actions` after it. This follows the routers' plan (`2026-11-01-narrow-routers.md`, run 4). Behaviour does not change.

**Files:**
- Modify: `src/ui/workspace.rs` (`show`, `top_bar`)
- Modify: `docs/superpowers/plans/2026-11-01-narrow-routers.md` (run 4)

## Survey (2026-11-01, `main` at `13eb5bd2`)

| Function | What it read of `App` | Now takes |
|---|---|---|
| `workspace::top_bar` | `workspace_mut(tab)` (took `database_menu_asked`); Paint; `bar_info(app, tab)`; `settings.env_bar`; `titlebar.inset` and `titlebar.height`; pushed what `mac_bar` or `terminal_bar` pushed | `ui, paint, tab, &mut Workspace, &BarInfo, EnvBar, (inset, height)`, returns its actions |
| `workspace::bar_info` | the workspace, `chips`, `keymap` with `layout()` | unchanged: a builder, called by `workspace::show` |
| `workspace::mac_bar`, `terminal_bar` | nothing of `App`: they took Paint's three fields, `&BarInfo` and the bar's colours already | unchanged |

`top_bar` took the one-time ask first, then built its `BarInfo` and returned where it found none. `bar_info` finds no `BarInfo` exactly where `workspace_mut` finds no workspace, so the ask was taken in every frame that drew the bar, and in none that did not.

Where the survey differs from the plan:

- **The bar takes the workspace beside its `BarInfo`.** The plan left open who takes `database_menu_asked`. The bar does, from a `&mut Workspace` (below), so it takes the workspace with the rest. It reads nothing else of it: what it shows comes in `BarInfo`.
- **The title bar's inset and height come as one pair,** `(inset, height)` in window points, beside egui's `zoom` as run 3 has it. As two arguments the bar would take eight, one over clippy's limit; `mac_bar` and `terminal_bar` allow theirs, but a pair names the title bar as one thing, as `app.titlebar` is.

## Who takes `database_menu_asked`

The bar takes it, from the `&mut Workspace` it is lent. That is the views' rule: a one-time request the workspace holds for a view is taken by the view that meets it (`focus_where` and `fold_documents` by the grid, the search's two flags by the start screen). Quick open's Switch database asks for the bar's own chip's menu, and the bar meets it: it opens `database_menu_id(tab, false)`, its own chip's id, which the chip draws in both looks. Taken by `workspace::show`, the request would be met by a router that knows nothing of the menu but its id, and the bar would be handed a `bool` to pass back to egui.

Taking it in the router would change nothing a frame does, since the bar took it first and in every frame it was drawn. The cost of the bar taking it is a `&mut Workspace` beside a `BarInfo` built from the same workspace: `workspace::show` builds the `BarInfo` from `&App` first and lends the workspace through `workspace_in_mut` after, as `start` does with `start_asked`.

## What this run teaches

- **A one-time ask is taken by the view that meets it, even where the router could take it without a change.** The view that answers the ask holds the id it answers with; the router stays dispatch.
- **Chrome that reads nothing of a workspace but a flag still takes the workspace** where the flag is the workspace's one-time ask, and what it shows comes in its answers. The answers are built before the workspace is lent `&mut`.
- **The bar's actions come before the notices',** as before: `workspace::show` extends `app.actions` after the bar and before it draws the notices, so the Connections button, the chips, the database menu and Disconnect still come before the notices' buttons in the order Tab moves.

## Tasks

### Task 1: the bar

- [x] `top_bar` takes `ui, paint, tab, &mut Workspace, &BarInfo, EnvBar, (inset, height)`, takes the workspace's `database_menu_asked` first as before, and returns what `mac_bar` or `terminal_bar` pushed.
- [x] `workspace::show` builds the `BarInfo` with `bar_info`, lends the bar the workspace through `workspace_in_mut`, and extends `app.actions` after it, before the notices. Where it finds no `BarInfo` it draws no bar, as `top_bar` did.

### Task 2: CI's checks and the PR

- [x] `cargo fmt --all --check`
- [x] `cargo clippy --locked --workspace --all-targets -- -D warnings`
- [x] `cargo test --locked --workspace --all-targets`
- [ ] Push and open the PR.

The net is the existing tests, which drive the whole `App`: the bar's in `src/ui/mod.rs` (`the_bar_has_a_chip_for_each_open_connection`, `a_chip_says_how_its_session_stands_while_it_is_not_connected`, `many_chips_share_the_bar_and_the_one_showing_stays_whole`, `the_top_bar_connections_button_opens_the_picker_in_a_new_tab`, `the_top_bar_disconnect_button_returns_to_the_picker`, `the_top_bar_says_the_connection_goes_through_ssh`, `the_top_bar_says_how_far_tls_can_be_trusted`, `the_top_bar_says_when_prefer_fell_back_to_plain_text`), `the_connection_bar_follows_the_setting` in `src/ui/env_tests.rs` for `env_bar`, `the_mac_window_buttons_centre_on_the_line_of_the_bar_that_leads` for the title bar's inset and height, the database menu's chord in `src/ui/start_tests.rs`, and the bar's shots in `src/shots.rs`. No test changed: no signature a test calls moved.

No committed test sends quick open's `ShowDatabaseMenu` through to the bar's chip: the chord in `start_tests.rs` opens the menu by its id from the keys. A throwaway test, not committed, applied `Action::ShowDatabaseMenu(tab)` to a workspace with two databases and drew a frame, in the terminal look and the macOS one: the chip's menu was open and listed the other database, and `database_menu_asked` was taken, before the change and after.

## For run 5

`workspace::show` now draws nothing with `&mut App` before the notices. The status line, drawn after them in the terminal look, is what run 5 narrows. It draws the command line itself, and `editing_status`, which builds the `Editing` both read, is called from `status_line` and `command_line` each.
