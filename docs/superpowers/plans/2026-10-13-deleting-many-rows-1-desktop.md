# Deleting Many Rows, Slice 1: A Range of Rows and Marking It Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** In a table's grid, Shift+click and Shift+↑ / Shift+↓ select a run of neighbouring rows, and Backspace marks them all for deletion, or restores them when all are marked. More than 1,000 rows in one action is refused with the spec's message.

**Architecture:** The selection keeps its one cell, the cursor, and gains a range: an anchor row (`ObjectTab::range`). Every write of the cursor goes through `ObjectTab::set_cursor`, which is told what becomes of the range. What a delete of several rows comes to is decided once, by a pure function (`Table::sweep`), which the view calls to word a refusal and the reducer calls to act. Marking and restoring a row stay what they are (`Edits::mark`, `App::restore`), called once for each row.

**Tech Stack:** Rust 1.98; egui/eframe (the crmne forks). Spec: `docs/superpowers/specs/2026-10-13-deleting-many-rows-design.md` ("spec N" is one of its sections, "test N" an entry of its Tests, "M N" a row of its decisions, all approved at their defaults on 2026-10-13). This is slice 1 of its three: sections 1 (desktop) and 2, and tests 1, 2, 6 (by selection), 7 to 12. It lands on the branch of pull request `igor-alexandrov/tabletist#120`.

Line numbers are of head `9e71520`.

---

## Where the plan leaves the spec, for a yes

| # | The spec says | The plan does | Why |
|---|---|---|---|
| P1 | `struct Range` | `RowRange` | `src/model.rs` already names `std::ops::Range` |
| P2 | One command row "Extend selection down / up", listed in Settings > Keyboard | Two commands, `ExtendUp` and `ExtendDown`, with no row of the settled table. Help lists them, as it lists `Move by row` | A key is spelled only in `src/keymap.rs` (`tests/keys.rs`), so the view cannot tell `shift+up` from `shift+down` in one command. The settled table is pinned to the board's list by a test, and the board has no such row |
| P3 | Three ways to set the cursor (extend, move, anything else) | A fourth, `Reach::Keep`: the rows under the cursor changed, not what is selected. Dropping a new row uses it | The inspector's Delete on a new row drops it and "leaves the range as it is" |
| P4 | `Action::DeleteCount`, `SelectRows`, `LeaveVisual`, `When::Visual`, `Typed::Counted` | Not in this slice | They are slice 2's |
| P5 | The Omarchy look's range: "the cursor's row and cell keep their own look" | A range made there by Shift+click takes the selection's fill on every row, and the gutter's cursor stays on the cursor's row | The rest of that look's range is slice 2's |

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
- Tasks 2, 3 and 5 touch what `src/shots.rs` builds. After each, also:

```bash
~/.cargo/bin/cargo clippy --locked --features shots --lib --tests -- -D warnings
```

- One commit per task, signed. `error: 1Password: agent refused operation` means 1Password is locked: stage the task, say so, and wait. Never commit unsigned.
- Commit messages end with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- Tests use the Bookshop's `book_covers` only (`testing::book_covers_page`, `book_covers_structure`). No colour, size or picture of the design is checked.
- UI copy is short and plain, with no em dashes.

## Files

| File | What it gets |
|---|---|
| `src/model.rs` | `RowRange`, `Reach`, `ObjectTab::range`, `set_cursor`, `selected_span`, `selected_rows`. `Action::ExtendTo`, `ExtendBy`, `DeleteSelected` |
| `src/app.rs` | Every write of an object tab's `selection` goes through `set_cursor`. The reducers of the three actions |
| `src/app/editing.rs` | The same for its writes. `delete_selected`, `sweep_rows`, `drop_named` |
| `src/edit.rs` | `Sweep`, `MAX_ROWS_DELETED`, `Lock::of_table`, `Table::sweep`. `Told::MarkedRows`, `Told::RestoredRows` |
| `src/keymap.rs` | `Command::ExtendUp`, `Command::ExtendDown` |
| `src/ui/keys.rs` | Shift+arrows. Backspace asks for the selected rows |
| `src/ui/grid.rs` | `GridOutput::shift`, `Row::ranged` |
| `src/ui/data_view.rs` | The Shift+click, the range handed to the grid, the footer's count, the announcements |
| `src/ui/cell_editor.rs` | `sweep_said` |
| `src/ui/pending_bar.rs` | `marked_text` |
| `src/testing.rs` | `Harness::shift_click` |
| `src/ui/delete_many_tests.rs` (new) | The spec's tests for this slice |
| `src/ui/mod.rs`, `tests/keys.rs` | The new test file is named |
| `docs/_guide/editing-data.md`, `docs/_guide/macos.md`, `docs/_reference/keyboard-shortcuts.md` | Selecting and deleting several rows |

---

### Task 1: The range in the model

**Files:**
- Modify: `src/model.rs` (`ObjectTab` at 1961, its `impl` at 2083, tests at 3209)
- Modify: `src/app.rs:658, 776, 804, 828, 875, 1209, 2512, 3881`
- Modify: `src/app/editing.rs:779, 793, 830, 977, 1851`

- [ ] **Step 1: Write the failing tests**

In `src/model.rs`, in `mod tests`:

```rust
    /// The Bookshop's covers, `rows` of them, as a tab holds them.
    fn covers(rows: usize) -> ObjectTab {
        let object = ObjectRef::new("main", "book_covers");
        let mut tab = ObjectTab::new(TabId(1), object, ObjectKind::Table, true, 300, None);
        tab.rows.value = Some(crate::testing::book_covers_page(rows));
        tab
    }

    fn at(row: usize) -> Option<CellPos> {
        Some(CellPos { row, col: 1 })
    }

    #[test]
    fn an_extend_selects_the_rows_from_the_anchor_to_the_cursor() {
        let mut tab = covers(5);
        tab.set_cursor(at(1), Reach::End);
        assert_eq!(tab.selected_rows(), [1]);
        tab.set_cursor(at(3), Reach::Extend);
        assert_eq!(tab.selected_rows(), [1, 2, 3]);
        // The anchor stays where the range began.
        tab.set_cursor(at(0), Reach::Extend);
        assert_eq!(tab.selected_rows(), [0, 1]);
        // Back on the anchor one row is selected, and no range stands.
        tab.set_cursor(at(1), Reach::Extend);
        assert_eq!((tab.selected_rows(), tab.range), (vec![1], None));
    }

    #[test]
    fn with_no_cursor_an_extend_selects_one_row() {
        let mut tab = covers(5);
        tab.set_cursor(at(2), Reach::Extend);
        assert_eq!((tab.selected_rows(), tab.range), (vec![2], None));
    }

    #[test]
    fn a_move_ends_a_range_unless_it_is_held() {
        let mut tab = covers(5);
        tab.set_cursor(at(1), Reach::End);
        tab.set_cursor(at(2), Reach::Extend);
        tab.set_cursor(at(3), Reach::Move);
        assert_eq!((tab.selected_rows(), tab.range), (vec![3], None));
        // A held range follows the cursor, along the row too.
        tab.range = Some(RowRange {
            anchor: 3,
            held: true,
        });
        tab.set_cursor(at(1), Reach::Move);
        assert_eq!(tab.selected_rows(), [1, 2, 3]);
        tab.set_cursor(Some(CellPos { row: 1, col: 4 }), Reach::Move);
        assert_eq!(tab.selected_rows(), [1, 2, 3]);
        // On its anchor it is still a range: it is held.
        tab.set_cursor(at(3), Reach::Move);
        assert!(tab.range.is_some());
        // Anything else ends it, and so does a cursor that goes.
        tab.set_cursor(at(2), Reach::End);
        assert_eq!(tab.range, None);
        tab.set_cursor(at(1), Reach::Extend);
        tab.set_cursor(None, Reach::Keep);
        assert_eq!((tab.selected_rows(), tab.range), (Vec::new(), None));
    }

    #[test]
    fn a_range_counts_a_new_row_where_it_stands() {
        let mut tab = covers(3);
        let new = tab.edits.add_row(crate::edit::Place::Below(0), 3);
        tab.set_cursor(at(0), Reach::End);
        tab.set_cursor(at(1), Reach::Extend);
        assert_eq!(tab.selected_rows(), [0, crate::edit::new_row(new), 1]);
        assert_eq!(tab.selected_span(), Some((0, 2)));
    }
```

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked --lib model::tests::`
Expected: does not compile: `no method named set_cursor`, `cannot find type Reach`.

- [ ] **Step 3: Add the range**

In `src/model.rs`, above `/// One open table or view.`:

```rust
/// The rows selected with the cursor's: from `anchor` to the cursor's row,
/// in the order the grid shows them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RowRange {
    /// A row as [`CellPos::row`] names it: of the page, or a new row.
    pub anchor: usize,
    /// Moves keep the range until it is left: the terminal look's visual
    /// selection.
    pub held: bool,
}

/// What becomes of the range when the cursor is set
/// ([`ObjectTab::set_cursor`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reach {
    /// Shift with a click or an arrow: the range runs from its anchor, or
    /// from the row the cursor was on, to the new row.
    Extend,
    /// A move by key: a held range follows, another ends.
    Move,
    /// Anything else ends it: a click, an editor that opens, a new row, a
    /// page that goes.
    End,
    /// The rows under the cursor changed, not what is selected: a new row
    /// was dropped.
    Keep,
}
```

In `ObjectTab`, after `pub selection: Option<CellPos>,`:

```rust
    /// The rows selected with the cursor's, where there are several.
    /// Written with the cursor, by [`ObjectTab::set_cursor`].
    pub range: Option<RowRange>,
```

In `ObjectTab::new`, after `selection: None,`: `range: None,`.

In `impl ObjectTab`, after `focus_field`:

```rust
    /// Puts the cursor on `cell`, or takes it away. The one place the
    /// app's own code writes `selection`: what becomes of the range is
    /// said here, by `reach`.
    pub fn set_cursor(&mut self, cell: Option<CellPos>, reach: Reach) {
        let was = self.selection;
        self.selection = cell;
        let Some(cell) = cell else {
            self.range = None;
            return;
        };
        let range = match reach {
            Reach::Extend => self.range.or_else(|| {
                was.map(|was| RowRange {
                    anchor: was.row,
                    held: false,
                })
            }),
            Reach::Move => self.range.filter(|range| range.held),
            Reach::End => None,
            Reach::Keep => self.range,
        };
        // One row that is not held is no range.
        self.range = range.filter(|range| range.held || range.anchor != cell.row);
    }

    /// The places of the grid the range covers, the first and the last,
    /// where a range stands.
    pub fn selected_span(&self) -> Option<(usize, usize)> {
        let (cell, range) = (self.selection?, self.range?);
        let height = self.page().map_or(0, |page| page.rows.len());
        let order = crate::edit::Order::of(&self.edits.added, height);
        let (from, to) = (order.place(range.anchor)?, order.place(cell.row)?);
        Some((from.min(to), from.max(to)))
    }

    /// The selected rows, in the grid's order: the cursor's, and with a
    /// range every row from its anchor to the cursor's.
    pub fn selected_rows(&self) -> Vec<usize> {
        let Some(cell) = self.selection else {
            return Vec::new();
        };
        let Some((first, last)) = self.selected_span() else {
            return vec![cell.row];
        };
        let height = self.page().map_or(0, |page| page.rows.len());
        let order = crate::edit::Order::of(&self.edits.added, height);
        (first..=last).filter_map(|place| order.row(place)).collect()
    }
```

In `drop_page` and `off_new_rows`, replace `self.selection = None;` with `self.set_cursor(None, Reach::End);`. `off_new_rows` also ends a range whose anchor is a new row, with the cursor on a row of the page: the set that held the anchor was dropped. At its end:

```rust
        if self
            .range
            .is_some_and(|range| crate::edit::new_id(range.anchor).is_some())
        {
            self.range = None;
        }
```

and a test for it, with the four of Step 1:

```rust
    #[test]
    fn a_range_anchored_on_a_new_row_ends_when_the_new_rows_go() {
        let mut tab = covers(3);
        let new = crate::edit::new_row(tab.edits.add_row(crate::edit::Place::Below(0), 3));
        tab.set_cursor(at(new), Reach::End);
        tab.set_cursor(at(2), Reach::Extend);
        assert_eq!(tab.selected_rows(), [new, 1, 2]);
        tab.edits = crate::edit::Edits::default();
        tab.off_new_rows();
        assert_eq!((tab.selected_rows(), tab.range), (vec![2], None));
    }
```

- [ ] **Step 4: Send every write through it**

Each line below replaces a write of `object.selection`. Add `Reach` to the `use crate::model::{…}` of `src/app.rs` and `src/app/editing.rs`.

