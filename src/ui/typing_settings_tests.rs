//! Settings › Editor's Close brackets and quotes and Wrap, in both looks:
//! what a bracket or a quote typed puts in, what Backspace takes out, how
//! long lines lay out with their numbers, and the rows that set them.

use egui::accesskit::Role;
use egui::{Key, Modifiers};

use crate::model::{Action, ConnTabId, SettingsPane};
use crate::settings::OptionValue;
use crate::testing::{Harness, bounds};
use crate::theme::Look;

use super::appearance_settings_tests::{settings_saves, shows};

/// A connected fixture in `look`, with a new SQL editor that has the
/// keyboard and holds `text`, its cursor at the end.
fn editor(look: Look, text: &str) -> (Harness, ConnTabId) {
    let mut harness = Harness::new();
    harness.set_look(look);
    let tab = harness.connect_fake();
    harness.press(Key::T, Modifiers::COMMAND);
    if !text.is_empty() {
        harness.frame(vec![egui::Event::Paste(text.into())]);
    }
    harness.settle();
    (harness, tab)
}

fn text(harness: &Harness, tab: ConnTabId) -> String {
    let workspace = harness.app.workspace(tab).unwrap();
    workspace.active_sql_tab().unwrap().text.clone()
}

fn cursor(harness: &Harness, tab: ConnTabId) -> usize {
    let workspace = harness.app.workspace(tab).unwrap();
    workspace.active_sql_tab().unwrap().cursor
}

/// Types `typed` one frame for each character, as text alone (the window
/// sends its key's press too: `a_bracket_typed_as_the_window_sends_it_is_paired`).
fn type_chars(harness: &mut Harness, typed: &str) {
    for c in typed.chars() {
        harness.frame(vec![egui::Event::Text(c.to_string())]);
        harness.settle();
    }
}

fn set(harness: &mut Harness, value: OptionValue) {
    harness.app.apply(Action::SetOption(value));
    harness.settle();
}

#[test]
fn a_bracket_gets_its_closing_one_and_a_closing_one_typed_steps_over() {
    for (open, close) in [('(', ')'), ('[', ']'), ('{', '}')] {
        let (mut harness, tab) = editor(Look::macos(), "SELECT count");
        type_chars(&mut harness, &open.to_string());
        assert_eq!(text(&harness, tab), format!("SELECT count{open}{close}"));
        assert_eq!(cursor(&harness, tab), 13, "between the two");
        type_chars(&mut harness, &format!("*{close}"));
        assert_eq!(text(&harness, tab), format!("SELECT count{open}*{close}"));
        assert_eq!(cursor(&harness, tab), 15, "past the closing one");
    }
}

#[test]
fn a_bracket_typed_as_the_window_sends_it_is_paired() {
    // The window sends the press of the key with the character it types.
    let (mut harness, tab) = editor(Look::macos(), "SELECT count");
    harness.frame(vec![
        crate::testing::key(Key::Num9, Modifiers::SHIFT),
        egui::Event::Text("(".into()),
        crate::testing::release(Key::Num9, Modifiers::SHIFT),
    ]);
    harness.settle();
    assert_eq!(text(&harness, tab), "SELECT count()");
    // Two characters in one frame are typed as they come, unpaired.
    harness.frame(vec![
        egui::Event::Text("*".into()),
        egui::Event::Text(")".into()),
    ]);
    harness.settle();
    assert_eq!(text(&harness, tab), "SELECT count(*))");
}

#[test]
fn a_bracket_typed_with_altgr_is_paired() {
    // Windows sends AltGr as Ctrl+Alt: on a German keyboard, `[` is
    // AltGr+8.
    let (mut harness, tab) = editor(Look::macos(), "SELECT ARRAY");
    let altgr = Modifiers::CTRL | Modifiers::ALT;
    harness.frame(vec![
        crate::testing::key(Key::Num8, altgr),
        egui::Event::Text("[".into()),
        crate::testing::release(Key::Num8, altgr),
    ]);
    harness.settle();
    assert_eq!(text(&harness, tab), "SELECT ARRAY[]");
    assert_eq!(cursor(&harness, tab), 13);
}

