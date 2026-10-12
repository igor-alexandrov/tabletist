# Foreign Key Label, Run 4: The Website and the Pull Request Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The website says what runs 1 to 3 built (the label column of a foreign key, its chooser and its keys in both looks), its Structure picture shows the view as it now is, and the brief's one pull request is open with the description the brief asks for.

**Architecture:** One small change of code, found while the pages were written: the key that the Structure view's button names answers wherever the arrows are. Then five pages of the Jekyll site in `docs/` follow the keymap and the feature, as `AGENTS.md` asks; one picture in `assets/screenshots/` is rendered again from the site's own Bookshop scene; the branch is pushed and the pull request opened.

**Tech Stack:** Rust and the headless tests for task 1; Jekyll (`docs/bin/check` builds the site and proofs its links), the `shots` feature for the picture, `magick`, `gh`. Spec: `docs/superpowers/specs/2026-10-12-foreign-key-label-design.md` (its last section, "PR description").

---

## Read this first

### What was run where this plan was written

- Task 1's change was made with its test first: the test fails on the code before it, and with it the four checks pass, 2967 tests, and clippy with the `shots` feature and `tests/keys.rs`.
- The five pages were written and the site built and checked with `docs/bin/check`: its internal links and their hashes pass, the two new anchors among them (`#the-label-column`, `#the-structure-view`). The diffs below are those pages.
- The Structure picture was rendered from the site's scene and looked at, and converted once to see that it is 2560 by 1600 as the one it replaces. Nothing of it is in the tree.
- Every test the pull request's description names was looked up by name in the tree.
- The plan's reviewer read the pages against the code. What it found is built in: the chooser's key in the Structure view (task 1), the letter that jumps to a column is the pointer looks' alone, the keys that need the view's cursor say so, and the description's smaller slips.

Not run: the pull request's checks. They run on GitHub when it is opened. The PostgreSQL and MySQL tests of this branch have never run: there is no Docker where the runs were made, so there they only print "skipped". The description says so.

### Four things that need a yes

| | What the plan does | Why it is asked |
|---|---|---|
| `cmd+l` in the Structure view, with the arrows on the tree | Task 1 makes it open the chooser. Run 3 built it to answer only while the view has the keys, and tested that. A table opened with a click leaves the arrows on the tree, and a click on **Structure** does not move them: so after the two clicks a new user makes, the key on the view's own button (`Change… ⌘L`) did nothing. The arrows and `alt+cmd+l` still need the view's cursor, and stay as run 3 built them | It changes what run 3's plan said and was approved. With a no, task 1 is left out and the pages say that the key works once a column's line is clicked |
| The Structure picture | Task 3 replaces `assets/screenshots/macos-structure.webp` with a render of the same Bookshop scene (`book_covers`), which now shows the label's card and the `label` tag. It is not blurred, as none of the site's pictures is: they are renders of invented data | The brief says "No unblurred screenshot goes to GitHub", and the site's pictures are pushed only on a yes. With a no, task 3 is left out: the site keeps the old picture, which shows the view without the card, and the description says so |
| Pushing and opening the pull request | Task 4 pushes the branch and opens the pull request. Nothing was pushed in runs 1 to 3 | It is the first thing of this work that leaves this computer |
| The optional door | Not built: **Label {table} by…** in the header menu of a foreign key column of the grid, and `:label` on a foreign key cell. The description lists it as skipped | The brief makes it optional "only if it's cheap". A grid column's header has no menu today, and `:label` over the rows would need the picker's chooser without a picker |

The description carries no screenshot.

### What the pages say, and what they leave to the app

The pages name the keys, the three choices (a column, Automatic, No label), where the choice is kept, and what cannot be chosen. They do not repeat the app's sentences word for word (a notice, an announcement): those are the app's, and a page that quoted them would follow every change of wording.

---

## Before you start

- Cargo is `~/.cargo/bin/cargo`. Never point `CARGO_TARGET_DIR` at `/tmp`.
- The site: `cd docs && bin/check` builds it into `docs/_site` (ignored by git) and proofs it. Run it from `docs/`, and come back to the worktree's root afterwards.
- No em dashes, in a page or in the description.
- This plan is committed first, as the plans of runs 1 to 3 were (`Plan run 4 of the foreign key label: the website and the pull request`). The steps below that read `git status` expect that.
- Commits are signed, one per task. If signing fails ("agent refused operation": 1Password is locked), do not commit unsigned: stage the task and tell the user. Each message ends with:

      Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>

- Chain a commit, a push and `gh` with `&&`, and look at what came back before the next step.

## File structure

