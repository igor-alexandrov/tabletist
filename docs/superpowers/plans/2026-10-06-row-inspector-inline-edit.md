# Row Inspector, Step 1: Fields That Edit Like Cells Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A field of the row panel is one control that edits like a grid cell: a click on its value opens the editor, Enter and Tab commit and walk to the next field, `Mod+I` (Omarchy `ctrl+l`) puts the keyboard on the row's first editable field, and a pending, failed or locked field looks as the grid's cell does. The Edit button and everything that made the panel a mode go.

**Architecture:** Nothing below the view's edge changes shape: the tab's one editor (`Edits::editor`, drawn in `EditorPlace::Panel`), the one pending set (`Edits::cells`) and `edit::Table::lock` stay as pull request #89 left them. What changes is how a field is reached. The value's place becomes a focusable control with a stable id (`row_panel::field_stop`); the field that has egui's keyboard focus is the panel's cursor, and its cell is the grid's selected cell. `ui/keys.rs` asks which field has the keyboard and routes every key of it, so the grid and the panel cannot both take one. `Action::EditRow` is replaced by `Action::FocusFields`.

**Tech Stack:** Rust 1.98, egui/eframe (the crmne fork). Spec: `docs/superpowers/specs/2026-10-06-row-inspector-inline-edit-design.md`.

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

- `src/shots.rs` is outside those four. Where a task touches it (tasks 4 and 7), also run `~/.cargo/bin/cargo clippy --locked --features shots --lib --tests -- -D warnings`.
- **What was run where this plan was written: nothing.** Unlike the plan of #89, this one was written from reading the tree at `c158b42` (the merge of #89) and the design canvas, not from a draft that was built. Every code block is written against names that exist in that tree, but none was compiled. Treat a block as what the code should come to, and expect to correct a name or a borrow on the first build. Where a block and the compiler disagree, the compiler is right and the task's tests say what must hold. A reviewer then read the plan against the tree (and the vendored egui, `crmne/egui` at `0b43114`) and found eight faults, all corrected here; what it confirmed is noted where it matters.
- **Databases.** Nothing here changes what a save sends, reads or compares. No test needs a PostgreSQL or a MySQL server.
- **The branch.** `claude/row-inspector-inline-edit-9297c4`, at `c158b42`. Before task 1, commit the spec and this plan on their own.
- House rules that bite here: no em dashes anywhere; comments explain why, in the surrounding code's voice; a view pushes `Action`s and never changes application state, apart from the text a field is editing; views draw text only through `TextRole`s; keyboard focus is drawn only by `src/ui/focus.rs` (a widget says its form with `focus::hint`); what a user typed never reaches a log; do not weaken a lint or delete a test to get green. Tests of #89 that assert what this plan removes (the double-click, the pencil of a plain value, the Edit button, `e`) are rewritten to assert what replaces it, in the task that replaces it. Each task says which.
- No design or pixel conformance in any test, and no design material in the repository. Fixtures use the Bookshop data. Compare with the design by eye, with the scenes of task 7.
- Commits are signed, one per task, after its checks pass. If signing fails, do not commit unsigned: stage the task and tell the user. Each message ends with:

      Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>

## What #89 built, and what the design asks

| | #89 | The design, and this plan |
|---|---|---|
| A value at rest | Text with a caret, to select and copy from | One control. No caret. |
| Pointer starts an edit | Double-click, or a pencil in the label line | One click on the value. Under the pointer: a fill, a border, a pencil at the value's right, the I-beam. |
| Keyboard reaches a field | `Mod+I` / `e` / Edit open an editor at once | `Mod+I` / `ctrl+l` put the keyboard on the first editable field. Enter, F2 or typing edits it. |
| Between fields | Tab through every control of the panel | Up and Down (`j`/`k`). Esc (`ctrl+h`) goes back to the grid's cell of that column. |
| Enter in the editor | Commits, stays | Commits, keyboard on the next editable field |
| Tab, Shift+Tab | Commit, stay | Commit, next or previous editable field |
| Pending | A dot after the label, "was x" under the value | Amber fill and left bar on the value; "was x · revert" at the label's right |
| To fix, failed | Reads as any pending field | Red fill and border, the message under the field |
| Footer | Edit, Duplicate, Delete, "⌘I edit" | Duplicate, Delete, "Click a value or press ⌘I to edit" |
| A row that cannot be edited | The footer's note says why | One line at the top of the panel says why |

## What was decided

The spec is the user's. These are the places where the app cannot yet do what it says, or where it leaves a choice. Each is the user's to overrule before the run.

