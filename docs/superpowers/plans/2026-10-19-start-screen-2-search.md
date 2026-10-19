# The start screen, run 2: the search. Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A connection's window with no open tab gains a search under the connection line. On macOS a 44 pt field, with the placeholder "Find a table, view, function or saved query" and quick open's key written at its end (`⌘P from anywhere`); a letter typed anywhere on the screen goes into it (a chord held with Cmd, Ctrl or Option still runs its shortcut). What is typed replaces the recents with the tables and views that match, the same index and ranking as quick open (`App::quick_open_matches`), grouped Tables then Views, the matched letters marked and the first picked; `↑↓` move, `↩` opens, `⌥↩` opens a new SQL editor reading the match's first rows, and `esc` clears the search. A footer under the matches lists those keys, and the status bar's right side says "Start typing to search". In the Omarchy look a `/` line holds the search: `/` focuses it while the start screen has the keys, the matches replace the start lists under a `matches` heading, ctrl+n/p move, enter opens, esc returns; the status line gains `/ search`.

**Architecture:** `Workspace::start_search` holds what is typed (the field edits it), `start_selected` which match is picked, and two one-shot flags the field reads, `focus_start_search` and `blur_start_search`. `App::start_matches` is `quick_open_matches` grouped with the tables first and the views after, each in the index's order; the reducers `StartSearchFocus`, `StartSearchChanged`, `StartSearchMove`, `StartSearchOpen` and `StartSearchClear` move among the matches, open one (as a tab, or in a new SQL editor through `App::open_sql_for`) and clear the search. The keymap gets a `When::StartSearch` and five commands: four (move, open, open-in-editor, clear) read while the search is on, and `StartSearchFocus` (the Omarchy `/`) read to open it. `src/ui/keys.rs` reads them where the window shows the start screen, asks for the field when a letter is typed on macOS, and routes the Omarchy `/`. `src/ui/start.rs` draws the field (`search_field`), the grouped matches (`results`, `result_row`, with `highlight`), the macOS footer and the status texts.

**Tech Stack:** Rust 2024, egui, serde and serde_json (all dependencies already).

---

## Where this sits

