//! Headless tests of the SQL editor's completion list: what opens it and
//! what closes it. The tasks that give it keys, a view and names from the
//! database add their tests here.

use egui::{Key, Modifiers};

use crate::backend::Command;
use crate::completion::LISTED;
use crate::model::{Action, Completion, ConnTabId, SqlTab, TabId, TextPrint, Wanted};
use crate::testing::Harness;
use crate::ui::sql_text::TOKENIZED;

/// A connected fixture workspace showing an empty SQL editor that has the
/// keyboard.
fn editor() -> (Harness, ConnTabId) {
    let mut harness = Harness::new();
    let tab = harness.connect_fake();
    harness.press(Key::T, Modifiers::COMMAND);
    (harness, tab)
}

/// Types `text` at the cursor, as the keyboard does.
fn type_text(harness: &mut Harness, text: &str) {
    harness.frame(vec![egui::Event::Text(text.into())]);
    harness.settle();
}

/// Pastes `text` at the cursor: an edit that is not typing.
fn paste(harness: &mut Harness, text: &str) {
    harness.frame(vec![egui::Event::Paste(text.into())]);
    harness.settle();
}

fn sql(harness: &Harness, tab: ConnTabId) -> &SqlTab {
    harness
        .app
        .workspace(tab)
        .unwrap()
        .active_sql_tab()
        .unwrap()
}

fn ids(harness: &Harness, tab: ConnTabId) -> (ConnTabId, TabId) {
    (tab, sql(harness, tab).id)
}

fn list(harness: &Harness, tab: ConnTabId) -> Option<&Completion> {
    sql(harness, tab).completion.as_ref()
}

/// What the open list offers, in order; nothing when it is closed.
fn labels(harness: &Harness, tab: ConnTabId) -> Vec<String> {
    list(harness, tab)
        .map(|list| list.candidates.iter().map(|c| c.label.clone()).collect())
        .unwrap_or_default()
}

#[test]
fn typing_two_letters_opens_the_list_and_one_does_not() {
    let (mut harness, tab) = editor();
    type_text(&mut harness, "s");
    assert!(list(&harness, tab).is_none());
    type_text(&mut harness, "e");
    assert_eq!(labels(&harness, tab), ["select", "set"]);
    let open = list(&harness, tab).unwrap();
    assert_eq!(
        (open.selected, open.manual, open.typed.as_str()),
        (0, false, "se")
    );
}

#[test]
fn the_list_narrows_while_typing_and_closes_with_nothing_left() {
    let (mut harness, tab) = editor();
    type_text(&mut harness, "se");
    type_text(&mut harness, "l");
    assert_eq!(labels(&harness, tab), ["select"]);
    type_text(&mut harness, "x");
    assert!(list(&harness, tab).is_none());
}

#[test]
fn a_word_that_is_already_whole_opens_nothing() {
    let (mut harness, tab) = editor();
    // `set` is the only keyword that starts with `set`.
    type_text(&mut harness, "set");
    assert!(list(&harness, tab).is_none());
}

#[test]
fn pasting_and_deleting_open_nothing_and_deleting_keeps_an_open_list() {
    let (mut harness, tab) = editor();
    paste(&mut harness, "sele");
    assert!(list(&harness, tab).is_none());
    harness.press(Key::Backspace, Modifiers::NONE);
    assert_eq!(sql(&harness, tab).text, "sel");
    assert!(list(&harness, tab).is_none());
    type_text(&mut harness, "e");
    assert_eq!(labels(&harness, tab), ["select"]);
    // Open: deleting keeps it while its word remains.
    harness.press(Key::Backspace, Modifiers::NONE);
    harness.press(Key::Backspace, Modifiers::NONE);
    assert_eq!(labels(&harness, tab), ["select", "set"]);
    harness.press(Key::Backspace, Modifiers::NONE);
    harness.press(Key::Backspace, Modifiers::NONE);
    assert_eq!(sql(&harness, tab).text, "");
    assert!(list(&harness, tab).is_none());
}

#[test]
fn the_list_closes_when_the_cursor_leaves_its_word() {
    let (mut harness, tab) = editor();
    type_text(&mut harness, "se");
    assert!(list(&harness, tab).is_some());
    harness.press(Key::Home, Modifiers::NONE);
    assert_eq!(sql(&harness, tab).cursor, 0);
    assert!(list(&harness, tab).is_none());
}

