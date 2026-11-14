//! Exporting a SQL result (run 4a), checked against its spec
//! (`docs/superpowers/specs/2026-11-04-export-copy-as-4-result-design.md`:
//! "test N" below is an entry of its Tests): the result's sheet, Run
//! again without the limit and its rule, the job it sends, and the ways
//! in.
//!
//! Behaviour and words, on the Bookshop's `book_covers` read in a SQL
//! editor. The backend is a recording one. No picture of the design is
//! compared.

use egui::{Key, Modifiers};
use tabletist_db::{ScriptEnd, StatementOutcome};

use super::insert_audit_tests::{painted, select, type_key};
use crate::backend::export::{Destination, ExportJob, ExportRows, ExportState};
use crate::backend::{Command, Event, HistoryCommand};
use crate::export::Format;
use crate::export_sheet::{DestinationChoice, ExportChange, ExportSheet, RowsChoice};
use crate::model::{Action, ConnTabId, Dialog, TabId};
use crate::testing::Harness;
use crate::theme::Look;

const TEXT: &str = "SELECT * FROM book_covers";

/// `book_covers`' rows as a statement returns them, `cut` where the
/// editor's limit stopped them.
fn covers(rows: usize, cut: bool) -> StatementOutcome {
    let page = crate::testing::book_covers_page(rows);
    StatementOutcome::Rows {
        columns: page.columns,
        rows: page.rows,
        truncated: cut,
    }
}

/// A writable Bookshop in `look`, a SQL editor holding `text`, run, and
/// its answer `outcome` (a run that only reads, unless `write`).
fn editor_in(
    look: Look,
    text: &str,
    outcome: StatementOutcome,
    write: bool,
) -> (Harness, ConnTabId, TabId) {
    let mut harness = Harness::with_size(egui::vec2(1400.0, 1800.0));
    harness.set_look(look);
    harness.app.settings.group_digits = Some(false);
    harness.app.settings.new_tab = crate::settings::NewTab::Connection;
    let tab = harness.connect_fake_as(false);
    harness.app.workspace_mut(tab).unwrap().driver = tabletist_db::Driver::Sqlite;
    harness.app.apply(Action::NewSqlTab(tab));
    let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
    {
        let sql = sql_mut(&mut harness, tab, id);
        sql.text = text.into();
        sql.cursor = 0;
    }
    harness.app.apply(Action::RunSql {
        tab,
        sql_tab: id,
        all: false,
    });
    let answer = match write {
        true => crate::testing::write_outcome(vec![outcome], ScriptEnd::Committed),
        false => crate::testing::script_outcome(vec![outcome]),
    };
    harness.answer_sql(Ok(answer), None);
    harness.settle();
    (harness, tab, id)
}

