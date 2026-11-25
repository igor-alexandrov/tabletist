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

/// The add line lists folders and the files a Files connection reads;
/// Enter on a file adds it to the list, empties the line and reads the
/// Source again, the keyboard staying on the line for the next.
#[test]
fn the_add_line_adds_the_file_taken() {
    let mut harness = dialog(FilesKind::Files);
    type_line(&mut harness, "/data/bookshop-exports/ret");
    assert_eq!(
        asks(&harness).last(),
        Some(&(
            "/data/bookshop-exports/ret".to_owned(),
            PathBuf::from("/data/bookshop-exports/"),
            "ret".to_owned(),
            Keep::Readable
        ))
    );
    answer(
        &mut harness,
        "/data/bookshop-exports/ret",
        &[("returns.csv", false), ("returns.xlsx", false)],
    );
    assert!(painted(&harness, "returns.csv"));
    let read = harness.app.backend.sources_read.len();
    harness.press(Key::Enter, Modifiers::NONE);
    harness.settle();
    let open = form(&harness);
    assert_eq!(open.files_list, ["/data/bookshop-exports/returns.csv"]);
    assert_eq!(open.files_cursor, Some(0));
    assert!(open.files_add.is_empty());
    assert!(open.files_names.is_none());
    assert_eq!(harness.app.backend.sources_read.len(), read + 1);
    assert!(on_line(&harness), "the line keeps the keyboard");
}

/// A file the list holds already is not offered again; its folder is.
#[test]
fn a_listed_file_is_not_offered_again() {
    let mut harness = dialog(FilesKind::Files);
    if let Some(Dialog::Connection(form)) = &mut harness.app.dialog {
        form.files_list = vec!["/data/bookshop-exports/orders.csv".into()];
    }
    type_line(&mut harness, "/data/bookshop-exports/ord");
    answer(
        &mut harness,
        "/data/bookshop-exports/ord",
        &[("orders-2025", true), ("orders.csv", false)],
    );
    let names: Vec<&str> = form(&harness)
        .files_names
        .as_ref()
        .expect("names listed")
        .names
        .iter()
        .map(|listed| listed.name.as_str())
        .collect();
    assert_eq!(names, ["orders-2025"]);
}

/// Enter with no name listed adds the path as typed: one that is not
/// there is listed all the same, its row saying it is missing.
/// (`enter_adds_only_a_files_path` has what it does not add.)
#[test]
fn enter_with_no_names_adds_the_path_as_typed() {
    let mut harness = dialog(FilesKind::Files);
    type_line(&mut harness, "/data/bookshop-exports/events.ndjson");
    answer(&mut harness, "/data/bookshop-exports/events.ndjson", &[]);
    harness.press(Key::Enter, Modifiers::NONE);
    harness.settle();
    let open = form(&harness);
    assert_eq!(open.files_list, ["/data/bookshop-exports/events.ndjson"]);
    assert!(open.files_add.is_empty());
    assert!(on_line(&harness), "the line keeps the keyboard");
    assert!(matches!(harness.app.dialog, Some(Dialog::Connection(_))));
}

/// In the add line `a` and `x` type: they add and remove files only
/// where no line has the keyboard. Each comes as a keyboard sends it, the
/// key and then its text.
#[test]
fn letters_in_the_add_line_are_typed() {
    let mut harness = dialog(FilesKind::Files);
    if let Some(Dialog::Connection(form)) = &mut harness.app.dialog {
        form.files_list = vec!["/data/bookshop-exports/orders.csv".into()];
    }
    for (key, text) in [(Key::A, "a"), (Key::X, "x")] {
        harness.frame(vec![
            crate::testing::key(key, Modifiers::NONE),
            egui::Event::Text(text.into()),
            crate::testing::release(key, Modifiers::NONE),
        ]);
    }
    harness.settle();
    let open = form(&harness);
    assert_eq!(open.files_add, "ax");
    assert_eq!(open.files_list.len(), 1, "x removed nothing");
    assert!(open.pick_request.is_none(), "a opened no picker");
}

/// Enter in the add line adds only a file's path from a root: not a
/// folder's, which ends in its `/`, nor a path with no root, which the
/// connection would read from wherever the app was started. The line
/// keeps its text and the keyboard.
#[test]
fn enter_adds_only_a_files_path() {
    let mut harness = dialog(FilesKind::Files);
    for line in ["/data/bookshop-exports/", "orders.csv"] {
        type_line(&mut harness, line);
        if form(&harness).files_listing.is_some() {
            answer(&mut harness, line, &[]);
        }
        harness.press(Key::Enter, Modifiers::NONE);
        harness.settle();
        let open = form(&harness);
        assert!(open.files_list.is_empty(), "{line}: {:?}", open.files_list);
        assert_eq!(open.files_add, line);
        assert!(on_line(&harness), "{line}: the line keeps the keyboard");
    }
}