| File | What changes |
|---|---|
| `src/ui/keys.rs` | The chooser's key is read while the Structure view shows, apart from the keys that need its cursor (task 1) |
| `src/ui/label_tests.rs` | Its test |
| `docs/_guide/editing-data.md` | The picker's table gets the key of the chooser; a new part, "The label column" |
| `docs/_guide/browsing-data.md` | The Structure view's label, its buttons and keys; in task 3 the picture's words |
| `docs/_reference/keyboard-shortcuts.md` | The Structure view's keys; the chooser's key among the editing keys |
| `docs/_guide/macos.md` | The same, written with Cmd |
| `docs/_guide/omarchy.md` | A part for the Structure view (`j` `k`, `gl`, the `:label` lines); the chooser's keys; what the `:` prompt takes |
| `assets/screenshots/macos-structure.webp` | Rendered again (task 3) |

`docs/_reference/settings-and-files.md` names `labels.json` since run 1. The app's own list of keys (`?`) is made from the keymap and needs nothing.

---

### Task 1: The chooser's key, wherever the arrows are

**Only with the user's yes** (see "Four things that need a yes"). With a no, skip it, and in task 2 write the pages' three "the last two work with the keyboard on the view's columns" sentences as "these work with the keyboard on the view's columns", and the browsing guide's first point as "Click a column's line, then **Change…** or Cmd/Ctrl+L opens the chooser of columns".

**Files:**
- Modify: `src/ui/keys.rs`
- Test: `src/ui/label_tests.rs`

`on_structure` in `src/ui/keys.rs` is the Structure view with the keys: not while the tree has the arrows. The cursor's keys and "use the column under the cursor" need that, since the cursor shows only then. The chooser's key needs no cursor, the tree has no `cmd+l`, and the view's button shows the key at all times. An open chooser already reads its keys wherever the arrows are (`in_structure`).

- [ ] **Step 1: Write the failing test**

It takes the place of the last lines of `the_structure_views_key_opens_the_chooser`, which held the key to the view's having the keys.

```diff
diff --git a/src/ui/label_tests.rs b/src/ui/label_tests.rs
--- a/src/ui/label_tests.rs
+++ b/src/ui/label_tests.rs
@@ -1424,11 +1424,28 @@ fn the_structure_views_key_opens_the_chooser() {
         view,
     });
     assert!(tab_chooser(&harness, tab, id).is_none());
-    // With the arrows on the tree the key is not the view's.
-    to_structure(&mut harness, tab, id);
+}
+
+/// The view's button names the chooser's key, so the key is the view's
+/// wherever the arrows go: a table opened with a click leaves them on
+/// the tree. The keys that need the view's cursor stay the tree's there.
+#[test]
+fn the_choosers_key_is_the_structure_views_with_the_arrows_on_the_tree() {
+    let (mut harness, tab, id) = publishers_structure_in(Look::macos(), false);
     harness.app.workspace_mut(tab).unwrap().pane = Pane::Tree;
+    harness.settle();
     harness.press(Key::L, Modifiers::COMMAND);
+    assert!(tab_chooser(&harness, tab, id).is_some());
+    // The chooser's own keys are read while it is open.
+    harness.press(Key::Escape, Modifiers::NONE);
     assert!(tab_chooser(&harness, tab, id).is_none());
+    // No cursor shows, and the key that uses the column under it keeps
+    // nothing.
+    harness.press(Key::L, Modifiers::COMMAND | Modifiers::ALT);
+    assert!(saved(&harness).is_empty());
+    assert_eq!(cursor(&harness, tab, id), 0);
+    harness.press(Key::ArrowDown, Modifiers::NONE);
+    assert_eq!(cursor(&harness, tab, id), 0);
 }
 
 /// The view has a cursor on the lines of its columns: the arrows move it
```

