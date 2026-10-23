//! Settings › Editor's preview, in both looks: the SQL editor over the
//! boards' Bookshop query, read-only, showing each setting as it is set.

use egui::accesskit::Role;
use egui::{Key, Modifiers};

use crate::model::{Action, SettingsPane};
use crate::settings::{Commas, EditorKeys, Indent, KeywordCase, LineNumbers, OptionValue};
use crate::testing::{Harness, bounds};
use crate::theme::Look;

/// Settings on its Editor pane in `look`, scrolled to its end, where the
/// preview is.
fn pane(look: Look) -> Harness {
    let size = if look.terminal {
        egui::vec2(1920.0, 1080.0)
    } else {
        egui::vec2(1440.0, 1100.0)
    };
    let mut harness = Harness::with_size(size);
    harness.set_look(look);
    harness.app.apply(Action::ShowSettings);
    harness
        .app
        .apply(Action::ShowSettingsPane(SettingsPane::Editor));
    harness.settle();
    to_the_end(&mut harness);
    harness
}

/// The pane scrolled to its end, where the preview is.
fn to_the_end(harness: &mut Harness) {
    let at = egui::pos2(harness.size.x / 2.0, harness.size.y / 2.0);
    harness.frame(vec![
        egui::Event::PointerMoved(at),
        egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, -2000.0),
            modifiers: Modifiers::NONE,
            phase: egui::TouchPhase::Move,
        },
    ]);
    harness.settle();
}

fn set(harness: &mut Harness, value: OptionValue) {
    harness.app.apply(Action::SetOption(value));
    harness.settle();
}

/// The preview's node: its text, and whether it is read-only.
fn preview(harness: &mut Harness) -> (String, bool) {
    let tree = harness.settle();
    let (_, node) = tree
        .nodes
        .iter()
        .find(|(_, node)| {
            node.role() == Role::MultilineTextInput && node.label() == Some("Editor preview")
        })
        .expect("the preview");
    (
        node.value().unwrap_or_default().to_owned(),
        node.is_read_only(),
    )
}

/// Where the preview is on screen.
fn place(harness: &mut Harness) -> egui::Rect {
    let tree = harness.settle();
    bounds(&tree, "Editor preview", Role::MultilineTextInput).expect("the preview")
}

/// The numbers painted in the preview's gutter, top to bottom.
fn numbers(harness: &mut Harness) -> Vec<String> {
    let field = place(harness);
    let mut found: Vec<(f32, String)> = harness
        .text_rects
        .iter()
        .filter(|(text, rect)| {
            text.parse::<usize>().is_ok()
                && rect.right() <= field.left()
                && field.y_range().contains(rect.center().y)
        })
        .map(|(text, rect)| (rect.top(), text.clone()))
        .collect();
    found.sort_by(|(a, _), (b, _)| a.total_cmp(b));
    found.into_iter().map(|(_, text)| text).collect()
}

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

#[test]
fn the_preview_follows_each_setting_as_it_is_set() {
    let mut harness = pane(Look::macos());
    // Keywords and commas: Format's.
    set(
        &mut harness,
        OptionValue::Keywords(Some(KeywordCase::Lower)),
    );
    set(&mut harness, OptionValue::Commas(Commas::Leading));
    let (text, _) = preview(&mut harness);
    assert!(text.starts_with("select p.name as publisher\n"), "{text}");
    assert!(text.contains(", count(*) as covers"), "{text}");
    // The numbers: relative, then none.
    set(
        &mut harness,
        OptionValue::LineNumbers(Some(LineNumbers::Relative)),
    );
    assert_eq!(
        numbers(&mut harness),
        ["2", "1", "3", "1", "2", "3", "4", "5", "6"]
    );
    set(
        &mut harness,
        OptionValue::LineNumbers(Some(LineNumbers::Off)),
    );
    assert!(numbers(&mut harness).is_empty());
    // The cursor's line, lit without the keyboard, until it is turned off.
    let palette = harness.app.palette;
    let lit = palette.window.lerp_to_gamma(palette.selection, 0.45);
    let field = place(&mut harness);
    let lines = |harness: &Harness| {
        harness
            .fills
            .iter()
            .filter(|(rect, fill)| *fill == lit && field.y_range().contains(rect.center().y))
            .count()
    };
    assert_eq!(lines(&harness), 1);
    set(&mut harness, OptionValue::HighlightLine(false));
    assert_eq!(lines(&harness), 0);
    // Vim's cursor, a block, without the keyboard too.
    let block = palette.accent.gamma_multiply(0.5);
    let blocks = |harness: &Harness| {
        harness
            .fills
            .iter()
            .filter(|(_, fill)| *fill == block)
            .count()
    };
    assert_eq!(blocks(&harness), 0);
    set(&mut harness, OptionValue::EditorKeys(Some(EditorKeys::Vim)));
    assert_eq!(blocks(&harness), 1);
}

