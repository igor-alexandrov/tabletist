//! Exporting a table or a SQL result: the sheet's reducers, the export
//! they start, and what the backend says of it (specs
//! `docs/superpowers/specs/2026-11-03-export-copy-as-3-table-design.md`
//! and `2026-11-04-export-copy-as-4-result-design.md`).

use std::path::PathBuf;
use std::time::Instant;

use super::App;
use crate::backend::Command;
use crate::backend::export::{
    Destination, ExportFailure, ExportId, ExportJob, ExportRows, ExportState, Opener,
};
use crate::export::Format;
use crate::export_sheet::{
    ColumnPick, Count, DestinationChoice, ExportChange, ExportSheet, ExportStatus, Exporting,
    NotAgain, ResultOf, RowsChoice,
};
use crate::history::{Outcome, Source};
use crate::i18n::gettext;
use crate::model::{ConnTabId, Dialog, ObjectTab, SqlTab, Workspace};

/// What the sheet shows that the app works out: the counts, the
/// statement, the notes. The view draws it with the sheet.
#[derive(Debug, Clone, PartialEq)]
pub struct ExportFacts {
    /// "reviews".
    pub table: String,
    /// "Bookshop".
    pub connection: String,
    pub environment: crate::env::Environment,
    /// "public.reviews", as the dialect writes it.
    pub qualified: String,
    /// The connection opens read-only.
    pub read_only: bool,
    /// The filters set, and the sort as the subtitle words it
    /// ("created_at ↓").
    pub filters: usize,
    pub sort: Option<String>,
    /// The filter as the Matching row's second line says it.
    pub filter_text: Option<String>,
    /// The rows selected (with a range), and the page's first and last.
    pub selected: usize,
    pub page: (u64, u64),
    pub matching: Option<Count>,
    pub whole: Option<Count>,
    /// A count runs for Count exactly.
    pub counting: bool,
    /// The tab's unsaved changes.
    pub pending: usize,
    /// The statement the choice runs: `None` for loaded rows (D7).
    pub statement: Option<String>,
    /// How many rows the choice writes, where it is known.
    pub rows: Option<Count>,
    /// The clipboard is refused for the choice: more than it takes.
    pub clipboard_refused: bool,
    /// The connection's dialect, for SQL INSERT's line.
    pub dialect: tabletist_db::Dialect,
    /// A SQL result's sheet: what it says of the result. `None` for a
    /// table.
    pub result: Option<ResultFacts>,
}

/// What a SQL result's sheet shows of it (spec 4).
#[derive(Debug, Clone, PartialEq)]
pub struct ResultFacts {
    /// The editor's number ("Query 1"), and the statement's among the
    /// run's, from 1.
    pub query: u32,
    pub statement: usize,
    /// The rows the editor loaded, and whether its limit cut them.
    pub loaded: u64,
    pub cut: bool,
    /// Run again without the limit, or why it is greyed (spec 4, F3).
    pub again: Result<(), NotAgain>,
}

/// A file name for `table` exported as `format`: what a name may not
/// hold on any system (a separator, a colon, a control) is `_`.
fn file_name(table: &str, format: Format) -> String {
    let safe: String = table
        .chars()
        .map(|char| match char {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            char if char.is_control() => '_',
            char => char,
        })
        .collect();
    let safe = safe.trim_start_matches('.');
    let safe = if safe.is_empty() { "export" } else { safe };
    format!("{safe}.{}", format.extension())
}

impl App {
    /// The table in front of `tab`, with its connection.
    fn export_table(&self, tab: ConnTabId) -> Option<(&Workspace, &ObjectTab)> {
        let workspace = self.workspace(tab)?;
        Some((workspace, workspace.active_object_tab()?))
    }

    /// The SQL editor in front of `tab` whose result can be exported
    /// ([`SqlTab::exports`]), with its connection.
    fn export_result(&self, tab: ConnTabId) -> Option<(&Workspace, &SqlTab)> {
        let workspace = self.workspace(tab)?;
        let sql = workspace.active_sql_tab().filter(|sql| sql.exports())?;
        Some((workspace, sql))
    }