#[test]
fn no_list_after_a_table_name_or_as() {
    let (mut harness, tab) = editor();
    paste(&mut harness, "select * from users ");
    // Most likely an alias: Enter must stay a line break.
    type_text(&mut harness, "wh");
    assert!(list(&harness, tab).is_none());
    // Asked for by hand, the keywords are there.
    let (tab_id, sql_tab) = ids(&harness, tab);
    harness.app.apply(Action::OpenCompletion {
        tab: tab_id,
        sql_tab,
    });
    harness.settle();
    assert_eq!(labels(&harness, tab), ["when", "where"]);
    assert!(list(&harness, tab).unwrap().manual);

    let (mut harness, tab) = editor();
    paste(&mut harness, "select 1 as ");
    type_text(&mut harness, "to");
    assert!(list(&harness, tab).is_none());
}

#[test]
fn a_list_asked_for_by_hand_opens_on_an_empty_word() {
    let (mut harness, tab) = editor();
    let (tab_id, sql_tab) = ids(&harness, tab);
    harness.app.apply(Action::OpenCompletion {
        tab: tab_id,
        sql_tab,
    });
    harness.settle();
    let open = list(&harness, tab).expect("a list");
    assert!(open.manual);
    assert_eq!(open.candidates[0].label, "ALL");
    // It goes on as its word is typed, still by hand.
    type_text(&mut harness, "s");
    assert_eq!(labels(&harness, tab), ["select", "set", "show"]);
    assert!(list(&harness, tab).unwrap().manual);
}

#[test]
fn a_list_belongs_to_the_editor_on_screen() {
    let (mut harness, tab) = editor();
    type_text(&mut harness, "se");
    harness.press(Key::T, Modifiers::COMMAND);
    let workspace = harness.app.workspace(tab).unwrap();
    assert_eq!(workspace.sql_tabs().count(), 2);
    assert!(workspace.sql_tabs().all(|sql| sql.completion.is_none()));
}

#[test]
fn running_closes_the_list() {
    let (mut harness, tab) = editor();
    type_text(&mut harness, "se");
    harness.press(Key::Enter, Modifiers::COMMAND);
    assert!(matches!(
        crate::testing::last_sent(&harness.app),
        Command::RunSql { .. }
    ));
    assert!(list(&harness, tab).is_none());
}

#[test]
fn a_quiet_frame_computes_nothing() {
    let (mut harness, tab) = editor();
    type_text(&mut harness, "se");
    assert!(list(&harness, tab).is_some());
    let counts = || (LISTED.with(|c| c.get()), TOKENIZED.with(|c| c.get()));
    let before = counts();
    harness.settle();
    harness.settle();
    assert_eq!(counts(), before);
    // One more letter: the script is tokenized once, for the editor and
    // the list together.
    type_text(&mut harness, "l");
    assert_eq!(counts().1, before.1 + 1);
    // And the list is worked out once.
    assert_eq!(counts().0, before.0 + 1);
    assert_eq!(labels(&harness, tab), ["select"]);
}

#[test]
fn text_an_input_method_commits_opens_the_list_and_text_it_composes_does_not() {
    let commit = |text: &str| egui::Event::Ime(egui::ImeEvent::Commit(text.into()));
    // Committed at once, with no composition before it.
    let (mut harness, tab) = editor();
    harness.frame(vec![commit("se")]);
    harness.settle();
    assert_eq!(sql(&harness, tab).text, "se");
    assert_eq!(labels(&harness, tab), ["select", "set"]);

    // Composed first: the editor shows the text, and no list opens on it
    // until it is committed.
    let (mut harness, tab) = editor();
    let preedit = egui::ImeEvent::Preedit {
        text: "se".into(),
        active_range_chars: None,
    };
    harness.frame(vec![egui::Event::Ime(preedit)]);
    harness.settle();
    assert_eq!(sql(&harness, tab).text, "se");
    assert!(list(&harness, tab).is_none());
    harness.frame(vec![commit("se")]);
    harness.settle();
    assert_eq!(sql(&harness, tab).text, "se");
    assert_eq!(labels(&harness, tab), ["select", "set"]);
}

#[test]
fn text_events_the_editor_ignores_are_not_typing() {
    let ignored = [
        egui::Event::Text("\n".into()),
        egui::Event::Text("\r".into()),
        egui::Event::Text(String::new()),
        egui::Event::Ime(egui::ImeEvent::Commit(String::new())),
    ];
    for event in ignored {
        let (mut harness, tab) = editor();
        // The paste changes the text; the event beside it types nothing.
        harness.frame(vec![egui::Event::Paste("sel".into()), event]);
        harness.settle();
        assert_eq!(sql(&harness, tab).text, "sel");
        assert!(list(&harness, tab).is_none());
    }
}

