//! The foreign key picker, checked against its spec
//! (`docs/superpowers/specs/2026-10-10-foreign-key-editing-design.md`:
//! "test N" below is a test of its list, "spec N" a section of it), on
//! the Bookshop's `book_covers` and `publishers`.
//!
//! Only behaviour, names and words are checked. No colour, size or
//! picture of the design is.

use std::time::{Duration, Instant};

use egui::accesskit::Role;
use egui::{Key, Modifiers};
use tabletist_db::{LookupPart, NewValue, ObjectRef, Value};

use crate::backend::{Command, Event, RequestId};
use crate::edit::{Doubt, DoubtState};
use crate::key_picker::{Columns, Picker, Take};
use crate::model::{Action, Advance, CellPos, ConnTabId, EditStart, Pane, TabId};
use crate::testing::{Harness, PUBLISHERS, publishers_structure};
use crate::theme::Look;

const NORTHLIGHT: &str = "9100000000000000001";
const HARBOR_PRESS: &str = "9100000000000000004";
const HARBOR_LIGHTS: &str = "9100000000000000007";
/// `publisher_id`, by its place in the page.
const PUBLISHER: usize = 1;
/// `kind`, the field after it.
const KIND: usize = 2;
/// A pause shorter than any other reason the frame has to come back.
const DELAY: Duration = Duration::from_millis(40);

/// The Bookshop's covers with their key to publishers, writable, the grid
/// holding the keyboard.
fn covers_in(look: Look) -> (Harness, ConnTabId, TabId) {
    let mut harness = Harness::new();
    harness.set_look(look);
    let (tab, id) = harness.keyed_covers();
    harness.app.workspace_mut(tab).unwrap().pane = Pane::Grid;
    harness.settle();
    (harness, tab, id)
}

/// Opens the editor on `publisher_id` of the page's row `row` from
/// `start`, and answers what the picker asks: the publishers' columns,
/// then its lookups.
fn edit(harness: &mut Harness, tab: ConnTabId, id: TabId, row: usize, start: EditStart) {
    open(harness, tab, id, row, start);
    // The newest Describe is the one for the publishers' columns, where
    // one was sent; where none was, this answers one already answered and
    // changes nothing.
    harness.answer_structure(publishers_structure());
    harness.answer_lookups(&PUBLISHERS);
}

/// Opens the editor as [`edit`] does, and answers nothing.
fn open(harness: &mut Harness, tab: ConnTabId, id: TabId, row: usize, start: EditStart) {
    harness.app.apply(Action::EditCell {
        tab,
        id,
        cell: CellPos {
            row,
            col: PUBLISHER,
        },
        start,
    });
}

/// Makes the editor's text `text`, as typing does, and no more.
fn set_text(harness: &mut Harness, tab: ConnTabId, id: TabId, text: &str) {
    let object = harness
        .app
        .workspace_mut(tab)
        .unwrap()
        .object_tab_mut(id)
        .unwrap();
    object.edits.editor.as_mut().unwrap().text = text.to_owned();
    harness.app.apply(Action::EditorTyped { tab, id });
}

fn picker(harness: &Harness, tab: ConnTabId, id: TabId) -> Option<&Picker> {
    let object = harness.app.workspace(tab)?.object_tab(id)?;
    object.edits.editor.as_ref()?.picker.as_ref()
}

/// The keys the picker lists, as text.
fn listed(harness: &Harness, tab: ConnTabId, id: TabId) -> Vec<String> {
    picker(harness, tab, id)
        .unwrap()
        .options
        .iter()
        .map(|option| crate::ui::format::plain_text(&option.key))
        .collect()
}

fn editing(harness: &Harness, tab: ConnTabId, id: TabId) -> bool {
    let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
    object.edits.editor.is_some()
}

/// The requests a `Cancel` was sent for.
fn cancelled(harness: &Harness) -> Vec<RequestId> {
    harness
        .app
        .backend
        .sent
        .iter()
        .filter_map(|command| match command {
            Command::Cancel { request, .. } => Some(*request),
            _ => None,
        })
        .collect()
}

/// Test 10, the part no view is needed for: an editor opened on the
/// cell's value searches for nothing, lists the current row first and
/// highlights it.
#[test]
fn an_editor_on_a_foreign_key_lists_the_table_with_the_current_row_first() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    // Row 1 of the page points at Harbor Press (`keyed_covers_page`).
    edit(&mut harness, tab, id, 1, EditStart::Value);
    let picker = picker(&harness, tab, id).expect("the editor has a picker");
    assert_eq!(picker.target.name, "publishers");
    assert_eq!(picker.key, "id");
    assert!(!picker.searching);
    assert_eq!(
        listed(&harness, tab, id),
        [HARBOR_PRESS, NORTHLIGHT, HARBOR_LIGHTS]
    );
    assert_eq!(picker.highlight, Some(0));
    assert!(picker.is_current(&picker.options[0].key));
    // Two reads: the row the cell points at, by its key as a number, and
    // the table's first rows.
    let sent = harness.lookups();
    assert_eq!(sent.len(), 2);
    assert!(sent[0].2.part == LookupPart::Pinned(Value::Int(9_100_000_000_000_000_004)));
    assert!(sent[1].2.part == LookupPart::Matching(String::new()));
    assert_eq!(sent[1].2.label.as_deref(), Some("name"));
    // On macOS the text is selected, so typing starts a search.
    let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
    assert!(object.edits.editor.as_ref().unwrap().select);
}

/// Spec 2: opened by a typed character, the search is for it from the
/// start.
#[test]
fn a_typed_character_is_searched_for_at_once() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    edit(&mut harness, tab, id, 0, EditStart::Typed("h".into()));
    let picker = picker(&harness, tab, id).unwrap();
    assert!(picker.searching);
    assert_eq!(picker.listed.as_deref(), Some("h"));
    // Northlight, Harbor Press and Harbor Lights all hold an h.
    assert_eq!(listed(&harness, tab, id).len(), 3);
    assert_eq!(picker.highlight, Some(0));
}

/// Test 16: a later answer replaces the list, an answer for an earlier
/// text is dropped, and a search overtaken by more typing is left to end:
/// nothing is cancelled while the editor is open.
#[test]
fn an_answer_for_an_earlier_text_is_dropped() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    edit(&mut harness, tab, id, 0, EditStart::Typed("h".into()));
    let of_h = listed(&harness, tab, id);
    assert_eq!(of_h.len(), 3);
    // Two searches sent, neither answered: `ha` is the second lookup sent,
    // `harb` the third.
    set_text(&mut harness, tab, id, "ha");
    harness.app.send_due_lookups(std::time::Instant::now());
    set_text(&mut harness, tab, id, "harb");
    harness.app.send_due_lookups(std::time::Instant::now());
    assert_eq!(harness.lookups().len(), 3);
    // Once more, so the sweep for lookups no picker waits for has run.
    harness.app.send_due_lookups(std::time::Instant::now());
    assert_eq!(cancelled(&harness), [], "a search was cancelled");
    // Only the earlier text's answer comes: the list is not its.
    harness.answer_lookup_at(1, &PUBLISHERS);
    assert_eq!(
        picker(&harness, tab, id).unwrap().listed.as_deref(),
        Some("h")
    );
    assert_eq!(listed(&harness, tab, id), of_h);
    // The text that shows answers last, and the list is its.
    harness.answer_lookups(&PUBLISHERS);
    assert_eq!(
        picker(&harness, tab, id).unwrap().listed.as_deref(),
        Some("harb")
    );
    assert_eq!(listed(&harness, tab, id), [HARBOR_PRESS, HARBOR_LIGHTS]);
    assert_eq!(cancelled(&harness), []);
}

/// Test 17: a column no one-column key names opens the text editor and
/// no list, and so does the large editor.
#[test]
fn a_column_without_a_key_and_the_large_editor_have_no_picker() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    // `kind` is no foreign key.
    harness.app.apply(Action::EditCell {
        tab,
        id,
        cell: CellPos { row: 0, col: 2 },
        start: EditStart::Value,
    });
    assert!(editing(&harness, tab, id));
    assert!(picker(&harness, tab, id).is_none());
    harness.app.apply(Action::CancelEdit { tab, id });
    // Alt+Enter moves the text to the large editor, which has no list.
    edit(&mut harness, tab, id, 0, EditStart::Typed("h".into()));
    assert!(picker(&harness, tab, id).is_some());
    harness.app.apply(Action::EditorBreak { tab, id });
    assert!(picker(&harness, tab, id).is_none());
}

/// Spec 3 and test 19: the columns of the referenced table that cannot be
/// read, and a search the database refuses, are the list's failure.
#[test]
fn a_search_that_fails_says_so_and_leaves_the_text_editor() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    edit(&mut harness, tab, id, 0, EditStart::Typed("9".into()));
    set_text(&mut harness, tab, id, "91");
    harness.app.send_due_lookups(std::time::Instant::now());
    harness.fail_lookup(tabletist_db::Error::query(
        "permission denied for table publishers",
    ));
    let picker = picker(&harness, tab, id).unwrap();
    assert!(picker.failed.is_some());
    assert_eq!(picker.take("91"), Take::Text);
}

/// Spec 8, the last point: an editor that goes takes its searches with
/// it, the read of the matches only. The pinned row's is one row by its
/// key, and is left to end.
#[test]
fn closing_the_editor_cancels_what_runs() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    open(&mut harness, tab, id, 0, EditStart::Typed("h".into()));
    harness.answer_structure(publishers_structure());
    let running = harness.lookups().last().unwrap().1;
    assert_eq!(cancelled(&harness), []);
    harness.app.apply(Action::CancelEdit { tab, id });
    harness.app.send_due_lookups(std::time::Instant::now());
    assert_eq!(cancelled(&harness), [running]);
    // Its late answer finds no picker and changes nothing.
    harness.answer_lookups(&PUBLISHERS);
    assert!(!editing(&harness, tab, id));

    // Opened on a value that names a row, there are two reads: the pinned
    // row's, then the matches'. Only the second is stopped.
    let (mut harness, tab, id) = covers_in(Look::macos());
    open(&mut harness, tab, id, 1, EditStart::Value);
    harness.answer_structure(publishers_structure());
    let sent = harness.lookups();
    assert_eq!(sent.len(), 2);
    let (pinned, matching) = (sent[0].1, sent[1].1);
    harness.app.apply(Action::CancelEdit { tab, id });
    harness.app.send_due_lookups(std::time::Instant::now());
    let stopped = cancelled(&harness);
    assert!(stopped.contains(&matching));
    assert!(!stopped.contains(&pinned));
}

/// A read the session cancelled that the picker did not ask to be
/// cancelled came back by a late signal: it is sent again, once.
#[test]
fn a_read_cancelled_unasked_is_sent_again() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    open(&mut harness, tab, id, 0, EditStart::Typed("h".into()));
    harness.answer_structure(publishers_structure());
    let before = harness.lookups().len();
    harness.fail_lookup(tabletist_db::Error::Cancelled);
    assert!(
        harness.lookups().len() > before,
        "the search was not sent again"
    );
    let picker = picker(&harness, tab, id).unwrap();
    assert!(picker.failed.is_none());
    assert!(picker.searching);
    harness.answer_lookups(&PUBLISHERS);
    assert_eq!(listed(&harness, tab, id).len(), 3);
    assert!(self::picker(&harness, tab, id).unwrap().failed.is_none());
}

/// When the picker's next search is due.
fn due_of(harness: &Harness, tab: ConnTabId, id: TabId) -> Option<Instant> {
    picker(harness, tab, id)?.due
}

/// Spec 3: a search is sent after a pause in the typing, not on each
/// keystroke. The test reads when the search is due and asks at that
/// time, so it does not depend on the clock.
#[test]
fn a_search_waits_for_a_pause_in_the_typing() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    harness.app.lookup_delay = Duration::from_millis(150);
    edit(&mut harness, tab, id, 0, EditStart::Typed("h".into()));
    set_text(&mut harness, tab, id, "ha");
    let due = due_of(&harness, tab, id).expect("the text waits for its search");
    let sent = harness.lookups().len();
    let wait = harness.app.send_due_lookups(due - Duration::from_millis(1));
    assert_eq!(harness.lookups().len(), sent, "sent before the pause");
    let wait = wait.expect("a search is due later");
    assert!(wait > Duration::ZERO, "{wait:?}");
    assert!(wait <= Duration::from_millis(150), "{wait:?}");
    let wait = harness.app.send_due_lookups(due + Duration::from_millis(1));
    assert_eq!(wait, None);
    assert_eq!(harness.lookups().len(), sent + 1);
}

/// The frame sends what is due itself, and asks to be drawn again when the
/// next search is. The delay is shorter than the 83 ms a focused text field
/// asks for anyway (its cursor), so a request within it is the search's.
#[test]
fn a_frame_sends_what_is_due_and_asks_to_come_back() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    harness.app.lookup_delay = DELAY;
    edit(&mut harness, tab, id, 0, EditStart::Typed("h".into()));
    harness.finish_animations();
    set_text(&mut harness, tab, id, "ha");
    let sent = harness.lookups().len();
    harness.frame(Vec::new());
    if harness.lookups().len() == sent {
        // The frame ran inside the pause: it asked to come back by the time
        // the search is due.
        assert!(
            harness.repaint_after <= DELAY,
            "{:?}",
            harness.repaint_after
        );
    } else {
        // The frame was slower than the pause (a loaded machine): it sent
        // the search itself, which is what the request to come back is for.
        // Either way the frame is wired to the searches.
        assert_eq!(harness.lookups().len(), sent + 1);
        set_text(&mut harness, tab, id, "har");
    }
    // The pause went by, by hand.
    let now = Instant::now();
    let past = now.checked_sub(Duration::from_secs(1)).unwrap_or(now);
    let object = harness
        .app
        .workspace_mut(tab)
        .unwrap()
        .object_tab_mut(id)
        .unwrap();
    object
        .edits
        .editor
        .as_mut()
        .unwrap()
        .picker
        .as_mut()
        .unwrap()
        .due = Some(past);
    let before = harness.lookups().len();
    harness.frame(Vec::new());
    assert_eq!(harness.lookups().len(), before + 1);
}

/// The newest `Describe` sent, answered with `error`.
fn fail_describe(harness: &mut Harness, error: tabletist_db::Error) {
    let (session, request) = harness
        .app
        .backend
        .sent
        .iter()
        .rev()
        .find_map(|command| match command {
            Command::Describe {
                session, request, ..
            } => Some((*session, *request)),
            _ => None,
        })
        .expect("a Describe was sent");
    harness.app.apply(Action::Backend(Event::Structure {
        session,
        request,
        result: Err(error),
    }));
}

/// How many times the columns of `publishers` were asked for.
fn publishers_described(harness: &Harness) -> usize {
    harness
        .app
        .backend
        .sent
        .iter()
        .filter(|command| {
            matches!(command, Command::Describe { object, .. } if object.name == "publishers")
        })
        .count()
}

/// Spec 3: columns that cannot be read leave the text editor, and the next
/// editor on a key to that table asks for them again.
#[test]
fn columns_that_cannot_be_read_are_the_lists_failure_and_are_asked_for_again() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    open(&mut harness, tab, id, 0, EditStart::Typed("h".into()));
    assert_eq!(publishers_described(&harness), 1);
    fail_describe(
        &mut harness,
        tabletist_db::Error::query("permission denied for table publishers"),
    );
    let failed = picker(&harness, tab, id).unwrap();
    assert!(failed.failed.is_some());
    assert!(harness.lookups().is_empty());
    assert_eq!(failed.take("h"), Take::Text);
    harness.app.apply(Action::CancelEdit { tab, id });
    // Opened again, it asks again and waits, with no failure of the old
    // fetch.
    open(&mut harness, tab, id, 0, EditStart::Typed("h".into()));
    assert_eq!(publishers_described(&harness), 2);
    assert!(picker(&harness, tab, id).unwrap().failed.is_none());
    assert!(harness.lookups().is_empty());
    harness.answer_structure(publishers_structure());
    harness.answer_lookups(&PUBLISHERS);
    assert!(picker(&harness, tab, id).unwrap().failed.is_none());
    assert_eq!(listed(&harness, tab, id).len(), 3);
}

/// An editor opened on another cell takes the first one's place: the
/// first picker's read is stopped and the second keeps its own.
#[test]
fn another_cells_editor_takes_over() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    open(&mut harness, tab, id, 0, EditStart::Typed("h".into()));
    harness.answer_structure(publishers_structure());
    open(&mut harness, tab, id, 1, EditStart::Typed("h".into()));
    let sent = harness.lookups();
    assert_eq!(sent.len(), 2);
    let editor = |harness: &Harness| {
        let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
        object.edits.editor.as_ref().unwrap().cell
    };
    assert_eq!(editor(&harness).row, 1);
    harness.app.send_due_lookups(Instant::now());
    assert_eq!(cancelled(&harness), [sent[0].1]);
    // The late answers: the first finds no picker, the second its own.
    harness.answer_lookups(&PUBLISHERS);
    assert_eq!(listed(&harness, tab, id).len(), 3);
    assert_eq!(
        picker(&harness, tab, id).unwrap().listed.as_deref(),
        Some("h")
    );
}