1. **Editors by type (spec 4).** The grid has one editor today: text, in a field on the cell or a popover. The spec says to reuse the grid's editors and build none for the inspector, so the inspector gets that one. Pick lists, the boolean switch, foreign key search, the calendar, array chips, binary from a file, JSON line numbers and Format, and `DEFAULT` arrive for both places when the grid gets them (the value editing spec's slice 2).
2. **Tall values stay in the grid's popover in this step.** A JSON value, a text with a line break or one over 256 characters still opens at its cell, and the panel says "Editing in the grid…". Editing them in the panel, as the spec asks, is step 2, its own plan: it needs the popover's body drawn in the panel.
3. **Undo and redo (spec 5).** `Mod+Z` and `u` put back one cell's loaded value, from the grid or from a field. The stack and `Shift+Mod+Z` are the value editing spec's slice 4.
4. **Duplicate and Delete (spec 1)** are drawn and stay disabled ("Duplicating and deleting rows arrive in a later version"): they change what a save sends, which the spec puts out of scope. The hint line reads "Click a value or press ⌘I to edit" and names `⌘D` and `⌫` once they do something.
5. **The keyboard leaving a field whose text its column refuses (spec 3).** The spec keeps the editor open. The grid keeps the text as a cell to fix and lets the keyboard go, so typing is never lost and nothing traps the pointer. The inspector does what the grid does: the field shows the Failed state with its message, and Save is blocked, until it is fixed or reverted. Enter and Tab on such a text keep the editor open, as on a cell.
6. **A value with clicks of its own** (a JSON tree, an array list, a binary's Save, an attachment card) keeps the pencil #89 gave its label line. A click in a tree folds it.
7. **A focused field's cell is the grid's selected cell.** Focusing a field selects that column's cell of the row, so `Mod+C`, `Mod+Backspace` and `Mod+Z` act on it, the grid shows it, and Esc lands on it. The grid scrolls sideways to it, as for any moved selection.
8. **A value that can be edited or is locked takes no caret.** Its text is copied with the label line's Copy, or `Mod+C` with the field focused. Under a row lock (a read-only connection, a view, a save that runs) and in a SQL result the values read as they always did, caret and all.
9. **The walk is in the page's column order**, in every look. Omarchy draws documents last: the walk reaches each in its column's turn.
10. **`Mod+I` puts the keyboard on the first editable field and opens no editor**, as the spec says. Not the selected cell's field, as #89's `EditRow` chose.
11. **Omarchy.** `ctrl+l` from the grid focuses the fields (it stepped to the panel's close button). `i` and Enter edit the focused field, `cc` replaces, `x` and `u` as on a cell, `j`/`k` move, `ctrl+h` goes back. `s` stays "Structure", as on the grid: the app never gave it to "replace". `e` goes. Esc still closes the panel.
12. **Locked** is what `edit::Table::lock` says. The app reads no column privileges.
13. **"Keyboard settings"** is the shortcuts list (`keys::SHORTCUTS`): the app has no screen that records keys.
14. **One PR** for this step, as the spec's Delivery says. Step 2 is another.

## What this run leaves

- Step 2: the tall field in the panel (multi-line text, JSON).
- The value editing spec's slices 2, 4 and 5, for the grid and the inspector at once: editors by type, the undo stack, rows.
- The "Editing in the grid…" stand-in stays: one editor is open at a time.

## The spec's tests, and where each is

| Spec test | Task | Test |
|---|---|---|
| 1 inspector edit marks the grid's cell, and back | 2 | `a_click_on_a_value_edits_it_in_the_row_panel` (and #89's `the_row_panel_shows_a_pending_value_and_what_it_was`) |
| 2 undo from the grid reverts an inspector edit | 3 | `mod_z_on_the_grid_puts_back_what_a_field_changed` |
| 3 Enter walks, locked skipped | 1 | `enter_and_tab_in_a_field_commit_and_walk_to_the_next_that_can_be_edited`, `a_walk_passes_over_a_locked_field` |
| 4 Esc cancels one field only | 3 | `escape_drops_one_fields_edit_and_keeps_the_others` |
| 5 invalid int8 keeps the editor, blocks Save | 1 | #89's `a_text_the_column_refuses_keeps_the_panels_field_and_says_why_under_it`, extended with `save_blocked` |
| 6 read-only: nothing edits | 6 | `a_read_only_connection_says_so_once_and_edits_nothing` |
| 7 `Mod+I` / `ctrl+l` | 4, 5 | `mod_i_shows_the_panel_and_focuses_the_first_field_that_can_be_edited`, `ctrl_l_focuses_the_rows_fields_in_the_terminal_look` |
| 8 rows and back keep edits | 6 | `a_fields_pending_value_is_there_when_its_row_comes_back` |
| 9 NULL and the empty string | 3 | `clearing_a_field_is_the_empty_string_and_mod_backspace_is_null` |
| 10 same SQL from either place | 6 | `an_edit_made_in_a_field_is_reviewed_as_the_same_edit_made_on_its_cell` |

## File map

| File | What changes |
|---|---|
| `src/model.rs` | `Advance::NextField`, `PrevField`; `Action::EditField` gains `start`; `Action::FocusFields`, `Action::MoveField` come, `Action::EditRow` goes; `Action::RevertCell` gains `cell`; `PendingField::trouble` |
| `src/app.rs`, `src/app/editing.rs` | `field_after`, `focus_fields`, `move_field`; `CommitEdit` walks; `FieldFocused` selects; `edit_row` goes; reducer tests |
| `src/ui/cell_editor.rs` | `in_panel` asks to walk instead of to stay |
| `src/ui/row_panel.rs` | `field_stop`, `focused_field`; the value as a control and its states; the label line's "was · revert"; the header's count; the row lock's line; the footer |
| `src/ui/row_form.rs` | `Part` loses nothing; `Form` carries the row's pending count and each field's trouble |
| `src/ui/keys.rs` | the focused field's keys in `editing_keys` and `letters`; `ctrl+l` / `ctrl+h`; `SHORTCUTS` |
| `src/ui/mod.rs` | the row form's tests |
| `src/shots.rs` | the row form's scenes |
| `docs/…`, `README.md` | the two specs' status, the README's line |

---

### Task 1: Enter and Tab walk the fields

In a field of the panel, Enter and Tab commit and put the keyboard on the row's next field that can be edited; Shift+Tab on the one before. At the row's end the keyboard stays on the field. A text the column refuses keeps the editor open. Nothing else changes yet: the field is still reached by its pencil or a double-click.

**Files:**
- Modify: `src/model.rs` (`Advance`)
- Modify: `src/ui/cell_editor.rs` (`in_panel`)
- Modify: `src/app/editing.rs` (new `field_after`), `src/app.rs` (`Action::CommitEdit`, every `match` on `Advance`)
- Test: `src/ui/mod.rs` (section "The row panel as a row form"), `src/app.rs` (tests)

- [ ] **Step 1: Write the failing tests.** In `src/ui/mod.rs`, after `a_double_click_on_a_value_edits_it_in_the_row_panel_and_one_click_does_not`:

```rust
    #[test]
    fn enter_and_tab_in_a_field_commit_and_walk_to_the_next_that_can_be_edited() {
        for look in desktop_looks() {
            let (mut harness, tab, id) = form_row(look, 1);
            harness.click("Edit email");
            type_text(&mut harness, "x");
            harness.press(Key::Enter, Modifiers::NONE);
            assert_eq!(
                pending_text(&harness, tab, id, (1, 1)).as_deref(),
                Some("user2@example.comx"),
                "{}",
                look.name
            );
            assert!(edits(&harness, tab, id).editor.is_none(), "{}", look.name);
            // The keyboard is on the next field, qty: F2 edits it.
            harness.press(Key::F2, Modifiers::NONE);
            assert_eq!(
                form_editor(&harness, tab, id),
                Some(((1, 2), true)),
                "{}",
                look.name
            );
            // Shift+Tab walks back to email.
            harness.press(Key::Tab, Modifiers::SHIFT);
            harness.press(Key::F2, Modifiers::NONE);
            assert_eq!(
                form_editor(&harness, tab, id),
                Some(((1, 1), true)),
                "{}",
                look.name
            );
            // Before it stands only the key, which is locked: the walk
            // stays where it is.
            harness.press(Key::Tab, Modifiers::SHIFT);
            harness.press(Key::F2, Modifiers::NONE);
            assert_eq!(
                form_editor(&harness, tab, id),
                Some(((1, 1), true)),
                "{}",
                look.name
            );
            // Tab from the last field stays on it.
            harness.press(Key::Tab, Modifiers::NONE);
            harness.press(Key::F2, Modifiers::NONE);
            harness.press(Key::Tab, Modifiers::NONE);
            harness.press(Key::F2, Modifiers::NONE);
            assert_eq!(
                form_editor(&harness, tab, id),
                Some(((1, 2), true)),
                "{}",
                look.name
            );
        }
    }
```

In `src/app.rs`'s tests, beside the tests of `EditField` (search `EditorPlace::Panel`), a reducer test `a_walk_passes_over_a_locked_field`: build the table those tests use with a generated column between two that can be edited (the fixture of `generated_binary_and_huge_cells_are_locked` in `src/edit.rs` shows how a column is marked generated), open the panel's editor on the first, apply `Action::CommitEdit { then: Advance::NextField }`, and assert `object.focus_field` is the third column, then `PrevField` from the third gives the first.

In #89's `a_text_the_column_refuses_keeps_the_panels_field_and_says_why_under_it`, after its last assertion that the editor is still open, add:

an assertion that nothing can be saved while it stands: press `Mod+S` and assert that no write was sent (the count of commands the fake backend received is what it was; `crate::testing::last_sent` shows how they are read). `App::save_blocked` is no help here: it answers `None` while only an editor is open (`src/app/editing.rs:311`).

- [ ] **Step 2: Run them to see them fail.**

Run: `~/.cargo/bin/cargo test --locked --lib walk`
Expected: does not compile, `no variant named NextField found for enum Advance`.

- [ ] **Step 3: Write the code.**

`src/model.rs`, `Advance`:

```rust
pub enum Advance {
    Stay,
    Down,
    Right,
    Left,
    /// To the row panel's next field that can be edited, in the page's
    /// column order: Enter and Tab in a field of the panel.
    NextField,
    /// To the one before it: Shift+Tab there.
    PrevField,
}
```

`src/ui/cell_editor.rs`, `in_panel`. Replace the three lines that force `Advance::Stay`:

```rust
    // In the panel a commit walks the row's fields, not the grid's cells.
    outcome.commit = outcome.commit.map(|then| match then {
        Advance::Stay => Advance::Stay,
        Advance::Left | Advance::PrevField => Advance::PrevField,
        Advance::Down | Advance::Right | Advance::NextField => Advance::NextField,
    });
```

and change `in_panel`'s doc comment: "Enter and Tab take the text and ask for the row's next field, Shift+Tab for the one before".

`src/app/editing.rs`, after `back_to_field`:

```rust
    /// The field a commit in the row panel's field of the column `col`
    /// walks to: the selected row's next that can be edited, in the page's
    /// column order, or the one before it. None at the row's end, and for
    /// a commit that stays.
    pub(super) fn field_after(
        &self,
        tab: ConnTabId,
        id: TabId,
        col: usize,
        then: Advance,
    ) -> Option<usize> {
        let forward = match then {
            Advance::NextField => true,
            Advance::PrevField => false,
            Advance::Stay | Advance::Down | Advance::Right | Advance::Left => return None,
        };
        self.table(tab, id, |table, object| {
            let row = object.selection?.row;
            let free = |col: &usize| table.lock(CellPos { row, col: *col }).is_none();
            if forward {
                (col + 1..table.page.columns.len()).find(free)
            } else {
                (0..col).rev().find(free)
            }
        })
        .flatten()
    }
```

`src/app.rs`, `Action::CommitEdit`:

```rust
            Action::CommitEdit { tab, id, then } => {
                let field = self.panel_field(tab, id);
                if self.close_editor(tab, id, false) {
                    // In the panel the keyboard goes to the field the
                    // commit walks to, or back to the one that was edited.
                    if let Some(col) = field {
                        let to = self.field_after(tab, id, col, then).unwrap_or(col);
                        self.back_to_field(tab, id, to);
                        return;
                    }
                    let (rows, cols) = match then {
                        Advance::Stay | Advance::NextField | Advance::PrevField => (0, 0),
                        Advance::Down => (1, 0),
                        Advance::Right => (0, 1),
                        Advance::Left => (0, -1),
                    };
```

The rest of the arm is as it was. `back_to_field` keeps its terminal exception until task 5.

In #89's `enter_and_f2_on_a_value_edit_it_and_the_keyboard_comes_back_to_it`, the part from "Tab commits as Enter does, and stays" asserts what this task ends: cut it there (the new test covers Tab).

- [ ] **Step 4: Run the tests, then the four checks.**

Run: `~/.cargo/bin/cargo test --locked --lib row_panel walk field`
Expected: PASS. Then the four checks: all pass.

- [ ] **Step 5: Commit.**

```bash
git add -A && git commit -S -m "Walk the row panel's fields with Enter and Tab" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: A click on a value edits it

The value of a field that can be edited, or that is locked for a reason of its own, becomes one control. A click opens the editor (on a locked one it says why). Under the pointer the control shows the design's hover state. With the keyboard on it, it wears an inset ring, and its cell becomes the grid's selected cell. The double-click and the pencil of a plain value go; a document keeps its pencil.

**Files:**
- Modify: `src/ui/row_panel.rs` (`field`, `value_of`, `Reading`, `Shown`, `outline`, new `field_stop` and `focused_field`)
- Modify: `src/app.rs` (`Action::FieldFocused`)
- Test: `src/ui/mod.rs`

- [ ] **Step 1: Write the failing tests.** Replace `a_double_click_on_a_value_edits_it_in_the_row_panel_and_one_click_does_not` with:

```rust
    #[test]
    fn a_click_on_a_value_edits_it_in_the_row_panel() {
        for look in Look::ALL {
            let (mut harness, tab, id) = form_row(look, 2);
            let at = panel_text(&harness, "user3@example.com").center();
            click_at(&mut harness, at);
            assert_eq!(
                form_editor(&harness, tab, id),
                Some(((2, 1), true)),
                "{}",
                look.name
            );
            assert_eq!(
                editor_text(&harness, tab, id).as_deref(),
                Some("user3@example.com"),
                "{}",
                look.name
            );
            // Committed, the same cell is pending in the grid: one set.
            type_text(&mut harness, "x");
            harness.press(Key::Enter, Modifiers::NONE);
            assert_eq!(
                pending_text(&harness, tab, id, (2, 1)).as_deref(),
                Some("user3@example.comx"),
                "{}",
                look.name
            );
            // The key's value takes no edit: a click says why.
            let at = panel_text(&harness, "3").center();
            click_at(&mut harness, at);
            let now = edits(&harness, tab, id);
            assert!(now.editor.is_none(), "{}", look.name);
            assert!(now.why.is_some(), "{}", look.name);
        }
    }

    #[test]
    fn a_field_is_one_control_named_by_its_column_type_and_value() {
        for look in Look::ALL {
            let (mut harness, _, _) = form_row(look, 1);
            let tree = harness.settle();
            let named = crate::testing::labels(&tree);
            let has = |start: &str| named.iter().any(|name| name.starts_with(start));
            assert!(has("email, "), "{}: {named:?}", look.name);
            assert!(
                named
                    .iter()
                    .any(|name| name.starts_with("email, ") && name.ends_with("user2@example.com")),
                "{}",
                look.name
            );
            // The key says it cannot be edited, and why.
            assert!(
                named
                    .iter()
                    .any(|name| name.starts_with("id, ") && name.contains(", read-only: ")),
                "{}",
                look.name
            );
        }
    }

    #[test]
    fn the_field_that_has_the_keyboard_is_the_grids_selected_cell() {
        for look in desktop_looks() {
            let (mut harness, tab, id) = form_row(look, 1);
            let at = panel_text(&harness, "user2@example.com").center();
            click_at(&mut harness, at);
            harness.press(Key::Escape, Modifiers::NONE);
            // The edit is dropped, the keyboard is on the field, and the
            // grid's selection is its cell.
            assert_eq!(selected(&harness, tab, id), Some((1, 1)), "{}", look.name);
            let stop = crate::ui::row_panel::field_stop(tab, id, 1);
            assert!(
                harness.ctx.memory(|memory| memory.has_focus(stop)),
                "{}",
                look.name
            );
        }
    }
```

Rewrite the tests of #89 that this ends:
- `a_fields_pencil_edits_its_value_in_the_row_panel`: no plain value has a pencil. Keep its body from "The field stands where the value stood" on, started by `click_at` on the value; drop the `has_button(.., "Edit email")` assertions; rename it `the_panels_field_stands_where_its_value_stood`.
- `a_pencil_clicked_while_another_field_is_edited_opens_its_own_and_keeps_the_text`: the same with clicks on the two values; rename `a_value_clicked_while_another_field_is_edited_opens_its_own_and_keeps_the_text`.
- `a_tall_values_pencil_opens_the_popover_at_its_cell`: a document keeps its pencil, so this stands if its value is a document; if it is a long text, start it with a click on the value.
- `a_value_that_can_be_edited_shows_a_fields_outline_under_the_pointer`: assert the fill as well (a rectangle painted in `palette.surface` behind the value while the pointer is over it, none when it is away).
- `a_row_that_cannot_be_edited_has_no_pencils`: also assert a click on a value opens no editor and that no node's name starts with `"email, "` (the values are text to read, not controls). Rename `a_row_that_cannot_be_edited_has_no_controls`.
- Every test of this section that starts an edit with `harness.click("Edit email")` or `"Edit qty"` starts it with `click_at(&mut harness, panel_text(&harness, <the value>).center())` instead, and loses its assertions about those two buttons: task 1's `enter_and_tab_in_a_field_commit_and_walk_to_the_next_that_can_be_edited`, `mod_s_saves_from_the_panels_field_with_what_is_typed`, `the_terminals_escape_keeps_a_panel_edit_and_ctrl_c_drops_it`, `a_text_the_column_refuses_keeps_the_panels_field_and_says_why_under_it`, `a_field_whose_cell_the_grid_is_editing_says_so` (which also asserts the field has no pencil: now that its value is no control, so a click on the stand-in opens nothing).
- `a_pencil_that_has_the_keyboard_edits_by_enter_and_f2_and_gets_it_back`: only a document has a pencil. Run it on a document's field (the fixture of `a_tall_values_pencil_opens_the_popover_at_its_cell`), where Enter and F2 open the popover at the cell; or delete it if that test already says so.
- `the_footer_says_why_a_row_cannot_be_edited_and_edit_waits` reads the pencils too: drop that part here (task 4 rewrites its footer half).
- `enter_and_f2_on_a_value_edit_it_and_the_keyboard_comes_back_to_it`: its first click now edits. Start it from `Action::FocusFields`'s stand-in for this task: click the value, press Esc (the keyboard is then on the field), and go on from "Enter edits".

- [ ] **Step 2: Run them to see them fail.**

Run: `~/.cargo/bin/cargo test --locked --lib a_click_on_a_value a_field_is_one_control the_field_that_has_the_keyboard`
Expected: does not compile, `cannot find function field_stop in module crate::ui::row_panel`.

- [ ] **Step 3: Write the code.**

`src/ui/row_panel.rs`, beside `width`:

```rust
/// The id of the control a field of the row panel is: its value, or its
/// pencil where the value has clicks of its own (a tree, a list). The
/// keyboard is on the field while it is on this. One per column, whatever
/// row the panel shows: the keyboard stays on a column's field when the
/// selection steps to another row.
pub fn field_stop(tab: ConnTabId, id: TabId, col: usize) -> Id {
    Id::new(("row-panel-field", tab, id, col))
}

/// The column of the panel's field that has the keyboard, of the tab's
/// `columns`: what `ui/keys.rs` asks before it gives a key to the grid.
pub fn focused_field(
    ctx: &egui::Context,
    tab: ConnTabId,
    id: TabId,
    columns: usize,
) -> Option<usize> {
    let focused = ctx.memory(|memory| memory.focused())?;
    (0..columns).find(|col| field_stop(tab, id, *col) == focused)
}
```

`Reading` gains `control: bool` in place of `editable` ("the row's form makes one control of the value: its text takes no caret"), true for `Part::Editable` and for `Part::Locked` with a reason. In `value_of`, the value's `TextEdit` is built with `.interactive(!control)`, and `focus::hint` is called only where it is interactive. `Shown::place` is set as today; `Shown::text` only for an interactive text.

In `field`, a placeholder shape goes in before the value so the states can be painted behind it, as `draw` does for a document's backdrop:

```rust
    let behind = ui.painter().add(egui::Shape::Noop);
```

and after `value_of`, in place of the double-click, the hover stroke and the Enter/F2 block:

```rust
    // The value as one control: a click edits it, or says why it cannot
    // be. A document's control is its pencil, which wears the same id.
    let stop = field_stop(tab, tab_id, col);
    let control = shown.place.filter(|_| editable || locked.is_some()).map(|place| {
        // The box is painted 30 tall, but only the value's own place takes
        // the pointer: the box reaches under the label line's Copy and
        // over "Show all", and would take their clicks.
        let frame = outline(place);
        let response = ui.interact(place, stop, Sense::click());
        response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, &said));
        if editable && response.hovered() {
            // The design's hover: the surface's fill, a border inside it,
            // a pencil at its right, and the text cursor.
            ui.ctx().set_cursor_icon(egui::CursorIcon::Text);
            let corner = CornerRadius::same(look.radius);
            if !look.terminal {
                ui.painter().set(
                    behind,
                    egui::epaint::RectShape::filled(frame, corner, palette.surface),
                );
            }
            let color = if look.terminal {
                palette.accent
            } else {
                palette.border
            };
            ui.painter()
                .rect_stroke(frame, corner, Stroke::new(1.0, color), StrokeKind::Inside);
            Icon::Pencil.image(palette.secondary, 12.0).paint_at(
                ui,
                Rect::from_center_size(
                    pos2(frame.right() - 8.0 - 6.0, frame.center().y),
                    vec2(12.0, 12.0),
                ),
            );
        }
        focus::hint(
            ui,
            &response,
            frame,
            focus::Ring::Inset {
                radius: look.radius,
            },
        );
        response
    });
    // Every field of a row that can be edited has a stop, so Up and Down
    // never ask for one that is not there: its value's control, else its
    // pencil, else its label line (a binary, a locked document, a field
    // the grid is editing).
    let bare = (control.is_none() && pencil.is_none() && part != Part::Read)
        .then(|| ui.interact(line, stop, Sense::click()));
    let stop_of = control.as_ref().or(pencil.as_ref()).or(bare.as_ref());
    let mut edit = stop_of.is_some_and(egui::Response::clicked);
    if let Some(stop) = stop_of
        && stop.has_focus()
    {
        // The field that takes the keyboard becomes the grid's selected
        // cell. Said once, as it takes it: a selection moved since (a
        // click in the grid, a key of the grid's) is not pulled back.
        if stop.gained_focus() && selected_col != col {
            actions.push(Action::FieldFocused {
                tab,
                id: tab_id,
                col,
            });
        }
        // Up and Down are the panel's, to step its fields with, and Esc
        // its way back to the grid: egui neither moves the keyboard off
        // the stop with them nor drops it. Whatever the stop is.
        let keys = egui::EventFilter {
            vertical_arrows: true,
            escape: true,
            ..Default::default()
        };
        ui.memory_mut(|memory| memory.set_focus_lock_filter(stop.id, keys));
        if !look.terminal {
            edit |= ui.input_mut(|input| consume_press(input, egui::Modifiers::NONE, egui::Key::F2));
        }
    }
