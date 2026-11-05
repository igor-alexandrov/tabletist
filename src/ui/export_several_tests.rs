//! Several exports at once, checked against run 4's spec
//! (`docs/superpowers/specs/2026-11-04-export-copy-as-4-result-design.md`,
//! tests 6 and 7; F5, F7, F8, S12 to S14).
//!
//! Behaviour and words, on the Bookshop's `book_covers`. The backend is a
//! recording one: what an export would send is checked, and what it would
//! answer is given. No picture of the design is compared.

use egui::accesskit::{self, Role};
use egui::{Key, Modifiers};

use super::insert_audit_tests::{painted, type_key};
use crate::backend::export::{ExportFailure, ExportId, ExportState};
use crate::backend::{Command, Event};
use crate::model::{Action, ConnTabId, Dialog, TabId};
use crate::testing::Harness;
use crate::theme::Look;

/// The Bookshop's covers in `look`, the grid holding the keyboard.
fn covers_in(look: Look) -> (Harness, ConnTabId, TabId) {
    let mut harness = Harness::with_size(egui::vec2(1400.0, 1000.0));
    harness.set_look(look);
    harness.app.settings.group_digits = Some(false);
    let (tab, id) = harness.book_covers();
    harness.app.workspace_mut(tab).unwrap().pane = crate::model::Pane::Grid;
    harness.settle();
    (harness, tab, id)
}

/// Two exports started, to `book_covers.csv` and `events.jsonl` in a
/// folder of the test's own, the first's total known: their ids, in the
/// order they started.
fn two_running(harness: &mut Harness, tab: ConnTabId) -> (tempfile::TempDir, ExportId, ExportId) {
    let dir = tempfile::tempdir().unwrap();
    for name in ["book_covers.csv", "events.jsonl"] {
        harness.app.apply(Action::ExportTo {
            tab,
            path: dir.path().join(name).display().to_string(),
            replace: false,
        });
    }
    harness.settle();
    assert_eq!(harness.app.exports.len(), 2, "both run (spec 4, S12)");
    let (one, two) = (harness.app.exports[0].job.id, harness.app.exports[1].job.id);
    harness.app.exports[0].total = Some(crate::export_sheet::Count {
        rows: 4,
        exact: true,
    });
    harness.app.exports[1].total = None;
    tell(harness, one, ExportState::Progress { rows: 2, bytes: 10 });
    tell(
        harness,
        two,
        ExportState::Progress {
            rows: 4000,
            bytes: 10,
        },
    );
    (dir, one, two)
}

/// What the backend says of `job`.
fn tell(harness: &mut Harness, job: ExportId, state: ExportState) {
    harness
        .app
        .apply(Action::Backend(Event::Export { job, state }));
    harness.settle();
}

fn done() -> ExportState {
    ExportState::Done {
        rows: 4,
        bytes: 900,
        took: std::time::Duration::from_secs(2),
        copied: None,
        statements: 0,
    }
}

/// The cancels sent, in order.
fn cancels(harness: &Harness) -> Vec<ExportId> {
    harness
        .app
        .backend
        .sent
        .iter()
        .filter_map(|command| match command {
            Command::CancelExport(id) => Some(*id),
            _ => None,
        })
        .collect()
}

/// The buttons labelled `label`, top to bottom.
fn buttons(harness: &mut Harness, label: &str) -> Vec<accesskit::NodeId> {
    let tree = harness.settle();
    let mut found: Vec<(f64, accesskit::NodeId)> = tree
        .nodes
        .iter()
        .filter(|(_, node)| node.label() == Some(label) && node.role() == Role::Button)
        .map(|(id, node)| (node.bounds().map_or(0.0, |bounds| bounds.y0), *id))
        .collect();
    found.sort_by(|one, two| one.0.total_cmp(&two.0));
    found.into_iter().map(|(_, id)| id).collect()
}

/// A click on the `nth` button labelled `label`, from the top.
fn click_nth(harness: &mut Harness, label: &str, nth: usize) {
    let target = buttons(harness, label)[nth];
    harness.frame(vec![egui::Event::AccessKitActionRequest(
        accesskit::ActionRequest {
            target_tree: accesskit::TreeId::ROOT,
            target_node: target,
            action: accesskit::Action::Click,
            data: None,
        },
    )]);
    harness.settle();
}

