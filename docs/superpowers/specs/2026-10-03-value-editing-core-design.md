# Editing values, slice 1: the core and its safety surfaces

Date: 2026-10-03. Status: all five steps are built: 1 (writable
connections), 2 (the save in `tabletist-db` and the backend), 3 (editing in
the grid), 4 (Review SQL) and 5 (the conflict question), see
`docs/superpowers/plans/2026-10-03-writable-connections.md`,
`docs/superpowers/plans/2026-10-03-connection-write.md`,
`docs/superpowers/plans/2026-10-04-grid-editing.md`,
`docs/superpowers/plans/2026-10-04-review-sql.md` and
`docs/superpowers/plans/2026-10-04-conflict-dialog.md`.

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
3. The row panel (the design's inspector), whose fields edit in place as
   cells do: `2026-10-06-row-inspector-inline-edit-design.md`, which
   replaces `2026-10-06-row-form-design.md`.
4. Power keys: undo and redo, pasting a TSV block, `.`, the rest of the vim
   set, `$EDITOR`.
5. Rows: add, duplicate, delete (the rest of the "Editing a row"
   artboards).
6. Writes from the SQL editor, and allowing writes for one tab of a
   read-only connection. The first is
   `2026-10-05-sql-editor-writes-design.md`, which rejects the second.

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
row panel, which slice 3 has since added
(`2026-10-06-row-inspector-inline-edit-design.md`). Rows of a page that is no longer
loaded: pending changes never outlive their page.

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
  (`access: Access`), as the session reports it once connected. A changed
  box applies from the next connect or reconnect. Only the box is read
  again then: where it was never set, the default is the one of the
  environment the tab opened with, since the tab still points at that
  server and is still drawn in that environment's colour. Relabelling a
  saved production connection does not make its open tab writable.
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
  values in a table's grid." on a writable connection. Slice 6 replaced
  this card with three, at the head of the Messages, each saying why its
  run was read-only (`2026-10-05-sql-editor-writes-design.md`, "The
  refused write").

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
- MySQL: `SET SESSION TRANSACTION READ ONLY` is not sent, and `SET SESSION
  TRANSACTION READ WRITE` is, since a server's own default can be
  read-only. Row fetches and counts already run in read-only
  transactions. The script runner
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
    denied, and so are `query_only`, `writable_schema` and `locking_mode`
    with a value and `wal_checkpoint` in any form. Writes are left to
    `query_only`,
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
modify data. On a writable connection only Save can, and since slice 6 a
run in a SQL tab in Read-write, as a new one there is; browsing, a raw WHERE
and every other run still cannot.

## What can be edited

The rule lives in one function, `edit::Table::lock` (`src/edit.rs`): from
the page, the structure, the connection's access and whether a fetch or a
save runs, it answers why a cell cannot be edited (`edit::Lock`), or that
it can. The reasons that hold for a whole table come first, so every cell
of such a table says the same. The view words the reason
(`cell_editor::lock_text`).

- **Tables with a row key,** which `Structure::row_key` gives. The key is
  the primary key, else the first unique index (by name) that is not
  partial, is over whole columns compared as the column compares, and
  whose columns are all NOT NULL. The catalog says both things about an
  index: `IndexInfo.partial: bool` (PostgreSQL `indpred`, SQLite
  `index_list`'s `partial`; never on MySQL) and `IndexInfo.key_columns`,
  the index's columns by name, or `None` for an index over an expression,
  a prefix (`UNIQUE (name(1))` on MySQL) or another collation than its
  column's: each of those can match two rows. A primary key whose own
  index is such an index is passed over too. SQLite does not say a
  column's declared collation, so there a key of another collation goes
  unseen; the save's own check (it reads the row by its key and refuses
  more than one) is what stops it. A table without a key is view-only, and
  its cells say "<table> has no primary key or unique index, so a row
  can't be targeted safely". Until the tab's structure has loaded, nothing
  is editable ("The table's structure is still loading").
- A row whose key holds a NULL (a SQLite primary key that is not an
  integer can) is locked: "This row's key is NULL".
- **Never editable:** views and materialized views ("Views cannot be
  edited"), SQL results, and anything on a read-only connection ("This
  connection opens read-only").
- **What the save would refuse, the grid locks up front** where it can
  tell from the page and the structure (see "Saving" for why the save
  refuses each):
  - a MySQL table whose key has a `timestamp`, `bit` or `float` column:
    "<table>'s key cannot be matched exactly, so a row can't be targeted
    safely";
  - on SQLite a row whose key text holds U+FFFD: "This row's key holds text
    that was not read exactly";
  - a column whose name the page holds twice, or that the structure does
    not list: "This column cannot be told apart in the table".

  Two cases are still known only when a save is tried: a MySQL table whose
  engine has no transactions (`Structure` does not say the engine), and a
  table of a database attached to a SQLite session. The grid lets them be
  edited and the save refuses them.
- **Locked for a while:** every cell of a tab while its page or its
  structure is being loaded again ("The page is loading": a refresh keeps
  the old page on screen, and what a describe brings may have another key)
  and while a save runs ("A save is running"). Nothing is edited on a page
  about to be replaced.
- **A row that is gone:** every cell of a row a save found gone from the
  server and whose changes the user discarded (see "Conflicts") is locked
  until the page is loaded again: "This row no longer exists on the
  server". The reasons that hold for the whole table or for a while come
  before it, and what the row's values say after it. Its cells are written
  in the dim colour in every look, with no tint and no mark in the gutter.
- **Locked cells** in an editable table:
  - identity-always and generated columns. The catalog gains
    `ColumnInfo.generated: bool` (PostgreSQL `attgenerated` and
    `attidentity = 'a'`, MySQL `EXTRA`, SQLite `table_xinfo`'s hidden
    values 2 and 3);
  - the row key's own columns, so a save finds and re-reads the row by the
    same key;
  - binary values (a column of the class `Binary`, MySQL's `bit` and
    spatial types among them, and any cell that holds bytes, which a
    SQLite column of any type can), and values over 256 KiB.
- **Only a computed column's cells are drawn locked.** In a table that can
  be edited, macOS and Windows draw the cells of a generated or
  identity-always column (`ColumnInfo.generated`) with the surface tint and
  the secondary text colour, also while a save or a fetch runs. Every other
  cell that cannot be edited is drawn as today: a whole table that cannot
  be edited (a view, a read-only connection, no key) would otherwise turn
  grey, and key columns already have their own colour. Omarchy draws every
  such cell as today.
- A cell that cannot be edited says why when it is asked for: Enter, F2 or
  a double-click (on Omarchy `i`, Enter, `cc` or a double-click) puts the
  reason in a note at the cell, or in the Omarchy status line, until the
  selection moves. Typing on it does nothing.

## Editing in the grid

### The cell lifecycle

1. **Active.** The selected cell. Enter, F2 or a double-click opens the
   editor on the value, or on its pending value, the cursor at its end. On
   macOS and Windows typing a character opens it with that character as the
   new text. Editing pins a preview tab, and so does setting a cell NULL: a
   tab that holds edits is never a preview for the next click to replace.
2. **Editing.** The editor sits on the cell, at the cell's size. An editor
   that was opened and closed without typing changes nothing
   (`Editor::touched`): on a NULL cell it starts empty, and the empty
   string is not NULL.
3. **Pending.** The cell shows the new value on an amber tint: with a bar
   at its left on macOS and Windows, with its text in the warning colour on
   Omarchy. Hovering it shows "was <loaded value>". Its row is marked: on
   macOS and Windows, which have no gutter, by a bar at the row's left edge
   and the colour of its key value; on Omarchy by `~` in the gutter.
   Nothing has been sent.
4. **Saving.** Every cell of the tab is locked, the pending ones keep their
   tint and show a spinner, and the grid stays usable for looking. The
   actions the leaving guard covers are ignored until the save ends;
   `Mod+.` or the bar's Cancel stops the save.
5. **Saved.** Green for 1.2 seconds, then the value as the database
   returned it.
6. **Failed.** The cells of the row whose statement failed turn red and
   carry the database's code and message (under the pointer; on Omarchy in
   a line under the grid, `! 2:book_id  23503 ...`); the row's mark turns
   red, `!` in the Omarchy gutter. The other pending cells stay amber.
   Nothing was applied.

### Editors

Every editable type is edited as text.

- **On the cell:** one line, for a value without line breaks of at most
  256 characters (the grid's cut) in a column that is not JSON.
- **In a popover anchored to the cell:** every other value, and every JSON
  column. It shows the character and line count. `Mod+Enter` applies and
  Esc cancels; Enter and Tab are the text's own there. `Alt+Enter` in the
  one-line editor adds a line break at the end of the text and moves the
  text into the popover.
- The editor starts from the value's full text as the database gave it
  (`edit::start_text`), never from the shortened text a cell or the row
  panel shows. On a cell that is already pending it starts from the pending
  value.
- **NULL:** `Mod+Backspace` on the active cell of a nullable column, while
  no editor is open, makes the cell NULL. Inside an open editor the key
  stays the text field's own. Opening the editor on a NULL cell starts
  empty. Clearing the text gives the empty string, never NULL.
- A cell is pending when its new value was typed and differs from the
  loaded one: typing the loaded text back, or NULL on a cell that was NULL,
  takes it out of the pending set.

### Checks before sending

Checked as the user types (`edit::check`). The column's class comes from
one function in `tabletist-db`, from the dialect and the structure's
`type_name` (`numeric(10,2)`, `varchar(200)`, `bigint unsigned`); a type it
does not know has no check. SQLite enforces neither ranges nor lengths, so
there only the integer (as `i64`), decimal, float, boolean, CHECK-list and
JSON rules apply, by the column's affinity. A verdict is data
(`edit::Problem`), which the view words (`cell_editor::problem_text`); the
type a message names is the one the grid's header shows (`int8`, where the
structure says `bigint`).

| Column | Rule | Message |
|---|---|---|
| Integer | A whole number | "int8 expects a whole number" |
| | within the type's range | "int2 holds -32768 to 32767" |
| Decimal | A plain number, without an exponent | "numeric expects a number" |
| | kept as typed by the scale the type states, whatever the scale: so many decimals, none, or, for PostgreSQL's scale below zero, whole tens, hundreds or thousands. The message says what would be stored | "Up to 2 decimals. 12.505 would be stored as 12.51.", "No decimals. 12.5 would be stored as 13.", "Whole hundreds only. 12345 would be stored as 12300." |
| | within the digits it holds before the point: the type's digits less its scale, so more of them for a scale below zero | "At most 8 digits before the point" |
| | with a scale past the digits (`numeric(3,5)`), under what the digits reach | "numeric holds values between -0.01 and 0.01" |
| Decimal on SQLite | A number SQLite keeps digit for digit | "9223372036854775808 would be stored as 9223372036854776000" |
| Float | A finite number; on PostgreSQL also `nan`, `inf` and `infinity`, in any case | "float8 expects a number" |
| Boolean | `true` or `false`; also `1` or `0` | "bool expects true or false" |
| Boolean on MySQL (`tinyint(1)`) | `true`, `false`, or any whole number from -128 to 127 | "... holds -128 to 127" |
| Enum, CHECK list | One of `allowed_values` | "Not one of: print, ebook, audio" |
| JSON | It parses | "Expected , or } at 3:23" |
| Text with a length | Within the length, shown as `27 / 200` | "At most 200 characters" |
| NOT NULL | Never NULL | the NULL key does nothing |

A boolean column's editor starts from `true` or `false` whatever the
driver loaded (SQLite and MySQL hold 1 and 0).

The checks agree with what the databases store. Only ASCII whitespace
around a value is overlooked, since the text is sent as it was typed.
SQLite keeps a number as an INTEGER or a REAL whatever digits the declared
type states, so a decimal there passes only when it is a whole number an
INTEGER holds or a REAL that is written with the same digits. A column
with a list of allowed values is checked against the list alone, before
its class.

Every rule blocks: a decimal its scale would round is refused with what
the database would have stored, never rounded silently. A type whose
digits or scale are past what a database takes (a scale beyond 1000 either
way) states nothing, and its value is checked as a number only. Every other
rule is the database's, and its rejection is the Failed state.

While the text fails its check the editor is red and shows the message;
Enter, Tab and `Mod+Enter` do not leave it. Clicking elsewhere keeps the
text as a pending cell marked "to fix", so typing is never lost. Save is
disabled while any cell is to fix, with "Fix 1 value to save". A failed
cell does not block: it is an ordinary pending cell that carries the
database's message until it is edited or saved, and the next Save sends it
again.

### Pending changes

- The set lives in `ObjectTab::edits` (`edit::Edits`), keyed by the row
  and column index of the loaded page: per cell the new value (`Text` or
  `Null`) and its state (ready, to fix with its problem, failed with the
  database's error); the editor that is open, if any; the save that is
  running; and what the last save came to. The reducer owns every
  transition (`src/app/editing.rs`); the only state a view changes is the
  text an editor is typing. `Edits` prints without what the user typed, so
  that stays out of logs and panics.
- **A tab that holds edits keeps its page and its structure.** It holds
  edits while a cell is pending, an editor is open or a save is in flight.
  Everything under "Leaving with pending changes" serves that.
- **The bar** above the footer on macOS and Windows, shown while the set is
  not empty, a save runs, or the last save left a line: "3 changes in 2
  rows", "1 to fix", **Review SQL** (**Hide SQL** while its drawer is
  open, see "Review SQL"), **Discard all**, **Save** (`Mod+S`). Review SQL
  stays enabled while a save runs: the drawer then shows what was sent.
  Save is disabled, with the reason as its tooltip: "Fix 1 value to save"
  while a cell is to fix, "Not connected", "This connection opens
  read-only" on a session that came back read-only, and "These changes
  cannot be sent: the table's key is not known" when no change set can be
  built from the set. While a save runs the bar reads "Saving…" with a
  Cancel, and Discard all is disabled. A line with nothing pending has
  **Dismiss** in place of the buttons.
- Omarchy has no bar. Its status line shows the same counts ("3 pending · 2
  rows", "1 error") whenever the set is not empty, and why a save cannot be
  made, in the same words.
- The tab carries the unsaved mark while a cell is pending or its editor
  holds typed text: on macOS and Windows a dot in the close button's place
  while the tab is not hovered, and the tab reads "<name> tab, unsaved" to
  a screen reader; `[+]` after the name on Omarchy. An editor that was
  typed into and left behind another tab keeps its text, which is not yet a
  pending cell, and the mark is all that says the tab holds it; an editor
  that was only opened holds nothing and marks nothing. A changed row
  carries its mark (see "The cell lifecycle"): amber, or red when one of
  its cells is to fix or failed, which is `~` and `!` in the Omarchy
  gutter.
- **Revert one cell:** `Mod+Z` on a pending cell that is active puts back
  the loaded value. The undo and redo stack is slice 4.
- The row panel shows a pending cell's new value on the pending tint, with
  "was <loaded value> · revert", so it never disagrees with the grid. In
  this slice it stayed read-only; slice 3 edits a field in it, and its
  Duplicate and Delete, and the header's Add row, stay disabled.
- Copying takes the pending value a cell shows, for the cell and for the
  row, in every look.

### Keys

| | macOS, Windows | Omarchy |
|---|---|---|
| Edit the cell | Enter, F2, double-click, typing | `i`, Enter, double-click (cursor at the end), `cc` (from nothing) |
| Commit and move down | Enter | Enter |
| Commit and move right, left | Tab, Shift+Tab | Tab, Shift+Tab |
| Leave the editor | Esc drops the edit | Esc keeps it as a pending change, Ctrl+C drops it |
| Move the text into the popover | Alt+Enter | Alt+Enter |
| Apply in the popover | `Mod+Enter` | Ctrl+Enter |
| Set NULL | `Mod+Backspace` | `x` |
| Revert the cell | `Mod+Z` | `u` |
| Focus the row panel's fields (slice 3) | `Mod+I` | `ctrl+l`, `Mod+I` |
| Review SQL | `Mod+Shift+D`, the bar's button | `:diff`, `Mod+Shift+D` |
| Close Review SQL | `Mod+Shift+D`, the bar's button | Esc, `Mod+Shift+D` |
| Copy the SQL | Copy SQL, in the drawer's head | `Y`, while the panel is open |
| Save all | `Mod+S` | `:w`, Ctrl+S |
| Discard all | `Mod+Alt+Backspace` | `:e!` |
| Answer a conflict | its buttons; Esc keeps mine | `o`, `s`, `k`; `d` for a row that is gone; Esc keeps mine |
| Read a conflict's lines | Page Up, Page Down | Page Up, Page Down |

- Typing opens the editor on macOS and Windows except for Space (the row
  panel) and `?` (the shortcuts), which keep their meaning. A chord types
  nothing.
- `Mod+S` saves wherever the bar offers Save: with something pending or an
  editor open, whatever has the keyboard (the grid, the tree, a button, a
  filter's field) and in the Structure view too. From an open editor it
  takes what is being typed. On Omarchy Ctrl+S is the same chord.
- `Mod+Shift+D` shows and hides Review SQL by the same rule, in every look:
  wherever the table's tab shows, with something pending or an editor
  open. Shown, it takes what is being typed, as a save does. It is matched
  by its key, so it reaches Review SQL on a keyboard layout that cannot
  type `:diff`. Held, it shows or hides once.
- **A key acts only on the cell it was meant for.** Within one frame the
  order of a key and a click is lost, and a click selects its cell only
  once the frame is drawn. So on macOS and Windows a typed character,
  Enter, F2, `Mod+Backspace` and `Mod+Z` act only in a frame that brings no
  click, and on Omarchy a letter acts only in a frame that brings no other
  key, text or mouse button. `Mod+Alt+Backspace` is about the whole set and
  does not wait.
- A letter typed in the frame a field asked for the keyboard (the WHERE
  line after `/`, the filter bar after `Mod+F`) is not the grid's: an `x`
  meant for the clause sets no cell NULL, and a character opens no editor.
- A first key that waits for its second (`c` of `cc`, `g`, `z`) is
  forgotten on a click and wherever the keys are not the grid's: under a
  field that has the keyboard, the `:` prompt or a dialog.
- On Omarchy `i` and Enter no longer open the row panel on a table; Space
  and `Mod+Shift+R` do, and the status line's hints read `space inspect`
  and, on a cell that can be edited, `i edit`. A SQL result keeps its keys.
  `s` stays Structure. In insert mode the mode line reads `-- INSERT --`,
  the column and its type, the pending counts, the errors, and "esc normal
  · tab next cell"; Esc there leaves insert mode with the text kept and
  does not close the row panel. Only Ctrl+C itself drops the edit, and it
  copies nothing: Ctrl+Shift+C and any other copy are the field's.
- Omarchy gains a `:` prompt in the status line, on any active table tab.
  It takes `w`, `diff` and `e!`. Any other text is "not a command", which
  the line says until the next key. `diff` opens the panel and never
  closes it; with nothing pending it opens nothing, and the line says
  `nothing pending` until the next key or the next edit. With a SQL editor
  in front it does nothing, as `w` does.
- All of it but a conflict's keys is handled in `ui/keys.rs`. The
  shortcuts screen lists each look's own editing keys
  (`keys::shortcuts(look)`): chords on macOS and Windows, letters and the
  prompt on Omarchy. `Mod+Shift+D` is listed in every look, and `:diff`,
  the Esc that closes the panel and `Y` on Omarchy.
- The conflict question reads its own keys (`ui/conflict_prompt.rs`) and
  names them itself, as the Leave box names `[w]` and `[d]`: the shortcuts
  screen lists none of them, and cannot be opened while the question is
  up. See "Conflicts".

### Leaving with pending changes

- **One guard, at the top of `App::apply`.** It asks which tabs holding
  edits the action would drop (`dropped_by`). None: the action runs. Some:
  the action is held and a prompt asks. What the backend says is never
  guarded.
- **Guarded actions**, the ones that drop the page, the structure or the
  tab: previous and next page, sorting and Clear sort, applying or clearing
  filters and dropping one, refresh, Retry of the rows and Retry of the
  structure, closing the tab, closing or disconnecting the connection,
  connecting again over a workspace, switching database, and closing the
  window. Following a foreign key into a table that is already open and
  holds edits is held the same way, since it filters that tab.
- An action that would do nothing is not asked about, since the prompt's
  Discard would throw the set away for nothing: the next page on the last
  one, the previous page on the first, Clear sort without a sort, a refresh
  while a SQL editor shows, a connect for a connection that is no longer
  saved.
- When one tab is affected the prompt has **Save**, **Discard** and
  **Cancel**, and names what was asked for: "Save 3 changes before closing
  the tab?" (Omarchy: "closing with pending edits", `[w]` write, `[d]`
  discard, `[esc]` stay). Save runs the save and performs the held action
  only if everything was written; a failure, a conflict or a cancelled
  production confirmation drops the held action. When several tabs with
  pending changes are affected (a connection, the window): **Discard** and
  **Cancel** only.
- **The prompt's count includes the cell being edited.** A tab counts its
  pending cells, and one more for an open editor that was typed into, on a
  cell that is not pending already, whose text is a change of what the
  cell loaded: Save closes the editor first, and that text is one of the
  changes it writes. A text its column refuses counts as well, since
  leaving the editor keeps it as a cell to fix. An editor on a cell that
  is pending already adds nothing, nor does one whose text is what the
  cell loaded or one that was only opened. A pending cell typed back to
  what it loaded counts one fewer: closing the editor takes it out of the
  set. A tab that holds edits never
  counts as none: an editor that was only opened, with nothing pending, is
  one. Several tabs are summed.
- **Enter never discards.** It follows the button that has the keyboard:
  on Cancel it cancels, on Save it saves, and on Discard it does nothing.
  With the keyboard on no button it saves where Save is offered (macOS and
  Windows) and does nothing otherwise, so in a prompt without Save a stray
  Enter drops nothing. On Omarchy the letters answer, and not while a field
  has the keyboard or a key is held: a letter typed in the frame the box
  opens is text, not an answer.
- **No Enter with a modifier presses anything,** in this prompt, in the
  production confirmation and in the conflict question. With Ctrl, Cmd,
  Shift or Alt held, Enter is no key of a prompt's: it presses no button,
  whichever has the keyboard, and it is not the plain Enter either, so it
  does not save here, answers no conflict as Keep mine and does not
  confirm the word in Omarchy's production box. Every Enter is taken out
  of the frame before a prompt's buttons are drawn (`keys::take_enter`),
  and only one pressed with nothing held answers. Space presses the button
  that has the keyboard, with a modifier as without.
- **What a prompt asks about cannot change while it is up.** Mod chords
  still reach the reducer under a dialog, and a click can be a frame behind
  it. So while the Leave prompt, the production confirmation or the
  conflict question is up the reducer drops the editing actions, the moves
  of the selection, and the actions that would put another dialog in its
  place (`dropped_under_a_prompt`), and the confirmation sends only the set
  it showed.
- A guarded action that arrives while another dialog is open is refused
  with the notice "Save or discard the pending changes first.": a dialog
  the user is in is never replaced. While a tab's save runs, guarded
  actions on it are ignored.
- **Closing the window** is held too (`App::hold_close`): while a tab
  holds edits the close request is answered with
  `ViewportCommand::CancelClose` and the prompt. It runs in both of a
  frame's passes, the drawing and the app's logic, which is all that runs
  while the window is hidden. Under a running save it asks as well, without
  Save: "A save is still running. It writes everything or nothing, and
  closing now means not seeing which."; Discard gives the save up and
  closes. On macOS, Quit (Cmd+Q, the app menu, the Dock) reaches the app as
  a close request through winit's `macos-quit-as-close` feature. That path
  is compiled and not yet run on a Mac: until someone quits there with a
  pending cell, it is unverified.
- Not guarded: switching tabs or connections, and the Data and Structure
  switch. Pending changes wait in their tab. Nor does anything that
  replaces a page without an action touch such a tab: a changed page size
  leaves it its page until its next fetch.
- **A dropped connection never costs the pending set.** While the session
  is disconnected the set is kept and Save is disabled with the reason.
  Editing does not need a live session; saving does. Reconnecting is not
  guarded: a tab with pending changes keeps its loaded page and its
  structure through the reconnect (they are not fetched again, as other
  tabs' are), and Save works afterwards. The page may be stale by then;
  the save's check against the loaded values covers that. The same holds
  after a script closed the session and after a connection lost while
  saving. If the reconnect comes back read-only (the box was turned on
  meanwhile), the set is still kept and Save is disabled with "This
  connection opens read-only".
- A guarded action's prompt has no Save, only **Discard** and **Cancel**,
  whenever Save itself is disabled: while disconnected, on a session that
  came back read-only, while a cell is to fix, and when the set cannot be
  sent. Its question then reads "Discard 1 change before closing the tab?".

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
returns `Error::ReadOnly` before it looks at the set, without contacting
the server. Then `ChangeSet::check` refuses, before anything is sent: a set
with no rows; a row with an empty `key` or an empty `set`; a key value that
is NULL; a key that names a column twice; a key column that is also
changed; a column changed twice in one row; and the same row twice (keys
that name the same columns with the same values, in whatever order), since
both changes would be compared with the row as it was and the second
written over the first.

One transaction, for every row of the set:

1. Every statement is built first. A value the builder cannot convert,
   and a row it refuses outright (see the drivers below), is `Failed` for
   its row before anything is sent. The builder is the one place that
   refuses, so Review SQL shows such a row without a statement, and a save
   to production asks nothing about it.
2. Begin, read-write, from no transaction: whatever the session has open
   is rolled back first, so a save can never commit what an earlier one
   left.
3. For each row, read it whole by its key, locked (`SELECT * ... WHERE
   <key> LIMIT 2 FOR UPDATE` on PostgreSQL and MySQL; SQLite holds the
   file). The key is matched with its loaded values in the driver's own
   form (a binary key as its bytes). No row is a conflict with `server:
   None`; more than one is an error.
4. Compare each changed column's value in that row with `loaded`, as
   decoded `Value`s (floats by their bits, so NaN equals NaN). Any
   difference is a conflict carrying the row as read.
5. With any conflict: roll back and return `Conflicts`. Every row is read
   and compared before any is changed.
6. For each row, `UPDATE <table> SET <column> = <new>, ... WHERE <key>`.
   It must touch exactly one row on PostgreSQL and SQLite; MySQL reports
   changed rather than matched rows, so there none or one. A statement
   that fails, touches another number of rows, or on MySQL raises a
   warning or a note (a truncated or adjusted value) rolls everything back
   and returns `Failed` with the database's error or the warning's text.
7. Read each row whole again by its key. It must still be exactly one row:
   a trigger the update fired can have made another that the key finds,
   and then which row was saved is not known. Commit, and return the rows.

Every exit leaves no transaction open and the session as the rest of the
app expects it. A session that cannot be put back is reported as
`Error::ConnectionLost` and the backend ends it. `Written` comes back only
after a `COMMIT` that took hold.

An `Err` is not always a lost session. A lock another session holds until
a timeout, a busy file when SQLite asks for it, a `COMMIT` the database
refuses, a cancel, a key that matches more than one row: each is an error
on a session that lives, with nothing written, and the pending set should
stay as it was. Only `ConnectionLost` means the session is gone; then
nothing was committed unless the connection was lost while committing.

A key must name one row, and a save refuses what it cannot be sure of
rather than guess. Per driver:

- **SQLite.** `query_only` is the session's standing state and is lifted
  for one `BEGIN IMMEDIATE` transaction, with no fence up: the statements
  are the app's own. Before it, the save puts back what a script can leave
  on the session and a write would feel: `main`'s journal mode (unless
  either mode is WAL, which is the file's and not the session's),
  `locking_mode`, `ignore_check_constraints`, `recursive_triggers` and
  `count_changes`. Only `main` is written: a table of an attached database
  is refused. Values are bound exactly as built, never through the
  filters' text-to-number guess. A name or a text that is not UTF-8 reads
  with U+FFFD for the bad bytes and so can read as another, and SQLite
  takes a name in other ASCII letters for the column all the same, so a
  save refuses: a name that more than one of the row's columns reads as; a
  name not spelled as the table spells it; a key whose text holds U+FFFD
  (a key that really holds one pays for this), which the statement builder
  refuses; and a changed column whose stored text is not UTF-8, as `Failed`
  and not as a conflict, since a conflict offers to write over what the
  file holds, which would still be unknown.
- **PostgreSQL.** The transaction is managed as text (`ROLLBACK; START
  TRANSACTION READ WRITE; SET LOCAL client_encoding = 'UTF8'`, then
  `COMMIT` or `ROLLBACK`), not through the driver's transaction type,
  which cannot answer a cancel that lands on its end. The statements carry
  their values as literals built from what the page read, so the encoding
  is pinned for the transaction, and every session, read-only or not,
  prints floats in full and dates in ISO (`SET extra_float_digits = 3; SET
  DateStyle = 'ISO'` at connect): with fewer digits two neighbouring
  floats print alike and the key of one finds the other, and a zone's
  abbreviation can read back as another zone. A page on a server
  configured otherwise shows the difference. Text with a NUL is refused
  by the statement builder, before anything is sent: PostgreSQL text holds
  none, and the driver cannot put one in a message.
- **MySQL.** Only a table whose engine has transactions: one transaction
  is the promise, and MyISAM cannot roll back. The engine is checked
  before the transaction and again after the locking reads, which hold the
  table's metadata lock to the end, so an `ALTER TABLE ... ENGINE` in
  between cannot leave updates that would not roll back. A view has no
  engine and is refused. The transaction is started as text (`START
  TRANSACTION READ WRITE`, checked by the server's status), not through
  the driver's options, whose two statements a cancel can come between,
  and it ends with `COMMIT` or `ROLLBACK AND NO CHAIN NO RELEASE`. Every
  session has `sql_notes = 1`, since a decimal rounded to its column's
  scale raises only a note and a server can have notes off. A name spelled
  in other letters than the table spells it is refused (MySQL takes `ID`
  and `id` for one column). A row key with a `timestamp`, `bit` or `float`
  column is refused: a TIMESTAMP shows in the session's zone without it,
  so two instants of a repeated daylight-saving hour read alike; a BIT
  bound as bytes is read as a number; a FLOAT bound as a double misses its
  row.

New values travel as text and the database converts them to the column's
type: a quoted literal on PostgreSQL (`'...'`, or `E'...'` when the text
holds a backslash, so it reads plainly and still runs as shown), a bound
string on MySQL. Where the database would store the text as it is, the
statement builder converts first, by the column's class (`column_class`,
from `CellChange::type_name`, the same function the checks use): on SQLite
integers and reals are numbers, and on SQLite and MySQL a boolean is 1 or
0; a MySQL `tinyint(1)` also takes any whole number a tinyint holds. A
SQLite column with no declared type, or one SQLite gives no affinity,
converts nothing, so there the builder follows what the cell held: a
number stays a number if the new text is one, and text stays text. A
binary column, or a cell that held bytes, is refused outright, a NULL for
it too. The builder is the only place that decides a value's form, so the
literal Review SQL shows (`12`, `1`) is the value the driver binds. Three
literals are written so that the shown text runs when pasted and holds no
raw NUL: SQLite text holding a NUL as `('a' || char(0) || 'b')`, an
infinite SQLite float as `9e999`, and a NUL in MySQL text as `\0`, after
its backslashes are doubled.

The backend has `Command::Write { session, request, changes }` and
`Event::Written { session, request, result }`, queued and answered like
every request; the reducer finds the tab by the request. A save's request
is one of its tab's pending requests, so `Mod+.` (and the bar's Cancel)
cancels a running save like any query: the backend sets the save's stop
flag (`StopFlag`, as for a script) and fires the session's cancel handle;
the `write` future is awaited to its end, never dropped, and rolls back.
A cancel only reaches a statement that is running, so the flag is what
honours one that arrives between two of a save's statements: every driver
asks it before each statement it sends and once more before `COMMIT`, and
a save it ends answers `Cancelled` with nothing written. A cancel that
reaches the `COMMIT` before it takes hold undoes the save, which answers
`Cancelled` too. After that it is too late: the save is written and says
so. A cancelled save's tab says "Save cancelled. Nothing was written."

What a save came to when it wrote nothing is kept with the set, which
stays as it was (`edit::Note`), and is said in the bar or in the Omarchy
status line:

- A connection lost during the save: "The connection was lost while
  saving. Reload to see what was written." A session swapped under a save
  (a reconnect) abandons the save and says the same: its answer, if one
  comes, finds no tab saving.
- A production confirmation answered after the session went, or a
  conflict's Overwrite: "Not connected. Nothing was sent."
- Rows that changed on the server or are gone: the user is asked about
  each, and the tab says nothing while the question is up. Where it is not
  asked, the conflict is a line (see "Conflicts").
- A statement that failed: its row's cells turn red, and the line is the
  database's code and message, then "Nothing was written."
- Any other error on a session that lives (a lock timeout, a refused
  `COMMIT`): the error's text, then "Nothing was written."

After `Written` the reducer replaces those rows in the page by what the
database returned, in place even when the sort or the filters would now
move or hide them, clears the pending set, and drops the row panel's text
so it is formatted again. The cells show green for 1.2 seconds. When a
returned row is not as wide as the page (the table changed since the page
was read), the page is fetched again instead. Either way what was held for
the save goes on. The footer's end reads "written 2 changes · 1 row · 14
ms" in place of the query's time, until the next edit or page; on Omarchy
the status line says it.

## Review SQL

Before saving, a user reads the statements a save of the tab's pending
changes would run. `src/review.rs` makes them from the pending set as
lines of data, and one view, `src/ui/review.rs`, words and draws them
wherever they show: the drawer, the `:diff` panel and the production
confirmation.

- **The statements.** One `UPDATE` per changed row, by key, with the values
  as literals, laid out in lines: the table, each value that is set, each
  column of the key, and `;` at its end. Only the white space between its
  words is the layout's. Quoted names, the schema and upper-case keywords
  are as the statement runs, in every look. PostgreSQL sends it as
  written, and MySQL and SQLite send the same statement with the values
  bound. One builder in `dialect.rs` (`Dialect::update_row`) produces both
  forms from a `ChangeSet`, so they cannot drift, and says where the parts
  of the shown text stand (`RowUpdate::parts`), so nothing searches a
  statement for where a value begins and ends. The `WHERE` is the key
  only.
- **Two comments stand above each statement:** `-- row id 2`, the row by
  the key that finds it (several columns as `-- row order_id 7, line 2`),
  and the check a save makes, `-- only if kind is still 'print' and
  alt_text is still NULL`, naming every changed column with what the page
  loaded: text in single quotes, a number bare, `NULL`, `true` or `false`.
- **No value ends a comment or passes for a line.** A line break or
  another hidden character in a name or a value is written out
  (`<U+000A>`) where it is shown, and in a comment where it is copied too.
- **A row without a statement is a comment only,** in the danger colour. A
  row with a cell to fix: `-- row id 4 · blocked: fix publisher_id first`,
  every column to fix named. A row the builder refuses: `-- row id 4 ·
  cannot be sent:` and the builder's reason. The builder refuses whatever
  a save refuses before it sends anything: a value its column's form does
  not take, a binary value, PostgreSQL text that holds a NUL, a SQLite key
  that may not have been read exactly. A set no change set comes of is one
  line: `-- these changes cannot be sent: the table's key is not known`.
- **A literal longer than 60 characters is cut where it is shown:** its
  beginning, `…`, and what closes it. The cut never falls between the two
  halves of a doubled quote or backslash, nor after the backslash of
  MySQL's `\0`, nor inside what joins the strings of SQLite's text around
  a NUL (`('aaa…')`, `('aaa'…)`). The value in a check's comment is cut
  the same way. That is display only: what runs, and what is copied, hold
  the whole value.
- **The lines can be selected,** as a label's text is: a drag selects
  across lines, a double click a word, and `Mod+C` copies the selection.
  It is copied as it reads: a value that is shown cut is copied cut, with
  its `…`, and nothing says what the text is. **Copy SQL** puts the whole
  statements on the clipboard (see "What a copied text is"). A line is
  named by its place in the review, so a selection stays on its line while
  the lines scroll; one whose ends scroll out of view is dropped. The lines
  take no Tab stop.
- **Copy SQL says that it copied.** On macOS and Windows a toast, "Copied
  SQL", floats over the bottom of the window, centred, for four seconds:
  the window's colour on the text colour, over a dialog too. Omarchy's
  status line says `copied sql` for as long, where it has nothing of a
  lock or of a save to say. A second copy starts the four seconds again.
  The toast is the frame's own (`src/ui/toast.rs`): it takes neither the
  pointer nor the keyboard, and nothing of the app's state knows of it.
- **The review is the tab's, and goes with the set.** Each table tab has
  its own (`Edits::reviewing`, `Edits::review`). It stays open while
  another tab shows, through a save that runs, fails or conflicts, and
  through a lost connection. It goes when nothing is pending any more
  (written, discarded, the last cell reverted) and does not come back by
  itself with the next change. The reducer makes the lines when the set
  changed or a structure arrived (`App::make_reviews`, once per round of
  actions); drawing lays out the lines in view and makes nothing.
- **Showing the review closes an open editor,** as a save does: its text is
  taken as a left edit, pending or to fix, so what is reviewed is what a
  save would send. An editor that was only opened is no change, and with
  nothing else pending nothing opens. Hiding leaves an open editor alone.
- **The head says when a cell is still being edited.** An editor opened
  under an open review is not in its lines until it commits, while Save
  would send its text. For as long as it holds typed text the head reads
  "Without the cell being edited", in the warning colour, where it says
  what a save is.
- **macOS and Windows:** a drawer above the pending bar, toggled by the
  bar's **Review SQL** and **Hide SQL** and by `Mod+Shift+D`. Its head
  reads "Runs in one transaction", with **Copy SQL** at its right. It is as
  tall as its lines, to twelve of them and to half of the tab's area, and
  scrolls both ways past that: a line is never wrapped into what could
  read as two.
- **Omarchy:** the `:diff` panel, a bottom panel above the status line and
  the grid's error line, as wide as the grid. Its head reads `pending · 3
  changes · 2 rows` and `one transaction`; its foot `esc close` and `Y
  copy sql`, each with a button that is not drawn over it ("Hide SQL",
  "Copy SQL"), and `:w write`. `:diff` and `Mod+Shift+D` open it, Esc
  closes it before it closes the row panel, and `Y` copies the whole SQL
  while it is open. It does not take the keyboard: the grid keeps its
  keys, and `u` reverts the active cell while the panel follows. While
  something is pending the status line's hints include `:diff review`.
- Every line in view is read to a screen reader, a comment as it is
  worded.
- Under a prompt about the pending changes nothing shows or hides the
  review (`dropped_under_a_prompt`).

### What a copied text is

The app's statements with every value whole, under their comments, whose
loaded values are whole too (on one line, hidden characters written out),
after a first comment line that says so: `-- What Tabletist runs to
save these changes, in one transaction. Each statement runs only while its
row is still as the comment above it says.` It is not a script that checks
or wraps anything: it has no `BEGIN` and no `COMMIT`, and nothing in it
compares a row with what was loaded. Pasted elsewhere it changes each row
by its key, whatever the row holds by then.

Each statement stores what the app's own save stores. In three places
another client reads it otherwise:

- **MySQL under `NO_BACKSLASH_ESCAPES`.** The text doubles a backslash and
  writes a NUL as `\0`, as the app's session reads strings. A session in
  that mode stores each backslash twice, and a backslash and a zero where
  the NUL was.
- **A SQLite REAL with a very large exponent.** Written as text it can
  read back as a neighbouring double, where the app binds the exact value.
- **CR before LF in a SQLite value, through the `sqlite3` shell.** The
  shell stores LF alone. Through the app the value is exact.

## Saving to production

Only when the workspace's environment is production
(`Environment::confirms_writes()`), every Save first asks, with every
statement it would send on screen. The statements are the review of the
set the confirmation was made with (`WritePrompt::review`), and both looks
draw them as Review SQL does: the same lines, colours and cut, selectable
as there, in a box that scrolls both ways. The box is as tall as its
lines, to twelve of them, and lower in a low window, so the question and
its answers stay on screen with them.

- macOS and Windows: a sheet with a band of the production red along the
  top, "Save 2 changes to production?", the connection's name and database
  and "1 row in book_covers", the statements, "One transaction", **Copy
  SQL**, **Cancel** and **Save to production**. Copy SQL gives the whole
  statements of the set the sheet shows. Enter does not confirm: the
  button is pressed. Enter cancels with the keyboard on Cancel and copies
  with it on Copy SQL, and Esc cancels. No drawer opens behind the sheet.
- Omarchy: the box with the danger border, its head with the `PROD` tag
  and "write 2 changes?", the connection's name and database, "1 row in
  book_covers" and the columns the save sets, the statements, "type write
  to confirm" and a field that takes the word; Enter confirms only when
  the field holds exactly `write`, Esc cancels. The reducer opens the
  tab's `:diff` panel with the box, holding the review the box was made
  with.
- **The Omarchy box points at the panel** ("sql shown with :diff", in
  place of the statements) only when all of this holds in the frame it is
  drawn: the panel is on screen for the box's own tab; it draws the review
  the box holds; it shows its lines, all of them or three at once at the
  least; the widest of them fits its width; and the window leaves 300
  points above it. The box then stands in that room, clear of the panel
  and without a backdrop, so the statements under it are read at full
  strength. It takes every click all the same.
- **Everywhere else the box lists the statements itself,** over the
  backdrop every dialog has: its tab is not the one in front (a save asked
  for by the Leave prompt of a tab, a connection or the window), another
  tab's panel is on screen, or the window is too low or the panel too
  narrow for its lines. Width counts as height does: under the box
  nothing moves the panel's lines sideways, and what is past its edge of a
  statement would be confirmed unread.
- **Page Up and Page Down** move the statements that are asked about, a
  page being the rows in view: the panel's where the box points at it
  (its foot then says `pgup/pgdn scroll sql`, since the pointer does not
  reach the panel under a dialog), and the confirmation's own list in the
  sheet and in the box that lists. A drawer or a panel behind a
  confirmation that lists is not moved.
- After Cancel the Omarchy panel stays open: the statements were just
  declined and are still what is pending. After a save that wrote it
  closes with the set.
- A row whose statement cannot be built opens no confirmation: its cells
  fail with the builder's reason, as they would in the save.
- The confirmation holds the change set it showed. When it is confirmed,
  the save is sent only if the tab's pending set still makes exactly that
  set (floats compared by their bits, so a NaN is the same value as
  itself), no editor is open and Save is not disabled.
- **Brought up by the answer to another dialog, it takes no answer in its
  first 500 ms** (`edit::ANSWER_AFTER`, `WritePrompt::after_answer`). Save
  in the Leave prompt and Overwrite in the conflict question open it in the
  place of the dialog that was just answered, under the hand that answered:
  the click or the key that gave that answer, given twice, is none to this
  question. In that moment no click on **Save to production** or
  **Cancel** is taken, no Enter and no Esc, and **Copy SQL** copies
  nothing: a click meant for the dialog before does not replace what is on
  the clipboard. The moment holds for a click's press too: a click pressed
  in it and let go after it is no answer, on **Save to production**,
  **Cancel**, **Copy SQL** or a key hint of Omarchy's box
  (`write_prompts::pressed_early`, which the conflict question asks as
  well). What is typed into Omarchy's field then stays typed, and
  Page Up and Page Down still move the statements, since reading them
  answers nothing. Nothing is kept for later, and nothing on screen shows
  that the moment runs. Opened by Save itself (the bar, `Mod+S`, Ctrl+S,
  `:w`) the confirmation answers at once.
- **A key that is held confirms nothing.** The key that answered the
  dialog before may still be down: what a held Enter repeats is no press,
  and what a held Space repeats presses no button, whichever has the
  keyboard.
- **No Enter with a modifier presses anything:** not **Save to
  production**, **Cancel** or **Copy SQL** with the keyboard on it, and in
  Omarchy's box it does not confirm the word. See "Leaving with pending
  changes".

## Conflicts

A save reads every row it would change by its key, locked, and compares
the changed columns with what the page loaded (see "Saving"). A row that
holds another value in one of them, or that is no longer there, is a
conflict. The save then writes nothing, for any row of the set, and
answers with every conflicting row as the server holds it now
(`WriteOutcome::Conflicts`). What was held for the save (a tab or a window
to close) is dropped by its conflict, for good: a question comes between
the wish and the save, and the user closes again.

The user is asked about each of those rows, one after another: the
conflict question (`Dialog::Conflict`, drawn by `src/ui/conflict_prompt.rs`
in both of its forms). It holds the save's conflicts as rows of the page
(`edit::conflicting`), the one being asked about and that row's lines
ready to draw. Every answer is applied when it is given, to the page and
to the pending set, so each row is whole whatever comes of the rows after
it. Of the answers only two things are kept until the last: whether a row
was overwritten and whether one was kept.

### The question

A sheet on macOS and Windows, 520 points wide; a box on Omarchy, 560 wide;
each narrower in a narrow window.

- **The title** names the row by its key, as the pending bar does: "Row id
  2 changed on the server", "Row org 7, id 2 changed on the server" for a
  key of two columns, the row's number where the key is not known. Only
  the name gives way to a narrow question: it is cut with "…", and the
  whole title is what a screen reader reads and what the pointer shows
  over it. On Omarchy the line starts with `≠ conflict` in the warning
  colour (the word alone where the look's font has no such mark), and the
  title follows in lower case, the key as the database has it.
- **Where the row is** has a line of its own under the title: the
  connection's name and the table's, and which of the save's rows this is
  when there are several: "Bookshop · book_covers · 1 of 2". The question
  can come up while another tab or another connection is in front, a table
  of one name can be open on two connections, and the question switches
  nothing.
- **What happened:** "Someone saved it after you loaded it. Nothing was
  written." On Omarchy too, where the design's panel has no such sentence.
- **The table** has a line for each column the user changed in that row,
  which are the row's pending cells, in the page's column order: the
  column's name, the value the page loaded, the value the server holds
  now, and the user's. The sheet's is a bordered table whose header reads
  "loaded", "now on server" and "yours"; the box's has no rules and its
  header reads `loaded`, `server` and `yours`.
- **What is marked.** The server's value is marked where it is another
  value than the loaded one, compared as the save compared them (a float
  by its bits), and plain where it is the same: on a red tint in red in
  the sheet, in the danger colour in the box. A conflict has at least one
  such column. The user's value is always marked as a pending cell is in
  the grid: on the amber tint in the sheet, in the warning colour in the
  box. A screen reader is told every value whole, and of a marked server
  value that it "changed on the server", after the value: the tint or the
  colour is all that says so on screen.
- **A value reads as a grid cell shows it:** on one line, with the mark
  for a line break, `''` for the empty text, a mark for each character of
  an all-white one, and a cell's 256 characters at the most, cut with "…"
  to its place. The whole reading is under the pointer over a value that
  is cut. NULL is the grid's NULL: the grey chip, or Omarchy's faint word.
  A value that became NULL on the server is the chip on the red tint, and
  on Omarchy the word `NULL` in the danger colour.
- **Two values of a line that differ never read alike for want of room.**
  A value's cell holds about sixteen characters, and fewer in a narrow
  window. What decides is what fits where it is drawn: where two cells of
  a line would paint the same and their values do not read the same, each
  is painted from inside itself, behind a "…", so that the first place the
  two differ is in its cell. Up to twelve characters stand before that
  place to find it by; a cell too narrow for them gives them up first, one
  by one, and the end is cut only when the value is too long from that
  place on. It is settled pair by pair: of three values, two that are
  still alike from the first difference are painted from before their own.
  Two values that are the same are painted the same. The view works this
  out from what a cell reads of each value, never from the whole values.
  For values that differ only past a cell's 256 characters the reducer
  keeps, once, when the row's question comes up, the part of each that
  starts twelve characters before the difference (`edit::shown_lines`),
  pair by pair in the same way: a value can be megabytes, too much to
  compare in every frame.
- **The lines scroll** where there are more than 220 points hold (seven
  in the sheet), under a header that stays. Page Up and Page Down move
  them by the whole lines in view, and the box's foot then says `pgup/pgdn
  scroll` before the keys that answer. When a row's question comes up the
  lines start with the first line the server changed in view, moved by as
  few whole lines as bring it in and by none where it shows from the top:
  the lines are in the page's column order, and the one that is marked can
  stand below what shows at once. From then on they are where the user
  moves them.
- **A row that is gone** has its own words, "Row id 2 no longer exists on
  the server" and "Someone deleted it after you loaded it. Nothing was
  written.", no column for the server, whose width the two others share,
  and one answer.

### The answers

- **Keep mine, reload row.** The server's row replaces the loaded row in
  the page, in place, also where the sort or the filters would now move or
  hide it. The row's pending cells stay on top of it, except one whose new
  value is no change against what the server holds now, which leaves the
  set (`edit::is_change`, the rule that takes a cell out of the set when
  the loaded text is typed back).
- **Use server values.** The server's row replaces the loaded row and
  every pending cell of the row is dropped.
- **Overwrite.** As Keep mine, and the save may run again once every row
  is answered. A row that changed once more conflicts once more.
- **Discard my changes,** the one answer of a row that is gone: its
  pending cells are dropped, and the row is marked gone (see "A row that
  is gone, in the grid").
- A cell that stays keeps its state: a failed cell's message is about its
  new value, which did not change. The row panel's text and the tab's
  Review SQL are made again from the row as it is now.
- **In the sheet** Keep mine reads as a link at the left, without a border
  and in the accent colour; "Use server values" is a bordered button and
  "Overwrite" the primary one, filled with the ink, at the right. Discard
  my changes is a plain button.
- **In the box** the answers are keys in its foot, as every terminal
  dialog of the app has them: `[o] overwrite`, `[s] use server`, `[k] keep
  mine, reload`, and `[d] discard my changes` for a row that is gone, each
  letter in the accent colour and each hint its button too. The letters
  are read as typed text, and not while a text field has the keyboard. A
  letter the row does not offer does nothing.
- **Esc is Keep mine** for the row shown, in both forms. For a row that is
  gone, Esc, and `k` on Omarchy, keep the row's changes pending, mark and
  lock nothing, and leave the line "Row id 2 no longer exists on the
  server. Nothing was written." in the bar or in Omarchy's status line:
  the row is not left looking like any other with pending changes, and
  the next save says once more that it is gone.
- **Return never presses Overwrite,** as it never presses "Save to
  production". Enter answers only as Keep mine, with the keyboard on that
  button or, in the box, on that hint's button; on every other button and
  with the keyboard on none it does nothing. No Enter with a modifier
  presses anything: with Ctrl, Cmd, Shift or Alt held it answers nothing,
  on Keep mine as on every other button. Space presses the button that
  has the keyboard, as everywhere. The Tab key comes to Keep mine first in
  the sheet, then to Use server values and Overwrite. In the box it takes
  the hints as they stand, `[o]` first.
- **An answer names the row it answers** (`Action::AnswerConflict { at,
  answer }`), and the reducer drops one for any other row, or one the row
  does not offer: a click or a key a frame behind is no answer to the next
  row.

### When an answer is taken

- **No answer in a question's first 500 ms** (`edit::ANSWER_AFTER`), for
  every row of the save. The question comes up when the database answers,
  not when the user asks: a key or a click on its way to the grid can land
  on it, and on Omarchy `k`, `s` and `d` are keys of the grid. And the next
  row's question takes the place of the last, button for button: the
  second click of a double click would answer a row the user never saw. A
  click, a key or Esc in that moment is dropped with no sign: the buttons
  look as they always do, and nothing is kept for later. A click counts
  only when its press, too, came after that moment.
- **A key that is held never answers,** however long the question has been
  up: what Space, Enter, Esc or a letter repeats is no press. The next
  row's question has the keyboard on the button that was just pressed, and
  a held key would answer row after row of the save.
- Page Up and Page Down move the lines in that moment too: reading answers
  nothing.
- The production confirmation that Overwrite opens has the same first
  moment (see "Saving to production").

### After the last answer

- **The save runs again when some row was answered Overwrite and none
  Keep mine.** A save writes the whole pending set, so a row the user kept
  to look at again is never written by another row's Overwrite. Use server
  values and Discard count neither way. Keep mine counts whatever its
  rebase left, and for a row that is gone too. An Overwrite counts even
  when every one of its cells left the set: then the rest of the set is
  what is saved.
- **That save is an ordinary save.** With nothing pending nothing is sent
  and nothing is said. Where a save cannot be made the bar's Save and
  Omarchy's line say why. On production the confirmation asks again, with
  the statements of the set as it is now, in which the server's values are
  the loaded ones. When the session went while the question was up and
  something is pending, the tab says "Not connected. Nothing was sent."
  Nothing is held for it: after it wrote, the tab the user wanted to close
  is still open.
- Otherwise the tab says nothing more: the bar shows what is still
  pending, or goes when nothing is.

### Where the question is not asked

The conflict is then a line, and the set stays as it was, with no row
rebased: "Row id 2 changed on the server. Nothing was written." (or "Row id
2 no longer exists on the server. Nothing was written."), with "1 more row
too." after it when other rows conflict as well, in the pending bar or,
after `≠ conflict`, in Omarchy's status line. The next save asks.

- **Another dialog is up** when the save is answered (the question before
  the window closes under a running save, the shortcuts, Settings): a
  dialog the user is in is never replaced.
- **The conflicts cannot be asked about:** one names a row the save did
  not send or the page does not hold, a row comes twice, or a server row
  is not as wide as the page (the table is no longer the one the page was
  read from). The line names the first of them, with the count of the
  others; where that first one is a row the save did not send, it is "The
  connection was lost while saving. Reload to see what was written."
  instead, since an answer about such a row is not one to tell the save's
  end by.
- **The question is closed without an answer** (`Action::CloseDialog`).
  Rows already answered stay settled; the row shown and the rows after it
  stay as they were loaded, with their cells pending, no save runs (the
  Overwrite of an earlier row waits for a last answer that never came),
  and the line names the row that was shown and counts the rows after it.
  The one sender of that action under this dialog is its own view, when
  the tab it asks about is no longer there to draw: then there is no tab
  to say anything.

### While the question is up

- Nothing changes the set or the page: the reducer drops what
  `dropped_under_a_prompt` names, as under the two other prompts. A
  guarded action is refused with the notice, and a request to close the
  window is cancelled the same way: every row still to be asked about has
  pending cells, so the tab holds edits until the last answer.
- What the backend says still arrives. A tab that holds edits is never
  fetched, so no page arrives for it; one that does ends the question. A
  lost connection changes the status and nothing else, since the answers
  need no session: the second save is where it shows.

### A row that is gone, in the grid

- Discard my changes marks the row (`Edits::gone`) and locks its cells
  (`Lock::Gone`, see "What can be edited"). It is drawn dim, and is
  selected, copied and shown in the row panel as it was loaded.
- The mark is about the page, so it goes when a page arrives, and it stays
  through what only drops pending changes: Discard all, the Leave prompt's
  Discard and a save that wrote. A fetch that fails leaves the old page on
  screen, and its gone rows gone.
- It does not hold the tab's page: a refresh is how the row leaves the
  screen, nothing of the user's would be lost by it, and nothing is asked.

## Steps

Each step ends compiling, tested and shippable, and gets its own plan run:

1. Writable connections: `Access`, the sessions, the script runner's fence,
   the dialog's box, the shipped screens.
2. `Connection::write`, the statement builder and the column classes, the
   catalog's `ColumnInfo.generated`, `IndexInfo.partial` and
   `IndexInfo.key_columns`, the row key rule, the backend command and
   event. Nothing in the UI saves yet.
3. Editing in the grid: the lifecycle, the editors, the checks, the pending
   bar without Review SQL, the keys but `:diff`, Save, the leaving guard,
   and the production confirmation with its statements in both looks.
   A conflict in this step is the plain line of "Where the question is not
   asked", with no rebase.
4. Review SQL: the drawer and `:diff`; the Omarchy PROD box opens the
   panel and points to it.
5. The conflict question: a row that is gone, what a conflict shows and
   what an answer settles, the question in its two forms, the save that
   runs again, and the first moment of the question and of a confirmation
   an answer opened.

No step ships a production save without its confirmation and its
statements.

All five steps are built.

## What step 3 leaves for steps 4 and 5

Steps 4 and 5 are built. Left as found in Review SQL:

- Omarchy's letters and its `:` are matched by the character typed, so on
  a keyboard layout without Latin letters `i`, `x`, `u`, `cc`, `Y` and `:`
  do nothing. Insert mode is still reached there by Enter or a
  double-click, and Review SQL by `Mod+Shift+D` and the panel's two
  buttons; the prompt's other commands have no way in.
- `Mod+Shift+D` on an editor that was only opened, with nothing else
  pending, closes the editor and shows nothing.
- A name in a shown line is not cut: only values are.
- A loaded text that holds `' and ` reads as two conditions in the check's
  comment. A comment is read and never run.
- Under the Omarchy box nothing copies the statements: `Y` and the panel's
  Copy SQL are the panel's, and are not reached under a dialog.
- A row whose last save failed is not marked in the review. The bar and
  the grid say it.
- The drawer and the panel are not resized, and remember no height.
- The Omarchy panel has no cursor of its own: the design's `]c next
  change` and `u revert under cursor` in its foot are not built.

Left as found in the conflict question:

- A line break reads as one mark whether it is CR, LF or CRLF, as in a
  grid cell. Two values that differ only in that read alike in the
  question, as do a tab against a space and the number 1 against the text
  `1`: the tint, and what a screen reader is told, are all that part them.
- Of three values that differ in two places, a cell shows one place. Where
  two of them differ only past a cell's 256 characters and the third
  differs from both early, the two are read from inside themselves and the
  third from its start: it is told from them, and its cell need not reach
  the place it differs.
- The server's row is put into the page on a check of its width alone
  (`edit::conflicting`), as a save that wrote puts its rows there. A table
  whose columns were put in another order, their number unchanged, between
  the page and the save is not noticed. The saves after it end in a
  conflict again and again, never in a silent write.
- In a window lower than about 430 points the question overflows the
  window.
- The question about a row that is gone shows one answer. Nothing on
  screen says that Esc (and `k` on Omarchy) keeps the row's changes.
- Every row is asked about by itself: there is no answer for all of them.
  Twenty conflicts are twenty questions, each with its first moment.
- A conflict that cannot be asked about is a line, and the user saves
  again to be asked: the rows are not kept for a question to open once the
  other dialog closes.
- A row that was answered Keep mine is written by the next save without a
  question, when it did not change again. Nothing marks it apart from any
  other pending row.
- After a rebase a pending cell can sit on a cell that is now locked (the
  server's value there is bytes, or over 256 KiB). It stays pending and
  cannot be opened; Revert and Discard all take it out, and a save of it
  fails its row with the builder's reason.
- A row that is gone is drawn dim until slice 5 (rows) draws a deleted
  row; the two should then agree.
- On a keyboard layout without Latin letters `o`, `s`, `k` and `d` do
  nothing, as Omarchy's other letters: the hints are buttons, and Esc
  keeps.
- The notice of a refused guarded action, "Save or discard the pending
  changes first.", is also what a close request under the question says.

Open in the save as the grid shows it:

- A MySQL `TIMESTAMP` as a changed column can miss a conflict in a repeated
  daylight-saving hour. The key case is locked; the cell case is not. The
  question is then not asked and the save writes.
- A cancel is honoured until `COMMIT` is sent: between two of a save's
  statements through the save's stop flag, and on the `COMMIT` itself when
  it reaches it before it takes hold. After that it is too late: the save
  is written and says so. Whether the Saving state should say more is
  open.
- The Leave prompt's Save, when the session went while the prompt was up,
  closes the prompt, saves nothing, drops the held action and leaves no
  line. The "A save is still running" prompt keeps its sentence if the
  save ends while it is up.
- A column with a list of allowed values is checked against the list
  before its class, so a SQLite `INTEGER` column whose list holds
  non-numbers passes the check. Review SQL says such a row cannot be sent,
  but Save is still offered, and fails the row.
- MySQL stores some values adjusted without a word (`'1.6'` into a
  `TINYINT` is 2). The checks catch a non-integer in an integer column; a
  `FLOAT`'s precision is not checked.
- A table whose engine has no transactions, and a table of a database
  attached to a SQLite session, are refused only by the save.
- Quit on macOS is unverified (see "Leaving with pending changes"). If it
  does not pass through a close request, holding it back needs the
  application delegate, in `crates/tabletist-appkit`.
- An input method cannot start an edit by typing (Enter or F2 does). The
  row's key value takes the row's colour, not a heavier weight.

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
- `src/edit.rs`: the lock of a row that is gone, a save's conflicts as
  rows of the page and where there is nothing to ask from them, what a
  conflict's lines hold and mark, which cells an answer settles, and what
  is kept of values that read alike for a cell's worth, pair by pair.
- Reducer tests: the pending set, the checks, the leaving guard and its
  held action, rows replaced after a save, the review made when the set or
  the structure changes and closed with the set, a row the builder refuses
  failing in place of a confirmation, rebasing after each conflict choice,
  mixed conflict answers and when the save runs again, the next row's
  question coming up with the answer before it, a conflict under another
  dialog and a question closed unanswered leaving the line, nothing
  changing under the question, and a tab with pending changes keeping its
  page across a reconnect and saving afterwards.
- `src/review.rs`: the lines against the builder's statement for every
  dialect, whatever names and values hold; the cut; that no value ends a
  comment.
- Headless UI tests, in every look: the lifecycle, the keys, the popover,
  the bar, the drawer and the panel, the three dialogs, where the Omarchy
  box points and where it lists, locked cells saying why, the shipped
  screens' new texts.
- Of the conflict question, in every look: what it says and marks, and
  where; each answer by its button, its key and Esc; Enter and the Tab
  order; the first moment of every row's question, for a click, its press,
  a key and a letter, and the confirmation's; a key held from one row's
  answer through the rows after it, and through the confirmation; values
  that differ where a cell does not reach, at four window widths; the
  lines that scroll, their keys, and the first changed line in view; what
  a screen reader is told; a small window; a row that is gone, in the
  question and in the grid.
- `src/shots.rs` has a scene for review of each editing state, on its
  Bookshop data and in every look: `edit-pending`, `edit-review`,
  `edit-field`, `edit-large`, `edit-saved`, `edit-failed`, `edit-conflict`,
  `edit-conflict-gone`, `edit-leave` and `edit-production`. They need a GPU
  and are run by hand. No test compares a screen with the design.

## Documents this changes

The main spec's success criterion 6, its sections 4.3 and 5.7 (Enter and
`i` on Omarchy) and its keyboard table; the SQL editor spec's intent, its
guard layers 2 and 3 and its cleanup list, restated for writable
connections; the crate documentation of `tabletist-db` ("nothing in this
crate writes") and the comment on `RowQuery::raw_where`; the README; the
shortcuts table in `ui/keys.rs`.
