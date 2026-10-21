# Settings: the Editor tab

Date: 2026-10-19. Status: for planning. The first four decisions below
were put to Igor and answered on 2026-10-19; the rest are the prompt's
stated defaults or this spec's, named in each step's plan for a yes.

## Intent

The SQL editor runs, types and formats one way, set by the look and by two
menus in its toolbar. This adds the Editor tab of Settings, in both looks,
in the same `settings.toml` that General and Appearance write: how a query
runs (the mode of a new tab, the row limit, the timeout, what the run key
runs, whether a write without `WHERE` asks), how typing behaves (keys,
indent, completion, pairs, line numbers, the cursor's line, wrap), how
Format writes SQL, and what history is kept.

The designs are the "Settings · Editor" boards of the design canvas
Artifact (`macOS – Settings, Editor`, `Omarchy – Settings, Editor`), with
the SQL editor, transaction and Components boards for reference. They are
not copied into the repository.

Success: a user opens Settings, picks Editor, and every open SQL tab
follows each change at once. On Omarchy the change shows in the file pane
beside the options, and an edit in `$EDITOR` comes back live.

## Decisions

| Question | Decision |
|---|---|
| Delivery | One spec, eight steps, each its own plan and pull request (Igor, 2026-10-19). Step 1 is the pane, every key of the file, and the rows whose features exist. A row whose feature is not built yet is not drawn. |
| Row limit | The fetch cap there is today, not `LIMIT` added to the SQL (Igor): a PostgreSQL cursor fetches `n + 1`, MySQL sets `sql_select_limit`, SQLite stops stepping. The server stops early either way, nothing of the person's SQL is rewritten, and a write is never touched. The board's values and No limit are added. |
| Timeout | The app's timer there is today, not server-side timeouts (Igor): at the time a cancel goes to the server (PostgreSQL's cancel, MySQL's `KILL QUERY`, SQLite's interrupt) and the run rolls back. It covers MySQL's writes too, so the MySQL "SELECT only" hint is not needed. The board's values and Off are added. |
| `ctrl+w` on Omarchy | It stays close tab (normal mode) and vim's delete-word (insert). The Omarchy row's hint names the commands there are, `:ro` / `:rw` (Igor). |
| Look-specific defaults (open question 1) | As drawn: Standard keys, absolute line numbers and UPPERCASE on macOS and Windows; vim keys, relative numbers and lowercase on Omarchy. A key absent from the file takes the running look's; once written, every look uses it. |
| Current line on Omarchy (open question 3) | No row. It follows `editor.typing.highlight_line`, on by default. |
| MySQL timeout (open question 4) | Moot: the app's timer covers every statement (decision above). |
| Completion keyword case with Keep (open question 5) | Uppercase, as the prompt has it (step 3). |
| Old keys | `editor.sql_limit` and `editor.sql_timeout_secs` are read when the new key is absent. A change of the value writes the new key and takes the old one out. |

## Where the prompt, the boards and the app differ

| What is asked | What is built | Why |
|---|---|---|
| "Adds LIMIT n" | A fetch cap (above) | Igor's choice. |
| Server-side timeouts, the MySQL hint | The app's timer (above) | Igor's choice. |
| Omarchy hint "ctrl+w toggles per tab" | ":ro / :rw per tab" | Igor's choice. |
| macOS row-limit small print "Adds LIMIT to results you browse; exports get everything"; Omarchy hint "exports get everything" | Step 1: "Caps the rows of results you browse" / "caps results you browse" | Nothing adds `LIMIT`, and the editor's results have no export or copy-all to get everything. Step 2 adds Show all; the words follow what is there. |
| macOS subtitle "The SQL editor: how queries run, how typing behaves, and what is remembered." | The title alone until History is built (step 6) | Until then it says what the pane does not have. As Appearance did. |
| Omarchy hint "a visual selection always wins" | "a selection always wins" until vim (step 7) | There is no visual mode before it. |
| The macOS board puts Row limit and Timeout on one row, and Line numbers, Current line and Wrap on one row of three switches | A row each, labelled as the prompt's table has them | The prompt wins where the two differ; each control is then named by its own setting's label. |
| "Autocomplete never queries the database while you type" | Names the sidebar has not loaded yet are read once, the first time a statement names them, and kept, as today | Without it a table's columns are not offered until the tree is opened. Typing with everything loaded sends nothing, which the test checks. |
| Every row at once | A row each time its feature is built (Delivery) | Igor's choice. |

