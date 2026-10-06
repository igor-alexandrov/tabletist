# Row Inspector, Step 2: Long and JSON Values Edited in the Panel Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A value of several lines, a long one or a JSON document is edited in the row panel, in its field's place, where its edit is asked for there: no popover at the grid's cell, no "Editing in the grid…". A JSON value reads as JSON while it is edited, in the panel and in the grid's popover alike.

**Architecture:** The tab's one editor already says where it is drawn (`Editor::place`) and whether it is the large one (`Editor::large`). Step 1 sent every large edit to the grid; this step lets it stay in the panel. The large editor's body (the text of several lines, its line, the band under it) is cut out of the popover in `ui/cell_editor.rs` into one function that the popover and the panel's tall field both draw, so the two cannot drift and nothing is built for the inspector alone. What a JSON value gains (a gutter of line numbers, its syntax in colour, "Valid JSON" in the band) is added to that shared body.

**Tech Stack:** Rust 1.98, egui/eframe (the crmne fork). Spec: `docs/superpowers/specs/2026-10-06-row-inspector-inline-edit-design.md`, section 4 ("Editors": text and json/jsonb). Step 1: `docs/superpowers/plans/2026-10-06-row-inspector-inline-edit.md`.

---

## Before you start

- Cargo is `~/.cargo/bin/cargo`. Never point `CARGO_TARGET_DIR` at `/tmp`.
- The four checks of `AGENTS.md` pass after every task, and the shots lint (`~/.cargo/bin/cargo clippy --locked --features shots --lib --tests -- -D warnings`) where `src/shots.rs` is touched.
- **The branch.** `claude/row-inspector-tall-values`, cut from `claude/row-inspector-inline-edit-9297c4` at `170ab7e` (step 1, pull request #91, not merged yet). Its pull request waits for #91.
- Written from reading the tree, as step 1's plan was: no block here was compiled. The tests say what must hold.
- House rules as in step 1's plan: no em dashes; a view pushes `Action`s; text through `TextRole`s; focus drawn by `ui/focus.rs`; what a user typed reaches no log; no design or pixel conformance in a test, and no design material in the repository; scenes on the Bookshop data, compared by eye.
- Commits are signed, one per task.

## What the design gives, and what this builds

The "Editing a row" artboard draws a JSON field being edited in the inspector: a bordered box, a gutter of line numbers on the hover's tone, the document in its colours, the changed line on the pending tint, and under the box "Valid JSON · 1 line changed" in green. "Editors by type" draws long text as a box with the accent's line, the text, and a band under it: "2,341 chars · 2 lines" at its left, "⌘↩ apply · esc cancel" at its right. Its JSON card has "Format ⇧⌘F" at the label's right, a red line round an invalid document, and the parser's message under it.

| | The design | This plan |
|---|---|---|
| Where a tall value is edited from the panel | In the panel | In the panel |
| The box | The accent's line, red while invalid | The same: the popover's body |
| Its height | Not said | Three lines to twelve of the text, scrolling past that |
| The band | Counts at the left, keys at the right | The popover's band, as it is |
| JSON: line numbers | A gutter on the hover's tone | Built, in the shared body: the popover gets it too |
| JSON: colours | Keys, strings, numbers | Built, in the shared body |
| JSON: validity | "Valid JSON" in green, the parser's words in red | In the band: "Valid JSON" and the counts, or what the check says |
| JSON: "1 line changed", the changed line's tint | Drawn | Not built: it needs a line diff against the loaded value |
| "Format ⇧⌘F" | A link at the label's right | Built, by the key in both places and by a link in the panel's label line: a re-indent that reads no value (see "What the run found") |
| The `NULL` checkbox in the label's line | Drawn | Not built: `Mod+Backspace` on the field, as for every field |

## What was decided

1. **The tall field is the popover's body**, not a second editor: one function draws both. What JSON gains, the grid's popover gains.
2. **`Mod+Enter` applies and the keyboard stays on the field** (`Advance::Stay`): Enter and Tab are the text's own in a tall field, so it ends the walk.
3. **Alt+Enter in a one-line field of the panel makes it tall in place.** It moved the edit to the grid's popover.
4. **A tall value whose edit is asked for in the grid still opens the grid's popover**, and the panel says "Editing in the grid…" for it: one editor is open at a time, where it was asked for.
5. **While a document is edited its tree, its attachment card and "Collapse all" are not drawn.** Copy stays: it copies the value as it is pending or loaded, not what is being typed.
6. **JSON's colours are its tree's** (`ui/json_view.rs`), so a document reads the same folded and edited. A text that does not parse is coloured as far as its tokens go.

## File map

| File | What changes |
|---|---|
| `src/app/editing.rs`, `src/app.rs` | A large edit keeps the place it was asked for; `EditorBreak` keeps it too |
| `src/ui/cell_editor.rs` | `large_body`, shared; `tall`, the panel's; the band says "Valid JSON"; the gutter and the colours |
| `src/ui/row_form.rs` | `Form::editing` draws `tall` for a large editor |
| `src/ui/row_panel.rs` | A document's label line while it is edited |
| `src/ui/mod.rs`, `src/app.rs` | Tests |
| `src/shots.rs`, `docs/…` | Scenes, the specs' status |

---

### Task 1: A tall value is edited in the panel

**Files:**
- Modify: `src/app/editing.rs` (`edit_cell`), `src/app.rs` (`Action::EditorBreak`)
- Modify: `src/ui/cell_editor.rs` (`large`, new `large_body` and `tall`)
- Modify: `src/ui/row_form.rs` (`Form::editing`), `src/ui/row_panel.rs` (a document's label line)
- Test: `src/app.rs`, `src/ui/mod.rs`

- [ ] **Step 1: Write the failing tests.**

In `src/app.rs`, `a_tall_value_asked_for_in_the_row_panel_is_edited_at_its_cell` becomes `a_tall_value_asked_for_in_the_row_panel_is_edited_there`: `EditField` on the document's cell `(0, 2)` opens `(at(0, 2), EditorPlace::Panel, true)`; after `CancelEdit` the field is owed the keyboard (`focus_field == Some(2)`); `EditorBreak` on a one-line field of the panel leaves `(at(1, 1), EditorPlace::Panel, true)` with the line break at the end of its text. An `EditCell` on `(0, 2)` still gives `EditorPlace::Grid`.

In `src/ui/mod.rs`, `a_tall_values_pencil_opens_the_popover_at_its_cell` becomes `a_tall_value_is_edited_in_the_row_panel`, in every look:

```rust
            let (mut harness, tab, id) = editable_in(look);
            select(&mut harness, tab, id, (0, 0));
            // A document: its pencil stands with its other controls.
            harness.click("Edit meta");
            let editor = edits(&harness, tab, id).editor.as_ref().expect("an editor");
            assert!(editor.large, "{}", look.name);
            assert_eq!(form_editor(&harness, tab, id), Some(((0, 2), true)));
            assert!(harness.ctx.text_edit_focused(), "{}", look.name);
            // In the panel, not at the cell: nothing says the grid has it.
            assert!(!painted(&harness, &look.label("Editing in the grid…")));
            let tree = harness.settle();
            let field = crate::testing::bounds(&tree, "Edit meta", egui::accesskit::Role::MultilineTextInput)
                .expect("the field");
            let cell = cell_of(&harness, "user2@example.com");
            assert!(field.left() > cell.x, "{}", look.name);
            // Enter is the text's own; Mod+Enter applies.
            harness.press(Key::Enter, Modifiers::NONE);
            assert!(edits(&harness, tab, id).editor.is_some(), "{}", look.name);
            harness.press(Key::Backspace, Modifiers::NONE);
            // A text that is no JSON is not applied, and the band says why.
            type_text(&mut harness, "x");
            harness.press(Key::Enter, Modifiers::COMMAND);
            assert!(edits(&harness, tab, id).editor.is_some(), "{}", look.name);
            harness.press(Key::Backspace, Modifiers::NONE);
            type_text(&mut harness, " ");
            harness.press(Key::Enter, Modifiers::COMMAND);
            assert!(edits(&harness, tab, id).editor.is_none(), "{}", look.name);
            assert!(pending_text(&harness, tab, id, (0, 2)).is_some(), "{}", look.name);
            // The keyboard is on the field it was on.
            harness.settle();
            assert!(field_focused(&harness, tab, id, 2), "{}", look.name);
```

(The role of a field of several lines is whatever the grid's popover's test finds it by: `a_long_value_is_edited_whole` shows it.) And `alt_enter_makes_a_field_of_the_panel_tall_in_its_place`: click a one-line value, press Alt+Enter, the editor is `(.., true)` in the panel and large, and its text ends in a line break.

- [ ] **Step 2: Run them to see them fail.** Expected: the editor's place is the grid's.

- [ ] **Step 3: Write the code.**

`src/app/editing.rs`, `edit_cell`: the editor is opened with `place`, not `if large { EditorPlace::Grid } else { place }`, and the comment over it says a tall value is edited where it was asked for. `src/app.rs`, `Action::EditorBreak`: the line `editor.place = EditorPlace::Grid;` goes, with the sentences of its comment that explain it.

`src/ui/cell_editor.rs`: cut the body out of `large`.

```rust
/// The large editor's body, in `rect`: a text of several lines, the
/// editor's line round it, and the band under it. The popover at a cell
/// and the row panel's tall field both draw it. `had` says its text had
/// the keyboard when it was last drawn; returns the text's response and
/// whether it has the keyboard now.
fn large_body(
    ui: &mut Ui,
    rect: Rect,
    salt: Id,
    editor: &mut Editor,
    target: &Target,
    had: bool,
    (look, palette, locale): (&Look, &Palette, Locale),
) -> (egui::Response, bool)
```

It holds what `large` does today from `let whole = ui.interact(rect, area.with("panel"), ..)` to `band(..)`, with `salt` for `area`. `large` keeps the area, the sizing pass, the keys read before the text is added, `sql_complete::panel` (the popover's ground and shadow), and `keyboard_after`.

```rust
/// The large editor in the row panel: the popover's body in the place of
/// its field's value, `outset` wider at each side than the room it is
/// given, as the one-line field is, and as tall as its text from three
/// lines to twelve. Its keys are the popover's: Enter and Tab are the
/// text's own, Mod+Enter applies, Esc drops the edit (the terminal look's
/// keeps it, and Ctrl+C drops it).
pub fn tall(
    ui: &mut Ui,
    editor: &mut Editor,
    target: &Target,
    outset: f32,
    (look, palette, locale): (&Look, &Palette, Locale),
) -> Outcome
```

It takes the keyboard when the editor opened (`editor.focus`), reads `Mod+Enter` (`Advance::Stay`) and `leaving_keys` before the text is added, lays the text out at the field's width to count its rows (`crate::typography::layouter`, clamped to 3..=12), allocates `LARGE_PAD + rows * line + LARGE_PAD / 2 + BAND` of height in the field's line, paints the panel's tone behind `rect` (`palette.window`; the terminal look keeps a field's fill), draws `large_body` in a child `Ui` over the line widened by `outset`, brings it into view once when it opens, and ends with `keyboard_after`.

`src/ui/row_form.rs`, `Form::editing`: `if editor.large { cell_editor::tall(ui, editor, target, place.1, skin) } else { cell_editor::in_panel(..) }`.

`src/ui/row_panel.rs`: `json_view::fold_all_link` is drawn only where `part != Part::Editing`.

- [ ] **Step 4: Run the tests, then the four checks.**

- [ ] **Step 5: Look at it.** A throwaway scene of a document and of a long text being edited in the panel, in each look: beside "Editors by type", card 2.

- [ ] **Step 6: Commit.** "Edit a long value and a document in the row panel, in its field's place".

---

### Task 2: A JSON value reads as JSON while it is edited

In the shared body, so in the popover and in the panel: a gutter of line numbers, the document's syntax in its tree's colours, and "Valid JSON" in the band.

**Files:**
- Modify: `src/ui/cell_editor.rs` (`large_body`, `band`, `Target`), `src/ui/data_view.rs` (`editor_target`)
- Modify: `src/ui/json_view.rs` (its colours, lent)
- Test: `src/ui/cell_editor.rs`, `src/ui/mod.rs`

- [ ] **Step 1: Write the failing tests.**

In `src/ui/cell_editor.rs`: `json_is_coloured_by_its_tokens`: the sections of `json_job(r#"{"a": "b", "n": 1, "t": true}"#, ..)` are a key in the key's colour, a string in the string's, a number and a literal in theirs, and the punctuation in the text's; a text that ends inside a string is coloured to its end and does not panic; the job's text is the input, byte for byte.

In `src/ui/mod.rs`: `a_document_being_edited_shows_its_line_numbers_and_says_it_is_valid`: with a document of three lines open in the panel, "1", "2" and "3" are painted left of the field's text in `palette.faint`; the band paints "Valid JSON" in `palette.success`; after a character that breaks it, "Valid JSON" is gone and the check's words are there in `palette.danger`. The same holds in the grid's popover (`open_editor` on the cell). A text column's tall editor paints no line numbers and never says "Valid JSON".

- [ ] **Step 2: Run them to see them fail.**

- [ ] **Step 3: Write the code.**

`Target` gains `json: bool` ("the column holds JSON: its editor numbers its lines and colours its syntax"), set in `data_view::editor_target` from the column's class.

`json_job(text, format: impl Fn(Color32) -> TextFormat, colors) -> LayoutJob`: a scanner over the text's bytes that never fails. A string runs to its closing quote (a backslash takes the byte after it) or to the text's end; it is a key when the next byte that is not whitespace is a colon. A number is a run of `-+.eE` and digits. `true`, `false` and `null` are literals. Everything else is the text's colour. The colours are the ones `json_view` draws a tree in, from one function both call.

In `large_body`, for a `target.json` editor: the text's layouter is `json_job`'s, and the text is added with `.show(ui)` so its galley is known. A gutter stands at the text's left, inside the scroll area so it moves with the text: as wide as the digits of the last line's number and 6 at each side, on the tone of a row under the pointer (`grid::row_fill`), with a number for each line of the text (not each wrapped row: a row that does not follow a line break has none), right-aligned, in `palette.faint`, on its row's line. `ui/sql_text.rs` shows how the rows of a galley are placed.

`band`: for a `target.json` editor with no problem, the left side reads "Valid JSON · {chars} · {lines}", "Valid JSON" in `palette.success`, the counts in `palette.dim`.

- [ ] **Step 4: Run the tests, then the four checks.**

- [ ] **Step 5: Look at it.** The scene of task 1, beside the JSON field of "Editing a row" and card 8 of "Editors by type".

- [ ] **Step 6: Commit.** "Number the lines of a document being edited, colour it, and say it is valid".

**The place to stop if the run grows long** is after task 1: the panel then edits every value it shows. Task 2 is how a document looks while it is edited.

---

### Task 3: Scenes and documents

**Files:**
- Modify: `src/shots.rs` (`row-form-tall`, `row-form-json`)
- Modify: `docs/superpowers/specs/2026-10-06-row-inspector-inline-edit-design.md` (status), `docs/superpowers/specs/2026-10-06-row-form-design.md` (its step 2 is built here), `docs/superpowers/specs/2026-09-27-tabletist-design.md` (section 5.7), `docs/superpowers/plans/2026-10-06-row-inspector-inline-edit.md` ("What this run leaves"), this plan ("What the run found")

- [ ] **Step 1:** The scenes: a long text of several lines being edited in the panel; a document being edited, valid; the same with a character that breaks it. In every look.
- [ ] **Step 2:** Render and read them beside the artboards. The images stay local.
- [ ] **Step 3:** The documents.
- [ ] **Step 4:** The four checks and the shots lint, then commit: "Add scenes of the row panel's tall field, and say in the documents that it is built".

## What the run found

Run on 2026-10-06, inline, one signed commit per task, the four checks and the shots lint passing after each (1,798 tests in the main crate at the end).

- **Format is built.** The plan left it out because `serde_json` here sorts an object's keys. Colouring needed a scanner of the text anyway (`ui/json_text.rs`), and a layout made from its pieces reads no value: it changes only the white space between them, so no key, string or number can come out other than it went in (a number no float holds, a key that stands twice, an escape). `Action::FormatEditor`, `Mod+Shift+F` wherever the large editor is open, and "Format ⇧⌘F" in the label line of a document being edited in the panel. It does nothing to a text its column does not take.
- **A press on the Format link is the editor's own** (`cell_editor::press_is_the_editors`): the text would otherwise give up the keyboard to the click, and the editor close before it was laid out.
- **The large editor's keys are read in one function** (`large_keys`) for the popover and the tall field, beside the shared body.
- **Tests of the popover that read its band** read "Valid JSON · " before a document's counts, and its text is narrower by the gutter.
- Not built, as the plan's table says: "1 line changed" and the changed line's tint, and the `NULL` checkbox.

After the user's first look at it ("formatting is missing", and the caret of a tall field going to its end):

- **A document opens laid out**, a member to a line, in the panel and in the grid's popover: `edit::start_text` lays a JSON column's text out where it is a document (a server gives it back on one line). What such a column holds that is none (SQLite keeps any text) opens as it is.
- **White space alone is no change of a document.** `edit::is_change` compares a JSON column's text laid out, so a document that was only opened, or set back on one line, or laid out again, leaves the cell as it loaded. White space inside a string is the string's and counts.
- **A tall value opens with its cursor at its start** (`Editor::top`), in both places: its end may be far below what the editor shows, and the editor scrolled there. A line break typed into a one-line field goes on after it, as before. The one-line field still opens with its cursor at the end.
- Tests of the large editor that typed at the end of a document type there by `Ctrl+End` first, and the ones that made a change with a space add a member instead.

## By hand, for the user

- A click in the tall field puts the caret where it was clicked; the wheel scrolls the text when it is longer than twelve lines, and the panel when the pointer is elsewhere.
- `Mod+Enter` applies; Esc drops; a click on another field keeps what was typed.
- On Omarchy: `i` on a document's field, type, `ctrl+enter`.
