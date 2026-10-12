# Foreign Key Label, Run 2: The Chooser from the Picker Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** From a foreign key's picker the user opens a chooser beside the list, picks what labels the referenced table's rows (Automatic, no label, or a column), and goes on editing: the choice is kept, the list is searched again for the text that shows, and the picker says what happened, in both looks.

**Architecture:** The chooser is state of the open picker (`key_picker::Picker::chooser`), as the picker is state of the open editor. `src/labels.rs` holds what it lists and where its keys are (`Line`, `lines`, `Chooser`), with no UI. `src/app/labels.rs` is its reducer: it opens it, moves in it, keeps the choice and labels afresh every open picker of the table. The chooser never takes the keyboard: the cell's field keeps it, as it does under the picker's list, and `src/ui/keys.rs` takes the chooser's keys before the field sees them, so the typed text and the cursor are never touched. `src/ui/label_chooser.rs` draws it with the app's menu row (`ui::menu::Item`), beside the list `src/ui/key_picker.rs` draws.

**Tech Stack:** Rust 1.98, egui (the crmne fork), AccessKit for the headless tests (`src/testing.rs`). Spec: `docs/superpowers/specs/2026-10-12-foreign-key-label-design.md` (the rest of section 2, sections 3, 5 and 6; tests 2, 4, 6 in the chooser, 7 to 11, 13 and 15). Run 1 (`2026-10-12-foreign-key-label-1-choice.md`) is built: this plan stands on `labels::{LabelChoice, Resolved, Source, Excluded, Labels, resolve}`, `App::label_of`, `App::check_label` and `ui::key_picker::label_line`.

---

## Read this first

### What was decided on 2026-10-12

The user said yes to the three things run 1's plan asked about:

1. The Omarchy picker stays the list under the cell's field. The chooser is a float beside that list.
2. A label is drawn plain, as the database writes the value as text, which is what the search matches. Not with the grid's digit grouping.
3. Two strings the notes board has none for: `Label keys only · Automatic` and `Label keys only · Set by you`.

### What was run where this plan was written

Everything in it. The whole run was built first, then replayed as the four tasks below, and each task's tree was checked alone: `cargo fmt --check`, `cargo clippy --workspace --all-targets -D warnings`, clippy with the `shots` feature (tasks 3 and 4), the app's tests and `tests/keys.rs`. The last tree passed the four checks whole: 2943 tests across the workspace, none failed. The code blocks and diffs below are those trees, not a retelling. Their line numbers are of the tree at `91fbde6`.

The plan's reviewer read the code against the spec and the boards. What it found is built in: the Omarchy picker's `ctrl+l label` key hint (the spec's section 2), the count of a list that holds only the first 50 matches, the chooser closing when its picker's search fails and not opening on one that failed, a press beside the chooser that is no click on the option it landed on, and a column that goes under an open picker confirming no choice.

Not run:

- **A real window.** Nothing was pressed by hand. The chooser, the "is gone" line, the flash and the Omarchy status line were rendered headless in the three looks, light and dark, and looked at (task 3, step 9 says how).
- **PostgreSQL and MySQL.** Nothing in this run talks to a database in a new way: the search is the one run 1 checked.

### How the chooser has the keys

The picker's list never takes the keyboard: the cell's field keeps it and `ui::keys` routes the list's keys. The chooser is built the same way, and that is what makes the brief's hardest promises true by construction:

- "The typed text and caret position are kept exactly": nothing ever takes the keyboard from the field, and while the chooser is open every key press and every letter of the frame is taken in `ui::keys` before the field is drawn.
- "Esc closes the chooser first, then the picker": the chooser's Esc is taken there too, so the field's own Esc, which closes the picker, sees only the second press.
- "The chooser has no text field": a letter that arrives while it is open is a key. Omarchy's `j` and `k` move; in the other looks a letter goes to the next column it begins.

In the keymap the chooser is the scope `prompt` with the condition `When::LabelChooser`, as Quick open and the question about leaving are: "nothing under it has the keys".

### Where the boards and the app differ

| The design asks | What is built | Why |
|---|---|---|
| While the chooser is open the field is drawn without its focus ring (macOS) and without its block cursor (Omarchy) | The field keeps its ring and its cursor | The field really has the keyboard, which is what keeps the text and the cursor. Drawing it as if it had lost the keyboard is paint that can be added later; nothing depends on it |
| Omarchy chooser lines: a `▌` column and a `✓` column, the type right after the name | The app's menu row, the one every menu of the Omarchy look uses: one mark column (`✓` on the choice in use, else `▌` on the line the keys are on), the type at the row's right | "Use only existing type styles, colours": the chooser is a menu, and is drawn as the app's menus are. The line the keys are on is filled as well, so it is never lost under the `✓` |
| The foot's button: its column in a 12 px mono | The mono role of the options' keys (`MonoSecondary`), a little larger than the words round it | There is no smaller mono role, and a view names no size |
| "The foot button flashes once" | The selection fill for one second | The boards give no time |
| Omarchy, column gone: the whole status line in amber, without the mode | `-- INSERT --` stays, the words after it are in the warning colour, for 4 s. The macOS line stays while that picker is open | The Components board: a status-line message is one line for 4 s, and the mode is the line's left end |
| Omarchy, columns loading: the status line says `esc back` alone | The same keys as when the columns are known | Automatic and No label are chosen with the same keys; the state lasts a moment |
| Screen reader: the foot button's description is "Command L changes it." | The description is the tooltip's words, `Change label column ⌘L`, with the key as the keymap writes it | A key is spelled only in the keymap (`tests/keys.rs`); a sentence with a key written into it would not follow a rebind |
| Announcement: "{n} matches" | "1 match" for one, "{n} matches" otherwise | The app counts with `ngettext` everywhere |
| The chooser opens beside the list | To the list's right; where the window has no room there, to its left; where neither side has it, on the side with more, narrowed down to 240 pt | A narrow window, and the Omarchy list grows to the width of its longest row |
| A menu | A press beside it closes it. One that landed on the list does nothing else: the option or the button under it is not clicked. (A press elsewhere in the window still does what it did before there was a chooser: it leaves the edit.) The pointer over a line puts the keys on that line | A press beside a menu closes the menu; the Components board: "Hover and keyboard highlight look the same" |
| Omarchy key hints: `ctrl+l` in the accent | `ctrl+l label` among the list's keys, in the colour of the other keys, and not where the table has nothing to choose | The line's keys are one colour; the board's accent points at what is new |

Not built in this run, and why:

- **A column of bytes listed in a chooser on screen.** The rule is tested (`labels::lines` lists it last, disabled, as `Excluded::Binary`), and the chooser draws it with the code that draws the key's line. But no Bookshop table that a key points at holds bytes, so no headless test opens a chooser with one. Run 3's Structure door opens the chooser on any table, and its fixture needs one with a column of bytes for this.

Two edges that are left as they are:

- **A column that is found gone is said by the first picker that learns it**, in whichever tab. If that tab is not the one shown, the Omarchy look's 4 s can pass unseen; the macOS look's line stays while that picker is open.
- **The field keeps its cursor and its ring while the chooser is open** (the first row of the table above).

### The Bookshop fixture

Run 2 adds two tables to `src/testing.rs`, the smallest that give a picker of a key-only table, as the brief allows. They go into the pull request's list:

- `isbns (isbn TEXT PRIMARY KEY)`: a table of its key alone.
- `editions (id, isbn, title)`, whose `isbn` is a foreign key to `isbns.isbn`, with two rows.

Everything else uses what run 1 used: `book_covers.publisher_id → publishers (id, name)`, and `legal_name` as the name of a column `publishers` lacks.

### For run 3

- `App::set_label(tab, table, choice)` keeps a choice from anywhere and labels the open pickers afresh. It does not check that a column can be chosen: the chooser offers only what can. `:label {column}` must check (`labels::excluded`, with the table's primary key) and say why it refuses.
- The chooser is drawn by `ui::label_chooser::show(ui, &Open, beside, field, skin)`. `field` is the field that has the keyboard, for the screen reader's active descendant: the Structure door has none, and needs that argument to be optional.
- The chooser's state lives in the picker. The Structure door needs a second home for the same `labels::Chooser` (on the table's tab), and `ui::keys` a second place that says a chooser is open.
- The fixture has no table with a column of bytes. The Structure door's tests need one, to see a line that says `binary`: the screenshot scenes' `editions` has `cover bytea`.

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

- `src/shots.rs` is outside those four and builds pickers. Tasks 3 and 4 also run `~/.cargo/bin/cargo clippy --locked --features shots --lib --tests -- -D warnings`.
- Run `~/.cargo/bin/cargo fmt --all` before each task's checks.
- **No pixels.** The tests check behaviour, names and words through the AccessKit tree and the commands sent. No snapshot is added and no picture is committed.
- House rules that bite here: views push `Action`s and change nothing; a key is spelled only in `src/keymap.rs`, and `tests/keys.rs` finds one spelled elsewhere (a test file that says what the user presses is listed there); only `src/env.rs` matches on an environment (`env::tests::environments_are_matched_only_in_this_module`); text is drawn through `TextRole`s; the focus ring is `src/ui/focus.rs`'s; strings go through `gettext`; no em dashes.
- Commits are signed, one per task, after its checks pass. If signing fails ("agent refused operation": 1Password is locked), do not commit unsigned: stage the task and tell the user. Each message ends with:

      Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>

- A diff below applies with `git apply`. Where the tree has moved, find the place by the names around it.

## File structure

| File | What it is for |
|---|---|
| `src/keymap.rs` | The chord that opens the chooser from the picker, the chooser's own keys, and `shift+enter` claimed for the variant |
| `src/labels.rs` | `Line`, `lines`: what the chooser lists. `Chooser`: where its keys are |
| `src/key_picker.rs` | The picker holds its chooser, and what it says of its label for a while (`changed`, `gone`, `nothing`). `relabel` keeps the highlight on its row |
| `src/model.rs` | The six actions; `Workspace::label_lines` |
| `src/app/labels.rs` | The reducer: open, move, choose, `set_label`, `relabel`, `tell_gone` |
| `src/app.rs`, `src/app/key_picker.rs` | The arms, and `tell_gone` where a picker learns its label |
| `src/ui/keys.rs` | `chooser_keys`, and the picker's chord |
| `src/ui/menu.rs` | A menu row gets a note after its words, and a form for a menu the keyboard never comes to |
| `src/ui/label_chooser.rs` (new) | The chooser's view |
| `src/ui/key_picker.rs` | The foot's button, the "is gone" line, the announcements, the chooser beside the list |
| `src/ui/cell_editor.rs`, `src/ui/data_view.rs`, `src/ui/row_form.rs` | What the chooser draws reaches both places the list is drawn |
| `src/env.rs` | `Environment::named`, for "Bookshop · dev" |
| `src/ui/workspace.rs` | The Omarchy status line: the chooser's key among the list's, the chooser's name and keys, and the three messages |
| `src/testing.rs`, `tests/keys.rs` | The two fixture tables; the test file may spell keys |
| `src/ui/label_tests.rs` | The headless tests |

---

### Task 1: The keys, in the keymap

**Files:**
- Modify: `src/keymap.rs`

Spec test 15. Nothing reads the commands yet: they are data, and the keymap's own tests hold them to its rules (no two commands share a key where both could be read, every command has a name of its own, Omarchy binds nothing over vim's insert keys).

- [ ] **Step 1: Write the failing test**

```diff
diff --git a/src/keymap.rs b/src/keymap.rs
--- a/src/keymap.rs
+++ b/src/keymap.rs
@@ -2748,6 +2748,69 @@ mod tests {
         // The Mac has no prompt.
         assert_eq!(keymap.ex(Layout::Mac, Scope::Grid, &nothing, "w"), Nothing);
     }
+    /// The keys of a foreign key's label, as its brief has them
+    /// (`docs/superpowers/specs/2026-10-12-foreign-key-label-design.md`,
+    /// test 15): the chord that opens the chooser from the picker, and the
+    /// chooser's own.
+    #[test]
+    fn the_label_choosers_keys_are_the_briefs() {
+        let keymap = Keymap::default();
+        let chords = |layout, command, scope| -> Vec<&str> {
+            keymap.chords(layout, command, scope).collect()
+        };
+        let (editor, prompt) = (Scope::CellEditor, Scope::Prompt);
+        use Command::{
+            ChangeLabelColumn, ChooseLabel, CloseLabelChooser, NextLabel, PreviousLabel,
+        };
+        for layout in [Layout::Mac, Layout::Windows] {
+            assert_eq!(chords(layout, ChangeLabelColumn, editor), ["cmd+l"]);
+            assert_eq!(chords(layout, NextLabel, prompt), ["down"]);
+            assert_eq!(chords(layout, PreviousLabel, prompt), ["up"]);
+        }
+        let omarchy = Layout::Omarchy;
+        assert_eq!(chords(omarchy, ChangeLabelColumn, editor), ["ctrl+l"]);
+        assert_eq!(chords(omarchy, NextLabel, prompt), ["j", "ctrl+n", "down"]);
+        assert_eq!(
+            chords(omarchy, PreviousLabel, prompt),
+            ["k", "ctrl+p", "up"]
+        );
+        for layout in Layout::ALL {
+            assert_eq!(chords(layout, ChooseLabel, prompt), ["enter"]);
+            assert_eq!(chords(layout, CloseLabelChooser, prompt), ["esc"]);
+            // Kept for a label of two columns: claimed, and no key yet.
+            assert!(chords(layout, Command::AddLabelPart, prompt).is_empty());
+        }
+        // The chord is insert mode's: with the keyboard out of a field
+        // Omarchy's `ctrl+l` is the pane on the right.
+        let now = |normal| -> Vec<&str> {
+            let command = ChangeLabelColumn;
+            keymap
+                .chords_now(omarchy, command, editor, normal)
+                .collect()
+        };
+        assert_eq!(now(false), ["ctrl+l"]);
+        assert!(now(true).is_empty());
+        // In the chooser `j` and `k` are keys. Under the picker alone
+        // they are letters of the search, and nothing here reads them.
+        let choosing = |when: When| when == When::LabelChooser;
+        let picking = |when: When| when == When::KeyPicker;
+        let command = Typed::Command;
+        assert_eq!(
+            keymap.typed(omarchy, prompt, &choosing, "j"),
+            command(NextLabel)
+        );
+        assert_eq!(
+            keymap.typed(omarchy, prompt, &choosing, "k"),
+            command(PreviousLabel)
+        );
+        assert_eq!(keymap.typed(omarchy, editor, &picking, "j"), Typed::Nothing);
+        assert_eq!(keymap.typed(omarchy, editor, &picking, "k"), Typed::Nothing);
+        assert_eq!(
+            keymap.typed(Layout::Mac, prompt, &choosing, "j"),
+            Typed::Nothing
+        );
+    }
+
     #[test]
     fn every_command_has_a_name_of_its_own() {
         let mut names: Vec<&str> = Vec::new();
```

- [ ] **Step 2: Run it to see it fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib keymap::`
Expected: it does not compile: `Command` has no `ChangeLabelColumn`, `NextLabel`, `PreviousLabel`, `ChooseLabel`, `CloseLabelChooser` or `AddLabelPart`, and `When` no `LabelChooser`.

- [ ] **Step 3: The commands and their keys**

```diff
diff --git a/src/keymap.rs b/src/keymap.rs
--- a/src/keymap.rs
+++ b/src/keymap.rs
@@ -146,6 +146,9 @@ pub enum When {
     NewRow,
     /// The list of a foreign key cell's picker is open under its field.
     KeyPicker,
+    /// The chooser of a table's label column is open: beside a picker's
+    /// list, which gives it the keys.
+    LabelChooser,
 }
 
 /// The mode a chord holds in: with the keyboard in a text field (insert),
@@ -362,6 +365,7 @@ commands! {
     PickPageDown,
     PickPageUp,
     ClosePicker,
+    ChangeLabelColumn,
     CancelEdit,
     OpenLargeEditor,
     ApplyLargeEditor,
@@ -454,6 +458,11 @@ commands! {
     QuickOpenMove,
     QuickOpenPick,
     QuickOpenClose,
+    NextLabel,
+    PreviousLabel,
+    ChooseLabel,
+    CloseLabelChooser,
+    AddLabelPart,
 }
 
 /// What is said of a command wherever it is listed.
@@ -574,6 +583,7 @@ impl Command {
             C::PickPageDown => info("A page down in a foreign key's list", "", Editing),
             C::PickPageUp => info("A page up in a foreign key's list", "", Editing),
             C::ClosePicker => info("Close a foreign key's list", "", Editing),
+            C::ChangeLabelColumn => info("Change label column", "", Editing),
             C::CancelEdit => info("Cancel cell edit", "Cancel cell edit", Editing),
             C::OpenLargeEditor => info("Open the large editor", "", Editing),
             C::ApplyLargeEditor => info("Apply large editor", LARGE, Editing),
@@ -687,6 +697,13 @@ impl Command {
             C::QuickOpenMove => info("Move in quick open", "", Prompts),
             C::QuickOpenPick => info("Open the one selected", "", Prompts),
             C::QuickOpenClose => info("Close quick open", "", Prompts),
+            C::NextLabel => info("Next line of the label chooser", "", Prompts),
+            C::PreviousLabel => info("Previous line of the label chooser", "", Prompts),
+            C::ChooseLabel => info("Choose the label column", "", Prompts),
+            C::CloseLabelChooser => info("Close the label chooser", "", Prompts),
+            C::AddLabelPart => {
+                info("Add a column as the label's second part", "", Prompts).reserved()
+            }
         }
     }
 }
@@ -1112,6 +1129,16 @@ pub const BINDINGS: &[Binding] = {
             keys(&[k("esc")]),
         )
         .when(When::KeyPicker),
+        // What labels the list's rows is chosen from the list: the chord
+        // is the field's while the list is open, and insert mode's, where
+        // Omarchy's `ctrl+l` is no move to the pane on the right.
+        bind(
+            C::ChangeLabelColumn,
+            CELL_EDITOR,
+            keys(&[k("cmd+l")]),
+            keys(&[k("ctrl+l")]),
+        )
+        .when(When::KeyPicker),
         bind(
             C::CancelEdit,
             CELL_EDITOR,
@@ -1536,6 +1563,44 @@ pub const BINDINGS: &[Binding] = {
             keys(&[k("esc")]),
         )
         .when(When::QuickOpen),
+        // The chooser of a table's label column. It has no field: a letter
+        // is a key of its own here. Shift with the key that chooses is
+        // kept for a label of two columns.
+        bind(
+            C::NextLabel,
+            PROMPT,
+            keys(&[k("down")]),
+            keys(&[k("j"), k("ctrl+n"), k("down")]),
+        )
+        .when(When::LabelChooser),
+        bind(
+            C::PreviousLabel,
+            PROMPT,
+            keys(&[k("up")]),
+            keys(&[k("k"), k("ctrl+p"), k("up")]),
+        )
+        .when(When::LabelChooser),
+        bind(
+            C::ChooseLabel,
+            PROMPT,
+            keys(&[k("enter")]),
+            keys(&[k("enter")]),
+        )
+        .when(When::LabelChooser),
+        bind(
+            C::CloseLabelChooser,
+            PROMPT,
+            keys(&[k("esc")]),
+            keys(&[k("esc")]),
+        )
+        .when(When::LabelChooser),
+        bind(
+            C::AddLabelPart,
+            PROMPT,
+            keys(&[claimed("shift+enter")]),
+            keys(&[claimed("shift+enter")]),
+        )
+        .when(When::LabelChooser),
     ]
 };
 
```

- [ ] **Step 4: Run the keymap's tests**

Run: `~/.cargo/bin/cargo fmt --all && ~/.cargo/bin/cargo test --locked -p tabletist --lib keymap::`
Expected: `test result: ok. 19 passed`. If `no_two_commands_share_a_key_where_both_could_be_read` fails, **stop and report it**: the brief says not to pick another key.

- [ ] **Step 5: Run the four checks**

Expected: all pass. The `?` list of every look now has the new commands: it reads the keymap.

- [ ] **Step 6: Commit**

```bash
git add src/keymap.rs
git commit -S -m "Give the chooser of a foreign key's label its keys

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: What the chooser lists, and where its keys are

**Files:**
- Modify: `src/labels.rs`

Pure, as the rest of the module. The lines are made from the table's columns each time they are read, so columns that arrive while the chooser is open show in it and its place holds: the first two lines never move.

- [ ] **Step 1: Write the failing tests**