```

The `Part::InGrid` arm returns before this today: let it fall through to the stop (its label line) instead, so the field can be stood on. Its `edit` is ignored. A click does not give a `Sense::click()` control the keyboard in egui: the control asks for it itself (`response.request_focus()` where it was clicked and no editor opened, which is the locked field's case; an editor that opens takes the keyboard anyway).

`said` is the field's name for a screen reader, built once above:

```rust
    // "<column>, <type>, <value>", then what a pending one was and why a
    // locked one cannot be edited.
    let mut said = format!(
        "{}, {}, {}",
        format::display_safe(&column.name),
        column.type_name,
        formatted.map_or("", |text| text.short.as_str()),
    );
    if let Some(pending) = pending_field(texts, col) {
        said = format!("{said}, {} {}", gettext(locale, "changed from"), pending.was);
    }
    if let Some(reason) = &locked {
        said = format!("{said}, {}: {reason}", gettext(locale, "read-only"));
    }
```

`selected_col` is the selected cell's column, a new argument of `field` (pack `row` and it as one `CellPos` to stay within the argument count: `draw` has `cell`). The pencil is drawn only where `doc.is_some()` or the value is an array list or a binary (`shown.place` is `None`): build it inside a child `Ui` with `.id_salt` so that its response's id is `stop` (`ui.interact(place, stop, Sense::click())` in place of `caption_button`'s own allocation; give `caption_button` an optional id). The owed keyboard (`form.focus == Some(col)`) goes to `stop_of`.

`outline`: the design's box is 30 tall and stands 8 clear of the value at its sides.

```rust
fn outline(place: Rect) -> Rect {
    let pad = ((30.0 - place.height()) / 2.0).max(3.0);
    place.expand2(vec2(8.0, pad))
}
```

A locked field's value is drawn in `palette.secondary` (pass the colour into `value_of` through `Reading`).

`src/app.rs`, `Action::FieldFocused`: the field has the keyboard, so its cell is the selected one.

```rust
            Action::FieldFocused { tab, id, col } => {
                if let Some(object) = self.object_tab_mut(tab, id) {
                    // The request that was met is forgotten: another edit
                    // may have ended since the frame that drew the field.
                    if object.focus_field == Some(col) {
                        object.focus_field = None;
                    }
                    // The field that has the keyboard is the grid's
                    // selected cell. The row's texts are the row's: none
                    // is formatted again. What a locked field said was
                    // said of the field the keyboard left.
                    if let Some(cell) = object.selection.as_mut()
                        && cell.col != col
                    {
                        cell.col = col;
                        object.edits.why = None;
                    }
                }
            }
