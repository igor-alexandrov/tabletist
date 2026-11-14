# Narrowing what the views see of `App`, run 3: the object-tab panes

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `row_form`, `pending_bar`, `structure`, `ddl` and `review` take `ui::Paint`, the object tab they show and what they read beside it, instead of `&App`/`&mut App`, as runs 1 and 2 did (`2026-10-31-narrow-app-access.md`, "The pattern"). Behaviour does not change.

**Files:**
- Modify: `src/ui/row_form.rs`, `src/ui/pending_bar.rs`, `src/ui/structure.rs`, `src/ui/ddl.rs`, `src/ui/review.rs`
- Modify: `src/ui/workspace.rs` (one private function per pane finds its tab and calls it)
- Modify: `src/ui/keys.rs`, `src/ui/data_view.rs` (call `ddl::copy` with the tab; Copy SQL builds the text from `App::review_whole`), `src/ui/row_panel.rs` (`take_editor`/`put_editor` with the tab)
- Modify: `src/ui/duplicate_tests.rs` (one test called `review::copy_text`, which is gone)

## Survey (2026-10-31, `main` at `ee6a184f`)

| View | What it read of `App` | Now takes |
|---|---|---|
| `row_form::take_editor`, `put_editor` | one object tab (`workspace_mut`) | `&mut ObjectTab` |
| `pending_bar::show` | Paint, one object tab, its workspace for `key_picker::unmatched`, `save_blocked`, `lacking` | `ui, paint, tab, &ObjectTab, Asked { doubts, blocked, lacking }`; `shows(&ObjectTab)` and `pass(ui)` |
| `structure::show` | Paint, one object tab, its workspace's `label_refused`, `pane`, `driver`, `name`, `environment`, `keymap`, `structure_label` | `ui, paint, tab, &Workspace, &ObjectTab, Option<Resolved>, &Keymap` |
| `ddl::show`, `footer` | Paint, one object tab, its workspace's `driver` | `ui, paint, tab, &ObjectTab, Driver` |
| `ddl::copy` | look, locale, one object tab | `ctx, paint, tab, &ObjectTab` |
| `review::show` | Paint, one object tab, its workspace's `access`, `review_whole` when Copy is pressed | `ui, paint, tab, &ObjectTab, read_only: bool, whole: impl FnOnce() -> Option<Review>`; `pass(ui)` |
| `review::copy_text` | `review_whole`, locale | gone: its two callers (`keys`, one test) put `App::review_whole` through `review::text` |

The main plan's survey listed `pending_bar` as calling `workspace` and `lacking`; it also called `save_blocked` and `key_picker::unmatched`. Neither changes the run: both are answers its caller gives.

No pane was more tangled than the plan assumed; nothing is deferred.

## The borrow an object-tab pane takes

What runs 5 to 7 reuse:

