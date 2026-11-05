# Export and Copy as, run 3: export a table

Run 1 (PR #248) gave both grids a selection. Run 2 (PR #271) wrote the six
formats (`crates/tabletist-core/src/export.rs`, `Writer::new` then
`start`, `row`, `finish`), remembered each format's options in
`export_options.json`, and built Copy as. This run exports a table: the
Export sheet (macOS) and float (Omarchy), the export itself in the
background on a read-only session of its own, its progress, done,
cancelled and failed states, and the table's entry points. Run 4 exports
a SQL result.

Design canvas: **Tabletist**, section **Export and Copy as**: boards
`macOS – Export, entry points`, `macOS – Export, sheet by format`,
`macOS – Export, progress and states`, `macOS – Export, dark`,
`Omarchy – Export, entry points`, `Omarchy – Export, sheet by format`,
`Omarchy – Export, progress and states`; section **Components**:
`macOS – Components, additions for Export` (progress toast, progress
pill, error toast, column row) and `Omarchy – Components, additions for
Export` (column list, radio list with counts, path line, status-line
progress). The design system gives the tokens only.

## Delivery

Four runs, a pull request each (Igor, 2026-10-30): 1 the selection
(merged, #248); 2 the writers and Copy as (merged, #271); **3 export a
table (this spec)**; 4 export a SQL result, with Run again without the
limit, the results bar, the inspector's Export row…, several exports at
once, the portal picker and path completions.

**Run 3 in two pull requests (Igor, 2026-11-03).** What run 3 holds is
larger than runs 1 and 2 together: a new streaming read in each driver, a
background job, two new dialogs with six option sets each, a stack of
toasts with buttons (the app's toast is one line that takes no click
today), and seven entry points, two of them new menus. The proposal:

- **3a (this pull request):** the export (job, streaming, temporary
  file, 0600, read-only session, errors, history, masking), the sheet and
  the float, every state (progress toast and pill, done, cancelled,
  failed; the status line), and the entry points that exist or are keys:
  the table header's **Export…**, **File › Export…** and **File ›
  Import…** (disabled), ⇧⌘E, `E` in the grid and the sidebar, `:export`,
  `:export <format>`, `:w <path>`, `:w! <path>`, `:export cancel`.
- **3b (the next pull request):** the sidebar's context menu (new: Open,
  Structure, DDL, Copy name, Export…, Import…), Quick open's `> Export
  reviews…` and `> Export a table…` and ⇧⌘E / `ctrl+e` on a picked
  table, the grid menu's **Export selection…** (run 2's D1), Copy as's
  **Export…** over the limit (run 2's D3), the system notification when
  the app is in the background, and ⌘⇧' to move focus to a toast.

Rows marked **3b** in the tables below say where each piece lands.

## Where this build differs from the boards or the prompt

Igor approved every row as written (2026-11-03):

| # | The board or the prompt | This build | Why |
|---|---|---|---|
| D1 | Files tables export like any other; DuckDB streams through a stepped statement | PostgreSQL, MySQL and SQLite stream. A Files connection joins when its driver lands, through the same stepped read as SQLite | The Files connection (PR #247) is not on `main`: there is no DuckDB driver to stream from |
| D2 | Columns: All columns · Visible only · Choose | All columns · Choose | The grid cannot hide a column, so Visible only would always equal All columns. It joins the day the grid can hide one |
| D3 | All rows matching the filter shows an estimate, "≈ 31,400" | Unfiltered: the table's estimate ("≈ 48,112") or its exact count. Filtered: the exact count once counted (the footer's Count, or the sheet's **Count exactly**), otherwise "not counted" beside Count exactly | No filtered estimate exists: today a filtered footer shows no total until it is counted. An `EXPLAIN` estimate per driver is its own piece of work |
| D4 | Whole table: an estimate from catalog statistics | The same, and on SQLite (whose catalog keeps none) no count until the tab is counted unfiltered | SQLite keeps no row estimate |
| D5 | Several exports run at once; Omarchy `2 exports · … · :export to list` | One export at a time. Export… while one runs says "An export is running. Cancel it first, or wait for it." (`an export is running · :export cancel`) | Run 4 builds several at once, as agreed on 2026-10-30 |
| D6 | Omarchy path line: completions (`ctrl+n/p`) and `ctrl+o` for the portal's save dialog | The path line is typed only; the foot does not offer `ctrl+o` | Run 4 builds both, as agreed |
| D7 | Review SQL shows the statement the export runs | Selected rows and Current page are written from the rows the tab has loaded: nothing runs, so Review SQL is disabled for them ("Nothing runs: these rows are the ones loaded") and the history records nothing | The prompt's selection covers loaded rows only, and a page is at most 10,000 rows already in memory. Running the page's `LIMIT … OFFSET` again could return other rows than the ones shown and selected |
| D8 | The temporary file is `.reviews.csv.tabletist-part` | `.reviews.csv.<6 random letters>.tabletist-part` | Two exports into one folder, or a leftover from a crash, never collide; the name still says what it is |
| D9 | Done toast: **Show in Finder** | Show in Finder on macOS, Show in folder on Windows and Linux, as the prompt says for Windows | The board names macOS only |
| D10 | A system notification when the app is in the background (macOS, `omarchy notify`) | **3b** | The app posts no notification today; each platform needs its own way |
| D11 | ⌘⇧' moves focus to the toast "as for other toasts" | **3b**. In 3a the toasts' buttons take a click; Cancel is also File › Export… while one runs (it reads "Cancel export") and `:export cancel` | No toast takes the keyboard today, so there is nothing to follow |
| D12 | The board's sheet is attached to the tab | The sheet is the app's centred modal, as every other sheet | Every sheet in the app is one; the native look is the same |
| D13 | Omarchy failed messages in the board and in the prompt's table read differently | The Omarchy look uses the board's terminal lines (`✗ disk full after 6.1 of ≈ 7.8 mb · partial file removed · :export to retry`); macOS uses the prompt's sentences | The board is the terminal's way of saying the prompt's sentence |
| D14 | The sidebar's context menu holds Open, Open in new tab, Structure, DDL, Copy name, Copy as SELECT, Export…, Import… | **3b**, and without Open in new tab and Copy as SELECT | The sidebar has no context menu today. Open in new tab and Copy as SELECT have no command to call |

## Decided while specifying

Not drawn or written anywhere. Igor approved every decision as written
(2026-11-03):

| # | Question | Decision | The other way |
|---|---|---|---|
| Q1 | Where the export runs | On a connection of its own, opened read-only from the tab's connection details and the secrets the tab connected with (through its own SSH tunnel when the tab has one), closed when the export ends | On the tab's session, which would hold every tab of the connection until the export ends |
| Q2 | The statement | `SELECT` the chosen columns, in their order, `FROM` the table, the tab's filters as `WHERE`, its sort then the primary key as `ORDER BY` (the page's order). Whole table: no `WHERE`, the primary key's order. It runs with its values bound; Review SQL and the history show them written in, as the history does for a filter | `SELECT *`, then the columns picked in the writer |
| Q3 | Streaming | PostgreSQL: a read-only transaction, `DECLARE … NO SCROLL CURSOR`, then `FETCH 5000` until it is empty. MySQL: `START TRANSACTION READ ONLY`, the rows read one by one from the server, unbuffered. SQLite: the read-only connection's statement stepped row by row. Batches of 5,000 rows go to the file writer through a channel that holds one batch, so at most two batches are in memory | A `LIMIT … OFFSET` loop, which can skip or repeat rows when the table changes |
| Q4 | Cancel | Cancel stops between batches and cancels the running `FETCH` or read through the connection's cancel handle, as the editor's Cancel does | Wait for the batch |
| Q5 | Failures, as they are told apart | Permission denied and disk full from the file system; connection lost from the driver; server timeout from PostgreSQL's `57014` with "statement timeout", MySQL's `3024` or MariaDB's `1969`; anything else "Couldn't export reviews: <the first line of the error>." with Try again | One "Export failed" |
| Q6 | A file that exists (Omarchy) | Export checks the path when it starts: "! exists · enter again replaces it" in the float, `! … exists · :w! to replace, or another name` for `:w`. The last step, the rename, never replaces a file that appeared meanwhile unless replacing was asked | Check at the end only |
| Q7 | Clipboard with rows that need a query | Clipboard is disabled when the known count is over 10,000. Otherwise the export reads at most 10,001 rows; over the limit (or past 20 MB) it copies nothing and says "That's more than 10,000 rows. Export to a file?" or "That's over 20 MB. Export to a file?" | Count first |
| Q8 | History | An entry with the new source **Export** (file word `export`): the statement, the rows written, the time, Ok, Cancelled or Error, read-only. It runs again in a SQL editor, as a read does. No rows, no file name | Also keep the file name |
| Q9 | Masked columns | `export::follows_masking()` returns `false`: files get the real values. When it returns `true`, each column that the connection's masked list (the person's and the administrator's) names is written as `•••`. Both are tested through `export::masks(follow, masked, object, columns)` | Follow the masking by default |
| Q10 | "3 unsaved changes" | Changed cells, new rows and rows to delete, added up | Changed cells only |
| Q11 | When the sheet's choices are remembered | When the export starts (format, its options); the rows and columns choices are not remembered | On every change |
| Q12 | The sheet's first destination | File, unless the last export of that format went to the clipboard and the rows still fit | Always File |
| Q13 | `:w <path>` | Writes at once: format from the extension, the rows matching the filter in the tab's sort, all columns, the format's remembered options. `~` is the home folder; a path that is not absolute is in the downloads folder. An unknown extension says `✗ no format for .xyz · csv tsv json jsonl sql md` | Open the float filled in |
| Q14 | `:export cancel` with nothing running | `no export is running` | Nothing said |
| Q15 | Omarchy `go open folder` | `go` opens the folder while the done line shows (4 s), as a key of the grid and the sidebar | A command `:export open` |
| Q16 | Which look picks how a file is named | The macOS look always asks with the system's save panel (on Linux through the portal); the Omarchy look always uses the path line, on any system | By the operating system |
| Q17 | Time left | Shown once 2 s have passed and the total is known: "about 20 s left", from the rows per second so far, rounded to 5 s under a minute and to the minute above | From the start |
| Q18 | Omarchy `E` in the sidebar | Opens the table under the cursor (as `enter` does) and its float | A float with Whole table only |
| Q19 | The downloads folder | `$XDG_DOWNLOAD_DIR`, else the system's Downloads folder (the `directories` crate's), else the home folder | `~/Downloads` alone |
| Q20 | Production | The sheet's header has the 4 px red strip and the `production` badge, the float the `PROD` badge, as the boards draw. No confirmation | — |

## The export (core and backend)

New in `crates/tabletist-core/src/export.rs` (no UI, no driver):

```rust
pub const BATCH: usize = 5_000;
impl Format { pub fn extension(self) -> &'static str;           // csv tsv json jsonl sql md
              pub fn of_extension(ext: &str) -> Option<Format>; }
pub enum Rows { Selected, Page, Matching, Whole }               // the sheet's choice
pub enum Destination { File { path: PathBuf, replace: bool }, Clipboard }
pub fn follows_masking() -> bool;                               // Q9
pub fn masks(follow: bool, masked: &[ColumnName], object: &ObjectRef, columns: &[&str]) -> Vec<bool>;
pub fn part_path(target: &Path) -> PathBuf;                     // D8, the name before the random letters
```

New in `crates/tabletist-db`: `Dialect::export_rows(&RowQuery, key,
binary, columns) -> Sql` and `Dialect::shown_export(…) -> String` (Q2,
over the private `rows_statement`), and

```rust
impl Connection {
    /// Streams `sql`'s rows in batches of `batch` into `to`, read-only.
    /// Returns the columns first, then the rows. Stops when `stop` is set.
    pub async fn stream_rows(&self, sql: &Sql, batch: usize,
                             to: mpsc::Sender<Streamed>, stop: &StopFlag) -> Result<u64>;
}
pub enum Streamed { Columns(Vec<ColumnMeta>), Rows(Vec<Vec<Value>>) }
```

with each driver's way of Q3.

The backend gains:

```rust
Command::Export { job: ExportId, how: ExportJob }   // ExportId(u64), from App::next_id
Command::CancelExport { job }
Event::Export { job, state: ExportState }
pub enum ExportState { Asked(Option<PathBuf>),      // the save panel's answer
                       Started { at: Instant },
                       Progress { rows: u64, bytes: u64 },
                       Done { rows: u64, bytes: u64, took: Duration, copied: Option<String> },
                       Cancelled, Failed(ExportFailure) }
```

`ExportJob` holds the connection's details and secrets, the statement or
the loaded rows (D7), the chosen columns, the format and its options, the
target (the table, and whether SQL INSERT starts with its CREATE TABLE,
which the job reads with `Connection::ddl`), the masking, and the
destination. The job, on its own task:

1. For a file, asks the save panel when the macOS look has no path yet
   (`Asked`), suggesting `<table>.<ext>` in the downloads folder; no
   answer ends it quietly.
2. Opens its connection (`Access::ReadOnly`) unless the rows are loaded.
3. Creates the temporary file next to the target with `tempfile`
   (0600 on Unix; the user folder's own ACL on Windows), writes the
   format's start, each batch's rows, the finish and a last line ending,
   flushing every batch, and sends `Progress` at most every 250 ms.
4. Syncs the file and renames it over the target (`persist`, or
   `persist_noclobber` unless replacing), then syncs the folder.
5. On cancel or failure removes the temporary file (`tempfile` removes it
   when dropped) and closes the connection.

The app keeps the running job in `App::exporting: Option<Exporting>`
(one at a time, D5) and the last finished one's toast. It records the
history entry (Q8) when the job ends, unless nothing ran (D7).

## The sheet (macOS) and the float (Omarchy)

`Dialog::Export(Box<ExportSheet>)` in `crates/tabletist-core/src/model.rs`,
with its reducers in `src/app/export.rs` and its drawing in
`src/ui/export.rs` (both looks; the terminal look's rows in
`src/ui/export_terminal.rs`). `ExportSheet` holds the tab, the format,
the rows choice and the counts it shows, the columns (All, or an ordered
list of included columns with a cursor), the format's options being
edited (a copy of `App::export`'s), the destination, the Omarchy path
line and its "exists" mark, and whether Review SQL is showing.

**macOS** (the `sheet by format` board): title "Export reviews", the
subtitle (environment badge, connection, `schema.table`, "1 filter",
"sorted by created_at ↓"; on production the red strip); then labelled
rows: **Format** (dropdown, the six names), **Rows** (radio group with
the count right-aligned and the second line in `ink-muted`), **Columns**
(segmented All columns · Choose; Choose shows "8 columns", "5 of 8, in
this order" and the column list), the format's options (as the board's
rows, through `widgets::segmented`, `checkbox` and a one-character
field for Other…), **Destination** (segmented File · Clipboard, its note
under it), then the notes (the unsaved changes warning, "Values the grid
cuts off at 256 KiB are exported in full. These options are remembered
for CSV."), then the footer: quiet **Review SQL** on the left, **Cancel
esc** and the primary button with its count ("Export ≈ 31,400 rows…",
"Export 1,000 rows…", "Copy 12 rows") on the right. ↩ is the primary
button; esc closes. The column list: rows of checkbox, name, type and the
⋮⋮ handle, 28 high in a bordered list; Space includes or leaves out the
focused row, ⌥↑ ⌥↓ move it, a drag on ⋮⋮ moves it, All and None below.

**Omarchy** (the `float by format` board): head `export reviews` and
`bookshop · dev · csv` (`PROD` on production), then the rows in the
150-wide label column: `format` (words `csv tsv json jsonl sql md`),
`rows` (the radio list with counts, `( )`/`(•)`), `columns` (`all
choose`; choose opens the column list `1 [x] id bigint`), the options as
words and `[x]` boxes, `destination` (`to file clipboard`), `save to`
(the path line, prefilled `<downloads>/<table>.<ext>`), the notes in
`warning` and `ink-muted`; foot `tab next · enter export · space choose ·
ctrl+r review sql · esc cancel`. Tab moves, space or enter chooses,
enter on the last row exports; in the column list space includes, `K`/`J`
move, `a` all, `n` none. Changing the format changes the path's
extension; typing a known extension changes the format.

**Review SQL** (both): the sheet's body becomes the statement (keywords
coloured as the editor does) and "Runs once, read-only, streamed to the
file in batches of 5,000 rows. Nothing is written to the database."; the
footer is Back · Cancel · the primary button (Omarchy `enter export ·
esc back`).

**Counts:** Selected rows (exact; disabled when nothing is selected),
Current page ("rows 1 to 1,000, as sorted", exact), All rows matching the
filter (D3), Whole table (D4). **Count exactly** sends the tab's existing
`CountRows` and the row shows the answer. The default choice: Selected
rows when there is a selection, otherwise All rows matching the filter.

**Clipboard** is disabled with "Clipboard: more than 10,000 rows, file
only" when the choice's known count is over 10,000 (Q7).

**Screen readers:** the sheet is a dialog named "Export reviews"; Rows is
a radio group whose options say their counts; the column list is a list
of checkboxes, and a move says "title moved to position 3".

## States

A new toast stack in `src/ui/toast.rs`, beside the one-line `say` (which
stays for every other message):

- **Progress toast** (macOS, bottom centre, the inverse toast colours,
  as the board): "Exporting reviews.csv", "18,200 of ≈ 31,400 rows" (or
  "18,200 rows" with no total), a bar, "about 20 s left" (Q17), **Cancel**
  and **×**. × hides it to the **pill**, bottom right ("reviews.csv 58%"),
  which a click opens again. A status region for screen readers,
  announced at most every 2 s ("Exporting reviews, 58 percent").
- **Done toast** for 4 s: "Exported 31,400 rows to reviews.csv · 7.8 MB ·
  18 s" with **Show in Finder** (D9); clipboard: "Copied 12 rows as TSV".
- **Cancelled** for 4 s: "Export cancelled. Nothing was saved: the
  partial file was removed."
- **Error toast** (`danger-soft`, one action, ×) until dismissed:

| Failure | macOS | Action | Omarchy (D13) |
|---|---|---|---|
| Permission denied | Can't write to ~/Shared/exports. Permission denied. | Choose folder… (the save panel again) | `✗ can't write ~/shared/exports/reviews.csv: permission denied · :w another path` |
| Disk full | The disk is full. 6.1 of ≈ 7.8 MB written. The partial file was removed. | Try again | `✗ disk full after 6.1 of ≈ 7.8 mb · partial file removed · :export to retry` |
| Connection lost | Connection to Bookshop · dev was lost after 18,200 of ≈ 31,400 rows. The partial file was removed. | Try again | `✗ connection lost after 18,200 of ≈ 31,400 rows · partial file removed · :export to retry` |
| Server timeout | The server stopped the query after 60 s (statement_timeout on Bookshop · production). | Review SQL (the sheet again, on Review SQL) | `✗ server stopped the query after 60 s (statement_timeout) · ctrl+r review sql` |
| Other (Q5) | Couldn't export reviews: <first line>. | Try again | `✗ couldn't export reviews: <first line> · :export to retry` |

Try again runs the same export again from the start. The ≈ total comes
from the count the sheet showed. Omarchy: the status line's right end
shows `export reviews.csv [███████░░░░░] 58% · 18,200/≈ 31,400 · :export
cancel` while running, `✓ exported 31,400 rows → ~/Downloads/reviews.csv
· 7.8 mb · 18 s · go open folder` for 4 s, `export cancelled · nothing
saved, the partial file was removed` for 4 s, and a failure until esc.
`:export` after a failure runs it again.

**Unsaved changes:** when the tab holds pending changes, the sheet warns
"3 unsaved changes in reviews aren't exported. Export reads what is saved
in the database. Save first ⌘S" (Omarchy `! 3 unsaved changes in reviews
aren't exported · :w first to include them`). Nothing to choose.

## Entry points (3a)

| Where | macOS | Omarchy |
|---|---|---|
| The table header (rows, read-only and editable) | **Export…** with ⇧⌘E, before Add row; it folds to its icon, then goes, before Add row does when room runs out | `E`, and `E export` in the grid's key line after `gy copy as` |
| The sidebar | ⇧⌘E on the table under the selection | `E` on the table under the cursor (Q18), `E export table` in its foot |
| The File menu | **Export…** ⇧⌘E (with a table in front; "Cancel export" while one runs), **Import…** disabled, no shortcut | — |
| Commands | — | `:export`, `:export <format>`, `:w <path>`, `:w! <path>`, `:export cancel` |

Bare `:w` still writes pending changes; `:w` with a path never touches the
database. `:export` with a failure showing runs it again (D13).

## Keys

New in `crates/tabletist-core/src/keymap.rs`, in their scopes, so
Settings › Keyboard lists them and the generated clash tests check them:

| Command | Scope | macOS | Omarchy |
|---|---|---|---|
| `Export` ("Export…") | Grid (a table's views), Tree | ⇧⌘E | `E` |
| `ExportLine` (`:export`, `:export {format}`) | Grid | — | `:export`, `:export {format}` |
| `ExportTo` (`:w {path}`, `:w! {path}`) | Grid | — | `:w {path}`, `:w! {path}` |
| `CancelExport` | Grid | (the toast's Cancel) | `:export cancel` |
| `OpenExportFolder` | Grid, Tree | (the toast's button) | `go` (Q15) |
| `ExportReviewSql`, `ExportColumnUp`, `ExportColumnDown`, `ExportAll`, `ExportNone` | Prompt (the sheet open) | (the button), ⌥↑, ⌥↓, (buttons) | `ctrl+r`, `K`, `J`, `a`, `n` |
| `Import` ("Import…") | Window | none, `built: false` | none |

## Tests (headless, no pixels)

Bookshop data only (the `reviews` table of the fixtures). Server tests in
a new `crates/tabletist-db/tests/export.rs`, listed in `ci.yml`'s
PostgreSQL and MySQL jobs; SQLite's run everywhere.

1. **Statement:** Matching and Whole as Q2 for each dialect: chosen and
   reordered columns, filters bound, the sort then the key; Review SQL's
   text is the history's.
2. **Streaming:** a 200,000-row SQLite table exports with no batch over
   5,000 rows and at most two batches held (counted through the channel);
   the same shape against PostgreSQL and MySQL.
3. **Read-only session:** the export's connection refuses an `INSERT` on
   every driver.
4. **Temporary file:** cancel, disk full (a writer that fails with
   `StorageFull`), permission denied (a folder without write permission,
   Unix) and connection lost (a stream that fails midway) each leave no
   file under the target name and no temporary file; success leaves the
   whole file, and an existing file is replaced only when asked.
5. **File mode** is 0600 on Unix.
6. **Rows:** selected, current page (loaded values, pending changes not in
   them), matching the filter, whole table; the counts exact or ≈ as
   stated; Count exactly sends one count.
7. **Columns:** chosen and reordered columns are written in that order;
   Space, ⌥↑ ⌥↓, All and None; `space`, `K`/`J`, `a`, `n`.
8. **Sheet and float:** each format's options change the writer's
   options and are remembered when the export starts; ↩ and esc; tab,
   space, enter; Review SQL and back; the format follows the typed
   extension and the extension the format.
9. **Unsaved changes:** the note shows only with pending changes, with
   their count, and the file holds the saved values.
10. **Clipboard limit:** disabled over a known 10,000; 10,001 read rows
    or 20 MB copy nothing and say why.
11. **States:** progress, the pill and back, done (with its words), Show
    in Finder sends a reveal, cancel, each failure's words and action,
    Try again sends the same job; the Omarchy status line for each.
12. **Commands:** `:export`, `:export json`, `:w <path>` (format from the
    extension), `:w! <path>`, `:w` alone still saving, `:export cancel`,
    an unknown extension, a file that exists.
13. **Entry points:** the header's Export… and its folding, the File
    menu's Export… and Import… (disabled), ⇧⌘E, `E` in the grid and the
    sidebar.
14. **History:** an export records an Export entry with the statement and
    no rows; Selected rows records nothing.
15. **Masked columns:** both answers of the decision, through `masks`.
16. **Keys:** the new commands in their scopes, free of clashes, listed in
    Settings › Keyboard.

## Out of scope for run 3

A SQL result's export, Run again without the limit, the results bar's
buttons, the inspector's Export row…, several exports at once, path
completions and the portal picker (run 4); 3b's pieces (above); import.