/// Where the text `text` was painted, from the top.
fn top_of(harness: &Harness, text: &str) -> f32 {
    harness
        .text_rects
        .iter()
        .find(|(piece, _)| piece == text)
        .map(|(_, rect)| rect.top())
        .unwrap_or_else(|| panic!("{text:?} not painted: {:?}", harness.painted))
}

/// The label of what has the keyboard.
fn focused_label(harness: &mut Harness) -> Option<String> {
    let id = harness.ctx.memory(|memory| memory.focused())?;
    let tree = harness.settle();
    tree.nodes
        .iter()
        .find(|(node, _)| *node == id.accesskit_id())
        .and_then(|(_, node)| node.label().map(str::to_owned))
}

/// A line of the terminal look's `:` prompt, run.
fn run_line(harness: &mut Harness, tab: ConnTabId, line: &str) {
    let text = line.strip_prefix(':').unwrap_or(line);
    harness.app.workspace_mut(tab).unwrap().command = Some(text.into());
    harness.app.apply(Action::RunCommand(tab));
    harness.settle();
}

fn line(harness: &Harness) -> Option<String> {
    crate::ui::workspace::export_line(&harness.app).map(|(text, _)| text)
}

/// Test 6, S12 and S13: two run, stacked with the newest at the bottom;
/// each cancels alone and ends with its own toast; a done goes after its
/// 4 s while a failure stays.
#[test]
fn two_exports_run_and_cancel_alone() {
    let (mut harness, tab, _id) = covers_in(Look::macos());
    let (_dir, one, two) = two_running(&mut harness, tab);
    assert!(painted(&harness, "Exporting book_covers.csv"));
    assert!(painted(&harness, "Exporting events.jsonl"));
    assert!(
        top_of(&harness, "Exporting book_covers.csv") < top_of(&harness, "Exporting events.jsonl"),
        "the newest at the bottom"
    );
    // The board's second toast: no total, so its connection under the bar
    // (plan 4b, B4).
    let connection = harness.app.exports[1].connection.clone();
    assert!(!connection.is_empty());
    assert!(painted(&harness, &connection), "{:?}", harness.painted);
    assert!(painted(&harness, "4,000 rows"), "{:?}", harness.painted);
    // The top toast's Cancel cancels the first alone.
    click_nth(&mut harness, "Cancel", 0);
    assert_eq!(cancels(&harness), [one]);
    tell(&mut harness, one, ExportState::Cancelled);
    assert!(painted(&harness, crate::ui::export_toast::CANCELLED));
    assert!(
        painted(&harness, "Exporting events.jsonl"),
        "the other runs"
    );
    tell(
        &mut harness,
        two,
        ExportState::Failed(ExportFailure::ConnectionLost { rows: 4000 }),
    );
    assert_eq!(harness.app.exports.len(), 2, "both ends are told");
    // The cancel goes after its 4 s; the failure stays.
    for exporting in &mut harness.app.exports {
        if let crate::export_sheet::ExportStatus::Cancelled { at } = &mut exporting.status {
            *at -= crate::export_sheet::TOLD_FOR;
        }
    }
    harness.settle();
    assert_eq!(harness.app.exports.len(), 1);
    assert_eq!(harness.app.exports[0].job.id, two);
    click_nth(&mut harness, "Dismiss", 0);
    assert!(harness.app.exports.is_empty());
}

/// F8: a hidden export is a pill of its own, stacked up from the bottom
/// right, newest at the bottom; a click on one shows its toast again.
#[test]
fn a_hidden_export_is_a_pill_each() {
    let (mut harness, tab, _id) = covers_in(Look::macos());
    let (_dir, one, two) = two_running(&mut harness, tab);
    click_nth(&mut harness, "Hide", 1);
    click_nth(&mut harness, "Hide", 0);
    assert!(harness.app.exports.iter().all(|exporting| exporting.hidden));
    assert!(
        painted(&harness, "book_covers.csv 50%"),
        "{:?}",
        harness.painted
    );
    assert!(painted(&harness, "events.jsonl"));
    assert!(top_of(&harness, "book_covers.csv 50%") < top_of(&harness, "events.jsonl"));
    assert!(!painted(&harness, "Exporting book_covers.csv"), "no toast");
    // The pill, as a screen reader hears it.
    let pill = buttons(&mut harness, "Exporting events.jsonl")[0];
    harness.frame(vec![egui::Event::AccessKitActionRequest(
        accesskit::ActionRequest {
            target_tree: accesskit::TreeId::ROOT,
            target_node: pill,
            action: accesskit::Action::Click,
            data: None,
        },
    )]);
    harness.settle();
    let hidden: Vec<(ExportId, bool)> = harness
        .app
        .exports
        .iter()
        .map(|exporting| (exporting.job.id, exporting.hidden))
        .collect();
    assert_eq!(hidden, [(one, true), (two, false)]);
    assert!(painted(&harness, "Exporting events.jsonl"));
}