Run 2 of four of `2026-10-19-start-screen.md`, the feature's master plan, written from the brief "build the start screen (no tab open)" and the four boards at the end of the "Empty, loading and error states" section of the Tabletist canvas (the `… Start screen, states` boards' Typing state, read on 2026-10-19). This plan is a branch of `main` at `69bbf50`, the merge of the run 1 work (`2026-10-19-start-screen-1-recents-and-screen.md`, pull request igor-alexandrov/tabletist#178). The sizes (run 3) and what is waiting with the reopening of closed tabs (run 4) are later runs, each shippable on its own; the master plan says what each holds.

## Where this departs from the boards or the brief

Each row needs Igor's yes before the plan is run.

| What the board or the brief says | What this plan builds | Why |
|---|---|---|
| Brief and board: results group Tables, Views, Functions and Saved queries | Tables and Views only | The `⌘P` index knows tables, views and materialized views (`ObjectKind`); the app has no functions in its index and no saved queries (the master plan's standing departure). Materialized views go with the views |
| Brief and board: the macOS field is focused when the screen appears (its accent border shows) | The field is focused when you start typing: a letter typed anywhere on the screen lands in it, and its border lights then. It is not focused on a bare appear | Run 1 settled that after a connect the keyboard stays in the sidebar, and the sidebar's arrows and `End` scroll the tree with no tab open; auto-focusing the field would take the keyboard from it. One line (requesting the field on connect and on the last close) would give Igor the board's behaviour |
| Board: each match shows its rows and size ("13 rows", "≈ 48.2k · 12 MB") | A match shows its name, its kind's group (macOS) or kind column (Omarchy), and its schema (macOS). No count or size | The counts and sizes are run 3's work (the sizes); run 2 is the search. Nothing is queried for the list |
| Brief: `⌥↩` opens `SELECT * FROM <table> LIMIT <n>` | `SELECT * FROM <schema>.<name> LIMIT <n>`, the names quoted as the dialect needs, `<n>` the editor limit of the settings | The index spans every schema the sidebar has loaded, so the schema is written to name the object without doubt. Nothing is run; the editor is the user's to change |
| Board: the matched letters are marked | Marked where the query is a contiguous, case-ignoring run of the name and lowercasing leaves the bytes in place (ASCII names); a scattered subsequence or a `schema.name` query marks nothing | The index ranks scattered and qualified matches too, but marking those needs the match's own spans, which `quick_open_matches` does not return. The common case (a prefix or a word) is marked |
| Omarchy board: the `/` line says "ctrl+p from anywhere" at its right | The `/` line shows only the placeholder | The Omarchy look has no quick open (run 1's settled keymap: quick open has no key there), so there is no key to name |
| Omarchy board: one `matches` list with a kind column; macOS board: Tables and Views sections | As each board: Omarchy heads one `matches` list, the kind of each in a column; macOS splits into Tables and Views | Each board's own layout |
| Brief: "esc clears and returns to the recents" | macOS: esc clears the text and the recents return, the field keeping the keyboard to type again. Omarchy: esc clears and gives the keys back to the screen | The macOS field is where typing goes, so it keeps the keyboard; the Omarchy `/` is opened on purpose, so esc closes it |
| Brief: the sizes, the waiting notes, `⇧⌘T` and `u` | Runs 3 and 4 | Each is a step of its own (the master plan) |

## What this run settles

| Question | Decision | Why |
|---|---|---|
| Where the search state lives | On the window: `Workspace::start_search` (what is typed; the field edits it), `start_selected` (an index into `App::start_matches`), `focus_start_search` and `blur_start_search` (one-shot, taken by the field to ask for or give up the keyboard). Kept with the window, cleared when a match opens | Per window, beside `tree.filter`; a view mutates only the text a field edits and pushes actions for the rest |
| The index | `App::start_matches`: `quick_open_matches` (the `⌘P` index and ranking, named by the brief), partitioned into the tables and then the views, each group in the index's order; empty where nothing is typed. The picked match is an index into this | The brief: "the index and ranking are ⌘P's; build no second search" |
| Opening a match | `StartSearchOpen { index, in_editor }`: the picked match, or `index` where a row was clicked; `open_object` (pinned) as a tab, or `App::open_sql_for` in a new SQL editor. Clears the search either way | The board's `↩` and `⌥↩`, and a click |
| The `SELECT` | `SELECT * FROM <schema>.<name> LIMIT <n>`, the names through `Dialect::ident`, `<n>` = `settings.sql_limit`; nothing is run | A statement the user reads and runs when ready, naming the object without doubt |
| The keys | `When::StartSearch`, and five commands (Navigation): `StartSearchFocus` (Omarchy `/`; the macOS field has the keyboard already), `StartSearchMove` (`↑↓`, ctrl+n/p), `StartSearchOpen` (`↩`, enter), `StartSearchOpenInEditor` (`⌥↩`; Omarchy has none), `StartSearchClear` (esc). The move, open and clear are read in a field as out of one (`.any()`) while the search is on | A key is spelled in the keymap alone; these answer while the field has the keyboard |
| Where the keys are read | `src/ui/keys.rs` reads them while the window shows the start screen (whatever pane the keyboard is in: the macOS field keeps the keyboard while the sidebar's pane stays the tree) and the search is on (the field has the keyboard, or something is typed). On macOS a letter typed with no text field holding the keyboard asks for the field and is left for it to read (a Cmd, Ctrl or Alt chord brings no letter); the Omarchy `/` is a letter of the Start scope, read where the dashboard letters are | The brief: typing goes into the field; the keys work while the search has focus |
| What a change picks | `StartSearchChanged`, pushed when the field's text changes, picks the first match again; `StartSearchMove` clamps to the matches; `start_selected` is clamped to the count when drawn | The first result is picked, and the pick never points past the matches |
| macOS drawing | A 44 pt capsule (`search_field`) with a magnifier, the placeholder and quick open's key at the right (`⌘P from anywhere`); the matches under "Tables" and "Views" headings (`results`), the picked on the selection's ground with its matched run in the accent (`result_row`, `highlight`), a footer of the keys (`search_footer`); the status bar's right says "Start typing to search" while nothing is typed | The board |
| Omarchy drawing | A `/`-led line (`search_field`); the matches under one `matches` heading, the kind of each in a column, the picked on the accent ground; the status line gains `/ search`, and while searching lists ctrl+n/p, enter and esc | The board |
| Accessibility | The field is labelled "Find a table, view, function or saved query" (its placeholder), whether empty or not; each match is a selectable row named "name, kind, schema", the picked one marked selected; the group headings are labels | The brief; the headless tests read them |
| The website | `_guide/macos.md` (the field and its keys), `_guide/omarchy.md` (the `/` search and its keys), `_reference/settings-and-files.md` is unchanged (no new file); `_reference/keyboard-shortcuts.md` gains the macOS search keys | AGENTS.md |
| Tests | `src/app.rs` (the grouped index, opening a match and in an editor, the move clamp and the resets); `src/ui/start_tests.rs` (the brief's 5 and 6, and the Omarchy `/` flow); the keymap's generated tests cover the new scope and keys | AGENTS.md |

## Before you start

- One commit for each task, in the order below. Every commit passes `cargo fmt --all --check`, `cargo clippy --locked --workspace --all-targets -- -D warnings`, `cargo test --locked --workspace --all-targets` and `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps`.
- On Igor's machine `cargo` is `~/.cargo/bin/cargo` (the mise shim fails), and a build in a worktree needs `CARGO_TARGET_DIR` set to that worktree's `target/`. The workspace tests need the test PostgreSQL and MySQL servers; the URLs are in `AGENTS.md` (`TABLETIST_TEST_PG_URL`, `TABLETIST_TEST_MYSQL_URL`).
- Read `AGENTS.md`: views push actions and mutate nothing but the text a field edits; keys are spelled in `src/keymap.rs` alone; state is on the window; no em dashes.
- Each step's diff applies with `git apply` from the repository's root: save the block to a file and run `git apply <file>`. The code here is formatted.
- The tests use only the Bookshop data. No test compares a picture or a position with the design.

## What was run before this plan was written

The draft was written on the run 1 branch (`a2dd20b`) and moved onto `main` at `69bbf50` once #178 was merged: a three-way merge of each file, with nothing to resolve (main's `9e22886` gives a closed SQL editor its run mode, beside `open_sql_for`, and `b4fdc37` tidies the screen's list heading). Every task's state was then built on `main` at `69bbf50` and passed `cargo fmt --all --check`, `cargo clippy --locked --workspace --all-targets -- -D warnings`, `cargo test --locked --workspace --all-targets` with the test PostgreSQL and MySQL servers (the counts are in each task's Step 4) and `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps`. Each task's tests were laid over the code before it and run alone: task 1's fail to build (the field, the method and the actions do not exist yet), task 2's three new tests fail as its Step 2 says. The two `… Start screen, states` boards were read again before the draft, and the screen was rendered in both looks, light and dark, through a throwaway scene of `src/shots.rs` (not committed): the macOS field, the grouped matches with the marked run and the footer, and the Omarchy `/` line with its `matches` list, each matched the board; the one departure the render showed is the macOS field's border, which lights when you type rather than on appear (the departures table). Every diff in this plan was applied in order to a `git archive` copy of `main` at `69bbf50`, and the result is the last state (zero mismatches). The render and the two reviews were of the draft on the run 1 branch; the checks ran again after the move to `main`. Not run: CI, macOS and Windows, and the app by hand.

## Files

| File | What changes |
|---|---|
| `src/model.rs` | `Workspace::start_search`, `start_selected`, `focus_start_search`, `blur_start_search`; `Action::StartSearchFocus`, `StartSearchChanged`, `StartSearchMove`, `StartSearchOpen`, `StartSearchClear` |
| `src/keymap.rs` | `When::StartSearch`; `Command::StartSearchFocus`, `StartSearchMove`, `StartSearchOpen`, `StartSearchOpenInEditor`, `StartSearchClear`, their info and bindings |
| `src/app.rs` | `App::start_matches`, `open_sql_for`; the five reducers; the tests |
| `src/ui/keys.rs` | `on_start_screen`, `searching`; the macOS type-anywhere focus; the keys that move, open and clear; the Omarchy `/` arm |
| `src/ui/start.rs` | `search_field_id`; the `Keys` search fields; `search_field`, `results`, `group_header`, `result_row`, `highlight`, `search_footer`; the screen branching on the search; the status bar and status line |
| `src/ui/start_tests.rs` | The brief's tests 5 and 6, and the Omarchy `/` flow (the run 1 macOS-letters test is rewritten) |
| `docs/_guide/macos.md`, `docs/_guide/omarchy.md`, `docs/_reference/keyboard-shortcuts.md` | The website |

---

### Task 1: The search state, its keys and its index

The window's search state, the grouped index and the reducers, and the keymap's commands. The start screen still shows no field after this task: the drawing and the reading of the keys are task 2's. Nothing changes on screen; the app's tests exercise the new logic.

**Files:** Modify `src/model.rs`, `src/keymap.rs`, `src/app.rs`.

- [ ] **Step 1: Write the failing tests**

The app's tests of the grouped index, opening a match (as a tab and in an editor), the move clamp, and the resets.

````diff
diff --git a/src/app.rs b/src/app.rs
--- a/src/app.rs
+++ b/src/app.rs
@@ -22608,4 +22608,188 @@
             assert_eq!(object(&harness, tab, id).focus_field, None);
         }
     }
+
+    /// A Bookshop development window, listed, for the start screen's search.
+    fn start_search_window() -> (Harness, ConnTabId) {
+        let mut harness = Harness::new();
+        let tab = harness.connect_with(
+            crate::testing::fixture_connection(),
+            crate::testing::bookshop_objects(),
+        );
+        (harness, tab)
+    }
+
+    #[test]
+    fn start_matches_are_the_quick_open_index_grouped_tables_then_views() {
+        let (mut harness, tab) = start_search_window();
+        harness.app.workspace_mut(tab).unwrap().start_search = "cov".into();
+        let matched: Vec<(String, ObjectKind)> = harness
+            .app
+            .start_matches(tab)
+            .into_iter()
+            .map(|(object, kind)| (object.name, kind))
+            .collect();
+        // The `⌘P` index ranks `covers` and `cover_counts` first; grouped,
+        // the tables come before the views, each in that order.
+        assert_eq!(
+            matched,
+            vec![
+                ("covers".into(), ObjectKind::Table),
+                ("book_covers".into(), ObjectKind::Table),
+                ("cover_counts".into(), ObjectKind::View),
+                ("missing_covers".into(), ObjectKind::View),
+            ]
+        );
+        // Nothing typed lists nothing.
+        harness.app.workspace_mut(tab).unwrap().start_search = "   ".into();
+        assert!(harness.app.start_matches(tab).is_empty());
+    }
+
+    #[test]
+    fn start_search_open_opens_the_picked_match_and_clears_the_search() {
+        let (mut harness, tab) = start_search_window();
+        let workspace = harness.app.workspace_mut(tab).unwrap();
+        workspace.start_search = "cov".into();
+        workspace.start_selected = 1;
+        harness.app.apply(Action::StartSearchOpen {
+            tab,
+            index: None,
+            in_editor: false,
+        });
+        let workspace = harness.app.workspace(tab).unwrap();
+        assert_eq!(
+            workspace.active_object_tab().unwrap().object.name,
+            "book_covers"
+        );
+        assert_eq!(workspace.start_search, "");
+        assert_eq!(workspace.start_selected, 0);
+        // A clicked row names its own index, whatever is picked.
+        harness.app.apply(Action::CloseTab {
+            tab,
+            id: harness.app.workspace(tab).unwrap().active_tab.unwrap(),
+        });
+        harness.app.workspace_mut(tab).unwrap().start_search = "cov".into();
+        harness.app.apply(Action::StartSearchOpen {
+            tab,
+            index: Some(2),
+            in_editor: false,
+        });
+        assert_eq!(
+            harness
+                .app
+                .workspace(tab)
+                .unwrap()
+                .active_object_tab()
+                .unwrap()
+                .object
+                .name,
+            "cover_counts"
+        );
+    }
+
+    #[test]
+    fn start_search_open_in_editor_writes_a_select_of_the_match() {
+        let (mut harness, tab) = start_search_window();
+        let workspace = harness.app.workspace_mut(tab).unwrap();
+        workspace.driver = Driver::Postgres;
+        workspace.start_search = "cov".into();
+        harness.app.apply(Action::StartSearchOpen {
+            tab,
+            index: None,
+            in_editor: true,
+        });
+        let workspace = harness.app.workspace(tab).unwrap();
+        assert_eq!(
+            workspace.active_sql_tab().unwrap().text,
+            format!(
+                "SELECT * FROM main.covers LIMIT {}",
+                harness.app.settings.sql_limit
+            )
+        );
+        assert_eq!(workspace.start_search, "");
+    }
+
+    #[test]
+    fn start_search_move_clamps_and_changed_and_clear_reset() {
+        let (mut harness, tab) = start_search_window();
+        harness.app.workspace_mut(tab).unwrap().start_search = "cov".into();
+        // Up at the first match stays there; down past the last stops there.
+        harness.app.apply(Action::StartSearchMove { tab, by: -1 });
+        assert_eq!(harness.app.workspace(tab).unwrap().start_selected, 0);
+        for _ in 0..9 {
+            harness.app.apply(Action::StartSearchMove { tab, by: 1 });
+        }
+        assert_eq!(harness.app.workspace(tab).unwrap().start_selected, 3);
+        // What is typed changing picks the first again.
+        harness.app.apply(Action::StartSearchChanged(tab));
+        assert_eq!(harness.app.workspace(tab).unwrap().start_selected, 0);
+        // Clear empties the search and the pick.
+        harness.app.workspace_mut(tab).unwrap().start_selected = 2;
+        harness.app.apply(Action::StartSearchClear(tab));
+        let workspace = harness.app.workspace(tab).unwrap();
+        assert_eq!(workspace.start_search, "");
+        assert_eq!(workspace.start_selected, 0);
+    }
+
+    #[test]
+    fn start_search_clear_keeps_the_macos_field_and_blurs_the_omarchy_one() {
+        let (mut harness, tab) = start_search_window();
+        harness.app.workspace_mut(tab).unwrap().start_search = "cov".into();
+        // macOS (the harness's default look): esc would drop a field's
+        // keyboard in egui, so the clear asks for it again.
+        harness.app.apply(Action::StartSearchClear(tab));
+        let workspace = harness.app.workspace(tab).unwrap();
+        assert!(workspace.focus_start_search);
+        assert!(!workspace.blur_start_search);
+        // Omarchy: esc gives the keys back to the screen.
+        harness.set_look(crate::theme::Look::omarchy());
+        harness.app.workspace_mut(tab).unwrap().start_search = "cov".into();
+        harness.app.apply(Action::StartSearchClear(tab));
+        let workspace = harness.app.workspace(tab).unwrap();
+        assert!(workspace.blur_start_search);
+        assert!(!workspace.focus_start_search);
+    }
+
+    #[test]
+    fn start_search_open_clamps_a_pick_left_past_the_matches() {
+        let (mut harness, tab) = start_search_window();
+        let workspace = harness.app.workspace_mut(tab).unwrap();
+        workspace.start_search = "cov".into();
+        workspace.start_selected = 3;
+        // The matches shrink under the same query, with no arrow or edit to
+        // reset the pick: only `covers` is left, so `start_selected` is
+        // stale at 3.
+        harness
+            .app
+            .workspace_mut(tab)
+            .unwrap()
+            .tree
+            .nodes
+            .get_mut("main")
+            .unwrap()
+            .objects
+            .value = Some(vec![tabletist_db::ObjectInfo {
+            name: "covers".into(),
+            kind: ObjectKind::Table,
+            estimated_rows: None,
+        }]);
+        assert_eq!(harness.app.start_matches(tab).len(), 1);
+        harness.app.apply(Action::StartSearchOpen {
+            tab,
+            index: None,
+            in_editor: false,
+        });
+        // It opens the match that is left, not nothing.
+        assert_eq!(
+            harness
+                .app
+                .workspace(tab)
+                .unwrap()
+                .active_object_tab()
+                .unwrap()
+                .object
+                .name,
+            "covers"
+        );
+    }
 }
````

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --locked -p tabletist --lib -- app::tests::start_`

Expected: the build fails, as the field, the method and the actions do not exist yet:

```
error[E0599]: no method named `start_matches` found for struct `app::App` in the current scope
error[E0599]: no variant named `StartSearchMove` found for enum `model::Action`
error[E0599]: no variant named `StartSearchOpen` found for enum `model::Action`
error[E0599]: no variant, associated function, or constant named `StartSearchChanged` found for enum `model::Action` in the current scope
error[E0599]: no variant, associated function, or constant named `StartSearchClear` found for enum `model::Action` in the current scope
error[E0609]: no field `blur_start_search` on type `&Workspace`
error[E0609]: no field `focus_start_search` on type `&Workspace`
error[E0609]: no field `start_search` on type `&Workspace`
error: could not compile `tabletist` (lib test) due to 37 previous errors
```

- [ ] **Step 3: Write the code**

The window's state, the five actions, the keymap's commands with `When::StartSearch`, `App::start_matches` and `open_sql_for`, and the reducers.

````diff
diff --git a/src/model.rs b/src/model.rs
--- a/src/model.rs
+++ b/src/model.rs
@@ -142,6 +142,27 @@
     /// Empty the connection's recents, and the window's SQL editors among
     /// them. Nothing is lost, so nothing asks.
     ClearRecents(ConnTabId),
+    /// Give the start screen's search field the keyboard: `/` in the
+    /// Omarchy look, where the screen has the keys.
+    StartSearchFocus(ConnTabId),
+    /// What the search field holds changed: pick the first match again.
+    StartSearchChanged(ConnTabId),
+    /// Move the start screen's search by `by` matches, clamped.
+    StartSearchMove {
+        tab: ConnTabId,
+        by: isize,
+    },
+    /// Open a start screen search match: the one picked, or `index` where a
+    /// row was clicked; in a new SQL editor (`SELECT * FROM … LIMIT n`)
+    /// where `in_editor`, else as a table or view tab. Clears the search.
+    StartSearchOpen {
+        tab: ConnTabId,
+        index: Option<usize>,
+        in_editor: bool,
+    },
+    /// Empty the start screen's search and show the recents again. The
+    /// Omarchy field gives the keys back to the screen.
+    StartSearchClear(ConnTabId),
     /// Hide or show the sidebar.
     ToggleSidebar(ConnTabId),
     /// Show timestamps in the grid to the microsecond, or to the second.
@@ -978,6 +999,22 @@
     /// them with the tables and views of `recents.json`, which are the
     /// connection's, and opens one again with its script.
     pub recent_sql: Vec<ClosedSql>,
+    /// What is typed in the start screen's search, while no tab is open:
+    /// the field edits it. Empty shows the recents; anything else shows
+    /// the tables and views that match (the `⌘P` index), grouped.
+    pub start_search: String,
+    /// Which match the start screen's search has picked, an index into
+    /// [`crate::app::App::start_matches`]. Clamped as it moves; reset to
+    /// the first when what is typed changes.
+    pub start_selected: usize,
+    /// Give the start screen's search field the keyboard on the next
+    /// frame: when `/` asks for it in the Omarchy look. On macOS a letter
+    /// typed on the screen asks for the field directly, in `src/ui/keys.rs`.
+    pub focus_start_search: bool,
+    /// Take the keyboard off the start screen's search field on the next
+    /// frame, so the screen's own keys answer again: when `esc` clears
+    /// the search in the Omarchy look.
+    pub blur_start_search: bool,
     pub sidebar_hidden: bool,
     /// The grid shows timestamps in full rather than to the second.
     pub full_precision: bool,
@@ -4565,6 +4602,10 @@
             needs_ssh_prompt: None,
             pane: Pane::Tree,
             recent_sql: Vec::new(),
+            start_search: String::new(),
+            start_selected: 0,
+            focus_start_search: false,
+            blur_start_search: false,
             sidebar_hidden: false,
             full_precision: false,
             zone: None,
diff --git a/src/keymap.rs b/src/keymap.rs
--- a/src/keymap.rs
+++ b/src/keymap.rs
@@ -168,6 +168,10 @@
     /// The Agents tab shows its Log view: its keys come before the
     /// activity's.
     AgentsLog,
+    /// The start screen's search is on: its field has the keyboard, or
+    /// something is typed in it. The keys that move among the matches,
+    /// open one and clear the search are read then.
+    StartSearch,
 }
 
 /// The mode a chord holds in: with the keyboard in a text field (insert),
@@ -363,6 +367,11 @@
     MoveInAgentLevels,
     ShowAgentsFromBox,
     OpenRecent,
+    StartSearchFocus,
+    StartSearchMove,
+    StartSearchOpen,
+    StartSearchOpenInEditor,
+    StartSearchClear,
     ToggleSidebar,
     PaneLeft,
     PaneDown,
@@ -629,6 +638,15 @@
             C::MoveInAgentLevels => agents(info("Move among what agents may do", "", Prompts)),
             C::ShowAgentsFromBox => agents(info("Show the connection's Agents tab", "", Prompts)),
             C::OpenRecent => info("Open a recent object", "", Navigation),
+            C::StartSearchFocus => info("Search the start screen", "", Navigation),
+            C::StartSearchMove => info("Move in the start screen's search", "", Navigation),
+            C::StartSearchOpen => info("Open the start screen's search match", "", Navigation),
+            C::StartSearchOpenInEditor => info(
+                "Open the start screen's search match in a SQL editor",
+                "",
+                Navigation,
+            ),
+            C::StartSearchClear => info("Clear the start screen's search", "", Navigation),
             C::ToggleAgentsLogScope => agents(info(
                 "Show every connection's calls in the log, or this one's",
                 "",
@@ -1130,6 +1148,40 @@
         bind(C::SwitchDatabase, START, NONE, keys(&[k("d")])),
         bind(C::EditConnection, START, NONE, keys(&[k("e")])),
         bind(C::ShowAgents, START, NONE, keys(&[k("a")])),
+        // The start screen's search. On macOS the field has the keyboard
+        // as soon as the screen appears, so it has no key of its own;
+        // Omarchy's `/` gives it the keys. The keys that move among the
+        // matches, open one and clear the search hold while the search is
+        // on, in a field as out of one.
+        bind(C::StartSearchFocus, START, NONE, keys(&[k("/")])),
+        bind(
+            C::StartSearchMove,
+            START,
+            keys(&[k("up"), k("down")]).shown("↑↓").any(),
+            keys(&[k("ctrl+p"), k("ctrl+n")]).shown("ctrl+n/p").any(),
+        )
+        .when(When::StartSearch),
+        bind(
+            C::StartSearchOpen,
+            START,
+            keys(&[k("enter")]).any(),
+            keys(&[k("enter")]).any(),
+        )
+        .when(When::StartSearch),
+        bind(
+            C::StartSearchOpenInEditor,
+            START,
+            keys(&[k("alt+enter")]).any(),
+            NONE,
+        )
+        .when(When::StartSearch),
+        bind(
+            C::StartSearchClear,
+            START,
+            keys(&[k("esc")]).any(),
+            keys(&[k("esc")]).any(),
+        )
+        .when(When::StartSearch),
         bind(
             C::CloseTab,
             GLOBAL,
diff --git a/src/app.rs b/src/app.rs
--- a/src/app.rs
+++ b/src/app.rs
@@ -1401,6 +1401,72 @@
                     self.save_recents();
                 }
             }
+            Action::StartSearchFocus(tab) => {
+                if let Some(workspace) = self.workspace_mut(tab) {
+                    workspace.focus_start_search = true;
+                }
+            }
+            Action::StartSearchChanged(tab) => {
+                if let Some(workspace) = self.workspace_mut(tab) {
+                    workspace.start_selected = 0;
+                }
+            }
+            Action::StartSearchMove { tab, by } => {
+                let count = self.start_matches(tab).len();
+                if let Some(workspace) = self.workspace_mut(tab) {
+                    workspace.start_selected = (workspace.start_selected as isize + by)
+                        .clamp(0, count.saturating_sub(1) as isize)
+                        as usize;
+                }
+            }
+            Action::StartSearchOpen {
+                tab,
+                index,
+                in_editor,
+            } => {
+                let matches = self.start_matches(tab);
+                let selected = self
+                    .workspace(tab)
+                    .map_or(0, |workspace| workspace.start_selected);
+                // A clicked row names its own index; the picked one is
+                // clamped, as the matches may have shrunk since it moved
+                // (a relist, a database switched under a typed query).
+                let picked = index.unwrap_or_else(|| selected.min(matches.len().saturating_sub(1)));
+                let Some((object, kind)) = matches.get(picked).cloned() else {
+                    return;
+                };
+                if let Some(workspace) = self.workspace_mut(tab) {
+                    workspace.start_search.clear();
+                    workspace.start_selected = 0;
+                }
+                if in_editor {
+                    self.open_sql_for(tab, &object);
+                } else {
+                    self.apply(Action::OpenObject {
+                        tab,
+                        object,
+                        kind,
+                        pin: true,
+                    });
+                }
+            }
+            Action::StartSearchClear(tab) => {
+                let terminal = self.look.terminal;
+                if let Some(workspace) = self.workspace_mut(tab) {
+                    workspace.start_search.clear();
+                    workspace.start_selected = 0;
+                    if terminal {
+                        // The Omarchy field gives the keys back to the
+                        // screen, so the screen's own keys answer again.
+                        workspace.focus_start_search = false;
+                        workspace.blur_start_search = true;
+                    } else {
+                        // egui drops a field's keyboard on Esc; ask for it
+                        // again so the macOS field keeps it, to type afresh.
+                        workspace.focus_start_search = true;
+                    }
+                }
+            }
             Action::SelectCell { tab, id, cell } => {
                 // The editor's text is kept, and the note of a locked cell
                 // was about the cell the selection leaves.
@@ -4325,6 +4391,27 @@
             .shown_schema(workspace.driver, self.settings.show_system_schemas)
     }
 
+    /// What the start screen's search lists for `tab`, in the order it
+    /// draws them: the `⌘P` index's matches (`quick_open_matches`, the
+    /// same index and ranking), grouped with the tables first and the
+    /// views after, each group in the index's order. Empty where nothing
+    /// is typed. Materialized views go with the views. The picked match
+    /// ([`Workspace::start_selected`]) is an index into this.
+    pub fn start_matches(&self, tab: ConnTabId) -> Vec<(ObjectRef, ObjectKind)> {
+        let query = self
+            .workspace(tab)
+            .map(|workspace| workspace.start_search.trim())
+            .unwrap_or_default();
+        if query.is_empty() {
+            return Vec::new();
+        }
+        let matched = self.quick_open_matches(tab, query);
+        let (tables, views): (Vec<_>, Vec<_>) = matched
+            .into_iter()
+            .partition(|(_, kind)| *kind == ObjectKind::Table);
+        tables.into_iter().chain(views).collect()
+    }
+
     /// Opens the start screen's SQL recent at `closed`: an editor with
     /// the script the closed one had, which then is no recent any more.
     fn reopen_sql(&mut self, tab: ConnTabId, closed: usize) {
@@ -4347,6 +4434,29 @@
                 sql.opened = kept.opened;
                 sql.number = kept.number;
                 sql.mode = kept.mode;
+            }
+        }
+    }
+
+    /// Opens a new SQL editor on `tab` that reads `object`'s first rows:
+    /// `SELECT * FROM <object> LIMIT <n>`, with the names quoted as the
+    /// dialect needs and `n` the editor limit of the settings. Nothing is
+    /// run; the editor is the user's to change and run. The start screen's
+    /// `⌥↩` opens one.
+    fn open_sql_for(&mut self, tab: ConnTabId, object: &ObjectRef) {
+        let Some(id) = self.new_sql_tab(tab) else {
+            return;
+        };
+        let limit = self.settings.sql_limit;
+        if let Some(workspace) = self.workspace_mut(tab) {
+            let dialect = workspace.driver.dialect();
+            let name = format!(
+                "{}.{}",
+                dialect.ident(&object.schema),
+                dialect.ident(&object.name)
+            );
+            if let Some(sql) = workspace.sql_tab_mut(id) {
+                sql.text = format!("SELECT * FROM {name} LIMIT {limit}");
             }
         }
     }
````

- [ ] **Step 4: Run the checks**

Run: `cargo fmt --all --check && cargo clippy --locked --workspace --all-targets -- -D warnings && cargo test --locked --workspace --all-targets && RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps`

Expected: all pass, among them every `app::tests::start_*` and the keymap's generated tests. The lib suite is 2897 tests:

```
fmt exit 0
clippy exit 0
test result: ok. 2897 passed; 0 failed; 20 ignored; 0 measured; 0 filtered out
doc exit 0
```

- [ ] **Step 5: Commit**

```bash
git add src/model.rs src/keymap.rs src/app.rs
git commit -S -m "Keep the start screen's search on the window, with its index and keys"
```

---

### Task 2: The search on the start screen, in both looks

`src/ui/start.rs` draws the field, the grouped matches with the marked run, the macOS footer and the status texts; `src/ui/keys.rs` reads the keys and routes typing into the field; the brief's tests and the website.

**Files:** Modify `src/ui/keys.rs`, `src/ui/start.rs`, `src/ui/start_tests.rs`, `docs/_guide/macos.md`, `docs/_guide/omarchy.md`, `docs/_reference/keyboard-shortcuts.md`.

- [ ] **Step 1: Write the failing tests**

The brief's test 5 (a letter typed on macOS lands in the search, and a Cmd shortcut still runs), test 6 (the matches are the `⌘P` index's, grouped by kind; `↩` opens the first, `⌥↩` opens it in a SQL editor, `esc` returns to the recents), and the Omarchy `/` flow. The run 1 test that typed letters on the macOS screen is rewritten as test 5.

````diff
diff --git a/src/ui/start_tests.rs b/src/ui/start_tests.rs
--- a/src/ui/start_tests.rs
+++ b/src/ui/start_tests.rs
@@ -523,17 +523,117 @@
     assert!(harness.has("reviews tab"));
 }
 
-/// The other looks leave the screen's letters and numbers alone: typed,
-/// they open nothing.
-#[test]
-fn the_mac_look_has_no_start_letters() {
+/// Test 5: on macOS the search field has the keyboard when the screen
+/// appears, so a letter typed anywhere lands in it, the matches taking the
+/// recents' place; a chord held with Cmd still runs its shortcut.
+#[test]
+fn a_letter_typed_on_macos_lands_in_the_search_and_cmd_shortcuts_run() {
     let (mut harness, tab) = bookshop(Look::macos());
+    harness.settle();
+    assert!(harness.has("Find a table, view, function or saved query"));
+    // A letter goes into the search: the recents give way to the matches.
+    for (key, text) in [(Key::C, "c"), (Key::O, "o"), (Key::V, "v")] {
+        type_key(&mut harness, key, text);
+    }
+    assert_eq!(harness.app.workspace(tab).unwrap().start_search, "cov");
+    assert!(harness.has("Tables"));
+    assert!(!harness.has("Recent on this connection"));
+    assert!(harness.app.workspace(tab).unwrap().tabs.is_empty());
+    // A Cmd shortcut still runs: ⌘T opens a SQL editor, past the field.
+    harness.press(Key::T, Modifiers::COMMAND);
+    assert!(harness.has("Query 1 tab"));
+}
+
+/// Test 6: the matches are the `⌘P` index's, grouped by kind; `↩` opens the
+/// first, `⌥↩` opens it in a SQL editor with its `SELECT`, and `esc` clears
+/// the search and brings the recents back.
+#[test]
+fn the_search_groups_the_index_opens_the_first_and_in_an_editor_and_clears() {
+    let (mut harness, tab) = bookshop(Look::macos());
+    harness.app.workspace_mut(tab).unwrap().driver = Driver::Postgres;
+    let search = |harness: &mut Harness| {
+        for (key, text) in [(Key::C, "c"), (Key::O, "o"), (Key::V, "v")] {
+            type_key(harness, key, text);
+        }
+    };
+    search(&mut harness);
+    // Grouped Tables then Views; the index's order inside each group.
+    assert!(harness.has("Tables") && harness.has("Views"));
+    assert_eq!(
+        harness.app.start_matches(tab).first().unwrap().0.name,
+        "covers"
+    );
+    // ↩ opens the first match.
+    harness.press(Key::Enter, Modifiers::NONE);
+    assert!(harness.has("covers tab"));
+    assert_eq!(harness.app.workspace(tab).unwrap().start_search, "");
+    let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
+    harness.app.apply(Action::CloseTab { tab, id });
+    // ⌥↩ opens the first match in a SQL editor that reads its first rows.
+    search(&mut harness);
+    harness.press(Key::Enter, Modifiers::ALT);
+    let sql = harness
+        .app
+        .workspace(tab)
+        .unwrap()
+        .active_sql_tab()
+        .expect("a SQL editor")
+        .text
+        .clone();
+    assert_eq!(
+        sql,
+        format!(
+            "SELECT * FROM main.covers LIMIT {}",
+            harness.app.settings.sql_limit
+        )
+    );
+    let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
+    harness.app.apply(Action::CloseTab { tab, id });
+    // esc clears the search: the recents are back.
+    search(&mut harness);
+    assert!(!harness.has("Recent on this connection"));
+    harness.press(Key::Escape, Modifiers::NONE);
+    assert_eq!(harness.app.workspace(tab).unwrap().start_search, "");
+    assert!(harness.has("Recent on this connection"));
+    // The field keeps the keyboard, to search again (egui drops it on esc,
+    // so the clear asks for it back). Typing lands in the search at once.
+    type_key(&mut harness, Key::A, "a");
+    assert_eq!(harness.app.workspace(tab).unwrap().start_search, "a");
+}
+
+/// The Omarchy `/` search: `/` focuses the field while the screen has the
+/// keys, ctrl+n/p move among the matches, enter opens, and esc returns to
+/// the start lists.
+#[test]
+fn the_omarchy_slash_search_moves_opens_and_returns() {
+    let (mut harness, tab) = bookshop(Look::omarchy());
+    let pane = |harness: &Harness| harness.app.workspace(tab).unwrap().pane;
+    harness.press(Key::L, Modifiers::CTRL);
+    assert_eq!(pane(&harness), Pane::Grid);
+    // `/` focuses the field; then what is typed lands in it.
+    type_key(&mut harness, Key::Slash, "/");
+    for (key, text) in [(Key::C, "c"), (Key::O, "o"), (Key::V, "v")] {
+        type_key(&mut harness, key, text);
+    }
+    assert_eq!(harness.app.workspace(tab).unwrap().start_search, "cov");
+    assert!(harness.has("matches"));
+    // The first match is picked; ctrl+n moves to the second, enter opens it.
+    assert_eq!(harness.app.start_matches(tab)[0].0.name, "covers");
+    assert_eq!(harness.app.start_matches(tab)[1].0.name, "book_covers");
+    harness.press(Key::N, Modifiers::CTRL);
+    harness.press(Key::Enter, Modifiers::NONE);
+    assert!(harness.has("book_covers tab"));
+    let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
+    harness.app.apply(Action::CloseTab { tab, id });
+    // Searching again, esc returns to the start lists and the screen's keys.
+    type_key(&mut harness, Key::Slash, "/");
+    type_key(&mut harness, Key::C, "c");
+    assert!(!harness.has("recent"));
+    harness.press(Key::Escape, Modifiers::NONE);
+    assert_eq!(harness.app.workspace(tab).unwrap().start_search, "");
+    // The screen's own keys answer again: a number opens a recent.
     type_key(&mut harness, Key::Num1, "1");
-    type_key(&mut harness, Key::T, "t");
-    assert!(harness.app.workspace(tab).unwrap().tabs.is_empty());
-    // Its buttons do the same work.
-    harness.click("New SQL query");
-    assert!(harness.has("Query 1 tab"));
+    assert!(harness.has("book_covers tab"));
 }
 
 /// The name a screen reader would read for the focused node.
````

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --locked -p tabletist --lib -- ui::start_tests::`

Expected: the three new tests fail, as the field and the reading of its keys do not exist yet; the others pass:

```
test ui::start_tests::a_letter_typed_on_macos_lands_in_the_search_and_cmd_shortcuts_run ... FAILED
test ui::start_tests::the_search_groups_the_index_opens_the_first_and_in_an_editor_and_clears ... FAILED
test ui::start_tests::the_omarchy_slash_search_moves_opens_and_returns ... FAILED
test result: FAILED. 15 passed; 3 failed
```

- [ ] **Step 3: Write the code**

The field, the grouped matches, the footer and the status texts; the keys that move, open and clear, the macOS type-anywhere focus and the Omarchy `/`; the website.

````diff
diff --git a/src/ui/keys.rs b/src/ui/keys.rs
--- a/src/ui/keys.rs
+++ b/src/ui/keys.rs
@@ -545,6 +545,35 @@
         && app
             .workspace(active)
             .is_some_and(crate::model::Workspace::on_start);
+    // The window shows the start screen (whatever pane the keyboard is in:
+    // the macOS field keeps the keyboard while the sidebar's pane stays
+    // the tree). Its search is on while the field has the keyboard or
+    // something is typed; the keys that move among the matches, open one
+    // and clear the search are read then.
+    let on_start_screen = !on_picker
+        && app
+            .workspace(active)
+            .is_some_and(crate::model::Workspace::on_start);
+    let searching = on_start_screen
+        && (ctx.memory(|memory| memory.has_focus(crate::ui::start::search_field_id(active)))
+            || app
+                .workspace(active)
+                .is_some_and(|workspace| !workspace.start_search.trim().is_empty()));
+    // On macOS the screen's search field takes what is typed, wherever the
+    // keyboard is: a character typed with no text field holding the
+    // keyboard asks for the search field, and is left in the frame for it
+    // to read. So the keyboard can stay in the sidebar until a letter is
+    // typed, and the sidebar's own filter keeps what is typed into it.
+    // Keys held with Cmd, Ctrl or Alt are shortcuts, and bring no
+    // character.
+    if on_start_screen
+        && !app.look.terminal
+        && !ctx.text_edit_focused()
+        && !typed_chars(ctx).is_empty()
+    {
+        let field = crate::ui::start::search_field_id(active);
+        ctx.memory_mut(|memory| memory.request_focus(field));
+    }
     let scopes: &[Scope] = if on_picker {
         &[Scope::Connections]
     } else if open {
@@ -766,6 +795,69 @@
         {
             actions.push(Action::OpenRecent { tab: active, index });
         }
+        // The start screen's search, while it is on: the keys that move
+        // among the matches, open one (in a SQL editor with Alt) and clear
+        // it. Read in a field as out of one, so they answer while the field
+        // has the keyboard.
+        if searching {
+            // The keymap lists the key that moves up first, then the one
+            // that moves down, in each look: the first steps back among the
+            // matches, the rest forward.
+            for (nth, keys) in keymap
+                .chords_now(layout, Command::StartSearchMove, Scope::Start, normal)
+                .enumerate()
+            {
+                let by = if nth == 0 { -1 } else { 1 };
+                let count = presses(input, layout, keys).count;
+                actions.extend((0..count).map(|_| Action::StartSearchMove { tab: active, by }));
+            }
+            if asked(
+                input,
+                keymap,
+                layout,
+                Command::StartSearchOpenInEditor,
+                Scope::Start,
+                normal,
+            )
+            .count
+                > 0
+            {
+                actions.push(Action::StartSearchOpen {
+                    tab: active,
+                    index: None,
+                    in_editor: true,
+                });
+            } else if asked(
+                input,
+                keymap,
+                layout,
+                Command::StartSearchOpen,
+                Scope::Start,
+                normal,
+            )
+            .count
+                > 0
+            {
+                actions.push(Action::StartSearchOpen {
+                    tab: active,
+                    index: None,
+                    in_editor: false,
+                });
+            }
+            if asked(
+                input,
+                keymap,
+                layout,
+                Command::StartSearchClear,
+                Scope::Start,
+                normal,
+            )
+            .count
+                > 0
+            {
+                actions.push(Action::StartSearchClear(active));
+            }
+        }
         // Where the server has another database: with one, no menu is
         // drawn, and an id left open would show it once the list came.
         if in_workspace
@@ -2473,6 +2565,9 @@
                 let id = crate::ui::workspace::database_menu_id(tab, true);
                 egui::Popup::open_id(ctx, id);
             }
+            (Some(Command::StartSearchFocus), _) if start => {
+                actions.push(Action::StartSearchFocus(tab));
+            }
             (Some(Command::EditCell), _) if edits && alone => {
                 opened = true;
                 actions.extend(edit(crate::model::EditStart::Value));
diff --git a/src/ui/start.rs b/src/ui/start.rs
--- a/src/ui/start.rs
+++ b/src/ui/start.rs
@@ -5,7 +5,7 @@
 //! look a dashboard whose keys the status line lists: while the screen has
 //! the keyboard, a number opens a row and a letter starts something.
 
-use egui::{CornerRadius, Frame, Margin, Rect, Sense, Stroke, WidgetInfo, WidgetType};
+use egui::{CornerRadius, Frame, Margin, Rect, Sense, Stroke, StrokeKind, WidgetInfo, WidgetType};
 use egui::{pos2, vec2};
 use tabletist_db::{Driver, ObjectKind, ObjectRef};
 
@@ -129,6 +129,22 @@
     recent: Written,
     sidebar: Written,
     help: Written,
+    /// Quick open's key, written at the macOS field's end: search goes on
+    /// from anywhere with it. Empty in the Omarchy look, which has none.
+    quick_open: Written,
+    /// The search: the keys that move among the matches (`↑↓`, `ctrl+n/p`),
+    /// open one, open one in a SQL editor, clear the search, and (Omarchy)
+    /// focus the field.
+    search_focus: Written,
+    search_move: Written,
+    search_open: Written,
+    search_editor: Written,
+    search_clear: Written,
+}
+
+/// Where the start screen's search field keeps the keyboard, by window.
+pub(crate) fn search_field_id(tab: ConnTabId) -> egui::Id {
+    egui::Id::new(("start-search", tab.0))
 }
 
 fn keys(app: &App) -> Keys {
@@ -148,6 +164,12 @@
         recent: here(Command::OpenRecent),
         sidebar: key(Command::ToggleSidebar),
         help: key(Command::Help),
+        quick_open: key(Command::FindObject),
+        search_focus: here(Command::StartSearchFocus),
+        search_move: app.keymap.label(layout, Command::StartSearchMove),
+        search_open: here(Command::StartSearchOpen),
+        search_editor: here(Command::StartSearchOpenInEditor),
+        search_clear: here(Command::StartSearchClear),
     }
 }
 
@@ -176,6 +198,22 @@
         return;
     };
     let keys = keys(app);
