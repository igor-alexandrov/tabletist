# SQL editor, slice 1: the core editor

Date: 2026-09-30. Status: implemented. This spec describes the slice as
built; where the code and the first draft differed, the text follows the
code. Since 2026-10-01 the row panel shows a selected result row (see Results
and Shortcuts on a SQL tab).

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
user with the privileges: PostgreSQL session advisory locks (held until
the run ends), `pg_terminate_backend`, `dblink_exec`, `lo_export`; MySQL `KILL`; SQLite
`ATTACH` and connection `PRAGMA`s for the life of the session (apart from
the few settings every run puts back, see the cleanup below).

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
| Tab model | `Workspace.objects` becomes `tabs: Vec<Tab>`, `enum Tab { Object(Box<ObjectTab>), Sql(Box<SqlTab>) }`. |
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
restoring query text, several result sets side by side, and running anything
outside a read-only transaction.

## Running queries (`tabletist-db`)

### Tokenizer and splitter

A new module `sql` in `tabletist-db`, with no UI dependencies:

- `sql::tokenize(dialect, text) -> Vec<Token>`. A token has a kind (keyword,
  identifier, quoted identifier, string, number, comment, executable
  comment, operator, punctuation, semicolon, whitespace) and a byte range.
  The tokenizer never fails: text it cannot classify becomes punctuation,
  and an unterminated string or comment runs to the end of the text.
- All dialects: `''` escapes inside strings, `/* */` comments.
- PostgreSQL: `--` comments, which end at a newline or a carriage return;
  `"quoted"` identifiers; `$tag$ ... $tag$` bodies (tags may hold non-ASCII
  letters); `E'...'` strings with backslash escapes, continued across a
  newline by another `'` as PostgreSQL does; nested `/* */` comments.
- Vertical tab is whitespace in every dialect.
- MySQL: `--` starts a comment only when followed by whitespace or a control
  character (`SELECT 1--1` is an expression); `#` comments; backtick
  identifiers; `"..."` is a string and backslashes escape (the session
  turns `ANSI_QUOTES` and `NO_BACKSLASH_ESCAPES` off at connect, and the
  guard refuses changing `sql_mode`, so the server lexes as we do).
  `/*! ... */` and MariaDB's `/*M! ... */` executable comments are their own
  token kind (the server runs their contents).
- SQLite: `--` comments; `"quoted"`, `[bracket]` and backtick identifiers.
- `sql::statements(dialect, text) -> Vec<Statement>`, splitting on `;`
  tokens (a `;` inside a string, comment or dollar body is not a token of its
  own). `Statement { range, end, start_line, start_column, first_line, text }`:
  `range` is the byte range in the editor text without surrounding
  whitespace or the `;`, `end` is the offset just past the `;` (or
  `range.end` without one), `text` is the script's text in `range`,
  `start_line` (1-based) and `start_column` say where `range` starts, and
  `first_line` (1-based) is the line of the statement's first token that is
  not a comment, so "Line N" points at the SQL, not at a comment above it.
  `Statement::line_col(position)` turns a 1-based character position in
  `text` into a line and column of the script. Pieces holding only
  whitespace and comments are dropped. `sql::statements_from(script, &tokens)`
  splits a script that is already tokenized, for the editor, which colours
  the same tokens.
- `sql::statement_at(statements, cursor) -> Option<&Statement>`: the one
  whose range (extended to its `;`) holds the cursor; otherwise the nearest
  one ending before the cursor; otherwise the first after it.
- `sql::words(dialect, statement)`: every keyword and name of the statement
  in order, quotes removed, upper-cased; strings, numbers, operators,
  punctuation and comments are skipped.
- `sql::refusal(dialect, statement) -> Option<String>`: the guard's check.
  It names what is refused, for the message, or `None` when the statement
  may run.
- `sql::is_keyword(dialect, word)`: whether a word is highlighted as a
  keyword.

