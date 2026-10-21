# Settings › Editor, step 4: Close brackets and quotes, and Wrap Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the two Typing settings whose keys step 1 added: Close brackets and quotes (a bracket or a quote typed gets its closing one, a closing one steps over its twin, Backspace takes an empty pair out, a selection gets wrapped) and Wrap (long lines wrap at the editor's width, numbered once). Show both rows in both looks.

**Architecture:** Both are in the editor's text field (`src/ui/sql_text.rs`). Close brackets and quotes rewrites the frame's input before the field reads it, as Tab does since step 3: a character typed becomes the pair and a caret move back, a closing one before its twin becomes a caret move over it, Backspace between an empty pair gets a Delete after it, so the field's own typing and undo do the rest; a selection wrapped is one outside edit. Whether a quote opens, closes or is text is the dialect's tokenizer's to say (`in_literal`, `closes_literal`). Wrap gives the layouter the field's width, lets the scroll area scroll down alone, and maps each line to its rows (`line_rows`) for the gutter's numbers, the statement's bar and the cursor's line; a press on the gutter goes to the start of the line under it, and the completion list reads a word's place at the start of its row.

**Tech Stack:** Rust, egui (crmne fork) with AccessKit, headless tests through `src/testing.rs`.

**Spec:** `docs/superpowers/specs/2026-10-19-settings-editor-design.md`, sections "Steps" item 4 and "Step 4: Close brackets and quotes, and Wrap".

---

## Before Task 1

Commit the spec's step 4 section and this plan:

```bash
git add docs/superpowers/specs/2026-10-19-settings-editor-design.md docs/superpowers/plans/2026-10-21-settings-editor-4-typing.md
git commit -m "Plan Settings › Editor step 4: close brackets and quotes, and wrap"
```

## Where this sits