```

Update `Action::FieldFocused`'s doc comment in `src/model.rs` to say both. The panel pushes it where the owed keyboard was given, as before, and where a field takes the keyboard (`gained_focus`) and is not the selected column.

- [ ] **Step 4: Run the tests, then the four checks.** Expected: PASS.

- [ ] **Step 5: Look at it.** Render a throwaway scene of the panel with the pointer over a value, in each look (`src/shots.rs`'s `row_form_field` shows how a scene is set up; do not commit the scene or its output). Compare by eye with the inspector of the "Table view, editable" artboard: the fill, the border, the pencil, the 30 pt box.

- [ ] **Step 6: Commit.**

```bash
git add -A && git commit -S -m "Edit a value of the row panel by a click on it" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: A focused field's keys (macOS and Windows)

With the keyboard on a field: Enter and F2 edit it, a printable character edits it starting with that character, `Mod+Backspace` makes it NULL where its column allows, `Mod+Z` puts back what was loaded, Up and Down step to the field above and below, Esc gives the keys back to the grid on the cell of that column. All of it is read in `ui/keys.rs`, where the grid's keys are.

**Files:**
- Modify: `src/model.rs` (`Action::EditField { start }`, `Action::MoveField`)
- Modify: `src/app.rs`, `src/app/editing.rs` (`move_field`)
- Modify: `src/ui/keys.rs` (`handle`, `editing_keys`)
- Modify: `src/ui/row_panel.rs` (the F2 it read itself goes)
- Modify: `src/shots.rs` (`row_form_field` and `row_form_locked` build `Action::EditField`, at lines 386 and 405: they pass `start: EditStart::Value`. The file is behind the `shots` feature, so the four checks do not see it: run the shots lint in this task.)
- Test: `src/ui/mod.rs`, `src/app.rs`

- [ ] **Step 1: Write the failing tests.** A helper first, since `Mod+I` comes in task 4:

```rust
    /// Puts the keyboard on the row panel's field of the column `col`, as
    /// the reducer asks the panel to.
    fn focus_field(harness: &mut Harness, tab: ConnTabId, id: TabId, col: usize) {
        let workspace = harness.app.workspace_mut(tab).unwrap();
        workspace.object_tab_mut(id).unwrap().focus_field = Some(col);
        harness.settle();
        let stop = crate::ui::row_panel::field_stop(tab, id, col);
        assert!(harness.ctx.memory(|memory| memory.has_focus(stop)));
    }
```

```rust
    #[test]
    fn typing_on_a_focused_field_edits_it_from_what_was_typed() {
        for look in desktop_looks() {
            let (mut harness, tab, id) = form_row(look, 1);
            focus_field(&mut harness, tab, id, 1);
            type_text(&mut harness, "z");
            assert_eq!(form_editor(&harness, tab, id), Some(((1, 1), true)), "{}", look.name);
            assert_eq!(editor_text(&harness, tab, id).as_deref(), Some("z"), "{}", look.name);
            // The grid opened no editor of its own on the selected cell.
            harness.press(Key::Escape, Modifiers::NONE);
            assert!(edits(&harness, tab, id).editor.is_none(), "{}", look.name);
        }
    }

    #[test]
    fn up_and_down_step_the_fields_and_escape_gives_the_keys_to_the_grid() {
        for look in desktop_looks() {
            let (mut harness, tab, id) = form_row(look, 1);
            focus_field(&mut harness, tab, id, 1);
            harness.press(Key::ArrowDown, Modifiers::NONE);
            assert_eq!(selected(&harness, tab, id), Some((1, 2)), "{}", look.name);
            // Up twice: over email to the key, which is locked and still a
            // field to stand on. Past the first there is nothing.
            harness.press(Key::ArrowUp, Modifiers::NONE);
            harness.press(Key::ArrowUp, Modifiers::NONE);
            harness.press(Key::ArrowUp, Modifiers::NONE);
            assert_eq!(selected(&harness, tab, id), Some((1, 0)), "{}", look.name);
            harness.press(Key::ArrowDown, Modifiers::NONE);
            harness.press(Key::Escape, Modifiers::NONE);
            // The keys are the grid's, on the cell of the field's column:
            // Down moves a row.
            assert_eq!(selected(&harness, tab, id), Some((1, 1)), "{}", look.name);
            harness.press(Key::ArrowDown, Modifiers::NONE);
            assert_eq!(selected(&harness, tab, id), Some((2, 1)), "{}", look.name);
        }
    }

    #[test]
    fn escape_drops_one_fields_edit_and_keeps_the_others() {
        for look in desktop_looks() {
            let (mut harness, tab, id) = form_row(look, 1);
            make_pending(&mut harness, tab, id, (1, 2), "7");
            focus_field(&mut harness, tab, id, 1);
            harness.press(Key::Enter, Modifiers::NONE);
            type_text(&mut harness, "x");
            harness.press(Key::Escape, Modifiers::NONE);
            assert_eq!(pending_text(&harness, tab, id, (1, 1)), None, "{}", look.name);
            assert_eq!(
                pending_text(&harness, tab, id, (1, 2)).as_deref(),
                Some("7"),
                "{}",
                look.name
            );
        }
    }

    #[test]
    fn mod_z_on_the_grid_puts_back_what_a_field_changed() {
        for look in desktop_looks() {
            let (mut harness, tab, id) = form_row(look, 1);
            focus_field(&mut harness, tab, id, 1);
            harness.press(Key::Enter, Modifiers::NONE);
            type_text(&mut harness, "x");
            harness.press(Key::Enter, Modifiers::NONE);
            assert!(pending_text(&harness, tab, id, (1, 1)).is_some(), "{}", look.name);
            // Back to the grid (the commit left the keyboard on qty's
            // field), and on that cell.
            harness.press(Key::Escape, Modifiers::NONE);
            select(&mut harness, tab, id, (1, 1));
            harness.press(Key::Z, Modifiers::COMMAND);
            assert_eq!(pending_text(&harness, tab, id, (1, 1)), None, "{}", look.name);
        }
    }

    #[test]
    fn clearing_a_field_is_the_empty_string_and_mod_backspace_is_null() {
        for look in desktop_looks() {
            let (mut harness, tab, id) = form_row(look, 1);
            // Cleared in its editor, a text is the empty string.
            focus_field(&mut harness, tab, id, 1);
            harness.press(Key::Enter, Modifiers::NONE);
            harness.press(Key::A, Modifiers::COMMAND);
            harness.press(Key::Backspace, Modifiers::NONE);
            harness.press(Key::Enter, Modifiers::NONE);
            assert_eq!(pending_text(&harness, tab, id, (1, 1)).as_deref(), Some(""), "{}", look.name);
            // Mod+Backspace on the field is NULL.
            focus_field(&mut harness, tab, id, 1);
            harness.press(Key::Backspace, Modifiers::COMMAND);
            let cell = edits(&harness, tab, id).cells.get(&(1, 1)).expect("pending");
            assert!(matches!(cell.new, tabletist_db::NewValue::Null), "{}", look.name);
        }
    }
```

For the NOT NULL half of the spec's test 9, a reducer test in `src/app.rs` beside the tests of `Action::SetNull`: on a column that is not nullable, with the panel's field focused (`selection` on it), `SetNull` leaves the set empty. (`set_null` already refuses; the test pins it for the panel's path.) `email` is not nullable in the fixture (`src/testing.rs:644`): the test above runs on a copy of `with_quantities` whose structure marks it nullable (answer the structure again with `answer_structure`, as `with_quantities` does).

- [ ] **Step 2: Run them to see them fail.** Expected: `typing_on_a_focused_field…` fails with `left: None` (nothing reads the text); the others fail on the first key.

- [ ] **Step 3: Write the code.**

`src/model.rs`: `EditField` gains `start: EditStart` (its doc: "From `start`, as `EditCell` is"), and

```rust
    /// Put the keyboard on the row panel's field `by` fields after the one
    /// of the column `from` (before it, below zero), in the page's column
    /// order, locked ones too. At the row's ends it stays.
    MoveField {
        tab: ConnTabId,
        id: TabId,
        from: usize,
        by: isize,
    },
```

`src/app.rs`:

```rust
            Action::EditField {
                tab,
                id,
                cell,
                start,
            } => self.edit_cell(tab, id, cell, start, EditorPlace::Panel),
            Action::MoveField { tab, id, from, by } => {
                if let Some(object) = self.object_tab_mut(tab, id)
                    && let Some(columns) = object.rows.value.as_ref().map(|page| page.columns.len())
                {
                    let last = columns.saturating_sub(1);
                    object.focus_field = Some(from.saturating_add_signed(by).min(last));
                    // What a locked field said was said of the one left.
                    object.edits.why = None;
                }
            }
```

`by: 0` asks for the field of `from` itself: task 5 uses it.

Add `Action::MoveField` nowhere in `dropped_under_a_prompt`: it edits nothing. `Action::EditField`'s other builders (the panel's click, tests) pass `EditStart::Value`.

`src/ui/keys.rs`, in `handle`, after `grid` is known:

```rust
    // The row panel's field that has the keyboard, of the table on
    // screen: its keys are read here with the grid's, so one key is never
    // both's.
    let field = object.filter(|_| grid).and_then(|(tab, id)| {
        let object = app.workspace(tab)?.object_tab(id)?;
        let columns = object.rows.value.as_ref()?.columns.len();
        crate::ui::row_panel::focused_field(ctx, tab, id, columns)
    });
```

and where `editing_keys` is called:

```rust
        let keyboard = !terminal && !editing && grid && !tree_arrows && !focused;
        let on_field = field.filter(|_| !terminal && !editing);
        editing_keys(app, ctx, keyboard, on_field, &mut actions);
```

`editing_keys` takes `field: Option<usize>`. Its early return becomes `if !(keyboard || field.is_some()) || open { return; }`, and inside the closure:

```rust
        // On a field of the row panel the same keys act on its cell, which
        // is the selected one, and open its editor in the panel.
        let edit = |cell: crate::model::CellPos, start| match field {
            Some(col) => Action::EditField {
                tab,
                id,
                cell: crate::model::CellPos { row: cell.row, col },
                start,
            },
            None => Action::EditCell {
                tab,
                id,
                cell,
                start,
            },
        };
```

used by the Enter/F2 arm and the typing arm in place of the two `Action::EditCell` literals. After the Enter/F2 arm, before the typing:

```rust
        if let Some(col) = field {
            // Up and Down step the fields; Esc gives the keys back to the
            // grid, on the cell of this field's column.
            for (key, by) in [(Key::ArrowUp, -1), (Key::ArrowDown, 1)] {
                if input.consume_key(Modifiers::NONE, key) {
                    actions.push(Action::MoveField {
                        tab,
                        id,
                        from: col,
                        by,
                    });
                }
            }
            if consume_press(input, Modifiers::NONE, Key::Escape) {
                left = Some(crate::ui::row_panel::field_stop(tab, id, col));
                actions.push(Action::GridKeys(tab));
            }
        }
```

with `let mut left = None;` before `ctx.input_mut` and, after it, `if let Some(stop) = left { ctx.memory_mut(|memory| memory.surrender_focus(stop)); }`. `Mod+Backspace` and `Mod+Z` need no change: they act on the selection, which the focused field's cell is.

`src/ui/row_panel.rs`: remove the `consume_press(.., Key::F2)` the field read itself in task 2. Enter and F2 are consumed in `keys.rs` before the panel is drawn, so the control's own press (egui presses a focused control on Enter) no longer fires for them; Space still presses it.

- [ ] **Step 4: Run the tests, then the four checks.** Expected: PASS.

- [ ] **Step 5: Commit.**

```bash
git add -A && git commit -S -m "Give a focused field of the row panel the keys a cell has" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: `Mod+I` focuses the fields, and Edit goes

`Mod+I` shows the panel and puts the keyboard on the selected row's first field that can be edited. The footer is Duplicate and Delete and the hint line. `Action::EditRow`, `App::edit_row`, the footer's Edit and the note "⌘I edit" go.

**Files:**
- Modify: `src/model.rs` (`Action::FocusFields` in place of `EditRow`)
- Modify: `src/app.rs`, `src/app/editing.rs` (`focus_fields` in place of `edit_row`; `dropped_under_a_prompt`)
- Modify: `src/ui/keys.rs` (`editing_keys`, `editing_letters`, `SHORTCUTS`)
- Modify: `src/ui/row_panel.rs` (`editing_footer`, its caller)
- Modify: `src/shots.rs` (a scene that sends `EditRow`, if one does)
- Test: `src/ui/mod.rs`, `src/app.rs`, `src/ui/keys.rs`

- [ ] **Step 1: Write the failing tests.** Replace `mod_i_edits_the_row_in_the_row_panel_in_every_look` with:

```rust
    #[test]
    fn mod_i_shows_the_panel_and_focuses_the_first_field_that_can_be_edited() {
        for look in Look::ALL {
            let (mut harness, tab, id) = form_row(look, 1);
            // The panel is hidden, and the grid's cursor is on the key.
            harness.app.apply(Action::ToggleRowPanel(tab));
            harness.settle();
            assert!(!harness.app.workspace(tab).unwrap().row_panel, "{}", look.name);
            harness.press(Key::I, Modifiers::COMMAND);
            assert!(harness.app.workspace(tab).unwrap().row_panel, "{}", look.name);
            // No editor: the keyboard is on email, the first that can be
            // edited, and its cell is the selected one.
            assert!(edits(&harness, tab, id).editor.is_none(), "{}", look.name);
            let stop = crate::ui::row_panel::field_stop(tab, id, 1);
            assert!(harness.ctx.memory(|memory| memory.has_focus(stop)), "{}", look.name);
            assert_eq!(selected(&harness, tab, id), Some((1, 1)), "{}", look.name);
        }
    }
```

Rewrite, for what replaces what they assert:
- `mod_i_reaches_the_row_from_a_caret_in_a_value_and_leaves_a_sql_editor_alone`: keep the SQL editor half; the first half becomes "from the filter's field".
- `mod_i_on_a_row_that_cannot_be_edited_shows_the_panel_and_its_reason`: the panel shows, no field has the keyboard, the reason is recorded (`edits.why`).
- `the_footer_says_why_a_row_cannot_be_edited_and_edit_waits`: its note half stays until task 6; its Edit half goes (Duplicate and Delete are disabled with the row's reason).
- `the_row_panels_edit_works_and_duplicate_and_delete_wait` becomes `the_row_panels_duplicate_and_delete_wait`: `footer_buttons` returns two, both disabled, with a value pending as without; no button is named "Edit".
- `the_row_panel_names_the_keys_that_edit_its_row`: the note reads "Click a value or press ⌘I to edit" (`Ctrl+I` on Windows; build it as the footer does, from `look.command_key()`).
- `the_terminals_e_edits_the_row_in_the_row_panel`: delete the `e` half here; task 5 gives the terminal its test.
- In `src/ui/keys.rs`: `the_shortcut_table_names_the_keys_that_edit_a_cell` and `the_shortcut_table_names_omarchys_keys_that_edit` expect ("Mod+I", "Focus inspector fields").
- In `src/app.rs`: the reducer tests of `EditRow` (takes the selected cell, the first editable one, says why, opens a closed panel, dropped under a prompt) become tests of `FocusFields`: `focus_field` is the first editable column whatever cell is selected; under a row lock `focus_field` stays `None` and `edits.why` holds the lock; the panel opens; an editor open in the grid is closed with its text kept.

- [ ] **Step 2: Run them to see them fail.** Expected: does not compile, `no variant named FocusFields`.

- [ ] **Step 3: Write the code.**

`src/model.rs`, in place of `EditRow`:

```rust
    /// Put the keyboard on the selected row's fields in the row panel: show
    /// the panel, and focus the row's first field that can be edited. No
    /// editor opens. On a row no field of which can be, the panel says why.
    FocusFields {
        tab: ConnTabId,
        id: TabId,
    },
```

`src/app/editing.rs`, in place of `edit_row`:

```rust
    /// Shows the row panel and asks it to give the keyboard to the selected
    /// row's first field that can be edited, in the page's column order. A
    /// row with none keeps the keyboard where it is: the panel's first line
    /// says why no cell of it can be edited, and the terminal's mode line.
    pub(super) fn focus_fields(&mut self, tab: ConnTabId, id: TabId) {
        let found = self.table(tab, id, |table, object| {
            let cell = object.selection?;
            // The panel shows a row of the Data view only.
            if object.view != crate::model::ObjectView::Data {
                return None;
            }
            if let Some(lock) = table.row_lock(cell.row) {
                return Some((cell, Err(lock)));
            }
            let first = (0..table.page.columns.len())
                .find(|col| table.lock(CellPos { row: cell.row, col: *col }).is_none());
            Some((cell, Ok(first.unwrap_or(cell.col))))
        });
        let Some(Some((cell, found))) = found else {
            return;
        };
        // An editor open on a cell keeps its text.
        self.close_editor(tab, id, true);
        if let Some(workspace) = self.workspace_mut(tab) {
            workspace.row_panel = true;
            // As an edit asked for does: the keys are the table's, not
            // the tree's, whose `j` and `k` the terminal look would
            // otherwise go on reading.
            workspace.pane = Pane::Grid;
        }
        let Some(object) = self.object_tab_mut(tab, id) else {
            return;
        };
        match found {
            Ok(col) => object.focus_field = Some(col),
            Err(lock) => {
                object.edits.why = Some((cell, lock));
                object.edits.why_place = EditorPlace::Panel;
            }
        }
    }
```

`dropped_under_a_prompt`: `FocusFields` takes `EditRow`'s place (it can close an editor into the set the prompt asks about).

`src/ui/keys.rs`: in `editing_keys` the closure `edit_row` becomes `focus_fields` and pushes `Action::FocusFields { tab, id }`; its comment says what the key does now. In `editing_letters`, `"e"` leaves `mine` and its arm goes. `SHORTCUTS`:

```rust
    ("Mod+I", "Focus inspector fields", DESKTOP),
    ("Ctrl+L, Mod+I", "Focus inspector fields", TERMINAL),
    ("Up, Down, Esc", "Step the inspector's fields, back to the grid", DESKTOP),