#[test]
fn a_quote_is_paired_where_a_string_starts_and_nowhere_else() {
    let (mut harness, tab) = editor(Look::macos(), "SELECT ");
    type_chars(&mut harness, "'ebook'");
    assert_eq!(text(&harness, tab), "SELECT 'ebook'");
    assert_eq!(cursor(&harness, tab), 14);
    // Inside a word, after a quote (an escape), and inside a string or a
    // comment, the quote is typed alone.
    for (before, typed, after) in [
        ("SELECT it", "'", "SELECT it'"),
        ("SELECT 'it", "'", "SELECT 'it'"),
        ("SELECT 'a ", "'", "SELECT 'a '"),
        ("-- note ", "'", "-- note '"),
        ("SELECT ''", "'", "SELECT '''"),
    ] {
        let (mut harness, tab) = editor(Look::macos(), before);
        type_chars(&mut harness, typed);
        assert_eq!(text(&harness, tab), after, "{before:?}");
    }
    // Not before a word: the bracket would split it off.
    let (mut harness, tab) = editor(Look::macos(), "SELECT title");
    for _ in 0.."title".len() {
        harness.press(Key::ArrowLeft, Modifiers::NONE);
    }
    type_chars(&mut harness, "(");
    assert_eq!(text(&harness, tab), "SELECT (title");
}

#[test]
fn backspace_between_an_empty_pair_takes_both_out() {
    let (mut harness, tab) = editor(Look::macos(), "SELECT count");
    type_chars(&mut harness, "(");
    harness.press(Key::Backspace, Modifiers::NONE);
    harness.settle();
    assert_eq!(text(&harness, tab), "SELECT count");
    // Not a pair with something between.
    type_chars(&mut harness, "(*");
    harness.press(Key::Backspace, Modifiers::NONE);
    harness.press(Key::ArrowRight, Modifiers::NONE);
    harness.press(Key::Backspace, Modifiers::NONE);
    harness.settle();
    assert_eq!(text(&harness, tab), "SELECT count(");
}

#[test]
fn backspace_inside_an_escaped_quote_takes_one_out() {
    let (mut harness, tab) = editor(Look::macos(), "SELECT 'it''s'");
    put_cursor(&mut harness, tab, 11);
    harness.press(Key::Backspace, Modifiers::NONE);
    harness.settle();
    assert_eq!(text(&harness, tab), "SELECT 'it's'");
}

#[test]
fn an_input_method_s_text_in_the_frame_leaves_a_bracket_unpaired() {
    let (mut harness, tab) = editor(Look::macos(), "SELECT ");
    harness.frame(vec![
        egui::Event::Ime(egui::ImeEvent::Commit("x".into())),
        egui::Event::Text("(".into()),
    ]);
    harness.settle();
    assert_eq!(text(&harness, tab), "SELECT x(");
}

/// Puts the editor's cursor at the character `at`, as a click there does.
fn put_cursor(harness: &mut Harness, tab: ConnTabId, at: usize) {
    use egui::text::{CCursor, CCursorRange};
    let workspace = harness.app.workspace(tab).unwrap();
    let id = workspace.active_sql_tab().unwrap().id;
    let editor = crate::ui::sql_text::editor_id(tab, id);
    let mut state = egui::TextEdit::load_state(&harness.ctx, editor).expect("an editor");
    state
        .cursor
        .set_char_range(Some(CCursorRange::one(CCursor::new(at))));
    egui::TextEdit::store_state(&harness.ctx, editor, state);
    harness.settle();
}

#[test]
fn a_bracket_over_a_selection_puts_it_between_and_keeps_it_selected() {
    let (mut harness, tab) = editor(Look::macos(), "SELECT title");
    for _ in 0.."title".len() {
        harness.press(Key::ArrowLeft, Modifiers::SHIFT);
    }
    type_chars(&mut harness, "(");
    assert_eq!(text(&harness, tab), "SELECT (title)");
    assert_eq!(cursor(&harness, tab), 8, "the moving end where it was");
    // Still selected: a quote goes round it again.
    type_chars(&mut harness, "\"");
    assert_eq!(text(&harness, tab), "SELECT (\"title\")");
    // One undo each.
    harness.press(Key::Z, Modifiers::COMMAND);
    harness.settle();
    assert_eq!(text(&harness, tab), "SELECT (title)");
    harness.press(Key::Z, Modifiers::COMMAND);
    harness.settle();
    assert_eq!(text(&harness, tab), "SELECT title");
}

#[test]
fn a_selection_made_forwards_keeps_its_moving_end_at_the_end() {
    let (mut harness, tab) = editor(Look::macos(), "SELECT title");
    for _ in 0.."title".len() {
        harness.press(Key::ArrowLeft, Modifiers::NONE);
    }
    for _ in 0.."title".len() {
        harness.press(Key::ArrowRight, Modifiers::SHIFT);
    }
    type_chars(&mut harness, "[");
    assert_eq!(text(&harness, tab), "SELECT [title]");
    assert_eq!(cursor(&harness, tab), 13);
}

