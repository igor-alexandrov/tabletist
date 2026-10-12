//! The label of a foreign key's picker, checked against its spec
//! (`docs/superpowers/specs/2026-10-12-foreign-key-label-design.md`:
//! "test N" below is a test of its list), on the Bookshop's `book_covers`,
//! `publishers` and `genres`.
//!
//! Only behaviour, names and words are checked. No colour, size or
//! picture of the design is.

use egui::{Key, Modifiers};
use tabletist_db::ObjectRef;

use crate::backend::{Command, Event, RequestId, StateFile};
use crate::connections::ConnectionId;
use crate::key_picker::{Columns, Picker};
use crate::labels::{LabelChoice, Resolved, Source};
use crate::model::{Action, CellPos, ConnTabId, EditStart, Pane, TabId};
use crate::testing::{GENRES, Harness, PUBLISHERS, publishers_structure};
use crate::theme::Look;

/// `publisher_id` in `book_covers`, and `parent_id` in `genres`, by their
/// place in the page.
const KEY: usize = 1;

fn publishers() -> ObjectRef {
    ObjectRef::new("main", "publishers")
}

fn genres() -> ObjectRef {
    ObjectRef::new("main", "genres")
}

fn column(name: &str) -> LabelChoice {
    LabelChoice::Column(name.into())
}

/// What a picker is labelled by: the column, and who picked it.
fn labelled(column: Option<&str>, source: Source, automatic: Option<&str>) -> Resolved {
    Resolved {
        column: column.map(str::to_owned),
        source,
        automatic: automatic.map(str::to_owned),
        gone: None,
    }
}

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

fn connection(harness: &Harness, tab: ConnTabId) -> ConnectionId {
    harness.app.workspace(tab).unwrap().conn_id.clone()
}

/// Keeps `choice` for `table` on the saved connection of `tab`, as the
/// file holds it at a start.
fn keep(harness: &mut Harness, tab: ConnTabId, table: &ObjectRef, choice: LabelChoice) {
    let conn = connection(harness, tab);
    harness.app.labels.choose(&conn, table, choice);
}

/// Opens the grid's editor on the foreign key cell of the page's row
/// `row`, typed into, and answers nothing.
fn open(harness: &mut Harness, tab: ConnTabId, id: TabId, row: usize) {
    let cell = CellPos { row, col: KEY };
    let start = EditStart::Typed("h".into());
    harness.app.apply(Action::EditCell {
        tab,
        id,
        cell,
        start,
    });
}

/// Opens it as [`open`] does, and answers what a picker of `publishers`
/// asks: the table's columns, then its lookups.
fn edit(harness: &mut Harness, tab: ConnTabId, id: TabId, row: usize) {
    open(harness, tab, id, row);
    harness.answer_structure(publishers_structure());
    harness.answer_lookups(&PUBLISHERS);
}

fn picker(harness: &Harness, tab: ConnTabId, id: TabId) -> &Picker {
    let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
    let editor = object.edits.editor.as_ref().expect("an editor is open");
    editor.picker.as_ref().expect("the editor has a picker")
}

/// The label column each lookup sent so far names.
fn searched(harness: &Harness) -> Vec<Option<String>> {
    let sent = harness.lookups();
    sent.into_iter()
        .map(|(_, _, lookup)| lookup.label)
        .collect()
}

/// The label stores the app sent to be saved.
fn saved(harness: &Harness) -> Vec<crate::labels::Labels> {
    let path = harness.app.dirs.labels_file();
    harness
        .app
        .backend
        .sent
        .iter()
        .filter_map(|command| match command {
            Command::Save {
                path: to,
                file: StateFile::Labels(labels),
            } if *to == path => Some(labels.clone()),
            _ => None,
        })
        .collect()
}

