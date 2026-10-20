# Quick open, run 2: commands, `>` and the settings' rows. Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Quick open runs the app's commands (macOS and Windows looks). A registry lists each command of the keymap that quick open can run, with the name it is listed by, what its row says of where it acts, its key, and whether it applies now: one that would do nothing now (Close tab with no tab, Save changes with nothing pending, Next connection with one window) is not listed. Typed without a prefix, the commands and the settings that match come after the tables and views, folded after three; a query that begins with `>` lists the commands alone. Each row of the Settings sheet is a result too (`Settings › General › Restore tabs and filters`): it opens Settings on that pane, with that row on the selection's ground, scrolled into view and its control given the keyboard. The footer names `>`. On macOS the View menu holds Quick Open beside History. The Omarchy look is unchanged: its commands stay on `:`.

**Architecture:** `src/app/commands.rs` is the registry: `RUNNABLE` (the keymap's commands quick open runs, in order), `App::runnable` (those that apply now, each a `Runnable` with its name and `About`) and `App::command_actions` (what running one pushes: what its key pushes in `src/ui/keys.rs`), from one function so that what is listed and what runs cannot part. `src/app/quick_open.rs` lists them (`QuickCommand::Run`, `QuickCommand::Setting`), reads the `>`, folds the Commands group (`QuickGroup::Commands`) and runs a pick. `Action::ShowSetting` opens Settings at an option (`SettingsDialog::asked`, `reveal`), and the sheet (`src/ui/settings/sheet.rs`) grounds, scrolls to and focuses that row. `src/macos.rs` puts Quick Open in the View menu through `tabletist_appkit::ViewItem`, as View › History is, and `src/entrypoint.rs` turns a choice into `Action::OpenQuickOpen`.

**Tech Stack:** Rust 2024, egui 0.36 (the repository's fork) with AccessKit. No new dependency.

---

## Where this sits

Run 2 of four of `2026-10-19-quick-open.md`, the feature's master plan, which Igor approved with its departures on 2026-10-19. Run 1 (`2026-10-19-quick-open-1-panel.md`, igor-alexandrov/tabletist#186) built the panel, the index and the scopes. The plan was written on run 1's branch at `06c9cb0` while #186 was open, approved, and then moved onto `main` at `b3a86b29` (#186 merged on 2026-10-20), where its diffs apply. Meanwhile the query history had landed on `main` (#196, #197, #198) with what this run meets: a History command among quick open's (`> History`), a `>` of its own, View › History through a new `tabletist_appkit::ViewItem`, and a History section in Settings › Editor. This plan takes History into the registry, replaces that `>` with the keymap's, adds Quick Open to the same View menu, and lists the History section's row. The boards were read again for this plan on 2026-10-20: `macOS – Quick open ⌘P, states` (its Commands state: `> re`, the commands and a setting's row, and its footer's `> commands`), `macOS – Quick open ⌘P`, `macOS – Settings, General` and `Omarchy – Find object ctrl+p, states` ("objects only: commands stay on :"). Columns (run 3) and functions (run 4) are later runs, each shippable on its own.

## Where this departs from the boards or the brief

Each row needs Igor's yes before the plan is run. The master plan's rows hold here too and are not repeated (among them: Reopen closed tabs is not listed until it is built, and Switch database has no macOS key).

| What the board or the brief says | What this plan builds | Why |
|---|---|---|
| Commands state: "Run in read-write transaction · this tab" | "Read-write tab" (or "Read-only tab") · `Query 2`, with ⇧⌘M, where the SQL editor in front can run writes | Transactions are not built: Commit and Rollback are claimed in the keymap and answer nothing. What is built is the editor's runs' mode, which ⇧⌘M toggles. A tab's commands name the tab (`book_covers`, `Query 2`), as the board's Reset filters row names `book_covers` |
| Commands state: "Reset filters and sort · book_covers" | Not listed. "Filter rows" (⌘F, the filter bar) is | The registry is the keymap's commands (the master plan). Clearing filters and a sort are controls with no key, no command of the keymap's |
| Commands state: "Reload schema · Bookshop · dev · ⌘R" | "Reload schema · Bookshop · dev", which reloads the schema (the sidebar's lists), with no key written | ⌘R reloads the schema only with the keyboard in the sidebar: on a table's rows it reloads the rows, and in a SQL editor it does nothing. A row writes a key that does what the row does (a review finding: the draft wrote ⌘R) |
| Brief: "one row per setting" | One row per row the macOS and Windows sheet draws (29): the data font's size, which stands beside the font in the Data font row, and the Editor pane's options that are not built yet are not listed | A row opens Settings at a row of the sheet; an option with no row of its own has none to open at |
| Board: Settings rows "jump to the setting" | The row is on the selection's ground while its pane shows, is scrolled to the middle of the rows, and its first control takes the keyboard, once | The boards draw no row of the sheet picked out. The selection's ground is the one the sheet's nav and quick open's pick use. A control with the keyboard is what Space and the arrows act on |
| Master plan: "The `>` prefix on macOS" | macOS and Windows | The Windows look draws the same panel (run 1); Omarchy's commands stay on `:` |
| Board's typing state: no Commands group | Typed without `>`, the commands and settings that match follow the tables and views, three and "N more commands" (run 1 listed its three commands unfolded) | The master plan's "Commands group while typing extended to that registry"; folded as each group is, so that a word that matches a dozen settings does not bury the objects |
| A command's key beside it (the boards) | Edit connection and New connection write none: ⌘E and ⌘N are the Connections picker's keys, which a connection's window does not read | A row writes a key that does what it does where quick open was opened (found by eye in a render of the draft) |
| Brief: View › Quick Open (and, in the plan Igor approved, with ⌘P beside it) | Quick Open in the View menu, after History, with no key equivalent: ⌘P is not written beside it. In every look a Mac draws (the Omarchy look's opens its picker) | Main's `ViewItem` (the query history's, which landed while this plan waited) makes the View menu and gives its items no key equivalent by design: AppKit takes a key equivalent before the window sees it, wherever the keyboard is. ⌘P stays the window's key, which while quick open is open selects what is typed. Changed after the plan's approval, when it was moved onto main |

## What this run settles

| Question | Decision | Why |
|---|---|---|
| The registry | `RUNNABLE`, 29 commands of the keymap, all built, each once (a test): New SQL query, Switch database, Agents, History, Reload schema, Edit connection, Close connection, Next and Previous connection, Connections, New connection, Hide or Show sidebar, Settings, Keyboard shortcuts, Larger, Smaller and Default text size, Cancel running query (while the window loads or runs); and of the tab in front, Close tab, Next and Previous tab, Filter rows, Hide or Show row panel, Review pending SQL, Save changes, Discard all pending, Run all, Format SQL, Read-write or Read-only tab. Each does what its key does in the window, but three: Reload schema reloads the schema, which ⌘R does only with the keyboard in the sidebar; Review pending SQL shows the review, which its key shows and hides by turns; and Agents shows the Agents tab, which its key does but where History has an agent's statement picked (there it shows that call) | The commands that act on the window or the tab in front. Left out: what acts where the keyboard is (a cursor, a cell, a selection, a field, a list: the grid's and the editor's keys, Run statement at the caret, the tree's moves); the Connections picker's own; the start screen's and quick open's own; the Agents tab's own (its lists); prompts' and dialogs' keys; the digit commands (Next and Previous cover them); the keys that move the keyboard (F6, Omarchy's ctrl+h/l); Omarchy-only keys (`/`, `t`); and what is not built. Previous and Next page could follow: whether there is a page to go to is the pager's |
| The pick | `QuickOpen::picked` holds the result the pick was put on (by opening, a move, what was typed, the scope, a "more" row), and `App::quick_open_picked` finds it in the list as it is now: Enter opens that result, wherever the list moved it, and nothing where it is gone (quick open stays); the panel shows it picked. A click and a digit name a row of the list drawn that frame, as before. A move starts from the pick as it shows; from a result that is gone, up goes to the result before it and down to the one in its place | The commands list changes by itself (a run ends and Cancel goes, a list of databases comes and Switch database is listed): with an index, Enter ran the command that slid into the pick's place, Discard all pending among them (a review finding) |
| Applies now | One function per command (`App::run_of`) gives its row and its actions, or nothing where it would do nothing: Switch database with one database; Agents where agents cannot reach the app; Edit connection for a connection not saved; Next and Previous connection with one window; Default text size where the size is the look's own, Larger and Smaller at the ends of the steps; Close tab with none; Next and Previous tab with one; Filter rows but on a table's Data view; Hide or Show row panel with no row to show; the pending changes' three with none pending or a save under way (Review also while it shows); Run all with no statement (comments alone) or a write in flight; Format SQL with an empty editor; the run mode where the connection allows no writes; Cancel with nothing running | Listed and run from the same answer, so a command that lists does what its key does, and one that would not is hidden (the brief) |
| Names | As a menu says them: "New SQL query", "Reload schema", "Keyboard shortcuts", "Hide sidebar" or "Show sidebar", "Filter rows", "Read-write tab" or "Read-only tab" | The boards' names where they draw one; the keymap's are its list's ("New SQL editor tab", "Toggle sidebar") |
| What a row says after its name (`About`) | The connection (`Bookshop · dev`) for the window's commands that act on it; the tab in front (`book_covers`, `Query 2`, `Agents`) for its commands; "opens read-only" and the Agents tab's who is connected as before | The board's |
| The key a row writes | The command's first key where a connection's window reads it: the global scope's, then the grid's, the sidebar's, the SQL editor's; none for Reload schema. A setting's row writes Settings' (⌘,). A screen reader hears "command" or "setting" after the name | A key that does something else, or nothing, where quick open was opened would mislead |
| With nothing typed | The boards' three and History, where they apply, as main lists them | The master plan's open question 1, and the query history's History |
| `>` | The character quick open's `QuickOpenCommands` is spelled as in the keymap, read from it (`App::quick_open_commands_query`), first in the query after spaces; the Omarchy look has none, and types it (main's `>` was a `'>'` of its own, read in every look). What follows it (trimmed) is matched against the commands' names in the tiers of `search::find`, best first, in the registry's order within a tier, then the settings' rows; no table or view is searched, in either scope. `>` alone lists every command that applies, and no setting. Never folded. Nothing matches: "No commands match “x”" | The board's "Start with > for commands only"; the settings' rows answer a word, and would bury the commands under `>` alone |
| A setting's row | `QuickCommand::Setting(option)`, named `Settings › {pane} › {label}` as the sheet writes them (`{run}` the run key), with "jumps to the setting" and ⌘,. What is typed matches that whole path, so `general` lists the General pane's rows and `> se` every setting (unfolded, after the commands that match better). Picking it applies `ShowSetting(option)`: Settings opens (or stays open) on the option's pane, `asked` is the option, `reveal` is set, and the terminal look's cursor goes on its row where that screen has one (it has none for some of the sheet's: Interface size, Current line highlight). The sheet grounds the row while `asked` holds; once the sheet is the dialog on top, it scrolls the row to the middle and gives its first focusable control the keyboard, and pushes `SettingShown`. Another pane clears `asked` | The board's row; the view asks for nothing it may not: the one-shot is cleared by an action |
| The sheet's sections | `SettingsPane::sections` holds the headings and options the sheet drew inline; `sheet_options` flattens them (the Agents pane: its switch and its Log rows) | One list for what the sheet draws and what quick open lists |
| The footer | `> commands` after `⌥↩ in SQL editor` (macOS, Windows), from a new keymap line: `QuickOpenCommands`, `>` in `When::QuickOpen`, Omarchy none | Keys are spelled in the keymap alone; quick open reads the `>` from its field's text, and the line makes the keymap own its spelling and the Keyboard page list it |
| View › Quick Open | `QuickOpenMenu` in `src/macos.rs`, as `HistoryMenu` is: `tabletist_appkit::ViewItem::insert("View", "Quick Open", chosen)`, after History. Choosing it pushes `Action::OpenQuickOpen` | View › History's way; with quick open open, `OpenQuickOpen` selects its text, as ⌘P does |
| The website | `_guide/macos.md`, `_guide/browsing-data.md`, `_reference/keyboard-shortcuts.md` | AGENTS.md |

## Before you start

- One commit for each task, in the order below, after one for this plan. Every commit passes `cargo fmt --all --check`, `cargo clippy --locked --workspace --all-targets -- -D warnings`, `cargo test --locked --workspace --all-targets` and `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps`.
- On Igor's machine `cargo` is `~/.cargo/bin/cargo` (the mise shim fails), and a build in a worktree needs `CARGO_TARGET_DIR` set to that worktree's `target/`. The workspace tests need the test PostgreSQL and MySQL servers; the URLs are in `AGENTS.md` (`TABLETIST_TEST_PG_URL`, `TABLETIST_TEST_MYSQL_URL`).
- Read `AGENTS.md`: views push actions and mutate nothing but the text a field edits; keys are spelled in `src/keymap.rs` alone; no em dashes.
- Each step's diff applies with `git apply` from the repository's root: save the block to a file and run `git apply <file>`. The code here is formatted.
- The tests use only the Bookshop data. No test compares a picture or a position with the design.
- Task 3's code builds only on macOS (`#[cfg(target_os = "macos")]`), and the workspace does not cross-compile here (ring): see "What was run" for how it was type-checked.

## What was run before this plan was written

The boards were read again before the draft. The draft was written whole on run 1's branch at `06c9cb0`, then cut into the three tasks below: whole files go to their task, and the files two tasks touch are rebuilt from the final text by exact replacements. Each task's state passed `cargo fmt --all --check`, `cargo clippy --locked --workspace --all-targets -- -D warnings`, `cargo test --locked --workspace --all-targets` (with the test PostgreSQL and MySQL servers; the counts are in each task's Step 4) and `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps`. Each task's tests were laid over the code before it and run alone, and they failed as its Step 2 says: task 1 fails to build, and task 2's two tests fail. Task 3 has no test of its own.

`src/macos.rs` and the entrypoint's new lines were type-checked for `aarch64-apple-darwin` (clippy, `-D warnings`) in a scratch crate that takes `src/macos.rs` whole (`#[path]`), `tabletist-appkit` by path, and the entrypoint's lines over stubs of the app; a type error planted in `macos.rs` fails it. Nothing of the menu was run: it needs a hand test on a Mac.

The panel and the sheet were rendered in every look, light and dark, through a throwaway scene of `src/shots.rs` (not committed): `> re`, `>` alone, `connection` (the fold), `> zzz`, and Settings opened at Disconnect when idle. The render found one defect: Edit connection and New connection wrote ⌘E and ⌘N, the Connections picker's keys, which a window does not read (now none, with a test of both).

A reviewer then tried to break the draft with 19 throwaway headless tests of its own, in a copy of the tree. It found these, all fixed here:
- The pick was a place in the list, which changes by itself: with a Cancel running query that went when its count ended, Enter ran Discard all pending, and with Switch database listed once the databases came in, Enter on Next connection closed the connection. The pick is now the result it was put on (`QuickOpen::picked`).
- Reload schema wrote ⌘R, which on a table's rows reloads the rows and in a SQL editor does nothing (now no key).
- Save, Discard and Review listed while a save was under way, and Run all on a script of comments alone: each did nothing.
- The `>` was a character constant beside the keymap's spelling of it (now read from the keymap).
- `ShowSetting` put the terminal look's cursor on the first row for an option that screen has no row for.
- A setting's row told a screen reader it was a command.
- The website said `>` lists the settings (it does with a word after it), and `menu_key`'s comment did not say it ignores where the key holds.

It also checked, and they held: every other command against what its key pushes over one frame in fifteen states; each guard; `>` with spaces, alone, with Tab, and in Omarchy (typed as text); the fold and its More row; the Windows look; every setting's row in both looks at two sizes each (its pane, its ground, the keyboard on its control and on screen, not taken back over ten frames); one-letter queries over 10,000 objects under 16 ms; `tests/keys.rs`.

A second reviewer then read this plan against its diffs and the brief. It found Default text size listed where the size set was the look's own (it did nothing there), a largest table's row losing the pick when its estimate was read again, Enter on a "more" row, after the list had moved, leaving the pick on another result than the first the row hid, and tests that asserted less than they said (a key painted elsewhere on the screen, a scroll never shown to be needed); and missing departures, prose that said more than the code (the commands that do not do what their key does: Review pending SQL besides Reload; "a rebound key"), the website's `>` row and app menu line. All are fixed here, and the guards it listed as untested (Filter rows in the Structure view, an empty editor, the text's sizes, `>` in the Windows and Omarchy looks) have tests now. Not taken: a test of a move down from a pick whose result is gone (the move up is tested).

Once approved, the plan was moved onto `main` at `b3a86b29`. The draft was applied there as a three-way patch over `06c9cb0`. The conflicts were the query history's lines beside this run's (resolved to keep both). Its History command became a command of the registry, listed with nothing typed as main lists it, and its test `src/ui/history_tests.rs` names it as one. Its `>` gave way to the keymap's. Its History row in Settings › Editor joined the sheet's sections (29 rows). Task 3, written first as a View menu of its own with ⌘P as its key (`ViewMenu`, `Keymap::menu_key`), was rewritten on main's `ViewItem`, which gives no item a key equivalent. The tasks were cut again, every state checked again (the counts above are of that run), the reds run again, and the macOS extract type-checked again.

Every diff in this plan was applied in order to a `git archive` copy of `b3a86b29` (with this plan's commit), and the result is the last state, with zero mismatches. Not run: CI, macOS and Windows, and the app by hand.

## Files

| File | What changes |
|---|---|
| `src/app/commands.rs` (new) | `RUNNABLE`, `About`, `TabName`, `Runnable`; `App::runnable`, `App::command_actions`, `App::run_of` (task 1) |
| `src/app.rs` | The module and its re-exports; a test that sets the query by hand now says so with `QuickOpenChanged` (task 1); the reducers of `ShowSetting` and `SettingShown`, and `ShowSettingsPane` clearing `asked` (task 2) |
| `src/app/quick_open.rs` | `QuickCommand::Run`, `QuickItem::same`, the pick held by its result, the `>`, `commands_found`, the folded Commands group, running a pick (task 1); `QuickCommand::Setting`, `setting_path`, the settings' rows (task 2) |
| `src/model.rs` | `QuickGroup::Commands`, `QuickOpen::picked` (task 1); `SettingsDialog::asked`, `reveal`, `SettingsPane::of`, `sections`, `sheet_options`, `Action::ShowSetting`, `SettingShown` (task 2) |
| `src/keymap.rs` | `QuickOpenCommands` (task 1) |
| `src/ui/quick_open.rs` | Command rows (`about`), the keys a window reads, `N more commands`, no command matching, the footer's `> commands` (task 1); setting rows (task 2) |
| `src/ui/quick_open_tests.rs` | The brief's test 6 and the run's (tasks 1 and 2) |
| `src/ui/agents_tests.rs` | The Agents command as a `Runnable` (task 1) |
| `src/ui/settings/mod.rs` | `label` is the crate's (task 2) |
| `src/ui/settings/sheet.rs` | The sections from `SettingsPane::sections`; `asked`, used by `row` and `tiles` (task 2) |
| `src/ui/settings/agents.rs` | The agents switch's row answers `asked` (task 2) |
| `src/ui/history_tests.rs` | The History command as a `Runnable` (task 1) |
| `src/macos.rs`, `src/entrypoint.rs` | `QuickOpenMenu`, attached and read each frame (task 3) |
| `docs/_guide/macos.md`, `docs/_guide/browsing-data.md`, `docs/_reference/keyboard-shortcuts.md` | The website (task 3) |

---

### Task 1: The command registry, and `>`

The registry and what quick open does with it: the commands that apply, the `>` that lists them alone, the Commands group folded while typing, the keys a window reads, and the footer's `> commands`.

**Files:** Create `src/app/commands.rs`; modify `src/app.rs`, `src/app/quick_open.rs`, `src/model.rs`, `src/keymap.rs`, `src/ui/quick_open.rs`, `src/ui/quick_open_tests.rs`, `src/ui/agents_tests.rs`, `src/ui/history_tests.rs`.

- [ ] **Step 1: Write the failing tests**

The brief's test 6 (its commands), the registry's tests, the pick held while the list changes, the guards, the fold, the footer, `>` in the Windows and Omarchy looks; and the run 1 and history tests that named the commands of old or set the pick by hand (`src/app.rs`'s pushes `QuickOpenChanged` after setting the query, as the field does).

````diff
diff --git a/src/app.rs b/src/app.rs
--- a/src/app.rs
+++ b/src/app.rs
@@ -16449,6 +16449,7 @@
             };
             // The fake connection has a main.users too: name the schema.
             open.query = "shop.us".into();
+            harness.app.apply(Action::QuickOpenChanged);
             harness.app.apply(Action::QuickOpenPick {
                 index: None,
                 in_editor: false,
diff --git a/src/ui/agents_tests.rs b/src/ui/agents_tests.rs
--- a/src/ui/agents_tests.rs
+++ b/src/ui/agents_tests.rs
@@ -181,15 +181,16 @@
     let Some(crate::model::Dialog::QuickOpen(open)) = &harness.app.dialog else {
         panic!("quick open");
     };
-    let agents = crate::app::QuickItem::Command {
-        command: crate::app::QuickCommand::Agents,
-        spans: std::iter::once(0..3).collect(),
-    };
-    let index = harness
-        .app
-        .quick_open_list(open)
-        .items()
-        .position(|item| *item == agents);
+    let index = harness.app.quick_open_list(open).items().position(|item| {
+        matches!(
+            item,
+            crate::app::QuickItem::Command {
+                command: crate::app::QuickCommand::Run(runnable),
+                spans,
+            } if runnable.command == crate::keymap::Command::ShowAgents
+                && spans.iter().eq(std::iter::once(&(0..3)))
+        )
+    });
     harness.app.apply(Action::QuickOpenPick {
         index,
         in_editor: false,
diff --git a/src/ui/history_tests.rs b/src/ui/history_tests.rs
--- a/src/ui/history_tests.rs
+++ b/src/ui/history_tests.rs
@@ -1119,13 +1119,17 @@
         matches!(
             item,
             crate::app::QuickItem::Command {
-                command: crate::app::QuickCommand::History,
+                command: crate::app::QuickCommand::Run(runnable),
                 ..
-            }
+            } if runnable.command == crate::keymap::Command::History
         )
     };
+    // The commands alone, History first; the setting of how much it keeps
+    // after it.
     let commands = listed(&mut harness, "> Hist");
-    assert_eq!(commands.len(), 1, "the commands alone");
+    let command =
+        |item: &crate::app::QuickItem| matches!(item, crate::app::QuickItem::Command { .. });
+    assert!(commands.iter().all(command), "the commands alone");
     assert!(history(&commands[0]));
     let index = listed(&mut harness, "> History").iter().position(history);
     harness.app.apply(Action::QuickOpenPick {
diff --git a/src/ui/quick_open_tests.rs b/src/ui/quick_open_tests.rs
--- a/src/ui/quick_open_tests.rs
+++ b/src/ui/quick_open_tests.rs
@@ -8,7 +8,9 @@
 use egui::{Key, Modifiers};
 use tabletist_db::{ObjectInfo, ObjectKind, ObjectRef};
 
-use crate::app::{QuickCommand, QuickHeading, QuickItem, QuickRow, StartRow};
+use crate::app::{
+    About, GROUP_SHOWN, QuickCommand, QuickHeading, QuickItem, QuickRow, Runnable, StartRow,
+};
 use crate::backend::Command;
 use crate::connections::SavedConnection;
 use crate::env::Environment;
@@ -134,7 +136,7 @@
                 }
             },
             QuickRow::Item(QuickItem::Command { command, .. }) => {
-                format!("> {}", command.name())
+                format!("> {}", harness.app.command_name(command))
             }
             QuickRow::Item(QuickItem::More { hidden, .. }) => format!("+{hidden}"),
         })
@@ -547,9 +549,13 @@
     let first = list.items().next().cloned();
     assert!(
         matches!(
-            first,
+            &first,
             Some(QuickItem::Command {
-                command: QuickCommand::NewSql { read_only: false },
+                command: QuickCommand::Run(Runnable {
+                    command: crate::keymap::Command::NewSqlTab,
+                    about: About::Nothing,
+                    ..
+                }),
                 ..
             })
         ),
@@ -826,8 +832,10 @@
 fn the_pick_moves_from_where_it_shows() {
     let (mut harness, _tab) = bookshop(Look::macos());
     ask(&mut harness, Look::macos(), "cov");
+    // A pick held by no result, past the list's end.
     if let Some(Dialog::QuickOpen(open)) = &mut harness.app.dialog {
         open.selected = 7;
+        open.picked = None;
     }
     let count = harness
         .app
@@ -933,6 +941,7 @@
         caret: None,
         scrolled_to: None,
         scroll_offset: 0.0,
+        picked: None,
     };
     // Each keystroke of a query, and taking the names apart beside it, the
     // fastest of five tries of each taken as its time, so that a pause of
@@ -961,3 +970,393 @@
         );
     }
 }
+
+/// The index among quick open's results of the one listed as `name`.
+fn result_named(harness: &Harness, name: &str) -> usize {
+    let list = harness.app.quick_open_list(quick_open(harness));
+    let names: Vec<String> = listed(harness)
+        .into_iter()
+        .filter(|name| !name.starts_with("# "))
+        .collect();
+    assert_eq!(names.len(), list.items().count());
+    names
+        .iter()
+        .position(|listed| listed == name)
+        .unwrap_or_else(|| panic!("{name} is not listed: {names:?}"))
+}
+
+/// Test 6: `>` lists the commands alone, and hides those that do not apply
+/// now.
+#[test]
+fn the_prefix_lists_the_commands_that_apply() {
+    let look = Look::macos();
+    let (mut harness, tab) = bookshop(look);
+    ask(&mut harness, look, ">");
+    let shown = listed(&harness);
+    assert_eq!(shown[0], "# Commands");
+    assert!(
+        shown[1..].iter().all(|name| name.starts_with("> ")),
+        "{shown:?}"
+    );
+    for offered in [
+        "> New SQL query",
+        "> Reload schema",
+        "> Edit connection",
+        "> Hide sidebar",
+        "> Settings",
+        "> Larger text",
+    ] {
+        assert!(shown.contains(&offered.to_owned()), "{offered}: {shown:?}");
+    }
+    // The start screen of the one window: no tab to close or filter,
+    // nothing to save or run, no other window, the text at its own size;
+    // and no setting's row until a word is typed.
+    for hidden in [
+        "> Close tab",
+        "> Next tab",
+        "> Filter rows",
+        "> Save changes",
+        "> Run all",
+        "> Next connection",
+        "> Default text size",
+        "> Cancel running query",
+    ] {
+        assert!(!shown.contains(&hidden.to_owned()), "{hidden}: {shown:?}");
+    }
+    assert!(!shown.iter().any(|name| name.starts_with("> Settings ›")));
+    // A row writes a key the window reads, as a screen reader hears it:
+    // New SQL query's; none for Edit connection and New connection, whose
+    // keys are the Connections picker's alone, nor for Reload schema, whose
+    // key on a table's rows reloads them.
+    let layout = crate::keymap::Layout::Mac;
+    let new_sql = harness
+        .app
+        .keymap
+        .key(layout, crate::keymap::Command::NewSqlTab);
+    let new_sql = format!("New SQL query, command, {}", new_sql.as_str());
+    for label in [
+        new_sql.as_str(),
+        "Edit connection, command, Bookshop · dev",
+        "New connection, command",
+        "Reload schema, command, Bookshop · dev",
+    ] {
+        assert!(harness.has(label), "{label}");
+    }
+    harness.press(Key::Escape, Modifiers::NONE);
+    // A table in front brings its own.
+    open_table(&mut harness, tab, "book_covers");
+    ask(&mut harness, look, ">");
+    let shown = listed(&harness);
+    assert!(shown.contains(&"> Close tab".to_owned()), "{shown:?}");
+    assert!(shown.contains(&"> Filter rows".to_owned()), "{shown:?}");
+    assert!(!shown.contains(&"> Save changes".to_owned()), "{shown:?}");
+    harness.press(Key::Escape, Modifiers::NONE);
+    // What follows `>` finds commands, never a table: `reviews` matches
+    // `re` too.
+    ask(&mut harness, look, "> re");
+    let shown = listed(&harness);
+    assert_eq!(shown[1], "> Reload schema");
+    assert!(!shown.contains(&"reviews".to_owned()), "{shown:?}");
+}
+
+/// The commands quick open runs are the keymap's, built, each once; and
+/// each listed does what it names, here a few that change what shows.
+#[test]
+fn listed_commands_do_what_they_name() {
+    let mut seen = Vec::new();
+    for command in crate::app::RUNNABLE {
+        assert!(command.info().built, "{command:?}");
+        assert!(!seen.contains(&command), "{command:?} twice");
+        seen.push(command);
+    }
+    let look = Look::macos();
+    let (mut harness, tab) = bookshop(look);
+    let run = |harness: &mut Harness, name: &str| {
+        ask(harness, look, ">");
+        let index = result_named(harness, &format!("> {name}"));
+        harness.app.apply(Action::QuickOpenPick {
+            index: Some(index),
+            in_editor: false,
+        });
+        harness.settle();
+    };
+    run(&mut harness, "Hide sidebar");
+    assert!(harness.app.workspace(tab).unwrap().sidebar_hidden);
+    run(&mut harness, "Show sidebar");
+    assert!(!harness.app.workspace(tab).unwrap().sidebar_hidden);
+    run(&mut harness, "Larger text");
+    assert!(harness.app.settings.font_size.is_some());
+    run(&mut harness, "Default text size");
+    assert!(harness.app.settings.font_size.is_none());
+    open_table(&mut harness, tab, "book_covers");
+    open_table(&mut harness, tab, "authors");
+    let panel = harness.app.workspace(tab).unwrap().row_panel;
+    let name = if panel {
+        "Hide row panel"
+    } else {
+        "Show row panel"
+    };
+    run(&mut harness, name);
+    assert_eq!(harness.app.workspace(tab).unwrap().row_panel, !panel);
+    run(&mut harness, "Previous tab");
+    let workspace = harness.app.workspace(tab).unwrap();
+    let front = workspace.active_object_tab().unwrap();
+    assert_eq!(front.object.name, "book_covers");
+    run(&mut harness, "Close tab");
+    let workspace = harness.app.workspace(tab).unwrap();
+    assert_eq!(workspace.tabs.len(), 1);
+    // A SQL editor's run and format, once it holds a script.
+    run(&mut harness, "New SQL query");
+    ask(&mut harness, look, ">");
+    assert!(!listed(&harness).contains(&"> Run all".to_owned()));
+    harness.press(Key::Escape, Modifiers::NONE);
+    let workspace = harness.app.workspace_mut(tab).unwrap();
+    let id = workspace.active_tab.unwrap();
+    workspace.sql_tab_mut(id).unwrap().text = "select 1".into();
+    run(&mut harness, "Format SQL");
+    let workspace = harness.app.workspace(tab).unwrap();
+    assert_eq!(workspace.sql_tab(id).unwrap().text, "SELECT 1");
+    run(&mut harness, "Settings");
+    assert!(matches!(harness.app.dialog, Some(Dialog::Settings(_))));
+}
+
+/// A Bookshop development window that writes, its start screen showing.
+fn bookshop_writable(look: Look) -> (Harness, ConnTabId) {
+    let mut harness = Harness::new();
+    harness.set_look(look);
+    let mut saved = bookshop_dev();
+    saved.read_only = Some(false);
+    let tab = harness.connect_with(saved, bookshop_with_publishers());
+    (harness, tab)
+}
+
+/// `book_covers` open on its rows, with a row deleted and not saved.
+fn covers_with_a_change(harness: &mut Harness, tab: ConnTabId) -> crate::model::TabId {
+    harness.app.apply(Action::OpenObject {
+        tab,
+        object: ObjectRef::new("main", "book_covers"),
+        kind: ObjectKind::Table,
+        pin: true,
+    });
+    harness.answer_structure(crate::testing::book_covers_structure());
+    harness.answer_rows(crate::testing::book_covers_page(3));
+    let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
+    harness.app.workspace_mut(tab).unwrap().pane = crate::model::Pane::Grid;
+    harness.settle();
+    let cell = crate::model::CellPos { row: 1, col: 1 };
+    harness.app.apply(Action::SelectCell { tab, id, cell });
+    harness.app.apply(Action::DeleteRow { tab, id });
+    harness.settle();
+    id
+}
+
+/// Whether `book_covers` still holds its change.
+fn changed(harness: &Harness, tab: ConnTabId, id: crate::model::TabId) -> bool {
+    let workspace = harness.app.workspace(tab).unwrap();
+    workspace.object_tab(id).unwrap().edits.pending()
+}
+
+/// The pick is the command it was put on, not a place in the list, which
+/// changes by itself: a command that comes above it moves it, and one that
+/// goes leaves Enter nothing to open (a review finding: Enter ran the
+/// command that slid into its place).
+#[test]
+fn the_pick_holds_its_command_while_the_list_changes() {
+    let look = Look::macos();
+    let (mut harness, first) = bookshop_writable(look);
+    harness.app.apply(Action::ShowConnections);
+    let second = harness.connect_with(bookshop_production(), bookshop_objects());
+    ask(&mut harness, look, ">");
+    let next = result_named(&harness, "> Next connection");
+    for _ in 0..next {
+        harness.press(Key::ArrowDown, Modifiers::NONE);
+    }
+    // The server's other databases come in: Switch database is listed
+    // above the pick.
+    let databases = Some(vec!["bookshop".into(), "bookshop_archive".into()]);
+    harness.app.workspace_mut(second).unwrap().databases.value = databases;
+    harness.settle();
+    assert_eq!(result_named(&harness, "> Next connection"), next + 1);
+    harness.press(Key::Enter, Modifiers::NONE);
+    assert!(harness.app.workspace(second).is_some(), "not closed");
+    assert_eq!(harness.app.active_tab_id(), first);
+    // A count in flight lists Cancel, after Discard all pending; the count
+    // ends before Enter.
+    let id = covers_with_a_change(&mut harness, first);
+    let request = Some(crate::backend::RequestId(9_999));
+    let workspace = harness.app.workspace_mut(first).unwrap();
+    workspace.object_tab_mut(id).unwrap().count.pending = request;
+    ask(&mut harness, look, ">");
+    let cancel = result_named(&harness, "> Cancel running query");
+    assert_eq!(cancel - 1, result_named(&harness, "> Discard all pending"));
+    for _ in 0..cancel {
+        harness.press(Key::ArrowDown, Modifiers::NONE);
+    }
+    let workspace = harness.app.workspace_mut(first).unwrap();
+    workspace.object_tab_mut(id).unwrap().count.pending = None;
+    harness.press(Key::Enter, Modifiers::NONE);
+    assert!(changed(&harness, first, id), "nothing discarded");
+    assert!(matches!(harness.app.dialog, Some(Dialog::QuickOpen(_))));
+    // The next move starts from where the pick stood.
+    harness.press(Key::ArrowUp, Modifiers::NONE);
+    let discard = result_named(&harness, "> Discard all pending");
+    let list = harness.app.quick_open_list(quick_open(&harness));
+    let picked = harness.app.quick_open_picked(quick_open(&harness), &list);
+    assert_eq!(picked, Some(discard));
+}
+
+/// A command that would do nothing is not listed: a save's three while a
+/// save is under way, Run all with comments alone (review findings).
+#[test]
+fn commands_with_nothing_to_do_are_not_listed() {
+    let look = Look::macos();
+    let (mut harness, tab) = bookshop_writable(look);
+    let id = covers_with_a_change(&mut harness, tab);
+    ask(&mut harness, look, ">");
+    let shown = listed(&harness);
+    assert!(shown.contains(&"> Save changes".to_owned()), "{shown:?}");
+    harness.press(Key::Escape, Modifiers::NONE);
+    harness.app.apply(Action::WriteEdits { tab, id });
+    ask(&mut harness, look, ">");
+    let shown = listed(&harness);
+    for hidden in [
+        "> Save changes",
+        "> Discard all pending",
+        "> Review pending SQL",
+    ] {
+        assert!(!shown.contains(&hidden.to_owned()), "{hidden}: {shown:?}");
+    }
+    harness.press(Key::Escape, Modifiers::NONE);
+    // The Structure view has no rows to filter.
+    let view = crate::model::ObjectView::Structure;
+    harness.app.apply(Action::SetView {
+        tab,
+        object_tab: id,
+        view,
+    });
+    ask(&mut harness, look, ">");
+    assert!(!listed(&harness).contains(&"> Filter rows".to_owned()));
+    harness.press(Key::Escape, Modifiers::NONE);
+    // An empty editor has nothing to run or format, comments alone
+    // nothing to run.
+    harness.app.apply(Action::NewSqlTab(tab));
+    ask(&mut harness, look, ">");
+    let shown = listed(&harness);
+    assert!(!shown.contains(&"> Run all".to_owned()), "{shown:?}");
+    assert!(!shown.contains(&"> Format SQL".to_owned()), "{shown:?}");
+    harness.press(Key::Escape, Modifiers::NONE);
+    let workspace = harness.app.workspace_mut(tab).unwrap();
+    let sql = workspace.active_tab.unwrap();
+    workspace.sql_tab_mut(sql).unwrap().text = "-- just a note".into();
+    ask(&mut harness, look, ">");
+    let shown = listed(&harness);
+    assert!(!shown.contains(&"> Run all".to_owned()), "{shown:?}");
+    assert!(shown.contains(&"> Format SQL".to_owned()), "{shown:?}");
+    harness.press(Key::Escape, Modifiers::NONE);
+    // The text's size: no larger at the largest, and no default where
+    // the size set is the look's own.
+    for _ in 0..20 {
+        harness.app.apply(Action::TextSize(Some(true)));
+    }
+    ask(&mut harness, look, ">");
+    assert!(!listed(&harness).contains(&"> Larger text".to_owned()));
+    harness.press(Key::Escape, Modifiers::NONE);
+    harness.app.apply(Action::TextSize(None));
+    harness.app.apply(Action::TextSize(Some(true)));
+    harness.app.apply(Action::TextSize(Some(false)));
+    assert!(harness.app.settings.font_size.is_some());
+    ask(&mut harness, look, ">");
+    assert!(!listed(&harness).contains(&"> Default text size".to_owned()));
+}
+
+/// The Windows look lists the commands after `>` as macOS does; Omarchy
+/// types it, and finds nothing by it.
+#[test]
+fn the_prefix_in_the_windows_and_omarchy_looks() {
+    let look = Look::standard();
+    let (mut harness, _tab) = bookshop(look);
+    ask(&mut harness, look, "> new sql");
+    assert_eq!(listed(&harness)[1], "> New SQL query");
+    let look = Look::omarchy();
+    let (mut harness, _tab) = bookshop(look);
+    ask(&mut harness, look, ">");
+    assert_eq!(quick_open(&harness).query, ">");
+    assert!(listed(&harness).is_empty(), "{:?}", listed(&harness));
+}
+
+/// Another window brings the commands that go to it.
+#[test]
+fn the_next_window_is_offered_where_there_is_one() {
+    let look = Look::macos();
+    let (mut harness, _tab) = bookshop(look);
+    ask(&mut harness, look, ">");
+    assert!(!listed(&harness).contains(&"> Next connection".to_owned()));
+    harness.press(Key::Escape, Modifiers::NONE);
+    harness.app.apply(Action::ShowConnections);
+    harness.connect_with(bookshop_production(), bookshop_objects());
+    ask(&mut harness, look, ">");
+    let shown = listed(&harness);
+    assert!(shown.contains(&"> Next connection".to_owned()), "{shown:?}");
+    assert!(shown.contains(&"> Previous connection".to_owned()));
+}
+
+/// Typed without `>`, the commands and settings that match come after the
+/// objects, folded after three as each kind of object is; with nothing
+/// typed, only the suggested three.
+#[test]
+fn the_commands_group_folds_after_three() {
+    let look = Look::macos();
+    let (mut harness, _tab) = bookshop(look);
+    ask(&mut harness, look, "");
+    let commands: Vec<String> = listed(&harness)
+        .into_iter()
+        .filter(|name| name.starts_with("> "))
+        .collect();
+    assert!(commands.len() <= 3, "{commands:?}");
+    harness.press(Key::Escape, Modifiers::NONE);
+    harness.app.apply(Action::ShowConnections);
+    harness.connect_with(bookshop_production(), bookshop_objects());
+    ask(&mut harness, look, "connection");
+    let list = harness.app.quick_open_list(quick_open(&harness));
+    let heading = list
+        .rows
+        .iter()
+        .position(|row| *row == QuickRow::Heading(QuickHeading::Commands))
+        .expect("the commands' heading");
+    let after = &list.rows[heading + 1..];
+    assert_eq!(after.len(), GROUP_SHOWN + 1, "{after:?}");
+    let Some(QuickRow::Item(QuickItem::More { group, hidden })) = after.last() else {
+        panic!("{after:?}");
+    };
+    assert_eq!(*group, QuickGroup::Commands);
+    assert!(*hidden > 0);
+    let more = list.items().count() - 1;
+    harness.app.apply(Action::QuickOpenPick {
+        index: Some(more),
+        in_editor: false,
+    });
+    let list = harness.app.quick_open_list(quick_open(&harness));
+    assert!(
+        list.items()
+            .all(|item| !matches!(item, QuickItem::More { .. }))
+    );
+}
+
+/// The footer has `> commands` on macOS; Omarchy's commands stay on `:`.
+/// What `>` finds nothing for is said.
+#[test]
+fn the_footer_names_the_prefix_and_no_command_is_said() {
+    for look in [Look::macos(), Look::omarchy()] {
+        let (mut harness, _tab) = bookshop(look);
+        ask(&mut harness, look, "");
+        let hint = harness
+            .painted
+            .iter()
+            .any(|(text, _)| text.ends_with(" commands"));
+        assert_eq!(hint, !look.terminal, "{}", look.name);
+    }
+    let look = Look::macos();
+    let (mut harness, _tab) = bookshop(look);
+    ask(&mut harness, look, "> zzz");
+    assert!(harness.painted_color("No commands match “zzz”").is_some());
+}
````

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --locked -p tabletist --lib -- quick_open agents_tests history_tests`

Expected: the build fails, as the registry does not exist yet:

```
error[E0432]: unresolved imports `crate::app::About`, `crate::app::Runnable`
  --> src/ui/quick_open_tests.rs:12:5
error[E0425]: cannot find value `RUNNABLE` in module `crate::app`
    --> src/ui/quick_open_tests.rs:1067:32
error[E0599]: no variant, associated function, or constant named `Run` found for enum `QuickCommand` in the current scope
   --> src/ui/agents_tests.rs:188:52
error[E0599]: no variant, associated function, or constant named `Run` found for enum `QuickCommand` in the current scope
    --> src/ui/history_tests.rs:1122:52
error[E0599]: no method named `command_name` found for struct `app::App` in the current scope
   --> src/ui/quick_open_tests.rs:139:45
error[E0599]: no variant, associated function, or constant named `Run` found for enum `QuickCommand` in the current scope
   --> src/ui/quick_open_tests.rs:554:40
error[E0609]: no field `picked` on type `&mut Box<QuickOpen>`
   --> src/ui/quick_open_tests.rs:838:14
error[E0560]: struct `QuickOpen` has no field named `picked`
   --> src/ui/quick_open_tests.rs:944:9
error[E0599]: no method named `quick_open_picked` found for struct `app::App` in the current scope
    --> src/ui/quick_open_tests.rs:1204:30
error[E0599]: no variant, associated function, or constant named `Commands` found for enum `QuickGroup` in the current scope
    --> src/ui/quick_open_tests.rs:1331:36
error: could not compile `tabletist` (lib test) due to 10 previous errors
```

- [ ] **Step 3: Write the code**

````diff
diff --git a/src/app.rs b/src/app.rs
--- a/src/app.rs
+++ b/src/app.rs
@@ -27,6 +27,7 @@
 use crate::settings::{Loaded, Settings, SettingsFile, Source};
 use crate::theme::{self, Catalog, Palette};
 
+mod commands;
 mod editing;
 mod history;
 mod key_picker;
@@ -35,6 +36,7 @@
 #[cfg(test)]
 mod startup_tests;
 
+pub use commands::{About, RUNNABLE, Runnable, TabName};
 pub use quick_open::{
     Found, GROUP_SHOWN, QuickCommand, QuickHeading, QuickItem, QuickList, QuickRow, StartRow,
 };
diff --git a/src/app/commands.rs b/src/app/commands.rs
new file mode 100644
--- /dev/null
+++ b/src/app/commands.rs
@@ -0,0 +1,364 @@
+//! The commands quick open runs (macOS, Windows): each a command of the
+//! keymap's, with the name quick open lists it by, what it says of where it
+//! acts, and what running it does. A command lists only where it applies
+//! now: a tab's commands with that tab in front, a save with something to
+//! save. The terminal look's commands stay on `:`.
+
+use crate::keymap::Command;
+use crate::model::{Action, ConnTabId, RunMode, Tab};
+
+use super::App;
+
+/// The commands quick open can run, in the order it lists them: the
+/// window's, then the tab's in front. Each is built, and does here what
+/// its key does in the window, but three: quick open's Reload reloads the
+/// schema, which the key does with the keyboard in the sidebar (on a
+/// table's rows it reloads them); its Review pending SQL shows the
+/// review, which the key shows and hides by turns; and its Agents shows
+/// the Agents tab, which the key does but where History has an agent's
+/// statement picked (there it shows that call).
+pub const RUNNABLE: [Command; 29] = [
+    Command::NewSqlTab,
+    Command::SwitchDatabase,
+    Command::ShowAgents,
+    Command::History,
+    Command::Reload,
+    Command::EditConnection,
+    Command::CloseConnection,
+    Command::NextConnection,
+    Command::PreviousConnection,
+    Command::Connections,
+    Command::NewConnection,
+    Command::ToggleSidebar,
+    Command::Settings,
+    Command::Help,
+    Command::LargerText,
+    Command::SmallerText,
+    Command::DefaultTextSize,
+    Command::CloseTab,
+    Command::NextTab,
+    Command::PreviousTab,
+    Command::FilterBar,
+    Command::ToggleInspector,
+    Command::ReviewSql,
+    Command::SaveChanges,
+    Command::DiscardChanges,
+    Command::RunAll,
+    Command::Format,
+    Command::ToggleSqlMode,
+    Command::CancelQuery,
+];
+
+/// What a command's row says after its name: where it acts.
+#[derive(Debug, Clone, PartialEq, Eq)]
+pub enum About {
+    Nothing,
+    /// New SQL query on production, where a new editor opens read-only.
+    OpensReadOnly,
+    /// The window's connection (`Bookshop · dev`).
+    Connection,
+    /// The window's connection, and who is connected to its Agents tab.
+    Agents,
+    /// The tab in front.
+    Tab(TabName),
+}
+
+/// A tab as a command's row names it.
+#[derive(Debug, Clone, PartialEq, Eq)]
+pub enum TabName {
+    /// A table or view, by its name.
+    Object(String),
+    /// A SQL editor, by its number (`Query 2`).
+    Query(u32),
+    Agents,
+    History,
+}
+
+impl TabName {
+    fn of(tab: &Tab) -> Self {
+        match tab {
+            Tab::Object(object) => Self::Object(object.object.name.clone()),
+            Tab::Sql(sql) => Self::Query(sql.number),
+            Tab::Agents(_) => Self::Agents,
+            Tab::History(_) => Self::History,
+        }
+    }
+}
+
+/// A command quick open offers: the keymap's command, the name it is
+/// listed by (English: the caller translates it), and what its row says
+/// after the name.
+#[derive(Debug, Clone, PartialEq, Eq)]
+pub struct Runnable {
+    pub command: Command,
+    pub name: &'static str,
+    pub about: About,
+}
+
+impl App {
+    /// The commands quick open offers in `tab`'s window, in the order of
+    /// [`RUNNABLE`], each where it applies now. None in the terminal look.
+    pub fn runnable(&self, tab: ConnTabId) -> Vec<Runnable> {
+        if self.look.terminal {
+            return Vec::new();
+        }
+        RUNNABLE
+            .into_iter()
+            .filter_map(|command| self.run_of(tab, command).map(|(runnable, _)| runnable))
+            .collect()
+    }
+
+    /// What running `command` in `tab`'s window does: the actions its key
+    /// pushes there ([`RUNNABLE`] says where not). None where it does not
+    /// apply now.
+    pub(super) fn command_actions(&self, tab: ConnTabId, command: Command) -> Option<Vec<Action>> {
+        self.run_of(tab, command).map(|(_, actions)| actions)
+    }
+
+    /// `command` as quick open offers it in `tab`'s window, and what
+    /// running it does; None where it does not apply now.
+    fn run_of(&self, tab: ConnTabId, command: Command) -> Option<(Runnable, Vec<Action>)> {
+        let workspace = self.workspace(tab)?;
+        // The tab in front, and the object or SQL editor it is.
+        let front = workspace.active_tab.and_then(|id| workspace.tab(id));
+        let object = front.and_then(Tab::as_object);
+        let sql = front.and_then(Tab::as_sql);
+        // Changes to save or drop: none while a save is under way, which
+        // the reducers refuse to touch.
+        let pending =
+            object.filter(|object| object.edits.pending() && object.edits.saving.is_none());
+        let in_front = || front.map(|tab| About::Tab(TabName::of(tab)));
+        let stepped = |larger| {
+            let terminal = self.look.terminal;
+            let settings = &self.settings;
+            settings.stepped_font_size(terminal, larger) != settings.font_size_in(terminal)
+        };
+        let (name, about, actions) = match command {
+            Command::NewSqlTab => {
+                let read_only = workspace.environment.is_production()
+                    && (self.settings.new_tab == crate::settings::NewTab::ReadOnly
+                        || RunMode::of_new_tab(workspace.sql_writes(), workspace.environment)
+                            == RunMode::ReadOnly);
+                let about = if read_only {
+                    About::OpensReadOnly
+                } else {
+                    About::Nothing
+                };
+                ("New SQL query", about, vec![Action::NewSqlTab(tab)])
+            }
+            // Where the server has another database to switch to.
+            Command::SwitchDatabase => {
+                let databases = workspace.databases.value.as_ref();
+                databases.filter(|databases| databases.len() > 1)?;
+                (
+                    "Switch database",
+                    About::Nothing,
+                    vec![Action::ShowDatabaseMenu(tab)],
+                )
+            }
+            // Where agents can reach the app.
+            Command::ShowAgents => {
+                if !crate::endpoint::AVAILABLE {
+                    return None;
+                }
+                ("Agents", About::Agents, vec![Action::ShowAgents(tab)])
+            }
+            Command::History => ("History", About::Connection, vec![Action::ShowHistory(tab)]),
+            Command::Reload => (
+                "Reload schema",
+                About::Connection,
+                vec![Action::RefreshTree(tab)],
+            ),
+            Command::EditConnection => {
+                self.connections.get(&workspace.conn_id)?;
+                let edit = Action::EditConnection(workspace.conn_id.clone());
+                ("Edit connection", About::Connection, vec![edit])
+            }
+            Command::CloseConnection => (
+                "Close connection",
+                About::Connection,
+                vec![Action::CloseConnTab(tab)],
+            ),
+            // Where there is another window to go to.
+            Command::NextConnection | Command::PreviousConnection => {
+                if self.tabs.len() < 2 {
+                    return None;
+                }
+                let (name, step) = match command {
+                    Command::NextConnection => ("Next connection", 1),
+                    _ => ("Previous connection", -1),
+                };
+                (name, About::Nothing, vec![Action::CycleConnTab(step)])
+            }
+            Command::Connections => ("Connections", About::Nothing, vec![Action::ShowConnections]),
+            Command::NewConnection => (
+                "New connection",
+                About::Nothing,
+                vec![Action::NewConnection],
+            ),
+            Command::ToggleSidebar => {
+                let name = if workspace.sidebar_hidden {
+                    "Show sidebar"
+                } else {
+                    "Hide sidebar"
+                };
+                (name, About::Nothing, vec![Action::ToggleSidebar(tab)])
+            }
+            Command::Settings => ("Settings", About::Nothing, vec![Action::ShowSettings]),
+            Command::Help => ("Keyboard shortcuts", About::Nothing, vec![Action::ShowHelp]),
+            // Each where it changes the size: not at the end of the steps,
+            // nor back to the look's own where it is that already.
+            Command::LargerText => {
+                stepped(true).then_some(())?;
+                (
+                    "Larger text",
+                    About::Nothing,
+                    vec![Action::TextSize(Some(true))],
+                )
+            }
+            Command::SmallerText => {
+                stepped(false).then_some(())?;
+                (
+                    "Smaller text",
+                    About::Nothing,
+                    vec![Action::TextSize(Some(false))],
+                )
+            }
+            Command::DefaultTextSize => {
+                let terminal = self.look.terminal;
+                let default = crate::settings::FontSizes::of(terminal).default;
+                if self.settings.font_size_in(terminal) == default {
+                    return None;
+                }
+                (
+                    "Default text size",
+                    About::Nothing,
+                    vec![Action::TextSize(None)],
+                )
+            }
+            // The tab in front's.
+            Command::CloseTab => {
+                let id = front?.id();
+                ("Close tab", in_front()?, vec![Action::CloseTab { tab, id }])
+            }
+            Command::NextTab | Command::PreviousTab => {
+                if workspace.tabs.len() < 2 {
+                    return None;
+                }
+                let (name, step) = match command {
+                    Command::NextTab => ("Next tab", 1),
+                    _ => ("Previous tab", -1),
+                };
+                (name, About::Nothing, vec![Action::CycleTab { tab, step }])
+            }
+            // A table's rows: its Structure view has none to filter. An
+            // open bar takes the keyboard back.
+            Command::FilterBar => {
+                let object =
+                    object.filter(|object| object.view == crate::model::ObjectView::Data)?;
+                let filter = if object.filter.open {
+                    Action::FocusFilterBar(tab)
+                } else {
+                    Action::ToggleFilterBar(tab)
+                };
+                ("Filter rows", in_front()?, vec![filter])
+            }
+            // Where it has a row to show: a table's rows, or the row
+            // selected in a SQL editor's result.
+            Command::ToggleInspector => {
+                let data =
+                    object.is_some_and(|object| object.view == crate::model::ObjectView::Data);
+                let row = sql.is_some_and(|sql| sql.selected_row().is_some());
+                if !data && !row {
+                    return None;
+                }
+                let name = if workspace.row_panel {
+                    "Hide row panel"
+                } else {
+                    "Show row panel"
+                };
+                (name, in_front()?, vec![Action::ToggleRowPanel(tab)])
+            }
+            // A table's pending changes: shown, saved or dropped.
+            Command::ReviewSql => {
+                let object = pending.filter(|object| !object.edits.reviewing)?;
+                let id = object.id;
+                let review = Action::ReviewEdits {
+                    tab,
+                    id,
+                    show: true,
+                };
+                ("Review pending SQL", in_front()?, vec![review])
+            }
+            Command::SaveChanges => {
+                let id = pending?.id;
+                (
+                    "Save changes",
+                    in_front()?,
+                    vec![Action::WriteEdits { tab, id }],
+                )
+            }
+            Command::DiscardChanges => {
+                let id = pending?.id;
+                (
+                    "Discard all pending",
+                    in_front()?,
+                    vec![Action::DiscardEdits { tab, id }],
+                )
+            }
+            // A SQL editor's: a script to run or format, and its runs'
+            // mode where the connection allows writes.
+            Command::RunAll => {
+                // A script with a statement in it: comments alone run
+                // nothing.
+                let dialect = workspace.driver.dialect();
+                let statements = |sql: &crate::model::SqlTab| {
+                    !tabletist_db::sql::statements(dialect, &sql.text).is_empty()
+                };
+                let sql = sql.filter(|sql| statements(sql) && !sql.is_writing())?;
+                let run = Action::RunSql {
+                    tab,
+                    sql_tab: sql.id,
+                    all: true,
+                };
+                ("Run all", in_front()?, vec![run])
+            }
+            Command::Format => {
+                let sql = sql.filter(|sql| !sql.text.trim().is_empty())?;
+                let format = Action::FormatSql {
+                    tab,
+                    sql_tab: sql.id,
+                };
+                ("Format SQL", in_front()?, vec![format])
+            }
+            Command::ToggleSqlMode => {
+                let sql = sql.filter(|_| workspace.sql_writes().is_ok())?;
+                let name = match sql.mode {
+                    RunMode::ReadOnly => "Read-write tab",
+                    RunMode::ReadWrite => "Read-only tab",
+                };
+                let toggle = Action::ToggleSqlMode {
+                    tab,
+                    sql_tab: sql.id,
+                };
+                (name, in_front()?, vec![toggle])
+            }
+            // While something of the window loads or runs.
+            Command::CancelQuery => {
+                self.is_busy(tab).then_some(())?;
+                (
+                    "Cancel running query",
+                    About::Connection,
+                    vec![Action::CancelQuery(tab)],
+                )
+            }
+            _ => return None,
+        };
+        let runnable = Runnable {
+            command,
+            name,
+            about,
+        };
+        Some((runnable, actions))
+    }
+}
diff --git a/src/app/quick_open.rs b/src/app/quick_open.rs
--- a/src/app/quick_open.rs
+++ b/src/app/quick_open.rs
@@ -11,11 +11,22 @@
 use tabletist_db::{ObjectKind, ObjectRef};
 
 use super::App;
-use crate::model::{Action, ConnTabId, Dialog, QuickGroup, QuickOpen, RunMode, StartRecent};
+use super::commands::Runnable;
+use crate::keymap::Command;
+use crate::model::{Action, ConnTabId, Dialog, QuickGroup, QuickOpen, StartRecent};
 use crate::search::{self, Tier};
 
 /// How many results a group shows before its "more" row (macOS).
 pub const GROUP_SHOWN: usize = 3;
+
+/// The commands quick open offers with nothing typed, where they apply:
+/// the boards' three, and History.
+const SUGGESTED: [Command; 4] = [
+    Command::NewSqlTab,
+    Command::SwitchDatabase,
+    Command::ShowAgents,
+    Command::History,
+];
 
 /// A table or view that matches what is typed.
 #[derive(Debug, Clone, PartialEq, Eq)]
@@ -135,41 +146,17 @@
 }
 
 /// One of the commands quick open offers in the macOS and Windows looks.
-#[derive(Debug, Clone, Copy, PartialEq, Eq)]
+#[derive(Debug, Clone, PartialEq, Eq)]
 pub enum QuickCommand {
-    /// A new SQL editor. `read_only` where a production connection's new
-    /// editors open read-only, which the row says.
-    NewSql { read_only: bool },
-    /// The menu of the server's other databases. Offered only where there
-    /// is another.
-    SwitchDatabase,
-    /// The window's Agents tab. Offered only where agents can reach the
-    /// app.
-    Agents,
-    /// The window's History tab.
-    History,
+    /// A command of the keymap's, run as its key runs it.
+    Run(Runnable),
 }
 
 impl QuickCommand {
-    /// The command's name, as the row writes it and what is typed
-    /// matches.
-    pub fn name(self) -> &'static str {
+    /// The keymap's command it is, whose key the row writes.
+    pub fn command(&self) -> Command {
         match self {
-            Self::NewSql { .. } => "New SQL query",
-            Self::SwitchDatabase => "Switch database",
-            Self::Agents => "Agents",
-            Self::History => "History",
-        }
-    }
-
-    /// The keymap's command it is, whose key the row writes.
-    pub fn command(self) -> crate::keymap::Command {
-        use crate::keymap::Command;
-        match self {
-            Self::NewSql { .. } => Command::NewSqlTab,
-            Self::SwitchDatabase => Command::SwitchDatabase,
-            Self::Agents => Command::ShowAgents,
-            Self::History => Command::History,
+            Self::Run(runnable) => runnable.command,
         }
     }
 }
@@ -194,6 +181,50 @@
     More { group: QuickGroup, hidden: usize },
 }
 
+impl QuickItem {
+    /// Whether `other` is this result, though it may be listed elsewhere
+    /// or say something else of itself now (an estimate read again, a
+    /// command's name): the same object of the same window, the same row
+    /// of a start screen, the same command, the same group's "more" row.
+    pub fn same(&self, other: &QuickItem) -> bool {
+        match (self, other) {
+            (
+                Self::Object { tab, found },
+                Self::Object {
+                    tab: at,
+                    found: other,
+                },
+            ) => tab == at && found.object == other.object && found.kind == other.kind,
+            (
+                Self::Start { tab, row, .. },
+                Self::Start {
+                    tab: at,
+                    row: other,
+                    ..
+                },
+            ) => {
+                // The largest tables by their table: an estimate read
+                // again is the same row.
+                let largest = |row: &StartRow| match row {
+                    StartRow::Largest(object, _) => Some(object.clone()),
+                    StartRow::Recent(_) => None,
+                };
+                tab == at
+                    && match (largest(row), largest(other)) {
+                        (Some(one), Some(two)) => one == two,
+                        _ => row == other,
+                    }
+            }
+            (Self::Command { command, .. }, Self::Command { command: other, .. }) => {
+                let (QuickCommand::Run(one), QuickCommand::Run(two)) = (command, other);
+                one.command == two.command
+            }
+            (Self::More { group, .. }, Self::More { group: other, .. }) => group == other,
+            _ => false,
+        }
+    }
+}
+
 /// What heads a group of quick open's results.
 #[derive(Debug, Clone, Copy, PartialEq, Eq)]
 pub enum QuickHeading {
@@ -248,6 +279,13 @@
             self.rows
                 .push(QuickRow::Item(QuickItem::More { group, hidden }));
         }
+    }
+}
+
+/// `command`'s name, translated.
+fn name_of(locale: crate::i18n::Locale, command: &QuickCommand) -> String {
+    match command {
+        QuickCommand::Run(runnable) => crate::i18n::gettext(locale, runnable.name).into_owned(),
     }
 }
 
@@ -377,57 +415,69 @@
         windows
     }
 
-    /// The commands quick open offers in the macOS and Windows looks, in
-    /// order, those that can do something now: none in the terminal look,
-    /// whose commands are on `:`.
-    fn quick_commands(&self, tab: ConnTabId) -> Vec<QuickCommand> {
-        let Some(workspace) = self.workspace(tab) else {
-            return Vec::new();
-        };
-        if self.look.terminal {
-            return Vec::new();
-        }
-        let read_only = workspace.environment.is_production()
-            && (self.settings.new_tab == crate::settings::NewTab::ReadOnly
-                || RunMode::of_new_tab(workspace.sql_writes(), workspace.environment)
-                    == RunMode::ReadOnly);
-        let switchable = workspace
-            .databases
-            .value
-            .as_ref()
-            .is_some_and(|databases| databases.len() > 1);
-        let mut commands = vec![QuickCommand::NewSql { read_only }];
-        if switchable {
-            commands.push(QuickCommand::SwitchDatabase);
-        }
-        if crate::endpoint::AVAILABLE {
-            commands.push(QuickCommand::Agents);
-        }
-        commands.push(QuickCommand::History);
-        commands
+    /// A command's name, as its row writes it and what is typed matches.
+    pub fn command_name(&self, command: &QuickCommand) -> String {
+        name_of(self.locale, command)
+    }
+
+    /// The commands `typed` (folded) finds in `tab`'s window, best first,
+    /// in the registry's order within a tier: each with the runs of its
+    /// name that matched. With nothing typed, every command
+    /// (`commands_only`), or the suggested ones. None in the terminal
+    /// look.
+    fn commands_found(
+        &self,
+        tab: ConnTabId,
+        typed: &[char],
+        commands_only: bool,
+    ) -> Vec<QuickItem> {
+        let commands: Vec<QuickCommand> = self
+            .runnable(tab)
+            .into_iter()
+            .filter(|runnable| {
+                !typed.is_empty() || commands_only || SUGGESTED.contains(&runnable.command)
+            })
+            .map(QuickCommand::Run)
+            .collect();
+        let mut found: Vec<(Tier, QuickItem)> = commands
+            .into_iter()
+            .filter_map(|command| {
+                if typed.is_empty() {
+                    let spans = Vec::new();
+                    return Some((Tier::Exact, QuickItem::Command { command, spans }));
+                }
+                let hit = search::find(typed, &name_of(self.locale, &command))?;
+                let spans = hit.spans;
+                Some((hit.tier, QuickItem::Command { command, spans }))
+            })
+            .collect();
+        // Best first; a sort that keeps their order within a tier.
+        found.sort_by_key(|(tier, _)| std::cmp::Reverse(*tier));
+        found.into_iter().map(|(_, item)| item).collect()
     }
 
     /// What quick open lists for `open`. With nothing typed: each searched
     /// window's start screen list (headed by its connection where every
-    /// open one is searched), then the commands. With a query: the tables
-    /// and then the views that match, or where every open connection is
-    /// searched, each connection's matches under it; then the commands
-    /// whose names match. In the macOS and Windows looks each group shows
-    /// its first three and a "more" row until that is opened; the terminal
-    /// look lists every match, in one list with no kind headings, and no
-    /// commands. After `>` it lists the commands alone, by what follows
-    /// it: `> History`.
+    /// open one is searched), then the suggested commands. With a query:
+    /// the tables and then the views that match, or where every open
+    /// connection is searched, each connection's matches under it; then
+    /// the commands whose names match. In the macOS
+    /// and Windows looks each group shows its first three and a "more" row
+    /// until that is opened, and a query that begins with `>` lists every
+    /// command that matches what follows it, and nothing else; the
+    /// terminal look lists every match, in one list with no kind headings,
+    /// and no commands.
     pub fn quick_open_list(&self, open: &QuickOpen) -> QuickList {
         let terminal = self.look.terminal;
-        let (commands_only, query) = match open.query.trim().strip_prefix('>') {
-            Some(rest) => (true, rest.trim()),
-            None => (false, open.query.trim()),
-        };
+        let commands_query = self.quick_open_commands_query(open);
+        let commands_only = commands_query.is_some();
+        let query = commands_query.unwrap_or(open.query.trim());
         let mut list = QuickList::default();
         let whole = |group| terminal || open.expanded.contains(&group);
-        let windows = match commands_only {
-            true => Vec::new(),
-            false => self.quick_open_windows(open),
+        let windows = if commands_only {
+            Vec::new()
+        } else {
+            self.quick_open_windows(open)
         };
         for tab in windows {
             if query.is_empty() {
@@ -478,28 +528,41 @@
             }
         }
         let typed = search::folded(query);
-        let mut commands: Vec<(Tier, QuickItem)> = self
-            .quick_commands(open.tab)
-            .into_iter()
-            .filter_map(|command| {
-                let name = crate::i18n::gettext(self.locale, command.name());
-                if typed.is_empty() {
-                    let spans = Vec::new();
-                    return Some((Tier::Exact, QuickItem::Command { command, spans }));
-                }
-                let hit = search::find(&typed, &name)?;
-                let spans = hit.spans;
-                Some((hit.tier, QuickItem::Command { command, spans }))
-            })
-            .collect();
-        // Best first; a sort that keeps their order within a tier.
-        commands.sort_by_key(|(tier, _)| std::cmp::Reverse(*tier));
+        let commands = self.commands_found(open.tab, &typed, commands_only);
         if !commands.is_empty() {
             list.rows.push(QuickRow::Heading(QuickHeading::Commands));
-            list.rows
-                .extend(commands.into_iter().map(|(_, item)| QuickRow::Item(item)));
+            // Folded under what was typed, as each kind of object is; the
+            // commands alone and the suggested three are listed whole.
+            let group = QuickGroup::Commands;
+            let all = commands_only || typed.is_empty() || whole(group);
+            list.group(group, commands, all);
         }
         list
+    }
+
+    /// What follows the `>` of a query that lists the commands alone,
+    /// trimmed (macOS, Windows): the character that quick open's
+    /// `QuickOpenCommands` is spelled as in the keymap, first in the query
+    /// after spaces. None where the query does not begin with it, or the
+    /// look has none (Omarchy).
+    pub fn quick_open_commands_query<'a>(&self, open: &'a QuickOpen) -> Option<&'a str> {
+        use crate::keymap::{Scope, Spelled, Stroke};
+        let keys = self
+            .keymap
+            .chords_now(
+                self.layout(),
+                Command::QuickOpenCommands,
+                Scope::Prompt,
+                true,
+            )
+            .next()?;
+        let Ok(Spelled::Strokes(strokes)) = crate::keymap::spell(keys) else {
+            return None;
+        };
+        let [Stroke::Text(prefix)] = strokes.as_slice() else {
+            return None;
+        };
+        open.query.trim().strip_prefix(*prefix).map(str::trim)
     }
 
     /// `OpenQuickOpen`: quick open for the window in front, searching its
@@ -522,7 +585,38 @@
                 caret: None,
                 scrolled_to: None,
                 scroll_offset: 0.0,
+                picked: None,
             })));
+            self.quick_open_hold();
+        }
+    }
+
+    /// Notes the result the pick is on as the pick: what Enter opens, and
+    /// what the panel shows picked, wherever the list moves it since.
+    fn quick_open_hold(&mut self) {
+        let picked = match &self.dialog {
+            Some(Dialog::QuickOpen(open)) => {
+                let list = self.quick_open_list(open);
+                list.items().nth(open.selected).cloned()
+            }
+            _ => return,
+        };
+        if let Some(open) = self.quick_open_mut() {
+            open.picked = picked;
+        }
+    }
+
+    /// Where the pick is in `list`, `open`'s list as it is now: the result
+    /// it was put on, wherever the list has moved it (a command that came
+    /// or went above it, a list that came in). None where that result is
+    /// no longer listed: then nothing is picked, and Enter opens nothing.
+    pub fn quick_open_picked(&self, open: &QuickOpen, list: &QuickList) -> Option<usize> {
+        match &open.picked {
+            Some(picked) => list.items().position(|item| item.same(picked)),
+            None => {
+                let count = list.items().count();
+                (count > 0).then(|| open.selected.min(count - 1))
+            }
         }
     }
 
@@ -540,6 +634,7 @@
             open.selected = 0;
             open.expanded.clear();
         }
+        self.quick_open_hold();
     }
 
     /// `QuickOpenScope`: this connection, or every open one.
@@ -549,21 +644,32 @@
             open.selected = 0;
             open.expanded.clear();
         }
+        self.quick_open_hold();
     }
 
     /// `QuickOpenMove`: the pick moves by `step`, and stops at the ends.
     pub(super) fn quick_open_move(&mut self, step: isize) {
-        let count = match &self.dialog {
-            Some(Dialog::QuickOpen(open)) => self.quick_open_list(open).items().count(),
+        let (count, shown) = match &self.dialog {
+            Some(Dialog::QuickOpen(open)) => {
+                let list = self.quick_open_list(open);
+                (list.items().count(), self.quick_open_picked(open, &list))
+            }
             _ => return,
         };
         if let Some(open) = self.quick_open_mut() {
-            // From the pick as it shows: clamped, as the list may have
-            // shrunk under it (a relist, a window closed).
-            let last = count.saturating_sub(1);
-            let from = open.selected.min(last) as isize;
-            open.selected = (from + step).clamp(0, last as isize) as usize;
-        }
+            // From the pick as it shows, wherever the list moved it. Where
+            // its result is gone (a relist, a run that ended), it stood
+            // between the result before it and the one now in its place:
+            // up is the one before, down the one in its place.
+            let last = count.saturating_sub(1) as isize;
+            let to = match shown {
+                Some(at) => at as isize + step,
+                None if step > 0 => open.selected as isize + step - 1,
+                None => open.selected as isize + step,
+            };
+            open.selected = to.clamp(0, last) as usize;
+        }
+        self.quick_open_hold();
     }
 
     /// `QuickOpenPick`: opens the picked result, or the one at `index`.
@@ -573,29 +679,32 @@
     /// opens as the start screen opens it. A command does what it does in
     /// the window quick open was opened in. A "more" row shows its group
     /// in full and quick open stays; the pick stays where it was, on the
-    /// first result that row hid. Anything else closes quick open.
+    /// first result that row hid. Anything else closes quick open. The
+    /// pick is the result it was put on, not a place in the list: where
+    /// that result is no longer listed (a Cancel whose run ended), Enter
+    /// opens nothing, and quick open stays.
     pub(super) fn quick_open_pick(&mut self, index: Option<usize>, in_editor: bool) {
-        let (here, item) = match &self.dialog {
+        let (here, at, item) = match &self.dialog {
             Some(Dialog::QuickOpen(open)) => {
                 let list = self.quick_open_list(open);
-                let count = list.items().count();
-                // The pick is clamped: the list may have shrunk since it
-                // moved (a relist under a typed query).
-                let picked = index.unwrap_or(open.selected.min(count.saturating_sub(1)));
-                (open.tab, list.items().nth(picked).cloned())
+                // A click and a digit name a row of the list drawn this
+                // frame; Enter, the result picked.
+                let picked = index.or_else(|| self.quick_open_picked(open, &list));
+                let item = picked.and_then(|at| list.items().nth(at).cloned());
+                (open.tab, picked, item)
             }
             _ => return,
         };
-        let Some(item) = item else {
+        let (Some(at), Some(item)) = (at, item) else {
             return;
         };
         if let QuickItem::More { group, .. } = item {
             if let Some(open) = self.quick_open_mut() {
                 open.expanded.push(group);
-                if let Some(index) = index {
-                    open.selected = index;
-                }
-            }
+                // Where the row stood: the first result it hid comes there.
+                open.selected = at;
+            }
+            self.quick_open_hold();
             return;
         }
         self.dialog = None;
@@ -630,12 +739,16 @@
                     _ => self.apply(Action::OpenRecent { tab, index }),
                 }
             }
-            QuickItem::Command { command, .. } => self.apply(match command {
-                QuickCommand::NewSql { .. } => Action::NewSqlTab(here),
-                QuickCommand::SwitchDatabase => Action::ShowDatabaseMenu(here),
-                QuickCommand::Agents => Action::ShowAgents(here),
-                QuickCommand::History => Action::ShowHistory(here),
-            }),
+            QuickItem::Command { command, .. } => match command {
+                // As its key does it in the window quick open was opened
+                // in: listed now, so it applies now.
+                QuickCommand::Run(runnable) => {
+                    let actions = self.command_actions(here, runnable.command);
+                    for action in actions.into_iter().flatten() {
+                        self.apply(action);
+                    }
+                }
+            },
             QuickItem::More { .. } => {}
         }
     }
diff --git a/src/keymap.rs b/src/keymap.rs
--- a/src/keymap.rs
+++ b/src/keymap.rs
@@ -553,6 +553,7 @@
     QuickOpenPick,
     QuickOpenPickInEditor,
     QuickOpenScope,
+    QuickOpenCommands,
     QuickOpenNumber,
     QuickOpenClose,
     NextLabel,
@@ -919,6 +920,7 @@
             C::QuickOpenPick => info("Open the one selected", "", Prompts),
             C::QuickOpenPickInEditor => info("Open the one selected in a SQL editor", "", Prompts),
             C::QuickOpenScope => info("Search this connection or every open one", "", Prompts),
+            C::QuickOpenCommands => info("List the commands alone", "", Prompts),
             C::QuickOpenNumber => info("Open a recent by its number", "", Prompts),
             C::QuickOpenClose => info("Close quick open", "", Prompts),
             C::NextLabel => info("Next line of the label chooser", "", Prompts),
@@ -2038,6 +2040,10 @@
             keys(&[k("tab")]),
         )
         .when(When::QuickOpen),
+        // Typed first, it lists the commands alone: what quick open reads
+        // from its field's text (macOS, Windows). Omarchy's commands stay
+        // on `:`.
+        bind(C::QuickOpenCommands, PROMPT, keys(&[k(">")]), NONE).when(When::QuickOpen),
         bind(C::QuickOpenNumber, PROMPT, NONE, keys(&[k("1..9")])).when(When::QuickOpen),
         bind(
             C::QuickOpenClose,
diff --git a/src/model.rs b/src/model.rs
--- a/src/model.rs
+++ b/src/model.rs
@@ -2650,6 +2650,10 @@
     pub scrolled_to: Option<usize>,
     /// The list's scroll offset, in points.
     pub scroll_offset: f32,
+    /// The result the pick was put on (by a move, a scope, what was
+    /// typed): what Enter opens, wherever the list, which can change by
+    /// itself (a run that ends, a list that comes in), has moved it.
+    pub picked: Option<crate::app::QuickItem>,
 }
 
 /// Where quick open moves its field's caret.
@@ -2671,6 +2675,8 @@
     Views,
     /// The objects of the window `ConnTabId`, of every kind.
     Connection(ConnTabId),
+    /// The commands whose names match.
+    Commands,
 }
 
 /// The Settings window while it is open. The settings themselves are the
diff --git a/src/ui/quick_open.rs b/src/ui/quick_open.rs
--- a/src/ui/quick_open.rs
+++ b/src/ui/quick_open.rs
@@ -1,7 +1,8 @@
 //! Quick open (`⌘P`, Omarchy `ctrl+p`): a panel over the window that finds
 //! a table or view of the window's connection by name, or of every open
-//! connection (`⇥`, `tab`), and in the macOS and Windows looks starts a
-//! few commands. What it lists is [`App::quick_open_list`]'s; this draws
+//! connection (`⇥`, `tab`), and in the macOS and Windows looks runs a
+//! command (`>` first lists the commands alone). What it lists is
+//! [`App::quick_open_list`]'s; this draws
 //! it and reads its keys, which are its own while it is open (the window's
 //! handler does not run under a dialog). macOS: a floating panel 640 wide,
 //! 84 from the window's top, the window behind it not dimmed. Omarchy: the
@@ -15,7 +16,8 @@
 use tabletist_db::ObjectKind;
 
 use crate::app::{
-    App, Found, QuickCommand, QuickHeading, QuickItem, QuickList, QuickRow, StartRow,
+    About, App, Found, QuickCommand, QuickHeading, QuickItem, QuickList, QuickRow, StartRow,
+    TabName,
 };
 use crate::env::{Environment, Platform, env_colors};
 use crate::i18n::{Locale, gettext};
@@ -79,6 +81,7 @@
     open: Written,
     editor: Written,
     scope: Written,
+    commands: Written,
     number: Written,
     close: Written,
 }
@@ -90,6 +93,7 @@
         open: key(Command::QuickOpenPick),
         editor: key(Command::QuickOpenPickInEditor),
         scope: key(Command::QuickOpenScope),
+        commands: key(Command::QuickOpenCommands),
         number: key(Command::QuickOpenNumber),
         close: key(Command::QuickOpenClose),
     }
@@ -148,6 +152,9 @@
         return;
     }
     let list = app.quick_open_list(open);
+    // The result picked, wherever the list has moved it: none where it
+    // is gone.
+    let picked = app.quick_open_picked(open, &list);
     let layout = app.layout();
     let keymap = app.keymap.clone();
     let keys = keys(&keymap, layout);
@@ -174,15 +181,23 @@
         };
         look.label(&said)
     });
-    let command_keys: HashMap<Command, Written> = [
-        Command::NewSqlTab,
-        Command::SwitchDatabase,
-        Command::ShowAgents,
-        Command::History,
-    ]
-    .into_iter()
-    .map(|command| (command, keymap.key_in(layout, command, Scope::Global)))
-    .collect();
+    // Each command's first key where a connection's window reads it: the
+    // whole window's, then the panes' (Format's is the SQL editor's). A key
+    // only Connections reads (Edit connection's) is none here, and so is
+    // Reload's: on a table's rows it reloads them, not the schema.
+    let window_scopes = [Scope::Global, Scope::Grid, Scope::Sidebar, Scope::SqlEditor];
+    let command_keys: HashMap<Command, Written> = crate::app::RUNNABLE
+        .into_iter()
+        .filter(|command| *command != Command::Reload)
+        .map(|command| {
+            let key = window_scopes
+                .iter()
+                .map(|scope| keymap.key_in(layout, command, *scope))
+                .find(|key| !key.as_str().is_empty())
+                .unwrap_or_default();
+            (command, key)
+        })
+        .collect();
     let skin = Skin {
         look: &look,
         palette: &palette,
@@ -202,6 +217,8 @@
     // Nothing typed but spaces lists what nothing typed lists, numbers
     // and all.
     let nothing_typed = open.query.trim().is_empty();
+    // What follows the `>` of the commands alone.
+    let commands_query = app.quick_open_commands_query(open).map(str::to_owned);
     ctx.input_mut(|input| read_keys(input, &keymap, layout, nothing_typed, &mut actions));
     let Some(Dialog::QuickOpen(open)) = &mut app.dialog else {
         return;
@@ -231,11 +248,11 @@
             if changed {
                 actions.push(Action::QuickOpenChanged);
             }
-            let selected = open.selected.min(list.items().count().saturating_sub(1));
+            let selected = picked.unwrap_or(usize::MAX);
             if list.is_empty() {
                 no_match(
                     ui,
-                    (&open.query, open.everywhere),
+                    (&open.query, open.everywhere, commands_query.as_deref()),
                     &keys,
                     &skin,
                     &mut actions,
@@ -675,32 +692,27 @@
             }
         }
         QuickItem::Command { command, spans } => {
-            let name = gettext(skin.locale, command.name()).into_owned();
             let key = skin
                 .command_keys
                 .get(&command.command())
                 .map(|key| key.as_str().to_owned())
                 .unwrap_or_default();
-            let (icon, detail) = match command {
-                QuickCommand::NewSql { read_only } => {
-                    let detail = if *read_only {
-                        skin.say("opens read-only")
-                    } else {
-                        String::new()
+            // A screen reader hears what it is.
+            let (name, icon, detail, what) = match command {
+                QuickCommand::Run(runnable) => {
+                    let name = gettext(skin.locale, runnable.name).into_owned();
+                    let icon = match runnable.command {
+                        Command::NewSqlTab => Icon::Code,
+                        Command::SwitchDatabase => Icon::Database,
+                        Command::History => Icon::Clock,
+                        Command::Reload => Icon::RefreshCw,
+                        Command::Settings => Icon::Settings,
+                        _ => Icon::SquareTerminal,
                     };
-                    (Icon::Code, detail)
+                    (name, icon, about(&runnable.about, skin), "command")
                 }
-                QuickCommand::SwitchDatabase => (Icon::Database, String::new()),
-                QuickCommand::Agents => {
-                    let mut detail = skin.window_name(skin.here);
-                    if let Some(said) = &skin.agents {
-                        detail = format!("{detail} · {said}");
-                    }
-                    (Icon::SquareTerminal, detail)
-                }
-                QuickCommand::History => (Icon::Clock, skin.window_name(skin.here)),
             };
-            let mut label = format!("{name}, {}", skin.say("command"));
+            let mut label = format!("{name}, {}", skin.say(what));
             for words in [&detail, &key] {
                 if !words.is_empty() {
                     label = format!("{label}, {words}");
@@ -722,6 +734,8 @@
                 QuickGroup::Tables => format!("{hidden} {}", skin.say("more tables")),
                 QuickGroup::Views if *hidden == 1 => skin.say("1 more view"),
                 QuickGroup::Views => format!("{hidden} {}", skin.say("more views")),
+                QuickGroup::Commands if *hidden == 1 => skin.say("1 more command"),
+                QuickGroup::Commands => format!("{hidden} {}", skin.say("more commands")),
                 QuickGroup::Connection(tab) => format!(
                     "{hidden} {} {}",
                     skin.say("more in"),
@@ -738,6 +752,26 @@
                 more: true,
             }
         }
+    }
+}
+
+/// What a command's row says after its name: where it acts.
+fn about(about: &About, skin: &Skin<'_>) -> String {
+    match about {
+        About::Nothing => String::new(),
+        About::OpensReadOnly => skin.say("opens read-only"),
+        About::Connection => skin.window_name(skin.here),
+        About::Agents => {
+            let mut detail = skin.window_name(skin.here);
+            if let Some(said) = &skin.agents {
+                detail = format!("{detail} · {said}");
+            }
+            detail
+        }
+        About::Tab(TabName::Object(name)) => display_safe(name).into_owned(),
+        About::Tab(TabName::Query(number)) => format!("{} {number}", skin.say("Query")),
+        About::Tab(TabName::Agents) => skin.say("Agents"),
+        About::Tab(TabName::History) => skin.say("History"),
     }
 }
 
@@ -1027,16 +1061,28 @@
 }
 
 /// What the panel says when nothing matches: where it looked, and in this
-/// connection's scope a way to look in every open one.
+/// connection's scope a way to look in every open one. Where the commands
+/// alone are listed (`>`, `commands_query` what follows it), that no
+/// command matches.
 fn no_match(
     ui: &mut egui::Ui,
-    (query, everywhere): (&str, bool),
+    (query, everywhere, commands_query): (&str, bool, Option<&str>),
     keys: &Keys,
     skin: &Skin<'_>,
     actions: &mut Vec<Action>,
 ) {
     let (look, palette, locale) = (skin.look, skin.palette, skin.locale);
     let query = query.trim();
+    if let Some(query) = commands_query {
+        let said =
+            gettext(locale, "No commands match “{query}”").replace("{query}", &display_safe(query));
+        let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 96.0), Sense::hover());
+        let laid = Text::one(look, TextRole::UiBody, &said, palette.secondary).layout(ui.ctx());
+        let y = rect.top() + 36.0;
+        laid.paint_left(ui.painter(), rect.center().x - laid.width() / 2.0, y);
+        widgets::announce(ui, rect, &said);
+        return;
+    }
     let place = if everywhere {
         gettext(locale, "open connections").into_owned()
     } else if look.terminal {
@@ -1088,9 +1134,9 @@
 }
 
 /// The key line under the list. macOS: `↑↓ move`, `↩ open`, `⌥↩ in SQL
-/// editor`, `⇥` and the scope it switches to, `esc close` at the right, on
-/// the sunken surface. Omarchy: the same in its words, `1..9 pick` only
-/// while nothing is typed.
+/// editor`, `> commands`, `⇥` and the scope it switches to, `esc close` at
+/// the right, on the sunken surface. Omarchy: the same in its words, but
+/// no `>`, and `1..9 pick` only while nothing is typed.
 fn footer(ui: &mut egui::Ui, everywhere: bool, nothing_typed: bool, keys: &Keys, skin: &Skin<'_>) {
     let (look, palette, locale) = (skin.look, skin.palette, skin.locale);
     let height = if look.terminal { 30.0 } else { 34.0 };
@@ -1110,10 +1156,11 @@
     } else {
         word("All connections")
     };
-    let (moves, open, editor, pick, close) = (
+    let (moves, open, editor, commands, pick, close) = (
         word("move"),
         word("open"),
         word("in SQL editor"),
+        word("commands"),
         word("pick"),
         word("close"),
     );
@@ -1121,6 +1168,9 @@
         vec![(&keys.moves, &moves, true), (&keys.open, &open, true)];
     if !keys.editor.as_str().is_empty() {
         hints.push((&keys.editor, &editor, true));
+    }
+    if !keys.commands.as_str().is_empty() {
+        hints.push((&keys.commands, &commands, true));
     }
     if look.terminal && nothing_typed && !keys.number.as_str().is_empty() {
         hints.push((&keys.number, &pick, true));
````

- [ ] **Step 4: Run the checks**

Run: `cargo fmt --all --check && cargo clippy --locked --workspace --all-targets -- -D warnings && cargo test --locked --workspace --all-targets && RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps`

Expected: all pass. The lib suite:

```
fmt exit 0
clippy exit 0
test result: ok. 3049 passed; 0 failed; 20 ignored; 0 measured; 0 filtered out
doc exit 0
```

- [ ] **Step 5: Commit**

```bash
git add src/app/commands.rs src/app.rs src/app/quick_open.rs src/model.rs src/keymap.rs src/ui/quick_open.rs src/ui/quick_open_tests.rs src/ui/agents_tests.rs src/ui/history_tests.rs
git commit -S -m "Run the keymap's commands from quick open, and list them alone after >"
```

### Task 2: A setting's row opens Settings at it

Each row of the Settings sheet is a result: picking it opens Settings at that row, grounded, scrolled to and with its control given the keyboard.

**Files:** Modify `src/model.rs`, `src/app.rs`, `src/app/quick_open.rs`, `src/ui/quick_open.rs`, `src/ui/quick_open_tests.rs`, `src/ui/settings/mod.rs`, `src/ui/settings/sheet.rs`, `src/ui/settings/agents.rs`.

- [ ] **Step 1: Write the failing tests**

The brief's test 6 (its setting), and a row below the fold of a short window (under the footer when Settings is opened plainly, in view when quick open opens it there).

````diff
diff --git a/src/ui/quick_open_tests.rs b/src/ui/quick_open_tests.rs
--- a/src/ui/quick_open_tests.rs
+++ b/src/ui/quick_open_tests.rs
@@ -1059,6 +1059,81 @@
     assert!(!shown.contains(&"reviews".to_owned()), "{shown:?}");
 }
 
+/// Test 6, its settings: a setting's row opens Settings at that row: its
+/// pane, the row on the selection's ground, its switch with the keyboard.
+#[test]
+fn a_setting_opens_settings_at_its_row() {
+    let look = Look::macos();
+    let (mut harness, _tab) = bookshop(look);
+    ask(&mut harness, look, "> re");
+    let restore = "> Settings › General › Restore tabs and filters";
+    let index = result_named(&harness, restore);
+    harness.app.apply(Action::QuickOpenPick {
+        index: Some(index),
+        in_editor: false,
+    });
+    harness.settle();
+    let Some(Dialog::Settings(dialog)) = &harness.app.dialog else {
+        panic!("Settings is not open");
+    };
+    assert_eq!(dialog.pane, crate::model::SettingsPane::General);
+    assert_eq!(focused(&mut harness), "Restore tabs and filters");
+    let grounded = |harness: &Harness| {
+        let label = harness
+            .painted_rect("Restore tabs and filters")
+            .expect("the row's label");
+        let selection = harness.app.palette.selection;
+        harness
+            .fills
+            .iter()
+            .any(|(rect, color)| *color == selection && rect.contains_rect(label))
+    };
+    assert!(grounded(&harness), "the row is on the selection's ground");
+    // The keyboard stays where it was put: it is given once.
+    harness.press(Key::Tab, Modifiers::NONE);
+    assert_ne!(focused(&mut harness), "Restore tabs and filters");
+    // Another pane leaves the row behind.
+    for pane in [
+        crate::model::SettingsPane::Appearance,
+        crate::model::SettingsPane::General,
+    ] {
+        harness.app.apply(Action::ShowSettingsPane(pane));
+        harness.settle();
+    }
+    assert!(!grounded(&harness));
+}
+
+/// A setting's row that the sheet shows only scrolled, in a short window,
+/// is scrolled to.
+#[test]
+fn a_setting_below_the_fold_is_scrolled_to() {
+    let look = Look::macos();
+    let mut harness = Harness::with_size(egui::vec2(1280.0, 560.0));
+    harness.set_look(look);
+    harness.connect_with(bookshop_dev(), bookshop_objects());
+    // Opened plainly, the sheet shows the row under its footer's line.
+    harness.app.apply(Action::ShowSettings);
+    harness.settle();
+    let row = harness.painted_rect("Disconnect when idle");
+    let footer = harness.painted_rect("Stored in").expect("the footer");
+    assert!(row.is_none_or(|row| row.top() > footer.top()), "{row:?}");
+    harness.app.apply(Action::CloseDialog);
+    harness.settle();
+    ask(&mut harness, look, "> idle");
+    let index = result_named(&harness, "> Settings › General › Disconnect when idle");
+    harness.app.apply(Action::QuickOpenPick {
+        index: Some(index),
+        in_editor: false,
+    });
+    harness.settle();
+    let row = harness
+        .painted_rect("Disconnect when idle")
+        .expect("the row's label");
+    let footer = harness.painted_rect("Stored in").expect("the footer");
+    assert!(row.bottom() < footer.top(), "{row:?} under {footer:?}");
+    assert_eq!(focused(&mut harness), "Disconnect when idle");
+}
+
 /// The commands quick open runs are the keymap's, built, each once; and
 /// each listed does what it names, here a few that change what shows.
 #[test]
````

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --locked -p tabletist --lib -- quick_open_tests::a_setting`

Expected: both fail, as no setting is listed yet:

```
test ui::quick_open_tests::a_setting_opens_settings_at_its_row ... FAILED
test ui::quick_open_tests::a_setting_below_the_fold_is_scrolled_to ... FAILED
thread 'ui::quick_open_tests::a_setting_opens_settings_at_its_row' panicked at src/ui/quick_open_tests.rs:985:28:
> Settings › General › Restore tabs and filters is not listed: ["> Reload schema"]
thread 'ui::quick_open_tests::a_setting_below_the_fold_is_scrolled_to' panicked at src/ui/quick_open_tests.rs:985:28:
> Settings › General › Disconnect when idle is not listed: []
test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 3069 filtered out
error: test failed, to rerun pass `-p tabletist --lib`
```

- [ ] **Step 3: Write the code**

````diff
diff --git a/src/app.rs b/src/app.rs
--- a/src/app.rs
+++ b/src/app.rs
@@ -39,6 +39,7 @@
 pub use commands::{About, RUNNABLE, Runnable, TabName};
 pub use quick_open::{
     Found, GROUP_SHOWN, QuickCommand, QuickHeading, QuickItem, QuickList, QuickRow, StartRow,
+    setting_path,
 };
 
 /// What a keyring read is for; each names the exact request it serves, so
@@ -2257,6 +2258,39 @@
                     // The question was about the other pane's options.
                     dialog.resetting = false;
                     dialog.history_question = None;
+                    // The row quick open asked for is the other pane's.
+                    dialog.asked = None;
+                    dialog.reveal = false;
+                }
+            }
+            Action::ShowSetting(option) => {
+                let pane = crate::model::SettingsPane::of(option);
+                if !crate::model::SettingsPane::listed().contains(&pane) {
+                    return;
+                }
+                if matches!(self.dialog, None | Some(Dialog::Help)) {
+                    self.dialog = Some(Dialog::Settings(Box::default()));
+                    self.list_fonts();
+                }
+                if let Some(Dialog::Settings(dialog)) = &mut self.dialog {
+                    let row = crate::model::SettingsRow::Option(option);
+                    // The terminal look's cursor, where its screen has the
+                    // row (it has none for some of the sheet's).
+                    if dialog.pane != pane {
+                        dialog.pane = pane;
+                        dialog.row = 0;
+                    }
+                    if let Some(at) = pane.rows().iter().position(|at| *at == row) {
+                        dialog.row = at;
+                    }
+                    dialog.resetting = false;
+                    dialog.asked = Some(option);
+                    dialog.reveal = true;
+                }
+            }
+            Action::SettingShown => {
+                if let Some(Dialog::Settings(dialog)) = &mut self.dialog {
+                    dialog.reveal = false;
                 }
             }
             Action::ChooseAgentSetup(agent) => {
diff --git a/src/app/quick_open.rs b/src/app/quick_open.rs
--- a/src/app/quick_open.rs
+++ b/src/app/quick_open.rs
@@ -13,8 +13,9 @@
 use super::App;
 use super::commands::Runnable;
 use crate::keymap::Command;
-use crate::model::{Action, ConnTabId, Dialog, QuickGroup, QuickOpen, StartRecent};
+use crate::model::{Action, ConnTabId, Dialog, QuickGroup, QuickOpen, SettingsPane, StartRecent};
 use crate::search::{self, Tier};
+use crate::settings::OptionId;
 
 /// How many results a group shows before its "more" row (macOS).
 pub const GROUP_SHOWN: usize = 3;
@@ -150,13 +151,17 @@
 pub enum QuickCommand {
     /// A command of the keymap's, run as its key runs it.
     Run(Runnable),
+    /// A row of the Settings window: the window opens at it.
+    Setting(OptionId),
 }
 
 impl QuickCommand {
-    /// The keymap's command it is, whose key the row writes.
+    /// The keymap's command it is, whose key the row writes: a setting's
+    /// is the one that opens Settings.
     pub fn command(&self) -> Command {
         match self {
             Self::Run(runnable) => runnable.command,
+            Self::Setting(_) => Command::Settings,
         }
     }
 }
@@ -185,7 +190,8 @@
     /// Whether `other` is this result, though it may be listed elsewhere
     /// or say something else of itself now (an estimate read again, a
     /// command's name): the same object of the same window, the same row
-    /// of a start screen, the same command, the same group's "more" row.
+    /// of a start screen, the same command or setting, the same group's
+    /// "more" row.
     pub fn same(&self, other: &QuickItem) -> bool {
         match (self, other) {
             (
@@ -216,8 +222,11 @@
                     }
             }
             (Self::Command { command, .. }, Self::Command { command: other, .. }) => {
-                let (QuickCommand::Run(one), QuickCommand::Run(two)) = (command, other);
-                one.command == two.command
+                match (command, other) {
+                    (QuickCommand::Run(one), QuickCommand::Run(two)) => one.command == two.command,
+                    (QuickCommand::Setting(one), QuickCommand::Setting(two)) => one == two,
+                    _ => false,
+                }
             }
             (Self::More { group, .. }, Self::More { group: other, .. }) => group == other,
             _ => false,
@@ -282,10 +291,22 @@
     }
 }
 
-/// `command`'s name, translated.
-fn name_of(locale: crate::i18n::Locale, command: &QuickCommand) -> String {
+/// A setting's row as quick open names it: the path to it in the Settings
+/// window, `Settings › General › Restore tabs and filters`, translated.
+/// `run` is the run key as the window writes it, for `{run} runs`.
+pub fn setting_path(locale: crate::i18n::Locale, run: &str, option: OptionId) -> String {
+    let say = |text| crate::i18n::gettext(locale, text);
+    let pane = SettingsPane::of(option);
+    let label = say(crate::ui::settings::label(option)).replace("{run}", run);
+    format!("{} › {} › {label}", say("Settings"), say(pane.name()))
+}
+
+/// `command`'s name, translated; `run` is the run key, as for
+/// [`setting_path`].
+fn name_of(locale: crate::i18n::Locale, run: &str, command: &QuickCommand) -> String {
     match command {
         QuickCommand::Run(runnable) => crate::i18n::gettext(locale, runnable.name).into_owned(),
+        QuickCommand::Setting(option) => setting_path(locale, run, *option),
     }
 }
 
@@ -415,23 +436,26 @@
         windows
     }
 
-    /// A command's name, as its row writes it and what is typed matches.
+    /// A command's name, as its row writes it and what is typed matches:
+    /// a setting's is the path to its row, `Settings › General › Restore
+    /// tabs and filters`.
     pub fn command_name(&self, command: &QuickCommand) -> String {
-        name_of(self.locale, command)
+        let run = self.keymap.key(self.layout(), Command::RunStatement);
+        name_of(self.locale, run.as_str(), command)
     }
 
     /// The commands `typed` (folded) finds in `tab`'s window, best first,
-    /// in the registry's order within a tier: each with the runs of its
-    /// name that matched. With nothing typed, every command
-    /// (`commands_only`), or the suggested ones. None in the terminal
-    /// look.
+    /// in the registry's order within a tier and then the settings' rows in
+    /// the window's: each with the runs of its name that matched. With
+    /// nothing typed, the commands alone (`commands_only`), or the
+    /// suggested ones. None in the terminal look.
     fn commands_found(
         &self,
         tab: ConnTabId,
         typed: &[char],
         commands_only: bool,
     ) -> Vec<QuickItem> {
-        let commands: Vec<QuickCommand> = self
+        let mut commands: Vec<QuickCommand> = self
             .runnable(tab)
             .into_iter()
             .filter(|runnable| {
@@ -439,6 +463,16 @@
             })
             .map(QuickCommand::Run)
             .collect();
+        // The settings' rows answer a word: with nothing typed they would
+        // bury the commands.
+        if !typed.is_empty() && !self.look.terminal {
+            let rows = SettingsPane::listed()
+                .iter()
+                .flat_map(|pane| pane.sheet_options());
+            commands.extend(rows.map(QuickCommand::Setting));
+        }
+        // Asked once: a setting's row may name it.
+        let run = self.keymap.key(self.layout(), Command::RunStatement);
         let mut found: Vec<(Tier, QuickItem)> = commands
             .into_iter()
             .filter_map(|command| {
@@ -446,7 +480,7 @@
                     let spans = Vec::new();
                     return Some((Tier::Exact, QuickItem::Command { command, spans }));
                 }
-                let hit = search::find(typed, &name_of(self.locale, &command))?;
+                let hit = search::find(typed, &name_of(self.locale, run.as_str(), &command))?;
                 let spans = hit.spans;
                 Some((hit.tier, QuickItem::Command { command, spans }))
             })
@@ -461,7 +495,7 @@
     /// open one is searched), then the suggested commands. With a query:
     /// the tables and then the views that match, or where every open
     /// connection is searched, each connection's matches under it; then
-    /// the commands whose names match. In the macOS
+    /// the commands and the settings' rows whose names match. In the macOS
     /// and Windows looks each group shows its first three and a "more" row
     /// until that is opened, and a query that begins with `>` lists every
     /// command that matches what follows it, and nothing else; the
@@ -748,6 +782,7 @@
                         self.apply(action);
                     }
                 }
+                QuickCommand::Setting(option) => self.apply(Action::ShowSetting(option)),
             },
             QuickItem::More { .. } => {}
         }
diff --git a/src/model.rs b/src/model.rs
--- a/src/model.rs
+++ b/src/model.rs
@@ -930,6 +930,14 @@
     /// The answer to the history's question in Settings: clear it, or keep
     /// it.
     AnswerHistoryQuestion(bool),
+    /// Show the Settings window at this option: its pane, the sheet's row
+    /// on the selection's ground, scrolled to, its control with the
+    /// keyboard (the terminal look's cursor on it, where that screen has
+    /// its row). From quick open.
+    ShowSetting(crate::settings::OptionId),
+    /// The sheet has scrolled to the row it was asked for and given its
+    /// control the keyboard.
+    SettingShown,
     /// Show how this agent is connected, in Settings › Agents.
     ChooseAgentSetup(crate::endpoint::Agent),
     /// Close Settings and show the Agents tab on this view: the window's in
@@ -2675,7 +2683,7 @@
     Views,
     /// The objects of the window `ConnTabId`, of every kind.
     Connection(ConnTabId),
-    /// The commands whose names match.
+    /// The commands and the settings' rows whose names match.
     Commands,
 }
 