/// A value the page loaded as text is pinned only where it compares
/// exactly, as a typed one is.
#[test]
fn a_loaded_key_is_pinned_only_where_it_compares_exactly() {
    let loaded = |harness: &mut Harness, tab: ConnTabId, id: TabId, value: Value| {
        let object = harness
            .app
            .workspace_mut(tab)
            .unwrap()
            .object_tab_mut(id)
            .unwrap();
        object.rows.value.as_mut().unwrap().rows[0][PUBLISHER] = value;
    };
    // Too wide for an `i64`: the matches' read finds it, `=` is not used.
    let (mut harness, tab, id) = covers_in(Look::macos());
    loaded(
        &mut harness,
        tab,
        id,
        Value::Text("18446744073709551615".into()),
    );
    open(&mut harness, tab, id, 0, EditStart::Value);
    harness.answer_structure(publishers_structure());
    let sent = harness.lookups();
    assert_eq!(sent.len(), 1);
    assert!(sent[0].2.part == LookupPart::Matching(String::new()));
    assert!(picker(&harness, tab, id).unwrap().current.is_none());
    // A whole number that came as text is a number to `=`.
    let (mut harness, tab, id) = covers_in(Look::macos());
    loaded(&mut harness, tab, id, Value::Text(HARBOR_PRESS.into()));
    open(&mut harness, tab, id, 0, EditStart::Value);
    harness.answer_structure(publishers_structure());
    let sent = harness.lookups();
    assert_eq!(sent.len(), 2);
    assert!(sent[0].2.part == LookupPart::Pinned(Value::Int(9_100_000_000_000_000_004)));
}

/// A picker that waits for columns nobody is fetching (the cache was
/// emptied under it) gets them asked for, once.
#[test]
fn a_picker_nobody_fetches_for_has_its_columns_asked_for() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    open(&mut harness, tab, id, 0, EditStart::Typed("h".into()));
    assert_eq!(publishers_described(&harness), 1);
    harness.app.workspace_mut(tab).unwrap().columns.clear();
    harness.app.send_due_lookups(Instant::now());
    assert_eq!(publishers_described(&harness), 2);
    harness.app.send_due_lookups(Instant::now());
    assert_eq!(publishers_described(&harness), 2);
    harness.answer_structure(publishers_structure());
    harness.answer_lookups(&PUBLISHERS);
    assert_eq!(listed(&harness, tab, id).len(), 3);
}

/// Columns that are known stay known when a later refresh of them failed:
/// the list works with them.
#[test]
fn a_failed_refresh_of_known_columns_does_not_take_the_list_away() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    // The first editor makes the publishers' columns known.
    edit(&mut harness, tab, id, 0, EditStart::Typed("h".into()));
    harness.app.apply(Action::CancelEdit { tab, id });
    let described = publishers_described(&harness);
    let sent = harness.lookups().len();
    let cache = &mut harness.app.workspace_mut(tab).unwrap().columns;
    let kept = cache
        .get_mut(&ObjectRef::new("main", "publishers"))
        .expect("the publishers' columns are cached");
    assert!(kept.value.is_some());
    kept.error = Some(tabletist_db::Error::query("the refresh was refused"));
    open(&mut harness, tab, id, 1, EditStart::Typed("h".into()));
    assert_eq!(publishers_described(&harness), described);
    let opened = picker(&harness, tab, id).unwrap();
    assert!(opened.failed.is_none());
    assert!(matches!(opened.label, Columns::Known(Some(_))));
    assert_eq!(harness.lookups().len(), sent + 1);
    harness.answer_lookups(&PUBLISHERS);
    assert!(picker(&harness, tab, id).unwrap().failed.is_none());
    assert_eq!(listed(&harness, tab, id).len(), 3);
}

/// Makes the editor's text `text`, as typing does, and has the search
/// for it sent and answered.
fn type_text(harness: &mut Harness, tab: ConnTabId, id: TabId, text: &str) {
    set_text(harness, tab, id, text);
    harness.app.send_due_lookups(std::time::Instant::now());
    harness.answer_lookups(&PUBLISHERS);
}

/// The pending text of `publisher_id` in the page's row `row`.
fn pending(harness: &Harness, tab: ConnTabId, id: TabId, row: usize) -> Option<String> {
    let object = harness.app.workspace(tab)?.object_tab(id)?;
    match &object.edits.cells.get(&(row, PUBLISHER))?.new {
        NewValue::Text(text) => Some(text.clone()),
        _ => None,
    }
}

fn commit(harness: &mut Harness, tab: ConnTabId, id: TabId, then: Advance) {
    harness.app.apply(Action::CommitEdit { tab, id, then });
}

fn selection(harness: &Harness, tab: ConnTabId, id: TabId) -> (usize, usize) {
    let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
    let cell = object.selection.unwrap();
    (cell.row, cell.col)
}

/// Test 11: the first option of `harb` is highlighted, and Enter makes
/// the cell pending with its key. Tab does the same and moves right.
#[test]
fn enter_takes_the_highlighted_option() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    edit(&mut harness, tab, id, 0, EditStart::Typed("h".into()));
    type_text(&mut harness, tab, id, "harb");
    assert_eq!(listed(&harness, tab, id), [HARBOR_PRESS, HARBOR_LIGHTS]);
    commit(&mut harness, tab, id, Advance::Down);
    assert!(!editing(&harness, tab, id));
    assert_eq!(pending(&harness, tab, id, 0).as_deref(), Some(HARBOR_PRESS));
    assert_eq!(selection(&harness, tab, id), (1, PUBLISHER));
    // What a screen reader is told of the pick.
    let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
    assert!(
        object.edits.picked
            == Some(crate::edit::Picked {
                at: (0, PUBLISHER),
                key: HARBOR_PRESS.to_owned(),
                label: Some("Harbor Press".to_owned()),
            })
    );

    edit(&mut harness, tab, id, 1, EditStart::Typed("l".into()));
    type_text(&mut harness, tab, id, "lights");
    commit(&mut harness, tab, id, Advance::Right);
    assert_eq!(
        pending(&harness, tab, id, 1).as_deref(),
        Some(HARBOR_LIGHTS)
    );
    assert_eq!(selection(&harness, tab, id), (1, PUBLISHER + 1));
}

/// Test 12: the arrows choose, and a click takes its option.
#[test]
fn the_arrows_and_a_click_choose_an_option() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    edit(&mut harness, tab, id, 0, EditStart::Typed("h".into()));
    type_text(&mut harness, tab, id, "harb");
    harness.app.apply(Action::PickerMove { tab, id, by: 1 });
    commit(&mut harness, tab, id, Advance::Down);
    assert_eq!(
        pending(&harness, tab, id, 0).as_deref(),
        Some(HARBOR_LIGHTS)
    );

    // Row 1 points at Harbor Press: Harbor Lights is a change.
    edit(&mut harness, tab, id, 1, EditStart::Typed("h".into()));
    type_text(&mut harness, tab, id, "harb");
    harness.app.apply(Action::PickOption { tab, id, index: 1 });
    assert!(!editing(&harness, tab, id));
    assert_eq!(
        pending(&harness, tab, id, 1).as_deref(),
        Some(HARBOR_LIGHTS)
    );
    // A click moves no selection.
    assert_eq!(selection(&harness, tab, id), (1, PUBLISHER));
}

/// Test 10's other half: an editor opened on its value and left with
/// Enter changes nothing, and an option moved to from there is taken.
#[test]
fn an_editor_that_was_only_opened_takes_nothing() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    edit(&mut harness, tab, id, 1, EditStart::Value);
    commit(&mut harness, tab, id, Advance::Stay);
    assert!(!editing(&harness, tab, id));
    assert_eq!(pending(&harness, tab, id, 1), None);

    edit(&mut harness, tab, id, 1, EditStart::Value);
    // From Harbor Press, the current row, one down: Northlight Books.
    harness.app.apply(Action::PickerMove { tab, id, by: 1 });
    commit(&mut harness, tab, id, Advance::Stay);
    assert_eq!(pending(&harness, tab, id, 1).as_deref(), Some(NORTHLIGHT));
}

/// Test 14: a text that only begins keys highlights nothing, and Enter
/// keeps the text.
#[test]
fn a_text_that_only_begins_keys_is_kept_as_typed() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    edit(&mut harness, tab, id, 0, EditStart::Typed("9".into()));
    type_text(&mut harness, tab, id, "91");
    assert_eq!(listed(&harness, tab, id).len(), 3);
    assert_eq!(picker(&harness, tab, id).unwrap().highlight, None);
    commit(&mut harness, tab, id, Advance::Stay);
    assert_eq!(pending(&harness, tab, id, 0).as_deref(), Some("91"));
    // Nothing was picked: nothing is said.
    let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
    assert!(object.edits.picked.is_none());
}

/// Test 15: an Enter sent before the answer takes the first option of
/// that answer. Typing after the Enter drops it. A full key and Enter is
/// applied by the pinned row's answer alone.
#[test]
fn an_enter_before_the_answer_waits_for_it() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    edit(&mut harness, tab, id, 0, EditStart::Typed("l".into()));
    // The list is for `l` (Northlight, Harbor Lights): `harb` is typed and
    // Enter pressed before anything of it is sent.
    assert_eq!(listed(&harness, tab, id), [NORTHLIGHT, HARBOR_LIGHTS]);
    set_text(&mut harness, tab, id, "harb");
    let before = harness.lookups().len();
    commit(&mut harness, tab, id, Advance::Down);
    assert!(editing(&harness, tab, id), "the commit did not wait");
    // The search went out with the Enter, not 150 ms later.
    assert!(harness.lookups().len() > before);
    harness.answer_lookups(&PUBLISHERS);
    assert!(!editing(&harness, tab, id));
    assert_eq!(pending(&harness, tab, id, 0).as_deref(), Some(HARBOR_PRESS));
    // The kept Down was applied with it.
    assert_eq!(selection(&harness, tab, id), (1, PUBLISHER));

    // Typing after the Enter drops it.
    edit(&mut harness, tab, id, 1, EditStart::Typed("l".into()));
    set_text(&mut harness, tab, id, "harb");
    commit(&mut harness, tab, id, Advance::Down);
    set_text(&mut harness, tab, id, "harbo");
    harness.app.send_due_lookups(std::time::Instant::now());
    harness.answer_lookups(&PUBLISHERS);
    assert!(editing(&harness, tab, id));
    assert_eq!(picker(&harness, tab, id).unwrap().kept, None);
    harness.app.apply(Action::CancelEdit { tab, id });

    // A full key: the pinned row's answer is enough. Row 2 points at
    // Harbor Lights, so Northlight Books is a change.
    edit(&mut harness, tab, id, 2, EditStart::Typed("9".into()));
    set_text(&mut harness, tab, id, NORTHLIGHT);
    commit(&mut harness, tab, id, Advance::Stay);
    let sent = harness.lookups();
    let (session, request, lookup) = sent[sent.len() - 2].clone();
    assert!(matches!(lookup.part, LookupPart::Pinned(_)));
    harness
        .app
        .apply(Action::Backend(crate::backend::Event::Lookup {
            session,
            request,
            result: Ok(crate::testing::fake_lookup(&lookup, &PUBLISHERS)),
        }));
    assert!(!editing(&harness, tab, id));
    assert_eq!(pending(&harness, tab, id, 2).as_deref(), Some(NORTHLIGHT));
}

/// Spec section 8: showing another tab leaves the editor open behind it,
/// and drops an Enter that waited for the answer. The answer is kept for
/// the return, and the next Enter takes.
#[test]
fn a_kept_enter_is_dropped_when_another_tab_is_shown() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    edit(&mut harness, tab, id, 0, EditStart::Typed("l".into()));
    set_text(&mut harness, tab, id, "harb");
    commit(&mut harness, tab, id, Advance::Down);
    assert!(picker(&harness, tab, id).unwrap().kept.is_some());
    harness.app.apply(Action::OpenObject {
        tab,
        object: tabletist_db::ObjectRef::new("main", "publishers"),
        kind: tabletist_db::ObjectKind::Table,
        pin: true,
    });
    assert_ne!(harness.app.active_workspace_tab(), Some((tab, id)));
    harness.answer_lookups(&PUBLISHERS);
    assert!(editing(&harness, tab, id), "the hidden tab's editor closed");
    assert_eq!(pending(&harness, tab, id, 0), None);
    assert_eq!(selection(&harness, tab, id), (0, PUBLISHER));
    assert_eq!(picker(&harness, tab, id).unwrap().kept, None);
    // Back on the tab, the list is the answer's and Enter takes from it.
    harness.app.apply(Action::ActivateTab { tab, id });
    assert_eq!(listed(&harness, tab, id), [HARBOR_PRESS, HARBOR_LIGHTS]);
    commit(&mut harness, tab, id, Advance::Stay);
    assert_eq!(pending(&harness, tab, id, 0).as_deref(), Some(HARBOR_PRESS));
}

/// The same for an Enter whose answer has not come when the user is back:
/// the frame that draws another tab drops it.
#[test]
fn a_frame_on_another_tab_drops_a_kept_enter() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    edit(&mut harness, tab, id, 0, EditStart::Typed("l".into()));
    set_text(&mut harness, tab, id, "harb");
    commit(&mut harness, tab, id, Advance::Down);
    harness.app.apply(Action::OpenObject {
        tab,
        object: tabletist_db::ObjectRef::new("main", "publishers"),
        kind: tabletist_db::ObjectKind::Table,
        pin: true,
    });
    harness.app.send_due_lookups(Instant::now());
    assert_eq!(picker(&harness, tab, id).unwrap().kept, None);
    harness.app.apply(Action::ActivateTab { tab, id });
    harness.answer_lookups(&PUBLISHERS);
    assert!(editing(&harness, tab, id));
    assert_eq!(pending(&harness, tab, id, 0), None);
}

/// Spec 4 and 5: what the column would say of a search's text is not
/// shown while it is a search. A commit that takes the text and is
/// refused shows it, until the text changes.
#[test]
fn a_search_is_no_mistake_until_a_commit_is_refused() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    edit(&mut harness, tab, id, 0, EditStart::Typed("z".into()));
    type_text(&mut harness, tab, id, "zzz");
    // An integer column refuses `zzz`, and nothing matches it.
    let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
    assert!(object.edits.editor.as_ref().unwrap().problem.is_some());
    assert!(!picker(&harness, tab, id).unwrap().refused);
    commit(&mut harness, tab, id, Advance::Down);
    assert!(
        editing(&harness, tab, id),
        "a text the column refuses was kept"
    );
    assert!(picker(&harness, tab, id).unwrap().refused);
    set_text(&mut harness, tab, id, "zz");
    assert!(!picker(&harness, tab, id).unwrap().refused);
}

/// Test 18: Set NULL closes the editor without its text and makes the
/// cell NULL, where the column takes one.
#[test]
fn set_null_on_a_column_that_takes_none_drops_the_text_and_says_why() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    edit(&mut harness, tab, id, 0, EditStart::Typed("harb".into()));
    harness.app.apply(Action::PickerNull { tab, id });
    assert!(!editing(&harness, tab, id));
    // `publisher_id` of the fixture is NOT NULL (its `ColumnInfo` has
    // `nullable: false`): nothing is set, and the cell says why.
    let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
    assert!(object.edits.cells.is_empty());
    assert!(object.edits.why.is_some());
}

/// Test 20: what leaves the editor without a pick keeps the text as
/// typed, never the highlighted option.
#[test]
fn leaving_the_editor_keeps_the_text_and_not_the_option() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    edit(&mut harness, tab, id, 0, EditStart::Typed("9".into()));
    type_text(&mut harness, tab, id, HARBOR_LIGHTS);
    // Harbor Lights is highlighted: its key is the text. Another key is
    // typed over it and the keyboard goes elsewhere before any answer.
    set_text(&mut harness, tab, id, "42");
    let leave = crate::testing::leave_edit(&harness.app, tab, id);
    harness.app.apply(leave);
    assert!(!editing(&harness, tab, id));
    assert_eq!(pending(&harness, tab, id, 0).as_deref(), Some("42"));
}

/// An Enter pressed while the referenced table's columns are being fetched
/// waits for them, and does not hang when that fetch fails: it takes the
/// text.
#[test]
fn an_enter_kept_for_the_columns_is_released_when_they_cannot_be_read() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    open(&mut harness, tab, id, 0, EditStart::Typed("9".into()));
    commit(&mut harness, tab, id, Advance::Stay);
    assert!(editing(&harness, tab, id), "the commit did not wait");
    assert!(picker(&harness, tab, id).unwrap().kept.is_some());
    fail_describe(
        &mut harness,
        tabletist_db::Error::query("permission denied for table publishers"),
    );
    assert!(!editing(&harness, tab, id));
    assert_eq!(pending(&harness, tab, id, 0).as_deref(), Some("9"));
}

/// An Enter kept while the reads of the text that shows run sends nothing
/// more, and the answer applies it.
#[test]
fn an_enter_kept_while_reads_run_sends_nothing_and_the_answer_applies_it() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    open(&mut harness, tab, id, 1, EditStart::Typed("l".into()));
    harness.answer_structure(publishers_structure());
    let sent = harness.lookups().len();
    commit(&mut harness, tab, id, Advance::Stay);
    assert!(editing(&harness, tab, id), "the commit did not wait");
    assert_eq!(harness.lookups().len(), sent, "a second search was sent");
    assert_eq!(picker(&harness, tab, id).unwrap().kept, Some(Advance::Stay));
    harness.answer_lookups(&PUBLISHERS);
    assert!(!editing(&harness, tab, id));
    assert!(pending(&harness, tab, id, 1).as_deref() == Some(NORTHLIGHT));
}

