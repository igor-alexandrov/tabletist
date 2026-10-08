//! Row inserting, checked against its spec
//! (`docs/superpowers/specs/2026-10-07-inserting-rows-design.md`) for the
//! audit of 2026-10-08. Each test names the audit's finding it shows
//! (INS-...). One that passes holds what the app does today. One that is
//! ignored holds what the spec asks and the app does not do yet: it fails
//! until its finding is fixed, and its `ignore` says which that is.
//!
//! Only behaviour, names and words are checked here, on the Bookshop's
//! `book_covers`. No colour, size or picture of the design is.

use egui::accesskit::Role;
use egui::{Key, Modifiers};

use crate::backend::Command;
use crate::edit::{Edits, Place, new_row};
use crate::model::{Action, Advance, CellPos, ConnTabId, EditStart, ObjectView, Pane, TabId};
use crate::review::Line;
use crate::testing::Harness;
use crate::theme::Look;

/// No publisher has this id.
const NO_PUBLISHER: &str = "9100000000000000099";
/// Harbor Press, as the design's picker finds it.
const HARBOR_PRESS: &str = "9100000000000000004";

/// The Bookshop's covers, writable, the grid holding the keyboard.
fn covers_in(look: Look) -> (Harness, ConnTabId, TabId) {
    let mut harness = Harness::new();
    harness.set_look(look);
    let (tab, id) = harness.book_covers();
    harness.app.workspace_mut(tab).unwrap().pane = Pane::Grid;
    harness.settle();
    (harness, tab, id)
}

/// The Bookshop's covers as an object of `kind`, on a connection that
/// opens read-only or not, the grid holding the keyboard.
fn covers_as(
    look: Look,
    kind: tabletist_db::ObjectKind,
    read_only: bool,
) -> (Harness, ConnTabId, TabId) {
    let mut harness = Harness::new();
    harness.set_look(look);
    let tab = harness.connect_fake_as(read_only);
    harness.app.apply(Action::OpenObject {
        tab,
        object: tabletist_db::ObjectRef::new("main", "book_covers"),
        kind,
        pin: true,
    });
    harness.answer_structure(crate::testing::book_covers_structure());
    harness.answer_rows(crate::testing::book_covers_page(3));
    let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
    harness.app.workspace_mut(tab).unwrap().pane = Pane::Grid;
    harness.settle();
    (harness, tab, id)
}

fn edits(harness: &Harness, tab: ConnTabId, id: TabId) -> &Edits {
    let workspace = harness.app.workspace(tab).unwrap();
    &workspace.object_tab(id).unwrap().edits
}

fn selected(harness: &Harness, tab: ConnTabId, id: TabId) -> Option<(usize, usize)> {
    let workspace = harness.app.workspace(tab).unwrap();
    let cell = workspace.object_tab(id).unwrap().selection?;
    Some((cell.row, cell.col))
}

/// Whether the last frame painted `text` as one piece.
fn painted(harness: &Harness, text: &str) -> bool {
    harness.painted.iter().any(|(piece, _)| piece == text)
}

/// Selects the cell at `at` (row, column), as a click on it does.
fn select(harness: &mut Harness, tab: ConnTabId, id: TabId, at: (usize, usize)) {
    let cell = CellPos {
        row: at.0,
        col: at.1,
    };
    harness.app.apply(Action::SelectCell { tab, id, cell });
    harness.settle();
}

/// A key the terminal look reads as the character it types.
fn type_key(harness: &mut Harness, key: Key, text: &str) {
    harness.settle();
    harness.frame(vec![
        crate::testing::key(key, Modifiers::NONE),
        egui::Event::Text(text.into()),
    ]);
    harness.frame(vec![crate::testing::release(key, Modifiers::NONE)]);
    harness.settle();
}

