# Export and Copy as, run 4: export a SQL result

Run 1 (PR #248) gave both grids a selection. Run 2 (PR #271) wrote the
six formats and built Copy as. Run 3 exported a table: 3a (PR #272) the
export on a read-only connection of its own, the sheet and the float,
every state; 3b (PR #273) the sidebar's menu, Quick open's rows, Export
selection…, Copy as's Export… over the limit, the notice and ⌘⇧'. This
run, the last, exports a SQL result, and builds what run 3 left for it:
several exports at once, and the Omarchy path line's completions and
`ctrl+o`.

Design canvas: **Tabletist**, section **Export and Copy as**: `macOS –
Export, entry points` (the SQL editor's results bar, the row panel's
Copy row ▾, Quick open's `> Export result of Query 1…`), `macOS –
Export, sheet by format` (the JSON Lines sheet: "SQL editor result · run
again without the limit"), `macOS – Export, progress and states`
("Several at once"), `Omarchy – Export, entry points` (the result's and
the inspector's key lines), `Omarchy – Export, sheet by format` (the
jsonl float), `Omarchy – Export, progress and states` ("two at once");
section **Components**: `macOS – Components, additions for Export` (two
progress toasts, "paused"), `Omarchy – Components, additions for Export`
(the radio list's "run again without the limit · not after a write", the
path line with its completion list and `ctrl+o pick`). Igor's prompt
(2026-10-30) wins where a board and it disagree.

## Delivery: two pull requests

Run 4 is about as large as run 3, which became two pull requests. Igor
approved the split (2026-11-04):

- **4a, export a SQL result:** a statement streamed on each engine
  (`Connection::stream_statement`), the result's sheet and float (Rows
  already loaded, Run again without the limit and its rule, Selected
  rows), its entry points (the results bar's **Copy as** and
  **Export…**, ⇧⌘E and `E` in the results, the row panel's **Export
  row…** and `E`, Quick open's `> Export result of Query 1…`, the result
  grid's **Export selection…**, a result copy's **Export…** over the
  limit, `:export`, `:export <format>`, `:w <path>`, `:w! <path>` in the
  results), the history.
- **4b, the rest:** several exports at once (the toast stack, a pill each,
  the Omarchy status line and its `:export cancel` picker, Quick open's
  cancel rows), and the path line's completions and `ctrl+o`.

4b changes 3a's one-export state into a list, which touches every place
that reads it; 4a adds no second export. Each lands whole on its own.

## Where this build differs from the boards or the prompt

Igor approved every row as written (2026-11-04):

| # | The board or the prompt | This build | Why |
|---|---|---|---|
| F1 | A result's Rows: Rows already loaded · Run again without the limit | Selected rows (disabled with no selection) · Rows already loaded · Run again without the limit | Export selection… on a result's grid (run 3b's E3), a result copy's Export… over the limit (3b's R6) and the row panel's Export row… on a result all export a selection, as on a table |
| F2 | "On PostgreSQL inside an open transaction, it is offered only after rollback" | No such state exists: every run is its own transaction, committed or rolled back as it ends, and the editor refuses `BEGIN`, `COMMIT` and `ROLLBACK` (`sql::refusal`). The rule is Show all's (`SqlTab::cut_statement`), on every engine | Tabletist keeps no transaction open between runs. The export's own session is read-only besides (3a's Q1) |
| F3 | After a read-write run the row shows "Not offered: this run changed data" | That, and two more reasons the row is greyed: "Not needed: every row is loaded" (the result was not cut) and "Not offered: the statement was edited since it ran" (the editor's text is not the one that ran, as Show all) | Show all is offered for exactly these; the row says why it is not |
| F4 | Progress toast, offline: "paused · offline, resumes on reconnect" | Not built: a lost connection fails the export with Try again, as in 3a | A cursor cannot resume in a new session, and a new read could return other rows. The prompt does not ask for it |
| F5 | Omarchy, two at once: `2 exports · reviews.csv 58% · events.jsonl 12% · :export to list` | `… · :export cancel`, and `:export cancel` with two or more running opens a picker of them (`1 reviews.csv 58%`), whose Enter or digit cancels the one picked | `:export` opens the float for a new export (the prompt's commands): one command cannot also list |
| F6 | Path line: "tab completion list" | The list follows the typing; `ctrl+n`/`ctrl+p` (and ↓ ↑) move in it, Enter takes the one picked; Tab still moves to the next row | The prompt: "Typing offers completions (`ctrl+n/p`, enter)". Tab is the float's next row (3a) |
| F7 | 3a: ⇧⌘E and File › Export… cancel the export while one runs ("Cancel export", 3a's B14 and D11); Quick open hides its export rows while one runs (3b's R2) | (4b) They always open the sheet: another export can start. Cancel is each toast's button, `:export cancel`, and a Quick open row per running export, "Cancel exporting reviews.csv" | With several at once Export… must start one more |
| F8 | One progress pill | (4b) A pill per hidden export, stacked up from the bottom right, newest at the bottom, as the toasts | The board draws one hidden export |
| F9 | Masked columns follow `export::follows_masking()` | A result is written as the editor shows it: the masking decision applies to a table's columns, and a result's columns name no table | Agents' masking is matched by table and column; a statement's columns may be computed. With the decision `false` (as merged) nothing changes |
| F10 | The results bar: "Results · Query 1 · statement 2 of 2 · 1,000 rows (limit reached) · Show all · 38 ms" | The bar keeps its words ("Statement at line 2 · 14 ms", Show all on the strip under the rows) and gains **Copy as** ⌥⌘C and **Export…** ⇧⌘E at its right end, on Results and Messages | Rewording the bar is not this run's; its words have their tests and their screen reader names |

## Decided while specifying

Not drawn or written anywhere. Igor approved every decision as written
(2026-11-04):

| # | Question | Decision | The other way |
|---|---|---|---|
| S1 | A result's first Rows choice | Selected rows when the sheet was asked for a selection (Export selection…, Export row…, a copy's Export…) or a range stands; otherwise Rows already loaded (the prompt) | Always Rows already loaded |
| S2 | What Run again runs | The shown statement's text as it ran, alone, on the export's own read-only connection to the tab's database, in a read-only transaction, streamed as 3a streams a table: PostgreSQL `DECLARE … NO SCROLL CURSOR FOR <statement>` then `FETCH 5000`; MySQL unbuffered; SQLite stepped. No client timeout; the server's own applies (3a's Q5). A statement the cursor cannot hold (PostgreSQL's `SHOW`, `EXPLAIN`) fails with the database's words | Through the editor's session, which would hold the editor until the export ends |
| S3 | The statement's check | Before anything is sent, the statement is refused unless `sql::kind` reads it as a read and `sql::refusal` passes, whatever the sheet says: "Run again without the limit is only for a statement that reads" | Trust the sheet |
| S4 | A result's names | Title "Export result", subtitle "Bookshop · result of Query 1, statement 2 · read-only transaction" (the statement's number among the run's, counted from 1); Omarchy head `export result` over `bookshop · dev · query 1, statement 2`; the file `query-1.<ext>`; SQL INSERT's target `result`, its Start with CREATE TABLE greyed ("Not for a result: no table to read") | — |
| S5 | Counts | Rows already loaded: the result's rows, "limit reached" when cut. Run again: "counted as it runs" (no total, so the toast says "4,000 rows"). The clipboard is disabled for Run again only once the count is known to be over 10,000; otherwise it stops at the 10,001st row as 3a's B16 | Count first |
| S6 | `:w <path>` and `:export` on a result | `:export` opens the float on the result. `:w <path>` writes at once what the float would with its defaults: Rows already loaded, every column, the format's options (3a's Q13 for a table writes the rows matching the filter, its float's default) | `:w` runs the statement again |
| S7 | History | Run again records an Export entry with the statement, the rows written, the time and its outcome, under the editor's name ("Query 1"). Rows already loaded and Selected rows record nothing (3a's D7) | — |
| S8 | Export row… | macOS: the last row of the row panel's Copy row ▾, after a separator, with ⇧⌘E; Omarchy: `E` in the inspector and `E export row` in its key line. It opens the sheet on Selected rows holding the panel's row, all columns; on a table and on a result alike | Only on a table |
| S9 | The results bar's buttons | **Copy as** opens run 2's Copy as menu at the cursor's cell, greyed while no cell is picked; **Export…** opens the sheet, greyed while the shown statement returned no rows or a run is in flight. Both on Results and on Messages. They fold to their icons, then go, before the bar's words do when room runs out (as 3a's header Export…) | Copy as on the whole result with no cell picked |
| S10 | ⇧⌘E and `E` in the results | Open the result's sheet (with a selection, on Selected rows, as 3b's R1 on a table) | — |
| S11 | Quick open's `> Export result of Query 1…` | Listed with "SQL editor" after it while a SQL editor whose shown statement returned rows is in front; it opens that result's sheet | Listed for every editor |
| S12 | (4b) How many exports at once | No cap: each has its own connection and its own toast | A cap of 4 |
| S13 | (4b) The toasts | Stacked at the bottom centre, newest at the bottom, 8 apart; each its own Cancel and ×; a done or cancelled one goes after its 4 s, a failure stays until dismissed. ⌘⇧' moves the keyboard to the newest toast's first button, and ⇥ walks every toast's buttons | Only the newest shows |
| S14 | (4b) The Omarchy status line | One running: 3a's line. Two or more: `2 exports · reviews.csv 58% · events.jsonl 4,000 rows · :export cancel`, in the order they started. An end (done 4 s, cancelled 4 s, a failure until esc) takes the line while it shows, the newest first; `go`, esc and `:export` (Try again) act on the one whose line shows | — |
| S15 | (4b) Completions | Read off the UI thread (AGENTS.md): the typed path's folder, the names that start with the typed last part (case aside), folders first with `/`, then files, by name, at most 8; dot files only once a `.` is typed; none for an empty line. Taking a folder lists its own; taking a file sets the format from its extension (3a). The list closes on esc (a second esc closes the float), on a pick, and when nothing matches | Tab completes as in a shell |
| S16 | (4b) `ctrl+o` | The float's `ctrl+o pick file` (back in its foot) opens the system's save dialog (on Linux through the portal, as the macOS look's panel), suggested from the line. Its answer fills the line, and a file it asked to replace is replaced; nothing starts until Enter | Export at once |

## Exporting a result (4a)

New in `crates/tabletist-db`:

```rust
impl Connection {
    /// Streams `statement`'s rows as `stream_rows` streams a table's,
    /// read-only: refuses one `sql::kind` does not read as a read.
    pub async fn stream_statement(&self, statement: &sql::Statement, batch: usize,
                                  to: &mpsc::Sender<Streamed>, stop: &StopFlag) -> Result<u64>;
}
```

with each driver's way of S2, sharing the cursor and the stepping that
`stream_rows` uses for a table's statement.

In the core, `ExportRows` gains `Statement(sql::Statement)`; `ExportJob`'s
`object` becomes the export's target (a table, or none for a result, which
SQL INSERT writes as `result`). `ExportSheet` gains its source, `Table {
id }` or `Result { id, statement, query }`, and `RowsChoice` gains
`Loaded` and `RunAgain`. A result's sheet reads its facts from the SQL
tab: the shown result's rows, its selection, `cut_statement` and why it is
not offered (F3).

The macOS sheet (the board's JSON Lines case): Format; Rows (Selected
rows, Rows already loaded "the result as shown" with "limit reached"
when cut, Run again without the limit "reads only · same statement, no
LIMIT, no timeout from Settings › Editor" and "counted as it runs", or
greyed with its reason); Columns (All columns · Choose, as 3a's D2);
the format's options; Destination; the footer's primary button "Run and
export…" for Run again, "Export 1,000 rows…" or "Copy 12 rows"
otherwise. Review SQL shows the statement as it ran (S2). The Omarchy
float as the jsonl board, with `after a read-write run only "rows
already loaded" is offered` as the board's note where F3's first reason
holds.

## Several exports at once (4b)

`App::exporting: Option<Exporting>` becomes `App::exports:
Vec<Exporting>`, in the order they started; every reader takes the one it
means by its `ExportId` (a toast's button, the save panel's answer, an
event) or the newest (S14). The macOS toasts and pills stack (S13, F8);
the Omarchy status line follows S14, and `:export cancel` F5.

## The path line (4b)

`Command::ListFolder { asked, typed }` reads the folder off the UI thread
and answers `Event::FolderListed { asked, names }`; the float keeps the
list for the line it was asked for, and drops an answer for a line since
changed. Drawn under the line, as the component board: the picked one in
`accent-soft`, the rest in `ink-muted`.

## Keys

| Command | Scope | macOS | Omarchy |
|---|---|---|---|
| `Export` | adds Results and Inspector | ⇧⌘E | `E` |
| `ExportLine`, `ExportAs`, `ExportTo`, `ExportOver` | add Results | — | `:export`, `:export {format}`, `:w {path}`, `:w! {path}` |
| `ExportPickFile` (4b) | Prompt, `When::Export` | — | `ctrl+o` |
| `PathNext`, `PathPrevious`, `PathPick` (4b) | Prompt, `When::PathListed` | — | `ctrl+n` ↓, `ctrl+p` ↑, Enter |
| `CancelWhichExport` picker keys (4b) | Prompt, `When::CancelExports` | — | digits, `ctrl+n/p`, Enter, esc |

## Tests (headless, no pixels; Bookshop data)

4a:

1. `stream_statement` on each engine: a `SELECT` of the books in batches,
   a write refused with nothing sent, a stop; PostgreSQL and MySQL in
   `crates/tabletist-db/tests/export.rs` (already in `ci.yml`'s lists).
2. Run again: offered after a cut read; greyed with each of F3's reasons;
   the job refuses a write (S3); it writes every row, with no limit and
   no timeout; its history entry has the statement; loaded rows record
   nothing.
3. The result's sheet: its title, rows and counts, Selected rows first
   with a selection, SQL INSERT's `result` and greyed CREATE TABLE,
   Review SQL's statement, the file name `query-1.csv`.
4. Entry points: the bar's Copy as and Export… (greyed as S9), ⇧⌘E and
   `E` in the results, Export row… on a table's and a result's row panel,
   `E` in the inspector, Quick open's row, the result grid's Export
   selection…, a result copy's Export… over the limit, `:export`, `:w`.
5. Keys free of clashes, listed in Settings › Keyboard.

4b:

6. Two exports at once: both run, each cancels alone, each ends with its
   own toast; the pills; ⌘⇧' and ⇥ over two toasts.
7. The Omarchy line for one, two, an end among runs; `:export cancel` with
   one and with two (the picker); `go`, esc and `:export` on the line
   shown.
8. Completions: listed off the UI thread (no disk read in the reducer),
   the filter, folders first, the eight, dot files, an answer for an old
   line dropped, ctrl+n/p and Enter, a folder taken lists its own, esc.
9. `ctrl+o`: the dialog is asked with the line's path; its answer fills
   the line and marks a replace; nothing starts until Enter.

## Out of scope

Import; a cursor that resumes after a lost connection (F4); rewording the
results bar (F10); masking a result's columns (F9).
