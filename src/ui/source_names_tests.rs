//! A Files connection's path lines in the terminal look (the "Omarchy –
//! Files, path completions" board): typing a path lists the names it
//! starts under the line, `ctrl+n`/`ctrl+p` move in them, Enter takes
//! one and Esc puts them away. The folder field lists folders; the add
//! line under the file list lists folders and the files a Files
//! connection reads, and adds the file taken.
//!
//! The backend is a recording one: the names a folder holds are given, as
//! its answer would give them. No picture of the design is compared.

use std::path::PathBuf;

use egui::{Key, Modifiers};

use crate::backend::{Command, Event};
use crate::export_sheet::Listed;
use crate::model::{Action, ConnectionForm, Dialog, FilesKind};
use crate::source_names::Keep;
use crate::testing::Harness;
use crate::theme::Look;

fn form(harness: &Harness) -> &ConnectionForm {
    match &harness.app.dialog {
        Some(Dialog::Connection(form)) => form,
        other => panic!("no connection dialog: {:?}", other.is_some()),
    }
}

/// A new Files connection's dialog in the terminal look, its `kind` of
/// Source chosen and the keyboard on its path line; the home folder the
/// test's own.
fn dialog(kind: FilesKind) -> Harness {
    let mut harness = Harness::new();
    harness.set_look(Look::omarchy());
    harness.app.dirs.home = Some(PathBuf::from("/home/reader"));
    harness.app.apply(Action::NewConnection);
    harness.settle();
    harness.click(&Look::omarchy().label("Files"));
    harness.app.apply(Action::SetFilesKind(kind));
    harness.settle();
    let id = crate::ui::connect_dialog::source_line_id(kind);
    harness.ctx.memory_mut(|memory| memory.request_focus(id));
    harness.settle();
    harness
}

/// The line, as typed into its field: what it held goes first.
fn type_line(harness: &mut Harness, line: &str) {
    harness.frame(vec![
        crate::testing::key(Key::A, Modifiers::COMMAND),
        egui::Event::Text(line.into()),
    ]);
    harness.settle();
}

/// The folder listings asked so far: the line each was for, its folder,
/// its last part and the names it keeps.
fn asks(harness: &Harness) -> Vec<(String, PathBuf, String, Keep)> {
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
                keep,
                ..
            } => Some((typed.clone(), folder.clone(), start.clone(), *keep)),
            _ => None,
        })
        .collect()
}

