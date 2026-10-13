# Deleting many rows at once

Status: built on 2026-10-13, in three slices, each to a plan of its own
in `docs/superpowers/plans/`: `2026-10-13-deleting-many-rows-1-desktop.md`,
`…-2-omarchy.md` and `…-3-runs.md`. Each plan lists where it left this
spec.

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
- The Omarchy grid has no visual mode and reads no count. `v y` is a
  two-key spelling of the copy of one cell, and `v p` is claimed and not
  built. Digits typed on the grid do nothing.
- `Action::DeleteRow { tab, id }` marks the cursor's row, restores it when
  it is marked, and drops it when it is new (`App::delete_row`). The grid's
  keys, the inspector's Delete and `dd` on a new row's form all send it.
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
spec is written to.

Status: every default was approved on 2026-10-13.

| # | Question | Default | The other way |
|---|---|---|---|
| M1 | What several selected rows are | A run of neighbouring rows, from an anchor row to the cursor's row | Scattered rows (⌘-click) too: a set of rows instead of a run. Not in this run |
| M2 | How the desktop looks select them | Shift+click and Shift+↑ / Shift+↓ | Also Select all (⌘A) and dragging over rows. Not in this run |
| M3 | Omarchy's visual row selection | `V` starts it, as vim's linewise visual. The mode line reads `-- VISUAL --` with the row count. `v y` stays as built | `v` starts it, and the copy of one cell moves to another key |
| M4 | Keys while it is on | Moves extend, `d` marks, `esc` or `V` leaves. The grid's other commands do nothing | Another command leaves visual and acts on the cursor's row |
| M5 | A press on rows of both kinds | It marks when any row named is unmarked. It restores only when all are marked | Always mark, and restore one row at a time |
| M6 | Rows named that cannot be deleted (a key that is NULL, a row that is gone) | Skipped, and the message counts them | Nothing is marked, and the first such row's reason is said |
| M7 | Which keys take a count | `dd` only. A count before any other key is dropped | `j` and `k` too |
| M8 | The inspector with several rows selected | It shows the cursor's row. Its Delete and Restore act on that row alone | Its Delete acts on the selection |
| M9 | Other commands with several rows selected (copy row, duplicate, set NULL, yank) | They act on the cursor's row, as today | Copy and duplicate act on every row. That is D3 of the duplication spec, a run of its own |
| M10 | The cursor after `{count}dd` | It moves to the row after the last row named, which is how I read the first spec's "moves down the way vim does". Plain `dd` leaves it, as built | It stays, so the same `{count}dd` again restores the same rows |
| M11 | A batch the database refuses | A savepoint before each batch. On a refusal the batch is undone and its rows are sent one at a time, so the error lands on its row | Batches only on PostgreSQL, whose error names the row |
| M12 | Reading the rows before they are deleted | One locking `SELECT` per row, as today. 1,000 marked rows are 1,000 short reads before ten deletes | Read in batches of 100 too. It is not in the first spec and is the riskiest part: see Out of scope |
| M13 | Cascade counts that are asked once per marked row, with many rows marked | Above 100 marked rows the line stands without a number | One count per marked row, as today, however many |
| M14 | Where this lands | On this branch, in PR #120: it is the rest of the same spec | A PR of its own, after #120 merges |

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

**One rule for every place that sets the cursor.** The cursor is set
through one function of `ObjectTab`, which is told what becomes of the
range:

| What set the cursor | The range |
|---|---|
| An extend (Shift+click, Shift+arrow) | Runs from its anchor, or from the row the cursor was on, to the new row. Not held |
| A move by key, to another row or along the row (an arrow, `j` `k` `h` `l`, Tab, a page up or down, the first or last row, `:N`, the row before or after from the inspector) | Kept when held, and so follows the cursor. Ended otherwise |
| Anything else (a click, a followed foreign key, a new or duplicated row, an editor that opens, a save, a jump from the pending bar) | Ended |

