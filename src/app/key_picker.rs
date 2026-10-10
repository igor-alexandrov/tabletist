//! The reducer's part in a foreign key cell's picker: it opens with the
//! editor, sends its searches and reads their answers. What it decides is
//! in `crate::key_picker`.

use std::time::{Duration, Instant};

use tabletist_db::{Error, LookupPage, LookupPart, ObjectRef, Value};

use super::App;
use crate::backend::{Command, RequestId, SessionId};
use crate::edit::{EditorPlace, Table};
use crate::key_picker::{self, Answered, Asked, Columns, Picker};
use crate::model::{ConnTabId, TabId};

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

impl App {
    /// Gives the tab's editor, which just opened, its picker, where its
    /// cell is a foreign key's and the editor is the small field on the
    /// cell. `from_value` says it opened on the cell's value, not on
    /// something typed. The referenced table's columns are asked for if
    /// the workspace does not know them, and the first search is sent as
    /// soon as they are known.
    pub(super) fn open_picker(&mut self, tab: ConnTabId, id: TabId, from_value: bool) {
        let terminal = self.look.terminal;
        let made = self.workspace(tab).and_then(|workspace| {
            let object = workspace.object_tab(id)?;
            let table = Table::of(workspace, object)?;
            let editor = object.edits.editor.as_ref()?;
            // The row panel's field gets its picker in run 3.
            if editor.large || editor.place != EditorPlace::Grid {
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
            // this asks once per editor opened. The failure of the old
            // fetch is not the new one's: the picker waits, and
            // `picker_columns` gives it the failure if the retry fails too.
            let load = kept.is_none_or(|kept| kept.value.is_none() && !kept.is_loading());
            let failed = if load {
                None
            } else {
                kept.and_then(|kept| kept.error.clone())
            };
            let mut picker = Picker::new(target, label, current, !from_value, &editor.text);
            picker.failed = failed;
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
        let Some(workspace) = self.workspace_mut(tab) else {
            return;
        };
        let dialect = workspace.driver.dialect();
        let mut ready = Vec::new();
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
            }
        }
        workspace.columns = columns;
        for id in ready {
            self.search(tab, id);
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
        let Some(workspace) = self.workspace_mut(tab) else {
            return;
        };
        let mut resend = None;
        for object in workspace.object_tabs_mut() {
            let id = object.id;
            let Some(picker) = object
                .edits
                .editor
                .as_mut()
                .and_then(|editor| editor.picker.as_mut())
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
                // A kept Enter is applied here from the next task on.
                Answered::NotMine | Answered::Taken => {}
            }
            break;
        }
        if let Some(id) = resend {
            self.search(tab, id);
        }
    }

    /// Called every frame. Sends the searches whose time has come, and
    /// stops the matches' reads of pickers that are gone: every one of a
    /// closed picker's still in the list, those that more typing overtook
    /// too (one that is queued costs nothing to cancel). The pinned row's
    /// read is left to end. A picker that waits for the referenced
    /// table's columns, has no failure and has nobody fetching them (the
    /// tree was refreshed under it, say) gets them asked for, once per
    /// table. Returns how long until the next search is due, for the
    /// frame to come back then.
    pub fn send_due_lookups(&mut self, now: Instant) -> Option<Duration> {
        let mut due = Vec::new();
        let mut next: Option<Duration> = None;
        // The pickers that are open, by their serials.
        let mut open = Vec::new();
        // Tables whose columns a waiting picker has nobody fetching.
        let mut stranded: Vec<(ConnTabId, ObjectRef)> = Vec::new();
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
        for (tab, table) in stranded {
            self.load_columns(tab, table);
        }
        for (tab, id) in due {
            self.search(tab, id);
        }
        next
    }
}
