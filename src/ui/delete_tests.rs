//! Deleting rows, checked against its spec
//! (`docs/superpowers/specs/2026-10-12-deleting-rows-design.md`: "spec N"
//! below is a section of its Behavior, "test N" an entry of its Tests).
//!
//! Only behaviour, names and words are checked here, on the Bookshop's
//! `book_covers`. No colour, size or picture of the design is.

use egui::{Key, Modifiers};

use super::duplicate_tests::button;
use super::insert_audit_tests::{
    add, covers_as, covers_in, edits, painted, select, selected, set, type_key,
};
use crate::backend::Command;
use crate::model::{Action, ConnTabId, TabId};
use crate::testing::Harness;
use crate::theme::Look;

/// The desktop looks: the keys and the buttons are the same in both.
fn desktop() -> [Look; 2] {
    [Look::standard(), Look::macos()]
}

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

/// Spec 1 and test 1, in the desktop looks: Backspace and Delete mark the
/// row, again restore it, and the key that puts a cell back restores it
/// too.
#[test]
fn backspace_marks_the_row_and_again_restores_it() {
    for look in desktop() {
        let (mut harness, tab, id) = covers_in(look);
        select(&mut harness, tab, id, (1, 2));
        harness.press(Key::Backspace, Modifiers::NONE);
        assert!(edits(&harness, tab, id).deleted.contains_key(&1));
        assert_eq!(selected(&harness, tab, id), Some((1, 2)));
        harness.press(Key::Backspace, Modifiers::NONE);
        assert!(edits(&harness, tab, id).deleted.is_empty());
        harness.press(Key::Delete, Modifiers::NONE);
        assert!(edits(&harness, tab, id).deleted.contains_key(&1));
        harness.press(Key::Z, Modifiers::COMMAND);
        assert!(edits(&harness, tab, id).deleted.is_empty());
    }
}

/// The same in the Omarchy look: `dd`, `dd` again, and `u`.
#[test]
fn dd_marks_the_row_and_again_restores_it() {
    let (mut harness, tab, id) = covers_in(Look::omarchy());
    select(&mut harness, tab, id, (1, 2));
    type_key(&mut harness, Key::D, "d");
    // One d waits for the other.
    assert!(edits(&harness, tab, id).deleted.is_empty());
    type_key(&mut harness, Key::D, "d");
    assert!(edits(&harness, tab, id).deleted.contains_key(&1));
    type_key(&mut harness, Key::D, "d");
    type_key(&mut harness, Key::D, "d");
    assert!(edits(&harness, tab, id).deleted.is_empty());
    type_key(&mut harness, Key::D, "d");
    type_key(&mut harness, Key::D, "d");
    assert!(edits(&harness, tab, id).deleted.contains_key(&1));
    type_key(&mut harness, Key::U, "u");
    assert!(edits(&harness, tab, id).deleted.is_empty());
}

/// Test 2: with a cell's editor open the key edits its text.
#[test]
fn backspace_in_an_open_editor_never_marks_the_row() {
    for look in desktop() {
        let (mut harness, tab, id) = covers_in(look);
        let cell = crate::model::CellPos { row: 1, col: 2 };
        let start = crate::model::EditStart::Replace("audio".into());
        harness.app.apply(Action::EditCell {
            tab,
            id,
            cell,
            start,
        });
        harness.settle();
        harness.press(Key::Backspace, Modifiers::NONE);
        let edits = edits(&harness, tab, id);
        assert!(edits.deleted.is_empty());
        assert!(edits.editor.is_some());
    }
}

/// Spec 2 and test 6: the key says why a row cannot be deleted, in the
/// spec's words, and marks nothing.
#[test]
fn the_key_says_why_a_row_cannot_be_deleted() {
    let cases = [
        (
            tabletist_db::ObjectKind::Table,
            true,
            "Read-only connection",
        ),
        (
            tabletist_db::ObjectKind::View,
            false,
            "book_covers can't be deleted from",
        ),
    ];
    for (kind, read_only, reason) in cases {
        let (mut harness, tab, id) = covers_as(Look::macos(), kind, read_only);
        select(&mut harness, tab, id, (1, 2));
        harness.press(Key::Backspace, Modifiers::NONE);
        assert!(edits(&harness, tab, id).deleted.is_empty());
        assert_eq!(
            crate::ui::toast::said(&harness.ctx).as_deref(),
            Some(reason)
        );
    }
}

/// The Omarchy look says it in its own lower case, and keeps the table's
/// name.
#[test]
fn dd_says_why_a_row_cannot_be_deleted() {
    let view = tabletist_db::ObjectKind::View;
    let (mut harness, tab, id) = covers_as(Look::omarchy(), view, false);
    select(&mut harness, tab, id, (1, 2));
    type_key(&mut harness, Key::D, "d");
    type_key(&mut harness, Key::D, "d");
    assert!(edits(&harness, tab, id).deleted.is_empty());
    assert_eq!(
        crate::ui::toast::said(&harness.ctx).as_deref(),
        Some("book_covers can't be deleted from")
    );
}

