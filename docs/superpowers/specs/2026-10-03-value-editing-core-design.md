# Editing values, slice 1: the core and its safety surfaces

Date: 2026-10-03. Status: design, not yet planned.

## Intent

Tabletist reads data and cannot change it. This slice lets a user change
the values of existing rows in a table's grid: edit cells, see them pending,
review the SQL, and save them in one transaction that never overwrites a row
someone else changed.

Success: on each driver, a user opens a table on a writable connection,
edits cells in several rows, reads the statements that will run, saves, and
sees the values the database now holds; a value the column cannot take is
caught before anything is sent or comes back as the database's own message
with nothing applied; a row that changed on the server since it was loaded
is never overwritten without the user choosing to; and on a read-only
connection nothing in the app can change data, as today.

The designs are the "Editing values" artboards (macOS flow, macOS editors by
type, Omarchy) and the "Changes to shipped screens" artboards in the design
canvas Artifact. They are not copied into the repository.

## Editing as a whole

The designs describe more than this slice. Editing is split into
sub-projects, each with its own spec, plan and pull request:

1. **Core and safety surfaces** (this spec): writable connections, editing
   cells as text, pending changes, one-transaction save, Review SQL, the
   production confirmation, conflicts.
2. Editors by type: enum and CHECK pickers, boolean cycling, foreign key
   search, the calendar, JSON highlighting, array chips, binary from a
   file, `DEFAULT`.
