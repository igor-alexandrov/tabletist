# Dialogs give the keyboard back. Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Closing a dialog gives the keyboard back to the widget that had it before the dialog opened, if that widget is still drawn, for every `app.dialog` variant; a dialog whose answer moved the keys (quick open opening a table) leaves them where the answer put them.

**Architecture:** `focus::dialog` in `src/ui/focus.rs`, called by `App::frame_ui` before anything is drawn, notes the focused widget in the first frame that starts with a dialog up and requests it again in the first frame that starts with the dialog gone and egui's modal layer cleared. `App::keys_place` (the window's tab, its workspace's tab in front, whether the tree has the keys) is noted with it: a give-back happens only where it is unchanged, only to a widget the frame before drew, and only while nothing has the keyboard.

**Tech Stack:** Rust 2024, egui 0.36 (the crmne fork at `ba6790f`); tests through `src/testing.rs` and plain `egui::Context` frames.

---

## Where this sits

Found while fixing review comments on igor-alexandrov/tabletist#180 (the start screen's search, run 2). Typing `?` over the sidebar tree and closing the list of keys leaves egui's focus empty, so the next F6, Tab or click is needed before the tree has the keyboard again. It happens with a table open as on the start screen, in every look, and for every dialog. This plan is a branch of `main` at `69bbf50`.

## Why the keyboard is lost

Every `app.dialog` is drawn as an `egui::Modal` (`widgets::modal`). egui's `Context::create_widget` finds a widget under the top modal layer not interested in focus and calls `surrender_focus` on it, so the frame after a dialog opens the tree, the grid or a field loses the keyboard. Nothing gives it back when the dialog closes.

egui knows the top modal layer from the frame before (`Memory::top_modal_layer`). In the first frame drawn without the dialog, what was behind it is still under that layer and gives up any focus asked for it. The give-back therefore waits for a frame that starts with `top_modal_layer()` empty: then the widget takes the keyboard when it is next drawn. (A probe of this plan confirmed it: asking in the frame the dialog closes, or in the next one, is undone.)

## What this run settles