#[test]
fn a_table_without_a_key_says_its_rows_cannot_be_deleted_here() {
    let locale = crate::i18n::Locale::default();
    assert_eq!(
        crate::ui::cell_editor::delete_text(crate::edit::Lock::NoKey, "book_imports", locale),
        "book_imports has no primary key, so rows can't be deleted here"
    );
    // What holds for a while is said as an edit's reason is.
    assert_eq!(
        crate::ui::cell_editor::delete_text(crate::edit::Lock::Saving, "book_covers", locale),
        "A save is running"
    );
}

/// Spec 3: the Omarchy look says in its status line why a marked row's
/// cell takes no edit.
#[test]
fn a_marked_rows_cell_says_restore_the_row_to_edit_it() {
    let (mut harness, tab, id) = covers_in(Look::omarchy());
    mark(&mut harness, tab, id, 1);
    select(&mut harness, tab, id, (1, 2));
    type_key(&mut harness, Key::I, "i");
    assert!(edits(&harness, tab, id).editor.is_none());
    assert!(painted(&harness, "restore the row to edit it."));
}

/// Spec 3 and 6: a marked row stays where it is, says that it is marked,
/// and its values are struck through. The desktop looks write a minus
/// before the row's key, which is struck with the rest.
#[test]
fn a_marked_row_is_struck_through_and_says_so() {
    for look in desktop() {
        let (mut harness, tab, id) = covers_in(look);
        // Only the grid's texts: the row panel writes the row's values too.
        harness.app.workspace_mut(tab).unwrap().row_panel = false;
        mark(&mut harness, tab, id, 1);
        assert!(harness.has("Row 2, marked for deletion"));
        assert!(harness.has("Row 1"));
        let struck =
            |harness: &Harness, text: &str| harness.struck.iter().any(|piece| piece == text);
        // The row's kind, its NULL as text (no chip stands in a marked
        // row), and its key after the minus.
        assert!(struck(&harness, "ebook"));
        assert!(struck(&harness, "NULL"));
        assert!(struck(&harness, "2"));
        assert!(painted(&harness, "\u{2212} 2"));
        // Another row's values are as they were.
        assert!(!struck(&harness, "print"));
        // Restored, nothing is struck.
        harness.app.apply(Action::RestoreRow { tab, id });
        harness.settle();
        assert!(harness.struck.is_empty());
        assert!(harness.has("Row 2"));
        assert!(!harness.has("Row 2, marked for deletion"));
    }
}

/// The Omarchy look keeps the key as it is, struck, and marks the gutter.
#[test]
fn the_terminal_marks_a_deleted_row_in_its_gutter() {
    let (mut harness, tab, id) = covers_in(Look::omarchy());
    harness.app.workspace_mut(tab).unwrap().row_panel = false;
    mark(&mut harness, tab, id, 1);
    assert!(painted(&harness, "-"));
    assert!(harness.struck.iter().any(|piece| piece == "2"));
    assert!(!painted(&harness, "\u{2212} 2"));
}

/// Row v: a marked row's edit is not drawn. The row shows what it loaded.
#[test]
fn a_marked_rows_edit_is_drawn_as_the_row_is() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    set(&mut harness, tab, id, (1, 2), "audio");
    mark(&mut harness, tab, id, 1);
    assert!(harness.struck.iter().any(|piece| piece == "ebook"));
    assert!(!painted(&harness, "audio"));
}

/// Spec 3: the row count and the pending bar's summary.
#[test]
fn a_marked_row_is_counted_where_the_pending_changes_are() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    set(&mut harness, tab, id, (0, 2), "audio");
    add(&mut harness, tab, id);
    mark(&mut harness, tab, id, 1);
    assert!(harness.has("3 rows + 1 new · 1 to delete · 5 columns · main"));
    assert!(harness.has("1 new row · 1 change in 1 row · 1 to delete"));
}

/// The Omarchy status line counts the rows to delete, and its key is one
/// of the keys that work.
#[test]
fn the_terminals_status_line_counts_the_rows_to_delete() {
    let (mut harness, tab, id) = covers_in(Look::omarchy());
    // The row panel's footer names the key too: only the line's is asked.
    harness.app.workspace_mut(tab).unwrap().row_panel = false;
    mark(&mut harness, tab, id, 1);
    assert!(painted(&harness, "-1 to delete"));
    let palette = harness.app.palette;
    // A hint is one piece, its key and its word, in the colour of a key
    // that works.
    let works =
        |(text, color): &(String, egui::Color32)| text == "dd delete" && *color == palette.text;
    assert!(harness.painted.iter().any(works), "{:?}", harness.painted);
}

#[test]
fn a_save_that_deleted_says_how_many_rows() {
    let locale = crate::i18n::Locale::default();
    let saved = crate::edit::Saved {
        at: std::time::Instant::now(),
        cells: Vec::new(),
        changes: 0,
        rows: 0,
        added: 0,
        deleted: 2,
        elapsed: std::time::Duration::from_millis(14),
    };
    assert_eq!(
        crate::ui::pending_bar::written_text(&saved, locale),
        "written 2 rows deleted · 14 ms"
    );
}

