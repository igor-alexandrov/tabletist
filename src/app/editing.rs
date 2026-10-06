//! The reducer's part in editing a table's values: the guard that keeps a
//! page with pending changes, the editor, the pending set and the save.
//! What all of it is decided from is in `crate::edit`.

use tabletist_db::{Access, ChangeSet, ColumnClass, Error, NewValue, WriteOutcome, column_class};

use super::App;
use crate::backend::{Command, RequestId, SessionId};
use crate::edit::{
    Answer, Editor, EditorPlace, Lock, Note, Pending, Problem, Saved, Saving, State, Table,
    change_set, check, conflicting, is_change, opens_large, same_changes, settled, shown_lines,
    start_text,
};
use crate::model::{
    Action, Advance, CellPos, ConflictPrompt, ConnTabId, Dialog, EditStart, Held, LeavePrompt,
    ObjectTab, Pane, SaveBlock, SessionStatus, TabId, WritePrompt,
};
use crate::review::Values;

/// Whether `action` is dropped while a prompt about pending changes is up:
/// what edits, saves or discards, what moves the selection (it closes an
/// editor; of any tab, nothing is told apart), what shows or hides Review
/// SQL (shown, it closes an editor as a left edit, and the set is another
/// than the one asked about), and what takes the dialog's place whatever
/// dialog it is, and with it what the prompt holds. The
/// prompts' own answers, `CloseDialog` and what the backend says are not
/// among them. Nor is `EditorTyped`: it changes no text, and a keystroke
/// that shares its frame with the key that raised the prompt must be noted,
/// or the prompt's Save would close the editor without it.
pub(super) fn dropped_under_a_prompt(action: &Action) -> bool {
    matches!(
        action,
        Action::EditCell { .. }
            | Action::EditField { .. }
            | Action::FocusFields { .. }
            | Action::EditorBreak { .. }
            | Action::FormatEditor { .. }
            | Action::CommitEdit { .. }
            | Action::LeaveEdit { .. }
            | Action::CancelEdit { .. }
            | Action::SetNull { .. }
            | Action::RevertCell { .. }
            | Action::DiscardEdits { .. }
            | Action::WriteEdits { .. }
            | Action::ReviewEdits { .. }
            | Action::SelectCell { .. }
            | Action::MoveSelection { .. }
            // Closing the row panel closes the editor it draws, and the
            // text of that editor joins the set.
            | Action::ToggleRowPanel(_)
            | Action::NewConnection
            | Action::EditConnection(_)
            // These two take the dialog before they look at its kind.
            | Action::CancelPassword
            | Action::TrustHostKey
    )
}

/// What the text of an editor that was typed into comes to once the editor
/// closes.
struct Typed {
    cell: CellPos,
    /// The text, as the cell's new value.
    new: NewValue,
    /// It differs from what the cell loaded. A text that does not is no
    /// pending change, and takes one the cell had out of the set.
    changed: bool,
    /// What the column's check refuses of a text that is a change.
    problem: Option<Problem>,
}

/// What closing the tab's editor makes of its text: `None` where no editor
/// is open, it was not typed into, or its cell is no longer on the page.
fn typed(table: &Table<'_>, object: &ObjectTab) -> Option<Typed> {
    let editor = object.edits.editor.as_ref()?;
    let cell = editor.cell;
    let column = table.column(cell.col)?;
    let class = column_class(table.dialect, &column.type_name);
    let loaded = table.page.rows.get(cell.row)?.get(cell.col)?;
    if !editor.touched {
        return None;
    }
    let new = NewValue::Text(editor.text.clone());
    let changed = is_change(loaded, &new, class);
    let problem = changed
        .then(|| check(table.dialect, column, &editor.text))
        .flatten();
    Some(Typed {
        cell,
        new,
        changed,
        problem,
    })
}