- [ ] **Step 2: Run it to see it fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib label_tests`
Expected: `the_choosers_key_is_the_structure_views_with_the_arrows_on_the_tree` fails on its first assertion: no chooser opened.

- [ ] **Step 3: Read the key while the view shows**

```diff
diff --git a/src/ui/keys.rs b/src/ui/keys.rs
--- a/src/ui/keys.rs
+++ b/src/ui/keys.rs
@@ -732,10 +732,19 @@ pub fn handle(app: &mut App, ctx: &egui::Context) {
         if on(input, Command::ToggleSidebar, Scope::Global).count > 0 {
             actions.push(Action::ToggleSidebar(active));
         }
-        // The Structure view's keys: its cursor, one line for each press,
-        // the chooser of the table's label, and the column under the
-        // cursor as that label. Fresh presses only for the last two: held,
-        // the first would open the chooser and close it by turns.
+        // The key of the chooser of the table's label is the Structure
+        // view's wherever the arrows go: the view's button names it, and
+        // the tree has no use for it. A fresh press only: held, it would
+        // open the chooser and close it by turns.
+        let labelling = structure.filter(|_| !editing && !focused && app.dialog.is_none());
+        if let Some((tab, id)) = labelling
+            && on(input, Command::OpenLabelChooser, Scope::Grid).fresh
+        {
+            actions.push(Action::ToggleLabelChooser { tab, id });
+        }
+        // The keys that need the view's cursor, which shows while the view
+        // has the keys: the cursor itself, one line for each press, and
+        // the column under it as the label (a fresh press only).
         if let Some((tab, id)) = on_structure {
             let scope = Scope::Grid;
             for keys in keymap.chords_now(layout, Command::MoveInStructure, scope, normal) {
@@ -743,9 +752,6 @@ pub fn handle(app: &mut App, ctx: &egui::Context) {
                 let presses = presses(input, layout, keys).count;
                 actions.extend((0..presses).map(|_| Action::MoveStructureCursor { tab, id, by }));
             }
-            if on(input, Command::OpenLabelChooser, scope).fresh {
-                actions.push(Action::ToggleLabelChooser { tab, id });
-            }
             if on(input, Command::UseAsLabel, scope).fresh {
                 let line = None;
                 actions.push(Action::UseAsLabel { tab, id, line });
```

- [ ] **Step 4: Run the tests**

Run: `~/.cargo/bin/cargo fmt --all && ~/.cargo/bin/cargo test --locked -p tabletist --lib`
Expected: `test result: ok. 2243 passed; 0 failed; 20 ignored`.

- [ ] **Step 5: Run the four checks, clippy with `--features shots`, and `tests/keys.rs`**

```bash
~/.cargo/bin/cargo fmt --all --check
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
~/.cargo/bin/cargo clippy --locked --features shots --lib --tests -- -D warnings
~/.cargo/bin/cargo test --locked --workspace --all-targets
RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --locked --workspace --no-deps
```

Expected: all pass, 2967 tests.

- [ ] **Step 6: Commit**

```bash
git add src/ui/keys.rs src/ui/label_tests.rs
git commit -S -m "Open the label's chooser with its key wherever the arrows are

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: The website's pages

**Files:**
- Modify: `docs/_guide/editing-data.md`, `docs/_guide/browsing-data.md`, `docs/_reference/keyboard-shortcuts.md`, `docs/_guide/macos.md`, `docs/_guide/omarchy.md`

`AGENTS.md`: the reference page follows the keymap's macOS column, written with Mod for macOS and Windows together; `_guide/macos.md` writes it with Cmd; `_guide/omarchy.md` writes the Omarchy column. The keys, from `BINDINGS` in `src/keymap.rs`:

| Command | macOS column | Omarchy column |
|---|---|---|
| `ChangeLabelColumn` (a foreign key cell's list) | `cmd+l` | `ctrl+l` |
| `NextLabel`, `PreviousLabel` (the chooser) | `down`, `up` | `j` `ctrl+n` `down`, `k` `ctrl+p` `up` |
| `ChooseLabel`, `CloseLabelChooser` | `enter`, `esc` | `enter`, `esc` |
| `MoveInStructure` | `up` `down` | `j` `k` `up` `down` |
| `OpenLabelChooser` | `cmd+l` | `:label` |
| `UseAsLabel` | `alt+cmd+l` | `gl` |
| `LabelAutomatic`, `LabelKeysOnly`, `LabelColumn` | the button and the chooser | `:label auto`, `:label none`, `:label {column}` |

- [ ] **Step 1: The editing guide**

```diff
diff --git a/docs/_guide/editing-data.md b/docs/_guide/editing-data.md
--- a/docs/_guide/editing-data.md
+++ b/docs/_guide/editing-data.md
@@ -90,10 +90,11 @@ the rows that match are listed under the cell with their ids.
 | Move in the list | Up, Down, Page Up, Page Down |
 | Set the cell to the highlighted row | Enter, or Tab to move on |
 | Take a row with the pointer | Click it |
+| Choose the column that labels the list | Cmd/Ctrl+L, or click the label at the list's foot |
 | Leave the cell as it was | Esc |
 
-The list is searched by the table's first text column and by its key. An id
-you type or paste in full is taken as it is.
+The list is searched by the table's [label column](#the-label-column) and
+by its key. An id you type or paste in full is taken as it is.
 
 The first row is highlighted when what you typed is in its name or is its
 id. When the rows only begin with what you typed in their ids, none is, and
@@ -109,6 +110,45 @@ NULL, **Set NULL** under the list sets it, as Cmd/Ctrl+Backspace does.
 The picker is on the cells of the grid and on the fields of the row panel,
 in a new row and in a copy too. A key of several columns is edited as text.
 
+#### The label column
+
+The label column is what a row of the other table is read by: each line of
+the list shows it beside the row's id, and it is what your text is searched
+in. The foot of the list names it, **Label name · Automatic**. The Omarchy
+look names it in the status line.
+
+Automatic is the table's first text column that is not a key. To pick
+another, press Cmd/Ctrl+L or click the label at the foot. A chooser opens
+beside the list. The cell stays in edit, with your text as you left it.
+
+| To | Press |
+| --- | --- |
+| Move in the chooser | Up, Down (`j` and `k` too in the Omarchy look) |
+| Jump to a column | Type the letter its name begins with (not in the Omarchy look, where letters are keys) |
+| Choose the highlighted line | Enter, or click it |
+| Close the chooser and keep the label | Esc |
+
+**Automatic** hands the pick back to the app, and **No label** lists the
+rows by their keys alone. A key and a column of bytes are listed, but
+cannot be chosen. A date or a number can: it is searched as the text the
+list shows.
+
+The list is searched again by the new label at once, and its foot says
+**Set by you** where you picked a column or No label. The choice is the
+table's: every foreign key that points at it uses it. It is kept for that
+saved connection, in `labels.json` among
+[the app's files]({% link _reference/settings-and-files.md %}) on this
+computer. Nothing is written to the database.
+
+The same choice can be made with no cell open, in the table's
+[Structure view]({% link _guide/browsing-data.md %}#structure). On a
+read-only connection, where no cell opens, that is where it is made.
+
+When a column you chose is renamed or dropped, the label goes back to
+Automatic and the list says so once (the status line does, in the Omarchy
+look). A table that has only its key column has nothing to choose, and
+says so.
+
 An id you type yourself is looked up when you leave the cell. If no row
 has it, the cell gets an outline and says so under the pointer, the row
 panel says it under the field ("No publisher with id 12"), and the bar
```

- [ ] **Step 2: The browsing guide**

The picture's words stay as they are until task 3 replaces the picture.

```diff
diff --git a/docs/_guide/browsing-data.md b/docs/_guide/browsing-data.md
--- a/docs/_guide/browsing-data.md
+++ b/docs/_guide/browsing-data.md
@@ -85,6 +85,23 @@ and foreign keys.
 
 {% include shot.html file="macos-structure" alt="The Structure view of a table, listing its columns and indexes" %}
 
+Above the columns, **Label for foreign keys** names the column that this
+table's rows are read by in the list of a foreign key that points at it,
+and says whether the app picked it (**Automatic**) or you did (**Set by
+you**). The column itself is tagged **label**.
+[Editing Data]({% link _guide/editing-data.md %}#the-label-column) says
+what the label does.
+
+- **Change…** (Cmd/Ctrl+L) opens the chooser of columns.
+- Click a column's line and press Cmd/Ctrl+Alt+L to use that column, or
+  right-click the line for **Use as label**. With the keyboard, Tab or F6
+  comes to the columns, and Up and Down move between them.
+- **Use Automatic** hands the pick back to the app.
+
+No cell is edited and nothing is written to the database, so this works on
+a read-only connection. The Omarchy look does the same with `gl` and the
+`:label` lines of [its Structure view]({% link _guide/omarchy.md %}#the-structure-view).
+
 ## Copying
 
 On macOS and Windows, Cmd/Ctrl+C copies the selected cell and
```

- [ ] **Step 3: The reference page and the two looks' guides**

```diff
diff --git a/docs/_reference/keyboard-shortcuts.md b/docs/_reference/keyboard-shortcuts.md
--- a/docs/_reference/keyboard-shortcuts.md
+++ b/docs/_reference/keyboard-shortcuts.md
@@ -71,6 +71,17 @@ In the connection dialog:
 | Mod+C | Copy the cell |
 | Mod+Shift+C | Copy the row |
 
+In a table's [Structure view]({% link _guide/browsing-data.md %}#structure):
+
+| Keys | Do |
+| --- | --- |
+| Mod+L | Choose the column that labels the table's rows for foreign keys |
+| Up, Down | Move by column |
+| Mod+Alt+L | Use the column under the cursor as that label |
+
+The last two work with the keyboard on the view's columns: click a
+column's line, or come to them with Tab or F6.
+
 ## Editing
 
 These are the keys of the macOS and Windows looks. The Omarchy look has
@@ -85,6 +96,7 @@ These are the keys of the macOS and Windows looks. The Omarchy look has
 | Enter, Tab, Shift+Tab | Keep the edit and move down, right or left |
 | Alt+Enter | Open the large editor. In it, Mod+Enter applies and Esc cancels |
 | Esc | Cancel the edit |
+| Mod+L | In a foreign key cell's list: [choose the column that labels it]({% link _guide/editing-data.md %}#the-label-column). In the chooser, Up and Down move, Enter chooses and Esc closes |
 | Mod+Backspace | Set NULL |
 | Mod+' | Set DEFAULT, where the column has a default |
 | Space | Flip a boolean cell: true, false, NULL |
```

```diff
diff --git a/docs/_guide/macos.md b/docs/_guide/macos.md
--- a/docs/_guide/macos.md
+++ b/docs/_guide/macos.md
@@ -110,6 +110,17 @@ a Mac keyboard labels Delete.
 | Cmd+I | Open the row panel, with the keyboard on its fields. Esc goes back to the rows |
 | Cmd+Shift+R | Show or hide the row panel |
 
+In its Structure view:
+
+| Keys | Do |
+| --- | --- |
+| Cmd+L | Choose the column that labels the table's rows for foreign keys |
+| Up, Down | Move by column |
+| Cmd+Option+L | Use the column under the cursor as that label |
+
+The last two work with the keyboard on the view's columns: click a
+column's line, or come to them with Tab or F6.
+
 ### Editing
 
 | Keys | Do |
@@ -120,6 +131,7 @@ a Mac keyboard labels Delete.
 | Backspace, Forward Delete | Drop a new row that is not saved yet |
 | Return, Tab, Shift+Tab | Keep the edit and move down, right or left |
 | Esc | Cancel the edit |
+| Cmd+L | In a foreign key cell's list: choose the column that labels it. In the chooser, Up and Down move, Return chooses and Esc closes |
 | Option+Return | Open the large editor. In it, Cmd+Return applies and Esc cancels |
 | Cmd+Backspace | Set NULL |
 | Cmd+' | Set DEFAULT, where the column has a default |
```

```diff
diff --git a/docs/_guide/omarchy.md b/docs/_guide/omarchy.md
--- a/docs/_guide/omarchy.md
+++ b/docs/_guide/omarchy.md
@@ -97,8 +97,27 @@ list instead.
 | `R` | Reload |
 | Ctrl+C | Cancel the running query |
 
-The Structure and Data views are clicked: they have no key. Nor has
-copying a whole row.
+The Structure and Data views are clicked: switching between them has no
+key. Nor has copying a whole row.
+
+### The Structure view
+
+Ctrl+L brings the keyboard to its columns, as it does to the rows.
+
+| Keys | Do |
+| --- | --- |
+| `j` `k`, Up, Down | Move by column |
+| `gl` | Use the column under the cursor as the label for foreign keys |
+| `:label` | Choose that label from a list. In it, `j` `k` move, Enter chooses and Esc closes |
+| `:label auto` | Back to the app's own pick |
+| `:label none` | No label: rows are listed by their keys alone |
+| `:label` and a column's name | Use that column |
+
+The label is the column that the table's rows are read by in the list of a
+foreign key that points at it:
+[Editing Data]({% link _guide/editing-data.md %}#the-label-column) says
+what it does. A key or a column of bytes is not taken, and the status line
+says why.
 
 ### The row panel
 
@@ -130,6 +149,7 @@ copying a whole row.
 | Ctrl+N, Ctrl+P, Down, Up, Page Down, Page Up | Move in a foreign key cell's list |
 | Enter, Tab | In that list: set the cell to the highlighted row |
 | Esc | In that list: close it and leave the cell as it was |
+| Ctrl+L | In that list: choose the column that labels it. In the chooser, `j` `k`, Ctrl+N, Ctrl+P and the arrows move, Enter chooses and Esc closes |
 | Alt+Enter | Open the large editor. In it, Ctrl+Enter applies, Esc keeps the text, Ctrl+C drops it and Ctrl+Shift+F formats JSON |
 | `x` | Set NULL. On a column that cannot be NULL, the status line says so |
 | `D` | Set DEFAULT, where the column has a default |
@@ -144,8 +164,8 @@ copying a whole row.
 A date or time cell has no key for the time of the save: type `now()`
 into it.
 
-The `:` prompt takes `w`, `e!` and `diff` on a table, `ro` and `rw` in a
-SQL editor, and nothing else.
+The `:` prompt takes `w`, `e!` and `diff` on a table, the `label` lines in
+its Structure view, `ro` and `rw` in a SQL editor, and nothing else.
 
 {% include shot.html file="omarchy-edit-review" alt="The :diff panel above the status line, showing the statement a save will run" %}
 
```

- [ ] **Step 4: Build and check the site**

Run: `cd docs && bin/check`
Expected: `HTML-Proofer finished successfully.` A link to an anchor that is not there fails here.

- [ ] **Step 5: Commit**

```bash
git add docs/_guide/editing-data.md docs/_guide/browsing-data.md docs/_reference/keyboard-shortcuts.md docs/_guide/macos.md docs/_guide/omarchy.md
git commit -S -m "Say on the website how a foreign key's label column is chosen

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: The Structure picture

**Only with the user's yes** (see "Four things that need a yes"). With a no, skip to task 4 and use the description's other line about the picture.

**Files:**
- Modify: `assets/screenshots/macos-structure.webp`, `docs/_guide/browsing-data.md`

No other picture of the site changes: none of their scenes shows a Structure view or the list of a foreign key.

- [ ] **Step 1: Render the scene**

Put this test into `mod mock` of `src/shots.rs`, right before `fn mock()`. It is the site's scene: the Bookshop's `book_covers`, the macOS look, light, 1280 by 800 at a scale of 2.

```rust
    #[test]
    #[ignore = "throwaway"]
    fn website_structure() {
        let size = egui::vec2(1280.0, 800.0);
        let mut harness = Harness::for_shots(size, 2.0, true, Look::macos());
        crate::util::pin_now(Some(NOW));
        let tab = workspace(&mut harness);
        let object_tab = harness.app.workspace(tab).unwrap().active_tab.unwrap();
        let view = crate::model::ObjectView::Structure;
        harness.app.apply(Action::SetView {
            tab,
            object_tab,
            view,
        });
        harness.finish_animations();
        harness.shot(&out("site-m-macos-structure.png"));
    }
```

Run: `~/.cargo/bin/cargo test --locked --features shots --lib shots::mock::website_structure -- --ignored --exact`
Then take the test out again: `git checkout -- src/shots.rs`.

- [ ] **Step 2: Convert it, and look at it**

```bash
magick target/shots/site-m-macos-structure.png -strip -define webp:lossless=true -define webp:method=6 assets/screenshots/macos-structure.webp
magick identify assets/screenshots/macos-structure.webp
```

Expected: `WEBP 2560x1600`. Look at the picture. Expected, as it was seen where this plan was written: `book_covers` in its Structure view; over Columns a card, `Label for foreign keys  kind · Automatic`, its sentence, and `Change… ⌘L` at the card's end; a blue tag `label` after `kind`; the cursor's fill on `id`; Indexes and Foreign keys under the columns. Bookshop names only.

Delete `target/shots/site-m-macos-structure.png`.

- [ ] **Step 3: The picture's words**

```diff
diff --git a/docs/_guide/browsing-data.md b/docs/_guide/browsing-data.md
--- a/docs/_guide/browsing-data.md
+++ b/docs/_guide/browsing-data.md
@@ -83,7 +83,7 @@ A foreign key has a link next to it that opens the row it points to.
 Switch a table from **Data** to **Structure** to read its columns, indexes
 and foreign keys.
 
-{% include shot.html file="macos-structure" alt="The Structure view of a table, listing its columns and indexes" %}
+{% include shot.html file="macos-structure" alt="The Structure view of a table: the label for foreign keys, then its columns, indexes and foreign keys" %}
 
 Above the columns, **Label for foreign keys** names the column that this
 table's rows are read by in the list of a foreign key that points at it,
```

- [ ] **Step 4: Build and check the site**

Run: `cd docs && bin/check`
Expected: `HTML-Proofer finished successfully.`

- [ ] **Step 5: Commit**

```bash
git status --short
git add assets/screenshots/macos-structure.webp docs/_guide/browsing-data.md
git commit -S -m "Show the label for foreign keys in the website's Structure picture

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

`git status --short` before it shows these two files and no `src/shots.rs`.

---

### Task 4: The pull request

**Only with the user's yes.**

- [ ] **Step 1: The four checks, on the commit that is pushed**

```bash
~/.cargo/bin/cargo fmt --all --check
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
~/.cargo/bin/cargo test --locked --workspace --all-targets
RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --locked --workspace --no-deps
```

Expected: all pass, 2967 tests. `git status --short` is empty, and `git log --oneline origin/main..HEAD` lists the branch's commits with `origin/main` (fetched now) an ancestor of `HEAD`. If `main` has moved, stop and tell the user: rebasing signed commits is theirs to ask for.

- [ ] **Step 2: Write the description to a file in the scratchpad**

Not in the repository. Where task 3 was skipped, replace the line under "Screenshots" as the note there says.

````markdown
## What this is

The user can choose which column labels the rows of a table in the list of a foreign key, in both looks. Until now the app picked it (the table's first text column that is not a key) and said nothing about it.

- The list of a foreign key cell says which column labels it and who chose it: **Label name · Automatic**, or **Set by you**. The Omarchy look says it in its status line.
- Cmd/Ctrl+L, or a click on that label, opens a chooser beside the list: Automatic, No label, and the table's columns. The cell stays in edit, the text and the caret stay as they were, and the list is searched again by the new label.
- The same choice is made with no cell open in the table's Structure view, on a read-only connection too: a "Label for foreign keys" card with **Change…** and **Use Automatic**, a `label` tag on the column, a cursor on the columns' lines, **Use as label** in a line's menu, and in the Omarchy look `gl` and the `:label` lines.
- The choice is kept per saved connection and table, in `labels.json` in the app's state directory. **Nothing is written to the database**: choosing sends only the list's own `SELECT` again.
- A column that is renamed or dropped goes back to Automatic and is said once. A table of its key alone has nothing to choose and says so. While the columns are being read, Automatic and No label can be chosen already.

The spec is `docs/superpowers/specs/2026-10-12-foreign-key-label-design.md` (the brief, word for word), and the four plans beside it in `docs/superpowers/plans/` say for each run what was built, what was checked and where the build differs from a board, with the reason.

## The boards

Canvas "Tabletist", section **Editing · Values**:

| Board | |
|---|---|
| `macOS – Foreign key label, picker and chooser` | built |
| `macOS – Foreign key label, Structure and states` | built |
| `Omarchy – Foreign key label, picker and chooser` | built, on the picker as the app has it (the list under the cell's field) |
| `Omarchy – Foreign key label, structure and states` | built, in the app's one Structure view |
| `Foreign key label, two columns (variant)` | not in this pull request; its key is reserved |
| `Foreign key label, notes` | the strings, names and announcements |

Matched as far as they bear on this: `macOS – Editing values, editors by type` (card 6), `macOS – Editing a row`, `macOS – Inserting a row`, `Omarchy – Editing values, editors by type` (card 6), `Omarchy – Components`.

## The keys

All of them are in the one keymap (`src/keymap.rs`), and its collision test passes. `cmd` is Ctrl on Windows and Linux.

| Command | macOS, Windows, Linux | Omarchy | Where |
|---|---|---|---|
| Change label column | `cmd+l` | `ctrl+l` | A foreign key cell's list |
| Next, previous line | `down`, `up` | `j` `ctrl+n` `down`, `k` `ctrl+p` `up` | The chooser |
| Choose | `enter` | `enter` | The chooser |
| Close the chooser | `esc` | `esc` | The chooser |
| Add a second column | `shift+enter` | `shift+enter` | Reserved for the two-column variant: nothing answers it |
| Open the chooser | `cmd+l` | `:label` | The Structure view |
| Use the column under the cursor | `alt+cmd+l` | `gl` | The Structure view |
| Back to Automatic | the Use Automatic button | `:label auto` | The Structure view |
| Keys only | the chooser | `:label none` | The Structure view |
| A column by its name | the chooser | `:label {column}` | The Structure view |
| Move the view's cursor | `up` `down` | `j` `k` `up` `down` | The Structure view |

The Structure view's keys are bound in the scope `grid` under a new condition ("the Structure view shows"), not in a scope of their own: the view is the table's tab, and the grid's `:w` and `:e!` work in it.

## The tests

Headless, through the AccessKit tree and the commands sent. None checks a pixel or a design, and no snapshot is added. By the brief's numbers:

| | Test |
|---|---|
| 1 | `automatic_resolves_to_the_apps_pick`, `automatic_is_the_apps_pick_and_the_picker_says_whose_it_is` |
| 2, 4 | `choosing_a_column_keeps_the_edit_and_searches_once` |
| 3 | `a_choice_labels_every_key_that_points_at_the_table`, `a_choice_is_its_saved_connections_alone`, `a_choice_is_kept_by_connection_and_table` |
| 5 | `the_structure_view_sets_the_label_on_a_read_only_connection` |
| 6 | `the_key_cannot_be_chosen_in_the_chooser`, `the_key_and_bytes_are_refused_with_their_reason`, `the_terminal_look_says_why_a_named_column_is_refused`, and the rule in `the_key_and_bytes_are_never_a_label`, `a_column_named_by_hand_is_refused_with_its_reason` |
| 7 | `esc_closes_the_chooser_first_and_then_the_picker` |
| 8 | `letters_are_the_searchs_until_the_chooser_has_the_keys` |
| 9 | `a_chosen_column_that_is_gone_falls_back_to_the_apps_pick`, `a_lost_column_is_said_once`, `a_chosen_column_that_is_gone_is_said_once`, `the_terminal_look_says_a_gone_column_in_its_status_line`, `an_open_picker_is_labelled_again_when_its_column_goes` |
| 10 | `a_table_of_its_key_alone_opens_no_chooser`, `the_terminal_look_says_that_a_table_has_nothing_to_choose`, `the_structure_view_of_a_table_of_its_key_alone_says_so` |
| 11 | `a_choice_can_be_made_before_the_columns_are_known`, `columns_that_arrive_show_in_the_open_chooser` |
| 12 | `the_terminal_looks_keys_keep_what_the_buttons_keep` |
| 13 | `a_choice_is_announced_with_the_count_of_its_matches`, `a_choice_is_not_announced_before_its_search_answers`, `a_choice_counts_a_list_that_goes_on_as_fifty_or_more`, `the_terminal_look_counts_a_list_that_goes_on_as_fifty_and_more` |
| 14 | `labels_that_are_no_text_are_searched_as_drawn_on_sqlite`, `..._on_postgres`, `..._on_mysql` |
| 15 | `the_label_choosers_keys_are_the_briefs`, `the_structure_views_label_keys_are_the_briefs`, and the keymap's `no_two_commands_share_a_key_where_both_could_be_read` |

More tests than the list asks for are in `src/ui/label_tests.rs` and `src/labels.rs` (a letter that jumps to a column in the chooser, a press beside the chooser, the Structure view's cursor kept in view, among others); the plans of runs 2 and 3 name them.

Every commit passed `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo doc` and the tests where it was made: 2967 on the last commit.

**Not run where this was built:** the PostgreSQL and MySQL tests, test 14's two among them. There was no Docker there, so they only printed "skipped". This pull request's checks are their first run. It was built and tested on Linux alone: the checks' macOS and Windows jobs are the first build there (the branch adds no code that depends on the system). Nothing was pressed in a real window either: the views were rendered headless in the three looks and looked at.

## The optional door: skipped

**Label {table} by…** in the header menu of a foreign key column of the grid, and `:label` on a foreign key cell, are not built. A grid column's header has no menu today, and `:label` over the rows would need the picker's chooser without a picker.

## Placeholder columns replaced

The boards' `legal_name`, `email`, `city`, `founded_on` and `logo` of `publishers`, and their invented authors, are in no test. The tests use what the Bookshop fixture has:

- `publishers (id, name)`, `book_covers.publisher_id`, `genres (id, parent_id, name)`.
- `legal_name` only as the name of a column a table lacks: the column that is gone, or one named by hand that is not there.
- The fixture had no `isbns` table. Two tables are added to `src/testing.rs`, the smallest that give a table of its key alone and a key that points at it: `isbns (isbn TEXT PRIMARY KEY)`, and `editions (id, isbn, title, cover BLOB)` with two rows, whose `isbn` references `isbns.isbn`. `cover` is there so that a chooser lists a column of bytes.
- The database tests of test 14 make a table of editions of their own (`editions` on SQLite, `lookup_editions` on PostgreSQL and MySQL), with a date and a number to label by.

## Open questions

Every default is kept: separate choices per saved connection, labels searched only as drawn, no sample values in the chooser, nothing new in the referencing table's Structure view, the state file and not `settings.toml`, lower case in the Omarchy look.

## Where the build differs from a board

Each is in its run's plan with the reason. The ones a reviewer will meet first:

- The Omarchy picker stays the list under the cell's field (pull request 114), not the boards' float; the chooser is a float beside it.
- The Omarchy chooser's lines are the app's menu rows (one mark column), and the Structure view keeps its five columns in every look.
- The Structure view gets a cursor on its columns' lines, and is a pane the keyboard can be brought to. While it has the keys, the arrows are its own and Reload reloads the table; with the keys on the tree they are the tree's, as before. The chooser's key (`cmd+l`) is the view's wherever the arrows are, as its button names it.
- The "Label for foreign keys" card shows for every table, not only for one a key points at: which tables are pointed at is not known without reading every table's keys.
- `:label auto` and `:label none` are read as spelled. A column called `auto` or `none` is chosen in the chooser.

## The website

`docs/_guide/editing-data.md` has a part on the label column, `docs/_guide/browsing-data.md` the Structure view's card, and the three key pages the new keys. `docs/bin/check` passes.

## Screenshots

None here. The website's `macos-structure.webp` is rendered again from its Bookshop scene, as the site's other pictures are: invented data, the same scene as before, now with the card.

<!-- Where the picture was not replaced, write instead: "None here. The website's Structure picture is the old one and shows the view without the card." -->

🤖 Generated with [Claude Code](https://claude.com/claude-code)
````

- [ ] **Step 3: Push, and open the pull request**

```bash
git push -u origin claude/foreign-key-label-chooser-062e4d && \
gh pr create --base main --head claude/foreign-key-label-chooser-062e4d \
  --title "Choose what labels a foreign key's rows: the spec, the plans, and the four runs" \
  --body-file <the file of step 2>
```

Never set `SSH_AUTH_SOCK` for the push: the hook finds the agent. Check that `git rev-parse HEAD` is `git rev-parse origin/claude/foreign-key-label-chooser-062e4d` afterwards.

- [ ] **Step 4: Bind the pull request and read its checks once**

With the `ccd_pr` tools: `get_status`, and `bind_pr` if it does not report the pull request. Read the checks it shows and tell the user where they stand. Do not poll them, and do not turn on auto-merge.

---

## When the run is done

- Report the pull request's link, what its checks said when they were read, and whether tasks 1 and 3 ran.
- The first run of the PostgreSQL and MySQL tests of this branch is the pull request's own. If one fails, read it and tell the user before changing anything.