+    // The matches of what is typed, grouped (tables, then views); empty
+    // where nothing is typed. The window left the field two one-shot
+    // flags, and holds what is typed and which match is picked.
+    let matches = app.start_matches(tab);
+    let (focus, blur, mut query, selected) = app
+        .workspace_mut(tab)
+        .map(|workspace| {
+            let focus = std::mem::take(&mut workspace.focus_start_search);
+            let blur = std::mem::take(&mut workspace.blur_start_search);
+            let selected = workspace
+                .start_selected
+                .min(matches.len().saturating_sub(1));
+            (focus, blur, workspace.start_search.clone(), selected)
+        })
+        .unwrap_or_default();
+    let searching = !query.trim().is_empty();
     let skin = Skin {
         look: &look,
         palette: &palette,
@@ -186,7 +224,7 @@
     };
     let mut actions = Vec::new();
     if !look.terminal {
-        status_bar(ui, skin);
+        status_bar(ui, skin, searching);
     }
     egui::CentralPanel::default()
         .frame(Frame::new().fill(palette.surface))
@@ -223,13 +261,39 @@
             ui.spacing_mut().item_spacing.y = 0.0;
             connection_line(&mut ui, &shown, skin);
             ui.add_space(gap(&look));
+            search_field(
+                &mut ui,
+                &mut query,
+                &keys,
+                skin,
+                (focus, blur),
+                &mut actions,
+            );
+            ui.add_space(gap(&look));
             // On macOS the first row, or the first button where there is
