# Export and Copy as, run 3a: export a table Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Export a table to a file or the clipboard: the Export sheet
(macOS) and float (Omarchy), the export on a read-only connection of its
own that streams the rows into a temporary file renamed into place
(0600), its progress, done, cancelled and failed states, the history's
Export entry, the masking decision, and the table's entry points that
exist or are keys (header Export…, File › Export…, ⇧⌘E, `E`, `:export`,
`:export <format>`, `:w <path>`, `:w! <path>`, `:export cancel`).

**Architecture:** `tabletist_db::Connection::stream_rows` reads a table's
rows in batches through a channel of one batch (a PostgreSQL cursor,
MySQL's unbuffered rows, SQLite's stepped statement), from
`Dialect::export_rows`. `tabletist_core::backend::export::run` opens the
connection, writes each batch through run 2's `export::Writer` into a
`tempfile` beside the target, renames it (`persist`, or
`persist_noclobber`), and tells `Event::Export`. `export_sheet` holds the
sheet's state (`ExportSheet`, `ExportChange`) and the running export's
(`Exporting`, `ExportStatus`) with their words. The root crate reduces
them in `src/app/export.rs` and draws them in `src/ui/export.rs` (sheet
and float), `src/ui/export_toast.rs` (macOS toasts) and
`workspace::export_line` (the Omarchy status line).

**Tech Stack:** Rust, egui (crmne fork) with AccessKit, tokio, tempfile;
headless tests through `src/testing.rs`; server tests against the test
PostgreSQL and MySQL.

**Spec:** `docs/superpowers/specs/2026-11-03-export-copy-as-3-table-design.md`.
Igor approved the split into 3a and 3b, D1–D14 and Q1–Q20 as written
(2026-11-03).

---

## Before Task 1

```bash
git add docs/superpowers/specs/2026-11-03-export-copy-as-3-table-design.md docs/superpowers/plans/2026-11-03-export-copy-as-3-table.md
git commit -m "Plan run 3a of Export and Copy as: export a table"
```

## Where this sits