```diff
diff --git a/src/labels.rs b/src/labels.rs
--- a/src/labels.rs
+++ b/src/labels.rs
@@ -426,6 +426,111 @@ mod tests {
         assert_eq!((resolved.column, resolved.source), (None, Source::User));
     }
 
+    /// The Bookshop's `editions` as the screenshot scenes have it, cut to
+    /// what matters: a key, two texts, bytes and a date.
+    fn editions() -> Vec<ColumnInfo> {
+        vec![
+            column("id", "bigint"),
+            column("title", "text"),
+            column("subtitle", "text"),
+            column("cover", "bytea"),
+            column("published_on", "date"),
+        ]
+    }
+
+    fn named(lines: &[Line]) -> Vec<(&str, Option<Excluded>)> {
+        lines
+            .iter()
+            .map(|line| match line {
+                Line::Automatic => ("Automatic", None),
+                Line::NoLabel => ("No label", None),
+                Line::Column { name, excluded, .. } => (name.as_str(), *excluded),
+            })
+            .collect()
+    }
+
+    #[test]
+    fn the_chooser_lists_what_can_be_chosen_then_what_cannot() {
+        let lines = lines(Dialect::Postgres, Some(&editions()), &["id"]);
+        assert_eq!(
+            named(&lines),
+            [
+                ("Automatic", None),
+                ("No label", None),
+                ("title", None),
+                ("subtitle", None),
+                ("published_on", None),
+                ("id", Some(Excluded::Key)),
+                ("cover", Some(Excluded::Binary)),
+            ]
+        );
+        // A column's type is the catalog's word for it.
+        assert!(matches!(&lines[4], Line::Column { type_name, .. } if type_name == "date"));
+        // What a line keeps, and that a key and bytes keep nothing.
+        assert_eq!(lines[0].choice(), Some(LabelChoice::Automatic));
+        assert_eq!(lines[1].choice(), Some(LabelChoice::NoLabel));
+        assert_eq!(lines[3].choice(), Some(chosen("subtitle")));
+        assert_eq!(lines[5].choice(), None);
+        assert_eq!(lines[6].choice(), None);
+    }
+
+    #[test]
+    fn a_chooser_without_columns_offers_automatic_and_no_label() {
+        let waiting = lines(Dialect::Postgres, None, &["id"]);
+        assert_eq!(named(&waiting), [("Automatic", None), ("No label", None)]);
+        let mut chooser = Chooser::open(&waiting, LabelChoice::Automatic, true);
+        assert!(chooser.waited);
+        chooser.step(&waiting, 1);
+        assert_eq!(chooser.chosen(&waiting), Some(LabelChoice::NoLabel));
+        // The columns came: the keys are where they were.
+        let known = lines(Dialect::Postgres, Some(&editions()), &["id"]);
+        assert_eq!(chooser.chosen(&known), Some(LabelChoice::NoLabel));
+    }
+
+    #[test]
+    fn a_chooser_opens_on_the_choice_and_steps_over_what_cannot_be_chosen() {
+        let lines = lines(Dialect::Postgres, Some(&editions()), &["id"]);
+        let mut chooser = Chooser::open(&lines, chosen("subtitle"), false);
+        assert_eq!(chooser.at, 3);
+        // Down to the last column that can be chosen, and no further.
+        chooser.step(&lines, 1);
+        assert_eq!(chooser.chosen(&lines), Some(chosen("published_on")));
+        chooser.step(&lines, 5);
+        assert_eq!(chooser.at, 4);
+        // Up to Automatic, and no further.
+        chooser.step(&lines, -9);
+        assert_eq!(chooser.chosen(&lines), Some(LabelChoice::Automatic));
+        // The pointer goes to a line that can be chosen, and to no other.
+        chooser.to(&lines, 2);
+        assert_eq!(chooser.at, 2);
+        chooser.to(&lines, 6);
+        chooser.to(&lines, 60);
+        assert_eq!(chooser.at, 2);
+        // A choice no line is (the column is bytes now) opens on Automatic.
+        assert_eq!(Chooser::open(&lines, chosen("cover"), false).at, 0);
+    }
+
+    #[test]
+    fn a_letter_jumps_to_the_next_column_it_begins() {
+        let lines = lines(Dialect::Postgres, Some(&editions()), &["id"]);
+        let mut chooser = Chooser::open(&lines, LabelChoice::Automatic, false);
+        chooser.jump(&lines, 's');
+        assert_eq!(chooser.chosen(&lines), Some(chosen("subtitle")));
+        // Whatever its case, and on to the next and round again.
+        chooser.jump(&lines, 'T');
+        assert_eq!(chooser.chosen(&lines), Some(chosen("title")));
+        chooser.jump(&lines, 't');
+        assert_eq!(chooser.chosen(&lines), Some(chosen("title")));
+        chooser.jump(&lines, 'p');
+        assert_eq!(chooser.chosen(&lines), Some(chosen("published_on")));
+        // Never to a column that cannot be chosen, nor to the first two
+        // lines: `a` is no Automatic, `c` no cover.
+        for letter in ['a', 'n', 'c', 'i', 'z'] {
+            chooser.jump(&lines, letter);
+            assert_eq!(chooser.at, 4, "{letter}");
+        }
+    }
+
     fn publishers() -> ObjectRef {
         ObjectRef::new("public", "publishers")
     }
```

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib labels::`
Expected: it does not compile: `Line`, `lines` and `Chooser` are not found.

- [ ] **Step 3: The lines and the chooser**

```diff
diff --git a/src/labels.rs b/src/labels.rs
--- a/src/labels.rs
+++ b/src/labels.rs
@@ -147,6 +147,148 @@ pub fn resolve(
     }
 }
 
+/// One line of the chooser of a table's label.
+#[derive(Debug, Clone, PartialEq, Eq)]
+pub enum Line {
+    /// The app picks.
+    Automatic,
+    /// The rows are listed by their keys alone.
+    NoLabel,
+    /// A column of the table, with its type as the catalog names it.
+    /// `excluded` says why it cannot be chosen, where it cannot: it is
+    /// listed all the same, so the chooser says what the table holds.
+    Column {
+        name: String,
+        type_name: String,
+        excluded: Option<Excluded>,
+    },
+}
+
+impl Line {
+    /// What choosing the line keeps. None for a column that cannot be a
+    /// label.
+    pub fn choice(&self) -> Option<LabelChoice> {
+        match self {
+            Self::Automatic => Some(LabelChoice::Automatic),
+            Self::NoLabel => Some(LabelChoice::NoLabel),
+            Self::Column {
+                name,
+                excluded: None,
+                ..
+            } => Some(LabelChoice::Column(name.clone())),
+            Self::Column {
+                excluded: Some(_), ..
+            } => None,
+        }
+    }
+}
+
+/// The chooser's lines for a table of `columns`: Automatic, No label, the
+/// columns that can be chosen in the table's order, then the ones that
+/// cannot. While the columns are not known (`None`) the first two are all
+/// there is, and both can be chosen. `keys` as for [`excluded`].
+pub fn lines(dialect: Dialect, columns: Option<&[ColumnInfo]>, keys: &[&str]) -> Vec<Line> {
+    let mut lines = vec![Line::Automatic, Line::NoLabel];
+    let columns = columns.unwrap_or_default();
+    let line = |column: &ColumnInfo| Line::Column {
+        name: column.name.clone(),
+        type_name: column.type_name.clone(),
+        excluded: excluded(dialect, column, keys),
+    };
+    let (allowed, refused): (Vec<Line>, Vec<Line>) = columns
+        .iter()
+        .map(line)
+        .partition(|line| line.choice().is_some());
+    lines.extend(allowed);
+    lines.extend(refused);
+    lines
+}
+
+/// The chooser of a table's label, while it is open: where its keys are.
+/// Its lines are made from the table's columns each time they are read
+/// ([`lines`]), so columns that arrive while it is open show in it.
+#[derive(Debug, Clone, PartialEq, Eq)]
+pub struct Chooser {
+    /// The line the keys are on, among [`lines`].
+    pub at: usize,
+    /// What was chosen when it opened: the line that is marked.
+    pub choice: LabelChoice,
+    /// It opened before the table's columns were known.
+    pub waited: bool,
+}
+
+impl Chooser {
+    /// A chooser that opens on the line of `choice`, or on Automatic where
+    /// no line is the choice's (its column cannot be chosen any more).
+    pub fn open(lines: &[Line], choice: LabelChoice, waited: bool) -> Self {
+        let chosen = |line: &Line| line.choice().as_ref() == Some(&choice);
+        Self {
+            at: lines.iter().position(chosen).unwrap_or(0),
+            choice,
+            waited,
+        }
+    }
+
+    /// Moves `by` lines that can be chosen, stopping at the last of them
+    /// either way: a line that cannot be chosen is stepped over.
+    pub fn step(&mut self, lines: &[Line], by: isize) {
+        let open = |at: &usize| lines.get(*at).is_some_and(|line| line.choice().is_some());
+        for _ in 0..by.unsigned_abs() {
+            let next = if by > 0 {
+                (self.at + 1..lines.len()).find(open)
+            } else {
+                (0..self.at).rev().find(open)
+            };
+            match next {
+                Some(at) => self.at = at,
+                None => break,
+            }
+        }
+    }
+
+    /// Goes to the line at `index`, if it can be chosen: the pointer came
+    /// over it.
+    pub fn to(&mut self, lines: &[Line], index: usize) {
+        if lines.get(index).is_some_and(|line| line.choice().is_some()) {
+            self.at = index;
+        }
+    }
+
+    /// Goes to the next column after the line the keys are on whose name
+    /// begins with `typed`, whatever its case, and round from the last to
+    /// the first: typing a letter again goes on to the next such column.
+    /// Only a column that can be chosen is gone to.
+    pub fn jump(&mut self, lines: &[Line], typed: char) {
+        let begins = |line: &Line| match line {
+            Line::Column {
+                name,
+                excluded: None,
+                ..
+            } => name
+                .chars()
+                .next()
+                .is_some_and(|first| first.to_lowercase().eq(typed.to_lowercase())),
+            Line::Column {
+                excluded: Some(_), ..
+            }
+            | Line::Automatic
+            | Line::NoLabel => false,
+        };
+        let count = lines.len();
+        let found = (1..=count)
+            .map(|step| (self.at + step) % count)
+            .find(|at| begins(&lines[*at]));
+        if let Some(at) = found {
+            self.at = at;
+        }
+    }
+
+    /// What Enter keeps: the choice of the line the keys are on.
+    pub fn chosen(&self, lines: &[Line]) -> Option<LabelChoice> {
+        lines.get(self.at).and_then(Line::choice)
+    }
+}
+
 /// The file's format.
 pub const VERSION: u32 = 1;
 
```

- [ ] **Step 4: Run the tests**

Run: `~/.cargo/bin/cargo fmt --all && ~/.cargo/bin/cargo test --locked -p tabletist --lib labels::`
Expected: `test result: ok. 16 passed`.

- [ ] **Step 5: Run the four checks**

Expected: all pass.

- [ ] **Step 6: Commit**

```bash
git add src/labels.rs
git commit -S -m "List what can label a table's rows, and keep the chooser's place among it

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: The chooser, from the picker

**Files:**
- Create: `src/ui/label_chooser.rs`
- Modify: `src/key_picker.rs`, `src/model.rs`, `src/app.rs`, `src/app/labels.rs`, `src/app/key_picker.rs`, `src/ui/keys.rs`, `src/ui/menu.rs`, `src/ui/key_picker.rs`, `src/ui/cell_editor.rs`, `src/ui/data_view.rs`, `src/ui/row_form.rs`, `src/ui/mod.rs`, `src/env.rs`
- Test: `src/ui/label_tests.rs`, `src/testing.rs`, `tests/keys.rs`

The whole of the chooser but the Omarchy look's status line, which is task 4. After this task the Omarchy look opens the chooser with its key and draws it; its status line still names the picker's keys while it is open.

