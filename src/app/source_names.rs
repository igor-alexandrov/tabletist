//! The names a Files connection's path lines list as a path is typed, in
//! the terminal look: the folder field's folders, and the add line's
//! folders and files. Asked for off this thread, as the export float's
//! save to line asks for its own (`export.rs`).

use std::path::Path;

use super::App;
use crate::backend::Command;
use crate::export_sheet::{Listed, PathList, complete};
use crate::model::{Dialog, FilesKind, SourceNames};
use tabletist_db::Driver;

impl App {
    pub(super) fn source_names(&mut self, change: SourceNames) {
        let Some(Dialog::Connection(form)) = &mut self.dialog else {
            return;
        };
        match change {
            SourceNames::Typed => {
                form.files_names = None;
                form.files_names_closed = false;
                self.list_source();
            }
            SourceNames::Move(by) => {
                if let Some(list) = &mut form.files_names {
                    crate::source_names::moved(list, by);
                }
            }
            SourceNames::Take(at) => self.take_source_name(at),
            SourceNames::Close => {
                form.files_names = None;
                form.files_names_closed = true;
            }
            SourceNames::Add => {
                form.files_names = None;
                // Enter takes the keyboard from the line: back to it.
                form.files_to_add += 1;
                // Only a file's path from a root: not a folder's (a held
                // Enter after one was taken), nor one the connection would
                // read from wherever the app was started.
                let typed = form.files_add.trim();
                let path = tabletist_db::files::expanded(Path::new(typed));
                let file = !typed.is_empty()
                    && !typed.ends_with(['/', std::path::MAIN_SEPARATOR])
                    && path.has_root();
                if file {
                    let typed = std::mem::take(&mut form.files_add);
                    self.add_typed_file(&typed);
                }
            }
        }
    }

    /// Asks for the names the path line starts, off this thread: none in
    /// the macOS look, whose Source has no typed line, nor for a line with
    /// no folder to list (`source_names::ask`).
    fn list_source(&mut self) {
        if !self.look.terminal {
            return;
        }
        let asked = self.next_id();
        let home = self.dirs.home.clone();
        let Some(Dialog::Connection(form)) = &mut self.dialog else {
            return;
        };
        // Only a Files connection's Source has path lines.
        match form.driver {
            Driver::Files => {}
            Driver::Postgres | Driver::MySql | Driver::Sqlite | Driver::BigQuery => return,
        }
        // One listing at a time: the one on its way is followed by another
        // for the line as it is then (`source_listed`).
        if form.files_listing.is_some() {
            return;
        }
        let (line, keep) = form.source_line();
        let Some((folder, start)) = crate::source_names::ask(line, home.as_deref()) else {
            return;
        };
        let command = Command::ListFolder {
            asked,
            typed: line.to_owned(),
            folder,
            start,
            keep,
        };
        form.files_listing = Some(asked);
        self.backend.send(command);
    }

    /// The names a path line starts: kept for the line they were asked
    /// for, dropped for one since changed or another form's.
    pub(super) fn source_listed(&mut self, asked: u64, typed: &str, names: Vec<Listed>) {
        let Some(Dialog::Connection(form)) = &mut self.dialog else {
            return;
        };
        if form.files_listing != Some(asked) {
            return;
        }
        form.files_listing = None;
        // The line changed while it was read: the line as it is now.
        if form.source_line().0 != typed {
            self.list_source();
            return;
        }
        if form.files_names_closed {
            return;
        }
        // A file the list holds already is not offered again.
        let names: Vec<Listed> = names
            .into_iter()
            .filter(|listed| listed.folder || !form.lists_file(&complete(typed, listed)))
            .collect();
        // Nothing matches: no list.
        form.files_names = (!names.is_empty()).then(|| PathList {
            line: typed.to_owned(),
            names,
            lit: 0,
        });
    }

    /// A name of the path line's list taken: the one at `at`, else the lit
    /// one. A folder's lists its own names next; a file's (the add line
    /// lists files) is added to the list, and the line is emptied for the
    /// next.
    fn take_source_name(&mut self, at: Option<usize>) {
        let Some(Dialog::Connection(form)) = &mut self.dialog else {
            return;
        };
        let line = form.source_line().0.to_owned();
        let picked = match at {
            Some(at) => form
                .files_names
                .as_ref()
                .filter(|list| list.line == line)
                .and_then(|list| list.names.get(at)),
            None => form.lit_source_name(),
        };
        let Some(picked) = picked.cloned() else {
            return;
        };
        let taken = complete(&line, &picked);
        form.files_names = None;
        // Typing goes on at the line's end, not where the part taken was.
        match form.files_kind {
            FilesKind::Folder => form.files_to_folder += 1,
            FilesKind::Files => form.files_to_add += 1,
        }
        match (picked.folder, form.files_kind) {
            (true, FilesKind::Folder) => {
                form.files_folder = taken;
                self.list_source();
            }
            (true, FilesKind::Files) => {
                form.files_add = taken;
                self.list_source();
            }
            (false, _) => {
                form.files_add.clear();
                self.add_typed_file(&taken);
            }
        }
    }

    /// Adds the file `typed` names to the list, once (however it is
    /// written), after the files already listed, and reads the Source
    /// again: as the file dialog's
    /// files are added. A path that is not there is listed all the same,
    /// and its row says it is missing.
    fn add_typed_file(&mut self, typed: &str) {
        let Some(Dialog::Connection(form)) = &mut self.dialog else {
            return;
        };
        let at = match form.listed_at(typed) {
            Some(at) => at,
            None => {
                let path = tabletist_db::files::expanded(Path::new(typed.trim()));
                form.files_list.push(tabletist_db::files::shown(&path));
                form.files_list.len() - 1
            }
        };
        form.files_cursor = Some(at);
        self.read_form_source();
    }
}