/// A move or a click drops an Enter kept for the list. A click takes its
/// own option instead. A second Enter leaves one kept commit, the second's.
#[test]
fn a_kept_enter_is_dropped_by_a_move_or_a_click_and_replaced_by_another_enter() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    open(&mut harness, tab, id, 0, EditStart::Typed("h".into()));
    harness.answer_structure(publishers_structure());
    commit(&mut harness, tab, id, Advance::Down);
    assert!(picker(&harness, tab, id).unwrap().kept.is_some());
    harness.app.apply(Action::PickerMove { tab, id, by: 1 });
    assert_eq!(picker(&harness, tab, id).unwrap().kept, None);
    harness.answer_lookups(&PUBLISHERS);
    assert!(editing(&harness, tab, id), "a dropped Enter was applied");
    harness.app.apply(Action::CancelEdit { tab, id });

    // A click: the list is the one for `h`, the text is `harb`.
    edit(&mut harness, tab, id, 0, EditStart::Typed("h".into()));
    let second = listed(&harness, tab, id)[1].clone();
    set_text(&mut harness, tab, id, "harb");
    commit(&mut harness, tab, id, Advance::Down);
    assert!(picker(&harness, tab, id).unwrap().kept.is_some());
    harness.app.apply(Action::PickOption { tab, id, index: 1 });
    assert!(!editing(&harness, tab, id));
    assert_eq!(pending(&harness, tab, id, 0), Some(second));
    assert_eq!(selection(&harness, tab, id).0, 0, "the dropped Down moved");
    harness.answer_lookups(&PUBLISHERS);

    // Two Enters: one kept commit, with the second's advance.
    edit(&mut harness, tab, id, 1, EditStart::Typed("h".into()));
    set_text(&mut harness, tab, id, "harb");
    commit(&mut harness, tab, id, Advance::Down);
    let sent = harness.lookups().len();
    commit(&mut harness, tab, id, Advance::Right);
    assert_eq!(harness.lookups().len(), sent);
    assert_eq!(
        picker(&harness, tab, id).unwrap().kept,
        Some(Advance::Right)
    );
    harness.answer_lookups(&PUBLISHERS);
    assert!(!editing(&harness, tab, id));
    assert_eq!(selection(&harness, tab, id), (1, PUBLISHER + 1));
}

/// An Enter kept for a list whose search then fails takes the text: one
/// the column takes is kept as typed, one it refuses leaves the editor open
/// and says why.
#[test]
fn a_kept_enter_takes_the_text_when_the_search_fails() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    open(&mut harness, tab, id, 0, EditStart::Typed("4".into()));
    harness.answer_structure(publishers_structure());
    set_text(&mut harness, tab, id, "42");
    commit(&mut harness, tab, id, Advance::Stay);
    assert!(editing(&harness, tab, id), "the commit did not wait");
    harness.fail_lookup(tabletist_db::Error::query("permission denied"));
    assert!(!editing(&harness, tab, id));
    assert_eq!(pending(&harness, tab, id, 0).as_deref(), Some("42"));

    edit(&mut harness, tab, id, 1, EditStart::Typed("h".into()));
    set_text(&mut harness, tab, id, "harb");
    commit(&mut harness, tab, id, Advance::Stay);
    harness.fail_lookup(tabletist_db::Error::query("permission denied"));
    assert!(
        editing(&harness, tab, id),
        "a text the column refuses was kept"
    );
    assert!(picker(&harness, tab, id).unwrap().refused);
    assert_eq!(pending(&harness, tab, id, 1), None);
}

/// A pinned row that is not found and matches that only begin keys
/// highlight nothing: the kept Enter keeps the text.
#[test]
fn a_kept_enter_keeps_the_text_when_no_row_is_found() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    open(&mut harness, tab, id, 0, EditStart::Typed("9".into()));
    harness.answer_structure(publishers_structure());
    set_text(&mut harness, tab, id, "91");
    commit(&mut harness, tab, id, Advance::Stay);
    assert!(editing(&harness, tab, id), "the commit did not wait");
    harness.answer_lookups(&PUBLISHERS);
    assert!(!editing(&harness, tab, id));
    assert_eq!(pending(&harness, tab, id, 0).as_deref(), Some("91"));
}

/// An Enter kept while the text that shows was put in the editor without
/// the search for it: the reads in flight are for another text. It stands
/// for an option's key the column refused (the editor stays open, the key
/// is the text now, no `EditorTyped` follows) and a second Enter: with
/// integer keys in an integer column the fixtures cannot refuse one.
#[test]
fn an_enter_kept_for_a_text_no_search_was_sent_for_sends_it() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    open(&mut harness, tab, id, 0, EditStart::Typed("h".into()));
    harness.answer_structure(publishers_structure());
    let object = harness
        .app
        .workspace_mut(tab)
        .unwrap()
        .object_tab_mut(id)
        .unwrap();
    object.edits.editor.as_mut().unwrap().text = "42".to_owned();
    let sent = harness.lookups().len();
    commit(&mut harness, tab, id, Advance::Stay);
    assert!(editing(&harness, tab, id), "the commit did not wait");
    assert!(
        harness.lookups().len() > sent,
        "no search for `42` was sent"
    );
    harness.answer_lookups(&PUBLISHERS);
    assert!(!editing(&harness, tab, id));
    assert_eq!(pending(&harness, tab, id, 0).as_deref(), Some("42"));
}

/// A click on an option that is not there, and a move with nothing listed,
/// change nothing.
#[test]
fn a_click_past_the_list_and_a_move_over_nothing_change_nothing() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    edit(&mut harness, tab, id, 0, EditStart::Typed("h".into()));
    harness.app.apply(Action::PickOption { tab, id, index: 99 });
    assert!(editing(&harness, tab, id));
    assert_eq!(pending(&harness, tab, id, 0), None);
    harness.app.apply(Action::CancelEdit { tab, id });

    open(&mut harness, tab, id, 0, EditStart::Typed("h".into()));
    harness.answer_structure(publishers_structure());
    assert!(listed(&harness, tab, id).is_empty());
    harness.app.apply(Action::PickerMove { tab, id, by: 1 });
    assert!(editing(&harness, tab, id));
    let picker = picker(&harness, tab, id).unwrap();
    assert_eq!(picker.highlight, None);
    assert!(!picker.moved);
}

/// Test 23: a new row's foreign key cell opens the list, and a pick fills
/// what was required.
#[test]
fn a_new_rows_cell_is_picked_into() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    harness.press(Key::N, Modifiers::COMMAND);
    harness.settle();
    let new = crate::edit::new_row(0);
    // Adding the row opened the editor on its first required cell,
    // `publisher_id`, on nothing: the table's first rows are listed.
    harness.answer_structure(publishers_structure());
    harness.answer_lookups(&PUBLISHERS);
    assert_eq!(
        listed(&harness, tab, id),
        [NORTHLIGHT, HARBOR_PRESS, HARBOR_LIGHTS]
    );
    assert_eq!(picker(&harness, tab, id).unwrap().highlight, None);
    assert!(!harness.app.lacking(tab, id).is_empty());
    type_text(&mut harness, tab, id, "harb");
    commit(&mut harness, tab, id, Advance::Stay);
    assert!(!editing(&harness, tab, id));
    assert_eq!(
        pending(&harness, tab, id, new).as_deref(),
        Some(HARBOR_PRESS)
    );
    assert!(harness.app.lacking(tab, id).is_empty());
}

/// Test 24: `gd` follows the value the cell shows, the pending one where
/// there is one.
#[test]
fn gd_follows_the_pending_value() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    // Row 0 points at Northlight; the pick is another publisher.
    edit(&mut harness, tab, id, 0, EditStart::Typed("h".into()));
    type_text(&mut harness, tab, id, "harb");
    commit(&mut harness, tab, id, Advance::Stay);
    assert_eq!(pending(&harness, tab, id, 0).as_deref(), Some(HARBOR_PRESS));
    harness.app.apply(Action::FollowSelectedKey {
        tab,
        object_tab: id,
    });
    let filter = harness
        .app
        .backend
        .sent
        .iter()
        .rev()
        .find_map(|command| match command {
            Command::FetchRows { query, .. } if query.object.name == "publishers" => {
                query.filters.first().cloned()
            }
            _ => None,
        })
        .expect("the publishers were asked for");
    assert_eq!(
        (filter.column.as_str(), filter.value.as_str()),
        ("id", HARBOR_PRESS)
    );
}

/// A key whose save the database refused still shows in its cell, and is
/// the one `gd` follows: the row it names may be what the user looks for.
#[test]
fn gd_follows_a_pending_value_whose_save_failed() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    select(&mut harness, tab, id, 0);
    let object = harness
        .app
        .workspace_mut(tab)
        .unwrap()
        .object_tab_mut(id)
        .unwrap();
    object.edits.cells.insert(
        (0, PUBLISHER),
        crate::edit::Pending {
            new: NewValue::Text(HARBOR_PRESS.into()),
            state: crate::edit::State::Failed(tabletist_db::Error::Cancelled),
        },
    );
    harness.app.apply(Action::FollowSelectedKey {
        tab,
        object_tab: id,
    });
    let followed = harness.app.backend.sent.iter().rev().find_map(|command| {
        let Command::FetchRows { query, .. } = command else {
            return None;
        };
        let filter = query.filters.first()?;
        (query.object.name == "publishers").then(|| filter.value.clone())
    });
    assert_eq!(followed.as_deref(), Some(HARBOR_PRESS));
}

/// A pending value with nothing to follow (NULL, the default, now, a text
/// the column refuses) follows nothing, and not the value it replaced.
#[test]
fn gd_follows_nothing_where_the_pending_value_is_not_a_key() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    select(&mut harness, tab, id, 0);
    let states = [
        (NewValue::Null, crate::edit::State::Ready),
        (
            NewValue::Default { expression: None },
            crate::edit::State::Ready,
        ),
        (NewValue::Now, crate::edit::State::Ready),
        (
            NewValue::Text("harb".into()),
            crate::edit::State::ToFix(crate::edit::Problem::WholeNumber),
        ),
    ];
    // With nothing pending the loaded key is followed: the cell is one
    // `gd` reads.
    harness.app.apply(Action::FollowSelectedKey {
        tab,
        object_tab: id,
    });
    let sent_before = harness.app.backend.sent.len();
    assert!(matches!(
        harness.app.backend.sent.last(),
        Some(Command::FetchRows { query, .. }) if query.object.name == "publishers"
    ));
    for (new, state) in states {
        let object = harness
            .app
            .workspace_mut(tab)
            .unwrap()
            .object_tab_mut(id)
            .unwrap();
        object
            .edits
            .cells
            .insert((0, PUBLISHER), crate::edit::Pending { new, state });
        harness.app.apply(Action::FollowSelectedKey {
            tab,
            object_tab: id,
        });
    }
    assert_eq!(harness.app.backend.sent.len(), sent_before);
}

/// Spec section 5: a pick that changes nothing (the cell already points at
/// that row, with or without a change pending before) leaves nothing
/// pending and nothing to say; so does a commit the column refused.
#[test]
fn a_pick_equal_to_the_loaded_value_says_nothing() {
    let picked = |harness: &Harness, tab, id| {
        let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
        object.edits.picked.clone()
    };
    let (mut harness, tab, id) = covers_in(Look::macos());
    // A pick that is a change says so.
    edit(&mut harness, tab, id, 0, EditStart::Typed("h".into()));
    type_text(&mut harness, tab, id, "lights");
    commit(&mut harness, tab, id, Advance::Stay);
    assert!(picked(&harness, tab, id).is_some());

    // Row 1 points at Harbor Press already.
    edit(&mut harness, tab, id, 1, EditStart::Typed("h".into()));
    type_text(&mut harness, tab, id, "harb");
    commit(&mut harness, tab, id, Advance::Stay);
    assert!(!editing(&harness, tab, id));
    assert_eq!(pending(&harness, tab, id, 1), None);
    assert!(picked(&harness, tab, id).is_none());

    // The same on a cell with a change pending: the change is reverted.
    edit(&mut harness, tab, id, 1, EditStart::Typed("l".into()));
    type_text(&mut harness, tab, id, "lights");
    commit(&mut harness, tab, id, Advance::Stay);
    assert_eq!(
        pending(&harness, tab, id, 1).as_deref(),
        Some(HARBOR_LIGHTS)
    );
    assert!(picked(&harness, tab, id).is_some());
    edit(&mut harness, tab, id, 1, EditStart::Typed("h".into()));
    type_text(&mut harness, tab, id, "harb");
    commit(&mut harness, tab, id, Advance::Stay);
    assert!(!editing(&harness, tab, id));
    assert_eq!(pending(&harness, tab, id, 1), None);
    assert!(picked(&harness, tab, id).is_none());

    // A text the column refuses is no pick.
    edit(&mut harness, tab, id, 2, EditStart::Typed("z".into()));
    type_text(&mut harness, tab, id, "zzz");
    commit(&mut harness, tab, id, Advance::Stay);
    assert!(editing(&harness, tab, id));
    assert!(picked(&harness, tab, id).is_none());
}

/// The picker's Set NULL is the picker's: an editor without one is left
/// open and nothing changes. One with a picker nulls its own cell, not the
/// selection.
#[test]
fn set_null_belongs_to_the_picker_and_acts_on_the_editors_cell() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    // `kind` is no foreign key: a text editor, no picker.
    harness.app.apply(Action::EditCell {
        tab,
        id,
        cell: CellPos { row: 0, col: 2 },
        start: EditStart::Value,
    });
    harness.app.apply(Action::PickerNull { tab, id });
    assert!(editing(&harness, tab, id));
    let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
    assert!(object.edits.why.is_none());
    harness.app.apply(Action::CancelEdit { tab, id });

    // The selection is elsewhere: the cell that says why is the editor's.
    edit(&mut harness, tab, id, 1, EditStart::Typed("h".into()));
    let object = harness
        .app
        .workspace_mut(tab)
        .unwrap()
        .object_tab_mut(id)
        .unwrap();
    object.selection = Some(CellPos { row: 0, col: 0 });
    harness.app.apply(Action::PickerNull { tab, id });
    assert!(!editing(&harness, tab, id));
    let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
    let (cell, _) = object.edits.why.expect("the cell says why");
    assert_eq!((cell.row, cell.col), (1, PUBLISHER));
}

/// A kept Enter whose answer comes while a prompt about pending changes is
/// up is dropped with the prompt's other editing actions: the key is lost,
/// the editor stays open, and the next Enter takes at once.
#[test]
fn an_enter_kept_across_a_prompt_is_lost_and_the_next_one_takes() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    // Something pending, so that a reload is asked about.
    edit(&mut harness, tab, id, 0, EditStart::Typed("h".into()));
    type_text(&mut harness, tab, id, "lights");
    commit(&mut harness, tab, id, Advance::Stay);
    assert_eq!(
        pending(&harness, tab, id, 0).as_deref(),
        Some(HARBOR_LIGHTS)
    );

    edit(&mut harness, tab, id, 1, EditStart::Typed("l".into()));
    set_text(&mut harness, tab, id, "lights");
    commit(&mut harness, tab, id, Advance::Stay);
    assert!(picker(&harness, tab, id).unwrap().kept.is_some());
    harness.app.apply(Action::Refresh(tab));
    assert!(harness.app.dialog.is_some(), "no prompt was raised");
    harness.answer_lookups(&PUBLISHERS);
    assert!(editing(&harness, tab, id));
    assert_eq!(pending(&harness, tab, id, 1), None);
    assert_eq!(picker(&harness, tab, id).unwrap().kept, None);
    // The prompt is answered, and the next Enter has its list.
    harness.app.apply(Action::CloseDialog);
    commit(&mut harness, tab, id, Advance::Stay);
    assert!(!editing(&harness, tab, id));
    assert_eq!(
        pending(&harness, tab, id, 1).as_deref(),
        Some(HARBOR_LIGHTS)
    );
}

/// Types `text` on the selected cell, a frame at a time, and answers what
/// the picker asks.
fn type_on_cell(harness: &mut Harness, text: &str) {
    harness.frame(vec![egui::Event::Text(text.into())]);
    harness.settle();
    harness.answer_structure(publishers_structure());
    harness.settle();
    harness.answer_lookups(&PUBLISHERS);
    harness.settle();
}

fn select(harness: &mut Harness, tab: ConnTabId, id: TabId, row: usize) {
    let cell = CellPos {
        row,
        col: PUBLISHER,
    };
    harness.app.apply(Action::SelectCell { tab, id, cell });
    harness.settle();
}

/// Tests 9 and 10: the field and the list are named as the board names
/// them, and the head line says what is listed.
#[test]
fn the_field_and_the_list_are_named_for_the_table_they_search() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    select(&mut harness, tab, id, 0);
    type_on_cell(&mut harness, "harb");
    let tree = harness.settle();
    assert!(
        crate::testing::node(&tree, "Search publishers for publisher_id", Role::TextInput)
            .is_some(),
        "{:?}",
        crate::testing::labels(&tree)
    );
    assert!(crate::testing::node(&tree, "Matching publishers", Role::ListBox).is_some());
    assert!(harness.has("publishers matching \"harb\" · ↑↓ choose · ↩ set"));
    assert!(
        crate::testing::node(
            &tree,
            "Harbor Press, 9100000000000000004",
            Role::ListBoxOption
        )
        .is_some()
    );
    assert!(harness.has("2 matches in publishers"));
    assert!(harness.has("Esc cancels"));
    // NOT NULL: no Set NULL.
    assert!(crate::testing::node(&tree, "Set NULL", Role::Button).is_none());
}

/// Test 11 through the keys: the arrows move the list, not the grid's
/// selection, and Enter takes.
#[test]
fn the_keys_reach_the_list_while_the_field_has_the_keyboard() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    select(&mut harness, tab, id, 0);
    type_on_cell(&mut harness, "harb");
    harness.press(Key::ArrowDown, Modifiers::NONE);
    assert_eq!(picker(&harness, tab, id).unwrap().highlight, Some(1));
    assert_eq!(selection(&harness, tab, id), (0, PUBLISHER));
    harness.press(Key::Enter, Modifiers::NONE);
    assert!(!editing(&harness, tab, id));
    assert_eq!(
        pending(&harness, tab, id, 0).as_deref(),
        Some(HARBOR_LIGHTS)
    );
    assert_eq!(selection(&harness, tab, id), (1, PUBLISHER));
    // What a screen reader is told of the pick.
    assert!(harness.has("publisher_id set to 9100000000000000007, Harbor Lights"));
}

/// Test 12: a click on an option takes it, and the text typed is not
/// first kept as a value.
#[test]
fn a_click_on_an_option_takes_it() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    select(&mut harness, tab, id, 0);
    type_on_cell(&mut harness, "harb");
    harness.click("Harbor Lights, 9100000000000000007");
    assert!(!editing(&harness, tab, id));
    assert_eq!(
        pending(&harness, tab, id, 0).as_deref(),
        Some(HARBOR_LIGHTS)
    );
}