fn restrict() -> tabletist_db::Error {
    tabletist_db::Error::Query {
        code: Some("23503".into()),
        message: "update or delete on table \"publishers\" violates foreign key constraint \
                  \"book_covers_publisher_id_fkey\" on table \"book_covers\""
            .into(),
        detail: None,
        hint: None,
        named: Box::default(),
    }
}

/// Spec 5: the banner, then the database's words with the constraint's
/// name. The row stays marked.
#[test]
fn a_refused_delete_says_that_nothing_was_saved_and_why() {
    for look in desktop() {
        let (mut harness, tab, id) = covers_in(look);
        mark(&mut harness, tab, id, 1);
        harness.app.apply(Action::WriteEdits { tab, id });
        harness.answer_written(Ok(tabletist_db::WriteOutcome::FailedDelete {
            delete: 0,
            error: restrict(),
        }));
        harness.settle();
        assert!(harness.has(
            "Nothing was saved. 1 row failed, so the whole transaction rolled back. 23503 · \
             update or delete on table \"publishers\" violates foreign key constraint \
             \"book_covers_publisher_id_fkey\" on table \"book_covers\"."
        ));
        assert!(harness.has("Row 2, marked for deletion"));
    }
}

/// The Omarchy look says it on its error line, by the row's number.
#[test]
fn the_terminal_says_a_refused_delete_on_its_error_line() {
    let (mut harness, tab, id) = covers_in(Look::omarchy());
    mark(&mut harness, tab, id, 1);
    harness.app.apply(Action::WriteEdits { tab, id });
    harness.answer_written(Ok(tabletist_db::WriteOutcome::FailedDelete {
        delete: 0,
        error: restrict(),
    }));
    harness.settle();
    let mut lines = harness.painted.iter();
    let line = lines.find(|(text, _)| text.starts_with("! 2: "));
    assert!(
        line.is_some_and(|(text, _)| text.contains("23503")),
        "{:?}",
        harness.painted
    );
}

/// Spec 4 and test 9: the words for a row someone deleted first.
#[test]
fn a_row_already_deleted_is_said_by_its_key() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    mark(&mut harness, tab, id, 0);
    mark(&mut harness, tab, id, 1);
    harness.app.apply(Action::WriteEdits { tab, id });
    harness.answer_written(written(vec![1]));
    harness.settle();
    assert!(harness.has("Row id 2 was already deleted."));
    // And where two were.
    let note = crate::edit::Note::AlreadyDeleted(vec!["id 2".into(), "id 3".into()]);
    let workspace = harness.app.workspace(tab).unwrap();
    let object = workspace.object_tab(id).unwrap();
    assert_eq!(
        crate::ui::pending_bar::note_text(&note, object, harness.app.locale),
        "2 rows were already deleted: id 2, id 3."
    );
}

/// Spec 4, "Production confirm": the deletes are counted with the rest.
#[test]
fn the_production_confirm_counts_the_rows_to_delete() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    harness.app.workspace_mut(tab).unwrap().environment = crate::env::Environment::Production;
    set(&mut harness, tab, id, (0, 2), "audio");
    // A new row needs its publisher before a save is offered.
    let new = add(&mut harness, tab, id);
    let publisher = (crate::edit::new_row(new), 1);
    set(&mut harness, tab, id, publisher, "9100000000000000004");
    mark(&mut harness, tab, id, 1);
    harness.app.apply(Action::WriteEdits { tab, id });
    harness.settle();
    assert!(harness.has("Save 1 new row, 1 change and 1 deleted row to production?"));
}

/// A save that only deletes says only that.
#[test]
fn the_production_confirm_of_deletes_alone_names_no_change() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    harness.app.workspace_mut(tab).unwrap().environment = crate::env::Environment::Production;
    mark(&mut harness, tab, id, 0);
    mark(&mut harness, tab, id, 1);
    harness.app.apply(Action::WriteEdits { tab, id });
    harness.settle();
    assert!(harness.has("Save 2 deleted rows to production?"));
}

/// Spec 1 and 6: the panel's Delete marks the row, by its plain name.
#[test]
fn the_panels_delete_marks_the_row() {
    for look in desktop() {
        let (mut harness, tab, id) = covers_in(look);
        select(&mut harness, tab, id, (1, 2));
        assert_eq!(button(&mut harness, "Delete"), (true, None));
        harness.click("Delete");
        harness.settle();
        assert!(edits(&harness, tab, id).deleted.contains_key(&1));
    }
}