A move along the row (`h`, `l`, ←, →, Tab) is a move like any other: it
ends a range that is not held, and under a held one it changes the
cursor's cell and not the rows.

The field stays `pub`, as tests write it. The rule holds for the app's own
code: nothing outside that function writes `selection`.

A range does not outlive its page: a reload, a sort, a filter, another
page and a save that removes rows end it, as they forget the cursor.

Slice 4 of the value editing spec (a block of cells, pasted over) will add
an anchor cell on the same footing. Nothing here stands in its way.

### Desktop looks

| Gesture | What it does |
|---|---|
| Shift+click a cell | An extend to that cell's row. The cursor moves to the clicked cell. With no cursor yet, it is a plain click |
| Shift+↓, Shift+↑ | An extend by one row |

- The footer counts: `3 rows selected`, and `3 rows selected · read-only`.
- Each row of the range is painted as the selected row of its kind is
  today (a new row stays green, a marked row stays struck). Only the
  cursor's cell is the active cell.

### Omarchy look

| Key | What it does |
|---|---|
| `V` | Starts a visual row selection on the cursor's row: a held range whose anchor is that row. `V` again leaves it |
| Any move by key | Extends it |
| `d` | Marks its rows (section 2), leaves visual, and puts the cursor on the first row of the range |
| `esc` | Leaves it. The cursor stays |

- The mode line reads `-- VISUAL --`, where `-- INSERT --` shows, followed
  by the count and the keys: `-- VISUAL -- 3 rows` and `d delete · esc
  normal`. The words after the mode are by analogy with the insert line:
  the board draws the mode alone.
- The rows of the range take the selection background of the components
  board. The cursor's row and cell keep their own look.
- While it is on, the commands that change, copy or follow a row or a
  cell do nothing, by whatever key: Edit cell and Edit cell from nothing
  (`i`, Enter), Set NULL, Set DEFAULT, Undo and Redo one cell, Copy cell
  values, Copy row, Paste, Add row in its three places, Duplicate row,
  Yank the row, Open referenced row (`gd`), the saved row (`gs`), and a
  count. Every other command works as ever: the moves, the inspector,
  save, discard, Review SQL and its keys, a reload, the filters, the
  pages, tabs, the tree and the `:` prompt. This is a list of commands in
  `src/ui/keys.rs`, gated where they are read: the keymap has no way to
  say "nothing else", and a scope does not tell them apart.
- `esc` leaves visual before it does anything else it does in the grid:
  with the review open, a note showing or the inspector open, the first
  `esc` leaves visual and the next does what it does today. `esc` is a
  chord, and `src/ui/keys.rs` asks each command in turn for it: Leave
  visual is asked before Close review, Dismiss note and Close inspector.
- A click ends it.
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

Three actions, by the rows they name:

| Action | Sent by | Rows named |
|---|---|---|
| `DeleteRow { tab, id }`, as today | The inspector's Delete, `dd` on a new row's form | The cursor's row alone (M8). A range is left as it is |
| `DeleteSelected { tab, id }` | ⌫, the grid's `dd`, `d` in visual, the grid's Delete command | The selected rows |
| `DeleteCount { tab, id, count }` | `{count}dd` | The cursor's row and the `count - 1` rows after it in the grid's order, stopping at the last row, as vim does. A range, if one stands, is ended |

`RestoreRow` is unchanged: the cursor's row alone.

What an action on several rows does, in this order. An action that names
one row does what it does today, and says what it says today.

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

**The cursor and the range afterwards.**

| Action | Cursor | Range |
|---|---|---|
| `DeleteSelected`, desktop | Stays | Stays, so a second ⌫ restores the same rows |
| `d` in visual | The first row of the range | Ended |
| `{count}dd` | The row after the last row named. On the last row of the grid it stays there (M10) | Ended |

Where new rows were dropped, the cursor and the anchor each go to the
nearest row of the range that is left, if theirs was dropped. Where none
is left, the range ends and the cursor goes where dropping one new row
puts it today.