Three things the brief does not spell out, built here: a search that fails closes the chooser, and none opens on one that failed (a failed search has no list, and its keys are the field's again); a press on the list beside the chooser closes it and is no click on the option it landed on, until the button comes up; and a column that goes under an open picker labels it afresh without confirming a choice, since nobody chose.

Three of run 1's tests change with it, each because what it checked changed: the foot's label is a button now (its words are painted, and its name for a screen reader is the notes board's), and a column that is gone is said by the first picker that opens, so the store forgets it at once.

- [ ] **Step 1: Write the failing tests, and the fixture they stand on**

The two tables of "The Bookshop fixture", above, and the test file among those that may spell a key:

```diff
diff --git a/src/testing.rs b/src/testing.rs
--- a/src/testing.rs
+++ b/src/testing.rs
@@ -844,6 +844,77 @@ pub fn genres_page() -> RowPage {
 /// The genres, as (id, name), for [`Harness::answer_lookups`].
 pub const GENRES: [(i64, &str); 2] = [(1, "Fiction"), (2, "Crime")];
 
+/// The Bookshop's `isbns`: a table of its key alone, with nothing to
+/// label a row by.
+pub fn isbns_structure() -> tabletist_db::Structure {
+    tabletist_db::Structure {
+        columns: vec![tabletist_db::ColumnInfo {
+            name: "isbn".into(),
+            type_name: "TEXT".into(),
+            ..tabletist_db::ColumnInfo::default()
+        }],
+        primary_key: vec!["isbn".into()],
+        ..tabletist_db::Structure::default()
+    }
+}
+
+/// The Bookshop's `editions`, cut to what a key to `isbns` needs: `isbn`
+/// is a foreign key to `isbns.isbn`.
+pub fn editions_structure() -> tabletist_db::Structure {
+    use tabletist_db::ColumnInfo;
+    let column = |name: &str, type_name: &str| ColumnInfo {
+        name: name.into(),
+        type_name: type_name.into(),
+        ..ColumnInfo::default()
+    };
+    tabletist_db::Structure {
+        columns: vec![
+            column("id", "INTEGER"),
+            column("isbn", "TEXT"),
+            column("title", "TEXT"),
+        ],
+        primary_key: vec!["id".into()],
+        foreign_keys: vec![tabletist_db::ForeignKeyInfo {
+            columns: vec!["isbn".into()],
+            ref_schema: "main".into(),
+            ref_table: "isbns".into(),
+            ref_columns: vec!["isbn".into()],
+            ..tabletist_db::ForeignKeyInfo::default()
+        }],
+        ..tabletist_db::Structure::default()
+    }
+}
+
+/// Two editions, each with its ISBN.
+pub fn editions_page() -> RowPage {
+    let meta = |name: &str, type_name: &str, kind: ValueKind| ColumnMeta {
+        name: name.into(),
+        type_name: type_name.into(),
+        kind,
+    };
+    let edition = |id: i64, isbn: &str, title: &str| {
+        vec![
+            Value::Int(id),
+            Value::Text(isbn.into()),
+            Value::Text(title.into()),
+        ]
+    };
+    RowPage {
+        columns: vec![
+            meta("id", "INTEGER", ValueKind::Numeric),
+            meta("isbn", "TEXT", ValueKind::Text),
+            meta("title", "TEXT", ValueKind::Text),
+        ],
+        rows: vec![
+            edition(1, "978-1-4028-9462-6", "The Lighthouse"),
+            edition(2, "978-1-4028-9463-3", "Low Tide"),
+        ],
+        has_more: false,
+        ordered_by_key: true,
+        elapsed: std::time::Duration::from_millis(2),
+    }
+}
+
 /// A page of `book_covers` whose three rows point at the three
 /// publishers in turn: Northlight Books, Harbor Press, Harbor Lights.
 /// [`book_covers_page`] numbers its publishers on from the first, and
@@ -1067,6 +1138,26 @@ impl Harness {
         (tab, id)
     }
 
+    /// `editions` open, writable, its structure and its two rows loaded:
+    /// a table whose `isbn` cells open a picker of `isbns`.
+    pub fn editions(&mut self) -> (ConnTabId, TabId) {
+        let tab = self.connect_fake_as(false);
+        self.app.apply(Action::OpenObject {
+            tab,
+            object: tabletist_db::ObjectRef::new("main", "editions"),
+            kind: ObjectKind::Table,
+            pin: true,
+        });
+        self.answer_structure(editions_structure());
+        self.answer_rows(editions_page());
+        let id = self
+            .app
+            .workspace(tab)
+            .and_then(|workspace| workspace.active_tab)
+            .expect("the table's tab is open");
+        (tab, id)
+    }
+
     /// The `Lookup` commands sent so far, with their requests.
     pub fn lookups(&self) -> Vec<(SessionId, RequestId, tabletist_db::Lookup)> {
         self.app
diff --git a/tests/keys.rs b/tests/keys.rs
--- a/tests/keys.rs
+++ b/tests/keys.rs
@@ -19,7 +19,7 @@ const MAY: [&str; 2] = ["src/keymap.rs", "src/typography/fonts.rs"];
 
 /// The files that are tests and nothing else, or scenes for screenshots: a
 /// test says what the user presses and sees, in its own words.
-const TESTS: [&str; 7] = [
+const TESTS: [&str; 8] = [
     "src/testing.rs",
     "src/shots.rs",
     "src/ui/complete_tests.rs",
@@ -27,6 +27,7 @@ const TESTS: [&str; 7] = [
     "src/ui/env_tests.rs",
     "src/ui/insert_audit_tests.rs",
     "src/ui/key_picker_tests.rs",
+    "src/ui/label_tests.rs",
 ];
 
 /// Every Rust file of the app, by its path from the repository root, with
```

The tests:

```diff
diff --git a/src/ui/label_tests.rs b/src/ui/label_tests.rs
--- a/src/ui/label_tests.rs
+++ b/src/ui/label_tests.rs
@@ -6,15 +6,16 @@
 //! Only behaviour, names and words are checked. No colour, size or
 //! picture of the design is.
 
+use egui::accesskit::Role;
 use egui::{Key, Modifiers};
-use tabletist_db::ObjectRef;
+use tabletist_db::{LookupPart, ObjectRef};
 
 use crate::backend::{Command, Event, RequestId, StateFile};
 use crate::connections::ConnectionId;
 use crate::key_picker::{Columns, Picker};
-use crate::labels::{LabelChoice, Resolved, Source};
+use crate::labels::{Chooser, LabelChoice, Resolved, Source};
 use crate::model::{Action, CellPos, ConnTabId, EditStart, Pane, TabId};
-use crate::testing::{GENRES, Harness, PUBLISHERS, publishers_structure};
+use crate::testing::{GENRES, Harness, PUBLISHERS, node, publishers_structure};
 use crate::theme::Look;
 
 /// `publisher_id` in `book_covers`, and `parent_id` in `genres`, by their
@@ -215,12 +216,13 @@ fn a_number_column_can_label_a_table() {
     assert!(searched(&harness).iter().all(|label| *label == parent));
 }
 
-/// Test 9, what is kept: a chosen column the table no longer has is found
-/// when its columns are read. The choice goes back to Automatic, the
-/// picker is labelled by the app's pick, and the column's name is kept to
-/// be said once. (Saying it is run 2's.)
+/// Test 9: a chosen column the table no longer has is found when its
+/// columns are read. The choice goes back to Automatic, the picker is
+/// labelled by the app's pick, and the first picker to open says so: in
+/// the list, and to a screen reader. It is said once: the next picker
+/// says nothing, and the store holds nothing of it.
 #[test]
-fn a_chosen_column_that_is_gone_goes_back_to_automatic() {
+fn a_chosen_column_that_is_gone_is_said_once() {
     let (mut harness, tab, id) = covers_in(Look::macos());
     // `publishers` has no such column: it was renamed since it was chosen.
     keep(&mut harness, tab, &publishers(), column("legal_name"));
@@ -232,9 +234,23 @@ fn a_chosen_column_that_is_gone_goes_back_to_automatic() {
     let conn = connection(&harness, tab);
     let labels = &harness.app.labels;
     assert_eq!(labels.choice(&conn, &publishers()), LabelChoice::Automatic);
-    assert_eq!(labels.gone(&conn, &publishers()), Some("legal_name"));
-    // Written once, with the choice dropped.
-    assert_eq!(saved(&harness), [harness.app.labels.clone()]);
+    // Told, so forgotten: the store is as if nothing had been chosen.
+    assert_eq!(labels.gone(&conn, &publishers()), None);
+    assert_eq!(*labels, crate::labels::Labels::default());
+    assert_eq!(saved(&harness).last(), Some(&harness.app.labels));
+    let said = "legal_name is gone, so labels use Automatic (name).";
+    let told = "legal_name is gone. Labels use Automatic, name.";
+    assert!(harness.has(said));
+    assert_eq!(live(&mut harness, told), Some(true));
+    // It stays for as long as this picker is open.
+    harness.frame(vec![egui::Event::Text("a".into())]);
+    assert!(harness.has(said));
+    // The next picker says nothing.
+    harness.app.apply(Action::CancelEdit { tab, id });
+    edit(&mut harness, tab, id, 1);
+    assert!(picker(&harness, tab, id).gone.is_none());
+    assert!(!harness.has(said));
+    assert_eq!(live(&mut harness, told), None);
 }
 
 /// A column is called gone only by columns that are fresh: while the
@@ -274,10 +290,10 @@ fn a_column_is_not_called_gone_while_the_columns_are_being_read_again() {
     let by_app = labelled(Some("name"), Source::Automatic, Some("name"));
     assert!(picker(&harness, tab, id).label == Columns::Known(by_app));
     assert!(harness.lookups().len() > sent);
-    assert_eq!(
-        harness.app.labels.gone(&conn, &publishers()),
-        Some("legal_name")
-    );
+    // The picker that waited is the one that says it.
+    let gone = picker(&harness, tab, id).gone.as_ref();
+    assert_eq!(gone.map(|(column, _)| column.as_str()), Some("legal_name"));
+    assert_eq!(harness.app.labels.gone(&conn, &publishers()), None);
 }
 
 /// Nor by a read that failed: the columns it leaves are the ones from
@@ -377,23 +393,36 @@ fn the_label_is_worded_by_its_column_and_who_chose_it() {
 }
 
 /// The macOS look says it in the list's foot, after the foot's own words,
-/// which stay as they were.
+/// which stay as they were: on a button, named for a screen reader as
+/// the notes board names it.
 #[test]
 fn the_foot_names_the_label_and_who_chose_it() {
     let (mut harness, tab, id) = covers_in(Look::macos());
     edit(&mut harness, tab, id, 1);
     assert!(harness.has("Esc cancels"));
-    assert!(harness.has("Label name · Automatic"));
-    for (choice, said) in [
-        (column("name"), "Label name · Set by you"),
-        (LabelChoice::NoLabel, "Label keys only · Set by you"),
+    let tree = harness.settle();
+    assert!(node(&tree, "Label column: name, Automatic", Role::Button).is_some());
+    assert!(harness.painted_rect("Label name · Automatic").is_some());
+    for (choice, said, name) in [
+        (
+            column("name"),
+            "Label name · Set by you",
+            "Label column: name, Set by you",
+        ),
+        (
+            LabelChoice::NoLabel,
+            "Label keys only · Set by you",
+            "Label column: keys only, Set by you",
+        ),
     ] {
         let (mut harness, tab, id) = covers_in(Look::macos());
         keep(&mut harness, tab, &publishers(), choice);
         edit(&mut harness, tab, id, 1);
         assert!(harness.has("Esc cancels"), "{said}");
-        assert!(harness.has(said), "{said}");
-        assert!(!harness.has("Label name · Automatic"), "{said}");
+        let tree = harness.settle();
+        assert!(node(&tree, name, Role::Button).is_some(), "{name}");
+        assert!(harness.painted_rect(said).is_some(), "{said}");
+        assert!(harness.painted_rect("Label name · Automatic").is_none());
     }
 }
 
@@ -410,3 +439,663 @@ fn the_terminal_looks_status_line_names_the_label() {
     edit(&mut harness, tab, id, 1);
     assert!(harness.has("label name · set by you"));
 }
+
+// The chooser: run 2.
+
+/// What a real Ctrl press reports: outside macOS Ctrl is the command key
+/// too.
+fn ctrl() -> Modifiers {
+    if cfg!(target_os = "macos") {
+        Modifiers::CTRL
+    } else {
+        Modifiers::CTRL | Modifiers::COMMAND
+    }
+}
+
+/// The chord that asks for the chooser, as `look` has it.
+fn ask(harness: &mut Harness, look: Look) {
+    let held = if look.terminal {
+        ctrl()
+    } else {
+        Modifiers::COMMAND
+    };
+    harness.press(Key::L, held);
+}
+
+/// Selects the foreign key cell of the page's row `row`.
+fn select(harness: &mut Harness, tab: ConnTabId, id: TabId, row: usize) {
+    let cell = CellPos { row, col: KEY };
+    harness.app.apply(Action::SelectCell { tab, id, cell });
+    harness.settle();
+}
+
+/// Types `text` on the selected cell of `book_covers`, as a keyboard
+/// does, and answers what the picker asks of `publishers`.
+fn type_on_cell(harness: &mut Harness, text: &str) {
+    harness.frame(vec![egui::Event::Text(text.into())]);
+    harness.settle();
+    harness.answer_structure(publishers_structure());
+    harness.settle();
+    harness.answer_lookups(&PUBLISHERS);
+    harness.settle();
+}
+
+/// The covers in `look`, `text` typed on the first row's `publisher_id`
+/// and its list answered.
+fn typed_in(look: Look, text: &str) -> (Harness, ConnTabId, TabId) {
+    let (mut harness, tab, id) = covers_in(look);
+    select(&mut harness, tab, id, 0);
+    if look.terminal {
+        // Insert mode, on nothing.
+        let cell = CellPos { row: 0, col: KEY };
+        let start = EditStart::Replace(String::new());
+        harness.app.apply(Action::EditCell {
+            tab,
+            id,
+            cell,
+            start,
+        });
+        harness.settle();
+    }
+    type_on_cell(&mut harness, text);
+    (harness, tab, id)
+}
+
+fn chooser(harness: &Harness, tab: ConnTabId, id: TabId) -> Option<&Chooser> {
+    picker(harness, tab, id).chooser.as_ref()
+}
+
+fn editing(harness: &Harness, tab: ConnTabId, id: TabId) -> bool {
+    let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
+    object.edits.editor.is_some()
+}
+
+/// The text the tab's editor holds.
+fn text(harness: &Harness, tab: ConnTabId, id: TabId) -> String {
+    let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
+    object.edits.editor.as_ref().unwrap().text.clone()
+}
+
+/// Where the cursor of the tab's field is, in characters.
+fn caret(harness: &Harness, tab: ConnTabId, id: TabId) -> Option<usize> {
+    let field = crate::ui::cell_editor::field_id(tab, id);
+    let state = egui::text_edit::TextEditState::load(&harness.ctx, field)?;
+    Some(state.cursor.char_range()?.primary.index.into())
+}
+
+fn has_keyboard(harness: &Harness, tab: ConnTabId, id: TabId) -> bool {
+    let field = crate::ui::cell_editor::field_id(tab, id);
+    harness.ctx.memory(|memory| memory.has_focus(field))
+}
+
+/// Whether the cell of the page's row `row` has anything pending.
+fn pending(harness: &Harness, tab: ConnTabId, id: TabId, row: usize) -> bool {
+    let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
+    object.edits.cells.contains_key(&(row, KEY))
+}
+
+/// Whether what is labelled `label` is in the tree, and a polite live
+/// region.
+fn live(harness: &mut Harness, label: &str) -> Option<bool> {
+    let tree = harness.settle();
+    let found = tree
+        .nodes
+        .iter()
+        .find(|(_, node)| node.label().or_else(|| node.value()) == Some(label))?;
+    Some(found.1.live() == Some(egui::accesskit::Live::Polite))
+}
+
+/// The key opens the chooser beside the list, on the line of what is
+/// chosen. It is a menu of radio items named as the notes board names
+/// them, and the field keeps the keyboard and its text.
+#[test]
+fn the_key_opens_the_chooser_and_the_field_keeps_the_keyboard() {
+    let (mut harness, tab, id) = typed_in(Look::macos(), "harb");
+    assert!(chooser(&harness, tab, id).is_none());
+    ask(&mut harness, Look::macos());
+    let open = chooser(&harness, tab, id).expect("the chooser is open");
+    assert_eq!((open.at, &open.choice), (0, &LabelChoice::Automatic));
+    assert!(!open.waited);
+    let tree = harness.settle();
+    assert!(node(&tree, "Label publishers by", Role::Menu).is_some());
+    for name in [
+        "Automatic, uses name",
+        "No label, keys only",
+        "name, TEXT",
+        // Listed, and not to be chosen.
+        "id, INTEGER, the key, unavailable",
+    ] {
+        assert!(node(&tree, name, Role::MenuItemRadio).is_some(), "{name}");
+    }
+    assert!(harness.has(
+        "For publishers in Fixture · dev, on this computer. Nothing is written to the database."
+    ));
+    assert!(has_keyboard(&harness, tab, id));
+    assert_eq!(text(&harness, tab, id), "harb");
+    // Nothing was chosen, and nothing asked of the database.
+    assert!(saved(&harness).is_empty());
+}
+
+/// Tests 2 and 4: choosing a column keeps the choice for the table on
+/// the saved connection, searches again for the text that was typed, and
+/// leaves the editor open with its text, its cursor and the cell's value
+/// as they were. The highlight stays on the row it was on. Nothing is
+/// sent to the database but that one search.
+#[test]
+fn choosing_a_column_keeps_the_edit_and_searches_once() {
+    let (mut harness, tab, id) = typed_in(Look::macos(), "harb");
+    // The cursor in the middle of the text, the highlight on the second
+    // of the two rows found.
+    harness.press(Key::ArrowLeft, Modifiers::NONE);
+    harness.press(Key::ArrowDown, Modifiers::NONE);
+    assert_eq!(caret(&harness, tab, id), Some(3));
+    assert_eq!(picker(&harness, tab, id).highlight, Some(1));
+    let before = harness.app.backend.sent.len();
+    ask(&mut harness, Look::macos());
+    // Automatic, No label, then `name`.
+    harness.press(Key::ArrowDown, Modifiers::NONE);
+    harness.press(Key::ArrowDown, Modifiers::NONE);
+    assert_eq!(chooser(&harness, tab, id).map(|open| open.at), Some(2));
+    harness.press(Key::Enter, Modifiers::NONE);
+    assert!(chooser(&harness, tab, id).is_none());
+    // The choice, kept and written.
+    let conn = connection(&harness, tab);
+    assert_eq!(
+        harness.app.labels.choice(&conn, &publishers()),
+        column("name")
+    );
+    assert_eq!(saved(&harness), [harness.app.labels.clone()]);
+    // The edit, as it was.
+    assert!(editing(&harness, tab, id));
+    assert!(has_keyboard(&harness, tab, id));
+    assert_eq!(text(&harness, tab, id), "harb");
+    assert_eq!(caret(&harness, tab, id), Some(3));
+    assert!(!pending(&harness, tab, id, 0));
+    // What was sent since the key: the file to be written, and one search
+    // for the same text by the chosen column. Nothing else.
+    let since = &harness.app.backend.sent[before..];
+    assert!(
+        matches!(
+            since,
+            [
+                Command::Save { file: StateFile::Labels(_), .. },
+                Command::Lookup { lookup, .. },
+            ] if lookup.label.as_deref() == Some("name")
+                && matches!(&lookup.part, LookupPart::Matching(text) if text == "harb")
+        ),
+        "{} commands",
+        since.len()
+    );
+    let by_user = labelled(Some("name"), Source::User, Some("name"));
+    assert!(picker(&harness, tab, id).label == Columns::Known(by_user));
+    // The answer: the highlight is on the row it was on.
+    harness.answer_lookups(&PUBLISHERS);
+    assert_eq!(picker(&harness, tab, id).highlight, Some(1));
+    let tree = harness.settle();
+    assert!(node(&tree, "Label column: name, Set by you", Role::Button).is_some());
+    // Enter still takes that row: the choice touched no value until now.
+    harness.press(Key::Enter, Modifiers::NONE);
+    assert!(!editing(&harness, tab, id));
+    assert!(pending(&harness, tab, id, 0));
+}
+
+/// Test 13: what a screen reader is told once the list is searched again,
+/// in the notes board's words, with the count of what was found.
+#[test]
+fn a_choice_is_announced_with_the_count_of_its_matches() {
+    // Three publishers' keys begin with 91.
+    let (mut harness, tab, id) = typed_in(Look::macos(), "91");
+    let choose = |harness: &mut Harness, line: usize| {
+        ask(harness, Look::macos());
+        let line = Some(line);
+        harness.app.apply(Action::ChooseLabel { tab, id, line });
+        harness.answer_lookups(&PUBLISHERS);
+    };
+    choose(&mut harness, 2);
+    assert_eq!(
+        live(&mut harness, "Labelled by name. 3 matches."),
+        Some(true)
+    );
+    choose(&mut harness, 1);
+    assert_eq!(live(&mut harness, "Keys only. 3 matches."), Some(true));
+    assert_eq!(live(&mut harness, "Labelled by name. 3 matches."), None);
+    choose(&mut harness, 0);
+    let said = "Labelled by name, Automatic. 3 matches.";
+    assert_eq!(live(&mut harness, said), Some(true));
+    // Said of the list as it was: the next text typed ends it.
+    harness.frame(vec![egui::Event::Text("0".into())]);
+    assert_eq!(live(&mut harness, said), None);
+}
+
+/// Nothing is said before the search answers: the count is the answer's.
+#[test]
+fn a_choice_is_not_announced_before_its_search_answers() {
+    let (mut harness, tab, id) = typed_in(Look::macos(), "91");
+    ask(&mut harness, Look::macos());
+    let line = Some(2);
+    harness.app.apply(Action::ChooseLabel { tab, id, line });
+    assert_eq!(live(&mut harness, "Labelled by name. 3 matches."), None);
+    harness.answer_lookups(&PUBLISHERS);
+    assert_eq!(
+        live(&mut harness, "Labelled by name. 3 matches."),
+        Some(true)
+    );
+}
+
+/// Test 7: Esc with the chooser open closes the chooser alone, and the
+/// text stays. The next Esc closes the picker.
+#[test]
+fn esc_closes_the_chooser_first_and_then_the_picker() {
+    for look in [Look::macos(), Look::omarchy()] {
+        let (mut harness, tab, id) = typed_in(look, "harb");
+        ask(&mut harness, look);
+        assert!(chooser(&harness, tab, id).is_some(), "{}", look.name);
+        harness.press(Key::Escape, Modifiers::NONE);
+        assert!(editing(&harness, tab, id), "{}", look.name);
+        assert!(chooser(&harness, tab, id).is_none(), "{}", look.name);
+        assert_eq!(text(&harness, tab, id), "harb", "{}", look.name);
+        assert!(has_keyboard(&harness, tab, id), "{}", look.name);
+        harness.press(Key::Escape, Modifiers::NONE);
+        assert!(!editing(&harness, tab, id), "{}", look.name);
+        assert!(!pending(&harness, tab, id, 0), "{}", look.name);
+        assert!(saved(&harness).is_empty(), "{}", look.name);
+    }
+}
+
+/// Test 8: under the picker every letter is the search's, `j` and `k`
+/// among them. In the chooser, which has no field, they move, and so do
+/// the terminal look's Ctrl+N and Ctrl+P. None of it reaches the text.
+#[test]
+fn letters_are_the_searchs_until_the_chooser_has_the_keys() {
+    let look = Look::omarchy();
+    let (mut harness, tab, id) = typed_in(look, "j");
+    harness.frame(vec![egui::Event::Text("k".into())]);
+    harness.settle();
+    assert_eq!(text(&harness, tab, id), "jk");
+    assert!(chooser(&harness, tab, id).is_none());
+    ask(&mut harness, look);
+    let at = |harness: &Harness| chooser(harness, tab, id).map(|open| open.at);
+    assert_eq!(at(&harness), Some(0));
+    for (typed, line) in [("j", 1), ("j", 2), ("j", 2), ("k", 1), ("x", 1), ("k", 0)] {
+        harness.frame(vec![egui::Event::Text(typed.into())]);
+        harness.settle();
+        assert_eq!(at(&harness), Some(line), "{typed}");
+    }
+    harness.press(Key::N, ctrl());
+    assert_eq!(at(&harness), Some(1));
+    harness.press(Key::P, ctrl());
+    assert_eq!(at(&harness), Some(0));
+    assert_eq!(text(&harness, tab, id), "jk");
+    // Enter chooses here, and commits nothing.
+    harness.press(Key::Enter, Modifiers::NONE);
+    assert!(chooser(&harness, tab, id).is_none());
+    assert!(editing(&harness, tab, id));
+    assert_eq!(text(&harness, tab, id), "jk");
+}
+
+/// In the looks whose keys are no letters a letter goes to the next
+/// column it begins, and the arrows step over what cannot be chosen.
+#[test]
+fn a_letter_jumps_to_a_column_and_the_arrows_skip_the_key() {
+    let (mut harness, tab, id) = typed_in(Look::macos(), "harb");
+    ask(&mut harness, Look::macos());
+    let at = |harness: &Harness| chooser(harness, tab, id).map(|open| open.at);
+    harness.frame(vec![egui::Event::Text("n".into())]);
+    harness.settle();
+    assert_eq!(at(&harness), Some(2));
+    // `id` is the key: no letter and no arrow goes to it.
+    harness.frame(vec![egui::Event::Text("i".into())]);
+    harness.settle();
+    assert_eq!(at(&harness), Some(2));
+    for _ in 0..3 {
+        harness.press(Key::ArrowDown, Modifiers::NONE);
+    }
+    assert_eq!(at(&harness), Some(2));
+    harness.press(Key::ArrowUp, Modifiers::NONE);
+    assert_eq!(at(&harness), Some(1));
+    assert_eq!(text(&harness, tab, id), "harb");
+    // The picker's own highlight did not move under it.
+    assert_eq!(picker(&harness, tab, id).highlight, Some(0));
+}
+
+/// Test 6, in the chooser: the key is listed and cannot be chosen, by a
+/// click or by asking for its line.
+#[test]
+fn the_key_cannot_be_chosen_in_the_chooser() {
+    let (mut harness, tab, id) = typed_in(Look::macos(), "harb");
+    ask(&mut harness, Look::macos());
+    harness.click("id, INTEGER, the key, unavailable");
+    let line = Some(3);
+    harness.app.apply(Action::ChooseLabel { tab, id, line });
+    assert!(chooser(&harness, tab, id).is_some());
+    assert!(saved(&harness).is_empty());
+    let conn = connection(&harness, tab);
+    assert_eq!(
+        harness.app.labels.choice(&conn, &publishers()),
+        LabelChoice::Automatic
+    );
+    // A line that can be chosen is chosen by a click.
+    harness.click("No label, keys only");
+    assert!(chooser(&harness, tab, id).is_none());
+    assert_eq!(
+        harness.app.labels.choice(&conn, &publishers()),
+        LabelChoice::NoLabel
+    );
+    assert!(editing(&harness, tab, id));
+    assert!(has_keyboard(&harness, tab, id));
+    assert_eq!(text(&harness, tab, id), "harb");
+}
+
+/// The foot's button opens the chooser and closes it, and says which.
+#[test]
+fn the_foots_button_opens_and_closes_the_chooser() {
+    let (mut harness, tab, id) = typed_in(Look::macos(), "harb");
+    let button = "Label column: name, Automatic";
+    let expanded = |harness: &mut Harness| {
+        let tree = harness.settle();
+        let id = node(&tree, button, Role::Button).expect("the button");
+        let found = tree.nodes.iter().find(|(node, _)| *node == id);
+        found.and_then(|(_, node)| node.is_expanded())
+    };
+    assert_eq!(expanded(&mut harness), Some(false));
+    harness.click(button);
+    assert!(chooser(&harness, tab, id).is_some());
+    assert_eq!(expanded(&mut harness), Some(true));
+    assert!(editing(&harness, tab, id));
+    assert!(has_keyboard(&harness, tab, id));
+    harness.click(button);
+    assert!(chooser(&harness, tab, id).is_none());
+    assert!(editing(&harness, tab, id));
+    assert_eq!(text(&harness, tab, id), "harb");
+}
+
+/// `editions` in `look`, the editor open on the first row's `isbn` and
+/// its list answered: a picker of `isbns`, a table of its key alone.
+fn isbn_picker(look: Look) -> (Harness, ConnTabId, TabId) {
+    let mut harness = Harness::new();
+    harness.set_look(look);
+    let (tab, id) = harness.editions();
+    harness.app.workspace_mut(tab).unwrap().pane = Pane::Grid;
+    harness.settle();
+    // `isbn` stands where `publisher_id` does: second in its page.
+    open(&mut harness, tab, id, 0);
+    harness.answer_structure(crate::testing::isbns_structure());
+    harness.answer_lookups(&[]);
+    (harness, tab, id)
+}
+
+/// Test 10: a table of its key alone has nothing to choose. Its options
+/// are keys, the button is disabled and says why, and the key opens no
+/// chooser: the picker notes that it was asked, to say why for a while.
+#[test]
+fn a_table_of_its_key_alone_opens_no_chooser() {
+    for look in [Look::macos(), Look::omarchy()] {
+        let (mut harness, tab, id) = isbn_picker(look);
+        let nothing = labelled(None, Source::KeysOnly, None);
+        assert!(picker(&harness, tab, id).label == Columns::Known(nothing));
+        assert!(searched(&harness).iter().all(Option::is_none));
+        assert!(picker(&harness, tab, id).nothing.is_none());
+        ask(&mut harness, look);
+        assert!(chooser(&harness, tab, id).is_none(), "{}", look.name);
+        assert!(picker(&harness, tab, id).nothing.is_some(), "{}", look.name);
+        assert!(editing(&harness, tab, id), "{}", look.name);
+        assert!(saved(&harness).is_empty(), "{}", look.name);
+    }
+    // The macOS look's button says it, to the eye and to a screen reader.
+    let (mut harness, _, _) = isbn_picker(Look::macos());
+    let name = "Label: keys only. isbns has only its key column.";
+    let tree = harness.settle();
+    assert!(node(&tree, name, Role::Button).is_some());
+    assert!(harness.painted_rect("Label keys only").is_some());
+    // Its key says why nothing opens, where the chooser would have.
+    let why = "isbns has only its key column";
+    assert!(harness.painted_rect(why).is_none());
+    ask(&mut harness, Look::macos());
+    harness.settle();
+    assert!(harness.painted_rect(why).is_some(), "{:?}", harness.painted);
+}
+
+/// Test 11: while the table's columns are on their way the chooser offers
+/// Automatic and No label, and a choice made then holds when they come.
+#[test]
+fn a_choice_can_be_made_before_the_columns_are_known() {
+    let (mut harness, tab, id) = covers_in(Look::macos());
+    // The editor is open, and nothing has answered for `publishers`.
+    open(&mut harness, tab, id, 0);
+    assert!(picker(&harness, tab, id).label == Columns::Waiting);
+    ask(&mut harness, Look::macos());
+    assert_eq!(
+        chooser(&harness, tab, id).map(|open| open.waited),
+        Some(true)
+    );
+    let tree = harness.settle();
+    assert!(node(&tree, "Automatic, checking…", Role::MenuItemRadio).is_some());
+    assert!(node(&tree, "No label, keys only", Role::MenuItemRadio).is_some());
+    assert!(harness.has("Loading columns…"));
+    assert!(harness.has("Automatic and No label work now"));
+    harness.press(Key::ArrowDown, Modifiers::NONE);
+    // Nothing to move to after No label yet.
+    harness.press(Key::ArrowDown, Modifiers::NONE);
+    harness.press(Key::Enter, Modifiers::NONE);
+    assert!(chooser(&harness, tab, id).is_none());
+    let conn = connection(&harness, tab);
+    assert_eq!(
+        harness.app.labels.choice(&conn, &publishers()),
+        LabelChoice::NoLabel
+    );
+    assert!(harness.lookups().is_empty());
+    // The columns come: the picker is labelled by the choice.
+    harness.answer_structure(publishers_structure());
+    let by_user = labelled(None, Source::User, Some("name"));
+    assert!(picker(&harness, tab, id).label == Columns::Known(by_user));
+    assert!(!searched(&harness).is_empty());
+    assert!(searched(&harness).iter().all(Option::is_none));
+}
+
+/// Columns that come while the chooser is open show in it, and a screen
+/// reader is told.
+#[test]
+fn columns_that_arrive_show_in_the_open_chooser() {
+    let (mut harness, tab, id) = covers_in(Look::macos());
+    open(&mut harness, tab, id, 0);
+    ask(&mut harness, Look::macos());
+    assert_eq!(live(&mut harness, "Columns loaded."), None);
+    harness.answer_structure(publishers_structure());
+    let tree = harness.settle();
+    assert!(node(&tree, "Automatic, uses name", Role::MenuItemRadio).is_some());
+    assert!(node(&tree, "name, TEXT", Role::MenuItemRadio).is_some());
+    assert!(!harness.has("Loading columns…"));
+    assert_eq!(live(&mut harness, "Columns loaded."), Some(true));
+    assert!(chooser(&harness, tab, id).is_some());
+}
+
+/// A choice searches its own table's pickers again, and no other's.
+#[test]
+fn a_choice_for_another_table_searches_nothing() {
+    let (mut harness, tab, id) = typed_in(Look::macos(), "harb");
+    let sent = harness.lookups().len();
+    assert!(harness.app.set_label(tab, &genres(), LabelChoice::NoLabel));
+    assert_eq!(harness.lookups().len(), sent);
+    assert!(picker(&harness, tab, id).changed.is_none());
+    // Its own table's is searched once, and a choice that changes
+    // nothing not at all.
+    assert!(
+        harness
+            .app
+            .set_label(tab, &publishers(), LabelChoice::NoLabel)
+    );
+    assert_eq!(harness.lookups().len(), sent + 1);
+    assert!(
+        !harness
+            .app
+            .set_label(tab, &publishers(), LabelChoice::NoLabel)
+    );
+    assert_eq!(harness.lookups().len(), sent + 1);
+}
+
+/// A picker that is open on a chosen column when the table's columns are
+/// read again without it is labelled by what is left, searched again,
+/// and is the one that says the column is gone.
+#[test]
+fn an_open_picker_is_labelled_again_when_its_column_goes() {
+    let (mut harness, tab, id) = covers_in(Look::macos());
+    keep(&mut harness, tab, &publishers(), column("name"));
+    edit(&mut harness, tab, id, 1);
+    let sent = harness.lookups().len();
+    // `name` was renamed to `title`, and the columns are read again.
+    let request = RequestId(u64::MAX);
+    let workspace = harness.app.workspace_mut(tab).unwrap();
+    let session = workspace.session;
+    let kept = workspace.columns.get_mut(&publishers()).unwrap();
+    kept.start(request);
+    let mut renamed = publishers_structure();
+    renamed.columns[1].name = "title".into();
+    harness.app.apply(Action::Backend(Event::Structure {
+        session,
+        request,
+        result: Ok(renamed),
+    }));
+    let by_app = labelled(Some("title"), Source::Automatic, Some("title"));
+    assert!(picker(&harness, tab, id).label == Columns::Known(by_app));
+    let again = harness.lookups().split_off(sent);
+    assert!(!again.is_empty());
+    assert!(
+        again
+            .iter()
+            .all(|(_, _, lookup)| lookup.label.as_deref() == Some("title"))
+    );
+    assert!(harness.has("name is gone, so labels use Automatic (title)."));
+    // Nobody chose: nothing confirms a choice.
+    assert!(picker(&harness, tab, id).changed.is_none());
+    harness.answer_lookups(&PUBLISHERS);
+    assert_eq!(
+        live(&mut harness, "Labelled by title, Automatic. 3 matches."),
+        None
+    );
+}
+
+/// A list that holds only the first of its matches says so where it
+/// confirms a choice, as it does everywhere else.
+#[test]
+fn a_choice_counts_a_list_that_goes_on_as_fifty_or_more() {
+    let (mut harness, tab, id) = typed_in(Look::macos(), "91");
+    ask(&mut harness, Look::macos());
+    let line = Some(2);
+    harness.app.apply(Action::ChooseLabel { tab, id, line });
+    harness.answer_lookups(&PUBLISHERS);
+    let object = harness.app.workspace_mut(tab).unwrap();
+    let object = object.object_tab_mut(id).unwrap();
+    let editor = object.edits.editor.as_mut().unwrap();
+    editor.picker.as_mut().unwrap().more = true;
+    let said = "Labelled by name. 50 or more matches.";
+    assert_eq!(live(&mut harness, said), Some(true));
+    assert_eq!(live(&mut harness, "Labelled by name. 3 matches."), None);
+}
+
+/// A search that fails has no list: the chooser beside it closes, and
+/// the keys are the field's again.
+#[test]
+fn a_search_that_fails_closes_the_chooser() {
+    let (mut harness, tab, id) = typed_in(Look::macos(), "harb");
+    harness.frame(vec![egui::Event::Text("o".into())]);
+    harness.app.send_due_lookups(std::time::Instant::now());
+    ask(&mut harness, Look::macos());
+    assert!(chooser(&harness, tab, id).is_some());
+    harness.fail_lookup(tabletist_db::Error::query("the table is locked"));
+    assert!(picker(&harness, tab, id).failed.is_some());
+    assert!(chooser(&harness, tab, id).is_none());
+    // Nor does one open there: its keys would be the field's.
+    harness.app.apply(Action::ToggleLabelChooser { tab, id });
+    assert!(chooser(&harness, tab, id).is_none());
+    // So with columns that fail to come.
+    let (mut harness, tab, id) = covers_in(Look::macos());
+    open(&mut harness, tab, id, 0);
+    ask(&mut harness, Look::macos());
+    assert!(chooser(&harness, tab, id).is_some());
+    let workspace = harness.app.workspace(tab).unwrap();
+    let session = workspace.session;
+    let request = workspace.columns[&publishers()].pending.unwrap();
+    harness.app.apply(Action::Backend(Event::Structure {
+        session,
+        request,
+        result: Err(tabletist_db::Error::query("no such table")),
+    }));
+    assert!(picker(&harness, tab, id).failed.is_some());
+    assert!(chooser(&harness, tab, id).is_none());
+}
+
+/// Clicks the node `label` of `role` with the pointer, as a mouse does:
+/// moved onto it, pressed and released in frames of their own.
+fn pointer_click(harness: &mut Harness, label: &str, role: Role) {
+    let tree = harness.settle();
+    let at = crate::testing::bounds(&tree, label, role)
+        .unwrap_or_else(|| panic!("nothing labelled {label:?}"))
+        .center();
+    let button = |pressed| egui::Event::PointerButton {
+        pos: at,
+        button: egui::PointerButton::Primary,
+        pressed,
+        modifiers: Modifiers::NONE,
+    };
+    harness.frame(vec![egui::Event::PointerMoved(at)]);
+    harness.frame(vec![button(true)]);
+    harness.frame(vec![button(false)]);
+    harness.settle();
+}
+
+/// A press beside the chooser closes it and does nothing else, as one
+/// beside a menu does: the option it landed on is not taken, and the
+/// edit goes on. The next click on that option takes it.
+#[test]
+fn a_press_beside_the_chooser_closes_it_and_picks_nothing() {
+    let (mut harness, tab, id) = typed_in(Look::macos(), "harb");
+    ask(&mut harness, Look::macos());
+    let option = "Harbor Lights, 9100000000000000007";
+    pointer_click(&mut harness, option, Role::ListBoxOption);
+    assert!(chooser(&harness, tab, id).is_none());
+    assert!(editing(&harness, tab, id));
+    assert!(has_keyboard(&harness, tab, id));
+    assert!(!pending(&harness, tab, id, 0));
+    assert_eq!(text(&harness, tab, id), "harb");
+    pointer_click(&mut harness, option, Role::ListBoxOption);
+    assert!(!editing(&harness, tab, id));
+    assert!(pending(&harness, tab, id, 0));
+}
+
+/// A press that closes the chooser and the picker with it (it landed on
+/// the grid) leaves nothing behind: the first click on an option of the
+/// next picker takes it.
+#[test]
+fn a_press_that_closed_the_picker_too_swallows_no_later_click() {
+    let (mut harness, tab, id) = typed_in(Look::macos(), "harb");
+    ask(&mut harness, Look::macos());
+    // The press, beside the chooser, with the button left down; then the
+    // editor is gone before the button comes up.
+    let tree = harness.settle();
+    let option = "Harbor Lights, 9100000000000000007";
+    let at = crate::testing::bounds(&tree, option, Role::ListBoxOption).unwrap();
+    let at = at.center();
+    harness.frame(vec![egui::Event::PointerMoved(at)]);
+    harness.frame(vec![egui::Event::PointerButton {
+        pos: at,
+        button: egui::PointerButton::Primary,
+        pressed: true,
+        modifiers: Modifiers::NONE,
+    }]);
+    assert!(chooser(&harness, tab, id).is_none());
+    harness.app.apply(Action::CancelEdit { tab, id });
+    harness.frame(vec![egui::Event::PointerButton {
+        pos: at,
+        button: egui::PointerButton::Primary,
+        pressed: false,
+        modifiers: Modifiers::NONE,
+    }]);
+    harness.settle();
+    assert!(!editing(&harness, tab, id));
+    // The next picker: one click takes its option.
+    select(&mut harness, tab, id, 0);
+    type_on_cell(&mut harness, "harb");
+    pointer_click(&mut harness, option, Role::ListBoxOption);
+    assert!(!editing(&harness, tab, id));
+    assert!(pending(&harness, tab, id, 0));
+}
```

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib label_tests`
Expected: it does not compile: `labels::Chooser` is not a field of the picker, `Action::ChooseLabel` and `App::set_label` are not there.

- [ ] **Step 3: The picker holds its chooser, and what it says of its label**

`relabel` notes the key of the highlighted option; the answer of the search that follows puts the highlight back on that row. `typed` ends what was said of the list before it.

```diff
diff --git a/src/key_picker.rs b/src/key_picker.rs
--- a/src/key_picker.rs
+++ b/src/key_picker.rs
@@ -12,7 +12,7 @@ use tabletist_db::{
 
 use crate::backend::RequestId;
 use crate::edit::{DoubtState, Edits, State, Table, new_id};
-use crate::labels::Resolved;
+use crate::labels::{Chooser, Resolved};
 use crate::model::Advance;
 
 /// How many matches a list holds; more are said, not paged.
@@ -21,6 +21,9 @@ pub const LIMIT: u32 = 50;
 pub const DELAY: Duration = Duration::from_millis(150);
 /// How many options show at once; the list follows the highlight.
 pub const VISIBLE: usize = 8;
+/// How long a picker goes on saying what just happened to its label: that
+/// it changed, or that there is nothing to choose.
+pub const SAID: Duration = Duration::from_secs(4);
 
 /// The table and the column that the one-column foreign key of `column`
 /// points at. `None` for a column no such key names, and for a key whose
@@ -333,6 +336,25 @@ pub struct Picker {
     /// list: it was just read from the table, and is not asked about
     /// again.
     pub taken: bool,
+    /// The chooser of what labels the list, while it is open beside it.
+    /// It has the keys then: the field keeps the keyboard, and nothing
+    /// typed reaches its text.
+    pub chooser: Option<Chooser>,
+    /// The key of the option that was highlighted when the label changed,
+    /// and whether the user had moved to it: the answer of the search
+    /// that follows puts the highlight back on that row, if it holds it.
+    held: Option<(Value, bool)>,
+    /// When the label last changed under this picker. What says so shows
+    /// for [`SAID`], and goes with the next text typed.
+    pub changed: Option<Instant>,
+    /// The column that was chosen for the table and found gone, where
+    /// this is the first picker to open since, and when it was noted: the
+    /// list says so while it is open, and the terminal look's status line
+    /// for [`SAID`].
+    pub gone: Option<(String, Instant)>,
+    /// When the chooser was asked for and the table has nothing to
+    /// choose: said for [`SAID`].
+    pub nothing: Option<Instant>,
 }
 
 impl Picker {
@@ -365,9 +387,43 @@ impl Picker {
             failed: None,
             refused: false,
             taken: false,
+            chooser: None,
+            held: None,
+            changed: None,
+            gone: None,
+            nothing: None,
         }
     }
 
+    /// The list is labelled by `label` from now on: the user chose, or
+    /// the column they had chosen is gone. `chosen` is when the user
+    /// chose, for what confirms a choice; none where nobody did. The
+    /// search for the text that shows is to be sent again (the reducer
+    /// sends it); until its answer the options stay as they are. A
+    /// failure was the old search's.
+    pub fn relabel(&mut self, label: Columns, chosen: Option<Instant>) {
+        self.held = self
+            .highlight
+            .and_then(|at| self.options.get(at))
+            .map(|option| (option.key.clone(), self.moved));
+        self.label = label;
+        self.changed = chosen;
+        self.nothing = None;
+        self.kept = None;
+        self.resends = 0;
+        if self.failed.take().is_some() {
+            self.listed = None;
+        }
+    }
+
+    /// Whether the list is the answer for the text that was last searched
+    /// for, with nothing of that search on its way.
+    pub fn answered_now(&self) -> bool {
+        self.asked.as_ref().is_some_and(|asked| {
+            asked.matching.is_none() && self.listed.as_deref() == Some(asked.text.as_str())
+        })
+    }
+
     /// What is searched for while the editor holds `typed`.
     pub fn text<'a>(&self, typed: &'a str) -> &'a str {
         if self.searching { typed.trim() } else { "" }
@@ -394,6 +450,10 @@ impl Picker {
         self.refused = false;
         self.resends = 0;
         self.taken = false;
+        // What was said of the label was said of the list before this.
+        self.held = None;
+        self.changed = None;
+        self.nothing = None;
     }
 
     /// The editor's text was replaced by an option's key. It is the text
@@ -515,9 +575,20 @@ impl Picker {
                 self.options.clear();
                 self.more = false;
                 self.failed = Some(error);
+                // A search that failed has no list, and its keys are the
+                // field's again: no chooser stands beside nothing.
+                self.chooser = None;
             }
         }
         self.highlight = self.first_highlight();
+        // The label changed under the list: the highlight goes back to
+        // the row it was on, where the answer still holds that row.
+        if let Some((key, moved)) = self.held.take()
+            && let Some(at) = self.options.iter().position(|option| option.key == key)
+        {
+            self.highlight = Some(at);
+            self.moved = moved;
+        }
         Answered::Taken
     }
 