/// Spec 3: the line at the panel's top, its Restore, and Delete that
/// waits with the line's words.
#[test]
fn a_marked_rows_panel_says_so_and_restores_it() {
    for look in desktop() {
        let (mut harness, tab, id) = covers_in(look);
        mark(&mut harness, tab, id, 1);
        assert!(harness.has("Will be deleted on save"));
        assert_eq!(button(&mut harness, "Restore"), (true, None));
        assert_eq!(
            button(&mut harness, "Delete"),
            (false, Some("Will be deleted on save".into()))
        );
        harness.click("Restore");
        harness.settle();
        assert!(edits(&harness, tab, id).deleted.is_empty());
        assert!(!harness.has("Will be deleted on save"));
    }
}

/// The Omarchy look's line names the key that restores, and its footer
/// names it as a key that works.
#[test]
fn the_terminals_panel_names_the_key_that_restores() {
    let (mut harness, tab, id) = covers_in(Look::omarchy());
    mark(&mut harness, tab, id, 1);
    assert!(harness.has("will be deleted on save · dd restore"));
    assert!(painted(&harness, "dd restore"));
}

/// Spec 2 and 6: where the row cannot be deleted, Delete is disabled with
/// the reason as its description.
#[test]
fn the_panels_delete_says_why_it_cannot_be_pressed() {
    let table = tabletist_db::ObjectKind::Table;
    let (mut harness, tab, id) = covers_as(Look::macos(), table, true);
    select(&mut harness, tab, id, (1, 2));
    assert_eq!(
        button(&mut harness, "Delete"),
        (false, Some("Read-only connection".into()))
    );
}

/// Spec 5: the row a delete was refused for shows the database's words in
/// its panel, under the line that says it is marked.
#[test]
fn a_refused_deletes_panel_shows_what_the_database_said() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    mark(&mut harness, tab, id, 1);
    harness.app.apply(Action::WriteEdits { tab, id });
    harness.answer_written(Ok(tabletist_db::WriteOutcome::FailedDelete {
        delete: 0,
        error: tabletist_db::Error::query("FOREIGN KEY constraint failed"),
    }));
    harness.settle();
    assert!(harness.has("Will be deleted on save"));
    assert!(harness.has("FOREIGN KEY constraint failed."));
}

/// Spec 6 and test 12: the three announcements, word for word, each a
/// polite live region.
#[test]
fn a_screen_reader_is_told_of_a_mark_a_restore_and_a_save() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    let polite = |harness: &mut Harness, words: &str| {
        let tree = harness.settle();
        let mut nodes = tree.nodes.iter();
        let node = nodes.find(|(_, node)| node.label().or_else(|| node.value()) == Some(words));
        node.map(|(_, node)| node.live()) == Some(Some(egui::accesskit::Live::Polite))
    };
    mark(&mut harness, tab, id, 1);
    assert!(polite(
        &mut harness,
        "Row id 2 marked for deletion. 1 to delete."
    ));
    harness.app.apply(Action::RestoreRow { tab, id });
    assert!(polite(&mut harness, "Row id 2 restored."));
    mark(&mut harness, tab, id, 1);
    harness.app.apply(Action::WriteEdits { tab, id });
    harness.answer_written(written(Vec::new()));
    assert!(polite(&mut harness, "1 row deleted."));
}

/// A refused delete's error is the save's that was refused: the next save
/// sends the row again without it, and what that save comes to is what
/// the tab says.
#[test]
fn a_refused_deletes_error_goes_when_the_row_is_sent_again() {
    let (mut harness, tab, id) = covers_in(Look::omarchy());
    mark(&mut harness, tab, id, 1);
    harness.app.apply(Action::WriteEdits { tab, id });
    harness.answer_written(Ok(tabletist_db::WriteOutcome::FailedDelete {
        delete: 0,
        error: restrict(),
    }));
    harness.settle();
    assert_eq!(edits(&harness, tab, id).counts().failed, 1);
    // Another row is changed, and the save is sent again.
    set(&mut harness, tab, id, (0, 2), "audio");
    harness.app.apply(Action::WriteEdits { tab, id });
    let sending = edits(&harness, tab, id);
    assert!(
        sending
            .deleted
            .get(&1)
            .is_some_and(|doomed| doomed.failed.is_none())
    );
    assert_eq!(sending.counts().failed, 0);
    assert_eq!(sending.row_mark(1), crate::edit::RowMark::Deleted);
    // This save fails on the changed row: its failure is the one said,
    // not the delete's of the save before.
    let check = tabletist_db::Error::Query {
        code: Some("23514".into()),
        message: "new row violates check constraint".into(),
        detail: None,
        hint: None,
        named: Box::default(),
    };
    harness.answer_written(Ok(tabletist_db::WriteOutcome::Failed {
        row: 0,
        error: check,
    }));
    harness.settle();
    let mut lines = harness.painted.iter();
    let line = lines.find(|(text, _)| text.starts_with("! "));
    assert!(
        line.is_some_and(|(text, _)| text.starts_with("! 1:") && text.contains("23514")),
        "{:?}",
        harness.painted
    );
    // The row is still marked, and has no error of its own.
    assert_eq!(
        edits(&harness, tab, id).row_mark(1),
        crate::edit::RowMark::Deleted
    );
}