## The file

Step 1 adds four tables under `[editor]`, all read and written from step 1
on. A key whose feature comes in a later step is read, kept and reset, and
does nothing until that step.

    [editor.run]
    new_tab               = "read-only"   # read-only | connection
    limit                 = 1000          # 0 is no limit
    timeout               = "30s"         # 5s | 15s | 30s | 1m | 5m | off
    run                   = "statement"   # statement | all
    confirm_unsafe_writes = true

    [editor.typing]
    keys           = "vim"        # standard | vim
    indent         = 2            # 2 | 4 | "tab"
    autocomplete   = "typing"     # typing | manual | off
    close_pairs    = true
    line_numbers   = "relative"   # absolute | relative | off
    highlight_line = true
    wrap           = false

    [editor.format]
    keywords = "lower"     # upper | lower | keep
    commas   = "trailing"  # trailing | leading

    [editor.history]
    keep = 1000  # 0 keeps none

| Key | Values | Default | Step it acts in |
|---|---|---|---|
| `editor.run.new_tab` | `"read-only"`, `"connection"` | `"read-only"` | 1 |
| `editor.run.limit` | `0` (none), or 1 to 10,000 | `1000` | 1 |
| `editor.run.timeout` | `"<n>s"`, `"<n>m"` (1 s to 1 h), `"off"` | `"30s"` | 1 |
| `editor.run.run` | `"statement"`, `"all"` | `"statement"` | 1 |
| `editor.run.confirm_unsafe_writes` | boolean | true | 5 |
| `editor.typing.keys` | `"standard"`, `"vim"` | the look's | 7 |
| `editor.typing.indent` | `2`, `4`, `"tab"` | `2` | 3 |
| `editor.typing.autocomplete` | `"typing"`, `"manual"`, `"off"` | `"typing"` | 1 |
| `editor.typing.close_pairs` | boolean | true | 4 |
| `editor.typing.line_numbers` | `"absolute"`, `"relative"`, `"off"` | the look's | 1 |
| `editor.typing.highlight_line` | boolean | true | 1 |
| `editor.typing.wrap` | boolean | false | 4 |
| `editor.format.keywords` | `"upper"`, `"lower"`, `"keep"` | the look's | 3 |
| `editor.format.commas` | `"trailing"`, `"leading"` | `"trailing"` | 3 |
| `editor.history.keep` | `0` (none), or 1 to 100,000 | `1000` | 6 |

Reading and writing follow General's rules: only values that differ from
the defaults are written, with the person's comments, order and unknown
keys kept; a value the key cannot have is ignored by its line and
reported; a key absent takes its default (the look's, where the default
depends on the look). A value outside the menus' lists that the file can
have (a limit of 300, a timeout of `"2m"`) is shown as one more entry in
its place, as Rows per page shows one.

## Steps

1. **The pane and the run.** The Editor pane in both looks, every key
   above, the old keys read. Rows: New tabs run in, Row limit, Timeout,
   ⌘↩ runs (with a selection winning), Autocomplete, Line numbers, Current
   line highlight (macOS). Reset to defaults for the pane. Detailed below.
2. **Results.** "Limited to 1,000 rows · Show all", which runs the
   statement that was cut again with no limit; "Cancelled after 30 s · Run
   without timeout", which runs the cancelled run again with none. The row
   limit's words gain "Show all fetches the rest".
3. **Format.** Keyword case (UPPERCASE, lowercase, Keep) and commas
   (trailing, leading) in the formatter, Indent in the formatter and the
   Tab key, completion's keywords in the chosen case (uppercase with Keep).
   Rows: Indent, Keywords, Commas, and the Format section.
