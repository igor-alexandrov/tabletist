//! The label of a foreign key's picker, checked against its spec
//! (`docs/superpowers/specs/2026-10-12-foreign-key-label-design.md`:
//! "test N" below is a test of its list), on the Bookshop's `book_covers`,
//! `publishers` and `genres`.
//!
//! Only behaviour, names and words are checked. No colour, size or
//! picture of the design is.

use egui::accesskit::Role;
use egui::{Key, Modifiers};
use tabletist_db::{LookupPart, ObjectRef};

use crate::backend::{Command, Event, RequestId, StateFile};
use crate::connections::ConnectionId;
use crate::key_picker::{Columns, Picker};
use crate::labels::{Chooser, Excluded, LabelChoice, Place, Refused, Resolved, Source};
use crate::model::{Action, CellPos, ConnTabId, EditStart, ObjectView, Pane, TabId};
use crate::testing::{GENRES, Harness, PUBLISHERS, node, publishers_structure};
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

/// Where the tables of `tab` are, for what is kept of their labels: its
/// saved connection, and the database it is on.
fn connection(harness: &Harness, tab: ConnTabId) -> Place {
    harness.app.workspace(tab).unwrap().label_place()
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
    let other = Place {
        conn: ConnectionId("another-connection".into()),
        database: String::new(),
    };
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

/// One PostgreSQL connection reaches several databases, and its tab is
/// switched from one to another. A choice made on one is not the other's:
/// the other's pickers are labelled by the app's pick, and a column the
/// other's table lacks is not gone from the first, whose choice stays.
#[test]
fn a_choice_made_on_one_database_is_not_anothers() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    // The connection as PostgreSQL's, on the database `app`, where the
    // publishers have a legal name.
    let workspace = harness.app.workspace_mut(tab).unwrap();
    workspace.driver = tabletist_db::Driver::Postgres;
    workspace.spec.database = "app".into();
    let app = connection(&harness, tab);
    assert_eq!(app.database, "app");
    keep(&mut harness, tab, &publishers(), column("legal_name"));
    // On `other`, as a switch leaves the tab, the publishers have none.
    harness.app.workspace_mut(tab).unwrap().spec.database = "other".into();
    let other = connection(&harness, tab);
    assert_ne!(other, app);
    edit(&mut harness, tab, id, 1);
    let by_app = labelled(Some("name"), Source::Automatic, Some("name"));
    assert!(picker(&harness, tab, id).label == Columns::Known(by_app));
    assert!(!harness.has("legal_name is gone, so labels use Automatic (name)."));
    let labels = &harness.app.labels;
    assert_eq!(labels.choice(&app, &publishers()), column("legal_name"));
    assert_eq!(labels.gone(&other, &publishers()), None);
    assert!(saved(&harness).is_empty());
    // The switch itself, back to the first: its choice is there. (An
    // open edit would be asked about first.)
    harness.press(Key::Escape, Modifiers::NONE);
    let database = "app".to_owned();
    harness.app.apply(Action::SwitchDatabase { tab, database });
    assert_eq!(connection(&harness, tab), app);
    let labels = &harness.app.labels;
    assert_eq!(labels.choice(&app, &publishers()), column("legal_name"));
}

/// Test 9: a chosen column the table no longer has is found when its
/// columns are read. The choice goes back to Automatic, the picker is
/// labelled by the app's pick, and the first picker to open says so: in
/// the list, and to a screen reader. It is said once: the next picker
/// says nothing, and the store holds nothing of it.
#[test]
fn a_chosen_column_that_is_gone_is_said_once() {
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
    // Told, so forgotten: the store is as if nothing had been chosen.
    assert_eq!(labels.gone(&conn, &publishers()), None);
    assert_eq!(*labels, crate::labels::Labels::default());
    assert_eq!(saved(&harness).last(), Some(&harness.app.labels));
    let said = "legal_name is gone, so labels use Automatic (name).";
    let told = "legal_name is gone. Labels use Automatic, name.";
    assert!(harness.has(said));
    assert_eq!(live(&mut harness, told), Some(true));
    // It stays for as long as this picker is open.
    harness.frame(vec![egui::Event::Text("a".into())]);
    assert!(harness.has(said));
    // The next picker says nothing.
    harness.app.apply(Action::CancelEdit { tab, id });
    edit(&mut harness, tab, id, 1);
    assert!(picker(&harness, tab, id).gone.is_none());
    assert!(!harness.has(said));
    assert_eq!(live(&mut harness, told), None);
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
    // The picker that waited is the one that says it.
    let gone = picker(&harness, tab, id).gone.as_ref();
    assert_eq!(gone.map(|(column, _)| column.as_str()), Some("legal_name"));
    assert_eq!(harness.app.labels.gone(&conn, &publishers()), None);
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
    harness
        .app
        .apply(Action::DeleteConnection(conn.conn.clone()));
    assert_eq!(
        harness.app.labels.choice(&conn, &publishers()),
        LabelChoice::Automatic
    );
    assert_eq!(saved(&harness), [crate::labels::Labels::default()]);
}

/// The words that say what labels a picker, by what it resolved to: the
/// notes board's, and in the Omarchy look its lower case round a name
/// that is left as it is.
#[test]
fn the_label_is_worded_by_its_column_and_who_chose_it() {
    use super::key_picker::label_line;
    let locale = crate::i18n::Locale::default();
    let said = |look: Look, label: Resolved| {
        let (said, column) = label_line(&Columns::Known(label), &look, locale).unwrap();
        // Where the name stands is where it reads.
        (said.clone(), column.map(|at| said[at].to_owned()))
    };
    let named = |words: &str| (words.to_owned(), Some("Name".to_owned()));
    let plain = |words: &str| (words.to_owned(), None);
    let by_app = labelled(Some("Name"), Source::Automatic, Some("Name"));
    let by_user = labelled(Some("Name"), Source::User, None);
    let nothing = labelled(None, Source::KeysOnly, None);
    assert_eq!(
        said(Look::macos(), by_app.clone()),
        named("Label Name · Automatic")
    );
    assert_eq!(
        said(Look::macos(), by_user.clone()),
        named("Label Name · Set by you")
    );
    assert_eq!(
        said(Look::macos(), nothing.clone()),
        plain("Label keys only")
    );
    assert_eq!(
        said(Look::omarchy(), by_app),
        named("label Name · automatic")
    );
    assert_eq!(
        said(Look::omarchy(), by_user),
        named("label Name · set by you")
    );
    assert_eq!(said(Look::omarchy(), nothing), plain("label keys only"));
    // Two the board does not draw: no column, and something to choose.
    let unpicked = labelled(None, Source::Automatic, None);
    let unlabelled = labelled(None, Source::User, Some("Name"));
    assert_eq!(
        said(Look::macos(), unpicked),
        plain("Label keys only · Automatic")
    );
    assert_eq!(
        said(Look::macos(), unlabelled),
        plain("Label keys only · Set by you")
    );
    // Nothing is said while the columns are not known.
    assert!(label_line(&Columns::Waiting, &Look::macos(), locale).is_none());
}

/// The macOS look says it in the list's foot, after the foot's own words,
/// which stay as they were: on a button, named for a screen reader as
/// the notes board names it.
#[test]
fn the_foot_names_the_label_and_who_chose_it() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    edit(&mut harness, tab, id, 1);
    assert!(harness.has("Esc cancels"));
    let tree = harness.settle();
    assert!(node(&tree, "Label column: name, Automatic", Role::Button).is_some());
    assert!(harness.painted_rect("Label name · Automatic").is_some());
    for (choice, said, name) in [
        (
            column("name"),
            "Label name · Set by you",
            "Label column: name, Set by you",
        ),
        (
            LabelChoice::NoLabel,
            "Label keys only · Set by you",
            "Label column: keys only, Set by you",
        ),
    ] {
        let (mut harness, tab, id) = covers_in(Look::macos());
        keep(&mut harness, tab, &publishers(), choice);
        edit(&mut harness, tab, id, 1);
        assert!(harness.has("Esc cancels"), "{said}");
        let tree = harness.settle();
        assert!(node(&tree, name, Role::Button).is_some(), "{name}");
        assert!(harness.painted_rect(said).is_some(), "{said}");
        assert!(harness.painted_rect("Label name · Automatic").is_none());
    }
}

/// The Omarchy look's list has no foot: the status line says it, in the
/// look's lower case.
#[test]
fn the_terminal_looks_status_line_names_the_label() {
    let (mut harness, tab, id) = covers_in(Look::omarchy());
    edit(&mut harness, tab, id, 1);
    assert!(harness.has("label name · automatic"));
    assert!(!harness.has("Esc cancels"));
    let (mut harness, tab, id) = covers_in(Look::omarchy());
    keep(&mut harness, tab, &publishers(), column("name"));
    edit(&mut harness, tab, id, 1);
    assert!(harness.has("label name · set by you"));
}

// The chooser: run 2.

/// What a real Ctrl press reports: outside macOS Ctrl is the command key
/// too.
fn ctrl() -> Modifiers {
    if cfg!(target_os = "macos") {
        Modifiers::CTRL
    } else {
        Modifiers::CTRL | Modifiers::COMMAND
    }
}

/// The chord that asks for the chooser, as `look` has it.
fn ask(harness: &mut Harness, look: Look) {
    let held = if look.terminal {
        ctrl()
    } else {
        Modifiers::COMMAND
    };
    harness.press(Key::L, held);
}

/// Selects the foreign key cell of the page's row `row`.
fn select(harness: &mut Harness, tab: ConnTabId, id: TabId, row: usize) {
    let cell = CellPos { row, col: KEY };
    harness.app.apply(Action::SelectCell { tab, id, cell });
    harness.settle();
}

