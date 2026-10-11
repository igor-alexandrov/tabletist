//! The reducer's part in a foreign key cell's picker: it opens with the
//! editor, sends its searches and reads their answers. What it decides is
//! in `crate::key_picker`.

use std::time::{Duration, Instant};

use tabletist_db::{ColumnClass, Error, LookupPage, LookupPart, ObjectRef, Value};

use super::App;
use crate::backend::{Command, RequestId, SessionId};
use crate::edit::{Picked, Table, start_text};
use crate::key_picker::{self, Answered, Asked, Columns, Picker, Take};
use crate::model::{Action, Advance, ConnTabId, TabId};

/// A lookup in flight, and the picker it was sent for.
#[derive(Debug, Clone, Copy)]
pub struct SentLookup {
    pub session: SessionId,
    pub request: RequestId,
    /// `Picker::serial`.
    pub picker: u64,
    /// The read of the matches: the one a closed editor cancels. The
    /// pinned row's read is one row by its key, and is left to end.
    pub matches: bool,
}

/// What a commit takes from the editor's picker.
pub(super) enum Taken {
    /// Nothing yet: the commit waits for the list.
    Wait,
    /// The text as typed, or there is no picker.
    Text,
    /// An option's key, which is the editor's text now.
    Option(Picked),
}

impl App {
    /// Gives the tab's editor, which just opened, its picker, where its
    /// cell is a foreign key's and the editor is the small field, on the
    /// cell or in the row panel. `from_value` says it opened on the cell's value, not on
    /// something typed. The referenced table's columns are asked for if
    /// the workspace does not know them, and the first search is sent as
    /// soon as they are known.
    pub(super) fn open_picker(&mut self, tab: ConnTabId, id: TabId, from_value: bool) {
        let terminal = self.look.terminal;
        let made = self.workspace(tab).and_then(|workspace| {
            let object = workspace.object_tab(id)?;
            let table = Table::of(workspace, object)?;
            let editor = object.edits.editor.as_ref()?;
            // The picker belongs to the small field, on the cell or in
            // the row panel: the large editor has no list.
            if editor.large {
                return None;
            }
            let column = table.column(editor.cell.col)?;
            let target = key_picker::target(table.structure?, &column.name)?;
            let class = table.class(editor.cell.col)?;
            // The row the cell points at now: what it loaded, or the
            // pending text the editor opened on.
            let pending = object
                .edits
                .cells
                .contains_key(&(editor.cell.row, editor.cell.col));
            let current = match (from_value, pending) {
                (false, _) => None,
                (true, true) => {
                    let text = editor.text.trim();
                    key_picker::pinned(text, class, &column.type_name, editor.problem.is_none())
                }
                // Pinned only where it compares exactly, as a typed text
                // is: a key that loaded as text because it is too wide or
                // a decimal is found by the matches alone.
                (true, false) => table.loaded(editor.cell).and_then(|value| match value {
                    Value::Int(_) => Some(value.clone()),
                    Value::Text(text) => key_picker::pinned(text, class, &column.type_name, true),
                    _ => None,
                }),
            };
            let kept = workspace.columns.get(&target.0);
            let label = match kept.and_then(|kept| kept.value.as_deref()) {
                Some(columns) => {
                    Columns::Known(key_picker::label_column(table.dialect, columns, &target.1))
                }
                None => Columns::Waiting,
            };
            // A fetch that failed is asked for again: the completion list
            // asks once per keystroke and so lets a failure stand, while
            // this asks once per editor opened. A cached error is never
            // the picker's failure at opening: a refresh that failed
            // leaves columns that are known, and the list works with them,
            // and columns that are not known are asked for again, so the
            // old failure is not the new fetch's. `picker_columns` gives
            // the picker the failure if the retry fails too.
            let load = kept.is_none_or(|kept| kept.value.is_none() && !kept.is_loading());
            let picker = Picker::new(target, label, current, !from_value, &editor.text);
            Some((picker, load))
        });
        let Some((mut picker, load)) = made else {
            return;
        };
        picker.serial = self.next_id();
        let table = picker.target.clone();
        if let Some(editor) = self.editor_mut(tab, id) {
            editor.picker = Some(picker);
            // The terminal look keeps its own rule: `i` goes on from the
            // text's end.
            editor.select = from_value && !terminal;
        }
        if load {
            self.load_columns(tab, table);
        }
        self.search(tab, id);
    }