| Where | Was | Becomes |
|---|---|---|
| `src/app.rs:658, 776, 804, 828, 3881` | `object.selection = None;` | `object.set_cursor(None, Reach::End);` |
| `src/app.rs:875` (`SelectCell`) | `object.selection = Some(cell);` | `object.set_cursor(Some(cell), Reach::End);` |
| `src/app.rs:1209` (`MoveSelection`) | `object.selection = moved;` | `object.set_cursor(moved, Reach::Move);` |
| `src/app.rs:2512` (the page read again) | `object.selection = match object.selection { … };` | `let clamped = match object.selection { … };` then `object.set_cursor(clamped, Reach::End);` |
| `src/app/editing.rs:779, 793` (`edit_cell`) | `object.selection = Some(cell);` | `object.set_cursor(Some(cell), Reach::End);` |
| `src/app/editing.rs:830` (`settle_new_row`) | `object.selection = Some(cell);` | `object.set_cursor(Some(cell), Reach::End);` |
| `src/app/editing.rs:1851` (a save's rows) | `object.selection = object.selection.and_then(\|cell\| { … });` | `let moved = object.selection.and_then(\|cell\| { … });` then `object.set_cursor(moved, Reach::End);` |

`drop_row` (`src/app/editing.rs:977`) keeps the range, unless its anchor was the row that went:

```rust
        let stays = stood
            .and_then(|place| order.row(place.min(last)))
            .map(|row| CellPos { row, col: cell.col });
        if object.range.is_some_and(|range| range.anchor == cell.row) {
            object.range = None;
        }
        object.set_cursor(stays, Reach::Keep);
```

(`order` borrows `object.edits.added`: take `stays` first, as above, so the borrow has ended.)

Then check that no other write is left:

Run: `grep -n 'object\.selection = \|self\.selection = ' src/app.rs src/app/editing.rs src/model.rs`
Expected: only the line inside `set_cursor`, lines of `SqlTab` (`sql.selection`, `self.selection` at `model.rs` 2600 and later), and lines under `#[cfg(test)]`.

- [ ] **Step 5: Run the tests**

Run: `~/.cargo/bin/cargo test --locked --lib model::tests::`, then the four checks.
Expected: the five new tests pass, and nothing else changed: no range is ever made yet.

- [ ] **Step 6: Commit**

```bash
git add src/model.rs src/app.rs src/app/editing.rs
git commit -m "Give a table's selection a range of rows

The cursor is set in one place, which says what becomes of the range.
Nothing makes a range yet.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: Shift+click and Shift+arrows

**Files:**
- Modify: `src/model.rs` (`Action`, after `SelectCell` at 238)
- Modify: `src/app.rs` (the reducer, after `MoveSelection` at 1168), `src/app/editing.rs:36` (`dropped_under_a_prompt`)
- Modify: `src/keymap.rs` (the enum at 345, `info` at 549, the lines at 988, tests)
- Modify: `src/ui/keys.rs:734`, `src/ui/grid.rs:158, 785`, `src/ui/data_view.rs:1694`
- Modify: `src/testing.rs` (after `click` at 234)
- Create: `src/ui/delete_many_tests.rs`
- Modify: `src/ui/mod.rs:12`, `tests/keys.rs:22`

- [ ] **Step 1: Name the test file**

`src/ui/mod.rs`, above `#[cfg(test)] mod delete_tests;`:

```rust
#[cfg(test)]
mod delete_many_tests;
```

`tests/keys.rs`: `TESTS` becomes `[&str; 9]`, with `"src/ui/delete_many_tests.rs",` before `"src/ui/delete_tests.rs",`.

- [ ] **Step 2: Write the harness's Shift+click**

`src/testing.rs`, after `Harness::click`:

```rust
    /// Clicks the row labelled `label` with Shift held, by the pointer: a
    /// click through AccessKit carries no modifier. The frame is told of
    /// Shift as a window tells it, before the pointer moves.
    pub fn shift_click(&mut self, label: &str) {
        let tree = self.settle();
        let row = bounds(&tree, label, Role::Button)
            .unwrap_or_else(|| panic!("no row labelled {label:?}: {:?}", labels(&tree)));
        let pos = row.left_center() + egui::vec2(40.0, 0.0);
        let shift = egui::Modifiers::SHIFT;
        let press = |pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: shift,
        };
        self.frame(vec![
            egui::Event::ModifiersChanged(shift),
            egui::Event::PointerMoved(pos),
        ]);
        self.frame(vec![press(true)]);
        self.frame(vec![press(false)]);
        self.frame(vec![egui::Event::ModifiersChanged(egui::Modifiers::NONE)]);
        self.settle();
    }
```

- [ ] **Step 3: Write the failing tests**

Create `src/ui/delete_many_tests.rs`:

```rust
//! Deleting many rows at once, checked against its spec
//! (`docs/superpowers/specs/2026-10-13-deleting-many-rows-design.md`:
//! "spec N" below is one of its sections, "test N" an entry of its Tests).
//!
//! Only behaviour, names and words are checked here, on the Bookshop's
//! `book_covers`. No colour, size or picture of the design is.

use egui::{Key, Modifiers};

use super::insert_audit_tests::{add, select, selected};
use crate::model::{Action, CellPos, ConnTabId, EditStart, Pane, TabId};
use crate::testing::{Harness, book_covers_page, book_covers_structure};
use crate::theme::Look;

/// The desktop looks: the keys and the buttons are the same in both.
fn desktop() -> [Look; 2] {
    [Look::standard(), Look::macos()]
}

/// The Bookshop's covers holding `page`, writable, the grid holding the
/// keyboard.
fn covers_with(look: Look, page: tabletist_db::RowPage) -> (Harness, ConnTabId, TabId) {
    let mut harness = Harness::new();
    harness.set_look(look);
    let tab = harness.connect_fake_as(false);
    harness.app.apply(Action::OpenObject {
        tab,
        object: tabletist_db::ObjectRef::new("main", "book_covers"),
        kind: tabletist_db::ObjectKind::Table,
        pin: true,
    });
    harness.answer_structure(book_covers_structure());
    harness.answer_rows(page);
    let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
    harness.app.workspace_mut(tab).unwrap().pane = Pane::Grid;
    harness.settle();
    (harness, tab, id)
}

/// The selected rows, in the grid's order.
fn rows(harness: &Harness, tab: ConnTabId, id: TabId) -> Vec<usize> {
    let workspace = harness.app.workspace(tab).unwrap();
    workspace.object_tab(id).unwrap().selected_rows()
}

/// Selects the rows from `from` to `to`, as Shift with the arrows does.
fn select_rows(harness: &mut Harness, tab: ConnTabId, id: TabId, from: usize, to: usize) {
    select(harness, tab, id, (from, 1));
    let rows = to as isize - from as isize;
    harness.app.apply(Action::ExtendBy { tab, id, rows });
    harness.settle();
}

/// Spec 1 and test 1: Shift with an arrow and with a click selects a run
/// of rows, the footer counts them, and a plain click or arrow selects
/// one again.
#[test]
fn shift_with_an_arrow_or_a_click_selects_a_run_of_rows() {
    for look in desktop() {
        let (mut harness, tab, id) = covers_with(look, book_covers_page(5));
        select(&mut harness, tab, id, (1, 2));
        harness.press(Key::ArrowDown, Modifiers::SHIFT);
        harness.press(Key::ArrowDown, Modifiers::SHIFT);
        assert_eq!(rows(&harness, tab, id), [1, 2, 3]);
        assert_eq!(selected(&harness, tab, id), Some((3, 2)));
        // Back over the anchor, to the row above it.
        for _ in 0..3 {
            harness.press(Key::ArrowUp, Modifiers::SHIFT);
        }
        assert_eq!(rows(&harness, tab, id), [0, 1]);
        // A plain arrow selects one row, down or along the row.
        harness.press(Key::ArrowDown, Modifiers::NONE);
        assert_eq!(rows(&harness, tab, id), [1]);
        harness.press(Key::ArrowDown, Modifiers::SHIFT);
        harness.press(Key::ArrowRight, Modifiers::NONE);
        assert_eq!(rows(&harness, tab, id), [2]);
        // Shift with a click runs from the cursor's row to the clicked one.
        harness.shift_click("Row 5");
        assert_eq!(rows(&harness, tab, id), [2, 3, 4]);
        // And a plain click selects one.
        select(&mut harness, tab, id, (0, 1));
        assert_eq!(rows(&harness, tab, id), [0]);
    }
}

/// Test 1: with no cursor yet, Shift with a click is a plain click.
#[test]
fn shift_with_a_click_and_no_cursor_selects_one_row() {
    let (mut harness, tab, id) = covers_with(Look::macos(), book_covers_page(5));
    harness.shift_click("Row 3");
    assert_eq!(rows(&harness, tab, id), [2]);
}

/// Spec 1 and test 10: a range does not outlive its page, and ends where
/// something works on the cursor's row alone.
#[test]
fn a_range_ends_with_its_page_and_with_what_works_on_one_row() {
    let ranged = || {
        let (mut harness, tab, id) = covers_with(Look::macos(), book_covers_page(5));
        select_rows(&mut harness, tab, id, 1, 3);
        assert_eq!(rows(&harness, tab, id), [1, 2, 3]);
        (harness, tab, id)
    };
    let range = |harness: &Harness, tab: ConnTabId, id: TabId| {
        let workspace = harness.app.workspace(tab).unwrap();
        workspace.object_tab(id).unwrap().range
    };
    // A reload.
    let (mut harness, tab, id) = ranged();
    harness.app.apply(Action::Refresh(tab));
    harness.answer_rows(book_covers_page(5));
    assert_eq!(range(&harness, tab, id), None);
    // A sort.
    let (mut harness, tab, id) = ranged();
    harness.app.apply(Action::SortBy {
        tab,
        object_tab: id,
        column: "kind".into(),
    });
    assert_eq!(range(&harness, tab, id), None);
    // A new row.
    let (mut harness, tab, id) = ranged();
    add(&mut harness, tab, id);
    assert_eq!(range(&harness, tab, id), None);
    // An editor that opens.
    let (mut harness, tab, id) = ranged();
    harness.app.apply(Action::EditCell {
        tab,
        id,
        cell: CellPos { row: 3, col: 2 },
        start: EditStart::Value,
    });
    assert_eq!(range(&harness, tab, id), None);
    // A save that takes a row out: the first row is marked, then the range
    // is made again.
    let (mut harness, tab, id) = ranged();
    select(&mut harness, tab, id, (0, 1));
    harness.app.apply(Action::DeleteRow { tab, id });
    select_rows(&mut harness, tab, id, 1, 3);
    harness.app.apply(Action::WriteEdits { tab, id });
    harness.answer_written(Ok(tabletist_db::WriteOutcome::Written {
        inserted: Vec::new(),
        rows: Vec::new(),
        gone: Vec::new(),
        elapsed: std::time::Duration::from_millis(14),
    }));
    assert_eq!(range(&harness, tab, id), None);
}
```

`covers_as`, `edits`, `painted` and `set` of `insert_audit_tests` join the `use` in the tasks whose tests need them, and `marked` is written in Task 5, so each commit compiles without a warning.

- [ ] **Step 4: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked --lib delete_many_tests`
Expected: does not compile: `no variant named ExtendBy`.

- [ ] **Step 5: The actions and their reducers**

`src/model.rs`, after `SelectCell { … },`:

```rust
    /// Select the rows from the cursor's row, or from the range's anchor,
    /// to `cell`'s row, and put the cursor on `cell`: Shift with a click.
    /// With no cursor it selects the cell.
    ExtendTo {
        tab: ConnTabId,
        id: TabId,
        cell: CellPos,
    },
    /// The same by rows from the cursor: Shift with an arrow.
    ExtendBy {
        tab: ConnTabId,
        id: TabId,
        rows: isize,
    },
```

`src/app/editing.rs`, in `dropped_under_a_prompt`, after `| Action::SelectCell { .. }`: `| Action::ExtendTo { .. }` and `| Action::ExtendBy { .. }`.

`src/app.rs`. First lift the move's arithmetic out of `MoveSelection`, as a function beside `step` (4199):

```rust
/// Where a move by `rows` and `cols` puts the cursor of `object`, through
/// the rows as the grid shows them: a new row stands among the page's, and
/// is no row of it. With no cursor, the first cell. `None` where the grid
/// has no cell.
fn moved(object: &ObjectTab, rows: isize, cols: isize) -> Option<CellPos> {
    let (height, width) = object
        .page()
        .map(|page| (page.rows.len(), page.columns.len()))
        .unwrap_or((0, 0));
    let order = crate::edit::Order::of(&object.edits.added, height);
    match object.selection {
        _ if order.is_empty() || width == 0 => None,
        None => order.row(0).map(|row| CellPos { row, col: 0 }),
        Some(cell) => {
            let from = order.place(cell.row).unwrap_or(0);
            let to = step(from, rows, order.len());
            order.row(to).map(|row| CellPos {
                row,
                col: step(cell.col, cols, width),
            })
        }
    }
}
```

The object branch of `MoveSelection` becomes:

```rust
                } else if let Some(object) = self.object_tab_mut(tab, id) {
                    let moved = moved(object, rows, cols);
                    object.pinned |= moved.is_some();
                    object.set_cursor(moved, Reach::Move);
                }
