# Narrowing what the views see of `App`, run 5: the dialogs over a workspace

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `conflict_prompt`, `write_prompts` (Leave and the production confirmation), `agent_access` and `connect_dialog` take `ui::Paint`, their dialog's prompt and what they read beside it, instead of `&mut App`, as runs 1 to 4 did (`2026-10-31-narrow-app-access.md`, "The pattern"). Behaviour does not change.

**Files:**
- Modify: `src/ui/conflict_prompt.rs`, `src/ui/write_prompts.rs`, `src/ui/agent_access.rs`, `src/ui/connect_dialog/mod.rs`
- Modify: `src/ui/mod.rs` (`ui::show` finds each prompt and what it reads, calls the view and extends `app.actions`)
- Modify: `src/app.rs` (`workspace_in`, which `App::workspace` now calls)

## Survey (2026-10-31, `main` at `025eb58f`)

| View | What it read of `App` | Now takes |
|---|---|---|
| `conflict_prompt::show` | Paint, `dialog` (`ConflictPrompt`, its `fresh` written), `workspace(prompt.tab)`: its `name` and the object tab | `ctx, paint, &mut ConflictPrompt, Option<&Workspace>` |
| `write_prompts::show` | both prompts below, one after the other | gone: `ui::show` calls each prompt for its own `Dialog` |
| `write_prompts::leave` | Paint, `dialog` (`LeavePrompt`) | `ctx, paint, &LeavePrompt`, public |
| `write_prompts::confirm_write` | Paint, `dialog` (`WritePrompt`, its typed name and focus written), `workspace(prompt.tab)`: `name`, `target`, `environment`, `driver`, the object tab's review | `ctx, paint, &mut WritePrompt, Option<&Workspace>`, public |
| `agent_access::show` | Paint, `dialog` (`AgentAccessBox`), `connections` and `agents_access` for the level in force | `ctx, paint, &AgentAccessBox, current: Option<AgentAccess>` |
| `connect_dialog::show` | Paint, `dialog` (`ConnectionForm`, edited), `keymap`, `layout()`, `loaded_tables` while the form lists names | `ctx, paint, &mut ConnectionForm, &Keymap, Option<LoadedTables>`; `lists_names(&ConnectionForm)`, `fold(ctx)` |

The main plan's survey has these right. Two writes of the prompt that are not a control's were already the view's and stay so: `conflict_prompt` marks the row's lines placed (`fresh = false`, the place it brings into view on the row's first frame, as a field reports its cursor), and `connect_dialog` keeps what was loaded on the form (`agents_loaded`) for its Agents group to read.

No dialog was more tangled than the plan assumed; nothing is deferred. `conflict_prompt.rs` and `write_prompts.rs` are large for their tests and their drawing; only `show`'s head and tail moved.

## The slice of a dialog over a workspace

What runs 6 to 8 and the routers' plan reuse:

- **The slice is the dialog's prompt**, found by its caller with `if let Some(Dialog::…(prompt)) = &mut app.dialog`: `&mut` where the view writes it (a field's text or focus, the marks above), `&` otherwise. Never `Option<Dialog>`. A module that draws two dialogs gives each its own function (`write_prompts::leave`, `confirm_write`), and the caller matches each variant where the module was called.
- **The workspace it asks about comes as `Option<&Workspace>`**, `None` where it was closed meanwhile. The view finds the object tab by the prompt's own ids (`prompt.tab`, `prompt.id`: a dialog carries what it asks about), and answers a workspace, tab or row that is gone as it did (Close, Cancel): that answer is the view's, as `every_row_prompt`'s `editor_open`.
- **The prompt sits in `app.dialog` and the workspace in `app.tabs`**, so the caller borrows the two fields apart: `crate::app::workspace_in(&app.tabs, prompt.tab)` finds the workspace as `App::workspace` does (which now calls it) while the prompt is held `&mut`. `app.keymap` is lent the same way (`connect_dialog`).
- **A question that joins more of `App`** (`loaded_tables`, which reads every open window; `agents_access`, which reads what is held) is answered before the prompt is borrowed `&mut` and passed as its answer. Whether the view needs a costly answer at all is a question of its own slice (`connect_dialog::lists_names(&ConnectionForm)`), which the caller asks first, as `filter_bar::is_open`.
- **Work a dialog does on frames it is not open** is its own `fold(ctx)` (`connect_dialog::fold` forgets where the dialog's top edge settled), which the caller calls where the dialog is not that variant, as `about::fold`.

The order of a frame's actions is unchanged: `ui::show` calls the dialogs in the order it called their modules, and each extends `app.actions` where its view did. `Paint::of(app)` is taken before the connection dialog where it was taken after it; nothing between the two changes the locale, palette or look.

## Tasks

### Task 1: `agent_access`

- [x] `show` takes `ctx, paint, &AgentAccessBox, current` and returns its actions; a connection that is gone (`current` is `None`) closes the box, as before. `ui::show` asks `app.agents_access` of the saved connection.

### Task 2: `conflict_prompt`

- [x] `app::workspace_in(&[ConnTab], ConnTabId)`; `App::workspace` calls it.
- [x] `show` takes `ctx, paint, &mut ConflictPrompt, Option<&Workspace>` and returns the answers it took, none before they are ripe or for an early press, as before.

### Task 3: `write_prompts`

- [x] `show` goes; `leave` takes `ctx, paint, &LeavePrompt`, `confirm_write` takes `ctx, paint, &mut WritePrompt, Option<&Workspace>`, each returning its actions. `confirm_write` holds the prompt `&mut` from its start, where it took it `&` and again `&mut` after reading the workspace.

### Task 4: `connect_dialog`

- [x] `show` takes `ctx, paint, &mut ConnectionForm, &Keymap, Option<LoadedTables>` and returns its actions, with the columns it wants loaded first, as before. `lists_names` and `fold` split off. `ui::show` asks `app.loaded_tables` of the form's connection only when `lists_names` is true.

### Task 5: CI's checks and the PR

- [x] `cargo fmt --all --check`
- [x] `cargo clippy --locked --workspace --all-targets -- -D warnings`
- [x] `cargo test --locked --workspace --all-targets`
- [x] Push and open the PR.

The net is the existing tests, unchanged: `conflict_prompt::tests`, `write_prompts::tests`, `src/ui/dialog_focus_tests.rs`, `env_tests.rs`, `delete_tests.rs`, `duplicate_tests.rs`, `agents_tests.rs`, `start_tests.rs`, and the connection dialog's, Leave's, production confirmation's and agent access box's tests in `src/ui/mod.rs` and `src/app.rs`. No test called a function whose signature moved.

## For the runs after

- Run 6 (`history`, the SQL panes, `object_tabs`) is unaffected: none reads `app.dialog`.
- Run 7 (`data_view`, `row_panel`) and run 8 (`quick_open`) read `dialog` beside a workspace: they borrow the two apart through `workspace_in`, as above, where one is `&mut`.
- The routers' plan inherits `ui::show`'s dialog calls, and `workspace_in` for a router that holds one field of `App` `&mut` beside another.
