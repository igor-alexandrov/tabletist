# Deleting many rows at once

Run 4 of [deleting rows](2026-10-12-deleting-rows-design.md). Runs 1 to 3
built the delete of one row at a time: the mark, the save, what a delete
takes with it and who holds a row. This spec covers what that spec asked
for and the app could not yet do:

- marking several selected rows in one action;
- `d` on a visual row selection and `{count}dd` in the Omarchy look;
- the cap of 1,000 rows per action, with its message;
- deletes sent in batches of up to 100 rows, `WHERE id IN (…)`.

The rules of the first spec stand: Bookshop demo data only, no pixel or
design snapshot in any test run, no unblurred screenshot on GitHub, every
commit passes `cargo fmt --check`, `cargo clippy -- -D warnings` and the
headless tests, UI copy is short and plain with no em dashes.

## What is there today

- The grid selects one cell: `ObjectTab.selection: Option<CellPos>`. The
  grid reports a click and a double click, with no modifiers. The footer
  of the desktop looks says `1 row selected`.
- The Omarchy grid has no visual mode and reads no count. `v y` and `v p`
  are two-key spellings of copy and paste for one cell. Digits typed on
  the grid do nothing.
- `Action::DeleteRow { tab, id }` marks the selected row, restores it when
  it is marked, and drops it when it is new (`App::delete_row`).
- A save reads each row to delete with its own locking `SELECT`, then sends
  one `DELETE` per row. A row the database refuses is named
  (`WriteOutcome::FailedDelete { delete, error }`), and the app counts who
  holds it.
- Review SQL shows one `-- delete row id 9` comment and one statement per
  row.
- The canvas draws no selection of several rows. Its component boards draw
  the pieces: `Row selected` and `Active cell` (macOS components), and
  `Row cursor`, `selection = sel background` and a `-- VISUAL --` mode
  line (Omarchy components). Neither Settings > Keyboard board lists a key
  that selects rows.

## Decisions that need a yes

The canvas and the first spec leave these open. Each has the default this
spec is written to. Say which to change.

| # | Question | Default | The other way |
|---|---|---|---|
| M1 | What several selected rows are | A run of neighbouring rows, from an anchor row to the cursor's row | Scattered rows (⌘-click) too: a set of rows instead of a run. Not in this run |
| M2 | How the desktop looks select them | Shift+click and Shift+↑ / Shift+↓ | Also Select all (⌘A) and dragging over rows. Not in this run |
| M3 | Omarchy's visual row selection | `V` starts it, as vim's linewise visual. The mode line reads `-- VISUAL --` with the row count. `v y` and `v p` stay as built | `v` starts it, and the copy and paste of one cell move to other keys |
| M4 | Keys while it is on | `j` `k` and the arrows extend, `d` marks, `esc` or `V` leaves. Other letters do nothing | Other letters leave visual and act on the cursor's row |
| M5 | A press on rows of both kinds | It marks when any row of the selection is unmarked. It restores only when all are marked | Always mark, and restore one row at a time |
| M6 | Rows of the selection that cannot be deleted (a key that is NULL, a row that is gone) | Skipped, and the message counts them | Nothing is marked, and the first such row's reason is said |
| M7 | Which keys take a count | `dd` only. A count before any other key is dropped | `j` and `k` too |
| M8 | The inspector with several rows selected | It shows the cursor's row. Its Delete and Restore act on that row alone | Its Delete acts on the selection |
| M9 | Other commands with several rows selected (copy row, duplicate, set NULL, yank) | They act on the cursor's row, as today | Copy and duplicate act on every row. That is D3 of the duplication spec, a run of its own |
| M10 | A batch the database refuses | A savepoint before each batch. On a refusal the batch is undone and its rows are sent one at a time, so the error lands on its row | Batches only on PostgreSQL, whose error names the row |
| M11 | Reading the rows before they are deleted | In batches of 100 too. This is an addition: the first spec batches only the `DELETE` | One locking `SELECT` per row, as today. 1,000 marked rows are then 1,000 round trips before the first delete |
| M12 | Cascade counts for a key of several columns, with many rows marked | Above 100 marked rows the line stands without a number | One count per marked row, as today, however many |
| M13 | Where this lands | On this branch, in PR #120: it is the rest of the same spec | A PR of its own, after #120 merges |

## 1. Selecting several rows

