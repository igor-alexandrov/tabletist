# The Files connection type

A fourth connection type, **Files**, opens data files as tables: CSV,
TSV, Parquet, JSON and NDJSON, Excel, and `.duckdb` database files.
DuckDB reads them underneath, in the app's own process. Each file shows
in the sidebar as a table to browse, filter and query with SQL like any
other.

Design canvas: **Tabletist**, section **Connections**, boards
`macOS – Connection, Files` (and its `details` and `Test and list`
boards), `Omarchy – Connection, Files` (and `details and Test`), the
SQLite and Edit connection boards beside them, and in **Components**
`macOS – Components, additions for Files` and `Omarchy – Components,
additions for Files`.

## Delivery: four runs, a pull request each

Asked and answered on 2026-10-30 (the prompt asked for one pull request):

1. **Run 1 (this spec's detail): read-only end to end.** `Driver::Files`
   and `Dialect::DuckDb` through every match; the DuckDB session with a
   view per file; the sidebar, Structure, DDL view, SQL editor, History
   and quick open on it; agents Off or Read; the Type row on its own line
   with four options, kept values and the change-type question; Source
   (Folder or Files, the pickers, the found line); the safety box, on and
   fixed; Test success and the missing folder; the list rows (Files and
   SQLite); a path in the URL field; `connections.toml`.
2. **Run 2: reading CSV.** Reading CSV with its options and per-file
   overrides, the background check, the partial banner with its three
   fixes in the dialog and on the table view, the macOS "not allowed"
   state and security-scoped bookmarks, Omarchy path completions.
3. **Run 3: join.** Join with saved connections: DuckDB's `postgres`,
   `mysql` and `sqlite` attach, read-only rules, credentials at attach
   time.
4. **Run 4: editing.** Read-only off, the in-memory table with its row
   numbers, writing back through a temporary file and a rename, the
   conflict check, `.duckdb` in place, Excel refused.

## What DuckDB is asked to do (run 1)

Measured on DuckDB 2.0.0 (`duckdb` crate 1.20000.1, bundled), before
this spec:

- `allowed_paths = [<every file of the connection>]`, then
  `enable_external_access = false`, then `lock_configuration = true`:
  the views read their files; `read_csv` of any other file, `glob`,
  `COPY … TO` (even over a file of the connection: DuckDB writes a
  `tmp_` file first), `INSTALL` and `LOAD` all fail with a permission
  error, and no statement can set the options back.
- `BEGIN TRANSACTION READ ONLY` refuses `CREATE TABLE` in the in-memory
  database.
- The `parquet` and `json` readers are compiled in. ICU is not: a
  `TIMESTAMPTZ` reads in UTC, and the session's zone is UTC. Excel
  (`excel`) is not: it is installed on first use into the app's own
  folder for DuckDB's extensions, and loaded before the fence goes up.

So a Files session is fenced twice, as SQLite's is: DuckDB's own
configuration, which no SQL can undo, and a read-only transaction around
every script, page, count and agent read.

Found by breaking the draft: `allowed_paths` does not tell reading from
writing, so `EXPLAIN ANALYZE COPY … TO '<a file of the connection>'
(USE_TMP_FILE false)` rewrote that file. A third fence closes it: the
script guard refuses a DuckDB `EXPLAIN` that names any statement it
refuses (`COPY`, `EXPORT`, `ATTACH`, …), and every Files session opens
read-only whatever access it is asked for, a `.duckdb` file attached
`READ_ONLY`.

## Decisions taken with the user

| Question | Decision |
| --- | --- |
| One pull request | Four runs, a pull request each (above) |
| Agents' Tables and Masked columns on Files (no catalog or plan to enforce them by) | Off or Read, rows and timeout only; a rule that hides or masks anything on a Files connection refuses every agent read, saying why |
| `connections.toml`'s form for Files | The file's own: `[[connections]]`, `driver = "files"`, `[connections.source]` |

## Where run 1 differs from the prompt

| The prompt says | Run 1 does | Why |
| --- | --- | --- |
| `connections.toml` keys each connection by name: `[connections."Bookshop exports"]`, `type = "files"` | `[[connections]]` with `driver = "files"`, and `[connections.source]` with `folder = "…"` or `files = […]`; version 7, written only when a Files connection is saved | The file is a list of entries keyed by `id`, with `driver`; a name is not unique. A Tabletist that does not know Files must refuse the file, not drop the entry |
| Reading CSV and Join groups, collapsed | Not shown | Runs 2 and 3; a group that cannot open would promise what is not built |
| Open read-only, a switch | Shown on and fixed, with the note "Files open read-only." | Editing is run 4; the prompt's note says "Turn off to edit" |
| Test partial: a banner with three fixes | A file that cannot be read fails Test, naming the file and the reader's words | Run 2 |
| Agents: the Agents section on the Files form | Access (Off, Read), rows and timeout; no Tables or Masked columns control on a Files form | A file has no catalog to tell what a statement reads; hiding and masking are refused on a Files connection rather than half kept (a rule from an administrator that hides or masks anything there refuses every agent read, with the reason) |
| Omarchy: typing a path offers completions, `ctrl+n/p` | Not built | Run 2 |
| macOS security-scoped bookmarks | Not stored | Run 2 |
| DuckDB extensions bundled with the app | `parquet` and `json` are; `excel` installs on first use into the app's own folder (`<state>/duckdb-extensions`), and offline a connection with an Excel file fails to open with "Can't read Excel files: the Excel reader for DuckDB could not be installed (…)". No "Downloading…" status | The crate builds only those two in; the install happens inside the open, which reports no progress |
| Type above **Name and Group** | Type, then Name, then Environment | The app has no Group field: connections group by equal names |
| **Paste URL** in the connections list takes a path | A path typed into a new connection's URL field (the dialog's URL tab, `u`) makes it a Files connection over that folder or file | The list has no Paste URL in this build |
| The sidebar shows row counts ("≈" estimates, Parquet's exact) | The counts are read (Parquet's metadata, an estimate from a CSV's or NDJSON's first 256 KiB) and kept as the table's estimate, which the rows view uses as it does PostgreSQL's | The sidebar shows no row counts for any connection; no board draws one |
| The failure states in the footer status ("Can't open ~/Data/bookshop-exports: no such folder") | The Source field turns red with the board's sentence under it; a failed Test's words go to the message area above the footer, as every type's do | The footer's status shows only a running or a passed Test today |
| Saving asks "Change Bookshop from PostgreSQL to Files? …" | The question is the dialog's message, ending "Save again to change it."; the next Save changes the type | The dialog has no second sheet for a question; the delete question lives in the picker |
| The file row's name in medium weight | The folder part dimmer, the file's name in the text colour, both monospace | The look has no medium monospace role; adding one is a typography change of its own |
| Omarchy footer `enter choose` | A keymap command `FormChoose` (Enter, Space) that only the footer names: egui presses the focused choice itself | A key is written only through the keymap |

