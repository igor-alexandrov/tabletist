# Settings › Editor, step 9: Indent shown, and Enter Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make Settings › Editor's Indent visible in the preview (a `CASE` in its query) and make Enter in the SQL editor keep the line's indentation, in both looks and with both keybindings.

**Architecture:** The preview's `QUERY` in `src/ui/settings/editor_preview.rs` gains `sum(CASE WHEN c.kind = 'front' then 1 else 0 END) as fronts`; Format already sets a `CASE`'s parts in by the indent, so nothing else changes there. Enter: a new `keep_indent` in `src/ui/sql_text.rs` runs in `edit` before the field reads the frame, and puts the leading whitespace of the cursor's line (up to the cursor) as text right after each line-break key. The field's events that must wait for the next frame (vim's today, Enter's now) go in one stash, restored at the top of `edit`.

**Tech Stack:** Rust, egui (crmne fork), headless tests through `src/testing.rs`.

**Spec:** `docs/superpowers/specs/2026-10-19-settings-editor-design.md`, sections "Steps" item 9 and "Step 9: Indent in the preview, and Enter".

---

## Before Task 1

The investigation's tests (`src/ui/indent_settings_tests.rs`, and its `mod` line in `src/ui/mod.rs`) are staged and not yet committed. Commit them, then the spec's step 9 section and this plan:

```bash
git commit -S -m "Show in tests what Indent changes in the SQL editor"
git add docs/superpowers/specs/2026-10-19-settings-editor-design.md docs/superpowers/plans/2026-10-22-settings-editor-9-indent.md
git commit -S -m "Plan Settings › Editor step 9: Indent shown, and Enter"
```

## Where this sits