The selection keeps its one cell, the **cursor**. It gains a **range**: an
anchor row, and with it every row from the anchor to the cursor's row, in
the order the grid shows them (`edit::Order`, so new rows count where they
stand). With no range, the cursor's row is the one selected row, as today.

```rust
/// The rows selected with the cursor's: from `anchor` to the cursor's row.
pub struct Range {
    /// A row as `CellPos::row` names it: of the page, or a new row.
    pub anchor: usize,
    /// Set by Omarchy's `V`: moves keep the range until it is left.
    pub held: bool,
}
// ObjectTab
pub range: Option<Range>,
```

`ObjectTab::selected_rows()` gives the rows in the grid's order, and is the
one place the rest of the app asks. A SQL result has no range.

Slice 4 of the value editing spec (a block of cells, pasted over) will add
an anchor cell on the same footing. Nothing here stands in its way.

### Desktop looks

| Gesture | What it does |
|---|---|
| Shift+click a row | The range runs from the cursor's row to the clicked row. The cursor moves to the clicked cell. The anchor is the row the cursor was on, or the anchor already set |
| Shift+↓, Shift+↑ | The cursor moves one row and the range follows. The first press sets the anchor on the cursor's row |
| A click, an arrow, Tab, a jump to a row | One row is selected again |

- The footer counts: `3 rows selected`, and `3 rows selected · read-only`.
- Each row of the range is painted as the selected row of its kind is
  today (a new row stays green, a marked row stays struck). Only the
  cursor's cell is the active cell.
- Opening a cell editor, adding a row or duplicating one ends the range:
  they work on the cursor's row.
- A range does not outlive its page. A reload, a sort, a filter, another
  page and a save that removes rows all end it.

### Omarchy look

| Key | What it does |
|---|---|
| `V` | Starts a visual row selection on the cursor's row (`held`). `V` again leaves it |
| `j` `k` `↓` `↑` | Extend it |
| `d` | Marks its rows (section 2), leaves visual, and puts the cursor on the first row of the range |
| `esc` | Leaves it. The cursor stays |

- The mode line reads `-- VISUAL --`, where `-- INSERT --` shows, followed
  by the count and the keys: `-- VISUAL -- 3 rows` and `d delete · esc
  normal`. The words after the mode are by analogy with the insert line:
  the board draws the mode alone.
- The rows of the range take the selection background of the components
  board. The cursor's row and cell keep their own look.
- While it is on, the other letters of the grid do nothing (`i`, `o`, `x`,
  `yy`, `p`, `gd`, a count). The `:` prompt and the chords work as ever.
- A click ends it, as a click ends the wait of a key of several.
- Shift+click works here too and makes a range that is not held. `dd`
  acts on it.

### Keys and where they are spelled

New commands, in `src/keymap.rs` as every key is:

| Command | macOS | Omarchy | Scope |
|---|---|---|---|
| Extend selection down / up | ⇧↓ / ⇧↑ | none | grid |
| Select rows | none | `V` | grid |
| Delete row, on a visual selection | | `d` | grid, when visual |
| Leave visual | | `esc` | grid, when visual |

A new `When::Visual` holds while the tab's range is held. With it, `d`
alone is the whole command and does not wait for a second `d`.

Settings > Keyboard lists them with the grid's commands. Its boards do not
draw them: they are additions to both boards.

## 2. Marking several rows

One action covers the **rows it names**:

- ⌫, the Omarchy `dd`, and the grid's Delete command: the selected rows.
- `d` in visual: the range.
- `{count}dd`: the cursor's row and the `count - 1` rows after it in the
  grid's order, stopping at the last row, as vim does. A range, if one
  stands, is ended.
- Delete and Restore in the inspector: the cursor's row alone (M8).

What it does, in this order:

1. **The cap.** More than 1,000 rows named: nothing changes, and the app
   says `Select fewer rows, or delete in the SQL editor.` The count is of
   the rows named, after `{count}dd` has stopped at the last row. The cap
   is per action. Nothing caps how many rows stand marked.
2. **A reason that holds for the table** (a read-only connection, no
   primary key, a view, a save in flight): nothing changes, and the reason
   is said, in the words of one row's refusal today.
3. **Rows that cannot be deleted on their own account** (a key that is
   NULL or inexact, a row that is gone) are left out and counted.
4. Of the rows left, the saved ones decide:
   - any of them is unmarked: every unmarked one is marked, and every new
     row named is dropped;
   - all are marked: all are restored, and new rows are left alone;
   - there are only new rows: they are dropped.