## Where run 1 differs from the boards

- Agents shows as the existing Agents part, not a collapsed section:
  the collapsed section row comes with Reading CSV in run 2, where it has
  a second user.
- The URL tab is disabled for Files with its tooltip; on Omarchy the
  header hint says `no url for files`.

## Run 1 in detail

### The engine

- `Driver::Files` (serde `files`, label "Files", dialect
  `Dialect::DuckDb`, no default port). Every `match` on `Driver` and on
  `Dialect` names the new variant: no `_` arm.
- `Dialect::DuckDb` quotes as PostgreSQL does (double quotes), and its
  tokenizer is PostgreSQL's (DuckDB's parser is PostgreSQL's). Agents'
  statements are read with sqlparser's `DuckDbDialect`.
- `ConnectSpec` gains `files: Option<FilesSpec>`, the source:
  `Source::Folder(PathBuf)` or `Source::Files(Vec<PathBuf>)`.
- Discovery (`tabletist_db::files`): a folder's supported files directly
  in it (`.csv`, `.tsv`, `.parquet`, `.json`, `.ndjson`, `.jsonl`,
  `.xlsx`, `.duckdb`, by extension, any case), sorted by name; hidden
  files, subfolders and other types are left out. A file list keeps its
  order. Each file's table name: the stem, lowercased, anything outside
  `[a-z0-9_]` turned into `_`; two files with the same name each take
  `_<extension>`. A `.duckdb` file is a schema by the same rule.
