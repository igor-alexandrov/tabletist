//! The foreign key picker, checked against its spec
//! (`docs/superpowers/specs/2026-10-10-foreign-key-editing-design.md`:
//! "test N" below is a test of its list, "spec N" a section of it), on
//! the Bookshop's `book_covers` and `publishers`.
//!
//! Only behaviour, names and words are checked. No colour, size or
//! picture of the design is.

use std::time::{Duration, Instant};

use tabletist_db::{LookupPart, NewValue, ObjectRef, Value};

use crate::backend::{Command, Event, RequestId};
use crate::key_picker::{Columns, Picker, Take};
use crate::model::{Action, Advance, CellPos, ConnTabId, EditStart, Pane, TabId};
use crate::testing::{Harness, PUBLISHERS, publishers_structure};
use crate::theme::Look;

const NORTHLIGHT: &str = "9100000000000000001";
const HARBOR_PRESS: &str = "9100000000000000004";
const HARBOR_LIGHTS: &str = "9100000000000000007";
/// `publisher_id`, by its place in the page.
const PUBLISHER: usize = 1;
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

/// A pick in a new row's cell leaves that cell set.
#[test]
fn a_pick_in_a_new_rows_cell_sets_it() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    harness.app.apply(Action::AddRow {
        tab,
        id,
        place: crate::edit::Place::Top,
    });
    harness.app.apply(Action::CancelEdit { tab, id });
    let new = harness
        .app
        .workspace(tab)
        .unwrap()
        .object_tab(id)
        .unwrap()
        .edits
        .added
        .last()
        .unwrap()
        .id;
    let row = crate::edit::new_row(new);
    harness.app.apply(Action::EditCell {
        tab,
        id,
        cell: CellPos {
            row,
            col: PUBLISHER,
        },
        start: EditStart::Typed("h".into()),
    });
    harness.answer_structure(publishers_structure());
    harness.answer_lookups(&PUBLISHERS);
    type_text(&mut harness, tab, id, "harb");
    commit(&mut harness, tab, id, Advance::Stay);
    assert!(!editing(&harness, tab, id));
    assert_eq!(
        pending(&harness, tab, id, row).as_deref(),
        Some(HARBOR_PRESS)
    );
}

/// Spec spec 5: a pick that changes nothing (the cell already points at
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