/// Test 13, on macOS: Esc closes the list and leaves the cell as it was.
#[test]
fn esc_closes_the_list_without_a_change() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    select(&mut harness, tab, id, 0);
    type_on_cell(&mut harness, "harb");
    harness.press(Key::Escape, Modifiers::NONE);
    assert!(!editing(&harness, tab, id));
    assert_eq!(pending(&harness, tab, id, 0), None);
    let tree = harness.settle();
    assert!(crate::testing::node(&tree, "Matching publishers", Role::ListBox).is_none());
}

/// Test 19: nothing found, a failure, and more than the list holds each
/// have their line.
#[test]
fn the_list_says_when_it_has_nothing_failed_or_holds_more() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    select(&mut harness, tab, id, 0);
    type_on_cell(&mut harness, "zzz");
    assert!(harness.has("No publishers match \"zzz\""));
    harness.press(Key::Escape, Modifiers::NONE);

    select(&mut harness, tab, id, 0);
    harness.frame(vec![egui::Event::Text("h".into())]);
    harness.settle();
    harness.fail_lookup(tabletist_db::Error::query(
        "permission denied for table publishers",
    ));
    assert!(harness.has("permission denied for table publishers"));
    harness.press(Key::Escape, Modifiers::NONE);

    // Sixty publishers whose names hold an x: fifty are listed.
    let names: Vec<String> = (0..60).map(|index| format!("Annex {index}")).collect();
    let many: Vec<(i64, &str)> = names
        .iter()
        .enumerate()
        .map(|(index, name)| (index as i64 + 1, name.as_str()))
        .collect();
    select(&mut harness, tab, id, 0);
    harness.frame(vec![egui::Event::Text("x".into())]);
    harness.settle();
    harness.answer_lookups(&many);
    assert!(harness.has("First 50 shown. Keep typing to narrow."));
    assert!(harness.has("50 or more matches in publishers"));
}

/// The Bookshop's covers with `publisher_id` taking NULL, the grid holding
/// the keyboard.
fn nullable_covers() -> (Harness, ConnTabId, TabId) {
    let mut harness = Harness::new();
    harness.set_look(Look::macos());
    let tab = harness.connect_fake_as(false);
    harness.app.apply(Action::OpenObject {
        tab,
        object: tabletist_db::ObjectRef::new("main", "book_covers"),
        kind: tabletist_db::ObjectKind::Table,
        pin: true,
    });
    let mut structure = crate::testing::keyed_covers_structure();
    structure.columns[PUBLISHER].nullable = true;
    harness.answer_structure(structure);
    harness.answer_rows(crate::testing::keyed_covers_page());
    let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
    harness.app.workspace_mut(tab).unwrap().pane = Pane::Grid;
    (harness, tab, id)
}

/// Whether the page's row 0 holds a pending NULL in `publisher_id`.
fn pending_null(harness: &Harness, tab: ConnTabId, id: TabId) -> bool {
    let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
    matches!(
        object
            .edits
            .cells
            .get(&(0, PUBLISHER))
            .map(|pending| &pending.new),
        Some(NewValue::Null)
    )
}

/// Test 18, the nullable half: the foot offers Set NULL, and it sets
/// NULL.
#[test]
fn a_nullable_columns_list_sets_null() {
    let (mut harness, tab, id) = nullable_covers();
    select(&mut harness, tab, id, 0);
    type_on_cell(&mut harness, "harb");
    harness.click("Set NULL");
    assert!(!editing(&harness, tab, id));
    assert!(pending_null(&harness, tab, id));
}

/// Clicks the node `label` of `role` with the pointer, as a mouse does:
/// moved onto it, pressed and released in frames of their own. The press
/// takes the keyboard from a field that is not asked to keep it.
fn pointer_click(harness: &mut Harness, label: &str, role: Role) {
    let tree = harness.settle();
    let at = crate::testing::bounds(&tree, label, role)
        .unwrap_or_else(|| {
            panic!(
                "nothing labelled {label:?}: {:?}",
                crate::testing::labels(&tree)
            )
        })
        .center();
    let button = |pressed| egui::Event::PointerButton {
        pos: at,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    harness.frame(vec![egui::Event::PointerMoved(at)]);
    harness.frame(vec![button(true)]);
    harness.frame(vec![button(false)]);
    harness.settle();
}

/// Test 12, as a mouse does it: the press on an option is the list's, so
/// the frame does not read as the editor left, and the text typed is not
/// first kept as a value.
#[test]
fn a_pointer_click_on_an_option_takes_it_and_not_the_text() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    select(&mut harness, tab, id, 0);
    type_on_cell(&mut harness, "harb");
    pointer_click(
        &mut harness,
        "Harbor Lights, 9100000000000000007",
        Role::ListBoxOption,
    );
    assert!(!editing(&harness, tab, id));
    assert_eq!(
        pending(&harness, tab, id, 0).as_deref(),
        Some(HARBOR_LIGHTS)
    );
}

/// A press between the options, on the list's head or foot, is the list's
/// too: the editor stays open with the text as it was.
#[test]
fn a_press_beside_the_options_keeps_the_editor_and_its_text() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    select(&mut harness, tab, id, 0);
    type_on_cell(&mut harness, "harb");
    pointer_click(&mut harness, "Esc cancels", Role::Label);
    assert!(editing(&harness, tab, id));
    assert!(picker(&harness, tab, id).is_some());
    assert_eq!(pending(&harness, tab, id, 0), None);
    // The keyboard is the field's still: Enter takes the highlighted row.
    harness.press(Key::Enter, Modifiers::NONE);
    assert!(!editing(&harness, tab, id));
    assert_eq!(pending(&harness, tab, id, 0).as_deref(), Some(HARBOR_PRESS));
}

/// Spec 5: while the text is a search the field says nothing of what the
/// column would make of it. Once a commit was refused it says why, above
/// the cell, since the list is under it.
#[test]
fn the_field_says_what_the_column_refuses_only_after_a_refused_commit() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    select(&mut harness, tab, id, 0);
    type_on_cell(&mut harness, "zzz");
    let note = "INTEGER expects a whole number";
    assert!(!harness.has(note), "said while it is a search");
    harness.press(Key::Enter, Modifiers::NONE);
    assert!(editing(&harness, tab, id));
    let tree = harness.settle();
    let said = crate::testing::bounds(&tree, note, Role::Label)
        .unwrap_or_else(|| panic!("not said: {:?}", crate::testing::labels(&tree)));
    let field =
        crate::testing::bounds(&tree, "Search publishers for publisher_id", Role::TextInput)
            .expect("the field");
    let list =
        crate::testing::bounds(&tree, "Matching publishers", Role::ListBox).expect("the list");
    assert!(said.bottom() <= field.top(), "{said:?} {field:?}");
    assert!(list.top() >= field.bottom(), "{list:?} {field:?}");
}

/// The list is under the cell and inside the window.
#[test]
fn the_list_stands_under_its_cell_and_inside_the_window() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    select(&mut harness, tab, id, 0);
    type_on_cell(&mut harness, "harb");
    let tree = harness.settle();
    let field =
        crate::testing::bounds(&tree, "Search publishers for publisher_id", Role::TextInput)
            .expect("the field");
    let list =
        crate::testing::bounds(&tree, "Matching publishers", Role::ListBox).expect("the list");
    assert!(list.top() >= field.bottom(), "{list:?} {field:?}");
    assert!(
        list.left() >= 0.0 && list.right() <= harness.size.x,
        "{list:?}"
    );
    assert!(list.bottom() <= harness.size.y, "{list:?}");
}

/// The Bookshop's covers in a window of `size`, as [`covers_in`] has them,
/// with `rows` rows in the page.
fn covers_sized(size: egui::Vec2, rows: usize) -> (Harness, ConnTabId, TabId) {
    let mut harness = Harness::with_size(size);
    harness.set_look(Look::macos());
    let tab = harness.connect_fake_as(false);
    harness.app.apply(Action::OpenObject {
        tab,
        object: ObjectRef::new("main", "book_covers"),
        kind: tabletist_db::ObjectKind::Table,
        pin: true,
    });
    harness.answer_structure(crate::testing::keyed_covers_structure());
    let mut page = crate::testing::book_covers_page(rows);
    for (row, (key, _)) in page.rows.iter_mut().zip(PUBLISHERS) {
        row[1] = Value::Int(key);
    }
    harness.answer_rows(page);
    let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
    harness.app.workspace_mut(tab).unwrap().pane = Pane::Grid;
    harness.settle();
    (harness, tab, id)
}

/// Sixty publishers whose names hold an x: more than the list holds.
fn annexes() -> Vec<String> {
    (0..60).map(|index| format!("Annex {index}")).collect()
}

/// Types an x on the selected cell, and has it answered by `names`, the
/// publishers numbered from 1.
fn search_annexes(harness: &mut Harness, names: &[String]) {
    harness.frame(vec![egui::Event::Text("x".into())]);
    harness.settle();
    harness.answer_structure(publishers_structure());
    let many: Vec<(i64, &str)> = names
        .iter()
        .enumerate()
        .map(|(index, name)| (index as i64 + 1, name.as_str()))
        .collect();
    harness.answer_lookups(&many);
    harness.settle();
}

fn window_of(harness: &Harness) -> egui::Rect {
    egui::Rect::from_min_size(egui::Pos2::ZERO, harness.size)
}

/// A full list does not run off the window's bottom: with the cell near
/// it, the list takes the side with more room, above the cell, and shows
/// the options that fit there.
#[test]
fn a_list_that_does_not_fit_below_goes_above_with_fewer_options() {
    let (mut harness, tab, id) = covers_sized(egui::vec2(1280.0, 500.0), 3);
    select(&mut harness, tab, id, 0);
    search_annexes(&mut harness, &annexes());
    let tree = harness.settle();
    let field =
        crate::testing::bounds(&tree, "Search publishers for publisher_id", Role::TextInput)
            .expect("the field");
    let list =
        crate::testing::bounds(&tree, "Matching publishers", Role::ListBox).expect("the list");
    assert!(window_of(&harness).contains_rect(list), "{list:?}");
    assert!(list.bottom() <= field.top(), "{list:?} {field:?}");
    // Fewer than eight options, and the first of them is drawn.
    let drawn = tree
        .nodes
        .iter()
        .filter(|(_, node)| node.role() == Role::ListBoxOption)
        .count();
    assert!((1..8).contains(&drawn), "{drawn}");
}

/// Where eight options fit on neither side, the highlighted one is still
/// among those drawn, wherever the keys take it.
#[test]
fn the_highlighted_option_is_drawn_in_a_short_window() {
    let (mut harness, tab, id) = covers_sized(egui::vec2(1280.0, 340.0), 3);
    select(&mut harness, tab, id, 0);
    search_annexes(&mut harness, &annexes());
    for _ in 0..7 {
        harness.press(Key::ArrowDown, Modifiers::NONE);
    }
    assert_eq!(picker(&harness, tab, id).unwrap().highlight, Some(7));
    let tree = harness.settle();
    let list =
        crate::testing::bounds(&tree, "Matching publishers", Role::ListBox).expect("the list");
    assert!(window_of(&harness).contains_rect(list), "{list:?}");
    assert!(
        crate::testing::node(&tree, "Annex 7, 8", Role::ListBoxOption).is_some(),
        "{:?}",
        crate::testing::labels(&tree)
    );
}

/// The list hangs from its cell while the cell shows. The harness cannot
/// scroll an edited cell away (the grid brings the cell it edits back into
/// view every frame), so the rule is checked on its own.
#[test]
fn a_list_hangs_only_from_a_cell_that_shows() {
    use super::key_picker::hung_from;
    let body = egui::Rect::from_min_max(egui::pos2(0.0, 40.0), egui::pos2(600.0, 400.0));
    let cell = |top: f32, left: f32| {
        egui::Rect::from_min_size(egui::pos2(left, top), egui::vec2(100.0, 30.0))
    };
    // In sight: the list hangs from the cell itself.
    assert_eq!(hung_from(cell(100.0, 10.0), body), Some(cell(100.0, 10.0)));
    // Its top edge under the header, wholly or by a little.
    assert_eq!(hung_from(cell(-20.0, 10.0), body), None);
    assert_eq!(hung_from(cell(30.0, 10.0), body), None);
    // Below the body, or beside it.
    assert_eq!(hung_from(cell(410.0, 10.0), body), None);
    assert_eq!(hung_from(cell(100.0, 620.0), body), None);
    assert_eq!(hung_from(cell(100.0, -150.0), body), None);
    // Cut at the bottom or at a side: from the part that shows.
    let cut = hung_from(cell(385.0, 10.0), body).unwrap();
    assert_eq!(cut.bottom(), 400.0);
    let cut = hung_from(cell(100.0, 560.0), body).unwrap();
    assert_eq!(cut.right(), 600.0);
}

/// A line of words longer than the list is cut with "…" inside it, and a
/// screen reader has the whole of it.
#[test]
fn a_long_search_text_is_cut_inside_the_list() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    select(&mut harness, tab, id, 0);
    let long = "harbor press and the lights of the north";
    assert_eq!(long.chars().count(), 40);
    type_on_cell(&mut harness, long);
    let head = format!("publishers matching \"{long}\" · ↑↓ choose · ↩ set");
    assert!(harness.has(&head), "the whole head line is not announced");
    assert!(harness.has(&format!("No publishers match \"{long}\"")));
    let tree = harness.settle();
    let list =
        crate::testing::bounds(&tree, "Matching publishers", Role::ListBox).expect("the list");
    let lines: Vec<_> = harness
        .text_rects
        .iter()
        .filter(|(text, _)| {
            text.starts_with("publishers matching") || text.starts_with("No publishers")
        })
        .cloned()
        .collect();
    assert_eq!(lines.len(), 2, "{lines:?}");
    for (text, rect) in lines {
        assert!(text.ends_with('…'), "{text:?}");
        assert!(list.contains_rect(rect), "{text:?} {rect:?} {list:?}");
    }
}

/// The error a database gives is a line of the list too, and is cut.
#[test]
fn a_long_error_is_cut_inside_the_list() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    select(&mut harness, tab, id, 0);
    harness.frame(vec![egui::Event::Text("h".into())]);
    harness.settle();
    harness.answer_structure(publishers_structure());
    let words = "permission denied for table publishers because the role has no grant on it";
    harness.fail_lookup(tabletist_db::Error::query(words));
    assert!(harness.has(words));
    let tree = harness.settle();
    let list =
        crate::testing::bounds(&tree, "Matching publishers", Role::ListBox).expect("the list");
    let cut = harness
        .text_rects
        .iter()
        .find(|(text, _)| text.starts_with("permission denied"))
        .cloned()
        .expect("the error is painted");
    assert!(cut.0.ends_with('…'), "{:?}", cut.0);
    assert!(list.contains_rect(cut.1), "{cut:?} {list:?}");
}

/// A count is a polite live region, and so is the pick.
#[test]
fn the_count_and_the_pick_are_polite_live_regions() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    select(&mut harness, tab, id, 0);
    type_on_cell(&mut harness, "harb");
    let live = |tree: &egui::accesskit::TreeUpdate, label: &str| {
        tree.nodes
            .iter()
            .find(|(_, node)| node.label().or_else(|| node.value()) == Some(label))
            .map(|(_, node)| node.live())
    };
    let tree = harness.settle();
    assert_eq!(
        live(&tree, "2 matches in publishers"),
        Some(Some(egui::accesskit::Live::Polite))
    );
    harness.press(Key::Enter, Modifiers::NONE);
    let tree = harness.settle();
    assert_eq!(
        live(
            &tree,
            "publisher_id set to 9100000000000000004, Harbor Press"
        ),
        Some(Some(egui::accesskit::Live::Polite))
    );
}

/// The row the cell points at is named so, not only marked.
#[test]
fn the_current_option_says_so_in_its_name() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    // Row 1 points at Harbor Press.
    edit(&mut harness, tab, id, 1, EditStart::Value);
    let tree = harness.settle();
    assert!(
        crate::testing::node(
            &tree,
            "Harbor Press, 9100000000000000004, current",
            Role::ListBoxOption
        )
        .is_some(),
        "{:?}",
        crate::testing::labels(&tree)
    );
    assert!(
        crate::testing::node(
            &tree,
            "Northlight Books, 9100000000000000001",
            Role::ListBoxOption
        )
        .is_some()
    );
}

/// A label that is an empty text names the option by its key alone, as a
/// NULL one does.
#[test]
fn an_option_with_an_empty_label_is_named_by_its_key() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    select(&mut harness, tab, id, 0);
    harness.frame(vec![egui::Event::Text("7".into())]);
    harness.settle();
    harness.answer_structure(publishers_structure());
    harness.answer_lookups(&[(7, "")]);
    let tree = harness.settle();
    assert!(
        crate::testing::node(&tree, "7", Role::ListBoxOption).is_some(),
        "{:?}",
        crate::testing::labels(&tree)
    );
}

/// Page Down and Page Up move the highlight by the list's height.
#[test]
fn page_down_and_page_up_move_by_the_height_of_the_list() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    select(&mut harness, tab, id, 0);
    let names: Vec<String> = (0..20).map(|index| format!("Annex {index}")).collect();
    search_annexes(&mut harness, &names);
    assert_eq!(picker(&harness, tab, id).unwrap().highlight, Some(0));
    harness.press(Key::PageDown, Modifiers::NONE);
    assert_eq!(
        picker(&harness, tab, id).unwrap().highlight,
        Some(crate::key_picker::VISIBLE)
    );
    harness.press(Key::PageDown, Modifiers::NONE);
    harness.press(Key::PageDown, Modifiers::NONE);
    // The list ends at its last option.
    assert_eq!(picker(&harness, tab, id).unwrap().highlight, Some(19));
    harness.press(Key::PageUp, Modifiers::NONE);
    assert_eq!(picker(&harness, tab, id).unwrap().highlight, Some(11));
    assert_eq!(selection(&harness, tab, id), (0, PUBLISHER));
}

