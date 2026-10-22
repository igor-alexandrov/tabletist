# Settings › Editor, step 7: Vim Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the Keybindings setting step 1 added: with Vim (the Omarchy look's default) the SQL editor has vim's normal, insert and visual modes, the mode in the status line (the macOS footer), `:w` doing nothing, and `=`, `==`, `gg=G` formatting. Switching applies at once, and a tab in insert mode goes back to normal mode. Show the row in both looks.

**Architecture:** A vim engine of its own, `src/vim.rs`, with no egui in it: it reads a key with the script and the cursor (character indices, as egui's field counts them), and returns the edit, the cursor or the visual selection, and what the editor does next (format lines, open the prompt, leave). It keeps vim's undo history itself: egui's counts cursor moves. Each SQL tab holds its `Vim` (its mode, a command typed in part, its history). In the editor (`src/ui/sql_text.rs`), the keymap still reads the frame first; then, before egui's text edit, `vim_keys` reads the frame's events in order, takes normal and visual mode's keys out and feeds them to the engine. Its edits go in as outside edits, a visual selection becomes the field's selection (so Run runs it), and Esc is held by the focus filter so that only normal mode's Esc gives the keyboard up. A key of vim's after one the field reads, or after a format, waits for the next frame, where only `vim_keys` reads it (the keymap read that frame already). Out of insert mode the cursor is a block and egui's caret is hidden. The status line and the macOS footer read the tab's mode.

**Tech Stack:** Rust, egui (crmne fork) with AccessKit, headless tests through `src/testing.rs`.

**Spec:** `docs/superpowers/specs/2026-10-19-settings-editor-design.md`, sections "Steps" item 7 and "Step 7: Vim".

---

## Before Task 1

Commit the spec's step 7 section and this plan:

```bash
git add docs/superpowers/specs/2026-10-19-settings-editor-design.md docs/superpowers/plans/2026-10-21-settings-editor-7-vim.md
git commit -m "Plan Settings › Editor step 7: vim"
```

## Where this sits

Steps 1 to 5 are merged (#181, #183, #204, #210, #212); the query history (#197) built step 6. This plan is a branch of `main` at `9b78a554`. Step 8 (the preview) comes after it.

## Decisions and departures, for a yes or a no before code

You chose two things before the draft: core vim (below), and that Esc in normal mode leaves the editor.

| Asked, or the boards | Built | Why |
|---|---|---|
| "Normal, insert and visual modes" (no more said) | Core vim, as you chose: counts; `h j k l w b e W B E 0 ^ $ gg G f F t T ; , % { }`; `i a I A o O`; `x X D C Y s S p P J r ~`; `d c y > < =` with a motion or doubled; `u`, Ctrl+R; `v`, `V` with `o` and the operators. Not: text objects, `.`, search, registers, marks, macros | Your choice |
| (where a new tab starts) | In insert mode; each tab keeps its own mode after; the keyboard coming back (a click, Ctrl+K) finds the mode it left | A new tab is opened to type in, as the board's `-- INSERT --` shows |
| Esc (no more said) | In insert mode, normal mode (an open completion list takes it first); in normal mode with nothing typed, the keyboard leaves the editor, as Esc did before; with a command typed in part, the command ends | Your choice |
| Omarchy's status line: `-- INSERT --` (the board) | `-- INSERT --`, `-- VISUAL --`, `-- VISUAL LINE --`, and nothing in normal mode, at the line's start; the Esc hint says "normal" in insert mode | As vim's own line |
| macOS: "the mode in the status line" | The footer starts with the mode | macOS has no status line |
| `:` | Opens Omarchy's command line (`:ro`, `:rw`); the editor gets the keyboard back when it closes with Enter or Esc, while its tab is in front. On macOS `:` does nothing | macOS has no command line |
| A result card's letters (`w` allow writes, `e`, `a`, `t`) | Read with the keyboard out of the editor, as before: from insert mode that is two Escs | In normal mode the letters are vim's (`w` is a motion) |
| (the field's chords in normal mode) | Copy, paste, select all and the like are the field's in every mode (a paste in normal mode goes in at the cursor; a cut or a paste ends visual mode); Ctrl+W, Ctrl+H, Ctrl+U, the field's other deleting chords (Ctrl+K, Ctrl, Alt or Cmd with Backspace or Delete) and Tab do nothing in normal mode; Ctrl+[ is Esc | They are the field's own; vim's insert-mode deletions have nothing to delete in normal mode, and Tab moves nothing in vim |
| (undo) | Vim's own history: a command is a step, an insert from the key that started it to Esc is one, and so is a change from outside (a paste, Format); moves are none, and keep the redo. Cmd/Ctrl+Z is still the field's own history, which counts typing in pauses | egui's history makes a step of a cursor that stays put a second, and drops the redo on any move |
| "`=` in normal mode" (the board's format section) | With the keyboard in the editor `=` waits for a motion (`==` the statement, `gg=G` the script, `=j`, or over a visual selection); with the keyboard out of the text, `=` formats as before | It is vim's operator |
| (paging, the error marks) | Page Up and Page Down are the field's in every mode; vim's Ctrl+F, Ctrl+B, Ctrl+D, Ctrl+U are not there. `]e` and `[e` work with the keyboard out of the editor, as before | Not in the core you chose; in normal mode the letters are vim's |
| (the mode's colours) | Omarchy: `-- INSERT --` green and visual magenta, as the grid's modes; macOS: the accent | The look's own mode colours |
| (a selection in normal mode) | A drag or select all makes the field's selection in normal mode too, which Run runs, as with the standard keys | The pointer and the field's chords select as anywhere |
| (the register) | One, shared by the SQL editors, not the system clipboard: `y` does not copy to the clipboard | Vim's unnamed register |
| (the pointer) | A press leaves visual mode; a drag selects as the field does | The field selects with the pointer in every look |
| (the block cursor) | The cell editor's (`block_cursor_at`), tinted, over the character; in visual mode on the cursor's end | What the terminal look draws for a cursor already |

A reviewer given the draft broke it with tests of its own (17 defects, each a failing test), and the draft now fixes them all; the tests are in it:

- Keys that arrived in one frame across a mode change went astray: `A` then `x` dropped the `x`, and Esc then `x` typed it. The frame's events are read in order now, each in the mode the ones before left, and a key of vim's after one the field reads, or after an undo, a redo or a format, waits for the next frame (`keys_in_one_frame_are_read_in_the_mode_the_ones_before_left`).
- `t` and `T` skipped a character right beside the cursor (the skip is `;`'s alone), so `ct)` in `count(|a)` did nothing and what was typed after ran as commands.
- Tab in normal mode typed an indent; it does nothing now.
- `dw` and `yw` on a line's last word took the line break when the next line is indented; the last word's end ends the text now, as in vim.
- `D` and `C` on an empty line took its line break; a count before `>>` and `<<` shifted that many times; `j` after an insert or an edit kept a column from before it.
- Undo after a visual edit put the visual selection back in normal mode, which Run would then run; it puts the cursor back now.
- A cut or a paste in visual mode left visual mode on, over a stale range.
- A held Esc in insert mode left the editor.
- Smaller: `Vp` on the last line added a line, `d}` in the last paragraph left its last character, `X` at a line's start emptied the register, `r` and Enter did nothing, `I` on a line of blanks went before the last, a put of characters over lines left the cursor at its end, and the status line said "esc leave editor" in visual mode.

From the reviewer's argument, not a test: an input method's text in normal mode is read as keys now, and Ctrl+[ is Esc.

It found working: text with `é` and emoji, a multi-character text event in normal mode, a press leaving visual mode, Esc in the prompt giving the keyboard back, and no panic over 47 edges (an empty script, unmatched brackets, `99999G`, `100000x`, CRLF).

A second reviewer read the plan against the spec, your request and the code. What it found is fixed:

- Closing the `:` prompt in any way (a click on the grid, Ctrl+J, another tab) pulled the keyboard back to the editor. Only Enter and Esc give it back now, and only while that SQL tab is in front.
- `=` left the lines it formats selected in normal mode where Format had nothing to change, which Run would then run, and its undo put the selection back; `==` on an empty line formatted the whole script. The cursor stays now, and an empty line formats nothing.
- egui's undo history makes a step of a cursor that stays put for a second and drops the redo on any move, so `u` could take a step that only moved the cursor, and `u`, `j`, Ctrl+R redid nothing (the headless tests run in less than a second and could not see it). Vim keeps its own history now, in the engine: a command a step, an insert from its start to Esc one, a change from outside one; moves none (`undo_takes_back_a_change_or_an_insert_whole_and_redo_survives_moves`).
- The field's deleting chords (Ctrl+K, Ctrl/Alt/Cmd+Backspace and Delete) deleted in normal mode; they do nothing there now.
- Keys in the frame after Esc out of the editor or after `:` still went to the engine; they are nobody's now.
- A press elsewhere in visual mode left its selection in normal mode; it goes.
- Missing departures (above): `=` waiting for a motion, paging, the error marks, the mode's colours, a selection in normal mode; the block cursor is the cell editor's own now, not a copy. Task 2's test runs name every test file it changes, and the website says more exactly what undo does, what visual `x` and `s` do, and what Ctrl+W does in normal mode.

For the record: a key of vim's held back a frame is read by the editor alone, not by the keymap (Ctrl+Enter in the same frame as `==` is the field's); it takes two keys within one frame.

## Files

| File | What it does after this plan |
|---|---|
| `src/vim.rs` | New: the engine (`Vim`, `Mode`, `Input`, `Register`, `Step`, `Then`, `clamp`) and its unit tests. |
| `src/lib.rs` | Declares it. |
| `src/ui/sql_text.rs` | `vim_input`, `vim_keys`, `vim_range`: the frame's keys to the engine before the field, the block cursor, Esc held; `Field::vim`; `:` asks for the prompt; `format_script` keeps vim's cursor. |
| `src/ui/cell_editor.rs` | `block_cursor_at`, the block at a given character, which `block_cursor` and the SQL editor use. |
| `src/model.rs` | `SqlTab::vim`, `Workspace::command_returns`, `Action::EditorCommand` and `Action::EscapeCommand`; the Omarchy screen's Keybindings row. |
| `src/app.rs`, `src/app/editing.rs` | The prompt opened from the editor gives the keyboard back (`command_closed`); `:w` in a SQL editor does nothing; a change of keybindings puts every tab in normal mode. |
| `src/ui/workspace.rs`, `src/ui/sql_editor.rs` | The mode at the start of Omarchy's status line and of the macOS footer (`sql_editor::vim_mode`). |
| `src/settings.rs`, `src/ui/settings/terminal.rs` | Keybindings first in `TYPING` and `TERMINAL_TYPING`; `EDITOR_LATER` goes; the Omarchy hints. |
| `src/testing.rs` | `Harness::leave_editor`, `start_typing`, `vim_inserting`. |
| `src/ui/vim_tests.rs` | New: this step's headless tests. |
| `src/ui/mod.rs` and the tests that leave an editor | Declare the tests; leave the editor with `leave_editor` (two Escs from vim's insert mode). |
| `docs/_reference/settings-and-files.md`, `docs/_guide/sql-editor.md`, `docs/_guide/omarchy.md` | The row, its key, the Vim section, the Omarchy keys. |

Every task ends with the repository's checks passing:

    cargo fmt --all --check
    cargo clippy --locked --workspace --all-targets -- -D warnings
    cargo test --locked --workspace --all-targets
    RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps

(`cargo` is `~/.cargo/bin/cargo` in these sessions.) Commit after each task, signed as usual. If signing says "agent refused operation", 1Password is locked: leave the change staged and say so.

---

### Task 1: The vim engine

**Files:**
- Create: `src/vim.rs`
- Modify: `src/lib.rs` (declares it; the tests are in it)

- [ ] **Step 1: Write the failing tests**

```diff
diff --git a/src/lib.rs b/src/lib.rs
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -37,4 +37,5 @@ pub mod theme;
 pub mod typography;
 pub mod ui;
 pub mod util;
+pub mod vim;
 pub mod zones;
```

```diff
diff --git a/src/vim.rs b/src/vim.rs
new file mode 100644
--- /dev/null
+++ b/src/vim.rs
@@ -0,0 +1,366 @@
+//! Vim's normal, insert and visual modes for the SQL editor, as
+//! Settings › Editor's Keybindings sets them.
+//!
+//! The engine knows nothing of egui: it reads a key with the script and
+//! the cursor, and says what changes. The editor feeds it the keys of
+//! normal and visual mode, and Esc in insert mode; insert mode's typing is
+//! the text field's own. Places are character indices, as the field
+//! counts them.
+//!
+//! What it has: counts; the motions `h j k l w b e W B E 0 ^ $ gg G f F t
+//! T ; , % { }`; the ways into insert mode `i a I A o O`; `x X D C Y s S p
+//! P J r ~`; the operators `d c y > < =` with a motion, or doubled for
+//! lines; undo `u` and redo Ctrl+R; visual mode by character `v` and by
+//! line `V`, which the same motions move and the operators act on. `:`
+//! opens the editor's command line, and Esc with nothing typed leaves the
+//! editor. Not: text objects, `.`, registers other than the one, search,
+//! marks, macros.
+
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+
+    /// Feeds `keys` to vim in normal mode over `text`, whose `|` marks the
+    /// cursor, and gives the script back with the cursor marked, the mode
+    /// and the last `Then`. In `keys`, `<esc>`, `<cr>`, `<bs>` and `<c-r>`
+    /// name keys.
+    fn run(text: &str, keys: &str) -> (String, Mode, Option<Then>) {
+        let (text, _, mode, then) = run_with(text, keys, Register::default());
+        (text, mode, then)
+    }
+
+    fn run_with(
+        text: &str,
+        keys: &str,
+        mut register: Register,
+    ) -> (String, Register, Mode, Option<Then>) {
+        let at = text.chars().position(|c| c == '|').expect("a cursor");
+        let mut script: String = text.chars().filter(|c| *c != '|').collect();
+        let mut cursor = at;
+        let mut vim = Vim::default();
+        let mut then = None;
+        let mut rest = keys;
+        while !rest.is_empty() {
+            let (input, len) = [
+                ("<esc>", Input::Escape),
+                ("<cr>", Input::Enter),
+                ("<bs>", Input::Backspace),
+                ("<c-r>", Input::Redo),
+            ]
+            .iter()
+            .find(|(name, _)| rest.starts_with(name))
+            .map(|(name, input)| (*input, name.len()))
+            .unwrap_or_else(|| {
+                let c = rest.chars().next().unwrap();
+                (Input::Char(c), c.len_utf8())
+            });
+            rest = &rest[len..];
+            if vim.mode == Mode::Insert && input != Input::Escape {
+                // Insert mode's typing is the field's.
+                if let Input::Char(c) = input {
+                    let byte = script
+                        .char_indices()
+                        .nth(cursor)
+                        .map_or(script.len(), |(b, _)| b);
+                    script.insert(byte, c);
+                    cursor += 1;
+                }
+                continue;
+            }
+            let step = vim.key(&script, cursor, input, &mut register, "  ");
+            if let Some(text) = step.text {
+                script = text;
+            }
+            cursor = step.cursor;
+            if step.then.is_some() {
+                then = step.then;
+            }
+        }
+        let byte = script
+            .char_indices()
+            .nth(cursor)
+            .map_or(script.len(), |(b, _)| b);
+        script.insert(byte, '|');
+        (script, register, vim.mode, then)
+    }
+
+    fn text(text: &str, keys: &str) -> String {
+        run(text, keys).0
+    }
+
+    #[test]
+    fn the_motions_move_along_a_line_and_across_lines() {
+        assert_eq!(text("|select id from books", "w"), "select |id from books");
+        assert_eq!(text("|select id from books", "3w"), "select id from |books");
+        assert_eq!(text("select id from |books", "b"), "select id |from books");
+        assert_eq!(text("|select id", "e"), "selec|t id");
+        assert_eq!(text("|count(*) as n", "w"), "count|(*) as n");
+        assert_eq!(text("|count(*) as n", "W"), "count(*) |as n");
+        assert_eq!(text("se|lect", "$"), "selec|t");
+        assert_eq!(text("  se|lect", "0"), "|  select");
+        assert_eq!(text("  se|lect", "^"), "  |select");
+        assert_eq!(text("|ab", "l"), "a|b");
+        assert_eq!(text("a|b", "l"), "a|b", "not past the last character");
+        assert_eq!(text("|a\nb\nc", "j"), "a\n|b\nc");
+        assert_eq!(text("|a\nb\nc", "G"), "a\nb\n|c");
+        assert_eq!(text("a\nb\n|c", "gg"), "|a\nb\nc");
+        assert_eq!(text("a\nb\n|c", "2G"), "a\n|b\nc");
+        // j and k keep the column, and `$` the end.
+        assert_eq!(text("abc|d\nx\nabcdef", "jj"), "abcd\nx\nabc|def");
+        assert_eq!(text("ab|c\nabcdef", "$j"), "abc\nabcde|f");
+        // A word across a line break, and an empty line is a word.
+        assert_eq!(text("|select\nfrom", "w"), "select\n|from");
+        assert_eq!(text("|a\n\nb", "w"), "a\n|\nb");
+        assert_eq!(text("a\n\n|b", "b"), "a\n|\nb");
+    }
+
+    #[test]
+    fn find_and_match_jump_on_the_line() {
+        assert_eq!(
+            text("|where a = 1 and b = 2", "f="),
+            "where a |= 1 and b = 2"
+        );
+        assert_eq!(
+            text("|where a = 1 and b = 2", "2f="),
+            "where a = 1 and b |= 2"
+        );
+        assert_eq!(
+            text("|where a = 1 and b = 2", "f=;"),
+            "where a = 1 and b |= 2"
+        );
+        assert_eq!(text("|where a = 1", "t="), "where a| = 1");
+        assert_eq!(text("where a = |1", "F="), "where a |= 1");
+        assert_eq!(text("|count(max(a))", "%"), "count(max(a)|)");
+        assert_eq!(text("count(max(a)|)", "%"), "count|(max(a))");
+    }
+
+    #[test]
+    fn delete_change_and_yank_take_what_their_motion_covers() {
+        assert_eq!(text("|select id from books", "dw"), "|id from books");
+        assert_eq!(text("select |id, title", "d2w"), "select |title");
+        assert_eq!(text("select |id, title", "2dw"), "select |title");
+        assert_eq!(
+            text("select |id\nfrom", "dw"),
+            "select| \nfrom",
+            "the line break stays"
+        );
+        assert_eq!(text("select |id from", "d$"), "select| ");
+        assert_eq!(text("select |id from", "D"), "select| ");
+        assert_eq!(text("where a |= 1", "dt1"), "where a |1");
+        assert_eq!(text("where a |= 1", "df1"), "where a| ");
+        // `cw` is `ce` on a word.
+        assert_eq!(
+            run("select |id from", "cwname<esc>"),
+            ("select nam|e from".into(), Mode::Normal, None)
+        );
+        assert_eq!(text("select |id from", "c$*<esc>"), "select |*");
+        assert_eq!(text("|abc", "x"), "|bc");
+        assert_eq!(text("ab|c", "x"), "a|b");
+        assert_eq!(text("ab|c", "X"), "a|c");
+        assert_eq!(text("|abc", "3x"), "|");
+        assert_eq!(text("|abc", "sz<esc>"), "|zbc");
+        // `cw` on a word of one character changes it alone.
+        assert_eq!(text("select |2;", "cw3<esc>"), "select |3;");
+        assert_eq!(text("|a b c", "c2wx<esc>"), "|x c");
+    }
+
+    #[test]
+    fn a_doubled_operator_acts_on_whole_lines() {
+        assert_eq!(text("a\n|b\nc", "dd"), "a\n|c");
+        assert_eq!(
+            text("a\nb\n|c", "dd"),
+            "a\n|b",
+            "the last line takes the break before it"
+        );
+        assert_eq!(text("|a\nb\nc", "2dd"), "|c");
+        assert_eq!(text("|a\nb\nc", "dj"), "|c");
+        assert_eq!(text("a\nb\n|c", "dk"), "|a");
+        assert_eq!(text("  |a\nb", "ccx<esc>"), "  |x\nb", "the indent stays");
+        assert_eq!(text("|a\nb", ">>"), "  |a\nb");
+        assert_eq!(text("|a\nb", ">j"), "  |a\n  b");
+        assert_eq!(text("    |a", "<<"), "  |a");
+        assert_eq!(text("\t|a", "<<"), "|a");
+    }
+
+    #[test]
+    fn a_yank_or_a_delete_puts_back_where_p_says() {
+        assert_eq!(text("|a\nb", "yyp"), "a\n|a\nb");
+        assert_eq!(text("a\n|b", "yyp"), "a\nb\n|b", "after the last line");
+        assert_eq!(text("a\n|b", "yyP"), "a\n|b\nb");
+        assert_eq!(text("|a\nb", "ddp"), "b\n|a");
+        assert_eq!(text("|select id", "yw$p"), "select idselect| ");
+        assert_eq!(text("|ab", "xp"), "b|a");
+        assert_eq!(text("|ab", "x3p"), "baa|a");
+        assert_eq!(text("|ab", "yl$P"), "a|ab");
+    }
+
+    #[test]
+    fn insert_mode_is_entered_where_each_key_says_and_left_with_esc() {
+        assert_eq!(run("se|lect", "i"), ("se|lect".into(), Mode::Insert, None));
+        assert_eq!(
+            text("se|lect", "ix<esc>"),
+            "se|xlect",
+            "Esc steps back over it"
+        );
+        assert_eq!(text("se|lect", "ax<esc>"), "sel|xect");
+        assert_eq!(text("  se|lect", "Ix<esc>"), "  |xselect");
+        assert_eq!(text("se|lect", "Ax<esc>"), "select|x");
+        assert_eq!(text("  se|lect\nfrom", "ox<esc>"), "  select\n  |x\nfrom");
+        assert_eq!(text("se|lect", "Ox<esc>"), "|x\nselect");
+        assert_eq!(text("|", "ix<esc>"), "|x");
+        assert_eq!(text("|", "i<esc>"), "|");
+    }
+
+    #[test]
+    fn join_replace_and_case_change_the_text_in_place() {
+        assert_eq!(text("|select\n  id", "J"), "select| id");
+        assert_eq!(text("|a\nb\nc", "3J"), "a b| c");
+        assert_eq!(text("|count(\n  )", "J"), "count(|)");
+        assert_eq!(text("|abc", "rx"), "|xbc");
+        assert_eq!(text("|abc", "2rx"), "x|xc");
+        assert_eq!(text("|abc", "5rx"), "|abc", "not past the line");
+        assert_eq!(text("|select", "3~"), "SEL|ect");
+    }
+
+    #[test]
+    fn visual_mode_selects_both_ends_and_operates_on_them() {
+        let (_, mode, _) = run("|select", "v");
+        assert_eq!(mode, Mode::Visual);
+        assert_eq!(text("|select id", "vd"), "|elect id");
+        assert_eq!(text("|select id", "ved"), "| id");
+        assert_eq!(text("|select id", "veyP"), "selec|tselect id");
+        assert_eq!(text("select |id", "vbd"), "|d");
+        assert_eq!(text("a\n|b\nc", "Vd"), "a\n|c");
+        assert_eq!(text("|a\nb\nc", "Vjd"), "|c");
+        assert_eq!(text("|a\nb", "Vj>"), "  |a\n  b");
+        assert_eq!(text("|select id", "veU"), "|SELECT id");
+        assert_eq!(text("|select id", "vecx<esc>"), "|x id");
+        assert_eq!(text("|a\nb", "VjJ"), "a| b");
+        // `o` moves the other end; Esc and v leave.
+        assert_eq!(text("se|lect", "vlohd"), "s|ct");
+        assert_eq!(run("|select", "v<esc>").1, Mode::Normal);
+        assert_eq!(run("|select", "vv").1, Mode::Normal);
+        assert_eq!(run("|select", "vV").1, Mode::VisualLine);
+        // What is selected is replaced by what was yanked.
+        assert_eq!(text("|ab cd", "yewvep"), "ab a|b");
+    }
+
+    #[test]
+    fn the_editor_is_asked_for_undo_redo_format_the_prompt_and_leaving() {
+        assert_eq!(run("|a", ":").2, Some(Then::Prompt));
+        assert_eq!(run("|a", "<esc>").2, Some(Then::Leave));
+        // Esc ends a command typed in part, and the keyboard stays.
+        assert_eq!(run("|a", "d<esc>").2, None);
+        assert_eq!(run("|a", "2<esc>").2, None);
+        // `==` the cursor's line, `gg=G` every line, `=j` two.
+        assert_eq!(run("a\n|bc\nd", "==").2, Some(Then::Format(2..4)));
+        assert_eq!(run("a\n|bc\nd", "gg=G").2, Some(Then::Format(0..6)));
+        assert_eq!(run("a\n|bc\nd", "=j").2, Some(Then::Format(2..6)));
+        assert_eq!(run("|a\nbc\nd", "Vj=").2, Some(Then::Format(0..4)));
+    }
+
+    #[test]
+    fn counts_multiply_and_an_unknown_key_drops_the_command() {
+        assert_eq!(text("|a b c d e f g", "2d2w"), "|e f g");
+        assert_eq!(
+            text("|a b c d e f g e f g h i j k", "10w"),
+            "a b c d e f g e f g |h i j k"
+        );
+        assert_eq!(text("|a b", "dqw"), "a |b", "`q` dropped the `d`");
+        assert_eq!(text("|", "x"), "|");
+        assert_eq!(text("|", "dd"), "|");
+        assert_eq!(text("|", "p"), "|");
+    }
+
+    #[test]
+    fn the_edges_a_reviewer_found_are_as_vim_has_them() {
+        // `t` and `T` find a character right beside the cursor; `;` does
+        // not stop where `t` stood.
+        assert_eq!(text("|a=b=c", "dt="), "|=b=c");
+        assert_eq!(text("count(|a)", "ct)x<esc>"), "count(|x)");
+        assert_eq!(text("|a=b=c", "t=;"), "a=|b=c");
+        // `dw` and `yw` on a line's last word stop at its end.
+        assert_eq!(text("a |b\n  c", "dw"), "a| \n  c");
+        let (_, register, _, _) = run_with("a |b\n  c", "yw", Register::default());
+        assert_eq!(register.text, "b");
+        // `D` and `C` on an empty line keep its break.
+        assert_eq!(text("a\n|\nb", "D"), "a\n|\nb");
+        assert_eq!(text("a\n|\nb", "Cx<esc>"), "a\n|x\nb");
+        // A count before `>>` is of lines.
+        assert_eq!(text("|a\nb\nc", "2>>"), "  |a\n  b\nc");
+        assert_eq!(text("    |a\n    b\nc", "2<<"), "  |a\n  b\nc");
+        // `j` after an insert starts from where the insert left it.
+        assert_eq!(
+            text("|abcdefghij\nabcdefghij", "llAx<esc>j"),
+            "abcdefghijx\nabcdefghi|j"
+        );
+        assert_eq!(text("|a\nb", "yyjVp"), "a\n|a");
+        assert_eq!(text("|a b", "d}"), "|");
+        assert_eq!(text("|a b", "}"), "a |b");
+        assert_eq!(text("|ab", "ylXp"), "a|ab");
+        assert_eq!(text("a|bc", "r<cr>"), "a\n|c");
+        assert_eq!(text("|  \nb", "Ix<esc>"), "  |x\nb");
+        let register = Register {
+            text: "x\ny".into(),
+            lines: false,
+        };
+        assert_eq!(run_with("|ab", "p", register).0, "a|x\nyb");
+    }
+
+    #[test]
+    fn undo_takes_back_a_change_or_an_insert_whole_and_redo_survives_moves() {
+        // A command each, and counts.
+        assert_eq!(text("|a b c", "xxu"), "| b c");
+        assert_eq!(text("|a b c", "xx2u"), "|a b c");
+        assert_eq!(text("|a b c", "xxuu<c-r>"), "| b c");
+        assert_eq!(text("|a b c", "xx2u2<c-r>"), "|b c");
+        // An insert, with the change that started it, is one step.
+        assert_eq!(text("|a b", "cwxyz<esc>u"), "|a b");
+        assert_eq!(text("|a", "oselect 1<esc>u"), "|a");
+        // Moves are no steps, and leave the redo.
+        assert_eq!(text("|ab\ncd", "xujl<c-r>"), "|b\ncd");
+        // Nothing to undo: nothing changes.
+        assert_eq!(text("a|b", "u"), "a|b");
+        // A new change leaves nothing to redo.
+        assert_eq!(text("|abc", "xu~<c-r>"), "A|bc");
+        // `==` on an empty line formats nothing.
+        assert_eq!(run("a\n|\nb", "==").2, None);
+    }
+
+    #[test]
+    fn normal_mode_keeps_the_cursor_on_a_character() {
+        assert_eq!(clamp("abc", 3), 2);
+        assert_eq!(clamp("abc\n", 3), 2);
+        assert_eq!(clamp("abc\n", 4), 4, "the empty last line");
+        assert_eq!(clamp("", 0), 0);
+    }
+
+    #[test]
+    fn the_register_holds_lines_or_characters() {
+        let (_, register, _, _) = run_with("|a\nb", "yy", Register::default());
+        assert_eq!(
+            register,
+            Register {
+                text: "a\n".into(),
+                lines: true
+            }
+        );
+        let (_, register, _, _) = run_with("a\n|b", "dd", Register::default());
+        assert_eq!(
+            register,
+            Register {
+                text: "b\n".into(),
+                lines: true
+            }
+        );
+        let (_, register, _, _) = run_with("|ab", "yl", Register::default());
+        assert_eq!(
+            register,
+            Register {
+                text: "a".into(),
+                lines: false
+            }
+        );
+    }
+}
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test --locked --lib vim::tests`
Expected: FAIL to compile (40 errors): `vim` has no `Vim`, `Register`, `Input`, `clamp` or the rest yet.

- [ ] **Step 3: Write the engine**

```diff
diff --git a/src/vim.rs b/src/vim.rs
--- a/src/vim.rs
+++ b/src/vim.rs
@@ -16,6 +16,1509 @@
 //! editor. Not: text objects, `.`, registers other than the one, search,
 //! marks, macros.
 
+use std::ops::Range;
+
+/// Where the keys go.
+#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
+pub enum Mode {
+    /// Keys are commands; the cursor sits on a character.
+    #[default]
+    Normal,
+    /// Keys type, as in the standard keybindings.
+    Insert,
+    /// A selection by character, from where `v` was typed to the cursor,
+    /// both ends in.
+    Visual,
+    /// A selection of whole lines, from the line `V` was typed on to the
+    /// cursor's.
+    VisualLine,
+}
+
+impl Mode {
+    /// What the status line says, as vim's: nothing in normal mode.
+    pub fn label(self) -> Option<&'static str> {
+        match self {
+            Mode::Normal => None,
+            Mode::Insert => Some("-- INSERT --"),
+            Mode::Visual => Some("-- VISUAL --"),
+            Mode::VisualLine => Some("-- VISUAL LINE --"),
+        }
+    }
+
+    pub fn is_visual(self) -> bool {
+        matches!(self, Mode::Visual | Mode::VisualLine)
+    }
+}
+
+/// A key as the engine reads it: a character typed, or a key that types
+/// none.
+#[derive(Clone, Copy, Debug, PartialEq, Eq)]
+pub enum Input {
+    Char(char),
+    Escape,
+    Enter,
+    Backspace,
+    Delete,
+    Left,
+    Right,
+    Up,
+    Down,
+    Home,
+    End,
+    /// Ctrl+R.
+    Redo,
+}
+
+/// What a delete or a yank keeps and `p` puts back: vim's unnamed
+/// register. Whole lines end with a line break.
+#[derive(Clone, Debug, Default, PartialEq, Eq)]
+pub struct Register {
+    pub text: String,
+    pub lines: bool,
+}
+
+/// What the editor does after a key, besides taking the text and the
+/// cursor the step gives.
+#[derive(Clone, Debug, PartialEq, Eq)]
+pub enum Then {
+    /// Format the statements these characters (whole lines) touch; all of
+    /// them where the range is the whole script.
+    Format(Range<usize>),
+    /// Open the command line, as `:` does with the keyboard out of a text
+    /// field.
+    Prompt,
+    /// Esc with nothing typed: the keyboard leaves the editor.
+    Leave,
+}
+
+/// What a key does.
+#[derive(Clone, Debug, PartialEq, Eq)]
+pub struct Step {
+    /// The script after the key, where it changed.
+    pub text: Option<String>,
+    /// The cursor.
+    pub cursor: usize,
+    /// In visual mode, the selection's other end; the selection covers
+    /// both ends' characters (by line, whole lines).
+    pub anchor: Option<usize>,
+    pub then: Option<Then>,
+}
+
+/// One editor's vim: its mode and the command typed so far.
+#[derive(Clone, Debug, Default, PartialEq, Eq)]
+pub struct Vim {
+    pub mode: Mode,
+    /// Where visual mode started.
+    anchor: usize,
+    /// The cursor in visual mode: the field's selection runs past it.
+    head: usize,
+    /// The column `j` and `k` keep to (`usize::MAX` after `$`), with the
+    /// cursor it was kept at: it holds while the cursor is there, and an
+    /// edit, an insert or a click that moves it starts from its own.
+    column: Option<(usize, usize)>,
+    /// The last `f`, `F`, `t` or `T`, for `;` and `,`.
+    find: Option<(char, char)>,
+    pending: Pending,
+    history: History,
+}
+
+/// Vim's undo history: the script before each change, with the cursor to
+/// put back. A change is a command of normal or visual mode, an insert
+/// from the key that started it to Esc, or what changed from outside
+/// between two keys (a paste, Format). Cursor moves are none.
+#[derive(Clone, Default, PartialEq, Eq)]
+struct History {
+    undo: Vec<(String, usize)>,
+    redo: Vec<(String, usize)>,
+    /// The script as the last change left it, and the cursor then.
+    seen: Option<(String, usize)>,
+}
+
+// The scripts stay out of logs, as `SqlTab`'s do.
+impl std::fmt::Debug for History {
+    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
+        f.debug_struct("History")
+            .field("undo", &self.undo.len())
+            .field("redo", &self.redo.len())
+            .finish_non_exhaustive()
+    }
+}
+
+impl History {
+    /// The most steps kept.
+    const KEPT: usize = 1000;
+
+    /// `before` (a script and its cursor) as a step to undo to; nothing
+    /// is left to redo.
+    fn push(&mut self, before: (String, usize)) {
+        self.undo.push(before);
+        if self.undo.len() > Self::KEPT {
+            self.undo.remove(0);
+        }
+        self.redo.clear();
+    }
+
+    /// The script is `text` now: where it is not what the last change
+    /// left, what changed since is a step of its own.
+    fn settle(&mut self, text: &str, cursor: usize) {
+        if let Some(seen) = self.seen.take()
+            && seen.0 != text
+        {
+            self.push(seen);
+        }
+        self.seen = Some((text.to_owned(), cursor));
+    }
+}
+
+/// A command typed in part.
+#[derive(Clone, Debug, Default, PartialEq, Eq)]
+struct Pending {
+    count: Option<usize>,
+    /// The operator and the count typed before it.
+    operator: Option<(Operator, usize)>,
+    /// `g` typed, waiting for the second `g`.
+    g: bool,
+    /// `f`, `F`, `t`, `T` or `r` typed, waiting for its character.
+    wait: Option<char>,
+}
+
+#[derive(Clone, Copy, Debug, PartialEq, Eq)]
+enum Operator {
+    Delete,
+    Change,
+    Yank,
+    Indent,
+    Outdent,
+    Format,
+}
+
+impl Operator {
+    fn of(key: char) -> Option<Operator> {
+        Some(match key {
+            'd' => Operator::Delete,
+            'c' => Operator::Change,
+            'y' => Operator::Yank,
+            '>' => Operator::Indent,
+            '<' => Operator::Outdent,
+            '=' => Operator::Format,
+            _ => return None,
+        })
+    }
+}
+
+/// How a motion's range is taken.
+#[derive(Clone, Copy, Debug, PartialEq, Eq)]
+enum Kind {
+    /// Up to the target, not it.
+    Exclusive,
+    /// Up to the target and it.
+    Inclusive,
+    /// The lines from the cursor's to the target's.
+    Lines,
+}
+
+/// Where a motion goes.
+#[derive(Clone, Copy, Debug, PartialEq, Eq)]
+struct Target {
+    to: usize,
+    kind: Kind,
+    /// The column `j` and `k` keep to after it: kept where `None`.
+    column: Option<usize>,
+}
+
+impl Vim {
+    /// Vim in `mode`, as an editor opens: with an empty script, which
+    /// undo goes back to.
+    pub fn new(mode: Mode) -> Self {
+        Self {
+            mode,
+            history: History {
+                seen: Some((String::new(), 0)),
+                ..History::default()
+            },
+            ..Self::default()
+        }
+    }
+
+    /// Whether a command is typed in part: Esc then ends it, and the
+    /// keyboard stays.
+    pub fn is_pending(&self) -> bool {
+        self.pending != Pending::default()
+    }
+
+    /// Reads `input` with `text` and the cursor at `cursor`. `indent` is
+    /// what `>` puts in at the start of a line.
+    pub fn key(
+        &mut self,
+        text: &str,
+        cursor: usize,
+        input: Input,
+        register: &mut Register,
+        indent: &str,
+    ) -> Step {
+        let chars: Vec<char> = text.chars().collect();
+        let t = Text { c: &chars };
+        let cursor = cursor.min(t.len());
+        let mut step = Step {
+            text: None,
+            cursor,
+            anchor: None,
+            then: None,
+        };
+        // Out of insert mode, what changed since the last key is a step of
+        // its own; in it, the insert is one, settled by Esc.
+        let undo_point = match self.history.seen.as_ref() {
+            Some((seen, at)) if self.mode == Mode::Insert => (seen.clone(), *at),
+            _ => {
+                self.history.settle(text, cursor);
+                (text.to_owned(), cursor)
+            }
+        };
+        let was = self.mode;
+        match self.mode {
+            Mode::Insert => {
+                if input == Input::Escape {
+                    // Back over the last character typed, as vim does.
+                    self.mode = Mode::Normal;
+                    let start = t.line_start(cursor);
+                    step.cursor = t.clamp(cursor.saturating_sub(1).max(start));
+                    self.history.seen = Some(undo_point.clone());
+                    self.history.settle(text, step.cursor);
+                }
+            }
+            Mode::Normal => {
+                // Undo and redo, which are the history's own.
+                let count = self.pending.count.unwrap_or(1);
+                let undo = input == Input::Char('u')
+                    && self.pending.operator.is_none()
+                    && !self.pending.g
+                    && self.pending.wait.is_none();
+                if undo || input == Input::Redo {
+                    self.pending = Pending::default();
+                    self.history_steps(&t, count, undo, &mut step);
+                    return step;
+                }
+                self.normal(&t, cursor, input, register, indent, &mut step);
+            }
+            Mode::Visual | Mode::VisualLine => {
+                self.visual(&t, cursor, input, register, indent, &mut step);
+            }
+        }
+        if let Some(new) = &step.text {
+            if self.mode == Mode::Insert {
+                // A change that goes on in insert mode: one step with the
+                // typing, from before it.
+                self.history.seen = Some(undo_point);
+            } else if was != Mode::Insert {
+                self.history.push(undo_point);
+                self.history.seen = Some((new.clone(), step.cursor));
+            }
+        }
+        if self.mode.is_visual() {
+            step.anchor = Some(self.anchor);
+            self.head = step.cursor;
+        }
+        step
+    }
+
+    /// `u` (or with `undo` false, Ctrl+R) `count` times: the script and
+    /// the cursor of a step of the history, the one it leaves kept for
+    /// the other way.
+    fn history_steps(&mut self, t: &Text<'_>, count: usize, undo: bool, step: &mut Step) {
+        let mut now = (t.slice(0..t.len()), step.cursor);
+        let mut moved = false;
+        for _ in 0..count {
+            let (from, to) = if undo {
+                (&mut self.history.undo, &mut self.history.redo)
+            } else {
+                (&mut self.history.redo, &mut self.history.undo)
+            };
+            let Some(then) = from.pop() else {
+                break;
+            };
+            to.push(std::mem::replace(&mut now, then));
+            moved = true;
+        }
+        if !moved {
+            return;
+        }
+        let chars: Vec<char> = now.0.chars().collect();
+        step.cursor = Text { c: &chars }.clamp(now.1);
+        self.history.seen = Some((now.0.clone(), step.cursor));
+        step.text = Some(now.0);
+    }
+
+    /// The cursor in visual mode, which the field's selection does not
+    /// say: it runs one past it.
+    pub fn head(&self) -> usize {
+        self.head
+    }
+
+    /// Leaves visual mode for normal, or insert mode for normal (where
+    /// the keybindings change, or the editor gives the keyboard up).
+    pub fn to_normal(&mut self) {
+        self.mode = Mode::Normal;
+        self.pending = Pending::default();
+    }
+
+    fn normal(
+        &mut self,
+        t: &Text<'_>,
+        cursor: usize,
+        input: Input,
+        register: &mut Register,
+        indent: &str,
+        step: &mut Step,
+    ) {
+        let pending = std::mem::take(&mut self.pending);
+        // A character a command waits for.
+        if let Some(wait) = pending.wait {
+            let c = match input {
+                Input::Char(c) => c,
+                // `r` and Enter: a line break.
+                Input::Enter if wait == 'r' => '\n',
+                _ => return,
+            };
+            let count = pending.count.unwrap_or(1);
+            if wait == 'r' {
+                replace(t, cursor, c, count, step);
+                return;
+            }
+            self.find = Some((wait, c));
+            let target = find(t, cursor, wait, c, count, false);
+            self.finish_motion(t, cursor, target, pending, register, indent, step);
+            return;
+        }
+        let key = match input {
+            Input::Char(c) => c,
+            Input::Escape => {
+                // Esc ends a command typed in part; with none it leaves.
+                if pending == Pending::default() {
+                    step.then = Some(Then::Leave);
+                }
+                return;
+            }
+            // Read as the history's, before this.
+            Input::Redo => return,
+            Input::Enter => '+',
+            Input::Backspace | Input::Left => 'h',
+            Input::Right => 'l',
+            Input::Up => 'k',
+            Input::Down => 'j',
+            Input::Home => '0',
+            Input::End => '$',
+            Input::Delete => 'x',
+        };
+        // A count: digits, and 0 only after one.
+        if key.is_ascii_digit() && (key != '0' || pending.count.is_some()) && !pending.g {
+            let digit = key.to_digit(10).unwrap_or(0) as usize;
+            let count = pending
+                .count
+                .unwrap_or(0)
+                .saturating_mul(10)
+                .saturating_add(digit);
+            self.pending = Pending {
+                count: Some(count.min(100_000)),
+                ..pending
+            };
+            return;
+        }
+        if pending.g {
+            if key == 'g' {
+                let target = Some(line_target(t, pending.count.unwrap_or(1)));
+                self.finish_motion(t, cursor, target, pending, register, indent, step);
+            }
+            return;
+        }
+        if let Some((operator, _)) = pending.operator {
+            // A doubled operator acts on lines: `dd`, `cc`, `yy`, `>>`.
+            if Operator::of(key) == Some(operator) {
+                let count = total(&pending);
+                let last = t.line_of(cursor) + count - 1;
+                let to = t.line_at(last.min(t.lines() - 1));
+                let target = Target {
+                    to,
+                    kind: Kind::Lines,
+                    column: None,
+                };
+                self.finish_motion(t, cursor, Some(target), pending, register, indent, step);
+                return;
+            }
+        }
+        match key {
+            'g' => {
+                self.pending = Pending { g: true, ..pending };
+                return;
+            }
+            'f' | 'F' | 't' | 'T' => {
+                self.pending = Pending {
+                    wait: Some(key),
+                    ..pending
+                };
+                return;
+            }
+            'r' if pending.operator.is_none() => {
+                self.pending = Pending {
+                    wait: Some('r'),
+                    ..pending
+                };
+                return;
+            }
+            _ => {}
+        }
+        if let Some(operator) = Operator::of(key)
+            && pending.operator.is_none()
+        {
+            self.pending = Pending {
+                count: None,
+                operator: Some((operator, pending.count.unwrap_or(1))),
+                ..Pending::default()
+            };
+            return;
+        }
+        let count = pending.count.unwrap_or(1);
+        if let Some(target) = self.motion(t, cursor, key, &pending) {
+            self.finish_motion(t, cursor, Some(target), pending, register, indent, step);
+            return;
+        }
+        if pending.operator.is_some() {
+            // An operator with no motion after it: dropped.
+            return;
+        }
+        let (start, end) = (t.line_start(cursor), t.line_end(cursor));
+        match key {
+            ':' => step.then = Some(Then::Prompt),
+            'i' => self.mode = Mode::Insert,
+            'a' => {
+                self.mode = Mode::Insert;
+                step.cursor = (cursor + 1).min(end);
+            }
+            'I' => {
+                self.mode = Mode::Insert;
+                // On a line of blanks, after them.
+                let at = t.first_non_blank(cursor);
+                let blank = t.c.get(at).is_some_and(|c| *c == ' ' || *c == '\t');
+                step.cursor = if blank { t.line_end(cursor) } else { at };
+            }
+            'A' => {
+                self.mode = Mode::Insert;
+                step.cursor = end;
+            }
+            'o' | 'O' => {
+                self.mode = Mode::Insert;
+                let lead: String = t.c[start..t.first_non_blank(cursor)].iter().collect();
+                let (at, put) = if key == 'o' {
+                    (end, format!("\n{lead}"))
+                } else {
+                    (start, format!("{lead}\n"))
+                };
+                let to = if key == 'o' {
+                    end + put.chars().count()
+                } else {
+                    start + lead.chars().count()
+                };
+                splice(t, at..at, &put, to, step);
+            }
+            'v' | 'V' => {
+                self.mode = if key == 'v' {
+                    Mode::Visual
+                } else {
+                    Mode::VisualLine
+                };
+                self.anchor = cursor;
+            }
+            'x' | 'X' | 's' => {
+                if start == end {
+                    if key == 's' {
+                        self.mode = Mode::Insert;
+                    }
+                    return;
+                }
+                let range = if key == 'X' {
+                    cursor.saturating_sub(count).max(start)..cursor
+                } else {
+                    cursor..(cursor + count).min(end)
+                };
+                // `X` at a line's start: nothing to take, and the register
+                // keeps what it has.
+                if range.is_empty() {
+                    return;
+                }
+                *register = Register {
+                    text: t.slice(range.clone()),
+                    lines: false,
+                };
+                let to = range.start;
+                splice(t, range, "", to, step);
+                if key == 's' {
+                    self.mode = Mode::Insert;
+                } else {
+                    step.cursor = clamp_in(step, to);
+                }
+            }
+            'D' | 'C' | 'Y' | 'S' => {
+                // `d$`, `c$`, `yy` and `cc`, as vim reads them.
+                let (operator, motion) = match key {
+                    'D' => (Operator::Delete, '$'),
+                    'C' => (Operator::Change, '$'),
+                    'Y' => (Operator::Yank, 'y'),
+                    _ => (Operator::Change, 'c'),
+                };
+                self.pending = Pending {
+                    operator: Some((operator, 1)),
+                    ..Pending::default()
+                };
+                self.pending.count = pending.count;
+                self.normal(t, cursor, Input::Char(motion), register, indent, step);
+            }
+            'p' | 'P' => put(t, cursor, key == 'p', count, register, step),
+            'J' => join(t, t.line_of(cursor), count.max(2) - 1, step),
+            '~' => {
+                if start == end {
+                    return;
+                }
+                let range = cursor..(cursor + count).min(end);
+                let flipped: String = t.c[range.clone()].iter().map(|c| flip(*c)).collect();
+                let to = range.end.min(end - 1);
+                splice(t, range, &flipped, to, step);
+            }
+            _ => {}
+        }
+    }
+
+    fn visual(
+        &mut self,
+        t: &Text<'_>,
+        cursor: usize,
+        input: Input,
+        register: &mut Register,
+        indent: &str,
+        step: &mut Step,
+    ) {
+        let pending = std::mem::take(&mut self.pending);
+        if let Some(wait) = pending.wait {
+            if let Input::Char(c) = input {
+                self.find = Some((wait, c));
+                if let Some(target) = find(t, cursor, wait, c, pending.count.unwrap_or(1), false) {
+                    self.moved(t, target, step);
+                }
+            }
+            return;
+        }
+        let key = match input {
+            Input::Char(c) => c,
+            Input::Escape => {
+                self.mode = Mode::Normal;
+                step.cursor = t.clamp(cursor);
+                return;
+            }
+            Input::Enter => '+',
+            Input::Backspace | Input::Left => 'h',
+            Input::Right => 'l',
+            Input::Up => 'k',
+            Input::Down => 'j',
+            Input::Home => '0',
+            Input::End => '$',
+            Input::Delete => 'x',
+            Input::Redo => return,
+        };
+        if key.is_ascii_digit() && (key != '0' || pending.count.is_some()) && !pending.g {
+            let digit = key.to_digit(10).unwrap_or(0) as usize;
+            let count = pending
+                .count
+                .unwrap_or(0)
+                .saturating_mul(10)
+                .saturating_add(digit);
+            self.pending = Pending {
+                count: Some(count.min(100_000)),
+                ..pending
+            };
+            return;
+        }
+        if pending.g {
+            if key == 'g' {
+                let target = line_target(t, pending.count.unwrap_or(1));
+                self.moved(t, target, step);
+            }
+            return;
+        }
+        match key {
+            'g' => {
+                self.pending = Pending { g: true, ..pending };
+                return;
+            }
+            'f' | 'F' | 't' | 'T' => {
+                self.pending = Pending {
+                    wait: Some(key),
+                    ..pending
+                };
+                return;
+            }
+            _ => {}
+        }
+        if let Some(target) = self.motion(t, cursor, key, &pending) {
+            self.moved(t, target, step);
+            return;
+        }
+        let lines = self.mode == Mode::VisualLine;
+        let (low, high) = (self.anchor.min(cursor), self.anchor.max(cursor));
+        let range = if lines {
+            t.line_start(low)..(t.line_end(high) + 1).min(t.len())
+        } else {
+            low..(high + 1).min(t.len())
+        };
+        match key {
+            'v' | 'V' => {
+                let mode = if key == 'v' {
+                    Mode::Visual
+                } else {
+                    Mode::VisualLine
+                };
+                self.mode = if self.mode == mode {
+                    Mode::Normal
+                } else {
+                    mode
+                };
+                step.cursor = t.clamp(cursor);
+            }
+            'o' => {
+                step.cursor = self.anchor;
+                self.anchor = cursor;
+            }
+            ':' => step.then = Some(Then::Prompt),
+            'd' | 'x' | 'c' | 's' | 'y' | '>' | '<' | '=' => {
+                let operator = match key {
+                    'd' | 'x' => Operator::Delete,
+                    'c' | 's' => Operator::Change,
+                    'y' => Operator::Yank,
+                    '>' => Operator::Indent,
+                    '<' => Operator::Outdent,
+                    _ => Operator::Format,
+                };
+                self.mode = Mode::Normal;
+                let count = pending.count.unwrap_or(1);
+                self.operate(
+                    t, cursor, operator, range, lines, count, register, indent, step,
+                );
+            }
+            'J' => {
+                self.mode = Mode::Normal;
+                let (first, last) = (t.line_of(low), t.line_of(high));
+                join(t, first, (last - first).max(1), step);
+            }
+            '~' | 'u' | 'U' => {
+                self.mode = Mode::Normal;
+                let changed: String = t.c[range.clone()]
+                    .iter()
+                    .map(|c| match key {
+                        '~' => flip(*c),
+                        'u' => c.to_lowercase().next().unwrap_or(*c),
+                        _ => c.to_uppercase().next().unwrap_or(*c),
+                    })
+                    .collect();
+                let to = if lines { range.start } else { low };
+                splice(t, range, &changed, to, step);
+                step.cursor = clamp_in(step, to);
+            }
+            'p' | 'P' => {
+                self.mode = Mode::Normal;
+                // The selection goes; what was yanked goes in its place.
+                let put = register.clone();
+                let start = range.start;
+                *register = Register {
+                    text: t.slice(range.clone()),
+                    lines,
+                };
+                let mut put_text = put.text.clone();
+                if put.lines && !lines {
+                    put_text = format!("\n{put_text}");
+                } else if lines && !put.lines {
+                    put_text.push('\n');
+                }
+                // The last line has no break after it, and gets none.
+                let last = range.end == t.len() && t.c.last() != Some(&'\n');
+                if lines && last && put_text.ends_with('\n') {
+                    put_text.pop();
+                }
+                // On the last character put, or a line's first.
+                let length = put_text.chars().count();
+                let to = if put.lines || lines {
+                    start
+                } else {
+                    start + length.saturating_sub(1)
+                };
+                splice(t, range, &put_text, to, step);
+                step.cursor = clamp_in(step, to);
+            }
+            _ => self.pending = Pending::default(),
+        }
+    }
+
+    /// A motion in visual mode: the cursor moves, the anchor stays.
+    fn moved(&mut self, t: &Text<'_>, target: Target, step: &mut Step) {
+        step.cursor = t.clamp(target.to);
+        self.column = target.column.map(|column| (step.cursor, column));
+    }
+
+    /// The column `j` and `k` keep to from `cursor`, on a line that starts
+    /// at `start`.
+    fn kept_column(&self, cursor: usize, start: usize) -> usize {
+        match self.column {
+            Some((at, column)) if at == cursor => column,
+            _ => cursor - start,
+        }
+    }
+
+    /// The motion `key` from `cursor`, where it is one.
+    fn motion(&self, t: &Text<'_>, cursor: usize, key: char, pending: &Pending) -> Option<Target> {
+        let count = total(pending);
+        let (start, end) = (t.line_start(cursor), t.line_end(cursor));
+        let along = |to: usize, kind| Target {
+            to,
+            kind,
+            column: Some(to - t.line_start(to)),
+        };
+        // `cw` on a word changes to its end, not the blanks after it: from
+        // the end of this word, as `e` goes on from there.
+        let change = matches!(pending.operator, Some((Operator::Change, _)));
+        if change && matches!(key, 'w' | 'W') && t.c.get(cursor).is_some_and(|c| !c.is_whitespace())
+        {
+            let big = key == 'W';
+            let own = class(t.c[cursor], big);
+            let mut to = cursor;
+            while to + 1 < t.len() && class(t.c[to + 1], big) == own {
+                to += 1;
+            }
+            for _ in 1..count {
+                to = word_end(t, to, big);
+            }
+            return Some(along(to, Kind::Inclusive));
+        }
+        Some(match key {
+            'h' => along(cursor.saturating_sub(count).max(start), Kind::Exclusive),
+            'l' | ' ' => along((cursor + count).min(end), Kind::Exclusive),
+            '0' => along(start, Kind::Exclusive),
+            '^' => along(t.first_non_blank(cursor), Kind::Exclusive),
+            '$' => {
+                let last = (t.line_of(cursor) + count - 1).min(t.lines() - 1);
+                let line_end = t.line_end(t.line_at(last));
+                Target {
+                    to: line_end.saturating_sub(1).max(t.line_at(last)),
+                    kind: Kind::Inclusive,
+                    column: Some(usize::MAX),
+                }
+            }
+            'j' | 'k' | '+' | '-' => {
+                let line = t.line_of(cursor);
+                let to_line = if matches!(key, 'j' | '+') {
+                    (line + count).min(t.lines() - 1)
+                } else {
+                    line.saturating_sub(count)
+                };
+                if to_line == line {
+                    return None;
+                }
+                let first = t.line_at(to_line);
+                let to = if matches!(key, '+' | '-') {
+                    t.first_non_blank(first)
+                } else {
+                    let column = self.kept_column(cursor, start);
+                    let line_end = t.line_end(first);
+                    first
+                        .saturating_add(column)
+                        .min(line_end.saturating_sub(1).max(first))
+                };
+                Target {
+                    to,
+                    kind: Kind::Lines,
+                    column: Some(self.kept_column(cursor, start)),
+                }
+            }
+            'G' => {
+                let line = pending.count.unwrap_or(t.lines());
+                line_target(t, line)
+            }
+            'w' | 'W' => {
+                let (mut to, mut last) = (cursor, cursor);
+                for _ in 0..count {
+                    last = to;
+                    to = word_start(t, to, key == 'W');
+                }
+                // After an operator, the last word moved over ends the
+                // text at its line's end, not at the next line's first
+                // word, as in vim.
+                let end = t.line_end(last);
+                if pending.operator.is_some() && t.line_of(to) > t.line_of(last) && end > last {
+                    to = end;
+                }
+                along(to.min(t.len()), Kind::Exclusive)
+            }
+            'e' | 'E' => {
+                let mut to = cursor;
+                for _ in 0..count {
+                    to = word_end(t, to, key == 'E');
+                }
+                along(to, Kind::Inclusive)
+            }
+            'b' | 'B' => {
+                let mut to = cursor;
+                for _ in 0..count {
+                    to = word_back(t, to, key == 'B');
+                }
+                along(to, Kind::Exclusive)
+            }
+            ';' | ',' => {
+                let (kind, c) = self.find?;
+                let kind = if key == ',' {
+                    match kind {
+                        'f' => 'F',
+                        'F' => 'f',
+                        't' => 'T',
+                        _ => 't',
+                    }
+                } else {
+                    kind
+                };
+                return find(t, cursor, kind, c, count, true);
+            }
+            '%' => along(matching(t, cursor)?, Kind::Inclusive),
+            '}' => {
+                let mut to = cursor;
+                for _ in 0..count {
+                    to = paragraph(t, to, true);
+                }
+                along(to, Kind::Exclusive)
+            }
+            '{' => {
+                let mut to = cursor;
+                for _ in 0..count {
+                    to = paragraph(t, to, false);
+                }
+                along(to, Kind::Exclusive)
+            }
+            _ => return None,
+        })
+    }
+
+    /// A motion typed, alone or after an operator.
+    #[allow(clippy::too_many_arguments)]
+    fn finish_motion(
+        &mut self,
+        t: &Text<'_>,
+        cursor: usize,
+        target: Option<Target>,
+        pending: Pending,
+        register: &mut Register,
+        indent: &str,
+        step: &mut Step,
+    ) {
+        let Some(target) = target else {
+            return;
+        };
+        let Some((operator, _)) = pending.operator else {
+            step.cursor = t.clamp(target.to);
+            self.column = target.column.map(|column| (step.cursor, column));
+            return;
+        };
+        let (low, high) = (cursor.min(target.to), cursor.max(target.to));
+        let (range, lines) = match target.kind {
+            Kind::Exclusive => {
+                // An exclusive motion that ends at the start of a later
+                // line stops at the end of the line before, as in vim:
+                // `dw` on a line's last word leaves the line break.
+                let mut high = high;
+                if high > low && t.column(high) == 0 && t.line_of(high) > t.line_of(low) {
+                    high -= 1;
+                }
+                (low..high, false)
+            }
+            // Up to a line break, not it: `D` on an empty line takes
+            // nothing.
+            Kind::Inclusive if t.c.get(high) == Some(&'\n') => (low..high, false),
+            Kind::Inclusive => (low..(high + 1).min(t.len()), false),
+            Kind::Lines => (t.line_start(low)..(t.line_end(high) + 1).min(t.len()), true),
+        };
+        // A count before `>>` is of lines, which the range has: one shift.
+        let count = match operator {
+            Operator::Indent | Operator::Outdent => 1,
+            _ => total(&pending),
+        };
+        self.operate(
+            t, cursor, operator, range, lines, count, register, indent, step,
+        );
+    }
+
+    /// `operator` over `range` (whole lines where `lines`).
+    #[allow(clippy::too_many_arguments)]
+    fn operate(
+        &mut self,
+        t: &Text<'_>,
+        cursor: usize,
+        operator: Operator,
+        range: Range<usize>,
+        lines: bool,
+        count: usize,
+        register: &mut Register,
+        indent: &str,
+        step: &mut Step,
+    ) {
+        let first = range.start;
+        match operator {
+            Operator::Delete => {
+                let mut taken = t.slice(range.clone());
+                let mut range = range;
+                if lines {
+                    if !taken.ends_with('\n') {
+                        taken.push('\n');
+                        // The last line has no break after it: the one
+                        // before it goes.
+                        if range.start > 0 {
+                            range.start -= 1;
+                        }
+                    }
+                } else if range.is_empty() {
+                    return;
+                }
+                *register = Register { text: taken, lines };
+                let at = range.start;
+                splice(t, range, "", at, step);
+                let new: Vec<char> = step.text.as_deref().unwrap_or("").chars().collect();
+                let n = Text { c: &new };
+                step.cursor = if lines {
+                    n.first_non_blank(n.line_start(first.min(n.len())))
+                } else {
+                    n.clamp(at)
+                };
+            }
+            Operator::Change => {
+                self.mode = Mode::Insert;
+                if lines {
+                    // The lines' text goes; the first line's indent stays.
+                    let mut taken = t.slice(range.clone());
+                    if !taken.ends_with('\n') {
+                        taken.push('\n');
+                    }
+                    *register = Register {
+                        text: taken,
+                        lines: true,
+                    };
+                    let lead: String = t.c[range.start..t.first_non_blank(range.start)]
+                        .iter()
+                        .collect();
+                    let end = t.line_end(range.end.saturating_sub(1).max(range.start));
+                    let to = range.start + lead.chars().count();
+                    splice(t, range.start..end, &lead, to, step);
+                } else {
+                    *register = Register {
+                        text: t.slice(range.clone()),
+                        lines: false,
+                    };
+                    splice(t, range.clone(), "", range.start, step);
+                }
+            }
+            Operator::Yank => {
+                let mut taken = t.slice(range.clone());
+                if lines && !taken.ends_with('\n') {
+                    taken.push('\n');
+                }
+                *register = Register { text: taken, lines };
+                step.cursor = if lines {
+                    if t.line_of(cursor) == t.line_of(first) {
+                        cursor
+                    } else {
+                        t.clamp(first + t.column(cursor).min(t.line_end(first) - first))
+                    }
+                } else {
+                    t.clamp(first)
+                };
+            }
+            Operator::Indent | Operator::Outdent => {
+                let (first_line, last_line) = (
+                    t.line_of(range.start),
+                    t.line_of(range.end.saturating_sub(1).max(range.start)),
+                );
+                let mut out = String::new();
+                for line in 0..t.lines() {
+                    let (s, e) = (t.line_at(line), t.line_end(t.line_at(line)));
+                    let body: String = t.c[s..e].iter().collect();
+                    if (first_line..=last_line).contains(&line) && !body.trim().is_empty() {
+                        if operator == Operator::Indent {
+                            for _ in 0..count.max(1) {
+                                out.push_str(indent);
+                            }
+                            out.push_str(&body);
+                        } else {
+                            out.push_str(outdent(&body, indent, count.max(1)));
+                        }
+                    } else {
+                        out.push_str(&body);
+                    }
+                    if e < t.len() {
+                        out.push('\n');
+                    }
+                }
+                let new: Vec<char> = out.chars().collect();
+                let n = Text { c: &new };
+                let to = n.first_non_blank(n.line_at(first_line));
+                if out != t.slice(0..t.len()) {
+                    step.text = Some(out);
+                }
+                step.cursor = to;
+            }
+            Operator::Format => {
+                let start = t.line_start(range.start);
+                let end = t.line_end(range.end.saturating_sub(1).max(range.start));
+                // An empty line holds no statement to format.
+                if start < end {
+                    step.then = Some(Then::Format(start..end));
+                }
+                step.cursor = t.first_non_blank(start);
+            }
+        }
+    }
+}
+
+/// The count of an operator and its motion: `2d3w` deletes six words.
+fn total(pending: &Pending) -> usize {
+    let before = pending.operator.map_or(1, |(_, count)| count);
+    before.saturating_mul(pending.count.unwrap_or(1)).max(1)
+}
+
+/// Line `line` (from 1, past the last is the last) as a motion to its
+/// first character that is not blank.
+fn line_target(t: &Text<'_>, line: usize) -> Target {
+    let line = line.clamp(1, t.lines()) - 1;
+    Target {
+        to: t.first_non_blank(t.line_at(line)),
+        kind: Kind::Lines,
+        column: None,
+    }
+}
+
+/// Replaces `range` of the script with `with`, the cursor then at `to`.
+fn splice(t: &Text<'_>, range: Range<usize>, with: &str, to: usize, step: &mut Step) {
+    let mut out: String = t.c[..range.start].iter().collect();
+    out.push_str(with);
+    out.extend(&t.c[range.end..]);
+    step.text = Some(out);
+    step.cursor = to;
+}
+
+/// `at` clamped to normal mode's places in the step's new script.
+fn clamp_in(step: &Step, at: usize) -> usize {
+    let chars: Vec<char> = step.text.as_deref().unwrap_or("").chars().collect();
+    Text { c: &chars }.clamp(at)
+}
+
+/// `r`: `count` characters from the cursor become `c`, where the line has
+/// them.
+fn replace(t: &Text<'_>, cursor: usize, c: char, count: usize, step: &mut Step) {
+    let end = t.line_end(cursor);
+    if cursor + count > end {
+        return;
+    }
+    if c == '\n' {
+        // The characters become one line break, as in vim.
+        splice(t, cursor..cursor + count, "\n", cursor + 1, step);
+        return;
+    }
+    let with: String = std::iter::repeat_n(c, count).collect();
+    splice(t, cursor..cursor + count, &with, cursor + count - 1, step);
+}
+
+/// `p` (after the cursor) or `P` (before it), `count` times.
+fn put(
+    t: &Text<'_>,
+    cursor: usize,
+    after: bool,
+    count: usize,
+    register: &Register,
+    step: &mut Step,
+) {
+    if register.text.is_empty() {
+        return;
+    }
+    let text = register.text.repeat(count.max(1));
+    if register.lines {
+        if after {
+            let end = t.line_end(cursor);
+            if end >= t.len() {
+                // The last line: a break before the lines, none after.
+                let lines = text.strip_suffix('\n').unwrap_or(&text);
+                splice(t, end..end, &format!("\n{lines}"), end + 1, step);
+            } else {
+                splice(t, end + 1..end + 1, &text, end + 1, step);
+            }
+        } else {
+            let start = t.line_start(cursor);
+            splice(t, start..start, &text, start, step);
+        }
+        let chars: Vec<char> = step.text.as_deref().unwrap_or("").chars().collect();
+        step.cursor = Text { c: &chars }.first_non_blank(step.cursor);
+    } else {
+        let empty = t.line_start(cursor) == t.line_end(cursor);
+        let at = if after && !empty { cursor + 1 } else { cursor };
+        let length = text.chars().count();
+        // On the last character put; at its start where it is of lines.
+        let to = if text.contains('\n') {
+            at
+        } else {
+            at + length - 1
+        };
+        splice(t, at..at, &text, to, step);
+    }
+}
+
+/// `J`: `joins` line breaks from line `line` on go, each with the indent
+/// after it, and a space goes between where neither side has one.
+fn join(t: &Text<'_>, line: usize, joins: usize, step: &mut Step) {
+    let mut chars: Vec<char> = t.c.to_vec();
+    let mut at = None;
+    for _ in 0..joins {
+        let n = Text { c: &chars };
+        let end = n.line_end(n.line_at(line));
+        if end >= n.len() {
+            break;
+        }
+        let mut next = end + 1;
+        while next < n.len() && matches!(n.c[next], ' ' | '\t') {
+            next += 1;
+        }
+        let space = end > n.line_at(line)
+            && !n.c[end - 1].is_whitespace()
+            && next < n.len()
+            && n.c[next] != '\n'
+            && n.c[next] != ')';
+        chars.splice(end..next, if space { vec![' '] } else { vec![] });
+        at = Some(end);
+    }
+    if let Some(at) = at {
+        let text: String = chars.iter().collect();
+        step.text = Some(text);
+        step.cursor = Text { c: &chars }.clamp(at);
+    }
+}
+
+/// `f`, `F`, `t` or `T` to the `count`th `c` on the line; `repeat` for
+/// `;` and `,`, where a `t` does not stop where it stands.
+fn find(
+    t: &Text<'_>,
+    cursor: usize,
+    kind: char,
+    c: char,
+    count: usize,
+    repeat: bool,
+) -> Option<Target> {
+    let (start, end) = (t.line_start(cursor), t.line_end(cursor));
+    let forward = matches!(kind, 'f' | 't');
+    let mut at = cursor;
+    for n in 0..count {
+        // A repeated `t` starts past the character it stopped before.
+        let skip = if repeat && n == 0 && matches!(kind, 't' | 'T') {
+            1
+        } else {
+            0
+        };
+        let found = if forward {
+            (at + 1 + skip..end).find(|&i| t.c[i] == c)
+        } else {
+            (start..at.saturating_sub(skip))
+                .rev()
+                .find(|&i| t.c[i] == c)
+        };
+        at = found?;
+    }
+    let to = match kind {
+        't' => at - 1,
+        'T' => at + 1,
+        _ => at,
+    };
+    Some(Target {
+        to,
+        kind: if forward {
+            Kind::Inclusive
+        } else {
+            Kind::Exclusive
+        },
+        column: Some(to - start),
+    })
+}
+
+/// `%`: the bracket that matches the first one at or after the cursor on
+/// its line.
+fn matching(t: &Text<'_>, cursor: usize) -> Option<usize> {
+    let end = t.line_end(cursor);
+    let at = (cursor..end).find(|&i| "()[]{}".contains(t.c[i]))?;
+    let (open, close, forward) = match t.c[at] {
+        '(' => ('(', ')', true),
+        '[' => ('[', ']', true),
+        '{' => ('{', '}', true),
+        ')' => ('(', ')', false),
+        ']' => ('[', ']', false),
+        _ => ('{', '}', false),
+    };
+    let mut depth = 0usize;
+    let mut i = at;
+    loop {
+        let c = t.c[i];
+        if c == open {
+            if forward {
+                depth += 1;
+            } else {
+                depth -= 1;
+            }
+        } else if c == close {
+            if forward {
+                depth -= 1;
+            } else {
+                depth += 1;
+            }
+        }
+        if depth == 0 {
+            return Some(i);
+        }
+        if forward {
+            i += 1;
+            if i >= t.len() {
+                return None;
+            }
+        } else {
+            i = i.checked_sub(1)?;
+        }
+    }
+}
+
+/// `}` or `{`: the next or last empty line, or the script's end or start.
+fn paragraph(t: &Text<'_>, cursor: usize, forward: bool) -> usize {
+    let blank = |line: usize| {
+        let s = t.line_at(line);
+        t.c[s..t.line_end(s)].iter().all(|c| c.is_whitespace())
+    };
+    let line = t.line_of(cursor);
+    if forward {
+        // Past the empty lines the cursor is in, to the next one.
+        let mut l = line;
+        while l < t.lines() && blank(l) {
+            l += 1;
+        }
+        while l < t.lines() && !blank(l) {
+            l += 1;
+        }
+        if l >= t.lines() {
+            // The script's end: a motion's cursor lands on its last
+            // character, and an operator takes it.
+            t.len()
+        } else {
+            t.line_at(l)
+        }
+    } else {
+        let mut l = line;
+        while l > 0 && blank(l) {
+            l -= 1;
+        }
+        while l > 0 && !blank(l) {
+            l -= 1;
+        }
+        t.line_at(l)
+    }
+}
+
+#[derive(Clone, Copy, PartialEq, Eq)]
+enum Class {
+    Blank,
+    Word,
+    Other,
+}
+
+/// A character's class for the word motions: with `big`, all that is not
+/// blank is one.
+fn class(c: char, big: bool) -> Class {
+    if c.is_whitespace() {
+        Class::Blank
+    } else if big || c.is_alphanumeric() || c == '_' {
+        Class::Word
+    } else {
+        Class::Other
+    }
+}
+
+/// Whether `at` is an empty line.
+fn empty_line(t: &Text<'_>, at: usize) -> bool {
+    t.c.get(at) == Some(&'\n') && (at == 0 || t.c[at - 1] == '\n')
+}
+
+/// `w`: the next word's start; an empty line is a word.
+fn word_start(t: &Text<'_>, from: usize, big: bool) -> usize {
+    let n = t.len();
+    if from >= n {
+        return n;
+    }
+    let mut i = from;
+    let own = class(t.c[i], big);
+    if own != Class::Blank {
+        while i < n && class(t.c[i], big) == own {
+            i += 1;
+        }
+    }
+    while i < n && t.c[i].is_whitespace() {
+        if i > from && empty_line(t, i) {
+            return i;
+        }
+        i += 1;
+    }
+    i
+}
+
+/// `e`: the end of this word, or of the next where the cursor is at one.
+fn word_end(t: &Text<'_>, from: usize, big: bool) -> usize {
+    let n = t.len();
+    let mut i = from + 1;
+    while i < n && t.c[i].is_whitespace() {
+        i += 1;
+    }
+    if i >= n {
+        return n.saturating_sub(1).max(from.min(n.saturating_sub(1)));
+    }
+    let own = class(t.c[i], big);
+    while i + 1 < n && class(t.c[i + 1], big) == own {
+        i += 1;
+    }
+    i
+}
+
+/// `b`: the start of this word, or of the one before where the cursor is
+/// at one; an empty line is a word.
+fn word_back(t: &Text<'_>, from: usize, big: bool) -> usize {
+    if from == 0 {
+        return 0;
+    }
+    let mut i = from - 1;
+    while i > 0 && t.c[i].is_whitespace() {
+        if empty_line(t, i) {
+            return i;
+        }
+        i -= 1;
+    }
+    let own = class(t.c[i], big);
+    while i > 0 && class(t.c[i - 1], big) == own && own != Class::Blank {
+        i -= 1;
+    }
+    i
+}
+
+/// `~`'s case change.
+fn flip(c: char) -> char {
+    if c.is_lowercase() {
+        c.to_uppercase().next().unwrap_or(c)
+    } else {
+        c.to_lowercase().next().unwrap_or(c)
+    }
+}
+
+/// `body` with `count` indents taken off its start, or what blanks it
+/// has, where it has fewer.
+fn outdent<'a>(body: &'a str, indent: &str, count: usize) -> &'a str {
+    let mut rest = body;
+    for _ in 0..count {
+        if let Some(after) = rest.strip_prefix(indent) {
+            rest = after;
+        } else if let Some(after) = rest.strip_prefix('\t') {
+            rest = after;
+        } else {
+            let width = indent.chars().count().max(1);
+            let blanks = rest.chars().take(width).take_while(|c| *c == ' ').count();
+            rest = &rest[blanks..];
+        }
+    }
+    rest
+}
+
+/// The cursor that normal mode allows at `at` in `text`: on a character,
+/// not after a line's last (an empty line has only its start).
+pub fn clamp(text: &str, at: usize) -> usize {
+    let chars: Vec<char> = text.chars().collect();
+    Text { c: &chars }.clamp(at)
+}
+
+/// A script as the motions read it: characters and lines.
+struct Text<'a> {
+    c: &'a [char],
+}
+
+impl Text<'_> {
+    fn len(&self) -> usize {
+        self.c.len()
+    }
+
+    fn line_start(&self, at: usize) -> usize {
+        let mut i = at.min(self.len());
+        while i > 0 && self.c[i - 1] != '\n' {
+            i -= 1;
+        }
+        i
+    }
+
+    fn line_end(&self, at: usize) -> usize {
+        let mut i = at.min(self.len());
+        while i < self.len() && self.c[i] != '\n' {
+            i += 1;
+        }
+        i
+    }
+
+    fn column(&self, at: usize) -> usize {
+        at - self.line_start(at)
+    }
+
+    fn first_non_blank(&self, at: usize) -> usize {
+        let (start, end) = (self.line_start(at), self.line_end(at));
+        let mut i = start;
+        while i < end && matches!(self.c[i], ' ' | '\t') {
+            i += 1;
+        }
+        // A line of blanks: its last.
+        if i == end && end > start { end - 1 } else { i }
+    }
+
+    /// The line `at` is on, from 0.
+    fn line_of(&self, at: usize) -> usize {
+        self.c[..at.min(self.len())]
+            .iter()
+            .filter(|c| **c == '\n')
+            .count()
+    }
+
+    fn lines(&self) -> usize {
+        self.c.iter().filter(|c| **c == '\n').count() + 1
+    }
+
+    /// The start of line `line` (from 0).
+    fn line_at(&self, line: usize) -> usize {
+        if line == 0 {
+            return 0;
+        }
+        let mut seen = 0;
+        for (i, c) in self.c.iter().enumerate() {
+            if *c == '\n' {
+                seen += 1;
+                if seen == line {
+                    return i + 1;
+                }
+            }
+        }
+        self.line_start(self.len())
+    }
+
+    fn clamp(&self, at: usize) -> usize {
+        let at = at.min(self.len());
+        let (start, end) = (self.line_start(at), self.line_end(at));
+        if end == start { start } else { at.min(end - 1) }
+    }
+
+    fn slice(&self, range: Range<usize>) -> String {
+        self.c[range].iter().collect()
+    }
+}
 
 #[cfg(test)]
 mod tests {
```

- [ ] **Step 4: Run the tests to see them pass**

Run: `cargo test --locked --lib vim::tests`
Expected: PASS. Then the four checks above.

- [ ] **Step 5: Commit**

```bash
git add src/lib.rs src/vim.rs
git commit -m "Add a vim engine for the SQL editor: motions, operators, visual mode"
```

---

### Task 2: Vim in the SQL editor

**Files:**
- Modify: `src/ui/sql_text.rs`, `src/ui/cell_editor.rs`, `src/model.rs`, `src/app.rs`, `src/app/editing.rs`, `src/ui/workspace.rs`, `src/ui/sql_editor.rs`
- Create: `src/ui/vim_tests.rs`
- Modify (tests): `src/testing.rs`, `src/ui/mod.rs`, and the tests that leave an editor with Esc

- [ ] **Step 1: Write the failing tests**

The Omarchy look's keys are vim's already (step 1's default), so the tests that leave an editor with one Esc take two from insert mode once vim is in: they call `leave_editor`, which is one Esc everywhere else.

```diff
diff --git a/src/app.rs b/src/app.rs
--- a/src/app.rs
+++ b/src/app.rs
@@ -20209,15 +20209,16 @@ masked = ["orders.email"]
             run(&mut harness, tab, "nope");
             harness.app.apply(Action::CloseCommand(tab));
             assert_eq!(prompt(&harness, tab), (None, None));
-            // With a SQL editor in front there is no table to write: the
-            // lines are a table's, and no command there.
+            // With a SQL editor in front there is no table to write: `e!`
+            // is a table's, and no command there, and `w` (vim's write)
+            // does nothing.
             type_into(&mut harness, tab, id, at(3, 1), "dan@example.com");
             harness.app.apply(Action::NewSqlTab(tab));
-            for line in ["w", "e!"] {
+            for (line, refused) in [("w", None), ("e!", Some("e!".to_owned()))] {
                 run(&mut harness, tab, line);
                 assert_eq!(writes(&harness), 1);
                 assert_eq!(pending(&harness), 1);
-                assert_eq!(prompt(&harness, tab), (None, Some(line.to_owned())));
+                assert_eq!(prompt(&harness, tab), (None, refused));
             }
         }
 
```

```diff
diff --git a/src/testing.rs b/src/testing.rs
--- a/src/testing.rs
+++ b/src/testing.rs
@@ -308,6 +308,59 @@ impl Harness {
         self.settle();
     }
 
+    /// Esc out of a SQL editor, as one leaves it: where the keybindings
+    /// are vim's and the editor types (insert mode), the first Esc goes to
+    /// normal mode and a second leaves. Anywhere else, Esc once.
+    pub fn leave_editor(&mut self) {
+        self.settle();
+        let inserting = self.vim_inserting();
+        self.press(egui::Key::Escape, egui::Modifiers::NONE);
+        if inserting {
+            self.press(egui::Key::Escape, egui::Modifiers::NONE);
+        }
+    }
+
+    /// Puts a SQL editor that has the keyboard in vim's normal mode into
+    /// insert mode (`i`), as one does to type. Anywhere else, nothing.
+    pub fn start_typing(&mut self) {
+        self.settle();
+        let terminal = self.app.look.terminal;
+        if self.app.settings.editor_keys_in(terminal) != crate::settings::EditorKeys::Vim {
+            return;
+        }
+        let normal = self.app.tabs.iter().any(|tab| {
+            self.app.workspace(tab.id).is_some_and(|workspace| {
+                workspace.sql_tabs().any(|sql| {
+                    let editor = crate::ui::sql_text::editor_id(tab.id, sql.id);
+                    sql.vim.mode == crate::vim::Mode::Normal
+                        && self.ctx.memory(|memory| memory.has_focus(editor))
+                })
+            })
+        });
+        if normal {
+            self.frame(vec![egui::Event::Text("i".into())]);
+            self.settle();
+        }
+    }
+
+    /// Whether a SQL editor has the keyboard in vim's insert mode.
+    pub fn vim_inserting(&self) -> bool {
+        let terminal = self.app.look.terminal;
+        if self.app.settings.editor_keys_in(terminal) != crate::settings::EditorKeys::Vim {
+            return false;
+        }
+        self.app.tabs.iter().any(|tab| {
+            let Some(workspace) = self.app.workspace(tab.id) else {
+                return false;
+            };
+            workspace.sql_tabs().any(|sql| {
+                let editor = crate::ui::sql_text::editor_id(tab.id, sql.id);
+                sql.vim.mode == crate::vim::Mode::Insert
+                    && self.ctx.memory(|memory| memory.has_focus(editor))
+            })
+        })
+    }
+
     /// The color the last frame painted `text` in.
     pub fn painted_color(&self, text: &str) -> Option<egui::Color32> {
         self.painted
```

```diff
diff --git a/src/ui/editor_results_tests.rs b/src/ui/editor_results_tests.rs
--- a/src/ui/editor_results_tests.rs
+++ b/src/ui/editor_results_tests.rs
@@ -274,7 +274,7 @@ fn the_terminal_looks_letters_take_the_links_on_screen() {
     let before = runs_sent(&harness);
     // In the editor's text a letter is typed; out of it, `t` offers
     // nothing here and `a` shows all.
-    harness.press(Key::Escape, Modifiers::NONE);
+    harness.leave_editor();
     harness.press(Key::T, Modifiers::NONE);
     assert_eq!(runs_sent(&harness), before);
     harness.press(Key::A, Modifiers::NONE);
```

```diff
diff --git a/src/ui/every_row_tests.rs b/src/ui/every_row_tests.rs
--- a/src/ui/every_row_tests.rs
+++ b/src/ui/every_row_tests.rs
@@ -242,7 +242,7 @@ fn the_keyboard_goes_back_only_where_it_was() {
     // Out of the text in the terminal look, the run key still runs; the
     // answer leaves the keyboard out of the text.
     let (mut harness, tab, id) = editor(Look::omarchy(), RunMode::ReadWrite, "delete from users");
-    harness.press(Key::Escape, Modifiers::NONE);
+    harness.leave_editor();
     harness.settle();
     assert!(!harness.ctx.text_edit_focused());
     run_key(&mut harness);
```

```diff
diff --git a/src/ui/format_settings_tests.rs b/src/ui/format_settings_tests.rs
--- a/src/ui/format_settings_tests.rs
+++ b/src/ui/format_settings_tests.rs
@@ -114,7 +114,7 @@ fn keep_leaves_the_case_as_typed_and_names_strings_and_comments_alone() {
 #[test]
 fn the_terminal_look_formats_in_lower_case_until_set() {
     let (mut harness, tab) = editor(Look::omarchy(), "SELECT 1 FROM t");
-    harness.press(Key::Escape, Modifiers::NONE);
+    harness.leave_editor();
     super::tests::type_key(&mut harness, Key::Equals, "=");
     harness.settle();
     assert_eq!(text(&harness, tab), "select 1\n  from t");
@@ -246,7 +246,7 @@ fn tab_is_not_the_editor_s_without_the_keyboard() {
         let (mut harness, tab) = editor(look, "SELECT 1");
         // Esc gives the keyboard up: Omarchy's normal mode, and the other
         // looks' editor left.
-        harness.press(Key::Escape, Modifiers::NONE);
+        harness.leave_editor();
         harness.press(Key::Tab, Modifiers::NONE);
         harness.press(Key::Tab, Modifiers::SHIFT);
         harness.settle();
```

```diff
diff --git a/src/ui/grid_keys_tests.rs b/src/ui/grid_keys_tests.rs
--- a/src/ui/grid_keys_tests.rs
+++ b/src/ui/grid_keys_tests.rs
@@ -270,7 +270,7 @@ fn the_error_keys_put_a_sql_editors_cursor_on_its_errors_mark() {
     }
     // The terminal's letters, out of the script.
     let (mut harness, tab, id) = failed_run(Look::omarchy(), SCRIPT, Some(position));
-    harness.press(Key::Escape, Modifiers::NONE);
+    harness.leave_editor();
     harness.settle();
     assert!(!in_script(&harness, tab, id));
     to_error(&mut harness, true);
```

```diff
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -101,6 +101,8 @@ pub mod toast;
 #[cfg(test)]
 mod typing_settings_tests;
 pub mod value_tags;
+#[cfg(test)]
+mod vim_tests;
 pub mod widgets;
 pub mod workspace;
 pub mod write_prompts;
@@ -2996,7 +2998,7 @@ value_tags   = true
         harness.copy(true);
         assert!(!on_picker(&harness), "from the SQL editor");
         // Out of the text it shows the connections, as from the rows.
-        harness.press(Key::Escape, Modifiers::NONE);
+        harness.leave_editor();
         assert!(!harness.ctx.text_edit_focused());
         harness.copy(true);
         assert!(on_picker(&harness));
@@ -5135,10 +5137,11 @@ value_tags   = true
             type_text(&mut harness, "\nSELECT 1");
             assert_eq!(bars(&harness), none + 1, "{}", look.name);
             // It stays with the statement when the keys leave the editor.
-            harness.press(Key::Escape, Modifiers::NONE);
+            harness.leave_editor();
             assert_eq!(bars(&harness), none + 1, "{}", look.name);
             let editor = editor_rect(&mut harness);
             click_at(&mut harness, editor.center());
+            harness.start_typing();
             for _ in 0.."SELECT 1".len() {
                 harness.press(Key::Backspace, Modifiers::NONE);
             }
@@ -5152,7 +5155,7 @@ value_tags   = true
             let (mut harness, tab) = sql_harness(look);
             harness.settle();
             type_text(&mut harness, "SELECT 1;\nSELECT 2;\nSELECT 3");
-            harness.press(Key::Escape, Modifiers::NONE);
+            harness.leave_editor();
             assert!(!harness.ctx.text_edit_focused());
             // Left of the text, beside its first line.
             let editor = editor_rect(&mut harness);
@@ -5160,6 +5163,7 @@ value_tags   = true
             click_at(&mut harness, egui::pos2(gutter, editor.top() + 12.0));
             assert!(harness.ctx.text_edit_focused(), "{}", look.name);
             assert_eq!(active_sql(&harness, tab).line_col(), (1, 1));
+            harness.start_typing();
             type_text(&mut harness, "x");
             assert!(active_sql(&harness, tab).text.starts_with("xSELECT 1;"));
             // With the keys in the editor, a press there keeps them. Under
@@ -5411,7 +5415,7 @@ value_tags   = true
             assert!(painted(&harness, keys), "{keys} in {}", look.name);
             // The editor gives the keys up (Esc); the button formats and
             // gives them back.
-            harness.press(Key::Escape, Modifiers::NONE);
+            harness.leave_editor();
             assert!(!harness.ctx.text_edit_focused(), "{}", look.name);
             harness.click("Format");
             assert_eq!(active_sql(&harness, tab).text, "SELECT 1", "{}", look.name);
@@ -5426,7 +5430,7 @@ value_tags   = true
         harness.settle();
         type_text(&mut harness, "select 1");
         assert!(!harness.has("Format"));
-        harness.press(Key::Escape, Modifiers::NONE);
+        harness.leave_editor();
         type_key(&mut harness, Key::Equals, "=");
         assert_eq!(active_sql(&harness, tab).text, "SELECT 1");
         assert!(harness.ctx.text_edit_focused());
@@ -5445,7 +5449,7 @@ value_tags   = true
         harness.press(Key::F, COMMAND_SHIFT);
         assert_eq!(active_sql(&harness, tab).text, "select 1=");
         harness.press(Key::Backspace, Modifiers::NONE);
-        harness.press(Key::Escape, Modifiers::NONE);
+        harness.leave_editor();
         type_key(&mut harness, Key::Equals, "=");
         assert_eq!(active_sql(&harness, tab).text, "SELECT 1");
         // The other looks type it wherever they are.
@@ -5753,7 +5757,7 @@ value_tags   = true
         let tab = harness.connect_fake();
         let id = with_sql_result(&mut harness, tab, 3);
         let at = |row, col| Some(crate::model::CellPos { row, col });
-        harness.press(Key::Escape, Modifiers::NONE);
+        harness.leave_editor();
         harness.press(Key::J, Modifiers::NONE);
         harness.press(Key::J, Modifiers::NONE);
         harness.press(Key::L, Modifiers::NONE);
@@ -6727,7 +6731,7 @@ value_tags   = true
         harness.settle();
         assert!(harness.ctx.text_edit_focused());
         // The first Esc only leaves the editor; the next one closes the panel.
-        harness.press(Key::Escape, Modifiers::NONE);
+        harness.leave_editor();
         assert!(!harness.ctx.text_edit_focused());
         assert!(open(&harness));
         assert!(panel_shows(&mut harness));
@@ -16427,7 +16431,7 @@ value_tags   = true
         assert_eq!(harness.copied.as_deref(), Some("user2@example.com"));
         type_key(&mut harness, Key::Slash, "/");
         assert!(harness.ctx.text_edit_focused(), "the WHERE line");
-        harness.press(Key::Escape, Modifiers::NONE);
+        harness.leave_editor();
 
         // A SQL editor's result: Enter is its row panel's still, and the
         // letters that edit do nothing there.
@@ -16436,7 +16440,7 @@ value_tags   = true
         let tab = harness.connect_fake_as(false);
         let sql = with_sql_result(&mut harness, tab, 3);
         let panel = |harness: &Harness| harness.app.workspace(tab).unwrap().row_panel;
-        harness.press(Key::Escape, Modifiers::NONE);
+        harness.leave_editor();
         type_key(&mut harness, Key::J, "j");
         type_key(&mut harness, Key::L, "l");
         assert_eq!(
```

```diff
diff --git a/src/ui/pane_keys_tests.rs b/src/ui/pane_keys_tests.rs
--- a/src/ui/pane_keys_tests.rs
+++ b/src/ui/pane_keys_tests.rs
@@ -92,7 +92,7 @@ fn ctrl_k_takes_it_back_up_to_the_script_and_ctrl_j_down_again() {
     assert!(in_script(&harness, tab, id), "up again: the script");
     // Out of the script with Esc, the keys are normal mode's: ctrl+j is
     // the pane below.
-    harness.press(Key::Escape, Modifiers::NONE);
+    harness.leave_editor();
     harness.settle();
     assert!(!in_script(&harness, tab, id));
     assert_eq!(press(&mut harness, Key::J), "Rows");
@@ -135,7 +135,7 @@ fn a_tables_rows_have_no_pane_above_or_below() {
 fn from_outside_the_stack_ctrl_j_and_ctrl_k_move_nothing() {
     let (mut harness, tab, id) = ran(Look::omarchy(), "SELECT 1");
     // The keyboard on the tree: out of the script, then to the left.
-    harness.press(Key::Escape, Modifiers::NONE);
+    harness.leave_editor();
     assert_eq!(press(&mut harness, Key::H), "Objects");
     for key in [Key::J, Key::K] {
         assert_eq!(press(&mut harness, key), "Objects", "{key:?}");
```

```diff
diff --git a/src/ui/sql_editor.rs b/src/ui/sql_editor.rs
--- a/src/ui/sql_editor.rs
+++ b/src/ui/sql_editor.rs
@@ -1511,10 +1511,12 @@ mod tests {
         assert_eq!(mode_of(&harness, tab, id), RunMode::ReadWrite);
         line(&mut harness, "ro");
         // A table's lines are no commands of an editor's, and the lines
-        // of what is not built yet are claimed and do nothing.
+        // of what is not built yet are claimed and do nothing. Vim's `:w`
+        // does nothing and says nothing.
         for text in ["w", "e!", "diff", "commit", "rollback", "12"] {
             line(&mut harness, text);
-            assert_eq!(refused(&harness).as_deref(), Some(text));
+            let said = (text != "w").then_some(text);
+            assert_eq!(refused(&harness).as_deref(), said);
             assert_eq!(mode_of(&harness, tab, id), RunMode::ReadOnly, "{text}");
         }
     }
```

```diff
diff --git a/src/ui/sql_results.rs b/src/ui/sql_results.rs
--- a/src/ui/sql_results.rs
+++ b/src/ui/sql_results.rs
@@ -2881,7 +2881,7 @@ mod tests {
             look.name
         );
         if look.terminal {
-            harness.press(Key::Escape, Modifiers::NONE);
+            harness.leave_editor();
             harness.press(letter, Modifiers::NONE);
         } else {
             harness.click(name);
@@ -3109,7 +3109,7 @@ mod tests {
         use crate::model::RunMode;
         let (mut harness, tab) = refused_in_a_read_only_tab(Look::omarchy());
         let sent = runs_sent(&harness);
-        harness.press(Key::Escape, Modifiers::NONE);
+        harness.leave_editor();
         // The key goes down and stays down. Its first event allows writes,
         // and the card's letter then means "run again": the repeats of the
         // same press must not take that for an answer.
@@ -3479,8 +3479,11 @@ mod tests {
             .filter(|(at, _)| *at > top)
             .collect();
         found.sort_by(|(a, _), (b, _)| a.total_cmp(b));
-        // The cursor's place is the first thing under the pane.
-        let under = |text: &str| text.starts_with("Ln ") || text.starts_with("ln ");
+        // The cursor's place is the first thing under the pane, or the
+        // terminal's line starts with vim's mode.
+        let under = |text: &str| {
+            text.starts_with("Ln ") || text.starts_with("ln ") || text.starts_with("-- ")
+        };
         let bottom = found
             .iter()
             .find(|(_, text)| under(text))
```

```diff
diff --git a/src/ui/sql_text.rs b/src/ui/sql_text.rs
--- a/src/ui/sql_text.rs
+++ b/src/ui/sql_text.rs
@@ -1685,6 +1685,7 @@ mod tests {
                     palette: &palette,
                     hold_escape,
                     pairs: false,
+                    vim: false,
                     style: sql::format::Style::default(),
                     // A tab character: what Tab put in before the setting.
                     indent: Indent::Tab,
```

```diff
diff --git a/src/ui/vim_tests.rs b/src/ui/vim_tests.rs
new file mode 100644
--- /dev/null
+++ b/src/ui/vim_tests.rs
@@ -0,0 +1,451 @@
+//! Settings › Editor's Keybindings: vim's normal, insert and visual modes
+//! in the SQL editor, the mode in the status line, and the rows that set
+//! them.
+
+use egui::{Key, Modifiers};
+
+use crate::model::{Action, ConnTabId, RunMode};
+use crate::settings::{EditorKeys, OptionValue};
+use crate::testing::Harness;
+use crate::theme::Look;
+use crate::vim::Mode;
+
+/// A connected fixture in `look`, with a new SQL editor that has the
+/// keyboard and holds `text`, its cursor at the end. A new tab types: in
+/// vim it is in insert mode.
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
+fn mode(harness: &Harness, tab: ConnTabId) -> Mode {
+    let workspace = harness.app.workspace(tab).unwrap();
+    workspace.active_sql_tab().unwrap().vim.mode
+}
+
+fn selected(harness: &Harness, tab: ConnTabId) -> Option<std::ops::Range<usize>> {
+    let workspace = harness.app.workspace(tab).unwrap();
+    workspace.active_sql_tab().unwrap().selected.clone()
+}
+
+/// Types `keys` as the window sends them: for each character the press of
+/// its key, where it has one, with the character.
+fn keys(harness: &mut Harness, keys: &str) {
+    for c in keys.chars() {
+        let key = Key::from_name(&c.to_ascii_uppercase().to_string());
+        let shift = if c.is_uppercase() {
+            Modifiers::SHIFT
+        } else {
+            Modifiers::NONE
+        };
+        let mut events = Vec::new();
+        if let Some(key) = key {
+            events.push(crate::testing::key(key, shift));
+        }
+        events.push(egui::Event::Text(c.to_string()));
+        if let Some(key) = key {
+            events.push(crate::testing::release(key, shift));
+        }
+        harness.frame(events);
+        harness.settle();
+    }
+}
+
+fn escape(harness: &mut Harness) {
+    harness.press(Key::Escape, Modifiers::NONE);
+}
+
+fn set(harness: &mut Harness, value: OptionValue) {
+    harness.app.apply(Action::SetOption(value));
+    harness.settle();
+}
+
+#[test]
+fn a_new_tab_types_and_esc_goes_to_normal_mode_then_out_of_the_editor() {
+    let (mut harness, tab) = editor(Look::omarchy(), "");
+    keys(&mut harness, "select 1");
+    assert_eq!(text(&harness, tab), "select 1");
+    assert_eq!(mode(&harness, tab), Mode::Insert);
+    // The line starts with the mode, and Esc is normal mode's way.
+    assert!(harness.painted_rect("-- INSERT --").is_some());
+    assert!(harness.painted_rect("esc normal").is_some());
+    escape(&mut harness);
+    assert_eq!(mode(&harness, tab), Mode::Normal);
+    assert!(harness.ctx.text_edit_focused(), "the editor keeps the keys");
+    assert_eq!(cursor(&harness, tab), 7, "back on the last character");
+    assert!(harness.painted_rect("-- INSERT --").is_none());
+    assert!(harness.painted_rect("esc leave editor").is_some());
+    // Letters are commands now, not text.
+    keys(&mut harness, "x");
+    assert_eq!(text(&harness, tab), "select ");
+    escape(&mut harness);
+    assert!(!harness.ctx.text_edit_focused());
+}
+
+#[test]
+fn normal_mode_edits_in_undo_steps_and_puts_back_what_it_took() {
+    let (mut harness, tab) = editor(Look::omarchy(), "select 1;\nselect 2;");
+    escape(&mut harness);
+    keys(&mut harness, "ggdd");
+    assert_eq!(text(&harness, tab), "select 2;");
+    keys(&mut harness, "p");
+    assert_eq!(text(&harness, tab), "select 2;\nselect 1;");
+    keys(&mut harness, "u");
+    assert_eq!(text(&harness, tab), "select 2;");
+    keys(&mut harness, "u");
+    assert_eq!(text(&harness, tab), "select 1;\nselect 2;");
+    harness.press(Key::R, Modifiers::CTRL);
+    assert_eq!(text(&harness, tab), "select 2;");
+    // A motion with a count, then insert mode where `cw` leaves it.
+    keys(&mut harness, "wcw3");
+    escape(&mut harness);
+    assert_eq!(text(&harness, tab), "select 3;");
+    assert_eq!(mode(&harness, tab), Mode::Normal);
+}
+
+#[test]
+fn ctrl_w_in_normal_mode_deletes_nothing_and_closes_no_tab() {
+    let (mut harness, tab) = editor(Look::omarchy(), "select 1");
+    escape(&mut harness);
+    let tabs = |harness: &Harness| harness.app.workspace(tab).unwrap().tabs.len();
+    let open = tabs(&harness);
+    harness.press(Key::W, Modifiers::CTRL);
+    assert_eq!(text(&harness, tab), "select 1");
+    assert_eq!(tabs(&harness), open);
+}
+
+#[test]
+fn the_field_s_deleting_chords_delete_nothing_in_normal_mode() {
+    let (mut harness, tab) = editor(Look::macos(), "select title");
+    set(&mut harness, OptionValue::EditorKeys(Some(EditorKeys::Vim)));
+    keys(&mut harness, "0w");
+    for (key, modifiers) in [
+        (Key::Backspace, Modifiers::COMMAND),
+        (Key::Backspace, Modifiers::ALT),
+        (Key::Delete, Modifiers::ALT),
+        (Key::K, Modifiers::CTRL),
+    ] {
+        harness.press(key, modifiers);
+        assert_eq!(text(&harness, tab), "select title", "{key:?} {modifiers:?}");
+    } // Page Down is the field's: it moves, and types nothing.
+    harness.press(Key::PageDown, Modifiers::NONE);
+    assert_eq!(text(&harness, tab), "select title");
+}
+
+#[test]
+fn a_visual_selection_is_what_runs() {
+    let (mut harness, tab) = editor(Look::omarchy(), "select 1;\nselect 2;");
+    escape(&mut harness);
+    keys(&mut harness, "ggV");
+    assert_eq!(mode(&harness, tab), Mode::VisualLine);
+    assert_eq!(selected(&harness, tab), Some(0..9));
+    assert!(harness.painted_rect("-- VISUAL LINE --").is_some());
+    assert!(harness.painted_rect("esc normal").is_some());
+    keys(&mut harness, "j");
+    assert_eq!(selected(&harness, tab), Some(0..19));
+    keys(&mut harness, "k");
+    harness.press(Key::Enter, Modifiers::CTRL);
+    let sent = harness
+        .app
+        .backend
+        .sent
+        .iter()
+        .rev()
+        .find_map(|command| match command {
+            crate::backend::Command::RunSql { statements, .. } => Some(
+                statements
+                    .iter()
+                    .map(|statement| statement.text.clone())
+                    .collect::<Vec<_>>(),
+            ),
+            _ => None,
+        })
+        .expect("a run");
+    assert_eq!(sent, ["select 1"]);
+    // By character: both ends in.
+    escape(&mut harness);
+    assert_eq!(mode(&harness, tab), Mode::Normal);
+    assert_eq!(selected(&harness, tab), None);
+    keys(&mut harness, "0ve");
+    assert_eq!(selected(&harness, tab), Some(0..6));
+    assert_eq!(
+        cursor(&harness, tab),
+        5,
+        "on the selection's last character"
+    );
+    keys(&mut harness, "d");
+    assert_eq!(text(&harness, tab), " 1;\nselect 2;");
+}
+
+#[test]
+fn equals_formats_the_cursor_s_statement_and_gg_equals_g_the_tab() {
+    let script = "select 1 from t;\nselect 2 from u;";
+    let (mut harness, tab) = editor(Look::omarchy(), script);
+    escape(&mut harness);
+    keys(&mut harness, "gg==");
+    assert_eq!(text(&harness, tab), "select 1\n  from t;\nselect 2 from u;");
+    assert!(harness.ctx.text_edit_focused());
+    assert_eq!(mode(&harness, tab), Mode::Normal);
+    assert_eq!(
+        selected(&harness, tab),
+        None,
+        "the lines are not left selected"
+    );
+    // One undo step, as Format's, and the cursor back, not the lines.
+    keys(&mut harness, "u");
+    assert_eq!(text(&harness, tab), script);
+    assert_eq!(selected(&harness, tab), None);
+    // A statement Format leaves as it is: nothing selected either.
+    keys(&mut harness, "gg==");
+    keys(&mut harness, "==");
+    assert_eq!(selected(&harness, tab), None);
+    keys(&mut harness, "gg=G");
+    assert_eq!(
+        text(&harness, tab),
+        "select 1\n  from t;\n\nselect 2\n  from u;"
+    );
+}
+
+#[test]
+fn colon_opens_the_prompt_which_gives_the_keyboard_back() {
+    let (mut harness, tab) = editor(Look::omarchy(), "select 1");
+    escape(&mut harness);
+    let sql = harness.app.workspace(tab).unwrap().active_tab.unwrap();
+    keys(&mut harness, ":");
+    assert_eq!(
+        harness.app.workspace(tab).unwrap().command.as_deref(),
+        Some("")
+    );
+    assert_eq!(text(&harness, tab), "select 1", "the colon is not typed");
+    harness.frame(vec![egui::Event::Text("ro".into())]);
+    harness.press(Key::Enter, Modifiers::NONE);
+    let workspace = harness.app.workspace(tab).unwrap();
+    assert_eq!(workspace.sql_tab(sql).unwrap().mode, RunMode::ReadOnly);
+    assert!(harness.ctx.text_edit_focused(), "back in the editor");
+    assert_eq!(mode(&harness, tab), Mode::Normal);
+    // `:w` writes nothing and says nothing.
+    keys(&mut harness, ":");
+    harness.frame(vec![egui::Event::Text("w".into())]);
+    harness.press(Key::Enter, Modifiers::NONE);
+    let workspace = harness.app.workspace(tab).unwrap();
+    assert_eq!(workspace.command_error, None);
+    assert!(harness.ctx.text_edit_focused());
+    assert_eq!(text(&harness, tab), "select 1");
+    // Esc in the prompt gives it back too; the prompt losing the keyboard
+    // to anything else does not.
+    keys(&mut harness, ":");
+    escape(&mut harness);
+    assert!(harness.ctx.text_edit_focused());
+    keys(&mut harness, ":");
+    harness.app.apply(Action::CloseCommand(tab));
+    harness.settle();
+    assert!(!harness.ctx.text_edit_focused());
+    // Esc out and keys in one frame: the keys are nobody's.
+    harness.leave_editor();
+    let workspace = harness.app.workspace_mut(tab).unwrap();
+    workspace.sql_tab_mut(sql).unwrap().focus_editor = true;
+    harness.settle();
+    let mut events = vec![
+        crate::testing::key(Key::Escape, Modifiers::NONE),
+        crate::testing::release(Key::Escape, Modifiers::NONE),
+    ];
+    events.extend(typed('d'));
+    events.extend(typed('d'));
+    harness.frame(events);
+    harness.settle();
+    assert!(!harness.ctx.text_edit_focused());
+    assert_eq!(text(&harness, tab), "select 1");
+}
+
+#[test]
+fn an_open_completion_list_takes_esc_and_insert_mode_stays() {
+    let (mut harness, tab) = editor(Look::omarchy(), "");
+    keys(&mut harness, "sel");
+    let open = |harness: &Harness| {
+        let workspace = harness.app.workspace(tab).unwrap();
+        workspace.active_sql_tab().unwrap().completion.is_some()
+    };
+    assert!(open(&harness), "the list for `sel`");
+    escape(&mut harness);
+    assert!(!open(&harness));
+    assert_eq!(mode(&harness, tab), Mode::Insert);
+    escape(&mut harness);
+    assert_eq!(mode(&harness, tab), Mode::Normal);
+}
+
+#[test]
+fn switching_the_keybindings_applies_at_once_and_keeps_the_keyboard_where_it_is() {
+    let (mut harness, tab) = editor(Look::omarchy(), "select 1");
+    assert_eq!(mode(&harness, tab), Mode::Insert);
+    // The standard keys: Esc leaves at once, as before vim.
+    set(
+        &mut harness,
+        OptionValue::EditorKeys(Some(EditorKeys::Standard)),
+    );
+    assert!(harness.ctx.text_edit_focused());
+    assert!(harness.painted_rect("-- INSERT --").is_none());
+    assert_eq!(mode(&harness, tab), Mode::Normal, "out of insert mode");
+    escape(&mut harness);
+    assert!(!harness.ctx.text_edit_focused());
+    // Back to vim: in normal mode, the keyboard where it was put.
+    let workspace = harness.app.workspace_mut(tab).unwrap();
+    let id = workspace.active_tab.unwrap();
+    workspace.sql_tab_mut(id).unwrap().focus_editor = true;
+    harness.settle();
+    set(&mut harness, OptionValue::EditorKeys(Some(EditorKeys::Vim)));
+    assert!(harness.ctx.text_edit_focused());
+    assert_eq!(mode(&harness, tab), Mode::Normal);
+    keys(&mut harness, "x");
+    assert_eq!(text(&harness, tab), "select ");
+}
+
+#[test]
+fn the_macos_look_has_vim_too_with_its_mode_in_the_footer_and_a_block_cursor() {
+    let (mut harness, tab) = editor(Look::macos(), "SELECT 1");
+    // Standard keys: Esc leaves at once.
+    assert!(harness.painted_rect("-- INSERT --").is_none());
+    set(&mut harness, OptionValue::EditorKeys(Some(EditorKeys::Vim)));
+    assert_eq!(mode(&harness, tab), Mode::Normal);
+    let block = harness.app.palette.accent.gamma_multiply(0.5);
+    let blocks = |harness: &Harness| {
+        harness
+            .fills
+            .iter()
+            .filter(|(_, fill)| *fill == block)
+            .count()
+    };
+    assert_eq!(blocks(&harness), 1, "normal mode's cursor is a block");
+    keys(&mut harness, "i");
+    assert!(harness.painted_rect("-- INSERT --").is_some());
+    assert_eq!(blocks(&harness), 0, "insert mode's is a line");
+    keys(&mut harness, "x");
+    assert_eq!(text(&harness, tab), "SELECT x1");
+    escape(&mut harness);
+    assert_eq!(mode(&harness, tab), Mode::Normal);
+    assert!(harness.ctx.text_edit_focused());
+    escape(&mut harness);
+    assert!(!harness.ctx.text_edit_focused());
+}
+
+/// The events of `c` typed, as the window sends them.
+fn typed(c: char) -> Vec<egui::Event> {
+    let key = Key::from_name(&c.to_ascii_uppercase().to_string()).expect("a letter");
+    let shift = if c.is_uppercase() {
+        Modifiers::SHIFT
+    } else {
+        Modifiers::NONE
+    };
+    vec![
+        crate::testing::key(key, shift),
+        egui::Event::Text(c.to_string()),
+        crate::testing::release(key, shift),
+    ]
+}
+
+#[test]
+fn keys_in_one_frame_are_read_in_the_mode_the_ones_before_left() {
+    // Into insert mode and typing, in one frame.
+    let (mut harness, tab) = editor(Look::omarchy(), "select 1");
+    escape(&mut harness);
+    let mut events = typed('A');
+    events.extend(typed('x'));
+    harness.frame(events);
+    harness.settle();
+    assert_eq!(mode(&harness, tab), Mode::Insert);
+    assert_eq!(text(&harness, tab), "select 1x");
+    // Typing, out of insert mode and a command, in one frame.
+    let mut events = typed('y');
+    events.push(crate::testing::key(Key::Escape, Modifiers::NONE));
+    events.push(crate::testing::release(Key::Escape, Modifiers::NONE));
+    events.extend(typed('x'));
+    harness.frame(events);
+    harness.settle();
+    assert_eq!(mode(&harness, tab), Mode::Normal);
+    assert_eq!(text(&harness, tab), "select 1x");
+    // An undo and the keys after it, in one frame: they read the script
+    // the undo gave back.
+    let (mut harness, tab) = editor(Look::omarchy(), "abc");
+    escape(&mut harness);
+    keys(&mut harness, "0x");
+    assert_eq!(text(&harness, tab), "bc");
+    let mut events = typed('u');
+    events.extend(typed('l'));
+    events.extend(typed('x'));
+    harness.frame(events);
+    harness.settle();
+    assert_eq!(text(&harness, tab), "ac");
+}
+
+#[test]
+fn tab_in_normal_mode_types_nothing() {
+    let (mut harness, tab) = editor(Look::omarchy(), "select 1");
+    escape(&mut harness);
+    harness.press(Key::Tab, Modifiers::NONE);
+    assert_eq!(text(&harness, tab), "select 1");
+}
+
+#[test]
+fn undo_after_a_visual_delete_leaves_no_selection() {
+    let (mut harness, tab) = editor(Look::omarchy(), "select 1;\nselect 2;");
+    escape(&mut harness);
+    keys(&mut harness, "ggVd");
+    keys(&mut harness, "u");
+    assert_eq!(text(&harness, tab), "select 1;\nselect 2;");
+    assert_eq!(mode(&harness, tab), Mode::Normal);
+    assert_eq!(selected(&harness, tab), None);
+}
+
+#[test]
+fn a_cut_in_visual_mode_ends_it() {
+    let (mut harness, tab) = editor(Look::omarchy(), "select 1");
+    escape(&mut harness);
+    keys(&mut harness, "0ve");
+    assert!(harness.painted_rect("esc leave editor").is_none());
+    harness.frame(vec![egui::Event::Cut]);
+    harness.settle();
+    assert_eq!(text(&harness, tab), " 1");
+    assert_eq!(mode(&harness, tab), Mode::Normal);
+}
+
+#[test]
+fn a_held_esc_in_insert_mode_stays_in_the_editor() {
+    let (mut harness, tab) = editor(Look::omarchy(), "select 1");
+    // Held: the press again with no release between, which egui reads as
+    // a repeat.
+    harness.frame(vec![crate::testing::key(Key::Escape, Modifiers::NONE)]);
+    harness.frame(vec![crate::testing::key(Key::Escape, Modifiers::NONE)]);
+    harness.frame(vec![crate::testing::release(Key::Escape, Modifiers::NONE)]);
+    harness.settle();
+    assert_eq!(mode(&harness, tab), Mode::Normal);
+    assert!(harness.ctx.text_edit_focused());
+    // Ctrl+[ is Esc too.
+    keys(&mut harness, "i");
+    harness.press(Key::OpenBracket, Modifiers::CTRL);
+    assert_eq!(mode(&harness, tab), Mode::Normal);
+}
+
+#[test]
+fn ct_to_the_next_character_changes_what_is_between() {
+    let (mut harness, tab) = editor(Look::omarchy(), "select count(a)");
+    escape(&mut harness);
+    keys(&mut harness, "$hct)n");
+    assert_eq!(mode(&harness, tab), Mode::Insert);
+    assert_eq!(text(&harness, tab), "select count(n)");
+}
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test --locked --lib -- vim_tests sql_editor::tests app::tests::editing editor_results_tests every_row_tests format_settings_tests grid_keys_tests pane_keys_tests`
Expected: FAIL to compile (4 errors): `SqlTab` has no `vim`, and the text field's `Field` none either.

- [ ] **Step 3: Put vim in the editor**

```diff
diff --git a/src/app.rs b/src/app.rs
--- a/src/app.rs
+++ b/src/app.rs
@@ -1128,6 +1128,14 @@ impl App {
                     workspace.focus_where = true;
                 }
             }
+            Action::EditorCommand { tab, sql_tab } => {
+                self.apply(Action::OpenCommand(tab));
+                if let Some(workspace) = self.workspace_mut(tab)
+                    && workspace.command.is_some()
+                {
+                    workspace.command_returns = Some(sql_tab);
+                }
+            }
             Action::OpenCommand(tab) => {
                 // The keys do not run under a dialog, and neither does
                 // the prompt they open.
@@ -1135,6 +1143,7 @@ impl App {
                     return;
                 }
                 if let Some(workspace) = self.workspace_mut(tab) {
+                    workspace.command_returns = None;
                     workspace.command = Some(String::new());
                     workspace.focus_command = true;
                     workspace.command_error = None;
@@ -1151,9 +1160,23 @@ impl App {
                     workspace.save_refused = false;
                     workspace.review_refused = false;
                     workspace.label_refused = None;
+                    workspace.command_returns = None;
                 }
             }
-            Action::RunCommand(tab) => self.run_command(tab),
+            Action::EscapeCommand(tab) => {
+                let returns = self
+                    .workspace(tab)
+                    .and_then(|workspace| workspace.command_returns);
+                self.apply(Action::CloseCommand(tab));
+                if let Some(workspace) = self.workspace_mut(tab) {
+                    workspace.command_returns = returns;
+                }
+                self.command_closed(tab);
+            }
+            Action::RunCommand(tab) => {
+                self.run_command(tab);
+                self.command_closed(tab);
+            }
             Action::ClearSort { tab, object_tab } => {
                 if let Some(object) = self.object_tab_mut(tab, object_tab)
                     && !object.query.sort.is_empty()
@@ -5141,6 +5164,18 @@ impl App {
         let old = std::mem::replace(&mut self.settings, new);
         // The keys: the text they were read from is the file's now.
         self.refresh_keymap();
+        let terminal = self.look.terminal;
+        if old.editor_keys_in(terminal) != self.settings.editor_keys_in(terminal) {
+            // Other keybindings: every editor in normal mode, as vim
+            // starts.
+            for tab in &mut self.tabs {
+                if let ConnTabContent::Workspace(workspace) = &mut tab.content {
+                    for sql in workspace.sql_tabs_mut() {
+                        sql.vim.to_normal();
+                    }
+                }
+            }
+        }
         if old.timestamps != self.settings.timestamps {
             let full = self.settings.timestamps == crate::settings::Timestamps::Full;
             for tab in &mut self.tabs {
```

```diff
diff --git a/src/app/editing.rs b/src/app/editing.rs
--- a/src/app/editing.rs
+++ b/src/app/editing.rs
@@ -1851,6 +1851,11 @@ impl App {
             None if workspace.history_in_front() => Scope::History,
             None => Scope::SqlEditor,
         };
+        // Vim's `:w` in a SQL editor: there is nothing to write, and it
+        // never writes to the database.
+        if scope == Scope::SqlEditor && line == "w" {
+            return;
+        }
         // The Structure view's own lines hold while it shows.
         let structure = workspace
             .active_object_tab()
@@ -1945,6 +1950,25 @@ impl App {
         self.apply(action);
     }
 
+    /// After the prompt closes with Enter or Esc: a SQL editor that opened
+    /// it from vim's normal mode gets the keyboard back, while it is still
+    /// in front. (Closed by anything else, the keyboard is where that put
+    /// it.)
+    pub(super) fn command_closed(&mut self, tab: ConnTabId) {
+        let Some(workspace) = self.workspace_mut(tab) else {
+            return;
+        };
+        if workspace.command.is_some() {
+            return;
+        }
+        if let Some(id) = workspace.command_returns.take()
+            && workspace.active_tab == Some(id)
+            && let Some(sql) = workspace.sql_tab_mut(id)
+        {
+            sql.focus_editor = true;
+        }
+    }
+
     /// Shows or hides the Review SQL of the tab. Shown, it takes the text
     /// being typed first, as a save does: what is reviewed is what a save
     /// would send. It opens only on something pending, and its text is
```

```diff
diff --git a/src/model.rs b/src/model.rs
--- a/src/model.rs
+++ b/src/model.rs
@@ -177,8 +177,17 @@ pub enum Action {
     FocusWhere(ConnTabId),
     /// Open the terminal look's `:` prompt in the status line.
     OpenCommand(ConnTabId),
+    /// Open the prompt from a SQL editor in vim's normal mode, which gets
+    /// the keyboard back when the prompt closes.
+    EditorCommand {
+        tab: ConnTabId,
+        sql_tab: TabId,
+    },
     /// Close the prompt, and take away what it said of its last line.
     CloseCommand(ConnTabId),
+    /// Close the prompt with Esc: as `CloseCommand`, and a SQL editor
+    /// that opened it gets the keyboard back.
+    EscapeCommand(ConnTabId),
     /// Run what the prompt holds: `w` writes the pending changes of the
     /// table on screen, `e!` discards them, and anything else is not a
     /// command.
@@ -1210,6 +1219,9 @@ pub struct Workspace {
     pub command: Option<String>,
     /// Focus the prompt on the next frame.
     pub focus_command: bool,
+    /// The SQL tab whose editor opened the prompt from vim's normal mode:
+    /// the keyboard goes back to it when the prompt closes.
+    pub command_returns: Option<TabId>,
     /// What the prompt was given that is not a command, until the next
     /// key.
     pub command_error: Option<String>,
@@ -4935,6 +4947,10 @@ pub struct SqlTab {
     /// Format the script on the next frame: the editor does it, where the
     /// selection and the undo history are.
     pub format: bool,
+    /// Vim's mode in the editor, a command typed in part and vim's undo
+    /// history, where the keybindings are vim's. A new tab is for typing: it opens in insert
+    /// mode.
+    pub vim: crate::vim::Vim,
     /// The text the last run that finished started with. Its error mark
     /// names a line of that text, so it holds only while the text is that.
     ran_text: Option<TextPrint>,
@@ -4992,6 +5008,7 @@ impl SqlTab {
             completion: None,
             completion_wanted: None,
             format: false,
+            vim: crate::vim::Vim::new(crate::vim::Mode::Insert),
             ran_text: None,
             ran_mode: tabletist_db::ScriptMode::ReadOnly,
         }
@@ -5385,6 +5402,7 @@ impl Workspace {
             focus_where: false,
             command: None,
             focus_command: false,
+            command_returns: None,
             command_error: None,
             save_refused: false,
             review_refused: false,
```

```diff
diff --git a/src/ui/cell_editor.rs b/src/ui/cell_editor.rs
--- a/src/ui/cell_editor.rs
+++ b/src/ui/cell_editor.rs
@@ -646,7 +646,20 @@ pub(crate) fn block_cursor(
     let Some(cursor) = output.cursor_range else {
         return;
     };
-    let caret = output.galley.pos_from_cursor(cursor.primary);
+    block_cursor_at(ui, output, cursor.primary, role, look, palette);
+}
+
+/// The block over the character at `at`: the SQL editor's in vim's
+/// visual mode, whose cursor is not the field's.
+pub(crate) fn block_cursor_at(
+    ui: &Ui,
+    output: &egui::text_edit::TextEditOutput,
+    at: egui::text::CCursor,
+    role: TextRole,
+    look: &Look,
+    palette: &Palette,
+) {
+    let caret = output.galley.pos_from_cursor(at);
     let block = Rect::from_min_size(
         output.galley_pos + caret.min.to_vec2(),
         vec2(role.width(ui.ctx(), look.faces, "0"), caret.height()),
```

```diff
diff --git a/src/ui/sql_editor.rs b/src/ui/sql_editor.rs
--- a/src/ui/sql_editor.rs
+++ b/src/ui/sql_editor.rs
@@ -1206,6 +1206,10 @@ fn footer(app: &App, ui: &mut Ui, tab: ConnTabId, id: TabId) {
     let summary = run_summary(sql, locale);
     let note = end_note(sql, locale);
     let version = workspace.server_version.value.clone();
+    // Vim's mode leads, where the editor has the keyboard.
+    let mode = vim_mode(app, ui.ctx(), tab)
+        .and_then(crate::vim::Mode::label)
+        .map(|label| gettext(locale, label).into_owned());
     egui::Panel::bottom(Id::new(("sql-footer", tab.0, id.0)))
         .exact_size(33.0)
         .resizable(false)
@@ -1235,6 +1239,7 @@ fn footer(app: &App, ui: &mut Ui, tab: ConnTabId, id: TabId) {
                 start
                     .into_iter()
                     .chain(end)
+                    .chain(mode.as_deref())
                     .map(|text| width(text) + gap)
                     .sum::<f32>()
                     + width(&cursor)
@@ -1254,6 +1259,10 @@ fn footer(app: &App, ui: &mut Ui, tab: ConnTabId, id: TabId) {
                 x -= gap;
             }
             let mut x = left;
+            if let Some(mode) = &mode {
+                let text = Text::one(&look, role, mode, palette.accent);
+                x += widgets::paint_label(ui, x, center, text) + gap;
+            }
             for piece in start {
                 x += widgets::paint_label(ui, x, center, text(piece)) + gap;
             }
@@ -1263,6 +1272,15 @@ fn footer(app: &App, ui: &mut Ui, tab: ConnTabId, id: TabId) {
         });
 }
 
+/// Vim's mode in the SQL editor in front, while it has the keyboard and
+/// the keybindings are vim's.
+pub fn vim_mode(app: &App, ctx: &egui::Context, tab: ConnTabId) -> Option<crate::vim::Mode> {
+    let vim = app.settings.editor_keys_in(app.look.terminal) == crate::settings::EditorKeys::Vim;
+    let sql = app.workspace(tab)?.active_sql_tab()?;
+    let editor = super::sql_text::editor_id(tab, sql.id);
+    (vim && ctx.memory(|memory| memory.has_focus(editor))).then_some(sql.vim.mode)
+}
+
 /// The right end of the terminal's status line on an editor: "ln 12:21 ·
 /// 5 rows · 14 ms · rolled back", or after a run sent to write "ln 3:1 ·
 /// 12 rows affected · 14 ms · committed".
```

```diff
diff --git a/src/ui/sql_text.rs b/src/ui/sql_text.rs
--- a/src/ui/sql_text.rs
+++ b/src/ui/sql_text.rs
@@ -20,6 +20,7 @@ use crate::settings::Indent;
 use crate::theme::{Look, Palette};
 use crate::typography::{Text, TextRole};
 use crate::ui::widgets;
+use crate::vim;
 
 /// The script's and the line numbers' role: monospace in every look.
 fn role(look: &Look) -> TextRole {
@@ -347,11 +348,25 @@ fn format_script(ui: &Ui, sql_tab: &mut SqlTab, field: &Field<'_>) -> bool {
     };
     let formatted =
         sql::format::format_with(field.dialect, &sql_tab.text, selection, cursor, field.style);
+    // Vim's `=` selects the lines it formats: its cursor, kept, is what
+    // stays, not the lines.
+    let vim_cursor: Option<usize> =
+        ui.data_mut(|data| data.remove_temp(field.id.with("vim format")));
     let Some(formatted) = formatted else {
+        if let Some(at) = vim_cursor {
+            state
+                .cursor
+                .set_char_range(Some(CCursorRange::one(CCursor::new(at))));
+            egui::TextEdit::store_state(ui.ctx(), field.id, state);
+            sql_tab.selected = None;
+        }
         return false;
     };
     let at = |text: &str, byte: usize| CCursorRange::one(CCursor::new(char_index(text, byte)));
-    let before = typed.unwrap_or_else(|| at(&sql_tab.text, sql_tab.cursor));
+    let before = match vim_cursor {
+        Some(cursor) => CCursorRange::one(CCursor::new(cursor)),
+        None => typed.unwrap_or_else(|| at(&sql_tab.text, sql_tab.cursor)),
+    };
     let after = at(&formatted.text, formatted.cursor);
     // The text as typed and the text as formatted, both: undo gives the
     // first back whole, and redo the second.
@@ -433,6 +448,8 @@ struct Edited {
     /// undo, a deletion or an input method's text while it is composed.
     /// Only text the field takes counts (see `edit`).
     typed: bool,
+    /// `:` typed in vim's normal or visual mode.
+    prompt: bool,
 }
 
 impl Edited {
@@ -474,6 +491,7 @@ pub fn show(app: &mut App, ui: &mut Ui, tab: ConnTabId, id: TabId) {
     let wraps = app.settings.wrap;
     let highlight = app.settings.highlight_line;
     let pairs = app.settings.close_pairs;
+    let vim = app.settings.editor_keys_in(look.terminal) == crate::settings::EditorKeys::Vim;
     let Some(workspace) = app.workspace_mut(tab) else {
         return;
     };
@@ -507,6 +525,7 @@ pub fn show(app: &mut App, ui: &mut Ui, tab: ConnTabId, id: TabId) {
         // editor's again.
         hold_escape: sql_tab.completion.as_ref().is_some_and(|list| !list.accept),
         pairs,
+        vim,
     };
     let mut child = ui.new_child(egui::UiBuilder::new().id_salt("text").max_rect(text_pane));
     child.set_clip_rect(text_pane.intersect(ui.clip_rect()));
@@ -571,6 +590,11 @@ pub fn show(app: &mut App, ui: &mut Ui, tab: ConnTabId, id: TabId) {
     if edited.typed {
         app.actions.push(Action::SqlTyped { tab, sql_tab: id });
     }
+    // Vim's `:`: the prompt the terminal look has. (The macOS look has
+    // none.)
+    if edited.prompt && look.terminal {
+        app.actions.push(Action::EditorCommand { tab, sql_tab: id });
+    }
     let spot = Spot {
         tab,
         sql_tab: id,
@@ -751,6 +775,9 @@ struct Field<'a> {
     indent: Indent,
     /// Long lines wrap at the pane's width.
     wraps: bool,
+    /// The keybindings are vim's: the field has normal, insert and visual
+    /// modes.
+    vim: bool,
 }
 
 /// How Format writes, as the settings have it in `look`: its keyword case
@@ -1216,17 +1243,322 @@ fn close_pairs(ui: &mut Ui, sql_tab: &mut SqlTab, field: &Field<'_>) -> bool {
     false
 }
 
+/// What vim does with one of the frame's events.
+enum Read {
+    /// The field's, as it is.
+    Keep,
+    /// Nobody's: a key's press whose character comes as text.
+    Drop,
+    /// Vim's.
+    Take(Vec<vim::Input>),
+}
+
+/// How vim in `mode` reads `event`. In insert mode it takes Esc (and
+/// Ctrl+[, vim's other Esc) alone, unless `held` (an open completion list
+/// takes it). Out of it, the characters typed and the keys that move or
+/// edit; the field keeps its chords (copy, paste, select all, undo), but
+/// not vim's deletions of insert mode nor Tab, which type nothing here. A
+/// held Esc is not normal mode's: the keyboard does not leave with it.
+fn vim_input(event: &egui::Event, mode: vim::Mode, held: bool) -> Read {
+    use egui::{Event, Key};
+    use vim::Input;
+    let escape = |key: &Key, modifiers: &egui::Modifiers| {
+        *key == Key::Escape || (modifiers.ctrl && *key == Key::OpenBracket)
+    };
+    if mode == vim::Mode::Insert {
+        return match event {
+            Event::Key {
+                key,
+                pressed: true,
+                modifiers,
+                ..
+            } if escape(key, modifiers) && !held => Read::Take(vec![Input::Escape]),
+            _ => Read::Keep,
+        };
+    }
+    match event {
+        Event::Text(text) | Event::Ime(egui::ImeEvent::Commit(text)) => Read::Take(
+            text.chars()
+                .filter(|c| !matches!(c, '\n' | '\r'))
+                .map(Input::Char)
+                .collect(),
+        ),
+        Event::Ime(_) => Read::Drop,
+        Event::Key {
+            key,
+            pressed,
+            repeat,
+            modifiers,
+            ..
+        } => {
+            if !pressed {
+                return Read::Keep;
+            }
+            if escape(key, modifiers) {
+                return if *repeat {
+                    Read::Drop
+                } else {
+                    Read::Take(vec![Input::Escape])
+                };
+            }
+            if modifiers.ctrl && modifiers.alt {
+                // AltGr on Windows: its character comes as text.
+                return Read::Drop;
+            }
+            if modifiers.ctrl && !modifiers.mac_cmd && *key == Key::R {
+                return Read::Take(vec![Input::Redo]);
+            }
+            // The field's chords that delete (a word, to a line's end) have
+            // nothing to delete in normal mode.
+            let deletes = matches!(key, Key::Backspace | Key::Delete)
+                || (modifiers.ctrl && matches!(key, Key::W | Key::H | Key::U | Key::K));
+            if modifiers.any() && deletes {
+                return Read::Drop;
+            }
+            if modifiers.command || modifiers.ctrl {
+                return Read::Keep;
+            }
+            let input = match key {
+                Key::Enter => Input::Enter,
+                Key::Backspace => Input::Backspace,
+                Key::Delete => Input::Delete,
+                Key::ArrowLeft => Input::Left,
+                Key::ArrowRight => Input::Right,
+                Key::ArrowUp => Input::Up,
+                Key::ArrowDown => Input::Down,
+                Key::Home => Input::Home,
+                Key::End => Input::End,
+                // The field pages, as in every mode.
+                Key::PageUp | Key::PageDown => return Read::Keep,
+                _ => return Read::Drop,
+            };
+            Read::Take(vec![input])
+        }
+        _ => Read::Keep,
+    }
+}
+
+/// Whether the field, reading `event`, may change the script or the
+/// cursor.
+fn edits(event: &egui::Event) -> bool {
+    matches!(
+        event,
+        egui::Event::Text(_)
+            | egui::Event::Paste(_)
+            | egui::Event::Cut
+            | egui::Event::Ime(_)
+            | egui::Event::Key { pressed: true, .. }
+    )
+}
+
+/// Where the keybindings are vim's, before the field reads the frame: in
+/// normal and visual mode the frame's keys go to vim, whose edits go in
+/// as edits from outside, and whose format Format carries out; in insert
+/// mode Esc goes back to normal mode. Normal mode's cursor stays on a
+/// character. Returns whether the script or the cursor changed, and
+/// whether `:` was typed. (Vim keeps its own undo history: the field's
+/// counts cursor moves.)
+///
+/// The frame's events are read in order, each in the mode the ones before
+/// it left. A key of vim's after one the field is to read (typing, or a
+/// chord), or after a format, which Format carries out after this, would
+/// read the script before them: it and what follows wait for the next
+/// frame, where the keymap has read its own already. After Esc out of the
+/// editor or `:`, the frame's keys are nobody's.
+fn vim_keys(ui: &mut Ui, sql_tab: &mut SqlTab, field: &Field<'_>) -> (bool, bool) {
+    let later_id = field.id.with("vim later");
+    if !field.vim || !ui.memory(|memory| memory.has_focus(field.id)) {
+        ui.data_mut(|data| data.remove_temp::<Vec<egui::Event>>(later_id));
+        return (false, false);
+    }
+    // A press leaves visual mode: the pointer selects as it does anywhere,
+    // and what visual mode selected goes (a press on the text sets its own).
+    let pressed = sql_tab.vim.mode.is_visual() && ui.input(|input| input.pointer.any_pressed());
+    if pressed {
+        sql_tab.vim.to_normal();
+    }
+    let waited: Vec<egui::Event> = ui
+        .data_mut(|data| data.remove_temp(later_id))
+        .unwrap_or_default();
+    let events = ui.input_mut(|input| {
+        let mut events = waited;
+        events.append(&mut input.events);
+        events
+    });
+    let mut state = egui::TextEdit::load_state(ui.ctx(), field.id).unwrap_or_default();
+    let mut changed = false;
+    if pressed {
+        let head = CCursorRange::one(CCursor::new(sql_tab.vim.head()));
+        state.cursor.set_char_range(Some(head));
+        changed = true;
+    }
+    if sql_tab.vim.mode == vim::Mode::Normal
+        && let Some(range) = state.cursor.char_range()
+        && range.is_empty()
+    {
+        // A click or an undo may leave it after a line's last character.
+        let at = vim::clamp(&sql_tab.text, range.primary.index.0);
+        if at != range.primary.index.0 {
+            state
+                .cursor
+                .set_char_range(Some(CCursorRange::one(CCursor::new(at))));
+            changed = true;
+        }
+    }
+    let mut prompt = false;
+    let register_id = Id::new("vim register");
+    let mut register: vim::Register = ui
+        .data(|data| data.get_temp(register_id))
+        .unwrap_or_default();
+    let time = ui.input(|input| input.time);
+    let unit = indent_unit(field.indent);
+    let (mut kept, mut later) = (Vec::new(), Vec::new());
+    // The field has events of this frame to read first.
+    let mut field_first = false;
+    let mut queue = events.into_iter();
+    while let Some(event) = queue.next() {
+        let mode = sql_tab.vim.mode;
+        // A cut or a paste ends visual mode: the selection is the field's
+        // to take.
+        if mode.is_visual() && matches!(event, egui::Event::Cut | egui::Event::Paste(_)) {
+            sql_tab.vim.to_normal();
+        }
+        let inputs = match vim_input(&event, sql_tab.vim.mode, field.hold_escape) {
+            Read::Keep => {
+                field_first |= edits(&event);
+                kept.push(event);
+                continue;
+            }
+            Read::Drop => continue,
+            Read::Take(inputs) => inputs,
+        };
+        if field_first {
+            later.push(event);
+            later.extend(queue.by_ref());
+            break;
+        }
+        let (mut waits, mut done) = (false, false);
+        for input in inputs {
+            let cursor = if sql_tab.vim.mode.is_visual() {
+                sql_tab.vim.head()
+            } else {
+                state.cursor.char_range().map_or_else(
+                    || char_index(&sql_tab.text, sql_tab.cursor),
+                    |range| range.primary.index.0,
+                )
+            };
+            let step = sql_tab
+                .vim
+                .key(&sql_tab.text, cursor, input, &mut register, unit);
+            if let Some(text) = &step.text {
+                // Undo puts the cursor back, not a selection.
+                let before = CCursorRange::one(CCursor::new(cursor));
+                let range = vim_range(text, &step, sql_tab.vim.mode);
+                let before = (before, std::mem::replace(&mut sql_tab.text, text.clone()));
+                outside_edit(&mut state, time, &before, &(range, text.clone()));
+            } else {
+                let range = vim_range(&sql_tab.text, &step, sql_tab.vim.mode);
+                state.cursor.set_char_range(Some(range));
+            }
+            changed = true;
+            match step.then {
+                Some(vim::Then::Format(lines)) => {
+                    // Format formats the statements a selection touches,
+                    // or with none every one.
+                    let whole = lines.start == 0 && lines.end >= sql_tab.text.chars().count();
+                    let range = if whole {
+                        CCursorRange::one(CCursor::new(lines.start))
+                    } else {
+                        CCursorRange::two(CCursor::new(lines.end), CCursor::new(lines.start))
+                    };
+                    state.cursor.set_char_range(Some(range));
+                    sql_tab.format = true;
+                    // Format's undo, and where it leaves nothing to do,
+                    // put the cursor back, not the lines.
+                    ui.data_mut(|data| data.insert_temp(field.id.with("vim format"), cursor));
+                    waits = true;
+                }
+                Some(vim::Then::Prompt) => {
+                    prompt = true;
+                    done = true;
+                }
+                Some(vim::Then::Leave) => {
+                    ui.memory_mut(|memory| memory.surrender_focus(field.id));
+                    done = true;
+                }
+                None => {}
+            }
+        }
+        if done {
+            sql_tab.vim.to_normal();
+            break;
+        }
+        if waits {
+            later.extend(queue.by_ref());
+            break;
+        }
+    }
+    ui.data_mut(|data| data.insert_temp(register_id, register));
+    if !later.is_empty() {
+        ui.data_mut(|data| data.insert_temp(later_id, later));
+        ui.ctx().request_repaint();
+    }
+    if changed {
+        if let Some(range) = state.cursor.char_range() {
+            sql_tab.cursor = byte_offset(&sql_tab.text, range.primary.index.0);
+        }
+        egui::TextEdit::store_state(ui.ctx(), field.id, state);
+    }
+    ui.input_mut(|input| input.events = kept);
+    (changed, prompt)
+}
+
+/// The field's cursor for vim's `step` in `text`, now in `mode`: the
+/// cursor, or in visual mode the selection, whose ends' characters are in
+/// it (by line, whole lines).
+fn vim_range(text: &str, step: &vim::Step, mode: vim::Mode) -> CCursorRange {
+    let one = |at: usize| CCursorRange::one(CCursor::new(at));
+    let Some(anchor) = step.anchor else {
+        return one(step.cursor);
+    };
+    let chars: Vec<char> = text.chars().collect();
+    let length = chars.len();
+    let (cursor, forward) = (step.cursor, step.cursor >= anchor);
+    let (low, high) = (anchor.min(cursor), anchor.max(cursor));
+    let (start, end) = if mode == vim::Mode::VisualLine {
+        let start = chars[..low.min(length)]
+            .iter()
+            .rposition(|c| *c == '\n')
+            .map_or(0, |at| at + 1);
+        let end = chars[high.min(length)..]
+            .iter()
+            .position(|c| *c == '\n')
+            .map_or(length, |at| high + at);
+        (start, end)
+    } else {
+        (low, (high + 1).min(length))
+    };
+    if forward {
+        CCursorRange::two(CCursor::new(start), CCursor::new(end))
+    } else {
+        CCursorRange::two(CCursor::new(end), CCursor::new(start))
+    }
+}
+
 /// The text field itself: edits the tab's text and reports its cursor.
 fn edit(ui: &mut Ui, sql_tab: &mut SqlTab, field: &Field<'_>) -> Edited {
     let inserted = insert_completion(ui.ctx(), field.id, sql_tab);
     let focus = std::mem::take(&mut sql_tab.focus_editor);
+    // Vim's normal and visual modes take the frame's keys before the
+    // field, and the edits that follow, before Tab and the pairs read it.
+    let (vimmed, prompt) = vim_keys(ui, sql_tab, field);
     // Before the field is drawn, so this frame shows the formatted text.
     // An indent is an outside edit too: what the field shows, and where
     // its caret goes, follow as after Format.
     let indented = indent_script(ui, sql_tab, field);
     let paired = close_pairs(ui, sql_tab, field);
     let formatted = std::mem::take(&mut sql_tab.format) && format_script(ui, sql_tab, field);
-    let formatted = formatted || indented || paired;
+    let formatted = formatted || indented || paired || vimmed;
     // The cursor put somewhere from outside (an error's mark): set before
     // the field is drawn, which then draws it there.
     let moved = sql_tab.go_to.take().is_some_and(|at| {
@@ -1267,6 +1599,13 @@ fn edit(ui: &mut Ui, sql_tab: &mut SqlTab, field: &Field<'_>) -> Edited {
     } else {
         f32::INFINITY
     };
+    // Out of insert mode vim's cursor is a block, drawn below: egui's
+    // caret is not.
+    let block = field.vim && sql_tab.vim.mode != vim::Mode::Insert;
+    let caret_stroke = ui.visuals().text_cursor.stroke;
+    if block {
+        ui.visuals_mut().text_cursor.stroke.color = Color32::TRANSPARENT;
+    }
     let output = egui::TextEdit::multiline(&mut sql_tab.text)
         .id(field.id)
         .font(font)
@@ -1277,6 +1616,26 @@ fn edit(ui: &mut Ui, sql_tab: &mut SqlTab, field: &Field<'_>) -> Edited {
         .desired_rows(rows as usize)
         .layouter(&mut layouter)
         .show(ui);
+    ui.visuals_mut().text_cursor.stroke = caret_stroke;
+    if block && output.response.has_focus() {
+        let at = if sql_tab.vim.mode.is_visual() {
+            sql_tab.vim.head()
+        } else {
+            output
+                .state
+                .cursor
+                .char_range()
+                .map_or(0, |range| range.primary.index.0)
+        };
+        crate::ui::cell_editor::block_cursor_at(
+            ui,
+            &output,
+            CCursor::new(at),
+            role(field.look),
+            field.look,
+            field.palette,
+        );
+    }
     crate::ui::focus::region(ui, crate::ui::focus::Region::Editor, output.response.rect);
     crate::ui::focus::claim(ui, crate::ui::focus::Region::Editor, &output.response);
     // The script fills its pane: the caret says where the keyboard is.
@@ -1331,7 +1690,9 @@ fn edit(ui: &mut Ui, sql_tab: &mut SqlTab, field: &Field<'_>) -> Edited {
         // before the row went in or the script was formatted.
         ui.ctx().request_repaint();
     }
-    if field.hold_escape && output.response.has_focus() {
+    // Vim's Esc is its own: insert mode's goes to normal mode, and normal
+    // mode's leaves the editor when `vim_keys` says so.
+    if (field.hold_escape || field.vim) && output.response.has_focus() {
         // The field set its own filter while it drew (the arrows and Tab).
         // egui replaces the whole filter, so those are named again. The
         // filter is read at the start of the next pass, so holding Esc takes
@@ -1354,7 +1715,13 @@ fn edit(ui: &mut Ui, sql_tab: &mut SqlTab, field: &Field<'_>) -> Edited {
     // line, drawn before the editor, say where the cursor is a frame later
     // (egui draws one after any event).
     if let Some(range) = output.state.cursor.range(&output.galley) {
-        sql_tab.cursor = byte_offset(&sql_tab.text, range.primary.index.0);
+        // In vim's visual mode the selection runs one past the cursor.
+        let cursor = if field.vim && sql_tab.vim.mode.is_visual() {
+            sql_tab.vim.head()
+        } else {
+            range.primary.index.0
+        };
+        sql_tab.cursor = byte_offset(&sql_tab.text, cursor);
         let [start, end] = range.sorted_cursors();
         let byte = |cursor: CCursor| byte_offset(&sql_tab.text, cursor.index.0);
         sql_tab.selected = (start != end).then(|| byte(start)..byte(end));
@@ -1362,6 +1729,7 @@ fn edit(ui: &mut Ui, sql_tab: &mut SqlTab, field: &Field<'_>) -> Edited {
     let typed = output.response.changed() && ui.input(|input| input.events.iter().any(is_typed));
     Edited {
         typed,
+        prompt,
         focused: output.response.has_focus(),
         origin: output.galley_pos,
         caret,
```

```diff
diff --git a/src/ui/workspace.rs b/src/ui/workspace.rs
--- a/src/ui/workspace.rs
+++ b/src/ui/workspace.rs
@@ -2024,8 +2024,11 @@ fn command_line(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
             // keyboard for any other reason the prompt is closed: it is
             // open only while it is typed into.
             let enter = ui.input(|input| input.key_pressed(egui::Key::Enter));
+            let escape = ui.input(|input| input.key_pressed(egui::Key::Escape));
             if response.lost_focus() && enter {
                 actions.push(Action::RunCommand(tab));
+            } else if response.lost_focus() && escape {
+                actions.push(Action::EscapeCommand(tab));
             } else if !response.has_focus() {
                 actions.push(Action::CloseCommand(tab));
             }
@@ -2150,6 +2153,10 @@ fn status_line(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
     let read_only = workspace.access == tabletist_db::Access::ReadOnly;
     let editor = super::sql_editor::status_summary(app, tab);
     let on_editor = editor.is_some();
+    // Vim's mode, where the editor has the keyboard: the line starts with
+    // it, as vim's does, and out of normal mode Esc goes to normal mode.
+    let vim_mode = super::sql_editor::vim_mode(app, ui.ctx(), tab);
+    let to_normal = vim_mode.is_some_and(|mode| mode != crate::vim::Mode::Normal);
     // The Structure view of the table in front shows: the line names
     // that view's keys in the rows' place. The keys that choose what
     // labels the table's rows are named where there is anything to
@@ -2310,7 +2317,7 @@ fn status_line(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
                 "run",
                 "run all",
                 "cancel",
-                "leave editor",
+                if to_normal { "normal" } else { "leave editor" },
                 "tables",
                 "complete",
             ]
@@ -2512,8 +2519,23 @@ fn status_line(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
             if on_editor {
                 // The toolbar says the transaction is read-only; the line
                 // has no editing keys to strike out.
-                let shown = fit(rect.right() - 12.0 - summary_width - gap);
-                let at = (rect.left() + 12.0, y);
+                let mut left = rect.left() + 12.0;
+                if let Some(mode) =
+                    vim_mode.and_then(|mode| mode.label().map(|label| (mode, label)))
+                {
+                    let (mode, label) = mode;
+                    let color = if mode == crate::vim::Mode::Insert {
+                        palette.success
+                    } else {
+                        palette.magenta
+                    };
+                    let label = gettext(locale, label);
+                    let text = Text::one(&look, TextRole::OModeLine, &label, color);
+                    left += widgets::paint_label(ui, left, y, text) + gap;
+                }
+                let shown =
+                    fit(rect.right() - 12.0 - summary_width - gap - (left - rect.left() - 12.0));
+                let at = (left, y);
                 widgets::key_hints(ui, at, &hints[..shown], gap, &look, &palette);
                 widgets::paint_text_right(
                     ui,
```

- [ ] **Step 4: Run the tests to see them pass**

Run: `cargo test --locked --lib -- vim_tests sql_editor::tests app::tests::editing sql_text::tests ui::tests sql_results editor_results_tests every_row_tests format_settings_tests grid_keys_tests pane_keys_tests`
Expected: PASS. Then the four checks above.

- [ ] **Step 5: Try it by hand**

Not a test. Run the app in the Omarchy look against the Bookshop demo database, open a SQL tab, and type a query; Esc, then `dd`, `p`, `u`, Ctrl+R, `V` and Ctrl+Enter, `==`, `:ro`, Esc twice. The status line shows the mode, the cursor is a block in normal mode, and nothing typed in normal mode lands in the script.

- [ ] **Step 6: Commit**

```bash
git add src/app.rs src/app/editing.rs src/model.rs src/testing.rs src/ui/cell_editor.rs src/ui/editor_results_tests.rs src/ui/every_row_tests.rs src/ui/format_settings_tests.rs src/ui/grid_keys_tests.rs src/ui/mod.rs src/ui/pane_keys_tests.rs src/ui/sql_editor.rs src/ui/sql_results.rs src/ui/sql_text.rs src/ui/vim_tests.rs src/ui/workspace.rs
git commit -m "Give the SQL editor vim's modes where Settings › Editor's keybindings are vim"
```

---

### Task 3: The Keybindings row

**Files:**
- Modify: `src/settings.rs`, `src/model.rs`, `src/ui/settings/terminal.rs`
- Modify (tests): `src/ui/vim_tests.rs`, `src/ui/editor_settings_tests.rs`, `src/ui/settings_access_tests.rs`, `src/ui/every_row_tests.rs`, `src/settings.rs`

- [ ] **Step 1: Write the failing tests**

```diff
diff --git a/src/settings.rs b/src/settings.rs
--- a/src/settings.rs
+++ b/src/settings.rs
@@ -4649,7 +4649,6 @@ bar = \"label\"  # tint | label
                 sections,
                 &OptionId::FORMAT[..],
                 &OptionId::HISTORY[..],
-                &OptionId::EDITOR_LATER[..],
             ]
             .concat()
             .into_iter()
```

```diff
diff --git a/src/ui/editor_settings_tests.rs b/src/ui/editor_settings_tests.rs
--- a/src/ui/editor_settings_tests.rs
+++ b/src/ui/editor_settings_tests.rs
@@ -527,9 +527,8 @@ fn the_sheet_sets_each_editor_option_with_a_click() {
     ] {
         assert!(shows(&mut harness, words), "{words}");
     }
-    // The rows of what is not built yet are not there.
-    assert!(!shows(&mut harness, "Keybindings"));
     let saves = settings_saves(&harness);
+    harness.click("Keybindings: Vim");
     harness.click("New tabs run in: Follow connection");
     harness.click("Row limit");
     harness.click("Row limit 500");
@@ -548,7 +547,8 @@ fn the_sheet_sets_each_editor_option_with_a_click() {
     assert_eq!(settings.autocomplete, Autocomplete::Off);
     assert_eq!(settings.line_numbers, Some(LineNumbers::Off));
     assert!(!settings.highlight_line);
-    assert_eq!(settings_saves(&harness), saves + 7);
+    assert_eq!(settings.editor_keys, Some(crate::settings::EditorKeys::Vim));
+    assert_eq!(settings_saves(&harness), saves + 8);
     // On again, the numbers are each line's own.
     harness.click("Line numbers");
     assert_eq!(
@@ -639,7 +639,8 @@ fn the_screen_sets_each_editor_option_with_its_keys() {
         read_only.as_str(),
         "per tab",
         "caps results you browse · show all and exports get every row",
-        "a selection always wins",
+        // Vim's, the look's keys: a selection is visual mode's.
+        "a visual selection always wins",
         "connections that allow writes",
         "schema objects",
     ] {
@@ -660,6 +661,9 @@ fn the_screen_sets_each_editor_option_with_its_keys() {
     // Writes without WHERE asked about no more, with space.
     type_key(&mut harness, Key::J, "j");
     harness.press(Key::Space, Modifiers::NONE);
+    // The standard keys in place of the look's vim.
+    type_key(&mut harness, Key::J, "j");
+    type_key(&mut harness, Key::L, "l");
     // Line numbers from relative to absolute; four spaces; completion on
     // the key; no closing brackets and wrapped lines, with space; keywords
     // from the look's lower case to upper; leading commas, with space.
@@ -691,7 +695,11 @@ fn the_screen_sets_each_editor_option_with_its_keys() {
     assert!(!settings.close_pairs);
     assert!(settings.wrap);
     assert!(!settings.confirm_unsafe_writes);
-    assert_eq!(settings_saves(&harness), saves + 12);
+    assert_eq!(
+        settings.editor_keys,
+        Some(crate::settings::EditorKeys::Standard)
+    );
+    assert_eq!(settings_saves(&harness), saves + 13);
     // The file has each of them, and R puts the cursor's back.
     let text = harness.app.settings_file.text.clone();
     assert!(
```

```diff
diff --git a/src/ui/every_row_tests.rs b/src/ui/every_row_tests.rs
--- a/src/ui/every_row_tests.rs
+++ b/src/ui/every_row_tests.rs
@@ -379,7 +379,7 @@ fn the_screen_has_the_confirm_row_after_the_run_key_and_toggles_it() {
     let top = |harness: &Harness, words: &str| harness.painted_rect(words).expect(words).top();
     let row = "confirm write without where";
     // After the run key's row, by its hint.
-    assert!(top(&harness, "a selection always wins") < top(&harness, row));
+    assert!(top(&harness, "a visual selection always wins") < top(&harness, row));
     assert!(top(&harness, row) < top(&harness, "line numbers"));
     assert!(shows(&mut harness, "connections that allow writes"));
     // Down to it from the first row: new tabs, limit, timeout, run key.
```

```diff
diff --git a/src/ui/settings_access_tests.rs b/src/ui/settings_access_tests.rs
--- a/src/ui/settings_access_tests.rs
+++ b/src/ui/settings_access_tests.rs
@@ -104,14 +104,12 @@ fn every_rows_controls_are_described_by_its_small_print() {
             }
         }
         // Every row that has small print, as each look lists its rows:
-        // the terminal look has no interface size, and the Editor pane has
-        // no row yet for some of its options.
+        // the terminal look has no interface size.
         described.sort_by_key(|option| OptionId::ALL.iter().position(|all| all == option));
         let expected: Vec<OptionId> = OptionId::ALL
             .into_iter()
             .filter(|option| small_print(*option).is_some())
             .filter(|option| !(look.terminal && *option == OptionId::InterfaceSize))
-            .filter(|option| !OptionId::EDITOR_LATER.contains(option))
             .collect();
         assert_eq!(described, expected, "in {}", look.name);
     }
```

```diff
diff --git a/src/ui/vim_tests.rs b/src/ui/vim_tests.rs
--- a/src/ui/vim_tests.rs
+++ b/src/ui/vim_tests.rs
@@ -4,12 +4,14 @@
 
 use egui::{Key, Modifiers};
 
-use crate::model::{Action, ConnTabId, RunMode};
+use crate::model::{Action, ConnTabId, RunMode, SettingsPane};
 use crate::settings::{EditorKeys, OptionValue};
 use crate::testing::Harness;
 use crate::theme::Look;
 use crate::vim::Mode;
 
+use super::appearance_settings_tests::shows;
+
 /// A connected fixture in `look`, with a new SQL editor that has the
 /// keyboard and holds `text`, its cursor at the end. A new tab types: in
 /// vim it is in insert mode.
@@ -449,3 +451,41 @@ fn ct_to_the_next_character_changes_what_is_between() {
     assert_eq!(mode(&harness, tab), Mode::Insert);
     assert_eq!(text(&harness, tab), "select count(n)");
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
+fn the_sheet_leads_typing_with_the_keybindings_row() {
+    let mut harness = Harness::with_size(egui::vec2(1440.0, 1400.0));
+    harness.set_look(Look::macos());
+    on_the_pane(&mut harness);
+    let top = |harness: &Harness, words: &str| harness.painted_rect(words).expect(words).top();
+    assert!(top(&harness, "TYPING") < top(&harness, "Keybindings"));
+    assert!(top(&harness, "Keybindings") < top(&harness, "Indent"));
+    harness.click("Keybindings: Vim");
+    assert_eq!(harness.app.settings.editor_keys, Some(EditorKeys::Vim));
+}
+
+#[test]
+fn the_screen_leads_typing_with_the_keybindings_row_and_its_hint() {
+    let mut harness = Harness::with_size(egui::vec2(1920.0, 1080.0));
+    harness.set_look(Look::omarchy());
+    on_the_pane(&mut harness);
+    for words in ["keybindings", "-- INSERT -- shown in the status line"] {
+        assert!(shows(&mut harness, words), "{words}");
+    }
+    let top = |harness: &Harness, words: &str| harness.painted_rect(words).expect(words).top();
+    assert!(top(&harness, "typing") < top(&harness, "keybindings"));
+    assert!(top(&harness, "keybindings") < top(&harness, "line numbers"));
+    // Vim first, as the board draws them, and the look's.
+    assert!(top(&harness, "vim") == top(&harness, "standard"));
+    let left = |harness: &Harness, words: &str| harness.painted_rect(words).expect(words).left();
+    assert!(left(&harness, "vim") < left(&harness, "standard"));
+}
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test --locked --lib -- vim_tests editor_settings_tests settings every_row_tests`
Expected: FAIL. Six tests fail: the two new ones find no Keybindings row; `the_sheet_sets_each_editor_option_with_a_click` and `the_screen_sets_each_editor_option_with_its_keys` find no row to set; the ⌘↩ runs row's Omarchy hint is not yet "a visual selection always wins"; and `every_option_is_stored_under_its_own_key_and_starts_at_the_default` finds Keybindings in no section.

- [ ] **Step 3: Show the row**

```diff
diff --git a/src/model.rs b/src/model.rs
--- a/src/model.rs
+++ b/src/model.rs
@@ -3068,6 +3068,7 @@ impl SettingsPane {
                 SettingsRow::Option(OptionId::Timeout),
                 SettingsRow::Option(OptionId::RunScope),
                 SettingsRow::Option(OptionId::ConfirmUnsafeWrites),
+                SettingsRow::Option(OptionId::EditorKeys),
                 SettingsRow::Option(OptionId::LineNumbers),
                 SettingsRow::Option(OptionId::Indent),
                 SettingsRow::Option(OptionId::Autocomplete),
```

```diff
diff --git a/src/settings.rs b/src/settings.rs
--- a/src/settings.rs
+++ b/src/settings.rs
@@ -1074,9 +1074,6 @@ impl OptionId {
         Self::Commas,
         Self::HistoryKeep,
     ];
-    /// The Editor pane's options whose features are not built yet: no row
-    /// draws them, and their keys are read, kept and reset.
-    pub const EDITOR_LATER: [OptionId; 1] = [Self::EditorKeys];
     /// The Editor pane's Running queries section, in both looks.
     pub const RUNNING: [OptionId; 5] = [
         Self::NewTab,
@@ -1086,7 +1083,8 @@ impl OptionId {
         Self::ConfirmUnsafeWrites,
     ];
     /// Its Typing section on macOS and Windows.
-    pub const TYPING: [OptionId; 6] = [
+    pub const TYPING: [OptionId; 7] = [
+        Self::EditorKeys,
         Self::Indent,
         Self::Autocomplete,
         Self::ClosePairs,
@@ -1096,7 +1094,8 @@ impl OptionId {
     ];
     /// The terminal look's, as its board orders them, with no row for the
     /// cursor's line.
-    pub const TERMINAL_TYPING: [OptionId; 5] = [
+    pub const TERMINAL_TYPING: [OptionId; 6] = [
+        Self::EditorKeys,
         Self::LineNumbers,
         Self::Indent,
         Self::Autocomplete,
```

```diff
diff --git a/src/ui/settings/terminal.rs b/src/ui/settings/terminal.rs
--- a/src/ui/settings/terminal.rs
+++ b/src/ui/settings/terminal.rs
@@ -1571,9 +1571,21 @@ fn hint(ui: &Ui, option: OptionId, settings: &Settings, skin: &Skin) -> Text {
         OptionId::RowLimit => {
             dim(&skin.say("caps results you browse · show all and exports get every row"))
         }
-        OptionId::RunScope => dim(&skin.say("a selection always wins")),
+        // A selection is visual mode's where the keys are vim's.
+        OptionId::RunScope => {
+            if settings.editor_keys_in(true) == crate::settings::EditorKeys::Vim {
+                dim(&skin.say("a visual selection always wins"))
+            } else {
+                dim(&skin.say("a selection always wins"))
+            }
+        }
         OptionId::ConfirmUnsafeWrites => dim(&skin.say("connections that allow writes")),
-        OptionId::EditorKeys => dim(&skin.say("-- INSERT -- shown in the status line")),
+        // Vim's own words for the mode, in its case.
+        OptionId::EditorKeys => Text::new(look).add(role, "-- INSERT --", palette.dim).add(
+            role,
+            &format!(" {}", skin.say("shown in the status line")),
+            palette.dim,
+        ),
         OptionId::Autocomplete => dim(&skin.say("schema objects")),
         OptionId::HistoryKeep => {
             dim(&skin.say("per connection · sql, saves, filters, agents · never results"))
```

- [ ] **Step 4: Run the tests to see them pass**

Run: `cargo test --locked --lib -- vim_tests editor_settings_tests settings every_row_tests ui::settings_access_tests`
Expected: PASS. Then the four checks above.

- [ ] **Step 5: Compare with the boards by eye**

Not a test, and nothing of it is committed. Shoot Settings › Editor in both looks (`Harness::for_shots`, 1440 × 1100 on macOS and 1920 × 1080 on Omarchy) and set it beside the two "Settings, Editor" boards' Typing rows. Remove the scene before committing.

- [ ] **Step 6: Commit**

```bash
git add src/model.rs src/settings.rs src/ui/editor_settings_tests.rs src/ui/every_row_tests.rs src/ui/settings/terminal.rs src/ui/settings_access_tests.rs src/ui/vim_tests.rs
git commit -m "Show Settings › Editor's Keybindings row in both looks"
```

---

### Task 4: The website

**Files:**
- Modify: `docs/_reference/settings-and-files.md`, `docs/_guide/sql-editor.md`, `docs/_guide/omarchy.md`

- [ ] **Step 1: Say what vim does**

```diff
diff --git a/docs/_guide/omarchy.md b/docs/_guide/omarchy.md
--- a/docs/_guide/omarchy.md
+++ b/docs/_guide/omarchy.md
@@ -20,7 +20,9 @@ dialogs and of the lists that open in an editor.
 
 A letter does its job when no text field has the keyboard. While a field
 has it, letters are typed into the field, and Esc gives the keyboard
-back.
+back. With vim keybindings (the look's own), the SQL editor has vim's
+modes besides: there Esc goes from insert mode to normal mode, and a
+second Esc gives the keyboard back.
 
 ### Everywhere
 
@@ -33,7 +35,7 @@ back.
 | Ctrl+Shift+D | Switch database, where the server has another |
 | Alt+1 to 9 | Go to that tab |
 | Ctrl+Shift+[ and ] | Previous and next tab |
-| Ctrl+W | Close the tab. While you type in a field it deletes a word instead |
+| Ctrl+W | Close the tab. While you type in a field it deletes a word instead; in the SQL editor's normal mode it does neither |
 | Ctrl+B | Show or hide the sidebar |
 | Ctrl+H, Ctrl+L | Move between the parts of the window |
 | Ctrl+K, Ctrl+J | In a SQL editor: up to the script, down to its results |
@@ -269,12 +271,13 @@ A save can ask you something first. Each question has its own keys:
 | Ctrl+Enter | Run the statement at the cursor, the whole script or the selection, as Settings › Editor says |
 | Ctrl+Shift+Enter | Run all |
 | Ctrl+C | Cancel the run, while one runs. With none running it copies |
-| Esc | Leave the editor: the letters below are read from then on |
+| Esc | In insert mode, normal mode. In normal mode, leave the editor: the letters below are read from then on |
+| vim's keys | In normal and visual mode: moves, `d` `c` `y`, `p`, `u` and Ctrl+R, `v` and `V`, as the [SQL Editor guide]({% link _guide/sql-editor.md %}#vim) lists them |
 | Ctrl+J | Give the keyboard to the results. Ctrl+K gives it back to the script |
 | `]e`, `[e` | Put the cursor where the last run's error is, while the script is the one that ran |
-| `=` | Format, with the keyboard out of the text |
+| `=` | Format, with the keyboard out of the text. In normal mode, `==` formats the statement at the cursor and `gg=G` all of it |
 | Tab, Shift+Tab | Indent, outdent, in the text: at the cursor, or every line of the selection |
-| `:ro`, `:rw` | Read-only and read-write runs, with the keyboard out of the text |
+| `:ro`, `:rw` | Read-only and read-write runs, from normal mode or with the keyboard out of the text |
 | Ctrl+Space | Complete |
 | `y`; `n`, Enter or Esc | When the status line asks before a run that changes every row of a table (an `UPDATE` or a `DELETE` without `WHERE`, a `TRUNCATE`): run it; do not |
 
```

```diff
diff --git a/docs/_guide/sql-editor.md b/docs/_guide/sql-editor.md
--- a/docs/_guide/sql-editor.md
+++ b/docs/_guide/sql-editor.md
@@ -92,7 +92,8 @@ back.
 
 ## Format
 
-**Format** (Cmd/Ctrl+Shift+F; in the Omarchy look Esc, then `=`) lays
+**Format** (Cmd/Ctrl+Shift+F; in the Omarchy look `=` with the keyboard
+out of the text, and with [vim keys](#vim) `==` and `gg=G`) lays
 queries out in river style, with reserved words in the case
 **Settings › Editor** sets (upper case on macOS and Windows and lower
 case in the Omarchy look until you choose, or as you typed them), the
@@ -111,6 +112,44 @@ or delete with Backspace while the pair is empty. Typed over a selection,
 the two go round it. **Settings › Editor** turns this off. There, Wrap
 wraps long lines at the editor's width, each numbered once.
 
+## Vim
+
+With **Settings › Editor › Keybindings** set to Vim (the Omarchy look's
+own), the editor has vim's modes. A new tab is in insert mode, where you
+type as in any editor and the status line (on macOS and Windows, the
+footer) says `-- INSERT --`. Esc (or Ctrl+[) goes to normal mode, where
+the cursor is a block and keys are commands; Esc there gives the keyboard
+up, to the results and the sidebar, where the keys of the results and of
+the error marks (`]e`, `[e`) work. Coming back, the editor is in the mode
+it was left in.
+
+In normal mode:
+
+- Moves, each with a count before it: `h` `j` `k` `l`, `w` `b` `e` and
+  `W` `B` `E`, `0` `^` `$`, `gg` and `G` (to a line with a count), `f`
+  `F` `t` `T` with `;` and `,`, `%` to the matching bracket, and `{` `}`
+  to the next blank line.
+- Into insert mode: `i` `a` `I` `A` `o` `O`, and `s` `S` `C`.
+- `d`, `c` and `y` with a move (`dw`, `c$`, `y2j`), or twice for whole
+  lines (`dd`, `cc`, `yy`); `>` and `<` indent and outdent the same way;
+  `x` `X` `D` `Y`, `p` `P` to put back what was deleted or yanked, `J`
+  to join lines, `r` to replace a character, `~` to change its case.
+- `u` undoes and Ctrl+R redoes: a command is a step, and so is an
+  insert, from the key that started it to Esc. Moving the cursor is not.
+- `==` formats the statement at the cursor, `gg=G` the whole script, and
+  `=` with a move the statements it covers.
+- `:` opens the Omarchy look's command line, as `:ro` and `:rw` use, and
+  gives the keyboard back to the editor after. `:w` does nothing: the
+  editor has nothing to save, and never writes to the database.
+
+`v` selects by character and `V` by line; the moves stretch the
+selection, `o` goes to its other end, and `d` (or `x`), `c` (or `s`),
+`y`, `>`, `<`, `=`, `J`, `~`, `u`, `U` and `p` act on it. What a visual
+selection holds is what Run runs. Text objects (`iw`, `i(`), `.`,
+search, registers, marks, macros and paging (`Ctrl+F`, `Ctrl+D`) are not
+there; the field's own keys (copy, paste, select all, Page Up and Page
+Down) work in every mode.
+
 ## Completion
 
 Keywords, schemas, tables, views and the columns of the tables in your
```

```diff
diff --git a/docs/_reference/settings-and-files.md b/docs/_reference/settings-and-files.md
--- a/docs/_reference/settings-and-files.md
+++ b/docs/_reference/settings-and-files.md
@@ -51,8 +51,9 @@ open SQL editor follows each change at once.
 | **New tabs run in** | Read-only transaction, or Follow connection. Read-only: a new SQL editor opens in Read-only on every connection, and its switch can still set it to Read-write. Follow connection: it opens in Read-write on a writable connection that is not production, as the [SQL Editor guide]({% link _guide/sql-editor.md %}#read-only-and-read-write) says. A connection that opens read-only keeps every editor read-only either way. A change is for the tabs opened after it | Read-only transaction |
 | **Row limit** | 100, 500, 1,000, 5,000 or 10,000 rows for each statement, or No limit. The rows past the limit are not fetched; nothing is added to your SQL. **Show all** under a cut result fetches the rest | 1,000 |
 | **Timeout** | 5, 15 or 30 seconds, 1 or 5 minutes, or Off. A run still going then is cancelled and rolled back; Cancel works either way. **Run without timeout** runs a stopped run again with none | 30 s |
-| **⌘↩ runs** (Omarchy: ctrl+enter runs) | Statement at cursor, or Whole tab: every statement in order, stopping at the first error. A selection always runs what it selects, whichever is set | Statement at cursor |
+| **⌘↩ runs** (Omarchy: ctrl+enter runs) | Statement at cursor, or Whole tab: every statement in order, stopping at the first error. A selection (with vim keys, a visual selection) always runs what it selects, whichever is set | Statement at cursor |
 | **Confirm UPDATE or DELETE without WHERE** (Omarchy: confirm write without where) | On or off. On, a run in Read-write asks before it sends an `UPDATE` or a `DELETE` with no `WHERE` or `LIMIT` of its own (a `WHERE` inside a subquery does not count; on MySQL, an inner join of the table with its `ON` counts as one; an `UPDATE` or a `DELETE` inside a `WITH` is asked about too): "Update every row in book_covers?", with about how many rows the table holds by the database's statistics, where the sidebar has loaded that count (SQLite keeps none). Cancel (Omarchy: `n`, Enter or Esc) sends nothing; Run anyway (Omarchy: `y`) runs it. A `TRUNCATE` asks either way. A tab in Read-only never asks: its runs cannot write | On |
+| **Keybindings** | Standard or Vim. Vim gives the SQL editor vim's normal, insert and visual modes, as the [SQL Editor guide]({% link _guide/sql-editor.md %}#vim) lists them, with the mode at the start of the status line (macOS and Windows: of the footer). A change applies to the open editors at once, and one in insert mode goes to normal mode | Standard on macOS and Windows, Vim in the Omarchy look |
 | **Indent** | 2 spaces, 4 spaces or Tab: what Tab puts in at the cursor, or at the start of every line a selection touches, and Shift+Tab takes off them; and how far Format sets a `CASE`'s `WHEN` and `ELSE` in (four for Tab: the layout is set in spaces) | 2 spaces |
 | **Autocomplete** | As you type, On Ctrl+Space (the list opens only when you ask), or Off. Keywords are offered in the case **Keywords** sets, upper case with Keep | As you type |
 | **Close brackets and quotes** (Omarchy: close brackets) | On or off. On, `(`, `[`, `{` and a quote (and a backtick, which quotes names on MySQL and SQLite) get their closing one after the cursor where nothing follows that it would split off. A quote is not paired inside a word, right after the same quote, or inside a string, a quoted name or a comment. A closing one typed before its twin steps over it, and Backspace takes both out while the pair is empty. Typed over a selection, the two go round it, which stays selected and undoes in one step. A paste, an input method's text, or two characters arriving at once go in as they come | On |
@@ -177,7 +178,7 @@ bar = "tint"
 | `editor.run.timeout` | `"1s"` … `"3600s"` or `"1m"` … `"60m"`, or `"off"`. A longer one is a line that cannot be read | `"30s"` |
 | `editor.run.run` | `"statement"` or `"all"`: what Cmd/Ctrl+Return runs | `"statement"` |
 | `editor.run.confirm_unsafe_writes` | `true` or `false` | `true` |
-| `editor.typing.keys` | `"standard"` or `"vim"`. Read and kept; not acted on yet | Not set: standard on macOS and Windows, vim in the Omarchy look |
+| `editor.typing.keys` | `"standard"` or `"vim"` | Not set: standard on macOS and Windows, vim in the Omarchy look |
 | `editor.typing.indent` | `2`, `4` or `"tab"` | `2` |
 | `editor.typing.autocomplete` | `"typing"`, `"manual"` or `"off"` | `"typing"` |
 | `editor.typing.close_pairs` | `true` or `false` | `true` |
```

- [ ] **Step 2: Build and check the site**

Run: `cd docs && bin/check` where Ruby and the bundle are installed; otherwise leave it to `docs.yml` on the pull request.
Expected: the build and its checks pass.

- [ ] **Step 3: Commit**

```bash
git add docs/_guide/omarchy.md docs/_guide/sql-editor.md docs/_reference/settings-and-files.md
git commit -m "Describe the SQL editor's vim keys on the website"
```

---

### Task 5: The pull request

- [ ] Push the branch and open one pull request: a summary, the row built (no row of the Editor pane is hidden now), the decisions and departures above, and what each look does by default. No keys added to the keymap. No screenshot.