-            // none, is where F6 brings the keyboard.
+            // none, is where F6 brings the keyboard. While searching the
+            // matches replace the recents, and the footer of keys the
+            // three ways to start.
             let mut claimed = look.terminal;
-            lists(&mut ui, &shown, &keys, skin, &mut claimed, &mut actions);
-            ui.add_space(gap(&look));
-            starts(&mut ui, &shown, &keys, skin, &mut claimed, &mut actions);
+            if searching {
+                results(&mut ui, &matches, selected, &query, skin, &mut actions);
+                if !look.terminal {
+                    ui.add_space(gap(&look));
+                    search_footer(&mut ui, &keys, skin);
+                }
+            } else {
+                lists(&mut ui, &shown, &keys, skin, &mut claimed, &mut actions);
+                ui.add_space(gap(&look));
+                starts(&mut ui, &shown, &keys, skin, &mut claimed, &mut actions);
+            }
         });
+    // What the field edited goes back; a change picks the first match anew.
+    if let Some(workspace) = app.workspace_mut(tab)
+        && workspace.start_search != query
+    {
+        workspace.start_search = query;
+        actions.push(Action::StartSearchChanged(tab));
+    }
     app.actions.extend(actions);
 }
 
@@ -855,8 +919,346 @@
     }
 }
 