@@ -2697,6 +2705,13 @@
     pub history_question: Option<HistoryQuestion>,
     /// The agent Settings › Agents says how to connect.
     pub agent: crate::endpoint::Agent,
+    /// The option quick open opened the window at: the sheet draws its row
+    /// on the selection's ground while its pane shows.
+    pub asked: Option<crate::settings::OptionId>,
+    /// That row is still to be scrolled to and its control given the
+    /// keyboard: once, by the sheet's next frame, which says so
+    /// ([`Action::SettingShown`]).
+    pub reveal: bool,
 }
 
 /// What Settings › Editor asks of the query history.
@@ -2738,6 +2753,54 @@
             Self::Appearance => "Appearance",
             Self::Editor => "Editor",
             Self::Agents => "Agents",
+        }
+    }
+
+    /// The pane whose rows hold `option`.
+    pub fn of(option: crate::settings::OptionId) -> Self {
+        [Self::General, Self::Appearance, Self::Editor, Self::Agents]
+            .into_iter()
+            .find(|pane| pane.options().contains(&option))
+            .unwrap_or_default()
+    }
+
+    /// The sections the sheet (macOS, Windows) shows the pane's options
+    /// in, each under its heading (English), in order. The Agents pane
+    /// draws its own, and has none here.
+    pub fn sections(self) -> &'static [(&'static str, &'static [crate::settings::OptionId])] {
+        use crate::settings::OptionId;
+        match self {
+            Self::General => &[
+                ("Startup", &OptionId::STARTUP),
+                ("Data", &OptionId::DATA),
+                ("Safety", &OptionId::SAFETY),
+            ],
+            Self::Appearance => &[
+                ("Theme", &OptionId::THEME),
+                ("Text", &OptionId::TEXT),
+                ("Grid", &OptionId::GRID),
+                ("Environment color", &OptionId::ENVIRONMENT),
+            ],
+            Self::Editor => &[
+                ("Running queries", &OptionId::RUNNING),
+                ("Typing", &OptionId::TYPING),
+                ("History", &OptionId::HISTORY),
+            ],
+            Self::Agents => &[],
+        }
+    }
+
+    /// The options the sheet (macOS, Windows) has a row for, in order:
+    /// what quick open lists of the pane. The Agents pane's switch stands
+    /// by its title, and its Log section has the other two.
+    pub fn sheet_options(self) -> Vec<crate::settings::OptionId> {
+        match self {
+            Self::Agents => crate::settings::OptionId::AGENTS.to_vec(),
+            Self::General | Self::Appearance | Self::Editor => self
+                .sections()
+                .iter()
+                .flat_map(|(_, options)| options.iter().copied())
+                .collect(),
         }
     }
 
