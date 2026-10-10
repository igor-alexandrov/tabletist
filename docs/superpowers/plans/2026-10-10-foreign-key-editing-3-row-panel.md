# Foreign Key Editing, Run 3: The Picker in the Row Panel Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The row panel's field of a foreign key column searches and picks with the same list as the grid's cell, on an existing row and in the "New row" form, and the screenshots' scenes show the picker.

**Architecture:** Nothing new is built. The picker is state of the editor, and the panel's editor is the same `edit::Editor` with `EditorPlace::Panel`: `open_picker` stops refusing it. `cell_editor::in_panel` learns what `cell_editor::field` learned in run 2 (the name, Esc, select all) and says where its field stands, and `RowForm::editing` draws `ui::key_picker::show` there and queues what it says, as `data_view` does for a cell. The list's keys need no change: the panel's field has the same egui id as the cell's.

**Tech Stack:** Rust 1.98, egui (the crmne fork), AccessKit for the headless tests. Spec: `docs/superpowers/specs/2026-10-10-foreign-key-editing-design.md` (section 8, test 22). Run 2 (`2026-10-10-foreign-key-editing-2-grid-picker.md`) must be merged first.

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

- `src/shots.rs` is outside those four. Task 2 changes it, and checks it with `~/.cargo/bin/cargo clippy --locked --features shots --lib --tests -- -D warnings`.
- **What was run where this plan was written.** Nothing. The code blocks were written from reading the tree at `6e04567`, before runs 1 and 2, and from those runs' plans. They were not compiled. Where a block and the compiler disagree, the compiler is right: keep the block's behaviour and its tests, and fix its spelling. Where run 2 was built otherwise than its plan says, follow what was built.
- House rules: views push `Action`s and change nothing but the field's text; text through `TextRole`s; strings through `gettext`; no em dashes; `app.rs` and `ui/mod.rs` get no new code.
- Commits are signed, one per task, after its checks pass. If signing fails, do not commit unsigned: stage the task and tell the user. Each message ends with:

      Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>

## What the spec asks, and what gets built