**What is said.**

| Case | Words |
|---|---|
| Marked | `12 rows marked for deletion. 15 to delete.` |
| Marked, some left out | `11 rows marked for deletion. 1 can't be deleted. 14 to delete.` |
| Restored | `12 rows restored.` |
| New rows dropped with them | The sentence above, then `2 new rows dropped.` |
| Only new rows | `2 new rows dropped.` |
| The cap | `Select fewer rows, or delete in the SQL editor.` |

The cap and the sentence with rows left out show where a refusal shows: a
toast in the desktop looks, the status line's message in the Omarchy
look. The others are for assistive technology only, as the announcement
of one marked row is.

Rows of a range are exposed as selected to assistive technology. Entering
visual says `Visual. 1 row selected.` and each extend says the count.

```rust
pub enum Told {
    Marked { row: String, deleted: usize },
    Restored { row: String },
    /// Several rows in one action. `deleted` is how many stand marked.
    MarkedRows { marked: usize, skipped: usize, dropped: usize, deleted: usize },
    RestoredRows { restored: usize },
}
```

`Edits::mark` and `Edits::restore` stay as they are and are called once
for each row.

## 3. Counts

- In the Omarchy look, on a table's grid, digits typed before a key are
  its count. `0` as the first digit is not a count. More than six digits
  are read as 999,999. No count is read on a SQL result, in the inspector
  or on a new row's form.
- Only `dd` takes one (M7). Before any other key the count is dropped and
  the key acts once, which is what digits do today. `:` after digits
  drops the count and opens the prompt.
- The count is read where the letters of a key of several are:
  `Keymap::typed` hands back `Typed::Counted(Command, u32)` for a command
  that takes a count and was given one. No digit is spelled outside
  `src/keymap.rs`.
- A count waits as the first `d` of `dd` waits, and ends with it: on a
  key that types nothing, a click, a field that takes the keyboard.
- The keyboard reference writes it `{count}dd`. Settings > Keyboard keeps
  `dd`.

## 4. Deletes in batches

The app sends what it sends today: `ChangeSet.deletes`, one `RowDelete`
for each marked row, in `delete_order`. Each row is read first with its
own locking `SELECT`, as today (M12). The batches are made in
`tabletist-db`.

### The runs

Each row's own statement is built first, with `Dialect::delete_row`, as
today. A row whose statement cannot be built is refused by itself, as
today (`DeleteRefused` in the review, `FailedDelete` in a save), and
joins no run.

`write::delete_runs(&[RowDelete], built: &[bool]) -> Vec<Range<usize>>`
cuts the deletes, in their order, into runs of at most 100 neighbours that
were built and whose keys name the same columns in the same order. It is
pure, and both the review and the drivers call it.

A run of one row is today's statement, unchanged. A marked row has no
NULL in its key (`Lock::KeyIsNull` keeps such a row from being marked), so
`IN` finds every row `=` does.

`Dialect::delete_run(object, &[&RowDelete]) -> Result<RowDeletion>` spells
a run of two or more, matching on every dialect by name:

| Engine | One key column | Several |
|---|---|---|
| PostgreSQL | `DELETE FROM "public"."book_covers" WHERE "id" IN (9, 10, 11)` | `DELETE FROM "public"."book_authors" WHERE ("book_id", "author_id") IN ((1, 2), (1, 3))` |
| MySQL | ``DELETE FROM `bookshop`.`book_covers` WHERE `id` IN (?, ?, ?)`` | ``… WHERE (`book_id`, `author_id`) IN ((?, ?), (?, ?))`` |
| SQLite | `DELETE FROM "main"."book_covers" WHERE "id" IN (?, ?, ?)` | `… WHERE ("book_id", "author_id") IN (VALUES (?, ?), (?, ?))` |

