# Writes from the SQL editor

Date: 2026-10-05. Status: step 1 (the run) is built, see
`docs/superpowers/plans/2026-10-05-sql-editor-writes-run.md`. Step 2 (the
tab) is built in two runs. The first is
`docs/superpowers/plans/2026-10-05-sql-editor-writes-tab.md`: a tab that
writes, on every connection but a production one. The second, not yet
planned, is the production confirmation and `editor.sql_new_tab`. Step 3
is not yet planned. Since 2026-10-06 a new tab follows its connection: it
opens in Read-write where its editors can write, which the first draft
left to `editor.sql_new_tab` and off by default. On production it opens
in Read-only all the same. Since the same day the toolbar's badge and its
menu are a segmented switch, as the design canvas now draws it ("Toolbar
and keys"). The PostgreSQL and MySQL runs were built with no
server at hand: their tests have been compiled, and are first run by CI.

## Intent

The SQL editor only reads: every run is a read-only transaction that is
rolled back, on a writable connection too. This slice lets a SQL tab of a
writable connection run `INSERT`, `UPDATE`, `DELETE`, DDL and whatever else
the database takes inside a transaction, and keep the result.

Success: on each driver, a user on a writable connection opens a SQL tab,
which is in Read-write, runs statements that change data, reads how many rows each
one changed and whether the run was committed or rolled back, and finds the
session afterwards as it was before; a run that fails, is cancelled or times
out leaves nothing behind, or says exactly what it left; on production the
statements are shown and confirmed before anything is sent; and on a
read-only connection nothing in the app can change data, as today.

This is slice 6 of editing values
(`2026-10-03-value-editing-core-design.md`, "Editing as a whole") and it
changes what the core editor promises (`2026-09-30-sql-editor-core-design.md`).

The design canvas had no artboard for a run that writes when this was
written. What it gave is the frame: the toolbar's "Read-only transaction"
badge as a switch for the tab, the Editor settings "New tabs run in" and "Confirm UPDATE or DELETE
without WHERE", and the "Write blocked" card of the "read-only connection"
error artboards (macOS and Omarchy). They are not copied into the
repository.

## Decisions

| Question | Decision |
|---|---|
| Who may write | A SQL tab in Read-write, on a writable connection. Never on a read-only one. |
| Commit model | One transaction per run. Committed when every statement succeeded, rolled back on the first error, cancel or timeout. |
| Which runs write | A run is read-write only when the tab is in Read-write mode and one of its statements looks like a write. Every other run is today's read-only run, unchanged. |
| A write taken for a read | The database refuses it in the read-only transaction, and the card offers to run it again read-write. |
| New tabs | They follow the connection: Read-write where its editors can write, Read-only everywhere else. On production always Read-only, to be switched by the user. `editor.sql_new_tab` can say Read-only for all of them. |
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

`SqlTab` gains `mode: RunMode`, `ReadOnly` or `ReadWrite`. A new tab
follows the connection (`RunMode::of_new_tab`): it starts `ReadWrite`
where an editor of its workspace can write when it opens
(`Workspace::sql_writes`), and `ReadOnly` everywhere else; with
`editor.sql_new_tab = "read-only"` it starts `ReadOnly` everywhere. The
user switches it with the toolbar's switch or `Mod+Shift+M`.

Production is the exception: a tab there starts `ReadOnly` on a writable
connection too, whatever `editor.sql_new_tab` says, and the user switches
it to `ReadWrite`. The rule is the environment's own,
`Environment::read_only_by_default`: where connections open read-only
unless the user says otherwise, so do their editors. Until the production
confirmation is built the switch there does nothing, as before. A setting
of its own for production's new tabs may follow later. None is planned.

The mode is given once, when the tab opens: a tab opened on a read-only
connection stays in `ReadOnly` when the session takes writes later, and
only the tabs opened from then on start in `ReadWrite`.

So on a writable connection that is not production's, a statement that
changes data is committed from the first run of a new tab, with no switch
before it. Until `editor.confirm_unsafe_writes` is built, that includes an
`UPDATE` or a `DELETE` without a `WHERE`.

The mode that counts is the effective one: `ReadWrite` only while the
workspace's `access` is `Writable`. A tab whose session comes back
read-only (the box was turned on meanwhile) runs read-only and shows the
read-only connection's switch; its own mode is kept and counts again if
the session is writable once more. On a read-only connection the key and
the switch do nothing.

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
`DESCRIBE` and `DESC` count as `EXPLAIN` (MySQL takes all three), and
`ANALYSE` as `ANALYZE` (PostgreSQL takes both).

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
        /// (MySQL's non-transactional tables), or why the run cannot say
        /// that it did (a MySQL DDL statement that failed, which no
        /// transaction holds). With it, `RolledBack` and `Partly` no
        /// longer say that the rest is gone.
        pub rollback_warning: Option<String>,
        /// A `Write` run only: the session could not be put back after
        /// the run and must be closed. `end` still holds.
        pub broken: Option<Error>,
    }
    pub enum ScriptEnd {
        /// The transaction was rolled back: nothing the run did to a row
        /// or a table remains. Every read-only run ends so.
        RolledBack,
        /// Every statement's work is written.
        Committed,
        /// The database committed on its own before the run failed or was
        /// stopped: the first `committed` statements are written, the rest
        /// is not.
        Partly { committed: usize },
        /// The commit itself failed, and what it would have kept is
        /// rolled back. `committed` counts what the database had committed
        /// on its own before that, as `Partly` does: 0 everywhere but on
        /// MySQL, and then nothing is written.
        CommitFailed { error: Error, committed: usize },
    }

