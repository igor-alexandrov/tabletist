//! Duplicating a row, checked against its spec
//! (`docs/superpowers/specs/2026-10-10-row-duplication-design.md`: "spec
//! N" below is a section of its Behavior, "test N" an entry of its Tests).
//! Only behaviour, names and words, on the Bookshop's `book_covers`. What
//! a copy holds cell by cell is tested where it is decided, in `edit.rs`.

use egui::{Key, Modifiers};
use tabletist_db::{IndexInfo, NewValue, ObjectKind, ObjectRef, RowPage, Structure, Value};

use super::insert_audit_tests::{add, covers_as, covers_in, edits, select, selected, set};
use crate::edit::{Cleared, Order, State, new_row};
use crate::model::{
    Action, CellPos, ConnTabId, Dialog, EditStart, ObjectView, Pane, SaveBlock, TabId,
};
use crate::testing::Harness;
use crate::theme::Look;

/// The Bookshop's covers, writable, the grid holding the keyboard, their
/// structure and their page changed by `vary` before they load.
fn covers(
    look: Look,
    vary: impl FnOnce(&mut Structure, &mut RowPage),
) -> (Harness, ConnTabId, TabId) {
    let mut harness = Harness::new();
    harness.set_look(look);
    let tab = harness.connect_fake_as(false);
    harness.app.apply(Action::OpenObject {
        tab,
        object: ObjectRef::new("main", "book_covers"),
        kind: ObjectKind::Table,
        pin: true,
    });
    let mut structure = crate::testing::book_covers_structure();
    let mut page = crate::testing::book_covers_page(3);
    vary(&mut structure, &mut page);
    harness.answer_structure(structure);
    harness.answer_rows(page);
    let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
    harness.app.workspace_mut(tab).unwrap().pane = Pane::Grid;
    harness.settle();
    (harness, tab, id)
}

/// A unique index on `publisher_id` alone, as the Bookshop has it.
fn unique_publisher(structure: &mut Structure, _: &mut RowPage) {
    structure.indexes.push(IndexInfo {
        name: "book_covers_publisher_id_key".into(),
        columns: vec!["publisher_id".into()],
        key_columns: Some(vec!["publisher_id".into()]),
        unique: true,
        ..IndexInfo::default()
    });
}

/// What is set in the cell at `at` (row, column), pending.
fn value(harness: &Harness, tab: ConnTabId, id: TabId, at: (usize, usize)) -> Option<NewValue> {
    let cells = &edits(harness, tab, id).cells;
    cells.get(&at).map(|cell| cell.new.clone())
}

fn text(value: &str) -> Option<NewValue> {
    Some(NewValue::Text(value.into()))
}

/// The rows the grid shows, in its order: the page's three and the new
/// ones among them.
fn shown(harness: &Harness, tab: ConnTabId, id: TabId) -> Vec<usize> {
    let order = Order::of(&edits(harness, tab, id).added, 3);
    (0..order.len())
        .filter_map(|place| order.row(place))
        .collect()
}

/// The row of the newest new row.
fn newest(harness: &Harness, tab: ConnTabId, id: TabId) -> usize {
    let added = edits(harness, tab, id).added.iter();
    new_row(added.map(|new| new.id).max().expect("a new row"))
}

fn duplicate(harness: &mut Harness, tab: ConnTabId, id: TabId) {
    harness.app.apply(Action::DuplicateRow { tab, id });
}

fn put(harness: &mut Harness, tab: ConnTabId, id: TabId, above: bool) {
    harness.app.apply(Action::PutRow { tab, id, above });
}

/// Whether the tab holds a yanked row.
fn yanked(harness: &Harness, tab: ConnTabId, id: TabId) -> bool {
    let workspace = harness.app.workspace(tab).unwrap();
    workspace.object_tab(id).unwrap().yanked.is_some()
}

// The reducer: what a copy is, and where it goes.

