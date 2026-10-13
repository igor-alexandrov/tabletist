# Prompt: delete rows from the table view

> Kept as it was given on 2026-10-12. The three files under "Read first" are
> not in this repository under those names. Their nearest are
> `2026-10-03-value-editing-core-design.md`, `2026-10-06-row-form-design.md`
> and `2026-10-06-row-inspector-inline-edit-design.md` (the pending store, the
> inspector, undo), `2026-10-07-inserting-rows-design.md` (transaction order,
> results and errors, tables without a primary key) and
> `2026-10-08-keyboard-shortcuts-design.md` (the single keymap). Where the app
> cannot yet do what this asks, the plans say so in a table at their top:
> `docs/superpowers/plans/2026-10-12-deleting-rows-1-save.md` has the whole
> list.

You are working on Tabletist, a native database client written in Rust (PostgreSQL, MySQL, SQLite), with a macOS look and an Omarchy look. Build **deleting rows** in the table view for both.

A delete is a **pending change**, like an edited value or a new row. Marking a row does nothing to the database. Save (⌘S / `:w`) writes every pending change in one transaction, after the same Review SQL step.

## Read first

- `inspector-editing-spec.md`: the shared pending store, the inspector, undo.
- `row-insert-spec.md`: transaction order, results and errors, tables without a primary key.
- `hotkeys-fix-prompt.md`: the single keymap. Delete row is ⌫ (macOS) and `dd` (Omarchy) in the grid scope.

## Designs

Canvas "Tabletist" (`https://claude.ai/artifact/4FtP3GnsXevWfpBsotjVb6`):

| Board | What to take from it |
|---|---|
| `Omarchy – Editing, full-width` | **The deleted-row state:** a red-tinted row, struck-through values, `-` in the gutter, the preview card `- delete · id 9` with `u undo` and its `DELETE` statement, and `dd delete row` in the status line |
| `Omarchy – Editing, half-width` | The pending list with `~` update, `-` delete and `+` insert, all in one transaction |
| `Omarchy – Editing values` | `:diff`, the PROD write confirm, and the results (written, failed, conflict) |
| `Omarchy – Inserting rows` | `dd` on a new row drops it without SQL |
| `macOS – Table view, editable` | The inspector's **Delete** button and the hint `Click a value or press ⌘I to edit · ⌘D · ⌫` |
| `macOS – Editing a row` | The pending bar, Review SQL drawer, Discard and Save |
| `macOS – Editing values, flow` | Save, Discard all, the production confirm, and the conflict check |
| `macOS – Inserting rows, flow and states` | Review SQL with inserts, updates and deletes; a failed save rolls everything back |
| `macOS – Components` (light and dark) | Pending, error and danger tokens; the focus ring |

Some boards are older than the hotkeys prompt: they still show `:w preview` and `:q! discard all`. Use `:diff` and `:e!`, as the hotkeys prompt says.

**Missing design:** no macOS board shows a row marked for deletion. Use this, built only from existing tokens:

- background `#FDF6F5`;
- a 3px `#C2261F` left bar;
- values in `#9A9892` with a line-through;
- the id cell's marker reads `− delete`, in the same place as `+ new`.

In dark mode, use the mapped tokens from `macOS – Components, dark`. Keep this behind one style constant so it can change once boards exist, and mention in the PR that it's provisional.

## Rules

- Use only the Bookshop demo data in fixtures and tests.
- Pixel or design snapshots must never be part of any test run, and snapshot material isn't committed.
- No unblurred screenshot goes to GitHub.
- One PR. Every commit passes `cargo fmt --check`, `cargo clippy -- -D warnings` and the headless tests.
- UI copy is short and plain. No em dashes.

## Behavior

### 1. Marking

| Action | macOS | Omarchy |
|---|---|---|
| Mark the selected row(s) | ⌫ with no cell editor open, or **Delete** in the inspector | `dd` on the cursor row; `d` in visual row selection; `{count}dd` |
| Unmark | ⌫ again on a marked row, **Restore** in the inspector, or ⌘Z | `dd` again on a marked row, or `u` |

- **Multiple rows:** marking with several rows selected marks them all, up to 1,000 per action. Above that, say: "Select fewer rows, or delete in the SQL editor."
- **No confirm when marking.** Pending is the confirmation. Production still asks once on Save, as today.
- **Existing edits:** marking a row that has pending edits keeps the edits in the store but hides them. The SQL is only the DELETE. Unmarking brings the edits back.
- **New rows:** marking a new (unsaved) row drops it from the store with no SQL, as the insert spec says.
- **Cursor:** marking leaves the cursor on the same row. Omarchy's `{count}dd` moves down the way vim does.

### 2. When deleting isn't allowed

The command is disabled, and the tooltip or status line says why, when:

| Case | Reason shown |
|---|---|
| Read-only connection | `Read-only connection` |
| View or object without DELETE | `{object} can't be deleted from` |
| No DELETE privilege | `No permission to delete from {table}` |
| Table without a primary key | `{table} has no primary key, so rows can't be deleted here` |
| Query result set | `Rows from a query can't be deleted here` |

