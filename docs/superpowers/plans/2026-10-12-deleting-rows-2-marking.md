# Deleting Rows, Run 2: Marking a Row Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A row of a table's page can be marked for deletion and restored (⌫ and the row panel's Delete and Restore in the desktop looks, `dd` in the Omarchy look, ⌘Z and `u` to take the mark back), is drawn as marked in both looks, is counted and reviewed with the other pending changes, and is deleted by the save, which then takes it out of the grid.

**Architecture:** The pending set (`Edits`, `src/edit.rs`) gains `deleted`: the page's rows that are marked, each with what its `DELETE` failed with in the last save and with the row's own pending cells, which marking takes out of `cells` and restoring puts back. So a marked row's edits stay in the set and nothing that reads `cells` sees them. `change_set` turns the marks into the `deletes` run 1 gave a save. A marked row is locked like a row that is gone (`Lock::Deleting`), so no editor opens on it and the row panel reads it. The reducer has two new actions, `DeleteRow` (marks, restores, or drops a new row) and `RestoreRow`. The views draw from the set: a row mark and two cell marks in the grid, one style behind `env::deleted_row_colors`, a count wherever the pending changes are counted, and a statement in the review.

**Tech Stack:** Rust 1.98, egui/eframe (the crmne forks). Spec: `docs/superpowers/specs/2026-10-12-deleting-rows-design.md` ("spec N" below is a section of its Behavior, "test N" an entry of its Tests). Run 1: `docs/superpowers/plans/2026-10-12-deleting-rows-1-save.md`, whose table "What the spec asks, and what the app can do today" the user approved as it stands on 2026-10-12 ("row a" and so on below are its rows).

Status: built on 2026-10-12 and 2026-10-13, one commit per task. Each task's blocks are the plan's as written, but for these:

- Row z was decided with the user before task 5: the marker is a minus before the row's key, which is struck through with the row's other values (`− 9`), not `− delete`. `Mark::Deleting` keeps the cell's value, `paint_struck` writes the minus unstruck before it, and `Harness::struck` holds each struck run of a text, not only texts that are struck whole.
- Task 1: the tests' helpers are `marked_covers` and `pending_text` (`typed_as` is the module's own). A static `NONE_DELETED`, for tests alone, stands where a test builds a `Table` without a set.
- Task 4: the reason is worded by a new `cell_editor::delete_said`, which keeps the table's name in the Omarchy look's lower case, and a test holds the Omarchy toast.
- Task 6: a row of the same test holds that `dd delete` is struck out on a read-only connection.
- Task 7: the Omarchy status line's tail after a refused delete is "the row stays marked". A second test holds the words for two rows already deleted.
- Task 9: in the Omarchy look the footer's cell on a marked row reads `dd restore` and works, since the key restores there; the desktop looks' Delete is disabled on a marked row, as planned. The Restore button takes its width from `ButtonSpec::width`. `the_panels_duplicate_makes_a_copy_and_delete_waits` became `the_panels_duplicate_makes_a_copy`, and `the_row_panels_duplicate_can_be_pressed_and_delete_waits` became `the_row_panels_duplicate_and_delete_can_be_pressed`.
- Task 11: the scene was rendered alone, through a test that was taken out again, and looked at in the macOS look light and dark and the Omarchy look dark. The Omarchy header shows "13 rows" without "1 to delete" at that width: its summary gives way to the views beside it, as it did before, and the status line has the count.
- Tasks were built while commits could not be signed (1Password was locked), each task's tree kept in a scratch store and committed in order once it could be.

---

## Before you start

- Cargo is `~/.cargo/bin/cargo` (the mise shim fails). Never point `CARGO_TARGET_DIR` at `/tmp`.
- The four checks, from `AGENTS.md`. "Run the four checks" below means these, and all four pass after every task:

```bash
~/.cargo/bin/cargo fmt --all --check
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
~/.cargo/bin/cargo test --locked --workspace --all-targets
RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --locked --workspace --no-deps
```