    /// The sheet's columns: the page's, or the structure's before a page.
    fn export_columns(object: &ObjectTab) -> Vec<ColumnPick> {
        let pick = |(at, (name, type_name)): (usize, (&str, &str))| ColumnPick {
            name: name.to_owned(),
            type_name: type_name.to_owned(),
            included: true,
            at,
        };
        if let Some(page) = object.page() {
            return page
                .columns
                .iter()
                .map(|column| (column.name.as_str(), column.type_name.as_str()))
                .enumerate()
                .map(pick)
                .collect();
        }
        object
            .structure
            .value
            .as_ref()
            .map(|structure| {
                structure
                    .columns
                    .iter()
                    .map(|column| (column.name.as_str(), column.type_name.as_str()))
                    .enumerate()
                    .map(pick)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// A result's columns, as the editor shows them.
    fn result_columns(sql: &SqlTab) -> Vec<ColumnPick> {
        let Some((columns, _, _)) = sql.shown_rows() else {
            return Vec::new();
        };
        columns
            .iter()
            .enumerate()
            .map(|(at, column)| ColumnPick {
                name: column.name.clone(),
                type_name: column.type_name.clone(),
                included: true,
                at,
            })
            .collect()
    }

    /// The rows of a result a sheet's Selected rows takes, in the grid's
    /// order: the range's, the cursor's where no range stands and
    /// `cursor_row` asks, or the cursor's alone for Export row….
    fn result_selected(sql: &SqlTab, cursor_row: bool, row_only: bool) -> Vec<usize> {
        let Some(selected) = sql.selected() else {
            return Vec::new();
        };
        if row_only {
            return vec![selected.cursor.row];
        }
        if sql.range.is_none() && !cursor_row {
            return Vec::new();
        }
        selected.rows()
    }

    /// Whether Run again without the limit is offered for `sql`'s shown
    /// result, or why not (spec 4, F2 and F3): Show all's rule.
    fn run_again(workspace: &Workspace, sql: &SqlTab) -> Result<(), NotAgain> {
        let Some(run) = sql.last_run() else {
            return Err(NotAgain::Edited);
        };
        if run.mode != tabletist_db::ScriptMode::ReadOnly {
            return Err(NotAgain::ChangedData);
        }
        if !workspace.takes_commands() {
            return Err(NotAgain::Disconnected);
        }
        if !sql.ran_this_text() || !run.ran_on(&workspace.spec) {
            return Err(NotAgain::Edited);
        }
        match sql.cut_statement(&workspace.spec) {
            Some(_) => Ok(()),
            None => Err(NotAgain::AllLoaded),
        }
    }

    /// The rows a range selects, saved ones only (a new row is not in the
    /// database yet), in the grid's order. With no range, the cursor's row
    /// where `cursor_row` asks for it (Export selection…), else none.
    fn export_selected(object: &ObjectTab, cursor_row: bool, row_only: bool) -> Vec<usize> {
        let height = object.page().map_or(0, |page| page.rows.len());
        if row_only {
            return object
                .selection
                .map(|cell| cell.row)
                .filter(|row| crate::edit::new_id(*row).is_none() && *row < height)
                .into_iter()
                .collect();
        }
        if object.range.is_none() && !cursor_row {
            return Vec::new();
        }
        object
            .selected_rows()
            .into_iter()
            .filter(|row| crate::edit::new_id(*row).is_none() && *row < height)
            .collect()
    }

    /// Where `job` is among the exports; with none named, the one whose
    /// end the terminal look's line shows ([`App::shown_export`]).
    fn export_at(&self, job: Option<ExportId>) -> Option<usize> {
        match job {
            Some(job) => self
                .exports
                .iter()
                .position(|exporting| exporting.job.id == job),
            None => self.shown_end(),
        }
    }

    /// The export whose end the terminal look's line shows: of those whose
    /// end is still told, the one that ended last (spec 4, S14).
    fn shown_end(&self) -> Option<usize> {
        let now = Instant::now();
        self.exports
            .iter()
            .enumerate()
            .filter(|(_, exporting)| !exporting.gone(now))
            .filter_map(|(at, exporting)| Some((at, exporting.ended_at()?)))
            .max_by_key(|(at, ended)| (*ended, *at))
            .map(|(at, _)| at)
    }

    /// The export whose end the terminal look's line shows, if any: `go`,
    /// Esc and `:export` act on it.
    pub fn shown_export(&self) -> Option<&Exporting> {
        self.shown_end().map(|at| &self.exports[at])
    }

    /// The exports running, in the order they started.
    pub fn running_exports(&self) -> impl Iterator<Item = &Exporting> {
        self.exports.iter().filter(|exporting| exporting.running())
    }

    /// A copy over the clipboard's limit offers the file (spec 3b, R6):
    /// the toast's Export… on the selection in the copy's format, or in
    /// the terminal look `E` on its line; of a table's grid or a SQL
    /// result's (spec 4). Returns whether it offered.
    pub(super) fn offer_export(
        &mut self,
        tab: ConnTabId,
        pick: crate::model::CopyPick,
        how: crate::model::CopyHow,
    ) -> bool {
        // The row panel's row has Export row… of its own. Copy rows as
        // TSV's whole rows export every column: the block's range is the
        // rows' too.
        if pick == crate::model::CopyPick::CursorRow {
            return false;
        }
        let Some(workspace) = self.workspace(tab) else {
            return false;
        };
        let copied = match (workspace.active_object_tab(), workspace.active_sql_tab()) {
            (Some(object), _) => object.selection.map(|cursor| {
                let copied = crate::model::CopiedSelection {
                    page: object.rows.loaded,
                    cursor,
                    range: object.range.clone(),
                };
                (object.id, copied)
            }),
            (None, Some(sql)) if sql.exports() => sql.selection.map(|cursor| {
                let copied = crate::model::CopiedSelection {
                    page: sql.run.loaded,
                    cursor,
                    range: sql.range.clone(),
                };
                (sql.id, copied)
            }),
            _ => None,
        };
        let Some((table, copied)) = copied else {
            return false;
        };
        let format = match how {
            crate::model::CopyHow::As(format) => format,
            crate::model::CopyHow::Plain | crate::model::CopyHow::ColumnNames => Format::Tsv,
        };
        self.offer = Some(crate::model::ExportOffer {
            tab,
            table: Some(table),
            format: Some(format),
            whole_rows: pick == crate::model::CopyPick::Rows,
            copied: Some(copied),
            row_only: false,
        });
        if self.look.terminal {
            let key = self.keymap.key_in(
                self.layout(),
                crate::keymap::Command::Export,
                crate::keymap::Scope::Grid,
            );
            if let Some(said) = &mut self.said {
                let export = gettext(self.locale, "export");
                *said = format!("{said} · {key} {export}");
            }
        }
        true
    }

    /// Whether `offer` still stands: the table (or the SQL editor) it was
    /// of is open, on the page (the run) the copy was of. Another page's
    /// rows are other rows.
    pub fn offer_holds(&self, offer: &crate::model::ExportOffer) -> bool {
        let Some(id) = offer.table else {
            return true;
        };
        let Some(workspace) = self.workspace(offer.tab) else {
            return false;
        };
        let loaded = match (workspace.object_tab(id), workspace.sql_tab(id)) {
            (Some(object), _) => object.rows.loaded,
            (None, Some(sql)) => sql.run.loaded,
            (None, None) => return false,
        };
        offer
            .copied
            .as_ref()
            .is_none_or(|copied| loaded == copied.page)
    }

    /// Opens the sheet over the table in front of `tab`.
    pub(super) fn open_export(&mut self, tab: ConnTabId, format: Option<Format>) {
        self.open_export_on(tab, format, (false, false, false));
    }

    /// Export selection…, and Copy as's Export… (`format` given): the
    /// sheet on the selection, the cursor's row where no range stands; a
    /// format given goes to a file, as the clipboard could not take it
    /// (spec 3b, R1 and R6).
    pub(super) fn export_selection(&mut self, offer: crate::model::ExportOffer) {
        // Nothing moves where no sheet would open.
        if self.dialog.is_some() || !self.offer_holds(&offer) {
            return;
        }
        let crate::model::ExportOffer {
            tab,
            table,
            format,
            whole_rows,
            copied,
            row_only,
        } = offer;
        // The table (or the editor) copied, though another came to the
        // front since.
        if let Some(id) = table {
            let front = self
                .workspace(tab)
                .and_then(|workspace| workspace.active_tab);
            match self.workspace(tab).and_then(|workspace| workspace.tab(id)) {
                Some(_) if front != Some(id) => {
                    self.apply(crate::model::Action::ActivateTab { tab, id });
                }
                Some(_) => {}
                None => return,
            }
            // What was copied, though the selection moved since.
            if let Some(copied) = copied {
                if let Some(object) = self.object_tab_mut(tab, id) {
                    object.set_cursor(Some(copied.cursor), crate::model::Reach::End);
                    object.range = copied.range;
                } else if let Some(sql) = self.sql_tab_mut(tab, id) {
                    sql.set_cursor(Some(copied.cursor), crate::model::Reach::End);
                    sql.range = copied.range;
                }
            }
        }
        self.open_export_on(tab, format, (true, whole_rows, row_only));
        if format.is_some()
            && let Some(Dialog::Export(sheet)) = &mut self.dialog
        {
            sheet.destination = DestinationChoice::File;
        }
    }

    /// Opens the sheet over the table (or the SQL result) in front of
    /// `tab`; on its selection alone (the cursor's row with no range)
    /// where `selection` asks for it, or on the cursor's row alone where
    /// `row_only` does (Export row…). A block of cells opens Choose with
    /// its columns, unless `whole_rows` asks for every column (spec 3b,
    /// R1).
    fn open_export_on(
        &mut self,
        tab: ConnTabId,
        format: Option<Format>,
        (selection, whole_rows, row_only): (bool, bool, bool),
    ) {
        if self.dialog.is_some() {
            return;
        }
        let asked = self.next_id();
        // The format of the last export, or CSV.
        let format = format.or(self.export_format).unwrap_or(Format::Csv);
        if self.export_table(tab).is_none() {
            return self.open_result_export(tab, asked, format, (selection, whole_rows, row_only));
        }
        let Some((_, object)) = self.export_table(tab) else {
            return;
        };
        let mut columns = Self::export_columns(object);
        let block = match object.range {
            Some(crate::selection::Range::Cells { .. }) if !whole_rows && !row_only => {
                object.selected_cols()
            }
            _ => None,
        };
        let choose = block.is_some_and(|(first, last)| first > 0 || last + 1 < columns.len());
        if choose && let Some(block) = block {
            columns = crate::export_sheet::block_columns(columns, block);
        }
        let selected = Self::export_selected(object, selection, row_only).len();
        // Export selection… on new rows alone: they are not in the
        // database yet, and all of the table is not what was asked for.
        if selection && selected == 0 {
            let save = self.keymap.key_in(
                self.layout(),
                crate::keymap::Command::SaveChanges,
                crate::keymap::Scope::Grid,
            );
            let words = match self.look.terminal {
                true => gettext(
                    self.locale,
                    "new rows aren't exported · :w first to include them",
                ),
                false => gettext(
                    self.locale,
                    "New rows aren't exported. Export reads what is saved in the database. Save first {key}",
                ),
            };
            self.said = Some(words.replace("{key}", save.as_str()));
            return;
        }
        let rows = crate::export_sheet::first_rows(selected, false);
        let destination = match self.export_clipboard.contains(&format) {
            true => DestinationChoice::Clipboard,
            false => DestinationChoice::File,
        };
        let suggested = self
            .dirs
            .downloads
            .join(file_name(&object.object.name, format));
        let path = crate::export_sheet::shown_path(&suggested, self.dirs.home.as_deref());
        let id = object.id;
        let sheet = ExportSheet {
            tab,
            id,
            result: None,
            asked,
            format,
            options: self.export.clone(),
            rows,
            choose,
            columns,
            column: 0,
            destination,
            path,
            exists: false,
            review: false,
            asking: false,
            cursor_row: selection,
            row_only,
            listed: None,
            picking: false,
            listing: false,
            list_closed: false,
            picked: None,
            to_line: 0,
        };
        self.open_sheet(sheet);
    }

    /// The sheet over the SQL result in front of `tab` (spec 4): on its
    /// selection as a table's is, else on Rows already loaded.
    fn open_result_export(
        &mut self,
        tab: ConnTabId,
        asked: u64,
        format: Format,
        (selection, whole_rows, row_only): (bool, bool, bool),
    ) {
        let Some((_, sql)) = self.export_result(tab) else {
            return;
        };
        let Some((index, _)) = sql.shown() else {
            return;
        };
        let mut columns = Self::result_columns(sql);
        let block = match (&sql.range, sql.selected()) {
            (Some(crate::selection::Range::Cells { .. }), Some(selected))
                if !whole_rows && !row_only =>
            {
                selected.cols(columns.len())
            }
            _ => None,
        };
        let choose = block.is_some_and(|(first, last)| first > 0 || last + 1 < columns.len());
        if choose && let Some(block) = block {
            columns = crate::export_sheet::block_columns(columns, block);
        }
        let selected = Self::result_selected(sql, selection, row_only).len();
        let rows = crate::export_sheet::first_rows(selected, true);
        let destination = match self.export_clipboard.contains(&format) {
            true => DestinationChoice::Clipboard,
            false => DestinationChoice::File,
        };
        let stem = crate::export_sheet::result_file_stem(sql.number);
        let suggested = self.dirs.downloads.join(file_name(&stem, format));
        let path = crate::export_sheet::shown_path(&suggested, self.dirs.home.as_deref());
        let sheet = ExportSheet {
            tab,
            id: sql.id,
            result: Some(ResultOf {
                statement: index,
                query: sql.number,
                run: sql.run.loaded,
            }),
            asked,
            format,
            options: self.export.clone(),
            rows,
            choose,
            columns,
            column: 0,
            destination,
            path,
            exists: false,
            review: false,
            asking: false,
            cursor_row: selection,
            row_only,
            listed: None,
            picking: false,
            listing: false,
            list_closed: false,
            picked: None,
            to_line: 0,
        };
        self.open_sheet(sheet);
    }

    /// Opens `sheet`, its destination the file where the clipboard would
    /// refuse its rows.
    fn open_sheet(&mut self, mut sheet: ExportSheet) {
        // Clipboard only where the rows fit.
        if sheet.destination == DestinationChoice::Clipboard
            && self
                .export_facts_of(&sheet)
                .is_some_and(|facts| facts.clipboard_refused)
        {
            sheet.destination = DestinationChoice::File;
        }
        self.dialog = Some(Dialog::Export(Box::new(sheet)));
    }

    /// The terminal look's `E` on the tree: the table under the cursor
    /// opens, as Enter opens it, then its float (spec, Q18).
    pub(super) fn export_from_tree(&mut self, tab: ConnTabId) {
        // A table or a view under the tree's cursor; anything else (a
        // schema, a folder) has nothing to export.
        let under = self
            .workspace(tab)
            .and_then(|workspace| match &workspace.tree.cursor {
                Some(crate::model::TreeNode::Object(object, kind)) => Some((object.clone(), *kind)),
                _ => None,
            });
        let Some((object, kind)) = under else {
            return;
        };
        self.export_object(tab, object, kind);
    }

    /// Opens `object` as Enter opens it, then its sheet over it: the tree's
    /// `E` and menu, quick open's picked table.
    pub(super) fn export_object(
        &mut self,
        tab: ConnTabId,
        object: tabletist_db::ObjectRef,
        kind: tabletist_db::ObjectKind,
    ) {
        if self.dialog.is_some() {
            return;
        }
        self.open_object(tab, object, kind, true);
        // Its columns come with its first page: the sheet takes them then
        // (`tick_export`).
        self.open_export(tab, None);
    }

    /// A row of the sidebar's menu on a table or view (spec 3b, R5).
    pub(super) fn tree_pick(
        &mut self,
        tab: ConnTabId,
        object: tabletist_db::ObjectRef,
        kind: tabletist_db::ObjectKind,
        pick: crate::model::TreePick,
    ) {
        use crate::model::{ObjectView, TreePick};
        let view = match pick {
            TreePick::Export => return self.export_object(tab, object, kind),
            TreePick::CopyName => {
                let said = gettext(self.locale, "Copied {name}").replace("{name}", &object.name);
                self.said = Some(self.look.label(&said));
                self.to_clipboard = Some(object.name);
                return;
            }
            TreePick::Open => None,
            TreePick::Structure => Some(ObjectView::Structure),
            TreePick::Ddl => Some(ObjectView::Ddl),
        };
        self.open_object(tab, object, kind, true);
        let front = self
            .workspace(tab)
            .and_then(Workspace::active_object_tab)
            .map(|object| object.id);
        if let (Some(view), Some(object_tab)) = (view, front) {
            self.apply(crate::model::Action::SetView {
                tab,
                object_tab,
                view,
            });
        }
    }

    pub(super) fn change_export(&mut self, change: ExportChange) {
        let locale = self.locale;
        let Some(Dialog::Export(sheet)) = &mut self.dialog else {
            return;
        };
        let typed = matches!(change, ExportChange::Path(_));
        if let Some(said) = sheet.change(change, locale) {
            self.export_said = Some(said);
        }

        // Over the clipboard's rows: the file, whatever was chosen.
        let sheet = sheet.clone();
        if sheet.destination == DestinationChoice::Clipboard
            && self
                .export_facts_of(&sheet)
                .is_some_and(|facts| facts.clipboard_refused)
            && let Some(Dialog::Export(open)) = &mut self.dialog
        {
            open.destination = DestinationChoice::File;
        }
        // Typed in the path line: the names it starts (spec 4, S15).
        if typed {
            self.list_folder();
        }
    }

    /// Asks for the names the float's path line starts, off this thread
    /// (spec 4, S15): none for an empty line, nor in the macOS look, whose
    /// sheet has no path line.
    fn list_folder(&mut self) {
        let Some(Dialog::Export(sheet)) = &self.dialog else {
            return;
        };
        if !self.look.terminal {
            return;
        }
        let asked = crate::export_sheet::completion_ask(
            &sheet.path,
            &self.dirs.downloads,
            self.dirs.home.as_deref(),
        );
        let Some((folder, start)) = asked else {
            return;
        };
        // One listing at a time: the one on its way is followed by
        // another for the line as it is then (`folder_listed`).
        if sheet.listing {
            return;
        }
        let command = Command::ListFolder {
            asked: sheet.asked,
            typed: sheet.path.clone(),
            folder,
            start,
        };
        self.backend.send(command);
        if let Some(Dialog::Export(sheet)) = &mut self.dialog {
            sheet.listing = true;
        }
    }

    /// The names a path line starts: kept for the line they were asked
    /// for, dropped for one since changed or another sheet's.
    pub(super) fn folder_listed(
        &mut self,
        asked: u64,
        typed: &str,
        names: Vec<crate::export_sheet::Listed>,
    ) {
        let Some(Dialog::Export(sheet)) = &mut self.dialog else {
            return;
        };
        if sheet.asked != asked {
            return;
        }
        sheet.listing = false;
        // The line changed while it was read: the line as it is now.
        if sheet.path != typed {
            self.list_folder();
            return;
        }
        // Nor while `ctrl+o`'s dialog is up, its answer to fill the line,
        // nor once the list was put away for this line.
        if sheet.asking || sheet.list_closed {
            return;
        }
        // Nothing matches: no list (spec 4, S15).
        sheet.listed = (!names.is_empty()).then(|| crate::export_sheet::PathList {
            line: typed.to_owned(),
            names,
            lit: 0,
        });
    }

    /// A name of the path line's list taken: the one at `at`, else the
    /// lit one. A folder lists its own names next; a file closes the list
    /// and sets the format from its ending (spec 4, S15).
    pub(super) fn pick_path(&mut self, at: Option<usize>) {
        let locale = self.locale;
        let Some(Dialog::Export(sheet)) = &mut self.dialog else {
            return;
        };
        let picked = match at {
            Some(at) => sheet
                .listed
                .as_ref()
                .filter(|list| list.line == sheet.path)
                .and_then(|list| list.names.get(at)),
            None => sheet.lit_name(),
        };
        let Some(picked) = picked.cloned() else {
            return;
        };
        let line = crate::export_sheet::complete(&sheet.path, &picked);
        sheet.change(ExportChange::Path(line), locale);
        sheet.listed = None;
        // Typing goes on at the line's end, not where the part taken was.
        sheet.to_line += 1;
        if picked.folder {
            self.list_folder();
        }
    }

    /// The float's `ctrl+o`: the system's save dialog (the portal's on
    /// Linux), suggested from the path line. Its answer fills the line;
    /// nothing starts until Enter (spec 4, S16).
    pub(super) fn pick_export_file(&mut self) {
        let Some(Dialog::Export(sheet)) = &self.dialog else {
            return;
        };
        if sheet.asking || sheet.review || sheet.destination != DestinationChoice::File {
            return;
        }
        let suggested = crate::export_sheet::typed_path(
            &sheet.path,
            &self.dirs.downloads,
            self.dirs.home.as_deref(),
        );
        let title = gettext(self.locale, "Export rows").into_owned();
        self.backend
            .ask_export_path(ExportId(sheet.asked), &title, &suggested);
        if let Some(Dialog::Export(sheet)) = &mut self.dialog {
            sheet.asking = true;
            sheet.picking = true;
            sheet.listed = None;
        }
    }

    /// Count exactly: the tab's own count, as the footer's.
    pub(super) fn count_for_export(&mut self) {
        let Some(Dialog::Export(sheet)) = &self.dialog else {
            return;
        };
        // A result's rows are counted as they are loaded, or as they run.
        if sheet.result.is_some() {
            return;
        }
        let (tab, id) = (sheet.tab, sheet.id);
        self.count_rows(tab, id);
    }

    /// What the sheet shows of the table now.
    pub fn export_facts(&self) -> Option<ExportFacts> {
        match &self.dialog {
            Some(Dialog::Export(sheet)) => self.export_facts_of(sheet),
            _ => None,
        }
    }

    fn export_facts_of(&self, sheet: &ExportSheet) -> Option<ExportFacts> {
        let workspace = self.workspace(sheet.tab)?;
        if let Some(result) = sheet.result {
            return Self::result_facts(workspace, sheet, result);
        }
        let object = workspace.object_tab(sheet.id)?;
        // Nothing of it without SQL (Redis).
        let dialect = workspace.driver.dialect()?;
        let query = &object.query;
        let filtered = !query.filters.is_empty() || query.raw_where.is_some();
        let counted = object.count.value.map(|rows| Count { rows, exact: true });
        let estimate = object
            .estimated_rows
            .map(|rows| Count { rows, exact: false });
        let matching = match filtered {
            true => counted,
            false => counted.or(estimate),
        };
        let whole = match filtered {
            true => estimate,
            false => counted.or(estimate),
        };
        let page_rows = object.page().map_or(0, |page| page.rows.len()) as u64;
        let first = query.offset + u64::from(page_rows > 0);
        let selected = Self::export_selected(object, sheet.cursor_row, sheet.row_only).len();
        let rows = match sheet.rows {
            RowsChoice::Selected => Some(Count {
                rows: selected as u64,
                exact: true,
            }),
            RowsChoice::Page => Some(Count {
                rows: page_rows,
                exact: true,
            }),
            RowsChoice::Matching => matching,
            RowsChoice::Whole => whole,
            // A result's: not on a table's sheet.
            RowsChoice::Loaded | RowsChoice::RunAgain => None,
        };
        let sort = query.sort.first().map(|sort| {
            let arrow = match sort.dir {
                tabletist_db::SortDir::Asc => "↑",
                tabletist_db::SortDir::Desc => "↓",
            };
            format!("{} {arrow}", sort.column)
        });
        let filter_text = match (query.filters.as_slice(), &query.raw_where) {
            ([], None) => None,
            (_, Some(raw)) => Some(raw.split_whitespace().collect::<Vec<_>>().join(" ")),
            (filters, None) => Some(
                filters
                    .iter()
                    .map(|filter| {
                        format!("{} {} {}", filter.column, filter.op.label(), filter.value)
                    })
                    .collect::<Vec<_>>()
                    .join(" · "),
            ),
        };
        let counts = object.edits.counts();
        let statement = self.export_statement(sheet, object, dialect);
        Some(ExportFacts {
            table: object.object.name.clone(),
            connection: workspace.name.clone(),
            environment: workspace.environment,
            qualified: format!("{}.{}", object.object.schema, object.object.name),
            read_only: workspace.access == tabletist_db::Access::ReadOnly,
            filters: query.filters.len() + usize::from(query.raw_where.is_some()),
            sort,
            filter_text,
            selected,
            page: (first, query.offset + page_rows),
            matching,
            whole,
            counting: object.count.pending.is_some(),
            pending: counts.changes + counts.added + counts.deleted,
            statement,
            rows,
            clipboard_refused: rows
                .is_some_and(|rows| rows.rows > crate::export::CLIPBOARD_ROWS as u64),
            dialect,
            result: None,
        })
    }

    /// What a SQL result's sheet shows: `None` once the editor shows
    /// another statement's rows (it ran again), which closes the sheet.
    fn result_facts(
        workspace: &Workspace,
        sheet: &ExportSheet,
        result: ResultOf,
    ) -> Option<ExportFacts> {
        let dialect = workspace.driver.dialect()?;
        let sql = workspace.sql_tab(sheet.id)?;
        let (index, _) = sql.shown()?;
        // Another statement's rows, or another run's (one in flight will
        // replace them): the sheet goes.
        if index != result.statement || sql.is_running() || sql.run.loaded != result.run {
            return None;
        }
        let (_, loaded, cut) = sql.shown_rows()?;
        let loaded = loaded.len() as u64;
        let again = Self::run_again(workspace, sql);
        let selected = Self::result_selected(sql, sheet.cursor_row, sheet.row_only).len();
        let exact = |rows: u64| Some(Count { rows, exact: true });
        let rows = match sheet.rows {
            RowsChoice::Selected => exact(selected as u64),
            RowsChoice::Loaded => exact(loaded),
            // Counted as it runs (spec 4, S5).
            _ => None,
        };
        let statement = match (sheet.rows, again) {
            (RowsChoice::RunAgain, Ok(())) => sql
                .cut_statement(&workspace.spec)
                .map(|statement| statement.text.clone()),
            _ => None,
        };
        Some(ExportFacts {
            table: "result".to_owned(),
            connection: workspace.name.clone(),
            environment: workspace.environment,
            qualified: "result".to_owned(),
            read_only: workspace.access == tabletist_db::Access::ReadOnly,
            filters: 0,
            sort: None,
            filter_text: None,
            selected,
            page: (u64::from(loaded > 0), loaded),
            matching: None,
            whole: None,
            counting: false,
            pending: 0,
            statement,
            rows,
            clipboard_refused: rows
                .is_some_and(|rows| rows.rows > crate::export::CLIPBOARD_ROWS as u64),
            dialect,
            result: Some(ResultFacts {
                query: result.query,
                statement: result.statement + 1,
                loaded,
                cut,
                again,
            }),
        })
    }

    /// The query a choice reads: the tab's, from its first row, or the
    /// whole table's.
    fn export_query(sheet: &ExportSheet, object: &ObjectTab) -> tabletist_db::RowQuery {
        match sheet.rows {
            RowsChoice::Whole => tabletist_db::RowQuery::new(object.object.clone(), 0),
            _ => {
                let mut query = object.query.clone();
                query.offset = 0;
                query
            }
        }
    }

    /// The statement the sheet's choice runs, with its values written in.
    fn export_statement(
        &self,
        sheet: &ExportSheet,
        object: &ObjectTab,
        dialect: tabletist_db::Dialect,
    ) -> Option<String> {
        if sheet.rows.loaded() {
            return None;
        }
        let key = object
            .structure
            .value
            .as_ref()
            .map(|structure| structure.primary_key.clone())
            .unwrap_or_default();
        let names: Vec<String> = sheet
            .chosen()
            .iter()
            .map(|column| column.name.clone())
            .collect();
        let query = Self::export_query(sheet, object);
        // The classes the streamed statement is built with, so what is
        // reviewed is what runs (BigQuery writes a number or a boolean
        // bare). None where a filter's value is none of its column's
        // class: the run names it then.
        let classes = object
            .structure
            .value
            .as_ref()
            .map(|structure| structure.classes(dialect))
            .unwrap_or_default();
        dialect.shown_export(&query, &key, &classes, &names).ok()
    }

    /// The columns agents see masked on `workspace`'s connection: the
    /// person's and the administrator's.
    fn export_masked(&self, workspace: &Workspace) -> Vec<tabletist_db::agent::ColumnName> {
        let mut masked = self
            .connections
            .get(&workspace.conn_id)
            .map(|saved| self.agents_masked(saved))
            .unwrap_or_default();
        for source in self
            .managed
            .sources(&workspace.spec, self.dirs.home.as_deref())
        {
            masked.extend(source.masked);
        }
        masked
    }

    /// The export the sheet asks for, to `destination`.
    fn export_job(&self, sheet: &ExportSheet, destination: Destination) -> Option<Exporting> {
        let workspace = self.workspace(sheet.tab)?;
        if let Some(result) = sheet.result {
            return self.result_job(workspace, sheet, result, destination);
        }
        let object = workspace.object_tab(sheet.id)?;
        // Nothing of it without SQL (Redis).
        let dialect = workspace.driver.dialect()?;
        let chosen = sheet.chosen();
        if chosen.is_empty() {
            return None;
        }
        let names: Vec<String> = chosen.iter().map(|column| column.name.clone()).collect();
        let rows = match sheet.rows {
            RowsChoice::Matching | RowsChoice::Whole => {
                ExportRows::Query(Self::export_query(sheet, object))
            }
            // A result's: not on a table's sheet.
            RowsChoice::Loaded | RowsChoice::RunAgain => return None,
            RowsChoice::Selected | RowsChoice::Page => {
                let page = object.page()?;
                let at: Vec<usize> = names
                    .iter()
                    .filter_map(|name| page.columns.iter().position(|column| column.name == *name))
                    .collect();
                let picked: Vec<usize> = match sheet.rows {
                    RowsChoice::Selected => {
                        Self::export_selected(object, sheet.cursor_row, sheet.row_only)
                    }
                    _ => (0..page.rows.len()).collect(),
                };
                ExportRows::Loaded {
                    columns: at.iter().map(|&col| page.columns[col].clone()).collect(),
                    rows: picked
                        .iter()
                        .filter_map(|&row| page.rows.get(row))
                        .map(|row| at.iter().map(|&col| row[col].clone()).collect())
                        .collect(),
                }
            }
        };
        let name_refs: Vec<&str> = names.iter().map(String::as_str).collect();
        let masked = crate::export::masks(
            crate::export::follows_masking(),
            &self.export_masked(workspace),
            &object.object,
            &name_refs,
        );
        let job = ExportJob {
            id: ExportId(sheet.asked),
            open: self.export_opener(workspace),
            rows,
            columns: names,
            masked,
            format: sheet.format,
            options: sheet.options.clone(),
            object: Some(object.object.clone()),
            dialect,
            destination: destination.clone(),
        };
        let facts = self.export_facts_of(sheet)?;
        let (file, shown_path) = self.export_file(&destination);
        Some(Exporting {
            job,
            tab: sheet.tab,
            table: object.object.name.clone(),
            file,
            shown_path,
            connection: self.history_name(sheet.tab),
            total: facts.rows,
            statement: facts.statement.clone(),
            sheet: Box::new(sheet.kept()),
            status: ExportStatus::Running { rows: 0, bytes: 0 },
            hidden: false,
            started: Instant::now(),
            announced: None,
            reopens: false,
            history: facts.statement.is_some().then(|| {
                super::history::entry_on(workspace, Source::Export, object.object.name.clone())
            }),
        })
    }

    /// How the export's own connection opens: the tab's details, and the
    /// secrets it connected with.
    fn export_opener(&self, workspace: &Workspace) -> Opener {
        // A blank answer means "no password" to the server, as at connect.
        let mut secrets = workspace.secrets.clone();
        for slot in [
            &mut secrets.password,
            &mut secrets.ssh_password,
            &mut secrets.ssh_passphrase,
        ] {
            if slot.as_deref() == Some("") {
                *slot = None;
            }
        }
        Opener {
            spec: workspace.spec.clone(),
            secrets,
            host_keys: self.host_keys.clone(),
        }
    }

    /// The file's name and its path as the person reads it; empty for the
    /// clipboard.
    fn export_file(&self, destination: &Destination) -> (String, String) {
        match destination {
            Destination::File { path, .. } => (
                path.file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                crate::export_sheet::shown_path(path, self.dirs.home.as_deref()),
            ),
            Destination::Clipboard => (String::new(), String::new()),
        }
    }

    /// A SQL result's export (spec 4): its loaded rows (nothing runs), or
    /// its statement run again, which the history records.
    fn result_job(
        &self,
        workspace: &Workspace,
        sheet: &ExportSheet,
        result: ResultOf,
        destination: Destination,
    ) -> Option<Exporting> {
        let sql = workspace.sql_tab(sheet.id)?;
        let facts = Self::result_facts(workspace, sheet, result)?;
        let chosen = sheet.chosen();
        if chosen.is_empty() {
            return None;
        }
        let names: Vec<String> = chosen.iter().map(|column| column.name.clone()).collect();
        let at: Vec<usize> = chosen.iter().map(|column| column.at).collect();
        let rows = match sheet.rows {
            RowsChoice::RunAgain => {
                let statement = sql.cut_statement(&workspace.spec)?.clone();
                // Every column as it comes needs no picking.
                let whole = at.iter().copied().eq(0..sheet.columns.len());
                ExportRows::Statement {
                    statement,
                    pick: (!whole).then_some(at),
                }
            }
            RowsChoice::Selected | RowsChoice::Loaded => {
                let (columns, loaded, _) = sql.shown_rows()?;
                let picked: Vec<usize> = match sheet.rows {
                    RowsChoice::Selected => {
                        Self::result_selected(sql, sheet.cursor_row, sheet.row_only)
                    }
                    _ => (0..loaded.len()).collect(),
                };
                ExportRows::Loaded {
                    columns: at
                        .iter()
                        .filter_map(|&col| columns.get(col).cloned())
                        .collect(),
                    rows: picked
                        .iter()
                        .filter_map(|&row| loaded.get(row))
                        .map(|row| at.iter().filter_map(|&col| row.get(col).cloned()).collect())
                        .collect(),
                }
            }
            // A table's: not on a result's sheet.
            RowsChoice::Page | RowsChoice::Matching | RowsChoice::Whole => return None,
        };
        let job = ExportJob {
            id: ExportId(sheet.asked),
            open: self.export_opener(workspace),
            rows,
            masked: vec![false; names.len()],
            columns: names,
            format: sheet.format,
            options: sheet.options.clone(),
            // A result's columns name no table (spec 4, F9 and S4).
            object: None,
            dialect: workspace.driver.dialect()?,
            destination: destination.clone(),
        };
        let (file, shown_path) = self.export_file(&destination);
        let query = gettext(self.locale, "Query {n}").replace("{n}", &result.query.to_string());
        Some(Exporting {
            job,
            tab: sheet.tab,
            table: facts.table.clone(),
            file,
            shown_path,
            connection: self.history_name(sheet.tab),
            total: facts.rows,
            statement: facts.statement.clone(),
            sheet: Box::new(sheet.kept()),
            status: ExportStatus::Running { rows: 0, bytes: 0 },
            hidden: false,
            started: Instant::now(),
            announced: None,
            reopens: false,
            history: facts
                .statement
                .is_some()
                .then(|| super::history::entry_on(workspace, Source::Export, query)),
        })
    }

    /// The sheet's primary button (↩): ask where the file goes, or start.
    pub(super) fn start_export(&mut self) {
        let Some(Dialog::Export(sheet)) = &self.dialog else {
            return;
        };
        if sheet.asking || sheet.chosen().is_empty() {
            return;
        }
        let sheet = sheet.clone();
        let Some(facts) = self.export_facts_of(&sheet) else {
            return;
        };
        let destination = match sheet.destination {
            DestinationChoice::Clipboard if facts.clipboard_refused => return,
            DestinationChoice::Clipboard => Destination::Clipboard,
            // A folder taken from the path line's list names no file yet:
            // Enter waits for one (plan 4b, B20).
            DestinationChoice::File
                if self.look.terminal
                    && sheet
                        .path
                        .trim_end()
                        .ends_with(['/', std::path::MAIN_SEPARATOR]) =>
            {
                return;
            }
            DestinationChoice::File if self.look.terminal => {
                let path = sheet.picked.clone().unwrap_or_else(|| {
                    crate::export_sheet::typed_path(
                        &sheet.path,
                        &self.dirs.downloads,
                        self.dirs.home.as_deref(),
                    )
                });
                // A file that is there fails the rename, and the float
                // opens again saying so (`export_state`): no disk is read
                // here.
                Destination::File {
                    path,
                    replace: sheet.exists,
                }
            }
            DestinationChoice::File => {
                // The system's panel names the file, and asks before one
                // is replaced.
                let stem = match &facts.result {
                    Some(result) => crate::export_sheet::result_file_stem(result.query),
                    None => facts.table.clone(),
                };
                let suggested = self.dirs.downloads.join(file_name(&stem, sheet.format));
                let rows = facts
                    .rows
                    .map(crate::export_sheet::count_words)
                    .map_or_else(String::new, |rows| format!(" {rows}"));
                let title = gettext(self.locale, "Export{rows} rows").replace("{rows}", &rows);
                self.backend
                    .ask_export_path(ExportId(sheet.asked), &title, &suggested);
                if let Some(Dialog::Export(sheet)) = &mut self.dialog {
                    sheet.asking = true;
                }
                return;
            }
        };
        let reopens = self.look.terminal && matches!(destination, Destination::File { .. });
        if self.run_export(&sheet, destination)
            && let Some(exporting) = self.exports.last_mut()
        {
            exporting.reopens = reopens;
        }
    }

    /// Starts `sheet`'s export to `destination`, after any running (spec
    /// 4, S12), and closes the sheet. Returns whether it started.
    fn run_export(&mut self, sheet: &ExportSheet, destination: Destination) -> bool {
        let clipboard = destination == Destination::Clipboard;
        let Some(exporting) = self.export_job(sheet, destination) else {
            return false;
        };
        // Remembered when it starts (spec, Q11), and the format the next
        // sheet opens on.
        self.export_format = Some(sheet.format);
        self.export = sheet.options.clone();
        self.export.chosen(sheet.format);
        self.save_export_options();
        match clipboard {
            true => self.export_clipboard.insert(sheet.format),
            false => self.export_clipboard.remove(&sheet.format),
        };
        if matches!(&self.dialog, Some(Dialog::Export(open)) if open.asked == sheet.asked) {
            self.dialog = None;
        }
        // A failure to write the same file is answered by this one
        // (`:w!` after "exists"): it goes.
        if let Destination::File { path, .. } = &exporting.job.destination {
            self.exports.retain(|other| {
                let failed = matches!(other.status, ExportStatus::Failed { .. });
                !(failed
                    && matches!(&other.job.destination, Destination::File { path: there, .. } if there == path))
            });
        }
        self.backend
            .send(Command::Export(Box::new(exporting.job.clone())));
        self.exports.push(exporting);
        true
    }

    /// The save panel's answer. It replaces only a file that was there as
    /// the panel answered: the panel asked about that one.
    pub(super) fn export_path_answered(
        &mut self,
        job: ExportId,
        path: Option<PathBuf>,
        replace: bool,
    ) {
        let sheet = match &mut self.dialog {
            Some(Dialog::Export(sheet)) if ExportId(sheet.asked) == job => sheet,
            // Choose folder… from a failure's toast: its export again,
            // whatever sheet is open for another (spec 4, F7).
            _ => {
                if let Some(path) = path
                    && let Some(at) = self.export_at(Some(job))
                    && !self.exports[at].running()
                {
                    self.rerun_export(at, Some(Destination::File { path, replace }));
                }
                return;
            }
        };
        sheet.asking = false;
        let Some(path) = path else {
            sheet.picking = false;
            return;
        };
        // `ctrl+o`'s answer: the line, and a replace where the dialog
        // asked for one; Enter starts it (spec 4, S16).
        if std::mem::take(&mut sheet.picking) {
            let line = crate::export_sheet::shown_path(&path, self.dirs.home.as_deref());
            sheet.change(ExportChange::Path(line), self.locale);
            sheet.exists = replace;
            sheet.listed = None;
            // Enter writes the file answered, though the line could not
            // say all of its name; from the line, which has the keyboard.
            sheet.picked = Some(path);
            sheet.to_line += 1;
            return;
        }
        let sheet = sheet.clone();
        self.run_export(&sheet, Destination::File { path, replace });
    }

    /// `:w <path>`, `:w! <path>`: the rows matching the filter, every
    /// column, at once (spec, Q13); of a SQL result, the rows already
    /// loaded (spec 4, S6).
    pub(super) fn export_to(&mut self, tab: ConnTabId, typed: &str, replace: bool) {
        let home = self.dirs.home.clone();
        let path = crate::export_sheet::typed_path(typed, &self.dirs.downloads, home.as_deref());
        let Some(format) = crate::export::Format::of_path(&path) else {
            let extension = match path.extension() {
                Some(extension) => format!(".{}", extension.to_string_lossy()),
                None => path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default(),
            };
            let said = gettext(
                self.locale,
                "✗ no format for {extension} · csv tsv json jsonl sql md",
            )
            .replace("{extension}", &extension);
            self.said = Some(said);
            return;
        };
        // A file that is there fails the rename, off this thread, and its
        // line says `:w!` (spec, Q6).
        let asked = self.next_id();
        if self.export_table(tab).is_none() {
            let Some((_, sql)) = self.export_result(tab) else {
                return;
            };
            let Some((index, _)) = sql.shown() else {
                return;
            };
            let sheet = ExportSheet {
                tab,
                id: sql.id,
                result: Some(ResultOf {
                    statement: index,
                    query: sql.number,
                    run: sql.run.loaded,
                }),
                asked,
                format,
                options: self.export.clone(),
                rows: RowsChoice::Loaded,
                choose: false,
                columns: Self::result_columns(sql),
                column: 0,
                destination: DestinationChoice::File,
                path: typed.to_owned(),
                exists: replace,
                review: false,
                asking: false,
                cursor_row: false,
                row_only: false,
                listed: None,
                picking: false,
                listing: false,
                list_closed: false,
                picked: None,
                to_line: 0,
            };
            self.run_export(&sheet, Destination::File { path, replace });
            return;
        }
        let Some((_, object)) = self.export_table(tab) else {
            return;
        };
        let sheet = ExportSheet {
            tab,
            id: object.id,
            result: None,
            asked,
            format,
            options: self.export.clone(),
            rows: RowsChoice::Matching,
            choose: false,
            columns: Self::export_columns(object),
            column: 0,
            destination: DestinationChoice::File,
            path: typed.to_owned(),
            exists: replace,
            review: false,
            asking: false,
            cursor_row: false,
            row_only: false,
            listed: None,
            picking: false,
            listing: false,
            list_closed: false,
            picked: None,
            to_line: 0,
        };
        self.run_export(&sheet, Destination::File { path, replace });
    }

    /// Cancels `job`; with none named (`:export cancel`) the one running,
    /// or with two or more the picker of them (spec 4, F5).
    pub(super) fn cancel_export(&mut self, job: Option<ExportId>) {
        if let Some(job) = job {
            if self
                .exports
                .iter()
                .any(|exporting| exporting.job.id == job && exporting.running())
            {
                self.backend.send(Command::CancelExport(job));
            }
            return;
        }
        let running: Vec<ExportId> = self
            .running_exports()
            .map(|exporting| exporting.job.id)
            .collect();
        match running.as_slice() {
            [] if self.look.terminal => {
                self.said = Some(gettext(self.locale, "no export is running").into_owned());
            }
            [] => {}
            [job] => self.backend.send(Command::CancelExport(*job)),
            _ if self.dialog.is_none() => {
                self.dialog = Some(Dialog::CancelExport(crate::model::CancelExportPicker {
                    lit: running[0],
                    jobs: running,
                }));
            }
            _ => {}
        }
    }

    /// The picker's row the keyboard is on, moved by `step`, round.
    pub(super) fn move_in_cancel_export(&mut self, step: isize) {
        let Some(Dialog::CancelExport(picker)) = &self.dialog else {
            return;
        };
        let rows: Vec<ExportId> = self
            .cancel_rows(picker)
            .iter()
            .map(|exporting| exporting.job.id)
            .collect();
        if let Some(Dialog::CancelExport(picker)) = &mut self.dialog
            && !rows.is_empty()
        {
            let lit = picker.lit_row(&rows) as isize;
            let to = (lit + step).rem_euclid(rows.len() as isize) as usize;
            picker.lit = rows[to];
        }
    }

    /// The picker's rows: those of its exports still running.
    pub fn cancel_rows(&self, picker: &crate::model::CancelExportPicker) -> Vec<&Exporting> {
        picker
            .jobs
            .iter()
            .filter_map(|job| {
                self.running_exports()
                    .find(|exporting| exporting.job.id == *job)
            })
            .collect()
    }

    /// A failed export's action: Choose folder… (the save panel again for
    /// the same export), Review SQL (the sheet again, on its statement),
    /// or Try again.
    pub(super) fn export_failure_action(&mut self, job: Option<ExportId>) {
        let Some(at) = self.export_at(job) else {
            return;
        };
        let exporting = &self.exports[at];
        let ExportStatus::Failed { failure, .. } = &exporting.status else {
            return;
        };
        match failure {
            ExportFailure::PermissionDenied { .. } | ExportFailure::Exists { .. }
                if !self.look.terminal =>
            {
                let suggested = self.dirs.downloads.join(&exporting.file);
                let title = gettext(self.locale, "Export rows").into_owned();
                let id = exporting.job.id;
                self.backend.ask_export_path(id, &title, &suggested);
            }
            ExportFailure::Timeout { .. } => {
                if self.dialog.is_none() {
                    let mut sheet = exporting.sheet.clone();
                    sheet.review = true;
                    sheet.asking = false;
                    sheet.asked = self.next_id();
                    self.exports.remove(at);
                    self.dialog = Some(Dialog::Export(sheet));
                }
            }
            // Over the clipboard's limit: the sheet again, to a file (spec
            // 3b, R7).
            ExportFailure::TooManyRows | ExportFailure::TooLarge => {
                if self.dialog.is_none() {
                    let mut sheet = exporting.sheet.clone();
                    sheet.destination = DestinationChoice::File;
                    sheet.review = false;
                    sheet.asking = false;
                    sheet.asked = self.next_id();
                    self.exports.remove(at);
                    self.dialog = Some(Dialog::Export(sheet));
                }
            }
            // The terminal look: another name, from the float.
            // Its own sheet: the table in front may be another now.
            ExportFailure::Exists { .. } | ExportFailure::PermissionDenied { .. } => {
                if self.dialog.is_none() {
                    let mut sheet = exporting.sheet.clone();
                    sheet.review = false;
                    sheet.asking = false;
                    sheet.asked = self.next_id();
                    self.exports.remove(at);
                    self.dialog = Some(Dialog::Export(sheet));
                }
            }
            _ => self.rerun_export(at, None),
        }
    }

    /// The export at `at` again from the start, under a new id, to
    /// `destination` where one is given. It starts anew, so it goes last
    /// among the others (spec 4, S14: in the order they started).
    fn rerun_export(&mut self, at: usize, destination: Option<Destination>) {
        if self.exports.get(at).is_none_or(Exporting::running) {
            return;
        }
        let id = ExportId(self.next_id());
        let Some(exporting) = self.exports.get_mut(at) else {
            return;
        };
        exporting.job.id = id;
        if let Some(destination) = destination {
            if let Destination::File { path, .. } = &destination {
                exporting.file = path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default();
                exporting.shown_path =
                    crate::export_sheet::shown_path(path, self.dirs.home.as_deref());
            }
            exporting.job.destination = destination;
        }
        exporting.status = ExportStatus::Running { rows: 0, bytes: 0 };
        exporting.started = Instant::now();
        exporting.hidden = false;
        if let Some(entry) = &mut exporting.history {
            entry.at_ms = super::history::now_ms();
        }
        let job = exporting.job.clone();
        self.backend.send(Command::Export(Box::new(job)));
        let again = self.exports.remove(at);
        self.exports.push(again);
    }

    /// Show in Finder (Show in folder), `go`: while the done line shows.
    pub(super) fn reveal_export(&mut self, job: Option<ExportId>) {
        let Some(exporting) = self.export_at(job).map(|at| &self.exports[at]) else {
            return;
        };
        if let (ExportStatus::Done { .. }, Destination::File { path, .. }) =
            (&exporting.status, &exporting.job.destination)
        {
            let path = path.clone();
            self.backend.send(Command::Reveal { path });
        }
    }

    /// What the backend says of the export `job`.
    pub(super) fn export_state(&mut self, job: ExportId, state: ExportState) {
        let Some(place) = self.export_at(Some(job)) else {
            return;
        };
        let exporting = &mut self.exports[place];
        let at = Instant::now();
        let (rows, bytes) = match exporting.status {
            ExportStatus::Running { rows, bytes } => (rows, bytes),
            _ => (0, 0),
        };
        let was_running = exporting.running();
        let mut copied_text = None;
        exporting.status = match state {
            ExportState::Started { statement } => {
                // The statement as the driver runs it: the history keeps it.
                if let Some(statement) = statement {
                    exporting.statement = Some(statement);
                }
                ExportStatus::Running { rows: 0, bytes: 0 }
            }
            ExportState::Progress { rows, bytes } => ExportStatus::Running { rows, bytes },
            ExportState::Done {
                rows,
                bytes,
                took,
                copied,
                ..
            } => {
                copied_text = copied;
                ExportStatus::Done {
                    rows,
                    bytes,
                    took,
                    at,
                }
            }
            ExportState::Cancelled => ExportStatus::Cancelled { at },
            ExportState::Failed(failure) => ExportStatus::Failed {
                failure,
                rows,
                bytes,
                at,
            },
        };
        // A file written while the window is behind is told (spec 3b, E4).
        let told = was_running
            && matches!(exporting.status, ExportStatus::Done { .. })
            && matches!(exporting.job.destination, Destination::File { .. })
            && !self.focused;
        let ended = !exporting.running();
        if told {
            self.tell_export_done(place);
        }
        if ended {
            self.export_ended(place, copied_text);
        }
        // The float's file is there: it opens again, and Enter replaces it.
        // Found again by its id: a copy's end takes it out of the list.
        let place = self.export_at(Some(job));
        let reopen = match place.and_then(|place| self.exports.get(place)) {
            Some(exporting) if self.look.terminal && exporting.reopens => matches!(
                exporting.status,
                ExportStatus::Failed {
                    failure: ExportFailure::Exists { .. },
                    ..
                }
            ),
            _ => false,
        };
        if let Some(place) = place.filter(|_| reopen && self.dialog.is_none()) {
            let asked = self.next_id();
            let mut sheet = self.exports.remove(place).sheet;
            sheet.exists = true;
            sheet.asked = asked;
            self.dialog = Some(Dialog::Export(sheet));
        }
    }

    /// Tells of a finished export behind other windows: a desktop
    /// notification on Linux, the Dock's bounce on macOS, the taskbar
    /// button's flash on Windows (spec 3b, E4 and E5).
    fn tell_export_done(&mut self, at: usize) {
        let Some(exporting) = self.exports.get(at) else {
            return;
        };
        let ExportStatus::Done { rows, .. } = exporting.status else {
            return;
        };
        let body = crate::i18n::ngettext(
            self.locale,
            "Exported {n} row to {file}",
            "Exported {n} rows to {file}",
            u32::try_from(rows).unwrap_or(u32::MAX),
        )
        .replace("{n}", &crate::format::group_digits(rows))
        .replace("{file}", &exporting.file);
        if cfg!(any(target_os = "macos", windows)) {
            self.window_asks
                .push(egui::ViewportCommand::RequestUserAttention(
                    egui::UserAttentionType::Informational,
                ));
        }
        self.backend.send(Command::Notify {
            summary: "Tabletist".into(),
            body,
        });
    }

    /// The end of the export at `at`: its history entry, and the
    /// clipboard's text.
    fn export_ended(&mut self, at: usize, copied: Option<String>) {
        let Some(exporting) = self.exports.get(at) else {
            return;
        };
        if let (Some(sql), Some(mut entry)) =
            (exporting.statement.clone(), exporting.history.clone())
        {
            entry.sql = Some(sql);
            let took = exporting.started.elapsed();
            let took_ms = u64::try_from(took.as_millis()).unwrap_or(u64::MAX);
            entry.took_ms = Some(took_ms);
            // When it started, as a filter's entry is.
            entry.at_ms =
                super::history::now_ms().saturating_sub(i64::try_from(took_ms).unwrap_or(0));
            match &exporting.status {
                ExportStatus::Done { rows, .. } => entry.rows = Some(*rows),
                ExportStatus::Cancelled { .. } => entry.outcome = Outcome::Cancelled,
                ExportStatus::Failed { failure, rows, .. } => {
                    entry.outcome = Outcome::Error;
                    entry.rows = Some(*rows);
                    // A few words, never the file's name or its folder.
                    entry.message = Some(match failure {
                        ExportFailure::PermissionDenied { .. } => "permission denied".into(),
                        ExportFailure::DiskFull { .. } => "the disk is full".into(),
                        ExportFailure::ConnectionLost { .. } => "the connection was lost".into(),
                        ExportFailure::Timeout { .. } => "the server's statement timeout".into(),
                        ExportFailure::Exists { .. } => "the file exists".into(),
                        ExportFailure::TooManyRows => "more than 10,000 rows".into(),
                        ExportFailure::TooLarge => "over 20 MB".into(),
                        ExportFailure::Other(why) => why.clone(),
                    });
                }
                ExportStatus::Asking | ExportStatus::Running { .. } => {}
            }
            self.record(entry);
        }
        if let Some(text) = copied
            && let Some(exporting) = self.exports.get(at)
        {
            let rows = match exporting.status {
                ExportStatus::Done { rows, .. } => rows,
                _ => 0,
            };
            let format = exporting.job.format;
            let said = crate::i18n::ngettext(
                self.locale,
                "Copied {n} row as {format}",
                "Copied {n} rows as {format}",
                u32::try_from(rows).unwrap_or(u32::MAX),
            )
            .replace("{n}", &crate::format::group_digits(rows))
            .replace("{format}", format.name());
            self.said = Some(self.look.label(&said));
            self.to_clipboard = Some(text);
            self.exports.remove(at);
        }
    }

    /// Puts away the end of each export once it has been told for long
    /// enough (a failure stays until dismissed), and the `:export cancel`
    /// picker once none of its exports runs.
    pub(super) fn tick_export(&mut self) {
        // A sheet opened over a table still loading takes its columns
        // once they are there.
        let columns = match &self.dialog {
            Some(Dialog::Export(sheet)) if sheet.columns.is_empty() => self
                .workspace(sheet.tab)
                .and_then(|workspace| workspace.object_tab(sheet.id))
                .map(Self::export_columns),
            _ => None,
        };
        if let (Some(columns), Some(Dialog::Export(sheet))) = (columns, &mut self.dialog) {
            sheet.columns = columns;
        }
        let now = Instant::now();
        self.exports.retain(|exporting| !exporting.gone(now));
        if let Some(Dialog::CancelExport(picker)) = &self.dialog
            && self.cancel_rows(picker).is_empty()
        {
            self.dialog = None;
        }
    }

    /// The failed export `job` put away; with none named, the one whose
    /// end the terminal look's line shows.
    pub(super) fn dismiss_export(&mut self, job: Option<ExportId>) {
        if let Some(at) = self.export_at(job)
            && !self.exports[at].running()
        {
            self.exports.remove(at);
        }
    }

    pub(super) fn hide_export(&mut self, job: ExportId, hidden: bool) {
        if let Some(at) = self.export_at(Some(job)) {
            self.exports[at].hidden = hidden;
        }
    }
}
