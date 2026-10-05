//! The reducer's part in editing a table's values: the guard that keeps a
//! page with pending changes, the editor, the pending set and the save.
//! What all of it is decided from is in `crate::edit`.

use tabletist_db::{Access, ChangeSet, ColumnClass, Error, NewValue, WriteOutcome, column_class};

use super::App;
use crate::backend::{Command, RequestId, SessionId};
use crate::edit::{
    Editor, Edits, Lock, Note, Pending, Problem, Saved, Saving, State, Table, change_set, check,
    is_change, opens_large, same_changes, start_text,
};
use crate::model::{
    Action, CellPos, ConnTabId, Dialog, EditStart, Held, LeavePrompt, ObjectTab, Pane, SaveBlock,
    SessionStatus, TabId, WritePrompt,
};

/// Whether `action` is dropped while a prompt about pending changes is up:
/// what edits, saves or discards, what moves the selection (it closes an
/// editor; of any tab, nothing is told apart), and what takes the dialog's
/// place whatever dialog it is, and with it what the prompt holds. The
/// prompts' own answers, `CloseDialog` and what the backend says are not
/// among them. Nor is `EditorTyped`: it changes no text, and a keystroke
/// that shares its frame with the key that raised the prompt must be noted,
/// or the prompt's Save would close the editor without it.
pub(super) fn dropped_under_a_prompt(action: &Action) -> bool {
    matches!(
        action,
        Action::EditCell { .. }
            | Action::EditorBreak { .. }
            | Action::CommitEdit { .. }
            | Action::LeaveEdit { .. }
            | Action::CancelEdit { .. }
            | Action::SetNull { .. }
            | Action::RevertCell { .. }
            | Action::DiscardEdits { .. }
            | Action::WriteEdits { .. }
            | Action::SelectCell { .. }
            | Action::MoveSelection { .. }
            | Action::NewConnection
            | Action::EditConnection(_)
            // These two take the dialog before they look at its kind.
            | Action::CancelPassword
            | Action::TrustHostKey
    )
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
        // An open editor with nothing pending yet still counts as one.
        let changes = tabs
            .iter()
            .filter_map(edits)
            .map(|edits| edits.counts().changes.max(1))
            .sum();
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
        if self.save_blocked(tab, id).is_some() {
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
            // builder's reason.
            let shown: Result<Vec<String>, (usize, Error)> = changes
                .rows
                .iter()
                .enumerate()
                .map(
                    |(index, row)| match dialect.update_row(&changes.object, row) {
                        Ok(update) => Ok(update.shown),
                        Err(error) => Err((index, error)),
                    },
                )
                .collect();
            let statements = match shown {
                Ok(statements) => statements,
                Err((index, error)) => {
                    if let Some(object) = self.object_tab_mut(tab, id) {
                        object.edits.saved = None;
                        object.edits.fail(rows.get(index).copied(), error);
                    }
                    return;
                }
            };
            let cells = changes.rows.iter().map(|row| row.set.len()).sum();
            self.dialog = Some(Dialog::ConfirmWrite(Box::new(WritePrompt {
                tab,
                id,
                statements,
                changes: cells,
                rows: rows.len(),
                typed: String::new(),
                focus: true,
                changeset: changes,
                then,
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

    pub(super) fn edit_cell(&mut self, tab: ConnTabId, id: TabId, cell: CellPos, start: EditStart) {
        // An editor open on another cell keeps its text.
        self.close_editor(tab, id, true);
        let (asked, touched) = match &start {
            EditStart::Value => (true, false),
            EditStart::Replace(_) => (true, true),
            EditStart::Typed(_) => (false, true),
        };
        let opened = self.table(tab, id, |table, object| {
            if let Some(lock) = table.lock(cell) {
                return Err(lock);
            }
            let class = table.class(cell.col).unwrap_or(ColumnClass::Other);
            let text = match start {
                EditStart::Replace(text) | EditStart::Typed(text) => text,
                EditStart::Value => match object.edits.cells.get(&(cell.row, cell.col)) {
                    Some(pending) => match &pending.new {
                        NewValue::Text(text) => text.clone(),
                        NewValue::Null => String::new(),
                    },
                    None => start_text(&table.page.rows[cell.row][cell.col], class),
                },
            };
            Ok(Editor {
                cell,
                large: opens_large(&text, class),
                text,
                focus: true,
                touched,
                problem: None,
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
                // A tab being edited is no preview to replace.
                object.pinned = true;
            }
            Err(Lock::NoSuchCell) => return,
            // Typing on a cell that cannot be edited does nothing.
            Err(_) if !asked => return,
            Err(lock) => {
                object.selection = Some(cell);
                object.edits.why = Some((cell, lock));
            }
        }
        if let Some(workspace) = self.workspace_mut(tab) {
            workspace.pane = Pane::Grid;
        }
    }

    /// Takes the open editor's text as its cell's new value and closes it.
    /// A text its column does not take keeps the editor open, unless the
    /// edit is `left` (the keyboard went elsewhere): then the text is kept
    /// as a cell to fix, so typing is never lost. Says whether the editor
    /// closed.
    pub(super) fn close_editor(&mut self, tab: ConnTabId, id: TabId, left: bool) -> bool {
        let verdict = self.table(tab, id, |table, object| {
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
            Some((cell, new, changed, problem))
        });
        let Some(object) = self.object_tab_mut(tab, id) else {
            return false;
        };
        let Some(Some((cell, new, changed, problem))) = verdict else {
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
            object.edits.cells.insert(key, Pending { new, state });
        } else {
            object.edits.cells.remove(&key);
        }
        // The row panel shows the pending value.
        object.fields = None;
        true
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
            object.edits.cells.insert(
                key,
                Pending {
                    new: NewValue::Null,
                    state: State::Ready,
                },
            );
            // As opening an editor does: a tab with a pending cell is no
            // preview for the next single click to replace.
            object.pinned = true;
        } else {
            object.edits.cells.remove(&key);
        }
        object.fields = None;
    }

    /// Runs what the terminal's `:` prompt holds, and closes it: `w` saves
    /// the pending changes of the table on screen and `e!` drops them, as
    /// their keys do in the other looks. Any other text is not a command,
    /// and is kept for the status line to say so (`diff` too, until Review
    /// SQL is there to show).
    pub(super) fn run_command(&mut self, tab: ConnTabId) {
        let Some(workspace) = self.workspace_mut(tab) else {
            return;
        };
        workspace.focus_command = false;
        workspace.command_error = None;
        let Some(text) = workspace.command.take() else {
            return;
        };
        let table = workspace.active_object_tab().map(|object| object.id);
        let action = match text.trim() {
            "" => return,
            "w" => table.map(|id| Action::WriteEdits { tab, id }),
            "e!" => table.map(|id| Action::DiscardEdits { tab, id }),
            other => {
                workspace.command_error = Some(other.to_owned());
                return;
            }
        };
        // As from a key: under a question about the changes it is dropped.
        if let Some(action) = action {
            self.apply(action);
        }
    }

    /// A save was answered: `Event::Written`.
    pub(super) fn written(
        &mut self,
        session: SessionId,
        request: RequestId,
        result: Result<WriteOutcome, Error>,
    ) {
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
                object.edits = Edits::default();
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
                object.edits.note = conflicts.first().map(|first| match place(first.row) {
                    Some(row) => Note::Conflict {
                        row,
                        gone: first.server.is_none(),
                        others: conflicts.len() - 1,
                    },
                    // An answer about a row that was not sent is not one
                    // to tell the save's end by.
                    None => Note::Lost,
                });
            }
            Ok(WriteOutcome::Failed { row, error }) => {
                object.edits.fail(place(row), error);
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
}