/// A save whose new row comes back unknown (a table with a trigger, say)
/// has its page read again. What it found already deleted is still said,
/// before the page arrives and after.
#[test]
fn a_row_already_deleted_is_said_where_the_page_is_read_again() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    let new = add(&mut harness, tab, id);
    let publisher = (crate::edit::new_row(new), 1);
    set(&mut harness, tab, id, publisher, "9100000000000000004");
    mark(&mut harness, tab, id, 1);
    harness.app.apply(Action::WriteEdits { tab, id });
    harness.answer_written(Ok(tabletist_db::WriteOutcome::Written {
        inserted: vec![None],
        rows: Vec::new(),
        gone: vec![0],
        elapsed: std::time::Duration::from_millis(14),
    }));
    let note = Some(crate::edit::Note::AlreadyDeleted(vec!["id 2".into()]));
    assert_eq!(edits(&harness, tab, id).note, note);
    // The page as the database now has it.
    harness.answer_rows(crate::testing::book_covers_page(3));
    harness.settle();
    assert_eq!(edits(&harness, tab, id).note, note);
    assert!(harness.has("Row id 2 was already deleted."));
}

/// The Bookshop's publishers, writable, the grid holding the keyboard:
/// `book_covers` loses its covers with a publisher, and `books` is
/// cleared of it.
fn publishers_in(look: Look) -> (Harness, ConnTabId, TabId) {
    let mut harness = Harness::new();
    harness.set_look(look);
    let tab = harness.connect_fake_as(false);
    harness.app.apply(Action::OpenObject {
        tab,
        object: tabletist_db::ObjectRef::new("main", "publishers"),
        kind: tabletist_db::ObjectKind::Table,
        pin: true,
    });
    harness.answer_structure(crate::testing::publishers_structure());
    harness.answer_rows(crate::testing::publishers_page());
    let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
    harness.app.workspace_mut(tab).unwrap().pane = crate::model::Pane::Grid;
    harness.settle();
    (harness, tab, id)
}

/// The counts asked of other tables than the publishers' own, by the
/// table each is of.
fn counts_asked(harness: &Harness) -> Vec<(crate::backend::RequestId, String)> {
    let sent = harness.app.backend.sent.iter();
    sent.filter_map(|command| match command {
        Command::CountRows { request, query, .. } if query.object.name != "publishers" => {
            Some((*request, query.object.name.clone()))
        }
        _ => None,
    })
    .collect()
}

/// The requests the app asked the backend to stop.
fn stopped(harness: &Harness) -> Vec<crate::backend::RequestId> {
    let sent = harness.app.backend.sent.iter();
    sent.filter_map(|command| match command {
        Command::Cancel { request, .. } => Some(*request),
        _ => None,
    })
    .collect()
}

fn answer_count(
    harness: &mut Harness,
    tab: ConnTabId,
    request: crate::backend::RequestId,
    rows: u64,
) {
    let session = harness.app.workspace(tab).unwrap().session;
    let result = Ok(rows);
    harness
        .app
        .apply(Action::Backend(crate::backend::Event::Count {
            session,
            request,
            result,
        }));
    harness.settle();
}

fn review(harness: &mut Harness, tab: ConnTabId, id: TabId, show: bool) {
    harness.app.apply(Action::ReviewEdits { tab, id, show });
    harness.settle();
}

const ALSO_DELETES: &str = "-- also deletes 3 rows in book_covers (ON DELETE CASCADE)";

/// Spec 4, "Cascades", and test 10: Review SQL asks for a count for each
/// key that acts, and says each once it has it.
#[test]
fn review_sql_says_what_the_deletes_take_with_them() {
    let (mut harness, tab, id) = publishers_in(Look::macos());
    mark(&mut harness, tab, id, 0);
    // Marking alone asks nothing.
    assert!(counts_asked(&harness).is_empty());
    review(&mut harness, tab, id, true);
    let asked = counts_asked(&harness);
    let tables: Vec<&str> = asked.iter().map(|(_, table)| table.as_str()).collect();
    assert_eq!(tables, ["book_covers", "books"]);
    // Until the answer, without a number.
    assert!(harness.has("-- also deletes rows in book_covers (ON DELETE CASCADE)"));
    answer_count(&mut harness, tab, asked[0].0, 3);
    answer_count(&mut harness, tab, asked[1].0, 0);
    assert!(harness.has(ALSO_DELETES));
    // No row of `books` has that publisher: no line.
    let lines = &edits(&harness, tab, id).review.as_ref().unwrap().lines;
    let also = |line: &&crate::review::Line| matches!(line, crate::review::Line::Also(_));
    assert_eq!(lines.iter().filter(also).count(), 1);
    // Kept while the marks stand: closing and opening the review asks
    // nothing again, and the line is there at once.
    review(&mut harness, tab, id, false);
    review(&mut harness, tab, id, true);
    assert_eq!(counts_asked(&harness).len(), 2);
    assert!(harness.has(ALSO_DELETES));
}

