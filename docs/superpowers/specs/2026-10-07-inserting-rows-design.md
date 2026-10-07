# Inserting rows

Date: 2026-10-07. Status: planned in runs. Run 1 (a save carries new rows,
and the three drivers write them) is
`docs/superpowers/plans/2026-10-07-inserting-rows-1-save.md`, whose "The
runs" lists the others (add a row in the grid, duplicate, paste, errors and
after the save) and whose "What the design asks, and what gets built" says
what of the text below the app can do today, and what waits for which run.

This is slice 5 of `2026-10-03-value-editing-core-design.md` ("Rows: add,
duplicate, delete"), without delete. The text below is the user's spec as
given. Its `inspector-editing-spec.md` is this folder's
`2026-10-06-row-inspector-inline-edit-design.md`.

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
| Paste rows | ⌘V with tabular text on the clipboard and no cell editor open | `"+p` (see note) | the paste preview (see 5) |
| Grow | ↓ on the last row | `j` on the last row does **not** add a row; use `o` | an empty new row |

- On macOS, ⌘V inside an open cell editor pastes text into that cell as usual. Only a grid-level paste of multi-row or multi-column text opens the preview.
- Omarchy note: the board shows `p` as "paste rows". To avoid clashing with `yy p`, plain `p` puts the last yanked row (duplicate), and `"+p` / `ctrl+shift+v` read the system clipboard and open the paste preview.
- All entry points are hidden or disabled when inserting isn't possible:
  - the connection is read-only;
  - the object is a view without INSERT rules;
  - the user has no INSERT privilege;
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
| Nullable, no default | `NULL` chip | `null` |

- Column metadata (NOT NULL, DEFAULT, identity, generated, CHECK, FK, unique) comes from the schema introspection that the Structure view already loads. Don't run a new query per row.
- A column is **required** when it is NOT NULL, has no DEFAULT, and isn't identity or generated.
- Defaults are shown only as a preview. The value is **not sent** unless the user changes it (see 6). If a default can't be shown as a literal, show its expression text.
- Editors are the shared editors from the inspector spec. A foreign key column opens the search picker on the first keystroke, searching the referenced table by its first text column and by id.

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

- New rows go in the **same** pending store as edits and deletes, keyed by a temporary client id (`new:1`, `new:2`, …) until saved.
- Each new row stores only the columns the user **set**. Defaults are never written into the store.
- Editing a new row's cell updates its insert values, not an UPDATE. Deleting a new row (⌫ / `dd`) drops it from the store with no SQL.
- Undo/redo (⌘Z / `u`) covers creating, editing and dropping new rows.
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

- New rows become one `INSERT` per row, before the UPDATEs and DELETEs, in the same transaction:

  ```sql
  INSERT INTO book_covers (publisher_id) VALUES (9100000000000000004) RETURNING *;
  ```

- The column list holds only the columns the user set. If none were set, use `INSERT INTO t DEFAULT VALUES RETURNING *`.
- MySQL has no `RETURNING`: run the insert, read `LAST_INSERT_ID()`, then `SELECT * … WHERE pk = ?` inside the same transaction. For a table without an auto-increment key, select by the primary key values that were sent.
- SQLite uses `RETURNING *` (3.35+). For older versions, use `last_insert_rowid()`.
- Batch up to 100 rows with the same column set into one multi-row `INSERT … VALUES (…), (…) RETURNING *`. Review SQL may show them that way.
- Values are always bound parameters. The Review SQL view shows them inlined for reading only.
- The production confirm counts inserts: "Save 1 insert and 1 update to bookshop_production?"

### 9. Results

- **Success:**
  - Each new row takes the values from `RETURNING` (real id, defaults, triggers' changes), loses its pending style, and flashes `#E3F1E6` for 1.2 s.
  - The row stays in place. **Show in sorted position** (macOS) or `gs` (Omarchy) re-applies sort and filter, then scrolls to the row.
  - If the saved row doesn't match the current filter, it stays visible and gets the note "Hidden by the current filter after reload".
- **Failure:**
  - The whole transaction rolls back. New rows stay pending and turn red, and the failing cells show messages:
    - 23505 unique: "Already used by row id 101 · Open row". Find that row with one `SELECT pk … WHERE col = $1 LIMIT 1` after the rollback.
    - 23503 foreign key: "No publisher with id …".
    - 23502 NOT NULL and 23514 CHECK: the server message, with the constraint name.
  - The banner reads "Nothing was saved. N new rows failed, so the whole transaction rolled back."
  - `]e` / `[e` (Omarchy) and ⌘' (macOS) jump between errors.
  - MySQL and SQLite errors map to the same messages by error code (1062, 1452, 1048, 3819 / SQLITE_CONSTRAINT_*).

### 10. Accessibility

- A new row has the label `"New row, not saved"`. A cell adds `", required"` / `", default <value>"` / `", assigned by the database"`.
- The paste preview is a dialog with a labeled table. Invalid rows are announced along with their reason.

## Out of scope

- CSV/JSON file import, and pasting more than 1,000 rows.
- Inserting from SQL editor result sets.
- Inserting into several tables at once (for example a parent and its children).
- Upserts (`ON CONFLICT`).
- Editing or deleting saved rows of tables without a primary key.

## Implementation notes

- Extend the pending store with `Insert { temp_id, values: HashMap<Column, Value> }`. Don't create a separate store.
- Put column insert metadata (required, default expression, identity, generated, unique) on the existing column model, filled from the introspection queries for each driver.
- The grid draws pinned new rows in a separate band above the virtualized body, so scrolling and virtualization don't need to know about them.
- SQL generation lives in the per-driver writer that already builds UPDATE/DELETE. Insert ordering and batching happen there.
- Keyboard routing stays in one place: ⌘V/`"+p` reach the paste handler only when no cell editor has focus.

## Tests (headless, no pixels)

Use only the Bookshop demo data in fixtures. Do not add pixel or design snapshot tests, and do not commit snapshot material.

1. ⌘N / `o` creates a new row in the pending store and focuses the first required column; `O` inserts above the cursor row.
2. A row with an empty required column blocks Save; filling it enables Save.
3. Untouched default columns are absent from the generated INSERT; `DEFAULT VALUES` is used when nothing was set.
4. Identity and generated columns are locked and never appear in the INSERT.
5. Duplicate clears the PK, identity, generated and single-column unique columns, keeps the rest (including pending values), and focuses the first cleared required column.
6. Paste with a header matches columns by name, ignores unknown columns, flags an invalid enum value, and "Add N valid" adds only the valid rows.
7. Paste without a header matches columns by position from the cursor column.
8. Inserts, updates and deletes are saved in one transaction, inserts first; a unique violation (23505 / 1062) rolls back everything and leaves all changes pending with the error on the right cell.
9. After a successful save, the row's values equal the `RETURNING` (or MySQL re-select) result, including the real id and `created_at`.
10. Dropping a new row with ⌫ / `dd` generates no SQL; undo brings it back.
11. Read-only connections, views without INSERT and query result sets disable every entry point.
12. A table without a primary key accepts an insert and then reports the saved row as not editable.
13. The generated SQL for PostgreSQL, MySQL and SQLite matches the expected strings for the same Bookshop insert.

## Delivery

- One PR. Each commit passes `cargo fmt --check`, `cargo clippy -- -D warnings` and the headless tests.
- PR description: summary, the test list above, screenshots only if blurred (no unblurred screenshots go to GitHub).
