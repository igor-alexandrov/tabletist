---
title: Browsing Data
description: The sidebar, the data grid, sorting, filters, the row panel, and the Structure and DDL views.
nav_order: 3
---

## The sidebar

The sidebar shows the tables and views of one schema at a time. Tables
that share a prefix, such as `book_`, fold into a group. You can also
list them flat. On macOS and Windows, the objects you opened recently
are listed above them.

Quick open (Cmd+P on macOS, Ctrl+P on Windows and Omarchy) finds any
table or view by name without the sidebar, in this connection or in every
open one. On macOS and Windows the sidebar's "Find any object…" field
opens it, and `>` typed first lists the commands instead (and, with a
word after it, the settings); in the Omarchy look `/` in the sidebar
filters its list.

System schemas such as `pg_catalog` and `information_schema` are hidden.
[A setting]({% link _reference/settings-and-files.md %}#the-settings-file)
shows them.

## The data grid

A table opens on its data. The grid marks primary keys and foreign keys,
shows JSON at a glance, and draws enum, CHECK and boolean values as colored
tags.

{% include shot.html file="macos-table" alt="A table's data grid sorted by a column, with a row open in the row panel beside it" %}

- **Pages.** The grid loads 1,000 rows at a time unless you change
  [Rows per page]({% link _reference/settings-and-files.md %}#the-settings-window)
  in Settings. Cmd/Ctrl+Alt+Left and Right move between pages.
- **Moving.** The arrows move a cell at a time. Cmd/Ctrl with an arrow
  goes to the first or last row, or the first or last column, in the
  macOS and Windows looks.
- **Sorting.** Click a column's header to sort by it: ascending, then
  descending, then not at all. The server does the sorting.
- **Counts.** The footer shows the range you are looking at and the
  table's size. For a large table the size is the database's estimate, such
  as `~1.2M`. Click **Count** for the exact number.
- **Cancel.** Cmd/Ctrl+. cancels a running query. In the Omarchy look
  the key is Ctrl+C.
- **Reload.** Cmd/Ctrl+R loads the page again. In the Omarchy look the
  key is `R`.

A table without a primary key is marked **Unordered**: its rows may move
between pages.

## Filters

Press Cmd/Ctrl+F, or click **Add filter**. A filter is a list of
conditions, and a row must match all of them. Each condition is a column,
an operator and a value:

| Operator | Matches |
| --- | --- |
| `=`, `≠`, `<`, `>`, `≤`, `≥` | A comparison with the value |
| `contains` | The value anywhere in the text. PostgreSQL ignores letter case, MySQL follows the column's collation, and SQLite ignores the case of ASCII letters |
| `starts with` | Text that begins with the value |
| `in (a, b, …)` | Any of the values, separated by commas |
| `is NULL`, `is not NULL` | Whether there is a value at all |

For anything else, tick **Raw WHERE** and write the condition yourself:

```sql
id > 10 AND name LIKE 'A%'
```

{% include shot.html file="macos-filter" alt="The filter bar open above the grid, with one condition being written" %}

On MySQL, a raw WHERE reads `"..."` as a string, and names take backticks.

## The row panel

Select a row and the panel on the right shows every one of its fields,
with long text and JSON documents laid out to read. Cmd/Ctrl+Shift+R
shows or hides it, and a connection opens with it as its window last
left it.

A very large value is shown up to 256 KiB. Copying it gives the whole
value.

A foreign key has a link next to it that opens the row it points to.

## Structure

Switch a table from **Rows** to **Structure** to read its columns, indexes
and foreign keys.

{% include shot.html file="macos-structure" alt="The Structure view of a table: its comment, the label for foreign keys, then its columns with their comments and its indexes" %}

On PostgreSQL and MySQL the view shows the comments written on the table
and its columns. The table's comment stands above everything else. Each
column's is in the **Comment** column, on one line: hold the pointer over
a comment that is cut to read all of it. SQLite keeps no comments, so its
tables have no Comment column. In a table's grid, hold the pointer over
a column's header to read its comment.

Above the columns, **Label for foreign keys** names the column that this
table's rows are read by in the list of a foreign key that points at it,
and says whether the app picked it (**Automatic**) or you did (**Set by
you**). The column itself is tagged **label**.
[Editing Data]({% link _guide/editing-data.md %}#the-label-column) says
what the label does.

- **Change…** (Cmd/Ctrl+L) opens the chooser of columns.
- Click a column's line and press Cmd/Ctrl+Alt+L to use that column, or
  right-click the line for **Use as label**. With the keyboard, Tab or F6
  comes to the columns, and Up and Down move between them.
- **Use Automatic** hands the pick back to the app.

No cell is edited and nothing is written to the database, so this works on
a read-only connection. The Omarchy look does the same with `gl` and the
`:label` lines of [its Structure view]({% link _guide/omarchy.md %}#the-structure-view).

## DDL

**DDL**, the third view of a table, a view or a materialized view, shows
its definition as text you can select and copy but not change, in the SQL
editor's colours, with line numbers. Long lines do not wrap: scroll
sideways to read them. Each tab keeps the view it shows, and a tab the
window reopens comes back in it.

- On **PostgreSQL**, which has no `SHOW CREATE TABLE`, the text is built
  from the catalog in the order `pg_dump` writes it: the `CREATE TABLE`
  with its columns and constraints, then the indexes that back no
  constraint, the triggers (a disabled one followed by the `ALTER TABLE`
  that disables it) and the comments. Tables, types and functions are
  named with their schema, so the text runs whatever the search path. The
  owner, the grants, the row-level policies and the sequences the table
  owns are not included, as the footer says. Nor are an identity's
  sequence options (its start and step), a tablespace or a table's access
  method. Each table's foreign keys are in its own `CREATE TABLE`, so two
  tables that point at each other cannot be made from their texts as
  they are.
- On **MySQL** it is `SHOW CREATE TABLE` (or `SHOW CREATE VIEW`), then
  each trigger's `SHOW CREATE TRIGGER` under a `-- Triggers` line.
- On **SQLite** it is the text the file keeps for the table or view, then
  for its indexes and triggers. Indexes SQLite made by itself keep no
  text and are left out.

On MySQL and SQLite each statement is as the database returns it: the
view only ends one with `;` where it has none, and sets the statements
apart.

The definition is read once, the first time the view shows, and kept with
the tab: Reload (Cmd/Ctrl+R, or **Reload** over the text) reads it again,
with the rows and the structure, as Reload does in the other views.
Reading it only reads the catalog, and adds nothing to History. Where the
connection's user may not read it, the view shows the database's error
and **Try again**, never part of a text; Rows and Structure still work.

## Copying

On macOS and Windows, Cmd/Ctrl+C copies the selected cell and
Cmd/Ctrl+Shift+C the whole row, with tabs between its values. In the
Omarchy look `v` then `y` copies the cell.

## Next

[Edit a table's values]({% link _guide/editing-data.md %}).
