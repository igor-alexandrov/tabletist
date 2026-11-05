//! Exporting a table, checked against its spec
//! (`docs/superpowers/specs/2026-11-03-export-copy-as-3-table-design.md`:
//! "test N" below is an entry of its Tests).
//!
//! Behaviour and words, on the Bookshop's `book_covers`. The backend is a
//! recording one: what an export would send is checked, and what it would
//! answer is given. No picture of the design is compared.

use std::path::PathBuf;

use egui::{Key, Modifiers};

use super::insert_audit_tests::{painted, select, type_key};
use crate::backend::export::{Destination, ExportFailure, ExportJob, ExportRows, ExportState};
use crate::backend::{Command, Event, HistoryCommand};
use crate::export::Format;
use crate::export_sheet::{DestinationChoice, ExportChange, ExportSheet, RowsChoice};
use crate::model::{Action, ConnTabId, Dialog, TabId};
use crate::testing::Harness;
use crate::theme::Look;

/// The Bookshop's covers in `look`, writable, the grid holding the
/// keyboard, in a window tall enough for the whole sheet.
fn covers_in(look: Look) -> (Harness, ConnTabId, TabId) {
    let mut harness = Harness::with_size(egui::vec2(1400.0, 1800.0));
    harness.set_look(look);
    harness.app.settings.group_digits = Some(false);
    let (tab, id) = harness.book_covers();
    harness.app.workspace_mut(tab).unwrap().pane = crate::model::Pane::Grid;
    harness.settle();
    (harness, tab, id)
}

/// ⇧⌘E, the macOS look's Export….
fn export_chord(harness: &mut Harness) {
    harness.press(Key::E, Modifiers::COMMAND | Modifiers::SHIFT);
    harness.settle();
}

fn sheet(harness: &Harness) -> Option<&ExportSheet> {
    match &harness.app.dialog {
        Some(Dialog::Export(sheet)) => Some(sheet),
        _ => None,
    }
}

fn change(harness: &mut Harness, change: ExportChange) {
    harness.app.apply(Action::ChangeExport(change));
    harness.settle();
}

/// The exports sent to the backend so far.
fn jobs(harness: &Harness) -> Vec<&ExportJob> {
    harness
        .app
        .backend
        .sent
        .iter()
        .filter_map(|command| match command {
            Command::Export(job) => Some(&**job),
            _ => None,
        })
        .collect()
}

fn said(harness: &Harness) -> Option<String> {
    crate::ui::toast::said(&harness.ctx)
}

/// A line of the terminal look's `:` prompt, run: what follows its colon
/// is the prompt's text.
fn run_line(harness: &mut Harness, tab: ConnTabId, line: &str) {
    let text = line.strip_prefix(':').unwrap_or(line);
    harness.app.workspace_mut(tab).unwrap().command = Some(text.into());
    harness.app.apply(Action::RunCommand(tab));
    harness.settle();
}

/// What the backend says of the running export.
fn tell(harness: &mut Harness, state: ExportState) {
    let job = harness.app.exports.last().unwrap().job.id;
    harness
        .app
        .apply(Action::Backend(Event::Export { job, state }));
    harness.settle();
}

/// A folder of the test's own, and a path in it.
fn folder() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("book_covers.csv");
    (dir, path)
}

/// Test 13: ⇧⌘E and the header's Export… open the sheet over the table;
/// it is titled by the table and starts on All rows matching the filter,
/// or on Selected rows over a range.
#[test]
fn the_sheet_opens_over_the_table_with_its_rows_chosen() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    export_chord(&mut harness);
    let open = sheet(&harness).expect("⇧⌘E opens the sheet");
    assert_eq!((open.tab, open.id), (tab, id));
    assert_eq!(open.rows, RowsChoice::Matching);
    assert_eq!(open.format, Format::Csv);
    assert!(
        painted(&harness, "Export book_covers"),
        "{:?}",
        harness.painted
    );
    assert!(painted(&harness, "All rows matching the filter"));
    assert!(painted(&harness, "Export…") || harness.has("Export…"));
    harness.app.apply(Action::CloseDialog);
    harness.settle();
    // A range of three rows: Selected rows, with its count.
    select(&mut harness, tab, id, (0, 1));
    harness.press(Key::ArrowDown, Modifiers::SHIFT);
    harness.press(Key::ArrowDown, Modifiers::SHIFT);
    harness.click("Export…");
    harness.settle();
    let open = sheet(&harness).expect("the header's Export… opens it");
    assert_eq!(open.rows, RowsChoice::Selected);
    assert!(painted(&harness, "Selected rows"));
    assert!(painted(&harness, "3"), "the selection's count");
}