/// A mark that changes while the counts are out stops them, and the next
/// review asks for the rows that are marked now.
#[test]
fn a_changed_mark_stops_the_counts_and_asks_again() {
    let (mut harness, tab, id) = publishers_in(Look::macos());
    mark(&mut harness, tab, id, 0);
    review(&mut harness, tab, id, true);
    let first: Vec<_> = counts_asked(&harness)
        .into_iter()
        .map(|(request, _)| request)
        .collect();
    assert_eq!(first.len(), 2);
    mark(&mut harness, tab, id, 1);
    for request in &first {
        assert!(stopped(&harness).contains(request));
    }
    // The review is open: it is made again, and asks again.
    assert_eq!(counts_asked(&harness).len(), 4);
    // An answer of the first round is no longer waited for.
    answer_count(&mut harness, tab, first[0], 9);
    let taken = edits(&harness, tab, id).taken.as_ref().unwrap();
    assert!(
        taken
            .lines()
            .iter()
            .all(|effect| !matches!(effect, crate::edit::Effect::Deletes { rows: Some(_), .. }))
    );
}

/// The same lines in the production confirm.
#[test]
fn the_production_confirm_says_what_the_deletes_take_with_them() {
    let (mut harness, tab, id) = publishers_in(Look::macos());
    harness.app.workspace_mut(tab).unwrap().environment = crate::env::Environment::Production;
    mark(&mut harness, tab, id, 0);
    harness.app.apply(Action::WriteEdits { tab, id });
    harness.settle();
    let asked = counts_asked(&harness);
    assert_eq!(asked.len(), 2);
    answer_count(&mut harness, tab, asked[0].0, 3);
    answer_count(&mut harness, tab, asked[1].0, 2);
    assert!(harness.has(ALSO_DELETES));
    assert!(harness.has("-- also clears publisher_id in 2 books rows (ON DELETE SET NULL)"));
}

/// Review SQL first, then the save: the confirm has the lines at once,
/// and asks nothing again.
#[test]
fn the_production_confirm_has_what_review_sql_counted() {
    let (mut harness, tab, id) = publishers_in(Look::macos());
    harness.app.workspace_mut(tab).unwrap().environment = crate::env::Environment::Production;
    mark(&mut harness, tab, id, 0);
    review(&mut harness, tab, id, true);
    let asked = counts_asked(&harness);
    answer_count(&mut harness, tab, asked[0].0, 3);
    answer_count(&mut harness, tab, asked[1].0, 0);
    harness.app.apply(Action::WriteEdits { tab, id });
    harness.settle();
    assert_eq!(counts_asked(&harness).len(), 2);
    let Some(crate::model::Dialog::ConfirmWrite(prompt)) = &harness.app.dialog else {
        panic!("the production confirm is up");
    };
    let deletes = crate::edit::Effect::Deletes {
        table: "book_covers".into(),
        rows: Some(3),
    };
    assert_eq!(prompt.review.effects(), [deletes]);
}

/// And in the Omarchy look's `:diff`.
#[test]
fn diff_says_what_the_deletes_take_with_them() {
    let (mut harness, tab, id) = publishers_in(Look::omarchy());
    mark(&mut harness, tab, id, 0);
    review(&mut harness, tab, id, true);
    let asked = counts_asked(&harness);
    answer_count(&mut harness, tab, asked[0].0, 3);
    assert!(harness.has(ALSO_DELETES));
}

/// A copy of the review has the lines too.
#[test]
fn a_copied_review_says_what_the_deletes_take_with_them() {
    let (mut harness, tab, id) = publishers_in(Look::macos());
    mark(&mut harness, tab, id, 0);
    review(&mut harness, tab, id, true);
    let asked = counts_asked(&harness);
    answer_count(&mut harness, tab, asked[0].0, 3);
    let whole = harness.app.review_whole(tab, id).unwrap();
    let text = crate::ui::review::text(&whole, harness.app.locale);
    assert!(text.contains(ALSO_DELETES), "{text}");
}

fn refused(code: &str, number: Option<u32>, constraint: Option<&str>) -> tabletist_db::Error {
    tabletist_db::Error::Query {
        code: Some(code.into()),
        message: "FOREIGN KEY constraint failed".into(),
        detail: None,
        hint: None,
        named: Box::new(tabletist_db::Named {
            number,
            constraint: constraint.map(str::to_owned),
            columns: Vec::new(),
        }),
    }
}

/// Refuses the delete of the one marked row with `error`.
fn refuse(harness: &mut Harness, tab: ConnTabId, id: TabId, error: tabletist_db::Error) {
    harness.app.apply(Action::WriteEdits { tab, id });
    harness.answer_written(Ok(tabletist_db::WriteOutcome::FailedDelete {
        delete: 0,
        error,
    }));
    harness.settle();
}