/// Test 3, the insert spec's test 5: a copy leaves what the database
/// numbers and what no other row may hold, keeps the rest, and opens the
/// editor on the first cell a save needs.
#[test]
fn a_copy_clears_what_it_must_and_opens_the_first_cell_it_needs() {
    let (mut harness, tab, id) = covers(Look::macos(), unique_publisher);
    select(&mut harness, tab, id, (1, 2));
    duplicate(&mut harness, tab, id);
    assert_eq!(edits(&harness, tab, id).added.len(), 1);
    let row = newest(&harness, tab, id);
    assert_eq!(value(&harness, tab, id, (row, 0)), None);
    assert_eq!(value(&harness, tab, id, (row, 1)), None);
    assert_eq!(value(&harness, tab, id, (row, 2)), text("ebook"));
    assert_eq!(value(&harness, tab, id, (row, 3)), None);
    let created = text("2026-01-12 09:14:03");
    assert_eq!(value(&harness, tab, id, (row, 4)), created);
    let now = edits(&harness, tab, id);
    let made = now.added[0].made.as_ref().expect("made as a copy");
    assert_eq!(
        made.cleared,
        [
            ("id".to_owned(), Cleared::Identity),
            ("publisher_id".to_owned(), Cleared::Unique)
        ]
    );
    // `publisher_id`: the one cell a save needs a value in.
    let editor = now.editor.as_ref().expect("an editor");
    assert_eq!(editor.cell, CellPos { row, col: 1 });
    assert_eq!(selected(&harness, tab, id), Some((row, 1)));
    // At the top, as an added row is.
    assert_eq!(shown(&harness, tab, id), [row, 0, 1, 2]);
    // Nothing of the source changed.
    assert_eq!(now.cells.keys().filter(|(of, _)| *of == 1).count(), 0);
}

/// Test 10: where nothing is asked of the copy the selection goes to it,
/// in the column the cursor was in, and no editor opens.
#[test]
fn a_copy_nothing_is_asked_of_takes_the_selection_in_the_cursors_column() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    select(&mut harness, tab, id, (1, 2));
    duplicate(&mut harness, tab, id);
    let row = newest(&harness, tab, id);
    // No index holds `publisher_id` unique here: it is copied.
    let publisher = text("9100000000000000002");
    assert_eq!(value(&harness, tab, id, (row, 1)), publisher);
    assert!(edits(&harness, tab, id).editor.is_none());
    assert_eq!(selected(&harness, tab, id), Some((row, 2)));
}

/// Test 4: the copy holds what is pending in the source, the text of an
/// editor open on it too, and a pending DEFAULT leaves its cell unset.
#[test]
fn a_copy_takes_what_is_pending_and_what_is_being_typed() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    set(&mut harness, tab, id, (1, 2), "audio");
    // An editor left open on the source, with a publisher typed into it.
    let other = "9100000000000000007";
    harness.app.apply(Action::EditCell {
        tab,
        id,
        cell: CellPos { row: 1, col: 1 },
        start: EditStart::Replace(other.into()),
    });
    duplicate(&mut harness, tab, id);
    let row = newest(&harness, tab, id);
    assert_eq!(value(&harness, tab, id, (row, 1)), text(other));
    assert_eq!(value(&harness, tab, id, (row, 2)), text("audio"));
    // The source keeps both, pending.
    assert_eq!(value(&harness, tab, id, (1, 1)), text(other));
    assert_eq!(value(&harness, tab, id, (1, 2)), text("audio"));
    // A pending DEFAULT: the copy's unset cell is its default already.
    let (mut harness, tab, id) = covers_in(Look::macos());
    select(&mut harness, tab, id, (1, 2));
    harness.app.apply(Action::SetDefault { tab, id });
    let pending = value(&harness, tab, id, (1, 2));
    assert!(matches!(pending, Some(NewValue::Default { .. })));
    duplicate(&mut harness, tab, id);
    let row = newest(&harness, tab, id);
    assert_eq!(value(&harness, tab, id, (row, 2)), None);
}