/// Types `text` over the cell at `at` and commits it: pending where the
/// column takes it.
fn set(harness: &mut Harness, tab: ConnTabId, id: TabId, at: (usize, usize), text: &str) {
    let cell = CellPos {
        row: at.0,
        col: at.1,
    };
    let start = EditStart::Replace(text.into());
    harness.app.apply(Action::EditCell {
        tab,
        id,
        cell,
        start,
    });
    let then = Advance::Stay;
    harness.app.apply(Action::CommitEdit { tab, id, then });
}

/// Types `text` over the cell at `at` and leaves the cell: pending, or to
/// fix where its column does not take the text.
fn leave(harness: &mut Harness, tab: ConnTabId, id: TabId, at: (usize, usize), text: &str) {
    let cell = CellPos {
        row: at.0,
        col: at.1,
    };
    let start = EditStart::Replace(text.into());
    harness.app.apply(Action::EditCell {
        tab,
        id,
        cell,
        start,
    });
    let left = crate::testing::leave_edit(&harness.app, tab, id);
    harness.app.apply(left);
}

/// A new row under the header, its editor closed. Says its id.
fn add(harness: &mut Harness, tab: ConnTabId, id: TabId) -> usize {
    let place = Place::Top;
    harness.app.apply(Action::AddRow { tab, id, place });
    harness.app.apply(Action::CancelEdit { tab, id });
    harness.settle();
    edits(harness, tab, id).added.last().unwrap().id
}

/// How many saves were sent.
fn writes(harness: &Harness) -> usize {
    let sent = harness.app.backend.sent.iter();
    sent.filter(|command| matches!(command, Command::Write { .. }))
        .count()
}

/// The save's failure on its first new row, as PostgreSQL words it.
fn fails(harness: &mut Harness, code: &str, message: &str, detail: &str) {
    harness.answer_written(Ok(tabletist_db::WriteOutcome::FailedInsert {
        insert: 0,
        error: tabletist_db::Error::Query {
            code: Some(code.into()),
            message: message.into(),
            detail: Some(detail.into()),
            hint: None,
        },
    }));
    harness.settle();
}

// A. Entry points.

/// INS-01d. Spec 1: "Grow: Down on the last row" adds an empty row on
/// macOS.
#[test]
#[ignore = "INS-01d: Down on the last row adds no row yet"]
fn down_on_the_last_row_adds_a_row() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    select(&mut harness, tab, id, (2, 1));
    harness.press(Key::ArrowDown, Modifiers::NONE);
    assert_eq!(edits(&harness, tab, id).added.len(), 1);
}

/// Spec 1: in the terminal look `j` on the last row adds none.
#[test]
fn j_on_the_last_row_adds_no_row() {
    let (mut harness, tab, id) = covers_in(Look::omarchy());
    select(&mut harness, tab, id, (2, 1));
    type_key(&mut harness, Key::J, "j");
    assert!(edits(&harness, tab, id).added.is_empty());
    assert_eq!(selected(&harness, tab, id), Some((2, 1)));
}

/// INS-02a. Spec 4: Mod+D makes a copy of the selected row.
#[test]
#[ignore = "INS-02a: Mod+D does not duplicate a row yet"]
fn mod_d_duplicates_the_selected_row() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    select(&mut harness, tab, id, (1, 2));
    harness.press(Key::D, Modifiers::COMMAND);
    assert_eq!(edits(&harness, tab, id).added.len(), 1);
}

/// INS-02b. Spec 4: `yy` then `p` puts a copy of the row under the cursor.
#[test]
#[ignore = "INS-02b: yy p does not duplicate a row yet"]
fn yy_p_duplicates_the_row_under_the_cursor() {
    let (mut harness, tab, id) = covers_in(Look::omarchy());
    select(&mut harness, tab, id, (1, 2));
    type_key(&mut harness, Key::Y, "y");
    type_key(&mut harness, Key::Y, "y");
    type_key(&mut harness, Key::P, "p");
    assert_eq!(edits(&harness, tab, id).added.len(), 1);
}

/// Two covers as a spreadsheet copies them, under a header row.
const PASTED: &str = "publisher_id\tkind\n9100000000000000004\tebook\n9100000000000000007\tprint";

