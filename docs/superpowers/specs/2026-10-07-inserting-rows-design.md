# Inserting rows

Date: 2026-10-07. Amended 2026-10-08. Status: built in runs. Run 1
(`docs/superpowers/plans/2026-10-07-inserting-rows-1-save.md`) is built: a
save carries new rows and the three drivers write them. Run 2
(`docs/superpowers/plans/2026-10-07-inserting-rows-2-add-row.md`) is
built: a row is added from the grid, filled, reviewed and saved. What is
left is listed under "Delivery". Each plan's "What the design asks, and
what gets built" says what of the text below the app can do, and in which
run.

This is slice 5 of `2026-10-03-value-editing-core-design.md` ("Rows: add,
duplicate, delete"), without delete. Its `inspector-editing-spec.md` is
this folder's `2026-10-06-row-inspector-inline-edit-design.md`.

The text below is the user's spec, as amended on 2026-10-08 after the audit
of pull request 108 checked the app against it. "Decisions of 2026-10-08",
at the end, lists each amendment and why it was made.

The designs are in the design canvas Artifact and are not copied into the
repository.

## Goal

Let people add rows to a table from the table view: add an empty row, duplicate a row, or paste rows. New rows are pending changes, just like edited values. They are written by the same Save (⌘S / `:w`), in the same transaction, after the same Review SQL step.

This builds on the inspector editing spec (`inspector-editing-spec.md`): it reuses the pending store, the editors, the inspector fields and the save flow from that spec.

Design references (canvas "Tabletist", section **Editing · Inserting rows**):

- `macOS – Inserting a row` (MacInsert): a new row pinned under the header, with the foreign key picker open, the inspector as a "New row" form, and the pending bar with Save disabled.
- `macOS – Inserting rows, flow and states` (MacInsertStates): ways to add a row, duplicate, paste preview, Review SQL, failed save, after save.
- `Omarchy – Inserting rows` (OmarchyInsert): the same flow with vim keys (`o`/`O`, `yy p`, paste, `:diff`, `:w`).
- `Components` sheets: the green "new" row state, plus required and default labels.

## Behavior

### 1. Entry points

| Action | macOS | Omarchy | Result |
|---|---|---|---|
| Add row | ⌘N, or the **Add row** toolbar button | `o` below / `O` above the cursor row | an empty new row |
| Duplicate | ⌘D | `yy` then `p` | a copy of the selected row (see 4) |
| Paste rows | ⇧⌘V (**Paste as new rows**) | `"+p` or `ctrl+shift+v` | the paste preview (see 5) |
| Grow | ↓ on the last row | `j` on the last row does **not** add a row; use `o` | an empty new row |

- On macOS, ⌘V keeps its meaning from the value-editing spec: it pastes a TSV block over the selected cells. Only ⇧⌘V opens the new-rows preview. The paste over cells is slice 4 of the value editing spec, not a run of this one.
- Omarchy: `yy p` / `yy P` duplicate a row below / above (vim linewise). `"+p` / `ctrl+shift+v` read the system clipboard and open the paste preview. Cell values are copied with `v` … `y` / `p`, which is slice 4 of the value editing spec too.
- All entry points are hidden or disabled when inserting isn't possible:
  - the connection is read-only;
  - the object is a view. Views are refused until views are edited;
  - the user has no INSERT privilege. This check comes later, with the same check for UPDATE that the value editing spec left for later: until then every entry point stays enabled for such a role, and the save's `42501` says it on the row;
  - the result set comes from a query rather than a table.

  The tooltip or status line says why.

### 2. Where a new row appears

- **macOS:** pinned directly under the header, above the sorted rows, regardless of sort, filter or scroll position. The grid scrolls to the top if needed.
- **Omarchy:** in place, below (`o`) or above (`O`) the cursor row.
- A new row stays where it was put until a reload, a sort change or a filter change after it is saved. It never jumps while still pending.
- Several new rows stack in creation order: on macOS the newest goes at the bottom of the pinned block.
- The row count reads `13 rows + 1 new`, and the Omarchy header shows `+1`.

### 3. The new row

| Part | macOS | Omarchy |
|---|---|---|
| Row background | `#EEF7F0`, 3px `#2C7A4B` left bar | `mix(green, 14%)`, 2px green left bar |
| Row marker | `+ new` in the id column | `+` in the gutter, `new` in the id column |
| Focus on create | the first required column, editor open | the same, in insert mode |
| Required, empty | red outline / `required` chip | `*` after the column name, red in the status line |
| Has a DEFAULT | the default value dimmed, `from DEFAULT` | dimmed value, `default` |
| `now()` / expression default | italic `now()` (grid), `now() on save` (inspector) | italic `now()`, `now() on :w` |
| Identity / serial / generated | locked; "Assigned by the database on save" | `▪`, `identity · assigned on :w` |
| Nullable, no default | `NULL` chip | `NULL` |

Omarchy writes `NULL` as every row of its grid does: the inserting board's lower case is the board's to change.

- The marker on macOS is `#1F6B35`. A selected new row stays green: only its cell takes the selection's colour. In dark mode the row is the same mix, made from the dark palette's green.
- The Omarchy status line says what the row still needs (`publisher_id required`, in red) in insert mode too, while its first value is typed.
- A row nothing is required of: the selection goes to the first column that takes a value, and no editor opens.
- Column metadata (NOT NULL, DEFAULT, identity, generated, CHECK, FK, unique) comes from the schema introspection that the Structure view already loads. Don't run a new query per row. A CHECK's list of allowed values is read on all three engines, for a text column whose comparison is exact: the app holds a typed value to the list letter for letter. On MySQL that is a collation that ends `_bin`, `_cs` or `_cs_ks` and has no `_ai_` in its name (MariaDB's `utf8mb4_uca1400_ai_cs` tells case apart and not accents); on SQLite a table whose statement names no `COLLATE`. One gap is accepted: a MySQL collation that pads (`utf8mb4_bin` is `PAD SPACE`) takes `'print '`, with a space after it, for `print`, and the app's list refuses it.
- A column is **required** when it is NOT NULL, has no DEFAULT, and isn't identity or generated.
- Defaults are shown only as a preview. The value is **not sent** unless the user changes it (see 6). If a default can't be shown as a literal, show its expression text, as the engine writes it: `now()` on PostgreSQL, `CURRENT_TIMESTAMP` on MySQL and SQLite.
- Editors are the shared editors from the inspector spec. A foreign key column opens the search picker on the first keystroke, searching the referenced table by its first text column and by id. The editors by type, the picker among them, are slice 2 of the value editing spec, not a run of this one.

### 4. Duplicate

- Copies every column of the selected row except:
  - primary key columns;
  - identity, serial and generated columns;
  - columns with a UNIQUE constraint or unique index (single-column only; multi-column unique indexes are copied and left to the database).
- Cleared columns that are required are marked `required · unique` (or `required`), and focus goes to the first of them.
- The inspector shows "Copied: format, title and 31 more. Cleared: id (identity), isbn (unique)."
- Duplicating a pending new row copies its pending values. Duplicating an existing row that has pending edits copies the pending values, not the loaded ones.
- With several rows selected, ⌘D duplicates each of them (at most 500). Omarchy uses `yy` with a count or a visual selection, then `p`.

### 5. Paste rows

- Clipboard text is parsed as TSV first, then CSV. The first line is treated as a header if its cells match column names (case-insensitive, `_` and space treated as the same). Otherwise columns are matched by position, starting at the cursor column, and the preview says "matched by position".
- The preview dialog (macOS) or prompt (Omarchy) shows:
  - "Add N pasted rows to <table>?", plus how columns were matched;
  - each column's header with ✓ when matched, or `· ignored` (amber) when it has no matching column;
  - up to 50 rows, then "and N more";
  - invalid rows in red, each with a one-line reason: type parse failure, CHECK / enum violation, NOT NULL missing.
- The client validates only what it can check without the database: type parsing, enum and CHECK IN lists, and NOT NULL without a default. Unique and FK violations surface on save.
- Actions are **Cancel**, **Add N valid**, and **Add all** (disabled while any row is invalid). Omarchy uses `[v]` add valid and `[esc]` cancel.
- At most 1,000 pasted rows. Above that, the preview says to use the SQL editor or an import (out of scope).
- Empty cells become NULL for nullable columns, and the default for columns that have one. The literal text `NULL` becomes NULL; `''` stays an empty string.

### 6. One pending model

- New rows go in the **same** pending store as edits and deletes, keyed by a temporary client id (`new:1`, `new:2`, …) until saved. Where several new rows stand, a line that names one says which, counted from the top: the Omarchy error line reads `! new 2:publisher_id …`.
- Each new row stores only the columns the user **set**. Defaults are never written into the store.
- Editing a new row's cell updates its insert values, not an UPDATE. Deleting a new row (⌫ / `dd`) drops it from the store with no SQL.
- Undo/redo (⌘Z / `u`) covers creating, editing and dropping new rows. The undo stack is slice 4 of the value editing spec: until it is built, ⌘Z / `u` unset one cell of a new row and nothing more.
- **Discard** / `:e!` drops new rows along with the other pending changes. The inspector also has **Discard new row**.
- Esc (`esc`) leaves the row pending; it does not drop it.
- Save stays disabled while any new row is missing a required value. The pending bar says "1 new row · publisher_id is required", and the button tooltip says "Fill required fields to save".

### 7. Inspector for a new row

- The header reads **New row** with "<table> · not saved" on a green background.
- Field order:
  1. required fields;
  2. fields the user set;
  3. other editable fields in table order;
  4. locked fields (identity, generated) last.
- Every field is editable in place, following the inspector spec. A field being edited in the grid shows "Editing in the grid…".
- Footer: **Discard new row** and **Add another ⌘N**, with the hint `↩ next field · ⇥ next column in the grid · esc leaves the row pending`.
- For a table without a primary key, show at the top: "This table has no primary key. The row can be inserted, but not edited or deleted afterwards."

### 8. Review SQL and save

- New rows become one `INSERT` per row, before the UPDATEs and DELETEs, in the same transaction. Review SQL shows each under a `-- new row` comment, with its names qualified and quoted as an UPDATE's are, so the statement runs as shown when it is pasted:

  ```sql
  -- new row
  INSERT INTO "public"."book_covers" ("publisher_id") VALUES ('9100000000000000004') RETURNING *;
  ```

  That is PostgreSQL's. SQLite shows the number bare, and MySQL quotes its names with backticks and has no `RETURNING`.
- The column list holds only the columns the user set. If none were set, use `INSERT INTO t DEFAULT VALUES RETURNING *`. MySQL has no `DEFAULT VALUES`: there it is ``INSERT INTO t () VALUES ()``.
- MySQL has no `RETURNING`: run the insert, read `LAST_INSERT_ID()`, then `SELECT * … WHERE pk = ?` inside the same transaction. For a table without an auto-increment key, select by the primary key values that were sent.
- SQLite uses `RETURNING *`. The app ships its own SQLite, which has it, so there is no fallback for an older one.
- Rows that set the same columns are sent together, up to 100 in one multi-row `INSERT … VALUES (…), (…) RETURNING *`. Review SQL may show them that way.
- Values are bound parameters on MySQL and SQLite. On PostgreSQL they are escaped literals in the statement's text, as the save's UPDATE sends them. On every engine what is sent is the statement Review SQL shows, with the same values.
- The production confirm counts new rows: "Save 1 new row and 1 change to production?", with the connection and its database on the line under it. On Omarchy the save is confirmed by typing the database's name, and anything else is refused. That replaces the word `write` of the value editing spec, for every save and not only one that adds rows.

### 9. Results

- **Success:**
  - Each new row takes the values from `RETURNING` (real id, defaults, triggers' changes), loses its pending style, and flashes `#E3F1E6` for 1.2 s.
  - The row stays in place. **Show in sorted position** (macOS) or `gs` (Omarchy) re-applies sort and filter, then scrolls to the row.
  - Where the row cannot be known for sure, the page is loaded again instead and a line says so: "Reloaded: the new row is where the sort puts it". That is a table with a trigger, a foreign key that acts on an update the same save makes, and a MySQL row that no key finds again. `RETURNING` shows a row before its AFTER triggers ran, and a row is shown only as the database is known to hold it.
  - If the saved row doesn't match the current filter, it stays visible and gets the note "Hidden by the current filter after reload".
- **Failure:**
  - The whole transaction rolls back. New rows stay pending and turn red, and the failing cells show messages:
    - 23505 unique: "Already used by row id 101 · Open row". Find that row with one `SELECT pk … WHERE col = $1 LIMIT 1` after the rollback.
    - 23503 foreign key: "No publisher with id …".
    - 23502 NOT NULL and 23514 CHECK: the server message, with the constraint name.
  - The message is on the cell that failed, not on every cell set in the row. Where the database names no column (SQLite's foreign key failure), the row is marked and the banner says it, and no cell is.
  - A save stops at the first statement that fails, so one new row fails at a time. The banner reads "Nothing was saved. 1 new row failed, so the whole transaction rolled back.", and where the save held several new rows it says which: "row 2 of 5".
  - `]e` / `[e` (Omarchy) and ⌥⌘↓ / ⌥⌘↑ (macOS) jump between errors.
  - MySQL and SQLite errors map to the same messages by error code (1062, 1452, 1048, 3819 / SQLITE_CONSTRAINT_*).

### 10. Accessibility

- A new row has the label `"New row, not saved"`. A cell adds `", required"` / `", default <value>"` / `", assigned by the database"`.
- The paste preview is a dialog with a labeled table. Invalid rows are announced along with their reason.

## Out of scope

- CSV/JSON file import, and pasting more than 1,000 rows.
- Inserting from SQL editor result sets.
- Inserting into views.
- Inserting into several tables at once (for example a parent and its children).
- Upserts (`ON CONFLICT`).
- Editing or deleting saved rows of tables without a primary key.

## Implementation notes

- Extend the pending store with `Insert { temp_id, values: HashMap<Column, Value> }`. Don't create a separate store.
- Put column insert metadata (required, default expression, identity, generated, unique) on the existing column model, filled from the introspection queries for each driver. The values a CHECK allows are read on MySQL and SQLite as they are on PostgreSQL, for a text column that compares exactly (section 3).
- An error of the database carries what the messages of section 9 are chosen by: the column and the constraint it names, and on MySQL the server's error number beside the SQLSTATE (1062, 1452 and 1048 share the state 23000).
- The grid draws pinned new rows in a separate band above the virtualized body, so scrolling and virtualization don't need to know about them.
- SQL generation lives in the per-driver writer that already builds UPDATE/DELETE. Insert ordering and batching happen there.
- Keyboard routing stays in one place: ⇧⌘V/`"+p` reach the paste handler only when no cell editor has focus.

## Tests (headless, no pixels)

Use only the Bookshop demo data in fixtures. Do not add pixel or design snapshot tests, and do not commit snapshot material.

1. ⌘N / `o` creates a new row in the pending store and focuses the first required column; `O` inserts above the cursor row.
2. A row with an empty required column blocks Save; filling it enables Save.
3. Untouched default columns are absent from the generated INSERT; `DEFAULT VALUES` is used when nothing was set.
4. Identity and generated columns are locked and never appear in the INSERT.
5. Duplicate clears the PK, identity, generated and single-column unique columns, keeps the rest (including pending values), and focuses the first cleared required column.
6. ⇧⌘V / `"+p` paste with a header matches columns by name, ignores unknown columns, flags an invalid enum value, and "Add N valid" adds only the valid rows.
7. Paste without a header matches columns by position from the cursor column.
8. Inserts, updates and deletes are saved in one transaction, inserts first; a unique violation (23505 / 1062) rolls back everything and leaves all changes pending with the error on the right cell.
9. After a successful save, the row's values equal the `RETURNING` (or MySQL re-select) result, including the real id and `created_at`.
10. Dropping a new row with ⌫ / `dd` generates no SQL; undo brings it back.
11. Read-only connections, views and query result sets disable every entry point.
12. A table without a primary key accepts an insert and then reports the saved row as not editable.
13. The generated SQL for PostgreSQL, MySQL and SQLite matches the expected strings for the same Bookshop insert.

The audit of pull request 108 holds what is not built yet as ignored tests, each named with its finding (`src/ui/insert_audit_tests.rs`, `crates/tabletist-db/tests/insert_audit.rs`). A run takes its findings' tests out of `ignore` as it builds them.

## Delivery

In runs, each its own pull request. Each commit passes `cargo fmt --check`, `cargo clippy -- -D warnings` and the headless tests. A pull request's description has a summary and the tests of the list above that it covers, and screenshots only if blurred (no unblurred screenshots go to GitHub).

| Run | What it builds | State |
|---|---|---|
| 1 | The save inserts rows on the three drivers | built (pull request 102) |
| 2 | Add a row in the grid, fill it, review it, save it | built (pull request 104) |
| Fixes | What run 2 built and the spec draws otherwise: the row's colours, a selected new row that stays green, a default as its tag at 60%, an expression default slanted (the same face leaned over: the boards load no italic one), Omarchy's 2px bar, its `+1` in the header and what the row needs in insert mode, and of the copy the audit lists the bar's green dot and its words without a mark. The rest of that list is left where the plan's table says: to run 2b, to run 5, or to the board (`docs/superpowers/plans/2026-10-08-inserting-rows-fixes.md`) | built |
| Groundwork | In the drivers, for runs 4 and 5: an error carries the constraint, the columns and MySQL's number, and a CHECK's list is read on MySQL and SQLite (`docs/superpowers/plans/2026-10-08-inserting-rows-driver-groundwork.md`) | built |
| 2b | The inspector as the "New row" form, the row pinned under the header on macOS, and a name for each cell of the grid to a screen reader | |
| 3 | Duplicate, and Down on the last row | |
| 4 | Paste rows with its preview, and rows sent together up to 100 | |
| 5 | Errors and after the save: the message on the failing cell by error code, "Open row", "row 2 of 5", `]e` / `[e` and ⌥⌘↓ / ⌥⌘↑, "Show in sorted position" and `gs`, the filter note, and the line that says a page was reloaded | |

Not runs of this spec, and asked of it above:

- The editors by type, the foreign key picker among them: slice 2 of the value editing spec.
- The paste over cells (⌘V, `v` … `y` / `p`) and undo and redo, of adding and dropping a row too: slice 4 of the value editing spec.
- Reading a role's INSERT privilege: later, with UPDATE's, for the three engines at once.
- Typing the database's name to confirm a production save on Omarchy: a change to the value editing spec's save, for every save.

## Decisions of 2026-10-08

Made after the audit of pull request 108, whose findings are named (INS, DG). "Accepts a departure" is where the text above was changed to what the app does. Everywhere else the spec stands and the app is to follow it.

| | What was asked | Decided | Why |
|---|---|---|---|
| A1 | PostgreSQL is sent values as literals, not bound (INS-28a) | Accepts a departure: section 8's sentence on values | The save's UPDATE has gone this way since the value editing spec. Binding the INSERT alone would make the two halves of one save differ |
| A2 | A new row of a table with a trigger comes back unknown, and the page reloads (INS-30f) | Accepts a departure: section 9's reload, with a line that says so | `RETURNING` shows a row before its AFTER triggers ran. Run 1's review chose to show a row only as the database is known to hold it |
| A3 | macOS: the new row scrolls away (INS-05a) | The spec stands: the pinned band, in run 2b | It is what the board draws |
| A4 | A failed row's error is on every cell set in it (INS-31e) | The spec stands: the cell that failed. No cell where the database names no column | Run 5 |
| A5 | `now()` is not italic (INS-08b) | The spec stands | The same face, slanted. The boards load no italic one, so no font file is added |
| A6 | A default is plain dimmed text (INS-08a) | The spec stands: its tag at 60% | |
| A7 | The row's colours on macOS, and a selected new row turning blue (INS-06a, INS-06c, DG-8, DG-10) | The spec stands. A selected new row stays green. Dark mode is the same mix | |
| A8 | Omarchy: no 2px bar, no `+1` in the header, nothing said of what the row needs while it is typed (INS-06d, INS-10b, INS-07b) | The spec stands on all three | |
| A9 | Review SQL qualifies and quotes its names, under `-- new row` | Accepts a departure: section 8's example | The value editing spec has a shown statement run as pasted, which bare names do not promise |
| A10 | The production question's words on macOS (INS-29a) | Accepts a departure: section 8's sentence | It is the form the editing board draws for changes |
| B1 | What confirms a production save on Omarchy (INS-29b, DG-4) | The database's name | A fixed word becomes a reflex. It changes the value editing spec's save |
| B2 | A role without INSERT (INS-04c, DG-5) | Later, with UPDATE's | The value editing spec left the same check for later |
| B3 | A view that takes INSERT (INS-04e) | Views are refused until views are edited | The value editing spec left updatable views out |
| B4 | "N new rows failed" (DG-6) | "1 new row failed", and which of them | A save stops at the first failure |
| B5 | A new row nothing is required of (DG-3) | The selection, and no editor | Nothing is asked of the user there |
| B6 | An expression default on MySQL and SQLite (DG-2) | As the engine writes it | |
| B7 | An INSERT that sets nothing on MySQL (DG-9) | `INSERT INTO t () VALUES ()` | MySQL has no `DEFAULT VALUES` |
| B8 | SQLite older than 3.35 (INS-27c) | No fallback | The app ships its own SQLite |
| B9 | Which new row a line is of (INS-15b) | `new 2`, counted from the top | |
| B10 | MySQL's error number (INS-32a) | The error carries it beside the SQLSTATE | Three failures share 23000 |
| B11 | CHECK lists on MySQL and SQLite (INS-25a) | Read into the structure | The paste's check and the inspector's label need them |
| B12 | The keymap (INS-36a, DG-1) | Not this spec's to decide | It names keys and stops there |
| C1 | The implementation note said ⌘V for the paste handler | ⇧⌘V | As section 1 |
| C2 | One INSERT per row, and batching | One per row, sent together up to 100 where they set the same columns | |
| C3 | One pull request | Runs, each its own | Two are merged |
| D1 | The paste over cells, undo and redo, the foreign key picker (INS-03b, INS-03c, INS-16c, INS-16d, INS-09b) | Slices 2 and 4 of the value editing spec | They are that spec's, for every cell |
| D2 | The order of the runs | As the table under "Delivery" | |
| D3 | The small fixes to what is built | One pull request before run 2b | |
| D4 | Names for a new row's cells to a screen reader (INS-35b) | With run 2b, for every cell of the grid | The grid's cells have no names today |
