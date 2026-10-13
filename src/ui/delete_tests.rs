//! Deleting rows, checked against its spec
//! (`docs/superpowers/specs/2026-10-12-deleting-rows-design.md`: "spec N"
//! below is a section of its Behavior, "test N" an entry of its Tests).
//!
//! Only behaviour, names and words are checked here, on the Bookshop's
//! `book_covers`. No colour, size or picture of the design is.

use super::insert_audit_tests::{add, covers_as, covers_in, edits, select, selected, set};
use crate::backend::Command;
use crate::model::{Action, ConnTabId, TabId};
use crate::testing::Harness;
use crate::theme::Look;

/// The last save that was sent.
fn sent(harness: &Harness) -> &tabletist_db::ChangeSet {
    let mut sent = harness.app.backend.sent.iter().rev();
    sent.find_map(|command| match command {
        Command::Write { changes, .. } => Some(changes),
        _ => None,
    })
    .expect("a save was sent")
}

/// Marks the row at `row` of the page, as the keys do.
fn mark(harness: &mut Harness, tab: ConnTabId, id: TabId, row: usize) {
    select(harness, tab, id, (row, 1));
    harness.app.apply(Action::DeleteRow { tab, id });
    harness.settle();
}

/// Spec 1 and test 1, by the action the keys and the button push: a mark,
/// and the mark taken back by the same action, by Restore and by the
/// action that puts a cell back. The cursor stays.
#[test]
fn a_row_is_marked_and_restored_three_ways() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    mark(&mut harness, tab, id, 1);
    assert!(edits(&harness, tab, id).deleted.contains_key(&1));
    assert_eq!(selected(&harness, tab, id), Some((1, 1)));
    harness.app.apply(Action::DeleteRow { tab, id });
    assert!(edits(&harness, tab, id).deleted.is_empty());
    mark(&mut harness, tab, id, 1);
    harness.app.apply(Action::RestoreRow { tab, id });
    assert!(edits(&harness, tab, id).deleted.is_empty());
    mark(&mut harness, tab, id, 1);
    let cell = None;
    harness.app.apply(Action::RevertCell { tab, id, cell });
    assert!(edits(&harness, tab, id).deleted.is_empty());
}

/// Spec 1, "New rows", and test 5: a new row is dropped, and nothing of
/// it is sent.
#[test]
fn deleting_a_new_row_drops_it() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    add(&mut harness, tab, id);
    assert_eq!(edits(&harness, tab, id).added.len(), 1);
    harness.app.apply(Action::DeleteRow { tab, id });
    let edits = edits(&harness, tab, id);
    assert!(edits.added.is_empty() && edits.deleted.is_empty());
    assert!(!edits.pending());
}

/// Spec 1, "Existing edits", and test 4: the save of a marked row with
/// edits is its DELETE alone.
#[test]
fn a_marked_row_with_edits_is_saved_as_its_delete_alone() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    set(&mut harness, tab, id, (1, 2), "audio");
    mark(&mut harness, tab, id, 1);
    harness.app.apply(Action::WriteEdits { tab, id });
    let changes = sent(&harness);
    assert!(changes.rows.is_empty() && changes.inserts.is_empty());
    assert_eq!(changes.deletes.len(), 1);
    assert_eq!(
        changes.deletes[0].key,
        [("id".to_owned(), tabletist_db::Value::Int(2))]
    );
}

/// And the edits are back with the row.
#[test]
fn restoring_a_row_brings_its_edits_back() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    set(&mut harness, tab, id, (1, 2), "audio");
    mark(&mut harness, tab, id, 1);
    assert_eq!(edits(&harness, tab, id).counts().changes, 0);
    harness.app.apply(Action::RestoreRow { tab, id });
    assert_eq!(edits(&harness, tab, id).counts().changes, 1);
    harness.app.apply(Action::WriteEdits { tab, id });
    let changes = sent(&harness);
    assert!(changes.deletes.is_empty());
    assert_eq!(changes.rows.len(), 1);
}

/// Spec 3: no editor opens on a marked row, and it says why.
#[test]
fn a_marked_rows_cell_is_not_edited() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    mark(&mut harness, tab, id, 1);
    let cell = crate::model::CellPos { row: 1, col: 2 };
    let start = crate::model::EditStart::Replace("audio".into());
    harness.app.apply(Action::EditCell {
        tab,
        id,
        cell,
        start,
    });
    let edits = edits(&harness, tab, id);
    assert!(edits.editor.is_none());
    assert_eq!(edits.why, Some((cell, crate::edit::Lock::Deleting)));
}