/// INS-03a. Spec 5: pasting rows opens a preview that says how many rows
/// it would add, and to which table.
#[test]
#[ignore = "INS-03a: pasted rows open no preview yet"]
fn pasted_rows_open_a_preview() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    select(&mut harness, tab, id, (0, 1));
    let chord = Modifiers::COMMAND | Modifiers::SHIFT;
    harness.frame(vec![
        crate::testing::key(Key::V, chord),
        egui::Event::Paste(PASTED.into()),
    ]);
    harness.frame(vec![crate::testing::release(Key::V, chord)]);
    harness.settle();
    assert!(harness.has("Add 2 pasted rows to book_covers?"));
}

/// Spec 1: a paste inside an open cell editor is text in that cell, as
/// it was before rows could be pasted.
#[test]
fn a_paste_in_an_open_editor_is_its_text() {
    for look in [Look::macos(), Look::omarchy()] {
        let (mut harness, tab, id) = covers_in(look);
        select(&mut harness, tab, id, (0, 2));
        // The cell's editor, with `print` in it.
        harness.press(Key::Enter, Modifiers::NONE);
        assert!(harness.ctx.text_edit_focused(), "{}", look.name);
        harness.frame(vec![egui::Event::Paste("ed".into())]);
        harness.settle();
        let editor = edits(&harness, tab, id).editor.as_ref().expect("an editor");
        assert_eq!(editor.text, "printed", "{}", look.name);
        // The paste was the editor's: no row came of it, and no dialog.
        assert!(edits(&harness, tab, id).added.is_empty(), "{}", look.name);
        assert!(harness.app.dialog.is_none(), "{}", look.name);
    }
}

/// INS-03b. The audit's item 3, and the editing design's "Paste a TSV
/// block across cells": a plain paste on the grid goes over the cells from
/// the selected one, and adds no row.
#[test]
#[ignore = "INS-03b: a paste on the grid does not go over its cells yet"]
fn a_paste_on_the_grid_goes_over_its_cells() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    // `kind` of the first two covers: `print` and `ebook`.
    select(&mut harness, tab, id, (0, 2));
    harness.frame(vec![egui::Event::Paste("audio\naudio".into())]);
    harness.settle();
    let pending: Vec<(usize, usize)> = edits(&harness, tab, id).cells.keys().copied().collect();
    assert_eq!(pending, [(0, 2), (1, 2)]);
    assert!(edits(&harness, tab, id).added.is_empty());
}

/// Spec 1: where no row can be added the button cannot be pressed, and
/// says why: on a connection that opens read-only, and on a view.
#[test]
fn add_row_cannot_be_pressed_where_no_row_is_taken_and_says_why() {
    use tabletist_db::ObjectKind;
    for (kind, read_only, why) in [
        (ObjectKind::Table, true, "This connection opens read-only"),
        (ObjectKind::View, false, "Views cannot be edited"),
    ] {
        let (mut harness, tab, id) = covers_as(Look::macos(), kind, read_only);
        let tree = harness.settle();
        let button = crate::testing::node(&tree, "Add row", Role::Button).expect("Add row");
        let (_, node) = tree.nodes.iter().find(|(node, _)| *node == button).unwrap();
        assert!(node.is_disabled(), "{why}");
        assert_eq!(node.description(), Some(why));
        harness.click("Add row");
        assert!(edits(&harness, tab, id).added.is_empty(), "{why}");
    }
}

/// Spec 1: "The tooltip or status line says why." The terminal look has
/// no button to say it. Its line says why a view's cells cannot be edited,
/// which is why `o` adds no row there.
#[test]
fn the_terminals_line_says_why_a_view_takes_no_row() {
    let kind = tabletist_db::ObjectKind::View;
    let (mut harness, tab, id) = covers_as(Look::omarchy(), kind, false);
    select(&mut harness, tab, id, (0, 1));
    type_key(&mut harness, Key::O, "o");
    assert!(edits(&harness, tab, id).added.is_empty());
    let why = Look::omarchy().label("Views cannot be edited");
    assert!(painted(&harness, &why), "{:?}", harness.painted);
}

