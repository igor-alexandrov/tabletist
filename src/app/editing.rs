//! The reducer's part in editing a table's values: the guard that keeps a
//! page with pending changes, the editor, the pending set and the save.
//! What all of it is decided from is in `crate::edit`.

use tabletist_db::{Access, ChangeSet, ColumnClass, Error, NewValue, WriteOutcome, column_class};

use super::App;
use crate::backend::{Command, RequestId, SessionId};
use crate::edit::{
    Answer, Duplicate, Editor, EditorPlace, Effect, Holder, Lock, NEW_ROWS, Note, Order, Pending,
    Place, Problem, Saved, Saving, Sent, State, Sweep, Table, Taken, Told, change_set, check,
    conflicting, is_change, new_id, new_row, opens_large, same_changes, settled, shown_lines,
    start_text,
};
use crate::model::{
    Action, Advance, CellPos, ConflictPrompt, ConnTabId, Dialog, EditStart, Held, LeavePrompt,
    ObjectTab, ObjectView, Pane, Reach, SaveBlock, SessionStatus, TabId, WritePrompt,
};
use crate::review::{Part, Values};

/// Whether `action` is dropped while a prompt about pending changes is up:
/// what edits, saves or discards, what moves the selection (it closes an
/// editor; of any tab, nothing is told apart), what shows or hides Review
/// SQL (shown, it closes an editor as a left edit, and the set is another
/// than the one asked about), and what takes the dialog's place whatever
/// dialog it is, and with it what the prompt holds. The
/// prompts' own answers, `CloseDialog` and what the backend says are not
/// among them. Nor is `EditorTyped`: it changes no text, and a keystroke
/// that shares its frame with the key that raised the prompt must be noted,
/// or the prompt's Save would close the editor without it. Nor is
/// `YankRow`: it changes nothing that is pending.
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
            | Action::PickerMove { .. }
            | Action::PickOption { .. }
            | Action::PickerNull { .. }
            | Action::SetNull { .. }
            | Action::SetFieldNull { .. }
            | Action::SetDefault { .. }
            | Action::CycleBoolean { .. }
            | Action::SetBoolean { .. }
            | Action::SetNow { .. }
            | Action::AddRow { .. }
            | Action::DuplicateRow { .. }
            | Action::PutRow { .. }
            | Action::DropRow { .. }
            | Action::DeleteRow { .. }
            | Action::DeleteSelected { .. }
            | Action::DeleteCount { .. }
            | Action::RestoreRow { .. }
            | Action::RevertCell { .. }
            | Action::DiscardEdits { .. }
            | Action::WriteEdits { .. }
            | Action::ReviewEdits { .. }
            | Action::SelectCell { .. }
            | Action::ExtendTo { .. }
            | Action::ExtendBy { .. }
            | Action::SelectRows { .. }
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