4. **Typing.** Close brackets and quotes; Wrap, with the gutter's numbers
   on a line's first row.
5. **Writes without WHERE.** The parse before a run; "Update every row in
   book_covers?" with the estimate from statistics, Cancel / Run anyway on
   macOS, `[y] yes  [N] no` in Omarchy's status line; `TRUNCATE` always
   asks; never on a read-only tab. Row: Confirm UPDATE or DELETE without
   WHERE.
6. **History.** Kept per connection in the app's data directory (the SQL
   text, when, how long, rows, ok / error / cancelled; never a result),
   trimmed to the limit, Don't keep asks whether to delete it, Clear
   history… and `:history clear` ask first. Row: Keep query history, and
   the macOS subtitle.
7. **Vim.** Normal, insert and visual modes in the editor, the mode in the
   status line, `:w` doing nothing, `=`, `==`, `gg=G`; switching applies
   at once and a tab in insert mode goes back to normal. Row: Keybindings,
   and "a visual selection always wins".
8. **Preview.** The real editor with the board's Bookshop query
   (publishers and `book_covers`), read-only, named "Editor preview", one
   Tab stop, showing every setting live; it never reaches the backend.

## Step 1: the pane and the run

### The pane

`SettingsPane::Editor` comes after Appearance, before Agents, in both looks.

macOS, from the board and the prompt, with their words:

- **Running queries**
  - New tabs run in: segments Read-only transaction, Follow connection. "The badge in each tab can still switch it".
  - Row limit: a menu of 100, 500, 1,000, 5,000, 10,000, No limit. "Caps the rows of results you browse".
  - Timeout: a menu of 5 s, 15 s, 30 s, 1 min, 5 min, Off.
  - ⌘↩ runs: segments Statement at cursor, Whole tab. Hint "A selection always wins". The key is the keymap's for Run statement.
- **Typing**
  - Autocomplete: segments As you type, On ⌃Space, Off. "Tables, views, columns and functions from the schema". The key is the keymap's for Complete.
  - Line numbers: a switch. On writes `"absolute"`, off `"off"`; a stored `"relative"` shows on and stays until the switch is used.
  - Current line highlight: a switch.

Omarchy, from the board, with its words:

- **running queries**
  - new tabs run in: read-only, follow connection. ":ro / :rw per tab".
  - row limit: stepped, `1000` … `no limit`. "caps results you browse".
  - timeout: stepped, `5s` … `off`.
  - ctrl+enter runs: statement, whole tab. "a selection always wins".
- **typing**
  - line numbers: relative, absolute, off.
  - autocomplete: as you type, ctrl+space, off. "schema objects".

The keys are General's: `j/k` move, `h/l` change, `space` toggles,
`ctrl+e` opens the file, `R` resets the option, `ctrl+h/l` move between
panes, `esc` closes; the file pane marks the cursor's line. Reset to
defaults on macOS asks "Reset all Editor settings to their defaults?" and
resets every key of the four tables, the ones without a row included.

Each control is named by its row's label and described by its small print
or hint; segments are radio groups, as every pane's are since #177.

### What each setting does

- **New tabs run in.** Read-only transaction: a new SQL tab opens in
  Read-only, whatever the connection. Follow connection: as today, the
  tab opens read-write where the session can write and the environment
  does not open read-only by default. A connection that cannot write keeps
  every tab read-only either way. A change affects new tabs only.
- **Row limit.** Every open tab's limit follows the setting, and the
  toolbar's Limit menu offers the setting's values and sets the setting.
  No limit keeps every row the server sends.
- **Timeout.** The same for the toolbar's Timeout menu; Off sets no timer.
  Cancel works as it does.
- **⌘↩ runs.** Statement at cursor runs the statement the cursor is in, as
  today; Whole tab runs every statement in order, stopping at the first
  error, as ⇧⌘↩ does (which stays Run all). A selection that is not empty
  runs what it covers, exactly: the text outside it is blanked, so each
  statement keeps its line and an error marks the right one.
