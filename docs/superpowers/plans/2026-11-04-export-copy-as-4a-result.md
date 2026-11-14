# Export and Copy as, run 4a: export a SQL result Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Export a SQL editor's result through run 3's sheet and float:
its Selected rows, its Rows already loaded, or its statement run again
without the limit (Show all's rule), from the results bar's **Export…**
(beside a new **Copy as**), ⇧⌘E and `E` on the result, the row panel's
**Export row…** and `E`, quick open's `> Export result of Query 1…`, the
result grid's **Export selection…**, a result copy's **Export…** over the
limit, and `:export`, `:export <format>`, `:w <path>`, `:w! <path>` in a
SQL editor.

**Architecture:** `tabletist_db::Connection::stream_statement` streams a
person's statement as `stream_rows` streams a table's (one shared body per
driver: PostgreSQL's cursor, MySQL's unbuffered rows, SQLite's stepped
statement behind the script fence), refusing what `sql::kind` and
`sql::refusal` do not let through. The core's job gains
`ExportRows::Statement { statement, pick }` (the columns chosen, by place)
and an optional table (`ExportJob::object`, `None` for a result: SQL
INSERT writes into `result`). `ExportSheet` gains its source
(`result: Option<ResultOf>`), `row_only` (Export row…) and two row
choices (`RowsChoice::Loaded`, `RowsChoice::RunAgain`); `ColumnPick::at`
keeps a column's place. The root crate's `App::open_export` opens a
table's sheet or, with a SQL editor in front, a result's
(`open_result_export`, `result_facts`, `result_job`, `run_again`); the
views draw the result's rows and subtitle (`src/ui/export.rs`), the bar's
buttons (`sql_results::bar_buttons`) and Export row… (`row_panel`).

**Tech Stack:** Rust, egui (crmne fork) with AccessKit, tokio; headless
tests through `src/testing.rs`; server tests against the test PostgreSQL
and MySQL.

**Spec:** `docs/superpowers/specs/2026-11-04-export-copy-as-4-result-design.md`
(F1–F10 and S1–S16, and the split into 4a and 4b, approved as written on
2026-11-04), over run 3's specs.

---

## Before Task 1

```bash
git add docs/superpowers/specs/2026-11-04-export-copy-as-4-result-design.md docs/superpowers/plans/2026-11-04-export-copy-as-4a-result.md
git commit -m "Plan run 4a of Export and Copy as: export a SQL result"
```

## Where this sits

Four runs, a pull request each (Igor, 2026-10-30): 1 (#248), 2 (#271), 3a
(#272) and 3b (#273) are merged. Run 4 is two pull requests (Igor,
2026-11-04): **4a, this plan**, and 4b (several exports at once, the path
line's completions and `ctrl+o`). Base: `main` at `a77ae0d0`.

## Decided while building

Not in the spec, or settled by the code. To put to Igor in the pull
request:

| # | The spec | Built | Why, and the other way |
|---|---|---|---|
| B1 | Keys: `Export` adds Results and Inspector | ⇧⌘E is read in the SQL editor's text too (macOS and Windows; `E` there is typed) | The board's shortcut table names "Editor"; a chord read only on the result's rows would do nothing from the script, where the keyboard is after a run |
| B2 | S8: `E export row` in the inspector's key line | `E` works on the row panel's fields; no line names it | The terminal look's row panel has no key line for its copies either (run 2's B5: `yy`, `gy`). The other way: a line for the three |
| B3 | (the result's key line) | The editor's status line adds `gy copy as` and `E export result` while the result's rows have the keys, as the board's results line | The board draws both; neither is said anywhere today |
| B4 | (where `go` and Esc on a failed export are read) | Also on a result's rows | A result's export ends where its rows are |
| B5 | S8: Export row… | ⇧⌘E with the row panel's field holding the keyboard exports its row, on a table too (3a exported the table from there) | The menu item names ⇧⌘E. The other way: the panel's chord the table's |
| B6 | S9: Copy as greyed while no cell is picked | Also greyed on Messages, where no cell shows | The menu opens at the picked cell |
| B7 | S9: they fold, then go | By the words' own width: the buttons fold to their icons, then go, before the words are cut | As 3a's header |
| B8 | F3: the greyed reasons | The terminal look's: `not after a write` (the component board), `every row is loaded`, `edited since it ran` | Short as the float's lines are |
| B9 | S2: the statement as it ran | Sent without a trailing `;`, and Review SQL and the history show it as the editor split it | One statement to a cursor |
| B10 | (a result that changes under the sheet) | The sheet's facts are gone once the editor shows another statement's rows, and it closes | A sheet is modal, so only a run already in flight could change it |
| B11 | S6: `:w <path>` writes the rows already loaded | The rows already loaded, every column, whatever is selected | `:w` is the float's defaults at once |
| B12 | S7: "Query 1" | The history's detail is "Query {n}", translated | As the tab names it |
| B13 | F9: a result is not masked | The job's masks are all off for a result | As F9 |
| B14 | S11: quick open's row | File › Export… exports the result in front too | The menu's item is the same command |
| B15 | S8: `E` and ⇧⌘E on the row panel export its row | On a table's panel. A SQL result's panel holds no field the keyboard can stand on (none of its fields can be edited), so its Export row… is the Copy row ▾ item, drawn there without a key; the keys on the result's rows export the result (or its selection) | The panel takes no keyboard. The other way: stops on a result's fields, for this alone |
| B16 | Keys: `:export`, `:export {format}`, `:w {path}`, `:w! {path}` "add Results" | They are bound in the SQL editor's scope, which the `:` prompt reads in a SQL editor; `:export` is `Export`'s line (3a has no `ExportLine`) | The prompt is the editor's, wherever its keyboard was |
| B17 | S9: "returned no rows" | A statement that returned rows but none of them (`SELECT … WHERE false`) counts as no rows: Export… greyed, and no key, line or quick open row opens the sheet. Nor while a run is in flight: its answer will replace the rows (`SqlTab::exports`) | A file of no rows has nothing in it. The other way: a header-only file |
| B18 | F3: three reasons | A fourth, as Show all is not offered then either: "Not offered: the connection is not ready" (`not connected`) while the tab's session is down or connecting | Show all's rule, whole |
| B19 | (a sheet over a result replaced) | A result's sheet keeps the answer it was opened on (`ResultOf::run`): another answer at the same statement closes it | Its Selected rows would otherwise be other rows at the same places |

## Files

| File | Task | What |
|---|---|---|
| `crates/tabletist-db/src/lib.rs`, `adapter.rs` | 1 | `Connection::stream_statement` and its checks |
| `crates/tabletist-db/src/pg.rs`, `mysql.rs`, `sqlite.rs` | 1 | One stream body each for a table's statement and a person's |
| `crates/tabletist-db/tests/export.rs` | 1 | `a_statement_streams_on_*` (already in `ci.yml`'s lists) |
| `crates/tabletist-core/src/backend/export.rs` | 2 | `ExportRows::Statement`, the optional table, the columns picked |
| `crates/tabletist-core/src/export_sheet.rs` | 2 | `ResultOf`, `NotAgain`, `RowsChoice::{Loaded, RunAgain}`, `row_only`, `ColumnPick::at`, `result_file_stem` |
| `src/app/export.rs` | 2, 3 | 2: what the core's change asks of it; 3: the result's sheet, facts and job, Export row… |
| `crates/tabletist-core/src/keymap.rs`, `model.rs` | 3 | The keys; `ExportOffer::row_only`, `About::CurrentResult` |
| `src/app.rs`, `src/app/commands.rs`, `src/app/editing.rs` | 3 | `ResultFacts`; quick open's row; the `:` lines in a SQL editor |
| `src/ui/export.rs`, `sql_results.rs`, `row_panel.rs`, `copy_as.rs`, `keys.rs`, `quick_open.rs`, `workspace.rs` | 3 | The sheet's result, the bar, Export row…, the grid menu, the keys, the key line |
| `src/ui/export_result_tests.rs`, `export_ways_tests.rs`, `mod.rs` | 3 | The new tests; two of 3b's that said a result had no export |
| `docs/_guide/*.md`, `docs/_reference/keyboard-shortcuts.md` | 3 | Exporting a SQL result, and its keys |

Every commit ends on `~/.cargo/bin/cargo fmt --all --check`,
`~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings`
and `~/.cargo/bin/cargo test --locked --workspace --all-targets`; task 3
also on `RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --locked
--workspace --no-deps` and `cd docs && bin/check`. Server tests run with
`TABLETIST_TEST_PG_URL=postgres://tabletist:tabletist@localhost:55432/tabletist`
and `TABLETIST_TEST_MYSQL_URL=mysql://tabletist:tabletist@localhost:53306/tabletist`.

---

### Task 1: Stream a statement on each engine

**Files:** `crates/tabletist-db/src/adapter.rs`, `lib.rs`, `mysql.rs`,
`pg.rs`, `sqlite.rs`, `crates/tabletist-db/tests/export.rs`.

`Connection::stream_statement(statement, batch, to, stop)` refuses a
statement `sql::refusal` refuses (`Error::Refused`) or one `sql::kind`
does not read as a read, with nothing sent; otherwise it streams it as
written, without its trailing `;`, through the body `stream_rows` uses
(each driver's `stream_rows` now builds its statement and hands it to
`stream_text` on PostgreSQL, `stream_sql` on MySQL, `stream_stepped` on
SQLite).

- [ ] **Step 1: Write the tests.** `tests/export.rs`: `streamed_statement`
  and `streams_a_statement` (a `SELECT` with a `WHERE`, an `ORDER BY` and
  a `;` in batches of 4; no rows; an `UPDATE`, a `DELETE`, an `INSERT`
  refused; `COMMIT` refused as `Error::Refused`; a stop; the session good
  after it), run as `a_statement_streams_on_sqlite`, `_on_postgres` (with
  `SHOW search_path` failing in the server's words) and `_on_mysql`.
- [ ] **Step 2: Run them, and see them fail.**
  Run: `~/.cargo/bin/cargo test -p tabletist-db --test export`
  Expected: `error[E0599]: no method named `stream_statement` found for reference `&tabletist_db::Connection` in the current scope`.
- [ ] **Step 3: Write the code** (the files above).
- [ ] **Step 4: Run the checks**, and the server tests:
  `~/.cargo/bin/cargo test --locked -p tabletist-db --test export` with
  both URLs. Expected: 16 passed, none skipped.
- [ ] **Step 5: Commit.**
  `git add crates/tabletist-db && git commit -m "Stream a SQL editor's statement read-only on each engine, as a table's rows are"`

### Task 2: The export job runs a statement

**Files:** `crates/tabletist-core/src/backend/export.rs`,
`crates/tabletist-core/src/export_sheet.rs`, `src/app/export.rs`.

The job takes `ExportRows::Statement { statement, pick }`: it streams it
through `stream_statement`, keeps the columns `pick` names in its order
(a column not at its place under its name, as when it ran, fails it:
"the statement's columns changed since it ran"), tells the statement in `Started`, and writes SQL INSERT
into `result` with no `CREATE TABLE` where `object` is `None`. The sheet's
types gain what task 3 draws; `src/app/export.rs` sets the table, the
columns' places and the new fields as before (`result: None`,
`row_only: false`), and treats the two new row choices as no table's.

- [ ] **Step 1: Write the tests.** In `backend/export.rs`:
  `a_result_runs_again_with_its_columns_chosen` (10 books, `title` then
  `id`, `INSERT INTO result (title, id)`, no `CREATE TABLE`, the
  statement told), `a_result_of_a_write_is_not_run_again` (a `DELETE`
  fails, nothing left, 12 books still),
  `a_result_whose_columns_moved_is_not_written`; in `export_sheet.rs` the first
  rows of a result and its file's stem.
- [ ] **Step 2: Run them, and see them fail.**
  Run: `~/.cargo/bin/cargo test -p tabletist-core --lib export`
  Expected: `error[E0308]: mismatched types` (the job's `object: Some(…)`) and
  `error[E0599]: no variant named `Statement` found for enum `backend::export::ExportRows``.
- [ ] **Step 3: Write the code.**
- [ ] **Step 4: Run the checks.** Expected: the core's export tests pass
  (47), the rest unchanged.
- [ ] **Step 5: Commit.**
  `git add crates/tabletist-core src/app/export.rs && git commit -m "Run a SQL result's statement again in the export job, into its chosen columns"`

### Task 3: The result's sheet, Run again and the ways in

**Files:** `crates/tabletist-core/src/keymap.rs`, `model.rs`;
`src/app.rs`, `src/app/commands.rs`, `src/app/editing.rs`,
`src/app/export.rs`; `src/ui/copy_as.rs`, `export.rs`, `keys.rs`,
`mod.rs`, `quick_open.rs`, `row_panel.rs`, `sql_results.rs`,
`workspace.rs`; `src/ui/export_result_tests.rs`,
`src/ui/export_ways_tests.rs`; `docs/_guide/browsing-data.md`,
`macos.md`, `omarchy.md`, `sql-editor.md`;
`docs/_reference/keyboard-shortcuts.md`.

- [ ] **Step 1: Write the tests.** `src/ui/export_result_tests.rs`:
  `the_results_sheet_names_it_and_starts_on_its_loaded_rows` (spec tests
  3: ⇧⌘E, its words, Run and export…, SQL INSERT's note),
  `the_results_float_says_it_the_terminals_way`,
  `run_again_is_greyed_with_its_reason` (F3's three, and nothing sent),
  `run_again_sends_the_statement_and_records_it` (test 2, S7),
  `loaded_rows_go_to_the_clipboard_as_loaded`,
  `the_results_bar_holds_copy_as_and_export` (S9, and no rows),
  `nothing_exports_while_a_run_is_in_flight` (B17, B19),
  `the_float_says_why_after_a_write` (F3's note),
  `run_again_needs_the_session_show_all_needs` (B18),
  `the_bars_words_stay_whole_while_its_buttons_fold` (B7),
  `the_terminal_looks_ways_export_the_result` (`E`, its key line, `:w`),
  `quick_open_exports_the_result_in_front` (S11),
  `export_row_exports_the_panels_row_alone` (S8). In
  `export_ways_tests.rs`, 3b's `a_results_grid_menu_has_no_export`
  becomes `a_results_grid_menu_exports_the_selection`, and
  `the_offer_follows_what_was_copied` checks a result's offer. The
  keymap's generated tests cover the keys (test 5).
- [ ] **Step 2: Run them, and see them fail.**
  Run: `~/.cargo/bin/cargo test --lib export_result`
  Expected: `error[E0425]: cannot find function `export_row` in module `crate::ui::keys`` and
  `error[E0609]: no field `result` on type `ExportFacts``.
- [ ] **Step 3: Write the code**, and the website's pages.
- [ ] **Step 4: Run the checks**, `cargo doc` and `docs/bin/check`.
- [ ] **Step 5: Commit.**
  `git add -A crates/tabletist-core src docs/_guide docs/_reference && git commit -m "Export a SQL result from its bar, its keys and its row panel, running it again without the limit where Show all would"`

## After Task 3

Push the branch and open the pull request: the summary, how a statement
streams per driver, Run again's rule (F2, F3), the keys added, F1–F10,
S1–S16, B1–B19, and what 4b holds. No screenshot goes to GitHub
unblurred.

## Checks by hand

On the Bookshop demo, in both looks: run `SELECT * FROM books` with a
limit of 5, export the result's loaded rows to CSV, then Run again
without the limit to JSON Lines; an `UPDATE … RETURNING` and its greyed
row; Export row… on a table's row and a result's; `:w ~/Downloads/query-1.md`.
