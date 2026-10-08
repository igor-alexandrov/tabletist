# Inserting Rows, Run 2: Add Row in the Grid Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A row can be added to a table from its grid: the Add row button, `Mod+N`, and the terminal look's `o` and `O` make a green new row, its cells are edited as any cell is, the pending bar counts it and holds Save while a required value is missing, Review SQL shows its `INSERT`, and Save writes it in the transaction run 1 built.

**Architecture:** A new row is pending like an edited cell, in the tab's one `Edits`. It has no place in the page, so its cells are kept in `Edits::cells` under a row of its own from `edit::NEW_ROWS` up, and `Edits::added` lists the new rows in the order they stand. Everything that reads a page's row by its number finds none there, so nothing that exists acts on a new row by accident: each path opts in. The grid is drawn through `edit::Order`, which maps the rows it shows (the page's and the new ones among them) to those rows. `edit::change_set` turns the new rows into `ChangeSet::inserts`, `review` lays their `INSERT`s out, and `App::written` puts what the database stored into the page where the new row stood.

**Tech Stack:** Rust 1.98, egui/eframe (the crmne forks), `tabletist-db`. Spec: `docs/superpowers/specs/2026-10-07-inserting-rows-design.md`. Run 1: `docs/superpowers/plans/2026-10-07-inserting-rows-1-save.md`.

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

- `src/shots.rs` is outside those four: it is compiled only with its feature. It calls `Harness::answer_written` and builds `ColumnInfo`s and `Error`s by hand, and task 12 adds scenes to it. Tasks 7 and 12 also run `~/.cargo/bin/cargo clippy --locked --features shots --lib --tests -- -D warnings` ("the shots' clippy").
- **What was run where this plan was written: nothing.** It was written from reading the tree at `90c127b` (v0.2.2) and the design canvas' three boards of "Editing · Inserting rows", not from a draft that was built. Every code block is written against names that exist in that tree, but none was compiled. Treat a block as what the code should come to, and expect to correct a name or a borrow on the first build. Where a block and the compiler disagree, the compiler is right and the task's tests say what must hold. A reviewer then read the plan against the tree, without building it, and traced its tests through the code. It found two faults, both corrected here: the terminal's error line would have named a new row by a number of nineteen digits, and the pending bar was not drawn for a new row with nothing set until two tasks after the row could be added. It also named the existing tests and literals each new field touches, which the tasks now list.
- **Databases.** Only task 4 touches `crates/tabletist-db`, and its tests need no server. The app's tests use the fake backend. Nothing here needs `docker compose`.
- **The branch.** `claude/inserting-rows-2-add-row`, from `90c127b`. This plan is its first commit.
- House rules that bite here: no em dashes anywhere; comments explain why, in the surrounding code's voice; code that depends on the engine matches on `Dialect` and names every variant (no `==`, `!=`, `matches!` or `_` arm); a view pushes `Action`s and changes no state but the text a field edits; views draw text only through `TextRole`s; a view never paints a focus ring; what a user typed never reaches a log (`Edits`' `Debug` prints counts); every behaviour change has a headless test; do not weaken a lint or delete a test to get green.
- **Design material is never a test and never committed.** The canvas is read while building, a throwaway scene of `src/shots.rs` is compared with it by eye (task 12), and no pixel or design check goes into the suite. Screenshots show the Bookshop data only, and none goes to GitHub unblurred.
- Commits are signed, one per task, after its checks pass. If signing fails ("agent refused operation"), do not commit unsigned: stage the task and tell the user. Chain `git add` and `git commit` with `&&`. Each message ends with:

      Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>

## The runs

Run 2 as run 1's plan listed it is too long for one run with a review of each task. It is cut where it first works alone: this plan ends with a row that can be added, filled, reviewed and saved from the grid. The row panel as the "New row" form is its own run after it.

