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

Ctrl+Cmd+1, 2 and 3 show a table's rows, its structure and its DDL on
macOS, and Alt+1, 2 and 3 on Windows.

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

Over the text, **Table**, **Indexes**, **Triggers** and **Comments** go to
that part, with how many statements each holds; a part with nothing is not
listed, and the one at the top of the text is marked. A part near the end
cannot come to the top: its link stays marked until you scroll. **Copy** (Cmd/Ctrl+Shift+C) copies the selected text, or all of it
with nothing selected. **Open in SQL editor** (Alt+Enter, Option+Return on macOS) puts
the text in a new SQL editor that runs nothing until you run it. It opens
read-only or not as Settings › Editor says for a new tab, and always
read-only on production or on a connection that cannot write. On
production, the table's name has the production badge beside it.

## Selecting

Each row has its number at the left of the grid. On macOS and Windows:

- Click a cell to select it. Drag over cells, Shift-click another cell,
  or hold Shift and press the arrows to select the cells between them.
  The footer says how many rows and columns they are.
- Click a row's number to select the row. Shift-click another number to
  select the rows between them, and Cmd/Ctrl-click one to add it or take
  it out. Shift+Space selects the rows of the selected cells, and
  Cmd/Ctrl+A every row loaded.

In the Omarchy look, `v` starts a selection of cells on the cursor's
cell, `Ctrl+V` a block, and `V` whole rows. The moves stretch it, the
status line says its mode (`-- VISUAL --`, `-- VISUAL BLOCK --`,
`-- VISUAL LINE --`) and its size, and Esc leaves it.

A selection holds the rows loaded on the page. Filtering, sorting,
reloading and going to another page end it. A SQL result's rows are
selected the same way.

## Copying

On macOS and Windows, Cmd/Ctrl+C copies the selection with tabs between
the values and a line for each row (the selected cell alone, without a
selection), and Cmd/Ctrl+Shift+C its rows whole. In the Omarchy look `y`
copies the selection, so `v` then `y` copies the cell, and `yy` copies the
cursor's row. One cell alone is copied as it is: its tabs and line breaks
kept, a NULL as `NULL`, and a 16-byte value as the UUID the grid shows.
Anything more, rows or a range, is written with a tab, a line break, a
carriage return or a backslash inside a value as `\t`, `\n`, `\r` or
`\\`, a NULL left empty, and binary values in hex (`0x…`).

**Copy as** copies the selection in another format: CSV, TSV, JSON,
Markdown or SQL INSERT, or only the names of its columns, ready for a
`SELECT` list. Open it with Cmd/Ctrl+Alt+C (`gy` in the Omarchy look) and
pick with one letter: `C`, `T`, `J`, `M`, `S` or `N`. A right-click on a
cell offers it too, as does **Copy row ▾** at the top of the row panel for
its row. With no range selected it copies the cursor's row. Values are
written as the database holds them, never as the grid formats them. A
change not yet saved is copied as it now reads, and SQL INSERT writes a
pending DEFAULT, or a new row's unset value, as a save would. SQL INSERT
writes into the table, or into `result` for a SQL result. The
toast says what was copied: "Copied 4 rows × 2 columns as Markdown".

The clipboard takes up to 10,000 rows and 20 MB. A larger selection is not
copied, and the toast says so. Copying records nothing in the history.

## Exporting

**Export…** writes a table, or a SQL editor's result, to a file or to
the clipboard: in the table's header, in **File › Export…**, or with
Cmd/Ctrl+Shift+E on the table in front or the one selected in the
sidebar. In the Omarchy look press `E` on the rows or on a table in the
sidebar, or type `:export`, or `:export json` to start in that format.

There are other ways in:

- **The sidebar:** right-click a table or view for **Open**,
  **Structure**, **DDL**, **Copy name** and **Export…**. **Import…** is
  there for later.
- **Quick open:** `> Export reviews…` exports the table in front, and
  `> Export a table…` lists the connection's tables and views for you to
  pick one, which opens with the export over it. With a table or view
  picked in quick open, Cmd/Ctrl+Shift+E (`Ctrl+E` in the Omarchy look)
  opens it with the export over it. **File › Export a Table…** is the
  same as the second row. It has no key of its own: give it one in
  **Settings › Keyboard**.
- **The grid:** right-click a table's or a result's rows for **Export selection…**. It
  starts on the selected rows (on a single cell, its row). A block of
  cells starts on its own columns too, ticked under **Choose**, so the
  export holds just what you selected. With a range selected,
  Cmd/Ctrl+Shift+E, `E` and the header's **Export…** start the same way;
  with none they start on all the rows matching the filter.
