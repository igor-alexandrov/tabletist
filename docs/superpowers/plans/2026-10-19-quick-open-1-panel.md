# Quick open, run 1: the panel, the index and the scopes. Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Quick open becomes the brief's panel. ⌘P (macOS, Windows: Ctrl+P) and, new, `ctrl+p` in the Omarchy look open it from anywhere in a connection's window. It searches one index per connection, built once from the cached schema and ranked in the brief's tiers (exact, prefix, word starts, substring; then the recently opened, shorter names, alphabetical). The start screen's search uses the same index. macOS draws a floating panel, 640 wide and 84 from the top over an undimmed window: a scope chip (the connection with its environment's dot, the production pill, or "All open connections"), the field, then Recent and three commands when nothing is typed, or Tables, Views and Commands (three each, then "N more") when something is. Each row shows the matched letters in the strong accent, the estimate, and `↩` on the picked one; a key footer sits under the list. Omarchy draws the picker: a `>` prompt with `DEV`, `PROD` or `ALL`, the match count, one numbered list with a kind column, `1`–`9` only while nothing is typed, and ctrl+n/p. Tab searches every open connection, grouped by connection, and opening a result brings its window forward. Enter opens a result or shows its tab; ⌥↩ opens `SELECT * … LIMIT n`. The macOS and Windows sidebar field opens quick open.

**Architecture:** `src/search.rs` decides how one name matches what is typed: a `Name`, taken apart once, and its `Tier` and marked `spans`. `src/app/quick_open.rs` holds the rest of the reducer side:
- each connection's `Index`, cached on the app and rebuilt when the catalog changes;
- `App::search_objects`, the ranked matches;
- `App::quick_open_list`, the rows the panel draws (`QuickRow`, `QuickItem`);
- the reducers of the new actions.

`src/ui/quick_open.rs` draws the panel and the picker and reads their keys, which the keymap gives a few new commands. `src/ui/sidebar.rs` turns the native field into quick open's way in.