- `src/shots.rs` is outside those four. Tasks that change a type it builds by hand (`grid::Row`, `Saved`, `WritePrompt`) and task 12 also run `~/.cargo/bin/cargo clippy --locked --features shots --lib --tests -- -D warnings`.
- **What was run where this plan was written: nothing of it.** It was written from reading the tree at `46e4ab8` (run 1's head) and the canvas' boards `OmarchyWideEdit`, `OmarchyEdit`, `OmarchyEditValues`, `OmarchyInsert`, `Main`, `MainEdit`, `MacEditFlow`, `MacInsertStates` and both `MacComponents`. The code blocks are written against names that exist in that tree and were not compiled. Treat a block as what the code should come to, and expect to correct a name, a type or a borrow on the first build. Where a block and the compiler disagree, the compiler is right and the task's tests say what must hold. Line numbers are of `46e4ab8` and move as the tasks go: find a place by the names around it.
- **The branch** is `claude/row-deletion-table-view-903d1e`, pushed, with run 1 on it. The pull request is opened after this run's last task (see "After the last task").
- House rules that bite here: no em dashes (the marker's `−` is a minus sign, U+2212, and stays); comments explain why, in the surrounding code's voice; a view pushes `Action`s and changes no state; views draw text only through `TextRole`s; a view never paints a focus ring; a key is spelled in `src/keymap.rs` and nowhere else (`tests/keys.rs` finds one spelled by hand); every behaviour change has a headless test; no test compares pixels, and no design material is committed; tests and scenes use the Bookshop's `book_covers` (`Harness::book_covers`: three rows, ids 1 to 3, `id` its key and its identity column) and never another project's data.
- A UI string goes through `gettext(locale, "...")`, and through `look.label(..)` where the Omarchy look says it (lower case). There is no catalogue to add a string to.
- Commits are signed, one per task, after its checks pass. If signing fails ("agent refused operation"), 1Password is locked: do not commit unsigned, stage the task and tell the user. Chain `git add` and `git commit` with `&&`. Each message ends with:

      Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>

- `cargo test` takes one name to filter by before `--` and any number after it.
- The new UI tests live in one new file, `src/ui/delete_tests.rs`, registered in `src/ui/mod.rs` beside `duplicate_tests` (`#[cfg(test)] mod delete_tests;`). Task 3 creates it with this head, and later tasks add to it:

```rust
//! Deleting rows, checked against its spec
//! (`docs/superpowers/specs/2026-10-12-deleting-rows-design.md`: "spec N"
//! below is a section of its Behavior, "test N" an entry of its Tests).
//!
//! Only behaviour, names and words are checked here, on the Bookshop's
//! `book_covers`. No colour, size or picture of the design is.

use egui::{Key, Modifiers};

use super::insert_audit_tests::{add, covers_as, covers_in, edits, select, selected, set};
use crate::backend::Command;
use crate::model::{Action, ConnTabId, TabId};
use crate::testing::Harness;
use crate::theme::Look;

/// The last save that was sent.
fn sent(harness: &Harness) -> &tabletist_db::ChangeSet {
    let mut sent = harness.app.backend.sent.iter().rev();
    sent.find_map(|command| match command {
        Command::Write { changes, .. } => Some(changes),
        _ => None,
    })
    .expect("a save was sent")
}

/// Marks the row at `row` of the page, as the keys do.
fn mark(harness: &mut Harness, tab: ConnTabId, id: TabId, row: usize) {
    select(harness, tab, id, (row, 1));
    harness.app.apply(Action::DeleteRow { tab, id });
    harness.settle();
}
```

  An import or a helper a task does not use yet fails clippy: add each with the first test that uses it. Task 4 adds `Key`, `Modifiers`, `painted`, `type_key` and:

```rust
/// The desktop looks: the keys and the buttons are the same in both.
fn desktop() -> [Look; 2] {
    [Look::standard(), Look::macos()]
}
```

  `tests/keys.rs` reads every file under `src` for a key spelled by hand and lets a list of test files be (`TESTS: [&str; 7]`, near line 22). Task 3 adds `src/ui/delete_tests.rs` to it (8), since its tests press keys by name.

## The runs

| Run | What it builds | Seen by a user |
|---|---|---|
| 1 (built) | The save deletes rows | No |
| **2 (this plan)** | Marking in the grid and the row panel, the marked row in both looks, the reasons, the counts, Review SQL, the production confirm, the save's results, the labels and announcements, the website's pages | Yes |
| 3 | The foreign keys that point at the table: the `ON DELETE CASCADE` and `SET NULL` warnings with their counts, a refused delete's "3 rows in book_covers still point at this row · Open rows", and a save's deletes ordered by the table's own keys | Yes |
| 4 | Many rows at once (a row selection, `d` on a visual selection, `{count}dd`, the cap, the batches): a spec of its own | Yes |

## What gets built where run 1's table left a choice

The rows of run 1's table that are this run's, as they were approved, and what reading the views added. Rows r to x are new: say so before task 1 if one should be otherwise.

| | The spec | What gets built |
|---|---|---|
| a | ⌘Z / `u` unmark; undo and redo cover both | ⌘Z and `u` on a marked row restore it, before they touch a cell. No redo |
| c | No DELETE privilege disables the command | Not disabled: the app reads no privilege. The server's refusal is the row's error after the save |
| d | In a query's result: "Rows from a query can't be deleted here" | Nothing new. ⌫ and `dd` are keys of the `Grid` scope and a result's rows are read in `Results`, where they are bound to nothing, and a result's row panel has no footer. Binding them there only to say a sentence would take the two keys from whatever a result's grid gives them later |
| i | A refused delete: "3 rows in book_covers still point at this row · Open rows" | The server's own words, after its code, on the row and in the bar. The count and Open rows are run 3's |
| j | Error navigation includes delete errors | Not built (the commands are not). A failed delete's row is marked as in trouble |
| k | "{object} can't be deleted from", "{table} has no primary key, so rows can't be deleted here" | Those sentences, for Delete alone. Editing keeps its own |
| l | "`{n} pending` as today" | A third count on the Omarchy status line, "-1 to delete", in the danger colour, after "+1 new" and "3 pending · 2 rows" |
| m | "Save 1 insert, 1 update and 1 delete to bookshop_production?" | "Save 1 new row, 2 changes and 1 deleted row to production?" |
| n | "Nothing was saved. N rows failed, so the whole transaction rolled back." | That sentence with N as 1, since a save stops at its first failing statement, then the database's words |
| r | The id cell's marker reads `− delete`, in the same place as `+ new` | In the desktop looks the first column of the row's key shows `− delete` in place of its value, as a new row's shows `+ new` in place of none. The row's key is still in the row panel's title and in Review SQL. The Omarchy look keeps the value and has `-` in the gutter, as its board draws it |
| s | Values struck through and dimmed | Every cell of a marked row is written as plain text, dimmed and struck through: a tag, a chip, a JSON count and the NULL chip are drawn as their text. One way to strike a value, whatever its column draws |
| t | "Row id 9 was already deleted," | A line in the pending bar and on the Omarchy status line after the save, until it is dismissed or the next save: "Row id 9 was already deleted." Where several were: "2 rows were already deleted: id 9, id 12." |
| u | The inspector's line "Will be deleted on save, with Restore" | The line under the panel's header, where a locked row says why, with a Restore button at its right end in the desktop looks. In the Omarchy look the line ends with the key (`dd restore`). The footer's Delete is disabled on a marked row, its reason the line's words |
| v | A marked row's edits are hidden | Marking takes the row's pending cells, and the warnings on them, out of the set's cells and keeps them with the mark. They are not drawn, counted, sent, copied or shown in the row panel, and hold no save back, also where one of them is to fix. Restoring puts them back as they were |
| y | The `:diff` card of the full-width board: `- delete · id 9`, `u undo`, the statement | The review's lines as they are for a changed and a new row: a comment, `-- delete row id 9`, and the statement under it. The board's cards (`~ update · id 2`, `+ insert · new row`, each with `u undo`) were built as comments when Review SQL was, and a delete follows them |
| z | `− delete` in the same place as `+ new` | The macOS look's key column is 64 points wide when it holds numbers, and `− delete` does not fit it as `+ new` does. Decided with the user before task 5 (the column grows while a row is marked, or the marker is shorter) |
| w | The cursor moves to the next row after the save | The row that stood after the selected one, or the last where it was the last. The page keeps its other rows where they are: nothing is fetched, so a page that was full shows fewer rows until it is loaded again |
| x | Announcements | A polite live region on a point of the grid, as a foreign key picker's choice is told: "Row id 9 marked for deletion. 1 to delete.", "Row id 9 restored.", and after the save "1 row deleted." No toast is shown for them |

## What was decided for this run

- **The mark is on the page's row, not on its key.** `deleted: BTreeMap<usize, Doomed>`, by the row's place in the loaded page, as `cells` and `gone` are. A tab that holds a mark holds its page (`Edits::holds`), so the place stays the row's.
- **A marked row's cells are set aside, not filtered.** `Doomed` holds the row's pending cells and their doubts while it is marked. Everything that reads `Edits::cells` (the counts, the set a save sends, the review, the row panel's fields, a copy, a duplicate, the warnings, the Omarchy error line) is right without knowing of marks. Found in the plan's review: the first draft filtered in four places and missed seven.
- **A marked row is a locked row.** `Table::row_lock` answers `Lock::Deleting` after `Lock::Gone`. Everything that asks whether a cell can be edited (an editor, Set NULL, a paste, the row panel's form) is refused by the one answer, worded "Restore the row to edit it."
- **Why a row cannot be deleted is its lock without that one.** `Table::no_delete(row)`: what keeps the row from being edited keeps it from being deleted (a read-only connection, a view, no key, a key that cannot be matched, a save, a fetch, a row that is gone, a key that is NULL).
- **`DeleteRow` is the keys' and the button's one action.** On a new row it drops it (`drop_row`, as before); on a row of the page it marks it, or restores it where it is marked (the spec's default 1, toggle). `DropRow` stays for the new row form's "Discard new row".
- **The deletes go in the page's order.** `change_set` reads the marked rows by their place. Ordering them by the table's own foreign keys is run 3's (row q of run 1's table).
- **Nothing is fetched after a save that deletes.** The reducer takes the rows out of the page as it puts the new ones in. Where the answer does not fit the page it reads the page again, as it does for a new row that came back unknown.

## File map

| File | What changes |
|---|---|
| `src/edit.rs` | `Edits::deleted`, `told`; `Counts::deleted`; `RowMark::Deleted`; `Lock::Deleting`; `Table::deleted`, `Table::no_delete`; `mark`, `restore`, `fail_delete`; `counts`, `row_mark`, `change_set`, `Sent`, `Saving`, `Saved`, `Note` |
| `src/review.rs`, `src/ui/review.rs` | A delete's lines, its comment, its count |
| `src/model.rs` | `Action::DeleteRow`, `Action::RestoreRow`; `WritePrompt::deleted` |
| `src/app.rs`, `src/app/editing.rs` | The two actions; `RevertCell` on a marked row; the save's set, prompt and answer |
| `src/ui/keys.rs` | ⌫, `dd`, the refusal's words |
| `src/ui/cell_editor.rs` | `lock_text` for `Lock::Deleting`; `delete_text` |
| `src/env.rs` | `DeletedRow`, `deleted_row_colors`: the one style |
| `src/ui/grid.rs`, `src/ui/data_view.rs` | The row's mark, fill, bar, gutter sign, label; the cells' marks; the strike; the subtitle; the announcements; the error line |
| `src/ui/pending_bar.rs`, `src/ui/workspace.rs` | The counts, the notes, what a save wrote; the status line's count and key |
| `src/ui/write_prompts.rs` | The production confirm's sentence |
| `src/ui/row_panel.rs` | Delete, Restore, the line, the hint |
| `src/testing.rs` | `Harness::struck` |
| `src/ui/delete_tests.rs` | New. This run's UI tests |
| `tests/keys.rs` | The new test file among those that may name a key |
| `src/ui/mod.rs`, `src/ui/duplicate_tests.rs` | The tests that said Delete waits |
| `src/shots.rs` | One scene, for looking at |
| `docs/_guide/editing-data.md`, `docs/_guide/macos.md`, `docs/_guide/omarchy.md`, `docs/_reference/keyboard-shortcuts.md` | Deleting a row |
| `docs/superpowers/specs/*.md` | "As built" in this spec; a line in the three older specs that say deleting waits |

---

### Task 1: The pending set marks rows

Everything here is in `src/edit.rs`, with one arm in `src/ui/cell_editor.rs` and one in `src/ui/workspace.rs` that the compiler asks for. No view shows a mark yet, and no key sets one.

**Files:**
- Modify: `src/edit.rs`
- Modify: `src/ui/cell_editor.rs` (`lock_text`), and wherever the compiler names a match on `Lock` or `RowMark` or a `Table { .. }`, `Counts { .. }`, `Sent { .. }`, `Saving { .. }` or `Saved { .. }` built by hand

- [ ] **Step 1: Write the failing tests**

In `src/edit.rs`, in `mod tests`. Use the module's own helpers for a table of `book_covers` where it has them (`the_bookshops_covers_have_a_column_of_each_kind` builds one from `crate::testing::book_covers_structure()` and `book_covers_page(3)`); `marked_covers` below is that, written out (`covers` and `typed` are taken in that module):

```rust
    /// The Bookshop's covers as editing sees them, with `edits`' rows.
    fn marked_covers<'a>(
        structure: &'a tabletist_db::Structure,
        page: &'a RowPage,
        edits: &'a Edits,
    ) -> Table<'a> {
        Table {
            access: Access::Writable,
            kind: ObjectKind::Table,
            dialect: Dialect::Sqlite,
            structure: Some(structure),
            page,
            refreshing: false,
            saving: false,
            gone: &edits.gone,
            added: &edits.added,
            deleted: &edits.deleted,
        }
    }

    fn typed_as(text: &str) -> Pending {
        Pending {
            new: NewValue::Text(text.into()),
            state: State::Ready,
        }
    }

    /// Spec 1 and test 1: a mark, and the mark taken back.
    #[test]
    fn a_row_is_marked_and_restored() {
        let mut edits = Edits::default();
        assert!(edits.mark(1));
        // A second mark is none: the row is marked already.
        assert!(!edits.mark(1));
        assert!(edits.pending() && edits.holds());
        assert_eq!(edits.counts().deleted, 1);
        assert_eq!(edits.row_mark(1), RowMark::Deleted);
        assert!(edits.restore(1));
        assert!(!edits.restore(1));
        assert!(!edits.pending());
        assert_eq!(edits.row_mark(1), RowMark::None);
    }

    /// Spec 1, "Existing edits", and test 4: the edits of a marked row stay
    /// in the set, are not counted and are not sent. They are back with
    /// the row.
    #[test]
    fn a_marked_rows_edits_are_kept_and_hidden() {
        let structure = crate::testing::book_covers_structure();
        let page = crate::testing::book_covers_page(3);
        let object = ObjectRef::new("main", "book_covers");
        let mut edits = Edits::default();
        edits.put((1, 2), typed_as("audio"));
        // One that would hold a save back, were it counted.
        edits.put(
            (1, 1),
            Pending {
                new: NewValue::Text("x".into()),
                state: State::ToFix(Problem::NotANumber),
            },
        );
        edits.mark(1);
        // Set aside with the mark: nothing that reads the cells sees them.
        assert!(edits.cells.is_empty());
        let counts = edits.counts();
        assert_eq!((counts.changes, counts.rows, counts.to_fix), (0, 0, 0));
        assert_eq!(counts.deleted, 1);
        let table = marked_covers(&structure, &page, &edits);
        let (changes, sent) = change_set(&object, &table, &edits.cells).unwrap();
        assert!(changes.rows.is_empty() && changes.inserts.is_empty());
        assert_eq!(
            changes.deletes,
            [tabletist_db::RowDelete {
                key: vec![("id".to_owned(), Value::Int(2))],
            }]
        );
        assert_eq!(sent.deletes, [1]);
        // Restored, the row is a changed row again.
        edits.restore(1);
        assert_eq!(edits.cells.len(), 2);
        assert_eq!(edits.counts().changes, 2);
    }

    /// Spec 3: no cell of a marked row is edited, and the row says why.
    #[test]
    fn a_marked_row_is_locked_until_it_is_restored() {
        let structure = crate::testing::book_covers_structure();
        let page = crate::testing::book_covers_page(3);
        let mut edits = Edits::default();
        edits.mark(1);
        let table = marked_covers(&structure, &page, &edits);
        assert_eq!(table.row_lock(1), Some(Lock::Deleting));
        assert_eq!(
            table.lock(CellPos { row: 1, col: 2 }),
            Some(Lock::Deleting)
        );
        assert_eq!(table.row_lock(0), None);
        // A marked row can be asked to be deleted again: that restores it.
        assert_eq!(table.no_delete(1), None);
    }

    /// Spec 2 and test 6, as far as the set knows: what keeps a row from
    /// being edited keeps it from being deleted.
    #[test]
    fn a_row_that_cannot_be_edited_cannot_be_deleted() {
        let structure = crate::testing::book_covers_structure();
        let page = crate::testing::book_covers_page(3);
        let edits = Edits::default();
        let table = marked_covers(&structure, &page, &edits);
        assert_eq!(table.no_delete(1), None);
        let read_only = Table {
            access: Access::ReadOnly,
            ..table
        };
        assert_eq!(read_only.no_delete(1), Some(Lock::ReadOnly));
        let view = Table {
            kind: ObjectKind::View,
            ..table
        };
        assert_eq!(view.no_delete(1), Some(Lock::NotATable));
        let keyless = tabletist_db::Structure {
            primary_key: Vec::new(),
            ..structure.clone()
        };
        let no_key = Table {
            structure: Some(&keyless),
            ..table
        };
        assert_eq!(no_key.no_delete(1), Some(Lock::NoKey));
        let saving = Table {
            saving: true,
            ..table
        };
        assert_eq!(saving.no_delete(1), Some(Lock::Saving));
    }

    /// Spec 5: a delete that failed says so on its row until the row is
    /// restored or marked again.
    #[test]
    fn a_failed_delete_stands_on_its_row() {
        let mut edits = Edits::default();
        edits.mark(1);
        let error = Error::query("FOREIGN KEY constraint failed");
        edits.fail_delete(Some(1), error.clone());
        let failed = edits.deleted.get(&1).and_then(|doomed| doomed.failed.clone());
        assert_eq!(failed, Some(error.clone()));
        assert_eq!(edits.counts().failed, 1);
        assert_eq!(edits.row_mark(1), RowMark::Trouble);
        assert_eq!(
            edits.note,
            Some(Note::FailedDelete {
                row: 1,
                error: error.clone()
            })
        );
        // A row the save did not send has no place to say it.
        let mut other = Edits::default();
        other.fail_delete(None, error.clone());
        assert_eq!(other.note, Some(Note::Refused(error)));
    }
```

`Problem::NotANumber` stands for any `Problem`: take a variant the enum has (`src/edit.rs`, near line 537).

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked --lib edit::tests::a_marked`
Expected: FAIL to compile: no field `deleted` on `Edits`, no method `mark`.

- [ ] **Step 3: The types**

`Lock` gains a variant after `Gone`:

```rust
    /// The row is marked for deletion: it is restored before it is edited.
    Deleting,
```

`Table` gains a field after `added`, and `Table::of` fills it:

```rust
    /// The page's rows marked for deletion (`Edits::deleted`).
    pub deleted: &'a BTreeMap<usize, Doomed>,
```

```rust
            added: &object.edits.added,
            deleted: &object.edits.deleted,
```

`Edits` gains two fields, after `gone`:

```rust
    /// The page's rows marked for deletion. A marked row has no cell in
    /// `cells` and no doubt in `doubts`: `mark` takes them out and keeps
    /// them with the mark, and `restore` puts them back.
    pub deleted: BTreeMap<usize, Doomed>,
    /// The last row that was marked or restored, for a screen reader to
    /// be told.
    pub told: Option<Told>,
```

After `Picked`:

```rust
/// A row of the page that is marked for deletion. No `Debug`: it holds
/// what the user typed.
#[derive(Default)]
pub struct Doomed {
    /// What its `DELETE` failed with in the last save.
    pub failed: Option<Error>,
    /// The row's pending cells as they stood when it was marked, by
    /// column: kept for its restoring, and no part of the set until then.
    cells: Vec<(usize, Pending)>,
    /// The warnings that stood on those cells.
    doubts: Vec<(usize, Doubt)>,
}

/// What a screen reader is told of the last row that was marked for
/// deletion or restored. The row by its key (`id 9`), as the row panel's
/// title names one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Told {
    /// With how many rows are marked now.
    Marked { row: String, deleted: usize },
    Restored { row: String },
}
```

`Counts` gains a field, `RowMark` a variant:

```rust
    /// The rows to delete.
    pub deleted: usize,
```

```rust
    /// A row of the page that is marked for deletion.
    Deleted,
```

`Sent` and `Saving` each gain the rows a set's deletes are of, and `Saved` what a save deleted:

```rust
    /// The page's row of each of the set's `deletes`.
    pub deletes: Vec<usize>,
```

```rust
    /// The page's row of each of the set's deletes, in its order.
    pub deletes: Vec<usize>,
```

```rust
    /// How many rows it deleted.
    pub deleted: usize,
```

`Note` gains two variants, and its first line changes, since one of them is of a save that wrote:

```rust
/// What a save came to when it wrote nothing, or what it has to say
/// beside what it wrote. The view words it.
```

```rust
    /// The `DELETE` of the page's row `row` failed.
    FailedDelete { row: usize, error: Error },
    /// The save wrote, and found these rows already deleted: each by its
    /// key (`id 9`), named before the save took them out of the page.
    AlreadyDeleted(Vec<String>),
```

- [ ] **Step 4: The set's own methods**

In `impl Edits`:

```rust
    pub fn holds(&self) -> bool {
        !self.cells.is_empty()
            || !self.added.is_empty()
            || !self.deleted.is_empty()
            || self.editor.is_some()
            || self.saving.is_some()
    }

    /// Whether anything waits for a save: a changed cell, a new row, or a
    /// row to delete. An open editor alone is not that yet.
    pub fn pending(&self) -> bool {
        !self.cells.is_empty() || !self.added.is_empty() || !self.deleted.is_empty()
    }
```

`counts` counts the marks. Its loop over the cells is as it is: a marked row has none there.

```rust
        let mut counts = Counts {
            added: self.added.len(),
            deleted: self.deleted.len(),
            ..Counts::default()
        };
```

and after the new rows' failures:

```rust
        // A row whose `DELETE` failed has no cell to say so.
        counts.failed += self
            .deleted
            .values()
            .filter(|doomed| doomed.failed.is_some())
            .count();
        counts
```

After `drop_row`:

```rust
    /// Marks the page's row `row` for deletion. What was made of the set,
    /// its review, is stale. Says whether the row was not marked before.
    pub fn mark(&mut self, row: usize) -> bool {
        if self.deleted.contains_key(&row) {
            return false;
        }
        // The row's own changes go with the mark: they are not the set's
        // while the row is to be deleted.
        let of_row = |at: &(usize, usize)| at.0 == row;
        let cols: Vec<usize> = self
            .cells
            .range((row, 0)..=(row, usize::MAX))
            .map(|(&(_, col), _)| col)
            .collect();
        let cells = cols
            .into_iter()
            .filter_map(|col| Some((col, self.cells.remove(&(row, col))?)))
            .collect();
        let doubted: Vec<usize> = self
            .doubts
            .keys()
            .filter(|at| of_row(at))
            .map(|&(_, col)| col)
            .collect();
        let doubts = doubted
            .into_iter()
            .filter_map(|col| Some((col, self.doubts.remove(&(row, col))?)))
            .collect();
        let doomed = Doomed {
            failed: None,
            cells,
            doubts,
        };
        self.deleted.insert(row, doomed);
        // No editor stays open on a row that is not edited, and what was
        // refused of it is no longer the reason.
        let edited = self.editor.as_ref().map(|editor| editor.cell.row);
        if edited == Some(row) {
            self.editor = None;
        }
        if self.why.is_some_and(|(cell, _)| cell.row == row) {
            self.why = None;
        }
        self.review = None;
        true
    }

    /// Takes the mark off the page's row `row`: its cells are the set's
    /// again. Says whether the row was marked.
    pub fn restore(&mut self, row: usize) -> bool {
        let Some(doomed) = self.deleted.remove(&row) else {
            return false;
        };
        for (col, pending) in doomed.cells {
            self.cells.insert((row, col), pending);
        }
        for (col, doubt) in doomed.doubts {
            self.doubts.insert((row, col), doubt);
        }
        if self.why.is_some_and(|(cell, _)| cell.row == row) {
            self.why = None;
        }
        self.review = None;
        true
    }

    /// The `DELETE` of the page's row `row` failed with `error`: the row
    /// says so until it is restored or the next save sends it again. With
    /// no such row the save is refused with the error.
    pub fn fail_delete(&mut self, row: Option<usize>, error: Error) {
        let Some((row, doomed)) =
            row.and_then(|row| self.deleted.get_mut(&row).map(|doomed| (row, doomed)))
        else {
            self.note = Some(Note::Refused(error));
            return;
        };
        doomed.failed = Some(error.clone());
        self.note = Some(Note::FailedDelete { row, error });
    }
```

`Edits::row_mark` knows the mark first:

```rust
    pub fn row_mark(&self, row: usize) -> RowMark {
        match self.deleted.get(&row) {
            Some(doomed) if doomed.failed.is_some() => RowMark::Trouble,
            Some(_) => RowMark::Deleted,
            None => row_mark(&self.cells, row),
        }
    }
```

The free `row_mark` keeps its signature: a view calls it with the cells beside an open editor, and asks the set about the mark itself (task 5).

The `Debug` of `Edits` prints the count: add `deleted: {}` after `added: {}` with `self.deleted.len()`.

- [ ] **Step 5: The lock**

In `Table::row_lock`, after the `gone` test:

```rust
        // After a row that is gone: such a row cannot be marked.
        if self.deleted.contains_key(&row) {
            return Some(Lock::Deleting);
        }
```

After `no_rows`:

```rust
    /// Why the page's row `row` cannot be deleted, or `None` when it can:
    /// what keeps it from being edited, for the whole table, for a while,
    /// or by its own key. A row that is marked can be asked again, which
    /// restores it. A new row is dropped, not deleted, and has no reason.
    pub fn no_delete(&self, row: usize) -> Option<Lock> {
        if new_id(row).is_some() {
            return None;
        }
        self.row_lock(row).filter(|lock| *lock != Lock::Deleting)
    }
```

`Lock` is `PartialEq`, and this is no engine's enum: `tests/engines.rs` looks at `Driver` and `Dialect` only.

In `src/ui/cell_editor.rs`, `lock_text` gains the arm, with the spec's words:

```rust
        Lock::Deleting => say("Restore the row to edit it."),
```

- [ ] **Step 6: The set a save sends**

`change_set` adds the marks. Replace from `// The cells of the page's rows` to the end of the function:

```rust
    // The cells of the page's rows: every row below the new ones'. A row
    // marked for deletion has none here.
    let changed = cells.range(..(NEW_ROWS, 0));
    // The key only where a row is changed or deleted: a new row is found
    // by none.
    let key = if changed.clone().next().is_some() || !table.deleted.is_empty() {
        table.key()?
    } else {
        Vec::new()
    };
    // What finds the row whose values are `values`.
    let key_of = |values: &Vec<Value>| -> Option<Vec<(String, Value)>> {
        key.iter()
            .map(|&place| {
                let name = table.page.columns.get(place)?.name.clone();
                Some((name, values.get(place)?.clone()))
            })
            .collect()
    };
    let mut rows: Vec<RowChange> = Vec::new();
    // The map is ordered by row, then column, so a row's cells are together.
    for (&(row, col), pending) in changed {
        let values = table.page.rows.get(row)?;
        let column = table.column(col)?;
        if sent.rows.last() != Some(&row) {
            sent.rows.push(row);
            rows.push(RowChange {
                key: key_of(values)?,
                set: Vec::new(),
            });
        }
        rows.last_mut()?.set.push(CellChange {
            column: column.name.clone(),
            type_name: column.type_name.clone(),
            loaded: values.get(col)?.clone(),
            new: pending.new.clone(),
        });
    }
    // The rows to delete, in the page's order.
    let mut deletes = Vec::new();
    for &row in table.deleted.keys() {
        deletes.push(RowDelete {
            key: key_of(table.page.rows.get(row)?)?,
        });
        sent.deletes.push(row);
    }
    (!rows.is_empty() || !inserts.is_empty() || !deletes.is_empty()).then(|| {
        (
            ChangeSet {
                object: object.clone(),
                inserts,
                rows,
                deletes,
            },
            sent,
        )
    })
```

Its doc gains the rows: "The change set a save sends for the table's new rows, the pending `cells` and the rows marked for deletion". Import `RowDelete` from `tabletist_db` with the file's other imports.

- [ ] **Step 7: What the compiler names**

Run: `~/.cargo/bin/cargo check --locked --workspace --all-targets`

- A `Table { .. }` built by hand (tests in `src/edit.rs`, `src/review.rs`, `src/key_picker.rs`) gains `deleted: &edits.deleted`, or a reference to an empty map the test owns where it has no `Edits`.
- A match on `Lock` (`lock_text`, done above) or on `RowMark` (`src/ui/grid.rs`, two places: the row's tone and the gutter's sign) that names every variant gains the new one. In the grid, give `RowMark::Deleted` what `RowMark::Trouble` has for now (`Some(Tone::Danger)`, and `"-"` for the sign): task 5 draws it.
- `Saving { .. }` in `send_write` (`src/app/editing.rs`) gains `deletes: sent.deletes`. `Saved { .. }` in `written` gains `deleted: 0` for now: task 3 counts it.
- `Sent { .. }`, `Saved { .. }` and `Counts { .. }` in tests gain the field's default, and so does the `Saving { .. }` in `src/model.rs` (near line 3965).
- `discard` needs nothing: it keeps `gone` and drops the rest, the marks and what they hold with them. `drop_row`, `put` and `revert` need nothing either: they are of cells that are in `cells`.
- A match on `Note` that names every variant (`note_said` in `src/ui/pending_bar.rs`; the one in `src/ui/workspace.rs` has a `_` arm and asks for nothing) gains two arms. For now: `Note::FailedDelete { error, .. } => format!("{} {nothing}", database_said(error))` and `Note::AlreadyDeleted(_) => String::new()`. Task 7 words them.

- [ ] **Step 8: Run the tests**

Run: `~/.cargo/bin/cargo test --locked --lib edit::tests`
Expected: PASS, the five new tests among them.

- [ ] **Step 9: Run the four checks, and the shots' clippy**

Expected: all pass.

- [ ] **Step 10: Commit**

```bash
git add src && git commit -m "Let the pending set mark rows for deletion

A marked row is one of the set's parts: it is counted, it holds its
page, and a save sends its DELETE. Its edits stay in the set and are not
the set's while it is marked. No cell of it is edited until it is
restored. Nothing marks a row yet.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: Review SQL shows the deletes

**Files:**
- Modify: `src/review.rs` (`Line`, `Part`, `Review`, `build`, `of`, a delete's statement in lines)
- Modify: `src/ui/review.rs` (`comment`, `comment_color`, `terminal_head`)
- Modify: `src/app/editing.rs` (`write_edits`: a refused delete)

- [ ] **Step 1: Write the failing tests**

In `src/review.rs`, in `mod tests` (the helper `sql(&review) -> Vec<String>` is there, near line 614):

```rust
    fn deleting(keys: Vec<Vec<(&str, Value)>>) -> ChangeSet {
        ChangeSet {
            object: ObjectRef::new("public", "book_covers"),
            inserts: Vec::new(),
            rows: Vec::new(),
            deletes: keys
                .into_iter()
                .map(|key| tabletist_db::RowDelete {
                    key: key
                        .into_iter()
                        .map(|(name, value)| (name.to_owned(), value))
                        .collect(),
                })
                .collect(),
        }
    }

    /// Spec 4, as the full-width board's card has it: a row to delete is
    /// its comment and its statement, on one line where its key is one
    /// column.
    #[test]
    fn a_row_to_delete_is_its_comment_and_its_statement() {
        let changes = deleting(vec![vec![("id", Value::Int(9))]]);
        let review = of(
            Dialect::Postgres,
            &changes,
            Blocked::default(),
            Values::Shown,
        );
        assert_eq!(review.lines[0], Line::Delete("id 9".into()));
        assert_eq!(
            sql(&review),
            [r#"DELETE FROM "public"."book_covers" WHERE "id" = 9;"#]
        );
        assert_eq!((review.deleted, review.changes, review.added), (1, 0, 0));
        assert_eq!(review.refused, None);
    }

    /// A key of several columns is a line for each after the first, as an
    /// `UPDATE`'s is.
    #[test]
    fn a_delete_by_a_key_of_two_columns_is_laid_out_in_lines() {
        let changes = deleting(vec![vec![
            ("book_id", Value::Int(101)),
            ("author", Value::Text("Ada Quill".into())),
        ]]);
        let review = of(Dialect::Sqlite, &changes, Blocked::default(), Values::Shown);
        assert_eq!(
            sql(&review),
            [
                r#"DELETE FROM "public"."book_covers""#,
                r#" WHERE "book_id" = 101"#,
                r#"   AND "author" = 'Ada Quill';"#,
            ]
        );
    }

    /// Spec 4: inserts, then updates, then deletes. And a delete the
    /// builder refuses is a comment, and the review's first refusal.
    #[test]
    fn the_deletes_come_last_and_a_refused_one_says_why() {
        let mut changes = deleting(vec![
            vec![("id", Value::Int(9))],
            vec![("id", Value::Text("a\u{FFFD}".into()))],
        ]);
        changes.inserts = vec![tabletist_db::RowInsert { set: Vec::new() }];
        let review = of(Dialect::Sqlite, &changes, Blocked::default(), Values::Shown);
        assert_eq!(review.lines[0], Line::New);
        assert!(matches!(review.lines.last(), Some(Line::DeleteRefused { .. })));
        assert!(matches!(review.refused, Some((Part::Delete(1), _))));
        assert_eq!(review.deleted, 2);
    }
```

In `src/ui/review.rs`, in the test `every_comment_reads_as_the_spec_writes_it` (near line 683), add:

```rust
        assert_eq!(
            comment(&Line::Delete("id 9".into()), locale).as_deref(),
            Some("-- delete row id 9")
        );
        assert_eq!(
            comment(
                &Line::DeleteRefused {
                    row: "id 9".into(),
                    reason: "no".into()
                },
                locale
            )
            .as_deref(),
            Some("-- delete row id 9 · cannot be sent: no")
        );
```

(`locale` is whatever that test calls its locale.)

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked --lib review::`
Expected: FAIL to compile: no variant `Line::Delete`.

- [ ] **Step 3: The lines**

In `src/review.rs`, `Line` gains two variants before `Sql`, `Part` one, `Review` a count (and its `Debug` prints it):

```rust
    /// `-- delete row id 9`: the row the `DELETE` under it removes.
    Delete(String),
    /// The builder refuses the row's `DELETE`, in its own words.
    DeleteRefused { row: String, reason: String },
```

```rust
    Delete(usize),
```

```rust
    /// How many rows the set deletes.
    pub deleted: usize,
```

`row_name` takes a key, so a delete names its row as a change does:

```rust
fn row_name(key: &[(String, Value)]) -> String {
    let parts: Vec<String> = key
        .iter()
```

(its body as it is, without `row.`), its caller in `of` passes `&row.key`, and its doc's "From the key the statement finds the row by" stays.

In `of`, after the loop over `changes.rows`:

```rust
    // The rows to delete, last, as a save deletes them last. No cell
    // holds one back: a marked row's cells are not the set's.
    for (index, row) in changes.deletes.iter().enumerate() {
        let name = row_name(&row.key);
        match dialect.delete_row(&changes.object, row) {
            Ok(deletion) => {
                lines.push(Line::Delete(name));
                lines.extend(delete_statement(&deletion, values));
            }
            Err(error) => {
                let reason = format::capped(&error.to_string()).into_owned();
                lines.push(Line::DeleteRefused {
                    row: name,
                    reason: format::escape_hidden(&reason).into_owned(),
                });
                refused.get_or_insert((Part::Delete(index), error));
            }
        }
    }
```

and the `Review` it returns gains `deleted: changes.deletes.len()`.

After `lay`:

```rust
/// A row's `DELETE` in lines: on one where its key is one column, as the
/// design writes it, and with a line for each further column, as an
/// `UPDATE`'s key is laid out.
fn delete_statement(deletion: &RowDeletion, values: Values) -> Vec<Line> {
    lay_delete(deletion, values).unwrap_or_else(|| {
        // Not the parts this was written for: the statement on one line.
        vec![Line::Sql(vec![
            plain(&deletion.shown, values),
            fixed(Ink::Plain, ";"),
        ])]
    })
}

fn lay_delete(deletion: &RowDeletion, values: Values) -> Option<Vec<Line>> {
    let RowDeletion { shown, parts, .. } = deletion;
    let last = parts.values.len().checked_sub(1)?;
    if parts.columns.len() != parts.values.len() {
        return None;
    }
    let table = shown
        .get(..parts.filter)?
        .strip_prefix("DELETE FROM")?
        .trim_end();
    let mut head = vec![fixed(Ink::Keyword, "DELETE FROM"), plain(table, values)];
    let mut lines = Vec::new();
    for (index, (column, range)) in parts.columns.iter().zip(&parts.values).enumerate() {
        // What stands before the value: its column and `=`.
        let lead = shown.get(*column..range.start)?;
        let literal = shown.get(range.clone())?;
        let mut pieces = match index {
            0 => vec![fixed(Ink::Plain, " "), fixed(Ink::Keyword, "WHERE")],
            _ => vec![fixed(Ink::Plain, "   "), fixed(Ink::Keyword, "AND")],
        };
        pieces.push(plain(&format!(" {lead}"), values));
        pieces.push(value(literal, values));
        if index == last {
            pieces.push(fixed(Ink::Plain, ";"));
        }
        if last == 0 {
            // One column: the whole statement on the table's line.
            head.append(&mut pieces);
        } else {
            lines.push(Line::Sql(pieces));
        }
    }
    lines.insert(0, Line::Sql(head));
    Some(lines)
}
```

Import `RowDeletion` from `tabletist_db` with the file's other imports.

`build` counts a marked row, and leaves its cells out where the set cannot be made:

```rust
    if cells.is_empty() && table.added.is_empty() && table.deleted.is_empty() {
        return None;
    }
```

In its `Unsendable` branch the `Review` gains `deleted: table.deleted.len()`. The `Review { .. }` built by hand in `src/edit.rs` (near line 4177) gains `deleted: 0`, and the test in `src/review.rs` that holds a review's printed form (`"Review { changes: 2, rows: 1, added: 0, lines: 7 }"`, near line 1466) gains the count where the `Debug` writes it: say so in the commit.

- [ ] **Step 4: The comments, and the head's count**

In `src/ui/review.rs`, `comment` gains two arms before `Line::Sql`:

```rust
        Line::Delete(row) => format!("-- {} {} {row}", say("delete"), say("row")),
        Line::DeleteRefused { row, reason } => format!(
            "-- {} {} {row} · {} {reason}",
            say("delete"),
            say("row"),
            say("cannot be sent:")
        ),
```

`comment_color` puts `Line::DeleteRefused { .. }` with the danger lines and `Line::Delete(_)` with the comments.

`terminal_head` takes the deletes with its counts. Its parameter becomes `(added, changes, rows, deleted): (usize, usize, usize, usize)`, its caller passes `review.deleted` (or the count it has beside the others), and the counts are:

```rust
    // The new rows, then the changes and their rows, then the rows to
    // delete, each where there is any: "pending · 1 new row · 2 changes
    // · 1 row · 1 to delete".
    let mut counts = vec![skin.say("pending")];
    if added > 0 {
        counts.push(look.label(&counted(locale, added, "new row", "new rows")));
    }
    if changes > 0 || (added == 0 && deleted == 0) {
        counts.push(look.label(&counted(locale, changes, "change", "changes")));
        counts.push(look.label(&counted(locale, rows, "row", "rows")));
    }
    if deleted > 0 {
        counts.push(format!("{deleted} {}", skin.say("to delete")));
    }
```

- [ ] **Step 5: A delete the builder refuses fails its row**

In `src/app/editing.rs`, `write_edits` matches the review's refused part. It gains:

```rust
                        Part::Delete(index) => {
                            let row = sent.deletes.get(index).copied();
                            object.edits.fail_delete(row, error);
                        }
```

- [ ] **Step 6: Run the tests**

Run: `~/.cargo/bin/cargo test --locked --lib review`
Expected: PASS. If the second test's first line differs in how the table is written, the builder is right: fix the expectation.

- [ ] **Step 7: Run the four checks**

Expected: all pass.

- [ ] **Step 8: Commit**

```bash
git add src && git commit -m "Show a row's DELETE in Review SQL

After the new rows and the changed ones, as a save runs them: the row by
its key under \"delete row\", and its statement. A delete the builder
refuses is a line that says why, and fails its row.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: The reducer marks, restores, saves and takes the rows out

**Files:**
- Modify: `src/model.rs` (two actions; `WritePrompt::deleted`)
- Modify: `src/app.rs` (dispatch; `RevertCell`)
- Modify: `src/app/editing.rs` (`delete_row`, `restore_row`, `dropped_under_a_prompt`, `unwritten`, `write_edits`, `written`)
- Create: `src/ui/delete_tests.rs`; Modify: `src/ui/mod.rs` (register it)

- [ ] **Step 1: Write the failing tests**

Create `src/ui/delete_tests.rs` with the head given under "Before you start", add it to `TESTS` in `tests/keys.rs`, and:

```rust
/// Spec 1 and test 1, by the action the keys and the button push: a mark,
/// and the mark taken back by the same action, by Restore and by the
/// action that puts a cell back. The cursor stays.
#[test]
fn a_row_is_marked_and_restored_three_ways() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    mark(&mut harness, tab, id, 1);
    assert!(edits(&harness, tab, id).deleted.contains_key(&1));
    assert_eq!(selected(&harness, tab, id), Some((1, 1)));
    harness.app.apply(Action::DeleteRow { tab, id });
    assert!(edits(&harness, tab, id).deleted.is_empty());
    mark(&mut harness, tab, id, 1);
    harness.app.apply(Action::RestoreRow { tab, id });
    assert!(edits(&harness, tab, id).deleted.is_empty());
    mark(&mut harness, tab, id, 1);
    let cell = None;
    harness.app.apply(Action::RevertCell { tab, id, cell });
    assert!(edits(&harness, tab, id).deleted.is_empty());
}

/// Spec 1, "New rows", and test 5: a new row is dropped, and nothing of
/// it is sent.
#[test]
fn deleting_a_new_row_drops_it() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    add(&mut harness, tab, id);
    assert_eq!(edits(&harness, tab, id).added.len(), 1);
    harness.app.apply(Action::DeleteRow { tab, id });
    let edits = edits(&harness, tab, id);
    assert!(edits.added.is_empty() && edits.deleted.is_empty());
    assert!(!edits.pending());
}

/// Spec 1, "Existing edits", and test 4: the save of a marked row with
/// edits is its DELETE alone, and the edits are back with the row.
#[test]
fn a_marked_row_with_edits_is_saved_as_its_delete_alone() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    set(&mut harness, tab, id, (1, 2), "audio");
    mark(&mut harness, tab, id, 1);
    harness.app.apply(Action::WriteEdits { tab, id });
    let changes = sent(&harness);
    assert!(changes.rows.is_empty() && changes.inserts.is_empty());
    assert_eq!(changes.deletes.len(), 1);
    assert_eq!(
        changes.deletes[0].key,
        [("id".to_owned(), tabletist_db::Value::Int(2))]
    );
}

#[test]
fn restoring_a_row_brings_its_edits_back() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    set(&mut harness, tab, id, (1, 2), "audio");
    mark(&mut harness, tab, id, 1);
    assert_eq!(edits(&harness, tab, id).counts().changes, 0);
    harness.app.apply(Action::RestoreRow { tab, id });
    assert_eq!(edits(&harness, tab, id).counts().changes, 1);
    harness.app.apply(Action::WriteEdits { tab, id });
    let changes = sent(&harness);
    assert!(changes.deletes.is_empty());
    assert_eq!(changes.rows.len(), 1);
}