    /// Sends the search for what the tab's editor holds: the pinned row's
    /// read, where the text names one, then the matches'. Cancels
    /// nothing: a search that more typing overtook is left to end, and
    /// its answer is dropped as not the picker's. Nothing is sent while
    /// the referenced table's columns are not known.
    pub(super) fn search(&mut self, tab: ConnTabId, id: TabId) {
        let planned = self.workspace(tab).and_then(|workspace| {
            let object = workspace.object_tab(id)?;
            let table = Table::of(workspace, object)?;
            let editor = object.edits.editor.as_ref()?;
            let picker = editor.picker.as_ref()?;
            let Columns::Known(label) = &picker.label else {
                return None;
            };
            let column = table.column(editor.cell.col)?;
            let class = table.class(editor.cell.col)?;
            let text = picker.text(&editor.text).to_owned();
            let pin = if !picker.searching {
                picker.current.clone()
            } else {
                let passes = crate::edit::problem(table.dialect, column, &text).is_none();
                key_picker::pinned(&text, class, &column.type_name, passes)
            };
            let reads = picker.lookups(label.clone(), &text, pin);
            Some((workspace.session, reads, text, picker.serial))
        });
        let Some((session, (pinned, matching), text, serial)) = planned else {
            return;
        };
        let pinned = pinned.map(|lookup| self.send_lookup(session, lookup, serial));
        let matching = Some(self.send_lookup(session, matching, serial));
        if let Some(picker) = self
            .editor_mut(tab, id)
            .and_then(|editor| editor.picker.as_mut())
        {
            picker.asked = Some(Asked {
                text,
                pinned,
                matching,
                first: None,
            });
            picker.due = None;
        }
    }

    fn send_lookup(
        &mut self,
        session: SessionId,
        lookup: tabletist_db::Lookup,
        picker: u64,
    ) -> RequestId {
        let request = RequestId(self.next_id());
        self.lookups.push(SentLookup {
            session,
            request,
            picker,
            matches: matches!(lookup.part, LookupPart::Matching(_)),
        });
        self.backend.send(Command::Lookup {
            session,
            request,
            lookup,
        });
        request
    }

    /// The columns of a table came, or failed to: a picker of `tab` that
    /// waited for them learns its label column and sends its first
    /// search, or says why it has no list.
    pub(super) fn picker_columns(&mut self, tab: ConnTabId) {
        let shown = self.active_workspace_tab();
        let Some(workspace) = self.workspace_mut(tab) else {
            return;
        };
        let dialect = workspace.driver.dialect();
        let mut ready = Vec::new();
        let mut commits = Vec::new();
        // Split, so the tabs and the cache are borrowed side by side.
        let columns = std::mem::take(&mut workspace.columns);
        for object in workspace.object_tabs_mut() {
            let id = object.id;
            let Some(picker) = object
                .edits
                .editor
                .as_mut()
                .and_then(|editor| editor.picker.as_mut())
                .filter(|picker| picker.label == Columns::Waiting)
            else {
                continue;
            };
            let Some(kept) = columns.get(&picker.target) else {
                continue;
            };
            if let Some(known) = kept.value.as_deref() {
                picker.label =
                    Columns::Known(key_picker::label_column(dialect, known, &picker.key));
                ready.push(id);
            } else if let Some(error) = &kept.error {
                picker.failed = Some(error.clone());
                // Nothing will answer: a kept Enter takes the text. Under
                // a prompt about pending changes the commit is dropped
                // (`App::apply`) and the key is lost: accepted, the next
                // Enter takes at once. So is the key of a tab that is not
                // the one shown: nothing moves behind the user's back.
                if let Some(then) = picker.kept.take()
                    && shown == Some((tab, id))
                {
                    commits.push((id, then));
                }
            }
        }
        workspace.columns = columns;
        for id in ready {
            self.search(tab, id);
        }
        for (id, then) in commits {
            self.apply(Action::CommitEdit { tab, id, then });
        }
    }