/// Opened on its value with Enter, the whole text is selected: one
/// character typed is the field's text, and a search for it.
#[test]
fn a_character_typed_over_an_opened_value_replaces_it() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    select(&mut harness, tab, id, 1);
    harness.press(Key::Enter, Modifiers::NONE);
    assert!(editing(&harness, tab, id));
    harness.answer_structure(publishers_structure());
    harness.answer_lookups(&PUBLISHERS);
    harness.settle();
    harness.frame(vec![egui::Event::Text("h".into())]);
    harness.settle();
    let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
    assert_eq!(object.edits.editor.as_ref().unwrap().text, "h");
    assert!(picker(&harness, tab, id).unwrap().searching);
}

/// An arrow pressed in the first frame of an editor, before its field has
/// the keyboard, is the list's.
#[test]
fn an_arrow_right_after_the_editor_opens_is_the_lists() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    // No frame has drawn the editor yet: its field has not taken the
    // keyboard.
    edit(&mut harness, tab, id, 0, EditStart::Typed("h".into()));
    assert_eq!(picker(&harness, tab, id).unwrap().highlight, Some(0));
    harness.frame(vec![crate::testing::key(Key::ArrowDown, Modifiers::NONE)]);
    assert_eq!(picker(&harness, tab, id).unwrap().highlight, Some(1));
}

/// Test 13, in the terminal look: Esc closes the picker without a change
/// (on a text cell it keeps the edit), and so does Ctrl+C. Ctrl+N and
/// Ctrl+P move the highlight and do nothing else.
#[test]
fn the_terminal_look_moves_with_ctrl_n_and_closes_with_esc() {
    let (mut harness, tab, id) = covers_in(Look::omarchy());
    select(&mut harness, tab, id, 0);
    // A letter on a selected cell is a normal-mode command here: the
    // editor is opened, and the text typed into its field.
    harness.app.apply(Action::EditCell {
        tab,
        id,
        cell: CellPos {
            row: 0,
            col: PUBLISHER,
        },
        start: EditStart::Replace(String::new()),
    });
    harness.settle();
    type_on_cell(&mut harness, "harb");
    assert_eq!(picker(&harness, tab, id).unwrap().highlight, Some(0));
    let tabs = harness.app.tabs.len();
    // What a real Ctrl press reports: outside macOS Ctrl is the command
    // key too (as `ctrl()` of `src/ui/complete_tests.rs`).
    let ctrl = if cfg!(target_os = "macos") {
        Modifiers::CTRL
    } else {
        Modifiers::CTRL | Modifiers::COMMAND
    };
    harness.press(Key::N, ctrl);
    assert_eq!(picker(&harness, tab, id).unwrap().highlight, Some(1));
    harness.press(Key::P, ctrl);
    assert_eq!(picker(&harness, tab, id).unwrap().highlight, Some(0));
    // No new connection, no quick open.
    assert_eq!(harness.app.tabs.len(), tabs);
    assert!(harness.app.dialog.is_none());
    // The line at the bottom names the picker's keys. It is painted, not
    // announced: the whole of it is one piece of text, and the window is
    // wide enough for the line to keep it.
    harness.settle();
    let keys = "ctrl+n/p move · enter set · esc close";
    assert!(
        harness.painted.iter().any(|(piece, _)| piece == keys),
        "{:?}",
        harness.painted
    );
    // No head line and no foot in this look.
    assert!(!harness.has("Esc cancels"));

    harness.press(Key::Escape, Modifiers::NONE);
    assert!(!editing(&harness, tab, id));
    assert_eq!(pending(&harness, tab, id, 0), None);
}

/// Spec 2: in the terminal look `i` goes on from the text's end, and
/// nothing is selected.
#[test]
fn the_terminal_look_selects_nothing_on_opening() {
    let (mut harness, tab, id) = covers_in(Look::omarchy());
    edit(&mut harness, tab, id, 1, EditStart::Value);
    let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
    assert!(!object.edits.editor.as_ref().unwrap().select);
}

/// Spec 2 and 5, in the terminal look: the field keeps the cell, with a
/// `>` prompt before the text and the count of the list's options at its
/// end. The other looks have neither.
#[test]
fn the_terminal_look_puts_a_prompt_and_the_count_in_the_field() {
    let (mut harness, tab, id) = covers_in(Look::omarchy());
    select(&mut harness, tab, id, 0);
    harness.app.apply(Action::EditCell {
        tab,
        id,
        cell: CellPos {
            row: 0,
            col: PUBLISHER,
        },
        start: EditStart::Replace(String::new()),
    });
    harness.settle();
    type_on_cell(&mut harness, "harb");
    // Harbor Press and Harbor Lights.
    let prompt = harness.painted_rect("> ").expect("the prompt is painted");
    let field = harness.painted_rect("harb").expect("the text is painted");
    // A row's number is a "2" too: the count is the one on the field's line.
    let count = harness
        .text_rects
        .iter()
        .filter(|(piece, _)| piece == "2")
        .map(|(_, rect)| *rect)
        .find(|rect| (rect.center().y - field.center().y).abs() < 2.0)
        .expect("the count is painted on the field's line");
    assert!(prompt.right() <= field.left() + 0.5, "{prompt:?} {field:?}");
    assert!(count.left() >= field.right(), "{count:?} {field:?}");
    assert!((count.center().y - field.center().y).abs() < 2.0);

    let (mut harness, tab, id) = covers_in(Look::macos());
    select(&mut harness, tab, id, 0);
    type_on_cell(&mut harness, "harb");
    assert!(harness.painted_rect("> ").is_none());
}

/// A list that moves (it flips above its cell, the window is resized)
/// keeps the node that announces its count: a screen reader is not told
/// the same count again.
#[test]
fn the_count_keeps_its_node_when_the_list_moves() {
    let (mut harness, tab, id) = covers_sized(egui::vec2(1280.0, 800.0), 3);
    select(&mut harness, tab, id, 0);
    search_annexes(&mut harness, &annexes());
    let count = "50 or more matches in publishers";
    let node_of =
        |tree: &egui::accesskit::TreeUpdate| crate::testing::node(tree, count, Role::Label);
    let before = node_of(&harness.settle()).expect("the count is announced");
    let list = crate::testing::bounds(&harness.settle(), "Matching publishers", Role::ListBox);
    harness.size = egui::vec2(1280.0, 420.0);
    let tree = harness.settle();
    let moved = crate::testing::bounds(&tree, "Matching publishers", Role::ListBox);
    assert_ne!(list, moved, "the list did not move");
    assert_eq!(node_of(&tree), Some(before));
}

/// Opens the editor on `publisher_id` of row 0 in the terminal look, as
/// `i` would, with nothing in it.
fn open_in_terminal(harness: &mut Harness, tab: ConnTabId, id: TabId) {
    select(harness, tab, id, 0);
    harness.app.apply(Action::EditCell {
        tab,
        id,
        cell: CellPos {
            row: 0,
            col: PUBLISHER,
        },
        start: EditStart::Replace(String::new()),
    });
    harness.settle();
}

/// The editor's own words at the bottom of the terminal look.
const EDITOR_KEYS: &str = "esc normal · tab next cell";
const PICKER_KEYS: &str = "ctrl+n/p move · enter set · esc close";

/// The Ctrl+C of a terminal: where Ctrl is the command key too it comes
/// as a copy (`src/ui/mod.rs` presses it so).
fn ctrl_c(harness: &mut Harness) {
    let ctrl = Modifiers::CTRL | Modifiers::COMMAND;
    harness.frame(vec![egui::Event::ModifiersChanged(ctrl), egui::Event::Copy]);
    harness.frame(vec![egui::Event::ModifiersChanged(Modifiers::NONE)]);
    harness.settle();
}

/// Test 13, in the terminal look: Ctrl+C closes the picker with nothing
/// pending, and copies nothing.
#[test]
fn ctrl_c_closes_the_terminal_looks_picker_without_a_change() {
    let (mut harness, tab, id) = covers_in(Look::omarchy());
    open_in_terminal(&mut harness, tab, id);
    type_on_cell(&mut harness, "harb");
    assert!(picker(&harness, tab, id).is_some());
    ctrl_c(&mut harness);
    assert!(!editing(&harness, tab, id));
    assert_eq!(pending(&harness, tab, id, 0), None);
    assert_eq!(harness.copied, None);
}

/// A search that failed leaves the plain text editor, in the terminal
/// look too: Esc keeps a text the column takes, and the line at the bottom
/// names the editor's keys, not the picker's.
#[test]
fn a_failed_search_leaves_the_editors_keys_and_its_esc() {
    let (mut harness, tab, id) = covers_in(Look::omarchy());
    open_in_terminal(&mut harness, tab, id);
    harness.frame(vec![egui::Event::Text("42".into())]);
    harness.settle();
    harness.answer_structure(publishers_structure());
    harness.settle();
    // While the search lives, the picker's keys.
    assert!(
        harness
            .painted
            .iter()
            .any(|(piece, _)| piece == PICKER_KEYS)
    );
    harness.fail_lookup(tabletist_db::Error::query(
        "permission denied for table publishers",
    ));
    harness.settle();
    assert!(picker(&harness, tab, id).unwrap().failed.is_some());
    assert!(
        harness
            .painted
            .iter()
            .any(|(piece, _)| piece == EDITOR_KEYS)
    );
    assert!(
        !harness
            .painted
            .iter()
            .any(|(piece, _)| piece == PICKER_KEYS)
    );
    harness.press(Key::Escape, Modifiers::NONE);
    assert!(!editing(&harness, tab, id));
    assert_eq!(pending(&harness, tab, id, 0).as_deref(), Some("42"));
}

/// A search that failed has no list, so the keys that move in one are the
/// field's again: Down goes to the end of the text, as in any cell.
#[test]
fn a_failed_search_leaves_the_arrows_to_the_field() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    select(&mut harness, tab, id, 0);
    harness.frame(vec![egui::Event::Text("42".into())]);
    harness.settle();
    harness.answer_structure(publishers_structure());
    harness.settle();
    harness.fail_lookup(tabletist_db::Error::query(
        "permission denied for table publishers",
    ));
    harness.settle();
    assert!(picker(&harness, tab, id).unwrap().failed.is_some());
    harness.press(Key::Home, Modifiers::NONE);
    harness.press(Key::ArrowDown, Modifiers::NONE);
    harness.frame(vec![egui::Event::Text("7".into())]);
    harness.settle();
    let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
    assert_eq!(object.edits.editor.as_ref().unwrap().text, "427");
}

/// On the keyed fixture, a column that is no foreign key still has the
/// terminal look's editor: its words at the bottom, and Esc keeps the text.
#[test]
fn a_column_that_is_no_foreign_key_keeps_the_terminal_editor() {
    let (mut harness, tab, id) = covers_in(Look::omarchy());
    harness.app.apply(Action::EditCell {
        tab,
        id,
        cell: CellPos { row: 0, col: 2 },
        start: EditStart::Replace(String::new()),
    });
    harness.settle();
    assert!(picker(&harness, tab, id).is_none());
    harness.frame(vec![egui::Event::Text("ab".into())]);
    harness.settle();
    assert!(
        harness
            .painted
            .iter()
            .any(|(piece, _)| piece == EDITOR_KEYS)
    );
    assert!(
        !harness
            .painted
            .iter()
            .any(|(piece, _)| piece == PICKER_KEYS)
    );
    harness.press(Key::Escape, Modifiers::NONE);
    assert!(!editing(&harness, tab, id));
    let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
    match &object
        .edits
        .cells
        .get(&(0, 2))
        .expect("the text is kept")
        .new
    {
        NewValue::Text(text) => assert_eq!(text, "ab"),
        other => panic!("{other:?}"),
    }
}

/// In a window too narrow for the picker's three keys, the line falls back
/// to the one key that differs from the editor's before it drops them all.
#[test]
fn a_narrow_line_keeps_esc_close_when_the_keys_do_not_fit() {
    let mut seen = Vec::new();
    for width in (300..=1280).rev().step_by(10) {
        let mut harness = Harness::with_size(egui::vec2(width as f32, 800.0));
        harness.set_look(Look::omarchy());
        let (tab, id) = harness.keyed_covers();
        harness.app.workspace_mut(tab).unwrap().pane = Pane::Grid;
        harness.settle();
        open_in_terminal(&mut harness, tab, id);
        type_on_cell(&mut harness, "harb");
        let has = |text: &str| harness.painted.iter().any(|(piece, _)| piece == text);
        seen.push((has(PICKER_KEYS), has("esc close")));
    }
    // Wide: the whole of it. Then the short one alone. Never both.
    assert_eq!(seen.first(), Some(&(true, false)));
    assert!(seen.contains(&(false, true)), "{seen:?}");
    assert!(!seen.contains(&(true, true)));
}

/// The terminal look's list for a search of `t`, answered by the table
/// `names`, whose keys are 9100000000000000001 on.
fn terminal_list(names: &[String]) -> (Harness, ConnTabId, TabId) {
    let (mut harness, tab, id) = covers_in(Look::omarchy());
    open(&mut harness, tab, id, 0, EditStart::Typed("t".into()));
    harness.answer_structure(publishers_structure());
    let rows: Vec<(i64, &str)> = names
        .iter()
        .enumerate()
        .map(|(index, name)| (9_100_000_000_000_000_001 + index as i64, name.as_str()))
        .collect();
    harness.answer_lookups(&rows);
    harness.settle();
    (harness, tab, id)
}

/// The keys of the options a list of `count` draws, as the table above
/// numbers them.
fn first_keys(count: usize) -> Vec<String> {
    (0..count)
        .map(|index| (9_100_000_000_000_000_001_i64 + index as i64).to_string())
        .collect()
}

/// The terminal look's option is one line, and the key, which Enter
/// sets, always shows whole: the list is as wide as its options need.
#[test]
fn the_terminal_looks_keys_show_whole() {
    let names: Vec<String> = (1..=12)
        .map(|index| format!("The Lighthouse Keeper's Daughter {index}"))
        .collect();
    let (mut harness, _, _) = terminal_list(&names);
    let tree = harness.settle();
    let list =
        crate::testing::bounds(&tree, "Matching publishers", Role::ListBox).expect("the list");
    for key in first_keys(8) {
        let (_, rect) = harness
            .text_rects
            .iter()
            .find(|(text, _)| *text == key)
            .unwrap_or_else(|| panic!("{key} is not painted: {:?}", harness.painted));
        assert!(list.contains_rect(*rect), "{key} {rect:?} {list:?}");
    }
    // No wider than the cap, and wide enough that no label is cut.
    assert!(list.width() <= 560.0, "{list:?}");
    let labels: Vec<_> = harness
        .text_rects
        .iter()
        .filter(|(text, _)| text.starts_with("The Lighthouse"))
        .collect();
    assert_eq!(labels.len(), 8, "{labels:?}");
    assert!(
        labels.iter().all(|(text, _)| !text.ends_with('…')),
        "{labels:?}"
    );
}

/// Where even the widest the list may grow is not enough, the label gives
/// way, never the key.
#[test]
fn a_label_longer_than_the_list_may_be_is_cut_before_the_key() {
    let long = "A publisher with a name that goes on and on past the end of any list ".repeat(2);
    let names = vec![long.trim().to_owned(); 3];
    let (mut harness, _, _) = terminal_list(&names);
    let tree = harness.settle();
    let list =
        crate::testing::bounds(&tree, "Matching publishers", Role::ListBox).expect("the list");
    assert!(list.width() <= 560.0, "{list:?}");
    let label = harness
        .text_rects
        .iter()
        .find(|(text, _)| text.starts_with("A publisher with"))
        .cloned()
        .expect("the label is painted");
    assert!(label.0.ends_with('…'), "{:?}", label.0);
    assert!(list.contains_rect(label.1), "{label:?} {list:?}");
    for key in first_keys(3) {
        let (_, rect) = harness
            .text_rects
            .iter()
            .find(|(text, _)| *text == key)
            .unwrap_or_else(|| panic!("{key} is not painted"));
        assert!(list.contains_rect(*rect), "{key} {rect:?} {list:?}");
        // The key stands after the label, not over it.
        assert!(rect.left() >= label.1.right(), "{rect:?} {label:?}");
    }
}

/// The row the cell points at has its "current" mark after the key, and
/// the key still shows whole.
#[test]
fn the_terminal_looks_current_mark_leaves_the_key_whole() {
    let (mut harness, tab, id) = covers_in(Look::omarchy());
    // Row 1 points at Harbor Press.
    open(&mut harness, tab, id, 1, EditStart::Value);
    harness.answer_structure(publishers_structure());
    let names: Vec<String> = [
        "Northlight Books",
        "Harbor Press with a rather long name",
        "Harbor Lights",
    ]
    .iter()
    .map(|name| (*name).to_owned())
    .collect();
    let rows = [
        (9_100_000_000_000_000_001, names[0].as_str()),
        (9_100_000_000_000_000_004, names[1].as_str()),
        (9_100_000_000_000_000_007, names[2].as_str()),
    ];
    harness.answer_lookups(&rows);
    harness.settle();
    let tree = harness.settle();
    let list =
        crate::testing::bounds(&tree, "Matching publishers", Role::ListBox).expect("the list");
    let key = harness
        .text_rects
        .iter()
        // The field above the list holds the same key as its text.
        .find(|(text, rect)| text == HARBOR_PRESS && rect.top() >= list.top())
        .cloned()
        .expect("the key is painted");
    let mark = harness
        .text_rects
        .iter()
        .find(|(text, _)| text == "· current")
        .cloned()
        .expect("the mark is painted");
    assert!(
        list.contains_rect(key.1) && list.contains_rect(mark.1),
        "{key:?} {mark:?} {list:?}"
    );
    assert!(key.1.right() <= mark.1.left(), "{key:?} {mark:?}");
}