The keyword list for highlighting is one list shared by every dialect, kept
short (reserved words and common clauses), plus a few words for each dialect.
Highlighting, splitting and the guard use the same tokens, so what looks like
a string is what the splitter and the guard treat as one.

### The read-only guard

Four layers, all in `tabletist-db`, so no caller can skip them.

1. **Refusal before running.** `run_script` checks every statement first and
   runs nothing if any is refused. The whole script fails with
   `Error::Refused { line, what }`, shown as "line 4: Tabletist runs every
   query in a read-only transaction, so COMMIT is not allowed". Refused:
   - `BEGIN`, `START` (TRANSACTION), `COMMIT`, `END`, `ROLLBACK`, `ABORT`,
     `SAVEPOINT`, `RELEASE`, `PREPARE TRANSACTION`, `COMMIT PREPARED`,
     `ROLLBACK PREPARED`;
   - `SET TRANSACTION`, `SET SESSION CHARACTERISTICS`,
     `SET SESSION TRANSACTION`, `SET GLOBAL TRANSACTION`, and any `SET` whose
     statement names `read_only` (`transaction_read_only`, `tx_read_only`,
     `default_transaction_read_only`), `autocommit` or `completion_type`
     (it can make our `ROLLBACK` end the session), matched on the name
     with its quotes or backticks removed, including `SET @@...` forms;
   - settings that change how later statements are lexed: any `SET` or
     `RESET` naming `sql_mode`, `standard_conforming_strings`,
     `client_encoding`, a `character_set_*` variable, and `SET NAMES`,
     `SET CHARACTER SET`, `SET CHARSET` (a client character set such as GBK
     can swallow a backslash the tokenizer saw), checked in every
     assignment of a `SET` that has several (`SET @a = 1, NAMES gbk`); on
     PostgreSQL also any statement with a `U&"..."` name (escapes can
     spell any name, `set_config` included) and any statement calling
     `set_config` (best effort: a function can still call it, but its
     changes roll back and cannot split a statement, since every statement
     is prepared alone);
   - `PREPARE`, `EXECUTE` and `DEALLOCATE` in both PostgreSQL and MySQL
     (and MariaDB's `EXECUTE IMMEDIATE`): the SQL they run is a string the
     guard cannot read, and prepared statements outlive the rollback;
   - `RESET ALL`, `RESET` of the names above, `DISCARD ALL`;
   - PostgreSQL only: `COPY` (`COPY ... TO STDOUT` answers with a copy
     stream the simple-query protocol cannot read, and `TO 'file'` or
     `TO PROGRAM` writes on the server; `COPY ... FROM` fails as read-only
     anyway);
   - MySQL only: `XA`, `LOCK TABLES`, `UNLOCK TABLES`, `CALL` (a MySQL
     procedure may commit and change the session's transaction mode);
     account and server statements that commit implicitly and are not table
     writes:
     `CREATE [OR REPLACE] USER`, `ALTER USER`, `DROP USER`, `RENAME USER`,
     `CREATE [OR REPLACE] ROLE`, `DROP ROLE`, `GRANT`, `REVOKE`,
     `SET PASSWORD`, `SET DEFAULT ROLE`, `FLUSH`, `INSTALL`, `UNINSTALL`;
     server state that is neither rolled back nor reset: any `SET` with
     `GLOBAL`, `PERSIST` or `PERSIST_ONLY` (also as `@@global.`,
     `@@persist.`), any `RESET` (`RESET MASTER`, `RESET REPLICA`,
     `RESET PERSIST`), `PURGE`, `CHANGE` (`CHANGE MASTER`,
     `CHANGE REPLICATION SOURCE`), `STOP` (`STOP REPLICA`), `SHUTDOWN`,
     `RESTART`, `CLONE`, `ALTER INSTANCE`, `BACKUP`, `DROP PREPARE`; `USE`
     (the default database is session state the reset may not restore);
     MariaDB's `SET STATEMENT ... FOR <statement>`, which wraps any
     statement behind a leading `SET`; any statement with the tokens
     `INTO OUTFILE` or `INTO DUMPFILE` (they write files on the server); and
     any statement holding a `/*! ... */` or `/*M! ... */` executable
     comment.
   The list is matched on tokens, never on raw text, so `SELECT 'COMMIT'`
   and a column named `end_date` are fine. It errs toward refusing: a `SET`
   naming a guarded word anywhere is refused.

   One statement at a time is a layer of its own: PostgreSQL prepares every
   statement before running it, and MySQL runs statements only through the
   prepared protocol; both refuse text holding a second statement, so a
   setting changed by one statement can never split another. mysql_async
   refuses `LOAD DATA LOCAL INFILE` unless a handler is configured, and none
   is.
2. **Snapshot first (PostgreSQL).** After `BEGIN READ ONLY`, the session runs
   `SELECT 1` before any user statement. PostgreSQL refuses to make a
   transaction read-write once a snapshot is taken, so even a statement the
   list missed cannot flip it. MySQL's session is already read-only
   (`SET SESSION TRANSACTION READ ONLY` at connect), and the list keeps it
   so. SQLite is opened read-only by the driver.
3. **Check before every statement.** So that a statement the list missed
   can end the transaction but never be followed by a write, PostgreSQL and
   MySQL confirm the session is still where it started before running the
   next statement (SQLite is opened read-only and has no such mode to leave,
   so it needs no check):
   PostgreSQL keeps one savepoint from the start of the run and swaps it
   (`RELEASE tabletist_guard; SAVEPOINT tabletist_guard`): that fails
   outside a transaction block and in a transaction the script chained to
   (`COMMIT AND CHAIN`), keeps every statement inside a subtransaction
   (where read-write mode cannot be set), and never nests deeper than one.
   MySQL reads `@@session.transaction_read_only` (with `LIMIT 1`, since a
   script can set `sql_select_limit` to 0) and the server's transaction
   status flags from the answer. MySQL commits before it refuses DDL, so a
   session found outside a transaction but still read-only gets a new
   `START TRANSACTION READ ONLY` and the run goes on; a transaction not
   marked read-only (`SET TRANSACTION READ WRITE` affects only the next
   transaction and does not show in the variable) means the script left. A
   server that does not mark its read-only transactions at all (every
   supported MySQL and MariaDB does) cannot be checked, so the run is not
   started.
   A failed check ends the run with `Error::LeftReadOnly` before the
   statement runs.
   After the last statement the same check runs once more, so on
   PostgreSQL a statement that ended the transaction in last position closes
   the session rather than leaving a committed setting behind. On MySQL a
   session merely found outside a transaction is not closed: nothing there
   is transactional, and the reset below wipes its state.
4. **Check before rolling back.** After the last statement, when the
   transaction is still usable, PostgreSQL asks `SHOW transaction_read_only`
   and MySQL `SELECT @@session.transaction_read_only` (falling back to
   `@@session.tx_read_only` on "unknown system variable", for MariaDB before
   11.1, as `server_identity` does). Only an explicit `off` (or `0`) is
   `Error::LeftReadOnly`. On PostgreSQL the check is skipped when the
   transaction is known to be aborted (a statement failed or was cancelled
   inside it): it cannot write, and `ROLLBACK` ends it. The driver tracks
   that state itself (open, aborted or left) rather than reading it off the
   last result, so a run stopped between statements is still checked.

Cleanup after every script, on every path:

- PostgreSQL: `ROLLBACK`, then `SELECT pg_advisory_unlock_all()`. Session
  settings changed by the script are transactional and roll back with it;
  session advisory locks are not, so they are released.
- MySQL: `ROLLBACK`, then `Conn::reset()` (`COM_RESET_CONNECTION`, which
  keeps the connection id the cancel uses), then the connect-time session
  statements again, read-only first: `SET SESSION TRANSACTION READ ONLY`,
  then `SET NAMES utf8mb4 COLLATE utf8mb4_general_ci` (the reset drops the
  handshake's character set, which the tokenizer relies on), then the
  `sql_mode` without `NO_BACKSLASH_ESCAPES`, `ANSI_QUOTES` and the
  combination modes that imply it (`ANSI` and the like). MySQL
  session state is not transactional; the reset makes sure nothing a script
  set (`sql_select_limit` below, a user's `SET time_zone`, `SET NAMES`)
  reaches table browsing or the next run. `reset()` answering `false` (the
  server predates `COM_RESET_CONNECTION`: MySQL 5.7.2, MariaDB 10.2.3 or
  older) is a cleanup failure.
- SQLite: `ROLLBACK` when a transaction is still open, then the
  connect-time settings again: the busy timeout, `PRAGMA query_only = ON`,
  `PRAGMA trusted_schema = OFF` and `PRAGMA case_sensitive_like = OFF`, so a
  script cannot leave the session writable or change how the filters
  compare. A script's other `PRAGMA`s and its `ATTACH`es last for the
  session (see Intent).

A cancel is meant for a user statement, but it can land on the queries
around them. One that lands on the opening queries (`BEGIN`, `SELECT 1`,
MySQL's `SET sql_select_limit`) ends the run with no statement results and
reports it as cancelled: `Ok` with no results and `stopped` set, and the
backend's `CancelReason`. One that lands on the check before a statement
counts as a stop between statements: that statement is `Cancelled` and the
run ends. In every driver a run that was stopped once it had begun ends in
a `Cancelled` result for the statement it was stopped in or before.
One that lands on the checks after the last statement or on the cleanup is
ignored for the outcome, and the interrupted step runs once more (the driver
marks the run as finishing first, so the backend stops repeating its
cancel); a MySQL session is never left read-write after a reset because a
`SET` was interrupted. On PostgreSQL a cancel aborts the transaction; the
driver then asks the server once more where it stands: still inside (a
failed transaction, which cannot write) means a clean `ROLLBACK`, outside
means `LeftReadOnly`. A `BEGIN` that fails for any reason but a cancel
closes the session, since it would fail the same way on every later run.
If `LeftReadOnly` is found, or a cleanup
step fails (including a retried step that fails again), `run_script`
returns the error and the backend closes the session, so its
state cannot reach later queries; the error counts as a lost connection
(`is_connection_lost`), so the tab shows the existing reconnect banner.

### `Connection::run_script`

```rust
pub async fn run_script(
    &self,
    statements: &[Statement],
    limit: u32,
    stop: &StopFlag,
) -> Result<ScriptOutcome>;

pub struct ScriptOutcome {
    /// One per statement that started, in order. After an Error or a
    /// Cancelled outcome, no further statement runs.
    pub results: Vec<StatementResult>,
    /// A stop or cancel ended the run.
    pub stopped: bool,
}
pub struct StatementResult {
    pub elapsed: Duration,
    pub outcome: StatementOutcome,
}
pub enum StatementOutcome {
    Rows { columns: Vec<ColumnMeta>, rows: Vec<Vec<Value>>, truncated: bool },
    Done { affected: Option<u64> },
    Error { error: Error, position: Option<usize> },
    /// Stopped while this statement ran, or before it began (user or
    /// timeout; the backend knows which). Any rows it had read are dropped.
    Cancelled,
}
```

- `StopFlag` is how a run is told to stop. The backend sets it (`stop`,
  which says whether this call was the first) together with the session's
  cancel; the drivers read it between statements (`is_stopped`), and SQLite
  also while a statement runs, so a cancel that arrives before its
  statement still ends the run. A driver calls `finish` when its cleanup
  begins, and the backend stops repeating its cancel once `is_finishing`
  holds. The future `run_script` returns must be awaited to its end.
- `ScriptOutcome::was_cancelled()` holds when `stopped` is set or a
  statement is `Cancelled`. `affected` is `Some` only for statements that
  count rows (`INSERT`, `UPDATE`, `DELETE` and the like). An `Error`
  outcome carries the whole `Error` (message, code, detail and hint), not
  only its text.
- All statements run in one read-only transaction that is rolled back when
  the script ends, on every path: success, statement error, cancel. The
  outer `Err` is only for the session itself (connection lost, transaction
  could not start), for `Refused`, for `LeftReadOnly`, for `Unsupported` (a
  MySQL server too old to reset its session; the session stays usable) and
  for a cleanup failure. A cancel that lands before the first statement is
  `Ok` with no results and `stopped` set; in Messages the first statement
  then reads "Line N: Cancelled" and the rest "Not run".
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
    statement's `Error` outcome, and so is a statement with parameters
    (`?`, or `:name` as the driver reads it): there are no values to bind,
    and the driver would close the connection. Rows past `limit + 1` that
    the server still sends (`SHOW`, a `SELECT` with its own larger `LIMIT`)
    are read and dropped. The run needs a server that can reset its
    session (MySQL 5.7.3, MariaDB 10.2.4); an older one is told so before
    anything runs. The script sets `sql_select_limit` to
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
`MariaDB`), `SQLite 3.46.0` from `sqlite_version()`. Asked for when a
workspace's first SQL tab opens on a connected session, and again whenever
the session connects (a reconnect, a database switch) while the workspace
has SQL tabs; kept on the workspace as a `Fetch<String>`.

### Backend

- `Command::RunSql { session, request, statements, limit, timeout }` and an
  `Event::SqlRan { session, request, result: Result<ScriptOutcome>, cancel: Option<CancelReason> }`,
  where `CancelReason` is `User` or `Timeout(Duration)`. A reason is given
  only when a stop of ours ended the run; a cancel nobody here asked for
  (the server's own `statement_timeout`) has none.
- `Command::ServerVersion { session, request }` and
  `Event::ServerVersion { session, request, result: Result<String> }` carry
  the footer's server version.
- `StateFile::Settings(Settings)` lets `Command::Save` write the settings
  file, as it writes the connections and the known hosts.
- The run is registered like other requests, so the existing Cancel action
  (`Mod+.`, the Cancel button) cancels it by request, and a late cancel
  cannot reach the next command (the session waits for the cancels sent for
  a request before it starts the next one). A run cancelled while it is
  still queued never starts and answers as one stopped before it began.
- A Cancel for a running script sets its `StopFlag` and then sends the
  session's cancel again and again until the script begins its cleanup or
  is gone: a cancel that reaches PostgreSQL or MySQL in the gap between two
  of the script's round trips is lost, so one is not enough. Each cancel
  waits for the one before it, then 500 ms, 1 s, 2 s and so on up to 5 s
  (on MySQL every cancel is a new connection). The timeout and closing the
  session stop a script the same way, and whichever stops it first is the
  only one that keeps cancelling. Pressing Cancel again while those cancels
  are being sent sends one more at once, so the user never waits out the
  pause. Any other command still takes a single cancel.
- The timeout does not drop the running future (that would leave the server
  running the query). When it elapses, the backend stops the script as a
  Cancel does, records `CancelReason::Timeout` when it was the first to stop
  it, and waits for `run_script` to return; the interrupted statement then
  reports `Cancelled`, and the transaction is rolled back.
- A new run in the same tab cancels the one still running (reason `User`).
  Results for a closed tab or a replaced run are dropped by `RequestId`.
  A run is sent only on a connected session: while the workspace is
  connecting or disconnected, Run does nothing.
- The SQL text is never logged, and neither are a run's request id,
  statement count or timings. `Statement`, `SqlTab` and the fingerprint a
  tab keeps of the text it ran print (`Debug`) without the text, so a
  logged command or a panic message cannot hold it. The one line a run can
  log is a warning with the server's error when PostgreSQL could not
  release the advisory locks.

### Settings

`Settings` gains `sql_limit: u32` (default 1,000; choices 100, 1,000,
10,000) and `sql_timeout_secs: Option<u32>` (default 30; choices 10, 30, 60,
300, none). Both are `#[serde(default)]`, so older settings files load
unchanged. A new SQL tab starts from them; changing a menu in a tab updates
that tab and the setting, and the settings file is saved when the value
changed.

## Model

```rust
pub enum Tab { Object(Box<ObjectTab>), Sql(Box<SqlTab>) }

pub struct SqlTab {
    pub id: TabId,          // shared id space with object tabs
    pub number: u32,        // "Query 3"
    pub text: String,
    pub cursor: usize,      // byte offset, reported by the view
    pub limit: u32,
    pub timeout: Option<Duration>,
    pub run: Fetch<SqlRun>, // the last run, or the one running
    pub in_flight: Option<RunInFlight>, // Some exactly while a run is pending
    pub pane: ResultPane,   // Results or Messages
    pub split: f32,         // editor share of the height, 0.45 by default
    pub selection: Option<CellPos>,
    pub fields: Option<RowFields>, // the row panel's text for the selected row
    pub focus_editor: bool, // focus the editor on the next frame
    ran_text: Option<TextPrint>, // the text the last finished run started with
}

pub struct RunInFlight {
    pub statements: Vec<Statement>, // as split when the run started
    pub started: Instant,           // for the elapsed time
    pub text: TextPrint,            // the editor's text when the run started
}

pub struct SqlRun {
    pub statements: Vec<Statement>, // as split when the run started
    pub outcome: ScriptOutcome,
    pub cancel: Option<CancelReason>,
    pub error_mark: Option<(usize, Option<usize>)>, // line, and column if known
}
```

A run changes a tab only through `start_run` (which returns the run it
replaces, for the caller to cancel), `finish_run` (which ignores a stale
request) and `abandon_run` (the session is gone; the last result stays).
`is_running` and `running_for` tell of the run in flight; `last_run`,
`shown`, `shown_rows` and `dims` read the result that shows; `line_col` is
the cursor's line and column; `error_mark` is where the last run failed.
`TextPrint` is a length and a hash of a text, never the text: the error
mark holds only while the editor's text is still the one that ran, and no
run is in flight.

`selected_row` is the selected row while the Results pane shows it: the row
the row panel is for. `fields` holds that row's text (`RowFields`, as on
`ObjectTab`): `App::format_rows` fills it once per selection and frees it
when no panel shows the row, and `selected_fields` gives it while it is
still that row of that run. `Workspace::row_panel_tab` names the tab whose
row the row panel shows: an object tab in its Data view, or a SQL tab with a
selected row, and none while the panel is closed. `FoldDocuments` carries
the `id` of either kind of tab.

A run that failed as a whole (`Refused`, `LeftReadOnly`, `Unsupported`,
connection lost) is the `Fetch`'s error; `SqlRun` exists only when
statements ran. Such a failure also drops the result of the run before it:
nothing ran, so the tab shows the error and no older rows.

`ObjectTabId` becomes `TabId`, `active_object` becomes `active_tab`, and
`Workspace` gains helpers (`tab(id)`, `tab_mut(id)`, `object_tab(id)`,
`sql_tab(id)`, `active_object_tab()`, `active_sql_tab()`, `object_tabs()`,
`sql_tabs()`, their `_mut` forms, `push_sql_tab` and
`forget_session_requests`) so call sites that only care about table tabs
stay short. `Workspace` also gains `next_query: u32` and
`server_version: Fetch<String>`. The rename is a mechanical first step,
separate from the feature.

New actions: `NewSqlTab(ConnTabId)`, `RunSql { tab, sql_tab, all: bool }`,
and `SetSqlLimit`, `SetSqlTimeout`, `SetResultPane` and `SetSqlSplit`, each
with `tab` and `sql_tab`. `SqlEditorFocused { tab, sql_tab }` is the view
saying that the editor took the keyboard. The tab actions are generic:
`ActivateTab { tab, id }`, `CloseTab { tab, id }` and `CycleTab { tab, step }`
act on either kind of tab. `CancelQuery` also cancels a SQL tab's run.
`SelectCell` and `MoveSelection` carry the tab's `id` and work on a SQL
tab's result grid as on a table's.

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
- The Omarchy strip's row panel toggle stays on a SQL tab: a result's row
  has a row panel too (see Results).

### Shortcuts on a SQL tab

- `Mod+Return` runs the statement at the cursor, `Mod+Shift+Return` runs
  all, also while typing. `Mod+.` cancels.
- `Mod+W`, `Mod+Shift+[ / ]`, `Mod+1..9`, `Mod+B`, `Mod+P`, `?` work as on
  any tab.
- `Mod+R`, `Mod+F` and `Mod+Alt+Left / Right` do nothing on a SQL tab (no
  refresh, filter or paging here).
- While a row of the result is selected, the row panel's keys work as in a
  table tab: Space (outside the editor) and `Mod+Shift+R` show or hide it,
  and on Omarchy `i` does the same, Enter opens it, Esc closes it and `za`
  folds its documents. With no row selected they do nothing: there is no
  panel to show or hide. Omarchy's `[` and `]` step through the result's
  rows whenever its grid shows.
- On Omarchy, in a table tab as well: the Esc that leaves the editor (or the
  WHERE line) does only that, and the next Esc closes the panel; Enter
  opens the panel only when no button has the keyboard (a focused button
  takes it); `za` folds only while the panel shows.
- While the editor has focus, keys go to it (Tab inserts a tab character,
  arrows move the cursor). Esc leaves the editor; then arrows, Page Up/Down
  and Home/End move in the result grid, and the Omarchy vim letters that
  move the selection work as in a table tab.

### Toolbar

- macOS: Run (with `Cmd+Return`), Run all (`Shift+Cmd+Return`); on the right
  a "Read-only transaction" badge whose tooltip explains it, then
  "Limit 1,000" and "Timeout 30 s" menus.
- Omarchy: the tab title, a muted `read-only transaction · limit 1000 ·
  timeout 30s` whose limit and timeout parts open the same menus, then
  `run ctrl+enter` and `run all ctrl+shift+enter`.
- Where the toolbar is too narrow, pieces give way in this order: the run
  buttons' keys, the read-only note, on Omarchy the tab title, then the
  menus' words (leaving "1,000" and "30 s"), on macOS the menus' chevrons,
  and last the menus. The run buttons stay.
- Explain and Format are absent until slice 3, not disabled.

### Editor

- A multi-line `TextEdit` in code mode, drawn in the code `TextRole`, with a
  line-number gutter: absolute numbers on macOS and Windows, relative numbers
  with the current line absolute on Omarchy.
- Highlighting through a layouter over `sql::tokenize`, in theme colours:
  keywords, strings, numbers, comments, identifiers.
- The statement that Run would execute is marked: on macOS and in the
  standard look by a faint background band and a bar beside it, on Omarchy
  by the bar alone, in the accent colour.
- A click on the gutter focuses the editor and puts the cursor at the start
  of that line.

### Results

- A draggable horizontal splitter between the editor and the results, 45 to
  55 by default, kept per tab. It is the app's own, not an egui panel: the
  editor's share of the height lives on the tab (`SqlTab.split`, changed by
  `SetSqlSplit`), and each pane keeps a least height.
- The results header: tabs "Results" and "Messages", then
  "Statement at line 2 · 14 ms". The Results tab paints the row count after
  its name, muted ("Results 13"); its accessible name stays "Results".
- Results shows the existing grid (`ui::grid`), column name over type, with
  selection and scrolling as in a table tab. Its column headers are inert:
  a result is in the order its statement gave it, so there is no sorting.
  When the limit cut rows off, a note reads "First 1,000 rows (limit
  reached)".
- Selecting a row of the result opens the row panel on it: the same right
  panel as a table tab's (one width and one open or closed state per
  workspace), beside the editor and the results. It is there only while a
  row is selected and the Results pane shows: no placeholder takes the
  editor's width before that, the Messages pane hides it (the row stays
  selected), and a finished run clears the selection and so closes it. A
  result has no table behind it: the panel is titled by the row's number
  over "Query N", shows no key and no foreign key link, tags only booleans,
  and leaves out the editing controls and Omarchy's `y copy` hint. Closed
  with its button or a key, it stays closed until it is toggled back. A
  result row that has the keyboard keeps it when the panel appears, and
  what was folded or expanded in one result's row is not carried to the
  next result's.
