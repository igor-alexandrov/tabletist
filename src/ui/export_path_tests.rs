//! The terminal look's save to line, checked against run 4's spec
//! (`docs/superpowers/specs/2026-11-04-export-copy-as-4-result-design.md`,
//! tests 8 and 9; S15 and S16).
//!
//! Behaviour and words, on the Bookshop's `book_covers`. The backend is a
//! recording one: the folder's names it would read are given, as its
//! answer would give them. No picture of the design is compared.

use std::path::PathBuf;

use egui::{Key, Modifiers};

use super::insert_audit_tests::{painted, type_key};
use crate::backend::export::ExportId;
use crate::backend::{Command, Event};
use crate::export::Format;
use crate::export_sheet::{ExportSheet, Listed};
use crate::model::{Action, Dialog};
use crate::testing::Harness;
use crate::theme::Look;

/// The Bookshop's covers in the terminal look, its float open with the
/// keyboard on the save to line, and the home and downloads folders the
/// test's own.
fn float() -> Harness {
    let mut harness = Harness::with_size(egui::vec2(1400.0, 1400.0));
    harness.set_look(Look::omarchy());
    let (tab, _id) = harness.book_covers();
    harness.app.workspace_mut(tab).unwrap().pane = crate::model::Pane::Grid;
    harness.app.dirs.home = Some(PathBuf::from("/home/reader"));
    harness.app.dirs.downloads = PathBuf::from("/home/reader/Downloads");
    harness.settle();
    type_key(&mut harness, Key::E, "E");
    let id = crate::ui::export::path_id(sheet(&harness));
    harness.ctx.memory_mut(|memory| memory.request_focus(id));
    harness.settle();
    harness
}

fn sheet(harness: &Harness) -> &ExportSheet {
    match &harness.app.dialog {
        Some(Dialog::Export(sheet)) => sheet,
        other => panic!("no float: {other:?}"),
    }
}

/// The line, as typed into its field: what it held goes first.
fn type_line(harness: &mut Harness, line: &str) {
    harness.frame(vec![
        crate::testing::key(Key::A, Modifiers::COMMAND),
        egui::Event::Text(line.into()),
    ]);
    harness.settle();
}

/// The folder listings asked so far: the line each was for, its folder
/// and its last part.
fn asks(harness: &Harness) -> Vec<(String, PathBuf, String)> {
    harness
        .app
        .backend
        .sent
        .iter()
        .filter_map(|command| match command {
            Command::ListFolder {
                typed,
                folder,
                start,
                ..
            } => Some((typed.clone(), folder.clone(), start.clone())),
            _ => None,
        })
        .collect()
}

/// The backend's answer for the line `typed`.
fn answer(harness: &mut Harness, typed: &str, names: &[(&str, bool)]) {
    let asked = sheet(harness).asked;
    let names = names
        .iter()
        .map(|(name, folder)| Listed {
            name: (*name).to_owned(),
            folder: *folder,
        })
        .collect();
    harness.app.apply(Action::Backend(Event::FolderListed {
        asked,
        typed: typed.to_owned(),
        names,
    }));
    harness.settle();
}

/// Test 8: typing asks for the folder's names off this thread (the
/// reducer reads no disk: nothing is listed until the answer), and the
/// answer is listed under the line.
#[test]
fn the_path_line_lists_what_it_starts() {
    let mut harness = float();
    type_line(&mut harness, "~/Downloads/rev");
    assert_eq!(
        asks(&harness).last(),
        Some(&(
            "~/Downloads/rev".to_owned(),
            PathBuf::from("/home/reader/Downloads"),
            "rev".to_owned()
        ))
    );
    assert!(sheet(&harness).listed.is_none(), "nothing until it answers");
    answer(
        &mut harness,
        "~/Downloads/rev",
        &[("reviews", true), ("reviews.csv", false)],
    );
    assert!(painted(&harness, "reviews/"), "{:?}", harness.painted);
    assert!(painted(&harness, "reviews.csv"));
    // Nothing matches: no list.
    type_line(&mut harness, "~/Downloads/zz");
    answer(&mut harness, "~/Downloads/zz", &[]);
    assert!(sheet(&harness).listed.is_none());
    // An empty line asks nothing.
    let before = asks(&harness).len();
    type_line(&mut harness, "");
    assert_eq!(asks(&harness).len(), before);
}

