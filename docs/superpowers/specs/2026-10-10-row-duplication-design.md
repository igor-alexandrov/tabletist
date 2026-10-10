# Duplicating a row

Date: 2026-10-10. Status: designed, not built.

This is the first half of run 3 of `2026-10-07-inserting-rows-design.md`
("Duplicate, and Down on the last row"): Duplicate alone. Section 4 of that
spec says what a duplicate is. This text says what of section 4 is built
now, what it leaves open and how that was decided, and what waits. Where
the two differ on a point this text decides, this text holds, and the
insert spec is amended to say so when the run is built (see "Delivery").

It builds on what is merged: a new row in the pending set (`Edits::added`,
`edit::Place`), `App::add_row`, the save's `INSERT` on the three drivers,
and the row panel as a new row's form (pull request 112).

## Goal

A copy of the row under the cursor becomes a new row, pending like any
other: filled in, reviewed and saved by the same Save. Nothing is written
when the copy is made.

## Decisions

Made with the user on 2026-10-10.

| | What was open | Decided | Why |
|---|---|---|---|
| D1 | A binary column, and text over the editor's limit (`MAX_EDIT_BYTES`, 256 KB) | Not copied, and the form's line says which columns | A pending value is text, NULL, DEFAULT or the moment of the save: it has no form for bytes. Giving it one is driver work on three engines and a way to show bytes in Review SQL. A text over the limit is one no editor opens |
| D2 | A NULL in the source | Set in the copy only where the column, left alone, would not be NULL: one with a default. Elsewhere the column stays unset | A new row holds only what was set (insert spec 6), and its `INSERT` names only that |
| D3 | Several rows at once: Mod+D on a selection, `yy` with a count or a visual selection, the cap of 500 | Waits for a selection of several rows | The grid selects one cell. The selection of several is slice 4 of the value editing spec |
| D4 | What `yy` leaves on the system clipboard | The row as TSV, as Mod+Shift+C copies it | The cell's `y` goes there already, and the insert spec's `"+p` (run 4) will read it from there |
| D5 | How a row gets from `yy` to `p` | `yy` keeps the row's values, by column name, on the tab. `p` makes the new row from them | A reload, a sort or an edit of the source between the two keys must not put another row. The clipboard's TSV cannot tell NULL from the text `NULL`, or carry a pending DEFAULT |
| D6 | Down on the last row | Not this run | Asked of the user: Duplicate alone |

## Behavior

### 1. Entry points

| Look | How | Where the copy goes |
|---|---|---|
| macOS, Windows, Linux | Mod+D, with a table's rows in front and a row selected | The top, below the new rows already there (`Place::Top`), as Add row puts its row |
| macOS, Windows, Linux | **Duplicate** in the row panel's footer of a loaded row | The same |
| Omarchy | `yy`, then `p` | Right below the cursor's row (`Place::Below`) |
| Omarchy | `yy`, then `P` | Right above it (`Place::Above`) |

- Mod+D is read where Mod+N is read as Add row: the grid or a field of the
  row panel has the keyboard, in a look that is not the terminal's. The
  press is taken only where it makes a copy. Only a fresh press makes one:
  held down, the key does not repeat.
- The row is the selected cell's row: a row of the page, or a new row.
- The new row's form has no Duplicate button: its footer is the insert
  spec's (section 7). Mod+D and `yy p` copy a new row all the same.
- **Delete** stays disabled. Its reason is the row's own where it has one,
  else "Deleting rows arrives in a later version".

A copy is refused where a new row is refused, by `Table::no_rows`: a
read-only connection, a view or a materialized view, a structure not
loaded yet, a save or a fetch running. And only in the Data view. Then:

- Mod+D and `p` do nothing.
- The Duplicate button is disabled, and says the reason under the pointer
  in the words the Add row button uses for it.
- A table without a key takes a copy, as it takes a new row. So Duplicate
  is not disabled by what keeps the row's fields from being edited (no
  key, a NULL in the key): its reason is its own.

### 2. `yy`, `p` and `P` in the Omarchy look

- The first `y` copies the cell, as it does today, and waits for a second
  the way the first `d` of `dd` waits. The wait ends as that one does.
- The second `y`, with the grid's rows holding the keys, yanks the cursor's
  row: the tab keeps its values (see 3). The status line says "row yanked".
- Where the row is one of the page's, `yy` also puts it on the system
  clipboard as TSV (`App::copy_text(true)`). A new row is not put there,
  as Mod+Shift+C puts none.
- `p` and `P` are read with the letters that edit (`i`, `o`, `O`, ...), and
  under their rule: alone in their frame. With nothing yanked they do
  nothing. With no cell selected the copy goes to the top.