`StatementOutcome::Done` gains `warnings: u16`, which only MySQL fills.

`ScriptMode::Write` on a `ReadOnly` connection returns `Error::ReadOnly`
without contacting the server, whatever the script holds, an empty one
included (the variant comes with `Connection::write`). On any other call
an empty list of statements is `Ok` with no results, as today.

### What holds in both modes

- The refusal check runs first and a refused statement fails the whole run
  with nothing sent. The list is the core spec's, unchanged. The sentences
  that name the transaction follow the mode. Read-only: "Tabletist runs
  every query in a read-only transaction, so COMMIT is not allowed". Write:
  "Tabletist runs and commits the script in one transaction of its own, so
  COMMIT is not allowed". `Error::Refused` carries the mode for it. The
  drivers' own "could not start (or end) the read-only transaction" say
  "the transaction" in a `Write` run.
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
  as one that may be. The end is known once the `COMMIT` or the `ROLLBACK`
  was answered. A connection lost before that, or a `ROLLBACK` that fails,
  stays the run's `Err`: no end is made up for it.

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
   commit's own work, where it fails the commit: the run then ends as a
   stopped one, rolled back. It never leaves a commit that failed and is
   written. A transaction found failed with every statement done (nothing
   a statement of the run can do unseen) is `CommitFailed` too, never a
   `COMMIT` sent and believed.
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
   mysql_async empties what it held, so only a new query tells. One case
   reads the other way: a deadlock, and a lock wait the server is set to
   answer by rolling back, make the server roll the whole transaction
   back, which also leaves the session outside one. The statement tells
   the two apart, not the error's code: one that reads or changes rows
   (`SELECT`, `INSERT`, `UPDATE`, `DELETE`, `REPLACE`, `WITH`, `VALUES`,
   `TABLE`), or sets, shows or explains (`SET`, `DO`, `SHOW`, `EXPLAIN`,
   `DESCRIBE`, `DESC`), never makes the server commit, so "outside" after
   such a statement failed notes nothing. The work since the last noted
   commit is gone, and commits noted earlier stand. A DDL statement that
   loses a deadlock has still committed what came before it. A stop
   between two statements is asked the same question, since the check
   that would have seen a commit by the statement before it does not run.
   A statement that can make the server commit is held by no transaction
   itself: when it fails or is stopped, part of it may be applied (`DROP
   TABLE here, missing` drops `here` where DDL is not atomic). The run
   then says so in `rollback_warning`, so that nothing reads "the rest
   was rolled back".