- Known limit: the panel is as wide on a SQL tab as on a table tab, so in
  the smallest window (720 x 480) it can leave the editor and the results
  no width (always on Omarchy, elsewhere with the widest sidebar) until it
  is closed. A table tab has the same limit.
- Messages lists each statement: its line, then rows returned or affected
  and the time, or the error, or "Cancelled". Statements that did not start
  read "Not run": after an error, and after the one statement a stopped run
  reports as cancelled.
- States: before any run, a hint "Run a statement with" and the run key as
  the look spells it; while running, a spinner, the elapsed time and a
  Cancel button; a statement with no result set shows "Statement ran · no
  rows returned". The Messages tab opens by itself on an error or a
  refusal, on a timeout, and on a cancel that left no rows to show; a run
  the user cancelled after rows came back stays on Results. A statement's
  error marks its line in the gutter in the error colour.

### Footer

- macOS: `5 rows · 14 ms`, `Read-only transaction · rolled back`, then on
  the right `Ln 12, Col 21` and the server version.
- Omarchy: a mode line of key hints (run, run all, cancel, leave editor,
  tables), then `ln 12:21 · 5 rows · 14 ms · rolled back` on the right. The
  vim mode indicator waits for slice 5.

## Errors and edge cases

- A statement's error shows the database's message prefixed with its line:
  "Line 12: relation "books_x" does not exist". With a position, the prefix
  is "Line 12, col 15:" and the gutter marks that line. The code, detail
  and hint a database gave follow on lines of their own. What a database
  said is cut at 2,000 characters where it is shown.