| | The spec | What gets built | Why |
|---|---|---|---|
| a | The panel's field is "the same field and the same list" | The same list. The field is the panel's own (it wraps, and stands in the value's place), with the picker's name, keys and select all. It has no magnifier and no `>` prompt | The panel's field has no room at its start: its text stands where the value was read. The label over it already says `publisher_id · int8 → publishers` |
| b | "at the field's width" | The list is as wide as the field, between 280 and 420 points, as the grid's | `key_picker::show` sizes it from its anchor |
| c | The terminal look counts the options in the field | Not in the panel's field | It has no place for a count: the grid's field puts it where a text column's `27 / 200` goes, which the panel says under the field |
| d | "Open publisher →" hides while the field is edited | Already so: an edited field draws its editor in the value's place, and the link is the value's | `src/ui/row_panel.rs`, `Part::Editing` |
| e | "Editing in the grid…" while the cell is edited in the grid | Already so (`row_form::in_grid`) | |
| f | A screenshot of the picker for each look | Two scenes of `src/shots.rs`, which write PNGs for the eye. The site's pictures in `assets/screenshots/` are made from them by hand, and none is added here | Say so in the pull request. The user decides whether the guide gets a picture |

## File map

| File | What changes |
|---|---|
| `src/app/key_picker.rs` | `open_picker` takes an editor in the panel (task 1) |
| `src/ui/cell_editor.rs` | `Outcome::rect`; `in_panel`: the name, Esc, select all (task 1) |
| `src/ui/row_form.rs` | `RowForm::editing` draws the list (task 1) |
| `src/ui/key_picker_tests.rs` | Test 22 and the panel's own (task 1) |
| `src/shots.rs` | The books' answers, two scenes (task 2) |
| `docs/_guide/editing-data.md`, the spec | Task 3 |

---

### Task 1: The panel's field picks

**Files:**
- Modify: `src/ui/key_picker_tests.rs`
- Modify: `src/app/key_picker.rs` (`open_picker`)
- Modify: `src/ui/cell_editor.rs` (`Outcome`, `in_panel`)
- Modify: `src/ui/row_form.rs` (`RowForm::editing`, near line 199)

- [ ] **Step 1: Write the failing tests**

Append to `src/ui/key_picker_tests.rs`:

```rust
/// Opens the row panel's editor on `publisher_id` of the page's row `row`
/// from `start`, the panel showing, and answers what the picker asks.
fn edit_field(harness: &mut Harness, tab: ConnTabId, id: TabId, row: usize, start: EditStart) {
    let cell = CellPos {
        row,
        col: PUBLISHER,
    };
    harness.app.apply(Action::SelectCell { tab, id, cell });
    harness.app.apply(Action::FocusFields { tab, id });
    harness.app.apply(Action::EditField {
        tab,
        id,
        cell,
        start,
    });
    harness.answer_structure(publishers_structure());
    harness.answer_lookups(&PUBLISHERS);
}

/// Test 22: the row panel's field opens the same list, and a pick there
/// marks the grid's cell pending.
#[test]
fn the_row_panels_field_opens_the_same_list() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    edit_field(&mut harness, tab, id, 0, EditStart::Replace(String::new()));
    type_text(&mut harness, tab, id, "harb");
    let tree = harness.settle();
    // The panel's field wraps: a text field of several lines to egui.
    assert!(
        crate::testing::node(
            &tree,
            "Search publishers for publisher_id",
            Role::MultilineTextInput
        )
        .is_some(),
        "{:?}",
        crate::testing::labels(&tree)
    );
    assert!(crate::testing::node(&tree, "Matching publishers", Role::ListBox).is_some());
    assert_eq!(listed(&harness, tab, id), [HARBOR_PRESS, HARBOR_LIGHTS]);
    // Enter in the panel takes the option and walks on to the next field.
    commit(&mut harness, tab, id, Advance::NextField);
    assert!(!editing(&harness, tab, id));
    assert_eq!(pending(&harness, tab, id, 0).as_deref(), Some(HARBOR_PRESS));
    let tree = harness.settle();
    assert!(crate::testing::node(&tree, "Matching publishers", Role::ListBox).is_none());
}

/// A click on an option of the panel's list takes it, and the text typed
/// is not first kept.
#[test]
fn a_click_takes_an_option_of_the_panels_list() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    edit_field(&mut harness, tab, id, 0, EditStart::Replace(String::new()));
    type_text(&mut harness, tab, id, "harb");
    harness.settle();
    harness.click("Harbor Lights, 9100000000000000007");
    assert!(!editing(&harness, tab, id));
    assert_eq!(pending(&harness, tab, id, 0).as_deref(), Some(HARBOR_LIGHTS));
}

/// Esc in the panel's picker leaves the field as it was, in both looks:
/// the terminal look's Esc keeps a text field's edit, and not a search.
#[test]
fn esc_closes_the_panels_picker_in_both_looks() {
    for look in [Look::macos(), Look::omarchy()] {
        let (mut harness, tab, id) = covers_in(look);
        edit_field(&mut harness, tab, id, 0, EditStart::Replace(String::new()));
        type_text(&mut harness, tab, id, "harb");
        harness.settle();
        harness.press(Key::Escape, Modifiers::NONE);
        assert!(!editing(&harness, tab, id), "{}", look.name);
        assert_eq!(pending(&harness, tab, id, 0), None, "{}", look.name);
    }
}

/// Spec 8: the "New row" form's field picks as any field does, and the
/// pick fills what the row needed.
#[test]
fn the_new_row_forms_field_picks() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    harness.app.apply(Action::AddRow {
        tab,
        id,
        place: crate::edit::Place::Top,
    });
    // Adding the row opened the grid's editor on its first required
    // cell: the form's field is edited instead.
    harness.app.apply(Action::CancelEdit { tab, id });
    let new = crate::edit::new_row(0);
    edit_field(&mut harness, tab, id, new, EditStart::Replace(String::new()));
    type_text(&mut harness, tab, id, "harb");
    commit(&mut harness, tab, id, Advance::NextField);
    assert_eq!(pending(&harness, tab, id, new).as_deref(), Some(HARBOR_PRESS));
    assert!(harness.app.lacking(tab, id).is_empty());
}

/// Spec 2: opened on its value by a key, the panel's field has its text
/// selected on macOS, and not in the terminal look.
#[test]
fn the_panels_field_selects_its_text_where_the_grids_does() {
    for (look, selects) in [(Look::macos(), true), (Look::omarchy(), false)] {
        let (mut harness, tab, id) = covers_in(look);
        edit_field(&mut harness, tab, id, 1, EditStart::Value);
        let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
        let editor = object.edits.editor.as_ref().unwrap();
        assert!(editor.picker.is_some());
        assert_eq!(editor.select, selects, "{}", look.name);
    }
}
```

`Look::omarchy()` is the terminal look, and `look.name` is the look's name as `src/shots.rs` reads it (`look.name`).

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked --lib key_picker_tests`
Expected: the five new tests FAIL. The first four: the editor has no picker (`listed` panics on `unwrap`), since `open_picker` refuses an editor in the panel. The fifth: `editor.picker.is_some()`.

- [ ] **Step 3: The panel's editor gets its picker**

In `src/app/key_picker.rs`, `open_picker` has:

```rust
            // The row panel's field gets its picker in run 3.
            if editor.large || editor.place != EditorPlace::Grid {
                return None;
            }
```

Replace it with:

```rust
            // The picker belongs to the small field, on the cell or in
            // the row panel: the large editor has no list.
            if editor.large {
                return None;
            }
```

and take `EditorPlace` out of the file's imports if nothing else there uses it.

Run: `~/.cargo/bin/cargo test --locked --lib key_picker_tests::the_panels_field_selects`
Expected: PASS, and `the_new_row_forms_field_picks` with it: neither needs a view. The other three still fail: no list is drawn in the panel, and the terminal's Esc keeps the text.

- [ ] **Step 4: The panel's field says where it is, and what it searches**

First, the panel's `Target` must know of the search at all. `row_panel::show` takes the editor out of the tab before it draws (`row_form::take_editor`, `src/ui/row_form.rs` near line 65), and `Form::of` then builds the target with `data_view::editor_target(workspace, object, ...)`, which run 2 has read `search` from `object.edits.editor`: empty at that moment. So in the panel `target.search` would always be `None`: no name, no keys, no list.

In `src/ui/data_view.rs`, move what `editor_target` works out for `search` into a function that is given the editor:

```rust
/// What a foreign key cell's field says of its search: from the picker
/// of `editor`, which is the tab's own or, while the row panel draws, the
/// one the panel took out of the tab.
pub(super) fn search_of(
    workspace: &crate::model::Workspace,
    object: &ObjectTab,
    editor: &crate::edit::Editor,
) -> Option<cell_editor::Search> {
    let picker = editor.picker.as_ref().filter(|_| !editor.large)?;
    let table = Table::of(workspace, object)?;
    Some(cell_editor::Search {
        table: picker.target.name.clone(),
        count: picker.listed.as_ref().map(|_| {
            if picker.more {
                format!("{}+", crate::key_picker::LIMIT)
            } else {
                picker.options.len().to_string()
            }
        }),
        nullable: table
            .column(editor.cell.col)
            .is_some_and(|column| column.nullable),
        refused: picker.refused,
    })
}
```

`editor_target`'s `search:` becomes:

```rust
        search: object
            .edits
            .editor
            .as_ref()
            .filter(|editor| editor.cell == cell)
            .and_then(|editor| search_of(workspace, object, editor)),
```

and in `src/ui/row_form.rs`, `Form::of` builds its target (near line 155) with the editor it holds:

```rust
        let target = editor.as_ref().and_then(|editor| {
            let mut target =
                crate::ui::data_view::editor_target(workspace, object, tab, editor.cell, hold)?;
            // The panel's editor is out of the tab while the panel draws.
            target.search = crate::ui::data_view::search_of(workspace, object, editor);
            Some(target)
        });
```

Then, in `src/ui/cell_editor.rs`:

1. `pub struct Outcome` gets a field:

```rust
    /// Where the field stands, for what is anchored to it: the row
    /// panel's field, whose place the panel's layout decides. `None` from
    /// a cell's field, whose place the grid gives.
    pub rect: Option<Rect>,
```

2. In `in_panel`, the keys. Its `ending_keys(ui, has, had, look.terminal, false, &mut outcome);` (run 2 gave it `false`) becomes:

```rust
    ending_keys(ui, has, had, look.terminal, target.search.is_some(), &mut outcome);
```

3. In `in_panel`, select all. The function opens with:

```rust
    let opened = std::mem::take(&mut editor.focus);
    if opened {
        take_keyboard(ui.ctx(), id, &editor.text);
    }
```

   and, a few lines down, `let at = opened.then(|| asked_at(ui.ctx(), id)).flatten();`. Take the flag with the focus, and select once it is known that no click placed the cursor:

```rust
    let opened = std::mem::take(&mut editor.focus);
    let select = std::mem::take(&mut editor.select);
    if opened {
        take_keyboard(ui.ctx(), id, &editor.text);
    }
```

```rust
    let at = opened.then(|| asked_at(ui.ctx(), id)).flatten();
    // A foreign key's field opened by a key: its text is selected, so
    // what is typed next starts a search. A click put the cursor where it
    // was read, and selects nothing.
    if opened && select && at.is_none() {
        take_keyboard_selected(ui.ctx(), id, &editor.text);
    }
```

4. In `in_panel`, the name. Replace the line `let name = format!("{} {}", gettext(locale, "Edit"), display_safe(&target.name));` with the `match &target.search` that `field` has (run 2, task 4, step 3). To write it once, move that `match` into a function beside `Target` and call it from both:

```rust
impl Target {
    /// What the field is called: "Edit title", or for a foreign key's
    /// field what it searches, "Search publishers for publisher_id".
    fn field_name(&self, locale: Locale) -> String {
        match &self.search {
            // A placeholder each: a translation puts the names where they
            // read.
            Some(search) => gettext(locale, "Search {table} for {column}")
                .replace("{table}", &display_safe(&search.table))
                .replace("{column}", &display_safe(&self.name)),
            None => format!("{} {}", gettext(locale, "Edit"), display_safe(&self.name)),
        }
    }
}
```

   `field` and `in_panel` both become `let name = target.field_name(locale);`.

5. In `in_panel`, what the column says of the text. Run 2 made the cell's field wait with its red ring and its note until a commit was refused (`let said = target.search.as_ref().is_none_or(|search| search.refused);`). `in_panel` has the same two things: the ring (`let ring = if editor.problem.is_some() { ... }`) and the problem's words under the field. Work `said` out the same way and make both read `editor.problem.is_some() && said`. The words are under the field, where the list is: while `target.search.is_some()` leave them out of the panel's own layout and say them with `say_under(ui, place, id, &message, look, palette, true)`, above the field, as the cell's field does.

6. In `in_panel`, the place. `place` is the field's rect (`let place = line.expand2(vec2(outset, 0.0));`). Before the function returns `outcome`, add:

```rust
    outcome.rect = Some(place);
```

`Outcome` derives `Default` and `PartialEq`, which `Option<Rect>` has. Tests that build an `Outcome` with every field named (`grep -n "Outcome {" src`) get `rect: None`.

- [ ] **Step 5: The form draws the list**

In `src/ui/row_form.rs`, `Form::editing` (near line 199): `let outcome = if editor.large { ... } else { ... };` becomes `let mut outcome = ...;`, and after it, before `if outcome.changed`, add:

```rust
        // A foreign key field's list, under the field. A press on it is
        // the editor's own: the field keeps the keyboard, and the frame
        // does not read as an edit that was left.
        if !editor.large
            && let Some(picker) = &editor.picker
            && let (Some(anchor), Some(search)) = (outcome.rect, &target.search)
        {
            let names = crate::ui::key_picker::Names {
                table: &picker.target.name,
                nullable: search.nullable,
            };
            let shown = crate::ui::key_picker::show(
                ui,
                picker,
                &editor.text,
                anchor,
                target.id,
                &names,
                skin,
            );
            if shown.pressed {
                ui.memory_mut(|memory| memory.request_focus(target.id));
                outcome.left = false;
                outcome.kept = false;
            }
            if let Some(index) = shown.picked {
                self.ending.push(Action::PickOption { tab, id, index });
            } else if shown.null {
                self.ending.push(Action::PickerNull { tab, id });
            }
        }
```

`editor`, `target` and `self.ending` are three fields of `self`, borrowed side by side as the function already borrows them. `skin` is the `(&Look, &Palette, Locale)` the function takes, which is `Copy`.

`crate::ui::key_picker` is a private module of `src/ui/mod.rs`, and `row_form` is its sibling: the path resolves.

- [ ] **Step 6: Run the tests**

Run: `~/.cargo/bin/cargo test --locked --lib key_picker`
Expected: PASS, all of them.

If `the_row_panels_field_opens_the_same_list` finds no list box: the panel is not showing. `Action::FocusFields` shows it; check that the test's `covers_in` did not leave the workspace on another pane, and print `crate::testing::labels(&tree)` to see whether the field is there.

Then the whole suite. The row panel's own tests (`src/ui/row_panel.rs`, `src/ui/mod.rs`) edit `users` and `book_covers` without a foreign key: none of them changes.

- [ ] **Step 7: Look at it**

Run the app, open the row panel on a row of a table with a foreign key (Cmd/Ctrl+I), and edit the key's field. Check by eye: the list stands under the field and inside the window, also when the field is near the window's bottom (it goes above), and "Open publisher →" is not drawn while the field is edited. The same in the Omarchy look. Say in the report what you looked at.

- [ ] **Step 8: Run the four checks, then commit**

```bash
git add src/app/key_picker.rs src/ui/cell_editor.rs src/ui/row_form.rs src/ui/key_picker_tests.rs && git commit -m "Pick a foreign key's row from the row panel's field

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: The screenshots' scenes

`src/shots.rs` renders the app's states to PNG for the eye (`cargo test --features shots -- --ignored`). Its Bookshop is `book_images`, whose `book_id` points at `books`: since run 2 an editor on that column has a picker. Three scenes open one and get no answer, so their pictures show a list that searches for ever: `edit_field` in the grid, `row_form_field` in the panel, and `edit_new_row`, whose new row opens its editor on the first cell a save needs, which is `book_id`. This task answers them, and adds a scene of the picker for the grid and one for the panel.

**Files:**
- Modify: `src/shots.rs`

- [ ] **Step 1: The books**

In `src/shots.rs`, after `structure()` (near line 225), add:

```rust
/// Some of the Bookshop's books, as (id, title): what the picker of
/// `book_images.book_id` finds. The ids are ones [`page_of`] points at.
const BOOKS: [(i64, &str); 5] = [
    (1_048_576, "The Lighthouse Keeper's Daughter"),
    (1_056_495, "Small Rooms"),
    (1_064_414, "Small Rooms, Second Edition"),
    (1_072_333, "Lighthouses of the North Coast"),
    (1_080_252, "A Year of Tides"),
];

/// What `books` describes as, as far as a picker reads it: its key and
/// the column a book is read by.
fn books_structure() -> Structure {
    let column = |name: &str, type_name: &str| ColumnInfo {
        name: name.into(),
        type_name: type_name.into(),
        ..ColumnInfo::default()
    };
    Structure {
        // As a structure names PostgreSQL's types: `column_class` reads
        // `character varying` as text, and not the page's `varchar`.
        columns: vec![column("id", "bigint"), column("title", "character varying")],
        primary_key: vec!["id".into()],
        ..Structure::default()
    }
}

/// Answers what a picker of `book_id` asked: the books' columns, then
/// its lookups.
fn answer_books(harness: &mut Harness) {
    harness.answer_structure(books_structure());
    harness.answer_lookups(&BOOKS);
}
```

`page_of` gives row `i` the book `1_048_576 + i * 7_919`: the five ids above are rows 0 to 4. `ColumnInfo` and `Structure` are in the file's `tabletist_db` imports already (`structure()` builds both).

- [ ] **Step 2: Answer the three scenes that already open a picker**

`edit_field` and `row_form_field` show a text its column does not take, `107233x` in `book_id`, and why. A foreign key's field says that only once a commit took the text and the column refused it (run 2), so the scenes press Enter. Each ends with `harness.app.apply(Action::EditorTyped { tab, id });`. Add after it, in both:

```rust
    // `book_id` is a foreign key: Enter takes the text once the list has
    // answered that no book matches, and the column refuses it.
    let then = Advance::Stay;
    harness.app.apply(Action::CommitEdit { tab, id, then });
    answer_books(harness);
```

with `Advance` added to the file's `use crate::model::{...}` line. The commit waits for the list (the books' columns are not known yet), `answer_books` answers, and the kept Enter is applied: the editor stays open with its problem, the field is red and says why above it, and the list under it says that no books match.

