# Foreign Key Editing, Run 4: A Key That Names No Row Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A foreign key cell whose typed value names no row of the referenced table says so before the save, in the grid, under the row panel's field and in the pending bar, and holds nothing: Save stays offered.

**Architecture:** When an editor closes on a foreign key cell with a typed value that was not taken from the picker's list, the reducer sends one pinned `Lookup` for that value and notes the question on the tab (`Edits::doubts`, by cell, with the text asked about). The answer "no row" turns the note into a warning. A warning counts only while the cell still holds that pending text, so changing, reverting or saving the value ends it with no bookkeeping. `key_picker::unmatched` lists the warnings that stand, and the views word them.

**Tech Stack:** Rust 1.98, egui (the crmne fork), AccessKit for the headless tests. Spec: `docs/superpowers/specs/2026-10-10-foreign-key-editing-design.md` (section 7, test 21). Runs 1 to 3 must be merged first.

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

- **What was run where this plan was written.** Nothing. The code blocks were written from reading the tree at `6e04567`, before runs 1 to 3, and from those runs' plans. They were not compiled. Where a block and the compiler disagree, the compiler is right: keep the block's behaviour and its tests, and fix its spelling. Where an earlier run was built otherwise than its plan says, follow what was built.
- House rules: views push `Action`s and change no state; text through `TextRole`s; strings through `gettext`; no em dashes; what a user typed never reaches a log; `app.rs` and `ui/mod.rs` get no new code beyond what is named.
- Commits are signed, one per task, after its checks pass. If signing fails, do not commit unsigned: stage the task and tell the user. Each message ends with:

      Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>

## What run 2's decisions change here