/// Test 8: an answer for a line since changed is dropped.
#[test]
fn an_answer_for_an_old_line_is_dropped() {
    let mut harness = float();
    type_line(&mut harness, "~/Downloads/r");
    type_line(&mut harness, "~/Downloads/re");
    // One listing at a time: the second line waits for the first's answer
    // (review).
    let typed: Vec<String> = asks(&harness).into_iter().map(|ask| ask.0).collect();
    assert_eq!(typed, ["~/Downloads/r"]);
    answer(&mut harness, "~/Downloads/r", &[("reports", true)]);
    assert!(sheet(&harness).listed.is_none(), "the old line's");
    assert!(!painted(&harness, "reports/"));
    assert_eq!(
        asks(&harness).last().map(|ask| ask.0.as_str()),
        Some("~/Downloads/re"),
        "then the line as it is"
    );
    answer(&mut harness, "~/Downloads/re", &[("reviews.csv", false)]);
    assert!(painted(&harness, "reviews.csv"));
}

/// Test 8: `ctrl+n` and `ctrl+p` (↓ ↑) move in the list, round, and Enter
/// takes the lit name, a file's setting the format; nothing exports.
#[test]
fn ctrl_n_and_enter_take_a_name() {
    let mut harness = float();
    type_line(&mut harness, "~/Downloads/rev");
    answer(
        &mut harness,
        "~/Downloads/rev",
        &[("reviews.json", false), ("reviews.md", false)],
    );
    harness.press(Key::N, Modifiers::CTRL);
    assert_eq!(sheet(&harness).listed.as_ref().unwrap().lit, 1);
    harness.press(Key::ArrowDown, Modifiers::NONE);
    assert_eq!(sheet(&harness).listed.as_ref().unwrap().lit, 0, "round");
    harness.press(Key::P, Modifiers::CTRL);
    assert_eq!(sheet(&harness).listed.as_ref().unwrap().lit, 1);
    let asked = asks(&harness).len();
    harness.press(Key::Enter, Modifiers::NONE);
    let open = sheet(&harness);
    assert_eq!(open.path, "~/Downloads/reviews.md");
    assert_eq!(open.format, Format::Markdown);
    assert!(open.listed.is_none(), "a file closes the list");
    assert_eq!(asks(&harness).len(), asked, "and lists nothing more");
    let exported = harness
        .app
        .backend
        .sent
        .iter()
        .any(|command| matches!(command, Command::Export(_)));
    assert!(!exported, "Enter took the name, not the export");
    // Enter again exports.
    harness.press(Key::Enter, Modifiers::NONE);
    assert!(
        harness
            .app
            .backend
            .sent
            .iter()
            .any(|command| matches!(command, Command::Export(_)))
    );
}

/// Test 8: a folder taken lists its own names.
#[test]
fn a_folder_taken_lists_its_own() {
    let mut harness = float();
    type_line(&mut harness, "~/Dow");
    answer(&mut harness, "~/Dow", &[("Downloads", true)]);
    harness.press(Key::Enter, Modifiers::NONE);
    assert_eq!(sheet(&harness).path, "~/Downloads/");
    assert_eq!(
        asks(&harness).last(),
        Some(&(
            "~/Downloads/".to_owned(),
            PathBuf::from("/home/reader/Downloads/"),
            String::new()
        ))
    );
    // A click on a name takes it too.
    answer(
        &mut harness,
        "~/Downloads/",
        &[("old", true), ("reviews.csv", false)],
    );
    harness.click("reviews.csv");
    assert_eq!(sheet(&harness).path, "~/Downloads/reviews.csv");
}

/// Test 8: Esc closes the list and keeps the float, the keyboard still
/// on the line; a second Esc closes the float.
#[test]
fn esc_closes_the_list_then_the_float() {
    let mut harness = float();
    type_line(&mut harness, "~/Downloads/rev");
    answer(&mut harness, "~/Downloads/rev", &[("reviews.csv", false)]);
    harness.press(Key::Escape, Modifiers::NONE);
    assert!(sheet(&harness).listed.is_none());
    let id = crate::ui::export::path_id(sheet(&harness));
    assert_eq!(harness.ctx.memory(|memory| memory.focused()), Some(id));
    harness.press(Key::Escape, Modifiers::NONE);
    assert!(harness.app.dialog.is_none(), "the float closes");
}

