# Narrowing what the views see of `App`, run 2: the other small dialogs

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `host_key_prompt`, `every_row_prompt`, `about`, `help` and `toast` take `ui::Paint` and their own state instead of `&App`/`&mut App`, as run 1 did for the password prompt (`2026-10-31-narrow-app-access.md`, "The pattern"). Behaviour does not change.

**Files:**
- Modify: `src/ui/mod.rs` (`Paint::layout`; `ui::show` finds each dialog's state and calls it)
- Modify: `src/ui/host_key_prompt.rs`, `src/ui/every_row_prompt.rs`, `src/ui/about.rs`, `src/ui/help.rs`, `src/ui/toast.rs`

## Survey (2026-10-31, `main` at `91dbe5bf`)

| View | What it read of `App` | Now takes |
|---|---|---|
| `host_key_prompt` | Paint, `dialog` (`HostKeyPrompt`) | `ctx, paint, &HostKeyPrompt` |
| `every_row_prompt` | Paint, `dialog` (`EveryRowPrompt`), `workspace(prompt.tab)` to see whether the SQL tab it was asked from is still open | `ctx, paint, &EveryRowPrompt, editor_open: bool` |
| `about` | Paint, `dialog` (whether it is About) | `ctx, paint`; `fold(ctx)` for the frames it is not open |
| `help` | Paint, `dialog` (whether it is Help), `keymap`, `layout()` | `ctx, paint, &Keymap` |
| `toast::show` | look and palette | `ctx, paint`, returns nothing |

`toast::say`, `say_for` and `said`, called from `app`, `keys`, `agents` and the tests, took only the `egui::Context` already: only `show` took `App`, so the toast is no more tangled than the others.

`about` did work on the frames it was not shown: it dropped whether its fonts' notices were unfolded, so the next About starts folded. That stays the view's own, as `about::fold`, which `ui::show` calls when the dialog is not About.

The net is the existing tests: `about::tests`, the shortcuts dialog's in `src/ui/mod.rs` (`question_mark_opens_the_shortcuts_and_escape_closes_them`, `the_shortcuts_dialog_lists_each_looks_own_editing_keys`, `the_shortcuts_dialog_opens_the_settings_in_every_look`, `the_shortcuts_keep_close_on_screen_in_the_smallest_window`), the host key's in `src/app.rs` (`an_unknown_host_key_asks_and_trusting_saves_it_and_reconnects`, `cancelling_the_host_key_prompt_trusts_nothing`), `src/ui/every_row_tests.rs`, and every test that reads `toast::said`. None calls a function whose signature moved.

### Task 1: `Paint::layout`

- [x] Add to `impl Paint` in `src/ui/mod.rs`:

```rust
    /// The keyboard layout the look writes keys in, as `App::layout` has it.
    pub fn layout(&self) -> crate::keymap::Layout {
        crate::keymap::Layout::of(&self.look)
    }
```

### Task 2: the views

- [x] Each view's first line takes `paint` apart; its `let Some(Dialog::…) = &app.dialog else { return; }` and its `app.actions.extend(actions)` go, and it returns `actions`.
- [x] `every_row_prompt::show` answers Cancel when `!editor_open`, where it used to when `app.workspace(prompt.tab).and_then(|w| w.sql_tab(prompt.id))` was `None`.

### Task 3: the callers in `ui::show`

- [x] With `paint` taken once, before the password prompt, as run 1 has it (nothing in `ui::show` changes the locale, palette or look: `App::apply_settings`, `attach` and `logic` do, outside it):

```rust
    if let Some(Dialog::HostKey(prompt)) = &app.dialog {
        let asked = host_key_prompt::show(&ui.ctx().clone(), paint, prompt);
        app.actions.extend(asked);
    }
    quick_open::show(app, &ui.ctx().clone());
    if matches!(app.dialog, Some(Dialog::Help)) {
        let asked = help::show(&ui.ctx().clone(), paint, &app.keymap);
        app.actions.extend(asked);
    }
    if matches!(app.dialog, Some(Dialog::About)) {
        let asked = about::show(&ui.ctx().clone(), paint);
        app.actions.extend(asked);
    } else {
        about::fold(ui.ctx());
    }
    settings::show(app, &ui.ctx().clone());
    write_prompts::show(app, &ui.ctx().clone());
    if let Some(Dialog::ConfirmEveryRow(prompt)) = &app.dialog {
        let editor_open = app
            .workspace(prompt.tab)
            .and_then(|workspace| workspace.sql_tab(prompt.id))
            .is_some();
        let asked = every_row_prompt::show(&ui.ctx().clone(), paint, prompt, editor_open);
        app.actions.extend(asked);
    }
    conflict_prompt::show(app, &ui.ctx().clone());
    agent_access::show(app, &ui.ctx().clone());
    // Last: it says what was done, whatever is on screen by now.
    toast::show(ui.ctx(), paint);
```

The views are called in the order they were, so the order of actions in a frame is unchanged.

### Task 4: CI's checks and the PR

- [x] `cargo fmt --all --check`
- [x] `cargo clippy --locked --workspace --all-targets -- -D warnings`
- [x] `cargo test --locked --workspace --all-targets`
- [x] Push and open the PR.

## For the runs after

Nothing in runs 3 to 8 changes. The pattern gains one line (in the main plan): a view that did something in the frames it was not shown gives that its own function, which its caller calls instead of the view, as `about::fold`.