    /// A lookup answered. The picker that waits for it takes it; one that
    /// nothing waits for any more is dropped.
    pub(super) fn lookup_answered(
        &mut self,
        session: SessionId,
        request: RequestId,
        result: Result<LookupPage, Error>,
    ) {
        self.lookups.retain(|sent| sent.request != request);
        let Some(tab) = self.tab_for_session(session) else {
            return;
        };
        let shown = self.active_workspace_tab();
        let Some(workspace) = self.workspace_mut(tab) else {
            return;
        };
        let mut resend = None;
        let mut commit = None;
        for object in workspace.object_tabs_mut() {
            let id = object.id;
            let Some(crate::edit::Editor {
                text,
                picker: Some(picker),
                ..
            }) = object.edits.editor.as_mut()
            else {
                continue;
            };
            // `result` moves into `answered`: the picker that does not
            // wait for the request is skipped before it.
            if !picker.in_flight().contains(&request) {
                continue;
            }
            match picker.answered(request, result) {
                // Cancelled unasked: sent again once the loop is over.
                Answered::Resend => resend = Some(id),
                Answered::NotMine => {}
                Answered::Taken => {
                    // A kept Enter or Tab, as soon as the answer says what
                    // it takes. It is applied after the loop: the tabs are
                    // borrowed here.
                    let kept = picker
                        .kept
                        .filter(|_| !matches!(picker.take(text), Take::Wait));
                    if let Some(then) = kept {
                        // Under a prompt about pending changes the commit
                        // is dropped (`App::apply`) and the key is lost:
                        // accepted. The editor stays open and the next
                        // Enter takes at once. A tab that is not the one
                        // shown loses the key too, and keeps the answer
                        // for the return.
                        picker.kept = None;
                        commit = (shown == Some((tab, id))).then_some((id, then));
                    }
                }
            }
            break;
        }
        if let Some(id) = resend {
            self.search(tab, id);
        }
        if let Some((id, then)) = commit {
            self.apply(Action::CommitEdit { tab, id, then });
        }
    }

    /// What a commit of the tab's editor takes, where the editor has a
    /// picker: the highlighted option's key becomes the editor's text, or
    /// the text stays as typed. `Taken::Wait` when the list has not
    /// answered the text yet: the commit is kept, the search is sent at
    /// once (or is already on its way), and the answer applies it.
    pub(super) fn picker_takes(&mut self, tab: ConnTabId, id: TabId, then: Advance) -> Taken {
        let class = self
            .table(tab, id, |table, object| {
                let editor = object.edits.editor.as_ref()?;
                table.class(editor.cell.col)
            })
            .flatten()
            .unwrap_or(ColumnClass::Other);
        let Some(editor) = self.editor_mut(tab, id) else {
            return Taken::Text;
        };
        let crate::edit::Editor {
            cell,
            text,
            touched,
            picker: Some(picker),
            ..
        } = editor
        else {
            return Taken::Text;
        };
        let search = match picker.take(text) {
            Take::Text => return Taken::Text,
            Take::Option(index) => {
                let Some(option) = picker.options.get(index) else {
                    return Taken::Text;
                };
                let label = option
                    .label
                    .as_ref()
                    .filter(|label| !label.is_null())
                    .map(crate::ui::format::plain_text);
                // As if its key had been typed: the same check, the same
                // pending cell.
                *text = start_text(&option.key, class);
                *touched = true;
                picker.put(text);
                return Taken::Option(Picked {
                    at: (cell.row, cell.col),
                    key: text.clone(),
                    label,
                });
            }
            Take::Wait => {
                picker.kept = Some(then);
                // Nothing may be on its way (no search was ever sent for
                // the text that shows): the commit must not wait for ever.
                let idle = picker.in_flight().is_empty();
                // Or what was sent last is not for the text that shows
                // (an option's key was put in the editor and refused):
                // no answer on its way decides this text.
                let stale = !picker.asked_for(text);
                picker.due.take().is_some() || idle || stale
            }
        };
        if search {
            self.search(tab, id);
        }
        Taken::Wait
    }

