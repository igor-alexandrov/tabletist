# Writes from the SQL editor

Date: 2026-10-05. Status: design, not yet planned.

## Intent

The SQL editor only reads: every run is a read-only transaction that is
rolled back, on a writable connection too. This slice lets a SQL tab of a
writable connection run `INSERT`, `UPDATE`, `DELETE`, DDL and whatever else
the database takes inside a transaction, and keep the result.

Success: on each driver, a user on a writable connection switches a SQL tab
to Read-write, runs statements that change data, reads how many rows each
one changed and whether the run was committed or rolled back, and finds the
session afterwards as it was before; a run that fails, is cancelled or times
out leaves nothing behind, or says exactly what it left; on production the
statements are shown and confirmed before anything is sent; and on a
read-only connection nothing in the app can change data, as today.

This is slice 6 of editing values
(`2026-10-03-value-editing-core-design.md`, "Editing as a whole") and it
changes what the core editor promises (`2026-09-30-sql-editor-core-design.md`).

The design canvas has no artboard for a run that writes. What it gives is
the frame: the toolbar's "Read-only transaction" badge as a switch for the
tab, the Editor settings "New tabs run in" and "Confirm UPDATE or DELETE
without WHERE", and the "Write blocked" card of the "read-only connection"
error artboards (macOS and Omarchy). They are not copied into the
repository.

## Decisions

| Question | Decision |
|---|---|
| Who may write | A SQL tab switched to Read-write, on a writable connection. Never on a read-only one. |
| Commit model | One transaction per run. Committed when every statement succeeded, rolled back on the first error, cancel or timeout. |
| Which runs write | A run is read-write only when the tab is in Read-write mode and one of its statements looks like a write. Every other run is today's read-only run, unchanged. |
| A write taken for a read | The database refuses it in the read-only transaction, and the card offers to run it again read-write. |
| New tabs | Read-only, until `editor.sql_new_tab` says to follow the connection. |
| Production | Every read-write run shows its statements and is confirmed first (Omarchy: by typing `write`). |
| `UPDATE` or `DELETE` without `WHERE` | Asked about on every writable connection, unless `editor.confirm_unsafe_writes` is off. |
| Transaction statements | `BEGIN`, `COMMIT`, `ROLLBACK` and the rest of the refusal list stay refused. The run owns its transaction. |
| After a commit | Table tabs of the workspace load again when next shown. The tree and the completions reload after DDL. |

Rejected: autocommit for each statement, as psql does (Run all could leave
a script half applied); a transaction held open across runs with Commit and
Rollback buttons (a session of its own for each tab, locks held while the
user thinks, and a much larger build); a write tab in which every run
commits (a write the classifier took for a read would then skip the
production confirmation, and a plain `SELECT` would end "committed"); asking
on production before every run, reads included (the question would be
answered without being read); letting a tab of a read-only connection
write, as one artboard offers (the session there is read-only, on SQLite
down to the file handle, so it needs a second session for the tab, and
"Open read-only" would no longer mean what it says); `ctrl+w` for the
switch, as the Omarchy artboard has it (it closes the tab).

## Out of scope

- Writes on a read-only connection, for a tab or otherwise.
- A transaction that outlives its run, savepoints, isolation levels and
  `SET TRANSACTION`.
- Statements that cannot run inside a transaction (PostgreSQL `VACUUM`,
  `CREATE DATABASE`, `CREATE INDEX CONCURRENTLY`, SQLite `VACUUM`). They
  keep failing with the database's own error.
- Everything on the refusal list: PostgreSQL `COPY`, MySQL account and
  server statements, `USE`, `LOCK TABLES`, `PREPARE` and the rest. The list
  does not change.
- MySQL `CALL`. A procedure can commit the run's transaction and open one
  of its own, and the run cannot tell that one from its own; it would
  report "nothing was written" over work that is. It stays refused until a
  run can check whose transaction it is in.
- A PostgreSQL `CALL` whose procedure commits: PostgreSQL refuses that
  inside a transaction block, with its own error. MySQL statements the
  prepared protocol does not take fail the same way, as today.