```

After the `MoveSelection` arm:

```rust
            Action::ExtendTo { tab, id, cell } => {
                self.close_editor(tab, id, true);
                if let Some(object) = self.object_tab_mut(tab, id) {
                    object.edits.why = None;
                    object.focus_field = None;
                    object.set_cursor(Some(cell), Reach::Extend);
                    object.pinned = true;
                }
                if let Some(workspace) = self.workspace_mut(tab) {
                    workspace.pane = Pane::Grid;
                }
            }
            Action::ExtendBy { tab, id, rows } => {
                self.close_editor(tab, id, true);
                if let Some(object) = self.object_tab_mut(tab, id) {
                    object.edits.why = None;
                    object.focus_field = None;
                    let moved = moved(object, rows, 0);
                    object.pinned |= moved.is_some();
                    object.set_cursor(moved, Reach::Extend);
                }
            }
```

- [ ] **Step 6: The keys**

`src/keymap.rs`. In the command enum, after `MovePage,`: `ExtendUp,` and `ExtendDown,`. In `info`, after the `C::MovePage` arm:

```rust
            C::ExtendUp => info("Extend the selection up", "", Editing),
            C::ExtendDown => info("Extend the selection down", "", Editing),
```

In the lines, after the `C::MovePage` binding:

```rust
        // A table's rows only: a result has one selected row.
        bind(C::ExtendUp, GRID, keys(&[k("shift+up")]), NONE),
        bind(C::ExtendDown, GRID, keys(&[k("shift+down")]), NONE),
```

In the keymap's test `the_settled_keys_are_the_ones_bound`, at its end:

```rust
        assert_eq!(mac(Command::ExtendUp), "⇧↑");
        assert_eq!(mac(Command::ExtendDown), "⇧↓");
        assert_eq!(omarchy(Command::ExtendDown), "");