/// Test 9: `ctrl+o` asks the system's dialog with the line's path; its
/// answer fills the line and marks a replace; nothing starts until Enter.
#[test]
fn ctrl_o_fills_the_line_and_starts_nothing() {
    let mut harness = float();
    type_line(&mut harness, "~/Downloads/covers.csv");
    // The line's hint, as the keymap writes the key.
    let key = harness.app.keymap.key(
        crate::keymap::Layout::Omarchy,
        crate::keymap::Command::ExportPickFile,
    );
    let hint = format!("{} pick", key.as_str());
    assert!(painted(&harness, &hint), "{:?}", harness.painted);
    assert!(
        harness
            .painted
            .iter()
            .any(|(text, _)| text.contains("pick file")),
        "the foot's key: {:?}",
        harness.painted
    );
    // From another row: the answer gives the keyboard back to the line.
    harness.press(Key::Tab, Modifiers::NONE);
    let line = crate::ui::export::path_id(sheet(&harness));
    assert_ne!(harness.ctx.memory(|memory| memory.focused()), Some(line));
    harness.press(Key::O, Modifiers::CTRL);
    let asks = harness.app.backend.export_asks.clone();
    let job = ExportId(sheet(&harness).asked);
    assert_eq!(
        asks,
        [(job, PathBuf::from("/home/reader/Downloads/covers.csv"))]
    );
    assert!(sheet(&harness).asking);
    // Enter waits for the dialog.
    harness.press(Key::Enter, Modifiers::NONE);
    let started = |harness: &Harness| {
        harness
            .app
            .backend
            .sent
            .iter()
            .any(|command| matches!(command, Command::Export(_)))
    };
    assert!(!started(&harness));
    let answered = PathBuf::from("/home/reader")
        .join("Desktop")
        .join("books.jsonl");
    harness.app.apply(Action::Backend(Event::ExportPath {
        job,
        path: Some(answered.clone()),
        replace: true,
    }));
    harness.settle();
    let open = sheet(&harness);
    let shown = ["~", "Desktop", "books.jsonl"].join(std::path::MAIN_SEPARATOR_STR);
    assert_eq!(open.path, shown);
    assert_eq!(open.format, Format::JsonLines);
    assert!(open.exists, "a replace, as the dialog asked");
    assert!(!open.asking);
    assert!(painted(&harness, "! exists · enter again replaces it"));
    assert!(!started(&harness), "nothing starts on the answer");
    let id = crate::ui::export::path_id(sheet(&harness));
    assert_eq!(harness.ctx.memory(|memory| memory.focused()), Some(id));
    harness.press(Key::Enter, Modifiers::NONE);
    let job = harness
        .app
        .backend
        .sent
        .iter()
        .find_map(|command| match command {
            Command::Export(job) => Some(job.destination.clone()),
            _ => None,
        });
    assert_eq!(
        job,
        Some(crate::backend::export::Destination::File {
            path: answered,
            replace: true
        })
    );
    // A closed dialog leaves the line as it was.
    let mut harness = float();
    type_line(&mut harness, "~/Downloads/kept.csv");
    harness.press(Key::O, Modifiers::CTRL);
    let job = ExportId(sheet(&harness).asked);
    harness.app.apply(Action::Backend(Event::ExportPath {
        job,
        path: None,
        replace: false,
    }));
    harness.settle();
    assert_eq!(sheet(&harness).path, "~/Downloads/kept.csv");
    assert!(!sheet(&harness).asking && !sheet(&harness).picking);
}