/// Test 1: with nothing chosen the label is the app's pick, as it was
/// before there was a choice, and the picker says whose pick it is.
#[test]
fn automatic_is_the_apps_pick_and_the_picker_says_whose_it_is() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    edit(&mut harness, tab, id, 1);
    let by_app = labelled(Some("name"), Source::Automatic, Some("name"));
    assert!(picker(&harness, tab, id).label == Columns::Known(by_app.clone()));
    assert_eq!(harness.app.label_of(tab, &publishers(), "id"), Some(by_app));
    let name = Some("name".to_owned());
    assert!(!searched(&harness).is_empty());
    assert!(searched(&harness).iter().all(|label| *label == name));
    // Nothing was chosen: nothing is written.
    assert!(saved(&harness).is_empty());
}

/// Test 3, the grid's editor: a kept choice labels the picker, and the
/// picker says the user made it.
#[test]
fn a_kept_choice_labels_the_picker_as_the_users() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    keep(&mut harness, tab, &publishers(), column("name"));
    edit(&mut harness, tab, id, 1);
    let by_user = labelled(Some("name"), Source::User, Some("name"));
    assert!(picker(&harness, tab, id).label == Columns::Known(by_user));
}

/// Test 3: the choice is the table's, so the row panel's field and a new
/// row's cell are labelled by it as the grid's editor is.
#[test]
fn a_choice_labels_every_key_that_points_at_the_table() {
    let by_user = Columns::Known(labelled(None, Source::User, Some("name")));
    // The row panel's field.
    let (mut harness, tab, id) = covers_in(Look::macos());
    keep(&mut harness, tab, &publishers(), LabelChoice::NoLabel);
    let cell = CellPos { row: 0, col: KEY };
    harness.app.apply(Action::SelectCell { tab, id, cell });
    harness.app.apply(Action::FocusFields { tab, id });
    let start = EditStart::Replace(String::new());
    harness.app.apply(Action::EditField {
        tab,
        id,
        cell,
        start,
    });
    harness.answer_structure(publishers_structure());
    harness.answer_lookups(&PUBLISHERS);
    assert!(picker(&harness, tab, id).label == by_user);
    assert!(searched(&harness).iter().all(Option::is_none));
    // A new row's cell: adding the row opens the editor on it.
    let (mut harness, tab, id) = covers_in(Look::macos());
    keep(&mut harness, tab, &publishers(), LabelChoice::NoLabel);
    harness.press(Key::N, Modifiers::COMMAND);
    harness.settle();
    harness.answer_structure(publishers_structure());
    harness.answer_lookups(&PUBLISHERS);
    assert!(picker(&harness, tab, id).label == by_user);
    assert!(!searched(&harness).is_empty());
    assert!(searched(&harness).iter().all(Option::is_none));
}

/// Test 3: a choice is one saved connection's. Another connection's, and
/// another table's, change nothing here.
#[test]
fn a_choice_is_its_saved_connections_alone() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    let other = ConnectionId("another-connection".into());
    assert_ne!(other, connection(&harness, tab));
    let none = LabelChoice::NoLabel;
    harness
        .app
        .labels
        .choose(&other, &publishers(), none.clone());
    keep(&mut harness, tab, &genres(), none);
    edit(&mut harness, tab, id, 1);
    let by_app = labelled(Some("name"), Source::Automatic, Some("name"));
    assert!(picker(&harness, tab, id).label == Columns::Known(by_app));
}

/// A column that is no text can be the label: `genres` listed by the
/// number of each genre's parent. The search names that column.
#[test]
fn a_number_column_can_label_a_table() {
    let mut harness = Harness::new();
    harness.set_look(Look::macos());
    let (tab, id) = harness.genres();
    keep(&mut harness, tab, &genres(), column("parent_id"));
    // The table points at itself: its columns came with its own tab.
    open(&mut harness, tab, id, 1);
    harness.answer_lookups(&GENRES);
    let by_user = labelled(Some("parent_id"), Source::User, Some("name"));
    assert!(picker(&harness, tab, id).label == Columns::Known(by_user));
    let parent = Some("parent_id".to_owned());
    assert!(!searched(&harness).is_empty());
    assert!(searched(&harness).iter().all(|label| *label == parent));
}