| Run | What it builds | Seen by a user |
|---|---|---|
| 1 (built, v0.2.2) | The save inserts rows: column identity in the structure, `INSERT` in the change set, the three drivers, what comes back | No |
| **2 (this plan)** | Add row in the grid: pending new rows in the one pending set, the Add row button, `Mod+N`, `o`/`O`, the green row with its required, default and assigned cells, dropping a new row, the pending bar, Review SQL, the save and the failed save | Yes |
| 2b | The row panel as the "New row" form: its green header, required fields first, each field edited in place, "Discard new row", "Add another", the note for a table without a primary key | Yes |
| 3 | Duplicate (`Mod+D`, `yy p`) and growing by Down on the last row | Yes |
| 4 | Paste rows with its preview, and the batched multi-row `INSERT` | Yes |
| 5 | Errors and after the save: a failed row's message on its cell by error code, "Open row", `]e`/`[e` and `Mod+'`, "Show in sorted position" and `gs`, the filter note | Yes |

**How long.** Twelve tasks. Run inline (`executing-plans`) with the checks after each task, expect most of a working day of agent time. With a subagent and two reviews for each task, two days: not recommended. If it must be cut shorter, the cut is after task 10: tasks 1 to 10 end at a row that can be added, read, reviewed and saved in every look, and tasks 11 and 12 (the pages and the scenes) can follow as a second pull request. Not earlier: until task 10 the terminal's line does not say why a save of a new row waits.

## What the design asks, and what gets built

Where the app cannot yet do what the canvas and the spec ask, or where they leave a choice. Rows h to p are run 1's, as they stand for this run. **Each is the user's to overrule before task 1.**

| | The design and the spec | What gets built | Why |
|---|---|---|---|
| h | `Mod+N` adds a row | `Mod+N` adds a row while a table's Data view is in front and it can take one, and stays "New connection" elsewhere | The canvas' Keyboard settings scope it: "Connections" for one, "Table" for the other |
| i | macOS: the new row is pinned under the header "regardless of sort, filter or scroll position" | New rows are the grid's first rows, above the page's, and the grid scrolls to one when it is made. They scroll with the body | Read with the spec's "The grid scrolls to the top if needed". A band that stays put while the body scrolls is a second grid inside the grid, with its own keys, editor and reveal |
| i2 | Omarchy: `o` opens the row below the cursor row, `O` above it | The same: the row stands where it was opened | As drawn |
| j | An expression default in italics (`now()`) | Dimmed, not italic | The app has no italic face: text is drawn only through `TextRole`s, and none is italic |
| j2 | A literal default dimmed, in its tag's colours at 60% | The default's text, dimmed, without a tag | A tag is drawn from a value the column holds. One faded tag style for a value it does not hold yet is a new cell style for one case |
| k | A foreign key column opens the search picker | The text editor | The grid has one editor. Editors by type are slice 2 of the value editing spec |
| l | Undo and redo cover creating and dropping new rows | `Mod+Z` and `u` put one cell of a new row back to unset. A dropped new row is not brought back | The undo stack is slice 4 |
| m | Inserts go before the UPDATEs and DELETEs | Before the UPDATEs | The app deletes no rows yet |
| n | Hidden when the user has no INSERT privilege, or for a view without INSERT rules | Add row is disabled, with its reason, for a read-only connection, a view, and while the structure loads. A missing privilege is the server's error on save | The app reads no privileges and edits no view. Disabled and saying why, not hidden: a button that comes and goes moves the header |
| o | "Already used by row id 101 · Open row", "No publisher with id …", each on its cell | The server's message on the new row: its set cells turn red and say it under the pointer, and the bar says "Nothing was saved." with it | Run 5 |
| p | "Save 1 insert and 1 update to bookshop_production?" | "Save 1 new row and 1 change to production?", the connection on the line under it | The prompt says "Save 2 changes to production?" today, and the app's word is "change" |
| q | The inspector is the "New row" form | While a new row is selected the row panel says "A new row is edited in the grid". `Mod+I` does nothing on a new row | Run 2b. The panel reads a row of the page, and a new row is none |
| r | "Discard new row" in the inspector; `⌫` / `dd` drops a new row | `Delete` and `Backspace` on a selected new row drop it in the desktop looks, `dd` in the terminal's. The inspector's button is run 2b | The keys are the spec's (6): "Deleting a new row (⌫ / dd) drops it from the store with no SQL" |
| s | Omarchy's header shows `+1` at its right end | The header's count reads `13 rows + 1 new` in every look, and the terminal's status line says `+1 new` in green | One place for the count in each look. The line is where the terminal says what is pending |
| t | Review SQL groups by kind: `-- 1 new row`, then `-- 1 changed row` | Each new row's `INSERT` under a comment `-- new row`, before the changed rows' statements with the comments they have today | A changed row's comment says what a save compares before it writes. One comment for each statement keeps that |
| u | "Batch up to 100 rows" | One `INSERT` for each row | Run 4, with paste |
| v | Focus on create: the first required column, editor open | The same. A table with no required column gets the selection on its first column that takes a value, and no editor | Nothing is asked of the user there: Save sends `DEFAULT VALUES` |
| w | After a save "the row stays in place" with `RETURNING`'s values | So on a table without triggers. On a table with one, and wherever the driver hands a new row back as unknown, the page is loaded again and the row goes where the sort puts it | Run 1's decision 8: a row is shown only as the database is known to hold it |

## What was decided for this run

1. **One set.** `Edits::cells` holds a new row's values under `(edit::new_row(id), col)`, beside the changed cells. `Edits::added` holds the new rows themselves, since one with nothing set has no cell. `Edits::holds`, `discard`, `put` and `revert` cover both. There is no second store.
2. **A row of its own.** `edit::NEW_ROWS` is `usize::MAX / 2`. A page never has that many rows, so `page.rows.get(row)` is `None` for a new row everywhere: the row panel, the copy, the conflict question and every lock read a new row as no row until they are taught it. The two places that index the page without a check (`edit_cell` and `set_null` in `src/app/editing.rs`) are changed in task 6.
3. **Where a new row stands.** `NewRow::before` is the page's row it stands before (the page's length for one after the last). `Edits::added` keeps the new rows in the order they stand among themselves, so `before` never decreases along it. The desktop looks add at the top, below the new rows already there. The terminal look adds below or above the cursor row.
4. **Shown rows and held rows.** The grid counts the rows it shows from 0. `edit::Order` turns such a place into the row it is (a page's row, or `new_row(id)`) and back. The view maps the selection in and the clicks out; the reducer moves the selection through the same `Order`. `CellPos::row` stays what it is: the row a cell is of, never where it is drawn.
5. **What an unset cell is** (`edit::Unset`): `Assigned` (an identity or computed column), `Default(text)`, `Null`, `Required`. Required is the spec's rule: NOT NULL, no default, not identity, not generated. An identity column is locked in a new row, as row f of run 1's plan had it (`Lock::Assigned`): an id cannot be typed for a new row.
6. **A value of a new row is set or it is not.** There is no loaded value to compare with, so a text typed into a new row's cell is set, the empty text too, and `Mod+Z` (the terminal's `u`) unsets the cell. Set NULL sets NULL.
7. **Save waits for required values.** `SaveBlock::Required` while a new row lacks one. The bar says which column: "1 new row · publisher_id is required", and the disabled Save says "Fill required fields to save".
8. **What a save says back.** Written, with every new row known: each takes its place in the page (`page.rows` grows), every later row's number moves with it (`Edits::gone`, the selection), and the whole new row is green for `SAVED_FOR`. Written, with one unknown or one that does not fit the page: the page is fetched again. `FailedInsert`: the row's set cells become `State::Failed`, `NewRow::failed` holds the error for a row with nothing set, and `Note::FailedInsert` words the bar.
9. **Counts.** `Counts::added` is the new rows. `Counts::changes` and `rows` stay the changed cells of loaded rows and the rows they are in, so every line that says "2 changes in 1 row" today says the same. `to_fix` and `failed` count every pending cell, a new row's too.
10. **No key is needed to add.** `Table::no_rows` is why no row can be added (read-only, not a table, the structure loading, a save, a fetch). A table without a key takes a new row, and after the save its rows are locked as they were (`Lock::NoKey`), which is the spec's test 12.

## The spec's tests, and where each is

| Spec test | In this run |
|---|---|
| 1 `Mod+N` / `o` creates a new row and focuses the first required column; `O` inserts above | Task 6 (`adding_a_row_opens_its_first_required_cell`, `the_terminals_o_and_O_place_the_row`), task 8 (the keys and the button) |
| 2 an empty required column blocks Save; filling it enables Save | Task 7 (`a_new_row_waits_for_its_required_values`) |
| 3 untouched defaults absent from the INSERT; `DEFAULT VALUES` when nothing was set | Task 5 (`a_new_row_sends_only_what_was_set`) |
| 4 identity and generated columns are locked and never in the INSERT | Task 3 (`an_assigned_cell_of_a_new_row_is_locked`), task 5 |
| 8 inserts and updates in one transaction, inserts first; a unique violation leaves all pending with the error | Task 7 (`a_failed_insert_leaves_everything_pending`) |
| 9 after a save the row's values equal what came back | Task 7 (`a_saved_new_row_takes_what_the_database_stored`) |
| 10 dropping a new row generates no SQL | Task 6 (`a_dropped_new_row_leaves_nothing_to_save`). Undo of the drop is not built (row l) |
| 11 read-only connections, views and query results disable every entry point | Task 3 (`no_row_is_added_where_none_can_be`), task 8 |
| 12 a table without a primary key accepts an insert and then reports the row as not editable | Task 7 (`a_table_without_a_key_takes_a_row_it_then_locks`) |
| 13 the SQL for the three engines | Run 1. Task 4 adds where the parts of each stand |
| 5 | Run 3 |
| 6, 7 | Run 4 |

## File map

| File | What changes |
|---|---|
| `src/testing.rs` | The Bookshop's `book_covers`: its structure and a page of it (task 1) |
| `src/edit.rs` | `NEW_ROWS`, `new_row`, `new_id`, `NewRow`, `Place`, `Order`, `Edits::added`, `add_row`, `drop_row`, `Counts::added` (task 2); `Table::added`, `no_rows`, `loaded`, `missing`, `Lock::Assigned`, `Unset`, `unset`, `default_shown` (task 3); `Sent`, `change_set` with inserts (task 5); `Saving::inserts`, `Saved::added`, `Note::FailedInsert`, `Edits::fail_insert` (task 7) |
| `crates/tabletist-db/src/dialect.rs`, `lib.rs` | `InsertParts`, `InsertStatement::parts` (task 4) |
| `src/review.rs`, `src/ui/review.rs` | `Part`, `Blocked`, `Stuck`, `Line::New*`, `Review::added`, the `INSERT` in lines, their comments (task 5) |
| `src/model.rs` | `Action::AddRow`, `Action::DropRow`, `SaveBlock::Required`, `WritePrompt::added` (tasks 6, 7) |
| `src/app.rs`, `src/app/editing.rs` | `add_row`, `drop_row`, the editor and Set NULL on a new row, the selection through `Order` (task 6); the save and its answers (task 7) |
| `src/ui/grid.rs` | `Row`, `Mark::Added` and `Mark::Unset`, `Column::required`, the green row (task 9) |
| `src/ui/data_view.rs` | The grid through `Order`, a new row's cells, the Add row button, the header's count, the footer's note (tasks 8, 9) |
| `src/ui/keys.rs` | `Mod+N` on a table, `o`, `O`, `dd`, `Delete` (task 8) |
| `src/ui/pending_bar.rs`, `src/ui/write_prompts.rs`, `src/ui/workspace.rs`, `src/ui/object_tabs.rs`, `src/ui/row_panel.rs`, `src/ui/cell_editor.rs` | The counts and the words (tasks 3, 8, 10) |
| `docs/_reference/keyboard-shortcuts.md`, `docs/_guide/macos.md`, `docs/_guide/omarchy.md`, `docs/_guide/*` on editing | The keys and the feature (task 11) |
| `src/shots.rs` | A scene with a new row (task 12) |
| `docs/superpowers/specs/2026-10-07-inserting-rows-design.md` | Its status line (task 12) |

---

### Task 1: The Bookshop's `book_covers` for the tests

The fixture table (`users`: `id`, `email`, `meta`) has no default, no column the database numbers and no list of allowed values. The canvas draws inserting on `book_covers`, and the spec's tests name it.

**Files:**
- Modify: `src/testing.rs` (after `fixture_structure`, and in the `impl Harness` that holds `editable`)

- [ ] **Step 1: The structure and a page.** The fake session's driver is SQLite (`connect_fake_as`), so the types are SQLite's. Add after `fixture_structure`:

```rust
/// The structure of the Bookshop's `book_covers`: the database numbers
/// `id`, `publisher_id` must be given, `kind` is one of three and has a
/// default, `image_data` may be NULL, and `created_at` has an expression
/// for a default.
pub fn book_covers_structure() -> tabletist_db::Structure {
    use tabletist_db::ColumnInfo;
    let column = |name: &str, type_name: &str| ColumnInfo {
        name: name.into(),
        type_name: type_name.into(),
        ..ColumnInfo::default()
    };
    tabletist_db::Structure {
        columns: vec![
            ColumnInfo {
                identity: true,
                ..column("id", "INTEGER")
            },
            column("publisher_id", "INTEGER"),
            ColumnInfo {
                default: Some("'print'".into()),
                allowed_values: Some(vec!["print".into(), "ebook".into(), "audio".into()]),
                ..column("kind", "TEXT")
            },
            ColumnInfo {
                nullable: true,
                ..column("image_data", "JSON")
            },
            ColumnInfo {
                default: Some("CURRENT_TIMESTAMP".into()),
                ..column("created_at", "TEXT")
            },
        ],
        primary_key: vec!["id".into()],
        ..tabletist_db::Structure::default()
    }
}

/// A page of `book_covers`, `rows` long.
pub fn book_covers_page(rows: usize) -> RowPage {
    let meta = |name: &str, type_name: &str, kind: ValueKind| ColumnMeta {
        name: name.into(),
        type_name: type_name.into(),
        kind,
    };
    RowPage {
        columns: vec![
            meta("id", "INTEGER", ValueKind::Numeric),
            meta("publisher_id", "INTEGER", ValueKind::Numeric),
            meta("kind", "TEXT", ValueKind::Text),
            meta("image_data", "JSON", ValueKind::Json),
            meta("created_at", "TEXT", ValueKind::Text),
        ],
        rows: (0..rows)
            .map(|index| {
                vec![
                    Value::Int(index as i64 + 1),
                    Value::Int(9_100_000_000_000_000_001 + (index as i64 % 3)),
                    Value::Text(if index % 2 == 0 { "print" } else { "ebook" }.into()),
                    Value::Null,
                    Value::Text("2026-01-12 09:14:03".into()),
                ]
            })
            .collect(),
        has_more: false,
        ordered_by_key: true,
        elapsed: std::time::Duration::from_millis(2),
    }
}
```

- [ ] **Step 2: A tab of it.** In the `impl Harness` that holds `editable`, after it:

```rust
    /// As [`Self::editable`], with the Bookshop's `book_covers` and three of
    /// its rows: a table a row can be added to.
    pub fn book_covers(&mut self) -> (ConnTabId, TabId) {
        let tab = self.connect_fake_as(false);
        self.app.apply(Action::OpenObject {
            tab,
            object: tabletist_db::ObjectRef::new("main", "book_covers"),
            kind: ObjectKind::Table,
            pin: true,
        });
        self.answer_structure(book_covers_structure());
        self.answer_rows(book_covers_page(3));
        let id = self
            .app
            .workspace(tab)
            .and_then(|workspace| workspace.active_tab)
            .expect("the table's tab is open");
        (tab, id)
    }
```

  The tree `connect_fake_as` answers with lists no `book_covers`. `open_object` (`src/app.rs`) opens a tab for the object it is given, listed or not. If it turns out to need the tree's entry, add `book_covers` to the objects of `connect_fake_as` in name order, and correct the tests that count the tree's rows.

- [ ] **Step 3: A test that the fixture is what the tasks lean on.** In `src/testing.rs` there is no test module; put it in `src/edit.rs`'s `mod tests`:

```rust
    #[test]
    fn the_bookshops_covers_have_a_column_of_each_kind() {
        let structure = crate::testing::book_covers_structure();
        let page = crate::testing::book_covers_page(3);
        let names: Vec<&str> = page.columns.iter().map(|c| c.name.as_str()).collect();
        let listed: Vec<&str> = structure.columns.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, listed);
        assert!(page.rows.iter().all(|row| row.len() == names.len()));
        assert_eq!(structure.row_key(), Some(vec!["id".to_owned()]));
    }
```

- [ ] **Step 4: Run it.** `~/.cargo/bin/cargo test --locked --lib the_bookshops_covers` passes. Run the four checks.

- [ ] **Step 5: Commit.**

```bash
git add src/testing.rs src/edit.rs && git commit -m "Give the tests the Bookshop's book_covers"
```

---

### Task 2: New rows in the pending set, and the order the grid shows them in

Pure data in `src/edit.rs`. Nothing calls it yet, so each new item is `pub` and used by this task's tests; clippy's dead-code lint does not fire on `pub` items of the library.

**Files:**
- Modify: `src/edit.rs`

- [ ] **Step 1: Write the failing tests.** In `mod tests` of `src/edit.rs`:

```rust
    #[test]
    fn a_new_row_has_a_row_of_its_own_that_no_page_holds() {
        assert_eq!(new_id(new_row(3)), Some(3));
        assert_eq!(new_id(0), None);
        assert_eq!(new_id(NEW_ROWS - 1), None);
        assert!(page(rows()).rows.get(new_row(0)).is_none());
    }

    /// The ids of the new rows, as they stand.
    fn ids(edits: &Edits) -> Vec<usize> {
        edits.added.iter().map(|new| new.id).collect()
    }

    #[test]
    fn new_rows_stand_where_they_were_put() {
        // The desktop looks: under the header, the newest last.
        let mut edits = Edits::default();
        let first = edits.add_row(Place::Top, 5);
        let second = edits.add_row(Place::Top, 5);
        assert_eq!(ids(&edits), [first, second]);
        assert!(edits.added.iter().all(|new| new.before == 0));
        // The terminal's `o` on the page's row 1: right below it, above a
        // new row that was there already.
        let mut edits = Edits::default();
        let old = edits.add_row(Place::Below(1), 5);
        let new = edits.add_row(Place::Below(1), 5);
        assert_eq!(ids(&edits), [new, old]);
        assert!(edits.added.iter().all(|row| row.before == 2));
        // `O` on the page's row 2: right above it, below those two.
        let above = edits.add_row(Place::Above(2), 5);
        assert_eq!(ids(&edits), [new, old, above]);
        // On a new row: beside it, wherever it stands.
        let under = edits.add_row(Place::Below(new_row(new)), 5);
        assert_eq!(ids(&edits), [new, under, old, above]);
        let over = edits.add_row(Place::Above(new_row(new)), 5);
        assert_eq!(ids(&edits), [over, new, under, old, above]);
        // Below the page's last row is after it, and never past it.
        let last = edits.add_row(Place::Below(4), 5);
        assert_eq!(edits.added.last().map(|row| (row.id, row.before)), Some((last, 5)));
        // The order never goes back up the page.
        assert!(edits.added.windows(2).all(|pair| pair[0].before <= pair[1].before));
        // No id is given twice.
        let mut seen = ids(&edits);
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(seen.len(), edits.added.len());
    }

    #[test]
    fn the_grid_shows_new_rows_among_the_pages() {
        let mut edits = Edits::default();
        let top = edits.add_row(Place::Top, 3);
        let mid = edits.add_row(Place::Below(1), 3);
        let end = edits.add_row(Place::Below(2), 3);
        let order = Order::of(&edits.added, 3);
        assert_eq!(order.len(), 6);
        let shown: Vec<usize> = (0..order.len()).filter_map(|place| order.row(place)).collect();
        assert_eq!(
            shown,
            [new_row(top), 0, 1, new_row(mid), 2, new_row(end)]
        );
        // And back: every row is found where it is shown.
        for (place, row) in shown.iter().enumerate() {
            assert_eq!(order.place(*row), Some(place), "{row}");
        }
        assert_eq!(order.row(6), None);
        assert_eq!(order.place(3), None, "no such row of the page");
        assert_eq!(order.place(new_row(99)), None, "no such new row");
        // With no new row the grid's rows are the page's.
        let plain = Order::of(&[], 3);
        assert_eq!(plain.len(), 3);
        assert_eq!((plain.row(2), plain.place(2)), (Some(2), Some(2)));
    }

    #[test]
    fn a_dropped_new_row_takes_its_cells_and_its_editor() {
        let mut edits = Edits::default();
        let kept = edits.add_row(Place::Top, 2);
        let dropped = edits.add_row(Place::Top, 2);
        let set = |text: &str| Pending {
            new: NewValue::Text(text.into()),
            state: State::Ready,
        };
        edits.put((new_row(kept), 1), set("a"));
        edits.put((new_row(dropped), 1), set("b"));
        edits.put((0, 1), set("c"));
        edits.editor = Some(Editor {
            cell: at(new_row(dropped), 1),
            place: EditorPlace::Grid,
            text: String::new(),
            large: false,
            focus: false,
            top: false,
            touched: false,
            problem: None,
        });
        edits.review = None;
        assert!(edits.drop_row(dropped));
        assert_eq!(ids(&edits), [kept]);
        assert!(edits.editor.is_none());
        let left: Vec<(usize, usize)> = edits.cells.keys().copied().collect();
        assert_eq!(left, [(0, 1), (new_row(kept), 1)]);
        // Not twice, and not a row of the page.
        assert!(!edits.drop_row(dropped));
        assert_eq!(ids(&edits), [kept]);
    }

    #[test]
    fn new_rows_are_counted_apart_from_changed_cells() {
        let mut edits = Edits::default();
        assert!(!edits.holds());
        let id = edits.add_row(Place::Top, 2);
        // A new row with nothing set is still something to keep the page for.
        assert!(edits.holds());
        assert_eq!(
            edits.counts(),
            Counts {
                added: 1,
                ..Counts::default()
            }
        );
        let pending = |state: State| Pending {
            new: NewValue::Text("x".into()),
            state,
        };
        edits.put((new_row(id), 1), pending(State::ToFix(Problem::Number)));
        edits.put((new_row(id), 2), pending(State::Ready));
        edits.put((1, 1), pending(State::Ready));
        edits.put((1, 2), pending(State::Ready));
        // The changes are the loaded rows': what is set in a new row is the
        // row. What is to fix is to fix wherever it is.
        assert_eq!(
            edits.counts(),
            Counts {
                changes: 2,
                rows: 1,
                to_fix: 1,
                failed: 0,
                added: 1,
            }
        );
        edits.discard();
        assert!(edits.added.is_empty() && !edits.holds());
    }
```

- [ ] **Step 2: Run them, and see them fail.** `~/.cargo/bin/cargo test --locked --lib new_row` does not compile: `new_row`, `NEW_ROWS`, `Place`, `Order` and `Edits::added` are not there.

- [ ] **Step 3: The row of a new row.** Above `pub struct Pending`:

```rust
/// The first row that is no row of a page. A new row has no place in the
/// page, so its cells are kept in the pending set under a row of its own,
/// from here up: `page.rows.get(row)` finds nothing there, and whatever
/// reads a page's row by its number takes a new row for no row at all.
pub const NEW_ROWS: usize = usize::MAX / 2;

/// The row the cells of the new row `id` are kept under.
pub fn new_row(id: usize) -> usize {
    NEW_ROWS + id
}

/// The new row whose cells are kept under `row`, by its id. `None` for a
/// row of the page.
pub fn new_id(row: usize) -> Option<usize> {
    row.checked_sub(NEW_ROWS)
}

/// A row the table does not hold yet. What was set in it is among the
/// pending cells, under `new_row(id)`: a new row with nothing set has none.
#[derive(Debug, Clone, PartialEq)]
pub struct NewRow {
    /// Its own among the tab's new rows while the tab holds any.
    pub id: usize,
    /// The page's row it stands before: as many as the page has rows, for
    /// one after the last.
    pub before: usize,
    /// What its `INSERT` failed with in the last save. It is sent again by
    /// the next.
    pub failed: Option<Error>,
}

/// Where a new row is put.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Place {
    /// Under the header, below the new rows already there: the desktop
    /// looks' Add row.
    Top,
    /// Right below this row, a page's or a new one: the terminal's `o`.
    Below(usize),
    /// Right above it: `O`.
    Above(usize),
}

/// The rows a table's grid shows, in its order: the page's, and the new
/// ones among them. A place counts those rows from 0; a row is a page's
/// row, or a new row's own (`new_row`).
#[derive(Clone, Copy)]
pub struct Order<'a> {
    added: &'a [NewRow],
    /// How many rows the page has.
    height: usize,
}

impl<'a> Order<'a> {
    pub fn of(added: &'a [NewRow], height: usize) -> Self {
        Self { added, height }
    }

    pub fn len(&self) -> usize {
        self.height + self.added.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The place of the `index`th new row: under the page's rows before
    /// it and the new rows before it. The new rows are in the order they
    /// stand, so this only grows along them.
    fn place_of(&self, index: usize) -> usize {
        self.added[index].before.min(self.height) + index
    }

    /// The row shown at `place`.
    pub fn row(&self, place: usize) -> Option<usize> {
        if place >= self.len() {
            return None;
        }
        // How many new rows stand above `place`: found by halving, since a
        // grid asks for every cell it draws and a paste can add a thousand.
        let (mut low, mut high) = (0, self.added.len());
        while low < high {
            let middle = (low + high) / 2;
            if self.place_of(middle) < place {
                low = middle + 1;
            } else {
                high = middle;
            }
        }
        match self.added.get(low) {
            Some(new) if self.place_of(low) == place => Some(new_row(new.id)),
            _ => Some(place - low),
        }
    }

    /// Where `row` is shown. `None` for a row that is neither the page's
    /// nor a new one's.
    pub fn place(&self, row: usize) -> Option<usize> {
        match new_id(row) {
            Some(id) => {
                let index = self.added.iter().position(|new| new.id == id)?;
                Some(self.place_of(index))
            }
            None => (row < self.height)
                .then(|| row + self.added.partition_point(|new| new.before <= row)),
        }
    }
}
```

- [ ] **Step 4: The set holds them.** In `pub struct Edits`, after `cells`:

```rust
    /// The rows to add, in the order they stand in the grid among
    /// themselves: `before` never decreases along them.
    pub added: Vec<NewRow>,
    /// The id the next new row takes.
    pub next_new: usize,
```

  In `pub struct Counts`, after `failed`:

```rust
    /// The new rows.
    pub added: usize,
```

  Replace `Edits::holds` and `Edits::counts`, and add `add_row` and `drop_row` to the same `impl`:

```rust
    /// Whether the tab's page must stay: something is pending, an editor
    /// is open, or a save is running.
    pub fn holds(&self) -> bool {
        !self.cells.is_empty()
            || !self.added.is_empty()
            || self.editor.is_some()
            || self.saving.is_some()
    }

    /// Whether anything waits for a save: a changed cell, or a new row.
    /// An open editor alone is not that yet.
    pub fn pending(&self) -> bool {
        !self.cells.is_empty() || !self.added.is_empty()
    }

    pub fn counts(&self) -> Counts {
        let mut counts = Counts {
            added: self.added.len(),
            ..Counts::default()
        };
        let mut last = None;
        for (&(row, _), cell) in &self.cells {
            // What is set in a new row is the row, not a change of one.
            if new_id(row).is_none() {
                counts.changes += 1;
                if last != Some(row) {
                    counts.rows += 1;
                    last = Some(row);
                }
            }
            match cell.state {
                State::Ready => {}
                State::ToFix(_) => counts.to_fix += 1,
                State::Failed(_) => counts.failed += 1,
            }
        }
        // A new row with nothing set has no cell to say that it failed.
        counts.failed += self
            .added
            .iter()
            .filter(|new| new.failed.is_some() && self.unset_all(new.id))
            .count();
        counts
    }

    /// Whether nothing is set in the new row `id`.
    fn unset_all(&self, id: usize) -> bool {
        let row = new_row(id);
        self.cells.range((row, 0)..=(row, usize::MAX)).next().is_none()
    }

    /// Adds a new row at `place` of a page `height` rows long, and says
    /// its id. What was made of the set, its review, is stale.
    pub fn add_row(&mut self, place: Place, height: usize) -> usize {
        // The new row a place names, by where it stands among the new ones.
        let beside = |row: usize| {
            let id = new_id(row)?;
            self.added.iter().position(|new| new.id == id)
        };
        let (at, before) = match place {
            Place::Top => (self.added.partition_point(|new| new.before == 0), 0),
            Place::Below(row) => match beside(row) {
                Some(at) => (at + 1, self.added[at].before),
                None => {
                    let before = row.saturating_add(1).min(height);
                    (self.added.partition_point(|new| new.before < before), before)
                }
            },
            Place::Above(row) => match beside(row) {
                Some(at) => (at, self.added[at].before),
                None => {
                    let before = row.min(height);
                    (self.added.partition_point(|new| new.before <= before), before)
                }
            },
        };
        let id = self.next_new;
        self.next_new += 1;
        self.added.insert(
            at,
            NewRow {
                id,
                before,
                failed: None,
            },
        );
        self.review = None;
        id
    }

    /// Takes the new row `id` out of the set, with what was set in it and
    /// an editor open on it. Says whether there was such a row.
    pub fn drop_row(&mut self, id: usize) -> bool {
        let Some(at) = self.added.iter().position(|new| new.id == id) else {
            return false;
        };
        self.added.remove(at);
        let row = new_row(id);
        self.cells.retain(|&(of, _), _| of != row);
        if self.editor.as_ref().is_some_and(|editor| editor.cell.row == row) {
            self.editor = None;
        }
        if self.why.is_some_and(|(cell, _)| cell.row == row) {
            self.why = None;
        }
        self.review = None;
        true
    }
```

  `Place::Top` puts the row after every new row that stands before the page's first row: `partition_point` is right because `before` never decreases along the list. `Edits::discard` needs no change: it resets everything but `gone`, the new rows with it.

  In `Edits::put` and `Edits::revert`, a new row whose value changes is no longer the row the last save failed on. Add to each, before `self.review = None;`:

```rust
        if let Some(id) = new_id(at.0)
            && let Some(new) = self.added.iter_mut().find(|new| new.id == id)
        {
            new.failed = None;
        }
```

  In the `Debug` of `Edits`, say how many: change the format string to `"Edits {{ cells: {}, added: {}, editor: {:?}, why: {:?}, saving: {}, gone: {:?} }}"` and pass `self.added.len()` after `self.cells.len()`. No test asserts that text.

- [ ] **Step 5: Run the tests.** `~/.cargo/bin/cargo test --locked --lib -- new_row new_rows the_grid_shows` passes. Then the four checks: an existing test that builds `Counts { .. }` in full has none (`Counts` is only built in `counts`), so nothing else changes.

- [ ] **Step 6: Commit.**

```bash
git add src/edit.rs && git commit -m "Keep new rows in the pending set, and say where the grid shows them"
```

---

### Task 3: What a new row's cells are

A new row's cell is locked or free by its column alone, and while nothing is set in it, it says what the database will do with it.

**Files:**
- Modify: `src/edit.rs`
- Modify: `src/ui/cell_editor.rs` (`lock_text`, and its test `every_problem_and_every_lock_has_words`)
- Modify: `src/review.rs` (the `table` helper of its tests)

- [ ] **Step 1: Write the failing tests.** In `mod tests` of `src/edit.rs`:

```rust
    #[test]
    fn an_assigned_cell_of_a_new_row_is_locked() {
        let structure = crate::testing::book_covers_structure();
        let page = crate::testing::book_covers_page(3);
        let mut edits = Edits::default();
        let id = edits.add_row(Place::Top, 3);
        let table = Table {
            added: &edits.added,
            ..table(Some(&structure), &page)
        };
        let cell = |col| at(new_row(id), col);
        assert_eq!(table.row_lock(new_row(id)), None);
        // `id` is the database's to number.
        assert_eq!(table.lock(cell(0)), Some(Lock::Assigned));
        for col in 1..5 {
            assert_eq!(table.lock(cell(col)), None, "{col}");
        }
        assert_eq!(table.lock(cell(5)), Some(Lock::NoSuchCell));
        // A new row the set does not hold is no row.
        assert_eq!(table.lock(at(new_row(id + 1), 1)), Some(Lock::NoSuchCell));
        // It loaded nothing.
        assert_eq!(table.loaded(cell(1)), None);
        assert_eq!(table.loaded(at(0, 0)), Some(&Value::Int(1)));
        // A column the database computes is locked as it is in any row.
        let mut computed = structure.clone();
        computed.columns[4].generated = true;
        let table = Table {
            added: &edits.added,
            ..self::table(Some(&computed), &page)
        };
        assert_eq!(table.lock(cell(4)), Some(Lock::Generated));
    }

    #[test]
    fn no_row_is_added_where_none_can_be() {
        let structure = crate::testing::book_covers_structure();
        let page = crate::testing::book_covers_page(3);
        let ok = || table(Some(&structure), &page);
        assert_eq!(ok().no_rows(), None);
        let read_only = Table {
            access: Access::ReadOnly,
            ..ok()
        };
        assert_eq!(read_only.no_rows(), Some(Lock::ReadOnly));
        let view = Table {
            kind: ObjectKind::View,
            ..ok()
        };
        assert_eq!(view.no_rows(), Some(Lock::NotATable));
        let loading = Table {
            structure: None,
            ..ok()
        };
        assert_eq!(loading.no_rows(), Some(Lock::StructureLoading));
        let saving = Table {
            saving: true,
            ..ok()
        };
        assert_eq!(saving.no_rows(), Some(Lock::Saving));
        let refreshing = Table {
            refreshing: true,
            ..ok()
        };
        assert_eq!(refreshing.no_rows(), Some(Lock::Refreshing));
        // A table without a key takes a row, though no row of its page can
        // be edited: only a changed row is found by its key.
        let mut keyless = structure.clone();
        keyless.primary_key.clear();
        let mut edits = Edits::default();
        let id = edits.add_row(Place::Top, 3);
        let table = Table {
            added: &edits.added,
            ..table(Some(&keyless), &page)
        };
        assert_eq!(table.no_rows(), None);
        assert_eq!(table.lock(at(0, 1)), Some(Lock::NoKey));
        assert_eq!(table.lock(at(new_row(id), 1)), None);
        // A new row is locked with the table while a save runs.
        let saving = Table {
            saving: true,
            ..table
        };
        assert_eq!(saving.lock(at(new_row(id), 1)), Some(Lock::Saving));
    }

    #[test]
    fn an_unset_cell_says_what_the_database_will_do() {
        let structure = crate::testing::book_covers_structure();
        let unsets: Vec<Unset> = structure
            .columns
            .iter()
            .map(|column| unset(Dialect::Sqlite, column))
            .collect();
        assert_eq!(
            unsets,
            [
                Unset::Assigned,
                Unset::Required,
                Unset::Default("print".into()),
                Unset::Null,
                Unset::Default("CURRENT_TIMESTAMP".into()),
            ]
        );
        // NOT NULL with a default of NULL is filled by nothing.
        let odd = ColumnInfo {
            default: Some("NULL".into()),
            ..column("note", "TEXT")
        };
        assert_eq!(unset(Dialect::Sqlite, &odd), Unset::Null);
        let odd = ColumnInfo {
            nullable: false,
            ..odd
        };
        assert_eq!(unset(Dialect::Sqlite, &odd), Unset::Required);
    }

    #[test]
    fn a_default_reads_as_its_value_or_as_its_expression() {
        let shown = |dialect, text: &str| default_shown(dialect, text);
        for dialect in [Dialect::Postgres, Dialect::MySql, Dialect::Sqlite] {
            assert_eq!(shown(dialect, "'print'").as_deref(), Some("print"));
            assert_eq!(shown(dialect, "'it''s'").as_deref(), Some("it's"));
            assert_eq!(shown(dialect, "0").as_deref(), Some("0"));
            assert_eq!(shown(dialect, "now()").as_deref(), Some("now()"));
            assert_eq!(shown(dialect, " NULL ").as_deref(), None);
            // No closing quote: as it is written.
            assert_eq!(shown(dialect, "'open").as_deref(), Some("'open"));
        }
        // PostgreSQL writes a literal with its type.
        let pg = Dialect::Postgres;
        assert_eq!(
            shown(pg, "'print'::character varying").as_deref(),
            Some("print")
        );
        assert_eq!(shown(pg, "'{}'::jsonb").as_deref(), Some("{}"));
        assert_eq!(shown(pg, "NULL::character varying").as_deref(), None);
        // Two strings joined are an expression, shown whole.
        let joined = "'a'::text || 'b'::text";
        assert_eq!(shown(pg, joined).as_deref(), Some(joined));
        // And a cast is no part of a literal elsewhere.
        let cast = "'x'::text";
        assert_eq!(shown(Dialect::Sqlite, cast).as_deref(), Some(cast));
    }

    #[test]
    fn a_new_row_misses_its_required_columns_until_they_are_set() {
        let structure = crate::testing::book_covers_structure();
        let page = crate::testing::book_covers_page(3);
        let mut edits = Edits::default();
        let id = edits.add_row(Place::Top, 3);
        let missing = |edits: &Edits| {
            let table = Table {
                added: &edits.added,
                ..table(Some(&structure), &page)
            };
            table.missing(id, &edits.cells)
        };
        // `publisher_id`, and nothing else: the others are filled.
        assert_eq!(missing(&edits), [1]);
        edits.put(
            (new_row(id), 1),
            Pending {
                new: NewValue::Text("9100000000000000004".into()),
                state: State::Ready,
            },
        );
        assert!(missing(&edits).is_empty());
    }
```

- [ ] **Step 2: Run them, and see them fail.** `~/.cargo/bin/cargo test --locked --lib -- an_assigned_cell no_row_is_added an_unset_cell a_default_reads a_new_row_misses` does not compile: `Table::added`, `no_rows`, `loaded`, `missing`, `Lock::Assigned`, `Unset`, `unset` and `default_shown` are not there.

- [ ] **Step 3: The table knows its new rows.** In `pub enum Lock`, after `Generated`:

```rust
    /// The database numbers the column itself: a new row's cell of it is
    /// given its value by the save.
    Assigned,
```

  In `pub struct Table`, after `gone`:

```rust
    /// The rows to add (`Edits::added`).
    pub added: &'a [NewRow],
```

  In `Table::of`, after `gone: &object.edits.gone,`: `added: &object.edits.added,`.

  Every other place that builds a `Table` in full gets `added: &[],`. There are two: the `table` helper of `src/edit.rs`'s tests and the one of `src/review.rs`'s. The rest are written `Table { .., ..ok() }` and need nothing.

- [ ] **Step 4: Its locks.** In `impl Table<'_>`, at the very start of `row_lock`:

```rust
        if let Some(id) = new_id(row) {
            // Before the key is asked for: a new row is found by nothing
            // yet, and a table without a key takes one.
            return self.no_rows().or_else(|| {
                let held = self.added.iter().any(|new| new.id == id);
                (!held).then_some(Lock::NoSuchCell)
            });
        }
```

  At the very start of `own_lock`:

```rust
        if new_id(cell.row).is_some() {
            return self.new_lock(cell.col);
        }
```

  `lock` stays as it is: `row_lock`, then `own_lock`. Add to the same `impl`:

```rust
    /// Why no row can be added to the table, or `None` when one can. A
    /// table without a key takes a row: only a changed row is found by
    /// its key.
    pub fn no_rows(&self) -> Option<Lock> {
        if self.access == Access::ReadOnly {
            return Some(Lock::ReadOnly);
        }
        if self.kind != ObjectKind::Table {
            return Some(Lock::NotATable);
        }
        if self.structure.is_none() {
            return Some(Lock::StructureLoading);
        }
        if self.saving {
            return Some(Lock::Saving);
        }
        if self.refreshing {
            return Some(Lock::Refreshing);
        }
        None
    }

    /// Why the column `col` takes no value in a new row, or `None` when it
    /// takes one. A key's column takes one like any other: a new row has
    /// no key to keep.
    fn new_lock(&self, col: usize) -> Option<Lock> {
        if col >= self.page.columns.len() {
            return Some(Lock::NoSuchCell);
        }
        let Some(column) = self.column(col) else {
            return Some(Lock::UnknownColumn);
        };
        // Its counter first: an identity column that is always generated
        // is both, and what a user reads is that the save numbers it.
        if column.identity {
            return Some(Lock::Assigned);
        }
        if column.generated {
            return Some(Lock::Generated);
        }
        if column_class(self.dialect, &column.type_name) == ColumnClass::Binary {
            return Some(Lock::Binary);
        }
        None
    }

    /// What `cell` loaded as. `None` for a cell of a new row, which loaded
    /// nothing, and for a cell the page does not hold.
    pub fn loaded(&self, cell: CellPos) -> Option<&Value> {
        self.page.rows.get(cell.row)?.get(cell.col)
    }

    /// The columns the new row `id` still needs a value in, by their place
    /// in the page: the required ones nothing is set in.
    pub fn missing(&self, id: usize, cells: &BTreeMap<(usize, usize), Pending>) -> Vec<usize> {
        let row = new_row(id);
        (0..self.page.columns.len())
            .filter(|&col| !cells.contains_key(&(row, col)))
            .filter(|&col| {
                self.column(col)
                    .is_some_and(|column| unset(self.dialect, column) == Unset::Required)
            })
            .collect()
    }
```

- [ ] **Step 5: What an unset cell is.** After `pub fn opens_large`:

```rust
/// What a cell of a new row holds while nothing is set in it: what the
/// database does with a column an `INSERT` does not name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unset {
    /// The database gives the value: its counter's, or a computed one.
    Assigned,
    /// The column's default as it reads: a literal's value, or the
    /// expression as the database writes it.
    Default(String),
    Null,
    /// Nothing fills it and it cannot be NULL: a save waits for a value.
    Required,
}

/// What `column` holds in a new row that sets nothing in it.
pub fn unset(dialect: Dialect, column: &ColumnInfo) -> Unset {
    if column.identity || column.generated {
        return Unset::Assigned;
    }
    let default = column
        .default
        .as_deref()
        .and_then(|default| default_shown(dialect, default));
    match default {
        Some(text) => Unset::Default(text),
        None if column.nullable => Unset::Null,
        None => Unset::Required,
    }
}

/// A column's default as a new row shows it: a string literal's value,
/// without its quotes and its cast, and anything else (a number, an
/// expression) as the database writes it. `None` for a default of NULL,
/// which is no default.
pub fn default_shown(dialect: Dialect, default: &str) -> Option<String> {
    let text = default.trim();
    let Some(rest) = text.strip_prefix('\'') else {
        // PostgreSQL writes a NULL with its type.
        let bare = match dialect {
            Dialect::Postgres => text.split("::").next().unwrap_or(text),
            Dialect::MySql | Dialect::Sqlite => text,
        };
        return (!bare.eq_ignore_ascii_case("null")).then(|| text.to_owned());
    };
    // To the literal's closing quote: a doubled quote is one of its text.
    let mut value = String::new();
    let mut after = None;
    let mut letters = rest.char_indices().peekable();
    while let Some((at, letter)) = letters.next() {
        if letter != '\'' {
            value.push(letter);
        } else if letters.next_if(|(_, next)| *next == '\'').is_some() {
            value.push('\'');
        } else {
            after = Some(&rest[at + 1..]);
            break;
        }
    }
    // No closing quote: not a literal this reads.
    let Some(after) = after else {
        return Some(text.to_owned());
    };
    // `'print'::character varying`: a cast to a type's name and nothing
    // more. Anything else after the quote makes an expression of it.
    let cast = match dialect {
        Dialect::Postgres => after.strip_prefix("::").is_some_and(|name| {
            !name.is_empty()
                && name
                    .chars()
                    .all(|letter| letter.is_ascii_alphanumeric() || " _.\"(),[]".contains(letter))
        }),
        Dialect::MySql | Dialect::Sqlite => false,
    };
    Some(if after.is_empty() || cast {
        value
    } else {
        text.to_owned()
    })
}
```

- [ ] **Step 6: The lock's words.** In `lock_text` of `src/ui/cell_editor.rs`, after the `Lock::Generated` arm:

```rust
        Lock::Assigned => say("Assigned by the database on save"),
```

  The compiler names every other `match` over `Lock` that has no `_` arm; give each the arm its `Lock::Generated` has. In the test `every_problem_and_every_lock_has_words`, add `Lock::Assigned` to the locks it lists, with its sentence.

- [ ] **Step 7: Run the tests, then the four checks.**

- [ ] **Step 8: Commit.**

```bash
git add src/edit.rs src/review.rs src/ui && git commit -m "Say what a new row's cell is: locked, free, and what fills it unset"
```

---

### Task 4: Where the parts of a shown `INSERT` stand

Review SQL lays an `UPDATE` out in lines and shortens its long values by `UpdateParts`, never by reading the SQL. An `INSERT` has no such parts, and a value can hold ` VALUES (`.

**Files:**
- Modify: `crates/tabletist-db/src/dialect.rs`
- Modify: `crates/tabletist-db/src/lib.rs` (the export)

- [ ] **Step 1: Write the failing test.** In `mod tests` of `crates/tabletist-db/src/dialect.rs`:

```rust
    #[test]
    fn a_new_rows_parts_are_where_its_statement_has_them() {
        use crate::InsertValue;
        let object = ObjectRef::new("public", "book_covers");
        let value = |column: &str, text: &str| InsertValue {
            column: column.into(),
            type_name: "text".into(),
            new: NewValue::Text(text.into()),
        };
        // A value that reads like the statement's own words moves nothing.
        let row = RowInsert {
            set: vec![value("kind", "print"), value("note", ") VALUES ('x') RETURNING *")],
        };
        let nothing = RowInsert { set: Vec::new() };
        for dialect in [Dialect::Postgres, Dialect::MySql, Dialect::Sqlite] {
            let InsertStatement { shown, parts, .. } = dialect.insert_row(&object, &row).unwrap();
            assert!(shown[parts.values..].starts_with("VALUES ("), "{shown}");
            assert!(shown[..parts.values].ends_with(") "), "{shown}");
            let literals: Vec<&str> = parts
                .literals
                .iter()
                .map(|range| &shown[range.clone()])
                .collect();
            assert_eq!(literals, ["'print'", "') VALUES (''x'') RETURNING *'"]);
            let back = parts.back.map(|at| &shown[at..]);
            match dialect {
                Dialect::Postgres | Dialect::Sqlite => {
                    assert_eq!(back, Some("RETURNING *"));
                    assert_eq!(&shown[parts.literals[1].end..], ") RETURNING *");
                }
                Dialect::MySql => {
                    assert_eq!(back, None);
                    assert_eq!(&shown[parts.literals[1].end..], ")");
                }
            }
            // With nothing set: the clause that takes every default.
            let InsertStatement { shown, parts, .. } =
                dialect.insert_row(&object, &nothing).unwrap();
            assert!(parts.literals.is_empty());
            let clause = &shown[parts.values..];
            match dialect {
                Dialect::Postgres | Dialect::Sqlite => {
                    assert_eq!(clause, "DEFAULT VALUES RETURNING *");
                    assert_eq!(parts.back.map(|at| &shown[at..]), Some("RETURNING *"));
                }
                Dialect::MySql => {
                    assert_eq!(clause, "VALUES ()");
                    assert_eq!(parts.back, None);
                }
            }
        }
    }
```

  If an engine writes the second literal otherwise (MySQL escapes with a backslash where its mode asks), keep the test's point and write the literal as that engine's `quote_literal` gives it.

- [ ] **Step 2: Run it, and see it fail.** `~/.cargo/bin/cargo test --locked -p tabletist-db --lib a_new_rows_parts` does not compile: `InsertStatement` has no `parts`.

- [ ] **Step 3: The parts.** After `pub struct InsertStatement`, and add `pub parts: InsertParts,` to it with the doc `/// Where the parts of `shown` stand.`:

```rust
/// Where the parts of a shown `INSERT` stand, as byte offsets into its
/// text, noted as the builder writes it: as [`UpdateParts`], so a name or
/// a value that holds ` VALUES (` cannot move them.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct InsertParts {
    /// Where the clause that gives the values begins: `VALUES`, or
    /// `DEFAULT VALUES`. The table and its columns end a space before it.
    pub values: usize,
    /// Each value's place, in the order its columns are named.
    pub literals: Vec<std::ops::Range<usize>>,
    /// Where `RETURNING` begins, where the statement hands its row back.
    pub back: Option<usize>,
}
```

  In `insert_row`, note them as the text is written. The empty set:

```rust
        let lead = format!("INSERT INTO {table}");
        if row.set.is_empty() {
            let (text, values) = match self {
                Self::Postgres | Self::Sqlite => (format!("{lead} DEFAULT VALUES"), lead.len() + 1),
                Self::MySql => (format!("{lead} () VALUES ()"), lead.len() + 4),
            };
            let parts = InsertParts {
                values,
                literals: Vec::new(),
                // `back` begins with the space before its word.
                back: (!back.is_empty()).then_some(text.len() + 1),
            };
            let text = format!("{text}{back}");
            return Ok(InsertStatement {
                shown: text.clone(),
                sql: Sql {
                    text,
                    params: Vec::new(),
                },
                parts,
            });
        }
```

  And the set with values, in place of the two `format!`s that join `shown` and `sent`:

```rust
        let head = format!("{lead} ({}) VALUES (", names.join(", "));
        let mut parts = InsertParts {
            values: head.len() - "VALUES (".len(),
            ..InsertParts::default()
        };
        let text = format!("{head}{}){back}", sent.join(", "));
        let mut written = head;
        for (index, literal) in shown.iter().enumerate() {
            if index > 0 {
                written.push_str(", ");
            }
            let start = written.len();
            written.push_str(literal);
            parts.literals.push(start..written.len());
        }
        written.push(')');
        parts.back = (!back.is_empty()).then_some(written.len() + 1);
        written.push_str(back);
```

  with `shown: written` and `parts` in the `InsertStatement` returned. Export `InsertParts` from `crates/tabletist-db/src/lib.rs` beside `InsertStatement`.

- [ ] **Step 4: Run the crate's tests.** `~/.cargo/bin/cargo test --locked -p tabletist-db --lib` passes: run 1's `the_same_new_row_on_each_engine` still reads the same `shown`. Run the four checks.

- [ ] **Step 5: Commit.**

```bash
git add crates/tabletist-db && git commit -m "Note where the parts of a shown INSERT stand"
```

---

### Task 5: A save's set and its review carry the new rows

`change_set` makes `inserts` of `Table::added`, and `review` lays each `INSERT` out. The reducer still adds no row, so nothing a user sees changes.

**Files:**
- Modify: `src/edit.rs` (`Sent`, `change_set`)
- Modify: `src/review.rs` (`Part`, `Stuck`, `Blocked`, `Line`, `Review`, `build`, `of`, the layout)
- Modify: `src/ui/review.rs` (`comment`, `comment_color`, `COPIED`)
- Modify: `src/app/editing.rs`, `src/ui/write_prompts.rs` (the callers)

- [ ] **Step 1: Write the failing tests.** In `mod tests` of `src/review.rs` (its `table` helper gained `added: &[]` in task 3):

```rust
    /// The Bookshop's covers with `count` new rows at the top, and their ids.
    fn covers(count: usize) -> (Structure, RowPage, crate::edit::Edits, Vec<usize>) {
        let mut edits = crate::edit::Edits::default();
        let ids = (0..count)
            .map(|_| edits.add_row(crate::edit::Place::Top, 3))
            .collect();
        (
            crate::testing::book_covers_structure(),
            crate::testing::book_covers_page(3),
            edits,
            ids,
        )
    }

    fn covers_ref() -> ObjectRef {
        ObjectRef::new("main", "book_covers")
    }

    #[test]
    fn a_new_row_sends_only_what_was_set() {
        use crate::edit::new_row;
        let (structure, page, mut edits, ids) = covers(2);
        edits.put((new_row(ids[0]), 1), ready("9100000000000000004"));
        edits.put((new_row(ids[1]), 1), ready("9100000000000000007"));
        edits.put((new_row(ids[1]), 2), ready("ebook"));
        // And a changed row, to come after them.
        edits.put((1, 2), ready("audio"));
        let table = Table {
            added: &edits.added,
            ..table(&structure, &page)
        };
        let (changes, sent) = change_set(&covers_ref(), &table, &edits.cells).unwrap();
        assert_eq!(sent.inserts, ids);
        assert_eq!(sent.rows, [1]);
        let named: Vec<Vec<&str>> = changes
            .inserts
            .iter()
            .map(|insert| insert.set.iter().map(|value| value.column.as_str()).collect())
            .collect();
        assert_eq!(named, [vec!["publisher_id"], vec!["publisher_id", "kind"]]);
        assert_eq!(changes.rows.len(), 1);
        let review = build(&covers_ref(), &table, &edits.cells, Values::Shown).unwrap();
        assert_eq!((review.added, review.changes, review.rows), (2, 1, 1));
        assert_eq!(review.refused, None);
        assert_eq!(review.lines[0], Line::New);
        assert_eq!(
            sql(&review)[..6],
            [
                r#"INSERT INTO "main"."book_covers" ("publisher_id")"#,
                "VALUES (9100000000000000004)",
                "RETURNING *;",
                r#"INSERT INTO "main"."book_covers" ("publisher_id", "kind")"#,
                "VALUES (9100000000000000007, 'ebook')",
                "RETURNING *;",
            ]
        );
        // The changed row's statement follows, as it reads today.
        assert_eq!(sql(&review)[6], r#"UPDATE "main"."book_covers""#);
    }

    #[test]
    fn a_new_row_with_nothing_set_takes_every_default() {
        let (mut structure, page, edits, _) = covers(1);
        // Nothing is required of this one.
        structure.columns[1].nullable = true;
        let table = Table {
            added: &edits.added,
            ..table(&structure, &page)
        };
        let review = build(&covers_ref(), &table, &edits.cells, Values::Shown).unwrap();
        assert_eq!((review.added, review.changes, review.rows), (1, 0, 0));
        assert_eq!(
            sql(&review),
            [
                r#"INSERT INTO "main"."book_covers""#,
                "DEFAULT VALUES",
                "RETURNING *;"
            ]
        );
        // MySQL has no such clause, and hands nothing back.
        let (changes, _) = change_set(&covers_ref(), &table, &edits.cells).unwrap();
        let mysql = of(Dialect::MySql, &changes, Blocked::default(), Values::Shown);
        assert_eq!(
            sql(&mysql),
            ["INSERT INTO `main`.`book_covers` ()", "VALUES ();"]
        );
    }

    #[test]
    fn a_new_row_that_cannot_be_sent_is_a_comment() {
        use crate::edit::new_row;
        let (structure, page, mut edits, ids) = covers(2);
        // The first lacks its publisher; the second has a kind to fix.
        edits.put((new_row(ids[1]), 1), ready("9100000000000000004"));
        edits.put(
            (new_row(ids[1]), 2),
            Pending {
                new: NewValue::Text("vinyl".into()),
                state: State::ToFix(Problem::NotOneOf(vec!["print".into()])),
            },
        );
        let table = Table {
            added: &edits.added,
            ..table(&structure, &page)
        };
        let review = build(&covers_ref(), &table, &edits.cells, Values::Shown).unwrap();
        assert_eq!(
            review.lines,
            [
                Line::NewRequired(vec!["publisher_id".into()]),
                Line::NewBlocked(vec!["kind".into()]),
            ]
        );
        assert!(sql(&review).is_empty());
    }

    #[test]
    fn a_table_without_a_key_sends_its_new_rows_and_no_changed_one() {
        use crate::edit::new_row;
        let (mut structure, page, mut edits, ids) = covers(1);
        structure.primary_key.clear();
        edits.put((new_row(ids[0]), 1), ready("9100000000000000004"));
        {
            let table = Table {
                added: &edits.added,
                ..table(&structure, &page)
            };
            let (changes, _) = change_set(&covers_ref(), &table, &edits.cells).unwrap();
            assert_eq!((changes.inserts.len(), changes.rows.len()), (1, 0));
        }
        // A changed cell there is found by no key: nothing can be sent.
        edits.put((0, 2), ready("audio"));
        let table = Table {
            added: &edits.added,
            ..table(&structure, &page)
        };
        assert!(change_set(&covers_ref(), &table, &edits.cells).is_none());
        let review = build(&covers_ref(), &table, &edits.cells, Values::Shown).unwrap();
        assert_eq!(review.lines, [Line::Unsendable]);
        assert_eq!((review.added, review.changes, review.rows), (1, 1, 1));
    }
```

  `unlaid` in that module joins a statement's lines until one starts with `UPDATE`; make it start a statement at `INSERT` too, so the existing tests that use it stay true once a review holds both.

- [ ] **Step 2: Run them, and see them fail.** `~/.cargo/bin/cargo test --locked --lib review::tests` does not compile.

- [ ] **Step 3: The set.** In `src/edit.rs`, import `InsertValue` and `RowInsert` from `tabletist_db`, and replace `change_set` and its doc:

```rust
/// Where each part of a change set came from. An answer names a part by
/// its place in the set.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Sent {
    /// The id of the new row of each of the set's `inserts`, in its order.
    pub inserts: Vec<usize>,
    /// The page's row of each of the set's `rows`.
    pub rows: Vec<usize>,
}

/// The change set a save sends for the table's new rows and the pending
/// `cells`, and where each part of it came from. `None` when nothing is
/// pending, or a row is changed in a table that has no key.
pub fn change_set(
    object: &ObjectRef,
    table: &Table<'_>,
    cells: &BTreeMap<(usize, usize), Pending>,
) -> Option<(ChangeSet, Sent)> {
    let mut sent = Sent::default();
    // The new rows as they stand, each with what was set in it, by column.
    let mut inserts = Vec::new();
    for new in table.added {
        let row = new_row(new.id);
        let set = cells
            .range((row, 0)..=(row, usize::MAX))
            .map(|(&(_, col), pending)| {
                let column = table.column(col)?;
                Some(InsertValue {
                    column: column.name.clone(),
                    type_name: column.type_name.clone(),
                    new: pending.new.clone(),
                })
            })
            .collect::<Option<Vec<_>>>()?;
        inserts.push(RowInsert { set });
        sent.inserts.push(new.id);
    }
    // The cells of the page's rows: every row below the new ones'.
    let changed = cells.range(..(NEW_ROWS, 0));
    // The key only where a row is changed: a new row is found by none.
    let key = match changed.clone().next() {
        Some(_) => table.key()?,
        None => Vec::new(),
    };
    let mut rows: Vec<RowChange> = Vec::new();
    // The map is ordered by row, then column, so a row's cells are together.
    for (&(row, col), pending) in changed {
        let values = table.page.rows.get(row)?;
        let column = table.column(col)?;
        if sent.rows.last() != Some(&row) {
            sent.rows.push(row);
            rows.push(RowChange {
                key: key
                    .iter()
                    .map(|&place| {
                        let name = table.page.columns.get(place)?.name.clone();
                        Some((name, values.get(place)?.clone()))
                    })
                    .collect::<Option<_>>()?,
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
    (!rows.is_empty() || !inserts.is_empty()).then(|| {
        (
            ChangeSet {
                object: object.clone(),
                inserts,
                rows,
            },
            sent,
        )
    })
}
```

  Its callers in `src/app/editing.rs` (`save_blocked`, `write_edits`, `confirm_write`) bind the pair as `(changes, sent)` and go on with `sent.rows` wherever they had `rows`. Task 7 gives them the rest. The test of `src/edit.rs` that binds `(changes, places)` and asserts `places == [0, 1]` asserts `sent.rows` instead.

- [ ] **Step 4: The review's types.** In `src/review.rs`, import `InsertStatement` from `tabletist_db` and `NEW_ROWS`, `new_row` from `crate::edit`. Add after `pub enum Values`:

```rust
/// A part of a change set: a new row or a changed one, by its place among
/// its kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
    Insert(usize),
    Row(usize),
}

/// What holds a new row's statement back, by its columns' names.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Stuck {
    pub to_fix: Vec<String>,
    /// The required columns nothing is set in.
    pub required: Vec<String>,
}

/// What holds statements back, for each part of a set in its order. A part
/// with nothing listed has its statement.
#[derive(Debug, Clone, Copy, Default)]
pub struct Blocked<'a> {
    pub inserts: &'a [Stuck],
    /// The columns to fix of each changed row.
    pub rows: &'a [Vec<String>],
}
```

  In `pub enum Line`, before `Sql`:

```rust
    /// `-- new row`: the row the `INSERT` under it adds.
    New,
    /// `-- new row · blocked: fix kind first`.
    NewBlocked(Vec<String>),
    /// `-- new row · blocked: publisher_id is required`.
    NewRequired(Vec<String>),
    /// The builder refuses a value of the new row, in its own words.
    NewRefused(String),
```

  In `pub struct Review`, after `rows`: `/// How many rows the set adds.` `pub added: usize,`. Its `refused` becomes `Option<(Part, Error)>`, with the doc "The first part whose statement the builder refused, and why." Its `Debug` prints `added` between `rows` and `lines`. The `Review { .. }` built by hand in `src/edit.rs`'s tests gets `added: 0`, and the test of `src/review.rs` that asserts the `Debug` text `"Review { changes: 2, rows: 1, lines: 7 }"` asserts it with `added: 0` in its place.

- [ ] **Step 5: `build` and `of`.** Replace both:

```rust
/// The review of the table's new rows and the pending `cells`. `None`
/// when nothing is pending.
pub fn build(
    object: &ObjectRef,
    table: &Table<'_>,
    cells: &BTreeMap<(usize, usize), Pending>,
    values: Values,
) -> Option<Review> {
    if cells.is_empty() && table.added.is_empty() {
        return None;
    }
    let Some((changes, sent)) = change_set(object, table, cells) else {
        // The map is ordered by row: each row's cells are together. The
        // changes are the page's rows', as everywhere they are counted.
        let mut rows: Vec<usize> = cells
            .range(..(NEW_ROWS, 0))
            .map(|(&(row, _), _)| row)
            .collect();
        let changes = rows.len();
        rows.dedup();
        return Some(Review {
            changes,
            rows: rows.len(),
            added: table.added.len(),
            lines: vec![Line::Unsendable],
            refused: None,
        });
    };
    let name = |col: usize| {
        let column = table.page.columns.get(col)?;
        Some(format::display_safe(&column.name).into_owned())
    };
    // The columns to fix of a row, a page's or a new one's.
    let to_fix = |row: usize| -> Vec<String> {
        cells
            .range((row, 0)..=(row, usize::MAX))
            .filter(|(_, cell)| matches!(cell.state, State::ToFix(_)))
            .filter_map(|(&(_, col), _)| name(col))
            .collect()
    };
    let inserts: Vec<Stuck> = sent
        .inserts
        .iter()
        .map(|&id| Stuck {
            to_fix: to_fix(new_row(id)),
            required: table.missing(id, cells).into_iter().filter_map(name).collect(),
        })
        .collect();
    let rows: Vec<Vec<String>> = sent.rows.iter().map(|&row| to_fix(row)).collect();
    let blocked = Blocked {
        inserts: &inserts,
        rows: &rows,
    };
    Some(of(table.dialect, &changes, blocked, values))
}

/// The review of `changes`, as `dialect` writes them: the new rows first,
/// as a save writes them first. A part `blocked` lists something for is a
/// comment only.
pub fn of(dialect: Dialect, changes: &ChangeSet, blocked: Blocked<'_>, values: Values) -> Review {
    let mut lines = Vec::new();
    let mut refused = None;
    for (index, insert) in changes.inserts.iter().enumerate() {
        let stuck = blocked.inserts.get(index);
        if let Some(stuck) = stuck.filter(|stuck| !stuck.to_fix.is_empty()) {
            lines.push(Line::NewBlocked(stuck.to_fix.clone()));
            continue;
        }
        if let Some(stuck) = stuck.filter(|stuck| !stuck.required.is_empty()) {
            lines.push(Line::NewRequired(stuck.required.clone()));
            continue;
        }
        match dialect.insert_row(&changes.object, insert) {
            Ok(statement) => {
                lines.push(Line::New);
                lines.extend(new_statement(&statement, values));
            }
            Err(error) => {
                let reason = format::capped(&error.to_string()).into_owned();
                lines.push(Line::NewRefused(
                    format::escape_hidden(&reason).into_owned(),
                ));
                refused.get_or_insert((Part::Insert(index), error));
            }
        }
    }
    for (index, row) in changes.rows.iter().enumerate() {
        // As it is today, with `blocked.rows` for `blocked` and
        // `(Part::Row(index), error)` for `(index, error)`.
    }
    Review {
        changes: changes.rows.iter().map(|row| row.set.len()).sum(),
        rows: changes.rows.len(),
        added: changes.inserts.len(),
        lines,
        refused,
    }
}
```

  The loop over `changes.rows` keeps its body, with the two substitutions its comment names. The other callers of `of` (`write_edits` in `src/app/editing.rs`, the sheet's Copy SQL in `src/ui/write_prompts.rs`, and the tests in `src/review.rs` and `src/ui/review.rs`) pass `Blocked::default()` where they passed `&[]`. A test that passed a list of blocked rows passes `Blocked { rows: &list, ..Blocked::default() }`. `write_edits` reads `refused` as a `Part` now: until task 7, send `Part::Row(index)` where it sent `index`, and treat `Part::Insert(_)` as `object.edits.fail(None, error)`.

- [ ] **Step 6: The `INSERT` in lines.** After `fn lay`:

```rust
/// A new row's statement in lines, as the design lays one out: the table
/// and its columns, the values, and what it hands back. Only white space
/// between its clauses is changed.
fn new_statement(statement: &InsertStatement, values: Values) -> Vec<Line> {
    lay_new(statement, values).unwrap_or_else(|| {
        // Not the parts this was written for: the statement on one line.
        vec![Line::Sql(vec![
            plain(&statement.shown, values),
            fixed(Ink::Plain, ";"),
        ])]
    })
}

fn lay_new(statement: &InsertStatement, values: Values) -> Option<Vec<Line>> {
    let InsertStatement { shown, parts, .. } = statement;
    let into = shown.get(..parts.values)?.strip_prefix("INSERT INTO")?;
    let mut lines = vec![Line::Sql(vec![
        fixed(Ink::Keyword, "INSERT INTO"),
        plain(into.trim_end(), values),
    ])];
    // The clause ends a space before `RETURNING`, or with the statement.
    let end = match parts.back {
        Some(back) => back.checked_sub(1)?,
        None => shown.len(),
    };
    let clause = shown.get(parts.values..end)?;
    let word = ["DEFAULT VALUES", "VALUES"]
        .into_iter()
        .find(|word| clause.starts_with(word))?;
    let mut pieces = vec![fixed(Ink::Keyword, word)];
    let mut at = parts.values + word.len();
    for range in &parts.literals {
        pieces.push(plain(shown.get(at..range.start)?, values));
        pieces.push(value(shown.get(range.clone())?, values));
        at = range.end;
    }
    let rest = shown.get(at..end)?;
    if !rest.is_empty() {
        pieces.push(plain(rest, values));
    }
    let Some(back) = parts.back else {
        pieces.push(fixed(Ink::Plain, ";"));
        lines.push(Line::Sql(pieces));
        return Some(lines);
    };
    lines.push(Line::Sql(pieces));
    let handed = shown.get(back..)?.strip_prefix("RETURNING")?;
    lines.push(Line::Sql(vec![
        fixed(Ink::Keyword, "RETURNING"),
        plain(handed, values),
        fixed(Ink::Plain, ";"),
    ]));
    Some(lines)
}
```

  `Ink::Keyword`'s doc lists the words it colours: add `INSERT INTO`, `VALUES`, `RETURNING`.

- [ ] **Step 7: The comments.** In `comment` of `src/ui/review.rs`, before the `Line::Sql` arm:

```rust
        Line::New => format!("-- {}", say("new row")),
        Line::NewBlocked(columns) => format!(
            "-- {} · {} {} {}",
            say("new row"),
            say("blocked: fix"),
            columns.join(", "),
            say("first")
        ),
        Line::NewRequired(columns) => {
            let plural = u32::try_from(columns.len()).unwrap_or(u32::MAX);
            format!(
                "-- {} · {} {} {}",
                say("new row"),
                say("blocked:"),
                columns.join(", "),
                ngettext(locale, "is required", "are required", plural)
            )
        }
        Line::NewRefused(reason) => {
            format!("-- {} · {} {reason}", say("new row"), say("cannot be sent:"))
        }
```

  In `comment_color`, `Line::New` goes with `Line::Row` and the other three with `Line::Blocked`. `COPIED` says "Each statement runs only while its row is still as the comment above it says", which no `INSERT` does: make it "What Tabletist runs to save these changes, in one transaction. Each UPDATE runs only while its row is still as the comment above it says." and correct the tests that assert the old sentence. Add to `mod tests` of `src/ui/review.rs`:

```rust
    #[test]
    fn a_new_rows_comments_say_what_holds_it() {
        let say = |line: &Line| comment(line, Locale::English).unwrap();
        assert_eq!(say(&Line::New), "-- new row");
        assert_eq!(
            say(&Line::NewBlocked(vec!["kind".into()])),
            "-- new row · blocked: fix kind first"
        );
        assert_eq!(
            say(&Line::NewRequired(vec!["publisher_id".into()])),
            "-- new row · blocked: publisher_id is required"
        );
        assert_eq!(
            say(&Line::NewRequired(vec!["isbn".into(), "title".into()])),
            "-- new row · blocked: isbn, title are required"
        );
        assert_eq!(
            say(&Line::NewRefused("kind: TEXT expects text".into())),
            "-- new row · cannot be sent: kind: TEXT expects text"
        );
    }
```

- [ ] **Step 8: Run the tests, then the four checks.**

- [ ] **Step 9: Commit.**

```bash
git add src crates && git commit -m "Make INSERTs of the new rows a save's set holds, and lay them out for review"
```

---

### Task 6: The reducer adds a row, edits it and drops it

From here a new row exists in the app, by `Action` only: no key and no button sends one until task 8.

**Files:**
- Modify: `src/model.rs` (`Action::AddRow`, `Action::DropRow`, `ObjectTab::off_new_rows`, the doc of `CellPos`)
- Modify: `src/app.rs` (the two arms, `MoveSelection`, `DiscardEdits`, `LeaveDiscard`)
- Modify: `src/app/editing.rs` (`dropped_under_a_prompt`, `typed`, `unwritten`, `edit_cell`, `set_null`, `focus_fields`, `add_row`, `drop_row`)
- Test: `src/app.rs`, `mod tests` → `mod editing`

- [ ] **Step 1: Write the failing tests.** In `mod editing` of `src/app.rs`'s tests, with `use crate::edit::{Place, new_row};` added to its imports:

```rust
        /// The Bookshop's covers with a new row at the top, and its id.
        fn with_new_row() -> (Harness, ConnTabId, TabId, usize) {
            let mut harness = Harness::new();
            let (tab, id) = harness.book_covers();
            let place = Place::Top;
            harness.app.apply(Action::AddRow { tab, id, place });
            let new = object(&harness, tab, id).edits.added[0].id;
            (harness, tab, id, new)
        }

        #[test]
        fn adding_a_row_opens_its_first_required_cell() {
            let (harness, tab, id, new) = with_new_row();
            let tab_now = object(&harness, tab, id);
            assert_eq!(tab_now.edits.added.len(), 1);
            // `publisher_id`: the one column a save needs a value in.
            let cell = at(new_row(new), 1);
            assert_eq!(tab_now.selection, Some(cell));
            let editor = tab_now.edits.editor.as_ref().expect("an editor");
            assert_eq!(
                (editor.cell, editor.text.as_str(), editor.touched),
                (cell, "", false)
            );
            // Nothing is set by opening it, and the page stays for the row.
            assert!(tab_now.edits.cells.is_empty());
            assert!(tab_now.edits.holds() && tab_now.pinned);
        }

        #[test]
        fn a_row_nothing_is_required_of_gets_the_selection_and_no_editor() {
            let mut harness = Harness::new();
            let (tab, id) = harness.book_covers();
            let object_tab = harness.app.workspace_mut(tab).unwrap().object_tab_mut(id);
            let structure = object_tab.unwrap().structure.value.as_mut().unwrap();
            structure.columns[1].nullable = true;
            let place = Place::Top;
            harness.app.apply(Action::AddRow { tab, id, place });
            let tab_now = object(&harness, tab, id);
            let new = tab_now.edits.added[0].id;
            // The first column that takes a value: `id` is the database's.
            assert_eq!(tab_now.selection, Some(at(new_row(new), 1)));
            assert!(tab_now.edits.editor.is_none());
        }

        #[test]
        fn the_terminals_o_and_O_place_the_row() {
            let mut harness = Harness::new();
            let (tab, id) = harness.book_covers();
            for place in [Place::Below(1), Place::Above(1)] {
                harness.app.apply(Action::AddRow { tab, id, place });
            }
            let tab_now = object(&harness, tab, id);
            let (above, below) = (tab_now.edits.added[0].id, tab_now.edits.added[1].id);
            let order = crate::edit::Order::of(&tab_now.edits.added, 3);
            let shown: Vec<usize> = (0..order.len()).filter_map(|at| order.row(at)).collect();
            assert_eq!(shown, [0, new_row(above), 1, new_row(below), 2]);
            // The second was asked for while the first one's editor was
            // open: that one closed with nothing set.
            assert!(tab_now.edits.cells.is_empty());
            assert_eq!(tab_now.selection, Some(at(new_row(above), 1)));
        }

        #[test]
        fn a_value_typed_into_a_new_row_is_set_and_can_be_unset() {
            let (mut harness, tab, id, new) = with_new_row();
            let row = new_row(new);
            type_into(&mut harness, tab, id, at(row, 1), "9100000000000000004");
            let edits = &object(&harness, tab, id).edits;
            let set = edits.cells.get(&(row, 1)).expect("a value");
            assert_eq!(set.new, NewValue::Text("9100000000000000004".into()));
            assert_eq!(set.state, State::Ready);
            // The row is counted, and what is set in it is no change.
            let counts = edits.counts();
            assert_eq!((counts.added, counts.changes, counts.rows), (1, 0, 0));
            // A text its column refuses is kept to fix, as in any cell.
            type_into(&mut harness, tab, id, at(row, 1), "harbor");
            harness.app.apply(crate::testing::leave_edit(&harness.app, tab, id));
            let fix = &object(&harness, tab, id).edits.cells[&(row, 1)];
            assert!(matches!(fix.state, State::ToFix(_)), "{:?}", fix.state);
            // Reverting unsets it: there is nothing it loaded to go back to.
            harness.app.apply(Action::SelectCell {
                tab,
                id,
                cell: at(row, 1),
            });
            let cell = None;
            harness.app.apply(Action::RevertCell { tab, id, cell });
            assert!(object(&harness, tab, id).edits.cells.is_empty());
            // NULL is a value where the column takes it, and nowhere else.
            for (col, nulled) in [(3, true), (1, false)] {
                let cell = at(row, col);
                harness.app.apply(Action::SelectCell { tab, id, cell });
                harness.app.apply(Action::SetNull { tab, id });
                let set = object(&harness, tab, id).edits.cells.get(&(row, col));
                assert_eq!(set.map(|set| &set.new), nulled.then_some(&NewValue::Null));
            }
            // The database numbers `id`: asked for, the cell says so.
            harness.app.apply(Action::EditCell {
                tab,
                id,
                cell: at(row, 0),
                start: EditStart::Value,
            });
            let tab_now = object(&harness, tab, id);
            assert!(tab_now.edits.editor.is_none());
            assert_eq!(tab_now.edits.why, Some((at(row, 0), Lock::Assigned)));
        }

        #[test]
        fn the_selection_walks_through_the_rows_as_they_are_shown() {
            let (mut harness, tab, id, new) = with_new_row();
            harness.app.apply(Action::CancelEdit { tab, id });
            let walk = |harness: &mut Harness, rows: isize| {
                let cols = 0;
                harness.app.apply(Action::MoveSelection {
                    tab,
                    id,
                    rows,
                    cols,
                });
                object(harness, tab, id).selection.map(|cell| cell.row)
            };
            // Down from the new row is the page's first, and back.
            assert_eq!(walk(&mut harness, 1), Some(0));
            assert_eq!(walk(&mut harness, -1), Some(new_row(new)));
            assert_eq!(walk(&mut harness, -1), Some(new_row(new)), "the top");
            assert_eq!(walk(&mut harness, isize::MAX), Some(2));
            assert_eq!(walk(&mut harness, isize::MIN), Some(new_row(new)));
        }

        #[test]
        fn a_dropped_new_row_leaves_nothing_to_save() {
            let (mut harness, tab, id, new) = with_new_row();
            type_into(&mut harness, tab, id, at(new_row(new), 1), "9100000000000000004");
            harness.app.apply(Action::DropRow { tab, id });
            let tab_now = object(&harness, tab, id);
            assert!(tab_now.edits.added.is_empty() && tab_now.edits.cells.is_empty());
            assert!(!tab_now.edits.holds());
            // The selection stays where the row stood: on the row now there.
            assert_eq!(tab_now.selection, Some(at(0, 1)));
            let before = harness.app.backend.sent.len();
            harness.app.apply(Action::WriteEdits { tab, id });
            assert!(write_since(&harness, before).is_none());
            // On a row of the page it drops nothing.
            harness.app.apply(Action::DropRow { tab, id });
            assert_eq!(object(&harness, tab, id).page().unwrap().rows.len(), 3);
        }

        #[test]
        fn discarding_drops_the_new_rows_and_a_selection_on_one() {
            let (mut harness, tab, id, _) = with_new_row();
            harness.app.apply(Action::DiscardEdits { tab, id });
            let tab_now = object(&harness, tab, id);
            assert!(tab_now.edits.added.is_empty() && tab_now.edits.editor.is_none());
            assert_eq!(tab_now.selection, None);
        }

        #[test]
        fn no_row_is_added_where_the_table_takes_none() {
            let place = Place::Top;
            // A read-only connection.
            let mut harness = Harness::new();
            let tab = harness.connect_fake();
            let id = open(&mut harness, tab, "users", true);
            harness.answer_structure(crate::testing::fixture_structure());
            harness.answer_rows(page(5, false));
            harness.app.apply(Action::AddRow { tab, id, place });
            assert!(object(&harness, tab, id).edits.added.is_empty());
            // A view, the Structure view, and a table that is saving.
            fn tab_mut(
                harness: &mut Harness,
                tab: ConnTabId,
                id: TabId,
            ) -> &mut crate::model::ObjectTab {
                let workspace = harness.app.workspace_mut(tab).unwrap();
                workspace.object_tab_mut(id).unwrap()
            }
            let mut harness = Harness::new();
            let (tab, id) = harness.book_covers();
            tab_mut(&mut harness, tab, id).kind = ObjectKind::View;
            harness.app.apply(Action::AddRow { tab, id, place });
            assert!(object(&harness, tab, id).edits.added.is_empty());
            tab_mut(&mut harness, tab, id).kind = ObjectKind::Table;
            tab_mut(&mut harness, tab, id).view = ObjectView::Structure;
            harness.app.apply(Action::AddRow { tab, id, place });
            assert!(object(&harness, tab, id).edits.added.is_empty());
            // And where it takes one, it does.
            tab_mut(&mut harness, tab, id).view = ObjectView::Data;
            harness.app.apply(Action::AddRow { tab, id, place });
            assert_eq!(object(&harness, tab, id).edits.added.len(), 1);
        }

        #[test]
        fn leaving_a_tab_with_a_new_row_is_asked_about() {
            let (mut harness, tab, id, _) = with_new_row();
            harness.app.apply(Action::CloseTab { tab, id });
            // One new row is one thing that would be lost.
            assert_eq!(asked_about(&harness), 1);
            assert!(object(&harness, tab, id).edits.holds());
            // And under the question no row is added.
            let place = Place::Top;
            harness.app.apply(Action::AddRow { tab, id, place });
            assert_eq!(object(&harness, tab, id).edits.added.len(), 1);
        }
```

  `type_into`, `write_since`, `asked_about`, `at`, `object`, `open` and `page` are that module's and its parent's own helpers.

- [ ] **Step 2: Run them, and see them fail.** `~/.cargo/bin/cargo test --locked --lib app::tests::editing` does not compile: there is no `Action::AddRow`.

- [ ] **Step 3: The actions.** In `pub enum Action` of `src/model.rs`, after `SetNull`:

```rust
    /// Add a new row to the tab's table at `place`, and open the editor on
    /// its first cell a save needs a value in.
    AddRow {
        tab: ConnTabId,
        id: TabId,
        place: crate::edit::Place,
    },
    /// Drop the selected row where it is a new one, with what was set in
    /// it. A row of the page stays.
    DropRow {
        tab: ConnTabId,
        id: TabId,
    },
```

  Change the doc of `CellPos` to: "A cell of an object tab's page or of a SQL result: its row there, and its column. In a table's tab the row can be a new row's own instead (`edit::new_row`), which no page holds." Add to `impl ObjectTab`:

```rust
    /// Takes the selection off a new row: the set that held it was dropped.
    pub fn off_new_rows(&mut self) {
        if self
            .selection
            .is_some_and(|cell| crate::edit::new_id(cell.row).is_some())
        {
            self.selection = None;
        }
    }
```

  In `dropped_under_a_prompt` (`src/app/editing.rs`), add `| Action::AddRow { .. }` and `| Action::DropRow { .. }` after `Action::SetNull { .. }`.

- [ ] **Step 4: The editor and Set NULL on a new row.** In `src/app/editing.rs`:

  `typed` reads what the cell loaded, and a new row's loaded nothing. Replace its `let loaded = …;` line and the `changed` line:

```rust
    // A new row's cell loaded nothing: a text typed into it is set, the
    // empty one too. A cell that is neither a page's nor a new row's is
    // no cell any more.
    let loaded = table.loaded(cell);
    if loaded.is_none() && crate::edit::new_id(cell.row).is_none() {
        return None;
    }
```

```rust
    let changed = loaded.is_none_or(|loaded| is_change(loaded, &new, class));
```

  (the `if !editor.touched { return None; }` between them stays). In `edit_cell`, the arm that indexes the page:

```rust
                    None => table
                        .loaded(cell)
                        .map(|loaded| start_text(loaded, class))
                        .unwrap_or_default(),
```

  In `set_null`, the last line of its closure:

```rust
            // A new row's cell loaded nothing: NULL is a value set in it.
            let changed = table.loaded(cell).is_none_or(|loaded| !loaded.is_null());
            Some((cell, changed))
```

  In `focus_fields`, right after `let cell = object.selection?;`:

```rust
            // The panel has no form for a new row yet (run 2b).
            if crate::edit::new_id(cell.row).is_some() {
                return None;
            }
```

  In `unwritten`, a new row is one more thing that would be lost, and the open editor changes the count only on a row of the page (what is typed into a new row is part of that row):

```rust
        let closed = self
            .table(tab, id, typed)
            .flatten()
            .filter(|typed| crate::edit::new_id(typed.cell.row).is_none());
```

  and its last line becomes `(changes + edits.counts().added).max(1)`. Read `edits.counts()` once into a `let counts`.

- [ ] **Step 5: Adding and dropping.** Add to `impl App` in `src/app/editing.rs` (import `Place`, `Order`, `new_id`, `new_row` from `crate::edit` and `ObjectView` from `crate::model`):

```rust
    /// Adds a new row to the tab's table at `place`, where it takes one,
    /// and opens the editor on the first cell a save needs a value in.
    /// With no such cell the selection goes to the first that takes a
    /// value, and nothing opens: nothing is asked of the user there.
    pub(super) fn add_row(&mut self, tab: ConnTabId, id: TabId, place: Place) {
        // An editor open on another cell keeps its text.
        self.close_editor(tab, id, true);
        // Only where the rows show, and only a table that takes one.
        let takes = self.table(tab, id, |table, object| {
            object.view == ObjectView::Data && table.no_rows().is_none()
        });
        if takes != Some(true) {
            return;
        }
        let Some(object) = self.object_tab_mut(tab, id) else {
            return;
        };
        let height = object.page().map_or(0, |page| page.rows.len());
        let new = object.edits.add_row(place, height);
        let row = new_row(new);
        object.edits.why = None;
        object.fields = None;
        // A tab with a new row is no preview to replace.
        object.pinned = true;
        let found = self.table(tab, id, |table, object| {
            let required = table.missing(new, &object.edits.cells).into_iter().next();
            let free = || {
                (0..table.page.columns.len()).find(|&col| table.lock(CellPos { row, col }).is_none())
            };
            (required, required.or_else(free))
        });
        let (required, col) = found.unwrap_or((None, None));
        let cell = CellPos {
            row,
            col: col.unwrap_or(0),
        };
        if required.is_some() {
            self.edit_cell(tab, id, cell, EditStart::Value, EditorPlace::Grid);
            return;
        }
        if let Some(object) = self.object_tab_mut(tab, id) {
            object.selection = Some(cell);
            object.focus_field = None;
        }
        if let Some(workspace) = self.workspace_mut(tab) {
            workspace.pane = Pane::Grid;
            workspace.save_refused = false;
            workspace.review_refused = false;
        }
    }

    /// Drops the selected row where it is a new one. The selection stays
    /// where the row stood, on the row that stands there now.
    pub(super) fn drop_row(&mut self, tab: ConnTabId, id: TabId) {
        let Some(object) = self.object_tab_mut(tab, id) else {
            return;
        };
        // Not under a save: its answer names the new rows it sent.
        if object.edits.saving.is_some() {
            return;
        }
        let Some(cell) = object.selection else {
            return;
        };
        let Some(new) = new_id(cell.row) else {
            return;
        };
        let height = object.page().map_or(0, |page| page.rows.len());
        let stood = Order::of(&object.edits.added, height).place(cell.row);
        if !object.edits.drop_row(new) {
            return;
        }
        object.fields = None;
        let order = Order::of(&object.edits.added, height);
        let last = order.len().saturating_sub(1);
        object.selection = stood
            .and_then(|place| order.row(place.min(last)))
            .map(|row| CellPos { row, col: cell.col });
    }
```

  In `App::apply` (`src/app.rs`), after the `Action::SetNull` arm:

```rust
            Action::AddRow { tab, id, place } => self.add_row(tab, id, place),
            Action::DropRow { tab, id } => self.drop_row(tab, id),
```

- [ ] **Step 6: The selection moves through the rows as they are shown.** In the `Action::MoveSelection` arm of `src/app.rs`, the object tab's branch becomes:

```rust
                } else if let Some(object) = self.object_tab_mut(tab, id) {
                    let (height, width) = object
                        .page()
                        .map(|page| (page.rows.len(), page.columns.len()))
                        .unwrap_or((0, 0));
                    // Through the rows as the grid shows them: a new row
                    // stands among the page's, and is no row of it.
                    let order = crate::edit::Order::of(&object.edits.added, height);
                    let moved = match object.selection {
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
                    };
                    object.pinned |= moved.is_some();
                    object.selection = moved;
                }
```

  In the `Action::DiscardEdits` arm, after `object.edits.discard();`, and in the `Action::LeaveDiscard` arm's loop after the same call: `object.off_new_rows();`.

- [ ] **Step 7: Run the tests, then the four checks.** The existing editing tests pass unchanged: a page's cell still loads what it loaded, and `Order` with no new row is the page's own order.

- [ ] **Step 8: Commit.**

```bash
git add src && git commit -m "Add a row to a table's pending set, edit it and drop it"
```

---

### Task 7: The save writes the new rows, and takes what comes back

**Files:**
- Modify: `src/edit.rs` (`Saving::inserts`, `Saved::added`, `Note::FailedInsert`, `Edits::fail_insert`)
- Modify: `src/model.rs` (`SaveBlock::Required`, `WritePrompt::added`)
- Modify: `src/app/editing.rs` (`save_blocked`, `write_edits`, `confirm_write`, `send_write`, `review_edits`, `run_command`, `written`, `answer_conflict`)
- Modify: `src/ui/pending_bar.rs`, `src/ui/workspace.rs` (an arm for the new `SaveBlock` and the new `Note`, so the tree compiles; their words are task 10's)
- Test: `src/app.rs`, `mod editing`

- [ ] **Step 1: Write the failing tests.** In `mod editing`:

```rust
        /// A row of `book_covers` as the database stores one.
        fn cover(id: i64, publisher: i64) -> Vec<Value> {
            vec![
                Value::Int(id),
                Value::Int(publisher),
                Value::Text("print".into()),
                Value::Null,
                Value::Text("2026-10-07 10:42:09".into()),
            ]
        }

        const HARBOR: i64 = 9_100_000_000_000_000_004;

        #[test]
        fn a_new_row_waits_for_its_required_values() {
            let (mut harness, tab, id, new) = with_new_row();
            harness.app.apply(Action::CancelEdit { tab, id });
            assert_eq!(
                harness.app.save_blocked(tab, id),
                Some(crate::model::SaveBlock::Required)
            );
            let before = harness.app.backend.sent.len();
            harness.app.apply(Action::WriteEdits { tab, id });
            assert!(write_since(&harness, before).is_none());
            // Filled, the save goes out: one INSERT, naming what was set.
            type_into(&mut harness, tab, id, at(new_row(new), 1), &HARBOR.to_string());
            assert_eq!(harness.app.save_blocked(tab, id), None);
            harness.app.apply(Action::WriteEdits { tab, id });
            let changes = write_since(&harness, before).expect("a Write");
            assert!(changes.rows.is_empty(), "{:?}", changes.rows);
            assert_eq!(changes.inserts.len(), 1);
            let set = &changes.inserts[0].set;
            assert_eq!(set.len(), 1, "{set:?}");
            assert_eq!(set[0].column, "publisher_id");
            let saving = object(&harness, tab, id).edits.saving.as_ref().unwrap();
            assert_eq!((saving.inserts.as_slice(), saving.rows.as_slice()), (&[new][..], &[][..]));
        }

        #[test]
        fn a_saved_new_row_takes_what_the_database_stored() {
            let (mut harness, tab, id, new) = with_new_row();
            type_into(&mut harness, tab, id, at(new_row(new), 1), &HARBOR.to_string());
            // A changed row beside it, and a row an earlier save found gone.
            type_into(&mut harness, tab, id, at(1, 2), "audio");
            let object_tab = harness.app.workspace_mut(tab).unwrap().object_tab_mut(id);
            object_tab.unwrap().edits.gone.insert(2);
            harness.app.apply(Action::SelectCell {
                tab,
                id,
                cell: at(new_row(new), 1),
            });
            harness.app.apply(Action::WriteEdits { tab, id });
            let mut changed = cover(2, 9_100_000_000_000_000_002);
            changed[2] = Value::Text("audio".into());
            harness.answer_written(Ok(WriteOutcome::Written {
                inserted: vec![Some(cover(15, HARBOR))],
                rows: vec![changed],
                elapsed: std::time::Duration::from_millis(5),
            }));
            let tab_now = object(&harness, tab, id);
            assert!(!tab_now.edits.holds() && tab_now.edits.added.is_empty());
            // The row is the page's first now, with the id the database
            // gave it and the defaults it filled, and every row after it
            // is one further down.
            let page = tab_now.page().unwrap();
            assert_eq!(page.rows.len(), 4);
            assert_eq!(page.rows[0], cover(15, HARBOR));
            assert_eq!(page.rows[2][2], Value::Text("audio".into()));
            assert_eq!(tab_now.selection, Some(at(0, 1)));
            let gone: Vec<usize> = tab_now.edits.gone.iter().copied().collect();
            assert_eq!(gone, [3]);
            // The whole new row shows that it was written, and the changed
            // cell where it is now.
            let saved = tab_now.edits.saved.as_ref().unwrap();
            assert_eq!((saved.added, saved.changes, saved.rows), (1, 1, 1));
            let mut cells: Vec<CellPos> = (0..5).map(|col| at(0, col)).collect();
            cells.push(at(2, 2));
            assert_eq!(saved.cells, cells);
        }

        #[test]
        fn a_new_row_that_comes_back_unknown_reloads_the_page() {
            let (mut harness, tab, id, new) = with_new_row();
            type_into(&mut harness, tab, id, at(new_row(new), 1), &HARBOR.to_string());
            harness.app.apply(Action::WriteEdits { tab, id });
            let before = harness.app.backend.sent.len();
            harness.answer_written(Ok(WriteOutcome::Written {
                inserted: vec![None],
                rows: Vec::new(),
                elapsed: std::time::Duration::ZERO,
            }));
            let tab_now = object(&harness, tab, id);
            assert!(tab_now.edits.added.is_empty() && tab_now.selection.is_none());
            assert_eq!(tab_now.page().unwrap().rows.len(), 3, "not guessed at");
            let fetched = harness.app.backend.sent[before..]
                .iter()
                .any(|command| matches!(command, Command::FetchRows { .. }));
            assert!(fetched, "the page is read again");
        }

        #[test]
        fn a_failed_insert_leaves_everything_pending() {
            let (mut harness, tab, id, new) = with_new_row();
            let row = new_row(new);
            type_into(&mut harness, tab, id, at(row, 1), "9100000000000000099");
            type_into(&mut harness, tab, id, at(1, 2), "audio");
            harness.app.apply(Action::WriteEdits { tab, id });
            let error = tabletist_db::Error::query("FOREIGN KEY constraint failed");
            harness.answer_written(Ok(WriteOutcome::FailedInsert {
                insert: 0,
                error: error.clone(),
            }));
            let edits = &object(&harness, tab, id).edits;
            assert!(edits.saving.is_none());
            // The row and its changed neighbour are both still pending; the
            // new row says what the database said.
            assert_eq!(edits.added.len(), 1);
            assert_eq!(edits.added[0].failed, Some(error.clone()));
            assert_eq!(edits.cells[&(row, 1)].state, State::Failed(error.clone()));
            assert_eq!(edits.cells[&(1, 2)].state, State::Ready);
            assert_eq!(edits.note, Some(crate::edit::Note::FailedInsert { error }));
            // Typing into it again takes the failure off the row.
            type_into(&mut harness, tab, id, at(row, 1), &HARBOR.to_string());
            let edits = &object(&harness, tab, id).edits;
            assert_eq!(edits.added[0].failed, None);
            assert_eq!(edits.cells[&(row, 1)].state, State::Ready);
        }

        #[test]
        fn a_table_without_a_key_takes_a_row_it_then_locks() {
            let mut harness = Harness::new();
            let (tab, id) = harness.book_covers();
            let object_tab = harness.app.workspace_mut(tab).unwrap().object_tab_mut(id);
            let structure = object_tab.unwrap().structure.value.as_mut().unwrap();
            structure.primary_key.clear();
            let place = Place::Top;
            harness.app.apply(Action::AddRow { tab, id, place });
            let new = object(&harness, tab, id).edits.added[0].id;
            type_into(&mut harness, tab, id, at(new_row(new), 1), &HARBOR.to_string());
            assert_eq!(harness.app.save_blocked(tab, id), None);
            harness.app.apply(Action::WriteEdits { tab, id });
            harness.answer_written(Ok(WriteOutcome::Written {
                inserted: vec![Some(cover(15, HARBOR))],
                rows: Vec::new(),
                elapsed: std::time::Duration::ZERO,
            }));
            assert_eq!(object(&harness, tab, id).page().unwrap().rows[0], cover(15, HARBOR));
            // Saved, it is a row of a table without a key: not to be edited.
            harness.app.apply(Action::EditCell {
                tab,
                id,
                cell: at(0, 2),
                start: EditStart::Value,
            });
            let tab_now = object(&harness, tab, id);
            assert!(tab_now.edits.editor.is_none());
            assert_eq!(tab_now.edits.why, Some((at(0, 2), Lock::NoKey)));
        }

        #[test]
        fn a_save_to_production_counts_its_new_rows() {
            let mut harness = Harness::new();
            let (tab, id) = harness.book_covers();
            harness.app.workspace_mut(tab).unwrap().environment =
                crate::env::Environment::Production;
            let place = Place::Top;
            harness.app.apply(Action::AddRow { tab, id, place });
            let new = object(&harness, tab, id).edits.added[0].id;
            type_into(&mut harness, tab, id, at(new_row(new), 1), &HARBOR.to_string());
            type_into(&mut harness, tab, id, at(1, 2), "audio");
            let before = harness.app.backend.sent.len();
            harness.app.apply(Action::WriteEdits { tab, id });
            assert!(write_since(&harness, before).is_none(), "asked first");
            let Some(Dialog::ConfirmWrite(prompt)) = &harness.app.dialog else {
                panic!("the confirmation");
            };
            assert_eq!((prompt.added, prompt.changes, prompt.rows), (1, 1, 1));
            harness.app.apply(Action::ConfirmWrite);
            let changes = write_since(&harness, before).expect("a Write");
            assert_eq!((changes.inserts.len(), changes.rows.len()), (1, 1));
        }
```

- [ ] **Step 2: Run them, and see them fail.** They do not compile: `SaveBlock::Required`, `Saving::inserts`, `Saved::added`, `Note::FailedInsert` and `WritePrompt::added` are not there.

- [ ] **Step 3: The model.** In `src/edit.rs`: `Saving` gains, after `rows`, `/// The id of the new row of each of the set's inserts, in its order.` `pub inserts: Vec<usize>,`. `Saved` gains, after `rows`, `/// How many rows it added.` `pub added: usize,`. `Note` gains:

```rust
    /// The `INSERT` of a new row failed. The row says so itself: it has no
    /// place in the page to be named by.
    FailedInsert { error: Error },
```

  and `impl Edits` gains:

```rust
    /// The `INSERT` of the new row `id` failed with `error`: what is set
    /// in it says so until the next save sends it again, and the row
    /// itself where nothing is set. With no such row the save is refused
    /// with the error.
    pub fn fail_insert(&mut self, id: Option<usize>, error: Error) {
        let held = id.filter(|id| self.added.iter().any(|new| new.id == *id));
        let Some(id) = held else {
            self.note = Some(Note::Refused(error));
            return;
        };
        let row = new_row(id);
        for (_, cell) in self.cells.range_mut((row, 0)..=(row, usize::MAX)) {
            cell.state = State::Failed(error.clone());
        }
        if let Some(new) = self.added.iter_mut().find(|new| new.id == id) {
            new.failed = Some(error.clone());
        }
        self.note = Some(Note::FailedInsert { error });
    }
```

  In `src/model.rs`: `SaveBlock` gains, after `ToFix`, `/// A new row lacks a value a save needs.` `Required,`; `WritePrompt` gains, after `rows`, `/// How many rows the save adds.` `pub added: usize,` (and its hand-written `Debug` prints it). Every `match` over `SaveBlock` and over `Note` without a `_` arm stops compiling: in `block_text` of `src/ui/pending_bar.rs` add `SaveBlock::Required => gettext(locale, "Fill required fields to save").into_owned(),` and in `note_said` give `Note::FailedInsert { error }` the arm `Note::Failed` has. Task 10 words both for good. A `Saving` is built by hand in a test of `src/model.rs` and a `Saved` in one of `src/app.rs`: `inserts: Vec::new()`, `added: 0`.

- [ ] **Step 4: What blocks a save, and what is sent.** In `src/app/editing.rs`:

  `save_blocked`, after the `ToFix` check:

```rust
        // A new row that lacks a value a save needs.
        let lacking = self.table(tab, id, |table, object| {
            let cells = &object.edits.cells;
            let mut added = object.edits.added.iter();
            added.any(|new| !table.missing(new.id, cells).is_empty())
        });
        if lacking == Some(true) {
            return Some(SaveBlock::Required);
        }
```

  and its last check reads what is pending, not the cells alone: `let unsendable = object.edits.pending() && self.table(…).flatten().is_none();`.

  `write_edits`: the `nothing` check becomes `.is_some_and(|object| !object.edits.pending())`. The pair is `(changes, sent)`. The review is `crate::review::of(dialect, &changes, crate::review::Blocked::default(), Values::Shown)`, and a refusal goes to the part it is of:

```rust
            if let Some((part, error)) = review.refused.clone() {
                if let Some(object) = self.object_tab_mut(tab, id) {
                    object.edits.saved = None;
                    match part {
                        Part::Row(index) => object.edits.fail(sent.rows.get(index).copied(), error),
                        Part::Insert(index) => {
                            object.edits.fail_insert(sent.inserts.get(index).copied(), error);
                        }
                    }
                    // The row panel says what stands against a cell.
                    object.fields = None;
                }
                return;
            }
```

  The prompt is built with `rows: sent.rows.len(), added: changes.inserts.len(),`, and both `send_write` calls (here and in `confirm_write`) pass `sent` where they passed `rows`. `send_write` takes `sent: crate::edit::Sent` and builds `Saving { request, rows: sent.rows, inserts: sent.inserts, started: …, then }`.

  `review_edits`: `object.edits.reviewing = show && object.edits.pending();`. `run_command`: the tab's second fact is `object.edits.pending()`. `answer_conflict`: its `pending` before the save runs again is `object.edits.pending()`.

- [ ] **Step 5: What comes back.** In `written`, the `Written` arm becomes:

```rust
            Ok(WriteOutcome::Written {
                inserted,
                rows,
                elapsed,
            }) => {
                let counts = object.edits.counts();
                // The changed cells, by the page's rows they were of.
                let changed: Vec<CellPos> = object
                    .edits
                    .cells
                    .range(..(crate::edit::NEW_ROWS, 0))
                    .map(|(&(row, col), _)| CellPos { row, col })
                    .collect();
                // Where the new rows stood: the set is dropped next.
                let added = std::mem::take(&mut object.edits.added);
                // The rows an earlier save found gone are still gone.
                object.edits.discard();
                object.fields = None;
                let fits = object.rows.value.as_ref().is_some_and(|page| {
                    let wide = |row: &Vec<tabletist_db::Value>| row.len() == page.columns.len();
                    rows.len() == saving.rows.len()
                        && rows.iter().all(wide)
                        && saving.rows.iter().all(|&at| at < page.rows.len())
                        // Every new row is known, and is one the set held.
                        && inserted.len() == saving.inserts.len()
                        && inserted.iter().all(|row| row.as_ref().is_some_and(wide))
                        && saving
                            .inserts
                            .iter()
                            .all(|id| added.iter().any(|new| new.id == *id))
                        && saving.inserts.len() == added.len()
                });
                if fits && let Some(page) = object.rows.value.as_mut() {
                    for (row, &at) in rows.into_iter().zip(&saving.rows) {
                        page.rows[at] = row;
                    }
                    // Each new row takes its place in the page: the rows
                    // are the page's from here on, in the order they were
                    // shown, so a row's number is where it was shown.
                    let order = Order::of(&added, page.rows.len());
                    let mut stored: std::collections::BTreeMap<usize, Vec<tabletist_db::Value>> =
                        saving.inserts.iter().copied().zip(inserted.into_iter().flatten()).collect();
                    let mut loaded: Vec<Option<Vec<tabletist_db::Value>>> =
                        std::mem::take(&mut page.rows).into_iter().map(Some).collect();
                    page.rows = (0..order.len())
                        .filter_map(|place| {
                            let row = order.row(place)?;
                            match new_id(row) {
                                Some(id) => stored.remove(&id),
                                None => loaded.get_mut(row)?.take(),
                            }
                        })
                        .collect();
                    let width = page.columns.len();
                    let now = |row: usize| order.place(row);
                    // What the save wrote, where it is now: each changed
                    // cell, and every cell of a new row.
                    let mut cells: Vec<CellPos> = changed
                        .into_iter()
                        .filter_map(|cell| {
                            let row = now(cell.row)?;
                            Some(CellPos { row, col: cell.col })
                        })
                        .collect();
                    for &id in &saving.inserts {
                        if let Some(row) = now(new_row(id)) {
                            cells.extend((0..width).map(|col| CellPos { row, col }));
                        }
                    }
                    // By row, then column: the grid finds a cell by halving.
                    cells.sort_unstable_by_key(|cell| (cell.row, cell.col));
                    let gone = std::mem::take(&mut object.edits.gone);
                    object.edits.gone = gone.into_iter().filter_map(now).collect();
                    object.selection = object.selection.and_then(|cell| {
                        let row = now(cell.row)?;
                        Some(CellPos { row, col: cell.col })
                    });
                    // The table has that many rows more, where it is known
                    // how many it had.
                    let more = saving.inserts.len() as u64;
                    if let Some(count) = object.count.value.as_mut() {
                        *count += more;
                    }
                    if let Some(estimate) = object.estimated_rows.as_mut() {
                        *estimate += more;
                    }
                    object.edits.saved = Some(Saved {
                        at: std::time::Instant::now(),
                        cells,
                        changes: counts.changes,
                        rows: counts.rows,
                        added: counts.added,
                        elapsed,
                    });
                } else {
                    // The table is not the one the page was read from, or a
                    // new row came back unknown (a table with a trigger,
                    // MySQL without a key to find it by): read it again. A
                    // new row is nowhere until the page says where.
                    object.off_new_rows();
                    self.fetch_rows(tab, id);
                }
                // Written either way, and the set is empty: what was held
                // passes the guard.
                if let Some(held) = then {
                    self.perform(held);
                }
            }
```

  The borrow of `object` ends before `self.fetch_rows`, as it does today; if the compiler holds `page` across the `else`, compute `fits` into a `bool`, then write `if fits { if let Some(page) = … { … } } else { … }`.

  The `FailedInsert` arm becomes:

```rust
            Ok(WriteOutcome::FailedInsert { insert, error }) => {
                object.edits.fail_insert(saving.inserts.get(insert).copied(), error);
                // The row panel says what stands against a cell.
                object.fields = None;
            }
```

  A page read again takes the tab's `Edits` with it (`Event::Rows` in `src/app.rs` resets them), so after a reload the footer does not say "written". That is how a save of a changed row that no longer fits the page ends today, and it is left so.

- [ ] **Step 6: Run the tests, the four checks, and the shots' clippy.** Existing tests of `written` pass: with no new row `Order` is the page's own order, `now` is the identity, and `cells` is the changed cells in the set's order as before.

- [ ] **Step 7: Commit.**

```bash
git add src && git commit -m "Write a table's new rows in its save, and put what was stored in the page"
```

---

### Task 8: The button and the keys

After this task a row can be added, filled and saved by hand. The row is drawn as any other until task 9, with its unset cells empty.

**Files:**
- Modify: `src/ui/data_view.rs` (`header`, and for now the grid's rows through `Order` in `show`: see step 3)
- Modify: `src/ui/keys.rs` (`handle`, `editing_keys`, `editing_letters`, `letters`, `SHORTCUTS`)
- Modify: `src/ui/object_tabs.rs` (the tab's dot)
- Modify: `src/ui/pending_bar.rs` (the bar is there for a new row, and counts it)
- Test: `src/ui/mod.rs`, `src/ui/keys.rs`

- [ ] **Step 1: Write the failing tests.** In `mod tests` of `src/ui/mod.rs`, near `editable_in`:

```rust
    /// The Bookshop's covers, writable, the grid holding the keyboard.
    fn covers_in(look: Look) -> (Harness, ConnTabId, TabId) {
        let mut harness = Harness::new();
        harness.set_look(look);
        let (tab, id) = harness.book_covers();
        focus_grid(&mut harness, tab);
        harness.settle();
        (harness, tab, id)
    }

    #[test]
    fn mod_n_adds_a_row_to_the_table_in_front_and_delete_drops_it() {
        for look in desktop_looks() {
            let (mut harness, tab, id) = covers_in(look);
            harness.press(Key::N, Modifiers::COMMAND);
            assert!(harness.app.dialog.is_none(), "{}", look.name);
            assert_eq!(edits(&harness, tab, id).added.len(), 1, "{}", look.name);
            // The bar is there for it, with nothing set in it yet.
            assert!(harness.has("1 new row"), "{}", look.name);
            // Its first required cell is open and has the keyboard.
            harness.settle();
            assert!(harness.ctx.text_edit_focused(), "{}", look.name);
            assert_eq!(selected(&harness, tab, id).map(|at| at.1), Some(1));
            // Esc leaves the row pending.
            harness.press(Key::Escape, Modifiers::NONE);
            assert!(edits(&harness, tab, id).editor.is_none());
            assert_eq!(edits(&harness, tab, id).added.len(), 1, "{}", look.name);
            // Held down, the chord adds one row, not a column of them. The
            // event is the one `crate::testing::key` builds, as a repeat.
            harness.frame(vec![egui::Event::Key {
                key: Key::N,
                physical_key: None,
                pressed: true,
                repeat: true,
                modifiers: Modifiers::COMMAND,
            }]);
            harness.settle();
            assert_eq!(edits(&harness, tab, id).added.len(), 1, "{}", look.name);
            // Delete drops the new row the selection is on, and no other.
            harness.press(Key::Delete, Modifiers::NONE);
            assert!(edits(&harness, tab, id).added.is_empty(), "{}", look.name);
            harness.press(Key::Delete, Modifiers::NONE);
            let workspace = harness.app.workspace(tab).unwrap();
            let rows = workspace.object_tab(id).unwrap().page().unwrap().rows.len();
            assert_eq!(rows, 3, "{}", look.name);
        }
    }

    #[test]
    fn mod_n_is_a_new_connection_where_no_row_can_be_added() {
        // A read-only connection's table.
        let mut harness = Harness::new();
        let tab = with_page(&mut harness);
        focus_grid(&mut harness, tab);
        harness.press(Key::N, Modifiers::COMMAND);
        assert!(matches!(harness.app.dialog, Some(Dialog::Connection(_))));
        // The Structure view of a table that takes rows.
        let (mut harness, tab, id) = covers_in(Look::macos());
        harness.app.apply(Action::SetView {
            tab,
            object_tab: id,
            view: crate::model::ObjectView::Structure,
        });
        harness.press(Key::N, Modifiers::COMMAND);
        assert!(matches!(harness.app.dialog, Some(Dialog::Connection(_))));
        assert!(edits(&harness, tab, id).added.is_empty());
        // The terminal look, whose key for a row is `o`.
        let (mut harness, tab, id) = covers_in(Look::omarchy());
        harness.press(Key::N, Modifiers::COMMAND);
        assert!(matches!(harness.app.dialog, Some(Dialog::Connection(_))));
        assert!(edits(&harness, tab, id).added.is_empty());
        // A SQL editor on a writable connection: a query's result takes
        // no row, and the key is the connection's there.
        let mut harness = Harness::new();
        let tab = harness.connect_fake_as(false);
        harness.app.apply(Action::NewSqlTab(tab));
        harness.press(Key::N, Modifiers::COMMAND);
        assert!(matches!(harness.app.dialog, Some(Dialog::Connection(_))));
    }

    #[test]
    fn o_opens_a_row_below_the_cursor_and_capital_o_above_it() {
        let (mut harness, tab, id) = covers_in(Look::omarchy());
        select(&mut harness, tab, id, (1, 2));
        type_key(&mut harness, Key::O, "o");
        let below = edits(&harness, tab, id).added[0].clone();
        assert_eq!(below.before, 2);
        assert!(edits(&harness, tab, id).editor.is_some(), "insert mode");
        // Esc keeps the row, and the cursor is on it.
        harness.press(Key::Escape, Modifiers::NONE);
        assert_eq!(edits(&harness, tab, id).added.len(), 1);
        select(&mut harness, tab, id, (1, 2));
        harness.frame(vec![
            crate::testing::key(Key::O, Modifiers::SHIFT),
            egui::Event::Text("O".into()),
        ]);
        harness.frame(vec![crate::testing::release(Key::O, Modifiers::SHIFT)]);
        harness.settle();
        let added = &edits(&harness, tab, id).added;
        assert_eq!(added.len(), 2);
        assert_eq!((added[0].before, added[1].id), (1, below.id));
        // `dd` drops the new row under the cursor; one `d` does not.
        harness.press(Key::Escape, Modifiers::NONE);
        type_key(&mut harness, Key::D, "d");
        assert_eq!(edits(&harness, tab, id).added.len(), 2);
        type_key(&mut harness, Key::D, "d");
        assert_eq!(edits(&harness, tab, id).added.len(), 1);
        assert_eq!(edits(&harness, tab, id).added[0].id, below.id);
        // A chord's letter is no `o`, and with the arrows on the tree the
        // letter adds nothing.
        harness.frame(vec![
            egui::Event::ModifiersChanged(Modifiers::ALT),
            crate::testing::key(Key::O, Modifiers::ALT),
            egui::Event::Text("o".into()),
        ]);
        harness.frame(vec![
            crate::testing::release(Key::O, Modifiers::ALT),
            egui::Event::ModifiersChanged(Modifiers::NONE),
        ]);
        harness.app.workspace_mut(tab).unwrap().pane = crate::model::Pane::Tree;
        type_key(&mut harness, Key::O, "o");
        assert_eq!(edits(&harness, tab, id).added.len(), 1);
    }

    #[test]
    fn the_add_row_button_adds_a_row_where_one_can_be_added() {
        let (mut harness, tab, id) = covers_in(Look::macos());
        harness.click("Add row");
        assert_eq!(edits(&harness, tab, id).added.len(), 1);
        // From the Structure view it shows the rows first.
        harness.app.apply(Action::DiscardEdits { tab, id });
        harness.app.apply(Action::SetView {
            tab,
            object_tab: id,
            view: crate::model::ObjectView::Structure,
        });
        harness.click("Add row");
        let workspace = harness.app.workspace(tab).unwrap();
        let object = workspace.object_tab(id).unwrap();
        assert_eq!(object.view, crate::model::ObjectView::Data);
        assert_eq!(object.edits.added.len(), 1);
    }
```

  And rewrite `a_button_that_cannot_be_pressed_still_says_why_to_the_keyboard`: its table is on the read-only fixture connection, so the button stays one that cannot be pressed, with another reason. Replace both "Editing arrives in a later version" in it with "This connection opens read-only". The rest of the test holds.

- [ ] **Step 2: Run them, and see them fail.**

- [ ] **Step 3: The grid shows the new rows.** A new row that is not drawn cannot hold its editor, so the grid goes through `Order` here, plainly; task 9 gives the row its look. In `show` of `src/ui/data_view.rs`, after `let mut changes = …`:

```rust
        // The rows as the grid shows them: the page's, and the new ones
        // among them. A place is the grid's; a row is the set's and the
        // page's, and the reducer's.
        let order = crate::edit::Order::of(&object.edits.added, page.rows.len());
        let shown = |cell: CellPos| {
            let row = order.place(cell.row)?;
            Some(CellPos { row, col: cell.col })
        };
        let held = |cell: CellPos| {
            let row = order.row(cell.row)?;
            Some(CellPos { row, col: cell.col })
        };
```

  (name the existing `let shown: Vec<Shown>` `fits` or the closure `placed`: one of the two names has to go.) Then in the call to `grid::show`: `order.len()` for `page.rows.len()`, `object.selection.and_then(shown)` for `object.selection`, `editing.and_then(shown)` for `editing`, the row's mark through `order.row(place)` (a new row's is `RowMark::None` until task 9), and the cell closure begins:

```rust
            |place, col| {
                let Some(row) = order.row(place) else {
                    return Cell::default();
                };
                let column = &page.columns[col];
                if crate::edit::new_id(row).is_some() {
                    // A new row: what was set in the cell, or nothing yet.
                    return match changes.cells.get(&(row, col)) {
                        Some(pending) => {
                            let value = drawn(&pending.new);
                            kept(cell(&ctx, &value, column, &tags[col], &look, fits[col]))
                        }
                        None => Cell::default(),
                    };
                }
                let loaded = &page.rows[row][col];
                // … as it is today, from here.
```

  After the call, `output.clicked.and_then(held)` and `output.double_clicked.and_then(held)` where the cells are read. `page.rows.is_empty() && !lasted` (the "no rows" state under the header) becomes `order.is_empty() && !lasted`. The row's number (`first_row_number`) stays the page's offset for now; task 9 gives each row its own.

- [ ] **Step 4: The button.** In `header` of `src/ui/data_view.rs`, before the header's closure reads `app` for the last time, find why no row can be added:

```rust
    // Why Add row cannot be pressed, where it cannot: the table takes no
    // row, or its page is not there yet.
    let no_rows = app.workspace(tab).and_then(|workspace| {
        let object = workspace.object_tab(object_tab)?;
        let table = format::display_safe(&object.object.name);
        let lock = match crate::edit::Table::of(workspace, object) {
            Some(table) => table.no_rows(),
            None => Some(crate::edit::Lock::Refreshing),
        };
        lock.map(|lock| super::cell_editor::lock_text(lock, &table, locale))
    });
```

  The button is the one that is there, pressable where `no_rows` is `None`, with the key the design draws on it:

```rust
            // Add row keeps 12 clear of the switch. When the room runs out
            // the summary gives way first, then Add row drops its text and
            // its key, then it goes, and only then is the title cut.
            let label = gettext(locale, "Add row");
            let keys = format!("{}N", look.command_key());
            let add_row = |short: bool| {
                let button = widgets::ButtonSpec::new(if short { "" } else { &label })
                    .label(&label)
                    .icon(Icon::Plus)
                    .role(TextRole::UiBodyStrong);
                let button = match &no_rows {
                    Some(reason) => button.disabled(reason),
                    None if short => button,
                    None => button.shortcut(&keys),
                };
                if short { button.gap(0.0) } else { button }
            };
```

  and where it is shown:

```rust
                if button.show_at(ui, place, &look, &palette).clicked() {
                    // From the Structure view: the rows first, then the row.
                    if view != ObjectView::Data {
                        actions.push(Action::SetView {
                            tab,
                            object_tab,
                            view: ObjectView::Data,
                        });
                    }
                    actions.push(Action::AddRow {
                        tab,
                        id: object_tab,
                        place: crate::edit::Place::Top,
                    });
                }
```

  (`view` is the tab's view, which the header reads for its switch already; use the name it has there.) Correct the header's doc comment: "(disabled until editing arrives)" goes.

- [ ] **Step 5: `Mod+N`.** In `handle` of `src/ui/keys.rs`, beside `grid`:

```rust
    // Mod+N adds a row where a table's rows are in front and the table
    // takes one, as the canvas scopes the key ("Table"), in the looks whose
    // key for it this is. Everywhere else it is a new connection.
    let adds = object.filter(|_| grid && !terminal).filter(|&(tab, id)| {
        app.workspace(tab)
            .and_then(|workspace| {
                let object = workspace.object_tab(id)?;
                crate::edit::Table::of(workspace, object)
            })
            .is_some_and(|table| table.no_rows().is_none())
    });
```

  Inside the `ctx.input_mut` block, **before** `let mut key = …` is made (the closure holds `input` and `actions` from there on):

```rust
        // A fresh press only: held down, it would add a row a frame. The
        // press is taken, so the line below that reads the same chord for
        // a new connection finds none.
        if let Some((tab, id)) = adds
            && consume_press(input, Modifiers::COMMAND, Key::N)
        {
            let place = crate::edit::Place::Top;
            actions.push(Action::AddRow { tab, id, place });
        }
```

  If `consume_press` leaves a repeat of the chord in the input, also skip `key(Modifiers::COMMAND, Key::N, Action::NewConnection)` while `adds.is_some()`: the test's held key says which.

- [ ] **Step 6: Delete on a new row.** In `editing_keys`, in its lower half after the `Mod+Z` block:

```rust
        // Delete (the key a Mac labels so too) drops a new row: nothing of
        // it is in the table. On a row of the page neither key does
        // anything yet.
        let on_new = object
            .selection
            .is_some_and(|cell| crate::edit::new_id(cell.row).is_some());
        let dropped = on_new
            && field.is_none()
            && (take_press(input, Modifiers::NONE, Key::Delete)
                + take_press(input, Modifiers::NONE, Key::Backspace))
                > 0;
        if dropped {
            actions.push(Action::DropRow { tab, id });
            return;
        }
```

  And `Mod+S` and `Mod+Shift+D`, above, act while anything is pending: `object.edits.pending()` where they read `!object.edits.cells.is_empty()`.

- [ ] **Step 7: `o`, `O` and `dd`.** In `editing_letters`, `mine` takes the two letters, `"i" | "c" | "x" | "u" | ":" | "o" | "O"`, and the `match` gains:

```rust
                // A row below the cursor's, or above it with the capital.
                // With no cursor, at the top.
                "o" | "O" => {
                    opened = true;
                    let place = match (selection, text.as_str()) {
                        (Some(cell), "o") => crate::edit::Place::Below(cell.row),
                        (Some(cell), _) => crate::edit::Place::Above(cell.row),
                        (None, _) => crate::edit::Place::Top,
                    };
                    actions.push(Action::AddRow { tab, id, place });
                }
```

  Its doc comment lists its letters: add these. In `letters`, before the `d` arm, whether the grid's rows have the keys (`tree` and `field` are that function's own):

```rust
    let on_rows = !tree
        && field.is_none()
        && app
            .workspace(tab)
            .and_then(|workspace| workspace.object_tab(object_tab))
            .is_some_and(|object| object.view == crate::model::ObjectView::Data);
```

  and the `d` arm:

```rust
    if pressed(Key::D) {
        if pending == Some('g') {
            actions.push(Action::FollowSelectedKey { tab, object_tab });
        } else {
            // The second `d` of `dd` drops the row under the cursor where
            // it is a new one. Each `d` is the Data view's key still,
            // which changes nothing where the rows show already. Only
            // there does a first `d` wait for a second: one that brought
            // the rows up from the Structure view was that key and no more,
            // and a dropped row is not brought back.
            if on_rows && pending == Some('d') {
                actions.push(Action::DropRow {
                    tab,
                    id: object_tab,
                });
            } else if on_rows {
                next_pending = Some('d');
            }
            actions.push(Action::SetView {
                tab,
                object_tab,
                view: crate::model::ObjectView::Data,
            });
        }
    }
```

- [ ] **Step 8: The table and the tab's dot.** In `SHORTCUTS`, after the two "Edit the cell" rows:

```rust
    ("Mod+N", "Add a row", DESKTOP),
    ("o, O", "Add a row below or above the cursor", TERMINAL),
    ("Delete", "Drop a new row", DESKTOP),
    ("dd", "Drop a new row", TERMINAL),
```

  and `o`, `O`, `dd` join the long row of Omarchy's vim keys, after `u`. Add to `mod tests` of `src/ui/keys.rs`:

```rust
    #[test]
    fn the_shortcut_table_names_the_keys_that_add_a_row() {
        for look in crate::theme::Look::ALL {
            let keys = |what: &str| {
                let mut rows = shortcuts(&look).filter(|(_, listed)| *listed == what);
                let found = rows.next().map(|(keys, _)| keys);
                assert!(rows.next().is_none(), "{}: {what} twice", look.name);
                found
            };
            if look.terminal {
                assert_eq!(keys("Add a row below or above the cursor"), Some("o, O"));
                assert_eq!(keys("Drop a new row"), Some("dd"));
                assert_eq!(keys("Add a row"), None);
            } else {
                assert_eq!(keys("Add a row"), Some("Mod+N"));
                assert_eq!(keys("Drop a new row"), Some("Delete"));
            }
            // A new connection is still Mod+N, where no table is in front.
            assert_eq!(keys("New connection"), Some("Mod+N"));
        }
    }
```

  In `src/ui/object_tabs.rs`, the tab's dot shows for a new row too: `object.edits.pending() || typed`.

- [ ] **Step 9: The bar is there for a new row.** A row with nothing set in it has no pending cell, and the bar is drawn only while there is one: no Save, no Review SQL. In `src/ui/pending_bar.rs`, `show` is there while `edits.pending() || edits.saving.is_some() || edits.note.is_some()`, and its `pending` is `edits.pending()`. Replace `counts_text` and the `short` beside it with one that knows the new rows:

```rust
/// "1 new row · 3 changes in 2 rows", each part where there is any. Short,
/// the changes go without their rows.
fn counts_text(counts: crate::edit::Counts, short: bool, locale: Locale) -> String {
    let mut parts = Vec::new();
    if counts.added > 0 {
        parts.push(counted(locale, counts.added, "new row", "new rows"));
    }
    if counts.changes > 0 {
        let changes = counted(locale, counts.changes, "change", "changes");
        parts.push(if short {
            changes
        } else {
            format!(
                "{changes} {} {}",
                gettext(locale, "in"),
                counted(locale, counts.rows, "row", "rows")
            )
        });
    }
    parts.join(" · ")
}
```

  `whole` is `counts_text(counts, false, locale)` and `short` is `counts_text(counts, true, locale)`, each under the `pending.then(..)` it has. With no new row both read as they did. The rest of the bar's words is task 10's.

- [ ] **Step 10: Run the tests, then the four checks.** The forty-odd tests that open the connection dialog with `Mod+N` start from the picker, from a SQL editor or from the read-only fixture, and pass. One that presses it over a writable table's rows now adds a row: give it `Action::NewConnection` instead, which is what it meant.

- [ ] **Step 11: Commit.**

```bash
git add src && git commit -m "Add a row with the Add row button, Mod+N, and the terminal's o and O"
```

---

### Task 9: The new row, as the design draws it

**Files:**
- Modify: `src/edit.rs` (`RowMark::New`)
- Modify: `src/ui/grid.rs` (`Row`, `Mark::Added`, `Mark::Unset`, `Column::required`, the row's fill, bar and gutter sign, the header's star)
- Modify: `src/ui/data_view.rs` (the new row's cells, the rows' numbers, the header's count, the footer's note)
- Modify: `src/ui/sql_results.rs` (the caller of `grid::show`, and its `Column`s)
- Modify: `src/ui/row_panel.rs` (what it says of a new row)
- Test: `src/ui/grid.rs`, `src/ui/mod.rs`

Read the two boards again before this task: "macOS – Inserting a row" and "Omarchy – Inserting rows" (`MacInsert`, `OmarchyInsert`), with the Artifact tool. What they give, and what is built of it:

| | macOS, Windows | Omarchy |
|---|---|---|
| The row | the success tone's fill, a 3 pt bar of the tone at its left | the same fill, `+` in the gutter in the tone |
| Its marker | `+ new` in the tone, in the first column the database assigns | `new`, dimmed, in that column |
| Required and empty | the danger outline and fill, the word `required` in the danger tone | an empty cell, and a red `*` after the column's name in the header while a new row is pending |
| A default | its text, dimmed. Under the pointer: "from DEFAULT" | its text, dimmed |
| Assigned (not the marker's cell) | empty. Under the pointer: "Assigned by the database on save" | empty |
| Nullable, no default | the NULL chip | `null` as the look writes it |
| Set | the value as any cell draws it, on the row's fill | the same |
| To fix, failed | as any pending cell in trouble: the danger tone, the reason under the pointer | the same, `!` in the gutter |
| Read by a screen reader | "New row, not saved" | the same |

`Tone::Success` is the tone a saved cell already has (`Mark::Saved`); its `fill` and `color` are the design's greens in each look. Use them, and name no colour.

- [ ] **Step 1: Write the failing tests.** In `mod tests` of `src/ui/grid.rs`, beside `a_marked_cell_is_tinted_and_its_row_is_marked`, with its `marked` helper:

```rust
    #[test]
    fn a_new_row_is_filled_and_marked_in_the_success_tone() {
        for look in Look::ALL {
            let ctx = context(&look);
            let palette = Palette::light(&look);
            let (rects, texts) = marked(&ctx, &look, &palette, Mark::Added, RowMark::New);
            let tone = Tone::Success;
            // The row's own fill, whatever its cells hold.
            assert!(
                rects.iter().any(|(_, fill)| *fill == tone.fill(&look, &palette)),
                "{}",
                look.name
            );
            if look.terminal {
                // `+` in the gutter, in the tone.
                assert!(
                    texts.iter().any(|(text, color, _)| text == "+" && *color == tone.color(&palette)),
                    "{}",
                    look.name
                );
            } else {
                // The bar at the row's left.
                assert!(
                    rects.iter().any(|(rect, fill)| rect.width() == 3.0 && *fill == tone.color(&palette)),
                    "{}",
                    look.name
                );
            }
        }
    }
```

  Take `context`, `Palette::light` and the shapes of `rects` and `texts` from `a_marked_cell_is_tinted_and_its_row_is_marked` as it stands in the file: the test above is written to its shape, not copied from it. In `mod tests` of `src/ui/mod.rs`:

```rust
    #[test]
    fn a_new_rows_cells_say_what_the_database_will_do() {
        for look in Look::ALL {
            let (mut harness, tab, id) = covers_in(look);
            harness.app.workspace_mut(tab).unwrap().row_panel = false;
            let place = crate::edit::Place::Top;
            harness.app.apply(Action::AddRow { tab, id, place });
            harness.app.apply(Action::CancelEdit { tab, id });
            let tree = harness.settle();
            // The row is one a screen reader finds, by what it is.
            assert!(
                crate::testing::node(&tree, "New row, not saved", Role::Button).is_some(),
                "{}",
                look.name
            );
            // And the page's rows keep their numbers under it.
            assert!(crate::testing::node(&tree, "Row 1", Role::Button).is_some());
            let marker = if look.terminal { "new" } else { "+ new" };
            assert!(painted(&harness, marker), "{}: {:?}", look.name, harness.painted);
            // `created_at`'s default, as the database writes it.
            assert!(painted(&harness, "CURRENT_TIMESTAMP"), "{}", look.name);
            // `publisher_id` is asked for: in words, or by the header's star.
            assert_eq!(painted(&harness, "required"), !look.terminal, "{}", look.name);
            assert_eq!(painted(&harness, "*"), look.terminal, "{}", look.name);
            // The count says it.
            let summary = if look.terminal {
                "3 rows + 1 new · 5 cols"
            } else {
                "3 rows + 1 new · 5 columns · main"
            };
            assert!(harness.has(summary), "{}", look.name);
            // Set, the cell shows its value and asks for nothing more.
            make_pending(&mut harness, tab, id, (crate::edit::new_row(0), 1), "9100000000000000004");
            harness.settle();
            assert!(painted(&harness, "9100000000000000004"), "{}", look.name);
            assert!(!painted(&harness, "required"), "{}", look.name);
            assert!(!painted(&harness, "*"), "{}", look.name);
        }
    }

    #[test]
    fn the_row_panel_says_a_new_row_is_edited_in_the_grid() {
        let (mut harness, tab, id) = covers_in(Look::macos());
        let place = crate::edit::Place::Top;
        harness.app.apply(Action::AddRow { tab, id, place });
        assert!(harness.has("A new row is edited in the grid"));
        // And Mod+I, which puts the keyboard on a row's fields, leaves it.
        harness.app.apply(Action::CancelEdit { tab, id });
        harness.press(Key::I, Modifiers::COMMAND);
        assert!(!harness.ctx.text_edit_focused());
    }
```

  `Role` is `egui::accesskit::Role`; the terminal look writes a number's digits as the page gives them, so the pending value reads the same in every look. If the fixture's subtitle differs (the schema is `main`), take the text from a first run.

- [ ] **Step 2: Run them, and see them fail.**

- [ ] **Step 3: The grid's row.** In `src/edit.rs`, `RowMark` gains `/// A row the table does not hold yet.` `New,` after `Changed`. In `src/ui/grid.rs`:

```rust
/// A row as the grid needs it besides its cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Row {
    pub mark: RowMark,
    /// Its number among the table's rows, from 1, for a screen reader.
    /// `None` for a row the table does not hold yet.
    pub number: Option<u64>,
}
```

  `show` loses `first_row_number` and takes `rows: &dyn Fn(usize) -> Row` in place of the mark's closure. In the row's loop:

```rust
                let Row { mark, number } = rows(row);
                let label = match number {
                    Some(number) => format!("Row {number}"),
                    None => "New row, not saved".to_owned(),
                };
```

```rust
                let row_tone = match mark {
                    RowMark::None => None,
                    RowMark::Changed => Some(Tone::Warning),
                    RowMark::Trouble => Some(Tone::Danger),
                    RowMark::New => Some(Tone::Success),
                };
                let new = mark == RowMark::New;
```

  A new row is filled in its tone unless it is the selected one, whose colour says where the keyboard is (the bar and the gutter still say it is new): in the `else` of `let fill = if lit_row`, `row_fill(…)` becomes

```rust
                    let fill = row_fill(selected_row, response.hovered(), row % 2 == 1, look, palette);
                    match fill {
                        _ if new && !selected_row => Some(Tone::Success.fill(look, palette)),
                        fill => fill,
                    }
```

  The gutter's sign: `let sign = match tone { Tone::Danger => "!", Tone::Success => "+", _ => "~" };`, written as a `match` over the three tones the row can have if `Tone` has more and the house rule asks for every arm. The 3 pt bar and the key column's tone need nothing: they follow `row_tone`.

  `Mark` gains two, after `Gone`:

```rust
    /// The cell that marks a new row: its text in the row's tone.
    Added,
    /// A cell of a new row nothing is set in: what the database will fill
    /// it with, quieter than a value.
    Unset,
```

  Neither tints its cell (`Mark::None | Mark::Locked | Mark::Gone | Mark::Added | Mark::Unset => None` in the `tone` match), and each writes in its own colour: in the `written` match, before the `locked` arm,

```rust
                        _ if content.mark == Mark::Added => {
                            written_in(palette, Tone::Success.color(palette))
                        }
                        _ if content.mark == Mark::Unset => written_in(palette, palette.dim),
```

  The terminal writes the marker dimmed, as its board has it: make the first of the two arms `_ if content.mark == Mark::Added && !look.terminal`, and the second `Mark::Unset | Mark::Added`.

  `Column` gains `/// A new row needs a value in it: the terminal's header says so.` `pub required: bool,`. In `draw_header`, where the name is painted in the terminal look, paint `*` a space's width after it in `Tone::Danger.color(palette)` when `column.required && look.terminal`, in the name's role and as a piece of its own (the test looks for the painted text `*`); the column's accessible name stays its name. Every `Column { .. }` built elsewhere (`src/ui/sql_results.rs`, the tests' `columns_that`) gets `required: false`, and every caller of `show` passes `&|row| Row { mark: RowMark::None, number: Some(first + row as u64 + 1) }` with the `first` it passed as `first_row_number`.

- [ ] **Step 4: The new row's cells.** In `src/ui/data_view.rs`, `Changes` gains what it needs of the table, read where `computed` is (`Table::of`, before the tab is borrowed for its editor):

```rust
    /// What each column holds in a new row nothing is set in, by the
    /// page's columns. Empty where the table takes no row.
    unset: Vec<Unset>,
    /// The column a new row's marker stands in: the first the database
    /// assigns.
    marker: Option<usize>,
```

  made by

```rust
/// What each of the page's columns holds in a new row that sets nothing
/// in it. None where there is no new row to show it in.
fn unset_columns(workspace: &crate::model::Workspace, object: &ObjectTab) -> Vec<Unset> {
    if object.edits.added.is_empty() {
        return Vec::new();
    }
    let Some(table) = Table::of(workspace, object) else {
        return Vec::new();
    };
    (0..table.page.columns.len())
        .map(|col| match table.column(col) {
            Some(column) => crate::edit::unset(table.dialect, column),
            // A column the structure does not list is left to the database.
            None => Unset::Assigned,
        })
        .collect()
}
```

  The why of a locked cell is said on a new row's cell as on any: lift the first block of `Changes::mark` (the one that sets `cell.note`) into `fn say_why(&self, cell: &mut Cell<'_>, at: (usize, usize))`, and call it from `mark`. Then:

```rust
    /// The cell `at` (row, column) of a new row: what is set in it, as
    /// `value` draws it, or what the database will do with it.
    fn new_cell<'c>(
        &self,
        at: (usize, usize),
        value: impl FnOnce(&Value) -> Cell<'c>,
        column: &tabletist_db::ColumnMeta,
        look: &Look,
        locale: crate::i18n::Locale,
    ) -> Cell<'c> {
        let say = |text: &'static str| gettext(locale, text).into_owned();
        let mut cell = match self.cells.get(&at) {
            Some(pending) => {
                let mut cell = kept(value(&drawn(&pending.new)));
                // On the row's own fill a value that waits needs no tint of
                // its own: the row says it is pending.
                self.mark_state(&mut cell, pending, None, column, look, locale);
                cell
            }
            None => match self.unset.get(at.1) {
                Some(Unset::Assigned) if self.marker == Some(at.1) => Cell {
                    text: if look.terminal { say("new") } else { format!("+ {}", say("new")) }.into(),
                    mark: Mark::Added,
                    ..Cell::default()
                },
                Some(Unset::Assigned) | None => Cell {
                    mark: Mark::Unset,
                    hint: Some(say("Assigned by the database on save")),
                    ..Cell::default()
                },
                Some(Unset::Default(text)) => Cell {
                    text: format::cell_line(text, Marks::PLAIN).into_owned().into(),
                    mark: Mark::Unset,
                    hint: Some(say("from DEFAULT")),
                    ..Cell::default()
                },
                Some(Unset::Null) => Cell {
                    mark: Mark::Unset,
                    ..kept(value(&Value::Null))
                },
                // The terminal asks in its header and its status line.
                Some(Unset::Required) if look.terminal => Cell::default(),
                Some(Unset::Required) => Cell {
                    text: say("required").into(),
                    mark: Mark::Trouble,
                    hint: Some(say("A save needs a value here")),
                    ..Cell::default()
                },
            },
        };
        self.say_why(&mut cell, at);
        cell
    }
```

  `mark_state` is the `match &pending.state` of `Changes::mark`, lifted out with the loaded value optional: `State::Ready` with `Some(loaded)` is what it is today (`Mark::Pending`, the "was" hint), and with `None` it is `Mark::Saving` while a save runs and `Mark::None` otherwise, with no hint. `mark` calls it with `Some(loaded)`. Use `format::cell_line` as `edit::read` does to cut a default to a cell's worth; if its `Marks` are the view's, pass `grid::marks(&ctx, &look)`.

  In `show`, the new row's branch of the cell closure (task 8) becomes one call:

```rust
                if crate::edit::new_id(row).is_some() {
                    let value = |value: &Value| cell(&ctx, value, column, &tags[col], &look, fits[col]);
                    return changes.new_cell((row, col), value, column, &look, locale);
                }
```

  the rows' closure is

```rust
            &|place| match order.row(place) {
                Some(row) => match crate::edit::new_id(row) {
                    Some(id) => grid::Row {
                        // In trouble where a value of it is, or its INSERT failed.
                        mark: match crate::edit::row_mark(changes.cells, row) {
                            RowMark::Trouble => RowMark::Trouble,
                            _ if failed(id) => RowMark::Trouble,
                            _ => RowMark::New,
                        },
                        number: None,
                    },
                    None => grid::Row {
                        mark: crate::edit::row_mark(changes.cells, row),
                        number: Some(object.query.offset + row as u64 + 1),
                    },
                },
                None => grid::Row::default(),
            },
```

  with `let failed = |id: usize| object.edits.added.iter().any(|new| new.id == id && new.failed.is_some());` before the call, and each `Column` says whether it is asked for: `required: !object.edits.added.is_empty() && lacking.contains(&col)`, where `lacking` is the columns some new row still misses, read with `computed` through `Table::missing` over `object.edits.added`.

- [ ] **Step 5: The count, the footer, the panel.** In `subtitle`, the rows' part says the new ones:

```rust
    let added = object.edits.added.len();
    let new = (added > 0).then(|| format!("{added} {}", gettext(locale, "new")));
    let rows = rows.map(|rows| {
        let noun = if rows == 1 { "row" } else { "rows" };
        format!("{} {}", format::group_digits(rows), gettext(locale, noun))
    });
    match (rows, new) {
        (Some(rows), Some(new)) => parts.push(format!("{rows} + {new}")),
        (Some(one), None) | (None, Some(one)) => parts.push(one),
        (None, None) => {}
    }
```

  In `footer`, `state_note` takes what is selected as text: `Some("1 row selected")`, or for a new row "New row" and, while it lacks values, how many: "New row · 1 required field" (`ngettext(locale, "required field", "required fields", n)`), counted with `Table::missing`. Its `selected: bool` becomes `selected: Option<String>`; correct its callers and any test of it.

  The terminal's line under the grid (`error_line`) names the first cell in trouble by its row's number, `! 4:publisher_id  …`, as `object.query.offset + row as u64 + 1`. A new row has no number: where `crate::edit::new_id(row).is_some()` the line reads `! new:publisher_id  …` (the word through `gettext` and the look's `label`). Add beside the test of that line in `src/ui/mod.rs`:

```rust
    #[test]
    fn the_terminals_error_line_names_a_new_row_by_what_it_is() {
        let (mut harness, tab, id) = covers_in(Look::omarchy());
        let place = crate::edit::Place::Top;
        harness.app.apply(Action::AddRow { tab, id, place });
        make_pending(&mut harness, tab, id, (crate::edit::new_row(0), 1), "harbor");
        harness.settle();
        let line = harness.painted.iter().find(|(text, _)| text.starts_with("! "));
        let line = line.map(|(text, _)| text.as_str()).unwrap_or_default();
        assert!(line.starts_with("! new:publisher_id"), "{line}");
    }
```

  In `draw` of `src/ui/row_panel.rs`, the branch that says "Select a row to see its fields" says of a new row what it can: when `source.selection` is on a new row (`crate::edit::new_id(cell.row).is_some()`), the text is `gettext(locale, "A new row is edited in the grid")`. Nothing else of the panel changes in this run.

- [ ] **Step 6: Run the tests, then the four checks.** `the_header_gives_way_instead_of_overlapping_in_a_narrow_view` runs at 1000 pt in all three looks: the button is wider by its key, and gives its text and key up sooner. If it overlaps, the fault is in `add_row(short).width`, not in the test.

- [ ] **Step 7: Commit.**

```bash
git add src && git commit -m "Draw a new row as one: its fill, its marker, and what fills each unset cell"
```

---

### Task 10: The words: the pending bar, the terminal's line, the confirmation

Everything that counts what is pending counts the new rows, and says why a save waits for one.

**Files:**
- Modify: `src/app/editing.rs` (`App::lacking`)
- Modify: `src/ui/pending_bar.rs`, `src/ui/workspace.rs`, `src/ui/write_prompts.rs`, `src/ui/review.rs`
- Test: `src/ui/mod.rs`

What each place says, with 1 new row and, where it matters, 2 changes in 1 row:

| Where | Today | With new rows |
|---|---|---|
| The bar's count | "2 changes in 1 row" | "1 new row" · "1 new row · 2 changes in 1 row". Short of room: "1 new row · 2 changes", then "1 new row" |
| The bar, a value missing | | after the count, in the danger tone: "publisher_id is required", or "3 values are required" for more than one |
| Save, disabled | "Fix 1 value to save" | "Fill required fields to save" (`SaveBlock::Required`, task 7) |
| The bar is there | while cells are pending | while anything is (`Edits::pending`) |
| The footer and the terminal, after a save | "written 2 changes · 1 row · 14 ms" | "written 1 new row · 5 ms" · "written 1 new row · 2 changes · 1 row · 14 ms" |
| The bar, an `INSERT` that failed | | "Nothing was saved. 1 new row failed, so the whole transaction rolled back." and then the database's own words, as a failed row's has them |
| The terminal's line, counts | "2 pending · 1 row" | "+1 new" in the success tone before it; alone where only a row is new |
| The terminal's line, a value missing | | "publisher_id required" in the danger tone |
| The terminal's keys | `o new row` struck through | `o new row` live where the table takes a row, struck through where it does not. `dd delete` stays struck: no row of the table is deleted yet |
| `:diff`, Review SQL, `Mod+S` | with cells pending | with anything pending |
| The review's head (terminal) | "pending · 2 changes · 1 row" | "pending · 1 new row · 2 changes · 1 row", and "pending · 1 new row" |
| The confirmation's title | "Save 2 changes to production?" | "Save 1 new row and 2 changes to production?" · "Save 1 new row to production?" |
| Under it | "Bookshop · … · 1 row in book_covers" | the same; with no changed row, "Bookshop · … · book_covers" |
| The terminal's box | "write 2 changes?" | "write 1 new row and 2 changes?" |
| Leaving | "Discard 2 changes before closing the tab?" | a new row counts as one change there (task 6): "Discard 3 changes …" |

- [ ] **Step 1: Write the failing tests.** In `mod tests` of `src/ui/mod.rs`:

```rust
    #[test]
    fn the_bar_counts_new_rows_and_says_what_one_still_needs() {
        let (mut harness, tab, id) = covers_in(Look::macos());
        let place = crate::edit::Place::Top;
        harness.app.apply(Action::AddRow { tab, id, place });
        harness.app.apply(Action::CancelEdit { tab, id });
        assert!(harness.has("1 new row"));
        assert!(harness.has("publisher_id is required"));
        // Save is there and cannot be pressed, and says why.
        let tree = harness.settle();
        let save = crate::testing::node(&tree, "Save", Role::Button).expect("Save");
        let (_, node) = tree.nodes.iter().find(|(id, _)| *id == save).unwrap();
        assert!(node.is_disabled());
        assert_eq!(node.description(), Some("Fill required fields to save"));
        // Filled, and with a changed cell beside it.
        let new = crate::edit::new_row(0);
        make_pending(&mut harness, tab, id, (new, 1), "9100000000000000004");
        make_pending(&mut harness, tab, id, (1, 2), "audio");
        assert!(harness.has("1 new row · 1 change in 1 row"));
        assert!(!harness.has("publisher_id is required"));
        // Its statements are there to review, the INSERT first.
        harness.click("Review SQL");
        assert!(harness.has("-- new row"));
        assert!(harness.has("VALUES (9100000000000000004)"));
    }

    #[test]
    fn a_save_says_how_many_rows_it_added() {
        let (mut harness, tab, id) = covers_in(Look::macos());
        let place = crate::edit::Place::Top;
        harness.app.apply(Action::AddRow { tab, id, place });
        make_pending(&mut harness, tab, id, (crate::edit::new_row(0), 1), "9100000000000000004");
        harness.click("Save");
        let stored = vec![
            Value::Int(15),
            Value::Int(9_100_000_000_000_000_004),
            Value::Text("print".into()),
            Value::Null,
            Value::Text("2026-10-07 10:42:09".into()),
        ];
        harness.answer_written(Ok(tabletist_db::WriteOutcome::Written {
            inserted: vec![Some(stored)],
            rows: Vec::new(),
            elapsed: std::time::Duration::from_millis(5),
        }));
        assert!(harness.has("written 1 new row · 5 ms"));
        // The row is the table's now, with the id the database gave it.
        assert!(painted(&harness, "15"));
        assert!(harness.has("4 rows · 5 columns · main"));
        let tree = harness.settle();
        assert!(crate::testing::node(&tree, "New row, not saved", Role::Button).is_none());
    }

    #[test]
    fn a_failed_insert_says_that_nothing_was_saved() {
        let (mut harness, tab, id) = covers_in(Look::macos());
        let place = crate::edit::Place::Top;
        harness.app.apply(Action::AddRow { tab, id, place });
        make_pending(&mut harness, tab, id, (crate::edit::new_row(0), 1), "9100000000000000099");
        harness.click("Save");
        harness.answer_written(Ok(tabletist_db::WriteOutcome::FailedInsert {
            insert: 0,
            error: tabletist_db::Error::query("FOREIGN KEY constraint failed"),
        }));
        let said = "Nothing was saved. 1 new row failed, so the whole transaction rolled back. \
                    FOREIGN KEY constraint failed.";
        assert!(harness.has(said), "{:?}", harness.painted);
        // The row is still there to fix, and still counted.
        assert!(harness.has("1 new row"));
        let tree = harness.settle();
        assert!(crate::testing::node(&tree, "New row, not saved", Role::Button).is_some());
    }

    #[test]
    fn the_terminals_line_counts_new_rows_and_offers_its_key() {
        let (mut harness, tab, id) = covers_in(Look::omarchy());
        harness.app.workspace_mut(tab).unwrap().row_panel = false;
        harness.settle();
        let palette = harness.app.palette;
        // The key is live where a row can be added.
        assert!(painted_in(&harness, "o new row", palette.text), "{:?}", harness.painted);
        select(&mut harness, tab, id, (0, 1));
        type_key(&mut harness, Key::O, "o");
        harness.press(Key::Escape, Modifiers::NONE);
        assert!(painted(&harness, "+1 new"), "{:?}", harness.painted);
        assert!(painted(&harness, "publisher_id required"), "{:?}", harness.painted);
        // `:diff` has something to show with nothing but a new row.
        harness.app.apply(Action::ReviewEdits {
            tab,
            id,
            show: true,
        });
        harness.settle();
        assert!(painted(&harness, "pending · 1 new row"), "{:?}", harness.painted);
    }

    #[test]
    fn the_confirmation_counts_new_rows() {
        let (mut harness, tab, id) = covers_in(Look::macos());
        harness.app.workspace_mut(tab).unwrap().environment = crate::env::Environment::Production;
        let place = crate::edit::Place::Top;
        harness.app.apply(Action::AddRow { tab, id, place });
        make_pending(&mut harness, tab, id, (crate::edit::new_row(0), 1), "9100000000000000004");
        harness.app.apply(Action::WriteEdits { tab, id });
        assert!(harness.has("Save 1 new row to production?"));
        harness.app.apply(Action::CancelWrite);
        make_pending(&mut harness, tab, id, (1, 2), "audio");
        harness.app.apply(Action::WriteEdits { tab, id });
        assert!(harness.has("Save 1 new row and 1 change to production?"));
    }
```

  `painted_in` and `painted` find whole painted pieces: where the status line paints a key and its word as two pieces, assert them as the neighbouring tests of that line do (`the_prompts_keys_are_offered_where_a_save_can_be_made`). Rewrite `the_keys_struck_through_are_the_ones_still_to_come`: on its writable table `o new row` is live now, and only `dd delete` is struck.

- [ ] **Step 2: Run them, and see them fail.**

- [ ] **Step 3: What a new row still needs.** In `src/app/editing.rs`:

```rust
    /// The columns the tab's new rows still need a value in, by name, each
    /// once, in the page's order: what the bar and the terminal's line say
    /// a save waits for.
    pub fn lacking(&self, tab: ConnTabId, id: TabId) -> Vec<String> {
        self.table(tab, id, |table, object| {
            let mut cols: Vec<usize> = object
                .edits
                .added
                .iter()
                .flat_map(|new| table.missing(new.id, &object.edits.cells))
                .collect();
            cols.sort_unstable();
            cols.dedup();
            cols.into_iter()
                .filter_map(|col| table.page.columns.get(col))
                .map(|column| crate::ui::format::display_safe(&column.name).into_owned())
                .collect()
        })
        .unwrap_or_default()
    }
```

- [ ] **Step 4: The bar.** In `src/ui/pending_bar.rs`:

  - The bar is there for a new row and counts it since task 8 (`counts_text`).
  - What a row still needs, beside `fix` and drawn as it is (the alert icon, the danger tone), from `app.lacking(tab, id)` read before the bar's closure:

```rust
            let needs = match lacking.as_slice() {
                [] => None,
                [one] => Some(format!("{one} {}", gettext(locale, "is required"))),
                more => Some(format!(
                    "{} {}",
                    more.len(),
                    gettext(locale, "values are required")
                )),
            };
```

    It gives way before the counts do, as `fix` does. Name it for a screen reader as the counts are named (`named(ui, place, "pending-needs", …)`).
  - `written_text`:

```rust
pub fn written_text(saved: &Saved, locale: Locale) -> String {
    let mut parts = Vec::new();
    if saved.added > 0 {
        parts.push(counted(locale, saved.added, "new row", "new rows"));
    }
    // A save of new rows alone changed none.
    if saved.changes > 0 || saved.added == 0 {
        parts.push(counted(locale, saved.changes, "change", "changes"));
        parts.push(counted(locale, saved.rows, "row", "rows"));
    }
    parts.push(format::elapsed(saved.elapsed));
    format!("{} {}", gettext(locale, "written"), parts.join(" · "))
}
```

  - `note_said`, the arm task 7 left as a failed row's:

```rust
        Note::FailedInsert { error } => {
            let failed = own("Nothing was saved. 1 new row failed, so the whole transaction rolled back.");
            // The database's words as a failed row's has them, without the
            // "Nothing was written." that the sentence before already says.
            format!("{failed} {}", /* the code and the sentence, as `Note::Failed` builds them */)
        }