// B. Where a new row stands.

/// A page long enough to scroll: the covers' columns, 200 rows.
fn long_covers(look: Look) -> (Harness, ConnTabId, TabId) {
    let mut harness = Harness::new();
    harness.set_look(look);
    let tab = harness.connect_fake_as(false);
    harness.app.apply(Action::OpenObject {
        tab,
        object: tabletist_db::ObjectRef::new("main", "book_covers"),
        kind: tabletist_db::ObjectKind::Table,
        pin: true,
    });
    harness.answer_structure(crate::testing::book_covers_structure());
    harness.answer_rows(crate::testing::book_covers_page(200));
    let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
    harness.app.workspace_mut(tab).unwrap().pane = Pane::Grid;
    harness.settle();
    (harness, tab, id)
}

/// Whether a screen reader finds the new row: it is drawn. Asked once the
/// grid has scrolled to where it was sent.
fn new_row_drawn(harness: &mut Harness) -> bool {
    let tree = harness.finish_animations();
    crate::testing::node(&tree, "New row, not saved", Role::Button).is_some()
}

/// Spec 2: "The grid scrolls to the top if needed."
#[test]
fn adding_a_row_scrolls_the_grid_to_it() {
    let (mut harness, tab, id) = long_covers(Look::macos());
    select(&mut harness, tab, id, (180, 1));
    let tree = harness.finish_animations();
    assert!(crate::testing::node(&tree, "Row 1", Role::Button).is_none());
    harness.press(Key::N, Modifiers::COMMAND);
    harness.settle();
    assert_eq!(edits(&harness, tab, id).added.len(), 1);
    assert!(new_row_drawn(&mut harness));
}

/// INS-05a. Spec 2: on macOS a new row is "pinned directly under the
/// header ... regardless of sort, filter or scroll position".
#[test]
#[ignore = "INS-05a: a new row scrolls away with the page's rows"]
fn a_new_row_stays_under_the_header_when_the_grid_scrolls() {
    let (mut harness, tab, id) = long_covers(Look::macos());
    add(&mut harness, tab, id);
    assert!(new_row_drawn(&mut harness));
    select(&mut harness, tab, id, (180, 1));
    assert!(new_row_drawn(&mut harness));
}

/// INS-07b. The Omarchy board's status line says `publisher_id required`
/// in insert mode, while the row's first value is being typed.
#[test]
#[ignore = "INS-07b: in insert mode the terminal's line does not say what the row needs"]
fn the_terminals_line_says_what_the_row_needs_while_it_is_typed() {
    let (mut harness, tab, id) = covers_in(Look::omarchy());
    harness.app.workspace_mut(tab).unwrap().row_panel = false;
    select(&mut harness, tab, id, (0, 1));
    type_key(&mut harness, Key::O, "o");
    assert!(edits(&harness, tab, id).editor.is_some(), "insert mode");
    assert!(
        painted(&harness, "publisher_id required"),
        "{:?}",
        harness.painted
    );
}

/// INS-09b. Spec 3: "A foreign key column opens the search picker on the
/// first keystroke."
#[test]
#[ignore = "INS-09b: a foreign key column opens the text editor, not a picker"]
fn typing_into_a_foreign_key_opens_its_picker() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    harness.press(Key::N, Modifiers::COMMAND);
    harness.settle();
    assert_eq!(selected(&harness, tab, id), Some((new_row(0), 1)));
    harness.frame(vec![egui::Event::Text("harb".into())]);
    harness.settle();
    assert!(harness.has("publishers matching \"harb\" · ↑↓ choose · ↩ set"));
}

// C. The inspector.

/// INS-11. Spec 7: the inspector of a new row is its form.
#[test]
#[ignore = "INS-11: the inspector has no form for a new row yet"]
fn the_inspector_is_the_new_rows_form() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    add(&mut harness, tab, id);
    assert!(harness.has("book_covers · not saved"));
    assert!(harness.has("Discard new row"));
    assert!(harness.has("Add another"));
}