Four runs, a pull request each (Igor, 2026-10-30): 1 the selection
(#248, merged), 2 the writers and Copy as (#271, merged), 3 export a
table, 4 export a SQL result. Run 3 is two pull requests (Igor,
2026-11-03): **3a, this plan**, and 3b (the sidebar's context menu, Quick
open's export rows and ⇧⌘E / `ctrl+e` on a picked table, the grid menu's
Export selection…, Copy as's Export… over the limit, the system
notification, ⌘⇧' to a toast). Base: `main` at `4e103430`.

## Decided while building

Not in the spec, or settled by the code. To put to Igor in the pull
request:

| # | The spec | Built | Why, and the other way |
|---|---|---|---|
| B1 | (the sheet's first format) | The format of the last export since the app started, else CSV | The next export is most often in the same format. The other way: always CSV |
| B2 | Quick open's export rows are 3b's | File › Export… is a quick open command, as every File menu command is: "Export…", about the tab in front | The menu's tests hold each menu command to quick open's list. 3b names it "Export reviews…". The other way: keep it out of the menu until 3b |
| B3 | File › Import…, disabled | A menu item of its own, always greyed; the keymap lists Import… as not built, with no key | Quick open lists only built commands. The other way: no keymap line |
| B4 | `E export` after `gy copy as` in the grid's key line | Last, after `:diff review` | The line gives way from its end on a narrow window: `:w write` and `:diff review` matter more while changes are pending. The other way: the board's place |
| B5 | The notes in the sheet's flow | Under the scrolling choices, above the footer, always in sight | A long format (CSV) scrolls; the unsaved changes warning must not scroll away. The other way: in the flow |
| B6 | ↩ exports; Omarchy: space or enter chooses, enter on the last row exports | macOS: ↩ always exports. Omarchy: Enter exports from the save to line or with nothing focused; on a control it chooses | As the spec, made exact |
| B7 | "after 60 s" | Times up to two minutes are in seconds, then minutes and seconds | The board's words for a minute's timeout |
| B8 | "3 unsaved changes in reviews aren't exported" | One change reads "1 unsaved change in reviews isn't exported", and Omarchy "… :w first to include it" | Grammar |
| B9 | (the export toast and the one-line toast) | The export's toast floats 48 higher while the one-line toast says something | They would overlap |
| B10 | (a failure in the history) | An Export entry with Error, the rows written, and a few words: "the disk is full", "the connection was lost", "the server's statement timeout", or the error's first line | The history keeps a message for an error |
| B11 | `:export cancel` | Read where `:export` is: with a table in front | Run 4 adds a result's export, and its scope |
| B12 | Show in Finder | `Command::Reveal { path }`, the settings file's reveal (`open -R`, `explorer /select`, the folder with `xdg-open`) | One way to show a file |
| B13 | Omarchy timeout: `· ctrl+r review sql` | `· :export review sql`: `:export` after a failure does what its line offers (again, Review SQL after a timeout, the float for another name after "exists" or "permission denied") | Ctrl+R is redo in the grid. The other way: Ctrl+R in the grid while a failure shows |
| B14 | (⇧⌘E while one runs) | ⇧⌘E cancels it, as File › Export… does then (D11) | The menu's key does what its item says. The other way: "An export is running" |
| B15 | At most two batches in memory | Markdown keeps its rows until the end, to pad its columns (run 2's Q7): a large Markdown file holds the table | The padding needs every row. The other way: cap Markdown exports, or no padding past 10,000 rows |
| B16 | The clipboard reads at most 10,001 rows | It stops at the 10,001st row or at 20 MB of values as they arrive; the database may have sent one batch more | The stream reads by batch. The other way: a 10,001-row LIMIT for the clipboard |
| B17 | Clipboard: "Copied 12 rows as TSV" | That line in the one-line toast (the Omarchy status line), as Copy as says it | The board's "copied" state is that line |
| B18 | (a file made while the export ran) | macOS: "~/Downloads/reviews.csv already exists. Nothing was replaced." with Choose folder…; Omarchy: `! … exists · :w! to replace, or another name` | The rename never replaces a file it was not told of (Q6) |
| B19 | (over the clipboard's limit after it started) | A failure: "That's more than 10,000 rows. Export to a file?", until dismissed, with no action | 3b adds Export… to it |
| B20 | D7: nothing runs for loaded rows | Except SQL INSERT's Start with CREATE TABLE, which reads the table's definition on a read-only connection | The definition is the database's |
| B21 | Selected rows | A new row not saved yet is left out | It is not in the database |
| B22 | Q18 is the Omarchy `E` | ⇧⌘E on the macOS sidebar opens the table too, then the sheet | One way, both looks |
| B23 | The spec's keys | `ExportAll`/`ExportNone` are `ExportAllColumns`/`ExportNoColumns`. New: `ExportAs` (`:export {format}`), `ExportOver` (`:w! {path}`), `DismissExport` (Esc, `When::ExportFailed`), `ExportNow` (Enter), `CloseExport` (Esc), and `ExportNext`/`ExportChoose`, which name the dialog's own Tab and Space in the float's foot, as `FormNextField` does in the connection form | Every key the sheet names is the keymap's |
| B24 | The spec's sketch of the code | `Event::ExportPath` (not `ExportState::Asked`), `ExportState::Started { statement }`, `Command::Export(Box<ExportJob>)`, `backend::export::Destination`, `part_beside`; the toasts in `src/ui/export_toast.rs`, the float's rows in `src/ui/export.rs`, the status line's in `workspace::export_line` | Where the code fell |
| B25 | Q2: Review SQL shows what runs | The history keeps the statement the driver ran (`Connection::export_statement`, the same key and binary columns as the stream); Review SQL is the app's, from the tab's structure: the same text once the structure has loaded | Review SQL is shown before anything runs |
| B26 | Q5: PostgreSQL's timeout | Told by its message ("statement timeout"): on a server whose messages are in another language it reads as a cancel | The code alone is a cancel's too. The other way: set `lc_messages` on the export's session, which needs a superuser |
| B27 | `<table>.<ext>` | A name's `/ \ : * ? " < > \|` and controls are `_`; a name that is nothing but dots is `export` | A table may be named what a file may not |
| B28 | Q6: the float checks the path when the export starts | No disk is read on the UI thread: the rename refuses a file that is there, and the float opens again with "! exists · enter again replaces it" (`:w` says `:w!`); the save panel's choice replaces, as the panel asked first | AGENTS.md keeps disk work off the UI thread (the PR review). The other way: a stat on the backend before the export |

## Files

| File | Task | What |
|---|---|---|
| `crates/tabletist-db/src/dialect.rs` | 1 | `export_rows`, `shown_export`, a select list for `rows_statement` |
| `crates/tabletist-db/src/stream.rs` | 1 | `Streamed`, and sending on a channel of one batch |
| `crates/tabletist-db/src/lib.rs`, `adapter.rs` | 1 | `Connection::stream_rows`, `Connection::export_statement` |
| `crates/tabletist-db/src/pg.rs` | 1 | A cursor and `FETCH 5000`; `statement_timeout` as `Error::Timeout` |
| `crates/tabletist-db/src/mysql.rs`, `agent.rs`, `agent/run.rs` | 1 | Unbuffered rows; MySQL's and MariaDB's timeouts as `Error::Timeout` |
| `crates/tabletist-db/src/sqlite.rs` | 1 | Stepped rows in one blocking job |
| `crates/tabletist-db/src/agent/hidden/masked.rs` | 1 | `ColumnName::names` public, for the masking decision |
| `crates/tabletist-db/tests/export.rs`, `.github/workflows/ci.yml` | 1 | The stream on each engine; CI's server jobs run it |
| `crates/tabletist-core/src/backend/export.rs` | 2 | The job |
| `crates/tabletist-core/src/backend.rs` | 2 | `Command::Export`, `CancelExport`, `Reveal`; `Event::Export`, `ExportPath`; `Backend::ask_export_path` |
| `crates/tabletist-core/src/export.rs` | 2 | `Format::extension`, `of_extension`, `of_path`; `BATCH`; `follows_masking`, `masks` |
| `crates/tabletist-core/src/history.rs`, `src/app/history.rs`, `src/ui/history.rs`, `src/ui/history_tests.rs` | 2 | `Source::Export`, and the History tab's words for it |
| `crates/tabletist-core/src/export_sheet.rs`, `lib.rs`, `src/lib.rs` | 2 | The sheet's state and the running export's, with their words |
| `crates/tabletist-core/src/model.rs`, `keymap.rs` | 2 | `Dialog::Export`, the actions, the keys |
| `src/app.rs`, `src/app/export.rs`, `src/app/editing.rs`, `src/app/commands.rs`, `src/app/menu.rs`, `src/menu.rs` | 2 | Reducers, events, the `:` lines, File › Export… and Import… |
| `src/ui/export.rs`, `src/ui/export_toast.rs`, `src/ui/mod.rs` | 2 | The sheet and float; the macOS toasts |
| `src/ui/workspace.rs`, `src/ui/sidebar.rs`, `src/ui/keys.rs`, `src/ui/data_view.rs` | 2 | The status line, key lines and keys; the header's Export… |
| `src/ui/export_tests.rs`, `tests/environments.rs`, `tests/keys.rs` | 2 | The headless tests; the two repository checks that list files |
| `docs/_guide/*.md`, `docs/_reference/*.md` | 2 | Exporting, and its keys |

Every commit ends on `~/.cargo/bin/cargo fmt --all --check`,
`~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings`
and `~/.cargo/bin/cargo test --locked --workspace --all-targets`; task 2
also on `RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --locked
--workspace --no-deps` and `cd docs && bin/check`. Server tests run with
`TABLETIST_TEST_PG_URL=postgres://tabletist:tabletist@localhost:55432/tabletist`
and `TABLETIST_TEST_MYSQL_URL=mysql://tabletist:tabletist@localhost:53306/tabletist`.

---

### Task 1: Stream a table's rows on each engine

**Files:** `.github/workflows/ci.yml`, `crates/tabletist-db/src/adapter.rs`,
`agent.rs`, `agent/hidden/masked.rs`, `agent/run.rs`, `dialect.rs`,
`lib.rs`, `mysql.rs`, `pg.rs`, `sqlite.rs`, `stream.rs`,
`crates/tabletist-db/tests/export.rs`.

`Connection::stream_rows(query, columns, batch, to, stop)` sends the
columns, then batches of up to `batch` rows, read-only, and returns the
rows sent; `stop` or a receiver that is gone ends it as
`Error::Cancelled`, the server's own timeout as `Error::Timeout`.

- [ ] **Step 1: Write the tests.** `dialect::tests::an_export_names_its_columns_and_reads_every_row`
  (Test 1: the columns in their order, the filters bound, the sort then
  the key, no `LIMIT`); `tests/export.rs`: on each engine
  `the_books_stream_on_*` (batches of at most 5, the key's order, chosen
  columns, filter and sort), `the_export_session_cannot_write_on_*`
  (Test 3; on PostgreSQL also a `nextval` refused by the server's
  read-only transaction), `a_stop_ends_the_stream_on_*`;
  `two_hundred_thousand_rows_stream_a_batch_at_a_time` (Test 2, SQLite:
  40 batches of 5,000, never more than one read ahead) and
  `two_hundred_thousand_rows_stream_on_*` on the servers;
  `the_servers_timeout_is_told_apart_on_postgres`.
- [ ] **Step 2: Run them, and see them fail.**
  Run: `~/.cargo/bin/cargo test -p tabletist-db --test export`
  Expected: `error[E0599]: no method named `stream_rows` found for struct `Connection``.
- [ ] **Step 3: Write the code** (the files above).
- [ ] **Step 4: Run the checks**, and the server tests:
  `~/.cargo/bin/cargo test --locked -p tabletist-db --lib --test export`
  with both URLs. Expected: 13 export tests pass, none skipped.
- [ ] **Step 5: Commit.**
  `git add .github/workflows/ci.yml crates/tabletist-db && git commit -m "Stream a table's rows in batches on each engine, read-only"`

### Task 2, part one: run an export in the background

**Files:** `crates/tabletist-core/src/backend.rs`, `backend/export.rs`,
`export.rs`, `history.rs`; `src/app/history.rs`, `src/ui/history.rs`,
`src/ui/history_tests.rs`.

`backend::export::run(job, stopper, report)` opens a read-only connection
from the tab's details (none for loaded rows), reads SQL INSERT's `CREATE
TABLE` where asked, streams into `.<name>.<random>.tabletist-part` beside
the target, renames it, and tells each state; the worker keeps a
`Stopper` per job for `Command::CancelExport`. The history gains
`Source::Export`, which runs again in a SQL editor.

- [ ] **Step 1: Write the tests.** In `backend/export.rs`:
  `the_books_are_written_to_the_file_and_nothing_else_is_left`,
  `the_file_is_readable_by_its_owner_only` (Test 5),
  `an_existing_file_is_replaced_only_when_asked`, `a_full_disk_leaves_nothing`,
  `a_folder_that_may_not_be_written_is_named`, `a_cancel_leaves_nothing`,
  `a_lost_connection_leaves_nothing` (Test 4),
  `two_hundred_thousand_rows_are_written_whole` (Test 2),
  `loaded_rows_go_to_the_clipboard_without_a_connection` (Test 10),
  `masked_columns_follow_the_decision` (Test 15),
  `sql_insert_starts_with_the_create_table_when_asked`,
  `a_cancel_after_the_last_batch_keeps_the_file_out`,
  `the_clipboard_stops_at_twenty_megabytes`,
  `the_temporary_file_is_hidden_beside_the_target`.
- [ ] **Step 2: Run them, and see them fail.**
  Run: `~/.cargo/bin/cargo test -p tabletist-core --lib backend::export`
  Expected: `error[E0432]: unresolved import` of `backend::export`.
- [ ] **Step 3: Write the code.**
- [ ] **Step 4: Run the checks.** Expected: the fourteen job tests pass.
- [ ] **Step 5: No commit yet.** The app matches every backend event
  (`App::apply_event`), so `Event::Export` lands with part two, and the
  two parts are one commit.

### Task 2, part two: the sheet, the float, the states and the entry points

**Files:** `crates/tabletist-core/src/export_sheet.rs`, `keymap.rs`,
`lib.rs`, `model.rs`; `docs/_guide/browsing-data.md`, `macos.md`,
`omarchy.md`; `docs/_reference/keyboard-shortcuts.md`,
`settings-and-files.md`; `src/app.rs`, `src/app/commands.rs`,
`src/app/editing.rs`, `src/app/export.rs`, `src/app/menu.rs`,
`src/lib.rs`, `src/menu.rs`, `src/ui/data_view.rs`, `src/ui/export.rs`,
`src/ui/export_tests.rs`, `src/ui/export_toast.rs`, `src/ui/keys.rs`,
`src/ui/mod.rs`, `src/ui/sidebar.rs`, `src/ui/workspace.rs`;
`tests/environments.rs`, `tests/keys.rs`.

- [ ] **Step 1: Write the tests.** In `export_sheet.rs`:
  `the_format_and_the_extension_follow_each_other`,
  `chosen_columns_are_written_in_their_order` (Test 7),
  `a_typed_path_is_in_the_downloads_folder_unless_it_says_otherwise`,
  `a_full_disk_says_what_was_written`,
  `sizes_times_and_counts_read_as_the_boards_write_them`. In
  `src/ui/export_tests.rs`: `the_sheet_opens_over_the_table_with_its_rows_chosen`
  (Test 13), `the_save_panel_names_the_file_and_the_export_starts`
  (Test 8 and the remembered options), `chosen_and_moved_columns_are_written_in_their_order`,
  `selected_rows_go_to_the_clipboard_and_nothing_is_recorded` (Tests 6,
  10, 14), `the_clipboard_is_refused_over_ten_thousand_rows`,
  `unsaved_changes_are_said_and_not_exported` (Test 9),
  `the_toasts_follow_an_export_to_its_end` (Tests 11 and 14),
  `a_failure_stays_until_dismissed_and_try_again_sends_it_again`,
  `the_terminal_looks_lines_export` (Test 12),
  `the_float_writes_where_its_path_line_says`,
  `the_status_line_follows_the_export`,
  `the_file_menu_has_export_and_a_greyed_import`,
  `e_in_the_tree_opens_the_table_and_its_float` (Test 13),
  `w_alone_still_saves_and_with_a_path_never_writes` (Test 12),
  `count_exactly_sends_one_count` (Test 6),
  `rows_past_the_clipboards_limit_take_the_file_back`,
  `the_export_key_cancels_while_one_runs`. The keymap's
  generated tests cover the new keys (Test 16).
- [ ] **Step 2: Run them, and see them fail.**
  Run: `~/.cargo/bin/cargo test --lib ui::export_tests`
  Expected: `error[E0432]: unresolved import `crate::export_sheet``.
- [ ] **Step 3: Write the code**, and the website's pages.
- [ ] **Step 4: Run the checks**, `cargo doc` and `docs/bin/check`.
- [ ] **Step 5: Commit.**
  `git add -A crates src docs/_guide docs/_reference tests && git commit -m "Export a table in the background from its sheet or float, with its progress, its end and its keys"`

## After Task 2

Push the branch and open the pull request: the summary, the streaming
method per driver, the temporary file and the read-only session, the
keys added, the open questions with the defaults chosen (masking: real
values, `export::follows_masking`; saved data only; 0600 for every file;
10,000 rows and 20 MB; error toasts stay), D1–D14, Q1–Q20 and B1–B28.
Question 1 (masking) needs Igor's answer before merge. No screenshot goes
to GitHub unblurred.

## Checks by hand

On the Bookshop demo, in both looks: export `books` to CSV, JSON Lines
and SQL INSERT with CREATE TABLE; a filter and a sort; Choose and reorder
columns; to the clipboard; cancel a large export; a folder without write
permission; `:w ~/Downloads/books.md`, then `:w` on it again and `:w!`.