/// Types `text` on the selected cell of `book_covers`, as a keyboard
/// does, and answers what the picker asks of `publishers`.
fn type_on_cell(harness: &mut Harness, text: &str) {
    harness.frame(vec![egui::Event::Text(text.into())]);
    harness.settle();
    harness.answer_structure(publishers_structure());
    harness.settle();
    harness.answer_lookups(&PUBLISHERS);
    harness.settle();
}

/// The covers in `look`, `text` typed on the first row's `publisher_id`
/// and its list answered.
fn typed_in(look: Look, text: &str) -> (Harness, ConnTabId, TabId) {
    let (mut harness, tab, id) = covers_in(look);
    select(&mut harness, tab, id, 0);
    if look.terminal {
        // Insert mode, on nothing.
        let cell = CellPos { row: 0, col: KEY };
        let start = EditStart::Replace(String::new());
        harness.app.apply(Action::EditCell {
            tab,
            id,
            cell,
            start,
        });
        harness.settle();
    }
    type_on_cell(&mut harness, text);
    (harness, tab, id)
}

fn chooser(harness: &Harness, tab: ConnTabId, id: TabId) -> Option<&Chooser> {
    picker(harness, tab, id).chooser.as_ref()
}

fn editing(harness: &Harness, tab: ConnTabId, id: TabId) -> bool {
    let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
    object.edits.editor.is_some()
}

/// The text the tab's editor holds.
fn text(harness: &Harness, tab: ConnTabId, id: TabId) -> String {
    let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
    object.edits.editor.as_ref().unwrap().text.clone()
}

/// Where the cursor of the tab's field is, in characters.
fn caret(harness: &Harness, tab: ConnTabId, id: TabId) -> Option<usize> {
    let field = crate::ui::cell_editor::field_id(tab, id);
    let state = egui::text_edit::TextEditState::load(&harness.ctx, field)?;
    Some(state.cursor.char_range()?.primary.index.into())
}

fn has_keyboard(harness: &Harness, tab: ConnTabId, id: TabId) -> bool {
    let field = crate::ui::cell_editor::field_id(tab, id);
    harness.ctx.memory(|memory| memory.has_focus(field))
}

/// Whether the cell of the page's row `row` has anything pending.
fn pending(harness: &Harness, tab: ConnTabId, id: TabId, row: usize) -> bool {
    let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
    object.edits.cells.contains_key(&(row, KEY))
}

/// Whether what is labelled `label` is in the tree, and a polite live
/// region.
fn live(harness: &mut Harness, label: &str) -> Option<bool> {
    let tree = harness.settle();
    let found = tree
        .nodes
        .iter()
        .find(|(_, node)| node.label().or_else(|| node.value()) == Some(label))?;
    Some(found.1.live() == Some(egui::accesskit::Live::Polite))
}

/// The key opens the chooser beside the list, on the line of what is
/// chosen. It is a menu of radio items named as the notes board names
/// them, and the field keeps the keyboard and its text.
#[test]
fn the_key_opens_the_chooser_and_the_field_keeps_the_keyboard() {
    let (mut harness, tab, id) = typed_in(Look::macos(), "harb");
    assert!(chooser(&harness, tab, id).is_none());
    ask(&mut harness, Look::macos());
    let open = chooser(&harness, tab, id).expect("the chooser is open");
    assert_eq!((open.at, &open.choice), (0, &LabelChoice::Automatic));
    assert!(!open.waited);
    let tree = harness.settle();
    assert!(node(&tree, "Label publishers by", Role::Menu).is_some());
    for name in [
        "Automatic, uses name",
        "No label, keys only",
        "name, TEXT",
        // Listed, and not to be chosen.
        "id, INTEGER, the key, unavailable",
    ] {
        assert!(node(&tree, name, Role::MenuItemRadio).is_some(), "{name}");
    }
    assert!(harness.has(
        "For publishers in Fixture · dev, on this computer. Nothing is written to the database."
    ));
    assert!(has_keyboard(&harness, tab, id));
    assert_eq!(text(&harness, tab, id), "harb");
    // Nothing was chosen, and nothing asked of the database.
    assert!(saved(&harness).is_empty());
}

/// Tests 2 and 4: choosing a column keeps the choice for the table on
/// the saved connection, searches again for the text that was typed, and
/// leaves the editor open with its text, its cursor and the cell's value
/// as they were. The highlight stays on the row it was on. Nothing is
/// sent to the database but that one search.
#[test]
fn choosing_a_column_keeps_the_edit_and_searches_once() {
    let (mut harness, tab, id) = typed_in(Look::macos(), "harb");
    // The cursor in the middle of the text, the highlight on the second
    // of the two rows found.
    harness.press(Key::ArrowLeft, Modifiers::NONE);
    harness.press(Key::ArrowDown, Modifiers::NONE);
    assert_eq!(caret(&harness, tab, id), Some(3));
    assert_eq!(picker(&harness, tab, id).highlight, Some(1));
    let before = harness.app.backend.sent.len();
    ask(&mut harness, Look::macos());
    // Automatic, No label, then `name`.
    harness.press(Key::ArrowDown, Modifiers::NONE);
    harness.press(Key::ArrowDown, Modifiers::NONE);
    assert_eq!(chooser(&harness, tab, id).map(|open| open.at), Some(2));
    harness.press(Key::Enter, Modifiers::NONE);
    assert!(chooser(&harness, tab, id).is_none());
    // The choice, kept and written.
    let conn = connection(&harness, tab);
    assert_eq!(
        harness.app.labels.choice(&conn, &publishers()),
        column("name")
    );
    assert_eq!(saved(&harness), [harness.app.labels.clone()]);
    // The edit, as it was.
    assert!(editing(&harness, tab, id));
    assert!(has_keyboard(&harness, tab, id));
    assert_eq!(text(&harness, tab, id), "harb");
    assert_eq!(caret(&harness, tab, id), Some(3));
    assert!(!pending(&harness, tab, id, 0));
    // What was sent since the key: the file to be written, and one search
    // for the same text by the chosen column. Nothing else.
    let since = &harness.app.backend.sent[before..];
    assert!(
        matches!(
            since,
            [
                Command::Save { file: StateFile::Labels(_), .. },
                Command::Lookup { lookup, .. },
            ] if lookup.label.as_deref() == Some("name")
                && matches!(&lookup.part, LookupPart::Matching(text) if text == "harb")
        ),
        "{} commands",
        since.len()
    );
    let by_user = labelled(Some("name"), Source::User, Some("name"));
    assert!(picker(&harness, tab, id).label == Columns::Known(by_user));
    // The answer: the highlight is on the row it was on.
    harness.answer_lookups(&PUBLISHERS);
    assert_eq!(picker(&harness, tab, id).highlight, Some(1));
    let tree = harness.settle();
    assert!(node(&tree, "Label column: name, Set by you", Role::Button).is_some());
    // Enter still takes that row: the choice touched no value until now.
    harness.press(Key::Enter, Modifiers::NONE);
    assert!(!editing(&harness, tab, id));
    assert!(pending(&harness, tab, id, 0));
}

/// Test 13: what a screen reader is told once the list is searched again,
/// in the notes board's words, with the count of what was found.
#[test]
fn a_choice_is_announced_with_the_count_of_its_matches() {
    // Three publishers' keys begin with 91.
    let (mut harness, tab, id) = typed_in(Look::macos(), "91");
    let choose = |harness: &mut Harness, line: usize| {
        ask(harness, Look::macos());
        let line = Some(line);
        harness.app.apply(Action::ChooseLabel { tab, id, line });
        harness.answer_lookups(&PUBLISHERS);
    };
    choose(&mut harness, 2);
    assert_eq!(
        live(&mut harness, "Labelled by name. 3 matches."),
        Some(true)
    );
    choose(&mut harness, 1);
    assert_eq!(live(&mut harness, "Keys only. 3 matches."), Some(true));
    assert_eq!(live(&mut harness, "Labelled by name. 3 matches."), None);
    choose(&mut harness, 0);
    let said = "Labelled by name, Automatic. 3 matches.";
    assert_eq!(live(&mut harness, said), Some(true));
    // Said of the list as it was: the next text typed ends it.
    harness.frame(vec![egui::Event::Text("0".into())]);
    assert_eq!(live(&mut harness, said), None);
}

/// Nothing is said before the search answers: the count is the answer's.
#[test]
fn a_choice_is_not_announced_before_its_search_answers() {
    let (mut harness, tab, id) = typed_in(Look::macos(), "91");
    ask(&mut harness, Look::macos());
    let line = Some(2);
    harness.app.apply(Action::ChooseLabel { tab, id, line });
    assert_eq!(live(&mut harness, "Labelled by name. 3 matches."), None);
    harness.answer_lookups(&PUBLISHERS);
    assert_eq!(
        live(&mut harness, "Labelled by name. 3 matches."),
        Some(true)
    );
}

/// Test 7: Esc with the chooser open closes the chooser alone, and the
/// text stays. The next Esc closes the picker.
#[test]
fn esc_closes_the_chooser_first_and_then_the_picker() {
    for look in [Look::macos(), Look::omarchy()] {
        let (mut harness, tab, id) = typed_in(look, "harb");
        ask(&mut harness, look);
        assert!(chooser(&harness, tab, id).is_some(), "{}", look.name);
        harness.press(Key::Escape, Modifiers::NONE);
        assert!(editing(&harness, tab, id), "{}", look.name);
        assert!(chooser(&harness, tab, id).is_none(), "{}", look.name);
        assert_eq!(text(&harness, tab, id), "harb", "{}", look.name);
        assert!(has_keyboard(&harness, tab, id), "{}", look.name);
        harness.press(Key::Escape, Modifiers::NONE);
        assert!(!editing(&harness, tab, id), "{}", look.name);
        assert!(!pending(&harness, tab, id, 0), "{}", look.name);
        assert!(saved(&harness).is_empty(), "{}", look.name);
    }
}

