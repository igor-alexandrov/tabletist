# Settings › Editor, step 1: the pane and the run Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add the Editor pane to Settings in both looks, with every key of the `[editor.run]`, `[editor.typing]`, `[editor.format]` and `[editor.history]` tables in `settings.toml`, and the rows whose features exist: New tabs run in, Row limit, Timeout, the run key's scope (a selection always winning), Autocomplete, Line numbers and Current line highlight. Every open SQL tab follows each change at once.

**Architecture:** The editor's choices are types of their own in a new `src/settings/editor.rs`; `src/settings.rs` gives each a field of `Settings`, a `Key` in the four new tables, and an `OptionId` and `OptionValue`, as Appearance's are. The two old keys (`editor.sql_limit`, `editor.sql_timeout_secs`) stay `Key`s that are read and never written: `Key::replaced` names the old key a new one took the place of, and `write_into` takes the old line out when it writes the new one. The pane plugs into the window's machinery (`SettingsPane::Editor`, `sheet.rs`, `terminal.rs`); options whose features are not built yet have their `OptionId` (so that Reset to defaults puts them back) but no row (`OptionId::EDITOR_LATER`). The editor reads the settings where it acts: `App::new_sql_tab` (the mode), `App::apply_settings` (every open tab's limit and timeout), `App::run_sql` (the scope, and the selection the view reports in `SqlTab::selected`), the completion's two actions and `refresh_completion`, and `sql_text::show` (the gutter and the cursor's line).

**Tech Stack:** Rust, egui (crmne fork) with AccessKit, the `toml` crate's `DeTable`, headless tests through `src/testing.rs`.

**Spec:** `docs/superpowers/specs/2026-10-19-settings-editor-design.md`, section "Step 1: the pane and the run". Steps 2 to 8 get plans of their own.

---

## What was decided

Asked and answered on 2026-10-19:

| Question | Answer |
|---|---|
| Delivery | Eight steps, each its own plan and pull request. This is step 1: the pane, every key, and the rows whose features exist. |
| Row limit | The fetch cap there is today, not `LIMIT` added to the SQL. The board's values and No limit are added. |
| Timeout | The app's timer there is today (a cancel at the time, then a rollback), not server-side timeouts; no MySQL hint is needed. The board's values and Off are added. |
| `ctrl+w` on Omarchy | Stays close tab. The row's hint names `:ro` / `:rw`. |

Taken from the prompt's stated defaults, for the PR to name:

- Look-specific defaults (open question 1): Standard keys, absolute line numbers and UPPERCASE on macOS and Windows; vim, relative and lowercase on Omarchy, while the file says nothing. Written once set, and then every look's.
- Current line on Omarchy (open question 3): no row; it follows `highlight_line`, on by default.
- Keyword case for completions with Keep (open question 5): uppercase. Not acted on in this step (step 3).

## Where this departs from the prompt and the boards

For a yes or a no before code:

| The prompt or a board asks | Step 1 builds | Why |
|---|---|---|
| "Adds LIMIT n … Never rewrites `INSERT … SELECT`" | The fetch cap (above); nothing in the SQL changes | Igor's choice. |
| Server-side timeouts, with a MySQL hint | The app's timer (above) | Igor's choice. |
| Omarchy: "ctrl+w toggles per tab" | `:ro / :rw per tab`, the two keys from the keymap | Igor's choice. |
| macOS small print "Adds LIMIT to results you browse; exports get everything"; Omarchy hint "exports get everything" | "Caps the rows of results you browse" / "caps results you browse" | Nothing adds `LIMIT`, and the editor's results have no export or copy-all. Step 2 (Show all) changes the words again. |
| macOS subtitle "The SQL editor: how queries run, how typing behaves, and what is remembered." | The title alone | Until History (step 6) it says what the pane does not have, as Appearance did before its preview. |
| Omarchy hint "a visual selection always wins" | "a selection always wins" | There is no visual mode before vim (step 7). |
| macOS board: Row limit and Timeout on one row; Line numbers, Current line and Wrap one row of three switches | A row each, labelled as the prompt's table has them | The prompt wins where the two differ, and each control is named by its own setting's label. |
| "Autocomplete … never queries the database while you type" | As today: a name the sidebar has not loaded is read once, the first time a statement names it, and kept | Without it a table's columns are not offered until its tree is opened. The test types a table's name the tree has loaded and sees nothing sent; the columns' path is today's and its tests stand. |
| Test 5: "splitting ignores `;` in strings, comments and dollar quotes" | The run is tested on the fixture's SQLite (a string's and a comment's `;`); a dollar quote is checked on the splitter the run calls, with PostgreSQL's dialect | The fixtures have no PostgreSQL session. |
| The Keybindings, Indent, Close brackets, Wrap, Confirm, Format and History rows | Not drawn; their keys are read, kept and reset | Their features come in steps 3 to 7. |
| (the run key's label) | "⌘↩ runs" and "ctrl+enter runs" come from the keymap's Run statement, and "On ⌃Space" / "ctrl+space" from its Complete | AGENTS.md: a key is spelled in `src/keymap.rs` only. |
| (the toolbar's Limit and Timeout menus) | They offer the setting's values (100 … 10,000 and No limit; 5 s … 5 min and Off) and set the setting, which every open tab follows. "Timeout 60 s" is now "Timeout 1 min" | "Changes apply at once to every open SQL tab": one value, wherever it is set. Today a pick set the tab and the default for new tabs, and left the other tabs. |
| (an old file) | `editor.sql_limit` and `editor.sql_timeout_secs` are read while the new keys are absent, and give way to them when the limit or the timeout is next set. An old timeout over an hour is read as an hour (`"60m"`), the most the new key says | Nobody's limit, or timeout of an hour or less, changes on upgrade. |
| "A value the key cannot have is ignored by its line and reported" | A word or a time no key has (`timeout = "2h"`, `indent = 3`) is; a number out of range (`limit = 20000`, `keep = 200000`) is brought into it without a word | As `page_size` and `log_days` are today. |
| (a new tab on a writable dev connection) | Read-only, the prompt's default; until now such a tab opened read-write | The prompt: "Read-only transaction" is the default. Follow connection gives today's behaviour. |

Found while drafting, fixed here: `the_sheet_shows_what_the_look_has_where_the_settings_do_not_say` (`src/ui/mod.rs`) fails on `main`. It looks for the Numbers segments as buttons, and #177 made every segment a radio button; #176, which added the test, was merged after it (Task 1).

A reviewer given the draft broke it with tests of its own, and the draft now fixes what it found:

- A timeout in the file with a character of more than one byte (`timeout = "5é"`, a Cyrillic `с` for `s`) panicked the app at launch and on a reload: the name was cut on a byte. It is read with `strip_suffix` now, and the line is ignored.
- An old key whose line could not be read, or was rejected by the parser, won over a good new key below it, so a limit set over such a file was lost at the next start. A key read now takes the old key it replaced off the keys that keep their last value; and a new key that cannot be read leaves a good old key standing, as an absent one does.
- A file that held nothing but `[editor] sql_limit` was written with two blank lines over its new table. A text left empty now starts with the table.
- An `[editor]` table written inline (`editor = { sql_timeout_secs = 10 }`) got an `[editor.run]` header under it when the limit was set: TOML refuses a table defined twice, so the line was ignored and the limit lost. A new table under one written inline or with dotted keys now has the file written whole, as a key added to such a table always had (`headed_parents`). Appearance's `[appearance.grid]` had the same gap.
- On Omarchy, `h` on Line numbers (the look's own relative, at the left end) or a click on the relative segment wrote `line_numbers = "relative"`, which macOS would then have taken too. A look's own default picked while nothing is set is not written now (`OptionValue::for_look`), for the keys and the keyword case as well.
- The Run button still said "Run the statement at the cursor" with Whole tab set or text selected. It says "Run every statement" or "Run the selection".

It found no key spelled by hand, no way past Reset to defaults, and the old keys, CRLF files, values outside the menus, a whitespace-only selection, Format and paste over a selection, and a completion list across a change of Autocomplete all behaving as the spec says.

For a yes or a no, from the review:

- **Run all (⇧⌘↩, ctrl+shift+enter) with text selected runs the selection**, as Run does: "a selection always wins". Say if Run all should run the whole tab whatever is selected.
- **The gutter's bar** still marks the statement at the cursor when Whole tab is set or text is selected, though Run then runs more or less than it. Left for now; say if the bar should span every statement, or none while text is selected.

Left as they are, for the record:

- Written whole, such a file loses its comments, as General's rule has it since its step 1.
- In the terminal look a row's controls are described by the macOS small print (`describe`), as on every pane.

A second reviewer read this plan against its diffs and the spec. Its points are in: two doc comments moved back to their functions and three made true again, the Run all and run-key rows of the guides, the old timeout's clamp said and tested, the inline `appearance` case tested, the cursor line's test run in both looks and turned back on, values outside the menus tested in the sheet and on the screen, a test renamed for what it asserts, and the files and commands of each task corrected. The out-of-range numbers and the two tests' reach (completion's columns, the dollar quote) are in the table above.

## Files

| File | What it does after this plan |
|---|---|
| `src/settings/editor.rs` | New: `NewTab`, `RunScope`, `EditorKeys`, `Indent`, `Autocomplete`, `LineNumbers`, `KeywordCase`, `Commas`, and the timeout's names (`"30s"`, `"1m"`, `"off"`). |
| `src/settings.rs` | Thirteen `Settings` fields; fifteen `Key`s in the four `[editor.…]` tables and the two old ones read in their place; `write_into` taking an old key out with its new one; fifteen `OptionId`s and `OptionValue`s with their steps; the menus' values (`SQL_LIMITS`, `SQL_TIMEOUTS`, `HISTORY_KEEPS` and the `…_with` lists); `fetch_cap`. |
| `src/backend.rs` | `RunSql`'s limit: `u32::MAX` keeps every row. |
| `src/ui/sql_editor.rs` | The toolbar's Limit and Timeout menus: the setting's values, No limit, whole minutes in minutes (`limit_words`, `time_words`); the Run button's words say what it runs (`Runs`). |
| `src/model.rs` | `SettingsPane::Editor` and its cursor's rows; `SqlTab::selected`. |
| `src/ui/settings/{mod,sheet,terminal,file_pane}.rs` | The pane's labels (the run key from the keymap), small print, choices and rows in both looks; the file pane marks an old key's line for the row it sets. |
| `src/app.rs` | A look's own default picked while none is set stays unset (`SetOption` through `OptionValue::for_look`); new tabs' mode, every tab following the limit and timeout, the run's scope and selection (`selection_only`), when completion opens. |
| `src/ui/sql_text.rs` | The gutter's numbers as set, the cursor's line only where set, the selection reported. |
| `src/ui/editor_settings_tests.rs` | New: the headless tests of this step. |
| `src/ui/mod.rs`, `src/ui/menu.rs`, `src/ui/agent_settings_tests.rs`, `src/ui/settings_access_tests.rs` | Declare the tests; tests that met the old menus, the pane list, the new tab's mode and the segments' role. |
| `docs/_reference/settings-and-files.md`, `docs/_guide/sql-editor.md`, `docs/_guide/omarchy.md`, `docs/_guide/macos.md`, `docs/_reference/keyboard-shortcuts.md` | The pane, the keys, the old keys, the run keys and the new tab's mode. |

Every task ends with the repository's checks passing:

    cargo fmt --all --check
    cargo clippy --locked --workspace --all-targets -- -D warnings
    cargo test --locked --workspace --all-targets
    RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps

(`cargo` is `~/.cargo/bin/cargo` in these sessions.) Commit after each task, signed as usual. If signing says "agent refused operation", 1Password is locked: leave the change staged and say so.

---

### Task 1: The sheet's segments are radio buttons in a test of `main`

**Files:**
- Modify: `src/ui/mod.rs` (a test)

- [ ] **Step 1: See it fail on `main`**

Run: `cargo test --locked --lib -- ui::tests::the_sheet_shows_what_the_look_has_where_the_settings_do_not_say`
Expected: FAIL with "Numbers: 1,240.50 is no segment".

- [ ] **Step 2: Look for the segments as radio buttons**

```diff
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -2552,7 +2552,7 @@ value_tags   = true
             assert_eq!(harness.app.settings.group_digits, None);
             let tree = harness.settle();
             let selected = |name: &str| {
-                let id = crate::testing::node(&tree, name, Role::Button)
+                let id = crate::testing::node(&tree, name, Role::RadioButton)
                     .unwrap_or_else(|| panic!("{name} is no segment"));
                 let (_, node) = tree.nodes.iter().find(|(node, _)| *node == id).unwrap();
                 node.toggled()
```

- [ ] **Step 3: Run it to see it pass**

Run: `cargo test --locked --lib -- ui::tests::the_sheet_shows_what_the_look_has_where_the_settings_do_not_say`
Expected: PASS. Then the four checks above.

- [ ] **Step 4: Commit**

```bash
git add src/ui/mod.rs
git commit -m "Find the sheet's Numbers segments as the radio buttons they are"
```

---

### Task 2: The editor's keys in the file, and the toolbar's values

**Files:**
- Create: `src/settings/editor.rs`
- Modify: `src/settings.rs`, `src/backend.rs`, `src/ui/sql_editor.rs`, `src/app.rs`, `src/ui/menu.rs` (tests), `src/ui/mod.rs` (a test)

- [ ] **Step 1: Write the failing tests**

```diff
diff --git a/src/app.rs b/src/app.rs
--- a/src/app.rs
+++ b/src/app.rs
@@ -9881,7 +9881,7 @@ masked = ["orders.email"]
         // ignored.
         assert_eq!(
             app.settings_file.text,
-            "[data]\npage_size = 100\ngroup_digits = \"yes\"\n\n[editor]\nsql_limit = 100\n"
+            "[data]\npage_size = 100\ngroup_digits = \"yes\"\n\n[editor.run]\nlimit = 100  # 0 is no limit\n"
         );
         assert_eq!(app.settings_file.invalid, [3]);
         assert_eq!(
@@ -12764,16 +12764,17 @@ masked = ["orders.email"]
     }
 
     #[test]
-    fn a_limit_or_timeout_that_cannot_work_is_not_taken() {
+    fn no_limit_is_every_row_and_a_limit_or_timeout_out_of_range_is_brought_in() {
         let mut harness = Harness::new();
         let (tab, id) = new_sql(&mut harness);
+        // No limit is 0: every row.
         harness.app.apply(Action::SetSqlLimit {
             tab,
             sql_tab: id,
             limit: 0,
         });
-        assert_eq!(sql(&harness, tab, id).limit, 1);
-        assert_eq!(harness.app.settings.sql_limit, 1);
+        assert_eq!(sql(&harness, tab, id).limit, 0);
+        assert_eq!(harness.app.settings.sql_limit, 0);
         harness.app.apply(Action::SetSqlLimit {
             tab,
             sql_tab: id,
```

```diff
diff --git a/src/settings.rs b/src/settings.rs
--- a/src/settings.rs
+++ b/src/settings.rs
@@ -2578,8 +2578,21 @@ mod tests {
             interface_size: InterfaceSize::Large,
             font: Some("JetBrains Mono".into()),
             font_size: FontSize::of(14.5),
-            sql_limit: 100,
+            sql_limit: 0,
             sql_timeout_secs: None,
+            new_tab: NewTab::Connection,
+            run_scope: RunScope::All,
+            confirm_unsafe_writes: false,
+            editor_keys: Some(EditorKeys::Vim),
+            indent: Indent::Tab,
+            autocomplete: Autocomplete::Manual,
+            close_pairs: false,
+            line_numbers: Some(LineNumbers::Relative),
+            highlight_line: false,
+            wrap: true,
+            keywords: Some(KeywordCase::Keep),
+            commas: Commas::Leading,
+            history_keep: 0,
             mcp_enabled: true,
             mcp_log_days: 7,
             mcp_notify: false,
@@ -2623,9 +2636,28 @@ idle_disconnect = \"4h\"  # never | 5m | 15m | 30m | 1h | 4h
 [sidebar]
 show_system_schemas = true
 
-[editor]
-sql_limit = 100
-sql_timeout_secs = 0  # 0 waits forever
+[editor.run]
+new_tab = \"connection\"  # read-only | connection
+limit = 0  # 0 is no limit
+timeout = \"off\"  # 5s | 15s | 30s | 1m | 5m | off
+run = \"all\"  # statement | all
+confirm_unsafe_writes = false
+
+[editor.typing]
+keys = \"vim\"  # standard | vim
+indent = \"tab\"  # 2 | 4 | \"tab\"
+autocomplete = \"manual\"  # typing | manual | off
+close_pairs = false
+line_numbers = \"relative\"  # absolute | relative | off
+highlight_line = false
+wrap = true
+
+[editor.format]
+keywords = \"keep\"  # upper | lower | keep
+commas = \"leading\"  # trailing | leading
+
+[editor.history]
+keep = 0  # 0 keeps none
 
 [mcp]
 enabled = true
@@ -2665,7 +2697,7 @@ bar = \"label\"  # tint | label
             "{text}"
         );
         assert!(
-            text.contains("sql_timeout_secs = 0  # 0 waits forever\n"),
+            text.contains("timeout = \"off\"  # 5s | 15s | 30s | 1m | 5m | off\n"),
             "{text}"
         );
         // The only key of its table that is not at its default: the grid's
@@ -2718,20 +2750,33 @@ bar = \"label\"  # tint | label
                 (Key::ConfirmCloseEdits, 15),
                 (Key::IdleDisconnect, 16),
                 (Key::ShowSystemSchemas, 19),
-                (Key::SqlLimit, 22),
-                (Key::SqlTimeoutSecs, 23),
-                (Key::McpEnabled, 26),
-                (Key::McpLogDays, 27),
-                (Key::McpNotify, 28),
-                (Key::Theme, 31),
-                (Key::InterfaceSize, 32),
-                (Key::Font, 33),
-                (Key::FontSize, 34),
-                (Key::RowHeight, 37),
-                (Key::Zebra, 38),
-                (Key::ColumnTypes, 39),
-                (Key::TableList, 40),
-                (Key::EnvBar, 43),
+                (Key::NewTab, 22),
+                (Key::SqlLimit, 23),
+                (Key::SqlTimeoutSecs, 24),
+                (Key::RunScope, 25),
+                (Key::ConfirmUnsafeWrites, 26),
+                (Key::EditorKeys, 29),
+                (Key::Indent, 30),
+                (Key::Autocomplete, 31),
+                (Key::ClosePairs, 32),
+                (Key::LineNumbers, 33),
+                (Key::HighlightLine, 34),
+                (Key::Wrap, 35),
+                (Key::Keywords, 38),
+                (Key::Commas, 39),
+                (Key::HistoryKeep, 42),
+                (Key::McpEnabled, 45),
+                (Key::McpLogDays, 46),
+                (Key::McpNotify, 47),
+                (Key::Theme, 50),
+                (Key::InterfaceSize, 51),
+                (Key::Font, 52),
+                (Key::FontSize, 53),
+                (Key::RowHeight, 56),
+                (Key::Zebra, 57),
+                (Key::ColumnTypes, 58),
+                (Key::TableList, 59),
+                (Key::EnvBar, 62),
             ]
         );
     }
@@ -2803,12 +2848,87 @@ bar = \"label\"  # tint | label
         );
     }
 
+    #[test]
+    fn a_new_key_is_read_over_its_old_one_whatever_the_old_one_says() {
+        for old in ["sql_limit = \"x\"", "sql_limit = 1 00", "sql_limit = 300"] {
+            let text = format!("[editor]\n{old}\n\n[editor.run]\nlimit = 500\n");
+            assert_eq!(Settings::from_toml(&text).settings.sql_limit, 500, "{old}");
+            let last = Settings {
+                sql_limit: 5_000,
+                ..Settings::default()
+            };
+            let reloaded = Settings::from_toml_over(&text, &last).settings;
+            assert_eq!(reloaded.sql_limit, 500, "{old}");
+        }
+        let text = "[editor]\nsql_timeout_secs = \"x\"\n\n[editor.run]\ntimeout = \"5m\"\n";
+        assert_eq!(
+            Settings::from_toml(text).settings.sql_timeout_secs,
+            Some(300)
+        );
+        // A new key that cannot be read leaves the old one standing.
+        let text = "[editor]\nsql_limit = 300\n\n[editor.run]\nlimit = \"x\"\n";
+        let loaded = Settings::from_toml(text);
+        assert_eq!(loaded.settings.sql_limit, 300);
+        assert_eq!(loaded.invalid, [5]);
+        // The limit set over a rejected old line is read back at the next
+        // start.
+        let text = "[editor]\nsql_limit = 1 00\n";
+        let set = written(text, |settings| settings.sql_limit = 500);
+        assert_eq!(Settings::from_toml(&set).settings.sql_limit, 500, "{set}");
+    }
+
+    #[test]
+    fn a_new_table_under_one_written_inline_writes_the_file_whole() {
+        for text in [
+            "editor = { sql_timeout_secs = 10 }\n",
+            "editor.sql_timeout_secs = 10\n",
+        ] {
+            let set = written(text, |settings| settings.sql_limit = 500);
+            let loaded = Settings::from_toml(&set);
+            assert_eq!(loaded.invalid, Vec::<usize>::new(), "{set}");
+            assert_eq!(loaded.settings.sql_limit, 500, "{set}");
+            assert_eq!(loaded.settings.sql_timeout_secs, Some(10), "{set}");
+        }
+        // Appearance's grid under an inline `appearance` too.
+        let set = written(
+            "appearance = { interface_size = \"large\" }\n",
+            |settings| settings.row_height = RowHeight::Roomy,
+        );
+        let loaded = Settings::from_toml(&set);
+        assert_eq!(loaded.invalid, Vec::<usize>::new(), "{set}");
+        assert_eq!(loaded.settings.row_height, RowHeight::Roomy, "{set}");
+        assert_eq!(loaded.settings.interface_size, InterfaceSize::Large);
+    }
+
+    #[test]
+    fn a_timeout_over_an_hour_is_an_hour() {
+        let old = Settings::from_toml("[editor]\nsql_timeout_secs = 7200\n").settings;
+        assert_eq!(old.sql_timeout_secs, Some(3_600));
+        assert_eq!(
+            Settings::from_toml(&old.to_toml())
+                .settings
+                .sql_timeout_secs,
+            Some(3_600)
+        );
+        assert!(old.to_toml().contains("timeout = \"60m\""));
+        assert_eq!(Settings::valid_sql_timeout(Some(7_200)), Some(3_600));
+    }
+
+    #[test]
+    fn a_file_of_the_old_key_alone_becomes_the_new_table_alone() {
+        let text = "[editor]\nsql_limit = 300\n";
+        assert_eq!(
+            written(text, |settings| settings.sql_limit = 500),
+            "[editor.run]\nlimit = 500  # 0 is no limit\n"
+        );
+    }
+
     #[test]
     fn a_new_table_goes_at_the_end_of_the_file() {
         let text = "[data]\npage_size = 500\n# the end";
         assert_eq!(
             written(text, |settings| settings.sql_limit = 100),
-            "[data]\npage_size = 500\n# the end\n\n[editor]\nsql_limit = 100\n"
+            "[data]\npage_size = 500\n# the end\n\n[editor.run]\nlimit = 100  # 0 is no limit\n"
         );
     }
 
@@ -3287,12 +3407,13 @@ bar = \"label\"  # tint | label
         // The lines and the tables under the bad one are still read.
         assert_eq!(loaded.settings.group_digits, Some(true));
         assert_eq!(loaded.settings.sql_limit, 100);
+        // The old key of the limit, which still sets it.
         assert_eq!(
             loaded.lines,
             vec![
                 (Key::PageSize, 2),
                 (Key::GroupDigits, 4),
-                (Key::SqlLimit, 7)
+                (Key::OldSqlLimit, 7)
             ]
         );
         // What the pane will show is the file, not what was parsed.
@@ -3640,7 +3761,7 @@ bar = \"label\"  # tint | label
         assert!(
             Settings::SQL_LIMITS
                 .iter()
-                .all(|limit| (1..=Settings::MAX_SQL_LIMIT).contains(limit)),
+                .all(|limit| Settings::valid_sql_limit(*limit) == *limit),
             "every choice of the Limit menu loads as it was saved"
         );
     }
```

```diff
diff --git a/src/settings/editor.rs b/src/settings/editor.rs
new file mode 100644
--- /dev/null
+++ b/src/settings/editor.rs
@@ -0,0 +1,74 @@
+//! The SQL editor's settings.
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+
+    #[test]
+    fn every_choice_has_a_name_it_is_read_back_from() {
+        for choice in [NewTab::ReadOnly, NewTab::Connection] {
+            assert_eq!(NewTab::from_name(choice.name()), Some(choice));
+        }
+        for choice in [RunScope::Statement, RunScope::All] {
+            assert_eq!(RunScope::from_name(choice.name()), Some(choice));
+        }
+        for choice in [EditorKeys::Standard, EditorKeys::Vim] {
+            assert_eq!(EditorKeys::from_name(choice.name()), Some(choice));
+        }
+        for choice in Autocomplete::ALL {
+            assert_eq!(Autocomplete::from_name(choice.name()), Some(choice));
+        }
+        for choice in LineNumbers::TERMINAL {
+            assert_eq!(LineNumbers::from_name(choice.name()), Some(choice));
+        }
+        for choice in KeywordCase::ALL {
+            assert_eq!(KeywordCase::from_name(choice.name()), Some(choice));
+        }
+        for choice in [Commas::Trailing, Commas::Leading] {
+            assert_eq!(Commas::from_name(choice.name()), Some(choice));
+        }
+        assert_eq!(NewTab::from_name("readonly"), None);
+        assert_eq!(LineNumbers::from_name("on"), None);
+    }
+
+    #[test]
+    fn the_looks_defaults_are_as_their_boards_draw_them() {
+        assert_eq!(EditorKeys::of_look(false), EditorKeys::Standard);
+        assert_eq!(EditorKeys::of_look(true), EditorKeys::Vim);
+        assert_eq!(LineNumbers::of_look(false), LineNumbers::Absolute);
+        assert_eq!(LineNumbers::of_look(true), LineNumbers::Relative);
+        assert_eq!(KeywordCase::of_look(false), KeywordCase::Upper);
+        assert_eq!(KeywordCase::of_look(true), KeywordCase::Lower);
+    }
+
+    #[test]
+    fn a_timeout_is_seconds_or_minutes_up_to_an_hour_or_off() {
+        for (name, secs) in [
+            ("off", None),
+            ("5s", Some(5)),
+            ("30s", Some(30)),
+            ("90s", Some(90)),
+            ("1m", Some(60)),
+            ("5m", Some(300)),
+            ("60m", Some(3_600)),
+        ] {
+            assert_eq!(timeout_from_name(name), Some(secs), "{name}");
+        }
+        for wrong in [
+            "", "s", "0s", "61m", "3601s", "1h", "-5s", "+5s", "5 s", "5", "5é", "é", "30с",
+        ] {
+            assert_eq!(timeout_from_name(wrong), None, "{wrong}");
+        }
+        assert_eq!(timeout_name(None), "off");
+        assert_eq!(timeout_name(Some(30)), "30s");
+        assert_eq!(timeout_name(Some(60)), "1m");
+        assert_eq!(timeout_name(Some(90)), "90s");
+        assert_eq!(timeout_name(Some(300)), "5m");
+        for secs in [5, 15, 30, 60, 90, 300, 3_600] {
+            assert_eq!(
+                timeout_from_name(&timeout_name(Some(secs))),
+                Some(Some(secs))
+            );
+        }
+    }
+}
```

```diff
diff --git a/src/ui/menu.rs b/src/ui/menu.rs
--- a/src/ui/menu.rs
+++ b/src/ui/menu.rs
@@ -699,7 +699,7 @@ mod tests {
         for look in Look::ALL {
             let mut harness = limit_menu(look);
             let rows =
-                ["Limit 100", "Limit 1,000", "Limit 10,000"].map(|name| row(&mut harness, name));
+                ["Limit 100", "Limit 500", "Limit 1,000"].map(|name| row(&mut harness, name));
             for pair in rows.windows(2) {
                 assert!(
                     (pair[0].bottom() - pair[1].top()).abs() < 0.5,
@@ -743,7 +743,7 @@ mod tests {
             give_keyboard(&mut harness, "Limit 100");
             harness.press(Key::ArrowDown, Modifiers::NONE);
             let name = focused_name(&harness.settle());
-            assert_eq!(name, "Limit 1,000", "{}", look.name);
+            assert_eq!(name, "Limit 500", "{}", look.name);
             let lit = row(&mut harness, &name);
             assert_eq!(
                 fill(&harness, lit),
```

```diff
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -5886,7 +5886,7 @@ value_tags   = true
             harness.click("No timeout");
             assert_eq!(active_sql(&harness, tab).timeout, None, "{}", look.name);
             harness.click("Timeout");
-            harness.click("Timeout 60 s");
+            harness.click("Timeout 1 min");
             assert_eq!(
                 active_sql(&harness, tab).timeout,
                 Some(std::time::Duration::from_secs(60)),
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test --locked --lib -- settings`
Expected: FAIL to compile (35 errors): `Key` has no `OldSqlLimit`, `NewTab`, `Wrap`, `Keywords`, `Commas`, `HistoryKeep` and the others, `Settings` has no `new_tab` and the other fields, and `src/settings/editor.rs` holds its tests alone.

- [ ] **Step 3: Read and write the keys, and offer their values in the toolbar**

```diff
diff --git a/src/app.rs b/src/app.rs
--- a/src/app.rs
+++ b/src/app.rs
@@ -5833,7 +5833,8 @@ impl App {
         let Some(sql) = workspace.sql_tab_mut(id) else {
             return;
         };
-        let (limit, timeout) = (sql.limit, sql.timeout);
+        // No limit keeps as many rows as there are.
+        let (limit, timeout) = (Settings::fetch_cap(sql.limit), sql.timeout);
         let superseded = sql.start_run(request, statements.clone(), mode);
         // The session runs one thing at a time: the run still going must
         // stop before this one can start.
```

```diff
diff --git a/src/backend.rs b/src/backend.rs
--- a/src/backend.rs
+++ b/src/backend.rs
@@ -146,7 +146,8 @@ pub enum Command {
         session: SessionId,
         request: RequestId,
         statements: Vec<tabletist_db::sql::Statement>,
-        /// The most rows kept per statement.
+        /// The most rows kept per statement: `u32::MAX` keeps every one
+        /// (see `Settings::fetch_cap`).
         limit: u32,
         /// How long the script may run before it is stopped; `None` lets
         /// it run until it ends or the user cancels it.
```

```diff
diff --git a/src/settings.rs b/src/settings.rs
--- a/src/settings.rs
+++ b/src/settings.rs
@@ -8,6 +8,11 @@ use crate::keymap::Layout;
 use crate::paths::AppDirs;
 use crate::util::toml_string;
 
+mod editor;
+pub use editor::{
+    Autocomplete, Commas, EditorKeys, Indent, KeywordCase, LineNumbers, NewTab, RunScope,
+};
+
 /// How much of a timestamp the grid shows.
 #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
 #[serde(rename_all = "lowercase")]
@@ -619,10 +624,53 @@ pub struct Settings {
     /// ([`FontSizes::default`]).
     #[serde(skip)]
     pub font_size: Option<FontSize>,
-    /// Rows a SQL editor statement keeps (the Limit menu).
+    /// Rows a SQL editor statement keeps (the Limit menu); 0 keeps every
+    /// row.
     pub sql_limit: u32,
     /// Seconds before a SQL editor run is cancelled; `None` waits forever.
     pub sql_timeout_secs: Option<u32>,
+    /// The mode a new SQL tab opens in.
+    #[serde(skip)]
+    pub new_tab: NewTab,
+    /// What the run key runs without a selection.
+    #[serde(skip)]
+    pub run_scope: RunScope,
+    /// Whether an `UPDATE` or `DELETE` without `WHERE` asks first.
+    #[serde(skip)]
+    pub confirm_unsafe_writes: bool,
+    /// The editor's keys. `None` until the user sets them: the look's own
+    /// ([`Settings::editor_keys_in`]).
+    #[serde(skip)]
+    pub editor_keys: Option<EditorKeys>,
+    /// What an indent is, for the Tab key and Format.
+    #[serde(skip)]
+    pub indent: Indent,
+    /// When the completion list opens.
+    #[serde(skip)]
+    pub autocomplete: Autocomplete,
+    /// Whether a bracket or a quote typed gets its closing one.
+    #[serde(skip)]
+    pub close_pairs: bool,
+    /// How the gutter numbers the lines. `None` until the user sets it:
+    /// the look's own ([`Settings::line_numbers_in`]).
+    #[serde(skip)]
+    pub line_numbers: Option<LineNumbers>,
+    /// Whether the cursor's line is filled.
+    #[serde(skip)]
+    pub highlight_line: bool,
+    /// Whether a long line wraps.
+    #[serde(skip)]
+    pub wrap: bool,
+    /// The case Format writes keywords in. `None` until the user sets it:
+    /// the look's own ([`Settings::keywords_in`]).
+    #[serde(skip)]
+    pub keywords: Option<KeywordCase>,
+    /// Where Format puts commas.
+    #[serde(skip)]
+    pub commas: Commas,
+    /// Queries the history keeps per connection; 0 keeps none.
+    #[serde(skip)]
+    pub history_keep: u32,
     /// The MCP server agents use the saved connections through. Off until
     /// the person turns it on: nothing listens before.
     pub mcp_enabled: bool,
@@ -660,8 +708,27 @@ pub enum Key {
     ConfirmCloseEdits,
     IdleDisconnect,
     ShowSystemSchemas,
+    /// `editor.sql_limit`, which versions before the Editor tab wrote:
+    /// read where `editor.run.limit` is not, and taken out when that is
+    /// written.
+    OldSqlLimit,
+    /// `editor.sql_timeout_secs`, as `OldSqlLimit` is.
+    OldSqlTimeoutSecs,
     SqlLimit,
     SqlTimeoutSecs,
+    NewTab,
+    RunScope,
+    ConfirmUnsafeWrites,
+    EditorKeys,
+    Indent,
+    Autocomplete,
+    ClosePairs,
+    LineNumbers,
+    HighlightLine,
+    Wrap,
+    Keywords,
+    Commas,
+    HistoryKeep,
     McpEnabled,
     McpLogDays,
     McpNotify,
@@ -678,7 +745,9 @@ pub enum Key {
 
 impl Key {
     /// In the order the file has them: a table's keys together.
-    pub const ALL: [Key; 25] = [
+    /// An old key comes before the one that took its place: the new one
+    /// is read over it.
+    pub const ALL: [Key; 40] = [
         Self::OnLaunch,
         Self::RestoreTabs,
         Self::PageSize,
@@ -690,8 +759,23 @@ impl Key {
         Self::ConfirmCloseEdits,
         Self::IdleDisconnect,
         Self::ShowSystemSchemas,
+        Self::OldSqlLimit,
+        Self::OldSqlTimeoutSecs,
+        Self::NewTab,
         Self::SqlLimit,
         Self::SqlTimeoutSecs,
+        Self::RunScope,
+        Self::ConfirmUnsafeWrites,
+        Self::EditorKeys,
+        Self::Indent,
+        Self::Autocomplete,
+        Self::ClosePairs,
+        Self::LineNumbers,
+        Self::HighlightLine,
+        Self::Wrap,
+        Self::Keywords,
+        Self::Commas,
+        Self::HistoryKeep,
         Self::McpEnabled,
         Self::McpLogDays,
         Self::McpNotify,
@@ -720,8 +804,23 @@ impl Key {
             Self::ConfirmCloseEdits => ("safety", "confirm_close_edits"),
             Self::IdleDisconnect => ("safety", "idle_disconnect"),
             Self::ShowSystemSchemas => ("sidebar", "show_system_schemas"),
-            Self::SqlLimit => ("editor", "sql_limit"),
-            Self::SqlTimeoutSecs => ("editor", "sql_timeout_secs"),
+            Self::OldSqlLimit => ("editor", "sql_limit"),
+            Self::OldSqlTimeoutSecs => ("editor", "sql_timeout_secs"),
+            Self::NewTab => ("editor.run", "new_tab"),
+            Self::SqlLimit => ("editor.run", "limit"),
+            Self::SqlTimeoutSecs => ("editor.run", "timeout"),
+            Self::RunScope => ("editor.run", "run"),
+            Self::ConfirmUnsafeWrites => ("editor.run", "confirm_unsafe_writes"),
+            Self::EditorKeys => ("editor.typing", "keys"),
+            Self::Indent => ("editor.typing", "indent"),
+            Self::Autocomplete => ("editor.typing", "autocomplete"),
+            Self::ClosePairs => ("editor.typing", "close_pairs"),
+            Self::LineNumbers => ("editor.typing", "line_numbers"),
+            Self::HighlightLine => ("editor.typing", "highlight_line"),
+            Self::Wrap => ("editor.typing", "wrap"),
+            Self::Keywords => ("editor.format", "keywords"),
+            Self::Commas => ("editor.format", "commas"),
+            Self::HistoryKeep => ("editor.history", "keep"),
             Self::McpEnabled => ("mcp", "enabled"),
             Self::McpLogDays => ("mcp", "log_days"),
             Self::McpNotify => ("mcp", "notify"),
@@ -737,6 +836,15 @@ impl Key {
         }
     }
 
+    /// The key of older versions this one took the place of.
+    pub fn replaced(self) -> Option<Key> {
+        match self {
+            Self::SqlLimit => Some(Self::OldSqlLimit),
+            Self::SqlTimeoutSecs => Some(Self::OldSqlTimeoutSecs),
+            _ => None,
+        }
+    }
+
     /// What the file says beside the key: the values of a closed set, or
     /// what a value that reads oddly means. On launch's depends on its
     /// value: see [`Settings::note`].
@@ -746,7 +854,17 @@ impl Key {
             Self::Timestamps => Some("second | full"),
             Self::Tz => Some("system | utc | \"Area/City\""),
             Self::IdleDisconnect => Some("never | 5m | 15m | 30m | 1h | 4h"),
-            Self::SqlTimeoutSecs => Some("0 waits forever"),
+            Self::NewTab => Some("read-only | connection"),
+            Self::SqlLimit => Some("0 is no limit"),
+            Self::SqlTimeoutSecs => Some("5s | 15s | 30s | 1m | 5m | off"),
+            Self::RunScope => Some("statement | all"),
+            Self::EditorKeys => Some("standard | vim"),
+            Self::Indent => Some("2 | 4 | \"tab\""),
+            Self::Autocomplete => Some("typing | manual | off"),
+            Self::LineNumbers => Some("absolute | relative | off"),
+            Self::Keywords => Some("upper | lower | keep"),
+            Self::Commas => Some("trailing | leading"),
+            Self::HistoryKeep => Some("0 keeps none"),
             Self::RowHeight => Some("compact | regular | roomy"),
             Self::TableList => Some("tree | flat"),
             Self::EnvBar => Some("tint | label"),
@@ -1389,6 +1507,26 @@ fn item_at<'d, 'i>(
     Ok(Some(item))
 }
 
+/// Whether every table the dotted `path` is under that `document` has
+/// (`editor` for `editor.run`) has a header of its own, or is there only
+/// because a table under it is: a table with a header can follow them.
+fn headed_parents(document: &toml::de::DeTable<'_>, working: &str, path: &str) -> bool {
+    let parts: Vec<&str> = path.split('.').collect();
+    (1..parts.len()).all(|end| match item_at(document, &parts[..end].join(".")) {
+        Ok(Some(item)) => {
+            let headed = working[item.span()].starts_with('[');
+            let implicit = item.get_ref().as_table().is_some_and(|items| {
+                !items.is_empty()
+                    && items
+                        .iter()
+                        .all(|(_, item)| is_subtable(working, item.get_ref(), item.span()))
+            });
+            headed || implicit
+        }
+        Ok(None) | Err(NotATable) => true,
+    })
+}
+
 /// Whether an item of a table is a table with a header of its own under
 /// it (`[data.more]`, `[[data.list]]`), and not a value of it: an array
 /// is written with a bracket too.
@@ -1439,19 +1577,41 @@ fn own_lines(text: &str, range: std::ops::Range<usize>) -> bool {
         && (after.starts_with('#') || after.trim_matches(['\r', '\n']).is_empty())
 }
 
+/// `choices`, in the order of `rank`, with `current` among them.
+fn with_current<T: Copy + PartialEq>(
+    choices: &[T],
+    current: T,
+    rank: impl Fn(&T) -> u64,
+) -> Vec<T> {
+    let mut all = choices.to_vec();
+    if !all.contains(&current) {
+        all.push(current);
+        all.sort_by_key(|choice| rank(choice));
+    }
+    all
+}
+
 impl Settings {
     pub const DEFAULT_PAGE_SIZE: u32 = 1_000;
     pub const MIN_PAGE_SIZE: u32 = 10;
     pub const MAX_PAGE_SIZE: u32 = 10_000;
     /// The page sizes the Settings window offers.
     pub const PAGE_SIZES: [u32; 5] = [100, 500, 1_000, 5_000, 10_000];
-    /// The most rows a SQL editor statement keeps.
+    /// The most rows a SQL editor statement keeps, where it keeps fewer
+    /// than all.
     pub const MAX_SQL_LIMIT: u32 = 10_000;
-    /// The row limits the SQL editor's Limit menu offers.
-    pub const SQL_LIMITS: [u32; 3] = [100, 1_000, 10_000];
-    /// The timeouts (seconds) the SQL editor's Timeout menu offers; `None`
-    /// is no timeout.
-    pub const SQL_TIMEOUTS: [Option<u32>; 5] = [Some(10), Some(30), Some(60), Some(300), None];
+    /// The row limits the Editor pane and the SQL editor's Limit menu
+    /// offer; 0 keeps every row.
+    pub const SQL_LIMITS: [u32; 6] = [100, 500, 1_000, 5_000, 10_000, 0];
+    /// The timeouts (seconds) the Editor pane and the SQL editor's Timeout
+    /// menu offer; `None` is no timeout.
+    pub const SQL_TIMEOUTS: [Option<u32>; 6] =
+        [Some(5), Some(15), Some(30), Some(60), Some(300), None];
+    /// The queries the history keeps that the Editor pane offers; 0 keeps
+    /// none.
+    pub const HISTORY_KEEPS: [u32; 4] = [100, 1_000, 10_000, 0];
+    /// The most queries the history keeps per connection.
+    pub const MAX_HISTORY: u32 = 100_000;
     /// Days the log of agents' calls keeps a call, at least and at most: a
     /// day, and ten years.
     pub const MIN_MCP_LOG_DAYS: u32 = 1;
@@ -1494,15 +1654,73 @@ impl Settings {
         secs.map(|secs| std::time::Duration::from_secs(u64::from(secs)))
     }
 
-    /// A row limit the SQL editor can run with.
+    /// A row limit the SQL editor can run with: 0, every row, or up to
+    /// [`Settings::MAX_SQL_LIMIT`].
     pub fn valid_sql_limit(limit: u32) -> u32 {
-        limit.clamp(1, Self::MAX_SQL_LIMIT)
+        limit.min(Self::MAX_SQL_LIMIT)
     }
 
-    /// A timeout the SQL editor can run with. One of no seconds would
-    /// cancel every run at once, so it counts as no timeout.
+    /// The rows a run of `limit` asks the backend for: as many as there
+    /// are for no limit.
+    pub fn fetch_cap(limit: u32) -> u32 {
+        if limit == 0 { u32::MAX } else { limit }
+    }
+
+    /// A timeout the SQL editor can run with, up to an hour. One of no
+    /// seconds would cancel every run at once, so it counts as no timeout.
     pub fn valid_sql_timeout(secs: Option<u32>) -> Option<u32> {
         secs.filter(|secs| *secs > 0)
+            .map(|secs| secs.min(editor::MAX_TIMEOUT_SECS))
+    }
+
+    /// The row limits a menu offers: the list, with `current` in its
+    /// place when the file set one that is not in it.
+    pub fn sql_limits_with(current: u32) -> Vec<u32> {
+        let rank = |limit: &u32| {
+            if *limit == 0 {
+                u64::MAX
+            } else {
+                u64::from(*limit)
+            }
+        };
+        with_current(&Self::SQL_LIMITS, current, rank)
+    }
+
+    /// The timeouts a menu offers, as [`Settings::sql_limits_with`].
+    pub fn sql_timeouts_with(current: Option<u32>) -> Vec<Option<u32>> {
+        let rank = |secs: &Option<u32>| secs.map_or(u64::MAX, u64::from);
+        with_current(&Self::SQL_TIMEOUTS, current, rank)
+    }
+
+    /// The history's sizes a menu offers, as [`Settings::sql_limits_with`].
+    pub fn history_keeps_with(current: u32) -> Vec<u32> {
+        let rank = |keep: &u32| {
+            if *keep == 0 {
+                u64::MAX
+            } else {
+                u64::from(*keep)
+            }
+        };
+        with_current(&Self::HISTORY_KEEPS, current, rank)
+    }
+
+    /// The editor's keys in a look that is the terminal's (`terminal`) or
+    /// not: as set, else the look's own.
+    pub fn editor_keys_in(&self, terminal: bool) -> EditorKeys {
+        self.editor_keys
+            .unwrap_or_else(|| EditorKeys::of_look(terminal))
+    }
+
+    /// The gutter's numbers in a look, as [`Settings::editor_keys_in`].
+    pub fn line_numbers_in(&self, terminal: bool) -> LineNumbers {
+        self.line_numbers
+            .unwrap_or_else(|| LineNumbers::of_look(terminal))
+    }
+
+    /// Format's keyword case in a look, as [`Settings::editor_keys_in`].
+    pub fn keywords_in(&self, terminal: bool) -> KeywordCase {
+        self.keywords
+            .unwrap_or_else(|| KeywordCase::of_look(terminal))
     }
 
     /// What `option` is set to.
@@ -1806,9 +2024,23 @@ impl Settings {
             Key::ConfirmCloseEdits => self.confirm_close_edits.to_string(),
             Key::IdleDisconnect => toml_string(self.idle_disconnect.name()),
             Key::ShowSystemSchemas => self.show_system_schemas.to_string(),
+            // Read, never written: the new key is.
+            Key::OldSqlLimit | Key::OldSqlTimeoutSecs => return None,
             Key::SqlLimit => self.sql_limit.to_string(),
-            // TOML has no null: no timeout is no seconds.
-            Key::SqlTimeoutSecs => self.sql_timeout_secs.unwrap_or(0).to_string(),
+            Key::SqlTimeoutSecs => toml_string(&editor::timeout_name(self.sql_timeout_secs)),
+            Key::NewTab => toml_string(self.new_tab.name()),
+            Key::RunScope => toml_string(self.run_scope.name()),
+            Key::ConfirmUnsafeWrites => self.confirm_unsafe_writes.to_string(),
+            Key::EditorKeys => toml_string(self.editor_keys?.name()),
+            Key::Indent => self.indent.written().to_owned(),
+            Key::Autocomplete => toml_string(self.autocomplete.name()),
+            Key::ClosePairs => self.close_pairs.to_string(),
+            Key::LineNumbers => toml_string(self.line_numbers?.name()),
+            Key::HighlightLine => self.highlight_line.to_string(),
+            Key::Wrap => self.wrap.to_string(),
+            Key::Keywords => toml_string(self.keywords?.name()),
+            Key::Commas => toml_string(self.commas.name()),
+            Key::HistoryKeep => self.history_keep.to_string(),
             Key::McpEnabled => self.mcp_enabled.to_string(),
             Key::McpLogDays => self.mcp_log_days.to_string(),
             Key::McpNotify => self.mcp_notify.to_string(),
@@ -1872,6 +2104,18 @@ impl Settings {
             text
         };
         let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
+        // A key written takes the old key it replaced out with it: the old
+        // one comes first, as the reader reads it.
+        let mut keys: Vec<Key> = keys
+            .iter()
+            .flat_map(|key| key.replaced().into_iter().chain([*key]))
+            .collect();
+        let mut seen = Vec::new();
+        keys.retain(|key| {
+            let new = !seen.contains(key);
+            seen.push(*key);
+            new
+        });
         let (working, rejected) = readable(text);
         // The keys the rejected lines would set: one of them taken out of
         // the file would leave the rejected line to be read in its place.
@@ -1893,13 +2137,19 @@ impl Settings {
                 None => format!("{name} = {literal}{newline}"),
             }
         };
-        for &key in keys {
+        for &key in &keys {
             let (table_name, name) = key.path();
             let literal = self.literal(key);
             let table = match item_at(&document, table_name) {
                 Ok(Some(table)) => table,
                 Ok(None) => {
                     if let Some(literal) = literal {
+                        // Under a table written inline (`editor = { … }`)
+                        // or with dotted keys, a header of its own would
+                        // define it again: the file is written whole.
+                        if !headed_parents(&document, &working, table_name) {
+                            return self.to_toml();
+                        }
                         new_tables.push((table_name, line(key, &literal)));
                     }
                     continue;
@@ -2003,10 +2253,12 @@ impl Settings {
         let mut open: Option<&str> = None;
         for (table, added) in new_tables {
             if open != Some(table) {
-                if !out.ends_with('\n') {
+                // A text left empty (its one table taken out) starts with
+                // the table.
+                if !out.is_empty() && !out.ends_with('\n') {
                     out.push_str(newline);
                 }
-                if out != HEADER {
+                if !out.is_empty() && out != HEADER {
                     out.push_str(newline);
                 }
                 out.push_str(&format!("[{table}]{newline}"));
@@ -2024,6 +2276,7 @@ impl Settings {
             .clamp(Self::MIN_PAGE_SIZE, Self::MAX_PAGE_SIZE);
         self.sql_limit = Self::valid_sql_limit(self.sql_limit);
         self.sql_timeout_secs = Self::valid_sql_timeout(self.sql_timeout_secs);
+        self.history_keep = self.history_keep.min(Self::MAX_HISTORY);
         self.mcp_log_days = self
             .mcp_log_days
             .clamp(Self::MIN_MCP_LOG_DAYS, Self::MAX_MCP_LOG_DAYS);
@@ -2069,8 +2322,46 @@ impl Settings {
                 .and_then(Idle::from_name)
                 .map(|choice| self.idle_disconnect = choice),
             Key::ShowSystemSchemas => flag.map(|flag| self.show_system_schemas = flag),
+            // The old key kept at least a row, and 0 seconds waited forever.
+            Key::OldSqlLimit => number.map(|number| self.sql_limit = number.max(1)),
+            Key::OldSqlTimeoutSecs => number.map(|number| self.sql_timeout_secs = Some(number)),
             Key::SqlLimit => number.map(|number| self.sql_limit = number),
-            Key::SqlTimeoutSecs => number.map(|number| self.sql_timeout_secs = Some(number)),
+            Key::SqlTimeoutSecs => text
+                .and_then(editor::timeout_from_name)
+                .map(|secs| self.sql_timeout_secs = secs),
+            Key::NewTab => text
+                .and_then(NewTab::from_name)
+                .map(|choice| self.new_tab = choice),
+            Key::RunScope => text
+                .and_then(RunScope::from_name)
+                .map(|scope| self.run_scope = scope),
+            Key::ConfirmUnsafeWrites => flag.map(|flag| self.confirm_unsafe_writes = flag),
+            Key::EditorKeys => text
+                .and_then(EditorKeys::from_name)
+                .map(|keys| self.editor_keys = Some(keys)),
+            Key::Indent => match (number, text) {
+                (Some(2), _) => Some(Indent::Two),
+                (Some(4), _) => Some(Indent::Four),
+                (_, Some("tab")) => Some(Indent::Tab),
+                _ => None,
+            }
+            .map(|indent| self.indent = indent),
+            Key::Autocomplete => text
+                .and_then(Autocomplete::from_name)
+                .map(|choice| self.autocomplete = choice),
+            Key::ClosePairs => flag.map(|flag| self.close_pairs = flag),
+            Key::LineNumbers => text
+                .and_then(LineNumbers::from_name)
+                .map(|numbers| self.line_numbers = Some(numbers)),
+            Key::HighlightLine => flag.map(|flag| self.highlight_line = flag),
+            Key::Wrap => flag.map(|flag| self.wrap = flag),
+            Key::Keywords => text
+                .and_then(KeywordCase::from_name)
+                .map(|case| self.keywords = Some(case)),
+            Key::Commas => text
+                .and_then(Commas::from_name)
+                .map(|commas| self.commas = commas),
+            Key::HistoryKeep => number.map(|number| self.history_keep = number),
             Key::McpEnabled => flag.map(|flag| self.mcp_enabled = flag),
             Key::McpLogDays => number.map(|number| self.mcp_log_days = number),
             Key::McpNotify => flag.map(|flag| self.mcp_notify = flag),
@@ -2188,10 +2479,19 @@ impl Settings {
                     }
                 }
                 lines.push((key, line));
-                kept.retain(|other| *other != key);
+                // A key read is read over the old key it took the place
+                // of, whether that was read, ignored or rejected.
+                kept.retain(|other| *other != key && Some(*other) != key.replaced());
             } else {
                 invalid.push(line);
-                kept.push(key);
+                // A new key that cannot be read is as good as absent: the
+                // old key it took the place of, read, stands.
+                let old_read = key
+                    .replaced()
+                    .is_some_and(|old| lines.iter().any(|(read, _)| *read == old));
+                if !old_read {
+                    kept.push(key);
+                }
             }
         }
         for key in kept {
@@ -2222,8 +2522,23 @@ impl Settings {
             Key::ConfirmCloseEdits => self.confirm_close_edits = other.confirm_close_edits,
             Key::IdleDisconnect => self.idle_disconnect = other.idle_disconnect,
             Key::ShowSystemSchemas => self.show_system_schemas = other.show_system_schemas,
-            Key::SqlLimit => self.sql_limit = other.sql_limit,
-            Key::SqlTimeoutSecs => self.sql_timeout_secs = other.sql_timeout_secs,
+            Key::OldSqlLimit | Key::SqlLimit => self.sql_limit = other.sql_limit,
+            Key::OldSqlTimeoutSecs | Key::SqlTimeoutSecs => {
+                self.sql_timeout_secs = other.sql_timeout_secs
+            }
+            Key::NewTab => self.new_tab = other.new_tab,
+            Key::RunScope => self.run_scope = other.run_scope,
+            Key::ConfirmUnsafeWrites => self.confirm_unsafe_writes = other.confirm_unsafe_writes,
+            Key::EditorKeys => self.editor_keys = other.editor_keys,
+            Key::Indent => self.indent = other.indent,
+            Key::Autocomplete => self.autocomplete = other.autocomplete,
+            Key::ClosePairs => self.close_pairs = other.close_pairs,
+            Key::LineNumbers => self.line_numbers = other.line_numbers,
+            Key::HighlightLine => self.highlight_line = other.highlight_line,
+            Key::Wrap => self.wrap = other.wrap,
+            Key::Keywords => self.keywords = other.keywords,
+            Key::Commas => self.commas = other.commas,
+            Key::HistoryKeep => self.history_keep = other.history_keep,
             Key::McpEnabled => self.mcp_enabled = other.mcp_enabled,
             Key::McpLogDays => self.mcp_log_days = other.mcp_log_days,
             Key::McpNotify => self.mcp_notify = other.mcp_notify,
@@ -2298,6 +2613,19 @@ impl Default for Settings {
             font_size: None,
             sql_limit: 1_000,
             sql_timeout_secs: Some(30),
+            new_tab: NewTab::ReadOnly,
+            run_scope: RunScope::Statement,
+            confirm_unsafe_writes: true,
+            editor_keys: None,
+            indent: Indent::Two,
+            autocomplete: Autocomplete::Typing,
+            close_pairs: true,
+            line_numbers: None,
+            highlight_line: true,
+            wrap: false,
+            keywords: None,
+            commas: Commas::Trailing,
+            history_keep: 1_000,
             mcp_enabled: false,
             mcp_log_days: 30,
             mcp_notify: true,
```

```diff
diff --git a/src/settings/editor.rs b/src/settings/editor.rs
--- a/src/settings/editor.rs
+++ b/src/settings/editor.rs
@@ -1,4 +1,286 @@
-//! The SQL editor's settings.
+//! The SQL editor's settings: the `[editor.run]`, `[editor.typing]`,
+//! `[editor.format]` and `[editor.history]` tables of `settings.toml`.
+
+/// The mode a new SQL tab opens in: read-only whatever the connection, or
+/// the connection's own.
+#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
+pub enum NewTab {
+    #[default]
+    ReadOnly,
+    Connection,
+}
+
+impl NewTab {
+    /// As the file says it.
+    pub fn name(self) -> &'static str {
+        match self {
+            Self::ReadOnly => "read-only",
+            Self::Connection => "connection",
+        }
+    }
+
+    pub fn from_name(name: &str) -> Option<Self> {
+        [Self::ReadOnly, Self::Connection]
+            .into_iter()
+            .find(|choice| choice.name() == name)
+    }
+}
+
+/// What the run key runs without a selection: the statement at the
+/// cursor, or every statement of the tab.
+#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
+pub enum RunScope {
+    #[default]
+    Statement,
+    All,
+}
+
+impl RunScope {
+    /// As the file says it.
+    pub fn name(self) -> &'static str {
+        match self {
+            Self::Statement => "statement",
+            Self::All => "all",
+        }
+    }
+
+    pub fn from_name(name: &str) -> Option<Self> {
+        [Self::Statement, Self::All]
+            .into_iter()
+            .find(|choice| choice.name() == name)
+    }
+}
+
+/// The editor's keys: the platform's text editing keys, or vim's modes.
+#[derive(Clone, Copy, Debug, PartialEq, Eq)]
+pub enum EditorKeys {
+    Standard,
+    Vim,
+}
+
+impl EditorKeys {
+    /// As the file says it.
+    pub fn name(self) -> &'static str {
+        match self {
+            Self::Standard => "standard",
+            Self::Vim => "vim",
+        }
+    }
+
+    pub fn from_name(name: &str) -> Option<Self> {
+        [Self::Standard, Self::Vim]
+            .into_iter()
+            .find(|choice| choice.name() == name)
+    }
+
+    /// The look's own, as its board draws it: vim in the terminal look.
+    pub fn of_look(terminal: bool) -> Self {
+        if terminal { Self::Vim } else { Self::Standard }
+    }
+}
+
+/// What an indent is: two spaces, four, or a tab.
+#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
+pub enum Indent {
+    #[default]
+    Two,
+    Four,
+    Tab,
+}
+
+impl Indent {
+    /// In the order the window draws them.
+    pub const ALL: [Indent; 3] = [Self::Two, Self::Four, Self::Tab];
+
+    /// As TOML writes it: a number of spaces, or `"tab"`.
+    pub fn written(self) -> &'static str {
+        match self {
+            Self::Two => "2",
+            Self::Four => "4",
+            Self::Tab => "\"tab\"",
+        }
+    }
+}
+
+/// When the completion list opens: as you type, on the Complete key
+/// alone, or never.
+#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
+pub enum Autocomplete {
+    #[default]
+    Typing,
+    Manual,
+    Off,
+}
+
+impl Autocomplete {
+    /// In the order the window draws them.
+    pub const ALL: [Autocomplete; 3] = [Self::Typing, Self::Manual, Self::Off];
+
+    /// As the file says it.
+    pub fn name(self) -> &'static str {
+        match self {
+            Self::Typing => "typing",
+            Self::Manual => "manual",
+            Self::Off => "off",
+        }
+    }
+
+    pub fn from_name(name: &str) -> Option<Self> {
+        Self::ALL.into_iter().find(|choice| choice.name() == name)
+    }
+
+    /// Whether typing opens a list.
+    pub fn as_you_type(self) -> bool {
+        match self {
+            Self::Typing => true,
+            Self::Manual | Self::Off => false,
+        }
+    }
+
+    /// Whether the Complete key opens a list.
+    pub fn on_key(self) -> bool {
+        match self {
+            Self::Typing | Self::Manual => true,
+            Self::Off => false,
+        }
+    }
+}
+
+/// How the gutter numbers the lines: each its own number, how far it is
+/// from the cursor's line, or not at all.
+#[derive(Clone, Copy, Debug, PartialEq, Eq)]
+pub enum LineNumbers {
+    Absolute,
+    Relative,
+    Off,
+}
+
+impl LineNumbers {
+    /// In the order the terminal look draws them.
+    pub const TERMINAL: [LineNumbers; 3] = [Self::Relative, Self::Absolute, Self::Off];
+
+    /// As the file says it.
+    pub fn name(self) -> &'static str {
+        match self {
+            Self::Absolute => "absolute",
+            Self::Relative => "relative",
+            Self::Off => "off",
+        }
+    }
+
+    pub fn from_name(name: &str) -> Option<Self> {
+        Self::TERMINAL
+            .into_iter()
+            .find(|choice| choice.name() == name)
+    }
+
+    /// The look's own, as its board draws it: relative in the terminal
+    /// look.
+    pub fn of_look(terminal: bool) -> Self {
+        if terminal {
+            Self::Relative
+        } else {
+            Self::Absolute
+        }
+    }
+
+    /// Whether the gutter shows numbers.
+    pub fn shown(self) -> bool {
+        match self {
+            Self::Absolute | Self::Relative => true,
+            Self::Off => false,
+        }
+    }
+}
+
+/// The case Format writes keywords in: upper, lower, or as typed.
+#[derive(Clone, Copy, Debug, PartialEq, Eq)]
+pub enum KeywordCase {
+    Upper,
+    Lower,
+    Keep,
+}
+
+impl KeywordCase {
+    /// In the order the window draws them.
+    pub const ALL: [KeywordCase; 3] = [Self::Upper, Self::Lower, Self::Keep];
+
+    /// As the file says it.
+    pub fn name(self) -> &'static str {
+        match self {
+            Self::Upper => "upper",
+            Self::Lower => "lower",
+            Self::Keep => "keep",
+        }
+    }
+
+    pub fn from_name(name: &str) -> Option<Self> {
+        Self::ALL.into_iter().find(|choice| choice.name() == name)
+    }
+
+    /// The look's own, as its board draws it: lower case in the terminal
+    /// look.
+    pub fn of_look(terminal: bool) -> Self {
+        if terminal { Self::Lower } else { Self::Upper }
+    }
+}
+
+/// Where Format puts the comma between two items of a list: at the end of
+/// the first one's line, or at the start of the next one's.
+#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
+pub enum Commas {
+    #[default]
+    Trailing,
+    Leading,
+}
+
+impl Commas {
+    /// As the file says it.
+    pub fn name(self) -> &'static str {
+        match self {
+            Self::Trailing => "trailing",
+            Self::Leading => "leading",
+        }
+    }
+
+    pub fn from_name(name: &str) -> Option<Self> {
+        [Self::Trailing, Self::Leading]
+            .into_iter()
+            .find(|choice| choice.name() == name)
+    }
+}
+
+/// The most seconds a timeout can be: an hour.
+pub const MAX_TIMEOUT_SECS: u32 = 60 * 60;
+
+/// A timeout as the file says it: `"30s"`, `"1m"`, or `"off"` for none. A
+/// whole number of minutes is said in minutes.
+pub fn timeout_name(secs: Option<u32>) -> String {
+    match secs {
+        None => "off".to_owned(),
+        Some(secs) if secs >= 60 && secs.is_multiple_of(60) => format!("{}m", secs / 60),
+        Some(secs) => format!("{secs}s"),
+    }
+}
+
+/// The timeout a value of the file names: `"<n>s"` or `"<n>m"`, from a
+/// second to an hour, or `"off"`. Anything else names none: `None` then,
+/// and `Some(None)` for off.
+pub fn timeout_from_name(name: &str) -> Option<Option<u32>> {
+    if name == "off" {
+        return Some(None);
+    }
+    let (number, scale) = match (name.strip_suffix('s'), name.strip_suffix('m')) {
+        (Some(number), _) => (number, 1),
+        (None, Some(number)) => (number, 60),
+        (None, None) => return None,
+    };
+    if number.is_empty() || !number.bytes().all(|byte| byte.is_ascii_digit()) {
+        return None;
+    }
+    let secs = number.parse::<u32>().ok()?.checked_mul(scale)?;
+    (1..=MAX_TIMEOUT_SECS).contains(&secs).then_some(Some(secs))
+}
 
 #[cfg(test)]
 mod tests {
```

```diff
diff --git a/src/ui/sql_editor.rs b/src/ui/sql_editor.rs
--- a/src/ui/sql_editor.rs
+++ b/src/ui/sql_editor.rs
@@ -179,6 +179,9 @@ fn splitter(
 
 /// "Limit 1,000": a limit's name, the same in every look.
 fn limit_name(limit: u32, locale: Locale) -> String {
+    if limit == 0 {
+        return gettext(locale, "No limit").into_owned();
+    }
     format!(
         "{} {}",
         gettext(locale, "Limit"),
@@ -186,27 +189,50 @@ fn limit_name(limit: u32, locale: Locale) -> String {
     )
 }
 
+/// A limit's rows as the look writes them: "1,000", the terminal's
+/// "1000", or "No limit" for 0.
+pub(crate) fn limit_words(limit: u32, look: &Look, locale: Locale) -> String {
+    match limit {
+        0 => look.label(&gettext(locale, "No limit")),
+        _ if look.terminal => limit.to_string(),
+        _ => format::group_digits(u64::from(limit)),
+    }
+}
+
 /// A limit as the look reads it: "Limit 1,000", or the terminal's "limit
-/// 1000". Where room is `short`, the number alone.
+/// 1000", or "No limit". Where room is `short`, the number alone.
 fn limit_text(limit: u32, short: bool, look: &Look, locale: Locale) -> String {
-    let number = if look.terminal {
-        limit.to_string()
-    } else {
-        format::group_digits(u64::from(limit))
-    };
-    if short {
+    let number = limit_words(limit, look, locale);
+    if short || limit == 0 {
         number
     } else {
         format!("{} {number}", look.label(&gettext(locale, "Limit")))
     }
 }
 
+/// A time as macOS and Windows write it, "30 s" and "1 min", or as the
+/// terminal look does (`terminal`), "30s" and "1m". Whole minutes are said
+/// in minutes.
+pub(crate) fn time_words(secs: u32, terminal: bool, locale: Locale) -> String {
+    let minutes = secs >= 60 && secs.is_multiple_of(60);
+    match (terminal, minutes) {
+        (true, true) => format!("{}m", secs / 60),
+        (true, false) => format!("{secs}s"),
+        (false, true) => format!("{} {}", secs / 60, gettext(locale, "min")),
+        (false, false) => format!("{secs} s"),
+    }
+}
+
 /// "Timeout 30 s", or "No timeout": a timeout's name, the same in every
 /// look.
 fn timeout_name(secs: Option<u32>, locale: Locale) -> String {
     match secs {
         None => gettext(locale, "No timeout").into_owned(),
-        Some(secs) => format!("{} {secs} s", gettext(locale, "Timeout")),
+        Some(secs) => format!(
+            "{} {}",
+            gettext(locale, "Timeout"),
+            time_words(secs, false, locale)
+        ),
     }
 }
 
@@ -218,11 +244,7 @@ fn timeout_text(secs: Option<u32>, short: bool, look: &Look, locale: Locale) ->
         let none = if short { "Off" } else { "No timeout" };
         return look.label(&gettext(locale, none));
     };
-    let time = if look.terminal {
-        format!("{secs}s")
-    } else {
-        format!("{secs} s")
-    };
+    let time = time_words(secs, look.terminal, locale);
     if short {
         time
     } else {
@@ -380,6 +402,9 @@ fn menus(
         palette,
         ..
     } = *bar;
+    // The settings' choices, and the tab's own in its place where the file
+    // set another.
+    let limits = Settings::sql_limits_with(bar.limit);
     let picked = menu(
         ui,
         rects[0],
@@ -387,7 +412,7 @@ fn menus(
         shape,
         (look, palette),
         || {
-            Settings::SQL_LIMITS
+            limits
                 .iter()
                 .map(|choice| menu::Choice {
                     text: limit_text(*choice, false, look, locale),
@@ -398,7 +423,7 @@ fn menus(
                 .collect()
         },
     );
-    if let Some(limit) = picked.and_then(|index| Settings::SQL_LIMITS.get(index))
+    if let Some(limit) = picked.and_then(|index| limits.get(index))
         && *limit != bar.limit
     {
         actions.push(Action::SetSqlLimit {
@@ -407,6 +432,7 @@ fn menus(
             limit: *limit,
         });
     }
+    let timeouts = Settings::sql_timeouts_with(bar.secs);
     let picked = menu(
         ui,
         rects[1],
@@ -414,7 +440,7 @@ fn menus(
         shape,
         (look, palette),
         || {
-            Settings::SQL_TIMEOUTS
+            timeouts
                 .iter()
                 .map(|choice| menu::Choice {
                     text: timeout_text(*choice, false, look, locale),
@@ -425,7 +451,7 @@ fn menus(
                 .collect()
         },
     );
-    if let Some(secs) = picked.and_then(|index| Settings::SQL_TIMEOUTS.get(index))
+    if let Some(secs) = picked.and_then(|index| timeouts.get(index))
         && *secs != bar.secs
     {
         actions.push(Action::SetSqlTimeout {
```

- [ ] **Step 4: Run the tests to see them pass**

Run: `cargo test --locked --lib -- settings ui::menu ui::sql_editor app::tests ui::tests`
Expected: PASS. Then the four checks above.

- [ ] **Step 5: Commit**

```bash
git add src/app.rs src/backend.rs src/settings.rs src/settings/editor.rs src/ui/menu.rs src/ui/mod.rs src/ui/sql_editor.rs
git commit -m "Read and write the SQL editor's settings in four [editor] tables, the old keys too, and offer their values in the toolbar"
```

---

### Task 3: The Editor pane in both looks

**Files:**
- Modify: `src/settings.rs`, `src/model.rs`, `src/app.rs`, `src/ui/settings/mod.rs`, `src/ui/settings/sheet.rs`, `src/ui/settings/terminal.rs`, `src/ui/settings/file_pane.rs`
- Create: `src/ui/editor_settings_tests.rs`
- Modify (tests): `src/ui/mod.rs`, `src/ui/agent_settings_tests.rs`, `src/ui/settings_access_tests.rs`

- [ ] **Step 1: Write the failing tests**

```diff
diff --git a/src/settings.rs b/src/settings.rs
--- a/src/settings.rs
+++ b/src/settings.rs
@@ -4210,6 +4210,21 @@ bar = \"label\"  # tint | label
                 Key::ColumnTypes,
                 Key::TableList,
                 Key::EnvBar,
+                Key::NewTab,
+                Key::SqlLimit,
+                Key::SqlTimeoutSecs,
+                Key::RunScope,
+                Key::ConfirmUnsafeWrites,
+                Key::EditorKeys,
+                Key::Indent,
+                Key::Autocomplete,
+                Key::ClosePairs,
+                Key::LineNumbers,
+                Key::HighlightLine,
+                Key::Wrap,
+                Key::Keywords,
+                Key::Commas,
+                Key::HistoryKeep,
                 Key::McpEnabled,
                 Key::McpLogDays,
                 Key::McpNotify,
@@ -4219,6 +4234,7 @@ bar = \"label\"  # tint | label
         let panes = [
             &OptionId::GENERAL[..],
             &OptionId::APPEARANCE[..],
+            &OptionId::EDITOR[..],
             &OptionId::AGENTS[..],
         ]
         .concat();
@@ -4247,6 +4263,28 @@ bar = \"label\"  # tint | label
             OptionId::TERMINAL_TEXT,
             [OptionId::Font, OptionId::FontSize]
         );
+        // The Editor pane's: every option is in a section of each look, or
+        // waits for its feature; the terminal look has no row for the
+        // cursor's line.
+        for (sections, without) in [
+            (&OptionId::TYPING[..], None),
+            (
+                &OptionId::TERMINAL_TYPING[..],
+                Some(OptionId::HighlightLine),
+            ),
+        ] {
+            let mut shown = [
+                &OptionId::RUNNING[..],
+                sections,
+                &OptionId::EDITOR_LATER[..],
+            ]
+            .concat()
+            .into_iter()
+            .chain(without)
+            .collect::<Vec<_>>();
+            shown.sort_by_key(|option| OptionId::EDITOR.iter().position(|all| all == option));
+            assert_eq!(shown, OptionId::EDITOR);
+        }
         assert_eq!(panes, OptionId::ALL);
         let defaults = Settings::default();
         for option in OptionId::ALL {
```

```diff
diff --git a/src/ui/agent_settings_tests.rs b/src/ui/agent_settings_tests.rs
--- a/src/ui/agent_settings_tests.rs
+++ b/src/ui/agent_settings_tests.rs
@@ -67,10 +67,11 @@ fn settings_lists_an_agents_pane_after_appearance() {
         harness.settle();
         assert_eq!(pane(&harness), Some(SettingsPane::General), "{}", look.name);
         assert!(!harness.has("Let agents use Tabletist"), "{}", look.name);
-        // Past Appearance, the pane between them.
+        // Past Appearance and Editor, the panes between them.
         if look.terminal {
             harness.press(egui::Key::L, egui::Modifiers::CTRL);
             harness.press(egui::Key::L, egui::Modifiers::CTRL);
+            harness.press(egui::Key::L, egui::Modifiers::CTRL);
         } else {
             harness.click("Agents");
         }
@@ -82,6 +83,7 @@ fn settings_lists_an_agents_pane_after_appearance() {
         if look.terminal {
             harness.press(egui::Key::H, egui::Modifiers::CTRL);
             harness.press(egui::Key::H, egui::Modifiers::CTRL);
+            harness.press(egui::Key::H, egui::Modifiers::CTRL);
         } else {
             harness.click("General");
         }
```

```diff
diff --git a/src/ui/editor_settings_tests.rs b/src/ui/editor_settings_tests.rs
new file mode 100644
--- /dev/null
+++ b/src/ui/editor_settings_tests.rs
@@ -0,0 +1,453 @@
+//! Settings › Editor, in both looks: what each option does to the SQL
+//! editor, and the pane that sets them with a click or a key and writes
+//! them to the file.
+
+use egui::accesskit::{Role, Toggled};
+use egui::{Key, Modifiers};
+
+use crate::keymap::Command as Keyed;
+use crate::model::{Action, SettingsPane};
+use crate::settings::{
+    Autocomplete, EditorKeys, KeywordCase, LineNumbers, NewTab, OptionValue, RunScope, Settings,
+};
+use crate::testing::Harness;
+use crate::theme::Look;
+
+use super::appearance_settings_tests::{settings_saves, shows};
+
+fn looks() -> [Look; 2] {
+    [Look::macos(), Look::omarchy()]
+}
+
+/// Settings on its Editor pane.
+fn on_the_pane(harness: &mut Harness) {
+    harness.app.apply(Action::ShowSettings);
+    harness
+        .app
+        .apply(Action::ShowSettingsPane(SettingsPane::Editor));
+    harness.settle();
+}
+
+fn type_key(harness: &mut Harness, key: Key, text: &str) {
+    super::tests::type_key(harness, key, text);
+}
+
+#[test]
+fn with_no_file_each_look_has_its_editor_defaults_and_nothing_is_written() {
+    let settings = Settings::default();
+    assert_eq!(settings.new_tab, NewTab::ReadOnly);
+    assert_eq!(settings.sql_limit, 1_000);
+    assert_eq!(settings.sql_timeout_secs, Some(30));
+    assert_eq!(settings.run_scope, RunScope::Statement);
+    assert!(settings.confirm_unsafe_writes);
+    assert_eq!(settings.autocomplete, Autocomplete::Typing);
+    assert!(settings.close_pairs && settings.highlight_line && !settings.wrap);
+    assert_eq!(settings.history_keep, 1_000);
+    // What each look's board draws, while the file says nothing.
+    assert_eq!(settings.editor_keys_in(false), EditorKeys::Standard);
+    assert_eq!(settings.editor_keys_in(true), EditorKeys::Vim);
+    assert_eq!(settings.line_numbers_in(false), LineNumbers::Absolute);
+    assert_eq!(settings.line_numbers_in(true), LineNumbers::Relative);
+    assert_eq!(settings.keywords_in(false), KeywordCase::Upper);
+    assert_eq!(settings.keywords_in(true), KeywordCase::Lower);
+    // Once written, every look has it.
+    let set = Settings {
+        line_numbers: Some(LineNumbers::Off),
+        ..Settings::default()
+    };
+    assert_eq!(set.line_numbers_in(false), LineNumbers::Off);
+    assert_eq!(set.line_numbers_in(true), LineNumbers::Off);
+    for look in looks() {
+        let mut harness = Harness::new();
+        harness.set_look(look);
+        on_the_pane(&mut harness);
+        assert_eq!(settings_saves(&harness), 0, "{}", look.name);
+        assert_eq!(
+            harness.app.settings_file.text,
+            "# written by tabletist, safe to edit by hand\n"
+        );
+    }
+}
+
+#[test]
+fn the_old_keys_are_read_and_give_way_to_the_new_on_a_change() {
+    let text = "# mine\n[editor]\n# the editor's\nsql_limit = 300\nsql_timeout_secs = 10\n";
+    let loaded = Settings::from_toml(text);
+    assert_eq!(loaded.invalid, Vec::<usize>::new());
+    assert_eq!(loaded.settings.sql_limit, 300);
+    assert_eq!(loaded.settings.sql_timeout_secs, Some(10));
+    let before = loaded.settings;
+    let mut after = before.clone();
+    after.sql_limit = 500;
+    let written = after.write_into(text, &before.changed_keys(&after));
+    assert_eq!(
+        written,
+        "# mine\n[editor]\n# the editor's\nsql_timeout_secs = 10\n\n\
+         [editor.run]\nlimit = 500  # 0 is no limit\n"
+    );
+    // The new key is read over an old one that is still there.
+    let both = "[editor]\nsql_limit = 300\n\n[editor.run]\nlimit = 0\n";
+    assert_eq!(Settings::from_toml(both).settings.sql_limit, 0);
+    // A value of the old key that the new one cannot have is no longer
+    // in the menus: it is kept and shown in its place.
+    assert_eq!(Settings::sql_limits_with(300)[..3], [100, 300, 500]);
+    assert_eq!(
+        Settings::sql_timeouts_with(Some(10))[..2],
+        [Some(5), Some(10)]
+    );
+}
+
+#[test]
+fn a_value_an_editor_key_cannot_have_is_ignored_by_its_line() {
+    let text = "[editor.run]\nnew_tab = \"rw\"\nlimit = 500\ntimeout = \"2h\"\nrun = \"all\"\n\n\
+                [editor.typing]\nindent = 3\nline_numbers = \"on\"\nautocomplete = \"manual\"\n";
+    let loaded = Settings::from_toml(text);
+    assert_eq!(loaded.invalid, [2, 4, 8, 9]);
+    let settings = loaded.settings;
+    assert_eq!(settings.new_tab, NewTab::ReadOnly);
+    assert_eq!(settings.sql_limit, 500);
+    assert_eq!(settings.sql_timeout_secs, Some(30));
+    assert_eq!(settings.run_scope, RunScope::All);
+    assert_eq!(settings.indent, crate::settings::Indent::Two);
+    assert_eq!(settings.line_numbers, None);
+    assert_eq!(settings.autocomplete, Autocomplete::Manual);
+    // Indent is two, four or a tab, as a number or a word.
+    for (value, indent) in [
+        ("2", crate::settings::Indent::Two),
+        ("4", crate::settings::Indent::Four),
+        ("\"tab\"", crate::settings::Indent::Tab),
+    ] {
+        let text = format!("[editor.typing]\nindent = {value}\n");
+        assert_eq!(
+            Settings::from_toml(&text).settings.indent,
+            indent,
+            "{value}"
+        );
+    }
+    assert_eq!(
+        Settings::from_toml("[editor.typing]\nindent = \"4\"\n").invalid,
+        [2]
+    );
+}
+
+#[test]
+fn the_sheet_sets_each_editor_option_with_a_click() {
+    let look = Look::macos();
+    let mut harness = Harness::with_size(egui::vec2(1440.0, 1100.0));
+    harness.set_look(look);
+    on_the_pane(&mut harness);
+    // The board's words, and the run key's from the keymap.
+    let run = crate::ui::keys::written(&harness.ctx, &look, Keyed::RunStatement);
+    let runs = format!("{} runs", run.as_str());
+    let complete = crate::ui::keys::written(&harness.ctx, &look, Keyed::Complete);
+    for words in [
+        "RUNNING QUERIES",
+        "TYPING",
+        "The badge in each tab can still switch it",
+        "Caps the rows of results you browse",
+        &runs,
+        "A selection always wins",
+        "Tables, views, columns and functions from the schema",
+        &format!("On {}", complete.as_str()),
+        "Current line highlight",
+    ] {
+        assert!(shows(&mut harness, words), "{words}");
+    }
+    // The rows of what is not built yet are not there.
+    for words in [
+        "Keybindings",
+        "Indent",
+        "Wrap",
+        "FORMAT",
+        "HISTORY",
+        "Keywords",
+    ] {
+        assert!(!shows(&mut harness, words), "{words}");
+    }
+    let saves = settings_saves(&harness);
+    harness.click("New tabs run in: Follow connection");
+    harness.click("Row limit");
+    harness.click("Row limit 500");
+    harness.click("Timeout");
+    harness.click("Timeout 1 min");
+    harness.click(&format!("{runs}: Whole tab"));
+    harness.click("Autocomplete: Off");
+    harness.click("Line numbers");
+    harness.click("Current line highlight");
+    harness.settle();
+    let settings = &harness.app.settings;
+    assert_eq!(settings.new_tab, NewTab::Connection);
+    assert_eq!(settings.sql_limit, 500);
+    assert_eq!(settings.sql_timeout_secs, Some(60));
+    assert_eq!(settings.run_scope, RunScope::All);
+    assert_eq!(settings.autocomplete, Autocomplete::Off);
+    assert_eq!(settings.line_numbers, Some(LineNumbers::Off));
+    assert!(!settings.highlight_line);
+    assert_eq!(settings_saves(&harness), saves + 7);
+    // On again, the numbers are each line's own.
+    harness.click("Line numbers");
+    assert_eq!(
+        harness.app.settings.line_numbers,
+        Some(LineNumbers::Absolute)
+    );
+}
+
+#[test]
+fn a_value_the_menus_do_not_list_is_shown_in_its_place() {
+    // The file sets a limit of 300: the sheet's menu lists it between 100
+    // and 500, and the screen steps from it to 500.
+    let mut harness = Harness::new();
+    harness.set_look(Look::macos());
+    harness
+        .app
+        .apply(Action::SetOption(OptionValue::RowLimit(300)));
+    harness
+        .app
+        .apply(Action::SetOption(OptionValue::Timeout(Some(120))));
+    on_the_pane(&mut harness);
+    harness.click("Row limit");
+    for entry in ["Row limit 100", "Row limit 300", "Row limit 500"] {
+        assert!(harness.has(entry), "{entry}");
+    }
+    harness.press(Key::Escape, Modifiers::NONE);
+    harness.click("Timeout");
+    assert!(harness.has("Timeout 2 min"));
+    let mut harness = Harness::with_size(egui::vec2(1920.0, 1080.0));
+    harness.set_look(Look::omarchy());
+    harness
+        .app
+        .apply(Action::SetOption(OptionValue::RowLimit(300)));
+    on_the_pane(&mut harness);
+    assert!(shows(&mut harness, "300"));
+    type_key(&mut harness, Key::J, "j");
+    type_key(&mut harness, Key::L, "l");
+    assert_eq!(harness.app.settings.sql_limit, 500);
+}
+
+#[test]
+fn a_stored_relative_shows_on_in_the_sheet_and_stays() {
+    let mut harness = Harness::new();
+    harness.set_look(Look::macos());
+    harness
+        .app
+        .apply(Action::SetOption(OptionValue::LineNumbers(Some(
+            LineNumbers::Relative,
+        ))));
+    on_the_pane(&mut harness);
+    let tree = harness.settle();
+    let switch = tree
+        .nodes
+        .iter()
+        .map(|(_, node)| node)
+        .find(|node| node.role() == Role::CheckBox && node.label() == Some("Line numbers"))
+        .expect("the switch");
+    assert_eq!(switch.toggled(), Some(Toggled::True));
+    harness.click("Current line highlight");
+    assert_eq!(
+        harness.app.settings.line_numbers,
+        Some(LineNumbers::Relative)
+    );
+}
+
+#[test]
+fn the_screen_sets_each_editor_option_with_its_keys() {
+    let mut harness = Harness::with_size(egui::vec2(1920.0, 1080.0));
+    harness.set_look(Look::omarchy());
+    on_the_pane(&mut harness);
+    let read_only = crate::ui::keys::written(&harness.ctx, &Look::omarchy(), Keyed::ReadOnlyTab);
+    for words in [
+        "running queries",
+        "typing",
+        read_only.as_str(),
+        "per tab",
+        "caps results you browse",
+        "a selection always wins",
+        "schema objects",
+    ] {
+        assert!(shows(&mut harness, words), "{words}");
+    }
+    assert!(!shows(&mut harness, "current line"));
+    let saves = settings_saves(&harness);
+    // New tabs: follow the connection.
+    type_key(&mut harness, Key::L, "l");
+    // The limit a step up, the timeout a step down.
+    type_key(&mut harness, Key::J, "j");
+    type_key(&mut harness, Key::L, "l");
+    type_key(&mut harness, Key::J, "j");
+    type_key(&mut harness, Key::H, "h");
+    // The whole tab, with space.
+    type_key(&mut harness, Key::J, "j");
+    harness.press(Key::Space, Modifiers::NONE);
+    // Line numbers from relative to absolute; completion on the key.
+    type_key(&mut harness, Key::J, "j");
+    type_key(&mut harness, Key::L, "l");
+    type_key(&mut harness, Key::J, "j");
+    type_key(&mut harness, Key::L, "l");
+    harness.settle();
+    let settings = &harness.app.settings;
+    assert_eq!(settings.new_tab, NewTab::Connection);
+    assert_eq!(settings.sql_limit, 5_000);
+    assert_eq!(settings.sql_timeout_secs, Some(15));
+    assert_eq!(settings.run_scope, RunScope::All);
+    assert_eq!(settings.line_numbers, Some(LineNumbers::Absolute));
+    assert_eq!(settings.autocomplete, Autocomplete::Manual);
+    assert_eq!(settings_saves(&harness), saves + 6);
+    // The file has each of them, and R puts the cursor's back.
+    let text = harness.app.settings_file.text.clone();
+    assert!(
+        text.contains("[editor.run]\nnew_tab = \"connection\""),
+        "{text}"
+    );
+    assert!(text.contains("autocomplete = \"manual\""), "{text}");
+    harness.press(Key::R, Modifiers::SHIFT);
+    harness.settle();
+    assert_eq!(harness.app.settings.autocomplete, Autocomplete::Typing);
+}
+
+#[test]
+fn a_look_s_own_value_picked_while_none_is_set_is_not_written() {
+    // Omarchy: `h` on line numbers, which show the look's relative, the
+    // leftmost; a click on the relative segment.
+    let mut harness = Harness::with_size(egui::vec2(1920.0, 1080.0));
+    harness.set_look(Look::omarchy());
+    on_the_pane(&mut harness);
+    for _ in 0..4 {
+        type_key(&mut harness, Key::J, "j");
+    }
+    type_key(&mut harness, Key::H, "h");
+    harness.click("Line numbers: relative");
+    harness.settle();
+    assert_eq!(harness.app.settings.line_numbers, None);
+    assert_eq!(settings_saves(&harness), 0);
+    // Another value is written, and is then every look's.
+    type_key(&mut harness, Key::L, "l");
+    assert_eq!(
+        harness.app.settings.line_numbers,
+        Some(LineNumbers::Absolute)
+    );
+    harness.click("Line numbers: relative");
+    assert_eq!(
+        harness.app.settings.line_numbers,
+        Some(LineNumbers::Relative)
+    );
+    // The same for the keys and the keyword case, in either look.
+    let settings = Settings::default();
+    for terminal in [false, true] {
+        let keys = OptionValue::EditorKeys(Some(EditorKeys::of_look(terminal)));
+        assert_eq!(
+            keys.for_look(&settings, terminal),
+            OptionValue::EditorKeys(None)
+        );
+        let case = OptionValue::Keywords(Some(KeywordCase::of_look(!terminal)));
+        assert_eq!(case.clone().for_look(&settings, terminal), case);
+    }
+}
+
+#[test]
+fn the_run_scope_is_a_radio_group_named_with_the_run_key_and_described() {
+    for look in looks() {
+        let mut harness = Harness::with_size(egui::vec2(1920.0, 2400.0));
+        harness.set_look(look);
+        on_the_pane(&mut harness);
+        let tree = harness.settle();
+        let run = crate::ui::keys::written(&harness.ctx, &look, Keyed::RunStatement);
+        let name = format!("{} runs", run.as_str());
+        let name = if look.terminal {
+            look.label(&name)
+        } else {
+            name
+        };
+        let (_, group) = tree
+            .nodes
+            .iter()
+            .find(|(_, node)| node.role() == Role::RadioGroup && node.label() == Some(&name))
+            .unwrap_or_else(|| panic!("no radio group {name} in {}", look.name));
+        let children = group.children().to_vec();
+        let buttons: Vec<_> = tree
+            .nodes
+            .iter()
+            .filter(|(id, node)| children.contains(id) && node.role() == Role::RadioButton)
+            .map(|(_, node)| node)
+            .collect();
+        assert_eq!(buttons.len(), 2, "{}", look.name);
+        for button in buttons {
+            assert_eq!(
+                button.description(),
+                Some("A selection always wins"),
+                "{:?} in {}",
+                button.label(),
+                look.name
+            );
+        }
+    }
+}
+
+#[test]
+fn reset_to_defaults_puts_every_editor_key_back_and_nothing_else() {
+    let mut harness = Harness::new();
+    harness.set_look(Look::macos());
+    for value in [
+        OptionValue::NewTab(NewTab::Connection),
+        OptionValue::RowLimit(0),
+        OptionValue::Timeout(None),
+        OptionValue::RunScope(RunScope::All),
+        OptionValue::Autocomplete(Autocomplete::Off),
+        OptionValue::LineNumbers(Some(LineNumbers::Off)),
+        OptionValue::HighlightLine(false),
+        // Keys the pane has no row for yet are its too.
+        OptionValue::Wrap(true),
+        OptionValue::EditorKeys(Some(EditorKeys::Vim)),
+        OptionValue::HistoryKeep(0),
+        OptionValue::PageSize(500),
+    ] {
+        harness.app.apply(Action::SetOption(value));
+    }
+    on_the_pane(&mut harness);
+    harness.click("Reset to defaults");
+    assert!(shows(
+        &mut harness,
+        "Reset all Editor settings to their defaults?"
+    ));
+    harness.app.apply(Action::ConfirmResetSettings(true));
+    harness.settle();
+    let settings = &harness.app.settings;
+    let defaults = Settings::default();
+    assert_eq!(settings.new_tab, defaults.new_tab);
+    assert_eq!(settings.sql_limit, defaults.sql_limit);
+    assert_eq!(settings.sql_timeout_secs, defaults.sql_timeout_secs);
+    assert_eq!(settings.run_scope, defaults.run_scope);
+    assert_eq!(settings.autocomplete, defaults.autocomplete);
+    assert_eq!(settings.line_numbers, None);
+    assert!(settings.highlight_line);
+    assert!(!settings.wrap);
+    assert_eq!(settings.editor_keys, None);
+    assert_eq!(settings.history_keep, defaults.history_keep);
+    // Another pane's options are its own.
+    assert_eq!(settings.page_size, 500);
+}
+
+#[test]
+fn the_file_pane_marks_the_old_keys_line_for_the_row_it_sets() {
+    let mut harness = Harness::with_size(egui::vec2(1920.0, 1080.0));
+    harness.set_look(Look::omarchy());
+    on_the_pane(&mut harness);
+    harness
+        .app
+        .apply(Action::Backend(crate::backend::Event::SettingsFile {
+            text: "[editor]\nsql_limit = 300\n".into(),
+            own: false,
+        }));
+    harness.settle();
+    // The row limit, one under the first row.
+    type_key(&mut harness, Key::J, "j");
+    harness.settle();
+    let selection = harness.app.palette.selection;
+    let line = harness
+        .painted_rect("sql_limit = ")
+        .expect("the old key's line");
+    assert!(harness.fills.iter().any(|(rect, color)| {
+        *color == selection
+            && rect.y_range().contains(line.center().y)
+            && rect.x_range().contains(line.center().x)
+    }));
+}
```

```diff
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -28,6 +28,8 @@ mod delete_tests;
 #[cfg(test)]
 mod duplicate_tests;
 #[cfg(test)]
+mod editor_settings_tests;
+#[cfg(test)]
 mod env_tests;
 pub mod filter_bar;
 pub mod focus;
```

```diff
diff --git a/src/ui/settings_access_tests.rs b/src/ui/settings_access_tests.rs
--- a/src/ui/settings_access_tests.rs
+++ b/src/ui/settings_access_tests.rs
@@ -9,7 +9,7 @@ use crate::settings::OptionId;
 use crate::testing::Harness;
 use crate::theme::Look;
 
-use super::settings::{label, small_print, terminal_label};
+use super::settings::{helper, label, small_print, terminal_label};
 
 fn looks() -> [Look; 2] {
     [Look::macos(), Look::omarchy()]
@@ -87,7 +87,7 @@ fn every_rows_controls_are_described_by_its_small_print() {
                 if found.is_empty() {
                     continue;
                 }
-                let said = small_print(row(&look, option));
+                let said = helper(row(&look, option));
                 for node in found {
                     assert_eq!(
                         node.description(),
@@ -104,12 +104,14 @@ fn every_rows_controls_are_described_by_its_small_print() {
             }
         }
         // Every row that has small print, as each look lists its rows:
-        // the terminal look has no interface size.
+        // the terminal look has no interface size, and the Editor pane has
+        // no row yet for some of its options.
         described.sort_by_key(|option| OptionId::ALL.iter().position(|all| all == option));
         let expected: Vec<OptionId> = OptionId::ALL
             .into_iter()
             .filter(|option| small_print(*option).is_some())
             .filter(|option| !(look.terminal && *option == OptionId::InterfaceSize))
+            .filter(|option| !OptionId::EDITOR_LATER.contains(option))
             .collect();
         assert_eq!(described, expected, "in {}", look.name);
     }
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test --locked --lib -- settings ui::editor_settings_tests ui::agent_settings_tests ui::settings_access_tests`
Expected: FAIL to compile (28 errors): `OptionId` has no `EDITOR`, `EDITOR_LATER` or editor options, `OptionValue` no `NewTab`, `RowLimit`, `EditorKeys`, `HighlightLine`, `Wrap`, `HistoryKeep` and the others, and `settings` no `helper`.

- [ ] **Step 3: Add the pane**

```diff
diff --git a/src/app.rs b/src/app.rs
--- a/src/app.rs
+++ b/src/app.rs
@@ -2134,7 +2134,10 @@ impl App {
             Action::SetOption(crate::settings::OptionValue::AgentsOn(true)) => {
                 self.turn_agents_on();
             }
-            Action::SetOption(value) => self.change_settings(|settings| value.set(settings)),
+            Action::SetOption(value) => {
+                let value = value.for_look(&self.settings, self.look.terminal);
+                self.change_settings(|settings| value.set(settings))
+            }
             Action::EditSettingsFile => {
                 let text = self.offer_settings_text();
                 self.backend.send(Command::EditSettingsFile {
```

```diff
diff --git a/src/model.rs b/src/model.rs
--- a/src/model.rs
+++ b/src/model.rs
@@ -2512,6 +2512,8 @@ pub enum SettingsPane {
     General,
     /// How the grid and the connection bar are drawn.
     Appearance,
+    /// How the SQL editor runs, types and formats.
+    Editor,
     /// How agents reach the app. Listed where they can.
     Agents,
 }
@@ -2520,9 +2522,9 @@ impl SettingsPane {
     /// The panes the window lists, in order.
     pub fn listed() -> &'static [SettingsPane] {
         if crate::endpoint::AVAILABLE {
-            &[Self::General, Self::Appearance, Self::Agents]
+            &[Self::General, Self::Appearance, Self::Editor, Self::Agents]
         } else {
-            &[Self::General, Self::Appearance]
+            &[Self::General, Self::Appearance, Self::Editor]
         }
     }
 
@@ -2531,6 +2533,7 @@ impl SettingsPane {
         match self {
             Self::General => "General",
             Self::Appearance => "Appearance",
+            Self::Editor => "Editor",
             Self::Agents => "Agents",
         }
     }
@@ -2540,18 +2543,21 @@ impl SettingsPane {
         match self {
             Self::General => &crate::settings::OptionId::GENERAL,
             Self::Appearance => &crate::settings::OptionId::APPEARANCE,
+            Self::Editor => &crate::settings::OptionId::EDITOR,
             Self::Agents => &crate::settings::OptionId::AGENTS,
         }
     }
 
     /// What Reset to defaults puts back: the pane's options but the
     /// server's switch, which turned off would end every agent's session
-    /// for a question about defaults.
+    /// for a question about defaults. The Editor pane's are all of its
+    /// keys, the ones it has no row for yet among them.
     pub fn reset_options(self) -> &'static [crate::settings::OptionId] {
         use crate::settings::OptionId;
         match self {
             Self::General => &OptionId::GENERAL,
             Self::Appearance => &OptionId::APPEARANCE,
+            Self::Editor => &OptionId::EDITOR,
             Self::Agents => &[OptionId::LogDays, OptionId::AgentsNotify],
         }
     }
@@ -2582,6 +2588,15 @@ impl SettingsPane {
                 SettingsRow::Option(OptionId::TableList),
                 SettingsRow::Option(OptionId::EnvBar),
             ],
+            // As the terminal look's board orders them.
+            Self::Editor => &[
+                SettingsRow::Option(OptionId::NewTab),
+                SettingsRow::Option(OptionId::RowLimit),
+                SettingsRow::Option(OptionId::Timeout),
+                SettingsRow::Option(OptionId::RunScope),
+                SettingsRow::Option(OptionId::LineNumbers),
+                SettingsRow::Option(OptionId::Autocomplete),
+            ],
             Self::Agents => &[
                 SettingsRow::Option(OptionId::AgentsOn),
                 SettingsRow::AgentsStatus,
```

```diff
diff --git a/src/settings.rs b/src/settings.rs
--- a/src/settings.rs
+++ b/src/settings.rs
@@ -926,12 +926,36 @@ pub enum OptionId {
     ColumnTypes,
     TableList,
     EnvBar,
+    /// The mode a new SQL tab opens in: `editor.run.new_tab`.
+    NewTab,
+    /// Rows a statement keeps: `editor.run.limit`.
+    RowLimit,
+    /// How long a run may take: `editor.run.timeout`.
+    Timeout,
+    /// What the run key runs: `editor.run.run`.
+    RunScope,
+    ConfirmUnsafeWrites,
+    /// The editor's keys: `editor.typing.keys`.
+    EditorKeys,
+    Indent,
+    Autocomplete,
+    /// Close brackets and quotes: `editor.typing.close_pairs`.
+    ClosePairs,
+    LineNumbers,
+    /// The cursor's line filled: `editor.typing.highlight_line`.
+    HighlightLine,
+    Wrap,
+    /// The case Format writes keywords in: `editor.format.keywords`.
+    Keywords,
+    Commas,
+    /// Queries the history keeps: `editor.history.keep`.
+    HistoryKeep,
 }
 
 impl OptionId {
-    /// Every option, the General pane's, the Appearance pane's and then
-    /// the Agents pane's.
-    pub const ALL: [OptionId; 22] = [
+    /// Every option, the General pane's, the Appearance pane's, the Editor
+    /// pane's and then the Agents pane's.
+    pub const ALL: [OptionId; 37] = [
         Self::OnLaunch,
         Self::RestoreTabs,
         Self::PageSize,
@@ -951,6 +975,21 @@ impl OptionId {
         Self::ColumnTypes,
         Self::TableList,
         Self::EnvBar,
+        Self::NewTab,
+        Self::RowLimit,
+        Self::Timeout,
+        Self::RunScope,
+        Self::ConfirmUnsafeWrites,
+        Self::EditorKeys,
+        Self::Indent,
+        Self::Autocomplete,
+        Self::ClosePairs,
+        Self::LineNumbers,
+        Self::HighlightLine,
+        Self::Wrap,
+        Self::Keywords,
+        Self::Commas,
+        Self::HistoryKeep,
         Self::AgentsOn,
         Self::LogDays,
         Self::AgentsNotify,
@@ -1010,6 +1049,46 @@ impl OptionId {
         Self::TableList,
     ];
     pub const ENVIRONMENT: [OptionId; 1] = [Self::EnvBar];
+    /// Every option of the Editor pane, the ones it does not show yet
+    /// among them: what its Reset to defaults puts back.
+    pub const EDITOR: [OptionId; 15] = [
+        Self::NewTab,
+        Self::RowLimit,
+        Self::Timeout,
+        Self::RunScope,
+        Self::ConfirmUnsafeWrites,
+        Self::EditorKeys,
+        Self::Indent,
+        Self::Autocomplete,
+        Self::ClosePairs,
+        Self::LineNumbers,
+        Self::HighlightLine,
+        Self::Wrap,
+        Self::Keywords,
+        Self::Commas,
+        Self::HistoryKeep,
+    ];
+    /// The Editor pane's options whose features are not built yet: no row
+    /// draws them, and their keys are read, kept and reset.
+    pub const EDITOR_LATER: [OptionId; 8] = [
+        Self::ConfirmUnsafeWrites,
+        Self::EditorKeys,
+        Self::Indent,
+        Self::ClosePairs,
+        Self::Wrap,
+        Self::Keywords,
+        Self::Commas,
+        Self::HistoryKeep,
+    ];
+    /// The Editor pane's Running queries section, as far as its options
+    /// are built.
+    pub const RUNNING: [OptionId; 4] =
+        [Self::NewTab, Self::RowLimit, Self::Timeout, Self::RunScope];
+    /// Its Typing section on macOS and Windows.
+    pub const TYPING: [OptionId; 3] = [Self::Autocomplete, Self::LineNumbers, Self::HighlightLine];
+    /// The terminal look's, as its board orders them, with no row for the
+    /// cursor's line.
+    pub const TERMINAL_TYPING: [OptionId; 2] = [Self::LineNumbers, Self::Autocomplete];
     /// The Agents pane's, in the order it has them.
     pub const AGENTS: [OptionId; 3] = [Self::AgentsOn, Self::LogDays, Self::AgentsNotify];
 
@@ -1038,6 +1117,21 @@ impl OptionId {
             Self::ColumnTypes => Key::ColumnTypes,
             Self::TableList => Key::TableList,
             Self::EnvBar => Key::EnvBar,
+            Self::NewTab => Key::NewTab,
+            Self::RowLimit => Key::SqlLimit,
+            Self::Timeout => Key::SqlTimeoutSecs,
+            Self::RunScope => Key::RunScope,
+            Self::ConfirmUnsafeWrites => Key::ConfirmUnsafeWrites,
+            Self::EditorKeys => Key::EditorKeys,
+            Self::Indent => Key::Indent,
+            Self::Autocomplete => Key::Autocomplete,
+            Self::ClosePairs => Key::ClosePairs,
+            Self::LineNumbers => Key::LineNumbers,
+            Self::HighlightLine => Key::HighlightLine,
+            Self::Wrap => Key::Wrap,
+            Self::Keywords => Key::Keywords,
+            Self::Commas => Key::Commas,
+            Self::HistoryKeep => Key::HistoryKeep,
         }
     }
 
@@ -1076,9 +1170,55 @@ pub enum OptionValue {
     ColumnTypes(bool),
     TableList(TableList),
     EnvBar(EnvBar),
+    NewTab(NewTab),
+    /// 0 keeps every row.
+    RowLimit(u32),
+    /// Seconds; `None` is no timeout.
+    Timeout(Option<u32>),
+    RunScope(RunScope),
+    ConfirmUnsafeWrites(bool),
+    /// `None` is the look's.
+    EditorKeys(Option<EditorKeys>),
+    Indent(Indent),
+    Autocomplete(Autocomplete),
+    ClosePairs(bool),
+    /// `None` is the look's.
+    LineNumbers(Option<LineNumbers>),
+    HighlightLine(bool),
+    Wrap(bool),
+    /// `None` is the look's.
+    Keywords(Option<KeywordCase>),
+    Commas(Commas),
+    /// 0 keeps none.
+    HistoryKeep(u32),
 }
 
 impl OptionValue {
+    /// The value as it is set from a look that is the terminal's
+    /// (`terminal`) or not, over `settings`: a look's own default picked
+    /// while nothing is set stays not set, and is not written. Once
+    /// written, a value is every look's.
+    pub fn for_look(self, settings: &Settings, terminal: bool) -> Self {
+        match self {
+            Self::EditorKeys(Some(keys))
+                if settings.editor_keys.is_none() && keys == EditorKeys::of_look(terminal) =>
+            {
+                Self::EditorKeys(None)
+            }
+            Self::LineNumbers(Some(numbers))
+                if settings.line_numbers.is_none() && numbers == LineNumbers::of_look(terminal) =>
+            {
+                Self::LineNumbers(None)
+            }
+            Self::Keywords(Some(case))
+                if settings.keywords.is_none() && case == KeywordCase::of_look(terminal) =>
+            {
+                Self::Keywords(None)
+            }
+            other => other,
+        }
+    }
+
     pub fn option(&self) -> OptionId {
         match self {
             Self::OnLaunch(_) => OptionId::OnLaunch,
@@ -1103,6 +1243,21 @@ impl OptionValue {
             Self::ColumnTypes(_) => OptionId::ColumnTypes,
             Self::TableList(_) => OptionId::TableList,
             Self::EnvBar(_) => OptionId::EnvBar,
+            Self::NewTab(_) => OptionId::NewTab,
+            Self::RowLimit(_) => OptionId::RowLimit,
+            Self::Timeout(_) => OptionId::Timeout,
+            Self::RunScope(_) => OptionId::RunScope,
+            Self::ConfirmUnsafeWrites(_) => OptionId::ConfirmUnsafeWrites,
+            Self::EditorKeys(_) => OptionId::EditorKeys,
+            Self::Indent(_) => OptionId::Indent,
+            Self::Autocomplete(_) => OptionId::Autocomplete,
+            Self::ClosePairs(_) => OptionId::ClosePairs,
+            Self::LineNumbers(_) => OptionId::LineNumbers,
+            Self::HighlightLine(_) => OptionId::HighlightLine,
+            Self::Wrap(_) => OptionId::Wrap,
+            Self::Keywords(_) => OptionId::Keywords,
+            Self::Commas(_) => OptionId::Commas,
+            Self::HistoryKeep(_) => OptionId::HistoryKeep,
         }
     }
 
@@ -1142,10 +1297,52 @@ impl OptionValue {
             Self::ColumnTypes(on) => settings.column_types = on,
             Self::TableList(list) => settings.table_list = list,
             Self::EnvBar(bar) => settings.env_bar = bar,
+            Self::NewTab(choice) => settings.new_tab = choice,
+            Self::RowLimit(limit) => settings.sql_limit = Settings::valid_sql_limit(limit),
+            Self::Timeout(secs) => settings.sql_timeout_secs = Settings::valid_sql_timeout(secs),
+            Self::RunScope(scope) => settings.run_scope = scope,
+            Self::ConfirmUnsafeWrites(on) => settings.confirm_unsafe_writes = on,
+            Self::EditorKeys(keys) => settings.editor_keys = keys,
+            Self::Indent(indent) => settings.indent = indent,
+            Self::Autocomplete(choice) => settings.autocomplete = choice,
+            Self::ClosePairs(on) => settings.close_pairs = on,
+            Self::LineNumbers(numbers) => settings.line_numbers = numbers,
+            Self::HighlightLine(on) => settings.highlight_line = on,
+            Self::Wrap(on) => settings.wrap = on,
+            Self::Keywords(case) => settings.keywords = case,
+            Self::Commas(commas) => settings.commas = commas,
+            Self::HistoryKeep(keep) => settings.history_keep = keep.min(Settings::MAX_HISTORY),
         }
     }
 }
 
+/// The value one step from `current` among `choices`, which are in the
+/// order of `rank`: the next larger (`forward`) or smaller one, or
+/// `current` past the ends. A value not among them steps from its place.
+fn stepped_in<T: Copy>(choices: &[T], current: T, rank: impl Fn(&T) -> u64, forward: bool) -> T {
+    let at = rank(&current);
+    let next = if forward {
+        choices.iter().find(|choice| rank(choice) > at)
+    } else {
+        choices.iter().rev().find(|choice| rank(choice) < at)
+    };
+    next.copied().unwrap_or(current)
+}
+
+/// The choice one step from `current` in `choices`, none past the ends.
+fn stepped_among<T: Copy + PartialEq>(choices: &[T], current: T, forward: bool) -> T {
+    let at = choices
+        .iter()
+        .position(|choice| *choice == current)
+        .unwrap_or(0);
+    let next = if forward {
+        (at + 1).min(choices.len() - 1)
+    } else {
+        at.saturating_sub(1)
+    };
+    choices[next]
+}
+
 /// The file's first line.
 const HEADER: &str = "# written by tabletist, safe to edit by hand\n";
 
@@ -1750,6 +1947,23 @@ impl Settings {
             OptionId::ColumnTypes => OptionValue::ColumnTypes(self.column_types),
             OptionId::TableList => OptionValue::TableList(self.table_list),
             OptionId::EnvBar => OptionValue::EnvBar(self.env_bar),
+            OptionId::NewTab => OptionValue::NewTab(self.new_tab),
+            OptionId::RowLimit => OptionValue::RowLimit(self.sql_limit),
+            OptionId::Timeout => OptionValue::Timeout(self.sql_timeout_secs),
+            OptionId::RunScope => OptionValue::RunScope(self.run_scope),
+            OptionId::ConfirmUnsafeWrites => {
+                OptionValue::ConfirmUnsafeWrites(self.confirm_unsafe_writes)
+            }
+            OptionId::EditorKeys => OptionValue::EditorKeys(self.editor_keys),
+            OptionId::Indent => OptionValue::Indent(self.indent),
+            OptionId::Autocomplete => OptionValue::Autocomplete(self.autocomplete),
+            OptionId::ClosePairs => OptionValue::ClosePairs(self.close_pairs),
+            OptionId::LineNumbers => OptionValue::LineNumbers(self.line_numbers),
+            OptionId::HighlightLine => OptionValue::HighlightLine(self.highlight_line),
+            OptionId::Wrap => OptionValue::Wrap(self.wrap),
+            OptionId::Keywords => OptionValue::Keywords(self.keywords),
+            OptionId::Commas => OptionValue::Commas(self.commas),
+            OptionId::HistoryKeep => OptionValue::HistoryKeep(self.history_keep),
         }
     }
 
@@ -1880,6 +2094,83 @@ impl Settings {
             OptionId::EnvBar => {
                 OptionValue::EnvBar(if forward { EnvBar::Label } else { EnvBar::Tint })
             }
+            // Read-only is the left of the two, and so is the statement.
+            OptionId::NewTab => OptionValue::NewTab(if forward {
+                NewTab::Connection
+            } else {
+                NewTab::ReadOnly
+            }),
+            // No limit is past the largest: 0 is the right end.
+            OptionId::RowLimit => OptionValue::RowLimit(stepped_in(
+                &Self::SQL_LIMITS,
+                self.sql_limit,
+                |limit| {
+                    if *limit == 0 {
+                        u64::MAX
+                    } else {
+                        u64::from(*limit)
+                    }
+                },
+                forward,
+            )),
+            OptionId::Timeout => OptionValue::Timeout(stepped_in(
+                &Self::SQL_TIMEOUTS,
+                self.sql_timeout_secs,
+                |secs| secs.map_or(u64::MAX, u64::from),
+                forward,
+            )),
+            OptionId::RunScope => OptionValue::RunScope(if forward {
+                RunScope::All
+            } else {
+                RunScope::Statement
+            }),
+            OptionId::ConfirmUnsafeWrites => OptionValue::ConfirmUnsafeWrites(forward),
+            // The terminal look's board has vim at the left.
+            OptionId::EditorKeys => OptionValue::EditorKeys(Some(if forward {
+                EditorKeys::Standard
+            } else {
+                EditorKeys::Vim
+            })),
+            OptionId::Indent => {
+                OptionValue::Indent(stepped_among(&Indent::ALL, self.indent, forward))
+            }
+            OptionId::Autocomplete => OptionValue::Autocomplete(stepped_among(
+                &Autocomplete::ALL,
+                self.autocomplete,
+                forward,
+            )),
+            OptionId::ClosePairs => OptionValue::ClosePairs(forward),
+            // As the terminal look orders them: relative, absolute, off.
+            OptionId::LineNumbers => OptionValue::LineNumbers(Some(stepped_among(
+                &LineNumbers::TERMINAL,
+                self.line_numbers_in(true),
+                forward,
+            ))),
+            OptionId::HighlightLine => OptionValue::HighlightLine(forward),
+            OptionId::Wrap => OptionValue::Wrap(forward),
+            OptionId::Keywords => OptionValue::Keywords(Some(stepped_among(
+                &KeywordCase::ALL,
+                self.keywords_in(true),
+                forward,
+            ))),
+            OptionId::Commas => OptionValue::Commas(if forward {
+                Commas::Leading
+            } else {
+                Commas::Trailing
+            }),
+            // Don't keep is past the most: 0 is the right end.
+            OptionId::HistoryKeep => OptionValue::HistoryKeep(stepped_in(
+                &Self::HISTORY_KEEPS,
+                self.history_keep,
+                |keep| {
+                    if *keep == 0 {
+                        u64::MAX
+                    } else {
+                        u64::from(*keep)
+                    }
+                },
+                forward,
+            )),
         }
     }
 
@@ -2006,6 +2297,39 @@ impl Settings {
                 EnvBar::Tint => EnvBar::Label,
                 EnvBar::Label => EnvBar::Tint,
             })),
+            OptionId::NewTab => Some(OptionValue::NewTab(match self.new_tab {
+                NewTab::ReadOnly => NewTab::Connection,
+                NewTab::Connection => NewTab::ReadOnly,
+            })),
+            OptionId::RunScope => Some(OptionValue::RunScope(match self.run_scope {
+                RunScope::Statement => RunScope::All,
+                RunScope::All => RunScope::Statement,
+            })),
+            OptionId::ConfirmUnsafeWrites => Some(OptionValue::ConfirmUnsafeWrites(
+                !self.confirm_unsafe_writes,
+            )),
+            // Unset, it is flipped from the terminal look's: the window
+            // flips the settings as it shows them, with the look's in.
+            OptionId::EditorKeys => Some(OptionValue::EditorKeys(Some(
+                match self.editor_keys_in(true) {
+                    EditorKeys::Standard => EditorKeys::Vim,
+                    EditorKeys::Vim => EditorKeys::Standard,
+                },
+            ))),
+            OptionId::Commas => Some(OptionValue::Commas(match self.commas {
+                Commas::Trailing => Commas::Leading,
+                Commas::Leading => Commas::Trailing,
+            })),
+            OptionId::ClosePairs => Some(OptionValue::ClosePairs(!self.close_pairs)),
+            OptionId::HighlightLine => Some(OptionValue::HighlightLine(!self.highlight_line)),
+            OptionId::Wrap => Some(OptionValue::Wrap(!self.wrap)),
+            OptionId::RowLimit
+            | OptionId::Timeout
+            | OptionId::Indent
+            | OptionId::Autocomplete
+            | OptionId::LineNumbers
+            | OptionId::Keywords
+            | OptionId::HistoryKeep => None,
         }
     }
 
```

```diff
diff --git a/src/ui/settings/file_pane.rs b/src/ui/settings/file_pane.rs
--- a/src/ui/settings/file_pane.rs
+++ b/src/ui/settings/file_pane.rs
@@ -243,7 +243,17 @@ pub(super) fn show(ui: &mut Ui, rect: Rect, app: &App, option: Option<OptionId>,
     );
 
     let marked = option.and_then(|option| {
-        let line = file.lines.iter().find(|(key, _)| *key == option.key());
+        // The new key's line, else the old key's it took the place of,
+        // which sets it while the file has no new one.
+        let key = option.key();
+        let line = file
+            .lines
+            .iter()
+            .find(|(read, _)| *read == key)
+            .or_else(|| {
+                let old = key.replaced()?;
+                file.lines.iter().find(|(read, _)| *read == old)
+            });
         line.map(|(_, line)| *line)
     });
     let line = role.row_height(ui.ctx(), look.faces);
```

```diff
diff --git a/src/ui/settings/mod.rs b/src/ui/settings/mod.rs
--- a/src/ui/settings/mod.rs
+++ b/src/ui/settings/mod.rs
@@ -19,8 +19,9 @@ use crate::i18n::{Locale, gettext, ngettext};
 use crate::keymap::{Command, Keymap, Layout, Scope, Spelled, Stroke};
 use crate::model::{Action, AgentsView, Dialog, SettingsDialog, SettingsPane, SettingsRow};
 use crate::settings::{
-    EnvBar, Idle, InterfaceSize, Launch, Mode, OptionId, OptionValue, RowHeight, Settings,
-    TableList, Theme, ThemeChoice, Timestamps, Zone,
+    Autocomplete, Commas, EditorKeys, EnvBar, Idle, InterfaceSize, KeywordCase, Launch,
+    LineNumbers, Mode, NewTab, OptionId, OptionValue, RowHeight, RunScope, Settings, TableList,
+    Theme, ThemeChoice, Timestamps, Zone,
 };
 
 /// The label of an option's row. English: the caller translates, and the
@@ -49,9 +50,41 @@ pub(super) fn label(option: OptionId) -> &'static str {
         OptionId::ColumnTypes => "Show column types",
         OptionId::TableList => "Table list",
         OptionId::EnvBar => "Window header",
+        OptionId::NewTab => "New tabs run in",
+        OptionId::RowLimit => "Row limit",
+        OptionId::Timeout => "Timeout",
+        // `{run}` is the run key, as the keymap writes it: see `said`.
+        OptionId::RunScope => "{run} runs",
+        OptionId::ConfirmUnsafeWrites => "Confirm UPDATE or DELETE without WHERE",
+        OptionId::EditorKeys => "Keybindings",
+        OptionId::Indent => "Indent",
+        OptionId::Autocomplete => "Autocomplete",
+        OptionId::ClosePairs => "Close brackets and quotes",
+        OptionId::LineNumbers => "Line numbers",
+        OptionId::HighlightLine => "Current line highlight",
+        OptionId::Wrap => "Wrap",
+        OptionId::Keywords => "Keywords",
+        OptionId::Commas => "Commas",
+        OptionId::HistoryKeep => "Keep query history",
     }
 }
 
+/// A row's label as the window writes it, translated: with the run key in
+/// place of `{run}`, as the keymap writes it for the look.
+pub(super) fn said(
+    ctx: &egui::Context,
+    look: &crate::theme::Look,
+    locale: Locale,
+    text: &'static str,
+) -> String {
+    let said = gettext(locale, text);
+    if !said.contains("{run}") {
+        return said.into_owned();
+    }
+    let key = crate::ui::keys::written(ctx, look, Command::RunStatement);
+    said.replace("{run}", key.as_str())
+}
+
 /// The label of an option's row in the terminal look, which names some
 /// options its own way, as their artboards do (`timestamptz zone`, the
 /// Appearance pane's). English, as [`label`]'s, and lower-cased by the
@@ -65,6 +98,9 @@ pub(super) fn terminal_label(option: OptionId) -> &'static str {
         OptionId::Zebra => "Alternate rows",
         OptionId::ColumnTypes => "Column types",
         OptionId::EnvBar => "Connection bar",
+        OptionId::ConfirmUnsafeWrites => "Confirm write without where",
+        OptionId::ClosePairs => "Close brackets",
+        OptionId::HistoryKeep => "Keep",
         // One font and one size for everything.
         OptionId::Font => "Font",
         OptionId::FontSize => "Size",
@@ -80,7 +116,19 @@ pub(super) fn terminal_label(option: OptionId) -> &'static str {
         | OptionId::AgentsNotify
         | OptionId::Theme
         | OptionId::InterfaceSize
-        | OptionId::TableList => label(option),
+        | OptionId::TableList
+        | OptionId::NewTab
+        | OptionId::RowLimit
+        | OptionId::Timeout
+        | OptionId::RunScope
+        | OptionId::EditorKeys
+        | OptionId::Indent
+        | OptionId::Autocomplete
+        | OptionId::LineNumbers
+        | OptionId::HighlightLine
+        | OptionId::Wrap
+        | OptionId::Keywords
+        | OptionId::Commas => label(option),
     }
 }
 
@@ -103,7 +151,25 @@ pub(super) fn small_print(option: OptionId) -> Option<&'static str> {
         OptionId::EnvBar => Some("Production always uses the full tint"),
         OptionId::InterfaceSize => Some("Sidebar, tabs, buttons, inspector labels"),
         OptionId::Font => Some("Grid cells, SQL editor, JSON"),
-        OptionId::Timestamps
+        OptionId::NewTab => Some("The badge in each tab can still switch it"),
+        // Not the board's "Adds LIMIT to results you browse; exports get
+        // everything": the rows are capped as they come, and the editor's
+        // results have no export.
+        OptionId::RowLimit => Some("Caps the rows of results you browse"),
+        OptionId::ConfirmUnsafeWrites => Some("On every connection that allows writes"),
+        OptionId::Autocomplete => Some("Tables, views, columns and functions from the schema"),
+        OptionId::HistoryKeep => Some("Per connection, on this Mac; results are never stored"),
+        OptionId::Timeout
+        | OptionId::RunScope
+        | OptionId::EditorKeys
+        | OptionId::Indent
+        | OptionId::ClosePairs
+        | OptionId::LineNumbers
+        | OptionId::HighlightLine
+        | OptionId::Wrap
+        | OptionId::Keywords
+        | OptionId::Commas
+        | OptionId::Timestamps
         | OptionId::GroupDigits
         | OptionId::ConfirmCloseEdits
         | OptionId::AgentsOn
@@ -115,6 +181,15 @@ pub(super) fn small_print(option: OptionId) -> Option<&'static str> {
     }
 }
 
+/// What describes `option`'s controls to a screen reader: its small print,
+/// or what is written beside its control where it has none. English.
+pub(super) fn helper(option: OptionId) -> Option<&'static str> {
+    small_print(option).or(match option {
+        OptionId::RunScope => Some("A selection always wins"),
+        _ => None,
+    })
+}
+
 /// What is said of a data font the settings name that is neither bundled
 /// nor installed, `{name}` for its name: the default is drawn instead.
 /// English.
@@ -213,6 +288,11 @@ pub(super) fn is_on(value: &OptionValue) -> bool {
             | OptionValue::AgentsNotify(true)
             | OptionValue::Zebra(Some(true))
             | OptionValue::ColumnTypes(true)
+            | OptionValue::ConfirmUnsafeWrites(true)
+            | OptionValue::ClosePairs(true)
+            | OptionValue::HighlightLine(true)
+            | OptionValue::Wrap(true)
+            | OptionValue::LineNumbers(Some(LineNumbers::Absolute | LineNumbers::Relative))
     )
 }
 
@@ -228,6 +308,9 @@ pub(super) fn shown(app: &App) -> Settings {
     settings.group_digits = Some(app.settings.grouped(app.layout()));
     settings.zebra = Some(settings.zebra_in(app.look.terminal));
     settings.font_size = Some(settings.font_size_in(app.look.terminal));
+    settings.editor_keys = Some(settings.editor_keys_in(app.look.terminal));
+    settings.line_numbers = Some(settings.line_numbers_in(app.look.terminal));
+    settings.keywords = Some(settings.keywords_in(app.look.terminal));
     settings.theme = match (app.look.terminal, settings.theme.mode()) {
         (true, _) if settings.theme.pin().is_some() => settings.theme,
         (true, _) => Theme::Omarchy,
@@ -273,14 +356,26 @@ pub(super) enum Said {
     /// A value as the app shows one, the same in every language: put into
     /// another, it would say something the app does not show.
     Sample(&'static str),
+    /// A command's first key, as the keymap writes it for the look.
+    Key(Command),
 }
 
 impl Said {
     /// What a screen reader and the tests call the choice.
-    pub(super) fn name(self, locale: Locale) -> Cow<'static, str> {
+    pub(super) fn name(
+        self,
+        ctx: &egui::Context,
+        look: &crate::theme::Look,
+        locale: Locale,
+    ) -> Cow<'static, str> {
         match self {
             Self::Word(word) => gettext(locale, word),
             Self::Sample(sample) => Cow::Borrowed(sample),
+            Self::Key(command) => Cow::Owned(
+                crate::ui::keys::written(ctx, look, command)
+                    .as_str()
+                    .to_owned(),
+            ),
         }
     }
 }
@@ -296,7 +391,7 @@ pub(super) fn choices(
     named: Option<&str>,
     launches: &[Launch],
 ) -> Vec<(Said, OptionValue)> {
-    use Said::{Sample, Word};
+    use Said::{Key, Sample, Word};
     match option {
         OptionId::OnLaunch => vec![
             (Word("fewer"), settings.stepped_launch(launches, false)),
@@ -397,6 +492,88 @@ pub(super) fn choices(
             (Word("tint + label"), OptionValue::EnvBar(EnvBar::Tint)),
             (Word("label only"), OptionValue::EnvBar(EnvBar::Label)),
         ],
+        OptionId::NewTab => vec![
+            (Word("read-only"), OptionValue::NewTab(NewTab::ReadOnly)),
+            (
+                Word("follow connection"),
+                OptionValue::NewTab(NewTab::Connection),
+            ),
+        ],
+        OptionId::RowLimit | OptionId::Timeout | OptionId::Indent | OptionId::HistoryKeep => {
+            vec![
+                (Word("fewer"), settings.stepped(option, false)),
+                (Word("more"), settings.stepped(option, true)),
+            ]
+        }
+        OptionId::RunScope => vec![
+            (
+                Word("statement"),
+                OptionValue::RunScope(RunScope::Statement),
+            ),
+            (Word("whole tab"), OptionValue::RunScope(RunScope::All)),
+        ],
+        OptionId::ConfirmUnsafeWrites => vec![
+            (Word("off"), OptionValue::ConfirmUnsafeWrites(false)),
+            (Word("on"), OptionValue::ConfirmUnsafeWrites(true)),
+        ],
+        // As the terminal look's board draws them: vim first.
+        OptionId::EditorKeys => vec![
+            (Word("vim"), OptionValue::EditorKeys(Some(EditorKeys::Vim))),
+            (
+                Word("standard"),
+                OptionValue::EditorKeys(Some(EditorKeys::Standard)),
+            ),
+        ],
+        OptionId::Autocomplete => vec![
+            (
+                Word("as you type"),
+                OptionValue::Autocomplete(Autocomplete::Typing),
+            ),
+            (
+                Key(Command::Complete),
+                OptionValue::Autocomplete(Autocomplete::Manual),
+            ),
+            (Word("off"), OptionValue::Autocomplete(Autocomplete::Off)),
+        ],
+        OptionId::ClosePairs => vec![
+            (Word("off"), OptionValue::ClosePairs(false)),
+            (Word("on"), OptionValue::ClosePairs(true)),
+        ],
+        OptionId::LineNumbers => LineNumbers::TERMINAL
+            .into_iter()
+            .map(|numbers| {
+                let word = match numbers {
+                    LineNumbers::Relative => "relative",
+                    LineNumbers::Absolute => "absolute",
+                    LineNumbers::Off => "off",
+                };
+                (Word(word), OptionValue::LineNumbers(Some(numbers)))
+            })
+            .collect(),
+        OptionId::HighlightLine => vec![
+            (Word("off"), OptionValue::HighlightLine(false)),
+            (Word("on"), OptionValue::HighlightLine(true)),
+        ],
+        OptionId::Wrap => vec![
+            (Word("off"), OptionValue::Wrap(false)),
+            (Word("on"), OptionValue::Wrap(true)),
+        ],
+        // The case is the choice: the look does not lower-case it.
+        OptionId::Keywords => vec![
+            (
+                Sample("UPPER"),
+                OptionValue::Keywords(Some(KeywordCase::Upper)),
+            ),
+            (
+                Sample("lower"),
+                OptionValue::Keywords(Some(KeywordCase::Lower)),
+            ),
+            (Word("keep"), OptionValue::Keywords(Some(KeywordCase::Keep))),
+        ],
+        OptionId::Commas => vec![
+            (Word("trailing"), OptionValue::Commas(Commas::Trailing)),
+            (Word("leading"), OptionValue::Commas(Commas::Leading)),
+        ],
     }
 }
 
```

```diff
diff --git a/src/ui/settings/sheet.rs b/src/ui/settings/sheet.rs
--- a/src/ui/settings/sheet.rs
+++ b/src/ui/settings/sheet.rs
@@ -8,20 +8,23 @@ use egui::{CornerRadius, Rect, Sense, Stroke, Ui, WidgetInfo, WidgetType, pos2,
 
 use crate::app::App;
 use crate::i18n::{Locale, gettext, ngettext};
+use crate::keymap::Command;
 use crate::model::{Action, SettingsDialog, SettingsPane};
 use crate::settings::{
-    EnvBar, FontSizes, Idle, InterfaceSize, Launch, Mode, OptionId, OptionValue, RowHeight,
-    Settings, TableList, ThemeChoice, Timestamps, Zone,
+    Autocomplete, Commas, EditorKeys, EnvBar, FontSizes, Idle, Indent, InterfaceSize, KeywordCase,
+    Launch, LineNumbers, Mode, NewTab, OptionId, OptionValue, RowHeight, RunScope, Settings,
+    TableList, ThemeChoice, Timestamps, Zone,
 };
 use crate::theme::{DataFace, Look, Palette, Typeface};
 use crate::typography::{Laid, Text, TextRole};
 use crate::ui::focus::{self, Ring};
 use crate::ui::format::group_digits;
 use crate::ui::menu;
+use crate::ui::sql_editor;
 use crate::ui::value_tags::slot_colors;
 use crate::ui::widgets::{self, Segment};
 
-use super::{label, path_shown, sample_timestamp, small_print, zone_text};
+use super::{label, path_shown, said, sample_timestamp, small_print, zone_text};
 
 /// The sheet at its widest and at its tallest, and what it leaves of the
 /// window at each side.
@@ -219,6 +222,23 @@ pub(super) fn show(
                     &skin,
                     actions,
                 ),
+                // The title alone: the board's line under it speaks of the
+                // history, which is not built yet.
+                SettingsPane::Editor => content(
+                    &mut pane,
+                    ("Editor", None),
+                    (
+                        &[
+                            ("Running queries", &OptionId::RUNNING),
+                            ("Typing", &OptionId::TYPING),
+                        ],
+                        false,
+                    ),
+                    &super::shown(app),
+                    limit,
+                    &skin,
+                    actions,
+                ),
                 SettingsPane::Agents => {
                     super::agents::content(&mut pane, app, dialog.agent, limit, &skin, actions);
                 }
@@ -608,7 +628,7 @@ pub(super) fn row(
     let name = Text::one(
         look,
         TextRole::UiBody,
-        &skin.say(label(option)),
+        &said(ui.ctx(), look, skin.locale, label(option)),
         palette.text,
     )
     .wrap(column)
@@ -624,9 +644,13 @@ pub(super) fn row(
         }
         None => small_print(option).map(|print| (skin.say(print), palette.dim)),
     };
-    // What the row says under its label is what its controls are for: a
-    // screen reader reads it as their description.
-    let description = print.as_ref().map(|(print, _)| print.clone());
+    // What the row says under its label, or beside its control where it
+    // says nothing under it, is what its controls are for: a screen reader
+    // reads it as their description.
+    let description = print
+        .as_ref()
+        .map(|(print, _)| print.clone())
+        .or_else(|| super::helper(option).map(|words| skin.say(words)));
     let print = print.map(|(print, color)| {
         Text::one(look, TextRole::Secondary, &print, color)
             .wrap(column)
@@ -714,6 +738,19 @@ fn control_height(option: OptionId, look: &Look) -> f32 {
         OptionId::RowHeight | OptionId::TableList | OptionId::EnvBar => SEGMENT + 4.0,
         OptionId::InterfaceSize => SEGMENT + 4.0,
         OptionId::Font | OptionId::FontSize => look.control_height,
+        OptionId::NewTab
+        | OptionId::RunScope
+        | OptionId::EditorKeys
+        | OptionId::Indent
+        | OptionId::Autocomplete
+        | OptionId::Keywords
+        | OptionId::Commas => SEGMENT + 4.0,
+        OptionId::RowLimit | OptionId::Timeout | OptionId::HistoryKeep => look.control_height,
+        OptionId::ConfirmUnsafeWrites
+        | OptionId::ClosePairs
+        | OptionId::LineNumbers
+        | OptionId::HighlightLine
+        | OptionId::Wrap => SWITCH,
         OptionId::RestoreTabs
         | OptionId::ValueTags
         | OptionId::ProductionReadOnly
@@ -739,8 +776,151 @@ fn control(
     actions: &mut Vec<Action>,
 ) {
     let Skin { look, palette, .. } = *skin;
-    let name = skin.say(label(option));
+    let name = said(ui.ctx(), look, skin.locale, label(option));
+    // Two or three words of segments, picked into a value.
+    let mut segmented = |words: &[&'static str], selected: usize| {
+        let words: Vec<String> = words.iter().map(|word| skin.say(word)).collect();
+        segments(ui, (&name, description), &words, selected, skin)
+    };
     match option {
+        OptionId::NewTab => {
+            let selected = usize::from(settings.new_tab == NewTab::Connection);
+            let words = ["Read-only transaction", "Follow connection"];
+            if let Some(picked) = segmented(&words, selected) {
+                let choice = [NewTab::ReadOnly, NewTab::Connection][picked];
+                actions.push(Action::SetOption(OptionValue::NewTab(choice)));
+            }
+        }
+        OptionId::RunScope => {
+            let selected = usize::from(settings.run_scope == RunScope::All);
+            if let Some(picked) = segmented(&["Statement at cursor", "Whole tab"], selected) {
+                let scope = [RunScope::Statement, RunScope::All][picked];
+                actions.push(Action::SetOption(OptionValue::RunScope(scope)));
+            }
+        }
+        OptionId::EditorKeys => {
+            let keys = settings.editor_keys_in(look.terminal);
+            let selected = usize::from(keys == EditorKeys::Vim);
+            if let Some(picked) = segmented(&["Standard", "Vim"], selected) {
+                let keys = [EditorKeys::Standard, EditorKeys::Vim][picked];
+                actions.push(Action::SetOption(OptionValue::EditorKeys(Some(keys))));
+            }
+        }
+        OptionId::Indent => {
+            let selected = Indent::ALL
+                .iter()
+                .position(|indent| *indent == settings.indent)
+                .unwrap_or(0);
+            if let Some(picked) = segmented(&["2 spaces", "4 spaces", "Tab"], selected) {
+                actions.push(Action::SetOption(OptionValue::Indent(Indent::ALL[picked])));
+            }
+        }
+        OptionId::Autocomplete => {
+            // The middle one names the key that asks for a list.
+            let complete = crate::ui::keys::written(ui.ctx(), look, Command::Complete);
+            let manual = skin.say("On {key}").replace("{key}", complete.as_str());
+            let words = [skin.say("As you type"), manual, skin.say("Off")];
+            let selected = Autocomplete::ALL
+                .iter()
+                .position(|choice| *choice == settings.autocomplete)
+                .unwrap_or(0);
+            if let Some(picked) = segments(ui, (&name, description), &words, selected, skin) {
+                let choice = Autocomplete::ALL[picked];
+                actions.push(Action::SetOption(OptionValue::Autocomplete(choice)));
+            }
+        }
+        OptionId::Keywords => {
+            let case = settings.keywords_in(look.terminal);
+            let selected = KeywordCase::ALL
+                .iter()
+                .position(|choice| *choice == case)
+                .unwrap_or(0);
+            if let Some(picked) = segmented(&["UPPERCASE", "lowercase", "Keep"], selected) {
+                let case = KeywordCase::ALL[picked];
+                actions.push(Action::SetOption(OptionValue::Keywords(Some(case))));
+            }
+        }
+        OptionId::Commas => {
+            let selected = usize::from(settings.commas == Commas::Leading);
+            if let Some(picked) = segmented(&["Trailing", "Leading"], selected) {
+                let commas = [Commas::Trailing, Commas::Leading][picked];
+                actions.push(Action::SetOption(OptionValue::Commas(commas)));
+            }
+        }
+        OptionId::RowLimit => {
+            let shown = sql_editor::limit_words(settings.sql_limit, look, skin.locale);
+            let choices = Settings::sql_limits_with(settings.sql_limit)
+                .into_iter()
+                .map(|limit| {
+                    let text = sql_editor::limit_words(limit, look, skin.locale);
+                    (
+                        text,
+                        limit == settings.sql_limit,
+                        OptionValue::RowLimit(limit),
+                    )
+                });
+            choice_menu(
+                ui,
+                ("row-limit", &name, description),
+                &shown,
+                choices,
+                skin,
+                actions,
+            );
+        }
+        OptionId::Timeout => {
+            let words = |secs: Option<u32>| match secs {
+                Some(secs) => sql_editor::time_words(secs, false, skin.locale),
+                None => skin.say("Off"),
+            };
+            let current = settings.sql_timeout_secs;
+            let choices = Settings::sql_timeouts_with(current)
+                .into_iter()
+                .map(|secs| (words(secs), secs == current, OptionValue::Timeout(secs)));
+            let shown = words(current);
+            choice_menu(
+                ui,
+                ("timeout", &name, description),
+                &shown,
+                choices,
+                skin,
+                actions,
+            );
+        }
+        OptionId::HistoryKeep => {
+            let words = |keep: u32| match keep {
+                0 => skin.say("Don't keep"),
+                keep => skin
+                    .say("Last {n} queries")
+                    .replace("{n}", &group_digits(u64::from(keep))),
+            };
+            let current = settings.history_keep;
+            let choices = Settings::history_keeps_with(current)
+                .into_iter()
+                .map(|keep| (words(keep), keep == current, OptionValue::HistoryKeep(keep)));
+            let shown = words(current);
+            choice_menu(
+                ui,
+                ("history", &name, description),
+                &shown,
+                choices,
+                skin,
+                actions,
+            );
+        }
+        // On is a number on each line: as they are, or absolute where
+        // they were off. A stored `relative` shows on and stays.
+        OptionId::LineNumbers => {
+            let numbers = settings.line_numbers_in(look.terminal);
+            if widgets::toggle(ui, numbers.shown(), &name, description, palette).clicked() {
+                let to = if numbers.shown() {
+                    LineNumbers::Off
+                } else {
+                    LineNumbers::Absolute
+                };
+                actions.push(Action::SetOption(OptionValue::LineNumbers(Some(to))));
+            }
+        }
         OptionId::PageSize => {
             let written = |size: u32| group_digits(u64::from(size));
             let size = written(settings.page_size);
@@ -876,7 +1056,11 @@ fn control(
         | OptionId::AgentsOn
         | OptionId::AgentsNotify
         | OptionId::Zebra
-        | OptionId::ColumnTypes => {
+        | OptionId::ColumnTypes
+        | OptionId::ConfirmUnsafeWrites
+        | OptionId::ClosePairs
+        | OptionId::HighlightLine
+        | OptionId::Wrap => {
             let on = super::is_on(&settings.value(option));
             if widgets::toggle(ui, on, &name, description, palette).clicked()
                 && let Some(flipped) = settings.flipped(option)
@@ -1216,6 +1400,12 @@ fn hint(ui: &Ui, at: egui::Pos2, right: f32, option: OptionId, settings: &Settin
             palette.dim,
         ),
         OptionId::ValueTags => return tags(ui, at, right, settings.value_tags, skin),
+        OptionId::RunScope => Text::one(
+            look,
+            TextRole::Secondary,
+            &skin.say("A selection always wins"),
+            palette.dim,
+        ),
         // What the Appearance pane's rows say is their small print, and its
         // tiles have their own line.
         OptionId::Theme
@@ -1234,7 +1424,21 @@ fn hint(ui: &Ui, at: egui::Pos2, right: f32, option: OptionId, settings: &Settin
         | OptionId::Zebra
         | OptionId::ColumnTypes
         | OptionId::TableList
-        | OptionId::EnvBar => return,
+        | OptionId::EnvBar
+        | OptionId::NewTab
+        | OptionId::RowLimit
+        | OptionId::Timeout
+        | OptionId::ConfirmUnsafeWrites
+        | OptionId::EditorKeys
+        | OptionId::Indent
+        | OptionId::Autocomplete
+        | OptionId::ClosePairs
+        | OptionId::LineNumbers
+        | OptionId::HighlightLine
+        | OptionId::Wrap
+        | OptionId::Keywords
+        | OptionId::Commas
+        | OptionId::HistoryKeep => return,
     };
     let text = text.layout(ui.ctx());
     if at.x + text.width() <= right {
@@ -1242,6 +1446,40 @@ fn hint(ui: &Ui, at: egui::Pos2, right: f32, option: OptionId, settings: &Settin
     }
 }
 
+/// A menu of `choices` (what each says, whether it is the one set, and the
+/// value it sets) named `name`, `shown` closed, with `description` for
+/// screen readers. `id` tells it from the pane's other menus.
+fn choice_menu(
+    ui: &mut Ui,
+    (id, name, description): (&str, &str, Option<&str>),
+    shown: &str,
+    choices: impl Iterator<Item = (String, bool, OptionValue)>,
+    skin: &Skin,
+    actions: &mut Vec<Action>,
+) {
+    let Skin { look, palette, .. } = *skin;
+    let dropdown = menu::Dropdown::new(id, shown, MENU).description(description);
+    let menu = dropdown.show(ui, look, palette, |ui| {
+        for (text, chosen, value) in choices {
+            let entry_name = format!("{name} {text}");
+            let entry = menu::Item::choice(&text, chosen)
+                .name(&entry_name)
+                .show(ui, look, palette);
+            if entry.clicked() {
+                actions.push(Action::SetOption(value));
+                // egui closes the menu on a pointer's click; a pick by key
+                // or by a screen reader closes it here.
+                ui.close();
+            }
+        }
+    });
+    menu.response.widget_info(|| {
+        let mut info = WidgetInfo::labeled(WidgetType::ComboBox, true, name);
+        info.current_text_value = Some(shown.to_owned());
+        info
+    });
+}
+
 /// Paints `text` from `x` on, centred on `y`, and names it for screen
 /// readers.
 fn write(ui: &Ui, x: f32, y: f32, text: &Laid) {
@@ -1324,6 +1562,7 @@ fn footer_height(ui: &Ui, ignored: Option<&Laid>, skin: &Skin) -> f32 {
 fn reset_question(pane: SettingsPane) -> &'static str {
     match pane {
         SettingsPane::General => "Reset all General settings to their defaults?",
+        SettingsPane::Editor => "Reset all Editor settings to their defaults?",
         SettingsPane::Appearance | SettingsPane::Agents => "Reset every option on this tab?",
     }
 }
```

```diff
diff --git a/src/ui/settings/terminal.rs b/src/ui/settings/terminal.rs
--- a/src/ui/settings/terminal.rs
+++ b/src/ui/settings/terminal.rs
@@ -18,7 +18,7 @@ use crate::ui::focus::{self, Ring};
 use crate::ui::value_tags::terminal_slots;
 use crate::ui::widgets::{self, ButtonSpec};
 
-use super::{Said, choices, file_pane, opened, sample_timestamp, small_print, terminal_label};
+use super::{Said, choices, file_pane, opened, sample_timestamp, terminal_label};
 
 /// The header's and the footer's heights, each without its rule.
 const HEADER: f32 = 40.0;
@@ -120,12 +120,22 @@ impl Skin<'_> {
         self.look.label(&gettext(self.locale, text))
     }
 
+    /// A row's label as the screen writes it: as [`Self::say`] has it,
+    /// with the run key in it where it names one.
+    fn label(&self, ctx: &egui::Context, text: &'static str) -> String {
+        self.look
+            .label(&super::said(ctx, self.look, self.locale, text))
+    }
+
     /// A choice as the screen writes it: a word as [`Self::say`] has it, a
-    /// sample as it is.
-    fn written(&self, choice: Said) -> String {
+    /// sample as it is, a key as the keymap writes it.
+    fn written(&self, ctx: &egui::Context, choice: Said) -> String {
         match choice {
             Said::Word(word) => self.say(word),
             Said::Sample(sample) => sample.to_owned(),
+            Said::Key(command) => crate::ui::keys::written(ctx, self.look, command)
+                .as_str()
+                .to_owned(),
         }
     }
 }
@@ -210,6 +220,7 @@ pub(super) fn show(
                 .show(&mut list, |ui| match pane {
                     SettingsPane::General => rows(ui, &settings, row, &skin, actions),
                     SettingsPane::Appearance => appearance_rows(ui, &settings, row, &skin, actions),
+                    SettingsPane::Editor => editor_rows(ui, &settings, row, &skin, actions),
                     SettingsPane::Agents => agent_rows(ui, app, &settings, row, &skin, actions),
                 });
             if let Some(beside) = beside {
@@ -448,6 +459,40 @@ fn appearance_rows(
     ui.add_space(4.0);
 }
 
+/// The Editor pane's options, a row each, under the headings of their
+/// groups, as far as they are built. Their columns line up across the
+/// groups.
+fn editor_rows(
+    ui: &mut Ui,
+    settings: &Settings,
+    cursor: usize,
+    skin: &Skin,
+    actions: &mut Vec<Action>,
+) {
+    let room = ui.available_width() - 2.0 * ROW_SIDE;
+    let shown: Vec<OptionId> = [&OptionId::RUNNING[..], &OptionId::TERMINAL_TYPING[..]].concat();
+    let columns = columns(ui, room, widths(ui, &shown, settings, skin), skin);
+    let mut index = 0;
+    for (heading_text, top, options) in [
+        ("Running queries", 8.0, &OptionId::RUNNING[..]),
+        ("Typing", 12.0, &OptionId::TERMINAL_TYPING[..]),
+    ] {
+        heading(ui, heading_text, top, skin);
+        for option in options.iter().copied() {
+            option_row(
+                ui,
+                (index, option),
+                index == cursor,
+                columns,
+                settings,
+                skin,
+                actions,
+            );
+            index += 1;
+        }
+    }
+}
+
 /// The colours the theme gave, eight of them as the board has them, and
 /// where they were read from. Nothing to change: the cursor passes it.
 fn palette_row(
@@ -884,7 +929,12 @@ fn widths(ui: &Ui, options: &[OptionId], settings: &Settings, skin: &Skin) -> (f
     let width = |text: &str| role.width(ui.ctx(), skin.look.faces, text);
     let labels = options
         .iter()
-        .map(|option| width(&format!("▌{}", skin.say(terminal_label(*option)))))
+        .map(|option| {
+            width(&format!(
+                "▌{}",
+                skin.label(ui.ctx(), terminal_label(*option))
+            ))
+        })
         .fold(0.0, f32::max);
     let values = options
         .iter()
@@ -942,7 +992,11 @@ fn value_width(ui: &Ui, option: OptionId, settings: &Settings, skin: &Skin) -> f
         | OptionId::Tz
         | OptionId::IdleDisconnect
         | OptionId::OnLaunch
-        | OptionId::FontSize => width(&format!("‹ {} ›", stepped_text(option, settings, skin))),
+        | OptionId::FontSize
+        | OptionId::RowLimit
+        | OptionId::Timeout
+        | OptionId::Indent
+        | OptionId::HistoryKeep => width(&format!("‹ {} ›", stepped_text(option, settings, skin))),
         OptionId::Font => width(&font_value(settings, skin)),
         OptionId::Timestamps
         | OptionId::GroupDigits
@@ -950,7 +1004,14 @@ fn value_width(ui: &Ui, option: OptionId, settings: &Settings, skin: &Skin) -> f
         | OptionId::InterfaceSize
         | OptionId::RowHeight
         | OptionId::TableList
-        | OptionId::EnvBar => {
+        | OptionId::EnvBar
+        | OptionId::NewTab
+        | OptionId::RunScope
+        | OptionId::EditorKeys
+        | OptionId::Autocomplete
+        | OptionId::LineNumbers
+        | OptionId::Keywords
+        | OptionId::Commas => {
             let words = choices(option, settings, skin.named, skin.launches);
             let between = width(" ") * words.len().saturating_sub(1) as f32;
             let segments = words.into_iter().map(|(word, _)| segment(ui, word, skin));
@@ -963,7 +1024,11 @@ fn value_width(ui: &Ui, option: OptionId, settings: &Settings, skin: &Skin) -> f
         | OptionId::AgentsOn
         | OptionId::AgentsNotify
         | OptionId::Zebra
-        | OptionId::ColumnTypes => width("[x]"),
+        | OptionId::ColumnTypes
+        | OptionId::ConfirmUnsafeWrites
+        | OptionId::ClosePairs
+        | OptionId::HighlightLine
+        | OptionId::Wrap => width("[x]"),
     }
 }
 
@@ -987,6 +1052,25 @@ fn stepped_text(option: OptionId, settings: &Settings, skin: &Skin) -> String {
             (zone, system) => super::zone_text(skin.locale, zone, system),
         }),
         OptionId::FontSize => settings.font_size_in(true).text(),
+        OptionId::RowLimit => {
+            crate::ui::sql_editor::limit_words(settings.sql_limit, skin.look, skin.locale)
+        }
+        // As the file writes it: `30s`, `1m`, `off`.
+        OptionId::Timeout => match settings.sql_timeout_secs {
+            Some(secs) => crate::ui::sql_editor::time_words(secs, true, skin.locale),
+            None => skin.say("Off"),
+        },
+        OptionId::Indent => skin.say(match settings.indent {
+            crate::settings::Indent::Two => "2 spaces",
+            crate::settings::Indent::Four => "4 spaces",
+            crate::settings::Indent::Tab => "Tab",
+        }),
+        OptionId::HistoryKeep => match settings.history_keep {
+            0 => skin.say("Don't keep"),
+            keep => skin
+                .say("{n} per connection")
+                .replace("{n}", &keep.to_string()),
+        },
         // A connection by its name as it is saved, and where it points:
         // three of one name are told apart.
         OptionId::OnLaunch => {
@@ -1018,7 +1102,7 @@ fn stepped_text(option: OptionId, settings: &Settings, skin: &Skin) -> String {
 
 /// How wide the segment of the choice `word` is.
 fn segment(ui: &Ui, word: Said, skin: &Skin) -> f32 {
-    let text = skin.written(word);
+    let text = skin.written(ui.ctx(), word);
     TextRole::OBody.width(ui.ctx(), skin.look.faces, &text) + 2.0 * SEGMENT_PAD
 }
 
@@ -1041,7 +1125,7 @@ fn row_frame(
     let role = TextRole::OBody;
     let (row, _) = ui.allocate_exact_size(vec2(ui.available_width(), ROW), Sense::hover());
     let button = (index != usize::MAX).then(|| {
-        let said = gettext(locale, name);
+        let said = super::said(ui.ctx(), look, locale, name);
         let button = ButtonSpec::new(&said).hidden_at(ui, row);
         // A row of a list: a ring outside it would be cut where the list
         // ends.
@@ -1058,7 +1142,7 @@ fn row_frame(
     let right = (row.right() - ROW_SIDE).max(left);
     // The cursor's mark stands where a label starts, so the label under
     // the cursor is a cell further in than the others.
-    let text = skin.say(name);
+    let text = skin.label(ui.ctx(), name);
     let text = if on_cursor {
         Text::one(look, role, &format!("▌{text}"), palette.accent)
     } else {
@@ -1133,9 +1217,13 @@ fn option_row(
                 | OptionId::IdleDisconnect
                 | OptionId::OnLaunch
                 | OptionId::FontSize
+                | OptionId::RowLimit
+                | OptionId::Timeout
+                | OptionId::Indent
+                | OptionId::HistoryKeep
         )
     {
-        let name = gettext(locale, terminal_label(option));
+        let name = super::said(ui.ctx(), skin.look, locale, terminal_label(option));
         let value = stepped_text(option, settings, skin);
         button.widget_info(|| {
             let mut info = WidgetInfo::labeled(WidgetType::Button, true, &name);
@@ -1159,7 +1247,11 @@ fn option_row(
         | OptionId::Tz
         | OptionId::IdleDisconnect
         | OptionId::OnLaunch
-        | OptionId::FontSize => stepper(ui, cell, option, settings, skin, actions),
+        | OptionId::FontSize
+        | OptionId::RowLimit
+        | OptionId::Timeout
+        | OptionId::Indent
+        | OptionId::HistoryKeep => stepper(ui, cell, option, settings, skin, actions),
         // Nothing to step to: a name is set in the file.
         OptionId::Font => {
             let value = Text::one(
@@ -1176,7 +1268,14 @@ fn option_row(
         | OptionId::InterfaceSize
         | OptionId::RowHeight
         | OptionId::TableList
-        | OptionId::EnvBar => {
+        | OptionId::EnvBar
+        | OptionId::NewTab
+        | OptionId::RunScope
+        | OptionId::EditorKeys
+        | OptionId::Autocomplete
+        | OptionId::LineNumbers
+        | OptionId::Keywords
+        | OptionId::Commas => {
             segments(ui, cell, option, settings, skin, actions);
         }
         OptionId::RestoreTabs
@@ -1186,7 +1285,11 @@ fn option_row(
         | OptionId::AgentsOn
         | OptionId::AgentsNotify
         | OptionId::Zebra
-        | OptionId::ColumnTypes => {
+        | OptionId::ColumnTypes
+        | OptionId::ConfirmUnsafeWrites
+        | OptionId::ClosePairs
+        | OptionId::HighlightLine
+        | OptionId::Wrap => {
             check(ui, cell, option, settings, skin, actions);
         }
     }
@@ -1377,6 +1480,27 @@ fn hint(ui: &Ui, option: OptionId, settings: &Settings, skin: &Skin) -> Text {
             .add(role, &format!("{} ", skin.say("default;")), palette.dim)
             .add(role, &key(ui, look, Command::ToggleTree), palette.accent)
             .add(role, &format!(" {}", skin.say("toggles")), palette.dim),
+        // Not the board's "ctrl+w toggles per tab": ctrl+w closes the tab.
+        // The commands that switch it, from the keymap.
+        OptionId::NewTab => Text::new(look)
+            .add(role, &key(ui, look, Command::ReadOnlyTab), palette.accent)
+            .add(role, " / ", palette.dim)
+            .add(role, &key(ui, look, Command::ReadWriteTab), palette.accent)
+            .add(role, &format!(" {}", skin.say("per tab")), palette.dim),
+        OptionId::RowLimit => dim(&skin.say("caps results you browse")),
+        OptionId::RunScope => dim(&skin.say("a selection always wins")),
+        OptionId::ConfirmUnsafeWrites => dim(&skin.say("connections that allow writes")),
+        OptionId::EditorKeys => dim(&skin.say("-- INSERT -- shown in the status line")),
+        OptionId::Autocomplete => dim(&skin.say("schema objects")),
+        OptionId::HistoryKeep => dim(&skin.say("text only, no results")),
+        OptionId::Timeout
+        | OptionId::Indent
+        | OptionId::ClosePairs
+        | OptionId::LineNumbers
+        | OptionId::HighlightLine
+        | OptionId::Wrap
+        | OptionId::Keywords
+        | OptionId::Commas => Text::new(look),
         // Production as the bar's label writes it, in its own case.
         OptionId::EnvBar => Text::new(look)
             .add(
@@ -1545,8 +1669,13 @@ fn choice(
 ) {
     let name = format!(
         "{}: {}",
-        gettext(skin.locale, terminal_label(value.option())),
-        word.name(skin.locale)
+        super::said(
+            ui.ctx(),
+            skin.look,
+            skin.locale,
+            terminal_label(value.option())
+        ),
+        word.name(ui.ctx(), skin.look, skin.locale)
     );
     let button = ButtonSpec::new(&name);
     let button = match press {
@@ -1572,7 +1701,7 @@ fn choice(
 /// description: what a screen reader reads after its name, as the macOS
 /// look's controls have it. In the case its name has, not the look's.
 fn describe(ui: &Ui, control: &egui::Response, option: OptionId, skin: &Skin) {
-    if let Some(print) = small_print(option) {
+    if let Some(print) = super::helper(option) {
         let said = gettext(skin.locale, print);
         ui.ctx().accesskit_node_builder(control.id, |node| {
             node.set_description(said.as_ref());
@@ -1590,7 +1719,7 @@ fn radio_group(ui: &mut Ui, cell: Cell, option: OptionId, skin: &Skin) -> Ui {
         .id_salt(("values", label))
         .max_rect(cell.row);
     let group = ui.new_child(builder);
-    let name = gettext(skin.locale, label);
+    let name = super::said(ui.ctx(), skin.look, skin.locale, label);
     group
         .ctx()
         .accesskit_node_builder(group.unique_id(), |node| {
@@ -1619,7 +1748,7 @@ fn segments(
     let mut group = radio_group(ui, cell, option, skin);
     let mut left = cell.left;
     for (word, value) in choices(option, settings, skin.named, skin.launches) {
-        let text = skin.written(word);
+        let text = skin.written(ui.ctx(), word);
         let width = segment(ui, word, skin);
         let place = Rect::from_min_size(
             pos2(left, cell.row.center().y - line / 2.0),
```

- [ ] **Step 4: Run the tests to see them pass**

Run: `cargo test --locked --lib -- settings ui::editor_settings_tests ui::agent_settings_tests ui::settings_access_tests ui::tests`
Expected: PASS. Then the four checks above.

- [ ] **Step 5: Compare with the boards by eye**

Not a test, and nothing of it is committed. Append a throwaway `#[test] #[ignore]` scene to `src/shots.rs` that opens Settings › Editor in `Harness::for_shots(egui::vec2(1440.0, 1100.0), 1.0, true, Look::macos())` and in `Harness::for_shots(egui::vec2(1920.0, 1080.0), 1.0, true, Look::omarchy())` and shoots each. Run it with `cargo test --locked --features shots --lib shots::<name> -- --ignored`, read the PNGs in `target/shots/`, and set them beside the two "Settings, Editor" boards. Remove the scene before committing. Keep the PNGs local.

- [ ] **Step 6: Commit**

```bash
git add src/app.rs src/model.rs src/settings.rs src/ui/agent_settings_tests.rs src/ui/editor_settings_tests.rs src/ui/mod.rs src/ui/settings/file_pane.rs src/ui/settings/mod.rs src/ui/settings/sheet.rs src/ui/settings/terminal.rs src/ui/settings_access_tests.rs
git commit -m "Add Settings › Editor with the rows of the run, completion and the gutter in both looks"
```

---

### Task 4: What the rows do

**Files:**
- Modify: `src/app.rs`, `src/model.rs`, `src/ui/sql_text.rs`, `src/ui/sql_editor.rs`
- Modify (tests): `src/ui/editor_settings_tests.rs`, `src/app.rs`, `src/ui/sql_editor.rs`, `src/ui/mod.rs`

- [ ] **Step 1: Write the failing tests**

```diff
diff --git a/src/app.rs b/src/app.rs
--- a/src/app.rs
+++ b/src/app.rs
@@ -12288,8 +12288,11 @@ masked = ["orders.email"]
         assert_eq!(cancels_since(&harness, sent), loading);
     }
 
-    /// A SQL editor on a connection that takes writes.
+    /// A SQL editor on a connection that takes writes, where new tabs
+    /// follow the connection (`editor.run.new_tab = "connection"`): it
+    /// opens read-write.
     fn writable_sql(harness: &mut Harness) -> (ConnTabId, TabId) {
+        harness.app.settings.new_tab = crate::settings::NewTab::Connection;
         let tab = harness.connect_fake_as(false);
         harness.app.apply(Action::NewSqlTab(tab));
         let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
@@ -12327,6 +12330,7 @@ masked = ["orders.email"]
         // opened there still does once the session takes writes. Only an
         // editor opened from then on writes.
         let mut harness = Harness::new();
+        harness.app.settings.new_tab = crate::settings::NewTab::Connection;
         let (tab, id) = new_sql(&mut harness);
         assert_eq!(sql(&harness, tab, id).mode, RunMode::ReadOnly);
         harness.reconnect_fake_as(tab, false);
@@ -12337,6 +12341,7 @@ masked = ["orders.email"]
         // A writable connection to production: an editor there opens in
         // Read-only, as production's connections do.
         let mut harness = Harness::new();
+        harness.app.settings.new_tab = crate::settings::NewTab::Connection;
         let tab = harness.connect_fake_as(false);
         harness.app.workspace_mut(tab).unwrap().environment = Environment::Production;
         harness.app.apply(Action::NewSqlTab(tab));
@@ -12688,8 +12693,8 @@ masked = ["orders.email"]
         ));
         assert_eq!(harness.app.settings.sql_limit, 100);
         assert_eq!(harness.app.settings.sql_timeout_secs, None);
-        // The next editor starts from them; the menus of one editor leave
-        // the others as they are.
+        // The next editor starts from them; the menus of one editor set
+        // the settings, which every open editor follows.
         harness.app.apply(Action::NewSqlTab(tab));
         let second = harness.app.workspace(tab).unwrap().active_tab.unwrap();
         assert_eq!(sql(&harness, tab, second).limit, 100);
@@ -12699,11 +12704,12 @@ masked = ["orders.email"]
             sql_tab: second,
             secs: Some(60),
         });
-        assert_eq!(
-            sql(&harness, tab, second).timeout,
-            Some(Duration::from_secs(60))
-        );
-        assert_eq!(sql(&harness, tab, id).timeout, None);
+        for editor in [second, id] {
+            assert_eq!(
+                sql(&harness, tab, editor).timeout,
+                Some(Duration::from_secs(60))
+            );
+        }
         assert_eq!(harness.app.settings.sql_timeout_secs, Some(60));
     }
 
```

```diff
diff --git a/src/ui/editor_settings_tests.rs b/src/ui/editor_settings_tests.rs
--- a/src/ui/editor_settings_tests.rs
+++ b/src/ui/editor_settings_tests.rs
@@ -5,12 +5,13 @@
 use egui::accesskit::{Role, Toggled};
 use egui::{Key, Modifiers};
 
+use crate::backend::Command;
 use crate::keymap::Command as Keyed;
-use crate::model::{Action, SettingsPane};
+use crate::model::{Action, ConnTabId, RunMode, SettingsPane, SqlTab, TabId};
 use crate::settings::{
     Autocomplete, EditorKeys, KeywordCase, LineNumbers, NewTab, OptionValue, RunScope, Settings,
 };
-use crate::testing::Harness;
+use crate::testing::{Harness, bounds};
 use crate::theme::Look;
 
 use super::appearance_settings_tests::{settings_saves, shows};
@@ -19,6 +20,57 @@ fn looks() -> [Look; 2] {
     [Look::macos(), Look::omarchy()]
 }
 
+/// A connection that takes writes (`writable`) or not, in `look`, with a
+/// new SQL editor that has the keyboard and holds `text`.
+fn editor(look: Look, writable: bool, text: &str) -> (Harness, ConnTabId, TabId) {
+    let mut harness = Harness::new();
+    harness.set_look(look);
+    let tab = harness.connect_fake_as(!writable);
+    harness.press(Key::T, Modifiers::COMMAND);
+    if !text.is_empty() {
+        harness.frame(vec![egui::Event::Paste(text.into())]);
+    }
+    harness.settle();
+    let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
+    (harness, tab, id)
+}
+
+fn sql(harness: &Harness, tab: ConnTabId, id: TabId) -> &SqlTab {
+    harness.app.workspace(tab).unwrap().sql_tab(id).unwrap()
+}
+
+/// The mode a run of the editor would have now.
+fn run_mode(harness: &Harness, tab: ConnTabId, id: TabId) -> RunMode {
+    let workspace = harness.app.workspace(tab).unwrap();
+    workspace.run_mode(workspace.sql_tab(id).unwrap())
+}
+
+/// A new editor of `tab`, shown.
+fn new_editor(harness: &mut Harness, tab: ConnTabId) -> TabId {
+    harness.app.apply(Action::NewSqlTab(tab));
+    harness.settle();
+    harness.app.workspace(tab).unwrap().active_tab.unwrap()
+}
+
+/// The statements, limit and timeout of the last run sent.
+fn last_run(harness: &Harness) -> (Vec<String>, u32, Option<std::time::Duration>) {
+    let mut sent = harness.app.backend.sent.iter().rev();
+    sent.find_map(|command| match command {
+        Command::RunSql {
+            statements,
+            limit,
+            timeout,
+            ..
+        } => Some((
+            statements.iter().map(|s| s.text.clone()).collect(),
+            *limit,
+            *timeout,
+        )),
+        _ => None,
+    })
+    .expect("a run was sent")
+}
+
 /// Settings on its Editor pane.
 fn on_the_pane(harness: &mut Harness) {
     harness.app.apply(Action::ShowSettings);
@@ -28,6 +80,14 @@ fn on_the_pane(harness: &mut Harness) {
     harness.settle();
 }
 
+/// The settings changed by `change`, as an edit of the file changes them.
+fn change(harness: &mut Harness, change: impl FnOnce(&mut Settings)) {
+    let mut settings = harness.app.settings.clone();
+    change(&mut settings);
+    harness.app.apply_settings(settings);
+    harness.settle();
+}
+
 fn type_key(harness: &mut Harness, key: Key, text: &str) {
     super::tests::type_key(harness, key, text);
 }
@@ -130,6 +190,316 @@ fn a_value_an_editor_key_cannot_have_is_ignored_by_its_line() {
     );
 }
 
+#[test]
+fn a_new_tab_opens_read_only_unless_the_settings_follow_the_connection() {
+    for look in looks() {
+        // A dev connection that takes writes: read-only all the same.
+        let (mut harness, tab, id) = editor(look, true, "");
+        assert_eq!(
+            sql(&harness, tab, id).mode,
+            RunMode::ReadOnly,
+            "{}",
+            look.name
+        );
+        // The badge still switches it.
+        harness.app.apply(Action::SetSqlMode {
+            tab,
+            sql_tab: id,
+            mode: RunMode::ReadWrite,
+        });
+        assert_eq!(run_mode(&harness, tab, id), RunMode::ReadWrite);
+        // Follow connection: a new tab writes where its connection does,
+        // and the tabs open keep their mode.
+        harness
+            .app
+            .apply(Action::SetOption(OptionValue::NewTab(NewTab::Connection)));
+        assert_eq!(run_mode(&harness, tab, id), RunMode::ReadWrite);
+        let follows = new_editor(&mut harness, tab);
+        assert_eq!(run_mode(&harness, tab, follows), RunMode::ReadWrite);
+        harness
+            .app
+            .apply(Action::SetOption(OptionValue::NewTab(NewTab::ReadOnly)));
+        assert_eq!(run_mode(&harness, tab, follows), RunMode::ReadWrite);
+        let read = new_editor(&mut harness, tab);
+        assert_eq!(run_mode(&harness, tab, read), RunMode::ReadOnly);
+        // A read-only connection caps its tabs, whatever the setting.
+        let (mut harness, tab, _) = editor(look, false, "");
+        harness
+            .app
+            .apply(Action::SetOption(OptionValue::NewTab(NewTab::Connection)));
+        let capped = new_editor(&mut harness, tab);
+        assert_eq!(
+            run_mode(&harness, tab, capped),
+            RunMode::ReadOnly,
+            "{}",
+            look.name
+        );
+    }
+}
+
+#[test]
+fn every_open_tab_follows_the_limit_and_the_timeout() {
+    let (mut harness, tab, first) = editor(Look::macos(), false, "SELECT 1");
+    let second = new_editor(&mut harness, tab);
+    harness
+        .app
+        .apply(Action::SetOption(OptionValue::RowLimit(5_000)));
+    harness
+        .app
+        .apply(Action::SetOption(OptionValue::Timeout(Some(300))));
+    for id in [first, second] {
+        assert_eq!(sql(&harness, tab, id).limit, 5_000);
+        assert_eq!(
+            sql(&harness, tab, id).timeout,
+            Some(std::time::Duration::from_secs(300))
+        );
+    }
+    // No limit asks for every row; Off sets no timer.
+    harness
+        .app
+        .apply(Action::SetOption(OptionValue::RowLimit(0)));
+    harness
+        .app
+        .apply(Action::SetOption(OptionValue::Timeout(None)));
+    harness.app.apply(Action::ActivateTab { tab, id: first });
+    harness.settle();
+    harness.press(Key::Enter, Modifiers::COMMAND);
+    let (statements, limit, timeout) = last_run(&harness);
+    assert_eq!(statements, ["SELECT 1"]);
+    assert_eq!(limit, u32::MAX);
+    assert_eq!(timeout, None);
+    // The toolbar's menus offer the setting's values and say No limit.
+    harness.click("Limit");
+    assert!(harness.has("No limit"));
+    assert!(harness.has("Limit 5,000"));
+}
+
+#[test]
+fn the_run_key_runs_the_statement_the_whole_tab_or_the_selection() {
+    let text = "SELECT 1;\nSELECT 2;\nSELECT 3";
+    for look in looks() {
+        let (mut harness, _, _) = editor(look, false, text);
+        // The cursor is at the end, in the last statement.
+        harness.press(Key::Enter, Modifiers::COMMAND);
+        assert_eq!(last_run(&harness).0, ["SELECT 3"], "{}", look.name);
+        harness
+            .app
+            .apply(Action::SetOption(OptionValue::RunScope(RunScope::All)));
+        harness.settle();
+        harness.press(Key::Enter, Modifiers::COMMAND);
+        assert_eq!(
+            last_run(&harness).0,
+            ["SELECT 1", "SELECT 2", "SELECT 3"],
+            "{}",
+            look.name
+        );
+    }
+    // A selection wins either way: what it covers, exactly.
+    for look in looks() {
+        for scope in [RunScope::All, RunScope::Statement] {
+            let (mut harness, _, _) = editor(look, false, text);
+            harness
+                .app
+                .apply(Action::SetOption(OptionValue::RunScope(scope)));
+            harness.settle();
+            // From the end back to the `2`: "2;\nSELECT 3".
+            for _ in 0..11 {
+                harness.press(Key::ArrowLeft, Modifiers::SHIFT);
+            }
+            harness.press(Key::Enter, Modifiers::COMMAND);
+            assert_eq!(
+                last_run(&harness).0,
+                ["2", "SELECT 3"],
+                "{} {scope:?}",
+                look.name
+            );
+        }
+    }
+}
+
+#[test]
+fn the_run_button_says_what_it_runs() {
+    let (mut harness, _, _) = editor(Look::macos(), false, "SELECT 1;\nSELECT 2");
+    let hovered = |harness: &mut Harness| {
+        let tree = harness.settle();
+        let at = bounds(&tree, "Run", Role::Button).expect("the Run button");
+        let shown = super::tests::hover(harness, at.center());
+        harness.frame(vec![egui::Event::PointerGone]);
+        harness.settle();
+        shown
+    };
+    assert!(hovered(&mut harness).contains(&"Run the statement at the cursor".to_owned()));
+    harness
+        .app
+        .apply(Action::SetOption(OptionValue::RunScope(RunScope::All)));
+    assert!(hovered(&mut harness).contains(&"Run every statement".to_owned()));
+    // The editor still has the keyboard.
+    for _ in 0..3 {
+        harness.press(Key::ArrowLeft, Modifiers::SHIFT);
+    }
+    assert!(hovered(&mut harness).contains(&"Run the selection".to_owned()));
+}
+
+#[test]
+fn a_selection_keeps_its_statements_lines_and_splits_where_the_sql_does() {
+    let text = "SELECT 'a;b';\n-- one; two\nSELECT 2;\nSELECT 3";
+    // The lines of the statements of the last run sent.
+    let lines = |harness: &Harness| -> Vec<(String, usize)> {
+        let mut sent = harness.app.backend.sent.iter().rev();
+        sent.find_map(|command| match command {
+            Command::RunSql { statements, .. } => Some(
+                statements
+                    .iter()
+                    .map(|s| (s.text.clone(), s.first_line))
+                    .collect(),
+            ),
+            _ => None,
+        })
+        .expect("a run was sent")
+    };
+    // From the end back to the comment: each statement where the editor
+    // has it, so that an error marks the line it shows.
+    let (mut harness, _, _) = editor(Look::macos(), false, text);
+    for _ in 0.."-- one; two\nSELECT 2;\nSELECT 3".chars().count() {
+        harness.press(Key::ArrowLeft, Modifiers::SHIFT);
+    }
+    harness.press(Key::Enter, Modifiers::COMMAND);
+    assert_eq!(
+        lines(&harness),
+        [
+            ("-- one; two\nSELECT 2".to_owned(), 3),
+            ("SELECT 3".to_owned(), 4)
+        ]
+    );
+    // The whole tab splits on neither the string's `;` nor the comment's.
+    let (mut harness, _, _) = editor(Look::macos(), false, text);
+    harness
+        .app
+        .apply(Action::SetOption(OptionValue::RunScope(RunScope::All)));
+    harness.settle();
+    harness.press(Key::Enter, Modifiers::COMMAND);
+    let texts: Vec<String> = lines(&harness).into_iter().map(|(text, _)| text).collect();
+    assert_eq!(texts, ["SELECT 'a;b'", "-- one; two\nSELECT 2", "SELECT 3"]);
+    // Nor on PostgreSQL's dollar quote, which the run takes from the same
+    // splitting (the fixture's database is SQLite).
+    let quoted =
+        tabletist_db::sql::statements(tabletist_db::Dialect::Postgres, "SELECT $$x;y$$; SELECT 3");
+    assert_eq!(quoted.len(), 2);
+}
+
+#[test]
+fn autocomplete_opens_as_set_and_typing_asks_the_database_nothing() {
+    let typed = |harness: &mut Harness, text: &str| {
+        harness.frame(vec![egui::Event::Text(text.into())]);
+        harness.settle();
+    };
+    let open = |harness: &Harness, tab, id| sql(harness, tab, id).completion.is_some();
+    // As you type, with the schema loaded: a list, and nothing sent.
+    let (mut harness, tab, id) = editor(Look::macos(), false, "");
+    let sent = harness.app.backend.sent.len();
+    typed(&mut harness, "SELECT * FROM us");
+    assert!(open(&harness, tab, id));
+    assert_eq!(harness.app.backend.sent.len(), sent, "typing sent nothing");
+    // On the key alone: typing opens nothing, the key does, and the list
+    // goes on as the word is typed.
+    let (mut harness, tab, id) = editor(Look::macos(), false, "");
+    harness
+        .app
+        .apply(Action::SetOption(OptionValue::Autocomplete(
+            Autocomplete::Manual,
+        )));
+    typed(&mut harness, "SELECT * FROM us");
+    assert!(!open(&harness, tab, id));
+    harness.press(Key::Space, Modifiers::CTRL);
+    assert!(open(&harness, tab, id));
+    typed(&mut harness, "e");
+    assert!(open(&harness, tab, id));
+    // Off: neither.
+    harness
+        .app
+        .apply(Action::SetOption(OptionValue::Autocomplete(
+            Autocomplete::Off,
+        )));
+    harness.settle();
+    assert!(!open(&harness, tab, id), "turned off, the open list closes");
+    typed(&mut harness, "r");
+    harness.press(Key::Space, Modifiers::CTRL);
+    assert!(!open(&harness, tab, id));
+}
+
+/// The numbers the gutter painted, top to bottom.
+fn gutter(harness: &mut Harness) -> Vec<String> {
+    let tree = harness.settle();
+    let field = bounds(&tree, "SQL", Role::MultilineTextInput).expect("the editor");
+    let mut numbers: Vec<(f32, String)> = harness
+        .text_rects
+        .iter()
+        .filter(|(text, rect)| {
+            rect.right() <= field.left()
+                && rect.top() >= field.top()
+                && text.bytes().all(|byte| byte.is_ascii_digit())
+        })
+        .map(|(text, rect)| (rect.top(), text.clone()))
+        .collect();
+    numbers.sort_by(|a, b| a.0.total_cmp(&b.0));
+    numbers.into_iter().map(|(_, text)| text).collect()
+}
+
+#[test]
+fn the_gutter_numbers_the_lines_as_set_in_either_look() {
+    for look in looks() {
+        // The cursor is on the last of three lines.
+        let (mut harness, _, _) = editor(look, false, "SELECT 1\nFROM t\nWHERE x");
+        let own = if look.terminal {
+            ["2", "1", "3"]
+        } else {
+            ["1", "2", "3"]
+        };
+        assert_eq!(gutter(&mut harness), own, "{}", look.name);
+        change(&mut harness, |settings| {
+            settings.line_numbers = Some(LineNumbers::Relative)
+        });
+        assert_eq!(gutter(&mut harness), ["2", "1", "3"], "{}", look.name);
+        change(&mut harness, |settings| {
+            settings.line_numbers = Some(LineNumbers::Absolute)
+        });
+        assert_eq!(gutter(&mut harness), ["1", "2", "3"], "{}", look.name);
+        change(&mut harness, |settings| {
+            settings.line_numbers = Some(LineNumbers::Off)
+        });
+        assert_eq!(gutter(&mut harness), Vec::<String>::new(), "{}", look.name);
+    }
+}
+
+#[test]
+fn the_cursors_line_is_filled_unless_the_settings_turn_it_off() {
+    for look in looks() {
+        let (mut harness, _, _) = editor(look, false, "SELECT 1\nFROM t");
+        let palette = harness.app.palette;
+        let fill = if look.terminal {
+            palette.selection
+        } else {
+            palette.window.lerp_to_gamma(palette.selection, 0.45)
+        };
+        // The cursor's line, across the editor: wider than any other fill
+        // of that colour a fixture draws.
+        let field = bounds(&harness.settle(), "SQL", Role::MultilineTextInput).expect("the editor");
+        let filled = |harness: &mut Harness| {
+            harness.settle();
+            harness
+                .fills
+                .iter()
+                .any(|(rect, color)| *color == fill && rect.width() >= field.width())
+        };
+        assert!(filled(&mut harness), "{}", look.name);
+        change(&mut harness, |settings| settings.highlight_line = false);
+        assert!(!filled(&mut harness), "{}", look.name);
+        // The editor kept the keyboard: on again, the line is filled again.
+        change(&mut harness, |settings| settings.highlight_line = true);
+        assert!(filled(&mut harness), "{}", look.name);
+    }
+}
+
 #[test]
 fn the_sheet_sets_each_editor_option_with_a_click() {
     let look = Look::macos();
```

```diff
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -5982,10 +5982,11 @@ value_tags   = true
             // The switch is one stop, on the segment it has chosen: the
             // arrows choose inside it. The terminal's toolbar leads with
             // it, the others' meet it after Format.
+            // A new tab opens read-only: that segment is chosen.
             let order: &[&str] = if look.terminal {
-                &["Read-write", "Limit", "Timeout", "Run"]
+                &["Read-only", "Limit", "Timeout", "Run"]
             } else {
-                &["Format", "Read-write", "Limit", "Timeout"]
+                &["Format", "Read-only", "Limit", "Timeout"]
             };
             focus(&mut harness, order[0], egui::accesskit::Role::Button);
             let mut reached = vec![focused_name(&harness.settle())];
```

```diff
diff --git a/src/ui/sql_editor.rs b/src/ui/sql_editor.rs
--- a/src/ui/sql_editor.rs
+++ b/src/ui/sql_editor.rs
@@ -1266,10 +1266,12 @@ mod tests {
     use crate::testing::{Harness, bounds, done_outcome, node, write_outcome};
 
     /// A SQL editor on a connection that takes writes, drawn in `look`,
-    /// with `text` typed into it.
+    /// with `text` typed into it, opened as the connection has it
+    /// (`editor.run.new_tab = "connection"`): read-write.
     fn editor(look: Look, text: &str) -> (Harness, ConnTabId, TabId) {
         let mut harness = Harness::new();
         harness.set_look(look);
+        harness.app.settings.new_tab = crate::settings::NewTab::Connection;
         let tab = harness.connect_fake_as(false);
         harness.press(Key::T, Modifiers::COMMAND);
         harness.frame(vec![egui::Event::Paste(text.into())]);
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test --locked --lib -- ui::editor_settings_tests ui::sql_editor app::tests ui::tests`
Expected: FAIL. Ten tests fail: eight of `editor_settings_tests` (a new tab still opens read-write on a writable connection; the second tab keeps its old limit; Whole tab runs `["SELECT 3"]` and a selection the statement at the cursor; with the selection `[("-- one; two\nSELECT 2", 3)]` alone; typing opens a list on the key alone; the gutter numbers `["1", "2", "3"]` where relative is set; the cursor's line is filled with the highlight off; the Run button does not say "Run every statement"), `app::tests::the_limit_and_timeout_menus_change_the_next_run_and_the_settings` (the first tab keeps no timeout) and `ui::tests::tab_stops_once_on_the_switch_of_a_connection_that_takes_writes` (the switch's chosen segment is still "Read-write").

- [ ] **Step 3: Make the editor follow the settings**

```diff
diff --git a/src/app.rs b/src/app.rs
--- a/src/app.rs
+++ b/src/app.rs
@@ -1959,15 +1959,22 @@ impl App {
             }
             Action::RunSqlAgain { tab, sql_tab } => self.run_sql_again(tab, sql_tab),
             Action::SqlTyped { tab, sql_tab } => {
+                // Off, or on the key alone, typing opens no list. One open
+                // goes on as the word is typed (`refresh_completion`).
+                let opens = self.settings.autocomplete.as_you_type();
                 if let Some(sql) = self.sql_tab_mut(tab, sql_tab) {
+                    let open = sql.completion.is_some();
                     // A list asked for by hand in the same frame stays so.
-                    if sql.completion_wanted.is_none() {
+                    if sql.completion_wanted.is_none() && (opens || open) {
                         sql.completion_wanted = Some(Wanted::Typed);
                     }
                 }
             }
             Action::OpenCompletion { tab, sql_tab } => {
-                if let Some(sql) = self.sql_tab_mut(tab, sql_tab) {
+                let opens = self.settings.autocomplete.on_key();
+                if let Some(sql) = self.sql_tab_mut(tab, sql_tab)
+                    && opens
+                {
                     sql.completion_wanted = Some(Wanted::Manual);
                 }
             }
@@ -2002,6 +2009,9 @@ impl App {
                     sql.focus_editor = true;
                 }
             }
+            // The toolbar's menus set the setting, which every open editor
+            // follows (`apply_settings`). The tab picked from takes it
+            // too, where the setting was already so.
             Action::SetSqlLimit {
                 tab,
                 sql_tab,
@@ -4569,6 +4579,32 @@ impl App {
                 }
             }
         }
+        if (old.sql_limit, old.sql_timeout_secs)
+            != (self.settings.sql_limit, self.settings.sql_timeout_secs)
+        {
+            // Every open editor runs with the limit and the timeout set:
+            // a run on its way keeps the ones it was sent with.
+            let (limit, timeout) = (self.settings.sql_limit, self.settings.sql_timeout());
+            for tab in &mut self.tabs {
+                if let ConnTabContent::Workspace(workspace) = &mut tab.content {
+                    for sql in workspace.sql_tabs_mut() {
+                        sql.limit = limit;
+                        sql.timeout = timeout;
+                    }
+                }
+            }
+        }
+        if old.autocomplete != self.settings.autocomplete && !self.settings.autocomplete.on_key() {
+            // Off, no list stays open either.
+            for tab in &mut self.tabs {
+                if let ConnTabContent::Workspace(workspace) = &mut tab.content {
+                    for sql in workspace.sql_tabs_mut() {
+                        sql.completion = None;
+                        sql.completion_wanted = None;
+                    }
+                }
+            }
+        }
         if old.table_list != self.settings.table_list {
             // Every open connection with no list of its own follows.
             let flat = self.settings.table_list.flat();
@@ -5445,13 +5481,19 @@ impl App {
 
     /// Opens an empty SQL editor after the workspace's other tabs, with the
     /// limit and timeout of the settings, and shows it: its id, where `tab`
-    /// is a workspace.
+    /// is a workspace. It opens read-only, or as the connection has it
+    /// where the settings say so (`editor.run.new_tab`).
     fn new_sql_tab(&mut self, tab: ConnTabId) -> Option<TabId> {
         let id = TabId(self.next_id());
         let request = RequestId(self.next_id());
         let (limit, timeout) = (self.settings.sql_limit, self.settings.sql_timeout());
+        let new_tab = self.settings.new_tab;
         let workspace = self.workspace_mut(tab)?;
-        workspace.push_sql_tab(id, limit, timeout);
+        let sql = workspace.push_sql_tab(id, limit, timeout);
+        match new_tab {
+            crate::settings::NewTab::ReadOnly => sql.mode = RunMode::ReadOnly,
+            crate::settings::NewTab::Connection => {}
+        }
         workspace.active_tab = Some(id);
         workspace.pane = Pane::Grid;
         // The editor's footer names the server. A session still connecting
@@ -5606,6 +5648,7 @@ impl App {
     /// is tokenized once per change.
     pub fn refresh_completion(&mut self, ctx: &egui::Context) -> bool {
         let active = self.active_sql();
+        let as_you_type = self.settings.autocomplete.as_you_type();
         let mut changed = false;
         for conn in &mut self.tabs {
             let ConnTabContent::Workspace(workspace) = &mut conn.content else {
@@ -5671,9 +5714,10 @@ impl App {
             }
         };
         // Opening, by typing: a word of two characters or more, or right
-        // after a dot, and not where a new name goes. Told before any row
-        // is looked for.
-        if !going && !crate::completion::may_open(&site, &typed, manual) {
+        // after a dot, and not where a new name goes, where typing opens
+        // lists at all. Told before any row is looked for.
+        let opens = manual || as_you_type;
+        if !going && !(opens && crate::completion::may_open(&site, &typed, manual)) {
             sql.completion = None;
             return changed | was_open;
         }
@@ -5747,6 +5791,8 @@ impl App {
     /// `Workspace::run_mode`) and one of the statements looks like a
     /// write. Every other run is the read-only one.
     fn run_sql(&mut self, tab: ConnTabId, id: TabId, all: bool) {
+        // The run key runs the whole tab where the settings say so.
+        let all = all || self.settings.run_scope == crate::settings::RunScope::All;
         let Some(workspace) = self.workspace(tab) else {
             return;
         };
@@ -5761,8 +5807,20 @@ impl App {
         if sql.is_writing() {
             return;
         }
-        let mut statements = tabletist_db::sql::statements(dialect, &sql.text);
-        if !all {
+        // A selection runs what it covers, exactly: the text around it is
+        // blanked, so each statement keeps its line and its offset.
+        let selected = sql
+            .selected
+            .clone()
+            .filter(|range| sql.text.get(range.clone()).is_some());
+        let whole = all || selected.is_some();
+        let mut statements = match selected {
+            Some(range) => {
+                tabletist_db::sql::statements(dialect, &selection_only(&sql.text, range))
+            }
+            None => tabletist_db::sql::statements(dialect, &sql.text),
+        };
+        if !whole {
             statements = tabletist_db::sql::statement_at(&statements, sql.cursor)
                 .cloned()
                 .into_iter()
@@ -6824,6 +6882,21 @@ fn apply_url(form: &mut ConnectionForm) -> Result<(), String> {
     Ok(())
 }
 
+/// `text` with every byte outside `range` blanked, but its line breaks: the
+/// text a selection runs, each of its statements where it is in the
+/// editor. A character of several bytes is as many spaces.
+pub(crate) fn selection_only(text: &str, range: std::ops::Range<usize>) -> String {
+    let mut out = String::with_capacity(text.len());
+    for (at, c) in text.char_indices() {
+        if range.contains(&at) || c == '\n' {
+            out.push(c);
+        } else {
+            out.extend(std::iter::repeat_n(' ', c.len_utf8()));
+        }
+    }
+    out
+}
+
 /// The transaction a run of `statements` is sent in, in an editor whose
 /// runs have `mode`: a read-write one only in Read-write, and only when a
 /// statement looks like a write. So an editor left in Read-write opens no
```

```diff
diff --git a/src/model.rs b/src/model.rs
--- a/src/model.rs
+++ b/src/model.rs
@@ -4143,10 +4143,15 @@ pub struct SqlTab {
     pub text: String,
     /// Byte offset of the editor's cursor, reported by the view.
     pub cursor: usize,
+    /// The bytes the editor's selection covers, where it covers any,
+    /// reported by the view: what Run runs in place of a statement.
+    pub selected: Option<std::ops::Range<usize>>,
+    /// The rows a statement keeps; 0 keeps every one.
     pub limit: u32,
     pub timeout: Option<Duration>,
-    /// What the switch was set to: by `Workspace::push_sql_tab` when the
-    /// tab opened, and by the user since. It counts only while the session
+    /// What the switch was set to: when the tab opened (by
+    /// `Workspace::push_sql_tab`, then `App::new_sql_tab` as the settings
+    /// say), and by the user since. It counts only while the session
     /// can write: ask `Workspace::run_mode` for the mode a run has.
     pub mode: RunMode,
     /// The last run, or the one running (a whole-run failure is its error).
@@ -4211,6 +4216,7 @@ impl SqlTab {
             number,
             text: String::new(),
             cursor: 0,
+            selected: None,
             limit,
             timeout,
             mode: RunMode::default(),
@@ -4739,8 +4745,10 @@ impl Workspace {
     }
 
     /// Adds an empty SQL editor at the end of the strip, numbered after the
-    /// last one, without showing it, in the mode a new editor of this
-    /// workspace opens in now (see `RunMode::of_new_tab`).
+    /// last one, without showing it, in the mode the connection gives a new
+    /// editor of this workspace now (see `RunMode::of_new_tab`): Follow
+    /// connection's. `App::new_sql_tab` makes it read-only where the
+    /// settings say so.
     pub fn push_sql_tab(
         &mut self,
         id: TabId,
```

```diff
diff --git a/src/ui/sql_editor.rs b/src/ui/sql_editor.rs
--- a/src/ui/sql_editor.rs
+++ b/src/ui/sql_editor.rs
@@ -295,6 +295,10 @@ struct Bar<'a> {
     timeout_label: &'a MenuLabel,
     /// The editor's read-write run is in flight: Run and Run all wait.
     writing: bool,
+    /// What Run runs: the selection, the whole tab (the settings' Whole
+    /// tab), or the statement at the cursor. Run all runs the selection
+    /// too where there is one.
+    runs: Runs,
     /// The transaction the editor's runs are in now.
     mode: RunMode,
     /// Whether the switch changes that, or why the editor cannot write.
@@ -341,6 +345,14 @@ fn toolbar(app: &mut App, ui: &mut Ui, tab: ConnTabId, id: TabId) {
         limit_label: &limit_label,
         timeout_label: &timeout_label,
         writing: sql.is_writing(),
+        runs: if sql.selected.is_some() {
+            Runs::Selection
+        } else {
+            match app.settings.run_scope {
+                crate::settings::RunScope::Statement => Runs::Statement,
+                crate::settings::RunScope::All => Runs::All,
+            }
+        },
         mode,
         writes,
         locale,
@@ -702,6 +714,14 @@ fn switch(ui: &mut Ui, rect: Rect, bar: &Bar<'_>) -> Option<RunMode> {
     picked.filter(|picked| *picked != mode)
 }
 
+/// What Run runs now.
+#[derive(Clone, Copy)]
+enum Runs {
+    Statement,
+    All,
+    Selection,
+}
+
 /// Run or Run all in `rect`, with what it does on hover. Neither can be
 /// pressed while the editor's read-write run is in flight, and says so: a
 /// new run would cancel a transaction that may be about to commit.
@@ -715,10 +735,10 @@ fn run_button(
 ) {
     let explained = gettext(
         bar.locale,
-        if all {
-            "Run every statement"
-        } else {
-            "Run the statement at the cursor"
+        match (bar.runs, all) {
+            (Runs::Selection, _) => "Run the selection",
+            (Runs::All, _) | (Runs::Statement, true) => "Run every statement",
+            (Runs::Statement, false) => "Run the statement at the cursor",
         },
     );
     if bar.writing {
```

```diff
diff --git a/src/ui/sql_text.rs b/src/ui/sql_text.rs
--- a/src/ui/sql_text.rs
+++ b/src/ui/sql_text.rs
@@ -357,9 +357,8 @@ fn format_script(ui: &Ui, sql_tab: &mut SqlTab, field: &Field<'_>) -> bool {
     true
 }
 
-/// The number the gutter shows for `line`: its own, or (the terminal's
-/// relative numbers) how far it is from the cursor's line, which keeps
-/// its own.
+/// The number the gutter shows for `line`: its own, or (relative numbers)
+/// how far it is from the cursor's line, which keeps its own.
 fn gutter_number(line: usize, cursor_line: usize, relative: bool) -> usize {
     if relative && line != cursor_line {
         line.abs_diff(cursor_line)
@@ -390,8 +389,9 @@ struct Gutter {
 impl Gutter {
     /// macOS: 52 pt of numbers ending 12 pt before a 4 pt bar, then 14 pt.
     /// The terminal: 48, 10, a 3 pt bar and 12. The numbers' column grows
-    /// with the digits of `lines`.
-    fn new(ui: &Ui, look: &Look, lines: usize) -> Self {
+    /// with the digits of `lines`. Without numbers (`numbered` false), the
+    /// bar is that 12 or 10 in.
+    fn new(ui: &Ui, look: &Look, lines: usize, numbered: bool) -> Self {
         let (column, pad, bar, text_left) = if look.terminal {
             (48.0, 10.0, 3.0, 12)
         } else {
@@ -399,7 +399,11 @@ impl Gutter {
         };
         let digits = lines.max(1).ilog10() as usize + 1;
         let digit = role(look).width(ui.ctx(), look.faces, "0");
-        let column = (digits as f32 * digit + pad + 8.0).ceil().max(column);
+        let column = if numbered {
+            (digits as f32 * digit + pad + 8.0).ceil().max(column)
+        } else {
+            pad
+        };
         Self {
             numbers_right: column - pad,
             bar: egui::Rangef::new(column, column + bar),
@@ -439,6 +443,8 @@ impl Edited {
 
 pub fn show(app: &mut App, ui: &mut Ui, tab: ConnTabId, id: TabId) {
     let (look, palette, locale) = (app.look, app.palette, app.locale);
+    let numbering = app.settings.line_numbers_in(look.terminal);
+    let highlight = app.settings.highlight_line;
     let Some(workspace) = app.workspace_mut(tab) else {
         return;
     };
@@ -449,7 +455,7 @@ pub fn show(app: &mut App, ui: &mut Ui, tab: ConnTabId, id: TabId) {
     };
     let pane = ui.max_rect();
     let lines = sql_tab.text.bytes().filter(|b| *b == b'\n').count() + 1;
-    let gutter = Gutter::new(ui, &look, lines);
+    let gutter = Gutter::new(ui, &look, lines, numbering.shown());
     let editor = editor_id(tab, id);
     // Behind the text: filled in once the rows are laid out.
     let backdrop = ui.painter().add(Shape::Noop);
@@ -538,8 +544,10 @@ pub fn show(app: &mut App, ui: &mut Ui, tab: ConnTabId, id: TabId) {
     {
         behind.push(fill(across(pane.x_range(), y), palette.panel));
     }
-    // The cursor's line, over the band, while the editor has the keys.
+    // The cursor's line, over the band, while the editor has the keys,
+    // unless the settings turn it off.
     if edited.focused
+        && highlight
         && let Some(y) = edited.span(&(cursor_line..=cursor_line))
     {
         let color = if look.terminal {
@@ -554,7 +562,10 @@ pub fn show(app: &mut App, ui: &mut Ui, tab: ConnTabId, id: TabId) {
         behind.push(fill(across(bar, y), bar_color(&look, &palette)));
     }
     ui.painter().set(backdrop, Shape::Vec(behind));
-    // The numbers of the rows in sight.
+    // The numbers of the rows in sight, where the gutter has numbers.
+    if !numbering.shown() {
+        return;
+    }
     let clip = ui.clip_rect();
     let rows = &edited.galley.rows;
     let first = rows.partition_point(|row| edited.origin.y + row.max_y() < clip.top());
@@ -578,7 +589,8 @@ pub fn show(app: &mut App, ui: &mut Ui, tab: ConnTabId, id: TabId) {
         } else {
             palette.faint
         };
-        let number = gutter_number(line, cursor_line, look.terminal);
+        let relative = numbering == crate::settings::LineNumbers::Relative;
+        let number = gutter_number(line, cursor_line, relative);
         widgets::paint_text_right(
             ui,
             pane.left() + gutter.numbers_right,
@@ -787,6 +799,9 @@ fn edit(ui: &mut Ui, sql_tab: &mut SqlTab, field: &Field<'_>) -> Edited {
     // (egui draws one after any event).
     if let Some(range) = output.state.cursor.range(&output.galley) {
         sql_tab.cursor = byte_offset(&sql_tab.text, range.primary.index.0);
+        let [start, end] = range.sorted_cursors();
+        let byte = |cursor: CCursor| byte_offset(&sql_tab.text, cursor.index.0);
+        sql_tab.selected = (start != end).then(|| byte(start)..byte(end));
     }
     let typed = output.response.changed() && ui.input(|input| input.events.iter().any(is_typed));
     Edited {
```

- [ ] **Step 4: Run the tests to see them pass**

Run: `cargo test --locked --lib -- ui::editor_settings_tests ui::sql_editor app::tests ui::tests ui::complete_tests`
Expected: PASS. Then the four checks above.

- [ ] **Step 5: Commit**

```bash
git add src/app.rs src/model.rs src/ui/editor_settings_tests.rs src/ui/mod.rs src/ui/sql_editor.rs src/ui/sql_text.rs
git commit -m "Open new SQL tabs read-only, follow the limit and timeout, and run, complete and number lines as set"
```

---

### Task 5: The website

**Files:**
- Modify: `docs/_reference/settings-and-files.md`, `docs/_reference/keyboard-shortcuts.md`, `docs/_guide/sql-editor.md`, `docs/_guide/omarchy.md`, `docs/_guide/macos.md`

- [ ] **Step 1: Say what the pane does**

```diff
diff --git a/docs/_guide/macos.md b/docs/_guide/macos.md
--- a/docs/_guide/macos.md
+++ b/docs/_guide/macos.md
@@ -157,7 +157,7 @@ column's line, or come to them with Tab or F6.
 
 | Keys | Do |
 | --- | --- |
-| Cmd+Return | Run the statement at the cursor |
+| Cmd+Return | Run the statement at the cursor, the whole script or the selection, as Settings › Editor says |
 | Cmd+Shift+Return | Run all |
 | Cmd+. | Cancel the run |
 | Cmd+Shift+F | Format |
```

```diff
diff --git a/docs/_guide/omarchy.md b/docs/_guide/omarchy.md
--- a/docs/_guide/omarchy.md
+++ b/docs/_guide/omarchy.md
@@ -217,7 +217,7 @@ A save can ask you something first. Each question has its own keys:
 
 | Keys | Do |
 | --- | --- |
-| Ctrl+Enter | Run the statement at the cursor |
+| Ctrl+Enter | Run the statement at the cursor, the whole script or the selection, as Settings › Editor says |
 | Ctrl+Shift+Enter | Run all |
 | Ctrl+C | Cancel the run, while one runs. With none running it copies |
 | Esc | Leave the editor: the letters below are read from then on |
@@ -257,7 +257,7 @@ offers.
 | `j` `k` | Move |
 | `h` `l` | Change the option |
 | Space | Toggle the option |
-| Ctrl+H, Ctrl+L | The pane before or after: General, Appearance, Agents |
+| Ctrl+H, Ctrl+L | The pane before or after: General, Appearance, Editor, Agents |
 | Enter | On the Agents pane: show the Agents tab, Connections or the log, from the status, access or log row |
 | `y` | On the Agents pane: copy the command that connects Claude Code |
 | Ctrl+E | Open `settings.toml` in your editor |
```

```diff
diff --git a/docs/_guide/sql-editor.md b/docs/_guide/sql-editor.md
--- a/docs/_guide/sql-editor.md
+++ b/docs/_guide/sql-editor.md
@@ -10,8 +10,11 @@ opened on.
 
 ## Run
 
-- **Run** (Cmd/Ctrl+Return) runs the statement the cursor is in.
-- **Run all** (Cmd/Ctrl+Shift+Return) runs the whole script.
+- **Run** (Cmd/Ctrl+Return) runs the statement the cursor is in, or the
+  whole script where **Settings › Editor** says so. With text selected,
+  it runs what you selected, exactly.
+- **Run all** (Cmd/Ctrl+Shift+Return) runs the whole script, or what you
+  selected.
 - Cmd/Ctrl+. cancels a run. In the Omarchy look the key is Ctrl+C.
 
 {% include shot.html file="macos-sql" alt="A SQL editor with two statements, the result of the one at the cursor under it" %}
@@ -20,10 +23,11 @@ Two menus in the toolbar bound every run:
 
 | Menu | Choices | Default |
 | --- | --- | --- |
-| **Limit** | 100, 1,000 or 10,000 rows for each statement | 1,000 |
-| **Timeout** | 10, 30, 60 or 300 seconds, or no timeout | 30 seconds |
+| **Limit** | 100, 500, 1,000, 5,000 or 10,000 rows for each statement, or no limit | 1,000 |
+| **Timeout** | 5, 15 or 30 seconds, 1 or 5 minutes, or no timeout | 30 seconds |
 
-Your choice is remembered for new tabs.
+Your choice is the one of **Settings › Editor**, and every open tab
+follows it.
 
 ## Read-only and read-write
 
@@ -39,9 +43,10 @@ look) sets what a run may do.
 
 {% include shot.html file="macos-sql-committed" alt="A read-write run of two statements, with Messages saying how many rows each changed and that the run was committed" %}
 
-A new tab opens in Read-write only on a writable connection that is not
-production. On a connection that opens read-only, and on any production
-connection, every run is read-only.
+A new tab opens in Read-only. With **New tabs run in** set to Follow
+connection in **Settings › Editor**, it opens in Read-write on a writable
+connection that is not production. On a connection that opens read-only,
+and on any production connection, every run is read-only.
 
 Because Tabletist runs your script in its own transaction, statements that
 would leave it are refused, and then nothing runs. `BEGIN`, `COMMIT`,
@@ -75,7 +80,8 @@ keywords change.
 
 Keywords, schemas, tables, views and the columns of the tables in your
 statement are offered while you type. Ctrl+Space asks for the list
-anywhere, and so does Cmd/Ctrl+I on macOS and Windows.
+anywhere, and so does Cmd/Ctrl+I on macOS and Windows. **Settings ›
+Editor** can have the list open only when you ask, or never.
 
 {% include shot.html file="macos-sql-complete" alt="The completion list under a table name being typed, offering the tables that match" %}
 
```

```diff
diff --git a/docs/_reference/keyboard-shortcuts.md b/docs/_reference/keyboard-shortcuts.md
--- a/docs/_reference/keyboard-shortcuts.md
+++ b/docs/_reference/keyboard-shortcuts.md
@@ -115,7 +115,7 @@ These are the keys of the macOS and Windows looks. The Omarchy look has
 | Keys | Do |
 | --- | --- |
 | Mod+T | New SQL editor |
-| Mod+Return | Run the statement at the cursor |
+| Mod+Return | Run the statement at the cursor, the whole script or the selection, as Settings › Editor says |
 | Mod+Shift+Return | Run all |
 | Mod+. | Cancel the run |
 | Mod+Shift+F | Format |
```

````diff
diff --git a/docs/_reference/settings-and-files.md b/docs/_reference/settings-and-files.md
--- a/docs/_reference/settings-and-files.md
+++ b/docs/_reference/settings-and-files.md
@@ -43,7 +43,23 @@ are made in the app, so it needs no connection and reads no database.
 | **Table list** | Grouped by prefix (Omarchy: tree) or Flat. The default for a connection whose list you have not switched; the sidebar's switch (Omarchy: `t`) sets the connection's own | Grouped by prefix |
 | **Window header** (Omarchy: connection bar) | Tint + stripe (Omarchy: tint + label), or Stripe only (Omarchy: label only). Production keeps its tint either way | Tint + stripe |
 
-A third pane, **Agents**, sets what the `[mcp]` keys
+A third pane, **Editor**, sets how the SQL editor runs and types. Every
+open SQL editor follows each change at once.
+
+| Option | Choices | Default |
+| --- | --- | --- |
+| **New tabs run in** | Read-only transaction, or Follow connection. Read-only: a new SQL editor opens in Read-only on every connection, and its switch can still set it to Read-write. Follow connection: it opens in Read-write on a writable connection that is not production, as the [SQL Editor guide]({% link _guide/sql-editor.md %}#read-only-and-read-write) says. A connection that opens read-only keeps every editor read-only either way. A change is for the tabs opened after it | Read-only transaction |
+| **Row limit** | 100, 500, 1,000, 5,000 or 10,000 rows for each statement, or No limit. The rows past the limit are not fetched; nothing is added to your SQL | 1,000 |
+| **Timeout** | 5, 15 or 30 seconds, 1 or 5 minutes, or Off. A run still going then is cancelled and rolled back; Cancel works either way | 30 s |
+| **⌘↩ runs** (Omarchy: ctrl+enter runs) | Statement at cursor, or Whole tab: every statement in order, stopping at the first error. A selection always runs what it selects, whichever is set | Statement at cursor |
+| **Autocomplete** | As you type, On Ctrl+Space (the list opens only when you ask), or Off | As you type |
+| **Line numbers** | On or off on macOS and Windows; relative, absolute or off in the Omarchy look. On writes `absolute`; a `relative` set in the file shows as on and stays | On (absolute) on macOS and Windows, relative in the Omarchy look |
+| **Current line highlight** (macOS and Windows) | On or off: the line the cursor is on is filled while the editor has the keyboard. The Omarchy look has no row: it follows the key, set in the file | On |
+
+The other keys of the four `[editor.…]` tables are read and kept now, and
+take effect in a later version.
+
+A fourth pane, **Agents**, sets what the `[mcp]` keys
 hold, and shows how to connect an agent:
 [Agents]({% link _guide/agents.md %}) is the guide.
 
@@ -55,7 +71,7 @@ hold, and shows how to connect an agent:
 
 On macOS and Windows the window shows where the settings file is, and can
 reveal it in your file manager, export it, or reset the options of the
-pane it shows. On the Agents pane, a reset leaves the server's switch as
+pane it shows (on the Editor pane, every key of the `[editor.…]` tables). On the Agents pane, a reset leaves the server's switch as
 it is. In the Omarchy look the file is shown beside the options.
 
 {% include shot.html file="omarchy-settings" alt="The Settings screen in the Omarchy look, with settings.toml beside the options" width="2880" height="1800" %}
@@ -66,8 +82,9 @@ Settings are kept in `settings.toml`, which is safe to edit by hand.
 Tabletist watches the file and applies your edits while it runs. The file
 holds only what differs from the defaults. Until you change an option
 there is no file, and one the app writes starts with a header line. Here
-is every key at its default (`group_digits`, `theme`, `font`, `font_size`
-and `zebra` have none of their own: they follow the look):
+is every key at its default (`group_digits`, `theme`, `font`, `font_size`,
+`zebra`, `keys`, `line_numbers` and `keywords` have none of their own:
+they follow the look):
 
 ```toml
 [startup]
@@ -88,9 +105,25 @@ idle_disconnect = "30m"
 [sidebar]
 show_system_schemas = false
 
-[editor]
-sql_limit = 1000
-sql_timeout_secs = 30
+[editor.run]
+new_tab = "read-only"
+limit = 1000
+timeout = "30s"
+run = "statement"
+confirm_unsafe_writes = true
+
+[editor.typing]
+indent = 2
+autocomplete = "typing"
+close_pairs = true
+highlight_line = true
+wrap = false
+
+[editor.format]
+commas = "trailing"
+
+[editor.history]
+keep = 1000
 
 [mcp]
 enabled = false
@@ -122,8 +155,21 @@ bar = "tint"
 | `safety.confirm_close_edits` | `true` asks before pending changes are dropped; `false` discards them without asking. While a save runs, what would drop them does nothing until it ends, and closing the window asks either way | `true` |
 | `safety.idle_disconnect` | `"never"`, `"5m"`, `"15m"`, `"30m"`, `"1h"` or `"4h"`. A connection left unused that long has its server session closed; its tabs stay as they were, and it connects again when you next refresh, page, open a table, run or save. Never while changes are pending or something is still loading, running or saving | `"30m"` |
 | `sidebar.show_system_schemas` | `true` shows schemas such as `pg_catalog` and `information_schema` | `false` |
-| `editor.sql_limit` | 1 to 10000 rows for each statement | `1000` |
-| `editor.sql_timeout_secs` | Seconds. `0` waits forever | `30` |
+| `editor.run.new_tab` | `"read-only"` or `"connection"` | `"read-only"` |
+| `editor.run.limit` | 1 to 10000 rows for each statement, or `0` for no limit | `1000` |
+| `editor.run.timeout` | `"1s"` … `"3600s"` or `"1m"` … `"60m"`, or `"off"`. A longer one is a line that cannot be read | `"30s"` |
+| `editor.run.run` | `"statement"` or `"all"`: what Cmd/Ctrl+Return runs | `"statement"` |
+| `editor.run.confirm_unsafe_writes` | `true` or `false`. Read and kept; not acted on yet | `true` |
+| `editor.typing.keys` | `"standard"` or `"vim"`. Read and kept; not acted on yet | Not set: standard on macOS and Windows, vim in the Omarchy look |
+| `editor.typing.indent` | `2`, `4` or `"tab"`. Read and kept; not acted on yet | `2` |
+| `editor.typing.autocomplete` | `"typing"`, `"manual"` or `"off"` | `"typing"` |
+| `editor.typing.close_pairs` | `true` or `false`. Read and kept; not acted on yet | `true` |
+| `editor.typing.line_numbers` | `"absolute"`, `"relative"` or `"off"` | Not set: absolute on macOS and Windows, relative in the Omarchy look |
+| `editor.typing.highlight_line` | `true` or `false` | `true` |
+| `editor.typing.wrap` | `true` or `false`. Read and kept; not acted on yet | `false` |
+| `editor.format.keywords` | `"upper"`, `"lower"` or `"keep"`. Read and kept; not acted on yet | Not set: upper on macOS and Windows, lower in the Omarchy look |
+| `editor.format.commas` | `"trailing"` or `"leading"`. Read and kept; not acted on yet | `"trailing"` |
+| `editor.history.keep` | 0 to 100000 queries for each connection. Read and kept; not acted on yet | `1000` |
 | `appearance.interface_size` | `"small"`, `"default"` or `"large"`, on macOS and Windows | `"default"` |
 | `appearance.font` | On macOS the data's face, `"IBM Plex Mono"`, `"JetBrains Mono"` or the family name of an installed font (`"Menlo"`); `"monospace"` is IBM Plex Mono too, and a name no installed font has is drawn as IBM Plex Mono, which the window says. Windows draws its data in Inter, whatever it says. In the Omarchy look a fontconfig name (`"Fira Code"`); `"monospace"` is the system's. Written once you set it | Not set: IBM Plex Mono, or the system's monospace |
 | `appearance.font_size` | Points, whole or by halves (`13`, `12.5`). On macOS and Windows the data's, 11 to 16; in the Omarchy look all of it, 9 to 24. A size past a look's range is drawn at its nearest end. Written once you set it | Not set: 12.5 on macOS and Windows, 13 in the Omarchy look |
@@ -140,7 +186,8 @@ bar = "tint"
 `show_system_schemas` is set only in the file, and so is a pin by a
 name or a path other than the Omarchy theme of the moment, and a font by
 its fontconfig name. The SQL editor's **Limit** and **Timeout** menus set
-the two `editor` keys, and the text size keys (Cmd+= Cmd+- Cmd+0 on macOS, Ctrl+=
+`editor.run.limit` and `editor.run.timeout`, which every open SQL editor
+follows, and the text size keys (Cmd+= Cmd+- Cmd+0 on macOS, Ctrl+=
 Ctrl+- Ctrl+0 on Windows, `ctrl+=` `ctrl+-` in the Omarchy look) set
 `font_size`, wherever the keyboard is.
 
@@ -160,7 +207,11 @@ saves a copy of the file as it is.
 A file an older version wrote holds every key, at the values it had then,
 the defaults of that time too (`page_size = 300`, `group_digits = false`).
 Those stay until you change them: take a key out of the file, or reset the
-tab, to have today's default.
+tab, to have today's default. An older file's `editor.sql_limit` and
+`editor.sql_timeout_secs` are read while the file has no
+`editor.run.limit` and `editor.run.timeout` (a timeout over an hour as an
+hour), and give way to them the first time you set the limit or the
+timeout.
 
 ## Files
 
````

- [ ] **Step 2: Build the site**

Run: `cd docs && bin/check` (it builds and checks the site as `docs.yml` does) where Ruby and the bundle are installed; otherwise leave it to `docs.yml` on the pull request.
Expected: the build and its checks pass.

- [ ] **Step 3: Commit**

```bash
git add docs/_reference/settings-and-files.md docs/_reference/keyboard-shortcuts.md docs/_guide/sql-editor.md docs/_guide/omarchy.md docs/_guide/macos.md
git commit -m "Describe Settings › Editor and its keys on the website"
```

---

### Task 6: The pull request

- [ ] Push the branch and open one pull request: a summary; the settings built and the rows hidden until their features exist (Keybindings, Indent, Close brackets, Wrap, Confirm UPDATE or DELETE without WHERE, Format, History, Preview); the keys added and the keymap check (the run and completion keys come from the keymap, `ctrl+w` is unchanged, `:ro` / `:rw` named in the hint); the open questions and the defaults chosen; the departures above; the fixed test of `main`. No screenshot.