impl App {
    /// What editing may know of a table tab: `None` while it has no page.
    fn table<T>(
        &self,
        tab: ConnTabId,
        id: TabId,
        read: impl FnOnce(&Table<'_>, &ObjectTab) -> T,
    ) -> Option<T> {
        let workspace = self.workspace(tab)?;
        let object = workspace.object_tab(id)?;
        let table = Table::of(workspace, object)?;
        Some(read(&table, object))
    }

    /// The tabs holding edits whose page or whose tab `action` would drop.
    pub(super) fn dropped_by(&self, action: &Action) -> Vec<(ConnTabId, TabId)> {
        let object = |tab: ConnTabId, id: TabId| {
            self.workspace(tab)
                .and_then(|workspace| workspace.object_tab(id))
        };
        let one = |tab: ConnTabId, id: TabId| {
            if object(tab, id).is_some_and(|object| object.edits.holds()) {
                vec![(tab, id)]
            } else {
                Vec::new()
            }
        };
        let all = |tab: ConnTabId| {
            self.workspace(tab)
                .map(|workspace| {
                    workspace
                        .object_tabs()
                        .filter(|object| object.edits.holds())
                        .map(|object| (tab, object.id))
                        .collect()
                })
                .unwrap_or_default()
        };
        match action {
            // The arm's own condition: an action that would do nothing is
            // not worth a question whose Discard throws the set away.
            Action::NextPage { tab, object_tab }
                if !object(*tab, *object_tab)
                    .and_then(|object| object.page())
                    .is_some_and(|page| page.has_more) =>
            {
                Vec::new()
            }
            Action::PrevPage { tab, object_tab }
                if object(*tab, *object_tab).is_none_or(|object| object.query.offset == 0) =>
            {
                Vec::new()
            }
            Action::ClearSort { tab, object_tab }
                if object(*tab, *object_tab).is_none_or(|object| object.query.sort.is_empty()) =>
            {
                Vec::new()
            }
            Action::Connect { conn, .. } if self.connections.get(conn).is_none() => Vec::new(),
            Action::NextPage { tab, object_tab }
            | Action::PrevPage { tab, object_tab }
            | Action::SortBy {
                tab, object_tab, ..
            }
            | Action::ClearSort { tab, object_tab }
            | Action::ApplyFilters { tab, object_tab }
            | Action::ClearFilters { tab, object_tab }
            | Action::DropFilter {
                tab, object_tab, ..
            }
            | Action::RetryRows { tab, object_tab }
            // The structure the set was made on stays with it, as the page
            // does: the retry forgets it, and what comes back may have no
            // key to save by.
            | Action::RetryStructure { tab, object_tab } => one(*tab, *object_tab),
            Action::CloseTab { tab, id } => one(*tab, *id),
            // The active tab's page, as the arm fetches it. Nothing while
            // a SQL editor shows: the arm does nothing there.
            Action::Refresh(tab) => self
                .workspace(*tab)
                .and_then(|workspace| workspace.active_object_tab())
                .map(|object| one(*tab, object.id))
                .unwrap_or_default(),
            Action::CloseConnTab(tab)
            | Action::Disconnect(tab)
            | Action::SwitchDatabase { tab, .. }
            | Action::Connect { tab, .. } => all(*tab),
            _ => Vec::new(),
        }
    }

    /// Keeps `held` and asks. While one of the tabs is saving an action is
    /// ignored (the actions the guard covers are disabled until a save
    /// ends), but not the window's close: no control of the app's asks for
    /// it, and a save that never answers must not keep the window open
    /// for good. A dialog the user is in is never replaced.
    pub(super) fn hold(&mut self, held: Held, tabs: Vec<(ConnTabId, TabId)>) {
        let edits = |&(tab, id): &(ConnTabId, TabId)| {
            self.workspace(tab)
                .and_then(|workspace| workspace.object_tab(id))
                .map(|object| &object.edits)
        };
        let saving = tabs
            .iter()
            .any(|at| edits(at).is_some_and(|edits| edits.saving.is_some()));
        if saving && !matches!(held, Held::CloseWindow) {
            return;
        }
        if self.dialog.is_some() {
            self.notice = Some("Save or discard the pending changes first.".into());
            return;
        }
        let changes = tabs.iter().map(|&(tab, id)| self.unwritten(tab, id)).sum();
        // Never under a save, which blocks the tab's Save as well.
        let can_save = match tabs.as_slice() {
            [(tab, id)] => self.save_blocked(*tab, *id).is_none(),
            _ => false,
        };
        self.dialog = Some(Dialog::Leave(Box::new(LeavePrompt {
            held,
            tabs,
            can_save,
            changes,
            saving,
        })));
    }

    /// How many changes the tab holds, as the question about leaving it
    /// counts them: its pending cells, and the cell being edited where
    /// Save, which closes the editor first, would make it one more. That
    /// is an editor that was typed into, on a cell that is not pending
    /// already, whose text is a change of what was loaded: one its column
    /// refuses as well, which is left as a cell to fix. A pending cell
    /// typed back to what it loaded is one fewer: closing the editor
    /// takes it out of the set. An open editor with nothing pending yet
    /// still counts as one: a tab that holds edits never counts as none.
    fn unwritten(&self, tab: ConnTabId, id: TabId) -> usize {
        let Some(edits) = self
            .workspace(tab)
            .and_then(|workspace| workspace.object_tab(id))
            .map(|object| &object.edits)
        else {
            return 0;
        };
        let closed = self.table(tab, id, typed).flatten();
        let pending = closed
            .as_ref()
            .is_some_and(|typed| edits.cells.contains_key(&(typed.cell.row, typed.cell.col)));
        let changed = closed.as_ref().map(|typed| typed.changed);
        let changes = edits.counts().changes;
        let changes = match (changed, pending) {
            (Some(true), false) => changes + 1,
            (Some(false), true) => changes.saturating_sub(1),
            _ => changes,
        };
        changes.max(1)
    }

    /// Does what was held, now that nothing is in its way. It passes the
    /// guard again like any action.
    pub(super) fn perform(&mut self, held: Held) {
        match held {
            Held::Action(action) => self.apply(*action),
            Held::CloseWindow => {
                let tabs = self.holding_edits();
                if tabs.is_empty() {
                    self.closing = true;
                } else {
                    self.hold(Held::CloseWindow, tabs);
                }
            }
        }
    }

    /// Every table tab that holds edits, over all the open connections:
    /// what closing the window would drop.
    fn holding_edits(&self) -> Vec<(ConnTabId, TabId)> {
        self.open_connections()
            .flat_map(|(tab, workspace)| {
                workspace
                    .object_tabs()
                    .filter(|object| object.edits.holds())
                    .map(move |object| (tab, object.id))
            })
            .collect()
    }

    /// Answers a request to close the window, when this frame brings one:
    /// while a tab holds edits the request is cancelled and the user is
    /// asked, as before any action that would drop them. Under a save too:
    /// the question then offers no Save, and its Discard gives the save up
    /// and closes. It runs with the app's logic too, which is all that
    /// runs while the window is hidden, and more than once in a frame
    /// changes nothing.
    pub(super) fn hold_close(&mut self, ctx: &egui::Context) {
        // The close this app asked for, once nothing was in its way.
        if self.closing || !ctx.input(|input| input.viewport().close_requested()) {
            return;
        }
        let tabs = self.holding_edits();
        if tabs.is_empty() {
            return;
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        // Asked already: the question is up, and stays as it is.
        let asked = matches!(
            &self.dialog,
            Some(Dialog::Leave(prompt)) if matches!(prompt.held, Held::CloseWindow)
        );
        if !asked {
            self.hold(Held::CloseWindow, tabs);
        }
    }

    /// Why the tab's pending changes cannot be saved now, if they cannot.
    /// The view shows it on the disabled Save.
    pub fn save_blocked(&self, tab: ConnTabId, id: TabId) -> Option<SaveBlock> {
        let workspace = self.workspace(tab)?;
        let object = workspace.object_tab(id)?;
        if object.edits.saving.is_some() {
            return Some(SaveBlock::Saving);
        }
        if object.edits.counts().to_fix > 0 {
            return Some(SaveBlock::ToFix);
        }
        if !matches!(workspace.status, SessionStatus::Connected) {
            return Some(SaveBlock::Disconnected);
        }
        if workspace.access == Access::ReadOnly {
            return Some(SaveBlock::ReadOnly);
        }
        // Cells are pending and no change set comes of them: a Save that
        // would do nothing is not offered.
        let unsendable = !object.edits.cells.is_empty()
            && self
                .table(tab, id, |table, object| {
                    change_set(&object.object, table, &object.edits.cells)
                })
                .flatten()
                .is_none();
        unsendable.then_some(SaveBlock::Unsendable)
    }

    /// Saves the tab's pending changes, and does `then` once everything
    /// is written. A save that is not sent, or that writes nothing, drops
    /// `then`.
    pub(super) fn write_edits(&mut self, tab: ConnTabId, id: TabId, then: Option<Held>) {
        // The text being typed is part of what is saved.
        self.close_editor(tab, id, true);
        // Only an editor that was opened and left as it was: there is
        // nothing to save, and what was held goes on. Only then: no change
        // set is built either while cells are pending on a table whose key
        // is gone, and going on there would drop them (`save_blocked`
        // stops that save below).
        let nothing = self
            .workspace(tab)
            .and_then(|workspace| workspace.object_tab(id))
            .is_some_and(|object| object.edits.cells.is_empty());
        if nothing {
            if let Some(held) = then {
                self.perform(held);
            }
            return;
        }
        if let Some(block) = self.save_blocked(tab, id) {
            // The terminal's line says why, whatever it said before. A
            // save that is running says so already.
            if let Some(workspace) = self.workspace_mut(tab) {
                workspace.save_refused = block != SaveBlock::Saving;
            }
            return;
        }
        let built = self
            .table(tab, id, |table, object| {
                change_set(&object.object, table, &object.edits.cells)
            })
            .flatten();
        let Some((changes, rows)) = built else {
            return;
        };
        let confirm = self
            .workspace(tab)
            .is_some_and(|workspace| workspace.environment.confirms_writes());
        if confirm {
            // A dialog the user is in is not replaced: the save waits.
            if self.dialog.is_some() {
                return;
            }
            let Some(dialect) = self
                .workspace(tab)
                .map(|workspace| workspace.driver.dialect())
            else {
                return;
            };
            // No save is offered with a statement that cannot be shown:
            // its row fails here as it would in the save, with the
            // builder's reason. No cell is to fix: Save was not disabled.
            let review = crate::review::of(dialect, &changes, &[], Values::Shown);
            if let Some((index, error)) = review.refused.clone() {
                if let Some(object) = self.object_tab_mut(tab, id) {
                    object.edits.saved = None;
                    object.edits.fail(rows.get(index).copied(), error);
                    // The row panel says what stands against a cell.
                    object.fields = None;
                }
                return;
            }
            // The terminal's box does not list the statements: its panel
            // does, beside it, open or not until now. The set is the one
            // the review was made of.
            if self.look.terminal
                && let Some(object) = self.object_tab_mut(tab, id)
            {
                object.edits.reviewing = true;
                object.edits.review = Some(review.clone());
            }
            let cells = changes.rows.iter().map(|row| row.set.len()).sum();
            self.dialog = Some(Dialog::ConfirmWrite(Box::new(WritePrompt {
                tab,
                id,
                review,
                changes: cells,
                rows: rows.len(),
                typed: String::new(),
                focus: true,
                changeset: changes,
                then,
                after_answer: None,
            })));
            return;
        }
        self.send_write(tab, id, changes, rows, then);
    }

    /// Sends the save the production confirmation shows, if it is still
    /// the save the tab would make. The prompt was up for a while: with the
    /// session gone, a save running, an editor open or another set pending,
    /// nothing is sent and what was held for the save is dropped.
    pub(super) fn confirm_write(&mut self) {
        // The kind is checked before the dialog is taken: another dialog
        // is not closed by it.
        if !matches!(self.dialog, Some(Dialog::ConfirmWrite(_))) {
            return;
        }
        let Some(Dialog::ConfirmWrite(prompt)) = self.dialog.take() else {
            return;
        };
        let WritePrompt {
            tab,
            id,
            changeset,
            then,
            ..
        } = *prompt;
        match self.save_blocked(tab, id) {
            None => {}
            // The session went while the prompt was up: the tab says so,
            // where the save would have said how it ended, and says that
            // nothing went out. Any other reason is the Save's own to show.
            Some(SaveBlock::Disconnected) => {
                if let Some(object) = self.object_tab_mut(tab, id) {
                    object.edits.note = Some(Note::NotSent);
                }
                return;
            }
            Some(_) => return,
        }
        // The set as the tab holds it now: nothing is sent that was not on
        // screen, and nothing that was is sent once it is no longer wanted.
        let now = self
            .table(tab, id, |table, object| {
                if object.edits.editor.is_some() {
                    return None;
                }
                change_set(&object.object, table, &object.edits.cells)
            })
            .flatten();
        // By the bits of its floats: a NaN the page loaded is the same
        // NaN now, and a set that holds one is still the set that was shown.
        if let Some((changes, rows)) = now
            && same_changes(&changes, &changeset)
        {
            self.send_write(tab, id, changes, rows, then);
        }
    }

    fn send_write(
        &mut self,
        tab: ConnTabId,
        id: TabId,
        changes: ChangeSet,
        rows: Vec<usize>,
        then: Option<Held>,
    ) {
        let request = RequestId(self.next_id());
        let Some(workspace) = self.workspace_mut(tab) else {
            return;
        };
        let session = workspace.session;
        let Some(object) = workspace.object_tab_mut(id) else {
            return;
        };
        object.edits.note = None;
        object.edits.saved = None;
        object.edits.saving = Some(Saving {
            request,
            rows,
            started: std::time::Instant::now(),
            then,
        });
        self.backend.send(Command::Write {
            session,
            request,
            changes,
        });
    }

    pub(super) fn editor_mut(&mut self, tab: ConnTabId, id: TabId) -> Option<&mut Editor> {
        self.object_tab_mut(tab, id)?.edits.editor.as_mut()
    }

    /// What the open editor's text fails, if anything.
    pub(super) fn editor_problem(&self, tab: ConnTabId, id: TabId) -> Option<Problem> {
        self.table(tab, id, |table, object| {
            let editor = object.edits.editor.as_ref()?;
            let column = table.column(editor.cell.col)?;
            check(table.dialect, column, &editor.text)
        })
        .flatten()
    }

    /// The column of the tab's open editor, when the row panel draws it.
    pub(super) fn panel_field(&self, tab: ConnTabId, id: TabId) -> Option<usize> {
        let object = self.workspace(tab)?.object_tab(id)?;
        let editor = object.edits.editor.as_ref()?;
        (editor.place == EditorPlace::Panel).then_some(editor.cell.col)
    }

    /// An edit made in the row panel ended, by a commit or a cancel: the
    /// keyboard goes to the panel's field of the column `col`, the one that
    /// was edited or the one the commit walks to, so Enter edits it and
    /// the keys go on from it.
    pub(super) fn back_to_field(&mut self, tab: ConnTabId, id: TabId, col: usize) {
        if let Some(object) = self.object_tab_mut(tab, id) {
            object.focus_field(col);
        }
    }

    /// The field a commit in the row panel's field of the column `col`
    /// walks to: the selected row's next that can be edited, in the page's
    /// column order, or the one before it. None at the row's end, and for
    /// a commit that stays.
    pub(super) fn field_after(
        &self,
        tab: ConnTabId,
        id: TabId,
        col: usize,
        then: Advance,
    ) -> Option<usize> {
        let forward = match then {
            Advance::NextField => true,
            Advance::PrevField => false,
            Advance::Stay | Advance::Down | Advance::Right | Advance::Left => return None,
        };
        self.table(tab, id, |table, object| {
            let row = object.selection?.row;
            let free = |col: &usize| table.lock(CellPos { row, col: *col }).is_none();
            if forward {
                (col + 1..table.page.columns.len()).find(free)
            } else {
                (0..col).rev().find(free)
            }
        })
        .flatten()
    }

    /// Opens the editor on `cell`, drawn in `place`, or says there why the
    /// cell cannot be edited.
    pub(super) fn edit_cell(
        &mut self,
        tab: ConnTabId,
        id: TabId,
        cell: CellPos,
        start: EditStart,
        place: EditorPlace,
    ) {
        // An editor open on another cell keeps its text.
        self.close_editor(tab, id, true);
        // What this edit comes to (a cell that is locked) is what the
        // terminal's line says next, not why the last save was not made.
        if let Some(workspace) = self.workspace_mut(tab) {
            workspace.save_refused = false;
            workspace.review_refused = false;
        }
        let (asked, touched) = match &start {
            EditStart::Value => (true, false),
            EditStart::Replace(_) => (true, true),
            EditStart::Typed(_) => (false, true),
        };
        // Opened from its value, to be read before it is changed.
        let from_value = matches!(start, EditStart::Value);
        let opened = self.table(tab, id, |table, object| {
            if let Some(lock) = table.lock(cell) {
                return Err(lock);
            }
            let class = table.class(cell.col).unwrap_or(ColumnClass::Other);
            // Whether the editor opens on a text the user typed before:
            // the cell's pending one. It is typed text still, whatever it
            // holds: its check holds for it, empty or not, and Enter does
            // not close on it while its column refuses it.
            let mut kept = false;
            let text = match start {
                EditStart::Replace(text) | EditStart::Typed(text) => text,
                EditStart::Value => match object.edits.cells.get(&(cell.row, cell.col)) {
                    Some(pending) => match &pending.new {
                        NewValue::Text(text) => {
                            kept = true;
                            text.clone()
                        }
                        NewValue::Null => String::new(),
                    },
                    None => start_text(&table.page.rows[cell.row][cell.col], class),
                },
            };
            // A value of several lines, a long one or a document is edited
            // in the large editor, where its edit was asked for: a popover
            // at its cell, or the tall field in the row panel.
            let large = opens_large(&text, class);
            // What the text it opens with fails, so the editor says so
            // from its first frame: a cell left to fix opened again, a
            // character typed on a cell that does not take it. Not the
            // empty text a NULL opens with, or an edit from nothing: that
            // one nobody typed.
            let problem = table
                .column(cell.col)
                .filter(|_| kept || !text.is_empty())
                .and_then(|column| check(table.dialect, column, &text));
            Ok(Editor {
                cell,
                place,
                large,
                text,
                focus: true,
                // A value of several lines is read from its top: its end
                // may be far below what its editor shows.
                top: large && from_value,
                touched: touched || kept,
                problem,
            })
        });
        let Some(opened) = opened else {
            return;
        };
        let Some(object) = self.object_tab_mut(tab, id) else {
            return;
        };
        match opened {
            Ok(editor) => {
                object.selection = Some(cell);
                object.edits.editor = Some(editor);
                object.edits.why = None;
                // The keyboard is the editor's: a field that was owed it
                // (an edit ended in the frame that asked for this one)
                // would take it from the editor, which would then be left.
                object.focus_field = None;
                // A tab being edited is no preview to replace.
                object.pinned = true;
            }
            Err(Lock::NoSuchCell) => return,
            // Typing on a cell that cannot be edited does nothing.
            Err(_) if !asked => return,
            Err(lock) => {
                object.selection = Some(cell);
                object.edits.why = Some((cell, lock));
                object.edits.why_place = place;
            }
        }
        if let Some(workspace) = self.workspace_mut(tab) {
            workspace.pane = Pane::Grid;
        }
    }

    /// Shows the row panel and asks it to give the keyboard to the selected
    /// row's first field that can be edited, in the page's column order. No
    /// editor opens. A row with no such field keeps the keyboard where it
    /// is: the panel says why no cell of it can be edited, and the reason
    /// is kept for the selected cell, for the terminal's mode line.
    pub(super) fn focus_fields(&mut self, tab: ConnTabId, id: TabId) {
        let found = self.table(tab, id, |table, object| {
            let cell = object.selection?;
            // The panel shows a row of the Data view only.
            if object.view != crate::model::ObjectView::Data {
                return None;
            }
            if let Some(lock) = table.row_lock(cell.row) {
                return Some((cell, Err(lock)));
            }
            let free = |col: &usize| {
                table
                    .lock(CellPos {
                        row: cell.row,
                        col: *col,
                    })
                    .is_none()
            };
            match (0..table.page.columns.len()).find(free) {
                Some(col) => Some((cell, Ok(col))),
                // Each cell is locked for a reason of its own: the
                // selected cell's is the one said.
                None => Some((cell, Err(table.lock(cell)?))),
            }
        });
        let Some(Some((cell, found))) = found else {
            return;
        };
        // An editor open on a cell keeps its text.
        self.close_editor(tab, id, true);
        if let Some(workspace) = self.workspace_mut(tab) {
            workspace.row_panel = true;
            // As an edit asked for does: the keys are the table's, not the
            // tree's, whose `j` and `k` the terminal look would otherwise
            // go on reading.
            workspace.pane = Pane::Grid;
            workspace.save_refused = false;
            workspace.review_refused = false;
        }
        let Some(object) = self.object_tab_mut(tab, id) else {
            return;
        };
        match found {
            Ok(col) => {
                object.focus_field(col);
                object.edits.why = None;
            }
            Err(lock) => {
                object.edits.why = Some((cell, lock));
                object.edits.why_place = EditorPlace::Panel;
            }
        }
    }

    /// Takes the open editor's text as its cell's new value and closes it.
    /// A text its column does not take keeps the editor open, unless the
    /// edit is `left` (the keyboard went elsewhere): then the text is kept
    /// as a cell to fix, so typing is never lost. Says whether the editor
    /// closed.
    pub(super) fn close_editor(&mut self, tab: ConnTabId, id: TabId, left: bool) -> bool {
        let verdict = self.table(tab, id, typed);
        let Some(object) = self.object_tab_mut(tab, id) else {
            return false;
        };
        let Some(Some(Typed {
            cell,
            new,
            changed,
            problem,
        })) = verdict
        else {
            // Nothing was typed, or there is no page or no such cell any
            // more: the editor closes and the set stays as it was.
            return object.edits.editor.take().is_some();
        };
        if problem.is_some() && !left {
            if let Some(editor) = object.edits.editor.as_mut() {
                editor.problem = problem;
            }
            return false;
        }
        object.edits.editor = None;
        let key = (cell.row, cell.col);
        if changed {
            let state = problem.map_or(State::Ready, State::ToFix);
            object.edits.put(key, Pending { new, state });
        } else {
            object.edits.revert(key);
        }
        // The row panel shows the pending value.
        object.fields = None;
        true
    }

    /// Lays the document in the tab's large editor out a member to a line,
    /// where the editor is on a JSON column and its text is a document: a
    /// text its column does not take stays as it was typed. Only the white
    /// space between its pieces changes (`ui::json_text::pretty`).
    pub(super) fn format_editor(&mut self, tab: ConnTabId, id: TabId) {
        let laid = self.table(tab, id, |table, object| {
            let editor = object.edits.editor.as_ref().filter(|editor| editor.large)?;
            let column = table.column(editor.cell.col)?;
            if table.class(editor.cell.col) != Some(ColumnClass::Json) {
                return None;
            }
            if check(table.dialect, column, &editor.text).is_some() {
                return None;
            }
            // Not one that would not fit the editor laid out: the editor
            // cuts what is over its limit, and the document's end with it.
            let laid = crate::edit::laid_out_json(&editor.text)?;
            (laid != editor.text).then_some(laid)
        });
        if let (Some(Some(laid)), Some(editor)) = (laid, self.editor_mut(tab, id)) {
            editor.text = laid;
            // It is typed text from here on: what was only opened and
            // laid out is a change the user asked for.
            editor.touched = true;
        }
    }

    pub(super) fn set_null(&mut self, tab: ConnTabId, id: TabId) {
        let verdict = self.table(tab, id, |table, object| {
            let cell = object.selection?;
            if object.edits.editor.is_some() || table.lock(cell).is_some() {
                return None;
            }
            let column = table.column(cell.col)?;
            if !column.nullable {
                return None;
            }
            Some((cell, !table.page.rows[cell.row][cell.col].is_null()))
        });
        let Some(Some((cell, changed))) = verdict else {
            return;
        };
        let Some(object) = self.object_tab_mut(tab, id) else {
            return;
        };
        let key = (cell.row, cell.col);
        if changed {
            let null = Pending {
                new: NewValue::Null,
                state: State::Ready,
            };
            object.edits.put(key, null);
            // As opening an editor does: a tab with a pending cell is no
            // preview for the next single click to replace.
            object.pinned = true;
        } else {
            object.edits.revert(key);
        }
        object.fields = None;
    }

    /// Runs what the terminal's `:` prompt holds, and closes it: `w` saves
    /// the pending changes of the table on screen, `e!` drops them and
    /// `diff` shows what a save would run, as their keys and the bar's
    /// buttons do in the other looks. Any other text is not a command, and
    /// is kept for the status line to say so.
    pub(super) fn run_command(&mut self, tab: ConnTabId) {
        let Some(workspace) = self.workspace_mut(tab) else {
            return;
        };
        workspace.focus_command = false;
        workspace.command_error = None;
        workspace.save_refused = false;
        workspace.review_refused = false;
        let Some(text) = workspace.command.take() else {
            return;
        };
        // The table on screen, and whether anything is pending in it.
        let table = workspace
            .active_object_tab()
            .map(|object| (object.id, !object.edits.cells.is_empty()));
        let action = match (text.trim(), table) {
            ("", _) => return,
            ("w", Some((id, _))) => Action::WriteEdits { tab, id },
            ("e!", Some((id, _))) => Action::DiscardEdits { tab, id },
            ("diff", Some((id, true))) => Action::ReviewEdits {
                tab,
                id,
                show: true,
            },
            // Nothing to show: the line says so, where a panel that opened
            // empty would say it less plainly.
            ("diff", Some((_, false))) => {
                workspace.review_refused = true;
                return;
            }
            // A SQL editor is in front: there is no table to act on.
            ("w" | "e!" | "diff", None) => return,
            (other, _) => {
                workspace.command_error = Some(other.to_owned());
                return;
            }
        };
        // As from a key: under a question about the changes it is dropped.
        self.apply(action);
    }

    /// Shows or hides the Review SQL of the tab. Shown, it takes the text
    /// being typed first, as a save does: what is reviewed is what a save
    /// would send. It opens only on something pending, and its text is
    /// made at the frame's end (see `make_reviews`).
    pub(super) fn review_edits(&mut self, tab: ConnTabId, id: TabId, show: bool) {
        if show {
            self.close_editor(tab, id, true);
        }
        if let Some(object) = self.object_tab_mut(tab, id) {
            object.edits.reviewing = show && !object.edits.cells.is_empty();
            if !object.edits.reviewing {
                object.edits.review = None;
            }
        }
    }

    /// Makes the Review SQL of every table tab that shows it and whose
    /// pending set changed since it was made, and closes the review of a
    /// tab with nothing pending any more. Once per batch of actions, as
    /// the row panel's text is formatted: a frame that changes no set
    /// builds no statement, and a tab whose review is closed builds none
    /// at all.
    pub(super) fn make_reviews(&mut self) {
        let stale: Vec<(ConnTabId, TabId)> = self
            .open_connections()
            .flat_map(|(tab, workspace)| {
                workspace
                    .object_tabs()
                    .filter(|object| object.edits.reviewing && object.edits.review.is_none())
                    .map(move |object| (tab, object.id))
            })
            .collect();
        for (tab, id) in stale {
            let review = self
                .table(tab, id, |table, object| {
                    crate::review::build(&object.object, table, &object.edits.cells, Values::Shown)
                })
                .flatten();
            if let Some(object) = self.object_tab_mut(tab, id) {
                object.edits.reviewing = review.is_some();
                object.edits.review = review;
            }
        }
    }

    /// The tab's Review SQL with every value whole: what the clipboard
    /// gets. Made when it is asked for, and kept nowhere: a value can be a
    /// quarter of a megabyte, and the review that is drawn holds sixty
    /// characters of each.
    pub fn review_whole(&self, tab: ConnTabId, id: TabId) -> Option<crate::review::Review> {
        self.table(tab, id, |table, object| {
            crate::review::build(&object.object, table, &object.edits.cells, Values::Whole)
        })
        .flatten()
    }

    /// A save was answered: `Event::Written`.
    pub(super) fn written(
        &mut self,
        session: SessionId,
        request: RequestId,
        result: Result<WriteOutcome, Error>,
    ) {
        // A dialog the user is in is never replaced: a conflict that arrives
        // under one is a line, as a failure is.
        let free = self.dialog.is_none();
        // An answer for a closed tab, or for a save a reconnect gave up,
        // finds no tab saving.
        let Some(tab) = self.tab_for_session(session) else {
            return;
        };
        let Some(workspace) = self.workspace_mut(tab) else {
            return;
        };
        let Some(object) = workspace.object_tabs_mut().find(|object| {
            object
                .edits
                .saving
                .as_ref()
                .is_some_and(|saving| saving.request == request)
        }) else {
            return;
        };
        let id = object.id;
        let Some(mut saving) = object.edits.saving.take() else {
            return;
        };
        // What was held for this save goes on only when everything was
        // written: every other outcome drops it.
        let then = saving.then.take();
        // The page's row of the set's row `index`. None for a row the save
        // did not send: no row is named for want of it.
        let place = |index: usize| saving.rows.get(index).copied();
        match result {
            Ok(WriteOutcome::Written { rows, elapsed }) => {
                let counts = object.edits.counts();
                let cells = object
                    .edits
                    .cells
                    .keys()
                    .map(|&(row, col)| CellPos { row, col })
                    .collect();
                // The rows an earlier save found gone are still gone.
                object.edits.discard();
                object.fields = None;
                let fits = object.rows.value.as_ref().is_some_and(|page| {
                    rows.len() == saving.rows.len()
                        && rows.iter().all(|row| row.len() == page.columns.len())
                        && saving.rows.iter().all(|&at| at < page.rows.len())
                });
                if fits {
                    if let Some(page) = object.rows.value.as_mut() {
                        for (row, &at) in rows.into_iter().zip(&saving.rows) {
                            page.rows[at] = row;
                        }
                    }
                    object.edits.saved = Some(Saved {
                        at: std::time::Instant::now(),
                        cells,
                        changes: counts.changes,
                        rows: counts.rows,
                        elapsed,
                    });
                } else {
                    // The table is not the one the page was read from:
                    // read it again.
                    self.fetch_rows(tab, id);
                }
                // Written either way, and the set is empty: what was held
                // passes the guard.
                if let Some(held) = then {
                    self.perform(held);
                }
            }
            Ok(WriteOutcome::Conflicts(conflicts)) => {
                // What the tab says where the rows cannot be asked about.
                let line = conflicts.first().map(|first| match place(first.row) {
                    Some(row) => Note::Conflict {
                        row,
                        gone: first.server.is_none(),
                        others: conflicts.len() - 1,
                    },
                    // An answer about a row that was not sent is not one
                    // to tell the save's end by.
                    None => Note::Lost,
                });
                let asked = object.rows.value.as_ref().and_then(|page| {
                    let rows = conflicting(&saving.rows, conflicts, page).filter(|_| free)?;
                    // None when the save named no row: nothing to ask.
                    let lines = shown_lines(page, &object.edits.cells, rows.first()?);
                    Some((rows, lines))
                });
                match asked {
                    // The question says what the line would.
                    Some((rows, lines)) => {
                        self.dialog = Some(Dialog::Conflict(Box::new(ConflictPrompt {
                            tab,
                            id,
                            rows,
                            at: 0,
                            lines,
                            shown: std::time::Instant::now(),
                            fresh: true,
                            overwrite: false,
                            kept: false,
                        })));
                    }
                    None => object.edits.note = line,
                }
            }
            Ok(WriteOutcome::Failed { row, error }) => {
                object.edits.fail(place(row), error);
                // The row panel says what stands against a cell.
                object.fields = None;
            }
            Err(error) => {
                object.edits.note = Some(if error.is_connection_lost() {
                    Note::Lost
                } else if error == Error::Cancelled {
                    Note::Cancelled
                } else {
                    Note::Refused(error)
                });
            }
        }
    }

    /// Answers the question about the row a save found changed, and asks
    /// about the next. `at` is the row the answer was given for: one for
    /// another row than the one asked about is dropped, a click or a key a
    /// frame behind. The answer is applied at once, to the page and to the
    /// set, so each row is whole whatever comes of the rows after it.
    pub(super) fn answer_conflict(&mut self, at: usize, answer: Answer) {
        // The kind is checked before the dialog is taken: another dialog
        // is not closed by it.
        if !matches!(&self.dialog, Some(Dialog::Conflict(prompt)) if prompt.at == at) {
            return;
        }
        let Some(Dialog::Conflict(mut prompt)) = self.dialog.take() else {
            return;
        };
        let (tab, id) = (prompt.tab, prompt.id);
        let Some(conflict) = prompt.rows.get_mut(at) else {
            // Nothing is left to ask about.
            return;
        };
        let (row, gone) = (conflict.row, conflict.server.is_none());
        if !answer.offered(gone) {
            // Not an answer the question about this row has.
            self.dialog = Some(Dialog::Conflict(prompt));
            return;
        }
        // The row is answered once: what the server holds goes into the
        // page, or nowhere.
        let server = conflict.server.take();
        // The cells the answer takes out of the set: every one of the row,
        // or those whose new value is what the server holds now.
        let all = matches!(answer, Answer::UseServer | Answer::Discard);
        let same = match &server {
            Some(server) if !all => self
                .table(tab, id, |table, object| {
                    settled(table, &object.edits.cells, row, server)
                })
                .unwrap_or_default(),
            _ => Vec::new(),
        };
        let Some(object) = self.object_tab_mut(tab, id) else {
            // The tab went: there is no row to ask about.
            return;
        };
        if let Some(server) = server
            && let Some(loaded) = object
                .rows
                .value
                .as_mut()
                .and_then(|page| page.rows.get_mut(row))
        {
            *loaded = server;
        }
        object
            .edits
            .cells
            .retain(|&(of, col), _| of != row || !(all || same.contains(&col)));
        match answer {
            Answer::Discard => {
                object.edits.gone.insert(row);
            }
            // The row's changes stay pending on a row that is not there.
            // The tab says so, with the line a conflict is where it is not
            // asked about: the row is not left looking like any other.
            Answer::KeepMine if gone => {
                object.edits.note = Some(Note::Conflict {
                    row,
                    gone: true,
                    others: 0,
                });
            }
            Answer::KeepMine | Answer::UseServer | Answer::Overwrite => {}
        }
        // The row panel shows the row as it is now. So does the review,
        // once it is made again: it was made of the cells that were
        // pending and of what the page had loaded under them.
        object.fields = None;
        object.edits.review = None;
        match answer {
            Answer::KeepMine => prompt.kept = true,
            Answer::Overwrite => prompt.overwrite = true,
            Answer::UseServer | Answer::Discard => {}
        }
        prompt.at += 1;
        if let Some(next) = prompt.rows.get(prompt.at) {
            // The next row's question is a new one on screen.
            let object = self
                .workspace(tab)
                .and_then(|workspace| workspace.object_tab(id));
            prompt.lines = object
                .and_then(|object| Some(shown_lines(object.page()?, &object.edits.cells, next)))
                .unwrap_or_default();
            prompt.shown = std::time::Instant::now();
            prompt.fresh = true;
            self.dialog = Some(Dialog::Conflict(prompt));
            return;
        }
        // Every row is answered. A save writes the whole set, so it runs
        // again only when a row was to be overwritten and none was kept to
        // look at again: that one would be written with it.
        if !prompt.overwrite || prompt.kept {
            return;
        }
        let pending = self
            .workspace(tab)
            .and_then(|workspace| workspace.object_tab(id))
            .is_some_and(|object| !object.edits.cells.is_empty());
        if !pending {
            return;
        }
        // The session went while the question was up: the tab says that
        // nothing went out, as it does after a confirmation.
        if self.save_blocked(tab, id) == Some(SaveBlock::Disconnected) {
            if let Some(object) = self.object_tab_mut(tab, id) {
                object.edits.note = Some(Note::NotSent);
            }
            return;
        }
        // As any save: checked, and on production confirmed again with the
        // statements it would send now. What the first save was to be
        // followed by went with its conflict.
        self.write_as_answer(tab, id, None);
    }

    /// Saves as the answer to another dialog asks for it: Save in the Leave
    /// prompt, Overwrite in the conflict question. The caller has taken that
    /// dialog away. Where the save is confirmed first, the confirmation
    /// comes up in the place of the dialog that was answered, under the
    /// hand that answered it, so it takes no answer in its first moment.
    pub(super) fn write_as_answer(&mut self, tab: ConnTabId, id: TabId, then: Option<Held>) {
        self.write_edits(tab, id, then);
        if let Some(Dialog::ConfirmWrite(prompt)) = &mut self.dialog {
            prompt.after_answer = Some(std::time::Instant::now());
        }
    }

    /// The conflict question was closed without an answer for the row it
    /// showed. That row and the rows after it are as they were loaded, with
    /// their changes pending, and the tab says so as it does where the
    /// question is not asked at all: the next save asks again.
    pub(super) fn conflict_unanswered(&mut self, prompt: &ConflictPrompt) {
        let Some(shown) = prompt.rows.get(prompt.at) else {
            return;
        };
        let note = Note::Conflict {
            row: shown.row,
            gone: shown.server.is_none(),
            others: prompt.rows.len() - prompt.at - 1,
        };
        if let Some(object) = self.object_tab_mut(prompt.tab, prompt.id) {
            object.edits.note = Some(note);
        }
    }
}