diff --git a/src/ui/quick_open.rs b/src/ui/quick_open.rs
--- a/src/ui/quick_open.rs
+++ b/src/ui/quick_open.rs
@@ -1,8 +1,8 @@
 //! Quick open (`⌘P`, Omarchy `ctrl+p`): a panel over the window that finds
 //! a table or view of the window's connection by name, or of every open
 //! connection (`⇥`, `tab`), and in the macOS and Windows looks runs a
-//! command (`>` first lists the commands alone). What it lists is
-//! [`App::quick_open_list`]'s; this draws
+//! command or opens Settings at one of its rows (`>` first lists those
+//! alone). What it lists is [`App::quick_open_list`]'s; this draws
 //! it and reads its keys, which are its own while it is open (the window's
 //! handler does not run under a dialog). macOS: a floating panel 640 wide,
 //! 84 from the window's top, the window behind it not dimmed. Omarchy: the
@@ -116,6 +116,8 @@
     agents: Option<String>,
     /// The commands' keys, as the window reads them.
     command_keys: &'a HashMap<Command, Written>,
+    /// The run key, as the window writes it: a setting's row names it.
+    run_key: Written,
     /// The key that opens the picked result, written on its row (macOS).
     open_key: String,
 }
@@ -184,10 +186,12 @@
     // Each command's first key where a connection's window reads it: the
     // whole window's, then the panes' (Format's is the SQL editor's). A key
     // only Connections reads (Edit connection's) is none here, and so is