`edit_new_row` adds a row, and the editor opens on `book_id` with nothing typed. Add `answer_books(harness);` as its last line: its picture then shows the books' first rows under the new row's cell.

- [ ] **Step 3: Two scenes of the picker**

Add to the `EDITING` array `("edit-key-picker", edit_key_picker),` and to `ROW_FORM` `("row-form-key-picker", row_form_key_picker),`, and make the arrays' lengths 14 and 10. Then, beside `edit_field`:

```rust
/// The picker of a foreign key cell: `light` typed on `book_id`, the two
/// books it finds under the cell, the first highlighted.
fn edit_key_picker(harness: &mut Harness) {
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
}
```

and beside `row_form_field`:

```rust
/// The same picker in the row panel: the key's field searched for
/// `rooms`.
fn row_form_key_picker(harness: &mut Harness) {
    let (tab, id) = editable(harness);
    harness.app.apply(Action::EditField {
        tab,
        id,
        cell: CellPos {
            row: 4,
            col: BOOK_ID,
        },
        start: EditStart::Replace("rooms".into()),
    });
    answer_books(harness);
}
```

The row panel shows by default, as in `row_form_field`.

- [ ] **Step 4: Check the build, and render if you can**

```bash
~/.cargo/bin/cargo clippy --locked --features shots --lib --tests -- -D warnings
```