/// Test 8: the macOS look asks the system's panel where the file goes,
/// then starts the export there, the sheet closed and the format's options
/// remembered.
#[test]
fn the_save_panel_names_the_file_and_the_export_starts() {
    let (mut harness, _tab, _id) = covers_in(Look::macos());
    export_chord(&mut harness);
    change(&mut harness, ExportChange::Format(Format::Json));
    let mut options = sheet(&harness).unwrap().options.clone();
    options.json.layout = crate::export::Layout::OneLine;
    change(&mut harness, ExportChange::Options(Box::new(options)));
    harness.press(Key::Enter, Modifiers::NONE);
    let asks = harness.app.backend.export_asks.clone();
    assert_eq!(asks.len(), 1, "↩ asks the panel");
    assert!(asks[0].1.ends_with("book_covers.json"), "{asks:?}");
    assert!(sheet(&harness).unwrap().asking);
    assert!(jobs(&harness).is_empty(), "nothing runs before the answer");
    // The panel closed without a choice: the sheet stays.
    let job = asks[0].0;
    harness.app.apply(Action::Backend(Event::ExportPath {
        job,
        path: None,
        replace: false,
    }));
    assert!(sheet(&harness).is_some_and(|sheet| !sheet.asking));
    let (_dir, path) = folder();
    harness.press(Key::Enter, Modifiers::NONE);
    let job = harness.app.backend.export_asks[1].0;
    harness.app.apply(Action::Backend(Event::ExportPath {
        job,
        path: Some(path.clone()),
        replace: false,
    }));
    harness.settle();
    assert!(sheet(&harness).is_none());
    let sent = jobs(&harness);
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].format, Format::Json);
    // No file was there as the panel answered: one made since is
    // refused, not replaced.
    assert_eq!(
        sent[0].destination,
        Destination::File {
            path,
            replace: false
        }
    );
    assert!(matches!(sent[0].rows, ExportRows::Query(_)));
    assert_eq!(
        sent[0].columns,
        ["id", "publisher_id", "kind", "image_data", "created_at"]
    );
    // Test 14 of run 2's spec, here: remembered when it starts.
    assert_eq!(
        harness.app.export.json.layout,
        crate::export::Layout::OneLine
    );
    let saved = harness.app.backend.sent.iter().any(|command| {
        matches!(
            command,
            Command::Save {
                file: crate::backend::StateFile::ExportOptions(_),
                ..
            }
        )
    });
    assert!(saved, "the options are written to export_options.json");
}

/// The panel asks before it replaces a file that is there as it answers:
/// that file is replaced, and only that one.
#[test]
fn a_file_there_as_the_panel_answers_is_replaced() {
    let (mut harness, _tab, _id) = covers_in(Look::macos());
    export_chord(&mut harness);
    harness.press(Key::Enter, Modifiers::NONE);
    let job = harness.app.backend.export_asks[0].0;
    let (_dir, path) = folder();
    harness.app.apply(Action::Backend(Event::ExportPath {
        job,
        path: Some(path.clone()),
        replace: true,
    }));
    harness.settle();
    assert_eq!(
        jobs(&harness)[0].destination,
        Destination::File {
            path,
            replace: true
        }
    );
}

