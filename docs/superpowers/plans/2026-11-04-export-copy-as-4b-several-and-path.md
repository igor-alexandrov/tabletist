# Export and Copy as, run 4b: several exports at once, and the path line Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let exports run side by side: a toast each in the macOS look,
stacked at the bottom centre with a pill each when hidden, and in the
terminal look one status line for them all with a picker behind
`:export cancel`; Export… always opens the sheet, and quick open can
cancel each. In the terminal look's float, the save to line offers the
folder's names as it is typed, and `ctrl+o` asks the system's save dialog.

**Architecture:** `App::exporting: Option<Exporting>` becomes
`App::exports: Vec<Exporting>`, in the order they started. The actions
that act on one name it by its `ExportId` (`CancelExport(Option<ExportId>)`,
`HideExport { job, hidden }`, `DismissExport(_)`, `ExportFailureAction(_)`,
`RevealExport(_)`), `None` meaning the one the terminal look's line shows
(`App::shown_export`). `src/ui/export_toast.rs` draws the stack and the
pills; `src/ui/toast.rs` keeps any number of buttons and walks them all;
`workspace::export_line` writes the line for one, for several and for an
end; `Dialog::CancelExport` is the picker. The path line asks
`Command::ListFolder` whenever it is typed in; the backend reads the
folder off the UI thread and answers `Event::FolderListed` with what
`export_sheet::completions` keeps; the sheet holds the list for the line
it was asked for (`ExportSheet::listed`). `ctrl+o` asks
`Backend::ask_export_path` with `ExportSheet::picking`, whose answer
fills the line.

**Tech Stack:** Rust, egui (crmne fork) with AccessKit, tokio; headless
tests through `src/testing.rs`.

**Spec:** `docs/superpowers/specs/2026-11-04-export-copy-as-4-result-design.md`,
sections "Several exports at once (4b)" and "The path line (4b)", rows
F5–F8 and S12–S16, its Keys and its tests 6–9 (approved as written on
2026-11-04).

---

## Where this sits

Run 4 is two pull requests (Igor, 2026-11-04): 4a (PR #274, merged) and
**4b, this plan**. Base: `main` at `6b25c210`.

## Decided while building

Not in the spec, or settled by the code. To put to Igor in the pull
request:

| # | The spec | Built | Why, and the other way |
|---|---|---|---|
| B1 | S14: an end takes the line, "the newest first" | The one that ended last | What just happened is what the line says. The other way: the one that started last |
| B2 | F5: `:export cancel`'s picker | Drawn as the Copy as menu is, at the status line's right end, over it: `cancel export` and how many, a row each `1  reviews.csv 58%`, its foot `one key picks · ctrl+n/p enter · esc close`; digits 1 to 9 pick | No board draws it; this is the look's one picker of a few rows |
| B3 | F7: "Cancel exporting reviews.csv" | A clipboard export's row is "Cancel copying reviews"; the rows are listed for a word typed (or `>`), as the commands not suggested are | The toast's own words. The other way: listed with nothing typed |
| B4 | The board's second toast (no total) says "Bookshop · production" under its bar | Under the bar: the time left where the total is known (once 2 s have gone, as 3a), else the connection | As the board draws it |
| B5 | S13: ⇥ walks every toast's buttons | Top to bottom, then left to right, over the export toasts, the pills and the copy offer's toast | The keyboard's order is the eye's |
| B6 | S13: ⌘⇧' to the newest toast | The newest export's first button, its toast's or its pill's; with no export, the copy offer's | 3b's rule for the offer stays |
| B7 | F8: the pills | 8 apart, as the toasts | As S13 |
| B8 | S15: the list closes "on a pick" | A file picked closes it; a folder picked lists that folder's names | A file picked would list itself again |
| B9 | S15: the list is the path line's | It shows while the path line has the keyboard; Tab to another row hides it, and coming back types nothing new, so it waits for the next key | The list is the line's |
| B10 | `Command::ListFolder { asked, typed }` | `{ asked, typed, folder, start }`: the app resolves `~` and the downloads folder (no disk), the backend reads `folder` and keeps the names starting with `start`; `Event::FolderListed { asked, typed, names }` | The app knows the home and downloads folders; the disk is the backend's |
| B11 | S16: `ctrl+o pick file` in the foot | Also `ctrl+o pick` at the line's right end, as the component board and the jsonl float draw it | Both boards |
| B12 | S16: the answer "fills the line" | As `~/…` where under the home folder, its extension setting the format; a file the dialog asked to replace shows the line's `! exists · enter again replaces it`. A closed dialog leaves the line | The line's own way of saying a replace |
| B13 | S12: no cap | No cap on the toasts either: a stack taller than the window goes off its top | Rare; the pills stay |
| B14 | F7: ⇧⌘E always opens the sheet | 3a's "An export is running" refusal goes everywhere (`:w <path>`, Export selection…, the tree's `E`) | One more export starts from wherever |
| B15 | S14: `2 exports · …` | A clipboard copy is named by its table there, as on its own line | As 3a |
| B16 | (a failure answered) | An export to a file puts away a failed export to the same file: `:w!` after "exists" answers it | 3a's one export was replaced by the next. The other way: the failure stays until Esc |
| B17 | F5: the picker | Lists those of its exports still running, and closes once none is; with one left it still asks | An export may end while it is open |
| B18 | F7: quick open's cancel row | Drawn with the × icon, "export running" after its name | As a command's row |
| B19 | S15: the list | A line changed puts its list away until the answer for the new line comes | Enter never takes a name listed for another line |