/// Test 6: a new row is copied with what is set in it, and its unset cells
/// stay unset.
#[test]
fn a_new_row_is_copied_with_what_is_set_in_it() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    let first = new_row(add(&mut harness, tab, id));
    set(&mut harness, tab, id, (first, 2), "audio");
    select(&mut harness, tab, id, (first, 2));
    duplicate(&mut harness, tab, id);
    assert_eq!(edits(&harness, tab, id).added.len(), 2);
    let copy = newest(&harness, tab, id);
    assert_ne!(copy, first);
    assert_eq!(value(&harness, tab, id, (copy, 2)), text("audio"));
    assert_eq!(value(&harness, tab, id, (copy, 1)), None);
    // Under the new rows already at the top.
    assert_eq!(shown(&harness, tab, id), [first, copy, 0, 1, 2]);
    // The source set no publisher, so the copy needs one.
    let editor = edits(&harness, tab, id).editor.as_ref().expect("an editor");
    assert_eq!(editor.cell, CellPos { row: copy, col: 1 });
}

/// Test 2 and spec 2: a yanked row is put below the cursor's row, or
/// above it, wherever the cursor has gone since.
#[test]
fn a_yanked_row_is_put_below_the_cursors_row_or_above_it() {
    let (mut harness, tab, id) = covers_in(Look::omarchy());
    select(&mut harness, tab, id, (1, 2));
    harness.app.apply(Action::YankRow { tab, id });
    // The yank adds nothing.
    assert!(edits(&harness, tab, id).added.is_empty());
    select(&mut harness, tab, id, (0, 2));
    put(&mut harness, tab, id, false);
    let below = newest(&harness, tab, id);
    assert_eq!(shown(&harness, tab, id), [0, below, 1, 2]);
    select(&mut harness, tab, id, (2, 2));
    put(&mut harness, tab, id, true);
    let above = newest(&harness, tab, id);
    assert_eq!(shown(&harness, tab, id), [0, below, 1, above, 2]);
    // Each is the yanked row, an `ebook`, not the `print` the cursor was on.
    for row in [below, above] {
        assert_eq!(value(&harness, tab, id, (row, 2)), text("ebook"));
    }
    // And the cursor is on the copy, in the column it was in.
    assert_eq!(selected(&harness, tab, id), Some((above, 2)));
}

/// Spec 2: with no cell selected a yanked row is put at the top.
#[test]
fn a_yanked_row_is_put_at_the_top_with_no_cell_selected() {
    let (mut harness, tab, id) = covers_in(Look::omarchy());
    select(&mut harness, tab, id, (1, 2));
    harness.app.apply(Action::YankRow { tab, id });
    // No action takes the selection off a page that has rows.
    let workspace = harness.app.workspace_mut(tab).unwrap();
    workspace.object_tab_mut(id).unwrap().selection = None;
    put(&mut harness, tab, id, false);
    let copy = newest(&harness, tab, id);
    assert_eq!(shown(&harness, tab, id), [copy, 0, 1, 2]);
    assert_eq!(value(&harness, tab, id, (copy, 2)), text("ebook"));
}