```

    Build the database's part with the same code the `Note::Failed` arm uses for `"{code} · {sentence(message)}"`, lifted into a small function both arms call.

- [ ] **Step 5: The terminal's line.** In `editing_status` of `src/ui/workspace.rs`:

  - `Editing` gains `/// The new rows: "+1 new".` `added: Option<String>,` and `/// `o` would add a row.` `can_add: bool,`. `added` is `(counts.added > 0).then(|| format!("+{} {}", counts.added, say("new")))`, and `can_add` is `data && Table::of(workspace, object).is_some_and(|table| table.no_rows().is_none())`.
  - `blocked` is asked for while `edits.pending()`, and words `SaveBlock::Required` by the column: `"{name} required"` for one, `"{n} values required"` for more, from `app.lacking(tab, object.id)`, in the danger tone; every other block keeps `block_text`.
  - "nothing pending" is said while `!edits.pending()`.
  - `Note::FailedInsert` is said by the arm every other note has (the failed mark, then its sentence), without the tail a failed row's has: its sentence says already that the transaction rolled back.
  - Where the line is drawn (`status_line` and `command_line`, the loop over `(&editing.errors, Danger)` and `(&editing.pending, Warning)`): `(&editing.added, states::Tone::Success)` joins it, drawn before `pending`.
  - The keys: `("o", "new row", true)` joins `table_hints` after `i` when `editing.can_add`, and `"o new row"` leaves the struck-through list in that case and stays in it otherwise. `:diff review` is offered while `editing.pending.is_some() || editing.added.is_some()`.