- **Copy as:** when a table's selection is more than the clipboard takes,
  the toast offers **Export…**, which opens the export of that selection
  in the same format, to a file (Copy rows as TSV's, with every column).
  In the Omarchy look the status line says `E export`, and the selection
  stays for `E`. A SQL result's copy offers the same.
- **The row panel:** **Copy row ▾** ends with **Export row…**
  (Cmd/Ctrl+Shift+E on the panel, `E` in the Omarchy look), which exports
  the panel's row alone, whatever else is selected.

The sheet asks for:

- **Format:** CSV, TSV, JSON, JSON Lines, SQL INSERT or Markdown, with the
  same options as Copy as. Each format keeps the options you last chose.
- **Rows:** the selected rows, the current page, all rows matching the
  filter (in the table's sort), or the whole table (in its key's order).
  The selected rows and the page are written as the grid loaded them. The
  others are read again from the database. **Count exactly** counts the
  rows matching a filter, which have no estimate.
- **Columns:** all of them, or **Choose** to leave some out and reorder
  them. Space includes or leaves out a column, Alt+Up and Alt+Down move it
  (`K` and `J` in the Omarchy look), and in the macOS look a drag on its
  handle moves it too.
- **Destination:** a file, or the clipboard (up to 10,000 rows and 20 MB).

**Review SQL** shows the `SELECT` the export will run, before anything
runs. The export only reads: it runs on a read-only connection of its
own, so the table stays usable while it runs, and production and
read-only connections can export too. A change not yet saved is not
exported, and the sheet says so. Values are written as the database
holds them, and a value the grid cuts off at 256 KiB is written whole.

In the macOS look the system's save panel names the file (on Linux, the
desktop's). In the Omarchy look the sheet's **save to** line does: your downloads folder
first, and a name you type without a folder goes there too. As you type,
the line lists up to eight names of its folder that start with what you
typed, folders first: Ctrl+N and Ctrl+P (or Down and Up) move in the
list, Enter takes a name (a folder then lists its own), and Esc closes
the list. Names starting with a dot are listed once you type the dot.
Ctrl+O opens the desktop's save dialog instead; the name you choose there
fills the line, and nothing is written until you press Enter. Changing the
format changes the name's ending, and typing a known ending changes the
format. `:w ~/Downloads/reviews.csv` writes the table straight away, in
the format of its ending, with the filter and the sort, and `:w!` replaces
a file of that name. `:w` alone still saves your changes to the database.

The file is written to a hidden temporary file beside it, then renamed
into place, so a cancel or a failure never leaves half a file under its
name. It is readable by your account only. While it runs the toast shows
how far it is (hide it with ×; the pill at the bottom right brings it
back), with **Cancel**; the Omarchy status line shows it too, and
`:export cancel` stops it. When it is done the toast offers **Show in
Finder** (**Show in folder** on Windows and Linux, `go` in the Omarchy
look). A failure stays until you dismiss it,
with what to do next: **Try again**, **Choose folder…** or **Review SQL**.
In the Omarchy look `:export` does what the line offers: it tries again,
shows the SQL after a timeout, or opens the float for another name, and
Esc puts the line away.

Several exports can run at once, each on its own connection. In the
macOS and Windows looks their toasts stack at the bottom, the newest
last, each with its own **Cancel** and ×, and each hidden one is a pill
of its own at the bottom right. Cmd/Ctrl+Shift+E and **File › Export…**
always open the sheet for one more, and quick open lists **Cancel
exporting reviews.csv** for each one running. The Omarchy status line
names them all while two or more run (`2 exports · reviews.csv 58% ·
events.jsonl 4,000 rows · :export cancel`), and `:export cancel` then asks
which: a digit, or Ctrl+N and Ctrl+P then Enter, cancels the one picked,
and Esc closes it. An end takes the line while it shows, the last to end
first, and `go`, Esc and `:export` act on that one.

An export to the clipboard that turns out larger than the clipboard
takes offers **Export…** to a file instead (`:export` in the Omarchy
look).

In the macOS and Windows looks Cmd/Ctrl+Shift+' moves the keyboard to
the newest export's toast (or to Copy as's offer): Tab and Shift+Tab go
round every toast's buttons, top to bottom, Return or Space presses one,
and Esc gives the keyboard back to where it was.

When an export to a file finishes while Tabletist is not the window in
front, it tells you: a desktop notification on Linux ("Exported 31,400
rows to reviews.csv", which mako shows on Omarchy), the Dock icon's
bounce on macOS, and the taskbar button's flash on Windows.

The history keeps the statement an export ran, as an **Export** entry,
never its rows. Exporting the loaded rows runs nothing, and records
nothing. **File › Import…** is there for later and does nothing yet.

### Exporting a SQL result

A [SQL editor's]({% link _guide/sql-editor.md %}) result exports as a table
does: **Export…** at the right of the results bar (on Results and on
Messages), Cmd/Ctrl+Shift+E, `E` on the result's rows or `:export` in the
Omarchy look, **Export selection…** on the rows' right-click menu, and
`> Export result of Query 1…` in quick open, and **File › Export…** with
the editor in front. **Copy as** beside it opens
the [Copy as](#copying) menu on the cells you picked. The sheet is
**Export result**, and its file is named `query-1.csv` (for Query 1).

Its rows are:

- **Selected rows:** the cells or rows you selected, as loaded.
- **Rows already loaded:** the result as the editor shows it, up to its
  row limit ("limit reached" when the limit cut it). Nothing runs.
- **Run again without the limit:** the statement runs again as you wrote
  it, with no row limit and no timeout from **Settings › Editor**, on the
  export's own read-only connection, and every row it returns is
  streamed to the file. The rows are counted as they come. It is offered
  just where **Show all** is: for a result the limit cut, of a run that
  only read, while the editor still holds the statement as it ran.
  Otherwise it is greyed and says why ("this run changed data", "every
  row is loaded", or "the statement was edited since it ran"). A
  statement that writes is never run again, whatever it looks like: the
  export refuses it.

SQL INSERT writes into a table named `result`, with no `CREATE TABLE`
(there is no table to read one from). Nothing exports while the editor
runs, nor a result with no rows. `:w ~/Downloads/query-1.csv`
writes the rows already loaded, at once. Running it again is recorded in
the history as an **Export** of the editor ("Query 1"); exporting the
loaded or selected rows records nothing.

## Next

[Edit a table's values]({% link _guide/editing-data.md %}).
