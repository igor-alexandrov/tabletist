//! The reducer's part in the label of a table that foreign keys point at:
//! what the stored choice comes to for a workspace's table, and what
//! happens to a choice whose column is gone. The choice and the rule are
//! in `crate::labels`. Nothing else reads the store.

use tabletist_db::{ColumnInfo, Dialect, ObjectRef};

use std::time::Instant;

use super::App;
use crate::connections::ConnectionId;
use crate::key_picker::Columns;
use crate::labels::{self, Chooser, LabelChoice, Labels, Line, Resolved, Source};
use crate::model::{ConnTabId, Fetch, TabId};

/// What labels the rows of `table`, whose columns the workspace of the
/// saved connection `conn` keeps as `kept`, in a picker whose foreign key
/// references `key`. None while the columns are not known. None too while
/// a chosen column is missing from columns that are being read again: it
/// is called gone only by columns that are fresh.
pub(super) fn resolved(
    labels: &Labels,
    (conn, dialect): (&ConnectionId, Dialect),
    kept: &Fetch<Vec<ColumnInfo>>,
    table: &ObjectRef,
    key: &str,
) -> Option<Resolved> {
    let columns = kept.value.as_deref()?;
    let choice = labels.choice(conn, table);
    let resolved = labels::resolve(&choice, dialect, columns, key);
    if resolved.gone.is_some() && kept.is_loading() {
        return None;
    }
    Some(resolved)
}

impl App {
    /// What labels the rows of `table` for the foreign keys of `tab` that
    /// reference its column `key`: the one resolver every view reads.
    pub fn label_of(&self, tab: ConnTabId, table: &ObjectRef, key: &str) -> Option<Resolved> {
        let workspace = self.workspace(tab)?;
        let of = (&workspace.conn_id, workspace.driver.dialect());
        resolved(&self.labels, of, workspace.columns.get(table)?, table, key)
    }

    /// The columns of `table` were just read. A column chosen as its label
    /// that is not among them is gone: the choice goes back to Automatic,
    /// and the column's name is kept to be said once.
    pub(super) fn check_label(&mut self, tab: ConnTabId, table: &ObjectRef) {
        let Some(workspace) = self.workspace(tab) else {
            return;
        };
        let fresh = workspace
            .columns
            .get(table)
            .filter(|kept| !kept.is_loading())
            .and_then(|kept| kept.value.as_deref());
        let Some(columns) = fresh else {
            return;
        };
        let conn = workspace.conn_id.clone();
        let LabelChoice::Column(name) = self.labels.choice(&conn, table) else {
            return;
        };
        if columns.iter().any(|column| column.name == name) {
            return;
        }
        self.labels.lost(&conn, table, &name);
        self.save_labels();
        // A picker that is open on the column searches by what is left,
        // and says the column is gone: nobody chose, so nothing confirms
        // a choice. One that waits for these columns is labelled for the
        // first time by their answer.
        self.relabel(&conn, table, false);
    }

    /// Opens the chooser of what labels the options of the picker of the
    /// tab's editor, on the line of the choice kept for its table; closes
    /// it where it is open. Where the table has nothing to choose no
    /// chooser opens, and the picker says so for a while.
    pub(super) fn toggle_label_chooser(&mut self, tab: ConnTabId, id: TabId) {
        let planned = self.workspace(tab).and_then(|workspace| {
            let editor = workspace.object_tab(id)?.edits.editor.as_ref()?;
            let picker = editor.picker.as_ref()?;
            if picker.chooser.is_some() {
                return Some(None);
            }
            // A search that failed has no list, and its keys are the
            // field's: a chooser there would get none of them.
            if picker.failed.is_some() {
                return None;
            }
            let lines = workspace.label_lines(&picker.target, &picker.key);
            let choice = self.labels.choice(&workspace.conn_id, &picker.target);
            // The columns are still on their way: Automatic and No label
            // can be chosen at once, and the rest shows when it comes.
            let waited = picker.label == Columns::Waiting;
            let nothing = matches!(
                &picker.label,
                Columns::Known(label) if label.source == Source::KeysOnly
            );
            Some(Some((Chooser::open(&lines, choice, waited), nothing)))
        });
        let picker = self
            .editor_mut(tab, id)
            .and_then(|editor| editor.picker.as_mut());
        let (Some(planned), Some(picker)) = (planned, picker) else {
            return;
        };
        match planned {
            None => picker.chooser = None,
            Some((_, true)) => picker.nothing = Some(Instant::now()),
            Some((chooser, false)) => {
                picker.chooser = Some(chooser);
                picker.nothing = None;
            }
        }
    }

    /// Lets `act` move the keys of the chooser that is open beside the
    /// picker of the tab's editor, among its lines as they are now.
    pub(super) fn in_label_chooser(
        &mut self,
        tab: ConnTabId,
        id: TabId,
        act: impl FnOnce(&mut Chooser, &[Line]),
    ) {
        let lines = self.workspace(tab).and_then(|workspace| {
            let editor = workspace.object_tab(id)?.edits.editor.as_ref()?;
            let picker = editor.picker.as_ref()?;
            picker.chooser.as_ref()?;
            Some(workspace.label_lines(&picker.target, &picker.key))
        });
        let chooser = self
            .editor_mut(tab, id)
            .and_then(|editor| editor.picker.as_mut())
            .and_then(|picker| picker.chooser.as_mut());
        if let (Some(lines), Some(chooser)) = (lines, chooser) {
            act(chooser, &lines);
        }
    }