- Writes fail through the database's own read-only error (PostgreSQL and
  MySQL read-only transactions; SQLite is opened read-only); escaping the
  transaction is refused by the guard.
- A timeout reads "Cancelled after 30 s (timeout)", a user cancel
  "Cancelled". A `Cancelled` outcome without a reason (a user-set
  `statement_timeout`, say) also reads "Cancelled". The cancelled
  statement's rows are dropped, so a partial result never looks complete;
  statements that finished before it keep theirs in Messages, and the last
  of them that returned rows stays in Results. The statements after it read
  "Not run". A run stopped before its first statement began has no results:
  its first statement reads as cancelled and the rest "Not run".
- Session settings a script changes (`SET search_path`, `SET time_zone`)
  last only for that run: PostgreSQL rolls them back, MySQL resets the
  session. SQLite puts `query_only`, `trusted_schema`, `case_sensitive_like`
  and the busy timeout back after every run; its other `PRAGMA`s and its
  `ATTACH`es last for the session.
- A run that failed as a whole (a refusal, a lost session) shows its error
  and no older rows: the result of the run before it is dropped.
- A lost connection shows the existing reconnect banner; the tab keeps its
  text.
- Run on an empty or comment-only editor does nothing and shows no error.
  Nor does Run while the session is connecting or disconnected.
- Statements that cannot run in a transaction (PostgreSQL `VACUUM`,
  `CREATE INDEX CONCURRENTLY`) fail with the server's own error.
- On PostgreSQL a row query runs through a cursor, so it is planned for a
  fast first row and never in parallel: it can be slower than the same
  query in psql, and `EXPLAIN` shows the plan psql would use, not the
  cursor's.

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
  `Mod+F` do nothing on a SQL tab; selecting a result row opens the row
  panel with that row's values, the Messages pane and a new run close it, a
  focused row keeps the keyboard, and the panel's keys work on a selected
  result row.
- Settings: an older settings file without the new keys loads with the
  defaults.
- No design or pixel conformance checks. Screenshots for review use the
  Bookshop demo data and stay local.