/// INS-14. Spec 7: what the inspector says of a table without a key.
#[test]
#[ignore = "INS-14: the inspector does not warn of a table without a primary key"]
fn the_inspector_warns_of_a_table_without_a_key() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    {
        let workspace = harness.app.workspace_mut(tab).unwrap();
        let object = workspace.object_tab_mut(id).unwrap();
        let structure = object.structure.value.as_mut().unwrap();
        structure.primary_key.clear();
        structure.indexes.clear();
    }
    add(&mut harness, tab, id);
    assert_eq!(edits(&harness, tab, id).added.len(), 1);
    assert!(harness.has(
        "This table has no primary key. The row can be inserted, but not edited or deleted \
         afterwards."
    ));
}

// D. The pending model.

/// INS-16c. Spec 6: "Undo/redo covers creating, editing and dropping new
/// rows."
#[test]
#[ignore = "INS-16c: undo does not bring a dropped new row back"]
fn undo_brings_a_dropped_row_back() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    let new = add(&mut harness, tab, id);
    set(&mut harness, tab, id, (new_row(new), 1), HARBOR_PRESS);
    select(&mut harness, tab, id, (new_row(new), 1));
    harness.press(Key::Delete, Modifiers::NONE);
    assert!(edits(&harness, tab, id).added.is_empty());
    harness.press(Key::Z, Modifiers::COMMAND);
    assert_eq!(edits(&harness, tab, id).added.len(), 1);
    assert_eq!(edits(&harness, tab, id).cells.len(), 1);
}

/// Spec 6: `:e!` drops new rows along with the other pending changes.
#[test]
fn e_bang_drops_new_rows_with_everything_else() {
    let (mut harness, tab, id) = covers_in(Look::omarchy());
    let new = add(&mut harness, tab, id);
    set(&mut harness, tab, id, (new_row(new), 1), HARBOR_PRESS);
    set(&mut harness, tab, id, (1, 2), "audio");
    assert_eq!(edits(&harness, tab, id).counts().added, 1);
    harness.app.apply(Action::OpenCommand(tab));
    harness.app.workspace_mut(tab).unwrap().command = Some("e!".into());
    harness.app.apply(Action::RunCommand(tab));
    harness.settle();
    assert!(!edits(&harness, tab, id).pending());
    assert_eq!(writes(&harness), 0);
}

// G. Review SQL and the save.

/// The statements of the tab's review, each as one line of SQL, and how
/// many of its comments are a new row's.
fn reviewed(harness: &Harness, tab: ConnTabId, id: TabId) -> (Vec<String>, usize) {
    let review = edits(harness, tab, id).review.as_ref().expect("a review");
    let mut statements: Vec<String> = Vec::new();
    let mut new = 0;
    let mut open = false;
    for line in &review.lines {
        match line.sql() {
            Some(sql) if open => {
                let last = statements.last_mut().unwrap();
                last.push(' ');
                last.push_str(sql.trim());
            }
            Some(sql) => {
                statements.push(sql.trim().to_owned());
                open = true;
            }
            None => {
                new += usize::from(matches!(line, Line::New));
                open = false;
            }
        }
    }
    (statements, new)
}