```

- [ ] **Step 4: The actions, and the reducer**

```diff
diff --git a/src/app.rs b/src/app.rs
--- a/src/app.rs
+++ b/src/app.rs
@@ -1034,6 +1034,25 @@ impl App {
                 }
             }
             Action::PickerNull { tab, id } => self.picker_null(tab, id),
+            Action::ToggleLabelChooser { tab, id } => self.toggle_label_chooser(tab, id),
+            Action::MoveLabelChooser { tab, id, by } => {
+                self.in_label_chooser(tab, id, |chooser, lines| chooser.step(lines, by));
+            }
+            Action::PointLabelChooser { tab, id, line } => {
+                self.in_label_chooser(tab, id, |chooser, lines| chooser.to(lines, line));
+            }
+            Action::JumpLabelChooser { tab, id, to } => {
+                self.in_label_chooser(tab, id, |chooser, lines| chooser.jump(lines, to));
+            }
+            Action::ChooseLabel { tab, id, line } => self.choose_label(tab, id, line),
+            Action::CloseLabelChooser { tab, id } => {
+                if let Some(picker) = self
+                    .editor_mut(tab, id)
+                    .and_then(|editor| editor.picker.as_mut())
+                {
+                    picker.chooser = None;
+                }
+            }
             Action::EditorBreak { tab, id } => {
                 if let Some(editor) = self.editor_mut(tab, id) {
                     // At the end of the text, where the cursor of a field
diff --git a/src/app/key_picker.rs b/src/app/key_picker.rs
--- a/src/app/key_picker.rs
+++ b/src/app/key_picker.rs
@@ -115,6 +115,7 @@ impl App {
         if load {
             self.load_columns(tab, table);
         }
+        self.tell_gone(tab, id);
         self.search(tab, id);
     }
 
@@ -230,6 +231,9 @@ impl App {
                 }
             } else if let Some(error) = &kept.error {
                 picker.failed = Some(error.clone());
+                // No list, and no chooser beside it: the keys are the
+                // field's.
+                picker.chooser = None;
                 // Nothing will answer: a kept Enter takes the text. Under
                 // a prompt about pending changes the commit is dropped
                 // (`App::apply`) and the key is lost: accepted, the next
@@ -244,6 +248,7 @@ impl App {
         }
         workspace.columns = columns;
         for id in ready {
+            self.tell_gone(tab, id);
             self.search(tab, id);
         }
         for (id, then) in commits {
diff --git a/src/model.rs b/src/model.rs
--- a/src/model.rs
+++ b/src/model.rs
@@ -323,6 +323,46 @@ pub enum Action {
         tab: ConnTabId,
         id: TabId,
     },
+    /// Open the chooser of what labels the options of the open editor's
+    /// foreign key picker, or close it where it is open: the picker's key
+    /// for it, and a click on its foot's button. Where the table has
+    /// nothing to choose the picker says so, and nothing opens.
+    ToggleLabelChooser {
+        tab: ConnTabId,
+        id: TabId,
+    },
+    /// Move that chooser's keys `by` lines: the arrows, the terminal
+    /// look's `j` and `k`.
+    MoveLabelChooser {
+        tab: ConnTabId,
+        id: TabId,
+        by: isize,
+    },
+    /// Put its keys on `line`: the pointer came over it.
+    PointLabelChooser {
+        tab: ConnTabId,
+        id: TabId,
+        line: usize,
+    },
+    /// A letter typed in it: go to the next column the letter begins.
+    JumpLabelChooser {
+        tab: ConnTabId,
+        id: TabId,
+        to: char,
+    },
+    /// Choose `line` of it, or the line its keys are on: a click, Enter.
+    /// The choice is kept, the chooser closes and the list is searched
+    /// again for the text that shows.
+    ChooseLabel {
+        tab: ConnTabId,
+        id: TabId,
+        line: Option<usize>,
+    },
+    /// Close it and choose nothing: Esc, a press beside it.
+    CloseLabelChooser {
+        tab: ConnTabId,
+        id: TabId,
+    },
     /// Add a line break and move the text into the large editor.
     EditorBreak {
         tab: ConnTabId,
@@ -2946,6 +2986,15 @@ impl Workspace {
         self.tabs.iter_mut().find(|tab| tab.id() == id)
     }
 
+    /// The lines of the chooser of what labels the rows of `table`, for
+    /// a foreign key that references its column `key`: by its columns as
+    /// the workspace knows them now.
+    pub fn label_lines(&self, table: &ObjectRef, key: &str) -> Vec<crate::labels::Line> {
+        let known = self.columns.get(table);
+        let columns = known.and_then(|kept| kept.value.as_deref());
+        crate::labels::lines(self.driver.dialect(), columns, &[key])
+    }
+
     pub fn object_tab(&self, id: TabId) -> Option<&ObjectTab> {
         self.tab(id)?.as_object()
     }
```

```diff
diff --git a/src/app/labels.rs b/src/app/labels.rs
--- a/src/app/labels.rs
+++ b/src/app/labels.rs
@@ -5,10 +5,13 @@
 
 use tabletist_db::{ColumnInfo, Dialect, ObjectRef};
 
+use std::time::Instant;
+
 use super::App;
 use crate::connections::ConnectionId;
-use crate::labels::{self, LabelChoice, Labels, Resolved};
-use crate::model::{ConnTabId, Fetch};
+use crate::key_picker::Columns;
+use crate::labels::{self, Chooser, LabelChoice, Labels, Line, Resolved, Source};
+use crate::model::{ConnTabId, Fetch, TabId};
 
 /// What labels the rows of `table`, whose columns the workspace of the
 /// saved connection `conn` keeps as `kept`, in a picker whose foreign key
@@ -64,5 +67,198 @@ impl App {
         }
         self.labels.lost(&conn, table, &name);
         self.save_labels();
+        // A picker that is open on the column searches by what is left,
+        // and says the column is gone: nobody chose, so nothing confirms
+        // a choice. One that waits for these columns is labelled for the
+        // first time by their answer.
+        self.relabel(&conn, table, false);
+    }
+
+    /// Opens the chooser of what labels the options of the picker of the
+    /// tab's editor, on the line of the choice kept for its table; closes
+    /// it where it is open. Where the table has nothing to choose no
+    /// chooser opens, and the picker says so for a while.
+    pub(super) fn toggle_label_chooser(&mut self, tab: ConnTabId, id: TabId) {
+        let planned = self.workspace(tab).and_then(|workspace| {
+            let editor = workspace.object_tab(id)?.edits.editor.as_ref()?;
+            let picker = editor.picker.as_ref()?;
+            if picker.chooser.is_some() {
+                return Some(None);
+            }
+            // A search that failed has no list, and its keys are the
+            // field's: a chooser there would get none of them.
+            if picker.failed.is_some() {
+                return None;
+            }
+            let lines = workspace.label_lines(&picker.target, &picker.key);
+            let choice = self.labels.choice(&workspace.conn_id, &picker.target);
+            // The columns are still on their way: Automatic and No label
+            // can be chosen at once, and the rest shows when it comes.
+            let waited = picker.label == Columns::Waiting;
+            let nothing = matches!(
+                &picker.label,
+                Columns::Known(label) if label.source == Source::KeysOnly
+            );
+            Some(Some((Chooser::open(&lines, choice, waited), nothing)))
+        });
+        let picker = self
+            .editor_mut(tab, id)
+            .and_then(|editor| editor.picker.as_mut());
+        let (Some(planned), Some(picker)) = (planned, picker) else {
+            return;
+        };
+        match planned {
+            None => picker.chooser = None,
+            Some((_, true)) => picker.nothing = Some(Instant::now()),
+            Some((chooser, false)) => {
+                picker.chooser = Some(chooser);
+                picker.nothing = None;
+            }
+        }
+    }
+
+    /// Lets `act` move the keys of the chooser that is open beside the
+    /// picker of the tab's editor, among its lines as they are now.
+    pub(super) fn in_label_chooser(
+        &mut self,
+        tab: ConnTabId,
+        id: TabId,
+        act: impl FnOnce(&mut Chooser, &[Line]),
+    ) {
+        let lines = self.workspace(tab).and_then(|workspace| {
+            let editor = workspace.object_tab(id)?.edits.editor.as_ref()?;
+            let picker = editor.picker.as_ref()?;
+            picker.chooser.as_ref()?;
+            Some(workspace.label_lines(&picker.target, &picker.key))
+        });
+        let chooser = self
+            .editor_mut(tab, id)
+            .and_then(|editor| editor.picker.as_mut())
+            .and_then(|picker| picker.chooser.as_mut());
+        if let (Some(lines), Some(chooser)) = (lines, chooser) {
+            act(chooser, &lines);
+        }
+    }
+
+    /// Chooses `line` of the chooser that is open beside the picker of
+    /// the tab's editor, or the line its keys are on. A line that cannot
+    /// be chosen (the key, a column of bytes) chooses nothing, and the
+    /// chooser stays. The editor stays open with its text as it is: no
+    /// value of the cell is touched.
+    pub(super) fn choose_label(&mut self, tab: ConnTabId, id: TabId, line: Option<usize>) {
+        let planned = self.workspace(tab).and_then(|workspace| {
+            let editor = workspace.object_tab(id)?.edits.editor.as_ref()?;
+            let picker = editor.picker.as_ref()?;
+            let chooser = picker.chooser.as_ref()?;
+            let lines = workspace.label_lines(&picker.target, &picker.key);
+            let choice = lines.get(line.unwrap_or(chooser.at))?.choice()?;
+            Some((picker.target.clone(), choice))
+        });
+        let Some((table, choice)) = planned else {
+            return;
+        };
+        if let Some(picker) = self
+            .editor_mut(tab, id)
+            .and_then(|editor| editor.picker.as_mut())
+        {
+            picker.chooser = None;
+        }
+        self.set_label(tab, &table, choice);
+    }
+
+    /// Keeps `choice` as what labels the rows of `table` on the saved
+    /// connection of `tab`, on this computer: nothing is sent to the
+    /// database but the searches of the pickers that are open on the
+    /// table, which are labelled afresh. `choice` is one a chooser's line
+    /// offered. Returns whether it changed anything.
+    pub fn set_label(&mut self, tab: ConnTabId, table: &ObjectRef, choice: LabelChoice) -> bool {
+        let Some(workspace) = self.workspace(tab) else {
+            return false;
+        };
+        let conn = workspace.conn_id.clone();
+        if !self.labels.choose(&conn, table, choice) {
+            return false;
+        }
+        self.save_labels();
+        self.relabel(&conn, table, true);
+        true
+    }
+
+    /// Labels afresh every open picker of `table` on the saved connection
+    /// `conn`, in whichever of its tabs, and sends each one's search again
+    /// for the text it holds: once, and no other table's. `chosen` says
+    /// the user just chose: each picker then confirms it for a while, and
+    /// one that still waits for the table's columns is among them, to
+    /// confirm when its list comes.
+    pub(super) fn relabel(&mut self, conn: &ConnectionId, table: &ObjectRef, chosen: bool) {
+        let now = Instant::now();
+        let labels = self.labels.clone();
+        let tabs: Vec<ConnTabId> = self
+            .open_connections()
+            .filter(|(_, workspace)| workspace.conn_id == *conn)
+            .map(|(tab, _)| tab)
+            .collect();
+        let mut again = Vec::new();
+        for tab in tabs {
+            let Some(workspace) = self.workspace_mut(tab) else {
+                continue;
+            };
+            let dialect = workspace.driver.dialect();
+            let columns = std::mem::take(&mut workspace.columns);
+            for object in workspace.object_tabs_mut() {
+                let picker = object
+                    .edits
+                    .editor
+                    .as_mut()
+                    .and_then(|editor| editor.picker.as_mut())
+                    .filter(|picker| picker.target == *table)
+                    .filter(|picker| chosen || picker.label != Columns::Waiting);
+                let Some(picker) = picker else {
+                    continue;
+                };
+                let label = columns
+                    .get(table)
+                    .and_then(|kept| resolved(&labels, (conn, dialect), kept, table, &picker.key))
+                    .map_or(Columns::Waiting, Columns::Known);
+                picker.relabel(label, chosen.then_some(now));
+                again.push((tab, object.id));
+            }
+            workspace.columns = columns;
+        }
+        for (tab, id) in again {
+            self.tell_gone(tab, id);
+            // Nothing is sent for a picker that waits for its columns:
+            // their answer sends its search.
+            self.search(tab, id);
+        }
+    }
+
+    /// The picker of the tab's editor knows what labels it. If the column
+    /// chosen for its table was found gone and nobody was told, this
+    /// picker says so for as long as it is open, and the store forgets
+    /// it: it is said once.
+    pub(super) fn tell_gone(&mut self, tab: ConnTabId, id: TabId) {
+        let found = self.workspace(tab).and_then(|workspace| {
+            let editor = workspace.object_tab(id)?.edits.editor.as_ref()?;
+            let picker = editor.picker.as_ref()?;
+            if picker.label == Columns::Waiting {
+                return None;
+            }
+            let conn = &workspace.conn_id;
+            let gone = self.labels.gone(conn, &picker.target)?.to_owned();
+            Some((conn.clone(), picker.target.clone(), gone))
+        });
+        let Some((conn, table, gone)) = found else {
+            return;
+        };
+        if let Some(picker) = self
+            .editor_mut(tab, id)
+            .and_then(|editor| editor.picker.as_mut())
+        {
+            picker.gone = Some((gone, Instant::now()));
+        }
+        if self.labels.told(&conn, &table) {
+            self.save_labels();
+        }
     }
 }
```

`relabel` is called with `chosen: false` where a column was found gone: nobody chose, so nothing confirms a choice, and a picker that still waits for those columns is left to be labelled for the first time by their answer. `set_label` passes `true`: a choice the user made is confirmed by every picker of the table, one that waits among them, when its list comes. No chooser opens beside a picker whose search failed: it would get no keys.

- [ ] **Step 5: The keys**

`chooser_keys` runs before the block that reads every other key, and leaves it nothing: no key press, no text, no paste. The picker's chord is read with the picker's other keys, in insert mode.

```diff
diff --git a/src/ui/keys.rs b/src/ui/keys.rs
--- a/src/ui/keys.rs
+++ b/src/ui/keys.rs
@@ -548,6 +548,20 @@ pub fn handle(app: &mut App, ctx: &egui::Context) {
             .any(|event| matches!(event, egui::Event::Key { pressed: true, .. }))
     });
     let mut actions = Vec::new();
+    // The chooser of what labels that picker's list is open beside it.
+    // It has no field, and every key and letter of the frame is its own:
+    // they are taken here, before the shortcuts below and before the
+    // picker's field, whose text and cursor stay as they are.
+    let choosing = picking.filter(|&(tab, id)| {
+        app.workspace(tab)
+            .and_then(|workspace| workspace.object_tab(id))
+            .and_then(|object| object.edits.editor.as_ref())
+            .and_then(|editor| editor.picker.as_ref())
+            .is_some_and(|picker| picker.chooser.is_some())
+    });
+    if let Some(chooser) = choosing {
+        chooser_keys(ctx, chooser, (&app.keymap, layout), &mut actions);
+    }
     // The place of the tab asked for with its digit.
     let mut tab_asked = None;
     ctx.input_mut(|input| {
@@ -566,6 +580,12 @@ pub fn handle(app: &mut App, ctx: &egui::Context) {
         // The foreign key picker's keys, by the same rule.
         if let Some((tab, id)) = picking {
             picker_keys(input, (tab, id), (keymap, layout), &mut actions);
+            // The chooser of what labels the list. A fresh press only:
+            // held, the chord would open it and close it by turns.
+            let scope = Scope::CellEditor;
+            if choosing.is_none() && on(input, Command::ChangeLabelColumn, scope).fresh {
+                actions.push(Action::ToggleLabelChooser { tab, id });
+            }
         }
         if let Some((tab, sql_tab)) = sql {
             // Running works while typing: the editor never sees these. A
@@ -1026,6 +1046,79 @@ fn picker_keys(
     }
 }
 
+/// The keys of the chooser of a label column, open beside the list of the
+/// picker of the table `(tab, id)`, as the keymap has them for a prompt:
+/// the keys that move in it, the one that chooses and the one that closes.
+/// The chooser has no field. A letter is the key the keymap makes it
+/// (Omarchy's `j` and `k`), and in the looks whose keys are no letters it
+/// goes to the next column it begins. Every key and letter of the frame is
+/// taken: none reaches the picker's field, a shortcut or a pane under it.
+fn chooser_keys(
+    ctx: &egui::Context,
+    (tab, id): (ConnTabId, TabId),
+    (keymap, layout): (&Keymap, Layout),
+    actions: &mut Vec<Action>,
+) {
+    use crate::keymap::When;
+    let scope = Scope::Prompt;
+    // What was typed with nothing held, read before the keys are taken.
+    let typed = typed_chars(ctx);
+    ctx.input_mut(|input| {
+        for (command, by) in [(Command::NextLabel, 1), (Command::PreviousLabel, -1)] {
+            for keys in keymap.chords(layout, command, scope) {
+                // One line for each press: a frame may hold several.
+                let presses = presses(input, layout, keys).count;
+                actions.extend((0..presses).map(|_| Action::MoveLabelChooser { tab, id, by }));
+            }
+        }
+        // A fresh press only: a held Enter or Esc acts once. Every
+        // chord of the command is taken, whichever went down.
+        let mut fresh = |command: Command| {
+            let chords: Vec<&'static str> = keymap.chords(layout, command, scope).collect();
+            let mut any = false;
+            for keys in chords {
+                any |= presses(input, layout, keys).fresh;
+            }
+            any
+        };
+        let line = None;
+        if fresh(Command::ChooseLabel) {
+            actions.push(Action::ChooseLabel { tab, id, line });
+        } else if fresh(Command::CloseLabelChooser) {
+            actions.push(Action::CloseLabelChooser { tab, id });
+        }
+        for to in typed {
+            let letter = to.to_string();
+            match (
+                prompt_letter(keymap, layout, When::LabelChooser, &letter),
+                layout,
+            ) {
+                (Some(Command::NextLabel), _) => {
+                    actions.push(Action::MoveLabelChooser { tab, id, by: 1 });
+                }
+                (Some(Command::PreviousLabel), _) => {
+                    actions.push(Action::MoveLabelChooser { tab, id, by: -1 });
+                }
+                (_, Layout::Mac | Layout::Windows) => {
+                    actions.push(Action::JumpLabelChooser { tab, id, to });
+                }
+                (_, Layout::Omarchy) => {}
+            }
+        }
+        input.events.retain(|event| {
+            !matches!(
+                event,
+                egui::Event::Key { pressed: true, .. }
+                    | egui::Event::Text(_)
+                    | egui::Event::Paste(_)
+                    | egui::Event::Copy
+                    | egui::Event::Cut
+                    | egui::Event::Ime(_)
+            )
+        });
+    });
+}
+
 /// The keys of the completion list of `editor`, which has the keyboard:
 /// one asks for a list, and an open one (`completing`) takes the keys that
 /// move in it, insert from it and close it. Which keys is the keymap's to
```

- [ ] **Step 6: The menu row, and the environment's name**

A row of a menu the keyboard never comes to is clicked and never focused, and lit by whoever draws the menu. Only `src/env.rs` matches on an environment, so the words "Bookshop · dev" get theirs there.

```diff
diff --git a/src/env.rs b/src/env.rs
--- a/src/env.rs
+++ b/src/env.rs
@@ -68,6 +68,19 @@ impl Environment {
         }
     }
 
+    /// What the environment is called in a sentence that names a
+    /// connection (`Bookshop · dev`): its label in lower case, in every
+    /// look. None for a connection that is none of them, which is named
+    /// by its name alone.
+    pub fn named(self, platform: Platform) -> Option<String> {
+        match self {
+            Self::None => None,
+            Self::Local | Self::Dev | Self::Staging | Self::Production => {
+                Some(self.label(platform).to_lowercase())
+            }
+        }
+    }
+
     /// The badge text: the lower-case name in a macOS or Windows pill, the
     /// terminal's upper-case tag on Omarchy.
     pub fn label(self, platform: Platform) -> &'static str {
diff --git a/src/ui/menu.rs b/src/ui/menu.rs
--- a/src/ui/menu.rs
+++ b/src/ui/menu.rs
@@ -106,6 +106,8 @@ pub struct Item<'a> {
     role: Option<TextRole>,
     detail: Option<&'a str>,
     disabled: Option<&'a str>,
+    note: Option<&'a str>,
+    lit: Option<bool>,
 }
 
 impl<'a> Item<'a> {
@@ -118,6 +120,8 @@ impl<'a> Item<'a> {
             role: None,
             detail: None,
             disabled: None,
+            note: None,
+            lit: None,
         }
     }
 
@@ -147,6 +151,23 @@ impl<'a> Item<'a> {
         self
     }
 
+    /// Muted words that go on after the row's own, on its line: what a
+    /// choice comes to (`Automatic · uses name`).
+    pub fn note(mut self, note: &'a str) -> Self {
+        self.note = Some(note);
+        self
+    }
+
+    /// A row of a menu the keyboard never comes to: a field beside it
+    /// keeps the keyboard, and `lit` says whether the menu's keys are on
+    /// this row. It is clicked and never focused, and the pointer lights
+    /// no row by itself: whoever draws the menu moves its keys to the row
+    /// the pointer is over.
+    pub fn kept_out(mut self, lit: bool) -> Self {
+        self.lit = Some(lit);
+        self
+    }
+
     /// Shown, and not to be picked: `reason` says why, to a screen reader,
     /// under the pointer and while the keyboard is on the row. A row that
     /// cannot be picked stays in its menu, so the menu says what there is
@@ -180,15 +201,21 @@ impl<'a> Item<'a> {
         let noted = self
             .detail
             .map_or(0.0, |detail| DETAIL_GAP + measure(note, detail));
-        let wanted = lead + measure(role, self.text) + noted + shape.inset;
+        let own = measure(role, self.text);
+        let after = self
+            .note
+            .map_or(0.0, |words| shape.gap + measure(note, words));
+        let wanted = lead + own + after + noted + shape.inset;
         // A row that cannot be picked answers no press, and still takes
         // the keyboard, as a button that cannot be pressed does: the
-        // keyboard can then read why.
+        // keyboard can then read why. In a menu the keyboard never comes
+        // to, no row takes it.
         let enabled = self.disabled.is_none();
-        let sense = if enabled {
-            Sense::click()
-        } else {
-            Sense::focusable_noninteractive()
+        let sense = match (self.lit, enabled) {
+            (Some(_), true) => Sense::CLICK,
+            (Some(_), false) => Sense::hover(),
+            (None, true) => Sense::click(),
+            (None, false) => Sense::focusable_noninteractive(),
         };
         let (rect, response) = ui.allocate_at_least(vec2(wanted, shape.height), sense);
         let name = self.name.unwrap_or(self.text);
@@ -211,7 +238,10 @@ impl<'a> Item<'a> {
         // The pointer and the keyboard light a row the same way. One that
         // cannot be picked is lit by the keyboard alone, which has to show
         // where it is; its words stay muted.
-        let lit = focus::shown(&response) || (enabled && response.hovered());
+        let lit = match self.lit {
+            Some(lit) => lit,
+            None => focus::shown(&response) || (enabled && response.hovered()),
+        };
         focus::hint(ui, &response, rect, Ring::Own);
         // A long menu scrolls: the row the keyboard came to is brought
         // into view. Jump, not animate: the next key may come at once.
@@ -256,6 +286,12 @@ impl<'a> Item<'a> {
         Text::one(look, role, self.text, ink)
             .layout(ui.ctx())
             .paint_left(painter, rect.left() + lead, center);
+        if let Some(words) = self.note {
+            let left = rect.left() + lead + own + shape.gap;
+            Text::one(look, note, words, palette.dim)
+                .layout(ui.ctx())
+                .paint_left(painter, left, center);
+        }
         if let Some(detail) = self.detail {
             Text::one(look, note, detail, palette.dim)
                 .layout(ui.ctx())
```

- [ ] **Step 7: The chooser's view**

Create `src/ui/label_chooser.rs`:

```rust
//! The chooser of what labels a table's rows for the foreign keys that
//! point at it: a menu beside a picker's list. Like the list it never
//! takes the keyboard: the picker's field keeps it, and `ui::keys` routes
//! the chooser's keys. What it lists and where its keys are is in
//! `crate::labels`.

use egui::accesskit::Role;
use egui::{Id, Rect, Sense, Ui, UiBuilder, WidgetInfo, WidgetType, pos2, vec2};

use crate::env::{Environment, Platform};
use crate::i18n::{Locale, gettext};
use crate::keymap::{Command, Scope};
use crate::labels::{Excluded, LabelChoice, Line};
use crate::theme::{Look, Palette};
use crate::typography::{Text, TextRole};
use crate::ui::format::display_safe;
use crate::ui::{menu, states, widgets};

/// What an open chooser draws.
pub struct Open {
    /// The line its keys are on.
    pub at: usize,
    pub lines: Vec<Line>,
    /// What is chosen now: the line that is marked.
    pub choice: LabelChoice,
    /// The column the app picks for the table. None where it finds none
    /// to read a row by, and while the columns are not known.
    pub automatic: Option<String>,
    /// The table's columns are not known yet.
    pub loading: bool,
    /// They came while the chooser was open.
    pub arrived: bool,
    /// The table whose label is chosen.
    pub table: String,
    /// The saved connection the choice is kept for: its name, and what it
    /// is.
    pub connection: (String, Environment),
}

/// What was asked of the chooser this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Asked {
    /// The pointer came over this line.
    Point(usize),
    /// This line was clicked.
    Choose(usize),
}

/// What the pointer did to the chooser this frame.
pub struct Shown {
    /// It is pressed or was clicked: the press landed outside the field,
    /// which must keep the keyboard.
    pub pressed: bool,
    pub asked: Option<Asked>,
    /// Where it stands.
    pub rect: Rect,
}

/// The chooser's width, as each look's board has it.
fn width(look: &Look) -> f32 {
    if look.terminal { 380.0 } else { 286.0 }
}

/// The narrowest the chooser is drawn, where the window has less room
/// beside the list than it wants.
const NARROW: f32 = 240.0;

/// The room between the list and the chooser beside it.
fn gap(look: &Look) -> f32 {
    if look.terminal { 8.0 } else { 6.0 }
}

/// What the chooser is called: "Label publishers by". A menu's name for a
/// screen reader, and what the Omarchy look's status line says while it
/// is open.
pub fn title(table: &str, look: &Look, locale: Locale) -> String {
    look.label(&gettext(locale, "Label {table} by"))
        .replace("{table}", &display_safe(table))
}

/// The saved connection as the chooser's foot names it: its name, and
/// what it is where it is anything, in lower case (`Bookshop · dev`).
fn connection_name((name, environment): &(String, Environment), look: &Look) -> String {
    let name = display_safe(name);
    match environment.named(Platform::of(look)) {
        Some(what) => format!("{name} · {what}"),
        None => name.into_owned(),
    }
}

/// Whom the choice is for, and that it stays on this computer.
fn foot_text(open: &Open, look: &Look, locale: Locale) -> String {
    let words = if look.terminal {
        "For {table} in {connection}, on this computer · nothing is written to the database"
    } else {
        "For {table} in {connection}, on this computer. Nothing is written to the database."
    };
    // The app's own words are the look's; the names are as they are.
    look.label(&gettext(locale, words))
        .replace("{table}", &display_safe(&open.table))
        .replace("{connection}", &connection_name(&open.connection, look))
}

/// The keys that move in the chooser, choose and close, as the keymap has
/// them for the look: `↑↓ · ↩ · esc`, `j/k · enter · esc`.
fn keys_line(ui: &Ui, look: &Look) -> String {
    let key = |command| crate::ui::keys::written_in(ui.ctx(), look, command, Scope::Prompt);
    let (next, previous) = (key(Command::NextLabel), key(Command::PreviousLabel));
    let one = |keys: &str| keys.chars().count() == 1;
    let letters = |keys: &str| keys.chars().all(char::is_alphanumeric);
    let moves = if one(&next) && one(&previous) && !letters(&next) {
        // Two arrows stand together, up first.
        format!("{previous}{next}")
    } else if one(&next) && one(&previous) {
        // Two letters, in the order a hand finds them.
        format!("{next}/{previous}")
    } else {
        format!("{previous}/{next}")
    };
    let (choose, close) = (key(Command::ChooseLabel), key(Command::CloseLabelChooser));
    format!("{moves} · {choose} · {close}")
}

/// Why a column cannot be chosen, in the look's words.
fn reason(excluded: Excluded, look: &Look, locale: Locale) -> String {
    let words = match excluded {
        Excluded::Key => "the key",
        Excluded::Binary => "binary",
    };
    look.label(&gettext(locale, words))
}

/// One line's words.
struct Row {
    /// What it is: `Automatic`, a column's name.
    text: String,
    /// Muted words after it: what Automatic and No label come to.
    note: Option<String>,
    /// At the row's right: a column's type, and why it cannot be chosen.
    detail: Option<String>,
    /// What a screen reader calls it.
    name: String,
    /// Why it cannot be chosen, where it cannot.
    refused: Option<String>,
}

fn row(line: &Line, open: &Open, look: &Look, locale: Locale) -> Row {
    let say = |words: &'static str| look.label(&gettext(locale, words));
    // The other looks set what a choice comes to off with a dot.
    let lead = if look.terminal { "" } else { "· " };
    match line {
        Line::Automatic => {
            let comes_to = match (&open.automatic, open.loading) {
                (_, true) => say("checking…"),
                (Some(column), false) => {
                    say("uses {column}").replace("{column}", &display_safe(column))
                }
                (None, false) => say("keys only"),
            };
            let text = say("Automatic");
            Row {
                name: format!("{text}, {comes_to}"),
                note: Some(format!("{lead}{comes_to}")),
                text,
                detail: None,
                refused: None,
            }
        }
        Line::NoLabel => {
            let (text, comes_to) = (say("No label"), say("keys only"));
            Row {
                name: format!("{text}, {comes_to}"),
                note: Some(format!("{lead}{comes_to}")),
                text,
                detail: None,
                refused: None,
            }
        }
        Line::Column {
            name,
            type_name,
            excluded,
        } => {
            let (text, kind) = (display_safe(name), display_safe(type_name));
            match excluded {
                None => Row {
                    name: format!("{text}, {kind}"),
                    detail: Some(kind.into_owned()),
                    text: text.into_owned(),
                    note: None,
                    refused: None,
                },
                Some(excluded) => {
                    let why = reason(*excluded, look, locale);
                    // The Omarchy look says first that it is not on offer.
                    let said = if look.terminal {
                        format!("{}, {why}", say("n/a"))
                    } else {
                        why.clone()
                    };
                    Row {
                        name: format!("{text}, {kind}, {why}, {}", say("unavailable")),
                        detail: Some(format!("{kind} · {said}")),
                        text: text.into_owned(),
                        note: None,
                        refused: Some(why),
                    }
                }
            }
        }
    }
}

/// One line of small words across the chooser, cut to `ui`'s width.
fn words(ui: &mut Ui, text: &str, color: egui::Color32, look: &Look) {
    let role = TextRole::pick(look, TextRole::Shortcut, TextRole::OCaption);
    Text::one(look, role, text, color)
        .wrap(ui.available_width())
        .layout(ui.ctx())
        .label(ui);
}

/// Draws the chooser beside the list at `beside`, on the side that has
/// the room. `field` is the picker's field, which has the keyboard: its
/// node says which line the keys are on.
pub fn show(
    ui: &Ui,
    open: &Open,
    beside: Rect,
    field: Id,
    (look, palette, locale): (&Look, &Palette, Locale),
) -> Shown {
    let frame = menu::frame(look, palette);
    let (wanted, gap) = (width(look), gap(look));
    let screen = ui.ctx().content_rect();
    // To the list's right where the window has the room, else to its
    // left. Where neither side has it, on the side with more, as wide as
    // that side lets it be and no narrower than a menu reads at: then it
    // stands over the list's edge.
    let right = screen.right() - beside.right() - gap;
    let before = beside.left() - gap - screen.left();
    let (left, width) = if right >= wanted {
        (beside.right() + gap, wanted)
    } else if before >= wanted {
        (beside.left() - gap - wanted, wanted)
    } else if right >= before {
        (beside.right() + gap, right.clamp(NARROW, wanted))
    } else {
        let width = before.clamp(NARROW, wanted);
        (beside.left() - gap - width, width)
    };
    let id = Id::new("label-chooser");
    let menu_id = id.with("menu");
    let title = title(&open.table, look, locale);
    let muted = if look.terminal {
        palette.dim
    } else {
        palette.secondary
    };
    let inset = if look.terminal { 10.0 } else { 8.0 };
    let mut asked = None;
    let mut pressed = false;
    let moving = ui.input(|input| input.pointer.is_moving());
    let area = egui::Area::new(id)
        .order(egui::Order::Foreground)
        .fade_in(false)
        .constrain(true)
        .fixed_pos(pos2(left, beside.top()))
        .show(ui.ctx(), |ui| {
            frame.show(ui, |ui| {
                ui.set_width(width - frame.total_margin().sum().x);
                ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
                let justified = egui::Layout::top_down_justified(egui::Align::LEFT);
                ui.with_layout(justified, |ui| {
                    // The head: what is chosen here, and the keys.
                    let head = ui
                        .horizontal(|ui| {
                            ui.add_space(inset);
                            words(ui, &title, muted, look);
                            let keys = keys_line(ui, look);
                            let right = egui::Layout::right_to_left(egui::Align::Center);
                            ui.with_layout(right, |ui| {
                                ui.add_space(inset);
                                words(ui, &keys, muted, look);
                            });
                        })
                        .response
                        .rect;
                    ui.add_space(4.0);
                    // The menu itself, for a screen reader: its lines are
                    // its own.
                    let whole = ui.interact(head, menu_id, Sense::hover());
                    whole.widget_info(|| WidgetInfo::labeled(WidgetType::Other, true, &title));
                    ui.ctx().accesskit_node_builder(menu_id, |node| {
                        node.set_role(Role::Menu);
                    });
                    let lines = UiBuilder::new().accessibility_parent(menu_id);
                    ui.scope_builder(lines, |ui| {
                        for (index, line) in open.lines.iter().enumerate() {
                            // The columns stand under a word of their own.
                            if index == 2 {
                                section(ui, inset, (look, palette, locale));
                            }
                            let row = row(line, open, look, locale);
                            let current = line.choice().as_ref() == Some(&open.choice);
                            let lit = index == open.at;
                            let mut item = menu::Item::choice(&row.text, current)
                                .name(&row.name)
                                .kept_out(lit);
                            if matches!(line, Line::Column { .. }) {
                                item = item.role(widgets::code(look));
                            }
                            if let Some(note) = &row.note {
                                item = item.note(note);
                            }
                            if let Some(detail) = &row.detail {
                                item = item.detail(detail);
                            }
                            if let Some(why) = &row.refused {
                                item = item.disabled(why);
                            }
                            let response = item.show(ui, look, palette);
                            ui.ctx().accesskit_node_builder(response.id, |node| {
                                node.set_role(Role::MenuItemRadio);
                            });
                            if lit {
                                // The keyboard stays in the field: its node
                                // says which line is the current one.
                                ui.ctx().accesskit_node_builder(field, |node| {
                                    node.set_active_descendant(response.id.accesskit_id());
                                });
                            }
                            pressed |= response.is_pointer_button_down_on() || response.clicked();
                            if response.clicked() {
                                asked = Some(Asked::Choose(index));
                            } else if response.hovered() && moving && !lit && asked.is_none() {
                                asked = Some(Asked::Point(index));
                            }
                        }
                        if open.loading {
                            section(ui, inset, (look, palette, locale));
                            loading(ui, inset, muted, (look, palette, locale));
                        }
                    });
                    // The foot: whom the choice is for.
                    ui.add_space(4.0);
                    let rule = ui.available_rect_before_wrap();
                    ui.painter()
                        .hline(rule.x_range(), rule.top(), (1.0, palette.border));
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.add_space(inset);
                        ui.scope(|ui| {
                            ui.set_width(ui.available_width() - inset);
                            words(ui, &foot_text(open, look, locale), muted, look);
                        });
                    });
                    ui.add_space(4.0);
                });
            });
        });
    let rect = area.response.rect;
    // A press between its lines chooses nothing, and is the chooser's.
    pressed |= ui.input(|input| {
        let over = input
            .pointer
            .interact_pos()
            .is_some_and(|at| rect.contains(at));
        over && (input.pointer.any_down() || input.pointer.any_click())
    });
    if open.arrived {
        let at = Rect::from_min_size(rect.min, vec2(1.0, 1.0));
        widgets::announce_polite(ui, at, &gettext(locale, "Columns loaded."));
    }
    Shown {
        pressed,
        asked,
        rect,
    }
}

/// The word the columns stand under.
fn section(ui: &mut Ui, inset: f32, (look, palette, locale): (&Look, &Palette, Locale)) {
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.add_space(inset);
        widgets::section_label(&gettext(locale, "Columns"), look, palette)
            .layout(ui.ctx())
            .label(ui);
    });
    ui.add_space(2.0);
}

/// What stands in the columns' place while they are on their way: that
/// they are loading, and that the two lines above can be chosen at once.
fn loading(
    ui: &mut Ui,
    inset: f32,
    muted: egui::Color32,
    (look, palette, locale): (&Look, &Palette, Locale),
) {
    let say = |words: &'static str| look.label(&gettext(locale, words));
    const SPINNER: f32 = 12.0;
    ui.horizontal(|ui| {
        ui.add_space(inset);
        let (ring, _) = ui.allocate_exact_size(vec2(SPINNER, SPINNER), Sense::hover());
        states::spinner(ui, ring, palette.accent, palette);
        ui.add_space(6.0);
        widgets::label(
            ui,
            widgets::body(look),
            &say("Loading columns…"),
            muted,
            look,
        );
    });
    ui.horizontal(|ui| {
        ui.add_space(inset);
        let text = say("Automatic and No label work now");
        widgets::label(ui, widgets::body(look), &text, palette.dim, look);
    });
}
```

- [ ] **Step 8: The picker's view, and what the chooser draws reaching it**

The foot's words become the button. The "is gone" line stands under the head. The chooser is drawn after the list, beside it; a press beside the chooser closes it.

```diff
diff --git a/src/ui/key_picker.rs b/src/ui/key_picker.rs
--- a/src/ui/key_picker.rs
+++ b/src/ui/key_picker.rs
@@ -10,11 +10,13 @@ use egui::{CornerRadius, Id, Rect, Sense, Ui, UiBuilder, WidgetInfo, WidgetType,
 use tabletist_db::{LookupRow, Value};
 
 use crate::i18n::{Locale, gettext, ngettext};
-use crate::key_picker::{Columns, LIMIT, Picker, VISIBLE, found_in};
+use crate::key_picker::{Columns, LIMIT, Picker, SAID, VISIBLE, found_in};
 use crate::labels::Source;
+use crate::model::{Action, ConnTabId, TabId};
 use crate::theme::{Icon, Look, Palette};
 use crate::typography::{Text, TextRole};
 use crate::ui::format::{display_safe, plain_text};
+use crate::ui::label_chooser;
 use crate::ui::{grid, sql_complete, widgets};
 
 /// What the pointer did to the list this frame.
@@ -27,6 +29,36 @@ pub struct Shown {
     pub picked: Option<usize>,
     /// Set NULL was clicked.
     pub null: bool,
+    /// What was asked of the chooser of the list's label.
+    pub label: Option<LabelAsked>,
+}
+
+/// What the pointer asked of the chooser of what labels the list.
+#[derive(Debug, Clone, Copy, PartialEq, Eq)]
+pub enum LabelAsked {
+    /// The foot's button was clicked: open it, or close it.
+    Toggle,
+    /// The pointer came over this line of it.
+    Point(usize),
+    /// This line of it was clicked.
+    Choose(usize),
+    /// A press landed beside it.
+    Close,
+}
+
+impl LabelAsked {
+    /// What it asks of the reducer, for the editor of the tab `id`.
+    pub fn action(self, tab: ConnTabId, id: TabId) -> Action {
+        match self {
+            Self::Toggle => Action::ToggleLabelChooser { tab, id },
+            Self::Point(line) => Action::PointLabelChooser { tab, id, line },
+            Self::Choose(line) => {
+                let line = Some(line);
+                Action::ChooseLabel { tab, id, line }
+            }
+            Self::Close => Action::CloseLabelChooser { tab, id },
+        }
+    }
 }
 
 /// What the list is of.
@@ -35,6 +67,8 @@ pub struct Names<'a> {
     pub table: &'a str,
     /// The cell's column takes NULL: the foot offers Set NULL.
     pub nullable: bool,
+    /// The chooser of what labels the list, while it is open beside it.
+    pub chooser: Option<&'a label_chooser::Open>,
 }
 
 /// The list's measures in a look.
@@ -215,6 +249,228 @@ pub fn label_line(
     Some((said, Some(at..at + column.len())))
 }
 
+/// What the foot's button is called for a screen reader: "Label column:
+/// name, Automatic". Of a table with nothing to choose it says so, and
+/// why: the button is disabled there.
+pub fn label_name(label: &Columns, table: &str, locale: Locale) -> Option<String> {
+    let Columns::Known(label) = label else {
+        return None;
+    };
+    let words = match (&label.column, label.source) {
+        (_, Source::KeysOnly) => {
+            let nothing = gettext(locale, "{table} has only its key column")
+                .replace("{table}", &display_safe(table));
+            return Some(format!(
+                "{} {nothing}.",
+                gettext(locale, "Label: keys only.")
+            ));
+        }
+        (Some(_), Source::Automatic) => "Label column: {column}, Automatic",
+        (Some(_), Source::User) => "Label column: {column}, Set by you",
+        (None, Source::Automatic) => "Label column: keys only, Automatic",
+        (None, Source::User) => "Label column: keys only, Set by you",
+    };
+    let column = label.column.as_deref().unwrap_or_default();
+    Some(gettext(locale, words).replace("{column}", &display_safe(column)))
+}
+
+/// What a picker says when its chooser is asked for and its table has
+/// nothing to choose: "isbns has only its key column".
+pub fn nothing_text(table: &str, look: &Look, locale: Locale) -> String {
+    look.label(&gettext(locale, "{table} has only its key column"))
+        .replace("{table}", &display_safe(table))
+}
+
+/// How many options the list holds, for what says the label changed:
+/// "2 matches". A list that holds only the first of them says so, in
+/// words for a screen reader ("50 or more matches") and, `terse`, as the
+/// terminal look's field counts them ("50+ matches").
+fn matches_text(picker: &Picker, terse: bool, locale: Locale) -> String {
+    if picker.more {
+        let words = if terse {
+            "{count}+ matches"
+        } else {
+            "{count} or more matches"
+        };
+        return gettext(locale, words).replace("{count}", &LIMIT.to_string());
+    }
+    let count = picker.options.len();
+    let plural = u32::try_from(count).unwrap_or(u32::MAX);
+    ngettext(locale, "{count} match", "{count} matches", plural)
+        .replace("{count}", &count.to_string())
+}
+
+/// What a screen reader is told once the list is searched again by the
+/// label just chosen: "Labelled by legal_name. 2 matches." None before
+/// the answer, and once a text is typed.
+pub fn changed_text(picker: &Picker, locale: Locale) -> Option<String> {
+    let Columns::Known(label) = &picker.label else {
+        return None;
+    };
+    if picker.changed.is_none() || !picker.answered_now() {
+        return None;
+    }
+    let words = match (&label.column, label.source) {
+        (None, _) => "Keys only. {matches}.",
+        (Some(_), Source::Automatic) => "Labelled by {column}, Automatic. {matches}.",
+        (Some(_), Source::User | Source::KeysOnly) => "Labelled by {column}. {matches}.",
+    };
+    let column = label.column.as_deref().unwrap_or_default();
+    let said = gettext(locale, words)
+        .replace("{column}", &display_safe(column))
+        .replace("{matches}", &matches_text(picker, false, locale));
+    Some(said)
+}
+
+/// What is said, once, of a chosen column that is gone: the line the list
+/// draws ("legal_name is gone, so labels use Automatic (name)."), the
+/// terminal look's own form of it, and what a screen reader is told.
+pub struct Gone {
+    pub line: String,
+    pub told: String,
+}
+
+pub fn gone_text(picker: &Picker, look: &Look, locale: Locale) -> Option<Gone> {
+    let (gone, _) = picker.gone.as_ref()?;
+    let Columns::Known(label) = &picker.label else {
+        return None;
+    };
+    // What the app picks in its place: a column, or the keys alone.
+    let keys_only = gettext(locale, "keys only");
+    let auto = label.automatic.as_deref().unwrap_or(&keys_only);
+    let (gone, auto) = (display_safe(gone), display_safe(auto));
+    let line = if look.terminal {
+        "{column} is gone · labels use Automatic ({auto})"
+    } else {
+        "{column} is gone, so labels use Automatic ({auto})."
+    };
+    let fill = |words: String| words.replace("{column}", &gone).replace("{auto}", &auto);
+    Some(Gone {
+        line: fill(look.label(&gettext(locale, line))),
+        told: fill(gettext(locale, "{column} is gone. Labels use Automatic, {auto}.").into_owned()),
+    })
+}
+
+/// How long the foot's button keeps the selection's fill once the label
+/// changed: one flash.
+const FLASH: std::time::Duration = std::time::Duration::from_secs(1);
+
+/// The room round the words of the foot's button, inside it.
+const BUTTON_PAD: f32 = 6.0;
+
+/// The size of the mark that says the button opens a menu.
+const CHEVRON: f32 = 10.0;
+
+/// The foot's button: what labels the options, who chose it, and the
+/// chooser it opens. `words` are its words with the column's name at
+/// `column` in them; they start at `left`, on the line at `y`, and have
+/// `room`. Disabled where the table has nothing to choose, when it says
+/// so under the pointer and, for a while, after its key was pressed.
+/// Clicked and never focused: the field keeps the keyboard. Returns its
+/// place and whether it was clicked.
+fn label_button(
+    ui: &Ui,
+    id: Id,
+    (picker, names): (&Picker, &Names<'_>),
+    (words, column): (&str, Option<std::ops::Range<usize>>),
+    (left, y, room, height): (f32, f32, f32, f32),
+    (look, palette, locale): (&Look, &Palette, Locale),
+) -> (egui::Response, bool) {
+    use crate::keymap::{Command, Scope};
+    let nothing =
+        matches!(&picker.label, Columns::Known(label) if label.source == Source::KeysOnly);
+    let enabled = !nothing;
+    let open = names.chooser.is_some();
+    let line_role = TextRole::pick(look, TextRole::Shortcut, TextRole::OCaption);
+    let name_role = TextRole::pick(look, TextRole::MonoSecondary, TextRole::OCode);
+    let ink = if enabled {
+        muted(look, palette)
+    } else {
+        palette.dim
+    };
+    let mark = if enabled { 4.0 + CHEVRON } else { 0.0 };
+    let at = column.unwrap_or(words.len()..words.len());
+    let whole = || {
+        Text::new(look)
+            .add(line_role, &words[..at.start], ink)
+            .add(name_role, &words[at.clone()], palette.text)
+            .add(line_role, &words[at.end..], ink)
+    };
+    // Cut to the room there is, and then drawn plain: the name's place in
+    // a cut text is not the one it had.
+    let fits = widgets::measure(ui, whole()) + mark <= room;
+    let width = |text: &str| line_role.width(ui.ctx(), look.faces, text);
+    let cut = grid::ellipsize(words, (room - mark).max(0.0), false, width);
+    let text = if fits {
+        whole()
+    } else {
+        Text::one(look, line_role, &cut, ink)
+    };
+    let laid = text.layout(ui.ctx());
+    let wide = laid.width() + mark;
+    let rect = Rect::from_min_size(
+        pos2(left - BUTTON_PAD, y - height / 2.0),
+        vec2(wide + 2.0 * BUTTON_PAD, height),
+    );
+    let sense = if enabled {
+        Sense::CLICK
+    } else {
+        Sense::hover()
+    };
+    let response = ui.interact(rect, id.with("label"), sense);
+    let flashing = picker.changed.is_some_and(|at| at.elapsed() < FLASH);
+    if flashing {
+        ui.ctx().request_repaint_after(FLASH);
+    }
+    let fill = if flashing {
+        Some(palette.selection)
+    } else if enabled && (open || response.hovered()) {
+        Some(palette.surface_hover)
+    } else {
+        None
+    };
+    if let Some(fill) = fill {
+        ui.painter().rect_filled(rect, CornerRadius::same(5), fill);
+    }
+    laid.paint_left(ui.painter(), left, y);
+    if enabled {
+        let place = Rect::from_center_size(
+            pos2(left + laid.width() + 4.0 + CHEVRON / 2.0, y),
+            vec2(CHEVRON, CHEVRON),
+        );
+        Icon::ChevronDown.image(ink, CHEVRON).paint_at(ui, place);
+    }
+    // What the pointer reads over it, and a screen reader after its name.
+    let tip = if enabled {
+        let key = crate::ui::keys::written_in(
+            ui.ctx(),
+            look,
+            Command::ChangeLabelColumn,
+            Scope::CellEditor,
+        );
+        format!("{} {key}", gettext(locale, "Change label column"))
+    } else {
+        nothing_text(names.table, look, locale)
+    };
+    let name = label_name(&picker.label, names.table, locale).unwrap_or_default();
+    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, enabled, &name));
+    ui.ctx().accesskit_node_builder(response.id, |node| {
+        node.set_description(tip.as_str());
+        if enabled {
+            node.set_has_popup(egui::accesskit::HasPopup::Menu);
+            node.set_expanded(open);
+        }
+    });
+    // Its key was pressed and there is nothing to choose: it says why
+    // where the chooser would have opened, for a while.
+    if picker.nothing.is_some_and(|at| at.elapsed() < SAID) {
+        response.show_tooltip_text(tip.as_str());
+        ui.ctx().request_repaint_after(SAID);
+    }
+    let clicked = enabled && response.clicked();
+    (response.on_hover_text(tip), clicked)
+}
+
 /// A label as the text it is written in: a text is borrowed, any other
 /// value is written out.
 fn label_text(label: &Value) -> Cow<'_, str> {
@@ -471,7 +727,15 @@ pub fn show(
     );
     let head = !look.terminal;
     let foot = !look.terminal;
-    let lines = usize::from(head) + usize::from(foot) + usize::from(planned.is_some());
+    // A chosen column that is gone is said in the list, under its head.
+    // The terminal look has no room in the list for words: its status
+    // line says it.
+    let gone = gone_text(picker, look, locale);
+    let notice = gone.as_ref().filter(|_| !look.terminal);
+    let lines = usize::from(head)
+        + usize::from(foot)
+        + usize::from(planned.is_some())
+        + usize::from(notice.is_some());
     // As wide as the cell, and wide enough for a label and a key.
     let base = anchor.width().max(280.0);
     let chrome = 2.0 * shape.pad.y + lines as f32 * shape.line;
@@ -503,6 +767,13 @@ pub fn show(
     let placed = place(anchor, vec2(width, chrome), wanted, screen);
     let size = vec2(width, chrome + rows as f32 * shape.option);
     let mut shown = Shown::default();
+    // Where the foot's label button stands, once it is drawn.
+    let mut button_rect = None;
+    // Where it is kept that a press beside the chooser closed it, until
+    // the button comes up: the frame the list was last drawn in with that
+    // press down.
+    let closed_by_press = Id::new("key-picker").with("closed-by-press");
+    let frame = ui.ctx().cumulative_frame_nr();
     // One area for the lists of every table: only the table on screen
     // has one, and egui keeps each area's place for good.
     let id = Id::new("key-picker");
@@ -540,6 +811,23 @@ pub fn show(
                 say(ui, (left, y), room, (&text, line_role, muted), inner, look);
                 top += shape.line;
             }
+            if let Some(gone) = notice {
+                // On the warning's tint, a dot in its colour before the
+                // words.
+                let tone = crate::ui::states::Tone::Warning;
+                let place = Rect::from_min_size(
+                    pos2(inner.left() + 2.0, top + 1.0),
+                    vec2(inner.width() - 4.0, shape.line - 2.0),
+                );
+                let y = place.center().y;
+                ui.painter()
+                    .rect_filled(place, CornerRadius::same(6), tone.fill(look, palette));
+                ui.painter()
+                    .circle_filled(pos2(left + 4.0, y), 4.0, tone.color(palette));
+                let words = (gone.line.as_str(), line_role, palette.text);
+                say(ui, (left + 16.0, y), room - 16.0, words, inner, look);
+                top += shape.line;
+            }
             let options = UiBuilder::new()
                 .max_rect(inner)
                 .accessibility_parent(whole.id);
@@ -616,21 +904,28 @@ pub fn show(
                 let taken = button.as_ref().map_or(0.0, |(_, wide)| wide + 8.0);
                 let args = (&*cancels, line_role, muted);
                 say(ui, (left, y), room - taken, args, inner, look);
-                // What labels the options stands after it, as words of
-                // its own, cut to the room there is. With too little for
-                // more than a mark it is not drawn.
-                if let Some((label, _)) = label_line(&picker.label, look, locale) {
+                // What labels the options stands after it: a button that
+                // opens the chooser, cut to the room there is. With too
+                // little for more than a mark it is not drawn.
+                if let Some((label, column)) = label_line(&picker.label, look, locale) {
                     let width = |text: &str| line_role.width(ui.ctx(), look.faces, text);
                     let dot = " · ";
                     let from = left + width(&cancels);
                     let label_left = from + width(dot);
-                    let label_room = left + room - taken - label_left;
+                    let label_room = left + room - taken - label_left - BUTTON_PAD;
                     if label_room >= LABEL_ROOM {
                         Text::one(look, line_role, dot, muted)
                             .layout(ui.ctx())
                             .paint_left(&ui.painter().with_clip_rect(inner), from, y);
-                        let args = (&*label, line_role, muted);
-                        say(ui, (label_left, y), label_room, args, inner, look);
+                        let place = (label_left, y, label_room, shape.line - 4.0);
+                        let skin = (look, palette, locale);
+                        let (response, clicked) =
+                            label_button(ui, id, (picker, names), (&label, column), place, skin);
+                        shown.pressed |= response.is_pointer_button_down_on() || response.clicked();
+                        if clicked {
+                            shown.label = Some(LabelAsked::Toggle);
+                        }
+                        button_rect = Some(response.rect);
                     }
                 }
                 if let Some((button, wide)) = button {
@@ -646,8 +941,70 @@ pub fn show(
             if let Some(count) = count_line(picker, names.table, locale) {
                 widgets::announce_polite(ui, rect, &count);
             }
+            // What became of the label, said once each: the words are
+            // the node, and a node that stays is not read again.
+            if let Some(said) = changed_text(picker, locale) {
+                widgets::announce_polite(ui, rect, &said);
+            }
+            if let Some(gone) = &gone {
+                widgets::announce_polite(ui, rect, &gone.told);
+            }
+            // Behind its chooser the terminal look's list gives up its
+            // accent: the chooser has the keys.
+            let border = if look.terminal && names.chooser.is_some() {
+                let corner = CornerRadius::same(shape.corner);
+                let line = egui::Stroke::new(1.0, palette.border);
+                egui::Shape::rect_stroke(rect, corner, line, egui::StrokeKind::Inside)
+            } else {
+                border
+            };
             ui.painter().add(border);
         });
+    if let Some(open) = names.chooser {
+        let list = Rect::from_min_size(placed.pos, size);
+        let chooser = label_chooser::show(ui, open, list, field, (look, palette, locale));
+        shown.pressed |= chooser.pressed;
+        match chooser.asked {
+            Some(label_chooser::Asked::Choose(line)) => {
+                shown.label = Some(LabelAsked::Choose(line));
+            }
+            Some(label_chooser::Asked::Point(line)) if shown.label.is_none() => {
+                shown.label = Some(LabelAsked::Point(line));
+            }
+            Some(label_chooser::Asked::Point(_)) | None => {}
+        }
+        // A press beside the chooser closes it, as it closes a menu. Not
+        // one on the button that opened it, whose click closes it.
+        let pressed_at = ui.input(|input| {
+            let at = input.pointer.interact_pos();
+            at.filter(|_| input.pointer.any_pressed())
+        });
+        let beside = pressed_at.is_some_and(|at| {
+            !chooser.rect.contains(at) && !button_rect.is_some_and(|button| button.contains(at))
+        });
+        if beside && shown.label.is_none() {
+            shown.label = Some(LabelAsked::Close);
+            // The press closes the chooser and does nothing else, as one
+            // beside a menu does: what it landed on is not clicked when
+            // the button comes up, a frame or more from now.
+            ui.data_mut(|data| data.insert_temp(closed_by_press, frame));
+        }
+    }
+    if let Some(seen) = ui.data(|data| data.get_temp::<u64>(closed_by_press)) {
+        // Noted a frame ago at most: the press is the one still down, or
+        // just up. An older note is of a list that closed under its
+        // press, and is nothing to a click on this one.
+        let live = frame.saturating_sub(seen) <= 1;
+        if live {
+            shown.picked = None;
+            shown.null = false;
+        }
+        if live && ui.input(|input| input.pointer.any_down()) {
+            ui.data_mut(|data| data.insert_temp(closed_by_press, frame));
+        } else {
+            ui.data_mut(|data| data.remove_temp::<u64>(closed_by_press));
+        }
+    }
     shown
 }
 
```

```diff
diff --git a/src/ui/cell_editor.rs b/src/ui/cell_editor.rs
--- a/src/ui/cell_editor.rs
+++ b/src/ui/cell_editor.rs
@@ -100,6 +100,8 @@ pub struct Search {
     pub count: Option<String>,
     /// The cell's column takes NULL: the list's foot offers Set NULL.
     pub nullable: bool,
+    /// The chooser of what labels the list, while it is open beside it.
+    pub chooser: Option<crate::ui::label_chooser::Open>,
     /// The search failed: the field goes on as the plain text editor it
     /// was, Esc keeping the edit in the terminal look.
     pub failed: bool,
@@ -1732,6 +1734,7 @@ mod tests {
                 table: "publishers".into(),
                 count: count.map(str::to_owned),
                 nullable: false,
+                chooser: None,
                 failed: false,
                 refused,
             }),
diff --git a/src/ui/data_view.rs b/src/ui/data_view.rs
--- a/src/ui/data_view.rs
+++ b/src/ui/data_view.rs
@@ -1565,6 +1565,7 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabId)
             let names = key_picker::Names {
                 table: &picker.target.name,
                 nullable: search.nullable,
+                chooser: search.chooser.as_ref(),
             };
             let skin = (&look, &palette, locale);
             let shown = key_picker::show(ui, picker, &editor.text, anchor, target.id, &names, skin);
@@ -1574,6 +1575,10 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabId)
                 outcome.kept = false;
             }
             let id = object_tab;
