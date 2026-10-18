# Settings › General, step 2: Safety. Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Settings › General gets its Safety group, in both looks: **New production connections open read-only** sets where a new production connection's switch starts, **Ask before closing with unsaved edits** off drops pending edits without the question, and **Disconnect when idle** closes the server session of a connection left unused, whose tabs stay and connect again when they are next used.

**Architecture:** Three keys under `[safety]` in `src/settings.rs` (`production_read_only`, `confirm_close_edits`, `idle_disconnect: Idle`), each an `OptionId` with a row in `src/ui/settings/{sheet,terminal}.rs`. The connection dialog's form carries the option for a new connection (`ConnectionForm::production_read_only`) and saves the switch it shows. `App::hold` (`src/app/editing.rs`) discards and goes ahead when the question is off. The backend's worker keeps what is sent to a session still connecting (`Worker::waiting`) and hands it over once the session is up. `App::close_idle(now)`, called from `App::logic`, closes a connected session unused for longer than the option (`SessionStatus::Idle`); every sender of a command that a use needs calls `App::wake` once it has something to send, which connects an idle one again, and a session opened again that way skips `after_connect`.

**Tech Stack:** Rust 2024, egui, tokio (the backend's worker), the `toml` reader the settings already use.

---

## Where this sits

Step 2 of `docs/superpowers/specs/2026-10-17-settings-general-rows-design.md`. Step 1 (the file and the Data rows) is pull request igor-alexandrov/tabletist#154, merged. Tasks 1 and 2 are pull request igor-alexandrov/tabletist#160, merged (merge commit `660cfd9`); tasks 3 to 5 are a branch of `main` after it. Their diffs were generated again on that `main`, which has Settings › Appearance (#156) and the zones in `src/zones.rs`: General passes its sections, `("Data", &OptionId::DATA), ("Safety", &OptionId::SAFETY)`, to the sheet's `content` and to the Omarchy screen's `rows()`, so task 5 puts `OptionId::IdleDisconnect` in `OptionId::SAFETY`, and the counts and line numbers of the tests take the Appearance keys. Step 3 (Startup: the session file, Restore tabs and filters, On launch) is its own plan and pull request.

## Where this departs from the spec or the boards

The spec's own departures were approved with it; the one of this step ("Ask before closing, off: the held action goes ahead and the edits are discarded … A save that is still running is still waited for") is built as the app treats a running save today: the action does nothing, and closing the window asks. These are new, and Igor approved them on 2026-10-18:

| What the spec or the board says | What this run builds | Why |
|---|---|---|
| Spec: an action that needs an idle connection's session "is queued; `after_connect` replays the queue, which already reloads the active page" | The commands the action sends wait in the backend until the new session is up, and that session answers them (task 3). A session opened again for a use does not run `after_connect`: the tree and the other tabs stay as they were, and nothing is fetched twice. A connect that has to ask for a secret first is not sent at once: a save or a run is not sent before it (the save says "Not connected. Nothing was sent."), a load fails as lost, and that connect runs `after_connect`, as any reconnect does (task 4) | `after_connect` fetches the tree and the active page again, so with the action's own fetch the page would be fetched twice. One queue in the backend holds what any of the app's dozen senders sends, where a queue of actions would have to know which actions need the session |
| Spec: never "with … a run, a fetch or a save in flight (`App::is_busy`)" | Never while anything of the connection waits for its session: any tab's page, structure, count, run or save, the tree, the databases, the server's version, a table's columns (`Workspace::waits`), or a foreign key's list (`App::lookups`). A connection that waits is in use while it waits, so its time starts again when the answer comes | `is_busy` looks only at the tab in front: a run in a tab behind it would be cut off. And a run of 40 minutes would lose its session the moment it ended |
| Spec: "the status line says 'Disconnected while idle'" | The connection's chip in the bar and its card's Status say "Disconnected while idle", in the dim colour; no banner, and the page is not dimmed. While it connects again for a use the chip says "Connecting…" as any connect does, and there is no "Reconnecting" strip either. The Omarchy status line keeps its keys and the page's range | The chip and the card are where both looks say how a session stands; the status line says nothing of the session today |
| macOS board: "How Tabletist starts, shows data and protects your work." | "How Tabletist shows data and protects your work." | Nothing of startup is in the window until step 3, which puts the board's words back |

## What this run settles

| Question | Decision | Why |
|---|---|---|
| Where the keys go in the file | `[safety]`, after `[data]`: `production_read_only`, `confirm_close_edits`, then `idle_disconnect` with its note `# never \| 5m \| 15m \| 30m \| 1h \| 4h`, each only where it is not the default | As the Omarchy board writes the file, by the rules of step 1 |
| A new production connection | Its switch starts at the option, and it is saved with `read_only` set, so a later change of the option changes no saved connection. A new connection of another environment is saved as before, with no switch set. A saved connection, edited or duplicated, keeps the built-in rule | The spec |
| The environment of a new connection changed in the dialog | The switch follows it until the person sets it: on (or as the option says) for production, off for the others | As today, with the option in production's place |
| Off, which actions drop edits without asking | Every one the question guards: closing the table's tab, its connection tab or the window, disconnecting, connecting another connection there, switching database, refreshing, paging, sorting, filtering, retrying, and following a foreign key into a table that holds edits | One rule: no question, and Discard is the answer |
| Off, with a save running | As on: the action is ignored, and closing the window asks, with no Save | A save cannot be discarded (the spec) |
| Off, under a dialog (a production save's confirmation) | As on: nothing is dropped, and the notice asks to save or discard first | The dialog may be about those edits: dropped under it, they would leave it showing a set that is gone (found by the review) |
| What is a use | A command sent for the session that a use needs: a page, a structure, a count, the tree's schemas and objects, a table's columns, a run, a save, a foreign key's list or question (each of those senders calls `App::wake` once it has something to send). A cancel, a close, the server's version and the databases' list do not wake: they are asked only of a session that is up. A SQL editor opened while idle learns the server when the session is next up, and a key, typed text, a click, a scroll, a paste, a copy or a cut while the connection's tab is in front. The pointer moving is not | A pointer crossing the window is no work in it |
| What an idle connection can do | Everything it could: Save and Run are not disabled, and connect it again; completion lists ask for names once it is connected | "Tabs stay and reconnect on use" |
| A connect again for a use that fails | The usual banner. A save that waited for it says "Not connected. Nothing was sent." and keeps its changes; a run that waited is forgotten, and does not say it may have written; what was being loaded shows the connection lost. The next connect, by Reconnect or after a new password, restarts the tab as any reconnect does | Nothing that waited left the app; saying it may have been written would be false (found by the review) |
| SQLite | Closed too | One rule for every engine; a file opens again at no cost |
| How the closing comes | `App::logic` calls `close_idle(now)` every frame and asks for a frame again when the next connection is due. Tests pass `now` and never sleep | The spec |
| The menu's words | Never, After 5 min, After 15 min, After 30 min, After 1 h, After 4 h. The Omarchy row writes the file's value, `30m`, between chevrons on the cursor's row | As the boards write them |
| The Omarchy `confirm close with edits` row | No hint | As drawn |
| A label longer than a narrow sheet's label column | Wraps in its column | "New production connections open read-only" ran under its switch below a width of 840 (found by the review) |
| A word from a session closed while idle | A late `Disconnected` changes nothing, and no banner is shown | It was closed on purpose |

## Before you start

- One commit for each task, in the order below. Every commit passes `cargo fmt --all --check`, `cargo clippy --locked --workspace --all-targets -- -D warnings` and `cargo test --locked --workspace --all-targets`.
- On Igor's machine `cargo` is `~/.cargo/bin/cargo` (the mise shim fails), and a build in a worktree needs `CARGO_TARGET_DIR` set to that worktree's `target/`.
- Read `AGENTS.md`: code that depends on the engine matches on `Driver` or `Dialect` naming every variant; views draw text only through `TextRole`s; the website's pages follow `src/settings.rs`.
- Each step's diff applies with `git apply` from the repository's root: save the block to a file (`/tmp` is fine) and run `git apply <file>`. Run `cargo fmt --all` before each commit: the code here is already formatted.
- The tests use only the Bookshop data. No test compares a picture or a position with the design.

## What was run before this plan was written

Every task's state was built here and passed `cargo fmt --all --check`, `cargo clippy --locked --workspace --all-targets -- -D warnings` and `cargo test --locked --workspace --all-targets` with the test PostgreSQL, MySQL and SSH servers (3,612 tests after task 1, 3,637 after task 5, none failed). Each task's tests were run alone on the code before it, and failed as its Step 2 says. The last state also passed `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps` and `docs/bin/check`. Both Settings screens were rendered headless at the boards' sizes (1440 by 1000 on macOS, 1920 by 1080 on Omarchy) and compared with the "Settings, General" artboards by eye; the pictures were not kept. A reviewer given the draft wrote throwaway tests against it and found four defects (two in the connect again for a use, one in the question off under a dialog, one in the sheet's layout), each fixed in the task it belongs to and covered by a test there (marked "found by the review" below). A second reviewer read the generated plan against its diffs and the spec, and found where the prose said more than the code, and four more defects (a save sent before a connect that asks for a secret, a "Reconnecting" strip on every use, a foreign key's list that did not count as waiting, a doc comment out of place); those were fixed the same way. Every diff in this plan was applied in order to a copy of the base and the result compared with the last state. Not run: CI, macOS and Windows, and the app by hand.

Tasks 3 to 5 were generated again on 2026-10-18, on `main` at `660cfd9` (tasks 1 and 2 merged). Task 3's diffs applied as they were; task 4's took the Appearance keys in `Key::ALL` and the key lines of the file; task 5's put `OptionId::IdleDisconnect` in `OptionId::SAFETY`, where General's sections (`content` in `src/ui/settings/sheet.rs`, `rows()` in `src/ui/settings/terminal.rs`) find it, in place of `group_starting`, and take the Appearance options in every match. Each task's state passed `cargo fmt --all --check`, `cargo clippy --locked --workspace --all-targets -- -D warnings` and `cargo test --locked --workspace --all-targets` with the test PostgreSQL, MySQL and SSH servers (3,680 tests after task 3, 3,698 after task 4, 3,699 after task 5, none failed; `the_agent_opens_the_tunnel` skipped, as `TABLETIST_TEST_SSH_AGENT` needs an agent that holds the test key, which CI loads). Each task's Step 2 failed as it says. The last state also passed `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps` and `docs/bin/check`, and the six diffs, applied in order to `main`, give that state.

## Files

| File | What changes |
|---|---|
| `src/settings.rs` | `Idle`; the three keys, their options and values |
| `src/model.rs` | `ConnectionForm::production_read_only`; `SessionStatus::Idle`; `Workspace::{used, woken}`, `waits`, `holds_edits`, `takes_commands`; the screen's rows |
| `src/app.rs` | The new connection's form; `close_idle`, `wake`, `note_use`; a woken session skips `after_connect`; runs on an idle session |
| `src/app/editing.rs`, `src/app/key_picker.rs` | `hold` off, `discard_edits`; the senders wake; Save on an idle session |
| `src/backend.rs` | `Worker::waiting` |
| `src/ui/settings/{mod,sheet,terminal}.rs` | The Safety group in both looks |
| `src/ui/workspace.rs`, `src/ui/sql_results.rs` | How an idle session is drawn; the card's Run again |
| `src/ui/mod.rs`, `src/entrypoint.rs` | Their tests |
| `docs/_reference/settings-and-files.md`, `docs/_reference/connections.md`, `docs/_guide/editing-data.md` | The website |

---

### Task 1: The Safety group, and a new production connection opens read-only as it says

The Safety group's heading and its first row in both looks, the key `safety.production_read_only`, and the connection dialog's switch for a new production connection. The tab's lead names what the tab now protects, and a row's label wraps in its column in a narrow sheet. The sheet's group headings scroll with the rows: the Data heading moves into the rows' scroll area, where the Safety heading is.

**Files:** Modify `docs/_reference/connections.md`; Modify `docs/_reference/settings-and-files.md`; Modify `src/app.rs`; Modify `src/model.rs`; Modify `src/settings.rs`; Modify `src/ui/mod.rs`; Modify `src/ui/settings/mod.rs`; Modify `src/ui/settings/sheet.rs`; Modify `src/ui/settings/terminal.rs`.

> **Merged as igor-alexandrov/tabletist#160; the merged code supersedes this task's diffs.** They were made before Settings › Appearance (#156): on `main` the General pane passes its sections to the sheet's `content` and the Omarchy screen's `rows()`, and `group_starting` is gone.

- [ ] **Step 1: Write the failing tests**

````diff
diff --git a/src/model.rs b/src/model.rs
index 56be103..ff16c81 100644
--- a/src/model.rs
+++ b/src/model.rs
@@ -5520,6 +5520,51 @@ mod tests {
         }
     }
 
+    /// A new connection to production, its switch as the settings start
+    /// it.
+    fn production_form(production_read_only: bool) -> ConnectionForm {
+        ConnectionForm {
+            environment: Some(crate::env::Environment::Production),
+            production_read_only,
+            ..ssh_form()
+        }
+    }
+
+    #[test]
+    fn a_new_production_connection_keeps_the_switch_the_settings_gave_it() {
+        for on in [true, false] {
+            let form = production_form(on);
+            assert_eq!(form.read_only(), on);
+            // Saved with the switch set: a later change of the settings
+            // changes no saved connection.
+            assert_eq!(form.to_saved().unwrap().read_only, Some(on));
+        }
+        // What the person sets wins.
+        let form = ConnectionForm {
+            read_only: Some(true),
+            ..production_form(false)
+        };
+        assert_eq!(form.to_saved().unwrap().read_only, Some(true));
+        // Another environment has its own rule, and saves no switch.
+        let form = ConnectionForm {
+            environment: Some(crate::env::Environment::Staging),
+            ..production_form(true)
+        };
+        assert!(!form.read_only());
+        assert_eq!(form.to_saved().unwrap().read_only, None);
+    }
+
+    #[test]
+    fn a_saved_connection_keeps_the_built_in_rule_whatever_the_settings_say() {
+        let saved = crate::connections::SavedConnection {
+            read_only: None,
+            ..production_form(false).to_saved().unwrap()
+        };
+        let form = ConnectionForm::from_saved(&saved);
+        assert!(form.read_only(), "production is read-only by its rule");
+        assert_eq!(form.to_saved().unwrap().read_only, None, "and stays unset");
+    }
+
     #[test]
     fn an_ssh_form_becomes_a_spec_and_back() {
         let saved = ssh_form().to_saved().unwrap();
diff --git a/src/settings.rs b/src/settings.rs
index f91cc45..e7b4136 100644
--- a/src/settings.rs
+++ b/src/settings.rs
@@ -1284,6 +1284,7 @@ mod tests {
             tz: Zone::Named("America/New_York".into()),
             group_digits: Some(true),
             value_tags: false,
+            production_read_only: false,
             show_system_schemas: true,
             custom_theme: Some("My \"Nord\".json".into()),
             sql_limit: 100,
@@ -1307,6 +1308,9 @@ tz = \"America/New_York\"  # system | utc | \"Area/City\"
 group_digits = true
 value_tags = false
 
+[safety]
+production_read_only = false
+
 [sidebar]
 show_system_schemas = true
 
@@ -1380,13 +1384,14 @@ theme = \"My \\\"Nord\\\".json\"
                 (Key::Tz, 5),
                 (Key::GroupDigits, 6),
                 (Key::ValueTags, 7),
-                (Key::ShowSystemSchemas, 10),
-                (Key::SqlLimit, 13),
-                (Key::SqlTimeoutSecs, 14),
-                (Key::McpEnabled, 17),
-                (Key::McpLogDays, 18),
-                (Key::McpNotify, 19),
-                (Key::Theme, 22),
+                (Key::ProductionReadOnly, 10),
+                (Key::ShowSystemSchemas, 13),
+                (Key::SqlLimit, 16),
+                (Key::SqlTimeoutSecs, 17),
+                (Key::McpEnabled, 20),
+                (Key::McpLogDays, 21),
+                (Key::McpNotify, 22),
+                (Key::Theme, 25),
             ]
         );
     }
@@ -2336,6 +2341,7 @@ theme = \"My \\\"Nord\\\".json\"
                 Key::Tz,
                 Key::GroupDigits,
                 Key::ValueTags,
+                Key::ProductionReadOnly,
                 Key::McpEnabled,
                 Key::McpLogDays,
                 Key::McpNotify,
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
index 6317f67..daa4a27 100644
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -1289,7 +1289,7 @@ value_tags   = true
             for text in [
                 "Settings",
                 "General",
-                "How Tabletist shows data. Changes apply right away.",
+                "How Tabletist shows data and protects your work. Changes apply right away.",
                 "Rows per page",
                 "Table view; the SQL editor has its own limit",
                 "Timestamps",
@@ -1343,6 +1343,82 @@ value_tags   = true
         }
     }
 
+    #[test]
+    fn the_safety_group_of_the_sheet_sets_where_a_new_production_connection_starts() {
+        for look in SHEET_LOOKS {
+            let mut harness = settings_sheet(look());
+            for text in [
+                // A section's heading is written in capitals.
+                "SAFETY",
+                "New production connections open read-only",
+                "Default for the switch in each connection",
+            ] {
+                assert!(harness.has(text), "{text}");
+            }
+            harness.click("New production connections open read-only");
+            assert!(!harness.app.settings.production_read_only);
+            let text = &harness.app.settings_file.text;
+            assert!(
+                text.ends_with("[safety]\nproduction_read_only = false\n"),
+                "{text}"
+            );
+        }
+    }
+
+    #[test]
+    fn a_long_label_of_the_sheet_wraps_before_its_control_in_a_narrow_window() {
+        use egui::accesskit::Role;
+        let label = "New production connections open read-only";
+        for width in [720.0, 800.0, 1280.0] {
+            for look in SHEET_LOOKS {
+                let mut harness = Harness::with_size(egui::vec2(width, 1000.0));
+                harness.set_look(look());
+                harness.press(egui::Key::Comma, egui::Modifiers::COMMAND);
+                let tree = harness.settle();
+                let text = harness.painted_rect(label).expect("the label");
+                let switch =
+                    crate::testing::bounds(&tree, label, Role::CheckBox).expect("the switch");
+                assert!(
+                    text.right() <= switch.left(),
+                    "{width}: {text:?} {switch:?}"
+                );
+            }
+        }
+    }
+
+    #[test]
+    fn the_safety_group_of_the_screen_has_its_rows_and_space_flips_them() {
+        let mut harness = settings_screen_on_the_artboard();
+        for text in [
+            "safety",
+            "production opens read-only",
+            "default for new connections",
+        ] {
+            assert!(harness.painted_color(text).is_some(), "{text}");
+        }
+        let row = crate::model::SettingsPane::General
+            .rows()
+            .iter()
+            .position(|row| {
+                *row == crate::model::SettingsRow::Option(
+                    crate::settings::OptionId::ProductionReadOnly,
+                )
+            })
+            .expect("the row");
+        for _ in 0..row {
+            harness.press(egui::Key::J, egui::Modifiers::NONE);
+        }
+        assert!(
+            harness
+                .painted_color("▌production opens read-only")
+                .is_some()
+        );
+        harness.press(egui::Key::Space, egui::Modifiers::NONE);
+        assert!(!harness.app.settings.production_read_only);
+        harness.press(egui::Key::R, egui::Modifiers::SHIFT);
+        assert!(harness.app.settings.production_read_only);
+    }
+
     #[test]
     fn a_page_size_from_the_file_that_is_not_in_the_list_is_in_the_menu() {
         let mut harness = settings_sheet(crate::theme::Look::standard());
@@ -1546,21 +1622,21 @@ value_tags   = true
             harness.press(egui::Key::Comma, egui::Modifiers::COMMAND);
             // The last row's switch: the window has room for one row or
             // two, and it is under them.
-            tab_to_settings(&mut harness, "Value tags");
+            let last = "New production connections open read-only";
+            tab_to_settings(&mut harness, last);
             let tree = harness.settle();
-            let switch =
-                crate::testing::bounds(&tree, "Value tags", Role::CheckBox).expect("the switch");
+            let switch = crate::testing::bounds(&tree, last, Role::CheckBox).expect("the switch");
             // The rows came with the keyboard: the switch is in the
             // window, between the rows' heading and the footer.
             let window = egui::Rect::from_min_size(egui::Pos2::ZERO, size);
             assert!(window.contains_rect(switch), "{switch:?}");
             let stored = harness.painted_rect("Stored in").expect("the footer");
             assert!(switch.bottom() < stored.top(), "{switch:?} {stored:?}");
-            let lead = "How Tabletist shows data. Changes apply right away.";
+            let lead = "How Tabletist shows data and protects your work. Changes apply right away.";
             let lead = harness.painted_rect(lead).expect("the tab's lead");
             assert!(switch.top() > lead.bottom(), "{switch:?} {lead:?}");
             // And back up with it: the first row's menu.
-            for _ in 0..4 {
+            for _ in 0..5 {
                 harness.press(egui::Key::Tab, egui::Modifiers::SHIFT);
             }
             let tree = harness.settle();
@@ -7708,6 +7784,44 @@ value_tags   = true
         );
     }
 