/// B9: the list goes with the keyboard; back on the line it waits for the
/// next key (review).
#[test]
fn the_list_goes_when_the_line_loses_the_keyboard() {
    let mut harness = float();
    type_line(&mut harness, "~/Downloads/rev");
    answer(&mut harness, "~/Downloads/rev", &[("reviews.csv", false)]);
    harness.ctx.memory_mut(|memory| {
        let id = memory.focused().unwrap();
        memory.surrender_focus(id);
    });
    harness.settle();
    assert!(sheet(&harness).listed.is_none());
    let id = crate::ui::export::path_id(sheet(&harness));
    harness.ctx.memory_mut(|memory| memory.request_focus(id));
    harness.settle();
    assert!(!painted(&harness, "reviews.csv"));
}

/// S16 and B20: no list while `ctrl+o`'s dialog is up, and Enter on a
/// folder taken waits for a file's name (review).
#[test]
fn the_list_waits_for_the_dialog_and_a_folder_for_a_name() {
    let mut harness = float();
    type_line(&mut harness, "~/Downloads/rev");
    harness.press(Key::O, Modifiers::CTRL);
    answer(&mut harness, "~/Downloads/rev", &[("reviews.csv", false)]);
    assert!(sheet(&harness).listed.is_none(), "not under the dialog");
    let job = ExportId(sheet(&harness).asked);
    harness.app.apply(Action::Backend(Event::ExportPath {
        job,
        path: None,
        replace: false,
    }));
    type_line(&mut harness, "~/Dow");
    answer(&mut harness, "~/Dow", &[("Downloads", true)]);
    harness.press(Key::Enter, Modifiers::NONE);
    assert_eq!(sheet(&harness).path, "~/Downloads/");
    harness.press(Key::Enter, Modifiers::NONE);
    let exported = harness
        .app
        .backend
        .sent
        .iter()
        .any(|command| matches!(command, Command::Export(_)));
    assert!(!exported, "a folder is no file to write");
    assert!(matches!(harness.app.dialog, Some(Dialog::Export(_))));
}

/// Test 8: a click with the pointer on a listed name takes it, the line
/// keeping the keyboard for a folder's names next (review).
#[test]
fn a_pointer_click_takes_a_name() {
    let mut harness = float();
    type_line(&mut harness, "~/Dow");
    answer(
        &mut harness,
        "~/Dow",
        &[("Desktop", true), ("Downloads", true)],
    );
    let pos = harness
        .text_rects
        .iter()
        .find(|(text, _)| text == "Downloads/")
        .map(|(_, rect)| rect.center())
        .unwrap_or_else(|| panic!("not drawn: {:?}", harness.painted));
    let press = |pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    harness.frame(vec![egui::Event::PointerMoved(pos)]);
    harness.frame(vec![press(true)]);
    harness.frame(vec![press(false)]);
    harness.settle();
    assert_eq!(sheet(&harness).path, "~/Downloads/");
    let id = crate::ui::export::path_id(sheet(&harness));
    assert_eq!(harness.ctx.memory(|memory| memory.focused()), Some(id));
    assert_eq!(
        asks(&harness).last().map(|ask| ask.0.as_str()),
        Some("~/Downloads/")
    );
}

/// Review: a list put away stays away though an answer for the same line
/// still comes, and Esc then closes the float.
#[test]
fn a_late_answer_leaves_the_list_put_away() {
    let mut harness = float();
    type_line(&mut harness, "~/Downloads/rev");
    answer(&mut harness, "~/Downloads/rev", &[("reviews.csv", false)]);
    harness.press(Key::Escape, Modifiers::NONE);
    answer(&mut harness, "~/Downloads/rev", &[("reviews.csv", false)]);
    assert!(sheet(&harness).listed.is_none());
    assert!(!painted(&harness, "reviews.csv"));
    harness.press(Key::Escape, Modifiers::NONE);
    assert!(harness.app.dialog.is_none(), "the float closes");
}

