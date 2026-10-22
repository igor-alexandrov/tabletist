//! Settings › Editor's Format section and indent, in both looks: what
//! Format writes, what Tab and Shift+Tab put in and take out, the case
//! completion offers keywords in, and the rows that set them.

use egui::{Key, Modifiers};

use crate::keymap::{Command as Keyed, Scope};
use crate::model::{Action, ConnTabId, SettingsPane};
use crate::settings::{Commas, Indent, KeywordCase, OptionValue};
use crate::testing::Harness;
use crate::theme::Look;

use super::appearance_settings_tests::{settings_saves, shows};

const COMMAND_SHIFT: Modifiers = Modifiers::COMMAND.plus(Modifiers::SHIFT);

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

/// Formats with the chord of the macOS and Windows looks.
fn format(harness: &mut Harness) {
    harness.press(Key::F, COMMAND_SHIFT);
    harness.settle();
}

fn set(harness: &mut Harness, value: OptionValue) {
    harness.app.apply(Action::SetOption(value));
    harness.settle();
}

#[test]
fn format_writes_the_case_the_commas_and_the_indent_set() {
    let script = "select p.name as publisher, case when c.kind = 'ebook' then 1 else 0 end \
        as ebook from book_covers c join publishers p on p.id = c.publisher_id";
    // The defaults of macOS and Windows: upper case, trailing commas, a
    // CASE's parts two in.
    let (mut harness, tab) = editor(Look::macos(), script);
    format(&mut harness);
    assert_eq!(
        text(&harness, tab),
        [
            "SELECT p.name AS publisher,",
            "       CASE",
            "         WHEN c.kind = 'ebook' THEN 1",
            "         ELSE 0",
            "       END AS ebook",
            "  FROM book_covers c",
            "  JOIN publishers p ON p.id = c.publisher_id",
        ]
        .join("\n")
    );
    // Lower case, leading commas, four in.
    let (mut harness, tab) = editor(Look::macos(), script);
    set(
        &mut harness,
        OptionValue::Keywords(Some(KeywordCase::Lower)),
    );
    set(&mut harness, OptionValue::Commas(Commas::Leading));
    set(&mut harness, OptionValue::Indent(Indent::Four));
    format(&mut harness);
    assert_eq!(
        text(&harness, tab),
        [
            "select p.name as publisher",
            "     , case",
            "           when c.kind = 'ebook' then 1",
            "           else 0",
            "       end as ebook",
            "  from book_covers c",
            "  join publishers p on p.id = c.publisher_id",
        ]
        .join("\n")
    );
    // One undo step gives the script back as typed.
    harness.press(Key::Z, Modifiers::COMMAND);
    harness.settle();
    assert_eq!(text(&harness, tab), script);
}

#[test]
fn keep_leaves_the_case_as_typed_and_names_strings_and_comments_alone() {
    let script = "Select Name, 'Select me' -- Select it\nFrom Books where Kind = 'From'";
    let (mut harness, tab) = editor(Look::macos(), script);
    set(&mut harness, OptionValue::Keywords(Some(KeywordCase::Keep)));
    format(&mut harness);
    assert_eq!(
        text(&harness, tab),
        [
            "Select Name,",
            "       'Select me' -- Select it",
            "  From Books",
            " where Kind = 'From'",
        ]
        .join("\n")
    );
}

#[test]
fn the_terminal_look_formats_in_lower_case_until_set() {
    let (mut harness, tab) = editor(Look::omarchy(), "SELECT 1 FROM t");
    harness.leave_editor();
    super::tests::type_key(&mut harness, Key::Equals, "=");
    harness.settle();
    assert_eq!(text(&harness, tab), "select 1\n  from t");
}

#[test]
fn tab_puts_in_the_indent_set_and_shift_tab_takes_one_out() {
    for (indent, unit) in [
        (Indent::Two, "  "),
        (Indent::Four, "    "),
        (Indent::Tab, "\t"),
    ] {
        let (mut harness, tab) = editor(Look::macos(), "");
        set(&mut harness, OptionValue::Indent(indent));
        harness.press(Key::Tab, Modifiers::NONE);
        harness.settle();
        harness.frame(vec![egui::Event::Text("SELECT 1".into())]);
        harness.settle();
        assert_eq!(text(&harness, tab), format!("{unit}SELECT 1"), "{indent:?}");
        harness.press(Key::Tab, Modifiers::SHIFT);
        harness.settle();
        assert_eq!(text(&harness, tab), "SELECT 1", "{indent:?}");
        // Shift+Tab is a step of its own. (A Tab at the cursor is typed
        // text, and undoes with what is typed beside it.)
        harness.press(Key::Z, Modifiers::COMMAND);
        harness.settle();
        assert_eq!(text(&harness, tab), format!("{unit}SELECT 1"), "{indent:?}");
    }
}

