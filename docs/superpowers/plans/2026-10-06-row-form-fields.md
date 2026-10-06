# Row Form, Step 1: Edit a Field in Place Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** On a table whose row can be edited, a one-line value is edited in the row panel, in its place: by a double-click, the pencil in its label line, Enter or F2, the footer's Edit, `Mod+I`, or `e` on Omarchy. Its text becomes the same pending cell an edit in the grid makes. A field that cannot be edited wears a lock and says why, and the footer says why a whole row cannot.

**Architecture:** The tab's one editor (`Edits::editor`) gains a place, `EditorPlace::Grid` or `Panel`. `Action::EditField` opens it in the panel and `Action::EditRow` picks the field; everything after that (the check, the pending set, Save, Review SQL, the questions, the leaving guard) is the path the grid's editor already takes. `edit::Table::row_lock` is cut out of `Table::lock`, so the panel can tell "no field of this row" from "not this field". A new view module, `src/ui/row_form.rs`, says what the form makes of each field and draws the editor through `cell_editor::in_panel`; `src/ui/row_panel.rs` draws the pencil, the lock, the reason and the footer. The grid draws an editor only where its place is the grid.

**Tech Stack:** Rust 1.98, egui/eframe (the crmne fork). Spec: `docs/superpowers/specs/2026-10-06-row-form-design.md` (this plan is its step 1, "Edit a field in place").

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