/// Spec 3: no editor opens on a marked row, and it says why.
#[test]
fn a_marked_rows_cell_is_not_edited() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    mark(&mut harness, tab, id, 1);
    let cell = crate::model::CellPos { row: 1, col: 2 };
    let start = crate::model::EditStart::Replace("audio".into());
    harness.app.apply(Action::EditCell {
        tab,
        id,
        cell,
        start,
    });
    let edits = edits(&harness, tab, id);
    assert!(edits.editor.is_none());
    assert_eq!(edits.why, Some((cell, crate::edit::Lock::Deleting)));
}

/// Spec 2 and test 6: a row that cannot be deleted is not marked.
#[test]
fn a_row_that_cannot_be_deleted_is_not_marked() {
    for (kind, read_only) in [
        (tabletist_db::ObjectKind::Table, true),
        (tabletist_db::ObjectKind::View, false),
    ] {
        let (mut harness, tab, id) = covers_as(Look::macos(), kind, read_only);
        mark(&mut harness, tab, id, 1);
        assert!(edits(&harness, tab, id).deleted.is_empty());
    }
}

fn written(gone: Vec<usize>) -> Result<tabletist_db::WriteOutcome, tabletist_db::Error> {
    Ok(tabletist_db::WriteOutcome::Written {
        inserted: Vec::new(),
        rows: Vec::new(),
        gone,
        elapsed: std::time::Duration::from_millis(14),
    })
}