Decided on 2026-10-10, after this plan was written (see "Amendments of 2026-10-10" in run 2's plan). **Where a task below disagrees, this is right.**

- `App::lookups` holds `SentLookup`s, and `send_lookup` takes the serial of the picker a lookup is for. The question of section 7 is no picker's: `ask_row` sends it as `self.send_lookup(session, lookup, 0)`.
- A pinned row's read is never cancelled. So `ask_row` does not cancel a stale question (it only forgets it), task 2's addition to `send_due_lookups` (the questions' requests as "waited") is not needed, and the test `a_late_or_failed_answer_warns_of_nothing` must not expect a `Command::Cancel` after the discard: it expects none, and that the late answer warns of nothing.

## What was decided and found while this was built

The run is built. The tasks below were left as they were written; this is where the code differs from them.

- **The user's answers of 2026-10-11** to rows a to d of the table below: a (the outline) as the row says; b (the Omarchy look's line under the grid, `~` in the gutter) as the row says; c (the hint, the row panel's field, the bar's count) as the row says; d is built as the spec has it and not as the row says: the count is a polite live region (`widgets::announce_polite`), on the pending bar and, in the Omarchy look, on the line under the grid.
- **A doubt goes with its cell's text.** `Edits::put` drops the cell's doubt unless the new value is that very text, and `revert`, `drop_row` and `discard` drop it with the value. Task 2's note that a stale doubt is harmless does not hold: `close_editor` reads a doubt by its text, so one left behind by a revert would be taken as known when the same text is typed again.
- **`close_editor` decides in this order:** not changed or to fix, no doubt; taken from the list, no doubt and nothing asked (the task's `keep` forgot `taken`, and a warned key picked from the list kept its warning); a doubt of this very text stands, kept; the cell already held this text before the close, nothing asked; otherwise asked.
- **A number is asked about trimmed**, as the column's check reads it. A text key is asked about as typed.
- **A question that comes back cancelled is asked again once**, as a picker's read is: the sweep cancels a closing editor's search on the same session just as the question is sent.
- **A reconnect forgets the questions** the old session was asked (`Workspace::forget_session_requests`).
- **Under the keyboard's cursor** the outline is drawn inside the cursor's line, in every look: the cursor is on the cell right after the key is typed and kept, and its line covered the outline.
- **A save that runs** takes every view of the warning with it (`ui::key_picker::unmatched` answers nothing while the table is saving), not the grid's cell alone.
- **The Omarchy look lowers the app's words** in the hint, the row panel and the line under the grid alike (`doubt_text` takes the look).
- **Two scenes** in `src/shots.rs`, `edit-key-missing` and `row-form-key-missing`, are where the look was checked. Nobody ran the app.
- **Left as it is:**
  - The pending bar's short form leaves the count out, so a narrow bar (the row panel open) does not show it.
  - The self-reference rule reads a number without the white space around it, and otherwise compares the key's text exactly (`099` is not `99`).
  - A cell whose question a reconnect forgot, or whose read failed or came back cancelled twice, is not asked again until its text changes: its warning can be missed, never falsely shown.
  - "Open publisher →" and `gd` still follow a warned value, and open the referenced table filtered to nothing.
  - `row_panel::singular` makes "addresse" of `addresses` and leaves `categories` as it is, in the warning as in the row panel's link. It is older than this run.

## What the spec asks, and what gets built

**Rows a to d change what the spec says a user sees. Ask the user before building them, and write the answer into the spec in task 4.**

| | The spec | What gets built | Why |
|---|---|---|---|
| a | "The cell keeps the pending fill and gets the look's caution mark at its start" | The pending fill and bar, and an outline in the pending tone, as a cell to fix has one in the failure's tone | The grid marks a cell by its fill, its bar and its outline (`Mark`). A glyph at a cell's start would stand on a right-aligned number's room and wants a measure of its own |
| b | "`?` in the row's gutter in the Omarchy look", and "The pending bar adds `· 1 value names no row`" | The row's gutter keeps `~`. The line under the grid, which says the first cell to fix, says the first warning when nothing is to fix: `? 4:publisher_id  no publisher with id 9100000000000000099`. The Omarchy look draws no pending bar, so the count is the other looks' alone | The gutter's mark is a row's (`RowMark`, worked out from the cells' states), and the spec has the row's mark stay a changed row's |
| c | The words show "in the status line while the cell is selected" | Under the pointer on the cell (its hint), under the row panel's field, and in the terminal's line of row b | The looks that are not the terminal's have no line that says a selected cell's problem: a cell to fix says its own in its hint |
| d | The warning "is announced when it appears" | The pending bar's count (`1 value names no row`) is a label a screen reader reads, and the row panel's field says it in its name | One announcement for the bar's words, as for its other counts |
| e | The question is kept "on the pending cell, beside its state" | In a map of the tab's edits, `Edits::doubts`, by cell, with the text that was asked about | `Pending` is built in two dozen places. A warning that names its text ends by itself when the cell's text changes |
| f | "A value taken from the list" is not asked about | `Picker::taken`, set when a commit takes an option, read when the editor closes | Run 2 left the place for it in `picker_takes` |

## File map

| File | What changes |
|---|---|
| `src/edit.rs` | `Doubt`, `DoubtState`, `Edits::doubts` (task 1) |
| `src/key_picker.rs` | `Picker::taken`, `Unmatched`, `unmatched` (task 1) |
| `src/testing.rs` | The `genres` fixture, a table that points at itself (task 1) |
| `src/app/key_picker.rs` | `ask_row`, the answer, the sweep's owners (task 2) |
| `src/app/editing.rs` | `close_editor` asks (task 2) |
| `src/ui/key_picker.rs` | `doubt_text` (task 3) |
| `src/ui/grid.rs` | `Mark::Doubt` (task 3) |
| `src/ui/data_view.rs` | The cell's mark and hint, the terminal's line (task 3) |
| `src/ui/pending_bar.rs` | The count (task 3) |
| `src/ui/row_panel.rs` | The field's words; `singular` is shared (task 3) |
| `src/ui/key_picker_tests.rs` | Test 21 (tasks 2, 3) |
| `docs/_guide/editing-data.md`, the spec | Task 4 |

---

### Task 1: What a warning is

**Files:**
- Modify: `src/edit.rs` (`Edits`, near line 1054)
- Modify: `src/key_picker.rs`
- Modify: `src/testing.rs`

- [ ] **Step 1: The question, on the tab's edits**

In `src/edit.rs`, before `pub struct Edits`:

```rust
/// What is known of whether a foreign key cell's pending value names a
/// row of the table its key points at. No `Debug`: it holds what was
/// typed, as `Pending` does.
#[derive(Clone, PartialEq, Eq)]
pub struct Doubt {
    /// The pending text that was asked about. A cell that holds another
    /// text by now has no doubt: this one was of a value it no longer
    /// has.
    pub text: String,
    pub state: DoubtState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DoubtState {
    /// The database was asked, and has not answered.
    Asked(crate::backend::RequestId),
    /// No row has that key.
    Missing,
}
```

and in `pub struct Edits`, after `cells`:

```rust
    /// The foreign key cells whose typed value was asked about, by row
    /// and column as `cells`. A warning, and it holds nothing: see
    /// `key_picker::unmatched`, which says which of them stand.
    pub doubts: BTreeMap<(usize, usize), Doubt>,
```

`Edits` derives `Default`, and `Edits::discard` builds a fresh one: the doubts go with what is pending. If `RequestId` does not derive `Eq`, drop `Eq` from the two derives.

- [ ] **Step 2: Write the failing tests**

In `src/key_picker.rs`, add to the tests module:

```rust
    use std::collections::BTreeSet;

    use crate::edit::{Doubt, DoubtState, Edits, NEW_ROWS, Pending, State, Table};
    use tabletist_db::{
        Access, ColumnMeta, ForeignKeyInfo, NewValue, ObjectKind, RowPage, ValueKind,
    };

    /// `genres (id, parent_id → genres.id, name)`, two rows loaded.
    fn genres() -> (Structure, RowPage) {
        let column = |name: &str| ColumnInfo {
            name: name.into(),
            type_name: if name == "name" { "TEXT" } else { "INTEGER" }.into(),
            nullable: name == "parent_id",
            ..ColumnInfo::default()
        };
        let meta = |name: &str, kind| ColumnMeta {
            name: name.into(),
            type_name: if name == "name" { "TEXT" } else { "INTEGER" }.into(),
            kind,
        };
        let structure = Structure {
            columns: vec![column("id"), column("parent_id"), column("name")],
            primary_key: vec!["id".into()],
            foreign_keys: vec![ForeignKeyInfo {
                columns: vec!["parent_id".into()],
                ref_schema: "main".into(),
                ref_table: "genres".into(),
                ref_columns: vec!["id".into()],
                ..ForeignKeyInfo::default()
            }],
            ..Structure::default()
        };
        let page = RowPage {
            columns: vec![
                meta("id", ValueKind::Numeric),
                meta("parent_id", ValueKind::Numeric),
                meta("name", ValueKind::Text),
            ],
            rows: vec![
                vec![Value::Int(1), Value::Null, Value::Text("Fiction".into())],
                vec![Value::Int(2), Value::Int(1), Value::Text("Crime".into())],
            ],
            has_more: false,
            ordered_by_key: true,
            elapsed: Duration::ZERO,
        };
        (structure, page)
    }

    fn ready(text: &str) -> Pending {
        Pending {
            new: NewValue::Text(text.into()),
            state: State::Ready,
        }
    }

    fn missing(text: &str) -> Doubt {
        Doubt {
            text: text.into(),
            state: DoubtState::Missing,
        }
    }

    #[test]
    fn a_warning_stands_only_for_the_text_the_cell_still_holds() {
        let (structure, page) = genres();
        let gone = BTreeSet::new();
        let own = ObjectRef::new("main", "genres");
        let mut edits = Edits::default();
        let table = |edits: &Edits| {
            let found = unmatched(
                &Table {
                    access: Access::Writable,
                    kind: ObjectKind::Table,
                    dialect: Dialect::Sqlite,
                    structure: Some(&structure),
                    page: &page,
                    refreshing: false,
                    saving: false,
                    gone: &gone,
                    added: &edits.added,
                },
                &own,
                edits,
            );
            found
                .into_iter()
                .map(|found| (found.at, found.table, found.key, found.value))
                .collect::<Vec<_>>()
        };
        // Row 1's parent is 99, and no genre has that id.
        edits.cells.insert((1, 1), ready("99"));
        edits.doubts.insert((1, 1), missing("99"));
        assert_eq!(
            table(&edits),
            [((1, 1), "genres".to_owned(), "id".to_owned(), "99".to_owned())]
        );
        // Asked, not answered: nothing is said yet.
        edits.doubts.insert(
            (1, 1),
            Doubt {
                text: "99".into(),
                state: DoubtState::Asked(RequestId(7)),
            },
        );
        assert!(table(&edits).is_empty());
        // The value changed since: the answer was of another text.
        edits.doubts.insert((1, 1), missing("99"));
        edits.cells.insert((1, 1), ready("98"));
        assert!(table(&edits).is_empty());
        // A cell to fix says its own problem.
        edits.cells.insert(
            (1, 1),
            Pending {
                new: NewValue::Text("99".into()),
                state: State::ToFix(crate::edit::Problem::WholeNumber),
            },
        );
        assert!(table(&edits).is_empty());
        // Reverted: no pending cell, no warning.
        edits.cells.remove(&(1, 1));
        assert!(table(&edits).is_empty());
    }

    #[test]
    fn a_key_that_a_new_row_of_the_same_table_holds_is_no_warning() {
        let (structure, page) = genres();
        let gone = BTreeSet::new();
        let own = ObjectRef::new("main", "genres");
        let mut edits = Edits::default();
        edits.cells.insert((1, 1), ready("99"));
        edits.doubts.insert((1, 1), missing("99"));
        // A new row whose `id` is 99: the same save writes it.
        edits.cells.insert((NEW_ROWS, 0), ready("99"));
        let table = Table {
            access: Access::Writable,
            kind: ObjectKind::Table,
            dialect: Dialect::Sqlite,
            structure: Some(&structure),
            page: &page,
            refreshing: false,
            saving: false,
            gone: &gone,
            added: &edits.added,
        };
        assert!(unmatched(&table, &own, &edits).is_empty());
        // Another table's key of the same number is no row of this one.
        let other = ObjectRef::new("main", "books");
        assert_eq!(unmatched(&table, &other, &edits).len(), 1);
    }
```

`Table`'s fields are the ones `Table::of` fills (`src/edit.rs` near line 67): if a run added one, the struct literals above get it. `NEW_ROWS` is the first row that is a new row's (`edit::new_row(0)` is the same number).

- [ ] **Step 3: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked --lib key_picker::tests`
Expected: a compile error, `cannot find function unmatched`.

- [ ] **Step 4: Which warnings stand**

In `src/key_picker.rs`, add `use crate::edit::{DoubtState, Edits, State, Table, new_id};` and `NewValue` to the `tabletist_db` imports, then:

```rust
/// A foreign key cell whose pending value names no row.
pub struct Unmatched {
    /// The cell, by row and column of the page.
    pub at: (usize, usize),
    /// The table its key points at, and the column there.
    pub table: String,
    pub key: String,
    /// The value, as it was typed.
    pub value: String,
}

/// The warnings of `edits` that stand, for the table `own` whose page
/// `table` is: a cell that was answered "no such row", still holds the
/// text that was asked about, and waits for a save with nothing to fix.
/// Not one whose key a new row of the same tab gives the referenced
/// column: a table that points at itself can have one new row name
/// another, and the save writes both.
pub fn unmatched(table: &Table<'_>, own: &ObjectRef, edits: &Edits) -> Vec<Unmatched> {
    let Some(structure) = table.structure else {
        return Vec::new();
    };
    edits
        .doubts
        .iter()
        .filter(|(_, doubt)| doubt.state == DoubtState::Missing)
        .filter_map(|(&at, doubt)| {
            let pending = edits.cells.get(&at)?;
            let holds = match (&pending.new, &pending.state) {
                (NewValue::Text(text), State::Ready) => *text == doubt.text,
                _ => false,
            };
            if !holds {
                return None;
            }
            let column = table.page.columns.get(at.1)?;
            let (target, key) = self::target(structure, &column.name)?;
            if target == *own {
                // The referenced column's place in this page, and whether
                // a new row sets it to this value.
                let place = table.page.columns.iter().position(|column| column.name == key);
                let made = place.is_some_and(|place| {
                    edits.cells.iter().any(|(&(row, col), cell)| {
                        col == place
                            && new_id(row).is_some()
                            && matches!(&cell.new, NewValue::Text(text) if *text == doubt.text)
                    })
                });
                if made {
                    return None;
                }
            }
            Some(Unmatched {
                at,
                table: target.name,
                key,
                value: doubt.text.clone(),
            })
        })
        .collect()
}
```

`self::target` is the module's own function (the closure's `target` binding would shadow it otherwise). Also add to `pub struct Picker`:

```rust
    /// The editor's text is the key of an option a commit took from the
    /// list: it was just read from the table, and is not asked about
    /// again.
    pub taken: bool,
```

with `taken: false` in `Picker::new`, and in `Picker::typed`, after `self.moved = false;`:

```rust
        self.taken = false;
```

- [ ] **Step 5: A table that points at itself, for the app's tests**

In `src/testing.rs`, after `keyed_covers_structure`:

```rust
/// `genres`: a table that points at itself. `parent_id` is a foreign key
/// to `genres.id`, and `id` is typed by hand: no database numbers it.
pub fn genres_structure() -> tabletist_db::Structure {
    use tabletist_db::ColumnInfo;
    let column = |name: &str, type_name: &str, nullable: bool| ColumnInfo {
        name: name.into(),
        type_name: type_name.into(),
        nullable,
        ..ColumnInfo::default()
    };
    tabletist_db::Structure {
        columns: vec![
            column("id", "INTEGER", false),
            column("parent_id", "INTEGER", true),
            column("name", "TEXT", false),
        ],
        primary_key: vec!["id".into()],
        foreign_keys: vec![tabletist_db::ForeignKeyInfo {
            columns: vec!["parent_id".into()],
            ref_schema: "main".into(),
            ref_table: "genres".into(),
            ref_columns: vec!["id".into()],
            ..tabletist_db::ForeignKeyInfo::default()
        }],
        ..tabletist_db::Structure::default()
    }
}

/// The two genres the tests hold: Fiction (1), and Crime (2) under it.
pub fn genres_page() -> RowPage {
    let meta = |name: &str, type_name: &str, kind: ValueKind| ColumnMeta {
        name: name.into(),
        type_name: type_name.into(),
        kind,
    };
    RowPage {
        columns: vec![
            meta("id", "INTEGER", ValueKind::Numeric),
            meta("parent_id", "INTEGER", ValueKind::Numeric),
            meta("name", "TEXT", ValueKind::Text),
        ],
        rows: vec![
            vec![Value::Int(1), Value::Null, Value::Text("Fiction".into())],
            vec![Value::Int(2), Value::Int(1), Value::Text("Crime".into())],
        ],
        has_more: false,
        ordered_by_key: true,
        elapsed: std::time::Duration::from_millis(2),
    }
}

/// The genres, as (id, name), for [`Harness::answer_lookups`].
pub const GENRES: [(i64, &str); 2] = [(1, "Fiction"), (2, "Crime")];
```

and in the `impl Harness` block that holds `keyed_covers`:

```rust
    /// `genres` open, writable, its structure and its two rows loaded.
    pub fn genres(&mut self) -> (ConnTabId, TabId) {
        let tab = self.connect_fake_as(false);
        self.app.apply(Action::OpenObject {
            tab,
            object: tabletist_db::ObjectRef::new("main", "genres"),
            kind: ObjectKind::Table,
            pin: true,
        });
        self.answer_structure(genres_structure());
        self.answer_rows(genres_page());
        let id = self
            .app
            .workspace(tab)
            .and_then(|workspace| workspace.active_tab)
            .expect("the table's tab is open");
        (tab, id)
    }
```

- [ ] **Step 6: Run the tests**

Run: `~/.cargo/bin/cargo test --locked --lib key_picker::tests`
Expected: PASS, with the two new tests.

- [ ] **Step 7: Run the four checks, then commit**

`unmatched`, `Edits::doubts` and the fixture are used by tests alone until task 2. They are `pub` items of `pub` modules, so clippy does not call them dead. If it does, the task's order is wrong for this tree: do task 2 before committing, and commit the two together.

```bash
git add src/edit.rs src/key_picker.rs src/testing.rs && git commit -m "Say which foreign key warnings stand

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: A typed key is asked about

**Files:**
- Modify: `src/ui/key_picker_tests.rs`
- Modify: `src/app/key_picker.rs`
- Modify: `src/app/editing.rs` (`close_editor`, near line 916)

- [ ] **Step 1: Write the failing tests**

Append to `src/ui/key_picker_tests.rs`:

```rust
use crate::edit::{Doubt, DoubtState};

/// No publisher has this id.
const NO_PUBLISHER: &str = "9100000000000000099";

/// The warnings that stand on the tab's table.
fn warned(harness: &Harness, tab: ConnTabId, id: TabId) -> Vec<((usize, usize), String)> {
    let workspace = harness.app.workspace(tab).unwrap();
    let object = workspace.object_tab(id).unwrap();
    let table = crate::edit::Table::of(workspace, object).unwrap();
    crate::key_picker::unmatched(&table, &object.object, &object.edits)
        .into_iter()
        .map(|found| (found.at, found.value))
        .collect()
}

/// Types `text` into `publisher_id` of the page's row `row` and keeps it
/// as typed, with every lookup answered: the picker's, then the question
/// the kept value raises.
fn keep(harness: &mut Harness, tab: ConnTabId, id: TabId, row: usize, text: &str) {
    edit(harness, tab, id, row, EditStart::Replace(String::new()));
    type_text(harness, tab, id, text);
    commit(harness, tab, id, Advance::Stay);
    harness.answer_lookups(&PUBLISHERS);
}

/// Test 21: a typed key no row has leaves the cell pending and warned,
/// with Save offered. Changing the value drops the warning.
#[test]
fn a_typed_key_that_names_no_row_is_warned_about() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    let before = harness.lookups().len();
    keep(&mut harness, tab, id, 0, NO_PUBLISHER);
    assert_eq!(pending(&harness, tab, id, 0).as_deref(), Some(NO_PUBLISHER));
    // One read of that key alone, after the picker's own.
    let sent = harness.lookups();
    let last = &sent.last().unwrap().2;
    assert!(sent.len() > before);
    assert!(last.part == LookupPart::Pinned(Value::Int(9_100_000_000_000_000_099)));
    assert_eq!(last.limit, 1);
    assert_eq!(
        warned(&harness, tab, id),
        [((0, PUBLISHER), NO_PUBLISHER.to_owned())]
    );
    // It holds nothing: the cell is ready, and a save is not blocked.
    assert!(harness.app.save_blocked(tab, id).is_none());

    // Another value: the warning was of the old one.
    keep(&mut harness, tab, id, 0, HARBOR_LIGHTS);
    assert!(warned(&harness, tab, id).is_empty());
    // And Harbor Lights is there: no warning of its own.
    let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
    assert!(!object.edits.doubts.contains_key(&(0, PUBLISHER)));
}

/// Test 21: a value taken from the list is not asked about, and neither
/// is one that is to fix.
#[test]
fn a_picked_value_and_a_value_to_fix_are_not_asked_about() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    edit(&mut harness, tab, id, 0, EditStart::Replace(String::new()));
    type_text(&mut harness, tab, id, "harb");
    let before = harness.lookups().len();
    commit(&mut harness, tab, id, Advance::Stay);
    assert_eq!(pending(&harness, tab, id, 0).as_deref(), Some(HARBOR_PRESS));
    assert_eq!(harness.lookups().len(), before, "a picked value was asked about");

    // `harb` kept by leaving the editor: an integer column refuses it,
    // and a text that is to fix names no key to ask about.
    edit(&mut harness, tab, id, 1, EditStart::Replace(String::new()));
    type_text(&mut harness, tab, id, "harb");
    let before = harness.lookups().len();
    let leave = crate::testing::leave_edit(&harness.app, tab, id);
    harness.app.apply(leave);
    assert_eq!(harness.lookups().len(), before);
    let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
    assert!(object.edits.doubts.is_empty());
}

/// Spec 7: an answer for a value the cell no longer has is dropped, a
/// failed answer says nothing, and a question that nothing waits for is
/// cancelled.
#[test]
fn a_late_or_failed_answer_warns_of_nothing() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    edit(&mut harness, tab, id, 0, EditStart::Replace(String::new()));
    type_text(&mut harness, tab, id, NO_PUBLISHER);
    commit(&mut harness, tab, id, Advance::Stay);
    // Asked, not answered: nothing is said.
    let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
    assert!(matches!(
        object.edits.doubts.get(&(0, PUBLISHER)),
        Some(Doubt {
            state: DoubtState::Asked(_),
            ..
        })
    ));
    assert!(warned(&harness, tab, id).is_empty());
    // The database refuses the read: no warning, and no question left.
    harness.fail_lookup(tabletist_db::Error::query("permission denied for table publishers"));
    let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
    assert!(object.edits.doubts.is_empty());

    // Asked again for another row, then discarded before the answer: the
    // question is cancelled with what was pending.
    edit(&mut harness, tab, id, 1, EditStart::Replace(String::new()));
    type_text(&mut harness, tab, id, NO_PUBLISHER);
    commit(&mut harness, tab, id, Advance::Stay);
    let asked = harness.lookups().last().unwrap().1;
    harness.app.apply(Action::DiscardEdits { tab, id });
    harness.app.send_due_lookups(std::time::Instant::now());
    let cancelled = harness.app.backend.sent.iter().any(|command| {
        matches!(command, Command::Cancel { request, .. } if *request == asked)
    });
    assert!(cancelled);
    harness.answer_lookups(&PUBLISHERS);
    assert!(warned(&harness, tab, id).is_empty());
}

/// Test 21: a key that a new row of the same tab holds is not warned
/// about, whichever of the two was typed first.
#[test]
fn a_key_a_new_row_of_the_same_table_holds_is_not_warned_about() {
    use crate::testing::{GENRES, genres_structure};
    let mut harness = Harness::new();
    harness.set_look(Look::macos());
    let (tab, id) = harness.genres();
    harness.app.workspace_mut(tab).unwrap().pane = Pane::Grid;
    harness.settle();
    // Crime's parent becomes 99, and no genre has that id.
    harness.app.apply(Action::EditCell {
        tab,
        id,
        cell: CellPos { row: 1, col: 1 },
        start: EditStart::Replace("99".into()),
    });
    harness.answer_structure(genres_structure());
    harness.answer_lookups(&GENRES);
    commit(&mut harness, tab, id, Advance::Stay);
    harness.answer_lookups(&GENRES);
    assert_eq!(warned(&harness, tab, id), [((1, 1), "99".to_owned())]);
    // A new genre with the id 99: the save writes both.
    harness.app.apply(Action::AddRow {
        tab,
        id,
        place: crate::edit::Place::Top,
    });
    harness.app.apply(Action::CancelEdit { tab, id });
    let new = crate::edit::new_row(0);
    harness.app.apply(Action::EditCell {
        tab,
        id,
        cell: CellPos { row: new, col: 0 },
        start: EditStart::Replace("99".into()),
    });
    commit(&mut harness, tab, id, Advance::Stay);
    assert!(warned(&harness, tab, id).is_empty());
}
```

`Action::DiscardEdits { tab, id }` is the action the pending bar's Discard all sends: check its fields in `src/model.rs`. The last test types `99` into `id` of the new row with `EditCell` on column 0: if `genres.id` reads as a column the database numbers (it does not: the fixture's `ColumnInfo` has `identity: false`), the cell would be locked.

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked --lib key_picker_tests`
Expected: the four new tests FAIL: no lookup is sent after the commit, and nothing is warned about.

- [ ] **Step 3: Ask when the editor closes**

In `src/app/key_picker.rs`:

1. `picker_takes`: in its `Take::Option(index)` arm, after `editor.touched = true;`, add:

```rust
                    // Just read from the table: not asked about again.
                    picker.taken = true;
```

2. A new method of `impl App` (add `Doubt, DoubtState` to the `crate::edit` imports, `LookupPart` and `NewValue` to the `tabletist_db` ones, `LIMIT` is not needed):

```rust
    /// Asks whether the pending value of the foreign key cell `cell`
    /// names a row of the table its key points at: one read of that key
    /// alone. Nothing is asked of a cell that is no foreign key's, of
    /// what is no typed text waiting for a save, or of a text that cannot
    /// be read as the key's type.
    pub(super) fn ask_row(&mut self, tab: ConnTabId, id: TabId, cell: crate::model::CellPos) {
        let planned = self.workspace(tab).and_then(|workspace| {
            let object = workspace.object_tab(id)?;
            let table = Table::of(workspace, object)?;
            let pending = object.edits.cells.get(&(cell.row, cell.col))?;
            let text = match (&pending.new, &pending.state) {
                (NewValue::Text(text), crate::edit::State::Ready) => text,
                _ => return None,
            };
            let column = table.column(cell.col)?;
            let (target, key) = key_picker::target(table.structure?, &column.name)?;
            let class = table.class(cell.col)?;
            // It passed the column's check: it is ready.
            let value = key_picker::pinned(text, class, &column.type_name, true)?;
            let lookup = tabletist_db::Lookup {
                object: target,
                key,
                label: None,
                part: LookupPart::Pinned(value),
                limit: 1,
            };
            Some((workspace.session, lookup, text.clone()))
        });
        // A question the cell had is of a value it no longer holds.
        let stale = self
            .object_tab_mut(tab, id)
            .and_then(|object| object.edits.doubts.remove(&(cell.row, cell.col)));
        let Some((session, lookup, text)) = planned else {
            return;
        };
        if let Some(Doubt {
            state: DoubtState::Asked(request),
            ..
        }) = stale
        {
            self.lookups.retain(|(_, sent)| *sent != request);
            self.cancel(session, [request]);
        }
        let request = self.send_lookup(session, lookup);
        if let Some(object) = self.object_tab_mut(tab, id) {
            let state = DoubtState::Asked(request);
            object
                .edits
                .doubts
                .insert((cell.row, cell.col), Doubt { text, state });
        }
    }
```

   `App::object_tab_mut(tab, id)` is private to `src/app.rs`, which a child module may call.

3. `lookup_answered`: a lookup no picker took may be a question. After the loop over the pickers (which returns when one takes the answer), add:

```rust
        // No picker's: a question about a kept value, where one waits.
        let Some(workspace) = self.workspace_mut(tab) else {
            return;
        };
        for object in workspace.object_tabs_mut() {
            let asked = object.edits.doubts.iter().find_map(|(&at, doubt)| {
                (doubt.state == DoubtState::Asked(request)).then_some(at)
            });
            let Some(at) = asked else {
                continue;
            };
            let none = matches!(&result, Ok(page) if page.rows.is_empty());
            if none {
                if let Some(doubt) = object.edits.doubts.get_mut(&at) {
                    doubt.state = DoubtState::Missing;
                }
            } else {
                // The row is there, or the read failed: nothing to say.
                object.edits.doubts.remove(&at);
            }
            return;
        }
```

   `result` was moved into `picker.answered(request, result)` inside the first loop. That loop returns right after the move, so the compiler lets the code after it read `result`. If it does not, find the picker's tab id in a first pass and take the answer in a second, as run 2's plan says for the same loop.

4. `send_due_lookups`: a question is waited for too. In its loop over the object tabs, before the `let Some(picker) = ... else { continue; };`, add:

```rust
                waited.extend(object.edits.doubts.values().filter_map(|doubt| match doubt.state {
                    DoubtState::Asked(request) => Some(request),
                    DoubtState::Missing => None,
                }));
```

- [ ] **Step 4: `close_editor` asks**

In `src/app/editing.rs`, `close_editor` reads what the editor's text comes to (`typed`), drops the editor, and puts the pending cell. Whether the value was taken from the list is the picker's to say, and the picker goes with the editor: read it first. At the function's start, after `let verdict = self.table(tab, id, typed);`:

```rust
        // Whether the text is the key of an option taken from the
        // picker's list: read before the editor goes.
        let taken = self
            .editor_mut(tab, id)
            .and_then(|editor| editor.picker.as_ref())
            .is_some_and(|picker| picker.taken);
```

and where the function puts the cell:

```rust
        if changed {
            let state = problem.map_or(State::Ready, State::ToFix);
            object.edits.put(key, Pending { new, state });
        } else {
            object.edits.revert(key);
        }
        // The row panel shows the pending value.
        object.fields = None;
        true
```

replace the tail with:

```rust
        // The cell was asked about this very text before (an editor
        // opened on a warned value and closed on it again): it is asked
        // once, and what is known of it stays.
        let known = matches!(
            &new,
            NewValue::Text(text)
                if object.edits.doubts.get(&key).is_some_and(|doubt| doubt.text == *text)
        );
        let ask = changed && problem.is_none() && !taken && !known;
        let keep = changed && problem.is_none() && known;
        if changed {
            let state = problem.map_or(State::Ready, State::ToFix);
            object.edits.put(key, Pending { new, state });
        } else {
            object.edits.revert(key);
        }
        // The row panel shows the pending value.
        object.fields = None;
        // A question the cell had was of the value before this one.
        if !ask && !keep {
            object.edits.doubts.remove(&key);
        }
        if ask {
            // A typed key: does it name a row?
            self.ask_row(tab, id, cell);
        }
        true
```

`problem` is moved by `problem.map_or(...)`, and `new` by the `put`: `known`, `ask` and `keep` are worked out before both, as above. `object` is a `&mut` borrow of `self` that must end before `self.ask_row`: it does, since nothing uses `object` after `object.edits.doubts.remove`. `ask_row` removes a stale question itself, and cancels it if it still runs.

A cell that is made NULL, DEFAULT or now, or reverted, keeps a stale doubt in the map until its row's edits go. It is harmless: `unmatched` counts a doubt only for a cell that holds its text. Do not chase those paths.

- [ ] **Step 5: Run the tests**

Run: `~/.cargo/bin/cargo test --locked --lib key_picker`
Expected: PASS, the four new tests with the rest.

Then the whole suite. A test of an earlier run that types a number into a foreign key cell and keeps it now sends one more `Lookup` (`harness.keyed_covers` is the only fixture with a key in `src/ui/key_picker_tests.rs`; `src/ui/mod.rs` near line 11188 has one for `gd`). A test that counts lookups, or takes "the newest lookup", may need its count moved: fix it there, and say so in the report.

- [ ] **Step 6: Run the four checks, then commit**

```bash
git add src/app/key_picker.rs src/app/editing.rs src/ui/key_picker_tests.rs && git commit -m "Ask whether a typed foreign key names a row

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: The warning shows

**Before this task, ask the user about rows a to d of "What the spec asks, and what gets built".** They are how the warning looks, and the boards draw none.

**Files:**
- Modify: `src/ui/key_picker.rs`
- Modify: `src/ui/row_panel.rs` (`singular` near line 2689, `FieldInfo` near line 99, where it is built near line 505, `field` near lines 1365 and 1778)
- Modify: `src/ui/grid.rs` (`Mark`, near line 102, and where a cell is painted near lines 886 and 930)
- Modify: `src/ui/data_view.rs` (`Changes` near line 1568, where it is built near line 1342, `Changes::mark` near line 1811, `error_line` near line 1177)
- Modify: `src/ui/pending_bar.rs` (`counts_text` near line 49, `show` near line 240)
- Modify: `src/ui/key_picker_tests.rs`

- [ ] **Step 1: Write the failing tests**

Append to `src/ui/key_picker_tests.rs`:

```rust
const WARNING: &str = "No publisher with id 9100000000000000099";

/// Test 21, what a user sees: the pending bar counts the value, Save is
/// offered, the cell says it under the pointer, and the row panel's field
/// says it under its value.
#[test]
fn the_warning_is_said_and_holds_no_save() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    keep(&mut harness, tab, id, 0, NO_PUBLISHER);
    harness.settle();
    assert!(harness.has("1 change in 1 row · 1 value names no row"));
    let tree = harness.settle();
    let save = crate::testing::node(&tree, "Save", Role::Button).expect("Save is offered");
    assert!(!tree.nodes.iter().any(|(node, built)| *node == save && built.is_disabled()));
    // The row panel's field of the cell.
    harness.app.apply(Action::FocusFields { tab, id });
    harness.settle();
    assert!(harness.has(WARNING));

    // Two of them are counted as two.
    keep(&mut harness, tab, id, 1, NO_PUBLISHER);
    harness.settle();
    assert!(harness.has("2 changes in 2 rows · 2 values name no row"));
}

/// The terminal look says the first warning in the line under the grid,
/// where it says the first cell to fix, and a cell to fix comes first.
#[test]
fn the_terminal_look_says_the_warning_under_the_grid() {
    let (mut harness, tab, id) = covers_in(Look::omarchy());
    keep(&mut harness, tab, id, 0, NO_PUBLISHER);
    harness.settle();
    assert!(harness.has("? 1:publisher_id  no publisher with id 9100000000000000099"));
}
```

How a disabled button reads in the tree: `src/ui/insert_audit_tests.rs` checks a disabled Save in its test of a new row that lacks a value. Use what it uses in place of the `is_disabled()` line above. The pending bar's whole line may be several labels (the counts in one weight, what is to fix in another): if `harness.has` does not find the sentence as one label, assert on the piece that holds the count, and print `crate::testing::labels(&tree)` to see how the bar is cut. The terminal's line is written in the look's lower case for the app's own words (`look.label`) and keeps the column's name and the value as they are: match what `error_line` does for a cell to fix.

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked --lib key_picker_tests::the_`
Expected: both FAIL: nothing says "names no row".

- [ ] **Step 3: The words**

In `src/ui/row_panel.rs`, make `singular` visible to its siblings: `pub(crate) fn singular(table: &str) -> &str`.

In `src/ui/key_picker.rs`, add:

```rust
/// What a foreign key cell whose value names no row says: "No publisher
/// with id 9100000000000000099". The table's name is made singular as
/// the row panel's "Open publisher →" makes it.
pub fn doubt_text(found: &crate::key_picker::Unmatched, locale: Locale) -> String {
    let table = display_safe(&found.table);
    // A placeholder each: a translation puts them where they read.
    gettext(locale, "No {row} with {key} {value}")
        .replace("{row}", crate::ui::row_panel::singular(&table))
        .replace("{key}", &display_safe(&found.key))
        .replace("{value}", &display_safe(&found.value))
}

/// The warnings that stand on a table's tab, worded, by cell.
pub fn doubts(
    workspace: &crate::model::Workspace,
    object: &crate::model::ObjectTab,
    locale: Locale,
) -> std::collections::BTreeMap<(usize, usize), String> {
    let Some(table) = crate::edit::Table::of(workspace, object) else {
        return std::collections::BTreeMap::new();
    };
    crate::key_picker::unmatched(&table, &object.object, &object.edits)
        .iter()
        .map(|found| (found.at, doubt_text(found, locale)))
        .collect()
}
```

`key_picker` is a private module of `src/ui/mod.rs`; `data_view`, `pending_bar` and `row_panel` are its siblings and reach it as `crate::ui::key_picker` (or `super::key_picker`).

- [ ] **Step 4: The grid's cell**

In `src/ui/grid.rs`, add a variant to `pub enum Mark`, after `Pending`:

```rust
    /// Pending, with a warning that holds no save: a foreign key that
    /// names no row.
    Doubt,
```

Where the cell's tint is chosen (`let tone = match content.mark {`), `Doubt` is pending's:

```rust
                        Mark::Pending | Mark::Doubt | Mark::Saving => Some(Tone::Warning),
```

Where the bar and the outline are drawn (`match content.mark {` with `Mark::Pending | Mark::Saving if !look.terminal`), add an arm before it:

```rust
                        Mark::Doubt => {
                            // A pending cell's bar, and an outline in its
                            // tone: a cell to fix has one in the
                            // failure's.
                            if !look.terminal {
                                let bar = Rect::from_min_size(cell_rect.min, vec2(2.0, height));
                                painter.rect_filled(
                                    bar,
                                    CornerRadius::ZERO,
                                    Tone::Warning.color(palette),
                                );
                            }
                            painter.rect_stroke(
                                cell_rect,
                                CornerRadius::ZERO,
                                Stroke::new(1.0, Tone::Warning.color(palette)),
                                StrokeKind::Inside,
                            );
                        }
```

Build. Every other `match` over `Mark` without a wildcard (the compiler names them, in `grid.rs` and wherever a mark is read as a word for a screen reader) treats `Doubt` as it treats `Pending`.

In `src/ui/data_view.rs`:

1. `struct Changes<'a>` gets a field, and `Changes::of` sets it empty:

```rust
    /// The cells whose foreign key names no row, with what each says.
    doubts: BTreeMap<(usize, usize), String>,
```

2. The map is worked out where `target` is made (near line 1270), while `workspace` and the tab are still borrowed shared: `object` is taken `&mut` a few lines on (near line 1278), for the editor.

```rust
        // The cells whose foreign key names no row, with what each says.
        let doubts = key_picker::doubts(workspace, object, locale);
```

   and after `let mut changes = Changes::of(...)` (near line 1342):

```rust
        changes.doubts = doubts;
```

3. A pending cell's mark and hint are set by `Changes::mark_state` (near line 1794), which `Changes::mark` (near line 1774) and `Changes::new_cell` (near line 1864) both call. `mark_state` does not know the cell's place, and has as many arguments as clippy lets a function have: the warning is a step of its own after it. Add to `impl Changes`:

```rust
    /// The warning that stands on the cell at `at`, over what its state
    /// made of it: the mark, and the words under the pointer after what
    /// the cell was. `doubts` holds only cells that wait for a save with
    /// nothing to fix.
    fn warn(&self, cell: &mut Cell<'_>, at: (usize, usize)) {
        let Some(words) = self.doubts.get(&at).filter(|_| !self.saving) else {
            return;
        };
        cell.mark = Mark::Doubt;
        cell.hint = Some(match cell.hint.take() {
            Some(hint) => format!("{hint}\n{words}"),
            None => words.clone(),
        });
    }
```

   and call `self.warn(cell, at);` right after the call of `self.mark_state(...)` in `mark` and in `new_cell`, with the `(row, col)` each of them has.

4. `error_line`, the terminal's line under the grid. It finds the first cell that is not ready and says it with `!` in the failure's colour, and draws nothing when there is none. Give it the first warning as its second choice. Where `said` is `None` and the function returns after `ui.skip_ahead_auto_ids(1)`, try the warning first:

```rust
    // Nothing to fix: the first warning, in a pending cell's tone.
    let warning = said.is_none().then(|| {
        let workspace = app.workspace(tab)?;
        let object = workspace.object_tab(object_tab)?;
        let page = object.page()?;
        // In the page's order, as the cells to fix are.
        let all = key_picker::doubts(workspace, object, locale);
        let (&(row, col), words) = all.iter().next()?;
        let column = page.columns.get(col)?;
        let row = match crate::edit::new_id(row) {
            Some(_) => look.label(&gettext(locale, "new")),
            None => (object.query.offset + row as u64 + 1).to_string(),
        };
        Some(format!(
            "? {row}:{}  {}",
            format::display_safe(&column.name),
            look.label(words)
        ))
    });
    let warning = warning.flatten();
    let (said, color) = match (said, warning) {
        (Some(said), _) => (Some(said), palette.danger),
        (None, Some(warning)) => (Some(warning), states::Tone::Warning.color(&palette)),
        (None, None) => (None, palette.danger),
    };
```

   and paint the line's text in `color` where it is painted in `palette.danger` today. `said` is the `Option<String>` the function already works out; if it is worked out from `app` in one chained expression, the block above goes right after it. `look.label` lowers the app's own words in the terminal look; the value in them is digits, which it leaves. `use crate::ui::key_picker;` at the top of the file is run 2's.

- [ ] **Step 5: The pending bar**

In `src/ui/pending_bar.rs`, `counts_text` builds "1 new row · 3 changes in 2 rows". Give it the warnings as a third part:

```rust
fn counts_text(counts: crate::edit::Counts, doubts: usize, short: bool, locale: Locale) -> String {
```

and before `parts.join(" · ")`:

```rust
    // What a save may be refused for, and is not held back by.
    if doubts > 0 && !short {
        parts.push(counted(
            locale,
            doubts,
            "value names no row",
            "values name no row",
        ));
    }
```

In `show`, beside `let counts = edits.counts();`:

```rust
    let doubts = app.workspace(tab).map_or(0, |workspace| {
        crate::ui::key_picker::doubts(workspace, object, locale).len()
    });
```

and pass `doubts` in the two calls (`counts_text(counts, doubts, false, locale)` and `counts_text(counts, doubts, true, locale)`). `show` has `object` as a local and no workspace: it is read from `app` as above. Any other caller of `counts_text` the compiler names passes `0`.

- [ ] **Step 6: The row panel's field**

In `src/ui/row_panel.rs`:

1. `struct FieldInfo` gets:

```rust
    /// What the field's cell says when its pending key names no row.
    doubt: Option<String>,
```

2. Where `info` is built (the closure `let info = |name: &str| { ... }`, near line 505), the closure knows the column by name. The warnings are by cell: work them out once before the closure, for the row the panel shows, and look the column up by its place:

```rust
            // The warnings on the row the panel shows, by column.
            let doubts: std::collections::BTreeMap<usize, String> = object
                .map(|object| super::key_picker::doubts(workspace, object, locale))
                .unwrap_or_default()
                .into_iter()
                .filter(|((row, _), _)| *row == cell.row)
                .map(|((_, col), words)| (col, words))
                .collect();
```

   and in the closure's `FieldInfo { ... }`:

```rust
                    doubt: source
                        .columns
                        .iter()
                        .position(|column| column.name == name)
                        .and_then(|col| doubts.get(&col).cloned()),
```

   There `object` is an `Option<&ObjectTab>` (see `object.zip(form.locked)` a few lines up), `workspace` a `&Workspace`, `cell.row` the page's row the panel shows (as `Form::of(workspace, object, tab, cell.row, ...)` reads it) and `source.columns` the columns it draws.

3. `field` works `trouble` out from the pending cell's state (near line 1365) and draws it under the value in the failure's colour (near line 1778). A warning is said in the same place when there is no trouble, in the pending tone. After the block that draws `trouble`:

```rust
    // A warning holds nothing, and is said where a problem would be.
    if let (None, Some(words)) = (&trouble, &info.doubt) {
        ui.add_space(if look.terminal { 2.0 } else { 3.0 });
        let width = ui.available_width();
        Text::one(look, caption(look), words, Tone::Warning.color(palette))
            .wrap(width)
            .layout(ui.ctx())
            .label(ui);
    }
```

   and where `said` (the control's name for a screen reader) gets `changed from`, add after it:

```rust
    if let Some(words) = &info.doubt {
        said = format!("{said}, {words}");
    }
```

   `Tone` is in the file's imports already (the field's `tone` is one).

- [ ] **Step 7: Run the tests**

Run: `~/.cargo/bin/cargo test --locked --lib key_picker`
Expected: PASS.

Then the whole suite. `src/ui/grid.rs` has a test that walks the marks in a table of `(Mark, RowMark, Tone, gutter)` (near line 2356), and checks each one's bar and outline. Add a row for `Mark::Doubt` beside `Mark::Pending`'s, with a changed row's mark and the warning tone, and make the test's check of the outline hold for it too: `Doubt` is outlined as `Trouble` is, and has `Pending`'s bar.

- [ ] **Step 8: Look at it**

Run the app on a table with a foreign key, type an id that no row has into a key's cell and keep it. Check by eye in both looks: the cell is a pending cell with an outline, its hint says the words, the bar counts it and Save is offered, the row panel's field says it, and the terminal look says it under the grid. Save it: the database refuses, and the cell turns to the failure's look. Say in the report what you saw.

- [ ] **Step 9: Run the four checks, then commit**

```bash
git add src/ui/key_picker.rs src/ui/row_panel.rs src/ui/grid.rs src/ui/data_view.rs src/ui/pending_bar.rs src/ui/key_picker_tests.rs && git commit -m "Say when a foreign key names no row

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: The website and the spec

**Files:**
- Modify: `docs/_guide/editing-data.md`
- Modify: `docs/superpowers/specs/2026-10-10-foreign-key-editing-design.md`

- [ ] **Step 1: The guide**

In `docs/_guide/editing-data.md`, at the end of the "Foreign keys" part:

```markdown
An id you type yourself is looked up when you leave the cell. If no row
has it, the cell is outlined and says so, and the bar counts it: "1 value
names no row". It is a warning, not a mistake to fix: you can still save,
and the database decides. A row may be there by then, and a table that
points at itself can name a row you are adding in the same save.
```

and in "Change a value", the sentence that lists what is checked before anything is sent ends with "and the syntax of JSON." Add after it:

```markdown
A foreign key you type is also looked up in the table it points at, as a
warning.
```

- [ ] **Step 2: Check the site**

```bash
cd docs && bundle install && bin/check
```

Expected: passes. If Ruby or the bundle is not there, say so in the report.

- [ ] **Step 3: The spec**

In `docs/superpowers/specs/2026-10-10-foreign-key-editing-design.md`:

1. Replace the status paragraph's "What is left is listed under "Delivery"" sentence and add run 4:

```markdown
Run 4 (`docs/superpowers/plans/2026-10-10-foreign-key-editing-4-missing-row.md`)
is built: a typed key that names no row is warned about. All four runs
are built.
```

2. In section 7, replace the two points that say how the warning looks ("The cell keeps the pending fill and gets the look's caution mark at its start..." and "The words are ... They show where a cell's problem shows...") with what the user agreed to in task 3. If they took the plan's rows a to c as they stand:

```markdown
  - The cell keeps the pending fill and its bar, and gets an outline in
    the pending tone, as a cell to fix has one in the failure's.
  - The words are `No publisher with id 9100000000000000099`, with the
    table's name made singular as the row panel's link makes it. They
    show under the pointer on the cell and under the row panel's field.
    The Omarchy look says the first warning in the line under the grid,
    after any cell to fix: `? 4:publisher_id  no publisher with id …`.
```

3. In the "Implementation notes", the sentence "`edit::Pending` gets the question of section 7 as a field beside `state`. It is not a `State`: it holds nothing." becomes:

```markdown
  The question of section 7 is kept in `Edits::doubts`, by cell, with the
  text that was asked about: a warning that names its text ends by itself
  when the cell's text changes. It is not a `State`: it holds nothing.
```

- [ ] **Step 4: Commit**

```bash
git add docs/_guide/editing-data.md docs/superpowers/specs/2026-10-10-foreign-key-editing-design.md && git commit -m "Say that a foreign key that names no row is warned about

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

## What this leaves

- **Pasting.** When pasting over cells and the paste of rows are built (slice 4 of value editing, and the inserting rows spec), a pasted key is a value "not taken from the list": whatever puts it calls `App::ask_row` for its cell, as `close_editor` does.
- **A save that fails on the key.** Unchanged: `2026-10-07-inserting-rows-design.md` (section 9) words a foreign key failure. A cell that was warned and then refused shows the failure, since `unmatched` leaves out a cell that is not ready.
- **Platform coverage.** The tests run headless and are the same on every platform. Say in the pull request on which platform the warning was looked at, and that the others were only compiled.