## Files

| File | Task | What |
|---|---|---|
| `crates/tabletist-core/src/model.rs` | 1, 2 | The actions with an id; `Dialog::CancelExport`; `QuickCommand::CancelExport`; `ExportPickFile`, the path list's actions |
| `crates/tabletist-core/src/keymap.rs` | 1, 2 | `CancelWhichExport` keys, `ExportPickFile`, `PathNext`, `PathPrevious`, `PathPick`, `ClosePathList`; `When::CancelExports`, `When::PathListed` |
| `crates/tabletist-core/src/export_sheet.rs` | 1, 2 | `Exporting::name`, `ended_at`; `ExportSheet::{listed, picking}`; `completions`, `completion_ask`, `complete` |
| `crates/tabletist-core/src/backend.rs` | 2 | `Command::ListFolder`, `Event::FolderListed` |
| `src/app.rs`, `src/app/export.rs` | 1, 2 | `exports`, every reader by id; the picker; the list's ask and answer; `ctrl+o` |
| `src/app/commands.rs`, `menu.rs`, `quick_open.rs`, `editing.rs` | 1 | Export… always opens; the cancel rows; `:export cancel` |
| `src/ui/export_toast.rs`, `toast.rs`, `mod.rs`, `keys.rs`, `workspace.rs`, `quick_open.rs` | 1 | The stack, the pills, the keyboard, the line, the picker, the rows |
| `src/ui/export.rs` | 2 | The list under the line, its keys, `ctrl+o` |
| `src/ui/export_several_tests.rs`, `export_path_tests.rs`, and 3a/3b/4a's tests | 1, 2 | The new tests; the old ones read `exports` |
| `docs/_guide/*.md`, `docs/_reference/keyboard-shortcuts.md` | 1, 2 | Several at once; the path line |

Every commit ends on `~/.cargo/bin/cargo fmt --all --check`,
`~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings`
and `~/.cargo/bin/cargo test --locked --workspace --all-targets`; task 2
also on `RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --locked
--workspace --no-deps` and `cd docs && bin/check`, and the server tests
with `TABLETIST_TEST_PG_URL` and `TABLETIST_TEST_MYSQL_URL`.

---

### Task 1: Several exports at once

**Files:** as the table's task 1 rows.

- [ ] **Step 1: Write the tests.** `src/ui/export_several_tests.rs` (spec
  tests 6 and 7): `two_exports_run_and_cancel_alone` (both sent, Cancel on
  the first toast cancels it alone, each ends with its own toast, a done
  goes after 4 s while a failure stays), `a_hidden_export_is_a_pill_each`,
  `the_keyboard_walks_every_toast` (⌘⇧' to the newest's Cancel, ⇥ round
  both), `export_always_opens_the_sheet` (⇧⌘E and File › Export… while one
  runs), `quick_open_cancels_each_export`,
  `the_terminal_line_says_one_two_and_an_end`,
  `export_cancel_picks_among_two` (one cancels at once; two open the
  picker; a digit, ctrl+n and Enter, esc), `go_esc_and_export_act_on_the_line_shown`.
- [ ] **Step 2: Run them, and see them fail.**
  Run: `~/.cargo/bin/cargo test --lib export_several`
  Expected: `error[E0609]: no field `exports` on type `App``.
- [ ] **Step 3: Write the code**, and the guides' words.
- [ ] **Step 4: Run the checks.**
- [ ] **Step 5: Commit.**
  `git commit -m "Run several exports at once, a toast or a pill each, one status line for them all"`

### Task 2: The path line's completions and ctrl+o

**Files:** as the table's task 2 rows.

- [ ] **Step 1: Write the tests.** In `export_sheet.rs`:
  `completions_keep_folders_first_and_eight`, `a_completion_fills_the_line`;
  in `backend.rs`: `a_folder_is_listed_off_the_ui_thread` (a real folder in
  a temp dir under `target/`); `src/ui/export_path_tests.rs` (spec tests 8
  and 9): `the_path_line_lists_what_it_starts`,
  `an_answer_for_an_old_line_is_dropped`, `ctrl_n_and_enter_take_a_name`,
  `a_folder_taken_lists_its_own`, `esc_closes_the_list_then_the_float`,
  `ctrl_o_fills_the_line_and_starts_nothing`.
- [ ] **Step 2: Run them, and see them fail.**
  Run: `~/.cargo/bin/cargo test --lib export_path`
  Expected: `error[E0599]: no variant named `FolderListed` found for enum `Event``.
- [ ] **Step 3: Write the code**, and the guides' words.
- [ ] **Step 4: Run the checks**, `cargo doc`, `docs/bin/check`, the server tests.
- [ ] **Step 5: Commit.**
  `git commit -m "Offer the save to line's folder names as it is typed, and ctrl+o for the system's dialog"`

## After Task 2

Push the branch and open the pull request: the summary, F5–F8, S12–S16,
B1–B19. No screenshot goes to GitHub unblurred.

## Checks by hand

On the Bookshop demo, in both looks: export `books` to CSV and, while it
runs, `authors` to JSON Lines; hide one to its pill; cancel the other;
`:export cancel` with both running; type `~/Dow` in the save to line and
take `Downloads/`; `ctrl+o` and pick a name.
