//! Settings › Editor's Close brackets and quotes and Wrap, in both looks:
//! what a bracket or a quote typed puts in, what Backspace takes out, how
//! long lines lay out with their numbers, and the rows that set them.

use egui::{Key, Modifiers};

use crate::model::{Action, ConnTabId};
use crate::settings::OptionValue;
use crate::testing::Harness;
use crate::theme::Look;

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