#[test]
fn tab_over_a_selection_indents_its_lines_and_keeps_them_selected() {
    let (mut harness, tab) = editor(Look::macos(), "SELECT 1\nFROM t\n\nWHERE x");
    // From the end up over the last three lines.
    for _ in 0.."FROM t\n\nWHERE x".chars().count() {
        harness.press(Key::ArrowLeft, Modifiers::SHIFT);
    }
    harness.press(Key::Tab, Modifiers::NONE);
    harness.settle();
    // The blank line stays blank.
    assert_eq!(text(&harness, tab), "SELECT 1\n  FROM t\n\n  WHERE x");
    // One undo takes the lines back, and redo puts them in again.
    harness.press(Key::Z, Modifiers::COMMAND);
    harness.settle();
    assert_eq!(text(&harness, tab), "SELECT 1\nFROM t\n\nWHERE x");
    harness.press(Key::Z, COMMAND_SHIFT);
    harness.settle();
    assert_eq!(text(&harness, tab), "SELECT 1\n  FROM t\n\n  WHERE x");
    // Still selected: a second Tab indents them again, Shift+Tab out.
    harness.press(Key::Tab, Modifiers::NONE);
    harness.settle();
    assert_eq!(text(&harness, tab), "SELECT 1\n    FROM t\n\n    WHERE x");
    harness.press(Key::Tab, Modifiers::SHIFT);
    harness.press(Key::Tab, Modifiers::SHIFT);
    harness.settle();
    assert_eq!(text(&harness, tab), "SELECT 1\nFROM t\n\nWHERE x");
}

/// Puts the editor's selection from `anchor` to `head` (characters), as a
/// drag from one to the other does.
fn select(harness: &mut Harness, tab: ConnTabId, anchor: usize, head: usize) {
    use egui::text::{CCursor, CCursorRange};
    let workspace = harness.app.workspace(tab).unwrap();
    let id = workspace.active_sql_tab().unwrap().id;
    let editor = crate::ui::sql_text::editor_id(tab, id);
    let mut state = egui::TextEdit::load_state(&harness.ctx, editor).expect("an editor");
    // `two` puts the moving end at its second.
    let range = CCursorRange::two(CCursor::new(anchor), CCursor::new(head));
    state.cursor.set_char_range(Some(range));
    egui::TextEdit::store_state(&harness.ctx, editor, state);
    harness.settle();
}

/// The editor's selection: where it was started, and its moving end.
fn selection(harness: &Harness, tab: ConnTabId) -> Option<(usize, usize)> {
    let workspace = harness.app.workspace(tab).unwrap();
    let id = workspace.active_sql_tab().unwrap().id;
    let editor = crate::ui::sql_text::editor_id(tab, id);
    let range = egui::TextEdit::load_state(&harness.ctx, editor)?
        .cursor
        .char_range()?;
    Some((range.secondary.index.0, range.primary.index.0))
}

#[test]
fn a_selection_made_backwards_keeps_its_moving_end() {
    let (mut harness, tab) = editor(Look::macos(), "SELECT 1\nFROM t");
    select(&mut harness, tab, 15, 9);
    harness.press(Key::Tab, Modifiers::NONE);
    harness.settle();
    assert_eq!(text(&harness, tab), "SELECT 1\n  FROM t");
    assert_eq!(selection(&harness, tab), Some((17, 9)));
    harness.press(Key::Tab, Modifiers::SHIFT);
    harness.settle();
    assert_eq!(selection(&harness, tab), Some((15, 9)));
    // Forwards stays forwards.
    select(&mut harness, tab, 9, 15);
    harness.press(Key::Tab, Modifiers::NONE);
    harness.settle();
    assert_eq!(selection(&harness, tab), Some((9, 17)));
}

#[test]
fn a_tab_and_text_typed_in_one_frame_keep_their_order() {
    for (events, want) in [(vec![true, false], "  a"), (vec![false, true], "a  ")] {
        let (mut harness, tab) = editor(Look::macos(), "");
        let frame = events
            .into_iter()
            .flat_map(|tab_key| {
                if tab_key {
                    vec![
                        crate::testing::key(Key::Tab, Modifiers::NONE),
                        crate::testing::release(Key::Tab, Modifiers::NONE),
                    ]
                } else {
                    vec![egui::Event::Text("a".into())]
                }
            })
            .collect();
        harness.frame(frame);
        harness.settle();
        assert_eq!(text(&harness, tab), want);
    }
}

#[test]
fn tab_is_not_the_editor_s_without_the_keyboard() {
    for look in [Look::macos(), Look::omarchy()] {
        let (mut harness, tab) = editor(look, "SELECT 1");
        // Esc gives the keyboard up: Omarchy's normal mode, and the other
        // looks' editor left.
        harness.leave_editor();
        harness.press(Key::Tab, Modifiers::NONE);
        harness.press(Key::Tab, Modifiers::SHIFT);
        harness.settle();
        assert_eq!(text(&harness, tab), "SELECT 1", "{}", look.name);
    }
}