- [ ] **Step 6: The review's head and the confirmation.** In `terminal_head` of `src/ui/review.rs`, the counts take `review.added` too: "pending", then "1 new row" where there is one, then the changes and their rows where there are any, joined with " · ". Pass `(review.added, review.changes, review.rows)`.

  In `src/ui/write_prompts.rs`, `Facts::changes` is what the title counts:

```rust
    let changed = counted(locale, prompt.changes, "change", "changes");
    let added = counted(locale, prompt.added, "new row", "new rows");
    let what = match (prompt.added, prompt.changes) {
        (0, _) => changed,
        (_, 0) => added,
        _ => format!("{added} {} {changed}", gettext(locale, "and")),
    };
```

  `Facts::rows` is "1 row in book_covers" while the save changes a row, and the table's name alone while it only adds: `if prompt.rows == 0 { table.clone() } else { … }`. `Facts::columns` names the columns the new rows set too: chain `prompt.changeset.inserts.iter().flat_map(|row| &row.set).map(|value| &value.column)` before the changed rows' columns, with the same check for a name already listed.

- [ ] **Step 7: Run the tests, then the four checks.** Every test that asserts today's sentences ("2 changes in 2 rows", "written 2 changes · 2 rows · 14 ms", "3 pending · 2 rows", "Save 2 changes to production?") passes unchanged: with no new row each sentence is built as it was.