+            // What was asked of the label's chooser ends no edit.
+            if let Some(asked) = shown.label {
+                actions.push(asked.action(tab, id));
+            }
             if let Some(index) = shown.picked {
                 pick = Some(Action::PickOption { tab, id, index });
             } else if shown.null {
@@ -1823,6 +1828,22 @@ pub(super) fn search_of(
             .is_some_and(|column| column.nullable),
         failed: picker.failed.is_some(),
         refused: picker.refused,
+        chooser: picker.chooser.as_ref().map(|chooser| {
+            let known = match &picker.label {
+                crate::key_picker::Columns::Known(label) => Some(label),
+                crate::key_picker::Columns::Waiting => None,
+            };
+            super::label_chooser::Open {
+                at: chooser.at,
+                lines: workspace.label_lines(&picker.target, &picker.key),
+                choice: chooser.choice.clone(),
+                automatic: known.and_then(|label| label.automatic.clone()),
+                loading: known.is_none(),
+                arrived: chooser.waited && known.is_some(),
+                table: picker.target.name.clone(),
+                connection: (workspace.name.clone(), workspace.environment),
+            }
+        }),
     })
 }
 
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -26,6 +26,7 @@ mod key_picker;
 #[cfg(test)]
 mod key_picker_tests;
 pub mod keys;