#[test]
fn wrap_lays_the_preview_s_long_line_out_in_rows_numbered_once() {
    let mut harness = pane(Look::omarchy());
    harness.size = egui::vec2(640.0, 1080.0);
    harness.settle();
    let unwrapped = place(&mut harness).height();
    set(&mut harness, OptionValue::Wrap(true));
    // Rows past the nine lines: the long one wraps.
    let wrapped = place(&mut harness).height();
    assert!(wrapped > unwrapped + 10.0, "{unwrapped} {wrapped}");
    set(
        &mut harness,
        OptionValue::LineNumbers(Some(LineNumbers::Absolute)),
    );
    // As many numbers as lines, whatever the rows.
    assert_eq!(
        numbers(&mut harness),
        ["1", "2", "3", "4", "5", "6", "7", "8", "9"]
    );
}

#[test]
fn nothing_typed_changes_the_preview_and_tab_moves_on() {
    for look in [Look::macos(), Look::omarchy()] {
        let mut harness = pane(look);
        let (before, _) = preview(&mut harness);
        let field = place(&mut harness);
        let at = field.center();
        harness.frame(vec![
            egui::Event::PointerMoved(at),
            egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: Modifiers::NONE,
            },
            egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: Modifiers::NONE,
            },
        ]);
        harness.settle();
        assert!(focused(&harness), "{}", look.name);
        // Typing, a paste, and vim's commands: the query stays.
        for c in "xdd".chars() {
            harness.frame(vec![egui::Event::Text(c.to_string())]);
            harness.settle();
        }
        harness.frame(vec![egui::Event::Paste("DROP TABLE books;".into())]);
        harness.press(Key::Backspace, Modifiers::NONE);
        harness.press(Key::Enter, Modifiers::NONE);
        assert_eq!(preview(&mut harness).0, before, "{}", look.name);
        // One stop: Tab takes the keyboard on.
        harness.press(Key::Tab, Modifiers::NONE);
        let editor = crate::ui::settings::editor_preview_field();
        assert!(
            !harness.ctx.memory(|memory| memory.has_focus(editor)),
            "{}",
            look.name
        );
        // Nothing of it reached the backend.
        assert!(
            !harness
                .app
                .backend
                .sent
                .iter()
                .any(|command| matches!(command, crate::backend::Command::RunSql { .. })),
            "{}",
            look.name
        );
    }
}

/// The preview's editor, as the window keeps it.
fn state<T>(harness: &Harness, read: impl FnOnce(&crate::model::SqlTab) -> T) -> T {
    let id = crate::ui::settings::editor_preview_field().with("tab");
    let shared = harness
        .ctx
        .data(|data| data.get_temp::<std::sync::Arc<std::sync::Mutex<crate::model::SqlTab>>>(id))
        .expect("the preview's tab");
    let tab = shared.lock().unwrap();
    read(&tab)
}