/// Test 7: chosen and reordered columns are the job's, in that order; the
/// keys move the one the list's cursor is on.
#[test]
fn chosen_and_moved_columns_are_written_in_their_order() {
    let (mut harness, _tab, _id) = covers_in(Look::macos());
    export_chord(&mut harness);
    change(&mut harness, ExportChange::Choose(true));
    assert!(painted(&harness, "image_data"), "the column list");
    change(&mut harness, ExportChange::Toggle(3));
    change(&mut harness, ExportChange::Cursor(2));
    let held = Modifiers::ALT;
    harness.press(Key::ArrowUp, held);
    harness.press(Key::ArrowUp, held);
    let columns: Vec<&str> = sheet(&harness)
        .unwrap()
        .chosen()
        .iter()
        .map(|column| column.name.as_str())
        .collect();
    assert_eq!(columns, ["kind", "id", "publisher_id", "created_at"]);
    change(
        &mut harness,
        ExportChange::Destination(DestinationChoice::Clipboard),
    );
    harness.press(Key::Enter, Modifiers::NONE);
    assert_eq!(
        jobs(&harness)[0].columns,
        ["kind", "id", "publisher_id", "created_at"]
    );
}

/// Test 6 and 10: the loaded rows (D7) go to the clipboard through the
/// export, which says what it copied; nothing is recorded, as nothing ran.
#[test]
fn selected_rows_go_to_the_clipboard_and_nothing_is_recorded() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    select(&mut harness, tab, id, (0, 1));
    harness.press(Key::ArrowDown, Modifiers::SHIFT);
    export_chord(&mut harness);
    change(&mut harness, ExportChange::Format(Format::Tsv));
    change(
        &mut harness,
        ExportChange::Destination(DestinationChoice::Clipboard),
    );
    assert!(painted(&harness, "Copy 2 rows"), "{:?}", harness.painted);
    harness.press(Key::Enter, Modifiers::NONE);
    let job = jobs(&harness)[0].clone();
    assert_eq!(job.destination, Destination::Clipboard);
    let ExportRows::Loaded { rows, .. } = &job.rows else {
        panic!("{:?}", job.rows);
    };
    assert_eq!(rows.len(), 2);
    let history = harness.app.backend.sent.len();
    tell(
        &mut harness,
        ExportState::Done {
            rows: 2,
            bytes: 9,
            took: std::time::Duration::from_millis(5),
            copied: Some("a\tb\nc\td".into()),
            statements: 0,
        },
    );
    assert_eq!(harness.copied.as_deref(), Some("a\tb\nc\td"));
    assert_eq!(said(&harness).as_deref(), Some("Copied 2 rows as TSV"));
    assert!(harness.app.exports.is_empty());
    let recorded = harness.app.backend.sent[history..]
        .iter()
        .any(|command| matches!(command, Command::History(HistoryCommand::Append { .. })));
    assert!(!recorded, "Test 14: nothing ran, so nothing is recorded");
    // The next sheet of TSV starts on the clipboard (Q12).
    export_chord(&mut harness);
    assert_eq!(
        sheet(&harness).unwrap().destination,
        DestinationChoice::Clipboard
    );
}

/// Test 10: over a known 10,000 rows the clipboard is refused, in the
/// sheet's words, and ↩ copies nothing.
#[test]
fn the_clipboard_is_refused_over_ten_thousand_rows() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    harness
        .app
        .workspace_mut(tab)
        .unwrap()
        .object_tab_mut(id)
        .unwrap()
        .estimated_rows = Some(31_400);
    export_chord(&mut harness);
    assert!(painted(&harness, "≈ 31,400"));
    assert!(harness.app.export_facts().unwrap().clipboard_refused);
    change(
        &mut harness,
        ExportChange::Destination(DestinationChoice::Clipboard),
    );
    assert_eq!(
        sheet(&harness).unwrap().destination,
        DestinationChoice::File
    );
    harness.press(Key::Enter, Modifiers::NONE);
    assert!(jobs(&harness).is_empty(), "nothing goes to the clipboard");
}