/// Test 8: under the picker every letter is the search's, `j` and `k`
/// among them. In the chooser, which has no field, they move, and so do
/// the terminal look's Ctrl+N and Ctrl+P. None of it reaches the text.
#[test]
fn letters_are_the_searchs_until_the_chooser_has_the_keys() {
    let look = Look::omarchy();
    let (mut harness, tab, id) = typed_in(look, "j");
    harness.frame(vec![egui::Event::Text("k".into())]);
    harness.settle();
    assert_eq!(text(&harness, tab, id), "jk");
    assert!(chooser(&harness, tab, id).is_none());
    ask(&mut harness, look);
    let at = |harness: &Harness| chooser(harness, tab, id).map(|open| open.at);
    assert_eq!(at(&harness), Some(0));
    for (typed, line) in [("j", 1), ("j", 2), ("j", 2), ("k", 1), ("x", 1), ("k", 0)] {
        harness.frame(vec![egui::Event::Text(typed.into())]);
        harness.settle();
        assert_eq!(at(&harness), Some(line), "{typed}");
    }
    harness.press(Key::N, ctrl());
    assert_eq!(at(&harness), Some(1));
    harness.press(Key::P, ctrl());
    assert_eq!(at(&harness), Some(0));
    assert_eq!(text(&harness, tab, id), "jk");
    // Enter chooses here, and commits nothing.
    harness.press(Key::Enter, Modifiers::NONE);
    assert!(chooser(&harness, tab, id).is_none());
    assert!(editing(&harness, tab, id));
    assert_eq!(text(&harness, tab, id), "jk");
}

/// In the looks whose keys are no letters a letter goes to the next
/// column it begins, and the arrows step over what cannot be chosen.
#[test]
fn a_letter_jumps_to_a_column_and_the_arrows_skip_the_key() {
    let (mut harness, tab, id) = typed_in(Look::macos(), "harb");
    ask(&mut harness, Look::macos());
    let at = |harness: &Harness| chooser(harness, tab, id).map(|open| open.at);
    harness.frame(vec![egui::Event::Text("n".into())]);
    harness.settle();
    assert_eq!(at(&harness), Some(2));
    // `id` is the key: no letter and no arrow goes to it.
    harness.frame(vec![egui::Event::Text("i".into())]);
    harness.settle();
    assert_eq!(at(&harness), Some(2));
    for _ in 0..3 {
        harness.press(Key::ArrowDown, Modifiers::NONE);
    }
    assert_eq!(at(&harness), Some(2));
    harness.press(Key::ArrowUp, Modifiers::NONE);
    assert_eq!(at(&harness), Some(1));
    assert_eq!(text(&harness, tab, id), "harb");
    // The picker's own highlight did not move under it.
    assert_eq!(picker(&harness, tab, id).highlight, Some(0));
}

/// Test 6, in the chooser: the key is listed and cannot be chosen, by a
/// click or by asking for its line.
#[test]
fn the_key_cannot_be_chosen_in_the_chooser() {
    let (mut harness, tab, id) = typed_in(Look::macos(), "harb");
    ask(&mut harness, Look::macos());
    harness.click("id, INTEGER, the key, unavailable");
    let line = Some(3);
    harness.app.apply(Action::ChooseLabel { tab, id, line });
    assert!(chooser(&harness, tab, id).is_some());
    assert!(saved(&harness).is_empty());
    let conn = connection(&harness, tab);
    assert_eq!(
        harness.app.labels.choice(&conn, &publishers()),
        LabelChoice::Automatic
    );
    // A line that can be chosen is chosen by a click.
    harness.click("No label, keys only");
    assert!(chooser(&harness, tab, id).is_none());
    assert_eq!(
        harness.app.labels.choice(&conn, &publishers()),
        LabelChoice::NoLabel
    );
    assert!(editing(&harness, tab, id));
    assert!(has_keyboard(&harness, tab, id));
    assert_eq!(text(&harness, tab, id), "harb");
}

/// The foot's button opens the chooser and closes it, and says which.
#[test]
fn the_foots_button_opens_and_closes_the_chooser() {
    let (mut harness, tab, id) = typed_in(Look::macos(), "harb");
    let button = "Label column: name, Automatic";
    let expanded = |harness: &mut Harness| {
        let tree = harness.settle();
        let id = node(&tree, button, Role::Button).expect("the button");
        let found = tree.nodes.iter().find(|(node, _)| *node == id);
        found.and_then(|(_, node)| node.is_expanded())
    };
    assert_eq!(expanded(&mut harness), Some(false));
    harness.click(button);
    assert!(chooser(&harness, tab, id).is_some());
    assert_eq!(expanded(&mut harness), Some(true));
    assert!(editing(&harness, tab, id));
    assert!(has_keyboard(&harness, tab, id));
    harness.click(button);
    assert!(chooser(&harness, tab, id).is_none());
    assert!(editing(&harness, tab, id));
    assert_eq!(text(&harness, tab, id), "harb");
}

/// `editions` in `look`, the editor open on the first row's `isbn` and
/// its list answered: a picker of `isbns`, a table of its key alone.
fn isbn_picker(look: Look) -> (Harness, ConnTabId, TabId) {
    let mut harness = Harness::new();
    harness.set_look(look);
    let (tab, id) = harness.editions();
    harness.app.workspace_mut(tab).unwrap().pane = Pane::Grid;
    harness.settle();
    // `isbn` stands where `publisher_id` does: second in its page.
    open(&mut harness, tab, id, 0);
    harness.answer_structure(crate::testing::isbns_structure());
    harness.answer_lookups(&[]);
    (harness, tab, id)
}

/// Test 10: a table of its key alone has nothing to choose. Its options
/// are keys, the button is disabled and says why, and the key opens no
/// chooser: the picker notes that it was asked, to say why for a while.
#[test]
fn a_table_of_its_key_alone_opens_no_chooser() {
    for look in [Look::macos(), Look::omarchy()] {
        let (mut harness, tab, id) = isbn_picker(look);
        let nothing = labelled(None, Source::KeysOnly, None);
        assert!(picker(&harness, tab, id).label == Columns::Known(nothing));
        assert!(searched(&harness).iter().all(Option::is_none));
        assert!(picker(&harness, tab, id).nothing.is_none());
        ask(&mut harness, look);
        assert!(chooser(&harness, tab, id).is_none(), "{}", look.name);
        assert!(picker(&harness, tab, id).nothing.is_some(), "{}", look.name);
        assert!(editing(&harness, tab, id), "{}", look.name);
        assert!(saved(&harness).is_empty(), "{}", look.name);
    }
    // The macOS look's button says it, to the eye and to a screen reader.
    let (mut harness, _, _) = isbn_picker(Look::macos());
    let name = "Label: keys only. isbns has only its key column.";
    let tree = harness.settle();
    assert!(node(&tree, name, Role::Button).is_some());
    assert!(harness.painted_rect("Label keys only").is_some());
    // Its key says why nothing opens, where the chooser would have.
    let why = "isbns has only its key column";
    assert!(harness.painted_rect(why).is_none());
    ask(&mut harness, Look::macos());
    harness.settle();
    assert!(harness.painted_rect(why).is_some(), "{:?}", harness.painted);
}

/// Test 11: while the table's columns are on their way the chooser offers
/// Automatic and No label, and a choice made then holds when they come.
#[test]
fn a_choice_can_be_made_before_the_columns_are_known() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    // The editor is open, and nothing has answered for `publishers`.
    open(&mut harness, tab, id, 0);
    assert!(picker(&harness, tab, id).label == Columns::Waiting);
    ask(&mut harness, Look::macos());
    assert_eq!(
        chooser(&harness, tab, id).map(|open| open.waited),
        Some(true)
    );
    let tree = harness.settle();
    assert!(node(&tree, "Automatic, checking…", Role::MenuItemRadio).is_some());
    assert!(node(&tree, "No label, keys only", Role::MenuItemRadio).is_some());
    assert!(harness.has("Loading columns…"));
    assert!(harness.has("Automatic and No label work now"));
    harness.press(Key::ArrowDown, Modifiers::NONE);
    // Nothing to move to after No label yet.
    harness.press(Key::ArrowDown, Modifiers::NONE);
    harness.press(Key::Enter, Modifiers::NONE);
    assert!(chooser(&harness, tab, id).is_none());
    let conn = connection(&harness, tab);
    assert_eq!(
        harness.app.labels.choice(&conn, &publishers()),
        LabelChoice::NoLabel
    );
    assert!(harness.lookups().is_empty());
    // The columns come: the picker is labelled by the choice.
    harness.answer_structure(publishers_structure());
    let by_user = labelled(None, Source::User, Some("name"));
    assert!(picker(&harness, tab, id).label == Columns::Known(by_user));
    assert!(!searched(&harness).is_empty());
    assert!(searched(&harness).iter().all(Option::is_none));
}

/// Columns that come while the chooser is open show in it, and a screen
/// reader is told.
#[test]
fn columns_that_arrive_show_in_the_open_chooser() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    open(&mut harness, tab, id, 0);
    ask(&mut harness, Look::macos());
    assert_eq!(live(&mut harness, "Columns loaded."), None);
    harness.answer_structure(publishers_structure());
    let tree = harness.settle();
    assert!(node(&tree, "Automatic, uses name", Role::MenuItemRadio).is_some());
    assert!(node(&tree, "name, TEXT", Role::MenuItemRadio).is_some());
    assert!(!harness.has("Loading columns…"));
    assert_eq!(live(&mut harness, "Columns loaded."), Some(true));
    assert!(chooser(&harness, tab, id).is_some());
}