/// Spec 5 and test 8: after the rollback the row says how many rows of
/// which table still point at it.
#[test]
fn a_refused_delete_says_who_still_points_at_the_row() {
    for look in desktop() {
        let (mut harness, tab, id) = publishers_in(look);
        mark(&mut harness, tab, id, 0);
        refuse(&mut harness, tab, id, refused("787", None, None));
        // One count: of the one key that holds.
        let asked = counts_asked(&harness);
        assert_eq!(asked.len(), 1, "{}", look.name);
        let (request, table) = asked.last().unwrap();
        assert_eq!(table, "book_reviews");
        answer_count(&mut harness, tab, *request, 3);
        let held = "3 rows in book_reviews still point at this row";
        // Under the database's words in the row panel, and after them in
        // the bar.
        assert!(harness.has(held), "{}", look.name);
        let bar = "Nothing was saved. 1 row failed, so the whole transaction rolled back. 787 · \
                   FOREIGN KEY constraint failed. 3 rows in book_reviews still point at this row.";
        assert!(harness.has(bar), "{}", look.name);
        let doomed = edits(&harness, tab, id).deleted.get(&0).unwrap();
        assert_eq!(doomed.holder.as_ref().map(|holder| holder.rows), Some(3));
        // Sent again, the row has no error and no holder of the last one.
        harness.app.apply(Action::WriteEdits { tab, id });
        let doomed = edits(&harness, tab, id).deleted.get(&0).unwrap();
        assert!(doomed.failed.is_none() && doomed.holder.is_none());
    }
}

/// MySQL's refusal is told by its number, under the state it shares, and
/// a refusal that is no foreign key's asks nothing.
#[test]
fn only_a_foreign_keys_refusal_is_asked_about() {
    let (mut harness, tab, id) = publishers_in(Look::macos());
    harness.app.workspace_mut(tab).unwrap().driver = tabletist_db::Driver::MySql;
    mark(&mut harness, tab, id, 0);
    let key = Some("book_reviews_publisher_id_fkey");
    refuse(&mut harness, tab, id, refused("23000", Some(1451), key));
    assert_eq!(counts_asked(&harness).len(), 1);
    // No row holds it after all: nothing is said.
    let (request, _) = counts_asked(&harness)[0].clone();
    answer_count(&mut harness, tab, request, 0);
    let doomed = edits(&harness, tab, id).deleted.get(&0).unwrap();
    assert!(doomed.holder.is_none());
    // Another failure under the same state.
    refuse(&mut harness, tab, id, refused("23000", Some(1142), None));
    assert_eq!(counts_asked(&harness).len(), 1);
}

/// The Omarchy look says it on its error line.
#[test]
fn the_terminal_says_who_holds_a_row() {
    let (mut harness, tab, id) = publishers_in(Look::omarchy());
    // The line is cut to the grid's width where it is painted: with the
    // row panel away it has more room, and a screen reader is told the
    // whole of it either way.
    harness.app.workspace_mut(tab).unwrap().row_panel = false;
    mark(&mut harness, tab, id, 0);
    refuse(&mut harness, tab, id, refused("787", None, None));
    let (request, _) = counts_asked(&harness).last().cloned().unwrap();
    answer_count(&mut harness, tab, request, 1);
    let tree = harness.settle();
    let labels = crate::testing::labels(&tree);
    let line = labels.iter().find(|label| label.starts_with("! 1: "));
    // With the key that opens them.
    let said = "1 row in book_reviews still points at this row · gd open rows";
    assert!(
        line.is_some_and(|label| label.ends_with(said)),
        "{labels:?}"
    );
}

/// A row of `publishers` whose delete was refused, held by three rows of
/// `book_reviews`.
fn held_publisher(look: Look) -> (Harness, ConnTabId, TabId) {
    let (mut harness, tab, id) = publishers_in(look);
    mark(&mut harness, tab, id, 0);
    refuse(&mut harness, tab, id, refused("787", None, None));
    let (request, _) = counts_asked(&harness).last().cloned().unwrap();
    answer_count(&mut harness, tab, request, 3);
    (harness, tab, id)
}

/// The table the workspace has in front, and its filter rows.
fn in_front(harness: &Harness, tab: ConnTabId) -> (String, Vec<(String, String)>) {
    let workspace = harness.app.workspace(tab).unwrap();
    let object = workspace.object_tab(workspace.active_tab.unwrap()).unwrap();
    let rows = object.filter.rows.iter();
    (
        object.object.name.clone(),
        rows.map(|row| (row.column.clone(), row.value.clone()))
            .collect(),
    )
}

fn the_publishers_reviews() -> (String, Vec<(String, String)>) {
    let by = ("publisher_id".to_owned(), "9100000000000000001".to_owned());
    ("book_reviews".to_owned(), vec![by])
}