#[test]
fn the_typed_word_is_counted_in_bytes_after_wide_characters() {
    let (mut harness, tab) = editor();
    // A statement before it, with characters of two bytes.
    paste(&mut harness, "select 'żółw';\n");
    type_text(&mut harness, "se");
    assert_eq!(labels(&harness, tab), ["select", "set"]);
    assert_eq!(list(&harness, tab).unwrap().typed, "se");
}

#[test]
fn no_list_inside_a_string_or_a_comment() {
    let (mut harness, tab) = editor();
    paste(&mut harness, "select '");
    type_text(&mut harness, "se");
    assert!(list(&harness, tab).is_none());

    let (mut harness, tab) = editor();
    paste(&mut harness, "-- ");
    type_text(&mut harness, "se");
    assert!(list(&harness, tab).is_none());

    // An open list closes when the cursor moves into one: up a line, into
    // the comment, then the same into a string.
    for above in ["-- a note\n", "'a string',\n"] {
        let (mut harness, tab) = editor();
        paste(&mut harness, above);
        type_text(&mut harness, "se");
        assert_eq!(labels(&harness, tab), ["select", "set"], "{above:?}");
        harness.press(Key::ArrowUp, Modifiers::NONE);
        assert_eq!(sql(&harness, tab).cursor, 2);
        assert!(list(&harness, tab).is_none(), "{above:?}");
    }
}

#[test]
fn the_list_follows_the_cursor_inside_its_word() {
    let (mut harness, tab) = editor();
    type_text(&mut harness, "sel");
    assert_eq!(labels(&harness, tab), ["select"]);
    harness.press(Key::ArrowLeft, Modifiers::NONE);
    assert_eq!(sql(&harness, tab).cursor, 2);
    assert_eq!(labels(&harness, tab), ["select", "set"]);
    assert_eq!(list(&harness, tab).unwrap().typed, "se");
}

#[test]
fn a_list_opened_on_an_empty_word_stays_when_its_word_is_typed_and_deleted() {
    let (mut harness, tab) = editor();
    let (tab_id, sql_tab) = ids(&harness, tab);
    harness.app.apply(Action::OpenCompletion {
        tab: tab_id,
        sql_tab,
    });
    harness.settle();
    type_text(&mut harness, "s");
    assert_eq!(labels(&harness, tab), ["select", "set", "show"]);
    harness.press(Key::Backspace, Modifiers::NONE);
    assert_eq!(sql(&harness, tab).text, "");
    // The whole list again, still by hand.
    let open = list(&harness, tab).expect("a list");
    assert_eq!(open.candidates[0].label, "ALL");
    assert!(open.manual);
    assert_eq!(open.typed, "");
}

#[test]
fn a_list_waiting_to_insert_its_row_is_not_worked_out_again() {
    let (mut harness, tab) = editor();
    type_text(&mut harness, "se");
    let (tab_id, sql_tab) = ids(&harness, tab);
    harness.app.apply(Action::AcceptCompletion {
        tab: tab_id,
        sql_tab,
        row: Some(1),
    });
    let listed = LISTED.with(|c| c.get());
    // The names it reads change under it.
    harness.app.workspace_mut(tab).unwrap().catalog_generation += 1;
    harness.settle();
    let open = list(&harness, tab).expect("a list");
    assert!(open.accept);
    assert_eq!(open.selected, 1);
    assert!(open.is_of((TextPrint::of("se"), 2, 0)));
    // A list asked for meanwhile waits too.
    harness.app.apply(Action::OpenCompletion {
        tab: tab_id,
        sql_tab,
    });
    harness.settle();
    let open = list(&harness, tab).expect("a list");
    assert!(open.accept && !open.manual);
    assert_eq!(sql(&harness, tab).completion_wanted, Some(Wanted::Manual));
    assert_eq!(LISTED.with(|c| c.get()), listed);
}

#[test]
fn a_click_on_a_row_that_is_gone_inserts_nothing() {
    let (mut harness, tab) = editor();
    type_text(&mut harness, "se");
    let (tab_id, sql_tab) = ids(&harness, tab);
    harness.app.apply(Action::AcceptCompletion {
        tab: tab_id,
        sql_tab,
        row: Some(2),
    });
    let open = list(&harness, tab).expect("a list");
    assert_eq!((open.selected, open.accept), (0, false));
}

#[test]
fn a_whole_word_opens_nothing_by_hand_either() {
    let (mut harness, tab) = editor();
    paste(&mut harness, "set");
    let (tab_id, sql_tab) = ids(&harness, tab);
    harness.app.apply(Action::OpenCompletion {
        tab: tab_id,
        sql_tab,
    });
    harness.settle();
    assert!(list(&harness, tab).is_none());
}
