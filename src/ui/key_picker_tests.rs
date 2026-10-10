//! The foreign key picker, checked against its spec
//! (`docs/superpowers/specs/2026-10-10-foreign-key-editing-design.md`:
//! "test N" below is a test of its list, "spec N" a section of it), on
//! the Bookshop's `book_covers` and `publishers`.
//!
//! Only behaviour, names and words are checked. No colour, size or
//! picture of the design is.

use std::time::{Duration, Instant};

use tabletist_db::{LookupPart, Value};

use crate::backend::{Command, Event, RequestId};
use crate::key_picker::{Picker, Take};
use crate::model::{Action, CellPos, ConnTabId, EditStart, Pane, TabId};
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

/// Spec 3: a search is sent after a pause in the typing, not on each
/// keystroke.
#[test]
fn a_search_waits_for_a_pause_in_the_typing() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    harness.app.lookup_delay = Duration::from_millis(150);
    edit(&mut harness, tab, id, 0, EditStart::Typed("h".into()));
    set_text(&mut harness, tab, id, "ha");
    let now = Instant::now();
    let sent = harness.lookups().len();
    let wait = harness.app.send_due_lookups(now);
    assert_eq!(harness.lookups().len(), sent, "sent before the pause");
    // A few microseconds went by since the reducer took its own time.
    let wait = wait.expect("a search is due later");
    assert!(wait <= Duration::from_millis(150), "{wait:?}");
    assert!(wait > Duration::from_millis(100), "{wait:?}");
    let wait = harness
        .app
        .send_due_lookups(now + Duration::from_millis(200));
    assert_eq!(wait, None);
    assert_eq!(harness.lookups().len(), sent + 1);
}

/// The frame sends what is due itself, and asks to be drawn again when the
/// next search is. The delay is shorter than the 83 ms a focused text field
/// asks for anyway (its cursor), so the request is the search's.
#[test]
fn a_frame_sends_what_is_due_and_asks_to_come_back() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    harness.app.lookup_delay = DELAY;
    edit(&mut harness, tab, id, 0, EditStart::Typed("h".into()));
    harness.finish_animations();
    let quiet = harness.repaint_after;
    set_text(&mut harness, tab, id, "ha");
    let sent = harness.lookups().len();
    // One frame: the pause is short, and many frames could outlast it.
    harness.frame(Vec::new());
    assert_eq!(harness.lookups().len(), sent, "sent before the pause");
    assert!(
        harness.repaint_after <= DELAY,
        "{:?}",
        harness.repaint_after
    );
    // Nothing was due before the text changed: the request is the search's.
    assert!(quiet > DELAY, "{quiet:?}");
    // The pause went by.
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
    harness.frame(Vec::new());
    assert_eq!(harness.lookups().len(), sent + 1);
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
