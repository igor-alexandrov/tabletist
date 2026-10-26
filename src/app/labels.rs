//! The reducer's part in the label of a table that foreign keys point at:
//! what the stored choice comes to for a workspace's table, and what
//! happens to a choice whose column is gone. The choice and the rule are
//! in `crate::labels`. Nothing else reads the store.

use tabletist_db::{ColumnInfo, Dialect, ObjectKind, ObjectRef};

use std::time::Instant;

use super::App;
use crate::key_picker::Columns;
use crate::labels::{self, Chooser, LabelChoice, Labels, Line, Place, Resolved, Source};
use crate::model::{ConnTabId, Fetch, ObjectView, TabId};

/// What labels the rows of `table`, whose columns the workspace at
/// `place` keeps as `kept`, in a picker whose foreign key references
/// `key`. None while the columns are not known. None too while
/// a chosen column is missing from columns that are being read again: it
/// is called gone only by columns that are fresh.
pub(super) fn resolved(
    labels: &Labels,
    (place, dialect): (&Place, Dialect),
    kept: &Fetch<Vec<ColumnInfo>>,
    table: &ObjectRef,
    key: &str,
) -> Option<Resolved> {
    let columns = kept.value.as_deref()?;
    let choice = labels.choice(place, table);
    let resolved = labels::resolve(&choice, dialect, columns, &[key]);
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
        let place = workspace.label_place();
        let of = (&place, workspace.driver.dialect());
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
        let place = workspace.label_place();
        let LabelChoice::Column(name) = self.labels.choice(&place, table) else {
            return;
        };
        if columns.iter().any(|column| column.name == name) {
            return;
        }
        self.labels.lost(&place, table, &name);
        self.save_labels();
        // A picker that is open on the column searches by what is left,
        // and says the column is gone: nobody chose, so nothing confirms
        // a choice. One that waits for these columns is labelled for the
        // first time by their answer.
        self.relabel(&place, table, false);
    }

    /// The chooser of the tab `id`, where it has a place for one: its
    /// Structure view's, on the tab itself, while that view shows, and
    /// else the one of its editor's picker. The DDL view has none.
    fn chooser_mut(&mut self, tab: ConnTabId, id: TabId) -> Option<&mut Option<Chooser>> {
        let object = self.object_tab_mut(tab, id)?;
        match object.view {
            ObjectView::Structure => Some(&mut object.label_chooser),
            ObjectView::Data => {
                let picker = object.edits.editor.as_mut()?.picker.as_mut()?;
                Some(&mut picker.chooser)
            }
            ObjectView::Ddl => None,
        }
    }

    /// The table that the chooser of the tab `id` chooses for, and its
    /// lines as they are now: the tab's own table by its structure, with
    /// its primary key as the key, in the Structure view; the table its
    /// picker searches, with the column the foreign key references, under
    /// an editor.
    fn chooser_lines(&self, tab: ConnTabId, id: TabId) -> Option<(ObjectRef, Vec<Line>)> {
        let workspace = self.workspace(tab)?;
        let object = workspace.object_tab(id)?;
        match object.view {
            ObjectView::Structure => {
                let structure = object.structure.value.as_ref()?;
                let dialect = workspace.driver.dialect();
                let lines = labels::lines(
                    dialect,
                    Some(&structure.columns),
                    &labels::own_keys(structure),
                );
                Some((object.object.clone(), lines))
            }
            ObjectView::Data => {
                let picker = object.edits.editor.as_ref()?.picker.as_ref()?;
                let lines = workspace.label_lines(&picker.target, &[&picker.key]);
                Some((picker.target.clone(), lines))
            }
            ObjectView::Ddl => None,
        }
    }

    /// What labels the rows of the table the tab `id` shows, as its
    /// Structure view says it: the choice kept for the table against its
    /// own columns, its primary key being the key. None while the
    /// structure is not known, and for what is no table: no foreign key
    /// points at a view.
    pub fn structure_label(&self, tab: ConnTabId, id: TabId) -> Option<Resolved> {
        let workspace = self.workspace(tab)?;
        let object = workspace.object_tab(id)?;
        match object.kind {
            ObjectKind::Table => {}
            ObjectKind::View | ObjectKind::MaterializedView => return None,
        }
        let structure = object.structure.value.as_ref()?;
        let choice = self.labels.choice(&workspace.label_place(), &object.object);
        let dialect = workspace.driver.dialect();
        let keys = labels::own_keys(structure);
        Some(labels::resolve(&choice, dialect, &structure.columns, &keys))
    }

    /// Opens the chooser of the tab `id` on the line of the choice kept
    /// for its table, and closes it where it is open: beside the picker
    /// of its editor, or in its Structure view. Where the table has
    /// nothing to choose no chooser opens: a picker says so for a while,
    /// and the Structure view's line says it as long as it shows.
    pub(super) fn toggle_label_chooser(&mut self, tab: ConnTabId, id: TabId) {
        // What the key does: nothing, close, say there is nothing to
        // choose, or open.
        enum Asked {
            Close,
            Nothing,
            Open(Chooser),
        }
        let asked = self.workspace(tab).and_then(|workspace| {
            let object = workspace.object_tab(id)?;
            match object.view {
                ObjectView::Structure => {
                    if object.label_chooser.is_some() {
                        return Some(Asked::Close);
                    }
                    let label = self.structure_label(tab, id)?;
                    let (table, lines) = self.chooser_lines(tab, id)?;
                    let choice = self.labels.choice(&workspace.label_place(), &table);
                    Some(match label.source {
                        Source::KeysOnly => Asked::Nothing,
                        Source::Automatic | Source::User => {
                            Asked::Open(Chooser::open(&lines, choice, false))
                        }
                    })
                }
                ObjectView::Data => {
                    let picker = object.edits.editor.as_ref()?.picker.as_ref()?;
                    if picker.chooser.is_some() {
                        return Some(Asked::Close);
                    }
                    // A search that failed has no list, and its keys are
                    // the field's: a chooser there would get none of them.
                    if picker.failed.is_some() {
                        return None;
                    }
                    let (table, lines) = self.chooser_lines(tab, id)?;
                    let choice = self.labels.choice(&workspace.label_place(), &table);
                    // The columns are still on their way: Automatic and
                    // No label can be chosen at once, and the rest shows
                    // when it comes.
                    let waited = picker.label == Columns::Waiting;
                    Some(match &picker.label {
                        Columns::Known(label) if label.source == Source::KeysOnly => Asked::Nothing,
                        Columns::Known(_) | Columns::Waiting => {
                            Asked::Open(Chooser::open(&lines, choice, waited))
                        }
                    })
                }
                // Nothing there chooses a label.
                ObjectView::Ddl => None,
            }
        });
        let Some(asked) = asked else {
            return;
        };
        // A picker notes that it was asked, to say why nothing opens.
        if let Asked::Nothing = asked {
            if let Some(picker) = self
                .editor_mut(tab, id)
                .and_then(|editor| editor.picker.as_mut())
            {
                picker.nothing = Some(Instant::now());
            }
            return;
        }
        if let Some(picker) = self
            .editor_mut(tab, id)
            .and_then(|editor| editor.picker.as_mut())
        {
            picker.nothing = None;
        }
        if let Some(home) = self.chooser_mut(tab, id) {
            *home = match asked {
                Asked::Open(chooser) => Some(chooser),
                Asked::Close | Asked::Nothing => None,
            };
        }
    }

    /// Closes the chooser of the tab `id` and chooses nothing.
    pub(super) fn close_label_chooser(&mut self, tab: ConnTabId, id: TabId) {
        if let Some(home) = self.chooser_mut(tab, id) {
            *home = None;
        }
    }

    /// Lets `act` move the keys of the chooser that is open in the tab
    /// `id`, among its lines as they are now.
    pub(super) fn in_label_chooser(
        &mut self,
        tab: ConnTabId,
        id: TabId,
        act: impl FnOnce(&mut Chooser, &[Line]),
    ) {
        let Some((_, lines)) = self.chooser_lines(tab, id) else {
            return;
        };
        let chooser = self.chooser_mut(tab, id).and_then(Option::as_mut);
        if let Some(chooser) = chooser {
            act(chooser, &lines);
        }
    }

    /// Chooses `line` of the chooser that is open in the tab `id`, or the
    /// line its keys are on. A line that cannot be chosen (the key, a
    /// column of bytes) chooses nothing, and the chooser stays. Under an
    /// editor the editor stays open with its text as it is: no value of
    /// the cell is touched.
    pub(super) fn choose_label(&mut self, tab: ConnTabId, id: TabId, line: Option<usize>) {
        let Some((table, lines)) = self.chooser_lines(tab, id) else {
            return;
        };
        let at = self
            .chooser_mut(tab, id)
            .and_then(|home| home.as_ref())
            .map(|chooser| line.unwrap_or(chooser.at));
        let choice = at.and_then(|at| lines.get(at)).and_then(Line::choice);
        let Some(choice) = choice else {
            return;
        };
        self.close_label_chooser(tab, id);
        self.set_label(tab, &table, choice);
    }

    /// Moves the cursor of the Structure view's columns `by` lines,
    /// stopping at the first and at the last.
    pub(super) fn move_structure_cursor(&mut self, tab: ConnTabId, id: TabId, by: isize) {
        let Some(object) = self.object_tab_mut(tab, id) else {
            return;
        };
        let last = object
            .structure
            .value
            .as_ref()
            .map_or(0, |structure| structure.columns.len().saturating_sub(1));
        object.structure_cursor = object.structure_cursor.saturating_add_signed(by).min(last);
        self.forget_refusal(tab);
    }

    /// What was said of a column that was not taken was said of the line
    /// the cursor was on, or of the name that was typed: it goes when the
    /// cursor moves, a label is kept or the view changes.
    pub(super) fn forget_refusal(&mut self, tab: ConnTabId) {
        if let Some(workspace) = self.workspace_mut(tab) {
            workspace.label_refused = None;
        }
    }

    /// Puts that cursor on `line`, where there is such a line.
    pub(super) fn set_structure_cursor(&mut self, tab: ConnTabId, id: TabId, line: usize) {
        let Some(object) = self.object_tab_mut(tab, id) else {
            return;
        };
        let columns = object
            .structure
            .value
            .as_ref()
            .map_or(0, |structure| structure.columns.len());
        if line < columns {
            object.structure_cursor = line;
        }
        self.forget_refusal(tab);
    }

    /// The table the tab `id` shows, where it is a table and its
    /// structure is known: what the Structure view's door chooses for.
    fn own_table(&self, tab: ConnTabId, id: TabId) -> Option<ObjectRef> {
        self.structure_label(tab, id)?;
        let object = self.workspace(tab)?.object_tab(id)?;
        Some(object.object.clone())
    }

    /// Keeps `choice` for the table the tab `id` shows: the Structure
    /// view's Use Automatic, and the prompt's lines that name no column.
    pub(super) fn set_tab_label(&mut self, tab: ConnTabId, id: TabId, choice: LabelChoice) {
        let Some(table) = self.own_table(tab, id) else {
            return;
        };
        self.close_label_chooser(tab, id);
        self.set_label(tab, &table, choice);
    }

    /// Uses the column on `line` of the Structure view of the tab `id` as
    /// what labels its table's rows, or the column under the cursor.
    pub(super) fn use_as_label(&mut self, tab: ConnTabId, id: TabId, line: Option<usize>) {
        let name = self.workspace(tab).and_then(|workspace| {
            let object = workspace.object_tab(id)?;
            let structure = object.structure.value.as_ref()?;
            let line = line.unwrap_or(object.structure_cursor);
            Some(structure.columns.get(line)?.name.clone())
        });
        if let Some(name) = name {
            self.label_by_name(tab, id, &name);
        }
    }

    /// Labels the table the tab `id` shows by its column `name`, named by
    /// hand: a line of the prompt, the key on a line of the Structure
    /// view, a column's menu. A name that is no column's, the key and a
    /// column of bytes are refused, and the workspace keeps why to say it
    /// until the next key.
    pub(super) fn label_by_name(&mut self, tab: ConnTabId, id: TabId, name: &str) {
        let asked = self.workspace(tab).and_then(|workspace| {
            let object = workspace.object_tab(id)?;
            let structure = object.structure.value.as_ref()?;
            let dialect = workspace.driver.dialect();
            let keys = labels::own_keys(structure);
            Some(labels::choosable(dialect, &structure.columns, &keys, name))
        });
        let (Some(table), Some(asked)) = (self.own_table(tab, id), asked) else {
            return;
        };
        match asked {
            Ok(()) => {
                self.close_label_chooser(tab, id);
                self.set_label(tab, &table, LabelChoice::Column(name.to_owned()));
            }
            Err(why) => {
                if let Some(workspace) = self.workspace_mut(tab) {
                    workspace.label_refused = Some((id, name.to_owned(), why));
                }
            }
        }
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
        let place = workspace.label_place();
        self.forget_refusal(tab);
        if !self.labels.choose(&place, table, choice) {
            return false;
        }
        self.save_labels();
        self.relabel(&place, table, true);
        true
    }

    /// Labels afresh every open picker of `table` at `place`, in whichever
    /// tab of its saved connection is on that database, and sends each one's search again
    /// for the text it holds: once, and no other table's. `chosen` says
    /// the user just chose: each picker then confirms it for a while, and
    /// one that still waits for the table's columns is among them, to
    /// confirm when its list comes.
    pub(super) fn relabel(&mut self, place: &Place, table: &ObjectRef, chosen: bool) {
        let now = Instant::now();
        let labels = self.labels.clone();
        let tabs: Vec<ConnTabId> = self
            .open_connections()
            .filter(|(_, workspace)| workspace.label_place() == *place)
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
                    .and_then(|kept| resolved(&labels, (place, dialect), kept, table, &picker.key))
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
            let place = workspace.label_place();
            let gone = self.labels.gone(&place, &picker.target)?.to_owned();
            Some((place, picker.target.clone(), gone))
        });
        let Some((place, table, gone)) = found else {
            return;
        };
        if let Some(picker) = self
            .editor_mut(tab, id)
            .and_then(|editor| editor.picker.as_mut())
        {
            picker.gone = Some((gone, Instant::now()));
        }
        if self.labels.told(&place, &table) {
            self.save_labels();
        }
    }
}