- **Autocomplete.** As you type: as today. On ⌃Space: only the Complete key
  opens a list; one open goes on as you type. Off: no list opens.
- **Line numbers.** Absolute or relative in either look, or off: the
  gutter keeps the bar beside the statement Run executes, without the
  numbers' column.
- **Current line highlight.** Off: the cursor's line is not filled.

### Tests (headless, no pixels)

1. With no file, each look has its defaults (new tabs read-only, limit
   1,000, 30 s, statement, autocomplete as you type, highlight on; Standard,
   absolute and UPPERCASE on macOS, vim, relative and lowercase on
   Omarchy), and nothing is written until a change.
2. Every key round-trips through the file; only non-default values are
   written; comments and unknown keys survive; the old keys are read and
   replaced on the next change of their value; a bad value is ignored by
   its line.
3. A new tab opens read-only by default on a writable dev connection;
   Follow connection opens it read-write there; a read-only connection
   keeps it read-only either way; an open tab keeps its mode when the
   setting changes.
4. A change of the limit or timeout reaches every open tab; No limit sends
   no cap; Off sets no timer.
5. ⌘↩ runs the statement at the cursor, Whole tab runs all, a selection
   runs what it covers; `;` inside a string, a comment or a dollar quote
   does not split.
6. Autocomplete Manual opens only on the key; Off never opens; typing with
   the schema loaded sends nothing to the backend.
7. Line numbers off draws no number; relative and absolute number as set;
   highlight off fills no line.
8. The pane's rows change with the keys on Omarchy and with clicks on
   macOS, and every change is written to the file.
9. Reset to defaults resets the Editor keys only.

## Step 3: Format and indent