fn click(harness: &mut Harness, at: egui::Pos2) {
    let press = |pressed| egui::Event::PointerButton {
        pos: at,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    harness.frame(vec![
        egui::Event::PointerMoved(at),
        press(true),
        press(false),
    ]);
    harness.settle();
}

/// A letter as a window sends it: its key, then its text.
fn letter(harness: &mut Harness, c: char) {
    let key = Key::from_name(&c.to_ascii_uppercase().to_string()).expect("a letter's key");
    let shift = if c.is_uppercase() {
        Modifiers::SHIFT
    } else {
        Modifiers::NONE
    };
    harness.frame(vec![
        crate::testing::key(key, shift),
        egui::Event::Text(c.to_string()),
    ]);
    harness.frame(vec![crate::testing::release(key, shift)]);
    harness.settle();
}

fn focused(harness: &Harness) -> bool {
    let id = crate::ui::settings::editor_preview_field();
    harness.ctx.memory(|memory| memory.has_focus(id))
}

#[test]
fn the_omarchy_screen_s_keys_are_the_preview_s_while_it_has_the_keyboard() {
    let mut harness = pane(Look::omarchy());
    let row = |harness: &Harness| match &harness.app.dialog {
        Some(crate::model::Dialog::Settings(dialog)) => dialog.row,
        _ => panic!("no Settings"),
    };
    let field = place(&mut harness);
    click(&mut harness, field.center());
    assert!(focused(&harness));
    let (at, cursor, before) = (
        row(&harness),
        state(&harness, |tab| tab.cursor),
        harness.app.settings.clone(),
    );
    // Vim's moves: the preview's cursor moves, the screen's stays, and no
    // setting changes.
    for c in "jlhlR".chars() {
        letter(&mut harness, c);
    }
    assert_eq!(row(&harness), at);
    assert_ne!(state(&harness, |tab| tab.cursor), cursor);
    assert_eq!(harness.app.settings, before);
    // `i` has nothing to type: no insert mode. Esc gives the keyboard up,
    // and the window stays; a second closes it.
    letter(&mut harness, 'i');
    assert_eq!(
        state(&harness, |tab| tab.vim.mode),
        crate::vim::Mode::Normal
    );
    harness.press(Key::Escape, Modifiers::NONE);
    assert!(harness.app.dialog.is_some());
    assert!(!focused(&harness));
    harness.press(Key::Escape, Modifiers::NONE);
    assert!(harness.app.dialog.is_none());
}

#[test]
fn the_preview_s_dropped_edits_leave_the_editors_register_be() {
    let mut harness = pane(Look::macos());
    set(&mut harness, OptionValue::EditorKeys(Some(EditorKeys::Vim)));
    let field = place(&mut harness);
    click(&mut harness, field.center());
    letter(&mut harness, 'd');
    letter(&mut harness, 'd');
    let register: crate::vim::Register = harness
        .ctx
        .data(|data| data.get_temp(egui::Id::new("vim register")))
        .unwrap_or_default();
    assert_eq!(register, crate::vim::Register::default());
    // And vim's cursor stays a block once the keyboard is gone.
    letter(&mut harness, 'i');
    harness.press(Key::Tab, Modifiers::NONE);
    assert!(!focused(&harness));
    let block = harness.app.palette.accent.gamma_multiply(0.5);
    let blocks = harness
        .fills
        .iter()
        .filter(|(_, fill)| *fill == block)
        .count();
    assert_eq!(blocks, 1);
}

#[test]
fn a_settings_change_puts_the_cursor_back_out_of_visual_mode() {
    let mut harness = pane(Look::macos());
    set(&mut harness, OptionValue::EditorKeys(Some(EditorKeys::Vim)));
    set(
        &mut harness,
        OptionValue::LineNumbers(Some(LineNumbers::Relative)),
    );
    let field = place(&mut harness);
    click(
        &mut harness,
        egui::pos2(field.center().x, field.top() + 12.0),
    );
    assert_eq!(
        numbers(&mut harness),
        ["1", "1", "2", "3", "4", "5", "6", "7", "8"]
    );
    letter(&mut harness, 'v');
    set(
        &mut harness,
        OptionValue::Keywords(Some(KeywordCase::Lower)),
    );
    assert_eq!(
        numbers(&mut harness),
        ["2", "1", "3", "1", "2", "3", "4", "5", "6"]
    );
    assert!(!state(&harness, |tab| tab.vim.mode.is_visual()));
}

#[test]
fn keep_shows_the_query_as_it_was_typed() {
    let mut harness = pane(Look::macos());
    let mut shown = Vec::new();
    for case in [KeywordCase::Upper, KeywordCase::Lower, KeywordCase::Keep] {
        set(&mut harness, OptionValue::Keywords(Some(case)));
        shown.push(preview(&mut harness).0);
    }
    assert_ne!(shown[2], shown[0]);
    assert_ne!(shown[2], shown[1]);
}

/// Clicks the preview, which then has the keyboard.
fn into_preview(harness: &mut Harness) {
    let field = place(harness);
    click(harness, field.center());
    assert!(focused(harness));
}

#[test]
fn esc_gives_the_preview_s_keyboard_up_before_it_closes_the_window_with_either_keys() {
    for look in [Look::macos(), Look::omarchy()] {
        for keys in [EditorKeys::Standard, EditorKeys::Vim] {
            let mut harness = pane(look);
            set(&mut harness, OptionValue::EditorKeys(Some(keys)));
            into_preview(&mut harness);
            harness.press(Key::Escape, Modifiers::NONE);
            assert!(harness.app.dialog.is_some(), "{} {keys:?}", look.name);
            assert!(!focused(&harness), "{} {keys:?}", look.name);
            harness.press(Key::Escape, Modifiers::NONE);
            assert!(harness.app.dialog.is_none(), "{} {keys:?}", look.name);
        }
    }
}

#[test]
fn the_omarchy_screen_s_chords_still_work_while_the_preview_has_the_keyboard() {
    let mut harness = pane(Look::omarchy());
    into_preview(&mut harness);
    harness.press(Key::L, Modifiers::CTRL);
    match &harness.app.dialog {
        Some(crate::model::Dialog::Settings(dialog)) => {
            assert_eq!(dialog.pane, SettingsPane::Keyboard);
        }
        _ => panic!("Settings closed"),
    }
}

#[test]
fn the_preview_starts_again_where_the_boards_have_it_when_the_window_opens_again() {
    let mut harness = pane(Look::omarchy());
    into_preview(&mut harness);
    letter(&mut harness, 'g');
    letter(&mut harness, 'g');
    assert_eq!(
        numbers(&mut harness),
        ["1", "1", "2", "3", "4", "5", "6", "7", "8"]
    );
    harness.app.apply(Action::CloseDialog);
    harness.settle();
    harness.app.apply(Action::ShowSettings);
    harness
        .app
        .apply(Action::ShowSettingsPane(SettingsPane::Editor));
    harness.settle();
    assert_eq!(
        numbers(&mut harness),
        ["2", "1", "3", "1", "2", "3", "4", "5", "6"]
    );
}

#[test]
fn a_run_key_in_the_preview_sends_nothing_with_a_connection_open() {
    for look in [Look::macos(), Look::omarchy()] {
        let mut harness = Harness::with_size(egui::vec2(1440.0, 1100.0));
        harness.set_look(look);
        harness.connect_fake();
        let sent = harness.app.backend.sent.len();
        harness.app.apply(Action::ShowSettings);
        harness
            .app
            .apply(Action::ShowSettingsPane(SettingsPane::Editor));
        harness.settle();
        to_the_end(&mut harness);
        into_preview(&mut harness);
        harness.press(Key::Enter, Modifiers::COMMAND);
        harness.press(Key::Enter, Modifiers::CTRL);
        harness.press(Key::Enter, Modifiers::COMMAND | Modifiers::SHIFT);
        assert_eq!(harness.app.backend.sent.len(), sent, "{}", look.name);
    }
}

#[test]
fn shift_tab_comes_back_to_the_preview_in_one_press() {
    let mut harness = pane(Look::macos());
    into_preview(&mut harness);
    harness.press(Key::Tab, Modifiers::NONE);
    assert!(!focused(&harness));
    harness.press(Key::Tab, Modifiers::SHIFT);
    assert!(focused(&harness), "the gutter is no stop of its own");
}

#[test]
fn the_card_is_the_preview_s_focus_ring_which_focus_paints() {
    for look in [Look::macos(), Look::omarchy()] {
        let mut harness = pane(look);
        into_preview(&mut harness);
        let field = place(&mut harness);
        let (rect, ring) = crate::ui::focus::hinted(&harness.ctx).expect("a ring asked for");
        let radius = if look.terminal { 3 } else { 10 };
        assert_eq!(
            ring,
            crate::ui::focus::Ring::Field { radius },
            "{}",
            look.name
        );
        // The card the field sits in: round it, from further left.
        assert!(rect.intersects(field), "{} {rect:?} {field:?}", look.name);
        assert!(
            rect.left() < field.left(),
            "{} {rect:?} {field:?}",
            look.name
        );
    }
}