/// A list that goes on past the rows drawn says how far.
#[test]
fn a_list_that_goes_on_says_how_many_more() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    select(&mut harness, tab, id, 0);
    let names: Vec<String> = (0..20).map(|index| format!("Annex {index}")).collect();
    search_annexes(&mut harness, &names);
    assert_eq!(picker(&harness, tab, id).unwrap().options.len(), 20);
    assert!(harness.has("12 more"));
    // Fewer rows drawn in a short window: more are hidden.
    let (mut harness, tab, id) = covers_sized(egui::vec2(1280.0, 500.0), 3);
    select(&mut harness, tab, id, 0);
    search_annexes(&mut harness, &names);
    let tree = harness.settle();
    let drawn = tree
        .nodes
        .iter()
        .filter(|(_, node)| node.role() == Role::ListBoxOption)
        .count();
    assert!(drawn < 8);
    assert!(harness.has(&format!("{} more", 20 - drawn)));
}

/// A list of two has no such line, and one at the limit says so in the
/// words it has.
#[test]
fn a_list_that_fits_or_is_at_its_limit_has_no_more_line() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    select(&mut harness, tab, id, 0);
    type_on_cell(&mut harness, "harb");
    let tree = harness.settle();
    let more = |tree: &egui::accesskit::TreeUpdate| {
        crate::testing::labels(tree)
            .into_iter()
            .any(|label| label.ends_with(" more"))
    };
    assert!(!more(&tree));
    harness.press(Key::Escape, Modifiers::NONE);

    select(&mut harness, tab, id, 0);
    search_annexes(&mut harness, &annexes());
    assert!(harness.has("First 50 shown. Keep typing to narrow."));
    let tree = harness.settle();
    assert!(!more(&tree));
}

/// Opens the row panel's editor on `publisher_id` of the page's row `row`
/// from `start`, the panel showing, and answers what the picker asks.
fn edit_field(harness: &mut Harness, tab: ConnTabId, id: TabId, row: usize, start: EditStart) {
    let cell = CellPos {
        row,
        col: PUBLISHER,
    };
    harness.app.apply(Action::SelectCell { tab, id, cell });
    harness.app.apply(Action::FocusFields { tab, id });
    harness.app.apply(Action::EditField {
        tab,
        id,
        cell,
        start,
    });
    harness.answer_structure(publishers_structure());
    harness.answer_lookups(&PUBLISHERS);
}

/// Test 22: the row panel's field opens the same list, and a pick there
/// marks the grid's cell pending.
#[test]
fn the_row_panels_field_opens_the_same_list() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    edit_field(&mut harness, tab, id, 0, EditStart::Replace(String::new()));
    type_text(&mut harness, tab, id, "harb");
    let tree = harness.settle();
    // The panel's field wraps: a text field of several lines to egui.
    assert!(
        crate::testing::node(
            &tree,
            "Search publishers for publisher_id",
            Role::MultilineTextInput
        )
        .is_some(),
        "{:?}",
        crate::testing::labels(&tree)
    );
    assert!(crate::testing::node(&tree, "Matching publishers", Role::ListBox).is_some());
    assert_eq!(listed(&harness, tab, id), [HARBOR_PRESS, HARBOR_LIGHTS]);
    // Enter in the panel takes the option and walks on to the next field:
    // the keyboard is asked for it, and the selection is its cell.
    commit(&mut harness, tab, id, Advance::NextField);
    assert!(!editing(&harness, tab, id));
    assert_eq!(pending(&harness, tab, id, 0).as_deref(), Some(HARBOR_PRESS));
    assert_eq!(selection(&harness, tab, id), (0, KIND));
    let tree = harness.settle();
    assert!(crate::testing::node(&tree, "Matching publishers", Role::ListBox).is_none());
    // What a screen reader is told of the pick, whichever field made it.
    assert!(harness.has("publisher_id set to 9100000000000000004, Harbor Press"));
}

/// Spec 5: a window that gives the keyboard to another app is not an
/// editor that was left. The search is kept as no value: the editor, its
/// list and the cell are as they were, in the grid and in the row panel,
/// and with the keyboard back Enter takes the option.
#[test]
fn a_search_stays_open_while_another_window_has_the_keyboard() {
    for look in [Look::macos(), Look::omarchy()] {
        for panel in [false, true] {
            let (mut harness, tab, id) = covers_in(look);
            let start = EditStart::Replace(String::new());
            if panel {
                edit_field(&mut harness, tab, id, 0, start);
            } else {
                edit(&mut harness, tab, id, 0, start);
            }
            type_text(&mut harness, tab, id, "harb");
            harness.settle();
            harness.window_focus(false);
            let said = format!("{}, panel: {panel}", look.name);
            assert!(editing(&harness, tab, id), "{said}");
            assert_eq!(pending(&harness, tab, id, 0), None, "{said}");
            assert_eq!(
                listed(&harness, tab, id),
                [HARBOR_PRESS, HARBOR_LIGHTS],
                "{said}"
            );
            let tree = harness.settle();
            let list = crate::testing::node(&tree, "Matching publishers", Role::ListBox);
            assert!(list.is_some(), "{said}");
            harness.window_focus(true);
            harness.press(Key::Enter, Modifiers::NONE);
            assert!(!editing(&harness, tab, id), "{said}");
            assert_eq!(
                pending(&harness, tab, id, 0).as_deref(),
                Some(HARBOR_PRESS),
                "{said}"
            );
        }
    }
}

/// The keys reach the panel's list as they reach a cell's: Down moves the
/// highlight and Enter takes the option it is on.
#[test]
fn the_keys_reach_the_panels_list_while_its_field_has_the_keyboard() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    edit_field(&mut harness, tab, id, 0, EditStart::Replace(String::new()));
    type_text(&mut harness, tab, id, "harb");
    harness.settle();
    harness.press(Key::ArrowDown, Modifiers::NONE);
    assert_eq!(picker(&harness, tab, id).unwrap().highlight, Some(1));
    harness.press(Key::Enter, Modifiers::NONE);
    assert!(!editing(&harness, tab, id));
    assert_eq!(
        pending(&harness, tab, id, 0).as_deref(),
        Some(HARBOR_LIGHTS)
    );
}

/// A click on an option of the panel's list takes it, as a mouse does it.
/// The field gives the keyboard up on that click, and the pick goes ahead
/// of the leave: the text typed is not first kept.
#[test]
fn a_click_takes_an_option_of_the_panels_list() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    edit_field(&mut harness, tab, id, 0, EditStart::Replace(String::new()));
    type_text(&mut harness, tab, id, "harb");
    pointer_click(
        &mut harness,
        "Harbor Lights, 9100000000000000007",
        Role::ListBoxOption,
    );
    assert!(!editing(&harness, tab, id));
    assert_eq!(
        pending(&harness, tab, id, 0).as_deref(),
        Some(HARBOR_LIGHTS)
    );
}

/// Esc in the panel's picker leaves the field as it was, in both looks:
/// the terminal look's Esc keeps a text field's edit, and not a search.
#[test]
fn esc_closes_the_panels_picker_in_both_looks() {
    for look in [Look::macos(), Look::omarchy()] {
        let (mut harness, tab, id) = covers_in(look);
        edit_field(&mut harness, tab, id, 0, EditStart::Replace(String::new()));
        type_text(&mut harness, tab, id, "harb");
        harness.settle();
        harness.press(Key::Escape, Modifiers::NONE);
        assert!(!editing(&harness, tab, id), "{}", look.name);
        assert_eq!(pending(&harness, tab, id, 0), None, "{}", look.name);
    }
}

/// Spec 8: the "New row" form's field picks as any field does, and the
/// pick fills what the row needed.
#[test]
fn the_new_row_forms_field_picks() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    harness.app.apply(Action::AddRow {
        tab,
        id,
        place: crate::edit::Place::Top,
    });
    // Adding the row opened the grid's editor on its first required
    // cell: the form's field is edited instead.
    harness.app.apply(Action::CancelEdit { tab, id });
    let new = crate::edit::new_row(0);
    edit_field(
        &mut harness,
        tab,
        id,
        new,
        EditStart::Replace(String::new()),
    );
    type_text(&mut harness, tab, id, "harb");
    // The form's field draws its list, as the row panel's does.
    let tree = harness.settle();
    assert!(
        crate::testing::node(&tree, "Matching publishers", Role::ListBox).is_some(),
        "{:?}",
        crate::testing::labels(&tree)
    );
    commit(&mut harness, tab, id, Advance::NextField);
    assert_eq!(
        pending(&harness, tab, id, new).as_deref(),
        Some(HARBOR_PRESS)
    );
    assert!(harness.app.lacking(tab, id).is_empty());
}

/// Spec 2: opened on its value by a key, the panel's field has its text
/// selected on macOS, and not in the terminal look: what is typed next
/// replaces the key in the first, and goes after it in the second.
#[test]
fn the_panels_field_selects_its_text_where_the_grids_does() {
    for (look, selects) in [(Look::macos(), true), (Look::omarchy(), false)] {
        let (mut harness, tab, id) = covers_in(look);
        // Row 1 points at Harbor Press (`keyed_covers_page`).
        edit_field(&mut harness, tab, id, 1, EditStart::Value);
        let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
        let editor = object.edits.editor.as_ref().unwrap();
        assert!(editor.picker.is_some());
        assert_eq!(editor.select, selects, "{}", look.name);
        let loaded = editor.text.clone();
        assert_eq!(loaded, HARBOR_PRESS, "{}", look.name);
        harness.settle();
        harness.frame(vec![egui::Event::Text("7".into())]);
        harness.settle();
        let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
        let text = &object.edits.editor.as_ref().unwrap().text;
        if selects {
            assert_eq!(text, "7", "{}", look.name);
        } else {
            // The cursor is at the end of the key, where the field opened.
            assert_eq!(*text, format!("{loaded}7"), "{}", look.name);
        }
    }
}

/// An Enter kept for the answer is dropped when another tab is shown, for
/// an editor opened in the row panel as for one on a cell.
#[test]
fn a_kept_enter_of_the_panels_field_is_dropped_when_another_tab_is_shown() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    edit_field(&mut harness, tab, id, 0, EditStart::Typed("l".into()));
    set_text(&mut harness, tab, id, "harb");
    commit(&mut harness, tab, id, Advance::NextField);
    assert!(picker(&harness, tab, id).unwrap().kept.is_some());
    harness.app.apply(Action::OpenObject {
        tab,
        object: tabletist_db::ObjectRef::new("main", "publishers"),
        kind: tabletist_db::ObjectKind::Table,
        pin: true,
    });
    assert_ne!(harness.app.active_workspace_tab(), Some((tab, id)));
    harness.answer_lookups(&PUBLISHERS);
    assert!(editing(&harness, tab, id), "the hidden tab's editor closed");
    assert_eq!(pending(&harness, tab, id, 0), None);
    assert_eq!(picker(&harness, tab, id).unwrap().kept, None);
}

/// A press between the options, on the list's foot, is the list's too: the
/// panel's editor stays open with its text, and Enter still takes.
#[test]
fn a_press_beside_the_options_of_the_panels_list_keeps_the_editor_and_its_text() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    edit_field(&mut harness, tab, id, 0, EditStart::Replace(String::new()));
    type_text(&mut harness, tab, id, "harb");
    pointer_click(&mut harness, "Esc cancels", Role::Label);
    assert!(editing(&harness, tab, id));
    assert!(picker(&harness, tab, id).is_some());
    assert_eq!(pending(&harness, tab, id, 0), None);
    harness.press(Key::Enter, Modifiers::NONE);
    assert!(!editing(&harness, tab, id));
    assert_eq!(pending(&harness, tab, id, 0).as_deref(), Some(HARBOR_PRESS));
}

/// The panel's field says what the column refuses only after a refused
/// commit, in the panel's flow under the field with the label over it still
/// readable, and the list hangs under the words.
#[test]
fn the_panels_field_says_what_the_column_refuses_only_after_a_refused_commit() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    edit_field(&mut harness, tab, id, 0, EditStart::Replace(String::new()));
    type_text(&mut harness, tab, id, "zzz");
    let note = "INTEGER expects a whole number";
    assert!(!harness.has(note), "said while it is a search");
    harness.press(Key::Enter, Modifiers::NONE);
    assert!(editing(&harness, tab, id));
    let tree = harness.settle();
    let said = crate::testing::bounds(&tree, note, Role::Label)
        .unwrap_or_else(|| panic!("not said: {:?}", crate::testing::labels(&tree)));
    let field = crate::testing::bounds(
        &tree,
        "Search publishers for publisher_id",
        Role::MultilineTextInput,
    )
    .expect("the field");
    let list =
        crate::testing::bounds(&tree, "Matching publishers", Role::ListBox).expect("the list");
    let label = crate::testing::bounds(&tree, "publisher_id \u{b7} INTEGER", Role::Label)
        .unwrap_or_else(|| panic!("no label: {:?}", crate::testing::labels(&tree)));
    // The words are under the field and the list under the words; the
    // label is over the field, whole.
    assert!(said.top() >= field.bottom() - 0.5, "{said:?} {field:?}");
    assert!(list.top() >= said.bottom() - 0.5, "{list:?} {said:?}");
    assert!(label.bottom() <= field.top() + 0.5, "{label:?} {field:?}");
    assert!(window_of(&harness).contains_rect(list), "{list:?}");
}

/// The panel's list is under its field and inside the window.
#[test]
fn the_panels_list_stands_under_its_field_and_inside_the_window() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    edit_field(&mut harness, tab, id, 0, EditStart::Replace(String::new()));
    type_text(&mut harness, tab, id, "harb");
    let tree = harness.settle();
    let field = crate::testing::bounds(
        &tree,
        "Search publishers for publisher_id",
        Role::MultilineTextInput,
    )
    .expect("the field");
    let list =
        crate::testing::bounds(&tree, "Matching publishers", Role::ListBox).expect("the list");
    assert!(list.top() >= field.bottom(), "{list:?} {field:?}");
    assert!(window_of(&harness).contains_rect(list), "{list:?}");
}

/// Set NULL from the panel's list sets the cell's NULL and closes the
/// editor.
#[test]
fn a_nullable_columns_panel_list_sets_null() {
    let (mut harness, tab, id) = nullable_covers();
    harness.settle();
    edit_field(&mut harness, tab, id, 0, EditStart::Replace(String::new()));
    type_text(&mut harness, tab, id, "harb");
    harness.click("Set NULL");
    assert!(!editing(&harness, tab, id));
    assert!(pending_null(&harness, tab, id));
}

/// No publisher has this id.
const NO_PUBLISHER: &str = "9100000000000000099";
/// Nor has this one.
const ALSO_NO_PUBLISHER: &str = "9100000000000000098";

/// The warnings that stand on the tab's table.
fn warned(harness: &Harness, tab: ConnTabId, id: TabId) -> Vec<((usize, usize), String)> {
    let workspace = harness.app.workspace(tab).unwrap();
    let object = workspace.object_tab(id).unwrap();
    let table = crate::edit::Table::of(workspace, object).unwrap();
    crate::key_picker::unmatched(&table, &object.object, &object.edits)
        .into_iter()
        .map(|found| (found.at, found.value))
        .collect()
}

/// Types `text` into `publisher_id` of the page's row `row` and keeps it
/// as typed, with every lookup answered: the picker's, then the question
/// the kept value raises.
fn keep(harness: &mut Harness, tab: ConnTabId, id: TabId, row: usize, text: &str) {
    edit(harness, tab, id, row, EditStart::Replace(String::new()));
    type_text(harness, tab, id, text);
    commit(harness, tab, id, Advance::Stay);
    harness.answer_lookups(&PUBLISHERS);
}

/// The doubt the tab holds on `publisher_id` of the page's row `row`.
fn doubt(harness: &Harness, tab: ConnTabId, id: TabId, row: usize) -> Option<&Doubt> {
    let object = harness.app.workspace(tab)?.object_tab(id)?;
    object.edits.doubts.get(&(row, PUBLISHER))
}

/// The questions asked so far: the pinned reads that no picker sent
/// (a picker's pinned read carries the label column).
fn questions(harness: &Harness) -> Vec<(RequestId, tabletist_db::Lookup)> {
    harness
        .lookups()
        .into_iter()
        .filter(|(_, _, lookup)| {
            matches!(lookup.part, LookupPart::Pinned(_)) && lookup.label.is_none()
        })
        .map(|(_, request, lookup)| (request, lookup))
        .collect()
}

/// As [`keep`], with the question left unanswered, and every other lookup
/// that waits left too: only the picker's reads of this editor are
/// answered, by their places. The publishers' columns are known.
fn ask(harness: &mut Harness, tab: ConnTabId, id: TabId, row: usize, text: &str) {
    let from = harness.lookups().len();
    open(harness, tab, id, row, EditStart::Replace(String::new()));
    set_text(harness, tab, id, text);
    harness.app.send_due_lookups(std::time::Instant::now());
    for at in from..harness.lookups().len() {
        harness.answer_lookup_at(at, &PUBLISHERS);
    }
    commit(harness, tab, id, Advance::Stay);
}

/// Answers the lookup `request` alone, as a table holding `rows` would.
fn answer_request(harness: &mut Harness, request: RequestId, rows: &[(i64, &str)]) {
    let at = harness
        .lookups()
        .iter()
        .position(|(_, sent, _)| *sent == request)
        .expect("the request was sent");
    harness.answer_lookup_at(at, rows);
}