/// Test 9: the note says how many changes are not exported, and only
/// while there are some.
#[test]
fn unsaved_changes_are_said_and_not_exported() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    export_chord(&mut harness);
    assert!(
        !harness
            .painted
            .iter()
            .any(|(text, _)| text.contains("unsaved"))
    );
    harness.app.apply(Action::CloseDialog);
    harness.app.apply(Action::AddRow {
        tab,
        id,
        place: crate::edit::Place::Top,
    });
    harness.press(Key::Escape, Modifiers::NONE);
    export_chord(&mut harness);
    let note = harness
        .painted
        .iter()
        .find(|(text, _)| text.contains("isn't exported"))
        .map(|(text, _)| text.clone());
    let note = note.expect("the note shows with a change pending");
    assert!(
        note.starts_with("1 unsaved change in book_covers"),
        "{note}"
    );
    // What is written is what is saved: the query, not the grid.
    change(&mut harness, ExportChange::Rows(RowsChoice::Whole));
    let (_dir, path) = folder();
    harness.press(Key::Enter, Modifiers::NONE);
    let job = harness.app.backend.export_asks[0].0;
    harness.app.apply(Action::Backend(Event::ExportPath {
        job,
        path: Some(path),
        replace: false,
    }));
    let ExportRows::Query(query) = &jobs(&harness)[0].rows else {
        panic!()
    };
    assert!(query.filters.is_empty() && query.sort.is_empty());
}

/// Test 11 and 14: the states the toasts say, their buttons, and the
/// history's entry of the statement that ran.
#[test]
fn the_toasts_follow_an_export_to_its_end() {
    let (mut harness, _tab, _id) = covers_in(Look::macos());
    export_chord(&mut harness);
    let (_dir, path) = folder();
    harness.press(Key::Enter, Modifiers::NONE);
    let job = harness.app.backend.export_asks[0].0;
    harness.app.apply(Action::Backend(Event::ExportPath {
        job,
        path: Some(path.clone()),
        replace: false,
    }));
    harness.settle();
    tell(&mut harness, ExportState::Started { statement: None });
    tell(&mut harness, ExportState::Progress { rows: 2, bytes: 90 });
    assert!(
        painted(&harness, "Exporting book_covers.csv"),
        "{:?}",
        harness.painted
    );
    harness.click("Hide");
    assert!(harness.app.exports.last().unwrap().hidden);
    assert!(
        harness.has("Exporting book_covers.csv"),
        "the pill, as a screen reader hears it"
    );
    harness.click("Exporting book_covers.csv");
    assert!(!harness.app.exports.last().unwrap().hidden);
    harness.click("Cancel");
    assert!(
        harness
            .app
            .backend
            .sent
            .iter()
            .any(|command| matches!(command, Command::CancelExport(id) if *id == job))
    );
    // It ends well after all.
    tell(
        &mut harness,
        ExportState::Done {
            rows: 3,
            bytes: 7_800_000,
            took: std::time::Duration::from_secs(18),
            copied: None,
            statements: 0,
        },
    );
    assert!(painted(
        &harness,
        "Exported 3 rows to book_covers.csv · 7.8 MB · 18 s"
    ));
    let reveal = crate::ui::export_toast::reveal_words(harness.app.locale);
    harness.click(&reveal);
    assert!(
        harness
            .app
            .backend
            .sent
            .iter()
            .any(|command| matches!(command, Command::Reveal { path: shown } if *shown == path))
    );
    let entry = harness
        .app
        .backend
        .sent
        .iter()
        .find_map(|command| match command {
            Command::History(HistoryCommand::Append { entry, .. }) => Some(entry.clone()),
            _ => None,
        });
    let entry = entry.expect("Test 14: the export is recorded");
    assert_eq!(entry.source, crate::history::Source::Export);
    assert_eq!(entry.rows, Some(3));
    let sql = entry.sql.unwrap();
    assert!(sql.starts_with("SELECT \"id\", \"publisher_id\""), "{sql}");
    assert!(!sql.contains("LIMIT"), "{sql}");
}

