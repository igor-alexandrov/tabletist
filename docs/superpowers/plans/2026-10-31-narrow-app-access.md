# Narrowing what the views see of `App` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Each view under `src/ui/` takes only the state it draws and edits (a dialog's own struct, an object tab, a workspace) instead of `&App` or `&mut App`, one view at a time, so that a view can no longer read or change the rest of the app and `ui` stops depending on `App` everywhere. Behaviour does not change.

**Architecture:** A narrowed view takes `ui::Paint` (the locale, palette and look every view draws with, copied out of `App`), the ids it names in its actions, and a borrow of its own slice of state; it returns the `Action`s it asks for. Its caller, which still holds `&mut App`, finds the slice, calls the view and extends `app.actions` with what came back. The callers are the routers (`ui::show`, `ui::workspace`, `ui::keys`, `ui::settings`): they keep `App` until everything under them is narrow, and are narrowed last, in a plan of their own.

**Tech Stack:** Rust, egui (crmne fork), headless UI tests through `src/testing.rs`.

**Scope:** inside the `tabletist` crate only. Splitting crates is not part of this work. The lower layer's upward dependencies (`edit` → `ui::format`/`ui::json_text`, `model` → `app::QuickItem` and `ui::format::FieldText`, `review` → `ui::format`, `typography` → `ui`) are removed only where one falls out of a view being narrowed.

---

## Survey (2026-10-31, `main` at `d1ed10aa`)

Non-test code only (inline `#[cfg(test)]` modules and comments left out). "Paint" stands for `app.locale`, `app.palette` and `app.look`, which nearly every view copies into locals on its first lines; "actions" for `app.actions`, which every view that asks for something extends at its end. `layout()` is `Layout::of(&app.look)`, a function of the look alone.

| View | Reads or writes besides Paint and actions | App methods it calls |
|---|---|---|
| `toast.rs` | (look and palette only, `&App`) | |
| `host_key_prompt.rs` | `dialog` (`HostKeyPrompt`) | |
| `password_prompt.rs` | `dialog` (`PasswordPrompt`, its field edited) | |
| `every_row_prompt.rs` | `dialog` (`ConfirmEveryRow`) | |
| `about.rs` | `dialog` (`About`) | |
| `help.rs` | `dialog` (`Help`), `keymap` | `layout` |
| `agent_access.rs` | `dialog` | `agents_access` |
| `filter_bar.rs` | one object tab (its filter bar edited) | `workspace`, `workspace_mut` |
| `row_form.rs` | one object tab's editor | `workspace_mut` |
| `pending_bar.rs` | one object tab | `workspace`, `lacking` |
| `structure.rs` | one object tab, `keymap` | `workspace`, `structure_label` |
| `ddl.rs` | one object tab | `workspace` |
| `review.rs` | one object tab | `workspace`, `review_whole` |
| `object_tabs.rs` | one workspace | `workspace`, `layout` |
| `conflict_prompt.rs` | `dialog`, one workspace | `workspace` |
| `write_prompts.rs` | `dialog`, one workspace | `workspace` |
| `connect_dialog/mod.rs` | `dialog`, `keymap` | `layout`, `loaded_tables` |
| `sql_text.rs` | one SQL tab, `settings` | `workspace_mut` |
| `sql_editor.rs` | one SQL tab, `settings` | `workspace` |
| `sql_results.rs` | one SQL tab, `settings`, `keymap` | `workspace` |
| `history.rs` | one workspace, `settings`, `keymap` | `workspace`, `layout`, `history_name` |
| `data_view.rs` | one object tab, `settings`, `keymap`, `dialog` | `workspace`, `layout`, `calendar` |
| `row_panel.rs` | one object tab, `settings`, `keymap`, `dialog` | `workspace`, `workspace_mut`, `layout`, `calendar` |
| `sidebar.rs` | one workspace, `settings`, `keymap` | `workspace`, `workspace_mut`, `layout` |
| `agents.rs` | one workspace, `agents`, `connections`, `settings`, `keymap` | `workspace`, `layout`, `agents_here`, `agents_name`, `agents_access`, `agents_selected` |
| `start.rs` | one workspace, `connections`, `updates`, `settings`, `keymap` | `workspace`, `workspace_mut`, `layout`, `agents_here`, `start_recents`, `start_largest`, `start_matches` |
| `picker.rs` | `tabs`, `active`, `connections`, `titlebar`, `keymap` | `tab_showing`, `agents_access`, `active_tab_id`, `layout` |
| `quick_open.rs` | `dialog`, `settings`, `keymap` | `workspace`, `layout`, `agents_here`, `quick_open_list`, `quick_open_picked`, `quick_open_commands_query` |
| `settings/file_pane.rs` | `settings`, `settings_file` (`&App`) | |
| `settings/updates.rs` | `updates`, `settings` (`&App`) | `now_secs` |
| `settings/updates_terminal.rs` | (`&App`) | `now_secs`, `work_in_progress` |
| `settings/keyboard.rs`, `keyboard_sheet.rs`, `keyboard_terminal.rs` | `keymap` (`&App`) | `layout` |
| `settings/agents.rs` | `agents` (`&App`) | `agents_window`, `agents_on` |
| `settings/sheet.rs` | `dialog`, `system_zone`, `zones`, `connections`, `installed_fonts`, `settings_file` (`&App`) | `data_font_missing`, `agents_window` |
| `settings/terminal.rs` | thirteen fields (`&App`) | `pin_state`, `agents_window`, `agents_on` |
| `settings/mod.rs` | eleven fields | `agents_on`, `agents_window`, `layout` |
| `workspace.rs` | the router of a workspace: titlebar, connections, settings, agents, keymap | `workspace` ×10, `layout` ×10, `open_connections`, `lacking`, `save_blocked`, `agents_here`, `structure_label` |
| `keys.rs` | the keys' router: keymap ×17, dialog ×10, agents, connections | `workspace` ×30, `layout`, `agents_*`, `active_*`, `copy_text`, `flips`, `is_busy` |
| `mod.rs` | the root: `notice`, `agents_held`, `connections` | `active_tab`, `active_tab_id` |