- A yank stays until the next one, or until its tab closes. It survives a
  reload, a sort, a filter, a save and Discard. `p` can be pressed again
  for another copy.
- A yank is its tab's: it is not put into another table.
- The row panel's footer shows `yy p duplicate` as a live key where a copy
  can be made, no longer dashed and faded. `dd delete` stays as it is.

### 3. What a yank holds

For each column of the page, by its name: what the row shows there.

- A row of the page: the pending value where the cell has one, else the
  loaded value.
- A new row: the value set in the cell. A cell nothing is set in is not
  held.

Mod+D and the button take the same from the selected row and make the copy
at once. They leave the tab's yank alone.

### 4. What the copy gets

One function decides, for each column of the page as it is when the copy
is made, in this order. The first that applies wins.

| | The column | In the copy | Said as |
|---|---|---|---|
| 1 | The structure does not list it, or the page names it twice | unset | not copied (unknown) |
| 2 | Identity | unset | cleared (identity) |
| 3 | Generated | unset | cleared (generated) |
| 4 | Of the primary key (`Structure::primary_key`) | unset | cleared (primary key) |
| 5 | Alone in a unique index | unset | cleared (unique) |
| 6 | Of a binary type, or holding bytes | unset | not copied (binary) |
| 7 | Holding text over `MAX_EDIT_BYTES` | unset | not copied (too large) |
| 8 | Anything else | the source's value | copied |

- Row 5: an index with `unique` set and one entry in `columns` that is the
  column's name. A partial index, a prefix index and one in another
  collation count: a copy of the value would collide there too, or might.
  An index over an expression names no column and clears none.
- A primary key of several columns is cleared whole. A unique index of
  several columns clears none: the copy holds them and the database judges
  the save, as the insert spec has it.
- Rows 6 and 7 are asked of the value the yank holds, and row 6 of the
  column's type too.
- A column the yank does not hold (a new row's unset cell, a column the
  table gained since) is unset in the copy. It counts as copied where rows
  1 to 7 do not apply: the copy has there what the source has.

A copied value becomes a pending cell of the new row:

- **Loaded, not NULL:** its whole text as an editor starts with it
  (`edit::start_text`), so a boolean reads `true` or `false`. A JSON
  document keeps the text it loaded with, not laid out: what is written is
  the document as it was.
- **NULL**, loaded or pending: set as NULL where the column has a default
  (`edit::unset` gives `Default` or `Expression`). Elsewhere not set (D2).
- **Pending:** the value as it is, DEFAULT and the moment of the save
  among them.
- A text is checked as a typed one is (`edit::check`): it waits for the
  save, or is to fix with its problem. What the source's cell failed with
  in a save is not carried: the copy has not been sent.

### 5. After the copy

- The new row is added through what Add row uses, so the tab is pinned,
  the grid scrolls to the row, an open editor is closed keeping its text
  (and that text, now pending, is what the copy takes), and the counts,
  the pending bar and Review SQL see one more new row.
- The editor opens on the first column, in the page's order, that a save
  needs a value in and that takes one. With no such column the selection
  goes to the new row, in the column the cursor was in, and no editor
  opens.
- **Required, and why.** A column of the copy that a save needs a value in
  and that was cleared as unique or as of the primary key says
  `required · unique` wherever a new row's column says `required`: the
  grid's cell and the form's field in the desktop looks, the status line's
  `publisher_id required · unique` in the terminal's where one column is
  lacking. Every other required column says `required`, as today.
- **The form's line.** The row panel, as the copy's form, says at its top,
  where the note of a table without a key stands, and above that note:

  > Copied: title, format and 3 more. Cleared: id (identity), isbn (unique). Not copied: cover (binary).

  - A sentence with no column is left out.
  - Copied names up to three columns in full. With more it names the first
    two, then "and N more".
  - Cleared and Not copied name every column, each with its reason: the
    words in brackets in the table of section 4.
  - Columns are in the page's order.
  - It goes through the locale and the look's case, wraps to the panel's
    width, and is announced whole to a screen reader, as the note under it
    is.
  - It is what the copy was made as, kept on the new row. It does not
    change as the row is filled in, and goes with the row: dropped,
    discarded or saved.
- Everything else of the copy is a new row's: it is edited, dropped
  (Delete, `dd`), unset a cell at a time (`u`), reviewed and saved as one.

## What waits

- Several rows at once (D3).
- A binary value and a text over the limit in a copy (D1).
- Down on the last row (D6): the other half of run 3.
- The copy pinned under the header on macOS: "2b, the rest" of the insert
  spec. Until then it stands at the top of the page's rows and scrolls
  with them, as an added row does.
