# SQL editor, slice 1: the core editor

Date: 2026-09-30. Status: approved in conversation and by spec review.

## Intent

Tabletist browses tables but cannot run SQL. This slice adds a SQL editor
tab: type SQL, run the statement at the cursor or the whole script, and read
the result in the same grid the table tabs use. The app still cannot change
table data: every run happens in a read-only transaction that is rolled back,
and scripts that would leave that transaction are refused before anything
runs.

Success: on each driver, a user opens a SQL tab, runs a query, sees its rows
with column types, sees the database's error when a statement fails, and can
cancel a slow query or let the timeout stop it; and no script typed into the
editor changes data through this session.

The promise is about data, as it is for the raw WHERE today. Side effects
outside table data that a read-only transaction allows stay possible for a
user with the privileges: PostgreSQL session advisory locks,
`pg_terminate_backend`, `dblink_exec`, `lo_export`; SQLite `ATTACH` and
connection `PRAGMA`s for the life of the session.

The designs are the "SQL editor" artboards (macOS and Omarchy) in the design
canvas Artifact. They are not copied into the repository.

## The SQL editor as a whole

The designs describe more than this slice. It is split into sub-projects,
each with its own spec, plan and pull request:

1. **Core editor** (this spec).
2. Autocomplete: keywords, tables and views, columns.
3. Explain and Format.
4. History, Copy and Export.
5. Vim normal mode in the Omarchy editor.

## Decisions

| Question | Decision |
|---|---|
| Scope | Core editor only; slices 2 to 5 follow separately. |
| Query text | In memory only. Closing a SQL tab never asks; no unsaved dot. |
| Run all | One read-only transaction, statements in order, stop at the first error. Results shows the last statement that returned rows; Messages lists every statement. |
| Read-only guard | Refuse transaction and session-mode statements before running; on PostgreSQL take the snapshot first; check the transaction is still read-only before rolling back; reset the MySQL session after every script. |
| Editor widget | egui `TextEdit` in code mode with our own layouter and a gutter; the SQL tokenizer lives in `tabletist-db`. |
| Tab model | `Workspace.objects` becomes `tabs: Vec<Tab>`, `enum Tab { Object(ObjectTab), Sql(SqlTab) }`. |
| Shortcuts | `Mod+T` opens a SQL editor. New connection tab moves to `Mod+O` on every platform. |

Rejected: the `egui_code_editor` crate (a new dependency with its own colour
themes and fonts, against the rule that views draw text only through
`TextRole`s); a custom text widget (cursor, selection, IME and undo from
scratch); a separate `sql_tabs` list beside `objects` (every piece of tab
strip logic would handle two lists); `Ctrl+Shift+C` for connections, as in
the Omarchy design (the egui-winit fork turns it into a Copy event, and it is
already the copy-row shortcut); an allow-list of statement kinds (blocks
harmless reads for little gain over the refusal list plus the check).

## Out of scope

Autocomplete, Explain, Format, History, Copy, Export, vim modes, saving or
restoring query text, the row panel for SQL results, several result sets side
by side, and running anything outside a read-only transaction.

## Running queries (`tabletist-db`)

### Tokenizer and splitter

A new module `sql` in `tabletist-db`, with no UI dependencies:

- `sql::tokenize(dialect, text) -> Vec<Token>`. A token has a kind (keyword,
  identifier, quoted identifier, string, number, comment, operator,
  punctuation, whitespace) and a byte range. The tokenizer never fails: text
  it cannot classify becomes punctuation, and an unterminated string or
  comment runs to the end of the text.
- All dialects: `''` escapes inside strings, `/* */` comments.
- PostgreSQL: `--` comments; `"quoted"` identifiers; `$tag$ ... $tag$`
  bodies; `E'...'` strings with backslash escapes; nested `/* */` comments.
- MySQL: `--` starts a comment only when followed by whitespace or a control
  character (`SELECT 1--1` is an expression); `#` comments; backtick
  identifiers; `"..."` is a string, as in MySQL's default `sql_mode`
  (`ANSI_QUOTES` is not detected); backslash escapes in strings (the session
  turns `NO_BACKSLASH_ESCAPES` off at connect). A `/*! ... */` executable
  comment is its own token kind (MySQL runs its contents).