#[test]
fn with_close_pairs_off_a_bracket_is_typed_alone() {
    let (mut harness, tab) = editor(Look::macos(), "SELECT count");
    set(&mut harness, OptionValue::ClosePairs(false));
    type_chars(&mut harness, "('");
    assert_eq!(text(&harness, tab), "SELECT count('");
}

#[test]
fn the_terminal_look_pairs_in_insert_mode_and_not_out_of_the_text() {
    let (mut harness, tab) = editor(Look::omarchy(), "select count");
    type_chars(&mut harness, "(");
    assert_eq!(text(&harness, tab), "select count()");
    assert_eq!(cursor(&harness, tab), 13);
    type_chars(&mut harness, "*)");
    assert_eq!(text(&harness, tab), "select count(*)");
    // Normal mode: the keyboard out of the text, which types nothing.
    harness.press(Key::Escape, Modifiers::NONE);
    type_chars(&mut harness, "(");
    assert_eq!(text(&harness, tab), "select count(*)");
}

#[test]
fn a_paste_is_put_in_as_it_is() {
    let (mut harness, tab) = editor(Look::macos(), "SELECT count");
    harness.frame(vec![egui::Event::Paste("(".into())]);
    harness.settle();
    assert_eq!(text(&harness, tab), "SELECT count(");
}

/// The tops of the gutter's numbers, in order, with each number.
fn numbers(harness: &mut Harness) -> Vec<(String, f32)> {
    let tree = harness.settle();
    let field = bounds(&tree, "SQL", Role::MultilineTextInput).expect("the editor");
    let mut found: Vec<(String, f32)> = harness
        .text_rects
        .iter()
        .filter(|(text, rect)| {
            rect.right() <= field.left() && text.bytes().all(|byte| byte.is_ascii_digit())
        })
        .map(|(text, rect)| (text.clone(), rect.top()))
        .collect();
    found.sort_by(|a, b| a.1.total_cmp(&b.1));
    found
}

#[test]
fn wrap_lays_a_long_line_out_in_rows_numbered_once() {
    let long = format!("SELECT {}\nFROM book_covers", "title, ".repeat(60));
    let (mut harness, _) = editor(Look::macos(), &long);
    // Off: one row a line, the second right under the first.
    let off = numbers(&mut harness);
    assert_eq!(
        off.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>(),
        ["1", "2"]
    );
    let row = off[1].1 - off[0].1;
    set(&mut harness, OptionValue::Wrap(true));
    let on = numbers(&mut harness);
    assert_eq!(
        on.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>(),
        ["1", "2"]
    );
    assert!(
        on[1].1 - on[0].1 > 2.5 * row,
        "the first line wraps to rows"
    );
    // The editor keeps the keyboard.
    assert!(harness.ctx.text_edit_focused());
}

#[test]
fn a_wrapped_line_is_one_line_to_the_bar_and_the_cursor() {
    let long = format!("SELECT {}", "title, ".repeat(60));
    let (mut harness, _) = editor(Look::macos(), &long);
    set(&mut harness, OptionValue::Wrap(true));
    harness.settle();
    let palette = harness.app.palette;
    let line = palette.window.lerp_to_gamma(palette.selection, 0.45);
    let bar = crate::ui::sql_text::bar_color(&harness.app.look, &palette);
    let tallest = |color| {
        harness
            .fills
            .iter()
            .filter(|(_, fill)| *fill == color)
            .map(|(rect, _)| rect.height())
            .fold(0.0, f32::max)
    };
    // Both stand beside every row the line wraps to: far taller than one.
    let rows = harness
        .text_rects
        .iter()
        .find(|(text, _)| text == "1")
        .map(|(_, rect)| rect.height())
        .expect("the number");
    assert!(tallest(line) > 2.5 * rows, "the cursor's line");
    assert!(tallest(bar) > 2.5 * rows, "the statement's bar");
}

#[test]
fn a_press_beside_a_wrapped_row_puts_the_cursor_at_its_line_s_start() {
    let long = format!("SELECT {}", "title, ".repeat(60));
    let (mut harness, tab) = editor(Look::macos(), &long);
    set(&mut harness, OptionValue::Wrap(true));
    let tree = harness.settle();
    let gutter = bounds(&tree, "Line numbers", Role::Unknown)
        .or_else(|| bounds(&tree, "Line numbers", Role::GenericContainer))
        .expect("the gutter");
    let number = numbers(&mut harness)[0].1;
    // Two rows down the first line, beside a row with no number.
    let at = egui::pos2(gutter.center().x, number + 50.0);
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
    assert_eq!(cursor(&harness, tab), 0);
}