Branch `claude/settings-editor-indent-7c99f6`, cut from `claude/settings-editor-preview` (PR #216, another session's: never push to it). #216 merged on 2026-10-23 (`ae587576`): before Task 1, merge `origin/main` into this branch (no rebase, no force-push), and open the PR against `main`.

## Departures, for a yes or a no before code

| Asked, or the boards | Built | Why |
|---|---|---|
| The boards' preview: five lines, line 3 `FROM book_covers c` lit | Nine lines with the `CASE`; the cursor and the lit line stay on line 3, now `sum(CASE` (the other way: find the `FROM` line) | The rule step 8 built; the lit line sits on what Indent moves |
| "Indent shows in the preview" | 2 spaces and 4 spaces differ; 4 spaces and Tab look the same | Format sets Tab's `CASE` in four spaces (the river is spaces), and egui draws a tab four spaces wide |
| "Enter keeps the line's indentation" | The line's own whitespace up to the cursor, whatever Indent is set; no more after `(` or `CASE` | What an editor without a parser does; Format sets the rest |
| (a line of only whitespace) | Enter after it leaves the whitespace there, and the new line has it too | Nothing else trims a line's end |
| (vim's `r` then Enter) | A bare line break, as now | Vim's own, without `autoindent` |

A reviewer read the plan against the spec and the code. What it found is fixed: the undo test expected a step egui does not make in a headless run (one undo gives the empty tab); `keep_indent` clamps the tab's own cursor before reading the text at it; `vim_keys`' `waited` binding is named as going; #216 is merged, so `main` is merged in first. It checked every expected text, offset and line number, the one stash's order (Enter's waiting events before vim's), egui's Enter and text in the field, and vim's insert and normal modes, and found them right.

## Files

| File | What it does after this plan |
|---|---|
| `src/ui/settings/editor_preview.rs` | The query with its `CASE`; the module's words name the indent. |
| `src/ui/editor_preview_tests.rs` | The nine-line query, its numbers, and the `CASE`'s indent for each value. |
| `src/ui/sql_text.rs` | `lead_at`, `keep_indent`, `is_line_break`; `edit` restores the field's stash (`later_id`) and calls `keep_indent`; `vim_keys` stashes into it and no longer restores it. |
| `src/ui/indent_settings_tests.rs` | Enter's tests replace `enter_starts_the_new_line_at_its_start_whatever_the_indent`. |
| `docs/_reference/settings-and-files.md` | The Indent row (vim's shifts, Enter, the preview) and the preview paragraph. |
| `docs/_guide/sql-editor.md` | Enter keeps the line's indentation. |

Every task ends with the repository's checks passing:

    cargo fmt --all --check
    cargo clippy --locked --workspace --all-targets -- -D warnings
    cargo test --locked --workspace --all-targets
    RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps

(`cargo` is `~/.cargo/bin/cargo` in these sessions; never build in `/tmp`.) Commit after each task with `git commit -S`. If signing says "agent refused operation" or "communication with agent failed", 1Password is locked: leave the change staged and say so.

---

### Task 1: A `CASE` in the preview's query

**Files:**
- Modify: `src/ui/settings/editor_preview.rs:1-24`
- Test: `src/ui/editor_preview_tests.rs`

- [ ] **Step 1: Write the failing tests**

In `src/ui/editor_preview_tests.rs`, `the_preview_is_the_boards_query_read_only_in_each_look_s_case` becomes:

```rust
#[test]
fn the_preview_is_the_boards_query_read_only_in_each_look_s_case() {
    let mut harness = pane(Look::macos());
    let (text, read_only) = preview(&mut harness);
    assert_eq!(
        text,
        [
            "SELECT p.name AS publisher,",
            "       count(*) AS covers,",
            "       sum(CASE",
            "             WHEN c.kind = 'front' THEN 1",
            "             ELSE 0",
            "           END) AS fronts",
            "  FROM book_covers c",
            "  JOIN publishers p ON p.id = c.publisher_id",
            " GROUP BY p.name;",
        ]
        .join("\n")
    );
    assert!(read_only);
    assert_eq!(
        numbers(&mut harness),
        ["1", "2", "3", "4", "5", "6", "7", "8", "9"]
    );
    // Omarchy: lower case, and the cursor's line the one absolute number.
    let mut harness = pane(Look::omarchy());
    let (text, read_only) = preview(&mut harness);
    assert!(text.starts_with("select p.name as publisher,"), "{text}");
    assert!(read_only);
    assert_eq!(
        numbers(&mut harness),
        ["2", "1", "3", "1", "2", "3", "4", "5", "6"]
    );
    // The cursor's one absolute number in the warning's colour.
    assert_eq!(
        harness.painted_color("3"),
        Some(harness.app.palette.warning)
    );
}
```

The Omarchy colour check: `painted_color("3")` finds the first "3" painted. With nine numbers there are two "3"s (the absolute one on line 3, a relative one on line 6). If the check fails for that reason, look the absolute one up by its place (the third number's rect) rather than by its text, using the helper `numbers` reads from.

Add, after it:

```rust
#[test]
fn the_preview_s_case_stands_in_by_the_indent_set() {
    for look in [Look::macos(), Look::omarchy()] {
        let mut harness = pane(look);
        for commas in [Commas::Trailing, Commas::Leading] {
            set(&mut harness, OptionValue::Commas(commas));
            for (indent, columns) in [(Indent::Two, 13), (Indent::Four, 15), (Indent::Tab, 15)] {
                set(&mut harness, OptionValue::Indent(indent));
                let (text, _) = preview(&mut harness);
                let when = text
                    .lines()
                    .find(|line| line.trim_start().to_lowercase().starts_with("when"))
                    .expect("a WHEN");
                let case = format!("{} {commas:?} {indent:?}", look.name);
                assert_eq!(when.len() - when.trim_start().len(), columns, "{case}");
                assert!(!text.contains('\t'), "{case}");
            }
        }
    }
}
```

Add `Indent` to the file's `use crate::settings::{…}` line.

Every other `numbers` assertion in the file grows to nine:

- `the_preview_follows_each_setting_as_it_is_set`: `["2", "1", "3", "1", "2"]` → `["2", "1", "3", "1", "2", "3", "4", "5", "6"]`.
- `wrap_lays_the_preview_s_long_line_out_in_rows_numbered_once`: the comment "Rows past the five lines" → "Rows past the nine lines"; `["1", "2", "3", "4", "5"]` → `["1", "2", "3", "4", "5", "6", "7", "8", "9"]`.
- `a_settings_change_puts_the_cursor_back_out_of_visual_mode` and `the_preview_starts_again_where_the_boards_have_it_when_the_window_opens_again`: `["1", "1", "2", "3", "4"]` → `["1", "1", "2", "3", "4", "5", "6", "7", "8"]`, and `["2", "1", "3", "1", "2"]` → `["2", "1", "3", "1", "2", "3", "4", "5", "6"]`.

- [ ] **Step 2: Run them and see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib editor_preview_tests`
Expected: FAIL: the text has five lines, and every case of the new test finds no `WHEN`.

- [ ] **Step 3: The query with its `CASE`**

In `src/ui/settings/editor_preview.rs`, the module's doc and `QUERY`:

```rust
//! Settings › Editor's preview: the SQL editor itself, over the boards'
//! Bookshop query (publishers and `book_covers`) as Format lays it out with
//! the settings, read-only. It shows every setting as it is set: the
//! keybindings' cursor, the line numbers, the cursor's line, wrap, the
//! keywords' case and the commas, and the indent (its `CASE`'s parts). Its
//! own state (the cursor, vim's mode) is the window's, in egui's memory:
//! nothing of it reaches the backend.
```

```rust
/// The boards' query, as typed: Format lays it out. Its keywords are of
/// both cases, so that Keep shows otherwise than upper and lower case, and
/// its `CASE` (not the boards') stands in by the indent.
const QUERY: &str = "SELECT p.name AS publisher, count(*) as covers, \
                     sum(CASE WHEN c.kind = 'front' then 1 else 0 END) as fronts \
                     from book_covers c JOIN publishers p on p.id = c.publisher_id \
                     GROUP BY p.name;";
```

In `show`, the comment on the cursor: "the start of the third line's words (`FROM book_covers c`)" → "the start of the third line's words (`sum(CASE`)". The code stays as it is.

- [ ] **Step 4: Run them and see them pass**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib editor_preview_tests`
Expected: PASS. Then the whole of `ui::` (`-p tabletist --lib ui::`): the preview's height, the sheet's scroll to its end and the Omarchy screen at 1920×1080 must still hold the nine lines. Most at risk: `wrap_lays_the_preview_s_long_line_out_in_rows_numbered_once` (640 points wide, nine lines wrapped) and every test on the 1920×1080 Omarchy pane. If one fails because the card no longer fits, say so before changing any size.

- [ ] **Step 5: Docs**

`docs/_reference/settings-and-files.md`, the preview paragraph under "A third pane, **Editor**": "laid out as Format would with the Keywords and Commas set, and showing the keybindings' cursor, …" becomes "laid out as Format would with the Keywords, Commas and Indent set (its `CASE` stands in two or four: a tab looks as four spaces), and showing the keybindings' cursor, …".

- [ ] **Step 6: The four checks, and commit**

```bash
git add src/ui/settings/editor_preview.rs src/ui/editor_preview_tests.rs docs/_reference/settings-and-files.md
git commit -S -m "Show the indent in the editor preview with a CASE in its query"
```

---

### Task 2: Enter keeps the line's indentation

**Files:**
- Modify: `src/ui/sql_text.rs` (`edit`, `vim_keys`, new `lead_at`, `is_line_break`, `keep_indent`)
- Test: `src/ui/indent_settings_tests.rs`

- [ ] **Step 1: Write the failing tests**

In `src/ui/indent_settings_tests.rs`, `enter_starts_the_new_line_at_its_start_whatever_the_indent` is replaced by the tests below. Add a `put_cursor` and a `select` helper, as `typing_settings_tests` and `format_settings_tests` have them (copy theirs: they set the field's `CCursorRange` through `egui::TextEdit::load_state` / `store_state` on `crate::ui::sql_text::editor_id(tab, id)`, then settle).

```rust
/// Presses Enter, then types `typed`, a frame each.
fn enter_then(harness: &mut Harness, typed: &str) {
    harness.press(Key::Enter, Modifiers::NONE);
    harness.settle();
    harness.frame(vec![egui::Event::Text(typed.into())]);
    harness.settle();
}

#[test]
fn enter_starts_the_new_line_at_the_line_s_indentation() {
    for (look, keys) in every_editor() {
        for (indent, unit) in INDENTS {
            let case = format!("{} {keys:?} {indent:?}", look.name);
            // Each value's own indentation is kept, and so is another's:
            // it is the line's whitespace, not the setting's.
            for lead in [unit.to_owned(), format!("{unit}{unit}"), " \t ".to_owned()] {
                let line = format!("{lead}SELECT 1");
                let (mut harness, tab) = editor(look, keys, indent, &line);
                enter_then(&mut harness, "x");
                assert_eq!(text(&harness, tab), format!("{line}\n{lead}x"), "{case} {lead:?}");
            }
            // Shift+Enter too.
            let line = format!("{unit}SELECT 1");
            let (mut harness, tab) = editor(look, keys, indent, &line);
            harness.press(Key::Enter, Modifiers::SHIFT);
            harness.settle();
            assert_eq!(text(&harness, tab), format!("{line}\n{unit}"), "{case}");
        }
    }
}

#[test]
fn enter_at_or_inside_the_indentation_keeps_what_follows_where_it_was() {
    for (look, keys) in every_editor() {
        let case = format!("{} {keys:?}", look.name);
        // At the line's start: nothing added, the line moves down whole.
        let (mut harness, tab) = editor(look, keys, Indent::Two, "SELECT 1\n    FROM t");
        put_cursor(&mut harness, tab, 9);
        harness.press(Key::Enter, Modifiers::NONE);
        harness.settle();
        assert_eq!(text(&harness, tab), "SELECT 1\n\n    FROM t", "{case}");
        // Two into four spaces: FROM keeps its column.
        let (mut harness, tab) = editor(look, keys, Indent::Two, "SELECT 1\n    FROM t");
        put_cursor(&mut harness, tab, 11);
        harness.press(Key::Enter, Modifiers::NONE);
        harness.settle();
        assert_eq!(text(&harness, tab), "SELECT 1\n  \n    FROM t", "{case}");
    }
}

#[test]
fn enter_over_a_selection_takes_the_indentation_of_its_start_s_line() {
    for look in [Look::macos(), Look::omarchy()] {
        let (mut harness, tab) =
            editor(look, EditorKeys::Standard, Indent::Two, "    SELECT 1\nFROM t");
        // `SELECT 1\nFR` selected.
        select(&mut harness, tab, 4, 15);
        harness.press(Key::Enter, Modifiers::NONE);
        harness.settle();
        assert_eq!(text(&harness, tab), "    \n    OM t", "{}", look.name);
    }
}

#[test]
fn typing_and_enters_in_one_frame_keep_their_order_and_each_line_s_indentation() {
    for look in [Look::macos(), Look::omarchy()] {
        let (mut harness, tab) = editor(look, EditorKeys::Standard, Indent::Two, "  ");
        let enter = || {
            [
                crate::testing::key(Key::Enter, Modifiers::NONE),
                crate::testing::release(Key::Enter, Modifiers::NONE),
            ]
        };
        let mut frame = vec![egui::Event::Text("a".into())];
        frame.extend(enter());
        frame.push(egui::Event::Text("b".into()));
        frame.extend(enter());
        frame.extend(enter());
        frame.push(egui::Event::Text("c".into()));
        harness.frame(frame);
        harness.settle();
        assert_eq!(text(&harness, tab), "  a\n  b\n  \n  c", "{}", look.name);
    }
}

#[test]
fn enter_under_an_open_list_breaks_the_line_with_its_indentation_or_accepts() {
    for look in [Look::macos(), Look::omarchy()] {
        // On what is already typed: a line break, and the list is done.
        let (mut harness, tab) = editor(look, EditorKeys::Standard, Indent::Two, "  ");
        harness.frame(vec![egui::Event::Text("as".into())]);
        harness.settle();
        harness.press(Key::Enter, Modifiers::NONE);
        harness.settle();
        assert_eq!(text(&harness, tab), "  as\n  ", "{}", look.name);
        let workspace = harness.app.workspace(tab).unwrap();
        assert!(workspace.active_sql_tab().unwrap().completion.is_none());
        // On a row: it is accepted, and no line break.
        let (mut harness, tab) = editor(look, EditorKeys::Standard, Indent::Two, "  ");
        harness.frame(vec![egui::Event::Text("sel".into())]);
        harness.settle();
        harness.press(Key::Enter, Modifiers::NONE);
        harness.settle();
        let word = if look.terminal { "select" } else { "SELECT" };
        assert_eq!(text(&harness, tab), format!("  {word}"), "{}", look.name);
    }
}

#[test]
fn enter_in_vim_s_normal_mode_moves_and_types_nothing() {
    for look in [Look::macos(), Look::omarchy()] {
        let script = "  SELECT 1\n  FROM t";
        let (mut harness, tab) = editor(look, EditorKeys::Vim, Indent::Two, script);
        harness.press(Key::Escape, Modifiers::NONE);
        keys(&mut harness, "gg");
        harness.press(Key::Enter, Modifiers::NONE);
        harness.settle();
        assert_eq!(text(&harness, tab), script, "{}", look.name);
        let workspace = harness.app.workspace(tab).unwrap();
        assert_eq!(workspace.active_sql_tab().unwrap().cursor, 13, "{}", look.name);
    }
}

#[test]
fn an_enter_and_its_indentation_undo_with_what_is_typed_beside_them() {
    let (mut harness, tab) = editor(Look::macos(), EditorKeys::Standard, Indent::Two, "  SELECT 1");
    enter_then(&mut harness, "x");
    // No undo point between: undo takes back all that was typed, the line
    // break and its indentation with it, as it does for a completion
    // (`complete_tests::tab_on_the_row_that_is_already_typed_changes_nothing`).
    harness.press(Key::Z, Modifiers::COMMAND);
    harness.settle();
    assert_eq!(text(&harness, tab), "");
}
```

egui's undoer marks a point only after the text has held still for a second, and the harness moves a frame's 1/60 s on: the fixture's paste, the Enter, its indentation and the `x` are one step, and one undo gives the empty tab. The test shows that the indentation is no step of its own; it passes before the change too.

Also add to `src/ui/editor_preview_tests.rs`, in `nothing_typed_changes_the_preview_and_tab_moves_on`, after the paste and Backspace: `harness.press(Key::Enter, Modifiers::NONE);` (still before the `assert_eq!(preview(…).0, before, …)`).

- [ ] **Step 2: Run them and see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib indent_settings_tests`
Expected: FAIL in `enter_starts_the_new_line_at_the_line_s_indentation` (`"  SELECT 1\nx"`), `enter_at_or_inside…` (the second case), `enter_over_a_selection…`, `typing_and_enters…` and the first half of `enter_under_an_open_list…`. `enter_in_vim_s_normal_mode…` passes already (nothing to change in normal mode); keep it.

- [ ] **Step 3: The leading whitespace, and the line break**

In `src/ui/sql_text.rs`, after `indent_script`:

```rust
/// The spaces and tabs the line of byte `at` starts with, up to `at`:
/// what Enter there puts at the new line's start, so that what follows
/// the cursor keeps its column.
fn lead_at(text: &str, at: usize) -> &str {
    let start = text[..at].rfind('\n').map_or(0, |end| end + 1);
    let before = &text[start..at];
    &before[..before.len() - before.trim_start_matches([' ', '\t']).len()]
}

/// Whether `event` is a line break as the field takes it: Enter pressed,
/// with Shift or without (egui's return key, matched logically).
fn is_line_break(event: &egui::Event) -> bool {
    matches!(
        event,
        egui::Event::Key { key: egui::Key::Enter, pressed: true, modifiers, .. }
            if modifiers.matches_logically(egui::Modifiers::NONE)
    )
}

/// Enter keeps the line's indentation: before the field reads the frame,
/// a line break gets the leading whitespace of the cursor's line up to the
/// cursor (with a selection, of its start's line, `lead_at`) typed right
/// after it, which the field puts in after the line break. Only while the
/// field has the keyboard, and the completion list took the Enter it
/// accepts with before. The whitespace is read from the script as the
/// frame starts: a line break after an event of the frame's that may
/// change the script or the cursor (`edits`) waits, with what follows it,
/// for the next frame, in the field's stash (`later_id`), before what
/// vim's keys put there.
fn keep_indent(ui: &mut Ui, sql_tab: &SqlTab, field: &Field<'_>) {
    if !ui.memory(|memory| memory.has_focus(field.id)) {
        return;
    }
    let start = egui::TextEdit::load_state(ui.ctx(), field.id)
        .and_then(|state| state.cursor.char_range())
        .map_or(sql_tab.cursor, |range| {
            byte_offset(&sql_tab.text, range.sorted_cursors()[0].index.0)
        });
    // The tab's own cursor may be of an older text.
    let start = sql_tab
        .text
        .floor_char_boundary(start.min(sql_tab.text.len()));
    let lead = lead_at(&sql_tab.text, start).to_owned();
    let waits = ui.input_mut(|input| {
        let mut queue = std::mem::take(&mut input.events).into_iter();
        let (mut kept, mut waits) = (Vec::new(), Vec::new());
        let mut edited = false;
        while let Some(event) = queue.next() {
            if is_line_break(&event) {
                if edited {
                    waits.push(event);
                    waits.extend(queue.by_ref());
                    break;
                }
                kept.push(event);
                if !lead.is_empty() {
                    kept.push(egui::Event::Text(lead.clone()));
                }
                edited = true;
                continue;
            }
            edited |= edits(&event);
            kept.push(event);
        }
        input.events = kept;
        waits
    });
    if !waits.is_empty() {
        let later_id = later_id(field);
        ui.data_mut(|data| {
            let after: Vec<egui::Event> = data.remove_temp(later_id).unwrap_or_default();
            let mut all = waits;
            all.extend(after);
            data.insert_temp(later_id, all);
        });
        ui.ctx().request_repaint();
    }
}
```

`edits` already exists (it is what `vim_keys` uses for the same question); `byte_offset` too.

- [ ] **Step 4: One stash for the field's waiting events**

The field's stash moves out of `vim_keys`, so that Enter's can share it and the standard keybindings restore it too.

Add, by `edits`:

```rust
/// Where the field keeps the frame's events that wait for the next one:
/// vim's keys after an edit of the field's (`vim_keys`), and a line break
/// after an edit (`keep_indent`).
fn later_id(field: &Field<'_>) -> Id {
    field.id.with("later")
}

/// Puts the events that waited (`later_id`) before this frame's, while the
/// field has the keyboard; without it they are nobody's.
fn restore_later(ui: &mut Ui, field: &Field<'_>) {
    let waited: Vec<egui::Event> = ui
        .data_mut(|data| data.remove_temp(later_id(field)))
        .unwrap_or_default();
    if waited.is_empty() || !ui.memory(|memory| memory.has_focus(field.id)) {
        return;
    }
    ui.input_mut(|input| {
        let mut events = waited;
        events.append(&mut input.events);
        input.events = events;
    });
}
```

In `vim_keys`:

- Remove `let later_id = field.id.with("vim later");`.
- In the early return (`if !field.vim || !ui.memory(…has_focus…)`), remove the `ui.data_mut(|data| data.remove_temp::<Vec<egui::Event>>(later_id));` line.
- Remove the `waited` read: the `let waited: Vec<egui::Event> = ui.data_mut(|data| data.remove_temp(later_id)).unwrap_or_default();` binding goes, and `let events = ui.input_mut(|input| { let mut events = waited; events.append(&mut input.events); events });` becomes `let events = ui.input_mut(|input| std::mem::take(&mut input.events));`.
- At the end, `ui.data_mut(|data| data.insert_temp(later_id, later));` becomes `ui.data_mut(|data| data.insert_temp(later_id(field), later));`. (Nothing is in the stash then: `restore_later` emptied it at the frame's start, and `keep_indent` runs after.)

Update `vim_keys`' doc where it says "it and what follows wait for the next frame": add ", in the field's stash (`later_id`)".

In `edit`, first thing after `let focus = std::mem::take(&mut sql_tab.focus_editor);`... no: first thing in the function, before `insert_completion`:

```rust
    // What waited from the frame before goes first.
    restore_later(ui, field);
```

and after `let (vimmed, prompt) = vim_keys(ui, sql_tab, field);` and the preview's Esc block, before `let indented = …`:

```rust
    // Enter keeps the line's indentation; the preview's text does not
    // change.
    if !field.read_only {
        keep_indent(ui, sql_tab, field);
    }
```

`keep_indent` takes `sql_tab` as `&SqlTab`: `&*sql_tab` if the borrow checker asks.

- [ ] **Step 5: Run the tests and see them pass**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib indent_settings_tests`
Expected: PASS.

Then the editor's whole suite, which has the frame-order tests vim's stash was built for: `~/.cargo/bin/cargo test --locked -p tabletist --lib ui::` (in particular `vim_tests::keys_in_one_frame_are_read_in_the_mode_the_ones_before_left`, `complete_tests::in_a_frame_that_also_types_tab_does_nothing_and_enter_is_a_line_break`, `format_settings_tests::a_tab_and_text_typed_in_one_frame_keep_their_order`). Expected: PASS.

- [ ] **Step 6: Docs**

`docs/_reference/settings-and-files.md`, the Indent row's Choices cell becomes:

> 2 spaces, 4 spaces or Tab: what Tab puts in at the cursor, or at the start of every line a selection touches, and Shift+Tab takes off them (with [vim keys]({% link _guide/sql-editor.md %}#vim), `>` and `<` too); and how far Format sets a `CASE`'s `WHEN` and `ELSE` in (four for Tab: the layout is set in spaces), as the preview's `CASE` shows. Enter keeps the indentation the line has, whatever is set

`docs/_guide/sql-editor.md`, the paragraph "Tab puts in an indent …": after "and Shift+Tab takes one off." add "Enter starts the new line with the spaces and tabs the line it breaks starts with, so what follows the cursor keeps its column."

- [ ] **Step 7: The four checks, and commit**

```bash
git add src/ui/sql_text.rs src/ui/indent_settings_tests.rs src/ui/editor_preview_tests.rs docs/_reference/settings-and-files.md docs/_guide/sql-editor.md
git commit -S -m "Keep the line's indentation on Enter in the SQL editor"
```

---

### After Task 2

`git fetch origin && git merge origin/main` again if `main` moved (no rebase), run the four checks, push the branch, and open a PR against `main` with the findings table (what Indent changes, from `indent_settings_tests`), the two changes and the departures. No attribution lines. The indentation's text counts as typed, so Enter on an indented line now sends `SqlTyped` too: harmless (the list closes on an empty word), and worth a line in the PR.