```

and `e` leaves the Omarchy line of vim keys.

`src/ui/row_panel.rs`, `editing_footer`: returns nothing. The terminal's cells are two (`yy p duplicate`, `dd delete`), each half the width less the gap, dashed and at 55% as they are. The other looks' buttons are Duplicate and Delete, sharing the width. Under a row lock both say the row's reason under the pointer; otherwise "Duplicating and deleting rows arrive in a later version". The note, where the row can be edited, in the looks that are not the terminal's:

```rust
            let key = format!("{}I", look.command_key());
            let hint = Text::one(look, note_role, &gettext(locale, "Click a value or press"), palette.dim)
                .space(note_role, " ")
                .add(note_role, &key, palette.secondary)
                .space(note_role, " ")
                .add(note_role, &gettext(locale, "to edit"), palette.dim);
            widgets::paint_text(ui, inner.left(), y, hint);
```

The note under a row lock goes to the top of the panel in task 6; until then it stays in the footer. The terminal's head hint loses `e edit` (task 5 gives it its own).

- [ ] **Step 4: Run the tests, then the four checks and the shots lint.** Expected: PASS.

- [ ] **Step 5: Commit.**

```bash
git add -A && git commit -S -m "Reach the row panel's fields with Mod+I, and take its Edit button away" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 5: Omarchy's keys on the fields

`ctrl+l` from the grid focuses the fields, showing the panel. On a field: `j`/`k` step, `i` and Enter edit, `cc` replaces, `x` and `u` as on a cell, `ctrl+h` goes back to the grid's cell. A field with the keyboard is drawn on the selection's tone with a 2 pt accent bar at its left. After a commit or `ctrl+c` the keyboard is on a field, as in the other looks.

**Files:**
- Modify: `src/ui/keys.rs` (`handle`, `letters`, `editing_letters`)
- Modify: `src/app/editing.rs` (`back_to_field`)
- Modify: `src/ui/row_panel.rs` (the focused field's look, the head's hint)
- Test: `src/ui/mod.rs`

- [ ] **Step 1: Write the failing tests.**

```rust
    #[test]
    fn ctrl_l_focuses_the_rows_fields_in_the_terminal_look() {
        let look = Look::omarchy();
        let (mut harness, tab, id) = form_row(look, 1);
        harness.app.apply(Action::ToggleRowPanel(tab));
        harness.settle();
        harness.press(Key::L, Modifiers::CTRL);
        assert!(harness.app.workspace(tab).unwrap().row_panel);
        let stop = |col| crate::ui::row_panel::field_stop(tab, id, col);
        assert!(harness.ctx.memory(|memory| memory.has_focus(stop(1))));
        // `j` steps the fields, not the grid's rows.
        type_key(&mut harness, Key::J, "j");
        assert_eq!(selected(&harness, tab, id), Some((1, 2)));
        type_key(&mut harness, Key::K, "k");
        assert_eq!(selected(&harness, tab, id), Some((1, 1)));
        // `i` edits in the panel; Esc keeps what was typed, and the
        // keyboard is on the field again.
        type_key(&mut harness, Key::I, "i");
        assert_eq!(form_editor(&harness, tab, id), Some(((1, 1), true)));
        type_text(&mut harness, "x");
        harness.press(Key::Escape, Modifiers::NONE);
        assert_eq!(
            pending_text(&harness, tab, id, (1, 1)).as_deref(),
            Some("user2@example.comx")
        );
        assert!(harness.ctx.memory(|memory| memory.has_focus(stop(1))));
        // `u` puts it back; `cc` edits from nothing.
        type_key(&mut harness, Key::U, "u");
        assert_eq!(pending_text(&harness, tab, id, (1, 1)), None);
        type_key(&mut harness, Key::C, "c");
        type_key(&mut harness, Key::C, "c");
        assert_eq!(editor_text(&harness, tab, id).as_deref(), Some(""));
        harness.press(Key::C, Modifiers::CTRL);
        // ctrl+h: the keys are the grid's, on that column's cell.
        harness.press(Key::H, Modifiers::CTRL);
        assert!(!harness.ctx.memory(|memory| memory.has_focus(stop(1))));
        type_key(&mut harness, Key::J, "j");
        assert_eq!(selected(&harness, tab, id), Some((2, 1)));
    }
```

The vim letters are sent with `type_key` (`src/ui/mod.rs:5339`), a key press with its text: `letters` reads nothing in a frame without a key press, and reads `j` and `k` as keys. `type_text` is for what is typed into the open editor.