/// Test 11: a failure stays, in the prompt's words, and Try again sends
/// the same export; a cancel says nothing was saved.
#[test]
fn a_failure_stays_until_dismissed_and_try_again_sends_it_again() {
    let (mut harness, tab, _id) = covers_in(Look::macos());
    let (_dir, path) = folder();
    harness.app.apply(Action::ExportTo {
        tab,
        path: path.display().to_string(),
        replace: false,
    });
    harness.settle();
    tell(
        &mut harness,
        ExportState::Progress {
            rows: 2,
            bytes: 6_100_000,
        },
    );
    tell(
        &mut harness,
        ExportState::Failed(ExportFailure::DiskFull { written: 6_100_000 }),
    );
    assert!(
        painted(
            &harness,
            "The disk is full. 6.1 MB written. The partial file was removed."
        ),
        "{:?}",
        harness.painted
    );
    harness.click("Try again");
    assert_eq!(jobs(&harness).len(), 2);
    // The same export, under an id of its own.
    assert_eq!(jobs(&harness)[0].destination, jobs(&harness)[1].destination);
    assert_ne!(jobs(&harness)[0].id, jobs(&harness)[1].id);
    tell(&mut harness, ExportState::Cancelled);
    assert!(painted(&harness, crate::ui::export_toast::CANCELLED));
    tell(
        &mut harness,
        ExportState::Failed(ExportFailure::ConnectionLost { rows: 2 }),
    );
    harness.click("Dismiss");
    assert!(harness.app.exports.is_empty());
}

/// Test 12: `:export`, `:export json`, `:w <path>` with the format from
/// its extension, `:w!`, an unknown extension, a file that is there, and
/// `:export cancel`.
#[test]
fn the_terminal_looks_lines_export() {
    let (mut harness, tab, _id) = covers_in(Look::omarchy());
    run_line(&mut harness, tab, ":export json");
    assert_eq!(sheet(&harness).unwrap().format, Format::Json);
    assert!(sheet(&harness).unwrap().path.ends_with("book_covers.json"));
    harness.app.apply(Action::CloseDialog);
    run_line(&mut harness, tab, ":export");
    assert_eq!(sheet(&harness).unwrap().format, Format::Csv);
    harness.app.apply(Action::CloseDialog);
    run_line(&mut harness, tab, ":export cancel");
    assert_eq!(said(&harness).as_deref(), Some("no export is running"));

    let (dir, _) = folder();
    let unknown = dir.path().join("covers.xyz");
    run_line(&mut harness, tab, &format!(":w {}", unknown.display()));
    assert_eq!(
        said(&harness).as_deref(),
        Some("✗ no format for .xyz · csv tsv json jsonl sql md")
    );
    let there = dir.path().join("covers.md");
    std::fs::write(&there, "kept").unwrap();
    run_line(&mut harness, tab, &format!(":w {}", there.display()));
    // No disk is read on the way: the rename refuses the file, and the
    // line says so.
    assert_eq!(
        jobs(&harness)[0].destination,
        Destination::File {
            path: there.clone(),
            replace: false
        }
    );
    tell(
        &mut harness,
        ExportState::Failed(ExportFailure::Exists {
            path: there.clone(),
        }),
    );
    // The line the status line shows once the last toast has gone.
    let (line, _) = crate::ui::workspace::export_line(&harness.app).unwrap();
    assert!(
        line.ends_with("exists · :w! to replace, or another name"),
        "{line}"
    );
    run_line(&mut harness, tab, &format!(":w! {}", there.display()));
    let job = jobs(&harness)[1].clone();
    assert_eq!(job.format, Format::Markdown);
    assert_eq!(
        job.destination,
        Destination::File {
            path: there,
            replace: true
        }
    );
    run_line(&mut harness, tab, ":export cancel");
    assert!(
        harness
            .app
            .backend
            .sent
            .iter()
            .any(|command| matches!(command, Command::CancelExport(_)))
    );
    // `:w!` answered the failure, which goes (plan 4b, B16); and another
    // export may start beside the running one (spec 4, F7).
    assert_eq!(harness.app.exports.len(), 1);
    run_line(&mut harness, tab, ":export");
    assert!(sheet(&harness).is_some());
}