The views fall into four slices:

1. **Paint and a dialog's own struct.** `toast`, `host_key_prompt`, `password_prompt`, `every_row_prompt`, `about`, `help` (plus the keymap). The cheapest: nothing else is read.
2. **Paint and one object tab, workspace or SQL tab.** `filter_bar`, `row_form`, `pending_bar`, `structure`, `ddl`, `review`, `object_tabs`, `sql_text`, `sql_editor`, `sql_results`, `history`, then the large ones, `data_view`, `row_panel`, `sidebar`. Most only call `app.workspace(tab)` to find their slice.
3. **The settings window's panes**, which take `&App` and read many settings and states but change nothing.
4. **The routers**: `mod.rs`, `workspace.rs`, `keys.rs`, `settings/mod.rs`, `picker.rs`, `quick_open.rs`, `start.rs`, `agents.rs`. They need many slices at once and dispatch to the others.

### The glue methods views call

- **Finding a slice:** `workspace`, `workspace_mut` (66 calls from views), `active_tab`, `active_tab_id`, `active_workspace_tab`, `active_sql`, `active_object`, `tab_showing`. A narrowed view stops calling these: its caller does, and hands the view what they found.
- **Derived from the look:** `layout()`, which reads only `look`. A narrowed view asks `Paint::layout` instead (added with the first view that needs it, run 2's `help`).
- **Queries that join a slice with something else of `App`:** `structure_label` (a workspace and `labels`), `lacking`, `save_blocked`, `review_whole`, `flips` (an object tab's edits and page), `agents_here`, `agents_name`, `agents_selected`, `agents_access`, `agents_on`, `agents_window` (a workspace and `agents`, `connections`, `settings`), `history_name`, `calendar`, `loaded_tables`, `data_font_missing`, `pin_state`, `now_secs`, `work_in_progress`, `start_*`, `quick_open_*`, `is_busy`. Their caller computes them and passes the answer in. One that reads only the slice (`lacking`, `save_blocked`, `flips` read one object tab) may move onto the slice's type in `model` when its view is narrowed; one that reads more stays on `App`.

## The pattern

### A narrowed view's signature

```rust
pub fn show(ui: &mut egui::Ui, paint: Paint, tab: ConnTabId, object: &mut ObjectTab) -> Vec<Action>
```

- `ctx: &egui::Context` or `ui: &mut egui::Ui` first, as now.
- `paint: Paint` by value: it is `Copy`, and the views copied its three fields into locals already. The view's first line takes it apart, `let Paint { locale, palette, look } = paint;`, so the body below reads as it did.
- The ids the view names in its actions or egui ids (`tab: ConnTabId`, a `TabId` where the slice does not carry its own).
- The slice: `&T` for what the view only reads, `&mut T` where a control edits a value in place (a field's text, a checkbox, a dropdown's choice) or a field reports its cursor (the edits `AGENTS.md` allows a view). The slice is the smallest struct that holds what the view reads: a dialog's prompt (`&mut PasswordPrompt`), an `ObjectTab`, a `Workspace`, never `Option<Dialog>` or `ConnTab`.
- Anything else the view reads (the keymap, a setting, a query's answer) comes as its own argument, `&Keymap`, `&Settings` or the answer itself. When that makes more than seven arguments, the view takes a struct of its own named for what it shows (`FooView<'a>`), built by its caller.

### How a view asks for a change

It returns `Vec<Action>`, in the order it asked. It does not take `&mut Vec<Action>`: the caller usually finds the slice with `app.workspace_mut(tab)`, which borrows all of `App`, so `&mut app.actions` cannot be lent beside it. Most views already gathered a local `actions` and extended `app.actions` at their end; that local is now what they return. A view that asks for nothing returns nothing.

### How `App`'s side calls it

```rust
let paint = Paint::of(app);
if let Some(object) = app
    .workspace_mut(tab)
    .and_then(|workspace| workspace.object_tab_mut(object_tab))
{
    let asked = super::filter_bar::show(ui, paint, tab, object);
    app.actions.extend(asked);
}
```

`Paint::of(app)` is taken before the slice is borrowed. The caller extends `app.actions` where the view used to, so the order of actions in a frame is unchanged. A view whose slice is missing is not called; it used to return early, with nothing asked. A view that did something in the frames it was not shown (About folds its fonts' notices) gives that its own function, which the caller calls instead of the view (`about::fold`).

A query a view makes of its own state (`filter_bar::is_open`) takes the slice too, and the caller or test finds the slice first.

### The slice of an object-tab pane

Set by run 3, for runs 5 to 7:

- The slice is the `ObjectTab` the pane shows, `&mut` where a control edits it in place, `&` otherwise. The pane names `object.id` instead of an id it is given beside it.
- One or two values of the workspace around it come as their own arguments (`ddl`: the `Driver`; `review`: whether the connection is read-only). A pane that reads more of the workspace takes `&Workspace` beside the `&ObjectTab` (`structure`).
- A question that joins the tab with the rest of `App` (`structure_label`, `save_blocked`, `lacking`) is answered by the caller; several answers go in a struct of the pane's own (`pending_bar::Asked`).
- An answer too costly to give every frame, needed only when a button is pressed, comes as a closure the pane calls then (`review::show`'s `whole`).
- A panel that passes over an egui id while it is not drawn gives that its own `pass(ui)`, and says with `shows(&ObjectTab)` whether it is drawn, so its caller asks the answers only then (`pending_bar`).

### The slice of a settings pane

Set by run 4, for the routers' plan:

- A pane of the settings window keeps its window's `Skin` (`sheet::Skin`, `terminal::Skin`), which is that window's Paint, and keeps pushing into the window's `actions: &mut Vec<Action>`: a local of `settings::show`, lent beside the shared borrows of `App` the panes read through, so the reason a view returns its actions does not hold there.
- The slice is the field the pane reads (`&Keymap`, `&SettingsFile`, `&[Bind]`); the layout comes from the Skin's look where the pane draws, as an argument where a router asks a query.
- The settings as the window shows them come as `&Settings` (`settings::shown(app)`); several answers that join a field with the rest of `App` go in a struct of the pane's own built in `settings/mod.rs` (`updates::Status` by `update_status`, `agents::Server` by `server`), which the pane's queries take too; a list asked in one state only comes as a closure (`interrupted`).

### The slice of a dialog over a workspace

Set by run 5, for runs 7 and 8 and the routers:

- The slice is the dialog's prompt, `&mut` where the view writes it, `&` otherwise; a module of two dialogs gives each its own function, and the caller matches each `Dialog` variant.
- The workspace it asks about comes as `Option<&Workspace>`, `None` where it was closed; the view finds the object tab by the prompt's ids and answers one that is gone itself, as it did.
- The prompt sits in `app.dialog` and the workspace in `app.tabs`, so the caller borrows the two fields apart, the workspace through `crate::app::workspace_in(&app.tabs, tab)` (what `App::workspace` calls). A field such as `app.keymap` is lent beside the prompt the same way.
- A question joining more of `App` (`loaded_tables`, `agents_access`) is answered before the prompt is borrowed `&mut`; whether the view needs a costly one is a question of its own slice (`connect_dialog::lists_names`). Work on frames the dialog is not open is its `fold(ctx)`.

### Tests

The headless tests drive the whole `App` through `src/testing.rs`, so they hold behaviour across a run without change. A test changes only where it calls a function whose signature moved. No run changes behaviour, so no run adds a behaviour test; each run states which existing tests draw the views it moved.

## Runs

Each run is one PR, from `main`, passing `cargo fmt --all --check`, `cargo clippy --locked --workspace --all-targets -- -D warnings` and `cargo test --locked --workspace --all-targets`. Each later run gets its own detailed plan beside this one when it starts, as `2026-…-narrow-app-access-N-<name>.md`, re-surveying its views first: `main` moves.

1. **`Paint`, the password prompt and the filter bar** (this plan, below). One dialog that edits its prompt's field, one pane that edits an object tab's filter: both shapes of the pattern.
2. **The other small dialogs:** `host_key_prompt`, `every_row_prompt` (its caller passes whether the SQL tab is still open), `about`, `help` (adds `Paint::layout`, takes `&Keymap`), `toast` (reads only; returns nothing). Done: `2026-10-31-narrow-app-access-2-small-dialogs.md`.
3. **Object-tab panes:** `row_form` (`take_editor`/`put_editor` take `&mut ObjectTab`), `pending_bar` (its caller passes `lacking`, `save_blocked` and the doubts), `structure` (passes `structure_label`), `ddl`, `review` (its caller passes `review_whole` as a closure, called when Copy is clicked). Done: `2026-10-31-narrow-app-access-3-object-tab-panes.md`, which sets the borrow an object-tab pane takes ("The slice of an object-tab pane", above).
4. **Settings panes that read `&App`:** `settings/file_pane`, `settings/updates`, `settings/updates_terminal`, `settings/keyboard`, `keyboard_sheet`, `keyboard_terminal`, `settings/agents`. Done: `2026-10-31-narrow-app-access-4-settings-panes.md`, which sets the slice a settings pane takes ("The slice of a settings pane", below).
5. **Dialogs over a workspace:** `conflict_prompt`, `write_prompts` (Leave and the production confirmation, each its own function), `agent_access` (its caller passes `agents_access`), `connect_dialog` (passes `loaded_tables` and the keymap; `fold` for the frames it is not open). Done: `2026-10-31-narrow-app-access-5-workspace-dialogs.md`, which sets the slice a dialog over a workspace takes ("The slice of a dialog over a workspace", below).
6. **Workspace and SQL panes:** `object_tabs`, `sql_text`, `sql_editor`, `sql_results`, `history`.
7. **The grid's view:** `data_view`, then `row_panel`, each its own PR if either is large (they are 2.9k and 3.3k lines, and take `settings`, `keymap`, `dialog` and the calendar).
8. **Panes beside the routers:** `sidebar`, `agents`, `start`, `picker`, `quick_open`.

Then a plan of its own for the routers (`workspace.rs`, `keys.rs`, `settings/mod.rs`, `settings/sheet.rs`, `settings/terminal.rs`, `mod.rs`), written once the views under them are narrow: what they still need of `App` decides whether they stay its methods, take a context struct, or move into `app`.

---

## Run 1: `Paint`, the password prompt and the filter bar

**Files:**
- Modify: `src/ui/mod.rs` (add `Paint`; call the password prompt with its prompt)
- Modify: `src/ui/password_prompt.rs` (take `Paint` and `&mut PasswordPrompt`, return actions)
- Modify: `src/ui/filter_bar.rs` (take `Paint` and `&mut ObjectTab`, return actions; `is_open` takes `&ObjectTab`)
- Modify: `src/ui/workspace.rs:133-148` (find the object tab, call the filter bar)
- Modify: `src/ui/mod.rs` tests at the three `filter_bar::is_open` calls (signature moved)

The net is the existing tests: `password_prompt::tests`, the password tests in `src/app.rs` and `src/ui/mod.rs` (Connect with a password, Save in keyring, Esc), and the filter bar's (Cmd+F opens it with the keyboard in it, Apply, Clear, + Condition, Raw WHERE, Enter applies).

### Task 1: `Paint`

- [ ] **Step 1: Add the struct to `src/ui/mod.rs`, after the `use` lines**

```rust
/// What every view draws with: the language of its words, the colours and
/// the platform's look. A view takes it, and the state it shows, instead
/// of the whole `App`.
#[derive(Clone, Copy)]
pub struct Paint {
    pub locale: crate::i18n::Locale,
    pub palette: crate::theme::Palette,
    pub look: crate::theme::Look,
}

impl Paint {
    pub fn of(app: &App) -> Self {
        Self {
            locale: app.locale,
            palette: app.palette,
            look: app.look,
        }
    }
}
```

- [ ] **Step 2: Build.** `cargo build` gives a dead-code warning until Task 2 uses it; go on.

### Task 2: the password prompt

- [ ] **Step 1: Run its tests before the change**

Run: `cargo test --locked -p tabletist password`
Expected: PASS.

- [ ] **Step 2: Change `src/ui/password_prompt.rs`'s head and tail**

```rust
use crate::i18n::gettext;
use crate::model::{Action, PasswordPrompt, SecretKind};
use crate::ui::{Paint, widgets};

/// Draws the prompt over the window, and returns what it asks for.
pub fn show(ctx: &egui::Context, paint: Paint, prompt: &mut PasswordPrompt) -> Vec<Action> {
    let Paint {
        locale,
        palette,
        look,
    } = paint;
    let mut actions = Vec::new();
    // ... the modal, unchanged ...
    actions
}
```

The `let Some(Dialog::Password(prompt)) = &mut app.dialog else { return; };` and the closing `app.actions.extend(actions);` go; the modal between them stays as it is.

- [ ] **Step 3: Call it from `ui::show` in `src/ui/mod.rs`**

```rust
    connect_dialog::show(app, &ui.ctx().clone());
    let paint = Paint::of(app);
    if let Some(Dialog::Password(prompt)) = &mut app.dialog {
        let asked = password_prompt::show(&ui.ctx().clone(), paint, prompt);
        app.actions.extend(asked);
    }
    host_key_prompt::show(app, &ui.ctx().clone());
```

with `Dialog` added to `use crate::model::{Action, ConnTabContent};`.

- [ ] **Step 4: Run the tests again**

Run: `cargo test --locked -p tabletist password`
Expected: PASS, the same count as Step 1.

- [ ] **Step 5: Commit**

```bash
git add src/ui/mod.rs src/ui/password_prompt.rs
git commit -m "Hand the password prompt its prompt and Paint instead of the App"
```

### Task 3: the filter bar

- [ ] **Step 1: Run its tests before the change**

Run: `cargo test --locked -p tabletist filter`
Expected: PASS.

- [ ] **Step 2: Change `src/ui/filter_bar.rs`**

```rust
use crate::model::{Action, ConnTabId, ObjectTab};
use crate::ui::Paint;

/// Whether the object tab's filter bar is open.
pub fn is_open(object: &ObjectTab) -> bool {
    object.filter.open
}

/// Draws the open filter bar of `object`, in the workspace `tab`, and
/// returns what it asks for.
pub fn show(ui: &mut egui::Ui, paint: Paint, tab: ConnTabId, object: &mut ObjectTab) -> Vec<Action> {
    let Paint {
        locale,
        palette,
        look,
    } = paint;
    let object_tab = object.id;
    if !object.filter.open {
        return Vec::new();
    }
    // ... unchanged from "Columns from the page" on ...
    if apply {
        actions.push(Action::ApplyFilters { tab, object_tab });
    }
    actions
}
```

`use crate::app::App;` and the `TabId` import go.

- [ ] **Step 3: Call it from `src/ui/workspace.rs`**

```rust
                                let open = app
                                    .workspace(tab)
                                    .and_then(|workspace| workspace.object_tab(object_tab))
                                    .is_some_and(super::filter_bar::is_open);
                                if open {
                                    // ... the panel, unchanged ...
                                    .show(ui, |ui| {
                                        let paint = super::Paint::of(app);
                                        if let Some(object) = app
                                            .workspace_mut(tab)
                                            .and_then(|workspace| workspace.object_tab_mut(object_tab))
                                        {
                                            let asked =
                                                super::filter_bar::show(ui, paint, tab, object);
                                            app.actions.extend(asked);
                                        }
                                        // ... the line under it, unchanged ...
```

- [ ] **Step 4: Move the three tests in `src/ui/mod.rs` that call `is_open`**

```rust
            assert!(
                harness
                    .app
                    .workspace(tab)
                    .and_then(|workspace| workspace.object_tab(id))
                    .is_some_and(crate::ui::filter_bar::is_open)
            );
```

- [ ] **Step 5: Run the tests again**

Run: `cargo test --locked -p tabletist filter`
Expected: PASS, the same count as Step 1.

- [ ] **Step 6: Commit**

```bash
git add src/ui/filter_bar.rs src/ui/workspace.rs src/ui/mod.rs
git commit -m "Hand the filter bar its object tab and Paint instead of the App"
```

### Task 4: CI's checks and the PR

- [ ] **Step 1:** `cargo fmt --all --check`
- [ ] **Step 2:** `cargo clippy --locked --workspace --all-targets -- -D warnings`
- [ ] **Step 3:** `cargo test --locked --workspace --all-targets`
- [ ] **Step 4:** Push the branch and open the PR: what moved, the pattern in a paragraph, and a link to this plan.