fn page_ids(harness: &Harness, tab: ConnTabId, id: TabId) -> Vec<tabletist_db::Value> {
    let workspace = harness.app.workspace(tab).unwrap();
    let page = workspace.object_tab(id).unwrap().page().unwrap();
    page.rows.iter().map(|row| row[0].clone()).collect()
}

/// Spec 5 and test 11: after the save the row is gone from the grid, the
/// count is one less, and the cursor is on the row that stood after it.
#[test]
fn a_saved_delete_takes_the_row_out_and_moves_the_cursor_on() {
    use tabletist_db::Value::Int;
    let (mut harness, tab, id) = covers_in(Look::macos());
    mark(&mut harness, tab, id, 1);
    harness.app.apply(Action::WriteEdits { tab, id });
    harness.answer_written(written(Vec::new()));
    harness.settle();
    assert_eq!(page_ids(&harness, tab, id), [Int(1), Int(3)]);
    // The row with id 3 stands where the deleted one stood.
    assert_eq!(selected(&harness, tab, id), Some((1, 1)));
    let edits = edits(&harness, tab, id);
    assert!(!edits.pending());
    assert_eq!(edits.saved.as_ref().map(|saved| saved.deleted), Some(1));
}

#[test]
fn deleting_the_last_row_leaves_the_cursor_on_the_one_before_it() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    mark(&mut harness, tab, id, 2);
    harness.app.apply(Action::WriteEdits { tab, id });
    harness.answer_written(written(Vec::new()));
    harness.settle();
    assert_eq!(selected(&harness, tab, id), Some((1, 1)));
}

/// Spec 4, "Conflict", and test 9: a row someone deleted first leaves the
/// grid with the rest, and is said by its key.
#[test]
fn a_row_already_deleted_is_said_and_leaves_with_the_rest() {
    use tabletist_db::Value::Int;
    let (mut harness, tab, id) = covers_in(Look::macos());
    mark(&mut harness, tab, id, 0);
    mark(&mut harness, tab, id, 1);
    harness.app.apply(Action::WriteEdits { tab, id });
    // The second of the set's deletes: the row with id 2.
    harness.answer_written(written(vec![1]));
    harness.settle();
    assert_eq!(page_ids(&harness, tab, id), [Int(3)]);
    let edits = edits(&harness, tab, id);
    assert_eq!(
        edits.note,
        Some(crate::edit::Note::AlreadyDeleted(vec!["id 2".into()]))
    );
    assert_eq!(edits.saved.as_ref().map(|saved| saved.deleted), Some(1));
}

/// Spec 5 and test 8, as the app sees it: a refused delete leaves the row
/// marked, with the database's error on it, and everything else pending.
#[test]
fn a_refused_delete_stays_marked_with_its_error() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    set(&mut harness, tab, id, (0, 2), "audio");
    mark(&mut harness, tab, id, 1);
    harness.app.apply(Action::WriteEdits { tab, id });
    let error = tabletist_db::Error::Query {
        code: Some("23503".into()),
        message: "update or delete on table \"book_covers\" violates foreign key constraint"
            .into(),
        detail: None,
        hint: None,
        named: Box::default(),
    };
    harness.answer_written(Ok(tabletist_db::WriteOutcome::FailedDelete {
        delete: 0,
        error: error.clone(),
    }));
    harness.settle();
    let edits = edits(&harness, tab, id);
    let failed = edits.deleted.get(&1).and_then(|doomed| doomed.failed.clone());
    assert_eq!(failed, Some(error));
    assert_eq!(edits.counts().changes, 1);
    assert_eq!(page_ids(&harness, tab, id).len(), 3);
}

/// Row v: the row panel shows a marked row as it loaded, with nothing of
/// its hidden edits.
#[test]
fn a_marked_rows_panel_shows_none_of_its_edits() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    set(&mut harness, tab, id, (1, 2), "audio");
    harness.settle();
    assert!(harness.painted.iter().any(|(text, _)| text == "audio"));
    mark(&mut harness, tab, id, 1);
    assert!(!harness.painted.iter().any(|(text, _)| text == "audio"));
}
```

In `src/ui/mod.rs`, beside `#[cfg(test)] mod duplicate_tests;`:

```rust
#[cfg(test)]
mod delete_tests;
```

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked --lib ui::delete_tests`
Expected: FAIL to compile: no variant `Action::DeleteRow`.

- [ ] **Step 3: The actions**

In `src/model.rs`, after `DropRow`:

```rust
    /// Delete the selected row: a new one is dropped with what was set in
    /// it, a row of the page is marked for deletion, and one that is
    /// marked is restored.
    DeleteRow {
        tab: ConnTabId,
        id: TabId,
    },
    /// Take the mark for deletion off the selected row.
    RestoreRow {
        tab: ConnTabId,
        id: TabId,
    },
```

`DropRow`'s doc keeps "A row of the page stays."

In `src/app.rs`, after the `DropRow` arm:

```rust
            Action::DeleteRow { tab, id } => self.delete_row(tab, id),
            Action::RestoreRow { tab, id } => self.restore_row(tab, id),
```

and `RevertCell` restores a marked row before it touches a cell (row a):

```rust
            Action::RevertCell { tab, id, cell } => {
                // Not under a save: its answer is put into this set.
                if let Some(object) = self.object_tab_mut(tab, id)
                    && let Some(cell) = cell.or(object.selection)
                    && object.edits.editor.is_none()
                    && object.edits.saving.is_none()
                {
                    // On a row that is marked for deletion the mark is
                    // what there is to take back: its cells are not the
                    // set's until it is.
                    if object.edits.deleted.contains_key(&cell.row) {
                        self.restore_row(tab, id);
                    } else {
                        object.edits.revert((cell.row, cell.col));
                        object.fields = None;
                    }
                }
            }
```

(If the borrow of `object` reaches into `self.restore_row`, read `marked` into a `bool` first and call after the `if let`.) `restore_row` acts on the selection, which is `cell` here whenever the keys ask: where `cell` is named and is not the selection, restore by the row: give `restore_row` the row (below).

In `src/app/editing.rs`, `dropped_under_a_prompt` lists the two beside `DropRow`:

```rust
            | Action::DeleteRow { .. }
            | Action::RestoreRow { .. }
```

After `drop_row`:

```rust
    /// Deletes the selected row: drops it where it is a new one, marks it
    /// for deletion where it is a row of the page, and restores it where
    /// it is marked. A row that cannot be deleted stays as it is: the
    /// keys and the button say why before they ask.
    pub(super) fn delete_row(&mut self, tab: ConnTabId, id: TabId) {
        let selected = self
            .workspace(tab)
            .and_then(|workspace| workspace.object_tab(id))
            .and_then(|object| object.selection);
        let Some(cell) = selected else {
            return;
        };
        if new_id(cell.row).is_some() {
            return self.drop_row(tab, id);
        }
        let marked = self
            .object_tab_mut(tab, id)
            .is_some_and(|object| object.edits.deleted.contains_key(&cell.row));
        if marked {
            return self.restore(tab, id, cell.row);
        }
        // The text being typed is part of the row's edits, which are kept
        // under the mark.
        self.close_editor(tab, id, true);
        let free = self.table(tab, id, |table, _| table.no_delete(cell.row).is_none());
        if free != Some(true) {
            return;
        }
        let Some(object) = self.object_tab_mut(tab, id) else {
            return;
        };
        if !object.edits.mark(cell.row) {
            return;
        }
        let row = crate::ui::pending_bar::row_name(object, cell.row);
        let deleted = object.edits.deleted.len();
        object.edits.told = Some(Told::Marked { row, deleted });
        // A tab that holds a change stays, as it does for a cell's.
        object.pinned = true;
        // What the last save said was of another set.
        object.edits.note = None;
        // The row panel shows the row as marked.
        object.fields = None;
    }

    /// Takes the mark for deletion off the selected row.
    pub(super) fn restore_row(&mut self, tab: ConnTabId, id: TabId) {
        let selected = self
            .workspace(tab)
            .and_then(|workspace| workspace.object_tab(id))
            .and_then(|object| object.selection);
        if let Some(cell) = selected {
            self.restore(tab, id, cell.row);
        }
    }

    /// Takes the mark for deletion off the page's row `row`.
    fn restore(&mut self, tab: ConnTabId, id: TabId, row: usize) {
        let Some(object) = self.object_tab_mut(tab, id) else {
            return;
        };
        // Not under a save: its answer names the rows it sent.
        if object.edits.saving.is_some() || !object.edits.restore(row) {
            return;
        }
        let row = crate::ui::pending_bar::row_name(object, row);
        object.edits.told = Some(Told::Restored { row });
        object.edits.note = None;
        object.fields = None;
    }
```

`RevertCell` then calls `self.restore(tab, id, cell.row)` (make it `pub(super)`). Import `Told` from `crate::edit`. `row_name` is `pub(crate)` in `src/ui/pending_bar.rs`; the reducer already words a column with `crate::ui::format`.

`unwritten` counts the marks: `(changes + counts.added + counts.deleted).max(1)`.

- [ ] **Step 4: The save's prompt**

`WritePrompt` (`src/model.rs`) gains, after `added`:

```rust
    /// How many rows the save deletes.
    pub deleted: usize,