3. The row panel (the design's inspector) as a row form.
4. Power keys: undo and redo, pasting a TSV block, `.`, the rest of the vim
   set, `$EDITOR`.
5. Rows: add, duplicate, delete (the "Editing a row" artboards).
6. Writes from the SQL editor, and allowing writes for one tab of a
   read-only connection.

## Decisions

| Question | Decision |
|---|---|
| Scope | Values of existing rows, edited in the grid. Slices 2 to 6 follow separately. |
| Who may write | A connection whose saved "Open read-only" box is off. On for production by default, off elsewhere. Fixed when the session connects. |
| Session | A writable connection gets a read-write session. Browsing keeps its explicit read-only transactions, and the script runner fences itself. |
| Commit model | Edits are pending in the tab until Save; Save writes every pending change of the tab in one transaction. |
| Stale rows | Inside the transaction each edited row is locked and read by its key, and the changed columns are compared with what the page loaded. A difference writes nothing. |
| Row identity | The primary key, else a unique index whose columns are all NOT NULL. No such key: the table is view-only. |
| Editors | Text only: on the cell, or in a popover for long, multi-line and JSON values. |
| Confirmation | Only on a production connection: the statements are shown and the save is confirmed (Omarchy: by typing `write`). |
| Omarchy keys | `i` and Enter edit the cell. The row panel keeps Space and `Mod+Shift+R`. |

Rejected: sessions that stay read-only with one read-write transaction for
the save (chosen against: a writable connection is meant to grow SQL editor
writes in slice 6); the old values as a guard in the `WHERE`, as one
artboard draws it (SQL equality is not reliable for every type: PostgreSQL
`json`, `xml` and geometric types have no `=`, MySQL compares a `DECIMAL`
with a string as floats, and a miss shows as a conflict that Overwrite
cannot get past); last write wins; writing a JSON edit as `jsonb_set(...)`,
as one artboard sketches (PostgreSQL only, and it needs a JSON diff);
locking a column the user has no `UPDATE` privilege on before the save
(per-driver privilege queries; the save's failure says it instead);
editing auto-updatable views.

## Out of scope

Everything in slices 2 to 6. Editing key columns, binary values, values
over 256 KiB, views, materialized views and SQL results. Editing in the
row panel: its Edit, Duplicate and Delete buttons stay disabled and say
"arrives in a later version". Rows of a page that is no longer loaded:
pending changes never outlive their page.

## Writable connections

### The connection dialog and the shipped screens

- "Open read-only" is no longer locked. Its default is what
  `SavedConnection::read_only()` computes today: on for production, off for
  every other environment, until the user sets it. Its note reads "Blocks
  every write from this app. On by default for production; turn off to
  edit." (Omarchy: "block every write from this app · default for
  production").
- A saved connection that never chose and is not production therefore
  becomes writable on its next connect after the upgrade.
- The access is fixed when the session connects. `Workspace` keeps it
  (`access: Access`); a changed box applies from the next connect or
  reconnect.
- Every "read-only" mark the app draws (the header's pill or tag, the
  footer's "· read-only", the Omarchy status line's tag) shows only for a
  read-only connection.
- A write the SQL editor refuses reads the same on every connection:
  "The SQL editor only reads data", "Every query runs in a read-only
  transaction, so this statement was refused. Nothing changed.", and the
  database's own words. The artboard's card for a read-only connection
  ("This connection opens read-only", "turn off Open read-only", **Edit
  connection**, the per-tab switch) belongs to slice 6: until the SQL
  editor can write, turning the box off would not make the statement run,
  and the card must not say it would. From step 3 on, the card adds "Edit
  values in a table's grid." on a writable connection.

### Sessions (`tabletist-db`)

    pub enum Access { ReadOnly, Writable }

    Connection::connect_with(spec, secrets, host_keys, access)

`Access::ReadOnly` is every session as it is today, unchanged. A
`Writable` session differs only in this:

- PostgreSQL: `SET default_transaction_read_only = on` is not sent
  (`standard_conforming_strings` still is). Row fetches, counts and the
  script runner already open explicit `READ ONLY` transactions, and the
  script guard's snapshot, savepoint and final check do not read the
  session default.
- MySQL: `SET SESSION TRANSACTION READ ONLY` is not sent. Row fetches and
  counts already run in `START TRANSACTION READ ONLY`. The script runner
  does lean on the session setting: MySQL commits implicitly before DDL,
  and only the session's read-only setting refuses the DDL that follows;
  its checks read `@@session.transaction_read_only`. So a script run on a
  writable session sends `SET SESSION TRANSACTION READ ONLY` before it
  begins, and its cleanup, which already replays the connect-time
  statements after the reset, ends with `SET SESSION TRANSACTION READ
  WRITE`. A cancel meant for a statement can land on either `SET`, so both
  are sent again when interrupted, as the connect-time statements are. A
  cleanup that fails closes the session, as today.
- SQLite: the file is opened `SQLITE_OPEN_READ_WRITE`, never
  `SQLITE_OPEN_CREATE`. `PRAGMA query_only = ON` stays the session's
  standing state (it is set at open and again after every script); only a
  save turns it off, for its own transaction, and turns it back on on every
  path. A file the operating system protects still opens, and a save to it
  fails with SQLite's message. Opening read-write may create `-wal` and
  `-shm` files or recover a journal.

  The read-only open was the script guard's independent layer on SQLite,
  and the SQL editor spec says SQLite "has no such mode to leave, so it
  needs no check". On a writable session that is no longer true, so the
  guard gains these there (they run for both accesses: the refusal knows
  the dialect, not the session):
  - The refusal list refuses `PRAGMA query_only` and `PRAGMA
    writable_schema` whenever the statement sets them. SQLite takes a
    pragma's name as an identifier, a quoted identifier or a string
    literal, with or without a schema in front, and a value after `=` or
    in parentheses; the match covers all of those spellings
    (`PRAGMA 'query_only' = 0`, `PRAGMA main."query_only"(0)`), so it
    reads the name token itself and not only `sql::words`, which skips
    strings.
  - It also refuses `PRAGMA wal_checkpoint` in every form, the bare one
    included: `query_only` does not stop a checkpoint, which rewrites a
    file in WAL mode.
  - SQLite reads a byte-order mark at the start of a token as whitespace,
    so the tokenizer does too for SQLite. Read as part of a word it hid
    `COMMIT` and `PRAGMA` from the refusal list.
  - Two checks before every statement, as PostgreSQL and MySQL have. The
    runner reads `PRAGMA query_only`, and asks whether its transaction is
    still open: `query_only` stops writes to tables, but only the open
    transaction stops `PRAGMA journal_mode = WAL` and `VACUUM INTO`. Either
    check failing ends the run with `Error::LeftReadOnly`, after the
    `ROLLBACK` that always happens. `query_only` is read once more before
    that rollback.

  Browsing needs a fence of its own on SQLite. rusqlite finds a second
  statement in a text by preparing it, and SQLite applies a flag pragma
  when it prepares one, so a raw WHERE holding `; PRAGMA query_only = 0;`
  fails as it should and still turns the setting off. So a raw WHERE that
  holds a `;` token is refused before anything is prepared, and a row
  fetch or count that fails puts the session's settings back. A raw WHERE
  holding a NUL is refused too: SQLite stops reading there, which dropped
  the page's ORDER BY, LIMIT and OFFSET.

  Every check above that reads text trusts our tokenizer to agree with
  SQLite's, and review kept finding places where it does not (a pragma
  name written as a string, `EXPLAIN` in front, a byte-order mark,
  SQLite's variable tokens such as `:a(')`). So SQLite's own parse is
  the backstop: the connection has an **authorizer**, which SQLite
  consults whenever it prepares a statement, the tail rusqlite prepares
  to detect a second statement included. It knows whose text is being
  prepared:
  - the app's own statements (the transaction around a script, the
    session's settings, the catalog, later the save): everything is
    allowed;
  - a script's statement: transaction and savepoint statements are
    denied, and so are `query_only` and `writable_schema` with a value
    and `wal_checkpoint` in any form. Writes are left to `query_only`,
    whose error the refused-write card recognises. `ATTACH` and other
    pragmas stay, as the SQL editor spec allows them;
  - a table's page or count, which holds the raw WHERE: a pragma with a
    value is denied, and so are transaction and savepoint statements,
    `ATTACH` and `DETACH` (also with a computed name). A statement hidden
    behind the filter is only ever prepared, never run, so what matters
    is what takes effect at prepare time, and that is a pragma with a
    value. A pragma without one changes nothing when it is prepared, and
    SQLite's own virtual tables need them (FTS5 asks for `data_version`).
    Those that act when run are never run, except `optimize` read as a
    table, which `query_only` refuses; `wal_checkpoint` and
    `incremental_vacuum` are denied by name all the same, in case a later
    SQLite makes them readable as tables. R*Tree prepares its write
    statements when a table is first touched, so writes are not denied
    here: a write is `query_only`'s to refuse, as in a script. A filter
    the authorizer denies says so in a sentence ("A filter cannot use a
    PRAGMA with an argument, ATTACH or a transaction statement."), since
    an honest one can reach it with `pragma_table_info('users')`.

  The refusal list and the two checks stay, as the layers in front of
  and behind it: the list gives the user a sentence instead of "not
  authorized", and the checks hold if the authorizer is ever wrong.

The promise, restated: on a read-only connection no action in the app can
modify data. On a writable connection only Save can; browsing, a raw WHERE
and the SQL editor still cannot.

## What can be edited

- **Tables with a row key.** The key is the primary key, else the first
  unique index (by name) that is not partial and whose entries are all
  columns of the table (no expressions; a name the catalog gives quoted is
  matched unquoted) and all NOT NULL. The catalog gains
  `IndexInfo.partial: bool` (PostgreSQL `indpred`, SQLite `index_list`'s
  `partial`; never on MySQL). A table without a key is view-only, and its
  cells say "<table> has no primary key or unique index, so a row can't be
  targeted safely". Until the tab's structure has loaded, nothing is
  editable.
- A row whose key holds a NULL (a SQLite primary key that is not an
  integer can) is locked: "this row's key is NULL".
- **Never editable:** views, materialized views, SQL results, and anything
  on a read-only connection ("This connection opens read-only").
- **Locked cells** in an editable table:
  - identity-always and generated columns. The catalog gains
    `ColumnInfo.generated: bool` (PostgreSQL `attgenerated` and
    `attidentity = 'a'`, MySQL `EXTRA`, SQLite `table_xinfo`'s hidden
    values 2 and 3);
  - the row key's own columns, so a save finds and re-reads the row by the
    same key;
  - binary values, and values over 256 KiB.
- A locked or uneditable cell is drawn as the design's "locked" (macOS and
  Windows) and looks as today on Omarchy. Enter (and `i` on Omarchy) on it
  says why, in a note at the cell or in the Omarchy mode line. Typing on it
  does nothing.

## Editing in the grid

### The cell lifecycle

1. **Active.** The selected cell. Enter, F2 or a double-click opens the
   editor on the value, the cursor at its end. On macOS and Windows typing
   a character opens it with that character as the new text. Editing pins
   a preview tab.
2. **Editing.** The editor sits on the cell, at the cell's size.
3. **Pending.** The cell shows the new value in amber with the left bar;
   hovering it shows "was <loaded value>". Nothing has been sent.
4. **Saving.** Pending cells are locked and the grid stays usable for
   looking. The actions the leaving guard covers are disabled until the
   save ends.
5. **Saved.** Green for 1.2 seconds, then the value as re-read from the
   database.
6. **Failed.** The cells of the row whose statement failed turn red with
   the database's code and message; the other pending cells stay amber.
   Nothing was applied.

### Editors

Every editable type is edited as text.

- **On the cell:** one line, for a value without line breaks of at most
  256 characters (the grid's cut) in a column that is not JSON.
- **In a popover anchored to the cell:** every other value, and every JSON
  column. It shows the character and line count. `Mod+Enter` applies and
  Esc cancels. `Alt+Enter` in the one-line editor adds a line break and
  moves the text into the popover.
- The editor starts from the value's full text as the database gave it,
  never from the shortened text a cell or the row panel shows.
- **NULL:** `Mod+Backspace` on the active cell of a nullable column, while
  no editor is open, makes the cell NULL. Inside an open editor the key
  stays the text field's own. Opening the editor on a NULL cell starts
  empty. Clearing the text gives the empty string, never NULL.
- A cell is pending when its new value differs from the loaded one: typing
  the loaded text back, or NULL on a cell that was NULL, takes it out of
  the pending set.

### Checks before sending

Checked as the user types. The column's class comes from one function in
`tabletist-db`, from the dialect and the structure's `type_name`
(`numeric(10,2)`, `varchar(200)`, `bigint unsigned`); a type it does not
know has no check. SQLite enforces neither ranges nor lengths, so there
only the integer (as `i64`), float, boolean, CHECK-list and JSON rules
apply, by the column's affinity.

| Column | Rule | Message |
|---|---|---|
| Integer | A whole number within the type's range | "int8 expects a whole number" |
| Decimal | A number, within the digits and scale the type states | "Up to 2 decimals. 12.505 would be stored as 12.51." |
| Float | A number (`NaN` and `Infinity` on PostgreSQL) | "float8 expects a number" |
| Boolean | `true` or `false`; also `1` or `0` | "boolean expects true or false" |
| Enum, CHECK list | One of `allowed_values` | "Not one of: print, ebook, audio" |
| JSON | It parses | "Expected , or } at 3:23" |
| Text with a length | Within the length, shown as `27 / 200` | "At most 200 characters" |
| NOT NULL | Never NULL | the NULL key does nothing |

A boolean column's editor starts from `true` or `false` whatever the
driver loaded (SQLite and MySQL hold 1 and 0).

Every rule blocks: a decimal with more digits than the scale is refused
with what the database would have stored, never rounded silently. Every
other rule is the database's, and its rejection is the Failed state.

While the text fails its check the editor is red and shows the message;
Enter, Tab and `Mod+Enter` do not leave it. Clicking elsewhere keeps the
text as a pending cell marked "to fix", so typing is never lost. Save is
disabled while any cell is to fix, with "Fix 1 value to save". A failed
cell does not block: it is an ordinary pending cell that carries the
database's message until it is edited or saved, and the next Save sends it
again.

### Pending changes

- `ObjectTab` holds the pending set of its loaded page: per cell (row
  index, column) the new value (`Text` or `Null`) and its state (pending,
  to fix, failed with a message), and the editor that is open, if any. Only
  the text being typed lives in the field itself.
- **The bar** above the footer, shown while the set is not empty: "3
  changes in 2 rows", "1 to fix", **Review SQL**, **Discard all**, **Save**
  (`Mod+S`). Omarchy shows the same counts in its mode line.
- The tab carries the unsaved dot (`[+]` on Omarchy). A changed row carries
  the gutter mark: `~`, or `!` when one of its cells is to fix or failed.
- **Revert one cell:** `Mod+Z` on a pending cell that is active puts back
  the loaded value. The undo and redo stack is slice 4.
- The row panel shows a pending cell's new value with the pending mark and
  "was <loaded value>", so it never disagrees with the grid. It stays
  read-only.

### Keys

| | macOS, Windows | Omarchy |
|---|---|---|
| Edit the cell | Enter, F2, double-click, typing | `i` and Enter (cursor at the end), `cc` (replace) |
| Commit and move down | Enter | Enter |
| Commit and move right, left | Tab, Shift+Tab | Tab, Shift+Tab |
| Leave the editor | Esc drops the edit | Esc keeps it, Ctrl+C drops it |
| Set NULL | `Mod+Backspace` | `x` |
| Revert the cell | `Mod+Z` | `u` |
| Review SQL | the bar's button | `:diff` |
| Save all | `Mod+S` | `:w`, Ctrl+S |
| Discard all | `Mod+Alt+Backspace` | `:e!` |

- Typing opens the editor on macOS and Windows except for Space (the row
  panel) and `?` (the shortcuts), which keep their meaning.
- On Omarchy `i` and Enter no longer open the row panel; Space and
  `Mod+Shift+R` do. `s` stays Structure. In insert mode the mode line reads
  `-- INSERT --`, the column and its type, the pending counts, and "esc
  normal · tab next cell"; Esc there leaves insert mode and does not close
  the row panel.
- Omarchy gains a `:` prompt in the mode line. It takes `w`, `diff` and
  `e!`, and nothing else in this slice; any other text is "not a command".
- All of it is handled in `ui/keys.rs` and listed in the shortcuts table.

### Leaving with pending changes

- **Guarded actions**, the ones that drop the page or the tab: previous and
  next page, sorting, applying or clearing filters, refresh, closing the
  tab, closing or disconnecting the connection, switching database, and
  closing the window.
- The action is held and a prompt asks. When one tab is affected: **Save**,
  **Discard**, **Cancel** (Omarchy: `[w]` write, `[d]` discard, `[esc]`
  stay). Save runs the save and performs the held action only if everything
  was written; a failure, a conflict or a cancelled production confirmation
  drops the held action. When several tabs with pending changes are
  affected (a connection, the window): **Discard** and **Cancel** only.
- Not guarded: switching tabs or connections, and the Data and Structure
  switch. Pending changes wait in their tab.
- **A dropped connection never costs the pending set.** While the session
  is disconnected the set is kept and Save is disabled with the reason.
  Reconnecting is not guarded: a tab with pending changes keeps its loaded
  page through the reconnect (it is not fetched again, as other tabs are),
  and Save works afterwards. The page may be stale by then; the save's
  check against the loaded values covers that. The same holds after a
  script closed the session and after a connection lost while saving. If
  the reconnect comes back read-only (the box was turned on meanwhile),
  the set is still kept and Save is disabled with "This connection opens
  read-only".
- A guarded action's prompt has no Save, only **Discard** and **Cancel**,
  whenever Save itself is disabled: while disconnected, on a session that
  came back read-only, and while a cell is to fix.

## Saving (`tabletist-db` and the backend)

    pub struct ChangeSet { pub object: ObjectRef, pub rows: Vec<RowChange> }
    pub struct RowChange {
        /// The row key's columns and their loaded values.
        pub key: Vec<(String, Value)>,
        pub set: Vec<CellChange>,
    }
    pub struct CellChange {
        pub column: String,
        /// The structure's type name, which decides how `new` is sent.
        pub type_name: String,
        pub loaded: Value,
        pub new: NewValue,
    }
    pub enum NewValue { Null, Text(String) }

    pub enum WriteOutcome {
        Written { rows: Vec<Vec<Value>>, elapsed: Duration },
        /// Nothing was written. `server` is the row now, `None` when gone.
        Conflicts(Vec<Conflict>),
        /// Nothing was written: the statement of `rows[row]` failed.
        Failed { row: usize, error: Error },
    }
    pub struct Conflict { pub row: usize, pub server: Option<Vec<Value>> }

    Connection::write(&self, changes: &ChangeSet) -> Result<WriteOutcome>

`write` is the crate's only writing call. On a `ReadOnly` connection it
returns `Error::ReadOnly` without contacting the server, and it refuses a
set with no rows, a row with an empty `key` or an empty `set`, before
anything is sent. An `Err` is a failure of the session or the run (lost
connection, cancelled); nothing was committed unless the connection was
lost while committing.

One transaction, for every row of the set:

1. Begin, read-write (`BEGIN IMMEDIATE` on SQLite, with `query_only` off).
2. For each row, read it whole by its key, locked (`SELECT * ... WHERE
   <key> FOR UPDATE` on PostgreSQL and MySQL). The key is matched with its
   loaded values in the driver's own form (a binary key as its bytes). No
   row is a conflict with `server: None`; more than one is an error.
3. Compare each changed column's value in that row with `loaded`, as
   decoded `Value`s (floats by their bits, so NaN equals NaN). Any
   difference is a conflict carrying the row as read.
4. With any conflict: roll back and return `Conflicts`.
5. For each row, `UPDATE <table> SET <column> = <new>, ... WHERE <key>`.
   It must touch exactly one row on PostgreSQL and SQLite; MySQL reports
   changed rather than matched rows, so there none or one. A statement
   that fails, touches another number of rows, or on MySQL raises a
   warning (a truncated or adjusted value) rolls everything back and
   returns `Failed` with the database's error or the warning's text.
6. Read each row whole again by its key, commit, and return the rows.

New values travel as text and the database converts them to the column's
type: a quoted literal on PostgreSQL, a bound string on MySQL. Where the
database would store the text as it is, the statement builder converts
first, by the column's class, which it takes from `CellChange::type_name`
with the same function the checks use: on SQLite integers and reals are
numbers, and on SQLite and MySQL a boolean is 1 or 0. The builder is the
only place that decides a value's form, so the literal Review SQL shows
(`12`, `1`) is the value the driver binds.

The backend gains `Command::Write { session, request, tab, changes }` and
`Event::Written { .. outcome }`, queued and answered like every request.
`Mod+.` cancels a running save through the session's cancel handle; the
`write` future is awaited to its end, never dropped, and rolls back. Once
`COMMIT` is sent a cancel is no longer honoured. A connection lost during
the save leaves the pending set as it was and says "The connection was lost
while saving. Reload to see what was written."

After `Written` the reducer replaces those rows in the page, in place even
when the sort or the filters would now move or hide them, clears their
pending cells, and formats the row panel's fields again. The status reads
"written 2 changes · 1 row · 14 ms".

## Review SQL

- One `UPDATE` per changed row, by key, with the values as literals, under
  "runs in one transaction". The check of step 3 is a comment line above
  the statement: `-- only if kind is still 'print' and alt_text is still
  NULL`.
- The text is the statement that runs: PostgreSQL sends it as written, and
  MySQL and SQLite send the same statement with the values bound. One
  builder in `dialect.rs` produces both forms from a `ChangeSet`, so they
  cannot drift.
- A row with a cell to fix appears as a comment only: `-- row id 4 ·
  blocked: fix publisher_id first`.
- A literal longer than 60 characters is shortened with `…` where it is
  shown. That is display only.
- macOS and Windows: a drawer above the pending bar, toggled by **Review
  SQL** and **Hide SQL**. Omarchy: the `:diff` panel, closed with Esc.
- The reducer builds the text when the pending set changes; drawing only
  lays it out.

## Saving to production

Only when the workspace's environment is production, every Save first asks:

- macOS and Windows: "Save 2 changes to production?", the connection's name
  and database, "1 row in book_covers", the statements, "One transaction",
  **Cancel** and **Save to production**.
- Omarchy: the red PROD box with the same facts and "sql shown with
  :diff", and a field that takes the word `write`; Enter confirms only when
  it holds exactly that, Esc cancels. A production save opens the `:diff`
  panel if it is closed and the box sits beside it, so the statements are
  on screen while `write` is typed. Until `:diff` exists (step 3), the box
  lists the statements itself in place of that line.

## Conflicts

`Conflicts` opens a dialog for the first conflicting row:

- "Row id 2 changed on the server", "Someone saved it after you loaded it.
  Nothing was written.", and a table of the columns the user changed:
  loaded, now on server, yours.
- Three choices (Omarchy `[k]`, `[s]`, `[o]`):
  - **Keep mine, reload row.** The server's row replaces the loaded row in
    the page and the pending cells stay on top of it. A pending cell whose
    new value now equals the server's leaves the set.
  - **Use server values.** The server's row replaces the loaded row and the
    row's pending cells are dropped.
  - **Overwrite.** As Keep mine, and the save may run again once every
    conflict is answered (see below). A row that changed once more
    conflicts once more.
- With `server: None`: "Row id 2 no longer exists on the server", and one
  choice, **Discard my changes**, which drops the row's pending cells and
  marks the row as gone until the page is reloaded.
- Several conflicts are asked one after another ("1 of 2"). A save writes
  the whole pending set, so it runs again after the last answer only when
  some row was answered Overwrite and none Keep mine: a row the user kept
  to look at again is never written by another row's Overwrite. Otherwise
  what is left stays pending until the user saves. On production that
  second save asks its confirmation again. When the answers left nothing
  pending, no save runs.
- Esc is Keep mine for the row shown.

## Steps

Each step ends compiling, tested and shippable, and gets its own plan run:

1. Writable connections: `Access`, the sessions, the script runner's fence,
   the dialog's box, the shipped screens. Nothing writes yet.
2. `Connection::write`, the statement builder and the column classes, the
   catalog's `ColumnInfo.generated` and `IndexInfo.partial`, the row key
   rule, the backend command and event.
3. Editing in the grid: the lifecycle, the editors, the checks, the pending
   bar without Review SQL, the keys but `:diff`, Save, the leaving guard,
   and the production confirmation with its statements in both looks.
   A conflict here is a plain message: "Row id 2 changed on the server.
   Nothing was written." (or "Row id 2 no longer exists on the server.").
   The pending set stays as it was, with no rebase: the user refreshes,
   which discards it, and edits again.
4. Review SQL: the drawer and `:diff`; the Omarchy PROD box opens the
   panel and points to it.
5. The conflict dialog.

No step ships a production save without its confirmation and its
statements.

## Testing

- `tabletist-db`, each driver, over the shared fixtures:
  - for every column type the fixtures hold, a value that is loaded,
    changed and saved is written, read back, and never conflicts with
    itself;
  - a row changed by another session, a row deleted by another session and
    a failing statement in the middle of a set each leave every row of the
    set untouched and return `Conflicts` or `Failed` with the right row;
  - a MySQL value the server would truncate is `Failed` and not stored;
  - `write` on a `ReadOnly` connection sends nothing;
  - on a `Writable` connection a script and a raw WHERE still cannot
    write: the existing guard tests run again in both modes; MySQL gains
    one for DDL; SQLite gains the authorizer's three modes, raw WHEREs
    that hide a second statement from the tokenizer (they leave every
    session setting as it was), a raw WHERE that tries to turn `query_only`
    off, a script that ends its transaction behind a byte-order mark, the
    refusal of a checkpoint, the refusal of `PRAGMA query_only` in each
    spelling (bare, `"..."`, `'...'`, backticks, brackets, with a schema,
    `=` and `()`), a script that gets `query_only` off by a spelling the
    list misses (forced in the test) ending with `LeftReadOnly` and
    nothing written, and proof that what the read-only open used to stop
    still changes no file from a script: `ATTACH` of a missing file,
    `VACUUM INTO`, `PRAGMA journal_mode`, `PRAGMA wal_checkpoint`;
  - the statement builder's two forms agree for every fixture type.
- Reducer tests: the pending set, the checks, the leaving guard and its
  held action, rows replaced after a save, rebasing after each conflict
  choice, mixed conflict answers, and a tab with pending changes keeping
  its page across a reconnect and saving afterwards.
- Headless UI tests, in every look: the lifecycle, the keys, the popover,
  the bar, the three dialogs, locked cells saying why, the shipped screens'
  new texts.
- `src/shots.rs` gains scenes for review, on the Bookshop demo data, whose
  SQLite file is a throwaway and writable. No test compares a screen with
  the design.

## Documents this changes

The main spec's success criterion 6, its sections 4.3 and 5.7 (Enter and
`i` on Omarchy) and its keyboard table; the SQL editor spec's intent, its
guard layers 2 and 3 and its cleanup list, restated for writable
connections; the crate documentation of `tabletist-db` ("nothing in this
crate writes") and the comment on `RowQuery::raw_where`; the README; the
shortcuts table in `ui/keys.rs`.