/// Test 11: a yank outlives a reload of the page and a discard, and is put
/// as often as it is asked for. Nothing is put before one, and nothing
/// while the page is fetched.
#[test]
fn a_yank_outlives_a_reload_and_a_discard_and_is_put_again() {
    let (mut harness, tab, id) = covers_in(Look::omarchy());
    // With nothing yanked a put does nothing: an editor stays open too.
    harness.app.apply(Action::EditCell {
        tab,
        id,
        cell: CellPos { row: 0, col: 2 },
        start: EditStart::Replace("audio".into()),
    });
    put(&mut harness, tab, id, false);
    assert!(edits(&harness, tab, id).added.is_empty(), "nothing yanked");
    assert!(edits(&harness, tab, id).editor.is_some(), "nothing yanked");
    harness.app.apply(Action::CancelEdit { tab, id });
    assert!(!edits(&harness, tab, id).holds());
    select(&mut harness, tab, id, (1, 2));
    harness.app.apply(Action::YankRow { tab, id });
    // The page comes back with every cover an `audio`.
    let mut other = crate::testing::book_covers_page(3);
    for row in &mut other.rows {
        row[2] = Value::Text("audio".into());
    }
    harness.app.apply(Action::Refresh(tab));
    // A fetch is running: the table takes no row until it answers.
    put(&mut harness, tab, id, false);
    assert!(edits(&harness, tab, id).added.is_empty(), "refreshing");
    harness.answer_rows(other);
    harness.answer_structure(crate::testing::book_covers_structure());
    // A copy to discard.
    select(&mut harness, tab, id, (0, 2));
    put(&mut harness, tab, id, false);
    assert_eq!(edits(&harness, tab, id).added.len(), 1);
    harness.app.apply(Action::DiscardEdits { tab, id });
    assert!(edits(&harness, tab, id).added.is_empty(), "discarded");
    select(&mut harness, tab, id, (0, 2));
    put(&mut harness, tab, id, false);
    let first = newest(&harness, tab, id);
    put(&mut harness, tab, id, false);
    let second = newest(&harness, tab, id);
    assert_eq!(edits(&harness, tab, id).added.len(), 2);
    for row in [first, second] {
        assert_eq!(value(&harness, tab, id, (row, 2)), text("ebook"));
    }
}

/// A prompt about the pending changes stands over what it asks about: no
/// copy is made under it, and the editor it found open keeps its text.
#[test]
fn no_copy_is_made_under_a_prompt_about_the_changes() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    set(&mut harness, tab, id, (0, 2), "audio");
    select(&mut harness, tab, id, (1, 2));
    harness.app.apply(Action::YankRow { tab, id });
    assert!(yanked(&harness, tab, id));
    // An editor left open, with a publisher typed into it.
    let other = "9100000000000000007";
    let typed = CellPos { row: 1, col: 1 };
    harness.app.apply(Action::EditCell {
        tab,
        id,
        cell: typed,
        start: EditStart::Replace(other.into()),
    });
    harness.app.apply(Action::CloseTab { tab, id });
    let stands = |harness: &Harness, what: &str| {
        assert!(
            matches!(harness.app.dialog, Some(Dialog::Leave(_))),
            "{what}"
        );
        let now = edits(harness, tab, id);
        // Closed, its text would join the set the prompt asks about.
        let editor = now.editor.as_ref().expect(what);
        assert_eq!(
            (editor.cell, editor.text.as_str()),
            (typed, other),
            "{what}"
        );
        assert!(now.added.is_empty(), "{what}");
        assert_eq!(now.cells.len(), 1, "{what}");
    };
    stands(&harness, "the prompt");
    duplicate(&mut harness, tab, id);
    stands(&harness, "a copy");
    put(&mut harness, tab, id, false);
    stands(&harness, "a put below");
    put(&mut harness, tab, id, true);
    stands(&harness, "a put above");
}

