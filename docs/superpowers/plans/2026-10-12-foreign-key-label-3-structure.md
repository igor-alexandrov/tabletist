# Foreign Key Label, Run 3: The Structure Door Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** In the Structure view of a table the user sees what labels its rows for the foreign keys that point at it, and changes it there with no cell edited and on a read-only connection: with the same chooser, with the column under the view's cursor, with a column's menu, or with a line of the Omarchy prompt.

**Architecture:** The Structure view gets what it never had: a cursor on the lines of its columns (`ObjectTab::structure_cursor`), and lines the pointer can click and open a menu on. The chooser of run 2 gets a second home, on the table's tab (`ObjectTab::label_chooser`), and `src/app/labels.rs` serves both homes with the same functions and the same actions. The rule in `src/labels.rs` is asked about a table's own keys (its primary key) where a picker asks about the one column its foreign key references, and says why a column named by hand is refused. The new keys are the grid's keys while the Structure view shows (`When::Structure`): that view is the table's tab, and the grid's lines of the prompt work in it today.

**Tech Stack:** Rust 1.98, egui (the crmne fork), AccessKit for the headless tests (`src/testing.rs`). Spec: `docs/superpowers/specs/2026-10-12-foreign-key-label-design.md` (section 4, the "only a key" state of section 5 for the Structure line, section 6's Structure line; tests 5, 6, 12 and 15). Runs 1 and 2 are built: this plan stands on `labels::{Chooser, Line, lines, resolve, excluded}`, `App::set_label`, `ui::label_chooser` and the actions of run 2's chooser.

---

## Read this first

### What was run where this plan was written

Everything in it. The whole run was built first, then replayed as the four tasks below, and each task's tree was checked alone: `cargo fmt --check`, `cargo clippy --workspace --all-targets -D warnings`, clippy with the `shots` feature (tasks 3 and 4), the app's tests and `tests/keys.rs`. The last tree passed the four checks whole: 2966 tests across the workspace, none failed. The code blocks and diffs below are those trees, not a retelling. Their line numbers are of the tree at `89d721f`.

The plan's reviewer read the code against the spec and the boards. What it found is built in: the Structure view is a pane the keyboard can be brought to (it was reachable by the pointer alone), a refusal is said in its own tab and not under another table, the two buttons stand in the board's order, the Omarchy line and status line have the board's words, the cursor stays inside the columns when a structure comes again, and the label's line is a group for a screen reader. A second reading found that the view's cursor was never brought into view: it is now, when it moves, as the tree's and the rows' are, and so is a refusal that is said at the view's top. It also named what still differed from the macOS board to the eye, and both are built: the label's part is the board's card with its buttons at the card's end, and the column's tag has the board's blue. A third reading approved the plan and named two places where the cursor's line could still be out of sight; both are closed: each tab's Structure view keeps its own place, and the cursor is brought into view when the keys come back to a view that was scrolled meanwhile.

Not run:

- **A real window.** Nothing was pressed by hand. The Structure view was rendered headless in the three looks and looked at: the label's card and line, the tag on the column, the cursor's line, the chooser under what opened it, the Omarchy status line and a refusal (task 3, step 9 says how).
- **PostgreSQL and MySQL.** Nothing in this run talks to a database at all: a choice made here is a file on this computer.

### The keys were checked against the keymap

The keymap's collision test passes with the keys of this run in place (task 1; 21 tests of the keymap). They are bound in the scope `grid` under a new condition, `When::Structure`.

That test compares bindings that hold under the same condition, so it sets these six against each other and not against the grid's own. Against those they were checked by reading: `cmd+l`, `alt+cmd+l`, `gl` and the `:label` lines are bound nowhere else that the grid or the global scope reads. `down` is also the key of "add a row at the end" under the condition "the cursor is on the last row", which is reserved and answers nothing; when it is built it must not hold in the Structure view.

| Command | macOS | Omarchy |
|---|---|---|
| Open the chooser | `cmd+l` | `:label` |
| Use the column under the cursor as the label | `alt+cmd+l` | `gl` |
| Back to Automatic | the Use Automatic button | `:label auto` |
| Keys only | the chooser | `:label none` |
| A column by its name | the chooser | `:label {column}` |
| Move the view's cursor | `up` `down` | `j` `k` `up` `down` |

### Where the brief, the boards and the app differ

| The brief or the design asks | What is built | Why |
|---|---|---|
| The keys' scope is `structure` | The scope is `grid`, with the condition "the Structure view shows" | The Structure view is the table's tab. The grid's own keys and lines work in it today (`:w` and `:e!` from the Structure view are tested), and a tenth scope would take them away. A binding with a condition takes its key before the same key without one, so `j` and `k` are the view's cursor there and the rows' elsewhere |
| `:label {column}` | The keymap spells the command so, and the prompt reads a line with a word after a command's name. Lines spelled whole are tried first: `:label auto` is always Automatic | The prompt compared a whole line with a binding, and only `:N` was read otherwise. A column called `auto` or `none` can be chosen in the chooser |
| "The focused column", "the column under the cursor" | The view gets a cursor on its columns' lines. It shows while the view has the keys: after a click on a line, or when the keyboard comes to it as it comes to the rows (Tab, F6, Omarchy's pane to the right): the columns are one Tab stop, in the rows' place among the panes. The arrows move the cursor in every look, `j` and `k` in Omarchy | The view had no cursor, no line that takes a click and no menu |
| (what the keys did in a Structure view before) | While the view has the keys, the arrows, Enter, Home and End are its own or nobody's, and Reload reloads the table and its structure. With the keys on the tree all of them are the tree's, as before | In a Structure view all of these went to the tree whatever had the keys, Reload reloaded the tree, and Omarchy's `j` and `k` moved the rows' selection, which the view does not show |
| Omarchy: a column of notes (`pk · identity`, `◂ label`, `n/a, binary`), the help sentence at the pane's foot, a status line with `-- NORMAL --` and the cursor's column | `◂ label` beside the column's name; the sentence under the label's line; the view keeps its five columns (Name, Type, Nullable, Default, Key). The status line names the view's keys: `j/k column`, then the board's `gl use as label`, `:label choose` and `:label auto`, and it keeps the key that shows the tables, `:w write` and the review, which the board's line does not have. It has no mode word and no cursor's column | The view is one for every look; the boards' Omarchy pane is another table. Saving and the tables key work from this view, and a test of today holds `:w write` to its line. The app's line has no mode word and no place out of insert mode; the cursor's line is kept in view instead |
| A refusal "reports the reason" | "{column} cannot be the label (the key)", "… (binary)", "No column {column} in {table}". The Omarchy status line says it until the next key; the other looks say it under the label's line until the cursor moves. It is said in the tab whose table it is of, and in no other | The notes board has the reasons ("the key", "binary") and no sentence |
| `:label auto`, `:label none` | Read as spelled, with one space. `:label  auto` with two is read as a column called `auto`, and refused | The prompt compares a line with a binding as it stands; it trims only the line's ends |
| The "Label for foreign keys" line | It shows for every table, not only for one a key points at. A view has none | Which tables are pointed at is not known without reading every table's keys. No key points at a view |
| A column's menu: Use as label | Use as label, with its key, and Copy name, as the board draws it | |
| macOS: the card, its buttons and the `label` tag | Built as drawn, from what the app has: the row panel's card (`panel` under a `surface_hover` border, the look's radius), and the value tags' blue for the tag, which is the board's colour to the digit. The card is as wide as the view's window. The sentence wraps beside the buttons where the window is narrow | No colour and no style is new |

**The optional door is not built**: `Label {table} by…` in the header menu of a foreign key column of the grid, and `:label` on a foreign key cell. A grid column's header has no menu today, and `:label` over the rows would need the picker's chooser without a picker. Run 4 lists it as skipped in the pull request.

### The Bookshop fixture

One column is added to `src/testing.rs`, for the pull request's list: `editions` gets `cover BLOB`, as the screenshot scenes' `editions` has `cover bytea`. With it the chooser of `editions` lists a column of bytes, which run 2 could not show.

### For run 4

- The website: `docs/_reference/keyboard-shortcuts.md` and the two guides follow the keymap (`AGENTS.md`), and the Structure view's pictures in `assets/screenshots/` show a view that has changed.
- The pull request's lists: the boards, the keys of runs 2 and 3, the tests, the optional door (skipped), the placeholders replaced (`isbns`, `editions` with `isbn`, `title` and `cover`; `legal_name` as the name of a column that is gone; the database tests' `editions`).

---

## Before you start

- Cargo is `~/.cargo/bin/cargo` (the mise shim fails). Never point `CARGO_TARGET_DIR` at `/tmp`.
- The four checks, from `AGENTS.md`. "Run the four checks" below means these, and all four pass after every task:

```bash
~/.cargo/bin/cargo fmt --all --check
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
~/.cargo/bin/cargo test --locked --workspace --all-targets
RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --locked --workspace --no-deps
```

- `src/shots.rs` is outside those four and draws a Structure view. Tasks 3 and 4 also run `~/.cargo/bin/cargo clippy --locked --features shots --lib --tests -- -D warnings`.
- Run `~/.cargo/bin/cargo fmt --all` before each task's checks.
- **No pixels.** The tests check behaviour, names and words through the AccessKit tree and the commands sent. No snapshot is added and no picture is committed.
- House rules that bite here: views push `Action`s and change nothing; a key is spelled only in `src/keymap.rs`; text is drawn through `TextRole`s; strings go through `gettext`; code that depends on the engine names every variant; no em dashes.
- Commits are signed, one per task, after its checks pass. If signing fails ("agent refused operation": 1Password is locked), do not commit unsigned: stage the task and tell the user. Each message ends with:

      Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>

- A diff below applies with `git apply`. Where the tree has moved, find the place by the names around it.

## File structure

| File | What it is for |
|---|---|
| `src/keymap.rs` | `When::Structure`, the six commands and their keys; a line of the prompt that takes a word (`ex`, `ex_argument`) |
| `src/labels.rs` | The rule by a table's keys (`automatic`, `resolve` take them all); `Refused` and `choosable`, for a column named by hand |
| `src/model.rs` | The tab's cursor and its chooser; the workspace's note of a refusal; five actions; `label_lines` by keys |
| `src/app/labels.rs` | The chooser's two homes; `structure_label`; the cursor; `use_as_label`, `label_by_name`, `set_tab_label` |
| `src/app.rs`, `src/app/editing.rs` | The arms; the prompt's lines |
| `src/ui/keys.rs` | The view's chords and letters, and its chooser's keys |
| `src/ui/label_chooser.rs`, `src/ui/key_picker.rs` | The chooser stands beside a list or under what opened it, with a field or with none |
| `src/ui/structure.rs` | The label's line, the tag, the cursor, a line's menu, the chooser |
| `src/ui/data_view.rs` | One call follows `label_lines` |
| `src/ui/workspace.rs` | The Omarchy status line: the view's keys, and a refusal |
| `src/testing.rs` | `editions.cover` |
| `src/ui/label_tests.rs` | The headless tests |

---

### Task 1: The keys, in the keymap

**Files:**
- Modify: `src/keymap.rs`

Spec test 15. The commands are data until task 3 reads them. Two things are new in the keymap's own code: a condition, and a line of the prompt that takes a word. `ex` tries the lines spelled whole first and only then the ones that take a word, so `:label auto` and `:label none` are never columns; `ex_argument` gives the word.

- [ ] **Step 1: Write the failing test**

```diff
diff --git a/src/keymap.rs b/src/keymap.rs
--- a/src/keymap.rs
+++ b/src/keymap.rs
@@ -2900,6 +2900,69 @@ mod tests {
         );
     }
 
+    /// The keys of the Structure view's door to a foreign key's label, as
+    /// its brief has them
+    /// (`docs/superpowers/specs/2026-10-12-foreign-key-label-design.md`,
+    /// test 15). They are the grid's keys while the Structure view shows.
+    #[test]
+    fn the_structure_views_label_keys_are_the_briefs() {
+        let keymap = Keymap::default();
+        let grid = Scope::Grid;
+        let chords =
+            |layout, command| -> Vec<&str> { keymap.chords(layout, command, grid).collect() };
+        use Command::{
+            LabelAutomatic, LabelColumn, LabelKeysOnly, MoveInStructure, OpenLabelChooser,
+            UseAsLabel,
+        };
+        for layout in [Layout::Mac, Layout::Windows] {
+            assert_eq!(chords(layout, OpenLabelChooser), ["cmd+l"]);
+            assert_eq!(chords(layout, UseAsLabel), ["alt+cmd+l"]);
+            assert_eq!(chords(layout, MoveInStructure), ["up", "down"]);
+            // A button and the chooser do these: no key.
+            for command in [LabelAutomatic, LabelKeysOnly, LabelColumn] {
+                assert!(chords(layout, command).is_empty(), "{command:?}");
+            }
+        }
+        let omarchy = Layout::Omarchy;
+        assert_eq!(chords(omarchy, OpenLabelChooser), [":label"]);
+        assert_eq!(chords(omarchy, UseAsLabel), ["gl"]);
+        assert_eq!(chords(omarchy, LabelAutomatic), [":label auto"]);
+        assert_eq!(chords(omarchy, LabelKeysOnly), [":label none"]);
+        assert_eq!(chords(omarchy, LabelColumn), [":label {column}"]);
+        // Every one of them holds only while the Structure view shows.
+        let structure = |when: When| when == When::Structure;
+        let rows = |_: When| false;
+        let command = Typed::Command;
+        let typed = |holds: &dyn Fn(When) -> bool, keys| keymap.typed(omarchy, grid, holds, keys);
+        assert_eq!(typed(&structure, "gl"), command(UseAsLabel));
+        assert_eq!(typed(&rows, "gl"), Typed::Nothing);
+        // `j` and `k` are the view's there, and the rows' elsewhere.
+        assert_eq!(typed(&structure, "j"), command(MoveInStructure));
+        assert_eq!(typed(&rows, "j"), command(Command::MoveRow));
+        // The grid's own `g` keys stay.
+        assert_eq!(typed(&structure, "gd"), command(Command::OpenReferencedRow));
+        let ex = |holds: &dyn Fn(When) -> bool, line| keymap.ex(omarchy, grid, holds, line);
+        assert_eq!(ex(&structure, "label"), command(OpenLabelChooser));
+        assert_eq!(ex(&structure, "label auto"), command(LabelAutomatic));
+        assert_eq!(ex(&structure, "label none"), command(LabelKeysOnly));
+        assert_eq!(ex(&structure, "label legal_name"), command(LabelColumn));
+        assert_eq!(ex(&rows, "label"), Typed::Nothing);
+        assert_eq!(ex(&rows, "label legal_name"), Typed::Nothing);
+        // The grid's lines are still its own there.
+        assert_eq!(ex(&structure, "w"), command(Command::SaveChanges));
+        // The word a line gives the command that takes one.
+        let given = |line| keymap.ex_argument(omarchy, LabelColumn, grid, line);
+        assert_eq!(given("label legal_name"), Some("legal_name"));
+        assert_eq!(given("label  first name "), Some("first name"));
+        assert_eq!(given("label"), None);
+        assert_eq!(given("label "), None);
+        assert_eq!(given("labels x"), None);
+        assert_eq!(
+            keymap.ex_argument(omarchy, OpenLabelChooser, grid, "label x"),
+            None
+        );
+    }
+
     #[test]
     fn two_keys_that_do_opposite_things_are_written_as_one_hint() {
         let keymap = Keymap::default();
```

- [ ] **Step 2: Run it to see it fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib keymap::`
Expected: it does not compile: `Command` has no `MoveInStructure`, `OpenLabelChooser`, `UseAsLabel`, `LabelAutomatic`, `LabelKeysOnly` or `LabelColumn`, `When` no `Structure`, and the keymap no `ex_argument`.

- [ ] **Step 3: The condition, the commands and their keys, and the line that takes a word**

```diff
diff --git a/src/keymap.rs b/src/keymap.rs
--- a/src/keymap.rs
+++ b/src/keymap.rs
@@ -149,6 +149,10 @@ pub enum When {
     /// The chooser of a table's label column is open: beside a picker's
     /// list, which gives it the keys.
     LabelChooser,
+    /// The Structure view of a table shows, in place of its rows: the
+    /// grid's keys that act on a row have nothing to act on, and the
+    /// view's own act on the line of a column.
+    Structure,
 }
 
 /// The mode a chord holds in: with the keyboard in a text field (insert),
