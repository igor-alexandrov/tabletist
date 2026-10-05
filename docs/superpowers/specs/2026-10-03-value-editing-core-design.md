# Editing values, slice 1: the core and its safety surfaces

Date: 2026-10-03. Status: steps 1 (writable connections), 2 (the save in
`tabletist-db` and the backend) and 3 (editing in the grid) are built, see
`docs/superpowers/plans/2026-10-03-writable-connections.md`,
`docs/superpowers/plans/2026-10-03-connection-write.md` and
`docs/superpowers/plans/2026-10-04-grid-editing.md`; steps 4 and 5 are not
yet planned.

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
| | within the scale the type states | "Up to 2 decimals. 12.505 would be stored as 12.51." |
| | within the digits it holds before the point | "At most 8 digits before the point" |
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
  rows", "1 to fix", **Discard all**, **Save** (`Mod+S`). Review SQL is
  step 4 and is not drawn yet. Save is disabled, with the reason as its
  tooltip: "Fix 1 value to save" while a cell is to fix, "Not connected",
  "This connection opens read-only" on a session that came back read-only,
  and "These changes cannot be sent: the table's key is not known" when no
  change set can be built from the set. While a save runs the bar reads
  "Saving…" with a Cancel, and Discard all is disabled. A line with nothing
  pending has **Dismiss** in place of the two buttons.