/// Test 6 and S13: ⌘⇧' moves the keyboard to the newest toast's Cancel,
/// and ⇥ walks every toast's buttons, top to bottom.
#[test]
fn the_keyboard_walks_every_toast() {
    let (mut harness, tab, _id) = covers_in(Look::macos());
    let (_dir, _one, two) = two_running(&mut harness, tab);
    harness.press(Key::Quote, Modifiers::COMMAND | Modifiers::SHIFT);
    let newest = buttons(&mut harness, "Cancel")[1];
    let focused = harness.ctx.memory(|memory| memory.focused()).unwrap();
    assert_eq!(focused.accesskit_id(), newest, "the newest toast's Cancel");
    let mut seen = Vec::new();
    let mut visited = std::collections::HashSet::new();
    for _ in 0..4 {
        harness.press(Key::Tab, Modifiers::NONE);
        seen.push(focused_label(&mut harness).unwrap_or_default());
        visited.extend(harness.ctx.memory(|memory| memory.focused()));
    }
    assert_eq!(seen, ["Hide", "Cancel", "Hide", "Cancel"]);
    assert_eq!(visited.len(), 4, "each of the two toasts' buttons, once");
    // Enter on it cancels the newest alone.
    harness.press(Key::Enter, Modifiers::NONE);
    assert_eq!(cancels(&harness), [two]);
}

/// F7: while one runs, ⇧⌘E and File › Export… open the sheet, and the
/// export started from it runs beside the first.
#[test]
fn export_always_opens_the_sheet() {
    let (mut harness, tab, _id) = covers_in(Look::macos());
    let (_dir, _one, _two) = two_running(&mut harness, tab);
    harness.press(Key::E, Modifiers::COMMAND | Modifiers::SHIFT);
    assert!(matches!(harness.app.dialog, Some(Dialog::Export(_))));
    harness.app.dialog = None;
    let item = crate::menu::Item::Command(crate::keymap::Command::Export);
    assert!(harness.app.menu_state(item).enabled);
    harness.app.choose(item);
    harness.settle();
    assert!(matches!(harness.app.dialog, Some(Dialog::Export(_))));
    harness.app.apply(Action::ChangeExport(
        crate::export_sheet::ExportChange::Destination(
            crate::export_sheet::DestinationChoice::Clipboard,
        ),
    ));
    harness.app.apply(Action::StartExport);
    harness.settle();
    assert_eq!(harness.app.exports.len(), 3, "a third beside them");
    assert!(cancels(&harness).is_empty());
}

/// F7: quick open lists a row per export running, which cancels it.
#[test]
fn quick_open_cancels_each_export() {
    let (mut harness, tab, _id) = covers_in(Look::macos());
    let (_dir, _one, two) = two_running(&mut harness, tab);
    harness.press(Key::P, Modifiers::COMMAND);
    assert!(matches!(harness.app.dialog, Some(Dialog::QuickOpen(_))));
    harness.frame(vec![egui::Event::Text(">cancel exporting".into())]);
    harness.settle();
    assert!(
        painted(&harness, "Cancel exporting book_covers.csv"),
        "{:?}",
        harness.painted
    );
    assert!(painted(&harness, "Cancel exporting events.jsonl"));
    harness.click("Cancel exporting events.jsonl, command, export running");
    assert_eq!(cancels(&harness), [two]);
}