Steps 1 to 3 are merged (#181, #183, #204); the query history (#197) built step 6. This plan is a branch of `main` at `18d3d761`.

## Departures, for a yes or a no before code

| Asked, or the boards | Built | Why |
|---|---|---|
| macOS board: "Line numbers · current line · wrap", one row of three switches | Close brackets and quotes, Line numbers, Current line highlight and Wrap a row each, in the board's order | Step 1's rule: a control is named by its own setting's label |
| "Close brackets and quotes" (no more said) | `(`, `[`, `{`, `'`, `"`, and `` ` `` on MySQL and SQLite (where it quotes names); paired only where nothing follows that the pair would split off (the end, whitespace, a closing bracket, `,`, `;`); brackets inside strings and comments too; a quote not inside a word (so not a prefixed `E'`), after the same quote, or inside a string, a quoted name or a comment | What an SQL editor pairs, and where a pair would be in the way |
| (a closing one typed before its twin) | Steps over it; a quote only where it closes the literal the cursor is in | A pair typed through stays one pair |
| (Backspace between an empty pair) | Takes both out; an escaped `''` inside a string one at a time | Undoes what the pair put in |
| (a bracket or a quote typed over a selection) | Goes round it, which stays selected; one undo step | What an editor does with a selection and a bracket |
| (a frame of more than one edit, a paste, an input method's text) | Put in as it comes, unpaired; an input method's one character too | The pair is worked out from the text before the frame; two edits in one frame are rare and would read a stale text; what an input method or a paste gives is meant as it is |
| "Wrap" (no more said) | At the editor's width; the editor then scrolls down alone; a line's number on its first row; the statement's bar and the cursor's line over every row of their lines; a press beside a wrapped row puts the cursor at its line's start | What the gutter shows for a line, wrapped |

A reviewer given the draft broke it with tests of its own, and the draft now fixes what it found:

- A quote paired at the end of a line comment with a line under it (`--`, and `#` on MySQL), and at the end of a dollar quote, of a string ending in an escaped `''`, of a quoted name ending in `""`, and of `/*/`; a closed `E'a'` was taken for open. Whether the cursor is inside a literal was worked out from a token's first and last characters; it is the tokenizer's to say now, of the text up to the cursor with a space after it (`a_literal_is_told_as_the_dialect_reads_it`).
- A quote typed just before the next string's opening quote stepped over it. It steps over only where it closes the literal the cursor is in.
- Backspace between the two quotes of an escaped `''` inside a string took both out (`backspace_inside_an_escaped_quote_takes_one_out`).
- With Wrap on, the completion list hung a row too high under a word that starts a wrapped row: the word's place was read at the end of the row before (`the_completion_list_hangs_under_a_word_that_starts_a_wrapped_row`).
- With Wrap on, a press on the gutter beside a row a line wraps to put the cursor at that row's start, mid-line. It goes to the line's start now; under the last row it is still the script's end (`a_press_beside_a_wrapped_row_puts_the_cursor_at_its_line_s_start`).
- An input method's text and a character typed in one frame were taken for one edit, and the character paired (`an_input_method_s_text_in_the_frame_leaves_a_bracket_unpaired`).

It found the window's event shape (a key's press with its character) paired as the tests send it, Shift held for `"`, key repeat, CRLF, the start of the text, a paste of `(`, Tab's indent then `(`, Enter accepting a completion then `(`, Omarchy's insert and normal modes, a trailing line break and an empty script numbered right, relative numbers wrapped, and the focus kept when Wrap changes.

Left as they are, for the record:

- A prefixed string (`E'`, `N'`, `X'`, `B'`) is not paired: its quote follows a letter, as in a word.
- `)` steps over a `)` after the cursor even where the brackets do not balance (`f(g(x|)`).
- After a selection is wrapped and the caret moved, the first undo puts the selection back first, as for an indent over a selection.

A second reviewer read this plan against the draft. What it found is fixed:

- A quote typed before a quote that closes nothing (a `"` inside `'{"a":1}'`, the second of an escaped `''`) stepped over it. It steps over only where the literal it is in ends right there (`closes_literal`).
- A cut (Cmd+X) in the frame was not counted as an edit, nor a chord with Ctrl or Cmd.
- A backtick paired on MySQL alone; SQLite quotes names with it too, so it pairs there (`Dialect::backtick_names`, public now).
- Tests that could pass without testing: the completion list's test now requires a length that puts the word at a row's start, and the Omarchy test reads the pair and the cursor after `(` alone. New: a paste, Omarchy's normal mode, a selection made forwards, the rows' defaults on the Omarchy screen, and a unit test of `line_rows` (a trailing line break, an empty script).

Not testable headless, for the record: two characters typed quickly merge into one undo step as the field merges typing; the harness's clock does not move between frames, so the tests undo per frame.

## Files

| File | What it does after this plan |
|---|---|
| `src/ui/sql_text.rs` | `close_pairs`, `paired`, `in_literal`, `closes_literal`, `closer_of`: the pairs, before the field; `layouter(.., wraps)`, `line_rows`, the gutter and `Edited::span` by line, the field's width and the scroll's direction, the gutter's press and the completion list's place on a wrapped row. |
| `crates/tabletist-db/src/sql.rs` | `Dialect::backtick_names` public, for the backtick's pair. |
| `src/settings.rs`, `src/model.rs` | The two rows in `OptionId::TYPING` and `TERMINAL_TYPING`, out of `EDITOR_LATER`; the Omarchy cursor's rows. |
| `src/ui/typing_settings_tests.rs` | New: this step's headless tests. |
| `src/ui/mod.rs`, `src/ui/editor_settings_tests.rs` | Declares the tests; step 1's tests meet the two rows. |
| `docs/_reference/settings-and-files.md`, `docs/_guide/sql-editor.md` | The two rows and their keys. |

Every task ends with the repository's checks passing:

    cargo fmt --all --check
    cargo clippy --locked --workspace --all-targets -- -D warnings
    cargo test --locked --workspace --all-targets
    RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps

(`cargo` is `~/.cargo/bin/cargo` in these sessions.) Commit after each task, signed as usual. If signing says "agent refused operation", 1Password is locked: leave the change staged and say so.

---

### Task 1: Close brackets and quotes

**Files:**
- Modify: `src/ui/sql_text.rs`, `crates/tabletist-db/src/sql.rs`
- Create: `src/ui/typing_settings_tests.rs`
- Modify (tests): `src/ui/mod.rs`

- [ ] **Step 1: Write the failing tests**

```diff
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -91,6 +91,8 @@ mod text_settings_tests;
 #[cfg(test)]
 mod theme_settings_tests;
 pub mod toast;
+#[cfg(test)]
+mod typing_settings_tests;
 pub mod value_tags;
 pub mod widgets;
 pub mod workspace;
```

```diff
diff --git a/src/ui/sql_text.rs b/src/ui/sql_text.rs
--- a/src/ui/sql_text.rs
+++ b/src/ui/sql_text.rs
@@ -1162,6 +1162,78 @@ mod tests {
         assert_eq!(byte_offset("", 3), 0);
     }
 
+    #[test]
+    fn a_pair_goes_in_where_nothing_follows_that_it_would_split_off() {
+        let pair = |text: &str, typed| paired(text, Dialect::Postgres, text.len(), typed);
+        assert_eq!(pair("SELECT count", '('), Some(Paired::Pair(')')));
+        assert_eq!(pair("SELECT ", '\''), Some(Paired::Pair('\'')));
+        assert_eq!(pair("SELECT ", '"'), Some(Paired::Pair('"')));
+        // A backtick quotes names on MySQL alone.
+        assert_eq!(pair("SELECT ", '`'), None);
+        let mysql = paired("SELECT ", Dialect::MySql, 7, '`');
+        assert_eq!(mysql, Some(Paired::Pair('`')));
+        // Before a word, after a word or a quote, inside a literal: none.
+        assert_eq!(paired("SELECT x", Dialect::Postgres, 7, '('), None);
+        assert_eq!(pair("SELECT it", '\''), None);
+        assert_eq!(pair("SELECT ''", '\''), None);
+        assert_eq!(pair("SELECT 'a ", '\''), None);
+        assert_eq!(pair("-- a ", '"'), None);
+        assert_eq!(pair("/* a ", '"'), None);
+        // Closed, a comment or a string is behind the cursor.
+        assert_eq!(pair("/* a */ ", '"'), Some(Paired::Pair('"')));
+        assert_eq!(pair("SELECT 'a' ", '"'), Some(Paired::Pair('"')));
+        // Before a closing one or a comma: room.
+        assert_eq!(
+            paired("f()", Dialect::Postgres, 2, '['),
+            Some(Paired::Pair(']'))
+        );
+        // Its twin after the cursor: stepped over.
+        assert_eq!(paired("f()", Dialect::Postgres, 2, ')'), Some(Paired::Over));
+        assert_eq!(paired("''", Dialect::Postgres, 1, '\''), Some(Paired::Over));
+        // A closing one with nothing to step over is typed.
+        assert_eq!(pair("f(", ')'), None);
+    }
+
+    #[test]
+    fn a_literal_is_told_as_the_dialect_reads_it() {
+        use Dialect::{MySql, Postgres};
+        let at = |text: &str, dialect, at, typed| paired(text, dialect, at, typed);
+        // A line comment with a line under it, `--` and MySQL's `#`.
+        assert_eq!(at("-- note \nSELECT 1", Postgres, 8, '"'), None);
+        assert_eq!(at("SELECT 1 # x \nSELECT 2", MySql, 13, '\''), None);
+        // Left open at the end: dollar quotes, an escaped quote, a quoted
+        // name, a block comment that only looks closed.
+        assert_eq!(at("SELECT $$", Postgres, 9, '\''), None);
+        assert_eq!(at("SELECT $f$", Postgres, 10, '\''), None);
+        assert_eq!(at("SELECT 'it''", Postgres, 12, '"'), None);
+        assert_eq!(at("SELECT \"a\"\"", Postgres, 11, '\''), None);
+        assert_eq!(at("/*/", Postgres, 3, '\''), None);
+        // Closed, whatever its first character: room for a pair.
+        assert_eq!(
+            at("SELECT E'a' ", Postgres, 12, '"'),
+            Some(Paired::Pair('"'))
+        );
+        // A quote before the next string's opening one is typed, not
+        // stepped over.
+        let two = "SELECT 'a',  'b'";
+        assert_eq!(at(two, Postgres, two.find("'b'").unwrap(), '\''), None);
+        // In a string, before its closing quote: stepped over.
+        assert_eq!(at("SELECT 'a'", Postgres, 9, '\''), Some(Paired::Over));
+        // Before a quote that closes nothing it is typed: one inside a
+        // string of the other quote, one of an escaped pair.
+        assert_eq!(at("'{\"a\":1}'", Postgres, 4, '"'), None);
+        assert_eq!(at("'it''s'", Postgres, 3, '\''), None);
+        // Backticks quote names on MySQL and SQLite, and step over there.
+        assert_eq!(at("``", MySql, 1, '`'), Some(Paired::Over));
+        assert_eq!(
+            at("SELECT ", Dialect::Sqlite, 7, '`'),
+            Some(Paired::Pair('`'))
+        );
+        // MySQL's `--` is a comment only before a space: right after it,
+        // a quote pairs.
+        assert_eq!(at("SELECT 1 --", MySql, 11, '\''), Some(Paired::Pair('\'')));
+    }
+
     #[test]
     fn an_indent_goes_in_at_the_cursor_or_on_each_line_selected() {
         let tab = |text: &str, selection: Range<usize>, indent, outdent| {
@@ -1256,6 +1328,7 @@ mod tests {
                     look: &look,
                     palette: &palette,
                     hold_escape,
+                    pairs: false,
                     style: sql::format::Style::default(),
                     // A tab character: what Tab put in before the setting.
                     indent: Indent::Tab,
```

```diff
diff --git a/src/ui/typing_settings_tests.rs b/src/ui/typing_settings_tests.rs
new file mode 100644
--- /dev/null
+++ b/src/ui/typing_settings_tests.rs
@@ -0,0 +1,224 @@
+//! Settings › Editor's Close brackets and quotes and Wrap, in both looks:
+//! what a bracket or a quote typed puts in, what Backspace takes out, how
+//! long lines lay out with their numbers, and the rows that set them.
+
+use egui::{Key, Modifiers};
+
+use crate::model::{Action, ConnTabId};
+use crate::settings::OptionValue;
+use crate::testing::Harness;
+use crate::theme::Look;
+
+/// A connected fixture in `look`, with a new SQL editor that has the
+/// keyboard and holds `text`, its cursor at the end.
+fn editor(look: Look, text: &str) -> (Harness, ConnTabId) {
+    let mut harness = Harness::new();
+    harness.set_look(look);
+    let tab = harness.connect_fake();
+    harness.press(Key::T, Modifiers::COMMAND);
+    if !text.is_empty() {
+        harness.frame(vec![egui::Event::Paste(text.into())]);
+    }
+    harness.settle();
+    (harness, tab)
+}
+
+fn text(harness: &Harness, tab: ConnTabId) -> String {
+    let workspace = harness.app.workspace(tab).unwrap();
+    workspace.active_sql_tab().unwrap().text.clone()
+}
+
+fn cursor(harness: &Harness, tab: ConnTabId) -> usize {
+    let workspace = harness.app.workspace(tab).unwrap();
+    workspace.active_sql_tab().unwrap().cursor
+}
+
+/// Types `typed` one frame for each character, as text alone (the window
+/// sends its key's press too: `a_bracket_typed_as_the_window_sends_it_is_paired`).
+fn type_chars(harness: &mut Harness, typed: &str) {
+    for c in typed.chars() {
+        harness.frame(vec![egui::Event::Text(c.to_string())]);
+        harness.settle();
+    }
+}
+
+fn set(harness: &mut Harness, value: OptionValue) {
+    harness.app.apply(Action::SetOption(value));
+    harness.settle();
+}
+
+#[test]
+fn a_bracket_gets_its_closing_one_and_a_closing_one_typed_steps_over() {
+    for (open, close) in [('(', ')'), ('[', ']'), ('{', '}')] {
+        let (mut harness, tab) = editor(Look::macos(), "SELECT count");
+        type_chars(&mut harness, &open.to_string());
+        assert_eq!(text(&harness, tab), format!("SELECT count{open}{close}"));
+        assert_eq!(cursor(&harness, tab), 13, "between the two");
+        type_chars(&mut harness, &format!("*{close}"));
+        assert_eq!(text(&harness, tab), format!("SELECT count{open}*{close}"));
+        assert_eq!(cursor(&harness, tab), 15, "past the closing one");
+    }
+}
+
+#[test]
+fn a_bracket_typed_as_the_window_sends_it_is_paired() {
+    // The window sends the press of the key with the character it types.
+    let (mut harness, tab) = editor(Look::macos(), "SELECT count");
+    harness.frame(vec![
+        crate::testing::key(Key::Num9, Modifiers::SHIFT),
+        egui::Event::Text("(".into()),
+        crate::testing::release(Key::Num9, Modifiers::SHIFT),
+    ]);
+    harness.settle();
+    assert_eq!(text(&harness, tab), "SELECT count()");
+    // Two characters in one frame are typed as they come, unpaired.
+    harness.frame(vec![
+        egui::Event::Text("*".into()),
+        egui::Event::Text(")".into()),
+    ]);
+    harness.settle();
+    assert_eq!(text(&harness, tab), "SELECT count(*))");
+}
+
+#[test]
+fn a_quote_is_paired_where_a_string_starts_and_nowhere_else() {
+    let (mut harness, tab) = editor(Look::macos(), "SELECT ");
+    type_chars(&mut harness, "'ebook'");
+    assert_eq!(text(&harness, tab), "SELECT 'ebook'");
+    assert_eq!(cursor(&harness, tab), 14);
+    // Inside a word, after a quote (an escape), and inside a string or a
+    // comment, the quote is typed alone.
+    for (before, typed, after) in [
+        ("SELECT it", "'", "SELECT it'"),
+        ("SELECT 'it", "'", "SELECT 'it'"),
+        ("SELECT 'a ", "'", "SELECT 'a '"),
+        ("-- note ", "'", "-- note '"),
+        ("SELECT ''", "'", "SELECT '''"),
+    ] {
+        let (mut harness, tab) = editor(Look::macos(), before);
+        type_chars(&mut harness, typed);
+        assert_eq!(text(&harness, tab), after, "{before:?}");
+    }
+    // Not before a word: the bracket would split it off.
+    let (mut harness, tab) = editor(Look::macos(), "SELECT title");
+    for _ in 0.."title".len() {
+        harness.press(Key::ArrowLeft, Modifiers::NONE);
+    }
+    type_chars(&mut harness, "(");
+    assert_eq!(text(&harness, tab), "SELECT (title");
+}
+
+#[test]
+fn backspace_between_an_empty_pair_takes_both_out() {
+    let (mut harness, tab) = editor(Look::macos(), "SELECT count");
+    type_chars(&mut harness, "(");
+    harness.press(Key::Backspace, Modifiers::NONE);
+    harness.settle();
+    assert_eq!(text(&harness, tab), "SELECT count");
+    // Not a pair with something between.
+    type_chars(&mut harness, "(*");
+    harness.press(Key::Backspace, Modifiers::NONE);
+    harness.press(Key::ArrowRight, Modifiers::NONE);
+    harness.press(Key::Backspace, Modifiers::NONE);
+    harness.settle();
+    assert_eq!(text(&harness, tab), "SELECT count(");
+}
+
+#[test]
+fn backspace_inside_an_escaped_quote_takes_one_out() {
+    let (mut harness, tab) = editor(Look::macos(), "SELECT 'it''s'");
+    put_cursor(&mut harness, tab, 11);
+    harness.press(Key::Backspace, Modifiers::NONE);
+    harness.settle();
+    assert_eq!(text(&harness, tab), "SELECT 'it's'");
+}
+
+#[test]
+fn an_input_method_s_text_in_the_frame_leaves_a_bracket_unpaired() {
+    let (mut harness, tab) = editor(Look::macos(), "SELECT ");
+    harness.frame(vec![
+        egui::Event::Ime(egui::ImeEvent::Commit("x".into())),
+        egui::Event::Text("(".into()),
+    ]);
+    harness.settle();
+    assert_eq!(text(&harness, tab), "SELECT x(");
+}
+
+/// Puts the editor's cursor at the character `at`, as a click there does.
+fn put_cursor(harness: &mut Harness, tab: ConnTabId, at: usize) {
+    use egui::text::{CCursor, CCursorRange};
+    let workspace = harness.app.workspace(tab).unwrap();
+    let id = workspace.active_sql_tab().unwrap().id;
+    let editor = crate::ui::sql_text::editor_id(tab, id);
+    let mut state = egui::TextEdit::load_state(&harness.ctx, editor).expect("an editor");
+    state
+        .cursor
+        .set_char_range(Some(CCursorRange::one(CCursor::new(at))));
+    egui::TextEdit::store_state(&harness.ctx, editor, state);
+    harness.settle();
+}
+
+#[test]
+fn a_bracket_over_a_selection_puts_it_between_and_keeps_it_selected() {
+    let (mut harness, tab) = editor(Look::macos(), "SELECT title");
+    for _ in 0.."title".len() {
+        harness.press(Key::ArrowLeft, Modifiers::SHIFT);
+    }
+    type_chars(&mut harness, "(");
+    assert_eq!(text(&harness, tab), "SELECT (title)");
+    assert_eq!(cursor(&harness, tab), 8, "the moving end where it was");
+    // Still selected: a quote goes round it again.
+    type_chars(&mut harness, "\"");
+    assert_eq!(text(&harness, tab), "SELECT (\"title\")");
+    // One undo each.
+    harness.press(Key::Z, Modifiers::COMMAND);
+    harness.settle();
+    assert_eq!(text(&harness, tab), "SELECT (title)");
+    harness.press(Key::Z, Modifiers::COMMAND);
+    harness.settle();
+    assert_eq!(text(&harness, tab), "SELECT title");
+}
+
+#[test]
+fn a_selection_made_forwards_keeps_its_moving_end_at_the_end() {
+    let (mut harness, tab) = editor(Look::macos(), "SELECT title");
+    for _ in 0.."title".len() {
+        harness.press(Key::ArrowLeft, Modifiers::NONE);
+    }
+    for _ in 0.."title".len() {
+        harness.press(Key::ArrowRight, Modifiers::SHIFT);
+    }
+    type_chars(&mut harness, "[");
+    assert_eq!(text(&harness, tab), "SELECT [title]");
+    assert_eq!(cursor(&harness, tab), 13);
+}
+
+#[test]
+fn with_close_pairs_off_a_bracket_is_typed_alone() {
+    let (mut harness, tab) = editor(Look::macos(), "SELECT count");
+    set(&mut harness, OptionValue::ClosePairs(false));
+    type_chars(&mut harness, "('");
+    assert_eq!(text(&harness, tab), "SELECT count('");
+}
+
+#[test]
+fn the_terminal_look_pairs_in_insert_mode_and_not_out_of_the_text() {
+    let (mut harness, tab) = editor(Look::omarchy(), "select count");
+    type_chars(&mut harness, "(");
+    assert_eq!(text(&harness, tab), "select count()");
+    assert_eq!(cursor(&harness, tab), 13);
+    type_chars(&mut harness, "*)");
+    assert_eq!(text(&harness, tab), "select count(*)");
+    // Normal mode: the keyboard out of the text, which types nothing.
+    harness.press(Key::Escape, Modifiers::NONE);
+    type_chars(&mut harness, "(");
+    assert_eq!(text(&harness, tab), "select count(*)");
+}
+
+#[test]
+fn a_paste_is_put_in_as_it_is() {
+    let (mut harness, tab) = editor(Look::macos(), "SELECT count");
+    harness.frame(vec![egui::Event::Paste("(".into())]);
+    harness.settle();
+    assert_eq!(text(&harness, tab), "SELECT count(");
+}
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test --locked --lib -- typing_settings_tests sql_text::tests`
Expected: FAIL to compile (22 errors), among them: `sql_text` has no `paired` or `Paired`, and its `Field` no `pairs`.

- [ ] **Step 3: Pair brackets and quotes**

```diff
diff --git a/crates/tabletist-db/src/sql.rs b/crates/tabletist-db/src/sql.rs
--- a/crates/tabletist-db/src/sql.rs
+++ b/crates/tabletist-db/src/sql.rs
@@ -199,7 +199,7 @@ impl Dialect {
     }
 
     /// `` `name` `` is a quoted name.
-    fn backtick_names(self) -> bool {
+    pub fn backtick_names(self) -> bool {
         match self {
             Self::MySql | Self::Sqlite => true,
             Self::Postgres => false,
```

```diff
diff --git a/src/ui/sql_text.rs b/src/ui/sql_text.rs
--- a/src/ui/sql_text.rs
+++ b/src/ui/sql_text.rs
@@ -448,6 +448,7 @@ pub fn show(app: &mut App, ui: &mut Ui, tab: ConnTabId, id: TabId) {
     let numbering = app.settings.line_numbers_in(look.terminal);
     let (style, indent) = (format_style(&app.settings, &look), app.settings.indent);
     let highlight = app.settings.highlight_line;
+    let pairs = app.settings.close_pairs;
     let Some(workspace) = app.workspace_mut(tab) else {
         return;
     };
@@ -478,6 +479,7 @@ pub fn show(app: &mut App, ui: &mut Ui, tab: ConnTabId, id: TabId) {
         // keyboard. Not one whose row this frame inserts: Esc is the
         // editor's again.
         hold_escape: sql_tab.completion.as_ref().is_some_and(|list| !list.accept),
+        pairs,
     };
     let mut child = ui.new_child(egui::UiBuilder::new().id_salt("text").max_rect(text_pane));
     child.set_clip_rect(text_pane.intersect(ui.clip_rect()));
@@ -694,6 +696,8 @@ struct Field<'a> {
     palette: &'a Palette,
     /// Keep the keyboard on Esc (a completion list is open and takes it).
     hold_escape: bool,
+    /// A bracket or a quote typed gets its closing one.
+    pairs: bool,
     /// How Format writes the script.
     style: sql::format::Style,
     /// What the Tab key puts in, and takes out with Shift.
@@ -911,6 +915,254 @@ fn indent_script(ui: &mut Ui, sql_tab: &mut SqlTab, field: &Field<'_>) -> bool {
     changed
 }
 
+/// The closing character of a bracket or a quote that `typed` opens in
+/// `dialect`: a backtick only where it quotes names (MySQL, SQLite).
+fn closer_of(typed: char, dialect: Dialect) -> Option<char> {
+    match typed {
+        '(' => Some(')'),
+        '[' => Some(']'),
+        '{' => Some('}'),
+        '\'' | '"' => Some(typed),
+        '`' => dialect.backtick_names().then_some('`'),
+        _ => None,
+    }
+}
+
+/// Whether the quote `typed` at byte `at` of `text` would close the
+/// string or the quoted name the cursor is in: the literal's own last
+/// character, its closing quote, is right after the cursor. A quote of an
+/// escaped pair, or one inside a literal of another kind, closes nothing.
+fn closes_literal(text: &str, dialect: Dialect, at: usize, typed: char) -> bool {
+    sql::tokenize(dialect, text).iter().any(|token| {
+        let quoted = matches!(token.kind, TokenKind::String | TokenKind::QuotedIdentifier);
+        let range = &token.range;
+        quoted
+            && range.start < at
+            && range.end == at + typed.len_utf8()
+            && text[range.clone()].ends_with(typed)
+            && text[range.clone()]
+                .trim_start_matches(char::is_alphabetic)
+                .starts_with(typed)
+    })
+}
+
+/// Whether `at` is inside a string, a quoted name or a comment of `text`
+/// (one left open counts to its end): a quote typed there closes one or
+/// is text, and is never paired. Told by the tokenizer, of the text up to
+/// `at` with a letter after it: a literal that takes the letter in goes
+/// on past `at`. So a line comment with lines under it, a dollar quote, an
+/// escaped quote at an open string's end are read as the dialect reads
+/// them.
+fn in_literal(text: &str, dialect: Dialect, at: usize) -> bool {
+    // A letter, not a space: MySQL reads `--` as a comment only before a
+    // space, and the text after the cursor is not known to be one.
+    let probe = format!("{}x", &text[..at]);
+    sql::tokenize(dialect, &probe).iter().any(|token| {
+        let literal = matches!(
+            token.kind,
+            TokenKind::String
+                | TokenKind::QuotedIdentifier
+                | TokenKind::Comment
+                | TokenKind::ExecutableComment
+        );
+        literal && token.range.start < at && at < token.range.end
+    })
+}
+
+/// The edit Close brackets and quotes makes of a character typed with
+/// nothing selected, at byte `at` of `text`: its pair put in (`Pair`), a
+/// closing character stepped over (`Over`), or none.
+#[derive(Debug, PartialEq, Eq)]
+enum Paired {
+    Pair(char),
+    Over,
+}
+
+fn paired(text: &str, dialect: Dialect, at: usize, typed: char) -> Option<Paired> {
+    let next = text[at..].chars().next();
+    let before = text[..at].chars().next_back();
+    // Its own closing character already there: stepped over. A quote
+    // only where it closes the literal the cursor is in: one that opens
+    // the next string is typed before it.
+    let bracket = matches!(typed, ')' | ']' | '}');
+    let quote = closer_of(typed, dialect) == Some(typed);
+    let closes = bracket || (quote && closes_literal(text, dialect, at, typed));
+    if closes && next == Some(typed) {
+        return Some(Paired::Over);
+    }
+    let closer = closer_of(typed, dialect)?;
+    // Only where nothing follows that the pair would split off.
+    let room = next.is_none_or(|next| next.is_whitespace() || ")]},;".contains(next));
+    if !room {
+        return None;
+    }
+    if closer == typed {
+        // A quote: not inside a word (`it's`), after another quote (`''`
+        // escapes), or inside a string or a comment.
+        let wordy = before
+            .is_some_and(|before| before.is_alphanumeric() || before == '_' || before == typed);
+        if wordy || in_literal(text, dialect, at) {
+            return None;
+        }
+    }
+    Some(Paired::Pair(closer))
+}
+
+/// Close brackets and quotes, before the field reads the frame's events:
+/// a bracket or a quote typed gets its closing one after the caret, a
+/// closing one typed before its twin steps over it, Backspace between an
+/// empty pair takes both out, and a bracket or a quote typed over a
+/// selection puts the selection between the two (one step of the undo
+/// history, the selection kept). The other edits are the field's own
+/// (the pair typed and the caret moved back), so they undo as typing
+/// does. Only in a frame of one edit, while the field has the keyboard.
+/// Returns whether the text changed here.
+fn close_pairs(ui: &mut Ui, sql_tab: &mut SqlTab, field: &Field<'_>) -> bool {
+    if !field.pairs || !ui.memory(|memory| memory.has_focus(field.id)) {
+        return false;
+    }
+    let key = |key: egui::Key| egui::Event::Key {
+        key,
+        physical_key: None,
+        pressed: true,
+        repeat: false,
+        modifiers: egui::Modifiers::NONE,
+    };
+    // The frame's one edit: a character typed, or a plain Backspace. A
+    // character comes with the press of its key too, which is no edit of
+    // its own; the keys that edit or move the caret are.
+    let edit = ui.input(|input| {
+        use egui::Key as K;
+        let edits: Vec<&egui::Event> = input
+            .events
+            .iter()
+            .filter(|event| match event {
+                egui::Event::Text(_)
+                | egui::Event::Paste(_)
+                | egui::Event::Cut
+                | egui::Event::Ime(egui::ImeEvent::Commit(_)) => true,
+                // The text field's own editing keys, which the keymap does
+                // not hold, and any chord (undo, redo).
+                egui::Event::Key {
+                    key,
+                    pressed,
+                    modifiers,
+                    ..
+                } => {
+                    *pressed && (modifiers.command || modifiers.ctrl)
+                        || *pressed
+                            && matches!(
+                                key,
+                                K::Backspace
+                                    | K::Delete
+                                    | K::Enter
+                                    | K::Tab
+                                    | K::ArrowLeft
+                                    | K::ArrowRight
+                                    | K::ArrowUp
+                                    | K::ArrowDown
+                                    | K::Home
+                                    | K::End
+                                    | K::PageUp
+                                    | K::PageDown
+                            )
+                }
+                _ => false,
+            })
+            .collect();
+        match edits.as_slice() {
+            [egui::Event::Text(text)] if text.chars().count() == 1 => text.chars().next(),
+            [
+                egui::Event::Key {
+                    key: egui::Key::Backspace,
+                    modifiers,
+                    ..
+                },
+            ] if modifiers.is_none() => Some('\u{8}'),
+            _ => None,
+        }
+    });
+    let Some(typed) = edit else {
+        return false;
+    };
+    let mut state = egui::TextEdit::load_state(ui.ctx(), field.id).unwrap_or_default();
+    let Some(range) = state.cursor.char_range() else {
+        return false;
+    };
+    let [start, end] = range.sorted_cursors();
+    let byte = |cursor: CCursor| byte_offset(&sql_tab.text, cursor.index.0);
+    let (start, end) = (byte(start), byte(end));
+    let text = &sql_tab.text;
+    if typed == '\u{8}' {
+        // Backspace between an empty pair: the closing one goes with it.
+        let before = text[..start].chars().next_back();
+        let next = text[start..].chars().next();
+        // A quote's pair only where the first one opens a literal: the two
+        // of an escaped quote (`'it''s'`) are inside one.
+        let empty = start == end
+            && before.is_some_and(|before| {
+                let opens = closer_of(before, field.dialect).is_some_and(|c| Some(c) == next);
+                let escaped = before == next.unwrap_or_default()
+                    && in_literal(text, field.dialect, start - before.len_utf8());
+                opens && !escaped
+            });
+        if empty {
+            ui.input_mut(|input| input.events.push(key(egui::Key::Delete)));
+        }
+        return false;
+    }
+    if start != end {
+        // Over a selection: the selection between the two.
+        let Some(closer) = closer_of(typed, field.dialect) else {
+            return false;
+        };
+        ui.input_mut(|input| {
+            input
+                .events
+                .retain(|event| !matches!(event, egui::Event::Text(_)));
+        });
+        let inner = &text[start..end];
+        let out = [
+            &text[..start],
+            &typed.to_string(),
+            inner,
+            &closer.to_string(),
+            &text[end..],
+        ]
+        .concat();
+        let (from, to) = (start + typed.len_utf8(), end + typed.len_utf8());
+        let at = |byte: usize| CCursor::new(char_index(&out, byte));
+        let backwards = range.primary.index < range.secondary.index;
+        let after = if backwards {
+            CCursorRange::two(at(to), at(from))
+        } else {
+            CCursorRange::two(at(from), at(to))
+        };
+        let before = (range, std::mem::take(&mut sql_tab.text));
+        let after = (after, out);
+        outside_edit(&mut state, ui.input(|input| input.time), &before, &after);
+        egui::TextEdit::store_state(ui.ctx(), field.id, state);
+        sql_tab.text = after.1;
+        sql_tab.cursor = if backwards { from } else { to };
+        return true;
+    }
+    let events = match paired(text, field.dialect, start, typed) {
+        Some(Paired::Pair(closer)) => vec![
+            egui::Event::Text(format!("{typed}{closer}")),
+            key(egui::Key::ArrowLeft),
+        ],
+        Some(Paired::Over) => vec![key(egui::Key::ArrowRight)],
+        None => return false,
+    };
+    ui.input_mut(|input| {
+        input
+            .events
+            .retain(|event| !matches!(event, egui::Event::Text(_)));
+        input.events.extend(events);
+    });
+    false
+}
+
 /// The text field itself: edits the tab's text and reports its cursor.
 fn edit(ui: &mut Ui, sql_tab: &mut SqlTab, field: &Field<'_>) -> Edited {
     let inserted = insert_completion(ui.ctx(), field.id, sql_tab);
@@ -919,8 +1171,9 @@ fn edit(ui: &mut Ui, sql_tab: &mut SqlTab, field: &Field<'_>) -> Edited {
     // An indent is an outside edit too: what the field shows, and where
     // its caret goes, follow as after Format.
     let indented = indent_script(ui, sql_tab, field);
+    let paired = close_pairs(ui, sql_tab, field);
     let formatted = std::mem::take(&mut sql_tab.format) && format_script(ui, sql_tab, field);
-    let formatted = formatted || indented;
+    let formatted = formatted || indented || paired;
     // The field fills the pane, so a click under the last line lands in
     // it: whole lines, and the rest as bottom padding. (egui sizes a text
     // edit in lines of its font.)
```

- [ ] **Step 4: Run the tests to see them pass**

Run: `cargo test --locked --lib -- typing_settings_tests sql_text::tests ui::complete_tests ui::format_settings_tests`
Expected: PASS. Then the four checks above.

- [ ] **Step 5: Commit**

```bash
git add crates/tabletist-db/src/sql.rs src/ui/mod.rs src/ui/sql_text.rs src/ui/typing_settings_tests.rs
git commit -m "Close brackets and quotes in the SQL editor as Settings › Editor says"
```

---

### Task 2: Wrap

**Files:**
- Modify: `src/ui/sql_text.rs`
- Modify (tests): `src/ui/typing_settings_tests.rs`

- [ ] **Step 1: Write the failing tests**

```diff
diff --git a/src/ui/sql_text.rs b/src/ui/sql_text.rs
--- a/src/ui/sql_text.rs
+++ b/src/ui/sql_text.rs
@@ -1359,7 +1359,7 @@ mod tests {
         let mut harness = crate::testing::Harness::new();
         harness.frame_with(|ui| {
             let id = Id::new("an editor");
-            let mut layouter = layouter(&look, &palette, Dialect::Postgres, id);
+            let mut layouter = layouter(&look, &palette, Dialect::Postgres, id, false);
             // A width to wrap at is not taken: lines scroll instead.
             galleys.push(layouter(ui, &long, 100.0));
             galleys.push(layouter(ui, &String::new(), 100.0));
@@ -1385,6 +1385,30 @@ mod tests {
         assert_eq!(empty.rows[0].height(), galley.rows[0].height());
     }
 
+    #[test]
+    fn a_line_starts_its_rows_and_a_wrapped_line_has_more() {
+        let (look, palette) = (Look::standard(), Palette::light());
+        let long = format!("SELECT {}\nFROM t\n", "title, ".repeat(40));
+        let mut galleys = Vec::new();
+        let mut harness = crate::testing::Harness::new();
+        harness.frame_with(|ui| {
+            let id = Id::new("an editor");
+            let mut wrapping = layouter(&look, &palette, Dialect::Postgres, id, true);
+            galleys.push(wrapping(ui, &long, 300.0));
+            galleys.push(wrapping(ui, &String::new(), 300.0));
+            galleys.push(wrapping(ui, &"a\nb\n".to_owned(), 300.0));
+        });
+        let wrapped = line_rows(&galleys[0]);
+        // Three lines (the last empty, after the break), the first of
+        // several rows.
+        assert_eq!(wrapped.len(), 3);
+        assert_eq!(wrapped[0], 0);
+        assert!(wrapped[1] > 1, "{wrapped:?}");
+        assert_eq!(wrapped[2], galleys[0].rows.len() - 1);
+        assert_eq!(line_rows(&galleys[1]), [0]);
+        assert_eq!(line_rows(&galleys[2]), [0, 1, 2]);
+    }
+
     #[test]
     fn a_byte_offset_becomes_a_character_index() {
         assert_eq!(char_index("SELECT 1", 0), 0);
@@ -1585,6 +1609,7 @@ mod tests {
                     style: sql::format::Style::default(),
                     // A tab character: what Tab put in before the setting.
                     indent: Indent::Tab,
+                    wraps: false,
                 };
                 focused = edit(ui, sql, &field).focused;
             });
```

```diff
diff --git a/src/ui/typing_settings_tests.rs b/src/ui/typing_settings_tests.rs
--- a/src/ui/typing_settings_tests.rs
+++ b/src/ui/typing_settings_tests.rs
@@ -2,11 +2,12 @@
 //! what a bracket or a quote typed puts in, what Backspace takes out, how
 //! long lines lay out with their numbers, and the rows that set them.
 
+use egui::accesskit::Role;
 use egui::{Key, Modifiers};
 
 use crate::model::{Action, ConnTabId};
 use crate::settings::OptionValue;
-use crate::testing::Harness;
+use crate::testing::{Harness, bounds};
 use crate::theme::Look;
 
 /// A connected fixture in `look`, with a new SQL editor that has the
@@ -222,3 +223,147 @@ fn a_paste_is_put_in_as_it_is() {
     harness.settle();
     assert_eq!(text(&harness, tab), "SELECT count(");
 }
+
+/// The tops of the gutter's numbers, in order, with each number.
+fn numbers(harness: &mut Harness) -> Vec<(String, f32)> {
+    let tree = harness.settle();
+    let field = bounds(&tree, "SQL", Role::MultilineTextInput).expect("the editor");
+    let mut found: Vec<(String, f32)> = harness
+        .text_rects
+        .iter()
+        .filter(|(text, rect)| {
+            rect.right() <= field.left() && text.bytes().all(|byte| byte.is_ascii_digit())
+        })
+        .map(|(text, rect)| (text.clone(), rect.top()))
+        .collect();
+    found.sort_by(|a, b| a.1.total_cmp(&b.1));
+    found
+}
+
+#[test]
+fn wrap_lays_a_long_line_out_in_rows_numbered_once() {
+    let long = format!("SELECT {}\nFROM book_covers", "title, ".repeat(60));
+    let (mut harness, _) = editor(Look::macos(), &long);
+    // Off: one row a line, the second right under the first.
+    let off = numbers(&mut harness);
+    assert_eq!(
+        off.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>(),
+        ["1", "2"]
+    );
+    let row = off[1].1 - off[0].1;
+    set(&mut harness, OptionValue::Wrap(true));
+    let on = numbers(&mut harness);
+    assert_eq!(
+        on.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>(),
+        ["1", "2"]
+    );
+    assert!(
+        on[1].1 - on[0].1 > 2.5 * row,
+        "the first line wraps to rows"
+    );
+    // The editor keeps the keyboard.
+    assert!(harness.ctx.text_edit_focused());
+}
+
+#[test]
+fn a_wrapped_line_is_one_line_to_the_bar_and_the_cursor() {
+    let long = format!("SELECT {}", "title, ".repeat(60));
+    let (mut harness, _) = editor(Look::macos(), &long);
+    set(&mut harness, OptionValue::Wrap(true));
+    harness.settle();
+    let palette = harness.app.palette;
+    let line = palette.window.lerp_to_gamma(palette.selection, 0.45);
+    let bar = crate::ui::sql_text::bar_color(&harness.app.look, &palette);
+    let tallest = |color| {
+        harness
+            .fills
+            .iter()
+            .filter(|(_, fill)| *fill == color)
+            .map(|(rect, _)| rect.height())
+            .fold(0.0, f32::max)
+    };
+    // Both stand beside every row the line wraps to: far taller than one.
+    let rows = harness
+        .text_rects
+        .iter()
+        .find(|(text, _)| text == "1")
+        .map(|(_, rect)| rect.height())
+        .expect("the number");
+    assert!(tallest(line) > 2.5 * rows, "the cursor's line");
+    assert!(tallest(bar) > 2.5 * rows, "the statement's bar");
+}
+
+#[test]
+fn a_press_beside_a_wrapped_row_puts_the_cursor_at_its_line_s_start() {
+    let long = format!("SELECT {}", "title, ".repeat(60));
+    let (mut harness, tab) = editor(Look::macos(), &long);
+    set(&mut harness, OptionValue::Wrap(true));
+    let tree = harness.settle();
+    let gutter = bounds(&tree, "Line numbers", Role::Unknown)
+        .or_else(|| bounds(&tree, "Line numbers", Role::GenericContainer))
+        .expect("the gutter");
+    let number = numbers(&mut harness)[0].1;
+    // Two rows down the first line, beside a row with no number.
+    let at = egui::pos2(gutter.center().x, number + 50.0);
+    harness.frame(vec![
+        egui::Event::PointerMoved(at),
+        egui::Event::PointerButton {
+            pos: at,
+            button: egui::PointerButton::Primary,
+            pressed: true,
+            modifiers: Modifiers::NONE,
+        },
+        egui::Event::PointerButton {
+            pos: at,
+            button: egui::PointerButton::Primary,
+            pressed: false,
+            modifiers: Modifiers::NONE,
+        },
+    ]);
+    harness.settle();
+    assert_eq!(cursor(&harness, tab), 0);
+}
+
+#[test]
+fn the_completion_list_hangs_under_a_word_that_starts_a_wrapped_row() {
+    // Filler of one length or another puts the word typed at the start of
+    // a wrapped row: the list hangs under the word's own row each time.
+    let mut started = 0;
+    for words in 40..80 {
+        let (mut harness, _) = editor(Look::macos(), &"a ".repeat(words));
+        set(&mut harness, OptionValue::Wrap(true));
+        type_chars(&mut harness, "se");
+        let tree = harness.settle();
+        let Some(list) = bounds(&tree, "Completions", Role::Unknown)
+            .or_else(|| bounds(&tree, "Completions", Role::GenericContainer))
+            .or_else(|| bounds(&tree, "Completions", Role::ListBox))
+        else {
+            continue;
+        };
+        // The row the word is on: the last of the editor's rows, each a
+        // text run to a screen reader.
+        let word = tree
+            .nodes
+            .iter()
+            .filter(|(_, node)| node.role() == Role::TextRun)
+            .filter_map(|(_, node)| node.bounds())
+            .map(|rect| {
+                egui::Rect::from_min_max(
+                    egui::pos2(rect.x0 as f32, rect.y0 as f32),
+                    egui::pos2(rect.x1 as f32, rect.y1 as f32),
+                )
+            })
+            .max_by(|a, b| a.top().total_cmp(&b.top()))
+            .expect("the word's row");
+        assert!(
+            list.top() >= word.bottom() - 1.0,
+            "{words}: {list:?} {word:?}"
+        );
+        // The word alone on its row: it starts the row.
+        let field = bounds(&tree, "SQL", Role::MultilineTextInput).expect("the editor");
+        if word.left() < field.left() + 40.0 && word.width() < 40.0 {
+            started += 1;
+        }
+    }
+    assert!(started > 0, "no length put the word at a row's start");
+}
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test --locked --lib -- typing_settings_tests sql_text::tests`
Expected: FAIL to compile (6 errors): `sql_text` has no `line_rows`, `layouter` takes no `wraps`, and `Field` has no `wraps`.

- [ ] **Step 3: Wrap long lines, numbered once**

```diff
diff --git a/src/ui/sql_text.rs b/src/ui/sql_text.rs
--- a/src/ui/sql_text.rs
+++ b/src/ui/sql_text.rs
@@ -175,16 +175,18 @@ pub fn scroll_offset(ctx: &egui::Context, tab: ConnTabId, id: TabId) -> Option<e
 }
 
 /// A text edit layouter for the editor `id` that colours `dialect`'s
-/// tokens and never wraps: one galley row per line, so the gutter can
-/// number the rows.
+/// tokens. It wraps a long line at the width it is given where `wraps`
+/// (Settings › Editor's Wrap), and otherwise never: one galley row per
+/// line, and the lines scroll.
 pub fn layouter(
     look: &Look,
     palette: &Palette,
     dialect: Dialect,
     id: Id,
+    wraps: bool,
 ) -> impl FnMut(&Ui, &dyn egui::TextBuffer, f32) -> Arc<Galley> + use<> {
     let (faces, role, palette) = (look.faces, role(look), *palette);
-    move |ui, buffer, _wrap| {
+    move |ui, buffer, width| {
         let text = buffer.as_str();
         let parsed = parsed(ui.ctx(), id, dialect, &palette, text);
         let runs = parsed.runs.iter();
@@ -196,6 +198,9 @@ pub fn layouter(
             // An empty script still has a line, as tall as any other.
             laid = laid.add(role, "", palette.text);
         }
+        if wraps && width.is_finite() {
+            laid = laid.wrap(width);
+        }
         laid.layout(ui.ctx()).galley
     }
 }
@@ -431,11 +436,18 @@ struct Edited {
 }
 
 impl Edited {
-    /// The screen's vertical range of the 1-based `lines`, each line being
-    /// a row of the galley.
-    fn span(&self, lines: &RangeInclusive<usize>) -> Option<egui::Rangef> {
-        let first = self.galley.rows.get(lines.start().checked_sub(1)?)?;
-        let last = self.galley.rows.get(lines.end().checked_sub(1)?)?;
+    /// The screen's vertical range of the 1-based `lines`, each with every
+    /// row it wraps to.
+    /// `starts` is `line_rows` of the galley.
+    fn span(&self, starts: &[usize], lines: &RangeInclusive<usize>) -> Option<egui::Rangef> {
+        let first = *starts.get(lines.start().checked_sub(1)?)?;
+        // The row before the next line's first, or the galley's last.
+        let last = match starts.get(*lines.end()) {
+            Some(next) => next.checked_sub(1)?,
+            None if *lines.end() == starts.len() => self.galley.rows.len().checked_sub(1)?,
+            None => return None,
+        };
+        let (first, last) = (self.galley.rows.get(first)?, self.galley.rows.get(last)?);
         Some(egui::Rangef::new(
             self.origin.y + first.min_y(),
             self.origin.y + last.max_y(),
@@ -443,10 +455,23 @@ impl Edited {
     }
 }
 
+/// The first row of each line of `galley`, in order: a line wrapped at
+/// the editor's width is more than one row.
+fn line_rows(galley: &Galley) -> Vec<usize> {
+    let mut starts = vec![0];
+    for (index, row) in galley.rows.iter().enumerate() {
+        if row.ends_with_newline && index + 1 < galley.rows.len() {
+            starts.push(index + 1);
+        }
+    }
+    starts
+}
+
 pub fn show(app: &mut App, ui: &mut Ui, tab: ConnTabId, id: TabId) {
     let (look, palette, locale) = (app.look, app.palette, app.locale);
     let numbering = app.settings.line_numbers_in(look.terminal);
     let (style, indent) = (format_style(&app.settings, &look), app.settings.indent);
+    let wraps = app.settings.wrap;
     let highlight = app.settings.highlight_line;
     let pairs = app.settings.close_pairs;
     let Some(workspace) = app.workspace_mut(tab) else {
@@ -463,7 +488,8 @@ pub fn show(app: &mut App, ui: &mut Ui, tab: ConnTabId, id: TabId) {
     let editor = editor_id(tab, id);
     // Behind the text: filled in once the rows are laid out.
     let backdrop = ui.painter().add(Shape::Noop);
-    // The text scrolls both ways beside the gutter, which stays put.
+    // The text scrolls beside the gutter, which stays put: both ways, or
+    // down alone where it wraps.
     let text_pane = Rect::from_min_max(pos2(pane.left() + gutter.width, pane.top()), pane.max);
     let name = gettext(locale, "SQL");
     let field = Field {
@@ -475,6 +501,7 @@ pub fn show(app: &mut App, ui: &mut Ui, tab: ConnTabId, id: TabId) {
         palette: &palette,
         style,
         indent,
+        wraps,
         // An open completion list takes Esc: the editor keeps the
         // keyboard. Not one whose row this frame inserts: Esc is the
         // editor's again.
@@ -483,7 +510,8 @@ pub fn show(app: &mut App, ui: &mut Ui, tab: ConnTabId, id: TabId) {
     };
     let mut child = ui.new_child(egui::UiBuilder::new().id_salt("text").max_rect(text_pane));
     child.set_clip_rect(text_pane.intersect(ui.clip_rect()));
-    let scrolled = egui::ScrollArea::both()
+    // Wrapped, the lines never run past the pane: it scrolls down alone.
+    let scrolled = egui::ScrollArea::new([!wraps, true])
         .id_salt("scroll")
         .auto_shrink([false, false])
         .show(&mut child, |ui| edit(ui, sql_tab, &field));
@@ -501,9 +529,17 @@ pub fn show(app: &mut App, ui: &mut Ui, tab: ConnTabId, id: TabId) {
     if (pressed.is_pointer_button_down_on() || pressed.clicked())
         && let Some(pointer) = pressed.interact_pointer_pos()
     {
-        let at = edited
-            .galley
-            .cursor_from_pos(vec2(0.0, pointer.y - edited.origin.y));
+        let y = pointer.y - edited.origin.y;
+        let row = edited.galley.cursor_from_pos(vec2(0.0, y));
+        // Beside a row, the start of its line, not of a row the line wraps
+        // to. Under the last, the end of the script.
+        let at = if y <= edited.galley.rect.max.y {
+            let byte = byte_offset(&sql_tab.text, row.index.0);
+            let start = sql_tab.text[..byte].rfind('\n').map_or(0, |at| at + 1);
+            CCursor::new(char_index(&sql_tab.text, start))
+        } else {
+            row
+        };
         let mut state = egui::TextEdit::load_state(ui.ctx(), editor).unwrap_or_default();
         state.cursor.set_char_range(Some(CCursorRange::one(at)));
         egui::TextEdit::store_state(ui.ctx(), editor, state);
@@ -524,8 +560,10 @@ pub fn show(app: &mut App, ui: &mut Ui, tab: ConnTabId, id: TabId) {
     let error_line = sql_tab.error_mark().map(|(line, _)| line);
     // The statement Run executes, from the tokens the colours came from.
     let parsed = parsed(ui.ctx(), editor, dialect, &palette, &sql_tab.text);
+    // The first row of each line: a wrapped line has more than one.
+    let starts = line_rows(&edited.galley);
     let statement = sql::statement_at(&parsed.statements, sql_tab.cursor)
-        .and_then(|statement| edited.span(&statement_lines(statement)));
+        .and_then(|statement| edited.span(&starts, &statement_lines(statement)));
     if edited.focused && tree_has_arrows {
         app.actions
             .push(Action::SqlEditorFocused { tab, sql_tab: id });
@@ -555,7 +593,7 @@ pub fn show(app: &mut App, ui: &mut Ui, tab: ConnTabId, id: TabId) {
     // unless the settings turn it off.
     if edited.focused
         && highlight
-        && let Some(y) = edited.span(&(cursor_line..=cursor_line))
+        && let Some(y) = edited.span(&starts, &(cursor_line..=cursor_line))
     {
         let color = if look.terminal {
             palette.selection
@@ -575,13 +613,18 @@ pub fn show(app: &mut App, ui: &mut Ui, tab: ConnTabId, id: TabId) {
     }
     let clip = ui.clip_rect();
     let rows = &edited.galley.rows;
+    // A line's number stands on its first row: the rows a long line wraps
+    // to have none.
     let first = rows.partition_point(|row| edited.origin.y + row.max_y() < clip.top());
     for (index, row) in rows.iter().enumerate().skip(first) {
         let top = edited.origin.y + row.min_y();
         if top > clip.bottom() {
             break;
         }
-        let line = index + 1;
+        let Ok(line) = starts.binary_search(&index) else {
+            continue;
+        };
+        let line = line + 1;
         let color = if error_line == Some(line) {
             palette.danger
         } else if line == cursor_line {
@@ -637,11 +680,15 @@ fn completion_list(
     };
     let close = Action::CloseCompletion { tab, sql_tab };
     // A character's place on screen, as tall as its line.
-    let on_screen = |at: usize| {
-        let place = edited.galley.pos_from_cursor(CCursor::new(at));
+    // `next`: on the row after where a wrapped line breaks, as a word's
+    // start is.
+    let on_screen = |at: usize, next: bool| {
+        let mut cursor = CCursor::new(at);
+        cursor.prefer_next_row = next;
+        let place = edited.galley.pos_from_cursor(cursor);
         place.translate(edited.origin.to_vec2())
     };
-    let (word, end) = (on_screen(chars.start), on_screen(chars.end));
+    let (word, end) = (on_screen(chars.start, true), on_screen(chars.end, false));
     // The list hangs under its word while some of the word is in the
     // pane: its line is, and across the word starts before the pane's
     // right edge and ends after its left one.
@@ -702,6 +749,8 @@ struct Field<'a> {
     style: sql::format::Style,
     /// What the Tab key puts in, and takes out with Shift.
     indent: Indent,
+    /// Long lines wrap at the pane's width.
+    wraps: bool,
 }
 
 /// How Format writes, as the settings have it in `look`: its keyword case
@@ -1189,14 +1238,26 @@ fn edit(ui: &mut Ui, sql_tab: &mut SqlTab, field: &Field<'_>) -> Edited {
         top: top as i8,
         bottom: bottom as i8,
     };
-    let mut layouter = layouter(field.look, field.palette, field.dialect, field.id);
+    let mut layouter = layouter(
+        field.look,
+        field.palette,
+        field.dialect,
+        field.id,
+        field.wraps,
+    );
+    // Wrapped, as wide as the pane, which the layouter wraps at.
+    let width = if field.wraps {
+        ui.available_width()
+    } else {
+        f32::INFINITY
+    };
     let output = egui::TextEdit::multiline(&mut sql_tab.text)
         .id(field.id)
         .font(font)
         // Tab indents; Esc gives the keys up unless `hold_escape`.
         .lock_focus(true)
         .frame(egui::Frame::new().inner_margin(margin))
-        .desired_width(f32::INFINITY)
+        .desired_width(width)
         .desired_rows(rows as usize)
         .layouter(&mut layouter)
         .show(ui);
```

- [ ] **Step 4: Run the tests to see them pass**

Run: `cargo test --locked --lib -- typing_settings_tests sql_text::tests ui::tests ui::complete_tests`
Expected: PASS (`ui::tests::a_press_on_the_gutter_gives_the_editor_the_keys_on_that_line` among them: a press under the last line is still the script's end). Then the four checks above.

- [ ] **Step 5: Look at it by eye**

Not a test, and nothing of it is committed. Append a throwaway `#[test] #[ignore]` scene to `src/shots.rs` that sets `harness.app.settings.wrap = true`, connects the fixture, opens a SQL editor with a line longer than the pane and two short ones, and shoots it in `Harness::for_shots(egui::vec2(1200.0, 700.0), 1.0, true, look)` for `Look::macos()` and `Look::omarchy()`. Run it with `cargo test --locked --features shots --lib shots::<name> -- --ignored`: the long line's number stands once, on its first row, and the statement's bar runs down all its rows. Remove the scene before committing.

- [ ] **Step 6: Commit**

```bash
git add src/ui/sql_text.rs src/ui/typing_settings_tests.rs
git commit -m "Wrap long lines in the SQL editor as Settings › Editor says, numbered once"
```

---

### Task 3: The Close brackets and Wrap rows

**Files:**
- Modify: `src/settings.rs`, `src/model.rs`
- Modify (tests): `src/ui/typing_settings_tests.rs`, `src/ui/editor_settings_tests.rs`

- [ ] **Step 1: Write the failing tests**

```diff
diff --git a/src/ui/editor_settings_tests.rs b/src/ui/editor_settings_tests.rs
--- a/src/ui/editor_settings_tests.rs
+++ b/src/ui/editor_settings_tests.rs
@@ -528,9 +528,7 @@ fn the_sheet_sets_each_editor_option_with_a_click() {
         assert!(shows(&mut harness, words), "{words}");
     }
     // The rows of what is not built yet are not there.
-    for words in ["Keybindings", "Wrap", "Close brackets and quotes"] {
-        assert!(!shows(&mut harness, words), "{words}");
-    }
+    assert!(!shows(&mut harness, "Keybindings"));
     let saves = settings_saves(&harness);
     harness.click("New tabs run in: Follow connection");
     harness.click("Row limit");
@@ -659,8 +657,8 @@ fn the_screen_sets_each_editor_option_with_its_keys() {
     type_key(&mut harness, Key::J, "j");
     harness.press(Key::Space, Modifiers::NONE);
     // Line numbers from relative to absolute; four spaces; completion on
-    // the key; keywords from the look's lower case to upper; leading
-    // commas, with space.
+    // the key; no closing brackets and wrapped lines, with space; keywords
+    // from the look's lower case to upper; leading commas, with space.
     type_key(&mut harness, Key::J, "j");
     type_key(&mut harness, Key::L, "l");
     type_key(&mut harness, Key::J, "j");
@@ -668,6 +666,10 @@ fn the_screen_sets_each_editor_option_with_its_keys() {
     type_key(&mut harness, Key::J, "j");
     type_key(&mut harness, Key::L, "l");
     type_key(&mut harness, Key::J, "j");
+    harness.press(Key::Space, Modifiers::NONE);
+    type_key(&mut harness, Key::J, "j");
+    harness.press(Key::Space, Modifiers::NONE);
+    type_key(&mut harness, Key::J, "j");
     type_key(&mut harness, Key::H, "h");
     type_key(&mut harness, Key::J, "j");
     harness.press(Key::Space, Modifiers::NONE);
@@ -682,7 +684,9 @@ fn the_screen_sets_each_editor_option_with_its_keys() {
     assert_eq!(settings.autocomplete, Autocomplete::Manual);
     assert_eq!(settings.keywords, Some(KeywordCase::Upper));
     assert_eq!(settings.commas, crate::settings::Commas::Leading);
-    assert_eq!(settings_saves(&harness), saves + 9);
+    assert!(!settings.close_pairs);
+    assert!(settings.wrap);
+    assert_eq!(settings_saves(&harness), saves + 11);
     // The file has each of them, and R puts the cursor's back.
     let text = harness.app.settings_file.text.clone();
     assert!(
```

```diff
diff --git a/src/ui/typing_settings_tests.rs b/src/ui/typing_settings_tests.rs
--- a/src/ui/typing_settings_tests.rs
+++ b/src/ui/typing_settings_tests.rs
@@ -5,11 +5,13 @@
 use egui::accesskit::Role;
 use egui::{Key, Modifiers};
 
-use crate::model::{Action, ConnTabId};
+use crate::model::{Action, ConnTabId, SettingsPane};
 use crate::settings::OptionValue;
 use crate::testing::{Harness, bounds};
 use crate::theme::Look;
 
+use super::appearance_settings_tests::{settings_saves, shows};
+
 /// A connected fixture in `look`, with a new SQL editor that has the
 /// keyboard and holds `text`, its cursor at the end.
 fn editor(look: Look, text: &str) -> (Harness, ConnTabId) {
@@ -367,3 +369,59 @@ fn the_completion_list_hangs_under_a_word_that_starts_a_wrapped_row() {
     }
     assert!(started > 0, "no length put the word at a row's start");
 }
+
+/// Settings on its Editor pane.
+fn on_the_pane(harness: &mut Harness) {
+    harness.app.apply(Action::ShowSettings);
+    harness
+        .app
+        .apply(Action::ShowSettingsPane(SettingsPane::Editor));
+    harness.settle();
+}
+
+#[test]
+fn the_sheet_has_the_close_pairs_and_wrap_rows_and_sets_them() {
+    let mut harness = Harness::with_size(egui::vec2(1440.0, 1400.0));
+    harness.set_look(Look::macos());
+    on_the_pane(&mut harness);
+    // As the board orders them: after Autocomplete, and Wrap last.
+    let top = |harness: &Harness, words: &str| harness.painted_rect(words).expect(words).top();
+    harness.settle();
+    assert!(top(&harness, "Autocomplete") < top(&harness, "Close brackets and quotes"));
+    assert!(top(&harness, "Close brackets and quotes") < top(&harness, "Line numbers"));
+    assert!(top(&harness, "Current line highlight") < top(&harness, "Wrap"));
+    let saves = settings_saves(&harness);
+    harness.click("Close brackets and quotes");
+    harness.click("Wrap");
+    harness.settle();
+    assert!(!harness.app.settings.close_pairs);
+    assert!(harness.app.settings.wrap);
+    assert_eq!(settings_saves(&harness), saves + 2);
+}
+
+#[test]
+fn the_screen_has_the_close_brackets_and_wrap_rows() {
+    let mut harness = Harness::with_size(egui::vec2(1920.0, 1080.0));
+    harness.set_look(Look::omarchy());
+    on_the_pane(&mut harness);
+    for words in ["close brackets", "wrap"] {
+        assert!(shows(&mut harness, words), "{words}");
+    }
+    let top = |harness: &Harness, words: &str| harness.painted_rect(words).expect(words).top();
+    assert!(top(&harness, "autocomplete") < top(&harness, "close brackets"));
+    assert!(top(&harness, "close brackets") < top(&harness, "wrap"));
+    // On and off until set, as each row's box says.
+    let mark = |harness: &Harness, words: &str| {
+        let row = harness.painted_rect(words).expect(words);
+        harness
+            .text_rects
+            .iter()
+            .find(|(text, rect)| {
+                (text == "[x]" || text == "[ ]") && (rect.center().y - row.center().y).abs() < 4.0
+            })
+            .map(|(text, _)| text.clone())
+            .expect("the row's box")
+    };
+    assert_eq!(mark(&harness, "close brackets"), "[x]");
+    assert_eq!(mark(&harness, "wrap"), "[ ]");
+}
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test --locked --lib -- typing_settings_tests editor_settings_tests settings`
Expected: FAIL. Three tests fail: `the_sheet_has_the_close_pairs_and_wrap_rows_and_sets_them` and `the_screen_has_the_close_brackets_and_wrap_rows` find no such rows, and `the_screen_sets_each_editor_option_with_its_keys` steps the rows as they were (keywords where close brackets will be).

- [ ] **Step 3: Show the rows**

```diff
diff --git a/src/model.rs b/src/model.rs
--- a/src/model.rs
+++ b/src/model.rs
@@ -3002,6 +3002,8 @@ impl SettingsPane {
                 SettingsRow::Option(OptionId::LineNumbers),
                 SettingsRow::Option(OptionId::Indent),
                 SettingsRow::Option(OptionId::Autocomplete),
+                SettingsRow::Option(OptionId::ClosePairs),
+                SettingsRow::Option(OptionId::Wrap),
                 SettingsRow::Option(OptionId::Keywords),
                 SettingsRow::Option(OptionId::Commas),
                 SettingsRow::Option(OptionId::HistoryKeep),
```

```diff
diff --git a/src/settings.rs b/src/settings.rs
--- a/src/settings.rs
+++ b/src/settings.rs
@@ -1076,27 +1076,29 @@ impl OptionId {
     ];
     /// The Editor pane's options whose features are not built yet: no row
     /// draws them, and their keys are read, kept and reset.
-    pub const EDITOR_LATER: [OptionId; 4] = [
-        Self::ConfirmUnsafeWrites,
-        Self::EditorKeys,
-        Self::ClosePairs,
-        Self::Wrap,
-    ];
+    pub const EDITOR_LATER: [OptionId; 2] = [Self::ConfirmUnsafeWrites, Self::EditorKeys];
     /// The Editor pane's Running queries section, as far as its options
     /// are built.
     pub const RUNNING: [OptionId; 4] =
         [Self::NewTab, Self::RowLimit, Self::Timeout, Self::RunScope];
     /// Its Typing section on macOS and Windows.
-    pub const TYPING: [OptionId; 4] = [
+    pub const TYPING: [OptionId; 6] = [
         Self::Indent,
         Self::Autocomplete,
+        Self::ClosePairs,
         Self::LineNumbers,
         Self::HighlightLine,
+        Self::Wrap,
     ];
     /// The terminal look's, as its board orders them, with no row for the
     /// cursor's line.
-    pub const TERMINAL_TYPING: [OptionId; 3] =
-        [Self::LineNumbers, Self::Indent, Self::Autocomplete];
+    pub const TERMINAL_TYPING: [OptionId; 5] = [
+        Self::LineNumbers,
+        Self::Indent,
+        Self::Autocomplete,
+        Self::ClosePairs,
+        Self::Wrap,
+    ];
     /// Its Format section, in both looks.
     pub const FORMAT: [OptionId; 2] = [Self::Keywords, Self::Commas];
     /// Its History section, in both looks.
```

- [ ] **Step 4: Run the tests to see them pass**

Run: `cargo test --locked --lib -- typing_settings_tests editor_settings_tests settings ui::settings_access_tests`
Expected: PASS. Then the four checks above.

- [ ] **Step 5: Compare with the boards by eye**

Not a test, and nothing of it is committed. Shoot Settings › Editor in both looks (`Harness::for_shots`, 1440 × 1100 on macOS and 1920 × 1080 on Omarchy) and set it beside the two "Settings, Editor" boards' Typing rows. Remove the scene before committing.

- [ ] **Step 6: Commit**

```bash
git add src/model.rs src/settings.rs src/ui/editor_settings_tests.rs src/ui/typing_settings_tests.rs
git commit -m "Show Settings › Editor's Close brackets and Wrap rows in both looks"
```

---

### Task 4: The website

**Files:**
- Modify: `docs/_reference/settings-and-files.md`, `docs/_guide/sql-editor.md`

- [ ] **Step 1: Say what the rows do**

```diff
diff --git a/docs/_guide/sql-editor.md b/docs/_guide/sql-editor.md
--- a/docs/_guide/sql-editor.md
+++ b/docs/_guide/sql-editor.md
@@ -95,6 +95,11 @@ start of every line a selection touches when there is one (blank lines
 stay blank), and Shift+Tab takes one off. While the completion list is
 open, Tab accepts its row first.
 
+A bracket or a quote you type gets its closing one, which you type over,
+or delete with Backspace while the pair is empty. Typed over a selection,
+the two go round it. **Settings › Editor** turns this off. There, Wrap
+wraps long lines at the editor's width, each numbered once.
+
 ## Completion
 
 Keywords, schemas, tables, views and the columns of the tables in your
```

```diff
diff --git a/docs/_reference/settings-and-files.md b/docs/_reference/settings-and-files.md
--- a/docs/_reference/settings-and-files.md
+++ b/docs/_reference/settings-and-files.md
@@ -54,8 +54,10 @@ open SQL editor follows each change at once.
 | **⌘↩ runs** (Omarchy: ctrl+enter runs) | Statement at cursor, or Whole tab: every statement in order, stopping at the first error. A selection always runs what it selects, whichever is set | Statement at cursor |
 | **Indent** | 2 spaces, 4 spaces or Tab: what Tab puts in at the cursor, or at the start of every line a selection touches, and Shift+Tab takes off them; and how far Format sets a `CASE`'s `WHEN` and `ELSE` in (four for Tab: the layout is set in spaces) | 2 spaces |
 | **Autocomplete** | As you type, On Ctrl+Space (the list opens only when you ask), or Off. Keywords are offered in the case **Keywords** sets, upper case with Keep | As you type |
+| **Close brackets and quotes** (Omarchy: close brackets) | On or off. On, `(`, `[`, `{` and a quote (and a backtick, which quotes names on MySQL and SQLite) get their closing one after the cursor where nothing follows that it would split off. A quote is not paired inside a word, right after the same quote, or inside a string, a quoted name or a comment. A closing one typed before its twin steps over it, and Backspace takes both out while the pair is empty. Typed over a selection, the two go round it, which stays selected and undoes in one step. A paste, an input method's text, or two characters arriving at once go in as they come | On |
 | **Line numbers** | On or off on macOS and Windows; relative, absolute or off in the Omarchy look. On writes `absolute`; a `relative` set in the file shows as on and stays | On (absolute) on macOS and Windows, relative in the Omarchy look |
 | **Current line highlight** (macOS and Windows) | On or off: the line the cursor is on is filled while the editor has the keyboard. The Omarchy look has no row: it follows the key, set in the file | On |
+| **Wrap** | On or off. On, a line longer than the editor wraps at its width and the editor scrolls down alone. The line keeps one number, on its first row, and a click in the gutter beside any of its rows puts the cursor at its start. Off, long lines scroll sideways | Off |
 | **Keywords** (in Format · Cmd/Ctrl+Shift+F; Omarchy: format · `=` in normal mode) | UPPERCASE, lowercase or Keep: the case Format writes reserved words in. Keep leaves them as you typed them. Names, strings and comments never change | UPPERCASE on macOS and Windows, lowercase in the Omarchy look |
 | **Commas** | Trailing (at the end of an item's line) or Leading (at the start of the next item's, on the edge of the river), in the lists Format breaks: a `SELECT`'s and a `WITH`'s. A comma with a comment after it, on its line or the next, stays at the end of its line | Trailing |
 | **Keep query history** (Omarchy: keep) | The last 100, 1,000 or 10,000 statements of each connection, on this computer, or Don't keep: every statement run from the SQL editor, a grid's save, a filter or sort you set, and agents' runs, as the [History tab]({% link _guide/sql-editor.md %}#history) shows them. Results are never kept. Don't keep records nothing more, and asks whether to delete what is kept (Omarchy: `D` deletes, `n` keeps). **Show History** beside it (Omarchy: Enter) shows the window's History tab | Last 1,000 |
@@ -177,10 +179,10 @@ bar = "tint"
 | `editor.typing.keys` | `"standard"` or `"vim"`. Read and kept; not acted on yet | Not set: standard on macOS and Windows, vim in the Omarchy look |
 | `editor.typing.indent` | `2`, `4` or `"tab"` | `2` |
 | `editor.typing.autocomplete` | `"typing"`, `"manual"` or `"off"` | `"typing"` |
-| `editor.typing.close_pairs` | `true` or `false`. Read and kept; not acted on yet | `true` |
+| `editor.typing.close_pairs` | `true` or `false` | `true` |
 | `editor.typing.line_numbers` | `"absolute"`, `"relative"` or `"off"` | Not set: absolute on macOS and Windows, relative in the Omarchy look |
 | `editor.typing.highlight_line` | `true` or `false` | `true` |
-| `editor.typing.wrap` | `true` or `false`. Read and kept; not acted on yet | `false` |
+| `editor.typing.wrap` | `true` or `false` | `false` |
 | `editor.format.keywords` | `"upper"`, `"lower"` or `"keep"` | Not set: upper on macOS and Windows, lower in the Omarchy look |
 | `editor.format.commas` | `"trailing"` or `"leading"` | `"trailing"` |
 | `editor.history.keep` | 0 to 100000 statements kept for each connection, from every source; `0` keeps none. A smaller number deletes the oldest past it | `1000` |
```

- [ ] **Step 2: Build and check the site**

Run: `cd docs && bin/check` where Ruby and the bundle are installed; otherwise leave it to `docs.yml` on the pull request.
Expected: the build and its checks pass.

- [ ] **Step 3: Commit**

```bash
git add docs/_guide/sql-editor.md docs/_reference/settings-and-files.md
git commit -m "Describe Settings › Editor's Close brackets and Wrap on the website"
```

---

### Task 5: The pull request

- [ ] Push the branch and open one pull request: a summary, the rows built (Keybindings and the write confirmation are what is left hidden), the departures above, and what each look does by default. No keys added. No screenshot.