- [ ] **Step 8: Commit.**

```bash
git add src && git commit -m "Count new rows wherever pending changes are counted, and say what one still needs"
```

---

### Task 11: The pages

`AGENTS.md`: the pages follow the keys and what a user sees. Nothing tests them against `SHORTCUTS`; `docs/bin/check` builds the site and checks its links.

**Files:**
- Modify: `docs/_reference/keyboard-shortcuts.md`, `docs/_guide/macos.md`, `docs/_guide/omarchy.md`, `docs/_guide/editing-data.md`

- [ ] **Step 1: The reference.** In `docs/_reference/keyboard-shortcuts.md`, the Connections row becomes `| Mod+N | New connection. With a table's rows in front, [a new row]({% link _guide/editing-data.md %}#adding-rows) |`, and the Editing table gains, after "Edit the cell":

```markdown
| Mod+N | Add a row, at the top of the grid |
| Delete | Drop a new row that is not saved yet |
```

- [ ] **Step 2: macOS.** In `docs/_guide/macos.md`: the same two rows in its Editing table, written `Cmd+N` and `Delete` (the page says which key a Mac labels so: keep its convention, and say Backspace does the same), and its "Everywhere" row for `Cmd+N` gains "With a table's rows in front it adds a row".

- [ ] **Step 3: Omarchy.** In `docs/_guide/omarchy.md`, its Editing table gains, after `cc`:

```markdown
| `o`, `O` | Add a row below the cursor's row, or above it |
| `dd` | Drop a new row that is not saved yet |
```

  Ctrl+N stays "New connection" there: the look's key for a row is `o`.

- [ ] **Step 4: The guide.** In `docs/_guide/editing-data.md`, a section `## Adding rows` after the one on saving, in the page's voice and with its Cmd/Ctrl convention. What it must say, each from the code as built:

  - Add row in the header, Cmd/Ctrl+N, or `o` and `O` in the Omarchy look; where the row appears in each look.
  - The row is green until it is saved, and what its cells show before anything is typed: a required value, a default, NULL, a value the database assigns.
  - It is saved with everything else that is pending, in one transaction, and Save waits while a required value is missing.
  - Only the columns that were set are sent. Review SQL shows the `INSERT`.
  - Delete (or `dd`) drops a new row. Discard all drops every new row with the other changes.
  - After the save the row shows what the database stored. On a table with a trigger, and on MySQL where the row cannot be found again, the page is loaded again instead.
  - A table without a primary key takes new rows, and they cannot be edited afterwards.
  - What is not there yet, in one line: duplicating a row, pasting rows, and the row panel as a form for the new row.

- [ ] **Step 5: Build the site.** `cd docs && bundle install && bin/check` passes. If Ruby is not set up in the session, say so in the task's report and in the pull request: `docs.yml` builds it there.