/// Review: a cancel row lit from the keyboard keeps its place as its
/// export's words change, and Enter cancels it.
#[test]
fn quick_open_keeps_a_cancel_row_lit_as_it_runs() {
    let (mut harness, tab, _id) = covers_in(Look::macos());
    let (_dir, one, two) = two_running(&mut harness, tab);
    harness.press(Key::P, Modifiers::COMMAND);
    harness.frame(vec![egui::Event::Text(">cancel exporting".into())]);
    harness.settle();
    let first = top_of(&harness, "Cancel exporting book_covers.csv")
        < top_of(&harness, "Cancel exporting events.jsonl");
    harness.press(Key::ArrowDown, Modifiers::NONE);
    // The lit row's export goes on: its words change.
    let lit = if first { two } else { one };
    tell(
        &mut harness,
        lit,
        ExportState::Progress { rows: 3, bytes: 20 },
    );
    harness.press(Key::Enter, Modifiers::NONE);
    assert_eq!(cancels(&harness), [lit]);
}

/// Test 7 and S14: the terminal look's line for one, for two in the order
/// they started, and an end among them, the last to end first.
#[test]
fn the_terminal_line_says_one_two_and_an_end() {
    let (mut harness, tab, _id) = covers_in(Look::omarchy());
    let (_dir, one, two) = two_running(&mut harness, tab);
    assert_eq!(
        line(&harness).as_deref(),
        Some("2 exports · book_covers.csv 50% · events.jsonl 4,000 rows · :export cancel")
    );
    tell(&mut harness, one, done());
    let shown = line(&harness).unwrap();
    assert!(shown.starts_with("✓ exported 4 rows → "), "{shown}");
    tell(
        &mut harness,
        two,
        ExportState::Failed(ExportFailure::ConnectionLost { rows: 4000 }),
    );
    let shown = line(&harness).unwrap();
    assert!(
        shown.starts_with("✗ connection lost after 4,000"),
        "the last to end: {shown}"
    );
    // The done goes after its 4 s; the failure stays until Esc.
    harness.app.exports[0].status = crate::export_sheet::ExportStatus::Cancelled {
        at: std::time::Instant::now() - crate::export_sheet::TOLD_FOR,
    };
    harness.settle();
    assert_eq!(harness.app.exports.len(), 1);
    harness.press(Key::Escape, Modifiers::NONE);
    assert_eq!(line(&harness), None);
    // One running: 3a's line.
    let dir = tempfile::tempdir().unwrap();
    run_line(
        &mut harness,
        tab,
        &format!(":w {}", dir.path().join("a.csv").display()),
    );
    let shown = line(&harness).unwrap();
    assert!(
        shown.starts_with("export a.csv · 0 rows · :export cancel"),
        "{shown}"
    );
}

/// F5: `:export cancel` with one running cancels it; with two it opens
/// the picker, whose digit, `ctrl+n` and Enter cancel the one picked, and
/// whose Esc closes it.
#[test]
fn export_cancel_picks_among_two() {
    let (mut harness, tab, _id) = covers_in(Look::omarchy());
    let (_dir, one, two) = two_running(&mut harness, tab);
    run_line(&mut harness, tab, ":export cancel");
    assert!(cancels(&harness).is_empty(), "nothing yet");
    assert!(matches!(harness.app.dialog, Some(Dialog::CancelExport(_))));
    assert!(
        painted(&harness, "1  book_covers.csv 50%"),
        "{:?}",
        harness.painted
    );
    assert!(painted(&harness, "2  events.jsonl 4,000 rows"));
    harness.press(Key::Escape, Modifiers::NONE);
    assert!(harness.app.dialog.is_none(), "Esc closes it");
    // A digit picks its row.
    run_line(&mut harness, tab, ":export cancel");
    harness.press(Key::Num2, Modifiers::NONE);
    assert_eq!(cancels(&harness), [two]);
    assert!(harness.app.dialog.is_none());
    // ctrl+n then Enter: the second row again, and wrapping round to the
    // first.
    run_line(&mut harness, tab, ":export cancel");
    harness.press(Key::N, Modifiers::CTRL);
    harness.press(Key::N, Modifiers::CTRL);
    harness.press(Key::Enter, Modifiers::NONE);
    assert_eq!(cancels(&harness), [two, one]);
    // With one left running, it is cancelled at once.
    tell(&mut harness, two, ExportState::Cancelled);
    run_line(&mut harness, tab, ":export cancel");
    assert!(harness.app.dialog.is_none());
    assert_eq!(cancels(&harness), [two, one, one]);
}

