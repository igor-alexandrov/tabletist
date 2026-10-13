# Deleting Many Rows, Slice 2: Omarchy's Visual Rows and `{count}dd` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** In the Omarchy look, `V` starts a visual selection of rows, the moves extend it, `d` marks its rows and `esc` leaves it, with `-- VISUAL --` on the status line. `{count}dd` marks the cursor's row and the rows after it.

**Architecture:** A visual selection is slice 1's range with `held` set: moves keep it (`Reach::Move`), and `ObjectTab::visual()` says it is on. The keymap gains `When::Visual` and three commands; while visual is on, the view holds back the commands that act on one cell or one row. A count is read by the keymap, which hands back `Typed::Counted(command, count)` for a command that takes one. Marking is slice 1's: `Table::sweep` decides, `App::sweep_rows` acts.

**Tech Stack:** Rust 1.98; egui/eframe (the crmne forks). Spec: `docs/superpowers/specs/2026-10-13-deleting-many-rows-design.md` ("spec N" is one of its sections, "test N" an entry of its Tests, "M N" a row of its decisions, all approved at their defaults). This is slice 2 of its three: sections 1 (Omarchy) and 3, and tests 3, 4, 5 and 6 (by count). Slice 1: `docs/superpowers/plans/2026-10-13-deleting-many-rows-1-desktop.md`, built, in pull request `igor-alexandrov/tabletist#120`.

Line numbers are of head `1a9cdf3`. Two commits since (`4acd417`, `3d887c2`) moved lines of `src/edit.rs` and `src/app/editing.rs` by a few dozen: find a place there by the text the plan quotes.

---

## Where the plan leaves the spec, for a yes

| # | The spec says | The plan does | Why |
|---|---|---|---|
| Q1 | "Delete row, on a visual selection" is `d`, a line of the Delete row command with `When::Visual` | A command of its own, `DeleteSelection`, bound to `d` when visual | The status line names the key (`d delete`), and the keymap gives a command's first key, which for Delete row is `dd` |
| Q2 | The mode line reads `-- VISUAL --` (the board draws the mode alone, in its magenta) | `-- VISUAL --` in `palette.magenta`, then `3 rows`, then the counts, and `d delete · esc normal` at the right, which gives way first | As approved in M3. The colour is the board's |
| Q3 | "`d` … leaves visual" | Where the press marks nothing (over 1,000 rows, a read-only connection), visual stays on, and the status line says why in place of its two keys | The message says "Select fewer rows": the selection is still there to shrink |
| Q4 | The commands held back in visual are a list of nineteen | The list, and with it Boolean cycle (space) and Enter on the rows, which puts the keyboard on the inspector's fields | Both act on the cursor's cell or row. Test 3 has Enter do nothing |
| Q5 | "A count … ends with [the wait]: on a key that types nothing, a click, a field that takes the keyboard" | The count is part of the characters waited for (`typed-keys`), so it ends exactly where the first `d` of `dd` does | One mechanism |
| Q6 | A visual selection is the terminal look's | `visual` is read as "the look is the terminal's and the range is held". A held range in a desktop look, which only a change of look mid-selection can leave, is a plain range there until a click ends it | No desktop key leaves visual |

## Before you start

- Cargo is `~/.cargo/bin/cargo` (the mise shim fails). Never point `CARGO_TARGET_DIR` at `/tmp`.
- The four checks, from `AGENTS.md`. "Run the four checks" below means these, and all four pass after every task:

```bash
~/.cargo/bin/cargo fmt --all --check
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
~/.cargo/bin/cargo test --locked --workspace --all-targets
RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --locked --workspace --no-deps
```

- The blocks below are not all as rustfmt lays them out. Run `~/.cargo/bin/cargo fmt --all` before the four checks of each task, and commit what it gives.
- After tasks 2 and 3, also: `~/.cargo/bin/cargo clippy --locked --features shots --lib --tests -- -D warnings`.
- One commit per task, signed. `error: 1Password: agent refused operation` means 1Password is locked: stage the task, record its tree (`git write-tree`), go on, and commit the trees in order once it is unlocked. Never commit unsigned.
- Commit messages end with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- Write each test first and see it fail before the code that makes it pass. Slice 1 did not, and its plan says so.
- Tests use the Bookshop's `book_covers` only. No colour, size or picture of the design is checked.
- UI copy is short and plain, with no em dashes.

## Files

| File | What it gets |
|---|---|
| `src/model.rs` | `ObjectTab::visual`, `toggle_visual`, `rows_from_cursor`. `Action::SelectRows`, `LeaveVisual`, `DeleteCount` |
| `src/app.rs` | The reducers of the three |
| `src/app/editing.rs` | `delete_selected` leaves visual. `sweep_rows` says whether it acted. `delete_count` |
| `src/keymap.rs` | `When::Visual`. `Command::SelectRows`, `DeleteSelection`, `LeaveVisual`. `count_of`, `is_count`, `Command::takes_count`, `Typed::Counted` |
| `src/ui/keys.rs` | `V`, `d`, `esc`, the commands held back, the count |
| `src/ui/workspace.rs` | The `-- VISUAL --` mode line |
| `src/ui/data_view.rs` | What a screen reader is told of a visual selection |
| `src/ui/delete_many_tests.rs` | The spec's tests for this slice |
| `docs/_guide/omarchy.md`, `docs/_guide/editing-data.md`, `docs/_reference/keyboard-shortcuts.md` | `V`, `d`, `{count}dd` |

---

### Task 1: A visual selection in the model, and what a delete does to it

**Files:**
- Modify: `src/model.rs` (`impl ObjectTab` after `selected_rows`, `Action` after `ExtendBy`, tests)
- Modify: `src/app.rs` (after the `ExtendBy` arm), `src/app/editing.rs` (`dropped_under_a_prompt`, `delete_selected`, `sweep_rows`)
- Test: `src/model.rs`, `src/ui/delete_many_tests.rs`

- [ ] **Step 1: Write the failing tests**

`src/model.rs`, in `mod tests`, after `a_range_anchored_on_a_new_row_ends_when_the_new_rows_go`:

```rust
    #[test]
    fn a_visual_selection_is_started_and_left_on_the_cursors_row() {
        let mut tab = covers(5);
        // With no cursor there is none to start.
        tab.toggle_visual();
        assert!(!tab.visual());
        tab.set_cursor(at(2), Reach::End);
        tab.toggle_visual();
        assert!(tab.visual());
        assert_eq!(tab.selected_rows(), [2]);
        // Moves extend it, up and down.
        tab.set_cursor(at(4), Reach::Move);
        assert_eq!(tab.selected_rows(), [2, 3, 4]);
        tab.set_cursor(at(0), Reach::Move);
        assert_eq!(tab.selected_rows(), [0, 1, 2]);
        tab.toggle_visual();
        assert_eq!((tab.visual(), tab.selected_rows()), (false, vec![0]));
        // A range made with Shift becomes the visual one, from its anchor.
        tab.set_cursor(at(1), Reach::Extend);
        tab.toggle_visual();
        assert!(tab.visual());
        tab.set_cursor(at(3), Reach::Move);
        assert_eq!(tab.selected_rows(), [0, 1, 2, 3]);
    }
```

`src/ui/delete_many_tests.rs`, at its end:

```rust
/// Whether a visual selection of rows is on.
fn visual(harness: &Harness, tab: ConnTabId, id: TabId) -> bool {
    let workspace = harness.app.workspace(tab).unwrap();
    workspace.object_tab(id).unwrap().visual()
}

/// Spec 1 (Omarchy) and spec 2: the delete of a visual selection marks its
/// rows, leaves visual and puts the cursor on the first of them. Leaving
/// it marks nothing and keeps the cursor.
#[test]
fn a_visual_selections_delete_marks_its_rows_and_leaves_it() {
    let (mut harness, tab, id) = covers_with(Look::omarchy(), book_covers_page(5));
    select(&mut harness, tab, id, (3, 2));
    harness.app.apply(Action::SelectRows { tab, id });
    assert!(visual(&harness, tab, id));
    // Up from the anchor: the first row of the range is the cursor's.
    let (rows_up, cols) = (-2, 0);
    harness.app.apply(Action::MoveSelection {
        tab,
        id,
        rows: rows_up,
        cols,
    });
    assert_eq!(rows(&harness, tab, id), [1, 2, 3]);
    harness.app.apply(Action::DeleteSelected { tab, id });
    assert_eq!(marked(&harness, tab, id), [1, 2, 3]);
    assert!(!visual(&harness, tab, id));
    assert_eq!(selected(&harness, tab, id), Some((1, 2)));
    assert_eq!(rows(&harness, tab, id), [1]);
    // Down from the anchor: the cursor goes back to the first row.
    select(&mut harness, tab, id, (1, 2));
    harness.app.apply(Action::SelectRows { tab, id });
    harness.app.apply(Action::MoveSelection {
        tab,
        id,
        rows: 2,
        cols,
    });
    harness.app.apply(Action::DeleteSelected { tab, id });
    assert!(marked(&harness, tab, id).is_empty());
    assert_eq!(selected(&harness, tab, id), Some((1, 2)));
    // Left, it marks nothing and the cursor stays.
    harness.app.apply(Action::SelectRows { tab, id });
    harness.app.apply(Action::MoveSelection {
        tab,
        id,
        rows: 2,
        cols,
    });
    harness.app.apply(Action::LeaveVisual { tab, id });
    assert!(!visual(&harness, tab, id));
    assert!(marked(&harness, tab, id).is_empty());
    assert_eq!(selected(&harness, tab, id), Some((3, 2)));
}

/// Spec 2: a visual selection's delete drops its new rows, and the cursor
/// is on the first row of it that is left.
#[test]
fn a_visual_selections_delete_lands_on_the_first_row_left() {
    let (mut harness, tab, id) = covers_with(Look::omarchy(), book_covers_page(4));
    let new = add_below(&mut harness, tab, id, 0);
    select(&mut harness, tab, id, (new, 1));
    harness.app.apply(Action::SelectRows { tab, id });
    let (rows_down, cols) = (2, 0);
    harness.app.apply(Action::MoveSelection {
        tab,
        id,
        rows: rows_down,
        cols,
    });
    assert_eq!(rows(&harness, tab, id), [new, 1, 2]);
    harness.app.apply(Action::DeleteSelected { tab, id });
    assert!(edits(&harness, tab, id).added.is_empty());
    assert_eq!(marked(&harness, tab, id), [1, 2]);
    assert!(!visual(&harness, tab, id));
    assert_eq!(selected(&harness, tab, id), Some((1, 1)));
}
```

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked --lib a_visual_selection`
Expected: does not compile: `no method named toggle_visual`, `no variant named SelectRows`.

- [ ] **Step 3: The model**

`src/model.rs`, in `impl ObjectTab`, after `selected_rows`:

```rust
    /// Whether a visual selection of rows is on: a range that moves keep.
    pub fn visual(&self) -> bool {
        self.range.is_some_and(|range| range.held)
    }

    /// Starts a visual selection of rows on the cursor's row, or leaves
    /// the one that is on. A range that stands becomes it, from its
    /// anchor. With no cursor there is none to start.
    pub fn toggle_visual(&mut self) {
        if self.visual() {
            self.range = None;
            return;
        }
        if let Some(cell) = self.selection {
            let anchor = self.range.map_or(cell.row, |range| range.anchor);
            self.range = Some(RowRange { anchor, held: true });
        }
    }
```

In `Action`, after `ExtendBy { … },`:

```rust
    /// Start a visual selection of rows on the cursor's row, or leave the
    /// one that is on: the terminal look's `V`.
    SelectRows {
        tab: ConnTabId,
        id: TabId,
    },
    /// Leave a visual selection of rows. The cursor stays.
    LeaveVisual {
        tab: ConnTabId,
        id: TabId,
    },
```

- [ ] **Step 4: The reducers**

`src/app.rs`, after the `Action::ExtendBy` arm:

```rust
            Action::SelectRows { tab, id } => {
                // As a move does: an editor's text is kept, and the keys
                // are the rows'.
                self.close_editor(tab, id, true);
                if let Some(object) = self.object_tab_mut(tab, id) {
                    object.edits.why = None;
                    object.focus_field = None;
                    object.toggle_visual();
                }
            }
            Action::LeaveVisual { tab, id } => {
                if let Some(object) = self.object_tab_mut(tab, id)
                    && object.visual()
                {
                    object.range = None;
                }
            }
```

`src/app/editing.rs`, in `dropped_under_a_prompt`, after `| Action::ExtendBy { .. }`: `| Action::SelectRows { .. }`.

`sweep_rows` says whether it did anything. Its signature becomes `fn sweep_rows(&mut self, tab: ConnTabId, id: TabId, rows: &[usize]) -> bool`: each `return;` in it becomes `return false;`, and it ends with `true` after `object.fields = None;`.

`delete_selected` becomes:

```rust
    /// Deletes the selected rows. One row is [`Self::delete_row`]'s. A
    /// visual selection is left once its rows were acted on, the cursor on
    /// the first of them that is left.
    pub(super) fn delete_selected(&mut self, tab: ConnTabId, id: TabId) {
        let found = self
            .workspace(tab)
            .and_then(|workspace| workspace.object_tab(id))
            .map(|object| (object.selected_rows(), object.visual()));
        let Some((rows, visual)) = found else {
            return;
        };
        let acted = if rows.len() < 2 {
            self.delete_row(tab, id);
            true
        } else {
            self.sweep_rows(tab, id, &rows)
        };
        if !(visual && acted) {
            return;
        }
        let Some(object) = self.object_tab_mut(tab, id) else {
            return;
        };
        // A row of the page is there still, and a new row where the mark
        // did not drop it.
        let left = |row: &&usize| match new_id(**row) {
            Some(new) => object.edits.added.iter().any(|added| added.id == new),
            None => true,
        };
        let first = rows.iter().find(left).copied();
        let col = object.selection.map_or(0, |cell| cell.col);
        match first {
            Some(row) => object.set_cursor(Some(CellPos { row, col }), Reach::End),
            // None of them is left: the cursor is where the drop put it.
            None => object.range = None,
        }
    }