    /// Chooses `line` of the chooser that is open beside the picker of
    /// the tab's editor, or the line its keys are on. A line that cannot
    /// be chosen (the key, a column of bytes) chooses nothing, and the
    /// chooser stays. The editor stays open with its text as it is: no
    /// value of the cell is touched.
    pub(super) fn choose_label(&mut self, tab: ConnTabId, id: TabId, line: Option<usize>) {
        let planned = self.workspace(tab).and_then(|workspace| {
            let editor = workspace.object_tab(id)?.edits.editor.as_ref()?;
            let picker = editor.picker.as_ref()?;
            let chooser = picker.chooser.as_ref()?;
            let lines = workspace.label_lines(&picker.target, &picker.key);
            let choice = lines.get(line.unwrap_or(chooser.at))?.choice()?;
            Some((picker.target.clone(), choice))
        });
        let Some((table, choice)) = planned else {
            return;
        };
        if let Some(picker) = self
            .editor_mut(tab, id)
            .and_then(|editor| editor.picker.as_mut())
        {
            picker.chooser = None;
        }
        self.set_label(tab, &table, choice);
    }

    /// Keeps `choice` as what labels the rows of `table` on the saved
    /// connection of `tab`, on this computer: nothing is sent to the
    /// database but the searches of the pickers that are open on the
    /// table, which are labelled afresh. `choice` is one a chooser's line
    /// offered. Returns whether it changed anything.
    pub fn set_label(&mut self, tab: ConnTabId, table: &ObjectRef, choice: LabelChoice) -> bool {
        let Some(workspace) = self.workspace(tab) else {
            return false;
        };
        let conn = workspace.conn_id.clone();
        if !self.labels.choose(&conn, table, choice) {
            return false;
        }
        self.save_labels();
        self.relabel(&conn, table, true);
        true
    }

    /// Labels afresh every open picker of `table` on the saved connection
    /// `conn`, in whichever of its tabs, and sends each one's search again
    /// for the text it holds: once, and no other table's. `chosen` says
    /// the user just chose: each picker then confirms it for a while, and
    /// one that still waits for the table's columns is among them, to
    /// confirm when its list comes.
    pub(super) fn relabel(&mut self, conn: &ConnectionId, table: &ObjectRef, chosen: bool) {
        let now = Instant::now();
        let labels = self.labels.clone();
        let tabs: Vec<ConnTabId> = self
            .open_connections()
            .filter(|(_, workspace)| workspace.conn_id == *conn)
            .map(|(tab, _)| tab)
            .collect();
        let mut again = Vec::new();
        for tab in tabs {
            let Some(workspace) = self.workspace_mut(tab) else {
                continue;
            };
            let dialect = workspace.driver.dialect();
            let columns = std::mem::take(&mut workspace.columns);
            for object in workspace.object_tabs_mut() {
                let picker = object
                    .edits
                    .editor
                    .as_mut()
                    .and_then(|editor| editor.picker.as_mut())
                    .filter(|picker| picker.target == *table)
                    .filter(|picker| chosen || picker.label != Columns::Waiting);
                let Some(picker) = picker else {
                    continue;
                };
                let label = columns
                    .get(table)
                    .and_then(|kept| resolved(&labels, (conn, dialect), kept, table, &picker.key))
                    .map_or(Columns::Waiting, Columns::Known);
                picker.relabel(label, chosen.then_some(now));
                again.push((tab, object.id));
            }
            workspace.columns = columns;
        }
        for (tab, id) in again {
            self.tell_gone(tab, id);
            // Nothing is sent for a picker that waits for its columns:
            // their answer sends its search.
            self.search(tab, id);
        }
    }

    /// The picker of the tab's editor knows what labels it. If the column
    /// chosen for its table was found gone and nobody was told, this
    /// picker says so for as long as it is open, and the store forgets
    /// it: it is said once.
    pub(super) fn tell_gone(&mut self, tab: ConnTabId, id: TabId) {
        let found = self.workspace(tab).and_then(|workspace| {
            let editor = workspace.object_tab(id)?.edits.editor.as_ref()?;
            let picker = editor.picker.as_ref()?;
            if picker.label == Columns::Waiting {
                return None;
            }
            let conn = &workspace.conn_id;
            let gone = self.labels.gone(conn, &picker.target)?.to_owned();
            Some((conn.clone(), picker.target.clone(), gone))
        });
        let Some((conn, table, gone)) = found else {
            return;
        };
        if let Some(picker) = self
            .editor_mut(tab, id)
            .and_then(|editor| editor.picker.as_mut())
        {
            picker.gone = Some((gone, Instant::now()));
        }
        if self.labels.told(&conn, &table) {
            self.save_labels();
        }
    }
}