/// Test 13: no copy is made where no row is added: a connection that
/// opens read-only, a view, a save in flight, and the Structure view.
#[test]
fn no_copy_is_made_where_no_row_is_taken() {
    let all = |harness: &mut Harness, tab: ConnTabId, id: TabId| {
        harness.app.apply(Action::YankRow { tab, id });
        duplicate(harness, tab, id);
        put(harness, tab, id, false);
        put(harness, tab, id, true);
    };
    for (kind, read_only) in [(ObjectKind::Table, true), (ObjectKind::View, false)] {
        let (mut harness, tab, id) = covers_as(Look::macos(), kind, read_only);
        select(&mut harness, tab, id, (1, 2));
        all(&mut harness, tab, id);
        assert!(edits(&harness, tab, id).added.is_empty(), "{kind:?}");
    }
    // A save in flight: its answer names the new rows it sent.
    let (mut harness, tab, id) = covers_in(Look::macos());
    set(&mut harness, tab, id, (0, 2), "audio");
    select(&mut harness, tab, id, (1, 2));
    harness.app.apply(Action::WriteEdits { tab, id });
    assert!(edits(&harness, tab, id).saving.is_some());
    all(&mut harness, tab, id);
    assert!(edits(&harness, tab, id).added.is_empty(), "saving");
    // The Structure view shows no row to copy.
    let (mut harness, tab, id) = covers_in(Look::macos());
    select(&mut harness, tab, id, (1, 2));
    harness.app.apply(Action::SetView {
        tab,
        object_tab: id,
        view: ObjectView::Structure,
    });
    all(&mut harness, tab, id);
    assert!(edits(&harness, tab, id).added.is_empty(), "structure");
    // Nor a row to yank: the put is not only refused, it has nothing.
    assert!(!yanked(&harness, tab, id), "structure");
}

/// Test 13: a table without a key takes a copy, as it takes a new row.
#[test]
fn a_table_without_a_key_takes_a_copy() {
    let keyless = |structure: &mut Structure, _: &mut RowPage| structure.primary_key.clear();
    let (mut harness, tab, id) = covers(Look::macos(), keyless);
    select(&mut harness, tab, id, (1, 2));
    duplicate(&mut harness, tab, id);
    let now = edits(&harness, tab, id);
    assert_eq!(now.added.len(), 1);
    // `id` is still the database's to number.
    let made = now.added[0].made.as_ref().unwrap();
    assert_eq!(made.cleared, [("id".to_owned(), Cleared::Identity)]);
}

/// Test 9: a copied value its column does not take is to fix, and a save
/// waits for it.
#[test]
fn a_save_waits_for_a_copied_value_its_column_does_not_take() {
    // Not one of the values `kind` lists.
    let odd = |_: &mut Structure, page: &mut RowPage| {
        page.rows[1][2] = Value::Text("vinyl".into());
    };
    let (mut harness, tab, id) = covers(Look::macos(), odd);
    select(&mut harness, tab, id, (1, 2));
    duplicate(&mut harness, tab, id);
    let row = newest(&harness, tab, id);
    let cell = &edits(&harness, tab, id).cells[&(row, 2)];
    assert!(matches!(cell.state, State::ToFix(_)));
    let blocked = harness.app.save_blocked(tab, id);
    assert!(matches!(blocked, Some(SaveBlock::ToFix)));
}

/// Test 17: Review SQL shows the copy as one `INSERT`, naming the columns
/// it holds and no other.
#[test]
fn review_shows_the_copy_as_one_insert_of_what_it_holds() {
    use super::insert_audit_tests::reviewed;
    let (mut harness, tab, id) = covers_in(Look::macos());
    select(&mut harness, tab, id, (1, 2));
    duplicate(&mut harness, tab, id);
    harness.click("Review SQL");
    harness.settle();
    let (statements, comments) = reviewed(&harness, tab, id);
    assert_eq!((statements.len(), comments), (1, 1), "{statements:?}");
    let named =
        r#"INSERT INTO "main"."book_covers" ("publisher_id", "kind", "created_at") VALUES ("#;
    assert!(statements[0].starts_with(named), "{statements:?}");
}

// The desktop looks' key.