```

(`left` borrows `object.edits` and ends before `set_cursor`: `first` is a `usize` by then.)

- [ ] **Step 5: Run the tests**

Run: `~/.cargo/bin/cargo fmt --all`, `~/.cargo/bin/cargo test --locked --lib a_visual_selection`, then the four checks.
Expected: the three pass. Slice 1's tests still pass: with no held range `delete_selected` does what it did.

- [ ] **Step 6: Commit**

```bash
git add src/model.rs src/app.rs src/app/editing.rs src/ui/delete_many_tests.rs
git commit -m "Hold a selection of rows through moves, and leave it once deleted

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: `V`, `d` and `esc`, and the keys a visual selection holds back

**Files:**
- Modify: `src/keymap.rs` (`When` at 112, the enum after `DeleteRow`, `info`, the lines after `DeleteRow`'s at 1190, tests)
- Modify: `src/ui/keys.rs` (`editing_keys` at 1150, `letters` at 1476: its `holds` at 1795, its Esc block at 1693, its loop at 1865; `flips` at 477)
- Test: `src/ui/delete_many_tests.rs`

- [ ] **Step 1: Write the failing tests**

Add `type_key` to the test file's `use super::insert_audit_tests::{…}`, and at its end:

```rust
/// Spec 1 (Omarchy) and test 3: `V` starts a visual selection, `j` and `k`
/// extend it, the keys that act on one cell or row do nothing in it, a
/// move along the row keeps its rows, and `d` marks them and leaves it.
#[test]
fn v_selects_rows_and_d_marks_them() {
    let (mut harness, tab, id) = covers_with(Look::omarchy(), book_covers_page(5));
    select(&mut harness, tab, id, (1, 2));
    type_key(&mut harness, Key::V, "V");
    assert!(visual(&harness, tab, id));
    type_key(&mut harness, Key::J, "j");
    type_key(&mut harness, Key::J, "j");
    assert_eq!(rows(&harness, tab, id), [1, 2, 3]);
    // Held back: an editor, a new row, Set NULL, a followed key, the
    // inspector's fields.
    type_key(&mut harness, Key::I, "i");
    type_key(&mut harness, Key::O, "o");
    type_key(&mut harness, Key::X, "x");
    type_key(&mut harness, Key::G, "g");
    type_key(&mut harness, Key::D, "d");
    harness.press(Key::Enter, Modifiers::NONE);
    let pending = edits(&harness, tab, id);
    assert!(pending.editor.is_none() && pending.added.is_empty() && pending.cells.is_empty());
    assert!(marked(&harness, tab, id).is_empty());
    let workspace = harness.app.workspace(tab).unwrap();
    assert_eq!(workspace.object_tab(id).unwrap().focus_field, None);
    assert_eq!(rows(&harness, tab, id), [1, 2, 3]);
    // Along the row the cell changes and the rows do not.
    type_key(&mut harness, Key::L, "l");
    assert_eq!(selected(&harness, tab, id), Some((3, 3)));
    assert_eq!(rows(&harness, tab, id), [1, 2, 3]);
    type_key(&mut harness, Key::D, "d");
    assert_eq!(marked(&harness, tab, id), [1, 2, 3]);
    assert!(!visual(&harness, tab, id));
    assert_eq!(selected(&harness, tab, id), Some((1, 3)));
    // On one row too: `V`, `d` marks it and leaves the selection.
    select(&mut harness, tab, id, (4, 2));
    type_key(&mut harness, Key::V, "V");
    type_key(&mut harness, Key::D, "d");
    assert_eq!(marked(&harness, tab, id), [1, 2, 3, 4]);
    assert!(!visual(&harness, tab, id));
}

/// Test 3: `esc` and a second `V` leave a visual selection and mark
/// nothing, and a save is not held back.
#[test]
fn esc_and_v_leave_a_visual_selection() {
    let (mut harness, tab, id) = covers_with(Look::omarchy(), book_covers_page(5));
    select(&mut harness, tab, id, (1, 2));
    type_key(&mut harness, Key::V, "V");
    type_key(&mut harness, Key::J, "j");
    assert!(visual(&harness, tab, id));
    assert_eq!(rows(&harness, tab, id), [1, 2]);
    harness.press(Key::Escape, Modifiers::NONE);
    assert!(!visual(&harness, tab, id));
    assert!(marked(&harness, tab, id).is_empty());
    assert_eq!(selected(&harness, tab, id), Some((2, 2)));
    type_key(&mut harness, Key::V, "V");
    assert!(visual(&harness, tab, id));
    type_key(&mut harness, Key::V, "V");
    assert!(!visual(&harness, tab, id));
    // With something pending, the save's chord works in it.
    set(&mut harness, tab, id, (0, 2), "audio");
    select(&mut harness, tab, id, (1, 2));
    type_key(&mut harness, Key::V, "V");
    harness.press(Key::S, Modifiers::COMMAND);
    assert!(edits(&harness, tab, id).saving.is_some());
}

/// Spec 1 (Omarchy) and test 4: with something else for `esc` to close,
/// the first `esc` leaves the visual selection and the next does what it
/// does today.
#[test]
fn esc_leaves_a_visual_selection_before_it_closes_anything() {
    // The Review SQL panel.
    let (mut harness, tab, id) = covers_with(Look::omarchy(), book_covers_page(5));
    set(&mut harness, tab, id, (0, 2), "audio");
    let show = true;
    harness.app.apply(Action::ReviewEdits { tab, id, show });
    select(&mut harness, tab, id, (1, 2));
    type_key(&mut harness, Key::V, "V");
    harness.press(Key::Escape, Modifiers::NONE);
    assert!(!visual(&harness, tab, id));
    assert!(edits(&harness, tab, id).reviewing);
    harness.press(Key::Escape, Modifiers::NONE);
    assert!(!edits(&harness, tab, id).reviewing);
    // The row panel.
    let (mut harness, tab, id) = covers_with(Look::omarchy(), book_covers_page(5));
    select(&mut harness, tab, id, (1, 2));
    harness.app.workspace_mut(tab).unwrap().row_panel = true;
    type_key(&mut harness, Key::V, "V");
    harness.press(Key::Escape, Modifiers::NONE);
    assert!(!visual(&harness, tab, id));
    assert!(harness.app.workspace(tab).unwrap().row_panel);
    harness.press(Key::Escape, Modifiers::NONE);
    assert!(!harness.app.workspace(tab).unwrap().row_panel);
    // What the last save came to: a row it found already deleted, which
    // is what the line keeps saying until it is dismissed.
    let (mut harness, tab, id) = covers_with(Look::omarchy(), book_covers_page(5));
    harness.app.workspace_mut(tab).unwrap().row_panel = false;
    select(&mut harness, tab, id, (0, 1));
    harness.app.apply(Action::DeleteRow { tab, id });
    harness.app.apply(Action::WriteEdits { tab, id });
    harness.answer_written(Ok(tabletist_db::WriteOutcome::Written {
        inserted: Vec::new(),
        rows: Vec::new(),
        gone: vec![0],
        elapsed: std::time::Duration::from_millis(14),
    }));
    harness.settle();
    assert!(edits(&harness, tab, id).note.is_some());
    select(&mut harness, tab, id, (1, 2));
    type_key(&mut harness, Key::V, "V");
    harness.press(Key::Escape, Modifiers::NONE);
    assert!(!visual(&harness, tab, id));
    assert!(edits(&harness, tab, id).note.is_some());
    harness.press(Key::Escape, Modifiers::NONE);
    assert!(edits(&harness, tab, id).note.is_none());
}
```

A plain successful save sets `saved`, not `note`: the line's Esc dismisses only a `note` with nothing pending (`When::Note`), and a row found already gone leaves one.

In the keymap's tests (`src/keymap.rs`), after `typed_characters_are_read_one_after_another`:

```rust
    #[test]
    fn a_visual_selection_has_its_own_keys() {
        let keymap = Keymap::default();
        let visual = |when: When| when == When::Visual;
        let nothing = |_: When| false;
        let grid = |holds: &dyn Fn(When) -> bool, typed: &str| {
            keymap.typed(Layout::Omarchy, Scope::Grid, holds, typed)
        };
        assert_eq!(grid(&nothing, "V"), Typed::Command(Command::SelectRows));
        // `d` alone is the whole key while a selection is on, and the
        // first of two without one.
        assert_eq!(grid(&visual, "d"), Typed::Command(Command::DeleteSelection));
        assert_eq!(grid(&nothing, "d"), Typed::Waiting);
        assert_eq!(grid(&nothing, "dd"), Typed::Command(Command::DeleteRow));
        assert_eq!(keymap.label(Layout::Omarchy, Command::LeaveVisual), "esc");
        assert_eq!(keymap.label(Layout::Mac, Command::SelectRows), "");
    }
```

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked --lib a_visual_selection_has`
Expected: does not compile: `no variant named Visual`.

- [ ] **Step 3: The keymap**

`src/keymap.rs`. In `enum When`, after `KeyPicker`:

```rust
    /// A visual selection of rows is on, in the terminal look.
    Visual,
```

In the command enum, after `DeleteRow,`: `SelectRows,`, `DeleteSelection,`, `LeaveVisual,`. In `info`, after the `C::DeleteRow` arm:

```rust
            C::SelectRows => info("Select rows", "", Editing),
            C::DeleteSelection => info("Delete the selected rows", "", Editing),
            C::LeaveVisual => info("Leave the selection of rows", "", Editing),
```

In the lines, after `bind(C::AddRow, INSPECTOR, NONE, keys(&[k("o")])).when(When::NewRow),`:

```rust
        // The terminal look's visual selection of rows: `V` starts and
        // leaves it, and while it is on `d` alone deletes its rows.
        bind(C::SelectRows, GRID, NONE, keys(&[k("V")])),
        bind(C::DeleteSelection, GRID, NONE, keys(&[k("d")])).when(When::Visual),
        bind(C::LeaveVisual, GRID, NONE, keys(&[k("esc")])).when(When::Visual),
```

Run: `~/.cargo/bin/cargo test --locked --lib keymap::`
Expected: the new test passes, and so do the clash tests: a line with a `when` shares a key only with lines of the same `when`.

- [ ] **Step 4: The view reads them**

`src/ui/keys.rs`, in `letters`. Where `on_new` is made (1662), add:

```rust
    // A visual selection of rows is on: its keys are read, and the keys
    // that act on one cell or one row are held back.
    let visual = object.is_some_and(|object| object.visual());
```

In `holds` (1795), before `_ => false`: `crate::keymap::When::Visual => visual,`.

Esc. Before `if let Some(id) = reviewing` (1699), so that it is asked first and takes the key:

```rust
    // A visual selection first: it is the innermost thing the key leaves.
    if let Some(id) = active.filter(|_| visual)
        && quiet
        && on(Command::LeaveVisual, Scope::Grid).fresh
    {
        actions.push(Action::LeaveVisual { tab, id });
    }
```

The loop. After `take_char(ctx, char); waiting.clear();` (1895) and before `match (command, active) {`:

```rust
        let command = command.filter(|command| !(visual && held_back(*command)));
```

and two arms in the match, before the `Command::DeleteRow` arm:

```rust
            (Some(Command::SelectRows), Some(id))
                if rows_show && !tree && field.is_none() =>
            {
                actions.push(Action::SelectRows { tab, id });
            }
            // The rows of the visual selection. Where none of them is
            // deleted the selection stays, to be made smaller.
            (Some(Command::DeleteSelection), Some(id))
                if rows_show && !tree && field.is_none() =>
            {
                let from = actions.len();
                let why = delete_selected(app, tab, id, actions);
                // One row's delete comes back as its own action: the
                // selection's is what leaves the visual selection.
                for action in &mut actions[from..] {
                    if matches!(action, Action::DeleteRow { .. }) {
                        *action = Action::DeleteSelected { tab, id };
                    }
                }
                if let Some(why) = why {
                    crate::ui::toast::say(ctx, &why);
                }
            }
```

(The reducer's `delete_selected` treats one row as `delete_row` does, then leaves visual: Task 1.)

As a function of the file, after `delete_selected`:

```rust
/// Whether a visual selection of rows holds `command` back: what changes,
/// copies or follows one cell or one row. The moves, the inspector's
/// show and hide, a save, a discard, Review SQL, a reload, the filters and
/// the pages are not among them.
fn held_back(command: Command) -> bool {
    matches!(
        command,
        Command::EditCell
            | Command::ReplaceCell
            | Command::OpenInspector
            | Command::SetNull
            | Command::SetDefault
            | Command::UndoCell
            | Command::RedoCell
            | Command::CopyCells
            | Command::CopyRow
            | Command::PasteCells
            | Command::PasteNewRows
            | Command::AddRow
            | Command::AddRowAbove
            | Command::AddRowAtEnd
            | Command::DuplicateRow
            | Command::YankRow
            | Command::PutRow
            | Command::PutRowAbove
            | Command::DeleteRow
            | Command::OpenReferencedRow
            | Command::ShowSavedRow
            | Command::BooleanCycle
    )
}
```

The chords. In `editing_keys` (1150), where `open` is made (1162):

```rust
    // The terminal look's visual selection of rows holds back the chords
    // that act on one cell or one row.
    let visual = app.look.terminal && object.visual();
```

and inside `ctx.input_mut(|input| {`, right after the `Command::DiscardChanges` read:

```rust
        if visual {
            return;
        }
```

`flips` (477) gains the same guard, so space does not turn a boolean:

```rust
    let flips = object
        .filter(|_| grid && !editing && !tree_arrows && !focused)
        .filter(|&(tab, id)| app.flips(tab, id))
        .filter(|&(tab, id)| {
            let object = app.workspace(tab).and_then(|workspace| workspace.object_tab(id));
            !(app.look.terminal && object.is_some_and(|object| object.visual()))
        });
```

The first block's `AddRow` and `DuplicateRow` chords (608, 619) have no key in the terminal look and need no guard.

- [ ] **Step 5: Run the tests**

Run: `~/.cargo/bin/cargo fmt --all`, `~/.cargo/bin/cargo test --locked --lib delete_many_tests`, then the four checks and the shots' clippy.
Expected: PASS. If `v_selects_rows…` fails at Enter (the inspector's fields took the keyboard), the read of `OpenInspector` in `editing_keys` is above the guard: it is inside the same `input_mut` closure, below `DiscardChanges`, and the guard must stand before it.

- [ ] **Step 6: Commit**

```bash
git add src/keymap.rs src/ui/keys.rs src/ui/delete_many_tests.rs
git commit -m "Select rows with V in the terminal look, and delete them with d

Esc and a second V leave the selection. While it is on, the keys that
act on one cell or one row do nothing.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: The mode line, and what a screen reader is told

**Files:**
- Modify: `src/ui/workspace.rs` (`Editing` at 1573, `editing_status` at 1637, the keys at 1941, the line before `if let Some(column) = &editing.insert` at 2138)
- Modify: `src/ui/data_view.rs` (beside `told`, about 1380, and where it is announced, about 1620)
- Test: `src/ui/delete_many_tests.rs`

- [ ] **Step 1: Write the failing test**

At the test file's end (`painted` and `covers_as` are in its `use` already):

```rust
/// Spec 1 (Omarchy), spec 2 and test 3: the status line says the mode, how
/// many rows the selection holds and its two keys, and a screen reader is
/// told the same as it grows.
#[test]
fn a_visual_selection_shows_in_the_status_line_and_is_told() {
    let (mut harness, tab, id) = covers_with(Look::omarchy(), book_covers_page(5));
    harness.app.workspace_mut(tab).unwrap().row_panel = false;
    select(&mut harness, tab, id, (1, 2));
    assert!(!painted(&harness, "-- VISUAL --"));
    type_key(&mut harness, Key::V, "V");
    assert!(painted(&harness, "-- VISUAL --"));
    assert!(painted(&harness, "1 row"));
    assert!(polite(&mut harness, "Visual. 1 row selected."));
    type_key(&mut harness, Key::J, "j");
    type_key(&mut harness, Key::J, "j");
    assert!(painted(&harness, "3 rows"));
    assert!(polite(&mut harness, "Visual. 3 rows selected."));
    // The mode is in the palette's magenta, as the board's is.
    let magenta = harness.app.palette.magenta;
    let mode = |(text, color): &(String, egui::Color32)| text == "-- VISUAL --" && *color == magenta;
    assert!(harness.painted.iter().any(mode), "{:?}", harness.painted);
    // Its keys, one piece, in place of the grid's.
    assert!(painted(&harness, "d delete · esc normal"));
    assert!(!painted(&harness, "dd delete"));
    harness.press(Key::Escape, Modifiers::NONE);
    assert!(!painted(&harness, "-- VISUAL --"));
    assert!(painted(&harness, "dd delete"));
}

/// Q3: a delete the table refuses leaves the visual selection on, and the
/// status line says why in place of its keys.
#[test]
fn a_refused_delete_is_said_on_the_visual_line() {
    let table = tabletist_db::ObjectKind::Table;
    let (mut harness, tab, id) = covers_as(Look::omarchy(), table, true);
    harness.app.workspace_mut(tab).unwrap().row_panel = false;
    select(&mut harness, tab, id, (0, 1));
    type_key(&mut harness, Key::V, "V");
    type_key(&mut harness, Key::J, "j");
    type_key(&mut harness, Key::D, "d");
    assert!(visual(&harness, tab, id));
    assert!(marked(&harness, tab, id).is_empty());
    assert!(painted(&harness, "-- VISUAL --"));
    assert!(painted(&harness, "read-only connection"));
    assert!(!painted(&harness, "d delete · esc normal"));
}
```

The mode's colour is the one token the board gives the word: it is checked as `-- INSERT --`'s is in `src/ui/mod.rs` (15628), not as a picture.

- [ ] **Step 2: Run it to see it fail**

Run: `~/.cargo/bin/cargo test --locked --lib a_visual_selection_shows`
Expected: FAIL at `-- VISUAL --`.

- [ ] **Step 3: The status line**

`src/ui/workspace.rs`. In `struct Editing`, after `insert`:

```rust
    /// A visual selection of rows is on, with this many rows.
    visual: Option<usize>,
```

In `editing_status`, where `can_delete` is made (1676). `object` there is the tab itself, and `data` says its rows show, which keeps the mode off the Structure view:

```rust
    let visual = (data && object.visual()).then(|| object.selected_rows().len());
```

and `visual,` in the one `Editing { … }` literal (1821).

With the keys (1941):

```rust
    let (visual_delete_key, leave_visual_key) =
        (key(Command::DeleteSelection), key(Command::LeaveVisual));
```

Before `if let Some(column) = &editing.insert {` (2138):

```rust
            if let Some(rows) = editing.visual {
                // A visual selection of rows: the mode, how many rows it
                // holds and the counts. Its keys, at the right, give way.
                let mode = gettext(locale, "-- VISUAL --");
                let mode = || Text::one(&look, TextRole::OModeLine, &mode, palette.magenta);
                let held = crate::ui::pending_bar::counted(locale, rows, "row", "rows");
                let deletes = gettext(locale, "delete");
                let normal = gettext(locale, "normal");
                let keys = || {
                    Text::new(&look)
                        .add(role, &visual_delete_key, palette.text)
                        .space(role, " ")
                        .add(role, &deletes, palette.dim)
                        .add(role, " · ", palette.dim)
                        .add(role, &leave_visual_key, palette.text)
                        .space(role, " ")
                        .add(role, &normal, palette.dim)
                };
                let right = rect.right() - 12.0;
                let mut x = left + widgets::paint_label(ui, left, y, mode()) + gap;
                let text = Text::one(&look, role, &held, palette.secondary);
                x += widgets::paint_label(ui, x, y, text) + gap;
                x = counts(ui, x);
                if let Some(said) = &editing.said {
                    // Why its delete was refused, in place of the keys:
                    // the selection stays on, and this look has no toast
                    // but the line.
                    let room = (right - x).max(0.0);
                    let shown = crate::ui::grid::ellipsize(&said.text, room, false, measure);
                    let text = Text::one(&look, role, &shown, said.color);
                    let width = widgets::measure(ui, text);
                    let text = Text::one(&look, role, &shown, said.color);
                    widgets::paint_text(ui, right - width, y, text);
                    // The whole of it for a screen reader, cut or not.
                    let place = Rect::from_min_size(
                        pos2(right - width, y - 8.0),
                        vec2(width.max(1.0), 16.0),
                    );
                    widgets::announce(ui, place, &said.text);
                } else if x + widgets::measure(ui, keys()) <= right {
                    widgets::paint_text_right(ui, right, y, keys());
                }
                return;
            }
```

`editing.said` is what the normal line says after its counts (2304): a lock's reason, what a save came to, or the toast of a refusal. It is drawn the same way here, at the right. Read 2231 to 2320 first and use that block's helpers by the names they have there (`measure`, `ellipsize`, `announce`); where `widgets::paint_text_right` takes a `Text`, use it in place of the measured `paint_text`.

- [ ] **Step 4: What is told**

`src/ui/data_view.rs`, after the `let told = match … ;` block (it ends about 1420), while the tab is still only read:

```rust
    // A visual selection of rows, and how many it holds: said when it
    // starts and each time it grows or shrinks.
    let visual = (look.terminal && object.visual()).then(|| {
        let rows = object.selected_rows().len();
        let held = super::pending_bar::counted(locale, rows, "row selected", "rows selected");
        format!("{} {held}.", gettext(locale, "Visual."))
    });
```

and where `told` is announced (after `if let Some(words) = &told { … }`):

```rust
        if let Some(words) = &visual {
            // On a point of its own, as the others are.
            let at = Rect::from_min_size(ui.max_rect().min + vec2(4.0, 0.0), vec2(1.0, 1.0));
            widgets::announce_polite(ui, at, words);
        }
```

- [ ] **Step 5: Run the tests**

Run: `~/.cargo/bin/cargo fmt --all`, `~/.cargo/bin/cargo test --locked --lib delete_many_tests`, then the four checks and the shots' clippy.
Expected: PASS.

- [ ] **Step 6: Look at it**

A throwaway scene at the end of `src/shots.rs` (not committed; copy the file aside first and put it back after): the covers, the cursor on the second row, `Action::SelectRows`, then `Action::MoveSelection` two rows down. Render it (`~/.cargo/bin/cargo test --locked --features shots --lib shots::<name> -- --ignored`), crop the Omarchy pictures, light and dark, to the grid and the status line, and read them: the three rows take the selection's fill, the cursor's row has the gutter's cursor and its cell the outline, and the line reads `-- VISUAL -- 3 rows` with the keys at its right. Put `src/shots.rs` back and delete the pictures. They stay local.

- [ ] **Step 7: Commit**

```bash
git add src/ui/workspace.rs src/ui/data_view.rs src/ui/delete_many_tests.rs
git commit -m "Say on the status line that a selection of rows is on

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: A count before `dd`

**Files:**
- Modify: `src/keymap.rs` (`Typed` at 1883, `typed` at 2052, a function beside it, tests)
- Modify: `src/model.rs` (`impl ObjectTab`, `Action`), `src/app.rs`, `src/app/editing.rs`
- Modify: `src/ui/keys.rs` (54, 1614, 1836, 1877, 1883, the `DeleteRow` arm)
- Test: `src/keymap.rs`, `src/model.rs`, `src/ui/delete_many_tests.rs`

- [ ] **Step 1: Write the failing tests**

`src/keymap.rs`, in its tests:

```rust
    #[test]
    fn a_count_is_read_before_the_key_that_takes_one() {
        let keymap = Keymap::default();
        let nothing = |_: When| false;
        let read = |layout, scope, typed: &str| keymap.typed(layout, scope, &nothing, typed);
        let grid = |typed: &str| read(Layout::Omarchy, Scope::Grid, typed);
        assert_eq!(grid("3"), Typed::Waiting);
        assert_eq!(grid("3d"), Typed::Waiting);
        assert_eq!(grid("3dd"), Typed::Counted(Command::DeleteRow, 3));
        assert_eq!(grid("12dd"), Typed::Counted(Command::DeleteRow, 12));
        assert_eq!(grid("1234567dd"), Typed::Counted(Command::DeleteRow, 999_999));
        // Before any other key the count is dropped, and the key acts once.
        assert_eq!(grid("3j"), Typed::Command(Command::MoveRow));
        // Nought begins no count.
        assert_eq!(grid("0"), Typed::Nothing);
        assert_eq!(grid("0dd"), Typed::Nothing);
        // Only the terminal look's grid of a table reads one.
        assert_eq!(read(Layout::Omarchy, Scope::Results, "3"), Typed::Nothing);
        assert_eq!(read(Layout::Omarchy, Scope::Inspector, "3"), Typed::Nothing);
        assert_eq!(read(Layout::Mac, Scope::Grid, "3"), Typed::Nothing);
        assert_eq!(count_of("12dd"), (Some(12), "dd"));
        assert_eq!(count_of("dd"), (None, "dd"));
        assert!(is_count("12") && !is_count("12d") && !is_count("") && !is_count("0"));
    }
```

`src/model.rs`, in its tests:

```rust
    #[test]
    fn a_count_names_the_cursors_row_and_the_rows_after_it() {
        let mut tab = covers(5);
        assert!(tab.rows_from_cursor(3).is_empty());
        tab.set_cursor(at(1), Reach::End);
        assert_eq!(tab.rows_from_cursor(1), [1]);
        assert_eq!(tab.rows_from_cursor(3), [1, 2, 3]);
        // It stops at the grid's last row.
        assert_eq!(tab.rows_from_cursor(999_999), [1, 2, 3, 4]);
        // A new row counts where it stands.
        let new = crate::edit::new_row(tab.edits.add_row(crate::edit::Place::Below(1), 5));
        assert_eq!(tab.rows_from_cursor(3), [1, new, 2]);
    }
```

`src/ui/delete_many_tests.rs`, at its end:

```rust
/// Types `text` on the grid, a key at a time, as digits and letters come.
fn type_keys(harness: &mut Harness, text: &str) {
    for char in text.chars() {
        let key = Key::from_name(&char.to_uppercase().to_string()).expect("a key of that name");
        type_key(harness, key, &char.to_string());
    }
}

/// Spec 2 and 3, M10 and test 5: a count before `dd` marks the cursor's
/// row and the rows after it, stops at the last row, and puts the cursor
/// after the last row it named.
#[test]
fn a_count_before_dd_marks_that_many_rows_down() {
    let (mut harness, tab, id) = covers_with(Look::omarchy(), book_covers_page(6));
    select(&mut harness, tab, id, (1, 2));
    type_keys(&mut harness, "3dd");
    assert_eq!(marked(&harness, tab, id), [1, 2, 3]);
    assert_eq!(selected(&harness, tab, id), Some((4, 2)));
    assert_eq!(rows(&harness, tab, id), [4]);
    // Two rows from the end, five asked for: two are marked, and the
    // cursor stays on the last.
    type_keys(&mut harness, "5dd");
    assert_eq!(marked(&harness, tab, id), [1, 2, 3, 4, 5]);
    assert_eq!(selected(&harness, tab, id), Some((5, 2)));
    // The same rows again are all marked: the count restores them.
    select(&mut harness, tab, id, (4, 2));
    type_keys(&mut harness, "2dd");
    assert_eq!(marked(&harness, tab, id), [1, 2, 3]);
    // A count of one moves on, where plain `dd` stays.
    select(&mut harness, tab, id, (0, 2));
    type_keys(&mut harness, "1dd");
    assert_eq!(marked(&harness, tab, id), [0, 1, 2, 3]);
    assert_eq!(selected(&harness, tab, id), Some((1, 2)));
}

/// Spec 3 and test 5: nought is no count, a count before another key is
/// dropped, and `:` after one opens the prompt.
#[test]
fn a_count_is_dropped_before_any_key_but_dd() {
    let (mut harness, tab, id) = covers_with(Look::omarchy(), book_covers_page(6));
    select(&mut harness, tab, id, (1, 2));
    type_keys(&mut harness, "0dd");
    assert_eq!(marked(&harness, tab, id), [1]);
    assert_eq!(selected(&harness, tab, id), Some((1, 2)));
    type_keys(&mut harness, "3j");
    assert_eq!(selected(&harness, tab, id), Some((2, 2)));
    assert_eq!(marked(&harness, tab, id), [1]);
    type_key(&mut harness, Key::Num3, "3");
    type_key(&mut harness, Key::Colon, ":");
    assert!(harness.app.workspace(tab).unwrap().command.is_some());
    harness.press(Key::Escape, Modifiers::NONE);
    // The count went with the prompt: `dd` marks one row.
    type_keys(&mut harness, "dd");
    assert_eq!(marked(&harness, tab, id), [1, 2]);
}

/// Spec 2 and test 6, by count: one action takes 1,000 rows and no more.
#[test]
fn a_count_over_a_thousand_rows_is_refused_with_the_message() {
    let (mut harness, tab, id) = covers_with(Look::omarchy(), book_covers_page(1_001));
    select(&mut harness, tab, id, (0, 1));
    type_keys(&mut harness, "1001dd");
    assert!(marked(&harness, tab, id).is_empty());
    assert_eq!(selected(&harness, tab, id), Some((0, 1)));
    assert_eq!(
        said(&harness).as_deref(),
        Some("select fewer rows, or delete in the sql editor.")
    );
    type_keys(&mut harness, "1000dd");
    assert_eq!(marked(&harness, tab, id).len(), 1_000);
    assert_eq!(selected(&harness, tab, id), Some((1_000, 1)));
}
```

`Key::from_name` reads `"3"` as `Key::Num3` and `"D"` as `Key::D`.

`a_count_is_dropped_before_any_key_but_dd` passes before this task but for its prompt: today a digit is nobody's and the key after it acts once. It is here to hold that, and the colon after a count.

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked --lib a_count`
Expected: does not compile: `no variant named Counted`, `cannot find function count_of`.

- [ ] **Step 3: The keymap reads a count**

`src/keymap.rs`. In `enum Typed`, after `Command(Command),`:

```rust
    /// They are this command's keys with a count before them (`3dd`), and
    /// the command takes one.
    Counted(Command, u32),
```

After `impl Command { pub fn info … }` (or beside `info`):

```rust
impl Command {
    /// Whether a count before the command's keys says how many times, or
    /// of how many rows. Before any other command's keys it is dropped.
    pub fn takes_count(self) -> bool {
        matches!(self, Self::DeleteRow)
    }
}

/// The count typed before a key, and what follows it: `12dd` is 12 and
/// `dd`. A count begins with a digit that is not nought, and more than six
/// digits read as 999,999.
pub fn count_of(typed: &str) -> (Option<u32>, &str) {
    let digits = typed.bytes().take_while(u8::is_ascii_digit).count();
    if digits == 0 || typed.starts_with('0') {
        return (None, typed);
    }
    let (count, rest) = typed.split_at(digits);
    let count = if digits > 6 {
        999_999
    } else {
        count.parse().unwrap_or(999_999)
    };
    (Some(count), rest)
}

/// Whether `typed` is a count with no key after it yet.
pub fn is_count(typed: &str) -> bool {
    matches!(count_of(typed), (Some(_), ""))
}
```

`Keymap::typed` (2052) reads the count first:

```rust
    pub fn typed(
        &self,
        layout: Layout,
        scope: Scope,
        holds: &dyn Fn(When) -> bool,
        typed: &str,
    ) -> Typed {
        // A count before the key, where one is read: on a table's rows, in
        // the terminal look.
        let (count, typed) = if layout == Layout::Omarchy && scope == Scope::Grid {
            count_of(typed)
        } else {
            (None, typed)
        };
        if count.is_some() && typed.is_empty() {
            return Typed::Waiting;
        }
        let letters = |spelled: &Spelled| -> Option<String> {
            // … as it is …
        };
        let read = self.read(
            layout,
            scope,
            holds,
            &|spelled| letters(spelled).is_some_and(|letters| letters == typed),
            &|spelled| letters(spelled).is_some_and(|letters| letters.starts_with(typed)),
        );
        match (read, count) {
            (Typed::Command(command), Some(count)) if command.takes_count() => {
                Typed::Counted(command, count)
            }
            _ => read,
        }
    }
```

`src/ui/keys.rs` matches on `Typed` in three places that must name the new case:

- 54: `Typed::Claimed | Typed::Waiting | Typed::Nothing | Typed::Counted(..) => None,` (a line of the prompt has no count).
- 1618: `Typed::Claimed | Typed::Nothing | Typed::Counted(..) => None,` (the connections list reads none).
- 1883, the grid's loop: Step 5 below.

Run: `~/.cargo/bin/cargo test --locked --lib keymap::tests::a_count`
Expected: PASS once Step 5's change to the loop lets the crate compile. Do Steps 4 and 5 before running.

- [ ] **Step 4: The rows a count names, and the reducer**

`src/model.rs`, in `impl ObjectTab`, after `toggle_visual`:

```rust
    /// The cursor's row and the rows after it in the grid's order, `count`
    /// in all, as far as the grid goes: what `{count}dd` names.
    pub fn rows_from_cursor(&self, count: u32) -> Vec<usize> {
        let Some(cell) = self.selection else {
            return Vec::new();
        };
        let height = self.page().map_or(0, |page| page.rows.len());
        let order = crate::edit::Order::of(&self.edits.added, height);
        let Some(from) = order.place(cell.row) else {
            return Vec::new();
        };
        let to = from.saturating_add(count as usize).min(order.len());
        (from..to).filter_map(|place| order.row(place)).collect()
    }
```

In `Action`, after `DeleteSelected { … },`:

```rust
    /// Delete the cursor's row and the rows after it, `count` in all, as
    /// far as the grid goes, and put the cursor after them: the terminal
    /// look's `{count}dd`.
    DeleteCount {
        tab: ConnTabId,
        id: TabId,
        count: u32,
    },
```

`src/app/editing.rs`: `| Action::DeleteCount { .. }` in `dropped_under_a_prompt`, after `DeleteSelected`. `src/app.rs`, after the `DeleteSelected` arm: `Action::DeleteCount { tab, id, count } => self.delete_count(tab, id, count),`.

`src/app/editing.rs`, after `delete_selected`:

```rust
    /// Deletes the cursor's row and the rows after it, `count` in all
    /// ([`ObjectTab::rows_from_cursor`]). A range that stands is ended,
    /// and the cursor goes to the row after the last one named, or stays
    /// on that row where the grid ends with it.
    pub(super) fn delete_count(&mut self, tab: ConnTabId, id: TabId, count: u32) {
        let found = self
            .workspace(tab)
            .and_then(|workspace| workspace.object_tab(id))
            .map(|object| {
                let rows = object.rows_from_cursor(count);
                let height = object.page().map_or(0, |page| page.rows.len());
                let order = Order::of(&object.edits.added, height);
                // The row after the last one named: no action names it, so
                // it is there afterwards.
                let after = rows
                    .last()
                    .and_then(|last| order.place(*last))
                    .and_then(|place| order.row(place + 1));
                (rows, after)
            });
        let Some((rows, after)) = found else {
            return;
        };
        let acted = match rows.len() {
            0 => false,
            1 => {
                self.delete_row(tab, id);
                true
            }
            _ => self.sweep_rows(tab, id, &rows),
        };
        if !acted {
            return;
        }
        let Some(object) = self.object_tab_mut(tab, id) else {
            return;
        };
        let last = rows.last().copied().filter(|row| match new_id(*row) {
            Some(new) => object.edits.added.iter().any(|added| added.id == new),
            None => true,
        });
        let col = object.selection.map_or(0, |cell| cell.col);
        match after.or(last) {
            Some(row) => object.set_cursor(Some(CellPos { row, col }), Reach::End),
            // The grid ended with new rows that were dropped: the cursor
            // is where the drop put it.
            None => object.range = None,
        }
    }
```

- [ ] **Step 5: The view hands the count on**

`src/ui/keys.rs`. After `delete_selected`:

```rust
/// Asks for the cursor's row and the rows after it, `count` in all, to be
/// deleted, or hands back what there is to say where a refusal is said.
fn delete_count(
    app: &App,
    tab: ConnTabId,
    id: TabId,
    count: u32,
    actions: &mut Vec<Action>,
) -> Option<String> {
    let (sweep, said) = app.workspace(tab).and_then(|workspace| {
        let object = workspace.object_tab(id)?;
        let rows = object.rows_from_cursor(count);
        if rows.is_empty() {
            return None;
        }
        let sweep = crate::edit::Table::of(workspace, object)?.sweep(&rows);
        let table = crate::ui::format::display_safe(&object.object.name);
        let said = crate::ui::cell_editor::sweep_said(&sweep, &table, &app.look, app.locale);
        Some((sweep, said))
    })?;
    if !matches!(
        sweep,
        crate::edit::Sweep::TooMany | crate::edit::Sweep::Refused(_)
    ) {
        actions.push(Action::DeleteCount { tab, id, count });
    }
    said
}
```

One row goes through `sweep` too: a row that cannot be deleted gives `Refused(lock)`, which `sweep_said` words as one row's refusal is worded today.

In `letters`, the colon after a count opens the prompt and drops the count. Both places that read `char == ':' && waiting.is_empty()` (1836 in `only_edits`, 1877 in the loop) become `waiting.is_empty() || crate::keymap::is_count(&waiting)`, and the loop's clears what was waited for:

```rust
        if char == ':' && (waiting.is_empty() || crate::keymap::is_count(&waiting)) {
            waiting.clear();
            take_char(ctx, char);
            if alone {
                opened = true;
                actions.push(Action::OpenCommand(tab));
            }
            continue;
        }
```

(In `only_edits` the closure's own `waiting` is cleared the same way before `return true`.)

The loop's match on what was read (1883) carries the count:

```rust
        let (command, count) = match read(&waiting) {
            // Nobody's: left in the frame, as any other text is.
            Typed::Nothing => {
                waiting.clear();
                continue;
            }
            Typed::Waiting => {
                take_char(ctx, char);
                continue;
            }
            // The key of something that is not built yet: taken.
            Typed::Claimed => (None, None),
            Typed::Command(command) => (Some(command), None),
            Typed::Counted(command, count) => (Some(command), Some(count)),
        };
```

and the `Command::DeleteRow` arm reads it. A count names its own rows; a form's field has none:

```rust
                let why = match count {
                    Some(count) if field.is_none() => delete_count(app, tab, id, count, actions),
                    _ if field.is_none() => delete_selected(app, tab, id, actions),
                    _ => delete_row(app, tab, id, actions),
                };
```

`only_edits` reads `Typed::Command(…)` of the keys that edit: a counted `dd` is not one of them, which is right, and its `matches!` needs no new case.

- [ ] **Step 6: Run the tests**

Run: `~/.cargo/bin/cargo fmt --all`, `~/.cargo/bin/cargo test --locked --lib a_count`, `~/.cargo/bin/cargo test --locked --lib delete_many_tests`, then the four checks.
Expected: PASS. `delete_tests::dd_marks_the_row_and_again_restores_it` still passes: `dd` without a count is `Typed::Command`.

If digits are not read on the grid at all, look at the first loop of `letters` (1496): it reads each character alone for the keys of every look, and a digit there now comes back `Waiting`, not `Nothing`. It must leave the character for the loop below, as it leaves any character that is not one of its four commands.

- [ ] **Step 7: Commit**

```bash
git add src/keymap.rs src/model.rs src/app.rs src/app/editing.rs src/ui/keys.rs src/ui/delete_many_tests.rs
git commit -m "Read a count before dd in the terminal look

3dd marks the cursor's row and the two after it, and puts the cursor
on the row after them. A count before any other key is dropped.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 5: The guide, and the checks

**Files:**
- Modify: `docs/_guide/omarchy.md:126`, `docs/_guide/editing-data.md` ("Delete a row"), `docs/_reference/keyboard-shortcuts.md`
- Modify: this plan (its status)

- [ ] **Step 1: The guide**

`docs/_guide/omarchy.md`, after the `dd` row of the editing table (126):

```markdown
| `3dd` | The same for the cursor's row and the two after it. Any count works, up to 1,000 rows. The cursor moves to the row after them |
| `V` | Start a selection of rows on the cursor's row. `j`, `k` and the other moves extend it. The status line reads `-- VISUAL --` and counts its rows |
| `d` | In a selection of rows: delete its rows, or restore them where all are marked. New rows among them are dropped |
| Esc, `V` | Leave the selection of rows |
```

Its keys table near line 55 (`| `dd` | Delete |`) gains `| `V` then `d` | Delete several rows |` after it.

`docs/_guide/editing-data.md`, in "Delete a row". The paragraph that begins `To delete several rows at once` gains, after its last sentence:

```markdown
In the Omarchy look, press `V` on a row, move to the last row with `j` or
`k`, and press `d`. A count does the same from the cursor down: `3dd`
marks three rows.
```

and the section's last paragraph loses `In the Omarchy look a press marks one row.`

`docs/_reference/keyboard-shortcuts.md`: where it sends the reader to the Omarchy look's own keys there is nothing to add. If it has a table of that look's editing keys, add the four rows above to it.

- [ ] **Step 2: Check the site**

Run: `cd docs && bin/check`
Expected: it passes.

- [ ] **Step 3: Everything, once more**

Run the four checks and the shots' clippy on the branch's head.
Expected: all pass. PostgreSQL and MySQL tests print `skipped` here: this slice changes nothing they cover.

- [ ] **Step 4: Note what was built**

Under the header of this plan, add a `Status:` line: the date, and each place a task left the plan as written. In the spec's parent, `docs/superpowers/specs/2026-10-12-deleting-rows-design.md`, the "As built" entry "One row at a time" now reads that several rows, `d` on a visual selection, `{count}dd` and the cap are built to `2026-10-13-deleting-many-rows-design.md`, and that the `IN (…)` batches are its slice 3.

- [ ] **Step 5: Commit, and push**

```bash
git add docs/_guide/omarchy.md docs/_guide/editing-data.md docs/_reference/keyboard-shortcuts.md docs/superpowers/plans/2026-10-13-deleting-many-rows-2-omarchy.md docs/superpowers/specs/2026-10-12-deleting-rows-design.md
git commit -m "Say in the guide how the terminal look deletes several rows

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
git push
```

Then check that the remote head is the commit, and bring the description of pull request `igor-alexandrov/tabletist#120` up to date: the Omarchy keys (`V`, `d`, `esc`, `{count}dd`) in its Keys table, test 3 as done, Q1 to Q6 of this plan, and "Still to come" down to slice 3.

---

## What this slice does not build

- Deletes in runs of `IN (…)`, the review's lines for a run, counts in parts and the shorter note of rows already gone: slice 3.
- A count before any key but `dd`, and `y`, `o` and `x` in a visual selection: out of the spec's scope.