/// Test 21: a typed key no row has leaves the cell pending and warned,
/// with Save offered. Changing the value drops the warning.
#[test]
fn a_typed_key_that_names_no_row_is_warned_about() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    let before = harness.lookups().len();
    keep(&mut harness, tab, id, 0, NO_PUBLISHER);
    assert_eq!(pending(&harness, tab, id, 0).as_deref(), Some(NO_PUBLISHER));
    // One read of that key alone, after the picker's own.
    let sent = harness.lookups();
    let last = &sent.last().unwrap().2;
    assert!(sent.len() > before);
    assert!(last.part == LookupPart::Pinned(Value::Int(9_100_000_000_000_000_099)));
    assert_eq!(last.limit, 1);
    assert_eq!(questions(&harness).len(), 1, "exactly one question");
    assert_eq!(
        warned(&harness, tab, id),
        [((0, PUBLISHER), NO_PUBLISHER.to_owned())]
    );
    // It holds nothing: the cell is ready, and a save is not blocked.
    assert!(harness.app.save_blocked(tab, id).is_none());

    // Another value: the warning was of the old one.
    keep(&mut harness, tab, id, 0, HARBOR_LIGHTS);
    assert!(warned(&harness, tab, id).is_empty());
    // And Harbor Lights is there: no warning of its own.
    assert!(doubt(&harness, tab, id, 0).is_none());
}

/// Test 21: a value taken from the list is not asked about, and neither
/// is one that is to fix.
#[test]
fn a_picked_value_and_a_value_to_fix_are_not_asked_about() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    edit(&mut harness, tab, id, 0, EditStart::Replace(String::new()));
    type_text(&mut harness, tab, id, "harb");
    let before = harness.lookups().len();
    commit(&mut harness, tab, id, Advance::Stay);
    assert_eq!(pending(&harness, tab, id, 0).as_deref(), Some(HARBOR_PRESS));
    assert_eq!(
        harness.lookups().len(),
        before,
        "a picked value was asked about"
    );

    // `harb` kept by leaving the editor: an integer column refuses it,
    // and a text that is to fix names no key to ask about. This half
    // shows little: `key_picker::pinned` answers `None` for `harb` in an
    // integer column whatever `close_editor` does, and no fixture has a
    // text key that pins and is to fix (a text column with a length
    // limit would be one). It shows only that no question is sent and no
    // doubt is left for a value to fix.
    edit(&mut harness, tab, id, 1, EditStart::Replace(String::new()));
    type_text(&mut harness, tab, id, "harb");
    let before = harness.lookups().len();
    let leave = crate::testing::leave_edit(&harness.app, tab, id);
    harness.app.apply(leave);
    assert_eq!(harness.lookups().len(), before);
    let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
    assert!(object.edits.doubts.is_empty());
}

/// Spec 7: an answer for a value the cell no longer has is dropped, a
/// failed answer says nothing, and a question that nothing waits for is
/// forgotten, not cancelled: a pinned row's read is left to end.
#[test]
fn a_late_or_failed_answer_warns_of_nothing() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    edit(&mut harness, tab, id, 0, EditStart::Replace(String::new()));
    type_text(&mut harness, tab, id, NO_PUBLISHER);
    commit(&mut harness, tab, id, Advance::Stay);
    // Asked, not answered: nothing is said.
    assert!(matches!(
        doubt(&harness, tab, id, 0),
        Some(Doubt {
            state: DoubtState::Asked(_),
            ..
        })
    ));
    assert!(warned(&harness, tab, id).is_empty());
    // The database refuses the read: no warning, and no question left.
    harness.fail_lookup(tabletist_db::Error::query(
        "permission denied for table publishers",
    ));
    let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
    assert!(object.edits.doubts.is_empty());
    assert!(warned(&harness, tab, id).is_empty());

    // Asked again for another row, then discarded before the answer: the
    // question is forgotten with what was pending.
    edit(&mut harness, tab, id, 1, EditStart::Replace(String::new()));
    type_text(&mut harness, tab, id, NO_PUBLISHER);
    commit(&mut harness, tab, id, Advance::Stay);
    let asked = harness.lookups().last().unwrap().1;
    harness.app.apply(Action::DiscardEdits { tab, id });
    harness.app.send_due_lookups(std::time::Instant::now());
    assert!(!cancelled(&harness).contains(&asked));
    harness.answer_lookups(&PUBLISHERS);
    assert!(warned(&harness, tab, id).is_empty());
    let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
    assert!(object.edits.doubts.is_empty());
}

/// Test 21: a key that a new row of the same tab holds is not warned
/// about, whichever of the two was typed first.
#[test]
fn a_key_a_new_row_of_the_same_table_holds_is_not_warned_about() {
    use crate::testing::{GENRES, genres_structure};
    let mut harness = Harness::new();
    harness.set_look(Look::macos());
    let (tab, id) = harness.genres();
    harness.app.workspace_mut(tab).unwrap().pane = Pane::Grid;
    harness.settle();
    // Crime's parent becomes 99, and no genre has that id.
    harness.app.apply(Action::EditCell {
        tab,
        id,
        cell: CellPos { row: 1, col: 1 },
        start: EditStart::Replace("99".into()),
    });
    harness.answer_structure(genres_structure());
    harness.answer_lookups(&GENRES);
    commit(&mut harness, tab, id, Advance::Stay);
    harness.answer_lookups(&GENRES);
    assert_eq!(warned(&harness, tab, id), [((1, 1), "99".to_owned())]);
    // A new genre with the id 99: the save writes both.
    harness.app.apply(Action::AddRow {
        tab,
        id,
        place: crate::edit::Place::Top,
    });
    harness.app.apply(Action::CancelEdit { tab, id });
    let new = crate::edit::new_row(0);
    harness.app.apply(Action::EditCell {
        tab,
        id,
        cell: CellPos { row: new, col: 0 },
        start: EditStart::Replace("99".into()),
    });
    commit(&mut harness, tab, id, Advance::Stay);
    assert!(warned(&harness, tab, id).is_empty());
}

/// Test 21 in the row panel: its field closes through the same code, so a
/// key typed there is asked about and warned.
#[test]
fn a_key_typed_in_the_row_panels_field_is_asked_about() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    edit_field(&mut harness, tab, id, 0, EditStart::Replace(String::new()));
    type_text(&mut harness, tab, id, NO_PUBLISHER);
    commit(&mut harness, tab, id, Advance::NextField);
    harness.answer_lookups(&PUBLISHERS);
    assert_eq!(pending(&harness, tab, id, 0).as_deref(), Some(NO_PUBLISHER));
    assert_eq!(
        warned(&harness, tab, id),
        [((0, PUBLISHER), NO_PUBLISHER.to_owned())]
    );
}

/// An editor opened on a warned cell and closed with its text as it was
/// keeps the warning and asks nothing again: when nothing was typed, and
/// when the same text was typed over it.
#[test]
fn a_warned_cell_closed_on_the_same_text_keeps_its_warning_and_asks_nothing() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    keep(&mut harness, tab, id, 0, NO_PUBLISHER);
    let warning = [((0, PUBLISHER), NO_PUBLISHER.to_owned())];
    assert_eq!(warned(&harness, tab, id), warning);

    // Opened on the pending value and left as it was.
    edit(&mut harness, tab, id, 0, EditStart::Value);
    let before = harness.lookups().len();
    commit(&mut harness, tab, id, Advance::Stay);
    assert_eq!(harness.lookups().len(), before);
    assert_eq!(warned(&harness, tab, id), warning);

    // The same text typed over it: the cell is put again, not asked again.
    edit(&mut harness, tab, id, 0, EditStart::Replace(String::new()));
    type_text(&mut harness, tab, id, NO_PUBLISHER);
    let before = harness.lookups().len();
    commit(&mut harness, tab, id, Advance::Stay);
    assert_eq!(harness.lookups().len(), before);
    assert_eq!(pending(&harness, tab, id, 0).as_deref(), Some(NO_PUBLISHER));
    assert_eq!(warned(&harness, tab, id), warning);
}

/// An editor closed with the loaded text reverts the pending cell, and no
/// doubt is left that a later value could match.
#[test]
fn a_cell_reverted_to_its_loaded_text_leaves_no_doubt() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    keep(&mut harness, tab, id, 0, NO_PUBLISHER);
    assert!(doubt(&harness, tab, id, 0).is_some());
    // Row 0 loaded Northlight's key. Leaving keeps the text without the
    // picker's say, as typed.
    edit(&mut harness, tab, id, 0, EditStart::Replace(String::new()));
    type_text(&mut harness, tab, id, NORTHLIGHT);
    let before = harness.lookups().len();
    let leave = crate::testing::leave_edit(&harness.app, tab, id);
    harness.app.apply(leave);
    assert_eq!(pending(&harness, tab, id, 0), None);
    assert!(doubt(&harness, tab, id, 0).is_none());
    assert_eq!(
        harness.lookups().len(),
        before,
        "a reverted cell was asked about"
    );
    assert!(warned(&harness, tab, id).is_empty());
}

/// Two cells asked about at once, answered in the other order: each answer
/// finds its own cell.
#[test]
fn answers_in_the_other_order_find_their_own_cells() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    // Row 0 is kept as typed, row 1 by leaving the editor: a key that a
    // row has, asked about all the same.
    edit(&mut harness, tab, id, 0, EditStart::Replace(String::new()));
    type_text(&mut harness, tab, id, NO_PUBLISHER);
    commit(&mut harness, tab, id, Advance::Stay);
    // The picker of row 1 is answered alone: the question of row 0 waits.
    let asked = harness.lookups().len();
    open(&mut harness, tab, id, 1, EditStart::Replace(String::new()));
    set_text(&mut harness, tab, id, HARBOR_LIGHTS);
    harness.app.send_due_lookups(std::time::Instant::now());
    for at in asked..harness.lookups().len() {
        harness.answer_lookup_at(at, &PUBLISHERS);
    }
    let leave = crate::testing::leave_edit(&harness.app, tab, id);
    harness.app.apply(leave);
    // The questions are the pinned reads that ask for no label.
    let sent = harness.lookups();
    let questions: Vec<usize> = (0..sent.len())
        .filter(|&at| {
            matches!(sent[at].2.part, LookupPart::Pinned(_)) && sent[at].2.label.is_none()
        })
        .collect();
    let [first, second] = questions[..] else {
        panic!("two questions expected: {questions:?}");
    };
    assert!(sent[first].2.part == LookupPart::Pinned(Value::Int(9_100_000_000_000_000_099)));
    assert!(sent[second].2.part == LookupPart::Pinned(Value::Int(9_100_000_000_000_000_007)));
    // The second question answers first.
    harness.answer_lookup_at(second, &PUBLISHERS);
    assert!(doubt(&harness, tab, id, 1).is_none());
    assert!(matches!(
        doubt(&harness, tab, id, 0),
        Some(Doubt {
            state: DoubtState::Asked(_),
            ..
        })
    ));
    harness.answer_lookup_at(first, &PUBLISHERS);
    assert_eq!(
        warned(&harness, tab, id),
        [((0, PUBLISHER), NO_PUBLISHER.to_owned())]
    );
    // Both missing, answered the other way round: both warned.
    keep(&mut harness, tab, id, 1, ALSO_NO_PUBLISHER);
    assert_eq!(warned(&harness, tab, id).len(), 2);
}

/// A typed full key the picker's own pinned read found is taken from the
/// list: Enter takes option 0, now or once the answer comes, and the key is
/// not asked about.
#[test]
fn a_typed_key_the_pickers_own_read_found_is_not_asked_about() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    edit(&mut harness, tab, id, 0, EditStart::Replace(String::new()));
    type_text(&mut harness, tab, id, HARBOR_LIGHTS);
    let before = harness.lookups().len();
    commit(&mut harness, tab, id, Advance::Stay);
    assert_eq!(
        pending(&harness, tab, id, 0).as_deref(),
        Some(HARBOR_LIGHTS)
    );
    assert_eq!(harness.lookups().len(), before);
    assert!(doubt(&harness, tab, id, 0).is_none());

    // Enter before the answer: kept, and applied by the answer.
    // Row 2 loaded Harbor Lights: Harbor Press is a change there.
    edit(&mut harness, tab, id, 2, EditStart::Replace(String::new()));
    set_text(&mut harness, tab, id, HARBOR_PRESS);
    harness.app.send_due_lookups(std::time::Instant::now());
    commit(&mut harness, tab, id, Advance::Stay);
    assert!(editing(&harness, tab, id));
    harness.answer_lookups(&PUBLISHERS);
    assert!(!editing(&harness, tab, id));
    let sent = harness.lookups().len();
    assert_eq!(pending(&harness, tab, id, 2).as_deref(), Some(HARBOR_PRESS));
    assert!(doubt(&harness, tab, id, 2).is_none());
    harness.answer_lookups(&PUBLISHERS);
    assert_eq!(harness.lookups().len(), sent);
}

/// The Bookshop's publishers and one more, whose id is [`NO_PUBLISHER`]:
/// the row that came to exist after the warning.
const WITH_LATE: [(i64, &str); 4] = [
    PUBLISHERS[0],
    PUBLISHERS[1],
    PUBLISHERS[2],
    (9_100_000_000_000_000_099, "Late Books"),
];

/// Review: a warned key that is then taken from the list has no doubt.
/// The row came to exist, the key is typed again, the picker's own pinned
/// read finds it and Enter takes option 0.
#[test]
fn a_warned_key_taken_from_the_list_loses_its_warning() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    keep(&mut harness, tab, id, 0, NO_PUBLISHER);
    assert_eq!(warned(&harness, tab, id).len(), 1);
    open(&mut harness, tab, id, 0, EditStart::Replace(String::new()));
    harness.answer_lookups(&WITH_LATE);
    set_text(&mut harness, tab, id, NO_PUBLISHER);
    harness.app.send_due_lookups(std::time::Instant::now());
    harness.answer_lookups(&WITH_LATE);
    let asked = questions(&harness).len();
    commit(&mut harness, tab, id, Advance::Stay);
    assert_eq!(pending(&harness, tab, id, 0).as_deref(), Some(NO_PUBLISHER));
    assert!(warned(&harness, tab, id).is_empty());
    assert!(doubt(&harness, tab, id, 0).is_none());
    assert_eq!(questions(&harness).len(), asked, "a taken value was asked");
}

/// Spec 7: a value taken from the list is not asked about, and a kept one
/// once. Reopening the editor and closing it with no change is no new
/// value: a picked key, and a key answered "found", are not asked again.
#[test]
fn a_value_reopened_and_closed_unchanged_is_not_asked_again() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    // Picked.
    edit(&mut harness, tab, id, 0, EditStart::Replace(String::new()));
    type_text(&mut harness, tab, id, "harb");
    commit(&mut harness, tab, id, Advance::Stay);
    assert_eq!(pending(&harness, tab, id, 0).as_deref(), Some(HARBOR_PRESS));
    edit(&mut harness, tab, id, 0, EditStart::Value);
    commit(&mut harness, tab, id, Advance::Stay);
    // Kept by leaving, asked once, answered "found".
    edit(&mut harness, tab, id, 2, EditStart::Replace(String::new()));
    type_text(&mut harness, tab, id, NORTHLIGHT);
    let leave = crate::testing::leave_edit(&harness.app, tab, id);
    harness.app.apply(leave);
    harness.answer_lookups(&PUBLISHERS);
    assert_eq!(questions(&harness).len(), 1);
    assert!(doubt(&harness, tab, id, 2).is_none());
    // Reopened on the value and left, and the same key typed over it and
    // left: it already held this text, ready.
    edit(&mut harness, tab, id, 2, EditStart::Value);
    commit(&mut harness, tab, id, Advance::Stay);
    edit(&mut harness, tab, id, 2, EditStart::Replace(String::new()));
    type_text(&mut harness, tab, id, NORTHLIGHT);
    let leave = crate::testing::leave_edit(&harness.app, tab, id);
    harness.app.apply(leave);
    harness.answer_lookups(&PUBLISHERS);
    assert_eq!(questions(&harness).len(), 1, "a held key was asked again");
    assert_eq!(pending(&harness, tab, id, 2).as_deref(), Some(NORTHLIGHT));
}

/// A key pasted with white space around it is `Ready` (the column's check
/// trims it) and is asked about all the same, by the trimmed text. The
/// warning says the key without the space.
#[test]
fn a_key_with_white_space_around_it_is_asked_about() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    let spaced = format!(" {NO_PUBLISHER} ");
    keep(&mut harness, tab, id, 0, &spaced);
    assert_eq!(pending(&harness, tab, id, 0).as_deref(), Some(&*spaced));
    let asked = questions(&harness);
    assert_eq!(asked.len(), 1);
    assert!(asked[0].1.part == LookupPart::Pinned(Value::Int(9_100_000_000_000_000_099)));
    assert_eq!(
        warned(&harness, tab, id),
        [((0, PUBLISHER), NO_PUBLISHER.to_owned())]
    );
}

/// A question that comes back cancelled is asked again, once: the cancel
/// meant for the editor's matches read can land on it.
#[test]
fn a_question_cancelled_once_is_asked_again() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    edit(&mut harness, tab, id, 0, EditStart::Replace(String::new()));
    type_text(&mut harness, tab, id, NO_PUBLISHER);
    commit(&mut harness, tab, id, Advance::Stay);
    assert_eq!(questions(&harness).len(), 1);
    harness.fail_lookup(tabletist_db::Error::Cancelled);
    let asked = questions(&harness);
    assert_eq!(asked.len(), 2);
    assert!(asked[0].1.part == asked[1].1.part);
    assert!(warned(&harness, tab, id).is_empty());
    harness.answer_lookups(&PUBLISHERS);
    assert_eq!(
        warned(&harness, tab, id),
        [((0, PUBLISHER), NO_PUBLISHER.to_owned())]
    );
}

/// Cancelled twice is a failure like any other: no doubt, no third read.
#[test]
fn a_question_cancelled_twice_is_dropped() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    edit(&mut harness, tab, id, 0, EditStart::Replace(String::new()));
    type_text(&mut harness, tab, id, NO_PUBLISHER);
    commit(&mut harness, tab, id, Advance::Stay);
    harness.fail_lookup(tabletist_db::Error::Cancelled);
    harness.fail_lookup(tabletist_db::Error::Cancelled);
    assert_eq!(questions(&harness).len(), 2, "a third read was sent");
    assert!(doubt(&harness, tab, id, 0).is_none());
    assert!(warned(&harness, tab, id).is_empty());
}