- A record of what was written. Query history is its own slice.
- The affected count of a statement that also returns rows (`RETURNING`):
  it reports its rows, as today.
- The text of MySQL warnings. A statement says how many it raised.
- A control for the two new settings: the Editor tab of the Settings window
  is not built. They are set in the file.

## Which runs write

### The tab's mode

`SqlTab` gains `mode: RunMode`, `ReadOnly` or `ReadWrite`. A new tab starts
`ReadOnly`; with `editor.sql_new_tab = "connection"` it starts `ReadWrite`
on a writable connection. The user switches it with the toolbar's badge or
`Mod+Shift+M`.

The mode that counts is the effective one: `ReadWrite` only while the
workspace's `access` is `Writable`. A tab whose session comes back
read-only (the box was turned on meanwhile) runs read-only and shows the
read-only connection's badge; its own mode is kept and counts again if the
session is writable once more. On a read-only connection the key and the
badge do nothing.

### The statement's kind

`tabletist-db` says what a statement looks like, on its tokens:

    pub enum StatementKind { Read, Write }
    sql::kind(dialect, statement) -> StatementKind

A statement is a `Read` when its first word is one of `SELECT`, `VALUES`,
`TABLE`, `WITH`, `SHOW`, `EXPLAIN`, `DESCRIBE`, `DESC`, on SQLite `PRAGMA`,
or its first token is `(`, and no unquoted word of it is `INSERT`,
`UPDATE`, `DELETE` or `MERGE`. One exception: a statement that starts with
`EXPLAIN` and has no `ANALYZE` among its words is a `Read` whatever it
explains, since it executes nothing. Everything else is a `Write`.

- It errs toward `Write`: `SELECT ... FOR UPDATE`, a data-modifying `WITH`
  and a column named `update` all count as writes. A read taken for a write
  costs a commit of nothing and, on production, a confirmation.
- `EXPLAIN ANALYZE UPDATE` executes its `UPDATE`, so it is a write like the
  statement it explains: in a read-write run its changes are committed, and
  on production it is shown and confirmed like any other.
- A write taken for a read (`SELECT setval(...)`, a function that writes,
  PostgreSQL `SELECT ... INTO`, SQLite `PRAGMA user_version = 5`) is not a
  hole: it runs in the read-only transaction and the database refuses it.

### The rule

A run is read-write when both hold:

1. the tab's effective mode is `ReadWrite`, and
2. at least one of the statements it would run is a `Write`, or the user
   asked for it from the card ("Run in a read-write transaction").

Every other run goes through `ScriptMode::ReadOnly`, which is the run the
core editor spec describes, guard layers, cleanup and wording included.
Nothing makes a run read-write in a tab whose effective mode is `ReadOnly`.

So the confirmations below and the transaction's mode are decided by the
same answer. No run is read-write without having passed them.

## The read-write run (`tabletist-db`)

    pub enum ScriptMode { ReadOnly, Write }

    pub async fn run_script(
        &self,
        statements: &[Statement],
        limit: u32,
        mode: ScriptMode,
        stop: &StopFlag,
    ) -> Result<ScriptOutcome>;

    pub struct ScriptOutcome {
        pub results: Vec<StatementResult>,
        pub stopped: bool,
        pub end: ScriptEnd,
        /// What the database said when it could not undo everything
        /// (MySQL's non-transactional tables). With it, `RolledBack` and
        /// `Partly` no longer say that the rest is gone.
        pub rollback_warning: Option<String>,
        /// A `Write` run only: the session could not be put back after
        /// the run and must be closed. `end` still holds.
        pub broken: Option<Error>,
    }
    pub enum ScriptEnd {
        /// Nothing of the run remains. Every read-only run ends so.
        RolledBack,
        /// Every statement's work is written.
        Committed,
        /// The database committed on its own before the run failed or was
        /// stopped: the first `committed` statements are written, the rest
        /// is not.
        Partly { committed: usize },
        /// The commit itself failed. Nothing is written.
        CommitFailed(Error),
    }

`StatementOutcome::Done` gains `warnings: u16`, which only MySQL fills.