- Opening: an in-memory DuckDB, its extension folder the app's, then one
  view per file (`read_csv` with `header = true, auto_detect = true,
  sample_size = 20000` and `delim = '\t'` for `.tsv`; `read_parquet`;
  `read_json` with `format = 'array'` for `.json` and
  `'newline_delimited'` for `.ndjson`/`.jsonl`; `read_xlsx` with
  `header = true` for `.xlsx`; `ATTACH … (READ_ONLY)` for `.duckdb`),
  then the fence above.
- The adapter answers every call: the server version (`DuckDB v2.0.0`),
  the zone (`UTC`), no databases, schemas (`main` and each `.duckdb`),
  objects (each file a table, with Parquet's row count from its metadata
  and an estimate from the sample for the others, shown with "≈"),
  describe (DuckDB's columns and types, no keys, and the file's path,
  size and format), DDL (the `CREATE VIEW orders AS SELECT * FROM
  read_csv(…)` statement the app made, since DuckDB keeps it
  rewritten), rows, counts, lookups, scripts
  (read-only; a write is `Error::ReadOnly`), agents' reads, and writes
  (refused).
- Test reads each file's schema and its first rows, never a whole file:
  "Read 3 files · 2.4 GB · 41 ms". A missing folder: "Can't open
  ~/Data/bookshop-exports: no such folder".

### The form

- **Type** is its own row, first, above Name and Group (macOS), with the
  hint beside it: "Changes the fields below"; for Files "Opens CSV,
  Parquet, JSON, Excel and .duckdb files as tables"; for SQLite "One
  database file on this computer". On Omarchy `type` is the first row,
  `sqlite  postgresql  mysql  files`, each a Tab stop, Enter or Space
  chooses.
- Changing Type keeps Name, Group and Environment and every field typed,
  until the dialog closes.
- Saving a saved connection that has been opened under another type
  asks: "Change Bookshop from PostgreSQL to Files? Its server settings
  will be removed."
- A new Files connection starts with Environment Local, read-only on,
  and agents Read (the prompt: Read for Local, Off for production; every
  other type still starts Off). The Agents hint for Files: "Agents can
  query the files, never change them."
- **Source:** Folder or Files (`folder` / `files`). Folder: a read-only
  path with a folder icon and **Choose…** (`ctrl+o`). Files: a bordered
  list of file rows (icon, path with the name in medium weight, type,
  size, × "Remove orders.csv"), and **Add files…** (`a`; `x` removes the
  row under the cursor, `↑↓` move). The pickers: rfd, folders only for
  Folder, several files for Files (NSOpenPanel on macOS, the portal on
  Linux).
- **What was found**, under the source, after every change: "4 files ·
  2.5 GB · CSV, Parquet, NDJSON" and the table names as tags (macOS);
  `4 files · 2.5 gb · csv, parquet, ndjson → orders, customers, events,
  returns` (Omarchy). Read on the backend.
- **Safety:** the box as for the other types, checked and fixed, with
  "Files open read-only."; Omarchy `[x] files open read-only`.
- Header: the URL tab disabled with the tooltip "Files connections have
  no URL. Choose a folder or files under Source." (the macOS Source box
  takes no paste); Omarchy `no url for files`. A folder or file path
  typed into a new connection's URL field makes it a Files connection
  with it.
- Omarchy footer for Files: `tab next · enter choose · ctrl+o pick ·
  ctrl+t test · ctrl+s save · esc cancel`.

### The list

- Files: "Files · 4 files · 2.5 GB" under the name; the folder (or the
  first file names, then the folder) and the table names in the middle;
  "Read-only" and "No joins" where TLS and SSH go. Omarchy:
  `files · ~/Data/bookshop-exports · 4 files · 2.5 gb`.
- SQLite: "SQLite · 12.4 MB" under the name, the folder over the file's
  name in the middle.
- Sizes and counts come from a scan on the backend when the list loads
  and after a save.

### Accessibility

- Type is a radio group named "Type"; the Source kind is one named
  "Source".
- The file list is a list; each × is a button "Remove orders.csv".
- The disabled URL tab keeps its tooltip for the keyboard.

### Tests (headless, Bookshop data only)

Fixtures are written into a temporary folder by the tests themselves:
`orders.csv`, `customers.parquet`, `events.ndjson`, `returns.csv`, a
TSV, a JSON array, a `.duckdb` file, a hidden file, a subfolder and a
`.txt` file. No test reads an Excel file: its reader is downloaded at
run time.