/// Test 8 and 12: `E` opens the float; Enter from the path line exports
/// there; a file that is there asks for Enter again.
#[test]
fn the_float_writes_where_its_path_line_says() {
    let (mut harness, _tab, _id) = covers_in(Look::omarchy());
    type_key(&mut harness, Key::E, "E");
    assert!(sheet(&harness).is_some(), "E opens the float");
    assert!(
        painted(&harness, "export book_covers"),
        "{:?}",
        harness.painted
    );
    let (_dir, path) = folder();
    std::fs::write(&path, "kept").unwrap();
    change(&mut harness, ExportChange::Path(path.display().to_string()));
    harness.app.apply(Action::StartExport);
    harness.settle();
    assert_eq!(
        jobs(&harness)[0].destination,
        Destination::File {
            path: path.clone(),
            replace: false
        }
    );
    // The rename finds the file: the float opens again, and says so.
    tell(
        &mut harness,
        ExportState::Failed(ExportFailure::Exists { path: path.clone() }),
    );
    harness.frame(Vec::new());
    harness.settle();
    assert!(sheet(&harness).unwrap().exists);
    assert!(
        painted(&harness, "! exists · enter again replaces it"),
        "{:?}",
        harness
            .painted
            .iter()
            .filter(|(text, _)| text.contains("exist")
                || text.contains("save")
                || text.contains("export")
                || text.contains("covers.csv"))
            .collect::<Vec<_>>()
    );
    harness.app.apply(Action::StartExport);
    harness.settle();
    assert_eq!(
        jobs(&harness)[1].destination,
        Destination::File {
            path,
            replace: true
        }
    );
    assert!(
        harness.app.backend.export_asks.is_empty(),
        "no panel in this look"
    );
}

/// Test 11: the terminal look's status line, while it runs and at its
/// end; Esc puts a failure away.
#[test]
fn the_status_line_follows_the_export() {
    let (mut harness, tab, _id) = covers_in(Look::omarchy());
    let (_dir, path) = folder();
    run_line(&mut harness, tab, &format!(":w {}", path.display()));
    harness.app.exports.last_mut().unwrap().total = Some(crate::export_sheet::Count {
        rows: 4,
        exact: true,
    });
    tell(&mut harness, ExportState::Progress { rows: 2, bytes: 10 });
    let line = |harness: &Harness, start: &str| {
        harness
            .painted
            .iter()
            .any(|(text, _)| text.starts_with(start))
    };
    assert!(
        line(
            &harness,
            "export book_covers.csv [██████░░░░░░] 50% · 2/4 · :export cancel"
        ),
        "{:?}",
        harness.painted
    );
    tell(
        &mut harness,
        ExportState::Failed(ExportFailure::Timeout {
            after: std::time::Duration::from_secs(60),
        }),
    );
    assert!(
        line(
            &harness,
            "✗ server stopped the query after 60 s (statement_timeout) · :export review sql"
        ),
        "{:?}",
        harness.painted
    );
    harness.press(Key::Escape, Modifiers::NONE);
    assert!(harness.app.exports.is_empty(), "Esc puts it away");
}

/// Test 13: the File menu's Export… acts on the table in front, and its
/// Import… is there, greyed.
#[test]
fn the_file_menu_has_export_and_a_greyed_import() {
    let (mut harness, _tab, _id) = covers_in(Look::macos());
    let export = crate::menu::Item::Command(crate::keymap::Command::Export);
    let import = crate::menu::Item::Import;
    assert!(harness.app.menu_state(export).enabled);
    assert_eq!(harness.app.menu_state(export).title, "Export…");
    assert!(!harness.app.menu_state(import).enabled);
    harness.app.choose(export);
    harness.settle();
    assert!(sheet(&harness).is_some());
}

/// Test 13: `E` on a table in the tree opens it, and its float, which
/// takes the columns once the table has them; on a schema it does nothing.
#[test]
fn e_in_the_tree_opens_the_table_and_its_float() {
    let (mut harness, tab, _id) = covers_in(Look::omarchy());
    let workspace = harness.app.workspace_mut(tab).unwrap();
    workspace.pane = crate::model::Pane::Tree;
    workspace.tree.cursor = Some(crate::model::TreeNode::Group("main".into(), "book".into()));
    type_key(&mut harness, Key::E, "E");
    assert!(sheet(&harness).is_none(), "a group has nothing to export");
    let users = tabletist_db::ObjectRef::new("main", "users");
    let workspace = harness.app.workspace_mut(tab).unwrap();
    workspace.tree.cursor = Some(crate::model::TreeNode::Object(
        users.clone(),
        tabletist_db::ObjectKind::Table,
    ));
    type_key(&mut harness, Key::E, "E");
    let open = sheet(&harness).expect("the float opens over the table");
    let front = harness
        .app
        .workspace(tab)
        .unwrap()
        .active_object_tab()
        .unwrap();
    assert_eq!(front.object, users);
    assert_eq!(open.id, front.id);
    assert!(open.columns.is_empty(), "nothing loaded yet");
    harness.answer_structure(crate::testing::book_covers_structure());
    harness.answer_rows(crate::testing::book_covers_page(3));
    harness.settle();
    assert_eq!(sheet(&harness).unwrap().columns.len(), 5);
}