/// Test 9, what is kept: a chosen column the table no longer has is found
/// when its columns are read. The choice goes back to Automatic, the
/// picker is labelled by the app's pick, and the column's name is kept to
/// be said once. (Saying it is run 2's.)
#[test]
fn a_chosen_column_that_is_gone_goes_back_to_automatic() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    // `publishers` has no such column: it was renamed since it was chosen.
    keep(&mut harness, tab, &publishers(), column("legal_name"));
    edit(&mut harness, tab, id, 1);
    let by_app = labelled(Some("name"), Source::Automatic, Some("name"));
    assert!(picker(&harness, tab, id).label == Columns::Known(by_app));
    let name = Some("name".to_owned());
    assert!(searched(&harness).iter().all(|label| *label == name));
    let conn = connection(&harness, tab);
    let labels = &harness.app.labels;
    assert_eq!(labels.choice(&conn, &publishers()), LabelChoice::Automatic);
    assert_eq!(labels.gone(&conn, &publishers()), Some("legal_name"));
    // Written once, with the choice dropped.
    assert_eq!(saved(&harness), [harness.app.labels.clone()]);
}

/// A column is called gone only by columns that are fresh: while the
/// table's columns are being read again the picker waits, and the choice
/// stays until their answer says the column is not there.
#[test]
fn a_column_is_not_called_gone_while_the_columns_are_being_read_again() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    edit(&mut harness, tab, id, 1);
    harness.app.apply(Action::CancelEdit { tab, id });
    keep(&mut harness, tab, &publishers(), column("legal_name"));
    // A refresh of the columns is on its way.
    let request = RequestId(u64::MAX);
    let workspace = harness.app.workspace_mut(tab).unwrap();
    let session = workspace.session;
    workspace
        .columns
        .get_mut(&publishers())
        .unwrap()
        .start(request);
    let sent = harness.lookups().len();
    open(&mut harness, tab, id, 1);
    assert!(picker(&harness, tab, id).label == Columns::Waiting);
    assert_eq!(harness.lookups().len(), sent);
    let conn = connection(&harness, tab);
    assert_eq!(
        harness.app.labels.choice(&conn, &publishers()),
        column("legal_name")
    );
    assert!(saved(&harness).is_empty());
    // The answer: the column is not among them.
    harness.app.apply(Action::Backend(Event::Structure {
        session,
        request,
        result: Ok(publishers_structure()),
    }));
    let by_app = labelled(Some("name"), Source::Automatic, Some("name"));
    assert!(picker(&harness, tab, id).label == Columns::Known(by_app));
    assert!(harness.lookups().len() > sent);
    assert_eq!(
        harness.app.labels.gone(&conn, &publishers()),
        Some("legal_name")
    );
}

/// Nor by a read that failed: the columns it leaves are the ones from
/// before it.
#[test]
fn a_read_that_failed_calls_no_column_gone() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    edit(&mut harness, tab, id, 1);
    harness.app.apply(Action::CancelEdit { tab, id });
    keep(&mut harness, tab, &publishers(), column("legal_name"));
    let request = RequestId(u64::MAX);
    let workspace = harness.app.workspace_mut(tab).unwrap();
    let session = workspace.session;
    workspace
        .columns
        .get_mut(&publishers())
        .unwrap()
        .start(request);
    harness.app.apply(Action::Backend(Event::Structure {
        session,
        request,
        result: Err(tabletist_db::Error::query("the read was refused")),
    }));
    let conn = connection(&harness, tab);
    assert_eq!(
        harness.app.labels.choice(&conn, &publishers()),
        column("legal_name")
    );
    assert!(saved(&harness).is_empty());
}

/// A saved connection that is deleted takes its choices with it.
#[test]
fn deleting_a_connection_forgets_its_choices() {
    let (mut harness, tab, _) = covers_in(Look::macos());
    keep(&mut harness, tab, &publishers(), column("name"));
    let conn = connection(&harness, tab);
    harness.app.apply(Action::DeleteConnection(conn.clone()));
    assert_eq!(
        harness.app.labels.choice(&conn, &publishers()),
        LabelChoice::Automatic
    );
    assert_eq!(saved(&harness), [crate::labels::Labels::default()]);
}