    /// The picker's Set NULL: the editor closes without its text, and its
    /// cell is made NULL where its column takes one. Only an editor with a
    /// picker has this button. Set NULL does nothing while an editor is
    /// open, so the close comes first, and acts on the selection, which is
    /// made the editor's cell.
    pub(super) fn picker_null(&mut self, tab: ConnTabId, id: TabId) {
        let Some((cell, place)) = self
            .editor_mut(tab, id)
            .filter(|editor| editor.picker.is_some())
            .map(|editor| (editor.cell, editor.place))
        else {
            return;
        };
        self.apply(Action::CancelEdit { tab, id });
        let selected = self
            .object_tab_mut(tab, id)
            .is_some_and(|object| object.selection == Some(cell));
        if !selected {
            self.apply(Action::SelectCell { tab, id, cell });
        }
        self.set_null(tab, id, place);
    }

    /// Called every frame. Sends the searches whose time has come, and
    /// stops the matches' reads of pickers that are gone: every one of a
    /// closed picker's still in the list, those that more typing overtook
    /// too (one that is queued costs nothing to cancel). The pinned row's
    /// read is left to end. A picker that waits for the referenced
    /// table's columns, has no failure and has nobody fetching them (the
    /// tree was refreshed under it, say) gets them asked for, once per
    /// table. A picker behind another tab loses a kept Enter or Tab.
    /// Returns how long until the next search is due, for the frame to
    /// come back then.
    pub fn send_due_lookups(&mut self, now: Instant) -> Option<Duration> {
        let mut due = Vec::new();
        let mut next: Option<Duration> = None;
        // The pickers that are open, by their serials.
        let mut open = Vec::new();
        // Tables whose columns a waiting picker has nobody fetching.
        let mut stranded: Vec<(ConnTabId, ObjectRef)> = Vec::new();
        // Pickers behind another tab that keep an Enter or a Tab.
        let mut hidden = Vec::new();
        let shown = self.active_workspace_tab();
        for (tab, workspace) in self.open_connections() {
            for object in workspace.object_tabs() {
                let Some(picker) = object
                    .edits
                    .editor
                    .as_ref()
                    .and_then(|editor| editor.picker.as_ref())
                else {
                    continue;
                };
                open.push(picker.serial);
                if picker.kept.is_some() && shown != Some((tab, object.id)) {
                    hidden.push((tab, object.id));
                }
                let fetching = workspace
                    .columns
                    .get(&picker.target)
                    .is_some_and(|kept| kept.value.is_some() || kept.is_loading());
                if picker.label == Columns::Waiting
                    && picker.failed.is_none()
                    && !fetching
                    && !stranded.contains(&(tab, picker.target.clone()))
                {
                    stranded.push((tab, picker.target.clone()));
                }
                match picker.due {
                    Some(at) if at <= now => due.push((tab, object.id)),
                    Some(at) => next = Some(next.map_or(at - now, |soonest| soonest.min(at - now))),
                    None => {}
                }
            }
        }
        let (kept, orphans): (Vec<_>, Vec<_>) = self
            .lookups
            .iter()
            .copied()
            .partition(|sent| open.contains(&sent.picker));
        self.lookups = kept;
        for sent in orphans {
            // Its editor closed. The matches' read may be a scan of a
            // large table: it is stopped. The pinned row's is not.
            if sent.matches {
                self.cancel(sent.session, [sent.request]);
            }
        }
        for (tab, id) in hidden {
            // Another tab shows: the key is dropped, the search goes on.
            if let Some(picker) = self
                .editor_mut(tab, id)
                .and_then(|editor| editor.picker.as_mut())
            {
                picker.kept = None;
            }
        }
        for (tab, table) in stranded {
            self.load_columns(tab, table);
        }
        for (tab, id) in due {
            self.search(tab, id);
        }
        next
    }
}