Expected: no warnings.

```bash
~/.cargo/bin/cargo test --locked --features shots --lib shots::shots -- --ignored --exact
```

This renders every scene with wgpu, and needs a GPU or a software adapter. Where it runs, look at `edit-key-picker-*` and `row-form-key-picker-*` in the directory `out()` writes to (the function at the top of `src/shots.rs` names it), in both looks, light and dark, and compare with the boards `macOS – Editing a row` and `Omarchy – Editing values, editors by type`. Where it cannot run, say so in the report: the scenes were then only compiled.

- [ ] **Step 5: Run the four checks, then commit**

```bash
git add src/shots.rs && git commit -m "Show the foreign key picker in the screenshots' scenes

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: The website and the spec

**Files:**
- Modify: `docs/_guide/editing-data.md`
- Modify: `docs/superpowers/specs/2026-10-10-foreign-key-editing-design.md`

- [ ] **Step 1: The guide**

In `docs/_guide/editing-data.md`, the part "Foreign keys" that run 2 added ends with:

```markdown
The picker is on the cells of the grid, in a new row too. A key of several
columns is edited as text.
```

Replace it with:

```markdown
The picker is on the cells of the grid and on the fields of the row panel,
in a new row too. A key of several columns is edited as text.
```

- [ ] **Step 2: Check the site**

```bash
cd docs && bundle install && bin/check
```

Expected: passes. If Ruby or the bundle is not there, say so in the report.

- [ ] **Step 3: The spec's status**

In `docs/superpowers/specs/2026-10-10-foreign-key-editing-design.md`, add to the status paragraph, after run 2's sentences:

```markdown
Run 3 (`docs/superpowers/plans/2026-10-10-foreign-key-editing-3-row-panel.md`)
is built: the row panel's field has the same list. Its field has no
magnifier, prompt or count, and no picture of the picker is on the website
yet.
```

- [ ] **Step 4: Commit**

```bash
git add docs/_guide/editing-data.md docs/superpowers/specs/2026-10-10-foreign-key-editing-design.md && git commit -m "Say that the row panel picks a foreign key's row

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

## What this leaves for the runs

- **Run 4** (`2026-10-10-foreign-key-editing-4-missing-row.md`): the warning for a key that names no row, in the grid and under the panel's field.
- **A picture on the website.** `assets/screenshots/` holds the site's pictures as lossless WebP, made by hand from the scenes. If the user wants one of the picker in `docs/_guide/editing-data.md`, it is `{% include shot.html file="macos-edit-key-picker" alt="..." %}` after the "Foreign keys" part, once the file is there.