5. Where step 3 left no row and no new row is named, the first row's
   reason is said, as for one row.

A row is marked and restored exactly as one row is today: its pending
cells and answered doubts move aside under the mark and come back on a
restore, an open cell editor closes and keeps its text, the tab is pinned,
the last save's note goes, and the counts of what the deletes take are
stopped once and asked again when Review SQL next opens.

**Where the cursor goes.** Desktop: nowhere, and the range stays, so a
second ⌫ restores the same rows. `{count}dd`: nowhere. `d` in visual: to
the first row of the range. Where the cursor's row was a new row that was
dropped, the cursor goes to the nearest row of the range that is left.

**What is said.** For one row, today's words. For several:

| Case | Announcement (and toast or status line where one shows today) |
|---|---|
| Marked | `12 rows marked for deletion. 15 to delete.` |
| Marked, some left out | `11 rows marked for deletion. 1 can't be deleted. 14 to delete.` |
| Restored | `12 rows restored.` |
| New rows dropped with them | The sentence above, then `2 new rows dropped.` |
| The cap | `Select fewer rows, or delete in the SQL editor.` |

The sentence about rows left out also shows where a refusal shows: a toast
in the desktop looks, the status line's message in the Omarchy look. The
others are for assistive technology only, as the announcement of one
marked row is.

Rows of a range are exposed as selected to assistive technology. Entering
visual says `Visual. 1 row selected.` and each move says the count.

`Told` grows to carry a count:

```rust
pub enum Told {
    Marked { row: String, deleted: usize },
    Restored { row: String },
    /// Several rows in one action.
    MarkedRows { marked: usize, skipped: usize, dropped: usize, deleted: usize },
    RestoredRows { restored: usize },
}
```

`Action::DeleteRow { tab, id }` acts on the selected rows.
`Action::DeleteRows { tab, id, count }` is `{count}dd`.
`Edits::mark` and `Edits::restore` stay as they are and are called once
for each row.

## 3. Counts

- In the Omarchy grid, digits typed before a key are its count. `0` as the
  first digit is not a count. More than six digits are read as 999,999.
- Only `dd` takes one (M7). Before any other key the count is dropped and
  the key acts once, which is what digits do today.
- The count is read where the letters of a key of several are
  (`Keymap::typed`), which hands the command back with its count. No digit
  is spelled outside `src/keymap.rs`.
- A count waits as the first `d` of `dd` waits, and ends with it: on a
  key that types nothing, a click, a field that takes the keyboard.
- The keyboard reference writes it `{count}dd`. Settings > Keyboard keeps
  `dd`.

## 4. Deletes in batches

The app sends what it sends today: `ChangeSet.deletes`, one `RowDelete`
for each marked row, in `delete_order`. The batches are made in
`tabletist-db`.

### The batches

`write::delete_batches(&[RowDelete]) -> Vec<Range<usize>>` cuts the
deletes, in their order, into runs of at most 100 whose keys name the same
columns in the same order. It is pure, and both the review and the drivers
call it, so Review SQL shows the statements a save sends.

A run of one row is today's statement, unchanged. A marked row has no
NULL in its key (`Lock::KeyIsNull` keeps such a row from being marked), so
`IN` finds every row `=` does.

`Dialect::delete_rows(object, &[&RowDelete]) -> Result<RowDeletion>` spells
a run, matching on every dialect by name:

| Engine | One key column | Several |
|---|---|---|
| PostgreSQL | `DELETE FROM "public"."book_covers" WHERE "id" IN (9, 10, 11)` | `DELETE FROM "public"."book_authors" WHERE ("book_id", "author_id") IN ((1, 2), (1, 3))` |
| MySQL | ``DELETE FROM `bookshop`.`book_covers` WHERE `id` IN (?, ?, ?)`` | ``… WHERE (`book_id`, `author_id`) IN ((?, ?), (?, ?))`` |
| SQLite | `DELETE FROM "main"."book_covers" WHERE "id" IN (?, ?, ?)` | `… WHERE ("book_id", "author_id") IN (VALUES (?, ?), (?, ?))` |

Each value is spelled as `delete_row` spells it today (a literal on
PostgreSQL, a bound value on the others). SQLite takes a list of rows on
the right of `IN` only as `VALUES`.

### What a driver does with a run

Rows the read found gone are left out and reported `gone`, as today. What
is left of the run, when it is more than one row:

1. `SAVEPOINT`, then the run's `DELETE`.
2. **It touched every row:** release the savepoint, go on.
3. **It touched fewer:** the rows are read again. Those that are gone were
   taken by an earlier delete of this save (a key of the table that points
   at the table), which is what the save meant. One that is still there is
   `FailedDelete` for that row, in today's words.
4. **It touched more:** `FailedDelete` for the run's first row: `the save
   would have deleted 101 rows where it meant 100`.
5. **The database refused it:** roll back to the savepoint, then send the
   run's rows one statement each, as today. The row that fails is
   `FailedDelete` with its own error, so the grid marks the right row and
   the count of who holds it is of that row. If every row goes through,
   the save goes on. MySQL checks a foreign key row by row, so a run that
   holds a row and the row it points at can be refused whole and still go
   through one at a time, child first.
6. **The rollback to the savepoint fails:** the transaction is no longer
   there (a deadlock, a trigger that rolled back). Nothing more is sent.
   The run's error is `FailedDelete` for its first row.

A cancel and a lost session end the save at any step, as today.

The savepoint is on every engine, so no driver has to know which errors
leave a transaction standing. It costs two statements for each 100 rows.

### Reading the rows first (M11)

Today each row to delete is read with its own locking `SELECT`. A run is
read with one: `SELECT … WHERE "id" IN (…) FOR UPDATE`, in the same shapes
as the table above (SQLite has no `FOR UPDATE`, as today).

- Each row of the answer is matched to a key of the run by its key
  columns' values, with the comparison `same_row_twice` uses.
- A key no row matches is a row that is gone.
- An answer that does not match one to one (a row that matches no key
  exactly, or two rows for one key, which a collation that ignores case
  can give) is not trusted: the run is read again one row at a time, as
  today, and that read says what is wrong.
- The checks on the answer's columns (`reads_as_several`,
  `key_spelled_otherwise`) run once for the run.

This is the last slice. Without it everything above works, with one read
for each row.

## 5. Review SQL, and what a delete takes

- A run of several rows is one comment and one statement:

  ```sql
  -- delete 3 rows
  DELETE FROM "public"."book_covers" WHERE "id" IN (9, 10, 11)
  ```

  A run of one keeps `-- delete row id 9`. `Line::Delete` carries the
  comment it is given. The `also deletes` and `also clears` lines and
  `Line::DeleteRefused` are unchanged.
- Copies of the review hold the same statements.
- The production confirm counts rows, as today: `Save 12 deleted rows to
  production?`
- **Counts with many rows marked** (`pointing_at`):
  - a key of one column is counted with `IN`, at most 1,000 values to a
    count. More values are counted in parts and added: the rows that
    point at different rows are different rows;
  - a key of several columns is counted once for each marked row, up to
    100 marked rows. Above that its line stands without a number (M12),
    as a line does while its count is out.
- The count of who holds a refused row is of one row, as today.

## 6. After the save, and errors

The rows leave the grid, the row count updates, the cursor moves to the
next row, and a range is ended. A refused row carries its error and `Open
rows`. All as today.

One change: the note of rows that were already gone names every one of
them today. With more than three it names the first three and counts the
rest: `12 rows were already deleted: id 1, id 2, id 3 and 9 more.`

## Architecture

| Where | What changes |
|---|---|
| `src/model.rs` | `Range`, `ObjectTab.range`, `ObjectTab::selected_rows`. `Action::ExtendTo { tab, id, cell }`, `ExtendBy { tab, id, rows }`, `SelectRows { tab, id }`, `LeaveVisual { tab, id }`, `DeleteRows { tab, id, count }`. Every place that clears `selection` clears `range` |
| `src/app.rs`, `src/app/editing.rs` | The reducers of those. `delete_row` becomes the marking of a list of rows (section 2). A move keeps a held range and ends another |
| `src/edit.rs` | `Told::MarkedRows`, `Told::RestoredRows`. `pointing_at` counts in parts and stops at 100 rows for a key of several columns |
| `src/keymap.rs` | The commands above, `When::Visual`, the count in `typed` |
| `src/ui/keys.rs` | Shift+arrows, `V`, `d`, the count handed to `DeleteRows`. The refusal and the cap said outside `input_mut`, as one row's is |
| `src/ui/grid.rs` | The click reports whether Shift was down. A row takes `selected` from the range. Rows of a range are exposed as selected |
| `src/ui/data_view.rs`, `src/ui/workspace.rs` | The footer's count, the `-- VISUAL --` mode line, the announcements |
| `src/review.rs` | Lines from `delete_batches` |
| `crates/tabletist-db/src/write.rs` | `delete_batches` |
| `crates/tabletist-db/src/dialect.rs` | `delete_rows`, and `select_rows` for the batched read |
| `crates/tabletist-db/src/{sqlite,pg,mysql}/write.rs` | The run with its savepoint, and the batched read |
| `docs/_guide/editing-data.md`, `macos.md`, `omarchy.md`, `docs/_reference/keyboard-shortcuts.md` | Selecting several rows, `V`, `{count}dd`, the cap |
| `docs/superpowers/specs/2026-10-12-deleting-rows-design.md` | "As built" points here for tests 3 and 7 |

`tabletist-db` gains no dependency and knows nothing of selections.

## Tests

Headless, on the Bookshop fixtures. Numbers continue the first spec's
where they finish one of its tests.

1. (Test 3) Shift+click and Shift+arrows select a run of rows. The footer
   counts them. A plain click selects one again.
2. (Test 3) ⌫ on several selected rows marks them all. A second ⌫ restores
   them. A selection with one unmarked row marks that row and restores
   none.
3. (Test 3) `V`, `j`, `j`, `d` marks three rows, leaves visual and puts
   the cursor on the first. `esc` leaves visual and marks nothing. The
   mode line shows `-- VISUAL --` while it is on, and `i` does nothing.
4. (Test 3) `3dd` marks the cursor's row and the two after it. `5dd` two
   rows from the end marks two. `0` is not a count. `3j` moves one row.
5. (Test 3) An action on 1,001 rows marks nothing and says `Select fewer
   rows, or delete in the SQL editor.`, by selection and by `1001dd`. On
   1,000 rows it marks them.
6. A range with a new row in it: marking drops the new row, restoring
   leaves it. A range with a row whose key is NULL marks the others and
   says one can't be deleted. A read-only connection marks none and says
   why.
7. Each marked row keeps its pending cells under the mark, and a restore
   of the range gives them back.
8. The announcements of section 2, and rows of a range exposed as
   selected.
9. The inspector's Delete with a range marks the cursor's row alone.
10. (Test 7) `delete_rows` on PostgreSQL, MySQL and SQLite gives the
    strings of section 4, for one key column and for two.
    `delete_batches` cuts 250 rows into 100, 100 and 50, and starts a new
    run where the key's columns change.
11. Review SQL shows `-- delete 3 rows` over one statement, and `-- delete
    row id 9` for a row alone.
12. On each engine (`tests/delete_rows.rs`): 250 rows deleted in one save
    are gone; a run with one row a foreign key holds rolls the save back
    and names that row; a run with a row and the row it points at, in a
    table with a key to itself, goes through; a row that was already gone
    is reported and the rest of its run deleted.
13. On each engine: the batched read finds the rows of a run, reports the
    ones that are gone, and on SQLite and MySQL falls back to one read a
    row where a key matches a row in another case.
14. Cascade counts: 1,500 marked rows are counted in two parts and
    added. A key of two columns with 101 marked rows shows its line
    without a number.
15. Five rows that were already gone: the note names three and says `and
    2 more`.

PostgreSQL and MySQL scenarios run in CI only: the integration jobs
already name `--test delete_rows`.

## Slices

Each ends in something that can ship, in this order:

| # | Slice | Ends with |
|---|---|---|
| 1 | A range of rows in the desktop looks, and marking it: sections 1 (desktop) and 2, the cap | Tests 1, 2, 5 (by selection), 6 to 9 |
| 2 | Omarchy: `V`, `d`, the mode line, `{count}dd`: sections 1 (Omarchy) and 3 | Tests 3, 4, 5 (by count) |
| 3 | Batches of deletes, the review's lines, counts in parts, the shorter note: sections 4 to 6 | Tests 10, 11, 12, 14, 15 |
| 4 | The batched read | Test 13 |

## Out of scope

- Scattered rows (⌘-click), Select all, dragging over rows.
- A block of cells and the paste over it, undo and redo: slice 4 of the
  value editing spec.
- Copy, duplicate and yank of several rows (D3 of the duplication spec,
  E3 of the inserting spec). The range makes them possible. They are not
  built here.
- A count before any key but `dd`. `y`, `o` and `x` in visual.
- Inserts and updates in batches.
- A selection of rows in a SQL result.
- Everything the first spec left out.