#[test]
fn shift_tab_under_an_open_list_leaves_no_list_of_the_old_text() {
    let (mut harness, tab) = editor(Look::macos(), "  ");
    harness.frame(vec![egui::Event::Text("sel".into())]);
    harness.settle();
    let list = |harness: &Harness| {
        let workspace = harness.app.workspace(tab).unwrap();
        let sql = workspace.active_sql_tab().unwrap();
        sql.completion
            .as_ref()
            .map(|list| list.is_of_text(&sql.text))
    };
    assert_eq!(list(&harness), Some(true));
    harness.press(Key::Tab, Modifiers::SHIFT);
    harness.settle();
    assert_eq!(text(&harness, tab), "sel");
    assert_ne!(list(&harness), Some(false), "a list of the text as it was");
}

#[test]
fn tab_still_accepts_an_open_completion() {
    let (mut harness, tab) = editor(Look::macos(), "");
    harness.frame(vec![egui::Event::Text("sel".into())]);
    harness.settle();
    harness.press(Key::Tab, Modifiers::NONE);
    harness.settle();
    assert_eq!(text(&harness, tab), "SELECT");
}

#[test]
fn completion_offers_keywords_in_the_case_set() {
    // Typed in the other case than the one offered: the case typed is not
    // what decides.
    for (case, typed, want) in [
        (None, "sel", "SELECT"),
        (Some(KeywordCase::Lower), "SEL", "select"),
        (Some(KeywordCase::Upper), "sel", "SELECT"),
        // Keep has no case of its own for a word not typed yet.
        (Some(KeywordCase::Keep), "sel", "SELECT"),
    ] {
        let (mut harness, tab) = editor(Look::macos(), "");
        set(&mut harness, OptionValue::Keywords(case));
        harness.frame(vec![egui::Event::Text(typed.into())]);
        harness.settle();
        harness.press(Key::Tab, Modifiers::NONE);
        harness.settle();
        assert_eq!(text(&harness, tab), want, "{case:?}");
    }
    // The terminal look's own is lower case.
    let (mut harness, tab) = editor(Look::omarchy(), "");
    harness.frame(vec![egui::Event::Text("SEL".into())]);
    harness.settle();
    harness.press(Key::Tab, Modifiers::NONE);
    harness.settle();
    assert_eq!(text(&harness, tab), "select");
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
fn the_sheet_has_the_indent_and_the_format_section_and_sets_them() {
    let look = Look::macos();
    let mut harness = Harness::with_size(egui::vec2(1440.0, 1400.0));
    harness.set_look(look);
    on_the_pane(&mut harness);
    // The heading names the Format key, as the keymap writes it.
    let key = crate::ui::keys::written_in(&harness.ctx, &look, Keyed::Format, Scope::SqlEditor);
    let heading = format!("FORMAT · {}", key.as_str()).to_uppercase();
    for words in [heading.as_str(), "Indent", "Keywords", "Commas"] {
        assert!(shows(&mut harness, words), "{words}");
    }
    // Indent leads the Typing section, as the board has it.
    let top = |harness: &Harness, words: &str| harness.painted_rect(words).expect(words).top();
    assert!(top(&harness, "TYPING") < top(&harness, "Indent"));
    assert!(top(&harness, "Indent") < top(&harness, "Autocomplete"));
    assert!(top(&harness, "Autocomplete") < top(&harness, "Keywords"));
    let saves = settings_saves(&harness);
    harness.click("Indent: Tab");
    harness.click("Keywords: lowercase");
    harness.click("Commas: Leading");
    harness.settle();
    let settings = &harness.app.settings;
    assert_eq!(settings.indent, Indent::Tab);
    assert_eq!(settings.keywords, Some(KeywordCase::Lower));
    assert_eq!(settings.commas, Commas::Leading);
    assert_eq!(settings_saves(&harness), saves + 3);
    // UPPERCASE is the look's own: picked while lower is set, it is set.
    harness.click("Keywords: UPPERCASE");
    assert_eq!(harness.app.settings.keywords, Some(KeywordCase::Upper));
}

#[test]
fn the_screen_has_the_indent_and_the_format_section() {
    let look = Look::omarchy();
    let mut harness = Harness::with_size(egui::vec2(1920.0, 1080.0));
    harness.set_look(look);
    on_the_pane(&mut harness);
    let key = crate::ui::keys::written_in(&harness.ctx, &look, Keyed::Format, Scope::SqlEditor);
    let heading = format!("format · {} in normal mode", key.as_str());
    for words in [heading.as_str(), "indent", "2 spaces", "keywords", "commas"] {
        assert!(shows(&mut harness, words), "{words}");
    }
    // As the board orders them: line numbers, indent, autocomplete.
    let top = |harness: &Harness, words: &str| harness.painted_rect(words).expect(words).top();
    assert!(top(&harness, "line numbers") < top(&harness, "indent"));
    assert!(top(&harness, "indent") < top(&harness, "autocomplete"));
    // The case is the choice: UPPER as it is, lower chosen.
    assert!(shows(&mut harness, "UPPER"));
    let tree = harness.settle();
    let chosen = tree.nodes.iter().any(|(_, node)| {
        node.label() == Some("Keywords: lower")
            && node.toggled() == Some(egui::accesskit::Toggled::True)
    });
    assert!(chosen, "the look's lower case is the one chosen");
}