/// Spec 8, with INS-28a: Review SQL shows the new row's `INSERT` before
/// the changed row's `UPDATE`, naming only the column that was set, and
/// the save sends that statement: built from the set it sends, it reads as
/// the review does.
#[test]
fn review_shows_the_insert_first_and_it_is_what_the_save_sends() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    let new = add(&mut harness, tab, id);
    set(&mut harness, tab, id, (new_row(new), 1), HARBOR_PRESS);
    set(&mut harness, tab, id, (1, 2), "audio");
    harness.click("Review SQL");
    harness.settle();
    let (statements, comments) = reviewed(&harness, tab, id);
    assert_eq!(comments, 1);
    assert_eq!(statements.len(), 2, "{statements:?}");
    assert_eq!(
        statements[0],
        r#"INSERT INTO "main"."book_covers" ("publisher_id") VALUES (9100000000000000004) RETURNING *;"#
    );
    assert!(statements[1].starts_with("UPDATE "), "{statements:?}");
    harness.click("Save");
    let Command::Write { changes, .. } = crate::testing::last_sent(&harness.app) else {
        panic!("a save was sent");
    };
    assert_eq!((changes.inserts.len(), changes.rows.len()), (1, 1));
    let insert = tabletist_db::Dialect::Sqlite
        .insert_row(&changes.object, &changes.inserts[0])
        .unwrap();
    assert_eq!(format!("{};", insert.shown), statements[0]);
    // What goes to SQLite is that statement with its value bound.
    assert_eq!(
        insert.sql.text,
        r#"INSERT INTO "main"."book_covers" ("publisher_id") VALUES (?) RETURNING *"#
    );
    assert_eq!(
        insert.sql.params,
        [tabletist_db::Value::Int(9_100_000_000_000_000_004)]
    );
}

/// INS-28b. Spec 8: "Batch up to 100 rows with the same column set into
/// one multi-row INSERT."
#[test]
#[ignore = "INS-28b: each new row is a statement of its own"]
fn new_rows_with_the_same_columns_are_one_statement() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    for publisher in [HARBOR_PRESS, "9100000000000000007"] {
        let new = add(&mut harness, tab, id);
        set(&mut harness, tab, id, (new_row(new), 1), publisher);
    }
    harness.click("Review SQL");
    harness.settle();
    let (statements, _) = reviewed(&harness, tab, id);
    assert_eq!(statements.len(), 1, "{statements:?}");
}

/// INS-29b. The audit's item 29: the terminal look confirms a save to
/// production by the database's name, and "anything else is refused".
/// The app asks for the word `write`.
#[test]
#[ignore = "INS-29b: the terminal's production prompt asks for the word write, not the database"]
fn the_terminal_refuses_a_production_save_until_the_database_is_named() {
    let (mut harness, tab, id) = covers_in(Look::omarchy());
    harness.app.workspace_mut(tab).unwrap().environment = crate::env::Environment::Production;
    let new = add(&mut harness, tab, id);
    set(&mut harness, tab, id, (new_row(new), 1), HARBOR_PRESS);
    harness.app.apply(Action::WriteEdits { tab, id });
    harness.settle();
    assert!(harness.app.dialog.is_some());
    harness.frame(vec![egui::Event::Text("write".into())]);
    harness.settle();
    harness.press(Key::Enter, Modifiers::NONE);
    assert_eq!(writes(&harness), 0);
}

// H. What a save says back.

/// A new row with a publisher that is not there, its save sent.
fn saving_a_cover_without_its_publisher(look: Look) -> (Harness, ConnTabId, TabId) {
    let (mut harness, tab, id) = covers_in(look);
    harness.app.workspace_mut(tab).unwrap().row_panel = false;
    let new = add(&mut harness, tab, id);
    set(&mut harness, tab, id, (new_row(new), 1), NO_PUBLISHER);
    harness.app.apply(Action::WriteEdits { tab, id });
    (harness, tab, id)
}

const NO_PUBLISHER_MESSAGE: &str = "insert or update on table \"book_covers\" violates foreign key constraint \
     \"book_covers_publisher_id_fkey\"";
const NO_PUBLISHER_DETAIL: &str =
    "Key (publisher_id)=(9100000000000000099) is not present in table \"publishers\".";