- SQLite: `--` comments; `"quoted"`, `[bracket]` and backtick identifiers.
- `sql::statements(dialect, text) -> Vec<Statement>`, splitting on `;`
  tokens (a `;` inside a string, comment or dollar body is not a token of its
  own). `Statement { range, first_line, text }`: `range` is the byte range in
  the editor text without surrounding whitespace, `text` is that range
  without the `;`, and `first_line` (1-based) is the line of the statement's
  first token that is not a comment, so "Line N" points at the SQL, not at a
  comment above it. Pieces holding only whitespace and comments are dropped.
- `sql::statement_at(statements, cursor) -> Option<&Statement>`: the one
  whose range (extended to its `;`) holds the cursor; otherwise the nearest
  one ending before the cursor; otherwise the first after it.
- `sql::leading_keywords(dialect, statement)`: the first two or three
  keyword tokens, skipping comments and whitespace, upper-cased. Used by the
  guard.

The keyword list for highlighting is one list per dialect, kept short
(reserved words and common clauses). Highlighting, splitting and the guard
use the same tokens, so what looks like a string is what the splitter and the
guard treat as one.

### The read-only guard

Three layers, all in `tabletist-db`, so no caller can skip them.

1. **Refusal before running.** `run_script` checks every statement first and
   runs nothing if any is refused. The whole script fails with
   `Error::Refused { line, reason }`, shown as "Line 4: Tabletist runs every
   query in a read-only transaction; COMMIT is not allowed". Refused, by
   leading keywords:
   - `BEGIN`, `START` (TRANSACTION), `COMMIT`, `END`, `ROLLBACK`, `ABORT`,
     `SAVEPOINT`, `RELEASE`, `PREPARE TRANSACTION`, `COMMIT PREPARED`,
     `ROLLBACK PREPARED`, `XA`;
   - `SET TRANSACTION`, `SET SESSION CHARACTERISTICS`,
     `SET SESSION TRANSACTION`, `SET GLOBAL TRANSACTION`, and any `SET` whose
     statement names `read_only` (`transaction_read_only`, `tx_read_only`,
     `default_transaction_read_only`) or `autocommit`, matched on the name
     with its quotes or backticks removed, including `SET @@...` forms;
   - `RESET ALL`, `RESET` of the names above, `DISCARD ALL`;
   - PostgreSQL only: `COPY` (`COPY ... TO STDOUT` answers with a copy
     stream the simple-query protocol cannot read, and `TO 'file'` or
     `TO PROGRAM` writes on the server; `COPY ... FROM` fails as read-only
     anyway);
   - MySQL only: `LOCK TABLES`, `UNLOCK TABLES`, `CALL` (a MySQL procedure
     may commit and change the session's transaction mode); account and
     server statements that commit implicitly and are not table writes:
     `CREATE USER`, `ALTER USER`, `DROP USER`, `RENAME USER`, `CREATE ROLE`,
     `DROP ROLE`, `GRANT`, `REVOKE`, `SET PASSWORD`, `SET DEFAULT ROLE`,
     `FLUSH`, `INSTALL`, `UNINSTALL`; any statement with the tokens
     `INTO OUTFILE` or `INTO DUMPFILE` (they write files on the server); and
     any statement holding a `/*! ... */` executable comment.
   The list is matched on tokens, never on raw text, so `SELECT 'COMMIT'`
   and a column named `end_date` are fine.
2. **Snapshot first (PostgreSQL).** After `BEGIN READ ONLY`, the session runs
   `SELECT 1` before any user statement. PostgreSQL refuses to make a
   transaction read-write once a snapshot is taken, so even a statement the
   list missed cannot flip it. MySQL's session is already read-only
   (`SET SESSION TRANSACTION READ ONLY` at connect), and the list keeps it
   so. SQLite is opened read-only by the driver.
3. **Check before rolling back.** After the last statement, when the
   transaction is still usable, PostgreSQL asks `SHOW transaction_read_only`
   and MySQL `SELECT @@session.transaction_read_only` (falling back to
   `@@session.tx_read_only` on "unknown system variable", for MariaDB before
   11.1, as `server_identity` does). Only an explicit `off` (or `0`) is
   `Error::LeftReadOnly`. On PostgreSQL the check is skipped when the last
   statement ended in `Error` or `Cancelled`: the transaction is then
   aborted, cannot write, and `ROLLBACK` ends it.

Cleanup after every script, on every path:

- PostgreSQL: `ROLLBACK`. Session settings changed by the script are
  transactional and roll back with it.
- MySQL: `ROLLBACK`, then `Conn::reset()` (`COM_RESET_CONNECTION`, which
  keeps the connection id the cancel uses), then the connect-time
  `SET SESSION TRANSACTION READ ONLY` and `sql_mode` statements again. MySQL
  session state is not transactional; the reset makes sure nothing a script
  set (`sql_select_limit` below, a user's `SET time_zone`, `SET NAMES`)
  reaches table browsing or the next run. `reset()` answering `false` (the
  server predates `COM_RESET_CONNECTION`: MySQL 5.7.2, MariaDB 10.2.3 or
  older) is a cleanup failure.
- SQLite: `ROLLBACK`. SQLite has no session read-only mode to lose, but a
  script's `PRAGMA`s and `ATTACH`es last for the session (see Intent).

A cancel is meant for a user statement, but it can land on the queries
around them. One that lands on the opening queries (`BEGIN`, `SELECT 1`,
MySQL's `SET sql_select_limit`) ends the run with no statement results and
reports it as cancelled: `Ok` with no results, and the backend's
`CancelReason`. One that lands on the check or on the cleanup is ignored for
the outcome, and the interrupted step runs once more (the backend sends at
most one cancel per press or timeout, so the retry is not cancelled again);
a MySQL session is never left read-write after a reset because a `SET` was
interrupted. If `LeftReadOnly` is found, or a cleanup step fails (including
a retried step that fails again), `run_script` returns the error and the backend closes the session, so its
state cannot reach later queries; the error counts as a lost connection
(`is_connection_lost`), so the tab shows the existing reconnect banner.

### `Connection::run_script`

```rust
pub async fn run_script(&self, statements: &[Statement], limit: u32) -> Result<ScriptOutcome>;

pub struct ScriptOutcome {
    /// One per statement that started, in order. After an Error or a
    /// Cancelled outcome, no further statement runs.
    pub results: Vec<StatementResult>,
}
pub struct StatementResult {
    pub elapsed: Duration,
    pub outcome: StatementOutcome,
}
pub enum StatementOutcome {
    Rows { columns: Vec<ColumnMeta>, rows: Vec<Vec<Value>>, truncated: bool },
    Done { affected: Option<u64> },
    Error { message: String, position: Option<usize> },
    /// The session's cancel fired while this statement ran (user or
    /// timeout; the backend knows which). Any rows it had read are dropped.
    Cancelled,
}
```

- All statements run in one read-only transaction that is rolled back when
  the script ends, on every path: success, statement error, cancel. The
  outer `Err` is only for the session itself (connection lost, transaction
  could not start), for `Refused`, for `LeftReadOnly` and for a cleanup
  failure. A cancel that lands before the first statement is `Ok` with no
  results; the app shows it as "Cancelled" with nothing in Messages.
- `position` is PostgreSQL's 1-based character position within the
  statement's text (`ErrorPosition::Original`); internal positions, which
  point into other text, are ignored. The app maps it to a line and column
  in the editor.
- Statements that completed before an error or a cancel keep their results.
- **Limit:** at most `limit` rows are kept; `truncated` says more existed.
  Nothing is cancelled to stop reading; each driver asks for only
  `limit + 1` rows:
  - **PostgreSQL.** The transaction is managed on the client by hand
    (`BEGIN READ ONLY`, `SELECT 1`, statements, the check, `ROLLBACK`),
    because `tokio_postgres::Transaction` has no streaming simple query.
    Every statement is prepared first; this yields the column types and
    refuses a second statement hidden in one piece. A prepare error is the
    statement's `Error` outcome, never a reason to run the text another way.
    Then, through the simple-query protocol (every value as text, turned
    into `Value`s with `value_from_pg_text` as the grid does now):
    - A statement that prepared with result columns and whose first token
      is `SELECT`, `VALUES`, `TABLE`, `WITH` or `(` runs as a cursor, each
      step its own `simple_query` call: `DECLARE tabletist_sql NO SCROLL
      CURSOR FOR <text>` followed by a newline (so a trailing `--` comment
      ends there), then `FETCH <limit + 1> FROM tabletist_sql`, then
      `CLOSE tabletist_sql`. An error position from `DECLARE` has the
      prefix's length subtracted, so it points into the user's text. A
      data-modifying `WITH` is refused by `DECLARE`; its message stands.
    - Any other statement with result columns (`SHOW`, `EXPLAIN`) runs
      through `simple_query_raw`, keeping `limit + 1` rows and discarding
      the rest as they stream; the timeout bounds the rest.
    - A statement that prepared without columns runs through `simple_query`
      and reports the affected count when there is one.
  - **MySQL.** Every statement runs through the prepared-statement protocol
    (`exec_iter`, as `fetch_rows` does), which cannot hold two statements;
    MySQL's "not supported in the prepared statement protocol" error is the
    statement's `Error` outcome. The script sets `sql_select_limit` to
    `limit + 1` after starting the transaction, so the server stops
    producing rows; the cleanup's reset puts it back. Column types come from
    the result metadata, as in `fetch_rows`.
  - **SQLite.** Stepping stops after `limit + 1` rows. The script runs in
    `BEGIN DEFERRED ... ROLLBACK` for one consistent snapshot.

### `Connection::server_version`

`server_version() -> Result<String>` for the footer: `PostgreSQL 17.2` from
`SHOW server_version` cut at the first space (dropping distribution
suffixes), `MySQL 8.4.3` or `MariaDB 10.11.6` from `SELECT VERSION()` cut at
the first `-` (the name is MariaDB when the full string contains
`MariaDB`), `SQLite 3.46.0` from `sqlite_version()`. Asked once when the session
connects and kept on the workspace.

### Backend

- `Command::RunSql { request, session, statements, limit, timeout }` and an
  `Event::SqlRan { request, result: Result<ScriptOutcome>, cancel: Option<CancelReason> }`,
  where `CancelReason` is `User` or `Timeout(Duration)`.
- The run is registered like other requests, so the existing Cancel action
  (`Mod+.`, the Cancel button) cancels it by request through the session's
  `CancelHandle` and the `Running.cancels` path, and a late cancel cannot
  reach the next command.
- The timeout does not drop the running future (that would leave the server
  running the query). When it elapses, the backend fires the same cancel,
  records `CancelReason::Timeout`, and waits for `run_script` to return; the
  interrupted statement then reports `Cancelled`, and the transaction is
  rolled back.
- A new run in the same tab cancels the one still running (reason `User`).
  Results for a closed tab or a replaced run are dropped by `RequestId`.
- The SQL text is never logged: only the request id, statement count and
  timings.

### Settings

`Settings` gains `sql_limit: u32` (default 1,000; choices 100, 1,000,
10,000) and `sql_timeout_secs: Option<u32>` (default 30; choices 10, 30, 60,
300, none). Both are `#[serde(default)]`, so older settings files load
unchanged. A new SQL tab starts from them; changing a menu in a tab updates
that tab and the setting.

## Model

```rust
pub enum Tab { Object(ObjectTab), Sql(SqlTab) }

pub struct SqlTab {
    pub id: TabId,          // shared id space with object tabs
    pub number: u32,        // "Query 3"
    pub text: String,
    pub limit: u32,
    pub timeout: Option<Duration>,
    pub run: Fetch<SqlRun>, // the last run, or the one running
    pub pane: ResultPane,   // Results or Messages
    pub split: f32,         // editor share of the height, 0.45 by default
    pub selection: Option<CellPos>,
    pub focus_editor: bool, // focus the editor on the next frame
}

pub struct SqlRun {
    pub statements: Vec<Statement>, // as split when the run started
    pub outcome: ScriptOutcome,
    pub cancel: Option<CancelReason>,
}
```

A run that failed as a whole (`Refused`, `LeftReadOnly`, connection lost)
is the `Fetch`'s error; `SqlRun` exists only when statements ran.

`ObjectTabId` becomes `TabId`, `active_object` becomes `active_tab`, and
`Workspace` gains helpers (`active_object()`, `object_tab(id)`,
`sql_tab(id)`) so call sites that only care about table tabs stay short.
`Workspace` also gains `next_query: u32` and `server_version: Option<String>`.
The rename is a mechanical first step, separate from the feature.

New actions: `NewSqlTab(ConnTabId)`, `RunSql { tab, id, all: bool }`,
`SetSqlLimit`, `SetSqlTimeout`, `SetResultPane`, `SetSqlSplit`.
`CancelQuery` also cancels a SQL tab's run. `MoveSelection` works on a SQL
tab's result grid through `TabId`.

## UI

### Opening

- `Mod+T` opens a new SQL tab in the active connection tab's workspace
  (connected or not; running needs a connection) and focuses its editor. On
  the connection picker it does nothing. On macOS an "SQL Editor" button with
  its shortcut sits at the bottom of the sidebar; on Omarchy a `+ sql` button
  with its hint ends the tab strip.
- New connection tab moves from `Mod+T` to `Mod+O` on every platform (the
  Omarchy design's `ctrl+shift+c` hint becomes `ctrl+o`). The help dialog,
  tooltips and hints follow.
- Tabs are named "Query 1", "Query 2", ... per connection, and sit in the
  same strip as table tabs with a code icon. They are always pinned (never a
  preview tab). Closing one never asks.
- The row panel and its toggle do not apply to SQL tabs in this slice.

### Shortcuts on a SQL tab

- `Mod+Return` runs the statement at the cursor, `Mod+Shift+Return` runs
  all, also while typing. `Mod+.` cancels.
- `Mod+W`, `Mod+Shift+[ / ]`, `Mod+1..9`, `Mod+B`, `Mod+P`, `?` work as on
  any tab.
- `Mod+R`, `Mod+F`, `Mod+Alt+Left / Right`, Space and `Mod+Shift+R` do
  nothing on a SQL tab (no refresh, filter, paging or row panel here).
- While the editor has focus, keys go to it (Tab indents, arrows move the
  cursor). Esc leaves the editor; then arrows, Page Up/Down and Home/End move
  in the result grid, and the Omarchy vim letters that move the selection
  work as in a table tab.

### Toolbar

- macOS: Run (with `Cmd+Return`), Run all (`Shift+Cmd+Return`); on the right
  a "Read-only transaction" badge whose tooltip explains it, then
  "Limit 1,000" and "Timeout 30 s" menus.
- Omarchy: the tab title, a muted `read-only transaction · limit 1000 ·
  timeout 30s` whose limit and timeout parts open the same menus, then
  `run ctrl+enter` and `run all ctrl+shift+enter`.
- Explain and Format are absent until slice 3, not disabled.

### Editor

- A multi-line `TextEdit` in code mode, drawn in the code `TextRole`, with a
  line-number gutter: absolute numbers on macOS and Windows, relative numbers
  with the current line absolute on Omarchy.
- Highlighting through a layouter over `sql::tokenize`, in theme colours:
  keywords, strings, numbers, comments, identifiers.
- The statement that Run would execute carries a faint background band.

### Results

- A draggable horizontal splitter between the editor and the results, 45 to
  55 by default, kept per tab.
- The results header: tabs "Results (n)" and "Messages", then
  "Statement at line 2 · 14 ms".
- Results shows the existing grid (`ui::grid`), column name over type, with
  selection and scrolling as in a table tab. When the limit cut rows off, a
  note reads "First 1,000 rows (limit reached)".
- Messages lists each statement: its line, then rows returned or affected
  and the time, or the error, or "Cancelled". Statements that did not start
  read "not run".
- States: before any run, a hint "Run a statement with Cmd+Return"; while
  running, a spinner, the elapsed time and a Cancel button; a statement with
  no result set shows "Statement ran · no rows returned"; on an error or a
  refusal the Messages tab opens by itself and the gutter marks the line in
  the error colour.

### Footer

- macOS: `5 rows · 14 ms`, `Read-only transaction · rolled back`, then on
  the right `Ln 12, Col 21` and the server version.
- Omarchy: a mode line of key hints (run, run all, cancel), then
  `ln 12:21 · 5 rows · 14 ms · rolled back` on the right. The vim mode
  indicator waits for slice 5.

## Errors and edge cases

- A statement's error shows the database's message prefixed with its line:
  "Line 12: relation "books_x" does not exist". With a position, the text
  says "line 12, col 15" and the gutter marks that line.
- Writes fail through the database's own read-only error (PostgreSQL and
  MySQL read-only transactions; SQLite is opened read-only); escaping the
  transaction is refused by the guard.
- A timeout reads "Cancelled after 30 s (timeout)", a user cancel
  "Cancelled". A `Cancelled` outcome without a reason (a user-set
  `statement_timeout`, say) also reads "Cancelled".
- Session settings a script changes (`SET search_path`, `SET time_zone`)
  last only for that run: PostgreSQL rolls them back, MySQL resets the
  session. SQLite `PRAGMA`s and `ATTACH`es last for the session. The cancelled statement's rows are dropped, so a partial
  result never looks complete; statements that finished before it keep
  theirs in Messages, and the last of them that returned rows stays in
  Results.
- A lost connection shows the existing reconnect banner; the tab keeps its
  text.
- Run on an empty or comment-only editor does nothing and shows no error.
- Statements that cannot run in a transaction (PostgreSQL `VACUUM`,
  `CREATE INDEX CONCURRENTLY`) fail with the server's own error.

## Testing

- `tabletist-db` unit tests for the tokenizer and splitter in all three
  dialects: `;` inside strings, comments, `$$` and `$tag$` bodies,
  backticks, `E'\''`; nested comments; MySQL `--` with and without a
  following space; `/*! */`; unterminated strings; comment-only pieces; a
  last statement without `;`; `first_line` after a leading comment;
  `statement_at` before, inside, between and after statements.
- Guard unit tests: every refused form is refused (including `SET @@...`,
  quoted and backticked names, lower case, leading comments, `COPY`,
  `INTO OUTFILE`), and look-alikes are not (`SELECT 'COMMIT'`,
  `SELECT end_date`, `SET search_path`, `SELECT "into outfile"` on MySQL).
- `run_script` tests on SQLite (always) and on PostgreSQL and MySQL when
  their test URLs are set: rows with column types; truncation at the limit
  on a table larger than the limit; a statement without a result set;
  stopping at the first error with earlier results kept; a write refused as
  read-only; the PostgreSQL error position; cancelling a long query
  (`pg_sleep`, `SLEEP`, a recursive CTE on SQLite) with earlier results
  kept; a statement error on PostgreSQL leaves the session connected (the
  check is skipped, not failed); `(SELECT ...) UNION (SELECT ...)` and a
  statement ending in a `--` comment run through the cursor; a `DECLARE`
  error position points into the user's text.
- MySQL session tests: after a run, and after one that failed or was
  cancelled, `sql_select_limit`, `time_zone` and the read-only mode are
  back to the connect-time values and `fetch_rows` and `describe` return
  every row; the same statement text run twice with different limits
  honours each (mysql_async caches prepared statements); a cancel that
  interrupts the re-applied `SET SESSION TRANSACTION READ ONLY` after the
  reset leaves the session read-only or closed, never read-write; the read-only
  check works on MariaDB when a MariaDB test server is available.
- Bypass tests on PostgreSQL and MySQL, each asserting a probe table is
  unchanged afterwards: `SET TRANSACTION READ WRITE; INSERT ...`;
  `ROLLBACK; SET default_transaction_read_only = off; INSERT ...`;
  `SET "default_transaction_read_only" = off`;
  `COMMIT; SET SESSION TRANSACTION READ WRITE; INSERT ...`; a MySQL `CALL`;
  a `/*!` comment; MySQL `CREATE USER` and `GRANT` refused.
- Headless UI tests through `src/testing.rs`: `Mod+T` opens "Query 1" with
  the editor focused; `Mod+Return` sends `RunSql` with only the statement at
  the cursor and `Mod+Shift+Return` with all; a result fills the grid's
  AccessKit table; an error or a refusal switches to Messages; a late result
  for a closed tab is dropped; the limit and timeout menus change the next
  request and the settings; `Mod+O` opens a connection tab; `Mod+R` and
  `Mod+F` do nothing on a SQL tab.
- Settings: an older settings file without the new keys loads with the
  defaults.
- No design or pixel conformance checks. Screenshots for review use the
  Bookshop demo data and stay local.