### 3. How a marked row looks

- It stays in place, at its position in the sort, until saved.
- Values are struck through and dimmed. The row is tinted red with a left bar: Omarchy per its board, macOS per the provisional style above.
- **Gutter:** `-` on Omarchy, `− delete` in the id cell on macOS.
- **Cells:** not editable. Starting an edit says "Restore the row to edit it."
- **Row count:** `13 rows · 1 to delete`.
- **Inspector:** a line at the top, `Will be deleted on save`, with **Restore**. Fields are read-only.
- **Pending bar:** keeps its existing layout. The count joins the summary, for example `2 changes · 1 new row · 1 to delete`.
- **Omarchy status line:** `{n} pending` as today. The `:diff` card is the one on the full-width board.

### 4. SQL

- **One statement per row, by primary key:** `DELETE FROM book_covers WHERE id = $1;`
  - Composite keys use every key column.
  - Batch up to 100 rows with the same key shape into `WHERE (k1, k2) IN (…)` or `id IN (…)`.
  - Review SQL may show them that way.
- **Order inside the one transaction:** inserts, then updates, then deletes.
- **Values:** always bound. Review SQL inlines them for reading only.
- **Conflict:**
  - 0 rows affected means the row is already gone. Report "Row id 9 was already deleted," and don't roll back for that alone.
  - Every other change still applies, and the row disappears after save.
  - Don't add extra WHERE checks on other columns.
- **Cascades:** when Review SQL opens, warn for each `ON DELETE CASCADE` or `SET NULL` foreign key that points at the table. Use one `count(*)` per such key:
  - `Also deletes 3 rows in book_covers (ON DELETE CASCADE).`
  - `Also clears publisher_id in 3 book_covers rows (ON DELETE SET NULL).`

  The same lines appear in the production confirm and in Omarchy's `:diff`.
- **Production confirm:** counts deletes, for example "Save 1 insert, 1 update and 1 delete to bookshop_production?" On Omarchy, confirm by typing the database name.

### 5. Results

- **Success:** marked rows leave the grid, the count updates, the cursor moves to the next row (or the previous one at the end), and the inspector shows that row.
- **Failure:** everything rolls back, rows stay marked, and the failing row shows the message:
  - **Foreign key restrict** (23503 / MySQL 1451 / SQLite FOREIGN KEY constraint): `3 rows in book_covers still point at this row · Open rows`. Open rows opens a filtered tab. Find the count with one `SELECT count(*)` after the rollback.
  - **Other errors:** the server message, with the constraint name.
  - The banner reads "Nothing was saved. N rows failed, so the whole transaction rolled back."
  - Error navigation (⌥⌘↓ / ⌥⌘↑, `]e` / `[e`) includes delete errors.

### 6. Accessibility

- A marked row's label adds ", marked for deletion".
- **Announcements:**
  - "Row id 9 marked for deletion. 1 to delete."
  - "Row id 9 restored."
  - after save, "1 row deleted."
- The **Delete** and **Restore** buttons have their plain names. The disabled state has the reason as its description.

## Implementation notes

- Add `Delete { key }` to the existing pending store. No separate store. Undo and redo cover mark and unmark.
- The per-driver writer that builds INSERT and UPDATE also builds DELETE, including the ordering and batching.
- Key routing: ⌫ deletes the row only when the grid has focus and no cell editor is open; inside an editor it is backspace. On Omarchy, `dd` is only in normal mode.
- Cascade counts are computed lazily, when Review SQL or `:diff` opens, and cached until the pending set changes.

## Tests (headless, no pixels)

1. ⌫ / `dd` marks the row in the pending store; doing it again unmarks it; ⌘Z / `u` also unmark it.
2. ⌫ inside an open cell editor edits text and never marks the row.
3. Several selected rows, `{count}dd` and visual `d` mark the right rows. More than 1,000 is refused with the message.
4. Marking a row with pending edits produces only the DELETE; unmarking restores the edits.
5. Marking a new row drops it with no SQL.
6. Read-only connections, views without DELETE, no privilege, no primary key and query results disable the command, each with its reason.
7. Generated SQL for PostgreSQL, MySQL and SQLite matches the expected strings, including composite keys and batching.
8. One transaction runs inserts, then updates, then deletes. A restrict violation rolls back everything and shows the count message on the right row.
9. A row already deleted elsewhere (0 rows affected) is reported, while the rest of the save is kept.
10. Cascade and SET NULL warnings show the right counts in Review SQL, `:diff` and the production confirm.
11. After save, the rows are gone, the count is right, and the cursor and inspector move to the next row.
12. Announcements carry the exact strings.

## Out of scope

- Deleting by filter ("delete all matching").
- Soft-delete conventions.
- `TRUNCATE`.
- Deleting from the SQL editor's results.
- New macOS boards (use the provisional style).
- Undo after save.

## Open questions

Proceed with the defaults and list them in the PR.

