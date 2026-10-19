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

## Out of scope

- The other Settings tabs.
- Per-connection overrides of these settings.
- Syncing history across machines.
- Format rules beyond keyword case, commas and indent.