/// A choice searches its own table's pickers again, and no other's.
#[test]
fn a_choice_for_another_table_searches_nothing() {
    let (mut harness, tab, id) = typed_in(Look::macos(), "harb");
    let sent = harness.lookups().len();
    assert!(harness.app.set_label(tab, &genres(), LabelChoice::NoLabel));
    assert_eq!(harness.lookups().len(), sent);
    assert!(picker(&harness, tab, id).changed.is_none());
    // Its own table's is searched once, and a choice that changes
    // nothing not at all.
    assert!(
        harness
            .app
            .set_label(tab, &publishers(), LabelChoice::NoLabel)
    );
    assert_eq!(harness.lookups().len(), sent + 1);
    assert!(
        !harness
            .app
            .set_label(tab, &publishers(), LabelChoice::NoLabel)
    );
    assert_eq!(harness.lookups().len(), sent + 1);
}

/// A picker that is open on a chosen column when the table's columns are
/// read again without it is labelled by what is left, searched again,
/// and is the one that says the column is gone.
#[test]
fn an_open_picker_is_labelled_again_when_its_column_goes() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    keep(&mut harness, tab, &publishers(), column("name"));
    edit(&mut harness, tab, id, 1);
    let sent = harness.lookups().len();
    // `name` was renamed to `title`, and the columns are read again.
    let request = RequestId(u64::MAX);
    let workspace = harness.app.workspace_mut(tab).unwrap();
    let session = workspace.session;
    let kept = workspace.columns.get_mut(&publishers()).unwrap();
    kept.start(request);
    let mut renamed = publishers_structure();
    renamed.columns[1].name = "title".into();
    harness.app.apply(Action::Backend(Event::Structure {
        session,
        request,
        result: Ok(renamed),
    }));
    let by_app = labelled(Some("title"), Source::Automatic, Some("title"));
    assert!(picker(&harness, tab, id).label == Columns::Known(by_app));
    let again = harness.lookups().split_off(sent);
    assert!(!again.is_empty());
    assert!(
        again
            .iter()
            .all(|(_, _, lookup)| lookup.label.as_deref() == Some("title"))
    );
    assert!(harness.has("name is gone, so labels use Automatic (title)."));
    // Nobody chose: nothing confirms a choice.
    assert!(picker(&harness, tab, id).changed.is_none());
    harness.answer_lookups(&PUBLISHERS);
    assert_eq!(
        live(&mut harness, "Labelled by title, Automatic. 3 matches."),
        None
    );
}

/// A list that holds only the first of its matches says so where it
/// confirms a choice, as it does everywhere else.
#[test]
fn a_choice_counts_a_list_that_goes_on_as_fifty_or_more() {
    let (mut harness, tab, id) = typed_in(Look::macos(), "91");
    ask(&mut harness, Look::macos());
    let line = Some(2);
    harness.app.apply(Action::ChooseLabel { tab, id, line });
    harness.answer_lookups(&PUBLISHERS);
    let object = harness.app.workspace_mut(tab).unwrap();
    let object = object.object_tab_mut(id).unwrap();
    let editor = object.edits.editor.as_mut().unwrap();
    editor.picker.as_mut().unwrap().more = true;
    let said = "Labelled by name. 50 or more matches.";
    assert_eq!(live(&mut harness, said), Some(true));
    assert_eq!(live(&mut harness, "Labelled by name. 3 matches."), None);
}

/// A search that fails has no list: the chooser beside it closes, and
/// the keys are the field's again.
#[test]
fn a_search_that_fails_closes_the_chooser() {
    let (mut harness, tab, id) = typed_in(Look::macos(), "harb");
    harness.frame(vec![egui::Event::Text("o".into())]);
    harness.app.send_due_lookups(std::time::Instant::now());
    ask(&mut harness, Look::macos());
    assert!(chooser(&harness, tab, id).is_some());
    harness.fail_lookup(tabletist_db::Error::query("the table is locked"));
    assert!(picker(&harness, tab, id).failed.is_some());
    assert!(chooser(&harness, tab, id).is_none());
    // Nor does one open there: its keys would be the field's.
    harness.app.apply(Action::ToggleLabelChooser { tab, id });
    assert!(chooser(&harness, tab, id).is_none());
    // So with columns that fail to come.
    let (mut harness, tab, id) = covers_in(Look::macos());
    open(&mut harness, tab, id, 0);
    ask(&mut harness, Look::macos());
    assert!(chooser(&harness, tab, id).is_some());
    let workspace = harness.app.workspace(tab).unwrap();
    let session = workspace.session;
    let request = workspace.columns[&publishers()].pending.unwrap();
    harness.app.apply(Action::Backend(Event::Structure {
        session,
        request,
        result: Err(tabletist_db::Error::query("no such table")),
    }));
    assert!(picker(&harness, tab, id).failed.is_some());
    assert!(chooser(&harness, tab, id).is_none());
}

