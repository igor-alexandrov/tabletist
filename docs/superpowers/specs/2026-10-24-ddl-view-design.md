# The DDL view

User feedback on the latest build: "There is no way to see the DDL."
Nothing read a definition: no `pg_get_*`, `SHOW CREATE TABLE` or
`sqlite_master.sql` anywhere in `crates/tabletist-db` or `src/`, and the
Structure view lists columns, indexes and foreign keys only.

The DDL view is a third view of a table, a view or a materialized view,
beside Rows and Structure: the object's definition as read-only text.

Design canvas: **Tabletist**, section **Table view · Structure**, boards
`macOS – DDL view`, `macOS – DDL, by driver and state` and
`Omarchy – DDL view`.

## Delivery: three runs, a pull request each

Asked and answered on 2026-10-24 (the prompt asked for one pull request):

1. **Run 1 (this spec's detail): read and show.** The three engines'
   read with its tests and the Bookshop fixtures it needs, the
   definition held by the tab, the DDL view with its text, its loading
   and error states, the Rows · Structure · DDL switch in both looks, and
   the view kept with the tab when the window reopens.
2. **Run 2: act on it.** Copy ⇧⌘C, Open in SQL editor ⌥↩, the jump links
   and the Omarchy sections outline, and every key: ⌃⌘1/2/3 (Alt+1/2/3 on
   Windows), Omarchy `r s S`, `:ddl`, and in the view `j/k gg G ]] [[ V y
   e esc`, with Settings › Keyboard rows under Table and the status line's
   "2 lines selected · 10/27".
3. **Run 3: find and menu.** ⌘F and `/ n N` in the text, and the macOS
   View menu's Rows, Structure and DDL.

## Decisions taken with the user

| Question | Decision |
| --- | --- |
| Omarchy `s` is Replace cell on the rows (vim's substitute; `cc` does the same) | `s` is Structure from every view; Replace cell keeps `cc` only (run 2) |
| ⌃⌘1/2/3 folds to Ctrl+1/2/3 on Windows, which goes to a connection window | ⌃⌘1/2/3 on macOS, Alt+1/2/3 on Windows: the keymap learns a Windows-only spelling (run 2) |
| One pull request | One per run |

## Where the prompt's premises and the build differ

| The prompt says | The build | Run 1 does |
| --- | --- | --- |
| "the build already says Rows" | The switch said **Data** (`data` on Omarchy) | Renames it **Rows** / `rows` (the prompt's default) |
| `macOS – Structure` has a **Show DDL** button to remove | The build has none | Nothing to remove |
| ⌘F finds "as in the SQL editor" | The SQL editor has no find | Find is run 3, built for the DDL text |
| The view is remembered "like Rows and Structure" | The view was not kept in `session.json` at all | Keeps it (`"view": "structure"` or `"ddl"`), for Structure too |
| "Cache the result with the schema cache" | There is no backend schema cache: a tab holds its structure, the workspace the completion list's columns | The tab holds its DDL, as it holds its structure (below) |
| "Timeouts follow the app's catalog queries" | The catalog reads (`describe`, the listings) have no timeout | So the DDL read has none |
| Tests on `book_covers` and `in_stock` | The Bookshop has no `in_stock`, and `book_covers` has no index, trigger or comment | The fixtures gain them (below) |

## Where the build differs from the boards

- The first view is **Rows** / `rows`, not Data / `d data` (the prompt).
- The PostgreSQL text's first line is the same for every object:
  `-- public.in_stock · built from the catalog · PostgreSQL 17.2`. The
  view board's `-- public.in_stock · view · pg_get_viewdef` is not used
  (the prompt states one first line).
- The MySQL board's `-- Triggers · SHOW CREATE TRIGGER` is `-- Triggers`
  (the prompt: the only lines added are the section comments).
- Statements are not wrapped: `pg_get_triggerdef` and
  `pg_get_constraintdef` return one line, and nothing is reformatted. The
  boards wrap a trigger and a foreign key over two lines.
- Views are read with an empty search path, so `pg_get_viewdef` writes
  every table with its schema (`FROM public.books b`); the view board
  shows the names bare.
- The SQLite text has no section comments: a blank line keeps its
  statements apart, as the prompt says (the board shows none either).
- The error card keeps the meaning sentence under the card, as the app's
  other error states do, and the terminal's retry button reads
  `try again R` (Reload's key).
- Until run 2 the header has no Copy or Open in SQL editor, and no jump
  links; Omarchy has no outline, and its key line names Reload and the
  sidebar (and write, and review, as the Structure view's line does).
- The Omarchy view words are laid out left to right in their order, so a
  screen reader reads them in it (before, right to left).

## Database (`crates/tabletist-db`)

- `ddl::Ddl { driver, server, heading, parts }`, `ddl::Part { kind,
  statements }`, `ddl::PartKind { Object, Indexes, Triggers, Comments }`,
  `ddl::Source { Catalog, Stored }`. Run 2's jump links and outline will
  list the parts, and the text is joined from the same parts, so the two
  cannot disagree. A part with nothing in it is left out. `Ddl::text()`
  joins them:
  - PostgreSQL: the heading as a `-- ` line, the `CREATE`, then each other
    part after a blank line and its comment (`-- Indexes`, `-- Triggers`,
    `-- Comments`), its statements one under the other.
  - MySQL: the same, with only `-- Triggers` to add.
  - SQLite: every statement apart from the next by a blank line.
  - Every statement ends in `;`, the one it has or one added. Where its
    last line holds a `--` comment, the `;` goes on a line of its own:
    after the comment it would be part of it (MySQL keeps a trigger's
    body as it was written).
  - `Ddl::server` names the server ("PostgreSQL 17.2", "MySQL 8.4.3",
    "SQLite 3.46.0") for the terminal header.
- `Connection::ddl(&ObjectRef) -> Result<Ddl>`, matched on `Inner` with
  every engine named (no `_` arm), through `Adapter::ddl`, which has no
  default: a new engine does not compile without one.
- **PostgreSQL** (`pg/ddl.rs`), in a read-only transaction with
  `SET LOCAL search_path = ''` (so the server writes every table, type
  and function with its schema; the setting ends with the transaction):
  - `CREATE [UNLOGGED] TABLE schema.name (` columns
    (`format_type`, `COLLATE` where it is not the type's, `DEFAULT`
    (`pg_get_expr`), `GENERATED … AS IDENTITY`, `GENERATED ALWAYS AS (…)
    STORED`/`VIRTUAL`, `NOT NULL`), then `CONSTRAINT name …`
    (`pg_get_constraintdef`, the table's own only: primary key, unique,
    check, foreign key, exclusion, each kind by name), then `)`, then
    `PARTITION OF parent` + `FOR VALUES …` for a partition (with none of
    its parent's columns), `INHERITS (…)`, `PARTITION BY …`
    (`pg_get_partkeydef`) and `WITH (…)` (its storage options).
  - After it, in the same statement's text: `ALTER TABLE ONLY … ALTER
    COLUMN … SET DEFAULT`, `DROP DEFAULT` or `SET NOT NULL` for what a
    partition or a child set itself on a column its parent gives it; and
    `ALTER TABLE ONLY … ADD CONSTRAINT … NOT VALID` for a constraint that
    does not hold for the rows already there, which `CREATE TABLE` would
    make valid.
  - Views: `CREATE OR REPLACE VIEW schema.name` with its `WITH (…)`
    options (`check_option`, `security_barrier`) and `AS`
    `pg_get_viewdef(oid, true)`. Materialized views:
    `CREATE MATERIALIZED VIEW schema.name AS` … `WITH NO DATA;`.
  - Indexes that back no constraint, by name (`pg_get_indexdef`). A
    partitioned table's index reads `ON ONLY`, which would make it on that
    table alone. It is written `ON` (the words after the index's name,
    which can hold them too), and the copies of it on the partitions are
    left out, so the index is made on the table and on its partitions.
  - Triggers a person made (not internal ones, not a partition's copy of
    its parent's), by name (`pg_get_triggerdef`). A trigger that is
    disabled, or enabled for replicas only or always, is followed by the
    `ALTER TABLE … DISABLE TRIGGER` (or `ENABLE REPLICA`, `ENABLE ALWAYS`)
    that sets it so.
  - `COMMENT ON TABLE|VIEW|MATERIALIZED VIEW` and `COMMENT ON COLUMN`,
    quoted by the server's `quote_literal`.
  - A foreign table answers "not supported: the DDL of a foreign table".
  - Not included: owner, grants, row-level policies, sequences owned by
    the table (the footer says these); also an identity's sequence options
    (`START WITH`, `INCREMENT`), a tablespace, a table's access method, and
    the `OF type` of a typed table. Each table's foreign keys are in its
    own `CREATE TABLE`, so two tables that point at each other cannot be
    made from their texts as they are.
- **MySQL**: `information_schema.tables` for whether it is a view, then
  `SHOW CREATE VIEW` or `SHOW CREATE TABLE` exactly as returned, then
  `SHOW CREATE TRIGGER` for each of `information_schema.triggers` by name,
  all in one read-only transaction. MySQL lists a table's triggers only
  to a user with the TRIGGER privilege on it: for any other the text has
  none.
- **SQLite**: `sqlite_master.sql` of the object, then of its indexes and
  triggers (`tbl_name`, in any case: a trigger keeps the table's name as
  it was written in it), indexes first, each by name; `sql IS NULL`
  (automatic indexes) left out. Under the session's `query_only`.

## The app

- `backend::Command::ReadDdl { session, request, object }` and
  `Event::Ddl`: the app's own read on the backend runtime, never a
  `RunSql`, and kept in no history.
- `ObjectView::Ddl`. `ObjectTab::ddl: Fetch<Ddl>` and `ddl_read` (when it
  arrived). The tab holds it, as it holds its structure:
  - it is read when the view is first shown (`SetView`), and not again
    while the tab holds it;
  - leaving the view while it reads cancels the read (`Command::Cancel`),
    and the view reads again when shown next;
  - Reload (⌘R, `R`, the source line's Reload) reads it again where the
    view shows it, and forgets it where it does not. Reload also reads
    the rows, the count and the structure again, as it does in the other
    views;
  - Try again drops the text under the error and reads again;
  - a new session (connect again) reads it for the tab in front that
    shows the view; a read that was on its way, or was lost, on a tab
    behind is forgotten;
  - a DDL view that holds nothing and waits for nothing reads it: so a
    tab whose read was forgotten reads when it comes in front again;
  - a tab reopened in the DDL view reads it with its rows;
  - a tab's pending requests include it, so closing the tab cancels it.
- `session.json`: a kept tab says `"view": "structure"` or `"ddl"`; the
  rows say nothing; a word this version does not know reads as the rows.

## The view (`src/ui/ddl.rs`)

- The switch is a tab list named "View" with three tabs, **Rows**,
  **Structure**, **DDL** (`rows structure ddl` on Omarchy), in their order.
  The grid of the rows is named "Rows" too: Tab reads the switch's tab,
  then the grid.
- The text is a read-only multi-line field named "DDL of book_covers",
  in the SQL editor's code role and colours (`sql_text::layouter`:
  keywords, strings, comments dimmed), with a gutter of line numbers. It
  does not wrap; it scrolls both ways. It is the Grid region's stop (F6
  comes to it), and the keyboard that comes to it starts at the top.
  A field that cannot be typed into draws no caret, so a ring inside the
  text's area says it has the keyboard. Selection and ⌘C, ⇧ arrows and
  ⌘A are the field's; Reload's letter (`R`) and chord work in it.
- macOS: over the text, "Read-only · built from the catalog · read 2 s
  ago" (or "as stored in the database") and a **Reload ⌘R** link; under
  it a footer, "27 lines · PostgreSQL", PostgreSQL's "Not included:
  owner, grants, sequences owned by the table, row-level policies", and
  "Select text to copy part of it". No Add row in this view.
- Omarchy: the header's summary is "ddl · read-only · built from the
  catalog · postgresql 17.2". The key line names Reload and the sidebar,
  and write where the connection writes and review where something is
  pending, as the Structure view's line does. Where the rows' line says
  which rows it shows, this one says how many lines the text has.
- Loading, after the app's 300 ms: macOS says "Reading the definition…"
  with a spinner at the header's end (the title gives it room) and draws
  grey lines where the text will be; Omarchy draws one line, a spinner
  and "reading the definition…". Each said once to a screen reader (a
  polite live region).
- Couldn't read it: "Couldn't read the definition of salaries", the
  database's number (MySQL) or code and its words ("1142 · SHOW command
  denied …"). Where the database refused the read to the user (MySQL
  1044, 1142, 1143, 1227; PostgreSQL 42501): "The connection's user can't
  run SHOW CREATE TABLE here. Rows and Structure still work." (`SHOW
  CREATE VIEW` for a view; "can't read this part of the catalog" on
  PostgreSQL). **Try again** and **Copy error** (the code and words, with
  the database's detail and hint). Said once. No text, never part of one,
  and nothing rebuilt from Structure.

## Tests

- `crates/tabletist-db/tests/ddl.rs`, with text files under `tests/ddl/`
  (text, not pictures): `book_covers` and `in_stock` on each engine,
  with what the server varies taken out (PostgreSQL's version,
  `AUTO_INCREMENT=`, a `DEFINER`'s user); PostgreSQL's round trip (every
  Bookshop table and view run into a new database, and the catalog read
  on its own matches for columns, defaults, identity, generated columns,
  collations, constraints and whether they hold, indexes, triggers with
  their enabled state, comments, view queries, partition keys and
  bounds); the same for what a partition and a child set themselves, a
  NOT VALID check, and an index named with `ON ONLY`; names with their
  schema and doubled quotes; identity, generated, partitioned,
  partition, unlogged, exclusion, disabled trigger, materialized view
  with an index; MySQL byte for byte with `SHOW CREATE` and the triggers
  by name; SQLite without automatic indexes, in order, and with a
  trigger written on the table in another case; a user without SHOW VIEW
  gets error 1142 and no text; every engine reads `book_covers`; the read
  leaves the session's search path as it was. The tests drop the
  databases and the view they make. Listed in both integration jobs of
  `ci.yml`.
- The Bookshop fixtures gain `in_stock`, two indexes on `book_covers`,
  and on PostgreSQL and SQLite two triggers on it, made in reverse order
  of their names (the text lists them by name). On PostgreSQL one of the
  two is disabled, and there are also comments with quotes,
  `price_changes` (identity, generated), `basket_lines` (unlogged),
  `shelf_bookings` (exclusion, a column collation) and `publisher_totals`
  (materialized, with an index). MySQL's two triggers are made by its
  tests, which skip that part where the server lets the tests' user make
  none (compose.yaml lets it).
- Headless (`src/ui/ddl_tests.rs`, `src/app/startup_tests.rs`,
  `src/session.rs`): the switch's tabs in order; the read sent once, with
  nothing but `ReadDdl`; the text read-only with its numbers, and nothing
  typed, pasted or cut changes it; a second open reads nothing, Reload
  reads again, Reload elsewhere forgets; leaving and closing the tab
  cancel; a new session reads the one in front, and the one behind when
  it comes in front; the refused read's card, Copy error and Try again
  (which drops the text), Rows and Structure working on, and no
  sentence for an error that is no refusal; the long read said; the
  source line, the footer and the terminal summary per engine; no Add
  row; the terminal's key line and line count; the arrows stay off the
  sidebar; F6 comes to the text; ⇧↓ selects from the top, ⌘A ⌘C copies
  it all; `R` and ⌘R read again from the text while the rows' letters do
  nothing there; a tab reopened in the view reads its definition.
- Not testable here: which statements the server ran (the tests' user
  may not read `performance_schema`, and PostgreSQL keeps no statement
  log a session can read). The tests check what can be: the read runs on
  read-only sessions, leaves the session's settings as they were, and the
  app sends nothing but `ReadDdl` and records no History.
- A dev server this branch loaded the Bookshop into keeps the new tables,
  which point at `books`: main's fixture cannot drop `books` then. Drop
  the `bookshop` database (PostgreSQL) to go back.