/// The backend's answer for the listing on its way, of the line `typed`.
fn answer(harness: &mut Harness, typed: &str, names: &[(&str, bool)]) {
    let asked = form(harness).files_listing.expect("a listing on its way");
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

fn painted(harness: &Harness, text: &str) -> bool {
    harness.painted.iter().any(|(painted, _)| painted == text)
}

fn lit(harness: &Harness) -> usize {
    form(harness)
        .files_names
        .as_ref()
        .expect("names listed")
        .lit
}

fn on_line(harness: &Harness) -> bool {
    let id = crate::ui::connect_dialog::source_line_id(form(harness).files_kind);
    harness.ctx.memory(|memory| memory.has_focus(id))
}

#[test]
fn the_folder_field_lists_the_folders_it_starts() {
    let mut harness = dialog(FilesKind::Folder);
    type_line(&mut harness, "~/Data/book");
    assert_eq!(
        asks(&harness).last(),
        Some(&(
            "~/Data/book".to_owned(),
            PathBuf::from("/home/reader/Data"),
            "book".to_owned(),
            Keep::Folders
        ))
    );
    assert!(
        form(&harness).files_names.is_none(),
        "nothing until it answers"
    );
    answer(
        &mut harness,
        "~/Data/book",
        &[("bookshop-exports", true), ("books-archive", true)],
    );
    assert!(
        painted(&harness, "bookshop-exports/"),
        "{:?}",
        harness.painted
    );
    assert!(painted(&harness, "books-archive/"));
    // Nothing matches: no list.
    type_line(&mut harness, "~/Data/zz");
    answer(&mut harness, "~/Data/zz", &[]);
    assert!(form(&harness).files_names.is_none());
}

/// A line with no folder, or one not absolute, lists nothing: the
/// connection would read it from wherever the app was started.
#[test]
fn a_line_that_is_not_absolute_lists_nothing() {
    let mut harness = dialog(FilesKind::Folder);
    for line in ["book", "Data/book", ""] {
        type_line(&mut harness, line);
    }
    assert!(asks(&harness).is_empty(), "{:?}", asks(&harness));
}

/// One listing at a time: a line typed meanwhile is listed once the
/// first answers, and that answer, for a line since changed, is dropped.
#[test]
fn an_answer_for_an_old_line_is_dropped() {
    let mut harness = dialog(FilesKind::Folder);
    type_line(&mut harness, "/data/b");
    type_line(&mut harness, "/data/bo");
    let typed: Vec<String> = asks(&harness).into_iter().map(|ask| ask.0).collect();
    assert_eq!(typed, ["/data/b"]);
    answer(&mut harness, "/data/b", &[("backups", true)]);
    assert!(form(&harness).files_names.is_none(), "the old line's");
    assert!(!painted(&harness, "backups/"));
    assert_eq!(
        asks(&harness).last().map(|ask| ask.0.as_str()),
        Some("/data/bo"),
        "then the line as it is"
    );
    answer(&mut harness, "/data/bo", &[("bookshop-exports", true)]);
    assert!(painted(&harness, "bookshop-exports/"));
}

/// `ctrl+n` and `ctrl+p` (↓ ↑) move in the names, round; Enter takes the
/// lit folder with its `/` and lists its own folders. The folder is not
/// read until the line is left.
#[test]
fn enter_takes_a_folder_and_lists_its_own() {
    let mut harness = dialog(FilesKind::Folder);
    type_line(&mut harness, "~/Data/book");
    answer(
        &mut harness,
        "~/Data/book",
        &[("books-archive", true), ("bookshop-exports", true)],
    );
    harness.press(Key::N, Modifiers::CTRL);
    assert_eq!(lit(&harness), 1);
    harness.press(Key::ArrowDown, Modifiers::NONE);
    assert_eq!(lit(&harness), 0, "round");
    harness.press(Key::P, Modifiers::CTRL);
    assert_eq!(lit(&harness), 1);
    let read = harness.app.backend.sources_read.len();
    harness.press(Key::Enter, Modifiers::NONE);
    assert_eq!(form(&harness).files_folder, "~/Data/bookshop-exports/");
    assert_eq!(
        asks(&harness).last(),
        Some(&(
            "~/Data/bookshop-exports/".to_owned(),
            PathBuf::from("/home/reader/Data/bookshop-exports"),
            String::new(),
            Keep::Folders
        ))
    );
    assert!(on_line(&harness), "the line keeps the keyboard");
    assert_eq!(harness.app.backend.sources_read.len(), read, "nor is read");
    assert!(matches!(harness.app.dialog, Some(Dialog::Connection(_))));
}

/// Esc puts the names away and leaves the dialog open; the next Esc
/// cancels it. The names stay away until the line changes.
#[test]
fn esc_puts_the_names_away_then_cancels() {
    let mut harness = dialog(FilesKind::Folder);
    type_line(&mut harness, "/data/book");
    answer(&mut harness, "/data/book", &[("bookshop-exports", true)]);
    harness.press(Key::Escape, Modifiers::NONE);
    assert!(form(&harness).files_names.is_none());
    assert!(form(&harness).files_names_closed);
    assert!(on_line(&harness), "the line keeps the keyboard");
    harness.press(Key::Escape, Modifiers::NONE);
    assert!(harness.app.dialog.is_none(), "the second Esc cancels");
}

/// Tab moves to the next field, as always: the names go, and the folder
/// is read as it was before.
#[test]
fn tab_leaves_the_line_and_reads_the_folder() {
    let mut harness = dialog(FilesKind::Folder);
    type_line(&mut harness, "/data/book");
    answer(&mut harness, "/data/book", &[("bookshop-exports", true)]);
    let read = harness.app.backend.sources_read.len();
    harness.press(Key::Tab, Modifiers::NONE);
    harness.settle();
    assert!(!on_line(&harness));
    assert!(form(&harness).files_names.is_none());
    assert_eq!(
        form(&harness).files_folder,
        "/data/book",
        "Tab takes nothing"
    );
    let sources = &harness.app.backend.sources_read;
    assert!(sources.len() > read, "the folder is read");
    assert_eq!(
        sources.last(),
        Some(&tabletist_db::files::Source::Folder("/data/book".into()))
    );
}

/// A click on a name takes it, as Enter does, and reads nothing.
#[test]
fn a_click_takes_a_name() {
    let mut harness = dialog(FilesKind::Folder);
    type_line(&mut harness, "/data/book");
    answer(
        &mut harness,
        "/data/book",
        &[("books-archive", true), ("bookshop-exports", true)],
    );
    let read = harness.app.backend.sources_read.len();
    harness.click("bookshop-exports/");
    harness.settle();
    assert_eq!(form(&harness).files_folder, "/data/bookshop-exports/");
    assert_eq!(harness.app.backend.sources_read.len(), read);
}