4. With every statement done: `COMMIT`, after `stop.finish()`, and
   `ScriptEnd::Committed`. A `COMMIT` that fails is `CommitFailed`, after a
   `ROLLBACK`, with the count of what the server had committed by itself
   before it. A `COMMIT` answered with a cancel is followed by the same
   question as step 2: still inside the transaction, nothing was committed
   and the run ends as a stopped one; outside it, the server does not say
   which way the commit went, and the run is an `Err`. A stop that came
   while the last statement ran is asked the question too, in place of
   the `COMMIT`: outside a transaction with every statement done, that
   statement made the server commit, nothing is left to roll back, and
   the run is `Committed` though it was stopped.
5. After an error or a stop: `ROLLBACK`. With nothing noted the end is
   `RolledBack`, otherwise `Partly { committed }`. When the `ROLLBACK`
   reports a warning, the driver reads it with `SHOW WARNINGS` before
   anything else and puts its text in `rollback_warning` (1196: changes to
   non-transactional tables could not be rolled back). It asks once: after
   a `SHOW WARNINGS` that failed a second would show that failure, so a
   sentence saying the text could not be read stands in.
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
   denies what it denies a read-only run: transaction and savepoint
   statements, `query_only`, `writable_schema` and `locking_mode` with a
   value, `wal_checkpoint`. What lets the statement write is `query_only`
   being off.
3. Before every statement the driver asks whether its transaction is still
   open. One that is gone ends the run with `Error::LeftTransaction`.
4. `COMMIT` after the last statement, `ROLLBACK` after an error or a stop.
   Before the `COMMIT` the driver asks once more whether its transaction
   is open: a last statement that ended it is `Error::LeftTransaction`
   too. A rollback that fails with the transaction still open is the
   run's `Err`.
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
- A driver calls `stop.finish()` and then reads the stop flag, before it
  sends `COMMIT`. A stop set before that wins over the commit. From then
  on no cancel is sent, and the run ends as the commit ends.
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

- macOS and Windows, writable connection: a segmented switch, "Read-only"
  and "Read-write", as the design's Components draw one: a sunken track
  with the chosen segment raised. The chosen Read-only stands behind a
  lock; the chosen Read-write reads in the warning tone. A click on a
  segment sets the mode, and with the keyboard on the switch the arrows
  do. Each segment says on hover what its mode does: "Runs are rolled
  back. Nothing is changed." and "A run that changes data is committed
  when every statement succeeds."
- Where no editor can write (a read-only connection, and production until
  its confirmation is built): the switch is drawn as a control that is
  off, with Read-only chosen. Its Read-write segment cannot be picked and
  says why, to a screen reader, to the keyboard while it is on it, and on
  hover: "This connection opens read-only." or the sentence about
  production.
- Omarchy: the same two segments before `· limit 1000 · timeout 30s`, the
  chosen one in a box and the other muted, `read-write` in the warning
  colour when it is chosen.
- `Mod+Shift+M` switches the mode of the active SQL tab. It is in the
  shortcuts table as "Read-only or read-write runs in the SQL editor".
- The toolbar gives way in the core spec's order: the switch goes after
  the run buttons' keys.