/// Clicks the node `label` of `role` with the pointer, as a mouse does:
/// moved onto it, pressed and released in frames of their own.
fn pointer_click(harness: &mut Harness, label: &str, role: Role) {
    let tree = harness.settle();
    let at = crate::testing::bounds(&tree, label, role)
        .unwrap_or_else(|| panic!("nothing labelled {label:?}"))
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

/// A press beside the chooser closes it and does nothing else, as one
/// beside a menu does: the option it landed on is not taken, and the
/// edit goes on. The next click on that option takes it.
#[test]
fn a_press_beside_the_chooser_closes_it_and_picks_nothing() {
    let (mut harness, tab, id) = typed_in(Look::macos(), "harb");
    ask(&mut harness, Look::macos());
    let option = "Harbor Lights, 9100000000000000007";
    pointer_click(&mut harness, option, Role::ListBoxOption);
    assert!(chooser(&harness, tab, id).is_none());
    assert!(editing(&harness, tab, id));
    assert!(has_keyboard(&harness, tab, id));
    assert!(!pending(&harness, tab, id, 0));
    assert_eq!(text(&harness, tab, id), "harb");
    pointer_click(&mut harness, option, Role::ListBoxOption);
    assert!(!editing(&harness, tab, id));
    assert!(pending(&harness, tab, id, 0));
}

/// A press that closes the chooser and the picker with it (it landed on
/// the grid) leaves nothing behind: the first click on an option of the
/// next picker takes it.
#[test]
fn a_press_that_closed_the_picker_too_swallows_no_later_click() {
    let (mut harness, tab, id) = typed_in(Look::macos(), "harb");
    ask(&mut harness, Look::macos());
    // The press, beside the chooser, with the button left down; then the
    // editor is gone before the button comes up.
    let tree = harness.settle();
    let option = "Harbor Lights, 9100000000000000007";
    let at = crate::testing::bounds(&tree, option, Role::ListBoxOption).unwrap();
    let at = at.center();
    harness.frame(vec![egui::Event::PointerMoved(at)]);
    harness.frame(vec![egui::Event::PointerButton {
        pos: at,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::NONE,
    }]);
    assert!(chooser(&harness, tab, id).is_none());
    harness.app.apply(Action::CancelEdit { tab, id });
    harness.frame(vec![egui::Event::PointerButton {
        pos: at,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    }]);
    harness.settle();
    assert!(!editing(&harness, tab, id));
    // The next picker: one click takes its option.
    select(&mut harness, tab, id, 0);
    type_on_cell(&mut harness, "harb");
    pointer_click(&mut harness, option, Role::ListBoxOption);
    assert!(!editing(&harness, tab, id));
    assert!(pending(&harness, tab, id, 0));
}

// The Omarchy look's status line.

/// The Omarchy look says it in its status line, in its own words.
#[test]
fn the_terminal_look_says_a_gone_column_in_its_status_line() {
    let (mut harness, tab, id) = covers_in(Look::omarchy());
    keep(&mut harness, tab, &publishers(), column("legal_name"));
    edit(&mut harness, tab, id, 1);
    assert!(harness.has("legal_name is gone · labels use automatic (name)"));
    // In place of the label and the keys, which come back after it.
    assert!(!harness.has("label name · automatic"));
}

/// While the chooser is open the status line names it and its keys, as
/// the keymap has them.
#[test]
fn the_terminal_looks_status_line_names_the_chooser_and_its_keys() {
    let look = Look::omarchy();
    let (mut harness, tab, id) = typed_in(look, "harb");
    ask(&mut harness, look);
    assert!(chooser(&harness, tab, id).is_some());
    assert!(harness.has("label publishers by"));
    let keys = "j/k or ctrl+n/p move · enter choose · esc back";
    assert!(
        harness.painted_rect(keys).is_some(),
        "{:?}",
        harness.painted
    );
    // In place of the label and the picker's keys.
    assert!(!harness.has("label name · automatic"));
    harness.press(Key::Escape, Modifiers::NONE);
    assert!(harness.has("label name · automatic"));
    // Under the picker the chooser's key stands among the list's.
    let keys = "ctrl+n/p move · enter set · ctrl+l label · esc close";
    assert!(
        harness.painted_rect(keys).is_some(),
        "{:?}",
        harness.painted
    );
}

/// Test 10, in the Omarchy look: its key prints why there is no chooser.
#[test]
fn the_terminal_look_says_that_a_table_has_nothing_to_choose() {
    let look = Look::omarchy();
    let (mut harness, _, _) = isbn_picker(look);
    assert!(harness.has("label keys only"));
    // Its key is not offered: there is nothing for it to open.
    let keys = "ctrl+n/p move · enter set · esc close";
    assert!(
        harness.painted_rect(keys).is_some(),
        "{:?}",
        harness.painted
    );
    ask(&mut harness, look);
    assert!(harness.has("isbns has only its key column"));
}

/// A list that goes on says so in the line that confirms a choice, as the
/// field counts it.
#[test]
fn the_terminal_look_counts_a_list_that_goes_on_as_fifty_and_more() {
    let look = Look::omarchy();
    let (mut harness, tab, id) = typed_in(look, "91");
    ask(&mut harness, look);
    let line = Some(2);
    harness.app.apply(Action::ChooseLabel { tab, id, line });
    harness.answer_lookups(&PUBLISHERS);
    let object = harness.app.workspace_mut(tab).unwrap();
    let object = object.object_tab_mut(id).unwrap();
    let editor = object.edits.editor.as_mut().unwrap();
    editor.picker.as_mut().unwrap().more = true;
    assert!(harness.has("label name · 50+ matches"));
}

/// "Just changed", in the Omarchy look: the status line says what the
/// list is labelled by now and how many it found, under the mark of a
/// thing done, in place of its keys.
#[test]
fn the_terminal_look_confirms_a_choice_in_its_status_line() {
    let look = Look::omarchy();
    let (mut harness, tab, id) = typed_in(look, "91");
    ask(&mut harness, look);
    for _ in 0..2 {
        harness.frame(vec![egui::Event::Text("j".into())]);
        harness.settle();
    }
    harness.press(Key::Enter, Modifiers::NONE);
    // Not before the answer.
    assert!(!harness.has("label name · 3 matches"));
    harness.answer_lookups(&PUBLISHERS);
    assert!(harness.has("label name · 3 matches"));
    assert!(picker(&harness, tab, id).changed.is_some());
    // The next text typed is a new search: the line is the keys' again.
    harness.frame(vec![egui::Event::Text("0".into())]);
    harness.settle();
    assert!(!harness.has("label name · 3 matches"));
}

// The Structure door: run 3.

fn editions() -> ObjectRef {
    ObjectRef::new("main", "editions")
}

/// Shows the Structure view of the tab `id` and gives it the keys.
fn to_structure(harness: &mut Harness, tab: ConnTabId, id: TabId) {
    let view = ObjectView::Structure;
    let object_tab = id;
    harness.app.apply(Action::SetView {
        tab,
        object_tab,
        view,
    });
    harness.app.workspace_mut(tab).unwrap().pane = Pane::Grid;
    harness.settle();
}

/// `publishers` open in a tab of its own, in its Structure view, on a
/// connection that is read-only or is not. No cell is edited, and its
/// rows never came: the view needs none.
fn publishers_structure_in(look: Look, read_only: bool) -> (Harness, ConnTabId, TabId) {
    let mut harness = Harness::new();
    harness.set_look(look);
    let tab = harness.connect_fake_as(read_only);
    harness.app.apply(Action::OpenObject {
        tab,
        object: publishers(),
        kind: tabletist_db::ObjectKind::Table,
        pin: true,
    });
    harness.answer_structure(publishers_structure());
    let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
    to_structure(&mut harness, tab, id);
    (harness, tab, id)
}

/// `editions` in its Structure view: a key, a key to `isbns`, a title
/// and bytes.
fn editions_structure_in(look: Look) -> (Harness, ConnTabId, TabId) {
    let mut harness = Harness::new();
    harness.set_look(look);
    let (tab, id) = harness.editions();
    to_structure(&mut harness, tab, id);
    (harness, tab, id)
}

/// The chooser that is open in the Structure view of the tab.
fn tab_chooser(harness: &Harness, tab: ConnTabId, id: TabId) -> Option<&Chooser> {
    let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
    object.label_chooser.as_ref()
}

fn cursor(harness: &Harness, tab: ConnTabId, id: TabId) -> usize {
    let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
    object.structure_cursor
}

/// What is kept for `table` on the connection of `tab`.
fn kept(harness: &Harness, tab: ConnTabId, table: &ObjectRef) -> LabelChoice {
    harness.app.labels.choice(&connection(harness, tab), table)
}

/// Why the last column named for a label was not taken.
fn refused(harness: &Harness, tab: ConnTabId) -> Option<(String, Refused)> {
    let noted = harness.app.workspace(tab).unwrap().label_refused.clone();
    noted.map(|(_, name, why)| (name, why))
}

/// Runs `line` at the terminal look's prompt.
fn run(harness: &mut Harness, tab: ConnTabId, line: &str) {
    harness.app.apply(Action::OpenCommand(tab));
    harness.app.workspace_mut(tab).unwrap().command = Some(line.to_owned());
    harness.app.apply(Action::RunCommand(tab));
    harness.settle();
}

/// A key that types `text`, as a keyboard sends it: the press with its
/// text, then the release.
fn type_key(harness: &mut Harness, key: Key, text: &str) {
    harness.settle();
    harness.frame(vec![
        crate::testing::key(key, Modifiers::NONE),
        egui::Event::Text(text.into()),
    ]);
    harness.frame(vec![crate::testing::release(key, Modifiers::NONE)]);
    harness.settle();
}

/// Clicks the text `label` with the pointer's `button`, as a mouse does.
fn click_text(harness: &mut Harness, label: &str, button: egui::PointerButton) {
    let tree = harness.settle();
    let at = crate::testing::bounds(&tree, label, Role::Label)
        .unwrap_or_else(|| panic!("nothing reads {label:?}"))
        .center();
    let event = |pressed| egui::Event::PointerButton {
        pos: at,
        button,
        pressed,
        modifiers: Modifiers::NONE,
    };
    harness.frame(vec![egui::Event::PointerMoved(at)]);
    harness.frame(vec![event(true)]);
    harness.frame(vec![event(false)]);
    harness.settle();
}

/// Whether the tree has a node `label` that cannot be acted on.
fn disabled(harness: &mut Harness, label: &str) -> Option<bool> {
    let tree = harness.settle();
    let mut nodes = tree.nodes.iter();
    let found = nodes.find(|(_, node)| node.label() == Some(label))?;
    Some(found.1.is_disabled())
}

/// What a screen reader reads after the name of the node `label`.
fn described(harness: &mut Harness, label: &str) -> Option<String> {
    let tree = harness.settle();
    let mut nodes = tree.nodes.iter();
    let found = nodes.find(|(_, node)| node.label() == Some(label))?;
    found.1.description().map(str::to_owned)
}

/// A press and a release of the pointer's first button at `at`.
fn click_at(harness: &mut Harness, at: egui::Pos2) {
    let event = |pressed| egui::Event::PointerButton {
        pos: at,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    harness.frame(vec![egui::Event::PointerMoved(at)]);
    harness.frame(vec![event(true)]);
    harness.frame(vec![event(false)]);
    harness.settle();
}

/// Test 5: the Structure view of the table a key points at sets its
/// label, on a connection that opens read-only and with no cell edited.
/// The chooser is the picker's. Nothing is sent to the database: the
/// file is written, and that is all.
#[test]
fn the_structure_view_sets_the_label_on_a_read_only_connection() {
    let (mut harness, tab, id) = publishers_structure_in(Look::macos(), true);
    let access = harness.app.workspace(tab).unwrap().access;
    assert_eq!(access, tabletist_db::Access::ReadOnly);
    // The line is one thing for a screen reader.
    let tree = harness.settle();
    let line = "Label for foreign keys: name, Automatic";
    assert!(node(&tree, line, Role::Group).is_some());
    let help = "Fields that point at publishers show and search this column. \
                On this computer only; nothing is written to the database.";
    assert!(harness.has(help));
    // The column that labels is marked on its line.
    assert!(harness.has("name, label column"));
    let before = harness.app.backend.sent.len();
    harness.click("Change label column");
    let open = tab_chooser(&harness, tab, id).expect("the chooser is open");
    assert_eq!((open.at, &open.choice), (0, &LabelChoice::Automatic));
    let tree = harness.settle();
    assert!(node(&tree, "Label publishers by", Role::Menu).is_some());
    for name in [
        "Automatic, uses name",
        "No label, keys only",
        "name, TEXT",
        "id, INTEGER, the key, unavailable",
    ] {
        assert!(node(&tree, name, Role::MenuItemRadio).is_some(), "{name}");
    }
    assert!(harness.has(
        "For publishers in Fixture · dev, on this computer. Nothing is written to the database."
    ));
    // Its keys are the chooser's, with no field anywhere.
    harness.press(Key::ArrowDown, Modifiers::NONE);
    harness.press(Key::ArrowDown, Modifiers::NONE);
    harness.press(Key::Enter, Modifiers::NONE);
    assert!(tab_chooser(&harness, tab, id).is_none());
    assert_eq!(kept(&harness, tab, &publishers()), column("name"));
    assert!(harness.has("Label for foreign keys: name, Set by you"));
    // The way back stands before what changes it, as the board has them.
    let tree = harness.settle();
    let left = |name| {
        let button = crate::testing::bounds(&tree, name, Role::Button);
        button.unwrap_or_else(|| panic!("no button {name}")).left()
    };
    assert!(left("Use Automatic") < left("Change label column"));
    let since = &harness.app.backend.sent[before..];
    let written = |command: &Command| {
        matches!(
            command,
            Command::Save {
                file: StateFile::Labels(_),
                ..
            }
        )
    };
    assert!(since.iter().all(written), "{} commands", since.len());
    assert_eq!(saved(&harness), [harness.app.labels.clone()]);
    // And the way back.
    harness.click("Use Automatic");
    assert_eq!(kept(&harness, tab, &publishers()), LabelChoice::Automatic);
    assert!(harness.has("Label for foreign keys: name, Automatic"));
    assert!(!harness.has("Use Automatic"));
}

/// The view's key opens the chooser and closes nothing else; Esc and a
/// change of view close it.
#[test]
fn the_structure_views_key_opens_the_chooser() {
    let (mut harness, tab, id) = publishers_structure_in(Look::macos(), false);
    harness.press(Key::L, Modifiers::COMMAND);
    assert!(tab_chooser(&harness, tab, id).is_some());
    harness.press(Key::Escape, Modifiers::NONE);
    assert!(tab_chooser(&harness, tab, id).is_none());
    assert!(saved(&harness).is_empty());
    harness.press(Key::L, Modifiers::COMMAND);
    assert!(tab_chooser(&harness, tab, id).is_some());
    let (view, object_tab) = (ObjectView::Data, id);
    harness.app.apply(Action::SetView {
        tab,
        object_tab,
        view,
    });
    assert!(tab_chooser(&harness, tab, id).is_none());
}

/// The view's button names the chooser's key, so the key is the view's
/// wherever the arrows go: a table opened with a click leaves them on
/// the tree. The keys that need the view's cursor stay the tree's there.
#[test]
fn the_choosers_key_is_the_structure_views_with_the_arrows_on_the_tree() {
    let (mut harness, tab, id) = publishers_structure_in(Look::macos(), false);
    harness.app.workspace_mut(tab).unwrap().pane = Pane::Tree;
    harness.settle();
    harness.press(Key::L, Modifiers::COMMAND);
    assert!(tab_chooser(&harness, tab, id).is_some());
    // The chooser's own keys are read while it is open.
    harness.press(Key::Escape, Modifiers::NONE);
    assert!(tab_chooser(&harness, tab, id).is_none());
    // No cursor shows, and the key that uses the column under it keeps
    // nothing.
    harness.press(Key::L, Modifiers::COMMAND | Modifiers::ALT);
    assert!(saved(&harness).is_empty());
    assert_eq!(cursor(&harness, tab, id), 0);
    harness.press(Key::ArrowDown, Modifiers::NONE);
    assert_eq!(cursor(&harness, tab, id), 0);
}

/// The view has a cursor on the lines of its columns: the arrows move it
/// in every look and `j` and `k` in the Omarchy look, it stops at the
/// first line and the last, and a click puts it on a line.
#[test]
fn the_structure_view_has_a_cursor_on_its_columns() {
    let (mut harness, tab, id) = editions_structure_in(Look::omarchy());
    assert_eq!(cursor(&harness, tab, id), 0);
    type_key(&mut harness, Key::J, "j");
    type_key(&mut harness, Key::J, "j");
    assert_eq!(cursor(&harness, tab, id), 2);
    type_key(&mut harness, Key::K, "k");
    assert_eq!(cursor(&harness, tab, id), 1);
    harness.press(Key::ArrowDown, Modifiers::NONE);
    assert_eq!(cursor(&harness, tab, id), 2);
    for _ in 0..5 {
        harness.press(Key::ArrowDown, Modifiers::NONE);
    }
    assert_eq!(cursor(&harness, tab, id), 3);
    for _ in 0..5 {
        harness.press(Key::ArrowUp, Modifiers::NONE);
    }
    assert_eq!(cursor(&harness, tab, id), 0);
    // The rows' selection did not move under it.
    let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
    assert_eq!(object.selection, None);
    // A click on a line, with the keys on the tree: the line has the
    // cursor and the view the keys.
    let (mut harness, tab, id) = editions_structure_in(Look::macos());
    harness.app.workspace_mut(tab).unwrap().pane = Pane::Tree;
    click_text(&mut harness, "title", egui::PointerButton::Primary);
    assert_eq!(cursor(&harness, tab, id), 2);
    assert_eq!(harness.app.workspace(tab).unwrap().pane, Pane::Grid);
    harness.press(Key::ArrowUp, Modifiers::NONE);
    assert_eq!(cursor(&harness, tab, id), 1);
}

/// Test 6: the key and a column of bytes cannot be chosen, in the
/// chooser, from a column's menu, with the view's key or by a line of
/// the prompt, and each says why.
#[test]
fn the_key_and_bytes_are_refused_with_their_reason() {
    // The chooser lists them, and they answer no click.
    let (mut harness, tab, id) = editions_structure_in(Look::macos());
    harness.click("Change label column");
    let tree = harness.settle();
    for name in [
        "id, INTEGER, the key, unavailable",
        "cover, BLOB, binary, unavailable",
    ] {
        assert!(node(&tree, name, Role::MenuItemRadio).is_some(), "{name}");
        harness.click(name);
        assert!(tab_chooser(&harness, tab, id).is_some(), "{name}");
    }
    harness.press(Key::Escape, Modifiers::NONE);
    // A column's menu offers it, and says why not on those two.
    let why = |harness: &mut Harness| described(harness, "Use as label");
    click_text(&mut harness, "cover", egui::PointerButton::Secondary);
    assert_eq!(disabled(&mut harness, "Use as label"), Some(true));
    assert_eq!(why(&mut harness).as_deref(), Some("binary"));
    harness.press(Key::Escape, Modifiers::NONE);
    click_text(&mut harness, "id", egui::PointerButton::Secondary);
    assert_eq!(why(&mut harness).as_deref(), Some("the key"));
    harness.press(Key::Escape, Modifiers::NONE);
    click_text(&mut harness, "title", egui::PointerButton::Secondary);
    assert_eq!(disabled(&mut harness, "Use as label"), Some(false));
    harness.press(Key::Escape, Modifiers::NONE);
    // The view's key on the key's line: refused, and the view says why
    // until the cursor moves.
    assert_eq!(cursor(&harness, tab, id), 2);
    harness.press(Key::ArrowUp, Modifiers::NONE);
    harness.press(Key::ArrowUp, Modifiers::NONE);
    harness.press(Key::L, Modifiers::COMMAND | Modifiers::ALT);
    let key = Refused::Excluded(Excluded::Key);
    assert_eq!(refused(&harness, tab), Some(("id".to_owned(), key)));
    assert!(harness.has("id cannot be the label (the key)"));
    harness.press(Key::ArrowDown, Modifiers::NONE);
    assert_eq!(refused(&harness, tab), None);
    assert!(!harness.has("id cannot be the label (the key)"));
    assert_eq!(kept(&harness, tab, &editions()), LabelChoice::Automatic);
    assert!(saved(&harness).is_empty());
    // A line of the prompt that names one, and one that names none.
    let (mut harness, tab, _) = editions_structure_in(Look::omarchy());
    for (line, why) in [
        ("label cover", Refused::Excluded(Excluded::Binary)),
        ("label id", Refused::Excluded(Excluded::Key)),
        ("label nope", Refused::NoColumn),
    ] {
        run(&mut harness, tab, line);
        let name = line.trim_start_matches("label ").to_owned();
        assert_eq!(refused(&harness, tab), Some((name, why)), "{line}");
        assert_eq!(kept(&harness, tab, &editions()), LabelChoice::Automatic);
    }
    // Until the next key.
    harness.press(Key::ArrowDown, Modifiers::NONE);
    assert_eq!(refused(&harness, tab), None);
    assert!(saved(&harness).is_empty());
}

/// Test 12: the Omarchy look's `gl` and the prompt's three lines keep
/// what the other looks' controls keep.
#[test]
fn the_terminal_looks_keys_keep_what_the_buttons_keep() {
    let (mut omarchy, tab, id) = editions_structure_in(Look::omarchy());
    let (mut macos, mac_tab, mac_id) = editions_structure_in(Look::macos());
    let same = |omarchy: &Harness, macos: &Harness, choice: LabelChoice| {
        assert_eq!(kept(omarchy, tab, &editions()), choice, "omarchy");
        assert_eq!(kept(macos, mac_tab, &editions()), choice, "macos");
    };
    // The column under the cursor: `gl`, and a column's menu.
    type_key(&mut omarchy, Key::J, "j");
    type_key(&mut omarchy, Key::J, "j");
    type_key(&mut omarchy, Key::G, "g");
    type_key(&mut omarchy, Key::L, "l");
    click_text(&mut macos, "title", egui::PointerButton::Secondary);
    macos.click("Use as label");
    same(&omarchy, &macos, column("title"));
    assert!(omarchy.has("title, label column"));
    assert!(macos.has("title, label column"));
    // Back to Automatic: `:label auto`, and the button.
    run(&mut omarchy, tab, "label auto");
    macos.click("Use Automatic");
    same(&omarchy, &macos, LabelChoice::Automatic);
    // Keys only: `:label none`, and the chooser's line.
    run(&mut omarchy, tab, "label none");
    macos.click("Change label column");
    macos.click("No label, keys only");
    same(&omarchy, &macos, LabelChoice::NoLabel);
    // A column by its name: `:label isbn`, and the chooser's line. `isbn`
    // is a foreign key here and no key of this table's.
    run(&mut omarchy, tab, "label isbn");
    macos.click("Change label column");
    macos.click("isbn, TEXT");
    same(&omarchy, &macos, column("isbn"));
    // `:label` alone opens the chooser, whose keys are `j`, `k` and Enter.
    run(&mut omarchy, tab, "label");
    assert!(tab_chooser(&omarchy, tab, id).is_some());
    assert!(tab_chooser(&macos, mac_tab, mac_id).is_none());
    type_key(&mut omarchy, Key::K, "k");
    type_key(&mut omarchy, Key::K, "k");
    omarchy.press(Key::Enter, Modifiers::NONE);
    assert!(tab_chooser(&omarchy, tab, id).is_none());
    assert_eq!(kept(&omarchy, tab, &editions()), LabelChoice::Automatic);
}

/// The Omarchy look's view says the label in its own words, and names
/// its keys where the other looks have buttons.
#[test]
fn the_terminal_looks_structure_view_says_the_label_in_its_words() {
    let (mut harness, tab, _) = publishers_structure_in(Look::omarchy(), true);
    assert!(harness.has("Label for foreign keys: name, automatic"));
    let help = "fields that point at publishers show and search the label column · \
                on this computer only, nothing is written to the database";
    assert!(harness.has(help));
    assert!(harness.painted_rect("fk label").is_some());
    let keys = ":label choose · gl use the column under the cursor";
    assert!(
        harness.painted_rect(keys).is_some(),
        "{:?}",
        harness.painted
    );
    assert!(!harness.has("Change label column"));
    // The column that labels has the look's mark at its name: an arrow,
    // in whichever form the font draws.
    let marks = ["◂ label", "< label"];
    let marked = marks
        .iter()
        .any(|mark| harness.painted_rect(mark).is_some());
    assert!(marked, "{:?}", harness.painted);
    // Once the user chose, the line names the way back in the keys' place.
    run(&mut harness, tab, "label name");
    assert!(harness.has("Label for foreign keys: name, set by you"));
    let back = ":label auto back to automatic";
    assert!(
        harness.painted_rect(back).is_some(),
        "{:?}",
        harness.painted
    );
    assert!(harness.painted_rect(keys).is_none());
}

/// A table of its key alone: the line says so, the button is disabled,
/// and nothing opens.
#[test]
fn the_structure_view_of_a_table_of_its_key_alone_says_so() {
    let mut harness = Harness::new();
    harness.set_look(Look::macos());
    let tab = harness.connect_fake_as(false);
    let isbns = ObjectRef::new("main", "isbns");
    harness.app.apply(Action::OpenObject {
        tab,
        object: isbns.clone(),
        kind: tabletist_db::ObjectKind::Table,
        pin: true,
    });
    harness.answer_structure(crate::testing::isbns_structure());
    let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
    to_structure(&mut harness, tab, id);
    assert!(harness.has("Label for foreign keys: isbns has only its key column"));
    assert_eq!(disabled(&mut harness, "Change label column"), Some(true));
    harness.press(Key::L, Modifiers::COMMAND);
    assert!(tab_chooser(&harness, tab, id).is_none());
    // Its one column is the key: the view's key says so.
    harness.press(Key::L, Modifiers::COMMAND | Modifiers::ALT);
    let key = Refused::Excluded(Excluded::Key);
    assert_eq!(refused(&harness, tab), Some(("isbn".to_owned(), key)));
    assert_eq!(kept(&harness, tab, &isbns), LabelChoice::Automatic);
}

/// A table with no primary key is pointed at through a column that is
/// unique alone. Its Structure view takes that column for the key, as a
/// picker of such a foreign key does: the app's pick is the next text
/// column, and the unique column is not taken as the label.
#[test]
fn the_structure_view_of_a_table_with_no_primary_key_keeps_out_its_unique_column() {
    use tabletist_db::{ColumnInfo, IndexInfo};
    let mut harness = Harness::new();
    harness.set_look(Look::macos());
    let tab = harness.connect_fake_as(false);
    let isbns = ObjectRef::new("main", "isbns");
    harness.app.apply(Action::OpenObject {
        tab,
        object: isbns.clone(),
        kind: tabletist_db::ObjectKind::Table,
        pin: true,
    });
    // The isbns, with a note beside the isbn and no primary key: the isbn
    // is unique, as a foreign key to it needs.
    let mut structure = crate::testing::isbns_structure();
    structure.primary_key.clear();
    structure.columns.push(ColumnInfo {
        name: "note".into(),
        type_name: "TEXT".into(),
        ..ColumnInfo::default()
    });
    structure.indexes.push(IndexInfo {
        name: "isbns_isbn".into(),
        columns: vec!["isbn".into()],
        key_columns: Some(vec!["isbn".into()]),
        unique: true,
        ..IndexInfo::default()
    });
    harness.answer_structure(structure);
    let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
    to_structure(&mut harness, tab, id);
    assert!(harness.has("Label for foreign keys: note, Automatic"));
    assert!(harness.has("note, label column"));
    // The cursor is on the isbn's line: the view's key does not take it.
    harness.press(Key::L, Modifiers::COMMAND | Modifiers::ALT);
    let key = Refused::Excluded(Excluded::Key);
    assert_eq!(refused(&harness, tab), Some(("isbn".to_owned(), key)));
    assert_eq!(kept(&harness, tab, &isbns), LabelChoice::Automatic);
    // The note can be: it is no key.
    harness
        .app
        .apply(Action::MoveStructureCursor { tab, id, by: 1 });
    harness.press(Key::L, Modifiers::COMMAND | Modifiers::ALT);
    assert_eq!(
        kept(&harness, tab, &isbns),
        LabelChoice::Column("note".into())
    );
}

/// A view labels nothing: no foreign key points at one. Its Structure
/// view has no label line, and the prompt's lines do nothing there.
#[test]
fn a_views_structure_has_no_label() {
    let mut harness = Harness::new();
    harness.set_look(Look::omarchy());
    let tab = harness.connect_fake_as(false);
    let view = ObjectRef::new("main", "active_users");
    harness.app.apply(Action::OpenObject {
        tab,
        object: view.clone(),
        kind: tabletist_db::ObjectKind::View,
        pin: true,
    });
    harness.answer_structure(publishers_structure());
    let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
    to_structure(&mut harness, tab, id);
    assert!(!harness.has("Label for foreign keys: name, automatic"));
    run(&mut harness, tab, "label name");
    run(&mut harness, tab, "label");
    assert!(tab_chooser(&harness, tab, id).is_none());
    assert_eq!(kept(&harness, tab, &view), LabelChoice::Automatic);
    assert!(saved(&harness).is_empty());
}

/// The lines of the prompt are the Structure view's: over the rows
/// `:label` is no command.
#[test]
fn the_label_lines_are_no_commands_over_the_rows() {
    let mut harness = Harness::new();
    harness.set_look(Look::omarchy());
    let (tab, id) = harness.editions();
    for line in ["label", "label auto", "label title"] {
        run(&mut harness, tab, line);
        let workspace = harness.app.workspace(tab).unwrap();
        assert_eq!(workspace.command_error.as_deref(), Some(line));
        assert!(tab_chooser(&harness, tab, id).is_none());
    }
    assert!(saved(&harness).is_empty());
}

/// A choice made in the Structure view labels the pickers of every key
/// that points at the table: the next one opened searches by it.
#[test]
fn a_choice_made_in_the_structure_view_labels_the_pickers() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    // `publishers` in a tab of its own, beside the covers'.
    harness.app.apply(Action::OpenObject {
        tab,
        object: publishers(),
        kind: tabletist_db::ObjectKind::Table,
        pin: true,
    });
    harness.answer_structure(publishers_structure());
    let other = harness.app.workspace(tab).unwrap().active_tab.unwrap();
    to_structure(&mut harness, tab, other);
    harness.click("Change label column");
    harness.click("No label, keys only");
    assert_eq!(kept(&harness, tab, &publishers()), LabelChoice::NoLabel);
    // Back on the covers: the picker lists keys alone.
    harness.app.apply(Action::ActivateTab { tab, id });
    harness.settle();
    open(&mut harness, tab, id, 1);
    let by_user = labelled(None, Source::User, Some("name"));
    assert!(picker(&harness, tab, id).label == Columns::Known(by_user));
    assert!(!searched(&harness).is_empty());
    assert!(searched(&harness).iter().all(Option::is_none));
}

/// The view is a pane for the keyboard, as the rows are whose place it
/// stands in: the Omarchy look's key for the pane to the right brings
/// the keys to it from the tree, and its cursor's keys then answer.
#[test]
fn the_pane_keys_give_the_structure_view_the_keys() {
    let (mut harness, tab, id) = editions_structure_in(Look::omarchy());
    harness.app.workspace_mut(tab).unwrap().pane = Pane::Tree;
    harness.settle();
    // On the tree `j` is the tree's.
    type_key(&mut harness, Key::J, "j");
    assert_eq!(cursor(&harness, tab, id), 0);
    harness.press(Key::L, ctrl());
    assert_eq!(harness.app.workspace(tab).unwrap().pane, Pane::Grid);
    // With the keyboard on the view's one stop its chords answer too: the
    // stop is a pane, and no control that would take the arrows itself.
    harness.press(Key::ArrowDown, Modifiers::NONE);
    assert_eq!(cursor(&harness, tab, id), 1);
    type_key(&mut harness, Key::J, "j");
    assert_eq!(cursor(&harness, tab, id), 2);
    type_key(&mut harness, Key::G, "g");
    type_key(&mut harness, Key::L, "l");
    assert_eq!(kept(&harness, tab, &editions()), column("title"));
}

/// What was said of a column that was not taken is said of its table,
/// in its tab: another table's Structure view does not say it.
#[test]
fn a_refusal_is_said_in_its_own_tab_alone() {
    let (mut harness, tab, _) = editions_structure_in(Look::macos());
    harness.press(Key::L, Modifiers::COMMAND | Modifiers::ALT);
    let said = "id cannot be the label (the key)";
    assert!(harness.has(said));
    harness.app.apply(Action::OpenObject {
        tab,
        object: publishers(),
        kind: tabletist_db::ObjectKind::Table,
        pin: true,
    });
    harness.answer_structure(publishers_structure());
    let other = harness.app.workspace(tab).unwrap().active_tab.unwrap();
    to_structure(&mut harness, tab, other);
    assert!(harness.has("Label for foreign keys: name, Automatic"));
    assert!(!harness.has(said));
}

/// A press beside the view's chooser closes it, as it closes a menu.
#[test]
fn a_press_beside_the_structure_views_chooser_closes_it() {
    let (mut harness, tab, id) = publishers_structure_in(Look::macos(), false);
    harness.click("Change label column");
    assert!(tab_chooser(&harness, tab, id).is_some());
    // Far from the chooser, which hangs under its button at the card's
    // end: the middle of the window, near its foot.
    let at = egui::pos2(harness.size.x / 2.0, harness.size.y - 120.0);
    click_at(&mut harness, at);
    assert!(tab_chooser(&harness, tab, id).is_none());
    assert!(saved(&harness).is_empty());
}

/// The cursor stays on a line there is when the table's structure comes
/// again with fewer columns.
#[test]
fn the_structure_views_cursor_stays_inside_the_columns() {
    let (mut harness, tab, id) = editions_structure_in(Look::macos());
    let line = 3;
    harness
        .app
        .apply(Action::SetStructureCursor { tab, id, line });
    assert_eq!(cursor(&harness, tab, id), 3);
    let object_tab = id;
    harness
        .app
        .apply(Action::RetryStructure { tab, object_tab });
    harness.answer_structure(publishers_structure());
    assert_eq!(cursor(&harness, tab, id), 1);
}

/// The editions with eighty columns after their own, in their Structure
/// view: more lines than the window shows.
fn wide_structure() -> (Harness, ConnTabId, TabId) {
    let mut harness = Harness::new();
    harness.set_look(Look::macos());
    let tab = harness.connect_fake_as(false);
    harness.app.apply(Action::OpenObject {
        tab,
        object: editions(),
        kind: tabletist_db::ObjectKind::Table,
        pin: true,
    });
    let mut wide = crate::testing::editions_structure();
    for number in 0..80 {
        wide.columns.push(tabletist_db::ColumnInfo {
            name: format!("extra_{number:02}"),
            type_name: "TEXT".into(),
            ..tabletist_db::ColumnInfo::default()
        });
    }
    harness.answer_structure(wide);
    let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
    to_structure(&mut harness, tab, id);
    (harness, tab, id)
}

/// Whether the words `name` are painted inside the window.
fn in_sight(harness: &Harness, name: &str) -> bool {
    let window = egui::Rect::from_min_size(egui::Pos2::ZERO, harness.size);
    let painted = harness.painted_rect(name);
    painted.is_some_and(|painted| window.contains_rect(painted))
}

/// Moves the Structure view's cursor and waits for the view: the jump is
/// asked for where the line is drawn, and the view has it two frames
/// after.
fn move_cursor(harness: &mut Harness, tab: ConnTabId, id: TabId, by: isize) {
    harness
        .app
        .apply(Action::MoveStructureCursor { tab, id, by });
    harness.settle();
    harness.settle();
}

/// A table with more columns than the window shows: the cursor's line
/// is brought into view as the cursor moves, so the view's keys never
/// act on a line that is out of sight.
#[test]
fn the_structure_views_cursor_is_scrolled_into_view() {
    let (mut harness, tab, id) = wide_structure();
    assert!(in_sight(&harness, "title"));
    assert!(!in_sight(&harness, "extra_70"));
    move_cursor(&mut harness, tab, id, 74);
    assert_eq!(cursor(&harness, tab, id), 74);
    assert!(
        in_sight(&harness, "extra_70"),
        "the cursor's line is out of sight"
    );
    // And back: the first line shows again.
    move_cursor(&mut harness, tab, id, -74);
    assert!(in_sight(&harness, "title"));
    assert!(!in_sight(&harness, "extra_70"));
}

/// The view may be scrolled while the tree has the keys. When the keys
/// come back to it, the cursor's line is brought into view again.
#[test]
fn the_structure_views_cursor_is_in_view_when_the_keys_come_back() {
    let (mut harness, tab, id) = wide_structure();
    move_cursor(&mut harness, tab, id, 74);
    assert!(in_sight(&harness, "extra_70"));
    harness.app.workspace_mut(tab).unwrap().pane = Pane::Tree;
    // The wheel, over the view, back to its top.
    let over = egui::pos2(harness.size.x / 2.0, harness.size.y / 2.0);
    harness.frame(vec![egui::Event::PointerMoved(over)]);
    for _ in 0..10 {
        harness.frame(vec![egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Line,
            delta: egui::vec2(0.0, 40.0),
            phase: egui::TouchPhase::Move,
            modifiers: Modifiers::NONE,
        }]);
    }
    harness.finish_animations();
    assert!(!in_sight(&harness, "extra_70"));
    harness.app.apply(Action::GridKeys(tab));
    harness.settle();
    harness.settle();
    assert!(
        in_sight(&harness, "extra_70"),
        "the cursor's line is out of sight"
    );
}