- **The slice is the `ObjectTab`** the pane shows, found by its caller: `&mut ObjectTab` where a control edits it in place (`filter_bar`, `row_form`'s editor), `&ObjectTab` otherwise. Never `ConnTab`, never the `(ConnTabId, TabId)` pair to look it up with: the pane names `object.id` where it used to name the id it was given.
- **What the pane reads of the workspace around the tab** comes as its own arguments while it is one or two values (`ddl`: the `Driver`; `review`: whether the connection is read-only). Where it reads more of it (`structure`: five fields), it takes `&Workspace` beside the `&ObjectTab`: both are shared borrows of `App`, so its caller finds both.
- **A question that joins the tab with the rest of `App`** (`structure_label`, `save_blocked`, `lacking`, `key_picker::unmatched`) is answered by the caller and passed in; several answers go in a struct of the pane's own (`pending_bar::Asked`). They stay `App`'s methods: they read `App::table`, which other reducers share, and moving them is not this work.
- **An answer too costly to give every frame**, needed only when a button is pressed, comes as a closure the pane calls then (`review::show`'s `whole`, which builds the review with every value whole). The clipboard's text and the toast are made in the same frame as before.
- **A panel that passes over an id while it is not drawn** (`pending_bar`, `review`: `ui.skip_ahead_auto_ids(1)`, so a field drawn after it keeps the keyboard) gives that its own `pass(ui)`, as `about::fold`. Its caller calls `pass` where the tab is gone or has nothing to show; `pending_bar::shows(&ObjectTab)` says which, so the caller asks the answers only for a bar that is drawn, as the bar did.

`ui::workspace` gives each pane a private function (`pending_bar`, `review`, `structure`, `ddl`, `ddl_footer`) that takes `Paint::of(app)` first, finds the tab, asks the answers, calls the pane and extends `app.actions`, in the place the pane was called before. The order of actions in a frame is unchanged: each pane asked for its actions at its end, and the caller extends with them at the same point.

## Tasks

### Task 1: `row_form`

- [x] `take_editor(object: &mut ObjectTab)` and `put_editor(object: &mut ObjectTab, editor: Option<Editor>)`. `row_panel::show`, not yet narrow (run 7), finds the tab with `workspace_mut` for each.

### Task 2: `ddl`

- [x] `show` and `footer` take `ui, paint, tab, &ObjectTab, Driver`; `show` returns its actions (the terminal look's early `app.actions.extend(actions); return;` is `return actions;`).
- [x] `copy` takes `ctx, paint, tab, &ObjectTab`. Its three callers (`keys` twice, `data_view` once) find the tab; one that is gone copies nothing, as before.

### Task 3: `structure`

- [x] `show` takes `ui, paint, tab, &Workspace, &ObjectTab, Option<Resolved>, &Keymap` and returns its actions. Its caller asks `app.structure_label(tab, object_tab)`.

### Task 4: `pending_bar`

- [x] `show` takes `ui, paint, tab, &ObjectTab, Asked` and returns its actions; `shows` and `pass` split off as above. Its caller computes `doubts` (`key_picker::unmatched(workspace, object).len()`), `blocked` (`app.save_blocked`) and `lacking` (`app.lacking`) only when `shows` is true.

### Task 5: `review`

- [x] `show` takes `ui, paint, tab, &ObjectTab, read_only, whole` and returns its one action (Hide); `pass` split off. Copy calls `whole()` and puts `text(&review, locale)` on the clipboard, where it called `copy_text(app, tab, id)`.
- [x] `copy_text` goes: `keys`' Copy SQL and `duplicate_tests::y_then_capital_y_with_the_review_open_yanks_nothing_and_the_capital_copies_the_sql` build the text from `app.review_whole(tab, id)` with `review::text`.

### Task 6: CI's checks and the PR

- [x] `cargo fmt --all --check`
- [x] `cargo clippy --locked --workspace --all-targets -- -D warnings`
- [x] `cargo test --locked --workspace --all-targets`
- [x] Push and open the PR.

The net is the existing tests, unchanged but for the one above: `src/ui/ddl_tests.rs`, `ddl_act_tests.rs`, `label_tests.rs`, `comment_tests.rs`, `structure::tests`, `review::tests`, `delete_tests.rs`, `delete_many_tests.rs`, `duplicate_tests.rs`, `insert_audit_tests.rs`, `key_picker_tests.rs`, and the pending bar's and Review SQL's tests in `src/ui/mod.rs` and `src/app.rs`.

## For the runs after

- Runs 5 to 7 take the borrow above for what sits over an object tab, and `&Workspace` for what sits over a workspace (`object_tabs`, `history`, the dialogs of run 5).
- Run 7 (`row_panel`) moves the `take_editor`/`put_editor` lookups into its caller with the rest of the panel; `data_view`'s `ddl::copy` lookup goes with `data_view`.
- The routers' plan inherits `ui::workspace`'s five pane functions and `keys`' two `ddl::copy` lookups and Copy SQL.