Date: 2026-10-20. Steps 1 and 2 are merged (#181, #183); the query
history of #197 has since built most of step 6.

### The rows

- macOS, Typing: **Indent**, segments 2 spaces, 4 spaces, Tab, first in
  the section as the board has it (Keybindings, before it, waits for
  vim).
- macOS, a **Format · ⇧⌘F** section between Typing and History:
  **Keywords** (UPPERCASE, lowercase, Keep) and **Commas** (Trailing,
  Leading). The key in the heading is the keymap's Format.
- Omarchy, typing: **indent**, stepped (`2 spaces` … `tab`), after line
  numbers; a **format · = in normal mode** section with **keywords**
  (`UPPER`, `lower`, `keep`, the case shown as it is) and **commas**
  (`trailing`, `leading`). `=` is the keymap's.

### What they do

- **Format** writes reserved words in the case set: upper, lower, or as
  typed (Keep); the words it would uppercase today are the ones it
  lowercases. Names, strings and comments never change, and Format stays
  one undo step. The formatter takes a `Style` (`tabletist_db::sql::format`:
  case, commas, a `CASE`'s indent); `format` keeps today's style for
  other callers.
- **Leading commas** stand on the river's edge, one column left of the
  list's items: `SELECT a` then `     , b`. Only the lists Format breaks
  (a `SELECT`'s and a `WITH`'s); a comma with a comment after it, on its
  line or the next, stays at the end of its line, so the comment stays
  where it was.
- **Indent** sets how far a `CASE`'s `WHEN` and `ELSE` stand in from it:
  two spaces or four; four for Tab, since the river is set in spaces.
  Today it is four whatever is set.
- **Tab** in the editor puts in an indent at the cursor (replacing no
  selection); with text selected, at the start of every line the
  selection touches but blank ones, and the selection covers the same
  text after (one that starts at a line's start grows to take in its new
  indent), its moving end kept. **Shift+Tab** takes one indent (a tab, or up to an indent's
  width of spaces) off the cursor's line or every selected line. Each is
  one undo step. Tab still accepts an open completion first. Both are
  keymap commands (`IndentLines`, `OutdentLines`), which the editor reads
  itself before its field does: a Tab with nothing selected becomes the
  indent's text in its place among the frame's events, so a Tab and text
  typed in one frame keep their order.
- **Completion** offers keywords in the case set, upper case with Keep,
  whatever case was typed.

### Departures, for a yes

| Asked, or today | Built | Why |
|---|---|---|
| Today: Format sets a `CASE`'s parts four in | Two in by default (the indent's default) | "Reformats … with … the indent from Typing" |
| Omarchy today: Format uppercases | Lowercase until set: the look's default | The board's lowercase default |
| Today: completion follows the case typed (`sel` gives `select`) | The case set, whatever is typed | "Keywords are inserted in the chosen keyword case" |
| egui today: Tab replaces a selection with a tab character; Shift+Tab takes a tab or four spaces off the cursor's line | An indent on every line touched; Shift+Tab one indent off each | What an editor's Tab does over a selection; the prompt says only "indent" |
| Indent Tab in Format | Four spaces | A tab cannot keep the river's columns |
| "Each one undo step" | A Tab at the cursor is typed text, and undoes with what is typed beside it, as text does; a line edit (a selection's Tab, Shift+Tab) is one step | Typed in its place among the frame's events, it keeps its order with the text around it |
| (where a leading comma goes) | On the river's edge, one column left of the list's items, only in a `SELECT`'s and a `WITH`'s list | No board draws leading commas |
| Omarchy board: "format · = in normal mode" | The same words, `=` from the keymap, read with the keyboard out of the text | Normal mode is the keyboard out of the text until vim (step 7) |
| (the in-app shortcuts sheet) | Indent and Outdent are not listed there; the website lists Tab and Shift+Tab | The sheet's rows are the Keyboard board's |

### Tests (headless)

1. Format writes each case and each comma side, keeps names, strings and
   comments, sets a `CASE`'s parts in by the indent, and is one undo.
2. Random scripts keep their tokens and hold on a second pass in every
   style (the formatter's own test, run in each of the twelve).
3. Tab puts in 2, 4 spaces or a tab; over a selection it indents each
   line and keeps the selection; Shift+Tab takes one off; each is an undo
   step; Tab still accepts an open completion.
4. Completion inserts keywords in each case.
5. The rows set the keys in both looks; the headings name the Format key.

## Step 4: Close brackets and quotes, and Wrap

Date: 2026-10-21. Step 3 is merged (#204).

### The rows

- macOS, Typing, in the board's order: Indent, Autocomplete, **Close
  brackets and quotes** (a switch), Line numbers, Current line highlight,
  **Wrap** (a switch). The board draws the last three as one row of
  switches; a row each, as step 1 has it.
- Omarchy, typing: line numbers, indent, autocomplete, **close brackets**
  `[x]`, **wrap** `[ ]`, as its board.

### What they do

- **Close brackets and quotes**, on by default:
  - `(`, `[`, `{`, `'`, `"` (and `` ` `` on MySQL and SQLite, where it
    quotes names) get their closing one after the cursor, the cursor
    between, where what follows is nothing, whitespace, a closing bracket,
    `,` or `;`. Brackets pair inside strings and comments too.
  - A quote is not paired inside a word (`it's`), right after the same
    quote (`''` escapes one), or inside a string, a quoted name or a comment, as
    the dialect's tokenizer reads them (dollar quotes, `#` on MySQL, a
    line comment with lines under it, one left open at the end). A
    prefixed string (`E'`, `N'`, `X'`) is not paired: its quote follows a
    letter.
  - A quote steps over its twin only where it closes the literal the
    cursor is in; Backspace takes an escaped `''` one at a time.
  - A closing one typed with its twin right after the cursor steps over
    it.
  - Backspace between an empty pair takes both out.
  - Typed over a selection, a bracket or a quote goes round it, which
    stays selected; one step of the undo history.
  - The rest is the field's own typing (the pair typed, the caret moved
    back, the twin stepped over, the second one deleted), so it undoes as
    typing does. Only in a frame of one edit, typed: a paste, an input
    method's text (one character too) and two characters in one frame go
    in as they come.
- **Wrap**, off by default: a long line wraps at the editor's width, which
  then scrolls down alone. The gutter numbers a line on its first row; the
  statement's bar and the cursor's line take in every row of their lines.
  A press on the gutter beside a row a line wraps to puts the cursor at
  the line's start, and the completion list hangs under a word that starts
  a wrapped row. A change applies at once and leaves the keyboard where it
  is.

### Tests (headless)

1. A bracket gets its closing one and the closing one steps over; the
   window's event shape (a key's press with its character) is paired; a
   paste and an input method's text are not.
2. A quote pairs where a string starts and not in a word, after a quote,
   or in a string or comment; a backtick only on MySQL and SQLite.
3. Backspace between an empty pair, and one quote of an escaped `''`; a
   bracket over a selection, made either way, keeps it selected and
   undoes in one step; off types it alone; Omarchy's insert mode pairs
   and normal mode types nothing.
4. Wrap: two lines numbered once each with the first wrapped to rows (a
   trailing line break and an empty script numbered too); the bar and the
   cursor's line as tall as the wrapped line; the keyboard stays; a press
   beside a wrapped row; the completion list under a word starting a row.
5. The rows in both looks, in order, with their defaults, setting the
   keys.

## Step 5: Writes without WHERE

Date: 2026-10-21. Step 4 is merged (#210).

### The row

**Confirm UPDATE or DELETE without WHERE**, a switch, on by default,
last in Running queries in both looks, as both boards draw it: "On every
connection that allows writes" on macOS, `[x]` and "connections that allow
writes" on Omarchy. Its words are step 1's, there already.

### What it does

- **Which runs ask.** A run sent to write (a tab in Read-write, a
  statement that writes) holding a statement that reaches every row of a
  table:
  - an `UPDATE` or a `DELETE` with nothing of its own to bound it: no
    `WHERE`, no `LIMIT` (MySQL's, SQLite's), no join condition (`ON`, in
    MySQL's statements of several tables). What stands in a subquery does
    not count; a `WITH` and an `EXPLAIN ANALYZE` in front are looked
    through, and a `WITH`'s own `UPDATE` or `DELETE`
    (`WITH d AS (DELETE FROM books RETURNING *) SELECT ...`) is asked
    about too. While the row is on;
  - a `TRUNCATE`, on PostgreSQL and MySQL, whatever the row says: it has
    no `WHERE` to leave out. SQLite has no `TRUNCATE`.
- Never a run that only reads, and so never one of a tab in Read-only:
  the database refuses its write, and the card offers to run it again,
  which asks then. Run without timeout asks again too: each is a run of
  its own.
- **The parse.** `sql::every_row` reads the verb and the tables after it
  off the statement's tokens: `UPDATE ONLY books b`, MySQL's
  `DELETE LOW_PRIORITY ... FROM` and its lists of several tables (named
  by alias, read as the tables the aliases stand for), SQLite's
  `UPDATE OR REPLACE`, `TRUNCATE TABLE a, ONLY b`, quoted names, a schema
  (of a name in three parts, the last two). Each table once. What it
  cannot read leaves the tables out, and the question still asks.
- **The estimate** is what the sidebar has of each table: PostgreSQL's
  `reltuples`, MySQL's `TABLE_ROWS`, as the catalog read them (0 counts as
  none), summed where every table has one. A name is looked up as
  completion looks one up, as each engine reads it: on PostgreSQL a name
  not quoted in lower case, and either exactly; on MySQL and SQLite
  whatever its case. Nothing is asked of the server before the question:
  where the sidebar has no count (SQLite keeps none, a schema not loaded)
  the question says no number.
- **macOS and Windows**, the Components board's Dialog: "Update every row
  in book_covers?" (Delete…, "Truncate book_covers?"), then "Line 3 has no
  WHERE. About 1,200 rows, by the database's statistics." Several
  statements: "Run 2 statements that change every row?" and a line each.
  **Cancel** and **Run anyway** (danger). Esc cancels; Enter answers as
  the button that has the keyboard, and cancels from anywhere else.
- **Omarchy**, the Components board's confirm prompt, which "replaces the
  status line": `update every row in book_covers? about 1,200 rows · line
  3  [y] yes  [N] no`, over a red rule. `y` (or `Y`) runs; `n` (or `N`),
  Enter and Esc cancel; each hint is its button for the pointer.
- Nothing is sent before the answer, and nothing else runs while the
  question is open. Run anyway sends the run as it was asked, in the mode
  Run gives it then, and nothing where the tab is on another database by
  then. After either answer the keyboard goes back into the editor's text
  where it was there when the question came, and stays out of it
  otherwise (Omarchy's keys out of the text).
- A production tab cannot write yet (no SQL run there is confirmed), so
  the question never meets the production confirmation. When that is
  built it carries these lines, one question and not two, as the writes
  spec of 2026-10-05 has it.

### Departures, for a yes

| Asked, or drawn | Built | Why |
|---|---|---|
| "`TRUNCATE` always asks" | Asks with the row off too, on PostgreSQL and MySQL | The row names UPDATE and DELETE; "always" read as whatever it says. The other reading: TRUNCATE follows the row |
| "with the row estimate from the database's statistics" | The sidebar's count, no query before the question; no number where it has none (SQLite, a schema not loaded) | The catalog's count is that estimate; asking the server first would put a round trip, and a failure, before the question |
| "without WHERE" | A `LIMIT` or a join's `ON` of its own counts as a `WHERE`; a `WITH`'s own `UPDATE` or `DELETE` is asked about | `DELETE FROM logs LIMIT 1000` and a join reach some rows, not every row; a `WITH`'s `DELETE` reaches them all |
| The writes spec (2026-10-05): "`TRUNCATE` is not asked about"; "This UPDATE has no WHERE" | It asks; the question names the table, as this spec's "Update every row in book_covers?" | This spec is newer |
| (the words the boards do not draw) | "Line 3 has no WHERE." / "takes out every row." (TRUNCATE); "Update every row of its table?" where no table can be read; "Run 2 statements that change every row?" with a line each, Omarchy's `lines 1, 3`; a TRUNCATE's tables' rows summed, only where each has a count | Only the one question is drawn |
| Omarchy: the question in the status line | A dialog drawn over the status line, transparent around it | The keyboard leaves the editor, as for every other question, so `y` and `n` type nothing |
| Omarchy `[N]` (the default) | `n`, `N`, Enter and Esc say no; `y` and `Y` yes | The design's capital marks the answer Enter gives; elsewhere a capital is Shift, so both cases answer |
| macOS: Cancel / Run anyway | Enter cancels unless Run anyway has the keyboard | Nothing that writes to every row is the default |
| (several statements) | One question, a line each | One answer for one run |
| (the card's Run again, Run without timeout) | Ask again | Each is a new run that writes |

### Tests (headless)

1. `sql::every_row`: each verb and dialect, the words before a table,
   `ONLY` before each, quotes, a schema, three parts, several tables and
   their aliases, a `WITH`'s `DELETE` and `UPDATE`, each table once,
   unreadable text; bounded statements (`WHERE`, `LIMIT`, `ON`) and reads
   are none; SQLite's `TRUNCATE` is none.
2. An `UPDATE` without `WHERE` asks on macOS and Windows, sends nothing,
   then Run anyway sends it to write and the editor has the keyboard;
   Cancel, Esc and Enter send nothing; a modified Enter answers nothing;
   the keyboard goes back only where it was.
3. A bounded write, a read and a tab in Read-only never ask; a table the
   sidebar does not know is asked about with no number; several
   statements in one question; off, `UPDATE` and `DELETE` run and
   `TRUNCATE` asks; the card's run and Run without timeout ask; nothing
   else runs under the question, and Run anyway sends nothing on another
   database; PostgreSQL's names looked up as it folds them.
4. Omarchy: the question in the status line's place; `n`, `N`, Enter and
   Esc cancel and type nothing; `y` runs.
5. The row in both looks, last in Running queries, setting the key.

## Out of scope

- The other Settings tabs.
- Per-connection overrides of these settings.
- Syncing history across machines.
- Format rules beyond keyword case, commas and indent.