/// S14: `go`, Esc and `:export` act on the export whose line shows.
#[test]
fn go_esc_and_export_act_on_the_line_shown() {
    let (mut harness, tab, _id) = covers_in(Look::omarchy());
    let (dir, one, two) = two_running(&mut harness, tab);
    tell(
        &mut harness,
        one,
        ExportState::Failed(ExportFailure::DiskFull { written: 10 }),
    );
    tell(&mut harness, two, done());
    // The done shows: `go` reveals its file.
    type_key(&mut harness, Key::G, "g");
    type_key(&mut harness, Key::O, "o");
    let revealed = harness
        .app
        .backend
        .sent
        .iter()
        .find_map(|command| match command {
            Command::Reveal { path } => Some(path.clone()),
            _ => None,
        });
    assert_eq!(revealed, Some(dir.path().join("events.jsonl")));
    // Once it goes, the failure shows: `:export` tries it again.
    harness.app.exports[1].status = crate::export_sheet::ExportStatus::Cancelled {
        at: std::time::Instant::now() - crate::export_sheet::TOLD_FOR,
    };
    harness.settle();
    let sent = harness.app.backend.sent.len();
    run_line(&mut harness, tab, ":export");
    let again = harness.app.backend.sent[sent..].iter().any(|command| {
        matches!(command, Command::Export(job)
            if job.destination == harness.app.exports[0].job.destination)
    });
    assert!(again, "Try again on the failure shown");
    assert!(harness.app.dialog.is_none(), "no float");
    // Esc on a failure shown puts that one away.
    let again = harness.app.exports[0].job.id;
    tell(
        &mut harness,
        again,
        ExportState::Failed(ExportFailure::DiskFull { written: 10 }),
    );
    harness.press(Key::Escape, Modifiers::NONE);
    assert!(harness.app.exports.is_empty());
}

/// A copy's end takes it out of the list: the export after it, a float's
/// file found there, keeps its line and opens no float (review).
#[test]
fn a_copy_ending_leaves_the_next_export_where_it_was() {
    use crate::export_sheet::{DestinationChoice, ExportChange};
    let (mut harness, tab, _id) = covers_in(Look::omarchy());
    let dir = tempfile::tempdir().unwrap();
    harness.app.apply(Action::OpenExport { tab, format: None });
    harness
        .app
        .apply(Action::ChangeExport(ExportChange::Destination(
            DestinationChoice::Clipboard,
        )));
    harness.app.apply(Action::StartExport);
    harness.settle();
    let copy = harness.app.exports[0].job.id;
    harness.app.apply(Action::OpenExport { tab, format: None });
    harness
        .app
        .apply(Action::ChangeExport(ExportChange::Destination(
            DestinationChoice::File,
        )));
    let target = dir.path().join("b.csv");
    harness.app.apply(Action::ChangeExport(ExportChange::Path(
        target.display().to_string(),
    )));
    harness.app.apply(Action::StartExport);
    harness.settle();
    let file = harness.app.exports[1].job.id;
    // A float is up as it fails: its line stays.
    harness.app.apply(Action::OpenExport { tab, format: None });
    harness.settle();
    tell(
        &mut harness,
        file,
        ExportState::Failed(ExportFailure::Exists {
            path: target.clone(),
        }),
    );
    harness.app.apply(Action::CloseDialog);
    harness.settle();
    tell(
        &mut harness,
        copy,
        ExportState::Done {
            rows: 4,
            bytes: 10,
            took: std::time::Duration::from_secs(1),
            copied: Some("a,b".into()),
            statements: 0,
        },
    );
    assert!(harness.app.dialog.is_none(), "no float for the copy's end");
    let left: Vec<ExportId> = harness.app.exports.iter().map(|e| e.job.id).collect();
    assert_eq!(left, [file]);
}

/// B6: ⌘⇧' goes to the newest toast with a button: a cancelled one has
/// none (review).
#[test]
fn the_toast_key_passes_over_a_cancelled_toast() {
    let (mut harness, tab, _id) = covers_in(Look::macos());
    let (_dir, _one, two) = two_running(&mut harness, tab);
    tell(&mut harness, two, ExportState::Cancelled);
    harness.press(Key::Quote, Modifiers::COMMAND | Modifiers::SHIFT);
    assert_eq!(focused_label(&mut harness).as_deref(), Some("Cancel"));
}

