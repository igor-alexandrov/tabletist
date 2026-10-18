# Settings › General, step 1: the file and the Data rows. Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `settings.toml` holds only what differs from the defaults and keeps the person's comments, key order and unknown keys when the app writes it; a value the file gets wrong keeps its last good value; Rows per page has the artboard's sizes and lands on the next load; Numbers follow the look; and a new Time zone for timestamptz row shows PostgreSQL's `timestamptz` values in the chosen zone in the grid, the SQL results and the row panel, and sends a value typed without an offset with the zone's.

**Architecture:** `Settings::write_into` (`src/settings.rs`) edits the text the app holds in place, at the byte spans the `toml` parser already gives (no new crate), touching only the keys that changed; the backend writes that text (`StateFile::Settings(String)`). `Settings::from_toml_over` reads a new text over the last settings. Each table tab keeps the size it pages by (`ObjectTab::page_size`). `group_digits` becomes `Option<bool>`, resolved by `Settings::grouped(layout)`. `Settings::tz: Zone` is resolved against the zone the app found at its start (`App::system_zone`) into `Workspace::zone`, which the cells (`data_view::Shown::zone`), the results, the row panel's texts (`row_fields`) and editing (`edit::Table::zone`) read; `ui::format::in_zone`, `with_offset` and `same_moment` do the work.

**Tech Stack:** Rust 2024, egui, the `toml` crate's spanned parser, `jiff` (already a dependency) for the zone database.

---

## Where this sits

Step 1 of `docs/superpowers/specs/2026-10-17-settings-general-rows-design.md`. Step 2 (Safety: production opens read-only, ask before closing with unsaved edits, disconnect when idle) and step 3 (Startup: the session file, restore tabs and filters, on launch) are their own plans and pull requests.

## Before you start

- Work on a branch of `main`. One commit for each task, in the order below. Every commit passes `cargo fmt --all --check`, `cargo clippy --locked --workspace --all-targets -- -D warnings` and `cargo test --locked --workspace --all-targets`.
- On Igor's machine `cargo` is `~/.cargo/bin/cargo` (the mise shim fails), and a build in a worktree needs `CARGO_TARGET_DIR` set to that worktree's `target/`.
- Read `AGENTS.md`: code that depends on the engine matches on `Dialect` naming every variant; views draw text only through `TextRole`s; the website's pages follow `src/settings.rs`.
- Each step's diff applies with `git apply` from the repository's root: save the block to a file (`/tmp` is fine) and run `git apply <file>`. Run `cargo fmt --all` before each commit: the code here is already formatted.
- The tests use only the Bookshop data. No test compares a picture or a position with the design.

## What was run before this plan was written

Every task's state was built here and passed `cargo fmt --all --check`, `cargo clippy --locked --workspace --all-targets -- -D warnings` and `cargo test --locked --workspace --all-targets` with the test PostgreSQL, MySQL and SSH servers (3,575 tests after task 1, 3,606 after task 7, none failed). Each task's tests were run alone on the code before it, and failed as its Step 2 says. The last state also passed `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps` and `docs/bin/check`. A reviewer given the draft wrote throwaway tests against it and found eleven defects, each fixed in the task it belongs to and covered by a test there (marked "found by the review" below). A second reviewer read the generated plan against its diffs and the spec; what it found (an edit of a text column taken for no change, the website saying more than the code, tests the spec lists that were missing) was fixed the same way. Every diff in this plan was applied in order to a copy of `main` and the result compared with the last state. Not run: CI, macOS and Windows, and the app by hand.

## Where this departs from the spec, the prompt or the board

The spec's own departures (the file holds only non-defaults; the macOS
folder stays `dev.tabletist.Tabletist`; the toml parser's spans instead of
`toml_edit`; the zone read once; Export saves a copy of the file) were
approved with it. These were settled while this run was drafted and
reviewed, and are new:

| What the spec says | What this run does | Why |
|---|---|---|
| A key set to its default is removed with its line | Where a rejected line names the same key (`page_size` given twice), its default is written in place instead | Taken out, the rejected line would be the one read, and the option would read back as what the person did not pick (found by the review) |
| A key not in the file is added at the end of its table | Where the file gives the table's name a value (`data = 5`, `[[data]]`), nothing is written for its keys, the log says why, and the keys keep the values they had | Nothing of ours fits there, and nothing of the person's is taken to make room (found by the review) |
| "The cell editor's original value [is] the server's text" | The editor opens on the moment in the zone, with that zone's offset; what is compared with the loaded value is still the server's text, and the same moment written another way is no change | The person edits what the cell shows them; the server's text with its own offset would be another time on screen |
| Omarchy: `h/l` step System, UTC and "the zone the file names" | They step System, UTC and the last named zone the session had (`App::named_zone`) | A step off the file's zone would otherwise lose it for good (found by the review) |

## What this run settles

| Question | Decision | Why |
|---|---|---|
| Which keys a change in the app writes | The ones whose value changed (`Settings::changed_keys`); Reset, the General tab's | A key the person wrote at its default stays as they wrote it until it is changed |
| A key set to its default | Taken out with its line; a table left with nothing of the person's (no key, no comment beside its header or under it, no line the reader ignored) goes too; a file left with nothing stays empty | "Write only values that differ from the defaults", and nothing of the person's lost |
| A key set to its default that a rejected line also names (`page_size` given twice) | Its default is written in place, not taken out | Taken out, the rejected line would be the one read (found by the review) |
| A key the file lacks | At the end of its table (after its last key, an array's too), with its note (`# second \| full`); a table the file lacks, or one only `[data.more]` names, at the end of the file | The person's order stays; the app's additions are where they can be found |
| A table name the file uses for a value (`data = 5`, `[[data]]`) | Left as written; the key is not written, and the log says so | Nothing of ours fits there, and nothing of the person's is taken to make room (found by the review) |
| A line the reader ignored | It stays through a write, and is still ignored and shown | Nothing the person wrote is lost to a change in the window |
| A table written inline (`data = { … }`) or with dotted keys | A value is replaced in place; adding or taking out a key there writes the whole file as `to_toml` has it | Lines cannot be added to an inline table |
| What "last good value" means | On a live reload and on a write, a key whose line is ignored, or whose table's name the file gives a value, keeps the value it had; one taken out of the file takes its default; at the start, there being no value before, the default | The prompt: "its key keeps its last good value" |
| A rejected line's key | Its name before `=` (bare, in quotes or dotted), under the last header above it that was read, after a byte order mark too; no `=` says none | Enough for the typo a person makes; a line that names no key keeps nothing |
| How Rows per page lands | Each table tab keeps the size it pages by; every load from a place (open, refresh, sort, filter, reconnect) takes the setting's, and Next and Previous keep the tab's | "Not mid-scroll" |
| Numbers' default | `None`, the look's: grouped on macOS and Windows, not on Omarchy. A value equal to the look's default, set in the app, is stored as `None` and not written; one the file has stays as the person wrote it | Open question 1 of the prompt, as drawn; a change of another option does not touch that line (found by the review) |
| Tests that read a value as it was typed | They set grouping off: they test editing, not grouping | The standard look groups by default now |
| Which values are moved to the zone | A temporal value whose text is a date, a time and an offset, as PostgreSQL's `timestamptz` is under `DateStyle = ISO`; MySQL and SQLite never send an offset. A text column never. A moment that would fall before the year 1 in the zone is shown as sent | Decided by the value, so a result with no structure is shown the same; PostgreSQL writes those years with ` BC` |
| Where the zone is shown | The grid and the SQL results (to the second or in full, as Timestamps says), and the row panel (in full), made again when the option changes | The prompt: open grids, the inspector and results, at once |
| What the editor opens on | The moment in the zone, with its offset. A text that names the moment that was loaded (in any zone, with `Z` or `+00:00` too) is no change, in a column of dates and times only: a text column's text is its own (found by the review) | What the person edits is what they see, and opening and closing changes nothing |
| A moment typed with no offset | Given the zone's offset, in a PostgreSQL `timestamptz` column (`edit::keeps_offset`, every `Dialect` named); the hour the clocks skip or have twice takes the offset from before the change. Only a date with a year of four digits and no zone's name in brackets is given one: anything else is sent as typed. SQL in the editor is the person's and is not touched | "Sent with their offset" |
| The macOS control | A menu: System (with the computer's zone's name), UTC, then a search field over every zone of jiff's database | 600 zones are no menu without a search |
| The Omarchy control | `h/l` step System, UTC and the last named zone the session had (`App::named_zone`), so a step off it can step back (found by the review); any other zone is set in the file. The row is labelled `timestamptz zone`, its hint `sent with offset`, its value `system (america/new_york)` | As the board writes it |
| The Omarchy screen at 1280 by 800 with the file pane | The zone's value widens the value column, and the timestamp's sample hint is left out as a hint that does not fit is | The board is 1920 by 1080, where it fits; its test runs at that size |
| Reset's question | "Reset all General settings to their defaults?" on General; the Agents pane keeps "Reset every option on this tab?" | The prompt's words are General's |

## Files

| File | What changes |
|---|---|
| `src/settings.rs` | The writer, the last-good reader, `Zone`, `data.tz`, the page sizes, `group_digits` by look, the Tz option |
| `src/backend.rs` | `StateFile::Settings` carries the text |
| `src/app.rs` | `change_settings` writes into the held text; Export; the tab's page size; `system_zone`, `zone()`; the zone reaches workspaces and row panels |
| `src/model.rs` | `ObjectTab::page_size`, `Workspace::zone`, the Omarchy cursor's rows |
| `src/edit.rs`, `src/app/editing.rs` | `Table::zone`, `keeps_offset`, a moment kept is no change, the editor's text and what is sent |
| `src/ui/format.rs` | `in_zone`, `same_moment`, `with_offset` |
| `src/ui/data_view.rs`, `src/ui/sql_results.rs`, `src/ui/workspace.rs` | The zone in `Shown` and `Fit`, grouping by look |
| `src/ui/settings/{mod,sheet,terminal,agents}.rs` | The zone's row in both looks, Reset's question |
| `src/testing.rs`, `src/ui/zone_tests.rs` | The Bookshop's covers with a `timestamptz`, and their tests |
| `docs/_reference/settings-and-files.md`, `docs/_guide/browsing-data.md`, `Cargo.toml` | The website, and why `jiff` is there |

---

### Task 1: Write only the keys that changed, in place

The app's change writes into the text it holds instead of rendering a canonical text: only the keys that changed, a default taken out, the rest of the person's text as it was. The backend writes that text. A value the file gets wrong keeps its last good one. Export saves the file as it is.

**Files:** Modify `docs/_reference/settings-and-files.md`; Modify `src/app.rs`; Modify `src/backend.rs`; Modify `src/settings.rs`; Modify `src/ui/mod.rs`.

- [ ] **Step 1: Write the failing tests**

````diff
diff --git a/src/app.rs b/src/app.rs
index d2e9268..e030518 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -5608,7 +5608,7 @@ mod tests {
                     matches!(
                         command,
                         Command::Save { path: to, file: StateFile::Settings(saved) }
-                            if *to == path && *saved == settings
+                            if *to == path && Settings::from_toml(saved).settings == settings
                     )
                 })
                 .count()
@@ -7246,15 +7246,16 @@ mod tests {
         );
     }
 