/// Spec 5: Open rows, in the pending bar, opens the referencing table
/// filtered to the rows that point at the row. The publishers' tab keeps
/// its mark.
#[test]
fn open_rows_in_the_bar_opens_the_rows_that_hold_the_row() {
    for look in desktop() {
        let (mut harness, tab, id) = held_publisher(look);
        // Only the bar's: the row panel has one too.
        harness.app.workspace_mut(tab).unwrap().row_panel = false;
        harness.settle();
        assert_eq!(button(&mut harness, "Open rows"), (true, None));
        harness.click("Open rows");
        harness.settle();
        assert_eq!(in_front(&harness, tab), the_publishers_reviews());
        assert!(edits(&harness, tab, id).deleted.contains_key(&0));
    }
}

/// And the button under the sentence in the row panel does.
#[test]
fn open_rows_in_the_row_panel_opens_the_rows_that_hold_the_row() {
    let (mut harness, tab, id) = held_publisher(Look::macos());
    let row = 0;
    harness.app.apply(Action::OpenHolders { tab, id, row });
    harness.settle();
    assert_eq!(in_front(&harness, tab), the_publishers_reviews());
    // The panel names it, beside the bar's.
    let (mut harness, _tab, _id) = held_publisher(Look::macos());
    let tree = harness.settle();
    let named = |(_, node): &&(egui::accesskit::NodeId, egui::accesskit::Node)| {
        node.label() == Some("Open rows")
    };
    assert_eq!(tree.nodes.iter().filter(named).count(), 2);
}

/// In the Omarchy look `gd` on the row opens them.
#[test]
fn gd_on_a_refused_row_opens_the_rows_that_hold_it() {
    let (mut harness, tab, _id) = held_publisher(Look::omarchy());
    type_key(&mut harness, Key::G, "g");
    type_key(&mut harness, Key::D, "d");
    assert_eq!(in_front(&harness, tab), the_publishers_reviews());
}

/// A row that is not held has nothing to open: the action does nothing,
/// and no button offers it.
#[test]
fn a_row_nothing_holds_has_no_rows_to_open() {
    let (mut harness, tab, id) = publishers_in(Look::macos());
    mark(&mut harness, tab, id, 0);
    let row = 0;
    harness.app.apply(Action::OpenHolders { tab, id, row });
    harness.settle();
    assert_eq!(in_front(&harness, tab).0, "publishers");
    assert!(!harness.has("Open rows"));
}

/// Where the table points at itself the rows that hold a row are in the
/// tab that has the mark: opening them would drop its page, so the app
/// asks first, as it does for any tab that holds changes.
#[test]
fn open_rows_asks_first_where_the_rows_are_the_tabs_own() {
    let mut harness = Harness::new();
    harness.set_look(Look::macos());
    let (tab, id) = harness.genres();
    harness.settle();
    // Fiction, which Crime stands under.
    mark(&mut harness, tab, id, 0);
    refuse(&mut harness, tab, id, refused("787", None, None));
    let sent = harness.app.backend.sent.iter();
    let asked = sent.filter_map(|command| match command {
        Command::CountRows { request, query, .. } if !query.filters.is_empty() => Some(*request),
        _ => None,
    });
    let mut asked = asked;
    let request = asked.next_back().expect("who holds Fiction was asked");
    answer_count(&mut harness, tab, request, 1);
    let row = 0;
    harness.app.apply(Action::OpenHolders { tab, id, row });
    harness.settle();
    assert!(matches!(
        harness.app.dialog,
        Some(crate::model::Dialog::Leave(_))
    ));
    // Nothing was dropped: the mark and its page stand.
    assert!(edits(&harness, tab, id).deleted.contains_key(&0));
}

/// A key of the table that points at the table and deletes or clears: the
/// save took, or changed, rows it did not name, some perhaps on the page.
/// The page is read again, and the count with it, where a key that holds
/// leaves the page to be patched.
#[test]
fn a_delete_that_reaches_rows_of_its_own_table_reads_the_page_again() {
    for (on_delete, reads) in [("CASCADE", true), ("SET NULL", true), ("NO ACTION", false)] {
        let mut harness = Harness::new();
        let tab = harness.connect_fake_as(false);
        harness.app.apply(Action::OpenObject {
            tab,
            object: tabletist_db::ObjectRef::new("main", "genres"),
            kind: tabletist_db::ObjectKind::Table,
            pin: true,
        });
        let mut structure = crate::testing::genres_structure();
        structure.referenced_by[0].on_delete = on_delete.into();
        harness.answer_structure(structure);
        harness.answer_rows(crate::testing::genres_page());
        let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
        // Crime, which nothing on the page stands under.
        mark(&mut harness, tab, id, 1);
        harness.app.apply(Action::WriteEdits { tab, id });
        harness.answer_written(written(Vec::new()));
        let workspace = harness.app.workspace(tab).unwrap();
        let object = workspace.object_tab(id).unwrap();
        assert_eq!(object.rows.is_loading(), reads, "{on_delete}");
        // Patched, the page has lost the row; read again, it waits.
        let rows = object.page().map(|page| page.rows.len());
        assert_eq!(rows == Some(1), !reads, "{on_delete}");
        assert!(object.edits.deleted.is_empty(), "{on_delete}");
    }
}