- Not built from those artboards: the "Auto-commit" and "Transaction"
  control beside the switch, the bar of an open transaction with Commit
  and Rollback, and the key the Omarchy artboard writes beside the switch
  (`ctrl+w` closes the tab here).

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
    database's error as a statement's error is shown. With a count above
    0 (MySQL), `Partly`'s line stands in place of "Nothing was written."
  - With a `rollback_warning`, in place of the `RolledBack` line and of
    `Partly`'s last sentence: "MySQL could not roll back every change.",
    then its text. Nothing then says "Nothing was written".
  - With `broken`: the end's own line, then "The session could not be put
    back and was closed.", and the reconnect banner.
  - A stop that came with every statement done has no statement's line to
    say it: "Cancelled" (or the timeout's words) then stands before the
    end's line.
  - A commit that failed counts beside the Messages tab as a statement's
    error does.
- In a run that did not end `Committed`, a statement whose work was undone
  ends its line with "· rolled back", so no count reads as a change that
  stayed: a statement without a result set, and one that returned rows
  and looks like a write (`INSERT ... RETURNING`). With a
  `rollback_warning` no line gets it.
- Results shows the last statement that returned rows, as today. A
  statement without a result set shows "Statement ran · 12 rows affected"
  when it counted rows, else today's "Statement ran · no rows returned". A
  statement counts rows where it does today (`WITH ... UPDATE` and `CREATE
  TABLE ... AS` do not). After a commit that failed, with no rows to show,
  Results says what the Messages say of it ("The commit failed. Nothing
  was written."), never that a statement ran. After a run that MySQL
  committed by itself under a stop that came too late, Results says that
  the statement ran, not only "Cancelled": the run is written.
- Messages opens by itself as today, and also when a read-write run ends
  `Partly`, `CommitFailed`, with a `rollback_warning` or `broken`.
- Footer, macOS and Windows: the shown statement's `12 rows affected ·
  14 ms`, as it shows a result's rows today, then `Read-write transaction ·
  committed`, `· rolled back`, `· partly committed`, `· commit failed`,
  or, with a `rollback_warning`, `· not fully rolled back`. A commit that
  failed after MySQL had committed part by itself reads `· partly
  committed`, which is what it left. Omarchy:
  `ln 3:1 · 12 rows affected · 14 ms · committed`.
- A connection lost during a read-write run shows the reconnect banner and
  "The connection was lost during a read-write run. Some or all of it may
  be written." A session closed because the script ended its own
  transaction (`Error::LeftTransaction`) says that itself, and under it
  "Some or all of the run may be written."

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

- "Allow writes in this tab" sets the mode and runs nothing. The card
  stays, for a run that is now one of a tab in Read-write: "This run was
  read-only", "It was sent before this tab could write, so PostgreSQL
  refused the UPDATE. Nothing changed.", and **Run in a read-write
  transaction**. Its statements held a write, so "looked like reads"
  would not be true of it.
- Edit connection opens the connection dialog on the workspace's saved
  connection (`Action::EditConnection`).
- The Omarchy keys work while the editor does not have the keyboard, and
  only while the card is on screen: not under the opening screen a switch
  of database puts over the editor.
- The card's button and its letter take a press of their own. "Allow
  writes in this tab" gives way to "Run in a read-write transaction" in
  the same place and under the same letter, so the repeats of a held key
  and the second click of a double-click answer nothing.
- "Run in a read-write transaction" is offered only on a connected
  session.
- Under the card stand the database's own words, then, for a read-only
  connection, the line "To write, turn off Open read-only ...", then the
  action. The terminal look writes the card in lower case throughout, the
  database's name and the statement's verb too, and names a read-only
  connection by its tag, as its artboard does: "PROD blocks writes, so
  postgresql refused the update. nothing changed." It writes the lines of
  a run's end the same way.
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
| `editor.sql_new_tab` | string | `"connection"` | `"connection"` or `"read-only"`. Any other value is an invalid line. On production a new tab is in Read-only under both. |
| `editor.confirm_unsafe_writes` | boolean | `true` | The question about `UPDATE` and `DELETE` without `WHERE`. |

Both are `#[serde(default)]` in `Settings` and reload live with the file.
`sql_new_tab` reaches the SQL tabs opened afterwards, as `sql_limit` does;
`confirm_unsafe_writes` is read when a run is decided.

## The promise, restated

On a read-only connection no action in the app can modify data. On a
writable connection only Save can, and a run in a SQL tab in Read-write;
browsing, a raw WHERE and every other run still
cannot.

## Errors and edge cases

- "Rolled back" and "Nothing was written" speak of what a transaction
  holds: rows, and tables where DDL is transactional. That is the promise
  the read-only run makes too. What a database keeps outside its
  transactions stays as the statements left it, whatever the run's end: a
  sequence `nextval` advanced or an insert drew from, an auto-increment
  counter. No client undoes those, a run cannot tell which statements
  touch them (a column's default does), and the app does not warn of them.
- A statement's error, its line and its position read as today. After it
  nothing more runs and the transaction is rolled back.
- The timeout cancels a read-write run and rolls it back: "Cancelled after
  30 s (timeout)", "Rolled back. Nothing was written." A long change needs
  a longer timeout.
- A cancel that arrives after the commit was sent does nothing. The run
  ends committed and says so.
- Session settings a script changes last only for the run, in both modes.
  A read-write run on PostgreSQL resets them itself. SQLite is the
  exception it has always been: a script's `PRAGMA`s, beyond the few that
  are put back, and its `ATTACH`es last for the session, as the core spec
  says. Some of them change what a later run that writes keeps
  (`ignore_check_constraints`, `recursive_triggers`, `legacy_alter_table`,
  `locking_mode`), and a script may set them in a read-only run. Step 2
  settled them before any tab could write. The first three are put back
  after every run, with the other connect-time settings: a script may set
  one for its own run, and no later run or save feels it. `locking_mode`
  with a value is denied to a script, as `query_only` is, and fails with
  SQLite's "not authorized": in a rollback journal an exclusive lock is
  let go only by the session's next read of the file, not by setting the
  mode back, and inside the one transaction of a run it gains nothing.
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
2. **The tab.** The mode, the toolbar's switch and `Mod+Shift+M`,
   `editor.sql_new_tab`, the decision in `RunSql`, the production
   confirmation in both looks, the three cards, Messages, Results and the
   footer for a read-write run, and Run held back while one is in flight.
   It also settles the SQLite pragmas that outlive a run (see "Errors and
   edge cases"), since it is the step that lets a tab write.
   It reuses the production sheet and the PROD box of value editing's step
   3, so it is planned once that has landed. It is built in two runs.
   The first leaves out the production confirmation and
   `editor.sql_new_tab`, and so lets no tab of a production connection
   write: there the switch's Read-write segment cannot be picked,
   the key does nothing, and the segment, the switch's tooltip and the card
   of a tab in Read-only say "Read-write runs on a production connection are
   not available yet." in place of the way on. So does the card of a
   production connection that opens read-only, in place of "To write, turn
   off Open read-only ..." and **Edit connection**: with the box off its
   tabs still could not write. The second run builds the confirmation and
   takes that sentence out. Until step 3 of this spec,
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
  - a stop set before the commit rolls back;
  - after a run, and after one that failed or was cancelled, the session
    is as it connected: PostgreSQL `search_path`, role, a `WITH HOLD`
    cursor and an advisory lock; MySQL `time_zone` and the read-write
    setting; SQLite `query_only`. A read-only run right after cannot write.
- PostgreSQL: a deferred constraint that fails at `COMMIT` is
  `CommitFailed` with nothing written; a script that ends the transaction
  past the refusal (forced in the test) ends `LeftTransaction`, with the
  statement after it never run; a `SELECT` that calls a writing function
  over more rows than the limit writes for every row; a data-modifying
  `WITH` that returns rows runs and is committed; a transaction that
  failed behind the run is `CommitFailed`, not committed. That an outcome
  keeps its end when the session could not be put back is a unit test of
  the one function all three drivers end a `Write` run with.
- MySQL: `CREATE TABLE` between two inserts with a failing statement at
  the end is `Partly` with the right count; a failing `CREATE TABLE` still
  counts what came before it; a failing statement with no DDL before it
  is `RolledBack`, not `Partly`; two sessions that deadlock end the
  losing run `RolledBack`, with the probe table untouched, and after a
  `CREATE TABLE` earlier in it `Partly` with only what preceded the commit;
  a stop right after a statement that committed is `Partly` too;
  an `INSERT ... SELECT` and a `CREATE
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
- Headless UI tests, in every look: the switch and the key; the
  three cards and their actions, in Messages, after a statement that
  returned rows too; the production confirmation, with
  `write` typed on Omarchy; the question about a missing `WHERE`, alone
  and inside the production sheet; Messages, Results and the footer for
  each `ScriptEnd`; the switch on a read-only connection.
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