+/// The search field under the connection line. macOS: a 44 pt capsule
+/// with a magnifier, the placeholder, and the `⌘P` key written at its end
+/// (quick open is where search goes on from anywhere). Omarchy: a `/`-led
+/// line, as the sidebar's filter. It edits what is typed; `focus` gives it
+/// the keyboard when `/` asks (the Omarchy look), `blur` takes it off (esc
+/// there). On macOS a letter typed asks for it in `keys.rs`. Its accessible
+/// name is the placeholder.
+fn search_field(
+    ui: &mut egui::Ui,
+    query: &mut String,
+    keys: &Keys,
+    skin: Skin<'_>,
+    (focus, blur): (bool, bool),
+    _actions: &mut [Action],
+) {
+    let (look, palette, locale) = (skin.look, skin.palette, skin.locale);
+    let id = search_field_id(skin.tab);
+    let hint = look.label(&gettext(
+        locale,
+        "Find a table, view, function or saved query",
+    ));
+    let full = ui.available_width();
+    let height = if look.terminal { 30.0 } else { 44.0 };
+    let (rect, _) = ui.allocate_exact_size(vec2(full, height), Sense::hover());
+    let corner = if look.terminal {
+        CornerRadius::same(3)
+    } else {
+        CornerRadius::same((height / 2.0) as u8)
+    };
+    let role = TextRole::pick(look, TextRole::UiBody, TextRole::OField);
+    let pad = if look.terminal { 8.0 } else { 14.0 };
+    // The lead mark: an accent `/` in the terminal look, a magnifier on
+    // macOS. The `⌘P` hint stands at the right on macOS; the Omarchy look
+    // has no quick open, so its line carries none.
+    let slash = Text::one(look, TextRole::OGroup, "/", palette.accent).layout(ui.ctx());
+    let mark = if look.terminal { slash.width() } else { 14.0 };
+    let shortcut = (!look.terminal && !keys.quick_open.as_str().is_empty()).then(|| {
+        let text = format!(
+            "{} {}",
+            keys.quick_open.as_str(),
+            gettext(locale, "from anywhere")
+        );
+        Text::one(look, widgets::secondary(look), &text, palette.dim).layout(ui.ctx())
+    });
+    let right_room = shortcut.as_ref().map_or(0.0, |laid| laid.width() + 10.0);
+    let left = rect.left() + pad + mark + 8.0;
+    let right = rect.right() - pad - right_room;
+    let line = role.row_height(ui.ctx(), look.faces);
+    let top = rect.center().y - role.middle(ui.ctx(), look.faces);
+    let inner = Rect::from_min_max(pos2(left, top), pos2(right.max(left), top + line));
+    let backdrop = ui.painter().add(egui::Shape::Noop);
+    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(inner));
+    let placeholder = Text::one(look, role, &hint, palette.dim).layout(child.ctx());
+    let response = child.add(
+        egui::TextEdit::singleline(query)
+            .id(id)
+            .font(role.font_id(look.faces))
+            .frame(Frame::NONE)
+            .margin(Margin::ZERO)
+            .desired_width(inner.width())
+            .text_color(palette.text)
+            .hint_text(placeholder.galley),
+    );
+    // The accessible name is the placeholder, so a screen reader names
+    // the field whether it is empty or holds a query.
+    response.widget_info(|| WidgetInfo::labeled(WidgetType::TextEdit, true, &hint));
+    if focus {
+        response.request_focus();
+    }
+    if blur {
+        response.surrender_focus();
+    }
+    let lit = focus || response.has_focus();
+    let fill = if look.terminal {
+        palette.window
+    } else {
+        palette.surface
+    };
+    ui.painter().set(
+        backdrop,
+        egui::epaint::RectShape::filled(rect, corner, fill),
+    );
+    let border = if lit {
+        palette.accent
+    } else if look.terminal {
+        palette.outline
+    } else {
+        palette.border
+    };
+    if lit && !look.terminal {
+        ui.painter().rect_stroke(
+            rect,
+            corner,
+            Stroke::new(3.0, focus::halo(palette)),
+            StrokeKind::Outside,
+        );
+    }
+    ui.painter()
+        .rect_stroke(rect, corner, Stroke::new(1.0, border), StrokeKind::Inside);
+    let y = rect.center().y;
+    if look.terminal {
+        slash.paint_left(ui.painter(), rect.left() + pad, y);
+    } else {
+        Icon::Search.image(palette.dim, 14.0).paint_at(
+            ui,
+            Rect::from_center_size(pos2(rect.left() + pad + 7.0, y), vec2(14.0, 14.0)),
+        );
+    }
+    if let Some(shortcut) = shortcut {
+        shortcut.paint_right(ui.painter(), rect.right() - pad, y);
+    }
+}
+
+/// The matches of what is typed, in the place of the recents: the tables
+/// first and the views after, each under its heading, the picked match on
+/// the selection's ground and the run of the name that matched in the
+/// accent. Opening a row opens its object.
+fn results(
+    ui: &mut egui::Ui,
+    matches: &[(ObjectRef, ObjectKind)],
+    selected: usize,
+    query: &str,
+    skin: Skin<'_>,
+    actions: &mut Vec<Action>,
+) {
+    let (look, palette, locale) = (skin.look, skin.palette, skin.locale);
+    if matches.is_empty() {
+        let text = gettext(locale, "No matches");
+        let laid = Text::one(look, widgets::secondary(look), &text, palette.dim);
+        let (rect, _) =
+            ui.allocate_exact_size(vec2(ui.available_width(), row_height(look)), Sense::hover());
+        let inset = if look.terminal { 2.0 } else { INSET };
+        widgets::paint_label(ui, rect.left() + inset, rect.center().y, laid);
+        widgets::announce(ui, rect, &look.label(&text));
+        return;
+    }
+    if look.terminal {
+        // One list, headed `matches`, the kind of each in a column of its
+        // own: the terminal board's dashboard, not the Mac's sections.
+        group_header(ui, &gettext(locale, "matches"), skin);
+        for (index, entry) in matches.iter().enumerate() {
+            result_row(ui, index, entry, query, index == selected, skin, actions);
+        }
+    } else {
+        // The matches under the kind they are, tables first, then views,
+        // as `start_matches` already orders them.
+        let tables = matches
+            .iter()
+            .take_while(|(_, k)| *k == ObjectKind::Table)
+            .count();
+        for (title, range) in [("Tables", 0..tables), ("Views", tables..matches.len())] {
+            if range.is_empty() {
+                continue;
+            }
+            group_header(ui, &gettext(locale, title), skin);
+            for index in range {
+                result_row(
+                    ui,
+                    index,
+                    &matches[index],
+                    query,
+                    index == selected,
+                    skin,
+                    actions,
+                );
+            }
+        }
+    }
+}
+
+/// A plain section heading over a group of matches: "Tables" or "Views",
+/// as the look writes a section label.
+fn group_header(ui: &mut egui::Ui, title: &str, skin: Skin<'_>) {
+    let (look, palette) = (skin.look, skin.palette);
+    let laid = widgets::section_label(title, look, palette).layout(ui.ctx());
+    let head_height = laid.height() + if look.terminal { 2.0 } else { 6.0 };
+    let (head, _) = ui.allocate_exact_size(vec2(ui.available_width(), head_height), Sense::hover());
+    let inset = if look.terminal { 2.0 } else { INSET };
+    let y = head.top() + laid.height() / 2.0;
+    let width = laid.paint_left(ui.painter(), head.left() + inset, y);
+    widgets::announce(
+        ui,
+        Rect::from_min_size(head.min, vec2(width, head_height)),
+        title,
+    );
+}
+
+/// One match: on macOS its kind's icon, the name with the matched run in
+/// the accent, and its schema; in the terminal look a cursor bar on the
+/// picked one, the name and the kind. A click opens it.
+fn result_row(
+    ui: &mut egui::Ui,
+    index: usize,
+    entry: &(ObjectRef, ObjectKind),
+    query: &str,
+    picked: bool,
+    skin: Skin<'_>,
+    actions: &mut Vec<Action>,
+) {
+    let (object, kind) = (&entry.0, entry.1);
+    let (look, palette) = (skin.look, skin.palette);
+    let width = ui.available_width();
+    let (rect, response) = ui.allocate_exact_size(vec2(width, row_height(look)), Sense::click());
+    let name = display_safe(&object.name).into_owned();
+    let schema = display_safe(&object.schema).into_owned();
+    let kind_word = skin.say(match kind {
+        ObjectKind::Table => "table",
+        ObjectKind::View | ObjectKind::MaterializedView => "view",
+    });
+    let label = format!("{name}, {kind_word}, {schema}");
+    response
+        .widget_info(|| WidgetInfo::selected(WidgetType::SelectableLabel, true, picked, &label));
+    widgets::selection(ui, rect, picked, response.hovered(), look, palette);
+    if !look.terminal {
+        focus::hint(
+            ui,
+            &response,
+            widgets::selection_rect(rect, look),
+            focus::Ring::Inset {
+                radius: look.radius.saturating_sub(2),
+            },
+        );
+    }
+    let [before, hit, after] = highlight(&name, query);
+    let y = rect.center().y;
+    let left = rect.left() + INSET;
+    let right = rect.right() - INSET;
+    if look.terminal {
+        let marker = if picked { "▌" } else { "" };
+        widgets::paint_text(
+            ui,
+            left,
+            y,
+            Text::one(look, TextRole::OGroup, marker, palette.accent),
+        );
+        let detail_x = right - 120.0;
+        widgets::paint_text(
+            ui,
+            detail_x,
+            y,
+            Text::one(look, TextRole::OBody, &kind_word, palette.dim),
+        );
+        let room = (detail_x - 10.0 - (left + 28.0)).max(0.0);
+        let name = Text::new(look)
+            .add(TextRole::OBody, &before, palette.text)
+            .add(TextRole::OGroup, &hit, palette.accent)
+            .add(TextRole::OBody, &after, palette.text);
+        name.wrap(room)
+            .layout(ui.ctx())
+            .paint_left(ui.painter(), left + 28.0, y);
+    } else {
+        let icon = match kind {
+            ObjectKind::Table => Icon::Table,
+            ObjectKind::View | ObjectKind::MaterializedView => Icon::Eye,
+        };
+        let ink = if picked { palette.accent } else { palette.dim };
+        icon.image(ink, 14.0).paint_at(
+            ui,
+            Rect::from_center_size(pos2(left + 7.0, y), vec2(14.0, 14.0)),
+        );
+        let x = left + 18.0 + INSET;
+        let name_ink = if picked {
+            palette.accent_hover
+        } else {
+            palette.text
+        };
+        let name = Text::new(look)
+            .add(TextRole::UiBodyStrong, &before, name_ink)
+            .add(TextRole::UiBodyStrong, &hit, palette.accent)
+            .add(TextRole::UiBodyStrong, &after, name_ink);
+        let laid = name.layout(ui.ctx());
+        let after_name = x + laid.width() + 8.0;
+        laid.paint_left(ui.painter(), x, y);
+        widgets::paint_text(
+            ui,
+            after_name,
+            y,
+            Text::one(look, widgets::secondary(look), &schema, palette.dim),
+        );
+    }
+    if response.clicked() {
+        actions.push(Action::StartSearchOpen {
+            tab: skin.tab,
+            index: Some(index),
+            in_editor: false,
+        });
+    }
+}
+
+/// Splits `name` into what stands before the run of it that matched
+/// `query`, that run, and what stands after. A contiguous, case-ignoring
+/// run only, and only where lowercasing leaves the bytes in place (ASCII
+/// names); a scattered or qualified query highlights nothing.
+fn highlight(name: &str, query: &str) -> [String; 3] {
+    let needle = query.trim();
+    let nothing = || [name.to_owned(), String::new(), String::new()];
+    if needle.is_empty() {
+        return nothing();
+    }
+    let lower = name.to_lowercase();
+    let found = lower.find(&needle.to_lowercase());
+    if lower.len() == name.len()
+        && let Some(at) = found
+    {
+        let end = at + needle.to_lowercase().len();
+        if name.is_char_boundary(at) && name.is_char_boundary(end) {
+            return [
+                name[..at].to_owned(),
+                name[at..end].to_owned(),
+                name[end..].to_owned(),
+            ];
+        }
+    }
+    nothing()
+}
+
+/// The macOS footer under the matches: the keys that move among them, open
+/// one, open one in a SQL editor, and clear the search.
+fn search_footer(ui: &mut egui::Ui, keys: &Keys, skin: Skin<'_>) {
+    let (look, palette, locale) = (skin.look, skin.palette, skin.locale);
+    let words = ["move", "open", "open in SQL editor", "clear"].map(|word| gettext(locale, word));
+    let hints: Vec<widgets::Hint<'_>> = vec![
+        (&keys.search_move, &*words[0], true),
+        (&keys.search_open, &*words[1], true),
+        (&keys.search_editor, &*words[2], true),
+        (&keys.search_clear, &*words[3], true),
+    ];
+    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 20.0), Sense::hover());
+    widgets::key_hints(
+        ui,
+        (rect.left() + INSET, rect.center().y),
+        &hints,
+        14.0,
+        look,
+        palette,
+    );
+}
+
 /// The macOS status bar under the screen: that no table is open.