`ScriptMode::Write` on a `ReadOnly` connection returns `Error::ReadOnly`
without contacting the server (the variant comes with
`Connection::write`; this slice adds it if it lands first). An empty list
of statements is `Ok` with no results, as today.

### What holds in both modes

- The refusal check runs first and a refused statement fails the whole run
  with nothing sent. The list is the core spec's, unchanged. The sentences
  that name the transaction follow the mode. Read-only: "Tabletist runs
  every query in a read-only transaction, so COMMIT is not allowed". Write:
  "Tabletist runs and commits the script in one transaction of its own, so
  COMMIT is not allowed". The drivers' own "could not start (or end) the
  read-only transaction" say "the transaction" in a `Write` run.
- One statement at a time: PostgreSQL prepares each, MySQL runs each
  through the prepared protocol, SQLite's authorizer sees each.
- Statements run in order and stop at the first error or cancel. The
  results before it are kept in the outcome.
- At most `limit` rows are kept for a statement that returns rows. Nothing
  is cancelled to stop reading.
- The session is put back after every run, on every path. A cleanup step
  that fails closes the session, as today. In a `ReadOnly` run that is the
  run's `Err`. In a `Write` run whose end is known the outcome is returned
  with the failure in `broken`, and the backend closes the session after
  it has delivered the result: a run known to be committed is never shown
  as one that may be.

### PostgreSQL

1. `BEGIN`, then `SAVEPOINT tabletist_guard`. The transaction takes the
   server's and the role's defaults: a role an administrator made read-only
   stays so, and its error is the statement's.
2. Before every statement the savepoint is swapped, as in a read-only run.
   That fails outside a transaction block and in one the script chained to.
   PostgreSQL never commits on its own, so a transaction that is gone means
   the script got past the refusal list: the run ends with
   `Error::LeftTransaction` before the statement runs, and the backend
   closes the session. `LeftTransaction` counts as a lost connection, as
   `LeftReadOnly` does.
3. Every statement is prepared first, as today. No statement of a `Write`
   run goes through a cursor: a cursor runs its query only as far as it is
   fetched, so `SELECT refill(id) FROM shelves` would do its work for
   `limit + 1` rows and the run would commit that much. A statement with
   columns, a row query as much as `INSERT ... RETURNING`, runs to its end
   through `simple_query_raw`, keeping `limit + 1` rows and dropping the
   rest as they stream; the timeout bounds it. That also runs a
   data-modifying `WITH` that returns rows, which `DECLARE` refuses. A
   statement without columns runs through `simple_query` with its count.
4. After the last statement, with none failed and no stop: the savepoint is
   swapped once more, the driver calls `stop.finish()`, and `COMMIT` is
   sent. A stop that was set before `finish` rolls back instead. The swap
   is what makes an answered `COMMIT` mean committed: in a failed
   transaction PostgreSQL answers `COMMIT` with a rollback and no error,
   and the driver does not see which.
5. A `COMMIT` that fails (a deferred constraint, a serialization failure)
   is `ScriptEnd::CommitFailed` with the database's error, and PostgreSQL
   has rolled the transaction back. A cancel that was on its way lands
   either while the session is idle, where the server drops it, or on the
   commit's own work, where it fails the commit like any other error. It
   never leaves a commit that failed and is written.
6. After an error or a stop: `ROLLBACK`, and `ScriptEnd::RolledBack`.
7. Cleanup. A committed transaction keeps what a rolled back one undid, so
   the run undoes it itself: `CLOSE ALL`, `UNLISTEN *`, `RESET SESSION
   AUTHORIZATION`, `RESET ROLE`, `RESET ALL`, the connect-time settings
   again (`SET standard_conforming_strings = on`), and `SELECT
   pg_advisory_unlock_all()`. `RESET ALL` leaves the role and the session
   authorization alone, which is why they are reset by name. Temporary
   tables last for the session.

### MySQL

1. The run needs a server that can reset its session, as today. It starts
   with `START TRANSACTION` on the writable session. It does not send `SET
   SESSION TRANSACTION READ ONLY` and does not set `sql_select_limit`: a
   `SELECT` that calls a function that writes must run for every row, and
   no server version is trusted to keep the limit off an `INSERT ...
   SELECT`. Rows past `limit + 1` are read and dropped, as for `SHOW`.