- Omarchy has no bar. Its status line shows the same counts ("3 pending · 2
  rows", "1 error") whenever the set is not empty, and why a save cannot be
  made, in the same words.
- The tab carries the unsaved mark: on macOS and Windows a dot in the close
  button's place while the tab is not hovered, and the tab reads "<name>
  tab, unsaved" to a screen reader; `[+]` after the name on Omarchy. A
  changed row carries its mark (see "The cell lifecycle"): amber, or red
  when one of its cells is to fix or failed, which is `~` and `!` in the
  Omarchy gutter.
- **Revert one cell:** `Mod+Z` on a pending cell that is active puts back
  the loaded value. The undo and redo stack is slice 4.
- The row panel shows a pending cell's new value with the pending mark and
  "was <loaded value>", so it never disagrees with the grid. It stays
  read-only: its Edit, Duplicate and Delete, and the header's Add row, stay
  disabled.
- Copying takes the pending value a cell shows, for the cell and for the
  row, in every look.

### Keys

| | macOS, Windows | Omarchy |
|---|---|---|
| Edit the cell | Enter, F2, double-click, typing | `i`, Enter, double-click (cursor at the end), `cc` (from nothing) |
| Commit and move down | Enter | Enter |
| Commit and move right, left | Tab, Shift+Tab | Tab, Shift+Tab |
| Leave the editor | Esc drops the edit | Esc keeps it, Ctrl+C drops it |
| Move the text into the popover | Alt+Enter | Alt+Enter |
| Apply in the popover | `Mod+Enter` | Ctrl+Enter |
| Set NULL | `Mod+Backspace` | `x` |
| Revert the cell | `Mod+Z` | `u` |
| Review SQL | step 4 | `:diff`, step 4 |
| Save all | `Mod+S` | `:w`, Ctrl+S |
| Discard all | `Mod+Alt+Backspace` | `:e!` |

- Typing opens the editor on macOS and Windows except for Space (the row
  panel) and `?` (the shortcuts), which keep their meaning. A chord types
  nothing.
- `Mod+S` saves wherever the bar offers Save: with something pending or an
  editor open, whatever has the keyboard (the grid, the tree, a button, a
  filter's field) and in the Structure view too. From an open editor it
  takes what is being typed. On Omarchy Ctrl+S is the same chord.
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
  It takes `w` and `e!`. `diff` arrives with Review SQL in step 4; until
  then it is "not a command", like any other text, which the line says
  until the next key.
- All of it is handled in `ui/keys.rs`. The shortcuts screen lists each
  look's own editing keys (`keys::shortcuts(look)`): chords on macOS and
  Windows, letters and the prompt on Omarchy.

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
- **Enter never discards.** It follows the button that has the keyboard:
  on Cancel it cancels, on Save it saves, and on Discard it does nothing.
  With the keyboard on no button it saves where Save is offered (macOS and
  Windows) and does nothing otherwise, so in a prompt without Save a stray
  Enter drops nothing. On Omarchy the letters answer, and not while a field
  has the keyboard or a key is held: a letter typed in the frame the box
  opens is text, not an answer.
- **What a prompt asks about cannot change while it is up.** Mod chords
  still reach the reducer under a dialog, and a click can be a frame behind
  it. So while the Leave prompt or the production confirmation is up the
  reducer drops the editing actions, the moves of the selection, and the
  actions that would put another dialog in its place
  (`dropped_under_a_prompt`), and the confirmation sends only the set it
  showed.
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

1. Every statement is built first. A value the builder cannot convert is
   `Failed` for its row before anything is sent.
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
  (a key that really holds one pays for this); and a changed column whose
  stored text is not UTF-8, as `Failed` and not as a conflict, since a
  conflict offers to write over what the file holds, which would still be
  unknown.
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
  before it is sent.
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
literal Review SQL shows (`12`, `1`) is the value the driver binds. Two
SQLite literals are written so that the shown text runs when pasted: text
holding a NUL as `('a' || char(0) || 'b')`, and an infinite float as
`9e999`.

The backend has `Command::Write { session, request, changes }` and
`Event::Written { session, request, result }`, queued and answered like
every request; the reducer finds the tab by the request. A save's request
is one of its tab's pending requests, so `Mod+.` (and the bar's Cancel)
cancels a running save like any query, through the session's cancel
handle; the `write` future is awaited to its end, never dropped, and rolls
back. A cancel only reaches a statement that is running: one that arrives
between two of a save's statements is lost on PostgreSQL and MySQL, and
the save goes on and commits. One that reaches the `COMMIT` before it
takes hold undoes the save, which answers `Cancelled`, and the tab says
"Save cancelled. Nothing was written."

What a save came to when it wrote nothing is kept with the set, which
stays as it was (`edit::Note`), and is said in the bar or in the Omarchy
status line:

- A connection lost during the save: "The connection was lost while
  saving. Reload to see what was written." A session swapped under a save
  (a reconnect) abandons the save and says the same: its answer, if one
  comes, finds no tab saving.
- A production confirmation answered after the session went: "Not
  connected. Nothing was sent."
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

Only when the workspace's environment is production
(`Environment::confirms_writes()`), every Save first asks, in a dialog
that lists every statement it would send (`Dialect::update_row(..).shown`
for each row, built once when the dialog opens). Both looks list the
statements in this step, since `:diff` does not exist yet.

- macOS and Windows: a band of the production red along the top, "Save 2
  changes to production?", the connection's name and database and "1 row
  in book_covers", the statements in a box that scrolls, "One transaction",
  **Cancel** and **Save to production**. Enter does not confirm: the button
  is pressed. Enter cancels with the keyboard on Cancel, and Esc cancels.
- Omarchy: the box with the danger border, its head with the `PROD` tag
  and "write 2 changes?", the connection's name and database, "1 row in
  book_covers" and the columns the save sets, the statements, "type write
  to confirm" and a field that takes the word; Enter confirms only when
  the field holds exactly `write`, Esc cancels. From step 4 on the box
  points at the `:diff` panel ("sql shown with :diff") in place of listing
  the statements: a production save then opens the panel if it is closed
  and the box sits beside it.
- A row whose statement cannot be built opens no confirmation: its cells
  fail with the builder's reason, as they would in the save.
- The confirmation holds the change set it showed. When it is confirmed,
  the save is sent only if the tab's pending set still makes exactly that
  set (floats compared by their bits, so a NaN is the same value as
  itself), no editor is open and Save is not disabled.

## Conflicts

In step 3 a conflict is a plain line. The pending bar reads "Row id 2
changed on the server. Nothing was written." (or "Row id 2 no longer exists
on the server. Nothing was written."), with "1 more row too." after it
when other rows conflict as well; the Omarchy status line says the same
after `≠ conflict`. The row is named by its key. The set stays as it was,
with no rebase: the user refreshes, which discards it, and edits again.

The dialog is step 5. `Conflicts` then opens it for the first conflicting
row:

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
   the dialog's box, the shipped screens.
2. `Connection::write`, the statement builder and the column classes, the
   catalog's `ColumnInfo.generated`, `IndexInfo.partial` and
   `IndexInfo.key_columns`, the row key rule, the backend command and
   event. Nothing in the UI saves yet.
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

Steps 1 to 3 are built.

## What step 3 leaves for steps 4 and 5

For step 4, Review SQL:

- The bar's **Review SQL** button and its drawer, and Omarchy's `:diff`
  panel. `run_command` (`src/app/editing.rs`) takes `w` and `e!` today and
  answers "not a command" to `diff`.
- The statements are built once, when the production confirmation opens
  (`WritePrompt::statements`). Review SQL needs them kept up to date in the
  reducer as the pending set changes.
- The Omarchy PROD box lists the statements itself. It should point at the
  panel instead.
- Omarchy's letters and its `:` are matched by the character typed, so on
  a keyboard layout without Latin letters `i`, `x`, `u`, `cc` and `:` do
  nothing. Insert mode is still reached there by Enter or a double-click;
  `:diff` would have no other way in.

For step 5, the conflict dialog:

- The reducer keeps of a `Conflicts` answer only the first row, whether it
  is gone, and how many others there are (`Note::Conflict`). The rows the
  server now holds are dropped; the dialog needs them.
- A conflict writes nothing and changes nothing in the set: no row is
  rebased yet.
- A MySQL `TIMESTAMP` as a changed column can miss a conflict in a repeated
  daylight-saving hour. The key case is locked; the cell case is not.

Open in the save as the grid shows it:

- A cancel between two of a save's statements is lost on PostgreSQL and
  MySQL: the save commits and says so. The bar's Cancel is honest only
  while a statement runs. Whether the Saving state should say more is
  open.
- The Leave prompt's Save, when the session went while the prompt was up,
  closes the prompt, saves nothing, drops the held action and leaves no
  line. The "A save is still running" prompt keeps its sentence if the
  save ends while it is up.
- A column with a list of allowed values is checked against the list
  before its class, so a SQLite `INTEGER` column whose list holds
  non-numbers passes the check and fails in the save.
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
- Reducer tests: the pending set, the checks, the leaving guard and its
  held action, rows replaced after a save, rebasing after each conflict
  choice, mixed conflict answers, and a tab with pending changes keeping
  its page across a reconnect and saving afterwards.
- Headless UI tests, in every look: the lifecycle, the keys, the popover,
  the bar, the three dialogs, locked cells saying why, the shipped screens'
  new texts.
- `src/shots.rs` has a scene for review of each editing state, on its
  Bookshop data and in every look: `edit-pending`, `edit-field`,
  `edit-large`, `edit-saved`, `edit-failed`, `edit-leave` and
  `edit-production`. They need a GPU and are run by hand. No test compares
  a screen with the design.

## Documents this changes

The main spec's success criterion 6, its sections 4.3 and 5.7 (Enter and
`i` on Omarchy) and its keyboard table; the SQL editor spec's intent, its
guard layers 2 and 3 and its cleanup list, restated for writable
connections; the crate documentation of `tabletist-db` ("nothing in this
crate writes") and the comment on `RowQuery::raw_where`; the README; the
shortcuts table in `ui/keys.rs`.