/// Enter held after it took a folder takes nothing more, though the
/// folder lists nothing: the folder is not added as a file.
#[test]
fn enter_held_after_a_folder_adds_nothing() {
    let mut harness = dialog(FilesKind::Files);
    type_line(&mut harness, "/data/bookshop-exports/ret");
    answer(
        &mut harness,
        "/data/bookshop-exports/ret",
        &[("returns-2025", true)],
    );
    let enter = || crate::testing::key(Key::Enter, Modifiers::NONE);
    harness.frame(vec![enter()]);
    harness.settle();
    assert_eq!(
        form(&harness).files_add,
        "/data/bookshop-exports/returns-2025/"
    );
    answer(&mut harness, "/data/bookshop-exports/returns-2025/", &[]);
    // Still held: egui sends the key down again, as a repeat.
    harness.frame(vec![enter()]);
    harness.frame(vec![crate::testing::release(Key::Enter, Modifiers::NONE)]);
    harness.settle();
    let open = form(&harness);
    assert!(open.files_list.is_empty(), "{:?}", open.files_list);
    assert_eq!(open.files_add, "/data/bookshop-exports/returns-2025/");
}

/// While names are listed under the add line, Down moves in them, not
/// the file list's cursor above it.
#[test]
fn down_moves_in_the_names_not_the_list() {
    let mut harness = dialog(FilesKind::Files);
    if let Some(Dialog::Connection(form)) = &mut harness.app.dialog {
        form.files_list = vec![
            "/data/bookshop-exports/orders.csv".into(),
            "/data/bookshop-exports/customers.parquet".into(),
        ];
        form.files_cursor = Some(0);
    }
    type_line(&mut harness, "/data/bookshop-exports/ret");
    answer(
        &mut harness,
        "/data/bookshop-exports/ret",
        &[("returns.csv", false), ("returns.xlsx", false)],
    );
    harness.press(Key::ArrowDown, Modifiers::NONE);
    assert_eq!(lit(&harness), 1);
    assert_eq!(form(&harness).files_cursor, Some(0));
}

/// A file is found in the list however its path is written: typed again
/// with a `./` in it, it is not added a second time.
#[test]
fn a_file_typed_another_way_is_not_added_twice() {
    let mut harness = dialog(FilesKind::Files);
    if let Some(Dialog::Connection(form)) = &mut harness.app.dialog {
        form.files_list = vec!["/data/bookshop-exports/orders.csv".into()];
    }
    type_line(&mut harness, "/data/bookshop-exports/./orders.csv");
    answer(&mut harness, "/data/bookshop-exports/./orders.csv", &[]);
    harness.press(Key::Enter, Modifiers::NONE);
    harness.settle();
    let open = form(&harness);
    assert_eq!(open.files_list, ["/data/bookshop-exports/orders.csv"]);
    assert_eq!(open.files_cursor, Some(0));
}

/// Each line takes the keyboard back only for a name taken from its own
/// list: the other kind's line, shown when the kind changes, leaves it
/// on the kind's choice.
#[test]
fn a_name_taken_in_one_line_does_not_focus_the_other() {
    let mut harness = dialog(FilesKind::Folder);
    type_line(&mut harness, "~/Data/book");
    answer(&mut harness, "~/Data/book", &[("bookshop-exports", true)]);
    harness.press(Key::Enter, Modifiers::NONE);
    assert!(on_line(&harness), "the folder field keeps the keyboard");
    harness.app.apply(Action::SetFilesKind(FilesKind::Files));
    harness.settle();
    assert!(!on_line(&harness), "the add line takes no keyboard");

    let mut harness = dialog(FilesKind::Files);
    type_line(&mut harness, "/data/bookshop-exports/events.ndjson");
    answer(&mut harness, "/data/bookshop-exports/events.ndjson", &[]);
    harness.press(Key::Enter, Modifiers::NONE);
    harness.settle();
    assert!(on_line(&harness), "the add line keeps the keyboard");
    harness.app.apply(Action::SetFilesKind(FilesKind::Folder));
    harness.settle();
    assert!(!on_line(&harness), "the folder field takes no keyboard");
}