/// Review: Enter held from the press that took a name takes nothing more
/// and starts nothing; typing goes on at the line's end.
#[test]
fn a_held_enter_after_a_pick_starts_nothing() {
    let mut harness = float();
    type_line(&mut harness, "~/Dow");
    answer(&mut harness, "~/Dow", &[("Downloads", true)]);
    for _ in 0..3 {
        harness.frame(vec![crate::testing::key(Key::Enter, Modifiers::NONE)]);
    }
    answer(&mut harness, "~/Downloads/", &[("reviews.csv", false)]);
    for _ in 0..3 {
        harness.frame(vec![crate::testing::key(Key::Enter, Modifiers::NONE)]);
    }
    harness.frame(vec![crate::testing::release(Key::Enter, Modifiers::NONE)]);
    harness.settle();
    assert_eq!(sheet(&harness).path, "~/Downloads/");
    let exported = harness
        .app
        .backend
        .sent
        .iter()
        .any(|command| matches!(command, Command::Export(_)));
    assert!(!exported);
    // The cursor at the line's end, after the part taken.
    harness.frame(vec![egui::Event::Text("a.md".into())]);
    harness.settle();
    assert_eq!(sheet(&harness).path, "~/Downloads/a.md");
}

/// Review: `ctrl+o` asks nothing while Review SQL shows, the line hidden.
#[test]
fn ctrl_o_waits_while_review_sql_shows() {
    let mut harness = float();
    harness.app.apply(Action::ChangeExport(
        crate::export_sheet::ExportChange::Review(true),
    ));
    harness.settle();
    harness.press(Key::O, Modifiers::CTRL);
    assert!(harness.app.backend.export_asks.is_empty());
    assert!(!sheet(&harness).asking);
}

/// Review: Enter writes the file the dialog answered, though the line
/// could not say its name (no UTF-8).
#[cfg(unix)]
#[test]
fn the_dialogs_file_is_written_whatever_the_line_says() {
    use std::os::unix::ffi::OsStrExt;
    let mut harness = float();
    harness.press(Key::O, Modifiers::CTRL);
    let job = ExportId(sheet(&harness).asked);
    let answered = PathBuf::from(std::ffi::OsStr::from_bytes(
        b"/home/reader/caf\xe9/books.csv",
    ));
    harness.app.apply(Action::Backend(Event::ExportPath {
        job,
        path: Some(answered.clone()),
        replace: false,
    }));
    harness.settle();
    harness.press(Key::Enter, Modifiers::NONE);
    let job = harness
        .app
        .backend
        .sent
        .iter()
        .find_map(|command| match command {
            Command::Export(job) => Some(job.destination.clone()),
            _ => None,
        });
    assert_eq!(
        job,
        Some(crate::backend::export::Destination::File {
            path: answered,
            replace: false
        })
    );
}

/// Review: a pointer resting on the list keeps it no longer than the
/// line keeps the keyboard.
#[test]
fn a_pointer_resting_on_the_list_does_not_keep_it() {
    let mut harness = float();
    type_line(&mut harness, "~/Dow");
    answer(&mut harness, "~/Dow", &[("Downloads", true)]);
    let pos = harness
        .text_rects
        .iter()
        .find(|(text, _)| text == "Downloads/")
        .map(|(_, rect)| rect.center())
        .unwrap_or_else(|| panic!("not drawn: {:?}", harness.painted));
    harness.frame(vec![egui::Event::PointerMoved(pos)]);
    harness.press(Key::Tab, Modifiers::NONE);
    assert!(sheet(&harness).listed.is_none());
}

/// Review: the format chosen again, the line unchanged, keeps the replace
/// the dialog asked for.
#[test]
fn the_same_format_again_keeps_the_dialogs_replace() {
    let mut harness = float();
    harness.press(Key::O, Modifiers::CTRL);
    let job = ExportId(sheet(&harness).asked);
    let answered = PathBuf::from("/home/reader").join("books.csv");
    harness.app.apply(Action::Backend(Event::ExportPath {
        job,
        path: Some(answered.clone()),
        replace: true,
    }));
    harness.app.apply(Action::ChangeExport(
        crate::export_sheet::ExportChange::Format(Format::Csv),
    ));
    harness.settle();
    assert!(sheet(&harness).exists, "still a replace");
    harness.press(Key::Enter, Modifiers::NONE);
    let job = harness
        .app
        .backend
        .sent
        .iter()
        .find_map(|command| match command {
            Command::Export(job) => Some(job.destination.clone()),
            _ => None,
        });
    assert_eq!(
        job,
        Some(crate::backend::export::Destination::File {
            path: answered,
            replace: true
        })
    );
}