**Tech Stack:** Rust 2024, egui 0.36 (the repository's fork) with AccessKit. No new dependency.

---

## Where this sits

Run 1 of four of `2026-10-19-quick-open.md`, the feature's master plan, written from the brief "build quick open (⌘P on macOS, ctrl+p on Omarchy)" and the four quick open boards in the Components section of the Tabletist canvas (read on 2026-10-19). Igor approved the four runs and the master plan's departures on 2026-10-19. This plan is a branch of `main` at `1e61307`. Commands with `>` and the settings rows (run 2), columns (run 3) and functions (run 4) are later runs, each shippable on its own. Saved queries are no run: the app has none.

## Where this departs from the boards or the brief

Each row needs Igor's yes before the plan is run. The master plan's rows (four pull requests, no saved queries, Switch database with no macOS key, every count an estimate, the ranking's prefix tier, `1-9 pick` only with nothing typed, the Windows look's field, Reopen closed tabs and View › Quick Open later) hold here too and are not repeated.

| What the board or the brief says | What this plan builds | Why |
|---|---|---|
| Placeholder "Find a table, view, column, saved query or command" | "Find a table, view or command". The Omarchy prompt shows none, as its board; its field is named "Find a table or view" | Run 1 finds tables, views and three commands, and the start screen's review narrowed its placeholder to what it finds. Runs 3 and 4 add columns and functions to it |
| Footer: `> commands` | Left out | `>` is run 2's. A hint for a key that does nothing would mislead |
| `⇥` in the footer and in "Search all open connections ⇥" | `Tab`, as the keymap writes the key on macOS (`⇧Tab` too, settled by its tests) | Keys are spelled by the keymap alone (`tests/keys.rs`) |
| The field's text at 15 px | The design system's body, 13 pt | The tokens have no 15 pt regular style (15 is `section-title`, semibold). Adding a text role is a change to `typography.rs` and its tests for one field |
| macOS, all connections: a heading "Bookshop · dev" and a `dev` badge | The connection's name in the heading's capitals ("BOOKSHOP") and its environment's badge | The board writes the environment twice |
| Omarchy board: `1-9 pick` | `1..9 pick` | The keymap writes a range of digits that way everywhere (the start screen's dashboard too) |
| Brief: "No objects match "zzz" in Bookshop · dev" | macOS as the brief. Omarchy names the database, as its board does: "no objects match “zzz” in bookshop_development" | Each look's board |
| Brief: the picked row reads "publishers, table, public, 6 rows" | "publishers, table, public, ≈ 6 rows" (Omarchy `≈ 6`) | The words of the row: every count is an estimate (the master plan) |
| Omarchy: "at the top centre of the window" | 120 from the window's top, as its board places it | The board's place; the brief gives no number |
| Omarchy board: headings 28 tall | 24, a row's height | Eight lines of the list then hold eight rows or headings, and the count of what is hidden is exact (a review finding) |
| Brief: "Typing never queries the database" with "one index per connection" | As the brief. The index is built when a connection is first searched and again whenever its catalog changes (the sidebar's lists, Reload) | The cached schema is the sidebar's: schemas it has not listed are not searched, as today |
| Brief: test 12, "a keystroke updates the list within 16 ms" | The test times the rows of each keystroke's list (`App::quick_open_list`, the macOS look) in the debug build the test suite runs: the best of five tries of each of ten queries, under 16 ms. Measured alone: 1.4–4.3 ms (`e`, `o` and `main.e`, which match nearly every one of the 10,000, are the slowest). Under the whole workspace's tests the same search once took 16.6 ms, before the matches shared the index's names, the ranking became one number and each schema was matched once, which is why they are Drawing is not timed: only the rows in view are drawn | A release build is several times faster. The test takes the best of five so that a busy machine does not fail it |
| Brief: with nothing typed, "Recent (the same 5 as the start screen)" | The start screen's list, whatever it is: on a first visit, with nothing opened yet on the connection, that is the largest tables, headed "Largest tables". Omarchy's digits open those rows too | The same list as the start screen, as the brief asks |

## What this run settles

| Question | Decision | Why |
|---|---|---|
| How a name matches | `search::Name::find`: every typed character in order first (a cheap reject), then the whole name (`Exact`), its start (`Prefix`), runs that each start a word (`WordStarts`: words part at `_`, `.`, `-`, a space and a lower-to-upper case change; the longest run first, memoised), a run anywhere (`Substring`). Case folded per character, so a name that is not ASCII is marked on its characters. Spaces at the ends of what is typed are dropped, spaces inside it kept | The brief's tiers. Scattered letters no longer match (out of scope: "no fuzzy matching beyond the stated tiers"), which the start screen's search loses too |
| The index | `Index`: each table, view and materialized view of the schemas the sidebar has listed, its name taken apart once, ordered by name and schema. A match (`Found`) shares the index's `Rc<ObjectRef>`, so a search that matches thousands copies no name. Cached per window in `App::indexes` (a `RefCell`), keyed by the catalog's generation and the identity of each schema's list, rebuilt when that changes. A closed window's index is dropped the next time an index is built | "Build the index once". The key also notices a list set directly, as tests do |
| The ranking | Tier, then the place among the connection's recents (newest first), then the name's length in characters, then by name and schema: packed in one number per match (`Ranked`), sorted once. A dotted query's schema part is matched once per schema | The brief's order, cheap enough for 10,000 names per keystroke |
| A query with a dot | What stands before the first dot matches the schema (any tier), what stands after it the name, whose tier ranks it. `main.` lists the schema's objects; a dot alone matches nothing (a review finding: it listed everything) | `schema.name` as the sidebar writes it |
| What the panel lists | `App::quick_open_list(&QuickOpen) -> QuickList`: rows of headings and results (`QuickItem::Object`, `Start`, `Command`, `More`), and how many objects matched. Nothing typed: each searched window's start screen list (recents, or on a first visit the largest tables), then the commands. Typed: Tables, Views (this connection) or one group per connection (every open one), then the commands whose names match. macOS: three per group and a `More` row until it is opened; Omarchy: every match, no kind headings, no commands | The brief's contents; one function the view and the reducers share, so a pick is an index into what is drawn |
| The commands | New SQL query (with "opens read-only" on production where new editors open read-only), Switch database (where the server has another), Agents (where agents can reach the app, with who is connected). Hidden where they cannot act. Matched by name with the same tiers | The board's three. Run 2 brings the rest |
| The state | `QuickOpen { tab, query, selected, everywhere, expanded, caret, scrolled_to, scroll_offset }`. `selected` indexes the results; `everywhere` resets on each open; `caret` is a one-shot (`All` for ⌘P again, `End` after text typed on the sidebar's field) | On the dialog, as before |
| The actions | `OpenQuickOpen` (selects the text where quick open is open: one panel), `QuickOpenTyped(String)` (task 3's field pushes it), `QuickOpenChanged`, `QuickOpenMove(isize)` (from the pick as it shows), `QuickOpenPick { index, in_editor }` (a `More` row opens its group and stays), `QuickOpenScope`, `ShowDatabaseMenu(ConnTabId)` (the bar's chip opens its menu on the next frame) | The view changes only its field's text, the one-shot caret it takes, and the list's scroll state; everything else is an action |
| Opening a result | An object: its window comes to the front (`ActivateConnTab`), then `OpenObject` (pinned: a new tab, or the tab it has), or `open_sql_for` with ⌥↩. A start screen row: `OpenRecent`, or `open_sql_for` with ⌥↩ for an object. A command: `NewSqlTab`, `ShowDatabaseMenu`, `ShowAgents` in the window quick open was opened in. Opening an object with ↩ notes it among the recents (`open_object` already does); ⌥↩ opens a SQL editor, which notes nothing, as the start screen's ⌥↩ | The brief |
| The keys | `FindObject` gains Omarchy `ctrl+p` (global). The Omarchy `/` of the sidebar becomes `FilterObjects`. New, in `When::QuickOpen` (Prompt scope): `QuickOpenPickInEditor` (⌥↩; Omarchy none), `QuickOpenScope` (Tab), `QuickOpenNumber` (Omarchy `1..9`). `QuickOpenMove` gains Omarchy ctrl+p and ctrl+n, the up keys listed first | The brief's keys, checked in the master plan; the open lists keep their ctrl+p (their keys are read before the global one) |
| Where the panel reads its keys | `src/ui/quick_open.rs`, before its field is drawn: Esc, the moves, ⌥↩, Enter, Tab, the digits while nothing but spaces is typed (their text taken with them), and ⌘P again. The field keeps the keyboard and holds Tab, the arrows and Esc (`EventFilter`) | The window's handler does not run under a dialog |
| macOS drawing | An `egui::Modal` on a transparent backdrop, anchored top centre at 84, `modal_frame` with no margin and the window's fill: the 52 pt field line (magnifier, chip, field; no ring round the field, its caret says where the keyboard is), the list (36 pt rows on the selection's ground when picked, the icon and the name in the strong accent, the marked letters semibold in it, `kind · schema`, the estimate or the key or when it was opened, the open key on the picked row), the footer on the sunken surface | The boards |
| Omarchy drawing | The same modal, at 120: a 1 pt accent border on the sunken surface (`panel`), the 32 pt prompt line (`>`, the scope's label, the field with a block cursor, the count), 24 pt rows (number, name with the marked letters bold in the accent, kind column, right column; the picked on the accent's soft ground with a 3 pt accent bar), the key line | The boards |
| Only the rows in view are drawn | `ScrollArea::show_viewport`: every row's place is worked out, and only those in view (and the picked one) are laid out and get AccessKit nodes | Omarchy lists every match, thousands where one letter is typed |
| Accessibility | The panel's node is a `Dialog` named "Quick open" (Omarchy "Find object"); the field an `EditableComboBox` that controls the `ListBox`, its active descendant the picked option; each result a `ListBoxOption` named "name, kind, schema" and then its estimate or when it was opened (a SQL recent: "Query N, SQL editor, …"), selected when picked; headings are labels | The brief |
| Closing | Esc, a click on the backdrop, or a result opened. `focus::dialog` (already) gives the keyboard back to what had it. Quick open closes with the window it was opened in | The brief, and a review finding |
| The sidebar field (macOS, Windows) | A button drawn as the field ("Find any object…", the magnifier, quick open's key), in the Search region for F6 and Tab. A click, Enter or Space opens quick open; text typed on it opens quick open with the text (`QuickOpenTyped`), and the start screen's type-anywhere leaves it alone. The terminal look keeps its `/` filter | Open question 2's default |
| The start screen | `App::start_matches` is `search_objects`' best 50, grouped; its rows mark the found spans (`start::marked`, which quick open shares). Its Omarchy `/` line now names `ctrl+p from anywhere`, as its board does, since the look has the key now | One index |
| The website | `_guide/macos.md`, `_guide/omarchy.md`, `_guide/browsing-data.md`, `_guide/getting-started.md`, `_reference/keyboard-shortcuts.md` | AGENTS.md |

## Before you start

- One commit for each task, in the order below, after one for this plan and the master plan. Every commit passes `cargo fmt --all --check`, `cargo clippy --locked --workspace --all-targets -- -D warnings`, `cargo test --locked --workspace --all-targets` and `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps`.
- On Igor's machine `cargo` is `~/.cargo/bin/cargo` (the mise shim fails), and a build in a worktree needs `CARGO_TARGET_DIR` set to that worktree's `target/`. The workspace tests need the test PostgreSQL and MySQL servers; the URLs are in `AGENTS.md` (`TABLETIST_TEST_PG_URL`, `TABLETIST_TEST_MYSQL_URL`).
- Read `AGENTS.md`: views push actions and mutate nothing but the text a field edits; keys are spelled in `src/keymap.rs` alone; state is on the window or the dialog; no em dashes.
- Each step's diff applies with `git apply` from the repository's root: save the block to a file and run `git apply <file>`. The code here is formatted.
- The tests use only the Bookshop data. No test compares a picture or a position with the design.

## What was run before this plan was written

The four quick open boards were read again before the draft. The draft was written whole on `main` at `3e716bf`, then cut into the three tasks below. Once the plan was approved, `main` had moved to `1e61307` (among its eleven commits, egui on the fork's 0.36.2 branch, and the start screen's fills), under eight of the draft's files. Each draft file was merged three ways onto `main`'s, with `3e716bf` as the base and nothing to resolve. Every check ran again on the result, and the tasks were cut again from it; what follows is of that run. Each task's state passed `cargo fmt --all --check`, `cargo clippy --locked --workspace --all-targets -- -D warnings`, `cargo test --locked --workspace --all-targets` (with the test PostgreSQL and MySQL servers; the counts are in each task's Step 4) and `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps`. Each task's tests were laid over the code before it and run alone, and they failed as its Step 2 says: tasks 1 and 2 fail to build, and task 3's four tests fail.

The panel was rendered in every look, light and dark, through a throwaway scene of `src/shots.rs` (not committed). The states were nothing typed, a query, all connections, no match and production, and each matched its board apart from the departures above. The render found one defect: an accent ring round the field, which the boards do not draw (the field now says `Ring::Own`).

A reviewer then tried to break the draft with 24 throwaway headless tests of its own, in a copy of the tree. It found ten things, all fixed here:
- a leftover `#[cfg]` that switched off an unrelated test of the agent server;
- the 16 ms promise held only for the queries the test chose, with the whole tree rescanned and re-sorted each frame (now the cached `Index`, and the test times ten queries, the worst cases among them);
- the digits typed after spaces alone;
- spaces in names;
- a lone dot listing everything;
- an unsafe name raw in a tooltip;
- a stale pick that made the first ↑ look dead;
- typing on the sidebar field doing nothing;
- Omarchy's hidden count ignoring its headings;
- quick open outliving its window.

A second reviewer read this plan against its diffs and the brief. It found a promised test missing (the completion list keeping ctrl+p, now checked in `src/ui/complete_tests.rs`), test 12's row worded wider than what it times, recents capped at five never reached in test 2 (it now opens six), and points of wording, all fixed here.

Every diff in this plan was applied in order to a `git archive` copy of `main` at `1e61307`, and the result is the last state, with zero mismatches. Not run: CI, macOS and Windows, and the app by hand.

## Files

| File | What changes |
|---|---|
| `src/search.rs` (new) | `Tier`, `Hit`, `Name` (`new`, `find`, `len`), `folded`, `find`; the word-start runs; its tests |
| `src/lib.rs` | `pub mod search` |
| `src/app/quick_open.rs` (new) | `Index`, `Indexes`, `Found`, `StartRow`, `QuickCommand`, `QuickItem`, `QuickHeading`, `QuickRow`, `QuickList`; `App::search_objects`, `quick_open_windows`, `quick_open_list`, the reducers |
| `src/app.rs` | The module and its re-exports, `App::indexes`; the reducers wired; `start_matches` on `search_objects`; the old `Found`, `quick_open_found` and `quick_open_matches` gone; tests moved to the new index |
| `src/model.rs` | `QuickOpen`'s new fields, `QuickCaret`, `QuickGroup`; the actions; `Workspace::database_menu_asked` |
| `src/keymap.rs` | Omarchy `ctrl+p`, `FilterObjects`, `QuickOpenPickInEditor`, `QuickOpenScope`, `QuickOpenNumber`, the moves |
| `src/ui/quick_open.rs` | Rewritten: the panel and the picker |
| `src/ui/quick_open_tests.rs` (new) | The brief's tests 1–5, 7, 8, 10–12 and the review's |
| `src/ui/start.rs` | `marked`, `estimate_words`; the matches' spans; the `ctrl+p from anywhere` hint in the terminal look |
| `src/ui/workspace.rs` | The chip's menu opens on `database_menu_asked` |
| `src/ui/keys.rs` | `/` is `FilterObjects`; the start screen leaves the sidebar field's typing alone (task 3) |
| `src/ui/cell_editor.rs` | `block_cursor` is the crate's |
| `src/ui/sidebar.rs` | `find_field`, `find_field_id` (task 3) |
| `src/env.rs` | Quick open is one of the environment's surfaces (its test) |
| `src/ui/mod.rs`, `src/ui/agents_tests.rs`, `src/ui/start_tests.rs`, `src/ui/complete_tests.rs` | The old quick open tests replaced; tests that named the old field or index; the completion list keeps ctrl+p |
| `docs/_guide/*.md`, `docs/_reference/keyboard-shortcuts.md` | The website (task 3) |

---

### Task 1: How a name matches

`src/search.rs`: the tiers, a name taken apart once, and the marked runs. Nothing calls it yet: task 2 does.

**Files:** Create `src/search.rs`; modify `src/lib.rs`.

- [ ] **Step 1: Write the failing tests**

The module with its tests alone.

````diff
diff --git a/src/lib.rs b/src/lib.rs
index 2f8d129..1b71b18 100644
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -24,6 +24,7 @@ pub mod paths;
 pub mod recents;
 pub mod relay;
 pub mod review;
+pub mod search;
 pub mod secrets;
 pub mod session;
 pub mod settings;
diff --git a/src/search.rs b/src/search.rs
new file mode 100644
index 0000000..a269e64
--- /dev/null
+++ b/src/search.rs
@@ -0,0 +1,98 @@
+//! How a name matches what is typed in quick open and in the start
+//! screen's search: the one search of a connection's objects. Case is
+//! ignored, and so is white space at the ends of what is typed. A name
+//! matches in the best of four tiers: the whole name, its start, the
+//! starts of its words (`bc` is `book_covers`), or a run anywhere in it.
+//! Nothing else matches: no letters scattered through the name.
+
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+
+    fn hit(query: &str, name: &str) -> Option<Hit> {
+        find(&folded(query), name)
+    }
+
+    fn tier(query: &str, name: &str) -> Option<Tier> {
+        hit(query, name).map(|hit| hit.tier)
+    }
+
+    /// The runs a match marks, as the name's text.
+    fn marked(query: &str, name: &str) -> Vec<String> {
+        hit(query, name)
+            .expect("a match")
+            .spans
+            .into_iter()
+            .map(|span| name[span].to_owned())
+            .collect()
+    }
+
+    #[test]
+    fn the_tiers_go_exact_prefix_word_starts_substring() {
+        assert_eq!(tier("publishers", "publishers"), Some(Tier::Exact));
+        assert_eq!(tier("pub", "publishers"), Some(Tier::Prefix));
+        assert_eq!(tier("bc", "book_covers"), Some(Tier::WordStarts));
+        assert_eq!(tier("cov", "book_covers"), Some(Tier::WordStarts));
+        assert_eq!(tier("over", "book_covers"), Some(Tier::Substring));
+        assert_eq!(tier("zzz", "book_covers"), None);
+        assert!(Tier::Exact > Tier::Prefix);
+        assert!(Tier::Prefix > Tier::WordStarts);
+        assert!(Tier::WordStarts > Tier::Substring);
+    }
+
+    #[test]
+    fn case_and_the_spaces_round_what_is_typed_are_ignored() {
+        assert_eq!(tier("PUB", "publishers"), Some(Tier::Prefix));
+        assert_eq!(tier("pub", "Publishers"), Some(Tier::Prefix));
+        assert_eq!(tier(" bookcovers ", "BookCovers"), Some(Tier::Exact));
+        assert_eq!(hit("   ", "publishers"), None);
+        // A space inside what is typed is a character of the name's.
+        assert_eq!(tier("order items", "Order Items"), Some(Tier::Exact));
+        assert_eq!(tier("r i", "Order Items"), Some(Tier::Substring));
+    }
+
+    #[test]
+    fn words_part_at_underscores_dots_and_case_changes() {
+        assert_eq!(tier("bc", "bookCovers"), Some(Tier::WordStarts));
+        assert_eq!(tier("pb", "public.books"), Some(Tier::WordStarts));
+        assert_eq!(marked("bc", "BookCovers"), ["B", "C"]);
+        // Upper case all through is one word.
+        assert_eq!(tier("bc", "BOOKCOVERS"), None);
+        assert_eq!(tier("kc", "BOOKCOVERS"), Some(Tier::Substring));
+    }
+
+    #[test]
+    fn word_starts_take_the_longest_runs_and_may_skip_words() {
+        assert_eq!(marked("bocov", "book_covers"), ["bo", "cov"]);
+        assert_eq!(marked("bc", "book_extra_covers"), ["b", "c"]);
+        // A run may go on into the next word.
+        assert_eq!(tier("book_c", "old_book_covers"), Some(Tier::WordStarts));
+        assert_eq!(marked("book_c", "old_book_covers"), ["book_c"]);
+        // A run starts a word: `k` starts none.
+        assert_eq!(tier("kc", "book_covers"), None);
+    }
+
+    #[test]
+    fn the_runs_marked_are_what_matched() {
+        assert_eq!(marked("pub", "publishers"), ["pub"]);
+        assert_eq!(marked("bc", "book_covers"), ["b", "c"]);
+        assert_eq!(marked("over", "book_covers"), ["over"]);
+        assert_eq!(marked("cov", "book_covers"), ["cov"]);
+    }
+
+    #[test]
+    fn names_that_are_not_ascii_are_marked_on_their_characters() {
+        assert_eq!(marked("ÉD", "éditions"), ["éd"]);
+        assert_eq!(marked("ü", "bücher"), ["ü"]);
+        assert_eq!(tier("ß", "straße"), Some(Tier::Substring));
+    }
+
+    #[test]
+    fn many_like_words_do_not_take_long() {
+        let name = vec!["a"; 40].join("_");
+        let started = std::time::Instant::now();
+        assert_eq!(hit(&format!("{}b", "a".repeat(30)), &name), None);
+        assert!(started.elapsed() < std::time::Duration::from_millis(50));
+    }
+}
````

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --locked -p tabletist --lib -- search::`

Expected: the build fails, as nothing they call exists yet:

```
error[E0433]: cannot find type `Tier` in this scope
error[E0425]: cannot find type `Hit` in this scope
error[E0425]: cannot find type `Tier` in this scope
error[E0425]: cannot find function `folded` in this scope
error[E0425]: cannot find function `find` in this scope
```

- [ ] **Step 3: Write the matcher**

````diff
diff --git a/src/search.rs b/src/search.rs
index a269e64..0f61a9a 100644
--- a/src/search.rs
+++ b/src/search.rs
@@ -5,6 +5,222 @@
 //! starts of its words (`bc` is `book_covers`), or a run anywhere in it.
 //! Nothing else matches: no letters scattered through the name.
 
+use std::ops::Range;
+
+/// How well a name matches, best last: the order a list ranks by.
+#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
+pub enum Tier {
+    /// What is typed is somewhere in the name.
+    Substring,
+    /// What is typed is the starts of the name's words, in order: `bc`
+    /// for `book_covers`, `cov` for `book_covers` too, `bocov` for both
+    /// words' first letters and more.
+    WordStarts,
+    /// The name starts with what is typed.
+    Prefix,
+    /// The name is what is typed.
+    Exact,
+}
+
+/// A name's match: its tier, and the runs of the name that matched, as
+/// byte ranges of the name, in order and apart.
+#[derive(Clone, Debug, PartialEq, Eq)]
+pub struct Hit {
+    pub tier: Tier,
+    pub spans: Vec<Range<usize>>,
+}
+
+/// One character of a name or of what is typed, as compared: lower case
+/// (the first character of its lower case where that is more than one).
+fn fold(char: char) -> char {
+    char.to_lowercase().next().unwrap_or(char)
+}
+
+/// Whether `char` parts a name's words.
+fn parts(char: char) -> bool {
+    matches!(char, '_' | '.' | '-' | ' ')
+}
+
+/// What `query` is compared as: its characters folded, without the white
+/// space at its ends. Empty where nothing but white space is typed.
+pub fn folded(query: &str) -> Vec<char> {
+    query.trim().chars().map(fold).collect()
+}
+
+/// A name taken apart for matching, once: its characters folded, where
+/// each starts in the name, and where its words start. An index keeps one
+/// per object, so a keystroke compares and takes nothing apart.
+#[derive(Clone, Debug)]
+pub struct Name {
+    /// The characters as compared.
+    text: Vec<char>,
+    /// Where each character starts in the name, in bytes, and then the
+    /// name's length.
+    places: Vec<usize>,
+    /// Where each word starts, as character indexes: at the name's start,
+    /// after `_`, `.`, `-` or a space, and where a lower case letter is
+    /// followed by an upper case one (`bookCovers`).
+    starts: Vec<usize>,
+}
+
+impl Name {
+    pub fn new(name: &str) -> Self {
+        let mut text = Vec::with_capacity(name.len());
+        let mut places = Vec::with_capacity(name.len() + 1);
+        let mut starts = Vec::new();
+        let mut before: Option<char> = None;
+        for (index, (at, char)) in name.char_indices().enumerate() {
+            let start = match before {
+                None => !parts(char),
+                Some(before) => {
+                    !parts(char)
+                        && (parts(before) || (before.is_lowercase() && char.is_uppercase()))
+                }
+            };
+            if start {
+                starts.push(index);
+            }
+            before = Some(char);
+            text.push(fold(char));
+            places.push(at);
+        }
+        places.push(name.len());
+        Self {
+            text,
+            places,
+            starts,
+        }
+    }
+
+    /// How many characters the name has.
+    pub fn len(&self) -> usize {
+        self.text.len()
+    }
+
+    /// Whether the name has no characters.
+    pub fn is_empty(&self) -> bool {
+        self.text.is_empty()
+    }
+
+    /// The bytes of `len` characters from the character `from`.
+    fn span(&self, from: usize, len: usize) -> Range<usize> {
+        self.places[from]..self.places[from + len]
+    }
+
+    /// Whether every character of `query` is in the name, in order: no
+    /// tier matches a name where they are not, which most names of a
+    /// schema are not, so this is asked first.
+    fn in_order(&self, query: &[char]) -> bool {
+        let mut typed = query.iter().peekable();
+        for char in &self.text {
+            match typed.peek() {
+                Some(next) if char == *next => {
+                    typed.next();
+                }
+                Some(_) => {}
+                None => break,
+            }
+        }
+        typed.peek().is_none()
+    }
+
+    /// How the name matches `query` (as [`folded`] gives it), or `None`
+    /// where it does not. An empty query matches nothing.
+    pub fn find(&self, query: &[char]) -> Option<Hit> {
+        if query.is_empty() || self.text.len() < query.len() || !self.in_order(query) {
+            return None;
+        }
+        if self.text.starts_with(query) {
+            let tier = if self.text.len() == query.len() {
+                Tier::Exact
+            } else {
+                Tier::Prefix
+            };
+            let spans = vec![self.span(0, query.len())];
+            return Some(Hit { tier, spans });
+        }
+        if let Some(pieces) = pieces(&self.text, &self.starts, query) {
+            let spans = pieces
+                .into_iter()
+                .map(|(from, len)| self.span(from, len))
+                .collect();
+            return Some(Hit {
+                tier: Tier::WordStarts,
+                spans,
+            });
+        }
+        let at = self
+            .text
+            .windows(query.len())
+            .position(|run| run == query)?;
+        Some(Hit {
+            tier: Tier::Substring,
+            spans: vec![self.span(at, query.len())],
+        })
+    }
+}
+
+/// How `name` matches `query` (as [`folded`] gives it), or `None` where it
+/// does not: [`Name::find`] for a name matched once.
+pub fn find(query: &[char], name: &str) -> Option<Hit> {
+    Name::new(name).find(query)
+}
+
+/// What is typed, as runs that each start a word, in order: each run's
+/// first character and length. The longest run is tried first at each
+/// word, so `bocov` takes `bo` then `cov`. `None` where no such runs
+/// exist. A run may go on past its word's end (`book_c` in
+/// `old_book_covers`).
+fn pieces(text: &[char], starts: &[usize], query: &[char]) -> Option<Vec<(usize, usize)>> {
+    // One character: the first word it starts, with nothing to try again.
+    if let [char] = query {
+        let start = starts.iter().find(|&&start| text[start] == *char)?;
+        return Some(vec![(*start, 1)]);
+    }
+    // Where the rest of the query from `typed` was tried from the word
+    // `word` on and found no runs: each pair is tried once, so a name of
+    // many like words costs no more than its words times the query.
+    let mut failed = vec![false; (query.len() + 1) * (starts.len() + 1)];
+    let mut found = Vec::new();
+    walk(text, starts, query, (0, 0), &mut failed, &mut found).then_some(found)
+}
+
+fn walk(
+    text: &[char],
+    starts: &[usize],
+    query: &[char],
+    (typed, word): (usize, usize),
+    failed: &mut [bool],
+    found: &mut Vec<(usize, usize)>,
+) -> bool {
+    if typed == query.len() {
+        return true;
+    }
+    let key = typed * (starts.len() + 1) + word;
+    if failed[key] {
+        return false;
+    }
+    for (offset, &start) in starts[word..].iter().enumerate() {
+        let longest = text[start..]
+            .iter()
+            .zip(&query[typed..])
+            .take_while(|(a, b)| a == b)
+            .count();
+        for len in (1..=longest).rev() {
+            // The next run starts at a word after this run's end.
+            let end = start + len;
+            let next =
+                word + offset + 1 + starts[word + offset + 1..].partition_point(|&s| s < end);
+            found.push((start, len));
+            if walk(text, starts, query, (typed + len, next), failed, found) {
+                return true;
+            }
+            found.pop();
+        }
+    }
+    failed[key] = true;
+    false
+}
 
 #[cfg(test)]
 mod tests {
````

- [ ] **Step 4: Run the checks**

Run: `cargo fmt --all --check && cargo clippy --locked --workspace --all-targets -- -D warnings && cargo test --locked --workspace --all-targets && RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps`

Expected: all pass, among them the seven `search::tests`. The lib suite:

```
fmt exit 0
clippy exit 0
test result: ok. 2974 passed; 0 failed; 20 ignored; 0 measured; 0 filtered out
doc exit 0
```

- [ ] **Step 5: Commit**

```bash
git add src/search.rs src/lib.rs
git commit -S -m "Match a name in the brief's tiers: whole, start, word starts, anywhere"
```

---

### Task 2: Quick open's index, list, keys and panel

The index and its ranking, what the panel lists and what its actions do, the keys, and the panel in both looks. The start screen's search moves onto the index. After this task quick open is the brief's, but the sidebar's field still filters the tree (task 3).

**Files:** Create `src/app/quick_open.rs`, `src/ui/quick_open_tests.rs`; modify `src/app.rs`, `src/model.rs`, `src/keymap.rs`, `src/ui/quick_open.rs`, `src/ui/start.rs`, `src/ui/workspace.rs`, `src/ui/keys.rs`, `src/ui/cell_editor.rs`, `src/env.rs`, `src/ui/mod.rs`, `src/ui/agents_tests.rs`, `src/ui/start_tests.rs`, `src/ui/complete_tests.rs`.

- [ ] **Step 1: Write the failing tests**

The new test file and the tests that move to the new index and actions. `src/ui/mod.rs` also registers the new test file, and loses the old quick open's tests. `src/env.rs`'s test of the environment's surfaces counts quick open among them, and `src/ui/complete_tests.rs` checks that the SQL editor's open completion list keeps its ctrl+p.

````diff
diff --git a/src/app.rs b/src/app.rs
index 0020091..d185ba8 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -8806,16 +8806,6 @@ mod tests {
             .collect()
     }
 
-    /// Quick open has no Agents tab to offer where agents cannot reach the
-    /// app.
-    #[cfg(not(any(unix, windows)))]
-    #[test]
-    fn quick_open_offers_no_agents_where_they_cannot_reach_the_app() {
-        let (app, _dir) = app();
-        let tab = app.active_tab_id();
-        assert!(!app.quick_open_found(tab, "agents").contains(&Found::Agents));
-    }
-
     #[test]
     fn the_settings_start_and_stop_the_agent_server_once_the_app_serves_it() {
         let (mut app, dir) = app();
@@ -16372,7 +16362,7 @@ masked = ["orders.email"]
         }
 
         #[test]
-        fn quick_open_ranks_and_caps_results() {
+        fn the_search_ranks_by_tier_and_reads_a_schema_before_a_dot() {
             let mut harness = Harness::new();
             let tab = harness.connect_fake();
             let many: Vec<String> = (0..200).map(|i| format!("log_{i:03}")).collect();
@@ -16384,12 +16374,16 @@ masked = ["orders.email"]
                 "shop",
                 &["orders", "product_reviews_old"],
             );
-            let matches = harness.app.quick_open_matches(tab, "ord");
-            assert_eq!(matches[0].0.name, "orders");
-            assert!(harness.app.quick_open_matches(tab, "log").len() <= 50);
-            let qualified = harness.app.quick_open_matches(tab, "shop.ord");
-            assert_eq!(qualified[0].0.name, "orders");
-            assert_eq!(qualified[0].0.schema, "shop");
+            let matches = harness.app.search_objects(tab, "ord");
+            assert_eq!(matches[0].object.name, "orders");
+            // Every match is found: the lists cap what they show.
+            assert_eq!(harness.app.search_objects(tab, "log").len(), 200);
+            let qualified = harness.app.search_objects(tab, "shop.ord");
+            assert_eq!(qualified[0].object.name, "orders");
+            assert_eq!(qualified[0].object.schema, "shop");
+            // Only the name's part of the match is marked.
+            let name = std::iter::once(0..3).collect::<Vec<_>>();
+            assert_eq!(qualified[0].spans, name);
         }
 
         #[test]
@@ -16403,7 +16397,10 @@ masked = ["orders.email"]
             };
             // The fake connection has a main.users too: name the schema.
             open.query = "shop.us".into();
-            harness.app.apply(Action::QuickOpenPick);
+            harness.app.apply(Action::QuickOpenPick {
+                index: None,
+                in_editor: false,
+            });
             assert!(harness.app.dialog.is_none());
             let workspace = harness.app.workspace(tab).unwrap();
             let object = workspace.object_tab(workspace.active_tab.unwrap()).unwrap();
@@ -23173,7 +23170,7 @@ masked = ["orders.email"]
             .app
             .start_matches(tab)
             .into_iter()
-            .map(|(object, kind)| (object.name, kind))
+            .map(|found| (found.object.name.clone(), found.kind))
             .collect();
         // The `⌘P` index ranks `covers` and `cover_counts` first; grouped,
         // the tables come before the views, each in that order.
diff --git a/src/env.rs b/src/env.rs
index c2a5e1d..b65a3aa 100644
--- a/src/env.rs
+++ b/src/env.rs
@@ -902,6 +902,7 @@ mod tests {
             "src/ui/settings/preview.rs",
             "src/ui/start.rs",
             "src/ui/start_tests.rs",
+            "src/ui/quick_open.rs",
         ];
         let mut found = Vec::new();
         for (path, text) in sources() {
diff --git a/src/ui/agents_tests.rs b/src/ui/agents_tests.rs
index 805f9a0..9ed1df9 100644
--- a/src/ui/agents_tests.rs
+++ b/src/ui/agents_tests.rs
@@ -171,35 +171,31 @@ fn every_way_in_shows_the_one_tab_and_none_opens_it_by_itself() {
     harness.settle();
     assert!(front(&harness, tab));
     assert_eq!(agents_tabs(&harness, tab), 1);
-    // Quick open, from the words that ask for it.
+    // Quick open's Agents command, found by its name.
     harness.app.apply(Action::ActivateTab { tab, id: query });
     harness.app.apply(Action::OpenQuickOpen);
     if let Some(crate::model::Dialog::QuickOpen(open)) = &mut harness.app.dialog {
         open.query = "age".into();
     }
     harness.settle();
-    assert_eq!(
-        harness.app.quick_open_found(tab, "age").first(),
-        Some(&crate::app::Found::Agents)
-    );
-    harness.app.apply(Action::QuickOpenPick);
+    let Some(crate::model::Dialog::QuickOpen(open)) = &harness.app.dialog else {
+        panic!("quick open");
+    };
+    let agents = crate::app::QuickItem::Command {
+        command: crate::app::QuickCommand::Agents,
+        spans: std::iter::once(0..3).collect(),
+    };
+    let index = harness
+        .app
+        .quick_open_list(open)
+        .items()
+        .position(|item| *item == agents);
+    harness.app.apply(Action::QuickOpenPick {
+        index,
+        in_editor: false,
+    });
     assert!(front(&harness, tab));
     assert_eq!(agents_tabs(&harness, tab), 1);
-    // One letter, or letters only somewhere in the word, are a table's.
-    for typed in ["a", "s", "gen", "act"] {
-        assert_ne!(
-            harness.app.quick_open_found(tab, typed).first(),
-            Some(&crate::app::Found::Agents),
-            "{typed}"
-        );
-    }
-    // And with nothing typed, quick open is the objects'.
-    assert!(
-        !harness
-            .app
-            .quick_open_found(tab, "")
-            .contains(&crate::app::Found::Agents)
-    );
 }
 
 #[test]
diff --git a/src/ui/complete_tests.rs b/src/ui/complete_tests.rs
index 0420a77..4b292f3 100644
--- a/src/ui/complete_tests.rs
+++ b/src/ui/complete_tests.rs
@@ -556,6 +556,8 @@ fn ctrl_n_and_ctrl_p_move_the_highlight_only_in_the_terminal_look() {
     assert!(harness.app.dialog.is_none());
     harness.press(Key::P, ctrl());
     assert_eq!(selected(&harness, tab), Some(0));
+    // And here quick open did not: the open list keeps its ctrl+p.
+    assert!(harness.app.dialog.is_none());
 
     // The other looks: not the list's keys. Ctrl+P is quick open where
     // Mod is Ctrl (it reports `command` too); Ctrl+N is nothing from an
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
index 3d620f1..3c6dad6 100644
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -60,6 +60,8 @@ pub mod picker;
 #[cfg(test)]
 mod preview_settings_tests;
 pub mod quick_open;
+#[cfg(test)]
+mod quick_open_tests;
 pub mod review;
 pub mod row_form;
 pub mod row_panel;
@@ -3041,22 +3043,6 @@ value_tags   = true
         assert_ne!(flat(&harness), before);
     }
 
-    #[test]
-    fn quick_open_has_its_chord_on_the_mac_and_none_on_omarchy() {
-        for (look, opens) in [
-            (Look::macos(), true),
-            (Look::standard(), true),
-            (Look::omarchy(), false),
-        ] {
-            let mut harness = Harness::new();
-            harness.set_look(look);
-            harness.connect_fake();
-            harness.press(Key::P, Modifiers::COMMAND);
-            let open = matches!(harness.app.dialog, Some(crate::model::Dialog::QuickOpen(_)));
-            assert_eq!(open, opens, "{}", look.name);
-        }
-    }
-
     fn cancels(harness: &Harness) -> usize {
         let sent = harness.app.backend.sent.iter();
         sent.filter(|c| matches!(c, Command::Cancel { .. })).count()
@@ -5622,7 +5608,7 @@ value_tags   = true
         assert!(!harness.has("orders"));
         harness.press(Key::B, Modifiers::COMMAND);
         harness.press(Key::P, Modifiers::COMMAND);
-        assert!(harness.has("Open table or view"));
+        assert!(harness.has("Quick open"));
         harness.press(Key::Escape, Modifiers::NONE);
         assert!(harness.app.dialog.is_none());
         harness.frame(vec![egui::Event::Text("?".into())]);
@@ -10406,79 +10392,6 @@ value_tags   = true
         workspace.databases.value = Some(vec!["tabletist".into(), "postgres".into()]);
     }
 
-    #[test]
-    fn command_p_opens_quick_open_and_enter_opens_the_match() {
-        let (mut harness, tab) = tree_harness();
-        harness.press(Key::P, Modifiers::COMMAND);
-        assert!(harness.has("Open table or view"));
-        harness.frame(vec![egui::Event::Text("users".into())]);
-        harness.press(Key::Enter, Modifiers::NONE);
-        assert!(harness.app.dialog.is_none());
-        let workspace = harness.app.workspace(tab).unwrap();
-        let object = workspace.object_tab(workspace.active_tab.unwrap()).unwrap();
-        assert_eq!(object.object.name, "users");
-    }
-
-    #[test]
-    fn quick_open_shortens_long_names_and_shows_them_on_hover() {
-        let (mut harness, tab) = tree_harness();
-        let long = "an_uncommonly_long_table_name_that_runs_well_past_the_right_edge_of_the_quick_open_dialog_in_every_look";
-        let objects = &mut harness
-            .app
-            .workspace_mut(tab)
-            .unwrap()
-            .tree
-            .nodes
-            .get_mut("main")
-            .unwrap()
-            .objects;
-        objects
-            .value
-            .as_mut()
-            .unwrap()
-            .push(tabletist_db::ObjectInfo {
-                name: long.into(),
-                kind: tabletist_db::ObjectKind::Table,
-                estimated_rows: None,
-            });
-        harness.press(Key::P, Modifiers::COMMAND);
-        let mut count_on_hover = |name: &str| {
-            let tree = harness.settle();
-            let row = tree
-                .nodes
-                .iter()
-                .find(|(_, node)| node.label() == Some(name))
-                .and_then(|(_, node)| node.bounds())
-                .unwrap_or_else(|| panic!("a row for {name}"));
-            let over = egui::pos2(row.x0 as f32 + 20.0, ((row.y0 + row.y1) / 2.0) as f32);
-            harness.frame(vec![egui::Event::PointerMoved(over)]);
-            // Past the tooltip delay: each frame is 1/60 s.
-            for _ in 0..60 {
-                harness.frame(Vec::new());
-            }
-            let tree = harness.frame(Vec::new());
-            crate::testing::labels(&tree)
-                .iter()
-                .filter(|label| *label == name)
-                .count()
-        };
-        let full = format!("main.{long}");
-        assert_eq!(count_on_hover(&full), 2, "the row and its tooltip");
-        assert_eq!(
-            count_on_hover("main.users"),
-            1,
-            "a short name has no tooltip"
-        );
-    }
-
-    #[test]
-    fn quick_open_explains_what_it_searches() {
-        let mut harness = Harness::new();
-        harness.connect_fake();
-        harness.press(Key::P, Modifiers::COMMAND);
-        assert!(harness.has("Searches schemas loaded in the sidebar"));
-    }
-
     #[test]
     fn clicking_the_tree_sends_arrows_there_and_the_grid_takes_them_back() {
         let (mut harness, tab) = tree_harness();
@@ -11376,10 +11289,12 @@ value_tags   = true
         assert_eq!(harness.painted_color("Counting…"), Some(secondary));
         assert_eq!(harness.painted_color("Unordered"), Some(secondary));
 
+        // Quick open's line when nothing matches.
         harness.press(Key::P, Modifiers::COMMAND);
+        harness.frame(vec![egui::Event::Text("zzz".into())]);
         harness.settle();
         assert_eq!(
-            harness.painted_color("Searches schemas loaded in the sidebar"),
+            harness.painted_color("No objects match “zzz” in Fixture · dev"),
             Some(secondary)
         );
     }
@@ -12392,62 +12307,6 @@ value_tags   = true
         assert!(harness.ctx.text_edit_focused());
     }
 
-    #[test]
-    fn quick_open_keeps_a_wheel_scroll() {
-        let (mut harness, tab) = tree_harness();
-        let names: Vec<String> = (0..60).map(|i| format!("t_{i:03}")).collect();
-        harness
-            .app
-            .workspace_mut(tab)
-            .unwrap()
-            .tree
-            .nodes
-            .get_mut("main")
-            .unwrap()
-            .objects
-            .value = Some(
-            names
-                .iter()
-                .map(|name| tabletist_db::ObjectInfo {
-                    name: name.clone(),
-                    kind: tabletist_db::ObjectKind::Table,
-                    estimated_rows: None,
-                })
-                .collect(),
-        );
-        harness.press(Key::P, Modifiers::COMMAND);
-        harness.settle();
-        let tree = harness.settle();
-        let list = tree
-            .nodes
-            .iter()
-            .find(|(_, node)| node.label() == Some("main.t_005"))
-            .and_then(|(_, node)| node.bounds())
-            .expect("a result row");
-        let over = egui::pos2(list.x0 as f32 + 20.0, list.y0 as f32 + 5.0);
-        harness.frame(vec![egui::Event::PointerMoved(over)]);
-        harness.frame(vec![
-            egui::Event::PointerMoved(over),
-            egui::Event::MouseWheel {
-                unit: egui::MouseWheelUnit::Point,
-                delta: egui::vec2(0.0, -200.0),
-                modifiers: Modifiers::NONE,
-                phase: egui::TouchPhase::Move,
-            },
-        ]);
-        for _ in 0..120 {
-            harness.frame(vec![]);
-        }
-        let Some(crate::model::Dialog::QuickOpen(open)) = &harness.app.dialog else {
-            panic!("quick open");
-        };
-        assert!(
-            open.scroll_offset > 50.0,
-            "snapped back to {}",
-            open.scroll_offset
-        );
-    }
-
     #[test]
     fn the_disconnected_banner_shows_the_exact_error_too() {
         let mut harness = Harness::new();
diff --git a/src/ui/quick_open_tests.rs b/src/ui/quick_open_tests.rs
new file mode 100644
index 0000000..6e087b9
--- /dev/null
+++ b/src/ui/quick_open_tests.rs
@@ -0,0 +1,905 @@
+//! Quick open: opening and closing it from every part of a window, what
+//! it lists with nothing typed and with a query, how it ranks, the scope
+//! of every open connection, what opening a result does, the Omarchy
+//! picker's digits, and how quickly a large schema answers. The Bookshop
+//! demo data only.
+
+use egui::accesskit::Role;
+use egui::{Key, Modifiers};
+use tabletist_db::{ObjectInfo, ObjectKind, ObjectRef};
+
+use crate::app::{QuickCommand, QuickHeading, QuickItem, QuickRow, StartRow};
+use crate::backend::Command;
+use crate::connections::SavedConnection;
+use crate::env::Environment;
+use crate::model::{Action, ConnTabId, Dialog, QuickGroup, QuickOpen};
+use crate::search::Tier;
+use crate::testing::{Harness, bookshop_objects, fixture_connection};
+use crate::theme::Look;
+
+/// The Bookshop's development connection.
+fn bookshop_dev() -> SavedConnection {
+    let mut saved = fixture_connection();
+    saved.name = "Bookshop".into();
+    saved
+}
+
+/// The Bookshop's production connection, on its own file.
+fn bookshop_production() -> SavedConnection {
+    let mut saved = fixture_connection();
+    saved.name = "Bookshop".into();
+    saved.environment = Environment::Production;
+    saved.spec = tabletist_db::ConnectSpec::sqlite("/tmp/bookshop_production.db");
+    saved
+}
+
+/// The Bookshop's objects with the publishers' tables and views of the
+/// boards: six tables and two views whose names start `pub`.
+fn bookshop_with_publishers() -> Vec<ObjectInfo> {
+    let table = |name: &str, rows: u64| ObjectInfo {
+        name: name.into(),
+        kind: ObjectKind::Table,
+        estimated_rows: Some(rows),
+    };
+    let view = |name: &str| ObjectInfo {
+        name: name.into(),
+        kind: ObjectKind::View,
+        estimated_rows: None,
+    };
+    let mut objects: Vec<ObjectInfo> = bookshop_objects()
+        .into_iter()
+        .filter(|info| info.name != "publishers")
+        .collect();
+    objects.extend([
+        table("publishers", 6),
+        table("publishers_members", 6_100),
+        table("publishers_settings", 6),
+        table("publishers_contacts", 14),
+        table("publishers_imprints", 22),
+        table("publishers_regions", 4),
+        view("publishers_overview"),
+        view("publishers_billing_status"),
+    ]);
+    objects
+}
+
+/// A Bookshop development window in `look`, its start screen showing.
+fn bookshop(look: Look) -> (Harness, ConnTabId) {
+    let mut harness = Harness::new();
+    harness.set_look(look);
+    let tab = harness.connect_with(bookshop_dev(), bookshop_with_publishers());
+    (harness, tab)
+}
+
+/// Opens `name` of the Bookshop in `tab`, its first page read.
+fn open_table(harness: &mut Harness, tab: ConnTabId, name: &str) {
+    harness.app.apply(Action::OpenObject {
+        tab,
+        object: ObjectRef::new("main", name),
+        kind: ObjectKind::Table,
+        pin: true,
+    });
+    harness.answer_rows(crate::testing::page(3, false));
+}
+
+/// The open quick open.
+fn quick_open(harness: &Harness) -> &QuickOpen {
+    match &harness.app.dialog {
+        Some(Dialog::QuickOpen(open)) => open,
+        _ => panic!("quick open is not open"),
+    }
+}
+
+/// Types `text` into what has the keyboard, one frame.
+fn type_text(harness: &mut Harness, text: &str) {
+    harness.frame(vec![egui::Event::Text(text.into())]);
+    harness.settle();
+}
+
+/// Opens quick open with its key in `look`, and types `query`.
+fn ask(harness: &mut Harness, look: Look, query: &str) {
+    harness.press(Key::P, chord(look));
+    assert!(
+        matches!(harness.app.dialog, Some(Dialog::QuickOpen(_))),
+        "{}",
+        look.name
+    );
+    if !query.is_empty() {
+        type_text(harness, query);
+    }
+}
+
+/// Quick open's key, held with P: ⌘P on macOS, ctrl+p on Omarchy.
+fn chord(look: Look) -> Modifiers {
+    if look.terminal {
+        Modifiers::CTRL
+    } else {
+        Modifiers::COMMAND
+    }
+}
+
+/// The names of the listed results, and of the headings, in order.
+fn listed(harness: &Harness) -> Vec<String> {
+    let list = harness.app.quick_open_list(quick_open(harness));
+    list.rows
+        .iter()
+        .map(|row| match row {
+            QuickRow::Heading(heading) => format!("# {heading:?}"),
+            QuickRow::Item(QuickItem::Object { found, .. }) => found.object.name.clone(),
+            QuickRow::Item(QuickItem::Start { row, .. }) => match row {
+                StartRow::Recent(crate::model::StartRecent::Object { object, .. })
+                | StartRow::Largest(object, _) => object.name.clone(),
+                StartRow::Recent(crate::model::StartRecent::Sql { number, .. }) => {
+                    format!("Query {number}")
+                }
+            },
+            QuickRow::Item(QuickItem::Command { command, .. }) => {
+                format!("> {}", command.name())
+            }
+            QuickRow::Item(QuickItem::More { hidden, .. }) => format!("+{hidden}"),
+        })
+        .collect()
+}
+
+/// The name a screen reader would read for what has the keyboard.
+fn focused(harness: &mut Harness) -> String {
+    super::tests::focused_name(&harness.settle())
+}
+
+/// Steps with F6 until the keyboard is on `name`.
+fn reach(harness: &mut Harness, name: &str) {
+    let mut seen = vec![focused(harness)];
+    while seen.last().is_none_or(|last| last != name) {
+        assert!(seen.len() <= 12, "{name} not reached: {seen:?}");
+        harness.press(Key::F6, Modifiers::NONE);
+        seen.push(focused(harness));
+    }
+}
+
+/// A click at `at`: the pointer moves there, presses and lets go.
+fn click_at(harness: &mut Harness, at: egui::Pos2) {
+    harness.frame(vec![egui::Event::PointerMoved(at)]);
+    let button = |pressed| egui::Event::PointerButton {
+        pos: at,
+        button: egui::PointerButton::Primary,
+        pressed,
+        modifiers: Modifiers::NONE,
+    };
+    harness.frame(vec![button(true)]);
+    harness.frame(vec![button(false)]);
+    harness.settle();
+}
+
+/// Test 1: the key opens quick open from the rows, the SQL editor, the
+/// start screen and the Agents tab, in both looks; Esc and a click
+/// outside close it.
+#[test]
+fn the_key_opens_quick_open_from_every_part_of_a_window() {
+    for look in [Look::macos(), Look::omarchy()] {
+        for place in ["start", "rows", "sql", "agents"] {
+            let (mut harness, tab) = bookshop(look);
+            match place {
+                "rows" => open_table(&mut harness, tab, "authors"),
+                "sql" => {
+                    harness.app.apply(Action::NewSqlTab(tab));
+                    harness.settle();
+                }
+                "agents" => {
+                    harness.app.apply(Action::ShowAgents(tab));
+                    harness.settle();
+                }
+                _ => {}
+            }
+            harness.settle();
+            ask(&mut harness, look, "");
+            harness.press(Key::Escape, Modifiers::NONE);
+            assert!(harness.app.dialog.is_none(), "{} {place}", look.name);
+            // A click outside the panel closes it too.
+            ask(&mut harness, look, "");
+            click_at(&mut harness, egui::pos2(40.0, 760.0));
+            assert!(harness.app.dialog.is_none(), "{} {place}", look.name);
+        }
+    }
+}
+
+/// Test 1: what had the keyboard has it again, after Esc and after a
+/// click outside.
+#[test]
+fn closing_quick_open_gives_the_keyboard_back_to_what_had_it() {
+    for look in [Look::macos(), Look::omarchy()] {
+        let (mut harness, tab) = bookshop(look);
+        open_table(&mut harness, tab, "authors");
+        reach(&mut harness, "Rows");
+        ask(&mut harness, look, "");
+        harness.press(Key::Escape, Modifiers::NONE);
+        assert_eq!(focused(&mut harness), "Rows", "{}", look.name);
+        reach(&mut harness, "Objects");
+        ask(&mut harness, look, "pub");
+        click_at(&mut harness, egui::pos2(40.0, 760.0));
+        assert!(harness.app.dialog.is_none());
+        assert_eq!(focused(&mut harness), "Objects", "{}", look.name);
+    }
+}
+
+/// The key again while quick open is open keeps the one panel, and
+/// selects what its field holds: what is typed next replaces it.
+#[test]
+fn the_key_again_selects_what_is_typed() {
+    let (mut harness, _tab) = bookshop(Look::macos());
+    ask(&mut harness, Look::macos(), "pub");
+    harness.press(Key::P, Modifiers::COMMAND);
+    harness.settle();
+    assert_eq!(quick_open(&harness).query, "pub");
+    type_text(&mut harness, "cov");
+    assert_eq!(quick_open(&harness).query, "cov");
+}
+
+/// Test 2: with nothing typed, the recents are the start screen's list,
+/// then (macOS) the commands.
+#[test]
+fn with_nothing_typed_the_recents_are_the_start_screens() {
+    for look in [Look::macos(), Look::omarchy()] {
+        let (mut harness, tab) = bookshop(look);
+        // Six opened: the start screen lists the newest five.
+        for name in [
+            "authors",
+            "editions",
+            "book_covers",
+            "publishers",
+            "ratings",
+            "reviews",
+        ] {
+            open_table(&mut harness, tab, name);
+        }
+        ask(&mut harness, look, "");
+        let open = quick_open(&harness);
+        let list = harness.app.quick_open_list(open);
+        let recents: Vec<StartRow> = list
+            .items()
+            .filter_map(|item| match item {
+                QuickItem::Start { row, .. } => Some(row.clone()),
+                _ => None,
+            })
+            .collect();
+        let start: Vec<StartRow> = harness
+            .app
+            .start_recents(tab)
+            .into_iter()
+            .map(StartRow::Recent)
+            .collect();
+        assert_eq!(recents, start, "{}", look.name);
+        assert_eq!(recents.len(), crate::recents::SHOWN);
+        assert_eq!(list.rows[0], QuickRow::Heading(QuickHeading::Recent));
+        let commands: Vec<String> = listed(&harness)
+            .into_iter()
+            .filter(|name| name.starts_with("> "))
+            .collect();
+        if look.terminal {
+            // Omarchy's commands stay on `:`.
+            assert!(commands.is_empty());
+        } else {
+            assert_eq!(commands[0], "> New SQL query");
+        }
+    }
+}
+
+/// Test 3: `pub` lists the publishers' tables, three and a "more" row,
+/// then the views, each under its heading; the "more" row shows the rest
+/// in place.
+#[test]
+fn a_query_lists_tables_then_views_three_of_each_and_more() {
+    let (mut harness, _tab) = bookshop(Look::macos());
+    ask(&mut harness, Look::macos(), "pub");
+    assert_eq!(
+        listed(&harness),
+        [
+            "# Tables",
+            "publishers",
+            "publishers_members",
+            "publishers_regions",
+            "+3",
+            "# Views",
+            "publishers_overview",
+            "publishers_billing_status",
+        ]
+    );
+    // The "more" row is the fourth result: Enter on it shows the group.
+    for _ in 0..3 {
+        harness.press(Key::ArrowDown, Modifiers::NONE);
+    }
+    harness.press(Key::Enter, Modifiers::NONE);
+    assert_eq!(quick_open(&harness).expanded, [QuickGroup::Tables]);
+    let names = listed(&harness);
+    assert_eq!(names.len(), 10, "{names:?}");
+    assert!(!names.contains(&"+3".to_owned()));
+    // What it hid is picked now: Enter opens it.
+    assert_eq!(quick_open(&harness).selected, 3);
+    // Typing again folds it.
+    type_text(&mut harness, "l");
+    assert!(quick_open(&harness).expanded.is_empty());
+}
+
+/// Test 3, Omarchy: one numbered list of every match, the kind in a
+/// column, no headings and no commands.
+#[test]
+fn omarchy_lists_every_match_in_one_list() {
+    let (mut harness, _tab) = bookshop(Look::omarchy());
+    ask(&mut harness, Look::omarchy(), "pub");
+    let names = listed(&harness);
+    assert_eq!(names.len(), 8, "{names:?}");
+    assert!(names.iter().all(|name| !name.starts_with('#')));
+    // The count stands at the prompt's right: eight shown, none hidden.
+    assert!(harness.painted_color("8 matches").is_some());
+    type_text(&mut harness, "l");
+    assert!(harness.painted_color("8 matches").is_some());
+}
+
+/// Test 4: exact names, then prefixes, then word starts, then any run;
+/// `bc` finds `book_covers`; a recent object ranks first in its tier.
+#[test]
+fn matches_rank_by_tier_then_by_recency() {
+    let (mut harness, tab) = bookshop(Look::macos());
+    let found = |harness: &Harness, query: &str| -> Vec<(String, Tier)> {
+        harness
+            .app
+            .search_objects(tab, query)
+            .into_iter()
+            .map(|found| (found.object.name.clone(), found.tier))
+            .collect()
+    };
+    assert_eq!(
+        found(&harness, "covers"),
+        [
+            ("covers".to_owned(), Tier::Exact),
+            ("book_covers".to_owned(), Tier::WordStarts),
+            ("missing_covers".to_owned(), Tier::WordStarts),
+        ]
+    );
+    assert_eq!(
+        found(&harness, "bc"),
+        [("book_covers".to_owned(), Tier::WordStarts)]
+    );
+    assert_eq!(found(&harness, "vers")[0].1, Tier::Substring);
+    // Shorter names first within a tier, then by name, until one of
+    // them is opened.
+    let names: Vec<String> = found(&harness, "publishers_")
+        .into_iter()
+        .map(|(name, _)| name)
+        .collect();
+    assert_eq!(names[..2], ["publishers_members", "publishers_regions"]);
+    open_table(&mut harness, tab, "publishers_settings");
+    let names: Vec<String> = found(&harness, "publishers_")
+        .into_iter()
+        .map(|(name, _)| name)
+        .collect();
+    assert_eq!(names[0], "publishers_settings");
+    // The exact name still comes first.
+    assert_eq!(found(&harness, "publishers")[0].0, "publishers");
+}
+
+/// Test 5: typing asks the database nothing: the results come from the
+/// listed schema.
+#[test]
+fn typing_queries_nothing() {
+    for look in [Look::macos(), Look::omarchy()] {
+        let (mut harness, _tab) = bookshop(look);
+        ask(&mut harness, look, "");
+        let sent = harness.app.backend.sent.len();
+        for letter in ["p", "u", "b", "l"] {
+            type_text(&mut harness, letter);
+        }
+        harness.press(Key::Tab, Modifiers::NONE);
+        type_text(&mut harness, "i");
+        let asked: Vec<&Command> = harness.app.backend.sent[sent..]
+            .iter()
+            .filter(|command| {
+                matches!(
+                    command,
+                    Command::ListSchemas { .. }
+                        | Command::ListObjects { .. }
+                        | Command::Describe { .. }
+                        | Command::FetchRows { .. }
+                        | Command::CountRows { .. }
+                        | Command::RunSql { .. }
+                )
+            })
+            .collect();
+        assert!(asked.is_empty(), "{}: {asked:?}", look.name);
+        assert_eq!(quick_open(&harness).query, "publi");
+    }
+}
+
+/// Test 7: Tab searches every open connection, grouped by connection;
+/// opening a production result brings its window to the front with the
+/// table open; the next open searches this connection again.
+#[test]
+fn tab_searches_every_open_connection() {
+    for look in [Look::macos(), Look::omarchy()] {
+        let (mut harness, dev) = bookshop(look);
+        harness.app.apply(Action::ShowConnections);
+        let production = harness.connect_with(bookshop_production(), bookshop_objects());
+        harness.app.apply(Action::ActivateConnTab(dev));
+        harness.settle();
+        ask(&mut harness, look, "cov");
+        harness.press(Key::Tab, Modifiers::NONE);
+        assert!(quick_open(&harness).everywhere, "{}", look.name);
+        let names = listed(&harness);
+        let headings: Vec<&String> = names.iter().filter(|name| name.starts_with('#')).collect();
+        assert_eq!(
+            headings,
+            [
+                &format!("# {:?}", QuickHeading::Connection(dev)),
+                &format!("# {:?}", QuickHeading::Connection(production)),
+            ],
+            "{}",
+            look.name
+        );
+        // The production window's `covers`: the fourth result, after
+        // this window's three.
+        let list = harness.app.quick_open_list(quick_open(&harness));
+        let index = list
+            .items()
+            .position(|item| {
+                matches!(item, QuickItem::Object { tab, found }
+                    if *tab == production && found.object.name == "covers")
+            })
+            .expect("production's covers");
+        harness.app.apply(Action::QuickOpenPick {
+            index: Some(index),
+            in_editor: false,
+        });
+        harness.settle();
+        assert_eq!(harness.app.active_tab_id(), production, "{}", look.name);
+        let workspace = harness.app.workspace(production).unwrap();
+        let object = workspace.object_tab(workspace.active_tab.unwrap()).unwrap();
+        assert_eq!(object.object.name, "covers");
+        // Tab again searches this connection; and each open starts there.
+        ask(&mut harness, look, "cov");
+        assert!(!quick_open(&harness).everywhere);
+        harness.press(Key::Tab, Modifiers::NONE);
+        harness.press(Key::Tab, Modifiers::NONE);
+        assert!(!quick_open(&harness).everywhere);
+    }
+}
+
+/// Test 7, macOS: the chip says what is searched, and on production it is
+/// the red pill.
+#[test]
+fn the_chip_names_what_is_searched() {
+    let (mut harness, _dev) = bookshop(Look::macos());
+    ask(&mut harness, Look::macos(), "");
+    assert!(harness.has("Bookshop · dev"));
+    harness.press(Key::Tab, Modifiers::NONE);
+    assert!(harness.has("All open connections"));
+    harness.press(Key::Escape, Modifiers::NONE);
+    let mut harness = Harness::new();
+    harness.set_look(Look::macos());
+    harness.connect_with(bookshop_production(), bookshop_objects());
+    ask(&mut harness, Look::macos(), "");
+    let pill = |(text, color): &(String, egui::Color32)| {
+        text == "production" && *color == egui::Color32::WHITE
+    };
+    assert!(
+        harness.painted.iter().any(pill),
+        "the pill's words on the danger red"
+    );
+}
+
+/// Nothing matches: the panel says where it looked and offers every open
+/// connection; there it offers nothing more.
+#[test]
+fn no_match_says_where_it_looked() {
+    let (mut harness, _tab) = bookshop(Look::macos());
+    ask(&mut harness, Look::macos(), "zzz");
+    assert!(harness.has("No objects match “zzz” in Bookshop · dev"));
+    // The link names the key that does the same.
+    let key = harness.app.keymap.key_in(
+        harness.app.layout(),
+        crate::keymap::Command::QuickOpenScope,
+        crate::keymap::Scope::Prompt,
+    );
+    let link = format!("Search all open connections {}", key.as_str());
+    harness.click(&link);
+    assert!(quick_open(&harness).everywhere);
+    harness.settle();
+    assert!(harness.has("No objects match “zzz” in open connections"));
+    assert!(!harness.has(&link));
+}
+
+/// Test 8: Enter opens a result as a tab, or shows the tab it has; ⌥↩
+/// opens a SQL editor reading its first rows, with the Editor's row
+/// limit.
+#[test]
+fn enter_opens_or_shows_a_tab_and_option_enter_opens_an_editor() {
+    let (mut harness, tab) = bookshop(Look::macos());
+    ask(&mut harness, Look::macos(), "authors");
+    harness.press(Key::Enter, Modifiers::NONE);
+    assert!(harness.app.dialog.is_none());
+    open_table(&mut harness, tab, "editions");
+    let tabs = harness.app.workspace(tab).unwrap().tabs.len();
+    ask(&mut harness, Look::macos(), "authors");
+    harness.press(Key::Enter, Modifiers::NONE);
+    let workspace = harness.app.workspace(tab).unwrap();
+    assert_eq!(workspace.tabs.len(), tabs, "the tab it had is shown");
+    let object = workspace.object_tab(workspace.active_tab.unwrap()).unwrap();
+    assert_eq!(object.object.name, "authors");
+    harness.app.settings.sql_limit = 250;
+    ask(&mut harness, Look::macos(), "editions");
+    harness.press(Key::Enter, Modifiers::ALT);
+    let workspace = harness.app.workspace(tab).unwrap();
+    let sql = workspace.active_sql_tab().expect("a SQL editor");
+    let dialect = workspace.driver.dialect();
+    let expected = format!(
+        "SELECT * FROM {}.{} LIMIT 250",
+        dialect.ident("main"),
+        dialect.ident("editions")
+    );
+    assert_eq!(sql.text, expected);
+}
+
+/// The macOS commands with nothing typed, and what they do; on production
+/// New SQL query says its editor opens read-only.
+#[test]
+fn the_commands_do_what_they_say() {
+    let (mut harness, tab) = bookshop(Look::macos());
+    ask(&mut harness, Look::macos(), "new sql");
+    let list = harness.app.quick_open_list(quick_open(&harness));
+    let first = list.items().next().cloned();
+    assert!(
+        matches!(
+            first,
+            Some(QuickItem::Command {
+                command: QuickCommand::NewSql { read_only: false },
+                ..
+            })
+        ),
+        "{first:?}"
+    );
+    harness.press(Key::Enter, Modifiers::NONE);
+    assert!(harness.app.dialog.is_none());
+    assert!(
+        harness
+            .app
+            .workspace(tab)
+            .unwrap()
+            .active_sql_tab()
+            .is_some()
+    );
+    // One database: no Switch database to offer.
+    ask(&mut harness, Look::macos(), "switch");
+    assert!(!listed(&harness).contains(&"> Switch database".to_owned()));
+    harness.press(Key::Escape, Modifiers::NONE);
+    let mut harness = Harness::new();
+    harness.set_look(Look::macos());
+    harness.connect_with(bookshop_production(), bookshop_objects());
+    ask(&mut harness, Look::macos(), "");
+    harness.settle();
+    assert!(harness.painted_color("opens read-only").is_some());
+}
+
+/// The Agents command is offered only where agents can reach the app.
+#[test]
+fn the_agents_command_is_offered_where_agents_can_reach_the_app() {
+    let (mut harness, _tab) = bookshop(Look::macos());
+    ask(&mut harness, Look::macos(), "");
+    let offered = listed(&harness).contains(&"> Agents".to_owned());
+    assert_eq!(offered, crate::endpoint::AVAILABLE);
+}
+
+/// Opening a result adds it to the connection's recents.
+#[test]
+fn opening_a_result_makes_it_recent() {
+    let (mut harness, tab) = bookshop(Look::macos());
+    ask(&mut harness, Look::macos(), "ratings");
+    harness.press(Key::Enter, Modifiers::NONE);
+    let recents = harness.app.start_recents(tab);
+    assert!(matches!(
+        recents.first(),
+        Some(crate::model::StartRecent::Object { object, .. }) if object.name == "ratings"
+    ));
+}
+
+/// Test 10: Omarchy's digits open a numbered recent while nothing is
+/// typed; once anything is, they are typed, and so is `>`; `:` still
+/// opens the command line once quick open is closed.
+#[test]
+fn omarchy_digits_pick_only_while_nothing_is_typed() {
+    let look = Look::omarchy();
+    let (mut harness, tab) = bookshop(look);
+    for name in ["authors", "editions"] {
+        open_table(&mut harness, tab, name);
+    }
+    ask(&mut harness, look, "");
+    // `2` is the second recent: authors, opened before editions.
+    harness.frame(vec![
+        crate::testing::key(Key::Num2, Modifiers::NONE),
+        egui::Event::Text("2".into()),
+    ]);
+    harness.frame(vec![crate::testing::release(Key::Num2, Modifiers::NONE)]);
+    harness.settle();
+    assert!(harness.app.dialog.is_none());
+    let workspace = harness.app.workspace(tab).unwrap();
+    let object = workspace.object_tab(workspace.active_tab.unwrap()).unwrap();
+    assert_eq!(object.object.name, "authors");
+    // Typed after a letter, a digit is the query's.
+    ask(&mut harness, look, "a");
+    harness.frame(vec![
+        crate::testing::key(Key::Num2, Modifiers::NONE),
+        egui::Event::Text("2".into()),
+    ]);
+    harness.settle();
+    type_text(&mut harness, ">");
+    assert_eq!(quick_open(&harness).query, "a2>");
+    harness.press(Key::Escape, Modifiers::NONE);
+    assert!(harness.app.dialog.is_none());
+    super::tests::type_key(&mut harness, Key::Colon, ":");
+    assert!(harness.app.workspace(tab).unwrap().command.is_some());
+}
+
+/// Omarchy's ctrl+n and ctrl+p move the pick. (The SQL editor's open
+/// completion list keeps its ctrl+p: `ui::complete_tests`.)
+#[test]
+fn omarchy_ctrl_n_and_ctrl_p_move_the_pick() {
+    let look = Look::omarchy();
+    let (mut harness, _tab) = bookshop(look);
+    ask(&mut harness, look, "pub");
+    harness.press(Key::N, Modifiers::CTRL);
+    harness.press(Key::N, Modifiers::CTRL);
+    assert_eq!(quick_open(&harness).selected, 2);
+    harness.press(Key::P, Modifiers::CTRL);
+    assert_eq!(quick_open(&harness).selected, 1);
+    assert!(matches!(harness.app.dialog, Some(Dialog::QuickOpen(_))));
+}
+
+/// The panel is a dialog named for the look; the field is a combobox
+/// that controls the list; the picked result is said with its kind, its
+/// schema and its estimate; headings are labels, not options.
+#[test]
+fn a_screen_reader_reads_the_panel() {
+    for (look, name) in [
+        (Look::macos(), "Quick open"),
+        (Look::omarchy(), "Find object"),
+    ] {
+        let (mut harness, _tab) = bookshop(look);
+        ask(&mut harness, look, "pub");
+        let tree = harness.settle();
+        assert!(
+            crate::testing::node(&tree, name, Role::Dialog).is_some(),
+            "{}",
+            look.name
+        );
+        let combo = tree
+            .nodes
+            .iter()
+            .find(|(_, node)| node.role() == Role::EditableComboBox)
+            .expect("the field");
+        assert_eq!(combo.1.controls().len(), 1);
+        let estimate = if look.terminal { "≈ 6" } else { "≈ 6 rows" };
+        let picked = format!("publishers, table, main, {estimate}");
+        let option = tree
+            .nodes
+            .iter()
+            .find(|(_, node)| {
+                node.role() == Role::ListBoxOption && node.is_selected() == Some(true)
+            })
+            .expect("a picked option");
+        assert_eq!(option.1.label(), Some(picked.as_str()), "{}", look.name);
+        assert_eq!(combo.1.active_descendant(), Some(option.0));
+        if !look.terminal {
+            assert!(crate::testing::node(&tree, "Tables", Role::Label).is_some());
+            assert!(crate::testing::node(&tree, "Tables", Role::ListBoxOption).is_none());
+        }
+    }
+}
+
+/// A long name ends in "…" and shows in full on hover.
+#[test]
+fn a_long_name_is_cut_and_shown_in_full_on_hover() {
+    let long =
+        "publishers_with_an_uncommonly_long_name_that_runs_well_past_the_right_edge_of_quick_open";
+    let mut objects = bookshop_objects();
+    objects.push(ObjectInfo {
+        name: long.into(),
+        kind: ObjectKind::Table,
+        estimated_rows: None,
+    });
+    let mut harness = Harness::new();
+    harness.set_look(Look::macos());
+    harness.connect_with(bookshop_dev(), objects);
+    ask(&mut harness, Look::macos(), "publishers_with");
+    let tree = harness.settle();
+    let label = format!("{long}, table, main");
+    let row = crate::testing::bounds(&tree, &label, Role::ListBoxOption).expect("the row");
+    let shown = |tree: &egui::accesskit::TreeUpdate| {
+        crate::testing::labels(tree)
+            .iter()
+            .any(|label| label == long)
+    };
+    assert!(!shown(&tree), "no tooltip before the pointer comes");
+    harness.frame(vec![egui::Event::PointerMoved(row.center())]);
+    // Past the tooltip's delay: each frame is 1/60 s.
+    for _ in 0..60 {
+        harness.frame(Vec::new());
+    }
+    let tree = harness.frame(Vec::new());
+    assert!(shown(&tree), "the name in full on hover");
+}
+
+/// The list keeps a wheel scroll: it scrolls only when the pick moves.
+#[test]
+fn the_list_keeps_a_wheel_scroll() {
+    let mut objects = bookshop_objects();
+    objects.extend((0..60).map(|i| ObjectInfo {
+        name: format!("reviews_{i:03}"),
+        kind: ObjectKind::Table,
+        estimated_rows: None,
+    }));
+    let mut harness = Harness::new();
+    harness.set_look(Look::omarchy());
+    harness.connect_with(bookshop_dev(), objects);
+    ask(&mut harness, Look::omarchy(), "reviews_");
+    let tree = harness.settle();
+    let row = crate::testing::bounds(&tree, "reviews_000, table, main", Role::ListBoxOption)
+        .expect("a result");
+    harness.frame(vec![egui::Event::PointerMoved(row.center())]);
+    harness.frame(vec![
+        egui::Event::PointerMoved(row.center()),
+        egui::Event::MouseWheel {
+            unit: egui::MouseWheelUnit::Point,
+            delta: egui::vec2(0.0, -200.0),
+            modifiers: Modifiers::NONE,
+            phase: egui::TouchPhase::Move,
+        },
+    ]);
+    for _ in 0..120 {
+        harness.frame(vec![]);
+    }
+    let offset = quick_open(&harness).scroll_offset;
+    assert!(offset > 50.0, "snapped back to {offset}");
+}
+
+/// Spaces alone are nothing typed: the digits still open a recent.
+#[test]
+fn omarchy_digits_pick_after_spaces_alone() {
+    let look = Look::omarchy();
+    let (mut harness, tab) = bookshop(look);
+    for name in ["authors", "editions"] {
+        open_table(&mut harness, tab, name);
+    }
+    ask(&mut harness, look, " ");
+    harness.frame(vec![
+        crate::testing::key(Key::Num2, Modifiers::NONE),
+        egui::Event::Text("2".into()),
+    ]);
+    harness.settle();
+    assert!(harness.app.dialog.is_none());
+    let workspace = harness.app.workspace(tab).unwrap();
+    let object = workspace.object_tab(workspace.active_tab.unwrap()).unwrap();
+    assert_eq!(object.object.name, "authors");
+}
+
+/// A dot alone names no schema: it matches nothing.
+#[test]
+fn a_dot_alone_matches_nothing() {
+    let (harness, tab) = bookshop(Look::macos());
+    assert!(harness.app.search_objects(tab, ".").is_empty());
+    assert!(!harness.app.search_objects(tab, "main.").is_empty());
+}
+
+/// The pick moves from where it shows, after the list shrank under it.
+#[test]
+fn the_pick_moves_from_where_it_shows() {
+    let (mut harness, _tab) = bookshop(Look::macos());
+    ask(&mut harness, Look::macos(), "cov");
+    if let Some(Dialog::QuickOpen(open)) = &mut harness.app.dialog {
+        open.selected = 7;
+    }
+    let count = harness
+        .app
+        .quick_open_list(quick_open(&harness))
+        .items()
+        .count();
+    assert!(count < 7);
+    harness.press(Key::ArrowUp, Modifiers::NONE);
+    assert_eq!(quick_open(&harness).selected, count - 2);
+}
+
+/// Quick open goes when the window it was opened in closes.
+#[test]
+fn quick_open_closes_with_its_window() {
+    let (mut harness, dev) = bookshop(Look::macos());
+    harness.app.apply(Action::ShowConnections);
+    harness.connect_with(bookshop_production(), bookshop_objects());
+    harness.app.apply(Action::ActivateConnTab(dev));
+    ask(&mut harness, Look::macos(), "cov");
+    harness.app.apply(Action::CloseConnTab(dev));
+    harness.settle();
+    assert!(harness.app.dialog.is_none());
+}
+
+/// Omarchy's count of what the list hides takes the connections'
+/// headings into account: they take lines too.
+#[test]
+fn omarchy_counts_what_its_headings_push_out_of_view() {
+    let look = Look::omarchy();
+    let (mut harness, dev) = bookshop(look);
+    harness.app.apply(Action::ShowConnections);
+    harness.connect_with(bookshop_production(), bookshop_with_publishers());
+    harness.app.apply(Action::ActivateConnTab(dev));
+    harness.settle();
+    ask(&mut harness, look, "pub");
+    harness.press(Key::Tab, Modifiers::NONE);
+    harness.settle();
+    // Sixteen matches under two headings: the eight lines show a heading
+    // and seven of them, and nine are hidden.
+    assert!(
+        harness.painted_color("16 matches · 9 hidden").is_some(),
+        "{:?}",
+        harness
+            .painted
+            .iter()
+            .filter(|(text, _)| text.contains("matches"))
+            .collect::<Vec<_>>()
+    );
+}
+
+/// Test 12: with 10,000 objects in a schema, each keystroke's results are
+/// worked out within 16 ms.
+#[test]
+fn a_large_schema_answers_each_keystroke_within_a_frame() {
+    let words = [
+        "book",
+        "cover",
+        "edition",
+        "author",
+        "review",
+        "rating",
+        "publisher",
+    ];
+    let objects: Vec<ObjectInfo> = (0..10_000)
+        .map(|i| ObjectInfo {
+            name: format!(
+                "{}_{}_{i:05}",
+                words[i % words.len()],
+                words[(i / 7) % words.len()]
+            ),
+            kind: if i % 5 == 0 {
+                ObjectKind::View
+            } else {
+                ObjectKind::Table
+            },
+            estimated_rows: Some(i as u64),
+        })
+        .collect();
+    let mut harness = Harness::new();
+    harness.set_look(Look::macos());
+    let tab = harness.connect_with(bookshop_dev(), objects);
+    let mut open = QuickOpen {
+        tab,
+        query: String::new(),
+        selected: 0,
+        everywhere: false,
+        expanded: Vec::new(),
+        caret: None,
+        scrolled_to: None,
+        scroll_offset: 0.0,
+    };
+    // Each keystroke of a query, the fastest of five tries taken as its
+    // time, so that a pause of the machine's is not the search's.
+    let _ = harness.app.quick_open_list(&open);
+    for typed in [
+        "p", "pu", "pub", "pub_", "pub_c", "bc", "zz", "e", "o", "main.e",
+    ] {
+        open.query = typed.into();
+        let mut best = std::time::Duration::MAX;
+        for _ in 0..5 {
+            let started = std::time::Instant::now();
+            std::hint::black_box(harness.app.quick_open_list(&open));
+            best = best.min(started.elapsed());
+        }
+        assert!(
+            best < std::time::Duration::from_millis(16),
+            "{typed}: {best:?}"
+        );
+    }
+}
diff --git a/src/ui/start_tests.rs b/src/ui/start_tests.rs
index 445a54c..cb94965 100644
--- a/src/ui/start_tests.rs
+++ b/src/ui/start_tests.rs
@@ -561,7 +561,7 @@ fn the_search_groups_the_index_opens_the_first_and_in_an_editor_and_clears() {
     // Grouped Tables then Views; the index's order inside each group.
     assert!(harness.has("Tables") && harness.has("Views"));
     assert_eq!(
-        harness.app.start_matches(tab).first().unwrap().0.name,
+        harness.app.start_matches(tab).first().unwrap().object.name,
         "covers"
     );
     // ↩ opens the first match.
@@ -715,8 +715,8 @@ fn the_omarchy_slash_search_moves_opens_and_returns() {
     assert_eq!(harness.app.workspace(tab).unwrap().start_search, "cov");
     assert!(harness.has("matches"));
     // The first match is picked; ctrl+n moves to the second, enter opens it.
-    assert_eq!(harness.app.start_matches(tab)[0].0.name, "covers");
-    assert_eq!(harness.app.start_matches(tab)[1].0.name, "book_covers");
+    assert_eq!(harness.app.start_matches(tab)[0].object.name, "covers");
+    assert_eq!(harness.app.start_matches(tab)[1].object.name, "book_covers");
     harness.press(Key::N, Modifiers::CTRL);
     harness.press(Key::Enter, Modifiers::NONE);
     assert!(harness.has("book_covers tab"));
````

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --locked -p tabletist --lib -- quick_open`

Expected: the build fails, as the index, the list and the new actions do not exist yet. The kinds of error, most frequent first:

```
error[E0599]: no method named `quick_open_list` found for struct `app::App` in the current scope
error[E0599]: no method named `search_objects` found for struct `app::App` in the current scope
error[E0609]: no field `everywhere` on type `&QuickOpen`
error[E0559]: variant `model::Action::QuickOpenPick` has no field named `index`
error[E0559]: variant `model::Action::QuickOpenPick` has no field named `in_editor`
error[E0609]: no field `object` on type `(tabletist_db::ObjectRef, tabletist_db::ObjectKind)`
error[E0609]: no field `expanded` on type `&QuickOpen`
error[E0432]: unresolved imports `crate::app::QuickCommand`, `crate::app::QuickHeading`, `crate::app::QuickItem`, `crate::app::QuickRow`, `crate::app::StartRow`
error[E0432]: unresolved import `crate::model::QuickGroup`
error[E0433]: cannot find `QuickItem` in `app`
error[E0433]: cannot find `QuickCommand` in `app`
error[E0609]: no field `kind` on type `(tabletist_db::ObjectRef, tabletist_db::ObjectKind)`
… and 5 more of the same kinds
```

- [ ] **Step 3: Write the index, the list, the keys and the panel**

````diff
diff --git a/src/app.rs b/src/app.rs
index d185ba8..fc7d3f4 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -18,9 +18,9 @@ use crate::i18n::{Locale, gettext};
 use crate::model::{Action, ConnTab, ConnTabContent, ConnTabId, PickerState};
 use crate::model::{
     Advance, CellPos, Completion, ConnectionForm, Dialog, Fetch, FilterBar, FilterRow, Held,
-    HostKeyPrompt, LeavePrompt, ObjectTab, ObjectView, Pane, PasswordPrompt, PickTarget, QuickOpen,
-    Reach, ResultPane, RunMode, SecretKind, SessionStatus, SqlTab, StartRecent, Tab, TabId,
-    TestState, TextPrint, Tree, TreeKey, TreeNode, Wanted, Workspace,
+    HostKeyPrompt, LeavePrompt, ObjectTab, ObjectView, Pane, PasswordPrompt, PickTarget, Reach,
+    ResultPane, RunMode, SecretKind, SessionStatus, SqlTab, StartRecent, Tab, TabId, TestState,
+    TextPrint, Tree, TreeKey, TreeNode, Wanted, Workspace,
 };
 use crate::paths::AppDirs;
 use crate::secrets::{SecretString, password_account, ssh_account};
@@ -30,9 +30,14 @@ use crate::theme::{self, Catalog, Palette};
 mod editing;
 mod key_picker;
 mod labels;
+mod quick_open;
 #[cfg(test)]
 mod startup_tests;
 
+pub use quick_open::{
+    Found, GROUP_SHOWN, QuickCommand, QuickHeading, QuickItem, QuickList, QuickRow, StartRow,
+};
+
 /// What a keyring read is for; each names the exact request it serves, so
 /// a late answer for an earlier attempt is dropped.
 #[derive(Debug, Clone, Copy)]
@@ -78,14 +83,6 @@ pub enum PinState {
     Missing,
 }
 
-/// What quick open lists.
-#[derive(Debug, Clone, PartialEq, Eq)]
-pub enum Found {
-    /// The window's Agents tab.
-    Agents,
-    Object(ObjectRef, ObjectKind),
-}
-
 pub struct App {
     pub dirs: AppDirs,
     pub settings: Settings,
@@ -134,6 +131,10 @@ pub struct App {
     writes_session: bool,
     /// What was opened lately on each connection: `recents.json`.
     pub recents: crate::recents::Recents,
+    /// Each open connection's index of its objects, for quick open and the
+    /// start screen's search: built when first searched, and again once
+    /// the catalog changes.
+    indexes: quick_open::Indexes,
     pub dialog: Option<Dialog>,
     /// Keyring reads in flight, and what each is for.
     pending_secrets: HashMap<RequestId, SecretPurpose>,
@@ -320,6 +321,7 @@ impl App {
             session,
             writes_session,
             recents,
+            indexes: Default::default(),
             dialog: None,
             pending_secrets: HashMap::new(),
             pending_stores: HashMap::new(),
@@ -1432,9 +1434,10 @@ impl App {
                 // clamped, as the matches may have shrunk since it moved
                 // (a relist, a database switched under a typed query).
                 let picked = index.unwrap_or_else(|| selected.min(matches.len().saturating_sub(1)));
-                let Some((object, kind)) = matches.get(picked).cloned() else {
+                let Some(Found { object, kind, .. }) = matches.into_iter().nth(picked) else {
                     return;
                 };
+                let object = std::rc::Rc::unwrap_or_clone(object);
                 if let Some(workspace) = self.workspace_mut(tab) {
                     workspace.start_search.clear();
                     workspace.start_selected = 0;
@@ -2320,51 +2323,22 @@ impl App {
                     });
                 }
             }
-            Action::OpenQuickOpen => {
-                let tab = self.active_tab_id();
-                if self.dialog.is_none() && self.workspace(tab).is_some() {
-                    self.dialog = Some(Dialog::QuickOpen(Box::new(QuickOpen {
-                        tab,
-                        query: String::new(),
-                        selected: 0,
-                        scrolled_to: None,
-                        scroll_offset: 0.0,
-                    })));
-                }
-            }
-            Action::QuickOpenMove(step) => {
-                let (tab, query) = match &self.dialog {
-                    Some(Dialog::QuickOpen(open)) => (open.tab, open.query.clone()),
-                    _ => return,
-                };
-                let count = self.quick_open_found(tab, &query).len();
+            Action::OpenQuickOpen => self.open_quick_open(),
+            Action::QuickOpenTyped(text) => {
+                self.open_quick_open();
                 if let Some(Dialog::QuickOpen(open)) = &mut self.dialog {
-                    open.selected = (open.selected as isize + step)
-                        .clamp(0, count.saturating_sub(1) as isize)
-                        as usize;
+                    open.query.push_str(&text);
+                    open.caret = Some(crate::model::QuickCaret::End);
                 }
+                self.quick_open_changed();
             }
-            Action::QuickOpenPick => {
-                let Some(Dialog::QuickOpen(open)) = &self.dialog else {
-                    return;
-                };
-                let (tab, query, selected) = (open.tab, open.query.clone(), open.selected);
-                let found = self.quick_open_found(tab, &query);
-                match found.get(selected).or(found.first()).cloned() {
-                    Some(Found::Agents) => {
-                        self.dialog = None;
-                        self.apply(Action::ShowAgents(tab));
-                    }
-                    Some(Found::Object(object, kind)) => {
-                        self.dialog = None;
-                        self.apply(Action::OpenObject {
-                            tab,
-                            object,
-                            kind,
-                            pin: true,
-                        });
-                    }
-                    None => {}
+            Action::QuickOpenChanged => self.quick_open_changed(),
+            Action::QuickOpenMove(step) => self.quick_open_move(step),
+            Action::QuickOpenPick { index, in_editor } => self.quick_open_pick(index, in_editor),
+            Action::QuickOpenScope => self.quick_open_scope(),
+            Action::ShowDatabaseMenu(tab) => {
+                if let Some(workspace) = self.workspace_mut(tab) {
+                    workspace.database_menu_asked = true;
                 }
             }
             Action::ToggleFilterBar(tab) => {
@@ -4455,23 +4429,21 @@ impl App {
     }
 
     /// What the start screen's search lists for `tab`, in the order it
-    /// draws them: the `⌘P` index's matches (`quick_open_matches`, the
-    /// same index and ranking), grouped with the tables first and the
+    /// draws them: quick open's matches (`search_objects`, the same index
+    /// and ranking), the best 50, grouped with the tables first and the
     /// views after, each group in the index's order. Empty where nothing
     /// is typed. Materialized views go with the views. The picked match
     /// ([`Workspace::start_selected`]) is an index into this.
-    pub fn start_matches(&self, tab: ConnTabId) -> Vec<(ObjectRef, ObjectKind)> {
+    pub fn start_matches(&self, tab: ConnTabId) -> Vec<Found> {
         let query = self
             .workspace(tab)
             .map(|workspace| workspace.start_search.trim())
             .unwrap_or_default();
-        if query.is_empty() {
-            return Vec::new();
-        }
-        let matched = self.quick_open_matches(tab, query);
+        let mut matched = self.search_objects(tab, query);
+        matched.truncate(50);
         let (tables, views): (Vec<_>, Vec<_>) = matched
             .into_iter()
-            .partition(|(_, kind)| *kind == ObjectKind::Table);
+            .partition(|found| found.kind == ObjectKind::Table);
         tables.into_iter().chain(views).collect()
     }
 
@@ -6602,52 +6574,6 @@ impl App {
         }
     }
 
-    /// What quick open lists for `query`: the Agents tab first where what
-    /// is typed is the start of "agents", two letters at least, and agents
-    /// can reach the app; then the objects. Fewer letters, or letters
-    /// that are only somewhere in the word, are a table's.
-    pub fn quick_open_found(&self, tab: ConnTabId, query: &str) -> Vec<Found> {
-        let typed = query.trim().to_lowercase();
-        let agents = crate::endpoint::AVAILABLE && typed.len() >= 2 && "agents".starts_with(&typed);
-        let objects = self.quick_open_matches(tab, query).into_iter();
-        agents
-            .then_some(Found::Agents)
-            .into_iter()
-            .chain(objects.map(|(object, kind)| Found::Object(object, kind)))
-            .collect()
-    }
-
-    /// Loaded objects of `tab`'s connection that match `query`, best first,
-    /// at most 50.
-    pub fn quick_open_matches(&self, tab: ConnTabId, query: &str) -> Vec<(ObjectRef, ObjectKind)> {
-        let Some(workspace) = self.workspace(tab) else {
-            return Vec::new();
-        };
-        let mut scored: Vec<(i64, ObjectRef, ObjectKind)> = workspace
-            .tree
-            .nodes
-            .iter()
-            .filter_map(|(schema, node)| Some((schema, node.objects.value.as_ref()?)))
-            .flat_map(|(schema, objects)| objects.iter().map(move |info| (schema, info)))
-            .filter_map(|(schema, info)| {
-                let qualified = format!("{schema}.{}", info.name);
-                let score = crate::util::fuzzy_score(query, &info.name)
-                    .max(crate::util::fuzzy_score(query, &qualified))?;
-                Some((score, ObjectRef::new(schema, &info.name), info.kind))
-            })
-            .collect();
-        scored.sort_by(|a, b| {
-            b.0.cmp(&a.0)
-                .then_with(|| a.1.schema.cmp(&b.1.schema))
-                .then_with(|| a.1.name.cmp(&b.1.name))
-        });
-        scored.truncate(50);
-        scored
-            .into_iter()
-            .map(|(_, object, kind)| (object, kind))
-            .collect()
-    }
-
     /// Runs the object tab's query with its filter bar's conditions, from
     /// the first page.
     fn apply_filters(&mut self, tab: ConnTabId, id: TabId) {
diff --git a/src/app/quick_open.rs b/src/app/quick_open.rs
new file mode 100644
index 0000000..07b2217
--- /dev/null
+++ b/src/app/quick_open.rs
@@ -0,0 +1,628 @@
+//! The reducer's part in quick open, and the search of a connection's
+//! objects that it shares with the start screen: what matches what is
+//! typed, in what order, what quick open lists, and what its actions do.
+//! How one name matches is `crate::search`'s.
+
+use std::cell::RefCell;
+use std::collections::HashMap;
+use std::ops::Range;
+use std::rc::Rc;
+
+use tabletist_db::{ObjectKind, ObjectRef};
+
+use super::App;
+use crate::model::{Action, ConnTabId, Dialog, QuickGroup, QuickOpen, RunMode, StartRecent};
+use crate::search::{self, Tier};
+
+/// How many results a group shows before its "more" row (macOS).
+pub const GROUP_SHOWN: usize = 3;
+
+/// A table or view that matches what is typed.
+#[derive(Debug, Clone, PartialEq, Eq)]
+pub struct Found {
+    /// The index's own: a search that matches thousands copies no name.
+    pub object: Rc<ObjectRef>,
+    pub kind: ObjectKind,
+    /// The catalog's estimate of its rows, where it has one.
+    pub estimate: Option<u64>,
+    pub tier: Tier,
+    /// The runs of the name that matched, as byte ranges of
+    /// `object.name`.
+    pub spans: Vec<Range<usize>>,
+}
+
+/// The index of a connection's objects: each with its name taken apart for
+/// matching once, in order by name and then schema (the last of the
+/// ranking's ties).
+pub(super) struct Index {
+    entries: Vec<Entry>,
+    /// The schemas listed: a query's schema part is matched once each.
+    schemas: Vec<String>,
+}
+
+struct Entry {
+    object: Rc<ObjectRef>,
+    /// Its schema's place in `Index::schemas`.
+    schema: usize,
+    kind: ObjectKind,
+    estimate: Option<u64>,
+    name: search::Name,
+}
+
+impl Index {
+    fn of(workspace: &crate::model::Workspace) -> Self {
+        let listed: Vec<(&String, &Vec<tabletist_db::ObjectInfo>)> = workspace
+            .tree
+            .nodes
+            .iter()
+            .filter_map(|(schema, node)| Some((schema, node.objects.value.as_ref()?)))
+            .collect();
+        let schemas = listed.iter().map(|(schema, _)| (*schema).clone()).collect();
+        let mut entries: Vec<Entry> = listed
+            .into_iter()
+            .enumerate()
+            .flat_map(|(at, (schema, objects))| {
+                objects.iter().map(move |info| Entry {
+                    object: Rc::new(ObjectRef::new(schema, &info.name)),
+                    schema: at,
+                    kind: info.kind,
+                    estimate: info.estimated_rows,
+                    name: search::Name::new(&info.name),
+                })
+            })
+            .collect();
+        entries.sort_by(|a, b| {
+            (a.object.name.as_str(), a.object.schema.as_str())
+                .cmp(&(b.object.name.as_str(), b.object.schema.as_str()))
+        });
+        Self { entries, schemas }
+    }
+
+    /// What says the index is still the catalog's: its generation, and
+    /// each schema's list as it stands (a list read again is a list of its
+    /// own).
+    fn key(workspace: &crate::model::Workspace) -> u64 {
+        use std::hash::{Hash, Hasher};
+        let mut key = workspace.catalog_generation;
+        for (schema, node) in &workspace.tree.nodes {
+            let mut hasher = std::collections::hash_map::DefaultHasher::new();
+            schema.hash(&mut hasher);
+            if let Some(objects) = &node.objects.value {
+                (objects.as_ptr() as usize, objects.len()).hash(&mut hasher);
+            }
+            // In whatever order the schemas come.
+            key = key.wrapping_add(hasher.finish());
+        }
+        key
+    }
+}
+
+/// A match with what it ranks by, packed in one number so that the sort
+/// compares numbers alone: its tier (best first), its place among the
+/// recents, its length, and its place in the index, which is by name.
+struct Ranked {
+    key: u64,
+    tier: Tier,
+    at: usize,
+    spans: Vec<Range<usize>>,
+}
+
+impl Ranked {
+    fn new(tier: Tier, rank: usize, len: usize, at: usize, spans: Vec<Range<usize>>) -> Self {
+        let worse = 3 - tier as u64;
+        let rank = rank.min(0xff) as u64;
+        let len = len.min(0x3f_ffff) as u64;
+        let key = worse << 62 | rank << 54 | len << 32 | (at as u64 & 0xffff_ffff);
+        Self {
+            key,
+            tier,
+            at,
+            spans,
+        }
+    }
+}
+
+/// The index of each open connection, as last built.
+#[derive(Default)]
+pub struct Indexes(RefCell<HashMap<ConnTabId, (u64, Rc<Index>)>>);
+
+/// A row of a window's start screen list, as quick open shows it with
+/// nothing typed: a recent, or on a first visit one of the largest tables.
+#[derive(Debug, Clone, PartialEq)]
+pub enum StartRow {
+    Recent(StartRecent),
+    Largest(ObjectRef, Option<u64>),
+}
+
+/// One of the commands quick open offers in the macOS and Windows looks.
+#[derive(Debug, Clone, Copy, PartialEq, Eq)]
+pub enum QuickCommand {
+    /// A new SQL editor. `read_only` where a production connection's new
+    /// editors open read-only, which the row says.
+    NewSql { read_only: bool },
+    /// The menu of the server's other databases. Offered only where there
+    /// is another.
+    SwitchDatabase,
+    /// The window's Agents tab. Offered only where agents can reach the
+    /// app.
+    Agents,
+}
+
+impl QuickCommand {
+    /// The command's name, as the row writes it and what is typed
+    /// matches.
+    pub fn name(self) -> &'static str {
+        match self {
+            Self::NewSql { .. } => "New SQL query",
+            Self::SwitchDatabase => "Switch database",
+            Self::Agents => "Agents",
+        }
+    }
+
+    /// The keymap's command it is, whose key the row writes.
+    pub fn command(self) -> crate::keymap::Command {
+        use crate::keymap::Command;
+        match self {
+            Self::NewSql { .. } => Command::NewSqlTab,
+            Self::SwitchDatabase => Command::SwitchDatabase,
+            Self::Agents => Command::ShowAgents,
+        }
+    }
+}
+
+/// One of quick open's results: what can be picked.
+#[derive(Debug, Clone, PartialEq)]
+pub enum QuickItem {
+    /// A table or view of the window `tab`'s connection.
+    Object { tab: ConnTabId, found: Found },
+    /// The row of `tab`'s start screen list numbered `index` there.
+    Start {
+        tab: ConnTabId,
+        index: usize,
+        row: StartRow,
+    },
+    /// A command, and the runs of its name that matched.
+    Command {
+        command: QuickCommand,
+        spans: Vec<Range<usize>>,
+    },
+    /// The row after a group's first results: the rest are `hidden`.
+    More { group: QuickGroup, hidden: usize },
+}
+
+/// What heads a group of quick open's results.
+#[derive(Debug, Clone, Copy, PartialEq, Eq)]
+pub enum QuickHeading {
+    Recent,
+    /// The start screen's first visit lists the largest tables.
+    Largest,
+    Tables,
+    Views,
+    Commands,
+    /// One connection's results, where every open one is searched.
+    Connection(ConnTabId),
+}
+
+#[derive(Debug, Clone, PartialEq)]
+pub enum QuickRow {
+    /// A heading: a label, not a result.
+    Heading(QuickHeading),
+    Item(QuickItem),
+}
+
+/// What quick open lists: its rows in order, and how many objects
+/// matched (more than are listed where a group is folded).
+#[derive(Debug, Clone, Default, PartialEq)]
+pub struct QuickList {
+    pub rows: Vec<QuickRow>,
+    pub matches: usize,
+}
+
+impl QuickList {
+    /// The results, in order: what the pick is an index into.
+    pub fn items(&self) -> impl Iterator<Item = &QuickItem> {
+        self.rows.iter().filter_map(|row| match row {
+            QuickRow::Item(item) => Some(item),
+            QuickRow::Heading(_) => None,
+        })
+    }
+
+    /// Whether nothing at all is listed.
+    pub fn is_empty(&self) -> bool {
+        self.rows.is_empty()
+    }
+
+    /// A group's results: the first [`GROUP_SHOWN`] and a "more" row,
+    /// unless `whole`.
+    fn group(&mut self, group: QuickGroup, items: Vec<QuickItem>, whole: bool) {
+        let count = items.len();
+        let shown = if whole { count } else { count.min(GROUP_SHOWN) };
+        self.rows
+            .extend(items.into_iter().take(shown).map(QuickRow::Item));
+        if shown < count {
+            let hidden = count - shown;
+            self.rows
+                .push(QuickRow::Item(QuickItem::More { group, hidden }));
+        }
+    }
+}
+
+impl App {
+    /// The index of `tab`'s connection: what the sidebar has listed, which
+    /// the SQL editor's completion reads too, each name taken apart once.
+    /// Built anew when the catalog changes, and kept until it does.
+    fn index(&self, tab: ConnTabId) -> Option<Rc<Index>> {
+        let workspace = self.workspace(tab)?;
+        let key = Index::key(workspace);
+        let mut kept = self.indexes.0.borrow_mut();
+        if let Some((built, index)) = kept.get(&tab)
+            && *built == key
+        {
+            return Some(index.clone());
+        }
+        let index = Rc::new(Index::of(workspace));
+        kept.insert(tab, (key, index.clone()));
+        // The windows closed since keep none.
+        kept.retain(|tab, _| self.workspace(*tab).is_some());
+        Some(index)
+    }
+
+    /// The tables, views and materialized views of `tab`'s connection that
+    /// match `query`, best first: by tier, then the recently opened ones
+    /// (the newest first), then the shorter names, then by name and
+    /// schema. From the connection's index: nothing is queried. A query
+    /// with a `.` in it is a schema and a name: what stands before the dot
+    /// matches the schema, in any tier, and what stands after it the name,
+    /// whose tier ranks it (`main.` lists the schema's objects, in the
+    /// schema's tier). Empty where nothing is typed, or only a dot.
+    pub fn search_objects(&self, tab: ConnTabId, query: &str) -> Vec<Found> {
+        let (Some(workspace), Some(index)) = (self.workspace(tab), self.index(tab)) else {
+            return Vec::new();
+        };
+        let typed = search::folded(query);
+        if typed.is_empty() {
+            return Vec::new();
+        }
+        let (schema_typed, name_typed) = match typed.iter().position(|char| *char == '.') {
+            Some(at) => (&typed[..at], &typed[at + 1..]),
+            None => (&typed[..0], &typed[..]),
+        };
+        if name_typed.is_empty() && schema_typed.is_empty() {
+            return Vec::new();
+        }
+        // Each schema's match, worked out once: a few schemas hold many
+        // objects.
+        let schemas: Vec<Option<Tier>> = if schema_typed.is_empty() {
+            Vec::new()
+        } else {
+            let tier = |schema: &String| search::find(schema_typed, schema).map(|hit| hit.tier);
+            index.schemas.iter().map(tier).collect()
+        };
+        let database = crate::session::database_of(&workspace.spec);
+        let recent: Vec<ObjectRef> = self
+            .recents
+            .of(&workspace.conn_id.0, &database)
+            .into_iter()
+            .map(crate::recents::Recent::object)
+            .collect();
+        let mut hits: Vec<Ranked> = Vec::new();
+        for (at, entry) in index.entries.iter().enumerate() {
+            let schema = if schema_typed.is_empty() {
+                None
+            } else {
+                let Some(tier) = schemas[entry.schema] else {
+                    continue;
+                };
+                Some(tier)
+            };
+            let hit = if name_typed.is_empty() {
+                schema.map(|tier| search::Hit {
+                    tier,
+                    spans: Vec::new(),
+                })
+            } else {
+                entry.name.find(name_typed)
+            };
+            let Some(hit) = hit else {
+                continue;
+            };
+            let rank = recent
+                .iter()
+                .position(|object| *object == *entry.object)
+                .unwrap_or(usize::MAX);
+            hits.push(Ranked::new(hit.tier, rank, entry.name.len(), at, hit.spans));
+        }
+        hits.sort_unstable_by_key(|hit| hit.key);
+        hits.into_iter()
+            .map(|hit| {
+                let entry = &index.entries[hit.at];
+                Found {
+                    object: entry.object.clone(),
+                    kind: entry.kind,
+                    estimate: entry.estimate,
+                    tier: hit.tier,
+                    spans: hit.spans,
+                }
+            })
+            .collect()
+    }
+
+    /// What `tab`'s start screen lists with nothing typed, as quick open
+    /// shows it: the recents, or on a first visit the largest tables, and
+    /// the heading that says which.
+    fn start_rows(&self, tab: ConnTabId) -> (QuickHeading, Vec<StartRow>) {
+        let recents = self.start_recents(tab);
+        if recents.is_empty() {
+            let largest = self.start_largest(tab).into_iter();
+            let rows = largest.map(|(object, estimate)| StartRow::Largest(object, estimate));
+            (QuickHeading::Largest, rows.collect())
+        } else {
+            let rows = recents.into_iter().map(StartRow::Recent);
+            (QuickHeading::Recent, rows.collect())
+        }
+    }
+
+    /// The windows quick open searches: its own, then where every open
+    /// connection is searched, the others in the order the bar shows them.
+    pub fn quick_open_windows(&self, open: &QuickOpen) -> Vec<ConnTabId> {
+        let mut windows = vec![open.tab];
+        if open.everywhere {
+            let others = self.open_connections().map(|(tab, _)| tab);
+            windows.extend(others.filter(|tab| *tab != open.tab));
+        }
+        windows
+    }
+
+    /// The commands quick open offers in the macOS and Windows looks, in
+    /// order, those that can do something now: none in the terminal look,
+    /// whose commands are on `:`.
+    fn quick_commands(&self, tab: ConnTabId) -> Vec<QuickCommand> {
+        let Some(workspace) = self.workspace(tab) else {
+            return Vec::new();
+        };
+        if self.look.terminal {
+            return Vec::new();
+        }
+        let read_only = workspace.environment.is_production()
+            && (self.settings.new_tab == crate::settings::NewTab::ReadOnly
+                || RunMode::of_new_tab(workspace.sql_writes(), workspace.environment)
+                    == RunMode::ReadOnly);
+        let switchable = workspace
+            .databases
+            .value
+            .as_ref()
+            .is_some_and(|databases| databases.len() > 1);
+        let mut commands = vec![QuickCommand::NewSql { read_only }];
+        if switchable {
+            commands.push(QuickCommand::SwitchDatabase);
+        }
+        if crate::endpoint::AVAILABLE {
+            commands.push(QuickCommand::Agents);
+        }
+        commands
+    }
+
+    /// What quick open lists for `open`. With nothing typed: each searched
+    /// window's start screen list (headed by its connection where every
+    /// open one is searched), then the commands. With a query: the tables
+    /// and then the views that match, or where every open connection is
+    /// searched, each connection's matches under it; then the commands
+    /// whose names match. In the macOS and Windows looks each group shows
+    /// its first three and a "more" row until that is opened; the terminal
+    /// look lists every match, in one list with no kind headings, and no
+    /// commands.
+    pub fn quick_open_list(&self, open: &QuickOpen) -> QuickList {
+        let terminal = self.look.terminal;
+        let query = open.query.trim();
+        let mut list = QuickList::default();
+        let whole = |group| terminal || open.expanded.contains(&group);
+        for tab in self.quick_open_windows(open) {
+            if query.is_empty() {
+                let (heading, rows) = self.start_rows(tab);
+                if rows.is_empty() {
+                    continue;
+                }
+                let heading = if open.everywhere {
+                    QuickHeading::Connection(tab)
+                } else {
+                    heading
+                };
+                list.rows.push(QuickRow::Heading(heading));
+                let rows = rows.into_iter().enumerate();
+                list.rows.extend(
+                    rows.map(|(index, row)| QuickRow::Item(QuickItem::Start { tab, index, row })),
+                );
+                continue;
+            }
+            let found = self.search_objects(tab, query);
+            list.matches += found.len();
+            let item = |found| QuickItem::Object { tab, found };
+            if open.everywhere {
+                if found.is_empty() {
+                    continue;
+                }
+                list.rows
+                    .push(QuickRow::Heading(QuickHeading::Connection(tab)));
+                let group = QuickGroup::Connection(tab);
+                list.group(group, found.into_iter().map(item).collect(), whole(group));
+                continue;
+            }
+            let (tables, views): (Vec<Found>, Vec<Found>) = found
+                .into_iter()
+                .partition(|found| found.kind == ObjectKind::Table);
+            let groups = [
+                (QuickGroup::Tables, QuickHeading::Tables, tables),
+                (QuickGroup::Views, QuickHeading::Views, views),
+            ];
+            for (group, heading, found) in groups {
+                if found.is_empty() {
+                    continue;
+                }
+                if !terminal {
+                    list.rows.push(QuickRow::Heading(heading));
+                }
+                list.group(group, found.into_iter().map(item).collect(), whole(group));
+            }
+        }
+        let typed = search::folded(query);
+        let mut commands: Vec<(Tier, QuickItem)> = self
+            .quick_commands(open.tab)
+            .into_iter()
+            .filter_map(|command| {
+                let name = crate::i18n::gettext(self.locale, command.name());
+                if typed.is_empty() {
+                    let spans = Vec::new();
+                    return Some((Tier::Exact, QuickItem::Command { command, spans }));
+                }
+                let hit = search::find(&typed, &name)?;
+                let spans = hit.spans;
+                Some((hit.tier, QuickItem::Command { command, spans }))
+            })
+            .collect();
+        // Best first; a sort that keeps their order within a tier.
+        commands.sort_by_key(|(tier, _)| std::cmp::Reverse(*tier));
+        if !commands.is_empty() {
+            list.rows.push(QuickRow::Heading(QuickHeading::Commands));
+            list.rows
+                .extend(commands.into_iter().map(|(_, item)| QuickRow::Item(item)));
+        }
+        list
+    }
+
+    /// `OpenQuickOpen`: quick open for the window in front, searching its
+    /// connection, where it is a connection's and no dialog is open. While
+    /// quick open is open, its field's text is selected instead: one panel
+    /// per window.
+    pub(super) fn open_quick_open(&mut self) {
+        if let Some(Dialog::QuickOpen(open)) = &mut self.dialog {
+            open.caret = Some(crate::model::QuickCaret::All);
+            return;
+        }
+        let tab = self.active_tab_id();
+        if self.dialog.is_none() && self.workspace(tab).is_some() {
+            self.dialog = Some(Dialog::QuickOpen(Box::new(QuickOpen {
+                tab,
+                query: String::new(),
+                selected: 0,
+                everywhere: false,
+                expanded: Vec::new(),
+                caret: None,
+                scrolled_to: None,
+                scroll_offset: 0.0,
+            })));
+        }
+    }
+
+    /// The quick open that is open, if one is.
+    fn quick_open_mut(&mut self) -> Option<&mut QuickOpen> {
+        match &mut self.dialog {
+            Some(Dialog::QuickOpen(open)) => Some(open),
+            _ => None,
+        }
+    }
+
+    /// `QuickOpenChanged`: what is typed changed.
+    pub(super) fn quick_open_changed(&mut self) {
+        if let Some(open) = self.quick_open_mut() {
+            open.selected = 0;
+            open.expanded.clear();
+        }
+    }
+
+    /// `QuickOpenScope`: this connection, or every open one.
+    pub(super) fn quick_open_scope(&mut self) {
+        if let Some(open) = self.quick_open_mut() {
+            open.everywhere = !open.everywhere;
+            open.selected = 0;
+            open.expanded.clear();
+        }
+    }
+
+    /// `QuickOpenMove`: the pick moves by `step`, and stops at the ends.
+    pub(super) fn quick_open_move(&mut self, step: isize) {
+        let count = match &self.dialog {
+            Some(Dialog::QuickOpen(open)) => self.quick_open_list(open).items().count(),
+            _ => return,
+        };
+        if let Some(open) = self.quick_open_mut() {
+            // From the pick as it shows: clamped, as the list may have
+            // shrunk under it (a relist, a window closed).
+            let last = count.saturating_sub(1);
+            let from = open.selected.min(last) as isize;
+            open.selected = (from + step).clamp(0, last as isize) as usize;
+        }
+    }
+
+    /// `QuickOpenPick`: opens the picked result, or the one at `index`.
+    /// An object opens as a tab, or the tab it has is shown, in its own
+    /// connection's window, which comes to the front; `in_editor` opens a
+    /// new SQL editor reading its first rows instead. A start screen row
+    /// opens as the start screen opens it. A command does what it does in
+    /// the window quick open was opened in. A "more" row shows its group
+    /// in full and quick open stays; the pick stays where it was, on the
+    /// first result that row hid. Anything else closes quick open.
+    pub(super) fn quick_open_pick(&mut self, index: Option<usize>, in_editor: bool) {
+        let (here, item) = match &self.dialog {
+            Some(Dialog::QuickOpen(open)) => {
+                let list = self.quick_open_list(open);
+                let count = list.items().count();
+                // The pick is clamped: the list may have shrunk since it
+                // moved (a relist under a typed query).
+                let picked = index.unwrap_or(open.selected.min(count.saturating_sub(1)));
+                (open.tab, list.items().nth(picked).cloned())
+            }
+            _ => return,
+        };
+        let Some(item) = item else {
+            return;
+        };
+        if let QuickItem::More { group, .. } = item {
+            if let Some(open) = self.quick_open_mut() {
+                open.expanded.push(group);
+                if let Some(index) = index {
+                    open.selected = index;
+                }
+            }
+            return;
+        }
+        self.dialog = None;
+        let front = |app: &mut App, tab: ConnTabId| {
+            if tab != here {
+                app.apply(Action::ActivateConnTab(tab));
+            }
+        };
+        match item {
+            QuickItem::Object { tab, found } => {
+                front(self, tab);
+                if in_editor {
+                    self.open_sql_for(tab, &found.object);
+                } else {
+                    self.apply(Action::OpenObject {
+                        tab,
+                        object: Rc::unwrap_or_clone(found.object),
+                        kind: found.kind,
+                        pin: true,
+                    });
+                }
+            }
+            QuickItem::Start { tab, index, row } => {
+                front(self, tab);
+                let object = match row {
+                    StartRow::Recent(StartRecent::Object { object, .. })
+                    | StartRow::Largest(object, _) => Some(object),
+                    StartRow::Recent(StartRecent::Sql { .. }) => None,
+                };
+                match object {
+                    Some(object) if in_editor => self.open_sql_for(tab, &object),
+                    _ => self.apply(Action::OpenRecent { tab, index }),
+                }
+            }
+            QuickItem::Command { command, .. } => self.apply(match command {
+                QuickCommand::NewSql { .. } => Action::NewSqlTab(here),
+                QuickCommand::SwitchDatabase => Action::ShowDatabaseMenu(here),
+                QuickCommand::Agents => Action::ShowAgents(here),
+            }),
+            QuickItem::More { .. } => {}
+        }
+    }
+}
diff --git a/src/keymap.rs b/src/keymap.rs
index 99fd455..8541203 100644
--- a/src/keymap.rs
+++ b/src/keymap.rs
@@ -351,6 +351,7 @@ commands! {
     FilterConnections,
     // Navigation.
     FindObject,
+    FilterObjects,
     NewSqlTab,
     ShowAgents,
     CopyAgentCommand,
@@ -531,6 +532,9 @@ commands! {
     SettingsCopyCommand,
     QuickOpenMove,
     QuickOpenPick,
+    QuickOpenPickInEditor,
+    QuickOpenScope,
+    QuickOpenNumber,
     QuickOpenClose,
     NextLabel,
     PreviousLabel,
@@ -617,6 +621,7 @@ impl Command {
             C::MoveInConnections => info("Move in the connections", "", Window),
             C::FilterConnections => info("Filter the connections", "", Window),
             C::FindObject => info("Find any object", "Find any object", Navigation),
+            C::FilterObjects => info("Filter the sidebar's objects", "", Navigation),
             C::NewSqlTab => info("New SQL editor tab", "New SQL editor tab", Navigation),
             // Nothing of agents answers where they cannot reach the app.
             C::ShowAgents => agents(info("Show the Agents tab", "", Navigation)),
@@ -874,6 +879,9 @@ impl Command {
             )),
             C::QuickOpenMove => info("Move in quick open", "", Prompts),
             C::QuickOpenPick => info("Open the one selected", "", Prompts),
+            C::QuickOpenPickInEditor => info("Open the one selected in a SQL editor", "", Prompts),
+            C::QuickOpenScope => info("Search this connection or every open one", "", Prompts),
+            C::QuickOpenNumber => info("Open a recent by its number", "", Prompts),
             C::QuickOpenClose => info("Close quick open", "", Prompts),
             C::NextLabel => info("Next line of the label chooser", "", Prompts),
             C::PreviousLabel => info("Previous line of the label chooser", "", Prompts),
@@ -1086,8 +1094,15 @@ pub const BINDINGS: &[Binding] = {
         ),
         bind(C::FilterConnections, CONNECTIONS, NONE, keys(&[k("/")])),
         // Navigation.
-        bind(C::FindObject, GLOBAL, keys(&[k("cmd+p")]), NONE),
-        bind(C::FindObject, SIDEBAR, NONE, keys(&[k("/")])),
+        // Quick open, from anywhere in a connection's window. The Omarchy
+        // `/` filters the sidebar's tree where it is.
+        bind(
+            C::FindObject,
+            GLOBAL,
+            keys(&[k("cmd+p")]),
+            keys(&[k("ctrl+p")]),
+        ),
+        bind(C::FilterObjects, SIDEBAR, NONE, keys(&[k("/")])),
         bind(
             C::NewSqlTab,
             GLOBAL,
@@ -1905,11 +1920,15 @@ pub const BINDINGS: &[Binding] = {
             keys(&[k("esc")]),
         )
         .when(When::SettingsScreen),
+        // Quick open: its field has the keyboard. Omarchy's digits pick a
+        // recent while nothing is typed; once anything is, they are typed
+        // (names hold digits), and quick open reads them as text. Its
+        // moves list the keys that go up first, then those that go down.
         bind(
             C::QuickOpenMove,
             PROMPT,
-            keys(&[k("up"), k("down")]),
-            keys(&[k("up"), k("down")]),
+            keys(&[k("up"), k("down")]).shown("↑↓"),
+            keys(&[k("ctrl+p"), k("up"), k("ctrl+n"), k("down")]).shown("ctrl+n/p"),
         )
         .when(When::QuickOpen),
         bind(
@@ -1919,6 +1938,21 @@ pub const BINDINGS: &[Binding] = {
             keys(&[k("enter")]),
         )
         .when(When::QuickOpen),
+        bind(
+            C::QuickOpenPickInEditor,
+            PROMPT,
+            keys(&[k("alt+enter")]),
+            NONE,
+        )
+        .when(When::QuickOpen),
+        bind(
+            C::QuickOpenScope,
+            PROMPT,
+            keys(&[k("tab")]),
+            keys(&[k("tab")]),
+        )
+        .when(When::QuickOpen),
+        bind(C::QuickOpenNumber, PROMPT, NONE, keys(&[k("1..9")])).when(When::QuickOpen),
         bind(
             C::QuickOpenClose,
             PROMPT,
@@ -3217,7 +3251,10 @@ mod tests {
         assert_eq!(omarchy(Scope::Grid, "R"), command(Command::Reload));
         assert_eq!(omarchy(Scope::Sidebar, "R"), command(Command::Reload));
         // The slash is the sidebar's filter there and the WHERE line here.
-        assert_eq!(omarchy(Scope::Sidebar, "/"), command(Command::FindObject));
+        assert_eq!(
+            omarchy(Scope::Sidebar, "/"),
+            command(Command::FilterObjects)
+        );
         assert_eq!(omarchy(Scope::Grid, "/"), command(Command::WhereFilter));
         // A global letter is read in every scope but a prompt's.
         assert_eq!(omarchy(Scope::Grid, "?"), command(Command::Help));
diff --git a/src/model.rs b/src/model.rs
index a714a34..22f2bc2 100644
--- a/src/model.rs
+++ b/src/model.rs
@@ -886,12 +886,30 @@ pub enum Action {
     ResetSettings,
     /// The answer: put them back, or leave them.
     ConfirmResetSettings(bool),
-    /// Open quick open for the active connection tab.
+    /// Open quick open for the active connection tab, its scope that
+    /// connection. While it is open, select what its field holds.
     OpenQuickOpen,
-    /// Move quick open's selection by this many results.
+    /// Open quick open as `OpenQuickOpen` does, with this typed in its
+    /// field: what was typed on the sidebar's field, which is its way in.
+    QuickOpenTyped(String),
+    /// Quick open's field changed: pick the first result again, and fold
+    /// the groups opened in full.
+    QuickOpenChanged,
+    /// Move quick open's pick by this many results.
     QuickOpenMove(isize),
-    /// Open (and pin) quick open's selected result.
-    QuickOpenPick,
+    /// Open quick open's picked result, or the result at `index` where a
+    /// row was clicked or its number typed; `in_editor` opens it in a new
+    /// SQL editor (`⌥↩`). A group's "more" row shows that group in full,
+    /// and quick open stays.
+    QuickOpenPick {
+        index: Option<usize>,
+        in_editor: bool,
+    },
+    /// Switch quick open between this connection and every open one.
+    QuickOpenScope,
+    /// Open the menu of the server's other databases under the connection
+    /// bar's chip, as Switch database's key does.
+    ShowDatabaseMenu(ConnTabId),
     /// Put keyboard focus back into the open filter bar.
     FocusFilterBar(ConnTabId),
     /// Open or close the active object tab's filter bar.
@@ -1031,6 +1049,9 @@ pub struct Workspace {
     /// frame, so the screen's own keys answer again: when `esc` clears
     /// the search in the Omarchy look.
     pub blur_start_search: bool,
+    /// Open the menu of the server's other databases under the connection
+    /// bar's chip on the next frame: quick open's Switch database asked.
+    pub database_menu_asked: bool,
     pub sidebar_hidden: bool,
     /// The grid shows timestamps in full rather than to the second.
     pub full_precision: bool,
@@ -2537,13 +2558,28 @@ impl std::fmt::Debug for ConflictPrompt {
     }
 }
 
-/// Cmd/Ctrl+P: find a loaded table or view by name.
+/// Quick open (`⌘P`, `ctrl+p`): find a table or view of the connection, or
+/// of every open one, by name, or (macOS) start one of a few commands.
+/// What it lists is [`crate::app::App::quick_open_list`]'s.
 #[derive(Debug)]
 pub struct QuickOpen {
+    /// The window it was opened in: the connection searched, and where
+    /// its commands act.
     pub tab: ConnTabId,
+    /// What the field holds: the field edits it.
     pub query: String,
-    /// Index into the current matches.
+    /// The picked result, an index into the list's results (headings are
+    /// none). Clamped as it moves; reset when what is typed changes.
     pub selected: usize,
+    /// Whether every open connection is searched (`⇥`), not only this
+    /// one. Each open starts with this one.
+    pub everywhere: bool,
+    /// The groups shown in full: their "more" row was opened.
+    pub expanded: Vec<QuickGroup>,
+    /// Where the field's caret goes on the next frame, where anything
+    /// moves it: `⌘P` again while open selects the text, and what was
+    /// typed on the sidebar's field leaves the caret after it.
+    pub caret: Option<QuickCaret>,
     /// The selection the list last scrolled to, so it scrolls only when the
     /// selection moves and a wheel scroll is left alone.
     pub scrolled_to: Option<usize>,
@@ -2551,6 +2587,27 @@ pub struct QuickOpen {
     pub scroll_offset: f32,
 }
 
+/// Where quick open moves its field's caret.
+#[derive(Debug, Clone, Copy, PartialEq, Eq)]
+pub enum QuickCaret {
+    /// Select the whole text.
+    All,
+    /// After the text.
+    End,
+}
+
+/// A group of quick open's results, which shows its first three until its
+/// "more" row opens it in full: a kind of object of this connection, or
+/// one connection's objects where every open one is searched.
+#[derive(Debug, Clone, Copy, PartialEq, Eq)]
+pub enum QuickGroup {
+    Tables,
+    /// Views and materialized views.
+    Views,
+    /// The objects of the window `ConnTabId`, of every kind.
+    Connection(ConnTabId),
+}
+
 /// The Settings window while it is open. The settings themselves are the
 /// app's: every change applies at once, so the window holds nothing of
 /// them.
@@ -4787,6 +4844,7 @@ impl Workspace {
             start_selected: 0,
             focus_start_search: false,
             blur_start_search: false,
+            database_menu_asked: false,
             sidebar_hidden: false,
             full_precision: false,
             zone: None,
diff --git a/src/ui/cell_editor.rs b/src/ui/cell_editor.rs
index 57d7b1c..cac37f8 100644
--- a/src/ui/cell_editor.rs
+++ b/src/ui/cell_editor.rs
@@ -636,7 +636,7 @@ pub fn field(
 /// A terminal's cursor is a block: one character wide from where the next
 /// one goes, over the caret's line of the field `output` is of. Tinted, so
 /// the character under it reads through.
-fn block_cursor(
+pub(crate) fn block_cursor(
     ui: &Ui,
     output: &egui::text_edit::TextEditOutput,
     role: TextRole,
diff --git a/src/ui/keys.rs b/src/ui/keys.rs
index e0892eb..7dfb204 100644
--- a/src/ui/keys.rs
+++ b/src/ui/keys.rs
@@ -1957,7 +1957,7 @@ pub fn after_frame(ctx: &egui::Context) {
 /// takes a key: [`letters`] takes such a character from the frame.
 const WINDOW_LETTERS: [Command; 4] = [
     Command::Reload,
-    Command::FindObject,
+    Command::FilterObjects,
     Command::ToggleTree,
     Command::Help,
 ];
@@ -2006,7 +2006,7 @@ fn letters(
             Command::Reload if app.active_sql().is_some() => {}
             Command::Reload if tree_arrows => actions.push(Action::RefreshTree(tab)),
             Command::Reload => actions.push(Action::Refresh(tab)),
-            Command::FindObject => {
+            Command::FilterObjects => {
                 use crate::ui::focus::{self, Region};
                 focus::step(ctx, true, Some(&[Region::Search]), None);
             }
diff --git a/src/ui/quick_open.rs b/src/ui/quick_open.rs
index 90edf6d..5b429f1 100644
--- a/src/ui/quick_open.rs
+++ b/src/ui/quick_open.rs
@@ -1,182 +1,1146 @@
-//! Quick open (Cmd/Ctrl+P): find a loaded table or view by name.
+//! Quick open (`⌘P`, Omarchy `ctrl+p`): a panel over the window that finds
+//! a table or view of the window's connection by name, or of every open
+//! connection (`⇥`, `tab`), and in the macOS and Windows looks starts a
+//! few commands. What it lists is [`App::quick_open_list`]'s; this draws
+//! it and reads its keys, which are its own while it is open (the window's
+//! handler does not run under a dialog). macOS: a floating panel 640 wide,
+//! 84 from the window's top, the window behind it not dimmed. Omarchy: the
+//! picker, 780 wide, an accent border on the sunken surface.
 
-use crate::app::App;
-use crate::i18n::gettext;
-use crate::model::{Action, Dialog};
-use crate::typography::Text;
-use crate::ui::widgets;
+use std::collections::HashMap;
+
+use egui::accesskit::Role;
+use egui::{Align2, Color32, CornerRadius, Rect, Sense, Stroke, StrokeKind, UiBuilder};
+use egui::{WidgetInfo, WidgetType, pos2, vec2};
+use tabletist_db::ObjectKind;
+
+use crate::app::{
+    App, Found, QuickCommand, QuickHeading, QuickItem, QuickList, QuickRow, StartRow,
+};
+use crate::env::{Environment, Platform, env_colors};
+use crate::i18n::{Locale, gettext};
+use crate::keymap::{Command, Keymap, Layout, Scope, Written};
+use crate::model::{Action, AgentsHere, ConnTabId, Dialog, QuickCaret, QuickGroup, StartRecent};
+use crate::theme::{Icon, Look, Palette};
+use crate::typography::{Text, TextRole};
+use crate::ui::format::display_safe;
+use crate::ui::widgets::{self, ButtonSpec};
+use crate::ui::workspace::{Badge, env_badge};
+
+/// The panel's id: its area's, and the root of its parts' ids.
+fn panel_id() -> egui::Id {
+    egui::Id::new("quick-open")
+}
+
+/// Where quick open's field keeps the keyboard.
+pub(crate) fn field_id() -> egui::Id {
+    panel_id().with("field")
+}
+
+/// The list's id: the listbox the field controls.
+fn list_id() -> egui::Id {
+    panel_id().with("list")
+}
+
+/// The panel's width and how far below the window's top it floats.
+fn place(look: &Look) -> (f32, f32) {
+    if look.terminal {
+        (780.0, 120.0)
+    } else {
+        (640.0, 84.0)
+    }
+}
+
+/// A result's row, and a heading's. The terminal look's headings are a
+/// row's height, so that its eight rows are eight lines of either.
+fn heights(look: &Look) -> (f32, f32) {
+    if look.terminal {
+        (24.0, 24.0)
+    } else {
+        (36.0, 28.0)
+    }
+}
+
+/// How many rows show before the list scrolls.
+fn rows_shown(look: &Look) -> usize {
+    if look.terminal { 8 } else { 9 }
+}
+
+/// A window quick open names: its connection, environment and database.
+struct Window {
+    name: String,
+    env: Environment,
+    database: String,
+}
+
+/// The keys the panel names and reads, as the keymap writes them.
+struct Keys {
+    moves: Written,
+    open: Written,
+    editor: Written,
+    scope: Written,
+    number: Written,
+    close: Written,
+}
+
+fn keys(keymap: &Keymap, layout: Layout) -> Keys {
+    let key = |command| keymap.key_in(layout, command, Scope::Prompt);
+    Keys {
+        moves: keymap.label(layout, Command::QuickOpenMove),
+        open: key(Command::QuickOpenPick),
+        editor: key(Command::QuickOpenPickInEditor),
+        scope: key(Command::QuickOpenScope),
+        number: key(Command::QuickOpenNumber),
+        close: key(Command::QuickOpenClose),
+    }
+}
+
+/// What the panel draws with, gathered before the dialog is borrowed.
+struct Skin<'a> {
+    look: &'a Look,
+    palette: &'a Palette,
+    locale: Locale,
+    /// Seconds since the Unix epoch, for how long ago a recent was opened.
+    now: u64,
+    /// Whether numbers are written in threes.
+    grouped: bool,
+    windows: &'a HashMap<ConnTabId, Window>,
+    /// The window quick open was opened in.
+    here: ConnTabId,
+    /// Who is connected to the window's Agents tab: the Agents command
+    /// says it after the connection's name.
+    agents: Option<String>,
+    /// The commands' keys, as the window reads them.
+    command_keys: &'a HashMap<Command, Written>,
+    /// The key that opens the picked result, written on its row (macOS).
+    open_key: String,
+}
+
+impl Skin<'_> {
+    fn say(&self, text: &'static str) -> String {
+        self.look.label(&gettext(self.locale, text))
+    }
+
+    /// A window's connection as the chip and the headings name it: its
+    /// name and its environment (`Bookshop · dev`).
+    fn window_name(&self, tab: ConnTabId) -> String {
+        let Some(window) = self.windows.get(&tab) else {
+            return String::new();
+        };
+        let name = display_safe(&window.name);
+        if window.env == Environment::None {
+            name.into_owned()
+        } else {
+            format!("{name} · {}", window.env.label(Platform::Native))
+        }
+    }
+}
 
 pub fn show(app: &mut App, ctx: &egui::Context) {
-    let locale = app.locale;
-    let palette = app.palette;
-    let look = app.look;
-    let (keymap, layout) = (app.keymap.clone(), app.layout());
-    let (tab, query) = match &app.dialog {
-        Some(Dialog::QuickOpen(open)) => (open.tab, open.query.clone()),
-        _ => return,
+    let (look, palette, locale) = (app.look, app.palette, app.locale);
+    let Some(Dialog::QuickOpen(open)) = &app.dialog else {
+        return;
     };
-    let matches = app.quick_open_found(tab, &query);
-    // The Agents entry's right side: the window's connection, and the key.
-    let agents_note = app.agents_name(tab).map(|name| {
-        let key = keymap.key(layout, crate::keymap::Command::ShowAgents);
-        format!("{name} · {}", key.as_str())
+    let here = open.tab;
+    // Quick open goes with the window it was opened in.
+    if app.workspace(here).is_none() {
+        app.actions.push(Action::CloseDialog);
+        return;
+    }
+    let list = app.quick_open_list(open);
+    let layout = app.layout();
+    let keymap = app.keymap.clone();
+    let keys = keys(&keymap, layout);
+    let windows: HashMap<ConnTabId, Window> = app
+        .open_connections()
+        .map(|(tab, workspace)| {
+            let database = crate::ui::workspace::bar_info(app, tab)
+                .map(|info| info.database)
+                .unwrap_or_default();
+            let window = Window {
+                name: workspace.name.clone(),
+                env: workspace.environment,
+                database,
+            };
+            (tab, window)
+        })
+        .collect();
+    let agents = app.agents_here(here).map(|state| {
+        let said = match &state {
+            AgentsHere::ServerOff => gettext(locale, "server off").into_owned(),
+            AgentsHere::OffHere => gettext(locale, "off here").into_owned(),
+            AgentsHere::NoneConnected => gettext(locale, "none connected").into_owned(),
+            AgentsHere::Connected { agent, .. } => agent.clone(),
+        };
+        look.label(&said)
     });
+    let command_keys: HashMap<Command, Written> = [
+        Command::NewSqlTab,
+        Command::SwitchDatabase,
+        Command::ShowAgents,
+    ]
+    .into_iter()
+    .map(|command| (command, keymap.key_in(layout, command, Scope::Global)))
+    .collect();
+    let skin = Skin {
+        look: &look,
+        palette: &palette,
+        locale,
+        now: crate::util::now_secs(),
+        grouped: app.settings.grouped(layout),
+        windows: &windows,
+        here,
+        agents,
+        command_keys: &command_keys,
+        open_key: keys.open.as_str().to_owned(),
+    };
+    let mut actions = Vec::new();
+    // The panel's keys come before its field sees the frame: Tab, the
+    // arrows, Enter and Esc are quick open's, and so are the digits while
+    // nothing is typed (Omarchy).
+    // Nothing typed but spaces lists what nothing typed lists, numbers
+    // and all.
+    let nothing_typed = open.query.trim().is_empty();
+    ctx.input_mut(|input| read_keys(input, &keymap, layout, nothing_typed, &mut actions));
     let Some(Dialog::QuickOpen(open)) = &mut app.dialog else {
         return;
     };
-    let mut actions = Vec::new();
-    let modal =
-        crate::ui::widgets::modal(egui::Id::new("quick-open"), &look, &palette).show(ctx, |ui| {
-            ui.set_width(520.0);
-            widgets::label(
-                ui,
-                widgets::dialog_title(&look),
-                &gettext(locale, "Open table or view"),
-                palette.text,
-                &look,
-            );
-            ui.add_space(6.0);
-            let field = crate::ui::widgets::add_search(
-                ui,
-                crate::ui::widgets::search_field(
-                    ui,
-                    &mut open.query,
-                    &gettext(locale, "Name"),
-                    &look,
-                )
-                .desired_width(f32::INFINITY),
-                &look,
-            );
-            field.request_focus();
-            if field.changed() {
-                open.selected = 0;
-            }
-            widgets::label(
-                ui,
-                widgets::body(&look),
-                &gettext(locale, "Searches schemas loaded in the sidebar"),
-                palette.secondary,
-                &look,
-            );
-            ui.add_space(4.0);
-            if matches.is_empty() {
-                widgets::label(
-                    ui,
-                    widgets::body(&look),
-                    &gettext(locale, "No matches"),
-                    palette.secondary,
-                    &look,
-                );
-            }
-            // Scroll to the selection only when it moved, so a wheel scroll
-            // is not undone on the next frame.
-            let reveal = open.scrolled_to != Some(open.selected);
-            open.scrolled_to = Some(open.selected);
-            let output = egui::ScrollArea::vertical()
-                .max_height(360.0)
-                .show(ui, |ui| {
-                    for (index, found) in matches.iter().enumerate() {
-                        let label = match found {
-                            crate::app::Found::Agents => gettext(locale, "Agents").into_owned(),
-                            crate::app::Found::Object(object, _) => {
-                                crate::ui::format::object_title(object, true)
-                            }
-                        };
-                        let note = match found {
-                            crate::app::Found::Agents => agents_note.as_deref(),
-                            crate::app::Found::Object(..) => None,
-                        };
-                        let selected = index == open.selected;
-                        let (rect, response) = ui.allocate_exact_size(
-                            egui::vec2(ui.available_width(), look.tree_row),
-                            egui::Sense::click(),
-                        );
-                        response.widget_info(|| {
-                            egui::WidgetInfo::selected(
-                                egui::WidgetType::SelectableLabel,
-                                true,
-                                selected,
-                                &label,
-                            )
-                        });
-                        crate::ui::widgets::selection(
-                            ui,
-                            rect,
-                            selected,
-                            response.hovered(),
-                            &look,
-                            &palette,
-                        );
-                        crate::ui::focus::hint(
-                            ui,
-                            &response,
-                            crate::ui::widgets::selection_rect(rect, &look),
-                            crate::ui::focus::Ring::Inset {
-                                radius: look.radius.saturating_sub(2),
-                            },
-                        );
-                        // Long names end in "…" and show in full on hover.
-                        let role = widgets::body(&look);
-                        let color = crate::ui::widgets::selection_text(selected, &look, &palette);
-                        let room = rect.width() - 20.0;
-                        let shown = crate::ui::grid::ellipsize(&label, room, false, |text| {
-                            role.width(ui.ctx(), look.faces, text)
-                        });
-                        if shown != label {
-                            response.clone().on_hover_text(&label);
-                        }
-                        widgets::paint_text(
-                            ui,
-                            rect.left() + 10.0,
-                            rect.center().y,
-                            Text::one(&look, role, &shown, color),
-                        );
-                        if let Some(note) = note {
-                            let laid = Text::one(
-                                &look,
-                                widgets::secondary(&look),
-                                note,
-                                palette.secondary,
-                            )
-                            .layout(ui.ctx());
-                            laid.paint_left(
-                                ui.painter(),
-                                rect.right() - 10.0 - laid.width(),
-                                rect.center().y,
-                            );
-                        }
-                        if reveal && index == open.selected {
-                            response.scroll_to_me(None);
-                        }
-                        if response.clicked() {
-                            open.selected = index;
-                            actions.push(Action::QuickOpenPick);
-                        }
-                    }
-                });
-            open.scroll_offset = output.state.offset.y;
-        });
-    if modal.is_top_modal {
-        use crate::keymap::{Command, Scope};
-        ctx.input_mut(|input| {
-            let moves = keymap.chords_now(layout, Command::QuickOpenMove, Scope::Prompt, true);
-            for keys in moves {
-                let step = if keys == "up" { -1 } else { 1 };
-                if crate::ui::keys::presses(input, layout, keys).count > 0 {
-                    actions.push(Action::QuickOpenMove(step));
-                }
-            }
-            let on = |input: &mut egui::InputState, command| {
-                let scope = Scope::Prompt;
-                crate::ui::keys::asked(input, &keymap, layout, command, scope, true).count > 0
+    let (width, top) = place(&look);
+    let width = width.min(ctx.content_rect().width() - 32.0).max(200.0);
+    let area = egui::Modal::default_area(panel_id()).anchor(Align2::CENTER_TOP, vec2(0.0, top));
+    let modal = egui::Modal::new(panel_id())
+        .area(area)
+        .frame(frame(&look, &palette))
+        // The window behind is not dimmed.
+        .backdrop_color(Color32::TRANSPARENT)
+        .show(ctx, |ui| {
+            ui.set_width(width);
+            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
+            let name = if look.terminal {
+                gettext(locale, "Find object")
+            } else {
+                gettext(locale, "Quick open")
             };
-            if on(input, Command::QuickOpenPick) {
-                actions.push(Action::QuickOpenPick);
+            ui.ctx().accesskit_node_builder(ui.unique_id(), |node| {
+                node.set_role(Role::Dialog);
+                node.set_label(name.into_owned());
+            });
+            let caret = open.caret.take();
+            let changed = field(ui, open, &list, &skin, caret);
+            if changed {
+                actions.push(Action::QuickOpenChanged);
             }
-            if on(input, Command::QuickOpenClose) {
-                actions.push(Action::CloseDialog);
+            let selected = open.selected.min(list.items().count().saturating_sub(1));
+            if list.is_empty() {
+                no_match(
+                    ui,
+                    (&open.query, open.everywhere),
+                    &keys,
+                    &skin,
+                    &mut actions,
+                );
+            } else {
+                // Scroll to the pick only when it moved, so a wheel scroll
+                // is not undone on the next frame.
+                let reveal = open.scrolled_to != Some(selected);
+                open.scrolled_to = Some(selected);
+                let offset = rows(ui, &list, selected, reveal, &skin, &mut actions);
+                open.scroll_offset = offset;
             }
+            footer(ui, open.everywhere, nothing_typed, &keys, &skin);
         });
+    if modal.backdrop_response.clicked() {
+        actions.push(Action::CloseDialog);
     }
     app.actions.extend(actions);
 }
+
+/// The panel's frame: the dialog's corner and shadow, its parts edge to
+/// edge (macOS); Omarchy's accent border on the sunken surface.
+fn frame(look: &Look, palette: &Palette) -> egui::Frame {
+    if look.terminal {
+        egui::Frame::new()
+            .fill(palette.panel)
+            .stroke(Stroke::new(1.0, palette.accent))
+            .corner_radius(CornerRadius::same(3))
+    } else {
+        widgets::modal_frame(look, palette)
+            .fill(palette.window)
+            .inner_margin(egui::Margin::ZERO)
+    }
+}
+
+/// Reads the panel's keys out of the frame: Esc closes; the arrows (and
+/// Omarchy's ctrl+n/p) move the pick; Enter opens it, `⌥↩` in a SQL
+/// editor; Tab switches the scope; a digit opens that numbered row while
+/// nothing is typed (Omarchy), and then is no text for the field; `⌘P`
+/// again selects what is typed.
+fn read_keys(
+    input: &mut egui::InputState,
+    keymap: &Keymap,
+    layout: Layout,
+    nothing_typed: bool,
+    actions: &mut Vec<Action>,
+) {
+    use crate::ui::keys::{asked, digit_asked, presses};
+    let scope = Scope::Prompt;
+    let on = |input: &mut egui::InputState, command| {
+        asked(input, keymap, layout, command, scope, true).count > 0
+    };
+    if on(input, Command::QuickOpenClose) {
+        actions.push(Action::CloseDialog);
+        return;
+    }
+    // The keymap lists the keys that move up first, then as many that
+    // move down.
+    let moves: Vec<&'static str> = keymap
+        .chords_now(layout, Command::QuickOpenMove, scope, true)
+        .collect();
+    let half = moves.len() / 2;
+    for (nth, keys) in moves.into_iter().enumerate() {
+        let step = if nth < half { -1 } else { 1 };
+        for _ in 0..presses(input, layout, keys).count {
+            actions.push(Action::QuickOpenMove(step));
+        }
+    }
+    if on(input, Command::QuickOpenPickInEditor) {
+        actions.push(Action::QuickOpenPick {
+            index: None,
+            in_editor: true,
+        });
+    }
+    if on(input, Command::QuickOpenPick) {
+        actions.push(Action::QuickOpenPick {
+            index: None,
+            in_editor: false,
+        });
+    }
+    if on(input, Command::QuickOpenScope) {
+        actions.push(Action::QuickOpenScope);
+    }
+    if nothing_typed
+        && let Some(index) =
+            digit_asked(input, keymap, layout, Command::QuickOpenNumber, scope, true)
+    {
+        // The digit's text goes with its key: the field must not get it.
+        input.events.retain(|event| {
+            !matches!(event, egui::Event::Text(text)
+                if text.len() == 1 && text.chars().all(|char| char.is_ascii_digit()))
+        });
+        actions.push(Action::QuickOpenPick {
+            index: Some(index),
+            in_editor: false,
+        });
+    }
+    if asked(
+        input,
+        keymap,
+        layout,
+        Command::FindObject,
+        Scope::Global,
+        true,
+    )
+    .count
+        > 0
+    {
+        actions.push(Action::OpenQuickOpen);
+    }
+}
+
+/// The field's line. macOS: a magnifier, the scope's chip (the window's
+/// connection with its environment's dot, the production pill, or "All
+/// open connections") and the field with its placeholder. Omarchy: the
+/// `>` prompt, the scope's label (`DEV`, `PROD`, `ALL`), the field with a
+/// block cursor, and the count of matches at the right. The field is a
+/// combobox that controls the list. Returns whether what is typed changed.
+fn field(
+    ui: &mut egui::Ui,
+    open: &mut crate::model::QuickOpen,
+    list: &QuickList,
+    skin: &Skin<'_>,
+    caret: Option<QuickCaret>,
+) -> bool {
+    let (look, palette) = (skin.look, skin.palette);
+    let id = field_id();
+    let height = if look.terminal { 32.0 } else { 52.0 };
+    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
+    let pad = if look.terminal { 10.0 } else { 14.0 };
+    let y = rect.center().y;
+    let mut x = rect.left() + pad;
+    if look.terminal {
+        let prompt = Text::one(look, TextRole::OGroup, ">", palette.accent);
+        x += widgets::paint_text(ui, x, y, prompt) + 10.0;
+        x += scope_label(ui, (x, y), open.everywhere, skin) + 10.0;
+    } else {
+        let icon = Rect::from_center_size(pos2(x + 8.0, y), vec2(16.0, 16.0));
+        Icon::Search.image(palette.dim, 16.0).paint_at(ui, icon);
+        x += 16.0 + 10.0;
+        x += chip(ui, (x, y), open.everywhere, skin) + 10.0;
+    }
+    // The count at the right (Omarchy): how many match, and how many of
+    // them the list's height hides.
+    let mut right = rect.right() - pad;
+    if look.terminal && !open.query.trim().is_empty() {
+        let count = list.matches;
+        // The results below the lines the list shows at once.
+        let hidden = list
+            .rows
+            .iter()
+            .skip(rows_shown(look))
+            .filter(|row| matches!(row, QuickRow::Item(QuickItem::Object { .. })))
+            .count();
+        let mut said = if count == 0 {
+            "0".to_owned()
+        } else {
+            let words = if count == 1 { "match" } else { "matches" };
+            format!("{count} {}", skin.say(words))
+        };
+        if hidden > 0 {
+            said = format!("{said} · {hidden} {}", skin.say("hidden"));
+        }
+        let text = Text::one(look, TextRole::OBody, &said, palette.dim);
+        right -= widgets::paint_text_right(ui, right, y, text) + 10.0;
+    }
+    let role = widgets::body(look);
+    let line = role.row_height(ui.ctx(), look.faces);
+    let inner = Rect::from_min_max(
+        pos2(x, y - role.middle(ui.ctx(), look.faces)),
+        pos2(right.max(x), y - role.middle(ui.ctx(), look.faces) + line),
+    );
+    if let Some(caret) = caret {
+        use egui::text::{CCursor, CCursorRange};
+        let mut state = egui::text_edit::TextEditState::load(ui.ctx(), id).unwrap_or_default();
+        let end = CCursor::new(open.query.chars().count());
+        let range = match caret {
+            QuickCaret::All => CCursorRange::two(CCursor::new(0), end),
+            QuickCaret::End => CCursorRange::one(end),
+        };
+        state.cursor.set_char_range(Some(range));
+        state.store(ui.ctx(), id);
+    }
+    // What the field finds, and no more: tables, views and (macOS) the
+    // commands. Omarchy's prompt shows no placeholder, as its board.
+    let hint = if look.terminal {
+        skin.say("Find a table or view")
+    } else {
+        skin.say("Find a table, view or command")
+    };
+    let placeholder = if look.terminal {
+        String::new()
+    } else {
+        hint.clone()
+    };
+    let mut child = ui.new_child(UiBuilder::new().max_rect(inner));
+    let placeholder = Text::one(look, role, &placeholder, palette.dim).layout(child.ctx());
+    let output = egui::TextEdit::singleline(&mut open.query)
+        .id(id)
+        .font(role.font_id(look.faces))
+        .frame(egui::Frame::NONE)
+        .margin(egui::Margin::ZERO)
+        .desired_width(inner.width())
+        .text_color(palette.text)
+        .hint_text(placeholder.galley)
+        .show(&mut child);
+    let response = &output.response;
+    response.widget_info(|| WidgetInfo::labeled(WidgetType::TextEdit, true, &hint));
+    ui.ctx().accesskit_node_builder(id, |node| {
+        node.set_role(Role::EditableComboBox);
+        node.push_controlled(list_id().accesskit_id());
+        node.set_expanded(true);
+    });
+    // The panel's one field: its caret (Omarchy's block) says where the
+    // keyboard is, and no ring is drawn round it, as the boards draw it.
+    crate::ui::focus::hint(ui, response, inner, crate::ui::focus::Ring::Own);
+    if look.terminal {
+        crate::ui::cell_editor::block_cursor(ui, &output, role, look, palette);
+    }
+    // The field keeps the keyboard while quick open is open: Tab, the
+    // arrows and Esc are the panel's, read before the field.
+    response.request_focus();
+    ui.memory_mut(|memory| {
+        memory.set_focus_lock_filter(
+            id,
+            egui::EventFilter {
+                tab: true,
+                horizontal_arrows: true,
+                vertical_arrows: true,
+                escape: true,
+            },
+        );
+    });
+    widgets::hline(ui, rect.x_range(), rect.bottom() - 0.5, palette.outline);
+    response.changed()
+}
+
+/// The macOS chip of what is searched, at `x`: the window's connection
+/// with its environment's dot, on production the red pill, or "All open
+/// connections" on the accent's soft ground. Returns its width.
+fn chip(ui: &egui::Ui, (x, y): (f32, f32), everywhere: bool, skin: &Skin<'_>) -> f32 {
+    let (look, palette) = (skin.look, skin.palette);
+    let env = skin
+        .windows
+        .get(&skin.here)
+        .map_or(Environment::None, |window| window.env);
+    let role = TextRole::Secondary;
+    let (words, fill, border, ink, dot) = if everywhere {
+        let words = skin.say("All open connections");
+        let border = palette.accent.gamma_multiply(0.35);
+        (words, palette.selection, border, palette.accent_hover, None)
+    } else if env.is_production() {
+        // The token `on-danger`: white on the danger red, in both modes.
+        let words = env.label(Platform::Native).to_owned();
+        (words, palette.danger, palette.danger, Color32::WHITE, None)
+    } else {
+        let colors = env_colors(env, Platform::of(look), palette);
+        let words = skin.window_name(skin.here);
+        let dot = (env != Environment::None).then(|| colors.base());
+        (
+            words,
+            Color32::TRANSPARENT,
+            palette.border,
+            palette.secondary,
+            dot,
+        )
+    };
+    let laid = Text::one(look, role, &words, ink).layout(ui.ctx());
+    let dot_room = if dot.is_some() { 7.0 + 6.0 } else { 0.0 };
+    let width = 8.0 + dot_room + laid.width() + 8.0;
+    let rect = Rect::from_min_size(pos2(x, y - 12.0), vec2(width, 24.0));
+    let corner = CornerRadius::same(12);
+    ui.painter().rect_filled(rect, corner, fill);
+    ui.painter()
+        .rect_stroke(rect, corner, Stroke::new(1.0, border), StrokeKind::Inside);
+    let mut left = rect.left() + 8.0;
+    if let Some(dot) = dot {
+        ui.painter().circle_filled(pos2(left + 3.5, y), 3.5, dot);
+        left += dot_room;
+    }
+    laid.paint_left(ui.painter(), left, y);
+    widgets::announce(ui, rect, &words);
+    width
+}
+
+/// The Omarchy prompt's label of what is searched, at `x`: the window's
+/// environment (`DEV`, `PROD` in the danger red) or `ALL` on the accent.
+/// Returns its width.
+fn scope_label(ui: &egui::Ui, (x, y): (f32, f32), everywhere: bool, skin: &Skin<'_>) -> f32 {
+    let (look, palette) = (skin.look, skin.palette);
+    if everywhere {
+        let laid = Text::one(look, TextRole::OBadge, "ALL", palette.on_accent).layout(ui.ctx());
+        let rect = Rect::from_min_size(
+            pos2(x, y - laid.height() / 2.0 - 1.0),
+            vec2(laid.width() + 12.0, laid.height() + 2.0),
+        );
+        ui.painter()
+            .rect_filled(rect, CornerRadius::same(2), palette.accent);
+        laid.paint_left(ui.painter(), x + 6.0, y);
+        widgets::announce(ui, rect, "ALL");
+        return rect.width();
+    }
+    let env = skin
+        .windows
+        .get(&skin.here)
+        .map_or(Environment::None, |window| window.env);
+    let colors = env_colors(env, Platform::Omarchy, palette);
+    let width = env_badge(ui, x, y, env, &colors, Badge::Plain, look);
+    let rect = Rect::from_min_size(pos2(x, y - 8.0), vec2(width, 16.0));
+    widgets::announce(ui, rect, env.label(Platform::Omarchy));
+    width
+}
+
+/// The words of a result: its icon, name with what matched marked, the
+/// words after it (macOS) or its kind (Omarchy), what stands at its right,
+/// and what a screen reader says of it.
+struct Shown {
+    icon: Icon,
+    name: String,
+    spans: Vec<std::ops::Range<usize>>,
+    /// macOS: the kind and the schema, or what a command does. Omarchy:
+    /// the kind.
+    detail: String,
+    meta: String,
+    label: String,
+    /// A group's "more" row: the name in the accent, no icon.
+    more: bool,
+}
+
+fn kind_word(kind: ObjectKind, skin: &Skin<'_>) -> String {
+    skin.say(match kind {
+        ObjectKind::Table => "table",
+        ObjectKind::View => "view",
+        ObjectKind::MaterializedView => "materialized view",
+    })
+}
+
+fn kind_icon(kind: ObjectKind) -> Icon {
+    match kind {
+        ObjectKind::Table => Icon::Table,
+        ObjectKind::View | ObjectKind::MaterializedView => Icon::Eye,
+    }
+}
+
+/// An object's result: `name`, `kind · schema` (Omarchy: the kind), and
+/// at its right `meta`.
+fn object_shown(
+    object: &tabletist_db::ObjectRef,
+    kind: ObjectKind,
+    spans: Vec<std::ops::Range<usize>>,
+    meta: String,
+    skin: &Skin<'_>,
+) -> Shown {
+    let name = object.name.clone();
+    let schema = display_safe(&object.schema).into_owned();
+    let kind_word = kind_word(kind, skin);
+    let detail = if skin.look.terminal {
+        kind_word.clone()
+    } else {
+        format!("{kind_word} · {schema}")
+    };
+    let mut label = format!("{}, {kind_word}, {schema}", display_safe(&name));
+    if !meta.is_empty() {
+        label = format!("{label}, {meta}");
+    }
+    Shown {
+        icon: kind_icon(kind),
+        name,
+        spans,
+        detail,
+        meta,
+        label,
+        more: false,
+    }
+}
+
+fn shown(item: &QuickItem, skin: &Skin<'_>) -> Shown {
+    let look = skin.look;
+    let estimate = |rows: Option<u64>| {
+        rows.map_or_else(String::new, |rows| {
+            crate::ui::start::estimate_words(rows, look, skin.grouped, skin.locale)
+        })
+    };
+    match item {
+        QuickItem::Object { found, .. } => {
+            let Found {
+                object,
+                kind,
+                estimate: rows,
+                spans,
+                ..
+            } = found;
+            object_shown(object, *kind, spans.clone(), estimate(*rows), skin)
+        }
+        QuickItem::Start { row, .. } => {
+            let zone = jiff::tz::TimeZone::system();
+            match row {
+                StartRow::Largest(object, rows) => {
+                    object_shown(object, ObjectKind::Table, Vec::new(), estimate(*rows), skin)
+                }
+                StartRow::Recent(StartRecent::Object {
+                    object,
+                    kind,
+                    opened,
+                }) => {
+                    let when = look.label(&crate::recents::when(*opened, skin.now, &zone));
+                    object_shown(object, *kind, Vec::new(), when, skin)
+                }
+                StartRow::Recent(StartRecent::Sql {
+                    number,
+                    comment,
+                    opened,
+                    ..
+                }) => {
+                    let when = look.label(&crate::recents::when(*opened, skin.now, &zone));
+                    let name = format!("{} {number}", skin.say("Query"));
+                    let editor = skin.say("SQL editor");
+                    let detail = if look.terminal {
+                        skin.say("sql")
+                    } else {
+                        comment.clone().unwrap_or_else(|| editor.clone())
+                    };
+                    let label = match comment {
+                        Some(comment) => format!("{name}, {editor}, {comment}, {when}"),
+                        None => format!("{name}, {editor}, {when}"),
+                    };
+                    Shown {
+                        icon: Icon::Code,
+                        name,
+                        spans: Vec::new(),
+                        detail,
+                        meta: when,
+                        label,
+                        more: false,
+                    }
+                }
+            }
+        }
+        QuickItem::Command { command, spans } => {
+            let name = gettext(skin.locale, command.name()).into_owned();
+            let key = skin
+                .command_keys
+                .get(&command.command())
+                .map(|key| key.as_str().to_owned())
+                .unwrap_or_default();
+            let (icon, detail) = match command {
+                QuickCommand::NewSql { read_only } => {
+                    let detail = if *read_only {
+                        skin.say("opens read-only")
+                    } else {
+                        String::new()
+                    };
+                    (Icon::Code, detail)
+                }
+                QuickCommand::SwitchDatabase => (Icon::Database, String::new()),
+                QuickCommand::Agents => {
+                    let mut detail = skin.window_name(skin.here);
+                    if let Some(said) = &skin.agents {
+                        detail = format!("{detail} · {said}");
+                    }
+                    (Icon::SquareTerminal, detail)
+                }
+            };
+            let mut label = format!("{name}, {}", skin.say("command"));
+            for words in [&detail, &key] {
+                if !words.is_empty() {
+                    label = format!("{label}, {words}");
+                }
+            }
+            Shown {
+                icon,
+                name,
+                spans: spans.clone(),
+                detail,
+                meta: key,
+                label,
+                more: false,
+            }
+        }
+        QuickItem::More { group, hidden } => {
+            let name = match group {
+                QuickGroup::Tables if *hidden == 1 => skin.say("1 more table"),
+                QuickGroup::Tables => format!("{hidden} {}", skin.say("more tables")),
+                QuickGroup::Views if *hidden == 1 => skin.say("1 more view"),
+                QuickGroup::Views => format!("{hidden} {}", skin.say("more views")),
+                QuickGroup::Connection(tab) => format!(
+                    "{hidden} {} {}",
+                    skin.say("more in"),
+                    skin.window_name(*tab)
+                ),
+            };
+            Shown {
+                icon: Icon::Ellipsis,
+                label: name.clone(),
+                name,
+                spans: Vec::new(),
+                detail: String::new(),
+                meta: String::new(),
+                more: true,
+            }
+        }
+    }
+}
+
+/// The list: its headings and results, scrolling past the first rows
+/// that fit, only the rows in view drawn. The picked result is on the
+/// selection's ground. A click opens a result. Returns the list's scroll
+/// offset.
+fn rows(
+    ui: &mut egui::Ui,
+    list: &QuickList,
+    selected: usize,
+    reveal: bool,
+    skin: &Skin<'_>,
+    actions: &mut Vec<Action>,
+) -> f32 {
+    let look = skin.look;
+    let (row, heading) = heights(look);
+    let pad = if look.terminal { 0.0 } else { 6.0 };
+    // Each row's top and height in the list.
+    let mut y = pad;
+    let placed: Vec<(f32, f32)> = list
+        .rows
+        .iter()
+        .map(|entry| {
+            let height = match entry {
+                QuickRow::Heading(_) => heading,
+                QuickRow::Item(_) => row,
+            };
+            let top = y;
+            y += height;
+            (top, height)
+        })
+        .collect();
+    let total = y + pad;
+    let count = list.items().count();
+    let max = rows_shown(look) as f32 * row + 2.0 * pad;
+    let width = ui.available_width();
+    let output = egui::ScrollArea::vertical()
+        .id_salt(panel_id().with("scroll"))
+        .max_height(max)
+        .auto_shrink([false, true])
+        .show_viewport(ui, |ui, viewport| {
+            ui.set_height(total);
+            ui.set_width(width);
+            let origin = ui.max_rect().min;
+            let whole = ui.interact(
+                Rect::from_min_size(origin, vec2(width, total)),
+                list_id(),
+                Sense::hover(),
+            );
+            whole.widget_info(|| WidgetInfo::labeled(WidgetType::Other, true, "Results"));
+            ui.ctx().accesskit_node_builder(list_id(), |node| {
+                node.set_role(Role::ListBox);
+                node.set_size_of_set(count);
+            });
+            let mut index = 0;
+            let mut inner = ui.new_child(
+                UiBuilder::new()
+                    .max_rect(ui.max_rect())
+                    .accessibility_parent(list_id()),
+            );
+            for (entry, (top, height)) in list.rows.iter().zip(&placed) {
+                let rect =
+                    Rect::from_min_size(origin + vec2(pad, *top), vec2(width - 2.0 * pad, *height));
+                let seen = *top + height >= viewport.min.y && *top <= viewport.max.y;
+                match entry {
+                    QuickRow::Heading(title) => {
+                        if seen {
+                            heading_row(&mut inner, rect, *title, skin);
+                        }
+                    }
+                    QuickRow::Item(item) => {
+                        let picked = index == selected;
+                        if picked && reveal {
+                            inner.scroll_to_rect(rect, None);
+                        }
+                        if seen || picked {
+                            let response =
+                                result_row(&mut inner, rect, item, (index, count), picked, skin);
+                            if response.clicked() {
+                                actions.push(Action::QuickOpenPick {
+                                    index: Some(index),
+                                    in_editor: false,
+                                });
+                            }
+                        }
+                        index += 1;
+                    }
+                }
+            }
+        });
+    output.state.offset.y
+}
+
+/// A heading over a group: Recent, Tables, Views, Commands, or a
+/// connection's name with its environment (Omarchy: in the danger red on
+/// production). A label, never a result.
+fn heading_row(ui: &mut egui::Ui, rect: Rect, title: QuickHeading, skin: &Skin<'_>) {
+    let (look, palette) = (skin.look, skin.palette);
+    let words = |text: &'static str| gettext(skin.locale, text).into_owned();
+    let (text, env) = match title {
+        QuickHeading::Recent => (words("Recent"), Environment::None),
+        QuickHeading::Largest => (words("Largest tables"), Environment::None),
+        QuickHeading::Tables => (words("Tables"), Environment::None),
+        QuickHeading::Views => (words("Views"), Environment::None),
+        QuickHeading::Commands => (words("Commands"), Environment::None),
+        QuickHeading::Connection(tab) => skin.windows.get(&tab).map_or_else(
+            || (String::new(), Environment::None),
+            |window| (display_safe(&window.name).into_owned(), window.env),
+        ),
+    };
+    let inset = 10.0;
+    let y = rect.bottom() - 4.0 - 8.0;
+    if look.terminal {
+        // As the brief writes them: `bookshop · dev`, and `bookshop ·
+        // PROD` in the danger red.
+        let ink = if env.is_production() {
+            palette.danger
+        } else {
+            palette.dim
+        };
+        let text = if env == Environment::None {
+            text.to_lowercase()
+        } else if env.is_production() {
+            format!("{} · {}", text.to_lowercase(), env.label(Platform::Omarchy))
+        } else {
+            format!("{} · {}", text.to_lowercase(), env.label(Platform::Native))
+        };
+        let laid = Text::one(look, TextRole::OGroup, &text, ink);
+        widgets::paint_label(ui, rect.left() + inset, y, laid);
+        return;
+    }
+    let laid = widgets::section_label(&text, look, palette).layout(ui.ctx());
+    let width = laid.paint_left(ui.painter(), rect.left() + inset, y);
+    widgets::announce(
+        ui,
+        Rect::from_min_size(rect.min, vec2(width + inset, rect.height())),
+        &text,
+    );
+    if env != Environment::None {
+        let colors = env_colors(env, Platform::Native, palette);
+        env_badge(
+            ui,
+            rect.left() + inset + width + 8.0,
+            y,
+            env,
+            &colors,
+            Badge::Mac,
+            look,
+        );
+    }
+}
+
+/// One result in `rect`: number `index` of `count`. macOS: its icon, the
+/// name with what matched in the strong accent, the words after it, what
+/// stands at its right, and `↩` on the picked one. Omarchy: its number,
+/// the name with what matched in the accent, its kind in a column of its
+/// own and what stands at its right; the picked one on the accent's soft
+/// ground with a bar of the accent at its left. Never focused: the field
+/// keeps the keyboard, and its node names the picked option.
+fn result_row(
+    ui: &mut egui::Ui,
+    rect: Rect,
+    item: &QuickItem,
+    (index, count): (usize, usize),
+    picked: bool,
+    skin: &Skin<'_>,
+) -> egui::Response {
+    let (look, palette) = (skin.look, skin.palette);
+    let id = panel_id().with(("result", index));
+    let response = ui.interact(rect, id, Sense::click());
+    let shown = shown(item, skin);
+    response.widget_info(|| WidgetInfo::labeled(WidgetType::Other, true, &shown.label));
+    ui.ctx().accesskit_node_builder(id, |node| {
+        node.set_role(Role::ListBoxOption);
+        node.set_selected(picked);
+        node.set_position_in_set(index + 1);
+        node.set_size_of_set(count);
+    });
+    if picked {
+        ui.ctx().accesskit_node_builder(field_id(), |node| {
+            node.set_active_descendant(id.accesskit_id());
+        });
+    }
+    let y = rect.center().y;
+    if look.terminal {
+        if picked {
+            ui.painter()
+                .rect_filled(rect, CornerRadius::ZERO, palette.selection);
+            let bar = Rect::from_min_size(rect.min, vec2(3.0, rect.height()));
+            ui.painter()
+                .rect_filled(bar, CornerRadius::ZERO, palette.accent);
+        } else if response.hovered() {
+            ui.painter()
+                .rect_filled(rect, CornerRadius::ZERO, palette.text.gamma_multiply(0.08));
+        }
+        let left = rect.left() + 10.0;
+        let right = rect.right() - 10.0;
+        let number = (index + 1).to_string();
+        let number = Text::one(look, TextRole::OGroup, &number, palette.accent);
+        widgets::paint_text(ui, left, y, number);
+        let meta = Text::one(look, TextRole::OBody, &shown.meta, palette.dim);
+        widgets::paint_text_right(ui, right, y, meta);
+        let kind_x = right - 150.0 - 110.0;
+        let detail = Text::one(look, TextRole::OBody, &shown.detail, palette.dim);
+        widgets::paint_text(ui, kind_x, y, detail);
+        let room = (kind_x - 10.0 - (left + 28.0)).max(0.0);
+        let ink = if shown.more {
+            palette.accent
+        } else {
+            palette.text
+        };
+        let runs = (TextRole::OBody, ink, TextRole::OGroup, palette.accent);
+        let name = crate::ui::start::marked(look, &shown.name, &shown.spans, runs);
+        let laid = one_line(name, room).layout(ui.ctx());
+        laid.paint_left(ui.painter(), left + 28.0, y);
+        full_on_hover(&response, &laid, &shown.name);
+        return response;
+    }
+    let corner = CornerRadius::same(6);
+    if picked {
+        ui.painter().rect_filled(rect, corner, palette.selection);
+    } else if response.hovered() {
+        ui.painter()
+            .rect_filled(rect, corner, palette.surface_hover);
+    }
+    let left = rect.left() + 10.0;
+    let mut right = rect.right() - 10.0;
+    let strong = palette.accent_hover;
+    if !shown.more {
+        let ink = if picked { strong } else { palette.dim };
+        let icon = Rect::from_center_size(pos2(left + 9.0, y), vec2(16.0, 16.0));
+        shown.icon.image(ink, 16.0).paint_at(ui, icon);
+    }
+    // The column of the key that opens it, on the picked row.
+    let enter = Text::one(look, TextRole::Shortcut, &skin.open_key, palette.dim);
+    if picked {
+        widgets::paint_text_right(ui, right, y, enter);
+    }
+    right -= 20.0 + 10.0;
+    let meta = || Text::one(look, widgets::secondary(look), &shown.meta, palette.dim);
+    let meta_width = widgets::measure(ui, meta());
+    widgets::paint_text_right(ui, right, y, meta());
+    let x = left + 18.0 + 10.0;
+    let room = (right - meta_width - 10.0 - x).max(0.0);
+    let (role, ink) = if shown.more {
+        (TextRole::UiBody, palette.accent)
+    } else if picked {
+        (TextRole::UiBodyStrong, strong)
+    } else {
+        (TextRole::UiBodyStrong, palette.text)
+    };
+    let marks = (role, ink, TextRole::UiBodySemibold, strong);
+    let name = crate::ui::start::marked(look, &shown.name, &shown.spans, marks);
+    let laid = one_line(name, room).layout(ui.ctx());
+    let after = x + laid.width() + 8.0;
+    laid.paint_left(ui.painter(), x, y);
+    full_on_hover(&response, &laid, &shown.name);
+    let room = (right - meta_width - 10.0 - after).max(0.0);
+    let detail = crate::ui::grid::ellipsize(&shown.detail, room, false, |text| {
+        widgets::secondary(look).width(ui.ctx(), look.faces, text)
+    });
+    widgets::paint_text(
+        ui,
+        after,
+        y,
+        Text::one(look, widgets::secondary(look), &detail, palette.dim),
+    );
+    response
+}
+
+/// `text` on one line `room` wide: what does not fit ends in "…".
+fn one_line(mut text: Text, room: f32) -> Text {
+    let wrap = &mut text.job_mut().wrap;
+    wrap.max_width = room;
+    wrap.max_rows = 1;
+    wrap.break_anywhere = true;
+    wrap.overflow_character = Some('…');
+    text
+}
+
+/// A name cut short shows in full on hover.
+fn full_on_hover(response: &egui::Response, laid: &crate::typography::Laid, name: &str) {
+    if laid.galley.elided {
+        response.clone().on_hover_text(display_safe(name));
+    }
+}
+
+/// What the panel says when nothing matches: where it looked, and in this
+/// connection's scope a way to look in every open one.
+fn no_match(
+    ui: &mut egui::Ui,
+    (query, everywhere): (&str, bool),
+    keys: &Keys,
+    skin: &Skin<'_>,
+    actions: &mut Vec<Action>,
+) {
+    let (look, palette, locale) = (skin.look, skin.palette, skin.locale);
+    let query = query.trim();
+    let place = if everywhere {
+        gettext(locale, "open connections").into_owned()
+    } else if look.terminal {
+        skin.windows
+            .get(&skin.here)
+            .map(|window| display_safe(&window.database).into_owned())
+            .unwrap_or_default()
+    } else {
+        skin.window_name(skin.here)
+    };
+    let said = gettext(locale, "No objects match “{query}” in {place}")
+        .replace("{query}", &display_safe(query))
+        .replace("{place}", &place);
+    let said = look.label(&said);
+    let link = gettext(locale, "Search all open connections");
+    let height = if look.terminal { 52.0 } else { 96.0 };
+    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
+    if look.terminal {
+        let y = rect.center().y;
+        let x = rect.left() + 10.0;
+        let text = Text::one(look, TextRole::OBody, &said, palette.secondary);
+        let width = widgets::paint_label(ui, x, y, text);
+        if !everywhere {
+            let key = Text::one(look, TextRole::OBody, keys.scope.as_str(), palette.accent);
+            let x = x + width + 24.0;
+            let key_width = widgets::paint_text(ui, x, y, key);
+            let words = look.label(&link);
+            let words = Text::one(look, TextRole::OBody, &words, palette.dim);
+            widgets::paint_label(ui, x + key_width + 8.0, y, words);
+        }
+        return;
+    }
+    let text = Text::one(look, TextRole::UiBody, &said, palette.secondary);
+    let laid = text.layout(ui.ctx());
+    let y = rect.top() + 36.0;
+    laid.paint_left(ui.painter(), rect.center().x - laid.width() / 2.0, y);
+    widgets::announce(ui, rect, &said);
+    if !everywhere {
+        let words = format!("{link} {}", keys.scope.as_str());
+        let words = words.trim();
+        let button = ButtonSpec::new(words).link();
+        let button_width = button.width(ui, look);
+        let place =
+            Rect::from_center_size(pos2(rect.center().x, y + 28.0), vec2(button_width, 24.0));
+        if button.show_at(ui, place, look, palette).clicked() {
+            actions.push(Action::QuickOpenScope);
+        }
+    }
+}
+
+/// The key line under the list. macOS: `↑↓ move`, `↩ open`, `⌥↩ in SQL
+/// editor`, `⇥` and the scope it switches to, `esc close` at the right, on
+/// the sunken surface. Omarchy: the same in its words, `1..9 pick` only
+/// while nothing is typed.
+fn footer(ui: &mut egui::Ui, everywhere: bool, nothing_typed: bool, keys: &Keys, skin: &Skin<'_>) {
+    let (look, palette, locale) = (skin.look, skin.palette, skin.locale);
+    let height = if look.terminal { 30.0 } else { 34.0 };
+    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
+    if !look.terminal {
+        let corner = CornerRadius {
+            sw: look.dialog_radius,
+            se: look.dialog_radius,
+            ..CornerRadius::ZERO
+        };
+        ui.painter().rect_filled(rect, corner, palette.panel);
+    }
+    widgets::hline(ui, rect.x_range(), rect.top() + 0.5, palette.outline);
+    let word = |text: &'static str| look.label(&gettext(locale, text));
+    let scope = if everywhere {
+        word("This connection")
+    } else {
+        word("All connections")
+    };
+    let (moves, open, editor, pick, close) = (
+        word("move"),
+        word("open"),
+        word("in SQL editor"),
+        word("pick"),
+        word("close"),
+    );
+    let mut hints: Vec<widgets::Hint<'_>> =
+        vec![(&keys.moves, &moves, true), (&keys.open, &open, true)];
+    if !keys.editor.as_str().is_empty() {
+        hints.push((&keys.editor, &editor, true));
+    }
+    if look.terminal && nothing_typed && !keys.number.as_str().is_empty() {
+        hints.push((&keys.number, &pick, true));
+    }
+    hints.push((&keys.scope, &scope, true));
+    let pad = if look.terminal { 10.0 } else { 14.0 };
+    let gap = if look.terminal { 14.0 } else { 12.0 };
+    let y = rect.center().y;
+    if look.terminal {
+        hints.push((&keys.close, &close, true));
+        widgets::key_hints(ui, (rect.left() + pad, y), &hints, gap, look, palette);
+    } else {
+        widgets::key_hints(ui, (rect.left() + pad, y), &hints, gap, look, palette);
+        let last = [(&keys.close, close.as_str(), true)];
+        let width = widgets::key_hints_width(ui, &last, gap, look, palette);
+        widgets::key_hints(
+            ui,
+            (rect.right() - pad - width, y),
+            &last,
+            gap,
+            look,
+            palette,
+        );
+    }
+}
diff --git a/src/ui/start.rs b/src/ui/start.rs
index 5743864..c8302b3 100644
--- a/src/ui/start.rs
+++ b/src/ui/start.rs
@@ -9,7 +9,7 @@ use egui::{CornerRadius, Frame, Margin, Rect, Sense, Stroke, StrokeKind, WidgetI
 use egui::{pos2, vec2};
 use tabletist_db::{Driver, ObjectKind, ObjectRef};
 
-use crate::app::App;
+use crate::app::{App, Found};
 use crate::env::{Platform, env_colors};
 use crate::i18n::{Locale, gettext};
 use crate::keymap::{Command, Scope, Written};
@@ -129,8 +129,8 @@ struct Keys {
     recent: Written,
     sidebar: Written,
     help: Written,
-    /// Quick open's key, written at the macOS field's end: search goes on
-    /// from anywhere with it. Empty in the Omarchy look, which has none.
+    /// Quick open's key, written at the field's end: search goes on from
+    /// anywhere with it.
     quick_open: Written,
     /// The search: the keys that move among the matches (`↑↓`, `ctrl+n/p`),
     /// open one, open one in a SQL editor, clear the search, and (Omarchy)
@@ -271,7 +271,7 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
             // three ways to start.
             let mut claimed = look.terminal;
             if searching {
-                results(&mut ui, &matches, selected, &query, skin, &mut actions);
+                results(&mut ui, &matches, selected, skin, &mut actions);
                 if !look.terminal {
                     ui.add_space(gap(&look));
                     search_footer(&mut ui, &keys, skin);
@@ -619,16 +619,21 @@ fn lists(
 /// A table's estimate, as the look writes numbers: `≈ 48,200 rows` on
 /// macOS, `≈ 48.2k` in the terminal look.
 fn estimate_text(rows: u64, skin: Skin<'_>) -> String {
-    if skin.look.terminal {
-        skin.look
-            .label(&format!("≈ {}", crate::ui::format::compact_count(rows)))
+    estimate_words(rows, skin.look, skin.grouped, skin.locale)
+}
+
+/// The words of a table's estimate of `rows`, as `look` writes numbers
+/// (`grouped`: in threes): quick open's too.
+pub(crate) fn estimate_words(rows: u64, look: &Look, grouped: bool, locale: Locale) -> String {
+    if look.terminal {
+        look.label(&format!("≈ {}", crate::ui::format::compact_count(rows)))
     } else {
-        let count = if skin.grouped {
+        let count = if grouped {
             crate::ui::format::group_digits(rows)
         } else {
             rows.to_string()
         };
-        format!("≈ {count} {}", skin.say("rows"))
+        format!("≈ {count} {}", look.label(&gettext(locale, "rows")))
     }
 }
 
@@ -931,7 +936,7 @@ fn search_field(
     let (look, palette, locale) = (skin.look, skin.palette, skin.locale);
     let id = search_field_id(skin.tab);
     // What the search finds, and no more: the index has no functions and
-    // the app no saved queries, which the brief's words named too.
+    // the app no saved queries.
     let hint = look.label(&gettext(locale, "Find a table or view"));
     let full = ui.available_width();
     let height = if look.terminal { 30.0 } else { 44.0 };
@@ -944,17 +949,17 @@ fn search_field(
     let role = TextRole::pick(look, TextRole::UiBody, TextRole::OField);
     let pad = if look.terminal { 8.0 } else { 14.0 };
     // The lead mark: an accent `/` in the terminal look, a magnifier on
-    // macOS. The `⌘P` hint stands at the right on macOS; the Omarchy look
-    // has no quick open, so its line carries none.
+    // macOS. Quick open's key stands at the right: `⌘P from anywhere`.
     let slash = Text::one(look, TextRole::OGroup, "/", palette.accent).layout(ui.ctx());
     let mark = if look.terminal { slash.width() } else { 14.0 };
-    let shortcut = (!look.terminal && !keys.quick_open.as_str().is_empty()).then(|| {
-        let text = format!(
+    let shortcut = (!keys.quick_open.as_str().is_empty()).then(|| {
+        let text = look.label(&format!(
             "{} {}",
             keys.quick_open.as_str(),
             gettext(locale, "from anywhere")
-        );
-        Text::one(look, widgets::secondary(look), &text, palette.dim).layout(ui.ctx())
+        ));
+        let role = TextRole::pick(look, widgets::secondary(look), TextRole::OSecondary);
+        Text::one(look, role, &text, palette.dim).layout(ui.ctx())
     });
     let right_room = shortcut.as_ref().map_or(0.0, |laid| laid.width() + 10.0);
     let left = rect.left() + pad + mark + 8.0;
@@ -1049,9 +1054,8 @@ fn search_field(
 /// accent. Opening a row opens its object.
 fn results(
     ui: &mut egui::Ui,
-    matches: &[(ObjectRef, ObjectKind)],
+    matches: &[Found],
     selected: usize,
-    query: &str,
     skin: Skin<'_>,
     actions: &mut Vec<Action>,
 ) {
@@ -1071,14 +1075,14 @@ fn results(
         // own: the terminal board's dashboard, not the Mac's sections.
         group_header(ui, &gettext(locale, "matches"), skin);
         for (index, entry) in matches.iter().enumerate() {
-            result_row(ui, index, entry, query, index == selected, skin, actions);
+            result_row(ui, index, entry, index == selected, skin, actions);
         }
     } else {
         // The matches under the kind they are, tables first, then views,
         // as `start_matches` already orders them.
         let tables = matches
             .iter()
-            .take_while(|(_, k)| *k == ObjectKind::Table)
+            .take_while(|found| found.kind == ObjectKind::Table)
             .count();
         for (title, range) in [("Tables", 0..tables), ("Views", tables..matches.len())] {
             if range.is_empty() {
@@ -1086,15 +1090,7 @@ fn results(
             }
             group_header(ui, &gettext(locale, title), skin);
             for index in range {
-                result_row(
-                    ui,
-                    index,
-                    &matches[index],
-                    query,
-                    index == selected,
-                    skin,
-                    actions,
-                );
+                result_row(ui, index, &matches[index], index == selected, skin, actions);
             }
         }
     }
@@ -1117,19 +1113,18 @@ fn group_header(ui: &mut egui::Ui, title: &str, skin: Skin<'_>) {
     );
 }
 
-/// One match: on macOS its kind's icon, the name with the matched run in
+/// One match: on macOS its kind's icon, the name with the matched runs in
 /// the accent, and its schema; in the terminal look a cursor bar on the
 /// picked one, the name and the kind. A click opens it.
 fn result_row(
     ui: &mut egui::Ui,
     index: usize,
-    entry: &(ObjectRef, ObjectKind),
-    query: &str,
+    found: &Found,
     picked: bool,
     skin: Skin<'_>,
     actions: &mut Vec<Action>,
 ) {
-    let (object, kind) = (&entry.0, entry.1);
+    let (object, kind) = (&found.object, found.kind);
     let (look, palette) = (skin.look, skin.palette);
     let width = ui.available_width();
     let (rect, response) = ui.allocate_exact_size(vec2(width, row_height(look)), Sense::click());
@@ -1153,7 +1148,6 @@ fn result_row(
             },
         );
     }
-    let [before, hit, after] = highlight(&name, query);
     let y = rect.center().y;
     let left = rect.left() + INSET;
     let right = rect.right() - INSET;
@@ -1173,10 +1167,13 @@ fn result_row(
             Text::one(look, TextRole::OBody, &kind_word, palette.dim),
         );
         let room = (detail_x - 10.0 - (left + 28.0)).max(0.0);
-        let name = Text::new(look)
-            .add(TextRole::OBody, &before, palette.text)
-            .add(TextRole::OGroup, &hit, palette.accent)
-            .add(TextRole::OBody, &after, palette.text);
+        let runs = (
+            TextRole::OBody,
+            palette.text,
+            TextRole::OGroup,
+            palette.accent,
+        );
+        let name = marked(look, &object.name, &found.spans, runs);
         name.wrap(room)
             .layout(ui.ctx())
             .paint_left(ui.painter(), left + 28.0, y);
@@ -1196,10 +1193,13 @@ fn result_row(
         } else {
             palette.text
         };
-        let name = Text::new(look)
-            .add(TextRole::UiBodyStrong, &before, name_ink)
-            .add(TextRole::UiBodyStrong, &hit, palette.accent)
-            .add(TextRole::UiBodyStrong, &after, name_ink);
+        let role = TextRole::UiBodyStrong;
+        let name = marked(
+            look,
+            &object.name,
+            &found.spans,
+            (role, name_ink, role, palette.accent),
+        );
         let laid = name.layout(ui.ctx());
         let after_name = x + laid.width() + 8.0;
         laid.paint_left(ui.painter(), x, y);
@@ -1219,31 +1219,34 @@ fn result_row(
     }
 }
 
-/// Splits `name` into what stands before the run of it that matched
-/// `query`, that run, and what stands after. A contiguous, case-ignoring
-/// run only, and only where lowercasing leaves the bytes in place (ASCII
-/// names); a scattered or qualified query highlights nothing.
-fn highlight(name: &str, query: &str) -> [String; 3] {
-    let needle = query.trim();
-    let nothing = || [name.to_owned(), String::new(), String::new()];
-    if needle.is_empty() {
-        return nothing();
+/// `name` as text whose `spans` (byte ranges of it, in order, as the
+/// search found them) are in the second role and colour and the rest in
+/// the first. What is not shown safely is shown as `display_safe` writes
+/// it, unmarked.
+pub(crate) fn marked(
+    look: &Look,
+    name: &str,
+    spans: &[std::ops::Range<usize>],
+    (plain, plain_ink, hit, hit_ink): (TextRole, egui::Color32, TextRole, egui::Color32),
+) -> Text {
+    let safe = display_safe(name);
+    if safe != name {
+        return Text::one(look, plain, &safe, plain_ink);
     }
-    let lower = name.to_lowercase();
-    let found = lower.find(&needle.to_lowercase());
-    if lower.len() == name.len()
-        && let Some(at) = found
-    {
-        let end = at + needle.to_lowercase().len();
-        if name.is_char_boundary(at) && name.is_char_boundary(end) {
-            return [
-                name[..at].to_owned(),
-                name[at..end].to_owned(),
-                name[end..].to_owned(),
-            ];
+    let mut text = Text::new(look);
+    let mut at = 0;
+    for span in spans {
+        if span.start < at || span.end > name.len() {
+            continue;
         }
+        text = text.add(plain, &name[at..span.start], plain_ink).add(
+            hit,
+            &name[span.clone()],
+            hit_ink,
+        );
+        at = span.end;
     }
-    nothing()
+    text.add(plain, &name[at..], plain_ink)
 }
 
 /// The macOS footer under the matches: the keys that move among them, open
diff --git a/src/ui/workspace.rs b/src/ui/workspace.rs
index b53f1be..0285351 100644
--- a/src/ui/workspace.rs
+++ b/src/ui/workspace.rs
@@ -867,6 +867,13 @@ pub(super) fn database_menu(
 /// The connection bar: the connection's environment colour behind the open
 /// connections, and the way out.
 fn top_bar(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
+    // Quick open's Switch database asks for the chip's menu, as its key
+    // does: the menu opens as the chip is drawn.
+    if let Some(workspace) = app.workspace_mut(tab)
+        && std::mem::take(&mut workspace.database_menu_asked)
+    {
+        egui::Popup::open_id(ui.ctx(), database_menu_id(tab, false));
+    }
     let locale = app.locale;
     let palette = app.palette;
     let look = app.look;
````

- [ ] **Step 4: Run the checks**

Run: `cargo fmt --all --check && cargo clippy --locked --workspace --all-targets -- -D warnings && cargo test --locked --workspace --all-targets && RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps`

Expected: all pass, among them every `ui::quick_open_tests` and the keymap's generated tests. The lib suite:

```
fmt exit 0
clippy exit 0
test result: ok. 2995 passed; 0 failed; 20 ignored; 0 measured; 0 filtered out
doc exit 0
```

- [ ] **Step 5: Commit**

```bash
git add src/app/quick_open.rs src/ui/quick_open_tests.rs src/app.rs src/model.rs src/keymap.rs src/ui/quick_open.rs src/ui/start.rs src/ui/workspace.rs src/ui/keys.rs src/ui/cell_editor.rs src/env.rs src/ui/mod.rs src/ui/agents_tests.rs src/ui/start_tests.rs src/ui/complete_tests.rs
git commit -S -m "Make quick open the brief's panel: one index, grouped results, every open connection, in both looks"
```

---

### Task 3: The sidebar's field opens quick open, and the website

In the macOS and Windows looks the sidebar's "Find any object…" field becomes quick open's way in; the terminal look's `/` line still filters the tree. The website describes the new quick open.

**Files:** Modify `src/ui/sidebar.rs`, `src/ui/keys.rs`, `src/ui/quick_open_tests.rs`, `src/ui/mod.rs`, `docs/_guide/macos.md`, `docs/_guide/omarchy.md`, `docs/_guide/browsing-data.md`, `docs/_guide/getting-started.md`, `docs/_reference/keyboard-shortcuts.md`.

- [ ] **Step 1: Write the failing tests**

The brief's test 11, and the tests that reach the sidebar's field by its name (it is a button named "Find any object…" now, in the native looks).

````diff
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
index 3c6dad6..b4153e1 100644
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -350,13 +350,15 @@ mod tests {
                         look.name
                     );
                 }
+                // The terminal look's filter, or the others' way into
+                // quick open.
                 assert!(
                     tree.nodes.iter().any(|(_, node)| {
-                        node.role() == egui::accesskit::Role::TextInput
-                            && matches!(
-                                node.placeholder(),
-                                Some("Find any object…" | "filter objects")
-                            )
+                        let filter = node.role() == egui::accesskit::Role::TextInput
+                            && node.placeholder() == Some("filter objects");
+                        let find = node.role() == egui::accesskit::Role::Button
+                            && node.label() == Some("Find any object…");
+                        filter || find
                     }),
                     "the sidebar filter is missing in {} at {size:?}",
                     look.name
@@ -7335,12 +7337,13 @@ value_tags   = true
     /// Drags the sidebar's edge as far right as it goes.
     fn widen_sidebar(harness: &mut Harness) {
         let filter = |harness: &mut Harness| {
-            crate::testing::bounds(
-                &harness.settle(),
-                "Filter",
-                egui::accesskit::Role::TextInput,
-            )
-            .expect("the sidebar's filter")
+            let tree = harness.settle();
+            let filter = crate::testing::bounds(&tree, "Filter", egui::accesskit::Role::TextInput);
+            let find = || {
+                let role = egui::accesskit::Role::Button;
+                crate::testing::bounds(&tree, "Find any object…", role)
+            };
+            filter.or_else(find).expect("the sidebar's field")
         };
         let before = filter(harness);
         // The edge is a few points right of the filter field.
@@ -11877,7 +11880,7 @@ value_tags   = true
         }
         // The tree and the grid are one stop each: the arrows move in them.
         for expected in [
-            "Filter",
+            "Find any object…",
             "Objects",
             "Rows",
             "Count",
@@ -12027,6 +12030,13 @@ value_tags   = true
             let forward: Vec<String> = (0..6)
                 .map(|_| step(&mut harness, Modifiers::NONE))
                 .collect();
+            // The sidebar's field: the terminal look's filter, the
+            // others' way into quick open.
+            let field = if look.terminal {
+                "Filter"
+            } else {
+                "Find any object…"
+            };
             assert_eq!(
                 forward,
                 [
@@ -12034,7 +12044,7 @@ value_tags   = true
                     "Data",
                     "Rows",
                     "Connections",
-                    "Filter",
+                    field,
                     "Objects"
                 ],
                 "{}",
@@ -12044,7 +12054,7 @@ value_tags   = true
             let back: Vec<String> = (0..3)
                 .map(|_| step(&mut harness, Modifiers::SHIFT))
                 .collect();
-            assert_eq!(back, ["Filter", "Connections", "Rows"], "{}", look.name);
+            assert_eq!(back, [field, "Connections", "Rows"], "{}", look.name);
         }
     }
 
diff --git a/src/ui/quick_open_tests.rs b/src/ui/quick_open_tests.rs
index 6e087b9..e8df171 100644
--- a/src/ui/quick_open_tests.rs
+++ b/src/ui/quick_open_tests.rs
@@ -651,6 +651,41 @@ fn omarchy_ctrl_n_and_ctrl_p_move_the_pick() {
     assert!(matches!(harness.app.dialog, Some(Dialog::QuickOpen(_))));
 }
 
+/// Test 11: the macOS sidebar's search field opens quick open.
+#[test]
+fn the_sidebar_field_opens_quick_open() {
+    for look in [Look::macos(), Look::standard()] {
+        let (mut harness, _tab) = bookshop(look);
+        harness.settle();
+        harness.click("Find any object…");
+        assert!(
+            matches!(harness.app.dialog, Some(Dialog::QuickOpen(_))),
+            "{}",
+            look.name
+        );
+        harness.press(Key::Escape, Modifiers::NONE);
+        // With the keyboard on it, Enter opens quick open, and what is
+        // typed goes into quick open's field.
+        reach(&mut harness, "Find any object…");
+        harness.press(Key::Enter, Modifiers::NONE);
+        assert!(matches!(harness.app.dialog, Some(Dialog::QuickOpen(_))));
+        harness.press(Key::Escape, Modifiers::NONE);
+        reach(&mut harness, "Find any object…");
+        type_text(&mut harness, "pu");
+        type_text(&mut harness, "b");
+        assert_eq!(quick_open(&harness).query, "pub", "{}", look.name);
+    }
+    // The terminal look's `/` line filters the tree, as it did.
+    let (mut harness, tab) = bookshop(Look::omarchy());
+    harness.settle();
+    assert!(!harness.has("Find any object…"));
+    reach(&mut harness, "Objects");
+    super::tests::type_key(&mut harness, Key::Slash, "/");
+    type_text(&mut harness, "pub");
+    assert!(harness.app.dialog.is_none());
+    assert_eq!(harness.app.workspace(tab).unwrap().tree.filter, "pub");
+}
+
 /// The panel is a dialog named for the look; the field is a combobox
 /// that controls the list; the picked result is said with its kind, its
 /// schema and its estimate; headings are labels, not options.
````

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --locked -p tabletist --lib`

Expected: four tests fail, as the field still filters the tree:

```
test ui::quick_open_tests::the_sidebar_field_opens_quick_open ... FAILED
test ui::tests::every_look_lays_out_at_small_and_large_sizes ... FAILED
test ui::tests::f6_steps_through_the_parts_of_the_window ... FAILED
test ui::tests::tab_reaches_the_main_controls ... FAILED
test result: FAILED. 2992 passed; 4 failed; 20 ignored; 0 measured; 0 filtered out
```

- [ ] **Step 3: Make the field quick open's, and write the website**

````diff
diff --git a/docs/_guide/browsing-data.md b/docs/_guide/browsing-data.md
index e8fa928..7394259 100644
--- a/docs/_guide/browsing-data.md
+++ b/docs/_guide/browsing-data.md
@@ -11,9 +11,10 @@ that share a prefix, such as `book_`, fold into a group. You can also
 list them flat. On macOS and Windows, the objects you opened recently
 are listed above them.
 
-Quick open (Cmd+P on macOS, Ctrl+P on Windows) finds any object by name
-without the sidebar. The Omarchy look has no key for it: there `/` in the
-sidebar filters its list.
+Quick open (Cmd+P on macOS, Ctrl+P on Windows and Omarchy) finds any
+table or view by name without the sidebar, in this connection or in every
+open one. On macOS and Windows the sidebar's "Find any object…" field
+opens it; in the Omarchy look `/` in the sidebar filters its list.
 
 System schemas such as `pg_catalog` and `information_schema` are hidden.
 [A setting]({% link _reference/settings-and-files.md %}#the-settings-file)
diff --git a/docs/_guide/getting-started.md b/docs/_guide/getting-started.md
index 24dac4c..739d9ac 100644
--- a/docs/_guide/getting-started.md
+++ b/docs/_guide/getting-started.md
@@ -53,8 +53,8 @@ TLS, SSH tunnels and where passwords are kept are covered in
 - **The sidebar** lists the tables and views of one schema. On macOS and
   Windows the ones you opened recently are on top. Cmd/Ctrl+B shows or
   hides it.
-- **Quick open** (Cmd/Ctrl+P, on macOS and Windows) finds any table or
-  view by name.
+- **Quick open** (Cmd/Ctrl+P) finds any table or view by name, from
+  anywhere in the window.
 - **Tabs** hold the tables and SQL editors you have open.
 - **The row panel** on the right shows every field of the selected row.
   Cmd/Ctrl+Shift+R shows or hides it.
diff --git a/docs/_guide/macos.md b/docs/_guide/macos.md
index 5c28a8c..1eaf75e 100644
--- a/docs/_guide/macos.md
+++ b/docs/_guide/macos.md
@@ -85,11 +85,26 @@ a Mac keyboard labels Delete.
 
 ### Quick open
 
+Quick open floats over the window, from anywhere in it: the rows, the
+SQL editor, the start screen or the Agents tab. The sidebar's "Find any
+object…" field opens it too. With nothing typed it lists what the start
+screen lists (what you opened lately), then three commands: New SQL
+query, Switch database (where the server has another) and Agents. Typing
+lists the tables and then the views that match, three of each and a row
+that shows the rest, and the commands whose names match. The best
+matches come first: the whole name, then its start, then the starts of
+its words (`bc` finds `book_covers`), then any run of its letters; what
+you opened lately comes first among equals. Nothing is asked of the
+database while you type. The chip beside the field says what is
+searched; Tab searches every open connection, each under its name.
+
 | Keys | Do |
 | --- | --- |
-| Cmd+P | Open it, then type to find a table or view |
+| Cmd+P | Open it. While it is open, select what is typed |
 | Up, Down | Move in the list |
-| Return | Open the one selected |
+| Return | Open the one picked, or show its tab where it has one |
+| Option+Return | Open a new SQL editor reading its first rows |
+| Tab | Search every open connection, or this one again |
 | Esc | Close |
 
 ### The picker
diff --git a/docs/_guide/omarchy.md b/docs/_guide/omarchy.md
index f652238..04887cd 100644
--- a/docs/_guide/omarchy.md
+++ b/docs/_guide/omarchy.md
@@ -43,8 +43,20 @@ back.
 | `:agents log` | Show the Agents tab's log |
 | `?` | The list of keys |
 
-Quick open has no key in the Omarchy look. `/` in the sidebar filters its
-list instead.
+Ctrl+P opens quick open from any pane: a picker at the top of the
+window that finds a table or view by name, in this connection or (Tab)
+in every open one. With nothing typed it lists what you opened lately,
+numbered; once you type, it lists what matches. Its commands stay on `:`.
+`/` in the sidebar still filters the sidebar's list.
+
+| Keys | Do |
+| --- | --- |
+| `Ctrl+P` | Open it |
+| `Ctrl+N`, `Ctrl+P`, Down, Up | Move in the list |
+| `Enter` | Open the one picked, or show its tab where it has one |
+| `1` to `9` | With nothing typed: open that recent. Once you type, digits are typed |
+| `Tab` | Search every open connection, or this one again |
+| `Esc` | Close |
 
 ### The picker
 
diff --git a/docs/_reference/keyboard-shortcuts.md b/docs/_reference/keyboard-shortcuts.md
index 91e05ef..efdca98 100644
--- a/docs/_reference/keyboard-shortcuts.md
+++ b/docs/_reference/keyboard-shortcuts.md
@@ -50,7 +50,7 @@ In the connection dialog:
 
 | Keys | Do |
 | --- | --- |
-| Mod+P | Quick open. In it, Up and Down move, Enter opens and Esc closes |
+| Mod+P | Quick open (Ctrl+P in the Omarchy look too). In it, Up and Down move, Enter opens, Tab searches every open connection and Esc closes. On macOS and Windows, Alt+Enter opens a SQL editor reading the rows |
 | Mod+B | Show or hide the sidebar |
 | Mod+Alt+A | Show the Agents tab |
 | Mod+W | Close the tab |
diff --git a/src/ui/keys.rs b/src/ui/keys.rs
index 7dfb204..0eb90a4 100644
--- a/src/ui/keys.rs
+++ b/src/ui/keys.rs
@@ -581,7 +581,8 @@ pub fn handle(app: &mut App, ctx: &egui::Context) {
     // keyboard is: a character typed with no text field holding the
     // keyboard asks for the search field, and is left in the frame for it
     // to read. So the keyboard can stay in the sidebar until a letter is
-    // typed, and the sidebar's own filter keeps what is typed into it.
+    // typed. The sidebar's search field keeps what is typed on it: it is
+    // quick open's way in, and takes it there.
     // Keys held with Cmd, Ctrl or Alt are shortcuts, and bring no
     // character. A space alone is no search: Space presses the focused
     // button or row, and a real keyboard sends it as text too. Nor is a
@@ -590,6 +591,7 @@ pub fn handle(app: &mut App, ctx: &egui::Context) {
     if on_start_screen
         && !app.look.terminal
         && !ctx.text_edit_focused()
+        && !ctx.memory(|memory| memory.has_focus(crate::ui::sidebar::find_field_id(active)))
         && typed_chars(ctx)
             .into_iter()
             .any(|char| !char.is_whitespace() && window_letter(app, scopes, char).is_none())
diff --git a/src/ui/sidebar.rs b/src/ui/sidebar.rs
index ee782b2..058ca33 100644
--- a/src/ui/sidebar.rs
+++ b/src/ui/sidebar.rs
@@ -33,6 +33,99 @@ fn field_height(look: &Look) -> f32 {
     if look.terminal { 32.0 } else { 36.0 }
 }
 
+/// Where the macOS and Windows sidebar's search field keeps the keyboard,
+/// by window.
+pub(crate) fn find_field_id(tab: ConnTabId) -> Id {
+    Id::new(("sidebar-find", tab.0))
+}
+
+/// The macOS and Windows sidebar's search field, which is quick open's
+/// way in: a box like the filter's, with the magnifier, "Find any
+/// object…" and quick open's key at its right. A click, or Space or Enter
+/// while it has the keyboard, opens quick open (the response is clicked);
+/// what is typed while it has the keyboard is returned, for quick open's
+/// field. A button to a screen reader, named as it reads.
+fn find_field(
+    ui: &mut egui::Ui,
+    tab: ConnTabId,
+    size: egui::Vec2,
+    key: &Written,
+    (look, palette, locale): (&Look, &Palette, Locale),
+) -> (egui::Response, Option<String>) {
+    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
+    let response = ui.interact(rect, find_field_id(tab), Sense::click());
+    let name = gettext(locale, "Find any object…");
+    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, &name));
+    focus::region(ui, focus::Region::Search, rect.expand(8.0));
+    focus::claim(ui, focus::Region::Search, &response);
+    let corner = CornerRadius::same(look.radius);
+    focus::hint(
+        ui,
+        &response,
+        rect,
+        focus::Ring::Field { radius: corner.nw },
+    );
+    let painter = ui.painter();
+    painter.rect_filled(rect, corner, palette.window);
+    let border = if response.hovered() {
+        palette.outline
+    } else {
+        palette.border
+    };
+    painter.rect_stroke(
+        rect,
+        corner,
+        egui::Stroke::new(widgets::hairline(ui), border),
+        egui::StrokeKind::Inside,
+    );
+    let y = rect.center().y;
+    let icon = Rect::from_center_size(pos2(rect.left() + 11.0 + 7.0, y), vec2(14.0, 14.0));
+    Icon::Search.image(palette.dim, 14.0).paint_at(ui, icon);
+    let role = TextRole::UiBody;
+    let right = rect.right() - 10.0;
+    let shortcut = || Text::one(look, TextRole::Shortcut, key.as_str(), palette.dim);
+    let key_width = if key.as_str().is_empty() {
+        0.0
+    } else {
+        widgets::paint_text_right(ui, right, y, shortcut()) + 8.0
+    };
+    let left = rect.left() + 11.0 + 14.0 + 8.0;
+    let room = (right - key_width - left).max(0.0);
+    let shown = crate::ui::grid::ellipsize(&name, room, false, |text| {
+        role.width(ui.ctx(), look.faces, text)
+    });
+    widgets::paint_text(ui, left, y, Text::one(look, role, &shown, palette.dim));
+    // Space and Enter press it while it has the keyboard, as a button.
+    let pressed = response.has_focus()
+        && ui.input_mut(|input| {
+            input.consume_key(egui::Modifiers::NONE, egui::Key::Enter)
+                || input.consume_key(egui::Modifiers::NONE, egui::Key::Space)
+        });
+    // What is typed on it is a search: it goes to quick open's field. A
+    // space alone is the press above.
+    let typed = response
+        .has_focus()
+        .then(|| {
+            ui.input_mut(|input| {
+                let mut typed = String::new();
+                input.events.retain(|event| match event {
+                    egui::Event::Text(text) if !text.trim().is_empty() => {
+                        typed.push_str(text);
+                        false
+                    }
+                    _ => true,
+                });
+                typed
+            })
+        })
+        .filter(|typed| !typed.is_empty());
+    let mut response = response;
+    if pressed {
+        response.flags |= egui::response::Flags::CLICKED;
+    }
+    (response, typed)
+}
+
 /// The Reload button's spinner, and a loading row's.
 const RELOAD_SPINNER: f32 = 14.0;
 const ROW_SPINNER: f32 = 12.0;
@@ -179,13 +272,19 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
             ui.add_space(pad);
             ui.horizontal(|ui| {
                 ui.add_space(pad);
-                if let Some(workspace) = app.workspace_mut(tab) {
-                    let hint = if look.terminal {
-                        gettext(locale, "filter objects")
-                    } else {
-                        gettext(locale, "Find any object…")
-                    };
-                    let size = vec2(full.width() - 2.0 * pad, field_height(&look));
+                let size = vec2(full.width() - 2.0 * pad, field_height(&look));
+                // The macOS and Windows field is quick open's: it opens
+                // it. The terminal look's `/` line filters the tree.
+                if !look.terminal {
+                    let key = crate::ui::keys::written(ui.ctx(), &look, Command::FindObject);
+                    let (field, typed) = find_field(ui, tab, size, &key, (&look, &palette, locale));
+                    if let Some(typed) = typed {
+                        actions.push(Action::QuickOpenTyped(typed));
+                    } else if field.clicked() {
+                        actions.push(Action::OpenQuickOpen);
+                    }
+                } else if let Some(workspace) = app.workspace_mut(tab) {
+                    let hint = gettext(locale, "filter objects");
                     let style = widgets::FieldStyle {
                         fill: Some(palette.window),
                         boxed: true,
````

- [ ] **Step 4: Run the checks**

Run: `cargo fmt --all --check && cargo clippy --locked --workspace --all-targets -- -D warnings && cargo test --locked --workspace --all-targets && RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps`

Expected: all pass. The lib suite:

```
fmt exit 0
clippy exit 0
test result: ok. 2996 passed; 0 failed; 20 ignored; 0 measured; 0 filtered out
doc exit 0
```

- [ ] **Step 5: Commit**

```bash
git add src/ui/sidebar.rs src/ui/keys.rs src/ui/quick_open_tests.rs src/ui/mod.rs docs/_guide/macos.md docs/_guide/omarchy.md docs/_guide/browsing-data.md docs/_guide/getting-started.md docs/_reference/keyboard-shortcuts.md
git commit -S -m "Open quick open from the sidebar's field, and describe it on the website"
```

## The pull request

One pull request for the run. Its description holds:
- a summary;
- that the shared index was built here (the start screen's run 2 used quick open's old one, and moves onto this);
- the keys added and the keymap check (the master plan's table);
- the brief's open questions and the defaults taken (the master plan);
- the departures (the master plan's and this plan's tables).

No screenshot goes to GitHub unblurred.