/// Test 12: `:w` alone still saves the changes; with a path it never
/// touches the database.
#[test]
fn w_alone_still_saves_and_with_a_path_never_writes() {
    let (mut harness, tab, id) = covers_in(Look::omarchy());
    let cell = crate::model::CellPos { row: 0, col: 2 };
    let start = crate::model::EditStart::Replace("ebook".into());
    harness.app.apply(Action::EditCell {
        tab,
        id,
        cell,
        start,
    });
    let then = crate::model::Advance::Stay;
    harness.app.apply(Action::CommitEdit { tab, id, then });
    harness.settle();
    let (_dir, path) = folder();
    run_line(&mut harness, tab, &format!(":w {}", path.display()));
    let wrote = |harness: &Harness| {
        harness
            .app
            .backend
            .sent
            .iter()
            .any(|command| matches!(command, Command::Write { .. }))
    };
    assert!(!wrote(&harness), "a path never touches the database");
    assert_eq!(jobs(&harness).len(), 1);
    run_line(&mut harness, tab, ":w");
    assert!(
        wrote(&harness) || harness.app.dialog.is_some(),
        "bare :w writes the pending changes, or asks first"
    );
}

/// Test 6: Count exactly sends one count, of the tab's query.
#[test]
fn count_exactly_sends_one_count() {
    let (mut harness, _tab, _id) = covers_in(Look::macos());
    harness
        .app
        .workspace_mut(_tab)
        .unwrap()
        .object_tab_mut(_id)
        .unwrap()
        .estimated_rows = Some(31_400);
    export_chord(&mut harness);
    let counts = |harness: &Harness| {
        harness
            .app
            .backend
            .sent
            .iter()
            .filter(|command| matches!(command, Command::CountRows { .. }))
            .count()
    };
    let before = counts(&harness);
    harness.click("Count exactly");
    assert_eq!(counts(&harness), before + 1);
}

/// Q7: the clipboard is no choice over 10,000 rows: a choice of rows that
/// passes it takes the file back.
#[test]
fn rows_past_the_clipboards_limit_take_the_file_back() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    harness
        .app
        .workspace_mut(tab)
        .unwrap()
        .object_tab_mut(id)
        .unwrap()
        .estimated_rows = Some(31_400);
    export_chord(&mut harness);
    change(&mut harness, ExportChange::Rows(RowsChoice::Page));
    change(
        &mut harness,
        ExportChange::Destination(DestinationChoice::Clipboard),
    );
    assert_eq!(
        sheet(&harness).unwrap().destination,
        DestinationChoice::Clipboard
    );
    change(&mut harness, ExportChange::Rows(RowsChoice::Whole));
    assert_eq!(
        sheet(&harness).unwrap().destination,
        DestinationChoice::File
    );
}

/// Spec 4, F7: while one runs, ⇧⌘E and File › Export… still open the
/// sheet, for another export beside it (3a's D11 is gone).
#[test]
fn the_export_key_opens_the_sheet_while_one_runs() {
    let (mut harness, tab, _id) = covers_in(Look::macos());
    let (_dir, path) = folder();
    harness.app.apply(Action::ExportTo {
        tab,
        path: path.display().to_string(),
        replace: false,
    });
    harness.settle();
    let export = crate::menu::Item::Command(crate::keymap::Command::Export);
    let state = harness.app.menu_state(export);
    assert_eq!(state.title, "Export…");
    assert!(state.enabled);
    export_chord(&mut harness);
    assert!(sheet(&harness).is_some(), "the sheet opens");
    assert!(
        !harness
            .app
            .backend
            .sent
            .iter()
            .any(|command| matches!(command, Command::CancelExport(_))),
        "nothing is cancelled"
    );
}