#[test]
fn the_completion_list_hangs_under_a_word_that_starts_a_wrapped_row() {
    // Filler of one length or another puts the word typed at the start of
    // a wrapped row: the list hangs under the word's own row each time.
    let mut started = 0;
    for words in 40..80 {
        let (mut harness, _) = editor(Look::macos(), &"a ".repeat(words));
        set(&mut harness, OptionValue::Wrap(true));
        type_chars(&mut harness, "se");
        let tree = harness.settle();
        let Some(list) = bounds(&tree, "Completions", Role::Unknown)
            .or_else(|| bounds(&tree, "Completions", Role::GenericContainer))
            .or_else(|| bounds(&tree, "Completions", Role::ListBox))
        else {
            continue;
        };
        // The row the word is on: the last of the editor's rows, each a
        // text run to a screen reader.
        let word = tree
            .nodes
            .iter()
            .filter(|(_, node)| node.role() == Role::TextRun)
            .filter_map(|(_, node)| node.bounds())
            .map(|rect| {
                egui::Rect::from_min_max(
                    egui::pos2(rect.x0 as f32, rect.y0 as f32),
                    egui::pos2(rect.x1 as f32, rect.y1 as f32),
                )
            })
            .max_by(|a, b| a.top().total_cmp(&b.top()))
            .expect("the word's row");
        assert!(
            list.top() >= word.bottom() - 1.0,
            "{words}: {list:?} {word:?}"
        );
        // The word alone on its row: it starts the row.
        let field = bounds(&tree, "SQL", Role::MultilineTextInput).expect("the editor");
        if word.left() < field.left() + 40.0 && word.width() < 40.0 {
            started += 1;
        }
    }
    assert!(started > 0, "no length put the word at a row's start");
}

/// Settings on its Editor pane.
fn on_the_pane(harness: &mut Harness) {
    harness.app.apply(Action::ShowSettings);
    harness
        .app
        .apply(Action::ShowSettingsPane(SettingsPane::Editor));
    harness.settle();
}

#[test]
fn the_sheet_has_the_close_pairs_and_wrap_rows_and_sets_them() {
    let mut harness = Harness::with_size(egui::vec2(1440.0, 1400.0));
    harness.set_look(Look::macos());
    on_the_pane(&mut harness);
    // As the board orders them: after Autocomplete, and Wrap last.
    let top = |harness: &Harness, words: &str| harness.painted_rect(words).expect(words).top();
    harness.settle();
    assert!(top(&harness, "Autocomplete") < top(&harness, "Close brackets and quotes"));
    assert!(top(&harness, "Close brackets and quotes") < top(&harness, "Line numbers"));
    assert!(top(&harness, "Current line highlight") < top(&harness, "Wrap"));
    let saves = settings_saves(&harness);
    harness.click("Close brackets and quotes");
    harness.click("Wrap");
    harness.settle();
    assert!(!harness.app.settings.close_pairs);
    assert!(harness.app.settings.wrap);
    assert_eq!(settings_saves(&harness), saves + 2);
}

#[test]
fn the_screen_has_the_close_brackets_and_wrap_rows() {
    let mut harness = Harness::with_size(egui::vec2(1920.0, 1080.0));
    harness.set_look(Look::omarchy());
    on_the_pane(&mut harness);
    for words in ["close brackets", "wrap"] {
        assert!(shows(&mut harness, words), "{words}");
    }
    let top = |harness: &Harness, words: &str| harness.painted_rect(words).expect(words).top();
    assert!(top(&harness, "autocomplete") < top(&harness, "close brackets"));
    assert!(top(&harness, "close brackets") < top(&harness, "wrap"));
    // On and off until set, as each row's box says.
    let mark = |harness: &Harness, words: &str| {
        let row = harness.painted_rect(words).expect(words);
        harness
            .text_rects
            .iter()
            .find(|(text, rect)| {
                (text == "[x]" || text == "[ ]") && (rect.center().y - row.center().y).abs() < 4.0
            })
            .map(|(text, _)| text.clone())
            .expect("the row's box")
    };
    assert_eq!(mark(&harness, "close brackets"), "[x]");
    assert_eq!(mark(&harness, "wrap"), "[ ]");
}
