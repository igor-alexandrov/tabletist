//! Headless tests of the SQL editor's completion list: what opens it and
//! what closes it. The tasks that give it keys, a view and names from the
//! database add their tests here.

use egui::text::{CCursor, CCursorRange};
use egui::{Key, Modifiers};

use crate::backend::Command;
use crate::completion::LISTED;
use crate::model::{Action, Completion, ConnTabId, SqlTab, TabId, TextPrint, Wanted};
use crate::testing::{Harness, key, release};
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

/// Puts the editor's cursor at the character `at`, as a click there does.
fn put_cursor(harness: &mut Harness, tab: ConnTabId, at: usize) {
    let (tab, id) = ids(harness, tab);
    let editor = crate::ui::sql_text::editor_id(tab, id);
    let mut state = egui::TextEdit::load_state(&harness.ctx, editor).expect("an editor");
    let cursor = CCursorRange::one(CCursor::new(at));
    state.cursor.set_char_range(Some(cursor));
    egui::TextEdit::store_state(&harness.ctx, editor, state);
    harness.settle();
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
    // the comment, then the same into a string. (Put there as a click
    // does: with a list open the arrows up and down are the list's.)
    for above in ["-- a note\n", "'a string',\n"] {
        let (mut harness, tab) = editor();
        paste(&mut harness, above);
        type_text(&mut harness, "se");
        assert_eq!(labels(&harness, tab), ["select", "set"], "{above:?}");
        put_cursor(&mut harness, tab, 2);
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
    // The names it reads change under it. (Not a frame: the editor's next
    // draw inserts the row and takes the list.)
    harness.app.workspace_mut(tab).unwrap().catalog_generation += 1;
    harness.app.refresh_completion(&harness.ctx);
    let open = list(&harness, tab).expect("a list");
    assert!(open.accept);
    assert_eq!(open.selected, 1);
    assert!(open.is_of((TextPrint::of("se"), 2, 0)));
    // A list asked for meanwhile waits too.
    harness.app.apply(Action::OpenCompletion {
        tab: tab_id,
        sql_tab,
    });
    harness.app.refresh_completion(&harness.ctx);
    let open = list(&harness, tab).expect("a list");
    assert!(open.accept && !open.manual);
    assert_eq!(sql(&harness, tab).completion_wanted, Some(Wanted::Manual));
    assert_eq!(LISTED.with(|c| c.get()), listed);
    // The editor's next draw inserts the row, and the list asked for by
    // hand opens after it.
    harness.settle();
    assert_eq!(sql(&harness, tab).text, "set");
    assert_eq!(labels(&harness, tab), ["set"]);
    let open = list(&harness, tab).expect("a list");
    assert!(open.manual && !open.accept);
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
fn a_whole_word_opens_its_one_row_by_hand() {
    let (mut harness, tab) = editor();
    paste(&mut harness, "set");
    let (tab_id, sql_tab) = ids(&harness, tab);
    harness.app.apply(Action::OpenCompletion {
        tab: tab_id,
        sql_tab,
    });
    harness.settle();
    // Asked for, the list shows even when all it has is the word itself.
    assert_eq!(labels(&harness, tab), ["set"]);
    assert!(list(&harness, tab).unwrap().manual);
}

#[test]
fn a_list_opened_on_an_empty_word_closes_at_the_start_of_a_word_typed_since() {
    let (mut harness, tab) = editor();
    let (tab_id, sql_tab) = ids(&harness, tab);
    harness.app.apply(Action::OpenCompletion {
        tab: tab_id,
        sql_tab,
    });
    harness.settle();
    type_text(&mut harness, "se");
    assert_eq!(labels(&harness, tab), ["select", "set"]);
    // Before `se`, a row would go in front of it instead of replacing it.
    harness.press(Key::Home, Modifiers::NONE);
    assert_eq!(sql(&harness, tab).cursor, 0);
    assert!(list(&harness, tab).is_none());
}

/// What a real Ctrl press reports: outside macOS Ctrl is the command key
/// too.
fn ctrl() -> Modifiers {
    if cfg!(target_os = "macos") {
        Modifiers::CTRL
    } else {
        Modifiers::CTRL | Modifiers::COMMAND
    }
}

fn editor_has_keyboard(harness: &Harness, tab: ConnTabId) -> bool {
    let (tab, id) = ids(harness, tab);
    let editor = crate::ui::sql_text::editor_id(tab, id);
    harness.ctx.memory(|memory| memory.has_focus(editor))
}

fn selected(harness: &Harness, tab: ConnTabId) -> Option<usize> {
    list(harness, tab).map(|list| list.selected)
}

#[test]
fn down_and_up_move_the_highlight_and_stop_at_the_ends() {
    let (mut harness, tab) = editor();
    type_text(&mut harness, "se");
    for (key, row) in [
        (Key::ArrowDown, 1),
        (Key::ArrowDown, 1),
        (Key::ArrowUp, 0),
        (Key::ArrowUp, 0),
    ] {
        harness.press(key, Modifiers::NONE);
        assert_eq!(selected(&harness, tab), Some(row), "{key:?}");
    }
    // The editor's cursor stayed where it was.
    assert_eq!(sql(&harness, tab).cursor, 2);
}

#[test]
fn tab_inserts_the_highlighted_row() {
    let (mut harness, tab) = editor();
    type_text(&mut harness, "se");
    harness.press(Key::ArrowDown, Modifiers::NONE);
    harness.press(Key::Tab, Modifiers::NONE);
    let sql = sql(&harness, tab);
    assert_eq!((sql.text.as_str(), sql.cursor), ("set", 3));
    assert!(sql.completion.is_none());
    assert!(editor_has_keyboard(&harness, tab));
}

#[test]
fn enter_inserts_and_what_is_typed_next_follows_it() {
    let (mut harness, tab) = editor();
    type_text(&mut harness, "sel");
    harness.press(Key::Enter, Modifiers::NONE);
    assert_eq!(sql(&harness, tab).text, "select");
    type_text(&mut harness, " 1");
    assert_eq!(sql(&harness, tab).text, "select 1");
}

#[test]
fn enter_on_what_is_already_typed_is_a_line_break() {
    let (mut harness, tab) = editor();
    type_text(&mut harness, "as");
    assert_eq!(labels(&harness, tab), ["as", "asc"]);
    harness.press(Key::Enter, Modifiers::NONE);
    assert_eq!(sql(&harness, tab).text, "as\n");
    assert!(list(&harness, tab).is_none());
}

#[test]
fn shift_enter_and_shift_tab_are_the_editors() {
    let (mut harness, tab) = editor();
    type_text(&mut harness, "se");
    harness.press(Key::Enter, Modifiers::SHIFT);
    assert_eq!(sql(&harness, tab).text, "se\n");

    let (mut harness, tab) = editor();
    type_text(&mut harness, "se");
    harness.press(Key::Tab, Modifiers::SHIFT);
    // Nothing to outdent, and nothing inserted.
    assert_eq!(sql(&harness, tab).text, "se");
    assert!(list(&harness, tab).is_some());
}

#[test]
fn escape_closes_the_list_then_leaves_the_editor() {
    let (mut harness, tab) = editor();
    type_text(&mut harness, "se");
    harness.press(Key::Escape, Modifiers::NONE);
    assert!(list(&harness, tab).is_none());
    assert!(
        editor_has_keyboard(&harness, tab),
        "the first Esc is the list's"
    );
    harness.press(Key::Escape, Modifiers::NONE);
    assert!(!editor_has_keyboard(&harness, tab), "the second leaves");
}

#[test]
fn left_and_right_stay_the_editors_while_the_list_is_open() {
    let (mut harness, tab) = editor();
    type_text(&mut harness, "se");
    harness.press(Key::ArrowLeft, Modifiers::NONE);
    assert_eq!(sql(&harness, tab).cursor, 1);
    assert!(editor_has_keyboard(&harness, tab));
    // Still on its word, with less of it typed.
    assert_eq!(labels(&harness, tab), ["select", "set", "show"]);
}

#[test]
fn undo_restores_the_typed_word() {
    let (mut harness, tab) = editor();
    type_text(&mut harness, "sel");
    harness.press(Key::Tab, Modifiers::NONE);
    assert_eq!(sql(&harness, tab).text, "select");
    harness.press(Key::Z, Modifiers::COMMAND);
    assert_eq!(sql(&harness, tab).text, "sel");
}

#[test]
fn ctrl_space_opens_the_list_by_hand() {
    let (mut harness, tab) = editor();
    harness.press(Key::Space, ctrl());
    let open = list(&harness, tab).expect("a list");
    assert!(open.manual);
    assert_eq!(open.candidates[0].label, "ALL");
    assert_eq!(sql(&harness, tab).text, "");
}

#[test]
fn ctrl_n_and_ctrl_p_move_the_highlight_only_in_the_terminal_look() {
    let (mut harness, tab) = editor();
    harness.set_look(crate::theme::Look::omarchy());
    type_text(&mut harness, "se");
    harness.press(Key::N, ctrl());
    assert_eq!(selected(&harness, tab), Some(1));
    // The list took the key: New connection did not open.
    assert!(harness.app.dialog.is_none());
    harness.press(Key::P, ctrl());
    assert_eq!(selected(&harness, tab), Some(0));

    // The other looks: not the list's keys. Ctrl+N is New connection
    // where Mod is Ctrl (it reports `command` too), and nothing on macOS.
    for (ctrl, opens) in [
        (Modifiers::CTRL, false),
        (Modifiers::CTRL | Modifiers::COMMAND, true),
    ] {
        for key in [Key::N, Key::P] {
            let (mut harness, tab) = editor();
            type_text(&mut harness, "se");
            harness.press(key, ctrl);
            assert_eq!(selected(&harness, tab), Some(0), "{key:?} {ctrl:?}");
            assert_eq!(harness.app.dialog.is_some(), opens, "{key:?} {ctrl:?}");
        }
    }
}

/// Takes the keyboard from whatever has it.
fn drop_keyboard(harness: &mut Harness) {
    harness.ctx.memory_mut(|memory| {
        if let Some(focused) = memory.focused() {
            memory.surrender_focus(focused);
        }
    });
    harness.settle();
}

#[test]
fn the_lists_keys_wait_while_the_keyboard_is_elsewhere() {
    for key in [Key::ArrowDown, Key::Tab, Key::Enter, Key::Escape] {
        let (mut harness, tab) = editor();
        type_text(&mut harness, "se");
        // The keyboard leaves the editor; the model still holds its list.
        drop_keyboard(&mut harness);
        assert!(!editor_has_keyboard(&harness, tab));
        assert_eq!(selected(&harness, tab), Some(0));
        harness.press(key, Modifiers::NONE);
        let open = list(&harness, tab).unwrap_or_else(|| panic!("{key:?} closed the list"));
        assert_eq!((open.selected, open.accept), (0, false), "{key:?}");
        assert_eq!(sql(&harness, tab).text, "se", "{key:?}");
        assert!(harness.app.dialog.is_none(), "{key:?}");
    }
    // Nor does Ctrl+Space open one.
    for ctrl in [Modifiers::CTRL, Modifiers::CTRL | Modifiers::COMMAND] {
        let (mut harness, tab) = editor();
        drop_keyboard(&mut harness);
        harness.press(Key::Space, ctrl);
        assert!(list(&harness, tab).is_none(), "{ctrl:?}");
        assert!(sql(&harness, tab).completion_wanted.is_none(), "{ctrl:?}");
    }
}

#[test]
fn a_list_opened_before_a_word_stays_when_a_letter_is_typed_and_deleted() {
    let (mut harness, tab) = editor();
    paste(&mut harness, "users");
    harness.press(Key::Home, Modifiers::NONE);
    let (tab_id, sql_tab) = ids(&harness, tab);
    harness.app.apply(Action::OpenCompletion {
        tab: tab_id,
        sql_tab,
    });
    harness.settle();
    assert!(list(&harness, tab).is_some());
    type_text(&mut harness, "s");
    assert_eq!(sql(&harness, tab).text, "susers");
    assert!(list(&harness, tab).is_some());
    harness.press(Key::Backspace, Modifiers::NONE);
    assert_eq!(sql(&harness, tab).text, "users");
    // As on an empty line: the whole list again, still by hand.
    let open = list(&harness, tab).expect("a list");
    assert!(open.manual);
    assert_eq!(open.typed, "");
}

#[test]
fn enter_in_the_frame_after_tab_is_a_line_break_after_the_inserted_row() {
    let (mut harness, tab) = editor();
    type_text(&mut harness, "sel");
    // No frame between the two: the row is inserted in the second.
    harness.frame(vec![key(Key::Tab, Modifiers::NONE)]);
    harness.frame(vec![key(Key::Enter, Modifiers::NONE)]);
    harness.settle();
    assert_eq!(sql(&harness, tab).text, "select\n");
    assert!(list(&harness, tab).is_none());
}

#[test]
fn escape_after_an_insertion_leaves_the_editor() {
    let (mut harness, tab) = editor();
    type_text(&mut harness, "sel");
    harness.frame(vec![key(Key::Tab, Modifiers::NONE)]);
    harness.frame(vec![release(Key::Tab, Modifiers::NONE)]);
    assert_eq!(sql(&harness, tab).text, "select");
    assert!(editor_has_keyboard(&harness, tab));
    // The very next frame: no list holds Esc any more.
    harness.frame(vec![key(Key::Escape, Modifiers::NONE)]);
    harness.frame(vec![release(Key::Escape, Modifiers::NONE)]);
    assert!(!editor_has_keyboard(&harness, tab));
}

#[test]
fn tab_and_enter_in_a_frame_that_also_types_are_the_editors() {
    let typing = |text: &str| -> [egui::Event; 3] {
        [
            egui::Event::Text(text.into()),
            egui::Event::Paste(text.into()),
            egui::Event::Ime(egui::ImeEvent::Commit(text.into())),
        ]
    };
    for (pressed, after) in [(Key::Tab, "set\t"), (Key::Enter, "set\n")] {
        for typed in typing("t") {
            let (mut harness, tab) = editor();
            type_text(&mut harness, "se");
            assert_eq!(labels(&harness, tab), ["select", "set"]);
            // The list was worked out from `se`, not from what this frame
            // types: its row is not inserted.
            let events = vec![typed.clone(), key(pressed, Modifiers::NONE)];
            harness.frame(events);
            harness.settle();
            assert_eq!(sql(&harness, tab).text, after, "{typed:?}");
            assert!(list(&harness, tab).is_none(), "{typed:?}");
        }
    }
}

#[test]
fn a_list_of_another_script_inserts_nothing() {
    let (mut harness, tab) = editor();
    type_text(&mut harness, "se");
    let (tab_id, sql_tab) = ids(&harness, tab);
    harness.app.apply(Action::AcceptCompletion {
        tab: tab_id,
        sql_tab,
        row: None,
    });
    // The script changes before the editor is drawn again.
    let workspace = harness.app.workspace_mut(tab).unwrap();
    workspace.sql_tab_mut(sql_tab).unwrap().text = "x se".into();
    harness.frame(Vec::new());
    assert_eq!(sql(&harness, tab).text, "x se");
    assert!(list(&harness, tab).is_none());
}

#[test]
fn a_row_goes_in_after_characters_of_more_than_one_byte() {
    let (mut harness, tab) = editor();
    paste(&mut harness, "select 'żółw';\n");
    type_text(&mut harness, "se");
    harness.press(Key::Tab, Modifiers::NONE);
    type_text(&mut harness, " 1");
    let sql_tab = sql(&harness, tab);
    assert_eq!(sql_tab.text, "select 'żółw';\nselect 1");
    assert_eq!(sql_tab.cursor, sql_tab.text.len());

    // With a line after it, so a cursor counted wrongly shows.
    let (mut harness, tab) = editor();
    paste(&mut harness, "select 'żółw';\n\nselect 2");
    // The start of the empty line: 14 characters and a line ending.
    put_cursor(&mut harness, tab, 15);
    type_text(&mut harness, "se");
    assert_eq!(labels(&harness, tab), ["select", "set"]);
    harness.press(Key::Tab, Modifiers::NONE);
    type_text(&mut harness, " 1");
    let sql_tab = sql(&harness, tab);
    assert_eq!(sql_tab.text, "select 'żółw';\nselect 1\nselect 2");
    assert_eq!(sql_tab.cursor, "select 'żółw';\nselect 1".len());
}

#[test]
fn a_row_replaces_the_whole_word_the_cursor_is_inside_of() {
    let (mut harness, tab) = editor();
    type_text(&mut harness, "selec");
    harness.press(Key::ArrowLeft, Modifiers::NONE);
    harness.press(Key::ArrowLeft, Modifiers::NONE);
    assert_eq!(sql(&harness, tab).cursor, 3);
    assert_eq!(labels(&harness, tab), ["select"]);
    harness.press(Key::Tab, Modifiers::NONE);
    let sql = sql(&harness, tab);
    assert_eq!((sql.text.as_str(), sql.cursor), ("select", 6));
}

#[test]
fn tab_on_the_row_that_is_already_typed_changes_nothing() {
    let (mut harness, tab) = editor();
    type_text(&mut harness, "as");
    assert_eq!(labels(&harness, tab), ["as", "asc"]);
    harness.press(Key::Tab, Modifiers::NONE);
    // No row inserted, no tab character, and the list is done.
    assert_eq!(sql(&harness, tab).text, "as");
    assert!(list(&harness, tab).is_none());
    // Nor an undo point: undo takes back all that was typed, as it does
    // when no list was in the way.
    type_text(&mut harness, " x");
    harness.press(Key::Z, Modifiers::COMMAND);
    assert_eq!(sql(&harness, tab).text, "");
}

#[test]
fn an_insertion_is_an_undo_step_of_its_own() {
    let (mut harness, tab) = editor();
    type_text(&mut harness, "sel");
    harness.press(Key::Tab, Modifiers::NONE);
    type_text(&mut harness, " 1");
    assert_eq!(sql(&harness, tab).text, "select 1");
    // What was typed after it, then the insertion itself.
    harness.press(Key::Z, Modifiers::COMMAND);
    assert_eq!(sql(&harness, tab).text, "select");
    harness.press(Key::Z, Modifiers::COMMAND);
    assert_eq!(sql(&harness, tab).text, "sel");
    // And back, by either redo key.
    harness.press(Key::Z, Modifiers::COMMAND | Modifiers::SHIFT);
    assert_eq!(sql(&harness, tab).text, "select");
    assert_eq!(sql(&harness, tab).cursor, 6);
    harness.press(Key::Y, Modifiers::COMMAND);
    assert_eq!(sql(&harness, tab).text, "select 1");
}

#[test]
fn every_press_of_a_frame_moves_the_highlight() {
    let (mut harness, tab) = editor();
    harness.press(Key::Space, ctrl());
    assert_eq!(selected(&harness, tab), Some(0));
    let down = || key(Key::ArrowDown, Modifiers::NONE);
    harness.frame(vec![down(), down()]);
    harness.settle();
    assert_eq!(selected(&harness, tab), Some(2));
    // A held key repeats, and each repeat moves it.
    let held = egui::Event::Key {
        key: Key::ArrowDown,
        physical_key: None,
        pressed: true,
        repeat: true,
        modifiers: Modifiers::NONE,
    };
    harness.frame(vec![held.clone()]);
    harness.frame(vec![held.clone(), held]);
    harness.settle();
    assert_eq!(selected(&harness, tab), Some(5));
    assert_eq!(sql(&harness, tab).text, "");
}

#[test]
fn ctrl_n_and_ctrl_p_are_the_lists_while_it_waits_for_its_rows() {
    let (mut harness, tab) = editor();
    harness.set_look(crate::theme::Look::omarchy());
    type_text(&mut harness, "se");
    // A list whose names are still being fetched: no row yet.
    let (_, id) = ids(&harness, tab);
    let workspace = harness.app.workspace_mut(tab).unwrap();
    let open = workspace.sql_tab_mut(id).unwrap().completion.as_mut();
    let open = open.expect("a list");
    open.candidates = std::sync::Arc::new(Vec::new());
    open.loading = true;
    for key in [Key::N, Key::P] {
        harness.press(key, ctrl());
        // Neither New connection nor Quick open.
        assert!(harness.app.dialog.is_none(), "{key:?}");
        assert!(list(&harness, tab).is_some(), "{key:?}");
    }
    // The arrows are the editor's until there is a row to move to.
    harness.press(Key::ArrowUp, Modifiers::NONE);
    assert_eq!(sql(&harness, tab).cursor, 0);
}

#[test]
fn ctrl_is_ctrl_as_every_platform_reports_it_and_cmd_is_not() {
    // Ctrl on macOS, then Ctrl elsewhere (the command key too).
    for ctrl in [Modifiers::CTRL, Modifiers::CTRL | Modifiers::COMMAND] {
        let (mut harness, tab) = editor();
        harness.set_look(crate::theme::Look::omarchy());
        harness.press(Key::Space, ctrl);
        assert!(
            list(&harness, tab).is_some_and(|list| list.manual),
            "{ctrl:?}"
        );
        harness.press(Key::N, ctrl);
        assert_eq!(selected(&harness, tab), Some(1), "{ctrl:?}");
        harness.press(Key::P, ctrl);
        assert_eq!(selected(&harness, tab), Some(0), "{ctrl:?}");
        assert!(harness.app.dialog.is_none(), "{ctrl:?}");
    }
    // Cmd on macOS, alone and with Ctrl: New connection, as ever.
    let cmd = Modifiers::MAC_CMD | Modifiers::COMMAND;
    for held in [cmd, cmd | Modifiers::CTRL] {
        let (mut harness, tab) = editor();
        harness.set_look(crate::theme::Look::omarchy());
        type_text(&mut harness, "se");
        harness.press(Key::N, held);
        assert_eq!(selected(&harness, tab), Some(0), "{held:?}");
        assert!(harness.app.dialog.is_some(), "{held:?}");
    }
}

#[test]
fn a_frame_follows_an_insertion() {
    let (mut harness, tab) = editor();
    type_text(&mut harness, "sel");
    // Quiet frames: nothing asks to be drawn again at once.
    harness.settle();
    harness.settle();
    assert_ne!(harness.repaint_after, std::time::Duration::ZERO);
    let (tab_id, sql_tab) = ids(&harness, tab);
    harness.app.apply(Action::AcceptCompletion {
        tab: tab_id,
        sql_tab,
        row: None,
    });
    // The frame that inserts the row asks for the next one: the footer,
    // drawn before the editor, still says the old column.
    harness.frame(Vec::new());
    assert_eq!(sql(&harness, tab).text, "select");
    assert_eq!(harness.repaint_after, std::time::Duration::ZERO);
}

#[test]
fn the_caret_is_scrolled_into_view_after_an_insertion() {
    let (mut harness, tab) = editor();
    // One line, wider than the pane: typing at its end scrolls to it.
    paste(&mut harness, &format!("select {}", "1, ".repeat(300)));
    type_text(&mut harness, "fr");
    let (tab_id, id) = ids(&harness, tab);
    harness.app.apply(Action::OpenCompletion {
        tab: tab_id,
        sql_tab: id,
    });
    harness.finish_animations();
    let row = list(&harness, tab).and_then(|list| list.highlighted());
    assert!(row.expect("a row").insert.len() > 2);
    let scrolled = |harness: &Harness| {
        crate::ui::sql_text::scrolled_to(&harness.ctx, tab_id, id)
            .expect("a scroll area")
            .x
    };
    let before = scrolled(&harness);
    assert!(before > 0.0);
    harness.press(Key::Tab, Modifiers::NONE);
    harness.finish_animations();
    assert!(sql(&harness, tab).text.len() > "select ".len() + 900 + 2);
    assert!(scrolled(&harness) > before);
}