/// Each table's Structure view keeps its own place: after another
/// table's view was looked at, the cursor's line is where it was left.
#[test]
fn each_structure_view_keeps_its_own_place() {
    let (mut harness, tab, id) = wide_structure();
    move_cursor(&mut harness, tab, id, 74);
    assert!(in_sight(&harness, "extra_70"));
    // Another table, whose few columns need no scrolling.
    harness.app.apply(Action::OpenObject {
        tab,
        object: publishers(),
        kind: tabletist_db::ObjectKind::Table,
        pin: true,
    });
    harness.answer_structure(publishers_structure());
    let other = harness.app.workspace(tab).unwrap().active_tab.unwrap();
    assert_ne!(other, id);
    to_structure(&mut harness, tab, other);
    harness.settle();
    assert!(!in_sight(&harness, "extra_70"));
    harness.app.apply(Action::ActivateTab { tab, id });
    harness.settle();
    harness.settle();
    assert!(
        in_sight(&harness, "extra_70"),
        "the cursor's line is out of sight"
    );
}

/// What the keys do in a Structure view that has them: Reload reloads
/// the table, where with the keys on the tree it reloads the tree.
#[test]
fn reload_in_the_structure_view_reloads_the_table() {
    let (mut harness, tab, _) = publishers_structure_in(Look::macos(), false);
    let described = |harness: &Harness| {
        let sent = harness.app.backend.sent.iter();
        sent.filter(|command| matches!(command, Command::Describe { .. }))
            .count()
    };
    let before = described(&harness);
    harness.press(Key::R, Modifiers::COMMAND);
    assert_eq!(described(&harness), before + 1);
    // The tree's own reload asks for no table's structure.
    harness.app.workspace_mut(tab).unwrap().pane = Pane::Tree;
    harness.press(Key::R, Modifiers::COMMAND);
    assert_eq!(described(&harness), before + 1);
}