-    // Reload's: on a table's rows it reloads them, not the schema.
+    // Reload's: on a table's rows it reloads them, not the schema. A
+    // setting's row has the key of Settings.
     let window_scopes = [Scope::Global, Scope::Grid, Scope::Sidebar, Scope::SqlEditor];
     let command_keys: HashMap<Command, Written> = crate::app::RUNNABLE
         .into_iter()
+        .chain([Command::Settings])
         .filter(|command| *command != Command::Reload)
         .map(|command| {
             let key = window_scopes
@@ -208,6 +212,7 @@
         here,
         agents,
         command_keys: &command_keys,
+        run_key: keymap.key(layout, Command::RunStatement),
         open_key: keys.open.as_str().to_owned(),
     };
     let mut actions = Vec::new();
@@ -697,7 +702,8 @@
                 .get(&command.command())
                 .map(|key| key.as_str().to_owned())
                 .unwrap_or_default();
-            // A screen reader hears what it is.
+            // A screen reader hears which it is: a setting's row opens
+            // Settings, not the setting.
             let (name, icon, detail, what) = match command {
                 QuickCommand::Run(runnable) => {
                     let name = gettext(skin.locale, runnable.name).into_owned();
@@ -710,6 +716,12 @@
                         _ => Icon::SquareTerminal,
                     };
                     (name, icon, about(&runnable.about, skin), "command")
+                }
+                QuickCommand::Setting(option) => {
+                    let name =
+                        crate::app::setting_path(skin.locale, skin.run_key.as_str(), *option);
+                    let detail = skin.say("jumps to the setting");
+                    (name, Icon::Settings, detail, "setting")
                 }
             };
             let mut label = format!("{name}, {}", skin.say(what));