@@ -390,6 +394,12 @@ commands! {
     DeleteRow,
     OpenReferencedRow,
     ShowSavedRow,
+    MoveInStructure,
+    OpenLabelChooser,
+    UseAsLabel,
+    LabelAutomatic,
+    LabelKeysOnly,
+    LabelColumn,
     ReviewSql,
     NextChange,
     CopyReviewSql,
@@ -623,6 +633,12 @@ impl Command {
                 Editing,
             )
             .reserved(),
+            C::MoveInStructure => info("Move by column in the Structure view", "", Editing),
+            C::OpenLabelChooser => info("Choose the label for foreign keys", "", Editing),
+            C::UseAsLabel => info("Use the column as the label", "", Editing),
+            C::LabelAutomatic => info("Label foreign keys automatically", "", Editing),
+            C::LabelKeysOnly => info("Label foreign keys by their keys alone", "", Editing),
+            C::LabelColumn => info("Label foreign keys by a column", "", Editing),
             C::ReviewSql => info("Review pending SQL", "Review pending SQL", Editing),
             C::NextChange => info("Next change", "Review pending SQL", Editing).reserved(),
             C::CopyReviewSql => info("Copy the pending SQL", "", Editing),
@@ -1223,6 +1239,53 @@ pub const BINDINGS: &[Binding] = {
         bind(C::AddRow, INSPECTOR, NONE, keys(&[k("o")])).when(When::NewRow),
         bind(C::OpenReferencedRow, GRID, NONE, keys(&[k("gd")])),
         bind(C::ShowSavedRow, GRID, NONE, keys(&[k("gs")])),
+        // The Structure view of a table, which is the grid's tab with its
+        // columns in the rows' place. Its cursor is on the line of a
+        // column, and what labels the table's rows for the foreign keys
+        // that point at it is chosen here: with the chooser, by the column
+        // under the cursor, or by a line of the prompt.
+        bind(
+            C::MoveInStructure,
+            GRID,
+            keys(&[k("up"), k("down")]),
+            keys(&[k("j"), k("k"), k("up"), k("down")]).shown("j/k"),
+        )
+        .when(When::Structure),
+        bind(
+            C::OpenLabelChooser,
+            GRID,
+            keys(&[k("cmd+l")]),
+            keys(&[k(":label")]),
+        )
+        .when(When::Structure),
+        bind(
+            C::UseAsLabel,
+            GRID,
+            keys(&[k("alt+cmd+l")]),
+            keys(&[k("gl")]),
+        )
+        .when(When::Structure),
+        bind(
+            C::LabelAutomatic,
+            GRID,
+            control("Use Automatic button"),
+            keys(&[k(":label auto")]),
+        )
+        .when(When::Structure),
+        bind(
+            C::LabelKeysOnly,
+            GRID,
+            control("label chooser"),
+            keys(&[k(":label none")]),
+        )
+        .when(When::Structure),
+        bind(
+            C::LabelColumn,
+            GRID,
+            control("label chooser"),
+            keys(&[k(":label {column}")]),
+        )
+        .when(When::Structure),
         bind(
             C::ReviewSql,
             GRID,
@@ -1604,6 +1667,20 @@ pub const BINDINGS: &[Binding] = {
     ]
 };
 
+/// What `line` of the `:` prompt gives the command spelled `name`, where
+/// the spelling ends in a word in braces (`label {column}`): what follows
+/// the command's own words and a space, without the white space round it.
+/// None for a spelling that takes nothing, for another command's line and
+/// for a line with nothing after the name.
+fn argument<'a>(name: &str, line: &'a str) -> Option<&'a str> {
+    let (head, word) = name.rsplit_once(' ')?;
+    if !(word.starts_with('{') && word.ends_with('}')) {
+        return None;
+    }
+    let given = line.strip_prefix(head)?.strip_prefix(' ')?.trim();
+    (!given.is_empty()).then_some(given)
+}
+
 /// The modifiers of a chord, as a spelling names them.
 #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
 pub struct Mods {
@@ -2136,7 +2213,11 @@ impl Keymap {
     }
 
     /// What a line of the `:` prompt (`line`, without its colon) comes to
-    /// in `scope`. A line of digits is the one spelled `:N`.
+    /// in `scope`. A line of digits is the one spelled `:N`. A line that
+    /// is no command as it stands is read as one that takes a word after
+    /// its name, where a command is spelled so (`:label {column}`): the
+    /// lines spelled whole come first, so `:label auto` is never a column
+    /// called `auto`.
     pub fn ex(
         &self,
         layout: Layout,
@@ -2145,7 +2226,7 @@ impl Keymap {
         line: &str,
     ) -> Typed {
         let number = !line.is_empty() && line.chars().all(|char| char.is_ascii_digit());
-        self.read(
+        let whole = self.read(
             layout,
             scope,
             holds,
@@ -2154,9 +2235,37 @@ impl Keymap {
                 Spelled::Strokes(_) | Spelled::Digits(_) => false,
             },
             &|_| false,
+        );
+        if whole != Typed::Nothing {
+            return whole;
+        }
+        self.read(
+            layout,
+            scope,
+            holds,
+            &|spelled| match spelled {
+                Spelled::Ex(name) => argument(name, line).is_some(),
+                Spelled::Strokes(_) | Spelled::Digits(_) => false,
+            },
+            &|_| false,
         )
     }
 
+    /// The word a line of the `:` prompt gives `command`, where the
+    /// command is spelled as taking one in `scope` (`:label {column}`) and
+    /// `line` is that command with something after its name.
+    pub fn ex_argument<'a>(
+        &self,
+        layout: Layout,
+        command: Command,
+        scope: Scope,
+        line: &'a str,
+    ) -> Option<&'a str> {
+        self.chords(layout, command, scope)
+            .filter_map(|keys| keys.strip_prefix(':'))
+            .find_map(|name| argument(name, line))
+    }
+
     /// The first key of `command` as `layout` writes it, for a hint beside
     /// a control. Empty where it has none.
     pub fn key(&self, layout: Layout, command: Command) -> Written {
```

- [ ] **Step 4: Run the keymap's tests**

Run: `~/.cargo/bin/cargo fmt --all && ~/.cargo/bin/cargo test --locked -p tabletist --lib keymap::`
Expected: `test result: ok. 21 passed`. If `no_two_commands_share_a_key_where_both_could_be_read` fails, **stop and report it**: the brief says not to pick another key.

- [ ] **Step 5: Run the four checks**

Expected: all pass.

- [ ] **Step 6: Commit**

```bash
git add src/keymap.rs
git commit -S -m "Give the Structure view's door to a foreign key's label its keys

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: The rule, by a table's own keys

**Files:**
- Modify: `src/labels.rs`, `src/app/labels.rs`

A picker asks the rule about one key: the column its foreign key references. A table's Structure view asks about the table's primary key, which may be no column or several. `automatic` and `resolve` take the keys as a list. `choosable` is for a column named by hand, which no chooser's line vouched for.

- [ ] **Step 1: Write the failing tests**

```diff
diff --git a/src/labels.rs b/src/labels.rs
--- a/src/labels.rs
+++ b/src/labels.rs
@@ -450,7 +450,7 @@ mod tests {
     }
 
     fn resolved(choice: &LabelChoice, columns: &[ColumnInfo]) -> Resolved {
-        resolve(choice, Dialect::Sqlite, columns, "id")
+        resolve(choice, Dialect::Sqlite, columns, &["id"])
     }
 
     #[test]
@@ -464,14 +464,14 @@ mod tests {
             column("email", "text"),
         ];
         assert_eq!(
-            automatic(Dialect::Postgres, &columns, "code"),
+            automatic(Dialect::Postgres, &columns, &["code"]),
             Some("name".into())
         );
         assert_eq!(
-            automatic(Dialect::Postgres, &columns, "founded"),
+            automatic(Dialect::Postgres, &columns, &["founded"]),
             Some("code".into())
         );
-        assert_eq!(automatic(Dialect::Postgres, &columns[1..2], "id"), None);
+        assert_eq!(automatic(Dialect::Postgres, &columns[1..2], &["id"]), None);
     }
 
     #[test]
@@ -502,7 +502,7 @@ mod tests {
     fn a_table_of_its_key_alone_has_nothing_to_choose() {
         let isbns = [column("isbn", "TEXT")];
         for choice in [LabelChoice::Automatic, LabelChoice::NoLabel, chosen("isbn")] {
-            let resolved = resolve(&choice, Dialect::Sqlite, &isbns, "isbn");
+            let resolved = resolve(&choice, Dialect::Sqlite, &isbns, &["isbn"]);
             assert_eq!(
                 (resolved.column, resolved.source),
                 (None, Source::KeysOnly),
@@ -564,7 +564,7 @@ mod tests {
         }
         // Chosen where another key references it: this picker lists keys,
         // and the choice stays the user's.
-        let resolved = resolve(&chosen("name"), Dialect::Sqlite, &genres(), "name");
+        let resolved = resolve(&chosen("name"), Dialect::Sqlite, &genres(), &["name"]);
         assert_eq!((resolved.column, resolved.source), (None, Source::User));
     }
 
@@ -673,6 +673,36 @@ mod tests {
         }
     }
 
+    #[test]
+    fn a_column_named_by_hand_is_refused_with_its_reason() {
+        let columns = editions();
+        let asked = |name| choosable(Dialect::Postgres, &columns, &["id"], name);
+        assert_eq!(asked("title"), Ok(()));
+        assert_eq!(asked("published_on"), Ok(()));
+        assert_eq!(asked("id"), Err(Refused::Excluded(Excluded::Key)));
+        assert_eq!(asked("cover"), Err(Refused::Excluded(Excluded::Binary)));
+        assert_eq!(asked("legal_name"), Err(Refused::NoColumn));
+        // A name is a column's as it is written, and no other's.
+        assert_eq!(asked("Title"), Err(Refused::NoColumn));
+    }
+
+    #[test]
+    fn a_table_with_no_key_or_a_key_of_several_columns_excludes_them_all() {
+        let columns = editions();
+        // No key: nothing is the key, and the app's pick is its first text.
+        let none = resolve(&LabelChoice::Automatic, Dialect::Postgres, &columns, &[]);
+        assert_eq!(none.column.as_deref(), Some("title"));
+        assert_eq!(excluded(Dialect::Postgres, &columns[0], &[]), None);
+        // Two columns: neither labels, and the pick is the next text.
+        let keys = ["id", "title"];
+        let both = resolve(&LabelChoice::Automatic, Dialect::Postgres, &columns, &keys);
+        assert_eq!(both.column.as_deref(), Some("subtitle"));
+        assert_eq!(
+            choosable(Dialect::Postgres, &columns, &keys, "title"),
+            Err(Refused::Excluded(Excluded::Key))
+        );
+    }
+
     fn publishers() -> ObjectRef {
         ObjectRef::new("public", "publishers")
     }
```

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib labels::`
Expected: it does not compile: `Refused` and `choosable` are not found, and `resolve` and `automatic` take one key.

- [ ] **Step 3: The rule**

```diff
diff --git a/src/labels.rs b/src/labels.rs
--- a/src/labels.rs
+++ b/src/labels.rs
@@ -83,11 +83,11 @@ pub fn excluded(dialect: Dialect, column: &ColumnInfo, keys: &[&str]) -> Option<
 }
 
 /// The column the app picks to read a row of a table by: its first text
-/// column that is not the key.
-pub fn automatic(dialect: Dialect, columns: &[ColumnInfo], key: &str) -> Option<String> {
+/// column that is not a key. `keys` as for [`excluded`].
+pub fn automatic(dialect: Dialect, columns: &[ColumnInfo], keys: &[&str]) -> Option<String> {
     columns
         .iter()
-        .filter(|column| column.name != key)
+        .filter(|column| !keys.contains(&column.name.as_str()))
         .find(|column| {
             matches!(
                 column_class(dialect, &column.type_name),
@@ -97,19 +97,47 @@ pub fn automatic(dialect: Dialect, columns: &[ColumnInfo], key: &str) -> Option<
         .map(|column| column.name.clone())
 }
 
-/// What `choice` comes to for a table of `columns`, in a picker whose
-/// foreign key references `key`. A column that was chosen and is not among
-/// them is `gone`, and the app's pick stands in for it. A chosen column
-/// that cannot be a label any more (it is this picker's key, or holds
-/// bytes now) labels nothing: the choice is still the user's.
+/// Why a column that was named for a table's label is not taken.
+#[derive(Debug, Clone, Copy, PartialEq, Eq)]
+pub enum Refused {
+    /// The table has no column of that name.
+    NoColumn,
+    /// It has, and the column cannot be a label.
+    Excluded(Excluded),
+}
+
+/// Whether the column `name` of a table of `columns` can label its rows.
+/// `keys` as for [`excluded`]. What a chooser's line offers needs no
+/// asking: this is for a column that was named by hand.
+pub fn choosable(
+    dialect: Dialect,
+    columns: &[ColumnInfo],
+    keys: &[&str],
+    name: &str,
+) -> Result<(), Refused> {
+    let column = columns.iter().find(|column| column.name == name);
+    let column = column.ok_or(Refused::NoColumn)?;
+    match excluded(dialect, column, keys) {
+        Some(why) => Err(Refused::Excluded(why)),
+        None => Ok(()),
+    }
+}
+
+/// What `choice` comes to for a table of `columns`. `keys` are the
+/// columns its rows are found by: the one a picker's foreign key
+/// references, or the table's primary key in its Structure view. A column
+/// that was chosen and is not among the columns is `gone`, and the app's
+/// pick stands in for it. A chosen column that cannot be a label any more
+/// (it is a key here, or holds bytes now) labels nothing: the choice is
+/// still the user's.
 pub fn resolve(
     choice: &LabelChoice,
     dialect: Dialect,
     columns: &[ColumnInfo],
-    key: &str,
+    keys: &[&str],
 ) -> Resolved {
-    let automatic = automatic(dialect, columns, key);
-    let allowed = |column: &ColumnInfo| excluded(dialect, column, &[key]).is_none();
+    let automatic = automatic(dialect, columns, keys);
+    let allowed = |column: &ColumnInfo| excluded(dialect, column, keys).is_none();
     let chosen = match choice {
         LabelChoice::Column(name) => Some(name),
         LabelChoice::Automatic | LabelChoice::NoLabel => None,
```

The one caller passes its key as a list of one:

```diff
diff --git a/src/app/labels.rs b/src/app/labels.rs
--- a/src/app/labels.rs
+++ b/src/app/labels.rs
@@ -27,7 +27,7 @@ pub(super) fn resolved(
 ) -> Option<Resolved> {
     let columns = kept.value.as_deref()?;
     let choice = labels.choice(conn, table);
-    let resolved = labels::resolve(&choice, dialect, columns, key);
+    let resolved = labels::resolve(&choice, dialect, columns, &[key]);
     if resolved.gone.is_some() && kept.is_loading() {
         return None;
     }
```

- [ ] **Step 4: Run the tests**

Run: `~/.cargo/bin/cargo fmt --all && ~/.cargo/bin/cargo test --locked -p tabletist --lib`
Expected: `test result: ok. 2222 passed; 0 failed; 20 ignored` (2220 after task 1; the count moves with `main`).

- [ ] **Step 5: Run the four checks**

Expected: all pass.

- [ ] **Step 6: Commit**

```bash
git add src/labels.rs src/app/labels.rs
git commit -S -m "Ask what can label a table's rows by the table's own keys

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: The Structure view's door

**Files:**
- Modify: `src/model.rs`, `src/app.rs`, `src/app/labels.rs`, `src/app/editing.rs`, `src/ui/keys.rs`, `src/ui/label_chooser.rs`, `src/ui/key_picker.rs`, `src/ui/structure.rs`, `src/ui/data_view.rs`
- Test: `src/ui/label_tests.rs`, `src/testing.rs`

The whole of the door but the Omarchy look's status line, which is task 4. After this task the Omarchy look's `gl` and `:label` lines work, and a refusal is noted; its status line does not say it yet.

While the chooser is open in the view the status line goes on naming the view's keys: the chooser's own are in its head (`j/k · enter · esc`). Under a picker the line names them, since insert mode has the line to itself.

- [ ] **Step 1: Write the failing tests, and the fixture's column of bytes**

```diff
diff --git a/src/testing.rs b/src/testing.rs
--- a/src/testing.rs
+++ b/src/testing.rs
@@ -858,8 +858,9 @@ pub fn isbns_structure() -> tabletist_db::Structure {
     }
 }
 
-/// The Bookshop's `editions`, cut to what a key to `isbns` needs: `isbn`
-/// is a foreign key to `isbns.isbn`.
+/// The Bookshop's `editions`, cut to what a key to `isbns` and the chooser
+/// of a label need: `isbn` is a foreign key to `isbns.isbn`, and `cover`
+/// holds bytes.
 pub fn editions_structure() -> tabletist_db::Structure {
     use tabletist_db::ColumnInfo;
     let column = |name: &str, type_name: &str| ColumnInfo {
@@ -872,6 +873,7 @@ pub fn editions_structure() -> tabletist_db::Structure {
             column("id", "INTEGER"),
             column("isbn", "TEXT"),
             column("title", "TEXT"),
+            column("cover", "BLOB"),
         ],
         primary_key: vec!["id".into()],
         foreign_keys: vec![tabletist_db::ForeignKeyInfo {
@@ -897,6 +899,7 @@ pub fn editions_page() -> RowPage {
             Value::Int(id),
             Value::Text(isbn.into()),
             Value::Text(title.into()),
+            Value::Null,
         ]
     };
     RowPage {
@@ -904,6 +907,7 @@ pub fn editions_page() -> RowPage {
             meta("id", "INTEGER", ValueKind::Numeric),
             meta("isbn", "TEXT", ValueKind::Text),
             meta("title", "TEXT", ValueKind::Text),
+            meta("cover", "BLOB", ValueKind::Binary),
         ],
         rows: vec![
             edition(1, "978-1-4028-9462-6", "The Lighthouse"),
```

```diff
diff --git a/src/ui/label_tests.rs b/src/ui/label_tests.rs
--- a/src/ui/label_tests.rs
+++ b/src/ui/label_tests.rs
@@ -13,8 +13,8 @@ use tabletist_db::{LookupPart, ObjectRef};
 use crate::backend::{Command, Event, RequestId, StateFile};
 use crate::connections::ConnectionId;
 use crate::key_picker::{Columns, Picker};
-use crate::labels::{Chooser, LabelChoice, Resolved, Source};
-use crate::model::{Action, CellPos, ConnTabId, EditStart, Pane, TabId};
+use crate::labels::{Chooser, Excluded, LabelChoice, Refused, Resolved, Source};
+use crate::model::{Action, CellPos, ConnTabId, EditStart, ObjectView, Pane, TabId};
 use crate::testing::{GENRES, Harness, PUBLISHERS, node, publishers_structure};
 use crate::theme::Look;
 
@@ -1198,3 +1198,731 @@ fn the_terminal_look_confirms_a_choice_in_its_status_line() {
     harness.settle();
     assert!(!harness.has("label name · 3 matches"));
 }
+
+// The Structure door: run 3.
+
+fn editions() -> ObjectRef {
+    ObjectRef::new("main", "editions")
+}
+
+/// Shows the Structure view of the tab `id` and gives it the keys.
+fn to_structure(harness: &mut Harness, tab: ConnTabId, id: TabId) {
+    let view = ObjectView::Structure;
+    let object_tab = id;
+    harness.app.apply(Action::SetView {
+        tab,
+        object_tab,
+        view,
+    });
+    harness.app.workspace_mut(tab).unwrap().pane = Pane::Grid;
+    harness.settle();
+}
+
+/// `publishers` open in a tab of its own, in its Structure view, on a
+/// connection that is read-only or is not. No cell is edited, and its
+/// rows never came: the view needs none.
+fn publishers_structure_in(look: Look, read_only: bool) -> (Harness, ConnTabId, TabId) {
+    let mut harness = Harness::new();
+    harness.set_look(look);
+    let tab = harness.connect_fake_as(read_only);
+    harness.app.apply(Action::OpenObject {
+        tab,
+        object: publishers(),
+        kind: tabletist_db::ObjectKind::Table,
+        pin: true,
+    });
+    harness.answer_structure(publishers_structure());
+    let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
+    to_structure(&mut harness, tab, id);
+    (harness, tab, id)
+}
+
+/// `editions` in its Structure view: a key, a key to `isbns`, a title
+/// and bytes.
+fn editions_structure_in(look: Look) -> (Harness, ConnTabId, TabId) {
+    let mut harness = Harness::new();
+    harness.set_look(look);
+    let (tab, id) = harness.editions();
+    to_structure(&mut harness, tab, id);
+    (harness, tab, id)
+}
+
+/// The chooser that is open in the Structure view of the tab.
+fn tab_chooser(harness: &Harness, tab: ConnTabId, id: TabId) -> Option<&Chooser> {
+    let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
+    object.label_chooser.as_ref()
+}
+
+fn cursor(harness: &Harness, tab: ConnTabId, id: TabId) -> usize {
+    let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
+    object.structure_cursor
+}
+
+/// What is kept for `table` on the connection of `tab`.
+fn kept(harness: &Harness, tab: ConnTabId, table: &ObjectRef) -> LabelChoice {
+    harness.app.labels.choice(&connection(harness, tab), table)
+}
+
+/// Why the last column named for a label was not taken.
+fn refused(harness: &Harness, tab: ConnTabId) -> Option<(String, Refused)> {
+    let noted = harness.app.workspace(tab).unwrap().label_refused.clone();
+    noted.map(|(_, name, why)| (name, why))
+}
+
+/// Runs `line` at the terminal look's prompt.
+fn run(harness: &mut Harness, tab: ConnTabId, line: &str) {
+    harness.app.apply(Action::OpenCommand(tab));
+    harness.app.workspace_mut(tab).unwrap().command = Some(line.to_owned());
+    harness.app.apply(Action::RunCommand(tab));
+    harness.settle();
+}
+
+/// A key that types `text`, as a keyboard sends it: the press with its
+/// text, then the release.
+fn type_key(harness: &mut Harness, key: Key, text: &str) {
+    harness.settle();
+    harness.frame(vec![
+        crate::testing::key(key, Modifiers::NONE),
+        egui::Event::Text(text.into()),
+    ]);
+    harness.frame(vec![crate::testing::release(key, Modifiers::NONE)]);
+    harness.settle();
+}
+
+/// Clicks the text `label` with the pointer's `button`, as a mouse does.
+fn click_text(harness: &mut Harness, label: &str, button: egui::PointerButton) {
+    let tree = harness.settle();
+    let at = crate::testing::bounds(&tree, label, Role::Label)
+        .unwrap_or_else(|| panic!("nothing reads {label:?}"))
+        .center();
+    let event = |pressed| egui::Event::PointerButton {
+        pos: at,
+        button,
+        pressed,
+        modifiers: Modifiers::NONE,
+    };
+    harness.frame(vec![egui::Event::PointerMoved(at)]);
+    harness.frame(vec![event(true)]);
+    harness.frame(vec![event(false)]);
+    harness.settle();
+}
+
+/// Whether the tree has a node `label` that cannot be acted on.
+fn disabled(harness: &mut Harness, label: &str) -> Option<bool> {
+    let tree = harness.settle();
+    let mut nodes = tree.nodes.iter();
+    let found = nodes.find(|(_, node)| node.label() == Some(label))?;
+    Some(found.1.is_disabled())
+}
+
+/// What a screen reader reads after the name of the node `label`.
+fn described(harness: &mut Harness, label: &str) -> Option<String> {
+    let tree = harness.settle();
+    let mut nodes = tree.nodes.iter();
+    let found = nodes.find(|(_, node)| node.label() == Some(label))?;
+    found.1.description().map(str::to_owned)
+}
+
+/// A press and a release of the pointer's first button at `at`.
+fn click_at(harness: &mut Harness, at: egui::Pos2) {
+    let event = |pressed| egui::Event::PointerButton {
+        pos: at,
+        button: egui::PointerButton::Primary,
+        pressed,
+        modifiers: Modifiers::NONE,
+    };
+    harness.frame(vec![egui::Event::PointerMoved(at)]);
+    harness.frame(vec![event(true)]);
+    harness.frame(vec![event(false)]);
+    harness.settle();
+}
+
+/// Test 5: the Structure view of the table a key points at sets its
+/// label, on a connection that opens read-only and with no cell edited.
+/// The chooser is the picker's. Nothing is sent to the database: the
+/// file is written, and that is all.
+#[test]
+fn the_structure_view_sets_the_label_on_a_read_only_connection() {
+    let (mut harness, tab, id) = publishers_structure_in(Look::macos(), true);
+    let access = harness.app.workspace(tab).unwrap().access;
+    assert_eq!(access, tabletist_db::Access::ReadOnly);
+    // The line is one thing for a screen reader.
+    let tree = harness.settle();
+    let line = "Label for foreign keys: name, Automatic";
+    assert!(node(&tree, line, Role::Group).is_some());
+    let help = "Fields that point at publishers show and search this column. \
+                On this computer only; nothing is written to the database.";
+    assert!(harness.has(help));
+    // The column that labels is marked on its line.
+    assert!(harness.has("name, label column"));
+    let before = harness.app.backend.sent.len();
+    harness.click("Change label column");
+    let open = tab_chooser(&harness, tab, id).expect("the chooser is open");
+    assert_eq!((open.at, &open.choice), (0, &LabelChoice::Automatic));
+    let tree = harness.settle();
+    assert!(node(&tree, "Label publishers by", Role::Menu).is_some());
+    for name in [
+        "Automatic, uses name",
+        "No label, keys only",
+        "name, TEXT",
+        "id, INTEGER, the key, unavailable",
+    ] {
+        assert!(node(&tree, name, Role::MenuItemRadio).is_some(), "{name}");
+    }
+    assert!(harness.has(
+        "For publishers in Fixture · dev, on this computer. Nothing is written to the database."
+    ));
+    // Its keys are the chooser's, with no field anywhere.
+    harness.press(Key::ArrowDown, Modifiers::NONE);
+    harness.press(Key::ArrowDown, Modifiers::NONE);
+    harness.press(Key::Enter, Modifiers::NONE);
+    assert!(tab_chooser(&harness, tab, id).is_none());
+    assert_eq!(kept(&harness, tab, &publishers()), column("name"));
+    assert!(harness.has("Label for foreign keys: name, Set by you"));
+    // The way back stands before what changes it, as the board has them.
+    let tree = harness.settle();
+    let left = |name| {
+        let button = crate::testing::bounds(&tree, name, Role::Button);
+        button.unwrap_or_else(|| panic!("no button {name}")).left()
+    };
+    assert!(left("Use Automatic") < left("Change label column"));
+    let since = &harness.app.backend.sent[before..];
+    let written = |command: &Command| {
+        matches!(
+            command,
+            Command::Save {
+                file: StateFile::Labels(_),
+                ..
+            }
+        )
+    };
+    assert!(since.iter().all(written), "{} commands", since.len());
+    assert_eq!(saved(&harness), [harness.app.labels.clone()]);
+    // And the way back.
+    harness.click("Use Automatic");
+    assert_eq!(kept(&harness, tab, &publishers()), LabelChoice::Automatic);
+    assert!(harness.has("Label for foreign keys: name, Automatic"));
+    assert!(!harness.has("Use Automatic"));
+}
+
+/// The view's key opens the chooser and closes nothing else; Esc and a
+/// change of view close it.
+#[test]
+fn the_structure_views_key_opens_the_chooser() {
+    let (mut harness, tab, id) = publishers_structure_in(Look::macos(), false);
+    harness.press(Key::L, Modifiers::COMMAND);
+    assert!(tab_chooser(&harness, tab, id).is_some());
+    harness.press(Key::Escape, Modifiers::NONE);
+    assert!(tab_chooser(&harness, tab, id).is_none());
+    assert!(saved(&harness).is_empty());
+    harness.press(Key::L, Modifiers::COMMAND);
+    assert!(tab_chooser(&harness, tab, id).is_some());
+    let (view, object_tab) = (ObjectView::Data, id);
+    harness.app.apply(Action::SetView {
+        tab,
+        object_tab,
+        view,
+    });
+    assert!(tab_chooser(&harness, tab, id).is_none());
+    // With the arrows on the tree the key is not the view's.
+    to_structure(&mut harness, tab, id);
+    harness.app.workspace_mut(tab).unwrap().pane = Pane::Tree;
+    harness.press(Key::L, Modifiers::COMMAND);
+    assert!(tab_chooser(&harness, tab, id).is_none());
+}
+
+/// The view has a cursor on the lines of its columns: the arrows move it
+/// in every look and `j` and `k` in the Omarchy look, it stops at the
+/// first line and the last, and a click puts it on a line.
+#[test]
+fn the_structure_view_has_a_cursor_on_its_columns() {
+    let (mut harness, tab, id) = editions_structure_in(Look::omarchy());
+    assert_eq!(cursor(&harness, tab, id), 0);
+    type_key(&mut harness, Key::J, "j");
+    type_key(&mut harness, Key::J, "j");
+    assert_eq!(cursor(&harness, tab, id), 2);
+    type_key(&mut harness, Key::K, "k");
+    assert_eq!(cursor(&harness, tab, id), 1);
+    harness.press(Key::ArrowDown, Modifiers::NONE);
+    assert_eq!(cursor(&harness, tab, id), 2);
+    for _ in 0..5 {
+        harness.press(Key::ArrowDown, Modifiers::NONE);
+    }
+    assert_eq!(cursor(&harness, tab, id), 3);
+    for _ in 0..5 {
+        harness.press(Key::ArrowUp, Modifiers::NONE);
+    }
+    assert_eq!(cursor(&harness, tab, id), 0);
+    // The rows' selection did not move under it.
+    let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
+    assert_eq!(object.selection, None);
+    // A click on a line, with the keys on the tree: the line has the
+    // cursor and the view the keys.
+    let (mut harness, tab, id) = editions_structure_in(Look::macos());
+    harness.app.workspace_mut(tab).unwrap().pane = Pane::Tree;
+    click_text(&mut harness, "title", egui::PointerButton::Primary);
+    assert_eq!(cursor(&harness, tab, id), 2);
+    assert_eq!(harness.app.workspace(tab).unwrap().pane, Pane::Grid);
+    harness.press(Key::ArrowUp, Modifiers::NONE);
+    assert_eq!(cursor(&harness, tab, id), 1);
+}
+
+/// Test 6: the key and a column of bytes cannot be chosen, in the
+/// chooser, from a column's menu, with the view's key or by a line of
+/// the prompt, and each says why.
+#[test]
+fn the_key_and_bytes_are_refused_with_their_reason() {
+    // The chooser lists them, and they answer no click.
+    let (mut harness, tab, id) = editions_structure_in(Look::macos());
+    harness.click("Change label column");
+    let tree = harness.settle();
+    for name in [
+        "id, INTEGER, the key, unavailable",
+        "cover, BLOB, binary, unavailable",
+    ] {
+        assert!(node(&tree, name, Role::MenuItemRadio).is_some(), "{name}");
+        harness.click(name);
+        assert!(tab_chooser(&harness, tab, id).is_some(), "{name}");
+    }
+    harness.press(Key::Escape, Modifiers::NONE);
+    // A column's menu offers it, and says why not on those two.
+    let why = |harness: &mut Harness| described(harness, "Use as label");
+    click_text(&mut harness, "cover", egui::PointerButton::Secondary);
+    assert_eq!(disabled(&mut harness, "Use as label"), Some(true));
+    assert_eq!(why(&mut harness).as_deref(), Some("binary"));
+    harness.press(Key::Escape, Modifiers::NONE);
+    click_text(&mut harness, "id", egui::PointerButton::Secondary);
+    assert_eq!(why(&mut harness).as_deref(), Some("the key"));
+    harness.press(Key::Escape, Modifiers::NONE);
+    click_text(&mut harness, "title", egui::PointerButton::Secondary);
+    assert_eq!(disabled(&mut harness, "Use as label"), Some(false));
+    harness.press(Key::Escape, Modifiers::NONE);
+    // The view's key on the key's line: refused, and the view says why
+    // until the cursor moves.
+    assert_eq!(cursor(&harness, tab, id), 2);
+    harness.press(Key::ArrowUp, Modifiers::NONE);
+    harness.press(Key::ArrowUp, Modifiers::NONE);
+    harness.press(Key::L, Modifiers::COMMAND | Modifiers::ALT);
+    let key = Refused::Excluded(Excluded::Key);
+    assert_eq!(refused(&harness, tab), Some(("id".to_owned(), key)));
+    assert!(harness.has("id cannot be the label (the key)"));
+    harness.press(Key::ArrowDown, Modifiers::NONE);
+    assert_eq!(refused(&harness, tab), None);
+    assert!(!harness.has("id cannot be the label (the key)"));
+    assert_eq!(kept(&harness, tab, &editions()), LabelChoice::Automatic);
+    assert!(saved(&harness).is_empty());
+    // A line of the prompt that names one, and one that names none.
+    let (mut harness, tab, _) = editions_structure_in(Look::omarchy());
+    for (line, why) in [
+        ("label cover", Refused::Excluded(Excluded::Binary)),
+        ("label id", Refused::Excluded(Excluded::Key)),
+        ("label nope", Refused::NoColumn),
+    ] {
+        run(&mut harness, tab, line);
+        let name = line.trim_start_matches("label ").to_owned();
+        assert_eq!(refused(&harness, tab), Some((name, why)), "{line}");
+        assert_eq!(kept(&harness, tab, &editions()), LabelChoice::Automatic);
+    }
+    // Until the next key.
+    harness.press(Key::ArrowDown, Modifiers::NONE);
+    assert_eq!(refused(&harness, tab), None);
+    assert!(saved(&harness).is_empty());
+}
+
+/// Test 12: the Omarchy look's `gl` and the prompt's three lines keep
+/// what the other looks' controls keep.
+#[test]
+fn the_terminal_looks_keys_keep_what_the_buttons_keep() {
+    let (mut omarchy, tab, id) = editions_structure_in(Look::omarchy());
+    let (mut macos, mac_tab, mac_id) = editions_structure_in(Look::macos());
+    let same = |omarchy: &Harness, macos: &Harness, choice: LabelChoice| {
+        assert_eq!(kept(omarchy, tab, &editions()), choice, "omarchy");
+        assert_eq!(kept(macos, mac_tab, &editions()), choice, "macos");
+    };
+    // The column under the cursor: `gl`, and a column's menu.
+    type_key(&mut omarchy, Key::J, "j");
+    type_key(&mut omarchy, Key::J, "j");
+    type_key(&mut omarchy, Key::G, "g");
+    type_key(&mut omarchy, Key::L, "l");
+    click_text(&mut macos, "title", egui::PointerButton::Secondary);
+    macos.click("Use as label");
+    same(&omarchy, &macos, column("title"));
+    assert!(omarchy.has("title, label column"));
+    assert!(macos.has("title, label column"));
+    // Back to Automatic: `:label auto`, and the button.
+    run(&mut omarchy, tab, "label auto");
+    macos.click("Use Automatic");
+    same(&omarchy, &macos, LabelChoice::Automatic);
+    // Keys only: `:label none`, and the chooser's line.
+    run(&mut omarchy, tab, "label none");
+    macos.click("Change label column");
+    macos.click("No label, keys only");
+    same(&omarchy, &macos, LabelChoice::NoLabel);
+    // A column by its name: `:label isbn`, and the chooser's line. `isbn`
+    // is a foreign key here and no key of this table's.
+    run(&mut omarchy, tab, "label isbn");
+    macos.click("Change label column");
+    macos.click("isbn, TEXT");
+    same(&omarchy, &macos, column("isbn"));
+    // `:label` alone opens the chooser, whose keys are `j`, `k` and Enter.
+    run(&mut omarchy, tab, "label");
+    assert!(tab_chooser(&omarchy, tab, id).is_some());
+    assert!(tab_chooser(&macos, mac_tab, mac_id).is_none());
+    type_key(&mut omarchy, Key::K, "k");
+    type_key(&mut omarchy, Key::K, "k");
+    omarchy.press(Key::Enter, Modifiers::NONE);
+    assert!(tab_chooser(&omarchy, tab, id).is_none());
+    assert_eq!(kept(&omarchy, tab, &editions()), LabelChoice::Automatic);
+}
+
+/// The Omarchy look's view says the label in its own words, and names
+/// its keys where the other looks have buttons.
+#[test]
+fn the_terminal_looks_structure_view_says_the_label_in_its_words() {
+    let (mut harness, tab, _) = publishers_structure_in(Look::omarchy(), true);
+    assert!(harness.has("Label for foreign keys: name, automatic"));
+    let help = "fields that point at publishers show and search the label column · \
+                on this computer only, nothing is written to the database";
+    assert!(harness.has(help));
+    assert!(harness.painted_rect("fk label").is_some());
+    let keys = ":label choose · gl use the column under the cursor";
+    assert!(
+        harness.painted_rect(keys).is_some(),
+        "{:?}",
+        harness.painted
+    );
+    assert!(!harness.has("Change label column"));
+    // The column that labels has the look's mark at its name: an arrow,
+    // in whichever form the font draws.
+    let marks = ["◂ label", "< label"];
+    let marked = marks
+        .iter()
+        .any(|mark| harness.painted_rect(mark).is_some());
+    assert!(marked, "{:?}", harness.painted);
+    // Once the user chose, the line names the way back in the keys' place.
+    run(&mut harness, tab, "label name");
+    assert!(harness.has("Label for foreign keys: name, set by you"));
+    let back = ":label auto back to automatic";
+    assert!(
+        harness.painted_rect(back).is_some(),
+        "{:?}",
+        harness.painted
+    );
+    assert!(harness.painted_rect(keys).is_none());
+}
+
+/// A table of its key alone: the line says so, the button is disabled,
+/// and nothing opens.
+#[test]
+fn the_structure_view_of_a_table_of_its_key_alone_says_so() {
+    let mut harness = Harness::new();
+    harness.set_look(Look::macos());
+    let tab = harness.connect_fake_as(false);
+    let isbns = ObjectRef::new("main", "isbns");
+    harness.app.apply(Action::OpenObject {
+        tab,
+        object: isbns.clone(),
+        kind: tabletist_db::ObjectKind::Table,
+        pin: true,
+    });
+    harness.answer_structure(crate::testing::isbns_structure());
+    let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
+    to_structure(&mut harness, tab, id);
+    assert!(harness.has("Label for foreign keys: isbns has only its key column"));
+    assert_eq!(disabled(&mut harness, "Change label column"), Some(true));
+    harness.press(Key::L, Modifiers::COMMAND);
+    assert!(tab_chooser(&harness, tab, id).is_none());
+    // Its one column is the key: the view's key says so.
+    harness.press(Key::L, Modifiers::COMMAND | Modifiers::ALT);
+    let key = Refused::Excluded(Excluded::Key);
+    assert_eq!(refused(&harness, tab), Some(("isbn".to_owned(), key)));
+    assert_eq!(kept(&harness, tab, &isbns), LabelChoice::Automatic);
+}
+
+/// A view labels nothing: no foreign key points at one. Its Structure
+/// view has no label line, and the prompt's lines do nothing there.
+#[test]
+fn a_views_structure_has_no_label() {
+    let mut harness = Harness::new();
+    harness.set_look(Look::omarchy());
+    let tab = harness.connect_fake_as(false);
+    let view = ObjectRef::new("main", "active_users");
+    harness.app.apply(Action::OpenObject {
+        tab,
+        object: view.clone(),
+        kind: tabletist_db::ObjectKind::View,
+        pin: true,
+    });
+    harness.answer_structure(publishers_structure());
+    let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
+    to_structure(&mut harness, tab, id);
+    assert!(!harness.has("Label for foreign keys: name, automatic"));
+    run(&mut harness, tab, "label name");
+    run(&mut harness, tab, "label");
+    assert!(tab_chooser(&harness, tab, id).is_none());
+    assert_eq!(kept(&harness, tab, &view), LabelChoice::Automatic);
+    assert!(saved(&harness).is_empty());
+}
+
+/// The lines of the prompt are the Structure view's: over the rows
+/// `:label` is no command.
+#[test]
+fn the_label_lines_are_no_commands_over_the_rows() {
+    let mut harness = Harness::new();
+    harness.set_look(Look::omarchy());
+    let (tab, id) = harness.editions();
+    for line in ["label", "label auto", "label title"] {
+        run(&mut harness, tab, line);
+        let workspace = harness.app.workspace(tab).unwrap();
+        assert_eq!(workspace.command_error.as_deref(), Some(line));
+        assert!(tab_chooser(&harness, tab, id).is_none());
+    }
+    assert!(saved(&harness).is_empty());
+}
+
+/// A choice made in the Structure view labels the pickers of every key
+/// that points at the table: the next one opened searches by it.
+#[test]
+fn a_choice_made_in_the_structure_view_labels_the_pickers() {
+    let (mut harness, tab, id) = covers_in(Look::macos());
+    // `publishers` in a tab of its own, beside the covers'.
+    harness.app.apply(Action::OpenObject {
+        tab,
+        object: publishers(),
+        kind: tabletist_db::ObjectKind::Table,
+        pin: true,
+    });
+    harness.answer_structure(publishers_structure());
+    let other = harness.app.workspace(tab).unwrap().active_tab.unwrap();
+    to_structure(&mut harness, tab, other);
+    harness.click("Change label column");
+    harness.click("No label, keys only");
+    assert_eq!(kept(&harness, tab, &publishers()), LabelChoice::NoLabel);
+    // Back on the covers: the picker lists keys alone.
+    harness.app.apply(Action::ActivateTab { tab, id });
+    harness.settle();
+    open(&mut harness, tab, id, 1);
+    let by_user = labelled(None, Source::User, Some("name"));
+    assert!(picker(&harness, tab, id).label == Columns::Known(by_user));
+    assert!(!searched(&harness).is_empty());
+    assert!(searched(&harness).iter().all(Option::is_none));
+}
+
+/// The view is a pane for the keyboard, as the rows are whose place it
+/// stands in: the Omarchy look's key for the pane to the right brings
+/// the keys to it from the tree, and its cursor's keys then answer.
+#[test]
+fn the_pane_keys_give_the_structure_view_the_keys() {
+    let (mut harness, tab, id) = editions_structure_in(Look::omarchy());
+    harness.app.workspace_mut(tab).unwrap().pane = Pane::Tree;
+    harness.settle();
+    // On the tree `j` is the tree's.
+    type_key(&mut harness, Key::J, "j");
+    assert_eq!(cursor(&harness, tab, id), 0);
+    harness.press(Key::L, ctrl());
+    assert_eq!(harness.app.workspace(tab).unwrap().pane, Pane::Grid);
+    // With the keyboard on the view's one stop its chords answer too: the
+    // stop is a pane, and no control that would take the arrows itself.
+    harness.press(Key::ArrowDown, Modifiers::NONE);
+    assert_eq!(cursor(&harness, tab, id), 1);
+    type_key(&mut harness, Key::J, "j");
+    assert_eq!(cursor(&harness, tab, id), 2);
+    type_key(&mut harness, Key::G, "g");
+    type_key(&mut harness, Key::L, "l");
+    assert_eq!(kept(&harness, tab, &editions()), column("title"));
+}
+
+/// What was said of a column that was not taken is said of its table,
+/// in its tab: another table's Structure view does not say it.
+#[test]
+fn a_refusal_is_said_in_its_own_tab_alone() {
+    let (mut harness, tab, _) = editions_structure_in(Look::macos());
+    harness.press(Key::L, Modifiers::COMMAND | Modifiers::ALT);
+    let said = "id cannot be the label (the key)";
+    assert!(harness.has(said));
+    harness.app.apply(Action::OpenObject {
+        tab,
+        object: publishers(),
+        kind: tabletist_db::ObjectKind::Table,
+        pin: true,
+    });
+    harness.answer_structure(publishers_structure());
+    let other = harness.app.workspace(tab).unwrap().active_tab.unwrap();
+    to_structure(&mut harness, tab, other);
+    assert!(harness.has("Label for foreign keys: name, Automatic"));
+    assert!(!harness.has(said));
+}
+
+/// A press beside the view's chooser closes it, as it closes a menu.
+#[test]
+fn a_press_beside_the_structure_views_chooser_closes_it() {
+    let (mut harness, tab, id) = publishers_structure_in(Look::macos(), false);
+    harness.click("Change label column");
+    assert!(tab_chooser(&harness, tab, id).is_some());
+    // Far from the chooser, which hangs under its button at the card's
+    // end: the middle of the window, near its foot.
+    let at = egui::pos2(harness.size.x / 2.0, harness.size.y - 120.0);
+    click_at(&mut harness, at);
+    assert!(tab_chooser(&harness, tab, id).is_none());
+    assert!(saved(&harness).is_empty());
+}
+
+/// The cursor stays on a line there is when the table's structure comes
+/// again with fewer columns.
+#[test]
+fn the_structure_views_cursor_stays_inside_the_columns() {
+    let (mut harness, tab, id) = editions_structure_in(Look::macos());
+    let line = 3;
+    harness
+        .app
+        .apply(Action::SetStructureCursor { tab, id, line });
+    assert_eq!(cursor(&harness, tab, id), 3);
+    let object_tab = id;
+    harness
+        .app
+        .apply(Action::RetryStructure { tab, object_tab });
+    harness.answer_structure(publishers_structure());
+    assert_eq!(cursor(&harness, tab, id), 1);
+}
+
+/// The editions with eighty columns after their own, in their Structure
+/// view: more lines than the window shows.
+fn wide_structure() -> (Harness, ConnTabId, TabId) {
+    let mut harness = Harness::new();
+    harness.set_look(Look::macos());
+    let tab = harness.connect_fake_as(false);
+    harness.app.apply(Action::OpenObject {
+        tab,
+        object: editions(),
+        kind: tabletist_db::ObjectKind::Table,
+        pin: true,
+    });
+    let mut wide = crate::testing::editions_structure();
+    for number in 0..80 {
+        wide.columns.push(tabletist_db::ColumnInfo {
+            name: format!("extra_{number:02}"),
+            type_name: "TEXT".into(),
+            ..tabletist_db::ColumnInfo::default()
+        });
+    }
+    harness.answer_structure(wide);
+    let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
+    to_structure(&mut harness, tab, id);
+    (harness, tab, id)
+}
+
+/// Whether the words `name` are painted inside the window.
+fn in_sight(harness: &Harness, name: &str) -> bool {
+    let window = egui::Rect::from_min_size(egui::Pos2::ZERO, harness.size);
+    let painted = harness.painted_rect(name);
+    painted.is_some_and(|painted| window.contains_rect(painted))
+}
+
+/// Moves the Structure view's cursor and waits for the view: the jump is
+/// asked for where the line is drawn, and the view has it two frames
+/// after.
+fn move_cursor(harness: &mut Harness, tab: ConnTabId, id: TabId, by: isize) {
+    harness
+        .app
+        .apply(Action::MoveStructureCursor { tab, id, by });
+    harness.settle();
+    harness.settle();
+}
+
+/// A table with more columns than the window shows: the cursor's line
+/// is brought into view as the cursor moves, so the view's keys never
+/// act on a line that is out of sight.
+#[test]
+fn the_structure_views_cursor_is_scrolled_into_view() {
+    let (mut harness, tab, id) = wide_structure();
+    assert!(in_sight(&harness, "title"));
+    assert!(!in_sight(&harness, "extra_70"));
+    move_cursor(&mut harness, tab, id, 74);
+    assert_eq!(cursor(&harness, tab, id), 74);
+    assert!(
+        in_sight(&harness, "extra_70"),
+        "the cursor's line is out of sight"
+    );
+    // And back: the first line shows again.
+    move_cursor(&mut harness, tab, id, -74);
+    assert!(in_sight(&harness, "title"));
+    assert!(!in_sight(&harness, "extra_70"));
+}
+
+/// The view may be scrolled while the tree has the keys. When the keys
+/// come back to it, the cursor's line is brought into view again.
+#[test]
+fn the_structure_views_cursor_is_in_view_when_the_keys_come_back() {
+    let (mut harness, tab, id) = wide_structure();
+    move_cursor(&mut harness, tab, id, 74);
+    assert!(in_sight(&harness, "extra_70"));
+    harness.app.workspace_mut(tab).unwrap().pane = Pane::Tree;
+    // The wheel, over the view, back to its top.
+    let over = egui::pos2(harness.size.x / 2.0, harness.size.y / 2.0);
+    harness.frame(vec![egui::Event::PointerMoved(over)]);
+    for _ in 0..10 {
+        harness.frame(vec![egui::Event::MouseWheel {
+            unit: egui::MouseWheelUnit::Line,
+            delta: egui::vec2(0.0, 40.0),
+            phase: egui::TouchPhase::Move,
+            modifiers: Modifiers::NONE,
+        }]);
+    }
+    harness.finish_animations();
+    assert!(!in_sight(&harness, "extra_70"));
+    harness.app.apply(Action::GridKeys(tab));
+    harness.settle();
+    harness.settle();
+    assert!(
+        in_sight(&harness, "extra_70"),
+        "the cursor's line is out of sight"
+    );
+}
+
+/// Each table's Structure view keeps its own place: after another
+/// table's view was looked at, the cursor's line is where it was left.
+#[test]
+fn each_structure_view_keeps_its_own_place() {
+    let (mut harness, tab, id) = wide_structure();
+    move_cursor(&mut harness, tab, id, 74);
+    assert!(in_sight(&harness, "extra_70"));
+    // Another table, whose few columns need no scrolling.
+    harness.app.apply(Action::OpenObject {
+        tab,
+        object: publishers(),
+        kind: tabletist_db::ObjectKind::Table,
+        pin: true,
+    });
+    harness.answer_structure(publishers_structure());
+    let other = harness.app.workspace(tab).unwrap().active_tab.unwrap();
+    assert_ne!(other, id);
+    to_structure(&mut harness, tab, other);
+    harness.settle();
+    assert!(!in_sight(&harness, "extra_70"));
+    harness.app.apply(Action::ActivateTab { tab, id });
+    harness.settle();
+    harness.settle();
+    assert!(
+        in_sight(&harness, "extra_70"),
+        "the cursor's line is out of sight"
+    );
+}
+
+/// What the keys do in a Structure view that has them: Reload reloads
+/// the table, where with the keys on the tree it reloads the tree.
+#[test]
+fn reload_in_the_structure_view_reloads_the_table() {
+    let (mut harness, tab, _) = publishers_structure_in(Look::macos(), false);
+    let described = |harness: &Harness| {
+        let sent = harness.app.backend.sent.iter();
+        sent.filter(|command| matches!(command, Command::Describe { .. }))
+            .count()
+    };
+    let before = described(&harness);
+    harness.press(Key::R, Modifiers::COMMAND);
+    assert_eq!(described(&harness), before + 1);
+    // The tree's own reload asks for no table's structure.
+    harness.app.workspace_mut(tab).unwrap().pane = Pane::Tree;
+    harness.press(Key::R, Modifiers::COMMAND);
+    assert_eq!(described(&harness), before + 1);
+}
```

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib label_tests`
Expected: it does not compile: the tab has no `label_chooser` and no `structure_cursor`, the workspace no `label_refused`, and `Action` no `UseAsLabel`.

- [ ] **Step 3: The tab's cursor and chooser, the workspace's note, the actions**

```diff
diff --git a/src/model.rs b/src/model.rs
--- a/src/model.rs
+++ b/src/model.rs
@@ -363,6 +363,42 @@ pub enum Action {
         tab: ConnTabId,
         id: TabId,
     },
+    /// Move the cursor of the Structure view's columns `by` lines.
+    MoveStructureCursor {
+        tab: ConnTabId,
+        id: TabId,
+        by: isize,
+    },
+    /// Put that cursor on `line`: a click on it.
+    SetStructureCursor {
+        tab: ConnTabId,
+        id: TabId,
+        line: usize,
+    },
+    /// Use the column on `line` of the Structure view as what labels the
+    /// table's rows, or the one under the cursor: a column's menu, the
+    /// view's key. A column that cannot be a label is refused, and the
+    /// workspace says why.
+    UseAsLabel {
+        tab: ConnTabId,
+        id: TabId,
+        line: Option<usize>,
+    },
+    /// Keep `choice` as what labels the rows of the table the tab shows:
+    /// the Structure view's Use Automatic, the prompt's lines.
+    SetLabel {
+        tab: ConnTabId,
+        id: TabId,
+        choice: crate::labels::LabelChoice,
+    },
+    /// Label the table the tab shows by its column `name`, as a line of
+    /// the prompt names it. Refused, with why, where no column is called
+    /// that or the column cannot be a label.
+    LabelByName {
+        tab: ConnTabId,
+        id: TabId,
+        name: String,
+    },
     /// Add a line break and move the text into the large editor.
     EditorBreak {
         tab: ConnTabId,
@@ -767,6 +803,10 @@ pub struct Workspace {
     /// `:diff` was run with nothing pending. The terminal's line says so,
     /// until the next key.
     pub review_refused: bool,
+    /// A column was named for a table's label and was not taken: the
+    /// tab whose table it was named for, its name as it was given, and
+    /// why. Said there, and nowhere else, until the next key.
+    pub label_refused: Option<(TabId, String, crate::labels::Refused)>,
     /// Fold or unfold the row panel's documents on the next frame (`za`).
     pub fold_documents: Option<TabId>,
     /// When the session last connected, in seconds since the Unix epoch
@@ -1997,6 +2037,14 @@ pub struct ObjectTab {
     /// values by column. It is the tab's own and goes with the tab. A
     /// reload, a save and a discard leave it.
     pub yanked: Option<crate::edit::Yank>,
+    /// The line of the Structure view's columns the cursor is on: the
+    /// column its keys act on. Held inside the columns there are, where
+    /// it moves and where a structure arrives.
+    pub structure_cursor: usize,
+    /// The chooser of what labels this table's rows, while it is open in
+    /// the Structure view. A picker's chooser is the picker's own
+    /// (`key_picker::Picker::chooser`).
+    pub label_chooser: Option<crate::labels::Chooser>,
 }
 
 /// The row panel's text for one row, formatted once when the selection or
@@ -2128,6 +2176,8 @@ impl ObjectTab {
             focus_field: None,
             edits: crate::edit::Edits::default(),
             yanked: None,
+            structure_cursor: 0,
+            label_chooser: None,
         }
     }
 
@@ -2973,6 +3023,7 @@ impl Workspace {
             command_error: None,
             save_refused: false,
             review_refused: false,
+            label_refused: None,
             fold_documents: None,
             connected_at: None,
         }
@@ -2986,13 +3037,14 @@ impl Workspace {
         self.tabs.iter_mut().find(|tab| tab.id() == id)
     }
 
-    /// The lines of the chooser of what labels the rows of `table`, for
-    /// a foreign key that references its column `key`: by its columns as
-    /// the workspace knows them now.
-    pub fn label_lines(&self, table: &ObjectRef, key: &str) -> Vec<crate::labels::Line> {
+    /// The lines of the chooser of what labels the rows of `table`, by
+    /// its columns as the workspace knows them now. `keys` are the
+    /// columns its rows are found by: the one a picker's foreign key
+    /// references, or the table's primary key.
+    pub fn label_lines(&self, table: &ObjectRef, keys: &[&str]) -> Vec<crate::labels::Line> {
         let known = self.columns.get(table);
         let columns = known.and_then(|kept| kept.value.as_deref());
-        crate::labels::lines(self.driver.dialect(), columns, &[key])
+        crate::labels::lines(self.driver.dialect(), columns, keys)
     }
 
     pub fn object_tab(&self, id: TabId) -> Option<&ObjectTab> {
diff --git a/src/ui/data_view.rs b/src/ui/data_view.rs
--- a/src/ui/data_view.rs
+++ b/src/ui/data_view.rs
@@ -1835,7 +1835,7 @@ pub(super) fn search_of(
             };
             super::label_chooser::Open {
                 at: chooser.at,
-                lines: workspace.label_lines(&picker.target, &picker.key),
+                lines: workspace.label_lines(&picker.target, &[&picker.key]),
                 choice: chooser.choice.clone(),
                 automatic: known.and_then(|label| label.automatic.clone()),
                 loading: known.is_none(),
```

- [ ] **Step 4: The reducer**

The chooser's functions of run 2 are rewritten to serve two homes: `chooser_mut` says where the tab's chooser lives (on the tab while its Structure view shows, else in its editor's picker) and `chooser_lines` whose table it chooses for and with which keys. `structure_label` is the one resolver for the view. A column named by hand goes through `label_by_name`, which asks `labels::choosable` and keeps why it refused, with the tab it was asked in: the note is said there and under no other table. It goes when the cursor moves, a label is kept or the view changes. A structure that arrives holds the cursor inside its columns.

```diff
diff --git a/src/app/labels.rs b/src/app/labels.rs
--- a/src/app/labels.rs
+++ b/src/app/labels.rs
@@ -3,7 +3,7 @@
 //! happens to a choice whose column is gone. The choice and the rule are
 //! in `crate::labels`. Nothing else reads the store.
 
-use tabletist_db::{ColumnInfo, Dialect, ObjectRef};
+use tabletist_db::{ColumnInfo, Dialect, ObjectKind, ObjectRef, Structure};
 
 use std::time::Instant;
 
@@ -11,7 +11,7 @@ use super::App;
 use crate::connections::ConnectionId;
 use crate::key_picker::Columns;
 use crate::labels::{self, Chooser, LabelChoice, Labels, Line, Resolved, Source};
-use crate::model::{ConnTabId, Fetch, TabId};
+use crate::model::{ConnTabId, Fetch, ObjectView, TabId};
 
 /// What labels the rows of `table`, whose columns the workspace of the
 /// saved connection `conn` keeps as `kept`, in a picker whose foreign key
@@ -74,98 +74,290 @@ impl App {
         self.relabel(&conn, table, false);
     }
 
-    /// Opens the chooser of what labels the options of the picker of the
-    /// tab's editor, on the line of the choice kept for its table; closes
-    /// it where it is open. Where the table has nothing to choose no
-    /// chooser opens, and the picker says so for a while.
-    pub(super) fn toggle_label_chooser(&mut self, tab: ConnTabId, id: TabId) {
-        let planned = self.workspace(tab).and_then(|workspace| {
-            let editor = workspace.object_tab(id)?.edits.editor.as_ref()?;
-            let picker = editor.picker.as_ref()?;
-            if picker.chooser.is_some() {
-                return Some(None);
-            }
-            // A search that failed has no list, and its keys are the
-            // field's: a chooser there would get none of them.
-            if picker.failed.is_some() {
-                return None;
-            }
-            let lines = workspace.label_lines(&picker.target, &picker.key);
-            let choice = self.labels.choice(&workspace.conn_id, &picker.target);
-            // The columns are still on their way: Automatic and No label
-            // can be chosen at once, and the rest shows when it comes.
-            let waited = picker.label == Columns::Waiting;
-            let nothing = matches!(
-                &picker.label,
-                Columns::Known(label) if label.source == Source::KeysOnly
-            );
-            Some(Some((Chooser::open(&lines, choice, waited), nothing)))
-        });
-        let picker = self
-            .editor_mut(tab, id)
-            .and_then(|editor| editor.picker.as_mut());
-        let (Some(planned), Some(picker)) = (planned, picker) else {
-            return;
-        };
-        match planned {
-            None => picker.chooser = None,
-            Some((_, true)) => picker.nothing = Some(Instant::now()),
-            Some((chooser, false)) => {
-                picker.chooser = Some(chooser);
-                picker.nothing = None;
+    /// The chooser of the tab `id`, where it has a place for one: its
+    /// Structure view's, on the tab itself, while that view shows, and
+    /// else the one of its editor's picker.
+    fn chooser_mut(&mut self, tab: ConnTabId, id: TabId) -> Option<&mut Option<Chooser>> {
+        let object = self.object_tab_mut(tab, id)?;
+        match object.view {
+            ObjectView::Structure => Some(&mut object.label_chooser),
+            ObjectView::Data => {
+                let picker = object.edits.editor.as_mut()?.picker.as_mut()?;
+                Some(&mut picker.chooser)
             }
         }
     }
 
-    /// Lets `act` move the keys of the chooser that is open beside the
-    /// picker of the tab's editor, among its lines as they are now.
+    /// The table that the chooser of the tab `id` chooses for, and its
+    /// lines as they are now: the tab's own table by its structure, with
+    /// its primary key as the key, in the Structure view; the table its
+    /// picker searches, with the column the foreign key references, under
+    /// an editor.
+    fn chooser_lines(&self, tab: ConnTabId, id: TabId) -> Option<(ObjectRef, Vec<Line>)> {
+        let workspace = self.workspace(tab)?;
+        let object = workspace.object_tab(id)?;
+        match object.view {
+            ObjectView::Structure => {
+                let structure = object.structure.value.as_ref()?;
+                let dialect = workspace.driver.dialect();
+                let lines = labels::lines(dialect, Some(&structure.columns), &keys_of(structure));
+                Some((object.object.clone(), lines))
+            }
+            ObjectView::Data => {
+                let picker = object.edits.editor.as_ref()?.picker.as_ref()?;
+                let lines = workspace.label_lines(&picker.target, &[&picker.key]);
+                Some((picker.target.clone(), lines))
+            }
+        }
+    }
+
+    /// What labels the rows of the table the tab `id` shows, as its
+    /// Structure view says it: the choice kept for the table against its
+    /// own columns, its primary key being the key. None while the
+    /// structure is not known, and for what is no table: no foreign key
+    /// points at a view.
+    pub fn structure_label(&self, tab: ConnTabId, id: TabId) -> Option<Resolved> {
+        let workspace = self.workspace(tab)?;
+        let object = workspace.object_tab(id)?;
+        match object.kind {
+            ObjectKind::Table => {}
+            ObjectKind::View | ObjectKind::MaterializedView => return None,
+        }
+        let structure = object.structure.value.as_ref()?;
+        let choice = self.labels.choice(&workspace.conn_id, &object.object);
+        let dialect = workspace.driver.dialect();
+        let keys = keys_of(structure);
+        Some(labels::resolve(&choice, dialect, &structure.columns, &keys))
+    }
+
+    /// Opens the chooser of the tab `id` on the line of the choice kept
+    /// for its table, and closes it where it is open: beside the picker
+    /// of its editor, or in its Structure view. Where the table has
+    /// nothing to choose no chooser opens: a picker says so for a while,
+    /// and the Structure view's line says it as long as it shows.
+    pub(super) fn toggle_label_chooser(&mut self, tab: ConnTabId, id: TabId) {
+        // What the key does: nothing, close, say there is nothing to
+        // choose, or open.
+        enum Asked {
+            Close,
+            Nothing,
+            Open(Chooser),
+        }
+        let asked = self.workspace(tab).and_then(|workspace| {
+            let object = workspace.object_tab(id)?;
+            match object.view {
+                ObjectView::Structure => {
+                    if object.label_chooser.is_some() {
+                        return Some(Asked::Close);
+                    }
+                    let label = self.structure_label(tab, id)?;
+                    let (table, lines) = self.chooser_lines(tab, id)?;
+                    let choice = self.labels.choice(&workspace.conn_id, &table);
+                    Some(match label.source {
+                        Source::KeysOnly => Asked::Nothing,
+                        Source::Automatic | Source::User => {
+                            Asked::Open(Chooser::open(&lines, choice, false))
+                        }
+                    })
+                }
+                ObjectView::Data => {
+                    let picker = object.edits.editor.as_ref()?.picker.as_ref()?;
+                    if picker.chooser.is_some() {
+                        return Some(Asked::Close);
+                    }
+                    // A search that failed has no list, and its keys are
+                    // the field's: a chooser there would get none of them.
+                    if picker.failed.is_some() {
+                        return None;
+                    }
+                    let (table, lines) = self.chooser_lines(tab, id)?;
+                    let choice = self.labels.choice(&workspace.conn_id, &table);
+                    // The columns are still on their way: Automatic and
+                    // No label can be chosen at once, and the rest shows
+                    // when it comes.
+                    let waited = picker.label == Columns::Waiting;
+                    Some(match &picker.label {
+                        Columns::Known(label) if label.source == Source::KeysOnly => Asked::Nothing,
+                        Columns::Known(_) | Columns::Waiting => {
+                            Asked::Open(Chooser::open(&lines, choice, waited))
+                        }
+                    })
+                }
+            }
+        });
+        let Some(asked) = asked else {
+            return;
+        };
+        // A picker notes that it was asked, to say why nothing opens.
+        if let Asked::Nothing = asked {
+            if let Some(picker) = self
+                .editor_mut(tab, id)
+                .and_then(|editor| editor.picker.as_mut())
+            {
+                picker.nothing = Some(Instant::now());
+            }
+            return;
+        }
+        if let Some(picker) = self
+            .editor_mut(tab, id)
+            .and_then(|editor| editor.picker.as_mut())
+        {
+            picker.nothing = None;
+        }
+        if let Some(home) = self.chooser_mut(tab, id) {
+            *home = match asked {
+                Asked::Open(chooser) => Some(chooser),
+                Asked::Close | Asked::Nothing => None,
+            };
+        }
+    }
+
+    /// Closes the chooser of the tab `id` and chooses nothing.
+    pub(super) fn close_label_chooser(&mut self, tab: ConnTabId, id: TabId) {
+        if let Some(home) = self.chooser_mut(tab, id) {
+            *home = None;
+        }
+    }
+
+    /// Lets `act` move the keys of the chooser that is open in the tab
+    /// `id`, among its lines as they are now.
     pub(super) fn in_label_chooser(
         &mut self,
         tab: ConnTabId,
         id: TabId,
         act: impl FnOnce(&mut Chooser, &[Line]),
     ) {
-        let lines = self.workspace(tab).and_then(|workspace| {
-            let editor = workspace.object_tab(id)?.edits.editor.as_ref()?;
-            let picker = editor.picker.as_ref()?;
-            picker.chooser.as_ref()?;
-            Some(workspace.label_lines(&picker.target, &picker.key))
-        });
-        let chooser = self
-            .editor_mut(tab, id)
-            .and_then(|editor| editor.picker.as_mut())
-            .and_then(|picker| picker.chooser.as_mut());
-        if let (Some(lines), Some(chooser)) = (lines, chooser) {
+        let Some((_, lines)) = self.chooser_lines(tab, id) else {
+            return;
+        };
+        let chooser = self.chooser_mut(tab, id).and_then(Option::as_mut);
+        if let Some(chooser) = chooser {
             act(chooser, &lines);
         }
     }
 
-    /// Chooses `line` of the chooser that is open beside the picker of
-    /// the tab's editor, or the line its keys are on. A line that cannot
-    /// be chosen (the key, a column of bytes) chooses nothing, and the
-    /// chooser stays. The editor stays open with its text as it is: no
-    /// value of the cell is touched.
+    /// Chooses `line` of the chooser that is open in the tab `id`, or the
+    /// line its keys are on. A line that cannot be chosen (the key, a
+    /// column of bytes) chooses nothing, and the chooser stays. Under an
+    /// editor the editor stays open with its text as it is: no value of
+    /// the cell is touched.
     pub(super) fn choose_label(&mut self, tab: ConnTabId, id: TabId, line: Option<usize>) {
-        let planned = self.workspace(tab).and_then(|workspace| {
-            let editor = workspace.object_tab(id)?.edits.editor.as_ref()?;
-            let picker = editor.picker.as_ref()?;
-            let chooser = picker.chooser.as_ref()?;
-            let lines = workspace.label_lines(&picker.target, &picker.key);
-            let choice = lines.get(line.unwrap_or(chooser.at))?.choice()?;
-            Some((picker.target.clone(), choice))
-        });
-        let Some((table, choice)) = planned else {
+        let Some((table, lines)) = self.chooser_lines(tab, id) else {
             return;
         };
-        if let Some(picker) = self
-            .editor_mut(tab, id)
-            .and_then(|editor| editor.picker.as_mut())
-        {
-            picker.chooser = None;
-        }
+        let at = self
+            .chooser_mut(tab, id)
+            .and_then(|home| home.as_ref())
+            .map(|chooser| line.unwrap_or(chooser.at));
+        let choice = at.and_then(|at| lines.get(at)).and_then(Line::choice);
+        let Some(choice) = choice else {
+            return;
+        };
+        self.close_label_chooser(tab, id);
         self.set_label(tab, &table, choice);
     }
 
+    /// Moves the cursor of the Structure view's columns `by` lines,
+    /// stopping at the first and at the last.
+    pub(super) fn move_structure_cursor(&mut self, tab: ConnTabId, id: TabId, by: isize) {
+        let Some(object) = self.object_tab_mut(tab, id) else {
+            return;
+        };
+        let last = object
+            .structure
+            .value
+            .as_ref()
+            .map_or(0, |structure| structure.columns.len().saturating_sub(1));
+        object.structure_cursor = object.structure_cursor.saturating_add_signed(by).min(last);
+        self.forget_refusal(tab);
+    }
+
+    /// What was said of a column that was not taken was said of the line
+    /// the cursor was on, or of the name that was typed: it goes when the
+    /// cursor moves, a label is kept or the view changes.
+    pub(super) fn forget_refusal(&mut self, tab: ConnTabId) {
+        if let Some(workspace) = self.workspace_mut(tab) {
+            workspace.label_refused = None;
+        }
+    }
+
+    /// Puts that cursor on `line`, where there is such a line.
+    pub(super) fn set_structure_cursor(&mut self, tab: ConnTabId, id: TabId, line: usize) {
+        let Some(object) = self.object_tab_mut(tab, id) else {
+            return;
+        };
+        let columns = object
+            .structure
+            .value
+            .as_ref()
+            .map_or(0, |structure| structure.columns.len());
+        if line < columns {
+            object.structure_cursor = line;
+        }
+        self.forget_refusal(tab);
+    }
+
+    /// The table the tab `id` shows, where it is a table and its
+    /// structure is known: what the Structure view's door chooses for.
+    fn own_table(&self, tab: ConnTabId, id: TabId) -> Option<ObjectRef> {
+        self.structure_label(tab, id)?;
+        let object = self.workspace(tab)?.object_tab(id)?;
+        Some(object.object.clone())
+    }
+
+    /// Keeps `choice` for the table the tab `id` shows: the Structure
+    /// view's Use Automatic, and the prompt's lines that name no column.
+    pub(super) fn set_tab_label(&mut self, tab: ConnTabId, id: TabId, choice: LabelChoice) {
+        let Some(table) = self.own_table(tab, id) else {
+            return;
+        };
+        self.close_label_chooser(tab, id);
+        self.set_label(tab, &table, choice);
+    }
+
+    /// Uses the column on `line` of the Structure view of the tab `id` as
+    /// what labels its table's rows, or the column under the cursor.
+    pub(super) fn use_as_label(&mut self, tab: ConnTabId, id: TabId, line: Option<usize>) {
+        let name = self.workspace(tab).and_then(|workspace| {
+            let object = workspace.object_tab(id)?;
+            let structure = object.structure.value.as_ref()?;
+            let line = line.unwrap_or(object.structure_cursor);
+            Some(structure.columns.get(line)?.name.clone())
+        });
+        if let Some(name) = name {
+            self.label_by_name(tab, id, &name);
+        }
+    }
+
+    /// Labels the table the tab `id` shows by its column `name`, named by
+    /// hand: a line of the prompt, the key on a line of the Structure
+    /// view, a column's menu. A name that is no column's, the key and a
+    /// column of bytes are refused, and the workspace keeps why to say it
+    /// until the next key.
+    pub(super) fn label_by_name(&mut self, tab: ConnTabId, id: TabId, name: &str) {
+        let asked = self.workspace(tab).and_then(|workspace| {
+            let object = workspace.object_tab(id)?;
+            let structure = object.structure.value.as_ref()?;
+            let dialect = workspace.driver.dialect();
+            let keys = keys_of(structure);
+            Some(labels::choosable(dialect, &structure.columns, &keys, name))
+        });
+        let (Some(table), Some(asked)) = (self.own_table(tab, id), asked) else {
+            return;
+        };
+        match asked {
+            Ok(()) => {
+                self.close_label_chooser(tab, id);
+                self.set_label(tab, &table, LabelChoice::Column(name.to_owned()));
+            }
+            Err(why) => {
+                if let Some(workspace) = self.workspace_mut(tab) {
+                    workspace.label_refused = Some((id, name.to_owned(), why));
+                }
+            }
+        }
+    }
+
     /// Keeps `choice` as what labels the rows of `table` on the saved
     /// connection of `tab`, on this computer: nothing is sent to the
     /// database but the searches of the pickers that are open on the
@@ -176,6 +368,7 @@ impl App {
             return false;
         };
         let conn = workspace.conn_id.clone();
+        self.forget_refusal(tab);
         if !self.labels.choose(&conn, table, choice) {
             return false;
         }
@@ -262,3 +455,9 @@ impl App {
         }
     }
 }
+
+/// The columns a table's rows are found by, for its own Structure view:
+/// its primary key.
+fn keys_of(structure: &Structure) -> Vec<&str> {
+    structure.primary_key.iter().map(String::as_str).collect()
+}
```

```diff
diff --git a/src/app.rs b/src/app.rs
--- a/src/app.rs
+++ b/src/app.rs
@@ -643,6 +643,7 @@ impl App {
                     workspace.command_error = None;
                     workspace.save_refused = false;
                     workspace.review_refused = false;
+                    workspace.label_refused = None;
                 }
             }
             Action::CloseCommand(tab) => {
@@ -652,6 +653,7 @@ impl App {
                     workspace.command_error = None;
                     workspace.save_refused = false;
                     workspace.review_refused = false;
+                    workspace.label_refused = None;
                 }
             }
             Action::RunCommand(tab) => self.run_command(tab),
@@ -753,11 +755,15 @@ impl App {
                 }
                 let describe = self.object_tab_mut(tab, object_tab).is_some_and(|object| {
                     object.view = view;
+                    // A chooser that was open in the Structure view was
+                    // that view's.
+                    object.label_chooser = None;
                     view == ObjectView::Structure && object.structure.needs_load()
                 });
                 if describe {
                     self.describe(tab, object_tab);
                 }
+                self.forget_refusal(tab);
             }
             Action::NextPage { tab, object_tab } => {
                 let moved = self.object_tab_mut(tab, object_tab).is_some_and(|object| {
@@ -1045,14 +1051,16 @@ impl App {
                 self.in_label_chooser(tab, id, |chooser, lines| chooser.jump(lines, to));
             }
             Action::ChooseLabel { tab, id, line } => self.choose_label(tab, id, line),
-            Action::CloseLabelChooser { tab, id } => {
-                if let Some(picker) = self
-                    .editor_mut(tab, id)
-                    .and_then(|editor| editor.picker.as_mut())
-                {
-                    picker.chooser = None;
-                }
+            Action::CloseLabelChooser { tab, id } => self.close_label_chooser(tab, id),
+            Action::MoveStructureCursor { tab, id, by } => {
+                self.move_structure_cursor(tab, id, by);
             }
+            Action::SetStructureCursor { tab, id, line } => {
+                self.set_structure_cursor(tab, id, line);
+            }
+            Action::UseAsLabel { tab, id, line } => self.use_as_label(tab, id, line),
+            Action::SetLabel { tab, id, choice } => self.set_tab_label(tab, id, choice),
+            Action::LabelByName { tab, id, name } => self.label_by_name(tab, id, &name),
             Action::EditorBreak { tab, id } => {
                 if let Some(editor) = self.editor_mut(tab, id) {
                     // At the end of the text, where the cursor of a field
@@ -1513,6 +1521,8 @@ impl App {
                     // From the Structure view: show the data, with the bar.
                     let from_structure = object.view != ObjectView::Data;
                     object.view = ObjectView::Data;
+                    // A chooser that was open in that view was the view's.
+                    object.label_chooser = None;
                     let bar = &mut object.filter;
                     bar.open = !bar.open || from_structure;
                     bar.focus = bar.open;
@@ -2545,6 +2555,12 @@ impl App {
                     let columns = columns.map(|structure| structure.columns.clone());
                     let named = object.object.clone();
                     object.structure.finish(request, result);
+                    // The Structure view's cursor stays on a line there
+                    // is: the table may have fewer columns now.
+                    if let Some(structure) = &object.structure.value {
+                        let last = structure.columns.len().saturating_sub(1);
+                        object.structure_cursor = object.structure_cursor.min(last);
+                    }
                     // The review was made of the structure before this
                     // one, its key and its types: it is made again.
                     object.edits.review = None;
diff --git a/src/app/editing.rs b/src/app/editing.rs
--- a/src/app/editing.rs
+++ b/src/app/editing.rs
@@ -1374,8 +1374,16 @@ impl App {
             Some(_) => Scope::Grid,
             None => Scope::SqlEditor,
         };
+        // The Structure view's own lines hold while it shows.
+        let structure = workspace
+            .active_object_tab()
+            .is_some_and(|object| object.view == crate::model::ObjectView::Structure);
+        let holds = |when: crate::keymap::When| match when {
+            crate::keymap::When::Structure => structure,
+            _ => false,
+        };
         // The prompt is Omarchy's, in whatever look it is driven.
-        let read = self.keymap.ex(Layout::Omarchy, scope, &|_| false, line);
+        let read = self.keymap.ex(Layout::Omarchy, scope, &holds, line);
         let refused = |app: &mut Self| {
             if let Some(workspace) = app.workspace_mut(tab) {
                 workspace.command_error = Some(line.to_owned());
@@ -1399,6 +1407,30 @@ impl App {
                 }
                 return;
             }
+            // What labels the table's rows for the foreign keys that point
+            // at it, from its Structure view: the chooser, Automatic, the
+            // keys alone, or the column the line names.
+            (Typed::Command(Command::OpenLabelChooser), Some((id, _))) => {
+                Action::ToggleLabelChooser { tab, id }
+            }
+            (Typed::Command(Command::LabelAutomatic), Some((id, _))) => {
+                let choice = crate::labels::LabelChoice::Automatic;
+                Action::SetLabel { tab, id, choice }
+            }
+            (Typed::Command(Command::LabelKeysOnly), Some((id, _))) => {
+                let choice = crate::labels::LabelChoice::NoLabel;
+                Action::SetLabel { tab, id, choice }
+            }
+            (Typed::Command(Command::LabelColumn), Some((id, _))) => {
+                let command = Command::LabelColumn;
+                let named = self
+                    .keymap
+                    .ex_argument(Layout::Omarchy, command, scope, line);
+                let Some(name) = named.map(str::to_owned) else {
+                    return refused(self);
+                };
+                Action::LabelByName { tab, id, name }
+            }
             // A SQL editor's lines set the mode of its runs, as its switch
             // does. Where the tab cannot write the mode stays, as it does
             // under the switch.
```

- [ ] **Step 5: The keys**

`structure` is the table on screen while its Structure view shows; `on_structure` is that view with the keys. The arrows are the tree's only where nothing else has them, and the Structure view now is something else. The chooser's keys are read where a chooser is open, beside a picker or in the view.

```diff
diff --git a/src/ui/keys.rs b/src/ui/keys.rs
--- a/src/ui/keys.rs
+++ b/src/ui/keys.rs
@@ -401,7 +401,9 @@ pub fn handle(app: &mut App, ctx: &egui::Context) {
         .map_or((false, false), |workspace| {
             let prompt = workspace.command.is_some();
             let save = workspace.save_refused && !prompt;
-            let said = workspace.command_error.is_some() || workspace.review_refused;
+            let said = workspace.command_error.is_some()
+                || workspace.review_refused
+                || workspace.label_refused.is_some();
             (prompt, said || save)
         });
     // An editor that just opened takes the keyboard when its field is
@@ -451,10 +453,22 @@ pub fn handle(app: &mut App, ctx: &egui::Context) {
     // Arrows go to the tree when the user last worked there (or no grid is
     // showing), else to the grid.
     let any_grid = grid || sql_grid;
+    // The Structure view of the table on screen: its columns stand where
+    // the rows do, and have a cursor of their own.
+    let structure = object.filter(|&(tab, id)| {
+        app.workspace(tab)
+            .and_then(|workspace| workspace.object_tab(id))
+            .is_some_and(|object| object.view == crate::model::ObjectView::Structure)
+    });
     let tree_arrows = !editing
-        && app
-            .workspace(active)
-            .is_some_and(|workspace| workspace.pane == crate::model::Pane::Tree || !any_grid);
+        && app.workspace(active).is_some_and(|workspace| {
+            let nowhere_else = !any_grid && structure.is_none();
+            workspace.pane == crate::model::Pane::Tree || nowhere_else
+        });
+    // That view has the keys: the tree does not, nor a field, a button or
+    // a dialog.
+    let on_structure =
+        structure.filter(|_| !editing && !tree_arrows && !focused && app.dialog.is_none());
     // The SQL editor on screen, while it has the keyboard: the completion
     // list's keys are its keys, and no other pane's. (A text field then
     // has the keyboard: no need to ask `editing` as well.)
@@ -552,13 +566,22 @@ pub fn handle(app: &mut App, ctx: &egui::Context) {
     // It has no field, and every key and letter of the frame is its own:
     // they are taken here, before the shortcuts below and before the
     // picker's field, whose text and cursor stay as they are.
-    let choosing = picking.filter(|&(tab, id)| {
+    let beside_picker = picking.filter(|&(tab, id)| {
         app.workspace(tab)
             .and_then(|workspace| workspace.object_tab(id))
             .and_then(|object| object.edits.editor.as_ref())
             .and_then(|editor| editor.picker.as_ref())
             .is_some_and(|picker| picker.chooser.is_some())
     });
+    // Or it is open in the Structure view, which has no field at all.
+    let in_structure = structure.filter(|&(tab, id)| {
+        app.dialog.is_none()
+            && app
+                .workspace(tab)
+                .and_then(|workspace| workspace.object_tab(id))
+                .is_some_and(|object| object.label_chooser.is_some())
+    });
+    let choosing = beside_picker.or(in_structure);
     if let Some(chooser) = choosing {
         chooser_keys(ctx, chooser, (&app.keymap, layout), &mut actions);
     }
@@ -709,6 +732,25 @@ pub fn handle(app: &mut App, ctx: &egui::Context) {
         if on(input, Command::ToggleSidebar, Scope::Global).count > 0 {
             actions.push(Action::ToggleSidebar(active));
         }
+        // The Structure view's keys: its cursor, one line for each press,
+        // the chooser of the table's label, and the column under the
+        // cursor as that label. Fresh presses only for the last two: held,
+        // the first would open the chooser and close it by turns.
+        if let Some((tab, id)) = on_structure {
+            let scope = Scope::Grid;
+            for keys in keymap.chords_now(layout, Command::MoveInStructure, scope, normal) {
+                let by = if keys == "up" { -1 } else { 1 };
+                let presses = presses(input, layout, keys).count;
+                actions.extend((0..presses).map(|_| Action::MoveStructureCursor { tab, id, by }));
+            }
+            if on(input, Command::OpenLabelChooser, scope).fresh {
+                actions.push(Action::ToggleLabelChooser { tab, id });
+            }
+            if on(input, Command::UseAsLabel, scope).fresh {
+                let line = None;
+                actions.push(Action::UseAsLabel { tab, id, line });
+            }
+        }
         if tree_arrows && !focused {
             use crate::model::TreeKey;
             let tab = active;
@@ -1685,6 +1727,8 @@ fn letters(
     // The rows of the table show: where the letters that edit a cell and
     // the one that drops a row are read, and not in the Structure view.
     let rows_show = object.is_some_and(|object| object.view == crate::model::ObjectView::Data);
+    let structure_shows =
+        object.is_some_and(|object| object.view == crate::model::ObjectView::Structure);
     // The cursor's row is a new one: the row panel is its form, and the
     // key that drops the row is read from the form's fields too.
     let on_new = selection.is_some_and(|cell| crate::edit::new_id(cell.row).is_some());
@@ -1826,6 +1870,9 @@ fn letters(
         crate::keymap::When::Note => note.is_some(),
         crate::keymap::When::RefusedWrite => has_card,
         crate::keymap::When::NewRow => on_new,
+        // The Structure view shows where the rows would: its own keys
+        // come before the rows' there.
+        crate::keymap::When::Structure => structure_shows,
         _ => false,
     };
     let review_only = |when: crate::keymap::When| when == crate::keymap::When::Review;
@@ -1992,6 +2039,16 @@ fn letters(
             (Some(Command::OpenReferencedRow), Some(object_tab)) => {
                 actions.push(Action::FollowSelectedKey { tab, object_tab });
             }
+            // The Structure view's cursor, and the column under it as
+            // what labels the table's rows.
+            (Some(Command::MoveInStructure), Some(id)) => {
+                let by = if char == 'k' { -1 } else { 1 };
+                actions.push(Action::MoveStructureCursor { tab, id, by });
+            }
+            (Some(Command::UseAsLabel), Some(id)) => {
+                let line = None;
+                actions.push(Action::UseAsLabel { tab, id, line });
+            }
             (Some(Command::CopyCells), Some(_)) => {
                 if let Some(text) = app.copy_text(false) {
                     ctx.copy_text(text);
```

- [ ] **Step 6: The chooser stands under what opened it, with no field**

Under an anchor it hangs from the anchor's left edge, or from its right edge where that would pass the window's: the Change… button ends its card.

```diff
diff --git a/src/ui/key_picker.rs b/src/ui/key_picker.rs
--- a/src/ui/key_picker.rs
+++ b/src/ui/key_picker.rs
@@ -984,7 +984,9 @@ pub fn show(
         });
     if let Some(open) = names.chooser {
         let list = Rect::from_min_size(placed.pos, size);
-        let chooser = label_chooser::show(ui, open, list, field, (look, palette, locale));
+        let place = label_chooser::Place::Beside(list);
+        let skin = (look, palette, locale);
+        let chooser = label_chooser::show(ui, open, place, Some(field), skin);
         shown.pressed |= chooser.pressed;
         match chooser.asked {
             Some(label_chooser::Asked::Choose(line)) => {
diff --git a/src/ui/label_chooser.rs b/src/ui/label_chooser.rs
--- a/src/ui/label_chooser.rs
+++ b/src/ui/label_chooser.rs
@@ -37,6 +37,17 @@ pub struct Open {
     pub connection: (String, Environment),
 }
 
+/// Where the chooser stands.
+#[derive(Debug, Clone, Copy)]
+pub enum Place {
+    /// Beside a picker's list: to its right where the window has the
+    /// room, else to its left.
+    Beside(Rect),
+    /// Under what opened it in the Structure view, their left edges
+    /// together.
+    Under(Rect),
+}
+
 /// What was asked of the chooser this frame.
 #[derive(Debug, Clone, Copy, PartialEq, Eq)]
 pub enum Asked {
@@ -122,7 +133,7 @@ fn keys_line(ui: &Ui, look: &Look) -> String {
 }
 
 /// Why a column cannot be chosen, in the look's words.
-fn reason(excluded: Excluded, look: &Look, locale: Locale) -> String {
+pub fn reason(excluded: Excluded, look: &Look, locale: Locale) -> String {
     let words = match excluded {
         Excluded::Key => "the key",
         Excluded::Binary => "binary",
@@ -220,34 +231,49 @@ fn words(ui: &mut Ui, text: &str, color: egui::Color32, look: &Look) {
         .label(ui);
 }
 
-/// Draws the chooser beside the list at `beside`, on the side that has
-/// the room. `field` is the picker's field, which has the keyboard: its
-/// node says which line the keys are on.
+/// Draws the chooser at `place`. `field` is the picker's field, which
+/// has the keyboard: its node says which line the keys are on. The
+/// Structure view has no field, and the keys are the view's.
 pub fn show(
     ui: &Ui,
     open: &Open,
-    beside: Rect,
-    field: Id,
+    place: Place,
+    field: Option<Id>,
     (look, palette, locale): (&Look, &Palette, Locale),
 ) -> Shown {
     let frame = menu::frame(look, palette);
     let (wanted, gap) = (width(look), gap(look));
     let screen = ui.ctx().content_rect();
-    // To the list's right where the window has the room, else to its
-    // left. Where neither side has it, on the side with more, as wide as
-    // that side lets it be and no narrower than a menu reads at: then it
-    // stands over the list's edge.
-    let right = screen.right() - beside.right() - gap;
-    let before = beside.left() - gap - screen.left();
-    let (left, width) = if right >= wanted {
-        (beside.right() + gap, wanted)
-    } else if before >= wanted {
-        (beside.left() - gap - wanted, wanted)
-    } else if right >= before {
-        (beside.right() + gap, right.clamp(NARROW, wanted))
-    } else {
-        let width = before.clamp(NARROW, wanted);
-        (beside.left() - gap - width, width)
+    let (left, top, width) = match place {
+        Place::Beside(beside) => {
+            // To the list's right where the window has the room, else to
+            // its left. Where neither side has it, on the side with more,
+            // as wide as that side lets it be and no narrower than a menu
+            // reads at: then it stands over the list's edge.
+            let right = screen.right() - beside.right() - gap;
+            let before = beside.left() - gap - screen.left();
+            let (left, width) = if right >= wanted {
+                (beside.right() + gap, wanted)
+            } else if before >= wanted {
+                (beside.left() - gap - wanted, wanted)
+            } else if right >= before {
+                (beside.right() + gap, right.clamp(NARROW, wanted))
+            } else {
+                let width = before.clamp(NARROW, wanted);
+                (beside.left() - gap - width, width)
+            };
+            (left, beside.top(), width)
+        }
+        // From the anchor's left edge, or from its right where that would
+        // pass the window's: a button that ends its line.
+        Place::Under(anchor) => {
+            let left = if anchor.left() + wanted > screen.right() {
+                anchor.right() - wanted
+            } else {
+                anchor.left()
+            };
+            (left, anchor.bottom() + gap, wanted)
+        }
     };
     let id = Id::new("label-chooser");
     let menu_id = id.with("menu");
@@ -265,7 +291,7 @@ pub fn show(
         .order(egui::Order::Foreground)
         .fade_in(false)
         .constrain(true)
-        .fixed_pos(pos2(left, beside.top()))
+        .fixed_pos(pos2(left, top))
         .show(ui.ctx(), |ui| {
             frame.show(ui, |ui| {
                 ui.set_width(width - frame.total_margin().sum().x);
@@ -323,7 +349,7 @@ pub fn show(
                             ui.ctx().accesskit_node_builder(response.id, |node| {
                                 node.set_role(Role::MenuItemRadio);
                             });
-                            if lit {
+                            if let (true, Some(field)) = (lit, field) {
                                 // The keyboard stays in the field: its node
                                 // says which line is the current one.
                                 ui.ctx().accesskit_node_builder(field, |node| {
```

- [ ] **Step 7: The view**

The label's part stands over the columns. In the looks with a pointer it is a card across the view, with the line and its sentence at the left and the buttons at the card's end, in the board's order (their widths are measured first, so they are laid left to right and Tab reads them in that order). In the terminal look it is a bare line with the keys. The line is a group for a screen reader. The columns are one Tab stop, as large as the view's window (`ui.clip_rect()`: a stop that follows the table would leave the window when the view is scrolled), registered as the grid's pane (`focus::region(.., Region::Grid, ..)`), as the rows' grid is: the pane keys and F6 bring the keyboard to it, and gaining it gives the view the keys. Each column's line gets its place from the name's cell, once the grid is drawn: a click puts the cursor on it and gives the view the keys, a secondary click opens its menu, and Tab passes it by (`Sense::CLICK`). The cursor's fill is painted into a slot reserved before the line's words, so it lies under them and over the grid's stripe. A cursor that moved is brought into view with a jump, as the tree brings its own (`ScrollAnimation::none()`: the next key may come at once), so the view's keys never act on a line that is out of sight; a refusal that was just said is brought into view the same way. The jump is up and down only: the view stays where it was scrolled sideways. Both remember what they last brought into view (egui's temporary data), so a wheel can scroll away from the cursor afterwards. The cursor's memory is forgotten while the view has no keys, so the line is brought into view again when the keys come back; and the scroll area is salted with the tab, so each table's view keeps its own place.

```diff
diff --git a/src/ui/structure.rs b/src/ui/structure.rs
--- a/src/ui/structure.rs
+++ b/src/ui/structure.rs
@@ -1,21 +1,316 @@
 //! The Structure view: columns, indexes and foreign keys as plain tables.
+//! Over them, for a table, what labels its rows in the pickers of the
+//! foreign keys that point at it, and the ways to change that here: the
+//! chooser, the column under the view's cursor, a column's menu.
+
+use egui::accesskit::Role;
+use egui::{CornerRadius, Frame, Rect, Sense, Stroke, WidgetInfo, WidgetType, vec2};
 
 use crate::app::App;
-use crate::i18n::gettext;
-use crate::model::{Action, ConnTabId, TabId};
+use crate::i18n::{Locale, gettext};
+use crate::keymap::{Command, Scope};
+use crate::labels::{self, Refused, Resolved, Source};
+use crate::model::{Action, ConnTabId, Pane, TabId};
 use crate::theme::{self, Look, Palette};
-use crate::typography::TextRole;
+use crate::typography::{Text, TextRole};
 use crate::ui::format::{display_safe, for_display};
-use crate::ui::widgets;
+use crate::ui::{focus, label_chooser, menu, states, widgets};
 
 /// One cell of a structure table: body text, or code.
-fn cell(ui: &mut egui::Ui, text: &str, code: bool, look: &Look, palette: &Palette) {
+fn cell(
+    ui: &mut egui::Ui,
+    text: &str,
+    code: bool,
+    look: &Look,
+    palette: &Palette,
+) -> egui::Response {
     let role = if code {
         widgets::code(look)
     } else {
         widgets::body(look)
     };
-    widgets::label(ui, role, text, palette.text, look);
+    widgets::label(ui, role, text, palette.text, look)
+}
+
+/// What labels the table's rows, as the view's line words it, and where
+/// the column's name stands in the words: "name · Automatic". Of a table
+/// with nothing to choose it says so.
+pub fn label_words(
+    label: &Resolved,
+    table: &str,
+    look: &Look,
+    locale: Locale,
+) -> (String, Option<std::ops::Range<usize>>) {
+    let words = match (&label.column, label.source) {
+        (_, Source::KeysOnly) => {
+            return (super::key_picker::nothing_text(table, look, locale), None);
+        }
+        (Some(_), Source::Automatic) => "{column} · Automatic",
+        (Some(_), Source::User) => "{column} · Set by you",
+        (None, Source::Automatic) => "Keys only · Automatic",
+        (None, Source::User) => "Keys only · Set by you",
+    };
+    let words = look.label(&gettext(locale, words));
+    let Some((column, at)) = label.column.as_deref().zip(words.find("{column}")) else {
+        return (words, None);
+    };
+    // A placeholder: a translation puts the name where it reads.
+    let column = display_safe(column);
+    let said = words.replacen("{column}", &column, 1);
+    (said, Some(at..at + column.len()))
+}
+
+/// Why a column that was named for the table's label was not taken: "id
+/// cannot be the label (the key)".
+pub fn refused_text(name: &str, why: Refused, table: &str, look: &Look, locale: Locale) -> String {
+    let name = display_safe(name);
+    match why {
+        Refused::NoColumn => look
+            .label(&gettext(locale, "No column {column} in {table}"))
+            .replace("{column}", &name)
+            .replace("{table}", &display_safe(table)),
+        Refused::Excluded(excluded) => {
+            let reason = label_chooser::reason(excluded, look, locale);
+            look.label(&gettext(locale, "{column} cannot be the label ({reason})"))
+                .replace("{column}", &name)
+                .replace("{reason}", &reason)
+        }
+    }
+}
+
+/// How tall the buttons of the label's line are.
+const BUTTON: f32 = 26.0;
+
+/// The widest the sentence under the label's line runs before it wraps,
+/// in the terminal look. In a card it has the card's room, and no less
+/// than `NARROW` in a window too narrow for the buttons beside it.
+const HELP: f32 = 640.0;
+const NARROW: f32 = 240.0;
+
+/// The value tags' slot the label's tag is drawn in: their blue.
+const TAG: usize = 1;
+
+/// What the label's part of the view was asked this frame.
+#[derive(Default)]
+struct Asked {
+    /// Change… was clicked.
+    change: bool,
+    /// Use Automatic was clicked.
+    automatic: bool,
+    /// What the chooser hangs under.
+    anchor: Option<Rect>,
+    /// The button that opens it, where the look has one.
+    button: Option<Rect>,
+}
+
+/// The label's part of the view, over the columns: what labels the
+/// table's rows and who chose it, the ways to change it (buttons in the
+/// looks with a pointer, the keys in the terminal look), and a sentence
+/// that says what it is for. The looks with a pointer draw it as a card
+/// across the view, with the buttons at its end. `refused` is why a column
+/// that was named was not taken, while that is to be said here.
+fn label_part(
+    ui: &mut egui::Ui,
+    label: &Resolved,
+    (table, refused): (&str, Option<&str>),
+    (look, palette, locale): (&Look, &Palette, Locale),
+) -> Asked {
+    let mut asked = Asked::default();
+    let (words, column) = label_words(label, table, look, locale);
+    let at = column.unwrap_or(words.len()..words.len());
+    let (body, small) = (widgets::body(look), widgets::secondary(look));
+    let nothing = label.source == Source::KeysOnly;
+    let key = |command| crate::ui::keys::written_in(ui.ctx(), look, command, Scope::Grid);
+    let (choose_key, use_key) = (key(Command::OpenLabelChooser), key(Command::UseAsLabel));
+    let automatic_key = key(Command::LabelAutomatic);
+    let title = if look.terminal {
+        gettext(locale, "fk label")
+    } else {
+        gettext(locale, "Label for foreign keys")
+    };
+    let automatic = gettext(locale, "Use Automatic");
+    let (change, name) = (
+        gettext(locale, "Change…"),
+        gettext(locale, "Change label column"),
+    );
+    let why = super::key_picker::nothing_text(table, look, locale);
+    // The way back, where the user chose, and what changes the label.
+    let back = || widgets::ButtonSpec::new(&automatic).padding(10.0);
+    let changes = || {
+        let button = widgets::ButtonSpec::new(&change)
+            .padding(10.0)
+            .shortcut(&choose_key)
+            .label(&name);
+        if nothing {
+            button.disabled(&why)
+        } else {
+            button
+        }
+    };
+    let chosen = label.source == Source::User;
+    let mut part = |ui: &mut egui::Ui| {
+        ui.horizontal(|ui| {
+            // The buttons end the card: the words have the room before
+            // them.
+            let buttons = if look.terminal {
+                0.0
+            } else if chosen {
+                back().width(ui, look) + ui.spacing().item_spacing.x + changes().width(ui, look)
+            } else {
+                changes().width(ui, look)
+            };
+            let room = ui.available_width() - buttons - 16.0;
+            ui.vertical(|ui| {
+                let line = ui.horizontal(|ui| {
+                    let role = TextRole::pick(look, TextRole::UiBodySemibold, TextRole::OBody);
+                    let title = widgets::label(ui, role, &title, palette.text, look);
+                    asked.anchor = Some(title.rect);
+                    ui.add_space(8.0);
+                    Text::new(look)
+                        .add(body, &words[..at.start], palette.secondary)
+                        .add(widgets::code(look), &words[at.clone()], palette.text)
+                        .add(body, &words[at.end..], palette.secondary)
+                        .layout(ui.ctx())
+                        .label(ui);
+                    if !look.terminal {
+                        return;
+                    }
+                    ui.add_space(16.0);
+                    // No buttons in this look: the keys. Where the user
+                    // chose, the one that hands the choice back; else the
+                    // two that choose, where there is anything to choose.
+                    let keys = match label.source {
+                        Source::KeysOnly => None,
+                        Source::User => {
+                            let back = gettext(locale, "back to automatic");
+                            let keys = Text::new(look)
+                                .add(small, &automatic_key, palette.text)
+                                .space(small, " ")
+                                .add(small, &back, palette.dim);
+                            Some(keys)
+                        }
+                        Source::Automatic => {
+                            let chooses = gettext(locale, "choose");
+                            let uses = gettext(locale, "use the column under the cursor");
+                            let keys = Text::new(look)
+                                .add(small, &choose_key, palette.text)
+                                .space(small, " ")
+                                .add(small, &chooses, palette.dim)
+                                .add(small, " · ", palette.dim)
+                                .add(small, &use_key, palette.text)
+                                .space(small, " ")
+                                .add(small, &uses, palette.dim);
+                            Some(keys)
+                        }
+                    };
+                    if let Some(keys) = keys {
+                        keys.layout(ui.ctx()).label(ui);
+                    }
+                });
+                // The line as one thing, for a screen reader: "Label for
+                // foreign keys: name, Automatic".
+                let group = gettext(locale, "Label for foreign keys: {label}")
+                    .replace("{label}", &words.replace(" · ", ", "));
+                let whole = ui.interact(
+                    line.response.rect,
+                    ui.id().with("label-for-foreign-keys"),
+                    Sense::hover(),
+                );
+                whole.widget_info(|| WidgetInfo::labeled(WidgetType::Other, true, &group));
+                ui.ctx().accesskit_node_builder(whole.id, |node| {
+                    node.set_role(Role::Group);
+                });
+                let (help, wrap) = if look.terminal {
+                    (
+                        "Fields that point at {table} show and search the label column · on this computer only, nothing is written to the database",
+                        HELP,
+                    )
+                } else {
+                    (
+                        "Fields that point at {table} show and search this column. On this computer only; nothing is written to the database.",
+                        room.max(NARROW),
+                    )
+                };
+                if look.terminal {
+                    ui.add_space(4.0);
+                } else {
+                    // The card's lines stand close, as the design's do.
+                    ui.spacing_mut().item_spacing.y = 2.0;
+                }
+                let help = look
+                    .label(&gettext(locale, help))
+                    .replace("{table}", &display_safe(table));
+                Text::one(look, small, &help, palette.secondary)
+                    .wrap(wrap)
+                    .layout(ui.ctx())
+                    .label(ui);
+                let seen = ui.id().with("label-refused");
+                if let Some(refused) = refused {
+                    let danger = states::Tone::Danger.color(palette);
+                    let said = widgets::label(ui, small, refused, danger, look);
+                    // It is said at the view's top, and the cursor it was
+                    // asked on may be far down: what was just said is
+                    // brought into view.
+                    let was: Option<String> = ui.data(|data| data.get_temp(seen));
+                    if was.as_deref() != Some(refused) {
+                        ui.data_mut(|data| data.insert_temp(seen, refused.to_owned()));
+                        said.scroll_to_me_animation(None, egui::style::ScrollAnimation::none());
+                    }
+                } else {
+                    ui.data_mut(|data| data.remove_temp::<String>(seen));
+                }
+            });
+            if look.terminal {
+                return;
+            }
+            ui.add_space((ui.available_width() - buttons).max(16.0));
+            if chosen {
+                asked.automatic = back().show(ui, BUTTON, look, palette).clicked();
+            }
+            let response = changes().show(ui, BUTTON, look, palette);
+            asked.change = response.clicked();
+            asked.anchor = Some(response.rect);
+            asked.button = Some(response.rect);
+        });
+    };
+    if look.terminal {
+        part(ui);
+    } else {
+        Frame::new()
+            .fill(palette.panel)
+            // The design's 1 pt border outside 12 and 10 of padding.
+            .stroke(Stroke::new(1.0, palette.surface_hover))
+            .corner_radius(CornerRadius::same(look.radius))
+            .inner_margin(egui::Margin::symmetric(12, 10))
+            .show(ui, |ui| {
+                ui.set_width(ui.available_width());
+                part(ui);
+            });
+    }
+    ui.add_space(16.0);
+    asked
+}
+
+/// The mark on the name of the column that labels the table's rows: a
+/// tag in the value tags' blue, and in the terminal look an arrow at the
+/// name in the accent.
+fn label_tag(ui: &mut egui::Ui, column: &str, (look, palette, locale): (&Look, &Palette, Locale)) {
+    let word = gettext(locale, "label");
+    let rect = if look.terminal {
+        let mark = super::workspace::drawable(ui, TextRole::OBody, look, &["◂", "<"]);
+        let text = format!("{} {word}", mark.unwrap_or("<"));
+        widgets::label(ui, widgets::body(look), &text, palette.accent, look).rect
+    } else {
+        let (color, fill) = super::value_tags::slot_colors(TAG, look, palette);
+        let laid = Text::one(look, TextRole::Shortcut, &word, color).layout(ui.ctx());
+        let (rect, _) = ui.allocate_exact_size(vec2(laid.width() + 12.0, 18.0), Sense::hover());
+        let fill = fill.unwrap_or(palette.surface_hover);
+        ui.painter().rect_filled(rect, CornerRadius::same(4), fill);
+        laid.paint_left(ui.painter(), rect.left() + 6.0, rect.center().y);
+        rect
+    };
+    let name = gettext(locale, "{column}, label column").replace("{column}", &display_safe(column));
+    widgets::announce(ui, rect, &name);
 }
 
 /// Column names as a list, each as `display_safe` shows it.
@@ -36,11 +331,30 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabId)
     let locale = app.locale;
     let palette = app.palette;
     let look = app.look;
-    let Some(object) = app.workspace(tab).and_then(|w| w.object_tab(object_tab)) else {
+    let Some(workspace) = app.workspace(tab) else {
+        return;
+    };
+    let Some(object) = workspace.object_tab(object_tab) else {
         return;
     };
     let mut actions = Vec::new();
     let area = ui.max_rect();
+    let id = object_tab;
+    let skin = (&look, &palette, locale);
+    // What labels the table's rows for the foreign keys that point at
+    // it: none for a view, and none before the structure is known.
+    let label = app.structure_label(tab, id);
+    // Why a column named for it was not taken. The terminal look says it
+    // in its status line.
+    let refused = workspace
+        .label_refused
+        .as_ref()
+        .filter(|(of, _, _)| *of == id && !look.terminal)
+        .map(|(_, name, why)| refused_text(name, *why, &object.object.name, &look, locale));
+    // The view's cursor shows while the view has the keys.
+    let cursor = (workspace.pane == Pane::Grid).then_some(object.structure_cursor);
+    let dialect = workspace.driver.dialect();
+    let mut asked = Asked::default();
     if let Some(error) = super::data_view::shown_error(&object.structure) {
         egui::Frame::new()
             .inner_margin(egui::Margin::same(theme::PAD))
@@ -52,14 +366,27 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabId)
     } else if let Some(structure) = &object.structure.value {
         // Both ways: a long default (a sequence's nextval) must not push
         // the Key column out of reach.
+        // A place of its own for each tab: the cursor's line, once brought
+        // into view, is in view when the tab is come back to.
         egui::ScrollArea::both()
+            .id_salt(("structure", object_tab.0))
             .auto_shrink([false, false])
             .show(ui, |ui| {
                 egui::Frame::new()
                     .inner_margin(egui::Margin::same(theme::PAD))
                     .show(ui, |ui| {
+                        if let Some(label) = &label {
+                            let table = (object.object.name.as_str(), refused.as_deref());
+                            asked = label_part(ui, label, table, skin);
+                        }
                         heading(ui, &gettext(locale, "Columns"), &look, &palette);
-                        egui::Grid::new(("structure-columns", object_tab.0))
+                        // Where each column's line stands, and the fill
+                        // under the cursor's, painted once its place is
+                        // known: under the line's words.
+                        let mut lines = Vec::with_capacity(structure.columns.len());
+                        let mut fill = None;
+                        let labelled = label.as_ref().and_then(|label| label.column.as_deref());
+                        let columns = egui::Grid::new(("structure-columns", object_tab.0))
                             .striped(true)
                             .spacing([16.0, 6.0])
                             .show(ui, |ui| {
@@ -67,8 +394,20 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabId)
                                     quiet(ui, &gettext(locale, title), &look, &palette);
                                 }
                                 ui.end_row();
-                                for column in &structure.columns {
-                                    cell(ui, &display_safe(&column.name), false, &look, &palette);
+                                for (line, column) in structure.columns.iter().enumerate() {
+                                    if cursor == Some(line) {
+                                        fill = Some(ui.painter().add(egui::Shape::Noop));
+                                    }
+                                    let name = ui
+                                        .horizontal(|ui| {
+                                            let name = display_safe(&column.name);
+                                            cell(ui, &name, false, &look, &palette);
+                                            if labelled == Some(column.name.as_str()) {
+                                                label_tag(ui, &column.name, skin);
+                                            }
+                                        })
+                                        .response;
+                                    lines.push(name.rect.y_range());
                                     let type_name = display_safe(&column.type_name);
                                     cell(ui, &type_name, true, &look, &palette);
                                     let nullable = if column.nullable {
@@ -88,7 +427,128 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabId)
                                     cell(ui, key, false, &look, &palette);
                                     ui.end_row();
                                 }
+                            })
+                            .response
+                            .rect;
+                        // The columns are one Tab stop, not one for each
+                        // line: with the keyboard on it the view has the
+                        // keys, and its cursor shows where they are. The
+                        // pane keys and F6 come to it as they come to the
+                        // rows, whose place this view stands in.
+                        // Over what shows of the view, as the rows' grid
+                        // is over its own: the columns may be scrolled
+                        // out of it, and a stop must stand somewhere.
+                        let seen = ui.clip_rect();
+                        let stop = ui.interact(
+                            seen,
+                            ui.id().with(("structure-keys", object_tab.0)),
+                            Sense::focusable_noninteractive(),
+                        );
+                        let named = gettext(locale, "Columns");
+                        stop.widget_info(|| WidgetInfo::labeled(WidgetType::Other, true, &named));
+                        ui.ctx().accesskit_node_builder(stop.id, |node| {
+                            node.set_role(Role::Group);
+                        });
+                        focus::pane(ui, &stop);
+                        focus::hint(ui, &stop, seen, focus::Ring::Own);
+                        focus::region(ui, focus::Region::Grid, seen);
+                        focus::claim(ui, focus::Region::Grid, &stop);
+                        if stop.gained_focus() {
+                            actions.push(Action::GridKeys(tab));
+                        }
+                        // The lines themselves, for the pointer: a click
+                        // puts the cursor on one and gives the view the
+                        // keys, and a line has a menu. Clicked and never
+                        // focused: Tab passes them by.
+                        let keys: Vec<&str> =
+                            structure.primary_key.iter().map(String::as_str).collect();
+                        // The line the cursor was last brought into view
+                        // on. Forgotten while the view has no keys: the
+                        // view may be scrolled meanwhile, and the cursor
+                        // is brought into view when the keys come back.
+                        let seen = ui.id().with(("structure-cursor", object_tab.0));
+                        if cursor.is_none() {
+                            ui.data_mut(|data| data.remove_temp::<usize>(seen));
+                        }
+                        for (line, (column, rows)) in
+                            structure.columns.iter().zip(&lines).enumerate()
+                        {
+                            let rect = Rect::from_x_y_ranges(columns.x_range(), rows.expand(3.0));
+                            let row = ("structure-column", object_tab.0, line);
+                            let response = ui.interact(rect, ui.id().with(row), Sense::CLICK);
+                            if response.clicked() || response.secondary_clicked() {
+                                actions.push(Action::SetStructureCursor { tab, id, line });
+                                actions.push(Action::GridKeys(tab));
+                            }
+                            if cursor == Some(line) {
+                                // A cursor that moved is brought into
+                                // view, as the tree brings its own: the
+                                // view scrolls, and its keys act on the
+                                // line the cursor is on. Jump, not
+                                // animate: the next key may come at once.
+                                let was: Option<usize> = ui.data(|data| data.get_temp(seen));
+                                if was != Some(line) {
+                                    ui.data_mut(|data| data.insert_temp(seen, line));
+                                    let jump = egui::style::ScrollAnimation::none();
+                                    // Up and down only: the line as far
+                                    // across as the window shows, so a
+                                    // view scrolled sideways stays there.
+                                    let across = ui.clip_rect().x_range();
+                                    let shown = Rect::from_x_y_ranges(across, rect.y_range());
+                                    ui.scroll_to_rect_animation(shown, None, jump);
+                                }
+                                if let Some(fill) = fill {
+                                    let shape = egui::Shape::rect_filled(
+                                        rect,
+                                        CornerRadius::ZERO,
+                                        palette.selection,
+                                    );
+                                    ui.painter().set(fill, shape);
+                                }
+                                if look.terminal {
+                                    let bar =
+                                        Text::one(&look, TextRole::OBody, "▌", palette.accent);
+                                    let wide = TextRole::OBody.width(ui.ctx(), look.faces, "▌");
+                                    bar.layout(ui.ctx()).paint_left(
+                                        ui.painter(),
+                                        rect.left() - wide - 2.0,
+                                        rect.center().y,
+                                    );
+                                }
+                            }
+                            // Only a table's columns label anything.
+                            if label.is_none() {
+                                continue;
+                            }
+                            menu::context(&response, &look, &palette, |ui| {
+                                let uses = gettext(locale, "Use as label");
+                                let key = crate::ui::keys::written_in(
+                                    ui.ctx(),
+                                    &look,
+                                    Command::UseAsLabel,
+                                    Scope::Grid,
+                                );
+                                let why = labels::excluded(dialect, column, &keys)
+                                    .map(|why| label_chooser::reason(why, &look, locale));
+                                let mut item = menu::Item::action(&uses).detail(&key);
+                                if let Some(why) = &why {
+                                    item = item.disabled(why);
+                                }
+                                if item.show(ui, &look, &palette).clicked() {
+                                    let line = Some(line);
+                                    actions.push(Action::UseAsLabel { tab, id, line });
+                                    ui.close();
+                                }
+                                let copy = gettext(locale, "Copy name");
+                                if menu::Item::action(&copy)
+                                    .show(ui, &look, &palette)
+                                    .clicked()
+                                {
+                                    ui.ctx().copy_text(column.name.clone());
+                                    ui.close();
+                                }
                             });
+                        }
                         ui.add_space(16.0);
                         heading(ui, &gettext(locale, "Indexes"), &look, &palette);
                         if structure.indexes.is_empty() {
@@ -181,6 +641,59 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabId)
                     });
             });
     }
+    if asked.change {
+        actions.push(Action::ToggleLabelChooser { tab, id });
+    }
+    if asked.automatic {
+        let choice = labels::LabelChoice::Automatic;
+        actions.push(Action::SetLabel { tab, id, choice });
+    }
+    // The chooser, under what opened it. It never takes the keyboard:
+    // `ui::keys` routes its keys, as under a picker.
+    if let (Some(chooser), Some(label), Some(anchor), Some(structure)) = (
+        &object.label_chooser,
+        &label,
+        asked.anchor,
+        &object.structure.value,
+    ) {
+        let keys: Vec<&str> = structure.primary_key.iter().map(String::as_str).collect();
+        let open = label_chooser::Open {
+            at: chooser.at,
+            lines: labels::lines(dialect, Some(&structure.columns), &keys),
+            choice: chooser.choice.clone(),
+            automatic: label.automatic.clone(),
+            loading: false,
+            arrived: false,
+            table: object.object.name.clone(),
+            connection: (workspace.name.clone(), workspace.environment),
+        };
+        let place = label_chooser::Place::Under(anchor);
+        let shown = label_chooser::show(ui, &open, place, None, skin);
+        match shown.asked {
+            Some(label_chooser::Asked::Choose(line)) => {
+                let line = Some(line);
+                actions.push(Action::ChooseLabel { tab, id, line });
+            }
+            Some(label_chooser::Asked::Point(line)) => {
+                actions.push(Action::PointLabelChooser { tab, id, line });
+            }
+            None => {
+                // A press beside it closes it, as it closes a menu. Not
+                // one on the button that opened it, whose click closes it.
+                let pressed = ui.input(|input| {
+                    let at = input.pointer.interact_pos();
+                    at.filter(|_| input.pointer.any_pressed())
+                });
+                let beside = pressed.is_some_and(|at| {
+                    !shown.rect.contains(at)
+                        && !asked.button.is_some_and(|button| button.contains(at))
+                });
+                if beside {
+                    actions.push(Action::CloseLabelChooser { tab, id });
+                }
+            }
+        }
+    }
     // A fetch that has lasted, over whatever is up: a refresh keeps the
     // structure it replaces on screen.
     if let Some(waited) = object.structure.running_for() {
```

- [ ] **Step 8: Run the tests**

Run: `~/.cargo/bin/cargo fmt --all && ~/.cargo/bin/cargo test --locked -p tabletist --lib`
Expected: `test result: ok. 2240 passed; 0 failed; 20 ignored`.

- [ ] **Step 9: Look at it**

Add a throwaway test at the end of `src/shots.rs`:

```rust
#[test]
#[ignore]
fn throwaway_structure() {
    let open = |harness: &mut Harness| {
        let tab = workspace(harness);
        let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
        harness.app.apply(Action::SetView {
            tab,
            object_tab: id,
            view: crate::model::ObjectView::Structure,
        });
        harness.answer_structure(structure());
        harness.app.apply(Action::GridKeys(tab));
        (tab, id)
    };
    both("structure-label", |harness| {
        let (tab, id) = open(harness);
        harness.app.apply(Action::MoveStructureCursor { tab, id, by: 2 });
    });
    both("structure-chooser", |harness| {
        let (tab, id) = open(harness);
        harness.app.apply(Action::UseAsLabel { tab, id, line: Some(2) });
        harness.app.apply(Action::ToggleLabelChooser { tab, id });
    });
    both("structure-refused", |harness| {
        let (tab, id) = open(harness);
        harness.app.apply(Action::UseAsLabel { tab, id, line: Some(0) });
    });
}
```

Run: `~/.cargo/bin/cargo test --locked --features shots --lib shots::throwaway_structure -- --ignored`, then read the pictures in `target/shots/`. Expected, as it was seen where this plan was written:

- `structure-label-macos-light.png`: over Columns, a card across the view: `Label for foreign keys  kind · Automatic`, close under it the sentence that begins `Fields that point at book_images`, and at the card's end the button `Change… ⌘L`; in the table a blue tag `label` after `kind`, and the line of `kind` on the selection's fill.
- `structure-chooser-macos-light.png`: `kind · Set by you`, and at the card's end `Use Automatic` then `Change… ⌘L`; under the second button, from its right edge, the chooser `Label book_images by`, `✓ kind` filled, `id` last and dimmed with `bigint · the key`.
- `structure-label-omarchy-dark.png`: `fk label  kind · automatic` and `:label choose · gl use the column under the cursor`; the sentence in lower case; a `▌` bar before the cursor's line.
- `structure-chooser-omarchy-dark.png`: the chooser under `fk label`, in the accent's border, headed `label book_images by` with `j/k · enter · esc`.
- `structure-refused-macos-light.png`: in the card under the sentence, in the danger colour, `id cannot be the label (the key)`.

Take the test out again and delete the pictures (only the ones this test made: `target/shots/` may hold others). Say in the task's report what you saw.

- [ ] **Step 10: Run the four checks, clippy with `--features shots`, and `tests/keys.rs`**

Expected: all pass.

- [ ] **Step 11: Commit**

```bash
git add src/model.rs src/app.rs src/app/labels.rs src/app/editing.rs src/ui/keys.rs \
  src/ui/label_chooser.rs src/ui/key_picker.rs src/ui/structure.rs src/ui/data_view.rs \
  src/ui/label_tests.rs src/testing.rs
git commit -S -m "Set what labels a foreign key's rows from the table's Structure view

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: The Omarchy status line

**Files:**
- Modify: `src/ui/workspace.rs`
- Test: `src/ui/label_tests.rs`

With the Structure view in front, the status line names that view's keys where it named the rows': the cursor's, and for a table with a label to choose `gl`, `:label` and `:label auto`, as the board has them. What saves and reviews the table's rows stays, as it works from this view too. And the line says why a column that was named is not taken, until the next key, as it says a line that is no command.

- [ ] **Step 1: Write the failing tests**

```diff
diff --git a/src/ui/label_tests.rs b/src/ui/label_tests.rs
--- a/src/ui/label_tests.rs
+++ b/src/ui/label_tests.rs
@@ -1926,3 +1926,79 @@ fn reload_in_the_structure_view_reloads_the_table() {
     harness.press(Key::R, Modifiers::COMMAND);
     assert_eq!(described(&harness), before + 1);
 }
+
+// The Structure door in the Omarchy look's status line.
+
+/// With the Structure view of a table in front the status line names
+/// that view's keys in the rows' place, as the board has them. What saves
+/// the table's rows stays.
+#[test]
+fn the_terminal_looks_status_line_names_the_structure_views_keys() {
+    let (mut harness, tab, _) = publishers_structure_in(Look::omarchy(), false);
+    let painted = |harness: &Harness, hint: &str| harness.painted_rect(hint).is_some();
+    let hints = [
+        "j/k column",
+        "gl use as label",
+        ":label choose",
+        ":label auto",
+        ":w write",
+    ];
+    for hint in hints {
+        assert!(painted(&harness, hint), "{hint}: {:?}", harness.painted);
+    }
+    // No key of the rows', which the view has none of.
+    assert!(!painted(&harness, "j/k row"));
+    // Over the rows the line is the rows' again.
+    let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
+    let (view, object_tab) = (ObjectView::Data, id);
+    harness.app.apply(Action::SetView {
+        tab,
+        object_tab,
+        view,
+    });
+    harness.settle();
+    assert!(painted(&harness, "j/k row"));
+    assert!(!painted(&harness, "gl use as label"));
+    // A table of its key alone has a cursor on its one column and no
+    // label to choose: the line names the first, and not the keys of
+    // the second.
+    let mut harness = Harness::new();
+    harness.set_look(Look::omarchy());
+    let tab = harness.connect_fake_as(false);
+    harness.app.apply(Action::OpenObject {
+        tab,
+        object: ObjectRef::new("main", "isbns"),
+        kind: tabletist_db::ObjectKind::Table,
+        pin: true,
+    });
+    harness.answer_structure(crate::testing::isbns_structure());
+    let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
+    to_structure(&mut harness, tab, id);
+    assert!(painted(&harness, "j/k column"), "{:?}", harness.painted);
+    assert!(!painted(&harness, "j/k row"));
+    assert!(!painted(&harness, "gl use as label"));
+    assert!(!painted(&harness, ":label choose"));
+}
+
+/// Test 6, in the Omarchy look: the status line says why a column that
+/// was named is not taken, until the next key.
+#[test]
+fn the_terminal_look_says_why_a_named_column_is_refused() {
+    let (mut harness, tab, _) = editions_structure_in(Look::omarchy());
+    for (line, said) in [
+        ("label cover", "cover cannot be the label (binary)"),
+        ("label id", "id cannot be the label (the key)"),
+        ("label nope", "no column nope in editions"),
+    ] {
+        run(&mut harness, tab, line);
+        assert!(harness.has(said), "{line}");
+    }
+    // `gl` on the key's line says the same.
+    harness.press(Key::ArrowUp, Modifiers::NONE);
+    assert!(!harness.has("no column nope in editions"));
+    type_key(&mut harness, Key::G, "g");
+    type_key(&mut harness, Key::L, "l");
+    assert!(harness.has("id cannot be the label (the key)"));
+    harness.press(Key::ArrowDown, Modifiers::NONE);
+    assert!(!harness.has("id cannot be the label (the key)"));
+}
```

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib label_tests`
Expected: two fail, on words the status line does not say yet: `the_terminal_looks_status_line_names_the_structure_views_keys` and `the_terminal_look_says_why_a_named_column_is_refused`.

- [ ] **Step 3: The status line**

```diff
diff --git a/src/ui/workspace.rs b/src/ui/workspace.rs
--- a/src/ui/workspace.rs
+++ b/src/ui/workspace.rs
@@ -1730,6 +1730,20 @@ fn editing_status(app: &App, tab: ConnTabId) -> Editing {
             tail: None,
             color: states::Tone::Danger.color(palette),
         })
+    } else if let Some((_, name, why)) = workspace
+        .label_refused
+        .as_ref()
+        .filter(|(of, _, _)| *of == object.id)
+    {
+        // A column that was named for this table's label and not taken,
+        // with why, until the next key.
+        let table = &object.object.name;
+        Some(Said {
+            mark: None,
+            text: super::structure::refused_text(name, *why, table, look, locale),
+            tail: None,
+            color: states::Tone::Danger.color(palette),
+        })
     } else if workspace.review_refused && !edits.pending() {
         // `:diff` found nothing to show. No mistake of the typing: it
         // reads as the keys do, until the next key. Never over a table
@@ -2006,6 +2020,22 @@ fn status_line(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
     let read_only = workspace.access == tabletist_db::Access::ReadOnly;
     let editor = super::sql_editor::status_summary(app, tab);
     let on_editor = editor.is_some();
+    // The Structure view of the table in front shows: the line names
+    // that view's keys in the rows' place. The keys that choose what
+    // labels the table's rows are named where there is anything to
+    // choose: not for a view, nor for a table of its key alone.
+    let structure = workspace
+        .active_object_tab()
+        .filter(|object| object.view == ObjectView::Structure);
+    let labels = structure
+        .and_then(|object| app.structure_label(tab, object.id))
+        .is_some_and(|label| match label.source {
+            crate::labels::Source::Automatic | crate::labels::Source::User => true,
+            crate::labels::Source::KeysOnly => false,
+        });
+    let (column_key, use_key) = (key(Command::MoveInStructure), key(Command::UseAsLabel));
+    let (chooser_key, automatic_key) =
+        (key(Command::OpenLabelChooser), key(Command::LabelAutomatic));
     // The completion list's own key, while it is open with a row to put
     // in. One whose row is about to be inserted is not open any more: the
     // mode line is drawn before the editor inserts it.
@@ -2115,8 +2145,33 @@ fn status_line(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
             if completing {
                 editor_hints.insert(0, (&complete_key, &*words[5], true));
             }
+            // The Structure view's keys: a row's do nothing on it.
+            let structure_words =
+                ["column", "use as label", "choose", "tables"].map(|word| gettext(locale, word));
+            let mut structure_hints: Vec<widgets::Hint<'_>> =
+                vec![(&column_key, &*structure_words[0], true)];
+            if labels {
+                // The line that hands the choice back says what it does
+                // by itself.
+                structure_hints.extend([
+                    (&use_key, &*structure_words[1], true),
+                    (&chooser_key, &*structure_words[2], true),
+                    (&automatic_key, "", true),
+                ]);
+            }
+            structure_hints.push((&sidebar_key, &*structure_words[3], true));
+            // What is pending in the table's rows is saved and reviewed
+            // from here too, as from the rows.
+            if !read_only {
+                structure_hints.push((&save_key, &*write, true));
+            }
+            if editing.pending.is_some() || editing.added.is_some() {
+                structure_hints.push((&review_key, &*review, true));
+            }
             let hints: &[widgets::Hint<'_>] = if on_editor {
                 &editor_hints
+            } else if structure.is_some() {
+                &structure_hints
             } else {
                 &table_hints
             };
```

- [ ] **Step 4: Run the tests, and look at it**

Run: `~/.cargo/bin/cargo fmt --all && ~/.cargo/bin/cargo test --locked -p tabletist --lib`
Expected: `test result: ok. 2242 passed; 0 failed; 20 ignored`.

Render the scenes of task 3, step 9 again and read the foot of the Omarchy ones. Expected:

- `structure-label-omarchy-dark.png`: `j/k column`, `gl use as label`, `:label choose`, `:label auto`, the key that shows the tables, `:w write`.
- `structure-refused-omarchy-dark.png`: after those, in the danger colour, `id cannot be the label (the key)`.

Take the test out again and delete its pictures.

- [ ] **Step 5: Run the four checks, and clippy with `--features shots`**

Expected: all pass.

- [ ] **Step 6: Commit**

```bash
git add src/ui/workspace.rs src/ui/label_tests.rs
git commit -S -m "Name the Structure view's label keys in the Omarchy status line

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

## The spec's tests, and where each is

| Spec test | Test |
|---|---|
| 5 | `the_structure_view_sets_the_label_on_a_read_only_connection` |
| 6 | `the_key_and_bytes_are_refused_with_their_reason`, `the_terminal_look_says_why_a_named_column_is_refused`; the rule in `labels::tests::a_column_named_by_hand_is_refused_with_its_reason` |
| 12 | `the_terminal_looks_keys_keep_what_the_buttons_keep` |
| 15 | `keymap::tests::the_structure_views_label_keys_are_the_briefs`, and the keymap's own collision test |
| Section 5, only a key, in Structure | `the_structure_view_of_a_table_of_its_key_alone_says_so` |
| Beyond the list | `the_structure_views_cursor_is_scrolled_into_view`, `the_structure_views_cursor_is_in_view_when_the_keys_come_back`, `each_structure_view_keeps_its_own_place`, `reload_in_the_structure_view_reloads_the_table`, `the_pane_keys_give_the_structure_view_the_keys`, `a_refusal_is_said_in_its_own_tab_alone`, `a_press_beside_the_structure_views_chooser_closes_it`, `the_structure_views_cursor_stays_inside_the_columns`, `the_structure_views_key_opens_the_chooser`, `the_structure_view_has_a_cursor_on_its_columns`, `the_terminal_looks_structure_view_says_the_label_in_its_words`, `the_terminal_looks_status_line_names_the_structure_views_keys`, `a_views_structure_has_no_label`, `the_label_lines_are_no_commands_over_the_rows`, `a_choice_made_in_the_structure_view_labels_the_pickers` |

## When the run is done

- The four checks pass on the last commit, and each of the four commits passed them.
- Report: what was looked at in tasks 3 and 4, and anything a diff needed that this plan did not say.
- Nothing was pressed in a real window. Worth a press by hand in a table's Structure view: a click on a column's line and the arrows; Tab and F6 to the columns; `⌘L` and `⌥⌘L`; a secondary click on a line; a table with more columns than the window shows (the cursor's line follows the keys); a narrow window (the card's sentence wraps beside its buttons); in the Omarchy look `ctrl+h` then `ctrl+l`, `j`, `k`, `gl`, `:label`, `:label auto`, `:label none`, `:label` with a column's name, and with the name of the key.
- Do not open the pull request: run 4 opens it, after the website's pages.