- `src/shots.rs` is outside those four. Where a task touches it (tasks 2 and 7), also run `~/.cargo/bin/cargo clippy --locked --features shots --lib --tests -- -D warnings`.
- **What was run where this plan was written.** The plan's code comes from a draft that was built task by task on Linux. The tasks were then replayed one after another from a clean tree, each passing the four checks and the shots lint (1,771 tests in the main crate at the end, 1,742 after task 1). Each block below was made from that replay: a task's tests' block applies on the tree the task before left, and its code's block after it gives exactly the draft's tree. For tasks 4, 5 and 6 the tests were also run alone on the code before the task, to record the failure their "run it to see it fail" step expects; for tasks 1, 2 and 3 the tests name things that do not exist yet, and the failure is the compiler's. The scenes of task 7 were rendered off screen and looked at in every look (see "By hand" for what that does not cover). A review of the plan found three small faults in the draft, fixed with their tests before the plan was finished: the field owed the keyboard was remembered after the selection moved or the panel closed; the keyboard's return to a field was not drawn until the next event; and F2 did nothing on a pencil that had the keyboard.
- **Databases.** Nothing here changes what a save sends, reads or compares: no test needs a PostgreSQL or a MySQL server, and the suite that does is as it was. Neither server was reachable where the plan was written; CI runs those suites.
- **Platforms.** Nothing here is behind a `cfg`, so one platform's build is every platform's. Only Linux was built; CI builds and tests macOS and Windows.
- **The branch.** Work on `claude/row-panel-form`, cut from `main` at `f971b5b` (the merge of pull request #87). The spec is committed there (`12ba3a2`). Before task 1, commit this plan on its own: `git add docs/superpowers/plans/2026-10-06-row-form-fields.md && git commit -S -m "Plan the row panel's fields that edit in place" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"`.
- The diffs are against the tree as the task before left it. Save a block to a file and `git apply` it, or make the change by hand. Each task has the tests' block first, which applies on its own, then the code's. A line number in a hunk header is where the draft had it and may have moved. After a block is applied, `~/.cargo/bin/cargo fmt --all --check` must still pass: the blocks are formatted.
- House rules that bite here: no em dashes anywhere; comments explain why, in the surrounding code's voice; a view pushes `Action`s and never changes application state, apart from the text a field is editing; views draw text only through `TextRole`s; keyboard focus is drawn only by `src/ui/focus.rs` (a widget says its form with `focus::hint`); what a user typed never reaches a log (`Edits` and `EditStart` print without it); do not weaken a lint or delete a test to get green. Task 5 changes three existing tests of the panel's footer: it says which and why.
- No design or pixel conformance in any test. The design canvas draws the form on one artboard (macOS, "Editing a row") and gives Omarchy only a key (`e edit`); the spec says where this departs from it. Compare by eye, with the scenes of task 7.
- Commits are signed, one per task, after its checks pass. If signing fails ("agent refused operation": 1Password is locked), do not commit unsigned: stage the task, tell the user, and go on once they have unlocked it. Subjects are plain sentences, each ending with:

      Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>

## What was decided

With the user, before the spec (each is in the spec's "Decisions", with what was rejected):

1. **No mode.** The panel is editable whenever its row is. The artboard's Edit button stays, as one more way to start.
2. **A field at rest reads as today.** An editable value shows a field's outline under the pointer, and becomes the text field when it is edited.
3. **A double-click or a pencil starts an edit**; a single click still selects text and folds a tree.
4. **Enter commits and stays; Tab walks** to the next editable field. The walk is step 3: in this step Tab commits as Enter does.
5. **On Omarchy `e` edits the cursor's cell in the panel.** The grid's cell cursor stays the only cursor.
6. **A `NULL` checkbox** shown while the pointer is near. Step 3.
7. **A tall value is edited in the panel**, in a field that grows. Step 2: in this step its edit opens the grid's popover at its cell.
8. **The footer's Edit works**; Duplicate and Delete wait for slice 5.

With the user, on the written spec: the grid scrolls once to the cell a panel edit selects; `Mod+I` works on Omarchy too; closing the panel closes its editor with the text kept.

Made while drafting, where the spec is silent or the code had to choose. Each is small, and each is the user's to overrule in review:

9. **`Action::EditField`, not a place on `Action::EditCell`.** The spec first gave `EditCell` a `place`; that would have touched the fifty-odd places that build one, most of them tests, to add a field that is always the grid's there. `EditField { tab, id, cell }` is `EditCell` from the value, drawn in the panel. The spec is updated in task 7.
10. **`Edits::why_place` beside `Edits::why`**, not a third member of its tuple, for the same reason: a dozen tests read `why` as a pair.
11. **`Action::LeaveEdit` names its cell and its place**, as the spec says. The eleven places in tests and the one scene that sent it by hand send `crate::testing::leave_edit(&app, tab, id)` instead, which names the open editor: what a view would send.
12. **The panel takes its editor out of the tab while it draws** (`row_form::take_editor`, `put_editor`). The panel reads the workspace through a shared borrow for the whole of its drawing, and the field needs the editor's text mutably. Nothing else reads the tab in between, and it is put back unchanged but for its text and `focus`.
13. **`Mod+I`, `e` and Edit go by the selected cell**, as the spec's `EditRow` does. With a caret in one value of the panel they can therefore open another field (the selected cell's, or the row's first editable one). Enter and F2 are the keys of the value the caret is in.
14. **A locked value takes a double-click too**, and Enter and F2 with a caret in it, and answers with its reason under the value. Without that, the reason would be reachable in the panel only through the lock's tooltip.
15. **The lock is named "Locked: <reason>"** for a screen reader, and shows the bare reason under the pointer. The bare reason as its name would have made every test that asks whether a reason "is said" true before anything was asked.
16. **The reason under a field is in the secondary text colour**, in the caption's role: it answers a question, it is no error.
17. **A failing field's border is red at once.** The focus ring (and its red form) shows only once a key was pressed, as everywhere in the app; a text can fail before that, so the panel's field also draws a 1 pt border in the danger colour itself.
18. **The panel's field is the look's own text field** (`widgets::field`, 30 pt, 26 on Omarchy), on the fill the look gives a field. The artboard's amber fill is a pending field's, which is step 3's.
19. **The footer's note on a row that can be edited** is the chord, "⌘I edit" ("Ctrl+I edit" on Windows), as the artboard's first hint. Omarchy's cells name their keys, so its note is empty there.
20. **Omarchy's head hint is two hints 12 pt apart**, `[ ] prev/next` and `e edit`, without the artboard's dot between them: the app's key hints are separated by space wherever several stand in a row.
21. **Under a prompt about pending changes `ToggleRowPanel` is dropped**, with `EditField` and `EditRow`: closing the panel would put the text of its editor into the set the prompt asks about.
22. **The tall field's stand-in.** Until step 2 a tall value's pencil (and Alt+Enter in the panel's field) moves the edit to the grid's popover, and the panel then shows "Editing in the grid…" for that field, since that is where its editor is.

## What this run leaves

- For step 2: the tall field in the panel, and Alt+Enter making a field tall.
- For step 3: Tab and Shift+Tab walking the fields; the `NULL` checkbox; `revert`; `Mod+Backspace` and `Mod+Z` on a field; the header's "2 unsaved changes"; what a field says of a cell to fix or failed. Until then a cell left to fix reads in the panel as any pending field: its cell in the grid and the pending bar say what it fails.
- The header's Add row still says "Editing arrives in a later version". It is slice 5's, and says what it said while the grid alone was edited.
- Edit pressed while the grid's popover is open on the selected cell's tall value closes the popover, its text kept: the click takes the keyboard from it, and a tall value has no editor in the panel to open instead until step 2.
- A double-click on a locked value, which says why, also moves the grid's selection to that cell: the reducer's refusal selects the cell it was asked about, as it does in the grid.

## File map

| File | What changes |
|---|---|
| `src/edit.rs` | `Table::row_lock`, cut out of `Table::lock`; `EditorPlace`, `Editor::place`, `Edits::why_place`; tests |
| `src/model.rs` | `Action::{EditField, EditRow}`; `Action::LeaveEdit` names its cell and place; `ObjectTab::focus_field` |
| `src/app.rs` | the arms for the new actions; `CommitEdit`, `CancelEdit`, `LeaveEdit`, `EditorBreak` and `ToggleRowPanel` learn the place; reducer tests |
| `src/app/editing.rs` | `edit_cell` takes a place; `edit_row`, `panel_field`, `back_to_field`; three more actions dropped under a prompt |
| `src/ui/row_form.rs` | new: `Part`, `Form`, `take_editor`, `put_editor`, `row_lock`, the "Editing in the grid…" box |
| `src/ui/cell_editor.rs` | `in_panel`, the panel's one-line field; `block_cursor` shared with the cell's field |
| `src/ui/row_panel.rs` | the pencil, the outline, the double-click, Enter and F2, the editor in a value's place, the lock and its reason, the footer, Omarchy's head hint |
| `src/ui/data_view.rs` | the grid draws only its own editor; `editor_target` takes the cell and is shared |
| `src/ui/keys.rs` | `Mod+I`, `e`, their rows in the shortcuts |
| `src/ui/workspace.rs` | the mode line leaves out "tab next cell" for a panel's editor |
| `src/testing.rs` | `leave_edit` |
| `src/ui/mod.rs` | headless tests of all of it |
| `src/shots.rs` | two scenes for review |
| `docs/superpowers/specs/*.md`, `README.md` | what the panel does now |

---

### Task 1: A row's lock, apart from a cell's own

**Files:**
- Modify: `src/edit.rs` (`Table::lock`, a new `Table::row_lock`, and its tests module)

`Table::lock` answers why one cell cannot be edited. It already asks in an order: what holds for the whole table, what holds for a while (a save, a fetch), what the row's own key says, and only then what the column says. The panel needs the first three without the last: under one of them no field of the row can be edited, and the footer says why once; otherwise each field answers for itself. So the function is cut where that order turns. No cell's answer changes but one that no view asks for: a column the row does not hold, in a row that is gone or whose key cannot be read, now answers the row's reason instead of `NoSuchCell`.

- [ ] **Step 1: Write the failing test**

Apply this block. It holds the task's test, and applies on the tree as it is.

````diff
diff --git a/src/edit.rs b/src/edit.rs
index 3aa630c..5fb43c5 100644
--- a/src/edit.rs
+++ b/src/edit.rs
@@ -1403,6 +1403,92 @@ mod tests {
         assert_eq!(ok().lock(at(0, 9)), Some(Lock::NoSuchCell));
     }
 
+    #[test]
+    fn a_rows_lock_is_every_reason_before_the_cells_own() {
+        let (structure, page) = (structure(), page(rows()));
+        let ok = || table(Some(&structure), &page);
+        // A row whose cells answer for themselves has none, whatever its
+        // cells say: the key column of it is locked, and the row is not.
+        assert_eq!(ok().row_lock(0), None);
+        assert_eq!(ok().lock(at(0, 0)), Some(Lock::KeyColumn));
+        // The table's reasons, and the ones that hold for a while.
+        let read_only = Table {
+            access: Access::ReadOnly,
+            ..ok()
+        };
+        assert_eq!(read_only.row_lock(0), Some(Lock::ReadOnly));
+        let view = Table {
+            kind: ObjectKind::View,
+            ..ok()
+        };
+        assert_eq!(view.row_lock(0), Some(Lock::NotATable));
+        assert_eq!(table(None, &page).row_lock(0), Some(Lock::StructureLoading));
+        let saving = Table {
+            saving: true,
+            ..ok()
+        };
+        assert_eq!(saving.row_lock(0), Some(Lock::Saving));
+        let refreshing = Table {
+            refreshing: true,
+            ..ok()
+        };
+        assert_eq!(refreshing.row_lock(0), Some(Lock::Refreshing));
+        // A table with no key, and one whose key a save could not match.
+        let keyless = Structure {
+            primary_key: Vec::new(),
+            ..structure.clone()
+        };
+        assert_eq!(table(Some(&keyless), &page).row_lock(0), Some(Lock::NoKey));
+        let mut stamped = structure.clone();
+        stamped.columns[0].type_name = "timestamp".into();
+        let mysql = Table {
+            dialect: Dialect::MySql,
+            ..table(Some(&stamped), &page)
+        };
+        assert_eq!(mysql.row_lock(0), Some(Lock::KeyType));
+        // The row's own: it is not on the page, it is gone, its key is
+        // NULL or was not read exactly.
+        assert_eq!(ok().row_lock(9), Some(Lock::NoSuchCell));
+        let gone = BTreeSet::from([1]);
+        let with_gone = Table {
+            gone: &gone,
+            ..ok()
+        };
+        assert_eq!(with_gone.row_lock(1), Some(Lock::Gone));
+        assert_eq!(with_gone.row_lock(0), None);
+        let nulls = self::page(vec![vec![Value::Null, text("a"), Value::Null]]);
+        assert_eq!(
+            table(Some(&structure), &nulls).row_lock(0),
+            Some(Lock::KeyIsNull)
+        );
+        let inexact = self::page(vec![vec![text("caf\u{FFFD}"), text("a"), Value::Null]]);
+        assert_eq!(
+            table(Some(&structure), &inexact).row_lock(0),
+            Some(Lock::KeyInexact)
+        );
+        // Only SQLite reads a key's text that way: the others hold it.
+        for dialect in [Dialect::Postgres, Dialect::MySql] {
+            let exact = Table {
+                dialect,
+                ..table(Some(&structure), &inexact)
+            };
+            assert_eq!(exact.row_lock(0), None, "{dialect:?}");
+        }
+        // A row's reason comes before a cell's: a column the row does not
+        // hold answers for the row first.
+        assert_eq!(with_gone.lock(at(1, 9)), Some(Lock::Gone));
+        assert_eq!(with_gone.lock(at(0, 9)), Some(Lock::NoSuchCell));
+        // And every cell of a row answers its row's reason, where it has
+        // one, and never another.
+        for row in 0..2 {
+            for col in 0..3 {
+                if let Some(lock) = with_gone.row_lock(row) {
+                    assert_eq!(with_gone.lock(at(row, col)), Some(lock));
+                }
+            }
+        }
+    }
+
     #[test]
     fn a_row_whose_key_is_null_is_locked() {
         let structure = structure();
````

- [ ] **Step 2: Run it to see it fail**

Run: `~/.cargo/bin/cargo test --locked --lib -- a_rows_lock_is_every_reason`
Expected: it does not compile: `error[E0599]: no method named `row_lock` found for struct `Table` in the current scope`.

- [ ] **Step 3: Cut `row_lock` out of `lock`**

````diff
diff --git a/src/edit.rs b/src/edit.rs
index 5fb43c5..0b15f80 100644
--- a/src/edit.rs
+++ b/src/edit.rs
@@ -129,10 +129,11 @@ impl Table<'_> {
             .map(|column| column_class(self.dialect, &column.type_name))
     }
 
-    /// Why `cell` cannot be edited, or `None` when it can. The reasons that
-    /// hold for the whole table come first, so every cell of such a table
-    /// says the same.
-    pub fn lock(&self, cell: CellPos) -> Option<Lock> {
+    /// Why no cell of the page's row `row` can be edited, or `None` when
+    /// each of its cells answers for itself: what holds for the whole
+    /// table, what holds for a while, and what the row's own key says. The
+    /// row panel says such a reason once, for the row.
+    pub fn row_lock(&self, row: usize) -> Option<Lock> {
         if let Some(lock) = self.never() {
             return Some(lock);
         }
@@ -145,19 +146,16 @@ impl Table<'_> {
         if self.refreshing {
             return Some(Lock::Refreshing);
         }
-        let Some(row) = self.page.rows.get(cell.row) else {
-            return Some(Lock::NoSuchCell);
-        };
-        let Some(value) = row.get(cell.col) else {
+        let Some(values) = self.page.rows.get(row) else {
             return Some(Lock::NoSuchCell);
         };
         // Before anything its values say: they are of a row that is no
         // longer there.
-        if self.gone.contains(&cell.row) {
+        if self.gone.contains(&row) {
             return Some(Lock::Gone);
         }
         // A row narrower than the page has no key to read.
-        let held = |col: &usize| row.get(*col);
+        let held = |col: &usize| values.get(*col);
         if key.iter().any(|col| held(col).is_none_or(Value::is_null)) {
             return Some(Lock::KeyIsNull);
         }
@@ -170,13 +168,28 @@ impl Table<'_> {
         if inexact {
             return Some(Lock::KeyInexact);
         }
+        None
+    }
+
+    /// Why `cell` cannot be edited, or `None` when it can. The reasons that
+    /// hold for the whole table come first, so every cell of such a table
+    /// says the same, then the row's (see [`Table::row_lock`]), then the
+    /// cell's own.
+    pub fn lock(&self, cell: CellPos) -> Option<Lock> {
+        if let Some(lock) = self.row_lock(cell.row) {
+            return Some(lock);
+        }
+        let row = self.page.rows.get(cell.row);
+        let Some(value) = row.and_then(|row| row.get(cell.col)) else {
+            return Some(Lock::NoSuchCell);
+        };
         let Some(column) = self.column(cell.col) else {
             return Some(Lock::UnknownColumn);
         };
         if column.generated {
             return Some(Lock::Generated);
         }
-        if key.contains(&cell.col) {
+        if self.key().is_some_and(|key| key.contains(&cell.col)) {
             return Some(Lock::KeyColumn);
         }
         if column_class(self.dialect, &column.type_name) == ColumnClass::Binary
````

- [ ] **Step 4: Run the tests**

Run: `~/.cargo/bin/cargo test --locked --lib -- edit::tests`
Expected: PASS, the new test and every older test of `lock` (`what_cannot_be_edited_says_why`, `a_row_whose_key_is_null_is_locked`, `a_key_the_save_would_refuse_locks_the_table_or_the_row` and the rest), unchanged.

- [ ] **Step 5: Run the four checks, then commit**

```bash
git add src/edit.rs
git commit -S -m "Tell a row's lock apart from a cell's own" -m "What holds for the whole table, for a while, or for the row's key locks every cell of a row alike. The row panel says such a reason once, for the row, and asks each field only for its own." -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: The editor has a place

**Files:**
- Modify: `src/edit.rs` (`EditorPlace`, `Editor::place`, `Edits::why_place`)
- Modify: `src/model.rs` (`Action::EditField`, `Action::LeaveEdit`, `ObjectTab::focus_field`)
- Modify: `src/app.rs` (the reducer's arms, and its `editing` tests)
- Modify: `src/app/editing.rs` (`edit_cell`, `panel_field`, `back_to_field`, `dropped_under_a_prompt`)
- Modify: `src/ui/data_view.rs` (the grid's editor, `LeaveEdit`, the reason at a cell)
- Modify: `src/testing.rs` (`leave_edit`), `src/shots.rs`, and the tests that sent `LeaveEdit` by hand

The model's half of the whole step. After it an editor can be open "in the panel", though no view draws one there yet and nothing on screen asks for one: the app behaves as before.

What changes, and why each:

- **`Editor::place`.** `EditCell` opens the editor in the grid, as it did. `EditField { tab, id, cell }` opens it in the panel, from the value (`EditStart::Value`). `close_editor`, which makes a pending cell of the text, does not ask where the editor is: that is the whole point.
- **A tall value goes to the grid.** `edit::opens_large` decides, as for a cell. The panel has no tall editor until step 2, so an edit asked for there opens the popover at the cell, and `EditorBreak` (Alt+Enter) on a panel's editor moves it there too.
- **After a commit or a cancel in the panel, nothing moves.** A cell's field commits with `Advance::Down` on Enter; for an editor in the panel the reducer ignores the advance and notes the field the keyboard goes back to (`ObjectTab::focus_field`, which the panel takes in task 4). Not in the terminal look, where the keys are the grid's again, and not after an edit that was left. It is forgotten when the selection moves or the panel closes: the field it named was a field of the row the selection left.
- **`LeaveEdit` names the editor it is about.** The row panel is drawn before the grid. A click on one of the panel's controls takes the keyboard from the grid's field and asks for a new editor in the same frame; the grid's field then says it was left, and that arrives after the new editor opened. Unnamed, it would close the new one.
- **Closing the panel closes the editor it draws,** with its text kept, in every table tab of the workspace: the panel is the workspace's, and an editor waits in a tab that is not in front. Under a prompt about pending changes `ToggleRowPanel` is therefore dropped with the other editing actions.
- **`why_place`.** A refused edit is said where it was asked for. The grid shows its note only for an edit asked for in the grid.

- [ ] **Step 1: Write the failing tests**

Apply this block. Besides the new tests in `src/app.rs`, it adds `crate::testing::leave_edit` and sends it wherever a test sent `Action::LeaveEdit { tab, id }` by hand (eleven places in four files): the action is about to name its cell and place, and the helper names the open editor's, which is what a view sends.

````diff
diff --git a/src/app.rs b/src/app.rs
index f05e6a6..152302f 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -10269,7 +10269,7 @@ mod tests {
     /// the guard that keeps a page with pending changes.
     mod editing {
         use super::*;
-        use crate::edit::{Answer, Conflicting, Lock, Problem, State};
+        use crate::edit::{Answer, Conflicting, EditorPlace, Lock, Problem, State};
         use crate::model::{Advance, EditStart};
         use tabletist_db::{Conflict, NewValue, Value, WriteOutcome};
 
@@ -10503,7 +10503,9 @@ mod tests {
                 cell: at(1, 2),
                 start: EditStart::Value,
             });
-            harness.app.apply(Action::LeaveEdit { tab, id });
+            harness
+                .app
+                .apply(crate::testing::leave_edit(&harness.app, tab, id));
             assert!(!object(&harness, tab, id).edits.holds());
             // A cell made NULL stays NULL when its editor is opened and left.
             harness.app.apply(Action::SelectCell {
@@ -10743,7 +10745,9 @@ mod tests {
             let mut harness = Harness::new();
             let (tab, id) = harness.editable();
             type_into(&mut harness, tab, id, at(1, 2), "{oops");
-            harness.app.apply(Action::LeaveEdit { tab, id });
+            harness
+                .app
+                .apply(crate::testing::leave_edit(&harness.app, tab, id));
             let before = harness.app.backend.sent.len();
             harness.app.apply(Action::WriteEdits { tab, id });
             assert!(write_since(&harness, before).is_none());
@@ -11116,7 +11120,9 @@ mod tests {
             let mut harness = Harness::new();
             let (tab, id) = harness.editable();
             type_into(&mut harness, tab, id, at(1, 2), "{oops");
-            harness.app.apply(Action::LeaveEdit { tab, id });
+            harness
+                .app
+                .apply(crate::testing::leave_edit(&harness.app, tab, id));
             harness.app.apply(Action::CloseTab { tab, id });
             assert_eq!(leave_prompt(&harness), Some(false));
             harness.app.apply(Action::LeaveStay);
@@ -11725,7 +11731,9 @@ mod tests {
             assert_eq!(reviewed(&mut harness, tab, id).unwrap(), two);
             // A cell to fix blocks its row: the row has no statement.
             type_into(&mut harness, tab, id, at(3, 2), "{oops");
-            harness.app.apply(Action::LeaveEdit { tab, id });
+            harness
+                .app
+                .apply(crate::testing::leave_edit(&harness.app, tab, id));
             assert_eq!(reviewed(&mut harness, tab, id).unwrap(), one);
             let review = object(&harness, tab, id).edits.review.as_ref().unwrap();
             assert_eq!(
@@ -12215,7 +12223,9 @@ mod tests {
                 cell,
                 start: EditStart::Replace(text.into()),
             });
-            harness.app.apply(Action::LeaveEdit { tab, id });
+            harness
+                .app
+                .apply(crate::testing::leave_edit(&harness.app, tab, id));
         }
 
         fn written(email: &str) -> Result<WriteOutcome, tabletist_db::Error> {
@@ -12330,7 +12340,9 @@ mod tests {
             }
             // So none is left to add a cell to a save that is running.
             harness.app.apply(Action::ConfirmWrite);
-            harness.app.apply(Action::LeaveEdit { tab, id });
+            harness
+                .app
+                .apply(crate::testing::leave_edit(&harness.app, tab, id));
             let saving = object(&harness, tab, id);
             assert!(saving.edits.saving.is_some());
             assert_eq!(saving.edits.cells.len(), 1);
@@ -12366,7 +12378,7 @@ mod tests {
                     id,
                     then: Advance::Down,
                 },
-                Action::LeaveEdit { tab, id },
+                crate::testing::leave_edit(&harness.app, tab, id),
                 Action::CancelEdit { tab, id },
             ] {
                 harness.app.apply(closes);
@@ -12531,6 +12543,7 @@ mod tests {
                 ("an editor", |object| {
                     object.edits.editor = Some(crate::edit::Editor {
                         cell: at(3, 1),
+                        place: crate::edit::EditorPlace::Grid,
                         text: "dan@example.com".into(),
                         large: false,
                         focus: false,
@@ -14186,5 +14199,347 @@ mod tests {
             assert_eq!(after_an_answer(&harness), None);
             assert_eq!(writes(&harness), 1);
         }
+
+        /// The tab's open editor: its cell, where it is drawn and whether
+        /// it is the large one.
+        fn editor(
+            harness: &Harness,
+            tab: ConnTabId,
+            id: TabId,
+        ) -> Option<(CellPos, EditorPlace, bool)> {
+            let editor = object(harness, tab, id).edits.editor.as_ref()?;
+            Some((editor.cell, editor.place, editor.large))
+        }
+
+        /// Sets the open editor's text as its field would.
+        fn type_text(harness: &mut Harness, tab: ConnTabId, id: TabId, text: &str) {
+            let object = harness
+                .app
+                .workspace_mut(tab)
+                .unwrap()
+                .object_tab_mut(id)
+                .unwrap();
+            object.edits.editor.as_mut().expect("an editor").text = text.to_owned();
+            harness.app.apply(Action::EditorTyped { tab, id });
+        }
+
+        #[test]
+        fn an_edit_asked_for_in_the_row_panel_opens_there_and_makes_the_same_pending_cell() {
+            let mut harness = Harness::new();
+            harness.set_look(crate::theme::Look::macos());
+            let (tab, id) = harness.editable();
+            // In the grid, as ever.
+            harness.app.apply(Action::EditCell {
+                tab,
+                id,
+                cell: at(1, 1),
+                start: EditStart::Value,
+            });
+            assert_eq!(
+                editor(&harness, tab, id),
+                Some((at(1, 1), EditorPlace::Grid, false))
+            );
+            harness.app.apply(Action::CancelEdit { tab, id });
+            assert_eq!(object(&harness, tab, id).focus_field, None);
+            // In the panel: the same editor, on the same cell, which is
+            // the selected one, in a tab that is no preview any more.
+            harness.app.apply(Action::EditField {
+                tab,
+                id,
+                cell: at(2, 1),
+            });
+            assert_eq!(
+                editor(&harness, tab, id),
+                Some((at(2, 1), EditorPlace::Panel, false))
+            );
+            let tab_now = object(&harness, tab, id);
+            assert_eq!(tab_now.selection, Some(at(2, 1)));
+            assert!(tab_now.pinned);
+            assert_eq!(
+                tab_now.edits.editor.as_ref().unwrap().text,
+                "user3@example.com"
+            );
+            // Committed as a cell's field commits (Enter there moves down):
+            // the text is the pending cell an edit in the grid makes, the
+            // selection stays on the field, and the keyboard goes back to
+            // it.
+            type_text(&mut harness, tab, id, "cy@example.com");
+            harness.app.apply(Action::CommitEdit {
+                tab,
+                id,
+                then: Advance::Down,
+            });
+            let tab_now = object(&harness, tab, id);
+            assert!(tab_now.edits.editor.is_none());
+            assert_eq!(
+                tab_now.edits.cells.get(&(2, 1)).map(|cell| &cell.new),
+                Some(&NewValue::Text("cy@example.com".into()))
+            );
+            assert_eq!(tab_now.selection, Some(at(2, 1)));
+            assert_eq!(tab_now.focus_field, Some(1));
+            // Opened again in the panel it starts from the pending text.
+            harness.app.apply(Action::EditField {
+                tab,
+                id,
+                cell: at(2, 1),
+            });
+            let text = &object(&harness, tab, id)
+                .edits
+                .editor
+                .as_ref()
+                .unwrap()
+                .text;
+            assert_eq!(text, "cy@example.com");
+        }
+
+        #[test]
+        fn an_edit_that_ends_in_the_panel_gives_the_keyboard_back_to_its_field() {
+            for look in crate::theme::Look::ALL {
+                let mut harness = Harness::new();
+                harness.set_look(look);
+                let (tab, id) = harness.editable();
+                let field = |harness: &Harness| object(harness, tab, id).focus_field;
+                let cell = at(1, 1);
+                // Dropped, and committed: on the terminal's look the keys
+                // are the grid's again, and nothing is asked of the panel.
+                let back = (!look.terminal).then_some(1);
+                harness.app.apply(Action::EditField { tab, id, cell });
+                harness.app.apply(Action::CancelEdit { tab, id });
+                assert_eq!(field(&harness), back, "{}", look.name);
+                // The field is the selected row's: once the selection
+                // moves, by a click or a key, no field is owed the keyboard.
+                harness.app.apply(Action::SelectCell { tab, id, cell });
+                assert_eq!(field(&harness), None, "{}", look.name);
+                harness.app.apply(Action::EditField { tab, id, cell });
+                harness.app.apply(Action::CommitEdit {
+                    tab,
+                    id,
+                    then: Advance::Stay,
+                });
+                assert_eq!(field(&harness), back, "{}", look.name);
+                harness.app.apply(Action::MoveSelection {
+                    tab,
+                    id,
+                    rows: 1,
+                    cols: 0,
+                });
+                assert_eq!(field(&harness), None, "{}", look.name);
+                // Nor once the panel closes: opened again, it starts as
+                // any panel does.
+                harness.app.apply(Action::EditField { tab, id, cell });
+                harness.app.apply(Action::CancelEdit { tab, id });
+                harness.app.apply(Action::ToggleRowPanel(tab));
+                assert_eq!(field(&harness), None, "{}", look.name);
+                harness.app.apply(Action::ToggleRowPanel(tab));
+                // Left, the keyboard is where the user put it.
+                harness.app.apply(Action::EditField { tab, id, cell });
+                let left = crate::testing::leave_edit(&harness.app, tab, id);
+                harness.app.apply(left);
+                assert!(editor(&harness, tab, id).is_none());
+                assert_eq!(field(&harness), None, "{}", look.name);
+            }
+        }
+
+        #[test]
+        fn a_tall_value_asked_for_in_the_row_panel_is_edited_at_its_cell() {
+            let mut harness = Harness::new();
+            let (tab, id) = harness.editable();
+            // A document: the popover at the cell is its only editor yet.
+            harness.app.apply(Action::EditField {
+                tab,
+                id,
+                cell: at(0, 2),
+            });
+            assert_eq!(
+                editor(&harness, tab, id),
+                Some((at(0, 2), EditorPlace::Grid, true))
+            );
+            harness.app.apply(Action::CancelEdit { tab, id });
+            assert_eq!(object(&harness, tab, id).focus_field, None);
+            // A line break typed into a field of the panel moves the edit
+            // there too.
+            harness.app.apply(Action::EditField {
+                tab,
+                id,
+                cell: at(1, 1),
+            });
+            harness.app.apply(Action::EditorBreak { tab, id });
+            assert_eq!(
+                editor(&harness, tab, id),
+                Some((at(1, 1), EditorPlace::Grid, true))
+            );
+            let text = &object(&harness, tab, id)
+                .edits
+                .editor
+                .as_ref()
+                .unwrap()
+                .text;
+            assert_eq!(text, "user2@example.com\n");
+        }
+
+        #[test]
+        fn leaving_closes_only_the_editor_that_was_left() {
+            let mut harness = Harness::new();
+            let (tab, id) = harness.editable();
+            let leave = |cell, place| Action::LeaveEdit {
+                tab,
+                id,
+                cell,
+                place,
+            };
+            // The grid's field on one cell loses the keyboard to a pencil
+            // of the panel: the panel is drawn first, so its edit is asked
+            // for before the grid's field says it was left.
+            harness.app.apply(Action::EditCell {
+                tab,
+                id,
+                cell: at(1, 1),
+                start: EditStart::Value,
+            });
+            type_text(&mut harness, tab, id, "bob@example.com");
+            harness.app.apply(Action::EditField {
+                tab,
+                id,
+                cell: at(2, 1),
+            });
+            harness.app.apply(leave(at(1, 1), EditorPlace::Grid));
+            // The new editor is open, and the old one's text was kept.
+            assert_eq!(
+                editor(&harness, tab, id),
+                Some((at(2, 1), EditorPlace::Panel, false))
+            );
+            let edits = &object(&harness, tab, id).edits;
+            assert_eq!(
+                edits.cells.get(&(1, 1)).map(|cell| &cell.new),
+                Some(&NewValue::Text("bob@example.com".into()))
+            );
+            // The same cell in the other place is another editor too.
+            harness.app.apply(leave(at(2, 1), EditorPlace::Grid));
+            assert!(editor(&harness, tab, id).is_some());
+            // Its own leaving closes it.
+            harness.app.apply(leave(at(2, 1), EditorPlace::Panel));
+            assert!(editor(&harness, tab, id).is_none());
+        }
+
+        #[test]
+        fn a_refused_edit_remembers_where_it_was_asked_for() {
+            let mut harness = Harness::new();
+            let (tab, id) = harness.editable();
+            let why = |harness: &Harness| {
+                let edits = &object(harness, tab, id).edits;
+                edits.why.map(|(cell, lock)| (cell, lock, edits.why_place))
+            };
+            harness.app.apply(Action::EditField {
+                tab,
+                id,
+                cell: at(0, 0),
+            });
+            assert!(editor(&harness, tab, id).is_none());
+            assert_eq!(
+                why(&harness),
+                Some((at(0, 0), Lock::KeyColumn, EditorPlace::Panel))
+            );
+            harness.app.apply(Action::EditCell {
+                tab,
+                id,
+                cell: at(0, 0),
+                start: EditStart::Value,
+            });
+            assert_eq!(
+                why(&harness),
+                Some((at(0, 0), Lock::KeyColumn, EditorPlace::Grid))
+            );
+        }
+
+        #[test]
+        fn closing_the_row_panel_closes_the_editor_it_draws_and_keeps_the_text() {
+            let mut harness = Harness::new();
+            let (tab, id) = harness.editable();
+            let panel = |harness: &Harness| harness.app.workspace(tab).unwrap().row_panel;
+            assert!(panel(&harness));
+            // The grid's editor is not the panel's to close.
+            harness.app.apply(Action::EditCell {
+                tab,
+                id,
+                cell: at(1, 1),
+                start: EditStart::Value,
+            });
+            harness.app.apply(Action::ToggleRowPanel(tab));
+            assert!(!panel(&harness));
+            assert!(editor(&harness, tab, id).is_some());
+            harness.app.apply(Action::CancelEdit { tab, id });
+            // Opening the panel closes nothing.
+            harness.app.apply(Action::ToggleRowPanel(tab));
+            harness.app.apply(Action::EditField {
+                tab,
+                id,
+                cell: at(1, 1),
+            });
+            type_text(&mut harness, tab, id, "bob@example.com");
+            // Closed from another tab, where the editor waits: the panel
+            // is the workspace's.
+            let sql = harness.add_sql_tab(tab);
+            harness.app.apply(Action::ActivateTab { tab, id: sql });
+            assert!(editor(&harness, tab, id).is_some());
+            harness.app.apply(Action::ToggleRowPanel(tab));
+            assert!(!panel(&harness));
+            let edits = &object(&harness, tab, id).edits;
+            assert!(edits.editor.is_none());
+            assert_eq!(
+                edits.cells.get(&(1, 1)).map(|cell| &cell.new),
+                Some(&NewValue::Text("bob@example.com".into()))
+            );
+            // The Structure view shows no panel: switching to it closes
+            // the panel's editor the same way, as it closes a cell's.
+            harness.app.apply(Action::ActivateTab { tab, id });
+            harness.app.apply(Action::ToggleRowPanel(tab));
+            harness.app.apply(Action::EditField {
+                tab,
+                id,
+                cell: at(2, 1),
+            });
+            type_text(&mut harness, tab, id, "cy@example.com");
+            harness.app.apply(Action::SetView {
+                tab,
+                object_tab: id,
+                view: crate::model::ObjectView::Structure,
+            });
+            let edits = &object(&harness, tab, id).edits;
+            assert!(edits.editor.is_none());
+            assert_eq!(
+                edits.cells.get(&(2, 1)).map(|cell| &cell.new),
+                Some(&NewValue::Text("cy@example.com".into()))
+            );
+        }
+
+        #[test]
+        fn the_row_panels_actions_are_dropped_under_a_prompt_about_the_changes() {
+            let mut harness = Harness::new();
+            let (tab, id) = harness.editable();
+            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
+            harness.app.apply(Action::EditField {
+                tab,
+                id,
+                cell: at(2, 1),
+            });
+            type_text(&mut harness, tab, id, "cy@example.com");
+            harness.app.apply(Action::CloseTab { tab, id });
+            assert!(leave_prompt(&harness).is_some());
+            // The panel stays open, and its editor with it: closing it
+            // would put the text into the set the prompt asks about.
+            harness.app.apply(Action::ToggleRowPanel(tab));
+            assert!(harness.app.workspace(tab).unwrap().row_panel);
+            // No other field opens either.
+            harness.app.apply(Action::EditField {
+                tab,
+                id,
+                cell: at(3, 1),
+            });
+            assert_eq!(
+                editor(&harness, tab, id),
+                Some((at(2, 1), EditorPlace::Panel, false))
+            );
+            assert_eq!(object(&harness, tab, id).edits.cells.len(), 1);
+        }
     }
 }
diff --git a/src/edit.rs b/src/edit.rs
index 0b15f80..7aec160 100644
--- a/src/edit.rs
+++ b/src/edit.rs
@@ -2144,6 +2144,7 @@ mod tests {
         );
         edits.editor = Some(Editor {
             cell: at(0, 2),
+            place: EditorPlace::Grid,
             text: "another secret".into(),
             large: false,
             focus: false,
diff --git a/src/testing.rs b/src/testing.rs
index 4c8a500..b036c75 100644
--- a/src/testing.rs
+++ b/src/testing.rs
@@ -568,6 +568,28 @@ impl Harness {
 
 use tabletist_db::{ColumnMeta, RowPage, Value, ValueKind};
 
+/// What a view sends when the tab's open editor loses the keyboard: the
+/// action names the editor it is about, by its cell and its place.
+pub fn leave_edit(app: &App, tab: ConnTabId, id: TabId) -> Action {
+    let editor = app
+        .workspace(tab)
+        .and_then(|workspace| workspace.object_tab(id))
+        .and_then(|object| object.edits.editor.as_ref());
+    let (cell, place) = editor.map_or(
+        (
+            crate::model::CellPos { row: 0, col: 0 },
+            crate::edit::EditorPlace::Grid,
+        ),
+        |editor| (editor.cell, editor.place),
+    );
+    Action::LeaveEdit {
+        tab,
+        id,
+        cell,
+        place,
+    }
+}
+
 /// A page shaped like the fixture's users table.
 pub fn page(rows: usize, has_more: bool) -> RowPage {
     RowPage {
diff --git a/src/ui/conflict_prompt.rs b/src/ui/conflict_prompt.rs
index 7257e3c..99801ee 100644
--- a/src/ui/conflict_prompt.rs
+++ b/src/ui/conflict_prompt.rs
@@ -1020,7 +1020,9 @@ mod tests {
             cell,
             start: EditStart::Replace(text.into()),
         });
-        harness.app.apply(Action::LeaveEdit { tab, id });
+        harness
+            .app
+            .apply(crate::testing::leave_edit(&harness.app, tab, id));
     }
 
     /// A writable table `users` with `structure` and `page`, in `look` and
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
index 9d18eb5..f521272 100644
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -15034,7 +15034,9 @@ mod tests {
             harness.app.apply(Action::CommitEdit { tab, id, then });
             assert!(edits(&harness, tab, id).editor.is_some(), "large: {large}");
             assert!(edits(&harness, tab, id).cells.is_empty(), "large: {large}");
-            harness.app.apply(Action::LeaveEdit { tab, id });
+            harness
+                .app
+                .apply(crate::testing::leave_edit(&harness.app, tab, id));
             let pending = edits(&harness, tab, id).cells.get(&at).unwrap();
             assert_eq!(
                 pending.state,
@@ -15319,7 +15321,9 @@ mod tests {
             cell,
             start,
         });
-        harness.app.apply(Action::LeaveEdit { tab, id });
+        harness
+            .app
+            .apply(crate::testing::leave_edit(&harness.app, tab, id));
     }
 
     /// The row `id 2` of the fixture's page as a save reads it back.
diff --git a/src/ui/write_prompts.rs b/src/ui/write_prompts.rs
index 3b14567..e2799c0 100644
--- a/src/ui/write_prompts.rs
+++ b/src/ui/write_prompts.rs
@@ -1007,7 +1007,9 @@ mod tests {
             cell: CellPos { row, col },
             start: EditStart::Replace(text.into()),
         });
-        harness.app.apply(Action::LeaveEdit { tab, id });
+        harness
+            .app
+            .apply(crate::testing::leave_edit(&harness.app, tab, id));
     }
 
     fn writes(harness: &Harness) -> usize {
````

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked --lib -- editing::`
Expected: it does not compile. The first errors name what the task adds: `error[E0432]: unresolved import `crate::edit::EditorPlace``, `error[E0559]: variant `Action::LeaveEdit` has no field named `cell``, `error[E0599]: no variant named `EditField` found for enum `Action``.

- [ ] **Step 3: Give the editor its place**

The block also changes `src/shots.rs`, where one scene sent `LeaveEdit` by hand.

````diff
diff --git a/src/app.rs b/src/app.rs
index 152302f..0ccd0da 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -11,13 +11,14 @@ use tabletist_db::{
 use crate::backend::{CancelReason, Command, Event, Opened, RequestId, StateFile};
 use crate::completion::Need;
 use crate::connections::{PasswordMode, SavedConnection};
+use crate::edit::EditorPlace;
 use crate::i18n::Locale;
 use crate::model::{Action, ConnTab, ConnTabContent, ConnTabId, PickerState};
 use crate::model::{
-    Advance, CellPos, Completion, ConnectionForm, Dialog, Fetch, FilterBar, FilterRow, Held,
-    HostKeyPrompt, LeavePrompt, ObjectTab, ObjectView, Pane, PasswordPrompt, PickTarget, QuickOpen,
-    ResultPane, RunMode, SecretKind, SessionStatus, SqlTab, Tab, TabId, TestState, TextPrint, Tree,
-    TreeKey, TreeNode, Wanted, Workspace,
+    Advance, CellPos, Completion, ConnectionForm, Dialog, EditStart, Fetch, FilterBar, FilterRow,
+    Held, HostKeyPrompt, LeavePrompt, ObjectTab, ObjectView, Pane, PasswordPrompt, PickTarget,
+    QuickOpen, ResultPane, RunMode, SecretKind, SessionStatus, SqlTab, Tab, TabId, TestState,
+    TextPrint, Tree, TreeKey, TreeNode, Wanted, Workspace,
 };
 use crate::paths::AppDirs;
 use crate::secrets::{SecretString, password_account, ssh_account};
@@ -784,6 +785,9 @@ impl App {
                 self.close_editor(tab, id, true);
                 if let Some(object) = self.object_tab_mut(tab, id) {
                     object.edits.why = None;
+                    // The field that was to get the keyboard back was the
+                    // row's the selection leaves.
+                    object.focus_field = None;
                     object.selection = Some(cell);
                     object.pinned = true;
                 } else if let Some(sql) = self.sql_tab_mut(tab, id) {
@@ -803,7 +807,10 @@ impl App {
                 id,
                 cell,
                 start,
-            } => self.edit_cell(tab, id, cell, start),
+            } => self.edit_cell(tab, id, cell, start, EditorPlace::Grid),
+            Action::EditField { tab, id, cell } => {
+                self.edit_cell(tab, id, cell, EditStart::Value, EditorPlace::Panel);
+            }
             Action::EditorTyped { tab, id } => {
                 let problem = self.editor_problem(tab, id);
                 if let Some(editor) = self.editor_mut(tab, id) {
@@ -812,7 +819,14 @@ impl App {
                 }
             }
             Action::CommitEdit { tab, id, then } => {
+                let field = self.panel_field(tab, id);
                 if self.close_editor(tab, id, false) {
+                    // In the panel nothing moves: the keyboard goes back
+                    // to the field that was edited.
+                    if let Some(col) = field {
+                        self.back_to_field(tab, id, col);
+                        return;
+                    }
                     let (rows, cols) = match then {
                         Advance::Stay => (0, 0),
                         Advance::Down => (1, 0),
@@ -829,21 +843,39 @@ impl App {
                     }
                 }
             }
-            Action::LeaveEdit { tab, id } => {
-                self.close_editor(tab, id, true);
+            Action::LeaveEdit {
+                tab,
+                id,
+                cell,
+                place,
+            } => {
+                // Only the editor that was left: another may have opened
+                // since, in the frame that took the keyboard from it.
+                let left = self
+                    .editor_mut(tab, id)
+                    .is_some_and(|editor| editor.cell == cell && editor.place == place);
+                if left {
+                    self.close_editor(tab, id, true);
+                }
             }
             Action::CancelEdit { tab, id } => {
+                let field = self.panel_field(tab, id);
                 if let Some(object) = self.object_tab_mut(tab, id) {
                     object.edits.editor = None;
                 }
+                if let Some(col) = field {
+                    self.back_to_field(tab, id, col);
+                }
             }
             Action::EditorBreak { tab, id } => {
                 if let Some(editor) = self.editor_mut(tab, id) {
                     // At the end of the text, where the cursor of a field
                     // that just opened is. A break elsewhere is typed in
-                    // the large editor.
+                    // the large editor, which is the grid's: an edit begun
+                    // in the row panel goes on at its cell.
                     editor.text.push('\n');
                     editor.large = true;
+                    editor.place = EditorPlace::Grid;
                     editor.focus = true;
                     editor.touched = true;
                 }
@@ -939,6 +971,7 @@ impl App {
                 self.close_editor(tab, id, true);
                 if let Some(object) = self.object_tab_mut(tab, id) {
                     object.edits.why = None;
+                    object.focus_field = None;
                 }
                 if let Some(sql) = self.sql_tab_mut(tab, id) {
                     let (height, width) = sql.dims();
@@ -969,7 +1002,33 @@ impl App {
                 }
             }
             Action::ToggleRowPanel(tab) => {
+                // An editor the panel draws goes with it, its text kept:
+                // closed, the panel would leave it open where nothing
+                // shows it. Of every table's tab: the panel is the
+                // workspace's, and an editor waits in a tab behind another.
+                let editing: Vec<TabId> = self
+                    .workspace(tab)
+                    .filter(|workspace| workspace.row_panel)
+                    .map(|workspace| {
+                        workspace
+                            .object_tabs()
+                            .filter(|object| {
+                                let editor = object.edits.editor.as_ref();
+                                editor.is_some_and(|editor| editor.place == EditorPlace::Panel)
+                            })
+                            .map(|object| object.id)
+                            .collect()
+                    })
+                    .unwrap_or_default();
+                for id in editing {
+                    self.close_editor(tab, id, true);
+                }
                 if let Some(workspace) = self.workspace_mut(tab) {
+                    // A field that was to get the keyboard back is none of
+                    // the panel that opens next.
+                    for object in workspace.object_tabs_mut() {
+                        object.focus_field = None;
+                    }
                     workspace.row_panel = !workspace.row_panel;
                 }
             }
diff --git a/src/app/editing.rs b/src/app/editing.rs
index a367946..6b45ca3 100644
--- a/src/app/editing.rs
+++ b/src/app/editing.rs
@@ -7,8 +7,9 @@ use tabletist_db::{Access, ChangeSet, ColumnClass, Error, NewValue, WriteOutcome
 use super::App;
 use crate::backend::{Command, RequestId, SessionId};
 use crate::edit::{
-    Answer, Editor, Lock, Note, Pending, Problem, Saved, Saving, State, Table, change_set, check,
-    conflicting, is_change, opens_large, same_changes, settled, shown_lines, start_text,
+    Answer, Editor, EditorPlace, Lock, Note, Pending, Problem, Saved, Saving, State, Table,
+    change_set, check, conflicting, is_change, opens_large, same_changes, settled, shown_lines,
+    start_text,
 };
 use crate::model::{
     Action, CellPos, ConflictPrompt, ConnTabId, Dialog, EditStart, Held, LeavePrompt, ObjectTab,
@@ -30,6 +31,7 @@ pub(super) fn dropped_under_a_prompt(action: &Action) -> bool {
     matches!(
         action,
         Action::EditCell { .. }
+            | Action::EditField { .. }
             | Action::EditorBreak { .. }
             | Action::CommitEdit { .. }
             | Action::LeaveEdit { .. }
@@ -41,6 +43,9 @@ pub(super) fn dropped_under_a_prompt(action: &Action) -> bool {
             | Action::ReviewEdits { .. }
             | Action::SelectCell { .. }
             | Action::MoveSelection { .. }
+            // Closing the row panel closes the editor it draws, and the
+            // text of that editor joins the set.
+            | Action::ToggleRowPanel(_)
             | Action::NewConnection
             | Action::EditConnection(_)
             // These two take the dialog before they look at its kind.
@@ -515,7 +520,36 @@ impl App {
         .flatten()
     }
 
-    pub(super) fn edit_cell(&mut self, tab: ConnTabId, id: TabId, cell: CellPos, start: EditStart) {
+    /// The column of the tab's open editor, when the row panel draws it.
+    pub(super) fn panel_field(&self, tab: ConnTabId, id: TabId) -> Option<usize> {
+        let object = self.workspace(tab)?.object_tab(id)?;
+        let editor = object.edits.editor.as_ref()?;
+        (editor.place == EditorPlace::Panel).then_some(editor.cell.col)
+    }
+
+    /// An edit made in the row panel's field of the column `col` ended, by
+    /// a commit or a cancel: the keyboard goes back to that field, so
+    /// Enter edits it again and Tab goes on from it. Not in the terminal
+    /// look, where the keys are the grid's again, in normal mode.
+    pub(super) fn back_to_field(&mut self, tab: ConnTabId, id: TabId, col: usize) {
+        if self.look.terminal {
+            return;
+        }
+        if let Some(object) = self.object_tab_mut(tab, id) {
+            object.focus_field = Some(col);
+        }
+    }
+
+    /// Opens the editor on `cell`, drawn in `place`, or says there why the
+    /// cell cannot be edited.
+    pub(super) fn edit_cell(
+        &mut self,
+        tab: ConnTabId,
+        id: TabId,
+        cell: CellPos,
+        start: EditStart,
+        place: EditorPlace,
+    ) {
         // An editor open on another cell keeps its text.
         self.close_editor(tab, id, true);
         // What this edit comes to (a cell that is locked) is what the
@@ -544,9 +578,14 @@ impl App {
                     None => start_text(&table.page.rows[cell.row][cell.col], class),
                 },
             };
+            // A value of several lines, a long one or a document is edited
+            // in the popover at its cell, wherever the edit was asked for:
+            // the row panel has no editor for it yet.
+            let large = opens_large(&text, class);
             Ok(Editor {
                 cell,
-                large: opens_large(&text, class),
+                place: if large { EditorPlace::Grid } else { place },
+                large,
                 text,
                 focus: true,
                 touched,
@@ -573,6 +612,7 @@ impl App {
             Err(lock) => {
                 object.selection = Some(cell);
                 object.edits.why = Some((cell, lock));
+                object.edits.why_place = place;
             }
         }
         if let Some(workspace) = self.workspace_mut(tab) {
diff --git a/src/edit.rs b/src/edit.rs
index 7aec160..523f2d4 100644
--- a/src/edit.rs
+++ b/src/edit.rs
@@ -574,9 +574,21 @@ pub enum State {
     Failed(Error),
 }
 
+/// Where a tab's editor is drawn: on its cell in the grid (or in the
+/// popover at the cell), or in the row panel, in the place of the field's
+/// value. Its text becomes the same pending cell from either.
+#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
+pub enum EditorPlace {
+    #[default]
+    Grid,
+    Panel,
+}
+
 /// The editor that is open. Only `text` is the view's to change.
 pub struct Editor {
     pub cell: CellPos,
+    /// The view that draws it.
+    pub place: EditorPlace,
     pub text: String,
     /// The popover rather than the field on the cell.
     pub large: bool,
@@ -598,6 +610,9 @@ pub struct Edits {
     pub editor: Option<Editor>,
     /// Why the cell last asked for could not be edited.
     pub why: Option<(CellPos, Lock)>,
+    /// Where that edit was asked for: the reason is said there, at the
+    /// cell or under the row panel's field.
+    pub why_place: EditorPlace,
     /// The save that is running.
     pub saving: Option<Saving>,
     /// The last save that wrote, for the cells' green and the status.
diff --git a/src/model.rs b/src/model.rs
index 85a3718..0c9a2d8 100644
--- a/src/model.rs
+++ b/src/model.rs
@@ -239,6 +239,15 @@ pub enum Action {
         cell: CellPos,
         start: EditStart,
     },
+    /// Open the editor on a field of the row panel: the selected row's
+    /// value in the column of `cell`, edited in the panel instead of on the
+    /// cell. From the value, as `EditStart::Value` is. On a field that
+    /// cannot be edited it says why.
+    EditField {
+        tab: ConnTabId,
+        id: TabId,
+        cell: CellPos,
+    },
     /// The editor's text changed: check it again.
     EditorTyped {
         tab: ConnTabId,
@@ -251,11 +260,16 @@ pub enum Action {
         id: TabId,
         then: Advance,
     },
-    /// The editor lost the keyboard: keep its text, as a cell to fix when
-    /// its column does not take it.
+    /// The editor on `cell`, drawn in `place`, lost the keyboard: keep its
+    /// text, as a cell to fix when its column does not take it. It names
+    /// the editor it is about: the click that took the keyboard can have
+    /// asked for another editor in the same frame, which is the one open
+    /// by the time this is applied, and is not the one that was left.
     LeaveEdit {
         tab: ConnTabId,
         id: TabId,
+        cell: CellPos,
+        place: crate::edit::EditorPlace,
     },
     /// Close the editor and drop its text.
     CancelEdit {
@@ -1804,6 +1818,10 @@ pub struct ObjectTab {
     pub filter: FilterBar,
     /// The row panel's text for the selected row (see `App::format_rows`).
     pub fields: Option<RowFields>,
+    /// The column whose field of the row panel gets the keyboard back: an
+    /// edit made there was committed or dropped. Taken by the panel when
+    /// it draws that field.
+    pub focus_field: Option<usize>,
     /// What is pending, while the tab's values are edited. A tab that holds
     /// edits keeps its page.
     pub edits: crate::edit::Edits,
@@ -1921,6 +1939,7 @@ impl ObjectTab {
             count: Fetch::default(),
             filter: FilterBar::default(),
             fields: None,
+            focus_field: None,
             edits: crate::edit::Edits::default(),
         }
     }
diff --git a/src/shots.rs b/src/shots.rs
index 6fce00c..1b4d2ce 100644
--- a/src/shots.rs
+++ b/src/shots.rs
@@ -321,7 +321,9 @@ fn retype(
         cell: CellPos { row, col },
         start: EditStart::Replace(text.into()),
     });
-    harness.app.apply(Action::LeaveEdit { tab, id });
+    harness
+        .app
+        .apply(crate::testing::leave_edit(&harness.app, tab, id));
 }
 
 /// Image 3 becomes a cover that was deleted this morning: two pending
diff --git a/src/ui/data_view.rs b/src/ui/data_view.rs
index a6b4616..b360092 100644
--- a/src/ui/data_view.rs
+++ b/src/ui/data_view.rs
@@ -11,7 +11,7 @@ use std::collections::{BTreeMap, BTreeSet};
 use tabletist_db::{NewValue, SortDir, Value, ValueKind};
 
 use crate::app::App;
-use crate::edit::{Pending, State, Table};
+use crate::edit::{EditorPlace, Pending, State, Table};
 use crate::i18n::gettext;
 use crate::model::{Action, CellPos, ConnTabId, EditStart, ObjectTab, ObjectView, TabId};
 use crate::theme::{Icon, Look, Palette};
@@ -1268,12 +1268,21 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabId)
         changes.why = object
             .edits
             .why
+            // Where it was asked for: an edit asked for in the row panel
+            // is refused there.
+            .filter(|_| object.edits.why_place == EditorPlace::Grid)
             .filter(|_| !look.terminal && hold)
             .map(|(cell, lock)| {
                 let table = format::display_safe(&object.object.name);
                 (cell, cell_editor::lock_text(lock, &table, locale))
             });
-        let mut editor = object.edits.editor.as_mut();
+        // The editor the grid draws. One that is open in the row panel is
+        // the panel's: to the grid its cell is the selected cell, no more.
+        let mut editor = object
+            .edits
+            .editor
+            .as_mut()
+            .filter(|editor| editor.place == EditorPlace::Grid);
         let editing = editor.as_ref().map(|editor| editor.cell);
         // A value of several lines, a long one or a document is edited in
         // a popover at the cell, not on it.
@@ -1342,8 +1351,14 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabId)
             actions.push(Action::CommitEdit { tab, id, then });
         } else if outcome.cancel {
             actions.push(Action::CancelEdit { tab, id });
-        } else if outcome.left {
-            actions.push(Action::LeaveEdit { tab, id });
+        } else if let (true, Some(cell)) = (outcome.left, editing) {
+            let place = EditorPlace::Grid;
+            actions.push(Action::LeaveEdit {
+                tab,
+                id,
+                cell,
+                place,
+            });
         }
         if let Some(cell) = output.clicked {
             actions.push(Action::SelectCell { tab, id, cell });
````

- [ ] **Step 4: Run the tests**

Run: `~/.cargo/bin/cargo test --locked --lib -- editing::`
Expected: PASS, with the seven new tests among them:
`an_edit_asked_for_in_the_row_panel_opens_there_and_makes_the_same_pending_cell`, `an_edit_that_ends_in_the_panel_gives_the_keyboard_back_to_its_field`, `a_tall_value_asked_for_in_the_row_panel_is_edited_at_its_cell`, `leaving_closes_only_the_editor_that_was_left`, `a_refused_edit_remembers_where_it_was_asked_for`, `closing_the_row_panel_closes_the_editor_it_draws_and_keeps_the_text`, `the_row_panels_actions_are_dropped_under_a_prompt_about_the_changes`.

- [ ] **Step 5: Run the four checks and the shots lint, then commit**

```bash
~/.cargo/bin/cargo clippy --locked --features shots --lib --tests -- -D warnings
git add src/edit.rs src/model.rs src/app.rs src/app/editing.rs src/ui/data_view.rs src/testing.rs src/shots.rs src/ui/mod.rs src/ui/conflict_prompt.rs src/ui/write_prompts.rs
git commit -S -m "Give a table's editor a place: the grid or the row panel" -m "The tab's one editor can be drawn in the row panel instead of on its cell. Its text becomes the same pending cell from either, by the same path. Nothing asks for the panel's yet." -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: Edit the row, by its selected cell

**Files:**
- Modify: `src/model.rs` (`Action::EditRow`)
- Modify: `src/app.rs` (its arm, and tests)
- Modify: `src/app/editing.rs` (`edit_row`, `dropped_under_a_prompt`)

`EditRow` is "edit this row in the panel": what the footer's Edit, `Mod+I` and `e` will ask for. It shows the panel, takes the selected cell where it can be edited and otherwise the row's first cell that can, in the page's column order, and opens the editor there in the panel. Where no cell of the row can be edited it asks for the selected cell all the same, which records why: the terminal's mode line says it, and the footer will (task 5). Only in the Data view, which is where the panel shows a row.

- [ ] **Step 1: Write the failing tests**

````diff
diff --git a/src/app.rs b/src/app.rs
index 0ccd0da..d40eaa3 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -14600,5 +14600,137 @@ mod tests {
             );
             assert_eq!(object(&harness, tab, id).edits.cells.len(), 1);
         }
+
+        #[test]
+        fn editing_the_row_takes_the_selected_cells_field_or_the_first_that_can_be_edited() {
+            let mut harness = Harness::new();
+            let (tab, id) = harness.editable();
+            let panel = |harness: &Harness| harness.app.workspace(tab).unwrap().row_panel;
+            // With no row selected there is nothing to edit.
+            harness.app.apply(Action::EditRow { tab, id });
+            assert!(editor(&harness, tab, id).is_none());
+            // The selected cell's field, in the panel, which is shown.
+            harness.app.apply(Action::ToggleRowPanel(tab));
+            assert!(!panel(&harness));
+            harness.app.apply(Action::SelectCell {
+                tab,
+                id,
+                cell: at(1, 1),
+            });
+            harness.app.apply(Action::EditRow { tab, id });
+            assert!(panel(&harness));
+            assert_eq!(
+                editor(&harness, tab, id),
+                Some((at(1, 1), EditorPlace::Panel, false))
+            );
+            harness.app.apply(Action::CancelEdit { tab, id });
+            // On the key's cell, which is locked: the row's first field
+            // that can be edited.
+            harness.app.apply(Action::SelectCell {
+                tab,
+                id,
+                cell: at(2, 0),
+            });
+            harness.app.apply(Action::EditRow { tab, id });
+            assert_eq!(
+                editor(&harness, tab, id),
+                Some((at(2, 1), EditorPlace::Panel, false))
+            );
+            assert_eq!(object(&harness, tab, id).edits.why, None);
+            // An editor open in the grid gives way, its text kept.
+            harness.app.apply(Action::EditCell {
+                tab,
+                id,
+                cell: at(3, 1),
+                start: EditStart::Value,
+            });
+            type_text(&mut harness, tab, id, "dan@example.com");
+            harness.app.apply(Action::EditRow { tab, id });
+            assert_eq!(
+                editor(&harness, tab, id),
+                Some((at(3, 1), EditorPlace::Panel, false))
+            );
+            let text = &object(&harness, tab, id)
+                .edits
+                .editor
+                .as_ref()
+                .unwrap()
+                .text;
+            assert_eq!(text, "dan@example.com");
+        }
+
+        #[test]
+        fn editing_a_row_that_cannot_be_edited_shows_the_panel_and_says_why() {
+            let mut harness = Harness::new();
+            let (tab, id) = harness.editable();
+            let why = |harness: &Harness| {
+                let edits = &object(harness, tab, id).edits;
+                edits.why.map(|(cell, lock)| (cell, lock, edits.why_place))
+            };
+            harness.app.apply(Action::ToggleRowPanel(tab));
+            harness.app.apply(Action::SelectCell {
+                tab,
+                id,
+                cell: at(1, 1),
+            });
+            // No cell of the row can be edited: the reason is the row's,
+            // kept for the selected cell.
+            harness.app.workspace_mut(tab).unwrap().access = tabletist_db::Access::ReadOnly;
+            harness.app.apply(Action::EditRow { tab, id });
+            assert!(harness.app.workspace(tab).unwrap().row_panel);
+            assert!(editor(&harness, tab, id).is_none());
+            assert_eq!(
+                why(&harness),
+                Some((at(1, 1), Lock::ReadOnly, EditorPlace::Panel))
+            );
+            // Each cell is locked for a reason of its own: the selected
+            // cell's is the one said.
+            harness.app.workspace_mut(tab).unwrap().access = tabletist_db::Access::Writable;
+            let mut structure = crate::testing::fixture_structure();
+            for column in &mut structure.columns[1..] {
+                column.generated = true;
+            }
+            harness
+                .app
+                .workspace_mut(tab)
+                .unwrap()
+                .object_tab_mut(id)
+                .unwrap()
+                .structure
+                .value = Some(structure);
+            harness.app.apply(Action::EditRow { tab, id });
+            assert!(editor(&harness, tab, id).is_none());
+            assert_eq!(
+                why(&harness),
+                Some((at(1, 1), Lock::Generated, EditorPlace::Panel))
+            );
+            // In the Structure view there is no panel to edit in.
+            harness.app.apply(Action::SetView {
+                tab,
+                object_tab: id,
+                view: crate::model::ObjectView::Structure,
+            });
+            harness
+                .app
+                .workspace_mut(tab)
+                .unwrap()
+                .object_tab_mut(id)
+                .unwrap()
+                .edits
+                .why = None;
+            harness.app.apply(Action::EditRow { tab, id });
+            assert_eq!(why(&harness), None);
+        }
+
+        #[test]
+        fn editing_the_row_is_dropped_under_a_prompt_about_the_changes() {
+            let mut harness = Harness::new();
+            let (tab, id) = harness.editable();
+            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
+            harness.app.apply(Action::CloseTab { tab, id });
+            assert!(leave_prompt(&harness).is_some());
+            harness.app.apply(Action::EditRow { tab, id });
+            assert!(editor(&harness, tab, id).is_none());
+        }
     }
 }
````

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked --lib -- editing::editing_`
Expected: it does not compile: `error[E0599]: no variant named `EditRow` found for enum `Action``.

- [ ] **Step 3: Add the action**

````diff
diff --git a/src/app.rs b/src/app.rs
index d40eaa3..fa701d9 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -811,6 +811,7 @@ impl App {
             Action::EditField { tab, id, cell } => {
                 self.edit_cell(tab, id, cell, EditStart::Value, EditorPlace::Panel);
             }
+            Action::EditRow { tab, id } => self.edit_row(tab, id),
             Action::EditorTyped { tab, id } => {
                 let problem = self.editor_problem(tab, id);
                 if let Some(editor) = self.editor_mut(tab, id) {
diff --git a/src/app/editing.rs b/src/app/editing.rs
index 6b45ca3..a04852a 100644
--- a/src/app/editing.rs
+++ b/src/app/editing.rs
@@ -32,6 +32,7 @@ pub(super) fn dropped_under_a_prompt(action: &Action) -> bool {
         action,
         Action::EditCell { .. }
             | Action::EditField { .. }
+            | Action::EditRow { .. }
             | Action::EditorBreak { .. }
             | Action::CommitEdit { .. }
             | Action::LeaveEdit { .. }
@@ -620,6 +621,36 @@ impl App {
         }
     }
 
+    /// Edits the selected row in the row panel: the selected cell's field
+    /// where it can be edited, and otherwise the row's first field that
+    /// can, in the page's column order. The panel is shown for it. A row
+    /// with no such field says why, for the selected cell: no cell of it
+    /// can be edited (a read-only connection, a view, a save that runs),
+    /// or each is locked for a reason of its own.
+    pub(super) fn edit_row(&mut self, tab: ConnTabId, id: TabId) {
+        let field = self.table(tab, id, |table, object| {
+            let cell = object.selection?;
+            // The panel shows a row of the Data view only.
+            if object.view != crate::model::ObjectView::Data {
+                return None;
+            }
+            if table.lock(cell).is_none() || table.row_lock(cell.row).is_some() {
+                return Some(cell);
+            }
+            let first = (0..table.page.columns.len())
+                .map(|col| CellPos { row: cell.row, col })
+                .find(|other| table.lock(*other).is_none());
+            Some(first.unwrap_or(cell))
+        });
+        let Some(Some(cell)) = field else {
+            return;
+        };
+        if let Some(workspace) = self.workspace_mut(tab) {
+            workspace.row_panel = true;
+        }
+        self.edit_cell(tab, id, cell, EditStart::Value, EditorPlace::Panel);
+    }
+
     /// Takes the open editor's text as its cell's new value and closes it.
     /// A text its column does not take keeps the editor open, unless the
     /// edit is `left` (the keyboard went elsewhere): then the text is kept
diff --git a/src/model.rs b/src/model.rs
index 0c9a2d8..a89bfca 100644
--- a/src/model.rs
+++ b/src/model.rs
@@ -248,6 +248,13 @@ pub enum Action {
         id: TabId,
         cell: CellPos,
     },
+    /// Edit the selected row in the row panel: show the panel, and open
+    /// the editor there on the selected cell's field, or on the row's
+    /// first field that can be edited where that one cannot.
+    EditRow {
+        tab: ConnTabId,
+        id: TabId,
+    },
     /// The editor's text changed: check it again.
     EditorTyped {
         tab: ConnTabId,
````

- [ ] **Step 4: Run the tests**

Run: `~/.cargo/bin/cargo test --locked --lib -- editing::editing_`
Expected: PASS: `editing_the_row_takes_the_selected_cells_field_or_the_first_that_can_be_edited`, `editing_a_row_that_cannot_be_edited_shows_the_panel_and_says_why`, `editing_the_row_is_dropped_under_a_prompt_about_the_changes`, and `editing_pins_a_preview_tab`, which was there.

- [ ] **Step 5: Run the four checks, then commit**

```bash
git add src/model.rs src/app.rs src/app/editing.rs
git commit -S -m "Edit a row in the row panel, by its selected cell" -m "EditRow shows the panel and opens its editor on the selected cell's field, or on the row's first field that can be edited. A row with none says why." -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: The panel edits a field

**Files:**
- Create: `src/ui/row_form.rs`
- Modify: `src/ui/mod.rs` (`pub mod row_form;`, and the tests)
- Modify: `src/ui/cell_editor.rs` (`in_panel`, `block_cursor`)
- Modify: `src/ui/row_panel.rs` (`show`, `draw`, `field`, `value_of`, `caption_button`)
- Modify: `src/ui/data_view.rs` (`editor_target`)

The view's half. After it a value of the panel can be edited with the pointer and with Enter.

How it is built:

- **`row_form::Form`** is made once a frame for the row the panel shows. It says what the form makes of each field (`Part`): `Read` (nothing: a SQL result's row, a row under a row lock), `Editable`, `Locked(lock)`, `InGrid` (the grid is editing that cell) or `Editing` (the panel's editor is on it). It holds the panel's editor for the frame and draws it (`Form::editing`), and collects what ends it in `Form::ending`.
- **The panel's own editor is taken out of the tab for the frame** (`row_form::take_editor`, given back by `put_editor`). `row_panel::show` becomes that wrapper round `draw`, the old body. It also takes `ObjectTab::focus_field`, which task 2 sets.
- **`cell_editor::in_panel`** is the cell's field without a cell: a `widgets::field` as wide as the room it has, in the role the value is read in. It shares the keys with the cell's field (`ending_keys`, `leaving_keys`, `hold_keys`, `keyboard_after`), so Esc, the terminal's Ctrl+C, a paste that is cut and a dialog that takes the keyboard all behave as on a cell. It turns every commit into `Advance::Stay`. Under the field it writes what the text fails, and the column's length counter.
- **`row_panel::field` is split in two.** The label line stays in `field`, with the pencil beside Copy for a `Part::Editable`. What was the rest of it, the value, is `value_of`, which now says what it drew (`Shown`): the place a double-click edits (the text, the NULL mark, the stand-in of an empty or blank text; none for a tree or a list, which have their own clicks) and the text's response (what takes a caret). `field` then adds the outline under the pointer, the double-click, Enter and F2 with the caret in the text, and F2 on a pencil that has the keyboard (Enter there is the button's press). When it gives the keyboard back to a field it asks for one more frame: the pencil and a caret show only once they have it.
- **What ends the open field goes first.** `draw` sends `Form::ending` to `app.actions` ahead of the panel's other actions. The field can be left for a pencil drawn above it, or for the footer's Edit: that control's action must find the old editor already closed.
- **The grid** needs one change here: `editor_target` takes the cell instead of reading the tab's editor, since the panel's is out of the tab when the panel asks.

`copy_button` becomes `caption_button`, with the icon as an argument: the pencil is the same button.

- [ ] **Step 1: Write the failing tests**

Twelve tests, in every look where the look has the thing. They use the fixture `with_quantities` (`id`, `email`, `qty`), which has two one-line values that can be edited.

````diff
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
index f521272..2a64660 100644
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -19616,4 +19616,530 @@ mod tests {
             assert!(!edits(&harness, tab, id).reviewing, "{}", look.name);
         }
     }
+
+    // The row panel as a row form.
+
+    /// The table of `with_quantities` (`id`, `email`, `qty`), its row `row`
+    /// selected: the panel shows it, with two fields that can be edited.
+    fn form_row(look: Look, row: usize) -> (Harness, ConnTabId, TabId) {
+        let (mut harness, tab, id) = with_quantities(look);
+        select(&mut harness, tab, id, (row, 0));
+        (harness, tab, id)
+    }
+
+    /// The tab's open editor: its cell (row, column) and whether the row
+    /// panel draws it.
+    fn form_editor(harness: &Harness, tab: ConnTabId, id: TabId) -> Option<((usize, usize), bool)> {
+        let editor = edits(harness, tab, id).editor.as_ref()?;
+        let panel = editor.place == crate::edit::EditorPlace::Panel;
+        Some(((editor.cell.row, editor.cell.col), panel))
+    }
+
+    /// Where the row panel wrote `text`: the grid writes a cell's value
+    /// too, left of the panel.
+    fn panel_text(harness: &Harness, text: &str) -> egui::Rect {
+        let written = harness.text_rects.iter().filter(|(piece, _)| piece == text);
+        let rightmost = written.max_by(|a, b| a.1.left().total_cmp(&b.1.left()));
+        rightmost
+            .unwrap_or_else(|| panic!("the panel does not show {text}"))
+            .1
+    }
+
+    /// Whether a button named `name` is on screen.
+    fn has_button(harness: &mut Harness, name: &str) -> bool {
+        let tree = harness.settle();
+        crate::testing::node(&tree, name, egui::accesskit::Role::Button).is_some()
+    }
+
+    #[test]
+    fn a_fields_pencil_edits_its_value_in_the_row_panel() {
+        for look in Look::ALL {
+            let (mut harness, tab, id) = form_row(look, 1);
+            // The key is locked: its field has no pencil.
+            assert!(has_button(&mut harness, "Edit email"), "{}", look.name);
+            assert!(has_button(&mut harness, "Edit qty"), "{}", look.name);
+            assert!(!has_button(&mut harness, "Edit id"), "{}", look.name);
+            let value = panel_text(&harness, "user2@example.com");
+            harness.click("Edit email");
+            assert_eq!(
+                form_editor(&harness, tab, id),
+                Some(((1, 1), true)),
+                "{}",
+                look.name
+            );
+            assert!(harness.ctx.text_edit_focused(), "{}", look.name);
+            // The field stands where the value stood, in the panel: under
+            // the field's label, and not on the grid's cell.
+            let tree = harness.settle();
+            let field =
+                crate::testing::bounds(&tree, "Edit email", egui::accesskit::Role::TextInput)
+                    .expect("the field");
+            assert!(field.contains(value.center()), "{}", look.name);
+            let cell = harness
+                .text_rects
+                .iter()
+                .find(|(text, _)| text == "user3@example.com")
+                .map(|(_, rect)| *rect)
+                .expect("the cell under the edited one");
+            assert!(field.left() > cell.right(), "{}", look.name);
+            // Its pencil is gone while it is edited; the other field's stays.
+            assert!(!has_button(&mut harness, "Edit email"), "{}", look.name);
+            assert!(has_button(&mut harness, "Edit qty"), "{}", look.name);
+            // Typed at the end of the value, and committed by Enter, which
+            // moves nothing: the same row is selected and shown.
+            type_text(&mut harness, "x");
+            harness.press(Key::Enter, Modifiers::NONE);
+            assert!(edits(&harness, tab, id).editor.is_none(), "{}", look.name);
+            assert_eq!(
+                pending_text(&harness, tab, id, (1, 1)).as_deref(),
+                Some("user2@example.comx"),
+                "{}",
+                look.name
+            );
+            assert_eq!(selected(&harness, tab, id), Some((1, 1)), "{}", look.name);
+            // The panel shows the new value, and what it was.
+            assert!(painted(&harness, "user2@example.comx"), "{}", look.name);
+            assert!(painted(&harness, "was user2@example.com"), "{}", look.name);
+            assert!(has_button(&mut harness, "Edit email"), "{}", look.name);
+        }
+    }
+
+    #[test]
+    fn a_double_click_on_a_value_edits_it_in_the_row_panel_and_one_click_does_not() {
+        for look in Look::ALL {
+            let (mut harness, tab, id) = form_row(look, 2);
+            let at = panel_text(&harness, "user3@example.com").center();
+            click_at(&mut harness, at);
+            assert!(edits(&harness, tab, id).editor.is_none(), "{}", look.name);
+            click_at(&mut harness, at);
+            assert_eq!(
+                form_editor(&harness, tab, id),
+                Some(((2, 1), true)),
+                "{}",
+                look.name
+            );
+            assert_eq!(
+                editor_text(&harness, tab, id).as_deref(),
+                Some("user3@example.com"),
+                "{}",
+                look.name
+            );
+            // The key's value takes no edit, however it is clicked.
+            let (mut harness, tab, id) = form_row(look, 2);
+            let at = panel_text(&harness, "3").center();
+            click_at(&mut harness, at);
+            click_at(&mut harness, at);
+            assert!(edits(&harness, tab, id).editor.is_none(), "{}", look.name);
+        }
+    }
+
+    #[test]
+    fn enter_and_f2_on_a_value_edit_it_and_the_keyboard_comes_back_to_it() {
+        for look in desktop_looks() {
+            let (mut harness, tab, id) = form_row(look, 1);
+            // A click puts a caret in the value, as it always did.
+            let at = panel_text(&harness, "user2@example.com").center();
+            click_at(&mut harness, at);
+            assert!(harness.ctx.text_edit_focused(), "{}", look.name);
+            assert!(edits(&harness, tab, id).editor.is_none(), "{}", look.name);
+            harness.press(Key::Enter, Modifiers::NONE);
+            assert_eq!(
+                form_editor(&harness, tab, id),
+                Some(((1, 1), true)),
+                "{}",
+                look.name
+            );
+            // Esc drops the edit, and the keyboard is on the value again:
+            // F2 edits it once more.
+            type_text(&mut harness, "x");
+            harness.press(Key::Escape, Modifiers::NONE);
+            let now = edits(&harness, tab, id);
+            assert!(
+                now.editor.is_none() && now.cells.is_empty(),
+                "{}",
+                look.name
+            );
+            assert!(harness.ctx.text_edit_focused(), "{}", look.name);
+            harness.press(Key::F2, Modifiers::NONE);
+            assert_eq!(
+                form_editor(&harness, tab, id),
+                Some(((1, 1), true)),
+                "{}",
+                look.name
+            );
+            assert_eq!(
+                editor_text(&harness, tab, id).as_deref(),
+                Some("user2@example.com"),
+                "{}",
+                look.name
+            );
+            // Tab commits as Enter does, and stays.
+            type_text(&mut harness, "y");
+            harness.press(Key::Tab, Modifiers::NONE);
+            assert_eq!(
+                pending_text(&harness, tab, id, (1, 1)).as_deref(),
+                Some("user2@example.comy"),
+                "{}",
+                look.name
+            );
+            assert_eq!(selected(&harness, tab, id), Some((1, 1)), "{}", look.name);
+            assert!(harness.ctx.text_edit_focused(), "{}", look.name);
+            harness.press(Key::Enter, Modifiers::NONE);
+            assert_eq!(
+                form_editor(&harness, tab, id),
+                Some(((1, 1), true)),
+                "{}",
+                look.name
+            );
+            assert_eq!(
+                editor_text(&harness, tab, id).as_deref(),
+                Some("user2@example.comy"),
+                "{}",
+                look.name
+            );
+        }
+    }
+
+    /// Gives the keyboard to the button named `name`, as the Tab key or a
+    /// screen reader does.
+    fn focus_button(harness: &mut Harness, name: &str) {
+        use egui::accesskit::{Action, ActionRequest, Role, TreeId};
+        let tree = harness.settle();
+        let target = crate::testing::node(&tree, name, Role::Button)
+            .unwrap_or_else(|| panic!("no button {name}"));
+        harness.frame(vec![egui::Event::AccessKitActionRequest(ActionRequest {
+            target_tree: TreeId::ROOT,
+            target_node: target,
+            action: Action::Focus,
+            data: None,
+        })]);
+        harness.settle();
+    }
+
+    /// Whether the button named `name` has the keyboard.
+    fn button_focused(harness: &mut Harness, name: &str) -> bool {
+        let tree = harness.settle();
+        let button = crate::testing::node(&tree, name, egui::accesskit::Role::Button);
+        button.is_some_and(|button| button == tree.focus)
+    }
+
+    #[test]
+    fn a_pencil_that_has_the_keyboard_edits_by_enter_and_f2_and_gets_it_back() {
+        for look in desktop_looks() {
+            let (mut harness, tab, id) = form_row(look, 1);
+            // A NULL has no text to put a caret in: its pencil is where
+            // the keyboard stands for the field.
+            let object = harness.app.workspace_mut(tab).unwrap();
+            let object = object.object_tab_mut(id).unwrap();
+            object.rows.value.as_mut().unwrap().rows[1][1] = tabletist_db::Value::Null;
+            object.fields = None;
+            focus_button(&mut harness, "Edit email");
+            assert!(button_focused(&mut harness, "Edit email"), "{}", look.name);
+            harness.press(Key::Enter, Modifiers::NONE);
+            assert_eq!(
+                form_editor(&harness, tab, id),
+                Some(((1, 1), true)),
+                "{}",
+                look.name
+            );
+            // Dropped: the value is NULL still, and the keyboard is on
+            // the pencil again. F2 edits from there too.
+            harness.press(Key::Escape, Modifiers::NONE);
+            assert!(edits(&harness, tab, id).editor.is_none(), "{}", look.name);
+            assert!(button_focused(&mut harness, "Edit email"), "{}", look.name);
+            harness.press(Key::F2, Modifiers::NONE);
+            assert_eq!(
+                form_editor(&harness, tab, id),
+                Some(((1, 1), true)),
+                "{}",
+                look.name
+            );
+        }
+    }
+
+    #[test]
+    fn mod_s_saves_from_the_panels_field_with_what_is_typed() {
+        for look in Look::ALL {
+            let (mut harness, tab, id) = form_row(look, 1);
+            harness.click("Edit email");
+            type_text(&mut harness, "x");
+            harness.press(Key::S, Modifiers::COMMAND);
+            assert!(edits(&harness, tab, id).editor.is_none(), "{}", look.name);
+            assert_eq!(writes(&harness), 1, "{}", look.name);
+            let Some(crate::backend::Command::Write { changes, .. }) =
+                harness.app.backend.sent.last()
+            else {
+                panic!("a save was sent");
+            };
+            assert_eq!(
+                changes.rows[0].set[0].new,
+                tabletist_db::NewValue::Text("user2@example.comx".into()),
+                "{}",
+                look.name
+            );
+        }
+    }
+
+    #[test]
+    fn the_terminals_escape_keeps_a_panel_edit_and_ctrl_c_drops_it() {
+        let (mut harness, tab, id) = form_row(Look::omarchy(), 1);
+        let panel = |harness: &Harness| harness.app.workspace(tab).unwrap().row_panel;
+        harness.click("Edit email");
+        type_text(&mut harness, "!");
+        assert!(painted(&harness, "-- INSERT --"), "{:?}", harness.painted);
+        harness.press(Key::Escape, Modifiers::NONE);
+        assert!(edits(&harness, tab, id).editor.is_none());
+        assert_eq!(
+            pending_text(&harness, tab, id, (1, 1)).as_deref(),
+            Some("user2@example.com!")
+        );
+        // The key left insert mode and did no more: the panel is open.
+        assert!(panel(&harness));
+        assert!(!painted(&harness, "-- INSERT --"));
+        harness.click("Edit email");
+        type_text(&mut harness, "?");
+        harness.press(Key::C, Modifiers::CTRL);
+        assert!(edits(&harness, tab, id).editor.is_none());
+        assert_eq!(
+            pending_text(&harness, tab, id, (1, 1)).as_deref(),
+            Some("user2@example.com!")
+        );
+        assert!(panel(&harness));
+    }
+
+    #[test]
+    fn a_text_the_column_refuses_keeps_the_panels_field_and_says_why_under_it() {
+        for look in Look::ALL {
+            let (mut harness, tab, id) = form_row(look, 1);
+            let palette = harness.app.palette;
+            harness.click("Edit qty");
+            assert_eq!(editor_text(&harness, tab, id).as_deref(), Some("71"));
+            type_text(&mut harness, "a");
+            let message = "INTEGER expects a whole number";
+            assert!(
+                painted_in(&harness, message, palette.danger),
+                "{}: {:?}",
+                look.name,
+                harness.painted
+            );
+            // Under the field, in the panel.
+            let tree = harness.settle();
+            let field = crate::testing::bounds(&tree, "Edit qty", egui::accesskit::Role::TextInput)
+                .expect("the field");
+            // The field's border is red, whether or not a key was pressed.
+            let red = egui::Stroke::new(1.0, palette.danger);
+            assert!(
+                harness.outlines.iter().any(|(rect, stroke)| *stroke == red
+                    && (rect.center() - field.center()).length() < 1.0),
+                "{}",
+                look.name
+            );
+            let said = harness.painted_rect(message).unwrap();
+            assert!(said.top() >= field.bottom() - 1.0, "{}", look.name);
+            assert!(said.left() >= field.left() - 1.0, "{}", look.name);
+            // Enter does not leave it.
+            harness.press(Key::Enter, Modifiers::NONE);
+            assert_eq!(
+                form_editor(&harness, tab, id),
+                Some(((1, 2), true)),
+                "{}",
+                look.name
+            );
+            assert!(edits(&harness, tab, id).cells.is_empty(), "{}", look.name);
+            // Left for another field, the text is kept as a cell to fix.
+            harness.click("Edit email");
+            assert_eq!(
+                form_editor(&harness, tab, id),
+                Some(((1, 1), true)),
+                "{}",
+                look.name
+            );
+            assert_eq!(
+                pending_text(&harness, tab, id, (1, 2)).as_deref(),
+                Some("71a"),
+                "{}",
+                look.name
+            );
+            assert_eq!(edits(&harness, tab, id).counts().to_fix, 1);
+        }
+    }
+
+    #[test]
+    fn a_pencil_clicked_while_another_field_is_edited_opens_its_own_and_keeps_the_text() {
+        for look in Look::ALL {
+            // The field below the one being edited, then the one above it:
+            // the panel draws a pencil above the open field before that
+            // field says it was left.
+            for (first, typed, then, kept) in [
+                (
+                    "Edit email",
+                    "x",
+                    "Edit qty",
+                    ((1, 1), "user2@example.comx"),
+                ),
+                ("Edit qty", "9", "Edit email", ((1, 2), "719")),
+            ] {
+                let (mut harness, tab, id) = form_row(look, 1);
+                harness.click(first);
+                type_text(&mut harness, typed);
+                let tree = harness.settle();
+                let pencil = crate::testing::bounds(&tree, then, egui::accesskit::Role::Button)
+                    .expect("the other field's pencil");
+                click_at(&mut harness, pencil.center());
+                let other = if kept.0 == (1, 1) { (1, 2) } else { (1, 1) };
+                assert_eq!(
+                    form_editor(&harness, tab, id),
+                    Some((other, true)),
+                    "{}: {first} then {then}",
+                    look.name
+                );
+                assert_eq!(
+                    pending_text(&harness, tab, id, kept.0).as_deref(),
+                    Some(kept.1),
+                    "{}: {first} then {then}",
+                    look.name
+                );
+                assert!(harness.ctx.text_edit_focused(), "{}", look.name);
+            }
+        }
+    }
+
+    #[test]
+    fn a_field_whose_cell_the_grid_is_editing_says_so() {
+        for look in Look::ALL {
+            let (mut harness, tab, id) = form_row(look, 1);
+            let palette = harness.app.palette;
+            let saying = look.label("Editing in the grid…");
+            assert!(!painted(&harness, &saying), "{}", look.name);
+            open_editor(&mut harness, tab, id, (1, 1));
+            assert_eq!(form_editor(&harness, tab, id), Some(((1, 1), false)));
+            assert!(
+                painted_in(&harness, &saying, palette.accent),
+                "{}: {:?}",
+                look.name,
+                harness.painted
+            );
+            assert!(harness.has(&saying), "{}", look.name);
+            // That field offers no second editor; the others still do.
+            assert!(!has_button(&mut harness, "Edit email"), "{}", look.name);
+            assert!(has_button(&mut harness, "Edit qty"), "{}", look.name);
+            // Another field's pencil takes the editor to the panel, and the
+            // grid's field, which lost the keyboard to it, closes nothing.
+            let tree = harness.settle();
+            let pencil = crate::testing::bounds(&tree, "Edit qty", egui::accesskit::Role::Button)
+                .expect("the pencil");
+            click_at(&mut harness, pencil.center());
+            assert_eq!(
+                form_editor(&harness, tab, id),
+                Some(((1, 2), true)),
+                "{}",
+                look.name
+            );
+            assert!(!painted(&harness, &saying), "{}", look.name);
+        }
+    }
+
+    #[test]
+    fn a_tall_values_pencil_opens_the_popover_at_its_cell() {
+        for look in Look::ALL {
+            let (mut harness, tab, id) = editable_in(look);
+            select(&mut harness, tab, id, (0, 0));
+            // A document: its pencil stands with its other controls.
+            harness.click("Edit meta");
+            let editor = edits(&harness, tab, id).editor.as_ref().expect("an editor");
+            assert!(editor.large, "{}", look.name);
+            assert_eq!(form_editor(&harness, tab, id), Some(((0, 2), false)));
+            assert!(harness.ctx.text_edit_focused(), "{}", look.name);
+            assert!(
+                painted(&harness, &look.label("Editing in the grid…")),
+                "{}",
+                look.name
+            );
+            // A NULL has no text to click: its pencil is its way in, and a
+            // double-click on the mark is another.
+            let (mut harness, tab, id) = form_row(look, 1);
+            let null = tabletist_db::Value::Null;
+            let object = harness.app.workspace_mut(tab).unwrap();
+            let object = object.object_tab_mut(id).unwrap();
+            object.rows.value.as_mut().unwrap().rows[1][1] = null;
+            object.fields = None;
+            harness.settle();
+            let at = panel_text(&harness, "NULL").center();
+            click_at(&mut harness, at);
+            click_at(&mut harness, at);
+            assert_eq!(
+                form_editor(&harness, tab, id),
+                Some(((1, 1), true)),
+                "{}",
+                look.name
+            );
+            assert_eq!(editor_text(&harness, tab, id).as_deref(), Some(""));
+        }
+    }
+
+    #[test]
+    fn a_row_that_cannot_be_edited_has_no_pencils() {
+        for look in Look::ALL {
+            // A read-only connection.
+            let mut harness = Harness::new();
+            harness.set_look(look);
+            let tab = harness.connect_fake_as(true);
+            harness.click("users");
+            harness.answer_structure(crate::testing::fixture_structure());
+            harness.answer_rows(crate::testing::page(5, false));
+            harness.click("Row 2");
+            assert!(harness.has("Copy email"), "{}", look.name);
+            assert!(!has_button(&mut harness, "Edit email"), "{}", look.name);
+            // No click starts an edit there.
+            let at = panel_text(&harness, "user2@example.com").center();
+            click_at(&mut harness, at);
+            click_at(&mut harness, at);
+            let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
+            let now = edits(&harness, tab, id);
+            assert!(now.editor.is_none() && now.why.is_none(), "{}", look.name);
+            // A SQL editor's result row.
+            let mut harness = Harness::new();
+            harness.set_look(look);
+            let tab = harness.connect_fake_as(false);
+            with_sql_result(&mut harness, tab, 3);
+            assert!(harness.app.workspace(tab).unwrap().row_panel);
+            let tree = harness.settle();
+            let pencils = crate::testing::labels(&tree)
+                .into_iter()
+                .filter(|label| label.starts_with("Edit "))
+                .count();
+            assert_eq!(pencils, 0, "{}", look.name);
+        }
+    }
+
+    #[test]
+    fn a_value_that_can_be_edited_shows_a_fields_outline_under_the_pointer() {
+        for look in Look::ALL {
+            let (mut harness, _, _) = form_row(look, 1);
+            let palette = harness.app.palette;
+            let color = if look.terminal {
+                palette.accent
+            } else {
+                palette.border
+            };
+            let outlined = |harness: &Harness, text: &str| {
+                let value = panel_text(harness, text);
+                harness.outlines.iter().any(|(rect, stroke)| {
+                    *stroke == egui::Stroke::new(1.0, color)
+                        && rect.contains_rect(value)
+                        && rect.height() < value.height() + 10.0
+                })
+            };
+            assert!(!outlined(&harness, "user2@example.com"), "{}", look.name);
+            let at = panel_text(&harness, "user2@example.com").center();
+            harness.frame(vec![egui::Event::PointerMoved(at)]);
+            harness.settle();
+            assert!(outlined(&harness, "user2@example.com"), "{}", look.name);
+            // The key's value shows none: it cannot be edited.
+            let at = panel_text(&harness, "2").center();
+            harness.frame(vec![egui::Event::PointerMoved(at)]);
+            harness.settle();
+            assert!(!outlined(&harness, "2"), "{}", look.name);
+        }
+    }
 }
````

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked --lib -- a_fields_pencil a_double_click_on_a_value enter_and_f2_on_a_value the_terminals_escape_keeps_a_panel a_text_the_column_refuses_keeps_the_panels a_pencil_clicked_while a_field_whose_cell_the_grid a_tall_values_pencil a_row_that_cannot_be_edited_has_no_pencils a_value_that_can_be_edited_shows a_pencil_that_has_the_keyboard mod_s_saves_from_the_panels`
Expected: they compile (the model is there since task 2), and eleven of the twelve FAIL. The ones that press or look for a pencil panic in the harness with `nothing labelled "Edit email"` (or `"Edit qty"`, `"Edit meta"`, `no button Edit email`); the others fail an assertion, since no editor opened and no outline was painted. `a_row_that_cannot_be_edited_has_no_pencils` passes already: it pins what must stay true once the others pass.

- [ ] **Step 3: Draw the form**

````diff
diff --git a/src/ui/cell_editor.rs b/src/ui/cell_editor.rs
index 5594282..d85a7e4 100644
--- a/src/ui/cell_editor.rs
+++ b/src/ui/cell_editor.rs
@@ -9,7 +9,7 @@ use crate::edit::{Editor, Lock, MAX_EDIT_BYTES, Problem};
 use crate::i18n::{Locale, gettext, ngettext};
 use crate::model::{Advance, ConnTabId, TabId};
 use crate::theme::{Look, Palette};
-use crate::typography::Text;
+use crate::typography::{Text, TextRole};
 use crate::ui::focus::{self, Ring};
 use crate::ui::format::display_safe;
 use crate::ui::keys::{consume_press, copy_is_ctrl_c, is_press};
@@ -263,25 +263,8 @@ pub fn field(
         .layouter(&mut layouter)
         .show(&mut child);
     bound(&mut editor.text);
-    if look.terminal
-        && output.response.has_focus()
-        && let Some(cursor) = output.cursor_range
-    {
-        // A terminal's cursor is a block: one character wide from where
-        // the next one goes, over the caret's line. Tinted, so the
-        // character under it reads through.
-        let caret = output.galley.pos_from_cursor(cursor.primary);
-        let block = Rect::from_min_size(
-            output.galley_pos + caret.min.to_vec2(),
-            vec2(role.width(ui.ctx(), look.faces, "0"), caret.height()),
-        );
-        ui.painter()
-            .with_clip_rect(output.text_clip_rect.intersect(ui.clip_rect()))
-            .rect_filled(
-                block,
-                CornerRadius::ZERO,
-                palette.accent.gamma_multiply(0.5),
-            );
+    if look.terminal && output.response.has_focus() {
+        block_cursor(ui, &output, role, look, palette);
     }
     let response: egui::Response = output.response.response;
     // The name only: the field keeps the role and the value egui gave it.
@@ -324,6 +307,142 @@ pub fn field(
     outcome
 }
 
+/// A terminal's cursor is a block: one character wide from where the next
+/// one goes, over the caret's line of the field `output` is of. Tinted, so
+/// the character under it reads through.
+fn block_cursor(
+    ui: &Ui,
+    output: &egui::text_edit::TextEditOutput,
+    role: TextRole,
+    look: &Look,
+    palette: &Palette,
+) {
+    let Some(cursor) = output.cursor_range else {
+        return;
+    };
+    let caret = output.galley.pos_from_cursor(cursor.primary);
+    let block = Rect::from_min_size(
+        output.galley_pos + caret.min.to_vec2(),
+        vec2(role.width(ui.ctx(), look.faces, "0"), caret.height()),
+    );
+    ui.painter()
+        .with_clip_rect(output.text_clip_rect.intersect(ui.clip_rect()))
+        .rect_filled(
+            block,
+            CornerRadius::ZERO,
+            palette.accent.gamma_multiply(0.5),
+        );
+}
+
+/// The editor in the row panel: a one-line field in the place of its
+/// field's value, as wide as the room it is given, its text in `role`, the
+/// role the value is read in. Its keys are the keys of the field on a cell,
+/// but nothing moves when it commits: Enter, Tab and Shift+Tab all take
+/// the text and stay. Under the field it says what the text fails, and how
+/// much of its column's length the text takes.
+pub fn in_panel(
+    ui: &mut Ui,
+    editor: &mut Editor,
+    target: &Target,
+    role: TextRole,
+    (look, palette, locale): (&Look, &Palette, Locale),
+) -> Outcome {
+    let mut outcome = Outcome::default();
+    let id = target.id;
+    let opened = std::mem::take(&mut editor.focus);
+    if opened {
+        take_keyboard(ui.ctx(), id, &editor.text);
+    }
+    let has = ui.memory(|memory| memory.has_focus(id));
+    let had = had_keyboard(ui.ctx(), id);
+    if has {
+        keep_keyboard(ui.ctx());
+    }
+    ending_keys(ui, has, had, look.terminal, &mut outcome);
+    if outcome.commit.is_some() {
+        outcome.commit = Some(Advance::Stay);
+    }
+    let width = ui.available_width();
+    let height = if look.terminal { 26.0 } else { 30.0 };
+    let mut layouter = crate::typography::layouter(look, role, palette.text);
+    let output = widgets::field(ui, &mut editor.text, look, role, height, 8)
+        .id(id)
+        .desired_width(width)
+        // Tab ends the edit; it is not egui's to move the keyboard with.
+        .lock_focus(true)
+        // As the field on a cell: a paste is cut, and left over the limit.
+        .char_limit(MAX_EDIT_BYTES + 1)
+        .layouter(&mut layouter)
+        .show(ui);
+    bound(&mut editor.text);
+    if look.terminal && output.response.has_focus() {
+        block_cursor(ui, &output, role, look, palette);
+    }
+    let response: egui::Response = output.response.response;
+    // The name only: the field keeps the role and the value egui gave it.
+    let name = format!("{} {}", gettext(locale, "Edit"), display_safe(&target.name));
+    ui.ctx().accesskit_node_builder(id, |node| {
+        node.set_label(name);
+    });
+    let has = response.has_focus();
+    if has {
+        hold_keys(ui.ctx(), id);
+    }
+    // A field's own ring, red while the text fails its check. The ring
+    // shows once the keyboard is in use; the red border does not wait for
+    // that, since a pasted text can fail before any key is pressed.
+    let radius = look.radius;
+    let ring = if editor.problem.is_some() {
+        ui.painter().rect_stroke(
+            response.rect,
+            CornerRadius::same(radius),
+            Stroke::new(1.0, palette.danger),
+            StrokeKind::Inside,
+        );
+        Ring::Failing { radius }
+    } else {
+        Ring::Field { radius }
+    };
+    focus::hint(ui, &response, response.rect, ring);
+    // The field can open far down a long row: it is brought into view
+    // once, when it takes the keyboard.
+    if opened {
+        response.scroll_to_me(None);
+    }
+    let problem = editor
+        .problem
+        .as_ref()
+        .map(|problem| problem_text(problem, &target.type_name, Some(&editor.text), locale));
+    let count = target
+        .max_chars
+        .map(|max| format!("{} / {max}", editor.text.chars().count()));
+    if problem.is_some() || count.is_some() {
+        let small = widgets::secondary(look);
+        ui.add_space(3.0);
+        ui.horizontal_top(|ui| {
+            let taken = count
+                .as_ref()
+                .map_or(0.0, |count| small.width(ui.ctx(), look.faces, count) + 8.0);
+            if let Some(problem) = &problem {
+                Text::one(look, small, problem, palette.danger)
+                    .wrap((width - taken).max(0.0))
+                    .layout(ui.ctx())
+                    .label(ui);
+            }
+            if let Some(count) = &count {
+                ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
+                    Text::one(look, small, count, palette.dim)
+                        .layout(ui.ctx())
+                        .label(ui);
+                });
+            }
+        });
+    }
+    outcome.changed = response.changed();
+    keyboard_after(ui.ctx(), target, has, had, &mut outcome);
+    outcome
+}
+
 /// Whether the field `id` had the keyboard when it was last drawn.
 fn had_keyboard(ctx: &egui::Context, id: Id) -> bool {
     ctx.data(|data| data.get_temp(had_id(id))).unwrap_or(false)
diff --git a/src/ui/data_view.rs b/src/ui/data_view.rs
index b360092..ded1416 100644
--- a/src/ui/data_view.rs
+++ b/src/ui/data_view.rs
@@ -1192,7 +1192,11 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabId)
     // What editing asks of the workspace and the tab together, read before
     // the tab is taken for its editor's text.
     let computed = computed_columns(workspace, object);
-    let target = editor_target(workspace, object, tab, hold);
+    let target = object
+        .edits
+        .editor
+        .as_ref()
+        .and_then(|editor| editor_target(workspace, object, tab, editor.cell, hold));
     // The tab itself from here on: the field on a cell edits the text its
     // editor holds, beside the page the grid reads. Nothing else of it is
     // changed.
@@ -1465,17 +1469,18 @@ fn computed_columns(workspace: &crate::model::Workspace, object: &ObjectTab) ->
         .collect()
 }
 
-/// The cell the tab's open editor is on, as its field needs it.
-fn editor_target(
+/// The cell `cell` an editor of the tab is open on, as its field needs it:
+/// the grid's on the cell, or the row panel's in the place of a value.
+pub(super) fn editor_target(
     workspace: &crate::model::Workspace,
     object: &ObjectTab,
     tab: ConnTabId,
+    cell: CellPos,
     hold: bool,
 ) -> Option<cell_editor::Target> {
-    let editor = object.edits.editor.as_ref()?;
     let table = Table::of(workspace, object)?;
-    let column = table.page.columns.get(editor.cell.col)?;
-    let max_chars = match table.class(editor.cell.col) {
+    let column = table.page.columns.get(cell.col)?;
+    let max_chars = match table.class(cell.col) {
         Some(tabletist_db::ColumnClass::Text { max_chars }) => max_chars,
         _ => None,
     };
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
index 2a64660..6978e88 100644
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -25,6 +25,7 @@ pub mod pending_bar;
 pub mod picker;
 pub mod quick_open;
 pub mod review;
+pub mod row_form;
 pub mod row_panel;
 pub mod settings;
 pub mod sidebar;
diff --git a/src/ui/row_form.rs b/src/ui/row_form.rs
new file mode 100644
index 0000000..57575ce
--- /dev/null
+++ b/src/ui/row_form.rs
@@ -0,0 +1,201 @@
+//! The row panel as a row form: what editing adds to the panel for the row
+//! of a table that can be edited. Which of its fields can be edited, the
+//! editor the panel draws in a field's place, and what stands there while
+//! the grid edits that cell. The panel itself (`row_panel`) draws the
+//! fields; the reducer owns every change (`app/editing.rs`).
+
+use egui::{Sense, WidgetInfo, WidgetType, vec2};
+
+use crate::app::App;
+use crate::edit::{Editor, EditorPlace, Lock, Table};
+use crate::i18n::{Locale, gettext};
+use crate::model::{Action, CellPos, ConnTabId, ObjectTab, TabId, Workspace};
+use crate::theme::{Look, Palette};
+use crate::typography::{Text, TextRole};
+use crate::ui::cell_editor::{self, Target};
+use crate::ui::widgets;
+
+/// What the form makes of one field of the row.
+#[derive(Debug, Clone, Copy, PartialEq, Eq)]
+pub enum Part {
+    /// Nothing: the field is read, as in a panel that edits nothing. A SQL
+    /// editor's result, and a row no cell of which can be edited.
+    Read,
+    /// Its value can be edited: it has a pencil, and takes a double-click.
+    Editable,
+    /// It cannot, for a reason of its own.
+    Locked(Lock),
+    /// The grid is editing its cell.
+    InGrid,
+    /// The panel's editor is on it.
+    Editing,
+}
+
+/// The form of the row the panel shows.
+pub struct Form<'a> {
+    /// The tab's editor, where the panel draws it: on a cell of this row.
+    editor: Option<&'a mut Editor>,
+    /// That editor's cell, as its field needs it.
+    target: Option<Target>,
+    /// One per column of the page. Empty where the form has no part in
+    /// the panel.
+    parts: Vec<Part>,
+    /// The column whose field gets the keyboard back, until that field
+    /// takes it.
+    pub focus: Option<usize>,
+    /// What ends the open field (a commit, a cancel, the keyboard going
+    /// elsewhere). The panel queues it ahead of everything else it asks
+    /// for in the frame: its field can be left for a control of the panel
+    /// that is drawn before it, which asks for another editor on the cell
+    /// this one is leaving.
+    pub ending: Vec<Action>,
+}
+
+/// Takes the tab's editor out of the tab for the frame, where the panel
+/// draws it: its field edits the text while the panel reads the rest of
+/// the workspace. [`put_editor`] gives it back when the panel is drawn.
+pub fn take_editor(app: &mut App, tab: ConnTabId, id: TabId) -> Option<Editor> {
+    let object = app.workspace_mut(tab)?.object_tab_mut(id)?;
+    object
+        .edits
+        .editor
+        .take_if(|editor| editor.place == EditorPlace::Panel)
+}
+
+/// Gives back what [`take_editor`] took.
+pub fn put_editor(app: &mut App, tab: ConnTabId, id: TabId, editor: Option<Editor>) {
+    let object = app
+        .workspace_mut(tab)
+        .and_then(|workspace| workspace.object_tab_mut(id));
+    if let (Some(editor), Some(object)) = (editor, object) {
+        object.edits.editor = Some(editor);
+    }
+}
+
+impl<'a> Form<'a> {
+    /// A panel the form has no part in: a SQL editor's result.
+    pub fn none() -> Self {
+        Self {
+            editor: None,
+            target: None,
+            parts: Vec::new(),
+            focus: None,
+            ending: Vec::new(),
+        }
+    }
+
+    /// The form of the page's row `row` of the table `object` shows.
+    /// `editor` is the panel's, taken out of the tab; `hold` says no dialog
+    /// is up, so an open field has the keyboard.
+    pub fn of(
+        workspace: &Workspace,
+        object: &ObjectTab,
+        tab: ConnTabId,
+        row: usize,
+        editor: Option<&'a mut Editor>,
+        hold: bool,
+    ) -> Self {
+        // Only an editor on this row is the panel's to draw.
+        let editor = editor.filter(|editor| editor.cell.row == row);
+        let Some(table) = Table::of(workspace, object) else {
+            return Self::none();
+        };
+        // No cell of the row can be edited: the fields are read as ever,
+        // and the footer says why, once.
+        if table.row_lock(row).is_some() {
+            return Self::none();
+        }
+        let editing = editor.as_ref().map(|editor| editor.cell.col);
+        // The editor still in the tab is the grid's.
+        let in_grid = object
+            .edits
+            .editor
+            .as_ref()
+            .filter(|editor| editor.cell.row == row)
+            .map(|editor| editor.cell.col);
+        let parts = (0..table.page.columns.len())
+            .map(|col| {
+                if editing == Some(col) {
+                    Part::Editing
+                } else if in_grid == Some(col) {
+                    Part::InGrid
+                } else {
+                    match table.lock(CellPos { row, col }) {
+                        Some(lock) => Part::Locked(lock),
+                        None => Part::Editable,
+                    }
+                }
+            })
+            .collect();
+        let target = editor.as_ref().and_then(|editor| {
+            crate::ui::data_view::editor_target(workspace, object, tab, editor.cell, hold)
+        });
+        Self {
+            editor,
+            target,
+            parts,
+            focus: None,
+            ending: Vec::new(),
+        }
+    }
+
+    /// What the form makes of the field of the column `col`.
+    pub fn part(&self, col: usize) -> Part {
+        self.parts.get(col).copied().unwrap_or(Part::Read)
+    }
+
+    /// Draws the panel's editor in its field's place, its text in `role`,
+    /// and queues what its frame came to. Returns whether there was an
+    /// editor to draw.
+    pub fn editing(
+        &mut self,
+        ui: &mut egui::Ui,
+        (tab, id): (ConnTabId, TabId),
+        role: TextRole,
+        skin: (&Look, &Palette, Locale),
+    ) -> bool {
+        let (Some(editor), Some(target)) = (self.editor.as_deref_mut(), &self.target) else {
+            return false;
+        };
+        let cell = editor.cell;
+        let outcome = cell_editor::in_panel(ui, editor, target, role, skin);
+        // The text is noted as typed before anything ends the edit, as for
+        // a cell's field.
+        if outcome.changed {
+            self.ending.push(Action::EditorTyped { tab, id });
+        }
+        if outcome.large {
+            self.ending.push(Action::EditorBreak { tab, id });
+        } else if let Some(then) = outcome.commit {
+            self.ending.push(Action::CommitEdit { tab, id, then });
+        } else if outcome.cancel {
+            self.ending.push(Action::CancelEdit { tab, id });
+        } else if outcome.left {
+            let place = EditorPlace::Panel;
+            self.ending.push(Action::LeaveEdit {
+                tab,
+                id,
+                cell,
+                place,
+            });
+        }
+        true
+    }
+}
+
+/// What stands in a value's place while the grid edits its cell: a dashed
+/// box that says so. One editor is open at a time, and it is there.
+pub fn in_grid(ui: &mut egui::Ui, look: &Look, palette: &Palette, locale: Locale) {
+    let height = if look.terminal { 26.0 } else { 32.0 };
+    let size = vec2(ui.available_width(), height);
+    let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
+    let text = look.label(&gettext(locale, "Editing in the grid…"));
+    response.widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, &text));
+    super::row_panel::dashed(ui, rect, palette.accent);
+    widgets::paint_text(
+        ui,
+        rect.left() + 10.0,
+        rect.center().y,
+        Text::one(look, widgets::secondary(look), &text, palette.accent),
+    );
+}
diff --git a/src/ui/row_panel.rs b/src/ui/row_panel.rs
index 0c150ab..3de5c21 100644
--- a/src/ui/row_panel.rs
+++ b/src/ui/row_panel.rs
@@ -9,6 +9,7 @@ use std::sync::Arc;
 use tabletist_db::{Value, ValueKind};
 
 use crate::app::App;
+use crate::edit::Editor;
 use crate::i18n::gettext;
 use crate::model::{Action, CellPos, ConnTabId, RowFields, Tab, TabId, Workspace};
 use crate::theme::{Icon, Look, Palette};
@@ -16,6 +17,8 @@ use crate::typography::{Text, TextRole};
 use crate::ui::focus::{self, Region};
 use crate::ui::format;
 use crate::ui::json_view;
+use crate::ui::keys::consume_press;
+use crate::ui::row_form::{self, Form, Part};
 use crate::ui::widgets;
 
 /// The panel's width when it opens: macOS 344 and its 1 pt rule, terminal
@@ -251,10 +254,33 @@ fn source(workspace: &Workspace, id: TabId, locale: crate::i18n::Locale) -> Opti
 
 /// Draws the panel for the tab `id`: an object tab, or a SQL editor.
 pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, id: TabId) {
+    // The editor the panel draws is out of its tab while the panel is
+    // drawn: the field edits its text, and everything else is read.
+    let mut editor = row_form::take_editor(app, tab, id);
+    // The field an edit ended in gets the keyboard back, once.
+    let focus = app
+        .workspace_mut(tab)
+        .and_then(|workspace| workspace.object_tab_mut(id))
+        .and_then(|object| object.focus_field.take());
+    draw(app, ui, (tab, id), editor.as_mut(), focus);
+    row_form::put_editor(app, tab, id, editor);
+}
+
+/// The panel itself. `editor` is the tab's editor where the panel draws
+/// it, and `focus` the column whose field takes the keyboard.
+fn draw(
+    app: &mut App,
+    ui: &mut egui::Ui,
+    (tab, id): (ConnTabId, TabId),
+    editor: Option<&mut Editor>,
+    focus: Option<usize>,
+) {
     let locale = app.locale;
     let palette = app.palette;
     let look = app.look;
     let value_tags = app.settings.value_tags;
+    // An editor that is open has the keyboard, unless a dialog has it.
+    let hold = app.dialog.is_none();
     // `za` asked to fold the documents.
     let fold = app.workspace_mut(tab).is_some_and(|workspace| {
         let asked = workspace.fold_documents == Some(id);
@@ -270,6 +296,8 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, id: TabId) {
         return;
     };
     let mut actions = Vec::new();
+    // What ends the panel's open field, queued ahead of the rest.
+    let mut ending = Vec::new();
     let panel = Id::new(("row-panel", tab.0));
     let range = width_range(ui.available_width());
     // egui remembers the width it last drew, which a small window cuts
@@ -340,6 +368,14 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, id: TabId) {
             };
             // A part of the window to step to once it has a row to show.
             focus::region(ui, Region::Panel, full);
+            // What editing adds, for a table's row that can be edited.
+            let mut form = match workspace.tab(id) {
+                Some(Tab::Object(object)) => {
+                    Form::of(workspace, object, tab, cell.row, editor, hold)
+                }
+                _ => Form::none(),
+            };
+            form.focus = focus;
             let structure = source.structure;
             let info = |name: &str| {
                 let key =
@@ -707,6 +743,7 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, id: TabId) {
                                                 &info,
                                                 tag_of(*col, value),
                                                 skin,
+                                                &mut form,
                                                 &mut actions,
                                             );
                                             was(ui, *col, skin);
@@ -744,6 +781,7 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, id: TabId) {
                                         &info,
                                         tag_of(*col, value),
                                         skin,
+                                        &mut form,
                                         &mut actions,
                                     );
                                     was(ui, *col, skin);
@@ -775,6 +813,7 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, id: TabId) {
                                         &info,
                                         tag_of(*col, value),
                                         skin,
+                                        &mut form,
                                         &mut actions,
                                     );
                                     was(ui, *col, skin);
@@ -800,6 +839,7 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, id: TabId) {
                         }
                     }
                 });
+            ending.append(&mut form.ending);
         });
     // The edge was dragged: that is the width wanted from now on. egui
     // stores a width only when the drag is over, so a width that did not
@@ -810,6 +850,7 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, id: TabId) {
     {
         ui.data_mut(|data| data.insert_persisted(panel.with("wanted"), after));
     }
+    app.actions.extend(ending);
     app.actions.extend(actions);
 }
 
@@ -858,7 +899,9 @@ fn was(ui: &mut egui::Ui, col: usize, skin: FieldSkin<'_>) {
 const MARK: f32 = 6.0;
 
 /// One field: its label (with a copy button, or a document's controls, and
-/// the pending mark when its cell is pending), then its value.
+/// the pending mark when its cell is pending), then its value. Where the
+/// row's form lets the value be edited, the label line has a pencil too,
+/// and the value's place holds the editor while it is open.
 #[allow(clippy::too_many_arguments)] // one call site per layout
 fn field(
     ui: &mut egui::Ui,
@@ -871,15 +914,16 @@ fn field(
     info: &FieldInfo,
     tag: Option<crate::ui::grid::Style>,
     skin: FieldSkin<'_>,
+    form: &mut Form<'_>,
     actions: &mut Vec<Action>,
 ) {
     let FieldSkin {
         look,
         palette,
         locale,
-        fold,
         texts,
         copy_key,
+        ..
     } = skin;
     // The request whose answer holds the row: the same row of another
     // page or another result keeps no folds and nothing expanded.
@@ -919,10 +963,13 @@ fn field(
         text.clone()
     };
     response.widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, &name));
-    // Cut at the column's edge, as the field's own width allows, and
-    // before the mark of a pending one.
+    let part = form.part(col);
+    let editable = part == Part::Editable;
+    // Cut at the column's edge, as the field's own width allows, before
+    // the mark of a pending one and the pencil of one that can be edited.
     let mark = if pending { 6.0 + MARK } else { 0.0 };
-    let room = line.width() - 30.0 - mark;
+    let pencil_room = if editable { PENCIL + 2.0 } else { 0.0 };
+    let room = line.width() - 30.0 - mark - pencil_room;
     let shown = crate::ui::grid::ellipsize(&text, room, false, |text| {
         label_role.width(ui.ctx(), look.faces, text)
     });
@@ -947,7 +994,19 @@ fn field(
     }
     let column_name = format::display_safe(&column.name);
     let copy_label = format!("{} {column_name}", gettext(locale, "Copy"));
+    let edit_label = format!("{} {column_name}", gettext(locale, "Edit"));
     let hovered = ui.rect_contains_pointer(line.expand2(vec2(16.0, 30.0)));
+    // The pencil of a value that can be edited, at `right`: beside Copy,
+    // and shown as Copy is.
+    let pencil_at = |ui: &mut egui::Ui, right: f32, size: f32, shown: bool| {
+        let place = Rect::from_min_size(
+            pos2(right - size, line.center().y - size / 2.0),
+            vec2(size, size),
+        );
+        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(place));
+        caption_button(&mut child, Icon::Pencil, &edit_label, shown, look, palette)
+    };
+    let mut pencil = None;
     if let Some(doc) = &doc {
         // A document's own controls: fold everything, and copy.
         let copy = Rect::from_min_size(
@@ -972,17 +1031,29 @@ fn field(
             } else {
                 hints.add(role, "fold", palette.dim)
             };
-            widgets::paint_text_right(ui, line.right(), line.center().y, hints);
-            if copy_button(&mut child, &copy_label, false, look, palette).clicked() {
+            let hints = widgets::paint_text_right(ui, line.right(), line.center().y, hints);
+            if caption_button(&mut child, Icon::Copy, &copy_label, false, look, palette).clicked() {
                 ui.ctx().copy_text(format::plain_text(value));
             }
+            // The pencil stands before the hints.
+            if editable {
+                let right = line.right() - hints - 6.0;
+                pencil = Some(pencil_at(ui, right, PENCIL, hovered));
+            }
         } else {
-            if copy_button(&mut child, &copy_label, true, look, palette).clicked() {
+            if caption_button(&mut child, Icon::Copy, &copy_label, true, look, palette).clicked() {
                 ui.ctx().copy_text(format::plain_text(value));
             }
+            // The pencil beside Copy, bordered as it is: 4 between them.
+            let mut left = copy.left();
+            if editable {
+                pencil = Some(pencil_at(ui, left - 4.0, 26.0, true));
+                left -= 4.0 + 26.0;
+            }
             let id = Id::new(("row-panel-json", tab, tab_id, request, row, col));
-            // "Collapse all": a 24 pt button 6 at its sides, 4 before copy.
-            let link_right = copy.left() - 4.0 - 6.0;
+            // "Collapse all": a 24 pt button 6 at its sides, 4 before the
+            // buttons.
+            let link_right = left - 4.0 - 6.0;
             json_view::fold_all_link(
                 ui,
                 id,
@@ -1000,9 +1071,12 @@ fn field(
             vec2(22.0, 22.0),
         );
         let mut child = ui.new_child(egui::UiBuilder::new().max_rect(copy));
-        if copy_button(&mut child, &copy_label, hovered, look, palette).clicked() {
+        if caption_button(&mut child, Icon::Copy, &copy_label, hovered, look, palette).clicked() {
             ui.ctx().copy_text(format::plain_text(value));
         }
+        if editable {
+            pencil = Some(pencil_at(ui, copy.left() - 2.0, PENCIL, hovered));
+        }
     }
     ui.add_space(if doc.is_some() {
         if look.terminal { 8.0 } else { 6.0 }
@@ -1011,10 +1085,174 @@ fn field(
     } else {
         3.0
     });
-    if value.is_null() {
-        crate::ui::grid::null_label(ui, look, palette);
+    // Values in the data face at 13; the terminal's timestamps at 12.
+    let role = if matches!(column.kind, ValueKind::Json | ValueKind::Binary) {
+        TextRole::pick(look, TextRole::MonoSecondary, TextRole::OSecondary)
+    } else if look.terminal && column.kind == ValueKind::Temporal {
+        TextRole::OSecondary
+    } else {
+        TextRole::pick(look, TextRole::InspectorValue, TextRole::OBody)
+    };
+    match part {
+        // The editor, in the value's place.
+        Part::Editing if form.editing(ui, (tab, tab_id), role, (look, palette, locale)) => return,
+        Part::InGrid => {
+            row_form::in_grid(ui, look, palette, locale);
+            return;
+        }
+        Part::Editing | Part::Read | Part::Editable | Part::Locked(_) => {}
+    }
+    let read = Reading {
+        doc,
+        formatted,
+        name_id,
+        role,
+        editable,
+    };
+    let shown = value_of(
+        ui, tab, tab_id, row, col, column, value, info, tag, skin, read, actions,
+    );
+    if !editable {
         return;
     }
+    // What starts an edit of the value: its pencil, a double-click on it,
+    // and Enter or F2 while its text has the keyboard (the terminal look
+    // edits with its own letters).
+    let mut edit = pencil.as_ref().is_some_and(egui::Response::clicked);
+    // Enter on the pencil is the button's press. F2 edits from it too: a
+    // value with no text of its own (a NULL, a document) has the keyboard
+    // there.
+    if let Some(pencil) = &pencil
+        && !look.terminal
+        && pencil.has_focus()
+    {
+        edit |= ui.input_mut(|input| consume_press(input, egui::Modifiers::NONE, egui::Key::F2));
+    }
+    if let Some(place) = shown.place {
+        let over = ui.rect_contains_pointer(place);
+        if over {
+            // A field's border: the value can be edited.
+            let color = if look.terminal {
+                palette.accent
+            } else {
+                palette.border
+            };
+            ui.painter().rect_stroke(
+                outline(place),
+                CornerRadius::same(look.radius),
+                Stroke::new(1.0, color),
+                StrokeKind::Inside,
+            );
+        }
+        let twice = ui.input(|input| {
+            input
+                .pointer
+                .button_double_clicked(egui::PointerButton::Primary)
+        });
+        edit |= over && twice;
+    }
+    if let Some(text) = &shown.text
+        && !look.terminal
+        && text.has_focus()
+    {
+        edit |= ui.input_mut(|input| {
+            consume_press(input, egui::Modifiers::NONE, egui::Key::Enter)
+                || consume_press(input, egui::Modifiers::NONE, egui::Key::F2)
+        });
+    }
+    if edit {
+        actions.push(Action::EditField {
+            tab,
+            id: tab_id,
+            cell: CellPos { row, col },
+        });
+    }
+    // An edit of this field ended: the keyboard is on it again, on its
+    // text, or on its pencil where the value has none (a NULL, a document).
+    if form.focus == Some(col) {
+        form.focus = None;
+        if let Some(back) = shown.text.as_ref().or(pencil.as_ref()) {
+            back.request_focus();
+            back.scroll_to_me(None);
+            // What shows that it has the keyboard (the pencil, a caret)
+            // was drawn before it had it: the next frame draws it.
+            ui.ctx().request_repaint();
+        }
+    }
+}
+
+/// The pencil of a field that can be edited: 22 pt, as Copy is.
+const PENCIL: f32 = 22.0;
+
+/// Where a field's outline is, round the place of its value: the value is
+/// flush with its label, and a field's border stands clear of its text.
+fn outline(place: Rect) -> Rect {
+    place.expand2(vec2(6.0, 3.0))
+}
+
+/// What a field's value is read from, besides the value itself.
+struct Reading<'a> {
+    /// The value as a tree, when it holds a document.
+    doc: Option<Arc<json_view::Doc>>,
+    /// The value's text, formatted by the app.
+    formatted: Option<&'a format::FieldText>,
+    /// The label the value is named by.
+    name_id: Id,
+    /// The role its text is drawn in.
+    role: TextRole,
+    /// Whether the row's form lets it be edited.
+    editable: bool,
+}
+
+/// What a field's value came to on screen.
+#[derive(Default)]
+struct Shown {
+    /// Where a double-click edits it: its text, the NULL mark, the stand-in
+    /// of an empty or a blank text. None for what has its own clicks (a
+    /// tree, a list) or cannot be edited at all.
+    place: Option<Rect>,
+    /// Its text, which takes a caret and the keyboard.
+    text: Option<egui::Response>,
+}
+
+/// A field's value, under its label: read, selected and copied from.
+#[allow(clippy::too_many_arguments)] // the field's own, passed on
+fn value_of(
+    ui: &mut egui::Ui,
+    tab: ConnTabId,
+    tab_id: TabId,
+    row: usize,
+    col: usize,
+    column: &tabletist_db::ColumnMeta,
+    value: &Value,
+    info: &FieldInfo,
+    tag: Option<crate::ui::grid::Style>,
+    skin: FieldSkin<'_>,
+    read: Reading<'_>,
+    actions: &mut Vec<Action>,
+) -> Shown {
+    let FieldSkin {
+        look,
+        palette,
+        locale,
+        fold,
+        texts,
+        ..
+    } = skin;
+    let Reading {
+        doc,
+        formatted,
+        name_id,
+        role,
+        editable,
+    } = read;
+    let request = texts.and_then(|texts| texts.request);
+    let column_name = format::display_safe(&column.name);
+    let mut shown = Shown::default();
+    if value.is_null() {
+        shown.place = Some(crate::ui::grid::null_label(ui, look, palette).rect);
+        return shown;
+    }
     if let Some(doc) = doc {
         if fold {
             json_view::toggle_fold_all(
@@ -1042,10 +1280,10 @@ fn field(
                     json_view::show(ui, id, &doc, &column_name, locale, palette, look);
                 });
         }
-        return;
+        return shown;
     }
     let Some(formatted) = formatted else {
-        return;
+        return shown;
     };
     let say = |text: &'static str| look.label(&gettext(locale, text));
     match value {
@@ -1060,18 +1298,18 @@ fn field(
                     col,
                 });
             }
-            return;
+            return shown;
         }
         Value::Text(text) => {
             let marks = crate::ui::grid::marks(ui.ctx(), look);
-            if let Some(shown) = format::blank_text(text, marks) {
+            if let Some(blank) = format::blank_text(text, marks) {
                 let note = if text.is_empty() {
                     say("empty string")
                 } else {
                     say("whitespace only")
                 };
-                stand_in(ui, &shown, &note, look, palette);
-                return;
+                shown.place = Some(stand_in(ui, &blank, &note, look, palette));
+                return shown;
             }
             if listed(text, column)
                 && let Some(array) = format::array_items(text)
@@ -1081,7 +1319,7 @@ fn field(
                 } else {
                     elements(ui, &array, look, palette, locale);
                 }
-                return;
+                return shown;
             }
         }
         Value::Null | Value::Bool(_) | Value::Int(_) | Value::Float(_) => {}
@@ -1092,18 +1330,10 @@ fn field(
     // known once it is laid out at the width it gets.
     let mut tall = false;
     let long = formatted.full.is_some();
-    let shown = match &formatted.full {
+    let read = match &formatted.full {
         Some(full) if expanded => full,
         _ => &formatted.short,
     };
-    // Values in the data face at 13; the terminal's timestamps at 12.
-    let role = if matches!(column.kind, ValueKind::Json | ValueKind::Binary) {
-        TextRole::pick(look, TextRole::MonoSecondary, TextRole::OSecondary)
-    } else if look.terminal && column.kind == ValueKind::Temporal {
-        TextRole::OSecondary
-    } else {
-        TextRole::pick(look, TextRole::InspectorValue, TextRole::OBody)
-    };
     // The follow link sits at the value's right.
     let follow = info.target.as_ref().map(|target| {
         if look.terminal {
@@ -1146,11 +1376,11 @@ fn field(
         let room = (ui.available_width() - link).max(24.0);
         ui.allocate_ui(vec2(room, 0.0), |ui| {
             let mut layouter = crate::typography::layouter(look, role, color);
-            tall = layouter(ui, &shown.as_str(), room).rows.len() > CLAMP_ROWS;
+            tall = layouter(ui, &read.as_str(), room).rows.len() > CLAMP_ROWS;
             let mut edit = |ui: &mut egui::Ui| {
                 let value = ui
                     .add(
-                        TextEdit::multiline(&mut shown.as_str())
+                        TextEdit::multiline(&mut read.as_str())
                             .font(role.font_id(look.faces))
                             // Flush with the field name above.
                             .frame(egui::Frame::NONE)
@@ -1161,8 +1391,18 @@ fn field(
                     )
                     .labelled_by(name_id);
                 // A value to read and select, not a field: its caret says
-                // where the keyboard is.
-                crate::ui::focus::hint(ui, &value, value.rect, crate::ui::focus::Ring::Own);
+                // where the keyboard is. One that can be edited wears a
+                // field's ring, where its outline is.
+                let ring = if editable {
+                    focus::Ring::Field {
+                        radius: look.radius,
+                    }
+                } else {
+                    focus::Ring::Own
+                };
+                focus::hint(ui, &value, outline(value.rect), ring);
+                shown.place = Some(value.rect);
+                shown.text = Some(value);
             };
             if tall && !expanded {
                 // The first lines, the last of them fading out (macOS):
@@ -1173,6 +1413,8 @@ fn field(
                 let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect));
                 child.set_clip_rect(rect.intersect(ui.clip_rect()));
                 edit(&mut child);
+                // The lines that show are the value's place.
+                shown.place = Some(rect);
                 if !look.terminal {
                     let last =
                         Rect::from_min_max(pos2(rect.left(), rect.bottom() - line), rect.max);
@@ -1218,6 +1460,7 @@ fn field(
             ui.data_mut(|data| data.insert_temp(expanded_id, !expanded));
         }
     }
+    shown
 }
 
 /// The lines of a long value the panel shows before "Show all".
@@ -1239,7 +1482,7 @@ fn fade(ui: &egui::Ui, rect: Rect, color: egui::Color32) {
 
 /// A value with nothing to see: what the grid writes for it (`''`, a mark
 /// for each space, `{}`), and in words what that is.
-fn stand_in(ui: &mut egui::Ui, shown: &str, note: &str, look: &Look, palette: &Palette) {
+fn stand_in(ui: &mut egui::Ui, shown: &str, note: &str, look: &Look, palette: &Palette) -> Rect {
     ui.horizontal(|ui| {
         ui.spacing_mut().item_spacing.x = 6.0;
         let role = crate::ui::grid::data_role(look);
@@ -1249,7 +1492,9 @@ fn stand_in(ui: &mut egui::Ui, shown: &str, note: &str, look: &Look, palette: &P
         Text::one(look, widgets::secondary(look), note, palette.secondary)
             .layout(ui.ctx())
             .label(ui);
-    });
+    })
+    .response
+    .rect
 }
 
 /// The most elements of an array the panel lists.
@@ -1382,11 +1627,12 @@ fn singular(table: &str) -> &str {
         .unwrap_or(table)
 }
 
-/// A field's copy button: bordered next to a document, otherwise a bare
-/// icon shown when the pointer is near. It is always there for keyboards
-/// and screen readers.
-fn copy_button(
+/// A button of a field's label line (Copy, the pencil): bordered next to a
+/// document, otherwise a bare icon shown when the pointer is near. It is
+/// always there for keyboards and screen readers.
+fn caption_button(
     ui: &mut egui::Ui,
+    icon: Icon,
     label: &str,
     shown: bool,
     look: &Look,
@@ -1408,8 +1654,7 @@ fn copy_button(
                 StrokeKind::Inside,
             );
         }
-        Icon::Copy
-            .image(palette.secondary, 13.0)
+        icon.image(palette.secondary, 13.0)
             .paint_at(ui, Rect::from_center_size(rect.center(), vec2(13.0, 13.0)));
     }
     response.on_hover_text(label)
@@ -1622,7 +1867,7 @@ fn editing_footer(
 }
 
 /// A dashed 1 pt outline round `rect`.
-fn dashed(ui: &egui::Ui, rect: Rect, color: egui::Color32) {
+pub(super) fn dashed(ui: &egui::Ui, rect: Rect, color: egui::Color32) {
     let stroke = Stroke::new(1.0, color);
     let rect = rect.shrink(0.5);
     for side in [
````

- [ ] **Step 4: Run the tests**

Run the command of step 2.
Expected: PASS, all twelve.

- [ ] **Step 5: Look at it**

No scene is committed in this task (task 7 adds them). To look now, append a throwaway `#[test] #[ignore]` to `src/shots.rs` built on `both(name, |harness| ..)`, which opens the panel's editor with `Action::EditField { tab, id, cell: CellPos { row: 4, col: KIND } }` after `editable(harness)`, run it with `~/.cargo/bin/cargo test --locked --features shots --lib shots::<name> -- --ignored --exact`, read the PNGs in `target/shots/`, and remove the test again. The field stands in the value's place under its label, as wide as the panel's column; the other fields read as before.

- [ ] **Step 6: Run the four checks, then commit**

```bash
git add src/ui/row_form.rs src/ui/mod.rs src/ui/cell_editor.rs src/ui/row_panel.rs src/ui/data_view.rs
git commit -S -m "Edit a field of the row panel in place" -m "A value that can be edited has a pencil beside Copy and takes a double-click, and Enter or F2 with a caret in it. The tab's editor is then drawn in the value's place. Enter commits and stays, Esc drops, and the terminal's Esc keeps. A field whose cell the grid is editing says so." -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 5: Locks, and the footer

**Files:**
- Modify: `src/ui/row_form.rs` (`row_lock`, `Form::refused`)
- Modify: `src/ui/row_panel.rs` (`field`, `editing_footer`, `draw`)
- Modify: `src/ui/mod.rs` (tests)

What cannot be edited says so:

- **A field with a lock of its own** (the key, a computed column, a binary value, one over 256 KiB) has a small lock after its label, always drawn, named "Locked: <reason>" for a screen reader and showing the reason under the pointer.
- **An edit asked for on such a field** (a double-click on its value, Enter or F2 with a caret in it, `EditRow` on a row with no editable field) opens nothing. The reducer records why, with the place; the panel writes the reason under the value on macOS and Windows, until the selection moves, and brings it into view once. The terminal's mode line already says it.
- **A row no field of which can be edited** (`Table::row_lock`) keeps its fields as they were, and the footer's note says why, in the grid's own words: `cell_editor::lock_text`, and `workspace::lock_line` for the terminal's lower case.
- **The footer.** Edit is a button now: it asks for `EditRow`, and is disabled with the row's reason where the row cannot be edited. Duplicate and Delete stay disabled, with "Duplicating and deleting rows arrive in a later version".

Three existing tests change, because what they pinned was the footer of a panel that edited nothing:

- `the_terminal_footer_fits_a_narrow_row_panel`: its note was "read-only connection · editing arrives in a later version" and is now the reason, "this connection opens read-only". That is shorter, so the window that cuts it is 700 wide, not 1000.
- `only_a_read_only_connection_carries_the_mark`: the note's new words, which a screen reader now hears too, and what the note says once the connection is writable (the fixture never answers the structure: "The table's structure is still loading").
- `the_row_panel_stays_read_only_with_a_pending_value` is replaced by `the_row_panels_edit_works_and_duplicate_and_delete_wait`: Edit is enabled on a row that can be edited, with a value pending as without, and the other two wait.

- [ ] **Step 1: Write the failing tests**

````diff
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
index 6978e88..7bcfa93 100644
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -5859,7 +5859,8 @@ mod tests {
         let tab = with_page(&mut harness);
         focus_grid(&mut harness, tab);
         harness.click("Row 1");
-        let note = "read-only connection · editing arrives in a later version";
+        // Why the row cannot be edited: the fixture connection is read-only.
+        let note = "this connection opens read-only";
         let pieces = |harness: &mut Harness| -> Vec<String> {
             harness.settle();
             let painted = harness.painted.iter();
@@ -5871,13 +5872,13 @@ mod tests {
         assert!(wide.iter().any(|piece| piece == note));
         assert!(wide.iter().any(|piece| piece == "yy p duplicate"));
         // A narrower window cuts the panel: the note is cut to fit it.
-        harness.size.x = 1000.0;
+        harness.size.x = 700.0;
         let narrow = pieces(&mut harness);
         assert!(!narrow.iter().any(|piece| piece == note), "{narrow:?}");
         assert!(
             narrow
                 .iter()
-                .any(|piece| piece.starts_with("read-only connection") && piece.ends_with('…')),
+                .any(|piece| piece.starts_with("this connection") && piece.ends_with('…')),
             "{narrow:?}"
         );
         // And a cell too narrow for its words keeps its key alone.
@@ -5910,34 +5911,35 @@ mod tests {
             // The fixture connection is read-only.
             let (named, painted) = marks(&mut harness);
             if look.terminal {
-                assert_eq!(named, ["read-only"], "{}", look.name);
-                // The bar's tag, the status line's, and the row panel's
-                // note.
+                // The row panel's note, which is why the row cannot be
+                // edited, and the bar's tag.
+                let marks = ["this connection opens read-only", "read-only"];
+                assert_eq!(named, marks, "{}", look.name);
+                // Those two, and the status line's tag.
                 assert_eq!(
                     painted,
-                    [
-                        "read-only",
-                        "read-only",
-                        "read-only connection · editing arrives in a later version"
-                    ],
+                    ["read-only", "read-only", "this connection opens read-only"],
                     "{}",
                     look.name
                 );
             } else {
-                // The bar's pill and the footer.
-                let marks = ["Read-only", "1 row selected · read-only"];
+                // The bar's pill, the row panel's note and the footer.
+                let marks = [
+                    "Read-only",
+                    "This connection opens read-only",
+                    "1 row selected · read-only",
+                ];
                 assert_eq!(named, marks, "{}", look.name);
             }
             harness.app.workspace_mut(tab).unwrap().access = tabletist_db::Access::Writable;
             let (named, painted) = marks(&mut harness);
             assert_eq!(named, Vec::<String>::new(), "{}", look.name);
             assert_eq!(painted, Vec::<String>::new(), "{}", look.name);
-            if look.terminal {
-                // The row panel's note says the rest of what it said.
-                let note = "editing arrives in a later version";
-                let mut pieces = harness.painted.iter();
-                assert!(pieces.any(|(text, _)| text == note), "{}", look.name);
-            }
+            // The row panel's note says what holds the row now: the
+            // fixture never answered the table's structure.
+            let note = look.label("The table's structure is still loading");
+            let mut pieces = harness.painted.iter();
+            assert!(pieces.any(|(text, _)| *text == note), "{}", look.name);
         }
     }
 
@@ -16779,35 +16781,223 @@ mod tests {
         }
     }
 
+    /// The buttons under the row panel's row, as the look names them, with
+    /// whether each can be pressed.
+    fn footer_buttons(harness: &mut Harness, look: &Look) -> Vec<(&'static str, bool)> {
+        let controls = if look.terminal {
+            ["e edit", "yy p duplicate", "dd delete"]
+        } else {
+            ["Edit", "Duplicate", "Delete"]
+        };
+        let tree = harness.settle();
+        let enabled = |name: &str| {
+            let mut nodes = tree.nodes.iter();
+            let node = nodes.find(|(_, node)| node.label() == Some(name));
+            !node.unwrap_or_else(|| panic!("no {name}")).1.is_disabled()
+        };
+        controls.map(|name| (name, enabled(name))).to_vec()
+    }
+
     #[test]
-    fn the_row_panel_stays_read_only_with_a_pending_value() {
+    fn the_row_panels_edit_works_and_duplicate_and_delete_wait() {
         for look in Look::ALL {
             let (mut harness, tab, id) = editable_in(look);
             select(&mut harness, tab, id, (1, 1));
-            // The editing controls under the row, as the look names them.
-            let controls = if look.terminal {
-                ["e edit", "yy p duplicate", "dd delete"]
-            } else {
-                ["Edit", "Duplicate", "Delete"]
-            };
-            let footer = |harness: &mut Harness| {
-                let tree = harness.settle();
-                let mut names: Vec<String> = controls
+            let pressable = |harness: &mut Harness| {
+                let buttons = footer_buttons(harness, &look);
+                buttons
                     .into_iter()
-                    .filter(|name| {
-                        tree.nodes
-                            .iter()
-                            .any(|(_, node)| node.label() == Some(name) && node.is_disabled())
-                    })
-                    .map(str::to_owned)
-                    .collect();
-                names.sort();
-                names
+                    .map(|(_, enabled)| enabled)
+                    .collect::<Vec<_>>()
             };
-            let before = footer(&mut harness);
-            assert_eq!(before.len(), 3, "{}", look.name);
+            assert_eq!(
+                pressable(&mut harness),
+                [true, false, false],
+                "{}",
+                look.name
+            );
+            // With a value pending as without.
             make_pending(&mut harness, tab, id, (1, 1), "bob@example.com");
-            assert_eq!(footer(&mut harness), before, "{}", look.name);
+            assert_eq!(
+                pressable(&mut harness),
+                [true, false, false],
+                "{}",
+                look.name
+            );
+            // Edit edits the row in the panel: the selected cell's field.
+            let edit = footer_buttons(&mut harness, &look)[0].0;
+            harness.click(edit);
+            assert_eq!(
+                form_editor(&harness, tab, id),
+                Some(((1, 1), true)),
+                "{}",
+                look.name
+            );
+            assert_eq!(
+                editor_text(&harness, tab, id).as_deref(),
+                Some("bob@example.com"),
+                "{}",
+                look.name
+            );
+            // Pressed while a field is edited, it leaves that field open
+            // with its text: the field is left for the button, which asks
+            // for the same cell.
+            type_text(&mut harness, "!");
+            let tree = harness.settle();
+            let button = crate::testing::bounds(&tree, edit, egui::accesskit::Role::Button)
+                .expect("the button");
+            click_at(&mut harness, button.center());
+            assert_eq!(
+                form_editor(&harness, tab, id),
+                Some(((1, 1), true)),
+                "{}",
+                look.name
+            );
+            assert_eq!(
+                editor_text(&harness, tab, id).as_deref(),
+                Some("bob@example.com!"),
+                "{}",
+                look.name
+            );
+            // On the key's cell, which is locked, it takes the row's first
+            // field that can be edited.
+            harness.press(Key::Escape, Modifiers::NONE);
+            select(&mut harness, tab, id, (2, 0));
+            harness.click(edit);
+            assert_eq!(
+                form_editor(&harness, tab, id),
+                Some(((2, 1), true)),
+                "{}",
+                look.name
+            );
+        }
+    }
+
+    #[test]
+    fn the_footer_says_why_a_row_cannot_be_edited_and_edit_waits() {
+        for look in Look::ALL {
+            // A read-only connection: the reason is the grid's own.
+            let mut harness = Harness::new();
+            harness.set_look(look);
+            harness.connect_fake_as(true);
+            harness.click("users");
+            harness.answer_structure(crate::testing::fixture_structure());
+            harness.answer_rows(crate::testing::page(5, false));
+            harness.click("Row 2");
+            let why = look.label("This connection opens read-only");
+            assert!(
+                painted(&harness, &why),
+                "{}: {:?}",
+                look.name,
+                harness.painted
+            );
+            assert!(harness.has(&why), "{}", look.name);
+            let buttons = footer_buttons(&mut harness, &look);
+            assert!(buttons.iter().all(|(_, enabled)| !enabled), "{}", look.name);
+            // A view, on a connection that can write: its own reason,
+            // and no field of it offers an edit.
+            let mut harness = Harness::new();
+            harness.set_look(look);
+            let tab = harness.connect_fake_as(false);
+            harness.app.apply(Action::OpenObject {
+                tab,
+                object: tabletist_db::ObjectRef::new("main", "users"),
+                kind: tabletist_db::ObjectKind::View,
+                pin: true,
+            });
+            harness.answer_structure(crate::testing::fixture_structure());
+            harness.answer_rows(crate::testing::page(5, false));
+            let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
+            select(&mut harness, tab, id, (1, 1));
+            let why = look.label("Views cannot be edited");
+            assert!(
+                painted(&harness, &why),
+                "{}: {:?}",
+                look.name,
+                harness.painted
+            );
+            assert!(!has_button(&mut harness, "Edit email"), "{}", look.name);
+            assert!(!footer_buttons(&mut harness, &look)[0].1, "{}", look.name);
+            // A table that can be edited has no note, until a save locks
+            // every cell of it for a while.
+            let (mut harness, tab, id) = editable_in(look);
+            select(&mut harness, tab, id, (1, 1));
+            let saving = look.label("A save is running");
+            assert!(!painted(&harness, &saving), "{}", look.name);
+            make_pending(&mut harness, tab, id, (1, 1), "bob@example.com");
+            harness.app.apply(Action::WriteEdits { tab, id });
+            harness.settle();
+            assert!(
+                painted(&harness, &saving),
+                "{}: {:?}",
+                look.name,
+                harness.painted
+            );
+            let buttons = footer_buttons(&mut harness, &look);
+            assert!(buttons.iter().all(|(_, enabled)| !enabled), "{}", look.name);
+            // And no field of the row offers an edit meanwhile.
+            assert!(!has_button(&mut harness, "Edit email"), "{}", look.name);
+            harness.answer_written(Ok(written_row("bob@example.com")));
+            harness.settle();
+            assert!(!painted(&harness, &saving), "{}", look.name);
+            assert!(footer_buttons(&mut harness, &look)[0].1, "{}", look.name);
+            assert!(has_button(&mut harness, "Edit email"), "{}", look.name);
+        }
+    }
+
+    #[test]
+    fn a_locked_field_wears_a_lock_and_says_why_when_its_edit_is_asked_for() {
+        for look in Look::ALL {
+            let (mut harness, tab, id) = form_row(look, 1);
+            let palette = harness.app.palette;
+            let why = "Part of the row's key";
+            // The lock is there for a screen reader, after the label; the
+            // fields that can be edited have none.
+            assert!(harness.has(&format!("Locked: {why}")), "{}", look.name);
+            let tree = harness.settle();
+            let locks = crate::testing::labels(&tree)
+                .into_iter()
+                .filter(|label| label.starts_with("Locked: "))
+                .count();
+            assert_eq!(locks, 1, "{}", look.name);
+            // Nothing says why until an edit of it is asked for.
+            let said = look.label(why);
+            assert!(!painted(&harness, &said), "{}", look.name);
+            let at = panel_text(&harness, "2").center();
+            click_at(&mut harness, at);
+            click_at(&mut harness, at);
+            let now = edits(&harness, tab, id);
+            assert!(now.editor.is_none(), "{}", look.name);
+            assert_eq!(
+                now.why,
+                Some((CellPos { row: 1, col: 0 }, crate::edit::Lock::KeyColumn)),
+                "{}",
+                look.name
+            );
+            assert_eq!(times_painted(&harness, &said), 1, "{}", look.name);
+            let note = harness.painted_rect(&said).unwrap();
+            if look.terminal {
+                // In the mode line, at the window's foot, as for a cell.
+                assert!(note.top() > harness.size.y - 40.0, "{note:?}");
+            } else {
+                // Under the field in the panel, and not at the grid's cell.
+                let value = panel_text(&harness, "2");
+                assert!(note.top() >= value.bottom() - 1.0, "{}", look.name);
+                assert!((note.left() - value.left()).abs() < 2.0, "{}", look.name);
+                assert!(painted_in(&harness, &said, palette.secondary));
+            }
+            // Until the selection moves.
+            select(&mut harness, tab, id, (2, 0));
+            assert!(!painted(&harness, &said), "{}", look.name);
+            // Asked for in the grid, it is said at the cell, and the panel
+            // does not repeat it.
+            if !look.terminal {
+                open_editor(&mut harness, tab, id, (2, 0));
+                assert_eq!(times_painted(&harness, &said), 1, "{}", look.name);
+                let note = harness.painted_rect(&said).unwrap();
+                let value = panel_text(&harness, "3");
+                assert!(note.right() < value.left(), "{}", look.name);
+            }
         }
     }
 
````

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked --lib -- the_terminal_footer_fits only_a_read_only_connection the_row_panels_edit_works the_footer_says_why a_locked_field_wears`
Expected: all five FAIL on assertions. The three that were changed look for the new note (`assertion failed: wide.iter().any(|piece| piece == note)`, and a list of marks that still lacks "This connection opens read-only"); `the_row_panels_edit_works_and_duplicate_and_delete_wait` finds Edit disabled; the two new ones find no lock and no reason.

- [ ] **Step 3: Draw the lock, the reason and the footer**

````diff
diff --git a/src/ui/row_form.rs b/src/ui/row_form.rs
index 57575ce..ce7af2f 100644
--- a/src/ui/row_form.rs
+++ b/src/ui/row_form.rs
@@ -40,6 +40,10 @@ pub struct Form<'a> {
     /// One per column of the page. Empty where the form has no part in
     /// the panel.
     parts: Vec<Part>,
+    /// The column whose edit was asked for in the panel and refused: its
+    /// field says why under its value. The terminal look says it in its
+    /// mode line.
+    refused: Option<usize>,
     /// The column whose field gets the keyboard back, until that field
     /// takes it.
     pub focus: Option<usize>,
@@ -72,6 +76,12 @@ pub fn put_editor(app: &mut App, tab: ConnTabId, id: TabId, editor: Option<Edito
     }
 }
 
+/// Why no field of the page's row `row` can be edited: what the panel's
+/// footer says, once, for a row whose fields the form leaves as they are.
+pub fn row_lock(workspace: &Workspace, object: &ObjectTab, row: usize) -> Option<Lock> {
+    Table::of(workspace, object)?.row_lock(row)
+}
+
 impl<'a> Form<'a> {
     /// A panel the form has no part in: a SQL editor's result.
     pub fn none() -> Self {
@@ -79,6 +89,7 @@ impl<'a> Form<'a> {
             editor: None,
             target: None,
             parts: Vec::new(),
+            refused: None,
             focus: None,
             ending: Vec::new(),
         }
@@ -86,14 +97,15 @@ impl<'a> Form<'a> {
 
     /// The form of the page's row `row` of the table `object` shows.
     /// `editor` is the panel's, taken out of the tab; `hold` says no dialog
-    /// is up, so an open field has the keyboard.
+    /// is up, so an open field has the keyboard; `terminal` is the look,
+    /// which says a refused edit's reason in its mode line.
     pub fn of(
         workspace: &Workspace,
         object: &ObjectTab,
         tab: ConnTabId,
         row: usize,
         editor: Option<&'a mut Editor>,
-        hold: bool,
+        (hold, terminal): (bool, bool),
     ) -> Self {
         // Only an editor on this row is the panel's to draw.
         let editor = editor.filter(|editor| editor.cell.row == row);
@@ -105,6 +117,14 @@ impl<'a> Form<'a> {
         if table.row_lock(row).is_some() {
             return Self::none();
         }
+        // What an edit asked for in the panel was refused for, where it
+        // is one of this row's cells.
+        let refused = object
+            .edits
+            .why
+            .filter(|(cell, _)| cell.row == row)
+            .filter(|_| object.edits.why_place == EditorPlace::Panel && !terminal)
+            .map(|(cell, _)| cell.col);
         let editing = editor.as_ref().map(|editor| editor.cell.col);
         // The editor still in the tab is the grid's.
         let in_grid = object
@@ -134,6 +154,7 @@ impl<'a> Form<'a> {
             editor,
             target,
             parts,
+            refused,
             focus: None,
             ending: Vec::new(),
         }
@@ -144,6 +165,12 @@ impl<'a> Form<'a> {
         self.parts.get(col).copied().unwrap_or(Part::Read)
     }
 
+    /// Whether an edit of the column `col` was asked for in the panel and
+    /// refused: its field says why.
+    pub fn refused(&self, col: usize) -> bool {
+        self.refused == Some(col)
+    }
+
     /// Draws the panel's editor in its field's place, its text in `role`,
     /// and queues what its frame came to. Returns whether there was an
     /// editor to draw.
diff --git a/src/ui/row_panel.rs b/src/ui/row_panel.rs
index 3de5c21..c12e642 100644
--- a/src/ui/row_panel.rs
+++ b/src/ui/row_panel.rs
@@ -369,13 +369,29 @@ fn draw(
             // A part of the window to step to once it has a row to show.
             focus::region(ui, Region::Panel, full);
             // What editing adds, for a table's row that can be edited.
-            let mut form = match workspace.tab(id) {
-                Some(Tab::Object(object)) => {
-                    Form::of(workspace, object, tab, cell.row, editor, hold)
+            let object = match workspace.tab(id) {
+                Some(Tab::Object(object)) => Some(object),
+                _ => None,
+            };
+            let mut form = match object {
+                Some(object) => {
+                    let how = (hold, look.terminal);
+                    Form::of(workspace, object, tab, cell.row, editor, how)
                 }
-                _ => Form::none(),
+                None => Form::none(),
             };
             form.focus = focus;
+            // Why no field of the row can be edited, as the footer says it:
+            // in the grid's own words, the terminal's in its lower case.
+            let locked = object.and_then(|object| {
+                let lock = row_form::row_lock(workspace, object, cell.row)?;
+                let table = format::display_safe(&object.object.name);
+                Some(if look.terminal {
+                    crate::ui::workspace::lock_line(lock, &table, &look, locale)
+                } else {
+                    crate::ui::cell_editor::lock_text(lock, &table, locale)
+                })
+            });
             let structure = source.structure;
             let info = |name: &str| {
                 let key =
@@ -619,9 +635,9 @@ fn draw(
                     });
                 }
             }
-            // Footer: the editing controls, disabled until editing arrives.
-            // Its rule, the buttons (28 or 32), the note under them. A SQL
-            // result has none: its rows are no table's to edit.
+            // Footer: the editing controls. Its rule, the buttons (28 or
+            // 32), the note under them. A SQL result has none: its rows are
+            // no table's to edit.
             let note = line_of(ui, caption(&look), &look);
             let footer_height = if !source.table {
                 0.0
@@ -635,9 +651,9 @@ fn draw(
                 pos2(full.right(), full.bottom() - footer_height),
             );
             let foot = Rect::from_min_max(pos2(full.left(), body.bottom()), full.max);
-            if source.table {
-                let read_only = workspace.access == tabletist_db::Access::ReadOnly;
-                editing_footer(ui, foot, read_only, &look, &palette, locale);
+            if source.table && editing_footer(ui, foot, locked.as_deref(), &look, &palette, locale)
+            {
+                actions.push(Action::EditRow { tab, id });
             }
             let mut body_ui = ui.new_child(egui::UiBuilder::new().max_rect(body));
             let skin = FieldSkin {
@@ -965,11 +981,21 @@ fn field(
     response.widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, &name));
     let part = form.part(col);
     let editable = part == Part::Editable;
+    // Why this field alone cannot be edited, where it has a reason to give.
+    let locked = match part {
+        Part::Locked(lock) => {
+            let reason = crate::ui::cell_editor::lock_text(lock, "", locale);
+            (!reason.is_empty()).then_some(reason)
+        }
+        Part::Read | Part::Editable | Part::InGrid | Part::Editing => None,
+    };
     // Cut at the column's edge, as the field's own width allows, before
-    // the mark of a pending one and the pencil of one that can be edited.
+    // the mark of a pending one, the lock of one that cannot be edited and
+    // the pencil of one that can.
     let mark = if pending { 6.0 + MARK } else { 0.0 };
+    let lock_room = if locked.is_some() { 6.0 + LOCK } else { 0.0 };
     let pencil_room = if editable { PENCIL + 2.0 } else { 0.0 };
-    let room = line.width() - 30.0 - mark - pencil_room;
+    let room = line.width() - 30.0 - mark - lock_room - pencil_room;
     let shown = crate::ui::grid::ellipsize(&text, room, false, |text| {
         label_role.width(ui.ctx(), look.faces, text)
     });
@@ -992,6 +1018,22 @@ fn field(
             );
         }
     }
+    if let Some(reason) = &locked {
+        // A small lock after the label, always there: under the pointer
+        // it says why, and a screen reader hears it after the label.
+        let place = Rect::from_center_size(
+            pos2(
+                line.left() + label_width + mark + 6.0 + LOCK / 2.0,
+                line.center().y,
+            ),
+            vec2(LOCK, LOCK),
+        );
+        let lock = ui.interact(place.expand(3.0), name_id.with("lock"), Sense::hover());
+        let said = format!("{}: {reason}", gettext(locale, "Locked"));
+        lock.widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, &said));
+        Icon::Lock.image(palette.dim, LOCK).paint_at(ui, place);
+        lock.on_hover_text(reason.as_str());
+    }
     let column_name = format::display_safe(&column.name);
     let copy_label = format!("{} {column_name}", gettext(locale, "Copy"));
     let edit_label = format!("{} {column_name}", gettext(locale, "Edit"));
@@ -1112,12 +1154,30 @@ fn field(
     let shown = value_of(
         ui, tab, tab_id, row, col, column, value, info, tag, skin, read, actions,
     );
-    if !editable {
+    // An edit of a locked field was asked for here: it says why, under
+    // its value, until the selection moves. Brought into view once.
+    if let (Some(reason), true) = (&locked, form.refused(col)) {
+        ui.add_space(if look.terminal { 2.0 } else { 3.0 });
+        let width = ui.available_width();
+        let note = Text::one(look, caption(look), reason, palette.secondary)
+            .wrap(width)
+            .layout(ui.ctx())
+            .label(ui);
+        let asked = Id::new(("row-panel-refused", tab, tab_id));
+        let at = (request, row, col);
+        let seen = ui.data(|data| data.get_temp(asked)) == Some(at);
+        if !seen {
+            note.scroll_to_me(None);
+            ui.data_mut(|data| data.insert_temp(asked, at));
+        }
+    }
+    if !editable && locked.is_none() {
         return;
     }
     // What starts an edit of the value: its pencil, a double-click on it,
     // and Enter or F2 while its text has the keyboard (the terminal look
-    // edits with its own letters).
+    // edits with its own letters). Asked of a locked field, the same
+    // things say why it is locked.
     let mut edit = pencil.as_ref().is_some_and(egui::Response::clicked);
     // Enter on the pencil is the button's press. F2 edits from it too: a
     // value with no text of its own (a NULL, a document) has the keyboard
@@ -1130,7 +1190,7 @@ fn field(
     }
     if let Some(place) = shown.place {
         let over = ui.rect_contains_pointer(place);
-        if over {
+        if over && editable {
             // A field's border: the value can be edited.
             let color = if look.terminal {
                 palette.accent
@@ -1184,6 +1244,9 @@ fn field(
 /// The pencil of a field that can be edited: 22 pt, as Copy is.
 const PENCIL: f32 = 22.0;
 
+/// The lock after the label of a field that cannot be edited.
+const LOCK: f32 = 11.0;
+
 /// Where a field's outline is, round the place of its value: the value is
 /// flush with its label, and a field's border stands clear of its text.
 fn outline(place: Rect) -> Rect {
@@ -1747,15 +1810,19 @@ fn attachment_card(
     );
 }
 
-/// Edit, Duplicate and Delete, disabled, and why.
+/// The footer of a table's row: Edit, which edits the row in the panel,
+/// and Duplicate and Delete, which wait for a later version. `locked` is
+/// why no field of the row can be edited, in the look's words: Edit is
+/// disabled with it, and the note under the buttons says it. Returns
+/// whether Edit was pressed.
 fn editing_footer(
     ui: &mut egui::Ui,
     rect: Rect,
-    read_only: bool,
+    locked: Option<&str>,
     look: &Look,
     palette: &Palette,
     locale: crate::i18n::Locale,
-) {
+) -> bool {
     let side = side(look);
     if !look.terminal {
         ui.painter()
@@ -1771,14 +1838,19 @@ fn editing_footer(
             palette.surface_hover
         },
     );
-    let reason = gettext(locale, "Editing arrives in a later version");
+    let later = gettext(
+        locale,
+        "Duplicating and deleting rows arrive in a later version",
+    );
     let inner = rect.shrink2(vec2(side, 0.0));
     let gap = 6.0;
     let top = rect.top() + 1.0 + if look.terminal { 10.0 } else { 12.0 };
     let role = widgets::body(look);
     let measure = |text: &str| role.width(ui.ctx(), look.faces, text);
+    let mut edit = false;
     let height = if look.terminal {
-        // Three equal cells, dashed, at 55%: the keys in the text colour.
+        // Three equal cells, the keys in the text colour. Edit is a
+        // button, in a solid line; what waits is dashed, at 55%.
         let keys = [("e", "edit"), ("yy p", "duplicate"), ("dd", "delete")];
         let width = (inner.width() - 2.0 * gap) / 3.0;
         let faded = |color: egui::Color32| palette.panel.lerp_to_gamma(color, 0.55);
@@ -1787,19 +1859,39 @@ fn editing_footer(
                 pos2(inner.left() + index as f32 * (width + gap), top),
                 vec2(width, 28.0),
             );
-            let response = ui.interact(place, ui.id().with(("edit", index)), Sense::hover());
+            let enabled = index == 0 && locked.is_none();
+            let sense = if enabled {
+                Sense::click()
+            } else {
+                Sense::hover()
+            };
+            let response = ui.interact(place, ui.id().with(("edit", index)), sense);
             let name = format!("{key} {label}");
-            response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, false, &name));
-            let _ = response.on_hover_text(reason.as_ref());
-            dashed(ui, place, faded(palette.outline));
+            response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, enabled, &name));
+            let tint = |color: egui::Color32| if enabled { color } else { faded(color) };
+            if enabled {
+                edit = response.clicked();
+                ui.painter().rect_stroke(
+                    place,
+                    CornerRadius::same(3),
+                    Stroke::new(1.0, palette.outline),
+                    StrokeKind::Inside,
+                );
+            } else {
+                // Why not, under the pointer: the row's reason for Edit,
+                // the later version for the others.
+                let reason = locked.filter(|_| index == 0).unwrap_or(later.as_ref());
+                let _ = response.on_hover_text(reason);
+                dashed(ui, place, faded(palette.outline));
+            }
             // A cell too narrow for both shows its key alone.
-            let text = Text::one(look, role, key, faded(palette.text));
+            let text = Text::one(look, role, key, tint(palette.text));
             let total = measure(&name);
             let (total, text) = if total > width {
                 (measure(key), text)
             } else {
                 let text = text.space(role, " ");
-                (total, text.add(role, label, faded(palette.dim)))
+                (total, text.add(role, label, tint(palette.dim)))
             };
             let x = place.center().x - total / 2.0;
             widgets::paint_text(ui, x, place.center().y, text);
@@ -1819,33 +1911,38 @@ fn editing_footer(
             .collect();
         let extra = (inner.width() - 2.0 * gap - widths.iter().sum::<f32>()) / 3.0;
         let mut x = inner.left();
-        for ((icon, text), width) in labels.iter().zip(widths) {
+        for (index, ((icon, text), width)) in labels.iter().zip(widths).enumerate() {
             let place = Rect::from_min_size(pos2(x, top), vec2(width + extra, 32.0));
             x += width + extra + gap;
-            widgets::ButtonSpec::new(text)
+            let button = widgets::ButtonSpec::new(text)
                 .icon(*icon)
                 .icon_size(13.0)
-                .padding(0.0)
-                .disabled(&reason)
-                .show_at(ui, place, look, palette);
+                .padding(0.0);
+            // Edit waits only for a row that can be edited.
+            let why = if index == 0 {
+                locked
+            } else {
+                Some(later.as_ref())
+            };
+            match why {
+                Some(reason) => {
+                    button.disabled(reason).show_at(ui, place, look, palette);
+                }
+                None => edit = button.show_at(ui, place, look, palette).clicked(),
+            }
         }
         32.0
     };
+    // The note: why the row cannot be edited, where it cannot.
+    let Some(reason) = locked else {
+        return edit;
+    };
     let note_role = caption(look);
     let y = top + height + 8.0 + line_of(ui, note_role, look) / 2.0;
+    let measure = |text: &str| note_role.width(ui.ctx(), look.faces, text);
     if look.terminal {
-        let note = if read_only {
-            gettext(
-                locale,
-                "read-only connection · editing arrives in a later version",
-            )
-        } else {
-            gettext(locale, "editing arrives in a later version")
-        };
         // Cut at the panel's side, as a field's label is.
-        let shown = crate::ui::grid::ellipsize(&note, inner.width(), false, |text| {
-            note_role.width(ui.ctx(), look.faces, text)
-        });
+        let shown = crate::ui::grid::ellipsize(reason, inner.width(), false, measure);
         widgets::paint_text(
             ui,
             inner.left(),
@@ -1857,13 +1954,21 @@ fn editing_footer(
             ui,
             Rect::from_center_size(pos2(inner.left() + 5.5, y), vec2(11.0, 11.0)),
         );
+        let shown = crate::ui::grid::ellipsize(reason, inner.width() - 17.0, false, measure);
         widgets::paint_text(
             ui,
             inner.left() + 17.0,
             y,
-            Text::one(look, note_role, &reason, palette.dim),
+            Text::one(look, note_role, &shown, palette.dim),
         );
     }
+    // The whole of it for a screen reader, cut or not.
+    let place = Rect::from_min_size(
+        pos2(inner.left(), y - 8.0),
+        vec2(inner.width().max(1.0), 16.0),
+    );
+    widgets::announce(ui, place, reason);
+    edit
 }
 
 /// A dashed 1 pt outline round `rect`.
````

- [ ] **Step 4: Run the tests**

Run the command of step 2.
Expected: PASS, all five.

- [ ] **Step 5: Run the four checks, then commit**

```bash
git add src/ui/row_form.rs src/ui/row_panel.rs src/ui/mod.rs
git commit -S -m "Mark a locked field, and say in the row panel's footer why a row cannot be edited" -m "A field that cannot be edited wears a lock and says why when its edit is asked for. The footer's Edit edits the row, and its note gives the grid's own reason where no field of the row can be edited." -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 6: The keys

**Files:**
- Modify: `src/ui/keys.rs` (`editing_keys`, `editing_letters`, `SHORTCUTS`)
- Modify: `src/ui/workspace.rs` (the mode line's hints)
- Modify: `src/ui/row_panel.rs` (the footer's hint, Omarchy's head hint)
- Modify: `src/ui/mod.rs` (tests)

- **`Mod+I`** asks for `EditRow` in every look. It is read where `Mod+S` is (`editing_keys`, before the keys that wait for the grid to have the keyboard): wherever a table's Data view shows, whatever has the keyboard, a caret in one of the panel's values included. Unlike `Mod+S` it needs nothing pending. It is not read while an editor is open, in the Structure view, or in a frame that brings a click (the click selects its row once the frame is drawn). A SQL editor keeps the chord for its completions: `editing_keys` runs only with a table's tab in front.
- **`e`** asks for the same on Omarchy, read with the grid's other letters (`editing_letters`) under their rule: alone in its frame, with the keys on the grid.
- **The mode line** reads `-- INSERT --` for a panel's editor as for a cell's, since both are the tab's editor. Its hint "tab next cell" is left out there: in the panel Tab commits and stays until step 3.
- **The hints.** The footer's note names the chord on a row that can be edited ("⌘I edit"). Omarchy's head reads `[ ] prev/next` and `e edit`, and `e edit` gives way first where the room runs out.
- **The shortcuts screen** lists "Edit the row in the row panel": `Mod+I` on macOS and Windows, `e, Mod+I` on Omarchy, and `e` joins the summary row of Omarchy's letters.

- [ ] **Step 1: Write the failing tests**

````diff
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
index 7bcfa93..7500881 100644
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -20333,4 +20333,221 @@ mod tests {
             assert!(!outlined(&harness, "2"), "{}", look.name);
         }
     }
+
+    #[test]
+    fn mod_i_edits_the_row_in_the_row_panel_in_every_look() {
+        for look in Look::ALL {
+            let (mut harness, tab, id) = form_row(look, 1);
+            let panel = |harness: &Harness| harness.app.workspace(tab).unwrap().row_panel;
+            // With the panel closed: it is shown for the edit. The selected
+            // cell is the key's, so the row's first field that can be
+            // edited is the one.
+            harness.app.apply(Action::ToggleRowPanel(tab));
+            harness.press(Key::I, Modifiers::COMMAND);
+            assert!(panel(&harness), "{}", look.name);
+            assert_eq!(
+                form_editor(&harness, tab, id),
+                Some(((1, 1), true)),
+                "{}",
+                look.name
+            );
+            assert!(harness.ctx.text_edit_focused(), "{}", look.name);
+            // The selected cell's own field where it can be edited.
+            harness.app.apply(Action::CancelEdit { tab, id });
+            select(&mut harness, tab, id, (1, 2));
+            harness.press(Key::I, Modifiers::COMMAND);
+            assert_eq!(
+                form_editor(&harness, tab, id),
+                Some(((1, 2), true)),
+                "{}",
+                look.name
+            );
+            // While an editor is open the chord is no second way in: the
+            // grid's editor stays the grid's.
+            harness.app.apply(Action::CancelEdit { tab, id });
+            open_editor(&mut harness, tab, id, (2, 1));
+            harness.press(Key::I, Modifiers::COMMAND);
+            assert_eq!(
+                form_editor(&harness, tab, id),
+                Some(((2, 1), false)),
+                "{}",
+                look.name
+            );
+            harness.app.apply(Action::CancelEdit { tab, id });
+            // Not in a frame that brings a click: the click's row is
+            // selected only once the frame is drawn.
+            let at = cell_of(&harness, "user4@example.com");
+            harness.frame(vec![
+                egui::Event::PointerButton {
+                    pos: at,
+                    button: egui::PointerButton::Primary,
+                    pressed: true,
+                    modifiers: Modifiers::NONE,
+                },
+                crate::testing::key(Key::I, Modifiers::COMMAND),
+            ]);
+            harness.frame(vec![crate::testing::release(Key::I, Modifiers::COMMAND)]);
+            harness.settle();
+            assert!(edits(&harness, tab, id).editor.is_none(), "{}", look.name);
+            // Nor in the Structure view, which shows no rows.
+            select(&mut harness, tab, id, (1, 1));
+            harness.app.apply(Action::SetView {
+                tab,
+                object_tab: id,
+                view: crate::model::ObjectView::Structure,
+            });
+            harness.press(Key::I, Modifiers::COMMAND);
+            assert!(edits(&harness, tab, id).editor.is_none(), "{}", look.name);
+        }
+    }
+
+    #[test]
+    fn mod_i_reaches_the_row_from_a_caret_in_a_value_and_leaves_a_sql_editor_alone() {
+        for look in desktop_looks() {
+            let (mut harness, tab, id) = form_row(look, 1);
+            // A caret in one of the panel's values is a text field to the
+            // grid's keys, and not to this chord. It edits the row as from
+            // anywhere: by the selected cell, which is the key's here, so
+            // the row's first field that can be edited. (Enter is the key
+            // of the value the caret is in.)
+            let at = panel_text(&harness, "71").center();
+            click_at(&mut harness, at);
+            assert!(harness.ctx.text_edit_focused(), "{}", look.name);
+            harness.press(Key::I, Modifiers::COMMAND);
+            assert_eq!(
+                form_editor(&harness, tab, id),
+                Some(((1, 1), true)),
+                "{}",
+                look.name
+            );
+            harness.app.apply(Action::CancelEdit { tab, id });
+            // With a SQL editor in front the chord is its own (it asks for
+            // completions): the table behind it gets no editor.
+            let sql = harness.add_sql_tab(tab);
+            harness.app.apply(Action::ActivateTab { tab, id: sql });
+            harness.settle();
+            harness.press(Key::I, Modifiers::COMMAND);
+            assert!(edits(&harness, tab, id).editor.is_none(), "{}", look.name);
+        }
+    }
+
+    #[test]
+    fn mod_i_on_a_row_that_cannot_be_edited_shows_the_panel_and_its_reason() {
+        for look in Look::ALL {
+            let mut harness = Harness::new();
+            harness.set_look(look);
+            let tab = harness.connect_fake_as(true);
+            harness.click("users");
+            harness.answer_structure(crate::testing::fixture_structure());
+            harness.answer_rows(crate::testing::page(5, false));
+            harness.click("Row 2");
+            let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
+            focus_grid(&mut harness, tab);
+            harness.app.apply(Action::ToggleRowPanel(tab));
+            harness.press(Key::I, Modifiers::COMMAND);
+            assert!(harness.app.workspace(tab).unwrap().row_panel);
+            assert!(edits(&harness, tab, id).editor.is_none(), "{}", look.name);
+            // The footer's note is the reason. The terminal's mode line
+            // says it too, as for a cell; nothing is said under a field.
+            let why = look.label("This connection opens read-only");
+            let said = if look.terminal { 2 } else { 1 };
+            assert_eq!(times_painted(&harness, &why), said, "{}", look.name);
+        }
+    }
+
+    #[test]
+    fn the_terminals_e_edits_the_row_in_the_row_panel() {
+        let (mut harness, tab, id) = normal_mode((1, 1));
+        type_key(&mut harness, Key::E, "e");
+        assert_eq!(form_editor(&harness, tab, id), Some(((1, 1), true)));
+        assert_eq!(
+            editor_text(&harness, tab, id).as_deref(),
+            Some("user2@example.com")
+        );
+        // Insert mode, as for a cell: but Tab does not move on to a next
+        // cell from the panel's field, and the line does not offer it.
+        assert!(painted(&harness, "-- INSERT --"), "{:?}", harness.painted);
+        assert!(painted(&harness, "esc normal"), "{:?}", harness.painted);
+        assert!(!painted(&harness, "esc normal · tab next cell"));
+        // The letter was the key, and is no part of the text.
+        type_text(&mut harness, "!");
+        harness.press(Key::Escape, Modifiers::NONE);
+        assert_eq!(
+            pending_text(&harness, tab, id, (1, 1)).as_deref(),
+            Some("user2@example.com!")
+        );
+        // On the key's cell it takes the row's first field that can be
+        // edited, and opens a closed panel.
+        select(&mut harness, tab, id, (2, 0));
+        harness.app.apply(Action::ToggleRowPanel(tab));
+        type_key(&mut harness, Key::E, "e");
+        assert!(harness.app.workspace(tab).unwrap().row_panel);
+        assert_eq!(form_editor(&harness, tab, id), Some(((2, 1), true)));
+        harness.press(Key::C, Modifiers::CTRL);
+        // In the other looks the letter is typed text, which starts an
+        // edit of the cell with it.
+        for look in desktop_looks() {
+            let (mut harness, tab, id) = editable_in(look);
+            select(&mut harness, tab, id, (1, 1));
+            type_key(&mut harness, Key::E, "e");
+            assert_eq!(
+                form_editor(&harness, tab, id),
+                Some(((1, 1), false)),
+                "{}",
+                look.name
+            );
+            assert_eq!(editor_text(&harness, tab, id).as_deref(), Some("e"));
+        }
+    }
+
+    #[test]
+    fn the_row_panel_names_the_keys_that_edit_its_row() {
+        for look in Look::ALL {
+            // A row that can be edited: the terminal's head offers `e` as
+            // its footer's cell does, and the other looks' footer names
+            // the chord.
+            let (mut harness, tab, id) = editable_in(look);
+            select(&mut harness, tab, id, (1, 1));
+            if look.terminal {
+                assert_eq!(times_painted(&harness, "e edit"), 2);
+                assert!(painted(&harness, "[ ] prev/next"));
+            } else {
+                let chord = format!("{}I edit", look.command_key());
+                assert!(
+                    painted(&harness, &chord),
+                    "{}: {:?}",
+                    look.name,
+                    harness.painted
+                );
+            }
+            // One that cannot: the footer's cell is all that is left of
+            // `e`, and it waits; the chord is not named.
+            harness.app.workspace_mut(tab).unwrap().access = tabletist_db::Access::ReadOnly;
+            harness
+                .app
+                .workspace_mut(tab)
+                .unwrap()
+                .object_tab_mut(id)
+                .unwrap()
+                .fields = None;
+            harness.settle();
+            if look.terminal {
+                assert_eq!(times_painted(&harness, "e edit"), 1);
+                assert!(painted(&harness, "[ ] prev/next"));
+            } else {
+                let chord = format!("{}I edit", look.command_key());
+                assert!(!painted(&harness, &chord), "{}", look.name);
+            }
+        }
+        // The shortcuts screen lists the chord in every look, and the
+        // letter where it is a key.
+        for look in Look::ALL {
+            let listed: Vec<&str> = crate::ui::keys::shortcuts(&look)
+                .filter(|(_, what)| *what == "Edit the row in the row panel")
+                .map(|(keys, _)| keys)
+                .collect();
+            let keys = if look.terminal { "e, Mod+I" } else { "Mod+I" };
+            assert_eq!(listed, [keys], "{}", look.name);
+        }
+    }
 }
````

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked --lib -- mod_i_edits_the_row mod_i_reaches_the_row mod_i_on_a_row the_terminals_e_edits the_row_panel_names_the_keys`
Expected: all five FAIL on assertions: no editor opens for the chord or the letter (`mod_i_on_a_row_that_cannot_be_edited_shows_the_panel_and_its_reason` stops at `assertion failed: harness.app.workspace(tab).unwrap().row_panel`), and the panel names no key.

- [ ] **Step 3: Read the keys, and name them**

````diff
diff --git a/src/ui/keys.rs b/src/ui/keys.rs
index 450169b..4c3bfae 100644
--- a/src/ui/keys.rs
+++ b/src/ui/keys.rs
@@ -84,6 +84,8 @@ pub const SHORTCUTS: &[(&str, &str, Holds)] = &[
     // Editing a table's cells: each look's own keys for the same things.
     ("Enter, F2", "Edit the cell", DESKTOP),
     ("i, Enter", "Edit the cell", TERMINAL),
+    ("Mod+I", "Edit the row in the row panel", DESKTOP),
+    ("e, Mod+I", "Edit the row in the row panel", TERMINAL),
     ("cc", "Edit the cell from nothing", TERMINAL),
     ("Tab, Shift+Tab", "Commit and move right or left", ALL),
     ("Esc", "Cancel the edit", DESKTOP),
@@ -106,7 +108,7 @@ pub const SHORTCUTS: &[(&str, &str, Holds)] = &[
     ("Esc", "Close the SQL of the pending changes", TERMINAL),
     ("Y", "Copy the SQL of the pending changes", TERMINAL),
     (
-        "j/k, h/l, Ctrl+H/L, [ ], i, Enter, cc, x, u, Mod+S, :w, :e!, :diff, Y, Space, Esc, /, y, s, d, gd, za, t, 1…9",
+        "j/k, h/l, Ctrl+H/L, [ ], i, Enter, cc, e, x, u, Mod+S, :w, :e!, :diff, Y, Space, Esc, /, y, s, d, gd, za, t, 1…9",
         "Omarchy: vim keys (shown in the status line)",
         ALL,
     ),
@@ -829,6 +831,24 @@ fn editing_keys(app: &App, ctx: &egui::Context, keyboard: bool, actions: &mut Ve
         let show = !object.edits.reviewing;
         actions.push(Action::ReviewEdits { tab, id, show });
     }
+    // Mod+I edits the row in the row panel, which it shows: from wherever
+    // the table's rows show, whatever has the keyboard, in every look, and
+    // with nothing pending. Not while an editor is open, which is where
+    // the user is, and not in a frame that brings a click: the click
+    // selects its row once the frame is drawn, and the key would edit the
+    // row the selection leaves. A fresh press only. (In a SQL editor the
+    // chord asks for completions: no table is in front there.)
+    let edit_row = |input: &mut egui::InputState| {
+        let clicked = input
+            .events
+            .iter()
+            .any(|event| matches!(event, egui::Event::PointerButton { .. }));
+        consume_press(input, Modifiers::COMMAND, Key::I) && !clicked
+    };
+    let rows = object.view == crate::model::ObjectView::Data;
+    if !open && rows && ctx.input_mut(edit_row) {
+        actions.push(Action::EditRow { tab, id });
+    }
     // An open editor has the keyboard or is about to take it, and takes
     // the keys below with it: none of them opens another, and what is typed
     // is its text.
@@ -905,9 +925,10 @@ fn editing_keys(app: &App, ctx: &egui::Context, keyboard: bool, actions: &mut Ve
 }
 
 /// The terminal look's normal mode on the grid of the table `id`: `i` and
-/// Enter edit the selected cell from its value, `cc` from nothing, `x` sets
-/// it NULL, `u` puts back what was loaded, and `:` opens the prompt that
-/// writes, discards and shows the SQL. The letters are read as the
+/// Enter edit the selected cell from its value, `cc` from nothing, `e`
+/// edits its row in the row panel, `x` sets the cell NULL, `u` puts back
+/// what was loaded, and `:` opens the prompt that writes, discards and
+/// shows the SQL. The letters are read as the
 /// text they type, in the order they came, and taken: a letter that opens
 /// an editor or the prompt is no part of its text, and what follows it in
 /// its frame does nothing (the field is not there yet to be typed into).
@@ -939,7 +960,7 @@ fn editing_letters(
         })
     };
     let alone = actions.is_empty();
-    let mine = |text: &str| matches!(text, "i" | "c" | "x" | "u" | ":");
+    let mine = |text: &str| matches!(text, "i" | "c" | "e" | "x" | "u" | ":");
     let enter = |event: &egui::Event| is_press(event, Modifiers::NONE, Key::Enter);
     ctx.input_mut(|input| {
         // A chord types nothing, though some systems send its letter as
@@ -1000,6 +1021,10 @@ fn editing_letters(
                     first = true;
                     waits = true;
                 }
+                "e" => {
+                    opened = true;
+                    actions.push(Action::EditRow { tab, id });
+                }
                 "x" => actions.push(Action::SetNull { tab, id }),
                 "u" => actions.push(Action::RevertCell { tab, id }),
                 ":" => {
diff --git a/src/ui/row_panel.rs b/src/ui/row_panel.rs
index c12e642..2e3e89d 100644
--- a/src/ui/row_panel.rs
+++ b/src/ui/row_panel.rs
@@ -66,6 +66,9 @@ fn line_of(ui: &egui::Ui, role: TextRole, look: &Look) -> f32 {
 /// A colour value's swatch.
 const SWATCH: f32 = 14.0;
 
+/// Between two key hints of the terminal's header.
+const HINT_GAP: f32 = 12.0;
+
 /// Field labels and notes: 11.5 in both looks.
 fn caption(look: &Look) -> TextRole {
     TextRole::pick(look, TextRole::FieldLabel, TextRole::OCaption)
@@ -474,17 +477,25 @@ fn draw(
                     None => number.to_string(),
                 };
                 let measure = |text: &str| role.width(ui.ctx(), look.faces, text);
-                // The hint gives way before the name does: its keys alone
-                // when the whole name does not fit before its words. The
-                // name is cut where the hint begins, 10 before it.
-                let mut hint = [("[ ]", "prev/next", true)];
-                let mut width = widgets::key_hints_width(ui, &hint, 0.0, &look, &palette);
-                let mut room = esc.left() - 10.0 - width - 10.0 - x;
-                if measure(&whole) > room {
-                    hint = [("[ ]", "", true)];
-                    width = widgets::key_hints_width(ui, &hint, 0.0, &look, &palette);
-                    room = esc.left() - 10.0 - width - 10.0 - x;
-                }
+                // The hint gives way before the name does: first `e edit`,
+                // which a row that can be edited offers, then the words of
+                // its keys, when the whole name does not fit before them.
+                // The name is cut where the hint begins, 10 before it.
+                let steps: widgets::Hint<'_> = ("[ ]", "prev/next", true);
+                let editable = source.table && locked.is_none();
+                let hints: [&[widgets::Hint<'_>]; 3] = [
+                    &[steps, ("e", "edit", true)],
+                    &[steps],
+                    &[("[ ]", "", true)],
+                ];
+                let fit = |hint: &[widgets::Hint<'_>]| {
+                    let width = widgets::key_hints_width(ui, hint, HINT_GAP, &look, &palette);
+                    (width, esc.left() - 10.0 - width - 10.0 - x)
+                };
+                let offered = &hints[usize::from(!editable)..];
+                let fitting = offered.iter().find(|hint| measure(&whole) <= fit(hint).1);
+                let hint = *fitting.unwrap_or(&hints[2]);
+                let (width, room) = fit(hint);
                 // A number keeps its last digits.
                 let shown = crate::ui::grid::ellipsize(&whole, room, keyed.is_none(), measure);
                 let name = match keyed {
@@ -527,8 +538,8 @@ fn draw(
                 widgets::key_hints(
                     ui,
                     (esc.left() - 10.0 - width, y),
-                    &hint,
-                    0.0,
+                    hint,
+                    HINT_GAP,
                     &look,
                     &palette,
                 );
@@ -1933,12 +1944,20 @@ fn editing_footer(
         }
         32.0
     };
-    // The note: why the row cannot be edited, where it cannot.
-    let Some(reason) = locked else {
-        return edit;
-    };
     let note_role = caption(look);
     let y = top + height + 8.0 + line_of(ui, note_role, look) / 2.0;
+    // The note: why the row cannot be edited, where it cannot. Where it
+    // can, the key that edits it; the terminal's cells name their keys.
+    let Some(reason) = locked else {
+        if !look.terminal {
+            let key = format!("{}I", look.command_key());
+            let hint = Text::one(look, note_role, &key, palette.secondary)
+                .space(note_role, " ")
+                .add(note_role, &gettext(locale, "edit"), palette.dim);
+            widgets::paint_text(ui, inner.left(), y, hint);
+        }
+        return edit;
+    };
     let measure = |text: &str| note_role.width(ui.ctx(), look.faces, text);
     if look.terminal {
         // Cut at the panel's side, as a field's label is.
diff --git a/src/ui/workspace.rs b/src/ui/workspace.rs
index 9b7d0cd..0c1d41a 100644
--- a/src/ui/workspace.rs
+++ b/src/ui/workspace.rs
@@ -1561,6 +1561,9 @@ struct Editing {
     can_edit: bool,
     /// Insert mode: the column being edited and its type.
     insert: Option<String>,
+    /// Tab moves on to the next cell: the editor is on a cell. In the row
+    /// panel's field Tab commits and stays.
+    walks: bool,
     /// How much is pending, while anything is: "3 pending · 2 rows".
     pending: Option<String>,
     /// The cells to fix and the ones a save failed on: "1 error".
@@ -1729,9 +1732,14 @@ fn editing_status(app: &App, tab: ConnTabId) -> Editing {
         });
         saved.or(blocked)
     };
+    let walks = edits
+        .editor
+        .as_ref()
+        .is_some_and(|editor| editor.place == crate::edit::EditorPlace::Grid);
     Editing {
         can_edit,
         insert,
+        walks,
         pending,
         errors,
         said,
@@ -2010,10 +2018,14 @@ fn status_line(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
                 let mode = || Text::one(&look, TextRole::OModeLine, &mode, palette.success);
                 let words = ["normal", "next cell"].map(|word| gettext(locale, word));
                 let keys = || {
-                    Text::new(&look)
+                    let leave = Text::new(&look)
                         .add(role, "esc", palette.text)
                         .space(role, " ")
-                        .add(role, &words[0], palette.dim)
+                        .add(role, &words[0], palette.dim);
+                    if !editing.walks {
+                        return leave;
+                    }
+                    leave
                         .add(role, " · ", palette.dim)
                         .add(role, "tab", palette.text)
                         .space(role, " ")
````

- [ ] **Step 4: Run the tests**

Run the command of step 2.
Expected: PASS, all five. `ui::complete_tests` also has tests whose names begin `mod_i_` (the SQL editor's completions): they pass before and after.

- [ ] **Step 5: Run the four checks, then commit**

```bash
git add src/ui/keys.rs src/ui/workspace.rs src/ui/row_panel.rs src/ui/mod.rs
git commit -S -m "Reach the row panel's form with Mod+I, and with e on Omarchy" -m "Mod+I edits the selected row in the row panel from wherever a table's rows show, in every look, and e does on Omarchy's grid. The panel names both, and the shortcuts list them." -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 7: The scenes, and what the documents say

**Files:**
- Modify: `src/shots.rs` (`ROW_FORM`, `row_form_field`, `row_form_locked`)
- Modify: `docs/superpowers/specs/2026-10-06-row-form-design.md` (status, the model's actions, what step 1 leaves)
- Modify: `docs/superpowers/specs/2026-10-03-value-editing-core-design.md` (its slice 3 and slice 5 items, "Out of scope", the row panel's line, the keys table)
- Modify: `docs/superpowers/specs/2026-09-27-tabletist-design.md` (section 5.7, the keyboard table)
- Modify: `README.md`

Two scenes for review, on the Bookshop data, in every look, light and dark: `row-form-field` (the editor in a field's place holding a text its column refuses, beside a pending field) and `row-form-locked` (an edit asked for on the key's field). They are looked at by hand and are never a test. The PNGs stay in `target/shots/`, which is ignored: they are not committed and not posted.

- [ ] **Step 1: Add the scenes and update the documents**

````diff
diff --git a/README.md b/README.md
index 699102d..9d96955 100644
--- a/README.md
+++ b/README.md
@@ -52,8 +52,9 @@ Linux (Omarchy and Hyprland first), macOS and Windows.
 - A sidebar of recent objects and one schema's tables and views, folded into
   prefix groups (`book_`) or listed flat; each table opens with a data grid
   (keys, foreign keys, value tags, JSON at a glance), a row panel showing
-  every field in full with a jump along foreign keys, and a Structure view
-  (columns, indexes, foreign keys).
+  every field in full with a jump along foreign keys, where a value of a
+  writable table is edited in place (Cmd/Ctrl+I, or a double-click on it),
+  and a Structure view (columns, indexes, foreign keys).
 - Server-side sorting and paging, a filter bar with a raw WHERE option, exact
   counts on demand, and cancel for any running query. MySQL sessions run in
   utf8mb4, with `ANSI_QUOTES`, the combination modes that imply it and
diff --git a/docs/superpowers/specs/2026-09-27-tabletist-design.md b/docs/superpowers/specs/2026-09-27-tabletist-design.md
index 9ea8fb7..b78509c 100644
--- a/docs/superpowers/specs/2026-09-27-tabletist-design.md
+++ b/docs/superpowers/specs/2026-09-27-tabletist-design.md
@@ -590,8 +590,14 @@ enum Dialog { Connection(..), Password(..), HostKey(..), QuickOpen(..), Help,
   another page, a refresh or a new result starts fresh.
 - A pending cell's field shows its new value with the pending mark and
   "was <loaded value>" under it, so the panel never disagrees with the
-  grid. The panel itself stays read-only: its Edit, Duplicate and Delete
-  are disabled.
+  grid.
+- On a table's row that can be edited, a field's value is edited in the
+  panel, in its place: by a double-click on it, the pencil in its label
+  line, Enter or F2 on it, the footer's Edit, or Cmd/Ctrl+I (Omarchy: also
+  `e`). The text becomes the pending cell an edit in the grid makes. A
+  field that cannot be edited wears a lock and says why when asked, and
+  the footer says why a whole row cannot. Duplicate and Delete are
+  disabled. See `2026-10-06-row-form-design.md`.
 - On Omarchy `i` and Enter edit the cell on a table's grid, and Space and
   Cmd/Ctrl+Shift+R open the panel there. On a SQL result `i` and Enter
   still open it, Enter only when no widget has the keyboard. An Esc that
@@ -629,6 +635,7 @@ read-only table (structure data is small; the data grid is not needed).
 | Cmd/Ctrl+Alt+Left / Right | Previous / next page |
 | Cmd/Ctrl+. | Cancel running query |
 | Space, Cmd/Ctrl+Shift+R | Toggle row panel |
+| Cmd/Ctrl+I (Omarchy: also `e`) | Edit the row in the row panel |
 | Cmd/Ctrl+C, Cmd/Ctrl+Shift+C | Copy cell / copy row |
 | Arrows, Home/End, Enter | Move in the tree |
 | Arrows, Page Up/Down, Home/End | Move in the grid |
diff --git a/docs/superpowers/specs/2026-10-03-value-editing-core-design.md b/docs/superpowers/specs/2026-10-03-value-editing-core-design.md
index 8ef64f0..c29aec4 100644
--- a/docs/superpowers/specs/2026-10-03-value-editing-core-design.md
+++ b/docs/superpowers/specs/2026-10-03-value-editing-core-design.md
@@ -39,10 +39,13 @@ sub-projects, each with its own spec, plan and pull request:
 2. Editors by type: enum and CHECK pickers, boolean cycling, foreign key
    search, the calendar, JSON highlighting, array chips, binary from a
    file, `DEFAULT`.
-3. The row panel (the design's inspector) as a row form.
+3. The row panel (the design's inspector) as a row form:
+   `2026-10-06-row-form-design.md`, which takes the form from the macOS
+   "Editing a row" artboard.
 4. Power keys: undo and redo, pasting a TSV block, `.`, the rest of the vim
    set, `$EDITOR`.
-5. Rows: add, duplicate, delete (the "Editing a row" artboards).
+5. Rows: add, duplicate, delete (the rest of the "Editing a row"
+   artboards).
 6. Writes from the SQL editor, and allowing writes for one tab of a
    read-only connection. The first is
    `2026-10-05-sql-editor-writes-design.md`, which rejects the second.
@@ -77,9 +80,9 @@ editing auto-updatable views.
 
 Everything in slices 2 to 6. Editing key columns, binary values, values
 over 256 KiB, views, materialized views and SQL results. Editing in the
-row panel: its Edit, Duplicate and Delete buttons stay disabled and say
-"arrives in a later version". Rows of a page that is no longer loaded:
-pending changes never outlive their page.
+row panel, which slice 3 has since added
+(`2026-10-06-row-form-design.md`). Rows of a page that is no longer
+loaded: pending changes never outlive their page.
 
 ## Writable connections
 
@@ -458,9 +461,9 @@ again.
 - **Revert one cell:** `Mod+Z` on a pending cell that is active puts back
   the loaded value. The undo and redo stack is slice 4.
 - The row panel shows a pending cell's new value with the pending mark and
-  "was <loaded value>", so it never disagrees with the grid. It stays
-  read-only: its Edit, Duplicate and Delete, and the header's Add row, stay
-  disabled.
+  "was <loaded value>", so it never disagrees with the grid. In this
+  slice it stayed read-only; slice 3 edits a field in it, and its
+  Duplicate and Delete, and the header's Add row, stay disabled.
 - Copying takes the pending value a cell shows, for the cell and for the
   row, in every look.
 
@@ -476,6 +479,7 @@ again.
 | Apply in the popover | `Mod+Enter` | Ctrl+Enter |
 | Set NULL | `Mod+Backspace` | `x` |
 | Revert the cell | `Mod+Z` | `u` |
+| Edit the row in the row panel (slice 3) | `Mod+I` | `e`, `Mod+I` |
 | Review SQL | `Mod+Shift+D`, the bar's button | `:diff`, `Mod+Shift+D` |
 | Close Review SQL | `Mod+Shift+D`, the bar's button | Esc, `Mod+Shift+D` |
 | Copy the SQL | Copy SQL, in the drawer's head | `Y`, while the panel is open |
diff --git a/docs/superpowers/specs/2026-10-06-row-form-design.md b/docs/superpowers/specs/2026-10-06-row-form-design.md
index da834de..d39c8b5 100644
--- a/docs/superpowers/specs/2026-10-06-row-form-design.md
+++ b/docs/superpowers/specs/2026-10-06-row-form-design.md
@@ -1,7 +1,8 @@
 # Editing values, slice 3: the row panel as a row form
 
-Date: 2026-10-06. Status: designed, nothing built. Step 1 is planned in
-`docs/superpowers/plans/2026-10-06-row-form-fields.md`.
+Date: 2026-10-06. Status: step 1 (edit a field in place) is built, see
+`docs/superpowers/plans/2026-10-06-row-form-fields.md`. Steps 2 and 3 are
+designed and not built.
 
 ## Intent
 
@@ -116,7 +117,8 @@ Rejected:
 
     pub struct Editor { ..., pub place: EditorPlace }
 
-    Action::EditCell { tab, id, cell, start, place }
+    Action::EditCell { tab, id, cell, start }   // in the grid, as before
+    Action::EditField { tab, id, cell }         // in the panel, from the value
     Action::EditRow { tab, id }
 
 - **`Editor::place`** says which view draws the tab's editor. Nothing else
@@ -207,7 +209,7 @@ where that order turns:
   After the cut such a cell of such a row answers the row's reason. No
   view asks for a cell the page does not hold.)
 - **Why an edit that was asked for was refused** is kept with the place it
-  was asked from (`Edits::why` gains the `EditorPlace`), so the reason
+  was asked from (`Edits::why_place`, beside `Edits::why`), so the reason
   shows where the user is looking: at the cell, or under the field.
 - **What the panel knows of a pending cell** (`model::PendingField`) gains
   the cell's state in step 3, for what a field says of a cell to fix or
@@ -258,7 +260,8 @@ drawn as today, with no outline, pencil or checkbox.
 - `Mod+I`, Omarchy's `e`, and the footer's Edit: `Action::EditRow`.
 - Typing on a value does not start an edit: the panel's values take a
   caret to select and copy from, and `Mod+C` there must stay a copy.
-- Each of these asks for `EditCell` with `EditStart::Value` in the panel.
+- Each of these asks for `EditField`, which is `EditCell` with
+  `EditStart::Value`, drawn in the panel.
   The editor starts from the pending value where the cell has one, and
   otherwise from the value's whole text as the database gave it
   (`edit::start_text`), never from the text the panel cut. The cursor is
@@ -475,6 +478,26 @@ Each step ends compiling, tested and shippable, and gets its own plan run.
 After step 1 every editable value has a way in from the panel, and nothing
 on screen offers what is not built.
 
+### What step 1 leaves for steps 2 and 3
+
+- `Mod+I`, `e` and Edit go by the selected cell, not by the field a caret
+  is in: with a caret in one value of the panel they can open another
+  field. Enter and F2 are the keys of the value the caret is in.
+- The panel's field is drawn in the look's own text field, on its fill.
+  The red border of a text that fails its check is drawn at once; the
+  focus ring and its halo show once a key was pressed, as everywhere.
+- A locked value takes a double-click as an editable one does, and answers
+  with its reason: the word under the pointer is selected as well, and the
+  grid's selection moves to that cell.
+- Edit pressed while the grid's popover is open on the selected cell's
+  tall value closes the popover, its text kept: the click takes the
+  keyboard from it, and until step 2 a tall value has no editor in the
+  panel to open instead.
+- On Omarchy the pencil of a document stands before the `za fold` hint and
+  shows only while the pointer is near, so the caption's hints do not move.
+- The header's Add row still says "Editing arrives in a later version",
+  as it did while the grid alone was edited.
+
 ## Testing
 
 - `src/edit.rs`: `row_lock` against `lock` for every reason, in each
@@ -507,10 +530,11 @@ on screen offers what is not built.
   the checkbox of a field above the one being edited;
   `revert`; the header's count; the messages under a field.
 - `src/shots.rs` gains scenes to look at by hand, on its Bookshop data and
-  in every look: `row-form-field` (a field being edited), `row-form-locked`
-  (a locked field and a row lock's note), and with the later steps
-  `row-form-tall` and `row-form-pending`. No test compares a screen with
-  the design.
+  in every look: `row-form-field` (a field being edited, its text refused,
+  beside a pending one), `row-form-locked` (a locked field saying why; a
+  row lock's note shows in the scenes of a read-only connection), and with
+  the later steps `row-form-tall` and `row-form-pending`. No test compares
+  a screen with the design.
 - Nothing here changes what a save sends, so no test needs a PostgreSQL or
   a MySQL server: the suite that does is as it was.
 
diff --git a/src/shots.rs b/src/shots.rs
index 1b4d2ce..3261e92 100644
--- a/src/shots.rs
+++ b/src/shots.rs
@@ -371,6 +371,44 @@ const EDITING: [(&str, Scene); 10] = [
     ("edit-production", edit_to_production),
 ];
 
+/// The row panel as a row form, one scene per state, in every look.
+const ROW_FORM: [(&str, Scene); 2] = [
+    ("row-form-field", row_form_field),
+    ("row-form-locked", row_form_locked),
+];
+
+/// A field of the row panel being edited: the editor in the value's place,
+/// holding a text its column does not take, with why under it. Another
+/// field of the row is pending, and the footer's Edit can be pressed.
+fn row_form_field(harness: &mut Harness) {
+    let (tab, id) = editable(harness);
+    retype(harness, tab, id, (4, DELETED_AT), DELETED);
+    harness.app.apply(Action::EditField {
+        tab,
+        id,
+        cell: CellPos {
+            row: 4,
+            col: BOOK_ID,
+        },
+    });
+    let workspace = harness.app.workspace_mut(tab).unwrap();
+    let editor = workspace.object_tab_mut(id).unwrap().edits.editor.as_mut();
+    editor.expect("the panel's editor").text = "107233x".into();
+    // What the field says when its text changed: the check runs.
+    harness.app.apply(Action::EditorTyped { tab, id });
+}
+
+/// An edit asked for on the key's field: its lock, and why under its value
+/// (the terminal look says it in its mode line).
+fn row_form_locked(harness: &mut Harness) {
+    let (tab, id) = editable(harness);
+    harness.app.apply(Action::EditField {
+        tab,
+        id,
+        cell: CellPos { row: 4, col: 0 },
+    });
+}
+
 /// Three pending cells in two rows, one of them a book's id that is no
 /// number: the counts, and "1 to fix" where a save would be.
 fn edit_pending(harness: &mut Harness) {
@@ -1057,6 +1095,11 @@ fn shots() {
     for (name, scene) in EDITING {
         both(name, scene);
     }
+    // The row panel as a row form: a field being edited, and a locked one
+    // saying why.
+    for (name, scene) in ROW_FORM {
+        both(name, scene);
+    }
 }
 
 /// Runs the active SQL editor's statement, or `all` of its script.
````

- [ ] **Step 2: Render the scenes and look at them**

```bash
~/.cargo/bin/cargo clippy --locked --features shots --lib --tests -- -D warnings
~/.cargo/bin/cargo test --locked --features shots --lib shots::shots -- --ignored --exact
```

Expected: the lint is clean, the test passes (about 45 s) and writes `target/shots/row-form-field-<look>-<light|dark>.png` and `row-form-locked-...`, twelve files. Read them. What to see:

- `row-form-field`: under `book_id`, a text field holding `107233x` with a red border and "int8 expects a whole number" under it; `deleted_at` pending, with its mark and "was NULL"; `id` with a lock after its label; the footer's Edit enabled and Duplicate and Delete greyed; "⌘I edit" under them on macOS ("Ctrl+I edit" in the standard look); on Omarchy `e edit` in the head and as a solid cell in the footer, and the mode line `-- INSERT --` with `esc normal` and no "tab next cell".
- `row-form-locked`: "Part of the row's key" under the value of `id` on macOS and in the standard look; on Omarchy the same words in the mode line, in lower case, and nothing under the field.

- [ ] **Step 3: Run the four checks, then commit**

```bash
git add src/shots.rs docs/superpowers/specs/2026-10-06-row-form-design.md docs/superpowers/specs/2026-10-03-value-editing-core-design.md docs/superpowers/specs/2026-09-27-tabletist-design.md README.md
git commit -S -m "Add scenes of the row form, and say in the documents what the row panel does now" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

## By hand, for the user

The window cannot open where this plan runs, so these were not done and are yours:

- On a writable connection, in each look you use: double-click a value in the row panel, type, Enter; see it amber in the grid and marked in the panel; Save. The same through the pencil, through Enter with a caret in the value, through Edit, and through `Mod+I` with the panel closed.
- On Omarchy: `e` on a cell, type, Esc (kept), `e`, type, Ctrl+C (dropped); that the status line reads `-- INSERT --` and Esc there does not close the panel.
- The outline under the pointer and the pencil beside Copy: that they come and go as Copy does, and that a click meant to select text in a value still only selects.
- A long row: `Mod+I` on a row whose first editable field is far down, to see the panel scroll to it.
- The lock's tooltip (it waits for the tooltip's delay, which the scenes do not show).
- macOS: that Cmd+I reaches the app with a table in front (no menu item takes it).
- A text typed through an input method into the panel's field.

## What the next run will find here

- `row_form::Part` is where step 3's states go: a field to fix or failed needs `PendingField` to carry the cell's state.
- `cell_editor::in_panel` forces `Advance::Stay`. Step 3's walk replaces that with `Advance::NextField` and `PrevField`, which the reducer performs only when the commit closed the editor.
- `edit_cell` sends a tall value to the grid (`if large { EditorPlace::Grid }`) and `EditorBreak` moves a panel's editor there. Step 2 removes both and draws the tall field in `Form::editing`.
- `SetNull` and `RevertCell` still act on the selected cell and do nothing while an editor is open. Step 3's checkbox and `revert` need them to name a cell.
