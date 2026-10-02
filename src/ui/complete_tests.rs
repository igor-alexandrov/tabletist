//! Headless tests of the SQL editor's completion list: what opens it and
//! what closes it. The tasks that give it keys, a view and names from the
//! database add their tests here.

use egui::accesskit::Role;
use egui::text::{CCursor, CCursorRange};
use egui::{Key, Modifiers};

use tabletist_db::{ObjectInfo, ObjectKind};

use crate::backend::{Command, Event};
use crate::completion::{Candidate, Kind, LISTED};
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
    let workspace = harness.app.workspace_mut(tab).unwrap();
    // The catalog the list was worked out from: connecting changed it.
    let generation = workspace.catalog_generation;
    workspace.catalog_changed();
    assert_ne!(workspace.catalog_generation, generation);
    harness.app.refresh_completion(&harness.ctx);
    let open = list(&harness, tab).expect("a list");
    assert!(open.accept);
    assert_eq!(open.selected, 1);
    assert!(open.is_of((TextPrint::of("se"), 2, generation)));
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
    // where Mod is Ctrl (it reports `command` too), and no shortcut on
    // macOS. There egui's text field reads Ctrl+N and Ctrl+P itself, as
    // cursor down and up: the cursor may leave the word, which closes the
    // list. Nowhere do the keys move the highlight.
    let cursor_keys = cfg!(target_os = "macos");
    let unmoved = |harness: &Harness, tab, what: &str| {
        let highlight = selected(harness, tab);
        if cursor_keys {
            assert!(highlight.is_none_or(|row| row == 0), "{what}");
        } else {
            assert_eq!(highlight, Some(0), "{what}");
        }
    };
    for (ctrl, opens) in [
        (Modifiers::CTRL, false),
        (Modifiers::CTRL | Modifiers::COMMAND, true),
    ] {
        for pressed in [Key::N, Key::P] {
            let what = format!("{pressed:?} {ctrl:?}");
            let (mut harness, tab) = editor();
            type_text(&mut harness, "se");
            // The frame of the press: the highlight has not moved.
            harness.frame(vec![key(pressed, ctrl)]);
            unmoved(&harness, tab, &what);
            harness.frame(vec![release(pressed, ctrl)]);
            harness.settle();
            assert_eq!(harness.app.dialog.is_some(), opens, "{what}");
            if opens {
                // The dialog took the keyboard, and the list went.
                assert_eq!(selected(&harness, tab), None, "{what}");
            } else {
                unmoved(&harness, tab, &what);
            }
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
fn the_list_closes_when_the_keyboard_leaves_and_its_keys_do_nothing() {
    for key in [Key::ArrowDown, Key::Tab, Key::Enter, Key::Escape] {
        let (mut harness, tab) = editor();
        type_text(&mut harness, "se");
        assert_eq!(selected(&harness, tab), Some(0));
        // The keyboard leaves the editor: its list goes with it.
        drop_keyboard(&mut harness);
        assert!(!editor_has_keyboard(&harness, tab));
        assert!(list(&harness, tab).is_none(), "{key:?}");
        // The keys that were the list's do nothing to the script.
        harness.press(key, Modifiers::NONE);
        assert!(list(&harness, tab).is_none(), "{key:?}");
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
fn in_a_frame_that_also_types_tab_does_nothing_and_enter_is_a_line_break() {
    let typing = |text: &str| -> [egui::Event; 3] {
        [
            egui::Event::Text(text.into()),
            egui::Event::Paste(text.into()),
            egui::Event::Ime(egui::ImeEvent::Commit(text.into())),
        ]
    };
    // The list was worked out from `se`, not from what this frame types:
    // its row is not inserted. Tab is taken all the same, so no tab
    // character lands in the middle of the word, and the list goes on for
    // the word as it now reads.
    for typed in typing("l") {
        let (mut harness, tab) = editor();
        type_text(&mut harness, "se");
        assert_eq!(labels(&harness, tab), ["select", "set"]);
        harness.frame(vec![typed.clone(), key(Key::Tab, Modifiers::NONE)]);
        harness.settle();
        assert_eq!(sql(&harness, tab).text, "sel", "{typed:?}");
        assert_eq!(labels(&harness, tab), ["select"], "{typed:?}");
        assert_eq!(list(&harness, tab).unwrap().typed, "sel", "{typed:?}");
    }
    // Enter is the editor's line break, and the list is done.
    for typed in typing("t") {
        let (mut harness, tab) = editor();
        type_text(&mut harness, "se");
        assert_eq!(labels(&harness, tab), ["select", "set"]);
        harness.frame(vec![typed.clone(), key(Key::Enter, Modifiers::NONE)]);
        harness.settle();
        assert_eq!(sql(&harness, tab).text, "set\n", "{typed:?}");
        assert!(list(&harness, tab).is_none(), "{typed:?}");
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
fn an_insertion_leaves_nothing_to_redo_as_an_edit_does() {
    let (mut harness, tab) = editor();
    paste(&mut harness, "select 1;\n");
    paste(&mut harness, "select 2;\n");
    // The undo leaves the script it took back to redo.
    harness.press(Key::Z, Modifiers::COMMAND);
    let undone = sql(&harness, tab).text.clone();
    assert_ne!(undone, "select 1;\nselect 2;\n");
    // A row put in from the list asked for by hand: nothing was typed
    // since the undo.
    harness.press(Key::Space, ctrl());
    let row = list(&harness, tab)
        .and_then(Completion::highlighted)
        .expect("a row")
        .insert
        .clone();
    harness.press(Key::Tab, Modifiers::NONE);
    let inserted = format!("{undone}{row}");
    assert_eq!(sql(&harness, tab).text, inserted);
    // What the undo left is gone: neither redo key brings it back over
    // the insertion.
    harness.press(Key::Z, Modifiers::COMMAND | Modifiers::SHIFT);
    assert_eq!(sql(&harness, tab).text, inserted);
    harness.press(Key::Y, Modifiers::COMMAND);
    assert_eq!(sql(&harness, tab).text, inserted);
    // And one undo still takes the insertion back.
    harness.press(Key::Z, Modifiers::COMMAND);
    assert_eq!(sql(&harness, tab).text, undone);
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
    wait_for_rows(&mut harness, tab);
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
        // The frame of the press: the highlight has not moved.
        harness.frame(vec![key(Key::N, held)]);
        assert_eq!(selected(&harness, tab), Some(0), "{held:?}");
        harness.frame(vec![release(Key::N, held)]);
        harness.settle();
        assert!(harness.app.dialog.is_some(), "{held:?}");
        // The dialog took the keyboard, and the list went.
        assert!(list(&harness, tab).is_none(), "{held:?}");
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
        crate::ui::sql_text::scroll_offset(&harness.ctx, tab_id, id)
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

/// The node labelled `label`, whatever its role.
fn named<'a>(
    tree: &'a egui::accesskit::TreeUpdate,
    label: &str,
) -> Option<&'a (egui::accesskit::NodeId, egui::accesskit::Node)> {
    tree.nodes
        .iter()
        .find(|(_, node)| node.label() == Some(label))
}

/// Where the node labelled `label` sits, in points.
fn bounds_of(tree: &egui::accesskit::TreeUpdate, label: &str) -> egui::Rect {
    let (_, node) = named(tree, label).unwrap_or_else(|| panic!("nothing labelled {label:?}"));
    let rect = node.bounds().expect("bounds");
    egui::Rect::from_min_max(
        egui::pos2(rect.x0 as f32, rect.y0 as f32),
        egui::pos2(rect.x1 as f32, rect.y1 as f32),
    )
}

/// How many pieces of text the last frame painted hold `part`.
fn painted(harness: &Harness, part: &str) -> usize {
    harness
        .painted
        .iter()
        .filter(|(text, _)| text.contains(part))
        .count()
}

/// The list's row named `name`.
fn row(tree: &egui::accesskit::TreeUpdate, name: &str) -> Option<egui::accesskit::NodeId> {
    crate::testing::node(tree, name, Role::ListBoxOption)
}

/// The node `id`.
fn node_of(
    tree: &egui::accesskit::TreeUpdate,
    id: egui::accesskit::NodeId,
) -> &egui::accesskit::Node {
    let found = tree.nodes.iter().find(|(node, _)| *node == id);
    &found.expect("a node").1
}

/// The row the editor names as its current one: what a screen reader
/// takes for the focused object.
fn active_row(tree: &egui::accesskit::TreeUpdate) -> Option<egui::accesskit::NodeId> {
    let (_, editor) = named(tree, "SQL").expect("the editor");
    editor.active_descendant()
}

#[test]
fn the_list_is_a_list_box_and_its_rows_are_its_options() {
    use egui::accesskit::Action;
    let (mut harness, _tab) = editor();
    type_text(&mut harness, "se");
    let tree = harness.settle();
    let (_, whole) = named(&tree, "Completions").expect("the list");
    assert_eq!(whole.role(), Role::ListBox);
    assert_eq!(whole.size_of_set(), Some(2));
    let rows = ["select, keyword", "set, keyword"].map(|name| row(&tree, name).expect(name));
    // The rows are the list's own, in order. Between them and it is the
    // node egui makes for the `Ui` they are drawn in: a generic container,
    // which AccessKit leaves out of what a screen reader is told, so the
    // rows are the list's children there.
    let [between] = whole.children() else {
        panic!("{:?}", whole.children());
    };
    let between = node_of(&tree, *between);
    assert_eq!(between.role(), Role::GenericContainer);
    assert_eq!(between.children(), rows);
    for (index, id) in rows.into_iter().enumerate() {
        let option = node_of(&tree, id);
        assert_eq!(option.is_selected(), Some(index == 0), "row {index}");
        assert_eq!(option.position_in_set(), Some(index), "row {index}");
        assert_eq!(option.size_of_set(), Some(2), "row {index}");
        // A row is picked, never focused: the keyboard is the editor's.
        assert!(option.supports_action(Action::Click), "row {index}");
        assert!(!option.supports_action(Action::Focus), "row {index}");
    }
    assert!(!whole.supports_action(Action::Focus));
    // The highlight moves: so does the selected state.
    harness.press(Key::ArrowDown, Modifiers::NONE);
    let tree = harness.settle();
    for (name, selected) in [("select, keyword", false), ("set, keyword", true)] {
        let option = node_of(&tree, row(&tree, name).expect(name));
        assert_eq!(option.is_selected(), Some(selected), "{name}");
    }
}

#[test]
fn a_typed_list_leaves_a_screen_reader_on_the_editor_until_the_highlight_moves() {
    let (mut harness, _tab) = editor();
    type_text(&mut harness, "se");
    // It opened by itself: the editor stays the focused object, or a
    // reader would jump to a row on nearly every word typed.
    let tree = harness.settle();
    assert!(row(&tree, "select, keyword").is_some());
    assert_eq!(active_row(&tree), None);
    // The user moved in it: now the highlighted row is the current one.
    harness.press(Key::ArrowDown, Modifiers::NONE);
    let tree = harness.settle();
    let second = row(&tree, "set, keyword");
    assert!(second.is_some());
    assert_eq!(active_row(&tree), second);
    harness.press(Key::ArrowUp, Modifiers::NONE);
    let tree = harness.settle();
    assert_eq!(active_row(&tree), row(&tree, "select, keyword"));
    // Closed: the editor names no row.
    harness.press(Key::Escape, Modifiers::NONE);
    let tree = harness.settle();
    assert_eq!(active_row(&tree), None);
}

#[test]
fn a_list_opened_by_hand_names_its_highlighted_row_at_once() {
    let (mut harness, tab) = editor();
    type_text(&mut harness, "se");
    harness.press(Key::Space, ctrl());
    assert!(list(&harness, tab).is_some_and(|list| list.manual && !list.moved));
    let tree = harness.settle();
    let first = row(&tree, "select, keyword");
    assert!(first.is_some());
    assert_eq!(active_row(&tree), first);
}

/// An AccessKit request for `action` on the node `target`.
fn request(
    harness: &mut Harness,
    target: egui::accesskit::NodeId,
    action: egui::accesskit::Action,
) {
    let request = egui::accesskit::ActionRequest {
        target_tree: egui::accesskit::TreeId::ROOT,
        target_node: target,
        action,
        data: None,
    };
    harness.frame(vec![egui::Event::AccessKitActionRequest(request)]);
    harness.settle();
}

#[test]
fn a_focus_request_on_the_list_leaves_the_keyboard_with_the_editor() {
    for name in ["set, keyword", "Completions"] {
        let (mut harness, tab) = editor();
        type_text(&mut harness, "se");
        let tree = harness.settle();
        let (target, _) = *named(&tree, name).expect(name);
        request(&mut harness, target, egui::accesskit::Action::Focus);
        assert!(editor_has_keyboard(&harness, tab), "{name}");
        assert_eq!(labels(&harness, tab), ["select", "set"], "{name}");
        assert_eq!(sql(&harness, tab).text, "se", "{name}");
    }
}

#[test]
fn a_click_request_on_a_row_inserts_it() {
    let (mut harness, tab) = editor();
    type_text(&mut harness, "se");
    let tree = harness.settle();
    let target = row(&tree, "set, keyword").expect("a row");
    request(&mut harness, target, egui::accesskit::Action::Click);
    assert_eq!(sql(&harness, tab).text, "set");
    assert!(list(&harness, tab).is_none());
    assert!(editor_has_keyboard(&harness, tab));
}

/// A click of `button` at `at`: the pointer moves there, the button goes
/// down and comes up, a frame each.
fn click_at(harness: &mut Harness, at: egui::Pos2, button: egui::PointerButton) {
    let press = |pressed| egui::Event::PointerButton {
        pos: at,
        button,
        pressed,
        modifiers: Modifiers::NONE,
    };
    harness.frame(vec![egui::Event::PointerMoved(at)]);
    harness.frame(vec![press(true)]);
    harness.frame(vec![press(false)]);
    harness.settle();
}

/// A point of the list's footer: under its last row.
fn footer_point(tree: &egui::accesskit::TreeUpdate) -> egui::Pos2 {
    let whole = bounds_of(tree, "Completions");
    let at = egui::pos2(whole.center().x, whole.bottom() - 4.0);
    assert!(!bounds_of(tree, "set, keyword").contains(at));
    at
}

#[test]
fn a_click_on_a_row_inserts_it_and_the_editor_keeps_the_keyboard() {
    let (mut harness, tab) = editor();
    type_text(&mut harness, "se");
    let tree = harness.settle();
    let at = bounds_of(&tree, "set, keyword").center();
    click_at(&mut harness, at, egui::PointerButton::Primary);
    assert_eq!(sql(&harness, tab).text, "set");
    assert!(list(&harness, tab).is_none());
    assert!(editor_has_keyboard(&harness, tab));
}

#[test]
fn a_click_on_the_list_beside_its_rows_keeps_the_list_and_the_keyboard() {
    let (mut harness, tab) = editor();
    type_text(&mut harness, "se");
    let tree = harness.settle();
    let at = footer_point(&tree);
    click_at(&mut harness, at, egui::PointerButton::Primary);
    assert_eq!(sql(&harness, tab).text, "se");
    assert_eq!(labels(&harness, tab), ["select", "set"]);
    assert!(editor_has_keyboard(&harness, tab));
    // And the list's keys still work.
    harness.press(Key::Tab, Modifiers::NONE);
    assert_eq!(sql(&harness, tab).text, "select");
}

#[test]
fn a_secondary_or_middle_click_on_the_list_keeps_the_list_and_the_keyboard() {
    for button in [egui::PointerButton::Secondary, egui::PointerButton::Middle] {
        for on_a_row in [true, false] {
            let (mut harness, tab) = editor();
            type_text(&mut harness, "se");
            let tree = harness.settle();
            let at = if on_a_row {
                bounds_of(&tree, "set, keyword").center()
            } else {
                footer_point(&tree)
            };
            click_at(&mut harness, at, button);
            let case = format!("{button:?}, on a row: {on_a_row}");
            // Only a primary click picks a row.
            assert_eq!(sql(&harness, tab).text, "se", "{case}");
            assert_eq!(labels(&harness, tab), ["select", "set"], "{case}");
            assert!(editor_has_keyboard(&harness, tab), "{case}");
            // And the list's keys still work.
            harness.press(Key::Tab, Modifiers::NONE);
            assert_eq!(sql(&harness, tab).text, "select", "{case}");
        }
    }
}

#[test]
fn escape_while_the_list_is_pressed_closes_it_and_the_editor_keeps_the_keyboard() {
    let (mut harness, tab) = editor();
    type_text(&mut harness, "se");
    let tree = harness.settle();
    let at = footer_point(&tree);
    harness.frame(vec![egui::Event::PointerMoved(at)]);
    harness.frame(vec![egui::Event::PointerButton {
        pos: at,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::NONE,
    }]);
    // The button stays down. The editor never lost the keyboard, so it is
    // not handed it again: that would let go of Esc for a frame.
    harness.settle();
    harness.frame(vec![key(Key::Escape, Modifiers::NONE)]);
    harness.frame(vec![release(Key::Escape, Modifiers::NONE)]);
    harness.settle();
    assert!(list(&harness, tab).is_none());
    assert!(editor_has_keyboard(&harness, tab));
}

#[test]
fn the_footer_names_the_keys_and_counts_the_rows_out_of_sight() {
    let (mut harness, tab) = editor();
    harness.press(Key::Space, ctrl());
    let open = list(&harness, tab).expect("a list");
    let hidden = open.candidates.len() + open.more - crate::ui::sql_complete::VISIBLE;
    assert!(hidden > 0);
    harness.settle();
    assert_eq!(painted(&harness, &format!("{hidden} more")), 1);
    assert_eq!(painted(&harness, "insert"), 1);
    // Five rows show, and no sixth.
    let tree = harness.settle();
    let rows = tree
        .nodes
        .iter()
        .filter(|(_, node)| {
            node.label()
                .is_some_and(|label| label.ends_with(", keyword"))
        })
        .count();
    assert_eq!(rows, crate::ui::sql_complete::VISIBLE);
}

#[test]
fn the_terminal_look_names_its_keys_in_the_list_and_the_mode_line() {
    let (mut harness, _tab) = editor();
    harness.set_look(crate::theme::Look::omarchy());
    harness.settle();
    assert_eq!(painted(&harness, "complete"), 0);
    type_text(&mut harness, "se");
    harness.settle();
    assert_eq!(painted(&harness, "ctrl+n/p"), 1);
    // The list's footer and the mode line's `tab complete`.
    assert_eq!(painted(&harness, "complete"), 2);
}

#[test]
fn the_terminal_footer_counts_the_rows_out_of_sight() {
    let (mut harness, tab) = editor();
    harness.set_look(crate::theme::Look::omarchy());
    harness.press(Key::Space, ctrl());
    let open = list(&harness, tab).expect("a list");
    let hidden = open.candidates.len() + open.more - crate::ui::sql_complete::VISIBLE;
    assert!(hidden > 0);
    harness.settle();
    // After the keys, in the same line.
    assert_eq!(painted(&harness, &format!("move · {hidden} more")), 1);
    // A list that shows all it has counts nothing.
    let (mut harness, _tab) = editor();
    harness.set_look(crate::theme::Look::omarchy());
    type_text(&mut harness, "se");
    harness.settle();
    assert_eq!(painted(&harness, "ctrl+n/p"), 1);
    assert_eq!(painted(&harness, "more"), 0);
}

#[test]
fn the_mode_line_stops_naming_tab_once_a_row_is_accepted() {
    let (mut harness, tab) = editor();
    harness.set_look(crate::theme::Look::omarchy());
    type_text(&mut harness, "se");
    harness.settle();
    assert_eq!(painted(&harness, "complete"), 2);
    let (tab_id, sql_tab) = ids(&harness, tab);
    harness.app.apply(Action::AcceptCompletion {
        tab: tab_id,
        sql_tab,
        row: None,
    });
    // The frame between the accept and the insertion: the mode line is
    // drawn before the editor puts the row in, and the list is no longer
    // one to complete from.
    harness.frame(Vec::new());
    assert_eq!(painted(&harness, "complete"), 0);
    assert_eq!(sql(&harness, tab).text, "select");
}

/// Makes the open list one that waits for its names: no row yet.
fn wait_for_rows(harness: &mut Harness, tab: ConnTabId) {
    let (_, id) = ids(harness, tab);
    let workspace = harness.app.workspace_mut(tab).unwrap();
    let open = workspace.sql_tab_mut(id).unwrap().completion.as_mut();
    let open = open.expect("a list");
    open.candidates = std::sync::Arc::new(Vec::new());
    open.loading = true;
}

#[test]
fn a_list_with_no_rows_names_no_keys() {
    // macOS: what it waits for, and no key to insert with.
    let (mut harness, tab) = editor();
    type_text(&mut harness, "se");
    harness.settle();
    assert_eq!(painted(&harness, "insert"), 1);
    wait_for_rows(&mut harness, tab);
    let tree = harness.settle();
    assert!(named(&tree, "Completions").is_some());
    assert_eq!(painted(&harness, "Loading"), 1);
    assert_eq!(painted(&harness, "insert"), 0);

    // The terminal: neither in its footer nor in the mode line.
    let (mut harness, tab) = editor();
    harness.set_look(crate::theme::Look::omarchy());
    type_text(&mut harness, "se");
    wait_for_rows(&mut harness, tab);
    let tree = harness.settle();
    assert!(named(&tree, "Completions").is_some());
    assert_eq!(painted(&harness, "loading"), 1);
    assert_eq!(painted(&harness, "complete"), 0);
    assert_eq!(painted(&harness, "ctrl+n/p"), 0);
}

/// Replaces the open list's rows with `rows`.
fn set_rows(harness: &mut Harness, tab: ConnTabId, rows: Vec<Candidate>) {
    let (_, id) = ids(harness, tab);
    let workspace = harness.app.workspace_mut(tab).unwrap();
    let open = workspace.sql_tab_mut(id).unwrap().completion.as_mut();
    open.expect("a list").candidates = std::sync::Arc::new(rows);
}

#[test]
fn a_long_name_and_a_long_type_are_cut_with_an_ellipsis() {
    use crate::theme::Look;
    for look in [Look::standard(), Look::omarchy()] {
        let (mut harness, tab) = editor();
        harness.set_look(look);
        type_text(&mut harness, "se");
        let name = format!("se{}", "_and_a_very_long_name".repeat(12));
        let kind = "character varying(255) ".repeat(8);
        let long = Candidate {
            kind: Kind::Column,
            label: name.clone(),
            insert: name.clone(),
            matched: 0..2,
            detail: kind.clone(),
        };
        set_rows(&mut harness, tab, vec![long]);
        let tree = harness.settle();
        // What is painted of `whole`: its start, then "…".
        let cut = |whole: &str| {
            harness.painted.iter().any(|(text, _)| {
                text.strip_suffix('…')
                    .is_some_and(|start| !start.is_empty() && whole.starts_with(start))
            })
        };
        assert!(cut(&name), "the name, terminal: {}", look.terminal);
        assert!(cut(&kind), "the type, terminal: {}", look.terminal);
        // A screen reader gets both whole.
        let whole = format!("{name}, column, {kind}");
        assert!(row(&tree, &whole).is_some());
    }
}

#[test]
fn losing_the_keyboard_closes_the_list() {
    let (mut harness, tab) = editor();
    type_text(&mut harness, "se");
    // Quick open takes the keyboard.
    harness.press(Key::P, Modifiers::COMMAND);
    assert!(harness.app.dialog.is_some());
    assert!(list(&harness, tab).is_none());
}

#[test]
fn escape_in_the_frame_after_the_list_opens_closes_it() {
    let (mut harness, tab) = editor();
    // No frame between the two: the list opened at the end of the first
    // and does not hold Esc yet, so Esc takes the keyboard from the editor.
    harness.frame(vec![egui::Event::Text("se".into())]);
    harness.frame(vec![key(Key::Escape, Modifiers::NONE)]);
    harness.frame(vec![release(Key::Escape, Modifiers::NONE)]);
    harness.settle();
    assert!(list(&harness, tab).is_none());
    assert_eq!(sql(&harness, tab).text, "se");
}

#[test]
fn scrolling_the_word_out_of_the_pane_closes_the_list() {
    let (mut harness, tab) = editor();
    paste(&mut harness, &"select 1;\n".repeat(200));
    type_text(&mut harness, "se");
    // The editor scrolls to its caret, and the list shows under its word.
    let tree = harness.finish_animations();
    assert!(named(&tree, "Completions").is_some());
    // The wheel over the editor, far enough for the last line to leave.
    wheel_up(&mut harness, 20.0);
    assert!(list(&harness, tab).is_none());
    assert!(editor_has_keyboard(&harness, tab));
}

/// Turns the wheel over the editor: the script moves down by `lines` a
/// frame, for ten frames, and then comes to rest (egui smooths the wheel
/// over the frames that follow).
fn wheel_up(harness: &mut Harness, lines: f32) {
    let tree = harness.settle();
    // Beside the gutter, which stays put: the field itself is as tall as
    // the script and scrolls. Near the top, clear of a list at the caret.
    let gutter = bounds_of(&tree, "Line numbers");
    let over = egui::pos2(gutter.right() + 40.0, gutter.top() + 10.0);
    harness.frame(vec![egui::Event::PointerMoved(over)]);
    for _ in 0..10 {
        harness.frame(vec![egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Line,
            delta: egui::vec2(0.0, lines),
            phase: egui::TouchPhase::Move,
            modifiers: Modifiers::NONE,
        }]);
    }
    harness.finish_animations();
}

#[test]
fn scrolling_the_word_out_of_the_pane_sideways_closes_the_list() {
    let (mut harness, tab) = editor();
    // A first line to type on, over one far wider than the pane.
    paste(&mut harness, &format!("\n{}", "1, ".repeat(400)));
    harness.finish_animations();
    harness.press(Key::ArrowUp, Modifiers::COMMAND);
    harness.finish_animations();
    assert_eq!(sql(&harness, tab).cursor, 0);
    type_text(&mut harness, "se");
    let tree = harness.finish_animations();
    assert!(named(&tree, "Completions").is_some());
    // The wheel, sideways, until the whole word has left on the left: the
    // list is not kept at the pane's edge for a word nobody sees.
    let gutter = bounds_of(&tree, "Line numbers");
    let over = egui::pos2(gutter.right() + 400.0, gutter.bottom() - 10.0);
    harness.frame(vec![egui::Event::PointerMoved(over)]);
    for _ in 0..10 {
        harness.frame(vec![egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Line,
            delta: egui::vec2(-20.0, 0.0),
            phase: egui::TouchPhase::Move,
            modifiers: Modifiers::NONE,
        }]);
    }
    harness.finish_animations();
    assert!(scrolled(&harness, tab).x > 0.0);
    assert!(list(&harness, tab).is_none());
    assert!(editor_has_keyboard(&harness, tab));
}

#[test]
fn a_list_opened_while_the_editor_scrolls_to_its_caret_waits_for_it() {
    let (mut harness, tab) = editor();
    // The paste puts the caret far below the pane: the editor is still
    // scrolling to it when the word is typed.
    harness.frame(vec![egui::Event::Paste("select 1;\n".repeat(200))]);
    harness.frame(vec![egui::Event::Text("se".into())]);
    let tree = harness.settle();
    assert!(named(&tree, "Completions").is_none(), "not in the pane yet");
    assert_eq!(labels(&harness, tab), ["select", "set"]);
    // It shows once its word is in the pane.
    let tree = harness.finish_animations();
    assert!(named(&tree, "Completions").is_some());
    assert_eq!(labels(&harness, tab), ["select", "set"]);
}

#[test]
fn a_list_asked_for_with_the_caret_out_of_the_pane_brings_it_into_view() {
    let (mut harness, tab) = editor();
    paste(&mut harness, &"select 1;\n".repeat(200));
    harness.finish_animations();
    let (tab_id, id) = ids(&harness, tab);
    let scrolled = |harness: &Harness| {
        crate::ui::sql_text::scroll_offset(&harness.ctx, tab_id, id)
            .expect("a scroll area")
            .y
    };
    let at_the_caret = scrolled(&harness);
    wheel_up(&mut harness, 20.0);
    let away = scrolled(&harness);
    assert!(away < at_the_caret / 2.0, "{away} of {at_the_caret}");
    // Asked for by hand: the editor scrolls back to its caret, and the
    // list shows there.
    harness.press(Key::I, Modifiers::COMMAND);
    let tree = harness.finish_animations();
    assert!(list(&harness, tab).is_some_and(|list| list.manual));
    assert!(named(&tree, "Completions").is_some());
    let back = scrolled(&harness);
    assert!(back > at_the_caret / 2.0, "{back} of {at_the_caret}");
}

#[test]
fn mod_i_opens_the_list_by_hand() {
    // As the harness presses Mod, then as macOS reports Cmd and as the
    // others report Ctrl.
    for held in [
        Modifiers::COMMAND,
        Modifiers::MAC_CMD | Modifiers::COMMAND,
        Modifiers::CTRL | Modifiers::COMMAND,
    ] {
        let (mut harness, tab) = editor();
        harness.press(Key::I, held);
        let open = list(&harness, tab).unwrap_or_else(|| panic!("no list for {held:?}"));
        assert!(open.manual, "{held:?}");
        assert_eq!(open.candidates[0].label, "ALL", "{held:?}");
        // The editor never saw the key.
        assert_eq!(sql(&harness, tab).text, "", "{held:?}");
        assert!(editor_has_keyboard(&harness, tab), "{held:?}");
    }
}

#[test]
fn mod_i_with_shift_or_alt_or_without_mod_opens_nothing() {
    for held in [
        Modifiers::COMMAND | Modifiers::SHIFT,
        Modifiers::COMMAND | Modifiers::ALT,
        Modifiers::CTRL | Modifiers::COMMAND | Modifiers::SHIFT,
        Modifiers::MAC_CMD | Modifiers::COMMAND | Modifiers::ALT,
        // Ctrl on macOS is not Mod there.
        Modifiers::CTRL,
    ] {
        let (mut harness, tab) = editor();
        harness.press(Key::I, held);
        assert!(list(&harness, tab).is_none(), "{held:?}");
        assert!(sql(&harness, tab).completion_wanted.is_none(), "{held:?}");
        assert_eq!(sql(&harness, tab).text, "", "{held:?}");
    }
}

#[test]
fn mod_i_opens_nothing_while_the_keyboard_is_elsewhere() {
    let (mut harness, tab) = editor();
    drop_keyboard(&mut harness);
    assert!(!editor_has_keyboard(&harness, tab));
    harness.press(Key::I, Modifiers::COMMAND);
    assert!(list(&harness, tab).is_none());
    assert!(sql(&harness, tab).completion_wanted.is_none());
    assert_eq!(sql(&harness, tab).text, "");
}

/// How far the editor is scrolled.
fn scrolled(harness: &Harness, tab: ConnTabId) -> egui::Vec2 {
    let (tab, id) = ids(harness, tab);
    crate::ui::sql_text::scroll_offset(&harness.ctx, tab, id).expect("a scroll area")
}

/// Runs frames until the open list is drawn, or closed. One that is open
/// and not drawn waits for the editor to scroll its caret into view: it
/// may wait only while the editor scrolls.
fn frames_until_the_list_shows(
    harness: &mut Harness,
    tab: ConnTabId,
) -> egui::accesskit::TreeUpdate {
    let mut still = 0;
    for _ in 0..200 {
        let before = scrolled(harness, tab);
        let tree = harness.frame(Vec::new());
        if list(harness, tab).is_none() || named(&tree, "Completions").is_some() {
            return tree;
        }
        if scrolled(harness, tab) == before {
            still += 1;
        }
        // egui starts to scroll the frame after it is asked to.
        assert!(
            still <= 2,
            "the list is open, not drawn, and nothing scrolls"
        );
    }
    panic!("the list neither showed nor closed");
}

#[test]
fn a_list_waits_undrawn_only_while_the_editor_scrolls_to_its_caret() {
    // Typed while the editor is on its way to a caret far below.
    let (mut harness, tab) = editor();
    harness.frame(vec![egui::Event::Paste("select 1;\n".repeat(200))]);
    harness.frame(vec![egui::Event::Text("se".into())]);
    let tree = frames_until_the_list_shows(&mut harness, tab);
    assert!(named(&tree, "Completions").is_some());
    assert_eq!(labels(&harness, tab), ["select", "set"]);

    // Asked for by hand after scrolling away from the caret.
    let (mut harness, tab) = editor();
    paste(&mut harness, &"select 1;\n".repeat(200));
    harness.finish_animations();
    wheel_up(&mut harness, 20.0);
    harness.frame(vec![key(Key::I, Modifiers::COMMAND)]);
    harness.frame(vec![release(Key::I, Modifiers::COMMAND)]);
    let tree = frames_until_the_list_shows(&mut harness, tab);
    assert!(named(&tree, "Completions").is_some());
    assert!(list(&harness, tab).is_some_and(|list| list.manual));
}

#[test]
fn a_list_whose_word_starts_left_of_the_pane_shows_at_the_panes_edge() {
    let (mut harness, tab) = editor();
    // A short line over one wider than the pane: with the caret at the
    // end of the long one, the view is scrolled to the right.
    let short = "select 1 from t where se";
    paste(&mut harness, &format!("{short}\n{}", "1, ".repeat(100)));
    harness.finish_animations();
    // Up: the caret lands at the end of the short line, and the editor
    // scrolls back just far enough to show it. The word before it starts
    // left of the pane.
    harness.press(Key::ArrowUp, Modifiers::NONE);
    harness.finish_animations();
    assert_eq!(sql(&harness, tab).cursor, short.len());
    assert!(scrolled(&harness, tab).x > 0.0);
    harness.frame(vec![egui::Event::Text("l".into())]);
    let tree = frames_until_the_list_shows(&mut harness, tab);
    assert_eq!(labels(&harness, tab), ["select"]);
    // Drawn, and inside the text's pane across: not over the gutter.
    let whole = bounds_of(&tree, "Completions");
    let gutter = bounds_of(&tree, "Line numbers");
    assert!(whole.left() >= gutter.right(), "{whole:?} and {gutter:?}");
    // Its keys act on a list the user sees.
    harness.press(Key::Enter, Modifiers::NONE);
    assert!(
        sql(&harness, tab)
            .text
            .starts_with("select 1 from t where select\n")
    );
}

#[test]
fn a_list_with_no_place_closes_once_the_caret_is_in_the_pane() {
    let (mut harness, tab) = editor();
    // The caret at the end, in the pane; the first line far above it.
    paste(&mut harness, &format!("se\n{}", "select 1;\n".repeat(200)));
    harness.finish_animations();
    // A list on the word of the first line, as no refresh would leave it:
    // worked out from this script and this cursor, so it is not worked out
    // again, yet hanging under a word that is out of the pane.
    let (_, id) = ids(&harness, tab);
    let workspace = harness.app.workspace_mut(tab).unwrap();
    let dialect = workspace.driver.dialect();
    let generation = workspace.catalog_generation;
    let sql_tab = workspace.sql_tab_mut(id).unwrap();
    let tokens = tabletist_db::sql::tokenize(dialect, &sql_tab.text);
    let site = tabletist_db::complete::site(&tokens, &sql_tab.text, 2).expect("a site");
    // Keywords only: the first word of a statement reads no names.
    let tree = crate::model::Tree::default();
    let catalog = crate::completion::Catalog {
        dialect,
        tree: &tree,
        bare: None,
        schemas: &[],
    };
    let listed = crate::completion::list(&site, "se", false, &catalog);
    assert!(!listed.candidates.is_empty());
    let of = (TextPrint::of(&sql_tab.text), sql_tab.cursor, generation);
    let open = Completion::new(false, of, site, "se".into(), listed, false);
    sql_tab.completion = Some(open);
    let before = sql_tab.text.clone();
    // Nothing is left to scroll to: the list has no place and goes, and
    // its keys with it.
    let tree = frames_until_the_list_shows(&mut harness, tab);
    assert!(named(&tree, "Completions").is_none());
    harness.settle();
    assert!(list(&harness, tab).is_none());
    assert!(editor_has_keyboard(&harness, tab));
    harness.press(Key::Tab, Modifiers::NONE);
    assert_eq!(sql(&harness, tab).text, format!("{before}\t"));
}

#[test]
fn a_list_asked_for_on_another_word_while_one_is_open_is_a_new_list() {
    let (mut harness, tab) = editor();
    paste(&mut harness, &"select 1;\n".repeat(200));
    type_text(&mut harness, "se");
    let tree = harness.finish_animations();
    assert!(named(&tree, "Completions").is_some());
    let serial = list(&harness, tab).expect("a list").serial;
    // In one frame the cursor goes to the start of the script, far above
    // the pane, and a list is asked for there: it takes the place of the
    // open one with no frame between them.
    let chord = [
        (Key::ArrowUp, Modifiers::COMMAND),
        (Key::I, Modifiers::COMMAND),
    ];
    harness.frame(chord.map(|(pressed, held)| key(pressed, held)).into());
    harness.frame(chord.map(|(pressed, held)| release(pressed, held)).into());
    let open = list(&harness, tab).expect("a list");
    assert_ne!(open.serial, serial);
    // It has not drawn yet, whatever the one before it did: it waits for
    // the editor to scroll to its caret, and shows there.
    let tree = frames_until_the_list_shows(&mut harness, tab);
    assert!(named(&tree, "Completions").is_some());
    let open = list(&harness, tab).expect("a list");
    assert!(open.manual);
    assert_eq!(sql(&harness, tab).cursor, 0);
    assert_eq!((open.typed.as_str(), open.site.word.clone()), ("", 0..0));
}

#[test]
fn the_first_list_of_another_editor_is_drawn_at_once() {
    let (mut harness, _tab) = editor();
    type_text(&mut harness, "se");
    harness.settle();
    assert_eq!(painted(&harness, "insert"), 1);
    harness.press(Key::Escape, Modifiers::NONE);
    assert_eq!(painted(&harness, "insert"), 0);
    // A second editor. The frame after its word is typed paints its list:
    // egui sizes a new area in a pass that paints nothing, and the lists
    // of all editors share one area.
    harness.press(Key::T, Modifiers::COMMAND);
    harness.frame(vec![egui::Event::Text("se".into())]);
    harness.frame(Vec::new());
    assert_eq!(painted(&harness, "insert"), 1);
}

#[test]
fn the_rows_in_view_follow_the_highlight_down_the_list() {
    use crate::ui::sql_complete::{VISIBLE, row_name};
    let (mut harness, tab) = editor();
    harness.press(Key::Space, ctrl());
    let open = list(&harness, tab).expect("a list");
    assert!(open.candidates.len() > VISIBLE + 1);
    let locale = harness.app.locale;
    let names: Vec<String> = open
        .candidates
        .iter()
        .map(|candidate| row_name(candidate, locale))
        .collect();
    let in_view = |tree: &egui::accesskit::TreeUpdate| -> Vec<usize> {
        let shown = |index: &usize| row(tree, &names[*index]).is_some();
        (0..names.len()).filter(shown).collect()
    };
    let tree = harness.settle();
    assert_eq!(in_view(&tree), (0..VISIBLE).collect::<Vec<_>>());
    // Down past the last row in view: the sixth candidate comes in and
    // the first leaves.
    for _ in 0..VISIBLE {
        harness.press(Key::ArrowDown, Modifiers::NONE);
    }
    assert_eq!(selected(&harness, tab), Some(VISIBLE));
    let tree = harness.settle();
    assert_eq!(in_view(&tree), (1..=VISIBLE).collect::<Vec<_>>());
    let last = node_of(&tree, row(&tree, &names[VISIBLE]).unwrap());
    assert_eq!(last.is_selected(), Some(true));
    assert_eq!(last.position_in_set(), Some(VISIBLE));
    // Back up inside the view: the rows stay where they are.
    harness.press(Key::ArrowUp, Modifiers::NONE);
    let tree = harness.settle();
    assert_eq!(in_view(&tree), (1..=VISIBLE).collect::<Vec<_>>());
    // Up to the first row again: the view follows.
    for _ in 0..VISIBLE {
        harness.press(Key::ArrowUp, Modifiers::NONE);
    }
    assert_eq!(selected(&harness, tab), Some(0));
    let tree = harness.settle();
    assert_eq!(in_view(&tree), (0..VISIBLE).collect::<Vec<_>>());
}

#[test]
fn a_closed_editor_leaves_nothing_of_its_list_in_eguis_memory() {
    use crate::ui::sql_text::remembered;
    let (mut harness, tab) = editor();
    type_text(&mut harness, "se");
    harness.settle();
    let (tab_id, id) = ids(&harness, tab);
    assert!(remembered(&harness.ctx, tab_id, id).contains(&"list"));
    // The editor goes with its list open: it is not drawn again, so only
    // forgetting the editor drops what the list kept.
    harness.press(Key::W, Modifiers::COMMAND);
    assert!(harness.app.workspace(tab).unwrap().sql_tab(id).is_none());
    assert!(remembered(&harness.ctx, tab_id, id).is_empty());

    // A list that closes drops it at once.
    let (mut harness, tab) = editor();
    type_text(&mut harness, "se");
    harness.settle();
    let (tab_id, id) = ids(&harness, tab);
    harness.press(Key::Escape, Modifiers::NONE);
    assert!(!remembered(&harness.ctx, tab_id, id).contains(&"list"));
}

#[test]
fn a_list_that_shrinks_is_drawn_at_once_where_it_stays() {
    // Where the list is: with five rows, in the first frame after one
    // typed word left it a single row, and once that has settled.
    let run = |height: f32, lines: usize| {
        let mut harness = Harness::with_size(egui::vec2(1280.0, height));
        harness.connect_fake();
        harness.press(Key::T, Modifiers::COMMAND);
        paste(&mut harness, &"\n".repeat(lines));
        harness.finish_animations();
        harness.press(Key::Space, ctrl());
        let tree = harness.settle();
        let place = |tree: &egui::accesskit::TreeUpdate| {
            let (_, node) = named(tree, "Completions")?;
            let rect = node.bounds()?;
            Some((rect.y0, rect.y1))
        };
        let long = place(&tree)?;
        harness.frame(vec![egui::Event::Text("sel".into())]);
        let first = place(&harness.frame(Vec::new()))?;
        let settled = place(&harness.settle())?;
        Some((long, first, settled))
    };
    // A window where the long list has no room under its word and goes
    // above it, and the short one fits under: there the short list is
    // wholly below where the long one was.
    let sizes = (0..16).map(|step| 360.0 + 20.0 * step as f32);
    let found = sizes
        .flat_map(|height| (1..16).map(move |lines| (height, lines)))
        .filter_map(|(height, lines)| run(height, lines))
        .find(|(long, _, settled)| settled.0 > long.1);
    let (_, first, settled) = found.expect("a window where only the short list fits below");
    // egui would keep an area inside the screen by the size it had a frame
    // ago, and so draw the short list over its word's line for a frame.
    assert_eq!(first, settled);
}

#[test]
fn format_leaves_no_list_of_the_script_as_it_was() {
    const COMMAND_SHIFT: Modifiers = Modifiers::COMMAND.plus(Modifiers::SHIFT);
    // An editor with a list open on `fr`, typed after `script`.
    let completing = |script: &str| {
        let (mut harness, tab) = editor();
        paste(&mut harness, script);
        type_text(&mut harness, "fr");
        assert_eq!(labels(&harness, tab), ["from"]);
        (harness, tab)
    };
    // An open list is of the script on screen, once a frame is over.
    let no_stale_list = |harness: &Harness, tab: ConnTabId| {
        let sql = sql(harness, tab);
        let open = sql.completion.as_ref();
        assert!(open.is_none_or(|list| list.is_of_text(&sql.text)));
    };

    // Format leaves the word where it was: the list goes on, worked out
    // again from the formatted script, and its row goes into that one.
    let (mut harness, tab) = completing("select 1 ");
    harness.frame(vec![key(Key::F, COMMAND_SHIFT)]);
    // The editor formats the script when it is drawn next.
    harness.frame(vec![release(Key::F, COMMAND_SHIFT)]);
    assert_eq!(sql(&harness, tab).text, "SELECT 1 fr");
    no_stale_list(&harness, tab);
    harness.settle();
    assert!(list(&harness, tab).is_some_and(|list| list.is_of_text("SELECT 1 fr")));
    assert_eq!(labels(&harness, tab), ["from"]);
    assert!(editor_has_keyboard(&harness, tab));
    harness.press(Key::Tab, Modifiers::NONE);
    assert_eq!(sql(&harness, tab).text, "SELECT 1 from");

    // Format moves the word: the list closes.
    let (mut harness, tab) = completing("select a,b ");
    harness.frame(vec![key(Key::F, COMMAND_SHIFT)]);
    harness.frame(vec![release(Key::F, COMMAND_SHIFT)]);
    assert_eq!(sql(&harness, tab).text, "SELECT a,\n       b fr");
    no_stale_list(&harness, tab);
    harness.settle();
    assert!(list(&harness, tab).is_none());
    assert!(editor_has_keyboard(&harness, tab));

    // Tab in the frame that formats: its keys are read before the editor
    // is drawn, from the list of the script as typed. That row is not put
    // into the formatted script.
    let (mut harness, tab) = completing("select 1 ");
    harness.frame(vec![key(Key::F, COMMAND_SHIFT)]);
    harness.frame(vec![
        release(Key::F, COMMAND_SHIFT),
        key(Key::Tab, Modifiers::NONE),
    ]);
    harness.frame(vec![release(Key::Tab, Modifiers::NONE)]);
    harness.settle();
    assert_eq!(sql(&harness, tab).text, "SELECT 1 fr");
    assert!(list(&harness, tab).is_none());
}

fn table(name: &str) -> ObjectInfo {
    ObjectInfo {
        name: name.to_owned(),
        kind: ObjectKind::Table,
        estimated_rows: None,
    }
}

/// How many times the backend was asked for the tables and views of the
/// schema `of`.
fn asked_for_objects(harness: &Harness, of: &str) -> usize {
    let sent = harness.app.backend.sent.iter();
    sent.filter(|command| matches!(command, Command::ListObjects { schema, .. } if schema == of))
        .count()
}

/// Answers the newest `ListObjects`.
fn answer_objects(harness: &mut Harness, result: Result<Vec<ObjectInfo>, tabletist_db::Error>) {
    let sent = harness.app.backend.sent.iter().rev();
    let (session, request, schema) = sent
        .filter_map(|command| match command {
            Command::ListObjects {
                session,
                request,
                schema,
            } => Some((*session, *request, schema.clone())),
            _ => None,
        })
        .next()
        .expect("a ListObjects was sent");
    harness.app.apply(Action::Backend(Event::Objects {
        session,
        request,
        schema,
        result,
    }));
    harness.settle();
}

/// Answers the newest `ListSchemas` with `schemas`.
fn answer_schemas(harness: &mut Harness, schemas: &[&str]) {
    let sent = harness.app.backend.sent.iter().rev();
    let (session, request) = sent
        .filter_map(|command| match command {
            Command::ListSchemas { session, request } => Some((*session, *request)),
            _ => None,
        })
        .next()
        .expect("a ListSchemas was sent");
    harness.app.apply(Action::Backend(Event::Schemas {
        session,
        request,
        result: Ok(schemas.iter().map(|schema| (*schema).to_owned()).collect()),
    }));
    harness.settle();
}

/// An editor on a workspace that also has a schema `reports`, whose
/// objects were never loaded, with `select * from reports` in it.
fn editor_before_reports() -> (Harness, ConnTabId) {
    let (mut harness, tab) = editor();
    let workspace = harness.app.workspace_mut(tab).unwrap();
    workspace.tree.schemas.value = Some(vec!["main".into(), "reports".into()]);
    paste(&mut harness, "select * from reports");
    (harness, tab)
}

#[test]
fn tables_and_views_are_offered_where_a_table_goes() {
    let (mut harness, tab) = editor();
    paste(&mut harness, "select * from ");
    type_text(&mut harness, "us");
    // What starts with the typed text, then what holds it.
    assert_eq!(labels(&harness, tab), ["users", "active_users"]);
    let kinds: Vec<Kind> = list(&harness, tab)
        .map(|list| list.candidates.iter().map(|c| c.kind).collect())
        .unwrap_or_default();
    assert_eq!(kinds, [Kind::Table, Kind::View]);
    assert!(!list(&harness, tab).unwrap().loading);
    harness.press(Key::Tab, Modifiers::NONE);
    assert_eq!(sql(&harness, tab).text, "select * from users");
    assert_eq!(
        asked_for_objects(&harness, "main"),
        1,
        "loaded at connect, not again"
    );
}

#[test]
fn a_schema_and_then_its_tables() {
    let (mut harness, tab) = editor();
    paste(&mut harness, "select * from ");
    type_text(&mut harness, "ma");
    assert_eq!(labels(&harness, tab), ["main"]);
    harness.press(Key::Tab, Modifiers::NONE);
    assert_eq!(sql(&harness, tab).text, "select * from main");
    // The dot opens the list on the schema's tables.
    type_text(&mut harness, ".");
    assert_eq!(labels(&harness, tab), ["active_users", "orders", "users"]);
    type_text(&mut harness, "or");
    harness.press(Key::Enter, Modifiers::NONE);
    assert_eq!(sql(&harness, tab).text, "select * from main.orders");
    assert_eq!(
        asked_for_objects(&harness, "main"),
        1,
        "loaded at connect, not again"
    );
}

#[test]
fn a_schema_is_loaded_on_demand() {
    let (mut harness, tab) = editor_before_reports();
    assert_eq!(asked_for_objects(&harness, "reports"), 0);
    type_text(&mut harness, ".");
    assert_eq!(asked_for_objects(&harness, "reports"), 1);
    // The list waits, open, with no rows.
    let open = list(&harness, tab).expect("a waiting list");
    assert!(open.candidates.is_empty() && open.loading);
    // And goes on waiting: it is not asked for again while it does.
    harness.settle();
    assert_eq!(asked_for_objects(&harness, "reports"), 1);
    answer_objects(&mut harness, Ok(vec![table("monthly"), table("yearly")]));
    assert_eq!(labels(&harness, tab), ["monthly", "yearly"]);
    assert!(!list(&harness, tab).unwrap().loading);
    // Not asked again, and not unfolded in the sidebar.
    type_text(&mut harness, "m");
    assert_eq!(labels(&harness, tab), ["monthly"]);
    assert_eq!(asked_for_objects(&harness, "reports"), 1);
    let workspace = harness.app.workspace(tab).unwrap();
    assert!(!workspace.tree.nodes["reports"].expanded);
}

#[test]
fn enter_is_a_line_break_while_a_list_waits() {
    let (mut harness, tab) = editor_before_reports();
    type_text(&mut harness, ".");
    assert!(list(&harness, tab).is_some_and(|list| list.candidates.is_empty()));
    harness.press(Key::Enter, Modifiers::NONE);
    assert_eq!(sql(&harness, tab).text, "select * from reports.\n");
    assert!(list(&harness, tab).is_none());
}

#[test]
fn an_answer_that_leaves_the_list_empty_closes_it() {
    let (mut harness, tab) = editor_before_reports();
    type_text(&mut harness, ".");
    assert!(list(&harness, tab).is_some());
    answer_objects(&mut harness, Ok(Vec::new()));
    assert!(list(&harness, tab).is_none());
}

#[test]
fn a_load_that_failed_is_not_asked_again() {
    let (mut harness, tab) = editor_before_reports();
    type_text(&mut harness, ".");
    answer_objects(
        &mut harness,
        Err(tabletist_db::Error::query("permission denied")),
    );
    assert!(list(&harness, tab).is_none());
    type_text(&mut harness, "mo");
    assert_eq!(asked_for_objects(&harness, "reports"), 1);
    assert!(list(&harness, tab).is_none());
}

#[test]
fn nothing_is_asked_of_a_session_that_is_not_connected() {
    let (mut harness, tab) = editor_before_reports();
    harness.app.workspace_mut(tab).unwrap().status =
        crate::model::SessionStatus::Disconnected(tabletist_db::Error::query("gone"));
    type_text(&mut harness, ".");
    assert_eq!(sql(&harness, tab).text, "select * from reports.");
    assert_eq!(asked_for_objects(&harness, "reports"), 0);
    assert!(list(&harness, tab).is_none());
}

#[test]
fn a_whole_word_opens_nothing_and_its_names_are_still_asked_for() {
    let (mut harness, tab) = editor();
    // The bare schema's tables were never loaded.
    let workspace = harness.app.workspace_mut(tab).unwrap();
    workspace.tree.nodes.remove("main");
    paste(&mut harness, "select * from ");
    assert_eq!(asked_for_objects(&harness, "main"), 1, "at connect");
    // The schema's name is all the list has, and it is what is typed.
    type_text(&mut harness, "main");
    assert_eq!(asked_for_objects(&harness, "main"), 2);
    assert!(list(&harness, tab).is_none());
    // Nor does it open when they arrive.
    answer_objects(&mut harness, Ok(vec![table("mainland")]));
    assert!(list(&harness, tab).is_none());
    // They are there for the next list.
    type_text(&mut harness, "l");
    assert_eq!(labels(&harness, tab), ["mainland"]);
    assert_eq!(asked_for_objects(&harness, "main"), 2);
}

#[test]
fn refreshing_the_tree_forgets_what_only_the_list_loaded() {
    let (mut harness, tab) = editor_before_reports();
    type_text(&mut harness, ".");
    answer_objects(&mut harness, Ok(vec![table("monthly")]));
    assert_eq!(labels(&harness, tab), ["monthly"]);
    harness.app.apply(Action::RefreshTree(tab));
    let workspace = harness.app.workspace(tab).unwrap();
    assert!(!workspace.tree.nodes.contains_key("reports"));
    assert!(
        workspace.tree.nodes.contains_key("main"),
        "the sidebar's own stays"
    );
    assert_eq!(asked_for_objects(&harness, "reports"), 1);
    // The open list needs them: they load again, and it waits.
    harness.settle();
    assert_eq!(asked_for_objects(&harness, "reports"), 2);
    let open = list(&harness, tab).expect("a waiting list");
    assert!(open.candidates.is_empty() && open.loading);
}

#[test]
fn refreshing_a_tree_with_nothing_unfolded_reaches_an_open_list() {
    let (mut harness, tab) = editor_before_reports();
    // Nothing is the sidebar's to load again.
    let workspace = harness.app.workspace_mut(tab).unwrap();
    workspace.tree.nodes.get_mut("main").unwrap().expanded = false;
    type_text(&mut harness, ".");
    answer_objects(&mut harness, Ok(vec![table("monthly")]));
    assert_eq!(labels(&harness, tab), ["monthly"]);
    let objects = |harness: &Harness| {
        let sent = harness.app.backend.sent.iter();
        sent.filter(|command| matches!(command, Command::ListObjects { .. }))
            .count()
    };
    let before = objects(&harness);
    harness.app.apply(Action::RefreshTree(tab));
    assert_eq!(objects(&harness), before, "the refresh asks for no tables");
    assert!(harness.app.workspace(tab).unwrap().tree.nodes.is_empty());
    // The list's own are asked for again.
    harness.settle();
    assert_eq!(asked_for_objects(&harness, "reports"), 2);
    assert!(list(&harness, tab).is_some_and(|list| list.loading));
}

#[test]
fn names_asked_for_again_show_on_an_open_list_as_loading() {
    let (mut harness, tab) = editor();
    paste(&mut harness, "select * from ");
    type_text(&mut harness, "us");
    assert_eq!(labels(&harness, tab), ["users", "active_users"]);
    assert!(!list(&harness, tab).unwrap().loading);
    // As the sidebar does for a schema it shows.
    harness.app.load_objects(tab, "main");
    harness.settle();
    // What it has stays while the answer is on its way.
    assert_eq!(labels(&harness, tab), ["users", "active_users"]);
    assert!(list(&harness, tab).unwrap().loading);
    answer_objects(&mut harness, Ok(vec![table("users")]));
    assert_eq!(labels(&harness, tab), ["users"]);
    assert!(!list(&harness, tab).unwrap().loading);
}

#[test]
fn schemas_that_arrive_join_an_open_list() {
    let (mut harness, tab) = editor();
    paste(&mut harness, "select * from ");
    type_text(&mut harness, "ma");
    assert_eq!(labels(&harness, tab), ["main"]);
    harness.app.apply(Action::RefreshTree(tab));
    harness.settle();
    // The bare schema's tables are on their way again, and the list says
    // so over the rows it has.
    assert_eq!(labels(&harness, tab), ["main"]);
    assert!(list(&harness, tab).unwrap().loading);
    answer_schemas(&mut harness, &["main", "mart"]);
    assert_eq!(labels(&harness, tab), ["main", "mart"]);
    assert!(list(&harness, tab).unwrap().loading);
    answer_objects(&mut harness, Ok(vec![table("mail"), table("users")]));
    assert_eq!(labels(&harness, tab), ["mail", "main", "mart"]);
    assert!(!list(&harness, tab).unwrap().loading);
}

#[test]
fn switching_the_database_takes_its_names_off_the_list() {
    let (mut harness, tab) = editor();
    paste(&mut harness, "select * from ");
    type_text(&mut harness, "us");
    assert_eq!(labels(&harness, tab), ["users", "active_users"]);
    harness.app.apply(Action::SwitchDatabase {
        tab,
        database: "other".into(),
    });
    harness.settle();
    // The other database's names are not known yet.
    assert!(list(&harness, tab).is_none());
    assert_eq!(sql(&harness, tab).text, "select * from us");
}

#[test]
fn a_large_schema_is_listed_once_per_change() {
    let (mut harness, tab) = editor();
    let objects: Vec<ObjectInfo> = (0..10_000)
        .map(|index| table(&format!("name_{index:05}")))
        .collect();
    let workspace = harness.app.workspace_mut(tab).unwrap();
    workspace.tree.nodes.get_mut("main").unwrap().objects.value = Some(objects);
    paste(&mut harness, "select * from ");
    type_text(&mut harness, "na");
    let open = list(&harness, tab).expect("a list");
    assert_eq!((open.candidates.len(), open.more), (100, 9_900));
    let listed = LISTED.with(|count| count.get());
    harness.settle();
    harness.press(Key::ArrowDown, Modifiers::NONE);
    assert_eq!(selected(&harness, tab), Some(1));
    assert_eq!(
        LISTED.with(|count| count.get()),
        listed,
        "moving lists nothing"
    );
}
