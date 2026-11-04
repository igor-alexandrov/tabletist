# Export and Copy as, run 3b: the table's other ways in Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Reach run 3a's export from the sidebar's new context menu, quick
open (its rows, and ⇧⌘E / `ctrl+e` on a picked table), the grid's
right-click menu (Export selection…) and Copy as's over-the-limit toast;
tell a finished export with a system notice while the window is behind;
and let ⌘⇧' take the keyboard to the export's toast.

**Architecture:** Every way in ends in 3a's reducers: `App::open_export`
(the table in front) or a new `App::export_object(tab, object, kind)`
(open the table as Enter does, then its sheet; 3a's `export_from_tree`
becomes a caller of it). `Action::ExportSelection(ExportOffer)` opens the
sheet on the selection: `ExportSheet::cursor_row` takes the cursor's row
where no range stands, and `export_sheet::block_columns` ticks a block's
columns under Choose (R1). The sidebar's menu is `menu::context` on a
tree row, its rows `Action::FromTree { pick: TreePick }`. Quick open's
rows are `RUNNABLE` commands (`Export`, named "Export {table}…" through
`About::CurrentTable` and `Runnable::named`, and a new `ExportTable`), its
picker `QuickOpen::exporting`, its key `QuickOpenExport`. The
over-the-limit offer is the one-line toast with one button
(`toast::offer`), which the terminal look's `E` reads too
(`toast::offered`). The notice is `Command::Notify` (D-Bus on Linux,
`plain` beside the agents' notice) plus the window's attention request on
macOS and Windows. The toast's keyboard is egui focus on its buttons
(`toast::button`), read by `toast::keys` before the window's keys:
`FocusToast`, `LeaveToast`, `ToastNext`, `ToastPrevious`.

**Tech Stack:** Rust, egui (crmne fork) with AccessKit, zbus; headless tests
through `src/testing.rs`.

**Spec:** `docs/superpowers/specs/2026-11-04-export-copy-as-3b-entry-points-design.md`
(E1–E6 and R1–R9, approved as written on 2026-11-04), over run 3's
`docs/superpowers/specs/2026-11-03-export-copy-as-3-table-design.md`.

---

## Before Task 1

```bash
git add docs/superpowers/specs/2026-11-04-export-copy-as-3b-entry-points-design.md docs/superpowers/plans/2026-11-04-export-copy-as-3b-entry-points.md
git commit -m "Plan run 3b of Export and Copy as: the table's other ways in"
```

## Where this sits

Four runs, a pull request each (Igor, 2026-10-30); run 3 in two (Igor,
2026-11-03): 3a (#272, merged) and **3b, this plan**. Base: `main` at
`74821060`. Run 4 exports a SQL result.

## Decided while building

To put to Igor in the pull request (filled in as the code settles them):

| # | The spec | Built | Why, and the other way |
|---|---|---|---|
| B1 | R1: a block opens Choose | Only a block that leaves a column out; one over every column opens All columns | The same columns either way. The other way: Choose always |
| B2 | R1: Export selection…, ⇧⌘E and `E` | Every way into the sheet of the table in front (the header's Export…, File › Export…, quick open's row, the tree on a table already open) starts a block on its columns | One sheet, one start. The other way: only those three |
| B3 | R1: the cursor's row | Only for Export selection… and the offer; ⇧⌘E, `E` and the header with no range keep 3a's All rows matching. On new rows alone Export selection… opens no sheet (B20) | 3a's start stays where nothing was asked for the selection |
| B4 | R6 | A refused copy keeps the terminal look's visual selection, for `E`; a copy made still leaves it | `E` exports what is selected |
| B5 | R6 | The terminal look's line is the copy's words and ` · E export`, the key from the keymap; `E` opens the float on the copy's format, to a file, while the line says it | As the toast's Export… |
| B6 | R6 | Copy rows as TSV's offer exports every column; the cursor's row alone (`yy`, the row panel's Copy row as) offers nothing | The row panel's Export row… is run 4's |
| B7 | R6 | The offer lasts 4 s, waits under the pointer or with the keyboard, goes once clicked; it floats over the window as the export's toast does, and a click under a dialog does nothing | A toast that takes a click |
| B8 | R6 | Export… opens over the table copied, which comes to the front if another is there | The copy's table |
| B9 | R7 | The macOS action reads "Export…"; the Omarchy lines read `✗ that's more than 10,000 rows · :export to a file` and `✗ that's over 20 mb · :export to a file`; the sheet comes back with its format, rows and columns | `:export` does what its line offers (3a's B13) |
| B10 | R5 | A right-click on any tree row puts the tree's cursor there; a rule stands before Export…; Import… says "Import comes after Export" under the pointer; Open and Export… name ↩ and ⇧⌘E in the macOS look; Copy name says "Copied ratings" | As a click does |
| B11 | E3 | The grid's menu has a rule before Export selection…, with ⇧⌘E in the macOS look. A rule is new to the menus: `menu::separator` | The board's |
| B12 | R2 | "Export {table}…" is a command's name: typing the table's name finds it too | Its name holds the table |
| B13 | (the menu bar) | **File › Export a Table…**, after Export…, greyed while an export runs | Every quick open command is one menu item (the menu's tests) |
| B14 | R3 | With nothing typed the picker lists the start list's tables and views; Tab keeps this connection; ⌥↩ exports as ↩ does; a "more" row shows the rest; its footer says `↩ export` and names no commands, scope or SQL editor; its placeholder shows in the terminal look too | A picker of this connection's tables |
| B15 | R4 | While an export runs the key says "An export is running…" and quick open stays | Run 3's D5 |
| B16 | (the spec's Keys) | `ExportTable` (no key; one given in Settings opens the picker), `ToastNext` (Tab) and `ToastPrevious` (⇧Tab), and `When::ToastFocused` | egui's Tab would leave the toast; every key is the keymap's |
| B17 | R9 | ⇥ goes round the buttons of the toast that has the keyboard only; with the export's toast and the offer both up, ⌘⇧' goes to the export's; a ⌘⇧' whose toast went in that frame asks nothing later; Esc with nothing to give back takes the keyboard off the toast; when the progress toast becomes the done toast the keyboard is not moved | One toast at a time |
| B18 | E5 | The notice's app name and summary are "Tabletist", its body "Exported 31,400 rows to reviews.csv"; no action and no setting; `:w` exports are told; told once per end | The board's words |
| B19 | E4 | The pill is a widget of its own over its frame | A frame's response cannot keep the keyboard |
| B20 | R1 | Export selection… on new rows alone opens no sheet and says "New rows aren't exported. Export reads what is saved in the database. Save first ⌘S" (Omarchy: "new rows aren't exported · :w first to include them") | A sheet on All matching rows, or on Selected rows with none |
| B21 | R6 | The offer keeps the cursor and the range the copy took; Export… (or Omarchy `E`) puts them back on the grid if the selection moved since | The selection as it is when Export… is clicked |
| B22 | R3 | Where every recent row is a SQL editor, Export a table… lists the largest tables under their heading, as the start screen does with nothing recent; with no match it offers no other connection to search | An empty list |
| B23 | R6 | ⌘C, ctrl+c and `y` over the limit offer Export… as Copy as's Copy does (TSV), and `y` keeps the visual selection for `E` | The words alone |
| B24 | R2 | Export a Table… takes a key of the person's in either look (Keyboard settings and `settings.toml`) | No key at all |

## Files

| File | Task | What |
|---|---|---|
| `crates/tabletist-core/src/export_sheet.rs` | 1, 3 | `ExportSheet::cursor_row`, `block_columns`; the clipboard failure's words |
| `crates/tabletist-core/src/model.rs` | 1, 2 | `ExportOffer`, `Action::ExportSelection`, `Action::FromTree`, `TreePick`; `QuickOpen::exporting`, `About::CurrentTable`, `About::PickNext`, `Runnable::named`, `Action::QuickOpenExport`, `Action::PickTableToExport` |
| `src/app/export.rs` | 1, 3 | `export_object`, `tree_pick`, the selection's start; `offer_export`, the failure's Export…, the notice |
| `src/ui/sidebar.rs`, `src/app.rs` | 1, 2, 3 | The tree row's menu; the reducers; the offer, the window's focus, the toast's keys |
| `src/ui/copy_as.rs`, `src/ui/data_view.rs`, `src/ui/sql_results.rs`, `src/ui/menu.rs` | 1 | Export selection… in a table's grid menu, and the menus' rule |
| `crates/tabletist-core/src/keymap.rs` | 2, 3 | `ExportTable`, `QuickOpenExport`; `FocusToast`, `LeaveToast`, `ToastNext`, `ToastPrevious`, `When::ToastFocused` |
| `src/app/commands.rs`, `src/app/quick_open.rs`, `src/ui/quick_open.rs`, `src/ui/quick_open_tests.rs`, `src/menu.rs` | 2 | The rows, the picker, the key and its hint, File › Export a Table… |
| `src/ui/toast.rs`, `src/ui/export_toast.rs`, `src/ui/keys.rs`, `src/ui/mod.rs` | 3 | The offer and `E` on it, the toast's keyboard |
| `crates/tabletist-core/src/backend.rs`, `backend/agents.rs`, `backend/agents/notify.rs` | 3 | `Command::Notify`, and the notice with no action beside the agents' |
| `src/ui/export_ways_tests.rs`, `src/ui/copy_as_tests.rs`, `tests/keys.rs` | 1, 2, 3 | The headless tests; the keys check lists the new test file |
| `docs/_guide/*.md`, `docs/_reference/keyboard-shortcuts.md` | 1, 2, 3 | The pages |

Every commit ends on `~/.cargo/bin/cargo fmt --all --check`,
`~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings`,
`~/.cargo/bin/cargo test --locked --workspace --all-targets` and
`RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --locked --workspace --no-deps`,
with `CARGO_TARGET_DIR` the worktree's `target/`. No server test changes.

---

### Task 1: The sidebar's menu and the grid's Export selection…

**Files:** `crates/tabletist-core/src/export_sheet.rs`, `model.rs`;
`src/app.rs`, `src/app/export.rs`, `src/ui/sidebar.rs`,
`src/ui/copy_as.rs`, `src/ui/data_view.rs`, `src/ui/sql_results.rs`,
`src/ui/menu.rs`, `src/ui/mod.rs`, `src/ui/export_ways_tests.rs`;
`docs/_guide/browsing-data.md`, `macos.md`, `omarchy.md`.

- [ ] **Step 1: Write the tests** (`src/ui/export_ways_tests.rs`):
  `the_sidebars_menu_holds_the_tables_ways` (spec test 1: six rows,
  Import… greyed, none on a schema), `the_sidebars_menu_rows_do_what_they_say`
  (Open, Structure, DDL, Copy name, Export…),
  `export_selection_opens_the_sheet_on_the_selection` (test 4: rows),
  `a_block_opens_choose_with_its_columns` (R1),
  `a_single_cell_exports_its_row`,
  `a_results_grid_menu_has_no_export`, `a_rows_menu_is_a_tables_or_a_views`,
  `the_headers_export_on_a_block_chooses_its_columns`; in `export_sheet.rs`
  `a_blocks_columns_come_first_and_the_rest_are_left_out`.
- [ ] **Step 2: Run them, and see them fail.**
  Run: `~/.cargo/bin/cargo test --lib ui::export_ways_tests`
  Expected: the menu rows are not found (`no widget labelled "Export…"`).
- [ ] **Step 3: Write the code.**
- [ ] **Step 4: Run the checks.**
- [ ] **Step 5: Commit.** "Export a table from the sidebar's new menu and the grid's selection from its menu"

### Task 2: Quick open's export rows and ⇧⌘E on a picked table

**Files:** `crates/tabletist-core/src/keymap.rs`, `model.rs`;
`src/app.rs`, `src/app/commands.rs`, `src/app/quick_open.rs`,
`src/ui/quick_open.rs`, `src/ui/quick_open_tests.rs`, `src/ui/keys.rs`,
`src/menu.rs`, `src/ui/export_ways_tests.rs`, `tests/keys.rs`;
`docs/_guide/macos.md`, `omarchy.md`, `docs/_reference/keyboard-shortcuts.md`.

- [ ] **Step 1: Write the tests:** `quick_open_lists_the_export_rows`
  (test 2), `export_a_table_lists_tables_and_opens_the_picked_ones_sheet`,
  `the_export_rows_give_way_to_cancel_while_one_runs`,
  `the_export_key_on_a_picked_table_opens_its_sheet` (test 3, both looks),
  `the_export_key_on_a_command_does_nothing`,
  `the_footer_names_the_export_key_on_a_table_only`,
  `the_picker_keeps_its_rows_and_its_connection`,
  `the_export_key_in_quick_open_waits_for_a_running_export`,
  `the_file_menus_export_a_table_opens_the_picker`. The keymap's
  generated tests cover the new keys.
- [ ] **Step 2: Run them, and see them fail.** Expected: no row
  "Export book_covers…".
- [ ] **Step 3: Write the code.**
- [ ] **Step 4: Run the checks.**
- [ ] **Step 5: Commit.** "Export from quick open: the table in front, a table picked next, or the one picked with ⇧⌘E"

### Task 3: Export… over the clipboard's limit, the notice, and ⌘⇧'

**Files:** `crates/tabletist-core/src/backend.rs`, `backend/agents.rs`,
`backend/agents/notify.rs`, `export_sheet.rs`, `keymap.rs`, `model.rs`;
`src/app.rs`, `src/app/export.rs`, `src/ui/toast.rs`,
`src/ui/export_toast.rs`, `src/ui/keys.rs`, `src/ui/mod.rs`,
`src/ui/copy_as.rs`, `src/ui/export_ways_tests.rs`, `src/ui/copy_as_tests.rs`;
`docs/_guide/browsing-data.md`, `macos.md`, `omarchy.md`,
`docs/_reference/keyboard-shortcuts.md`.

- [ ] **Step 1: Write the tests:** `copy_over_the_limit_offers_export`
  (test 5), `the_terminal_lines_offer_is_e`,
  `the_offer_stays_while_the_pointer_or_the_keyboard_is_on_it`,
  `the_offer_follows_what_was_copied`, `the_offer_opens_over_the_table_copied`,
  `the_clipboards_failure_offers_the_file` (test 6, both looks),
  `the_clipboards_failure_keeps_the_sheets_choices`,
  `a_finished_export_behind_other_windows_is_told` (test 7),
  `no_notice_while_in_front_or_for_a_failure_or_a_copy`,
  `a_done_said_twice_is_told_once`,
  `the_toast_key_moves_the_keyboard_to_the_toast` (test 8),
  `the_toast_key_goes_to_each_toasts_first_button`,
  `the_toast_key_asks_once_and_tab_keeps_to_its_toast`,
  `the_toast_key_with_no_toast_does_nothing`; and in `copy_as_tests.rs`
  the terminal line's words.
- [ ] **Step 2: Run them, and see them fail.**
- [ ] **Step 3: Write the code.**
- [ ] **Step 4: Run the checks.**
- [ ] **Step 5: Commit.** "Offer the file over the clipboard's limit, tell a finished export behind other windows, and reach the toast with ⌘⇧'"

## After Task 3

Push the branch and open the pull request: what each way in does, the
keys added, E1–E6 and R1–R9 as approved, and B1–B24 from the table
above. Checked by hand only: ⌘⇧' as macOS and Windows send it (the tests
send the bare key).
No screenshot goes to GitHub unblurred.

## Checks by hand

On the Bookshop demo, in both looks: right-click `books` in the sidebar
and pick each row; quick open `> exp`, both rows; quick open `boo` then
⇧⌘E (`ctrl+e`); right-click a block of cells, Export selection…;
select a large `books` page past 20 MB of values and Copy as, then
Export…; start an export and switch to another window until it ends;
⌘⇧' during an export, then ⇥ and Esc.