```

and `write_edits` fills it: `deleted: changes.deletes.len()`. Its doc line "What the Omarchy box's field holds: `write` confirms." is stale since the insert run: it now reads "What the Omarchy box's field holds: the database's name confirms."

- [ ] **Step 5: The save's answer**

In `written` (`src/app/editing.rs`), the `Written` arm takes the list and the marks. Replace the arm's head and what follows up to `object.edits.discard();`:

```rust
            Ok(WriteOutcome::Written {
                inserted,
                rows,
                gone,
                elapsed,
            }) => {
                let counts = object.edits.counts();
                // The rows the save found already deleted, by their keys,
                // while the page still holds them.
                let already: Vec<String> = gone
                    .iter()
                    .filter_map(|&index| saving.deletes.get(index))
                    .map(|&row| crate::ui::pending_bar::row_name(object, row))
                    .collect();
                // The changed cells, by the page's rows they were of. A
                // marked row has none among them.
                let changed: Vec<CellPos> = object
                    .edits
                    .cells
                    .range(..(NEW_ROWS, 0))
                    .map(|(&(row, col), _)| CellPos { row, col })
                    .collect();
```

`fits` also asks that every deleted row is one of the page's:

```rust
                        && saving.deletes.iter().all(|&at| at < page.rows.len())
```

Inside `if let Some(page) = page`, the rows are laid out without the deleted ones. Replace from `let order = Order::of(&added, page.rows.len());` to the line that sets `object.selection`:

```rust
                    let order = Order::of(&added, page.rows.len());
                    let doomed: BTreeSet<usize> = saving.deletes.iter().copied().collect();
                    // The rows as they stand once the save is done, in the
                    // order they were shown, without the deleted ones: a
                    // row's number is where it was shown among them.
                    let kept: Vec<usize> = (0..order.len())
                        .filter_map(|place| order.row(place))
                        .filter(|row| !doomed.contains(row))
                        .collect();
                    let stored = inserted.into_iter().flatten();
                    let mut stored: BTreeMap<usize, Vec<tabletist_db::Value>> =
                        saving.inserts.iter().copied().zip(stored).collect();
                    let mut loaded: Vec<Option<Vec<tabletist_db::Value>>> =
                        std::mem::take(&mut page.rows)
                            .into_iter()
                            .map(Some)
                            .collect();
                    page.rows = kept
                        .iter()
                        .filter_map(|&row| match new_id(row) {
                            Some(id) => stored.remove(&id),
                            None => loaded.get_mut(row)?.take(),
                        })
                        .collect();
                    let width = page.columns.len();
                    let places: BTreeMap<usize, usize> = kept
                        .iter()
                        .enumerate()
                        .map(|(place, &row)| (row, place))
                        .collect();
                    let now = |row: usize| places.get(&row).copied();
```

(`cells`, the `for &id in &saving.inserts` loop, the sort and the `gone` remap stay as they are, on the new `now`. Import `BTreeMap` and `BTreeSet` where the file does not have them.) The selection moves on from a row that was deleted (row w):

```rust
                    object.selection = object.selection.and_then(|cell| {
                        let row = match now(cell.row) {
                            Some(row) => row,
                            // Its row was deleted: the row that stood
                            // after it, or the last where it was the last.
                            None => {
                                let stood = order.place(cell.row)?;
                                let before = kept
                                    .iter()
                                    .filter(|&&row| order.place(row).is_some_and(|at| at < stood))
                                    .count();
                                before.min(kept.len().checked_sub(1)?)
                            }
                        };
                        Some(CellPos { row, col: cell.col })
                    });
```

The table's count goes down by the rows that are no longer there, deleted now or found gone:

```rust
                    // The table has that many rows more and that many
                    // fewer, where it is known how many it had. A row
                    // found already deleted was counted when the page was
                    // read.
                    let more = saving.inserts.len() as u64;
                    let fewer = saving.deletes.len() as u64;
                    if let Some(count) = object.count.value.as_mut() {
                        *count = (*count + more).saturating_sub(fewer);
                    }
                    if let Some(estimate) = object.estimated_rows.as_mut() {
                        *estimate = (*estimate + more).saturating_sub(fewer);
                    }
```

`Saved` gains `deleted: saving.deletes.len().saturating_sub(already.len())`, and after it is set the rows that were gone are said:

```rust
                    if !already.is_empty() {
                        object.edits.note = Some(Note::AlreadyDeleted(already));
                    }
```

`object.edits.discard()` above drops the marks with the rest of the set, which is right: they were saved. The arm for `FailedDelete` becomes:

```rust
            Ok(WriteOutcome::FailedDelete { delete, error }) => {
                let row = saving.deletes.get(delete).copied();
                object.edits.fail_delete(row, error);
                // The row panel says what stands against the row.
                object.fields = None;
            }
```

- [ ] **Step 6: Run the tests**

Run: `~/.cargo/bin/cargo test --locked --lib ui::delete_tests`
Expected: PASS, eleven tests. `a_row_that_cannot_be_deleted_is_not_marked` passes for a view only if `mark` selects a cell there: `select` does. (`object.pinned = true` is how `set_null` and its neighbours keep a tab that gains a change: write it as they do, or leave it out if they do it another way.)

- [ ] **Step 7: Run the four checks, and the shots' clippy**

Expected: all pass.

- [ ] **Step 8: Commit**

```bash
git add src && git commit -m "Mark, restore and save a row to delete

One action deletes the selected row: a new row is dropped, a row of the
page is marked, a marked row is restored. Putting a cell back restores a
marked row too. A save sends the marked rows, takes them out of the page
when it is written, and moves the cursor to the row that stood after.
One that was already deleted leaves with them and is said. A delete the
database refuses stays marked, with its error.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: ⌫, `dd`, and why a row cannot be deleted

**Files:**
- Modify: `src/ui/keys.rs` (`editing_keys` near line 1282, `letters` near line 1892)
- Modify: `src/ui/cell_editor.rs` (`delete_text`)
- Modify: `src/ui/delete_tests.rs`; `src/ui/mod.rs` (two tests that said a page's row stays)

- [ ] **Step 1: Write the failing tests**

In `src/ui/delete_tests.rs` (add `painted`, `type_key`, `Key` and `Modifiers` to its imports):

```rust
/// Spec 1 and test 1, in the desktop looks: ⌫ and ⌦ mark the row, again
/// restore it, and the key that puts a cell back restores it too.
#[test]
fn backspace_marks_the_row_and_again_restores_it() {
    for look in desktop() {
        let (mut harness, tab, id) = covers_in(look);
        select(&mut harness, tab, id, (1, 2));
        harness.press(Key::Backspace, Modifiers::NONE);
        assert!(edits(&harness, tab, id).deleted.contains_key(&1));
        assert_eq!(selected(&harness, tab, id), Some((1, 2)));
        harness.press(Key::Backspace, Modifiers::NONE);
        assert!(edits(&harness, tab, id).deleted.is_empty());
        harness.press(Key::Delete, Modifiers::NONE);
        assert!(edits(&harness, tab, id).deleted.contains_key(&1));
        harness.press(Key::Z, Modifiers::COMMAND);
        assert!(edits(&harness, tab, id).deleted.is_empty());
    }
}

/// The same in the Omarchy look: `dd`, `dd` again, and `u`.
#[test]
fn dd_marks_the_row_and_again_restores_it() {
    let (mut harness, tab, id) = covers_in(Look::omarchy());
    select(&mut harness, tab, id, (1, 2));
    type_key(&mut harness, Key::D, "d");
    // One d waits for the other.
    assert!(edits(&harness, tab, id).deleted.is_empty());
    type_key(&mut harness, Key::D, "d");
    assert!(edits(&harness, tab, id).deleted.contains_key(&1));
    type_key(&mut harness, Key::D, "d");
    type_key(&mut harness, Key::D, "d");
    assert!(edits(&harness, tab, id).deleted.is_empty());
    type_key(&mut harness, Key::D, "d");
    type_key(&mut harness, Key::D, "d");
    type_key(&mut harness, Key::U, "u");
    assert!(edits(&harness, tab, id).deleted.is_empty());
}

/// Test 2: with a cell's editor open the key edits its text.
#[test]
fn backspace_in_an_open_editor_never_marks_the_row() {
    for look in desktop() {
        let (mut harness, tab, id) = covers_in(look);
        let cell = crate::model::CellPos { row: 1, col: 2 };
        let start = crate::model::EditStart::Replace("audio".into());
        harness.app.apply(Action::EditCell {
            tab,
            id,
            cell,
            start,
        });
        harness.settle();
        harness.press(Key::Backspace, Modifiers::NONE);
        let edits = edits(&harness, tab, id);
        assert!(edits.deleted.is_empty());
        assert!(edits.editor.is_some());
    }
}

/// Spec 2 and test 6: the key says why a row cannot be deleted, in the
/// spec's words, and marks nothing.
#[test]
fn the_key_says_why_a_row_cannot_be_deleted() {
    let cases = [
        (tabletist_db::ObjectKind::Table, true, "Read-only connection"),
        (
            tabletist_db::ObjectKind::View,
            false,
            "book_covers can't be deleted from",
        ),
    ];
    for (kind, read_only, reason) in cases {
        let (mut harness, tab, id) = covers_as(Look::macos(), kind, read_only);
        select(&mut harness, tab, id, (1, 2));
        harness.press(Key::Backspace, Modifiers::NONE);
        assert!(edits(&harness, tab, id).deleted.is_empty());
        assert_eq!(
            crate::ui::toast::said(&harness.ctx).as_deref(),
            Some(reason)
        );
    }
}

#[test]
fn a_table_without_a_key_says_its_rows_cannot_be_deleted_here() {
    let locale = crate::i18n::Locale::default();
    assert_eq!(
        crate::ui::cell_editor::delete_text(crate::edit::Lock::NoKey, "book_imports", locale),
        "book_imports has no primary key, so rows can't be deleted here"
    );
    // What holds for a while is said as an edit's reason is.
    assert_eq!(
        crate::ui::cell_editor::delete_text(crate::edit::Lock::Saving, "book_covers", locale),
        "A save is running"
    );
}

/// Spec 3: the Omarchy look says in its status line why a marked row's
/// cell takes no edit.
#[test]
fn a_marked_rows_cell_says_restore_the_row_to_edit_it() {
    let (mut harness, tab, id) = covers_in(Look::omarchy());
    mark(&mut harness, tab, id, 1);
    select(&mut harness, tab, id, (1, 2));
    type_key(&mut harness, Key::I, "i");
    assert!(edits(&harness, tab, id).editor.is_none());
    assert!(painted(&harness, "restore the row to edit it."));
}
```

`Locale::default()` stands for however the tests name the English locale (`harness.app.locale` works where a harness is at hand).

In `src/ui/mod.rs`, two tests hold what this task ends. `mod_n_adds_a_row_to_the_table_in_front_and_delete_drops_it` (near line 11893) ends by pressing Delete on a row of the page and asserting three rows: it now asserts that the page still has three rows and that the row is marked (`edits(..).deleted.len() == 1`). The test near line 12461, that `dd` on a field of a page's row drops nothing, keeps its assertion that no row is dropped and also asserts that none is marked: the keys are on a field, not on the rows. Say in the commit that the two expectations changed with the behaviour.

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked --lib ui::delete_tests`
Expected: the new tests FAIL: the keys push `DropRow`, which leaves a row of the page as it is.

- [ ] **Step 3: Delete's own words**

In `src/ui/cell_editor.rs`, after `lock_text`:

```rust
/// Why a row cannot be deleted, as Delete and its key say it. `table` is
/// the table's name as it is shown. What holds only for a while, or for
/// the one row, is said as an edit's reason is.
pub fn delete_text(lock: Lock, table: &str, locale: Locale) -> String {
    let say = |text: &'static str| gettext(locale, text).into_owned();
    match lock {
        Lock::ReadOnly => say("Read-only connection"),
        Lock::NotATable => format!("{table} {}", say("can't be deleted from")),
        Lock::NoKey => format!(
            "{table} {}",
            say("has no primary key, so rows can't be deleted here")
        ),
        Lock::StructureLoading
        | Lock::KeyType
        | Lock::Saving
        | Lock::Refreshing
        | Lock::NoSuchCell
        | Lock::Gone
        | Lock::Deleting
        | Lock::KeyIsNull
        | Lock::KeyInexact
        | Lock::UnknownColumn
        | Lock::Generated
        | Lock::Assigned
        | Lock::KeyColumn
        | Lock::Binary
        | Lock::TooLarge
        | Lock::NotNull => lock_text(lock, table, locale),
    }
}
```

- [ ] **Step 4: The keys**

In `src/ui/keys.rs`, both paths ask the table before they push. A helper beside `written`:

```rust
/// Asks for the selected row of the tab `id` to be deleted, or hands back
/// why it cannot be, for the caller to say: a toast in the desktop looks,
/// the status line's message in the Omarchy look.
fn delete_row(app: &App, tab: ConnTabId, id: TabId, actions: &mut Vec<Action>) -> Option<String> {
    let refused = app.workspace(tab).and_then(|workspace| {
        let object = workspace.object_tab(id)?;
        let row = object.selection?.row;
        let lock = crate::edit::Table::of(workspace, object)?.no_delete(row)?;
        let table = crate::ui::format::display_safe(&object.object.name);
        Some(crate::ui::cell_editor::delete_text(lock, &table, app.locale))
    });
    if refused.is_none() {
        actions.push(Action::DeleteRow { tab, id });
    }
    refused
}
```

(with the names the file uses for `App`, `ConnTabId` and `TabId`). The Omarchy look says the app's words in lower case and keeps a table's name as it is: word the reason there as `workspace::lock_line` does for a lock's (`src/ui/workspace.rs`, near line 1558), not with `look.label` over the whole sentence.

In `editing_keys` the keys are read inside `ctx.input_mut(|input| { .. })` (from near line 1233), which holds the context's lock: `toast::say` takes it again and would hang there. The reason is carried out of the closure, as `left` is (near lines 1232 and 1368), and said after it. The block that begins "The row's delete drops a new row" becomes:

```rust
        // The row's delete: a new row is dropped, a row of the page is
        // marked for deletion, or restored where it is marked. With the
        // keys on a field of the row panel nothing is deleted here.
        let deletes = on(input, Command::DeleteRow, Scope::Grid, true).count;
        if deletes > 0 && field.is_none() {
            refused = delete_row(app, tab, id, actions);
            return;
        }
```

with `let mut refused = None;` before the closure, and after it:

```rust
    if let Some(why) = refused {
        crate::ui::toast::say(ctx, &why);
    }
```

`on_new` is used nowhere after the block: it goes.

In `letters`, the arm becomes:

```rust
            // The row under the cursor is deleted: dropped where it is a
            // new one, marked or restored where it is the page's. With
            // the keys on a field, only a new row's form has the key.
            (Some(Command::DeleteRow), Some(id))
                if rows_show && !tree && (field.is_none() || on_new) =>
            {
                if let Some(why) = delete_row(app, tab, id, actions) {
                    crate::ui::toast::say(ctx, &why);
                }
            }
```

`letters` is under no lock of the context's. `on_new` there is the keymap's `When::NewRow`: unchanged.

`UndoCell` needs nothing: it pushes `RevertCell`, which restores a marked row since task 3. But `only_edits` (near line 1778) and the guards `edits && alone` decide where `u` is read: on a view or a read-only connection nothing is marked, so nothing is lost.

- [ ] **Step 5: Run the tests**

Run: `~/.cargo/bin/cargo test --locked --lib ui::delete_tests`
Expected: PASS. A test that hangs for ten seconds and panics with "Deadlock?" said the toast inside `input_mut`.

Run: `~/.cargo/bin/cargo test --locked --lib ui::tests::mod_n_adds`
Expected: PASS with its new expectation.

- [ ] **Step 6: Run the four checks**

Expected: all pass, `tests/keys.rs` among them: no key is spelled here.

- [ ] **Step 7: Commit**

```bash
git add src && git commit -m "Delete a row of the page with the row's delete key

Backspace and Delete in the desktop looks and dd in the Omarchy look
mark the row under the cursor for deletion, and restore it where it is
marked. They still drop a new row. Where the row cannot be deleted they
say why, in Delete's own words. Two tests that held a page's row to stay
as it was under the key now hold that it is marked, or that a field's
key marks nothing.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 5: How a marked row looks

**Files:**
- Modify: `src/env.rs` (`DeletedRow`, `deleted_row_colors`)
- Modify: `src/ui/grid.rs` (`Row`, `Mark`, the row's fill, bar, sign and label, the cells' colours, `paint_struck`)
- Modify: `src/ui/data_view.rs` (`Changes`, the row closure)
- Modify: `src/testing.rs` (`Harness::struck`)
- Modify: `src/ui/delete_tests.rs`

- [ ] **Step 1: Write the failing tests**

In `src/env.rs`, in `mod tests`:

```rust
    /// The provisional style of a row marked for deletion: the spec's
    /// three colours in the light, the palette's in the dark and in the
    /// terminal look.
    #[test]
    fn a_deleted_rows_colours_are_the_specs_in_the_light() {
        let light = deleted_row_colors(Platform::Native, &Palette::light());
        assert_eq!(light.fill, Color32::from_rgb(0xfd, 0xf6, 0xf5));
        assert_eq!(light.bar, Color32::from_rgb(0xc2, 0x26, 0x1f));
        assert_eq!(light.text, Color32::from_rgb(0x9a, 0x98, 0x92));
        let palette = Palette::dark();
        let dark = deleted_row_colors(Platform::Native, &palette);
        assert_eq!(dark.bar, palette.danger);
        assert_eq!(dark.text, palette.faint);
        assert_ne!(dark.fill, palette.window);
        let terminal = deleted_row_colors(Platform::Omarchy, &palette);
        assert_eq!(terminal.text, palette.dim);
    }
```

In `src/ui/delete_tests.rs`:

```rust
/// Spec 3 and 6: a marked row stays where it is, says that it is marked,
/// and its values are struck through. The desktop looks write `− delete`
/// where the row's key stands.
#[test]
fn a_marked_row_is_struck_through_and_says_so() {
    for look in desktop() {
        let (mut harness, tab, id) = covers_in(look);
        mark(&mut harness, tab, id, 1);
        assert!(harness.has("Row 2, marked for deletion"));
        assert!(harness.has("Row 1"));
        // The row's kind, and its NULL as text: no chip stands in a
        // marked row.
        assert!(harness.struck.iter().any(|text| text == "ebook"));
        assert!(harness.struck.iter().any(|text| text == "NULL"));
        assert!(painted(&harness, "\u{2212} delete"));
        // Another row's values are as they were.
        assert!(!harness.struck.iter().any(|text| text == "print"));
        // Restored, nothing is struck.
        harness.app.apply(Action::RestoreRow { tab, id });
        harness.settle();
        assert!(harness.struck.is_empty());
        assert!(harness.has("Row 2"));
    }
}

/// The Omarchy look keeps the key's value, struck, and marks the gutter.
#[test]
fn the_terminal_marks_a_deleted_row_in_its_gutter() {
    let (mut harness, tab, id) = covers_in(Look::omarchy());
    mark(&mut harness, tab, id, 1);
    assert!(painted(&harness, "-"));
    assert!(harness.struck.iter().any(|text| text == "2"));
    assert!(!painted(&harness, "\u{2212} delete"));
}

/// Row v: a marked row's edits are not drawn as pending.
#[test]
fn a_marked_rows_edit_is_drawn_as_the_row_is() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    set(&mut harness, tab, id, (1, 2), "audio");
    mark(&mut harness, tab, id, 1);
    // What the row loaded, struck: the edit is hidden.
    assert!(harness.struck.iter().any(|text| text == "ebook"));
    assert!(!painted(&harness, "audio"));
}
```

`harness.has(label)` reads the AccessKit tree; if the row's label is looked up with a role in this file's neighbours (`crate::testing::labels(&tree)`), use what they use.

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked --lib ui::delete_tests::a_marked_row_is_struck`
Expected: FAIL to compile: no field `struck` on `Harness`.

- [ ] **Step 3: The one style**

In `src/env.rs`, after `new_row_colors`:

```rust
/// A row marked for deletion, in the grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeletedRow {
    /// Behind the row, for as long as it is marked.
    pub fill: Color32,
    /// The bar at its left.
    pub bar: Color32,
    /// What its values are written and struck through in.
    pub text: Color32,
    /// What its marker (`− delete`) is written in.
    pub marker: Color32,
}

/// The colours of a row marked for deletion on `platform` with `palette`.
///
/// Provisional: no board of the design draws such a row in the desktop
/// looks. The light ones are the three its spec names from the design's
/// own (`docs/superpowers/specs/2026-10-12-deleting-rows-design.md`): the
/// fill of a field that failed, the production red, and the grey of what
/// is disabled. In the dark, and in the terminal look, the palette's red
/// is mixed into what the rows stand on, as a new row's green is. This is
/// the one place to change when a board exists.
pub fn deleted_row_colors(platform: Platform, palette: &Palette) -> DeletedRow {
    let (base, _, text) = native_table(Environment::Production);
    match platform {
        Platform::Native if !palette.dark => DeletedRow {
            fill: Color32::from_rgb(0xfd, 0xf6, 0xf5),
            bar: base,
            text: Color32::from_rgb(0x9a, 0x98, 0x92),
            marker: text,
        },
        Platform::Native => DeletedRow {
            fill: mix(palette.window, palette.danger, 0.10),
            bar: palette.danger,
            text: palette.faint,
            marker: palette.danger,
        },
        // As a new row's green: 14% of the theme's red.
        Platform::Omarchy => DeletedRow {
            fill: mix(palette.panel, palette.danger, 0.14),
            bar: palette.danger,
            text: palette.dim,
            marker: palette.dim,
        },
    }
}
```

- [ ] **Step 4: The grid**

In `src/ui/grid.rs`, `Row` says whether its row is marked, apart from its mark (a row whose delete failed is in trouble and still marked):

```rust
    /// The row is marked for deletion: a screen reader is told so.
    pub deleted: bool,
```

`Mark` gains two, after `Gone`:

```rust
    /// A cell of a row marked for deletion: its value, dimmed and struck
    /// through.
    Deleted,
    /// The cell that marks such a row in the desktop looks: its text in
    /// the row's tone.
    Deleting,
```

The row's label:

```rust
                let Row {
                    mark,
                    number,
                    deleted,
                } = rows(row);
                let label = match number {
                    Some(number) if deleted => format!("Row {number}, marked for deletion"),
                    Some(number) => format!("Row {number}"),
                    None => "New row, not saved".to_owned(),
                };
```

The row's tone and fill. `RowMark::Deleted` has the danger tone (task 1 gave it that), and the row keeps a fill of its own under the selection, as a new row does:

```rust
                let doomed = crate::env::deleted_row_colors(crate::env::Platform::of(look), palette);
                let fill = if new {
                    Some(fresh.fill)
                } else if deleted {
                    // As a new row: the row keeps its fill under the
                    // selection, and its cell alone says where the
                    // keyboard is.
                    Some(doomed.fill)
                } else if lit_row {
```

The gutter's sign in the terminal look, for the three marks a row of the page has and the new row's:

```rust
                                let sign = match mark {
                                    RowMark::Trouble => "!",
                                    RowMark::New => "+",
                                    RowMark::Deleted => "-",
                                    RowMark::None | RowMark::Changed => "~",
                                };
```

The cell's tint (the `tone` match): `Mark::Deleted | Mark::Deleting` join the arm that gives `None`. The desktop looks' bar at the row's left takes the style's colour:

```rust
                        let color = match row_tone {
                            Some(_) if new => Some(fresh.bar),
                            Some(_) if deleted => Some(doomed.bar),
                            Some(tone) => Some(tone.color(palette)),
                            None => (selected_row && !lit_row).then_some(palette.accent),
                        };
```

The text's colours, before the arm for `Mark::Added`:

```rust
                        _ if content.mark == Mark::Deleted => written_in(palette, doomed.text),
                        _ if content.mark == Mark::Deleting => written_in(palette, doomed.marker),
```

and the arm `(None, Some(tone)) if col == 0 && columns[col].key && !look.terminal`, which writes a changed row's key in the row's tone, does not reach a marked row: those two arms stand before it.

A struck value. After `paint_slanted`:

```rust
/// [`paint`], struck through: a value of a row marked for deletion.
#[allow(clippy::too_many_arguments)] // the text, its place, and its look
fn paint_struck(
    painter: &egui::Painter,
    ui: &Ui,
    role: TextRole,
    text: &str,
    color: egui::Color32,
    x: f32,
    y: f32,
    right: bool,
    look: &Look,
) {
    let laid = Text::new(look)
        .add_with(role, text, 0.0, |format| {
            format.color = color;
            format.strikethrough = Stroke::new(1.0, color);
        })
        .layout(ui.ctx());
    if right {
        laid.paint_right(painter, x, y);
    } else {
        laid.paint_left(painter, x, y);
    }
}
```

In `draw_cell`, the `Style::Plain` arm paints with it first:

```rust
            if content.mark == Mark::Deleted {
                paint_struck(&clip, ui, role, &shown, color, x, center, numeric, look);
            } else if content.slanted {
```

A marked row's cells are all `Style::Plain` and none is `null` (step 5 makes them so), so this is the one arm that strikes.

- [ ] **Step 5: The data view**

In `src/ui/data_view.rs`, `Changes` gains two fields, and `Changes::of` a parameter for the first. It is called near line 1404 with `&object.edits.deleted` after `&object.edits.gone`, and once more near line 2562, in a test, with an empty map:

```rust
    /// The page's rows marked for deletion.
    deleted: &'a BTreeMap<usize, crate::edit::Doomed>,
    /// The column a deleted row's marker stands in, in the desktop looks:
    /// the first of the row's key.
    delete_marker: Option<usize>,
```

Work the marker's column out beside `computed` and `unset` (near line 1322), where the function still has the workspace, and set it where `changes.marker` is set:

```rust
        // A deleted row's marker stands where the row's key begins.
        let delete_marker = crate::edit::Table::of(workspace, object)
            .and_then(|table| table.key())
            .and_then(|key| key.into_iter().min());
```

```rust
        changes.delete_marker = delete_marker;
```

Row z is decided before this step: whether the macOS look's key column grows to hold `− delete` while a row is marked, or the marker is a shorter word. Ask the user with the two as choices, and build what they pick. The test's expected marker follows.

In `Changes::mark`, after the `gone` test:

```rust
        // A row marked for deletion is that and nothing else: its pending
        // cells are hidden until it is restored. Every value is plain
        // text, so one way of striking it through serves every column.
        if self.deleted.contains_key(&row) {
            let marker = !look.terminal && self.delete_marker == Some(col);
            if marker {
                let delete = gettext(locale, "delete");
                cell.text = format!("\u{2212} {delete}").into();
            } else if cell.null {
                cell.text = "NULL".into();
            }
            cell.null = false;
            cell.slanted = false;
            cell.hint = None;
            cell.style = grid::Style::Plain;
            cell.mark = if marker { Mark::Deleting } else { Mark::Deleted };
            return;
        }
```

`cell.text` is whatever `Cell`'s text is (a `Cow<str>` or a `String`): assign in its type. The cell passed in is made of the loaded value: a marked row has no pending cell to make it of.

The row closure (near line 1496) asks the set about the mark:

```rust
                    None => {
                        let doomed = object.edits.deleted.get(&row);
                        grid::Row {
                            mark: match doomed {
                                Some(doomed) if doomed.failed.is_some() => RowMark::Trouble,
                                Some(_) => RowMark::Deleted,
                                None => crate::edit::row_mark(changes.cells, row),
                            },
                            number: Some(object.query.offset + row as u64 + 1),
                            deleted: doomed.is_some(),
                        }
                    }
```

and the new row's `grid::Row` gains `deleted: false`. Every other `grid::Row { .. }` (the grid's tests' `numbered`, `src/ui/sql_results.rs` if it builds one, `src/shots.rs`) gains the field or `..Default::default()`.

- [ ] **Step 6: What a test reads of a strike**

In `src/testing.rs`, beside `slanted`:

```rust
    /// The texts the last frame wrote struck through.
    pub struck: Vec<String>,
```

cleared and collected where `slanted` is, with:

```rust
/// The texts of `shape` that are written struck through, every piece of
/// them.
fn collect_struck(shape: &egui::Shape, into: &mut Vec<String>) {
    match shape {
        egui::Shape::Vec(shapes) => shapes.iter().for_each(|shape| collect_struck(shape, into)),
        egui::Shape::Text(text) => {
            let sections = &text.galley.job.sections;
            let struck = |section: &egui::text::LayoutSection| {
                section.format.strikethrough != egui::Stroke::NONE
            };
            if !sections.is_empty() && sections.iter().all(struck) {
                into.push(text.galley.text().to_owned());
            }
        }
        _ => {}
    }
}
```

This reads what was laid out, not a picture: as `slanted` does.

- [ ] **Step 7: Run the tests**

Run: `~/.cargo/bin/cargo test --locked --lib ui::delete_tests`, then `~/.cargo/bin/cargo test --locked --lib env::tests`, then `~/.cargo/bin/cargo test --locked --lib ui::grid`
Expected: PASS.

- [ ] **Step 8: Run the four checks, and the shots' clippy**

Expected: all pass.

- [ ] **Step 9: Commit**

```bash
git add src && git commit -m "Draw a row that is marked for deletion

Its values are dimmed and struck through, on a red tint of its own. The
desktop looks give it a bar and write \"− delete\" where its key
stands; the Omarchy look writes - in its gutter. A screen reader is told
the row is marked. The desktop looks' colours are provisional, and stand
in one function until the design draws the row.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 6: The counts

**Files:**
- Modify: `src/ui/data_view.rs` (`subtitle`)
- Modify: `src/ui/pending_bar.rs` (`counts_text`, `written_text`)
- Modify: `src/ui/workspace.rs` (`Editing`, `editing_status`, `status_line`, `command_line`)
- Modify: `src/ui/delete_tests.rs`; `src/ui/mod.rs` (the tests of the struck `dd delete`)

- [ ] **Step 1: Write the failing tests**

In `src/ui/delete_tests.rs`:

```rust
/// Spec 3: the row count and the pending bar's summary.
#[test]
fn a_marked_row_is_counted_where_the_pending_changes_are() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    set(&mut harness, tab, id, (0, 2), "audio");
    add(&mut harness, tab, id);
    mark(&mut harness, tab, id, 1);
    assert!(harness.has("3 rows + 1 new · 1 to delete · 5 columns · main"));
    assert!(harness.has("1 new row · 1 change in 1 row · 1 to delete"));
}

/// Row l: the Omarchy status line counts the rows to delete, and its key
/// is one of the keys that work.
#[test]
fn the_terminals_status_line_counts_the_rows_to_delete() {
    let (mut harness, tab, id) = covers_in(Look::omarchy());
    mark(&mut harness, tab, id, 1);
    // The row panel's footer names the key too: only the line's is asked.
    harness.app.workspace_mut(tab).unwrap().row_panel = false;
    harness.settle();
    assert!(painted(&harness, "-1 to delete"));
    let palette = harness.app.palette;
    // A hint is one piece, its key and its word, in the colour of a key
    // that works.
    let works = |(text, color): &(String, egui::Color32)| {
        text == "dd delete" && *color == palette.text
    };
    assert!(harness.painted.iter().any(works));
}

#[test]
fn a_save_that_deleted_says_how_many_rows() {
    let locale = crate::i18n::Locale::default();
    let saved = crate::edit::Saved {
        at: std::time::Instant::now(),
        cells: Vec::new(),
        changes: 0,
        rows: 0,
        added: 0,
        deleted: 2,
        elapsed: std::time::Duration::from_millis(14),
    };
    assert_eq!(
        crate::ui::pending_bar::written_text(&saved, locale),
        "written 2 rows deleted · 14 ms"
    );
}
```

The first test's two strings are the subtitle and the bar's counts as their views announce them whole (`data_view::paint_named`, `pending_bar::named`). If a view announces the parts apart, assert each part. `harness.app.palette` is however the harness holds its palette.

The tests in `src/ui/mod.rs` that hold `dd delete` to be struck out on the status line (near lines 16467, and the "strikes its own keys through" tests near 16329, 16438, 16461 and 20570, where they name the delete key) now hold what is left struck there: the key that adds a row where the table takes none, and the line that saves on a read-only connection. Where a test asserted `"dd delete"` painted and not in `palette.text`, it now asserts the opposite on a table that can be edited, and the old on a read-only one.

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked --lib ui::delete_tests`
Expected: the three FAIL.

- [ ] **Step 3: The subtitle**

In `subtitle` (`src/ui/data_view.rs`), after the rows and the new rows are pushed:

```rust
    // The rows to delete, beside them: "13 rows · 1 to delete".
    let deleted = object.edits.deleted.len();
    if deleted > 0 {
        parts.push(format!("{deleted} {}", gettext(locale, "to delete")));
    }
```

Its doc's example gains the part.

- [ ] **Step 4: The pending bar**

`counts_text` (`src/ui/pending_bar.rs`), after the changes:

```rust
    if counts.deleted > 0 {
        parts.push(format!("{} {}", counts.deleted, gettext(locale, "to delete")));
    }
```

and its doc's example becomes "1 new row · 3 changes in 2 rows · 1 to delete · 1 value names no row".

`written_text` says what was deleted, and says no changes where a save only deleted:

```rust
    // A save of new rows or of deleted rows alone changed none.
    if saved.changes > 0 || (saved.added == 0 && saved.deleted == 0) {
        parts.push(counted(locale, saved.changes, "change", "changes"));
        parts.push(counted(locale, saved.rows, "row", "rows"));
    }
    if saved.deleted > 0 {
        parts.push(counted(locale, saved.deleted, "row deleted", "rows deleted"));
    }
```

The bar's dot (near line 464) is green where the set only adds rows (`counts.added > 0 && counts.changes == 0`). A set that also deletes is not that: add `&& counts.deleted == 0`, so it has the warning's amber, as a set that changes has.

(Close the row panel in the test as the neighbouring status line tests do, if `row_panel` is not the field's name. One more key that works makes the status line longer: a test in `ui::tests` that holds what fits a width may move, and its commit says so.)

- [ ] **Step 5: The Omarchy status line**

In `src/ui/workspace.rs`, `Editing` gains two fields:

```rust
    /// The rows to delete: "-1 to delete".
    deleted: Option<String>,
    /// The row under the cursor can be deleted, dropped or restored.
    can_delete: bool,
```

`editing_status` fills them, after `added`:

```rust
    let deleted =
        (counts.deleted > 0).then(|| format!("-{} {}", counts.deleted, say("to delete")));
    let can_delete = data
        && object.selection.is_some_and(|cell| {
            crate::edit::Table::of(workspace, object)
                .is_some_and(|table| table.no_delete(cell.row).is_none())
        });
```

`status_line` and `command_line` paint the count after the pending ones, in the danger tone. In `status_line`'s `counts` closure and in the room it measures:

```rust
            let pending_room =
                room(&editing.added) + room(&editing.pending) + room(&editing.deleted);
```

```rust
                for (text, tone) in [
                    (&editing.added, states::Tone::Success),
                    (&editing.pending, states::Tone::Warning),
                    (&editing.deleted, states::Tone::Danger),
                    (&editing.errors, states::Tone::Danger),
                ] {
```

and the same pair in `command_line` (near line 1843). The key moves from the struck ones to the ones that work:

```rust
            // The row under the cursor is deleted, where it can be.
            let delete = gettext(locale, "delete");
            if editing.can_delete {
                table_hints.push((&delete_key, &*delete, true));
            }
```

(after the new row's hint), and among the disabled:

```rust
            if !editing.can_delete {
                disabled.push(format!("{delete_key} delete"));
            }
```

`disabled` can now be empty on a table that takes everything: the line `14.0 * (disabled.len() - 1) as f32` underflows on an empty list. Guard it: `disabled.len().saturating_sub(1)`, and skip the struck block where the list is empty.

The review's hint shows for a set that only deletes: `editing.pending.is_some() || editing.added.is_some() || editing.deleted.is_some()`.

The mode line's word for the key on the full-width board is `dd delete row`. The line already says `o new row` for the key that adds one and `dd delete` struck: keep `delete`, the word the line has, and say so in the pull request if the board's is wanted.

- [ ] **Step 6: Run the tests**

Run: `~/.cargo/bin/cargo test --locked --lib ui::delete_tests`, then `~/.cargo/bin/cargo test --locked --lib ui::tests`
Expected: PASS, the changed status line tests among them.

- [ ] **Step 7: Run the four checks, and the shots' clippy**

Expected: all pass.

- [ ] **Step 8: Commit**

```bash
git add src && git commit -m "Count the rows to delete with the pending changes

Under the table's name, in the pending bar, on the Omarchy status line
and in what a save says it wrote. The status line's dd is a key that
works where the row under the cursor can be deleted, and is struck out
where it cannot, as the key that adds a row is. The tests that held it
always struck now hold that.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 7: What the save says

**Files:**
- Modify: `src/ui/pending_bar.rs` (`note_said`)
- Modify: `src/ui/data_view.rs` (`error_line`)
- Modify: `src/ui/delete_tests.rs`

- [ ] **Step 1: Write the failing tests**

```rust
fn restrict() -> tabletist_db::Error {
    tabletist_db::Error::Query {
        code: Some("23503".into()),
        message: "update or delete on table \"publishers\" violates foreign key constraint \
                  \"book_covers_publisher_id_fkey\" on table \"book_covers\""
            .into(),
        detail: None,
        hint: None,
        named: Box::default(),
    }
}

/// Spec 5: the banner, then the database's words with the constraint's
/// name. The row is marked as in trouble, and stays marked.
#[test]
fn a_refused_delete_says_that_nothing_was_saved_and_why() {
    for look in desktop() {
        let (mut harness, tab, id) = covers_in(look);
        mark(&mut harness, tab, id, 1);
        harness.app.apply(Action::WriteEdits { tab, id });
        harness.answer_written(Ok(tabletist_db::WriteOutcome::FailedDelete {
            delete: 0,
            error: restrict(),
        }));
        harness.settle();
        assert!(harness.has(
            "Nothing was saved. 1 row failed, so the whole transaction rolled back. 23503 · \
             update or delete on table \"publishers\" violates foreign key constraint \
             \"book_covers_publisher_id_fkey\" on table \"book_covers\"."
        ));
        assert!(harness.has("Row 2, marked for deletion"));
    }
}

/// The Omarchy look says it on its error line, by the row's key.
#[test]
fn the_terminal_says_a_refused_delete_on_its_error_line() {
    let (mut harness, tab, id) = covers_in(Look::omarchy());
    mark(&mut harness, tab, id, 1);
    harness.app.apply(Action::WriteEdits { tab, id });
    harness.answer_written(Ok(tabletist_db::WriteOutcome::FailedDelete {
        delete: 0,
        error: restrict(),
    }));
    harness.settle();
    let line = harness
        .painted
        .iter()
        .find(|(text, _)| text.starts_with("! 2: "));
    assert!(line.is_some_and(|(text, _)| text.contains("23503")));
}

/// Spec 4 and test 9: the words for a row someone deleted first.
#[test]
fn a_row_already_deleted_is_said_by_its_key() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    mark(&mut harness, tab, id, 0);
    mark(&mut harness, tab, id, 1);
    harness.app.apply(Action::WriteEdits { tab, id });
    harness.answer_written(written(vec![1]));
    harness.settle();
    assert!(harness.has("Row id 2 was already deleted."));
}
```

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked --lib ui::delete_tests`
Expected: the three FAIL: the bar says the database's words and "Nothing was written.", the error line says nothing, and the note of a row already deleted is empty.

- [ ] **Step 3: The notes' words**

In `note_said` (`src/ui/pending_bar.rs`), the two arms task 1 left:

```rust
        // The spec's banner. A save stops at its first statement that
        // fails, so one row fails at a time.
        Note::FailedDelete { error, .. } => format!(
            "{} {}",
            say("Nothing was saved. 1 row failed, so the whole transaction rolled back."),
            database_said(error)
        ),
        Note::AlreadyDeleted(rows) => match rows.as_slice() {
            [row] => format!("{} {row} {}", say("Row"), say("was already deleted.")),
            rows => format!(
                "{} {} {}.",
                rows.len(),
                say("rows were already deleted:"),
                rows.join(", ")
            ),
        },
```

The bar shows while `edits.pending() || saving || note` (near line 234), so the second shows on a set that is empty, with the bar's own way of dismissing a note. Check that the bar draws no Save, Discard or Review beside a note alone: where it draws them from `counts`, they are already hidden or disabled on an empty set. If Save is drawn enabled there, disable it on `!edits.pending()`.

The Omarchy status line takes `edits.note` through the same words (`editing_status`, `said`): its tail "rolled back · cells stay pending in red" is right for `FailedDelete` and wrong for `AlreadyDeleted`, which wrote. In `editing_status` only `Note::Failed` has that tail (near line 1755), and every other note falls into a `_` arm with the failed mark in the danger tone. `AlreadyDeleted` is of a save that wrote: give it an arm of its own with the mark and the colour `written_text`'s line has (`WROTE`, `palette.secondary`), and `FailedDelete` the arm `Failed` has.

A refused delete is also said where the row is. In `Changes::mark` (task 5's block), a marked row whose delete failed gives each of its cells the database's words as its hint, as a failed cell has them: `cell.hint = doomed.failed.as_ref().map(..)`, worded as `mark_state` words a `State::Failed`. And task 9 puts them under the row panel's line.

- [ ] **Step 4: The terminal's error line**

`error_line` (`src/ui/data_view.rs`, near line 1190) finds the first cell that is not ready. Before that, the first marked row whose delete failed:

```rust
            // A row whose delete failed has no cell to say it: the row,
            // by its number, and the database's words.
            let failed = object
                .edits
                .deleted
                .iter()
                .find_map(|(&row, doomed)| Some((row, doomed.failed.as_ref()?)));
            if let Some((row, error)) = failed {
                let number = object.query.offset + row as u64 + 1;
                return Some(format!("! {number}:  {}", said(error)));
            }
```

where `said` is how the function words a `State::Failed(error)` for a cell (the code and the message). Shape the line as the function shapes a cell's (`"! {row}:{column}  {message}"`), with no column.

- [ ] **Step 5: Run the tests**

Run: `~/.cargo/bin/cargo test --locked --lib ui::delete_tests`
Expected: PASS. Where a look says the note through `look.label` (lower case), the desktop looks do not: the tests above are of them.

- [ ] **Step 6: Run the four checks**

Expected: all pass.

- [ ] **Step 7: Commit**

```bash
git add src && git commit -m "Say what a save that deletes came to

A delete the database refuses: that nothing was saved and the whole
transaction rolled back, then the database's words. A row that was
already deleted: which, by its key, after a save that wrote the rest.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 8: The production confirm counts the deletes

**Files:**
- Modify: `src/ui/write_prompts.rs` (`confirm_write`, near line 516)
- Modify: `src/ui/delete_tests.rs`

- [ ] **Step 1: Write the failing test**

```rust
/// Spec 4, "Production confirm", as row m has it.
#[test]
fn the_production_confirm_counts_the_rows_to_delete() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    harness.app.workspace_mut(tab).unwrap().environment = crate::env::Environment::Production;
    set(&mut harness, tab, id, (0, 2), "audio");
    // A new row needs its publisher before a save is offered.
    let new = add(&mut harness, tab, id);
    let publisher = (crate::edit::new_row(new), 1);
    set(&mut harness, tab, id, publisher, "9100000000000000004");
    mark(&mut harness, tab, id, 1);
    harness.app.apply(Action::WriteEdits { tab, id });
    harness.settle();
    assert!(harness.has("Save 1 new row, 1 change and 1 deleted row to production?"));
}
```

Set the environment as the file's neighbours do (search `Environment::Production` in `src/ui/insert_audit_tests.rs`).

- [ ] **Step 2: Run it to see it fail**

Expected: FAIL: the sentence has no deleted row.

- [ ] **Step 3: The sentence**

Replace `what` and `facts.rows`:

```rust
    // What the save comes to: "1 new row, 2 changes and 1 deleted row".
    let mut parts = Vec::new();
    if prompt.added > 0 {
        parts.push(counted(locale, prompt.added, "new row", "new rows"));
    }
    if prompt.changes > 0 || (prompt.added == 0 && prompt.deleted == 0) {
        parts.push(counted(locale, prompt.changes, "change", "changes"));
    }
    if prompt.deleted > 0 {
        parts.push(counted(
            locale,
            prompt.deleted,
            "deleted row",
            "deleted rows",
        ));
    }
    let what = match parts.as_slice() {
        [] => String::new(),
        [one] => one.clone(),
        [first @ .., last] => format!("{} {} {last}", first.join(", "), gettext(locale, "and")),
    };
```

```rust
        // A save that only adds or deletes rows changes none of the
        // table's.
        rows: if prompt.rows == 0 && (prompt.added > 0 || prompt.deleted > 0) {
            table.clone()
        } else {
```

The Omarchy box builds its title from the same `facts.changes` ("write 1 new row, 1 change and 1 deleted row?"), and is confirmed by typing the database's name, as before.

- [ ] **Step 4: Run the test, then the four checks**

Expected: PASS, and all four pass.

- [ ] **Step 5: Commit**

```bash
git add src && git commit -m "Count the rows to delete in the production confirm

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 9: Delete and Restore in the row panel

**Files:**
- Modify: `src/ui/row_panel.rs` (the line under the header near line 874, `editing_footer` near line 3081 and its caller near line 955)
- Modify: `src/ui/delete_tests.rs`; `src/ui/duplicate_tests.rs` and `src/ui/mod.rs` (the tests that said Delete waits)

- [ ] **Step 1: Write the failing tests**

In `src/ui/delete_tests.rs`. The row panel is open once a row is selected: `open_panel` below is nothing more than `harness.settle()`, and can go. `duplicate_tests.rs` has `control` and `button` (near line 698), private and taking `&mut Harness`: make them `pub(super)`, import them, and call them as they are written (`button(&mut harness, ..)`):

```rust
/// Spec 1 and 6: the panel's Delete marks the row, by its plain name.
#[test]
fn the_panels_delete_marks_the_row() {
    for look in desktop() {
        let (mut harness, tab, id) = covers_in(look);
        select(&mut harness, tab, id, (1, 2));
        open_panel(&mut harness, tab);
        assert_eq!(button(&harness, "Delete"), (true, None));
        harness.click("Delete");
        harness.settle();
        assert!(edits(&harness, tab, id).deleted.contains_key(&1));
    }
}

/// Spec 3: the line at the panel's top, its Restore, and fields that are
/// read.
#[test]
fn a_marked_rows_panel_says_so_and_restores_it() {
    for look in desktop() {
        let (mut harness, tab, id) = covers_in(look);
        mark(&mut harness, tab, id, 1);
        open_panel(&mut harness, tab);
        assert!(harness.has("Will be deleted on save"));
        assert_eq!(button(&harness, "Restore"), (true, None));
        // Delete waits, and says the line's words.
        assert_eq!(
            button(&harness, "Delete"),
            (false, Some("Will be deleted on save".into()))
        );
        harness.click("Restore");
        harness.settle();
        assert!(edits(&harness, tab, id).deleted.is_empty());
        assert!(!harness.has("Will be deleted on save"));
    }
}

/// Spec 2 and 6: where the row cannot be deleted, Delete is disabled with
/// the reason as its description.
#[test]
fn the_panels_delete_says_why_it_cannot_be_pressed() {
    let read_only = covers_as(Look::macos(), tabletist_db::ObjectKind::Table, true);
    let (mut harness, tab, id) = read_only;
    select(&mut harness, tab, id, (1, 2));
    open_panel(&mut harness, tab);
    assert_eq!(
        button(&harness, "Delete"),
        (false, Some("Read-only connection".into()))
    );
}

/// The board's hint under the buttons names the two keys.
#[test]
fn the_panels_hint_names_the_keys_that_copy_and_delete() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    select(&mut harness, tab, id, (1, 2));
    open_panel(&mut harness, tab);
    assert!(harness.has("Click a value or press ⌘I to edit · ⌘D · ⌫"));
}
```

The hint's keys are as the keymap writes them for the look and the platform the tests run on: build the expected string from `crate::ui::keys::written(&harness.ctx, &look, Command::..)` for the three keys, not from a literal.

Tests that end here: in `src/ui/duplicate_tests.rs`, `the_panels_duplicate_makes_a_copy_and_delete_waits` (near line 715) and the assertion near line 779 that `dd delete` says "deleting rows arrives in a later version"; `a_row_that_cannot_be_edited_can_still_be_copied` (near line 750), which holds Delete's reason on a table without a key to be `lock_text`'s sentence, now holds `delete_text`'s ("has no primary key, so rows can't be deleted here"); in `src/ui/mod.rs`, `the_row_panels_duplicate_can_be_pressed_and_delete_waits` (near line 19160), and `the_row_panel_says_how_a_value_is_edited` (near line 23951), which holds the hint whole and now holds the longer one. Each keeps what it holds of Duplicate and now holds that Delete can be pressed, or is removed where the tests above hold all it held. Rename a test whose name says Delete waits. Say in the commit which changed and why.

- [ ] **Step 2: Run them to see them fail**

Expected: FAIL: Delete is disabled, and there is no Restore.

- [ ] **Step 3: The footer**

`editing_footer` answers for both buttons and takes Delete's reason apart from the row's. Its signature and its first lines:

```rust
/// Says whether a copy was asked for, and whether the row's delete was.
fn editing_footer(
    ui: &mut egui::Ui,
    rect: Rect,
    (locked, no_copy, no_delete): (Option<&str>, Option<&str>, Option<&str>),
    look: &Look,
    palette: &Palette,
    locale: crate::i18n::Locale,
) -> (bool, bool) {
```

Remove `later` and the old `no_delete`. In the terminal branch the second cell's reason is `no_delete`; in the desktop branch the second label's is, and both buttons are read:

```rust
            let clicked = button.show_at(ui, place, look, palette).clicked();
            copy |= clicked && index == 0;
            delete |= clicked && index == 1;
```

(with `let mut delete = false;` beside `copy`, and `(copy, delete)` returned at both ends). The terminal branch's cells are text, not buttons: nothing is clicked there, and the key does it.

The hint under the buttons names the two keys, as the board has it:

```rust
    let key = |command| crate::ui::keys::written(ui.ctx(), look, command);
    let click = gettext(locale, "Click a value or press");
    let edit = gettext(locale, "to edit");
    let said = format!(
        "{click} {} {edit} · {} · {}",
        key(Command::OpenInspector),
        key(Command::DuplicateRow),
        key(Command::DeleteRow)
    );
```

The caller (near line 955) works out why the row cannot be deleted, in Delete's words, and pushes the action:

```rust
                // Why the row cannot be deleted, in Delete's own words:
                // what keeps it from being edited, or that it is marked
                // already.
                let marked = form.locked == Some(Lock::Deleting);
                let no_delete = object.and_then(|object| {
                    if marked {
                        return Some(look.label(&gettext(locale, "Will be deleted on save")));
                    }
                    let lock = Table::of(workspace, object)?.no_delete(selected.row)?;
                    let table = format::display_safe(&object.object.name);
                    let said = crate::ui::cell_editor::delete_text(lock, &table, locale);
                    Some(look.label(&said))
                });
                let why = (locked.as_deref(), no_copy.as_deref(), no_delete.as_deref());
                let (copy, delete) = editing_footer(ui, foot, why, &look, &palette, locale);
                if copy {
                    actions.push(Action::DuplicateRow { tab, id });
                }
                if delete {
                    actions.push(Action::DeleteRow { tab, id });
                }
```

(`selected` is the panel's selected cell, by the name it has there.) The footer's height depends on `locked.is_some()`: a marked row is locked, so it has the short footer without the hint, which is right.

- [ ] **Step 4: The line under the header**

`locked` (near line 523) words the row's lock. A marked row's reads as the spec has it:

```rust
                if lock == Lock::Deleting {
                    return look.label(&gettext(locale, "Will be deleted on save"));
                }
```

In the strip that draws it (near line 874), a marked row has no lock icon and has Restore at the right:

```rust
                let marked = form.locked == Some(Lock::Deleting);
```

- Skip the `Icon::Lock` where `marked`.
- In the desktop looks, where `marked`, a button at the strip's right end, as wide as its word and 22 high, and the text's `room` ends 8 before it:

```rust
                if marked && !look.terminal {
                    let word = gettext(locale, "Restore");
                    let width = widgets::body(&look).width(ui.ctx(), look.faces, &word) + 20.0;
                    let place = Rect::from_center_size(
                        pos2(strip.right() - side - width / 2.0, y),
                        vec2(width, 22.0),
                    );
                    let button = widgets::ButtonSpec::new(&word).padding(0.0);
                    if button.show_at(ui, place, &look, &palette).clicked() {
                        actions.push(Action::RestoreRow { tab, id });
                    }
                }
```

  Take the button's size and its `ButtonSpec` from a small button the panel already draws (a field's "Set NULL", if it has one) so it matches the Components board's small button: check `MacComponents` before choosing.
- In the Omarchy look, where `marked`, the line ends with the key: `format!("{reason} · {} {}", key(Command::DeleteRow), say("restore"))`, in the look's lower case.

The strip's text colour for a marked row is the danger colour (`palette.danger`), not `palette.dim`: the line says something will be lost.

Where the row's delete failed in the last save (`object.edits.deleted[row].failed`), the database's words stand under the line, as a copy's line stands there: `top = form_note(ui, full, top, &said, &look, &palette)` with `said` the code and the message as the pending bar's `database_said` words them (make that function `pub(crate)`). A test: after a `FailedDelete`, the panel has the message.

- [ ] **Step 5: Run the tests**

Run: `~/.cargo/bin/cargo test --locked --lib ui::delete_tests`, then `--lib ui::duplicate_tests`, then `--lib ui::tests::the_row_panels`
Expected: PASS.

- [ ] **Step 6: Run the four checks**

Expected: all pass, `tests/keys.rs` among them.

- [ ] **Step 7: Commit**

```bash
git add src && git commit -m "Delete and restore a row from the row panel

Delete marks the row the panel shows, and says why where it cannot. A
marked row's panel says \"Will be deleted on save\" at its top, with
Restore, and reads its fields. The hint under the buttons names the keys
that copy and delete. The tests that held Delete to wait for a later
version now hold that it works.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 10: What a screen reader is told

**Files:**
- Modify: `src/ui/data_view.rs` (beside the `picked` announcement, near line 1543)
- Modify: `src/ui/delete_tests.rs`

- [ ] **Step 1: Write the failing test**

```rust
/// Spec 6 and test 12: the three announcements, word for word, each a
/// polite live region.
#[test]
fn a_screen_reader_is_told_of_a_mark_a_restore_and_a_save() {
    use egui::accesskit::{Live, Role};
    let (mut harness, tab, id) = covers_in(Look::macos());
    let polite = |harness: &mut Harness, words: &str| {
        // As `key_picker_tests.rs` reads a live node (near line 1777):
        // `crate::testing::node` hands back an id, not the node.
        let tree = harness.frame(Vec::new());
        tree.nodes.iter().any(|(_, node)| {
            node.label() == Some(words)
                && node.role() == Role::Label
                && node.live() == Some(Live::Polite)
        })
    };
    mark(&mut harness, tab, id, 1);
    assert!(polite(&mut harness, "Row id 2 marked for deletion. 1 to delete."));
    harness.app.apply(Action::RestoreRow { tab, id });
    harness.settle();
    assert!(polite(&mut harness, "Row id 2 restored."));
    mark(&mut harness, tab, id, 1);
    harness.app.apply(Action::WriteEdits { tab, id });
    harness.answer_written(written(Vec::new()));
    harness.settle();
    assert!(polite(&mut harness, "1 row deleted."));
}
```

Read a live node as `src/ui/key_picker_tests.rs` does (near line 1781): the role and the way to the node are that test's.

- [ ] **Step 2: Run it to see it fail**

Expected: FAIL: nothing is announced.

- [ ] **Step 3: The announcements**

In `src/ui/data_view.rs`, where the picker's choice is told (`if let Some(words) = &picked`), a second point of the grid tells the row's:

```rust
        // The last row that was marked or restored, or what the last save
        // deleted, on a point of its own: two live nodes do not share one.
        let told = match (&object.edits.told, &object.edits.saved) {
            (Some(Told::Marked { row, deleted }), _) => Some(format!(
                "{} {row} {} {deleted} {}",
                gettext(locale, "Row"),
                gettext(locale, "marked for deletion."),
                gettext(locale, "to delete.")
            )),
            (Some(Told::Restored { row }), _) => Some(format!(
                "{} {row} {}",
                gettext(locale, "Row"),
                gettext(locale, "restored.")
            )),
            (None, Some(saved)) if saved.deleted > 0 => Some(format!(
                "{}.",
                super::pending_bar::counted(locale, saved.deleted, "row deleted", "rows deleted")
            )),
            (None, _) => None,
        };
        if let Some(words) = &told {
            let at = Rect::from_min_size(ui.max_rect().min + vec2(2.0, 0.0), vec2(1.0, 1.0));
            widgets::announce_polite(ui, at, words);
        }
```

`object.edits.told` is dropped with the set when a save writes (`Edits::discard`), so the save's announcement follows the mark's. The node's id comes from its words, so it is read again only when they change.

- [ ] **Step 4: Run the test, then the four checks**

Expected: PASS, and all four pass.

- [ ] **Step 5: Commit**

```bash
git add src && git commit -m "Tell a screen reader of a row marked, restored and deleted

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 11: A scene to look at

No test compares a picture. This scene is for the eye, in both looks, light and dark, and its pictures stay in `target/shots/`.

**Files:**
- Modify: `src/shots.rs` (`EDITING`, one scene)

- [ ] **Step 1: The scene**

`EDITING` is `[(&str, Scene); 16]`: it becomes 17, with `("edit-delete-row", edit_delete_row)` after `("edit-pending", edit_pending)`, and:

```rust
/// A row marked for deletion beside a changed one: the row's own fill and
/// bar, its values struck through, `− delete` where its key stands (the
/// terminal's `-` in the gutter), and the bar that counts it.
fn edit_delete_row(harness: &mut Harness) {
    let (tab, id) = editable(harness);
    retype(harness, tab, id, (0, 2), "audio");
    let cell = crate::model::CellPos { row: 2, col: 1 };
    harness.app.apply(Action::SelectCell { tab, id, cell });
    harness.app.apply(Action::DeleteRow { tab, id });
    step_aside(harness, tab, id);
}
```

`editable`, `retype` and `step_aside` are the file's (near lines 337, 347 and 373): use them as `edit_pending` does, on the Bookshop table that scene uses.

- [ ] **Step 2: Render and look**

Run: `~/.cargo/bin/cargo test --locked --features shots --lib shots -- --ignored`
(It renders on the GPU without a display.) Open `target/shots/edit-delete-row-{macos,standard,omarchy}-{light,dark}.png` with the Read tool and compare by eye with:

- the spec's provisional style for the desktop looks (the fill, the 3 px bar, the grey struck values, `− delete` where `+ new` would stand);
- the `OmarchyWideEdit` board for the terminal look (the red tint, struck values, `-` in the gutter).

Fix what is off in the code and render again. Note in the task's report what was looked at. Do not commit a picture, and do not attach one to the pull request unblurred.

- [ ] **Step 3: Run the four checks, and the shots' clippy**

Expected: all pass.

- [ ] **Step 4: Commit**

```bash
git add src/shots.rs && git commit -m "Add a scene of a row marked for deletion

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 12: The website, and the specs that said deleting waits

**Files:**
- Modify: `docs/_guide/editing-data.md`, `docs/_guide/macos.md`, `docs/_guide/omarchy.md`, `docs/_reference/keyboard-shortcuts.md`
- Modify: `docs/superpowers/specs/2026-10-12-deleting-rows-design.md` ("As built"), and one line each in `2026-10-08-keyboard-shortcuts-design.md` (line 89), `2026-10-10-row-duplication-design.md` (lines 69 and 302) and `2026-10-03-value-editing-core-design.md` (line 1342)

- [ ] **Step 1: The guide**

`docs/_guide/editing-data.md` says near line 12 that deleting rows is not built, and names Delete as waiting near lines 178 and 245. Read the page whole first, and write in its voice. It gains a section after the one on new rows:

```markdown
## Delete a row

Select a row and press Backspace (Delete works too), or press **Delete** in
the row panel. In the Omarchy look the key is `dd`. The row is marked: it
stays where it is, struck through on a red tint, until you save. Nothing has
reached the database yet.

Press the key again on a marked row to restore it, or press **Restore** in
the row panel, or undo (Cmd+Z, `u`). Edits you had made to the row are kept
while it is marked and come back with it. They are not saved while it is.

A new row that is not saved yet is dropped by the same key, and leaves
nothing to save.

Save writes the deletes with everything else that is pending, in one
transaction: new rows first, then changed rows, then deleted rows. Review SQL
shows each row's `DELETE` before you save.

- If someone else deleted the row first, the save still goes through, and a
  line says which row was already gone.
- If the database refuses a delete (a foreign key still points at the row,
  most often), nothing is saved. The row stays marked and shows the
  database's message.

Rows cannot be deleted on a read-only connection, in a view, or in a table
without a primary key or unique index. The key and the button say why.
One row is marked at a time.
```

Remove the sentences that say deleting is not built, and correct the two that name Delete as waiting.

- [ ] **Step 2: The keys**

`BINDINGS` did not change: Delete row is still ⌫ and `dd` in the grid. `docs/_reference/keyboard-shortcuts.md`, `docs/_guide/macos.md` and `docs/_guide/omarchy.md` list it as "Delete row" with "(drops a new row)" or the like: the words in the row now read "Delete row (marks it; drops a new row)", in each page's own form.

- [ ] **Step 3: Build and check the site**

```bash
cd docs && bin/check
```

Expected: the build and its checks pass. (If the gems are not installed in the session, say so in the report: `docs.yml` builds the site on the pull request.)

- [ ] **Step 4: The specs**

In `docs/superpowers/specs/2026-10-12-deleting-rows-design.md`, after "PR description", an "As built" section: the table of this plan ("What gets built where run 1's table left a choice") with run 1's rows e, f, g, o and q, each as one line, and what is left for runs 3 and 4.

The three older specs each say in one place that deleting waits or that the key only drops a new row. Add to each line "(built on 2026-10-12: see `2026-10-12-deleting-rows-design.md`)", and change nothing else in them.

- [ ] **Step 5: Run the four checks**

Expected: all pass (nothing in `src` changed).

- [ ] **Step 6: Commit**

```bash
git add docs && git commit -m "Say on the website how a row is deleted

And in the specs that deleting rows is built, with what was built
otherwise than the spec asks.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

## After the last task

- Run the four checks and the shots' clippy on the branch's head, and `cd docs && bin/check`.
- Mark this plan built, as run 1's is: a "Status" paragraph under its head with what was built otherwise than written.
- Push, and open the pull request (the user asked for one for the feature). Its description has what the spec asks for: a summary; the board list; the provisional macOS style, and that it stands in `env::deleted_row_colors`; the keys (⌫, ⌦, `dd`, ⌘Z, `u`); the test list by the spec's numbers (1, 2, 4, 5, 6 in part, 7, 8, 9, 11, 12 here and in run 1; 3 and the batches of 7 are run 4's, 10 and the count message of 8 run 3's); the defaults kept (toggle; the 1,000 cap and the cascade counts not built yet); and the rows of the two plans' tables, as "what was built otherwise". No screenshot unless it is blurred. It ends with:

      🤖 Generated with [Claude Code](https://claude.com/claude-code)

- After it is open, use the pull request tools to bind it and read its CI. PostgreSQL's and MySQL's delete tests run there for the first time on a pull request.
- Offer run 3's plan.