-fn status_bar(ui: &mut egui::Ui, skin: Skin<'_>) {
+fn status_bar(ui: &mut egui::Ui, skin: Skin<'_>, searching: bool) {
     let (look, palette) = (skin.look, skin.palette);
     egui::Panel::bottom(egui::Id::new(("start-status", skin.tab.0)))
         .exact_size(STATUS_BAR)
@@ -874,12 +1276,23 @@
             let status = palette.secondary.lerp_to_gamma(palette.dim, 0.5);
             let text = gettext(skin.locale, "No table open");
             let rect = ui.max_rect();
+            let y = rect.center().y + 0.5;
             widgets::paint_label(
                 ui,
                 rect.left(),
-                rect.center().y + 0.5,
+                y,
                 Text::one(look, widgets::secondary(look), &text, status),
             );
+            // The right side invites a search, while nothing is typed.
+            if !searching {
+                let invite = gettext(skin.locale, "Start typing to search");
+                widgets::paint_text_right(
+                    ui,
+                    rect.right(),
+                    y,
+                    Text::one(look, widgets::secondary(look), &invite, status),
+                );
+            }
         });
 }
 
@@ -888,23 +1301,47 @@
 pub fn status_line(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
     let (look, palette, locale) = (app.look, app.palette, app.locale);
     let keys = keys(app);
+    // The search is on while its field has the keyboard or something is
+    // typed, as `keys.rs` reads it: the line then names the keys that move
+    // among the matches, open one and go back, in step with those live keys.
+    let searching = ui
+        .ctx()
+        .memory(|memory| memory.has_focus(search_field_id(tab)))
+        || app
+            .workspace(tab)
+            .is_some_and(|workspace| !workspace.start_search.trim().is_empty());
+    // Otherwise the line names the ways to start, `/` for the search among
+    // them.
     let words = [
         "open recent",
+        "search",
         "new sql",
         "database",
         "edit connection",
         "tables",
         "all keys",
+        "move",
+        "open",
+        "back",
     ]
     .map(|word| gettext(locale, word));
-    let hints: Vec<widgets::Hint<'_>> = vec![
-        (&keys.recent, &*words[0], true),
-        (&keys.sql_here, &*words[1], true),
-        (&keys.database_here, &*words[2], true),
-        (&keys.edit_here, &*words[3], true),
-        (&keys.sidebar, &*words[4], true),
-        (&keys.help, &*words[5], true),
-    ];
+    let hints: Vec<widgets::Hint<'_>> = if searching {
+        vec![
+            (&keys.search_move, &*words[7], true),
+            (&keys.search_open, &*words[8], true),
+            (&keys.search_clear, &*words[9], true),
+        ]
+    } else {
+        vec![
+            (&keys.recent, &*words[0], true),
+            (&keys.search_focus, &*words[1], true),
+            (&keys.sql_here, &*words[2], true),
+            (&keys.database_here, &*words[3], true),
+            (&keys.edit_here, &*words[4], true),
+            (&keys.sidebar, &*words[5], true),
+            (&keys.help, &*words[6], true),
+        ]
+    };
     egui::Panel::bottom(egui::Id::new(("status-line", tab.0)))
         .exact_size(STATUS_LINE)
         .resizable(false)