diff --git a/src/ui/settings/agents.rs b/src/ui/settings/agents.rs
--- a/src/ui/settings/agents.rs
+++ b/src/ui/settings/agents.rs
@@ -15,7 +15,7 @@
 use crate::typography::{Laid, Text, TextRole};
 use crate::ui::widgets::{self, ButtonSpec, Segment};
 
-use super::sheet::{After, Skin, label_column, row};
+use super::sheet::{After, Skin, asked, label_column, row};
 
 /// What a short window leaves of the pane under its title at the least.
 const ROWS_MIN: f32 = 56.0;
@@ -102,6 +102,7 @@
         .wrap(words)
         .layout(ui.ctx());
     let height = name.height() + 2.0 + what.height();
+    let ground = ui.painter().add(egui::Shape::Noop);
     let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
     paint(ui, &name, pos2(rect.left(), rect.top()));
     paint(
@@ -121,6 +122,8 @@
             !on,
         )));
     }
+    // Quick open's row of the switch is the title's.
+    asked(ui, ground, (rect, place), OptionId::AgentsOn, skin, actions);
 }
 
 /// Paints `text` with its top left at `at`, and names it for screen
diff --git a/src/ui/settings/mod.rs b/src/ui/settings/mod.rs
--- a/src/ui/settings/mod.rs
+++ b/src/ui/settings/mod.rs
@@ -25,8 +25,8 @@
 };
 
 /// The label of an option's row. English: the caller translates, and the