/// Test 1: a held Mod+D makes one copy. Its repeats are no presses.
#[test]
fn mod_d_makes_one_copy_however_long_it_is_held() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    select(&mut harness, tab, id, (1, 2));
    let repeat = egui::Event::Key {
        key: Key::D,
        physical_key: None,
        pressed: true,
        repeat: true,
        modifiers: Modifiers::COMMAND,
    };
    harness.frame(vec![crate::testing::key(Key::D, Modifiers::COMMAND)]);
    for _ in 0..3 {
        harness.frame(vec![repeat.clone()]);
    }
    harness.frame(vec![crate::testing::release(Key::D, Modifiers::COMMAND)]);
    harness.settle();
    assert_eq!(edits(&harness, tab, id).added.len(), 1);
    // A second press is a second copy, of the copy the cursor is on.
    harness.press(Key::D, Modifiers::COMMAND);
    assert_eq!(edits(&harness, tab, id).added.len(), 2);
}

/// Test 19: Mod+Shift+D is Review SQL still, and makes no copy.
#[test]
fn mod_shift_d_is_review_sql_and_makes_no_copy() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    set(&mut harness, tab, id, (1, 2), "audio");
    select(&mut harness, tab, id, (1, 2));
    harness.press(Key::D, Modifiers::COMMAND | Modifiers::SHIFT);
    assert!(edits(&harness, tab, id).reviewing);
    assert!(edits(&harness, tab, id).added.is_empty());
}

/// Spec 1: the key is the desktop looks', and copies the selected row:
/// with none selected it does nothing.
#[test]
fn mod_d_needs_a_row_and_is_not_the_terminals_key() {
    let (mut harness, tab, id) = covers_in(Look::omarchy());
    select(&mut harness, tab, id, (1, 2));
    harness.press(Key::D, Modifiers::COMMAND);
    assert!(edits(&harness, tab, id).added.is_empty(), "the terminal");
    let (mut harness, tab, id) = covers_in(Look::macos());
    assert_eq!(selected(&harness, tab, id), None);
    harness.press(Key::D, Modifiers::COMMAND);
    assert!(edits(&harness, tab, id).added.is_empty(), "no row");
    // Nor where no row is taken: a connection that opens read-only.
    let (mut harness, tab, id) = covers_as(Look::macos(), ObjectKind::Table, true);
    select(&mut harness, tab, id, (1, 2));
    harness.press(Key::D, Modifiers::COMMAND);
    assert!(edits(&harness, tab, id).added.is_empty(), "read-only");
}

/// Test 21: like Mod+N, the key does not ask what has the keyboard.
#[test]
fn mod_d_copies_with_the_keyboard_in_the_filters_field() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    select(&mut harness, tab, id, (1, 2));
    harness.press(Key::F, Modifiers::COMMAND);
    assert!(harness.ctx.text_edit_focused(), "the filter's field");
    harness.press(Key::D, Modifiers::COMMAND);
    assert_eq!(edits(&harness, tab, id).added.len(), 1);
}

/// A click selects its cell when the grid is drawn, after the keys were
/// read: a Mod+D in the click's frame would copy the row the selection
/// leaves. It makes no copy, and the next press copies the clicked row.
#[test]
fn mod_d_in_the_frame_of_a_click_makes_no_copy() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    select(&mut harness, tab, id, (0, 2));
    // The page's one `ebook`, in row 1.
    let at = harness.painted_rect("ebook").expect("its cell").center();
    let button = |pressed| egui::Event::PointerButton {
        pos: at,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    harness.frame(vec![egui::Event::PointerMoved(at)]);
    harness.frame(vec![
        button(true),
        button(false),
        crate::testing::key(Key::D, Modifiers::COMMAND),
    ]);
    harness.frame(vec![crate::testing::release(Key::D, Modifiers::COMMAND)]);
    harness.settle();
    assert!(edits(&harness, tab, id).added.is_empty());
    assert_eq!(selected(&harness, tab, id), Some((1, 2)), "the click");
    harness.press(Key::D, Modifiers::COMMAND);
    assert_eq!(edits(&harness, tab, id).added.len(), 1);
    let row = newest(&harness, tab, id);
    assert_eq!(value(&harness, tab, id, (row, 2)), text("ebook"));
}
