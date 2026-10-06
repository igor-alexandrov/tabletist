---
title: Browsing Data
description: The sidebar, the data grid, sorting, filters, the row panel and the Structure view.
nav_order: 3
---

## The sidebar

The sidebar shows the tables and views of one schema at a time. Tables
that share a prefix, such as `book_`, fold into a group. You can also
list them flat. On macOS and Windows, the objects you opened recently
are listed above them.

Quick open (Cmd/Ctrl+P) finds any object by name without the sidebar.

System schemas such as `pg_catalog` and `information_schema` are hidden.
[A setting]({% link _reference/settings-and-files.md %}#the-settings-file)
shows them.

## The data grid

A table opens on its data. The grid marks primary keys and foreign keys,
shows JSON at a glance, and draws enum, CHECK and boolean values as colored
tags.

{% include shot.html file="macos-table" alt="A table's data grid sorted by a column, with a row open in the row panel beside it" %}

- **Pages.** The grid loads 300 rows at a time unless you change
  [Rows per page]({% link _reference/settings-and-files.md %}#the-settings-window)
  in Settings. Cmd/Ctrl+Alt+Left and Right move between pages.
- **Sorting.** Click a column's header to sort by it: ascending, then
  descending, then not at all. The server does the sorting.
- **Counts.** The footer shows the range you are looking at and the
  table's size. For a large table the size is the database's estimate, such
  as `~1.2M`. Click **Count** for the exact number.
- **Cancel.** Cmd/Ctrl+. cancels a running query.
- **Refresh.** Cmd/Ctrl+R loads the page again.

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
with long text and JSON documents laid out to read. Space shows or hides
it.

A very large value is shown up to 256 KiB. Copying it gives the whole
value.

A foreign key has a link next to it that opens the row it points to.

## Structure

Switch a table from **Data** to **Structure** to read its columns, indexes
and foreign keys.

{% include shot.html file="macos-structure" alt="The Structure view of a table, listing its columns and indexes" %}

## Copying

Cmd/Ctrl+C copies the selected cell. Cmd/Ctrl+Shift+C copies the whole
row, with tabs between its values.

## Next

[Edit a table's values]({% link _guide/editing-data.md %}).