-/// terminal look lower-cases.
-pub(super) fn label(option: OptionId) -> &'static str {
+/// terminal look lower-cases. Quick open lists the rows by it.
+pub(crate) fn label(option: OptionId) -> &'static str {
     match option {
         OptionId::OnLaunch => "On launch",
         OptionId::RestoreTabs => "Restore tabs and filters",
diff --git a/src/ui/settings/sheet.rs b/src/ui/settings/sheet.rs
--- a/src/ui/settings/sheet.rs
+++ b/src/ui/settings/sheet.rs
@@ -95,6 +95,10 @@
     pub(super) connections: &'a [crate::connections::SavedConnection],
     /// The data fonts beyond the bundled faces, on macOS.
     pub(super) fonts: DataFonts<'a>,
+    /// The option quick open opened the window at, whose row is on the
+    /// selection's ground, and whether it is still to be scrolled to and
+    /// given the keyboard.
+    pub(super) asked: Option<(OptionId, bool)>,
 }
 
 /// The installed fonts macOS's data font menu lists after the bundled
@@ -135,6 +139,7 @@
             installed: &app.installed_fonts,
             missing: app.data_font_missing(),
         },
+        asked: dialog.asked.map(|option| (option, dialog.reveal)),
     };
     let screen = ctx.content_rect();
     let width = WIDTH.min(screen.width() - 2.0 * MARGIN).max(NAV + 320.0);