/// A session replaced by a reconnect never answers the questions it was
/// asked: they are forgotten. What was answered stays.
#[test]
fn a_reconnect_forgets_the_questions_and_keeps_the_answers() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    keep(&mut harness, tab, id, 0, NO_PUBLISHER);
    edit(&mut harness, tab, id, 1, EditStart::Replace(String::new()));
    type_text(&mut harness, tab, id, ALSO_NO_PUBLISHER);
    commit(&mut harness, tab, id, Advance::Stay);
    assert!(matches!(
        doubt(&harness, tab, id, 1),
        Some(Doubt {
            state: DoubtState::Asked(_),
            ..
        })
    ));
    harness.app.apply(Action::Reconnect(tab));
    assert!(doubt(&harness, tab, id, 1).is_none());
    assert!(matches!(
        doubt(&harness, tab, id, 0),
        Some(Doubt {
            state: DoubtState::Missing,
            ..
        })
    ));
}

/// A question overtaken by another value of the cell: its answer touches
/// neither the second doubt nor the warning, and the second answer warns
/// of the second text.
#[test]
fn an_overtaken_question_warns_of_nothing_and_leaves_the_next_alone() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    edit(&mut harness, tab, id, 0, EditStart::Replace(String::new()));
    type_text(&mut harness, tab, id, NO_PUBLISHER);
    commit(&mut harness, tab, id, Advance::Stay);
    let (first, _) = questions(&harness)[0].clone();
    ask(&mut harness, tab, id, 0, ALSO_NO_PUBLISHER);
    let (second, _) = questions(&harness)[1].clone();
    answer_request(&mut harness, first, &PUBLISHERS);
    assert!(warned(&harness, tab, id).is_empty());
    assert_eq!(
        doubt(&harness, tab, id, 0).map(|doubt| (doubt.text.clone(), doubt.state)),
        Some((ALSO_NO_PUBLISHER.to_owned(), DoubtState::Asked(second)))
    );
    answer_request(&mut harness, second, &PUBLISHERS);
    assert_eq!(
        warned(&harness, tab, id),
        [((0, PUBLISHER), ALSO_NO_PUBLISHER.to_owned())]
    );
}

/// The answer of a tab that is not the one shown still lands.
#[test]
fn an_answer_for_a_tab_that_is_not_shown_still_lands() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    edit(&mut harness, tab, id, 0, EditStart::Replace(String::new()));
    type_text(&mut harness, tab, id, NO_PUBLISHER);
    commit(&mut harness, tab, id, Advance::Stay);
    harness.app.apply(Action::OpenObject {
        tab,
        object: ObjectRef::new("main", "publishers"),
        kind: tabletist_db::ObjectKind::Table,
        pin: true,
    });
    assert_ne!(harness.app.active_workspace_tab(), Some((tab, id)));
    harness.answer_lookups(&PUBLISHERS);
    assert_eq!(
        warned(&harness, tab, id),
        [((0, PUBLISHER), NO_PUBLISHER.to_owned())]
    );
}

/// A new row's foreign key cell is asked about and warned, and dropping
/// the row leaves no doubt.
#[test]
fn a_new_rows_key_is_asked_about_and_its_doubt_goes_with_the_row() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    harness.press(Key::N, Modifiers::COMMAND);
    harness.settle();
    let new = crate::edit::new_row(0);
    harness.answer_structure(publishers_structure());
    harness.answer_lookups(&PUBLISHERS);
    type_text(&mut harness, tab, id, NO_PUBLISHER);
    commit(&mut harness, tab, id, Advance::Stay);
    harness.answer_lookups(&PUBLISHERS);
    assert_eq!(
        warned(&harness, tab, id),
        [((new, PUBLISHER), NO_PUBLISHER.to_owned())]
    );
    harness.app.apply(Action::SelectCell {
        tab,
        id,
        cell: CellPos {
            row: new,
            col: PUBLISHER,
        },
    });
    harness.app.apply(Action::DropRow { tab, id });
    assert!(warned(&harness, tab, id).is_empty());
    let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
    assert!(object.edits.doubts.is_empty());
}

const WARNING: &str = "No publisher with id 9100000000000000099";

/// Test 21, what a user sees: the pending bar counts the value, Save is
/// offered, and the row panel's field says it under its value.
#[test]
fn the_warning_is_said_and_holds_no_save() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    keep(&mut harness, tab, id, 0, NO_PUBLISHER);
    harness.settle();
    assert!(harness.has("1 change in 1 row · 1 value names no row"));
    let tree = harness.settle();
    let save = crate::testing::node(&tree, "Save", Role::Button).expect("Save is offered");
    assert!(
        !tree
            .nodes
            .iter()
            .any(|(node, built)| *node == save && built.is_disabled())
    );
    // The row panel's field of the cell.
    harness.app.apply(Action::FocusFields { tab, id });
    harness.settle();
    assert!(harness.has(WARNING));

    // Two of them are counted as two.
    keep(&mut harness, tab, id, 1, NO_PUBLISHER);
    harness.settle();
    assert!(harness.has("2 changes in 2 rows · 2 values name no row"));
}

/// The warning is gone with the value it was about: the bar does not count
/// it and the panel does not say it.
#[test]
fn a_changed_value_takes_its_warning_off_the_bar_and_the_panel() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    keep(&mut harness, tab, id, 0, NO_PUBLISHER);
    harness.app.apply(Action::FocusFields { tab, id });
    assert!(harness.has(WARNING));
    keep(&mut harness, tab, id, 0, HARBOR_LIGHTS);
    let tree = harness.settle();
    let said = crate::testing::labels(&tree);
    assert!(said.iter().all(|label| !label.contains("names no row")));
    assert!(said.iter().all(|label| !label.contains(WARNING)));
}

/// The row panel's field says the warning in its name too, for a screen
/// reader, after what it holds.
#[test]
fn the_panels_field_names_the_warning() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    keep(&mut harness, tab, id, 0, NO_PUBLISHER);
    harness.app.apply(Action::FocusFields { tab, id });
    let tree = harness.settle();
    let named = tree.nodes.iter().any(|(_, node)| {
        node.role() == Role::Button
            && node.label().is_some_and(|label| {
                label.starts_with("publisher_id, ") && label.ends_with(WARNING)
            })
    });
    assert!(named, "{:?}", crate::testing::labels(&tree));
}

/// The terminal look says the first warning in the line under the grid,
/// in the pending tone, and a cell to fix comes first there.
#[test]
fn the_terminal_look_says_the_warning_under_the_grid() {
    let (mut harness, tab, id) = covers_in(Look::omarchy());
    keep(&mut harness, tab, id, 0, NO_PUBLISHER);
    harness.settle();
    assert!(harness.has("? 1:publisher_id  no publisher with id 9100000000000000099"));

    // Row 1 (the second) holds a text its column refuses: that is said
    // first, though the warning stands on.
    edit(&mut harness, tab, id, 1, EditStart::Replace(String::new()));
    type_text(&mut harness, tab, id, "harb");
    let leave = crate::testing::leave_edit(&harness.app, tab, id);
    harness.app.apply(leave);
    let tree = harness.settle();
    let lines: Vec<String> = crate::testing::labels(&tree)
        .into_iter()
        .filter(|label| label.starts_with("! ") || label.starts_with("? "))
        .collect();
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].starts_with("! 2:publisher_id  "), "{lines:?}");
}

/// A screen reader is told when a warning appears and when the count
/// changes: the count is a polite live region, in the pending bar and, in
/// the terminal look, over the line under the grid.
#[test]
fn the_count_of_warnings_is_a_polite_live_region() {
    let live = |tree: &egui::accesskit::TreeUpdate, label: &str| {
        tree.nodes
            .iter()
            .find(|(_, node)| node.label().or_else(|| node.value()) == Some(label))
            .map(|(_, node)| node.live())
    };
    let polite = Some(Some(egui::accesskit::Live::Polite));
    for look in [Look::macos(), Look::omarchy()] {
        let (mut harness, tab, id) = covers_in(look);
        let tree = harness.settle();
        assert_eq!(live(&tree, "1 value names no row"), None, "{}", look.name);
        keep(&mut harness, tab, id, 0, NO_PUBLISHER);
        let tree = harness.settle();
        assert_eq!(
            live(&tree, "1 value names no row"),
            polite,
            "{}: {:?}",
            look.name,
            crate::testing::labels(&tree)
        );
        keep(&mut harness, tab, id, 1, NO_PUBLISHER);
        let tree = harness.settle();
        assert_eq!(live(&tree, "2 values name no row"), polite, "{}", look.name);
        assert_eq!(live(&tree, "1 value names no row"), None, "{}", look.name);
    }
}

/// With room the bar draws the warning count among its others; where only
/// the short counts fit it draws none of it, and a screen reader is still
/// told the whole.
#[test]
fn the_bar_draws_the_warning_count_where_it_fits() {
    let whole = "1 change in 1 row · 1 value names no row";
    let (mut harness, tab, id) = covers_in(Look::macos());
    keep(&mut harness, tab, id, 0, NO_PUBLISHER);
    harness.settle();
    assert!(
        harness.painted_color(whole).is_some(),
        "{:?}",
        harness.painted
    );

    // A narrow window: the bar is cut to its short counts.
    let mut narrow = Harness::with_size(egui::vec2(760.0, 800.0));
    narrow.set_look(Look::macos());
    let (tab, id) = narrow.keyed_covers();
    narrow.app.workspace_mut(tab).unwrap().pane = Pane::Grid;
    keep(&mut narrow, tab, id, 0, NO_PUBLISHER);
    let tree = narrow.settle();
    assert!(narrow.painted_color(whole).is_none());
    assert!(
        narrow.painted_color("1 change").is_some()
            || narrow.painted_color("1 change in 1 row").is_some()
    );
    assert!(
        crate::testing::labels(&tree)
            .iter()
            .any(|label| label == whole)
    );
}

/// Hovers the pointer at `at` and waits past the tooltip's delay (a frame
/// is 1/60 s). Returns every name on screen then.
fn hover(harness: &mut Harness, at: egui::Pos2) -> Vec<String> {
    harness.frame(vec![egui::Event::PointerMoved(at)]);
    for _ in 0..60 {
        harness.frame(Vec::new());
    }
    crate::testing::labels(&harness.frame(Vec::new()))
}

/// What the warned cell says under the pointer, through the view: what the
/// cell was, then the words. In the terminal look the app's own words are
/// the look's lower case and the table, column and value are as they are.
#[test]
fn a_warned_cell_says_its_words_under_the_pointer() {
    for (look, words) in [
        (
            Look::macos(),
            "was 9100000000000000001\nNo publisher with id 9100000000000000099",
        ),
        (
            Look::omarchy(),
            "was 9100000000000000001\nno publisher with id 9100000000000000099",
        ),
    ] {
        let name = look.name;
        let (mut harness, tab, id) = covers_in(look);
        // The grid alone: the row panel says the value too.
        harness.app.workspace_mut(tab).unwrap().row_panel = false;
        keep(&mut harness, tab, id, 0, NO_PUBLISHER);
        harness.settle();
        let at = harness
            .painted_rect(NO_PUBLISHER)
            .unwrap_or_else(|| panic!("{name}: {:?}", harness.painted))
            .center();
        let said = hover(&mut harness, at);
        assert!(said.iter().any(|label| label == words), "{name}: {said:?}");
    }
}

/// The row panel's words are the look's too: lower case in the terminal
/// look, the value as it is.
#[test]
fn the_panels_words_are_the_looks() {
    let (mut harness, tab, id) = covers_in(Look::omarchy());
    harness.app.workspace_mut(tab).unwrap().row_panel = true;
    keep(&mut harness, tab, id, 0, NO_PUBLISHER);
    harness.app.apply(Action::FocusFields { tab, id });
    assert!(harness.has("no publisher with id 9100000000000000099"));
    assert!(!harness.has(WARNING));
}

/// A save that runs holds every warning's views back together: the bar's
/// count and its live region, the panel's words and name, the terminal's
/// line. A save that fails as a whole wrote nothing: the cell is ready as
/// it was, and they are all back.
#[test]
fn a_save_that_runs_takes_every_view_of_the_warning_with_it() {
    let names_no_row = |tree: &egui::accesskit::TreeUpdate| {
        let labels = crate::testing::labels(tree);
        labels.iter().any(|label| {
            let lower = label.to_lowercase();
            lower.contains("names no row") || lower.contains("no publisher with")
        })
    };
    for look in [Look::macos(), Look::omarchy()] {
        let name = look.name;
        let (mut harness, tab, id) = covers_in(look);
        keep(&mut harness, tab, id, 0, NO_PUBLISHER);
        harness.app.apply(Action::FocusFields { tab, id });
        let tree = harness.settle();
        assert!(names_no_row(&tree), "{name}: the warning stands");
        harness.app.apply(Action::WriteEdits { tab, id });
        let tree = harness.settle();
        assert!(
            harness
                .app
                .workspace(tab)
                .unwrap()
                .object_tab(id)
                .unwrap()
                .edits
                .saving
                .is_some(),
            "{name}: a save runs"
        );
        assert!(
            !names_no_row(&tree),
            "{name}: {:?}",
            crate::testing::labels(&tree)
        );
        // It fails as a whole: nothing was written, the cell is ready as it
        // was and its warning is back, every view of it together.
        harness.answer_written(Err(tabletist_db::Error::query("the server went away")));
        let tree = harness.settle();
        let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
        assert!(object.edits.saving.is_none(), "{name}");
        assert_eq!(
            object.edits.cells[&(0, PUBLISHER)].state,
            crate::edit::State::Ready,
            "{name}"
        );
        assert!(names_no_row(&tree), "{name}: back after the failure");
        // The count too, which a screen reader is told: on the bar, and in
        // the terminal look over the line under the grid.
        assert!(harness.has("1 value names no row"), "{name}");
    }
}

/// The terminal's line says the warning in the pending tone, as the line
/// of a cell to fix is in the failure's.
#[test]
fn the_terminal_lines_warning_is_in_the_pending_tone() {
    let (mut harness, tab, id) = covers_in(Look::omarchy());
    keep(&mut harness, tab, id, 0, NO_PUBLISHER);
    harness.settle();
    let palette = harness.app.palette;
    let line = "? 1:publisher_id  no publisher with id 9100000000000000099";
    assert_eq!(harness.painted_color(line), Some(palette.warning));
    // A cell to fix comes first, in the failure's tone, and the count of
    // warnings is still told.
    edit(&mut harness, tab, id, 1, EditStart::Replace(String::new()));
    type_text(&mut harness, tab, id, "harb");
    let leave = crate::testing::leave_edit(&harness.app, tab, id);
    harness.app.apply(leave);
    let tree = harness.settle();
    let first = harness
        .painted
        .iter()
        .find(|(text, _)| text.starts_with("! 2:publisher_id"))
        .map(|(_, color)| *color);
    assert_eq!(first, Some(palette.danger));
    assert!(harness.painted_color(line).is_none());
    let count = tree
        .nodes
        .iter()
        .find(|(_, node)| node.value() == Some("1 value names no row"))
        .map(|(_, node)| node.live());
    assert_eq!(count, Some(Some(egui::accesskit::Live::Polite)));
}

/// A new row's warning, through the view: the terminal's line names the
/// row `new`, the bar counts the value, and the new row's form says it.
#[test]
fn a_new_rows_warning_is_said_by_the_line_the_bar_and_the_form() {
    for look in [Look::macos(), Look::omarchy()] {
        let name = look.name;
        let (mut harness, tab, id) = covers_in(look);
        harness.app.apply(Action::AddRow {
            tab,
            id,
            place: crate::edit::Place::Top,
        });
        harness.app.apply(Action::CancelEdit { tab, id });
        let new = crate::edit::new_row(0);
        let cell = CellPos {
            row: new,
            col: PUBLISHER,
        };
        harness.app.apply(Action::EditCell {
            tab,
            id,
            cell,
            start: EditStart::Replace(NO_PUBLISHER.into()),
        });
        harness.answer_structure(publishers_structure());
        harness.answer_lookups(&PUBLISHERS);
        commit(&mut harness, tab, id, Advance::Stay);
        harness.answer_lookups(&PUBLISHERS);
        assert_eq!(
            warned(&harness, tab, id),
            [((new, PUBLISHER), NO_PUBLISHER.to_owned())],
            "{name}"
        );
        let tree = harness.settle();
        let labels = crate::testing::labels(&tree);
        let said = |text: &str| labels.iter().any(|label| label == text);
        if look.terminal {
            assert!(
                said("? new:publisher_id  no publisher with id 9100000000000000099"),
                "{name}: {labels:?}"
            );
        } else {
            assert!(
                said("1 new row · 1 value names no row"),
                "{name}: {labels:?}"
            );
            // The new row's form, the panel on the row.
            harness.app.apply(Action::SelectCell { tab, id, cell });
            harness.app.apply(Action::FocusFields { tab, id });
            assert!(harness.has(WARNING), "{name}: the form");
        }
    }
}

/// The panel's words are not drawn while its field is edited: the list is
/// what is under it then.
#[test]
fn the_panels_words_go_while_its_field_is_edited() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    keep(&mut harness, tab, id, 0, NO_PUBLISHER);
    harness.app.apply(Action::FocusFields { tab, id });
    assert!(harness.has(WARNING));
    edit_field(&mut harness, tab, id, 0, EditStart::Value);
    let tree = harness.settle();
    assert!(editing(&harness, tab, id));
    assert!(
        !crate::testing::labels(&tree)
            .iter()
            .any(|label| label == WARNING),
        "{:?}",
        crate::testing::labels(&tree)
    );
    assert!(
        crate::testing::node(&tree, "Matching publishers", Role::ListBox).is_some(),
        "the list"
    );
}
