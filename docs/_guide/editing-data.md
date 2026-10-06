---
title: Editing Data
description: Edit values in the grid, review the SQL before it runs, and what happens when someone else changed the same row.
nav_order: 4
---

Editing needs a writable connection. A connection with **Open read-only**
ticked blocks every write, and that is the default for production.
[Read about read-only connections]({% link _reference/connections.md %}#open-read-only).

Today you can change values in existing rows. Adding, duplicating and
deleting rows are not built yet.

## Change a value

Select a cell and press Enter or F2, double-click it, or simply start
typing. Tab moves to the next cell and keeps your change.

To edit a long value, open the row panel and press Cmd/Ctrl+I, or
double-click the value there.

| To | Press |
| --- | --- |
| Set the value to NULL | Cmd/Ctrl+Backspace |
| Put the cell back as it was | Cmd/Ctrl+Z |
| Leave the editor without the change | Esc |

The Omarchy look uses vim keys for these. They are in
[The Omarchy Look]({% link _guide/omarchy.md %}#editing).

What you type is checked against the column's type before anything is
sent: numbers and their ranges, booleans, enum values, the maximum length,
and the syntax of JSON.

## Save

Nothing is written while you edit. Your changes stay pending, and a bar
appears with what you can do with them.

- **Save** (Cmd/Ctrl+S) writes all of them in one transaction. Either
  every change is written or none is.
- **Discard all** drops them.
- **Review SQL** (Cmd/Ctrl+Shift+D) shows the statements a save will run.
  **Copy SQL** in that panel copies them.

A save to a production connection asks first, with every statement on
screen.

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

If someone deleted the row, there is nothing to reload or overwrite:
**Discard my changes** drops your change, and Esc leaves it pending.

## What cannot be edited

A cell that cannot be edited says why when you try. The common reasons:

- The connection opens read-only.
- The object is a view.
- The table has no primary key or unique index, so a row cannot be
  targeted safely.
- The column is part of the row's key, or is computed by the database.
- The value is binary, or larger than 256 KiB.
