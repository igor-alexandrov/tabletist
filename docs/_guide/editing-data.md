---
title: Editing Data
description: Edit values in the grid, add rows, review the SQL before it runs, and what happens when someone else changed the same row.
nav_order: 4
---

Editing needs a writable connection. A connection with **Open read-only**
ticked blocks every write, and that is the default for production.
[Read about read-only connections]({% link _reference/connections.md %}#open-read-only).

Today you can change values in existing rows and add rows. Duplicating,
pasting and deleting rows are not built yet.

## Change a value

Select a cell and press Enter or F2, double-click it, or simply start
typing. Tab moves to the next cell and keeps your change.

To edit a long value, open the row panel and press Cmd/Ctrl+I, or
double-click the value there.

| To | Press |
| --- | --- |
| Set the value to NULL | Cmd/Ctrl+Backspace |
| Set the value to the column's DEFAULT | Cmd/Ctrl+' |
| Flip a boolean: true, false, NULL | Space |
| Put the cell back as it was | Cmd/Ctrl+Z |
| Leave the editor without the change | Esc |

Set NULL on a column that cannot be NULL does nothing, and the cell says
why. Pressed on a field of the row panel, the field says it.

The Omarchy look uses vim keys for these. They are in
[The Omarchy Look]({% link _guide/omarchy.md %}#editing).

What you type is checked against the column's type before anything is
sent: numbers and their ranges, booleans, enum values, the maximum length,
and the syntax of JSON.

### Words that are not text

In a column that is not text, three words typed into a cell are what they
say, not their letters. They are read in any case.

| Typed | Is |
| --- | --- |
| `NULL` | NULL, where the column takes it |
| `DEFAULT` | The column's default, where it has one |
| `now()`, `now`, `current_timestamp` | The moment of the save by the database's own clock, in a date or time column |

The cell shows the word until you save, and what the database stored
after. Review SQL shows the keyword as it is sent.

- In a text column these are text: `NULL` there is four letters. Use
  Cmd/Ctrl+Backspace for NULL and Cmd/Ctrl+' for DEFAULT.
- In a JSON column `null` is a JSON document. `NULL` and `DEFAULT` in
  capitals are the database's.
- The same goes for a type that is no number, boolean, date or time and
  may keep text: an enum, `citext`, a UUID, a SQLite column with no type.
  `null` there is four letters, and only the capitals are the database's.
- An array of dates is no date: `now()` there is text.
- A word the column cannot take stays text, and is checked like any other:
  `NULL` in a column that cannot be NULL, `DEFAULT` where there is no
  default, `now()` outside date and time columns.
- A date column gets the date and a time column the time of day. On
  SQLite the time is UTC, to the second.
- While a date or time value is being edited, the **now** button beside
  it sets the same thing. In the Omarchy look the key is Ctrl+T.
- In a new row, a cell you leave alone is its default already, so
  `DEFAULT` there takes the value out again.

{% include shot.html file="macos-edit-pending" alt="The grid with three changed cells, one of them red because its column does not take the text, and the bar of pending changes under it" %}

## Save

Nothing is written while you edit. Your changes stay pending, and a bar
appears with what you can do with them.

- **Save** (Cmd/Ctrl+S) writes all of them in one transaction. Either
  every change is written or none is.
- **Discard all** drops them.
- **Review SQL** (Cmd/Ctrl+Shift+D) shows the statements a save will run.
  **Copy SQL** in that panel copies them.

{% include shot.html file="macos-edit-review" alt="Review SQL open above the bar, showing the statement a save will run and the conditions it runs under" %}

Where the window leaves the bar little room, it says less, and
**Discard all** and **Review SQL** move into the **…** menu beside
**Save**.

A save to a production connection asks first, with every statement on
screen.

{% include shot.html file="macos-edit-production" alt="The question before a save to production, with the statement it will run and a red Save to production button" %}

## Add a row

Press **Add row** in the table's header, or Cmd/Ctrl+N while the rows are
in front. The new row appears at the top of the grid, with the cursor in
its first value that has to be given. In the Omarchy look, `o` opens the
row below the cursor's row and `O` above it.

A new row is green until it is saved, and each of its cells says what the
database will do if you leave it alone:

- **required** (a red `*` after the column's name in the Omarchy look):
  the column cannot be NULL and has no default. Save waits for a value.
- A dimmed value or expression: the column's default.
- **NULL**: the column takes NULL and has no default.
- **+ new**, or an empty cell: the database assigns the value on save, as
  it does for an identity column or a computed one. Such a cell cannot be
  typed into.

Fill it as you edit any row. Cmd/Ctrl+Z on a cell takes its value out
again. Esc leaves the editor and keeps the row.

The row is saved with everything else that is pending, in the same
transaction. Only the columns you set are sent, and **Review SQL** shows
the `INSERT`. After the save the row shows what the database stored: its
id, its defaults. It stays where it was until the next reload or sort.

- Delete drops a new row that is not saved yet (`dd` in the Omarchy
  look). **Discard all** drops every new row with the other changes.
- If the database refuses the row, nothing is written. The row stays, in
  red, with the database's words on it.
- On a table with a trigger, and on MySQL where the new row cannot be
  found again by its key, the page is loaded again after the save and
  the row goes where the sort puts it.
- A table without a primary key takes new rows. They cannot be edited
  afterwards, as no row of such a table can.

The row panel does not show a new row yet: it is edited in the grid.

## When someone else changed the row

A save does not overwrite a row that changed after you loaded it unless
you say so. If it finds one, it writes nothing and asks about each such
row. You see three
versions side by side: what you loaded, what the server holds now, and
yours. Then you choose:

- **Keep mine, reload row** loads the row again and keeps your change
  pending. Esc does the same.
- **Use server values** drops your change to that row.
- **Overwrite** writes your values over the server's.

{% include shot.html file="macos-edit-conflict" alt="The question about a row that changed on the server: what was loaded, what the server holds now and your values, side by side" %}

If someone deleted the row, there is nothing to reload or overwrite:
**Discard my changes** drops your change, and Esc leaves it pending.

## What cannot be edited

A cell that cannot be edited says why when you try. The common reasons:

- The connection opens read-only.
- The object is a view.
- The table has no primary key or unique index, so a row cannot be
  targeted safely.
- The column is part of the row's key, or is computed by the database.
  In a new row a key's column can be given, unless the database numbers
  it.
- The value is binary, or larger than 256 KiB.