+    #[test]
+    fn a_new_production_connection_starts_its_switch_where_the_settings_say() {
+        for on in [false, true] {
+            let mut harness = Harness::new();
+            harness.app.settings.production_read_only = on;
+            new_connection(&mut harness);
+            harness.click("Production");
+            assert_eq!(form(&harness).read_only(), on);
+            // The box shows it.
+            let tree = harness.settle();
+            let id = crate::testing::node(&tree, "Open read-only", egui::accesskit::Role::CheckBox)
+                .expect("the box");
+            let toggled = tree
+                .nodes
+                .iter()
+                .find(|(node, _)| *node == id)
+                .and_then(|(_, data)| data.toggled());
+            let expected = if on {
+                egui::accesskit::Toggled::True
+            } else {
+                egui::accesskit::Toggled::False
+            };
+            assert_eq!(toggled, Some(expected));
+        }
+        // A saved connection's dialog keeps the rule.
+        let mut harness = Harness::new();
+        harness.app.settings.production_read_only = false;
+        let mut saved = crate::testing::fixture_connection();
+        saved.environment = crate::env::Environment::Production;
+        saved.read_only = None;
+        let conn = saved.id.clone();
+        harness.app.connections.upsert(saved);
+        harness
+            .app
+            .apply(crate::model::Action::EditConnection(conn));
+        assert!(form(&harness).read_only());
+    }
+
     #[test]
     fn choosing_an_environment_sets_the_connections() {
         let mut harness = Harness::new();
````

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --locked -p tabletist --lib`
Expected: FAIL to compile: `` struct `model::ConnectionForm` has no field named `production_read_only` ``; `` struct `settings::Settings` has no field named `production_read_only` ``; `` no variant, associated function, or constant named `ProductionReadOnly` found for enum `settings::Key` in the current scope ``; `` no field `production_read_only` on type `settings::Settings` ``, and 1 more.

- [ ] **Step 3: Write the code**

````diff
diff --git a/docs/_reference/connections.md b/docs/_reference/connections.md
index 190b1df..93ceb30 100644
--- a/docs/_reference/connections.md
+++ b/docs/_reference/connections.md
@@ -204,7 +204,9 @@ SQLite file is opened read-only.
 
 The box is ticked by default for production, and not for the other
 environments. Turn it off to edit. A change applies from the next time
-you connect.
+you connect. Where a new production connection's box starts is
+**New production connections open read-only** in
+[Settings]({% link _reference/settings-and-files.md %}#the-settings-window).
 
 Production stays careful with the box off too:
 
diff --git a/docs/_reference/settings-and-files.md b/docs/_reference/settings-and-files.md
index af4e10e..d6932c3 100644
--- a/docs/_reference/settings-and-files.md
+++ b/docs/_reference/settings-and-files.md
@@ -17,6 +17,7 @@ Cmd/Ctrl+, opens Settings. Changes apply right away.
 | **Time zone for timestamptz** | System, UTC or any zone. A `timestamptz` is shown in this zone, a copy gives the value as the server sent it, and a value you type without an offset is sent with this zone's. In the Omarchy look `h`/`l` choose System, UTC or the last zone the file named; any other zone is set in the file | System |
 | **Numbers** | `1,240.50` or `1240.50`. Grouping is for display only: a copy gives the raw value | `1,240.50` on macOS and Windows, `1240.50` in the Omarchy look |
 | **Value tags** | Colors for enum, CHECK and boolean columns | On |
+| **New production connections open read-only** | On or off: where the **Open read-only** box of a new production connection starts. A connection you saved keeps its own box | On |
 
 A second pane, **Agents**, sets what the `[mcp]` keys
 hold, and shows how to connect an agent:
@@ -51,6 +52,9 @@ timestamps = "second"
 tz = "system"
 value_tags = true
 
+[safety]
+production_read_only = true
+
 [sidebar]
 show_system_schemas = false
 
@@ -71,6 +75,7 @@ notify = true
 | `data.tz` | `"system"`, `"utc"` or a zone such as `"America/New_York"`. A name the zone database does not know is a line that cannot be read. A `timestamptz` value is shown and edited in this zone, a copy gives it as the server sent it, and one you type without an offset is sent with this zone's | `"system"` |
 | `data.group_digits` | `true` or `false` | Not set: grouped on macOS and Windows, not in the Omarchy look |
 | `data.value_tags` | `true` or `false` | `true` |
+| `safety.production_read_only` | `true` or `false`. Where the **Open read-only** box of a new production connection starts; the connection is saved with its box set, so a change here changes no saved connection | `true` |
 | `sidebar.show_system_schemas` | `true` shows schemas such as `pg_catalog` and `information_schema` | `false` |
 | `editor.sql_limit` | 1 to 10000 rows for each statement | `1000` |
 | `editor.sql_timeout_secs` | Seconds. `0` waits forever | `30` |
diff --git a/src/app.rs b/src/app.rs
index 5444e6d..c81e4a7 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -2047,7 +2047,10 @@ impl App {
             }
             Action::Backend(event) => self.apply_event(event),
             Action::NewConnection => {
-                self.dialog = Some(Dialog::Connection(Box::default()));
+                self.dialog = Some(Dialog::Connection(Box::new(ConnectionForm {
+                    production_read_only: self.settings.production_read_only,
+                    ..ConnectionForm::default()
+                })));
                 self.list_ssh_hosts();
             }
             Action::EditConnection(id) => {
diff --git a/src/model.rs b/src/model.rs
index ff16c81..45bf9c5 100644
--- a/src/model.rs
+++ b/src/model.rs
@@ -1285,6 +1285,10 @@ pub struct ConnectionForm {
     pub environment: Option<crate::env::Environment>,
     /// Read-only as saved: `None` until the user sets it.
     pub read_only: Option<bool>,
+    /// Whether production is read-only while the switch is not set: the
+    /// settings' "New production connections open read-only" for a new
+    /// connection, the built-in rule (yes) for a saved one.
+    pub production_read_only: bool,
     /// What agents may do on the connection: its access as the person
     /// chooses it here. Rows and the timeout are typed in the two fields
     /// below.
@@ -1426,6 +1430,7 @@ impl Default for ConnectionForm {
             name: String::new(),
             environment: None,
             read_only: None,
+            production_read_only: true,
             agents: crate::connections::Agents::default(),
             agents_opened: crate::connections::AgentAccess::Off,
             agents_tables: TablesChoice::default(),
@@ -1565,10 +1570,23 @@ impl ConnectionForm {
     }
 
     /// Whether the connection is saved read-only: as set, else the
-    /// environment's default.
+    /// environment's default, which for production is
+    /// [`Self::production_read_only`].
     pub fn read_only(&self) -> bool {
+        self.read_only.unwrap_or_else(|| {
+            // Production is the one environment read-only by default.
+            self.environment().read_only_by_default() && self.production_read_only
+        })
+    }
+
+    /// The switch as the connection is saved with it: as set; for a new
+    /// connection to production, as it shows, so that it keeps what the
+    /// settings said when it was made; else not set, and the environment's
+    /// rule applies.
+    fn saved_read_only(&self) -> Option<bool> {
+        let new_production = self.editing.is_none() && self.environment().read_only_by_default();
         self.read_only
-            .unwrap_or(self.environment().read_only_by_default())
+            .or_else(|| new_production.then(|| self.read_only()))
     }
 
     pub fn from_saved(saved: &crate::connections::SavedConnection) -> Self {
@@ -1578,6 +1596,7 @@ impl ConnectionForm {
             name: saved.name.clone(),
             environment: Some(saved.environment),
             read_only: saved.read_only,
+            production_read_only: true,
             agents: saved.agents.clone(),
             agents_opened: saved.agents.access,
             agents_tables: TablesChoice::of(&saved.agents.tables),
@@ -1841,7 +1860,7 @@ impl ConnectionForm {
             id: self.editing.clone().unwrap_or_else(ConnectionId::new),
             name: name.to_owned(),
             environment: self.environment(),
-            read_only: self.read_only,
+            read_only: self.saved_read_only(),
             password,
             ssh_secret,
             spec,
@@ -2167,6 +2186,7 @@ impl SettingsPane {
                 SettingsRow::Option(OptionId::Tz),
                 SettingsRow::Option(OptionId::GroupDigits),
                 SettingsRow::Option(OptionId::ValueTags),
+                SettingsRow::Option(OptionId::ProductionReadOnly),
             ],
             Self::Agents => &[
                 SettingsRow::Option(OptionId::AgentsOn),
diff --git a/src/settings.rs b/src/settings.rs
index e7b4136..4af4d4f 100644
--- a/src/settings.rs
+++ b/src/settings.rs
@@ -112,6 +112,10 @@ pub struct Settings {
     pub group_digits: Option<bool>,
     /// Enum, CHECK and boolean values drawn as coloured tags.
     pub value_tags: bool,
+    /// Where a new connection to production starts its "Open read-only"
+    /// switch, which it is saved with. A saved connection whose switch was
+    /// never set keeps the built-in rule: production is read-only.
+    pub production_read_only: bool,
     /// Show `pg_catalog`, `information_schema`, `mysql`, `sys` and friends.
     pub show_system_schemas: bool,
     /// A palette file name in the themes directory. `None` follows the
@@ -140,6 +144,7 @@ pub enum Key {
     Tz,
     GroupDigits,
     ValueTags,
+    ProductionReadOnly,
     ShowSystemSchemas,
     SqlLimit,
     SqlTimeoutSecs,
@@ -151,12 +156,13 @@ pub enum Key {
 
 impl Key {
     /// In the order the file has them: a table's keys together.
-    pub const ALL: [Key; 12] = [
+    pub const ALL: [Key; 13] = [
         Self::PageSize,
         Self::Timestamps,
         Self::Tz,
         Self::GroupDigits,
         Self::ValueTags,
+        Self::ProductionReadOnly,
         Self::ShowSystemSchemas,
         Self::SqlLimit,
         Self::SqlTimeoutSecs,
@@ -174,6 +180,7 @@ impl Key {
             Self::Tz => ("data", "tz"),
             Self::GroupDigits => ("data", "group_digits"),
             Self::ValueTags => ("data", "value_tags"),
+            Self::ProductionReadOnly => ("safety", "production_read_only"),
             Self::ShowSystemSchemas => ("sidebar", "show_system_schemas"),
             Self::SqlLimit => ("editor", "sql_limit"),
             Self::SqlTimeoutSecs => ("editor", "sql_timeout_secs"),
@@ -205,6 +212,9 @@ pub enum OptionId {
     Tz,
     GroupDigits,
     ValueTags,
+    /// Where a new production connection's read-only switch starts:
+    /// `safety.production_read_only`.
+    ProductionReadOnly,
     /// Whether agents can reach the app: `mcp.enabled`.
     AgentsOn,
     /// How long the log of agents' calls keeps one: `mcp.log_days`.
@@ -215,23 +225,25 @@ pub enum OptionId {
 
 impl OptionId {
     /// Every option, the General pane's and then the Agents pane's.
-    pub const ALL: [OptionId; 8] = [
+    pub const ALL: [OptionId; 9] = [
         Self::PageSize,
         Self::Timestamps,
         Self::Tz,
         Self::GroupDigits,
         Self::ValueTags,
+        Self::ProductionReadOnly,
         Self::AgentsOn,
         Self::LogDays,
         Self::AgentsNotify,
     ];
     /// The General pane's, in the order it has them.
-    pub const GENERAL: [OptionId; 5] = [
+    pub const GENERAL: [OptionId; 6] = [
         Self::PageSize,
         Self::Timestamps,
         Self::Tz,
         Self::GroupDigits,
         Self::ValueTags,
+        Self::ProductionReadOnly,
     ];
     /// The Agents pane's, in the order it has them.
     pub const AGENTS: [OptionId; 3] = [Self::AgentsOn, Self::LogDays, Self::AgentsNotify];
@@ -244,6 +256,7 @@ impl OptionId {
             Self::Tz => Key::Tz,
             Self::GroupDigits => Key::GroupDigits,
             Self::ValueTags => Key::ValueTags,
+            Self::ProductionReadOnly => Key::ProductionReadOnly,
             Self::AgentsOn => Key::McpEnabled,
             Self::LogDays => Key::McpLogDays,
             Self::AgentsNotify => Key::McpNotify,
@@ -265,6 +278,7 @@ pub enum OptionValue {
     /// `None` is the look's default.
     GroupDigits(Option<bool>),
     ValueTags(bool),
+    ProductionReadOnly(bool),
     AgentsOn(bool),
     LogDays(u32),
     AgentsNotify(bool),
@@ -278,6 +292,7 @@ impl OptionValue {
             Self::Tz(_) => OptionId::Tz,
             Self::GroupDigits(_) => OptionId::GroupDigits,
             Self::ValueTags(_) => OptionId::ValueTags,
+            Self::ProductionReadOnly(_) => OptionId::ProductionReadOnly,
             Self::AgentsOn(_) => OptionId::AgentsOn,
             Self::LogDays(_) => OptionId::LogDays,
             Self::AgentsNotify(_) => OptionId::AgentsNotify,
@@ -292,6 +307,7 @@ impl OptionValue {
             Self::Tz(zone) => settings.tz = zone,
             Self::GroupDigits(on) => settings.group_digits = on,
             Self::ValueTags(on) => settings.value_tags = on,
+            Self::ProductionReadOnly(on) => settings.production_read_only = on,
             Self::AgentsOn(on) => settings.mcp_enabled = on,
             Self::LogDays(days) => settings.mcp_log_days = days,
             Self::AgentsNotify(on) => settings.mcp_notify = on,
@@ -746,6 +762,9 @@ impl Settings {
             OptionId::Tz => OptionValue::Tz(self.tz.clone()),
             OptionId::GroupDigits => OptionValue::GroupDigits(self.group_digits),
             OptionId::ValueTags => OptionValue::ValueTags(self.value_tags),
+            OptionId::ProductionReadOnly => {
+                OptionValue::ProductionReadOnly(self.production_read_only)
+            }
             OptionId::AgentsOn => OptionValue::AgentsOn(self.mcp_enabled),
             OptionId::LogDays => OptionValue::LogDays(self.mcp_log_days),
             OptionId::AgentsNotify => OptionValue::AgentsNotify(self.mcp_notify),
@@ -781,6 +800,7 @@ impl Settings {
             // Grouped is the left of the two.
             OptionId::GroupDigits => OptionValue::GroupDigits(Some(!forward)),
             OptionId::ValueTags => OptionValue::ValueTags(forward),
+            OptionId::ProductionReadOnly => OptionValue::ProductionReadOnly(forward),
             OptionId::AgentsOn => OptionValue::AgentsOn(forward),
             OptionId::LogDays => {
                 let days = self.mcp_log_days;
@@ -829,6 +849,9 @@ impl Settings {
                 !self.group_digits.unwrap_or_default(),
             ))),
             OptionId::ValueTags => Some(OptionValue::ValueTags(!self.value_tags)),
+            OptionId::ProductionReadOnly => {
+                Some(OptionValue::ProductionReadOnly(!self.production_read_only))
+            }
             OptionId::AgentsOn => Some(OptionValue::AgentsOn(!self.mcp_enabled)),
             OptionId::LogDays => None,
             OptionId::AgentsNotify => Some(OptionValue::AgentsNotify(!self.mcp_notify)),
@@ -844,6 +867,7 @@ impl Settings {
             Key::Tz => toml_string(self.tz.name()),
             Key::GroupDigits => self.group_digits?.to_string(),
             Key::ValueTags => self.value_tags.to_string(),
+            Key::ProductionReadOnly => self.production_read_only.to_string(),
             Key::ShowSystemSchemas => self.show_system_schemas.to_string(),
             Key::SqlLimit => self.sql_limit.to_string(),
             // TOML has no null: no timeout is no seconds.
@@ -1064,6 +1088,7 @@ impl Settings {
             Key::Tz => text.and_then(Zone::from_name).map(|zone| self.tz = zone),
             Key::GroupDigits => flag.map(|flag| self.group_digits = Some(flag)),
             Key::ValueTags => flag.map(|flag| self.value_tags = flag),
+            Key::ProductionReadOnly => flag.map(|flag| self.production_read_only = flag),
             Key::ShowSystemSchemas => flag.map(|flag| self.show_system_schemas = flag),
             Key::SqlLimit => number.map(|number| self.sql_limit = number),
             Key::SqlTimeoutSecs => number.map(|number| self.sql_timeout_secs = Some(number)),
@@ -1172,6 +1197,7 @@ impl Settings {
             Key::Tz => self.tz = other.tz.clone(),
             Key::GroupDigits => self.group_digits = other.group_digits,
             Key::ValueTags => self.value_tags = other.value_tags,
+            Key::ProductionReadOnly => self.production_read_only = other.production_read_only,
             Key::ShowSystemSchemas => self.show_system_schemas = other.show_system_schemas,
             Key::SqlLimit => self.sql_limit = other.sql_limit,
             Key::SqlTimeoutSecs => self.sql_timeout_secs = other.sql_timeout_secs,
@@ -1229,6 +1255,7 @@ impl Default for Settings {
             tz: Zone::System,
             group_digits: None,
             value_tags: true,
+            production_read_only: true,
             show_system_schemas: false,
             custom_theme: None,
             sql_limit: 1_000,
diff --git a/src/ui/settings/mod.rs b/src/ui/settings/mod.rs
index d073ac6..97d4cc8 100644
--- a/src/ui/settings/mod.rs
+++ b/src/ui/settings/mod.rs
@@ -27,6 +27,7 @@ pub(super) fn label(option: OptionId) -> &'static str {
         OptionId::Tz => "Time zone for timestamptz",
         OptionId::GroupDigits => "Numbers",
         OptionId::ValueTags => "Value tags",
+        OptionId::ProductionReadOnly => "New production connections open read-only",
         OptionId::AgentsOn => "Let agents use Tabletist",
         OptionId::LogDays => "Keep the activity log",
         OptionId::AgentsNotify => "Notify when an agent connects",
@@ -39,6 +40,7 @@ pub(super) fn label(option: OptionId) -> &'static str {
 pub(super) fn terminal_label(option: OptionId) -> &'static str {
     match option {
         OptionId::Tz => "timestamptz zone",
+        OptionId::ProductionReadOnly => "production opens read-only",
         OptionId::PageSize
         | OptionId::Timestamps
         | OptionId::GroupDigits
@@ -49,6 +51,23 @@ pub(super) fn terminal_label(option: OptionId) -> &'static str {
     }
 }
 
+/// The heading of the group of the General pane that starts at
+/// `option`'s row, where one starts there. English: the caller translates,
+/// and the terminal look lower-cases.
+pub(super) fn group_starting(option: OptionId) -> Option<&'static str> {
+    match option {
+        OptionId::PageSize => Some("Data"),
+        OptionId::ProductionReadOnly => Some("Safety"),
+        OptionId::Timestamps
+        | OptionId::Tz
+        | OptionId::GroupDigits
+        | OptionId::ValueTags
+        | OptionId::AgentsOn
+        | OptionId::LogDays
+        | OptionId::AgentsNotify => None,
+    }
+}
+
 /// What a row says under its label, where it says something. English.
 pub(super) fn small_print(option: OptionId) -> Option<&'static str> {
     match option {
@@ -56,6 +75,7 @@ pub(super) fn small_print(option: OptionId) -> Option<&'static str> {
         OptionId::ValueTags => Some("Colors for enum, CHECK (…IN…) and boolean columns"),
         OptionId::LogDays => Some("Every call and its SQL, never the rows. On this computer only"),
         OptionId::Tz => Some("Values are shown in this zone and sent with their offset"),
+        OptionId::ProductionReadOnly => Some("Default for the switch in each connection"),
         OptionId::Timestamps
         | OptionId::GroupDigits
         | OptionId::AgentsOn
@@ -103,6 +123,7 @@ pub(super) fn is_on(value: &OptionValue) -> bool {
     matches!(
         value,
         OptionValue::ValueTags(true)
+            | OptionValue::ProductionReadOnly(true)
             | OptionValue::AgentsOn(true)
             | OptionValue::AgentsNotify(true)
     )
@@ -188,6 +209,10 @@ pub(super) fn choices(
             (Word("off"), OptionValue::ValueTags(false)),
             (Word("on"), OptionValue::ValueTags(true)),
         ],
+        OptionId::ProductionReadOnly => vec![
+            (Word("off"), OptionValue::ProductionReadOnly(false)),
+            (Word("on"), OptionValue::ProductionReadOnly(true)),
+        ],
         OptionId::AgentsOn => vec![
             (Word("off"), OptionValue::AgentsOn(false)),
             (Word("on"), OptionValue::AgentsOn(true)),
diff --git a/src/ui/settings/sheet.rs b/src/ui/settings/sheet.rs
index b89a636..893bd7c 100644
--- a/src/ui/settings/sheet.rs
+++ b/src/ui/settings/sheet.rs
@@ -18,7 +18,7 @@ use crate::ui::menu;
 use crate::ui::value_tags::slot_colors;
 use crate::ui::widgets::{self, Segment};
 
-use super::{label, path_shown, sample_timestamp, small_print, zone_text, zones};
+use super::{group_starting, label, path_shown, sample_timestamp, small_print, zone_text, zones};
 
 /// The sheet at its widest and at its tallest, and what it leaves of the
 /// window at each side.
@@ -256,8 +256,8 @@ fn nav(
     }
 }
 
-/// The tab: its title, what it is for, and its options under the heading
-/// of their group. The rows end at `limit` at the latest, and scroll when
+/// The tab: its title, what it is for, and its options under the headings
+/// of their groups. The rows end at `limit` at the latest, and scroll when
 /// they are taller than that leaves them.
 fn content(ui: &mut Ui, settings: &Settings, limit: f32, skin: &Skin, actions: &mut Vec<Action>) {
     let Skin { look, palette, .. } = *skin;
@@ -272,15 +272,11 @@ fn content(ui: &mut Ui, settings: &Settings, limit: f32, skin: &Skin, actions: &
     widgets::label(
         ui,
         TextRole::UiBody,
-        &skin.say("How Tabletist shows data. Changes apply right away."),
+        &skin.say("How Tabletist shows data and protects your work. Changes apply right away."),
         palette.dim,
         look,
     );
-    ui.add_space(6.0 + 14.0);
-    widgets::section_label(&skin.say("Data"), look, palette)
-        .layout(ui.ctx())
-        .label(ui);
-    ui.add_space(2.0);
+    ui.add_space(6.0);
     let column = label_column(ui, &OptionId::GENERAL, skin);
     let room = (limit - ui.cursor().top()).max(ROWS_MIN);
     egui::ScrollArea::vertical()
@@ -292,6 +288,13 @@ fn content(ui: &mut Ui, settings: &Settings, limit: f32, skin: &Skin, actions: &
             // right over that would read as a double line.
             let last = OptionId::GENERAL.len() - 1;
             for (index, option) in OptionId::GENERAL.into_iter().enumerate() {
+                if let Some(heading) = group_starting(option) {
+                    ui.add_space(14.0);
+                    widgets::section_label(&skin.say(heading), look, palette)
+                        .layout(ui.ctx())
+                        .label(ui);
+                    ui.add_space(2.0);
+                }
                 row(
                     ui,
                     option,
@@ -345,12 +348,15 @@ pub(super) fn row(
     after: Option<After<'_>>,
 ) {
     let Skin { look, palette, .. } = *skin;
+    // Wrapped in its column too: a narrow sheet takes from the labels'
+    // column, and a long label would run under the control.
     let name = Text::one(
         look,
         TextRole::UiBody,
         &skin.say(label(option)),
         palette.text,
     )
+    .wrap(column)
     .layout(ui.ctx());
     let print = small_print(option).map(|print| {
         Text::one(look, TextRole::Secondary, &skin.say(print), palette.dim)
@@ -426,7 +432,10 @@ fn control_height(option: OptionId, look: &Look) -> f32 {
         OptionId::PageSize | OptionId::Tz => look.control_height,
         // The track has 2 over its segments and under them.
         OptionId::Timestamps | OptionId::GroupDigits => SEGMENT + 4.0,
-        OptionId::ValueTags | OptionId::AgentsOn | OptionId::AgentsNotify => SWITCH,
+        OptionId::ValueTags
+        | OptionId::ProductionReadOnly
+        | OptionId::AgentsOn
+        | OptionId::AgentsNotify => SWITCH,
         OptionId::LogDays => look.control_height,
     }
 }
@@ -493,7 +502,10 @@ fn control(
                 ))));
             }
         }
-        OptionId::ValueTags | OptionId::AgentsOn | OptionId::AgentsNotify => {
+        OptionId::ValueTags
+        | OptionId::ProductionReadOnly
+        | OptionId::AgentsOn
+        | OptionId::AgentsNotify => {
             let on = super::is_on(&settings.value(option));
             if widgets::toggle(ui, on, &name, palette).clicked()
                 && let Some(flipped) = settings.flipped(option)
@@ -650,7 +662,10 @@ fn hint(ui: &Ui, at: egui::Pos2, right: f32, option: OptionId, settings: &Settin
             palette.dim,
         ),
         OptionId::ValueTags => return tags(ui, at, right, settings.value_tags, skin),
-        OptionId::AgentsOn | OptionId::LogDays | OptionId::AgentsNotify => return,
+        OptionId::ProductionReadOnly
+        | OptionId::AgentsOn
+        | OptionId::LogDays
+        | OptionId::AgentsNotify => return,
     };
     let text = text.layout(ui.ctx());
     if at.x + text.width() <= right {
diff --git a/src/ui/settings/terminal.rs b/src/ui/settings/terminal.rs
index a5ce4dc..e3ce60d 100644
--- a/src/ui/settings/terminal.rs
+++ b/src/ui/settings/terminal.rs
@@ -18,7 +18,9 @@ use crate::ui::focus::{self, Ring};
 use crate::ui::value_tags::terminal_slots;
 use crate::ui::widgets::{self, ButtonSpec};
 
-use super::{Said, choices, file_pane, label, opened, sample_timestamp, terminal_label};
+use super::{
+    Said, choices, file_pane, group_starting, label, opened, sample_timestamp, terminal_label,
+};
 
 /// The header's and the footer's heights, each without its rule.
 const HEADER: f32 = 40.0;
@@ -304,10 +306,9 @@ fn heading(ui: &mut Ui, text: &'static str, top: f32, skin: &Skin) {
     );
 }
 
-/// The General pane's options, one to a row, under the heading of their
-/// group.
+/// The General pane's options, one to a row, under the headings of their
+/// groups.
 fn rows(ui: &mut Ui, settings: &Settings, cursor: usize, skin: &Skin, actions: &mut Vec<Action>) {
-    heading(ui, "Data", 8.0, skin);
     let room = ui.available_width() - 2.0 * ROW_SIDE;
     let columns = columns(
         ui,
@@ -316,6 +317,10 @@ fn rows(ui: &mut Ui, settings: &Settings, cursor: usize, skin: &Skin, actions: &
         skin,
     );
     for (index, option) in OptionId::GENERAL.into_iter().enumerate() {
+        if let Some(text) = group_starting(option) {
+            // The pane's first group stands 8 under its top.
+            heading(ui, text, if index == 0 { 8.0 } else { 0.0 }, skin);
+        }
         option_row(
             ui,
             (index, option),
@@ -677,7 +682,10 @@ fn value_width(ui: &Ui, option: OptionId, settings: &Settings, skin: &Skin) -> f
             let segments = words.into_iter().map(|(word, _)| segment(ui, word, skin));
             segments.sum::<f32>() + between
         }
-        OptionId::ValueTags | OptionId::AgentsOn | OptionId::AgentsNotify => width("[x]"),
+        OptionId::ValueTags
+        | OptionId::ProductionReadOnly
+        | OptionId::AgentsOn
+        | OptionId::AgentsNotify => width("[x]"),
     }
 }
 
@@ -835,7 +843,10 @@ fn option_row(
         OptionId::Timestamps | OptionId::GroupDigits => {
             segments(ui, cell, option, settings, skin, actions);
         }
-        OptionId::ValueTags | OptionId::AgentsOn | OptionId::AgentsNotify => {
+        OptionId::ValueTags
+        | OptionId::ProductionReadOnly
+        | OptionId::AgentsOn
+        | OptionId::AgentsNotify => {
             check(ui, cell, option, settings, skin, actions);
         }
     }
@@ -978,6 +989,7 @@ fn hint(ui: &Ui, option: OptionId, settings: &Settings, skin: &Skin) -> Text {
                 .space(role, "  ")
                 .add(role, "ebook", second)
         }
+        OptionId::ProductionReadOnly => dim(&skin.say("default for new connections")),
         OptionId::AgentsOn => dim(&skin.say("while tabletist is open")),
         // The key that shows the log, from this row, where there is a
         // window to show it in.
````

- [ ] **Step 4: Check**

Run: `cargo fmt --all && cargo clippy --locked --workspace --all-targets -- -D warnings && cargo test --locked --workspace --all-targets`
Expected: no warning, every test passes.

- [ ] **Step 5: Commit**

```bash
git add docs/_reference/connections.md docs/_reference/settings-and-files.md src/app.rs src/model.rs src/settings.rs src/ui/mod.rs src/ui/settings/mod.rs src/ui/settings/sheet.rs src/ui/settings/terminal.rs
git commit -m "Add the Safety group to Settings › General, and start a new production connection read-only as it says"
```

### Task 2: Ask before dropping unsaved edits only where the settings say to

The key `safety.confirm_close_edits` and its row. Off, `App::hold` drops the edits as Discard does and the held action goes ahead; a running save and an open dialog hold as before.

> **Merged as igor-alexandrov/tabletist#160; the merged code supersedes this task's diffs.** Rebased on the delete counts of main, `discard_edits` (`src/app/editing.rs`) also takes each tab's `edits.counts_out()` and stops them with `stop_counts`, as Discard does, and `src/ui/delete_tests.rs` has the test `with_the_question_off_closing_stops_what_the_deletes_were_counting`. Neither is in the diffs below.

**Files:** Modify `docs/_guide/editing-data.md`; Modify `docs/_reference/settings-and-files.md`; Modify `src/app.rs`; Modify `src/app/editing.rs`; Modify `src/model.rs`; Modify `src/settings.rs`; Modify `src/ui/mod.rs`; Modify `src/ui/settings/mod.rs`; Modify `src/ui/settings/sheet.rs`; Modify `src/ui/settings/terminal.rs`.

- [ ] **Step 1: Write the failing tests**

````diff
diff --git a/src/settings.rs b/src/settings.rs
index 4af4d4f..0e4c45e 100644
--- a/src/settings.rs
+++ b/src/settings.rs
@@ -1312,6 +1312,7 @@ mod tests {
             group_digits: Some(true),
             value_tags: false,
             production_read_only: false,
+            confirm_close_edits: false,
             show_system_schemas: true,
             custom_theme: Some("My \"Nord\".json".into()),
             sql_limit: 100,
@@ -1337,6 +1338,7 @@ value_tags = false
 
 [safety]
 production_read_only = false
+confirm_close_edits = false
 
 [sidebar]
 show_system_schemas = true
@@ -1412,13 +1414,14 @@ theme = \"My \\\"Nord\\\".json\"
                 (Key::GroupDigits, 6),
                 (Key::ValueTags, 7),
                 (Key::ProductionReadOnly, 10),
-                (Key::ShowSystemSchemas, 13),
-                (Key::SqlLimit, 16),
-                (Key::SqlTimeoutSecs, 17),
-                (Key::McpEnabled, 20),
-                (Key::McpLogDays, 21),
-                (Key::McpNotify, 22),
-                (Key::Theme, 25),
+                (Key::ConfirmCloseEdits, 11),
+                (Key::ShowSystemSchemas, 14),
+                (Key::SqlLimit, 17),
+                (Key::SqlTimeoutSecs, 18),
+                (Key::McpEnabled, 21),
+                (Key::McpLogDays, 22),
+                (Key::McpNotify, 23),
+                (Key::Theme, 26),
             ]
         );
     }
@@ -2369,6 +2372,7 @@ theme = \"My \\\"Nord\\\".json\"
                 Key::GroupDigits,
                 Key::ValueTags,
                 Key::ProductionReadOnly,
+                Key::ConfirmCloseEdits,
                 Key::McpEnabled,
                 Key::McpLogDays,
                 Key::McpNotify,
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
index daa4a27..cedc5c4 100644
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -1362,6 +1362,10 @@ value_tags   = true
                 text.ends_with("[safety]\nproduction_read_only = false\n"),
                 "{text}"
             );
+            harness.click("Ask before closing with unsaved edits");
+            assert!(!harness.app.settings.confirm_close_edits);
+            let text = &harness.app.settings_file.text;
+            assert!(text.ends_with("confirm_close_edits = false\n"), "{text}");
         }
     }
 
@@ -1393,6 +1397,7 @@ value_tags   = true
             "safety",
             "production opens read-only",
             "default for new connections",
+            "confirm close with edits",
         ] {
             assert!(harness.painted_color(text).is_some(), "{text}");
         }
@@ -1622,7 +1627,7 @@ value_tags   = true
             harness.press(egui::Key::Comma, egui::Modifiers::COMMAND);
             // The last row's switch: the window has room for one row or
             // two, and it is under them.
-            let last = "New production connections open read-only";
+            let last = "Ask before closing with unsaved edits";
             tab_to_settings(&mut harness, last);
             let tree = harness.settle();
             let switch = crate::testing::bounds(&tree, last, Role::CheckBox).expect("the switch");
@@ -1636,7 +1641,7 @@ value_tags   = true
             let lead = harness.painted_rect(lead).expect("the tab's lead");
             assert!(switch.top() > lead.bottom(), "{switch:?} {lead:?}");
             // And back up with it: the first row's menu.
-            for _ in 0..5 {
+            for _ in 0..6 {
                 harness.press(egui::Key::Tab, egui::Modifiers::SHIFT);
             }
             let tree = harness.settle();
@@ -19344,6 +19349,83 @@ value_tags   = true
         }
     }
 
+    #[test]
+    fn with_the_question_off_the_edits_go_and_what_was_asked_for_happens() {
+        use egui::ViewportCommand::Close;
+        for look in Look::ALL {
+            let (mut harness, tab, id) = editable_in(look);
+            harness.app.settings.confirm_close_edits = false;
+            // A reload of the page: no question, and the page comes again
+            // without the change.
+            make_pending(&mut harness, tab, id, (1, 1), "bob@example.com");
+            let before = fetches(&harness);
+            harness.app.apply(Action::Refresh(tab));
+            assert!(!leaving(&harness), "{}", look.name);
+            assert!(!edits(&harness, tab, id).holds(), "{}", look.name);
+            assert_eq!(fetches(&harness), before + 1, "{}", look.name);
+            harness.answer_structure(crate::testing::fixture_structure());
+            harness.answer_rows(crate::testing::page(5, false));
+            // The window: it closes.
+            make_pending(&mut harness, tab, id, (2, 1), "cy@example.com");
+            ask_to_close(&mut harness);
+            assert!(!leaving(&harness), "{}", look.name);
+            assert!(harness.app.closing, "{}", look.name);
+            assert!(sends(&harness, Close), "{}", look.name);
+            // The tab: it closes.
+            let (mut harness, tab, id) = editable_in(look);
+            harness.app.settings.confirm_close_edits = false;
+            make_pending(&mut harness, tab, id, (1, 1), "bob@example.com");
+            harness.app.apply(Action::CloseTab { tab, id });
+            assert!(!leaving(&harness), "{}", look.name);
+            assert!(!open(&harness, tab, id), "{}", look.name);
+        }
+    }
+
+    #[test]
+    fn with_the_question_off_a_dialog_about_the_edits_still_holds_them() {
+        for look in Look::ALL {
+            let (mut harness, tab, id) = editable_in(look);
+            harness.app.settings.confirm_close_edits = false;
+            harness.app.workspace_mut(tab).unwrap().environment =
+                crate::env::Environment::Production;
+            make_pending(&mut harness, tab, id, (1, 1), "bob@example.com");
+            // The save's confirmation, which a production save asks first.
+            harness.app.apply(Action::WriteEdits { tab, id });
+            let confirming = |harness: &Harness| {
+                matches!(
+                    harness.app.dialog,
+                    Some(crate::model::Dialog::ConfirmWrite(_))
+                )
+            };
+            assert!(confirming(&harness), "{}", look.name);
+            // A reload under it, as the key runs while a dialog is open:
+            // the edits the confirmation shows stay, as with the question on.
+            harness.app.apply(Action::Refresh(tab));
+            assert!(confirming(&harness), "{}", look.name);
+            assert!(edits(&harness, tab, id).holds(), "{}", look.name);
+            assert!(harness.app.notice.is_some(), "{}", look.name);
+        }
+    }
+
+    #[test]
+    fn with_the_question_off_a_save_that_runs_still_holds() {
+        for look in Look::ALL {
+            let (mut harness, tab, id) = editable_in(look);
+            harness.app.settings.confirm_close_edits = false;
+            make_pending(&mut harness, tab, id, (1, 1), "bob@example.com");
+            harness.app.apply(Action::WriteEdits { tab, id });
+            assert_eq!(writes(&harness), 1, "{}", look.name);
+            // The tab stays, with its set, as it does with the question on.
+            harness.app.apply(Action::CloseTab { tab, id });
+            assert!(open(&harness, tab, id), "{}", look.name);
+            assert!(edits(&harness, tab, id).saving.is_some(), "{}", look.name);
+            // The window asks: a save cannot be dropped without a word.
+            ask_to_close(&mut harness);
+            assert!(leaving(&harness), "{}", look.name);
+            assert!(!harness.app.closing, "{}", look.name);
+        }
+    }
+
     #[test]
     fn saving_before_the_window_closes_closes_it_once_everything_is_written() {
         use egui::ViewportCommand::{CancelClose, Close};
````

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --locked -p tabletist --lib`
Expected: FAIL to compile: `` struct `settings::Settings` has no field named `confirm_close_edits` ``; `` no variant, associated function, or constant named `ConfirmCloseEdits` found for enum `settings::Key` in the current scope ``; `` no field `confirm_close_edits` on type `settings::Settings` ``.

- [ ] **Step 3: Write the code**

````diff
diff --git a/docs/_guide/editing-data.md b/docs/_guide/editing-data.md
index 0f6f359..3557249 100644
--- a/docs/_guide/editing-data.md
+++ b/docs/_guide/editing-data.md
@@ -188,6 +188,17 @@ Where the window leaves the bar little room, it says less, and
 **Discard all** and **Review SQL** move into the **…** menu beside
 **Save**.
 
+While changes are pending, Tabletist asks before anything that would drop
+them: closing the table's tab, its connection or the window, disconnecting,
+opening another connection in that tab, switching database, loading
+another page of the table (refreshing, paging, sorting, filtering,
+retrying), or following a foreign key into it. You can discard them or
+stay, and save them first where one table holds them. With
+**Ask before closing with unsaved edits** off in
+[Settings]({% link _reference/settings-and-files.md %}#the-settings-window),
+nothing is asked and the changes are discarded. While a save runs, these do
+nothing until it ends, and closing the window asks either way.
+
 A save to a production connection asks first, with every statement on
 screen.
 
diff --git a/docs/_reference/settings-and-files.md b/docs/_reference/settings-and-files.md
index d6932c3..cf18191 100644
--- a/docs/_reference/settings-and-files.md
+++ b/docs/_reference/settings-and-files.md
@@ -18,6 +18,7 @@ Cmd/Ctrl+, opens Settings. Changes apply right away.
 | **Numbers** | `1,240.50` or `1240.50`. Grouping is for display only: a copy gives the raw value | `1,240.50` on macOS and Windows, `1240.50` in the Omarchy look |
 | **Value tags** | Colors for enum, CHECK and boolean columns | On |
 | **New production connections open read-only** | On or off: where the **Open read-only** box of a new production connection starts. A connection you saved keeps its own box | On |
+| **Ask before closing with unsaved edits** | On or off. Off, closing a table, its connection or the window, or loading another page of it, discards its pending changes without asking | On |
 
 A second pane, **Agents**, sets what the `[mcp]` keys
 hold, and shows how to connect an agent:
@@ -54,6 +55,7 @@ value_tags = true
 
 [safety]
 production_read_only = true
+confirm_close_edits = true
 
 [sidebar]
 show_system_schemas = false
@@ -76,6 +78,7 @@ notify = true
 | `data.group_digits` | `true` or `false` | Not set: grouped on macOS and Windows, not in the Omarchy look |
 | `data.value_tags` | `true` or `false` | `true` |
 | `safety.production_read_only` | `true` or `false`. Where the **Open read-only** box of a new production connection starts; the connection is saved with its box set, so a change here changes no saved connection | `true` |
+| `safety.confirm_close_edits` | `true` asks before pending changes are dropped; `false` discards them without asking. While a save runs, what would drop them does nothing until it ends, and closing the window asks either way | `true` |
 | `sidebar.show_system_schemas` | `true` shows schemas such as `pg_catalog` and `information_schema` | `false` |
 | `editor.sql_limit` | 1 to 10000 rows for each statement | `1000` |
 | `editor.sql_timeout_secs` | Seconds. `0` waits forever | `30` |
diff --git a/src/app.rs b/src/app.rs
index c81e4a7..1c3506e 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -1329,15 +1329,7 @@ impl App {
                     return;
                 };
                 let LeavePrompt { held, tabs, .. } = *prompt;
-                // A save one of them runs goes with its set: its answer
-                // finds no tab saving, and tells none.
-                for (tab, id) in tabs {
-                    if let Some(object) = self.object_tab_mut(tab, id) {
-                        object.edits.discard();
-                        object.off_new_rows();
-                        object.fields = None;
-                    }
-                }
+                self.discard_edits(&tabs);
                 self.perform(held);
             }
             Action::LeaveSave => {
diff --git a/src/app/editing.rs b/src/app/editing.rs
index b3fd938..9de07e5 100644
--- a/src/app/editing.rs
+++ b/src/app/editing.rs
@@ -287,6 +287,12 @@ impl App {
     /// ends), but not the window's close: no control of the app's asks for
     /// it, and a save that never answers must not keep the window open
     /// for good. A dialog the user is in is never replaced.
+    ///
+    /// With "Ask before closing with unsaved edits" off nothing is asked:
+    /// the edits are dropped as Discard drops them, and `held` goes ahead.
+    /// Not while a save runs, which cannot be dropped, nor under a dialog,
+    /// which may be about those edits (a save's confirmation): those are
+    /// as above.
     pub(super) fn hold(&mut self, held: Held, tabs: Vec<(ConnTabId, TabId)>) {
         let edits = |&(tab, id): &(ConnTabId, TabId)| {
             self.workspace(tab)
@@ -303,6 +309,11 @@ impl App {
             self.notice = Some("Save or discard the pending changes first.".into());
             return;
         }
+        if !saving && !self.settings.confirm_close_edits {
+            self.discard_edits(&tabs);
+            self.perform(held);
+            return;
+        }
         let changes = tabs.iter().map(|&(tab, id)| self.unwritten(tab, id)).sum();
         // Never under a save, which blocks the tab's Save as well.
         let can_save = match tabs.as_slice() {
@@ -354,6 +365,19 @@ impl App {
         (changes + counts.added + counts.deleted).max(1)
     }
 
+    /// Drops the edits of `tabs`, as the question's Discard does. A save one
+    /// of them runs goes with its set: its answer finds no tab saving, and
+    /// tells none.
+    pub(super) fn discard_edits(&mut self, tabs: &[(ConnTabId, TabId)]) {
+        for &(tab, id) in tabs {
+            if let Some(object) = self.object_tab_mut(tab, id) {
+                object.edits.discard();
+                object.off_new_rows();
+                object.fields = None;
+            }
+        }
+    }
+
     /// Does what was held, now that nothing is in its way. It passes the
     /// guard again like any action.
     pub(super) fn perform(&mut self, held: Held) {
diff --git a/src/model.rs b/src/model.rs
index 45bf9c5..93b8e27 100644
--- a/src/model.rs
+++ b/src/model.rs
@@ -2187,6 +2187,7 @@ impl SettingsPane {
                 SettingsRow::Option(OptionId::GroupDigits),
                 SettingsRow::Option(OptionId::ValueTags),
                 SettingsRow::Option(OptionId::ProductionReadOnly),
+                SettingsRow::Option(OptionId::ConfirmCloseEdits),
             ],
             Self::Agents => &[
                 SettingsRow::Option(OptionId::AgentsOn),
diff --git a/src/settings.rs b/src/settings.rs
index 0e4c45e..2003644 100644
--- a/src/settings.rs
+++ b/src/settings.rs
@@ -116,6 +116,10 @@ pub struct Settings {
     /// switch, which it is saved with. A saved connection whose switch was
     /// never set keeps the built-in rule: production is read-only.
     pub production_read_only: bool,
+    /// Whether an action that would drop a table's unsaved edits (closing
+    /// its tab, its connection or the window, or loading another page)
+    /// asks first. Off, the edits are dropped and the action goes ahead.
+    pub confirm_close_edits: bool,
     /// Show `pg_catalog`, `information_schema`, `mysql`, `sys` and friends.
     pub show_system_schemas: bool,
     /// A palette file name in the themes directory. `None` follows the
@@ -145,6 +149,7 @@ pub enum Key {
     GroupDigits,
     ValueTags,
     ProductionReadOnly,
+    ConfirmCloseEdits,
     ShowSystemSchemas,
     SqlLimit,
     SqlTimeoutSecs,
@@ -156,13 +161,14 @@ pub enum Key {
 
 impl Key {
     /// In the order the file has them: a table's keys together.
-    pub const ALL: [Key; 13] = [
+    pub const ALL: [Key; 14] = [
         Self::PageSize,
         Self::Timestamps,
         Self::Tz,
         Self::GroupDigits,
         Self::ValueTags,
         Self::ProductionReadOnly,
+        Self::ConfirmCloseEdits,
         Self::ShowSystemSchemas,
         Self::SqlLimit,
         Self::SqlTimeoutSecs,
@@ -181,6 +187,7 @@ impl Key {
             Self::GroupDigits => ("data", "group_digits"),
             Self::ValueTags => ("data", "value_tags"),
             Self::ProductionReadOnly => ("safety", "production_read_only"),
+            Self::ConfirmCloseEdits => ("safety", "confirm_close_edits"),
             Self::ShowSystemSchemas => ("sidebar", "show_system_schemas"),
             Self::SqlLimit => ("editor", "sql_limit"),
             Self::SqlTimeoutSecs => ("editor", "sql_timeout_secs"),
@@ -215,6 +222,8 @@ pub enum OptionId {
     /// Where a new production connection's read-only switch starts:
     /// `safety.production_read_only`.
     ProductionReadOnly,
+    /// Whether dropping unsaved edits asks first: `safety.confirm_close_edits`.
+    ConfirmCloseEdits,
     /// Whether agents can reach the app: `mcp.enabled`.
     AgentsOn,
     /// How long the log of agents' calls keeps one: `mcp.log_days`.
@@ -225,25 +234,27 @@ pub enum OptionId {
 
 impl OptionId {
     /// Every option, the General pane's and then the Agents pane's.
-    pub const ALL: [OptionId; 9] = [
+    pub const ALL: [OptionId; 10] = [
         Self::PageSize,
         Self::Timestamps,
         Self::Tz,
         Self::GroupDigits,
         Self::ValueTags,
         Self::ProductionReadOnly,
+        Self::ConfirmCloseEdits,
         Self::AgentsOn,
         Self::LogDays,
         Self::AgentsNotify,
     ];
     /// The General pane's, in the order it has them.
-    pub const GENERAL: [OptionId; 6] = [
+    pub const GENERAL: [OptionId; 7] = [
         Self::PageSize,
         Self::Timestamps,
         Self::Tz,
         Self::GroupDigits,
         Self::ValueTags,
         Self::ProductionReadOnly,
+        Self::ConfirmCloseEdits,
     ];
     /// The Agents pane's, in the order it has them.
     pub const AGENTS: [OptionId; 3] = [Self::AgentsOn, Self::LogDays, Self::AgentsNotify];
@@ -257,6 +268,7 @@ impl OptionId {
             Self::GroupDigits => Key::GroupDigits,
             Self::ValueTags => Key::ValueTags,
             Self::ProductionReadOnly => Key::ProductionReadOnly,
+            Self::ConfirmCloseEdits => Key::ConfirmCloseEdits,
             Self::AgentsOn => Key::McpEnabled,
             Self::LogDays => Key::McpLogDays,
             Self::AgentsNotify => Key::McpNotify,
@@ -279,6 +291,7 @@ pub enum OptionValue {
     GroupDigits(Option<bool>),
     ValueTags(bool),
     ProductionReadOnly(bool),
+    ConfirmCloseEdits(bool),
     AgentsOn(bool),
     LogDays(u32),
     AgentsNotify(bool),
@@ -293,6 +306,7 @@ impl OptionValue {
             Self::GroupDigits(_) => OptionId::GroupDigits,
             Self::ValueTags(_) => OptionId::ValueTags,
             Self::ProductionReadOnly(_) => OptionId::ProductionReadOnly,
+            Self::ConfirmCloseEdits(_) => OptionId::ConfirmCloseEdits,
             Self::AgentsOn(_) => OptionId::AgentsOn,
             Self::LogDays(_) => OptionId::LogDays,
             Self::AgentsNotify(_) => OptionId::AgentsNotify,
@@ -308,6 +322,7 @@ impl OptionValue {
             Self::GroupDigits(on) => settings.group_digits = on,
             Self::ValueTags(on) => settings.value_tags = on,
             Self::ProductionReadOnly(on) => settings.production_read_only = on,
+            Self::ConfirmCloseEdits(on) => settings.confirm_close_edits = on,
             Self::AgentsOn(on) => settings.mcp_enabled = on,
             Self::LogDays(days) => settings.mcp_log_days = days,
             Self::AgentsNotify(on) => settings.mcp_notify = on,
@@ -765,6 +780,7 @@ impl Settings {
             OptionId::ProductionReadOnly => {
                 OptionValue::ProductionReadOnly(self.production_read_only)
             }
+            OptionId::ConfirmCloseEdits => OptionValue::ConfirmCloseEdits(self.confirm_close_edits),
             OptionId::AgentsOn => OptionValue::AgentsOn(self.mcp_enabled),
             OptionId::LogDays => OptionValue::LogDays(self.mcp_log_days),
             OptionId::AgentsNotify => OptionValue::AgentsNotify(self.mcp_notify),
@@ -801,6 +817,7 @@ impl Settings {
             OptionId::GroupDigits => OptionValue::GroupDigits(Some(!forward)),
             OptionId::ValueTags => OptionValue::ValueTags(forward),
             OptionId::ProductionReadOnly => OptionValue::ProductionReadOnly(forward),
+            OptionId::ConfirmCloseEdits => OptionValue::ConfirmCloseEdits(forward),
             OptionId::AgentsOn => OptionValue::AgentsOn(forward),
             OptionId::LogDays => {
                 let days = self.mcp_log_days;
@@ -852,6 +869,9 @@ impl Settings {
             OptionId::ProductionReadOnly => {
                 Some(OptionValue::ProductionReadOnly(!self.production_read_only))
             }
+            OptionId::ConfirmCloseEdits => {
+                Some(OptionValue::ConfirmCloseEdits(!self.confirm_close_edits))
+            }
             OptionId::AgentsOn => Some(OptionValue::AgentsOn(!self.mcp_enabled)),
             OptionId::LogDays => None,
             OptionId::AgentsNotify => Some(OptionValue::AgentsNotify(!self.mcp_notify)),
@@ -868,6 +888,7 @@ impl Settings {
             Key::GroupDigits => self.group_digits?.to_string(),
             Key::ValueTags => self.value_tags.to_string(),
             Key::ProductionReadOnly => self.production_read_only.to_string(),
+            Key::ConfirmCloseEdits => self.confirm_close_edits.to_string(),
             Key::ShowSystemSchemas => self.show_system_schemas.to_string(),
             Key::SqlLimit => self.sql_limit.to_string(),
             // TOML has no null: no timeout is no seconds.
@@ -1089,6 +1110,7 @@ impl Settings {
             Key::GroupDigits => flag.map(|flag| self.group_digits = Some(flag)),
             Key::ValueTags => flag.map(|flag| self.value_tags = flag),
             Key::ProductionReadOnly => flag.map(|flag| self.production_read_only = flag),
+            Key::ConfirmCloseEdits => flag.map(|flag| self.confirm_close_edits = flag),
             Key::ShowSystemSchemas => flag.map(|flag| self.show_system_schemas = flag),
             Key::SqlLimit => number.map(|number| self.sql_limit = number),
             Key::SqlTimeoutSecs => number.map(|number| self.sql_timeout_secs = Some(number)),
@@ -1198,6 +1220,7 @@ impl Settings {
             Key::GroupDigits => self.group_digits = other.group_digits,
             Key::ValueTags => self.value_tags = other.value_tags,
             Key::ProductionReadOnly => self.production_read_only = other.production_read_only,
+            Key::ConfirmCloseEdits => self.confirm_close_edits = other.confirm_close_edits,
             Key::ShowSystemSchemas => self.show_system_schemas = other.show_system_schemas,
             Key::SqlLimit => self.sql_limit = other.sql_limit,
             Key::SqlTimeoutSecs => self.sql_timeout_secs = other.sql_timeout_secs,
@@ -1256,6 +1279,7 @@ impl Default for Settings {
             group_digits: None,
             value_tags: true,
             production_read_only: true,
+            confirm_close_edits: true,
             show_system_schemas: false,
             custom_theme: None,
             sql_limit: 1_000,
diff --git a/src/ui/settings/mod.rs b/src/ui/settings/mod.rs
index 97d4cc8..815f761 100644
--- a/src/ui/settings/mod.rs
+++ b/src/ui/settings/mod.rs
@@ -28,6 +28,7 @@ pub(super) fn label(option: OptionId) -> &'static str {
         OptionId::GroupDigits => "Numbers",
         OptionId::ValueTags => "Value tags",
         OptionId::ProductionReadOnly => "New production connections open read-only",
+        OptionId::ConfirmCloseEdits => "Ask before closing with unsaved edits",
         OptionId::AgentsOn => "Let agents use Tabletist",
         OptionId::LogDays => "Keep the activity log",
         OptionId::AgentsNotify => "Notify when an agent connects",
@@ -41,6 +42,7 @@ pub(super) fn terminal_label(option: OptionId) -> &'static str {
     match option {
         OptionId::Tz => "timestamptz zone",
         OptionId::ProductionReadOnly => "production opens read-only",
+        OptionId::ConfirmCloseEdits => "confirm close with edits",
         OptionId::PageSize
         | OptionId::Timestamps
         | OptionId::GroupDigits
@@ -62,6 +64,7 @@ pub(super) fn group_starting(option: OptionId) -> Option<&'static str> {
         | OptionId::Tz
         | OptionId::GroupDigits
         | OptionId::ValueTags
+        | OptionId::ConfirmCloseEdits
         | OptionId::AgentsOn
         | OptionId::LogDays
         | OptionId::AgentsNotify => None,
@@ -78,6 +81,7 @@ pub(super) fn small_print(option: OptionId) -> Option<&'static str> {
         OptionId::ProductionReadOnly => Some("Default for the switch in each connection"),
         OptionId::Timestamps
         | OptionId::GroupDigits
+        | OptionId::ConfirmCloseEdits
         | OptionId::AgentsOn
         | OptionId::AgentsNotify => None,
     }
@@ -124,6 +128,7 @@ pub(super) fn is_on(value: &OptionValue) -> bool {
         value,
         OptionValue::ValueTags(true)
             | OptionValue::ProductionReadOnly(true)
+            | OptionValue::ConfirmCloseEdits(true)
             | OptionValue::AgentsOn(true)
             | OptionValue::AgentsNotify(true)
     )
@@ -213,6 +218,10 @@ pub(super) fn choices(
             (Word("off"), OptionValue::ProductionReadOnly(false)),
             (Word("on"), OptionValue::ProductionReadOnly(true)),
         ],
+        OptionId::ConfirmCloseEdits => vec![
+            (Word("off"), OptionValue::ConfirmCloseEdits(false)),
+            (Word("on"), OptionValue::ConfirmCloseEdits(true)),
+        ],
         OptionId::AgentsOn => vec![
             (Word("off"), OptionValue::AgentsOn(false)),
             (Word("on"), OptionValue::AgentsOn(true)),
diff --git a/src/ui/settings/sheet.rs b/src/ui/settings/sheet.rs
index 893bd7c..50aefa3 100644
--- a/src/ui/settings/sheet.rs
+++ b/src/ui/settings/sheet.rs
@@ -434,6 +434,7 @@ fn control_height(option: OptionId, look: &Look) -> f32 {
         OptionId::Timestamps | OptionId::GroupDigits => SEGMENT + 4.0,
         OptionId::ValueTags
         | OptionId::ProductionReadOnly
+        | OptionId::ConfirmCloseEdits
         | OptionId::AgentsOn
         | OptionId::AgentsNotify => SWITCH,
         OptionId::LogDays => look.control_height,
@@ -504,6 +505,7 @@ fn control(
         }
         OptionId::ValueTags
         | OptionId::ProductionReadOnly
+        | OptionId::ConfirmCloseEdits
         | OptionId::AgentsOn
         | OptionId::AgentsNotify => {
             let on = super::is_on(&settings.value(option));
@@ -663,6 +665,7 @@ fn hint(ui: &Ui, at: egui::Pos2, right: f32, option: OptionId, settings: &Settin
         ),
         OptionId::ValueTags => return tags(ui, at, right, settings.value_tags, skin),
         OptionId::ProductionReadOnly
+        | OptionId::ConfirmCloseEdits
         | OptionId::AgentsOn
         | OptionId::LogDays
         | OptionId::AgentsNotify => return,
diff --git a/src/ui/settings/terminal.rs b/src/ui/settings/terminal.rs
index e3ce60d..6e93df8 100644
--- a/src/ui/settings/terminal.rs
+++ b/src/ui/settings/terminal.rs
@@ -684,6 +684,7 @@ fn value_width(ui: &Ui, option: OptionId, settings: &Settings, skin: &Skin) -> f
         }
         OptionId::ValueTags
         | OptionId::ProductionReadOnly
+        | OptionId::ConfirmCloseEdits
         | OptionId::AgentsOn
         | OptionId::AgentsNotify => width("[x]"),
     }
@@ -845,6 +846,7 @@ fn option_row(
         }
         OptionId::ValueTags
         | OptionId::ProductionReadOnly
+        | OptionId::ConfirmCloseEdits
         | OptionId::AgentsOn
         | OptionId::AgentsNotify => {
             check(ui, cell, option, settings, skin, actions);
@@ -990,6 +992,8 @@ fn hint(ui: &Ui, option: OptionId, settings: &Settings, skin: &Skin) -> Text {
                 .add(role, "ebook", second)
         }
         OptionId::ProductionReadOnly => dim(&skin.say("default for new connections")),
+        // As drawn: nothing to add.
+        OptionId::ConfirmCloseEdits => Text::new(look),
         OptionId::AgentsOn => dim(&skin.say("while tabletist is open")),
         // The key that shows the log, from this row, where there is a
         // window to show it in.
````

- [ ] **Step 4: Check**

Run: `cargo fmt --all && cargo clippy --locked --workspace --all-targets -- -D warnings && cargo test --locked --workspace --all-targets`
Expected: no warning, every test passes.

- [ ] **Step 5: Commit**

```bash
git add docs/_guide/editing-data.md docs/_reference/settings-and-files.md src/app.rs src/app/editing.rs src/model.rs src/settings.rs src/ui/mod.rs src/ui/settings/mod.rs src/ui/settings/sheet.rs src/ui/settings/terminal.rs
git commit -m "Ask before dropping unsaved edits only where Settings › General says to"
```

### Task 3: Keep what is sent to a session still connecting until it is up

The backend's worker holds a command for a session whose connect is under way, and hands it to the session once it is up, after `Connected`, in the order it came. A connect that fails, or a close before it ends, answers it as a lost connection; a cancel answers it as skipped. Until now such a command failed at once as a lost connection.

**Files:** Modify `src/backend.rs`.

- [ ] **Step 1: Write the failing tests**

````diff
diff --git a/src/backend.rs b/src/backend.rs
index acbe818..982bfd5 100644
--- a/src/backend.rs
+++ b/src/backend.rs
@@ -3244,6 +3244,163 @@ mod tests {
     /// would have been sent by the end of it.
     const THIRD_CANCEL: Duration = Duration::from_millis(1500);
 
+    /// A worker connecting `session` to `spec`, with the commands
+    /// `queued` sent while it connects, and what came of the connect, not
+    /// handed to the worker yet.
+    async fn connecting_with(
+        outbox: Outbox,
+        session: SessionId,
+        spec: ConnectSpec,
+        queued: Vec<Command>,
+    ) -> (Worker, Ready) {
+        let (mut worker, mut ready) = Worker::new(outbox, Keyring::memory());
+        worker.handle(Command::Connect {
+            session,
+            request: RequestId(1),
+            spec,
+            secrets: Secrets::default(),
+            host_keys: HostKeys::default(),
+            access: Access::ReadOnly,
+        });
+        for command in queued {
+            worker.handle(command);
+        }
+        let done = tokio::time::timeout(WAIT, ready.recv())
+            .await
+            .expect("the connect ends")
+            .expect("the connect says how it ended");
+        (worker, done)
+    }
+
+    /// The next event, waited for.
+    fn next(received: &mpsc::Receiver<Event>) -> Event {
+        received.recv_timeout(WAIT).expect("an event")
+    }
+
+    #[test]
+    fn what_is_sent_while_a_session_connects_is_answered_once_it_is_up() {
+        let (outbox, received) = quiet_outbox();
+        let (_dir, spec) = fixture();
+        let session = SessionId(1);
+        let queued = vec![
+            Command::ListSchemas {
+                session,
+                request: RequestId(2),
+            },
+            Command::FetchRows {
+                session,
+                request: RequestId(3),
+                query: RowQuery::new(ObjectRef::new("main", "users"), 2),
+            },
+        ];
+        runtime().block_on(async {
+            let (mut worker, done) = connecting_with(outbox, session, spec, queued).await;
+            // Nothing is answered before the session is up.
+            assert!(received.try_recv().is_err());
+            worker.adopt(done);
+            assert!(matches!(next(&received), Event::Connected { .. }));
+            // In the order they were sent.
+            assert!(matches!(
+                next(&received),
+                Event::Schemas {
+                    request: RequestId(2),
+                    result: Ok(_),
+                    ..
+                }
+            ));
+            match next(&received) {
+                Event::Rows {
+                    request: RequestId(3),
+                    result: Ok(page),
+                    ..
+                } => assert_eq!(page.rows.len(), 2),
+                other => panic!("expected rows, got {other:?}"),
+            }
+        });
+    }
+
+    #[test]
+    fn what_waits_for_a_connect_that_fails_or_is_closed_is_lost() {
+        let lost = |event: Event| match event {
+            Event::Schemas {
+                request: RequestId(2),
+                result: Err(error),
+                ..
+            } => error.is_connection_lost(),
+            other => panic!("expected schemas, got {other:?}"),
+        };
+        let session = SessionId(1);
+        let queued = || {
+            vec![Command::ListSchemas {
+                session,
+                request: RequestId(2),
+            }]
+        };
+        // A file that cannot be opened.
+        let dir = tempfile::tempdir().unwrap();
+        let missing = ConnectSpec::sqlite(dir.path().join("no").join("such.db"));
+        let (outbox, received) = quiet_outbox();
+        runtime().block_on(async {
+            let (mut worker, done) = connecting_with(outbox, session, missing, queued()).await;
+            worker.adopt(done);
+            assert!(matches!(next(&received), Event::ConnectFailed { .. }));
+            assert!(lost(next(&received)));
+        });
+        // Closed before it is up: answered at once.
+        let (_dir, spec) = fixture();
+        let (outbox, received) = quiet_outbox();
+        runtime().block_on(async {
+            let (mut worker, done) = connecting_with(outbox, session, spec, queued()).await;
+            worker.handle(Command::Close { session });
+            assert!(lost(next(&received)));
+            worker.adopt(done);
+            assert!(received.try_recv().is_err(), "and nothing more");
+        });
+    }
+
+    #[test]
+    fn a_cancel_of_what_waits_for_the_session_takes_it_away() {
+        let (outbox, received) = quiet_outbox();
+        let (_dir, spec) = fixture();
+        let session = SessionId(1);
+        let queued = vec![
+            Command::ListSchemas {
+                session,
+                request: RequestId(2),
+            },
+            Command::Cancel {
+                session,
+                request: RequestId(2),
+            },
+            Command::ListSchemas {
+                session,
+                request: RequestId(3),
+            },
+        ];
+        runtime().block_on(async {
+            let (mut worker, done) = connecting_with(outbox, session, spec, queued).await;
+            // Skipped, as a session skips what it was told to cancel.
+            assert!(matches!(
+                next(&received),
+                Event::Schemas {
+                    request: RequestId(2),
+                    result: Err(Error::Cancelled),
+                    ..
+                }
+            ));
+            worker.adopt(done);
+            assert!(matches!(next(&received), Event::Connected { .. }));
+            assert!(matches!(
+                next(&received),
+                Event::Schemas {
+                    request: RequestId(3),
+                    result: Ok(_),
+                    ..
+                }
+            ));
+        });
+    }
+
     #[test]
     fn a_session_closed_before_its_script_is_registered_never_runs_it() {
         let (outbox, received) = quiet_outbox();
````

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --locked -p tabletist --lib`
Expected: FAIL: 3 tests (`backend::tests::a_cancel_of_what_waits_for_the_session_takes_it_away`, `backend::tests::what_is_sent_while_a_session_connects_is_answered_once_it_is_up`, `backend::tests::what_waits_for_a_connect_that_fails_or_is_closed_is_lost`).

- [ ] **Step 3: Write the code**

````diff
diff --git a/src/backend.rs b/src/backend.rs
index 982bfd5..ef2de9f 100644
--- a/src/backend.rs
+++ b/src/backend.rs
@@ -1613,6 +1613,12 @@ struct Worker {
     /// Sessions closed before they finished connecting.
     closed_early: std::collections::HashSet<SessionId>,
     connecting: std::collections::HashSet<SessionId>,
+    /// The commands sent to a session that is still connecting, in the
+    /// order they came: each goes to the session once it is up, and fails
+    /// as a lost connection if it never is. A tab whose session was closed
+    /// while idle connects again when it is used, and what it was used for
+    /// is answered by the new session.
+    waiting: HashMap<SessionId, Vec<Command>>,
     /// Watches and reads the settings file. Dropping it ends both.
     settings_watch: Option<SettingsReader>,
     /// The MCP server, its gateway and what agents may use.
@@ -1633,6 +1639,7 @@ impl Worker {
             ready,
             closed_early: Default::default(),
             connecting: Default::default(),
+            waiting: HashMap::new(),
             settings_watch: None,
             #[cfg(test)]
             watched: Watched::default(),
@@ -1662,13 +1669,14 @@ impl Worker {
     fn adopt(&mut self, done: Ready) {
         self.connecting.remove(&done.session);
         let closed = self.closed_early.remove(&done.session);
+        let waiting = self.waiting.remove(&done.session).unwrap_or_default();
         match done.outcome {
             // Dropping the handle stops the session task, which closes it.
+            // What waited was answered when it was closed.
             Ok(_) if closed => {}
             Ok((driver, encrypted, access, handle)) => {
                 #[cfg(test)]
                 lock(&self.watched).insert(done.session, Arc::clone(&handle.running));
-                self.sessions.insert(done.session, handle);
                 self.outbox.emit(Event::Connected {
                     session: done.session,
                     request: done.request,
@@ -1676,12 +1684,27 @@ impl Worker {
                     encrypted,
                     access,
                 });
+                // After `Connected`, so that the UI knows the session
+                // before its answers; before anything the UI sends once
+                // it does, so that they run in the order they were sent.
+                for command in waiting {
+                    if let Err(error) = handle.queue.send(command) {
+                        fail(&self.outbox, error.0, lost());
+                    }
+                }
+                self.sessions.insert(done.session, handle);
+            }
+            Err(error) => {
+                self.outbox.emit(Event::ConnectFailed {
+                    session: done.session,
+                    request: done.request,
+                    error,
+                });
+                // Lost, not failed for good: a reconnect sends them again.
+                for command in waiting {
+                    fail(&self.outbox, command, lost());
+                }
             }
-            Err(error) => self.outbox.emit(Event::ConnectFailed {
-                session: done.session,
-                request: done.request,
-                error,
-            }),
         }
     }
 
@@ -1793,12 +1816,23 @@ impl Worker {
                     }
                 } else if self.connecting.contains(&session) {
                     self.closed_early.insert(session);
+                    for command in self.waiting.remove(&session).unwrap_or_default() {
+                        fail(&self.outbox, command, lost());
+                    }
                 }
             }
             Command::Cancel { session, request } => {
                 if let Some(handle) = self.sessions.get(&session) {
                     let cancel = handle.cancel.clone();
                     cancel_request(&handle.running, request, move || send_cancel(&cancel));
+                } else if let Some(waiting) = self.waiting.get_mut(&session)
+                    && let Some(at) = waiting
+                        .iter()
+                        .position(|command| request_of(command) == Some(request))
+                {
+                    // Never sent: it is answered as a command the session
+                    // skipped.
+                    skip(&self.outbox, waiting.remove(at));
                 }
             }
             Command::LoadSecret { request, account } => {
@@ -1898,6 +1932,11 @@ impl Worker {
                             fail(&self.outbox, error.0, lost());
                         }
                     }
+                    None if self.connecting.contains(&session)
+                        && !self.closed_early.contains(&session) =>
+                    {
+                        self.waiting.entry(session).or_default().push(query);
+                    }
                     None => fail(&self.outbox, query, lost()),
                 }
             }
````

- [ ] **Step 4: Check**

Run: `cargo fmt --all && cargo clippy --locked --workspace --all-targets -- -D warnings && cargo test --locked --workspace --all-targets`
Expected: no warning, every test passes.

- [ ] **Step 5: Commit**

```bash
git add src/backend.rs
git commit -m "Keep what is sent to a session that is still connecting until it is up"
```

### Task 4: Close a session left unused, and connect it again when it is next used

The key `safety.idle_disconnect` (`Idle`, default `30m`), `SessionStatus::Idle`, `App::close_idle` from `App::logic`, `App::wake` in every sender of a command a use needs, and `App::note_use` for the person's input. A session closed while idle draws no banner, and its chip and card say so. A connect again that fails clears what waited for it; one that has to ask for a secret first sends no save and no run before it. The window's row comes in task 5: until then the key is set in the file.

**Files:** Modify `docs/_reference/settings-and-files.md`; Modify `src/app.rs`; Modify `src/app/editing.rs`; Modify `src/app/key_picker.rs`; Modify `src/entrypoint.rs`; Modify `src/model.rs`; Modify `src/settings.rs`; Modify `src/ui/sql_results.rs`; Modify `src/ui/workspace.rs`.

- [ ] **Step 1: Write the failing tests**

````diff
diff --git a/src/app.rs b/src/app.rs
index 7c15355..58fe666 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -13599,6 +13599,582 @@ mod tests {
 
     /// Editing a table's values: the pending set, the editor, the save and
     /// the guard that keeps a page with pending changes.
+    mod idle {
+        use super::*;
+        use crate::settings::Idle;
+        use std::time::Instant;
+
+        const MINUTE: Duration = Duration::from_secs(60);
+
+        /// A connection with `users` open and loaded, last used at the
+        /// instant it returns.
+        fn loaded(harness: &mut Harness) -> (ConnTabId, TabId, Instant) {
+            let tab = harness.connect_fake();
+            let id = open(harness, tab, "users", true);
+            harness.answer_structure(crate::testing::fixture_structure());
+            harness.answer_rows(page(5, false));
+            let used = harness.app.workspace(tab).unwrap().used;
+            (tab, id, used)
+        }
+
+        fn status(harness: &Harness, tab: ConnTabId) -> &SessionStatus {
+            &harness.app.workspace(tab).unwrap().status
+        }
+
+        fn closed(harness: &Harness, session: SessionId) -> bool {
+            harness
+                .app
+                .backend
+                .sent
+                .iter()
+                .any(|command| matches!(command, Command::Close { session: s } if *s == session))
+        }
+
+        #[test]
+        fn a_session_left_unused_for_the_time_is_closed_and_its_tab_stays() {
+            let mut harness = Harness::new();
+            let (tab, id, used) = loaded(&mut harness);
+            let session = harness.app.workspace(tab).unwrap().session;
+            // Not yet: the frame is asked back when it is due.
+            assert_eq!(harness.app.close_idle(used + 29 * MINUTE), Some(MINUTE));
+            assert!(matches!(status(&harness, tab), SessionStatus::Connected));
+            assert!(!closed(&harness, session));
+            assert_eq!(harness.app.close_idle(used + 30 * MINUTE), None);
+            assert!(matches!(status(&harness, tab), SessionStatus::Idle));
+            assert!(closed(&harness, session));
+            // The tab and its page stay.
+            assert_eq!(object(&harness, tab, id).page().unwrap().rows.len(), 5);
+            // Closed once.
+            let closes = harness.app.backend.sent.len();
+            harness.app.close_idle(used + 60 * MINUTE);
+            assert_eq!(harness.app.backend.sent.len(), closes);
+        }
+
+        #[test]
+        fn the_time_is_the_settings_and_never_is_never() {
+            let mut harness = Harness::new();
+            let (tab, _, used) = loaded(&mut harness);
+            harness.app.settings.idle_disconnect = Idle::Minutes5;
+            assert_eq!(harness.app.close_idle(used + 4 * MINUTE), Some(MINUTE));
+            harness.app.settings.idle_disconnect = Idle::Never;
+            assert_eq!(harness.app.close_idle(used + 24 * 60 * MINUTE), None);
+            assert!(matches!(status(&harness, tab), SessionStatus::Connected));
+        }
+
+        #[test]
+        fn a_session_is_never_closed_under_edits_or_while_it_works() {
+            // Edits pending.
+            let mut harness = Harness::new();
+            let (tab, id) = harness.editable();
+            let used = harness.app.workspace(tab).unwrap().used;
+            harness.app.apply(Action::EditCell {
+                tab,
+                id,
+                cell: CellPos { row: 1, col: 1 },
+                start: crate::model::EditStart::Replace("bob@example.com".into()),
+            });
+            harness.app.close_idle(used + 60 * MINUTE);
+            assert!(matches!(status(&harness, tab), SessionStatus::Connected));
+            // A fetch in flight: in use while it waits, so its time starts
+            // again when it is answered.
+            let mut harness = Harness::new();
+            let (tab, _, used) = loaded(&mut harness);
+            harness.app.apply(Action::Refresh(tab));
+            let later = used + 60 * MINUTE;
+            assert_eq!(harness.app.close_idle(later), Some(30 * MINUTE));
+            assert!(matches!(status(&harness, tab), SessionStatus::Connected));
+            harness.answer_structure(crate::testing::fixture_structure());
+            harness.answer_rows(page(5, false));
+            assert_eq!(
+                harness.app.close_idle(later + 10 * MINUTE),
+                Some(20 * MINUTE)
+            );
+            // A run in flight.
+            let mut harness = Harness::new();
+            let tab = harness.connect_fake();
+            let sql = harness.add_sql_tab(tab);
+            harness.app.sql_tab_mut(tab, sql).unwrap().text = "SELECT 1".into();
+            harness.app.apply(Action::RunSql {
+                tab,
+                sql_tab: sql,
+                all: true,
+            });
+            let used = harness.app.workspace(tab).unwrap().used;
+            harness.app.close_idle(used + 60 * MINUTE);
+            assert!(matches!(status(&harness, tab), SessionStatus::Connected));
+            // A session that is not connected has nothing to close.
+            let mut harness = Harness::new();
+            let (tab, _, used) = loaded(&mut harness);
+            harness.app.workspace_mut(tab).unwrap().status =
+                SessionStatus::Disconnected(Error::ConnectionLost("gone".into()));
+            let sent = harness.app.backend.sent.len();
+            harness.app.close_idle(used + 60 * MINUTE);
+            assert_eq!(harness.app.backend.sent.len(), sent);
+        }
+
+        #[test]
+        fn the_next_fetch_connects_again_and_is_answered_by_the_new_session() {
+            let mut harness = Harness::new();
+            let (tab, id, used) = loaded(&mut harness);
+            let old = harness.app.workspace(tab).unwrap().session;
+            harness.app.close_idle(used + 30 * MINUTE);
+            let before = harness.app.backend.sent.len();
+            harness.app.apply(Action::Refresh(tab));
+            let sent = &harness.app.backend.sent[before..];
+            // A connect for a new session, then the fetch for it, which
+            // the backend keeps until the session is up.
+            let Some(&Command::Connect {
+                session, request, ..
+            }) = sent.first()
+            else {
+                panic!("a connect first: {sent:?}");
+            };
+            assert_ne!(session, old);
+            assert!(sent.iter().any(
+                |command| matches!(command, Command::FetchRows { session: s, .. } if *s == session)
+            ));
+            assert!(matches!(
+                status(&harness, tab),
+                SessionStatus::Connecting { .. }
+            ));
+            let before = harness.app.backend.sent.len();
+            harness.app.apply(Action::Backend(Event::Connected {
+                session,
+                request,
+                driver: Driver::Sqlite,
+                encrypted: false,
+                access: crate::testing::asked_access(&harness.app),
+            }));
+            assert!(matches!(status(&harness, tab), SessionStatus::Connected));
+            // Nothing is fetched again: the fetch that woke it is answered.
+            let again = harness.app.backend.sent[before..].iter().any(|command| {
+                matches!(
+                    command,
+                    Command::ListSchemas { .. }
+                        | Command::FetchRows { .. }
+                        | Command::Describe { .. }
+                )
+            });
+            assert!(!again, "{:?}", &harness.app.backend.sent[before..]);
+            harness.answer_structure(crate::testing::fixture_structure());
+            harness.answer_rows(page(3, false));
+            assert_eq!(object(&harness, tab, id).page().unwrap().rows.len(), 3);
+        }
+
+        #[test]
+        fn a_save_or_a_run_connects_again_too() {
+            let mut harness = Harness::new();
+            let (tab, id) = harness.editable();
+            let used = harness.app.workspace(tab).unwrap().used;
+            harness.app.close_idle(used + 30 * MINUTE);
+            harness.app.apply(Action::EditCell {
+                tab,
+                id,
+                cell: CellPos { row: 1, col: 1 },
+                start: crate::model::EditStart::Replace("bob@example.com".into()),
+            });
+            harness.app.apply(Action::CommitEdit {
+                tab,
+                id,
+                then: crate::model::Advance::Stay,
+            });
+            assert_eq!(harness.app.save_blocked(tab, id), None);
+            harness.app.apply(Action::WriteEdits { tab, id });
+            let session = harness.app.workspace(tab).unwrap().session;
+            let sent = &harness.app.backend.sent;
+            let connect = sent
+                .iter()
+                .position(|c| matches!(c, Command::Connect { session: s, .. } if *s == session));
+            let write = sent
+                .iter()
+                .position(|c| matches!(c, Command::Write { session: s, .. } if *s == session));
+            assert!(connect.is_some() && connect < write, "{sent:?}");
+            // A run.
+            let mut harness = Harness::new();
+            let tab = harness.connect_fake();
+            let sql = harness.add_sql_tab(tab);
+            let used = harness.app.workspace(tab).unwrap().used;
+            harness.app.close_idle(used + 30 * MINUTE);
+            harness.app.sql_tab_mut(tab, sql).unwrap().text = "SELECT 1".into();
+            harness.app.apply(Action::RunSql {
+                tab,
+                sql_tab: sql,
+                all: true,
+            });
+            let session = harness.app.workspace(tab).unwrap().session;
+            assert!(
+                harness
+                    .app
+                    .backend
+                    .sent
+                    .iter()
+                    .any(|c| matches!(c, Command::RunSql { session: s, .. } if *s == session))
+            );
+        }
+
+        #[test]
+        fn a_connect_again_that_fails_says_so_and_reconnect_restarts_the_tab() {
+            let mut harness = Harness::new();
+            let (tab, _, used) = loaded(&mut harness);
+            harness.app.close_idle(used + 30 * MINUTE);
+            harness.app.apply(Action::Refresh(tab));
+            let Some(&Command::Connect {
+                session, request, ..
+            }) = harness
+                .app
+                .backend
+                .sent
+                .iter()
+                .rev()
+                .find(|c| matches!(c, Command::Connect { .. }))
+            else {
+                panic!("expected Connect");
+            };
+            harness.app.apply(Action::Backend(Event::ConnectFailed {
+                session,
+                request,
+                error: Error::Connect("refused".into()),
+            }));
+            assert!(matches!(
+                status(&harness, tab),
+                SessionStatus::Disconnected(_)
+            ));
+            // Reconnect is a reconnect: the tree and the page come again.
+            harness.app.apply(Action::Reconnect(tab));
+            let Command::Connect {
+                session, request, ..
+            } = *last_sent(&harness.app)
+            else {
+                panic!("expected Connect");
+            };
+            let before = harness.app.backend.sent.len();
+            harness.app.apply(Action::Backend(Event::Connected {
+                session,
+                request,
+                driver: Driver::Sqlite,
+                encrypted: false,
+                access: crate::testing::asked_access(&harness.app),
+            }));
+            let new = &harness.app.backend.sent[before..];
+            assert!(new.iter().any(|c| matches!(c, Command::ListSchemas { .. })));
+            assert!(new.iter().any(|c| matches!(c, Command::FetchRows { .. })));
+        }
+
+        /// The newest connect sent.
+        fn last_connect(harness: &Harness) -> (SessionId, RequestId) {
+            harness
+                .app
+                .backend
+                .sent
+                .iter()
+                .rev()
+                .find_map(|command| match command {
+                    Command::Connect {
+                        session, request, ..
+                    } => Some((*session, *request)),
+                    _ => None,
+                })
+                .expect("a connect")
+        }
+
+        fn type_password(harness: &mut Harness, text: &str) {
+            let Some(Dialog::Password(prompt)) = &mut harness.app.dialog else {
+                panic!("the password is asked for");
+            };
+            prompt.password = text.into();
+            harness.app.apply(Action::SubmitPassword);
+        }
+
+        #[test]
+        fn a_connect_again_the_server_refuses_restarts_the_tab_once_it_is_answered() {
+            let mut harness = Harness::new();
+            // A connection whose password is asked for, and kept by its tab.
+            let mut saved = crate::testing::fixture_connection();
+            saved.password = PasswordMode::Ask;
+            let conn = saved.id.clone();
+            harness.app.connections.upsert(saved);
+            let tab = harness.app.active_tab_id();
+            harness.app.apply(Action::Connect { tab, conn });
+            type_password(&mut harness, "pw");
+            let (session, request) = last_connect(&harness);
+            harness.app.apply(Action::Backend(Event::Connected {
+                session,
+                request,
+                driver: Driver::Sqlite,
+                encrypted: false,
+                access: crate::testing::asked_access(&harness.app),
+            }));
+            // Its tree, answered: nothing waits for the session.
+            let Command::ListSchemas { request, .. } = *last_sent(&harness.app) else {
+                panic!("expected ListSchemas");
+            };
+            harness.app.apply(Action::Backend(Event::Schemas {
+                session,
+                request,
+                result: Ok(vec!["main".into()]),
+            }));
+            let Command::ListObjects { request, .. } = *last_sent(&harness.app) else {
+                panic!("expected ListObjects");
+            };
+            harness.app.apply(Action::Backend(Event::Objects {
+                session,
+                request,
+                schema: "main".into(),
+                result: Ok(Vec::new()),
+            }));
+            let used = harness.app.workspace(tab).unwrap().used;
+            harness.app.close_idle(used + 30 * MINUTE);
+            assert!(matches!(status(&harness, tab), SessionStatus::Idle));
+            harness.app.apply(Action::Refresh(tab));
+            // The password was changed on the server meanwhile.
+            let (session, request) = last_connect(&harness);
+            harness.app.apply(Action::Backend(Event::ConnectFailed {
+                session,
+                request,
+                error: Error::Auth("password authentication failed".into()),
+            }));
+            type_password(&mut harness, "new");
+            let (session, request) = last_connect(&harness);
+            let before = harness.app.backend.sent.len();
+            harness.app.apply(Action::Backend(Event::Connected {
+                session,
+                request,
+                driver: Driver::Sqlite,
+                encrypted: false,
+                access: crate::testing::asked_access(&harness.app),
+            }));
+            // What the use sent was lost with the refused connect: the tab
+            // starts again, as after any reconnect.
+            let new = &harness.app.backend.sent[before..];
+            assert!(
+                new.iter().any(|c| matches!(c, Command::ListSchemas { .. })),
+                "{new:?}"
+            );
+        }
+
+        #[test]
+        fn a_save_or_a_run_that_waited_for_a_connect_that_failed_was_never_sent() {
+            let mut harness = Harness::new();
+            let (tab, id) = harness.editable();
+            let sql = harness.add_sql_tab(tab);
+            harness.app.workspace_mut(tab).unwrap().active_tab = Some(id);
+            let used = harness.app.workspace(tab).unwrap().used;
+            harness.app.close_idle(used + 30 * MINUTE);
+            harness.app.apply(Action::EditCell {
+                tab,
+                id,
+                cell: CellPos { row: 1, col: 1 },
+                start: crate::model::EditStart::Replace("bob@example.com".into()),
+            });
+            harness.app.apply(Action::CommitEdit {
+                tab,
+                id,
+                then: crate::model::Advance::Stay,
+            });
+            harness.app.apply(Action::WriteEdits { tab, id });
+            harness.app.sql_tab_mut(tab, sql).unwrap().text = "SELECT 1".into();
+            harness.app.apply(Action::RunSql {
+                tab,
+                sql_tab: sql,
+                all: true,
+            });
+            let (session, request) = last_connect(&harness);
+            harness.app.apply(Action::Backend(Event::ConnectFailed {
+                session,
+                request,
+                error: Error::Connect("refused".into()),
+            }));
+            let object = object(&harness, tab, id);
+            assert!(object.edits.saving.is_none());
+            assert_eq!(object.edits.note, Some(crate::edit::Note::NotSent));
+            assert!(object.edits.pending(), "the changes stay");
+            let run = harness.app.workspace(tab).unwrap().sql_tab(sql).unwrap();
+            assert!(!run.is_running());
+            assert!(!run.lost_writing());
+        }
+
+        #[test]
+        fn a_connect_again_that_asks_for_a_secret_first_sends_no_save_before_it() {
+            let mut harness = Harness::new();
+            let (tab, id) = harness.editable();
+            let used = harness.app.workspace(tab).unwrap().used;
+            harness.app.close_idle(used + 30 * MINUTE);
+            // The password is asked for, not kept.
+            let workspace = harness.app.workspace_mut(tab).unwrap();
+            workspace.password_mode = PasswordMode::Ask;
+            workspace.secrets.password = None;
+            harness.app.apply(Action::EditCell {
+                tab,
+                id,
+                cell: CellPos { row: 1, col: 1 },
+                start: crate::model::EditStart::Replace("bob@example.com".into()),
+            });
+            harness.app.apply(Action::CommitEdit {
+                tab,
+                id,
+                then: crate::model::Advance::Stay,
+            });
+            harness.app.apply(Action::WriteEdits { tab, id });
+            assert!(matches!(harness.app.dialog, Some(Dialog::Password(_))));
+            let sent = &harness.app.backend.sent;
+            assert!(
+                !sent.iter().any(|c| matches!(c, Command::Write { .. })),
+                "{sent:?}"
+            );
+            let object = object(&harness, tab, id);
+            assert_eq!(object.edits.note, Some(crate::edit::Note::NotSent));
+            assert!(object.edits.saving.is_none());
+            assert!(object.edits.pending(), "the changes stay");
+        }
+
+        #[test]
+        fn an_editor_opened_while_idle_learns_the_server_once_it_connects_again() {
+            let mut harness = Harness::new();
+            let (tab, _, used) = loaded(&mut harness);
+            harness.app.close_idle(used + 30 * MINUTE);
+            harness.add_sql_tab(tab);
+            assert!(
+                !harness
+                    .app
+                    .backend
+                    .sent
+                    .iter()
+                    .any(|c| matches!(c, Command::ServerVersion { .. }))
+            );
+            harness.app.apply(Action::Refresh(tab));
+            let (session, request) = last_connect(&harness);
+            harness.app.apply(Action::Backend(Event::Connected {
+                session,
+                request,
+                driver: Driver::Sqlite,
+                encrypted: false,
+                access: crate::testing::asked_access(&harness.app),
+            }));
+            assert!(
+                harness.app.backend.sent.iter().any(
+                    |c| matches!(c, Command::ServerVersion { session: s, .. } if *s == session)
+                )
+            );
+        }
+
+        #[test]
+        fn a_list_of_a_foreign_key_that_waits_keeps_the_session_in_use() {
+            let mut harness = Harness::new();
+            let (tab, _, used) = loaded(&mut harness);
+            let session = harness.app.workspace(tab).unwrap().session;
+            harness.app.lookups.push(key_picker::SentLookup {
+                session,
+                request: RequestId(9_999),
+                picker: 1,
+                matches: true,
+            });
+            let later = used + 60 * MINUTE;
+            assert_eq!(harness.app.close_idle(later), Some(30 * MINUTE));
+            assert!(matches!(status(&harness, tab), SessionStatus::Connected));
+        }
+
+        #[test]
+        fn opening_a_schema_connects_again_too() {
+            let mut harness = Harness::new();
+            let (tab, _, used) = loaded(&mut harness);
+            harness.app.close_idle(used + 30 * MINUTE);
+            harness.app.load_objects(tab, "main");
+            let (session, _) = last_connect(&harness);
+            assert!(
+                harness
+                    .app
+                    .backend
+                    .sent
+                    .iter()
+                    .any(|c| matches!(c, Command::ListObjects { session: s, .. } if *s == session))
+            );
+        }
+
+        #[test]
+        fn a_connect_again_for_a_use_shows_no_strip() {
+            let mut harness = Harness::new();
+            let (tab, _, used) = loaded(&mut harness);
+            harness.app.close_idle(used + 30 * MINUTE);
+            harness.app.apply(Action::Refresh(tab));
+            assert!(matches!(
+                status(&harness, tab),
+                SessionStatus::Connecting { .. }
+            ));
+            harness.settle();
+            assert!(
+                !harness
+                    .painted
+                    .iter()
+                    .any(|(text, _)| text.starts_with("Reconnecting")),
+                "{:?}",
+                harness.painted
+            );
+            // A reconnect the person asked for shows it.
+            harness.app.apply(Action::Reconnect(tab));
+            harness.settle();
+            assert!(
+                harness
+                    .painted
+                    .iter()
+                    .any(|(text, _)| text.starts_with("Reconnecting"))
+            );
+        }
+
+        #[test]
+        fn a_key_in_the_connections_tab_is_a_use() {
+            let mut harness = Harness::new();
+            let (tab, _, _) = loaded(&mut harness);
+            let long_ago = Instant::now()
+                .checked_sub(60 * MINUTE)
+                .expect("an hour ago");
+            harness.app.workspace_mut(tab).unwrap().used = long_ago;
+            harness.settle();
+            assert_eq!(
+                harness.app.workspace(tab).unwrap().used,
+                long_ago,
+                "a frame alone is none"
+            );
+            harness.press(egui::Key::ArrowDown, egui::Modifiers::NONE);
+            assert!(harness.app.workspace(tab).unwrap().used > long_ago);
+            // A scroll and a click are uses too.
+            for event in [
+                egui::Event::MouseWheel {
+                    unit: egui::MouseWheelUnit::Point,
+                    delta: egui::vec2(0.0, -40.0),
+                    modifiers: egui::Modifiers::NONE,
+                    phase: egui::TouchPhase::Move,
+                },
+                egui::Event::PointerButton {
+                    pos: egui::pos2(600.0, 400.0),
+                    button: egui::PointerButton::Primary,
+                    pressed: true,
+                    modifiers: egui::Modifiers::NONE,
+                },
+            ] {
+                harness.app.workspace_mut(tab).unwrap().used = long_ago;
+                harness.frame(vec![event]);
+                assert!(harness.app.workspace(tab).unwrap().used > long_ago);
+            }
+        }
+
+        #[test]
+        fn a_session_closed_while_idle_says_so_and_shows_no_banner() {
+            let mut harness = Harness::new();
+            let (tab, _, used) = loaded(&mut harness);
+            harness.app.close_idle(used + 30 * MINUTE);
+            harness.settle();
+            assert!(!harness.has("Reconnect"));
+            assert!(harness.painted_color("Disconnected while idle").is_some());
+            // A late word from the closed session changes nothing.
+            let session = harness.app.workspace(tab).unwrap().session;
+            harness.app.apply(Action::Backend(Event::Disconnected {
+                session,
+                error: Error::ConnectionLost("closed".into()),
+            }));
+            assert!(matches!(status(&harness, tab), SessionStatus::Idle));
+        }
+    }
+
     mod editing {
         use super::*;
         use crate::edit::{Answer, Conflicting, EditorPlace, Lock, Place, Problem, State, new_row};
diff --git a/src/entrypoint.rs b/src/entrypoint.rs
index f14caf8..b281b2b 100644
--- a/src/entrypoint.rs
+++ b/src/entrypoint.rs
@@ -615,6 +615,44 @@ mod tests {
         }
     }
 
+    #[test]
+    fn a_session_closed_while_idle_connects_again_for_the_next_fetch() {
+        use crate::testing::Harness;
+        let backend = crate::backend::Backend::start_with(
+            crate::backend::Waker::default(),
+            keyring_for(true),
+        );
+        let mut harness = Harness::with_backend(egui::vec2(1280.0, 800.0), backend);
+        demo_setup(&mut harness.app);
+        let rows = |app: &crate::app::App| {
+            app.workspace(app.active_tab_id())
+                .and_then(|workspace| workspace.active_object_tab())
+                .and_then(|object| object.page())
+                .map(|page| page.rows.len())
+        };
+        let wait = std::time::Duration::from_secs(20);
+        assert!(harness.run_until(wait, |app| rows(app) == Some(5)));
+        let tab = harness.app.active_tab_id();
+        let old = harness.app.workspace(tab).unwrap().session;
+        let later = std::time::Instant::now() + std::time::Duration::from_secs(31 * 60);
+        harness.app.close_idle(later);
+        assert!(matches!(
+            harness.app.workspace(tab).unwrap().status,
+            crate::model::SessionStatus::Idle
+        ));
+        harness.app.apply(crate::model::Action::Refresh(tab));
+        let answered = harness.run_until(wait, |app| {
+            let workspace = app.workspace(tab).unwrap();
+            matches!(workspace.status, crate::model::SessionStatus::Connected)
+                && workspace.session != old
+                && workspace
+                    .active_object_tab()
+                    .is_some_and(|object| !object.rows.is_loading() && object.rows.error.is_none())
+        });
+        assert!(answered, "the new session answers the fetch");
+        assert_eq!(rows(&harness.app), Some(5));
+    }
+
     #[test]
     fn the_desktop_entry_matches_the_app_id() {
         let entry = include_str!("../packaging/linux/dev.tabletist.Tabletist.desktop");
diff --git a/src/settings.rs b/src/settings.rs
index 781082c..89cec9f 100644
--- a/src/settings.rs
+++ b/src/settings.rs
@@ -1631,6 +1631,36 @@ mod tests {
         assert_eq!(Timestamps::from_name("minute"), None);
     }
 
+    #[test]
+    fn an_idle_choice_has_a_name_and_a_wait() {
+        for choice in Idle::ALL {
+            assert_eq!(Idle::from_name(choice.name()), Some(choice));
+        }
+        assert_eq!(Idle::from_name("2m"), None);
+        assert_eq!(Idle::Never.after(), None);
+        assert_eq!(
+            Idle::default().after(),
+            Some(std::time::Duration::from_secs(30 * 60))
+        );
+        assert_eq!(
+            Idle::Hours4.after(),
+            Some(std::time::Duration::from_secs(4 * 60 * 60))
+        );
+    }
+
+    #[test]
+    fn an_idle_time_out_of_the_set_is_ignored_and_keeps_the_last_good_one() {
+        let last = Settings {
+            idle_disconnect: Idle::Hour1,
+            ..Settings::default()
+        };
+        let text = "[safety]\nidle_disconnect = \"2m\"\nconfirm_close_edits = false\n";
+        let loaded = Settings::from_toml_over(text, &last);
+        assert_eq!(loaded.invalid, [2]);
+        assert_eq!(loaded.settings.idle_disconnect, Idle::Hour1);
+        assert!(!loaded.settings.confirm_close_edits);
+    }
+
     #[test]
     fn the_defaults_are_written_as_the_header_alone() {
         assert_eq!(Settings::default().to_toml(), HEADER);
@@ -1646,6 +1676,7 @@ mod tests {
             value_tags: false,
             production_read_only: false,
             confirm_close_edits: false,
+            idle_disconnect: Idle::Hours4,
             show_system_schemas: true,
             custom_theme: Some("My \"Nord\".json".into()),
             sql_limit: 100,
@@ -1677,6 +1708,7 @@ value_tags = false
 [safety]
 production_read_only = false
 confirm_close_edits = false
+idle_disconnect = \"4h\"  # never | 5m | 15m | 30m | 1h | 4h
 
 [sidebar]
 show_system_schemas = true
@@ -1764,18 +1796,19 @@ bar = \"label\"  # tint | label
                 (Key::ValueTags, 7),
                 (Key::ProductionReadOnly, 10),
                 (Key::ConfirmCloseEdits, 11),
-                (Key::ShowSystemSchemas, 14),
-                (Key::SqlLimit, 17),
-                (Key::SqlTimeoutSecs, 18),
-                (Key::McpEnabled, 21),
-                (Key::McpLogDays, 22),
-                (Key::McpNotify, 23),
-                (Key::Theme, 26),
-                (Key::RowHeight, 29),
-                (Key::Zebra, 30),
-                (Key::ColumnTypes, 31),
-                (Key::TableList, 32),
-                (Key::EnvBar, 35),
+                (Key::IdleDisconnect, 12),
+                (Key::ShowSystemSchemas, 15),
+                (Key::SqlLimit, 18),
+                (Key::SqlTimeoutSecs, 19),
+                (Key::McpEnabled, 22),
+                (Key::McpLogDays, 23),
+                (Key::McpNotify, 24),
+                (Key::Theme, 27),
+                (Key::RowHeight, 30),
+                (Key::Zebra, 31),
+                (Key::ColumnTypes, 32),
+                (Key::TableList, 33),
+                (Key::EnvBar, 36),
             ]
         );
     }
````

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --locked -p tabletist --lib`
Expected: FAIL to compile: `` unresolved import `crate::settings::Idle` ``; `` no field `used` on type `&Workspace` ``; `` no method named `close_idle` found for struct `app::App` in the current scope ``; `` no variant, associated function, or constant named `Idle` found for enum `model::SessionStatus` in the current scope ``, and 5 more.

- [ ] **Step 3: Write the code**

````diff
diff --git a/docs/_reference/settings-and-files.md b/docs/_reference/settings-and-files.md
index 4ce215c..8b99727 100644
--- a/docs/_reference/settings-and-files.md
+++ b/docs/_reference/settings-and-files.md
@@ -67,6 +67,7 @@ value_tags = true
 [safety]
 production_read_only = true
 confirm_close_edits = true
+idle_disconnect = "30m"
 
 [sidebar]
 show_system_schemas = false
@@ -98,6 +99,7 @@ bar = "tint"
 | `data.value_tags` | `true` or `false` | `true` |
 | `safety.production_read_only` | `true` or `false`. Where the **Open read-only** box of a new production connection starts; the connection is saved with its box set, so a change here changes no saved connection | `true` |
 | `safety.confirm_close_edits` | `true` asks before pending changes are dropped; `false` discards them without asking. While a save runs, what would drop them does nothing until it ends, and closing the window asks either way | `true` |
+| `safety.idle_disconnect` | `"never"`, `"5m"`, `"15m"`, `"30m"`, `"1h"` or `"4h"`. A connection left unused that long has its server session closed; its tabs stay as they were, and it connects again when you next refresh, page, open a table, run or save. Never while changes are pending or something is still loading, running or saving | `"30m"` |
 | `sidebar.show_system_schemas` | `true` shows schemas such as `pg_catalog` and `information_schema` | `false` |
 | `editor.sql_limit` | 1 to 10000 rows for each statement | `1000` |
 | `editor.sql_timeout_secs` | Seconds. `0` waits forever | `30` |
diff --git a/src/app.rs b/src/app.rs
index 58fe666..00d6a9c 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -2718,6 +2718,7 @@ impl App {
             let old = std::mem::replace(&mut workspace.session, session);
             workspace.status = SessionStatus::Connecting { request };
             workspace.connect_started = None;
+            workspace.woken = false;
             workspace.forget_session_requests();
             self.backend.send(Command::Close { session: old });
         }
@@ -2867,6 +2868,7 @@ impl App {
         let old = std::mem::replace(&mut workspace.session, session);
         workspace.status = SessionStatus::Connecting { request };
         workspace.connect_started = None;
+        workspace.woken = false;
         workspace.forget_session_requests();
         self.backend.send(Command::Close { session: old });
         // Reuses the secrets this tab already has; asks only for missing ones.
@@ -2901,7 +2903,17 @@ impl App {
                     Some(tab) => {
                         self.save_accepted_password(tab);
                         self.mark_used(tab);
-                        self.after_connect(tab);
+                        // Opened again for a use, after it was closed while
+                        // idle: what the use sent is answered by this
+                        // session, and the rest of the tab is as it was.
+                        let woken = self
+                            .workspace_mut(tab)
+                            .is_some_and(|workspace| std::mem::take(&mut workspace.woken));
+                        if woken {
+                            self.ask_server_version(tab);
+                        } else {
+                            self.after_connect(tab);
+                        }
                     }
                     // Nobody is waiting for this session any more.
                     None => self.backend.send(Command::Close { session }),
@@ -2922,6 +2934,21 @@ impl App {
                 {
                     return;
                 }
+                // A connect again for a use that failed: what the use sent
+                // waited for it and never left the app. A save says so; a
+                // run is forgotten, and claims nothing was written. Their
+                // answers, lost, find nothing waiting. The next connect,
+                // whatever brings it, restarts the tab as a reconnect does.
+                if std::mem::take(&mut workspace.woken) {
+                    for sql in workspace.sql_tabs_mut() {
+                        sql.abandon_run();
+                    }
+                    for object in workspace.object_tabs_mut() {
+                        if object.edits.saving.take().is_some() {
+                            object.edits.note = Some(crate::edit::Note::NotSent);
+                        }
+                    }
+                }
                 let rejected = matches!(error, Error::Auth(_))
                     && workspace.password_mode != PasswordMode::None;
                 if rejected {
@@ -2984,6 +3011,8 @@ impl App {
             Event::Disconnected { session, error } => {
                 if let Some(tab) = self.tab_for_session(session)
                     && let Some(workspace) = self.workspace_mut(tab)
+                    // A session closed while idle was closed on purpose.
+                    && !matches!(workspace.status, SessionStatus::Idle)
                 {
                     workspace.status = SessionStatus::Disconnected(error);
                 }
@@ -4195,6 +4224,25 @@ impl App {
         }
     }
 
+    /// The footer of a SQL editor names the server: asked for once a
+    /// session is up, where an editor is open and it is not known (a new
+    /// session, see `Workspace::forget_session_requests`, or an editor
+    /// opened while the session was closed for being idle).
+    fn ask_server_version(&mut self, tab: ConnTabId) {
+        if let Some(workspace) = self.workspace(tab)
+            && workspace.sql_tabs().next().is_some()
+            && workspace.server_version.needs_load()
+        {
+            let session = workspace.session;
+            let request = RequestId(self.next_id());
+            if let Some(workspace) = self.workspace_mut(tab) {
+                workspace.server_version.start(request);
+            }
+            self.backend
+                .send(Command::ServerVersion { session, request });
+        }
+    }
+
     /// After a (re)connect: load the tree, restart everything that waited on
     /// the old session, and open anything queued for this connection.
     pub fn after_connect(&mut self, tab: ConnTabId) {
@@ -4213,20 +4261,7 @@ impl App {
                 Driver::MySql | Driver::Sqlite => {}
             }
         }
-        // The footer of a SQL editor names the server; ask again when the
-        // session is new (see `Workspace::forget_session_requests`).
-        if let Some(workspace) = self.workspace(tab)
-            && workspace.sql_tabs().next().is_some()
-            && workspace.server_version.needs_load()
-        {
-            let session = workspace.session;
-            let request = RequestId(self.next_id());
-            if let Some(workspace) = self.workspace_mut(tab) {
-                workspace.server_version.start(request);
-            }
-            self.backend
-                .send(Command::ServerVersion { session, request });
-        }
+        self.ask_server_version(tab);
         self.refresh_tree(tab);
         let Some(workspace) = self.workspace_mut(tab) else {
             return;
@@ -4308,6 +4343,7 @@ fn catalog_of<'a>(
 
 impl App {
     pub fn load_schemas(&mut self, tab: ConnTabId) {
+        self.wake(tab);
         let request = RequestId(self.next_id());
         let Some(workspace) = self.workspace_mut(tab) else {
             return;
@@ -4318,6 +4354,7 @@ impl App {
     }
 
     pub fn load_objects(&mut self, tab: ConnTabId, schema: &str) {
+        self.wake(tab);
         let request = RequestId(self.next_id());
         let Some(workspace) = self.workspace_mut(tab) else {
             return;
@@ -4787,6 +4824,7 @@ impl App {
     /// Asks for `object`'s structure, for its columns in the completion
     /// list. Nothing waits for it but the workspace's `columns`.
     fn load_columns(&mut self, tab: ConnTabId, object: ObjectRef) {
+        self.wake(tab);
         let request = RequestId(self.next_id());
         let Some(workspace) = self.workspace_mut(tab) else {
             return;
@@ -4961,7 +4999,8 @@ impl App {
         let Some(workspace) = self.workspace(tab) else {
             return;
         };
-        if !matches!(workspace.status, SessionStatus::Connected) {
+        // One closed while idle connects again for the run.
+        if !workspace.takes_commands() {
             return;
         }
         let dialect = workspace.driver.dialect();
@@ -4998,7 +5037,7 @@ impl App {
         let Some(workspace) = self.workspace(tab) else {
             return;
         };
-        if !matches!(workspace.status, SessionStatus::Connected) {
+        if !workspace.takes_commands() {
             return;
         }
         let Some(sql) = workspace.sql_tab(id) else {
@@ -5029,6 +5068,15 @@ impl App {
         statements: Vec<tabletist_db::sql::Statement>,
         mode: tabletist_db::ScriptMode,
     ) {
+        self.wake(tab);
+        // A connect again that has to ask for a secret first is not sent
+        // yet: the run is not sent before it, and Run can be pressed again.
+        if self
+            .workspace(tab)
+            .is_some_and(|workspace| !workspace.takes_commands())
+        {
+            return;
+        }
         let request = RequestId(self.next_id());
         let Some(workspace) = self.workspace_mut(tab) else {
             return;
@@ -5198,6 +5246,7 @@ impl App {
     /// settings' size. Previous asks for the tab's size, or for the rows
     /// before a page that begins less than a page from the start.
     fn fetch_page(&mut self, tab: ConnTabId, id: TabId, limit: u32) {
+        self.wake(tab);
         let request = RequestId(self.next_id());
         let Some(workspace) = self.workspace_mut(tab) else {
             return;
@@ -5376,6 +5425,7 @@ impl App {
     }
 
     pub fn count_rows(&mut self, tab: ConnTabId, id: TabId) {
+        self.wake(tab);
         let request = RequestId(self.next_id());
         let Some(workspace) = self.workspace_mut(tab) else {
             return;
@@ -5396,6 +5446,7 @@ impl App {
     }
 
     pub fn describe(&mut self, tab: ConnTabId, id: TabId) {
+        self.wake(tab);
         let request = RequestId(self.next_id());
         let Some(workspace) = self.workspace_mut(tab) else {
             return;
@@ -5500,6 +5551,101 @@ impl App {
         // A hidden window draws no frame: its close request is answered
         // here, or nothing would hold it back.
         self.hold_close(ctx);
+        // And comes back when the next session is due to be closed.
+        if let Some(wait) = self.close_idle(std::time::Instant::now()) {
+            ctx.request_repaint_after(wait);
+        }
+    }
+
+    /// Closes the session of each connection left unused for longer than
+    /// "Disconnect when idle" gives it: its tab stays as it is, says so,
+    /// and connects again when something needs the session (see
+    /// `App::wake`). Never one whose tables hold edits or that is not
+    /// connected; one that waits for an answer is in use while it waits.
+    /// Returns how long until the next one is due.
+    pub fn close_idle(&mut self, now: std::time::Instant) -> Option<std::time::Duration> {
+        let limit = self.settings.idle_disconnect.after()?;
+        // A foreign key's list waits on its session too.
+        let looking: Vec<SessionId> = self.lookups.iter().map(|sent| sent.session).collect();
+        let mut due = Vec::new();
+        let mut next: Option<std::time::Duration> = None;
+        for tab in &mut self.tabs {
+            let ConnTabContent::Workspace(workspace) = &mut tab.content else {
+                continue;
+            };
+            if !matches!(workspace.status, SessionStatus::Connected) || workspace.holds_edits() {
+                continue;
+            }
+            if workspace.waits() || looking.contains(&workspace.session) {
+                workspace.used = now;
+            }
+            let unused = now.saturating_duration_since(workspace.used);
+            match limit.checked_sub(unused).filter(|left| !left.is_zero()) {
+                Some(left) => next = Some(next.map_or(left, |next| next.min(left))),
+                None => {
+                    workspace.status = SessionStatus::Idle;
+                    due.push(workspace.session);
+                }
+            }
+        }
+        for session in due {
+            self.backend.send(Command::Close { session });
+        }
+        next
+    }
+
+    /// Marks `tab`'s session used, and connects it again where it was
+    /// closed while idle, with the secrets the tab keeps: what is sent for
+    /// it next waits in the backend until the session is up. Everything
+    /// that sends a command a use needs calls it once it has something to
+    /// send; a cancel, a close, the server's version and the databases'
+    /// list are asked only of a session that is up, and do not.
+    fn wake(&mut self, tab: ConnTabId) {
+        let Some(workspace) = self.workspace_mut(tab) else {
+            return;
+        };
+        workspace.used = std::time::Instant::now();
+        if !matches!(workspace.status, SessionStatus::Idle) {
+            return;
+        }
+        let session = SessionId(self.next_id());
+        let request = RequestId(self.next_id());
+        let Some(workspace) = self.workspace_mut(tab) else {
+            return;
+        };
+        workspace.session = session;
+        workspace.status = SessionStatus::Connecting { request };
+        workspace.connect_started = None;
+        self.authenticate(tab);
+        // A connect that has to ask for a secret first is not sent yet:
+        // what is sent meanwhile is lost, and the tab is restarted when
+        // the session is up, as after any reconnect.
+        if let Some(workspace) = self.workspace_mut(tab) {
+            workspace.woken = workspace.connect_started.is_some();
+        }
+    }
+
+    /// The connection in front is in use while the person works in its
+    /// tab: a key, a click or a scroll in this frame.
+    fn note_use(&mut self, ctx: &egui::Context) {
+        let used = ctx.input(|input| {
+            input.events.iter().any(|event| {
+                matches!(
+                    event,
+                    egui::Event::Key { .. }
+                        | egui::Event::Text(_)
+                        | egui::Event::PointerButton { .. }
+                        | egui::Event::MouseWheel { .. }
+                        | egui::Event::Paste(_)
+                        | egui::Event::Copy
+                        | egui::Event::Cut
+                )
+            })
+        });
+        let tab = self.active_tab_id();
+        if used && let Some(workspace) = self.workspace_mut(tab) {
+            workspace.used = std::time::Instant::now();
+        }
     }
 
     /// The foreign key picker's searches wait for a pause in the typing:
@@ -5513,6 +5659,7 @@ impl App {
 
     /// Draws one frame, then applies what the frame asked for.
     pub fn frame_ui(&mut self, ui: &mut egui::Ui) {
+        self.note_use(ui.ctx());
         self.poll_backend();
         self.apply_actions();
         self.tick_lookups(ui.ctx());
diff --git a/src/app/editing.rs b/src/app/editing.rs
index 777a1aa..11fab5e 100644
--- a/src/app/editing.rs
+++ b/src/app/editing.rs
@@ -14,7 +14,7 @@ use crate::edit::{
 };
 use crate::model::{
     Action, Advance, CellPos, ConflictPrompt, ConnTabId, Dialog, EditStart, Held, LeavePrompt,
-    ObjectTab, ObjectView, Pane, Reach, SaveBlock, SessionStatus, TabId, WritePrompt,
+    ObjectTab, ObjectView, Pane, Reach, SaveBlock, TabId, WritePrompt,
 };
 use crate::review::{Part, Values};
 
@@ -461,7 +461,8 @@ impl App {
         if lacking == Some(true) {
             return Some(SaveBlock::Required);
         }
-        if !matches!(workspace.status, SessionStatus::Connected) {
+        // One closed while idle connects again for the save.
+        if !workspace.takes_commands() {
             return Some(SaveBlock::Disconnected);
         }
         if workspace.access == Access::ReadOnly {
@@ -675,6 +676,18 @@ impl App {
         sent: Sent,
         then: Option<Held>,
     ) {
+        self.wake(tab);
+        // A connect again that has to ask for a secret first is not sent
+        // yet, and nothing goes before it: the tab says nothing was sent.
+        if self
+            .workspace(tab)
+            .is_some_and(|workspace| !workspace.takes_commands())
+        {
+            if let Some(object) = self.object_tab_mut(tab, id) {
+                object.edits.note = Some(Note::NotSent);
+            }
+            return;
+        }
         let request = RequestId(self.next_id());
         let Some(workspace) = self.workspace_mut(tab) else {
             return;
@@ -1949,6 +1962,7 @@ impl App {
         if asked.is_empty() {
             return;
         }
+        self.wake(tab);
         let Some(session) = self.workspace(tab).map(|workspace| workspace.session) else {
             return;
         };
@@ -2336,6 +2350,9 @@ impl App {
             })
             .flatten()
             .unwrap_or_default();
+        if !asked.is_empty() {
+            self.wake(tab);
+        }
         let Some(session) = self.workspace(tab).map(|workspace| workspace.session) else {
             return;
         };
diff --git a/src/app/key_picker.rs b/src/app/key_picker.rs
index 6f3dbfd..157d1fe 100644
--- a/src/app/key_picker.rs
+++ b/src/app/key_picker.rs
@@ -148,9 +148,15 @@ impl App {
                 key_picker::pinned(key, class, &column.type_name, passes)
             };
             let reads = picker.lookups(label.column.clone(), &text, pin);
-            Some((workspace.session, reads, text, picker.serial))
+            Some((reads, text, picker.serial))
         });
-        let Some((session, (pinned, matching), text, serial)) = planned else {
+        let Some(((pinned, matching), text, serial)) = planned else {
+            return;
+        };
+        // Only now, with something to send: it opens a session closed
+        // while idle.
+        self.wake(tab);
+        let Some(session) = self.workspace(tab).map(|workspace| workspace.session) else {
             return;
         };
         let pinned = pinned.map(|lookup| self.send_lookup(session, lookup, serial));
@@ -421,12 +427,16 @@ impl App {
                 part: LookupPart::Pinned(value),
                 limit: 1,
             };
-            Some((workspace.session, lookup, text.clone()))
+            Some((lookup, text.clone()))
         });
         if let Some(object) = self.object_tab_mut(tab, id) {
             object.edits.doubts.remove(&(cell.row, cell.col));
         }
-        let Some((session, lookup, text)) = planned else {
+        let Some((lookup, text)) = planned else {
+            return;
+        };
+        self.wake(tab);
+        let Some(session) = self.workspace(tab).map(|workspace| workspace.session) else {
             return;
         };
         // No picker's: serial 0 is nobody's.
diff --git a/src/model.rs b/src/model.rs
index 7c235ff..1c63604 100644
--- a/src/model.rs
+++ b/src/model.rs
@@ -967,6 +967,13 @@ pub struct Workspace {
     /// When the session last connected, in seconds since the Unix epoch
     /// (the connection bar's card says how long ago).
     pub connected_at: Option<u64>,
+    /// When the session was last used: a command sent for it, or a key, a
+    /// click or a scroll in the connection's tab while it is in front.
+    pub used: std::time::Instant,
+    /// The connect under way opens again a session closed while idle,
+    /// for a use: what the use sent waits for it in the backend, and what
+    /// the tab shows stays as it is.
+    pub woken: bool,
 }
 
 /// How many objects the sidebar's Recent section keeps.
@@ -981,6 +988,10 @@ pub enum SessionStatus {
     Disconnected(tabletist_db::Error),
     /// The user cancelled a password or SSH secret prompt.
     Cancelled,
+    /// Closed after it went unused for the time the settings give
+    /// (`safety.idle_disconnect`). The tab stays as it was, and connects
+    /// again when something needs its session.
+    Idle,
 }
 
 #[derive(Debug, Default, PartialEq)]
@@ -4339,6 +4350,8 @@ impl Workspace {
             label_refused: None,
             fold_documents: None,
             connected_at: None,
+            used: std::time::Instant::now(),
+            woken: false,
         }
     }
 
@@ -4428,6 +4441,39 @@ impl Workspace {
         }
     }
 
+    /// Whether what is sent for the session now is answered: it is up, it
+    /// was closed while idle (sending opens it again), or it is opening
+    /// again for a use, and what is sent waits for it.
+    pub fn takes_commands(&self) -> bool {
+        match self.status {
+            SessionStatus::Connected | SessionStatus::Idle => true,
+            SessionStatus::Connecting { .. } => self.woken,
+            SessionStatus::Disconnected(_) | SessionStatus::Cancelled => false,
+        }
+    }
+
+    /// Whether a table tab holds edits: something pending, an editor open,
+    /// or a save running.
+    pub fn holds_edits(&self) -> bool {
+        self.object_tabs().any(|object| object.edits.holds())
+    }
+
+    /// Whether something waits for the session's answer: a tab's page,
+    /// structure, count, run or save, the tree, the databases, the
+    /// server's version, or a table's columns.
+    pub fn waits(&self) -> bool {
+        self.tabs.iter().any(|tab| !tab.pending().is_empty())
+            || self.tree.schemas.is_loading()
+            || self
+                .tree
+                .nodes
+                .values()
+                .any(|node| node.objects.is_loading())
+            || self.databases.is_loading()
+            || self.server_version.is_loading()
+            || self.columns.values().any(Fetch::is_loading)
+    }
+
     /// The object tabs, in strip order.
     pub fn object_tabs(&self) -> impl Iterator<Item = &ObjectTab> {
         self.tabs.iter().filter_map(Tab::as_object)
diff --git a/src/settings.rs b/src/settings.rs
index 89cec9f..c25911e 100644
--- a/src/settings.rs
+++ b/src/settings.rs
@@ -201,6 +201,67 @@ impl EnvBar {
     }
 }
 
+/// How long a connection may go unused before its session is closed.
+#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
+pub enum Idle {
+    #[serde(rename = "never")]
+    Never,
+    #[serde(rename = "5m")]
+    Minutes5,
+    #[serde(rename = "15m")]
+    Minutes15,
+    #[default]
+    #[serde(rename = "30m")]
+    Minutes30,
+    #[serde(rename = "1h")]
+    Hour1,
+    #[serde(rename = "4h")]
+    Hours4,
+}
+
+impl Idle {
+    /// Every choice, as the menu lists them: never first, then from the
+    /// shortest wait.
+    pub const ALL: [Self; 6] = [
+        Self::Never,
+        Self::Minutes5,
+        Self::Minutes15,
+        Self::Minutes30,
+        Self::Hour1,
+        Self::Hours4,
+    ];
+
+    /// As the file says it.
+    pub fn name(self) -> &'static str {
+        match self {
+            Self::Never => "never",
+            Self::Minutes5 => "5m",
+            Self::Minutes15 => "15m",
+            Self::Minutes30 => "30m",
+            Self::Hour1 => "1h",
+            Self::Hours4 => "4h",
+        }
+    }
+
+    pub fn from_name(name: &str) -> Option<Self> {
+        Self::ALL.into_iter().find(|choice| choice.name() == name)
+    }
+
+    /// How long a session goes unused before it is closed; `None` never
+    /// closes it.
+    pub fn after(self) -> Option<std::time::Duration> {
+        let minutes = match self {
+            Self::Never => return None,
+            Self::Minutes5 => 5,
+            Self::Minutes15 => 15,
+            Self::Minutes30 => 30,
+            Self::Hour1 => 60,
+            Self::Hours4 => 240,
+        };
+        Some(std::time::Duration::from_secs(minutes * 60))
+    }
+}
+
 /// Everything the user can set. A key the file lacks takes its default and
 /// one this version does not know is ignored, so older and newer files both
 /// load. The serde derive reads the `settings.json` of versions before the
@@ -231,6 +292,9 @@ pub struct Settings {
     /// asks first. Off, the edits are dropped and the action goes ahead,
     /// except while a save runs or a dialog is open.
     pub confirm_close_edits: bool,
+    /// How long a connection goes unused before its session is closed. Its
+    /// tab stays, and connects again when it is next used.
+    pub idle_disconnect: Idle,
     /// Show `pg_catalog`, `information_schema`, `mysql`, `sys` and friends.
     pub show_system_schemas: bool,
     /// A palette file name in the themes directory. `None` follows the
@@ -273,6 +337,7 @@ pub enum Key {
     ValueTags,
     ProductionReadOnly,
     ConfirmCloseEdits,
+    IdleDisconnect,
     ShowSystemSchemas,
     SqlLimit,
     SqlTimeoutSecs,
@@ -289,7 +354,7 @@ pub enum Key {
 
 impl Key {
     /// In the order the file has them: a table's keys together.
-    pub const ALL: [Key; 19] = [
+    pub const ALL: [Key; 20] = [
         Self::PageSize,
         Self::Timestamps,
         Self::Tz,
@@ -297,6 +362,7 @@ impl Key {
         Self::ValueTags,
         Self::ProductionReadOnly,
         Self::ConfirmCloseEdits,
+        Self::IdleDisconnect,
         Self::ShowSystemSchemas,
         Self::SqlLimit,
         Self::SqlTimeoutSecs,
@@ -321,6 +387,7 @@ impl Key {
             Self::ValueTags => ("data", "value_tags"),
             Self::ProductionReadOnly => ("safety", "production_read_only"),
             Self::ConfirmCloseEdits => ("safety", "confirm_close_edits"),
+            Self::IdleDisconnect => ("safety", "idle_disconnect"),
             Self::ShowSystemSchemas => ("sidebar", "show_system_schemas"),
             Self::SqlLimit => ("editor", "sql_limit"),
             Self::SqlTimeoutSecs => ("editor", "sql_timeout_secs"),
@@ -342,6 +409,7 @@ impl Key {
         match self {
             Self::Timestamps => Some("second | full"),
             Self::Tz => Some("system | utc | \"Area/City\""),
+            Self::IdleDisconnect => Some("never | 5m | 15m | 30m | 1h | 4h"),
             Self::SqlTimeoutSecs => Some("0 waits forever"),
             Self::RowHeight => Some("compact | regular | roomy"),
             Self::TableList => Some("tree | flat"),
@@ -1158,6 +1226,7 @@ impl Settings {
             Key::ValueTags => self.value_tags.to_string(),
             Key::ProductionReadOnly => self.production_read_only.to_string(),
             Key::ConfirmCloseEdits => self.confirm_close_edits.to_string(),
+            Key::IdleDisconnect => toml_string(self.idle_disconnect.name()),
             Key::ShowSystemSchemas => self.show_system_schemas.to_string(),
             Key::SqlLimit => self.sql_limit.to_string(),
             // TOML has no null: no timeout is no seconds.
@@ -1392,6 +1461,9 @@ impl Settings {
             Key::ValueTags => flag.map(|flag| self.value_tags = flag),
             Key::ProductionReadOnly => flag.map(|flag| self.production_read_only = flag),
             Key::ConfirmCloseEdits => flag.map(|flag| self.confirm_close_edits = flag),
+            Key::IdleDisconnect => text
+                .and_then(Idle::from_name)
+                .map(|choice| self.idle_disconnect = choice),
             Key::ShowSystemSchemas => flag.map(|flag| self.show_system_schemas = flag),
             Key::SqlLimit => number.map(|number| self.sql_limit = number),
             Key::SqlTimeoutSecs => number.map(|number| self.sql_timeout_secs = Some(number)),
@@ -1520,6 +1592,7 @@ impl Settings {
             Key::ValueTags => self.value_tags = other.value_tags,
             Key::ProductionReadOnly => self.production_read_only = other.production_read_only,
             Key::ConfirmCloseEdits => self.confirm_close_edits = other.confirm_close_edits,
+            Key::IdleDisconnect => self.idle_disconnect = other.idle_disconnect,
             Key::ShowSystemSchemas => self.show_system_schemas = other.show_system_schemas,
             Key::SqlLimit => self.sql_limit = other.sql_limit,
             Key::SqlTimeoutSecs => self.sql_timeout_secs = other.sql_timeout_secs,
@@ -1584,6 +1657,7 @@ impl Default for Settings {
             value_tags: true,
             production_read_only: true,
             confirm_close_edits: true,
+            idle_disconnect: Idle::Minutes30,
             show_system_schemas: false,
             custom_theme: None,
             sql_limit: 1_000,
diff --git a/src/ui/sql_results.rs b/src/ui/sql_results.rs
index 14c9d21..6cc75e8 100644
--- a/src/ui/sql_results.rs
+++ b/src/ui/sql_results.rs
@@ -1327,10 +1327,9 @@ fn blocked<'a>(workspace: &'a Workspace, sql: &'a SqlTab) -> Option<Blocked<'a>>
         error,
         statement: run.statements.get(index),
         why,
-        // On a session that would take it: a run is sent only to a
-        // connected one.
-        again: sql.ran_this_text()
-            && matches!(workspace.status, crate::model::SessionStatus::Connected),
+        // On a session that would take it: a run is sent only to one that
+        // is connected, or that connects again for it.
+        again: sql.ran_this_text() && workspace.takes_commands(),
         workspace,
     })
 }
diff --git a/src/ui/workspace.rs b/src/ui/workspace.rs
index d61231d..09800e7 100644
--- a/src/ui/workspace.rs
+++ b/src/ui/workspace.rs
@@ -67,8 +67,9 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
         return;
     };
     let active = workspace.active_tab;
-    // What a lost connection left on screen reads as old.
-    let stale = !matches!(workspace.status, SessionStatus::Connected);
+    // What a lost connection left on screen reads as old. A session closed
+    // while idle, or opening again for a use, left the page as it was.
+    let stale = !workspace.takes_commands();
     let view = workspace.active_object_tab().map(|object| object.view);
     // The tab the row panel shows a row of: a table's Data view, or a SQL
     // editor with a row of its result selected.
@@ -476,6 +477,8 @@ enum Link {
     Connecting,
     Disconnected,
     Cancelled,
+    /// Closed while idle: it connects again when it is used.
+    Idle,
 }
 
 /// One open connection in the bar.
@@ -545,6 +548,7 @@ fn chips(app: &App, own: ConnTabId) -> Vec<Chip> {
                 SessionStatus::Connecting { .. } => (Link::Connecting, Some("Connecting…")),
                 SessionStatus::Disconnected(_) => (Link::Disconnected, Some("Disconnected")),
                 SessionStatus::Cancelled => (Link::Cancelled, Some("Cancelled")),
+                SessionStatus::Idle => (Link::Idle, Some("Disconnected while idle")),
             };
             let line = match state {
                 Some(state) => app.look.label(&gettext(app.locale, state)),
@@ -629,6 +633,7 @@ fn card_rows(
         SessionStatus::Connecting { .. } => (say("Status"), say("Connecting…")),
         SessionStatus::Disconnected(_) => (say("Status"), say("Disconnected")),
         SessionStatus::Cancelled => (say("Status"), say("Cancelled")),
+        SessionStatus::Idle => (say("Status"), say("Disconnected while idle")),
     };
     rows.push(state);
     rows
@@ -805,7 +810,7 @@ fn link_color(link: Link, env: &crate::env::EnvColors, palette: &Palette) -> egu
         Link::Connected => env.base(),
         Link::Connecting => palette.warning,
         Link::Disconnected => palette.danger,
-        Link::Cancelled => palette.dim,
+        Link::Cancelled | Link::Idle => palette.dim,
     }
 }
 
@@ -2739,7 +2744,10 @@ fn banner(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
     let name = display_safe(&workspace.name).into_owned();
     // What happened, the plain sentence when it says more, the exact error.
     let (lead, sentence, raw, reconnecting) = match &workspace.status {
-        SessionStatus::Connected => return,
+        // Closed on purpose: nothing went wrong, and the next use connects,
+        // with no strip of its own.
+        SessionStatus::Connected | SessionStatus::Idle => return,
+        SessionStatus::Connecting { .. } if workspace.woken => return,
         SessionStatus::Connecting { .. } => (
             format!("{} {name}…", say("Reconnecting to")),
             String::new(),
````

- [ ] **Step 4: Check**

Run: `cargo fmt --all && cargo clippy --locked --workspace --all-targets -- -D warnings && cargo test --locked --workspace --all-targets`
Expected: no warning, every test passes.

- [ ] **Step 5: Commit**

```bash
git add docs/_reference/settings-and-files.md src/app.rs src/app/editing.rs src/app/key_picker.rs src/entrypoint.rs src/model.rs src/settings.rs src/ui/sql_results.rs src/ui/workspace.rs
git commit -m "Close a session left unused, and connect it again when it is next used"
```

### Task 5: Disconnect when idle in Settings › General

The row in both looks: a menu on macOS and Windows, a stepper on Omarchy. The option is the last of `OptionId::SAFETY`, so both looks draw it under the Safety heading with the other two. Reset, which takes every option of `OptionId::GENERAL`, puts it back with the other two Safety options; its test now says so for all three.

**Files:** Modify `docs/_reference/settings-and-files.md`; Modify `src/app.rs`; Modify `src/model.rs`; Modify `src/settings.rs`; Modify `src/ui/mod.rs`; Modify `src/ui/settings/mod.rs`; Modify `src/ui/settings/sheet.rs`; Modify `src/ui/settings/terminal.rs`.

- [ ] **Step 1: Write the failing tests**

````diff
diff --git a/src/app.rs b/src/app.rs
index 00d6a9c..8f05ace 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -8389,6 +8389,9 @@ mod tests {
             settings.page_size = 500;
             settings.tz = crate::settings::Zone::Utc;
             settings.group_digits = Some(false);
+            settings.production_read_only = false;
+            settings.confirm_close_edits = false;
+            settings.idle_disconnect = crate::settings::Idle::Never;
             settings.show_system_schemas = true;
             settings.sql_limit = 50;
         });
@@ -8411,6 +8414,10 @@ mod tests {
         assert_eq!(app.settings.page_size, defaults.page_size);
         assert_eq!(app.settings.tz, defaults.tz);
         assert_eq!(app.settings.group_digits, defaults.group_digits);
+        // The Safety group's too.
+        assert!(app.settings.production_read_only);
+        assert!(app.settings.confirm_close_edits);
+        assert_eq!(app.settings.idle_disconnect, defaults.idle_disconnect);
         assert!(app.settings.show_system_schemas);
         assert_eq!(app.settings.sql_limit, 50);
         assert_eq!(app.settings_file.text, app.settings.to_toml());
diff --git a/src/settings.rs b/src/settings.rs
index c25911e..11155dc 100644
--- a/src/settings.rs
+++ b/src/settings.rs
@@ -2900,6 +2900,7 @@ bar = \"label\"  # tint | label
                 Key::ValueTags,
                 Key::ProductionReadOnly,
                 Key::ConfirmCloseEdits,
+                Key::IdleDisconnect,
                 Key::RowHeight,
                 Key::Zebra,
                 Key::ColumnTypes,
@@ -2950,6 +2951,38 @@ bar = \"label\"  # tint | label
         assert_eq!(step(9_000, false), OptionValue::PageSize(5_000));
     }
 
+    #[test]
+    fn the_idle_time_steps_through_its_choices_and_stops_at_the_ends() {
+        let step = |choice: Idle, forward: bool| {
+            let settings = Settings {
+                idle_disconnect: choice,
+                ..Settings::default()
+            };
+            settings.stepped(OptionId::IdleDisconnect, forward)
+        };
+        assert_eq!(
+            step(Idle::Minutes30, true),
+            OptionValue::IdleDisconnect(Idle::Hour1)
+        );
+        assert_eq!(
+            step(Idle::Minutes30, false),
+            OptionValue::IdleDisconnect(Idle::Minutes15)
+        );
+        assert_eq!(
+            step(Idle::Minutes5, false),
+            OptionValue::IdleDisconnect(Idle::Never)
+        );
+        assert_eq!(
+            step(Idle::Never, false),
+            OptionValue::IdleDisconnect(Idle::Never)
+        );
+        assert_eq!(
+            step(Idle::Hours4, true),
+            OptionValue::IdleDisconnect(Idle::Hours4)
+        );
+        assert_eq!(Settings::default().flipped(OptionId::IdleDisconnect), None);
+    }
+
     #[test]
     fn the_logs_days_step_through_their_choices_and_stop_at_the_ends() {
         let step = |days: u32, forward: bool| {
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
index 9d39af9..48c6a27 100644
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -1376,6 +1376,30 @@ value_tags   = true
             assert!(!harness.app.settings.confirm_close_edits);
             let text = &harness.app.settings_file.text;
             assert!(text.ends_with("confirm_close_edits = false\n"), "{text}");
+            // The menu of how long a connection may go unused.
+            assert!(harness.has("Closes server sessions; tabs stay and reconnect on use"));
+            harness.click("Disconnect when idle");
+            for choice in [
+                "Never",
+                "After 5 min",
+                "After 15 min",
+                "After 30 min",
+                "After 1 h",
+                "After 4 h",
+            ] {
+                let entry = format!("Disconnect when idle {choice}");
+                assert!(harness.has(&entry), "{entry}");
+            }
+            harness.click("Disconnect when idle After 1 h");
+            assert_eq!(
+                harness.app.settings.idle_disconnect,
+                crate::settings::Idle::Hour1
+            );
+            let text = &harness.app.settings_file.text;
+            assert!(
+                text.ends_with("idle_disconnect = \"1h\"  # never | 5m | 15m | 30m | 1h | 4h\n"),
+                "{text}"
+            );
         }
     }
 
@@ -1408,6 +1432,9 @@ value_tags   = true
             "production opens read-only",
             "default for new connections",
             "confirm close with edits",
+            "disconnect when idle",
+            "30m",
+            "tabs reconnect on use",
         ] {
             assert!(harness.painted_color(text).is_some(), "{text}");
         }
@@ -1432,6 +1459,20 @@ value_tags   = true
         assert!(!harness.app.settings.production_read_only);
         harness.press(egui::Key::R, egui::Modifiers::SHIFT);
         assert!(harness.app.settings.production_read_only);
+        // Two rows on: the idle time, stepped.
+        harness.press(egui::Key::J, egui::Modifiers::NONE);
+        harness.press(egui::Key::J, egui::Modifiers::NONE);
+        harness.press(egui::Key::L, egui::Modifiers::NONE);
+        assert_eq!(
+            harness.app.settings.idle_disconnect,
+            crate::settings::Idle::Hour1
+        );
+        assert!(harness.painted_color("‹ 1h ›").is_some());
+        harness.press(egui::Key::R, egui::Modifiers::SHIFT);
+        assert_eq!(
+            harness.app.settings.idle_disconnect,
+            crate::settings::Idle::Minutes30
+        );
     }
 
     #[test]
@@ -1635,12 +1676,12 @@ value_tags   = true
             let mut harness = Harness::with_size(size);
             harness.set_look(look());
             harness.press(egui::Key::Comma, egui::Modifiers::COMMAND);
-            // The last row's switch: the window has room for one row or
+            // The last row's menu: the window has room for one row or
             // two, and it is under them.
-            let last = "Ask before closing with unsaved edits";
+            let last = "Disconnect when idle";
             tab_to_settings(&mut harness, last);
             let tree = harness.settle();
-            let switch = crate::testing::bounds(&tree, last, Role::CheckBox).expect("the switch");
+            let switch = crate::testing::bounds(&tree, last, Role::ComboBox).expect("the menu");
             // The rows came with the keyboard: the switch is in the
             // window, between the tab's lead and the footer.
             let window = egui::Rect::from_min_size(egui::Pos2::ZERO, size);
@@ -1651,7 +1692,7 @@ value_tags   = true
             let lead = harness.painted_rect(lead).expect("the tab's lead");
             assert!(switch.top() > lead.bottom(), "{switch:?} {lead:?}");
             // And back up with it: the first row's menu.
-            for _ in 0..6 {
+            for _ in 0..7 {
                 harness.press(egui::Key::Tab, egui::Modifiers::SHIFT);
             }
             let tree = harness.settle();
````

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --locked -p tabletist --lib`
Expected: FAIL to compile: `` no variant, associated function, or constant named `IdleDisconnect` found for enum `settings::OptionId` in the current scope ``; `` no variant, associated function, or constant named `IdleDisconnect` found for enum `settings::OptionValue` in the current scope ``.

- [ ] **Step 3: Write the code**

````diff
diff --git a/docs/_reference/settings-and-files.md b/docs/_reference/settings-and-files.md
index 8b99727..a27bacf 100644
--- a/docs/_reference/settings-and-files.md
+++ b/docs/_reference/settings-and-files.md
@@ -19,6 +19,7 @@ Cmd/Ctrl+, opens Settings. Changes apply right away.
 | **Value tags** | Colors for enum, CHECK and boolean columns | On |
 | **New production connections open read-only** | On or off: where the **Open read-only** box of a new production connection starts. A connection you saved keeps its own box | On |
 | **Ask before closing with unsaved edits** | On or off. Off, closing a table, its connection or the window, or loading another page of it, discards its pending changes without asking | On |
+| **Disconnect when idle** | Never, or after 5, 15 or 30 minutes, 1 hour or 4 hours unused. The server session is closed; the tabs stay, and the connection connects again when something next needs it: a refresh, a page, a table opened, a run or a save | After 30 min |
 
 A second pane, **Appearance**, sets how the grids and the connection bar
 are drawn. Each change shows in every open window at once.
diff --git a/src/model.rs b/src/model.rs
index 1c63604..a489646 100644
--- a/src/model.rs
+++ b/src/model.rs
@@ -2493,6 +2493,7 @@ impl SettingsPane {
                 SettingsRow::Option(OptionId::ValueTags),
                 SettingsRow::Option(OptionId::ProductionReadOnly),
                 SettingsRow::Option(OptionId::ConfirmCloseEdits),
+                SettingsRow::Option(OptionId::IdleDisconnect),
             ],
             Self::Appearance => &[
                 SettingsRow::Option(OptionId::RowHeight),
diff --git a/src/settings.rs b/src/settings.rs
index 11155dc..fed1ebf 100644
--- a/src/settings.rs
+++ b/src/settings.rs
@@ -433,6 +433,9 @@ pub enum OptionId {
     ProductionReadOnly,
     /// Whether dropping unsaved edits asks first: `safety.confirm_close_edits`.
     ConfirmCloseEdits,
+    /// How long a connection goes unused before its session is closed:
+    /// `safety.idle_disconnect`.
+    IdleDisconnect,
     /// Whether agents can reach the app: `mcp.enabled`.
     AgentsOn,
     /// How long the log of agents' calls keeps one: `mcp.log_days`.
@@ -449,7 +452,7 @@ pub enum OptionId {
 impl OptionId {
     /// Every option, the General pane's, the Appearance pane's and then
     /// the Agents pane's.
-    pub const ALL: [OptionId; 15] = [
+    pub const ALL: [OptionId; 16] = [
         Self::PageSize,
         Self::Timestamps,
         Self::Tz,
@@ -457,6 +460,7 @@ impl OptionId {
         Self::ValueTags,
         Self::ProductionReadOnly,
         Self::ConfirmCloseEdits,
+        Self::IdleDisconnect,
         Self::RowHeight,
         Self::Zebra,
         Self::ColumnTypes,
@@ -467,7 +471,7 @@ impl OptionId {
         Self::AgentsNotify,
     ];
     /// The General pane's, in the order it has them.
-    pub const GENERAL: [OptionId; 7] = [
+    pub const GENERAL: [OptionId; 8] = [
         Self::PageSize,
         Self::Timestamps,
         Self::Tz,
@@ -475,6 +479,7 @@ impl OptionId {
         Self::ValueTags,
         Self::ProductionReadOnly,
         Self::ConfirmCloseEdits,
+        Self::IdleDisconnect,
     ];
     /// The General pane's sections: Data, then Safety.
     pub const DATA: [OptionId; 5] = [
@@ -484,7 +489,11 @@ impl OptionId {
         Self::GroupDigits,
         Self::ValueTags,
     ];
-    pub const SAFETY: [OptionId; 2] = [Self::ProductionReadOnly, Self::ConfirmCloseEdits];
+    pub const SAFETY: [OptionId; 3] = [
+        Self::ProductionReadOnly,
+        Self::ConfirmCloseEdits,
+        Self::IdleDisconnect,
+    ];
     /// The Appearance pane's, in the order it has them: its Grid
     /// section's, then its Environment section's.
     pub const APPEARANCE: [OptionId; 5] = [
@@ -514,6 +523,7 @@ impl OptionId {
             Self::ValueTags => Key::ValueTags,
             Self::ProductionReadOnly => Key::ProductionReadOnly,
             Self::ConfirmCloseEdits => Key::ConfirmCloseEdits,
+            Self::IdleDisconnect => Key::IdleDisconnect,
             Self::AgentsOn => Key::McpEnabled,
             Self::LogDays => Key::McpLogDays,
             Self::AgentsNotify => Key::McpNotify,
@@ -542,6 +552,7 @@ pub enum OptionValue {
     ValueTags(bool),
     ProductionReadOnly(bool),
     ConfirmCloseEdits(bool),
+    IdleDisconnect(Idle),
     AgentsOn(bool),
     LogDays(u32),
     AgentsNotify(bool),
@@ -563,6 +574,7 @@ impl OptionValue {
             Self::ValueTags(_) => OptionId::ValueTags,
             Self::ProductionReadOnly(_) => OptionId::ProductionReadOnly,
             Self::ConfirmCloseEdits(_) => OptionId::ConfirmCloseEdits,
+            Self::IdleDisconnect(_) => OptionId::IdleDisconnect,
             Self::AgentsOn(_) => OptionId::AgentsOn,
             Self::LogDays(_) => OptionId::LogDays,
             Self::AgentsNotify(_) => OptionId::AgentsNotify,
@@ -584,6 +596,7 @@ impl OptionValue {
             Self::ValueTags(on) => settings.value_tags = on,
             Self::ProductionReadOnly(on) => settings.production_read_only = on,
             Self::ConfirmCloseEdits(on) => settings.confirm_close_edits = on,
+            Self::IdleDisconnect(choice) => settings.idle_disconnect = choice,
             Self::AgentsOn(on) => settings.mcp_enabled = on,
             Self::LogDays(days) => settings.mcp_log_days = days,
             Self::AgentsNotify(on) => settings.mcp_notify = on,
@@ -1076,6 +1089,7 @@ impl Settings {
                 OptionValue::ProductionReadOnly(self.production_read_only)
             }
             OptionId::ConfirmCloseEdits => OptionValue::ConfirmCloseEdits(self.confirm_close_edits),
+            OptionId::IdleDisconnect => OptionValue::IdleDisconnect(self.idle_disconnect),
             OptionId::AgentsOn => OptionValue::AgentsOn(self.mcp_enabled),
             OptionId::LogDays => OptionValue::LogDays(self.mcp_log_days),
             OptionId::AgentsNotify => OptionValue::AgentsNotify(self.mcp_notify),
@@ -1124,6 +1138,19 @@ impl Settings {
             OptionId::ValueTags => OptionValue::ValueTags(forward),
             OptionId::ProductionReadOnly => OptionValue::ProductionReadOnly(forward),
             OptionId::ConfirmCloseEdits => OptionValue::ConfirmCloseEdits(forward),
+            // Never is the first: the shortest wait is one to the right.
+            OptionId::IdleDisconnect => {
+                let at = Idle::ALL
+                    .iter()
+                    .position(|choice| *choice == self.idle_disconnect)
+                    .unwrap_or(0);
+                let next = if forward {
+                    (at + 1).min(Idle::ALL.len() - 1)
+                } else {
+                    at.saturating_sub(1)
+                };
+                OptionValue::IdleDisconnect(Idle::ALL[next])
+            }
             OptionId::AgentsOn => OptionValue::AgentsOn(forward),
             OptionId::LogDays => {
                 let days = self.mcp_log_days;
@@ -1182,7 +1209,7 @@ impl Settings {
     /// The other value of an option that has two; `None` for one with more.
     pub fn flipped(&self, option: OptionId) -> Option<OptionValue> {
         match option {
-            OptionId::PageSize | OptionId::Tz => None,
+            OptionId::PageSize | OptionId::Tz | OptionId::IdleDisconnect => None,
             OptionId::Timestamps => Some(OptionValue::Timestamps(match self.timestamps {
                 Timestamps::Second => Timestamps::Full,
                 Timestamps::Full => Timestamps::Second,
diff --git a/src/ui/settings/mod.rs b/src/ui/settings/mod.rs
index b818f0a..e85f249 100644
--- a/src/ui/settings/mod.rs
+++ b/src/ui/settings/mod.rs
@@ -17,7 +17,7 @@ use crate::i18n::{Locale, gettext, ngettext};
 use crate::keymap::{Command, Keymap, Layout, Scope, Spelled, Stroke};
 use crate::model::{Action, AgentsView, Dialog, SettingsDialog, SettingsPane, SettingsRow};
 use crate::settings::{
-    EnvBar, OptionId, OptionValue, RowHeight, Settings, TableList, Timestamps, Zone,
+    EnvBar, Idle, OptionId, OptionValue, RowHeight, Settings, TableList, Timestamps, Zone,
 };
 
 /// The label of an option's row. English: the caller translates, and the
@@ -31,6 +31,7 @@ pub(super) fn label(option: OptionId) -> &'static str {
         OptionId::ValueTags => "Value tags",
         OptionId::ProductionReadOnly => "New production connections open read-only",
         OptionId::ConfirmCloseEdits => "Ask before closing with unsaved edits",
+        OptionId::IdleDisconnect => "Disconnect when idle",
         OptionId::AgentsOn => "Let agents use Tabletist",
         OptionId::LogDays => "Keep the activity log",
         OptionId::AgentsNotify => "Notify when an agent connects",
@@ -59,6 +60,7 @@ pub(super) fn terminal_label(option: OptionId) -> &'static str {
         | OptionId::Timestamps
         | OptionId::GroupDigits
         | OptionId::ValueTags
+        | OptionId::IdleDisconnect
         | OptionId::AgentsOn
         | OptionId::LogDays
         | OptionId::AgentsNotify
@@ -74,6 +76,7 @@ pub(super) fn small_print(option: OptionId) -> Option<&'static str> {
         OptionId::LogDays => Some("Every call and its SQL, never the rows. On this computer only"),
         OptionId::Tz => Some("Values are shown in this zone and sent with their offset"),
         OptionId::ProductionReadOnly => Some("Default for the switch in each connection"),
+        OptionId::IdleDisconnect => Some("Closes server sessions; tabs stay and reconnect on use"),
         OptionId::ColumnTypes => Some("Second line in headers: int8, varchar, jsonb"),
         OptionId::TableList => Some("Default for new connections; the sidebar toggle overrides it"),
         OptionId::EnvBar => Some("Production always uses the full tint"),
@@ -102,6 +105,19 @@ pub(super) fn zone_text(locale: Locale, zone: &Zone, system: Option<&str>) -> St
     }
 }
 
+/// How long a connection may go unused, as the menu writes it.
+pub(super) fn idle_text(locale: Locale, choice: Idle) -> String {
+    let (words, count) = match choice {
+        Idle::Never => return gettext(locale, "Never").into_owned(),
+        Idle::Minutes5 => ("After {n} min", 5),
+        Idle::Minutes15 => ("After {n} min", 15),
+        Idle::Minutes30 => ("After {n} min", 30),
+        Idle::Hour1 => ("After {n} h", 1),
+        Idle::Hours4 => ("After {n} h", 4),
+    };
+    gettext(locale, words).replace("{n}", &count.to_string())
+}
+
 /// The days the log keeps a call, as the window writes them.
 pub(super) fn days(locale: Locale, days: u32) -> String {
     ngettext(locale, "{n} day", "{n} days", days).replace("{n}", &days.to_string())
@@ -214,7 +230,7 @@ pub(super) fn choices(
             (Word("off"), OptionValue::AgentsOn(false)),
             (Word("on"), OptionValue::AgentsOn(true)),
         ],
-        OptionId::LogDays => vec![
+        OptionId::LogDays | OptionId::IdleDisconnect => vec![
             (Word("fewer"), settings.stepped(option, false)),
             (Word("more"), settings.stepped(option, true)),
         ],
diff --git a/src/ui/settings/sheet.rs b/src/ui/settings/sheet.rs
index 4e3bc19..f4bc5f8 100644
--- a/src/ui/settings/sheet.rs
+++ b/src/ui/settings/sheet.rs
@@ -10,7 +10,7 @@ use crate::app::App;
 use crate::i18n::{Locale, gettext, ngettext};
 use crate::model::{Action, SettingsDialog, SettingsPane};
 use crate::settings::{
-    EnvBar, OptionId, OptionValue, RowHeight, Settings, TableList, Timestamps, Zone,
+    EnvBar, Idle, OptionId, OptionValue, RowHeight, Settings, TableList, Timestamps, Zone,
 };
 use crate::theme::{Look, Palette};
 use crate::typography::{Laid, Text, TextRole};
@@ -493,7 +493,7 @@ fn control_height(option: OptionId, look: &Look) -> f32 {
         | OptionId::AgentsNotify
         | OptionId::Zebra
         | OptionId::ColumnTypes => SWITCH,
-        OptionId::LogDays => look.control_height,
+        OptionId::LogDays | OptionId::IdleDisconnect => look.control_height,
     }
 }
 
@@ -616,6 +616,28 @@ fn control(
                 actions.push(Action::SetOption(flipped));
             }
         }
+        OptionId::IdleDisconnect => {
+            let shown = super::idle_text(skin.locale, settings.idle_disconnect);
+            let dropdown = menu::Dropdown::new("idle", &shown, MENU);
+            let menu = dropdown.show(ui, look, palette, |ui| {
+                for choice in Idle::ALL {
+                    let text = super::idle_text(skin.locale, choice);
+                    let entry_name = format!("{name} {text}");
+                    let entry = menu::Item::choice(&text, choice == settings.idle_disconnect)
+                        .name(&entry_name)
+                        .show(ui, look, palette);
+                    if entry.clicked() {
+                        actions.push(Action::SetOption(OptionValue::IdleDisconnect(choice)));
+                        ui.close();
+                    }
+                }
+            });
+            menu.response.widget_info(|| {
+                let mut info = WidgetInfo::labeled(WidgetType::ComboBox, true, &name);
+                info.current_text_value = Some(shown.clone());
+                info
+            });
+        }
         OptionId::LogDays => {
             let shown = super::days(skin.locale, settings.mcp_log_days);
             let dropdown = menu::Dropdown::new("log-days", &shown, MENU);
@@ -773,6 +795,7 @@ fn hint(ui: &Ui, at: egui::Pos2, right: f32, option: OptionId, settings: &Settin
         // What the Appearance pane's rows say is their small print.
         OptionId::ProductionReadOnly
         | OptionId::ConfirmCloseEdits
+        | OptionId::IdleDisconnect
         | OptionId::AgentsOn
         | OptionId::LogDays
         | OptionId::AgentsNotify
diff --git a/src/ui/settings/terminal.rs b/src/ui/settings/terminal.rs
index 5f4875c..cf02f24 100644
--- a/src/ui/settings/terminal.rs
+++ b/src/ui/settings/terminal.rs
@@ -713,7 +713,7 @@ fn value_width(ui: &Ui, option: OptionId, settings: &Settings, skin: &Skin) -> f
     let width = |text: &str| role.width(ui.ctx(), skin.look.faces, text);
     match option {
         // Between its chevrons, as the cursor's row has it.
-        OptionId::PageSize | OptionId::LogDays | OptionId::Tz => {
+        OptionId::PageSize | OptionId::LogDays | OptionId::Tz | OptionId::IdleDisconnect => {
             width(&format!("‹ {} ›", stepped_text(option, settings, skin)))
         }
         OptionId::Timestamps
@@ -743,6 +743,8 @@ fn stepped_text(option: OptionId, settings: &Settings, skin: &Skin) -> String {
         OptionId::LogDays => skin
             .look
             .label(&super::days(skin.locale, settings.mcp_log_days)),
+        // As the file writes it: `30m`.
+        OptionId::IdleDisconnect => settings.idle_disconnect.name().to_owned(),
         // As the artboard writes it: `system (america/new_york)`.
         OptionId::Tz => skin.look.label(&match (&settings.tz, skin.system) {
             (crate::settings::Zone::System, Some(name)) => {
@@ -862,7 +864,7 @@ fn option_row(
     if let Some(button) = button
         && matches!(
             option,
-            OptionId::PageSize | OptionId::LogDays | OptionId::Tz
+            OptionId::PageSize | OptionId::LogDays | OptionId::Tz | OptionId::IdleDisconnect
         )
     {
         let name = gettext(locale, terminal_label(option));
@@ -884,7 +886,7 @@ fn option_row(
         on_cursor,
     };
     match option {
-        OptionId::PageSize | OptionId::LogDays | OptionId::Tz => {
+        OptionId::PageSize | OptionId::LogDays | OptionId::Tz | OptionId::IdleDisconnect => {
             stepper(ui, cell, option, settings, skin, actions)
         }
         OptionId::Timestamps
@@ -1046,6 +1048,7 @@ fn hint(ui: &Ui, option: OptionId, settings: &Settings, skin: &Skin) -> Text {
         OptionId::ProductionReadOnly => dim(&skin.say("default for new connections")),
         // As drawn: nothing to add.
         OptionId::ConfirmCloseEdits => Text::new(look),
+        OptionId::IdleDisconnect => dim(&skin.say("tabs reconnect on use")),
         OptionId::AgentsOn => dim(&skin.say("while tabletist is open")),
         // The key that shows the log, from this row, where there is a
         // window to show it in.
````

- [ ] **Step 4: Check**

Run: `cargo fmt --all && cargo clippy --locked --workspace --all-targets -- -D warnings && cargo test --locked --workspace --all-targets`
Expected: no warning, every test passes.

- [ ] **Step 5: Commit**

```bash
git add docs/_reference/settings-and-files.md src/app.rs src/model.rs src/settings.rs src/ui/mod.rs src/ui/settings/mod.rs src/ui/settings/sheet.rs src/ui/settings/terminal.rs
git commit -m "Add Disconnect when idle to Settings › General"
```

## Pull request

Tasks 1 and 2 are merged (#160). One pull request for the commits of tasks 3 to 5, on `main`. Its description has:

- a summary;
- the three rows and what each does, the backend's queue, and the idle close;
- the departures above, and the spec's;
- what was not run: CI, macOS and Windows, and the app by hand;
- that the website's `macos-settings` and `omarchy-settings` pictures show General without its Safety group until they are rendered again from `src/shots.rs`.

No screenshot unless blurred.
