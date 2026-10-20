# Column and table comments in the Structure view

Issue: igor-alexandrov/tabletist#193. User feedback: "The data structure
has no Comment field (PostgreSQL and MySQL support it)."

## What happens now

`ColumnInfo.comment` is read on PostgreSQL (`col_description`) and MySQL
(`column_comment`), and SQLite has none. Only the MCP schema tool says it.
The Structure view's Columns table shows Name, Type, Nullable, Default and
Key. A table's own comment is not read at all.

## Decision

All three asks of the issue: a Comment column in the Structure view, the
table's comment over the view, and a column's comment as the tooltip of
its header in the grid. The MCP schema tool says the table's comment too.

The table's comment is a field of `Structure`, read by the `describe` that
reads the columns: one fetch, one cache, one refresh. Rejected: putting it
on `ObjectInfo` (every sidebar listing would read comments for one view),
and a fetch of its own (an action and backend path for one string).

## Database (`crates/tabletist-db`)

- `Structure` gets `pub comment: Option<String>`: the comment on the table
  or view itself. `None` where there is none or it is empty.
- PostgreSQL: `obj_description($1, 'pg_class')` on the oid `describe`
  already resolves. Tables, views and materialized views alike.
- MySQL: `table_comment` from `information_schema.tables` by schema and
  name. MySQL writes `VIEW` there for every view: a view (`table_type =
  'VIEW'`) has `None`. An empty comment is `None`.
- SQLite: `None`.
- The `Structure` literals without `..Default::default()` (the three
  `describe`s, `src/backend.rs`, some in `src/shots.rs` and `src/ui/mod.rs`)
  name the field; the compiler finds them.

## Structure view (`src/ui/structure.rs`)

- **The table's comment**: at the very top of the view, over the label
  card (over Columns for a view, which has none). Secondary text, wrapped
  at 640 pt (`HELP`) or the view's width, whichever is less. Laid out through `for_display`: line breaks stay, hidden
  characters are written out. Nothing, and no space, without a comment.
- **A Comment column** after Key, titled "Comment". Whether there is one is
  the engine's: a function matching `Driver` with every variant named,
  `Postgres` and `MySql` yes, `Sqlite` no. On the first two it always
  shows, empty or not, so an engine's tables keep one shape.
- **Its cell**: the comment's first line, first cut to a local
  `COMMENT_LINE_CHARS` (256, as `NAME_MAX_CHARS`: a PostgreSQL comment has
  no limit, and the cell is measured every frame), then hidden characters written out (`escape_hidden`), then
  cut with "…" to at most 320 pt by `grid::ellipsize` in the cell's font. Lines stay one high, so the stripes and the cursor's
  fill are unchanged. When it was cut, or has more lines, the whole
  comment (through `for_display`) shows on hover. The cell's accessible
  label is the whole comment as `for_display` lays it out.
- The terminal look draws the same column and paragraph with its own
  roles. A cut comment is read whole by hovering, or by a screen reader;
  no key shows it (left for later if asked for).

## Grid header (`src/ui/grid.rs`, `src/ui/data_view.rs`)

- `grid::Column` gets `comment: Option<&'a str>`. The data view fills it
  from the structure it already holds, by column name. SQL results, the
  settings preview and the tests' columns pass `None`.
- A header with a comment shows it on hover (through `for_display`),
  sortable or not, in every look. Its AccessKit node takes the comment as
  its description.

## MCP schema tool (`crates/tabletist-mcp/src/tools/schema.rs`)

- A table's entry has `"comment"` where the table has one, through
  `answer::text` as a column's comment is. A hidden table is never
  described, so nothing of it is said.
- It is left out where the table has a masked column, or is a view marked
  `reads_masked`: like a masked column's comment, its words are its
  author's and may give one of that column's values as an example. The
  tool's description says so, in its sentence on what is left out.

## Docs and screenshot

- `docs/_guide/browsing-data.md`, Structure: on PostgreSQL and MySQL the
  view shows the table's comment over it and a Comment column, cut to one
  line with the whole on hover; in the grid, a column's comment is its
  header's tooltip. The `macos-structure` shot's alt text names the
  table's comment over the card.
- `src/shots.rs`: the Bookshop `book_images` scene gets a table comment and
  comments on two columns. `assets/screenshots/macos-structure.webp` is
  rendered again where the shots run (they need a GPU); if they cannot run
  here, that is said.

## Tests

- PostgreSQL and MySQL integration: `COMMENT ON TABLE` / `COMMENT=` is
  read; a table without one reads `None`; a PostgreSQL view's
  `COMMENT ON VIEW` is read; a MySQL view reads `None`.
- MCP: a table's entry carries its comment, none without one, and none
  for a table with a masked column or a view marked `reads_masked`.
- Unit: the Comment cell's text (first line, cut, whether a tooltip is
  needed) for a short, a long and a multi-line comment.
- Headless, through `src/testing.rs`:
  1. On PostgreSQL the Columns table has a Comment title and a column's
     comment in its line; a long one is cut, with the whole as its label.
     A comment of 100 000 characters on one line draws: the frame ends,
     and the cell's drawn text is short and ends in "…".
  2. On SQLite there is no Comment title.
  3. A table's comment shows over the view; with none, nothing does.
  4. A grid header of a column with a comment has it as its description;
     one without has none.