fn sql_mut(harness: &mut Harness, tab: ConnTabId, id: TabId) -> &mut crate::model::SqlTab {
    harness
        .app
        .workspace_mut(tab)
        .unwrap()
        .sql_tab_mut(id)
        .unwrap()
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

fn open(harness: &mut Harness, tab: ConnTabId) {
    harness.app.apply(Action::OpenExport { tab, format: None });
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

/// The save panel's answer, `path`, to the last ask.
fn answer_panel(harness: &mut Harness, path: std::path::PathBuf) {
    let job = harness.app.backend.export_asks.last().unwrap().0;
    harness.app.apply(Action::Backend(Event::ExportPath {
        job,
        path: Some(path),
        replace: false,
    }));
    harness.settle();
}

/// A click on the first cell of the result's row `row` (from 1): its grid
/// takes the keys.
fn click_cell(harness: &mut Harness, row: usize) {
    let tree = harness.settle();
    let label = format!("Row {row}");
    let at = crate::testing::bounds(&tree, &label, egui::accesskit::Role::Button).expect("the row");
    let pos = at.left_center() + egui::vec2(crate::ui::grid::gutter(&harness.app.look) + 10.0, 0.0);
    let button = |pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    harness.frame(vec![egui::Event::PointerMoved(pos)]);
    harness.frame(vec![button(true)]);
    harness.frame(vec![button(false)]);
    harness.settle();
}

fn run_line(harness: &mut Harness, tab: ConnTabId, line: &str) {
    let text = line.strip_prefix(':').unwrap_or(line);
    harness.app.workspace_mut(tab).unwrap().command = Some(text.into());
    harness.app.apply(Action::RunCommand(tab));
    harness.settle();
}

/// Test 3: ⇧⌘E over a result opens its sheet: "Export result", the
/// editor and statement in the subtitle, Rows already loaded first with
/// its count and "limit reached", and Run again offered after a cut read.
#[test]
fn the_results_sheet_names_it_and_starts_on_its_loaded_rows() {
    let (mut harness, _tab, _id) = editor_in(Look::macos(), TEXT, covers(3, true), false);
    harness.press(Key::E, Modifiers::COMMAND | Modifiers::SHIFT);
    harness.settle();
    let opened = sheet(&harness).expect("the export chord opens the result's sheet");
    assert!(opened.result.is_some());
    assert_eq!(opened.rows, RowsChoice::Loaded);
    for words in [
        "Export result",
        "Fixture · result of Query 1, statement 1 · read-only transaction",
        "Rows already loaded",
        "the result as shown · limit reached",
        "Run again without the limit",
        "reads only · same statement, no LIMIT, no timeout from Settings › Editor",
        "counted as it runs",
        "Export 3 rows…",
    ] {
        assert!(painted(&harness, words), "{words}: {:?}", harness.painted);
    }
    // Run again: the statement, as Review SQL shows it, and its button.
    change(&mut harness, ExportChange::Rows(RowsChoice::RunAgain));
    assert!(
        painted(&harness, "Run and export…"),
        "{:?}",
        harness.painted
    );
    let facts = harness.app.export_facts().unwrap();
    assert_eq!(facts.statement.as_deref(), Some(TEXT));
    // SQL INSERT writes into `result`, with no table to read a definition
    // of (S4): the box is greyed, and says why.
    change(&mut harness, ExportChange::Format(Format::SqlInsert));
    assert!(painted(&harness, "Not for a result: no table to read"));
    let tree = harness.settle();
    let create = "Start with CREATE TABLE from the DDL view";
    let id = crate::testing::node(&tree, create, egui::accesskit::Role::CheckBox).expect("the box");
    let off = tree
        .nodes
        .iter()
        .find(|(node, _)| *node == id)
        .is_some_and(|(_, node)| node.is_disabled());
    assert!(off, "greyed for a result");
    // Over a range, the sheet starts on its rows (S1).
    harness.app.apply(Action::CloseDialog);
    let tab = harness.app.active_tab_id();
    let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
    harness.app.apply(Action::SelectCell {
        tab,
        id,
        cell: crate::model::CellPos { row: 0, col: 0 },
    });
    harness.app.apply(Action::ExtendTo {
        tab,
        id,
        cell: crate::model::CellPos { row: 1, col: 4 },
    });
    open(&mut harness, tab);
    assert_eq!(sheet(&harness).unwrap().rows, RowsChoice::Selected);
    assert_eq!(harness.app.export_facts().unwrap().selected, 2);
}

/// The float: its head and its rows in the terminal's words.
#[test]
fn the_results_float_says_it_the_terminals_way() {
    let (mut harness, tab, _id) = editor_in(Look::omarchy(), TEXT, covers(3, true), false);
    run_line(&mut harness, tab, ":export");
    assert!(sheet(&harness).is_some_and(|open| open.result.is_some()));
    for words in [
        "export result",
        "fixture · dev · query 1, statement 1",
        "rows already loaded",
        "limit reached",
        "run again without the limit",
        "reads only",
    ] {
        assert!(painted(&harness, words), "{words}: {:?}", harness.painted);
    }
}

/// F3 in the float: after a run sent to write, the board's note.
#[test]
fn the_float_says_why_after_a_write() {
    let text = "UPDATE book_covers SET kind = 'print' WHERE id < 4 RETURNING *";
    let (mut harness, tab, _id) = editor_in(Look::omarchy(), text, covers(3, true), true);
    run_line(&mut harness, tab, ":export");
    for words in [
        "not after a write",
        "after a read-write run only \"rows already loaded\" is offered",
    ] {
        assert!(painted(&harness, words), "{words}: {:?}", harness.painted);
    }
}

/// Test 2 and F3: Run again is greyed, with why, after a run sent to
/// write, for a result the limit did not cut, and once the statement was
/// edited; then ↩ sends nothing for it.
#[test]
fn run_again_is_greyed_with_its_reason() {
    let returning = "UPDATE book_covers SET kind = 'print' WHERE id < 4 RETURNING *";
    let cases = [
        (
            returning,
            true,
            true,
            false,
            "Not offered: this run changed data",
        ),
        (TEXT, false, false, false, "Not needed: every row is loaded"),
        (
            TEXT,
            false,
            true,
            true,
            "Not offered: the statement was edited since it ran",
        ),
    ];
    for (text, write, cut, edited, why) in cases {
        let (mut harness, tab, id) = editor_in(Look::macos(), text, covers(3, cut), write);
        if edited {
            sql_mut(&mut harness, tab, id).text.push(' ');
        }
        open(&mut harness, tab);
        assert!(painted(&harness, why), "{why}: {:?}", harness.painted);
        let again = harness.app.export_facts().unwrap().result.unwrap().again;
        assert!(again.is_err(), "{why}");
        // Asked all the same, it runs nothing.
        change(&mut harness, ExportChange::Rows(RowsChoice::RunAgain));
        harness.app.apply(Action::StartExport);
        let (_dir, path) = folder();
        if !harness.app.backend.export_asks.is_empty() {
            answer_panel(&mut harness, path);
        }
        assert!(jobs(&harness).is_empty(), "{why}: nothing is sent");
    }
}

/// A folder of the test's own, and a file in it.
fn folder() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("query-1.csv");
    (dir, path)
}

/// Test 2: Run again sends the statement as it was written, on the
/// export's own connection, with the columns chosen; SQL INSERT has no
/// table; the history records it under the editor's name.
#[test]
fn run_again_sends_the_statement_and_records_it() {
    let (mut harness, tab, _id) = editor_in(Look::macos(), TEXT, covers(3, true), false);
    open(&mut harness, tab);
    change(&mut harness, ExportChange::Rows(RowsChoice::RunAgain));
    change(&mut harness, ExportChange::Choose(true));
    change(&mut harness, ExportChange::Toggle(0));
    change(&mut harness, ExportChange::Move { from: 2, by: -1 });
    harness.press(Key::Enter, Modifiers::NONE);
    let asks = harness.app.backend.export_asks.clone();
    assert!(asks[0].1.ends_with("query-1.csv"), "{asks:?}");
    let (_dir, path) = folder();
    answer_panel(&mut harness, path.clone());
    let sent = jobs(&harness);
    assert_eq!(sent.len(), 1);
    let ExportRows::Statement { statement, pick } = &sent[0].rows else {
        panic!("{:?}", sent[0].rows);
    };
    assert_eq!(statement.text, TEXT);
    // `id` left out; `kind` moved before `publisher_id`.
    assert_eq!(pick.as_deref(), Some(&[2, 1, 3, 4][..]));
    assert_eq!(
        sent[0].columns,
        ["kind", "publisher_id", "image_data", "created_at"]
    );
    assert_eq!(sent[0].object, None);
    assert!(sent[0].masked.iter().all(|masked| !masked));
    assert_eq!(
        sent[0].destination,
        Destination::File {
            path,
            replace: false
        }
    );
    let history = harness.app.backend.sent.len();
    let job = harness.app.exports.last().unwrap().job.id;
    for state in [
        ExportState::Started {
            statement: Some(TEXT.into()),
        },
        ExportState::Done {
            rows: 4_200,
            bytes: 90_000,
            took: std::time::Duration::from_secs(2),
            copied: None,
            statements: 0,
        },
    ] {
        harness
            .app
            .apply(Action::Backend(Event::Export { job, state }));
    }
    let entry = harness.app.backend.sent[history..]
        .iter()
        .find_map(|command| match command {
            Command::History(HistoryCommand::Append { entry, .. }) => Some(entry.clone()),
            _ => None,
        })
        .expect("S7: Run again is recorded");
    assert_eq!(entry.source, crate::history::Source::Export);
    assert_eq!(entry.sql.as_deref(), Some(TEXT));
    assert_eq!(entry.rows, Some(4_200));
    assert_eq!(entry.detail, "Query 1");
}

/// Test 3: the loaded rows go out as loaded (nothing runs, nothing is
/// recorded), in the columns chosen, and fit the clipboard.
#[test]
fn loaded_rows_go_to_the_clipboard_as_loaded() {
    let (mut harness, tab, _id) = editor_in(Look::macos(), TEXT, covers(3, true), false);
    open(&mut harness, tab);
    change(&mut harness, ExportChange::Format(Format::Tsv));
    change(
        &mut harness,
        ExportChange::Destination(DestinationChoice::Clipboard),
    );
    change(&mut harness, ExportChange::Choose(true));
    change(&mut harness, ExportChange::Toggle(3));
    assert!(painted(&harness, "Copy 3 rows"), "{:?}", harness.painted);
    harness.press(Key::Enter, Modifiers::NONE);
    let job = jobs(&harness)[0].clone();
    assert_eq!(job.destination, Destination::Clipboard);
    let ExportRows::Loaded { columns, rows } = &job.rows else {
        panic!("{:?}", job.rows);
    };
    let names: Vec<&str> = columns.iter().map(|column| column.name.as_str()).collect();
    assert_eq!(names, ["id", "publisher_id", "kind", "created_at"]);
    assert_eq!((rows.len(), rows[0].len()), (3, 4));
    assert!(harness.app.exports.last().unwrap().history.is_none());
}

/// Test 4, S9: the results bar's Copy as and Export…, greyed with no cell
/// picked and with no rows; on Messages too.
#[test]
fn the_results_bar_holds_copy_as_and_export() {
    let (mut harness, tab, id) = editor_in(Look::macos(), TEXT, covers(3, true), false);
    let tree = harness.settle();
    let copy = crate::testing::node(&tree, "Copy as", egui::accesskit::Role::Button)
        .expect("Copy as on the bar");
    let disabled = |tree: &egui::accesskit::TreeUpdate, id| {
        tree.nodes
            .iter()
            .find(|(node, _)| *node == id)
            .is_some_and(|(_, node)| node.is_disabled())
    };
    assert!(disabled(&tree, copy), "no cell picked");
    harness.click("Export…");
    assert!(sheet(&harness).is_some_and(|open| open.result.is_some()));
    harness.app.apply(Action::CloseDialog);
    harness.app.apply(Action::SelectCell {
        tab,
        id,
        cell: crate::model::CellPos { row: 0, col: 0 },
    });
    harness.click("Copy as");
    assert!(
        matches!(harness.app.dialog, Some(Dialog::CopyAs(_))),
        "{:?}",
        harness.app.dialog
    );
    harness.app.apply(Action::CloseDialog);
    // The Messages pane has them too.
    harness.app.apply(Action::SetResultPane {
        tab,
        sql_tab: id,
        pane: crate::model::ResultPane::Messages,
    });
    harness.click("Export…");
    assert!(sheet(&harness).is_some());
    // A statement that returned no rows: Export… greyed, and the chord
    // and `:export` open nothing.
    let (mut harness, tab, _id) = editor_in(Look::macos(), TEXT, covers(0, false), false);
    let tree = harness.settle();
    let export = crate::testing::node(&tree, "Export…", egui::accesskit::Role::Button)
        .expect("Export… on the bar");
    assert!(disabled(&tree, export));
    harness.press(Key::E, Modifiers::COMMAND | Modifiers::SHIFT);
    harness.settle();
    assert!(sheet(&harness).is_none());
    run_line(&mut harness, tab, ":export");
    assert!(sheet(&harness).is_none());
}

/// S9: while a run is in flight its answer will replace the rows: no way
/// in opens the sheet on the rows about to go.
#[test]
fn nothing_exports_while_a_run_is_in_flight() {
    let (mut harness, tab, id) = editor_in(Look::macos(), TEXT, covers(3, true), false);
    harness.app.apply(Action::RunSql {
        tab,
        sql_tab: id,
        all: false,
    });
    let tree = harness.settle();
    let export = crate::testing::node(&tree, "Export…", egui::accesskit::Role::Button)
        .expect("Export… on the bar");
    let off = tree
        .nodes
        .iter()
        .find(|(node, _)| *node == export)
        .is_some_and(|(_, node)| node.is_disabled());
    assert!(off, "greyed while the run is on its way");
    harness.press(Key::E, Modifiers::COMMAND | Modifiers::SHIFT);
    harness.settle();
    assert!(sheet(&harness).is_none());
    open(&mut harness, tab);
    assert!(sheet(&harness).is_none(), "the reducer refuses it too");
    // A sheet open over a result goes once another answer replaces it,
    // though at the same statement's place.
    harness.answer_sql(
        Ok(crate::testing::script_outcome(vec![covers(3, true)])),
        None,
    );
    open(&mut harness, tab);
    assert!(sheet(&harness).is_some());
    // As if a run sent before the sheet opened had answered: the rows are
    // another request's.
    sql_mut(&mut harness, tab, id).run.loaded = Some(crate::backend::RequestId(u64::MAX));
    harness.settle();
    assert!(sheet(&harness).is_none(), "the sheet went with its rows");
}

/// F3: a dropped session takes Run again with it, as it takes Show all.
#[test]
fn run_again_needs_the_session_show_all_needs() {
    let (mut harness, tab, _id) = editor_in(Look::macos(), TEXT, covers(3, true), false);
    harness.app.workspace_mut(tab).unwrap().status = crate::model::SessionStatus::Disconnected(
        tabletist_db::Error::ConnectionLost("gone".into()),
    );
    open(&mut harness, tab);
    assert!(
        painted(&harness, "Not offered: the connection is not ready"),
        "{:?}",
        harness.painted
    );
}

/// S9: the bar's words are never cut while its buttons keep theirs: the
/// buttons fold first, then go.
#[test]
fn the_bars_words_stay_whole_while_its_buttons_fold() {
    for width in [720.0, 800.0, 900.0, 1000.0, 1400.0] {
        let mut harness = Harness::with_size(egui::vec2(width, 900.0));
        harness.set_look(Look::macos());
        harness.app.settings.new_tab = crate::settings::NewTab::Connection;
        let tab = harness.connect_fake_as(false);
        harness.app.apply(Action::NewSqlTab(tab));
        let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
        sql_mut(&mut harness, tab, id).text = TEXT.into();
        harness.app.apply(Action::RunSql {
            tab,
            sql_tab: id,
            all: false,
        });
        let answer = crate::testing::script_outcome(vec![covers(3, true)]);
        harness.answer_sql(Ok(answer), None);
        harness.settle();
        let whole = painted(&harness, "Statement at line 1 · 14 ms");
        let buttons = painted(&harness, "Export…") || painted(&harness, "Copy as");
        assert!(whole || !buttons, "{width}: {:?}", harness.painted);
    }
}

/// Test 4: the terminal look's `E` on the result's rows, its key line,
/// and `:w <path>` (S6).
#[test]
fn the_terminal_looks_ways_export_the_result() {
    let (mut harness, tab, _id) = editor_in(Look::omarchy(), TEXT, covers(3, true), false);
    click_cell(&mut harness, 1);
    let hint = |(text, _): &(String, egui::Color32)| text.contains("E export result");
    assert!(harness.painted.iter().any(hint), "{:?}", harness.painted);
    type_key(&mut harness, Key::E, "E");
    assert!(sheet(&harness).is_some_and(|open| open.result.is_some()));
    harness.app.apply(Action::CloseDialog);
    let (_dir, path) = folder();
    let line = format!(":w {}", path.with_extension("jsonl").display());
    run_line(&mut harness, tab, &line);
    let sent = jobs(&harness);
    assert_eq!(
        sent.len(),
        1,
        "{:?}",
        harness.app.workspace(tab).unwrap().command_error
    );
    assert_eq!(sent[0].format, Format::JsonLines);
    assert!(matches!(&sent[0].rows, ExportRows::Loaded { rows, .. } if rows.len() == 3));
}

/// S11: quick open lists the result's export while its editor is in
/// front, and it opens the sheet.
#[test]
fn quick_open_exports_the_result_in_front() {
    let (mut harness, _tab, _id) = editor_in(Look::macos(), TEXT, covers(3, true), false);
    harness.press(Key::P, Modifiers::COMMAND);
    harness.frame(vec![egui::Event::Text("> export res".into())]);
    harness.settle();
    assert!(
        painted(&harness, "Export result of Query 1…"),
        "{:?}",
        harness.painted
    );
    assert!(painted(&harness, "SQL editor"));
    harness.press(Key::Enter, Modifiers::NONE);
    harness.settle();
    assert!(sheet(&harness).is_some_and(|open| open.result.is_some()));
}

/// Test 4, S8: the row panel's Copy row ▾ ends with Export row…, which
/// exports the panel's row alone, whatever range stands; on a table's
/// row and on a result's.
#[test]
fn export_row_exports_the_panels_row_alone() {
    // A table's, under a range of three rows: the cursor's row alone.
    let mut harness = Harness::with_size(egui::vec2(1400.0, 1800.0));
    harness.set_look(Look::macos());
    harness.app.settings.group_digits = Some(false);
    let (tab, id) = harness.book_covers();
    harness.app.workspace_mut(tab).unwrap().pane = crate::model::Pane::Grid;
    select(&mut harness, tab, id, (0, 1));
    harness.press(Key::ArrowDown, Modifiers::SHIFT);
    harness.press(Key::ArrowDown, Modifiers::SHIFT);
    harness.click("Copy row ▾");
    assert!(painted(&harness, "Export row…"), "{:?}", harness.painted);
    harness.click("Export row…");
    let open = sheet(&harness).expect("the sheet");
    assert!(open.row_only && open.result.is_none());
    assert_eq!(open.rows, RowsChoice::Selected);
    assert!(!open.choose, "every column of the row");
    assert_eq!(harness.app.export_facts().unwrap().selected, 1);
    harness.app.apply(Action::StartExport);
    // The panel asks where; its rows are the cursor's row alone.
    assert_eq!(harness.app.backend.export_asks.len(), 1);
    harness.app.apply(Action::CloseDialog);
    // A result's, as its panel's Copy row ▾ asks for it (its fields hold
    // no keyboard: none can be edited; plan, B15).
    let (mut harness, tab, _id) = editor_in(Look::omarchy(), TEXT, covers(3, true), false);
    click_cell(&mut harness, 3);
    harness
        .app
        .apply(Action::ExportSelection(crate::ui::keys::export_row(tab)));
    let open = sheet(&harness).expect("the result's sheet");
    assert!(open.row_only && open.result.is_some());
    assert_eq!(open.rows, RowsChoice::Selected);
    harness.app.apply(Action::StartExport);
    let ExportRows::Loaded { rows, .. } = &jobs(&harness)[0].rows else {
        panic!("loaded rows");
    };
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0][0], tabletist_db::Value::Int(3), "the cursor's row");
}