(How the harness sends `ctrl+c` as the terminal's drop: copy it from `the_terminals_escape_keeps_a_panel_edit_and_ctrl_c_drops_it`, which this task also changes: after Esc and after `ctrl+c` the keyboard is on the field, not the grid.) `the_terminal_panel_has_its_head_and_its_foot` expects the new hint, below.

- [ ] **Step 2: Run it to see it fail.** Expected: FAIL at the first `has_focus`: `ctrl+l` steps to the panel's close button.

- [ ] **Step 3: Write the code.**

`src/ui/keys.rs`, `handle`, the pane step:

```rust
            None if app.look.terminal && !editing => {
                if let Some(index) = pressed([(Modifiers::CTRL, Key::H), (Modifiers::CTRL, Key::L)])
                {
                    match (index, field, object) {
                        // From a field of the row panel, back to the grid,
                        // on the cell of that column.
                        (0, Some(col), Some((tab, id))) => {
                            let stop = crate::ui::row_panel::field_stop(tab, id, col);
                            ctx.memory_mut(|memory| memory.surrender_focus(stop));
                            actions.push(Action::GridKeys(tab));
                        }
                        // From the grid, to the row's fields: the panel is
                        // shown for it.
                        // With no row selected there are none: the key
                        // steps the panes, as it did.
                        (1, None, Some((tab, id))) if grid && has_row && from == Region::Grid => {
                            actions.push(Action::FocusFields { tab, id });
                        }
                        _ => focus::step(ctx, index == 1, Some(&Region::PANES), Some(from)),
                    }
                }
            }
```

(`has_row` is whether the table's tab has a selection; read it where `grid` is.)

`letters` takes `field: Option<usize>`. The grid's `j/k/h/l` loop runs only with `field.is_none()`. Before it:

```rust
    // On a field of the row panel `j` and `k` step the fields.
    if let (Some(col), Some(id)) = (field, active) {
        for (key, by) in [(Key::J, 1), (Key::K, -1)] {
            if pressed(key) {
                actions.push(Action::MoveField {
                    tab,
                    id,
                    from: col,
                    by,
                });
            }
        }
    }
```

The letters that edit run on a field as on the grid: the guard `!crate::ui::focus::on_control(ctx)` becomes `(field.is_some() || !crate::ui::focus::on_control(ctx))`, and `editing_letters` takes `field` and builds `Action::EditField { cell: CellPos { row: cell.row, col }, start }` where it is `Some`, as `editing_keys` does in task 3. `x` and `u` act on the selection, which is the field's cell.

`src/app/editing.rs`: `back_to_field` loses its `if self.look.terminal { return; }` and the sentence of its comment that explains it.

The terminal's Esc is not a commit or a cancel: it leaves insert mode with the text kept, which the editor reports as `Outcome::left` (`src/ui/cell_editor.rs:134`), as it reports a click elsewhere, and `Action::LeaveEdit` gives no keyboard back. Tell the two apart: `Outcome` gains `kept: bool`, set beside `left` where `leaving_keys` takes the terminal's Esc. In `row_form::Form::editing`, an outcome that was `kept` queues, after its `LeaveEdit`, `Action::MoveField { tab, id, from: cell.col, by: 0 }`: the keyboard goes back to the field. A click elsewhere stays a plain `LeaveEdit`. The grid's field ignores `kept`.

Space on a focused field presses it, in this look as in the others: it edits. (On the grid Space shows and hides the panel; that key is the grid's, and is not read with the keyboard on a control.)

`src/ui/row_panel.rs`: in the terminal look the control with the keyboard paints `palette.selection` behind the value (through `behind`) and a 2 pt bar in `palette.accent` at the frame's left, and asks `focus::hint(.., focus::Ring::Own)` so `focus.rs` draws no ring of its own. The head's hints, in the order they give way:

```rust
                let hints: [&[widgets::Hint<'_>]; 4] = [
                    &[steps, ("ctrl+l", "focus", true), ("i", "edit field", true)],
                    &[steps, ("ctrl+l", "focus", true)],
                    &[steps],
                    &[("[ ]", "", true)],
                ];
```

with `offered` starting at index 2 where the row cannot be edited, and the fallback `hints[3]` where it was `hints[2]`.

- [ ] **Step 4: Run the tests, then the four checks.** Expected: PASS.

- [ ] **Step 5: Commit.**

```bash
git add -A && git commit -S -m "Give Omarchy's keys to the row panel's fields" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

**The first place to stop.** After this task the panel edits as the design does, by pointer and by keys, in every look. Tasks 6 and 7 are how a field looks once it holds a change.

---

### Task 6: What a field says of its cell

A pending field wears the amber fill and left bar on its value, and its label line ends in "was x · revert". A field whose text its column refuses, or whose row's statement failed, wears the red fill and border, with the message under it. The header counts the row's unsaved changes. A row that cannot be edited says why in one line at the top of the panel.

**Files:**
- Modify: `src/model.rs` (`PendingField::trouble`, `Action::RevertCell { cell }`)
- Modify: `src/app.rs` (`row_fields`, where `PendingField` is built; `Action::RevertCell`)
- Modify: `src/ui/row_panel.rs` (`field`, `was`, the header, the row lock's line, `editing_footer`)
- Modify: `src/ui/keys.rs` (the two `RevertCell` it builds pass `cell: None`), `src/ui/object_tabs.rs` (line 570 builds one: `cell: None`)
- Test: `src/ui/mod.rs`, `src/app.rs`

- [ ] **Step 1: Write the failing tests.** In `src/ui/mod.rs`:

- `a_pending_field_says_what_it_was_and_reverts`: make `(1, 1)` pending; the panel paints "was user2@example.com" on the label's line (its rectangle's centre is above the value's top); a link named "Revert email" is there; clicking it empties the set, whatever cell is selected (select `(1, 2)` first).
- `a_field_to_fix_says_what_its_column_refuses`: type `abc` into `qty` in the panel and click another value (the keyboard leaves: the cell is kept to fix). The panel paints the message the grid's cell gives under the pointer (`cell_editor::problem_text` for the problem), in `palette.danger`, and `harness.app.save_blocked(tab, id)` is `Some(SaveBlock::ToFix)`.
- `the_row_panels_header_counts_its_rows_unsaved_changes` (desktop looks): with two cells of the row pending the header paints "2 unsaved changes" and not the table's name; with none, the table's name.
- `a_read_only_connection_says_so_once_and_edits_nothing`: `connect_fake_as(true)` and a table open on it, a row selected. The panel paints "Read-only connection" once, above the first field's label; a click on a value opens no editor; `Mod+I` focuses no field; Duplicate and Delete are disabled. In every look.
- `a_fields_pending_value_is_there_when_its_row_comes_back`: make `(1, 1)` pending through the panel, step to the next row with the header's Next and back with Previous; the set still holds it and the panel paints the new value.
- `an_edit_made_in_a_field_is_reviewed_as_the_same_edit_made_on_its_cell`: two harnesses. In one, edit `(1, 1)` to `x@example.com` in the panel (click, select all, type, Enter); in the other, the same in the grid (`Action::EditCell`, type, Enter). `harness.app.review_whole(tab, id)` of the two are equal (compare their statements' text).

Rewrite `the_row_panel_shows_a_pending_value_and_what_it_was`: "was x" is on the label's line now.

- [ ] **Step 2: Run them to see them fail.** Expected: the first fails with no node named "Revert email".

- [ ] **Step 3: Write the code.**

`src/model.rs`:

```rust
pub struct PendingField {
    /// The new value, as a value the panel draws.
    pub new: tabletist_db::Value,
    /// What the cell loaded as, short, as the grid shows it.
    pub was: String,
    /// What stands against the cell, in the grid's words: what its column
    /// refuses of the text, or what the database said of its row's
    /// statement. None for a cell that is ready to save.
    pub trouble: Option<String>,
}
```

`row_fields` (`src/app.rs:3986`) has no locale, look or column types to word it with, so it does not word it: make `trouble` carry what the cell's state holds (the `Problem` of a cell to fix, or the failed statement's code and message, as `edit::Pending::state` and the last save's note keep them), and let the panel word it with the function `ui/data_view.rs` builds a troubled cell's `Cell::hint` with (lines 1601 to 1633), moved out so both call it. The type in the block above becomes that carrier, not a `String`. `RowFields`' `Debug` prints nothing of it: it can quote what was typed.

`Action::RevertCell` gains `cell: Option<CellPos>` ("None is the selected cell, as its keys ask"); the reducer reverts `cell.or(object.selection)`.

`src/ui/row_panel.rs`:

- `was` goes from under the value to the label line's right, before Copy: "was {loaded}" in `palette.warning`, cut with `grid::ellipsize` to the room the label leaves, then " · " and `revert`, a link in the same colour, underlined, named "Revert {column}" for a screen reader. A click pushes `Action::RevertCell { tab, id, cell: Some(CellPos { row, col }) }`. The dot after the label goes. In the terminal look the label line shows the same words without the underline, and a `~` in `palette.warning` is painted in the panel's side margin at the label's line (the gutter), in place of the one after the label.
- The value's control, behind the value (`behind`), where the field is not under the pointer: pending, the pending cell's fill and a 2 pt bar in `palette.warning` at the frame's left; in trouble, the troubled cell's fill and a 1 pt border in `palette.danger`. Take both fills from where `ui/grid.rs` tints `Mark::Pending` and `Mark::Trouble` (make that function `pub(crate)`), so a cell and its field cannot drift. The terminal look tints nothing: its value is drawn in `palette.warning`, or `palette.danger`.
- Under a troubled field, `trouble` in `palette.danger`, in the caption's role, wrapped to the field's width.
- The header (macOS and Windows): where `texts.pending` holds any, the line under the title reads "{n} unsaved changes" ("1 unsaved change") in `palette.warning`.
- The row lock's line: where `locked` is `Some`, a line under the header's rule, above the first field: the lock icon and the reason (11 pt icon, the caption's role, `palette.dim`; 10 above and below, on `palette.panel`, a rule under it). For `Lock::ReadOnly` the words are "Read-only connection" ("read-only connection" in the terminal look). `editing_footer`'s note under a row lock goes, and the footer's height loses the note's line there.

- [ ] **Step 4: Run the tests, then the four checks.** Expected: PASS.

- [ ] **Step 5: Commit.**

```bash
git add -A && git commit -S -m "Show in a field of the row panel what its cell holds" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 7: Scenes and documents

**Files:**
- Modify: `src/shots.rs` (`row_form_field`, `row_form_locked`, new `row_form_pending`)
- Modify: `docs/superpowers/specs/2026-10-06-row-form-design.md`, `docs/superpowers/specs/2026-10-03-value-editing-core-design.md`, `docs/superpowers/specs/2026-09-27-tabletist-design.md` (section 5.7 and its keyboard table), `README.md`
- Modify: this plan ("What the run found")

- [ ] **Step 1: The scenes.** `row-form-field`: a field being edited with a refused text, a pending field above it, the pointer over a third value. `row-form-locked`: a click on the key's value, its reason under it; and, on a read-only connection, the line at the top. `row-form-pending`: two pending fields and one to fix, the keyboard on a fourth. All on the Bookshop data, in every look.

- [ ] **Step 2: Render them and look.** As the README of `src/shots.rs` says. Read each image beside the artboards "Table view, editable" (the hover, the lock, the footer), "Editing a row" (pending, "was print · revert", "2 unsaved changes") and the Components' grid cell states. Fix what differs, in the task it belongs to. The images stay local: none is committed or pushed.

- [ ] **Step 3: The documents.** `2026-10-06-row-form-design.md`: its status line says it is replaced by `2026-10-06-row-inspector-inline-edit-design.md`, and which of its parts still hold (the one editor and its place, `row_lock`, "Editing in the grid…"). The value editing spec and the main spec: the row panel's lines and the keys tables say what the panel does now. The README's line on the row panel.

- [ ] **Step 4: Run the four checks and the shots lint, then commit.**

```bash
git add -A && git commit -S -m "Add scenes of the row panel's fields, and say in the documents how they edit" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

## By hand, for the user

The app's window does not open where this runs. After the run, in the real window:

- A click on a value puts the caret where expected (at the end), and the I-beam shows over a value that can be edited.
- `Mod+I`, then Down, Enter, type, Enter, Tab: the keyboard never lands on the grid by itself.
- Esc from a field: the grid's cell of that column is the active one.
- On Omarchy: `ctrl+l`, `j`, `i`, type, Esc, `ctrl+h`.
- A screen reader reads a field as "email, varchar, user2@example.com".