diff --git a/docs/_guide/macos.md b/docs/_guide/macos.md
--- a/docs/_guide/macos.md
+++ b/docs/_guide/macos.md
@@ -33,6 +33,13 @@
   SQLite file lists its tables by name). Then
   three buttons: New SQL query, Switch database and Edit connection.
   Closing the last tab brings the screen back.
+- **A search field sits under the connection line.** Start typing
+  anywhere on the screen and what you type goes into it; the recents give
+  way to the tables and views that match, grouped, with the first picked
+  and the matched letters marked. It is the same search as Quick open
+  (Cmd+P), which opens it from anywhere. Up and Down move the pick, Return
+  opens it, Option+Return opens it in a new SQL editor reading its first
+  rows, and Esc clears the search and brings the recents back.
 - **Light and dark follow the system.** Tabletist changes with the
   Appearance setting while it runs. **Settings › Appearance** can keep it
   light or dark whatever the system says.
diff --git a/docs/_guide/omarchy.md b/docs/_guide/omarchy.md
--- a/docs/_guide/omarchy.md
+++ b/docs/_guide/omarchy.md
@@ -98,17 +98,27 @@
 opened last on this connection, numbered, and under `start` the ways to
 begin. On your first visit it lists the largest tables instead, from the
 database's statistics, so nothing is queried (a SQLite file lists its