- [ ] **Step 6: Commit.**

```bash
git add docs/_reference docs/_guide && git commit -m "Say in the pages how a row is added"
```

---

### Task 12: See it, and say what is built

**Files:**
- Modify: `src/shots.rs` (the scenes' structure, two scenes)
- Modify: `docs/superpowers/specs/2026-10-07-inserting-rows-design.md` (its status paragraph)

- [ ] **Step 1: The scenes' table numbers its own ids.** `structure()` of `src/shots.rs` gives `id` the default `nextval('book_images_id_seq'::regclass)` with `identity: false`: a new row there would show the sequence call as a default. Make it `identity: true`, as PostgreSQL describes a serial since run 1.

- [ ] **Step 2: Two scenes.** In `src/shots.rs`, with the editing scenes (the array's length is in its type: `10` becomes `12`):

```rust
/// A new row at the top, its first required cell open: the green row, its
/// marker, the defaults dimmed, and the bar that waits for a value.
fn edit_new_row(harness: &mut Harness) {
    let (tab, id) = editable(harness);
    let place = crate::edit::Place::Top;
    harness.app.apply(Action::AddRow { tab, id, place });
}

/// A new row whose save the database refused, beside a changed cell: the
/// row in red, and the bar saying that nothing was saved.
fn edit_new_row_failed(harness: &mut Harness) {
    let (tab, id) = editable(harness);
    let place = crate::edit::Place::Top;
    harness.app.apply(Action::AddRow { tab, id, place });
    let new = crate::edit::new_row(0);
    retype(harness, tab, id, (new, BOOK_ID), "9100000000000000099");
    harness.app.apply(Action::WriteEdits { tab, id });
    harness.answer_written(Ok(tabletist_db::WriteOutcome::FailedInsert {
        insert: 0,
        error: tabletist_db::Error::Query {
            code: Some("23503".into()),
            message: "insert or update on table \"book_images\" violates foreign key constraint \"fk_book\"".into(),
            detail: None,
            hint: None,
        },
    }));
    step_aside(harness, tab, id);
}
```

  with `("edit-new-row", edit_new_row)` and `("edit-new-row-failed", edit_new_row_failed)` in `EDITING`. Build `Error::Query` as `edit_failed` builds its own (its fields are the ones that scene names). If every required column of `book_images` must be set for the save to go out, `retype` each: the scene is reached as a user reaches it.

- [ ] **Step 3: Look at them.** They render here without a display:

```bash
~/.cargo/bin/cargo test --locked --features shots --lib shots -- --ignored
```

  Read `target/shots/edit-new-row-{standard,macos,omarchy}-{light,dark}.png` and the failed scene's six beside the canvas' `MacInsert` and `OmarchyInsert` boards (read them again with the Artifact tool), against the table of task 9. What differs and is not a row of "What the design asks, and what gets built" is a fault to fix in this task, or a row to add there and name in the pull request. The pictures stay in `target/`: none is committed, none goes to GitHub, and no test compares one.

- [ ] **Step 4: The spec's status.** Replace the first paragraph of `docs/superpowers/specs/2026-10-07-inserting-rows-design.md` with:

```markdown
Date: 2026-10-07. Status: built in runs. Run 1
(`docs/superpowers/plans/2026-10-07-inserting-rows-1-save.md`) is built: a
save carries new rows and the three drivers write them. Run 2
(`docs/superpowers/plans/2026-10-07-inserting-rows-2-add-row.md`) is
built: a row is added from the grid, filled, reviewed and saved. The row
panel as the "New row" form is run 2b. Each plan's "What the design asks,
and what gets built" says what of the text below the app can do, and in
which run.
```

- [ ] **Step 5: Run the four checks and the shots' clippy, then commit.**

```bash
git add src/shots.rs docs/superpowers && git commit -m "Show a new row in the scenes, and say that adding one is built"
```

---

## After the last task

- Run the four checks once more on the branch's head, and the shots' clippy. Report what ran and what was only compiled. Nothing here is platform code; say that macOS and Windows were not built, and type-check any `cfg` code the run did touch in a scratch crate.
- The website's screenshots (`assets/screenshots/`) are not touched by this run. A picture of a new row for the guide is the user's to ask for.
- The pull request: retitle it "Add a row in the grid", and write its description as run 1's was: summary, where it departs from the spec (the table above, as built), the spec's tests with the test that covers each, what was run. No screenshots unless blurred.
- **By hand, for the user** (the window does not open in a session): on the Bookshop's development database, on each look,
  1. Add row, type a publisher, Tab through the row, Save. The row turns green for a moment and shows its id.
  2. Add row, Save at once: Save is disabled and says why.
  3. Add two rows, drop one with Delete (`dd`), Discard all.
  4. A publisher that does not exist: the save fails, the row stays, the bar says nothing was saved.
  5. Review SQL with a new row and a changed cell: the `INSERT` first.
  6. On MySQL and on a PostgreSQL table with a trigger: the page loads again after the save and the row is where the sort puts it.
  7. A read-only connection and a view: Add row is disabled with its reason, and `Mod+N` opens a new connection.
- Stop here. Run 2b (the row panel as the "New row" form) gets its own plan, written from the tree as this run leaves it.

## As built

The run followed the plan. Where the code that was built differs from a block above, this is how, and the code is right:

- **Task 2.** `Edits::put` and `Edits::revert` share one `Edits::mended` for taking a failure off a new row, where the plan repeats the lines in each. A test of it was added.
- **Task 4.** The test asserts the shape of the long literal (its quotes, that it holds the statement's own words) and what stands around it, not one spelling of it for the three engines.
- **Task 5.** One more test: a long value of a new row is cut where it is shown and whole for the clipboard.
- **Task 7.** The words of `Note::FailedInsert` were written here, with `database_said` lifted out for it and `Note::Failed`, not left to task 10. One more test: rows opened below two different rows keep those places once saved.
- **Task 8.** A held `Mod+N` is tested as egui reports one: a key that goes down while it is down. An event built with `repeat: true` is read as a fresh press once the key was released. One more test: a first `d` that brought the rows up from the Structure view is not half of `dd`.
- **Task 9.** The star after a required column's name is given room of its own, so it shows where the header stands at the right (a numeric column): seen in the scenes. The row panel's line is "A new row is edited in the grid": the longer sentence ran out of the panel in the terminal look. The footer's note is `selection_note`. A grid test reads a new row and the page's rows by their names.
- **Task 10.** What a new row needs stands in the bar where "1 to fix" stands, as one text with it ("1 to fix · publisher_id is required"), so it gives way and is named as that note is. The terminal's "publisher_id required" is said when a save is asked for and refused, as every reason a save waits is said there.
- **Task 12.** Three scenes: the new row with its editor open, the row with a value given beside a changed cell, and the failed save. The terminal look opens its row below the first, as `o` does. `book_images` has two required columns (`book_id`, `created_at`), so the failed scene gives both.
- **Not sent round the reviewer a second time.** The plan's two faults were corrected after one review.

What the scenes showed that the plan's table does not list: in a window too narrow for the bar's words (the scenes' 1000 pt with the row panel open), what a new row needs gives way before the counts do, and with a changed cell beside it the counts give way to the bar's dot, whose name still says them. That is how the bar treats "1 to fix" today.