| Question | Decision | Why |
|---|---|---|
| Where what had the keyboard is kept | `focus::dialog` in `src/ui/focus.rs`, in egui's temp data, as the other per-frame focus state there is | Views only push actions; the frame has the context |
| When it is noted | Before anything is drawn, in the first frame that starts with a dialog up | The dialog is not drawn yet: egui has not taken the keyboard, and no field of the dialog (quick open's) has asked for it. Dialogs opened by the frame's keys (applied after it draws) and by the backend or a test (applied before it draws) both pass here first |
| When it is given back | Before anything is drawn, in the first frame that starts with no dialog and `top_modal_layer()` empty | egui's lag of a frame, above |
| To what | The noted widget, only if the frame before drew it (`ctx.read_response`, which before anything is drawn reads the frame before's widgets) and nothing has the keyboard | A tab the dialog's answer closed is not drawn; a widget the answer gave the keyboard (the filter bar's field) keeps it |
| A dialog whose answer moved the keys | No give-back: `App::keys_place()` is noted with the widget and compared | Opening a table makes it the tab in front and sets `pane` to the grid. Giving the tree its focus back would set `pane` to the tree again (its `gained_focus` pushes `SetTreeCursor`) and take the keys from the table the person asked for |
| One dialog giving way to another (the list of keys to About or Settings) | One dialog: `app.dialog` stays `Some` | What had the keyboard before the list of keys has it once Settings closes |
| Which dialogs | Every `app.dialog` variant: one check in the frame, none in a dialog | The cause is the modal, which they all are |
| The frame the give-back waits for | `dialog` asks for it (`request_repaint`), as `step` does for F6 | egui already repaints in the frame after a modal closes (the probe saw a zero delay there), so no headless test tells the request apart; asking keeps the give-back off an egui detail. The by-hand check covers it |
| The ring | Unchanged | Escape is a key, so the ring shows; a click on Close hides it, and the tree has the keyboard all the same |
| The website | No change | The pages list keys; none says where the keyboard goes after a dialog |

## Files

- Modify: `src/ui/focus.rs` (the `Behind` state, `dialog`, four unit tests)
- Modify: `src/app.rs` (`App::keys_place`, one call in `App::frame_ui`)
- Create: `src/ui/dialog_focus_tests.rs` (five harness tests)
- Modify: `src/ui/mod.rs` (the test module)

## Task 1: the failing tests

**Files:** Create `src/ui/dialog_focus_tests.rs`; modify `src/ui/mod.rs`.

- [x] **Step 1: Declare the module.** In `src/ui/mod.rs`, after `mod delete_tests;` (rustfmt keeps the modules in order):

```rust
#[cfg(test)]
mod dialog_focus_tests;
```

- [x] **Step 2: Write the tests.** `src/ui/dialog_focus_tests.rs`:

```rust
//! Closing a dialog gives the keyboard back to what had it before the
//! dialog opened: egui takes it from everything drawn behind a dialog.

use egui::{Key, Modifiers};
use tabletist_db::{ObjectKind, ObjectRef};

use crate::model::{Action, ConnTabId, Dialog, Pane};
use crate::testing::{Harness, bookshop_objects, fixture_connection};
use crate::theme::Look;

/// A Bookshop connection in `look`, its start screen showing.
fn bookshop(look: Look) -> (Harness, ConnTabId) {
    let mut harness = Harness::new();
    harness.set_look(look);
    let tab = harness.connect_with(fixture_connection(), bookshop_objects());
    (harness, tab)
}

/// Opens the Bookshop's `authors` table, its first page read.
fn open_authors(harness: &mut Harness, tab: ConnTabId) {
    harness.app.apply(Action::OpenObject {
        tab,
        object: ObjectRef::new("main", "authors"),
        kind: ObjectKind::Table,
        pin: true,
    });
    harness.answer_rows(crate::testing::page(3, false));
}

/// The name a screen reader would read for what has the keyboard.
fn focused(harness: &mut Harness) -> String {
    super::tests::focused_name(&harness.settle())
}

/// Steps with F6 until the keyboard is on `name`.
fn reach(harness: &mut Harness, name: &str) {
    let mut seen = vec![focused(harness)];
    while seen.last().is_none_or(|last| last != name) {
        assert!(seen.len() <= 12, "{name} not reached: {seen:?}");
        harness.press(Key::F6, Modifiers::NONE);
        seen.push(focused(harness));
    }
}

/// `?` over the tree, then Escape: the tree has the keyboard again, on
/// the start screen and with a table open, in every look.
#[test]
fn closing_the_shortcuts_gives_the_keyboard_back_to_the_tree() {
    for look in Look::ALL {
        for table in [false, true] {
            let (mut harness, tab) = bookshop(look);
            if table {
                open_authors(&mut harness, tab);
            }
            reach(&mut harness, "Objects");
            super::tests::type_key(&mut harness, Key::Questionmark, "?");
            assert!(
                matches!(harness.app.dialog, Some(Dialog::Help)),
                "{}",
                look.name
            );
            harness.press(Key::Escape, Modifiers::NONE);
            assert!(harness.app.dialog.is_none(), "{}", look.name);
            let name = focused(&mut harness);
            assert_eq!(name, "Objects", "{}, a table open: {table}", look.name);
        }
    }
}

/// The rows have the keyboard again after the shortcuts too.
#[test]
fn closing_the_shortcuts_gives_the_keyboard_back_to_the_rows() {
    let (mut harness, tab) = bookshop(Look::macos());
    open_authors(&mut harness, tab);
    reach(&mut harness, "Rows");
    super::tests::type_key(&mut harness, Key::Questionmark, "?");
    harness.press(Key::Escape, Modifiers::NONE);
    assert!(harness.app.dialog.is_none());
    assert_eq!(focused(&mut harness), "Rows");
}

/// Quick open closed with Escape: the tree has the keyboard again.
#[test]
fn closing_quick_open_gives_the_keyboard_back() {
    let (mut harness, _tab) = bookshop(Look::macos());
    reach(&mut harness, "Objects");
    harness.press(Key::P, Modifiers::COMMAND);
    assert!(matches!(harness.app.dialog, Some(Dialog::QuickOpen(_))));
    harness.press(Key::Escape, Modifiers::NONE);
    assert!(harness.app.dialog.is_none());
    assert_eq!(focused(&mut harness), "Objects");
}

/// Settings opened from the shortcuts take their place: what had the
/// keyboard before the shortcuts has it once Settings closes.
#[test]
fn a_dialog_that_gives_way_to_another_gives_the_keyboard_back_once_closed() {
    let (mut harness, _tab) = bookshop(Look::macos());
    reach(&mut harness, "Objects");
    super::tests::type_key(&mut harness, Key::Questionmark, "?");
    harness.click("Settings");
    assert!(matches!(harness.app.dialog, Some(Dialog::Settings(_))));
    harness.press(Key::Escape, Modifiers::NONE);
    assert!(harness.app.dialog.is_none());
    assert_eq!(focused(&mut harness), "Objects");
}

/// Quick open's answer opens a table and gives it the keys: the tree,
/// which had the keyboard, does not take it back.
#[test]
fn quick_open_leaves_the_keyboard_with_the_table_it_opens() {
    let (mut harness, tab) = bookshop(Look::macos());
    reach(&mut harness, "Objects");
    harness.press(Key::P, Modifiers::COMMAND);
    harness.frame(vec![egui::Event::Text("authors".into())]);
    harness.press(Key::Enter, Modifiers::NONE);
    assert!(harness.app.dialog.is_none());
    assert_ne!(focused(&mut harness), "Objects");
    let workspace = harness.app.workspace(tab).unwrap();
    assert_eq!(workspace.pane, Pane::Grid);
    let object = workspace.object_tab(workspace.active_tab.unwrap()).unwrap();
    assert_eq!(object.object.name, "authors");
}
```

- [x] **Step 3: Run them and see the four fail.**

Run: `CARGO_TARGET_DIR=$PWD/target ~/.cargo/bin/cargo test --locked -p tabletist --lib dialog_focus_tests`
Expected: the four `closing_…` and `a_dialog_that_gives_way…` tests FAIL with `left: ""`, `right: "Objects"` (or `"Rows"`); `quick_open_leaves_the_keyboard_with_the_table_it_opens` passes: it holds today's behaviour, which a give-back without the place check breaks (`left: Tree`, `right: Grid`).

## Task 2: give the keyboard back

**Files:** Modify `src/ui/focus.rs`, `src/app.rs`.

- [x] **Step 1: The state and `dialog`.** In `src/ui/focus.rs`, after `begin_frame` and before `visible`:

```rust
fn behind_id() -> Id {
    Id::new("focus-behind")
}

/// What had the keyboard when a dialog opened.
#[derive(Clone, Copy, Default)]
struct Behind {
    /// A dialog was up when [`dialog`] was last called.
    open: bool,
    /// The widget, and where the app's keys went then.
    had: Option<(Id, Id)>,
}

/// Gives the keyboard back to what had it before a dialog opened, once
/// the dialog has gone: egui takes it from everything drawn behind a
/// dialog and gives nothing back. Call before anything is drawn, with
/// whether a dialog is up and `place`, where the app's keys go (its tab,
/// its pane). A dialog whose answer moved them (quick open opening a
/// table) leaves the keyboard where the answer put it. It goes back only
/// to a widget the frame before drew, and only while nothing has it.
pub fn dialog(ctx: &egui::Context, open: bool, place: Id) {
    let focused = ctx.memory(|memory| memory.focused());
    // egui keeps a closed dialog's layer for a frame, and in that one
    // nothing behind it takes the keyboard.
    let modal = ctx.memory(|memory| memory.top_modal_layer()).is_some();
    let (back, waiting) = ctx.data_mut(|data| {
        let behind: &mut Behind = data.get_temp_mut_or_default(behind_id());
        let was_open = std::mem::replace(&mut behind.open, open);
        if open {
            // The dialog is not drawn yet: the keyboard is where it was.
            if !was_open {
                behind.had = focused.map(|id| (id, place));
            }
            (None, false)
        } else if modal {
            (None, behind.had.is_some())
        } else {
            (behind.had.take(), false)
        }
    });
    // The frame that gives it back comes without waiting for a key.
    if waiting {
        ctx.request_repaint();
    }
    // Before anything is drawn a response is the frame before's.
    if let Some((id, then)) = back
        && then == place
        && focused.is_none()
        && ctx.read_response(id).is_some()
    {
        ctx.memory_mut(|memory| memory.request_focus(id));
    }
}
```

- [x] **Step 2: Its unit tests.** In the `tests` module of `src/ui/focus.rs`, before `focus_the_pointer_gave_is_not_shown`. Each condition of the give-back has one that fails without it (checked in the probe):

```rust
    /// One frame: the button `Behind` where `drawn`, `Other` asking for
    /// the keyboard where `takes`, and over them a dialog while `open`.
    /// The app's keys go to `place`. Returns Behind's and Other's ids.
    fn scene(
        ctx: &egui::Context,
        open: bool,
        (drawn, takes): (bool, bool),
        place: &str,
    ) -> (Id, Id) {
        let (mut behind, mut other) = (Id::NULL, Id::NULL);
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            dialog(ui.ctx(), open, Id::new(place));
            egui::CentralPanel::default().show(ui, |ui| {
                if drawn {
                    behind = ui.push_id("behind", |ui| ui.button("Behind")).inner.id;
                }
                other = ui.push_id("other", |ui| ui.button("Other")).inner.id;
                if takes {
                    ui.memory_mut(|memory| memory.request_focus(other));
                }
            });
            if open {
                egui::Modal::new(Id::new("dialog")).show(ui.ctx(), |ui| {
                    let _ = ui.button("Inside");
                });
            }
        });
        output.textures_delta.clear();
        (behind, other)
    }

    /// `Behind` has the keyboard, a dialog opens over it, then closes.
    /// `after` is what each frame without the dialog draws (`Other` asks
    /// for the keyboard in the first of them only), and `place` where its
    /// keys go. Returns Behind's and Other's ids, and what had the
    /// keyboard after each frame without the dialog.
    fn across_a_dialog(
        ctx: &egui::Context,
        (drawn, takes): (bool, bool),
        place: &str,
    ) -> (Id, Id, Vec<Option<Id>>) {
        let (button, _) = scene(ctx, false, (true, false), "tree");
        ctx.memory_mut(|memory| memory.request_focus(button));
        scene(ctx, false, (true, false), "tree");
        for _ in 0..3 {
            scene(ctx, true, (true, false), "tree");
        }
        // egui's doing: nothing behind a dialog keeps the keyboard.
        assert_eq!(ctx.memory(|memory| memory.focused()), None);
        let mut other = Id::NULL;
        let mut had = Vec::new();
        for frame in 0..4 {
            other = scene(ctx, false, (drawn, takes && frame == 0), place).1;
            had.push(ctx.memory(|memory| memory.focused()));
        }
        (button, other, had)
    }

    #[test]
    fn a_closed_dialog_gives_the_keyboard_back() {
        let ctx = egui::Context::default();
        let (button, _, had) = across_a_dialog(&ctx, (true, false), "tree");
        assert_eq!(had.last(), Some(&Some(button)), "{had:?}");
    }

    #[test]
    fn the_keyboard_goes_back_only_to_a_widget_still_drawn() {
        let ctx = egui::Context::default();
        let (_, _, had) = across_a_dialog(&ctx, (false, false), "tree");
        assert!(had.iter().all(Option::is_none), "{had:?}");
    }

    #[test]
    fn a_widget_that_took_the_keyboard_keeps_it() {
        let ctx = egui::Context::default();
        let (_, other, had) = across_a_dialog(&ctx, (true, true), "tree");
        assert!(had.iter().all(|had| *had == Some(other)), "{had:?}");
    }

    #[test]
    fn a_dialog_that_moved_the_keys_leaves_the_keyboard_alone() {
        let ctx = egui::Context::default();
        let (_, _, had) = across_a_dialog(&ctx, (true, false), "grid");
        assert!(had.iter().all(Option::is_none), "{had:?}");
    }
```

- [x] **Step 3: Where the keys go.** In `src/app.rs`, before `/// Draws one frame, then applies what the frame asked for.`:

```rust
    /// Where the keys go: the window's tab, and in its workspace the tab in
    /// front and whether the tree has them. A dialog whose answer changes
    /// it moved the keyboard on purpose.
    fn keys_place(&self) -> egui::Id {
        let tab = self.active_tab_id();
        let workspace = self
            .workspace(tab)
            .map(|workspace| (workspace.active_tab, workspace.pane == Pane::Tree));
        egui::Id::new((tab, workspace))
    }
```

- [x] **Step 4: Call it in the frame.** In `App::frame_ui`, right after `crate::ui::focus::begin_frame(ui.ctx());`:

```rust
        // Before anything is drawn: what had the keyboard as a dialog
        // opened has it back once the dialog has gone.
        crate::ui::focus::dialog(ui.ctx(), self.dialog.is_some(), self.keys_place());
```

- [x] **Step 5: Run the new tests.**

Run: `CARGO_TARGET_DIR=$PWD/target ~/.cargo/bin/cargo test --locked -p tabletist --lib -- dialog_focus_tests ui::focus::tests`
Expected: 13 passed.

## Task 3: checks, commit and pull request

- [x] **Step 1: The checks.** With `CARGO_TARGET_DIR=$PWD/target` and `~/.cargo/bin/cargo`:

```bash
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace --all-targets
RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps
```

Expected: all pass. (The probe ran the whole workspace's tests with this change: 2893 passed in the app crate, every other suite green.) The server integration tests print "skipped" without their URLs; this change does not touch them.

- [x] **Step 2: Commit.** One topic, one commit: the tests and the change together (each commit passes the checks).

```bash
git add src/ui/focus.rs src/app.rs src/ui/mod.rs src/ui/dialog_focus_tests.rs docs/superpowers/plans/2026-10-19-dialogs-give-the-keyboard-back.md && git commit -m "Give the keyboard back to what had it when a dialog closes"
```

If signing says "agent refused operation", 1Password is locked: leave it staged and say so.

- [ ] **Step 3: By hand, if Igor wants it.** No session here has a display, so this goes to Igor: on the Bookshop, F6 to the tree, `?`, Escape, then ↓ at once. The tree's cursor moves and the tree shows it has the keyboard; the same after Cmd+P (Omarchy: its quick open key) and Escape.

- [ ] **Step 4: The pull request.** Push the branch and open one pull request: what was lost and why (egui's modal), the decisions above, and platform coverage (built and tested on Linux; the change is egui code with no `cfg`).

## Risks

- igor-alexandrov/tabletist#180 (open) adds a search field to the start screen. If it lands first, run `dialog_focus_tests` again after rebasing: a field that asks for the keyboard whenever nothing has it would take it in the frame the give-back waits out, and the tree would not get it back on the start screen.
- A dialog that opens while the give-back waits out egui's frame (right after another closed) is noted with nothing focused, so closing it gives nothing back. No dialog does this today.
