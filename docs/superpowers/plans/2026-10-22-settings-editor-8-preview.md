# Settings › Editor, step 8: Preview Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** End Settings › Editor with its preview, in both looks: the real SQL editor over the boards' Bookshop query (publishers and `book_covers`), read-only, named "Editor preview", one Tab stop, showing every setting live, and never reaching a database.

**Architecture:** The SQL editor's drawing is split out of `sql_text::show` into `draw_text` (the field scrolling beside the gutter, and the gutter's press) and `draw_lines` (the bands behind the text and the gutter's numbers), with how it draws (`Shown`) read from the settings in one place. `show` keeps what is a workspace's between the two: its actions and the completion list. `sql_text::preview` draws the same two over a `SqlTab` of its own with a read-only field: the text field reads a `&str`, so nothing typed changes it; Tab, the pairs and vim's edits are dropped; Tab moves on. `settings/editor_preview.rs` keeps that tab in egui's memory (the window's, not the app's), sets its text to the query as Format lays it out with the settings, and draws it in a card as tall as the text last laid out. The macOS sheet and the Omarchy screen draw it under History, as Appearance draws its preview.

**Tech Stack:** Rust, egui (crmne fork) with AccessKit, headless tests through `src/testing.rs`.

**Spec:** `docs/superpowers/specs/2026-10-19-settings-editor-design.md`, sections "Steps" item 8 and "Step 8: Preview".

---

## Before Task 1

Commit the spec's step 8 section and this plan:

```bash
git add docs/superpowers/specs/2026-10-19-settings-editor-design.md docs/superpowers/plans/2026-10-22-settings-editor-8-preview.md
git commit -m "Plan Settings › Editor step 8: preview"
```

## Where this sits

Steps 1 to 7 are merged (#181, #183, #204, #210, #212, #214; history with #197). This plan is a branch of `main` at `62752b72`. It is the last step: after it no row the boards draw is missing.

## Departures, for a yes or a no before code

| Asked, or the boards | Built | Why |
|---|---|---|
| "Shows every setting live: keybindings mode, line numbers, current line, wrap, indent and keyword case" | Keyword case and commas (as Format lays the query out), line numbers, the cursor's line, wrap and vim's block cursor show live. Indent shows nothing: the boards' query has no `CASE`, the only place Format uses it (the other way: add a `CASE` to the query, which the boards do not draw). Close brackets and autocomplete show nothing: they act on typing, which the preview has none of | The boards' query, kept as drawn |
| The boards' preview: line 3 lit, no cursor drawn | The cursor at the start of line 3's words, the line lit whether or not the preview has the keyboard (unless Current line highlight is off); vim's block cursor shown without the keyboard too | The editor lights the line only with the keyboard; the boards light it always |
| "A read-only text area labelled Editor preview … one Tab stop" | The editor's own field, reading a `&str`: the cursor, a selection, copy and vim's moves work; typing, a paste, a cut, Tab's indent, the pairs, vim's edits and insert mode change nothing. No gutter to press and no statement bar | A second stop would be the gutter; the boards draw no bar |
| (Esc, and the Omarchy screen's keys) | While the preview has the keyboard, Esc gives it up and the next closes the window, with either keybindings. On the Omarchy screen its letters and Esc are the preview's then (the cursor's `j` `k`, `h` `l`, Space, Enter, `R`, `D`, `y`, `n`); Ctrl+E and the panes' Ctrl+H, Ctrl+L stay the screen's | Vim's moves would otherwise change settings |
| (which has the keyboard) | The card's border in the accent while the preview has the keyboard | With vim its cursor and lit line show either way |
| (the window reopened) | The preview starts again, its cursor on line 3, when it was not drawn the frame before (the window closed, another pane shown) | As the boards have it |
| The boards' query, all of one case | The same query, typed with keywords of both cases (`SELECT … as … from … JOIN … on … GROUP BY`) | So that Keep shows as Keep, not as one of the other two |
| (where it is) | macOS: under History, a "Preview" heading, at the sheet's end, which scrolls to it. Omarchy: a "preview" heading under history; no row of the cursor's, reached with the pointer | As Appearance's preview |
| (its height) | As tall as the query's rows were laid out the frame before (an estimate before the first), in a card bordered as Appearance's | The field lays out whole lines, a little taller than the font says |

A reviewer given the draft broke it with tests of its own, and the draft now fixes what it found:

- On the Omarchy screen, the screen's letter keys still acted while the preview had the keyboard: a vim `j` moved the screen's cursor too, `h` and `l` changed and saved the option under it, and `R` reset it; Esc in insert mode closed the window. While the preview has the keyboard the screen's keys are its own now (`the_omarchy_screen_s_keys_are_the_preview_s_while_it_has_the_keyboard`).
- A dropped `dd` still filled vim's register, which the SQL editors share; the preview's register is its own now.
- Visual mode outlived the reset a settings change makes, so the cursor's line and the relative numbers stayed on the old line.
- After `i` and Tab away the preview showed no cursor at all: insert mode is dropped in the preview, which has nothing to type.
- Keep looked exactly like lowercase: the query is typed with keywords of both cases now.

It found working: the SQL editor itself after the split (every editor test, and the order of its parts), the preview's height held still across widths, scales, font sizes and wrap with no repaint loop, the whole query inside the card, a wheel over it, select all and copy, Shift+Tab and Tab back to it once, no preview on the other panes, nothing sent to the backend, and the AccessKit role, name and read-only flag.

A second reviewer read the plan against the spec, your request and the code. What it found is fixed:

- With the standard keybindings, Esc in the preview closed the window at once (egui let the field go before the window read the key). The preview now holds Esc with either keybindings, gives the keyboard up and takes the key; the next closes the window (`esc_gives_the_preview_s_keyboard_up_before_it_closes_the_window_with_either_keys`).
- On the Omarchy screen every key of the screen's went dead while the preview had the keyboard, Ctrl+E and the panes' keys too; only its letters and Esc are the preview's now.
- Nothing showed that the preview had the keyboard: its border is in the accent then.
- The preview kept its cursor and mode when the window closed and opened again; it starts on line 3 again.
- Tests that could pass for want of anything to test: the run keys are now pressed with a connection open, and Shift+Tab is shown to come back to the preview in one press (no stop for the gutter). The cursor's number in the warning's colour on Omarchy is checked, and the focus check names the preview's field.
- The preview claimed the workspace's editor focus region; it does not now. Step 4's test run names every module the change touches; the prose (the website's Current line highlight row, the Keywords and Commas it lays the query out with) is corrected.

## Files

| File | What it does after this plan |
|---|---|
| `src/ui/sql_text.rs` | `Shown`, `Place`, `Lines`; `show` over `draw_text` and `draw_lines`; `preview` and `preview_height`; `Field::read_only` and what it drops in `edit` and `vim_keys`; `format_style` for the preview. |
| `src/ui/settings/editor_preview.rs` | New: the preview's tab in egui's memory, its text, its card and height. |
| `src/ui/settings/sheet.rs`, `src/ui/settings/terminal.rs`, `src/ui/settings/mod.rs` | The preview under History in each look (`Preview` in the sheet). |
| `src/ui/editor_preview_tests.rs`, `src/ui/mod.rs` | New: this step's headless tests. |
| `docs/_reference/settings-and-files.md` | The preview, and no keys left "for a later version". |

Every task ends with the repository's checks passing:

    cargo fmt --all --check
    cargo clippy --locked --workspace --all-targets -- -D warnings
    cargo test --locked --workspace --all-targets
    RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps

(`cargo` is `~/.cargo/bin/cargo` in these sessions.) Commit after each task, signed as usual. If signing says "agent refused operation", 1Password is locked: leave the change staged and say so.

---

### Task 1: The editor's preview

**Files:**
- Modify: `src/ui/sql_text.rs`, `src/ui/settings/sheet.rs`, `src/ui/settings/terminal.rs`, `src/ui/settings/mod.rs`
- Create: `src/ui/settings/editor_preview.rs`
- Create (tests): `src/ui/editor_preview_tests.rs`; Modify (tests): `src/ui/mod.rs`

- [ ] **Step 1: Write the failing tests**

```diff
diff --git a/src/ui/editor_preview_tests.rs b/src/ui/editor_preview_tests.rs
new file mode 100644
--- /dev/null
+++ b/src/ui/editor_preview_tests.rs
@@ -0,0 +1,463 @@
+//! Settings › Editor's preview, in both looks: the SQL editor over the
+//! boards' Bookshop query, read-only, showing each setting as it is set.
+
+use egui::accesskit::Role;
+use egui::{Key, Modifiers};
+
+use crate::model::{Action, SettingsPane};
+use crate::settings::{Commas, EditorKeys, KeywordCase, LineNumbers, OptionValue};
+use crate::testing::{Harness, bounds};
+use crate::theme::Look;
+
+/// Settings on its Editor pane in `look`, scrolled to its end, where the
+/// preview is.
+fn pane(look: Look) -> Harness {
+    let size = if look.terminal {
+        egui::vec2(1920.0, 1080.0)
+    } else {
+        egui::vec2(1440.0, 1100.0)
+    };
+    let mut harness = Harness::with_size(size);
+    harness.set_look(look);
+    harness.app.apply(Action::ShowSettings);
+    harness
+        .app
+        .apply(Action::ShowSettingsPane(SettingsPane::Editor));
+    harness.settle();
+    to_the_end(&mut harness);
+    harness
+}
+
+/// The pane scrolled to its end, where the preview is.
+fn to_the_end(harness: &mut Harness) {
+    let at = egui::pos2(harness.size.x / 2.0, harness.size.y / 2.0);
+    harness.frame(vec![
+        egui::Event::PointerMoved(at),
+        egui::Event::MouseWheel {
+            unit: egui::MouseWheelUnit::Point,
+            delta: egui::vec2(0.0, -2000.0),
+            modifiers: Modifiers::NONE,
+            phase: egui::TouchPhase::Move,
+        },
+    ]);
+    harness.settle();
+}
+
+fn set(harness: &mut Harness, value: OptionValue) {
+    harness.app.apply(Action::SetOption(value));
+    harness.settle();
+}
+
+/// The preview's node: its text, and whether it is read-only.
+fn preview(harness: &mut Harness) -> (String, bool) {
+    let tree = harness.settle();
+    let (_, node) = tree
+        .nodes
+        .iter()
+        .find(|(_, node)| {
+            node.role() == Role::MultilineTextInput && node.label() == Some("Editor preview")
+        })
+        .expect("the preview");
+    (
+        node.value().unwrap_or_default().to_owned(),
+        node.is_read_only(),
+    )
+}
+
+/// Where the preview is on screen.
+fn place(harness: &mut Harness) -> egui::Rect {
+    let tree = harness.settle();
+    bounds(&tree, "Editor preview", Role::MultilineTextInput).expect("the preview")
+}
+
+/// The numbers painted in the preview's gutter, top to bottom.
+fn numbers(harness: &mut Harness) -> Vec<String> {
+    let field = place(harness);
+    let mut found: Vec<(f32, String)> = harness
+        .text_rects
+        .iter()
+        .filter(|(text, rect)| {
+            text.parse::<usize>().is_ok()
+                && rect.right() <= field.left()
+                && field.y_range().contains(rect.center().y)
+        })
+        .map(|(text, rect)| (rect.top(), text.clone()))
+        .collect();
+    found.sort_by(|(a, _), (b, _)| a.total_cmp(b));
+    found.into_iter().map(|(_, text)| text).collect()
+}
+
+#[test]
+fn the_preview_is_the_boards_query_read_only_in_each_look_s_case() {
+    let mut harness = pane(Look::macos());
+    let (text, read_only) = preview(&mut harness);
+    assert_eq!(
+        text,
+        "SELECT p.name AS publisher,\n       count(*) AS covers\n  FROM book_covers c\n  JOIN publishers p ON p.id = c.publisher_id\n GROUP BY p.name;"
+    );
+    assert!(read_only);
+    assert_eq!(numbers(&mut harness), ["1", "2", "3", "4", "5"]);
+    // Omarchy: lower case, and the cursor's line the one absolute number.
+    let mut harness = pane(Look::omarchy());
+    let (text, read_only) = preview(&mut harness);
+    assert!(text.starts_with("select p.name as publisher,"), "{text}");
+    assert!(read_only);
+    assert_eq!(numbers(&mut harness), ["2", "1", "3", "1", "2"]);
+    // The cursor's one absolute number in the warning's colour.
+    assert_eq!(
+        harness.painted_color("3"),
+        Some(harness.app.palette.warning)
+    );
+}
+
+#[test]
+fn the_preview_follows_each_setting_as_it_is_set() {
+    let mut harness = pane(Look::macos());
+    // Keywords and commas: Format's.
+    set(
+        &mut harness,
+        OptionValue::Keywords(Some(KeywordCase::Lower)),
+    );
+    set(&mut harness, OptionValue::Commas(Commas::Leading));
+    let (text, _) = preview(&mut harness);
+    assert!(text.starts_with("select p.name as publisher\n"), "{text}");
+    assert!(text.contains(", count(*) as covers"), "{text}");
+    // The numbers: relative, then none.
+    set(
+        &mut harness,
+        OptionValue::LineNumbers(Some(LineNumbers::Relative)),
+    );
+    assert_eq!(numbers(&mut harness), ["2", "1", "3", "1", "2"]);
+    set(
+        &mut harness,
+        OptionValue::LineNumbers(Some(LineNumbers::Off)),
+    );
+    assert!(numbers(&mut harness).is_empty());
+    // The cursor's line, lit without the keyboard, until it is turned off.
+    let palette = harness.app.palette;
+    let lit = palette.window.lerp_to_gamma(palette.selection, 0.45);
+    let field = place(&mut harness);
+    let lines = |harness: &Harness| {
+        harness
+            .fills
+            .iter()
+            .filter(|(rect, fill)| *fill == lit && field.y_range().contains(rect.center().y))
+            .count()
+    };
+    assert_eq!(lines(&harness), 1);
+    set(&mut harness, OptionValue::HighlightLine(false));
+    assert_eq!(lines(&harness), 0);
+    // Vim's cursor, a block, without the keyboard too.
+    let block = palette.accent.gamma_multiply(0.5);
+    let blocks = |harness: &Harness| {
+        harness
+            .fills
+            .iter()
+            .filter(|(_, fill)| *fill == block)
+            .count()
+    };
+    assert_eq!(blocks(&harness), 0);
+    set(&mut harness, OptionValue::EditorKeys(Some(EditorKeys::Vim)));
+    assert_eq!(blocks(&harness), 1);
+}
+
+#[test]
+fn wrap_lays_the_preview_s_long_line_out_in_rows_numbered_once() {
+    let mut harness = pane(Look::omarchy());
+    harness.size = egui::vec2(640.0, 1080.0);
+    harness.settle();
+    let unwrapped = place(&mut harness).height();
+    set(&mut harness, OptionValue::Wrap(true));
+    // Rows past the five lines: the long one wraps.
+    let wrapped = place(&mut harness).height();
+    assert!(wrapped > unwrapped + 10.0, "{unwrapped} {wrapped}");
+    set(
+        &mut harness,
+        OptionValue::LineNumbers(Some(LineNumbers::Absolute)),
+    );
+    // As many numbers as lines, whatever the rows.
+    assert_eq!(numbers(&mut harness), ["1", "2", "3", "4", "5"]);
+}
+
+#[test]
+fn nothing_typed_changes_the_preview_and_tab_moves_on() {
+    for look in [Look::macos(), Look::omarchy()] {
+        let mut harness = pane(look);
+        let (before, _) = preview(&mut harness);
+        let field = place(&mut harness);
+        let at = field.center();
+        harness.frame(vec![
+            egui::Event::PointerMoved(at),
+            egui::Event::PointerButton {
+                pos: at,
+                button: egui::PointerButton::Primary,
+                pressed: true,
+                modifiers: Modifiers::NONE,
+            },
+            egui::Event::PointerButton {
+                pos: at,
+                button: egui::PointerButton::Primary,
+                pressed: false,
+                modifiers: Modifiers::NONE,
+            },
+        ]);
+        harness.settle();
+        assert!(focused(&harness), "{}", look.name);
+        // Typing, a paste, and vim's commands: the query stays.
+        for c in "xdd".chars() {
+            harness.frame(vec![egui::Event::Text(c.to_string())]);
+            harness.settle();
+        }
+        harness.frame(vec![egui::Event::Paste("DROP TABLE books;".into())]);
+        harness.press(Key::Backspace, Modifiers::NONE);
+        assert_eq!(preview(&mut harness).0, before, "{}", look.name);
+        // One stop: Tab takes the keyboard on.
+        harness.press(Key::Tab, Modifiers::NONE);
+        let editor = crate::ui::settings::editor_preview_field();
+        assert!(
+            !harness.ctx.memory(|memory| memory.has_focus(editor)),
+            "{}",
+            look.name
+        );
+        // Nothing of it reached the backend.
+        assert!(
+            !harness
+                .app
+                .backend
+                .sent
+                .iter()
+                .any(|command| matches!(command, crate::backend::Command::RunSql { .. })),
+            "{}",
+            look.name
+        );
+    }
+}
+
+/// The preview's editor, as the window keeps it.
+fn state<T>(harness: &Harness, read: impl FnOnce(&crate::model::SqlTab) -> T) -> T {
+    let id = crate::ui::settings::editor_preview_field().with("tab");
+    let shared = harness
+        .ctx
+        .data(|data| data.get_temp::<std::sync::Arc<std::sync::Mutex<crate::model::SqlTab>>>(id))
+        .expect("the preview's tab");
+    let tab = shared.lock().unwrap();
+    read(&tab)
+}
+
+fn click(harness: &mut Harness, at: egui::Pos2) {
+    let press = |pressed| egui::Event::PointerButton {
+        pos: at,
+        button: egui::PointerButton::Primary,
+        pressed,
+        modifiers: Modifiers::NONE,
+    };
+    harness.frame(vec![
+        egui::Event::PointerMoved(at),
+        press(true),
+        press(false),
+    ]);
+    harness.settle();
+}
+
+/// A letter as a window sends it: its key, then its text.
+fn letter(harness: &mut Harness, c: char) {
+    let key = Key::from_name(&c.to_ascii_uppercase().to_string()).expect("a letter's key");
+    let shift = if c.is_uppercase() {
+        Modifiers::SHIFT
+    } else {
+        Modifiers::NONE
+    };
+    harness.frame(vec![
+        crate::testing::key(key, shift),
+        egui::Event::Text(c.to_string()),
+    ]);
+    harness.frame(vec![crate::testing::release(key, shift)]);
+    harness.settle();
+}
+
+fn focused(harness: &Harness) -> bool {
+    let id = crate::ui::settings::editor_preview_field();
+    harness.ctx.memory(|memory| memory.has_focus(id))
+}
+
+#[test]
+fn the_omarchy_screen_s_keys_are_the_preview_s_while_it_has_the_keyboard() {
+    let mut harness = pane(Look::omarchy());
+    let row = |harness: &Harness| match &harness.app.dialog {
+        Some(crate::model::Dialog::Settings(dialog)) => dialog.row,
+        _ => panic!("no Settings"),
+    };
+    let field = place(&mut harness);
+    click(&mut harness, field.center());
+    assert!(focused(&harness));
+    let (at, cursor, before) = (
+        row(&harness),
+        state(&harness, |tab| tab.cursor),
+        harness.app.settings.clone(),
+    );
+    // Vim's moves: the preview's cursor moves, the screen's stays, and no
+    // setting changes.
+    for c in "jlhlR".chars() {
+        letter(&mut harness, c);
+    }
+    assert_eq!(row(&harness), at);
+    assert_ne!(state(&harness, |tab| tab.cursor), cursor);
+    assert_eq!(harness.app.settings, before);
+    // `i` has nothing to type: no insert mode. Esc gives the keyboard up,
+    // and the window stays; a second closes it.
+    letter(&mut harness, 'i');
+    assert_eq!(
+        state(&harness, |tab| tab.vim.mode),
+        crate::vim::Mode::Normal
+    );
+    harness.press(Key::Escape, Modifiers::NONE);
+    assert!(harness.app.dialog.is_some());
+    assert!(!focused(&harness));
+    harness.press(Key::Escape, Modifiers::NONE);
+    assert!(harness.app.dialog.is_none());
+}
+
+#[test]
+fn the_preview_s_dropped_edits_leave_the_editors_register_be() {
+    let mut harness = pane(Look::macos());
+    set(&mut harness, OptionValue::EditorKeys(Some(EditorKeys::Vim)));
+    let field = place(&mut harness);
+    click(&mut harness, field.center());
+    letter(&mut harness, 'd');
+    letter(&mut harness, 'd');
+    let register: crate::vim::Register = harness
+        .ctx
+        .data(|data| data.get_temp(egui::Id::new("vim register")))
+        .unwrap_or_default();
+    assert_eq!(register, crate::vim::Register::default());
+    // And vim's cursor stays a block once the keyboard is gone.
+    letter(&mut harness, 'i');
+    harness.press(Key::Tab, Modifiers::NONE);
+    assert!(!focused(&harness));
+    let block = harness.app.palette.accent.gamma_multiply(0.5);
+    let blocks = harness
+        .fills
+        .iter()
+        .filter(|(_, fill)| *fill == block)
+        .count();
+    assert_eq!(blocks, 1);
+}
+
+#[test]
+fn a_settings_change_puts_the_cursor_back_out_of_visual_mode() {
+    let mut harness = pane(Look::macos());
+    set(&mut harness, OptionValue::EditorKeys(Some(EditorKeys::Vim)));
+    set(
+        &mut harness,
+        OptionValue::LineNumbers(Some(LineNumbers::Relative)),
+    );
+    let field = place(&mut harness);
+    click(
+        &mut harness,
+        egui::pos2(field.center().x, field.top() + 12.0),
+    );
+    assert_eq!(numbers(&mut harness), ["1", "1", "2", "3", "4"]);
+    letter(&mut harness, 'v');
+    set(
+        &mut harness,
+        OptionValue::Keywords(Some(KeywordCase::Lower)),
+    );
+    assert_eq!(numbers(&mut harness), ["2", "1", "3", "1", "2"]);
+    assert!(!state(&harness, |tab| tab.vim.mode.is_visual()));
+}
+
+#[test]
+fn keep_shows_the_query_as_it_was_typed() {
+    let mut harness = pane(Look::macos());
+    let mut shown = Vec::new();
+    for case in [KeywordCase::Upper, KeywordCase::Lower, KeywordCase::Keep] {
+        set(&mut harness, OptionValue::Keywords(Some(case)));
+        shown.push(preview(&mut harness).0);
+    }
+    assert_ne!(shown[2], shown[0]);
+    assert_ne!(shown[2], shown[1]);
+}
+
+/// Clicks the preview, which then has the keyboard.
+fn into_preview(harness: &mut Harness) {
+    let field = place(harness);
+    click(harness, field.center());
+    assert!(focused(harness));
+}
+
+#[test]
+fn esc_gives_the_preview_s_keyboard_up_before_it_closes_the_window_with_either_keys() {
+    for look in [Look::macos(), Look::omarchy()] {
+        for keys in [EditorKeys::Standard, EditorKeys::Vim] {
+            let mut harness = pane(look);
+            set(&mut harness, OptionValue::EditorKeys(Some(keys)));
+            into_preview(&mut harness);
+            harness.press(Key::Escape, Modifiers::NONE);
+            assert!(harness.app.dialog.is_some(), "{} {keys:?}", look.name);
+            assert!(!focused(&harness), "{} {keys:?}", look.name);
+            harness.press(Key::Escape, Modifiers::NONE);
+            assert!(harness.app.dialog.is_none(), "{} {keys:?}", look.name);
+        }
+    }
+}
+
+#[test]
+fn the_omarchy_screen_s_chords_still_work_while_the_preview_has_the_keyboard() {
+    let mut harness = pane(Look::omarchy());
+    into_preview(&mut harness);
+    harness.press(Key::L, Modifiers::CTRL);
+    match &harness.app.dialog {
+        Some(crate::model::Dialog::Settings(dialog)) => {
+            assert_eq!(dialog.pane, SettingsPane::Keyboard);
+        }
+        _ => panic!("Settings closed"),
+    }
+}
+
+#[test]
+fn the_preview_starts_again_where_the_boards_have_it_when_the_window_opens_again() {
+    let mut harness = pane(Look::omarchy());
+    into_preview(&mut harness);
+    letter(&mut harness, 'g');
+    letter(&mut harness, 'g');
+    assert_eq!(numbers(&mut harness), ["1", "1", "2", "3", "4"]);
+    harness.app.apply(Action::CloseDialog);
+    harness.settle();
+    harness.app.apply(Action::ShowSettings);
+    harness
+        .app
+        .apply(Action::ShowSettingsPane(SettingsPane::Editor));
+    harness.settle();
+    assert_eq!(numbers(&mut harness), ["2", "1", "3", "1", "2"]);
+}
+
+#[test]
+fn a_run_key_in_the_preview_sends_nothing_with_a_connection_open() {
+    for look in [Look::macos(), Look::omarchy()] {
+        let mut harness = Harness::with_size(egui::vec2(1440.0, 1100.0));
+        harness.set_look(look);
+        harness.connect_fake();
+        let sent = harness.app.backend.sent.len();
+        harness.app.apply(Action::ShowSettings);
+        harness
+            .app
+            .apply(Action::ShowSettingsPane(SettingsPane::Editor));
+        harness.settle();
+        to_the_end(&mut harness);
+        into_preview(&mut harness);
+        harness.press(Key::Enter, Modifiers::COMMAND);
+        harness.press(Key::Enter, Modifiers::CTRL);
+        harness.press(Key::Enter, Modifiers::COMMAND | Modifiers::SHIFT);
+        assert_eq!(harness.app.backend.sent.len(), sent, "{}", look.name);
+    }
+}
+
+#[test]
+fn shift_tab_comes_back_to_the_preview_in_one_press() {
+    let mut harness = pane(Look::macos());
+    into_preview(&mut harness);
+    harness.press(Key::Tab, Modifiers::NONE);
+    assert!(!focused(&harness));
+    harness.press(Key::Tab, Modifiers::SHIFT);
+    assert!(focused(&harness), "the gutter is no stop of its own");
+}
```

```diff
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -32,6 +32,8 @@ mod dialog_focus_tests;
 #[cfg(test)]
 mod duplicate_tests;
 #[cfg(test)]
+mod editor_preview_tests;
+#[cfg(test)]
 mod editor_results_tests;
 #[cfg(test)]
 mod editor_settings_tests;
```

```diff
diff --git a/src/ui/sql_text.rs b/src/ui/sql_text.rs
--- a/src/ui/sql_text.rs
+++ b/src/ui/sql_text.rs
@@ -2122,6 +2122,7 @@ mod tests {
                     hold_escape,
                     pairs: false,
                     vim: false,
+                    read_only: false,
                     style: sql::format::Style::default(),
                     // A tab character: what Tab put in before the setting.
                     indent: Indent::Tab,
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test --locked --lib editor_preview_tests`
Expected: FAIL to compile (4 errors): `settings` has no `editor_preview_field` yet, and the text field's `Field` no `read_only`.

- [ ] **Step 3: Split the editor's drawing, and draw its preview**

```diff
diff --git a/src/ui/settings/editor_preview.rs b/src/ui/settings/editor_preview.rs
new file mode 100644
--- /dev/null
+++ b/src/ui/settings/editor_preview.rs
@@ -0,0 +1,141 @@
+//! Settings › Editor's preview: the SQL editor itself, over the boards'
+//! Bookshop query (publishers and `book_covers`) as Format lays it out with
+//! the settings, read-only. It shows every setting as it is set: the
+//! keybindings' cursor, the line numbers, the cursor's line, wrap, and the
+//! keywords' case and the commas. Its own state (the cursor, vim's mode) is
+//! the window's, in egui's memory: nothing of it reaches the backend.
+
+use std::sync::{Arc, Mutex, PoisonError};
+
+use egui::text::{CCursor, CCursorRange};
+use egui::{CornerRadius, Id, Sense, Stroke, StrokeKind, Ui, vec2};
+use tabletist_db::Dialect;
+use tabletist_db::sql;
+
+use crate::i18n::Locale;
+use crate::model::{SqlTab, TabId};
+use crate::settings::Settings;
+use crate::theme::{Look, Palette};
+
+/// The boards' query, as typed: Format lays it out. Its keywords are of
+/// both cases, so that Keep shows otherwise than upper and lower case.
+const QUERY: &str = "SELECT p.name AS publisher, count(*) as covers from book_covers c \
+                     JOIN publishers p on p.id = c.publisher_id GROUP BY p.name;";
+
+/// The preview's text field: the editor's state is kept by it.
+pub(crate) fn field_id() -> Id {
+    Id::new("editor preview")
+}
+
+/// The query as Format writes it with `settings` in `look`.
+fn text(settings: &Settings, look: &Look) -> String {
+    let style = crate::ui::sql_text::format_style(settings, look);
+    sql::format::format_with(Dialect::Postgres, QUERY, None, 0, style)
+        .map_or_else(|| QUERY.to_owned(), |formatted| formatted.text)
+}
+
+/// The preview's editor, as the window keeps it: one for every frame, so
+/// the cursor and vim's mode stay between them. A preview not drawn the
+/// frame before (the window closed, or another pane shown) starts again,
+/// with the cursor where the boards have it.
+fn tab(ui: &Ui) -> Arc<Mutex<SqlTab>> {
+    let frame = ui.ctx().cumulative_frame_nr();
+    let drawn = field_id().with("drawn");
+    let last: Option<u64> = ui.data(|data| data.get_temp(drawn));
+    ui.data_mut(|data| {
+        data.insert_temp(drawn, frame);
+        // A new tab's text is empty: `show` sets its query and cursor.
+        if last.is_some_and(|last| last + 1 < frame) {
+            data.remove::<Arc<Mutex<SqlTab>>>(field_id().with("tab"));
+        }
+    });
+    ui.data_mut(|data| {
+        data.get_temp_mut_or_insert_with(field_id().with("tab"), || {
+            let mut tab = SqlTab::new(TabId(0), 0, 0, None);
+            // Nothing to type: vim starts in normal mode.
+            tab.vim = crate::vim::Vim::new(crate::vim::Mode::Normal);
+            tab.focus_editor = false;
+            Arc::new(Mutex::new(tab))
+        })
+        .clone()
+    })
+}
+
+/// The preview under its heading, `side` in from either side: the editor
+/// in a card as tall as the query.
+pub(super) fn show(
+    ui: &mut Ui,
+    side: f32,
+    settings: &Settings,
+    (look, palette, locale): (&Look, &Palette, Locale),
+) {
+    let shared = tab(ui);
+    let mut tab = shared.lock().unwrap_or_else(PoisonError::into_inner);
+    let text = text(settings, look);
+    if tab.text != text {
+        // The settings changed the query's layout: the cursor goes back
+        // to where the boards have it, the start of the third line's
+        // words (`FROM book_covers c`).
+        let third = text.match_indices('\n').nth(1).map_or(0, |(at, _)| at + 1);
+        let words = third + text[third..].len() - text[third..].trim_start_matches(' ').len();
+        // Out of visual mode too: the cursor is where it is put now.
+        tab.vim.to_normal();
+        tab.vim.start_over(&text);
+        tab.text = text;
+        tab.cursor = words;
+        tab.selected = None;
+        let mut state = egui::TextEdit::load_state(ui.ctx(), field_id()).unwrap_or_default();
+        let at = tab.text[..words].chars().count();
+        state
+            .cursor
+            .set_char_range(Some(CCursorRange::one(CCursor::new(at))));
+        egui::TextEdit::store_state(ui.ctx(), field_id(), state);
+    }
+    // As tall as the field showed the query last frame, the card's border
+    // round it; before it has drawn, as its lines would be.
+    let height_id = field_id().with("height");
+    let height = ui
+        .data(|data| data.get_temp::<f32>(height_id))
+        .unwrap_or_else(|| crate::ui::sql_text::preview_height(ui, look, &tab.text))
+        + 2.0;
+    let (place, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
+    let card = place.shrink2(vec2(side, 0.0));
+    if !card.is_positive() {
+        return;
+    }
+    let mut inside = ui.new_child(
+        egui::UiBuilder::new()
+            .id_salt("editor preview")
+            .max_rect(card.shrink(1.0)),
+    );
+    inside.set_clip_rect(card.shrink(1.0).intersect(ui.clip_rect()));
+    let shown = crate::ui::sql_text::preview(
+        &mut inside,
+        &mut tab,
+        field_id(),
+        settings,
+        (look, palette, locale),
+    );
+    if ui.data(|data| data.get_temp::<f32>(height_id)) != Some(shown) {
+        ui.data_mut(|data| data.insert_temp(height_id, shown));
+        ui.ctx().request_repaint();
+    }
+    // The card's border, as the Appearance pane's preview has it; in the
+    // accent while the preview has the keyboard, which its letters then
+    // move.
+    let radius = if look.terminal { 3 } else { 10 };
+    let focused = ui.memory(|memory| memory.has_focus(field_id()));
+    let border = if focused {
+        palette.accent
+    } else if look.terminal {
+        palette.outline
+    } else {
+        palette.border
+    };
+    ui.painter().rect_stroke(
+        card,
+        CornerRadius::same(radius),
+        Stroke::new(1.0, border),
+        StrokeKind::Inside,
+    );
+}
```

```diff
diff --git a/src/ui/settings/mod.rs b/src/ui/settings/mod.rs
--- a/src/ui/settings/mod.rs
+++ b/src/ui/settings/mod.rs
@@ -12,6 +12,7 @@ mod keyboard_sheet;
 mod keyboard_terminal;
 #[cfg(test)]
 pub(crate) use keyboard_sheet::search_id as keyboard_search_id;
+mod editor_preview;
 mod preview;
 mod sheet;
 mod terminal;
@@ -727,7 +728,17 @@ fn asked(keymap: &Keymap, event: &egui::Event, held: egui::Modifiers, free: bool
 /// before it left: the cursor where they moved it, an option as they set
 /// it. `j` then `l` changes the row `j` moved to, and `l` twice steps
 /// twice.
-fn keys(app: &App, ctx: &egui::Context, dialog: &SettingsDialog, actions: &mut Vec<Action>) {
+///
+/// While the editor's preview has the keyboard (`preview`), its letters
+/// and Esc are its own, as in the editor: only the chords that open the
+/// file and change the pane are the screen's.
+fn keys(
+    app: &App,
+    ctx: &egui::Context,
+    dialog: &SettingsDialog,
+    preview: bool,
+    actions: &mut Vec<Action>,
+) {
     let free = !super::focus::on_control(ctx);
     // As `App::apply` will have them once it has run the actions so far.
     let mut cursor = dialog.row;
@@ -742,6 +753,9 @@ fn keys(app: &App, ctx: &egui::Context, dialog: &SettingsDialog, actions: &mut V
             let Some(asked) = asked(&keymap, event, held, free) else {
                 return true;
             };
+            if preview && !matches!(asked, Asked::Edit | Asked::Pane { .. }) {
+                return true;
+            }
             let repeat = matches!(event, egui::Event::Key { repeat: true, .. });
             match asked {
                 // A fresh press only: every repeat of a key held down
@@ -846,6 +860,12 @@ fn keys(app: &App, ctx: &egui::Context, dialog: &SettingsDialog, actions: &mut V
     }
 }
 
+/// The editor preview's text field, for the tests.
+#[cfg(test)]
+pub(crate) fn editor_preview_field() -> egui::Id {
+    editor_preview::field_id()
+}
+
 pub fn show(app: &mut App, ctx: &egui::Context) {
     let Some(Dialog::Settings(dialog)) = &app.dialog else {
         return;
@@ -863,10 +883,15 @@ pub fn show(app: &mut App, ctx: &egui::Context) {
         // The screen's keys act on its cursor. The sheet has none: its
         // controls are reached with Tab. The Keyboard pane's are its own,
         // and its `/` line's field has them while it is open.
-        if !keyboard {
-            keys(app, ctx, dialog, &mut actions);
-        } else if dialog.keyboard.recording.is_none() && !dialog.keyboard.filtering {
-            keyboard_terminal::keys(app, ctx, dialog, &mut actions);
+        // The editor's preview, while it has the keyboard, has the
+        // screen's letters and Esc: they are its own, as in the editor.
+        let in_preview = ctx.memory(|memory| memory.has_focus(editor_preview::field_id()));
+        if keyboard {
+            if dialog.keyboard.recording.is_none() && !dialog.keyboard.filtering {
+                keyboard_terminal::keys(app, ctx, dialog, &mut actions);
+            }
+        } else {
+            keys(app, ctx, dialog, in_preview, &mut actions);
         }
         terminal::show(app, ctx, dialog, &mut actions);
     } else {
```

```diff
diff --git a/src/ui/settings/sheet.rs b/src/ui/settings/sheet.rs
--- a/src/ui/settings/sheet.rs
+++ b/src/ui/settings/sheet.rs
@@ -197,7 +197,7 @@ pub(super) fn show(
                 SettingsPane::General => content(
                     &mut pane,
                     ("General", Some(GENERAL)),
-                    (SettingsPane::General.sections(), false, None),
+                    (SettingsPane::General.sections(), Preview::None, None),
                     // As the window shows them: what the look has where the
                     // settings do not say (the grouping of numbers).
                     &super::shown(app),
@@ -208,7 +208,7 @@ pub(super) fn show(
                 SettingsPane::Appearance => content(
                     &mut pane,
                     ("Appearance", Some(APPEARANCE)),
-                    (SettingsPane::Appearance.sections(), true, None),
+                    (SettingsPane::Appearance.sections(), Preview::Grid, None),
                     &super::shown(app),
                     limit,
                     &skin,
@@ -224,7 +224,7 @@ pub(super) fn show(
                     ),
                     (
                         SettingsPane::Editor.sections(),
-                        false,
+                        Preview::Editor,
                         // Show History, which shows the window's tab.
                         Some((
                             OptionId::HistoryKeep,
@@ -258,8 +258,9 @@ pub(super) fn show(
             let bottom = limit.max(rows) + foot;
             // The last row leaves its rule to the footer's: where there is
             // room between them, it has one of its own. The preview, which
-            // ends the Appearance pane, has its border.
-            if rows < limit - 0.5 && dialog.pane != SettingsPane::Appearance {
+            // ends the Appearance and Editor panes, has its border.
+            let previewed = matches!(dialog.pane, SettingsPane::Appearance | SettingsPane::Editor);
+            if rows < limit - 0.5 && !previewed {
                 let across = egui::Rangef::new(room.left() + NAV + SIDE, room.right() - SIDE);
                 widgets::hline(ui, across, rows - 0.5, skin.palette.surface);
             }
@@ -368,11 +369,21 @@ const GENERAL: &str =
 const APPEARANCE: &str =
     "Theme, type and how dense the grid is. The preview updates as you change things.";
 
-/// A pane's sections, each its heading and its options; whether it has
-/// the preview; and the button after one of its rows.
+/// The preview that ends a pane, where it has one.
+#[derive(Clone, Copy, PartialEq, Eq)]
+enum Preview {
+    None,
+    /// Appearance's: a connection's bar and two rows of a table.
+    Grid,
+    /// Editor's: the SQL editor over the boards' query.
+    Editor,
+}
+
+/// A pane's sections, each its heading and its options; the preview it
+/// has; and the button after one of its rows.
 type Sections<'a> = (
     &'a [(&'static str, &'a [OptionId])],
-    bool,
+    Preview,
     Option<(OptionId, After<'a>)>,
 );
 
@@ -468,14 +479,18 @@ fn content(
             }
             // Under the options, 14 under the last of them, the preview,
             // and 14 under it.
-            if preview {
+            if preview != Preview::None {
                 let name = skin.say("Preview");
                 ui.add_space(14.0);
                 widgets::section_label(&name, look, palette)
                     .layout(ui.ctx())
                     .label(ui);
                 ui.add_space(2.0);
-                super::preview::show(ui, (&name, 0.0), settings, look, palette);
+                if preview == Preview::Grid {
+                    super::preview::show(ui, (&name, 0.0), settings, look, palette);
+                } else {
+                    super::editor_preview::show(ui, 0.0, settings, (look, palette, skin.locale));
+                }
                 ui.add_space(14.0);
             }
         });
```

```diff
diff --git a/src/ui/settings/terminal.rs b/src/ui/settings/terminal.rs
--- a/src/ui/settings/terminal.rs
+++ b/src/ui/settings/terminal.rs
@@ -529,8 +529,7 @@ fn appearance_rows(
 }
 
 /// The Editor pane's options, a row each, under the headings of their
-/// groups, as far as they are built. Their columns line up across the
-/// groups.
+/// groups, then its preview. Their columns line up across the groups.
 fn editor_rows(
     ui: &mut Ui,
     settings: &Settings,
@@ -573,6 +572,17 @@ fn editor_rows(
             index += 1;
         }
     }
+    // Under the options, the editor's preview, no row of the cursor's: 4
+    // over and under it, as far in as the rows' words.
+    heading(ui, "Preview", 12.0, skin);
+    ui.add_space(4.0);
+    super::editor_preview::show(
+        ui,
+        ROW_SIDE,
+        settings,
+        (skin.look, skin.palette, skin.locale),
+    );
+    ui.add_space(4.0);
 }
 
 /// The colours the theme gave, eight of them as the board has them, and
```

```diff
diff --git a/src/ui/sql_text.rs b/src/ui/sql_text.rs
--- a/src/ui/sql_text.rs
+++ b/src/ui/sql_text.rs
@@ -484,14 +484,80 @@ fn line_rows(galley: &Galley) -> Vec<usize> {
     starts
 }
 
+/// How the editor draws, as the settings and the look have them.
+struct Shown {
+    look: Look,
+    palette: Palette,
+    locale: Locale,
+    numbering: crate::settings::LineNumbers,
+    style: sql::format::Style,
+    indent: Indent,
+    wraps: bool,
+    highlight: bool,
+    pairs: bool,
+    vim: bool,
+}
+
+impl Shown {
+    fn of(
+        settings: &crate::settings::Settings,
+        look: Look,
+        palette: Palette,
+        locale: Locale,
+    ) -> Self {
+        Self {
+            numbering: settings.line_numbers_in(look.terminal),
+            style: format_style(settings, &look),
+            indent: settings.indent,
+            wraps: settings.wrap,
+            highlight: settings.highlight_line,
+            pairs: settings.close_pairs,
+            vim: settings.editor_keys_in(look.terminal) == crate::settings::EditorKeys::Vim,
+            look,
+            palette,
+            locale,
+        }
+    }
+
+    /// The text field named `name` beside `gutter`: one that edits, or
+    /// with `read_only` the preview's.
+    fn field<'a>(
+        &'a self,
+        (id, name): (Id, &'a str),
+        gutter: &Gutter,
+        dialect: Dialect,
+        hold_escape: bool,
+        read_only: bool,
+    ) -> Field<'a> {
+        Field {
+            id,
+            name,
+            left: gutter.text_left,
+            dialect,
+            look: &self.look,
+            palette: &self.palette,
+            style: self.style,
+            indent: self.indent,
+            wraps: self.wraps,
+            hold_escape,
+            pairs: self.pairs,
+            vim: self.vim,
+            read_only,
+        }
+    }
+}
+
+/// Where the editor draws: its pane, the text's part of it beside the
+/// gutter, and the slot behind the text the bands go in.
+struct Place {
+    pane: Rect,
+    text_pane: Rect,
+    backdrop: egui::layers::ShapeIdx,
+}
+
 pub fn show(app: &mut App, ui: &mut Ui, tab: ConnTabId, id: TabId) {
-    let (look, palette, locale) = (app.look, app.palette, app.locale);
-    let numbering = app.settings.line_numbers_in(look.terminal);
-    let (style, indent) = (format_style(&app.settings, &look), app.settings.indent);
-    let wraps = app.settings.wrap;
-    let highlight = app.settings.highlight_line;
-    let pairs = app.settings.close_pairs;
-    let vim = app.settings.editor_keys_in(look.terminal) == crate::settings::EditorKeys::Vim;
+    let shown = Shown::of(&app.settings, app.look, app.palette, app.locale);
+    let locale = shown.locale;
     let Some(workspace) = app.workspace_mut(tab) else {
         return;
     };
@@ -500,42 +566,134 @@ pub fn show(app: &mut App, ui: &mut Ui, tab: ConnTabId, id: TabId) {
     let Some(sql_tab) = workspace.sql_tab_mut(id) else {
         return;
     };
-    let pane = ui.max_rect();
     let lines = sql_tab.text.bytes().filter(|b| *b == b'\n').count() + 1;
-    let gutter = Gutter::new(ui, &look, lines, numbering.shown());
+    let gutter = Gutter::new(ui, &shown.look, lines, shown.numbering.shown());
     let editor = editor_id(tab, id);
+    let name = gettext(locale, "SQL");
+    // An open completion list takes Esc: the editor keeps the keyboard.
+    // Not one whose row this frame inserts: Esc is the editor's again.
+    let hold_escape = sql_tab.completion.as_ref().is_some_and(|list| !list.accept);
+    let field = shown.field((editor, &name), &gutter, dialect, hold_escape, false);
+    let (mut edited, place) = draw_text(ui, sql_tab, (&field, &gutter), locale);
+    // The open completion list, and the characters its word starts and
+    // ends at. (After the field drew: a list whose row was accepted went
+    // in there and is off the tab, so the one here is open.)
+    let completing = sql_tab.completion.as_ref().map(|list| {
+        let word = &list.site.word;
+        let start = char_index(&sql_tab.text, word.start);
+        let end = char_index(&sql_tab.text, word.end);
+        (list.clone(), start..end)
+    });
+    let (cursor_line, _) = sql_tab.line_col();
+    let error_line = sql_tab.error_mark().map(|(line, _)| line);
+    // The statement Run executes, from the tokens the colours came from.
+    let parsed = parsed(ui.ctx(), editor, dialect, &shown.palette, &sql_tab.text);
+    // The first row of each line: a wrapped line has more than one.
+    let starts = line_rows(&edited.galley);
+    let statement = sql::statement_at(&parsed.statements, sql_tab.cursor)
+        .and_then(|statement| edited.span(&starts, &statement_lines(statement)));
+    if edited.focused && tree_has_arrows {
+        app.actions
+            .push(Action::SqlEditorFocused { tab, sql_tab: id });
+    }
+    if edited.typed {
+        app.actions.push(Action::SqlTyped { tab, sql_tab: id });
+    }
+    // Vim's `:`: the prompt the terminal look has. (The macOS look has
+    // none.)
+    if edited.prompt && shown.look.terminal {
+        app.actions.push(Action::EditorCommand { tab, sql_tab: id });
+    }
+    let spot = Spot {
+        tab,
+        sql_tab: id,
+        text_pane: place.text_pane,
+        locale,
+    };
+    let asked = completion_list(ui, completing.as_ref(), &mut edited, &field, &spot);
+    app.actions.extend(asked);
+    let lines = Lines {
+        starts: &starts,
+        cursor: cursor_line,
+        error: error_line,
+        statement,
+    };
+    draw_lines(ui, &shown, &gutter, &place, &edited, &lines, false);
+}
+
+/// Settings › Editor's preview: `sql_tab`'s script in the editor, as the
+/// settings have it now, read-only and named "Editor preview". Its cursor's
+/// line is lit whether or not it has the keyboard, and it has no statement
+/// bar and no gutter to press: one Tab stop. Nothing of it reaches a
+/// database. Returns how tall the field is that shows all of the script:
+/// the preview's height from the next frame on.
+pub fn preview(
+    ui: &mut Ui,
+    sql_tab: &mut SqlTab,
+    id: Id,
+    settings: &crate::settings::Settings,
+    (look, palette, locale): (&Look, &Palette, Locale),
+) -> f32 {
+    let shown = Shown::of(settings, *look, *palette, locale);
+    let lines = sql_tab.text.bytes().filter(|b| *b == b'\n').count() + 1;
+    let gutter = Gutter::new(ui, &shown.look, lines, shown.numbering.shown());
+    let name = gettext(locale, "Editor preview");
+    let field = shown.field((id, &name), &gutter, Dialect::Postgres, false, true);
+    let (edited, place) = draw_text(ui, sql_tab, (&field, &gutter), locale);
+    let starts = line_rows(&edited.galley);
+    let (cursor_line, _) = sql_tab.line_col();
+    let lines = Lines {
+        starts: &starts,
+        cursor: cursor_line,
+        error: None,
+        statement: None,
+    };
+    draw_lines(ui, &shown, &gutter, &place, &edited, &lines, true);
+    // The rows as laid out, with the field's 8 above and under them.
+    edited.galley.rect.height() + 16.0
+}
+
+/// How tall the preview is that shows all of `text` before it has drawn:
+/// its lines in the field's font, with the field's 8 above and under them.
+pub fn preview_height(ui: &Ui, look: &Look, text: &str) -> f32 {
+    let font = role(look).font_id(look.faces);
+    let line = ui.fonts_mut(|fonts| fonts.row_height(&font)) + ui.spacing().extra_text_line_spacing;
+    let lines = text.bytes().filter(|b| *b == b'\n').count() + 1;
+    lines as f32 * line + 16.0
+}
+
+/// The editor's text field, scrolling beside the gutter, and the gutter's
+/// press (not the preview's).
+fn draw_text(
+    ui: &mut Ui,
+    sql_tab: &mut SqlTab,
+    (field, gutter): (&Field<'_>, &Gutter),
+    locale: Locale,
+) -> (Edited, Place) {
+    let pane = ui.max_rect();
+    let editor = field.id;
     // Behind the text: filled in once the rows are laid out.
     let backdrop = ui.painter().add(Shape::Noop);
     // The text scrolls beside the gutter, which stays put: both ways, or
     // down alone where it wraps.
     let text_pane = Rect::from_min_max(pos2(pane.left() + gutter.width, pane.top()), pane.max);
-    let name = gettext(locale, "SQL");
-    let field = Field {
-        id: editor,
-        name: &name,
-        left: gutter.text_left,
-        dialect,
-        look: &look,
-        palette: &palette,
-        style,
-        indent,
-        wraps,
-        // An open completion list takes Esc: the editor keeps the
-        // keyboard. Not one whose row this frame inserts: Esc is the
-        // editor's again.
-        hold_escape: sql_tab.completion.as_ref().is_some_and(|list| !list.accept),
-        pairs,
-        vim,
-    };
     let mut child = ui.new_child(egui::UiBuilder::new().id_salt("text").max_rect(text_pane));
     child.set_clip_rect(text_pane.intersect(ui.clip_rect()));
     // Wrapped, the lines never run past the pane: it scrolls down alone.
-    let scrolled = egui::ScrollArea::new([!wraps, true])
+    let scrolled = egui::ScrollArea::new([!field.wraps, true])
         .id_salt("scroll")
         .auto_shrink([false, false])
-        .show(&mut child, |ui| edit(ui, sql_tab, &field));
+        .show(&mut child, |ui| edit(ui, sql_tab, field));
     ui.data_mut(|data| data.insert_temp(editor, ScrollId(scrolled.id)));
     let mut edited = scrolled.inner;
+    let place = Place {
+        pane,
+        text_pane,
+        backdrop,
+    };
+    if field.read_only {
+        return (edited, place);
+    }
     // The gutter is the editor's too: a press on it gives the editor the
     // keys, with the cursor at the start of the line pressed. (After the
     // field, which gives the keys up when a press lands outside it.)
@@ -570,58 +728,47 @@ pub fn show(app: &mut App, ui: &mut Ui, tab: ConnTabId, id: TabId) {
         ui.memory_mut(|memory| memory.request_focus(editor));
         edited.focused = true;
     }
-    // The open completion list, and the characters its word starts and
-    // ends at. (After the field drew: a list whose row was accepted went
-    // in there and is off the tab, so the one here is open.)
-    let completing = sql_tab.completion.as_ref().map(|list| {
-        let word = &list.site.word;
-        let start = char_index(&sql_tab.text, word.start);
-        let end = char_index(&sql_tab.text, word.end);
-        (list.clone(), start..end)
-    });
-    let (cursor_line, _) = sql_tab.line_col();
-    let error_line = sql_tab.error_mark().map(|(line, _)| line);
-    // The statement Run executes, from the tokens the colours came from.
-    let parsed = parsed(ui.ctx(), editor, dialect, &palette, &sql_tab.text);
-    // The first row of each line: a wrapped line has more than one.
-    let starts = line_rows(&edited.galley);
-    let statement = sql::statement_at(&parsed.statements, sql_tab.cursor)
-        .and_then(|statement| edited.span(&starts, &statement_lines(statement)));
-    if edited.focused && tree_has_arrows {
-        app.actions
-            .push(Action::SqlEditorFocused { tab, sql_tab: id });
-    }
-    if edited.typed {
-        app.actions.push(Action::SqlTyped { tab, sql_tab: id });
-    }
-    // Vim's `:`: the prompt the terminal look has. (The macOS look has
-    // none.)
-    if edited.prompt && look.terminal {
-        app.actions.push(Action::EditorCommand { tab, sql_tab: id });
-    }
-    let spot = Spot {
-        tab,
-        sql_tab: id,
-        text_pane,
-        locale,
-    };
-    let asked = completion_list(ui, completing.as_ref(), &mut edited, &field, &spot);
-    app.actions.extend(asked);
+    (edited, place)
+}
+
+/// What the gutter and the bands mark: each line's first row, the
+/// cursor's line, the line of the last run's error, and the rows of the
+/// statement Run executes.
+struct Lines<'a> {
+    starts: &'a [usize],
+    cursor: usize,
+    error: Option<usize>,
+    statement: Option<egui::Rangef>,
+}
+
+/// Behind the text, the statement's band and bar and the cursor's line;
+/// in the gutter, the numbers of the rows in sight. With `always`, the
+/// cursor's line is lit without the keyboard too (the preview).
+fn draw_lines(
+    ui: &Ui,
+    shown: &Shown,
+    gutter: &Gutter,
+    place: &Place,
+    edited: &Edited,
+    lines: &Lines<'_>,
+    always: bool,
+) {
+    let (look, palette, pane) = (&shown.look, &shown.palette, place.pane);
     let across = |x: egui::Rangef, y| Rect::from_x_y_ranges(x, y);
     let fill = |rect, color| Shape::rect_filled(rect, CornerRadius::ZERO, color);
     let mut behind = Vec::new();
     // macOS: a faint band behind the statement, the gutter included, and a
     // bar beside it. The terminal: the bar alone, in the accent.
-    if let Some(y) = statement
+    if let Some(y) = lines.statement
         && !look.terminal
     {
         behind.push(fill(across(pane.x_range(), y), palette.panel));
     }
     // The cursor's line, over the band, while the editor has the keys,
     // unless the settings turn it off.
-    if edited.focused
-        && highlight
-        && let Some(y) = edited.span(&starts, &(cursor_line..=cursor_line))
+    if (edited.focused || always)
+        && shown.highlight
+        && let Some(y) = edited.span(lines.starts, &(lines.cursor..=lines.cursor))
     {
         let color = if look.terminal {
             palette.selection
@@ -630,13 +777,13 @@ pub fn show(app: &mut App, ui: &mut Ui, tab: ConnTabId, id: TabId) {
         };
         behind.push(fill(across(pane.x_range(), y), color));
     }
-    if let Some(y) = statement {
+    if let Some(y) = lines.statement {
         let bar = egui::Rangef::new(pane.left() + gutter.bar.min, pane.left() + gutter.bar.max);
-        behind.push(fill(across(bar, y), bar_color(&look, &palette)));
+        behind.push(fill(across(bar, y), bar_color(look, palette)));
     }
-    ui.painter().set(backdrop, Shape::Vec(behind));
+    ui.painter().set(place.backdrop, Shape::Vec(behind));
     // The numbers of the rows in sight, where the gutter has numbers.
-    if !numbering.shown() {
+    if !shown.numbering.shown() {
         return;
     }
     let clip = ui.clip_rect();
@@ -649,13 +796,13 @@ pub fn show(app: &mut App, ui: &mut Ui, tab: ConnTabId, id: TabId) {
         if top > clip.bottom() {
             break;
         }
-        let Ok(line) = starts.binary_search(&index) else {
+        let Ok(line) = lines.starts.binary_search(&index) else {
             continue;
         };
         let line = line + 1;
-        let color = if error_line == Some(line) {
+        let color = if lines.error == Some(line) {
             palette.danger
-        } else if line == cursor_line {
+        } else if line == lines.cursor {
             // The terminal's one absolute number stands out.
             if look.terminal {
                 palette.warning
@@ -667,13 +814,13 @@ pub fn show(app: &mut App, ui: &mut Ui, tab: ConnTabId, id: TabId) {
         } else {
             palette.faint
         };
-        let relative = numbering == crate::settings::LineNumbers::Relative;
-        let number = gutter_number(line, cursor_line, relative);
+        let relative = shown.numbering == crate::settings::LineNumbers::Relative;
+        let number = gutter_number(line, lines.cursor, relative);
         widgets::paint_text_right(
             ui,
             pane.left() + gutter.numbers_right,
             top + row.height() / 2.0,
-            Text::one(&look, role(&look), &number.to_string(), color),
+            Text::one(look, role(look), &number.to_string(), color),
         );
     }
 }
@@ -782,12 +929,17 @@ struct Field<'a> {
     /// The keybindings are vim's: the field has normal, insert and visual
     /// modes.
     vim: bool,
+    /// The preview's: the text cannot change, and Tab moves on.
+    read_only: bool,
 }
 
 /// How Format writes, as the settings have it in `look`: its keyword case
 /// and commas, and a `CASE`'s parts in by the indent (four spaces for a
 /// tab: the river is set in spaces).
-fn format_style(settings: &crate::settings::Settings, look: &Look) -> sql::format::Style {
+pub(crate) fn format_style(
+    settings: &crate::settings::Settings,
+    look: &Look,
+) -> sql::format::Style {
     use crate::settings::{Commas, KeywordCase};
     use sql::format::Case;
     sql::format::Style {
@@ -1454,9 +1606,14 @@ fn vim_keys(ui: &mut Ui, sql_tab: &mut SqlTab, field: &Field<'_>) -> (bool, bool
     }
     let mut prompt = false;
     let register_id = Id::new("vim register");
-    let mut register: vim::Register = ui
-        .data(|data| data.get_temp(register_id))
-        .unwrap_or_default();
+    // The preview's yanks and deletes are its own: the editors' register
+    // is not theirs to fill.
+    let mut register: vim::Register = if field.read_only {
+        vim::Register::default()
+    } else {
+        ui.data(|data| data.get_temp(register_id))
+            .unwrap_or_default()
+    };
     let time = ui.input(|input| input.time);
     let unit = indent_unit(field.indent);
     let chords = VimChords::of(ui, field.look);
@@ -1498,6 +1655,13 @@ fn vim_keys(ui: &mut Ui, sql_tab: &mut SqlTab, field: &Field<'_>) -> (bool, bool
             let step = sql_tab
                 .vim
                 .key(&sql_tab.text, cursor, input, &mut register, unit);
+            if field.read_only && (step.text.is_some() || sql_tab.vim.mode == vim::Mode::Insert) {
+                // The preview's text does not change: what would change it
+                // is dropped, and so is insert mode, which has nothing to
+                // type; vim stays in normal mode.
+                sql_tab.vim.to_normal();
+                continue;
+            }
             if let Some(text) = &step.text {
                 // Undo puts the cursor back, not a selection.
                 let before = CCursorRange::one(CCursor::new(cursor));
@@ -1510,6 +1674,7 @@ fn vim_keys(ui: &mut Ui, sql_tab: &mut SqlTab, field: &Field<'_>) -> (bool, bool
             }
             changed = true;
             match step.then {
+                Some(vim::Then::Format(_)) if field.read_only => {}
                 Some(vim::Then::Format(lines)) => {
                     // Format formats the statements a selection touches,
                     // or with none every one.
@@ -1546,7 +1711,9 @@ fn vim_keys(ui: &mut Ui, sql_tab: &mut SqlTab, field: &Field<'_>) -> (bool, bool
             break;
         }
     }
-    ui.data_mut(|data| data.insert_temp(register_id, register));
+    if !field.read_only {
+        ui.data_mut(|data| data.insert_temp(register_id, register));
+    }
     if !later.is_empty() {
         ui.data_mut(|data| data.insert_temp(later_id, later));
         ui.ctx().request_repaint();
@@ -1604,11 +1771,21 @@ fn edit(ui: &mut Ui, sql_tab: &mut SqlTab, field: &Field<'_>) -> Edited {
         sql_tab.vim.start(&sql_tab.text);
     }
     let (vimmed, prompt) = vim_keys(ui, sql_tab, field);
+    // The preview's Esc, with the standard keys, gives the keyboard up and
+    // is taken: the next closes the window. (Vim's is `vim_keys`'.)
+    if field.read_only
+        && !field.vim
+        && ui.memory(|memory| memory.has_focus(field.id))
+        && ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
+    {
+        ui.memory_mut(|memory| memory.surrender_focus(field.id));
+    }
     // Before the field is drawn, so this frame shows the formatted text.
     // An indent is an outside edit too: what the field shows, and where
     // its caret goes, follow as after Format.
-    let indented = indent_script(ui, sql_tab, field);
-    let paired = close_pairs(ui, sql_tab, field);
+    // The preview's text does not change.
+    let indented = !field.read_only && indent_script(ui, sql_tab, field);
+    let paired = !field.read_only && close_pairs(ui, sql_tab, field);
     let formatted = std::mem::take(&mut sql_tab.format) && format_script(ui, sql_tab, field);
     let formatted = formatted || indented || paired || vimmed;
     // The cursor put somewhere from outside (an error's mark): set before
@@ -1658,18 +1835,29 @@ fn edit(ui: &mut Ui, sql_tab: &mut SqlTab, field: &Field<'_>) -> Edited {
     if block {
         ui.visuals_mut().text_cursor.stroke.color = Color32::TRANSPARENT;
     }
-    let output = egui::TextEdit::multiline(&mut sql_tab.text)
+    // The preview's text, which the field reads and cannot change.
+    let kept = field.read_only.then(|| sql_tab.text.clone());
+    let mut fixed: &str = kept.as_deref().unwrap_or_default();
+    let buffer: &mut dyn egui::TextBuffer = if field.read_only {
+        &mut fixed
+    } else {
+        &mut sql_tab.text
+    };
+    let output = egui::TextEdit::multiline(buffer)
         .id(field.id)
         .font(font)
-        // Tab indents; Esc gives the keys up unless `hold_escape`.
-        .lock_focus(true)
+        // Tab indents; Esc gives the keys up unless `hold_escape`. The
+        // preview's Tab moves on: it is one stop.
+        .lock_focus(!field.read_only)
         .frame(egui::Frame::new().inner_margin(margin))
         .desired_width(width)
         .desired_rows(rows as usize)
         .layouter(&mut layouter)
         .show(ui);
     ui.visuals_mut().text_cursor.stroke = caret_stroke;
-    if block && output.response.has_focus() {
+    // The preview shows vim's cursor without the keyboard too: the mode
+    // the keybindings give.
+    if block && (output.response.has_focus() || field.read_only) {
         let at = if sql_tab.vim.mode.is_visual() {
             sql_tab.vim.head()
         } else {
@@ -1688,8 +1876,12 @@ fn edit(ui: &mut Ui, sql_tab: &mut SqlTab, field: &Field<'_>) -> Edited {
             field.palette,
         );
     }
-    crate::ui::focus::region(ui, crate::ui::focus::Region::Editor, output.response.rect);
-    crate::ui::focus::claim(ui, crate::ui::focus::Region::Editor, &output.response);
+    // The workspace's editor region, which F6 and the pane keys move to:
+    // not the preview's.
+    if !field.read_only {
+        crate::ui::focus::region(ui, crate::ui::focus::Region::Editor, output.response.rect);
+        crate::ui::focus::claim(ui, crate::ui::focus::Region::Editor, &output.response);
+    }
     // The script fills its pane: the caret says where the keyboard is.
     crate::ui::focus::hint(
         ui,
@@ -1700,6 +1892,9 @@ fn edit(ui: &mut Ui, sql_tab: &mut SqlTab, field: &Field<'_>) -> Edited {
     // The name only: the field keeps the role and the value egui gave it.
     ui.ctx().accesskit_node_builder(field.id, |node| {
         node.set_label(field.name);
+        if field.read_only {
+            node.set_read_only();
+        }
     });
     if focus {
         output.response.request_focus();
@@ -1744,7 +1939,7 @@ fn edit(ui: &mut Ui, sql_tab: &mut SqlTab, field: &Field<'_>) -> Edited {
     }
     // Vim's Esc is its own: insert mode's goes to normal mode, and normal
     // mode's leaves the editor when `vim_keys` says so.
-    if (field.hold_escape || field.vim) && output.response.has_focus() {
+    if (field.hold_escape || field.vim || field.read_only) && output.response.has_focus() {
         // The field set its own filter while it drew (the arrows and Tab).
         // egui replaces the whole filter, so those are named again. The
         // filter is read at the start of the next pass, so holding Esc takes
@@ -1754,7 +1949,8 @@ fn edit(ui: &mut Ui, sql_tab: &mut SqlTab, field: &Field<'_>) -> Edited {
             memory.set_focus_lock_filter(
                 field.id,
                 egui::EventFilter {
-                    tab: true,
+                    // The preview's Tab moves on: it is one stop.
+                    tab: !field.read_only,
                     horizontal_arrows: true,
                     vertical_arrows: true,
                     escape: true,
```

- [ ] **Step 4: Run the tests to see them pass**

Run: `cargo test --locked --lib -- editor_preview_tests sql_text ui::tests vim_tests editor_settings_tests typing_settings_tests complete_tests pane_keys_tests history format_settings_tests appearance_settings_tests settings_access_tests`
Expected: PASS. Then the four checks above.

- [ ] **Step 5: Compare with the boards by eye**

Not a test, and nothing of it is committed. Shoot Settings › Editor in both looks (`Harness::for_shots`, 1440 × 1500 on macOS and 1920 × 1080 on Omarchy, a mouse wheel down to the sheet's end) and set the preview beside the two "Settings, Editor" boards'. Remove the scene before committing.

- [ ] **Step 6: Commit**

```bash
git add src/ui/editor_preview_tests.rs src/ui/mod.rs src/ui/settings/editor_preview.rs src/ui/settings/mod.rs src/ui/settings/sheet.rs src/ui/settings/terminal.rs src/ui/sql_text.rs
git commit -m "Preview the SQL editor in Settings › Editor, read-only, as the settings have it"
```

---

### Task 2: The website

**Files:**
- Modify: `docs/_reference/settings-and-files.md`

- [ ] **Step 1: Say what the preview is**

```diff
diff --git a/docs/_reference/settings-and-files.md b/docs/_reference/settings-and-files.md
--- a/docs/_reference/settings-and-files.md
+++ b/docs/_reference/settings-and-files.md
@@ -44,7 +44,15 @@ are made in the app, so it needs no connection and reads no database.
 | **Window header** (Omarchy: connection bar) | Tint + stripe (Omarchy: tint + label), or Stripe only (Omarchy: label only). Production keeps its tint either way | Tint + stripe |
 
 A third pane, **Editor**, sets how the SQL editor runs and types. Every
-open SQL editor follows each change at once.
+open SQL editor follows each change at once, and so does the preview
+under the options: the SQL editor itself over a query of the Bookshop's,
+laid out as Format would with the Keywords and Commas set, and showing
+the keybindings' cursor, the line numbers, the cursor's line and wrap as
+set. It can be read, selected and copied, not changed, and it never
+reaches a database. While it has the keyboard its border is in the
+accent, and Esc gives the keyboard back before a second closes the
+window; on the Omarchy screen its letters are the preview's then, and
+only Ctrl+E and the pane keys are the screen's.
 
 | Option | Choices | Default |
 | --- | --- | --- |
@@ -58,15 +66,12 @@ open SQL editor follows each change at once.
 | **Autocomplete** | As you type, On Ctrl+Space (the list opens only when you ask), or Off. Keywords are offered in the case **Keywords** sets, upper case with Keep | As you type |
 | **Close brackets and quotes** (Omarchy: close brackets) | On or off. On, `(`, `[`, `{` and a quote (and a backtick, which quotes names on MySQL and SQLite) get their closing one after the cursor where nothing follows that it would split off. A quote is not paired inside a word, right after the same quote, or inside a string, a quoted name or a comment. A closing one typed before its twin steps over it, and Backspace takes both out while the pair is empty. Typed over a selection, the two go round it, which stays selected and undoes in one step. A paste, an input method's text, or two characters arriving at once go in as they come | On |
 | **Line numbers** | On or off on macOS and Windows; relative, absolute or off in the Omarchy look. On writes `absolute`; a `relative` set in the file shows as on and stays | On (absolute) on macOS and Windows, relative in the Omarchy look |
-| **Current line highlight** (macOS and Windows) | On or off: the line the cursor is on is filled while the editor has the keyboard. The Omarchy look has no row: it follows the key, set in the file | On |
+| **Current line highlight** (macOS and Windows) | On or off: the line the cursor is on is filled while the editor has the keyboard (in the preview, always). The Omarchy look has no row: it follows the key, set in the file | On |
 | **Wrap** | On or off. On, a line longer than the editor wraps at its width and the editor scrolls down alone. The line keeps one number, on its first row, and a click in the gutter beside any of its rows puts the cursor at its start. Off, long lines scroll sideways | Off |
 | **Keywords** (in Format · Cmd/Ctrl+Shift+F; Omarchy: format · `=` in normal mode) | UPPERCASE, lowercase or Keep: the case Format writes reserved words in. Keep leaves them as you typed them. Names, strings and comments never change | UPPERCASE on macOS and Windows, lowercase in the Omarchy look |
 | **Commas** | Trailing (at the end of an item's line) or Leading (at the start of the next item's, on the edge of the river), in the lists Format breaks: a `SELECT`'s and a `WITH`'s. A comma with a comment after it, on its line or the next, stays at the end of its line | Trailing |
 | **Keep query history** (Omarchy: keep) | The last 100, 1,000 or 10,000 statements of each connection, on this computer, or Don't keep: every statement run from the SQL editor, a grid's save, a filter or sort you set, and agents' runs, as the [History tab]({% link _guide/sql-editor.md %}#history) shows them. Results are never kept. Don't keep records nothing more, and asks whether to delete what is kept (Omarchy: `D` deletes, `n` keeps). **Show History** beside it (Omarchy: Enter) shows the window's History tab | Last 1,000 |
 
-The other keys of the four `[editor.…]` tables are read and kept now, and
-take effect in a later version.
-
 On macOS and Windows the Editor pane's **Clear history…** deletes the
 history of every connection, once you say so; a History tab's own clears
 its connection's alone.
```

- [ ] **Step 2: Build and check the site**

Run: `cd docs && bin/check` where Ruby and the bundle are installed; otherwise leave it to `docs.yml` on the pull request.
Expected: the build and its checks pass.

- [ ] **Step 3: Commit**

```bash
git add docs/_reference/settings-and-files.md
git commit -m "Describe Settings › Editor's preview on the website"
```

---

### Task 3: The pull request

- [ ] Push the branch and open one pull request: a summary, the preview in both looks, the departures above. No keys added. No screenshot.