1. Toggle with ⌫ / `dd`, or only undo and Restore to unmark? Default: toggle.
2. Should cascade counts run on every Review SQL open, or only on production? Default: always.
3. The 1,000-row cap per action. Default: 1,000.
4. The macOS deleted-row style above, until boards exist. Default: as written.

## PR description

- A summary.
- The board list.
- The provisional macOS style.
- The keys used.
- The test list.
- The defaults you kept.
- Screenshots only if blurred.

## As built

Runs 1 and 2 (`docs/superpowers/plans/2026-10-12-deleting-rows-1-save.md`,
`2026-10-12-deleting-rows-2-marking.md`), on 2026-10-12. What was built
otherwise than this asks, each agreed with the user before it was built:

- **Undo.** ⌘Z and `u` on a marked row restore it. There is no redo: the
  app has no undo stack, only the key that puts one cell back.
- **Several rows.** Runs 1 to 3 marked one row at a time. Several
  selected rows, `d` on a visual selection, `{count}dd` and the cap of
  1,000 are built to a spec of their own,
  `2026-10-13-deleting-many-rows-design.md` (its slices 1 and 2), which
  finishes test 3. The `IN (…)` batches are its slice 3, and with them
  the batching of test 7: rows next to each other go in one `DELETE`, up
  to 100 rows each.
- **Privileges.** The app reads no DELETE privilege, so the command is
  not disabled for want of one: the server's refusal is the row's error
  after the save.
- **A query's result.** Nothing new: a result's row panel has no Delete,
  and ⌫ and `dd` are not keys of a result's grid.
- **The statement.** One `DELETE` per row, the names qualified and
  quoted. PostgreSQL is sent the key as literals, as it is for an
  `UPDATE`; MySQL and SQLite bind it.
- **Already gone.** Each row to delete is found and locked before
  anything is written. One that is not there is reported, and the rest
  is saved: "Row id 9 was already deleted." A row that another of the
  save's deletes took with it (a tree whose key cascades) is deleted, not
  failed.
- **Cascades.** Built in run 3
  (`docs/superpowers/plans/2026-10-13-deleting-rows-3-what-a-delete-takes.md`).
  A table's structure lists the foreign keys that point at it. When a
  review is made while rows are marked (Review SQL, `:diff`, the
  production confirm), the app counts the rows each such key reaches and
  the review says so after its statements, as comment lines in the
  warning colour: `-- also deletes 3 rows in book_covers (ON DELETE
  CASCADE)`, `-- also clears publisher_id in 3 book_covers rows (ON
  DELETE SET NULL)`. Lower case and without a full stop, as the review's
  other comments are. What differs from this spec:
  - One count per key where the key is one column; a key of several
    columns is counted once for each marked row, and the counts added.
  - Only the rows that point at the marked rows directly are counted.
    `SET DEFAULT` has no line.
  - Until a count is in, and where one fails, its line stands without a
    number. A count of 0 has no line.
  - The counts are kept until the marks change, and are stopped by
    Cancel, by closing the tab and by a mark that changes: a count runs
    on the tab's one session, ahead of its next fetch or save.
  - On SQLite every table's own keys are read each time a structure is
    loaded, since it keeps no list of the keys that point at a table.
- **The restrict count.** After a delete a foreign key refuses
  (PostgreSQL 23503, and 23001 from version 18 on; MySQL 1451; SQLite 787,
  and 1811 for `RESTRICT`) the app counts the rows that point at the row:
  of the key the database names, or of every key that holds where it
  names none. "3 rows in book_covers still point at this row" stands
  after the database's words in the pending bar, in the row panel, on
  the row's cells and on the Omarchy error line. **Open rows** is a
  button of the bar and of the row panel; in the Omarchy look `gd` on
  the row opens them.
- **The order of deletes.** A row is deleted before the row it points
  at, where the table's own foreign key points at the table.
- **Error navigation.** Not built: the commands are not.
- **The marker.** In the desktop looks the first column of the row's key
  shows a minus before its value, which is struck through with the
  row's others (`− 9`), not `− delete`: the macOS look's key column is 64
  points wide and does not hold the word. The style stands in
  `env::deleted_row_colors`.
- **Every value of a marked row is plain text**, struck through: a tag,
  a chip and the NULL chip are drawn as their text.
- **Hidden edits.** Marking takes the row's pending cells out of the
  set's cells and keeps them with the mark; restoring puts them back.
- **The words.** The pending bar: "1 new row · 1 change in 1 row · 1 to
  delete". The Omarchy status line: "-1 to delete" beside its other
  counts. The production confirm: "Save 1 new row, 2 changes and 1
  deleted row to production?". Review SQL: `-- delete row id 9` over the
  statement. A refused delete: "Nothing was saved. 1 row failed, so the
  whole transaction rolled back." and the database's words.
- **The inspector.** "Will be deleted on save" under the panel's header
  with Restore at its right; the Omarchy look's line ends with `dd
  restore`. The footer's Delete is disabled on a marked row in the
  desktop looks.
- **Announcements.** Polite live regions, with the spec's words.