// The Structure door in the Omarchy look's status line.

/// With the Structure view of a table in front the status line names
/// that view's keys in the rows' place, as the board has them. What saves
/// the table's rows stays.
#[test]
fn the_terminal_looks_status_line_names_the_structure_views_keys() {
    let (mut harness, tab, _) = publishers_structure_in(Look::omarchy(), false);
    let painted = |harness: &Harness, hint: &str| harness.painted_rect(hint).is_some();
    let hints = [
        "j/k column",
        "gl use as label",
        ":label choose",
        ":label auto",
        ":w write",
    ];
    for hint in hints {
        assert!(painted(&harness, hint), "{hint}: {:?}", harness.painted);
    }
    // No key of the rows', which the view has none of.
    assert!(!painted(&harness, "j/k row"));
    // Over the rows the line is the rows' again.
    let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
    let (view, object_tab) = (ObjectView::Data, id);
    harness.app.apply(Action::SetView {
        tab,
        object_tab,
        view,
    });
    harness.settle();
    assert!(painted(&harness, "j/k row"));
    assert!(!painted(&harness, "gl use as label"));
    // A table of its key alone has a cursor on its one column and no
    // label to choose: the line names the first, and not the keys of
    // the second.
    let mut harness = Harness::new();
    harness.set_look(Look::omarchy());
    let tab = harness.connect_fake_as(false);
    harness.app.apply(Action::OpenObject {
        tab,
        object: ObjectRef::new("main", "isbns"),
        kind: tabletist_db::ObjectKind::Table,
        pin: true,
    });
    harness.answer_structure(crate::testing::isbns_structure());
    let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
    to_structure(&mut harness, tab, id);
    assert!(painted(&harness, "j/k column"), "{:?}", harness.painted);
    assert!(!painted(&harness, "j/k row"));
    assert!(!painted(&harness, "gl use as label"));
    assert!(!painted(&harness, ":label choose"));
}

/// Test 6, in the Omarchy look: the status line says why a column that
/// was named is not taken, until the next key.
#[test]
fn the_terminal_look_says_why_a_named_column_is_refused() {
    let (mut harness, tab, _) = editions_structure_in(Look::omarchy());
    for (line, said) in [
        ("label cover", "cover cannot be the label (binary)"),
        ("label id", "id cannot be the label (the key)"),
        ("label nope", "no column nope in editions"),
    ] {
        run(&mut harness, tab, line);
        assert!(harness.has(said), "{line}");
    }
    // `gl` on the key's line says the same.
    harness.press(Key::ArrowUp, Modifiers::NONE);
    assert!(!harness.has("no column nope in editions"));
    type_key(&mut harness, Key::G, "g");
    type_key(&mut harness, Key::L, "l");
    assert!(harness.has("id cannot be the label (the key)"));
    harness.press(Key::ArrowDown, Modifiers::NONE);
    assert!(!harness.has("id cannot be the label (the key)"));
}