/// Drops the new rows `new` of `object`, which an action named among
/// `named` (the grid's order, before the drop). The cursor and the range's
/// anchor, where theirs went, go to the nearest row of `named` that is
/// left; with none left, the range ends and the cursor is on the row that
/// stands where its own stood. Says how many were dropped.
fn drop_named(object: &mut ObjectTab, named: &[usize], new: &[usize]) -> usize {
    if new.is_empty() {
        return 0;
    }
    let height = object.page().map_or(0, |page| page.rows.len());
    // Where the cursor's row stood, for when no row named is left.
    let stood = object
        .selection
        .and_then(|cell| Order::of(&object.edits.added, height).place(cell.row));
    let dropped = new
        .iter()
        .filter_map(|row| new_id(*row))
        .filter(|id| object.edits.drop_row(*id))
        .count();
    // The rows named that are left, each with its place among them.
    let left: Vec<(usize, usize)> = named
        .iter()
        .copied()
        .enumerate()
        .filter(|(_, row)| !new.contains(row))
        .collect();
    let nearest = |row: usize| {
        let at = named.iter().position(|named| *named == row)?;
        let near = left.iter().min_by_key(|(place, _)| place.abs_diff(at));
        near.map(|(_, row)| *row)
    };
    if let Some(anchor) = object.range.map(|range| range.anchor)
        && new.contains(&anchor)
    {
        match (nearest(anchor), object.range.as_mut()) {
            (Some(row), Some(range)) => range.anchor = row,
            _ => object.range = None,
        }
    }
    if let Some(cell) = object.selection.filter(|cell| new.contains(&cell.row)) {
        let row = nearest(cell.row).or_else(|| {
            let order = Order::of(&object.edits.added, height);
            order.row(stood?.min(order.len().saturating_sub(1)))
        });
        let cell = row.map(|row| CellPos { row, col: cell.col });
        object.set_cursor(cell, Reach::Keep);
    }
    dropped
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
    // A new row's cell loaded nothing: a text typed into it is set, the
    // empty one too. A cell that is neither a page's nor a new row's is
    // no cell any more.
    let loaded = table.loaded(cell);
    if loaded.is_none() && new_id(cell.row).is_none() {
        return None;
    }
    if !editor.touched {
        return None;
    }
    // What was typed, as the column reads it: a keyword where it is one.
    let new = crate::edit::typed_as(table.dialect, column, &editor.text);
    let changed = match (&new, loaded) {
        // A new row's unset cell is its default already.
        (NewValue::Default { .. }, None) => false,
        (new, loaded) => loaded.is_none_or(|loaded| is_change(loaded, new, class)),
    };
    let problem = changed
        .then(|| crate::edit::problem(table.dialect, column, &editor.text))
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
    pub(super) fn table<T>(
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
        // On a row of the page only: what is typed into a new row is part
        // of that row, which is counted as one whatever it holds.
        let closed = self
            .table(tab, id, typed)
            .flatten()
            .filter(|typed| new_id(typed.cell.row).is_none());
        let pending = closed
            .as_ref()
            .is_some_and(|typed| edits.cells.contains_key(&(typed.cell.row, typed.cell.col)));
        let changed = closed.as_ref().map(|typed| typed.changed);
        let counts = edits.counts();
        let changes = match (changed, pending) {
            (Some(true), false) => counts.changes + 1,
            (Some(false), true) => counts.changes.saturating_sub(1),
            _ => counts.changes,
        };
        (changes + counts.added + counts.deleted).max(1)
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
        // A new row that lacks a value a save needs.
        let lacking = self.table(tab, id, |table, object| {
            let cells = &object.edits.cells;
            let mut added = object.edits.added.iter();
            added.any(|new| !table.missing(new.id, cells).is_empty())
        });
        if lacking == Some(true) {
            return Some(SaveBlock::Required);
        }
        if !matches!(workspace.status, SessionStatus::Connected) {
            return Some(SaveBlock::Disconnected);
        }
        if workspace.access == Access::ReadOnly {
            return Some(SaveBlock::ReadOnly);
        }
        // Something is pending and no change set comes of it: a Save that
        // would do nothing is not offered.
        let unsendable = object.edits.pending()
            && self
                .table(tab, id, |table, object| {
                    change_set(&object.object, table, &object.edits.cells)
                })
                .flatten()
                .is_none();
        unsendable.then_some(SaveBlock::Unsendable)
    }

    /// The columns the tab's new rows still need a value in, by name, each
    /// once, in the page's order: what the bar and the terminal's line say
    /// a save waits for.
    pub fn lacking(&self, tab: ConnTabId, id: TabId) -> Vec<String> {
        self.table(tab, id, |table, object| {
            let added = object.edits.added.iter();
            let mut cols: Vec<usize> = added
                .flat_map(|new| table.missing(new.id, &object.edits.cells))
                .collect();
            cols.sort_unstable();
            cols.dedup();
            cols.into_iter()
                .filter_map(|col| table.page.columns.get(col))
                .map(|column| crate::ui::format::display_safe(&column.name).into_owned())
                .collect()
        })
        .unwrap_or_default()
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
            .is_some_and(|object| !object.edits.pending());
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
        let Some((changes, sent)) = built else {
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
            let blocked = crate::review::Blocked::default();
            let mut review = crate::review::of(dialect, &changes, blocked, Values::Shown);
            // What was counted already, before the review goes to the
            // panel and to the prompt: the two are then the same review.
            let counted = self
                .workspace(tab)
                .and_then(|workspace| workspace.object_tab(id))
                .and_then(|object| object.edits.taken.as_ref())
                .map(Taken::lines);
            if let Some(lines) = counted {
                review.also(&lines);
            }
            if let Some((part, error)) = review.refused.clone() {
                if let Some(object) = self.object_tab_mut(tab, id) {
                    object.edits.saved = None;
                    match part {
                        Part::Row(index) => object.edits.fail(sent.rows.get(index).copied(), error),
                        Part::Insert(index) => {
                            let new = sent.inserts.get(index).copied();
                            object.edits.fail_insert(new, error);
                        }
                        Part::Delete(index) => {
                            let row = sent.deletes.get(index).copied();
                            object.edits.fail_delete(row, error);
                        }
                    }
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
                rows: sent.rows.len(),
                added: changes.inserts.len(),
                deleted: changes.deletes.len(),
                typed: String::new(),
                focus: true,
                changeset: changes,
                then,
                after_answer: None,
            })));
            // What the deletes take with them, where it was not counted
            // yet: the prompt says it as the answers come.
            self.count_taken(tab, id);
            return;
        }
        self.send_write(tab, id, changes, sent, then);
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
        if let Some((changes, sent)) = now
            && same_changes(&changes, &changeset)
        {
            self.send_write(tab, id, changes, sent, then);
        }
    }

    fn send_write(
        &mut self,
        tab: ConnTabId,
        id: TabId,
        changes: ChangeSet,
        sent: Sent,
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
        // A delete the last save was refused is sent again: its error is
        // that save's, and would stand before what this one comes to.
        object.edits.resend_deletes();
        object.edits.saving = Some(Saving {
            request,
            rows: sent.rows,
            inserts: sent.inserts,
            deletes: sent.deletes,
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
            crate::edit::problem(table.dialect, column, &editor.text)
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

    /// The field Up or Down steps to from the row panel's field of the
    /// column `from`: the one `by` after it as the panel lists the selected
    /// row's fields, or the last that way. `None` while the tab has no
    /// page.
    pub(super) fn field_beside(
        &self,
        tab: ConnTabId,
        id: TabId,
        from: usize,
        by: isize,
    ) -> Option<usize> {
        self.table(tab, id, |table, object| {
            let last = table.page.columns.len().checked_sub(1)?;
            // With no row selected the panel lists none: the page's order.
            let row = object.selection.map_or(0, |cell| cell.row);
            let fields = table.fields(row, &object.edits.cells);
            let Some(at) = fields.iter().position(|col| *col == from) else {
                return Some(from.saturating_add_signed(by).min(last));
            };
            fields.get(at.saturating_add_signed(by).min(last)).copied()
        })
        .flatten()
    }

    /// The field a commit in the row panel's field of the column `col`
    /// walks to: the selected row's next that can be edited, in the order
    /// the panel lists its fields, or the one before it. None at the row's
    /// end, and for a commit that stays.
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
            let free = |col: &&usize| table.lock(CellPos { row, col: **col }).is_none();
            let fields = table.fields(row, &object.edits.cells);
            let at = fields.iter().position(|field| *field == col)?;
            if forward {
                fields[at + 1..].iter().find(free).copied()
            } else {
                fields[..at].iter().rev().find(free).copied()
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
                        // As a pending NULL: nothing of it is text
                        // to go on from.
                        NewValue::Null | NewValue::Default { .. } | NewValue::Now => String::new(),
                    },
                    // A new row's cell starts from nothing.
                    None => table
                        .loaded(cell)
                        .map(|loaded| start_text(loaded, class))
                        .unwrap_or_default(),
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
                .and_then(|column| crate::edit::problem(table.dialect, column, &text));
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
                picker: None,
                select: false,
            })
        });
        let Some(opened) = opened else {
            return;
        };
        let Some(object) = self.object_tab_mut(tab, id) else {
            return;
        };
        let is_open = opened.is_ok();
        match opened {
            Ok(editor) => {
                object.set_cursor(Some(cell), Reach::End);
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
                object.set_cursor(Some(cell), Reach::End);
                object.edits.why = Some((cell, lock));
                object.edits.why_place = place;
            }
        }
        if let Some(workspace) = self.workspace_mut(tab) {
            workspace.pane = Pane::Grid;
        }
        if is_open {
            self.open_picker(tab, id, from_value);
        }
    }

    /// Whether the tab's table takes a new row now: its rows show, and
    /// nothing stands against a row (`Table::no_rows`).
    fn takes_rows(&self, tab: ConnTabId, id: TabId) -> bool {
        let takes = self.table(tab, id, |table, object| {
            object.view == ObjectView::Data && table.no_rows().is_none()
        });
        takes == Some(true)
    }

    /// What follows a new row put into the set, for its cell `cell`: the
    /// editor on the cell where it is to `open`, else the selection there
    /// and the keys with the grid.
    fn settle_new_row(&mut self, tab: ConnTabId, id: TabId, cell: CellPos, open: bool) {
        if let Some(object) = self.object_tab_mut(tab, id) {
            object.edits.why = None;
            object.fields = None;
            // A tab with a new row is no preview to replace.
            object.pinned = true;
        }
        if open {
            self.edit_cell(tab, id, cell, EditStart::Value, EditorPlace::Grid);
            return;
        }
        if let Some(object) = self.object_tab_mut(tab, id) {
            object.set_cursor(Some(cell), Reach::End);
            object.focus_field = None;
        }
        if let Some(workspace) = self.workspace_mut(tab) {
            workspace.pane = Pane::Grid;
            workspace.save_refused = false;
            workspace.review_refused = false;
        }
    }

    /// Adds a new row to the tab's table at `place`, where it takes one,
    /// and opens the editor on the first cell a save needs a value in.
    /// With no such cell the selection goes to the first that takes a
    /// value, and nothing opens: nothing is asked of the user there.
    pub(super) fn add_row(&mut self, tab: ConnTabId, id: TabId, place: Place) {
        // An editor open on another cell keeps its text.
        self.close_editor(tab, id, true);
        if !self.takes_rows(tab, id) {
            return;
        }
        let Some(object) = self.object_tab_mut(tab, id) else {
            return;
        };
        let height = object.page().map_or(0, |page| page.rows.len());
        let new = object.edits.add_row(place, height);
        let row = new_row(new);
        let found = self.table(tab, id, |table, object| {
            let required = table.missing(new, &object.edits.cells).into_iter().next();
            let free = || {
                let mut cols = 0..table.page.columns.len();
                cols.find(|&col| table.lock(CellPos { row, col }).is_none())
            };
            (required, required.or_else(free))
        });
        let (required, col) = found.unwrap_or((None, None));
        let cell = CellPos {
            row,
            col: col.unwrap_or(0),
        };
        self.settle_new_row(tab, id, cell, required.is_some());
    }

    /// Adds a copy of the selected row as a new row at the top.
    pub(super) fn duplicate_row(&mut self, tab: ConnTabId, id: TabId) {
        // Before the row is read: an open editor's text is part of what
        // the row shows, and closed it is pending.
        self.close_editor(tab, id, true);
        let copy = self.table(tab, id, |table, object| {
            let yank = table.yank(object.selection?.row, &object.edits.cells)?;
            Some(table.duplicate(&yank))
        });
        if let Some(Some(copy)) = copy {
            self.put_copy(tab, id, copy, Place::Top);
        }
    }

    /// Keeps what the selected row shows on the tab, where its rows show.
    /// No editor is closed for it, as `duplicate_row` closes one: the keys
    /// that yank are not read while an editor has them.
    pub(super) fn yank_row(&mut self, tab: ConnTabId, id: TabId) {
        let yank = self.table(tab, id, |table, object| {
            if object.view != ObjectView::Data {
                return None;
            }
            table.yank(object.selection?.row, &object.edits.cells)
        });
        if let (Some(Some(yank)), Some(object)) = (yank, self.object_tab_mut(tab, id)) {
            object.yanked = Some(yank);
        }
    }

    /// Adds a copy of the tab's yanked row right below the selected row,
    /// or right above it. With no row selected, at the top. With nothing
    /// yanked, nothing: an open editor stays open too.
    pub(super) fn put_row(&mut self, tab: ConnTabId, id: TabId, above: bool) {
        // The copy is made from the yank where it is kept, not from a
        // clone of it: a yanked value can be megabytes.
        let read = self.table(tab, id, |table, object| {
            let yank = object.yanked.as_ref()?;
            Some((table.duplicate(yank), object.selection))
        });
        let Some(Some((copy, selection))) = read else {
            return;
        };
        // An editor open on another cell keeps its text: it is pending
        // before a row is put under it.
        self.close_editor(tab, id, true);
        let place = match (selection, above) {
            (Some(cell), false) => Place::Below(cell.row),
            (Some(cell), true) => Place::Above(cell.row),
            (None, _) => Place::Top,
        };
        self.put_copy(tab, id, copy, place);
    }

    /// Adds `copy` as a new row at `place`, where the table takes one, and
    /// settles it. The editor opens on the first cell a save needs a value
    /// in and that takes one. With none the selection goes to the copy, in
    /// the column the cursor was in. Both are the copy's own rules: an
    /// added row opens on its first required cell whatever it is, and
    /// otherwise selects the first cell that takes a value.
    fn put_copy(&mut self, tab: ConnTabId, id: TabId, copy: Duplicate, place: Place) {
        if !self.takes_rows(tab, id) {
            return;
        }
        let Some(object) = self.object_tab_mut(tab, id) else {
            return;
        };
        let stood = object.selection.map_or(0, |cell| cell.col);
        let height = object.page().map_or(0, |page| page.rows.len());
        let new = object.edits.add_copy(place, height, copy);
        let row = new_row(new);
        let open = self.table(tab, id, |table, object| {
            table.first_to_fill(new, &object.edits.cells)
        });
        let open = open.flatten();
        let cell = CellPos {
            row,
            col: open.unwrap_or(stood),
        };
        self.settle_new_row(tab, id, cell, open.is_some());
    }

    /// Drops the selected row where it is a new one. The selection stays
    /// where the row stood, on the row that stands there now.
    pub(super) fn drop_row(&mut self, tab: ConnTabId, id: TabId) {
        let Some(object) = self.object_tab_mut(tab, id) else {
            return;
        };
        // Not under a save: its answer names the new rows it sent.
        if object.edits.saving.is_some() {
            return;
        }
        let Some(cell) = object.selection else {
            return;
        };
        let Some(new) = new_id(cell.row) else {
            return;
        };
        let height = object.page().map_or(0, |page| page.rows.len());
        let stood = Order::of(&object.edits.added, height).place(cell.row);
        if !object.edits.drop_row(new) {
            return;
        }
        object.fields = None;
        let order = Order::of(&object.edits.added, height);
        let last = order.len().saturating_sub(1);
        let stays = stood
            .and_then(|place| order.row(place.min(last)))
            .map(|row| CellPos { row, col: cell.col });
        // The rows selected with it stay selected, unless they began on
        // the row that went.
        if object.range.is_some_and(|range| range.anchor == cell.row) {
            object.range = None;
        }
        object.set_cursor(stays, Reach::Keep);
    }

    /// Deletes the selected row: drops it where it is a new one, marks it
    /// for deletion where it is a row of the page, and restores it where
    /// it is marked. A row that cannot be deleted stays as it is: the
    /// keys and the button say why before they ask.
    pub(super) fn delete_row(&mut self, tab: ConnTabId, id: TabId) {
        let selected = self
            .workspace(tab)
            .and_then(|workspace| workspace.object_tab(id))
            .and_then(|object| object.selection);
        let Some(cell) = selected else {
            return;
        };
        if new_id(cell.row).is_some() {
            return self.drop_row(tab, id);
        }
        let marked = self
            .workspace(tab)
            .and_then(|workspace| workspace.object_tab(id))
            .is_some_and(|object| object.edits.deleted.contains_key(&cell.row));
        if marked {
            return self.restore(tab, id, cell.row);
        }
        // The text being typed is part of the row's edits, which are kept
        // under the mark.
        self.close_editor(tab, id, true);
        let free = self.table(tab, id, |table, _| table.no_delete(cell.row).is_none());
        if free != Some(true) {
            return;
        }
        let Some(object) = self.object_tab_mut(tab, id) else {
            return;
        };
        // The counts of the rows that were marked are of another set now.
        let stale = object.edits.counts_out();
        if !object.edits.mark(cell.row) {
            return;
        }
        let row = crate::ui::pending_bar::row_name(object, cell.row);
        let deleted = object.edits.deleted.len();
        object.edits.told = Some(Told::Marked { row, deleted });
        // A tab that holds a change stays, as it does for a cell's.
        object.pinned = true;
        // What the last save said was of another set.
        object.edits.note = None;
        // The row panel shows the row as marked.
        object.fields = None;
        self.stop_counts(tab, stale);
    }

    /// Deletes the selected rows. One row is [`Self::delete_row`]'s. A
    /// visual selection is left once its rows were acted on, the cursor on
    /// the first of them that is left.
    pub(super) fn delete_selected(&mut self, tab: ConnTabId, id: TabId) {
        let found = self
            .workspace(tab)
            .and_then(|workspace| workspace.object_tab(id))
            .map(|object| (object.selected_rows(), object.visual()));
        let Some((rows, visual)) = found else {
            return;
        };
        let acted = if rows.len() < 2 {
            self.delete_row(tab, id);
            true
        } else {
            self.sweep_rows(tab, id, &rows)
        };
        if !(visual && acted) {
            return;
        }
        let Some(object) = self.object_tab_mut(tab, id) else {
            return;
        };
        // A row of the page is there still, and a new row where the mark
        // did not drop it.
        let left = |row: &&usize| match new_id(**row) {
            Some(new) => object.edits.added.iter().any(|added| added.id == new),
            None => true,
        };
        let first = rows.iter().find(left).copied();
        let col = object.selection.map_or(0, |cell| cell.col);
        match first {
            Some(row) => object.set_cursor(Some(CellPos { row, col }), Reach::End),
            // None of them is left: the cursor is where the drop put it.
            None => object.range = None,
        }
    }

    /// Deletes the cursor's row and the rows after it, `count` in all
    /// ([`ObjectTab::rows_from_cursor`]). A range that stands is ended,
    /// and the cursor goes to the row after the last one named, or stays
    /// on that row where the grid ends with it.
    pub(super) fn delete_count(&mut self, tab: ConnTabId, id: TabId, count: u32) {
        let found = self
            .workspace(tab)
            .and_then(|workspace| workspace.object_tab(id))
            .map(|object| {
                let rows = object.rows_from_cursor(count);
                let height = object.page().map_or(0, |page| page.rows.len());
                let order = Order::of(&object.edits.added, height);
                // The row after the last one named: no action names it, so
                // it is there afterwards.
                let after = rows
                    .last()
                    .and_then(|last| order.place(*last))
                    .and_then(|place| order.row(place + 1));
                (rows, after)
            });
        let Some((rows, after)) = found else {
            return;
        };
        let acted = match rows.len() {
            0 => false,
            1 => {
                self.delete_row(tab, id);
                true
            }
            _ => self.sweep_rows(tab, id, &rows),
        };
        if !acted {
            return;
        }
        let Some(object) = self.object_tab_mut(tab, id) else {
            return;
        };
        let last = rows.last().copied().filter(|row| match new_id(*row) {
            Some(new) => object.edits.added.iter().any(|added| added.id == new),
            None => true,
        });
        let col = object.selection.map_or(0, |cell| cell.col);
        match after.or(last) {
            Some(row) => object.set_cursor(Some(CellPos { row, col }), Reach::End),
            // The grid ended with new rows that were dropped: the cursor
            // is where the drop put it.
            None => object.range = None,
        }
    }

    /// Deletes `rows` in one action, as [`Table::sweep`] has it: marks
    /// them and drops the new ones among them, or restores them. A row is
    /// marked and restored as one row is. Says whether it did any of it.
    fn sweep_rows(&mut self, tab: ConnTabId, id: TabId, rows: &[usize]) -> bool {
        // The text being typed is part of its row's edits, which are kept
        // under the mark.
        self.close_editor(tab, id, true);
        let Some(sweep) = self.table(tab, id, |table, _| table.sweep(rows)) else {
            return false;
        };
        let told = match sweep {
            // The view said why.
            Sweep::TooMany | Sweep::Refused(_) => return false,
            Sweep::Restore { rows } => {
                for row in &rows {
                    self.restore(tab, id, *row);
                }
                Told::RestoredRows {
                    restored: rows.len(),
                }
            }
            Sweep::Mark {
                rows: marks,
                new,
                skipped,
            } => {
                let Some(object) = self.object_tab_mut(tab, id) else {
                    return false;
                };
                // The counts of the rows that were marked are of another
                // set now.
                let stale = object.edits.counts_out();
                let marked = marks.iter().filter(|row| object.edits.mark(**row)).count();
                let dropped = drop_named(object, rows, &new);
                let deleted = object.edits.deleted.len();
                self.stop_counts(tab, stale);
                Told::MarkedRows {
                    marked,
                    skipped,
                    dropped,
                    deleted,
                }
            }
            Sweep::Drop { new } => {
                let Some(object) = self.object_tab_mut(tab, id) else {
                    return false;
                };
                let dropped = drop_named(object, rows, &new);
                Told::MarkedRows {
                    marked: 0,
                    skipped: 0,
                    dropped,
                    deleted: object.edits.deleted.len(),
                }
            }
        };
        let Some(object) = self.object_tab_mut(tab, id) else {
            return false;
        };
        object.edits.told = Some(told);
        // A tab that holds a change stays, as it does for a cell's.
        object.pinned = true;
        // What the last save said was of another set.
        object.edits.note = None;
        // The row panel shows the cursor's row as it is now.
        object.fields = None;
        true
    }

    /// Stops counts a tab no longer waits for.
    pub(super) fn stop_counts(&mut self, tab: ConnTabId, requests: Vec<RequestId>) {
        if let Some(session) = self.workspace(tab).map(|workspace| workspace.session) {
            self.cancel(session, requests);
        }
    }

    /// Takes the mark for deletion off the selected row.
    pub(super) fn restore_row(&mut self, tab: ConnTabId, id: TabId) {
        let selected = self
            .workspace(tab)
            .and_then(|workspace| workspace.object_tab(id))
            .and_then(|object| object.selection);
        if let Some(cell) = selected {
            self.restore(tab, id, cell.row);
        }
    }

    /// Takes the mark for deletion off the page's row `row`.
    pub(super) fn restore(&mut self, tab: ConnTabId, id: TabId, row: usize) {
        let Some(object) = self.object_tab_mut(tab, id) else {
            return;
        };
        // Not under a save: its answer names the rows it sent.
        let stale = object.edits.counts_out();
        if object.edits.saving.is_some() || !object.edits.restore(row) {
            return;
        }
        // The row's cells that came back without a warning: a question
        // about one may have been out when the row was marked, and was
        // dropped with the mark.
        let unasked: Vec<CellPos> = object
            .edits
            .cells
            .range((row, 0)..=(row, usize::MAX))
            .filter(|(at, _)| !object.edits.doubts.contains_key(at))
            .map(|(&(row, col), _)| CellPos { row, col })
            .collect();
        let name = crate::ui::pending_bar::row_name(object, row);
        object.edits.told = Some(Told::Restored { row: name });
        object.edits.note = None;
        object.fields = None;
        self.stop_counts(tab, stale);
        // Asked of a foreign key's typed value only: every other cell has
        // nothing to ask.
        for cell in unasked {
            self.ask_row(tab, id, cell);
        }
    }

    /// Shows the row panel and asks it to give the keyboard to the selected
    /// row's first field that can be edited, as the panel lists them. No
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
            let fields = table.fields(cell.row, &object.edits.cells);
            match fields.into_iter().find(free) {
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
        // Whether the text is the key of an option taken from the
        // picker's list: read before the editor goes.
        let taken = self
            .editor_mut(tab, id)
            .and_then(|editor| editor.picker.as_ref())
            .is_some_and(|picker| picker.taken);
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
        // What the cell's doubt comes to, as one ordered rule:
        // 1. not changed, or a problem: the cell has no doubt;
        // 2. taken from the picker's list: no doubt, nothing asked (the
        //    row was just read);
        // 3. a doubt of this very text stands: it is kept, nothing asked;
        // 4. the cell already held this very text, ready, before this
        //    close: nothing asked. It was asked when it was kept, or taken
        //    from the list, or copied, and an editor reopened and closed
        //    with no change is no new value;
        // 5. otherwise the text is a new typed key: asked.
        let same = |value: &NewValue| match (value, &new) {
            (NewValue::Text(held), NewValue::Text(text)) => held == text,
            (NewValue::Null | NewValue::Text(_) | NewValue::Default { .. } | NewValue::Now, _) => {
                false
            }
        };
        let known = object
            .edits
            .doubts
            .get(&key)
            .is_some_and(|doubt| matches!(&new, NewValue::Text(text) if doubt.text == *text));
        let held = object
            .edits
            .cells
            .get(&key)
            .is_some_and(|held| held.state == State::Ready && same(&held.new));
        let doubtful = changed && problem.is_none() && !taken;
        let keep = doubtful && known;
        let ask = doubtful && !known && !held;
        if changed {
            let state = problem.map_or(State::Ready, State::ToFix);
            object.edits.put(key, Pending { new, state });
        } else {
            object.edits.revert(key);
        }
        // The row panel shows the pending value.
        object.fields = None;
        // `put` keeps a doubt of the same text, which a taken value must
        // not keep, and a question the cell had was of the value before.
        if !keep {
            object.edits.doubts.remove(&key);
        }
        if ask {
            // A typed key: does it name a row?
            self.ask_row(tab, id, cell);
        }
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

    /// Makes the selected cell NULL, where its column takes one. Where it
    /// takes none the cell says why, at `place`: where it was asked for.
    pub(super) fn set_null(&mut self, tab: ConnTabId, id: TabId, place: EditorPlace) {
        let verdict = self.table(tab, id, |table, object| {
            let cell = object.selection?;
            if object.edits.editor.is_some() || table.lock(cell).is_some() {
                return None;
            }
            let column = table.column(cell.col)?;
            if !column.nullable {
                // Refused, and the cell says why.
                return Some((cell, Err(Lock::NotNull)));
            }
            // A new row's cell loaded nothing: NULL is a value set in it.
            let changed = table.loaded(cell).is_none_or(|loaded| !loaded.is_null());
            Some((cell, Ok(changed)))
        });
        let Some(Some((cell, verdict))) = verdict else {
            return;
        };
        let Some(object) = self.object_tab_mut(tab, id) else {
            return;
        };
        let changed = match verdict {
            Ok(changed) => changed,
            Err(lock) => {
                object.edits.why = Some((cell, lock));
                object.edits.why_place = place;
                return;
            }
        };
        object.edits.why = None;
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

    /// Gives the selected cell its column's default, where the column has
    /// one. In a new row that is the cell with nothing set in it.
    pub(super) fn set_default(&mut self, tab: ConnTabId, id: TabId) {
        let verdict = self.table(tab, id, |table, object| {
            let cell = object.selection?;
            if object.edits.editor.is_some() || table.lock(cell).is_some() {
                return None;
            }
            let column = table.column(cell.col)?;
            let expression = column.default.clone()?;
            // A new row's unset cell is its default already.
            let new = table.loaded(cell).map(|_| NewValue::Default {
                expression: Some(expression),
            });
            Some((cell, new))
        });
        let Some(Some((cell, new))) = verdict else {
            return;
        };
        let Some(object) = self.object_tab_mut(tab, id) else {
            return;
        };
        let key = (cell.row, cell.col);
        match new {
            Some(new) => {
                let state = State::Ready;
                object.edits.put(key, Pending { new, state });
                // As opening an editor does: a tab with a pending cell is
                // no preview for the next single click to replace.
                object.pinned = true;
            }
            None => object.edits.revert(key),
        }
        object.edits.why = None;
        object.fields = None;
    }

    /// Whether the tab's selected cell is a boolean's that can be edited
    /// now: where Space flips the cell.
    pub fn flips(&self, tab: ConnTabId, id: TabId) -> bool {
        self.table(tab, id, |table, object| {
            let Some(cell) = object.selection else {
                return false;
            };
            object.view == ObjectView::Data
                && object.edits.editor.is_none()
                && table.class(cell.col) == Some(ColumnClass::Boolean)
                && table.lock(cell).is_none()
        })
        .unwrap_or(false)
    }

    /// Flips the selected boolean cell: true, then false, then NULL where
    /// the column takes it, then true again. A value that is what the cell
    /// loaded is no change.
    pub(super) fn cycle_boolean(&mut self, tab: ConnTabId, id: TabId) {
        if !self.flips(tab, id) {
            return;
        }
        let next = self.table(tab, id, |table, object| {
            let cell = object.selection?;
            let column = table.column(cell.col)?;
            let loaded = table.loaded(cell);
            // What the cell holds now: what is pending in it, or what it
            // loaded. Anything that is no flag is taken for none.
            let flag = |text: &str| match text.trim().to_ascii_lowercase().as_str() {
                "true" | "1" => Some(true),
                "false" | "0" => Some(false),
                _ => None,
            };
            let class = ColumnClass::Boolean;
            let now = match object.edits.cells.get(&(cell.row, cell.col)) {
                Some(pending) => match &pending.new {
                    NewValue::Text(text) => flag(text),
                    NewValue::Null | NewValue::Default { .. } | NewValue::Now => None,
                },
                None => loaded.and_then(|loaded| flag(&start_text(loaded, class))),
            };
            let new = match now {
                Some(true) => NewValue::Text("false".into()),
                Some(false) if column.nullable => NewValue::Null,
                Some(false) | None => NewValue::Text("true".into()),
            };
            let changed = loaded.is_none_or(|loaded| is_change(loaded, &new, class));
            Some((cell, new, changed))
        });
        let Some(Some((cell, new, changed))) = next else {
            return;
        };
        self.put_value(tab, id, cell, new, changed);
    }

    /// Makes `new` the pending value of `cell`, or takes what was pending
    /// there out where `new` is no change of what the cell loaded.
    fn put_value(
        &mut self,
        tab: ConnTabId,
        id: TabId,
        cell: CellPos,
        new: NewValue,
        changed: bool,
    ) {
        let Some(object) = self.object_tab_mut(tab, id) else {
            return;
        };
        let key = (cell.row, cell.col);
        if changed {
            let state = State::Ready;
            object.edits.put(key, Pending { new, state });
            // As opening an editor does: a tab with a pending cell is no
            // preview for the next single click to replace.
            object.pinned = true;
        } else {
            object.edits.revert(key);
        }
        object.edits.why = None;
        object.fields = None;
    }

    /// Sets the selected boolean cell to `value`: the terminal's `t` and
    /// `f`. A value that is what the cell loaded is no change.
    pub(super) fn set_boolean(&mut self, tab: ConnTabId, id: TabId, value: bool) {
        if !self.flips(tab, id) {
            return;
        }
        let set = self.table(tab, id, |table, object| {
            let cell = object.selection?;
            let new = NewValue::Text(value.to_string());
            let loaded = table.loaded(cell);
            let changed = loaded.is_none_or(|loaded| is_change(loaded, &new, ColumnClass::Boolean));
            Some((cell, new, changed))
        });
        if let Some(Some((cell, new, changed))) = set {
            self.put_value(tab, id, cell, new, changed);
        }
    }

    /// The cell of the tab's open editor, where it is one of a column that
    /// keeps a date or a time: where "now" is offered, and what it sets.
    pub fn stamps(&self, tab: ConnTabId, id: TabId) -> Option<CellPos> {
        self.table(tab, id, |table, object| {
            let cell = object.edits.editor.as_ref()?.cell;
            let column = table.column(cell.col)?;
            tabletist_db::temporal(&column.type_name).map(|_| cell)
        })
        .flatten()
    }

    /// Gives the cell of the open editor the time of the save, where its
    /// column keeps a date or a time. The editor closes without its text:
    /// the value is the database's to make.
    pub(super) fn set_now(&mut self, tab: ConnTabId, id: TabId) {
        let Some(cell) = self.stamps(tab, id) else {
            return;
        };
        let field = self.panel_field(tab, id);
        if let Some(object) = self.object_tab_mut(tab, id) {
            object.edits.editor = None;
        }
        self.put_value(tab, id, cell, NewValue::Now, true);
        // In the panel the keyboard goes back to the field that was edited.
        if let Some(col) = field {
            self.back_to_field(tab, id, col);
        }
    }

    /// Runs what the terminal's `:` prompt holds, and closes it. Which
    /// line does what is the keymap's to say, for the tab in front: on a
    /// table's, `w` saves its pending changes, `e!` drops them and `diff`
    /// shows what a save would run, as their keys and the bar's buttons do
    /// in the other looks; on a SQL editor's, `ro` and `rw` set the mode of
    /// its runs. Any other text is not a command there, and is
    /// kept for the status line to say so: a line of another kind of tab,
    /// and one whose command is not built yet, too.
    pub(super) fn run_command(&mut self, tab: ConnTabId) {
        use crate::keymap::{Command, Layout, Scope, Typed};
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
        let line = text.trim();
        if line.is_empty() {
            return;
        }
        // The table on screen, and whether anything is pending in it. With
        // none, the SQL editor in front.
        let table = workspace
            .active_object_tab()
            .map(|object| (object.id, object.edits.pending()));
        let sql_tab = workspace.active_sql_tab().map(|sql| sql.id);
        let scope = match table {
            Some(_) => Scope::Grid,
            None => Scope::SqlEditor,
        };
        // The prompt is Omarchy's, in whatever look it is driven.
        let read = self.keymap.ex(Layout::Omarchy, scope, &|_| false, line);
        let refused = |app: &mut Self| {
            if let Some(workspace) = app.workspace_mut(tab) {
                workspace.command_error = Some(line.to_owned());
            }
        };
        let action = match (read, table) {
            (Typed::Command(Command::SaveChanges), Some((id, _))) => Action::WriteEdits { tab, id },
            (Typed::Command(Command::DiscardChanges), Some((id, _))) => {
                Action::DiscardEdits { tab, id }
            }
            (Typed::Command(Command::ReviewSql), Some((id, true))) => Action::ReviewEdits {
                tab,
                id,
                show: true,
            },
            // Nothing to show: the line says so, where a panel that opened
            // empty would say it less plainly.
            (Typed::Command(Command::ReviewSql), Some((_, false))) => {
                if let Some(workspace) = self.workspace_mut(tab) {
                    workspace.review_refused = true;
                }
                return;
            }
            // A SQL editor's lines set the mode of its runs, as its switch
            // does. Where the tab cannot write the mode stays, as it does
            // under the switch.
            (Typed::Command(command @ (Command::ReadWriteTab | Command::ReadOnlyTab)), None) => {
                let Some(sql_tab) = sql_tab else {
                    return refused(self);
                };
                let mode = if command == Command::ReadWriteTab {
                    crate::model::RunMode::ReadWrite
                } else {
                    crate::model::RunMode::ReadOnly
                };
                Action::SetSqlMode { tab, sql_tab, mode }
            }
            _ => return refused(self),
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
            object.edits.reviewing = show && object.edits.pending();
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
                // What was counted since the marks last changed is the
                // review's too.
                let lines = object.edits.taken.as_ref().map(Taken::lines);
                if let (Some(review), Some(lines)) = (object.edits.review.as_mut(), lines) {
                    review.also(&lines);
                }
            }
            // And what was not is asked now.
            self.count_taken(tab, id);
        }
    }

    /// The tab's Review SQL with every value whole: what the clipboard
    /// gets. Made when it is asked for, and kept nowhere: a value can be a
    /// quarter of a megabyte, and the review that is drawn holds sixty
    /// characters of each.
    pub fn review_whole(&self, tab: ConnTabId, id: TabId) -> Option<crate::review::Review> {
        self.table(tab, id, |table, object| {
            let mut review =
                crate::review::build(&object.object, table, &object.edits.cells, Values::Whole)?;
            // What the deletes take with them is copied as it is shown.
            if let Some(taken) = &object.edits.taken {
                review.also(&taken.lines());
            }
            Some(review)
        })
        .flatten()
    }

    /// Asks how many rows the tab's deletes take with them in other
    /// tables, where rows are marked, a key that acts points at the table,
    /// and nothing was asked since the marks last changed. The answers
    /// come as counts (`taken_counted`).
    pub(super) fn count_taken(&mut self, tab: ConnTabId, id: TabId) {
        let asked = self
            .table(tab, id, |table, object| {
                let unasked = object.edits.taken.is_none();
                unasked.then(|| crate::edit::taken_by(table))
            })
            .flatten()
            .unwrap_or_default();
        if asked.is_empty() {
            return;
        }
        let Some(session) = self.workspace(tab).map(|workspace| workspace.session) else {
            return;
        };
        let mut sent = Vec::with_capacity(asked.len());
        for (effect, queries) in asked {
            let mut requests = Vec::with_capacity(queries.len());
            for query in queries {
                let request = RequestId(self.next_id());
                self.backend.send(Command::CountRows {
                    session,
                    request,
                    query,
                });
                requests.push(request);
            }
            sent.push((effect, requests));
        }
        let taken = Taken::asked(sent);
        let lines = taken.lines();
        if let Some(object) = self.object_tab_mut(tab, id) {
            object.edits.taken = Some(taken);
            if let Some(review) = object.edits.review.as_mut() {
                review.also(&lines);
            }
        }
        self.also_in_prompt(tab, id, &lines);
    }

    /// The production confirmation of the tab `id`, where one is up: its
    /// review says what the deletes take too.
    fn also_in_prompt(&mut self, tab: ConnTabId, id: TabId, lines: &[Effect]) {
        if let Some(Dialog::ConfirmWrite(prompt)) = self.dialog.as_mut()
            && prompt.tab == tab
            && prompt.id == id
        {
            prompt.review.also(lines);
        }
    }

    /// A count came back: one of a tab's counts of what its deletes take,
    /// where it is one. `rows` is `None` where the count failed. Says
    /// whether it was one of them.
    pub(super) fn taken_counted(
        &mut self,
        session: SessionId,
        request: RequestId,
        rows: Option<u64>,
    ) -> bool {
        let Some(tab) = self.tab_for_session(session) else {
            return false;
        };
        let Some(workspace) = self.workspace_mut(tab) else {
            return false;
        };
        let found = workspace.object_tabs_mut().find_map(|object| {
            let taken = object.edits.taken.as_mut()?;
            if !taken.answer(request, rows) {
                return None;
            }
            let lines = taken.lines();
            if let Some(review) = object.edits.review.as_mut() {
                review.also(&lines);
            }
            Some((object.id, lines))
        });
        match found {
            Some((id, lines)) => {
                self.also_in_prompt(tab, id, &lines);
                true
            }
            None => false,
        }
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
        // The row a delete was refused for, to ask who holds it once the
        // tab is let go of.
        let mut refused = None;
        match result {
            Ok(WriteOutcome::Written {
                inserted,
                rows,
                gone,
                elapsed,
            }) => {
                let counts = object.edits.counts();
                // The rows the save found already deleted, by their keys,
                // while the page still holds them.
                let already: Vec<String> = gone
                    .iter()
                    .filter_map(|&index| saving.deletes.get(index))
                    .map(|&row| crate::ui::pending_bar::row_name(object, row))
                    .collect();
                // The changed cells, by the page's rows they were of. A
                // row marked for deletion has none among them.
                let changed: Vec<CellPos> = object
                    .edits
                    .cells
                    .range(..(NEW_ROWS, 0))
                    .map(|(&(row, col), _)| CellPos { row, col })
                    .collect();
                // Where the new rows stood: the set is dropped next.
                let added = std::mem::take(&mut object.edits.added);
                // The rows an earlier save found gone are still gone.
                object.edits.discard();
                object.fields = None;
                // Written, and these were not there to delete: said
                // whether the page is patched or read again.
                let gone_before = already.len();
                if !already.is_empty() {
                    object.edits.note = Some(Note::AlreadyDeleted(already));
                }
                // A key of the table that points at the table and acts on
                // a delete: the database deleted rows the save did not
                // name, or cleared their key, and some may be on this
                // page. Only a read says which.
                let reaches_its_own = !saving.deletes.is_empty()
                    && object.structure.value.as_ref().is_some_and(|structure| {
                        structure.referenced_by.iter().any(|key| {
                            key.schema == object.object.schema
                                && key.table.eq_ignore_ascii_case(&object.object.name)
                                && key.acts().is_some()
                        })
                    });
                let fits = object.rows.value.as_ref().is_some_and(|page| {
                    let wide = |row: &Vec<tabletist_db::Value>| row.len() == page.columns.len();
                    !reaches_its_own
                        && rows.len() == saving.rows.len()
                        && rows.iter().all(wide)
                        && saving.rows.iter().all(|&at| at < page.rows.len())
                        && saving.deletes.iter().all(|&at| at < page.rows.len())
                        // Every new row is known, and is one the set held.
                        && inserted.len() == saving.inserts.len()
                        && inserted.iter().all(|row| row.as_ref().is_some_and(wide))
                        && saving.inserts.len() == added.len()
                        && saving
                            .inserts
                            .iter()
                            .all(|id| added.iter().any(|new| new.id == *id))
                });
                let page = object.rows.value.as_mut().filter(|_| fits);
                if let Some(page) = page {
                    for (row, &at) in rows.into_iter().zip(&saving.rows) {
                        page.rows[at] = row;
                    }
                    // Each new row takes its place in the page: the rows
                    // are the page's from here on, in the order they were
                    // shown, so a row's number is where it was shown.
                    let order = Order::of(&added, page.rows.len());
                    let doomed: std::collections::BTreeSet<usize> =
                        saving.deletes.iter().copied().collect();
                    // The rows as they stand once the save is done, in the
                    // order they were shown, without the deleted ones: a
                    // row's number is where it was shown among them.
                    let kept: Vec<usize> = (0..order.len())
                        .filter_map(|place| order.row(place))
                        .filter(|row| !doomed.contains(row))
                        .collect();
                    let stored = inserted.into_iter().flatten();
                    let mut stored: std::collections::BTreeMap<usize, Vec<tabletist_db::Value>> =
                        saving.inserts.iter().copied().zip(stored).collect();
                    let mut loaded: Vec<Option<Vec<tabletist_db::Value>>> =
                        std::mem::take(&mut page.rows)
                            .into_iter()
                            .map(Some)
                            .collect();
                    page.rows = kept
                        .iter()
                        .filter_map(|&row| match new_id(row) {
                            Some(id) => stored.remove(&id),
                            None => loaded.get_mut(row)?.take(),
                        })
                        .collect();
                    let width = page.columns.len();
                    let places: std::collections::BTreeMap<usize, usize> = kept
                        .iter()
                        .enumerate()
                        .map(|(place, &row)| (row, place))
                        .collect();
                    let now = |row: usize| places.get(&row).copied();
                    // What the save wrote, where it is now: each changed
                    // cell, and every cell of a new row.
                    let mut cells: Vec<CellPos> = changed
                        .into_iter()
                        .filter_map(|cell| {
                            let row = now(cell.row)?;
                            Some(CellPos { row, col: cell.col })
                        })
                        .collect();
                    for &id in &saving.inserts {
                        if let Some(row) = now(new_row(id)) {
                            cells.extend((0..width).map(|col| CellPos { row, col }));
                        }
                    }
                    // By row, then column: the grid finds a cell by halving.
                    cells.sort_unstable_by_key(|cell| (cell.row, cell.col));
                    let gone = std::mem::take(&mut object.edits.gone);
                    object.edits.gone = gone.into_iter().filter_map(now).collect();
                    let moved = object.selection.and_then(|cell| {
                        let row = match now(cell.row) {
                            Some(row) => row,
                            // Its row was deleted: the row that stood
                            // after it, or the last where it was the last.
                            None => {
                                let stood = order.place(cell.row)?;
                                let before = kept
                                    .iter()
                                    .filter(|&&row| order.place(row).is_some_and(|at| at < stood))
                                    .count();
                                before.min(kept.len().checked_sub(1)?)
                            }
                        };
                        Some(CellPos { row, col: cell.col })
                    });
                    // The rows are others now: a range was of the page
                    // the save found.
                    object.set_cursor(moved, Reach::End);
                    // The table has that many rows more and that many
                    // fewer, where it is known how many it had. A row
                    // found already deleted was counted when the page was
                    // read.
                    let more = saving.inserts.len() as u64;
                    let fewer = saving.deletes.len() as u64;
                    if let Some(count) = object.count.value.as_mut() {
                        *count = (*count + more).saturating_sub(fewer);
                    }
                    if let Some(estimate) = object.estimated_rows.as_mut() {
                        *estimate = (*estimate + more).saturating_sub(fewer);
                    }
                    object.edits.saved = Some(Saved {
                        at: std::time::Instant::now(),
                        cells,
                        changes: counts.changes,
                        rows: counts.rows,
                        added: counts.added,
                        deleted: saving.deletes.len().saturating_sub(gone_before),
                        elapsed,
                    });
                } else {
                    // The table is not the one the page was read from, a
                    // new row came back unknown (a table with a trigger,
                    // MySQL without a key to find it by), or a delete
                    // reached rows the save did not name: read it again. A
                    // new row is nowhere until the page says where.
                    object.off_new_rows();
                    // How many rows the table lost is not known either.
                    if reaches_its_own {
                        self.reset_count(tab, id);
                    }
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
            Ok(WriteOutcome::FailedInsert { insert, error }) => {
                let new = saving.inserts.get(insert).copied();
                object.edits.fail_insert(new, error);
                // The row panel says what stands against a cell.
                object.fields = None;
            }
            Ok(WriteOutcome::FailedDelete { delete, error }) => {
                let row = saving.deletes.get(delete).copied();
                object.edits.fail_delete(row, error);
                // The row panel says what stands against the row.
                object.fields = None;
                refused = row;
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
        if let Some(row) = refused {
            self.count_holders(tab, id, row);
        }
    }

    /// Asks who holds the page's row `row`, whose delete was refused: a
    /// count for each key that could be the one, now that the save is
    /// undone. Nothing is asked where the refusal was no foreign key's.
    fn count_holders(&mut self, tab: ConnTabId, id: TabId, row: usize) {
        let asked = self
            .table(tab, id, |table, object| {
                let error = object.edits.deleted.get(&row)?.failed.as_ref()?;
                Some(crate::edit::held_by(table, row, error))
            })
            .flatten()
            .unwrap_or_default();
        let Some(session) = self.workspace(tab).map(|workspace| workspace.session) else {
            return;
        };
        let mut out = Vec::with_capacity(asked.len());
        for (holder, query) in asked {
            let request = RequestId(self.next_id());
            self.backend.send(Command::CountRows {
                session,
                request,
                query,
            });
            out.push((request, row, holder));
        }
        if let Some(object) = self.object_tab_mut(tab, id) {
            object.edits.holders = out;
        }
    }

    /// A count came back: one of a refused delete's, where it is one. The
    /// first key that holds rows is the row's holder. Says whether it was
    /// one of them.
    pub(super) fn held_counted(
        &mut self,
        session: SessionId,
        request: RequestId,
        rows: Option<u64>,
    ) -> bool {
        let Some(tab) = self.tab_for_session(session) else {
            return false;
        };
        let Some(workspace) = self.workspace_mut(tab) else {
            return false;
        };
        for object in workspace.object_tabs_mut() {
            let edits = &mut object.edits;
            let mut out = edits.holders.iter();
            let Some(at) = out.position(|(asked, ..)| *asked == request) else {
                continue;
            };
            let (_, row, holder) = edits.holders.remove(at);
            // Only of the refusal it was asked about: a row that was sent
            // again since has no error for this to explain.
            if let Some(rows) = rows.filter(|rows| *rows > 0)
                && let Some(doomed) = edits.deleted.get_mut(&row)
                && doomed.failed.is_some()
                && doomed.holder.is_none()
            {
                doomed.holder = Some(Holder { rows, ..holder });
                // The row panel says it under the row's line.
                object.fields = None;
            }
            return true;
        }
        false
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
            .is_some_and(|object| object.edits.pending());
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