```

`src/ui/keys.rs`, inside `if !editing && any_grid && !tree_arrows && !focused {` (734), before `let page = 20;`:

```rust
                // Shift with an arrow takes the row it moves to into the
                // selection. A table's rows only.
                if sql.is_none() {
                    for (command, rows) in [(Command::ExtendUp, -1), (Command::ExtendDown, 1)] {
                        let presses = on(input, command, Scope::Grid).count;
                        actions.extend((0..presses).map(|_| Action::ExtendBy { tab, id, rows }));
                    }
                }
```

- [ ] **Step 7: The click**

`src/ui/grid.rs`. In `GridOutput`, after `clicked`:

```rust
    /// Shift was held at the click: it extends the selection to its row.
    pub shift: bool,
```

Where the click is reported (785):

```rust
                    if response.clicked() {
                        output.clicked = cell;
                        output.shift = ui.input(|input| input.modifiers.shift);
                    }
```

`src/ui/data_view.rs` (1694):

```rust
        if let Some(cell) = output.clicked.and_then(held) {
            actions.push(if output.shift {
                Action::ExtendTo { tab, id, cell }
            } else {
                Action::SelectCell { tab, id, cell }
            });
        }
```

A SQL result's grid (`src/ui/sql_results.rs`) ignores `shift`: nothing changes there.

- [ ] **Step 8: Run the tests**

Run: `~/.cargo/bin/cargo test --locked --lib delete_many_tests`, then the four checks and the shots' clippy.
Expected: the three tests pass. If `shift_with_an_arrow_or_a_click…` fails only at `shift_click`, print `harness.ctx.input(|input| input.modifiers)` in the frame of the press: `ModifiersChanged` is how `Harness::copy` sets a modifier, and the helper must end with the grid seeing Shift at `response.clicked()`.

- [ ] **Step 9: Commit**

```bash
git add src/model.rs src/app.rs src/app/editing.rs src/keymap.rs src/ui/keys.rs src/ui/grid.rs src/ui/data_view.rs src/ui/mod.rs src/ui/delete_many_tests.rs src/testing.rs tests/keys.rs
git commit -m "Select a run of rows with Shift and a click or an arrow

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: Show the range

**Files:**
- Modify: `src/ui/grid.rs:69` (`Row`), `760-892` (the row), its tests' `Row` literals at 1954, 2422, 2581
- Modify: `src/ui/data_view.rs:58` (`selection_note`), `1549-1583` (the grid's rows)
- Modify: `src/ui/sql_results.rs:1695`
- Test: `src/ui/delete_many_tests.rs`

- [ ] **Step 1: Write the failing test**

Add `painted` to the test file's `use super::insert_audit_tests::{…}`, and:

```rust
/// Spec 1 and tests 1 and 11: the footer counts the selected rows, and
/// each row of a range is told to a screen reader as selected.
#[test]
fn a_range_is_counted_in_the_footer_and_told_as_selected() {
    for look in desktop() {
        let (mut harness, tab, id) = covers_with(look, book_covers_page(5));
        select(&mut harness, tab, id, (1, 2));
        assert!(painted(&harness, "1 row selected"));
        select_rows(&mut harness, tab, id, 1, 3);
        assert!(painted(&harness, "3 rows selected"));
        let tree = harness.settle();
        let told = |label: &str| {
            let mut nodes = tree.nodes.iter();
            let node = nodes.find(|(_, node)| node.label() == Some(label));
            node.and_then(|(_, node)| node.is_selected())
        };
        assert_eq!(told("Row 2"), Some(true));
        assert_eq!(told("Row 4"), Some(true));
        assert_eq!(told("Row 1"), None);
        assert_eq!(told("Row 5"), None);
    }
}
```

- [ ] **Step 2: Run it to see it fail**

Run: `~/.cargo/bin/cargo test --locked --lib delete_many_tests::a_range_is_counted`
Expected: FAIL at `3 rows selected`.

- [ ] **Step 3: The footer**

`src/ui/data_view.rs`, in `selection_note`, after `let cell = object.selection?;`:

```rust
    // Several rows: how many, whatever kind each is.
    let rows = object.selected_rows().len();
    if rows > 1 {
        return Some(format!("{rows} {}", gettext(locale, "rows selected")));
    }
```

- [ ] **Step 4: The rows**

`src/ui/grid.rs`, in `Row`, after `deleted`:

```rust
    /// The row is one of several selected rows: it is drawn as the
    /// selected row of its kind is, and told as selected.
    pub ranged: bool,
```

Add `ranged: false,` to the `Row` literals of the grid's tests (1954, 2422, 2581) and of `src/ui/sql_results.rs:1695`.

In the row (760), take `ranged` out of `rows(row)` with the others, and after `response.widget_info(…)`:

```rust
                if ranged {
                    ui.ctx().accesskit_node_builder(response.id, |node| {
                        node.set_selected(true);
                    });
                }
```

Where `selected_row` is made (805):

```rust
                // The cursor's row, and with it the rows selected with it.
                let cursor_row = selection.is_some_and(|cell| cell.row == row);
                let selected_row = cursor_row || ranged;
```

The terminal's gutter cursor (881) is the cursor's alone: `if selected_row {` there becomes `if cursor_row {`. Every other use of `selected_row` stays: the fill, the tint while the grid has the keyboard, and the accent bar are a selected row's.

`src/ui/data_view.rs`, right after `let order = crate::edit::Order::of(…);` (1502), above `// Why the cell last asked for cannot be edited`. Not lower: from `let mut editor = object.edits.editor.as_mut()` (1525) on, the tab is borrowed for its editor's text until the grid is drawn, and a method of the whole tab cannot be called.

```rust
        // The rows selected with the cursor's, as places of the grid.
        let span = object.selected_span();
        let ranged =
            |place: usize| span.is_some_and(|(first, last)| (first..=last).contains(&place));
```

and in both `grid::Row { … }` literals of the `&|place| …` closure: `ranged: ranged(place),`.

- [ ] **Step 5: Run the tests**

Run: `~/.cargo/bin/cargo test --locked --lib delete_many_tests`, then the four checks and the shots' clippy.
Expected: PASS. `a_selected_new_row_keeps_its_fill…` and the grid's other tests still pass: with no range, `selected_row` is what it was.

- [ ] **Step 6: Look at it**

Add a throwaway scene to `src/shots.rs` (not committed): the covers with five rows, rows 2 to 4 selected, row 3 marked for deletion, in the macOS look light and dark. Render it (`~/.cargo/bin/cargo test --locked --features shots --lib shots -- --ignored`), read the picture, and check that the three rows read as selected, the marked one stays struck, and one cell is the active cell. Remove the scene. The picture stays local.

- [ ] **Step 7: Commit**

```bash
git add src/ui/grid.rs src/ui/data_view.rs src/ui/sql_results.rs src/ui/delete_many_tests.rs
git commit -m "Draw and count the rows of a selection

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: What a delete of several rows comes to

**Files:**
- Modify: `src/edit.rs` (`Lock` at 24, `impl Table` after `no_delete` at 293, tests at 2762)

- [ ] **Step 1: Write the failing tests**

In `src/edit.rs`, in `mod tests`, after `marked_covers` (2872), which makes the Bookshop's covers as editing sees them from a structure, a page and a set:

```rust
    /// What deleting `rows` of `page` comes to, with `edits` pending.
    fn swept(page: &RowPage, edits: &Edits, rows: &[usize]) -> Sweep {
        let structure = crate::testing::book_covers_structure();
        marked_covers(&structure, page, edits).sweep(rows)
    }

    fn mark_rows(rows: Vec<usize>, new: Vec<usize>, skipped: usize) -> Sweep {
        Sweep::Mark { rows, new, skipped }
    }

    #[test]
    fn a_sweep_marks_what_is_unmarked_and_restores_only_when_all_are_marked() {
        let page = crate::testing::book_covers_page(5);
        let mut edits = Edits::default();
        assert_eq!(
            swept(&page, &edits, &[1, 2, 3]),
            mark_rows(vec![1, 2, 3], Vec::new(), 0)
        );
        // One of them marked already: the others are marked, none restored.
        edits.mark(2);
        assert_eq!(
            swept(&page, &edits, &[1, 2, 3]),
            mark_rows(vec![1, 3], Vec::new(), 0)
        );
        // All marked: they are restored.
        edits.mark(1);
        edits.mark(3);
        let rows = vec![1, 2, 3];
        assert_eq!(swept(&page, &edits, &[1, 2, 3]), Sweep::Restore { rows });
    }

    #[test]
    fn a_sweep_drops_new_rows_with_a_mark_and_leaves_them_at_a_restore() {
        let page = crate::testing::book_covers_page(5);
        let mut edits = Edits::default();
        let new = new_row(edits.add_row(Place::Below(1), 5));
        assert_eq!(
            swept(&page, &edits, &[1, new, 2]),
            mark_rows(vec![1, 2], vec![new], 0)
        );
        assert_eq!(
            swept(&page, &edits, &[new]),
            Sweep::Drop { new: vec![new] }
        );
        edits.mark(1);
        edits.mark(2);
        let rows = vec![1, 2];
        assert_eq!(swept(&page, &edits, &[1, new, 2]), Sweep::Restore { rows });
    }

    #[test]
    fn a_sweep_leaves_out_a_row_that_cannot_be_deleted_and_counts_it() {
        let mut page = crate::testing::book_covers_page(5);
        page.rows[2][0] = Value::Null;
        let edits = Edits::default();
        assert_eq!(
            swept(&page, &edits, &[1, 2, 3]),
            mark_rows(vec![1, 3], Vec::new(), 1)
        );
        // Alone, its own reason is the answer.
        assert_eq!(
            swept(&page, &edits, &[2]),
            Sweep::Refused(Lock::KeyIsNull)
        );
    }

    #[test]
    fn a_sweep_is_refused_for_the_table_and_over_the_cap() {
        let structure = crate::testing::book_covers_structure();
        let page = crate::testing::book_covers_page(MAX_ROWS_DELETED + 1);
        let edits = Edits::default();
        let all: Vec<usize> = (0..=MAX_ROWS_DELETED).collect();
        assert_eq!(swept(&page, &edits, &all), Sweep::TooMany);
        assert!(matches!(
            swept(&page, &edits, &all[..MAX_ROWS_DELETED]),
            Sweep::Mark { rows, .. } if rows.len() == MAX_ROWS_DELETED
        ));
        let mut read_only = marked_covers(&structure, &page, &edits);
        read_only.access = Access::ReadOnly;
        assert_eq!(read_only.sweep(&[1, 2]), Sweep::Refused(Lock::ReadOnly));
        let mut saving = marked_covers(&structure, &page, &edits);
        saving.saving = true;
        assert_eq!(saving.sweep(&[1, 2]), Sweep::Refused(Lock::Saving));
    }
```

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked --lib edit::tests::a_sweep`
Expected: does not compile: `cannot find type Sweep`.

- [ ] **Step 3: Write `sweep`**

`src/edit.rs`, after `enum Lock`:

```rust
impl Lock {
    /// Whether the reason holds for every row of the table, and not for
    /// one row by what it holds.
    pub fn of_table(self) -> bool {
        matches!(
            self,
            Self::ReadOnly
                | Self::NotATable
                | Self::StructureLoading
                | Self::NoKey
                | Self::KeyType
                | Self::Saving
                | Self::Refreshing
        )
    }
}

/// How many rows one action marks for deletion, at most.
pub const MAX_ROWS_DELETED: usize = 1_000;

/// What a delete of several rows comes to ([`Table::sweep`]). Rows are as
/// [`CellPos::row`] names them, in the order they were given.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Sweep {
    /// More rows than one action takes.
    TooMany,
    /// None is deleted: no row of the table can be now, or none of these.
    Refused(Lock),
    /// The page's rows to mark and the new rows to drop. `skipped` rows
    /// cannot be deleted on their own account.
    Mark {
        rows: Vec<usize>,
        new: Vec<usize>,
        skipped: usize,
    },
    /// Every row that can be deleted is marked already: they are restored.
    Restore { rows: Vec<usize> },
    /// Only new rows: they are dropped.
    Drop { new: Vec<usize> },
}
```

In `impl Table`, after `no_delete`:

```rust
    /// What deleting `rows` in one action comes to: the selected rows, as
    /// the grid orders them. A press marks when any of them is unmarked,
    /// and restores only when all are marked.
    pub fn sweep(&self, rows: &[usize]) -> Sweep {
        if rows.len() > MAX_ROWS_DELETED {
            return Sweep::TooMany;
        }
        // Under a save nothing is marked, restored or dropped: its answer
        // names the rows it sent.
        if self.saving {
            return Sweep::Refused(Lock::Saving);
        }
        let (new, saved): (Vec<usize>, Vec<usize>) =
            rows.iter().partition(|row| new_id(**row).is_some());
        let mut free = Vec::with_capacity(saved.len());
        let mut skipped = 0;
        let mut why = None;
        for row in saved {
            match self.no_delete(row) {
                None => free.push(row),
                Some(lock) if lock.of_table() => return Sweep::Refused(lock),
                Some(lock) => {
                    skipped += 1;
                    why.get_or_insert(lock);
                }
            }
        }
        let unmarked: Vec<usize> = free
            .iter()
            .copied()
            .filter(|row| !self.deleted.contains_key(row))
            .collect();
        if !unmarked.is_empty() {
            return Sweep::Mark {
                rows: unmarked,
                new,
                skipped,
            };
        }
        if !free.is_empty() {
            return Sweep::Restore { rows: free };
        }
        if !new.is_empty() {
            return Sweep::Drop { new };
        }
        Sweep::Refused(why.unwrap_or(Lock::NoSuchCell))
    }
```

- [ ] **Step 4: Run the tests**

Run: `~/.cargo/bin/cargo test --locked --lib edit::tests::a_sweep`, then the four checks.
Expected: PASS. Nothing calls `sweep` yet outside its tests; if clippy calls it dead code, the item is `pub` in a library crate and it will not.

- [ ] **Step 5: Commit**

```bash
git add src/edit.rs
git commit -m "Decide in one place what a delete of several rows comes to

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 5: Mark, restore and drop several rows

**Files:**
- Modify: `src/edit.rs:1457` (`Told`)
- Modify: `src/model.rs:415` (`Action`), `src/app.rs:1072`, `src/app/editing.rs:36, 986`
- Modify: `src/ui/keys.rs:1288, 1382`, `src/ui/cell_editor.rs:1511`, `src/ui/pending_bar.rs:45`, `src/ui/data_view.rs:1380`
- Test: `src/ui/delete_many_tests.rs`

- [ ] **Step 1: Write the failing tests**

Add `covers_as`, `edits` and `set` to the test file's `use super::insert_audit_tests::{…}`, add `use super::duplicate_tests::button;`, and:

```rust
/// The rows marked for deletion.
fn marked(harness: &Harness, tab: ConnTabId, id: TabId) -> Vec<usize> {
    edits(harness, tab, id).deleted.keys().copied().collect()
}

/// What the toast says, where one shows.
fn said(harness: &Harness) -> Option<String> {
    crate::ui::toast::said(&harness.ctx)
}

/// Whether `words` are said to a screen reader, as a polite live region.
fn polite(harness: &mut Harness, words: &str) -> bool {
    let tree = harness.settle();
    let mut nodes = tree.nodes.iter();
    let node = nodes.find(|(_, node)| node.label().or_else(|| node.value()) == Some(words));
    node.map(|(_, node)| node.live()) == Some(Some(egui::accesskit::Live::Polite))
}

/// Spec 2 and test 2: Backspace on several rows marks them all and keeps
/// the range, and a second one restores them. With one of them unmarked,
/// that one is marked and none is restored.
#[test]
fn backspace_marks_the_selected_rows_and_again_restores_them() {
    for look in desktop() {
        let (mut harness, tab, id) = covers_with(look, book_covers_page(5));
        select_rows(&mut harness, tab, id, 1, 3);
        harness.press(Key::Backspace, Modifiers::NONE);
        assert_eq!(marked(&harness, tab, id), [1, 2, 3]);
        assert_eq!(rows(&harness, tab, id), [1, 2, 3]);
        assert_eq!(selected(&harness, tab, id), Some((3, 1)));
        assert_eq!(said(&harness), None);
        harness.press(Key::Backspace, Modifiers::NONE);
        assert!(marked(&harness, tab, id).is_empty());
        // One row of the range marked alone, then the range.
        select(&mut harness, tab, id, (2, 1));
        harness.press(Key::Backspace, Modifiers::NONE);
        select_rows(&mut harness, tab, id, 1, 3);
        harness.press(Key::Backspace, Modifiers::NONE);
        assert_eq!(marked(&harness, tab, id), [1, 2, 3]);
    }
}

/// Spec 2 and test 6: one action takes 1,000 rows and no more.
#[test]
fn more_than_a_thousand_rows_are_refused_with_the_message() {
    let (mut harness, tab, id) = covers_with(Look::macos(), book_covers_page(1_001));
    select_rows(&mut harness, tab, id, 0, 1_000);
    harness.press(Key::Backspace, Modifiers::NONE);
    assert!(marked(&harness, tab, id).is_empty());
    assert_eq!(
        said(&harness).as_deref(),
        Some("Select fewer rows, or delete in the SQL editor.")
    );
    select_rows(&mut harness, tab, id, 0, 999);
    harness.press(Key::Backspace, Modifiers::NONE);
    assert_eq!(marked(&harness, tab, id).len(), 1_000);
}

/// Spec 2 and test 7: new rows of a range are dropped with a mark and left
/// at a restore, a range of new rows alone drops them, and the range that
/// is left stays selected.
#[test]
fn new_rows_of_a_range_are_dropped_with_the_mark() {
    let (mut harness, tab, id) = covers_with(Look::macos(), book_covers_page(3));
    // A new row under row 0, and the range from it down to row 2.
    harness.app.apply(Action::AddRow {
        tab,
        id,
        place: crate::edit::Place::Below(0),
    });
    harness.app.apply(Action::CancelEdit { tab, id });
    let new = crate::edit::new_row(edits(&harness, tab, id).added[0].id);
    select(&mut harness, tab, id, (new, 1));
    harness.app.apply(Action::ExtendBy { tab, id, rows: 2 });
    assert_eq!(rows(&harness, tab, id), [new, 1, 2]);
    harness.press(Key::Backspace, Modifiers::NONE);
    assert!(edits(&harness, tab, id).added.is_empty());
    assert_eq!(marked(&harness, tab, id), [1, 2]);
    // The anchor was the row that went: the rows left are still selected.
    assert_eq!(rows(&harness, tab, id), [1, 2]);
    assert!(polite(
        &mut harness,
        "2 rows marked for deletion. 2 to delete. 1 new row dropped."
    ));
    // A restore leaves a new row of the range alone: a new row under row
    // 0 again, and with it rows 1 and 2, which are both marked.
    harness.app.apply(Action::AddRow {
        tab,
        id,
        place: crate::edit::Place::Below(0),
    });
    harness.app.apply(Action::CancelEdit { tab, id });
    let new = crate::edit::new_row(edits(&harness, tab, id).added[0].id);
    select(&mut harness, tab, id, (new, 1));
    harness.app.apply(Action::ExtendBy { tab, id, rows: 2 });
    assert_eq!(rows(&harness, tab, id), [new, 1, 2]);
    harness.press(Key::Backspace, Modifiers::NONE);
    assert_eq!(edits(&harness, tab, id).added.len(), 1);
    assert!(marked(&harness, tab, id).is_empty());
}

/// Test 7: a range of new rows alone drops them and says so.
#[test]
fn a_range_of_new_rows_alone_drops_them() {
    let (mut harness, tab, id) = covers_with(Look::macos(), book_covers_page(3));
    add(&mut harness, tab, id);
    add(&mut harness, tab, id);
    let first = crate::edit::new_row(edits(&harness, tab, id).added[0].id);
    select(&mut harness, tab, id, (first, 1));
    harness.app.apply(Action::ExtendBy { tab, id, rows: 1 });
    assert_eq!(rows(&harness, tab, id).len(), 2);
    harness.press(Key::Backspace, Modifiers::NONE);
    assert!(edits(&harness, tab, id).added.is_empty());
    assert!(marked(&harness, tab, id).is_empty());
    assert!(polite(&mut harness, "2 new rows dropped."));
    // The cursor was on the second of them, the grid's second row: it is
    // on the row that stands there now, as after one new row is dropped.
    assert_eq!(selected(&harness, tab, id), Some((1, 1)));
}

/// Spec 2 and test 8: a row that cannot be deleted is left out and
/// counted, and a table that takes no delete says why.
#[test]
fn a_row_that_cannot_be_deleted_is_left_out_and_said() {
    let mut page = book_covers_page(5);
    page.rows[2][0] = tabletist_db::Value::Null;
    let (mut harness, tab, id) = covers_with(Look::macos(), page);
    select_rows(&mut harness, tab, id, 1, 3);
    harness.press(Key::Backspace, Modifiers::NONE);
    assert_eq!(marked(&harness, tab, id), [1, 3]);
    assert_eq!(
        said(&harness).as_deref(),
        Some("2 rows marked for deletion. 1 can't be deleted.")
    );
    assert!(polite(
        &mut harness,
        "2 rows marked for deletion. 1 can't be deleted. 2 to delete."
    ));
    let table = tabletist_db::ObjectKind::Table;
    let (mut harness, tab, id) = covers_as(Look::macos(), table, true);
    select_rows(&mut harness, tab, id, 0, 2);
    harness.press(Key::Backspace, Modifiers::NONE);
    assert!(marked(&harness, tab, id).is_empty());
    assert_eq!(said(&harness).as_deref(), Some("Read-only connection"));
}

/// Spec 2 and test 9: each marked row keeps its pending cells under the
/// mark, and a restore of the range gives them back.
#[test]
fn the_rows_of_a_range_keep_their_edits_under_the_mark() {
    let (mut harness, tab, id) = covers_with(Look::macos(), book_covers_page(5));
    set(&mut harness, tab, id, (1, 2), "audio");
    set(&mut harness, tab, id, (3, 2), "audio");
    select_rows(&mut harness, tab, id, 1, 3);
    harness.press(Key::Backspace, Modifiers::NONE);
    assert!(edits(&harness, tab, id).cells.is_empty());
    harness.press(Key::Backspace, Modifiers::NONE);
    let cells: Vec<(usize, usize)> = edits(&harness, tab, id).cells.keys().copied().collect();
    assert_eq!(cells, [(1, 2), (3, 2)]);
}

/// Spec 2 and test 11: what a screen reader is told of several rows.
#[test]
fn a_screen_reader_is_told_how_many_rows_were_marked_and_restored() {
    let (mut harness, tab, id) = covers_with(Look::macos(), book_covers_page(5));
    select(&mut harness, tab, id, (0, 1));
    harness.press(Key::Backspace, Modifiers::NONE);
    select_rows(&mut harness, tab, id, 1, 3);
    harness.press(Key::Backspace, Modifiers::NONE);
    assert!(polite(
        &mut harness,
        "3 rows marked for deletion. 4 to delete."
    ));
    harness.press(Key::Backspace, Modifiers::NONE);
    assert!(polite(&mut harness, "3 rows restored."));
}

/// Spec 2, M8 and test 12: the inspector's Delete is of the row it shows.
#[test]
fn the_panels_delete_marks_the_cursors_row_alone() {
    for look in desktop() {
        let (mut harness, tab, id) = covers_with(look, book_covers_page(5));
        select_rows(&mut harness, tab, id, 1, 3);
        assert_eq!(button(&mut harness, "Delete"), (true, None));
        harness.click("Delete");
        harness.settle();
        assert_eq!(marked(&harness, tab, id), [3]);
        assert_eq!(rows(&harness, tab, id), [1, 2, 3]);
    }
}
```

In `the_panels_delete…` the row panel must be open: open it the way `delete_tests::the_panels_delete_marks_the_row` finds it open (it calls nothing, so the panel shows by default with a selected row).

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked --lib delete_many_tests`
Expected: `backspace_marks_the_selected_rows…` FAILS with `[3]` marked where `[1, 2, 3]` was expected: Backspace still marks the cursor's row. The others fail the same way.

- [ ] **Step 3: What is told**

`src/edit.rs`, in `enum Told`:

```rust
    /// Several rows in one action: how many were marked, how many of the
    /// rows named cannot be deleted, how many new rows were dropped with
    /// them, and how many rows are marked now.
    MarkedRows {
        marked: usize,
        skipped: usize,
        dropped: usize,
        deleted: usize,
    },
    RestoredRows {
        restored: usize,
    },
```

`src/ui/pending_bar.rs`, after `counted`:

```rust
/// "11 rows marked for deletion. 1 can't be deleted.": what a delete of
/// several rows marked, and how many of the rows it named it left out.
pub fn marked_text(marked: usize, skipped: usize, locale: Locale) -> String {
    let rows = counted(
        locale,
        marked,
        "row marked for deletion",
        "rows marked for deletion",
    );
    if skipped == 0 {
        return format!("{rows}.");
    }
    format!("{rows}. {skipped} {}", gettext(locale, "can't be deleted."))
}
```

`src/ui/data_view.rs`, in the `told` match (1380), after the `Told::Restored` arm:

```rust
        (
            Some(Told::MarkedRows {
                marked,
                skipped,
                dropped,
                deleted,
            }),
            _,
        ) => {
            let mut parts = Vec::new();
            if *marked > 0 {
                parts.push(super::pending_bar::marked_text(*marked, *skipped, locale));
                parts.push(format!("{deleted} {}", gettext(locale, "to delete.")));
            }
            if *dropped > 0 {
                let rows = super::pending_bar::counted(
                    locale,
                    *dropped,
                    "new row dropped",
                    "new rows dropped",
                );
                parts.push(format!("{rows}."));
            }
            Some(parts.join(" "))
        }
        (Some(Told::RestoredRows { restored }), _) => Some(format!(
            "{}.",
            super::pending_bar::counted(locale, *restored, "row restored", "rows restored")
        )),
```

The spec's order with rows left out is `11 rows marked for deletion. 1 can't be deleted. 14 to delete.`: `marked_text` gives the first two sentences, and the count follows.

- [ ] **Step 4: The words of a refusal**

`src/ui/cell_editor.rs`, after `delete_said`:

```rust
/// What a delete of several rows says where a refusal is said: why none
/// of them is deleted, or how many it left out. `None` where every row it
/// named is marked, restored or dropped.
pub fn sweep_said(
    sweep: &crate::edit::Sweep,
    table: &str,
    look: &Look,
    locale: Locale,
) -> Option<String> {
    use crate::edit::Sweep;
    match sweep {
        Sweep::TooMany => {
            let text = gettext(locale, "Select fewer rows, or delete in the SQL editor.");
            Some(look.label(&text))
        }
        Sweep::Refused(lock) => Some(delete_said(*lock, table, look, locale)),
        Sweep::Mark { rows, skipped, .. } if *skipped > 0 => {
            let text = crate::ui::pending_bar::marked_text(rows.len(), *skipped, locale);
            Some(look.label(&text))
        }
        Sweep::Mark { .. } | Sweep::Restore { .. } | Sweep::Drop { .. } => None,
    }
}
```

- [ ] **Step 5: The action and its reducer**

`src/model.rs`, after `DeleteRow { … },`:

```rust
    /// Delete the selected rows. One row is `DeleteRow`'s. Of several, the
    /// unmarked are marked and the new ones dropped, or, where every one
    /// is marked, all are restored (`edit::Table::sweep`).
    DeleteSelected {
        tab: ConnTabId,
        id: TabId,
    },
```

`src/app/editing.rs`, in `dropped_under_a_prompt`, after `| Action::DeleteRow { .. }`: `| Action::DeleteSelected { .. }`.

`src/app.rs`, after the `Action::DeleteRow` arm: `Action::DeleteSelected { tab, id } => self.delete_selected(tab, id),`.

`src/app/editing.rs`, after `delete_row`. Add `Sweep` to the file's `use crate::edit::{…}`:

```rust
    /// Deletes the selected rows. One row is [`Self::delete_row`]'s.
    pub(super) fn delete_selected(&mut self, tab: ConnTabId, id: TabId) {
        let rows = self
            .workspace(tab)
            .and_then(|workspace| workspace.object_tab(id))
            .map(ObjectTab::selected_rows)
            .unwrap_or_default();
        if rows.len() < 2 {
            return self.delete_row(tab, id);
        }
        self.sweep_rows(tab, id, &rows);
    }

    /// Deletes `rows` in one action, as [`Table::sweep`] has it: marks
    /// them and drops the new ones among them, or restores them. A row is
    /// marked and restored as one row is.
    fn sweep_rows(&mut self, tab: ConnTabId, id: TabId, rows: &[usize]) {
        // The text being typed is part of its row's edits, which are kept
        // under the mark.
        self.close_editor(tab, id, true);
        let Some(sweep) = self.table(tab, id, |table, _| table.sweep(rows)) else {
            return;
        };
        let told = match sweep {
            // The view said why.
            Sweep::TooMany | Sweep::Refused(_) => return,
            Sweep::Restore { rows } => {
                for row in &rows {
                    self.restore(tab, id, *row);
                }
                Told::RestoredRows {
                    restored: rows.len(),
                }
            }
            Sweep::Mark {
                rows: marks,
                new,
                skipped,
            } => {
                let Some(object) = self.object_tab_mut(tab, id) else {
                    return;
                };
                // The counts of the rows that were marked are of another
                // set now.
                let stale = object.edits.counts_out();
                let marked = marks.iter().filter(|row| object.edits.mark(**row)).count();
                let dropped = drop_named(object, rows, &new);
                let deleted = object.edits.deleted.len();
                self.stop_counts(tab, stale);
                Told::MarkedRows {
                    marked,
                    skipped,
                    dropped,
                    deleted,
                }
            }
            Sweep::Drop { new } => {
                let Some(object) = self.object_tab_mut(tab, id) else {
                    return;
                };
                let dropped = drop_named(object, rows, &new);
                Told::MarkedRows {
                    marked: 0,
                    skipped: 0,
                    dropped,
                    deleted: object.edits.deleted.len(),
                }
            }
        };
        let Some(object) = self.object_tab_mut(tab, id) else {
            return;
        };
        object.edits.told = Some(told);
        // A tab that holds a change stays, as it does for a cell's.
        object.pinned = true;
        // What the last save said was of another set.
        object.edits.note = None;
        // The row panel shows the cursor's row as it is now.
        object.fields = None;
    }
```

And, as a function of the file:

```rust
/// Drops the new rows `new` of `object`, which an action named among
/// `named` (the grid's order, before the drop). The cursor and the range's
/// anchor, where theirs went, go to the nearest row of `named` that is
/// left; with none left, the range ends and the cursor is on the row that
/// stands where its own stood. Says how many were dropped.
fn drop_named(object: &mut ObjectTab, named: &[usize], new: &[usize]) -> usize {
    if new.is_empty() {
        return 0;
    }
    let height = object.page().map_or(0, |page| page.rows.len());
    // Where the cursor's row stood, for when no row named is left.
    let stood = object.selection.and_then(|cell| {
        Order::of(&object.edits.added, height).place(cell.row)
    });
    let dropped = new
        .iter()
        .filter_map(|row| new_id(*row))
        .filter(|id| object.edits.drop_row(*id))
        .count();
    // The rows named that are left, each with its place among them.
    let left: Vec<(usize, usize)> = named
        .iter()
        .copied()
        .enumerate()
        .filter(|(_, row)| !new.contains(row))
        .collect();
    let nearest = |row: usize| {
        let at = named.iter().position(|named| *named == row)?;
        let near = left.iter().min_by_key(|(place, _)| place.abs_diff(at));
        near.map(|(_, row)| *row)
    };
    if let Some(range) = object.range.as_mut()
        && new.contains(&range.anchor)
    {
        match nearest(range.anchor) {
            Some(row) => range.anchor = row,
            None => object.range = None,
        }
    }
    if let Some(cell) = object.selection.filter(|cell| new.contains(&cell.row)) {
        let order = Order::of(&object.edits.added, height);
        let last = order.len().saturating_sub(1);
        let row = nearest(cell.row).or_else(|| order.row(stood?.min(last)));
        let cell = row.map(|row| CellPos { row, col: cell.col });
        object.set_cursor(cell, Reach::Keep);
    }
    dropped
}
```

(`let … && …` in an `if let` is a let chain, as the file already uses them. If the borrow of `order` and `object.set_cursor` clash, take `row` in its own block, as `drop_row` does.)

`App::restore` sets `Told::Restored` for each row and stops the counts once: the first call takes them, the rest find none. `sweep_rows` writes `told` after the loop, so the last word is the count.

- [ ] **Step 6: The key asks for the selected rows**

`src/ui/keys.rs`. After the `delete_row` function (1382):

```rust
/// Asks for the selected rows of the tab `id` to be deleted. One row is
/// [`delete_row`]'s. Of several, hands back what there is to say where a
/// refusal is said: why none is deleted, or how many were left out.
fn delete_selected(
    app: &App,
    tab: ConnTabId,
    id: TabId,
    actions: &mut Vec<Action>,
) -> Option<String> {
    let several = app.workspace(tab).and_then(|workspace| {
        let object = workspace.object_tab(id)?;
        let rows = object.selected_rows();
        if rows.len() < 2 {
            return None;
        }
        let sweep = crate::edit::Table::of(workspace, object)?.sweep(&rows);
        let table = crate::ui::format::display_safe(&object.object.name);
        let said = crate::ui::cell_editor::sweep_said(&sweep, &table, &app.look, app.locale);
        Some((sweep, said))
    });
    let Some((sweep, said)) = several else {
        return delete_row(app, tab, id, actions);
    };
    if !matches!(
        sweep,
        crate::edit::Sweep::TooMany | crate::edit::Sweep::Refused(_)
    ) {
        actions.push(Action::DeleteSelected { tab, id });
    }
    said
}
```

At 1288, the desktop looks' Backspace: `refused = delete_row(app, tab, id, actions);` becomes `refused = delete_selected(app, tab, id, actions);`.

At 1920, the terminal look's `dd`: the same change, inside its arm, only where the keys are the grid's:

```rust
                let why = if field.is_none() {
                    delete_selected(app, tab, id, actions)
                } else {
                    delete_row(app, tab, id, actions)
                };
                if let Some(why) = why {
                    crate::ui::toast::say(ctx, &why);
                }
```

(A new row's form sends `dd` from a field: that is the cursor's row alone.)

The inspector's Delete (`src/ui/row_panel.rs:1055`) is not touched: it sends `DeleteRow`.

- [ ] **Step 7: Run the tests**

Run: `~/.cargo/bin/cargo test --locked --lib delete_many_tests`, then `~/.cargo/bin/cargo test --locked --lib delete_tests`, then the four checks and the shots' clippy.
Expected: all pass. `delete_tests` is unchanged: with one row selected, every path is the one it was.

If `more_than_a_thousand_rows…` is slow (it draws a grid of 1,001 rows, of which a frame draws those in view), it should still take well under a second.

- [ ] **Step 8: Commit**

```bash
git add src/edit.rs src/model.rs src/app.rs src/app/editing.rs src/ui/keys.rs src/ui/cell_editor.rs src/ui/pending_bar.rs src/ui/data_view.rs src/ui/delete_many_tests.rs
git commit -m "Mark, restore and drop the selected rows in one press

Up to 1,000 rows an action. A press marks when any of them is
unmarked and restores only when all are marked.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 6: The guide, and the checks

**Files:**
- Modify: `docs/_guide/editing-data.md:247-287`, `docs/_guide/macos.md:120`, `docs/_reference/keyboard-shortcuts.md:84`
- Modify: this plan (its status)

- [ ] **Step 1: The guide**

`docs/_guide/editing-data.md`. In "Delete a row", after the paragraph that ends `leaves nothing to save.`:

```markdown
To delete several rows at once, select them first: hold Shift and click
another row, or hold Shift and press the Up or Down arrow. The footer says
how many rows are selected. Backspace then marks them all. Pressed again
it restores them all, and where only some of them were marked it marks the
rest. New rows among them are dropped. One press takes up to 1,000 rows:
for more, delete with a statement in the SQL editor.
```

The last paragraph of the section loses its last sentence (`Each press marks one row; mark several by pressing the key on each.`) and gains: `In the Omarchy look a press marks one row.` (Slice 2 takes that sentence out again.)

`docs/_reference/keyboard-shortcuts.md`, after the `Backspace, Delete` row:

```markdown
| Shift+Up, Shift+Down | Select the row above or below with the selected rows. Shift with a click selects through to the clicked row. Backspace then [deletes them all]({% link _guide/editing-data.md %}#delete-a-row) |
```

`docs/_guide/macos.md`, after its `Backspace, Forward Delete` row:

```markdown
| Shift+Up, Shift+Down | Select the row above or below with the selected rows. Backspace then deletes them all |
```

- [ ] **Step 2: Check the site**

Run: `cd docs && bin/check`
Expected: it passes, with no broken link.

- [ ] **Step 3: Everything, once more**

Run the four checks and the shots' clippy on the branch's head.
Expected: all pass. PostgreSQL and MySQL tests print `skipped` here (no Docker): this slice changes nothing they cover.

- [ ] **Step 4: Note what was built**

Under the header of this plan, add a `Status:` line as the earlier plans have: the date, and each place a task left the plan as written.

- [ ] **Step 5: Commit, and push**

```bash
git add docs/_guide/editing-data.md docs/_guide/macos.md docs/_reference/keyboard-shortcuts.md docs/superpowers/plans/2026-10-13-deleting-many-rows-1-desktop.md
git commit -m "Say in the guide how several rows are selected and deleted

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
git push
```

Then check that the remote head is the commit, and add slice 1 to the description of pull request `igor-alexandrov/tabletist#120`: what it builds, the defaults M1, M2, M5, M6, M8 and M9 it was built to, and P2 (the two commands are in Help and not in Settings > Keyboard).

---

## What this slice does not build

- `V`, `d` on a visual selection, the `-- VISUAL --` mode line and `{count}dd`: slice 2.
- Deletes in runs of `IN (…)`, the review's lines for a run, counts in parts and the shorter note of rows already gone: slice 3.
- The parent spec's "As built" note points at tests 3 and 7 only once slices 2 and 3 are in.