/// Spec 9: after a failed save the new row stays pending, its set cell is
/// the one that failed, and the row says the database's code and message.
/// (INS-31c: PostgreSQL names the value only in the error's detail, which
/// is shown nowhere yet. The test under this one asks for it.)
#[test]
fn a_failed_row_stays_pending_and_says_the_databases_code_and_message() {
    let (mut harness, tab, id) = saving_a_cover_without_its_publisher(Look::omarchy());
    fails(
        &mut harness,
        "23503",
        NO_PUBLISHER_MESSAGE,
        NO_PUBLISHER_DETAIL,
    );
    assert_eq!(edits(&harness, tab, id).counts().added, 1);
    assert_eq!(edits(&harness, tab, id).counts().failed, 1);
    // The terminal's error line names the row and the column, then says
    // the code and the message, cut where the line ends.
    let lines = harness.painted.iter().map(|(piece, _)| piece.as_str());
    let lines: Vec<&str> = lines
        .filter(|piece| piece.starts_with("! new:publisher_id"))
        .collect();
    assert_eq!(lines.len(), 1, "{:?}", harness.painted);
    let says = "! new:publisher_id  23503 insert or update on table \"book_covers\" violates";
    assert!(lines[0].starts_with(says), "{lines:?}");
}

/// INS-31b, INS-31c. Spec 9: a foreign key that fails says "No publisher
/// with id ...". The id is in the error's detail.
#[test]
#[ignore = "INS-31b, INS-31c: a failed foreign key says the server's message, without the value"]
fn a_publisher_that_is_not_there_is_said_in_the_specs_words() {
    let (mut harness, _, _) = saving_a_cover_without_its_publisher(Look::macos());
    fails(
        &mut harness,
        "23503",
        NO_PUBLISHER_MESSAGE,
        NO_PUBLISHER_DETAIL,
    );
    assert!(harness.has("No publisher with id 9100000000000000099"));
}

/// INS-31a. Spec 9: a value that is taken says by which row, found with
/// one `SELECT pk ... WHERE col = $1 LIMIT 1` after the rollback, and opens
/// it. Here a publisher has one cover: `publisher_id` is unique, and the
/// new row is given the publisher of the cover with id 1.
#[test]
#[ignore = "INS-31a: a unique violation does not look up or offer the row that has the value"]
fn a_taken_value_offers_the_row_that_has_it() {
    use tabletist_db::{Filter, FilterOp, IndexInfo};
    const TAKEN: &str = "9100000000000000001";
    let (mut harness, tab, id) = covers_in(Look::macos());
    harness.app.workspace_mut(tab).unwrap().row_panel = false;
    {
        let workspace = harness.app.workspace_mut(tab).unwrap();
        let object = workspace.object_tab_mut(id).unwrap();
        let structure = object.structure.value.as_mut().unwrap();
        structure.indexes.push(IndexInfo {
            name: "book_covers_publisher_id_key".into(),
            columns: vec!["publisher_id".into()],
            key_columns: Some(vec!["publisher_id".into()]),
            unique: true,
            ..IndexInfo::default()
        });
    }
    let new = add(&mut harness, tab, id);
    set(&mut harness, tab, id, (new_row(new), 1), TAKEN);
    harness.app.apply(Action::WriteEdits { tab, id });
    let sent = harness.app.backend.sent.len();
    fails(
        &mut harness,
        "23505",
        "duplicate key value violates unique constraint \"book_covers_publisher_id_key\"",
        "Key (publisher_id)=(9100000000000000001) already exists.",
    );
    // One read after the rollback: the row of this table that holds the
    // value in the column that failed.
    let after = harness.app.backend.sent[sent..].iter();
    let lookups: Vec<&tabletist_db::RowQuery> = after
        .filter_map(|command| match command {
            Command::FetchRows { query, .. } => Some(query),
            _ => None,
        })
        .collect();
    assert_eq!(
        lookups.len(),
        1,
        "one lookup for the row that has the value"
    );
    assert_eq!(lookups[0].object.name, "book_covers");
    assert_eq!(lookups[0].limit, 1);
    let by_the_value = Filter {
        column: "publisher_id".into(),
        op: FilterOp::Eq,
        value: TAKEN.into(),
    };
    assert_eq!(lookups[0].filters, [by_the_value]);
    // The cover that has the publisher is the page's first, with id 1.
    harness.answer_rows(crate::testing::book_covers_page(1));
    harness.settle();
    assert!(harness.has("Already used by row id 1 · Open row"));
}