-    fn settings_saves(app: &App) -> Vec<&Settings> {
+    /// The settings each save of the settings file writes, as they read.
+    fn settings_saves(app: &App) -> Vec<Settings> {
         app.backend
             .sent
             .iter()
             .filter_map(|command| match command {
                 Command::Save {
-                    file: StateFile::Settings(settings),
+                    file: StateFile::Settings(text),
                     ..
-                } => Some(settings),
+                } => Some(Settings::from_toml(text).settings),
                 _ => None,
             })
             .collect()
@@ -7274,14 +7275,18 @@ mod tests {
         assert_eq!(app.settings.sql_limit, 100);
         assert_eq!(app.settings.page_size, 100, "what the file set stays");
         let saved = settings_saves(&app);
-        assert_eq!(saved.len(), 1);
-        assert_eq!(*saved[0], app.settings);
-        // The file is the canonical text now: nothing in it is invalid.
-        assert_eq!(app.settings_file.text, app.settings.to_toml());
-        assert!(app.settings_file.invalid.is_empty());
+        assert_eq!(saved, [app.settings.clone()]);
+        // The new key joins the file, and the rest stays as the person
+        // wrote it: the line the reader ignored is still there, and still
+        // ignored.
+        assert_eq!(
+            app.settings_file.text,
+            "[data]\npage_size = 100\ngroup_digits = \"yes\"\n\n[editor]\nsql_limit = 100\n"
+        );
+        assert_eq!(app.settings_file.invalid, [3]);
         assert_eq!(
             app.settings_file.lines,
-            Settings::from_toml(&app.settings.to_toml()).lines
+            Settings::from_toml(&app.settings_file.text).lines
         );
     }
 
@@ -7312,6 +7317,24 @@ mod tests {
         assert_eq!(settings_saves(&app).len(), saved);
     }
 
+    #[test]
+    fn a_value_the_file_gets_wrong_keeps_the_last_good_one_and_is_reported() {
+        let (mut app, _dir) = app();
+        app.apply(from_disk("[data]\npage_size = 500\n", false));
+        assert_eq!(app.settings.page_size, 500);
+        app.apply(from_disk("[data]\npage_size = \"lots\"\n", false));
+        assert_eq!(app.settings.page_size, 500);
+        assert_eq!(app.settings_file.invalid, [2]);
+        // A change made in the app then keeps it too, and the line.
+        app.change_settings(|settings| settings.value_tags = false);
+        assert_eq!(app.settings.page_size, 500);
+        assert_eq!(
+            app.settings_file.text,
+            "[data]\npage_size = \"lots\"\nvalue_tags = false\n"
+        );
+        assert_eq!(app.settings_file.invalid, [2]);
+    }
+
     #[test]
     fn a_change_of_the_file_is_applied_and_not_written_back() {
         let (mut app, _dir) = app();
@@ -7585,13 +7608,14 @@ mod tests {
     }
 
     #[test]
-    fn export_offers_the_canonical_text_under_the_settings_name() {
+    fn export_offers_a_copy_of_the_file_under_the_settings_name() {
         let (mut app, _dir) = app();
-        // A file as someone wrote it: not the text the app would write.
-        app.apply(from_disk("[data]\npage_size = 500\n", false));
+        // A file as someone wrote it, with a comment the app would not
+        // write: the copy has it.
+        let text = "# mine\n[data]\npage_size = 500\n";
+        app.apply(from_disk(text, false));
         assert_eq!(app.settings.page_size, 500);
-        let text = app.settings.to_toml();
-        assert_ne!(app.settings_file.text.len(), text.len());
+        assert_ne!(app.settings.to_toml().len(), text.len());
         app.apply(Action::ExportSettings);
         assert_eq!(
             app.backend.saves.last(),
@@ -9621,8 +9645,8 @@ mod tests {
         });
         assert!(matches!(
             last_sent(&harness.app),
-            Command::Save { file: StateFile::Settings(settings), .. }
-                if settings.sql_limit == 100
+            Command::Save { file: StateFile::Settings(text), .. }
+                if Settings::from_toml(text).settings.sql_limit == 100
         ));
         let sent = harness.app.backend.sent.len();
         harness.app.apply(Action::SetSqlTimeout {
@@ -9632,8 +9656,9 @@ mod tests {
         });
         assert!(matches!(
             &harness.app.backend.sent[sent..],
-            [Command::Save { file: StateFile::Settings(settings), .. }]
-                if settings.sql_limit == 100 && settings.sql_timeout_secs.is_none()
+            [Command::Save { file: StateFile::Settings(text), .. }]
+                if Settings::from_toml(text).settings.sql_limit == 100
+                    && Settings::from_toml(text).settings.sql_timeout_secs.is_none()
         ));
         type_sql(&mut harness, tab, id, "SELECT 1", 0);
         run(&mut harness, tab, id, false);
diff --git a/src/backend.rs b/src/backend.rs
index 037a9d6..7e300c6 100644
--- a/src/backend.rs
+++ b/src/backend.rs
@@ -4240,7 +4240,7 @@ mod tests {
         let mut backend = Backend::start_with(Waker::default(), Keyring::memory());
         backend.send(Command::Save {
             path: path.clone(),
-            file: StateFile::Settings(settings.clone()),
+            file: StateFile::Settings(settings.to_toml()),
         });
         match backend.wait(WAIT) {
             Some(Event::Saved { result: Ok(()), .. }) => {}
@@ -4265,7 +4265,7 @@ mod tests {
         let files = [
             (
                 "settings.toml",
-                StateFile::Settings(crate::settings::Settings::default()),
+                StateFile::Settings(crate::settings::Settings::default().to_toml()),
             ),
             (
                 "connections.toml",
@@ -5873,7 +5873,7 @@ mod tests {
         };
         backend.send(Command::Save {
             path: path.clone(),
-            file: StateFile::Settings(settings.clone()),
+            file: StateFile::Settings(settings.to_toml()),
         });
         assert_eq!(
             files_until(&mut backend, &settings.to_toml()),
@@ -5893,7 +5893,7 @@ mod tests {
         };
         backend.send(Command::Save {
             path: path.clone(),
-            file: StateFile::Settings(newer.clone()),
+            file: StateFile::Settings(newer.to_toml()),
         });
         assert_eq!(
             files_until(&mut backend, &newer.to_toml()),
@@ -5918,7 +5918,7 @@ mod tests {
         };
         backend.send(Command::Save {
             path: path.clone(),
-            file: StateFile::Settings(first.clone()),
+            file: StateFile::Settings(first.to_toml()),
         });
         assert_eq!(
             files_until(&mut backend, &first.to_toml()),
@@ -5934,7 +5934,7 @@ mod tests {
         };
         backend.send(Command::Save {
             path: path.clone(),
-            file: StateFile::Settings(second),
+            file: StateFile::Settings(second.to_toml()),
         });
         let deadline = std::time::Instant::now() + WAIT;
         loop {
diff --git a/src/settings.rs b/src/settings.rs
index 3ad616a..ff9dd83 100644
--- a/src/settings.rs
+++ b/src/settings.rs
@@ -816,28 +816,53 @@ mod tests {
     }
 
     #[test]
-    fn the_defaults_are_written_as_the_canonical_text() {
+    fn the_defaults_are_written_as_the_header_alone() {
+        assert_eq!(Settings::default().to_toml(), HEADER);
+    }
+
+    /// Every key away from its default.
+    fn all_set() -> Settings {
+        Settings {
+            page_size: 500,
+            timestamps: Timestamps::Full,
+            group_digits: true,
+            value_tags: false,
+            show_system_schemas: true,
+            custom_theme: Some("My \"Nord\".json".into()),
+            sql_limit: 100,
+            sql_timeout_secs: None,
+            mcp_enabled: true,
+            mcp_log_days: 7,
+            mcp_notify: false,
+        }
+    }
+
+    #[test]
+    fn only_what_is_not_the_default_is_written_each_table_in_its_place() {
         assert_eq!(
-            Settings::default().to_toml(),
+            all_set().to_toml(),
             "\
 # written by tabletist, safe to edit by hand
 [data]
-page_size    = 300
-timestamps   = \"second\"  # second | full
-group_digits = false
-value_tags   = true
+page_size = 500
+timestamps = \"full\"  # second | full
+group_digits = true
+value_tags = false
 
 [sidebar]
-show_system_schemas = false
+show_system_schemas = true
 
 [editor]
-sql_limit        = 1000
-sql_timeout_secs = 30  # 0 waits forever
+sql_limit = 100
+sql_timeout_secs = 0  # 0 waits forever
 
 [mcp]
-enabled  = false
-log_days = 30
-notify   = true
+enabled = true
+log_days = 7
+notify = false
+
+[appearance]
+theme = \"My \\\"Nord\\\".json\"
 "
         );
     }
@@ -852,7 +877,7 @@ notify   = true
         };
         let text = settings.to_toml();
         assert!(
-            text.contains("timestamps   = \"full\"  # second | full\n"),
+            text.contains("timestamps = \"full\"  # second | full\n"),
             "{text}"
         );
         assert!(
@@ -875,19 +900,7 @@ notify   = true
 
     #[test]
     fn what_is_written_reads_back_the_same() {
-        let settings = Settings {
-            page_size: 500,
-            timestamps: Timestamps::Full,
-            group_digits: true,
-            value_tags: false,
-            show_system_schemas: true,
-            custom_theme: Some("My \"Nord\".json".into()),
-            sql_limit: 100,
-            sql_timeout_secs: None,
-            mcp_enabled: true,
-            mcp_log_days: 7,
-            mcp_notify: false,
-        };
+        let settings = all_set();
         let text = settings.to_toml();
         let loaded = Settings::from_toml(&text);
         assert_eq!(loaded.settings, settings);
@@ -900,7 +913,7 @@ notify   = true
 
     #[test]
     fn each_key_is_found_on_its_line() {
-        let loaded = Settings::from_toml(&Settings::default().to_toml());
+        let loaded = Settings::from_toml(&all_set().to_toml());
         assert_eq!(
             loaded.lines,
             vec![
@@ -914,10 +927,345 @@ notify   = true
                 (Key::McpEnabled, 16),
                 (Key::McpLogDays, 17),
                 (Key::McpNotify, 18),
+                (Key::Theme, 21),
             ]
         );
     }
 
+    /// `text` with `change` made in the app: what the file says after.
+    fn written(text: &str, change: impl FnOnce(&mut Settings)) -> String {
+        let before = Settings::from_toml(text).settings;
+        let mut after = before.clone();
+        change(&mut after);
+        after.write_into(text, &before.changed_keys(&after))
+    }
+
+    #[test]
+    fn a_value_is_set_in_place_and_the_comment_beside_it_stays() {
+        let text = "# mine\n[data]\n# rows\npage_size = 500 # as I like it\nvalue_tags = false\n";
+        assert_eq!(
+            written(text, |settings| settings.page_size = 100),
+            "# mine\n[data]\n# rows\npage_size = 100 # as I like it\nvalue_tags = false\n"
+        );
+    }
+
+    #[test]
+    fn a_key_set_to_its_default_goes_with_its_line_and_its_comments_stay() {
+        let text = "[data]\n# rows\npage_size = 500 # as I like it\nvalue_tags = false\n";
+        assert_eq!(
+            written(text, |settings| settings.page_size =
+                Settings::DEFAULT_PAGE_SIZE),
+            "[data]\n# rows\nvalue_tags = false\n"
+        );
+    }
+
+    #[test]
+    fn a_table_left_with_nothing_in_it_goes() {
+        let text = "# mine\n\n[data]\npage_size = 500\n\n[editor]\nsql_limit = 100\n";
+        assert_eq!(
+            written(text, |settings| settings.page_size =
+                Settings::DEFAULT_PAGE_SIZE),
+            "# mine\n\n[editor]\nsql_limit = 100\n"
+        );
+        // The last table too, and the blank lines it leaves at the end.
+        assert_eq!(
+            written(text, |settings| settings.sql_limit = 1_000),
+            "# mine\n\n[data]\npage_size = 500\n"
+        );
+    }
+
+    #[test]
+    fn a_table_with_a_comment_left_in_it_stays() {
+        let text = "[data]\npage_size = 500\n# keep me\n\n[editor]\nsql_limit = 100\n";
+        assert_eq!(
+            written(text, |settings| settings.page_size =
+                Settings::DEFAULT_PAGE_SIZE),
+            "[data]\n# keep me\n\n[editor]\nsql_limit = 100\n"
+        );
+    }
+
+    #[test]
+    fn a_new_key_goes_at_the_end_of_its_table_with_its_note() {
+        let text = "[data]\npage_size = 500\n\n[data.more]\nx = 1\n\n[editor]\nsql_limit = 100\n";
+        assert_eq!(
+            written(text, |settings| settings.timestamps = Timestamps::Full),
+            "[data]\npage_size = 500\ntimestamps = \"full\"  # second | full\n\n\
+             [data.more]\nx = 1\n\n[editor]\nsql_limit = 100\n"
+        );
+        // Under the header of a table that has no key yet.
+        assert_eq!(
+            written("[data]\n", |settings| settings.page_size = 500),
+            "[data]\npage_size = 500\n"
+        );
+    }
+
+    #[test]
+    fn a_new_table_goes_at_the_end_of_the_file() {
+        let text = "[data]\npage_size = 500\n# the end";
+        assert_eq!(
+            written(text, |settings| settings.sql_limit = 100),
+            "[data]\npage_size = 500\n# the end\n\n[editor]\nsql_limit = 100\n"
+        );
+    }
+
+    #[test]
+    fn unknown_keys_and_tables_and_their_comments_survive_a_write() {
+        let text = "[data]\nfrom_the_future = 1 # later\npage_size = 500\n\n[tomorrow]\nkey = 1\n";
+        assert_eq!(
+            written(text, |settings| settings.page_size = 100),
+            "[data]\nfrom_the_future = 1 # later\npage_size = 100\n\n[tomorrow]\nkey = 1\n"
+        );
+        assert_eq!(
+            written(text, |settings| settings.page_size =
+                Settings::DEFAULT_PAGE_SIZE),
+            "[data]\nfrom_the_future = 1 # later\n\n[tomorrow]\nkey = 1\n"
+        );
+    }
+
+    #[test]
+    fn a_key_at_its_default_by_hand_stays_until_it_is_changed() {
+        let text = format!("[data]\npage_size = {}\n", Settings::DEFAULT_PAGE_SIZE);
+        assert_eq!(
+            written(&text, |settings| settings.value_tags = false),
+            format!("{text}value_tags = false\n")
+        );
+    }
+
+    #[test]
+    fn a_line_the_reader_ignored_stays_through_a_write() {
+        let text = "[data]\ntimestamps = full\ngroup_digits = \"yes\"\npage_size = 500\n";
+        let loaded = Settings::from_toml(text);
+        assert_eq!(loaded.invalid, [2, 3]);
+        let out = written(text, |settings| settings.page_size = 100);
+        assert_eq!(
+            out,
+            "[data]\ntimestamps = full\ngroup_digits = \"yes\"\npage_size = 100\n"
+        );
+        assert_eq!(Settings::from_toml(&out).invalid, [2, 3]);
+        // A key whose value was refused gets the new one in its place.
+        assert_eq!(
+            written(text, |settings| settings.group_digits = true),
+            "[data]\ntimestamps = full\ngroup_digits = true\npage_size = 500\n"
+        );
+    }
+
+    #[test]
+    fn a_file_with_crlf_endings_gets_crlf_lines() {
+        let text = "[data]\r\npage_size = 500\r\n";
+        assert_eq!(
+            written(text, |settings| settings.value_tags = false),
+            "[data]\r\npage_size = 500\r\nvalue_tags = false\r\n"
+        );
+    }
+
+    #[test]
+    fn an_inline_table_gets_values_in_place_and_otherwise_the_whole_file() {
+        let text = "data = { page_size = 500 } # inline\n";
+        assert_eq!(
+            written(text, |settings| settings.page_size = 100),
+            "data = { page_size = 100 } # inline\n"
+        );
+        let mut settings = Settings::from_toml(text).settings;
+        settings.value_tags = false;
+        let out = written(text, |changed| changed.value_tags = false);
+        assert_eq!(out, settings.to_toml());
+    }
+
+    #[test]
+    fn a_value_that_is_ignored_keeps_its_last_good_value() {
+        let last = Settings {
+            page_size: 500,
+            timestamps: Timestamps::Full,
+            ..Settings::default()
+        };
+        // A value of the wrong type, and a line the parser rejects.
+        let text = "[data]\npage_size = \"lots\"\ntimestamps = full\n";
+        let loaded = Settings::from_toml_over(text, &last);
+        assert_eq!(loaded.invalid, [2, 3]);
+        assert_eq!(loaded.settings.page_size, 500);
+        assert_eq!(loaded.settings.timestamps, Timestamps::Full);
+        // Read with nothing before it, the last good value is the default.
+        let loaded = Settings::from_toml(text);
+        assert_eq!(loaded.settings.page_size, Settings::DEFAULT_PAGE_SIZE);
+        assert_eq!(loaded.settings.timestamps, Timestamps::Second);
+    }
+
+    #[test]
+    fn a_key_taken_out_of_the_file_takes_its_default() {
+        let last = Settings {
+            page_size: 500,
+            ..Settings::default()
+        };
+        let loaded = Settings::from_toml_over("[data]\nvalue_tags = false\n", &last);
+        assert_eq!(loaded.settings.page_size, Settings::DEFAULT_PAGE_SIZE);
+        assert!(!loaded.settings.value_tags);
+    }
+
+    #[test]
+    fn a_rejected_line_is_taken_for_the_key_of_the_table_above_it() {
+        assert_eq!(
+            key_on_line("[data] # rows\npage_size = 1 2\n", "[data] # rows\n", 2),
+            Some(Key::PageSize)
+        );
+        assert_eq!(
+            key_on_line("[editor]\npage_size = x y\n", "[editor]\n", 2),
+            None
+        );
+        assert_eq!(key_on_line("[data]\npage_size\n", "[data]\n", 2), None);
+        assert_eq!(key_on_line("page_size = x y\n", "", 1), None);
+        // In quotes, dotted, and under a header after a byte order mark.
+        assert_eq!(
+            key_on_line("[data]\n\"page_size\" = x y\n", "[data]\n", 2),
+            Some(Key::PageSize)
+        );
+        assert_eq!(
+            key_on_line("data.page_size = x y\n", "", 1),
+            Some(Key::PageSize)
+        );
+        assert_eq!(
+            key_on_line("\u{feff}[data]\npage_size = x y\n", "\u{feff}[data]\n", 2),
+            Some(Key::PageSize)
+        );
+        assert_eq!(
+            key_on_line("[data]\nx.page_size = 1 2\n", "[data]\n", 2),
+            None
+        );
+    }
+
+    #[test]
+    fn a_text_too_long_to_read_keeps_every_last_good_value() {
+        let last = Settings {
+            page_size: 500,
+            ..Settings::default()
+        };
+        let text = "# note\n".repeat(MAX_LINES + 1);
+        assert_eq!(Settings::from_toml_over(&text, &last).settings, last);
+    }
+
+    #[test]
+    fn a_table_with_a_line_the_reader_ignored_stays() {
+        let text = "[data]\npage_size = 500\nnot toml at all\n";
+        assert_eq!(
+            written(text, |settings| settings.page_size =
+                Settings::DEFAULT_PAGE_SIZE),
+            "[data]\nnot toml at all\n"
+        );
+        // A header given twice, the second rejected: it stays rejected.
+        let text = "[data]\ntimestamps = \"full\"\n[data]\n";
+        let out = written(text, |settings| settings.timestamps = Timestamps::Second);
+        assert_eq!(out, "[data]\n[data]\n");
+        assert_eq!(Settings::from_toml(&out).invalid, [2]);
+    }
+
+    #[test]
+    fn a_comment_beside_a_header_keeps_its_table() {
+        let text = "[data] # my table\npage_size = 500\n";
+        assert_eq!(
+            written(text, |settings| settings.page_size =
+                Settings::DEFAULT_PAGE_SIZE),
+            "[data] # my table\n"
+        );
+    }
+
+    #[test]
+    fn a_file_left_with_nothing_stays_empty() {
+        assert_eq!(
+            written("[data]\npage_size = 500\n", |settings| {
+                settings.page_size = Settings::DEFAULT_PAGE_SIZE;
+            }),
+            ""
+        );
+    }
+
+    #[test]
+    fn a_key_with_a_rejected_twin_is_set_to_its_default_in_place() {
+        // The second line is rejected: taking out the first would make it
+        // the one read.
+        let text = "[data]\npage_size = 500\npage_size = 100\n";
+        let out = written(text, |settings| {
+            settings.page_size = Settings::DEFAULT_PAGE_SIZE
+        });
+        assert_eq!(
+            out,
+            format!(
+                "[data]\npage_size = {}\npage_size = 100\n",
+                Settings::DEFAULT_PAGE_SIZE
+            )
+        );
+        assert_eq!(
+            Settings::from_toml(&out).settings.page_size,
+            Settings::DEFAULT_PAGE_SIZE
+        );
+    }
+
+    #[test]
+    fn a_table_only_a_table_under_it_names_gets_a_header_at_the_end() {
+        let text = "# mine\n[data.more]\ny = 1\n";
+        let out = written(text, |settings| settings.page_size = 100);
+        assert_eq!(
+            out,
+            "# mine\n[data.more]\ny = 1\n\n[data]\npage_size = 100\n"
+        );
+        assert_eq!(Settings::from_toml(&out).settings.page_size, 100);
+    }
+
+    #[test]
+    fn a_name_that_is_a_value_is_left_as_the_person_wrote_it() {
+        for text in ["# mine\ndata = 5\n", "# mine\n[[data]]\nx = 1\n"] {
+            assert_eq!(written(text, |settings| settings.page_size = 100), text);
+        }
+    }
+
+    #[test]
+    fn a_table_name_given_a_value_keeps_its_keys_last_good_values() {
+        let last = Settings {
+            page_size: 100,
+            ..Settings::default()
+        };
+        for text in ["data = 5\n", "[[data]]\npage_size = 500\n"] {
+            let loaded = Settings::from_toml_over(text, &last);
+            assert_eq!(loaded.settings.page_size, 100, "{text:?}");
+        }
+    }
+
+    #[test]
+    fn a_new_key_goes_after_a_key_whose_value_is_an_array() {
+        let text = "[data]\npage_size = 500\nmine = [1, 2]\n";
+        assert_eq!(
+            written(text, |settings| settings.value_tags = false),
+            "[data]\npage_size = 500\nmine = [1, 2]\nvalue_tags = false\n"
+        );
+        let text = "[data]\nmine = [1, 2]\n";
+        assert_eq!(
+            written(text, |settings| settings.value_tags = false),
+            "[data]\nmine = [1, 2]\nvalue_tags = false\n"
+        );
+    }
+
+    #[test]
+    fn a_rejected_line_in_quotes_or_dotted_keeps_the_last_good_value() {
+        let last = Settings {
+            timestamps: Timestamps::Full,
+            ..Settings::default()
+        };
+        for text in [
+            "[data]\n\"timestamps\" = full\n",
+            "data.timestamps = full\n",
+            "\u{feff}[data]\ntimestamps = full\n",
+        ] {
+            let loaded = Settings::from_toml_over(text, &last);
+            assert_eq!(loaded.settings.timestamps, Timestamps::Full, "{text:?}");
+        }
+    }
+
+    #[test]
+    fn an_empty_file_starts_with_the_header() {
+        assert_eq!(
+            written("", |settings| settings.page_size = 500),
+            "# written by tabletist, safe to edit by hand\n[data]\npage_size = 500\n"
+        );
+    }
+
     #[test]
     fn a_missing_key_keeps_its_default_and_an_unknown_one_is_ignored() {
         let loaded = Settings::from_toml(
@@ -1266,7 +1614,7 @@ notify   = true
             sql_timeout_secs: None,
             ..Settings::default()
         };
-        settings.save(&dirs.settings_file()).unwrap();
+        crate::util::write_atomic(&dirs.settings_file(), settings.to_toml().as_bytes()).unwrap();
         let loaded = Settings::load(&dirs);
         assert_eq!(loaded.settings, settings);
         assert_eq!(loaded.source, Source::Toml);
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
index 1159b8b..045f90b 100644
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -1045,6 +1045,30 @@ mod tests {
         harness
     }
 
+    /// A settings file as a version before this one wrote it: every key of
+    /// the Data table, aligned.
+    const OLDER_FILE: &str = "\
+# written by tabletist, safe to edit by hand
+[data]
+page_size    = 300
+timestamps   = \"second\"  # second | full
+group_digits = false
+value_tags   = true
+";
+
+    /// The Settings screen over [`OLDER_FILE`], as read from the disk.
+    fn settings_screen_over_older_file() -> Harness {
+        let mut harness = settings_screen();
+        harness.app.apply(crate::model::Action::Backend(
+            crate::backend::Event::SettingsFile {
+                text: OLDER_FILE.into(),
+                own: false,
+            },
+        ));
+        harness.settle();
+        harness
+    }
+
     fn settings_open(harness: &Harness) -> bool {
         matches!(harness.app.dialog, Some(crate::model::Dialog::Settings(_)))
     }
@@ -1860,7 +1884,7 @@ mod tests {
 
     #[test]
     fn the_settings_screen_shows_the_file_beside_the_options() {
-        let mut harness = settings_screen();
+        let mut harness = settings_screen_over_older_file();
         // The text the app holds, a line of the file per line.
         assert!(harness.painted_color("[data]").is_some());
         assert!(harness.painted_color("page_size    = ").is_some());
@@ -1913,7 +1937,7 @@ mod tests {
 
     #[test]
     fn the_file_pane_marks_the_line_of_the_cursors_option() {
-        let mut harness = settings_screen();
+        let mut harness = settings_screen_over_older_file();
         let selection = harness.app.palette.selection;
         // The fill behind the line the text `key` starts.
         let marked = |harness: &Harness, key: &str| {
@@ -1931,6 +1955,22 @@ mod tests {
         assert!(!marked(&harness, "page_size    = "));
     }
 
+    #[test]
+    fn with_no_file_the_pane_shows_the_header_and_marks_no_line() {
+        let harness = settings_screen();
+        let selection = harness.app.palette.selection;
+        let header = "# written by tabletist, safe to edit by hand";
+        assert!(harness.painted_color(header).is_some());
+        assert!(harness.painted_color("[data]").is_none());
+        // Nothing behind the header: the cursor's option has no line.
+        let line = harness.painted_rect(header).expect("the header");
+        assert!(!harness.fills.iter().any(|(rect, color)| {
+            *color == selection
+                && rect.y_range().contains(line.center().y)
+                && rect.x_range().contains(line.center().x)
+        }));
+    }
+
     #[test]
     fn the_file_panes_note_reads_as_one_sentence_without_its_marks() {
         let harness = settings_screen();
@@ -1944,8 +1984,7 @@ mod tests {
     /// lines of comments, with one line that is ignored among the first.
     fn settings_screen_over_a_long_file(notes: usize) -> Harness {
         let mut harness = settings_screen();
-        let settings = harness.app.settings.to_toml();
-        let settings = settings.replace("group_digits = false", "group_digits = \"yes\"");
+        let settings = OLDER_FILE.replace("group_digits = false", "group_digits = \"yes\"");
         let notes: String = (1..=notes).map(|note| format!("# note {note}\n")).collect();
         harness.app.apply(crate::model::Action::Backend(
             crate::backend::Event::SettingsFile {
@@ -2132,7 +2171,7 @@ mod tests {
 
     #[test]
     fn the_settings_screen_shows_a_notice_until_it_is_dismissed() {
-        let mut harness = settings_screen();
+        let mut harness = settings_screen_over_older_file();
         let message = "Could not save the password in the keyring.";
         harness.app.notice = Some(message.into());
         let shown = settings_notice(&mut harness, message).expect("the notice on the screen");
````

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --locked -p tabletist --lib`
Expected: FAIL to compile: `` mismatched types ``; `` no method named `write_into` found for struct `settings::Settings` in the current scope ``; `` no method named `changed_keys` found for struct `settings::Settings` in the current scope ``; `` no associated function or constant named `from_toml_over` found for struct `settings::Settings` in the current scope ``, and 1 more.

- [ ] **Step 3: Write the code**

````diff
diff --git a/docs/_reference/settings-and-files.md b/docs/_reference/settings-and-files.md
index 3e49552..22d4467 100644
--- a/docs/_reference/settings-and-files.md
+++ b/docs/_reference/settings-and-files.md
@@ -37,7 +37,10 @@ it is. In the Omarchy look the file is shown beside the options.
 ## The settings file
 
 Settings are kept in `settings.toml`, which is safe to edit by hand.
-Tabletist watches the file and applies your edits while it runs.
+Tabletist watches the file and applies your edits while it runs. The file
+holds only what differs from the defaults. Until you change an option
+there is no file, and one the app writes starts with a header line. Here
+is every key at its default:
 
 ```toml
 [data]
@@ -76,9 +79,17 @@ notify = true
 `show_system_schemas` and `theme` are set only in the file. The SQL
 editor's **Limit** and **Timeout** menus set the two `editor` keys.
 
-A line that cannot be read is ignored, keeps its default, and is reported
-in the Settings window. When you change an option in the app, the file is
-written again in a standard form, which drops comments you added.
+A line that cannot be read is ignored and reported in the Settings window,
+and its key keeps its last good value: the one it had before the edit, or
+its default when Tabletist starts. When you change an option in the app,
+only its key is written: set back to its default, the key is taken out of
+the file, and otherwise it gets the new value where it is, or goes at the
+end of its table. Your comments, the order of your keys, the lines that
+could not be read and keys Tabletist does not know all stay as you wrote
+them. The one exception is a table written inline (`data = { … }`) or
+with dotted keys (`data.page_size = 500`): a key added to it or taken out
+of it writes the whole file again, without your comments. **Export…**
+saves a copy of the file as it is.
 
 ## Files
 
diff --git a/src/app.rs b/src/app.rs
index e030518..4931e8f 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -1754,12 +1754,12 @@ impl App {
                     text,
                 });
             }
-            // The app's own text, whatever the file holds: a line the app
-            // ignores is not a setting to hand on.
+            // A copy of the file as the app holds it: the person's
+            // comments with it, and a line the app ignores as it is.
             Action::ExportSettings => self.backend.save_bytes(
                 "Export settings",
                 "tabletist-settings.toml".to_owned(),
-                self.settings.to_toml().into_bytes(),
+                self.settings_file.text.clone().into_bytes(),
             ),
             Action::ResetSettings => {
                 if let Some(Dialog::Settings(dialog)) = &mut self.dialog {
@@ -3087,7 +3087,7 @@ impl App {
                 if own && !newest {
                     return;
                 }
-                let loaded = Settings::from_toml(&text);
+                let loaded = Settings::from_toml_over(&text, &self.settings);
                 loaded.warn_invalid(&self.dirs.settings_file());
                 let live = self.settings_file.live;
                 let saved = self.settings_file.saved.take();
@@ -3555,13 +3555,14 @@ impl App {
     /// [`App::change_settings`] and the start that read the old
     /// settings.json call it.
     fn save_settings(&mut self) {
-        self.settings_file.saved = Some(self.settings.to_toml());
+        let text = self.settings_file.text.clone();
+        self.settings_file.saved = Some(text.clone());
         // The save is newer than any text handed over with the file: where
         // one of those was written, this lands over it.
         self.settings_file.offered.clear();
         self.backend.send(Command::Save {
             path: self.dirs.settings_file(),
-            file: StateFile::Settings(self.settings.clone()),
+            file: StateFile::Settings(text),
         });
     }
 
@@ -3586,16 +3587,20 @@ impl App {
     }
 
     /// A change made in the app (a menu, the Settings window): applied, and
-    /// written as the canonical text, which is the file from then on. The
-    /// lines the reader had ignored are gone with the text they were in.
-    /// A change that changes nothing writes nothing.
+    /// written into the file's text, where it changes the keys that changed
+    /// and nothing else (`Settings::write_into`). The lines the reader had
+    /// ignored stay, and are still ignored. A change that changes nothing
+    /// writes nothing.
     pub fn change_settings(&mut self, change: impl FnOnce(&mut Settings)) {
         let mut changed = self.settings.clone();
         change(&mut changed);
-        let loaded = Loaded::of(changed, Source::Toml);
-        if loaded.settings == self.settings {
+        let changed = changed.validated();
+        let keys = self.settings.changed_keys(&changed);
+        if keys.is_empty() {
             return;
         }
+        let text = changed.write_into(&self.settings_file.text, &keys);
+        let loaded = Settings::from_toml_over(&text, &changed);
         let live = self.settings_file.live;
         let saved = self.settings_file.saved.take();
         let (settings, file) = loaded.into_parts();
diff --git a/src/backend.rs b/src/backend.rs
index 7e300c6..ea440fa 100644
--- a/src/backend.rs
+++ b/src/backend.rs
@@ -203,7 +203,8 @@ pub enum StateFile {
     /// The note that the connections are kept in `connections.toml`.
     ConnectionsMoved,
     KnownHosts(HostKeys),
-    Settings(crate::settings::Settings),
+    /// The settings file's text, as the app made it.
+    Settings(String),
     Labels(crate::labels::Labels),
     /// What agents may do, as the app last showed it.
     AgentsShown(crate::agents_shown::Shown),
@@ -218,7 +219,7 @@ impl StateFile {
                 crate::util::write_atomic(path, crate::connections::MOVED.as_bytes())
             }
             Self::KnownHosts(keys) => crate::known_hosts::save(path, keys),
-            Self::Settings(settings) => settings.save(path),
+            Self::Settings(text) => crate::util::write_atomic(path, text.as_bytes()),
             Self::Labels(labels) => labels.save(path),
             Self::AgentsShown(shown) => shown.save(path),
         }
@@ -861,14 +862,14 @@ impl Saves {
             };
             let result = match &file {
                 // Known to the settings file's reader for the app's own.
-                StateFile::Settings(settings) => {
+                StateFile::Settings(text) => {
                     let mut written = lock(&self.settings_written);
                     let result = file.save(path);
                     // After a write that failed nothing on the disk is
                     // known to be the app's: the text there may be an
                     // older write of its own, but the app has moved on
                     // from it, and someone putting it back must be heard.
-                    *written = result.is_ok().then(|| settings.to_toml());
+                    *written = result.is_ok().then(|| text.clone());
                     result
                 }
                 _ => file.save(path),
diff --git a/src/settings.rs b/src/settings.rs
index ff9dd83..8fc10e2 100644
--- a/src/settings.rs
+++ b/src/settings.rs
@@ -402,23 +402,201 @@ fn last_line(text: &str) -> Option<usize> {
         .last()
 }
 
-/// `text` with the lines numbered in `lines` (from 1) emptied. Their line
-/// breaks stay, so every other line keeps its number.
+/// `text` with the lines numbered in `lines` (from 1) blanked: each byte
+/// but the line break becomes a space. Every other line keeps its number,
+/// and every byte its offset, so what is found in the blanked text is at
+/// the same place in `text`.
 fn without_lines(text: &str, lines: &[usize]) -> String {
     text.split_inclusive('\n')
         .enumerate()
         .map(|(index, line)| {
             if !lines.contains(&(index + 1)) {
-                line
-            } else if line.ends_with('\n') {
-                "\n"
-            } else {
-                ""
+                return line.to_owned();
             }
+            let body = line.strip_suffix('\n');
+            let mut blank = " ".repeat(body.unwrap_or(line).len());
+            if body.is_some() {
+                blank.push('\n');
+            }
+            blank
         })
         .collect()
 }
 
+/// `text` as the reader reads it: every line the parser rejects blanked,
+/// until it rejects nothing more, and those lines. The parser recovers
+/// from a bad line in two ways that are not "ignored": it stops reading at
+/// some (a key with no `=`), and reads past others with a guess at the
+/// value (`full` unquoted). So a line it rejects is blanked and the text
+/// read again.
+fn readable(text: &str) -> (String, Vec<usize>) {
+    let mut working = text.to_owned();
+    let mut invalid: Vec<usize> = Vec::new();
+    loop {
+        let (_, errors) = toml::de::DeTable::parse_recoverable(&working);
+        let mut rejected = Vec::new();
+        for error in &errors {
+            match error.span() {
+                // The parser blames the end of the text for a value that
+                // never closes (`"""` with no end). That is no line of the
+                // file, so the last line with anything on it goes: the
+                // lines are blanked from the bottom up until the opening
+                // one is. With every line blank the error marks none.
+                Some(span) if span.start >= working.len() => {
+                    rejected.extend(last_line(&working));
+                }
+                Some(span) => rejected.push(line_of(&working, span.start)),
+                None => log::warn!("settings: {}", error.message()),
+            }
+        }
+        // A line already blanked has nothing left to reject: an error
+        // that still names one ends the loop.
+        rejected.retain(|line| !invalid.contains(line));
+        rejected.sort_unstable();
+        rejected.dedup();
+        if rejected.is_empty() {
+            return (working, invalid);
+        }
+        working = without_lines(&working, &rejected);
+        invalid.extend(rejected);
+    }
+}
+
+/// `text` without the table `name` where it has a header of its own and
+/// nothing with it: no key, no table of its own, no comment beside its
+/// header, and nothing of the person's (a comment, a line the reader
+/// ignored) before the next header that was read. Its header goes, with
+/// the blank lines under it.
+fn without_empty_table(text: &str, name: &str) -> String {
+    let (working, _) = readable(text);
+    let (document, _) = toml::de::DeTable::parse_recoverable(&working);
+    let document = document.into_inner();
+    let Some(table) = document.get(name) else {
+        return text.to_owned();
+    };
+    let header = table.span();
+    let empty = table
+        .get_ref()
+        .as_table()
+        .is_some_and(|items| items.is_empty());
+    if !empty || !working[header.clone()].starts_with('[') {
+        return text.to_owned();
+    }
+    let blank = |line: &str| line.trim_matches([' ', '\t', '\r', '\n']).is_empty();
+    let start = line_start(&working, header.start);
+    let mut end = line_end(&working, header.end);
+    // A comment beside the header is the person's: the table stays with it.
+    if !blank(&text[header.end..end]) {
+        return text.to_owned();
+    }
+    while end < working.len() {
+        let next = line_end(&working, end);
+        // The next header that was read ends the table.
+        if working[end..next]
+            .trim_start_matches([' ', '\t'])
+            .starts_with('[')
+        {
+            break;
+        }
+        // Anything else of the person's text keeps it: a comment, or a
+        // line the reader ignored, which is blank only where it was read.
+        if !blank(&text[end..next]) {
+            return text.to_owned();
+        }
+        end = next;
+    }
+    let mut out = text.to_owned();
+    out.replace_range(start..end, "");
+    out
+}
+
+/// The key a line the parser rejected would have set: its name before
+/// the `=` (bare, in quotes, or dotted), under the last table header above
+/// it that was read (`working`, where rejected lines are blank). A line
+/// with no `=`, or a name of another form, says none.
+fn key_on_line(text: &str, working: &str, line: usize) -> Option<Key> {
+    // The parts of a dotted name, each bare or in quotes with nothing to
+    // escape.
+    let parts = |name: &str| -> Option<Vec<String>> {
+        name.split('.')
+            .map(|part| {
+                let part = part.trim_matches([' ', '\t']);
+                let part = ['"', '\'']
+                    .into_iter()
+                    .find_map(|quote| part.strip_prefix(quote)?.strip_suffix(quote))
+                    .unwrap_or(part);
+                let bare = !part.is_empty()
+                    && part
+                        .bytes()
+                        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-');
+                bare.then(|| part.to_owned())
+            })
+            .collect()
+    };
+    let unmarked = |line: &str| line.trim_start_matches('\u{feff}').to_owned();
+    let written = unmarked(text.lines().nth(line.checked_sub(1)?)?);
+    let name = parts(written.split_once('=')?.0)?;
+    let table = working
+        .lines()
+        .take(line - 1)
+        .filter_map(|above| {
+            let above = unmarked(above);
+            let above = above.split('#').next()?.trim_matches([' ', '\t']);
+            let inner = above.strip_prefix('[')?.strip_suffix(']')?;
+            (!inner.starts_with('[')).then(|| parts(inner)).flatten()
+        })
+        .last()
+        .unwrap_or_default();
+    let path = [table, name].concat();
+    let [table, name] = path.as_slice() else {
+        return None;
+    };
+    Key::ALL
+        .into_iter()
+        .find(|key| key.path() == (table.as_str(), name.as_str()))
+}
+
+/// Whether an item of a table is a table with a header of its own under
+/// it (`[data.more]`, `[[data.list]]`), and not a value of it: an array
+/// is written with a bracket too.
+fn is_subtable(working: &str, item: &toml::de::DeValue<'_>, span: std::ops::Range<usize>) -> bool {
+    let written = &working[span];
+    match item {
+        toml::de::DeValue::Table(_) => written.starts_with('['),
+        toml::de::DeValue::Array(items) => {
+            written.starts_with("[[")
+                && items
+                    .iter()
+                    .all(|item| matches!(item.get_ref(), toml::de::DeValue::Table(_)))
+        }
+        _ => false,
+    }
+}
+
+/// Where the line that `offset` is on begins.
+fn line_start(text: &str, offset: usize) -> usize {
+    text[..offset].rfind('\n').map_or(0, |at| at + 1)
+}
+
+/// Where the line after the one `offset` is on begins: past its break, or
+/// the end of the text.
+fn line_end(text: &str, offset: usize) -> usize {
+    text[offset..]
+        .find('\n')
+        .map_or(text.len(), |at| offset + at + 1)
+}
+
+/// Whether `range` of `text` is a line of its own (or lines): nothing but
+/// spaces and tabs before it on its first line, and nothing but those and
+/// a comment after it on its last.
+fn own_lines(text: &str, range: std::ops::Range<usize>) -> bool {
+    let before = &text[line_start(text, range.start)..range.start];
+    let after = &text[range.end..line_end(text, range.end)];
+    let after = after.trim_start_matches([' ', '\t']);
+    before.trim_matches([' ', '\t']).is_empty()
+        && (after.starts_with('#') || after.trim_matches(['\r', '\n']).is_empty())
+}
+
 impl Settings {
     pub const DEFAULT_PAGE_SIZE: u32 = 300;
     pub const MIN_PAGE_SIZE: u32 = 10;
@@ -534,7 +712,7 @@ impl Settings {
 
     /// `key`'s value as TOML writes it. `None` for a key the file leaves
     /// out: a theme that is not set.
-    fn literal(&self, key: Key) -> Option<String> {
+    fn written(&self, key: Key) -> Option<String> {
         Some(match key {
             Key::PageSize => self.page_size.to_string(),
             Key::Timestamps => toml_string(self.timestamps.name()),
@@ -551,42 +729,184 @@ impl Settings {
         })
     }
 
-    /// The file's text: the one form the app writes, the same every time.
-    /// Tables in a fixed order, a table's keys aligned, and a note beside
-    /// the keys that have one.
-    pub fn to_toml(&self) -> String {
-        let written: Vec<(Key, String)> = Key::ALL
+    /// `key`'s value as the file has it: `None` where it is the default,
+    /// which the file leaves out.
+    fn literal(&self, key: Key) -> Option<String> {
+        self.written(key)
+            .filter(|written| Some(written) != Settings::default().written(key).as_ref())
+    }
+
+    /// The keys whose value differs between `self` and `other`, in the
+    /// order of [`Key::ALL`]: what a write of `other` over `self` touches.
+    pub fn changed_keys(&self, other: &Settings) -> Vec<Key> {
+        Key::ALL
             .into_iter()
-            .filter_map(|key| Some((key, self.literal(key)?)))
+            .filter(|key| self.literal(*key) != other.literal(*key))
+            .collect()
+    }
+
+    /// The text of a file that holds nothing but these settings: the
+    /// header line, and a table for the keys that are not at their
+    /// default, in the order of [`Key::ALL`], each with its note.
+    pub fn to_toml(&self) -> String {
+        self.write_into("", &Key::ALL)
+    }
+
+    /// `text` with `keys` set to their values here, and nothing else of it
+    /// changed: the person's comments, the order of their keys, keys and
+    /// tables this version does not know, and the lines the reader ignored
+    /// all stay. A key at its default is taken out, with its line, and a
+    /// table left with nothing in it goes too. A key the file has gets its
+    /// new value in place, the comment beside it kept; one it lacks goes at
+    /// the end of its table, with its note, and a table it lacks at the end
+    /// of the file. An empty text starts as the header line.
+    ///
+    /// A table the file writes inline (`data = { page_size = 500 }`) or
+    /// with dotted keys can have a value replaced in place and nothing
+    /// more: a key to add or take out there gets the file written as
+    /// [`Settings::to_toml`] has it. So does a text too long to be read.
+    pub fn write_into(&self, text: &str, keys: &[Key]) -> String {
+        if text.len() > MAX_BYTES || text.lines().count() > MAX_LINES {
+            return self.to_toml();
+        }
+        let text = if text.trim_matches([' ', '\t', '\r', '\n']).is_empty() {
+            HEADER
+        } else {
+            text
+        };
+        let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
+        let (working, rejected) = readable(text);
+        // The keys the rejected lines would set: one of them taken out of
+        // the file would leave the rejected line to be read in its place.
+        let shadowed: Vec<Key> = rejected
+            .iter()
+            .filter_map(|line| key_on_line(text, &working, *line))
             .collect();
-        let mut out = String::from(HEADER);
-        let mut open = None;
-        for (key, literal) in &written {
-            let (table, name) = key.path();
-            if open != Some(table) {
-                if open.is_some() {
-                    out.push('\n');
+        let (document, _) = toml::de::DeTable::parse_recoverable(&working);
+        let document = document.into_inner();
+        // Replacements of ranges of the text, each with its place in the
+        // order they were found: inserts at one place keep that order.
+        let mut edits: Vec<(std::ops::Range<usize>, String)> = Vec::new();
+        let mut new_tables: Vec<(&str, String)> = Vec::new();
+        let mut emptied: Vec<&str> = Vec::new();
+        let line = |key: Key, literal: &str| {
+            let name = key.path().1;
+            match key.note() {
+                Some(note) => format!("{name} = {literal}  # {note}{newline}"),
+                None => format!("{name} = {literal}{newline}"),
+            }
+        };
+        for &key in keys {
+            let (table_name, name) = key.path();
+            let literal = self.literal(key);
+            let Some(table) = document.get(table_name) else {
+                if let Some(literal) = literal {
+                    new_tables.push((table_name, line(key, &literal)));
                 }
-                out.push_str(&format!("[{table}]\n"));
+                continue;
+            };
+            let header = table.span();
+            // A table with a header of its own, which lines can be added
+            // to and taken from.
+            let headed = working[header.clone()].starts_with('[');
+            let Some(items) = table.get_ref().as_table() else {
+                // The name is a value (`data = 5`) or a list of tables:
+                // nothing of ours can be written there, and nothing of the
+                // person's is taken to make room.
+                log::warn!("settings: {table_name} is not a table, {name} is not written");
+                continue;
+            };
+            // A table no header names, there because one under it is
+            // (`[data.more]`): a header of its own can follow.
+            let implicit = !headed
+                && !items.is_empty()
+                && items
+                    .iter()
+                    .all(|(_, item)| is_subtable(&working, item.get_ref(), item.span()));
+            match (items.get_key_value(name), literal) {
+                (Some((_, value)), Some(literal)) => edits.push((value.span(), literal)),
+                // The default in place: taken out, the rejected line of
+                // the same key would be the one read.
+                (Some((_, value)), None) if shadowed.contains(&key) => {
+                    if let Some(default) = Settings::default().written(key) {
+                        edits.push((value.span(), default));
+                    }
+                }
+                (Some((found, value)), None) => {
+                    let whole = found.span().start..value.span().end;
+                    if !headed || !own_lines(&working, whole.clone()) {
+                        return self.to_toml();
+                    }
+                    let lines = line_start(&working, whole.start)..line_end(&working, whole.end);
+                    edits.push((lines, String::new()));
+                    emptied.push(table_name);
+                }
+                (None, None) => {}
+                (None, Some(literal)) if implicit => {
+                    new_tables.push((table_name, line(key, &literal)));
+                }
+                (None, Some(literal)) => {
+                    if !headed {
+                        return self.to_toml();
+                    }
+                    // After the table's last key, or under its header. A
+                    // table under it (`[data.more]`) is no key of it.
+                    let last = items
+                        .iter()
+                        .filter(|(_, item)| !is_subtable(&working, item.get_ref(), item.span()))
+                        .map(|(_, item)| item.span().end)
+                        .max()
+                        .unwrap_or(header.end);
+                    let at = line_end(&working, last);
+                    let mut added = line(key, &literal);
+                    if !working[..at].ends_with('\n') {
+                        added.insert_str(0, newline);
+                    }
+                    edits.push((at..at, added));
+                }
+            }
+        }
+        let mut out = text.to_owned();
+        let mut order: Vec<usize> = (0..edits.len()).collect();
+        // From the end, so a range found in the text is where it was; at
+        // one place the last found goes in first, and the ones before it
+        // go in front of it.
+        order.sort_by(|a, b| (edits[*b].0.start, *b).cmp(&(edits[*a].0.start, *a)));
+        for index in order {
+            let (range, with) = &edits[index];
+            out.replace_range(range.clone(), with);
+        }
+        let emptied_any = !emptied.is_empty();
+        for table in emptied {
+            out = without_empty_table(&out, table);
+        }
+        if emptied_any {
+            let kept = out.trim_end_matches(['\r', '\n']).len();
+            out.truncate(kept);
+            // A file left with nothing in it stays empty.
+            if !out.is_empty() {
+                out.push_str(newline);
+            }
+        }
+        let mut open: Option<&str> = None;
+        for (table, added) in new_tables {
+            if open != Some(table) {
+                if !out.ends_with('\n') {
+                    out.push_str(newline);
+                }
+                if out != HEADER {
+                    out.push_str(newline);
+                }
+                out.push_str(&format!("[{table}]{newline}"));
                 open = Some(table);
             }
-            let width = written
-                .iter()
-                .filter(|(other, _)| other.path().0 == table)
-                .map(|(other, _)| other.path().1.len())
-                .max()
-                .unwrap_or(0);
-            out.push_str(&format!("{name:width$} = {literal}"));
-            if let Some(note) = key.note() {
-                out.push_str(&format!("  # {note}"));
-            }
-            out.push('\n');
+            out.push_str(&added);
         }
         out
     }
 
     /// The settings with every number in its range.
-    fn validated(mut self) -> Self {
+    pub fn validated(mut self) -> Self {
         self.page_size = self
             .page_size
             .clamp(Self::MIN_PAGE_SIZE, Self::MAX_PAGE_SIZE);
@@ -638,6 +958,18 @@ impl Settings {
     /// settings file can be (`MAX_LINES`, `MAX_BYTES`) is not read at all:
     /// every line of it that says something is one that was ignored.
     pub fn from_toml(text: &str) -> Loaded {
+        Self::read(text, &Settings::default())
+    }
+
+    /// Reads a new text of the file over the settings it last gave: as
+    /// [`Settings::from_toml`], but a key whose line is ignored keeps its
+    /// value in `last`, its last good one, and not its default. A key the
+    /// text leaves out takes its default: it was taken out.
+    pub fn from_toml_over(text: &str, last: &Settings) -> Loaded {
+        Self::read(text, last)
+    }
+
+    fn read(text: &str, last: &Settings) -> Loaded {
         if text.len() > MAX_BYTES || text.lines().count() > MAX_LINES {
             let invalid = text
                 .lines()
@@ -646,71 +978,54 @@ impl Settings {
                 .map(|(index, _)| index + 1)
                 .collect();
             return Loaded {
-                settings: Settings::default(),
+                settings: last.clone(),
                 text: text.to_owned(),
                 invalid,
                 lines: Vec::new(),
                 source: Source::Toml,
             };
         }
-        // The parser recovers from a bad line in two ways that are not
-        // "ignored": it stops reading at some (a key with no `=`), and
-        // reads past others with a guess at the value (`full` unquoted).
-        // So a line it rejects is emptied and the text read again, until
-        // it rejects nothing more.
-        let mut working = text.to_owned();
-        let mut invalid: Vec<usize> = Vec::new();
-        loop {
-            let (_, errors) = toml::de::DeTable::parse_recoverable(&working);
-            let mut rejected = Vec::new();
-            for error in &errors {
-                match error.span() {
-                    // The parser blames the end of the text for a value
-                    // that never closes (`"""` with no end). That is no
-                    // line of the file, so the last line with anything on
-                    // it goes: the lines are emptied from the bottom up
-                    // until the opening one is. With every line blank the
-                    // error marks none.
-                    Some(span) if span.start >= working.len() => {
-                        rejected.extend(last_line(&working));
-                    }
-                    Some(span) => rejected.push(line_of(&working, span.start)),
-                    None => log::warn!("settings: {}", error.message()),
-                }
-            }
-            // A line already emptied has nothing left to reject: an error
-            // that still names one ends the loop.
-            rejected.retain(|line| !invalid.contains(line));
-            rejected.sort_unstable();
-            rejected.dedup();
-            if rejected.is_empty() {
-                break;
-            }
-            working = without_lines(&working, &rejected);
-            invalid.extend(rejected);
-        }
+        let (working, mut invalid) = readable(text);
         let (document, _) = toml::de::DeTable::parse_recoverable(&working);
         let document = document.into_inner();
         let mut settings = Settings::default();
         let mut lines = Vec::new();
+        // The keys the rejected lines would have set, as far as a line
+        // says it: those keep their last good value, unless a line that
+        // was read sets them.
+        let mut kept: Vec<Key> = invalid
+            .iter()
+            .filter_map(|line| key_on_line(text, &working, *line))
+            .collect();
         for key in Key::ALL {
             let (table, name) = key.path();
-            let Some((found, value)) = document
-                .get(table)
+            let place = document.get(table);
+            // A table's name the file gives a value (`data = 5`): its keys
+            // cannot be read or written there, and keep what they had.
+            if place.is_some_and(|place| place.get_ref().as_table().is_none()) {
+                kept.push(key);
+                continue;
+            }
+            let Some((found, value)) = place
                 .and_then(|table| table.get_ref().as_table())
                 .and_then(|table| table.get_key_value(name))
             else {
                 continue;
             };
-            // Emptied lines kept their breaks: a line has the same number
+            // Blanked lines kept their breaks: a line has the same number
             // in `working` as in `text`.
             let line = line_of(&working, found.span().start);
             if settings.take(key, value.get_ref()) {
                 lines.push((key, line));
+                kept.retain(|other| *other != key);
             } else {
                 invalid.push(line);
+                kept.push(key);
             }
         }
+        for key in kept {
+            settings.keep(key, last);
+        }
         invalid.sort_unstable();
         invalid.dedup();
         Loaded {
@@ -722,6 +1037,23 @@ impl Settings {
         }
     }
 
+    /// Takes `key`'s value from `other`.
+    fn keep(&mut self, key: Key, other: &Settings) {
+        match key {
+            Key::PageSize => self.page_size = other.page_size,
+            Key::Timestamps => self.timestamps = other.timestamps,
+            Key::GroupDigits => self.group_digits = other.group_digits,
+            Key::ValueTags => self.value_tags = other.value_tags,
+            Key::ShowSystemSchemas => self.show_system_schemas = other.show_system_schemas,
+            Key::SqlLimit => self.sql_limit = other.sql_limit,
+            Key::SqlTimeoutSecs => self.sql_timeout_secs = other.sql_timeout_secs,
+            Key::McpEnabled => self.mcp_enabled = other.mcp_enabled,
+            Key::McpLogDays => self.mcp_log_days = other.mcp_log_days,
+            Key::McpNotify => self.mcp_notify = other.mcp_notify,
+            Key::Theme => self.custom_theme = other.custom_theme.clone(),
+        }
+    }
+
     /// The settings a start has: from `settings.toml`, else from an older
     /// version's `settings.json`, else the defaults. It writes no
     /// settings (a file it cannot use is moved aside); what came from the
@@ -759,11 +1091,6 @@ impl Settings {
             }
         }
     }
-
-    /// Writes the canonical text, atomically.
-    pub fn save(&self, path: &Path) -> std::io::Result<()> {
-        crate::util::write_atomic(path, self.to_toml().as_bytes())
-    }
 }
 
 impl Default for Settings {
````

- [ ] **Step 4: Check**

Run: `cargo fmt --all && cargo clippy --locked --workspace --all-targets -- -D warnings && cargo test --locked --workspace --all-targets`
Expected: no warning, every test passes.

- [ ] **Step 5: Commit**

```bash
git add docs/_reference/settings-and-files.md src/app.rs src/backend.rs src/settings.rs src/ui/mod.rs
git commit -m "Write only the settings that changed into settings.toml, in place" \
  -m "A value the file gets wrong keeps its last good one, and Export saves a copy of the file as the app holds it."
```

### Task 2: A new page size lands on the next load

The artboard's sizes and default. A table tab keeps the size it pages by; a change of the option fetches nothing.

**Files:** Modify `docs/_guide/browsing-data.md`; Modify `docs/_reference/settings-and-files.md`; Modify `src/app.rs`; Modify `src/model.rs`; Modify `src/settings.rs`; Modify `src/ui/mod.rs`; Modify `src/ui/settings/sheet.rs`.

- [ ] **Step 1: Write the failing tests**

````diff
diff --git a/src/app.rs b/src/app.rs
index 4931e8f..b81e29f 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -9996,12 +9996,13 @@ mod tests {
         let mut harness = Harness::new();
         let tab = connect_tab(&mut harness);
         let id = open(&mut harness, tab, "users", false);
-        harness.answer_rows(page(300, true));
+        let size = Settings::DEFAULT_PAGE_SIZE;
+        harness.answer_rows(page(size as usize, true));
         harness.app.apply(Action::NextPage {
             tab,
             object_tab: id,
         });
-        assert_eq!(last_query(&harness).offset, 300);
+        assert_eq!(last_query(&harness).offset, u64::from(size));
         assert!(object(&harness, tab, id).pinned);
         harness.answer_rows(page(10, false));
         let sent = harness.app.backend.sent.len();
@@ -14863,21 +14864,17 @@ mod tests {
                 ..harness.app.settings.clone()
             };
             harness.app.apply_settings(settings);
-            // Only the tab without edits is fetched again.
-            let fetched: Vec<&ObjectRef> = harness.app.backend.sent[before..]
-                .iter()
-                .filter_map(|command| match command {
-                    Command::FetchRows { query, .. } => Some(&query.object),
-                    _ => None,
-                })
-                .collect();
-            assert_eq!(fetched, [&ObjectRef::new("main", "orders")]);
-            assert_eq!(object(&harness, tab, orders).query.limit, 500);
+            // No tab is fetched again: each takes the size when it loads.
+            assert!(
+                !harness.app.backend.sent[before..]
+                    .iter()
+                    .any(|command| matches!(command, Command::FetchRows { .. }))
+            );
+            assert_ne!(object(&harness, tab, orders).query.limit, 500);
             let users_tab = object(&harness, tab, id);
             assert_eq!(users_tab.edits.cells.len(), 1);
             assert_eq!(users_tab.page().unwrap().rows.len(), 5);
-            assert_ne!(users_tab.query.limit, 500);
-            // It takes the new size at its next fetch.
+            // It takes the new size at its next load.
             harness.app.apply(Action::DiscardEdits { tab, id });
             harness.app.apply(Action::ActivateTab { tab, id });
             harness.app.apply(Action::Refresh(tab));
diff --git a/src/settings.rs b/src/settings.rs
index 8fc10e2..78ac4ce 100644
--- a/src/settings.rs
+++ b/src/settings.rs
@@ -1118,7 +1118,7 @@ mod tests {
     #[test]
     fn defaults_match_the_spec() {
         let settings = Settings::default();
-        assert_eq!(settings.page_size, 300);
+        assert_eq!(settings.page_size, 1_000);
         assert_eq!(settings.timestamps, Timestamps::Second);
         assert!(!settings.group_digits);
         assert!(settings.value_tags);
@@ -2117,14 +2117,14 @@ theme = \"My \\\"Nord\\\".json\"
             };
             settings.stepped(OptionId::PageSize, forward)
         };
+        assert_eq!(step(1_000, true), OptionValue::PageSize(5_000));
+        assert_eq!(step(500, false), OptionValue::PageSize(100));
+        assert_eq!(step(100, false), OptionValue::PageSize(100));
+        assert_eq!(step(10_000, true), OptionValue::PageSize(10_000));
+        // A size set by hand is between two choices: it steps to the next.
         assert_eq!(step(300, true), OptionValue::PageSize(500));
         assert_eq!(step(300, false), OptionValue::PageSize(100));
-        assert_eq!(step(100, false), OptionValue::PageSize(100));
-        assert_eq!(step(5_000, true), OptionValue::PageSize(5_000));
-        // A size set by hand is between two choices: it steps to the next.
-        assert_eq!(step(250, true), OptionValue::PageSize(300));
-        assert_eq!(step(250, false), OptionValue::PageSize(100));
-        assert_eq!(step(9_000, true), OptionValue::PageSize(9_000));
+        assert_eq!(step(9_000, true), OptionValue::PageSize(10_000));
         assert_eq!(step(9_000, false), OptionValue::PageSize(5_000));
     }
 
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
index 045f90b..ae4ecab 100644
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -614,8 +614,18 @@ mod tests {
             .expect("a FetchRows was sent")
     }
 
+    /// `harness`'s settings with Rows per page at `size`.
+    fn page_size_set(harness: &mut Harness, size: u32) {
+        let settings = crate::settings::Settings {
+            page_size: size,
+            ..harness.app.settings.clone()
+        };
+        harness.app.apply_settings(settings);
+    }
+
     #[test]
-    fn a_new_page_size_fetches_an_open_page_again_from_where_it_is() {
+    fn a_new_page_size_lands_on_the_next_load_and_not_on_the_page_shown() {
+        let size = crate::settings::Settings::DEFAULT_PAGE_SIZE;
         let mut harness = Harness::new();
         let tab = harness.connect_fake();
         harness.click("users");
@@ -624,80 +634,79 @@ mod tests {
         harness
             .app
             .apply(crate::model::Action::NextPage { tab, object_tab });
-        assert_eq!(last_fetch(&harness), (300, 300));
+        assert_eq!(last_fetch(&harness), (u64::from(size), size));
         harness.answer_rows(crate::testing::page(3, true));
         let before = fetches(&harness);
-        let settings = crate::settings::Settings {
-            page_size: 500,
-            ..harness.app.settings.clone()
-        };
-        harness.app.apply_settings(settings);
-        // The same offset, the new size.
-        assert_eq!(fetches(&harness), before + 1);
-        assert_eq!(last_fetch(&harness), (300, 500));
-        // The page of the old size is not left on screen meanwhile: were
-        // this fetch to fail, Next would move past it by the new size.
+        page_size_set(&mut harness, 500);
+        // Nothing is fetched, and the page stays where it is.
+        assert_eq!(fetches(&harness), before);
         let object = harness
             .app
             .workspace(tab)
             .unwrap()
             .active_object_tab()
             .unwrap();
-        assert!(object.page().is_none());
-        harness.answer_rows(crate::testing::page(3, true));
-        // Next moves by the size of the page that is shown: no row skipped.
+        assert!(object.page().is_some());
+        // Paging keeps the size the table loaded with.
         harness
             .app
             .apply(crate::model::Action::NextPage { tab, object_tab });
-        assert_eq!(last_fetch(&harness), (800, 500));
+        assert_eq!(last_fetch(&harness), (2 * u64::from(size), size));
+        harness.answer_rows(crate::testing::page(3, true));
+        // A refresh is a load: from where it is, at the new size, which
+        // Next then moves by.
+        harness.app.apply(crate::model::Action::Refresh(tab));
+        assert_eq!(last_fetch(&harness), (2 * u64::from(size), 500));
+        harness.answer_rows(crate::testing::page(3, true));
+        harness
+            .app
+            .apply(crate::model::Action::NextPage { tab, object_tab });
+        assert_eq!(last_fetch(&harness), (2 * u64::from(size) + 500, 500));
     }
 
     #[test]
-    fn a_page_still_on_its_way_is_fetched_again_at_the_new_size() {
+    fn a_page_still_on_its_way_is_left_to_land() {
+        let size = crate::settings::Settings::DEFAULT_PAGE_SIZE;
         let mut harness = Harness::new();
-        harness.connect_fake();
+        let tab = harness.connect_fake();
         harness.click("users");
         // Not answered: the first page is in flight at the old size.
         let before = fetches(&harness);
-        let settings = crate::settings::Settings {
-            page_size: 500,
-            ..harness.app.settings.clone()
-        };
-        harness.app.apply_settings(settings);
-        assert_eq!(fetches(&harness), before + 1);
-        assert_eq!(last_fetch(&harness), (0, 500));
+        page_size_set(&mut harness, 500);
+        assert_eq!(fetches(&harness), before);
+        let object = harness
+            .app
+            .workspace(tab)
+            .unwrap()
+            .active_object_tab()
+            .unwrap();
+        assert!(object.rows.pending.is_some());
+        assert_eq!(object.query.limit, size);
     }
 
     #[test]
-    fn a_table_whose_session_is_down_waits_for_its_next_fetch() {
+    fn a_table_opened_or_sorted_after_a_new_page_size_loads_at_it() {
         let mut harness = Harness::new();
         let tab = harness.connect_fake();
         harness.click("users");
         harness.answer_rows(crate::testing::page(3, true));
-        harness.app.workspace_mut(tab).unwrap().status =
-            crate::model::SessionStatus::Disconnected(tabletist_db::Error::query("gone"));
-        let before = fetches(&harness);
-        let settings = crate::settings::Settings {
-            page_size: 500,
-            ..harness.app.settings.clone()
-        };
-        harness.app.apply_settings(settings);
-        assert_eq!(
-            fetches(&harness),
-            before,
-            "nothing is asked of a dead session"
-        );
-        // Its next fetch moves by the page it shows and asks for the new size.
-        harness.app.workspace_mut(tab).unwrap().status = crate::model::SessionStatus::Connected;
+        page_size_set(&mut harness, 500);
+        harness.click("orders");
+        assert_eq!(last_fetch(&harness), (0, 500));
+        harness.answer_rows(crate::testing::page(3, true));
+        page_size_set(&mut harness, 100);
         let object_tab = harness.app.workspace(tab).unwrap().active_tab.unwrap();
-        harness
-            .app
-            .apply(crate::model::Action::NextPage { tab, object_tab });
-        assert_eq!(last_fetch(&harness), (300, 500));
+        harness.app.apply(crate::model::Action::SortBy {
+            tab,
+            object_tab,
+            column: "id".into(),
+        });
+        assert_eq!(last_fetch(&harness), (0, 100));
     }
 
     #[test]
-    fn previous_moves_by_the_size_of_the_page_it_fetches() {
+    fn previous_moves_by_the_size_the_table_pages_by() {
+        let size = crate::settings::Settings::DEFAULT_PAGE_SIZE;
         let mut harness = Harness::new();
         let tab = harness.connect_fake();
         harness.click("users");
@@ -709,26 +718,17 @@ mod tests {
                 .apply(crate::model::Action::NextPage { tab, object_tab });
             harness.answer_rows(crate::testing::page(3, true));
         }
-        assert_eq!(last_fetch(&harness), (900, 300));
-        // The size changes while the session is down: the page keeps its own.
-        harness.app.workspace_mut(tab).unwrap().status =
-            crate::model::SessionStatus::Disconnected(tabletist_db::Error::query("gone"));
-        let settings = crate::settings::Settings {
-            page_size: 100,
-            ..harness.app.settings.clone()
-        };
-        harness.app.apply_settings(settings);
-        harness.app.workspace_mut(tab).unwrap().status = crate::model::SessionStatus::Connected;
+        assert_eq!(last_fetch(&harness), (3 * u64::from(size), size));
+        page_size_set(&mut harness, 100);
         harness
             .app
             .apply(crate::model::Action::PrevPage { tab, object_tab });
-        // The page fetched has the new size and ends where the one that was
-        // shown begins. Back by the old size, rows 700 to 899 are skipped.
-        assert_eq!(last_fetch(&harness), (800, 100));
+        assert_eq!(last_fetch(&harness), (2 * u64::from(size), size));
     }
 
     #[test]
     fn previous_near_the_start_fetches_only_the_rows_before_the_page() {
+        let size = crate::settings::Settings::DEFAULT_PAGE_SIZE;
         let mut harness = Harness::new();
         let tab = harness.connect_fake();
         harness.click("users");
@@ -737,29 +737,27 @@ mod tests {
         harness
             .app
             .apply(crate::model::Action::NextPage { tab, object_tab });
-        assert_eq!(last_fetch(&harness), (300, 300));
+        assert_eq!(last_fetch(&harness), (u64::from(size), size));
         harness.answer_rows(crate::testing::page(3, true));
-        // The size grows: the page on screen is rows 300 to 799.
-        let settings = crate::settings::Settings {
-            page_size: 500,
-            ..harness.app.settings.clone()
-        };
-        harness.app.apply_settings(settings);
-        assert_eq!(last_fetch(&harness), (300, 500));
+        // The size grows and the table is refreshed: the page on screen
+        // starts a page of the old size from the start.
+        page_size_set(&mut harness, 5 * size);
+        harness.app.apply(crate::model::Action::Refresh(tab));
+        assert_eq!(last_fetch(&harness), (u64::from(size), 5 * size));
         harness.answer_rows(crate::testing::page(3, true));
         harness
             .app
             .apply(crate::model::Action::PrevPage { tab, object_tab });
-        // Only 300 rows are before that page. A whole page from the start
-        // would show rows 300 to 499 a second time.
-        assert_eq!(last_fetch(&harness), (0, 300));
+        // Only a page of the old size is before it. A whole page from the
+        // start would show its first rows a second time.
+        assert_eq!(last_fetch(&harness), (0, size));
         harness.answer_rows(crate::testing::page(3, true));
-        // Next moves by the short page, back to where the user was, and the
-        // page fetched there has the settings' size again.
+        // Next moves by the short page, back to where the user was, and
+        // the page fetched there has the table's size again.
         harness
             .app
             .apply(crate::model::Action::NextPage { tab, object_tab });
-        assert_eq!(last_fetch(&harness), (300, 500));
+        assert_eq!(last_fetch(&harness), (u64::from(size), 5 * size));
     }
 
     #[test]
@@ -814,22 +812,10 @@ mod tests {
     #[test]
     fn an_option_that_is_not_the_page_size_fetches_no_page_again() {
         let mut harness = Harness::new();
-        let tab = harness.connect_fake();
+        harness.connect_fake();
         harness.click("users");
         harness.answer_rows(crate::testing::page(3, true));
-        // A page of the old size on a session that answers again: the size
-        // changed while it was down, and nothing was fetched since.
-        harness.app.workspace_mut(tab).unwrap().status =
-            crate::model::SessionStatus::Disconnected(tabletist_db::Error::query("gone"));
-        let settings = crate::settings::Settings {
-            page_size: 500,
-            ..harness.app.settings.clone()
-        };
-        harness.app.apply_settings(settings);
-        harness.app.workspace_mut(tab).unwrap().status = crate::model::SessionStatus::Connected;
         let before = fetches(&harness);
-        // A new page size would fetch this page again. Another option is no
-        // reason to.
         let settings = crate::settings::Settings {
             group_digits: true,
             ..harness.app.settings.clone()
@@ -1327,7 +1313,7 @@ value_tags   = true
             crate::settings::OptionValue::PageSize(250),
         ));
         harness.click("Rows per page");
-        for size in ["100", "250", "300", "500", "1,000", "5,000"] {
+        for size in ["100", "250", "500", "1,000", "5,000", "10,000"] {
             let entry = format!("Rows per page {size}");
             assert!(harness.has(&entry), "{entry}");
         }
@@ -1587,7 +1573,7 @@ value_tags   = true
             harness.press(egui::Key::Escape, egui::Modifiers::NONE);
             assert!(settings_open(&harness), "the menu closes first");
             assert!(!harness.has("Rows per page 500"));
-            assert_eq!(harness.app.settings.page_size, 300);
+            assert_eq!(harness.app.settings.page_size, 1_000);
             harness.press(egui::Key::Escape, egui::Modifiers::NONE);
             assert!(harness.app.dialog.is_none());
         }
@@ -1692,10 +1678,11 @@ value_tags   = true
         let mut harness = settings_screen();
         // Rows per page.
         harness.press(egui::Key::L, egui::Modifiers::NONE);
-        assert_eq!(harness.app.settings.page_size, 500);
+        assert_eq!(harness.app.settings.page_size, 5_000);
         assert_eq!(settings_saves(&harness), 1);
         harness.press(egui::Key::ArrowLeft, egui::Modifiers::NONE);
         harness.press(egui::Key::H, egui::Modifiers::NONE);
+        harness.press(egui::Key::H, egui::Modifiers::NONE);
         assert_eq!(harness.app.settings.page_size, 100);
         // At the end of the choices a step changes and writes nothing.
         let saved = settings_saves(&harness);
@@ -1798,7 +1785,7 @@ value_tags   = true
         harness.click("Value tags: off");
         assert!(!harness.app.settings.value_tags);
         harness.click("Rows per page: more");
-        assert_eq!(harness.app.settings.page_size, 500);
+        assert_eq!(harness.app.settings.page_size, 5_000);
     }
 
     #[test]
@@ -1856,15 +1843,15 @@ value_tags   = true
         harness.press(egui::Key::J, egui::Modifiers::NONE);
         // No chevron is drawn on that row: the number starts where the
         // cursor's row has one.
-        let number = harness.painted_rect("300").expect("the page size");
+        let number = harness.painted_rect("1000").expect("the page size");
         click_at(&mut harness, number.left_center() + egui::vec2(1.0, 0.0));
         assert_eq!(settings_cursor(&harness), 0);
-        assert_eq!(harness.app.settings.page_size, 300);
+        assert_eq!(harness.app.settings.page_size, 1_000);
         assert_eq!(settings_saves(&harness), 0);
         // On the cursor's row the chevrons are drawn, and each takes a click.
-        let stepper = harness.painted_rect("‹ 300 ›").expect("the chevrons");
+        let stepper = harness.painted_rect("‹ 1000 ›").expect("the chevrons");
         click_at(&mut harness, stepper.left_center() + egui::vec2(1.0, 0.0));
-        assert_eq!(harness.app.settings.page_size, 100);
+        assert_eq!(harness.app.settings.page_size, 500);
     }
 
     #[test]
@@ -2236,7 +2223,7 @@ value_tags   = true
         let tree = harness.settle();
         let row = crate::testing::node(&tree, "Rows per page", Role::Button).expect("the row");
         let (_, row) = tree.nodes.iter().find(|(node, _)| *node == row).unwrap();
-        assert_eq!(row.value(), Some("300"));
+        assert_eq!(row.value(), Some("1000"));
     }
 
     /// Tabs until the control named `name` has the keyboard.
@@ -3766,7 +3753,7 @@ value_tags   = true
         harness.answer_rows(crate::testing::page(300, true));
         harness.click("Next page");
         match crate::testing::last_sent(&harness.app) {
-            Command::FetchRows { query, .. } => assert_eq!(query.offset, 300),
+            Command::FetchRows { query, .. } => assert_eq!(query.offset, 1_000),
             other => panic!("{other:?}"),
         }
     }
@@ -7295,7 +7282,7 @@ value_tags   = true
         harness.answer_rows(crate::testing::page(300, true));
         harness.press(Key::ArrowRight, Modifiers::COMMAND | Modifiers::ALT);
         match crate::testing::last_sent(&harness.app) {
-            Command::FetchRows { query, .. } => assert_eq!(query.offset, 300),
+            Command::FetchRows { query, .. } => assert_eq!(query.offset, 1_000),
             other => panic!("{other:?}"),
         }
         harness.press(Key::Period, Modifiers::COMMAND);
diff --git a/src/ui/settings/sheet.rs b/src/ui/settings/sheet.rs
index 1c26c50..7c33ed2 100644
--- a/src/ui/settings/sheet.rs
+++ b/src/ui/settings/sheet.rs
@@ -900,8 +900,8 @@ mod tests {
     fn a_size_that_is_not_in_the_list_takes_its_place_among_the_others() {
         assert_eq!(log_days(30), Settings::LOG_DAYS);
         assert_eq!(log_days(14), [7, 14, 30, 90, 365]);
-        assert_eq!(page_sizes(300), Settings::PAGE_SIZES);
-        assert_eq!(page_sizes(250), [100, 250, 300, 500, 1_000, 5_000]);
-        assert_eq!(page_sizes(9_000), [100, 300, 500, 1_000, 5_000, 9_000]);
+        assert_eq!(page_sizes(1_000), Settings::PAGE_SIZES);
+        assert_eq!(page_sizes(300), [100, 300, 500, 1_000, 5_000, 10_000]);
+        assert_eq!(page_sizes(20), [20, 100, 500, 1_000, 5_000, 10_000]);
     }
 }
````

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --locked -p tabletist --lib`
Expected: FAIL: 15 tests (`app::tests::editing::a_new_page_size_leaves_a_tab_with_pending_changes_alone`, `settings::tests::defaults_match_the_spec`, `settings::tests::the_page_size_steps_through_its_choices_and_stops_at_the_ends`, `ui::settings::sheet::tests::a_size_that_is_not_in_the_list_takes_its_place_among_the_others`, `ui::tests::a_click_moves_the_settings_cursor_and_a_click_on_a_value_sets_it`, `ui::tests::a_click_on_another_rows_page_size_only_moves_the_settings_cursor`, `ui::tests::a_new_page_size_lands_on_the_next_load_and_not_on_the_page_shown`, `ui::tests::a_page_still_on_its_way_is_left_to_land`, `ui::tests::a_page_size_from_the_file_that_is_not_in_the_list_is_in_the_menu`, `ui::tests::a_screen_reader_hears_which_value_of_an_option_is_set`, `ui::tests::escape_closes_the_page_size_menu_before_the_settings_sheet`, `ui::tests::h_and_l_change_the_cursors_option_and_save_it`, `ui::tests::header_clicks_sort_and_the_next_page_button_pages`, `ui::tests::object_tab_and_page_shortcuts_work`, `ui::tests::previous_moves_by_the_size_the_table_pages_by`).

- [ ] **Step 3: Write the code**

````diff
diff --git a/docs/_guide/browsing-data.md b/docs/_guide/browsing-data.md
index ba0feed..0868a27 100644
--- a/docs/_guide/browsing-data.md
+++ b/docs/_guide/browsing-data.md
@@ -27,7 +27,7 @@ tags.
 
 {% include shot.html file="macos-table" alt="A table's data grid sorted by a column, with a row open in the row panel beside it" %}
 
-- **Pages.** The grid loads 300 rows at a time unless you change
+- **Pages.** The grid loads 1,000 rows at a time unless you change
   [Rows per page]({% link _reference/settings-and-files.md %}#the-settings-window)
   in Settings. Cmd/Ctrl+Alt+Left and Right move between pages.
 - **Sorting.** Click a column's header to sort by it: ascending, then
diff --git a/docs/_reference/settings-and-files.md b/docs/_reference/settings-and-files.md
index 22d4467..9f6af8b 100644
--- a/docs/_reference/settings-and-files.md
+++ b/docs/_reference/settings-and-files.md
@@ -12,7 +12,7 @@ Cmd/Ctrl+, opens Settings. Changes apply right away.
 
 | Option | Choices | Default |
 | --- | --- | --- |
-| **Rows per page** | 100, 300, 500, 1,000 or 5,000 | 300 |
+| **Rows per page** | 100, 500, 1,000, 5,000 or 10,000. A table takes a new size when it next loads, not while you page through it | 1,000 |
 | **Timestamps** | To the second, or full precision | To the second |
 | **Numbers** | `1240.50` or `1,240.50`. Grouping is for display only: a copy gives the raw value | `1240.50` |
 | **Value tags** | Colors for enum, CHECK and boolean columns | On |
@@ -44,7 +44,7 @@ is every key at its default:
 
 ```toml
 [data]
-page_size = 300
+page_size = 1000
 timestamps = "second"
 group_digits = false
 value_tags = true
@@ -64,7 +64,7 @@ notify = true
 
 | Key | Values | Default |
 | --- | --- | --- |
-| `data.page_size` | 10 to 10000 | `300` |
+| `data.page_size` | 10 to 10000. A table takes a new size when it next loads (opened, refreshed, sorted, filtered or reconnected), not while you page through it | `1000` |
 | `data.timestamps` | `"second"` or `"full"` | `"second"` |
 | `data.group_digits` | `true` or `false` | `false` |
 | `data.value_tags` | `true` or `false` | `true` |
diff --git a/src/app.rs b/src/app.rs
index b81e29f..8d051b9 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -931,24 +931,24 @@ impl App {
                     true
                 });
                 if moved {
-                    self.fetch_rows(tab, object_tab);
+                    self.fetch_next_page(tab, object_tab);
                 }
             }
             Action::PrevPage { tab, object_tab } => {
                 // Back by the size of the page about to be fetched, which
                 // must end where the one on screen begins. That one may be
-                // of another size (the size changed while its session was
-                // down): back by its limit, the rows between the two would
-                // be skipped. The page fetched has the settings' size, or
-                // is only the rows before this one when they are fewer (the
-                // size grew on a page near the start): a whole page from
+                // of another size (a short page near the start): back by
+                // its limit, the rows between the two would be skipped. The
+                // page fetched has the tab's size, or is only the rows
+                // before this one when they are fewer (the tab loaded a
+                // larger size on a page near the start): a whole page from
                 // the start would show the first rows of this one a second
                 // time, and Next from it would not come back here.
-                let page_size = self.settings.page_size;
                 let step = self.object_tab_mut(tab, object_tab).and_then(|object| {
                     if object.query.offset == 0 {
                         return None;
                     }
+                    let page_size = object.page_size;
                     let step = u32::try_from(object.query.offset)
                         .map_or(page_size, |before| before.min(page_size));
                     object.query.offset -= u64::from(step);
@@ -3632,9 +3632,6 @@ impl App {
                 }
             }
         }
-        if old.page_size != self.settings.page_size {
-            self.resize_pages();
-        }
         if old.show_system_schemas != self.settings.show_system_schemas {
             // A completion list offers the schemas that are shown, and an
             // open one is worked out again only when its script, its cursor
@@ -3665,37 +3662,6 @@ impl App {
         }
     }
 
-    /// Fetches again, at the settings' page size, every table that shows a
-    /// page or waits for one on a session that can answer. `fetch_rows`
-    /// drops the page of the old size as it takes the new one. The others,
-    /// and a table that holds edits, take the size, and lose their page
-    /// the same way, when they next fetch.
-    fn resize_pages(&mut self) {
-        let size = self.settings.page_size;
-        let mut again = Vec::new();
-        for tab in &self.tabs {
-            let ConnTabContent::Workspace(workspace) = &tab.content else {
-                continue;
-            };
-            if !matches!(workspace.status, SessionStatus::Connected) {
-                continue;
-            }
-            again.extend(
-                workspace
-                    .object_tabs()
-                    .filter(|object| object.query.limit != size)
-                    .filter(|object| object.page().is_some() || object.rows.pending.is_some())
-                    // A page with pending changes stays: the tab takes the
-                    // size at its next fetch.
-                    .filter(|object| !object.edits.holds())
-                    .map(|object| (tab.id, object.id)),
-            );
-        }
-        for (tab, id) in again {
-            self.fetch_rows(tab, id);
-        }
-    }
-
     fn save_dialog(&mut self, connect: bool) {
         // The tables in force now, of the connection being edited.
         let tables_now = match &self.dialog {
@@ -4814,16 +4780,31 @@ impl App {
         self.cancel(session, running);
     }
 
-    /// Loads the object tab's rows at the settings' page size, replacing
-    /// any load still pending. A page of another size does not stay on
-    /// screen meanwhile, whatever the caller kept.
+    /// Loads the object tab's rows from where it is, at the settings' page
+    /// size, which the tab pages by from then on, replacing any load still
+    /// pending. A page of another size does not stay on screen meanwhile,
+    /// whatever the caller kept. Every load but Next's and Previous's comes
+    /// here: a new Rows per page lands on the next one.
     pub fn fetch_rows(&mut self, tab: ConnTabId, id: TabId) {
-        self.fetch_page(tab, id, self.settings.page_size);
+        let size = self.settings.page_size;
+        if let Some(object) = self.object_tab_mut(tab, id) {
+            object.page_size = size;
+        }
+        self.fetch_page(tab, id, size);
+    }
+
+    /// Loads the page after the one on screen, at the size the tab pages
+    /// by: Next has moved past that page by its limit.
+    fn fetch_next_page(&mut self, tab: ConnTabId, id: TabId) {
+        let Some(size) = self.object_tab_mut(tab, id).map(|object| object.page_size) else {
+            return;
+        };
+        self.fetch_page(tab, id, size);
     }
 
     /// Loads a page of `limit` rows, as [`App::fetch_rows`] does at the
-    /// settings' size. Only Previous asks for another: the rows before a
-    /// page that begins less than a page from the start.
+    /// settings' size. Previous asks for the tab's size, or for the rows
+    /// before a page that begins less than a page from the start.
     fn fetch_page(&mut self, tab: ConnTabId, id: TabId, limit: u32) {
         let request = RequestId(self.next_id());
         let Some(workspace) = self.workspace_mut(tab) else {
diff --git a/src/model.rs b/src/model.rs
index 9458560..5aba7cf 100644
--- a/src/model.rs
+++ b/src/model.rs
@@ -2708,6 +2708,10 @@ pub struct ObjectTab {
     pub pinned: bool,
     pub view: ObjectView,
     pub query: RowQuery,
+    /// The size the tab pages by: the settings' Rows per page when it last
+    /// loaded from a place (opened, refreshed, sorted, filtered). Next and
+    /// Previous keep it, so a new size never lands while the person pages.
+    pub page_size: u32,
     pub rows: Fetch<RowPage>,
     pub structure: Fetch<Structure>,
     pub selection: Option<CellPos>,
@@ -2942,6 +2946,7 @@ impl ObjectTab {
         Self {
             id,
             query: RowQuery::new(object.clone(), page_size),
+            page_size,
             object,
             kind,
             pinned,
diff --git a/src/settings.rs b/src/settings.rs
index 78ac4ce..cce057a 100644
--- a/src/settings.rs
+++ b/src/settings.rs
@@ -598,11 +598,11 @@ fn own_lines(text: &str, range: std::ops::Range<usize>) -> bool {
 }
 
 impl Settings {
-    pub const DEFAULT_PAGE_SIZE: u32 = 300;
+    pub const DEFAULT_PAGE_SIZE: u32 = 1_000;
     pub const MIN_PAGE_SIZE: u32 = 10;
     pub const MAX_PAGE_SIZE: u32 = 10_000;
     /// The page sizes the Settings window offers.
-    pub const PAGE_SIZES: [u32; 5] = [100, 300, 500, 1_000, 5_000];
+    pub const PAGE_SIZES: [u32; 5] = [100, 500, 1_000, 5_000, 10_000];
     /// The most rows a SQL editor statement keeps.
     pub const MAX_SQL_LIMIT: u32 = 10_000;
     /// The row limits the SQL editor's Limit menu offers.
````

- [ ] **Step 4: Check**

Run: `cargo fmt --all && cargo clippy --locked --workspace --all-targets -- -D warnings && cargo test --locked --workspace --all-targets`
Expected: no warning, every test passes.

- [ ] **Step 5: Commit**

```bash
git add docs/_guide/browsing-data.md docs/_reference/settings-and-files.md src/app.rs src/model.rs src/settings.rs src/ui/mod.rs src/ui/settings/sheet.rs
git commit -m "Take a new page size at a table's next load, and offer the artboard's sizes"
```

### Task 3: Numbers grouped as the look has them

`group_digits` not set is the look's default, and a value equal to it is not written.

**Files:** Modify `docs/_reference/settings-and-files.md`; Modify `src/app.rs`; Modify `src/settings.rs`; Modify `src/ui/data_view.rs`; Modify `src/ui/insert_audit_tests.rs`; Modify `src/ui/mod.rs`; Modify `src/ui/settings/mod.rs`; Modify `src/ui/settings/sheet.rs`; Modify `src/ui/settings/terminal.rs`; Modify `src/ui/sql_results.rs`; Modify `src/ui/workspace.rs`.

- [ ] **Step 1: Write the failing tests**

````diff
diff --git a/src/app.rs b/src/app.rs
index 8d051b9..45fb2d1 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -7276,6 +7276,40 @@ mod tests {
         );
     }
 
+    #[test]
+    fn grouping_numbers_as_the_look_does_anyway_writes_nothing() {
+        let (mut app, _dir) = app();
+        let layout = app.layout();
+        let default = Settings::grouped_by_default(layout);
+        app.change_settings(|settings| settings.group_digits = Some(default));
+        assert!(settings_saves(&app).is_empty());
+        app.change_settings(|settings| settings.group_digits = Some(!default));
+        assert_eq!(app.settings.group_digits, Some(!default));
+        assert!(
+            app.settings_file
+                .text
+                .contains(&format!("group_digits = {}", !default))
+        );
+        // Back to the look's: the key goes.
+        app.change_settings(|settings| settings.group_digits = Some(default));
+        assert_eq!(app.settings.group_digits, None);
+        assert!(!app.settings_file.text.contains("group_digits"));
+    }
+
+    #[test]
+    fn grouping_the_file_sets_as_the_look_does_stays_through_another_change() {
+        let (mut app, _dir) = app();
+        let default = Settings::grouped_by_default(app.layout());
+        let text = format!("[data]\ngroup_digits = {default} # mine\npage_size = 500\n");
+        app.apply(from_disk(&text, false));
+        assert_eq!(app.settings.group_digits, Some(default));
+        app.change_settings(|settings| settings.page_size = 100);
+        assert_eq!(
+            app.settings_file.text,
+            format!("[data]\ngroup_digits = {default} # mine\npage_size = 100\n")
+        );
+    }
+
     #[test]
     fn a_change_that_changes_nothing_is_not_written() {
         let (mut app, _dir) = app();
@@ -7331,7 +7365,7 @@ mod tests {
         }));
         assert_eq!(app.settings.page_size, 500);
         assert_eq!(app.settings.timestamps, crate::settings::Timestamps::Full);
-        assert!(!app.settings.group_digits);
+        assert_eq!(app.settings.group_digits, None);
         // The file as the user wrote it, with the line that was ignored.
         assert_eq!(app.settings_file.text, text);
         assert_eq!(app.settings_file.invalid, vec![3]);
@@ -7620,7 +7654,7 @@ mod tests {
         // An option of the window, and a key it does not show.
         app.change_settings(|settings| {
             settings.page_size = 500;
-            settings.group_digits = true;
+            settings.group_digits = Some(false);
             settings.show_system_schemas = true;
             settings.sql_limit = 50;
         });
diff --git a/src/settings.rs b/src/settings.rs
index cce057a..320958f 100644
--- a/src/settings.rs
+++ b/src/settings.rs
@@ -1120,7 +1120,7 @@ mod tests {
         let settings = Settings::default();
         assert_eq!(settings.page_size, 1_000);
         assert_eq!(settings.timestamps, Timestamps::Second);
-        assert!(!settings.group_digits);
+        assert_eq!(settings.group_digits, None);
         assert!(settings.value_tags);
         assert!(!settings.show_system_schemas);
         assert_eq!(settings.custom_theme, None);
@@ -1152,7 +1152,7 @@ mod tests {
         Settings {
             page_size: 500,
             timestamps: Timestamps::Full,
-            group_digits: true,
+            group_digits: Some(true),
             value_tags: false,
             show_system_schemas: true,
             custom_theme: Some("My \"Nord\".json".into()),
@@ -1371,7 +1371,7 @@ theme = \"My \\\"Nord\\\".json\"
         assert_eq!(Settings::from_toml(&out).invalid, [2, 3]);
         // A key whose value was refused gets the new one in its place.
         assert_eq!(
-            written(text, |settings| settings.group_digits = true),
+            written(text, |settings| settings.group_digits = Some(true)),
             "[data]\ntimestamps = full\ngroup_digits = true\npage_size = 500\n"
         );
     }
@@ -1585,6 +1585,31 @@ theme = \"My \\\"Nord\\\".json\"
         }
     }
 
+    #[test]
+    fn numbers_not_set_are_grouped_as_the_look_has_them() {
+        let settings = Settings::default();
+        assert!(settings.grouped(Layout::Mac));
+        assert!(settings.grouped(Layout::Windows));
+        assert!(!settings.grouped(Layout::Omarchy));
+        let set = Settings {
+            group_digits: Some(false),
+            ..Settings::default()
+        };
+        assert!(!set.grouped(Layout::Mac));
+    }
+
+    #[test]
+    fn what_the_look_has_anyway_is_not_set() {
+        let grouped = Settings {
+            group_digits: Some(true),
+            ..Settings::default()
+        };
+        assert_eq!(grouped.clone().for_layout(Layout::Mac).group_digits, None);
+        assert_eq!(grouped.for_layout(Layout::Omarchy).group_digits, Some(true));
+        // Not set, nothing is written.
+        assert_eq!(Settings::default().written(Key::GroupDigits), None);
+    }
+
     #[test]
     fn an_empty_file_starts_with_the_header() {
         assert_eq!(
@@ -1633,7 +1658,7 @@ theme = \"My \\\"Nord\\\".json\"
         assert_eq!(loaded.invalid, vec![3]);
         assert_eq!(loaded.settings.page_size, 100);
         // The lines and the tables under the bad one are still read.
-        assert!(loaded.settings.group_digits);
+        assert_eq!(loaded.settings.group_digits, Some(true));
         assert_eq!(loaded.settings.sql_limit, 100);
         assert_eq!(
             loaded.lines,
@@ -1838,7 +1863,7 @@ theme = \"My \\\"Nord\\\".json\"
         );
         assert_eq!(loaded.invalid, vec![3]);
         assert_eq!(loaded.settings.page_size, 100);
-        assert!(loaded.settings.group_digits);
+        assert_eq!(loaded.settings.group_digits, Some(true));
         assert_eq!(
             loaded.lines,
             vec![(Key::PageSize, 2), (Key::GroupDigits, 4)]
@@ -1900,7 +1925,7 @@ theme = \"My \\\"Nord\\\".json\"
         let loaded = Settings::from_toml("[data]\npage_size = 100\ngroup_digits = true\n\r");
         assert_eq!(loaded.invalid, vec![4]);
         assert_eq!(loaded.settings.page_size, 100);
-        assert!(loaded.settings.group_digits);
+        assert_eq!(loaded.settings.group_digits, Some(true));
         let loaded = Settings::from_toml("\r");
         assert_eq!(loaded.invalid, vec![1]);
         assert_eq!(loaded.settings, Settings::default());
@@ -1912,7 +1937,7 @@ theme = \"My \\\"Nord\\\".json\"
         let loaded = Settings::from_toml("[data]\npage_size = 100\ngroup_digits = true\n\u{a0}");
         assert_eq!(loaded.invalid, vec![4]);
         assert_eq!(loaded.settings.page_size, 100);
-        assert!(loaded.settings.group_digits);
+        assert_eq!(loaded.settings.group_digits, Some(true));
     }
 
     fn dirs() -> (crate::paths::AppDirs, tempfile::TempDir) {
@@ -1937,7 +1962,7 @@ theme = \"My \\\"Nord\\\".json\"
         let (dirs, _root) = dirs();
         let settings = Settings {
             page_size: 500,
-            group_digits: true,
+            group_digits: Some(true),
             sql_timeout_secs: None,
             ..Settings::default()
         };
@@ -2070,7 +2095,7 @@ theme = \"My \\\"Nord\\\".json\"
                 OptionId::Timestamps,
                 OptionValue::Timestamps(Timestamps::Full),
             ),
-            (OptionId::GroupDigits, OptionValue::GroupDigits(true)),
+            (OptionId::GroupDigits, OptionValue::GroupDigits(Some(true))),
             (OptionId::ValueTags, OptionValue::ValueTags(false)),
         ] {
             assert_eq!(value.option(), option);
@@ -2080,7 +2105,7 @@ theme = \"My \\\"Nord\\\".json\"
         }
         assert_eq!(settings.page_size, 500);
         assert_eq!(settings.timestamps, Timestamps::Full);
-        assert!(settings.group_digits);
+        assert_eq!(settings.group_digits, Some(true));
         assert!(!settings.value_tags);
     }
 
@@ -2163,11 +2188,11 @@ theme = \"My \\\"Nord\\\".json\"
         // Numbers: grouped on the left, plain on the right.
         assert_eq!(
             settings.stepped(OptionId::GroupDigits, false),
-            OptionValue::GroupDigits(true)
+            OptionValue::GroupDigits(Some(true))
         );
         assert_eq!(
             settings.stepped(OptionId::GroupDigits, true),
-            OptionValue::GroupDigits(false)
+            OptionValue::GroupDigits(Some(false))
         );
         // A check: off to the left, on to the right.
         assert_eq!(
@@ -2189,7 +2214,7 @@ theme = \"My \\\"Nord\\\".json\"
         );
         assert_eq!(
             settings.flipped(OptionId::GroupDigits),
-            Some(OptionValue::GroupDigits(true))
+            Some(OptionValue::GroupDigits(Some(true)))
         );
         assert_eq!(
             settings.flipped(OptionId::Timestamps),
@@ -2199,7 +2224,7 @@ theme = \"My \\\"Nord\\\".json\"
         // And back, from the other value of each.
         let settings = Settings {
             value_tags: false,
-            group_digits: true,
+            group_digits: Some(true),
             timestamps: Timestamps::Full,
             ..Default::default()
         };
@@ -2209,7 +2234,7 @@ theme = \"My \\\"Nord\\\".json\"
         );
         assert_eq!(
             settings.flipped(OptionId::GroupDigits),
-            Some(OptionValue::GroupDigits(false))
+            Some(OptionValue::GroupDigits(Some(false)))
         );
         assert_eq!(
             settings.flipped(OptionId::Timestamps),
diff --git a/src/ui/insert_audit_tests.rs b/src/ui/insert_audit_tests.rs
index a63ee37..aa54fdb 100644
--- a/src/ui/insert_audit_tests.rs
+++ b/src/ui/insert_audit_tests.rs
@@ -29,6 +29,8 @@ const HARBOR_PRESS: &str = "9100000000000000004";
 pub(super) fn covers_in(look: Look) -> (Harness, ConnTabId, TabId) {
     let mut harness = Harness::new();
     harness.set_look(look);
+    // The values are read as they were typed: no look groups them here.
+    harness.app.settings.group_digits = Some(false);
     let (tab, id) = harness.book_covers();
     harness.app.workspace_mut(tab).unwrap().pane = Pane::Grid;
     harness.settle();
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
index ae4ecab..62d25df 100644
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -817,7 +817,7 @@ mod tests {
         harness.answer_rows(crate::testing::page(3, true));
         let before = fetches(&harness);
         let settings = crate::settings::Settings {
-            group_digits: true,
+            group_digits: Some(true),
             ..harness.app.settings.clone()
         };
         harness.app.apply_settings(settings);
@@ -840,7 +840,7 @@ mod tests {
             .app
             .apply(crate::model::Action::ToggleFullPrecision(tab));
         let settings = crate::settings::Settings {
-            group_digits: true,
+            group_digits: Some(true),
             ..harness.app.settings.clone()
         };
         harness.app.apply_settings(settings);
@@ -850,6 +850,7 @@ mod tests {
     #[test]
     fn a_grid_is_fitted_again_when_an_option_widens_its_cells() {
         let mut harness = Harness::new();
+        harness.app.settings.group_digits = Some(false);
         harness.connect_fake();
         harness.click("users");
         let mut page = crate::testing::page(1, false);
@@ -858,7 +859,7 @@ mod tests {
         harness.settle();
         assert!(harness.painted_color("9223372036854775807").is_some());
         let settings = crate::settings::Settings {
-            group_digits: true,
+            group_digits: Some(true),
             ..harness.app.settings.clone()
         };
         harness.app.apply_settings(settings);
@@ -875,7 +876,7 @@ mod tests {
         let tab = harness.connect_fake();
         let group = |harness: &mut Harness, group_digits| {
             let settings = crate::settings::Settings {
-                group_digits,
+                group_digits: Some(group_digits),
                 ..harness.app.settings.clone()
             };
             harness.app.apply_settings(settings);
@@ -905,6 +906,7 @@ mod tests {
     #[test]
     fn an_edit_of_the_file_changes_what_an_open_grid_shows() {
         let mut harness = Harness::new();
+        harness.app.settings.group_digits = Some(false);
         harness.connect_fake();
         harness.click("users");
         let mut page = crate::testing::page(1, false);
@@ -925,7 +927,7 @@ mod tests {
     #[test]
     fn numbers_are_grouped_when_the_settings_say_so_but_keys_never_are() {
         let mut harness = Harness::new();
-        harness.app.settings.group_digits = true;
+        harness.app.settings.group_digits = Some(true);
         let tab = harness.connect_fake();
         harness.click("users");
         let mut page = crate::testing::page(1, false);
@@ -967,6 +969,7 @@ mod tests {
     #[test]
     fn numbers_are_plain_when_the_settings_do_not_group() {
         let mut harness = Harness::new();
+        harness.app.settings.group_digits = Some(false);
         harness.connect_fake();
         harness.click("users");
         let mut page = crate::testing::page(1, false);
@@ -976,6 +979,26 @@ mod tests {
         assert!(harness.painted_color("1234567").is_some());
     }
 
+    #[test]
+    fn numbers_not_set_are_grouped_as_the_look_has_them() {
+        for (look, grouped) in [
+            (crate::theme::Look::macos(), "1,234,567"),
+            (crate::theme::Look::standard(), "1,234,567"),
+            (crate::theme::Look::omarchy(), "1234567"),
+        ] {
+            let mut harness = Harness::new();
+            harness.set_look(look);
+            assert_eq!(harness.app.settings.group_digits, None);
+            harness.connect_fake();
+            harness.click("users");
+            let mut page = crate::testing::page(1, false);
+            page.rows[0][0] = tabletist_db::Value::Int(1_234_567);
+            harness.answer_rows(page);
+            harness.settle();
+            assert!(harness.painted_color(grouped).is_some(), "{grouped}");
+        }
+    }
+
     /// The colours a table's grid paints `true` and a plain text cell in,
     /// with the row selected so the row panel shows the value too: every
     /// `true` the frame painted, then the plain cell.
@@ -1290,10 +1313,13 @@ value_tags   = true
             );
             // It is written as before: the word alone.
             assert!(harness.painted_color("Full precision").is_some());
-            harness.click("Numbers: 1,240.50");
-            assert!(harness.app.settings.group_digits);
+            // Grouped is the look's default: set, it is not set at all,
+            // and nothing is written.
             harness.click("Numbers: 1240.50");
-            assert!(!harness.app.settings.group_digits);
+            assert_eq!(harness.app.settings.group_digits, Some(false));
+            harness.click("Numbers: 1,240.50");
+            assert_eq!(harness.app.settings.group_digits, None);
+            assert!(harness.app.settings.grouped(harness.app.layout()));
             let tags = harness.app.settings.value_tags;
             harness.click("Value tags");
             assert_eq!(harness.app.settings.value_tags, !tags);
@@ -1701,7 +1727,7 @@ value_tags   = true
         // Numbers: grouped is the left of the two.
         harness.press(egui::Key::J, egui::Modifiers::NONE);
         harness.press(egui::Key::H, egui::Modifiers::NONE);
-        assert!(harness.app.settings.group_digits);
+        assert!(harness.app.settings.grouped(harness.app.layout()));
         assert!(harness.painted_color("grouping on").is_some());
     }
 
@@ -2254,7 +2280,7 @@ value_tags   = true
         // Where it takes the keyboard is egui's to say: each arrow starts
         // from a button again.
         harness.press(egui::Key::ArrowLeft, egui::Modifiers::NONE);
-        assert!(!harness.app.settings.group_digits);
+        assert!(!harness.app.settings.grouped(harness.app.layout()));
         assert_eq!(settings_saves(&harness), 0);
         tab_to_settings(&mut harness, "Numbers");
         harness.press(egui::Key::ArrowUp, egui::Modifiers::NONE);
@@ -5910,7 +5936,7 @@ value_tags   = true
     #[test]
     fn a_result_groups_every_number_when_the_settings_say_so() {
         let mut harness = Harness::new();
-        harness.app.settings.group_digits = true;
+        harness.app.settings.group_digits = Some(true);
         let tab = harness.connect_fake();
         let mut page = crate::testing::page(1, false);
         page.rows[0][0] = tabletist_db::Value::Int(1_234_567);
@@ -12219,6 +12245,8 @@ value_tags   = true
     fn covers_in(look: Look) -> (Harness, ConnTabId, TabId) {
         let mut harness = Harness::new();
         harness.set_look(look);
+        // The values are read as they were typed: no look groups them here.
+        harness.app.settings.group_digits = Some(false);
         let (tab, id) = harness.book_covers();
         focus_grid(&mut harness, tab);
         harness.settle();
diff --git a/src/ui/sql_results.rs b/src/ui/sql_results.rs
index 03a00b1..eed7505 100644
--- a/src/ui/sql_results.rs
+++ b/src/ui/sql_results.rs
@@ -3977,7 +3977,7 @@ mod tests {
         run(&mut harness);
         harness.answer_sql(Ok(script_outcome(vec![result_with("email")])), None);
         let narrow = width(&mut harness, "email");
-        let fit = data_view::Fit::of(harness.app.workspace(tab).unwrap(), &harness.app.settings);
+        let fit = data_view::Fit::of(harness.app.workspace(tab).unwrap(), &harness.app);
         let first = sql(&harness, tab).run.loaded;
         let first = grid_id(tab, sql(&harness, tab).id, first, fit);
         assert!(crate::ui::grid::remembered(&harness.ctx, first));
````

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --locked -p tabletist --lib`
Expected: FAIL to compile: `` cannot find type `Layout` in this scope ``; `` no associated function or constant named `grouped_by_default` found for struct `settings::Settings` in the current scope ``; `` can't compare `bool` with `std::option::Option<_>` ``; `` mismatched types ``, and 3 more.

- [ ] **Step 3: Write the code**

````diff
diff --git a/docs/_reference/settings-and-files.md b/docs/_reference/settings-and-files.md
index 9f6af8b..30389d7 100644
--- a/docs/_reference/settings-and-files.md
+++ b/docs/_reference/settings-and-files.md
@@ -14,7 +14,7 @@ Cmd/Ctrl+, opens Settings. Changes apply right away.
 | --- | --- | --- |
 | **Rows per page** | 100, 500, 1,000, 5,000 or 10,000. A table takes a new size when it next loads, not while you page through it | 1,000 |
 | **Timestamps** | To the second, or full precision | To the second |
-| **Numbers** | `1240.50` or `1,240.50`. Grouping is for display only: a copy gives the raw value | `1240.50` |
+| **Numbers** | `1,240.50` or `1240.50`. Grouping is for display only: a copy gives the raw value | `1,240.50` on macOS and Windows, `1240.50` in the Omarchy look |
 | **Value tags** | Colors for enum, CHECK and boolean columns | On |
 
 A second pane, **Agents**, sets what the `[mcp]` keys
@@ -40,13 +40,13 @@ Settings are kept in `settings.toml`, which is safe to edit by hand.
 Tabletist watches the file and applies your edits while it runs. The file
 holds only what differs from the defaults. Until you change an option
 there is no file, and one the app writes starts with a header line. Here
-is every key at its default:
+is every key at its default (`group_digits` has none of its own: it
+follows the look):
 
 ```toml
 [data]
 page_size = 1000
 timestamps = "second"
-group_digits = false
 value_tags = true
 
 [sidebar]
@@ -66,7 +66,7 @@ notify = true
 | --- | --- | --- |
 | `data.page_size` | 10 to 10000. A table takes a new size when it next loads (opened, refreshed, sorted, filtered or reconnected), not while you page through it | `1000` |
 | `data.timestamps` | `"second"` or `"full"` | `"second"` |
-| `data.group_digits` | `true` or `false` | `false` |
+| `data.group_digits` | `true` or `false` | Not set: grouped on macOS and Windows, not in the Omarchy look |
 | `data.value_tags` | `true` or `false` | `true` |
 | `sidebar.show_system_schemas` | `true` shows schemas such as `pg_catalog` and `information_schema` | `false` |
 | `editor.sql_limit` | 1 to 10000 rows for each statement | `1000` |
diff --git a/src/app.rs b/src/app.rs
index 45fb2d1..db2ba69 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -3594,7 +3594,13 @@ impl App {
     pub fn change_settings(&mut self, change: impl FnOnce(&mut Settings)) {
         let mut changed = self.settings.clone();
         change(&mut changed);
-        let changed = changed.validated();
+        // What the look has anyway, set here, is not set: it is not
+        // written, and follows the look. One the file has is the person's,
+        // and stays as they wrote it.
+        let mut changed = changed.validated();
+        if changed.group_digits != self.settings.group_digits {
+            changed = changed.for_layout(self.layout());
+        }
         let keys = self.settings.changed_keys(&changed);
         if keys.is_empty() {
             return;
diff --git a/src/settings.rs b/src/settings.rs
index 320958f..8fd399d 100644
--- a/src/settings.rs
+++ b/src/settings.rs
@@ -4,6 +4,7 @@ use std::path::Path;
 
 use serde::Deserialize;
 
+use crate::keymap::Layout;
 use crate::paths::AppDirs;
 use crate::util::toml_string;
 
@@ -46,8 +47,10 @@ pub struct Settings {
     /// Timestamps in grid cells: to the second, or as the server sent them.
     pub timestamps: Timestamps,
     /// Numbers in grid cells with their integer digits in threes. Display
-    /// only: a copy gives the value as it is.
-    pub group_digits: bool,
+    /// only: a copy gives the value as it is. `None` is the look's default
+    /// ([`Settings::grouped`]), which is not written: grouped on macOS and
+    /// Windows, as their artboards draw it, and not on Omarchy.
+    pub group_digits: Option<bool>,
     /// Enum, CHECK and boolean values drawn as coloured tags.
     pub value_tags: bool,
     /// Show `pg_catalog`, `information_schema`, `mysql`, `sys` and friends.
@@ -190,7 +193,8 @@ impl OptionId {
 pub enum OptionValue {
     PageSize(u32),
     Timestamps(Timestamps),
-    GroupDigits(bool),
+    /// `None` is the look's default.
+    GroupDigits(Option<bool>),
     ValueTags(bool),
     AgentsOn(bool),
     LogDays(u32),
@@ -617,6 +621,31 @@ impl Settings {
     /// The days the Settings window offers for the log.
     pub const LOG_DAYS: [u32; 4] = [7, 30, 90, 365];
 
+    /// Whether numbers are grouped in `layout`: as set, or the look's
+    /// default.
+    pub fn grouped(&self, layout: Layout) -> bool {
+        self.group_digits
+            .unwrap_or_else(|| Self::grouped_by_default(layout))
+    }
+
+    /// Whether numbers are grouped in `layout` before anyone sets it: as
+    /// each look's artboard draws them.
+    pub fn grouped_by_default(layout: Layout) -> bool {
+        match layout {
+            Layout::Mac | Layout::Windows => true,
+            Layout::Omarchy => false,
+        }
+    }
+
+    /// The settings with what is set to the look's default taken for not
+    /// set, so that it is not written and follows the look.
+    pub fn for_layout(mut self, layout: Layout) -> Self {
+        if self.group_digits == Some(Self::grouped_by_default(layout)) {
+            self.group_digits = None;
+        }
+        self
+    }
+
     /// The SQL editor's timeout as a duration; `None` waits forever.
     pub fn sql_timeout(&self) -> Option<std::time::Duration> {
         Self::timeout_of(self.sql_timeout_secs)
@@ -675,7 +704,7 @@ impl Settings {
                 Timestamps::Second
             }),
             // Grouped is the left of the two.
-            OptionId::GroupDigits => OptionValue::GroupDigits(!forward),
+            OptionId::GroupDigits => OptionValue::GroupDigits(Some(!forward)),
             OptionId::ValueTags => OptionValue::ValueTags(forward),
             OptionId::AgentsOn => OptionValue::AgentsOn(forward),
             OptionId::LogDays => {
@@ -702,7 +731,9 @@ impl Settings {
                 Timestamps::Second => Timestamps::Full,
                 Timestamps::Full => Timestamps::Second,
             })),
-            OptionId::GroupDigits => Some(OptionValue::GroupDigits(!self.group_digits)),
+            OptionId::GroupDigits => Some(OptionValue::GroupDigits(Some(
+                !self.group_digits.unwrap_or_default(),
+            ))),
             OptionId::ValueTags => Some(OptionValue::ValueTags(!self.value_tags)),
             OptionId::AgentsOn => Some(OptionValue::AgentsOn(!self.mcp_enabled)),
             OptionId::LogDays => None,
@@ -716,7 +747,7 @@ impl Settings {
         Some(match key {
             Key::PageSize => self.page_size.to_string(),
             Key::Timestamps => toml_string(self.timestamps.name()),
-            Key::GroupDigits => self.group_digits.to_string(),
+            Key::GroupDigits => self.group_digits?.to_string(),
             Key::ValueTags => self.value_tags.to_string(),
             Key::ShowSystemSchemas => self.show_system_schemas.to_string(),
             Key::SqlLimit => self.sql_limit.to_string(),
@@ -935,7 +966,7 @@ impl Settings {
             Key::Timestamps => text
                 .and_then(Timestamps::from_name)
                 .map(|choice| self.timestamps = choice),
-            Key::GroupDigits => flag.map(|flag| self.group_digits = flag),
+            Key::GroupDigits => flag.map(|flag| self.group_digits = Some(flag)),
             Key::ValueTags => flag.map(|flag| self.value_tags = flag),
             Key::ShowSystemSchemas => flag.map(|flag| self.show_system_schemas = flag),
             Key::SqlLimit => number.map(|number| self.sql_limit = number),
@@ -1098,7 +1129,7 @@ impl Default for Settings {
         Self {
             page_size: Self::DEFAULT_PAGE_SIZE,
             timestamps: Timestamps::Second,
-            group_digits: false,
+            group_digits: None,
             value_tags: true,
             show_system_schemas: false,
             custom_theme: None,
diff --git a/src/ui/data_view.rs b/src/ui/data_view.rs
index 18b5f65..5066094 100644
--- a/src/ui/data_view.rs
+++ b/src/ui/data_view.rs
@@ -873,11 +873,11 @@ pub struct Fit {
 }
 
 impl Fit {
-    pub fn of(workspace: &crate::model::Workspace, settings: &crate::settings::Settings) -> Self {
+    pub fn of(workspace: &crate::model::Workspace, app: &crate::app::App) -> Self {
         Self {
             full_precision: workspace.full_precision,
-            grouped: settings.group_digits,
-            value_tags: settings.value_tags,
+            grouped: app.settings.grouped(app.layout()),
+            value_tags: app.settings.value_tags,
         }
     }
 }
@@ -992,7 +992,7 @@ pub fn footer(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabI
         .workspace(tab)
         .filter(|_| view == ObjectView::Data)
         .and_then(|workspace| {
-            let fit = Fit::of(workspace, &app.settings);
+            let fit = Fit::of(workspace, app);
             columns_note(ui.ctx(), tab, object, fit, &look, locale)
         });
     let mut actions = Vec::new();
@@ -1352,7 +1352,7 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabId)
     let Some(workspace) = app.workspace(tab) else {
         return;
     };
-    let fit = Fit::of(workspace, &app.settings);
+    let fit = Fit::of(workspace, app);
     // The arrows are the grid's once the user worked in it.
     let keys = workspace.pane == crate::model::Pane::Grid;
     let Some(object) = workspace.object_tab(object_tab) else {
diff --git a/src/ui/settings/mod.rs b/src/ui/settings/mod.rs
index 5cc6fb7..973c470 100644
--- a/src/ui/settings/mod.rs
+++ b/src/ui/settings/mod.rs
@@ -62,10 +62,12 @@ pub(super) fn is_on(value: OptionValue) -> bool {
 
 /// The settings as the window shows them: the server's switch as it is in
 /// force, off where a change made outside the app that turned it on is
-/// held.
+/// held, and the grouping of numbers as the look has it where it is not
+/// set.
 pub(super) fn shown(app: &App) -> Settings {
     let mut settings = app.settings.clone();
     settings.mcp_enabled = app.agents_on();
+    settings.group_digits = Some(app.settings.grouped(app.layout()));
     settings
 }
 
@@ -121,8 +123,8 @@ pub(super) fn choices(option: OptionId, settings: &Settings) -> Vec<(Said, Optio
         ],
         // A number as each value writes it.
         OptionId::GroupDigits => vec![
-            (Sample("1,240.50"), OptionValue::GroupDigits(true)),
-            (Sample("1240.50"), OptionValue::GroupDigits(false)),
+            (Sample("1,240.50"), OptionValue::GroupDigits(Some(true))),
+            (Sample("1240.50"), OptionValue::GroupDigits(Some(false))),
         ],
         OptionId::ValueTags => vec![
             (Word("off"), OptionValue::ValueTags(false)),
diff --git a/src/ui/settings/sheet.rs b/src/ui/settings/sheet.rs
index 7c33ed2..24fa97c 100644
--- a/src/ui/settings/sheet.rs
+++ b/src/ui/settings/sheet.rs
@@ -477,9 +477,11 @@ fn control(
         OptionId::GroupDigits => {
             // A number as each value writes it: no language has others.
             let words = ["1,240.50".to_owned(), "1240.50".to_owned()];
-            let selected = usize::from(!settings.group_digits);
+            let selected = usize::from(!settings.group_digits.unwrap_or_default());
             if let Some(picked) = segments(ui, &name, &words, selected, skin) {
-                actions.push(Action::SetOption(OptionValue::GroupDigits(picked == 0)));
+                actions.push(Action::SetOption(OptionValue::GroupDigits(Some(
+                    picked == 0,
+                ))));
             }
         }
         OptionId::ValueTags | OptionId::AgentsOn | OptionId::AgentsNotify => {
diff --git a/src/ui/settings/terminal.rs b/src/ui/settings/terminal.rs
index 1ef6eb7..ff34e64 100644
--- a/src/ui/settings/terminal.rs
+++ b/src/ui/settings/terminal.rs
@@ -936,7 +936,7 @@ fn hint(ui: &Ui, option: OptionId, settings: &Settings, skin: &Skin) -> Text {
     match option {
         OptionId::PageSize => dim(&skin.say("table view")),
         OptionId::Timestamps => dim(sample_timestamp(settings.timestamps)),
-        OptionId::GroupDigits => dim(&skin.say(if settings.group_digits {
+        OptionId::GroupDigits => dim(&skin.say(if settings.group_digits.unwrap_or_default() {
             "grouping on"
         } else {
             "grouping off"
diff --git a/src/ui/sql_results.rs b/src/ui/sql_results.rs
index eed7505..9b7d014 100644
--- a/src/ui/sql_results.rs
+++ b/src/ui/sql_results.rs
@@ -212,7 +212,7 @@ fn draw(app: &App, ui: &mut Ui, tab: ConnTabId, id: TabId, actions: &mut Vec<Act
     let place = Place {
         tab,
         sql,
-        fit: data_view::Fit::of(workspace, &app.settings),
+        fit: data_view::Fit::of(workspace, app),
         driver: workspace.driver,
         blocked: blocked(workspace, sql),
         keys: workspace.pane == crate::model::Pane::Grid,
diff --git a/src/ui/workspace.rs b/src/ui/workspace.rs
index fcd825d..fb9d90b 100644
--- a/src/ui/workspace.rs
+++ b/src/ui/workspace.rs
@@ -2143,7 +2143,7 @@ fn status_line(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
             // The columns in view first, while some are out of it.
             let columns = (object.view == ObjectView::Data)
                 .then(|| {
-                    let fit = super::data_view::Fit::of(workspace, &app.settings);
+                    let fit = super::data_view::Fit::of(workspace, app);
                     super::data_view::columns_note(ui.ctx(), tab, object, fit, &look, locale)
                 })
                 .flatten();
````

- [ ] **Step 4: Check**

Run: `cargo fmt --all && cargo clippy --locked --workspace --all-targets -- -D warnings && cargo test --locked --workspace --all-targets`
Expected: no warning, every test passes.

- [ ] **Step 5: Commit**

```bash
git add docs/_reference/settings-and-files.md src/app.rs src/settings.rs src/ui/data_view.rs src/ui/insert_audit_tests.rs src/ui/mod.rs src/ui/settings/mod.rs src/ui/settings/sheet.rs src/ui/settings/terminal.rs src/ui/sql_results.rs src/ui/workspace.rs
git commit -m "Group numbers as the look draws them where the settings do not say"
```

### Task 4: The zone setting

`data.tz`: `system`, `utc` or a name jiff's database knows; anything else is an ignored line.

**Files:** Modify `Cargo.toml`; Modify `docs/_reference/settings-and-files.md`; Modify `src/settings.rs`.

- [ ] **Step 1: Write the failing tests**

````diff
diff --git a/src/settings.rs b/src/settings.rs
index 8fd399d..fb0c8ba 100644
--- a/src/settings.rs
+++ b/src/settings.rs
@@ -1183,6 +1183,7 @@ mod tests {
         Settings {
             page_size: 500,
             timestamps: Timestamps::Full,
+            tz: Zone::Named("America/New_York".into()),
             group_digits: Some(true),
             value_tags: false,
             show_system_schemas: true,
@@ -1204,6 +1205,7 @@ mod tests {
 [data]
 page_size = 500
 timestamps = \"full\"  # second | full
+tz = \"America/New_York\"  # system | utc | \"Area/City\"
 group_digits = true
 value_tags = false
 
@@ -1277,15 +1279,16 @@ theme = \"My \\\"Nord\\\".json\"
             vec![
                 (Key::PageSize, 3),
                 (Key::Timestamps, 4),
-                (Key::GroupDigits, 5),
-                (Key::ValueTags, 6),
-                (Key::ShowSystemSchemas, 9),
-                (Key::SqlLimit, 12),
-                (Key::SqlTimeoutSecs, 13),
-                (Key::McpEnabled, 16),
-                (Key::McpLogDays, 17),
-                (Key::McpNotify, 18),
-                (Key::Theme, 21),
+                (Key::Tz, 5),
+                (Key::GroupDigits, 6),
+                (Key::ValueTags, 7),
+                (Key::ShowSystemSchemas, 10),
+                (Key::SqlLimit, 13),
+                (Key::SqlTimeoutSecs, 14),
+                (Key::McpEnabled, 17),
+                (Key::McpLogDays, 18),
+                (Key::McpNotify, 19),
+                (Key::Theme, 22),
             ]
         );
     }
@@ -1641,6 +1644,59 @@ theme = \"My \\\"Nord\\\".json\"
         assert_eq!(Settings::default().written(Key::GroupDigits), None);
     }
 
+    #[test]
+    fn a_zone_is_system_utc_or_a_name_the_database_knows() {
+        assert_eq!(Zone::from_name("system"), Some(Zone::System));
+        assert_eq!(Zone::from_name("UTC"), Some(Zone::Utc));
+        assert_eq!(
+            Zone::from_name("Europe/Lisbon"),
+            Some(Zone::Named("Europe/Lisbon".into()))
+        );
+        assert_eq!(Zone::from_name("Mars/Olympus"), None);
+        assert_eq!(Zone::from_name(""), None);
+        for zone in [Zone::System, Zone::Utc, Zone::Named("Asia/Tokyo".into())] {
+            assert_eq!(Zone::from_name(zone.name()), Some(zone));
+        }
+        let system = jiff::tz::TimeZone::fixed(jiff::tz::offset(3));
+        assert_eq!(Zone::System.resolve(&system), system);
+        assert_eq!(Zone::Utc.resolve(&system), jiff::tz::TimeZone::UTC);
+        assert_eq!(
+            Zone::Named("Asia/Tokyo".into())
+                .resolve(&system)
+                .iana_name(),
+            Some("Asia/Tokyo")
+        );
+    }
+
+    #[test]
+    fn a_zone_the_database_does_not_know_is_ignored_and_keeps_the_last_good_one() {
+        let last = Settings {
+            tz: Zone::Utc,
+            ..Settings::default()
+        };
+        let text = "[data]\ntz = \"Mars/Olympus\"\npage_size = 500\n";
+        let loaded = Settings::from_toml_over(text, &last);
+        assert_eq!(loaded.invalid, [2]);
+        assert_eq!(loaded.settings.tz, Zone::Utc);
+        assert_eq!(loaded.settings.page_size, 500);
+    }
+
+    #[test]
+    fn a_zone_goes_in_the_data_table_with_its_note() {
+        assert_eq!(
+            written("[data]\npage_size = 500\n", |settings| {
+                settings.tz = Zone::Named("Europe/Lisbon".into());
+            }),
+            "[data]\npage_size = 500\ntz = \"Europe/Lisbon\"  # system | utc | \"Area/City\"\n"
+        );
+        // Back to the computer's own, which is not written.
+        assert_eq!(
+            written("[data]\ntz = \"utc\"\n", |settings| settings.tz =
+                Zone::System),
+            ""
+        );
+    }
+
     #[test]
     fn an_empty_file_starts_with_the_header() {
         assert_eq!(
````

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --locked -p tabletist --lib`
Expected: FAIL to compile: `` struct `settings::Settings` has no field named `tz` ``; `` cannot find type `Zone` in this scope ``; `` no variant, associated function, or constant named `Tz` found for enum `settings::Key` in the current scope ``; `` no field `tz` on type `settings::Settings` ``, and 1 more.

- [ ] **Step 3: Write the code**

````diff
diff --git a/Cargo.toml b/Cargo.toml
index 1a84f5a..c287dc8 100644
--- a/Cargo.toml
+++ b/Cargo.toml
@@ -112,9 +112,9 @@ toml = { version = "1.1", default-features = false, features = ["parse", "serde"
 # features fastframe-theme already brings in for following the desktop's
 # theme: nothing new is built.
 notify = "8.2"
-# The person's clock and time zone, for the times of agents' calls. The
-# version and the features env_logger already brings in: nothing new is
-# built.
+# The person's clock and time zone, for the times of agents' calls, and the
+# zone database a `timestamptz` value is shown in. The version and the
+# features env_logger already brings in: nothing new is built.
 jiff = "0.2"
 
 [target.'cfg(target_os = "linux")'.dependencies]
diff --git a/docs/_reference/settings-and-files.md b/docs/_reference/settings-and-files.md
index 30389d7..06b7a69 100644
--- a/docs/_reference/settings-and-files.md
+++ b/docs/_reference/settings-and-files.md
@@ -47,6 +47,7 @@ follows the look):
 [data]
 page_size = 1000
 timestamps = "second"
+tz = "system"
 value_tags = true
 
 [sidebar]
@@ -66,6 +67,7 @@ notify = true
 | --- | --- | --- |
 | `data.page_size` | 10 to 10000. A table takes a new size when it next loads (opened, refreshed, sorted, filtered or reconnected), not while you page through it | `1000` |
 | `data.timestamps` | `"second"` or `"full"` | `"second"` |
+| `data.tz` | `"system"`, `"utc"` or a zone such as `"America/New_York"`. A name the zone database does not know is a line that cannot be read | `"system"` |
 | `data.group_digits` | `true` or `false` | Not set: grouped on macOS and Windows, not in the Omarchy look |
 | `data.value_tags` | `true` or `false` | `true` |
 | `sidebar.show_system_schemas` | `true` shows schemas such as `pg_catalog` and `information_schema` | `false` |
diff --git a/src/settings.rs b/src/settings.rs
index fb0c8ba..93dd2fb 100644
--- a/src/settings.rs
+++ b/src/settings.rs
@@ -35,6 +35,62 @@ impl Timestamps {
     }
 }
 
+/// The zone a `timestamptz` value is shown in.
+#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
+#[serde(from = "String")]
+pub enum Zone {
+    /// The computer's own, as it was when the app started.
+    #[default]
+    System,
+    Utc,
+    /// A zone of the IANA database, by its name: `America/New_York`.
+    Named(String),
+}
+
+impl Zone {
+    /// As the file says it: `system`, `utc` or the zone's name.
+    pub fn name(&self) -> &str {
+        match self {
+            Self::System => "system",
+            Self::Utc => "utc",
+            Self::Named(name) => name,
+        }
+    }
+
+    /// The zone the file names, where it is one: `system`, `utc` (in any
+    /// case), or a name the zone database knows.
+    pub fn from_name(name: &str) -> Option<Self> {
+        if name.eq_ignore_ascii_case("system") {
+            Some(Self::System)
+        } else if name.eq_ignore_ascii_case("utc") {
+            Some(Self::Utc)
+        } else if !name.is_empty() && jiff::tz::db().get(name).is_ok() {
+            Some(Self::Named(name.to_owned()))
+        } else {
+            None
+        }
+    }
+
+    /// The zone itself. `system` is the zone the app found at its start.
+    pub fn resolve(&self, system: &jiff::tz::TimeZone) -> jiff::tz::TimeZone {
+        match self {
+            Self::System => system.clone(),
+            Self::Utc => jiff::tz::TimeZone::UTC,
+            // Known to the database when it was read: a name that has
+            // gone from it since shows as UTC.
+            Self::Named(name) => jiff::tz::db().get(name).unwrap_or(jiff::tz::TimeZone::UTC),
+        }
+    }
+}
+
+/// The JSON of versions before the TOML file has no zone: anything else
+/// it says is no zone either, and is the default.
+impl From<String> for Zone {
+    fn from(name: String) -> Self {
+        Self::from_name(&name).unwrap_or_default()
+    }
+}
+
 /// Everything the user can set. A key the file lacks takes its default and
 /// one this version does not know is ignored, so older and newer files both
 /// load. The serde derive reads the `settings.json` of versions before the
@@ -46,6 +102,9 @@ pub struct Settings {
     pub page_size: u32,
     /// Timestamps in grid cells: to the second, or as the server sent them.
     pub timestamps: Timestamps,
+    /// The zone a value with an offset (a `timestamptz`) is shown in, and
+    /// a value typed into such a column without one is sent from.
+    pub tz: Zone,
     /// Numbers in grid cells with their integer digits in threes. Display
     /// only: a copy gives the value as it is. `None` is the look's default
     /// ([`Settings::grouped`]), which is not written: grouped on macOS and
@@ -78,6 +137,7 @@ pub struct Settings {
 pub enum Key {
     PageSize,
     Timestamps,
+    Tz,
     GroupDigits,
     ValueTags,
     ShowSystemSchemas,
@@ -91,9 +151,10 @@ pub enum Key {
 
 impl Key {
     /// In the order the file has them: a table's keys together.
-    pub const ALL: [Key; 11] = [
+    pub const ALL: [Key; 12] = [
         Self::PageSize,
         Self::Timestamps,
+        Self::Tz,
         Self::GroupDigits,
         Self::ValueTags,
         Self::ShowSystemSchemas,
@@ -110,6 +171,7 @@ impl Key {
         match self {
             Self::PageSize => ("data", "page_size"),
             Self::Timestamps => ("data", "timestamps"),
+            Self::Tz => ("data", "tz"),
             Self::GroupDigits => ("data", "group_digits"),
             Self::ValueTags => ("data", "value_tags"),
             Self::ShowSystemSchemas => ("sidebar", "show_system_schemas"),
@@ -127,6 +189,7 @@ impl Key {
     fn note(self) -> Option<&'static str> {
         match self {
             Self::Timestamps => Some("second | full"),
+            Self::Tz => Some("system | utc | \"Area/City\""),
             Self::SqlTimeoutSecs => Some("0 waits forever"),
             _ => None,
         }
@@ -747,6 +810,7 @@ impl Settings {
         Some(match key {
             Key::PageSize => self.page_size.to_string(),
             Key::Timestamps => toml_string(self.timestamps.name()),
+            Key::Tz => toml_string(self.tz.name()),
             Key::GroupDigits => self.group_digits?.to_string(),
             Key::ValueTags => self.value_tags.to_string(),
             Key::ShowSystemSchemas => self.show_system_schemas.to_string(),
@@ -966,6 +1030,7 @@ impl Settings {
             Key::Timestamps => text
                 .and_then(Timestamps::from_name)
                 .map(|choice| self.timestamps = choice),
+            Key::Tz => text.and_then(Zone::from_name).map(|zone| self.tz = zone),
             Key::GroupDigits => flag.map(|flag| self.group_digits = Some(flag)),
             Key::ValueTags => flag.map(|flag| self.value_tags = flag),
             Key::ShowSystemSchemas => flag.map(|flag| self.show_system_schemas = flag),
@@ -1073,6 +1138,7 @@ impl Settings {
         match key {
             Key::PageSize => self.page_size = other.page_size,
             Key::Timestamps => self.timestamps = other.timestamps,
+            Key::Tz => self.tz = other.tz.clone(),
             Key::GroupDigits => self.group_digits = other.group_digits,
             Key::ValueTags => self.value_tags = other.value_tags,
             Key::ShowSystemSchemas => self.show_system_schemas = other.show_system_schemas,
@@ -1129,6 +1195,7 @@ impl Default for Settings {
         Self {
             page_size: Self::DEFAULT_PAGE_SIZE,
             timestamps: Timestamps::Second,
+            tz: Zone::System,
             group_digits: None,
             value_tags: true,
             show_system_schemas: false,
````

- [ ] **Step 4: Check**

Run: `cargo fmt --all && cargo clippy --locked --workspace --all-targets -- -D warnings && cargo test --locked --workspace --all-targets`
Expected: no warning, every test passes.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml docs/_reference/settings-and-files.md src/settings.rs
git commit -m "Keep the zone timestamptz values are shown in"
```

### Task 5: Show a moment with an offset in the zone

The grid, the SQL results and the row panel show a `timestamptz` in the zone; nothing is fetched again, and Copy gives the server's text.

**Files:** Modify `docs/_reference/settings-and-files.md`; Modify `src/app.rs`; Modify `src/model.rs`; Modify `src/testing.rs`; Modify `src/ui/data_view.rs`; Modify `src/ui/format.rs`; Modify `src/ui/mod.rs`; Modify `src/ui/sql_results.rs`; Create `src/ui/zone_tests.rs`.

- [ ] **Step 1: Write the failing tests**

````diff
diff --git a/src/testing.rs b/src/testing.rs
index 12d3d01..8c48b2e 100644
--- a/src/testing.rs
+++ b/src/testing.rs
@@ -803,6 +803,30 @@ pub fn book_covers_page(rows: usize) -> RowPage {
     }
 }
 
+/// When the Bookshop's covers were made, as PostgreSQL sends a
+/// `timestamptz` in a session whose zone is UTC.
+pub const COVERS_MADE: &str = "2026-01-12 09:14:03.482915+00";
+
+/// The structure of `book_covers` on PostgreSQL, where `created_at` is a
+/// `timestamptz`.
+pub fn stamped_covers_structure() -> tabletist_db::Structure {
+    let mut structure = book_covers_structure();
+    structure.columns[4].type_name = "timestamp with time zone".into();
+    structure
+}
+
+/// A page of `book_covers` with `created_at` as a `timestamptz`, made at
+/// [`COVERS_MADE`].
+pub fn stamped_covers_page(rows: usize) -> RowPage {
+    let mut page = book_covers_page(rows);
+    page.columns[4].type_name = "timestamptz".into();
+    page.columns[4].kind = ValueKind::Temporal;
+    for row in &mut page.rows {
+        row[4] = Value::Text(COVERS_MADE.into());
+    }
+    page
+}
+
 /// The Bookshop's publishers, as (id, name).
 pub const PUBLISHERS: [(i64, &str); 3] = [
     (9_100_000_000_000_000_001, "Northlight Books"),
@@ -1131,6 +1155,26 @@ impl Harness {
         (tab, id)
     }
 
+    /// `book_covers` with `created_at` as a `timestamptz`, open in a table
+    /// tab: [`Harness::book_covers`] over [`stamped_covers_page`].
+    pub fn stamped_covers(&mut self) -> (ConnTabId, TabId) {
+        let tab = self.connect_fake_as(false);
+        self.app.apply(Action::OpenObject {
+            tab,
+            object: tabletist_db::ObjectRef::new("main", "book_covers"),
+            kind: ObjectKind::Table,
+            pin: true,
+        });
+        self.answer_structure(stamped_covers_structure());
+        self.answer_rows(stamped_covers_page(3));
+        let id = self
+            .app
+            .workspace(tab)
+            .and_then(|workspace| workspace.active_tab)
+            .expect("the table's tab is open");
+        (tab, id)
+    }
+
     /// Answers the newest FetchRows command with `page`.
     pub fn answer_rows(&mut self, page: RowPage) {
         let (session, request) = self
diff --git a/src/ui/format.rs b/src/ui/format.rs
index 41ecad9..0453434 100644
--- a/src/ui/format.rs
+++ b/src/ui/format.rs
@@ -989,6 +989,64 @@ mod tests {
     }
 
     use super::*;
+
+    fn new_york() -> jiff::tz::TimeZone {
+        jiff::tz::db().get("America/New_York").unwrap()
+    }
+
+    #[test]
+    fn a_value_with_an_offset_is_shown_in_the_zone() {
+        let zone = new_york();
+        assert_eq!(
+            in_zone("2026-01-12 09:14:03.482915+00", &zone).as_deref(),
+            Some("2026-01-12 04:14:03.482915-05")
+        );
+        // Summer time, and a moment the zone's day is the day before.
+        assert_eq!(
+            in_zone("2026-07-01 02:00:00+02", &zone).as_deref(),
+            Some("2026-06-30 20:00:00-04")
+        );
+        // An offset in minutes, and one in seconds, read; the separator
+        // and the fraction's digits kept.
+        assert_eq!(
+            in_zone("2026-01-12T09:14:03.5+05:30", &jiff::tz::TimeZone::UTC).as_deref(),
+            Some("2026-01-12T03:44:03.5+00")
+        );
+        assert_eq!(
+            in_zone("1900-01-01 00:00:00-00:17:30", &jiff::tz::TimeZone::UTC).as_deref(),
+            Some("1900-01-01 00:17:30+00")
+        );
+        let kolkata = jiff::tz::db().get("Asia/Kolkata").unwrap();
+        assert_eq!(
+            in_zone("2026-01-12 09:14:03+00", &kolkata).as_deref(),
+            Some("2026-01-12 14:44:03+05:30")
+        );
+    }
+
+    #[test]
+    fn what_is_no_moment_with_an_offset_is_left_as_it_is() {
+        let zone = new_york();
+        for text in [
+            "2026-01-12 09:14:03",
+            "2026-01-12 09:14:03.482915",
+            "09:14:03+00",
+            "2026-01-12",
+            "infinity",
+            "-infinity",
+            "2026-01-12 09:14:03+00 BC",
+            "2026-02-30 09:14:03+00",
+            "2026-01-12 09:14:03+0",
+            "2026-01-12 09:14:03+00:00:00:00",
+            "2026-01-12 09:14:03.+00",
+            "",
+        ] {
+            assert_eq!(in_zone(text, &zone), None, "{text}");
+        }
+        // The first moment of the year 1 is the year before it in New
+        // York, which PostgreSQL writes with ` BC`.
+        assert_eq!(in_zone("0001-01-01 00:00:00+00", &zone), None);
+    }
+
     use std::time::Duration;
     use tabletist_db::Value;
 
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
index 62d25df..0ee7cf6 100644
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -44,6 +44,8 @@ pub mod keys;
 pub mod label_chooser;
 #[cfg(test)]
 mod label_tests;
+#[cfg(test)]
+mod zone_tests;
 pub mod menu;
 pub mod object_tabs;
 pub mod password_prompt;
diff --git a/src/ui/zone_tests.rs b/src/ui/zone_tests.rs
new file mode 100644
index 0000000..8ebeb9b
--- /dev/null
+++ b/src/ui/zone_tests.rs
@@ -0,0 +1,197 @@
+//! The zone a `timestamptz` value is shown in (Settings › General, Time
+//! zone for timestamptz), on the Bookshop's `book_covers.created_at`.
+//!
+//! Only what is shown and what is copied are checked. No colour, size or
+//! picture of the design is.
+
+use egui::{Key, Modifiers};
+
+use crate::backend::Command;
+use crate::model::{Action, CellPos, ConnTabId, Pane, TabId};
+use crate::settings::{Settings, Zone};
+use crate::testing::{COVERS_MADE, Harness, script_outcome, stamped_covers_page};
+
+/// `created_at`, by its place in the page.
+const MADE: usize = 4;
+
+fn zone(harness: &mut Harness, zone: Zone) {
+    let settings = Settings {
+        tz: zone,
+        ..harness.app.settings.clone()
+    };
+    harness.app.apply_settings(settings);
+    harness.settle();
+}
+
+fn new_york() -> Zone {
+    Zone::Named("America/New_York".into())
+}
+
+fn fetches(harness: &Harness) -> usize {
+    harness
+        .app
+        .backend
+        .sent
+        .iter()
+        .filter(|command| matches!(command, Command::FetchRows { .. }))
+        .count()
+}
+
+/// The covers' tab with the cursor on the first row's `created_at`.
+fn covers() -> (Harness, ConnTabId, TabId) {
+    let mut harness = Harness::new();
+    let (tab, id) = harness.stamped_covers();
+    harness.app.workspace_mut(tab).unwrap().pane = Pane::Grid;
+    harness.app.apply(Action::SelectCell {
+        tab,
+        id,
+        cell: CellPos { row: 0, col: MADE },
+    });
+    harness.settle();
+    (harness, tab, id)
+}
+
+#[test]
+fn a_moment_is_shown_in_the_zone_and_nothing_is_fetched_again() {
+    let (mut harness, _tab, _id) = covers();
+    // System is UTC in a test: the moment as the server sent it, to the
+    // second.
+    assert!(harness.painted_color("2026-01-12 09:14:03+00").is_some());
+    let before = fetches(&harness);
+    zone(&mut harness, new_york());
+    assert!(harness.painted_color("2026-01-12 04:14:03-05").is_some());
+    assert!(harness.painted_color("2026-01-12 09:14:03+00").is_none());
+    zone(&mut harness, Zone::Named("Asia/Kolkata".into()));
+    assert!(harness.painted_color("2026-01-12 14:44:03+05:30").is_some());
+    assert_eq!(fetches(&harness), before);
+}
+
+#[test]
+fn the_system_zone_is_the_one_the_app_found_at_its_start() {
+    let mut harness = Harness::new();
+    harness.app.system_zone = jiff::tz::db().get("Asia/Tokyo").unwrap();
+    harness.stamped_covers();
+    harness.settle();
+    assert_eq!(harness.app.settings.tz, Zone::System);
+    assert!(harness.painted_color("2026-01-12 18:14:03+09").is_some());
+    zone(&mut harness, Zone::Utc);
+    assert!(harness.painted_color("2026-01-12 09:14:03+00").is_some());
+}
+
+#[test]
+fn full_precision_keeps_the_fraction_the_server_sent() {
+    let (mut harness, _tab, _id) = covers();
+    let settings = Settings {
+        timestamps: crate::settings::Timestamps::Full,
+        tz: new_york(),
+        ..harness.app.settings.clone()
+    };
+    harness.app.apply_settings(settings);
+    harness.settle();
+    assert!(
+        harness
+            .painted_color("2026-01-12 04:14:03.482915-05")
+            .is_some()
+    );
+}
+
+#[test]
+fn copy_gives_the_value_the_server_sent() {
+    let (mut harness, _tab, _id) = covers();
+    zone(&mut harness, new_york());
+    harness.copy(false);
+    assert_eq!(harness.copied.as_deref(), Some(COVERS_MADE));
+}
+
+#[test]
+fn the_row_panel_shows_the_moment_in_the_zone_and_follows_a_change() {
+    let (mut harness, tab, _id) = covers();
+    harness.app.workspace_mut(tab).unwrap().row_panel = true;
+    harness.settle();
+    // The panel writes the whole value: its fraction too.
+    assert!(
+        harness
+            .painted_color("2026-01-12 09:14:03.482915+00")
+            .is_some()
+    );
+    zone(&mut harness, new_york());
+    assert!(
+        harness
+            .painted_color("2026-01-12 04:14:03.482915-05")
+            .is_some()
+    );
+}
+
+#[test]
+fn a_text_column_that_reads_as_a_moment_is_left_alone() {
+    let mut harness = Harness::new();
+    let tab = harness.connect_fake_as(false);
+    harness.app.apply(Action::OpenObject {
+        tab,
+        object: tabletist_db::ObjectRef::new("main", "book_covers"),
+        kind: tabletist_db::ObjectKind::Table,
+        pin: true,
+    });
+    harness.answer_structure(crate::testing::book_covers_structure());
+    let mut page = crate::testing::book_covers_page(1);
+    page.rows[0][MADE] = tabletist_db::Value::Text(COVERS_MADE.into());
+    harness.answer_rows(page);
+    zone(&mut harness, new_york());
+    assert!(harness.painted_color(COVERS_MADE).is_some());
+}
+
+#[test]
+fn a_sql_result_shows_the_moment_in_the_zone() {
+    let mut harness = Harness::new();
+    let _tab = harness.connect_fake();
+    harness.press(Key::T, Modifiers::COMMAND);
+    harness.frame(vec![egui::Event::Paste(
+        "SELECT created_at FROM book_covers".into(),
+    )]);
+    harness.press(Key::Enter, Modifiers::COMMAND);
+    let page = stamped_covers_page(1);
+    harness.answer_sql(
+        Ok(script_outcome(vec![tabletist_db::StatementOutcome::Rows {
+            columns: vec![page.columns[MADE].clone()],
+            rows: vec![vec![page.rows[0][MADE].clone()]],
+            truncated: false,
+        }])),
+        None,
+    );
+    zone(&mut harness, new_york());
+    assert!(harness.painted_color("2026-01-12 04:14:03-05").is_some());
+}
+
+#[test]
+fn a_zone_set_by_hand_in_the_file_shows_at_once() {
+    let (mut harness, _tab, _id) = covers();
+    harness
+        .app
+        .apply(Action::Backend(crate::backend::Event::SettingsFile {
+            text: "[data]\ntz = \"America/New_York\"\n".into(),
+            own: false,
+        }));
+    harness.settle();
+    assert_eq!(harness.app.settings.tz, new_york());
+    assert!(harness.painted_color("2026-01-12 04:14:03-05").is_some());
+}
+
+#[test]
+fn the_row_panel_shows_the_system_zone_too() {
+    let mut harness = Harness::new();
+    harness.app.system_zone = jiff::tz::db().get("Asia/Tokyo").unwrap();
+    let (tab, id) = harness.stamped_covers();
+    harness.app.workspace_mut(tab).unwrap().pane = Pane::Grid;
+    harness.app.workspace_mut(tab).unwrap().row_panel = true;
+    harness.app.apply(Action::SelectCell {
+        tab,
+        id,
+        cell: CellPos { row: 0, col: MADE },
+    });
+    harness.settle();
+    assert!(
+        harness
+            .painted_color("2026-01-12 18:14:03.482915+09")
+            .is_some()
+    );
+}
````

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --locked -p tabletist --lib`
Expected: FAIL to compile: `` cannot find function `in_zone` in this scope ``; `` no field `system_zone` on type `app::App` ``.

- [ ] **Step 3: Write the code**

````diff
diff --git a/docs/_reference/settings-and-files.md b/docs/_reference/settings-and-files.md
index 06b7a69..11787de 100644
--- a/docs/_reference/settings-and-files.md
+++ b/docs/_reference/settings-and-files.md
@@ -67,7 +67,7 @@ notify = true
 | --- | --- | --- |
 | `data.page_size` | 10 to 10000. A table takes a new size when it next loads (opened, refreshed, sorted, filtered or reconnected), not while you page through it | `1000` |
 | `data.timestamps` | `"second"` or `"full"` | `"second"` |
-| `data.tz` | `"system"`, `"utc"` or a zone such as `"America/New_York"`. A name the zone database does not know is a line that cannot be read | `"system"` |
+| `data.tz` | `"system"`, `"utc"` or a zone such as `"America/New_York"`. A name the zone database does not know is a line that cannot be read. A `timestamptz` value is shown in this zone, and a copy gives it as the server sent it | `"system"` |
 | `data.group_digits` | `true` or `false` | Not set: grouped on macOS and Windows, not in the Omarchy look |
 | `data.value_tags` | `true` or `false` | `true` |
 | `sidebar.show_system_schemas` | `true` shows schemas such as `pg_catalog` and `information_schema` | `false` |
diff --git a/src/app.rs b/src/app.rs
index db2ba69..0c1a8f8 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -79,6 +79,9 @@ pub struct App {
     /// The settings file as the app last read or wrote it or, before any
     /// file exists, as it would be written.
     pub settings_file: SettingsFile,
+    /// The computer's time zone as the app found it at its start: what the
+    /// System zone of the settings is.
+    pub system_zone: jiff::tz::TimeZone,
     pub locale: Locale,
     pub palette: Palette,
     pub look: crate::theme::Look,
@@ -180,6 +183,7 @@ impl App {
             dirs,
             settings,
             settings_file,
+            system_zone: system_zone(),
             locale: Locale::default(),
             palette: Palette::dark(),
             look: crate::theme::Look::for_platform(),
@@ -432,6 +436,11 @@ impl App {
         !self.cancellable(tab).is_empty()
     }
 
+    /// The zone a value with an offset is shown in, as the settings say.
+    pub fn zone(&self) -> jiff::tz::TimeZone {
+        self.settings.tz.resolve(&self.system_zone)
+    }
+
     /// How the look in use binds and writes its keys.
     pub fn layout(&self) -> crate::keymap::Layout {
         crate::keymap::Layout::of(&self.look)
@@ -473,6 +482,7 @@ impl App {
                 continue;
             };
             let shown = workspace.row_panel_tab();
+            let zone = workspace.zone.clone();
             for sql in workspace.sql_tabs_mut() {
                 let row = sql.selected_row().filter(|_| shown == Some(sql.id));
                 let Some(row) = row else {
@@ -484,9 +494,16 @@ impl App {
                     continue;
                 }
                 let request = sql.run.loaded;
-                let values = sql.shown_rows().and_then(|(_, rows, _)| rows.get(row));
+                let Some((columns, rows, _)) = sql.shown_rows() else {
+                    sql.fields = None;
+                    continue;
+                };
+                let kinds: Vec<_> = columns.iter().map(|column| column.kind).collect();
                 let nothing = std::collections::BTreeMap::new();
-                sql.fields = values.map(|values| row_fields(request, row, values, &nothing));
+                let fields = rows
+                    .get(row)
+                    .map(|values| row_fields(request, row, values, &kinds, &zone, &nothing));
+                sql.fields = fields;
             }
             for object in workspace.object_tabs_mut() {
                 let row = object
@@ -506,18 +523,19 @@ impl App {
                 // it is made again from the set as it stands.
                 let request = object.rows.loaded;
                 let cells = &object.edits.cells;
+                let kinds: Vec<_> = page.columns.iter().map(|column| column.kind).collect();
                 object.fields = match crate::edit::new_id(row) {
                     // A new row loaded nothing: each field is what was
                     // set in it, and a NULL to draw where nothing was.
                     Some(new) => {
                         let held = object.edits.added.iter().any(|added| added.id == new);
                         let blank = vec![tabletist_db::Value::Null; page.columns.len()];
-                        held.then(|| row_fields(request, row, &blank, cells))
+                        held.then(|| row_fields(request, row, &blank, &kinds, &zone, cells))
                     }
                     None => page
                         .rows
                         .get(row)
-                        .map(|values| row_fields(request, row, values, cells)),
+                        .map(|values| row_fields(request, row, values, &kinds, &zone, cells)),
                 };
             }
         }
@@ -2326,11 +2344,13 @@ impl App {
         let request = RequestId(self.next_id());
         // Where the option puts it; the grid's link switches it from there.
         let full_precision = self.settings.timestamps == crate::settings::Timestamps::Full;
+        let zone = self.zone();
         let Some(entry) = self.tabs.iter_mut().find(|t| t.id == tab) else {
             return;
         };
         let mut workspace = Workspace::new(session, request, saved, typed);
         workspace.full_precision = full_precision;
+        workspace.zone = zone;
         entry.content = ConnTabContent::Workspace(Box::new(workspace));
     }
 
@@ -3638,6 +3658,21 @@ impl App {
                 }
             }
         }
+        if old.tz != self.settings.tz {
+            let zone = self.zone();
+            for tab in &mut self.tabs {
+                if let ConnTabContent::Workspace(workspace) = &mut tab.content {
+                    workspace.zone = zone.clone();
+                    // The row panels' texts are made again, in the zone.
+                    for sql in workspace.sql_tabs_mut() {
+                        sql.fields = None;
+                    }
+                    for object in workspace.object_tabs_mut() {
+                        object.fields = None;
+                    }
+                }
+            }
+        }
         if old.show_system_schemas != self.settings.show_system_schemas {
             // A completion list offers the schemas that are shown, and an
             // open one is worked out again only when its script, its cursor
@@ -3901,6 +3936,19 @@ impl App {
 }
 
 /// Whether the request failed only because the connection went away.
+/// The computer's time zone, or UTC where it cannot be found. Tests have
+/// UTC whatever the computer they run on says, so what they show does not
+/// depend on it: a test of the System zone sets `App::system_zone`.
+fn system_zone() -> jiff::tz::TimeZone {
+    if cfg!(test) {
+        return jiff::tz::TimeZone::UTC;
+    }
+    jiff::tz::TimeZone::try_system().unwrap_or_else(|error| {
+        log::warn!("the system's time zone could not be found, UTC is used: {error}");
+        jiff::tz::TimeZone::UTC
+    })
+}
+
 fn lost<T>(fetch: &Fetch<T>) -> bool {
     fetch.error.as_ref().is_some_and(Error::is_connection_lost)
 }
@@ -5338,10 +5386,15 @@ fn moved(object: &ObjectTab, rows: isize, cols: isize) -> Option<CellPos> {
 /// row `row` that is pending in `cells` reads as its new value, and keeps
 /// what it loaded as beside it, so the panel never disagrees with the
 /// grid.
+/// The row panel's texts of a row: each value as the panel shows it, the
+/// pending one where there is one, and a moment with an offset (`kinds`
+/// says which columns hold dates and times) in `zone`.
 fn row_fields(
     request: Option<RequestId>,
     row: usize,
     values: &[tabletist_db::Value],
+    kinds: &[tabletist_db::ValueKind],
+    zone: &jiff::tz::TimeZone,
     cells: &std::collections::BTreeMap<(usize, usize), crate::edit::Pending>,
 ) -> crate::model::RowFields {
     use crate::ui::format::{cell_text, field_text};
@@ -5361,7 +5414,20 @@ fn row_fields(
     let fields = values
         .iter()
         .zip(&pending)
-        .map(|(loaded, pending)| field_text(pending.as_ref().map_or(loaded, |cell| &cell.new)))
+        .enumerate()
+        .map(|(col, (loaded, pending))| {
+            let value = pending.as_ref().map_or(loaded, |cell| &cell.new);
+            let temporal = kinds.get(col) == Some(&tabletist_db::ValueKind::Temporal);
+            match value {
+                tabletist_db::Value::Text(text) if temporal => {
+                    match crate::ui::format::in_zone(text, zone) {
+                        Some(moved) => field_text(&tabletist_db::Value::Text(moved.into())),
+                        None => field_text(value),
+                    }
+                }
+                _ => field_text(value),
+            }
+        })
         .collect();
     crate::model::RowFields {
         request,
diff --git a/src/model.rs b/src/model.rs
index 5aba7cf..8a08da1 100644
--- a/src/model.rs
+++ b/src/model.rs
@@ -931,6 +931,9 @@ pub struct Workspace {
     pub sidebar_hidden: bool,
     /// The grid shows timestamps in full rather than to the second.
     pub full_precision: bool,
+    /// The zone a value with an offset is shown in: the settings' zone,
+    /// set when the workspace opens and when the option changes.
+    pub zone: jiff::tz::TimeZone,
     /// Focus the WHERE line on the next frame.
     pub focus_where: bool,
     /// The terminal look's `:` prompt: the text after the colon while it
@@ -3992,6 +3995,7 @@ impl Workspace {
             recent: Vec::new(),
             sidebar_hidden: false,
             full_precision: false,
+            zone: jiff::tz::TimeZone::UTC,
             focus_where: false,
             command: None,
             focus_command: false,
diff --git a/src/ui/data_view.rs b/src/ui/data_view.rs
index 5066094..f50fd35 100644
--- a/src/ui/data_view.rs
+++ b/src/ui/data_view.rs
@@ -870,6 +870,9 @@ pub struct Fit {
     pub grouped: bool,
     /// Values of a closed set as tags, which pad their text.
     pub value_tags: bool,
+    /// A hash of the zone's name: a value moved to another zone can be
+    /// wider.
+    pub zone: Option<u64>,
 }
 
 impl Fit {
@@ -878,6 +881,12 @@ impl Fit {
             full_precision: workspace.full_precision,
             grouped: app.settings.grouped(app.layout()),
             value_tags: app.settings.value_tags,
+            zone: workspace.zone.iana_name().map(|name| {
+                use std::hash::{Hash as _, Hasher as _};
+                let mut hasher = std::hash::DefaultHasher::new();
+                name.hash(&mut hasher);
+                hasher.finish()
+            }),
         }
     }
 }
@@ -1353,6 +1362,8 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabId)
         return;
     };
     let fit = Fit::of(workspace, app);
+    // Kept apart from the workspace, which the editor's text is taken from.
+    let zone = workspace.zone.clone();
     // The arrows are the grid's once the user worked in it.
     let keys = workspace.pane == crate::model::Pane::Grid;
     let Some(object) = workspace.object_tab(object_tab) else {
@@ -1499,6 +1510,7 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabId)
             .map(|column| Shown {
                 full_precision: fit.full_precision,
                 grouped: fit.grouped && !is_key(&column.name, structure),
+                zone: Some(&zone),
             })
             .collect();
         // What is pending, and what else of editing the cells show: read
@@ -2462,11 +2474,14 @@ fn empty_rows(
 
 /// How a cell writes its value out.
 #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
-pub struct Shown {
+pub struct Shown<'z> {
     /// Timestamps with the fraction the server sent.
     pub full_precision: bool,
     /// A number's integer digits in threes.
     pub grouped: bool,
+    /// The zone a value with an offset is shown in. None shows it as the
+    /// server sent it.
+    pub zone: Option<&'z jiff::tz::TimeZone>,
 }
 
 /// Whether `name` is a column of the table's primary key or of one of its
@@ -2596,6 +2611,15 @@ pub fn plain_cell<'a>(
             }
         }
     };
+    // A moment with an offset in the zone it is shown in, before it is
+    // cut to the second.
+    let text = match shown.zone {
+        Some(zone) if kind == ValueKind::Temporal => match format::in_zone(&text, zone) {
+            Some(moved) => moved.into(),
+            None => text,
+        },
+        _ => text,
+    };
     let text = if kind == ValueKind::Temporal && !shown.full_precision {
         match format::to_the_second(&text) {
             std::borrow::Cow::Borrowed(_) => text,
diff --git a/src/ui/format.rs b/src/ui/format.rs
index 0453434..3127015 100644
--- a/src/ui/format.rs
+++ b/src/ui/format.rs
@@ -423,6 +423,162 @@ pub fn to_the_second(text: &str) -> Cow<'_, str> {
     Cow::Owned(format!("{before}{}", &rest[digits..]))
 }
 
+/// A date and a time as a server writes them, with what follows them:
+/// `2026-01-12 09:14:03.482915+00` is the date and time, the fraction's six
+/// digits, the separator ` `, and `+00`.
+struct Stamp<'a> {
+    at: jiff::civil::DateTime,
+    separator: char,
+    /// The fraction's digits as written, none where it has none.
+    digits: usize,
+    rest: &'a str,
+}
+
+/// Reads `text` as `YYYY-MM-DD HH:MM:SS[.fraction]` (or with a `T`), and
+/// whatever comes after it. `None` for anything else: another form, or a
+/// date or time that is no date or time.
+fn stamp(text: &str) -> Option<Stamp<'_>> {
+    let bytes = text.as_bytes();
+    let number = |range: std::ops::Range<usize>| -> Option<i32> {
+        let part = text.get(range)?;
+        part.bytes()
+            .all(|byte| byte.is_ascii_digit())
+            .then(|| part.parse().ok())?
+    };
+    if bytes.len() < 19
+        || bytes[4] != b'-'
+        || bytes[7] != b'-'
+        || bytes[13] != b':'
+        || bytes[16] != b':'
+    {
+        return None;
+    }
+    let separator = match bytes[10] {
+        b' ' => ' ',
+        b'T' => 'T',
+        _ => return None,
+    };
+    let mut end = 19;
+    let mut digits = 0;
+    if bytes.get(19) == Some(&b'.') {
+        digits = bytes[20..]
+            .iter()
+            .take_while(|byte| byte.is_ascii_digit())
+            .count();
+        if digits == 0 || digits > 9 {
+            return None;
+        }
+        end = 20 + digits;
+    }
+    let nanos = if digits == 0 {
+        0
+    } else {
+        let written: i32 = text[20..end].parse().ok()?;
+        written * 10_i32.pow(9 - digits as u32)
+    };
+    let at = jiff::civil::DateTime::new(
+        i16::try_from(number(0..4)?).ok()?,
+        i8::try_from(number(5..7)?).ok()?,
+        i8::try_from(number(8..10)?).ok()?,
+        i8::try_from(number(11..13)?).ok()?,
+        i8::try_from(number(14..16)?).ok()?,
+        i8::try_from(number(17..19)?).ok()?,
+        nanos,
+    )
+    .ok()?;
+    Some(Stamp {
+        at,
+        separator,
+        digits,
+        rest: &text[end..],
+    })
+}
+
+/// An offset as PostgreSQL writes one: `+00`, `-05`, `+05:30`,
+/// `-03:30:15`. `None` for anything else.
+fn offset_of(text: &str) -> Option<jiff::tz::Offset> {
+    let sign = match text.as_bytes().first()? {
+        b'+' => 1,
+        b'-' => -1,
+        _ => return None,
+    };
+    let mut parts = text[1..].split(':');
+    let mut seconds = 0;
+    for (index, scale) in [3_600, 60, 1].into_iter().enumerate() {
+        let Some(part) = parts.next() else {
+            // The hours are always there.
+            if index == 0 {
+                return None;
+            }
+            break;
+        };
+        if part.len() != 2 || !part.bytes().all(|byte| byte.is_ascii_digit()) {
+            return None;
+        }
+        seconds += part.parse::<i32>().ok()? * scale;
+    }
+    if parts.next().is_some() {
+        return None;
+    }
+    jiff::tz::Offset::from_seconds(sign * seconds).ok()
+}
+
+/// An offset as [`offset_of`] reads it: the minutes and the seconds only
+/// where they are not nothing.
+fn written_offset(offset: jiff::tz::Offset) -> String {
+    let seconds = offset.seconds();
+    let sign = if seconds < 0 { '-' } else { '+' };
+    let seconds = seconds.unsigned_abs();
+    let (hours, minutes, seconds) = (seconds / 3_600, seconds / 60 % 60, seconds % 60);
+    match (minutes, seconds) {
+        (0, 0) => format!("{sign}{hours:02}"),
+        (_, 0) => format!("{sign}{hours:02}:{minutes:02}"),
+        _ => format!("{sign}{hours:02}:{minutes:02}:{seconds:02}"),
+    }
+}
+
+/// `at` written as a [`Stamp`] is, with `digits` of its fraction.
+fn written_stamp(at: jiff::civil::DateTime, separator: char, digits: usize) -> String {
+    let mut out = format!(
+        "{:04}-{:02}-{:02}{separator}{:02}:{:02}:{:02}",
+        at.year(),
+        at.month(),
+        at.day(),
+        at.hour(),
+        at.minute(),
+        at.second()
+    );
+    if digits > 0 {
+        let nanos = format!("{:09}", at.subsec_nanosecond());
+        out.push('.');
+        out.push_str(&nanos[..digits]);
+    }
+    out
+}
+
+/// A value with an offset (a `timestamptz`, as PostgreSQL writes one) as
+/// the same moment in `zone`, with that zone's offset then, and the
+/// fraction the server sent: `2026-01-12 09:14:03.482915+00` in New York
+/// is `2026-01-12 04:14:03.482915-05`. `None` for text that is no such
+/// value: one without an offset, a time with no date, `infinity`, a date
+/// before Christ (PostgreSQL writes ` BC` after the offset).
+pub fn in_zone(text: &str, zone: &jiff::tz::TimeZone) -> Option<String> {
+    let stamp = stamp(text)?;
+    let offset = offset_of(stamp.rest)?;
+    let moment = offset.to_timestamp(stamp.at).ok()?;
+    let shown = moment.to_zoned(zone.clone());
+    // Before the year 1 PostgreSQL writes ` BC`, which a value moved there
+    // would need: it is shown as the server sent it.
+    if shown.year() < 1 {
+        return None;
+    }
+    Some(format!(
+        "{}{}",
+        written_stamp(shown.datetime(), stamp.separator, stamp.digits),
+        written_offset(shown.offset())
+    ))
+}
+
 /// `text` cut in the middle with "…" to fit `max` points as measured by
 /// `width`, keeping both ends (a file's name and its extension).
 pub fn ellipsize_middle(text: &str, max: f32, width: impl Fn(&str) -> f32) -> String {
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
index 0ee7cf6..b50af97 100644
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -44,8 +44,6 @@ pub mod keys;
 pub mod label_chooser;
 #[cfg(test)]
 mod label_tests;
-#[cfg(test)]
-mod zone_tests;
 pub mod menu;
 pub mod object_tabs;
 pub mod password_prompt;
@@ -69,6 +67,8 @@ pub mod value_tags;
 pub mod widgets;
 pub mod workspace;
 pub mod write_prompts;
+#[cfg(test)]
+mod zone_tests;
 
 use egui::Frame;
 
diff --git a/src/ui/sql_results.rs b/src/ui/sql_results.rs
index 9b7d014..eb1885b 100644
--- a/src/ui/sql_results.rs
+++ b/src/ui/sql_results.rs
@@ -186,6 +186,8 @@ struct Place<'a> {
     sql: &'a SqlTab,
     /// How the cells are drawn, as the workspace's tables draw them.
     fit: data_view::Fit,
+    /// The zone a value with an offset is shown in.
+    zone: &'a jiff::tz::TimeZone,
     /// Whose error codes the results read.
     driver: tabletist_db::Driver,
     /// The write a database refused in the last run, when that run was
@@ -213,6 +215,7 @@ fn draw(app: &App, ui: &mut Ui, tab: ConnTabId, id: TabId, actions: &mut Vec<Act
         tab,
         sql,
         fit: data_view::Fit::of(workspace, app),
+        zone: &workspace.zone,
         driver: workspace.driver,
         blocked: blocked(workspace, sql),
         keys: workspace.pane == crate::model::Pane::Grid,
@@ -1579,6 +1582,7 @@ fn results(ui: &mut Ui, run: &SqlRun, place: &Place<'_>, env: &Env<'_>, actions:
         tab,
         sql,
         fit,
+        zone,
         keys,
         ..
     } = *place;
@@ -1711,6 +1715,7 @@ fn results(ui: &mut Ui, run: &SqlRun, place: &Place<'_>, env: &Env<'_>, actions:
                     full_precision: fit.full_precision,
                     // A result has no key to leave alone.
                     grouped: fit.grouped,
+                    zone: Some(zone),
                 },
             )
         },
````

- [ ] **Step 4: Check**

Run: `cargo fmt --all && cargo clippy --locked --workspace --all-targets -- -D warnings && cargo test --locked --workspace --all-targets`
Expected: no warning, every test passes.

- [ ] **Step 5: Commit**

```bash
git add docs/_reference/settings-and-files.md src/app.rs src/model.rs src/testing.rs src/ui/data_view.rs src/ui/format.rs src/ui/mod.rs src/ui/sql_results.rs src/ui/zone_tests.rs
git commit -m "Show a timestamptz in the zone of the settings"
```

### Task 6: Send a moment typed in the zone with its offset

The editor opens on the moment as the cell shows it; one typed with no offset into a PostgreSQL `timestamptz` gets the zone's; the same moment typed back is no change in a column of dates and times. The editing guide says so.

**Files:** Modify `docs/_guide/editing-data.md`; Modify `docs/_reference/settings-and-files.md`; Modify `src/app/editing.rs`; Modify `src/edit.rs`; Modify `src/key_picker.rs`; Modify `src/review.rs`; Modify `src/testing.rs`; Modify `src/ui/format.rs`; Modify `src/ui/zone_tests.rs`.

- [ ] **Step 1: Write the failing tests**

````diff
diff --git a/src/edit.rs b/src/edit.rs
index 79b550a..e21154a 100644
--- a/src/edit.rs
+++ b/src/edit.rs
@@ -3124,6 +3124,7 @@ mod tests {
             gone: &edits.gone,
             added: &edits.added,
             deleted: &edits.deleted,
+            zone: &crate::testing::UTC,
         }
     }
 
@@ -4753,6 +4754,7 @@ mod tests {
             gone: &NONE_GONE,
             added: &[],
             deleted: &NONE_DELETED,
+            zone: &crate::testing::UTC,
         }
     }
 
diff --git a/src/key_picker.rs b/src/key_picker.rs
index 96f3917..caa370f 100644
--- a/src/key_picker.rs
+++ b/src/key_picker.rs
@@ -1363,6 +1363,7 @@ mod tests {
                     gone: &gone,
                     added: &edits.added,
                     deleted: &edits.deleted,
+                    zone: &crate::testing::UTC,
                 },
                 &own,
                 edits,
@@ -1459,6 +1460,7 @@ mod tests {
             gone: &gone,
             added: &edits.added,
             deleted: &edits.deleted,
+            zone: &crate::testing::UTC,
         };
         assert!(unmatched(&table, &own, &edits).is_empty());
         // Another table's key of the same number is no row of this one.
diff --git a/src/review.rs b/src/review.rs
index ab731c8..1892f3c 100644
--- a/src/review.rs
+++ b/src/review.rs
@@ -761,6 +761,7 @@ mod tests {
             gone: &NONE_GONE,
             added: &[],
             deleted: &crate::edit::NONE_DELETED,
+            zone: &crate::testing::UTC,
         }
     }
 
diff --git a/src/testing.rs b/src/testing.rs
index 8c48b2e..f311bad 100644
--- a/src/testing.rs
+++ b/src/testing.rs
@@ -803,6 +803,9 @@ pub fn book_covers_page(rows: usize) -> RowPage {
     }
 }
 
+/// UTC, for a test's [`crate::edit::Table`] to hold a zone by reference.
+pub static UTC: jiff::tz::TimeZone = jiff::tz::TimeZone::UTC;
+
 /// When the Bookshop's covers were made, as PostgreSQL sends a
 /// `timestamptz` in a session whose zone is UTC.
 pub const COVERS_MADE: &str = "2026-01-12 09:14:03.482915+00";
diff --git a/src/ui/format.rs b/src/ui/format.rs
index 3127015..e01c0ca 100644
--- a/src/ui/format.rs
+++ b/src/ui/format.rs
@@ -1179,6 +1179,73 @@ mod tests {
         );
     }
 
+    #[test]
+    fn a_moment_written_in_another_zone_is_the_same_moment() {
+        assert!(same_moment(
+            "2026-01-12 09:14:03.482915+00",
+            "2026-01-12 04:14:03.482915-05"
+        ));
+        assert!(!same_moment(
+            "2026-01-12 09:14:03+00",
+            "2026-01-12 09:14:03-05"
+        ));
+        // Without an offset a text names no moment.
+        assert!(!same_moment("2026-01-12 09:14:03", "2026-01-12 09:14:03"));
+        // Typed back in another form of the same moment.
+        assert!(same_moment(
+            "2026-01-12 09:14:03+00",
+            "2026-01-12T09:14:03Z"
+        ));
+        assert!(same_moment(
+            "2026-01-12 09:14:03+00",
+            "2026-01-12 09:14:03+00:00"
+        ));
+    }
+
+    #[test]
+    fn a_date_and_time_typed_with_no_offset_gets_the_zones() {
+        let zone = new_york();
+        assert_eq!(
+            with_offset("2026-01-12 04:14:03", &zone).as_deref(),
+            Some("2026-01-12 04:14:03-05")
+        );
+        assert_eq!(
+            with_offset(" 2026-07-01 08:00 ", &zone).as_deref(),
+            Some("2026-07-01 08:00-04")
+        );
+        assert_eq!(
+            with_offset("2026-01-12T04:14:03.5", &zone).as_deref(),
+            Some("2026-01-12T04:14:03.5-05")
+        );
+        // A date alone is its midnight there.
+        assert_eq!(
+            with_offset("2026-01-12", &zone).as_deref(),
+            Some("2026-01-12 00:00:00-05")
+        );
+        // The hour the clocks skip, and the one they have twice, take the
+        // offset from before the change.
+        assert_eq!(
+            with_offset("2026-03-08 02:30:00", &zone).as_deref(),
+            Some("2026-03-08 02:30:00-05")
+        );
+        assert_eq!(
+            with_offset("2026-11-01 01:30:00", &zone).as_deref(),
+            Some("2026-11-01 01:30:00-04")
+        );
+        // Typed with an offset, or as no date: as typed.
+        for typed in [
+            "2026-01-12 04:14:03+02",
+            "2026-01-12T04:14:03Z",
+            "soon",
+            "2026-01-12T04:14[America/New_York]",
+            "-002026-01-12",
+            "+002026-01-12",
+            "04:14",
+        ] {
+            assert_eq!(with_offset(typed, &zone), None, "{typed}");
+        }
+    }
+
     #[test]
     fn what_is_no_moment_with_an_offset_is_left_as_it_is() {
         let zone = new_york();
diff --git a/src/ui/zone_tests.rs b/src/ui/zone_tests.rs
index 8ebeb9b..9c3c189 100644
--- a/src/ui/zone_tests.rs
+++ b/src/ui/zone_tests.rs
@@ -195,3 +195,129 @@ fn the_row_panel_shows_the_system_zone_too() {
             .is_some()
     );
 }
+
+/// The covers' tab on PostgreSQL, the one engine with an offset to send.
+fn covers_on_postgres() -> (Harness, ConnTabId, TabId) {
+    let (mut harness, tab, id) = covers();
+    harness.app.workspace_mut(tab).unwrap().driver = tabletist_db::Driver::Postgres;
+    (harness, tab, id)
+}
+
+/// Opens the editor on `created_at` of the first row, from its value.
+fn open_editor(harness: &mut Harness, tab: ConnTabId, id: TabId) {
+    harness.app.apply(Action::EditCell {
+        tab,
+        id,
+        cell: CellPos { row: 0, col: MADE },
+        start: crate::model::EditStart::Value,
+    });
+}
+
+fn editor_text(harness: &Harness, tab: ConnTabId, id: TabId) -> String {
+    let workspace = harness.app.workspace(tab).unwrap();
+    let object = workspace.object_tab(id).unwrap();
+    object
+        .edits
+        .editor
+        .as_ref()
+        .expect("an editor")
+        .text
+        .clone()
+}
+
+/// Types `text` into the open editor and commits it.
+fn commit(harness: &mut Harness, tab: ConnTabId, id: TabId, text: &str) {
+    let workspace = harness.app.workspace_mut(tab).unwrap();
+    let object = workspace.object_tab_mut(id).unwrap();
+    object.edits.editor.as_mut().expect("an editor").text = text.to_owned();
+    harness.app.apply(Action::EditorTyped { tab, id });
+    harness.app.apply(Action::CommitEdit {
+        tab,
+        id,
+        then: crate::model::Advance::Stay,
+    });
+}
+
+/// The cell's pending text, where it has one.
+fn pending(harness: &Harness, tab: ConnTabId, id: TabId) -> Option<String> {
+    let workspace = harness.app.workspace(tab).unwrap();
+    let object = workspace.object_tab(id).unwrap();
+    match &object.edits.cells.get(&(0, MADE))?.new {
+        tabletist_db::NewValue::Text(text) => Some(text.clone()),
+        other => panic!("not a text: {other:?}"),
+    }
+}
+
+#[test]
+fn the_editor_opens_on_the_moment_as_the_cell_shows_it() {
+    let (mut harness, tab, id) = covers_on_postgres();
+    zone(&mut harness, new_york());
+    open_editor(&mut harness, tab, id);
+    assert_eq!(
+        editor_text(&harness, tab, id),
+        "2026-01-12 04:14:03.482915-05"
+    );
+    // Committed as it opened, or as the server wrote it: the moment that
+    // was loaded, and no change.
+    commit(&mut harness, tab, id, "2026-01-12 04:14:03.482915-05");
+    assert_eq!(pending(&harness, tab, id), None);
+    open_editor(&mut harness, tab, id);
+    commit(&mut harness, tab, id, COVERS_MADE);
+    assert_eq!(pending(&harness, tab, id), None);
+}
+
+#[test]
+fn a_moment_typed_without_an_offset_is_sent_with_the_zones() {
+    let (mut harness, tab, id) = covers_on_postgres();
+    zone(&mut harness, new_york());
+    open_editor(&mut harness, tab, id);
+    commit(&mut harness, tab, id, "2026-01-12 06:00:00");
+    assert_eq!(
+        pending(&harness, tab, id).as_deref(),
+        Some("2026-01-12 06:00:00-05")
+    );
+    // One typed with an offset is sent as typed.
+    open_editor(&mut harness, tab, id);
+    commit(&mut harness, tab, id, "2026-01-12 06:00:00+01");
+    assert_eq!(
+        pending(&harness, tab, id).as_deref(),
+        Some("2026-01-12 06:00:00+01")
+    );
+}
+
+#[test]
+fn an_engine_that_keeps_no_offset_is_sent_what_was_typed() {
+    // The fake session is SQLite's.
+    let (mut harness, tab, id) = covers();
+    zone(&mut harness, new_york());
+    open_editor(&mut harness, tab, id);
+    commit(&mut harness, tab, id, "2026-01-12 06:00:00");
+    assert_eq!(
+        pending(&harness, tab, id).as_deref(),
+        Some("2026-01-12 06:00:00")
+    );
+}
+
+#[test]
+fn a_text_column_takes_a_moment_written_another_way_as_a_change() {
+    // `created_at` is TEXT here: what it holds is text, not a moment.
+    let mut harness = Harness::new();
+    let tab = harness.connect_fake_as(false);
+    harness.app.apply(Action::OpenObject {
+        tab,
+        object: tabletist_db::ObjectRef::new("main", "book_covers"),
+        kind: tabletist_db::ObjectKind::Table,
+        pin: true,
+    });
+    harness.answer_structure(crate::testing::book_covers_structure());
+    let mut page = crate::testing::book_covers_page(1);
+    page.rows[0][MADE] = tabletist_db::Value::Text(COVERS_MADE.into());
+    harness.answer_rows(page);
+    let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
+    open_editor(&mut harness, tab, id);
+    commit(&mut harness, tab, id, "2026-01-12T09:14:03.482915Z");
+    assert_eq!(
+        pending(&harness, tab, id).as_deref(),
+        Some("2026-01-12T09:14:03.482915Z")
+    );
+}
````

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --locked -p tabletist --lib`
Expected: FAIL to compile: `` struct `edit::Table<'_>` has no field named `zone` ``; `` cannot find function `same_moment` in this scope ``; `` cannot find function `with_offset` in this scope ``.

- [ ] **Step 3: Write the code**

````diff
diff --git a/docs/_guide/editing-data.md b/docs/_guide/editing-data.md
index 29c1300..0f6f359 100644
--- a/docs/_guide/editing-data.md
+++ b/docs/_guide/editing-data.md
@@ -43,6 +43,15 @@ sent: numbers and their ranges, booleans, enum values, the maximum length,
 and the syntax of JSON. A foreign key you type is also looked up in the
 table it points at, as a [warning](#foreign-keys).
 
+### Dates and times with a zone
+
+A PostgreSQL `timestamptz` is shown, and opens to be edited, in the zone
+of [Time zone for timestamptz]({% link _reference/settings-and-files.md %}#the-settings-window),
+with that zone's offset. A value you type without an offset is sent with
+the zone's, so the server reads it as the time you see; one with an
+offset is sent as you typed it. Typing back the same moment, in any zone,
+changes nothing.
+
 ### Words that are not text
 
 In a column that is not text, three words typed into a cell are what they
diff --git a/docs/_reference/settings-and-files.md b/docs/_reference/settings-and-files.md
index 11787de..bb30eda 100644
--- a/docs/_reference/settings-and-files.md
+++ b/docs/_reference/settings-and-files.md
@@ -67,7 +67,7 @@ notify = true
 | --- | --- | --- |
 | `data.page_size` | 10 to 10000. A table takes a new size when it next loads (opened, refreshed, sorted, filtered or reconnected), not while you page through it | `1000` |
 | `data.timestamps` | `"second"` or `"full"` | `"second"` |
-| `data.tz` | `"system"`, `"utc"` or a zone such as `"America/New_York"`. A name the zone database does not know is a line that cannot be read. A `timestamptz` value is shown in this zone, and a copy gives it as the server sent it | `"system"` |
+| `data.tz` | `"system"`, `"utc"` or a zone such as `"America/New_York"`. A name the zone database does not know is a line that cannot be read. A `timestamptz` value is shown and edited in this zone, a copy gives it as the server sent it, and one you type without an offset is sent with this zone's | `"system"` |
 | `data.group_digits` | `true` or `false` | Not set: grouped on macOS and Windows, not in the Omarchy look |
 | `data.value_tags` | `true` or `false` | `true` |
 | `sidebar.show_system_schemas` | `true` shows schemas such as `pg_catalog` and `information_schema` | `false` |
diff --git a/src/app/editing.rs b/src/app/editing.rs
index 03dc240..b3fd938 100644
--- a/src/app/editing.rs
+++ b/src/app/editing.rs
@@ -158,11 +158,27 @@ fn typed(table: &Table<'_>, object: &ObjectTab) -> Option<Typed> {
         return None;
     }
     // What was typed, as the column reads it: a keyword where it is one.
-    let new = crate::edit::typed_as(table.dialect, column, &editor.text);
+    let mut new = crate::edit::typed_as(table.dialect, column, &editor.text);
+    // A moment typed with no offset is the zone's, where the column keeps
+    // one: the server would read it in its own.
+    if let NewValue::Text(text) = &new
+        && crate::edit::keeps_offset(table.dialect, &column.type_name)
+        && let Some(sent) = crate::ui::format::with_offset(text, table.zone)
+    {
+        new = NewValue::Text(sent);
+    }
+    // A moment typed back as it was, in the zone it is shown in or in any
+    // other, is the moment that was loaded: in a column of dates and
+    // times, where it is a moment and not text.
+    let temporal = table.page.columns.get(cell.col).map(|column| column.kind)
+        == Some(tabletist_db::ValueKind::Temporal);
     let changed = match (&new, loaded) {
         // A new row's unset cell is its default already.
         (NewValue::Default { .. }, None) => false,
-        (new, loaded) => loaded.is_none_or(|loaded| is_change(loaded, new, class)),
+        (new, loaded) => loaded.is_none_or(|loaded| {
+            is_change(loaded, new, class)
+                && !(temporal && crate::edit::same_moment_typed(loaded, new))
+        }),
     };
     let problem = changed
         .then(|| crate::edit::problem(table.dialect, column, &editor.text))
@@ -789,9 +805,20 @@ impl App {
                         NewValue::Null | NewValue::Default { .. } | NewValue::Now => String::new(),
                     },
                     // A new row's cell starts from nothing.
+                    // A moment with an offset as the cell shows it: in
+                    // the zone, and edited there.
                     None => table
                         .loaded(cell)
-                        .map(|loaded| start_text(loaded, class))
+                        .map(|loaded| {
+                            let start = start_text(loaded, class);
+                            let temporal =
+                                table.page.columns.get(cell.col).map(|column| column.kind)
+                                    == Some(tabletist_db::ValueKind::Temporal);
+                            temporal
+                                .then(|| crate::ui::format::in_zone(&start, table.zone))
+                                .flatten()
+                                .unwrap_or(start)
+                        })
                         .unwrap_or_default(),
                 },
             };
diff --git a/src/edit.rs b/src/edit.rs
index e21154a..7a3b96c 100644
--- a/src/edit.rs
+++ b/src/edit.rs
@@ -122,6 +122,8 @@ pub struct Table<'a> {
     pub added: &'a [NewRow],
     /// The page's rows marked for deletion (`Edits::deleted`).
     pub deleted: &'a BTreeMap<usize, Doomed>,
+    /// The zone a moment with an offset is shown and typed in.
+    pub zone: &'a jiff::tz::TimeZone,
 }
 
 impl<'a> Table<'a> {
@@ -141,6 +143,7 @@ impl<'a> Table<'a> {
             gone: &object.edits.gone,
             added: &object.edits.added,
             deleted: &object.edits.deleted,
+            zone: &workspace.zone,
         })
     }
 }
@@ -972,6 +975,31 @@ pub fn is_change(loaded: &Value, new: &NewValue, class: ColumnClass) -> bool {
     }
 }
 
+/// Whether `new` is the moment `loaded` holds, written another way: in
+/// another zone, with `Z`. Only a column of dates and times holds a moment:
+/// the caller asks of those alone, as a text column's text is its own.
+pub fn same_moment_typed(loaded: &Value, new: &NewValue) -> bool {
+    match (loaded, new) {
+        (Value::Text(was), NewValue::Text(text)) => crate::ui::format::same_moment(was, text),
+        _ => false,
+    }
+}
+
+/// Whether a column of `type_name` keeps a moment with its offset, so that
+/// a value typed into it without one is read in the server session's zone
+/// (PostgreSQL's `timestamptz`). MySQL's `TIMESTAMP` and SQLite's dates
+/// keep no offset: what is typed is what is kept.
+pub fn keeps_offset(dialect: Dialect, type_name: &str) -> bool {
+    match dialect {
+        Dialect::Postgres => {
+            let name = type_name.trim().to_ascii_lowercase();
+            name.starts_with("timestamptz")
+                || (name.starts_with("timestamp") && name.ends_with("with time zone"))
+        }
+        Dialect::MySql | Dialect::Sqlite => false,
+    }
+}
+
 /// What `text`, typed into a cell of `column`, is as the cell's new value.
 /// A word is a keyword only where the column is not text, and only where
 /// the column can take it: NULL where it may be NULL, DEFAULT where it has
diff --git a/src/ui/format.rs b/src/ui/format.rs
index e01c0ca..84e30c4 100644
--- a/src/ui/format.rs
+++ b/src/ui/format.rs
@@ -579,6 +579,58 @@ pub fn in_zone(text: &str, zone: &jiff::tz::TimeZone) -> Option<String> {
     ))
 }
 
+/// Whether `a` and `b` are both moments with an offset (as [`in_zone`]
+/// reads one) and the same moment, however each is written: what was
+/// loaded, and the same moment shown in another zone.
+pub fn same_moment(a: &str, b: &str) -> bool {
+    // As the server writes one, or as a person may type one: `Z`, `+00:00`.
+    let moment = |text: &str| {
+        let text = text.trim();
+        let written =
+            stamp(text).and_then(|stamp| offset_of(stamp.rest)?.to_timestamp(stamp.at).ok());
+        written.or_else(|| text.parse::<jiff::Timestamp>().ok())
+    };
+    matches!((moment(a), moment(b)), (Some(a), Some(b)) if a == b)
+}
+
+/// `text`, a date and a time typed with no offset, with the offset `zone`
+/// has at that time: `2026-01-12 04:14` in New York is sent as
+/// `2026-01-12 04:14-05`, so the server reads it in the zone the person
+/// sees and not in its session's. A date alone is its midnight. A time the
+/// zone skips or has twice (a change of the clocks) takes the offset from
+/// before the change. `None` for anything else: a text with an offset
+/// already, or no date and time.
+pub fn with_offset(text: &str, zone: &jiff::tz::TimeZone) -> Option<String> {
+    let typed = text.trim();
+    // One with an offset, or a `Z`, reads as a moment: it is sent as typed.
+    if typed.parse::<jiff::Timestamp>().is_ok() {
+        return None;
+    }
+    // Only a date of four digits for its year, as PostgreSQL reads one,
+    // and no zone's name in brackets, which it does not: anything else is
+    // the person's to send as typed.
+    let bytes = typed.as_bytes();
+    let plain = bytes.len() >= 10
+        && bytes[..4].iter().all(u8::is_ascii_digit)
+        && bytes[4] == b'-'
+        && !typed.contains('[');
+    if !plain {
+        return None;
+    }
+    let at: jiff::civil::DateTime = typed.parse().ok()?;
+    let offset = match zone.to_ambiguous_timestamp(at).offset() {
+        jiff::tz::AmbiguousOffset::Unambiguous { offset } => offset,
+        jiff::tz::AmbiguousOffset::Gap { before, .. }
+        | jiff::tz::AmbiguousOffset::Fold { before, .. } => before,
+    };
+    let typed = if typed.contains(':') {
+        typed.to_owned()
+    } else {
+        written_stamp(at, ' ', 0)
+    };
+    Some(format!("{typed}{}", written_offset(offset)))
+}
+
 /// `text` cut in the middle with "…" to fit `max` points as measured by
 /// `width`, keeping both ends (a file's name and its extension).
 pub fn ellipsize_middle(text: &str, max: f32, width: impl Fn(&str) -> f32) -> String {
````

- [ ] **Step 4: Check**

Run: `cargo fmt --all && cargo clippy --locked --workspace --all-targets -- -D warnings && cargo test --locked --workspace --all-targets`
Expected: no warning, every test passes.

- [ ] **Step 5: Commit**

```bash
git add docs/_guide/editing-data.md docs/_reference/settings-and-files.md src/app/editing.rs src/edit.rs src/key_picker.rs src/review.rs src/testing.rs src/ui/format.rs src/ui/zone_tests.rs
git commit -m "Edit a timestamptz in the zone it is shown in, and send it with its offset"
```

### Task 7: The zone's row in both looks, and Reset's question

The Time zone for timestamptz row: a menu with a search on macOS and Windows, a stepper on Omarchy. Reset to defaults asks the prompt's question on General.

**Files:** Modify `docs/_reference/settings-and-files.md`; Modify `src/app.rs`; Modify `src/model.rs`; Modify `src/settings.rs`; Modify `src/ui/mod.rs`; Modify `src/ui/settings/agents.rs`; Modify `src/ui/settings/mod.rs`; Modify `src/ui/settings/sheet.rs`; Modify `src/ui/settings/terminal.rs`; Modify `src/ui/zone_tests.rs`.

- [ ] **Step 1: Write the failing tests**

````diff
diff --git a/src/app.rs b/src/app.rs
index 0c1a8f8..9850a6a 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -7720,12 +7720,13 @@ mod tests {
     }
 
     #[test]
-    fn reset_asks_first_and_then_resets_only_the_four_options() {
+    fn reset_asks_first_and_then_resets_only_the_general_options() {
         let (mut app, _dir) = app();
         app.apply(Action::ShowSettings);
-        // An option of the window, and a key it does not show.
+        // Options of the window, and a key it does not show.
         app.change_settings(|settings| {
             settings.page_size = 500;
+            settings.tz = crate::settings::Zone::Utc;
             settings.group_digits = Some(false);
             settings.show_system_schemas = true;
             settings.sql_limit = 50;
@@ -7741,12 +7742,13 @@ mod tests {
         app.apply(Action::ConfirmResetSettings(false));
         assert!(!resetting(&app));
         assert_eq!(app.settings.page_size, 500);
-        // Reset puts the four back and leaves the others.
+        // Reset puts the General tab's back and leaves the others.
         app.apply(Action::ResetSettings);
         app.apply(Action::ConfirmResetSettings(true));
         assert!(!resetting(&app));
         let defaults = Settings::default();
         assert_eq!(app.settings.page_size, defaults.page_size);
+        assert_eq!(app.settings.tz, defaults.tz);
         assert_eq!(app.settings.group_digits, defaults.group_digits);
         assert!(app.settings.show_system_schemas);
         assert_eq!(app.settings.sql_limit, 50);
diff --git a/src/settings.rs b/src/settings.rs
index 93dd2fb..5ab948e 100644
--- a/src/settings.rs
+++ b/src/settings.rs
@@ -1764,6 +1764,37 @@ theme = \"My \\\"Nord\\\".json\"
         );
     }
 
+    #[test]
+    fn the_zone_steps_through_system_utc_and_the_one_the_file_names() {
+        let step = |tz: Zone, forward: bool| {
+            let settings = Settings {
+                tz,
+                ..Settings::default()
+            };
+            settings.stepped(OptionId::Tz, forward)
+        };
+        let lisbon = || Zone::Named("Europe/Lisbon".into());
+        assert_eq!(step(Zone::System, true), OptionValue::Tz(Zone::Utc));
+        assert_eq!(step(Zone::System, false), OptionValue::Tz(Zone::System));
+        assert_eq!(step(Zone::Utc, true), OptionValue::Tz(Zone::Utc));
+        assert_eq!(step(lisbon(), false), OptionValue::Tz(Zone::Utc));
+        assert_eq!(step(lisbon(), true), OptionValue::Tz(lisbon()));
+        assert_eq!(Settings::default().flipped(OptionId::Tz), None);
+        // Stepped off, the zone the session had is a step away still.
+        let utc = Settings {
+            tz: Zone::Utc,
+            ..Settings::default()
+        };
+        assert_eq!(
+            utc.stepped_zone(Some("Europe/Lisbon"), true),
+            OptionValue::Tz(lisbon())
+        );
+        assert_eq!(
+            utc.stepped_zone(Some("Europe/Lisbon"), false),
+            OptionValue::Tz(Zone::System)
+        );
+    }
+
     #[test]
     fn an_empty_file_starts_with_the_header() {
         assert_eq!(
@@ -2271,6 +2302,7 @@ theme = \"My \\\"Nord\\\".json\"
             vec![
                 Key::PageSize,
                 Key::Timestamps,
+                Key::Tz,
                 Key::GroupDigits,
                 Key::ValueTags,
                 Key::McpEnabled,
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
index b50af97..6317f67 100644
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -1049,7 +1049,16 @@ mod tests {
 
     /// A harness in the terminal look with the Settings screen open.
     fn settings_screen() -> Harness {
-        let mut harness = Harness::new();
+        settings_screen_in(Harness::new())
+    }
+
+    /// The Settings screen as the artboard draws it: on a screen of
+    /// 1920 by 1080, where every hint has room beside the file.
+    fn settings_screen_on_the_artboard() -> Harness {
+        settings_screen_in(Harness::with_size(egui::vec2(1920.0, 1080.0)))
+    }
+
+    fn settings_screen_in(mut harness: Harness) -> Harness {
         harness.set_look(crate::theme::Look::omarchy());
         harness.press(egui::Key::Comma, egui::Modifiers::COMMAND);
         assert!(settings_open(&harness));
@@ -1399,7 +1408,7 @@ value_tags   = true
             let mut harness = settings_sheet(look());
             harness.click("Timestamps: Full precision");
             harness.click("Reset to defaults");
-            assert!(harness.has("Reset every option on this tab?"));
+            assert!(harness.has("Reset all General settings to their defaults?"));
             assert!(!harness.has("Export…"), "the links give way");
             harness.click("Cancel");
             assert!(harness.has("Export…"));
@@ -1450,7 +1459,7 @@ value_tags   = true
         harness.click("Timestamps: Full precision");
         tab_to_settings(&mut harness, "Reset to defaults");
         harness.press(egui::Key::Enter, egui::Modifiers::NONE);
-        assert!(harness.has("Reset every option on this tab?"));
+        assert!(harness.has("Reset all General settings to their defaults?"));
         assert_eq!(focused_name(&harness.settle()), "Cancel");
         // A second Enter is the answer that changes nothing.
         harness.press(egui::Key::Enter, egui::Modifiers::NONE);
@@ -1483,7 +1492,7 @@ value_tags   = true
         for _ in 0..4 {
             harness.frame(vec![held(true)]);
         }
-        assert!(!harness.has("Reset every option on this tab?"));
+        assert!(!harness.has("Reset all General settings to their defaults?"));
         harness.frame(vec![crate::testing::release(egui::Key::Space, none)]);
         // Escape answers no, and takes the keyboard from where it was: it
         // is put back on the link too.
@@ -1491,7 +1500,7 @@ value_tags   = true
         assert_eq!(focused_name(&harness.settle()), "Cancel");
         harness.press(egui::Key::Escape, egui::Modifiers::NONE);
         assert!(settings_open(&harness));
-        assert!(!harness.has("Reset every option on this tab?"));
+        assert!(!harness.has("Reset all General settings to their defaults?"));
         assert_eq!(focused_name(&harness.settle()), "Reset to defaults");
     }
 
@@ -1551,7 +1560,7 @@ value_tags   = true
             let lead = harness.painted_rect(lead).expect("the tab's lead");
             assert!(switch.top() > lead.bottom(), "{switch:?} {lead:?}");
             // And back up with it: the first row's menu.
-            for _ in 0..3 {
+            for _ in 0..4 {
                 harness.press(egui::Key::Tab, egui::Modifiers::SHIFT);
             }
             let tree = harness.settle();
@@ -1702,8 +1711,8 @@ value_tags   = true
 
     #[test]
     fn h_and_l_change_the_cursors_option_and_save_it() {
-        use crate::settings::Timestamps;
-        let mut harness = settings_screen();
+        use crate::settings::{Timestamps, Zone};
+        let mut harness = settings_screen_on_the_artboard();
         // Rows per page.
         harness.press(egui::Key::L, egui::Modifiers::NONE);
         assert_eq!(harness.app.settings.page_size, 5_000);
@@ -1726,6 +1735,17 @@ value_tags   = true
                 .painted_color("2026-01-12 09:14:03.482915")
                 .is_some()
         );
+        // The zone: the computer's, then UTC, and no further.
+        harness.press(egui::Key::J, egui::Modifiers::NONE);
+        assert!(harness.painted_color("‹ system (utc) ›").is_some());
+        harness.press(egui::Key::L, egui::Modifiers::NONE);
+        assert_eq!(harness.app.settings.tz, Zone::Utc);
+        assert!(harness.painted_color("‹ utc ›").is_some());
+        let saved = settings_saves(&harness);
+        harness.press(egui::Key::L, egui::Modifiers::NONE);
+        assert_eq!(settings_saves(&harness), saved);
+        harness.press(egui::Key::H, egui::Modifiers::NONE);
+        assert_eq!(harness.app.settings.tz, Zone::System);
         // Numbers: grouped is the left of the two.
         harness.press(egui::Key::J, egui::Modifiers::NONE);
         harness.press(egui::Key::H, egui::Modifiers::NONE);
@@ -1739,7 +1759,7 @@ value_tags   = true
         // Space on the page size does nothing: it has more than two values.
         harness.press(egui::Key::Space, egui::Modifiers::NONE);
         assert_eq!(settings_saves(&harness), 0);
-        for _ in 0..3 {
+        for _ in 0..4 {
             harness.press(egui::Key::J, egui::Modifiers::NONE);
         }
         harness.press(egui::Key::Space, egui::Modifiers::NONE);
@@ -1806,7 +1826,7 @@ value_tags   = true
         use crate::settings::Timestamps;
         let mut harness = settings_screen();
         harness.click("Numbers");
-        assert_eq!(settings_cursor(&harness), 2);
+        assert_eq!(settings_cursor(&harness), 3);
         harness.click("Timestamps: full");
         assert_eq!(harness.app.settings.timestamps, Timestamps::Full);
         assert_eq!(settings_cursor(&harness), 1, "the cursor follows the click");
@@ -2269,14 +2289,14 @@ value_tags   = true
     fn a_focused_button_of_the_settings_screen_keeps_space_and_the_arrows() {
         let mut harness = settings_screen();
         // The cursor on an option Space flips.
-        for _ in 0..3 {
+        for _ in 0..4 {
             harness.press(egui::Key::J, egui::Modifiers::NONE);
         }
         tab_to_settings(&mut harness, "Numbers");
         // Space presses the button, which is another row's: the cursor
         // goes there, and the option it was on is as it was.
         harness.press(egui::Key::Space, egui::Modifiers::NONE);
-        assert_eq!(settings_cursor(&harness), 2);
+        assert_eq!(settings_cursor(&harness), 3);
         assert!(harness.app.settings.value_tags);
         // An arrow moves the keyboard from the button, and nothing else.
         // Where it takes the keyboard is egui's to say: each arrow starts
@@ -2286,10 +2306,12 @@ value_tags   = true
         assert_eq!(settings_saves(&harness), 0);
         tab_to_settings(&mut harness, "Numbers");
         harness.press(egui::Key::ArrowUp, egui::Modifiers::NONE);
-        assert_eq!(settings_cursor(&harness), 2);
+        assert_eq!(settings_cursor(&harness), 3);
         // The letters are the screen's wherever the keyboard is.
         tab_to_settings(&mut harness, "Close");
         harness.press(egui::Key::K, egui::Modifiers::NONE);
+        assert_eq!(settings_cursor(&harness), 2);
+        harness.press(egui::Key::K, egui::Modifiers::NONE);
         assert_eq!(settings_cursor(&harness), 1);
         harness.press(egui::Key::L, egui::Modifiers::NONE);
         assert_eq!(
diff --git a/src/ui/zone_tests.rs b/src/ui/zone_tests.rs
index 9c3c189..756707e 100644
--- a/src/ui/zone_tests.rs
+++ b/src/ui/zone_tests.rs
@@ -321,3 +321,158 @@ fn a_text_column_takes_a_moment_written_another_way_as_a_change() {
         Some("2026-01-12T09:14:03.482915Z")
     );
 }
+
+/// Gives the keyboard to the field labelled `label`, as a screen reader
+/// does: a click through AccessKit presses, and a field takes no press.
+fn focus(harness: &mut Harness, label: &str) {
+    let tree = harness.settle();
+    let (target, _) = tree
+        .nodes
+        .iter()
+        .find(|(_, node)| node.label() == Some(label))
+        .unwrap_or_else(|| panic!("nothing labelled {label:?}"));
+    let target = *target;
+    harness.frame(vec![egui::Event::AccessKitActionRequest(
+        egui::accesskit::ActionRequest {
+            target_tree: egui::accesskit::TreeId::ROOT,
+            target_node: target,
+            action: egui::accesskit::Action::Focus,
+            data: None,
+        },
+    )]);
+}
+
+/// The Settings sheet of `look`, open.
+fn sheet(look: crate::theme::Look) -> Harness {
+    let mut harness = Harness::new();
+    harness.set_look(look);
+    harness.press(Key::Comma, Modifiers::COMMAND);
+    assert!(matches!(
+        harness.app.dialog,
+        Some(crate::model::Dialog::Settings(_))
+    ));
+    harness
+}
+
+#[test]
+fn the_sheets_menu_picks_a_zone_found_by_its_name() {
+    for look in [crate::theme::Look::macos(), crate::theme::Look::standard()] {
+        let mut harness = sheet(look);
+        // Its row, and what it says under its label.
+        assert!(harness.has("Time zone for timestamptz"), "{}", look.name);
+        assert!(
+            harness.has("Values are shown in this zone and sent with their offset"),
+            "{}",
+            look.name
+        );
+        harness.click("Time zone for timestamptz");
+        // The computer's zone with its name, and UTC, first.
+        assert!(harness.has("Time zone for timestamptz System · UTC"));
+        harness.click("Time zone for timestamptz UTC");
+        assert_eq!(harness.app.settings.tz, Zone::Utc, "{}", look.name);
+        // A zone by its name: typed into the menu's field, then picked.
+        harness.click("Time zone for timestamptz");
+        focus(&mut harness, "Search zones");
+        harness.frame(vec![egui::Event::Text("lisb".into())]);
+        harness.settle();
+        assert!(!harness.has("Time zone for timestamptz Asia/Tokyo"));
+        harness.click("Time zone for timestamptz Europe/Lisbon");
+        assert_eq!(harness.app.settings.tz, Zone::Named("Europe/Lisbon".into()));
+        assert!(
+            harness
+                .app
+                .settings_file
+                .text
+                .contains("tz = \"Europe/Lisbon\"")
+        );
+        // Opened again, it lists every zone.
+        harness.click("Time zone for timestamptz");
+        assert!(
+            harness.has("Time zone for timestamptz Asia/Tokyo"),
+            "{}",
+            look.name
+        );
+    }
+}
+
+#[test]
+fn the_terminal_screen_names_the_row_as_its_artboard_does() {
+    let mut harness = Harness::new();
+    harness.set_look(crate::theme::Look::omarchy());
+    harness.press(Key::Comma, Modifiers::COMMAND);
+    assert!(harness.painted_color("timestamptz zone").is_some());
+    assert!(harness.has("timestamptz zone"));
+}
+
+#[test]
+fn the_file_pane_marks_the_zones_line() {
+    let mut harness = Harness::with_size(egui::vec2(1920.0, 1080.0));
+    harness.set_look(crate::theme::Look::omarchy());
+    harness.press(Key::Comma, Modifiers::COMMAND);
+    harness
+        .app
+        .apply(Action::Backend(crate::backend::Event::SettingsFile {
+            text: "[data]\npage_size = 500\ntz = \"utc\"\n".into(),
+            own: false,
+        }));
+    for _ in 0..2 {
+        harness.press(Key::J, Modifiers::NONE);
+    }
+    let selection = harness.app.palette.selection;
+    let line = harness.painted_rect("tz = ").expect("the zone's line");
+    assert!(harness.fills.iter().any(|(rect, color)| {
+        *color == selection
+            && rect.y_range().contains(line.center().y)
+            && rect.x_range().contains(line.center().x)
+    }));
+}
+
+#[test]
+fn a_step_off_the_files_zone_can_step_back_to_it() {
+    let mut harness = Harness::with_size(egui::vec2(1920.0, 1080.0));
+    harness.set_look(crate::theme::Look::omarchy());
+    harness.press(Key::Comma, Modifiers::COMMAND);
+    harness
+        .app
+        .apply(Action::Backend(crate::backend::Event::SettingsFile {
+            text: "[data]\ntz = \"America/New_York\"\n".into(),
+            own: false,
+        }));
+    for _ in 0..2 {
+        harness.press(Key::J, Modifiers::NONE);
+    }
+    harness.press(Key::H, Modifiers::NONE);
+    assert_eq!(harness.app.settings.tz, Zone::Utc);
+    harness.press(Key::L, Modifiers::NONE);
+    assert_eq!(harness.app.settings.tz, new_york());
+    assert!(harness.painted_color("‹ america/new_york ›").is_some());
+}
+
+#[test]
+fn a_zone_the_database_does_not_know_is_reported_in_both_looks() {
+    let text = "[data]\ntz = \"Mars/Olympus\"\n";
+    // Omarchy: the line in the colour of an error.
+    let mut harness = Harness::with_size(egui::vec2(1920.0, 1080.0));
+    harness.set_look(crate::theme::Look::omarchy());
+    harness.press(Key::Comma, Modifiers::COMMAND);
+    harness
+        .app
+        .apply(Action::Backend(crate::backend::Event::SettingsFile {
+            text: text.into(),
+            own: false,
+        }));
+    harness.settle();
+    assert_eq!(harness.app.settings.tz, Zone::System);
+    let danger = harness.app.palette.danger;
+    assert_eq!(harness.painted_color("tz = \"Mars/Olympus\""), Some(danger));
+    // macOS: the footer counts it.
+    let mut harness = sheet(crate::theme::Look::macos());
+    harness
+        .app
+        .apply(Action::Backend(crate::backend::Event::SettingsFile {
+            text: text.into(),
+            own: false,
+        }));
+    harness.settle();
+    assert!(harness.has("1 line in the file could not be read and was ignored"));
+}
````

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --locked -p tabletist --lib`
Expected: FAIL to compile: `` no variant, associated function, or constant named `Tz` found for enum `settings::OptionId` in the current scope ``; `` no variant, associated function, or constant named `Tz` found for enum `settings::OptionValue` in the current scope ``; `` no method named `stepped_zone` found for struct `settings::Settings` in the current scope ``.

- [ ] **Step 3: Write the code**

````diff
diff --git a/docs/_reference/settings-and-files.md b/docs/_reference/settings-and-files.md
index bb30eda..af4e10e 100644
--- a/docs/_reference/settings-and-files.md
+++ b/docs/_reference/settings-and-files.md
@@ -8,12 +8,13 @@ nav_order: 2
 
 Cmd/Ctrl+, opens Settings. Changes apply right away.
 
-{% include shot.html file="macos-settings" alt="The Settings window with its four options" %}
+{% include shot.html file="macos-settings" alt="The Settings window with its General options" %}
 
 | Option | Choices | Default |
 | --- | --- | --- |
 | **Rows per page** | 100, 500, 1,000, 5,000 or 10,000. A table takes a new size when it next loads, not while you page through it | 1,000 |
 | **Timestamps** | To the second, or full precision | To the second |
+| **Time zone for timestamptz** | System, UTC or any zone. A `timestamptz` is shown in this zone, a copy gives the value as the server sent it, and a value you type without an offset is sent with this zone's. In the Omarchy look `h`/`l` choose System, UTC or the last zone the file named; any other zone is set in the file | System |
 | **Numbers** | `1,240.50` or `1240.50`. Grouping is for display only: a copy gives the raw value | `1,240.50` on macOS and Windows, `1240.50` in the Omarchy look |
 | **Value tags** | Colors for enum, CHECK and boolean columns | On |
 
diff --git a/src/app.rs b/src/app.rs
index 9850a6a..5444e6d 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -82,6 +82,9 @@ pub struct App {
     /// The computer's time zone as the app found it at its start: what the
     /// System zone of the settings is.
     pub system_zone: jiff::tz::TimeZone,
+    /// The last named zone the settings had this session: the Omarchy
+    /// screen's steps reach it after a step off it.
+    pub named_zone: Option<String>,
     pub locale: Locale,
     pub palette: Palette,
     pub look: crate::theme::Look,
@@ -179,11 +182,16 @@ impl App {
             Some(crate::labels::database_key(spec.driver, &spec.database))
         });
         let agents_shown = crate::agents_shown::Shown::load(&dirs.agents_shown_file());
+        let named_zone = match &settings.tz {
+            crate::settings::Zone::Named(name) => Some(name.clone()),
+            crate::settings::Zone::System | crate::settings::Zone::Utc => None,
+        };
         let mut app = Self {
             dirs,
             settings,
             settings_file,
             system_zone: system_zone(),
+            named_zone,
             locale: Locale::default(),
             palette: Palette::dark(),
             look: crate::theme::Look::for_platform(),
@@ -3658,6 +3666,9 @@ impl App {
                 }
             }
         }
+        if let crate::settings::Zone::Named(name) = &self.settings.tz {
+            self.named_zone = Some(name.clone());
+        }
         if old.tz != self.settings.tz {
             let zone = self.zone();
             for tab in &mut self.tabs {
diff --git a/src/model.rs b/src/model.rs
index 8a08da1..56be103 100644
--- a/src/model.rs
+++ b/src/model.rs
@@ -2164,6 +2164,7 @@ impl SettingsPane {
             Self::General => &[
                 SettingsRow::Option(OptionId::PageSize),
                 SettingsRow::Option(OptionId::Timestamps),
+                SettingsRow::Option(OptionId::Tz),
                 SettingsRow::Option(OptionId::GroupDigits),
                 SettingsRow::Option(OptionId::ValueTags),
             ],
diff --git a/src/settings.rs b/src/settings.rs
index 5ab948e..f91cc45 100644
--- a/src/settings.rs
+++ b/src/settings.rs
@@ -201,6 +201,8 @@ impl Key {
 pub enum OptionId {
     PageSize,
     Timestamps,
+    /// The zone a `timestamptz` is shown in: `data.tz`.
+    Tz,
     GroupDigits,
     ValueTags,
     /// Whether agents can reach the app: `mcp.enabled`.
@@ -213,9 +215,10 @@ pub enum OptionId {
 
 impl OptionId {
     /// Every option, the General pane's and then the Agents pane's.
-    pub const ALL: [OptionId; 7] = [
+    pub const ALL: [OptionId; 8] = [
         Self::PageSize,
         Self::Timestamps,
+        Self::Tz,
         Self::GroupDigits,
         Self::ValueTags,
         Self::AgentsOn,
@@ -223,9 +226,10 @@ impl OptionId {
         Self::AgentsNotify,
     ];
     /// The General pane's, in the order it has them.
-    pub const GENERAL: [OptionId; 4] = [
+    pub const GENERAL: [OptionId; 5] = [
         Self::PageSize,
         Self::Timestamps,
+        Self::Tz,
         Self::GroupDigits,
         Self::ValueTags,
     ];
@@ -237,6 +241,7 @@ impl OptionId {
         match self {
             Self::PageSize => Key::PageSize,
             Self::Timestamps => Key::Timestamps,
+            Self::Tz => Key::Tz,
             Self::GroupDigits => Key::GroupDigits,
             Self::ValueTags => Key::ValueTags,
             Self::AgentsOn => Key::McpEnabled,
@@ -252,10 +257,11 @@ impl OptionId {
 }
 
 /// An option with a value: what a key, a click or a reset sets.
-#[derive(Clone, Copy, Debug, PartialEq, Eq)]
+#[derive(Clone, Debug, PartialEq, Eq)]
 pub enum OptionValue {
     PageSize(u32),
     Timestamps(Timestamps),
+    Tz(Zone),
     /// `None` is the look's default.
     GroupDigits(Option<bool>),
     ValueTags(bool),
@@ -265,10 +271,11 @@ pub enum OptionValue {
 }
 
 impl OptionValue {
-    pub fn option(self) -> OptionId {
+    pub fn option(&self) -> OptionId {
         match self {
             Self::PageSize(_) => OptionId::PageSize,
             Self::Timestamps(_) => OptionId::Timestamps,
+            Self::Tz(_) => OptionId::Tz,
             Self::GroupDigits(_) => OptionId::GroupDigits,
             Self::ValueTags(_) => OptionId::ValueTags,
             Self::AgentsOn(_) => OptionId::AgentsOn,
@@ -278,10 +285,11 @@ impl OptionValue {
     }
 
     /// Puts the value in `settings`.
-    pub fn set(self, settings: &mut Settings) {
-        match self {
+    pub fn set(&self, settings: &mut Settings) {
+        match self.clone() {
             Self::PageSize(size) => settings.page_size = size,
             Self::Timestamps(choice) => settings.timestamps = choice,
+            Self::Tz(zone) => settings.tz = zone,
             Self::GroupDigits(on) => settings.group_digits = on,
             Self::ValueTags(on) => settings.value_tags = on,
             Self::AgentsOn(on) => settings.mcp_enabled = on,
@@ -735,6 +743,7 @@ impl Settings {
         match option {
             OptionId::PageSize => OptionValue::PageSize(self.page_size),
             OptionId::Timestamps => OptionValue::Timestamps(self.timestamps),
+            OptionId::Tz => OptionValue::Tz(self.tz.clone()),
             OptionId::GroupDigits => OptionValue::GroupDigits(self.group_digits),
             OptionId::ValueTags => OptionValue::ValueTags(self.value_tags),
             OptionId::AgentsOn => OptionValue::AgentsOn(self.mcp_enabled),
@@ -761,6 +770,9 @@ impl Settings {
                 };
                 OptionValue::PageSize(next.unwrap_or(size))
             }
+            // System, UTC, and the zone the file names, where it names
+            // one: any other is set in the file.
+            OptionId::Tz => self.stepped_zone(None, forward),
             OptionId::Timestamps => OptionValue::Timestamps(if forward {
                 Timestamps::Full
             } else {
@@ -786,10 +798,29 @@ impl Settings {
         }
     }
 
+    /// The zone one step from this one: System, UTC, then a named zone,
+    /// the one set or else `named` (the last the session had), so that a
+    /// step off it can come back. Any other zone is set in the file.
+    pub fn stepped_zone(&self, named: Option<&str>, forward: bool) -> OptionValue {
+        let mut zones = vec![Zone::System, Zone::Utc];
+        match (&self.tz, named) {
+            (Zone::Named(_), _) => zones.push(self.tz.clone()),
+            (Zone::System | Zone::Utc, Some(name)) => zones.push(Zone::Named(name.to_owned())),
+            (Zone::System | Zone::Utc, None) => {}
+        }
+        let at = zones.iter().position(|zone| *zone == self.tz).unwrap_or(0);
+        let to = if forward {
+            (at + 1).min(zones.len() - 1)
+        } else {
+            at.saturating_sub(1)
+        };
+        OptionValue::Tz(zones.swap_remove(to))
+    }
+
     /// The other value of an option that has two; `None` for one with more.
     pub fn flipped(&self, option: OptionId) -> Option<OptionValue> {
         match option {
-            OptionId::PageSize => None,
+            OptionId::PageSize | OptionId::Tz => None,
             OptionId::Timestamps => Some(OptionValue::Timestamps(match self.timestamps {
                 Timestamps::Second => Timestamps::Full,
                 Timestamps::Full => Timestamps::Second,
diff --git a/src/ui/settings/agents.rs b/src/ui/settings/agents.rs
index 2b10aad..a981d1f 100644
--- a/src/ui/settings/agents.rs
+++ b/src/ui/settings/agents.rs
@@ -162,6 +162,7 @@ fn status(ui: &mut Ui, app: &App, skin: &Skin, actions: &mut Vec<Action>) {
         look,
         palette,
         locale,
+        ..
     } = *skin;
     let role = TextRole::UiBody;
     let on = app.agents_on();
@@ -280,6 +281,7 @@ fn connect(ui: &mut Ui, agent: Agent, skin: &Skin, actions: &mut Vec<Action>) {
         look,
         palette,
         locale,
+        ..
     } = *skin;
     // The switch, and what the agent takes beside it, wrapped in what the
     // switch leaves.
@@ -384,6 +386,7 @@ fn access(ui: &mut Ui, skin: &Skin, actions: &mut Vec<Action>) {
         look,
         palette,
         locale,
+        ..
     } = *skin;
     let label = gettext(locale, "Open Connections");
     let key = crate::ui::keys::written(ui.ctx(), look, Command::Connections);
diff --git a/src/ui/settings/mod.rs b/src/ui/settings/mod.rs
index 973c470..d073ac6 100644
--- a/src/ui/settings/mod.rs
+++ b/src/ui/settings/mod.rs
@@ -16,7 +16,7 @@ use crate::app::App;
 use crate::i18n::{Locale, gettext, ngettext};
 use crate::keymap::{Command, Keymap, Layout, Scope, Spelled, Stroke};
 use crate::model::{Action, AgentsView, Dialog, SettingsDialog, SettingsPane, SettingsRow};
-use crate::settings::{OptionId, OptionValue, Settings, Timestamps};
+use crate::settings::{OptionId, OptionValue, Settings, Timestamps, Zone};
 
 /// The label of an option's row. English: the caller translates, and the
 /// terminal look lower-cases.
@@ -24,6 +24,7 @@ pub(super) fn label(option: OptionId) -> &'static str {
     match option {
         OptionId::PageSize => "Rows per page",
         OptionId::Timestamps => "Timestamps",
+        OptionId::Tz => "Time zone for timestamptz",
         OptionId::GroupDigits => "Numbers",
         OptionId::ValueTags => "Value tags",
         OptionId::AgentsOn => "Let agents use Tabletist",
@@ -32,12 +33,29 @@ pub(super) fn label(option: OptionId) -> &'static str {
     }
 }
 
+/// The label of an option's row on the terminal screen, where its
+/// artboard words it apart from the others' (`timestamptz zone`). English,
+/// lower-cased by the look.
+pub(super) fn terminal_label(option: OptionId) -> &'static str {
+    match option {
+        OptionId::Tz => "timestamptz zone",
+        OptionId::PageSize
+        | OptionId::Timestamps
+        | OptionId::GroupDigits
+        | OptionId::ValueTags
+        | OptionId::AgentsOn
+        | OptionId::LogDays
+        | OptionId::AgentsNotify => label(option),
+    }
+}
+
 /// What a row says under its label, where it says something. English.
 pub(super) fn small_print(option: OptionId) -> Option<&'static str> {
     match option {
         OptionId::PageSize => Some("Table view; the SQL editor has its own limit"),
         OptionId::ValueTags => Some("Colors for enum, CHECK (…IN…) and boolean columns"),
         OptionId::LogDays => Some("Every call and its SQL, never the rows. On this computer only"),
+        OptionId::Tz => Some("Values are shown in this zone and sent with their offset"),
         OptionId::Timestamps
         | OptionId::GroupDigits
         | OptionId::AgentsOn
@@ -45,13 +63,43 @@ pub(super) fn small_print(option: OptionId) -> Option<&'static str> {
     }
 }
 
+/// Every zone of the zone database, by name, in alphabetical order: what
+/// the zone menu lists. Read once.
+pub(super) fn zones() -> &'static [String] {
+    static ZONES: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
+    ZONES.get_or_init(|| {
+        let mut names: Vec<String> = jiff::tz::db()
+            .available()
+            .map(|name| name.to_string())
+            .collect();
+        names.sort_unstable();
+        names.dedup();
+        names
+    })
+}
+
+/// A zone as the window writes it: the computer's with its own name
+/// beside it where it has one (`System · America/New_York`), UTC, or the
+/// zone's name. `system` is the computer's zone's name. English: the
+/// terminal look writes it its own way.
+pub(super) fn zone_text(locale: Locale, zone: &Zone, system: Option<&str>) -> String {
+    match zone {
+        Zone::System => match system {
+            Some(name) => format!("{} · {name}", gettext(locale, "System")),
+            None => gettext(locale, "System").into_owned(),
+        },
+        Zone::Utc => "UTC".to_owned(),
+        Zone::Named(name) => name.clone(),
+    }
+}
+
 /// The days the log keeps a call, as the window writes them.
 pub(super) fn days(locale: Locale, days: u32) -> String {
     ngettext(locale, "{n} day", "{n} days", days).replace("{n}", &days.to_string())
 }
 
 /// Whether an option of on and off is on.
-pub(super) fn is_on(value: OptionValue) -> bool {
+pub(super) fn is_on(value: &OptionValue) -> bool {
     matches!(
         value,
         OptionValue::ValueTags(true)
@@ -109,9 +157,19 @@ impl Said {
 /// The values a click can set on `option`'s row, each with what it is
 /// called: what the row draws as segments or as a check, and what a screen
 /// reader and the tests call them.
-pub(super) fn choices(option: OptionId, settings: &Settings) -> Vec<(Said, OptionValue)> {
+/// `named` is the last named zone the session had, which the zone's steps
+/// reach.
+pub(super) fn choices(
+    option: OptionId,
+    settings: &Settings,
+    named: Option<&str>,
+) -> Vec<(Said, OptionValue)> {
     use Said::{Sample, Word};
     match option {
+        OptionId::Tz => vec![
+            (Word("fewer"), settings.stepped_zone(named, false)),
+            (Word("more"), settings.stepped_zone(named, true)),
+        ],
         // Stepped, not listed: one way and the other.
         OptionId::PageSize => vec![
             (Word("fewer"), settings.stepped(option, false)),
@@ -349,6 +407,9 @@ fn keys(app: &App, ctx: &egui::Context, dialog: &SettingsDialog, actions: &mut V
                     }
                     let value = match asked {
                         Asked::Reset => Some(option.default_value()),
+                        Asked::Step { forward } if option == OptionId::Tz => {
+                            Some(settings.stepped_zone(app.named_zone.as_deref(), forward))
+                        }
                         Asked::Step { forward } => Some(settings.stepped(option, forward)),
                         _ => settings.flipped(option),
                     };
diff --git a/src/ui/settings/sheet.rs b/src/ui/settings/sheet.rs
index 24fa97c..b89a636 100644
--- a/src/ui/settings/sheet.rs
+++ b/src/ui/settings/sheet.rs
@@ -9,7 +9,7 @@ use egui::{CornerRadius, Rect, Sense, Stroke, Ui, WidgetInfo, WidgetType, pos2,
 use crate::app::App;
 use crate::i18n::{Locale, gettext, ngettext};
 use crate::model::{Action, SettingsDialog, SettingsPane};
-use crate::settings::{OptionId, OptionValue, Settings, Timestamps};
+use crate::settings::{OptionId, OptionValue, Settings, Timestamps, Zone};
 use crate::theme::{Look, Palette};
 use crate::typography::{Laid, Text, TextRole};
 use crate::ui::focus::{self, Ring};
@@ -18,7 +18,7 @@ use crate::ui::menu;
 use crate::ui::value_tags::slot_colors;
 use crate::ui::widgets::{self, Segment};
 
-use super::{label, path_shown, sample_timestamp, small_print};
+use super::{label, path_shown, sample_timestamp, small_print, zone_text, zones};
 
 /// The sheet at its widest and at its tallest, and what it leaves of the
 /// window at each side.
@@ -44,6 +44,10 @@ const CONTROLS: f32 = 210.0;
 const HINT_GAP: f32 = 12.0;
 /// The page size menu's button.
 const MENU: f32 = 120.0;
+/// The zone's menu: as wide as the artboard's, for `System · America/…`.
+const ZONE_MENU: f32 = 220.0;
+/// How tall the zone menu's list of zones is at most: it scrolls.
+const ZONE_LIST: f32 = 240.0;
 /// A segment's height, and the switch's.
 const SEGMENT: f32 = 22.0;
 const SWITCH: f32 = 18.0;
@@ -66,6 +70,8 @@ pub(super) struct Skin<'a> {
     pub(super) look: &'a Look,
     pub(super) palette: &'a Palette,
     pub(super) locale: Locale,
+    /// The computer's zone's name, which the System zone writes beside it.
+    pub(super) system: Option<&'a str>,
 }
 
 impl Skin<'_> {
@@ -86,6 +92,7 @@ pub(super) fn show(
         look: &app.look,
         palette: &app.palette,
         locale: app.locale,
+        system: app.system_zone.iana_name(),
     };
     let screen = ctx.content_rect();
     let width = WIDTH.min(screen.width() - 2.0 * MARGIN).max(NAV + 320.0);
@@ -158,7 +165,8 @@ pub(super) fn show(
                 pos2(room.left() + NAV, bottom - foot),
                 pos2(room.right(), bottom),
             );
-            footer(ui, under, app, resetting, ignored.as_ref(), &skin, actions);
+            let asking = resetting.then_some(dialog.pane);
+            footer(ui, under, app, asking, ignored.as_ref(), &skin, actions);
             let side = Rect::from_min_max(room.min, pos2(room.left() + NAV, bottom));
             // The sheet's own corners, inside its border.
             let radius = skin.look.dialog_radius.saturating_sub(1);
@@ -415,7 +423,7 @@ fn keyboard_came_to(ui: &Ui, place: Rect) -> bool {
 /// How tall `option`'s control is: its row is at least as tall.
 fn control_height(option: OptionId, look: &Look) -> f32 {
     match option {
-        OptionId::PageSize => look.control_height,
+        OptionId::PageSize | OptionId::Tz => look.control_height,
         // The track has 2 over its segments and under them.
         OptionId::Timestamps | OptionId::GroupDigits => SEGMENT + 4.0,
         OptionId::ValueTags | OptionId::AgentsOn | OptionId::AgentsNotify => SWITCH,
@@ -466,6 +474,7 @@ fn control(
                 info
             });
         }
+        OptionId::Tz => zone_menu(ui, &name, settings, skin, actions),
         OptionId::Timestamps => {
             let words = [skin.say("To the second"), skin.say("Full precision")];
             let selected = usize::from(settings.timestamps == Timestamps::Full);
@@ -485,7 +494,7 @@ fn control(
             }
         }
         OptionId::ValueTags | OptionId::AgentsOn | OptionId::AgentsNotify => {
-            let on = super::is_on(settings.value(option));
+            let on = super::is_on(&settings.value(option));
             if widgets::toggle(ui, on, &name, palette).clicked()
                 && let Some(flipped) = settings.flipped(option)
             {
@@ -568,13 +577,66 @@ fn page_sizes(current: u32) -> Vec<u32> {
     sizes
 }
 
+/// The zone's menu: the computer's zone and UTC, then every zone of the
+/// database, which a search field over them narrows. What was typed there
+/// is the field's own, kept in egui's memory while the menu is open.
+fn zone_menu(ui: &mut Ui, name: &str, settings: &Settings, skin: &Skin, actions: &mut Vec<Action>) {
+    let Skin { look, palette, .. } = *skin;
+    let shown = zone_text(skin.locale, &settings.tz, skin.system);
+    let search_id = ui.id().with("zone-search");
+    let dropdown = menu::Dropdown::new("zone", &shown, ZONE_MENU);
+    let menu = dropdown.show(ui, look, palette, |ui| {
+        let mut pick = |ui: &mut Ui, zone: Zone| {
+            let text = zone_text(skin.locale, &zone, skin.system);
+            let entry_name = format!("{name} {text}");
+            let entry = menu::Item::choice(&text, zone == settings.tz)
+                .name(&entry_name)
+                .show(ui, look, palette);
+            if entry.clicked() {
+                actions.push(Action::SetOption(OptionValue::Tz(zone)));
+                ui.close();
+            }
+        };
+        pick(ui, Zone::System);
+        pick(ui, Zone::Utc);
+        let mut typed: String = ui.data(|data| data.get_temp(search_id)).unwrap_or_default();
+        let search = skin.say("Search zones");
+        let field = widgets::search_field(ui, &mut typed, &search, look);
+        widgets::add_search(ui, field, look)
+            .widget_info(|| WidgetInfo::labeled(WidgetType::TextEdit, true, &search));
+        ui.data_mut(|data| data.insert_temp(search_id, typed.clone()));
+        let typed = typed.to_lowercase();
+        egui::ScrollArea::vertical()
+            .max_height(ZONE_LIST)
+            .show(ui, |ui| {
+                for zone in zones()
+                    .iter()
+                    .filter(|zone| typed.is_empty() || zone.to_lowercase().contains(&typed))
+                {
+                    pick(ui, Zone::Named(zone.clone()));
+                }
+            });
+    });
+    if menu.inner.is_none() {
+        // Closed: the next opening starts from every zone.
+        ui.data_mut(|data| data.remove::<String>(search_id));
+    }
+    // The row's label is beside it, not its own: named for screen readers,
+    // with the zone as its value.
+    menu.response.widget_info(|| {
+        let mut info = WidgetInfo::labeled(WidgetType::ComboBox, true, name);
+        info.current_text_value = Some(shown.clone());
+        info
+    });
+}
+
 /// What the row says after `option`'s control, from `at` on its middle: a
 /// sample of what the value does, or what it does not do. Whole or left
 /// out: a hint that does not end before `right` tells nothing cut short.
 fn hint(ui: &Ui, at: egui::Pos2, right: f32, option: OptionId, settings: &Settings, skin: &Skin) {
     let Skin { look, palette, .. } = *skin;
     let text = match option {
-        OptionId::PageSize => return,
+        OptionId::PageSize | OptionId::Tz => return,
         OptionId::Timestamps => Text::one(
             look,
             TextRole::MonoSecondary,
@@ -674,14 +736,23 @@ fn footer_height(ui: &Ui, ignored: Option<&Laid>, skin: &Skin) -> f32 {
     (1.0 + FOOT_PAD + ignored + line + FOOT_PAD).ceil()
 }
 
+/// What Reset to defaults asks on `pane`. English.
+fn reset_question(pane: SettingsPane) -> &'static str {
+    match pane {
+        SettingsPane::General => "Reset all General settings to their defaults?",
+        SettingsPane::Agents => "Reset every option on this tab?",
+    }
+}
+
 /// The footer, in `rect` under the content: where the settings file is
-/// and, at the right, what can be done with it. While `resetting`, the
-/// question Reset to defaults asks stands where those links were.
+/// and, at the right, what can be done with it. While the pane in
+/// `asking` asks it, the question Reset to defaults asks stands where
+/// those links were.
 fn footer(
     ui: &Ui,
     rect: Rect,
     app: &App,
-    resetting: bool,
+    asking: Option<SettingsPane>,
     ignored: Option<&Laid>,
     skin: &Skin,
     actions: &mut Vec<Action>,
@@ -707,7 +778,7 @@ fn footer(
     // now. Noted in every frame that asks, and read once.
     let asked = ui.id().with("asked");
     let now = ui.ctx().cumulative_frame_nr();
-    let answered = if resetting {
+    let answered = if asking.is_some() {
         ui.data_mut(|data| data.insert_temp(asked, now));
         false
     } else {
@@ -715,9 +786,9 @@ fn footer(
     };
 
     // The right side first: what it leaves is the path's.
-    let parts = if resetting {
+    let parts = if let Some(pane) = asking {
         vec![
-            (skin.say("Reset every option on this tab?"), None),
+            (skin.say(reset_question(pane)), None),
             (
                 skin.say("Reset"),
                 Some((palette.danger, Action::ConfirmResetSettings(true))),
diff --git a/src/ui/settings/terminal.rs b/src/ui/settings/terminal.rs
index ff34e64..a5ce4dc 100644
--- a/src/ui/settings/terminal.rs
+++ b/src/ui/settings/terminal.rs
@@ -18,7 +18,7 @@ use crate::ui::focus::{self, Ring};
 use crate::ui::value_tags::terminal_slots;
 use crate::ui::widgets::{self, ButtonSpec};
 
-use super::{Said, choices, file_pane, label, opened, sample_timestamp};
+use super::{Said, choices, file_pane, label, opened, sample_timestamp, terminal_label};
 
 /// The header's and the footer's heights, each without its rule.
 const HEADER: f32 = 40.0;
@@ -53,6 +53,10 @@ pub(super) struct Skin<'a> {
     /// Whether a connection window is open: what Enter on the Agents
     /// pane's status and log rows shows the tab of.
     pub(super) window: bool,
+    /// The computer's zone's name, which the System zone writes beside it.
+    pub(super) system: Option<&'a str>,
+    /// The last named zone the session had, which the zone's steps reach.
+    pub(super) named: Option<&'a str>,
 }
 
 impl Skin<'_> {
@@ -94,6 +98,8 @@ pub(super) fn show(
         palette: &app.palette,
         locale: app.locale,
         window: app.agents_window().is_some(),
+        system: app.system_zone.iana_name(),
+        named: app.named_zone.as_deref(),
     };
     let screen = ctx.content_rect();
     let id = egui::Id::new("settings");
@@ -617,7 +623,7 @@ fn widths(ui: &Ui, options: &[OptionId], settings: &Settings, skin: &Skin) -> (f
         .fold(0.0, f32::max);
     let hints = options
         .iter()
-        .flat_map(|option| choices(*option, settings))
+        .flat_map(|option| choices(*option, settings, skin.named))
         .map(|(_, value)| {
             let mut changed = settings.clone();
             value.set(&mut changed);
@@ -662,11 +668,11 @@ fn value_width(ui: &Ui, option: OptionId, settings: &Settings, skin: &Skin) -> f
     let width = |text: &str| role.width(ui.ctx(), skin.look.faces, text);
     match option {
         // Between its chevrons, as the cursor's row has it.
-        OptionId::PageSize | OptionId::LogDays => {
+        OptionId::PageSize | OptionId::LogDays | OptionId::Tz => {
             width(&format!("‹ {} ›", stepped_text(option, settings, skin)))
         }
         OptionId::Timestamps | OptionId::GroupDigits => {
-            let words = choices(option, settings);
+            let words = choices(option, settings, skin.named);
             let between = width(" ") * words.len().saturating_sub(1) as f32;
             let segments = words.into_iter().map(|(word, _)| segment(ui, word, skin));
             segments.sum::<f32>() + between
@@ -682,6 +688,13 @@ fn stepped_text(option: OptionId, settings: &Settings, skin: &Skin) -> String {
         OptionId::LogDays => skin
             .look
             .label(&super::days(skin.locale, settings.mcp_log_days)),
+        // As the artboard writes it: `system (america/new_york)`.
+        OptionId::Tz => skin.look.label(&match (&settings.tz, skin.system) {
+            (crate::settings::Zone::System, Some(name)) => {
+                format!("{} ({name})", gettext(skin.locale, "System"))
+            }
+            (zone, system) => super::zone_text(skin.locale, zone, system),
+        }),
         _ => settings.page_size.to_string(),
     }
 }
@@ -782,14 +795,22 @@ fn option_row(
     let locale = skin.locale;
     // The row's button comes before its values', so that a click on a
     // value is the value's.
-    let (row, left, right, button) =
-        row_frame(ui, (index, label(option)), on_cursor, skin, actions);
+    let (row, left, right, button) = row_frame(
+        ui,
+        (index, terminal_label(option)),
+        on_cursor,
+        skin,
+        actions,
+    );
     let y = row.center().y;
     // A stepped option has no button that is its value: its row says it.
     if let Some(button) = button
-        && matches!(option, OptionId::PageSize | OptionId::LogDays)
+        && matches!(
+            option,
+            OptionId::PageSize | OptionId::LogDays | OptionId::Tz
+        )
     {
-        let name = gettext(locale, label(option));
+        let name = gettext(locale, terminal_label(option));
         let value = stepped_text(option, settings, skin);
         button.widget_info(|| {
             let mut info = WidgetInfo::labeled(WidgetType::Button, true, &name);
@@ -808,7 +829,7 @@ fn option_row(
         on_cursor,
     };
     match option {
-        OptionId::PageSize | OptionId::LogDays => {
+        OptionId::PageSize | OptionId::LogDays | OptionId::Tz => {
             stepper(ui, cell, option, settings, skin, actions)
         }
         OptionId::Timestamps | OptionId::GroupDigits => {
@@ -842,23 +863,23 @@ fn stepper(
     let y = cell.row.center().y;
     let value = stepped_text(option, settings, skin);
     let current = settings.value(option);
-    let [fewer, more] = choices(option, settings)[..] else {
+    let Ok([fewer, more]) = <[_; 2]>::try_from(choices(option, settings, skin.named)) else {
         return;
     };
     let text = if cell.on_cursor {
-        let chevron = |(_, step): (Said, OptionValue)| {
-            if step == current {
+        let chevron = |(_, step): &(Said, OptionValue)| {
+            if *step == current {
                 palette.dim
             } else {
                 palette.accent
             }
         };
         Text::new(look)
-            .add(role, "‹", chevron(fewer))
+            .add(role, "‹", chevron(&fewer))
             .space(role, " ")
             .add(role, &value, palette.text)
             .space(role, " ")
-            .add(role, "›", chevron(more))
+            .add(role, "›", chevron(&more))
     } else {
         Text::one(look, role, &value, palette.text)
     };
@@ -904,7 +925,7 @@ fn check(
 ) {
     let Skin { look, palette, .. } = *skin;
     let current = settings.value(option);
-    let (glyph, color) = if super::is_on(current) {
+    let (glyph, color) = if super::is_on(&current) {
         ("[x]", palette.success)
     } else {
         ("[ ]", palette.dim)
@@ -917,7 +938,7 @@ fn check(
     );
     // One mark stands for both values, and a click on it flips it: the
     // value it is not set to is the button on top.
-    let mut values = choices(option, settings);
+    let mut values = choices(option, settings, skin.named);
     values.sort_by_key(|(_, value)| *value != current);
     for (word, value) in values {
         let press = Press::Value {
@@ -935,6 +956,7 @@ fn hint(ui: &Ui, option: OptionId, settings: &Settings, skin: &Skin) -> Text {
     let dim = |text: &str| Text::one(look, role, text, palette.dim);
     match option {
         OptionId::PageSize => dim(&skin.say("table view")),
+        OptionId::Tz => dim(&skin.say("sent with offset")),
         OptionId::Timestamps => dim(sample_timestamp(settings.timestamps)),
         OptionId::GroupDigits => dim(&skin.say(if settings.group_digits.unwrap_or_default() {
             "grouping on"
@@ -1158,7 +1180,7 @@ fn segments(
     let line = role.row_height(ui.ctx(), look.faces);
     let gap = role.width(ui.ctx(), look.faces, " ");
     let mut left = cell.left;
-    for (word, value) in choices(option, settings) {
+    for (word, value) in choices(option, settings, skin.named) {
         let text = skin.written(word);
         let width = segment(ui, word, skin);
         let place = Rect::from_min_size(
````

- [ ] **Step 4: Check**

Run: `cargo fmt --all && cargo clippy --locked --workspace --all-targets -- -D warnings && cargo test --locked --workspace --all-targets`
Expected: no warning, every test passes.

- [ ] **Step 5: Commit**

```bash
git add docs/_reference/settings-and-files.md src/app.rs src/model.rs src/settings.rs src/ui/mod.rs src/ui/settings/agents.rs src/ui/settings/mod.rs src/ui/settings/sheet.rs src/ui/settings/terminal.rs src/ui/zone_tests.rs
git commit -m "Add the time zone's row to Settings › General, and ask Reset's question of General"
```

## Pull request

One pull request for the seven commits. Its description has:

- a summary;
- the settings built: the writer's rules, Rows per page, Numbers, Time zone for timestamptz;
- the rows not in this step: On launch, Restore tabs and filters, and the three Safety rows (steps 2 and 3);
- the prompt's open questions and the defaults chosen: Numbers by look; the connection id, which arrives with step 3;
- the departures above, and the spec's;
- what was not run: CI, macOS and Windows, and the app by hand;
- that the website's `macos-settings` and `omarchy-settings` pictures still show the four rows of before, until they are rendered again from `src/shots.rs`.

No screenshot unless blurred.