-tables by name). The status line says `-- START --` while no tab is open.
-After a connect the keyboard is in the sidebar; Ctrl+L gives it to the
-screen, whose border lights, and Ctrl+H puts it back.
+tables by name). A `/` line under the connection holds the search. The
+status line says `-- START --` while no tab is open. After a connect the
+keyboard is in the sidebar; Ctrl+L gives it to the screen, whose border
+lights, and Ctrl+H puts it back.
 
 | Keys | Do |
 | --- | --- |
 | `1` to `9` | Open that row |
+| `/` | Search the tables and views; type to filter |
 | `t` | New SQL editor |
 | `d` | Switch database, where the server has another |
 | `e` | Edit the connection |
 | `a` | Show the Agents tab |
+
+With the search open, the matches replace the start lists:
+
+| Keys | Do |
+| --- | --- |
+| `Ctrl+N`, `Ctrl+P` | Move down and up the matches |
+| `Enter` | Open the one picked |
+| `Esc` | Clear the search and go back |
 
 ### A table
 
diff --git a/docs/_reference/keyboard-shortcuts.md b/docs/_reference/keyboard-shortcuts.md
--- a/docs/_reference/keyboard-shortcuts.md
+++ b/docs/_reference/keyboard-shortcuts.md
@@ -59,6 +59,16 @@
 | Mod+, | Settings |
 | Mod+=, Mod+-, Mod+0 | Larger, smaller and default data text, under a dialog too |
 | ? | This list |
+
+With no tab open the window shows the start screen, whose search field
+takes what you type (Mod+P opens the same search from anywhere):
+
+| Keys | Do |
+| --- | --- |
+| Up, Down | Move among the matches |
+| Return | Open the one picked |
+| Alt+Return | Open it in a new SQL editor |
+| Esc | Clear the search |
 
 ## Tables
 
````

- [ ] **Step 4: Run the checks**

Run: `cargo fmt --all --check && cargo clippy --locked --workspace --all-targets -- -D warnings && cargo test --locked --workspace --all-targets && RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps`, then the website: `cd docs && bin/check`.

Expected: all pass, among them every `ui::start_tests::*`. The lib suite is 2899 tests:

```
fmt exit 0
clippy exit 0
test result: ok. 2899 passed; 0 failed; 20 ignored; 0 measured; 0 filtered out
test result: ok. 502 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out
test result: ok. 102 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
doc exit 0
HTML-Proofer finished successfully
```

- [ ] **Step 5: Commit**

```bash
git add src/ui/keys.rs src/ui/start.rs src/ui/start_tests.rs docs/_guide/macos.md docs/_guide/omarchy.md docs/_reference/keyboard-shortcuts.md
git commit -S -m "Search the tables and views from the start screen"
```

## The pull request

One pull request for the run, from the branch of these two commits (and the one with the plan). Its description: a summary; what the search does on macOS (the field focused on typing, `⌘P` from anywhere, the grouped matches, `↑↓`, `↩`, `⌥↩`, `esc`) and on Omarchy (`/`, ctrl+n/p, enter, esc); the keys added (`StartSearchFocus`, `StartSearchMove`, `StartSearchOpen`, `StartSearchOpenInEditor`, `StartSearchClear`, under `When::StartSearch`); the departures above, the macOS focus-on-type one first; no screenshot, or a blurred one.