/// INS-30c. Spec 9: after a save the row stays where it was, and "Show in
/// sorted position" puts it where the sort has it.
#[test]
#[ignore = "INS-30c: a saved row has no Show in sorted position"]
fn a_saved_row_can_be_shown_where_the_sort_puts_it() {
    use tabletist_db::Value;
    let (mut harness, tab, id) = covers_in(Look::macos());
    let new = add(&mut harness, tab, id);
    set(&mut harness, tab, id, (new_row(new), 1), HARBOR_PRESS);
    harness.app.apply(Action::WriteEdits { tab, id });
    harness.answer_written(Ok(tabletist_db::WriteOutcome::Written {
        inserted: vec![Some(vec![
            Value::Int(15),
            Value::Int(9_100_000_000_000_000_004),
            Value::Text("print".into()),
            Value::Null,
            Value::Text("2026-10-07 10:42:09".into()),
        ])],
        rows: Vec::new(),
        elapsed: std::time::Duration::from_millis(5),
    }));
    harness.settle();
    assert!(harness.has("Show in sorted position"));
}

/// Two new rows, each with a publisher that is no number: two cells to
/// fix, with the selection on the page's first row.
fn two_rows_to_fix(look: Look) -> (Harness, ConnTabId, TabId) {
    let (mut harness, tab, id) = covers_in(look);
    for _ in 0..2 {
        let new = add(&mut harness, tab, id);
        leave(&mut harness, tab, id, (new_row(new), 1), "harbor");
    }
    assert_eq!(edits(&harness, tab, id).counts().to_fix, 2);
    select(&mut harness, tab, id, (0, 0));
    (harness, tab, id)
}

/// INS-33a. The audit's item 33: Alt+Mod+Down and Alt+Mod+Up move between
/// errors on macOS.
#[test]
#[ignore = "INS-33a: no key moves between errors on macOS"]
fn alt_mod_down_moves_to_the_next_error() {
    let (mut harness, tab, id) = two_rows_to_fix(Look::macos());
    harness.press(Key::ArrowDown, Modifiers::ALT | Modifiers::COMMAND);
    let at = selected(&harness, tab, id).expect("a selection");
    assert!(crate::edit::new_id(at.0).is_some(), "{at:?}");
    assert_eq!(at.1, 1);
}

/// INS-33b. Spec 9: `]e` and `[e` move between errors in the terminal
/// look.
#[test]
#[ignore = "INS-33b: ]e and [e do not move between errors"]
fn bracket_e_moves_to_the_next_error() {
    let (mut harness, tab, id) = two_rows_to_fix(Look::omarchy());
    type_key(&mut harness, Key::CloseBracket, "]");
    type_key(&mut harness, Key::E, "e");
    let at = selected(&harness, tab, id).expect("a selection");
    assert!(crate::edit::new_id(at.0).is_some(), "{at:?}");
    assert_eq!(at.1, 1);
}

// I. Accessibility.

/// INS-35b. Spec 10: a new row is labeled "New row, not saved" (it is),
/// and a cell adds ", required", ", default <value>" or ", assigned by the
/// database". The grid's cells are painted and have no names of their own.
#[test]
#[ignore = "INS-35b: a new row's cells are not named to a screen reader"]
fn a_new_rows_cells_say_what_is_asked_of_them() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    add(&mut harness, tab, id);
    harness.app.apply(Action::SetView {
        tab,
        object_tab: id,
        view: ObjectView::Data,
    });
    let tree = harness.settle();
    let labels = crate::testing::labels(&tree);
    assert!(labels.iter().any(|label| label == "New row, not saved"));
    for ending in [
        ", required",
        ", default print",
        ", assigned by the database",
    ] {
        assert!(
            labels.iter().any(|label| label.ends_with(ending)),
            "{ending}"
        );
    }
}