2. MySQL commits on its own before DDL and some other statements, also
   when the statement then fails. So before every statement the driver
   asks where the session stands, with a query of its own whose answer
   carries the server's transaction status, as the read-only run's check
   does. Outside a transaction means the statements so far are written:
   the driver notes how many, starts a new transaction and goes on.
3. After a statement fails or the run is stopped, the driver asks the same
   question once more before `ROLLBACK`, and notes the same. A failed
   `CREATE TABLE` has still committed what came before it. The status the
   driver holds is no answer here: an error packet carries none, and
   mysql_async empties what it held, so only a new query tells.
4. With every statement done: `COMMIT`, after `stop.finish()`, and
   `ScriptEnd::Committed`. A `COMMIT` that fails is `CommitFailed`, after a
   `ROLLBACK`.
5. After an error or a stop: `ROLLBACK`. With nothing noted the end is
   `RolledBack`, otherwise `Partly { committed }`. When the `ROLLBACK`
   reports a warning, the driver reads it with `SHOW WARNINGS` before
   anything else and puts its text in `rollback_warning` (1196: changes to
   non-transactional tables could not be rolled back).
6. A statement's `Done` carries the warning count the server reports.
7. Cleanup is today's: the reset, then the connect-time statements, ending
   with `SET SESSION TRANSACTION READ WRITE` only after a run that ended
   cleanly.

The status tells a transaction from none, not one transaction from
another. Without stored procedures no single statement can end the run's
transaction and leave another open, which is why `CALL` stays refused. A
statement that ends the transaction past the refusal list is handled by
step 2 like any commit the server makes: reported, and never followed by a
statement outside a transaction.

### SQLite

1. The app's own statements, outside the fence: `PRAGMA query_only = OFF`,
   then `BEGIN IMMEDIATE`, so a locked database is found before any
   statement runs. A `BEGIN` that fails as busy is the run's error with
   SQLite's message; the session stays open.
2. Each statement runs behind the script fence (`Fence::Script`), which
   denies what it denies today: transaction and savepoint statements,
   `query_only` and `writable_schema` with a value, `wal_checkpoint`. What
   lets the statement write is `query_only` being off.
3. Before every statement the driver asks whether its transaction is still
   open. One that is gone ends the run with `Error::LeftTransaction`.
4. `COMMIT` after the last statement, `ROLLBACK` after an error or a stop.
   SQLite keeps the transaction open when a `COMMIT` fails (a deferred
   foreign key, a reader holding the file in rollback-journal mode), so a
   failed `COMMIT` is followed by `ROLLBACK` and is then
   `ScriptEnd::CommitFailed`.
5. On every path the session's settings are put back (`query_only = ON`
   first, then the rest of the connect-time pragmas), so browsing and the
   next run are fenced again.

### Cancel, timeout and a lost connection

- The backend stops a read-write run as it stops any run: the stop flag and
  the session's cancel, repeated until the driver says it is finishing. The
  timeout applies to read-write runs too. A stopped run is rolled back.
- A driver calls `stop.finish()` before it sends `COMMIT`. From then on no
  cancel is sent, and the run ends as the commit ends.
- A connection lost during a read-write run is the run's `Err`, as today.
  The app cannot know what of it was written, and says so.

## The app

### Backend and model

- `Command::RunSql` gains `mode: ScriptMode`. `Event::SqlRan` is unchanged;
  the outcome carries the end.
- `SqlTab` gains `mode: RunMode` and `asking: Option<HeldRun>`, the run a
  confirmation is holding: its statements as split, the print of the text
  they came from, and what is asked. `SqlRun` and `RunInFlight` gain the
  `ScriptMode` the run was sent with. `HeldRun` prints (`Debug`) without
  the text, as `Statement` does.
- New actions: `SetSqlMode { tab, sql_tab, mode }`, `ToggleSqlMode { tab,
  sql_tab }`, `ConfirmSqlRun { tab, sql_tab, run: bool }` and `RunSqlAgain {
  tab, sql_tab }`, the card's "Run in a read-write transaction". `RunSql`
  keeps its shape.