@@ -192,15 +197,7 @@
                 SettingsPane::General => content(
                     &mut pane,
                     ("General", Some(GENERAL)),
-                    (
-                        &[
-                            ("Startup", &OptionId::STARTUP),
-                            ("Data", &OptionId::DATA),
-                            ("Safety", &OptionId::SAFETY),
-                        ],
-                        false,
-                        None,
-                    ),
+                    (SettingsPane::General.sections(), false, None),
                     // As the window shows them: what the look has where the
                     // settings do not say (the grouping of numbers).
                     &super::shown(app),
@@ -211,16 +208,7 @@
                 SettingsPane::Appearance => content(
                     &mut pane,
                     ("Appearance", Some(APPEARANCE)),
-                    (
-                        &[
-                            ("Theme", &OptionId::THEME),
-                            ("Text", &OptionId::TEXT),
-                            ("Grid", &OptionId::GRID),
-                            ("Environment color", &OptionId::ENVIRONMENT),
-                        ],
-                        true,
-                        None,
-                    ),
+                    (SettingsPane::Appearance.sections(), true, None),
                     &super::shown(app),
                     limit,
                     &skin,
@@ -235,11 +223,7 @@
                         ),
                     ),
                     (
-                        &[
-                            ("Running queries", &OptionId::RUNNING),
-                            ("Typing", &OptionId::TYPING),
-                            ("History", &OptionId::HISTORY),
-                        ],
+                        SettingsPane::Editor.sections(),
                         false,
                         // Show History, which shows the window's tab.
                         Some((
@@ -497,6 +481,7 @@
         TILE_PAD + TILE.y + 6.0 + line + TILE_PAD,
     );
     let height = 6.0 + tile.y + 12.0 + 1.0;
+    let ground = ui.painter().add(egui::Shape::Noop);
     let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
     widgets::hline(ui, rect.x_range(), rect.bottom() - 0.5, palette.surface);
     // The tiles in a Ui of their own, whose node is their parent in the
@@ -611,6 +596,8 @@
     if at + hint.width() <= rect.right() {
         write(ui, at, top + tile.y / 2.0, &hint);
     }
+    let row = rect.with_max_y(rect.bottom() - 1.0);
+    asked(ui, ground, (row, place), OptionId::Theme, skin, actions);
 }
 
 /// How wide the rows' labels have it: `LABEL`, or what the widest small
@@ -692,6 +679,7 @@
     let words = name.height() + print.as_ref().map_or(0.0, |print| 2.0 + print.height());
     let inner = words.max(control_height(option, look));
     let height = ROW_PAD + inner + ROW_PAD + 1.0;
+    let ground = ui.painter().add(egui::Shape::Noop);
     let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
     if ruled {
         widgets::hline(ui, rect.x_range(), rect.bottom() - 0.5, palette.surface);
@@ -737,6 +725,56 @@
     }
     let left = line.min_rect().right() + HINT_GAP;
     hint(ui, pos2(left, middle), rect.right(), option, settings, skin);
+    let row = rect.with_max_y(rect.bottom() - 1.0);
+    asked(ui, ground, (row, place), option, skin, actions);
+}
+
+/// Where `option` is the one quick open opened the window at: its row
+/// (`rect`) on the selection's ground, painted under what the row drew
+/// through `ground`. Once, as the window opens, the row is scrolled to the
+/// middle of what shows, and the first control in `place` takes the
+/// keyboard.
+pub(super) fn asked(
+    ui: &Ui,
+    ground: egui::layers::ShapeIdx,
+    (rect, place): (Rect, Rect),
+    option: OptionId,
+    skin: &Skin,
+    actions: &mut Vec<Action>,
+) {
+    let Some((asked, reveal)) = skin.asked else {
+        return;
+    };
+    if asked != option {
+        return;
+    }
+    let fill = egui::epaint::RectShape::filled(rect, CornerRadius::same(6), skin.palette.selection);
+    ui.painter().set(ground, fill);
+    if !reveal {
+        return;
+    }
+    let center = Some(egui::Align::Center);
+    ui.scroll_to_rect_animation(rect, center, egui::style::ScrollAnimation::none());
+    // The keyboard comes to the sheet once it is the dialog on top: egui
+    // keeps quick open's layer a frame after it closes, and a control
+    // under it takes no keyboard.
+    let layer = ui.layer_id();
+    if ui.ctx().memory(|memory| memory.top_modal_layer()) != Some(layer) {
+        ui.ctx().request_repaint();
+        return;
+    }
+    // The controls are drawn by now: the first that takes the keyboard.
+    let first = ui.ctx().viewport(|viewport| {
+        let widgets = viewport.this_pass.widgets.get_layer(layer);
+        widgets
+            .filter(|widget| widget.enabled && widget.sense.is_focusable())
+            .find(|widget| place.contains(widget.rect.center()))
+            .map(|widget| widget.id)
+    });
+    if let Some(id) = first {
+        ui.memory_mut(|memory| memory.request_focus(id));
+    }
+    actions.push(Action::SettingShown);
 }
 
 /// The data font the settings name, on its row, where it is no face the