+pub mod label_chooser;
 #[cfg(test)]
 mod label_tests;
 pub mod menu;
diff --git a/src/ui/row_form.rs b/src/ui/row_form.rs
--- a/src/ui/row_form.rs
+++ b/src/ui/row_form.rs
@@ -238,6 +238,7 @@ impl<'a> Form<'a> {
             let names = crate::ui::key_picker::Names {
                 table: &picker.target.name,
                 nullable: search.nullable,
+                chooser: search.chooser.as_ref(),
             };
             let shown = crate::ui::key_picker::show(
                 ui,
@@ -258,6 +259,10 @@ impl<'a> Form<'a> {
             } else if shown.null {
                 pick = Some(Action::PickerNull { tab, id });
             }
+            // What was asked of the label's chooser ends no edit.
+            if let Some(asked) = shown.label {
+                self.ending.push(asked.action(tab, id));
+            }
         }
         // The text is noted as typed before anything ends the edit, as for
         // a cell's field.
```

- [ ] **Step 9: Run the tests, and look at it**

Run: `~/.cargo/bin/cargo fmt --all && ~/.cargo/bin/cargo test --locked -p tabletist --lib`
Expected: `test result: ok. 2213 passed; 0 failed; 20 ignored` (2195 after task 2; the count moves with `main`).

Then render it. Add a throwaway test at the end of `src/shots.rs`:

```rust
#[test]
#[ignore]
fn throwaway_label() {
    let open = |harness: &mut Harness| {
        let (tab, id) = editable(harness);
        harness.app.apply(Action::EditCell {
            tab,
            id,
            cell: CellPos {
                row: 2,
                col: BOOK_ID,
            },
            start: EditStart::Typed("light".into()),
        });
        answer_books(harness);
        (tab, id)
    };
    both("label-chooser", |harness| {
        let (tab, id) = open(harness);
        harness.app.apply(Action::ToggleLabelChooser { tab, id });
        harness.app.apply(Action::MoveLabelChooser { tab, id, by: 2 });
    });
    both("label-changed", |harness| {
        let (tab, id) = open(harness);
        harness.app.apply(Action::ToggleLabelChooser { tab, id });
        let line = Some(2);
        harness.app.apply(Action::ChooseLabel { tab, id, line });
        answer_books(harness);
    });
    both("label-gone", |harness| {
        let (tab, _) = editable(harness);
        let conn = harness.app.workspace(tab).unwrap().conn_id.clone();
        let books = tabletist_db::ObjectRef::new("public", "books");
        let choice = crate::labels::LabelChoice::Column("subtitle".into());
        harness.app.labels.choose(&conn, &books, choice);
        let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
        harness.app.apply(Action::EditCell {
            tab,
            id,
            cell: CellPos {
                row: 2,
                col: BOOK_ID,
            },
            start: EditStart::Typed("light".into()),
        });
        answer_books(harness);
    });
}
```

Run: `~/.cargo/bin/cargo test --locked --features shots --lib shots::throwaway_label -- --ignored`, then read the pictures in `target/shots/`. Expected, as it was seen where this plan was written:

- `label-chooser-macos-light.png`: the chooser to the list's right, headed `Label books by` with `↑↓ · ↩ · esc`; `✓ Automatic · uses title`, `No label · keys only`, `COLUMNS`, `title` filled with `character varying` at its right, `id` dimmed with `bigint · the key`; the foot's two lines of words. The foot's button is filled and ends in a chevron.
- `label-chooser-omarchy-dark.png`: the chooser to the list's left (the window has no room at its right), in the accent's border, the list's border dimmed. The status line still reads as it does under the picker (`label title · automatic`, and `ctrl+n/p move · enter set · esc close`): task 4 changes it.
- `label-changed-macos-light.png`: the foot's button `Label title · Set by you` on the selection's fill.
- `label-gone-macos-light.png`: under the head, on the warning's tint, a dot and `subtitle is gone, so labels use Automatic (title).`

Take the test out again and delete the pictures. Say in the task's report what you saw.

- [ ] **Step 10: Run the four checks, clippy with `--features shots`, and `tests/keys.rs`**

Expected: all pass.

- [ ] **Step 11: Commit**

```bash
git add src/ui/label_chooser.rs src/key_picker.rs src/model.rs src/app.rs src/app/labels.rs \
  src/app/key_picker.rs src/ui/keys.rs src/ui/menu.rs src/ui/key_picker.rs src/ui/cell_editor.rs \
  src/ui/data_view.rs src/ui/row_form.rs src/ui/mod.rs src/env.rs src/ui/label_tests.rs \
  src/testing.rs tests/keys.rs
git commit -S -m "Choose what labels a foreign key's list from the list itself

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: The Omarchy status line

**Files:**
- Modify: `src/ui/workspace.rs`, `src/ui/key_picker.rs`, `src/keymap.rs`
- Test: `src/ui/label_tests.rs`, `src/ui/key_picker_tests.rs`

The Omarchy look's list has no foot. Its status line says what the macOS look's foot and tooltip say:

- under the picker, `ctrl+l label` among the list's keys (spec section 2), where the table has anything to choose;
- while the chooser is open, what it is called and its keys;
- for 4 s each, that the table has nothing to choose, that a chosen column is gone, and that the label changed, with how many rows the new search found.

The chooser's keys are written by the keymap, the two pairs that move in it as one hint each: `j/k`, then `ctrl+n/p`. `Keymap::pair` is new for that: a key is written in the keymap and nowhere else.

- [ ] **Step 1: Write the failing tests**

The picker's own tests name its line's keys, which gain the chooser's:

```diff
diff --git a/src/ui/key_picker_tests.rs b/src/ui/key_picker_tests.rs
--- a/src/ui/key_picker_tests.rs
+++ b/src/ui/key_picker_tests.rs
@@ -1934,7 +1934,7 @@ fn the_terminal_look_moves_with_ctrl_n_and_closes_with_esc() {
     // announced: the whole of it is one piece of text, and the window is
     // wide enough for the line to keep it.
     harness.settle();
-    let keys = "ctrl+n/p move · enter set · esc close";
+    let keys = "ctrl+n/p move · enter set · ctrl+l label · esc close";
     assert!(
         harness.painted.iter().any(|(piece, _)| piece == keys),
         "{:?}",
@@ -2035,7 +2035,7 @@ fn open_in_terminal(harness: &mut Harness, tab: ConnTabId, id: TabId) {
 
 /// The editor's own words at the bottom of the terminal look.
 const EDITOR_KEYS: &str = "esc normal · tab next cell";
-const PICKER_KEYS: &str = "ctrl+n/p move · enter set · esc close";
+const PICKER_KEYS: &str = "ctrl+n/p move · enter set · ctrl+l label · esc close";
 
 /// The Ctrl+C of a terminal: where Ctrl is the command key too it comes
 /// as a copy (`src/ui/mod.rs` presses it so).
@@ -2165,7 +2165,7 @@ fn a_column_that_is_no_foreign_key_keeps_the_terminal_editor() {
     }
 }
 
-/// In a window too narrow for the picker's three keys, the line falls back
+/// In a window too narrow for the picker's keys, the line falls back
 /// to the one key that differs from the editor's before it drops them all.
 #[test]
 fn a_narrow_line_keeps_esc_close_when_the_keys_do_not_fit() {
```

```diff
diff --git a/src/ui/label_tests.rs b/src/ui/label_tests.rs
--- a/src/ui/label_tests.rs
+++ b/src/ui/label_tests.rs
@@ -1099,3 +1099,102 @@ fn a_press_that_closed_the_picker_too_swallows_no_later_click() {
     assert!(!editing(&harness, tab, id));
     assert!(pending(&harness, tab, id, 0));
 }
+
+// The Omarchy look's status line.
+
+/// The Omarchy look says it in its status line, in its own words.
+#[test]
+fn the_terminal_look_says_a_gone_column_in_its_status_line() {
+    let (mut harness, tab, id) = covers_in(Look::omarchy());
+    keep(&mut harness, tab, &publishers(), column("legal_name"));
+    edit(&mut harness, tab, id, 1);
+    assert!(harness.has("legal_name is gone · labels use automatic (name)"));
+    // In place of the label and the keys, which come back after it.
+    assert!(!harness.has("label name · automatic"));
+}
+
+/// While the chooser is open the status line names it and its keys, as
+/// the keymap has them.
+#[test]
+fn the_terminal_looks_status_line_names_the_chooser_and_its_keys() {
+    let look = Look::omarchy();
+    let (mut harness, tab, id) = typed_in(look, "harb");
+    ask(&mut harness, look);
+    assert!(chooser(&harness, tab, id).is_some());
+    assert!(harness.has("label publishers by"));
+    let keys = "j/k or ctrl+n/p move · enter choose · esc back";
+    assert!(
+        harness.painted_rect(keys).is_some(),
+        "{:?}",
+        harness.painted
+    );
+    // In place of the label and the picker's keys.
+    assert!(!harness.has("label name · automatic"));
+    harness.press(Key::Escape, Modifiers::NONE);
+    assert!(harness.has("label name · automatic"));
+    // Under the picker the chooser's key stands among the list's.
+    let keys = "ctrl+n/p move · enter set · ctrl+l label · esc close";
+    assert!(
+        harness.painted_rect(keys).is_some(),
+        "{:?}",
+        harness.painted
+    );
+}
+
+/// Test 10, in the Omarchy look: its key prints why there is no chooser.
+#[test]
+fn the_terminal_look_says_that_a_table_has_nothing_to_choose() {
+    let look = Look::omarchy();
+    let (mut harness, _, _) = isbn_picker(look);
+    assert!(harness.has("label keys only"));
+    // Its key is not offered: there is nothing for it to open.
+    let keys = "ctrl+n/p move · enter set · esc close";
+    assert!(
+        harness.painted_rect(keys).is_some(),
+        "{:?}",
+        harness.painted
+    );
+    ask(&mut harness, look);
+    assert!(harness.has("isbns has only its key column"));
+}
+
+/// A list that goes on says so in the line that confirms a choice, as the
+/// field counts it.
+#[test]
+fn the_terminal_look_counts_a_list_that_goes_on_as_fifty_and_more() {
+    let look = Look::omarchy();
+    let (mut harness, tab, id) = typed_in(look, "91");
+    ask(&mut harness, look);
+    let line = Some(2);
+    harness.app.apply(Action::ChooseLabel { tab, id, line });
+    harness.answer_lookups(&PUBLISHERS);
+    let object = harness.app.workspace_mut(tab).unwrap();
+    let object = object.object_tab_mut(id).unwrap();
+    let editor = object.edits.editor.as_mut().unwrap();
+    editor.picker.as_mut().unwrap().more = true;
+    assert!(harness.has("label name · 50+ matches"));
+}
+
+/// "Just changed", in the Omarchy look: the status line says what the
+/// list is labelled by now and how many it found, under the mark of a
+/// thing done, in place of its keys.
+#[test]
+fn the_terminal_look_confirms_a_choice_in_its_status_line() {
+    let look = Look::omarchy();
+    let (mut harness, tab, id) = typed_in(look, "91");
+    ask(&mut harness, look);
+    for _ in 0..2 {
+        harness.frame(vec![egui::Event::Text("j".into())]);
+        harness.settle();
+    }
+    harness.press(Key::Enter, Modifiers::NONE);
+    // Not before the answer.
+    assert!(!harness.has("label name · 3 matches"));
+    harness.answer_lookups(&PUBLISHERS);
+    assert!(harness.has("label name · 3 matches"));
+    assert!(picker(&harness, tab, id).changed.is_some());
+    // The next text typed is a new search: the line is the keys' again.
+    harness.frame(vec![egui::Event::Text("0".into())]);
+    harness.settle();
+    assert!(!harness.has("label name · 3 matches"));
+}
```

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib`
Expected: eight fail, each on words the status line does not say yet. In `label_tests`: `the_terminal_look_says_a_gone_column_in_its_status_line`, `the_terminal_looks_status_line_names_the_chooser_and_its_keys`, `the_terminal_look_says_that_a_table_has_nothing_to_choose`, `the_terminal_look_counts_a_list_that_goes_on_as_fifty_and_more`, `the_terminal_look_confirms_a_choice_in_its_status_line`. In `key_picker_tests`: `the_terminal_look_moves_with_ctrl_n_and_closes_with_esc`, `a_failed_search_leaves_the_editors_keys_and_its_esc`, `a_narrow_line_keeps_esc_close_when_the_keys_do_not_fit`.

- [ ] **Step 3: Two keys as one hint, in the keymap**

With its own test.

```diff
diff --git a/src/keymap.rs b/src/keymap.rs
--- a/src/keymap.rs
+++ b/src/keymap.rs
@@ -2181,6 +2181,30 @@ impl Keymap {
             })
     }
 