- Undo of a copy as one step: slice 4 of the value editing spec.

## Out of scope

- A context menu on the grid's rows. The grid has none.
- A menu item on macOS. Add row has none.
- Duplicating a row of a SQL result.

## Implementation notes

- `src/edit.rs`: the yank (a column's name and what it holds: a loaded
  `Value` or a pending `NewValue`), the function of section 4 from a yank
  and a `Table` to the pending cells and the three lists, and the lists
  kept on `NewRow`. No state of the app is read there: it is tested alone.
- `src/model.rs`: `Action::DuplicateRow { tab, id }`, `YankRow { tab, id }`
  and `PutRow { tab, id, above }`, and the tab's yank on `ObjectTab`.
- `src/app/editing.rs`: the three, on `add_row`'s path. What `add_row` does
  after the row is in the set (the selection, the editor, the pane) is
  shared and not written twice.
- `src/ui/keys.rs`: Mod+D beside Mod+N, the second `y` beside the second
  `d`, `p` and `P` among the editing letters, and `SHORTCUTS`.
- `src/ui/row_panel.rs`: the footer's button and key, and the form's line.
- `src/ui/data_view.rs`, `src/ui/workspace.rs`: `required · unique`.
- A view pushes the action and changes nothing. Nothing here asks the
  database anything, and nothing depends on the engine but through
  `Dialect`, which the functions used already match in full.

## Tests (headless, no pixels)

On the Bookshop's `book_covers`, with a unique index on `publisher_id`
added to the fixture's structure where a test needs one.

1. Mod+D adds one new row (the audit's INS-02a, out of `ignore`).
2. `yy` then `p` adds one below the cursor's row, `P` one above (INS-02b,
   out of `ignore`).
3. The insert spec's test 5: the copy leaves `id` unset (identity) and
   `publisher_id` unset (unique), holds `kind` and `created_at`, and the
   editor is open on `publisher_id`.
4. A pending edit of the source is what the copy holds, not the loaded
   value. A pending DEFAULT is carried as DEFAULT.
5. A NULL is not set where the column has no default, and is set where it
   has one.
6. A new row as the source: its set cells are copied, its unset ones stay
   unset.
7. A primary key of several columns is cleared whole. A unique index of
   two columns clears neither.
8. A binary column and a text over the limit are not copied, and the line
   names them.
9. A copied value that fails its column's check is to fix, and a save
   waits for it.
10. With nothing required of the copy, no editor opens and the selection
    is on the new row in the cursor's column.
11. A yank survives a reload of the page. `p` twice makes two copies. `p`
    with nothing yanked makes none.
12. `yy` puts the row's TSV on the clipboard and says "row yanked".
13. Read-only, a view, and a save in flight: Mod+D, `p` and the button
    make no copy, and the button says why. A table without a key takes a
    copy.
14. The Duplicate button makes a copy. Delete is disabled and says that
    deleting arrives later. A SQL result's row has neither, as before.
15. The form's line, in both looks: the three sentences, "and N more", a
    sentence left out when it is empty.
16. `required · unique` in the grid's cell, the form's field and the
    terminal's status line, and `required` for a column that was not
    cleared as unique.
17. Review SQL shows the copy as one `INSERT` naming the copied columns
    alone.
18. `SHORTCUTS` lists Mod+D for the desktop looks and `yy`, `p`, `P` for
    the terminal's.

Tests that hold today's wording change with it: the row panel's "Duplicate
and Delete wait" tests, and the terminal's `yy p duplicate` as a dashed
key.

## Website

- `docs/_guide/editing-data.md`: "Duplicate a row" under "Add a row", with
  what is cleared and what is not copied. Its line on what the row panel
  shows for a row.
- `docs/_reference/keyboard-shortcuts.md` and `docs/_guide/macos.md`:
  Mod+D and Cmd+D.
- `docs/_guide/omarchy.md`: `yy`, `p`, `P`.

## Delivery

One pull request. Each commit passes `cargo fmt --all --check`, `cargo
clippy --locked --workspace --all-targets -- -D warnings`, `cargo test
--locked --workspace --all-targets` and the `cargo doc` check.

The last commit amends `2026-10-07-inserting-rows-design.md`: section 4
gets D1, D2 and D3 where it says otherwise, its Delivery table has run 3
as "3, Duplicate" (built) and "3, the rest" (Down on the last row), and
its status names this spec. `2026-10-06-row-inspector-inline-edit-design.md`
and `2026-10-06-row-form-design.md` say of Duplicate what the app does.