````

- [ ] **Step 4: Run the checks**

Run: the four checks of task 1's Step 4.

Expected: all pass. The lib suite:

```
fmt exit 0
clippy exit 0
test result: ok. 3051 passed; 0 failed; 20 ignored; 0 measured; 0 filtered out
doc exit 0
```

- [ ] **Step 5: Commit**

```bash
git add src/model.rs src/app.rs src/app/quick_open.rs src/ui/quick_open.rs src/ui/quick_open_tests.rs src/ui/settings/mod.rs src/ui/settings/sheet.rs src/ui/settings/agents.rs
git commit -S -m "Open Settings at a setting's row from quick open"
```

### Task 3: View › Quick Open, and the website

On macOS the View menu holds Quick Open beside History. The website describes the commands, `>`, the settings' rows and the menu.

**Files:** Modify `src/macos.rs`, `src/entrypoint.rs`, `docs/_guide/macos.md`, `docs/_guide/browsing-data.md`, `docs/_reference/keyboard-shortcuts.md`.

Nothing here can be tested on Linux: the item is AppKit's, and main's `view_menu` test (`crates/tabletist-appkit`) already covers `ViewItem` on CI's macOS runner.

- [ ] **Step 1: Write the code**

````diff
diff --git a/docs/_guide/browsing-data.md b/docs/_guide/browsing-data.md
--- a/docs/_guide/browsing-data.md
+++ b/docs/_guide/browsing-data.md
@@ -14,7 +14,9 @@
 Quick open (Cmd+P on macOS, Ctrl+P on Windows and Omarchy) finds any
 table or view by name without the sidebar, in this connection or in every
 open one. On macOS and Windows the sidebar's "Find any object…" field
-opens it; in the Omarchy look `/` in the sidebar filters its list.
+opens it, and `>` typed first lists the commands instead (and, with a
+word after it, the settings); in the Omarchy look `/` in the sidebar
+filters its list.
 
 System schemas such as `pg_catalog` and `information_schema` are hidden.
 [A setting]({% link _reference/settings-and-files.md %}#the-settings-file)
diff --git a/docs/_guide/macos.md b/docs/_guide/macos.md
--- a/docs/_guide/macos.md
+++ b/docs/_guide/macos.md
@@ -46,7 +46,8 @@
 - **The app menu has the usual two.** **Tabletist → About Tabletist**
   shows the version and the licenses of the fonts it bundles, and
   **Tabletist → Settings…** opens Settings. **View → History** shows the
-  window's [History tab]({% link _guide/sql-editor.md %}#history).
+  window's [History tab]({% link _guide/sql-editor.md %}#history), and
+  **View → Quick Open** opens quick open.
 - **The sidebar's SQL Editor button has a clock.** Its main part opens a
   new SQL editor; the clock at its right shows the History tab, and looks
   pressed while that tab is in front.
@@ -92,16 +93,26 @@
 
 Quick open floats over the window, from anywhere in it: the rows, the
 SQL editor, the start screen or the Agents tab. The sidebar's "Find any
-object…" field opens it too. With nothing typed it lists what the start
-screen lists (what you opened lately), then three commands: New SQL
-query, Switch database (where the server has another) and Agents. Typing
-lists the tables and then the views that match, three of each and a row
-that shows the rest, and the commands whose names match. The best
-matches come first: the whole name, then its start, then the starts of
-its words (`bc` finds `book_covers`), then any run of its letters; what
-you opened lately comes first among equals. Nothing is asked of the
+object…" field and View › Quick Open in the menu bar open it too. With
+nothing typed it lists what the start screen lists (what you opened
+lately), then four commands: New SQL query, Switch database (where the
+server has another), Agents and History. Typing lists the tables and
+then the views that match, three of each and a row that shows the rest,
+then the commands and the settings whose names match, three and a row
+that shows the rest. The best matches come first: the whole name, then
+its start, then the starts of its words (`bc` finds `book_covers`), then
+any run of its letters; what you opened lately comes first among equals. Nothing is asked of the
 database while you type. The chip beside the field says what is
 searched; Tab searches every open connection, each under its name.
+
+Start with `>` to list the commands alone: those of the window, such as
+Reload schema, Close connection or Hide sidebar, and those of the tab in
+front, such as Close tab, Save changes, Run all or Format SQL. A command
+is listed only where it can do something now, with its key beside it
+where the window has one. Typing after `>` finds the settings too, each
+by where it is (`Settings › General › Restore tabs and filters`):
+Return opens Settings at that row, with its control ready for the
+keyboard.
 
 | Keys | Do |
 | --- | --- |
@@ -110,6 +121,7 @@
 | Return | Open the one picked, or show its tab where it has one |
 | Option+Return | Open a new SQL editor reading its first rows |
 | Tab | Search every open connection, or this one again |
+| `>` | Typed first: list the commands alone (and, with a word after it, the settings) |
 | Esc | Close |
 
 ### The picker
diff --git a/docs/_reference/keyboard-shortcuts.md b/docs/_reference/keyboard-shortcuts.md
--- a/docs/_reference/keyboard-shortcuts.md
+++ b/docs/_reference/keyboard-shortcuts.md
@@ -50,7 +50,7 @@
 
 | Keys | Do |
 | --- | --- |
-| Mod+P | Quick open (Ctrl+P in the Omarchy look too). In it, Up and Down move, Enter opens, Tab searches every open connection and Esc closes. On macOS and Windows, Alt+Enter opens a SQL editor reading the rows |
+| Mod+P | Quick open (Ctrl+P in the Omarchy look too). In it, Up and Down move, Enter opens, Tab searches every open connection and Esc closes. On macOS and Windows, Alt+Enter opens a SQL editor reading the rows, and `>` typed first lists the commands alone (and, with a word after it, the settings) |
 | Mod+B | Show or hide the sidebar |
 | Mod+Alt+A | Show the Agents tab |
 | Mod+Y | Show the [History tab]({% link _guide/sql-editor.md %}#history). Not while you type in a text field, where it is redo |
diff --git a/src/entrypoint.rs b/src/entrypoint.rs
--- a/src/entrypoint.rs
+++ b/src/entrypoint.rs
@@ -172,6 +172,12 @@
             if history_menu.is_none() {
                 log::warn!("the menu bar has no place for a View menu");
             }
+            #[cfg(target_os = "macos")]
+            let quick_open_menu = crate::macos::QuickOpenMenu::attach(
+                &cc.egui_ctx,
+                &crate::i18n::gettext(app.locale, "View"),
+                &crate::i18n::gettext(app.locale, "Quick Open"),
+            );
             Ok(Box::new(Window {
                 app,
                 shot,
@@ -184,6 +190,8 @@
                 settings_menu,
                 #[cfg(target_os = "macos")]
                 history_menu,
+                #[cfg(target_os = "macos")]
+                quick_open_menu,
             }))
         }),
     )
@@ -323,6 +331,9 @@
     /// macOS: View › History, which shows the window's History tab.
     #[cfg(target_os = "macos")]
     history_menu: Option<crate::macos::HistoryMenu>,
+    /// macOS: View › Quick Open, which opens quick open.
+    #[cfg(target_os = "macos")]
+    quick_open_menu: Option<crate::macos::QuickOpenMenu>,
 }
 
 impl Window {
@@ -401,6 +412,15 @@
             self.app
                 .actions
                 .push(crate::model::Action::ShowHistory(tab));
+        }
+        // Quick open, as ⌘P opens it: while it is open, its text selected.
+        #[cfg(target_os = "macos")]
+        if self
+            .quick_open_menu
+            .as_ref()
+            .is_some_and(crate::macos::QuickOpenMenu::take)
+        {
+            self.app.actions.push(crate::model::Action::OpenQuickOpen);
         }
         self.app.logic(ctx);
         self.drive_shot(ctx);
diff --git a/src/macos.rs b/src/macos.rs
--- a/src/macos.rs
+++ b/src/macos.rs
@@ -16,7 +16,8 @@
 //!
 //! The app menu's About item opens the app's own About dialog ([`AboutMenu`]),
 //! and its Settings item, added the same way, opens the Settings window
-//! ([`SettingsMenu`]). Pointing a menu item somewhere does need `unsafe`,
+//! ([`SettingsMenu`]). The View menu has History ([`HistoryMenu`]) and Quick
+//! Open ([`QuickOpenMenu`]). Pointing a menu item somewhere does need `unsafe`,
 //! which this crate forbids, so that one call lives in `tabletist-appkit`.
 
 use std::cell::Cell;
@@ -237,3 +238,41 @@
         self.chosen.replace(false)
     }
 }
+
+/// The View menu's Quick Open item, asking for quick open.
+///
+/// The item has no key equivalent, as History's has none: ⌘P is the
+/// window's key, which while quick open is open selects what is typed, and
+/// AppKit would take it before the window sees it.
+pub struct QuickOpenMenu {
+    chosen: Rc<Cell<bool>>,
+    /// The item is in the menu for as long as this lives.
+    _item: tabletist_appkit::ViewItem,
+}
+
+impl QuickOpenMenu {
+    /// Adds the item, titled `title`, to the View menu titled `menu`.
+    /// Choosing it draws a frame, which finds it with
+    /// [`QuickOpenMenu::take`]. `None` before the menu bar exists (or off
+    /// the main thread).
+    pub fn attach(ctx: &egui::Context, menu: &str, title: &str) -> Option<Self> {
+        let chosen = Rc::new(Cell::new(false));
+        let item = tabletist_appkit::ViewItem::insert(menu, title, {
+            let chosen = chosen.clone();
+            let ctx = ctx.clone();
+            move || {
+                chosen.set(true);
+                ctx.request_repaint();
+            }
+        })?;
+        Some(Self {
+            chosen,
+            _item: item,
+        })
+    }
+
+    /// Whether the item was chosen since the last call.
+    pub fn take(&self) -> bool {
+        self.chosen.replace(false)
+    }
+}
````

- [ ] **Step 2: Run the checks**

Run: the four checks of task 1's Step 4. On a Mac, or in an extract as "What was run" says, `src/macos.rs` and the entrypoint type-check for `aarch64-apple-darwin`.

Expected: all pass. The lib suite:

```
fmt exit 0
clippy exit 0
test result: ok. 3051 passed; 0 failed; 20 ignored; 0 measured; 0 filtered out
doc exit 0
```

- [ ] **Step 3: Commit**

```bash
git add src/macos.rs src/entrypoint.rs docs/_guide/macos.md docs/_guide/browsing-data.md docs/_reference/keyboard-shortcuts.md
git commit -S -m "Add Quick Open to the macOS View menu, and describe the commands on the website"
```

## The pull request

One pull request for the run. Its description holds:
- a summary;
- the registry: what it lists, what it leaves out and why (this plan's table);
- the key added (`>`, `QuickOpenCommands`), and View › Quick Open with no key equivalent (a change after approval: main's `ViewItem` has none);
- that the macOS menu item was type-checked here, and needs a hand test on a Mac;
- the departures (the master plan's and this plan's tables).

No screenshot goes to GitHub unblurred.