Each value is spelled as `delete_row` spells it today (a literal on
PostgreSQL, a bound value on the others). SQLite takes a list of rows on
the right of `IN` only as `VALUES`.

`DeleteParts` of a run: `columns` holds each key column once, and `values`
each value's place, row by row, so its length is the rows times the
columns.

### What a driver does with a run

Rows the read found gone are left out and reported `gone`, as today, and
the statement is spelled for the rows that are left. Where one row is
left, it is today's statement. Where more are:

1. `SAVEPOINT`, then the run's `DELETE`.
2. **It touched every row:** release the savepoint, go on.
3. **It touched fewer:** the run's rows are read again. Those that are
   gone were taken by an earlier delete of this save (a key of the table
   that points at the table), which is what the save meant. One that is
   still there (a trigger kept it) is `FailedDelete` for that row, in
   today's words.
4. **It touched more:** `FailedDelete` for the run's first row: `the save
   would have deleted 101 rows where it meant 100`.
5. **The database was busy** (a lock wait or a deadlock: MySQL 1205 and
   1213, PostgreSQL 40P01, 55P03 and 40001, SQLite busy or locked):
   `FailedDelete` for the run's first row with that error. Nothing is
   sent again: each row could wait as long once more.
6. **The database refused it otherwise:** roll back to the savepoint,
   then send the run's rows one statement each, as today. The row that
   fails is `FailedDelete` with its own error, so the grid marks the
   right row and the count of who holds it is of that row. If every row
   goes through, the save goes on. MySQL checks a foreign key row by row,
   so a run that holds a row and the row it points at can be refused
   whole and still go through one at a time, child first.
7. **The rollback to the savepoint fails:** a cancel or a lost session
   ends the save as it does anywhere. Anything else means the transaction
   is no longer there (on MySQL a deadlock, on SQLite a trigger that
   rolled back). Nothing more is sent, and the run's error is
   `FailedDelete` for its first row.

A cancel and a lost session end the save at any step, as today.

The savepoint is on every engine, so no driver has to know which errors
leave a transaction standing. It costs two statements for each run.

Two things a run gives up, knowingly:

- Today each statement is checked to have touched one row. A run is
  checked by its sum. Each row was read and found once just before, in
  the same transaction and locked where the engine has locks, so the sum
  is off only where a trigger or a cascade stepped in, which steps 3 and
  4 look at.
- A trigger that fires once per statement fires once for a run, not once
  for each row.

## 5. Review SQL, and what a delete takes

- A run of several rows is one comment and one statement:

  ```sql
  -- delete 3 rows
  DELETE FROM "public"."book_covers" WHERE "id" IN (9, 10, 11)
  ```

  A run of one keeps `-- delete row id 9`. `Line::Delete` carries the
  comment it is given. The statement is laid out as one row's is: the
  table, then the `WHERE`, with each key column and each value inked as
  they are today and each value shortened as it is today. `lay_delete`
  takes a `values` that is a whole number of rows.
- The review shows the runs a save starts with. A save sends other
  statements where it finds a row gone or sends a refused run one row at
  a time. The review does not show the reads either, as today.
- Copies of the review hold the same statements.
- The production confirm counts rows, as today: `Save 12 deleted rows to
  production?`
- **Counts with many rows marked** (`pointing_at`):
  - a key counted with one `IN` (one column, values `IN` can list) takes
    at most 1,000 values to a count. More are counted in parts and added:
    the rows that point at different rows are different rows;
  - a key counted once for each marked row (several columns, or values
    `IN` cannot list) is counted up to 100 marked rows. Above that its
    line stands without a number (M13), as a line does while its count is
    out.
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
| `src/model.rs` | `Range`, `ObjectTab.range`, `ObjectTab::selected_rows`, the one function that sets the cursor. `Action::ExtendTo { tab, id, cell }`, `ExtendBy { tab, id, rows }`, `SelectRows { tab, id }`, `LeaveVisual { tab, id }`, `DeleteSelected { tab, id }`, `DeleteCount { tab, id, count }` |
| `src/app.rs`, `src/app/editing.rs` | The reducers of those. The marking of a list of rows (section 2), which `delete_row` calls with one row |
| `src/edit.rs` | `Told::MarkedRows`, `Told::RestoredRows`. `pointing_at` counts in parts and stops at 100 rows where it counts per row |
| `src/keymap.rs` | The commands above, `When::Visual`, `Typed::Counted` |
| `src/ui/keys.rs` | Shift+arrows, `V`, `d`, the count, the gate on the grid's commands in visual. The refusal and the cap said outside `input_mut`, as one row's is |
| `src/ui/grid.rs` | The click reports whether Shift was down. A row takes `selected` from the range. Rows of a range are exposed as selected |
| `src/ui/data_view.rs`, `src/ui/workspace.rs` | The footer's count, the `-- VISUAL --` mode line, the announcements |
| `src/ui/row_panel.rs` | Nothing: its Delete sends `DeleteRow`, the cursor's row |
| `src/ui/pending_bar.rs` | The shorter note of rows already gone |
| `src/review.rs` | Lines from `delete_runs`, `lay_delete` for a run |
| `src/testing.rs` | A click with Shift held: pointer events at a cell's place, with Shift set on the frame's input as well as on the events, since a click by AccessKit carries none and `Harness::frame` leaves the frame's modifiers at their default |
| `crates/tabletist-db/src/write.rs` | `delete_runs` |
| `crates/tabletist-db/src/dialect.rs` | `delete_run`, `DeleteParts` for a run |
| `crates/tabletist-db/src/{sqlite,pg,mysql}/write.rs` | The run with its savepoint |
| `crates/tabletist-db/tests/delete_rows.rs` | A `genres` table on each engine with a key to itself that does not cascade, the parent's id the lower |
| `docs/_guide/editing-data.md`, `macos.md`, `omarchy.md`, `docs/_reference/keyboard-shortcuts.md` | Selecting several rows, `V`, `{count}dd`, the cap |
| `docs/superpowers/specs/2026-10-12-deleting-rows-design.md` | "As built" points here for tests 3 and 7 |

`tabletist-db` gains no dependency and knows nothing of selections.

## Tests

Headless, on the Bookshop fixtures. "(Test 3)" and "(Test 7)" finish
those tests of the first spec.

1. (Test 3) Shift+click and Shift+arrows select a run of rows. The footer
   counts them. A plain click selects one again, and so do ↓ and →.
   Shift+click with no cursor selects one row.
2. (Test 3) ⌫ on several selected rows marks them all and keeps the
   range. A second ⌫ restores them. A selection with one unmarked row
   marks that row and restores none.
3. (Test 3) `V`, `j`, `j`, `d` marks three rows, leaves visual and puts
   the cursor on the first. `esc` leaves visual and marks nothing. The
   mode line shows `-- VISUAL --` and the count while it is on. `i`, `o`,
   `x`, `gd` and Enter do nothing in it, `l` moves the cell and keeps the
   rows, and `:w` saves.
4. With the review open and visual on, the first `esc` leaves visual and
   the second closes the review. The same with a note and with the
   inspector.
5. (Test 3) `3dd` marks the cursor's row and the two after it and puts
   the cursor on the fourth. `5dd` two rows from the end marks two and
   stays on the last. `0` is not a count. `3j` moves one row. `3:` opens
   the prompt. A count on a SQL result does nothing.
6. (Test 3) An action on 1,001 rows marks nothing and says `Select fewer
   rows, or delete in the SQL editor.`, by selection and by `1001dd`. On
   1,000 rows it marks them.
7. A range with new rows in it: marking drops them, restoring leaves
   them, and a range of new rows alone drops them and says so. With the
   anchor on a dropped row, the range that is left is still selected.
8. A range with a row whose key is NULL marks the others and says one
   can't be deleted. A read-only connection marks none and says why.
9. Each marked row keeps its pending cells under the mark, and a restore
   of the range gives them back.
10. A range ends on a reload, a sort, a filter, another page, a save that
    removes rows, a new row and an editor that opens.
11. The announcements of section 2, and rows of a range exposed as
    selected.
12. The inspector's Delete with a range marks the cursor's row alone and
    leaves the range.
13. (Test 7) `delete_run` on PostgreSQL, MySQL and SQLite gives the
    strings of section 4, for one key column and for two. `delete_runs`
    cuts 250 rows into 100, 100 and 50, starts a new run where the key's
    columns change, and leaves a row that was not built out of any run.
14. Review SQL shows `-- delete 3 rows` over one statement with its
    values inked, and `-- delete row id 9` for a row alone. A row whose
    statement cannot be built shows as refused between two runs.
15. On each engine (`tests/delete_rows.rs`): 250 rows deleted in one save
    are gone. A run with one row a foreign key holds rolls the save back
    and names that row. A parent and its child in `genres`, in one run,
    go through. A row that was already gone is reported and the rest of
    its run deleted. A run after which an earlier cascade left fewer rows
    goes through (`shelves`).
16. On SQLite: a trigger that keeps one row of a run (`RAISE(IGNORE)`)
    fails the save on that row. A trigger that rolls the transaction back
    (`RAISE(ROLLBACK)`) fails the save on the run's first row and sends
    nothing more.
17. Cascade counts: 1,500 marked rows are counted in two parts and
    added. A key of two columns with 101 marked rows shows its line
    without a number.
18. Five rows that were already gone: the note names three and says `and
    2 more`.

Step 4 of a run (more rows touched than meant) and step 5 (a busy
database) have no test of their own on a server: the first cannot be made
once each row was read and found once, the second needs two sessions that
wait on each other. Each driver's sorting of an error into busy or not is
tested as a function. SQLite's reads the primary code under the extended
one, and can hardly meet a busy file on a run at all: the save's `BEGIN
IMMEDIATE` already holds it.

One check that is not a behaviour: the MySQL scenario for a key of two
columns reads the `EXPLAIN` of its run's `DELETE` and fails where it is a
scan of the whole table. A row list on the right of `IN` should find its
rows by the key. A server that does not would lock every row for a run
where it locks 100 today.

PostgreSQL and MySQL scenarios run in CI only: the integration jobs
already name `--test delete_rows`.

## Slices

Each ends in something that can ship, in this order:

| # | Slice | Ends with |
|---|---|---|
| 1 | A range of rows in the desktop looks, and marking it: sections 1 (desktop) and 2, the cap | Tests 1, 2, 6 (by selection), 7 to 12 |
| 2 | Omarchy: `V`, `d`, the mode line, `{count}dd`: sections 1 (Omarchy) and 3 | Tests 3, 4, 5, 6 (by count) |
| 3 | Runs of deletes, the review's lines, counts in parts, the shorter note: sections 4 to 6 | Tests 13 to 18 |

## Out of scope

- Scattered rows (⌘-click), Select all, dragging over rows.
- A block of cells and the paste over it, undo and redo: slice 4 of the
  value editing spec.
- Copy, duplicate and yank of several rows (D3 of the duplication spec,
  E3 of the inserting spec). The range makes them possible. They are not
  built here.
- A count before any key but `dd`. `y`, `o` and `x` in visual.
- **Reading the rows to delete in batches** (M12). What it would have to
  keep, for whoever builds it: `select_row` stops at two rows because a
  SQLite key can match a whole table, so a batched read needs a limit of
  its rows and one; MySQL's `inexact_key` check; and two keys of one run
  that the database takes for one row (`1` and `'1'`), which
  `same_row_twice` catches today and a read that matches rows to keys by
  value would call one row found and one gone.
- Inserts and updates in batches.
- A selection of rows in a SQL result.
- Everything the first spec left out.