+    /// The `nth` keys that `first` and `second` have in `scope`, written
+    /// as one hint of two keys that do opposite things: either side of a
+    /// stroke (`j/k`), and what they hold written once where they hold
+    /// alike (`ctrl+n/p`). None where either has no such key.
+    pub fn pair(
+        &self,
+        layout: Layout,
+        (first, second): (Command, Command),
+        scope: Scope,
+        nth: usize,
+    ) -> Option<Written> {
+        let key = |command| {
+            let keys = self.chords(layout, command, scope).nth(nth)?;
+            Some(written(layout, keys))
+        };
+        let (first, second) = (key(first)?, key(second)?);
+        let held = first.rfind('+').map_or("", |plus| &first[..=plus]);
+        let pair = match second.strip_prefix(held) {
+            Some(rest) if !held.is_empty() => format!("{first}/{rest}"),
+            _ => format!("{first}/{second}"),
+        };
+        Some(Written(pair))
+    }
+
     /// The first keys of `commands` written as one hint: `[ ]` for the
     /// previous row and the next, `ctrl+n/p` where they are held alike.
     pub fn together(&self, layout: Layout, commands: &[Command]) -> Written {
@@ -2876,6 +2900,20 @@ mod tests {
         );
     }
 
+    #[test]
+    fn two_keys_that_do_opposite_things_are_written_as_one_hint() {
+        let keymap = Keymap::default();
+        let moves = (Command::NextLabel, Command::PreviousLabel);
+        let pair = |layout, nth| keymap.pair(layout, moves, Scope::Prompt, nth);
+        // Letters either side of a stroke; what is held, once.
+        assert_eq!(pair(Layout::Omarchy, 0).as_deref(), Some("j/k"));
+        assert_eq!(pair(Layout::Omarchy, 1).as_deref(), Some("ctrl+n/p"));
+        assert_eq!(pair(Layout::Omarchy, 2).as_deref(), Some("down/up"));
+        // No such key: no hint.
+        assert_eq!(pair(Layout::Omarchy, 3), None);
+        assert_eq!(pair(Layout::Mac, 1), None);
+    }
+
     #[test]
     fn every_command_has_a_name_of_its_own() {
         let mut names: Vec<&str> = Vec::new();
```

- [ ] **Step 4: The words of a change, for the line**

```diff
diff --git a/src/ui/key_picker.rs b/src/ui/key_picker.rs
--- a/src/ui/key_picker.rs
+++ b/src/ui/key_picker.rs
@@ -322,6 +322,28 @@ pub fn changed_text(picker: &Picker, locale: Locale) -> Option<String> {
     Some(said)
 }
 
+/// The same for the terminal look's status line, which says it for a
+/// while under its own mark: "label legal_name · 2 matches".
+pub fn changed_line(picker: &Picker, look: &Look, locale: Locale) -> Option<String> {
+    let Columns::Known(label) = &picker.label else {
+        return None;
+    };
+    let fresh = picker.changed.is_some_and(|at| at.elapsed() < SAID);
+    if !fresh || !picker.answered_now() {
+        return None;
+    }
+    let words = match &label.column {
+        Some(_) => "Label {column} · {matches}",
+        None => "Label keys only · {matches}",
+    };
+    let column = label.column.as_deref().unwrap_or_default();
+    let said = look
+        .label(&gettext(locale, words))
+        .replace("{column}", &display_safe(column))
+        .replace("{matches}", &matches_text(picker, true, locale));
+    Some(said)
+}
+
 /// What is said, once, of a chosen column that is gone: the line the list
 /// draws ("legal_name is gone, so labels use Automatic (name)."), the
 /// terminal look's own form of it, and what a screen reader is told.
```

- [ ] **Step 5: The status line**

```diff
diff --git a/src/ui/workspace.rs b/src/ui/workspace.rs
--- a/src/ui/workspace.rs
+++ b/src/ui/workspace.rs
@@ -1585,6 +1585,17 @@ struct Editing {
     /// What the picker's options are labelled by, and where in the words
     /// the column's name stands: "label name · automatic".
     label: Option<(String, Option<std::ops::Range<usize>>)>,
+    /// The chooser of that label is open and has the keys: what it is
+    /// called, for the line to say in place of the column and its keys.
+    choosing: Option<String>,
+    /// The picker's table has something to choose a label from: the
+    /// chooser's key stands among the list's.
+    labels: bool,
+    /// What the picker says of its label for a while, in place of the
+    /// editor's keys, and in which tone: that it changed (a success),
+    /// that the column chosen is gone (a warning), or that there is
+    /// nothing to choose (plain words).
+    told: Option<(Option<states::Tone>, String)>,
     /// How much is pending, while anything is: "3 pending · 2 rows".
     pending: Option<String>,
     /// The rows to add, while there are any: "+1 new".
@@ -1797,18 +1808,57 @@ fn editing_status(app: &App, tab: ConnTabId) -> Editing {
                 .is_some_and(|picker| picker.failed.is_none())
     });
     // The list has no foot in this look: the line says what labels it.
-    let label = edits
+    let picker = edits
         .editor
         .as_ref()
         .filter(|_| picking)
-        .and_then(|editor| editor.picker.as_ref())
-        .and_then(|picker| super::key_picker::label_line(&picker.label, look, locale));
+        .and_then(|editor| editor.picker.as_ref());
+    let label =
+        picker.and_then(|picker| super::key_picker::label_line(&picker.label, look, locale));
+    // Not where the table is its key alone: the key opens nothing there.
+    let labels = picker.is_some_and(|picker| {
+        use crate::key_picker::Columns;
+        use crate::labels::Source;
+        match &picker.label {
+            Columns::Waiting => true,
+            Columns::Known(label) => match label.source {
+                Source::Automatic | Source::User => true,
+                Source::KeysOnly => false,
+            },
+        }
+    });
+    let choosing = picker
+        .filter(|picker| picker.chooser.is_some())
+        .map(|picker| super::label_chooser::title(&picker.target.name, look, locale));
+    // One thing at a time: what was just asked for, what is gone, what
+    // changed.
+    let told = picker.and_then(|picker| {
+        use super::key_picker::{changed_line, gone_text, nothing_text};
+        use crate::key_picker::SAID;
+        if picker.nothing.is_some_and(|at| at.elapsed() < SAID) {
+            let said = nothing_text(&picker.target.name, look, locale);
+            return Some((None, said));
+        }
+        if picker
+            .gone
+            .as_ref()
+            .is_some_and(|(_, at)| at.elapsed() < SAID)
+        {
+            let gone = gone_text(picker, look, locale)?;
+            return Some((Some(states::Tone::Warning), gone.line));
+        }
+        let said = changed_line(picker, look, locale)?;
+        Some((Some(states::Tone::Success), said))
+    });
     Editing {
         can_edit,
         insert,
         walks,
         picking,
         label,
+        choosing,
+        labels,
+        told,
         pending,
         added,
         can_add,
@@ -1933,6 +1983,19 @@ fn status_line(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
         .keymap
         .together(app.layout(), &[Command::NextPick, Command::PreviousPick]);
     let (set_key, close_key) = (key(Command::CommitDown), key(Command::ClosePicker));
+    // The keys of the chooser of that list's label, while it is open: the
+    // pairs that move in it, each as one hint (`j/k`, `ctrl+n/p`). The
+    // arrows are not said.
+    let chooser_moves: Vec<crate::keymap::Written> = (0..2)
+        .filter_map(|nth| {
+            let moves = (Command::NextLabel, Command::PreviousLabel);
+            let scope = crate::keymap::Scope::Prompt;
+            app.keymap.pair(app.layout(), moves, scope, nth)
+        })
+        .collect();
+    // The key that opens it from the list.
+    let label_key = key(Command::ChangeLabelColumn);
+    let (choose_key, back_key) = (key(Command::ChooseLabel), key(Command::CloseLabelChooser));
     let Some(workspace) = app.workspace(tab) else {
         return;
     };
@@ -2119,13 +2182,14 @@ fn status_line(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
                 let moves = gettext(locale, "move");
                 let sets = gettext(locale, "set");
                 let closes = gettext(locale, "close");
+                let labels = gettext(locale, "label");
                 let keys = || {
                     if editing.picking {
                         // The list's keys, in place of the editor's. Only
                         // Esc differs: it closes the picker and drops the
                         // edit, where it keeps a text; Enter takes the
                         // highlighted option, and Tab goes on as ever.
-                        return Text::new(&look)
+                        let mut keys = Text::new(&look)
                             .add(role, &pick_keys, palette.text)
                             .space(role, " ")
                             .add(role, &moves, palette.dim)
@@ -2133,7 +2197,17 @@ fn status_line(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
                             .add(role, &set_key, palette.text)
                             .space(role, " ")
                             .add(role, &sets, palette.dim)
-                            .add(role, " · ", palette.dim)
+                            .add(role, " · ", palette.dim);
+                        // The key that chooses what labels the list, where
+                        // there is anything to choose.
+                        if editing.labels {
+                            keys = keys
+                                .add(role, &label_key, palette.text)
+                                .space(role, " ")
+                                .add(role, &labels, palette.dim)
+                                .add(role, " · ", palette.dim);
+                        }
+                        return keys
                             .add(role, &close_key, palette.text)
                             .space(role, " ")
                             .add(role, &closes, palette.dim);
@@ -2152,6 +2226,62 @@ fn status_line(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
                         .add(role, next, palette.dim)
                 };
                 let right = rect.right() - 12.0;
+                if let Some(title) = &editing.choosing {
+                    // The chooser has the keys: the line names it, and
+                    // them where there is room.
+                    let mut x = left + widgets::paint_label(ui, left, y, mode()) + gap;
+                    let text = Text::one(&look, role, title, palette.secondary);
+                    x += widgets::paint_label(ui, x, y, text) + gap;
+                    let (or, chooses, back) = (
+                        gettext(locale, "or"),
+                        gettext(locale, "choose"),
+                        gettext(locale, "back"),
+                    );
+                    let keys = || {
+                        let mut keys = Text::new(&look);
+                        if let Some(first) = chooser_moves.first() {
+                            keys = keys.add(role, first, palette.text);
+                        }
+                        for more in chooser_moves.iter().skip(1) {
+                            keys = keys
+                                .space(role, " ")
+                                .add(role, &or, palette.dim)
+                                .space(role, " ")
+                                .add(role, more, palette.text);
+                        }
+                        keys.space(role, " ")
+                            .add(role, &moves, palette.dim)
+                            .add(role, " · ", palette.dim)
+                            .add(role, &choose_key, palette.text)
+                            .space(role, " ")
+                            .add(role, &chooses, palette.dim)
+                            .add(role, " · ", palette.dim)
+                            .add(role, &back_key, palette.text)
+                            .space(role, " ")
+                            .add(role, &back, palette.dim)
+                    };
+                    if x + widgets::measure(ui, keys()) <= right {
+                        widgets::paint_text_right(ui, right, y, keys());
+                    }
+                    return;
+                }
+                if let Some((tone, said)) = &editing.told {
+                    // What became of the label, for a while: in place of
+                    // the column and the keys, which come back after it.
+                    let mut x = left + widgets::paint_label(ui, left, y, mode()) + gap;
+                    let color = tone.map_or(palette.text, |tone| tone.color(&palette));
+                    let wrote = match tone {
+                        Some(states::Tone::Success) => drawable(ui, role, &look, WROTE),
+                        Some(states::Tone::Warning | states::Tone::Danger) | None => None,
+                    };
+                    if let Some(mark) = wrote {
+                        let mark = Text::one(&look, role, mark, color);
+                        x += widgets::paint_label(ui, x, y, mark) + 6.0;
+                    }
+                    widgets::paint_label(ui, x, y, Text::one(&look, role, said, color));
+                    ui.ctx().request_repaint_after(crate::key_picker::SAID);
+                    return;
+                }
                 let fixed = left + widgets::measure(ui, mode()) + pending_room + errors_room;
                 let column_room = measure(column) + gap;
                 let named = fixed + column_room <= right;
```

- [ ] **Step 6: Run the tests, and look at it**

Run: `~/.cargo/bin/cargo fmt --all && ~/.cargo/bin/cargo test --locked -p tabletist --lib`
Expected: `test result: ok. 2219 passed; 0 failed; 20 ignored`.

Render the three scenes of task 3, step 9 again and read the Omarchy ones. Expected:

- `label-chooser-omarchy-dark.png`: `-- INSERT --  label books by` and, at the right, `j/k or ctrl+n/p move · enter choose · esc back`.
- `label-changed-omarchy-dark.png`: `-- INSERT --  ✓ label title · 2 matches` in the success colour (`√` where the look's font has no `✓`).
- `label-gone-omarchy-dark.png`: `-- INSERT --  subtitle is gone · labels use automatic (title)` in the warning colour.

For the picker's own line add `both("label-picker", edit_key_picker);` to the throwaway test: `label-picker-omarchy-dark.png` reads `-- INSERT --  book_id · int8  label title · automatic` and, at the right, `ctrl+n/p move · enter set · ctrl+l label · esc close`.

Take the test out again and delete the pictures.

- [ ] **Step 7: Run the four checks, and clippy with `--features shots`**

Expected: all pass.

- [ ] **Step 8: Commit**

```bash
git add src/ui/workspace.rs src/ui/key_picker.rs src/keymap.rs src/ui/label_tests.rs \
  src/ui/key_picker_tests.rs
git commit -S -m "Say in the Omarchy status line what the label's chooser is doing

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

## The spec's tests, and where each is

| Spec test | Test |
|---|---|
| Section 2, the Omarchy key hint | `the_terminal_looks_status_line_names_the_chooser_and_its_keys`, and the picker's own tests of its line |
| 2, 4 | `choosing_a_column_keeps_the_edit_and_searches_once` |
| 6, in the chooser | `the_key_cannot_be_chosen_in_the_chooser`; the rule for bytes in `labels::tests::the_chooser_lists_what_can_be_chosen_then_what_cannot` |
| 7 | `esc_closes_the_chooser_first_and_then_the_picker` |
| 8 | `letters_are_the_searchs_until_the_chooser_has_the_keys`, `a_letter_jumps_to_a_column_and_the_arrows_skip_the_key` |
| 9 | `a_chosen_column_that_is_gone_is_said_once`, `the_terminal_look_says_a_gone_column_in_its_status_line`, `an_open_picker_is_labelled_again_when_its_column_goes` |
| 10 | `a_table_of_its_key_alone_opens_no_chooser`, `the_terminal_look_says_that_a_table_has_nothing_to_choose` |
| 11 | `a_choice_can_be_made_before_the_columns_are_known`, `columns_that_arrive_show_in_the_open_chooser` |
| 13 | `a_choice_is_announced_with_the_count_of_its_matches`, `a_choice_is_not_announced_before_its_search_answers`, `a_choice_counts_a_list_that_goes_on_as_fifty_or_more`, `the_terminal_look_counts_a_list_that_goes_on_as_fifty_and_more`, and the "is gone" and "Columns loaded." lines of the tests of 9 and 11 |
| 15 | `keymap::tests::the_label_choosers_keys_are_the_briefs`, and the keymap's own collision test |
| Beyond the list | `the_foots_button_opens_and_closes_the_chooser`, `a_press_beside_the_chooser_closes_it_and_picks_nothing`, `a_press_that_closed_the_picker_too_swallows_no_later_click`, `a_search_that_fails_closes_the_chooser`, `a_choice_for_another_table_searches_nothing` |

## When the run is done

- The four checks pass on the last commit, and each of the four commits passed them.
- Report: what was looked at in tasks 3 and 4, and anything a diff needed that this plan did not say.
- Nothing was pressed in a real window. Worth a press by hand, since the window rewrites some chords before the app sees them: `⌘L` and `ctrl+l` with the picker open, a letter and `j`/`k` in the chooser, Esc twice, and a click on the foot's button and on a line.
- Do not open the pull request: run 4 opens it. Run 3's plan (the Structure door) is written next, against this tree.