/// Spec 2 and test 6: a row that cannot be deleted is not marked.
#[test]
fn a_row_that_cannot_be_deleted_is_not_marked() {
    for (kind, read_only) in [
        (tabletist_db::ObjectKind::Table, true),
        (tabletist_db::ObjectKind::View, false),
    ] {
        let (mut harness, tab, id) = covers_as(Look::macos(), kind, read_only);
        mark(&mut harness, tab, id, 1);
        assert!(edits(&harness, tab, id).deleted.is_empty());
    }
}

/// Row v: the row panel shows a marked row as it loaded, with nothing of
/// its hidden edits.
#[test]
fn a_marked_rows_panel_shows_none_of_its_edits() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    set(&mut harness, tab, id, (1, 2), "audio");
    harness.settle();
    assert!(harness.painted.iter().any(|(text, _)| text == "audio"));
    mark(&mut harness, tab, id, 1);
    assert!(!harness.painted.iter().any(|(text, _)| text == "audio"));
}

fn written(gone: Vec<usize>) -> Result<tabletist_db::WriteOutcome, tabletist_db::Error> {
    Ok(tabletist_db::WriteOutcome::Written {
        inserted: Vec::new(),
        rows: Vec::new(),
        gone,
        elapsed: std::time::Duration::from_millis(14),
    })
}

fn page_ids(harness: &Harness, tab: ConnTabId, id: TabId) -> Vec<tabletist_db::Value> {
    let workspace = harness.app.workspace(tab).unwrap();
    let page = workspace.object_tab(id).unwrap().page().unwrap();
    page.rows.iter().map(|row| row[0].clone()).collect()
}

/// Spec 5 and test 11: after the save the row is gone from the grid, the
/// count is one less, and the cursor is on the row that stood after it.
#[test]
fn a_saved_delete_takes_the_row_out_and_moves_the_cursor_on() {
    use tabletist_db::Value::Int;
    let (mut harness, tab, id) = covers_in(Look::macos());
    mark(&mut harness, tab, id, 1);
    harness.app.apply(Action::WriteEdits { tab, id });
    harness.answer_written(written(Vec::new()));
    harness.settle();
    assert_eq!(page_ids(&harness, tab, id), [Int(1), Int(3)]);
    // The row with id 3 stands where the deleted one stood.
    assert_eq!(selected(&harness, tab, id), Some((1, 1)));
    let edits = edits(&harness, tab, id);
    assert!(!edits.pending());
    assert_eq!(edits.saved.as_ref().map(|saved| saved.deleted), Some(1));
}

#[test]
fn deleting_the_last_row_leaves_the_cursor_on_the_one_before_it() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    mark(&mut harness, tab, id, 2);
    harness.app.apply(Action::WriteEdits { tab, id });
    harness.answer_written(written(Vec::new()));
    harness.settle();
    assert_eq!(selected(&harness, tab, id), Some((1, 1)));
}

/// Spec 4, "Conflict", and test 9: a row someone deleted first leaves the
/// grid with the rest, and is said by its key.
#[test]
fn a_row_already_deleted_is_said_and_leaves_with_the_rest() {
    use tabletist_db::Value::Int;
    let (mut harness, tab, id) = covers_in(Look::macos());
    mark(&mut harness, tab, id, 0);
    mark(&mut harness, tab, id, 1);
    harness.app.apply(Action::WriteEdits { tab, id });
    // The second of the set's deletes: the row with id 2.
    harness.answer_written(written(vec![1]));
    harness.settle();
    assert_eq!(page_ids(&harness, tab, id), [Int(3)]);
    let edits = edits(&harness, tab, id);
    assert_eq!(
        edits.note,
        Some(crate::edit::Note::AlreadyDeleted(vec!["id 2".into()]))
    );
    assert_eq!(edits.saved.as_ref().map(|saved| saved.deleted), Some(1));
}

/// Spec 5 and test 8, as the app sees it: a refused delete leaves the row
/// marked, with the database's error on it, and everything else pending.
#[test]
fn a_refused_delete_stays_marked_with_its_error() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    set(&mut harness, tab, id, (0, 2), "audio");
    mark(&mut harness, tab, id, 1);
    harness.app.apply(Action::WriteEdits { tab, id });
    let error = tabletist_db::Error::Query {
        code: Some("23503".into()),
        message: "update or delete on table \"book_covers\" violates foreign key constraint".into(),
        detail: None,
        hint: None,
        named: Box::default(),
    };
    harness.answer_written(Ok(tabletist_db::WriteOutcome::FailedDelete {
        delete: 0,
        error: error.clone(),
    }));
    harness.settle();
    let edits = edits(&harness, tab, id);
    let failed = edits
        .deleted
        .get(&1)
        .and_then(|doomed| doomed.failed.clone());
    assert_eq!(failed, Some(error));
    assert_eq!(edits.counts().changes, 1);
    assert_eq!(page_ids(&harness, tab, id).len(), 3);
}