- `RunSql` splits the text as today, takes the kinds, and decides. A
  read-only run is sent at once. A read-write run that needs no
  confirmation is sent at once. Otherwise the run is held and asked about,
  and `ConfirmSqlRun` sends it or drops it. What is sent is what was shown,
  whatever the editor holds by then.
- `RunSqlAgain` runs the statements of the tab's last run again as a
  read-write run, through the same confirmations. It is offered only while
  the editor's text is still the text that ran, as the error mark is.
- `RunSqlAgain` and `ConfirmSqlRun` look at the tab's effective mode again
  when they are applied, and do nothing unless it is `ReadWrite`: a card
  left on screen or a question still open cannot write for a tab that was
  switched back or whose session came back read-only. A confirmation is
  modal. While it is up the tab takes no Run and its mode cannot be
  switched.
- While a read-write run of a tab is in flight, Run and Run all do nothing
  in that tab, and their buttons are disabled. A read-only run is still
  replaced by a new run, as today.
- The SQL text is never logged, in any of this.

### Confirmations

Both are asked before anything is sent, and only for a read-write run.

**Production.** When the workspace's environment is production:

- macOS and Windows: "Run 3 statements on production?" ("Run this
  statement on production?" for one), the connection's name and database,
  the statements as typed in a pane that scrolls, "One transaction.
  Committed when every statement succeeds.", **Cancel** and **Run on
  production**. On MySQL, when a statement starts with `CREATE`, `ALTER`,
  `DROP`, `RENAME` or `TRUNCATE`, the line reads "MySQL commits
  everything so far when it runs CREATE, ALTER, DROP, RENAME or TRUNCATE.
  A later error undoes neither them nor what came before."
- Omarchy: the red PROD box with the same facts and the statements, and a
  field that takes the word `write`; Enter confirms only when it holds
  exactly that, Esc cancels.
- It is the sheet and the box of a save to production
  (`2026-10-03-value-editing-core-design.md`, "Saving to production"),
  with a script in place of the changes.

**No `WHERE`.** `sql::unbounded(dialect, statement) -> Option<Verb>` names
`UPDATE` or `DELETE` when the statement's first word is that verb, or
`WITH` followed by that verb outside parentheses, and no `WHERE` stands
outside parentheses after it. A `WHERE` inside a subquery does not count.
An `EXPLAIN` with `ANALYZE` in front, options included, is looked through:
the statement behind it is the one that runs.
`INSERT ... ON CONFLICT DO UPDATE`, `MERGE` and `TRUNCATE` are not asked
about: the first two are bounded by their rows, and the last says what it
does.

- With `editor.confirm_unsafe_writes` on, a read-write run holding such a
  statement asks: "This UPDATE has no WHERE", "Line 3 changes every row of
  its table.", **Cancel** and **Run anyway** (Omarchy: `[y]` run, `[esc]`
  cancel). Several such statements are listed in the one prompt.
- On production there is one question, not two: the production sheet
  carries the same lines as a warning above its statements.

### Toolbar and keys

- macOS and Windows, writable connection: the badge is a menu, as Limit and
  Timeout are, with "Read-only transaction" and "Read-write transaction".
  In Read-write it is drawn in the warning tone, on production in the
  production tone. Its tooltip says what the mode does: "Runs are rolled
  back. Nothing is changed." and "A run that changes data is committed
  when every statement succeeds."
- Read-only connection: the badge is today's, inert, and its tooltip adds
  "This connection opens read-only."
- Omarchy: the first part of `read-only transaction · limit 1000 · timeout
  30s` opens the same menu and reads `read-write transaction` in the
  warning colour, the production colour on PROD.
- `Mod+Shift+M` switches the mode of the active SQL tab. It is in the
  shortcuts table as "Read-only or read-write runs in the SQL editor".
- The toolbar gives way in the core spec's order, with the badge in the
  place of the read-only note.

### Results, Messages and the footer

A read-only run reads as today. For a read-write run:

- Messages lists each statement as today ("Line 3: 12 rows affected ·
  4 ms"), with "· 2 warnings" where MySQL raised some, then one line for
  the run's end:
  - `Committed`: "Committed · 3 statements · 21 ms".
  - `RolledBack` after an error: "Rolled back. Nothing was written."
  - `RolledBack` after a cancel or a timeout: the cancel's own words, then
    the same line.
  - `Partly`: "Lines 1 to 4 are written: MySQL commits CREATE, ALTER, DROP
    and similar statements as they run. The rest was rolled back."
  - `CommitFailed`: "The commit failed. Nothing was written.", then the
    database's error as a statement's error is shown.
  - With a `rollback_warning`, in place of the `RolledBack` line and of
    `Partly`'s last sentence: "MySQL could not roll back every change.",
    then its text. Nothing then says "Nothing was written".
  - With `broken`: the end's own line, then "The session could not be put
    back and was closed.", and the reconnect banner.
- In a run that did not end `Committed`, a statement whose work was undone
  ends its line with "· rolled back", so no count reads as a change that
  stayed. With a `rollback_warning` no line gets it.
- Results shows the last statement that returned rows, as today. A
  statement without a result set shows "Statement ran · 12 rows affected"
  when it counted rows, else today's "Statement ran · no rows returned". A
  statement counts rows where it does today (`WITH ... UPDATE` and `CREATE
  TABLE ... AS` do not).
- Messages opens by itself as today, and also when a read-write run ends
  `Partly`, `CommitFailed`, with a `rollback_warning` or `broken`.
- Footer, macOS and Windows: the shown statement's `12 rows affected ·
  14 ms`, as it shows a result's rows today, then `Read-write transaction ·
  committed`, `· rolled back`, `· partly committed`, `· commit failed`,
  or, with a `rollback_warning`, `· not fully rolled back`. Omarchy:
  `ln 3:1 · 12 rows affected · 14 ms · committed`.
- A connection lost during a read-write run shows the reconnect banner and
  "The connection was lost during a read-write run. Some or all of it may
  be written."

### The refused write

A write the database refused in a read-only run is still a card, not a
statement error, with the database's own words under it. It moves to
where the user is looking: the head of the Messages pane, above the
statements' lines. A statement's error opens Messages by itself, and today
the card is drawn in Results, and only when no statement of the run
returned rows, so a run whose first statement returned rows never showed
it. Results no longer draws it. What it says depends on why the run was
read-only:

| Situation | Title and text | Action |
|---|---|---|
| Read-only connection | "This connection opens read-only". "Bookshop · production blocks writes, so PostgreSQL refused the UPDATE. Nothing changed." Then: "To write, turn off Open read-only in the connection. It applies from the next connect." | **Edit connection** (Omarchy `e`) |
| Writable connection, tab in Read-only | "This tab runs read-only". "Every run here is a read-only transaction, so PostgreSQL refused the UPDATE. Nothing changed." | **Allow writes in this tab** (Omarchy `w`) |
| Writable connection, tab in Read-write, run taken for a read | "This run was read-only". "Its statements looked like reads, so they ran in a read-only transaction and PostgreSQL refused this one. Nothing changed." | **Run in a read-write transaction** (Omarchy `w`) |

- "Allow writes in this tab" sets the mode and runs nothing.
- Edit connection opens the connection dialog on the workspace's saved
  connection (`Action::EditConnection`).
- The Omarchy keys work while the editor does not have the keyboard.
- A statement the guard refused (`Error::Refused`) keeps its own sentence
  and is not one of these cards any more: the refusal list is about the
  transaction, not about writing.
- In a read-write run a read-only error from the database (a standby, a
  role made read-only) is an ordinary statement error.

This replaces "The SQL editor only reads data".

### After a commit

- When a read-write run ends `Committed` or `Partly`, with a
  `rollback_warning`, or with its connection lost, every table tab of the
  workspace is marked stale. A stale tab with no
  pending changes loads its page and structure again when it is next
  shown, as Refresh does. A tab with pending changes is left alone: a
  save's check against the loaded values covers it. Any fetch of the page
  clears the mark.
- When a statement that starts with `CREATE`, `ALTER`, `DROP` or `RENAME`
  completed in such a run, the tree is refreshed (`Action::RefreshTree`,
  which also drops the columns the completions hold).

### Leaving while a read-write run is in flight

Closing the tab, closing or disconnecting the connection, switching
database and closing the window are held while a read-write run of theirs
is in flight: "Query 2 is still writing", "Closing cancels the run. It is
rolled back unless its commit was already sent.", **Cancel the run and
close** and **Keep waiting** (Omarchy `[c]`, `[esc]`). It is the leaving
guard of value editing with one more reason. A read-only run in flight is
dropped without a question, as today.

### Settings

| Key | Type | Default | Notes |
|---|---|---|---|
| `editor.sql_new_tab` | string | `"read-only"` | `"read-only"` or `"connection"`. Any other value is an invalid line. |
| `editor.confirm_unsafe_writes` | boolean | `true` | The question about `UPDATE` and `DELETE` without `WHERE`. |

Both are `#[serde(default)]` in `Settings` and reload live with the file.
`sql_new_tab` reaches the SQL tabs opened afterwards, as `sql_limit` does;
`confirm_unsafe_writes` is read when a run is decided.

## The promise, restated

On a read-only connection no action in the app can modify data. On a
writable connection only Save can, and a run in a SQL tab the user
switched to Read-write; browsing, a raw WHERE and every other run still
cannot.

## Errors and edge cases

- A statement's error, its line and its position read as today. After it
  nothing more runs and the transaction is rolled back.
- The timeout cancels a read-write run and rolls it back: "Cancelled after
  30 s (timeout)", "Rolled back. Nothing was written." A long change needs
  a longer timeout.
- A cancel that arrives after the commit was sent does nothing. The run
  ends committed and says so.
- Session settings a script changes last only for the run, in both modes.
  A read-write run on PostgreSQL resets them itself.
- Run on an empty or comment-only editor does nothing, and neither does
  Run while the session is connecting or disconnected.
- A tab switched to Read-write whose runs are all reads never opens a
  read-write transaction.
- A run held by a confirmation is dropped when its tab or its connection
  closes, and when the session is lost.
- On PostgreSQL a savepoint is taken before every statement of a
  read-write run. A script of very many statements pays for each.
- A SQLite database another program holds locked fails the run at `BEGIN`
  after the busy timeout, with SQLite's message.

## Steps

Each step ends compiling, tested and shippable, and gets its own plan run:

1. **The run.** `ScriptMode`, `ScriptEnd`, `sql::kind`, `sql::unbounded`,
   the read-write run in the three drivers, `Error::LeftTransaction`. The
   backend passes `ScriptMode::ReadOnly` everywhere; nothing in the app
   writes. It needs nothing from value editing and can be built on `main`.
2. **The tab.** The mode, the badge's menu and `Mod+Shift+M`,
   `editor.sql_new_tab`, the decision in `RunSql`, the production
   confirmation in both looks, the three cards, Messages, Results and the
   footer for a read-write run, and Run held back while one is in flight.
   It reuses the production sheet and the PROD box of value editing's step
   3, so it is planned once that has landed. Until step 3 of this spec,
   closing a tab or a connection cancels a read-write run in flight
   without asking, as it cancels any run (rolled back unless its commit
   was already sent), and table tabs show what they loaded until they are
   refreshed by hand.
3. **Around it.** The question about a missing `WHERE` and
   `editor.confirm_unsafe_writes`, stale table tabs, the tree's refresh
   after DDL, and the leaving guard for a run in flight.

No step ships a read-write run on production without its confirmation.

## Testing

- `tabletist-db` unit tests:
  - `sql::kind` in the three dialects: every read form; a data-modifying
    `WITH`; `SELECT ... FOR UPDATE`; `EXPLAIN DELETE` as a read and
    `EXPLAIN ANALYZE DELETE`, also as `EXPLAIN (ANALYZE) DELETE`, as a
    write; `REPLACE INTO` as a write and `SELECT replace(...)` as a read;
    DML words inside strings, comments and quoted names not counting.
  - `sql::unbounded`: no `WHERE`; a `WHERE` only inside a subquery; a
    `WHERE` at the top; `WITH ... DELETE`; `WHERE CURRENT OF`; `EXPLAIN
    ANALYZE DELETE FROM t`; `INSERT ... ON CONFLICT DO UPDATE` not asked
    about.
  - The refusal's sentence in each mode, and MySQL `CALL` refused in both.
- `run_script` in `Write` mode on SQLite (always) and on PostgreSQL and
  MySQL when their test URLs are set, each asserting a probe table
  afterwards:
  - an insert, an update and a delete are committed and report their
    counts;
  - a failing statement in the middle, a cancel and a timeout each leave
    the table untouched and end `RolledBack`;
  - `Write` on a `ReadOnly` connection sends nothing;
  - rows of `INSERT ... RETURNING` (PostgreSQL, SQLite) are cut at the
    limit while every row is written;
  - a stop set before the commit rolls back, and one set after `finish`
    does not stop the commit;
  - after a run, and after one that failed or was cancelled, the session
    is as it connected: PostgreSQL `search_path`, role, a `WITH HOLD`
    cursor and an advisory lock; MySQL `time_zone` and the read-write
    setting; SQLite `query_only`. A read-only run right after cannot write.
- PostgreSQL: a deferred constraint that fails at `COMMIT` is
  `CommitFailed` with nothing written; a script that ends the transaction
  past the refusal (forced in the test) ends `LeftTransaction`, with the
  statement after it never run; a `SELECT` that calls a writing function
  over more rows than the limit writes for every row; a data-modifying
  `WITH` that returns rows runs and is committed; a cleanup step that
  fails after the commit (forced) returns `Committed` with `broken`.
- MySQL: `CREATE TABLE` between two inserts with a failing statement at
  the end is `Partly` with the right count; a failing `CREATE TABLE` still
  counts what came before it; a failing statement with no DDL before it
  is `RolledBack`, not `Partly`; an `INSERT ... SELECT` and a `CREATE
  TABLE ... SELECT` copy more rows than the limit; a rolled back change to
  a MyISAM table sets `rollback_warning`; a truncating insert in a session
  without strict mode reports its warning.
- SQLite: the authorizer still denies `COMMIT`, a savepoint and `PRAGMA
  query_only` in a `Write` run; a database locked by another handle fails
  at `BEGIN` and leaves the session open and fenced; a deferred foreign
  key that fails at `COMMIT` is `CommitFailed`, with nothing written, no
  transaction left open and the next run able to begin.
- The existing guard and bypass tests run unchanged in `ReadOnly` mode on
  both accesses.
- Reducer tests: the mode of a new tab under each setting and access; the
  effective mode after a reconnect that came back read-only; which runs
  are read-write; a held run sends nothing until it is confirmed and sends
  what was shown; a declined one sends nothing; Run during a read-write
  run in flight; `RunSqlAgain` only while the text is the one that ran;
  `RunSqlAgain` and `ConfirmSqlRun` doing nothing once the effective mode
  is read-only;
  stale marks and their refetch, with and without pending changes; the
  tree's refresh after DDL; the leaving guard.
- Headless UI tests, in every look: the badge's menu and the key; the
  three cards and their actions, in Messages, after a statement that
  returned rows too; the production confirmation, with
  `write` typed on Omarchy; the question about a missing `WHERE`, alone
  and inside the production sheet; Messages, Results and the footer for
  each `ScriptEnd`; the badge on a read-only connection.
- Settings: an older file without the new keys loads with the defaults; an
  unknown `sql_new_tab` is an invalid line.
- `src/shots.rs` gains scenes for review on the Bookshop demo data, whose
  SQLite file is a throwaway and writable. No test compares a screen with
  the design.

## Documents this changes

The SQL editor spec's intent, its "Run all" decision, its guard (which now
describes the read-only mode of two), its toolbar, footer and "Errors and
edge cases", "Closing one never asks" and "A new run in the same tab
cancels the one still running"; the value editing spec's slice list, its "Writable
connections" card text and its restated promise; the main spec's section
4.3 and its keyboard table; the settings spec's key table; the crate
documentation of `tabletist-db` and of `Connection::run_script`; the
README; the shortcuts table in `ui/keys.rs`.