/// Choose folder…'s answer reaches its failed export while a sheet is
/// open for another; and an export tried again goes last, as it starts
/// anew (plan 4b, B22; review).
#[test]
fn choose_folder_answers_beside_an_open_sheet_and_goes_last() {
    let (mut harness, tab, _id) = covers_in(Look::macos());
    let (dir, one, two) = two_running(&mut harness, tab);
    tell(
        &mut harness,
        one,
        ExportState::Failed(ExportFailure::PermissionDenied {
            folder: dir.path().to_path_buf(),
        }),
    );
    harness.app.apply(Action::ExportFailureAction(Some(one)));
    assert_eq!(
        harness.app.backend.export_asks.last().map(|ask| ask.0),
        Some(one)
    );
    // Another sheet is opened while the panel is up.
    harness.app.apply(Action::OpenExport { tab, format: None });
    harness.settle();
    assert!(matches!(harness.app.dialog, Some(Dialog::Export(_))));
    let there = dir.path().join("elsewhere.csv");
    harness.app.apply(Action::Backend(Event::ExportPath {
        job: one,
        path: Some(there.clone()),
        replace: false,
    }));
    harness.settle();
    let order: Vec<ExportId> = harness.app.exports.iter().map(|e| e.job.id).collect();
    assert_eq!(order.len(), 2);
    assert_eq!(order[0], two, "the one tried again goes last");
    assert_eq!(harness.app.exports[1].file, "elsewhere.csv");
    assert!(harness.app.exports[1].running());
}

/// Review: the picker keeps the export the keyboard is on though one
/// above it ends, and Enter held from the line that opened it cancels
/// nothing.
#[test]
fn the_picker_keeps_its_export_and_waits_for_a_fresh_enter() {
    let (mut harness, tab, _id) = covers_in(Look::omarchy());
    let (dir, one, two) = two_running(&mut harness, tab);
    harness.app.apply(Action::ExportTo {
        tab,
        path: dir.path().join("third.md").display().to_string(),
        replace: false,
    });
    harness.settle();
    assert_eq!(harness.app.exports.len(), 3);
    // Enter held from the `:` line that asks for it.
    type_key(&mut harness, Key::Colon, ":");
    harness.frame(vec![egui::Event::Text("export cancel".into())]);
    for _ in 0..3 {
        harness.frame(vec![crate::testing::key(Key::Enter, Modifiers::NONE)]);
    }
    harness.frame(vec![crate::testing::release(Key::Enter, Modifiers::NONE)]);
    harness.settle();
    assert!(
        matches!(harness.app.dialog, Some(Dialog::CancelExport(_))),
        "{:?}",
        harness.app.dialog
    );
    assert!(
        cancels(&harness).is_empty(),
        "the held Enter cancels nothing"
    );
    harness.press(Key::N, Modifiers::CTRL);
    tell(&mut harness, one, done());
    harness.press(Key::Enter, Modifiers::NONE);
    assert_eq!(
        cancels(&harness),
        [two],
        "the lit one, not the row now in its place"
    );
}

/// Review: in the terminal look, a failure's float opens on its own
/// export's sheet, its choices as they were.
#[test]
fn a_failures_float_is_its_own_sheet() {
    let (mut harness, tab, _id) = covers_in(Look::omarchy());
    type_key(&mut harness, Key::E, "E");
    let dir = tempfile::tempdir().unwrap();
    let change = |harness: &mut Harness, change| {
        harness.app.apply(Action::ChangeExport(change));
        harness.settle();
    };
    use crate::export_sheet::ExportChange;
    change(&mut harness, ExportChange::Choose(true));
    change(&mut harness, ExportChange::Toggle(0));
    let path = dir.path().join("locked").join("covers.csv");
    change(&mut harness, ExportChange::Path(path.display().to_string()));
    harness.app.apply(Action::StartExport);
    harness.settle();
    let job = harness.app.exports[0].job.id;
    tell(
        &mut harness,
        job,
        ExportState::Failed(ExportFailure::PermissionDenied {
            folder: dir.path().join("locked"),
        }),
    );
    run_line(&mut harness, tab, ":export");
    let Some(Dialog::Export(sheet)) = &harness.app.dialog else {
        panic!("no float: {:?}", harness.app.dialog);
    };
    assert!(
        sheet.choose && !sheet.columns[0].included,
        "its own columns"
    );
    assert_eq!(sheet.path, path.display().to_string());
    assert!(harness.app.exports.is_empty(), "the failure is put away");
}
