//! Application state and the reducer.

use crate::backend::{Backend, SessionId};
use crate::connections::SavedConnections;
use std::collections::HashMap;

use tabletist_db::{
    Driver, Error, FilterOp, HostKeys, ObjectKind, ObjectRef, Secrets, Sort, SortDir, SshStage,
};

use crate::backend::{Command, Event, RequestId, StateFile};
use crate::connections::{PasswordMode, SavedConnection};
use crate::i18n::Locale;
use crate::model::{Action, ConnTab, ConnTabContent, ConnTabId, PickerState};
use crate::model::{
    CellPos, ConnectionForm, Dialog, Fetch, FilterBar, FilterRow, HostKeyPrompt, ObjectTab,
    ObjectTabId, ObjectView, Pane, PasswordPrompt, PickTarget, QuickOpen, SecretKind,
    SessionStatus, TestState, Tree, TreeKey, TreeNode, Workspace,
};
use crate::paths::AppDirs;
use crate::secrets::{SecretString, password_account, ssh_account};
use crate::settings::Settings;
use crate::theme::{self, Catalog, Palette};

/// What a keyring read is for; each names the exact request it serves, so
/// a late answer for an earlier attempt is dropped.
#[derive(Debug, Clone, Copy)]
enum SecretPurpose {
    /// A secret for `tab`'s connect attempt `request`.
    Connect {
        tab: ConnTabId,
        request: RequestId,
        kind: SecretKind,
    },
    /// A secret for the dialog's Test `test`.
    Test { test: RequestId, kind: SecretKind },
}

/// A keyring write or delete in flight.
#[derive(Debug)]
struct PendingStore {
    conn: crate::connections::ConnectionId,
    /// The SSH secret rather than the database password.
    ssh: bool,
    /// A write rather than a delete.
    saving: bool,
    /// Switch the connection to keyring mode once the write lands.
    adopt: bool,
}

/// The native title bar the tab bar shares: its height, and how far the
/// window's own buttons (the macOS traffic lights) reach from the left.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct TitleBar {
    pub height: f32,
    pub inset: f32,
}

pub struct App {
    pub dirs: AppDirs,
    pub settings: Settings,
    pub locale: Locale,
    pub palette: Palette,
    pub look: crate::theme::Look,
    pub themes: Catalog,
    pub tabs: Vec<ConnTab>,
    /// Index into `tabs`. Always valid: `tabs` is never empty.
    pub active: usize,
    /// Pushed by views and shortcuts, applied after the frame is drawn.
    pub actions: Vec<Action>,
    pub backend: Backend,
    pub connections: SavedConnections,
    pub dialog: Option<Dialog>,
    /// Keyring reads in flight, and what each is for.
    pending_secrets: HashMap<RequestId, SecretPurpose>,
    /// Keyring writes and deletes in flight.
    pending_stores: HashMap<RequestId, PendingStore>,
    /// A problem to show above the window's content until dismissed (a
    /// keyring write that failed).
    pub notice: Option<String>,
    /// SSH host keys the user trusts, from known_hosts.json.
    pub host_keys: HostKeys,
    /// Why known_hosts.json could not be read; no new host is trusted then.
    pub host_keys_error: Option<String>,
    /// The window's own title bar the tab bar shares (macOS), measured from
    /// the window every frame; zero elsewhere.
    pub titlebar: TitleBar,
    /// The OS theme seen last frame, to notice light/dark switches.
    system_theme: Option<egui::Theme>,
    /// The window title last sent, so it is sent only when it changes.
    window_title: String,
    next_id: u64,
}

impl App {
    pub fn new(dirs: AppDirs, settings: Settings, backend: Backend) -> Self {
        let (connections, upgraded) = SavedConnections::load_upgrading(&dirs.connections_file());
        let (host_keys, host_keys_error) = match crate::known_hosts::load(&dirs.known_hosts_file())
        {
            Ok(keys) => (keys, None),
            Err(problem) => {
                log::error!("could not read known hosts: {problem}");
                (HostKeys::default(), Some(problem))
            }
        };
        let mut app = Self {
            dirs,
            settings,
            locale: Locale::default(),
            palette: Palette::dark(),
            look: crate::theme::Look::for_platform(),
            themes: Catalog::default(),
            tabs: Vec::new(),
            active: 0,
            actions: Vec::new(),
            backend,
            connections,
            dialog: None,
            pending_secrets: HashMap::new(),
            pending_stores: HashMap::new(),
            notice: None,
            host_keys,
            host_keys_error,
            titlebar: TitleBar::default(),
            system_theme: None,
            window_title: "Tabletist".into(),
            next_id: 1,
        };
        let tab = app.picker_tab();
        app.tabs.push(tab);
        // An older file is written in this version once, off the UI thread.
        if upgraded {
            app.save_connections();
        }
        app
    }

    /// The window's title: the active connection, its environment and its
    /// database, which is how the window switcher, Mission Control and
    /// Hyprland tell connection windows apart. Plain text: no colour.
    pub fn window_title(&self) -> String {
        let ConnTabContent::Workspace(workspace) = &self.tabs[self.active].content else {
            return "Tabletist".into();
        };
        let env = workspace
            .environment
            .label(crate::env::Platform::of(&self.look));
        let database = match &workspace.spec.sqlite_path {
            Some(path) => crate::model::file_name(&path.display().to_string()),
            None => workspace.spec.database.clone(),
        };
        if database.is_empty() {
            format!("{} · {env}", workspace.name)
        } else {
            format!("{} · {env} — {database}", workspace.name)
        }
    }

    pub fn workspace(&self, tab: ConnTabId) -> Option<&Workspace> {
        match &self.tabs.iter().find(|t| t.id == tab)?.content {
            ConnTabContent::Workspace(workspace) => Some(workspace),
            ConnTabContent::Picker(_) => None,
        }
    }

    pub fn picker_mut(&mut self, tab: ConnTabId) -> Option<&mut PickerState> {
        match &mut self.tabs.iter_mut().find(|t| t.id == tab)?.content {
            ConnTabContent::Picker(picker) => Some(picker),
            ConnTabContent::Workspace(_) => None,
        }
    }

    pub fn workspace_mut(&mut self, tab: ConnTabId) -> Option<&mut Workspace> {
        match &mut self.tabs.iter_mut().find(|t| t.id == tab)?.content {
            ConnTabContent::Workspace(workspace) => Some(workspace),
            ConnTabContent::Picker(_) => None,
        }
    }

    pub fn tab_for_session(&self, session: SessionId) -> Option<ConnTabId> {
        self.tabs.iter().find_map(|tab| match &tab.content {
            ConnTabContent::Workspace(workspace) if workspace.session == session => Some(tab.id),
            _ => None,
        })
    }

    /// Moves backend events into the action queue.
    pub fn poll_backend(&mut self) {
        for event in self.backend.poll() {
            self.actions.push(Action::Backend(event));
        }
    }

    /// A fresh id for tabs, sessions and requests. Never reused.
    pub fn next_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn picker_tab(&mut self) -> ConnTab {
        ConnTab {
            id: ConnTabId(self.next_id()),
            content: ConnTabContent::Picker(PickerState::default()),
        }
    }

    pub fn active_tab(&self) -> &ConnTab {
        &self.tabs[self.active]
    }

    pub fn active_tab_id(&self) -> ConnTabId {
        self.active_tab().id
    }

    fn tab_index(&self, id: ConnTabId) -> Option<usize> {
        self.tabs.iter().position(|tab| tab.id == id)
    }

    /// Applies queued actions until none are left (an action may queue more).
    pub fn apply_actions(&mut self) {
        while !self.actions.is_empty() {
            for action in std::mem::take(&mut self.actions) {
                self.apply(action);
            }
        }
        self.format_rows();
    }

    /// Formats the row each open row panel shows, once per selection or
    /// page, so drawing never reads a whole (possibly huge) value.
    fn format_rows(&mut self) {
        for tab in &mut self.tabs {
            let ConnTabContent::Workspace(workspace) = &mut tab.content else {
                continue;
            };
            let (open, active) = (workspace.row_panel, workspace.active_object);
            for object in &mut workspace.objects {
                let row = object
                    .selection
                    .filter(|_| open && active == Some(object.id))
                    .map(|cell| cell.row);
                let Some((row, page)) = row.zip(object.page()) else {
                    // Nothing shows it: free the text.
                    object.fields = None;
                    continue;
                };
                if object.selected_fields().is_some() {
                    continue;
                }
                object.fields = page.rows.get(row).map(|values| crate::model::RowFields {
                    request: object.rows.loaded,
                    row,
                    fields: values.iter().map(crate::ui::format::field_text).collect(),
                });
            }
        }
    }

    pub fn apply(&mut self, action: Action) {
        match action {
            Action::NewConnTab => {
                let tab = self.picker_tab();
                self.tabs.push(tab);
                self.active = self.tabs.len() - 1;
            }
            Action::CloseConnTab(id) => self.close_tab(id),
            Action::ActivateConnTab(id) => {
                if let Some(index) = self.tab_index(id) {
                    self.active = index;
                }
            }
            Action::ActivateConnTabIndex(index) => {
                if index < self.tabs.len() {
                    self.active = index;
                }
            }
            Action::CycleConnTab(step) => {
                let len = self.tabs.len() as isize;
                self.active = (self.active as isize + step).rem_euclid(len) as usize;
            }
            Action::Connect { tab, conn } => {
                if let Some(saved) = self.connections.get(&conn).cloned() {
                    self.connect_tab(tab, saved, Secrets::default());
                }
            }
            Action::Reconnect(tab) => self.reconnect(tab),
            Action::Disconnect(tab) => {
                if let Some(workspace) = self.workspace(tab) {
                    let session = workspace.session;
                    self.backend.send(Command::Close { session });
                    if let Some(entry) = self.tabs.iter_mut().find(|t| t.id == tab) {
                        entry.content = ConnTabContent::Picker(PickerState::default());
                    }
                }
            }
            Action::ToggleSchema { tab, schema } => {
                let expanded = self
                    .workspace(tab)
                    .and_then(|workspace| workspace.tree.nodes.get(&schema))
                    .is_some_and(|node| node.expanded);
                if expanded {
                    if let Some(workspace) = self.workspace_mut(tab)
                        && let Some(node) = workspace.tree.nodes.get_mut(&schema)
                    {
                        node.expanded = false;
                    }
                } else {
                    self.expand_schema(tab, &schema);
                }
            }
            Action::SelectConnection { tab, conn } => {
                if let Some(picker) = self.picker_mut(tab) {
                    picker.selected = conn;
                }
            }
            Action::MovePickerSelection { tab, step } => {
                let search = self.picker_mut(tab).map(|picker| picker.search.clone());
                let Some(search) = search else {
                    return;
                };
                let order = crate::ui::picker::visible(self, &search);
                if let Some(picker) = self.picker_mut(tab)
                    && !order.is_empty()
                {
                    let at = picker
                        .selected
                        .as_ref()
                        .and_then(|selected| order.iter().position(|id| id == selected));
                    let next = match at {
                        None if step < 0 => order.len() - 1,
                        None => 0,
                        Some(at) => {
                            (at as isize + step).clamp(0, order.len() as isize - 1) as usize
                        }
                    };
                    picker.selected = Some(order[next].clone());
                }
            }
            Action::FoldConnectionGroup { tab, group } => {
                if let Some(picker) = self.picker_mut(tab) {
                    if let Some(index) = picker.folded.iter().position(|title| *title == group) {
                        picker.folded.remove(index);
                    } else {
                        picker.folded.push(group);
                    }
                }
            }
            Action::ShowSchema { tab, schema } => {
                if let Some(workspace) = self.workspace_mut(tab) {
                    workspace.tree.schema = Some(schema.clone());
                    workspace.tree.cursor = None;
                }
                self.expand_schema(tab, &schema);
            }
            Action::ToggleGroup {
                tab,
                schema,
                prefix,
            } => {
                if let Some(workspace) = self.workspace_mut(tab) {
                    let node = workspace.tree.nodes.entry(schema).or_default();
                    if !node.open_groups.remove(&prefix) {
                        node.open_groups.insert(prefix);
                    }
                }
            }
            Action::ToggleFlatTree(tab) => {
                if let Some(workspace) = self.workspace_mut(tab) {
                    workspace.tree.flat = !workspace.tree.flat;
                }
            }
            Action::ToggleSidebar(tab) => {
                if let Some(workspace) = self.workspace_mut(tab) {
                    workspace.sidebar_hidden = !workspace.sidebar_hidden;
                    if workspace.sidebar_hidden {
                        workspace.pane = Pane::Grid;
                    }
                }
            }
            Action::ToggleFullPrecision(tab) => {
                if let Some(workspace) = self.workspace_mut(tab) {
                    workspace.full_precision = !workspace.full_precision;
                }
            }
            Action::DropFilter {
                tab,
                object_tab,
                index,
            } => {
                if let Some(object) = self.object_tab_mut(tab, object_tab) {
                    let mut rows: Vec<FilterRow> = object
                        .query
                        .filters
                        .iter()
                        .map(|filter| FilterRow {
                            column: filter.column.clone(),
                            op: filter.op,
                            value: filter.value.clone(),
                        })
                        .collect();
                    let raw = object.query.raw_where.is_some();
                    if index < rows.len() {
                        rows.remove(index);
                        object.filter.raw = raw;
                    } else {
                        object.filter.raw = false;
                        object.filter.raw_text.clear();
                    }
                    object.filter.rows = rows;
                    self.apply_filters(tab, object_tab);
                }
            }
            Action::FocusPickerSearch(tab) => {
                if let Some(picker) = self.picker_mut(tab) {
                    picker.focus_search = true;
                }
            }
            Action::FoldDocuments { tab, object_tab } => {
                if let Some(workspace) = self.workspace_mut(tab) {
                    workspace.fold_documents = Some(object_tab);
                }
            }
            Action::FollowSelectedKey { tab, object_tab } => {
                let target =
                    self.workspace(tab).and_then(|workspace| {
                        let object = workspace.object_tab(object_tab)?;
                        let cell = object.selection?;
                        let page = object.page()?;
                        let column = &page.columns.get(cell.col)?.name;
                        let foreign = object.structure.value.as_ref()?.foreign_keys.iter().find(
                            |foreign| foreign.columns.len() == 1 && &foreign.columns[0] == column,
                        )?;
                        let value = page.rows.get(cell.row)?.get(cell.col)?;
                        (!value.is_null()).then(|| {
                            (
                                ObjectRef::new(
                                    foreign.ref_schema.clone(),
                                    foreign.ref_table.clone(),
                                ),
                                foreign.ref_columns.first().cloned().unwrap_or_default(),
                                crate::ui::format::plain_text(value),
                            )
                        })
                    });
                if let Some((object, column, value)) = target {
                    self.follow_foreign_key(tab, object, column, value);
                }
            }
            Action::FocusWhere(tab) => {
                if let Some(workspace) = self.workspace_mut(tab) {
                    workspace.focus_where = true;
                }
            }
            Action::ClearSort { tab, object_tab } => {
                if let Some(object) = self.object_tab_mut(tab, object_tab)
                    && !object.query.sort.is_empty()
                {
                    object.query.sort.clear();
                    object.query.offset = 0;
                    object.selection = None;
                    object.rows.value = None;
                    self.fetch_rows(tab, object_tab);
                }
            }
            Action::FollowForeignKey {
                tab,
                object,
                column,
                value,
            } => self.follow_foreign_key(tab, object, column, value),
            Action::RefreshTree(tab) => self.refresh_tree(tab),
            Action::OpenObject {
                tab,
                object,
                kind,
                pin,
            } => {
                self.open_object(tab, object, kind, pin);
                if let Some(workspace) = self.workspace_mut(tab) {
                    workspace.pane = Pane::Grid;
                }
            }
            Action::SetTreeCursor { tab, node } => {
                if let Some(workspace) = self.workspace_mut(tab) {
                    workspace.tree.cursor = Some(node);
                    workspace.pane = Pane::Tree;
                }
            }
            Action::TreeKey { tab, key } => self.tree_key(tab, key),
            Action::ActivateObjectTab { tab, object_tab } => {
                if let Some(workspace) = self.workspace_mut(tab)
                    && workspace.object_tab(object_tab).is_some()
                {
                    workspace.active_object = Some(object_tab);
                }
            }
            Action::CloseObjectTab { tab, object_tab } => {
                if let Some(workspace) = self.workspace_mut(tab)
                    && let Some(index) = workspace.objects.iter().position(|o| o.id == object_tab)
                {
                    let closed = workspace.objects.remove(index);
                    let session = workspace.session;
                    if workspace.active_object == Some(object_tab) {
                        workspace.active_object = workspace
                            .objects
                            .get(index)
                            .or_else(|| workspace.objects.last())
                            .map(|o| o.id);
                    }
                    // Nothing will show what it was loading (a count can
                    // hold the connection for minutes).
                    self.cancel(session, closed.pending());
                }
            }
            Action::PinObjectTab { tab, object_tab } => {
                if let Some(object) = self.object_tab_mut(tab, object_tab) {
                    object.pinned = true;
                }
            }
            Action::CycleObjectTab { tab, step } => {
                if let Some(workspace) = self.workspace_mut(tab)
                    && !workspace.objects.is_empty()
                {
                    let len = workspace.objects.len() as isize;
                    let current = workspace
                        .active_object
                        .and_then(|id| workspace.objects.iter().position(|o| o.id == id))
                        .unwrap_or(0) as isize;
                    let next = (current + step).rem_euclid(len) as usize;
                    workspace.active_object = Some(workspace.objects[next].id);
                }
            }
            Action::SetView {
                tab,
                object_tab,
                view,
            } => {
                let describe = self.object_tab_mut(tab, object_tab).is_some_and(|object| {
                    object.view = view;
                    view == ObjectView::Structure && object.structure.needs_load()
                });
                if describe {
                    self.describe(tab, object_tab);
                }
            }
            Action::NextPage { tab, object_tab } => {
                let moved = self.object_tab_mut(tab, object_tab).is_some_and(|object| {
                    if !object.page().is_some_and(|page| page.has_more) {
                        return false;
                    }
                    object.query.offset += u64::from(object.query.limit);
                    object.pinned = true;
                    object.selection = None;
                    // Never show rows for a query we no longer display.
                    object.rows.value = None;
                    true
                });
                if moved {
                    self.fetch_rows(tab, object_tab);
                }
            }
            Action::PrevPage { tab, object_tab } => {
                let moved = self.object_tab_mut(tab, object_tab).is_some_and(|object| {
                    if object.query.offset == 0 {
                        return false;
                    }
                    object.query.offset = object
                        .query
                        .offset
                        .saturating_sub(u64::from(object.query.limit));
                    object.pinned = true;
                    object.selection = None;
                    object.rows.value = None;
                    true
                });
                if moved {
                    self.fetch_rows(tab, object_tab);
                }
            }
            Action::SortBy {
                tab,
                object_tab,
                column,
            } => {
                if let Some(object) = self.object_tab_mut(tab, object_tab) {
                    let next = match object.sort_of(&column) {
                        None => Some(SortDir::Asc),
                        Some(SortDir::Asc) => Some(SortDir::Desc),
                        Some(SortDir::Desc) => None,
                    };
                    object.query.sort = next
                        .map(|dir| vec![Sort { column, dir }])
                        .unwrap_or_default();
                    object.query.offset = 0;
                    object.pinned = true;
                    object.selection = None;
                    object.rows.value = None;
                    self.fetch_rows(tab, object_tab);
                }
            }
            Action::SelectCell {
                tab,
                object_tab,
                cell,
            } => {
                if let Some(object) = self.object_tab_mut(tab, object_tab) {
                    object.selection = Some(cell);
                    object.pinned = true;
                }
                if let Some(workspace) = self.workspace_mut(tab) {
                    workspace.pane = Pane::Grid;
                }
            }
            Action::MoveSelection {
                tab,
                object_tab,
                rows,
                cols,
            } => {
                if let Some(object) = self.object_tab_mut(tab, object_tab) {
                    let (height, width) = object
                        .page()
                        .map(|page| (page.rows.len(), page.columns.len()))
                        .unwrap_or((0, 0));
                    if height == 0 || width == 0 {
                        object.selection = None;
                    } else {
                        object.selection = Some(match object.selection {
                            None => CellPos { row: 0, col: 0 },
                            Some(cell) => CellPos {
                                row: step(cell.row, rows, height),
                                col: step(cell.col, cols, width),
                            },
                        });
                        object.pinned = true;
                    }
                }
            }
            Action::ToggleRowPanel(tab) => {
                if let Some(workspace) = self.workspace_mut(tab) {
                    workspace.row_panel = !workspace.row_panel;
                }
            }
            Action::Refresh(tab) => {
                let active = self
                    .workspace(tab)
                    .and_then(|w| w.active_object_tab())
                    .map(|o| {
                        (
                            o.id,
                            o.structure.value.is_some() || o.structure.error.is_some(),
                        )
                    });
                match active {
                    Some((id, described)) => {
                        self.reset_count(tab, id);
                        self.fetch_rows(tab, id);
                        if described {
                            self.describe(tab, id);
                        }
                    }
                    None => self.refresh_tree(tab),
                }
            }
            Action::CancelQuery(tab) => {
                // What the tab shows a spinner for: the active object's
                // loads, or the tree's when no object is open.
                if let Some(workspace) = self.workspace(tab) {
                    let session = workspace.session;
                    let pending: Vec<RequestId> = match workspace.active_object_tab() {
                        Some(object) => object.pending().collect(),
                        None => std::iter::once(workspace.tree.schemas.pending)
                            .chain(workspace.tree.nodes.values().map(|n| n.objects.pending))
                            .chain(std::iter::once(workspace.databases.pending))
                            .flatten()
                            .collect(),
                    };
                    self.cancel(session, pending);
                }
            }
            Action::CountRows { tab, object_tab } => self.count_rows(tab, object_tab),
            Action::ShowHelp => {
                if self.dialog.is_none() {
                    self.dialog = Some(Dialog::Help);
                }
            }
            Action::OpenQuickOpen => {
                let tab = self.active_tab_id();
                if self.dialog.is_none() && self.workspace(tab).is_some() {
                    self.dialog = Some(Dialog::QuickOpen(Box::new(QuickOpen {
                        tab,
                        query: String::new(),
                        selected: 0,
                        scrolled_to: None,
                        scroll_offset: 0.0,
                    })));
                }
            }
            Action::QuickOpenMove(step) => {
                let (tab, query) = match &self.dialog {
                    Some(Dialog::QuickOpen(open)) => (open.tab, open.query.clone()),
                    _ => return,
                };
                let count = self.quick_open_matches(tab, &query).len();
                if let Some(Dialog::QuickOpen(open)) = &mut self.dialog {
                    open.selected = (open.selected as isize + step)
                        .clamp(0, count.saturating_sub(1) as isize)
                        as usize;
                }
            }
            Action::QuickOpenPick => {
                let Some(Dialog::QuickOpen(open)) = &self.dialog else {
                    return;
                };
                let (tab, query, selected) = (open.tab, open.query.clone(), open.selected);
                let matches = self.quick_open_matches(tab, &query);
                if let Some((object, kind)) = matches.get(selected).or(matches.first()).cloned() {
                    self.dialog = None;
                    self.apply(Action::OpenObject {
                        tab,
                        object,
                        kind,
                        pin: true,
                    });
                }
            }
            Action::ToggleFilterBar(tab) => {
                let Some((tab, id)) = self.active_object().filter(|(t, _)| *t == tab) else {
                    return;
                };
                if let Some(object) = self.object_tab_mut(tab, id) {
                    let first = object
                        .page()
                        .and_then(|page| page.columns.first())
                        .map(|column| column.name.clone())
                        .unwrap_or_default();
                    // From the Structure view: show the data, with the bar.
                    let from_structure = object.view != ObjectView::Data;
                    object.view = ObjectView::Data;
                    let bar = &mut object.filter;
                    bar.open = !bar.open || from_structure;
                    bar.focus = bar.open;
                    if bar.open && bar.rows.is_empty() {
                        bar.rows.push(FilterRow {
                            column: first,
                            op: FilterOp::Eq,
                            value: String::new(),
                        });
                    }
                }
            }
            Action::FocusFilterBar(tab) => {
                if let Some((tab, id)) = self.active_object().filter(|(t, _)| *t == tab)
                    && let Some(object) = self.object_tab_mut(tab, id)
                {
                    object.filter.focus = true;
                }
            }
            Action::AddFilterRow { tab, object_tab } => {
                if let Some(object) = self.object_tab_mut(tab, object_tab) {
                    // The last row's column, else the table's first.
                    let column = object
                        .filter
                        .rows
                        .last()
                        .map(|row| row.column.clone())
                        .or_else(|| {
                            object
                                .page()
                                .and_then(|page| page.columns.first())
                                .map(|column| column.name.clone())
                        })
                        .or_else(|| object.filter.columns.first().cloned())
                        .unwrap_or_default();
                    object.filter.rows.push(FilterRow {
                        column,
                        op: FilterOp::Eq,
                        value: String::new(),
                    });
                    object.filter.focus = true;
                }
            }
            Action::RemoveFilterRow {
                tab,
                object_tab,
                index,
            } => {
                if let Some(object) = self.object_tab_mut(tab, object_tab)
                    && index < object.filter.rows.len()
                {
                    object.filter.rows.remove(index);
                }
            }
            Action::ApplyFilters { tab, object_tab } => self.apply_filters(tab, object_tab),
            Action::ClearFilters { tab, object_tab } => {
                if let Some(object) = self.object_tab_mut(tab, object_tab) {
                    object.filter = FilterBar::default();
                }
                self.apply_filters(tab, object_tab);
            }
            // With the filter bar open, Retry runs what the bar shows now
            // (the user may have fixed the value that failed); an unchanged
            // bar retries the same page.
            Action::RetryRows { tab, object_tab } => {
                let bar_changed = self.object_tab_mut(tab, object_tab).is_some_and(|object| {
                    let (filters, raw_where) = object.filter.to_query();
                    object.filter.open
                        && (filters != object.query.filters || raw_where != object.query.raw_where)
                });
                if bar_changed {
                    self.apply_filters(tab, object_tab);
                } else {
                    self.fetch_rows(tab, object_tab);
                }
            }
            Action::RetryStructure { tab, object_tab } => self.describe(tab, object_tab),
            Action::SetDriver(driver) => {
                if let Some(Dialog::Connection(form)) = &mut self.dialog {
                    // A port left at the old driver's default follows the driver.
                    let old_default = form.driver.default_port().to_string();
                    form.driver = driver;
                    if driver != Driver::Sqlite {
                        if form.port.trim().is_empty() || form.port.trim() == old_default {
                            form.port = driver.default_port().to_string();
                        }
                        if form.password_mode == PasswordMode::None {
                            form.password_mode = PasswordMode::Keyring;
                        }
                    }
                }
            }
            Action::SubmitPassword => {
                if !matches!(self.dialog, Some(Dialog::Password(_))) {
                    return;
                }
                let Some(Dialog::Password(prompt)) = self.dialog.take() else {
                    return;
                };
                let PasswordPrompt {
                    tab,
                    kind,
                    password,
                    save,
                    ..
                } = *prompt;
                // A blank answer tries the server without one; it is kept as
                // `Some("")` so it is not asked again (see send_connect).
                let save = save && !password.is_empty();
                if let Some(workspace) = self.workspace_mut(tab) {
                    *kind.slot(&mut workspace.secrets) = Some(password);
                    // Saved only once the server accepts it (see Connected).
                    if kind.is_ssh() {
                        workspace.save_ssh = save;
                        workspace.needs_ssh_prompt = None;
                    } else {
                        workspace.save_password = save;
                        workspace.needs_prompt = None;
                    }
                }
                self.ensure_connecting(tab);
                self.authenticate(tab);
            }
            Action::CancelPassword => {
                if let Some(Dialog::Password(prompt)) = self.dialog.take()
                    && let Some(workspace) = self.workspace_mut(prompt.tab)
                {
                    workspace.status = SessionStatus::Cancelled;
                }
            }
            Action::SwitchDatabase { tab, database } => {
                if let Some(workspace) = self.workspace_mut(tab) {
                    workspace.spec.database = database;
                    workspace.tree = Tree::default();
                    workspace.objects.clear();
                    workspace.active_object = None;
                }
                self.reconnect(tab);
            }
            Action::Backend(event) => self.apply_event(event),
            Action::NewConnection => {
                self.dialog = Some(Dialog::Connection(Box::default()));
            }
            Action::EditConnection(id) => {
                if let Some(saved) = self.connections.get(&id) {
                    self.dialog = Some(Dialog::Connection(Box::new(ConnectionForm::from_saved(
                        saved,
                    ))));
                }
            }
            Action::DuplicateConnection(id) => {
                if self.connections.duplicate(&id).is_some() {
                    self.save_connections();
                }
            }
            Action::DeleteConnection(id) => {
                if let Some(removed) = self.connections.remove(&id) {
                    self.save_connections();
                    if removed.password == PasswordMode::Keyring {
                        self.store_secret(&id, false, None, false);
                    }
                    if removed.ssh_secret == PasswordMode::Keyring {
                        self.store_secret(&id, true, None, false);
                    }
                }
            }
            Action::CloseDialog => self.dialog = None,
            Action::DismissNotice => self.notice = None,
            Action::PickSqliteFile => {
                let request = RequestId(self.next_id());
                if let Some(Dialog::Connection(form)) = &mut self.dialog {
                    form.pick_request = Some(request);
                    form.pick_target = PickTarget::Sqlite;
                    self.backend.pick_sqlite_file(request);
                }
            }
            Action::PickKeyFile => {
                let request = RequestId(self.next_id());
                if let Some(Dialog::Connection(form)) = &mut self.dialog {
                    form.pick_request = Some(request);
                    form.pick_target = PickTarget::KeyFile;
                    self.backend.pick_key_file(request);
                }
            }
            Action::ApplyUrl => {
                if let Some(Dialog::Connection(form)) = &mut self.dialog {
                    apply_url(form);
                }
            }
            Action::TrustHostKey => {
                if let Some(Dialog::HostKey(prompt)) = self.dialog.take() {
                    self.trust(&prompt.host, prompt.port, &prompt.fingerprint);
                    self.reconnect(prompt.tab);
                }
            }
            Action::CancelHostKey => {
                if matches!(self.dialog, Some(Dialog::HostKey(_))) {
                    self.dialog = None;
                }
            }
            Action::TrustTestHostKey => {
                let untrusted = match &self.dialog {
                    Some(Dialog::Connection(form)) => match &form.test {
                        TestState::Untrusted {
                            host,
                            port,
                            fingerprint,
                        } => Some((host.clone(), *port, fingerprint.clone())),
                        _ => None,
                    },
                    _ => None,
                };
                if let Some((host, port, fingerprint)) = untrusted {
                    self.trust(&host, port, &fingerprint);
                    self.apply(Action::TestConnection);
                }
            }
            Action::TestConnection => {
                let request = RequestId(self.next_id());
                let Some(Dialog::Connection(form)) = &mut self.dialog else {
                    return;
                };
                let spec = match form.to_spec() {
                    Ok(spec) => spec,
                    Err(message) => {
                        form.message = Some(message);
                        return;
                    }
                };
                form.message = None;
                form.test = TestState::Running(request);
                // Typed secrets are used as they are; unchanged saved ones
                // are read from the keyring first, and the test runs once
                // the last one arrives.
                let mut secrets = Secrets::default();
                let mut loads = Vec::new();
                let ssh_kind = SecretKind::for_ssh(&spec);
                for (kind, typed, saved, stale, account) in [
                    (
                        Some(SecretKind::Database),
                        &form.password,
                        form.password_is_saved(),
                        form.password_is_stale(),
                        password_account as fn(&crate::connections::ConnectionId) -> String,
                    ),
                    (
                        ssh_kind,
                        &form.ssh_secret,
                        form.ssh_secret_is_saved(),
                        form.ssh_secret_is_stale(),
                        ssh_account,
                    ),
                ] {
                    let Some(kind) = kind else {
                        continue;
                    };
                    if !typed.is_empty() {
                        *kind.slot(&mut secrets) = Some(typed.clone());
                    } else if saved && let Some(id) = &form.editing {
                        loads.push((kind, account(id)));
                    } else if stale {
                        // The saved secret was for another server: never
                        // send it anywhere else.
                        form.test = TestState::Failed(if kind.is_ssh() {
                            "The SSH server changed; type its secret to test.".into()
                        } else {
                            "The server changed; type its password to test.".into()
                        });
                        return;
                    }
                }
                form.test_spec = Some(spec.clone());
                form.test_secrets = secrets.clone();
                form.test_waiting = loads.len() as u8;
                if loads.is_empty() {
                    self.backend.send(Command::Test {
                        request,
                        spec,
                        secrets,
                        host_keys: self.host_keys.clone(),
                    });
                } else {
                    for (kind, account) in loads {
                        // Each load has its own id; the test keeps `request`.
                        let load = RequestId(self.next_id());
                        self.pending_secrets.insert(
                            load,
                            SecretPurpose::Test {
                                test: request,
                                kind,
                            },
                        );
                        self.backend.send(Command::LoadSecret {
                            request: load,
                            account,
                        });
                    }
                }
            }
            Action::SaveConnection { connect } => self.save_dialog(connect),
        }
    }

    fn close_tab(&mut self, id: ConnTabId) {
        let Some(index) = self.tab_index(id) else {
            return;
        };
        if let ConnTabContent::Workspace(workspace) = &self.tabs[index].content {
            self.backend.send(Command::Close {
                session: workspace.session,
            });
        }
        self.tabs.remove(index);
        if self.tabs.is_empty() {
            let tab = self.picker_tab();
            self.tabs.push(tab);
            self.active = 0;
        } else if index < self.active {
            self.active -= 1;
        } else {
            self.active = self.active.min(self.tabs.len() - 1);
        }
    }

    /// Opens `saved` in `tab`; `typed` holds secrets typed in the dialog.
    fn connect_tab(&mut self, tab: ConnTabId, saved: SavedConnection, typed: Secrets) {
        self.open_workspace(tab, saved, typed);
        self.authenticate(tab);
    }

    /// Puts a connecting workspace for `saved` in `tab`, without starting to
    /// authenticate.
    fn open_workspace(&mut self, tab: ConnTabId, saved: SavedConnection, typed: Secrets) {
        let session = SessionId(self.next_id());
        let request = RequestId(self.next_id());
        let environment = saved.environment;
        let Some(entry) = self.tabs.iter_mut().find(|t| t.id == tab) else {
            return;
        };
        entry.content = ConnTabContent::Workspace(Box::new(Workspace {
            session,
            conn_id: saved.id,
            environment,
            name: saved.name,
            driver: saved.spec.driver,
            encrypted: false,
            spec: saved.spec,
            status: SessionStatus::Connecting { request },
            tree: Tree::default(),
            objects: Vec::new(),
            active_object: None,
            row_panel: true,
            pending_open: None,
            password_mode: saved.password,
            secrets: typed,
            databases: Fetch::default(),
            save_password: false,
            needs_prompt: None,
            ssh_mode: saved.ssh_secret,
            save_ssh: false,
            needs_ssh_prompt: None,
            pane: Pane::Tree,
            recent: Vec::new(),
            sidebar_hidden: false,
            full_precision: false,
            focus_where: false,
            fold_documents: None,
        }));
    }

    /// Fills the tab's secrets one at a time (the database password, then
    /// the SSH secret), each from the keyring or a prompt, then connects.
    fn authenticate(&mut self, tab: ConnTabId) {
        let Some(workspace) = self.workspace(tab) else {
            return;
        };
        let mut steps = vec![(
            SecretKind::Database,
            workspace.password_mode,
            workspace.secrets.password.is_some(),
            workspace.needs_prompt.clone(),
        )];
        if let Some(kind) = SecretKind::for_ssh(&workspace.spec) {
            let mut secrets = workspace.secrets.clone();
            steps.push((
                kind,
                workspace.ssh_mode,
                kind.slot(&mut secrets).is_some(),
                workspace.needs_ssh_prompt.clone(),
            ));
        }
        let conn = workspace.conn_id.clone();
        let secrets = workspace.secrets.clone();
        let SessionStatus::Connecting { request: attempt } = workspace.status else {
            return;
        };
        for (kind, mode, known, reason) in steps {
            if let Some(reason) = reason {
                self.prompt_password(tab, kind, Some(reason));
                return;
            }
            if known || mode == PasswordMode::None {
                continue;
            }
            if mode == PasswordMode::Keyring {
                let account = if kind.is_ssh() {
                    ssh_account(&conn)
                } else {
                    password_account(&conn)
                };
                let request = RequestId(self.next_id());
                self.pending_secrets.insert(
                    request,
                    SecretPurpose::Connect {
                        tab,
                        request: attempt,
                        kind,
                    },
                );
                self.backend.send(Command::LoadSecret { request, account });
            } else {
                self.prompt_password(tab, kind, None);
            }
            return;
        }
        self.send_connect(tab, secrets);
    }

    /// Sends the Connect for the tab's current session and request.
    fn send_connect(&mut self, tab: ConnTabId, secrets: Secrets) {
        let Some(workspace) = self.workspace_mut(tab) else {
            return;
        };
        let SessionStatus::Connecting { request } = workspace.status else {
            return;
        };
        workspace.secrets = secrets.clone();
        let (session, spec) = (workspace.session, workspace.spec.clone());
        // A blank answer means "no password" to the server.
        let mut secrets = secrets;
        for slot in [
            &mut secrets.password,
            &mut secrets.ssh_password,
            &mut secrets.ssh_passphrase,
        ] {
            if slot.as_deref() == Some("") {
                *slot = None;
            }
        }
        self.backend.send(Command::Connect {
            session,
            request,
            spec,
            secrets,
            host_keys: self.host_keys.clone(),
        });
    }

    fn prompt_password(&mut self, tab: ConnTabId, kind: SecretKind, message: Option<String>) {
        let dialog_open = self.dialog.is_some();
        let Some(workspace) = self.workspace_mut(tab) else {
            return;
        };
        if dialog_open {
            // Never replace a dialog the user is working in: wait for
            // Reconnect, which asks then.
            if kind.is_ssh() {
                let reason = message.unwrap_or_else(|| "The SSH login needs a secret.".into());
                workspace.status = SessionStatus::Disconnected(Error::Ssh {
                    stage: SshStage::Secret,
                    message: reason.clone(),
                });
                workspace.needs_ssh_prompt = Some(reason);
            } else {
                let reason = message.unwrap_or_else(|| "A password is needed to connect.".into());
                workspace.status = SessionStatus::Disconnected(Error::Auth(reason.clone()));
                workspace.needs_prompt = Some(reason);
            }
            return;
        }
        let mode = if kind.is_ssh() {
            workspace.ssh_mode
        } else {
            workspace.password_mode
        };
        let prompt = PasswordPrompt {
            tab,
            kind,
            name: workspace.name.clone(),
            password: String::new(),
            // A secret with no saved mode yet (an encrypted key found at
            // connect time) is offered for the keyring too.
            save: mode != PasswordMode::Ask,
            message,
        };
        self.dialog = Some(Dialog::Password(Box::new(prompt)));
    }

    /// Gives the tab a fresh session and request when it is not already
    /// connecting (a prompt answered after a failure).
    fn ensure_connecting(&mut self, tab: ConnTabId) {
        if self
            .workspace(tab)
            .is_none_or(|workspace| matches!(workspace.status, SessionStatus::Connecting { .. }))
        {
            return;
        }
        let session = SessionId(self.next_id());
        let request = RequestId(self.next_id());
        if let Some(workspace) = self.workspace_mut(tab) {
            let old = std::mem::replace(&mut workspace.session, session);
            workspace.status = SessionStatus::Connecting { request };
            self.backend.send(Command::Close { session: old });
        }
    }

    /// After a successful connect, saves a password the prompt was asked
    /// to keep.
    fn save_accepted_password(&mut self, tab: ConnTabId) {
        self.save_accepted_ssh_secret(tab);
        let Some(workspace) = self.workspace_mut(tab) else {
            return;
        };
        workspace.needs_prompt = None;
        if !std::mem::take(&mut workspace.save_password) {
            return;
        }
        let Some(password) = workspace.secrets.password.clone() else {
            return;
        };
        let conn = workspace.conn_id.clone();
        // Keyring mode is recorded once the keyring has it (SecretStored).
        self.store_secret(&conn, false, Some(SecretString(password)), true);
    }

    /// The SSH half of `save_accepted_password`.
    fn save_accepted_ssh_secret(&mut self, tab: ConnTabId) {
        let Some(workspace) = self.workspace_mut(tab) else {
            return;
        };
        workspace.needs_ssh_prompt = None;
        if !std::mem::take(&mut workspace.save_ssh) {
            return;
        }
        let Some(kind) = SecretKind::for_ssh(&workspace.spec) else {
            return;
        };
        let mut secrets = workspace.secrets.clone();
        let Some(secret) = kind.slot(&mut secrets).clone() else {
            return;
        };
        let conn = workspace.conn_id.clone();
        self.store_secret(&conn, true, Some(SecretString(secret)), true);
    }

    /// Writes (`Some`) or deletes (`None`) a connection's database password
    /// or SSH secret in the keyring. With `adopt`, the connection switches
    /// to keyring mode once the write succeeds. Nothing is written for a
    /// connection that no longer exists.
    fn store_secret(
        &mut self,
        conn: &crate::connections::ConnectionId,
        ssh: bool,
        secret: Option<SecretString>,
        adopt: bool,
    ) {
        if secret.is_some() && self.connections.get(conn).is_none() {
            return;
        }
        let request = RequestId(self.next_id());
        self.pending_stores.insert(
            request,
            PendingStore {
                conn: conn.clone(),
                ssh,
                saving: secret.is_some(),
                adopt,
            },
        );
        self.backend.send(Command::StoreSecret {
            request,
            account: if ssh {
                ssh_account(conn)
            } else {
                password_account(conn)
            },
            secret,
        });
    }

    /// A keyring write or delete finished.
    fn secret_stored(&mut self, request: RequestId, result: Result<(), String>) {
        let Some(store) = self.pending_stores.remove(&request) else {
            return;
        };
        fn mode(saved: &mut SavedConnection, ssh: bool) -> &mut PasswordMode {
            if ssh {
                &mut saved.ssh_secret
            } else {
                &mut saved.password
            }
        }
        match result {
            Ok(()) if store.adopt => {
                let Some(saved) = self
                    .connections
                    .connections
                    .iter_mut()
                    .find(|c| c.id == store.conn)
                else {
                    return;
                };
                *mode(saved, store.ssh) = PasswordMode::Keyring;
                self.save_connections();
                for tab in &mut self.tabs {
                    if let ConnTabContent::Workspace(workspace) = &mut tab.content
                        && workspace.conn_id == store.conn
                    {
                        if store.ssh {
                            workspace.ssh_mode = PasswordMode::Keyring;
                        } else {
                            workspace.password_mode = PasswordMode::Keyring;
                        }
                    }
                }
            }
            Ok(()) => {}
            Err(error) => {
                let what = if store.ssh { "SSH secret" } else { "password" };
                self.notice = Some(if store.saving {
                    format!("Could not save the {what} in the keyring: {error}.")
                } else {
                    format!("Could not remove the {what} from the keyring: {error}.")
                });
                // What the keyring holds is unknown now (maybe an older
                // secret, maybe one for a server the connection no longer
                // points at): ask next time instead of reading it.
                if let Some(saved) = self
                    .connections
                    .connections
                    .iter_mut()
                    .find(|c| c.id == store.conn)
                    && *mode(saved, store.ssh) == PasswordMode::Keyring
                {
                    *mode(saved, store.ssh) = PasswordMode::Ask;
                    self.save_connections();
                }
            }
        }
    }

    fn reconnect(&mut self, tab: ConnTabId) {
        let session = SessionId(self.next_id());
        let request = RequestId(self.next_id());
        let Some(workspace) = self.workspace_mut(tab) else {
            return;
        };
        let old = std::mem::replace(&mut workspace.session, session);
        workspace.status = SessionStatus::Connecting { request };
        self.backend.send(Command::Close { session: old });
        // Reuses the secrets this tab already has; asks only for missing ones.
        self.authenticate(tab);
    }

    fn apply_event(&mut self, event: Event) {
        match event {
            Event::Connected {
                session,
                request,
                driver,
                encrypted,
            } => {
                let adopted = self.tab_for_session(session).and_then(|tab| {
                    let workspace = self.workspace_mut(tab)?;
                    match workspace.status {
                        SessionStatus::Connecting { request: waiting } if waiting == request => {
                            workspace.status = SessionStatus::Connected;
                            workspace.driver = driver;
                            workspace.encrypted = encrypted;
                            Some(tab)
                        }
                        _ => None,
                    }
                });
                match adopted {
                    Some(tab) => {
                        self.save_accepted_password(tab);
                        self.mark_used(tab);
                        self.after_connect(tab);
                    }
                    // Nobody is waiting for this session any more.
                    None => self.backend.send(Command::Close { session }),
                }
            }
            Event::ConnectFailed {
                session,
                request,
                error,
            } => {
                let Some(tab) = self.tab_for_session(session) else {
                    return;
                };
                let Some(workspace) = self.workspace_mut(tab) else {
                    return;
                };
                if !matches!(workspace.status, SessionStatus::Connecting { request: r } if r == request)
                {
                    return;
                }
                let rejected = matches!(error, Error::Auth(_))
                    && workspace.password_mode != PasswordMode::None;
                if rejected {
                    // Never resend or save a password the server refused.
                    workspace.secrets.password = None;
                    workspace.save_password = false;
                    workspace.needs_prompt = Some(error.to_string());
                }
                let ssh_rejected = match &error {
                    Error::Ssh {
                        stage: SshStage::Secret,
                        ..
                    } => SecretKind::for_ssh(&workspace.spec),
                    _ => None,
                };
                if ssh_rejected.is_some() {
                    // Likewise for the SSH password or passphrase.
                    workspace.secrets.ssh_password = None;
                    workspace.secrets.ssh_passphrase = None;
                    workspace.save_ssh = false;
                    workspace.needs_ssh_prompt = Some(error.to_string());
                }
                let message = error.to_string();
                let unknown_key = match &error {
                    Error::Ssh {
                        stage: SshStage::HostKeyUnknown { fingerprint },
                        ..
                    } => workspace
                        .spec
                        .ssh
                        .as_ref()
                        .map(|ssh| (ssh.host.clone(), ssh.port, fingerprint.clone())),
                    _ => None,
                };
                workspace.status = SessionStatus::Disconnected(error);
                if rejected {
                    self.prompt_password(tab, SecretKind::Database, Some(message.clone()));
                }
                if let Some(kind) = ssh_rejected {
                    self.prompt_password(tab, kind, Some(message));
                }
                // A new SSH host: ask, unless the user is busy in a dialog
                // (the tab shows the error and Reconnect asks then).
                if let Some((host, port, fingerprint)) = unknown_key {
                    if let Some(refusal) = self.refuse_to_trust(&host, port, &fingerprint) {
                        if let Some(workspace) = self.workspace_mut(tab) {
                            workspace.status = SessionStatus::Disconnected(refusal);
                        }
                    } else if self.dialog.is_none() {
                        self.dialog = Some(Dialog::HostKey(Box::new(HostKeyPrompt {
                            tab,
                            host,
                            port,
                            fingerprint,
                        })));
                    }
                }
            }
            Event::Disconnected { session, error } => {
                if let Some(tab) = self.tab_for_session(session)
                    && let Some(workspace) = self.workspace_mut(tab)
                {
                    workspace.status = SessionStatus::Disconnected(error);
                }
            }
            Event::Tested { request, result } => {
                if let Some(Dialog::Connection(form)) = &mut self.dialog
                    && form.test == TestState::Running(request)
                {
                    // The key belongs to the SSH host the test reached, not
                    // whatever the fields say now.
                    let tested = form
                        .test_spec
                        .as_ref()
                        .and_then(|spec| spec.ssh.as_ref())
                        .map(|ssh| (ssh.host.clone(), ssh.port));
                    form.test = match (result, tested) {
                        (Ok(()), _) => TestState::Passed,
                        (
                            Err(Error::Ssh {
                                stage: SshStage::HostKeyUnknown { fingerprint },
                                ..
                            }),
                            Some((host, port)),
                        ) if self.host_keys_error.is_none() => TestState::Untrusted {
                            host,
                            port,
                            fingerprint,
                        },
                        (Err(error), _) => TestState::Failed(error.to_string()),
                    };
                }
            }
            Event::FilePicked { request, path } => {
                if let Some(Dialog::Connection(form)) = &mut self.dialog
                    && form.pick_request == Some(request)
                {
                    form.pick_request = None;
                    match (path, form.pick_target) {
                        (Some(path), PickTarget::KeyFile) => {
                            form.ssh_key_file = path.display().to_string();
                        }
                        (Some(path), PickTarget::Sqlite) => {
                            form.sqlite_path = path.display().to_string();
                            if form.name.trim().is_empty() {
                                form.name = crate::model::file_name(&form.sqlite_path);
                            }
                        }
                        (None, _) => {}
                    }
                }
            }
            Event::Schemas {
                session,
                request,
                result,
            } => {
                let Some(tab) = self.tab_for_session(session) else {
                    return;
                };
                let Some(workspace) = self.workspace_mut(tab) else {
                    return;
                };
                if !workspace.tree.schemas.finish(request, result) {
                    return;
                }
                let database = workspace.spec.database.clone();
                let default = workspace.tree.schemas.value.as_ref().and_then(|schemas| {
                    schemas
                        .iter()
                        .find(|schema| !database.is_empty() && **schema == database)
                        .or_else(|| {
                            schemas
                                .iter()
                                .find(|schema| *schema == "public" || *schema == "main")
                        })
                        .or(if schemas.len() == 1 {
                            schemas.first()
                        } else {
                            None
                        })
                        .cloned()
                });
                if let Some(schema) = default {
                    self.expand_schema(tab, &schema);
                }
            }
            Event::Objects {
                session,
                request,
                schema,
                result,
            } => {
                if let Some(tab) = self.tab_for_session(session)
                    && let Some(workspace) = self.workspace_mut(tab)
                    && let Some(node) = workspace.tree.nodes.get_mut(&schema)
                {
                    node.objects.finish(request, result);
                }
            }
            Event::Rows {
                session,
                request,
                result,
            } => {
                let Some(tab) = self.tab_for_session(session) else {
                    return;
                };
                let Some(workspace) = self.workspace_mut(tab) else {
                    return;
                };
                let Some(object) = workspace
                    .objects
                    .iter_mut()
                    .find(|o| o.rows.pending == Some(request))
                else {
                    return;
                };
                object.rows.finish(request, result);
                let (height, width) = object
                    .page()
                    .map(|page| (page.rows.len(), page.columns.len()))
                    .unwrap_or((0, 0));
                object.selection = match object.selection {
                    Some(_) if height == 0 || width == 0 => None,
                    Some(cell) => Some(CellPos {
                        row: cell.row.min(height - 1),
                        col: cell.col.min(width - 1),
                    }),
                    None => None,
                };
            }
            Event::Structure {
                session,
                request,
                result,
            } => {
                if let Some(tab) = self.tab_for_session(session)
                    && let Some(workspace) = self.workspace_mut(tab)
                    && let Some(object) = workspace
                        .objects
                        .iter_mut()
                        .find(|o| o.structure.pending == Some(request))
                {
                    object.structure.finish(request, result);
                }
            }
            Event::SecretLoaded { request, result } => {
                match self.pending_secrets.remove(&request) {
                    Some(SecretPurpose::Connect {
                        tab,
                        request: attempt,
                        kind,
                    }) if self.workspace(tab).is_some_and(|workspace| {
                        matches!(workspace.status, SessionStatus::Connecting { request } if request == attempt)
                    }) =>
                    {
                        match result {
                        Ok(Some(secret)) => {
                            if let Some(workspace) = self.workspace_mut(tab) {
                                *kind.slot(&mut workspace.secrets) = Some(secret.0);
                            }
                            self.authenticate(tab);
                        }
                        Ok(None) => self.prompt_password(
                            tab,
                            kind,
                            Some(if kind.is_ssh() {
                                "No SSH secret is saved in the keyring for this connection.".into()
                            } else {
                                "No password is saved in the keyring for this connection.".into()
                            }),
                        ),
                        Err(message) => self.prompt_password(
                            tab,
                            kind,
                            Some(format!("Could not read the saved password: {message}.")),
                        ),
                        }
                    }
                    // An earlier attempt's answer: the tab has moved on.
                    Some(SecretPurpose::Connect { .. }) => {}
                    Some(SecretPurpose::Test { test, kind }) => {
                        let Some(Dialog::Connection(form)) = &mut self.dialog else {
                            return;
                        };
                        if form.test != TestState::Running(test) {
                            return;
                        }
                        match result {
                            Ok(Some(secret)) => {
                                *kind.slot(&mut form.test_secrets) = Some(secret.0);
                                form.test_waiting = form.test_waiting.saturating_sub(1);
                                // The spec the Test started with: fields
                                // edited meanwhile never get the secret.
                                if form.test_waiting == 0
                                    && let Some(spec) = form.test_spec.clone()
                                {
                                    self.backend.send(Command::Test {
                                        request: test,
                                        spec,
                                        secrets: form.test_secrets.clone(),
                                        host_keys: self.host_keys.clone(),
                                    });
                                }
                            }
                            Ok(None) => {
                                form.test = TestState::Failed(if kind.is_ssh() {
                                    "No SSH secret is saved; type it to test.".into()
                                } else {
                                    "No password is saved; type it to test.".into()
                                })
                            }
                            Err(message) => form.test = TestState::Failed(message),
                        }
                    }
                    None => {}
                }
            }
            Event::SecretStored { request, result } => self.secret_stored(request, result),
            Event::Saved { path, result } => {
                if let Err(error) = result {
                    self.notice = Some(format!("Could not save {}: {error}.", path.display()));
                }
            }
            Event::Databases {
                session,
                request,
                result,
            } => {
                if let Some(tab) = self.tab_for_session(session)
                    && let Some(workspace) = self.workspace_mut(tab)
                {
                    workspace.databases.finish(request, result);
                }
            }
            Event::Count {
                session,
                request,
                result,
            } => {
                if let Some(tab) = self.tab_for_session(session)
                    && let Some(workspace) = self.workspace_mut(tab)
                    && let Some(object) = workspace
                        .objects
                        .iter_mut()
                        .find(|o| o.count.pending == Some(request))
                {
                    object.count.finish(request, result);
                }
            }
        }
    }

    /// Why a key seen as unknown must not be offered for trust: the store
    /// could not be read (trusting would overwrite it), or the host already
    /// has a different trusted key (a late or forged "unknown").
    fn refuse_to_trust(&self, host: &str, port: u16, fingerprint: &str) -> Option<Error> {
        if let Some(problem) = &self.host_keys_error {
            return Some(Error::Ssh {
                stage: SshStage::HostKeyUnknown {
                    fingerprint: fingerprint.to_owned(),
                },
                message: format!(
                    "{} could not be read ({problem}); fix or remove it before trusting new hosts",
                    self.dirs.known_hosts_file().display()
                ),
            });
        }
        match self.host_keys.fingerprint(host, port) {
            Some(known) if known != fingerprint => Some(Error::Ssh {
                stage: SshStage::HostKeyMismatch {
                    fingerprint: fingerprint.to_owned(),
                },
                message: "the host key changed since you trusted it, which can mean someone \
                          is intercepting the connection"
                    .into(),
            }),
            _ => None,
        }
    }

    /// Trusts an SSH host key and saves the store. Never replaces a
    /// different trusted key, and never writes over an unreadable store.
    fn trust(&mut self, host: &str, port: u16, fingerprint: &str) {
        if self.refuse_to_trust(host, port, fingerprint).is_some() {
            log::warn!("refused to trust a host key for {host}:{port}");
            return;
        }
        self.host_keys.trust(host, port, fingerprint);
        self.backend.send(Command::Save {
            path: self.dirs.known_hosts_file(),
            file: StateFile::KnownHosts(self.host_keys.clone()),
        });
    }

    /// Remembers that `tab`'s saved connection connected now.
    fn mark_used(&mut self, tab: ConnTabId) {
        let Some(conn) = self
            .workspace(tab)
            .map(|workspace| workspace.conn_id.clone())
        else {
            return;
        };
        if self.connections.get(&conn).is_none() {
            return;
        }
        self.connections.mark_used(&conn, crate::util::now_secs());
        self.save_connections();
    }

    /// Saves the connections on the backend (writing syncs the disk, which
    /// can stall a frame).
    fn save_connections(&mut self) {
        self.backend.send(Command::Save {
            path: self.dirs.connections_file(),
            file: StateFile::Connections(self.connections.clone()),
        });
    }

    fn save_dialog(&mut self, connect: bool) {
        let Some(Dialog::Connection(form)) = &mut self.dialog else {
            return;
        };
        // The connection being edited was deleted while the dialog was open:
        // saving must not bring it back.
        if let Some(id) = &form.editing
            && self.connections.get(id).is_none()
        {
            form.message = Some("This connection was deleted.".into());
            return;
        }
        let saved = match form.to_saved() {
            Ok(saved) => saved,
            Err(message) => {
                form.message = Some(message);
                return;
            }
        };
        let typed = (!form.password.is_empty()).then(|| form.password.clone());
        let typed_ssh = (!form.ssh_secret.is_empty()).then(|| form.ssh_secret.clone());
        // A secret saved for a server the connection no longer points at is
        // deleted, so connecting asks for the new one instead of sending it.
        let stale = form.password_is_stale();
        let stale_ssh = form.ssh_secret_is_stale();
        let previous = self.connections.get(&saved.id).map(|c| c.password);
        let previous_ssh = self.connections.get(&saved.id).map(|c| c.ssh_secret);
        self.dialog = None;
        self.connections.upsert(saved.clone());
        self.save_connections();
        if saved.password == PasswordMode::Keyring {
            if typed.is_some() || stale {
                self.store_secret(&saved.id, false, typed.clone().map(SecretString), false);
            }
        } else if previous == Some(PasswordMode::Keyring) {
            self.store_secret(&saved.id, false, None, false);
        }
        if saved.ssh_secret == PasswordMode::Keyring {
            if typed_ssh.is_some() || stale_ssh {
                self.store_secret(&saved.id, true, typed_ssh.clone().map(SecretString), false);
            }
        } else if previous_ssh == Some(PasswordMode::Keyring) {
            self.store_secret(&saved.id, true, None, false);
        }
        if connect {
            if !matches!(self.active_tab().content, ConnTabContent::Picker(_)) {
                self.apply(Action::NewConnTab);
            }
            let tab = self.active_tab_id();
            let mut secrets = Secrets {
                password: typed.clone(),
                ..Secrets::default()
            };
            if let (Some(kind), Some(secret)) = (SecretKind::for_ssh(&saved.spec), &typed_ssh) {
                *kind.slot(&mut secrets) = Some(secret.clone());
            }
            self.open_workspace(tab, saved, secrets);
            // Ask for a secret whose saved copy was for the old server,
            // rather than trusting the keyring to have dropped it.
            if let Some(workspace) = self.workspace_mut(tab) {
                if stale && typed.is_none() {
                    workspace.needs_prompt =
                        Some("The server changed. Enter the password for it.".into());
                }
                if stale_ssh && typed_ssh.is_none() {
                    workspace.needs_ssh_prompt =
                        Some("The SSH server changed. Enter the secret for it.".into());
                }
            }
            self.authenticate(tab);
        }
    }

    /// After a (re)connect: load the tree, restart everything that waited on
    /// the old session, and open anything queued for this connection.
    pub fn after_connect(&mut self, tab: ConnTabId) {
        if let Some(workspace) = self.workspace(tab)
            && workspace.driver == Driver::Postgres
        {
            let session = workspace.session;
            let request = RequestId(self.next_id());
            if let Some(workspace) = self.workspace_mut(tab) {
                workspace.databases.start(request);
            }
            self.backend
                .send(Command::ListDatabases { session, request });
        }
        self.refresh_tree(tab);
        let Some(workspace) = self.workspace_mut(tab) else {
            return;
        };
        let active = workspace.active_object;
        let pending = workspace.pending_open.take();
        // A count queued on the old session will never answer, and one the
        // lost connection failed is worth another try.
        for object in &mut workspace.objects {
            if object.count.is_loading() || lost(&object.count) {
                // The old session is closed, so its count never runs.
                let _ = object.reset_count();
            }
        }
        let stale: Vec<(ObjectTabId, bool, bool)> = workspace
            .objects
            .iter()
            .map(|object| {
                let is_active = Some(object.id) == active;
                (
                    object.id,
                    is_active || object.rows.is_loading() || lost(&object.rows),
                    object.structure.is_loading()
                        || lost(&object.structure)
                        || (is_active && object.structure.value.is_some()),
                )
            })
            .collect();
        for (id, rows, structure) in stale {
            if rows {
                self.fetch_rows(tab, id);
            }
            if structure {
                self.describe(tab, id);
            }
        }
        if let Some((object, kind)) = pending {
            self.open_object(tab, object, kind, true);
        }
    }
}

/// Whether the request failed only because the connection went away.
fn lost<T>(fetch: &Fetch<T>) -> bool {
    fetch.error.as_ref().is_some_and(Error::is_connection_lost)
}

impl App {
    pub fn load_schemas(&mut self, tab: ConnTabId) {
        let request = RequestId(self.next_id());
        let Some(workspace) = self.workspace_mut(tab) else {
            return;
        };
        workspace.tree.schemas.start(request);
        let session = workspace.session;
        self.backend.send(Command::ListSchemas { session, request });
    }

    pub fn load_objects(&mut self, tab: ConnTabId, schema: &str) {
        let request = RequestId(self.next_id());
        let Some(workspace) = self.workspace_mut(tab) else {
            return;
        };
        workspace
            .tree
            .nodes
            .entry(schema.to_owned())
            .or_default()
            .objects
            .start(request);
        let session = workspace.session;
        self.backend.send(Command::ListObjects {
            session,
            request,
            schema: schema.to_owned(),
        });
    }

    pub fn expand_schema(&mut self, tab: ConnTabId, schema: &str) {
        let Some(workspace) = self.workspace_mut(tab) else {
            return;
        };
        let node = workspace.tree.nodes.entry(schema.to_owned()).or_default();
        node.expanded = true;
        if node.objects.needs_load() || node.objects.error.is_some() {
            self.load_objects(tab, schema);
        }
    }

    fn refresh_tree(&mut self, tab: ConnTabId) {
        self.load_schemas(tab);
        let expanded: Vec<String> = self
            .workspace(tab)
            .map(|workspace| {
                workspace
                    .tree
                    .nodes
                    .iter()
                    .filter(|(_, node)| node.expanded || node.objects.is_loading())
                    .map(|(name, _)| name.clone())
                    .collect()
            })
            .unwrap_or_default();
        for schema in expanded {
            self.load_objects(tab, &schema);
        }
    }

    fn object_tab_mut(&mut self, tab: ConnTabId, id: ObjectTabId) -> Option<&mut ObjectTab> {
        self.workspace_mut(tab)?.object_tab_mut(id)
    }

    /// The connection tab and object tab the keyboard acts on.
    pub fn active_object(&self) -> Option<(ConnTabId, ObjectTabId)> {
        let tab = self.active_tab_id();
        let workspace = self.workspace(tab)?;
        Some((tab, workspace.active_object?))
    }

    pub fn open_object(&mut self, tab: ConnTabId, object: ObjectRef, kind: ObjectKind, pin: bool) {
        let page_size = self.settings.page_size;
        let new_id = ObjectTabId(self.next_id());
        let Some(workspace) = self.workspace_mut(tab) else {
            return;
        };
        workspace.recent.retain(|(seen, _)| *seen != object);
        workspace.recent.insert(0, (object.clone(), kind));
        workspace.recent.truncate(crate::model::RECENT);
        workspace.tree.reveal(&object);
        if let Some(existing) = workspace.objects.iter_mut().find(|o| o.object == object) {
            existing.pinned |= pin;
            workspace.active_object = Some(existing.id);
            return;
        }
        let estimate = workspace
            .tree
            .object_info(&object)
            .and_then(|info| info.estimated_rows);
        let opened = ObjectTab::new(new_id, object, kind, pin, page_size, estimate);
        let preview = if pin {
            None
        } else {
            workspace.objects.iter().position(|o| !o.pinned)
        };
        let session = workspace.session;
        let replaced = match preview {
            Some(index) => Some(std::mem::replace(&mut workspace.objects[index], opened)),
            None => {
                workspace.objects.push(opened);
                None
            }
        };
        workspace.active_object = Some(new_id);
        if let Some(replaced) = replaced {
            self.cancel(session, replaced.pending());
        }
        // The keys and foreign keys label the grid and the row panel.
        self.describe(tab, new_id);
        self.fetch_rows(tab, new_id);
    }

    /// Opens `object` filtered to the rows whose `column` is `value`: where
    /// a foreign key points.
    fn follow_foreign_key(
        &mut self,
        tab: ConnTabId,
        object: ObjectRef,
        column: String,
        value: String,
    ) {
        let kind = self
            .workspace(tab)
            .and_then(|workspace| workspace.tree.object_info(&object))
            .map_or(ObjectKind::Table, |info| info.kind);
        self.open_object(tab, object, kind, true);
        let Some(id) = self
            .workspace(tab)
            .and_then(|workspace| workspace.active_object)
        else {
            return;
        };
        if let Some(opened) = self.object_tab_mut(tab, id) {
            opened.filter.rows = vec![crate::model::FilterRow {
                column,
                op: tabletist_db::FilterOp::Eq,
                value,
            }];
        }
        self.apply_filters(tab, id);
        if let Some(workspace) = self.workspace_mut(tab) {
            workspace.pane = Pane::Grid;
        }
    }

    /// Stops requests nothing waits for any more: a running one is
    /// cancelled, a queued one never runs.
    fn cancel(&mut self, session: SessionId, requests: impl IntoIterator<Item = RequestId>) {
        for request in requests {
            self.backend.send(Command::Cancel { session, request });
        }
    }

    /// Forgets the object tab's exact count and stops it if still running.
    fn reset_count(&mut self, tab: ConnTabId, id: ObjectTabId) {
        let Some(workspace) = self.workspace_mut(tab) else {
            return;
        };
        let session = workspace.session;
        let running = workspace
            .object_tab_mut(id)
            .and_then(ObjectTab::reset_count);
        self.cancel(session, running);
    }

    /// Loads the object tab's rows, replacing any load still pending.
    pub fn fetch_rows(&mut self, tab: ConnTabId, id: ObjectTabId) {
        let request = RequestId(self.next_id());
        let Some(workspace) = self.workspace_mut(tab) else {
            return;
        };
        let session = workspace.session;
        let Some(object) = workspace.object_tab_mut(id) else {
            return;
        };
        let superseded = object.rows.pending;
        object.rows.start(request);
        let query = object.query.clone();
        self.cancel(session, superseded);
        self.backend.send(Command::FetchRows {
            session,
            request,
            query,
        });
    }

    fn tree_key(&mut self, tab: ConnTabId, key: TreeKey) {
        let show_system = self.settings.show_system_schemas;
        let Some(workspace) = self.workspace(tab) else {
            return;
        };
        let rows = workspace.tree.visible_rows(workspace.driver, show_system);
        if rows.is_empty() {
            return;
        }
        let at = workspace
            .tree
            .cursor
            .as_ref()
            .and_then(|cursor| rows.iter().position(|row| &row.node == cursor));
        let last = rows.len() - 1;
        let target = match (key, at) {
            // No cursor, or it is on a row no longer shown: End and Up start
            // at the bottom, anything else at the top.
            (TreeKey::End | TreeKey::Up, None) => Some(last),
            (_, None) => Some(0),
            (TreeKey::Up, Some(at)) => Some(at.saturating_sub(1)),
            (TreeKey::Down, Some(at)) => Some((at + 1).min(last)),
            (TreeKey::Home, _) => Some(0),
            (TreeKey::End, _) => Some(last),
            (TreeKey::Right, Some(at)) => match rows[at].expanded {
                Some(false) => {
                    self.toggle_tree_row(tab, &rows[at].node);
                    Some(at)
                }
                Some(true) => Some((at + 1).min(last)),
                None => Some(at),
            },
            (TreeKey::Left, Some(at)) => match rows[at].expanded {
                Some(true) => {
                    self.toggle_tree_row(tab, &rows[at].node);
                    Some(at)
                }
                // Up to the parent: the nearest row above that is shallower.
                _ => Some(
                    (0..at)
                        .rev()
                        .find(|&i| rows[i].depth < rows[at].depth)
                        .unwrap_or(at),
                ),
            },
            (TreeKey::Enter, Some(at)) => {
                match &rows[at].node {
                    TreeNode::Object(object, kind) => self.apply(Action::OpenObject {
                        tab,
                        object: object.clone(),
                        kind: *kind,
                        pin: true,
                    }),
                    node => self.toggle_tree_row(tab, node),
                }
                None
            }
        };
        if let Some(index) = target
            && let Some(workspace) = self.workspace_mut(tab)
        {
            workspace.tree.cursor = Some(rows[index].node.clone());
            workspace.tree.reveal_cursor = true;
            workspace.pane = Pane::Tree;
        }
    }

    /// Folds or unfolds a schema or group row.
    fn toggle_tree_row(&mut self, tab: ConnTabId, node: &TreeNode) {
        match node {
            TreeNode::Group(schema, prefix) => self.apply(Action::ToggleGroup {
                tab,
                schema: schema.clone(),
                prefix: prefix.clone(),
            }),
            TreeNode::Object(..) | TreeNode::Empty(_) => {}
        }
    }

    /// Loaded objects of `tab`'s connection that match `query`, best first,
    /// at most 50.
    pub fn quick_open_matches(&self, tab: ConnTabId, query: &str) -> Vec<(ObjectRef, ObjectKind)> {
        let Some(workspace) = self.workspace(tab) else {
            return Vec::new();
        };
        let mut scored: Vec<(i64, ObjectRef, ObjectKind)> = workspace
            .tree
            .nodes
            .iter()
            .filter_map(|(schema, node)| Some((schema, node.objects.value.as_ref()?)))
            .flat_map(|(schema, objects)| objects.iter().map(move |info| (schema, info)))
            .filter_map(|(schema, info)| {
                let qualified = format!("{schema}.{}", info.name);
                let score = crate::util::fuzzy_score(query, &info.name)
                    .max(crate::util::fuzzy_score(query, &qualified))?;
                Some((score, ObjectRef::new(schema, &info.name), info.kind))
            })
            .collect();
        scored.sort_by(|a, b| {
            b.0.cmp(&a.0)
                .then_with(|| a.1.schema.cmp(&b.1.schema))
                .then_with(|| a.1.name.cmp(&b.1.name))
        });
        scored.truncate(50);
        scored
            .into_iter()
            .map(|(_, object, kind)| (object, kind))
            .collect()
    }

    /// Runs the object tab's query with its filter bar's conditions, from
    /// the first page.
    fn apply_filters(&mut self, tab: ConnTabId, id: ObjectTabId) {
        let Some(object) = self.object_tab_mut(tab, id) else {
            return;
        };
        let (filters, raw_where) = object.filter.to_query();
        // Keep the columns for the bar: the page is about to go.
        if let Some(page) = object.page() {
            object.filter.columns = page.columns.iter().map(|c| c.name.clone()).collect();
        }
        // Never show rows for a query we no longer display.
        object.rows.value = None;
        object.query.filters = filters;
        object.query.raw_where = raw_where;
        object.query.offset = 0;
        object.pinned = true;
        object.selection = None;
        self.reset_count(tab, id);
        self.fetch_rows(tab, id);
    }

    pub fn count_rows(&mut self, tab: ConnTabId, id: ObjectTabId) {
        let request = RequestId(self.next_id());
        let Some(workspace) = self.workspace_mut(tab) else {
            return;
        };
        let session = workspace.session;
        let Some(object) = workspace.object_tab_mut(id) else {
            return;
        };
        let superseded = object.count.pending;
        object.count.start(request);
        let query = object.query.clone();
        self.cancel(session, superseded);
        self.backend.send(Command::CountRows {
            session,
            request,
            query,
        });
    }

    pub fn describe(&mut self, tab: ConnTabId, id: ObjectTabId) {
        let request = RequestId(self.next_id());
        let Some(workspace) = self.workspace_mut(tab) else {
            return;
        };
        let session = workspace.session;
        let Some(object) = workspace.object_tab_mut(id) else {
            return;
        };
        let superseded = object.structure.pending;
        object.structure.start(request);
        let target = object.object.clone();
        self.cancel(session, superseded);
        self.backend.send(Command::Describe {
            session,
            request,
            object: target,
        });
    }

    /// Text for the clipboard: the selected cell, or its whole row as TSV.
    pub fn copy_text(&self, whole_row: bool) -> Option<String> {
        let (tab, id) = self.active_object()?;
        let object = self.workspace(tab)?.object_tab(id)?;
        let cell = object.selection?;
        let row = object.page()?.rows.get(cell.row)?;
        Some(if whole_row {
            crate::ui::format::tsv_row(row)
        } else {
            crate::ui::format::plain_text(row.get(cell.col)?)
        })
    }

    /// Called once the window exists. `follow_desktop` is false in demo mode
    /// and tests, which must not scan the user's themes or Omarchy.
    pub fn attach(&mut self, ctx: &egui::Context, follow_desktop: bool) {
        theme::install(ctx, follow_desktop, &self.look);
        if follow_desktop {
            theme::enable_desktop_themes(&mut self.themes);
            let repaint = ctx.clone();
            self.themes.start(
                self.dirs.themes_dir(),
                self.settings.custom_theme.clone(),
                &fastframe_theme::Waker::new(move || repaint.request_repaint()),
            );
        }
        self.system_theme = ctx.system_theme();
        self.palette = self.resolve_palette();
        theme::apply(ctx, &self.palette, &self.look);
    }

    fn resolve_palette(&self) -> Palette {
        theme::resolve(
            &self.themes,
            self.settings.custom_theme.as_deref(),
            self.system_theme,
        )
    }

    /// Work that does not draw: theme changes on disk or in the OS.
    pub fn logic(&mut self, ctx: &egui::Context) {
        if self.themes.needs_reload() {
            let repaint = ctx.clone();
            self.themes.start(
                self.dirs.themes_dir(),
                self.settings.custom_theme.clone(),
                &fastframe_theme::Waker::new(move || repaint.request_repaint()),
            );
        }
        let scanned = self.themes.poll();
        let system = ctx.system_theme();
        if scanned || system != self.system_theme {
            self.system_theme = system;
            let palette = self.resolve_palette();
            if palette != self.palette {
                self.palette = palette;
                theme::apply(ctx, &palette, &self.look);
            }
        }
    }

    /// Draws one frame, then applies what the frame asked for.
    pub fn frame_ui(&mut self, ui: &mut egui::Ui) {
        self.poll_backend();
        self.apply_actions();
        // A dialog takes the keyboard: no shortcut acts behind it.
        if self.dialog.is_none() {
            crate::ui::keys::handle(self, ui.ctx());
        }
        crate::ui::show(self, ui);
        self.apply_actions();
        let title = self.window_title();
        if title != self.window_title {
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.window_title = title;
        }
    }
}

/// Fills the form from its URL field.
fn apply_url(form: &mut ConnectionForm) {
    let tabletist_db::ParsedUrl {
        spec,
        secrets,
        names_tls,
        names_ca_file,
        without_password,
    } = match tabletist_db::ParsedUrl::parse(&form.url) {
        Ok(parsed) => parsed,
        Err(error) => {
            form.message = Some(error.to_string());
            return;
        }
    };
    // The password moves to the masked field; the URL field is not masked.
    form.url = without_password;
    match spec.driver {
        Driver::Sqlite => {
            let path = spec
                .sqlite_path
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_default();
            if form.name.trim().is_empty() {
                form.name = crate::model::file_name(&path);
            }
            form.driver = Driver::Sqlite;
            form.sqlite_path = path;
            form.message = None;
        }
        Driver::Postgres | Driver::MySql => {
            if form.name.trim().is_empty() {
                form.name = spec.summary();
            }
            form.driver = spec.driver;
            form.host = spec.host;
            form.port = spec.port.to_string();
            form.user = spec.user;
            form.database = spec.database;
            // A URL that does not name the TLS settings keeps the form's,
            // rather than quietly dropping to `prefer`.
            if names_tls {
                form.tls = spec.tls;
            }
            if names_ca_file {
                form.ca_file = spec
                    .ca_file
                    .map(|path| path.display().to_string())
                    .unwrap_or_default();
            }
            if let Some(password) = secrets.password {
                form.password = password;
            }
            if form.password_mode == PasswordMode::None {
                form.password_mode = PasswordMode::Keyring;
            }
            form.message = None;
        }
    }
}

/// Moves `index` by `delta` within `0..len` (len > 0), saturating.
fn step(index: usize, delta: isize, len: usize) -> usize {
    let moved = (index as i128 + delta as i128).clamp(0, len as i128 - 1);
    moved as usize
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Action, ConnTabContent};

    fn app() -> (App, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let app = App::new(
            AppDirs::at(dir.path()),
            Settings::default(),
            Backend::recording(),
        );
        (app, dir)
    }

    #[test]
    fn an_older_connections_file_is_saved_upgraded_by_the_backend() {
        let dir = tempfile::tempdir().unwrap();
        let path = AppDirs::at(dir.path()).connections_file();
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, include_str!("../tests/fixtures/connections-v1.json")).unwrap();
        let app = App::new(
            AppDirs::at(dir.path()),
            Settings::default(),
            Backend::recording(),
        );
        let saves = |app: &App| {
            app.backend
                .sent
                .iter()
                .filter(|command| matches!(command, Command::Save { path: to, .. } if *to == path))
                .count()
        };
        assert_eq!(saves(&app), 1);
        assert!(
            std::fs::read_to_string(&path)
                .unwrap()
                .contains("\"color\""),
            "the UI thread writes nothing"
        );
        // A current file is not saved again.
        app.connections.save(&path).unwrap();
        let app = App::new(
            AppDirs::at(dir.path()),
            Settings::default(),
            Backend::recording(),
        );
        assert_eq!(saves(&app), 0);
    }

    fn ids(app: &App) -> Vec<u64> {
        app.tabs.iter().map(|tab| tab.id.0).collect()
    }

    #[test]
    fn a_new_app_has_one_picker_tab() {
        let (app, _dir) = app();
        assert_eq!(app.tabs.len(), 1);
        assert_eq!(app.active, 0);
        assert!(matches!(app.tabs[0].content, ConnTabContent::Picker(_)));
    }

    #[test]
    fn new_tabs_open_at_the_end_and_become_active() {
        let (mut app, _dir) = app();
        app.apply(Action::NewConnTab);
        app.apply(Action::NewConnTab);
        assert_eq!(app.tabs.len(), 3);
        assert_eq!(app.active, 2);
        let unique: std::collections::HashSet<_> = ids(&app).into_iter().collect();
        assert_eq!(unique.len(), 3, "tab ids must be unique");
    }

    #[test]
    fn closing_the_last_tab_leaves_a_fresh_picker() {
        let (mut app, _dir) = app();
        let only = app.tabs[0].id;
        app.apply(Action::CloseConnTab(only));
        assert_eq!(app.tabs.len(), 1);
        assert_ne!(app.tabs[0].id, only);
        assert_eq!(app.active, 0);
    }

    #[test]
    fn closing_a_tab_before_the_active_one_keeps_the_active_tab() {
        let (mut app, _dir) = app();
        app.apply(Action::NewConnTab);
        app.apply(Action::NewConnTab);
        let active = app.active_tab_id();
        let first = app.tabs[0].id;
        app.apply(Action::CloseConnTab(first));
        assert_eq!(app.active_tab_id(), active);
    }

    #[test]
    fn closing_the_active_tab_activates_its_right_neighbour_or_the_new_last() {
        let (mut app, _dir) = app();
        app.apply(Action::NewConnTab);
        app.apply(Action::NewConnTab);
        let [a, b, c] = [app.tabs[0].id, app.tabs[1].id, app.tabs[2].id];
        app.apply(Action::ActivateConnTab(b));
        app.apply(Action::CloseConnTab(b));
        assert_eq!(app.active_tab_id(), c);
        app.apply(Action::CloseConnTab(c));
        assert_eq!(app.active_tab_id(), a);
    }

    #[test]
    fn closing_an_unknown_tab_changes_nothing() {
        let (mut app, _dir) = app();
        let before = ids(&app);
        app.apply(Action::CloseConnTab(crate::model::ConnTabId(9999)));
        assert_eq!(ids(&app), before);
    }

    #[test]
    fn tabs_activate_by_index_and_cycle_with_wrapping() {
        let (mut app, _dir) = app();
        app.apply(Action::NewConnTab);
        app.apply(Action::NewConnTab);
        app.apply(Action::ActivateConnTabIndex(0));
        assert_eq!(app.active, 0);
        app.apply(Action::ActivateConnTabIndex(7));
        assert_eq!(app.active, 0, "an index past the end is ignored");
        app.apply(Action::CycleConnTab(-1));
        assert_eq!(app.active, 2);
        app.apply(Action::CycleConnTab(1));
        assert_eq!(app.active, 0);
    }

    #[test]
    fn queued_actions_are_drained_in_order() {
        let (mut app, _dir) = app();
        app.actions.push(Action::NewConnTab);
        app.actions.push(Action::ActivateConnTabIndex(0));
        app.apply_actions();
        assert!(app.actions.is_empty());
        assert_eq!(app.tabs.len(), 2);
        assert_eq!(app.active, 0);
    }

    use crate::backend::{Command, Event, RequestId, SessionId};
    use crate::connections::{ConnectionId, SavedConnection};
    use tabletist_db::{ConnectSpec, Driver, Error};

    fn with_saved(app: &mut App) -> ConnectionId {
        let saved = SavedConnection {
            id: ConnectionId::new(),
            name: "Local".into(),
            environment: crate::env::Environment::Dev,
            read_only: None,
            password: crate::connections::PasswordMode::None,
            ssh_secret: crate::connections::PasswordMode::None,
            spec: ConnectSpec::sqlite("/tmp/local.db"),
        };
        let id = saved.id.clone();
        app.connections.upsert(saved);
        id
    }

    /// Connects the active tab and returns (tab, session, request).
    fn connect(app: &mut App) -> (ConnTabId, SessionId, RequestId) {
        let conn = with_saved(app);
        let tab = app.active_tab_id();
        app.apply(Action::Connect { tab, conn });
        match app.backend.sent.last() {
            Some(Command::Connect {
                session, request, ..
            }) => (tab, *session, *request),
            other => panic!("expected a Connect command, got {other:?}"),
        }
    }

    #[test]
    fn connecting_turns_the_picker_into_a_connecting_workspace() {
        let (mut app, _dir) = app();
        let (tab, _, request) = connect(&mut app);
        let workspace = app.workspace(tab).unwrap();
        assert_eq!(workspace.name, "Local");
        assert!(
            matches!(workspace.status, SessionStatus::Connecting { request: r } if r == request)
        );
    }

    #[test]
    fn a_connected_event_marks_the_tab_connected() {
        let (mut app, _dir) = app();
        let (tab, session, request) = connect(&mut app);
        app.apply(Action::Backend(Event::Connected {
            session,
            request,
            driver: Driver::Sqlite,
            encrypted: false,
        }));
        assert!(matches!(
            app.workspace(tab).unwrap().status,
            SessionStatus::Connected
        ));
    }

    #[test]
    fn a_failed_connect_shows_the_error_in_the_tab() {
        let (mut app, _dir) = app();
        let (tab, session, request) = connect(&mut app);
        app.apply(Action::Backend(Event::ConnectFailed {
            session,
            request,
            error: Error::Connect("no such file".into()),
        }));
        assert!(matches!(
            &app.workspace(tab).unwrap().status,
            SessionStatus::Disconnected(Error::Connect(_))
        ));
    }

    #[test]
    fn a_connect_result_for_a_closed_tab_is_ignored() {
        let (mut app, _dir) = app();
        let (tab, session, request) = connect(&mut app);
        app.apply(Action::CloseConnTab(tab));
        assert!(
            matches!(app.backend.sent.last(), Some(Command::Close { session: s }) if *s == session)
        );
        app.apply(Action::Backend(Event::Connected {
            session,
            request,
            driver: Driver::Sqlite,
            encrypted: false,
        }));
        assert!(app.workspace(tab).is_none());
    }

    #[test]
    fn a_late_session_is_closed_not_adopted() {
        let (mut app, _dir) = app();
        let (tab, first_session, first_request) = connect(&mut app);
        app.apply(Action::Reconnect(tab));
        let before = app.backend.sent.len();
        app.apply(Action::Backend(Event::Connected {
            session: first_session,
            request: first_request,
            driver: Driver::Sqlite,
            encrypted: false,
        }));
        assert!(matches!(
            app.workspace(tab).unwrap().status,
            SessionStatus::Connecting { .. }
        ));
        assert!(matches!(
            app.backend.sent.get(before),
            Some(Command::Close { session }) if *session == first_session
        ));
    }

    #[test]
    fn a_disconnect_event_shows_the_banner_state() {
        let (mut app, _dir) = app();
        let (tab, session, request) = connect(&mut app);
        app.apply(Action::Backend(Event::Connected {
            session,
            request,
            driver: Driver::Sqlite,
            encrypted: false,
        }));
        app.apply(Action::Backend(Event::Disconnected {
            session,
            error: Error::ConnectionLost("gone".into()),
        }));
        assert!(matches!(
            app.workspace(tab).unwrap().status,
            SessionStatus::Disconnected(Error::ConnectionLost(_))
        ));
    }

    #[test]
    fn reconnect_opens_a_new_session_for_the_same_tab() {
        let (mut app, _dir) = app();
        let (tab, session, request) = connect(&mut app);
        app.apply(Action::Backend(Event::Connected {
            session,
            request,
            driver: Driver::Sqlite,
            encrypted: false,
        }));
        app.apply(Action::Reconnect(tab));
        let new_session = app.workspace(tab).unwrap().session;
        assert_ne!(new_session, session);
        assert!(
            app.backend
                .sent
                .iter()
                .any(|c| matches!(c, Command::Close { session: s } if *s == session))
        );
        assert!(matches!(
            app.backend.sent.last(),
            Some(Command::Connect { session: s, .. }) if *s == new_session
        ));
    }

    #[test]
    fn a_failed_reconnect_keeps_the_tab_with_its_error() {
        let (mut app, _dir) = app();
        let (tab, _, _) = connect(&mut app);
        app.apply(Action::Reconnect(tab));
        let (session, request) = match app.backend.sent.last() {
            Some(Command::Connect {
                session, request, ..
            }) => (*session, *request),
            other => panic!("{other:?}"),
        };
        app.apply(Action::Backend(Event::ConnectFailed {
            session,
            request,
            error: Error::Connect("file deleted".into()),
        }));
        assert!(matches!(
            app.workspace(tab).unwrap().status,
            SessionStatus::Disconnected(_)
        ));
    }

    #[test]
    fn disconnect_closes_the_session_and_returns_to_the_picker() {
        let (mut app, _dir) = app();
        let (tab, session, _) = connect(&mut app);
        app.apply(Action::Disconnect(tab));
        assert!(app.workspace(tab).is_none());
        assert!(matches!(
            app.tabs[app.active].content,
            ConnTabContent::Picker(_)
        ));
        assert!(
            matches!(app.backend.sent.last(), Some(Command::Close { session: s }) if *s == session)
        );
    }

    #[test]
    fn connecting_an_unknown_saved_connection_does_nothing() {
        let (mut app, _dir) = app();
        let tab = app.active_tab_id();
        app.apply(Action::Connect {
            tab,
            conn: ConnectionId("missing".into()),
        });
        assert!(app.workspace(tab).is_none());
        assert!(app.backend.sent.is_empty());
    }

    use crate::model::{ConnectionForm, Dialog, TestState};

    fn form(app: &mut App) -> &mut ConnectionForm {
        match app.dialog.as_mut() {
            Some(Dialog::Connection(form)) => form,
            _ => panic!("the connection dialog is not open"),
        }
    }

    #[test]
    fn saving_requires_a_name_and_a_file() {
        let (mut app, _dir) = app();
        app.apply(Action::NewConnection);
        app.apply(Action::SaveConnection { connect: false });
        assert!(form(&mut app).message.is_some());
        assert!(app.connections.connections.is_empty());
        form(&mut app).name = "Local".into();
        app.apply(Action::SaveConnection { connect: false });
        assert!(form(&mut app).message.as_deref().unwrap().contains("file"));
        form(&mut app).sqlite_path = "/tmp/local.db".into();
        app.apply(Action::SaveConnection { connect: false });
        assert!(app.dialog.is_none());
        assert_eq!(app.connections.connections.len(), 1);
    }

    /// The newest state file the app asked the backend to save at `path`.
    fn saved(app: &App, path: &std::path::Path) -> Option<StateFile> {
        app.backend
            .sent
            .iter()
            .rev()
            .find_map(|command| match command {
                Command::Save { path: to, file } if to == path => Some(file.clone()),
                _ => None,
            })
    }

    #[test]
    fn saved_connections_are_written_by_the_backend() {
        let (mut app, dir) = app();
        app.apply(Action::NewConnection);
        form(&mut app).name = "Disk".into();
        form(&mut app).sqlite_path = "/tmp/disk.db".into();
        app.apply(Action::SaveConnection { connect: false });
        let path = AppDirs::at(dir.path()).connections_file();
        assert!(!path.exists(), "the UI thread writes nothing");
        match saved(&app, &path) {
            Some(StateFile::Connections(stored)) => {
                assert_eq!(stored.connections[0].name, "Disk")
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_failed_save_shows_a_notice() {
        let (mut app, _dir) = app();
        app.apply(Action::Backend(Event::Saved {
            path: "/config/connections.json".into(),
            result: Err("No space left on device".into()),
        }));
        let notice = app.notice.clone().expect("a notice");
        assert!(notice.contains("connections.json"), "{notice}");
        assert!(notice.contains("No space left"), "{notice}");
    }

    #[test]
    fn save_and_connect_opens_it_in_the_active_picker_tab() {
        let (mut app, _dir) = app();
        let tab = app.active_tab_id();
        app.apply(Action::NewConnection);
        form(&mut app).name = "Now".into();
        form(&mut app).sqlite_path = "/tmp/now.db".into();
        app.apply(Action::SaveConnection { connect: true });
        assert_eq!(app.workspace(tab).unwrap().name, "Now");
    }

    #[test]
    fn save_and_connect_from_a_workspace_opens_a_new_tab() {
        let (mut app, _dir) = app();
        let (first, _, _) = connect(&mut app);
        app.apply(Action::NewConnection);
        form(&mut app).name = "Second".into();
        form(&mut app).sqlite_path = "/tmp/second.db".into();
        app.apply(Action::SaveConnection { connect: true });
        assert_eq!(app.tabs.len(), 2);
        assert_eq!(app.workspace(first).unwrap().name, "Local");
        assert_eq!(app.workspace(app.active_tab_id()).unwrap().name, "Second");
    }

    #[test]
    fn editing_keeps_the_id_and_updates_the_entry() {
        let (mut app, _dir) = app();
        let id = with_saved(&mut app);
        app.apply(Action::EditConnection(id.clone()));
        assert_eq!(form(&mut app).name, "Local");
        form(&mut app).name = "Renamed".into();
        app.apply(Action::SaveConnection { connect: false });
        assert_eq!(app.connections.connections.len(), 1);
        assert_eq!(app.connections.get(&id).unwrap().name, "Renamed");
    }

    #[test]
    fn a_pasted_url_fills_the_form_or_explains_why_not() {
        let (mut app, _dir) = app();
        app.apply(Action::NewConnection);
        form(&mut app).url = "sqlite:///srv/app.db".into();
        app.apply(Action::ApplyUrl);
        assert_eq!(form(&mut app).sqlite_path, "/srv/app.db");
        assert_eq!(form(&mut app).name, "app.db");
        form(&mut app).url = "mysql://root@db.example.com/shop?ssl-mode=REQUIRED".into();
        app.apply(Action::ApplyUrl);
        assert_eq!(form(&mut app).driver, Driver::MySql);
        assert_eq!(form(&mut app).port, "3306");
        assert_eq!(form(&mut app).database, "shop");
        assert_eq!(form(&mut app).tls, tabletist_db::TlsMode::Require);
        assert!(form(&mut app).message.is_none());
    }

    #[test]
    fn test_results_update_only_the_dialog_that_asked() {
        let (mut app, _dir) = app();
        app.apply(Action::NewConnection);
        form(&mut app).name = "T".into();
        form(&mut app).sqlite_path = "/tmp/t.db".into();
        app.apply(Action::TestConnection);
        let request = match form(&mut app).test {
            TestState::Running(request) => request,
            ref other => panic!("{other:?}"),
        };
        app.apply(Action::Backend(Event::Tested {
            request: RequestId(request.0 + 1000),
            result: Ok(()),
        }));
        assert!(matches!(form(&mut app).test, TestState::Running(_)));
        app.apply(Action::Backend(Event::Tested {
            request,
            result: Err(Error::Connect("nope".into())),
        }));
        assert!(
            matches!(&form(&mut app).test, TestState::Failed(message) if message.contains("nope"))
        );
        app.apply(Action::CloseDialog);
        app.apply(Action::Backend(Event::Tested {
            request,
            result: Ok(()),
        }));
        assert!(app.dialog.is_none());
    }

    #[test]
    fn a_picked_file_fills_the_path_and_a_cancelled_pick_changes_nothing() {
        let (mut app, _dir) = app();
        app.apply(Action::NewConnection);
        form(&mut app).pick_request = Some(RequestId(77));
        app.apply(Action::Backend(Event::FilePicked {
            request: RequestId(77),
            path: None,
        }));
        assert_eq!(form(&mut app).sqlite_path, "");
        form(&mut app).pick_request = Some(RequestId(78));
        app.apply(Action::Backend(Event::FilePicked {
            request: RequestId(78),
            path: Some("/data/shop.sqlite".into()),
        }));
        assert_eq!(form(&mut app).sqlite_path, "/data/shop.sqlite");
        assert_eq!(form(&mut app).name, "shop.sqlite");
    }

    #[test]
    fn deleting_a_connection_leaves_open_tabs_alone() {
        let (mut app, dir) = app();
        let (tab, _, _) = connect(&mut app);
        let conn = app.workspace(tab).unwrap().conn_id.clone();
        app.apply(Action::DeleteConnection(conn.clone()));
        assert!(app.connections.get(&conn).is_none());
        assert_eq!(app.workspace(tab).unwrap().name, "Local");
        let stored =
            crate::connections::SavedConnections::load(&AppDirs::at(dir.path()).connections_file());
        assert!(stored.get(&conn).is_none());
    }

    #[test]
    fn duplicating_adds_a_copy_right_after() {
        let (mut app, _dir) = app();
        let id = with_saved(&mut app);
        app.apply(Action::DuplicateConnection(id));
        let names: Vec<&str> = app
            .connections
            .connections
            .iter()
            .map(|c| c.name.as_str())
            .collect();
        assert_eq!(names, ["Local", "Local copy"]);
    }

    use crate::model::{TreeNode, TreeRow};
    use crate::testing::Harness;

    #[test]
    fn the_window_title_names_the_connection_its_environment_and_database() {
        let mut harness = Harness::new();
        harness.settle();
        assert_eq!(harness.app.window_title(), "Tabletist");
        let tab = harness.connect_fake();
        let workspace = harness.app.workspace_mut(tab).unwrap();
        workspace.name = "Bookshop".into();
        workspace.environment = crate::env::Environment::Production;
        let title = "Bookshop · production — fixture.db";
        assert_eq!(harness.app.window_title(), title);
        let sent = |harness: &mut Harness| {
            harness.frame(Vec::new());
            harness
                .viewport_commands
                .contains(&egui::ViewportCommand::Title(title.into()))
        };
        // Sent when it changes, not every frame.
        assert!(sent(&mut harness));
        assert!(!sent(&mut harness));
        let workspace = harness.app.workspace_mut(tab).unwrap();
        workspace.spec = tabletist_db::ConnectSpec::from_url(
            "postgres://app@db.example.com:5432/bookshop_production",
        )
        .unwrap()
        .0;
        assert_eq!(
            harness.app.window_title(),
            "Bookshop · production — bookshop_production"
        );
        harness.set_look(crate::theme::Look::omarchy());
        assert_eq!(
            harness.app.window_title(),
            "Bookshop · PROD — bookshop_production"
        );
    }

    #[test]
    fn connecting_loads_schemas_then_expands_the_default_schema() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        let workspace = harness.app.workspace(tab).unwrap();
        assert_eq!(
            workspace.tree.schemas.value.as_deref(),
            Some(&["main".to_owned()][..])
        );
        assert!(workspace.tree.nodes["main"].expanded);
        let rows: Vec<TreeRow> = workspace.tree.visible_rows(Driver::Sqlite, false);
        assert!(
            rows.iter().any(
                |row| matches!(&row.node, TreeNode::Object(object, _) if object.name == "users")
            )
        );
    }

    #[test]
    fn toggling_a_schema_folds_it_and_unfolding_does_not_reload() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        let sent = harness.app.backend.sent.len();
        harness.app.apply(Action::ToggleSchema {
            tab,
            schema: "main".into(),
        });
        assert!(!harness.app.workspace(tab).unwrap().tree.nodes["main"].expanded);
        harness.app.apply(Action::ToggleSchema {
            tab,
            schema: "main".into(),
        });
        assert!(harness.app.workspace(tab).unwrap().tree.nodes["main"].expanded);
        assert_eq!(
            harness.app.backend.sent.len(),
            sent,
            "loaded objects are reused"
        );
    }

    #[test]
    fn toggling_a_group_unfolds_and_folds_it() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        let toggle = |harness: &mut Harness| {
            harness.app.apply(Action::ToggleGroup {
                tab,
                schema: "main".into(),
                prefix: "order".into(),
            });
        };
        let open = |harness: &Harness| {
            harness.app.workspace(tab).unwrap().tree.nodes["main"]
                .open_groups
                .contains("order")
        };
        toggle(&mut harness);
        assert!(open(&harness));
        toggle(&mut harness);
        assert!(!open(&harness));
    }

    #[test]
    fn refreshing_the_tree_reloads_schemas_and_expanded_objects() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        let before = harness.app.backend.sent.len();
        harness.app.apply(Action::RefreshTree(tab));
        let new: Vec<_> = harness.app.backend.sent[before..].iter().collect();
        assert!(new.iter().any(|c| matches!(c, Command::ListSchemas { .. })));
        assert!(
            new.iter()
                .any(|c| matches!(c, Command::ListObjects { schema, .. } if schema == "main"))
        );
    }

    #[test]
    fn a_stale_objects_result_is_ignored() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        harness.app.apply(Action::RefreshTree(tab));
        let session = harness.app.workspace(tab).unwrap().session;
        harness.app.apply(Action::Backend(Event::Objects {
            session,
            request: RequestId(1),
            schema: "main".into(),
            result: Ok(Vec::new()),
        }));
        let node = &harness.app.workspace(tab).unwrap().tree.nodes["main"];
        assert_eq!(
            node.objects.value.as_ref().unwrap().len(),
            3,
            "old value kept"
        );
    }

    use crate::model::{CellPos, ObjectTabId, ObjectView};
    use crate::testing::{last_sent, page};
    use tabletist_db::{ObjectRef, SortDir};

    #[test]
    fn reconnecting_reloads_the_tree_and_the_active_tab() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        harness.app.apply(Action::OpenObject {
            tab,
            object: tabletist_db::ObjectRef::new("main", "users"),
            kind: ObjectKind::Table,
            pin: true,
        });
        harness.app.apply(Action::Reconnect(tab));
        let Command::Connect {
            session, request, ..
        } = *last_sent(&harness.app)
        else {
            panic!("expected Connect");
        };
        let before = harness.app.backend.sent.len();
        harness.app.apply(Action::Backend(Event::Connected {
            session,
            request,
            driver: Driver::Sqlite,
            encrypted: false,
        }));
        let new: Vec<_> = harness.app.backend.sent[before..].iter().collect();
        assert!(
            new.iter()
                .any(|c| matches!(c, Command::ListSchemas { session: s, .. } if *s == session))
        );
        assert!(
            new.iter()
                .any(|c| matches!(c, Command::ListObjects { session: s, .. } if *s == session))
        );
        assert!(
            new.iter()
                .any(|c| matches!(c, Command::FetchRows { session: s, .. } if *s == session))
        );
    }

    #[test]
    fn reconnecting_restarts_inactive_tabs_that_were_loading() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        let background = open(&mut harness, tab, "orders", true);
        open(&mut harness, tab, "users", true);
        // `orders` is still waiting on the old session when it dies.
        harness.app.apply(Action::Reconnect(tab));
        let Command::Connect {
            session, request, ..
        } = *last_sent(&harness.app)
        else {
            panic!("expected Connect");
        };
        let before = harness.app.backend.sent.len();
        harness.app.apply(Action::Backend(Event::Connected {
            session,
            request,
            driver: Driver::Sqlite,
            encrypted: false,
        }));
        let refetched = harness.app.backend.sent[before..]
            .iter()
            .filter(|c| matches!(c, Command::FetchRows { .. }))
            .count();
        assert_eq!(
            refetched, 2,
            "both the active tab and the loading background tab refetch"
        );
        assert!(object(&harness, tab, background).rows.is_loading());
    }

    #[test]
    fn reconnecting_restarts_tabs_the_lost_connection_failed() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        let background = open(&mut harness, tab, "orders", true);
        open(&mut harness, tab, "users", true);
        let session = harness.app.workspace(tab).unwrap().session;
        let request = object(&harness, tab, background).rows.pending.unwrap();
        let lost = Error::ConnectionLost("reset".into());
        harness.app.apply(Action::Backend(Event::Rows {
            session,
            request,
            result: Err(lost.clone()),
        }));
        harness.app.apply(Action::Backend(Event::Disconnected {
            session,
            error: lost,
        }));
        harness.app.apply(Action::Reconnect(tab));
        let Command::Connect {
            session, request, ..
        } = *last_sent(&harness.app)
        else {
            panic!("expected Connect");
        };
        harness.app.apply(Action::Backend(Event::Connected {
            session,
            request,
            driver: Driver::Sqlite,
            encrypted: false,
        }));
        assert!(object(&harness, tab, background).rows.is_loading());
    }

    fn users() -> ObjectRef {
        ObjectRef::new("main", "users")
    }

    fn open(harness: &mut Harness, tab: ConnTabId, name: &str, pin: bool) -> ObjectTabId {
        harness.app.apply(Action::OpenObject {
            tab,
            object: ObjectRef::new("main", name),
            kind: ObjectKind::Table,
            pin,
        });
        harness.app.workspace(tab).unwrap().active_object.unwrap()
    }

    fn object(harness: &Harness, tab: ConnTabId, id: ObjectTabId) -> &crate::model::ObjectTab {
        harness.app.workspace(tab).unwrap().object_tab(id).unwrap()
    }

    fn last_query(harness: &Harness) -> tabletist_db::RowQuery {
        match last_sent(&harness.app) {
            Command::FetchRows { query, .. } => query.clone(),
            other => panic!("expected FetchRows, got {other:?}"),
        }
    }

    #[test]
    fn opening_an_object_fetches_its_first_page_with_the_estimate() {
        let mut harness = Harness::new();
        let tab = connect_tab(&mut harness);
        let id = open(&mut harness, tab, "users", false);
        let query = last_query(&harness);
        assert_eq!(query.object, users());
        assert_eq!(query.offset, 0);
        assert_eq!(query.limit, harness.app.settings.page_size);
        assert_eq!(object(&harness, tab, id).estimated_rows, Some(1_200_000));
        harness.answer_rows(page(5, false));
        assert_eq!(object(&harness, tab, id).page().unwrap().rows.len(), 5);
    }

    fn connect_tab(harness: &mut Harness) -> ConnTabId {
        harness.connect_fake()
    }

    #[test]
    fn a_single_click_replaces_the_preview_and_a_double_click_pins() {
        let mut harness = Harness::new();
        let tab = connect_tab(&mut harness);
        open(&mut harness, tab, "users", false);
        open(&mut harness, tab, "orders", false);
        let workspace = harness.app.workspace(tab).unwrap();
        assert_eq!(workspace.objects.len(), 1, "the preview was replaced");
        assert_eq!(workspace.objects[0].object.name, "orders");
        let orders = open(&mut harness, tab, "orders", true);
        assert!(object(&harness, tab, orders).pinned);
        open(&mut harness, tab, "users", false);
        assert_eq!(
            harness.app.workspace(tab).unwrap().objects.len(),
            2,
            "a pinned tab is never replaced"
        );
    }

    #[test]
    fn opening_an_open_object_activates_it_without_refetching() {
        let mut harness = Harness::new();
        let tab = connect_tab(&mut harness);
        let first = open(&mut harness, tab, "users", true);
        open(&mut harness, tab, "orders", true);
        let sent = harness.app.backend.sent.len();
        assert_eq!(open(&mut harness, tab, "users", false), first);
        assert_eq!(harness.app.backend.sent.len(), sent);
    }

    #[test]
    fn closing_the_active_object_tab_activates_its_neighbour() {
        let mut harness = Harness::new();
        let tab = connect_tab(&mut harness);
        let a = open(&mut harness, tab, "users", true);
        let b = open(&mut harness, tab, "orders", true);
        harness
            .app
            .apply(Action::CloseObjectTab { tab, object_tab: b });
        assert_eq!(harness.app.workspace(tab).unwrap().active_object, Some(a));
        harness
            .app
            .apply(Action::CloseObjectTab { tab, object_tab: a });
        assert_eq!(harness.app.workspace(tab).unwrap().active_object, None);
    }

    #[test]
    fn object_tabs_cycle_with_wrapping() {
        let mut harness = Harness::new();
        let tab = connect_tab(&mut harness);
        let a = open(&mut harness, tab, "users", true);
        let b = open(&mut harness, tab, "orders", true);
        harness.app.apply(Action::CycleObjectTab { tab, step: 1 });
        assert_eq!(harness.app.workspace(tab).unwrap().active_object, Some(a));
        harness.app.apply(Action::CycleObjectTab { tab, step: -1 });
        assert_eq!(harness.app.workspace(tab).unwrap().active_object, Some(b));
    }

    #[test]
    fn next_and_previous_pages_move_the_offset_and_pin() {
        let mut harness = Harness::new();
        let tab = connect_tab(&mut harness);
        let id = open(&mut harness, tab, "users", false);
        harness.answer_rows(page(300, true));
        harness.app.apply(Action::NextPage {
            tab,
            object_tab: id,
        });
        assert_eq!(last_query(&harness).offset, 300);
        assert!(object(&harness, tab, id).pinned);
        harness.answer_rows(page(10, false));
        let sent = harness.app.backend.sent.len();
        harness.app.apply(Action::NextPage {
            tab,
            object_tab: id,
        });
        assert_eq!(
            harness.app.backend.sent.len(),
            sent,
            "no next page after the last one"
        );
        harness.app.apply(Action::PrevPage {
            tab,
            object_tab: id,
        });
        assert_eq!(last_query(&harness).offset, 0);
        let sent = harness.app.backend.sent.len();
        harness.answer_rows(page(300, true));
        harness.app.apply(Action::PrevPage {
            tab,
            object_tab: id,
        });
        assert_eq!(
            harness.app.backend.sent.len(),
            sent,
            "no previous page before the first"
        );
    }

    #[test]
    fn sorting_cycles_ascending_descending_off_and_restarts_paging() {
        let mut harness = Harness::new();
        let tab = connect_tab(&mut harness);
        let id = open(&mut harness, tab, "users", true);
        harness.answer_rows(page(300, true));
        harness.app.apply(Action::NextPage {
            tab,
            object_tab: id,
        });
        let sort = |harness: &mut Harness| {
            harness.app.apply(Action::SortBy {
                tab,
                object_tab: id,
                column: "email".into(),
            });
            last_query(harness)
        };
        let query = sort(&mut harness);
        assert_eq!(query.offset, 0);
        assert_eq!(query.sort[0].dir, SortDir::Asc);
        assert_eq!(sort(&mut harness).sort[0].dir, SortDir::Desc);
        assert!(sort(&mut harness).sort.is_empty());
    }

    #[test]
    fn a_superseded_page_result_is_dropped() {
        let mut harness = Harness::new();
        let tab = connect_tab(&mut harness);
        let id = open(&mut harness, tab, "users", true);
        let (session, first) = match last_sent(&harness.app) {
            Command::FetchRows {
                session, request, ..
            } => (*session, *request),
            other => panic!("{other:?}"),
        };
        harness.app.apply(Action::SortBy {
            tab,
            object_tab: id,
            column: "email".into(),
        });
        harness.app.apply(Action::Backend(Event::Rows {
            session,
            request: first,
            result: Ok(page(300, true)),
        }));
        assert!(object(&harness, tab, id).page().is_none());
        // And a page already shown is cleared the moment the query changes.
        harness.answer_rows(page(300, true));
        harness.app.apply(Action::NextPage {
            tab,
            object_tab: id,
        });
        assert!(object(&harness, tab, id).page().is_none());
        assert!(object(&harness, tab, id).rows.is_loading());
    }

    #[test]
    fn a_result_for_a_closed_object_tab_is_ignored() {
        let mut harness = Harness::new();
        let tab = connect_tab(&mut harness);
        let id = open(&mut harness, tab, "users", true);
        harness.app.apply(Action::CloseObjectTab {
            tab,
            object_tab: id,
        });
        harness.answer_rows(page(5, false));
        assert!(harness.app.workspace(tab).unwrap().objects.is_empty());
    }

    #[test]
    fn selecting_a_cell_pins_and_selection_is_clamped_to_a_shorter_page() {
        let mut harness = Harness::new();
        let tab = connect_tab(&mut harness);
        let id = open(&mut harness, tab, "users", false);
        harness.answer_rows(page(300, true));
        harness.app.apply(Action::SelectCell {
            tab,
            object_tab: id,
            cell: CellPos { row: 200, col: 2 },
        });
        assert!(object(&harness, tab, id).pinned);
        harness.app.apply(Action::Refresh(tab));
        harness.answer_rows(page(5, false));
        assert_eq!(
            object(&harness, tab, id).selection,
            Some(CellPos { row: 4, col: 2 })
        );
        harness.app.apply(Action::Refresh(tab));
        harness.answer_rows(page(0, false));
        assert_eq!(object(&harness, tab, id).selection, None);
    }

    #[test]
    fn selection_is_cleared_when_paging() {
        let mut harness = Harness::new();
        let tab = connect_tab(&mut harness);
        let id = open(&mut harness, tab, "users", true);
        harness.answer_rows(page(300, true));
        harness.app.apply(Action::SelectCell {
            tab,
            object_tab: id,
            cell: CellPos { row: 3, col: 0 },
        });
        harness.app.apply(Action::NextPage {
            tab,
            object_tab: id,
        });
        assert_eq!(object(&harness, tab, id).selection, None);
    }

    #[test]
    fn moving_the_selection_stays_in_range() {
        let mut harness = Harness::new();
        let tab = connect_tab(&mut harness);
        let id = open(&mut harness, tab, "users", true);
        let mv = |harness: &mut Harness, rows: isize, cols: isize| {
            harness.app.apply(Action::MoveSelection {
                tab,
                object_tab: id,
                rows,
                cols,
            });
            object(harness, tab, id).selection
        };
        assert_eq!(mv(&mut harness, 1, 0), None, "no page, nothing to select");
        harness.answer_rows(page(0, false));
        assert_eq!(
            mv(&mut harness, isize::MAX, 0),
            None,
            "an empty page stays empty"
        );
        harness.app.apply(Action::Refresh(tab));
        harness.answer_rows(page(5, false));
        assert_eq!(
            mv(&mut harness, 1, 0),
            Some(CellPos { row: 0, col: 0 }),
            "the first move selects the first cell"
        );
        assert_eq!(mv(&mut harness, -1, -1), Some(CellPos { row: 0, col: 0 }));
        assert_eq!(
            mv(&mut harness, isize::MAX, isize::MAX),
            Some(CellPos { row: 4, col: 2 })
        );
        assert_eq!(
            mv(&mut harness, isize::MIN, 0),
            Some(CellPos { row: 0, col: 2 })
        );
    }

    #[test]
    fn opening_describes_once_for_the_keys() {
        let mut harness = Harness::new();
        let tab = connect_tab(&mut harness);
        let id = open(&mut harness, tab, "users", true);
        // The grid and the row panel name the keys, so opening describes.
        let describes = |harness: &Harness| {
            harness
                .app
                .backend
                .sent
                .iter()
                .filter(|command| matches!(command, Command::Describe { object, .. } if *object == users()))
                .count()
        };
        assert_eq!(describes(&harness), 1);
        harness.app.apply(Action::SetView {
            tab,
            object_tab: id,
            view: ObjectView::Structure,
        });
        assert_eq!(
            describes(&harness),
            1,
            "the pending describe serves the view"
        );
        let sent = harness.app.backend.sent.len();
        harness.app.apply(Action::SetView {
            tab,
            object_tab: id,
            view: ObjectView::Data,
        });
        harness.app.apply(Action::SetView {
            tab,
            object_tab: id,
            view: ObjectView::Structure,
        });
        assert_eq!(
            harness.app.backend.sent.len(),
            sent,
            "a pending describe is not repeated"
        );
    }

    #[test]
    fn cancel_names_the_request_it_stops_and_refresh_refetches() {
        let mut harness = Harness::new();
        let tab = connect_tab(&mut harness);
        let id = open(&mut harness, tab, "users", true);
        let rows = object(&harness, tab, id).rows.pending.unwrap();
        let structure = object(&harness, tab, id).structure.pending.unwrap();
        let sent = harness.app.backend.sent.len();
        harness.app.apply(Action::CancelQuery(tab));
        let session = harness.app.workspace(tab).unwrap().session;
        assert!(
            harness.app.backend.sent[sent..].iter().all(
                |command| matches!(command, Command::Cancel { session: s, .. } if *s == session)
            )
        );
        assert_eq!(cancels_since(&harness, sent), vec![rows, structure]);
        harness.app.apply(Action::Refresh(tab));
        assert!(matches!(last_sent(&harness.app), Command::FetchRows { .. }));
    }

    /// The Cancel commands sent since `from`, by request.
    fn cancels_since(harness: &Harness, from: usize) -> Vec<RequestId> {
        harness.app.backend.sent[from..]
            .iter()
            .filter_map(|command| match command {
                Command::Cancel { request, .. } => Some(*request),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_new_load_stops_the_one_it_replaces() {
        let mut harness = Harness::new();
        let tab = connect_tab(&mut harness);
        let id = open(&mut harness, tab, "users", true);
        let first = object(&harness, tab, id).rows.pending.unwrap();
        let sent = harness.app.backend.sent.len();
        // A held Mod+R repeats: each refresh replaces the last.
        harness.app.apply(Action::Refresh(tab));
        let second = object(&harness, tab, id).rows.pending.unwrap();
        harness.app.apply(Action::Refresh(tab));
        assert_eq!(cancels_since(&harness, sent), vec![first, second]);
        // The cancel goes out before the load that replaces it.
        let tail = &harness.app.backend.sent[harness.app.backend.sent.len() - 2..];
        assert!(matches!(
            tail,
            [Command::Cancel { request, .. }, Command::FetchRows { .. }] if *request == second
        ));
    }

    #[test]
    fn closing_an_object_tab_stops_its_count() {
        let mut harness = Harness::new();
        let tab = connect_tab(&mut harness);
        let id = open(&mut harness, tab, "users", true);
        harness.answer_rows(crate::testing::page(3, true));
        harness.app.apply(Action::CountRows {
            tab,
            object_tab: id,
        });
        let counting = object(&harness, tab, id).count.pending.unwrap();
        let sent = harness.app.backend.sent.len();
        harness.app.apply(Action::CloseObjectTab {
            tab,
            object_tab: id,
        });
        // With the describe that opening it started.
        assert!(cancels_since(&harness, sent).contains(&counting));
    }

    #[test]
    fn a_new_filter_stops_the_count_of_the_old_one() {
        let mut harness = Harness::new();
        let tab = connect_tab(&mut harness);
        let id = open(&mut harness, tab, "users", true);
        harness.answer_rows(crate::testing::page(3, true));
        harness.app.apply(Action::CountRows {
            tab,
            object_tab: id,
        });
        let counting = object(&harness, tab, id).count.pending.unwrap();
        let sent = harness.app.backend.sent.len();
        harness.app.apply(Action::Refresh(tab));
        assert!(cancels_since(&harness, sent).contains(&counting));
        assert!(!object(&harness, tab, id).count.is_loading());
    }

    #[test]
    fn the_row_panel_toggles() {
        let mut harness = Harness::new();
        let tab = connect_tab(&mut harness);
        assert!(harness.app.workspace(tab).unwrap().row_panel);
        harness.app.apply(Action::ToggleRowPanel(tab));
        assert!(!harness.app.workspace(tab).unwrap().row_panel);
    }

    #[test]
    fn copy_text_gives_the_selected_cell_or_row() {
        let mut harness = Harness::new();
        let tab = connect_tab(&mut harness);
        let id = open(&mut harness, tab, "users", true);
        assert_eq!(harness.app.copy_text(false), None);
        harness.answer_rows(page(2, false));
        harness.app.apply(Action::SelectCell {
            tab,
            object_tab: id,
            cell: CellPos { row: 0, col: 1 },
        });
        assert_eq!(
            harness.app.copy_text(false).as_deref(),
            Some("user1@example.com")
        );
        assert_eq!(
            harness.app.copy_text(true).as_deref(),
            Some("1\tuser1@example.com\t{\"plan\":\"pro\"}")
        );
    }

    #[test]
    fn saving_an_edit_of_a_deleted_connection_does_not_restore_it() {
        let (mut app, _dir) = app();
        let id = with_saved(&mut app);
        app.apply(Action::EditConnection(id.clone()));
        app.apply(Action::DeleteConnection(id.clone()));
        app.apply(Action::SaveConnection { connect: false });
        assert!(app.connections.get(&id).is_none());
        assert!(form(&mut app).message.is_some());
    }

    use crate::connections::PasswordMode;
    use crate::model::PasswordPrompt;
    use crate::secrets::{SecretString, password_account};

    fn postgres_saved(app: &mut App, mode: PasswordMode) -> ConnectionId {
        let (spec, _) = ConnectSpec::from_url("postgres://me@db.example.com/app").unwrap();
        let saved = SavedConnection {
            id: ConnectionId::new(),
            name: "Prod".into(),
            environment: crate::env::Environment::Production,
            read_only: None,
            password: mode,
            ssh_secret: crate::connections::PasswordMode::None,
            spec,
        };
        let id = saved.id.clone();
        app.connections.upsert(saved);
        id
    }

    fn sent_secrets(app: &App) -> Vec<&Command> {
        app.backend
            .sent
            .iter()
            .filter(|c| matches!(c, Command::LoadSecret { .. } | Command::StoreSecret { .. }))
            .collect()
    }

    fn prompt(app: &mut App) -> &mut PasswordPrompt {
        match app.dialog.as_mut() {
            Some(Dialog::Password(prompt)) => prompt,
            other => panic!("expected the password prompt, got {other:?}"),
        }
    }

    #[test]
    fn a_keyring_connection_loads_its_password_then_connects() {
        let (mut app, _dir) = app();
        let conn = postgres_saved(&mut app, PasswordMode::Keyring);
        let tab = app.active_tab_id();
        app.apply(Action::Connect {
            tab,
            conn: conn.clone(),
        });
        let request = match app.backend.sent.last() {
            Some(Command::LoadSecret { request, account }) => {
                assert_eq!(account, &password_account(&conn));
                *request
            }
            other => panic!("{other:?}"),
        };
        app.apply(Action::Backend(Event::SecretLoaded {
            request,
            result: Ok(Some(SecretString("pw".into()))),
        }));
        match app.backend.sent.last() {
            Some(Command::Connect { secrets, .. }) => {
                assert_eq!(secrets.password.as_deref(), Some("pw"))
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_missing_keyring_password_opens_the_prompt() {
        let (mut app, _dir) = app();
        let conn = postgres_saved(&mut app, PasswordMode::Keyring);
        let tab = app.active_tab_id();
        app.apply(Action::Connect { tab, conn });
        let Some(Command::LoadSecret { request, .. }) = app.backend.sent.last() else {
            panic!()
        };
        let request = *request;
        app.apply(Action::Backend(Event::SecretLoaded {
            request,
            result: Ok(None),
        }));
        assert!(
            prompt(&mut app).save,
            "a keyring connection offers to save again"
        );
        assert!(prompt(&mut app).message.is_some());
    }

    #[test]
    fn a_keyring_error_opens_the_prompt_with_its_message() {
        let (mut app, _dir) = app();
        let conn = postgres_saved(&mut app, PasswordMode::Keyring);
        let tab = app.active_tab_id();
        app.apply(Action::Connect { tab, conn });
        let Some(Command::LoadSecret { request, .. }) = app.backend.sent.last() else {
            panic!()
        };
        let request = *request;
        app.apply(Action::Backend(Event::SecretLoaded {
            request,
            result: Err("the keyring is locked".into()),
        }));
        assert!(
            prompt(&mut app)
                .message
                .as_deref()
                .unwrap()
                .contains("locked")
        );
    }

    #[test]
    fn an_ask_connection_prompts_and_submitting_connects() {
        let (mut app, _dir) = app();
        let conn = postgres_saved(&mut app, PasswordMode::Ask);
        let tab = app.active_tab_id();
        app.apply(Action::Connect { tab, conn });
        assert!(
            sent_secrets(&app).is_empty(),
            "no keyring for an Ask connection"
        );
        prompt(&mut app).password = "typed".into();
        app.apply(Action::SubmitPassword);
        assert!(app.dialog.is_none());
        match app.backend.sent.last() {
            Some(Command::Connect { secrets, .. }) => {
                assert_eq!(secrets.password.as_deref(), Some("typed"))
            }
            other => panic!("{other:?}"),
        }
        assert!(sent_secrets(&app).is_empty(), "not saved unless asked");
    }

    #[test]
    fn saving_from_the_prompt_stores_the_password_and_switches_to_keyring() {
        let (mut app, _dir) = app();
        let conn = postgres_saved(&mut app, PasswordMode::Ask);
        let tab = app.active_tab_id();
        app.apply(Action::Connect {
            tab,
            conn: conn.clone(),
        });
        prompt(&mut app).password = "typed".into();
        prompt(&mut app).save = true;
        app.apply(Action::SubmitPassword);
        // Saved only once the server accepts it.
        assert!(sent_secrets(&app).is_empty());
        let Some(Command::Connect {
            session, request, ..
        }) = app.backend.sent.last()
        else {
            panic!()
        };
        let (session, request) = (*session, *request);
        app.apply(Action::Backend(Event::Connected {
            session,
            request,
            driver: Driver::Postgres,
            encrypted: false,
        }));
        assert!(app.backend.sent.iter().any(|c| matches!(
            c,
            Command::StoreSecret { secret: Some(SecretString(p)), .. } if p == "typed"
        )));
        // Keyring mode only once the keyring has it.
        assert_eq!(
            app.connections.get(&conn).unwrap().password,
            PasswordMode::Ask
        );
        answer_last_store(&mut app, Ok(()));
        assert_eq!(
            app.connections.get(&conn).unwrap().password,
            PasswordMode::Keyring
        );
        assert_eq!(
            app.workspace(tab).unwrap().password_mode,
            PasswordMode::Keyring
        );
        assert!(app.notice.is_none());
    }

    fn answer_last_store(app: &mut App, result: Result<(), String>) {
        let request = app
            .backend
            .sent
            .iter()
            .rev()
            .find_map(|c| match c {
                Command::StoreSecret { request, .. } => Some(*request),
                _ => None,
            })
            .expect("a StoreSecret");
        app.apply(Action::Backend(Event::SecretStored { request, result }));
    }

    fn connect_saving_prompted_password(app: &mut App, conn: &ConnectionId) -> ConnTabId {
        let tab = app.active_tab_id();
        app.apply(Action::Connect {
            tab,
            conn: conn.clone(),
        });
        prompt(app).password = "typed".into();
        prompt(app).save = true;
        app.apply(Action::SubmitPassword);
        let Some(Command::Connect {
            session, request, ..
        }) = app.backend.sent.last()
        else {
            panic!()
        };
        let (session, request) = (*session, *request);
        app.apply(Action::Backend(Event::Connected {
            session,
            request,
            driver: Driver::Postgres,
            encrypted: false,
        }));
        tab
    }

    #[test]
    fn a_failed_keyring_write_is_shown_and_keeps_asking() {
        let (mut app, _dir) = app();
        let conn = postgres_saved(&mut app, PasswordMode::Ask);
        let tab = connect_saving_prompted_password(&mut app, &conn);
        answer_last_store(&mut app, Err("the keyring is locked".into()));
        assert_eq!(
            app.connections.get(&conn).unwrap().password,
            PasswordMode::Ask
        );
        assert_eq!(app.workspace(tab).unwrap().password_mode, PasswordMode::Ask);
        let notice = app.notice.clone().expect("a notice");
        assert!(notice.contains("the keyring is locked"), "{notice}");
        app.apply(Action::DismissNotice);
        assert!(app.notice.is_none());
    }

    #[test]
    fn a_connection_deleted_meanwhile_gets_no_keyring_entry() {
        let (mut app, _dir) = app();
        let conn = postgres_saved(&mut app, PasswordMode::Ask);
        let tab = app.active_tab_id();
        app.apply(Action::Connect {
            tab,
            conn: conn.clone(),
        });
        prompt(&mut app).password = "typed".into();
        prompt(&mut app).save = true;
        app.apply(Action::SubmitPassword);
        app.apply(Action::DeleteConnection(conn));
        // Deleting also sent a save; the connect is the one before it.
        let (session, request) = app
            .backend
            .sent
            .iter()
            .rev()
            .find_map(|command| match command {
                Command::Connect {
                    session, request, ..
                } => Some((*session, *request)),
                _ => None,
            })
            .expect("a connect");
        app.apply(Action::Backend(Event::Connected {
            session,
            request,
            driver: Driver::Postgres,
            encrypted: false,
        }));
        assert!(sent_secrets(&app).is_empty());
        assert!(app.connections.connections.is_empty());
    }

    #[test]
    fn a_failed_write_from_the_dialog_stops_reading_the_old_entry() {
        let (mut app, _dir) = app();
        let conn = postgres_saved(&mut app, PasswordMode::Keyring);
        app.apply(Action::EditConnection(conn.clone()));
        form(&mut app).password = "new".into();
        app.apply(Action::SaveConnection { connect: false });
        answer_last_store(&mut app, Err("the keyring is not available".into()));
        assert!(app.notice.is_some());
        // The keyring may still hold the old password: ask instead.
        assert_eq!(
            app.connections.get(&conn).unwrap().password,
            PasswordMode::Ask
        );
    }

    fn last_connect(app: &App) -> (SessionId, RequestId, Option<String>) {
        match app
            .backend
            .sent
            .iter()
            .rev()
            .find(|c| matches!(c, Command::Connect { .. }))
        {
            Some(Command::Connect {
                session,
                request,
                secrets,
                ..
            }) => (*session, *request, secrets.password.clone()),
            _ => panic!("no Connect was sent"),
        }
    }

    fn rejected() -> Error {
        Error::Auth("password authentication failed for user \"me\"".into())
    }

    #[test]
    fn a_rejected_password_prompts_again_and_is_not_resent() {
        let (mut app, _dir) = app();
        let conn = postgres_saved(&mut app, PasswordMode::Ask);
        let tab = app.active_tab_id();
        app.apply(Action::Connect { tab, conn });
        prompt(&mut app).password = "wrong".into();
        prompt(&mut app).save = true;
        app.apply(Action::SubmitPassword);
        let (session, request, _) = last_connect(&app);
        app.apply(Action::Backend(Event::ConnectFailed {
            session,
            request,
            error: rejected(),
        }));
        assert!(
            prompt(&mut app)
                .message
                .as_deref()
                .unwrap()
                .contains("password authentication failed")
        );
        assert!(app.workspace(tab).unwrap().secrets.password.is_none());
        assert!(
            sent_secrets(&app).is_empty(),
            "a rejected password is never saved"
        );
    }

    #[test]
    fn a_rejected_keyring_password_prompts_instead_of_reusing_it() {
        let (mut app, _dir) = app();
        let conn = postgres_saved(&mut app, PasswordMode::Keyring);
        let tab = app.active_tab_id();
        app.apply(Action::Connect { tab, conn });
        let Some(Command::LoadSecret { request, .. }) = app.backend.sent.last() else {
            panic!()
        };
        let request = *request;
        app.apply(Action::Backend(Event::SecretLoaded {
            request,
            result: Ok(Some(SecretString("stale".into()))),
        }));
        let (session, request, _) = last_connect(&app);
        app.apply(Action::Backend(Event::ConnectFailed {
            session,
            request,
            error: rejected(),
        }));
        let loads = |app: &App| {
            app.backend
                .sent
                .iter()
                .filter(|c| matches!(c, Command::LoadSecret { .. }))
                .count()
        };
        assert_eq!(
            loads(&app),
            1,
            "the stale keyring password is not read again"
        );
        assert!(
            prompt(&mut app).save,
            "the new password will replace the stale one"
        );
        prompt(&mut app).password = "new".into();
        app.apply(Action::SubmitPassword);
        let (session, request, password) = last_connect(&app);
        assert_eq!(password.as_deref(), Some("new"));
        app.apply(Action::Backend(Event::Connected {
            session,
            request,
            driver: Driver::Postgres,
            encrypted: false,
        }));
        assert!(app.backend.sent.iter().any(|c| matches!(
            c,
            Command::StoreSecret { secret: Some(SecretString(p)), .. } if p == "new"
        )));
    }

    #[test]
    fn a_prompt_never_replaces_an_open_dialog() {
        let (mut app, _dir) = app();
        let conn = postgres_saved(&mut app, PasswordMode::Ask);
        let tab = app.active_tab_id();
        app.apply(Action::NewConnection);
        form(&mut app).name = "half typed".into();
        app.apply(Action::Connect { tab, conn });
        assert_eq!(form(&mut app).name, "half typed");
        assert!(matches!(
            app.workspace(tab).unwrap().status,
            SessionStatus::Disconnected(Error::Auth(_))
        ));
        app.apply(Action::CloseDialog);
        app.apply(Action::Reconnect(tab));
        assert!(
            matches!(app.dialog, Some(Dialog::Password(_))),
            "Reconnect asks for the password"
        );
    }

    #[test]
    fn forms_and_prompts_do_not_print_passwords() {
        let (mut app, _dir) = app();
        app.apply(Action::NewConnection);
        form(&mut app).password = "hunter2".into();
        assert!(!format!("{:?}", app.dialog).contains("hunter2"));
        let conn = postgres_saved(&mut app, PasswordMode::Ask);
        app.apply(Action::CloseDialog);
        let tab = app.active_tab_id();
        app.apply(Action::Connect { tab, conn });
        prompt(&mut app).password = "hunter3".into();
        assert!(!format!("{:?}", app.dialog).contains("hunter3"));
    }

    #[test]
    fn cancelling_the_prompt_leaves_a_cancelled_tab() {
        let (mut app, _dir) = app();
        let conn = postgres_saved(&mut app, PasswordMode::Ask);
        let tab = app.active_tab_id();
        app.apply(Action::Connect { tab, conn });
        app.apply(Action::CancelPassword);
        assert!(matches!(
            app.workspace(tab).unwrap().status,
            SessionStatus::Cancelled
        ));
        app.apply(Action::Reconnect(tab));
        assert!(
            matches!(app.dialog, Some(Dialog::Password(_))),
            "Reconnect asks again"
        );
    }

    #[test]
    fn reconnecting_reuses_the_password_of_this_tab() {
        let (mut app, _dir) = app();
        let conn = postgres_saved(&mut app, PasswordMode::Ask);
        let tab = app.active_tab_id();
        app.apply(Action::Connect { tab, conn });
        prompt(&mut app).password = "typed".into();
        app.apply(Action::SubmitPassword);
        app.apply(Action::Reconnect(tab));
        assert!(app.dialog.is_none());
        match app.backend.sent.last() {
            Some(Command::Connect { secrets, .. }) => {
                assert_eq!(secrets.password.as_deref(), Some("typed"))
            }
            other => panic!("{other:?}"),
        }
    }

    fn postgres_form(app: &mut App) {
        app.apply(Action::NewConnection);
        app.apply(Action::SetDriver(Driver::Postgres));
        let form = form(app);
        form.name = "Prod".into();
        form.host = "db.example.com".into();
        form.user = "me".into();
        form.database = "app".into();
    }

    #[test]
    fn choosing_postgres_fills_the_default_port_and_keyring_mode() {
        let (mut app, _dir) = app();
        app.apply(Action::NewConnection);
        app.apply(Action::SetDriver(Driver::Postgres));
        assert_eq!(form(&mut app).port, "5432");
        assert_eq!(form(&mut app).password_mode, PasswordMode::Keyring);
    }

    #[test]
    fn a_postgres_form_needs_host_user_and_a_port_number() {
        let (mut app, _dir) = app();
        postgres_form(&mut app);
        form(&mut app).port = "fifty".into();
        app.apply(Action::SaveConnection { connect: false });
        assert!(form(&mut app).message.as_deref().unwrap().contains("port"));
        form(&mut app).port = "5433".into();
        form(&mut app).user.clear();
        app.apply(Action::SaveConnection { connect: false });
        assert!(form(&mut app).message.as_deref().unwrap().contains("user"));
    }

    #[test]
    fn saving_with_a_typed_password_stores_it_and_connect_uses_it() {
        let (mut app, _dir) = app();
        postgres_form(&mut app);
        form(&mut app).password = "pw".into();
        app.apply(Action::SaveConnection { connect: true });
        let saved = &app.connections.connections[0];
        assert_eq!(saved.password, PasswordMode::Keyring);
        assert_eq!(saved.spec.port, 5432);
        assert!(app.backend.sent.iter().any(|c| matches!(
            c,
            Command::StoreSecret { secret: Some(SecretString(p)), .. } if p == "pw"
        )));
        match app.backend.sent.last() {
            Some(Command::Connect { secrets, .. }) => {
                assert_eq!(secrets.password.as_deref(), Some("pw"))
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn saving_without_retyping_keeps_the_keyring_password() {
        let (mut app, _dir) = app();
        let conn = postgres_saved(&mut app, PasswordMode::Keyring);
        app.apply(Action::EditConnection(conn));
        assert!(form(&mut app).has_saved_password);
        form(&mut app).name = "Renamed".into();
        app.apply(Action::SaveConnection { connect: false });
        assert!(
            sent_secrets(&app).is_empty(),
            "the saved password is left alone"
        );
    }

    #[test]
    fn changing_to_ask_deletes_the_saved_password() {
        let (mut app, _dir) = app();
        let conn = postgres_saved(&mut app, PasswordMode::Keyring);
        app.apply(Action::EditConnection(conn.clone()));
        form(&mut app).password_mode = PasswordMode::Ask;
        app.apply(Action::SaveConnection { connect: false });
        assert!(app.backend.sent.iter().any(|c| matches!(
            c,
            Command::StoreSecret { secret: None, account, .. } if *account == password_account(&conn)
        )));
    }

    #[test]
    fn deleting_a_connection_deletes_its_password() {
        let (mut app, _dir) = app();
        let conn = postgres_saved(&mut app, PasswordMode::Keyring);
        app.apply(Action::DeleteConnection(conn));
        assert!(matches!(
            sent_secrets(&app).as_slice(),
            [Command::StoreSecret { secret: None, .. }]
        ));
    }

    #[test]
    fn sqlite_saves_never_touch_the_keyring() {
        let (mut app, _dir) = app();
        let id = with_saved(&mut app);
        app.apply(Action::EditConnection(id.clone()));
        app.apply(Action::SaveConnection { connect: true });
        app.apply(Action::DeleteConnection(id));
        assert!(sent_secrets(&app).is_empty());
    }

    #[test]
    fn a_pasted_postgres_url_fills_every_field() {
        let (mut app, _dir) = app();
        app.apply(Action::NewConnection);
        form(&mut app).url = "postgres://me:pw@db.example.com:6543/app?sslmode=verify-full".into();
        app.apply(Action::ApplyUrl);
        let form = form(&mut app);
        assert_eq!(form.driver, Driver::Postgres);
        assert_eq!(
            (form.host.as_str(), form.port.as_str()),
            ("db.example.com", "6543")
        );
        assert_eq!((form.user.as_str(), form.database.as_str()), ("me", "app"));
        assert_eq!(form.password, "pw");
        assert_eq!(form.tls, tabletist_db::TlsMode::VerifyFull);
        assert_eq!(form.name, "me@db.example.com:6543/app");
    }

    #[test]
    fn a_pasted_url_keeps_unnamed_tls_settings_and_hides_its_password() {
        let (mut app, _dir) = app();
        postgres_form(&mut app);
        form(&mut app).tls = tabletist_db::TlsMode::VerifyFull;
        form(&mut app).ca_file = "/etc/ca.pem".into();
        form(&mut app).url = "postgres://me:secret@other.example.com/app".into();
        app.apply(Action::ApplyUrl);
        let filled = form(&mut app);
        assert_eq!(filled.host, "other.example.com");
        assert_eq!(filled.tls, tabletist_db::TlsMode::VerifyFull);
        assert_eq!(filled.ca_file, "/etc/ca.pem");
        assert_eq!(filled.password, "secret");
        assert_eq!(filled.url, "postgres://me@other.example.com/app");

        // A URL that names them wins; one that names them twice is refused.
        form(&mut app).url = "postgres://me@h/app?sslmode=require".into();
        app.apply(Action::ApplyUrl);
        assert_eq!(form(&mut app).tls, tabletist_db::TlsMode::Require);
        form(&mut app).url = "postgres://me@h2/app?sslmode=verify-full&sslmode=disable".into();
        app.apply(Action::ApplyUrl);
        assert!(form(&mut app).message.is_some());
        assert_eq!(form(&mut app).host, "h");
        assert_eq!(form(&mut app).tls, tabletist_db::TlsMode::Require);
    }

    #[test]
    fn testing_an_unchanged_keyring_connection_loads_its_password() {
        let (mut app, _dir) = app();
        let conn = postgres_saved(&mut app, PasswordMode::Keyring);
        app.apply(Action::EditConnection(conn));
        app.apply(Action::TestConnection);
        let Some(Command::LoadSecret { request, .. }) = app.backend.sent.last() else {
            panic!()
        };
        let request = *request;
        app.apply(Action::Backend(Event::SecretLoaded {
            request,
            result: Ok(Some(SecretString("pw".into()))),
        }));
        let (sent_request, password) = match app.backend.sent.last() {
            Some(Command::Test {
                request: r,
                secrets,
                ..
            }) => (*r, secrets.password.clone()),
            other => panic!("{other:?}"),
        };
        // The test runs under the id the dialog waits on.
        assert_eq!(form(&mut app).test, TestState::Running(sent_request));
        assert_eq!(password.as_deref(), Some("pw"));
    }

    #[test]
    fn a_saved_password_never_goes_to_a_changed_host() {
        let (mut app, _dir) = app();
        let conn = postgres_saved(&mut app, PasswordMode::Keyring);
        app.apply(Action::EditConnection(conn.clone()));
        assert!(form(&mut app).password_is_saved());
        form(&mut app).host = "elsewhere.example.com".into();
        assert!(!form(&mut app).password_is_saved());

        app.apply(Action::TestConnection);
        assert!(sent_secrets(&app).is_empty(), "Test must not load it");
        assert!(matches!(form(&mut app).test, TestState::Failed(_)));
        assert!(
            !app.backend
                .sent
                .iter()
                .any(|c| matches!(c, Command::Test { .. }))
        );

        app.apply(Action::SaveConnection { connect: true });
        assert_eq!(
            app.connections.get(&conn).unwrap().password,
            PasswordMode::Keyring,
            "the user's choice to keep a password stays"
        );
        assert!(
            matches!(
                sent_secrets(&app).as_slice(),
                [Command::StoreSecret { secret: None, account, .. }]
                    if *account == password_account(&conn)
            ),
            "the old password is deleted and never loaded"
        );
        assert!(matches!(app.dialog, Some(Dialog::Password(_))));
        assert!(
            !app.backend
                .sent
                .iter()
                .any(|c| matches!(c, Command::Connect { .. }))
        );
    }

    #[test]
    fn a_saved_password_follows_only_its_driver_port_and_user() {
        let (mut app, _dir) = app();
        let conn = postgres_saved(&mut app, PasswordMode::Keyring);
        app.apply(Action::EditConnection(conn));
        for change in [
            (|form: &mut ConnectionForm| form.port = "6543".into()) as fn(&mut ConnectionForm),
            |form| form.user = "admin".into(),
            |form| form.driver = Driver::MySql,
        ] {
            let form = form(&mut app);
            let (driver, port, user) = (form.driver, form.port.clone(), form.user.clone());
            change(form);
            assert!(form.password_is_stale());
            (form.driver, form.port, form.user) = (driver, port, user);
            assert!(form.password_is_saved());
        }
        // Whitespace and other fields do not matter.
        form(&mut app).host = " db.example.com ".into();
        form(&mut app).database = "other".into();
        assert!(form(&mut app).password_is_saved());
    }

    #[test]
    fn a_host_edited_while_a_test_loads_its_password_is_not_used() {
        let (mut app, _dir) = app();
        let conn = postgres_saved(&mut app, PasswordMode::Keyring);
        app.apply(Action::EditConnection(conn));
        app.apply(Action::TestConnection);
        let Some(Command::LoadSecret { request, .. }) = app.backend.sent.last() else {
            panic!()
        };
        let request = *request;
        form(&mut app).host = "elsewhere.example.com".into();
        app.apply(Action::Backend(Event::SecretLoaded {
            request,
            result: Ok(Some(SecretString("pw".into()))),
        }));
        match app.backend.sent.last() {
            Some(Command::Test { spec, .. }) => assert_eq!(spec.host, "db.example.com"),
            other => panic!("{other:?}"),
        }
    }

    fn connected_postgres(app: &mut App) -> ConnTabId {
        let conn = postgres_saved(app, PasswordMode::None);
        let tab = app.active_tab_id();
        app.apply(Action::Connect { tab, conn });
        let Some(Command::Connect {
            session, request, ..
        }) = app.backend.sent.last()
        else {
            panic!()
        };
        let (session, request) = (*session, *request);
        app.apply(Action::Backend(Event::Connected {
            session,
            request,
            driver: Driver::Postgres,
            encrypted: false,
        }));
        tab
    }

    #[test]
    fn postgres_tabs_load_their_database_list() {
        let (mut app, _dir) = app();
        let tab = connected_postgres(&mut app);
        let (session, request) = app
            .backend
            .sent
            .iter()
            .find_map(|c| match c {
                Command::ListDatabases { session, request } => Some((*session, *request)),
                _ => None,
            })
            .expect("ListDatabases");
        app.apply(Action::Backend(Event::Databases {
            session,
            request,
            result: Ok(vec!["app".into(), "other".into()]),
        }));
        assert_eq!(
            app.workspace(tab)
                .unwrap()
                .databases
                .value
                .as_ref()
                .unwrap()
                .len(),
            2
        );
    }

    #[test]
    fn switching_database_reconnects_with_a_fresh_tree() {
        let (mut app, _dir) = app();
        let tab = connected_postgres(&mut app);
        app.apply(Action::SwitchDatabase {
            tab,
            database: "other".into(),
        });
        let workspace = app.workspace(tab).unwrap();
        assert_eq!(workspace.spec.database, "other");
        assert!(workspace.tree.schemas.value.is_none() && workspace.objects.is_empty());
        match app.backend.sent.last() {
            Some(Command::Connect { spec, .. }) => assert_eq!(spec.database, "other"),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_failed_database_switch_can_switch_back() {
        let (mut app, _dir) = app();
        let tab = connected_postgres(&mut app);
        app.apply(Action::SwitchDatabase {
            tab,
            database: "forbidden".into(),
        });
        let Some(Command::Connect {
            session, request, ..
        }) = app.backend.sent.last()
        else {
            panic!()
        };
        let (session, request) = (*session, *request);
        app.apply(Action::Backend(Event::ConnectFailed {
            session,
            request,
            error: Error::Connect("permission denied for database".into()),
        }));
        assert!(matches!(
            app.workspace(tab).unwrap().status,
            SessionStatus::Disconnected(_)
        ));
        app.apply(Action::SwitchDatabase {
            tab,
            database: "app".into(),
        });
        match app.backend.sent.last() {
            Some(Command::Connect { spec, .. }) => assert_eq!(spec.database, "app"),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_mysql_form_saves_with_the_default_port() {
        let (mut app, _dir) = app();
        app.apply(Action::NewConnection);
        app.apply(Action::SetDriver(Driver::MySql));
        let form = form(&mut app);
        assert_eq!(form.port, "3306");
        form.name = "Shop".into();
        form.host = "db.example.com".into();
        form.user = "root".into();
        app.apply(Action::SaveConnection { connect: false });
        let saved = &app.connections.connections[0];
        assert_eq!(saved.spec.driver, Driver::MySql);
        assert_eq!(saved.spec.port, 3306);
        // No password typed and none saved.
        assert_eq!(saved.password, PasswordMode::None);
    }

    #[test]
    fn the_connections_database_is_expanded_first() {
        let (mut app, _dir) = app();
        let (spec, _) = ConnectSpec::from_url("mysql://root@db/shop").unwrap();
        let saved = SavedConnection {
            id: ConnectionId::new(),
            name: "Shop".into(),
            environment: crate::env::Environment::None,
            read_only: None,
            password: PasswordMode::None,
            ssh_secret: crate::connections::PasswordMode::None,
            spec,
        };
        let conn = saved.id.clone();
        app.connections.upsert(saved);
        let tab = app.active_tab_id();
        app.apply(Action::Connect { tab, conn });
        let Some(Command::Connect {
            session, request, ..
        }) = app.backend.sent.last()
        else {
            panic!()
        };
        let (session, request) = (*session, *request);
        app.apply(Action::Backend(Event::Connected {
            session,
            request,
            driver: Driver::MySql,
            encrypted: false,
        }));
        let Some(Command::ListSchemas { request, .. }) = app
            .backend
            .sent
            .iter()
            .rev()
            .find(|c| matches!(c, Command::ListSchemas { .. }))
        else {
            panic!("ListSchemas")
        };
        let request = *request;
        app.apply(Action::Backend(Event::Schemas {
            session,
            request,
            result: Ok(vec![
                "analytics".into(),
                "information_schema".into(),
                "shop".into(),
            ]),
        }));
        let nodes = &app.workspace(tab).unwrap().tree.nodes;
        assert!(nodes["shop"].expanded);
        assert!(!nodes.get("analytics").is_some_and(|node| node.expanded));
    }

    #[test]
    fn switching_driver_replaces_a_default_port_but_keeps_a_custom_one() {
        let (mut app, _dir) = app();
        app.apply(Action::NewConnection);
        app.apply(Action::SetDriver(Driver::Postgres));
        assert_eq!(form(&mut app).port, "5432");
        app.apply(Action::SetDriver(Driver::MySql));
        assert_eq!(form(&mut app).port, "3306");
        form(&mut app).port = "6543".into();
        app.apply(Action::SetDriver(Driver::Postgres));
        assert_eq!(form(&mut app).port, "6543");
    }

    #[test]
    fn a_blank_password_with_nothing_saved_connects_without_one() {
        let (mut app, _dir) = app();
        postgres_form(&mut app);
        assert_eq!(form(&mut app).password_mode, PasswordMode::Keyring);
        app.apply(Action::SaveConnection { connect: true });
        let saved = app.connections.connections.last().unwrap();
        assert_eq!(saved.password, PasswordMode::None);
        assert!(sent_secrets(&app).is_empty(), "nothing to load or store");
        assert!(app.dialog.is_none(), "no password prompt");
        match app.backend.sent.last() {
            Some(Command::Connect { secrets, .. }) => assert_eq!(secrets.password, None),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_blank_answer_in_the_prompt_connects_without_a_password() {
        let (mut app, _dir) = app();
        let conn = postgres_saved(&mut app, PasswordMode::Ask);
        let tab = app.active_tab_id();
        app.apply(Action::Connect { tab, conn });
        app.apply(Action::SubmitPassword);
        assert!(app.dialog.is_none());
        match app.backend.sent.last() {
            Some(Command::Connect { secrets, .. }) => assert_eq!(secrets.password, None),
            other => panic!("{other:?}"),
        }
    }

    /// SSH connections: host keys and secrets.
    mod ssh {
        use super::*;
        use crate::model::{HostKeyPrompt, SecretKind, SessionStatus, SshAuthKind};
        use tabletist_db::{HostKeys, SshStage};

        pub(super) fn ssh_saved(app: &mut App) -> ConnectionId {
            let (mut spec, _) = ConnectSpec::from_url("postgres://me@db/app").unwrap();
            spec.ssh = Some(tabletist_db::SshSpec {
                host: "bastion".into(),
                port: 22,
                user: "ops".into(),
                auth: tabletist_db::SshAuth::Agent,
            });
            let saved = SavedConnection {
                id: ConnectionId::new(),
                name: "Prod".into(),
                environment: crate::env::Environment::Production,
                read_only: None,
                password: PasswordMode::None,
                ssh_secret: PasswordMode::None,
                spec,
            };
            let id = saved.id.clone();
            app.connections.upsert(saved);
            id
        }

        pub(super) fn fail_last_connect(app: &mut App, error: Error) {
            let Some(Command::Connect {
                session, request, ..
            }) = app.backend.sent.last()
            else {
                panic!("expected a Connect, got {:?}", app.backend.sent.last());
            };
            let (session, request) = (*session, *request);
            app.apply(Action::Backend(Event::ConnectFailed {
                session,
                request,
                error,
            }));
        }

        fn unknown_key() -> Error {
            unknown_key_with("SHA256:abc")
        }

        fn unknown_key_with(fingerprint: &str) -> Error {
            Error::Ssh {
                stage: SshStage::HostKeyUnknown {
                    fingerprint: fingerprint.into(),
                },
                message: "bastion is not a trusted host yet".into(),
            }
        }

        #[test]
        fn a_saved_ssh_secret_never_goes_to_a_changed_ssh_host() {
            let (mut app, _dir) = app();
            let conn = ssh_saved(&mut app);
            let mut saved = app.connections.get(&conn).unwrap().clone();
            if let Some(ssh) = &mut saved.spec.ssh {
                ssh.auth = tabletist_db::SshAuth::Password;
            }
            saved.ssh_secret = PasswordMode::Keyring;
            app.connections.upsert(saved);
            app.apply(Action::EditConnection(conn.clone()));
            assert!(form(&mut app).ssh_secret_is_saved());
            for change in [
                (|form: &mut ConnectionForm| form.ssh_host = "other".into())
                    as fn(&mut ConnectionForm),
                |form| form.ssh_port = "2222".into(),
                |form| form.ssh_user = "root".into(),
            ] {
                let form = form(&mut app);
                let (host, port, user) = (
                    form.ssh_host.clone(),
                    form.ssh_port.clone(),
                    form.ssh_user.clone(),
                );
                change(form);
                assert!(!form.ssh_secret_is_saved());
                assert!(form.ssh_secret_is_stale());
                (form.ssh_host, form.ssh_port, form.ssh_user) = (host, port, user);
            }
            form(&mut app).ssh_host = "other".into();
            app.apply(Action::TestConnection);
            assert!(sent_secrets(&app).is_empty(), "Test must not load it");
            assert!(matches!(form(&mut app).test, TestState::Failed(_)));

            app.apply(Action::SaveConnection { connect: true });
            assert!(matches!(
                sent_secrets(&app).as_slice(),
                [Command::StoreSecret { secret: None, account, .. }]
                    if *account == ssh_account(&conn)
            ));
            match &app.dialog {
                Some(Dialog::Password(prompt)) => assert_eq!(prompt.kind, SecretKind::SshPassword),
                other => panic!("{other:?}"),
            }
        }

        #[test]
        fn an_unknown_host_key_asks_and_trusting_saves_it_and_reconnects() {
            let (mut app, dir) = app();
            let conn = ssh_saved(&mut app);
            let tab = app.active_tab_id();
            app.apply(Action::Connect { tab, conn });
            fail_last_connect(&mut app, unknown_key());
            match &app.dialog {
                Some(Dialog::HostKey(prompt)) => {
                    let HostKeyPrompt {
                        host,
                        port,
                        fingerprint,
                        ..
                    } = prompt.as_ref();
                    assert_eq!((host.as_str(), *port), ("bastion", 22));
                    assert_eq!(fingerprint, "SHA256:abc");
                }
                other => panic!("{other:?}"),
            }
            app.apply(Action::TrustHostKey);
            assert!(app.dialog.is_none());
            assert_eq!(app.host_keys.fingerprint("bastion", 22), Some("SHA256:abc"));
            match saved(&app, &AppDirs::at(dir.path()).known_hosts_file()) {
                Some(StateFile::KnownHosts(keys)) => {
                    assert_eq!(keys.fingerprint("bastion", 22), Some("SHA256:abc"))
                }
                other => panic!("{other:?}"),
            }
            match app.backend.sent.last() {
                Some(Command::Connect { host_keys, .. }) => {
                    assert_eq!(host_keys.fingerprint("bastion", 22), Some("SHA256:abc"))
                }
                other => panic!("{other:?}"),
            }
        }

        #[test]
        fn cancelling_the_host_key_prompt_trusts_nothing() {
            let (mut app, _dir) = app();
            let conn = ssh_saved(&mut app);
            let tab = app.active_tab_id();
            app.apply(Action::Connect { tab, conn });
            fail_last_connect(&mut app, unknown_key());
            let sent = app.backend.sent.len();
            app.apply(Action::CancelHostKey);
            assert!(app.dialog.is_none());
            assert_eq!(app.host_keys, HostKeys::default());
            assert_eq!(app.backend.sent.len(), sent, "no reconnect");
            assert!(matches!(
                app.workspace(tab).unwrap().status,
                SessionStatus::Disconnected(_)
            ));
        }

        #[test]
        fn a_changed_host_key_shows_the_error_and_no_prompt() {
            let (mut app, _dir) = app();
            let conn = ssh_saved(&mut app);
            let tab = app.active_tab_id();
            app.apply(Action::Connect { tab, conn });
            fail_last_connect(
                &mut app,
                Error::Ssh {
                    stage: SshStage::HostKeyMismatch {
                        fingerprint: "SHA256:new".into(),
                    },
                    message: "the host key changed".into(),
                },
            );
            assert!(app.dialog.is_none());
            match &app.workspace(tab).unwrap().status {
                SessionStatus::Disconnected(error) => {
                    assert!(error.to_string().contains("changed"))
                }
                other => panic!("{other:?}"),
            }
        }

        #[test]
        fn an_unknown_host_key_during_test_offers_trust_in_the_dialog() {
            let (mut app, _dir) = app();
            postgres_form(&mut app);
            let form = form(&mut app);
            form.ssh = true;
            form.ssh_host = "bastion".into();
            form.ssh_user = "ops".into();
            form.ssh_auth = SshAuthKind::Agent;
            app.apply(Action::TestConnection);
            let Some(Command::Test { request, .. }) = app.backend.sent.last() else {
                panic!("expected a Test");
            };
            let request = *request;
            app.apply(Action::Backend(Event::Tested {
                request,
                result: Err(unknown_key()),
            }));
            assert!(matches!(
                super::form(&mut app).test,
                TestState::Untrusted { .. }
            ));
            app.apply(Action::TrustTestHostKey);
            assert_eq!(app.host_keys.fingerprint("bastion", 22), Some("SHA256:abc"));
            assert!(matches!(
                app.backend.sent.last(),
                Some(Command::Test { .. })
            ));
            assert!(matches!(super::form(&mut app).test, TestState::Running(_)));
        }

        #[test]
        fn trust_from_a_test_names_the_host_tested_not_the_one_typed_since() {
            let (mut app, _dir) = app();
            postgres_form(&mut app);
            let form = form(&mut app);
            form.ssh = true;
            form.ssh_host = "bastion".into();
            form.ssh_user = "ops".into();
            form.ssh_auth = SshAuthKind::Agent;
            app.apply(Action::TestConnection);
            let Some(Command::Test { request, .. }) = app.backend.sent.last() else {
                panic!("expected a Test");
            };
            let request = *request;
            // Edited while the test runs.
            super::form(&mut app).ssh_host = "other".into();
            super::form(&mut app).ssh_port = "2222".into();
            app.apply(Action::Backend(Event::Tested {
                request,
                result: Err(unknown_key()),
            }));
            match &super::form(&mut app).test {
                TestState::Untrusted { host, port, .. } => {
                    assert_eq!((host.as_str(), *port), ("bastion", 22))
                }
                other => panic!("{other:?}"),
            }
            app.apply(Action::TrustTestHostKey);
            assert_eq!(app.host_keys.fingerprint("bastion", 22), Some("SHA256:abc"));
            assert_eq!(app.host_keys.fingerprint("other", 2222), None);
        }

        fn ssh_password_saved(app: &mut App, mode: PasswordMode) -> ConnectionId {
            let conn = ssh_saved(app);
            let saved = app
                .connections
                .connections
                .iter_mut()
                .find(|c| c.id == conn)
                .unwrap();
            saved.spec.ssh.as_mut().unwrap().auth = tabletist_db::SshAuth::Password;
            saved.ssh_secret = mode;
            conn
        }

        fn last_connect_secrets(app: &App) -> Secrets {
            match app.backend.sent.last() {
                Some(Command::Connect { secrets, .. }) => secrets.clone(),
                other => panic!("expected a Connect, got {other:?}"),
            }
        }

        #[test]
        fn a_keyring_ssh_password_is_loaded_before_connecting() {
            let (mut app, _dir) = app();
            let conn = ssh_password_saved(&mut app, PasswordMode::Keyring);
            let tab = app.active_tab_id();
            app.apply(Action::Connect {
                tab,
                conn: conn.clone(),
            });
            let Some(Command::LoadSecret { request, account }) = app.backend.sent.last() else {
                panic!("expected a LoadSecret");
            };
            assert_eq!(account, &crate::secrets::ssh_account(&conn));
            let request = *request;
            app.apply(Action::Backend(Event::SecretLoaded {
                request,
                result: Ok(Some(SecretString("s3".into()))),
            }));
            assert_eq!(
                last_connect_secrets(&app).ssh_password.as_deref(),
                Some("s3")
            );
        }

        #[test]
        fn an_ask_ssh_password_prompts_with_its_own_title() {
            let (mut app, _dir) = app();
            let conn = ssh_password_saved(&mut app, PasswordMode::Ask);
            let tab = app.active_tab_id();
            app.apply(Action::Connect { tab, conn });
            assert_eq!(prompt(&mut app).kind, SecretKind::SshPassword);
            prompt(&mut app).password = "typed".into();
            app.apply(Action::SubmitPassword);
            assert_eq!(
                last_connect_secrets(&app).ssh_password.as_deref(),
                Some("typed")
            );
        }

        #[test]
        fn both_secrets_are_asked_one_after_the_other() {
            let (mut app, _dir) = app();
            let conn = ssh_password_saved(&mut app, PasswordMode::Ask);
            app.connections
                .connections
                .iter_mut()
                .find(|c| c.id == conn)
                .unwrap()
                .password = PasswordMode::Ask;
            let tab = app.active_tab_id();
            app.apply(Action::Connect { tab, conn });
            assert_eq!(prompt(&mut app).kind, SecretKind::Database);
            prompt(&mut app).password = "db".into();
            app.apply(Action::SubmitPassword);
            assert_eq!(prompt(&mut app).kind, SecretKind::SshPassword);
            prompt(&mut app).password = "ssh".into();
            app.apply(Action::SubmitPassword);
            let secrets = last_connect_secrets(&app);
            assert_eq!(
                (secrets.password.as_deref(), secrets.ssh_password.as_deref()),
                (Some("db"), Some("ssh"))
            );
        }

        #[test]
        fn a_rejected_ssh_secret_prompts_and_is_not_resent() {
            let (mut app, _dir) = app();
            let conn = ssh_password_saved(&mut app, PasswordMode::Ask);
            let tab = app.active_tab_id();
            app.apply(Action::Connect { tab, conn });
            prompt(&mut app).password = "wrong".into();
            app.apply(Action::SubmitPassword);
            fail_last_connect(
                &mut app,
                Error::Ssh {
                    stage: SshStage::Secret,
                    message: "the server refused the SSH password".into(),
                },
            );
            assert_eq!(prompt(&mut app).kind, SecretKind::SshPassword);
            assert!(
                prompt(&mut app)
                    .message
                    .as_deref()
                    .unwrap()
                    .contains("refused")
            );
            assert_eq!(app.workspace(tab).unwrap().secrets.ssh_password, None);
        }

        #[test]
        fn an_ssh_secret_from_the_prompt_is_saved_after_connecting() {
            let (mut app, _dir) = app();
            let conn = ssh_password_saved(&mut app, PasswordMode::Ask);
            let tab = app.active_tab_id();
            app.apply(Action::Connect {
                tab,
                conn: conn.clone(),
            });
            prompt(&mut app).password = "good".into();
            prompt(&mut app).save = true;
            app.apply(Action::SubmitPassword);
            assert!(
                sent_secrets(&app).is_empty(),
                "not before the server accepts it"
            );
            let Some(Command::Connect {
                session, request, ..
            }) = app.backend.sent.last()
            else {
                panic!("expected a Connect");
            };
            let (session, request) = (*session, *request);
            app.apply(Action::Backend(Event::Connected {
                session,
                request,
                driver: Driver::Postgres,
                encrypted: false,
            }));
            assert!(app.backend.sent.iter().any(|c| matches!(c,
                Command::StoreSecret { account, secret: Some(_), .. }
                    if account == &crate::secrets::ssh_account(&conn))));
            answer_last_store(&mut app, Ok(()));
            assert_eq!(
                app.connections.get(&conn).unwrap().ssh_secret,
                PasswordMode::Keyring
            );
        }

        #[test]
        fn a_key_that_needs_a_passphrase_asks_for_it() {
            let (mut app, _dir) = app();
            let conn = ssh_saved(&mut app);
            app.connections
                .connections
                .iter_mut()
                .find(|c| c.id == conn)
                .unwrap()
                .spec
                .ssh
                .as_mut()
                .unwrap()
                .auth = tabletist_db::SshAuth::KeyFile { path: "/k".into() };
            let tab = app.active_tab_id();
            app.apply(Action::Connect { tab, conn });
            fail_last_connect(
                &mut app,
                Error::Ssh {
                    stage: SshStage::Secret,
                    message: "the key file needs its passphrase".into(),
                },
            );
            assert_eq!(prompt(&mut app).kind, SecretKind::SshPassphrase);
            prompt(&mut app).password = "pp".into();
            app.apply(Action::SubmitPassword);
            assert_eq!(
                last_connect_secrets(&app).ssh_passphrase.as_deref(),
                Some("pp")
            );
        }

        #[test]
        fn saving_the_form_stores_the_typed_ssh_secret_and_deleting_removes_it() {
            let (mut app, _dir) = app();
            postgres_form(&mut app);
            let form = form(&mut app);
            form.ssh = true;
            form.ssh_host = "bastion".into();
            form.ssh_user = "ops".into();
            form.ssh_auth = SshAuthKind::Password;
            form.ssh_secret = "s3".into();
            app.apply(Action::SaveConnection { connect: false });
            let id = app.connections.connections[0].id.clone();
            assert_eq!(
                app.connections.connections[0].ssh_secret,
                PasswordMode::Keyring
            );
            assert!(app.backend.sent.iter().any(|c| matches!(c,
                Command::StoreSecret { account, secret: Some(_), .. }
                    if account == &crate::secrets::ssh_account(&id))));
            app.apply(Action::DeleteConnection(id.clone()));
            assert!(app.backend.sent.iter().any(|c| matches!(c,
                Command::StoreSecret { account, secret: None, .. }
                    if account == &crate::secrets::ssh_account(&id))));
        }

        #[test]
        fn testing_loads_both_saved_secrets() {
            let (mut app, _dir) = app();
            let conn = ssh_password_saved(&mut app, PasswordMode::Keyring);
            app.connections
                .connections
                .iter_mut()
                .find(|c| c.id == conn)
                .unwrap()
                .password = PasswordMode::Keyring;
            app.apply(Action::EditConnection(conn.clone()));
            app.apply(Action::TestConnection);
            let loads: Vec<_> = app
                .backend
                .sent
                .iter()
                .filter_map(|c| match c {
                    Command::LoadSecret { request, account } => Some((*request, account.clone())),
                    _ => None,
                })
                .collect();
            assert_eq!(loads.len(), 2);
            for (request, account) in loads {
                let secret = if account.ends_with("ssh-secret") {
                    "ssh"
                } else {
                    "db"
                };
                app.apply(Action::Backend(Event::SecretLoaded {
                    request,
                    result: Ok(Some(SecretString(secret.into()))),
                }));
            }
            match app.backend.sent.last() {
                Some(Command::Test { secrets, .. }) => assert_eq!(
                    (secrets.password.as_deref(), secrets.ssh_password.as_deref()),
                    (Some("db"), Some("ssh"))
                ),
                other => panic!("{other:?}"),
            }
        }

        #[test]
        fn a_picked_key_file_fills_the_ssh_field() {
            let (mut app, _dir) = app();
            postgres_form(&mut app);
            app.apply(Action::PickKeyFile);
            let request = form(&mut app).pick_request.unwrap();
            app.apply(Action::Backend(Event::FilePicked {
                request,
                path: Some("/k/id".into()),
            }));
            assert_eq!(form(&mut app).ssh_key_file, "/k/id");
            assert_eq!(form(&mut app).sqlite_path, "");
        }

        fn ssh_conn(
            app: &mut App,
            host: &str,
            auth: tabletist_db::SshAuth,
            secret: PasswordMode,
        ) -> ConnectionId {
            let conn = ssh_saved(app);
            let saved = app
                .connections
                .connections
                .iter_mut()
                .find(|c| c.id == conn)
                .unwrap();
            let ssh = saved.spec.ssh.as_mut().unwrap();
            ssh.host = host.into();
            ssh.auth = auth;
            saved.ssh_secret = secret;
            conn
        }

        #[test]
        fn a_stale_test_load_never_reaches_another_dialog() {
            let (mut app, _dir) = app();
            let a = postgres_saved(&mut app, PasswordMode::Keyring);
            app.apply(Action::EditConnection(a));
            app.apply(Action::TestConnection);
            let Some(Command::LoadSecret {
                request: load_a, ..
            }) = app.backend.sent.last()
            else {
                panic!("expected a LoadSecret");
            };
            let load_a = *load_a;
            app.apply(Action::CloseDialog);
            postgres_form(&mut app);
            form(&mut app).host = "other.example".into();
            app.apply(Action::TestConnection);
            let sent = app.backend.sent.len();
            app.apply(Action::Backend(Event::SecretLoaded {
                request: load_a,
                result: Ok(Some(SecretString("secret-of-A".into()))),
            }));
            assert_eq!(
                app.backend.sent.len(),
                sent,
                "{:?}",
                app.backend.sent.last()
            );
        }

        #[test]
        fn a_stale_connect_load_is_ignored_after_the_tab_moves_on() {
            let (mut app, _dir) = app();
            let a = ssh_conn(
                &mut app,
                "bastion-a",
                tabletist_db::SshAuth::Password,
                PasswordMode::Keyring,
            );
            let b = ssh_conn(
                &mut app,
                "bastion-b",
                tabletist_db::SshAuth::Password,
                PasswordMode::Keyring,
            );
            let tab = app.active_tab_id();
            app.apply(Action::Connect { tab, conn: a });
            let Some(Command::LoadSecret {
                request: load_a, ..
            }) = app.backend.sent.last()
            else {
                panic!("expected a LoadSecret");
            };
            let load_a = *load_a;
            app.apply(Action::Disconnect(tab));
            app.apply(Action::Connect { tab, conn: b });
            let sent = app.backend.sent.len();
            app.apply(Action::Backend(Event::SecretLoaded {
                request: load_a,
                result: Ok(Some(SecretString("ssh-pw-of-A".into()))),
            }));
            assert_eq!(
                app.backend.sent.len(),
                sent,
                "{:?}",
                app.backend.sent.last()
            );
            assert_eq!(app.workspace(tab).unwrap().secrets.ssh_password, None);
        }

        #[test]
        fn changing_the_ssh_login_forgets_the_saved_secret() {
            let (mut app, _dir) = app();
            let id = ssh_conn(
                &mut app,
                "bastion",
                tabletist_db::SshAuth::KeyFile { path: "/k".into() },
                PasswordMode::Keyring,
            );
            app.apply(Action::EditConnection(id.clone()));
            form(&mut app).ssh_auth = SshAuthKind::Password;
            app.apply(Action::SaveConnection { connect: true });
            assert_eq!(
                app.connections.get(&id).unwrap().ssh_secret,
                PasswordMode::None
            );
            assert!(app.backend.sent.iter().any(|c| matches!(c,
                Command::StoreSecret { account, secret: None, .. }
                    if account == &crate::secrets::ssh_account(&id))));
            assert!(!app.backend.sent.iter().any(|c| matches!(c,
                Command::LoadSecret { account, .. } if account == &crate::secrets::ssh_account(&id))));
            assert_eq!(last_connect_secrets(&app).ssh_password, None);
        }

        #[test]
        fn a_late_unknown_key_never_replaces_a_trusted_one() {
            let (mut app, _dir) = app();
            let conn = ssh_saved(&mut app);
            let tab = app.active_tab_id();
            app.apply(Action::Connect { tab, conn });
            app.host_keys.trust("bastion", 22, "SHA256:F1");
            fail_last_connect(&mut app, unknown_key_with("SHA256:F2"));
            assert!(app.dialog.is_none(), "{:?}", app.dialog);
            assert_eq!(app.host_keys.fingerprint("bastion", 22), Some("SHA256:F1"));
            match &app.workspace(tab).unwrap().status {
                SessionStatus::Disconnected(error) => {
                    assert!(error.to_string().contains("changed"), "{error}")
                }
                other => panic!("{other:?}"),
            }
        }

        #[test]
        fn an_unreadable_known_hosts_file_refuses_new_hosts() {
            let dir = tempfile::tempdir().unwrap();
            let dirs = AppDirs::at(dir.path());
            dirs.ensure().unwrap();
            std::fs::write(
                dirs.known_hosts_file(),
                "{ \"hosts\": { \"a:22\": \"SHA256:x\", } }",
            )
            .unwrap();
            let mut app = App::new(dirs, Settings::default(), Backend::recording());
            let conn = ssh_saved(&mut app);
            let tab = app.active_tab_id();
            app.apply(Action::Connect { tab, conn });
            fail_last_connect(&mut app, unknown_key());
            assert!(app.dialog.is_none(), "{:?}", app.dialog);
            match &app.workspace(tab).unwrap().status {
                SessionStatus::Disconnected(error) => {
                    assert!(error.to_string().contains("known_hosts.json"), "{error}")
                }
                other => panic!("{other:?}"),
            }
            let text = std::fs::read_to_string(AppDirs::at(dir.path()).known_hosts_file()).unwrap();
            assert!(
                text.contains("SHA256:x"),
                "the damaged file is left for the user to fix"
            );
        }
    }

    /// Batch 7: count, filters, quick open, tree keys.
    mod power {
        use super::*;
        use crate::model::{FilterRow, ObjectTabId, Pane, TreeKey, TreeNode};
        use crate::testing::{Harness, page};
        use tabletist_db::{ObjectKind, ObjectRef};

        pub(super) fn open_users(harness: &mut Harness) -> (ConnTabId, ObjectTabId) {
            let tab = harness.connect_fake();
            harness.app.apply(Action::OpenObject {
                tab,
                object: ObjectRef::new("main", "users"),
                kind: ObjectKind::Table,
                pin: true,
            });
            harness.answer_rows(page(3, false));
            let id = harness.app.workspace(tab).unwrap().active_object.unwrap();
            (tab, id)
        }

        fn last_count(harness: &Harness) -> (SessionId, RequestId) {
            match harness.app.backend.sent.last() {
                Some(Command::CountRows {
                    session, request, ..
                }) => (*session, *request),
                other => panic!("expected CountRows, got {other:?}"),
            }
        }

        #[test]
        fn count_sends_the_query_and_shows_the_exact_total() {
            let mut harness = Harness::new();
            let (tab, id) = open_users(&mut harness);
            harness.app.apply(Action::CountRows {
                tab,
                object_tab: id,
            });
            let Some(Command::CountRows { query, .. }) = harness.app.backend.sent.last() else {
                panic!("expected CountRows");
            };
            assert_eq!(query.object.name, "users");
            let (session, request) = last_count(&harness);
            harness.app.apply(Action::Backend(Event::Count {
                session,
                request,
                result: Ok(42),
            }));
            let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
            assert_eq!(object.count.value, Some(42));
        }

        #[test]
        fn a_cancelled_count_keeps_the_rows() {
            let mut harness = Harness::new();
            let (tab, id) = open_users(&mut harness);
            harness.app.apply(Action::CountRows {
                tab,
                object_tab: id,
            });
            let (session, request) = last_count(&harness);
            harness.app.apply(Action::CancelQuery(tab));
            assert!(matches!(
                harness.app.backend.sent.last(),
                Some(Command::Cancel { .. })
            ));
            harness.app.apply(Action::Backend(Event::Count {
                session,
                request,
                result: Err(Error::Cancelled),
            }));
            let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
            assert_eq!(object.count.value, None);
            assert!(object.count.error.is_some());
            assert_eq!(object.page().unwrap().rows.len(), 3, "rows untouched");
        }

        #[test]
        fn refresh_forgets_the_count() {
            let mut harness = Harness::new();
            let (tab, id) = open_users(&mut harness);
            harness.app.apply(Action::CountRows {
                tab,
                object_tab: id,
            });
            let (session, request) = last_count(&harness);
            harness.app.apply(Action::Backend(Event::Count {
                session,
                request,
                result: Ok(42),
            }));
            harness.app.apply(Action::Refresh(tab));
            assert_eq!(
                harness
                    .app
                    .workspace(tab)
                    .unwrap()
                    .object_tab(id)
                    .unwrap()
                    .count
                    .value,
                None
            );
        }

        fn age_over(value: &str) -> Vec<FilterRow> {
            vec![FilterRow {
                column: "age".into(),
                op: tabletist_db::FilterOp::Gt,
                value: value.into(),
            }]
        }

        #[test]
        fn applying_filters_refetches_from_the_first_page_and_forgets_the_count() {
            let mut harness = Harness::new();
            let (tab, id) = open_users(&mut harness);
            {
                let object = harness.app.object_tab_mut(tab, id).unwrap();
                object.query.offset = 300;
                object.count.value = Some(9);
                object.filter.rows = age_over("30");
            }
            harness.app.apply(Action::ApplyFilters {
                tab,
                object_tab: id,
            });
            let Some(Command::FetchRows { query, .. }) = harness.app.backend.sent.last() else {
                panic!("expected FetchRows");
            };
            assert_eq!(query.offset, 0);
            assert_eq!(query.filters.len(), 1);
            let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
            assert_eq!(object.count.value, None);
            assert!(object.pinned);
        }

        #[test]
        fn a_count_for_old_filters_is_dropped() {
            let mut harness = Harness::new();
            let (tab, id) = open_users(&mut harness);
            harness.app.apply(Action::CountRows {
                tab,
                object_tab: id,
            });
            let (session, request) = last_count(&harness);
            harness.app.object_tab_mut(tab, id).unwrap().filter.rows = age_over("30");
            harness.app.apply(Action::ApplyFilters {
                tab,
                object_tab: id,
            });
            harness.app.apply(Action::Backend(Event::Count {
                session,
                request,
                result: Ok(1000),
            }));
            assert_eq!(
                harness
                    .app
                    .workspace(tab)
                    .unwrap()
                    .object_tab(id)
                    .unwrap()
                    .count
                    .value,
                None
            );
        }

        #[test]
        fn a_failed_filter_keeps_the_bar_and_applying_again_recovers() {
            let mut harness = Harness::new();
            let (tab, id) = open_users(&mut harness);
            harness.app.apply(Action::ToggleFilterBar(tab));
            harness.app.object_tab_mut(tab, id).unwrap().filter.rows = age_over("old");
            harness.app.apply(Action::ApplyFilters {
                tab,
                object_tab: id,
            });
            let (session, request) = match harness.app.backend.sent.last() {
                Some(Command::FetchRows {
                    session, request, ..
                }) => (*session, *request),
                other => panic!("{other:?}"),
            };
            harness.app.apply(Action::Backend(Event::Rows {
                session,
                request,
                result: Err(Error::query("invalid input syntax for type integer")),
            }));
            let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
            assert!(object.filter.open);
            assert_eq!(object.filter.rows[0].value, "old");
            assert!(object.rows.error.is_some());
            harness.app.object_tab_mut(tab, id).unwrap().filter.rows[0].value = "30".into();
            harness.app.apply(Action::ApplyFilters {
                tab,
                object_tab: id,
            });
            harness.answer_rows(page(2, false));
            let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
            assert!(object.rows.error.is_none());
            assert_eq!(object.page().unwrap().rows.len(), 2);
        }

        #[test]
        fn clearing_filters_closes_the_bar_and_refetches_everything() {
            let mut harness = Harness::new();
            let (tab, id) = open_users(&mut harness);
            harness.app.apply(Action::ToggleFilterBar(tab));
            harness.app.object_tab_mut(tab, id).unwrap().filter.rows[0].value = "1".into();
            harness.app.apply(Action::ApplyFilters {
                tab,
                object_tab: id,
            });
            harness.app.apply(Action::ClearFilters {
                tab,
                object_tab: id,
            });
            let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
            assert!(!object.filter.open);
            assert!(object.query.filters.is_empty() && object.query.raw_where.is_none());
            assert!(matches!(
                harness.app.backend.sent.last(),
                Some(Command::FetchRows { query, .. }) if query.filters.is_empty()
            ));
        }

        #[test]
        fn opening_the_filter_bar_starts_with_one_row_on_the_first_column() {
            let mut harness = Harness::new();
            let (tab, id) = open_users(&mut harness);
            harness.app.apply(Action::ToggleFilterBar(tab));
            let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
            assert!(object.filter.open && object.filter.focus);
            assert_eq!(object.filter.rows.len(), 1);
            assert_eq!(
                object.filter.rows[0].column,
                object.page().unwrap().columns[0].name
            );
            harness.app.apply(Action::ToggleFilterBar(tab));
            assert!(
                !harness
                    .app
                    .workspace(tab)
                    .unwrap()
                    .object_tab(id)
                    .unwrap()
                    .filter
                    .open
            );
        }

        pub(super) fn with_objects(
            harness: &mut Harness,
            tab: ConnTabId,
            schema: &str,
            names: &[&str],
        ) {
            let workspace = harness.app.workspace_mut(tab).unwrap();
            let node = workspace.tree.nodes.entry(schema.to_owned()).or_default();
            node.objects.value = Some(
                names
                    .iter()
                    .map(|name| tabletist_db::ObjectInfo {
                        name: (*name).into(),
                        kind: ObjectKind::Table,
                        estimated_rows: None,
                    })
                    .collect(),
            );
        }

        #[test]
        fn quick_open_ranks_and_caps_results() {
            let mut harness = Harness::new();
            let tab = harness.connect_fake();
            let many: Vec<String> = (0..200).map(|i| format!("log_{i:03}")).collect();
            let many: Vec<&str> = many.iter().map(String::as_str).collect();
            with_objects(&mut harness, tab, "main", &many);
            with_objects(
                &mut harness,
                tab,
                "shop",
                &["orders", "product_reviews_old"],
            );
            let matches = harness.app.quick_open_matches(tab, "ord");
            assert_eq!(matches[0].0.name, "orders");
            assert!(harness.app.quick_open_matches(tab, "log").len() <= 50);
            let qualified = harness.app.quick_open_matches(tab, "shop.ord");
            assert_eq!(qualified[0].0.name, "orders");
            assert_eq!(qualified[0].0.schema, "shop");
        }

        #[test]
        fn picking_a_quick_open_result_opens_and_pins_it() {
            let mut harness = Harness::new();
            let tab = harness.connect_fake();
            with_objects(&mut harness, tab, "shop", &["orders", "users"]);
            harness.app.apply(Action::OpenQuickOpen);
            let Some(Dialog::QuickOpen(open)) = &mut harness.app.dialog else {
                panic!("expected quick open");
            };
            // The fake connection has a main.users too: name the schema.
            open.query = "shop.us".into();
            harness.app.apply(Action::QuickOpenPick);
            assert!(harness.app.dialog.is_none());
            let workspace = harness.app.workspace(tab).unwrap();
            let object = workspace
                .object_tab(workspace.active_object.unwrap())
                .unwrap();
            assert_eq!(object.object, ObjectRef::new("shop", "users"));
            assert!(object.pinned);
        }

        fn tree_tab(harness: &mut Harness) -> ConnTabId {
            let tab = harness.connect_fake();
            // Two schemas; the first shown, with a folded group of two
            // objects and one object on its own.
            harness.app.workspace_mut(tab).unwrap().tree.schemas.value =
                Some(vec!["main".into(), "temp".into()]);
            with_objects(
                harness,
                tab,
                "main",
                &["order_items", "order_notes", "users"],
            );
            harness
                .app
                .workspace_mut(tab)
                .unwrap()
                .tree
                .nodes
                .get_mut("main")
                .unwrap()
                .expanded = true;
            tab
        }

        fn cursor(harness: &Harness, tab: ConnTabId) -> Option<TreeNode> {
            harness.app.workspace(tab).unwrap().tree.cursor.clone()
        }

        fn object_node(name: &str) -> TreeNode {
            TreeNode::Object(ObjectRef::new("main", name), ObjectKind::Table)
        }

        fn group_node() -> TreeNode {
            TreeNode::Group("main".into(), "order".into())
        }

        fn group_open(harness: &Harness, tab: ConnTabId) -> bool {
            harness.app.workspace(tab).unwrap().tree.nodes["main"]
                .open_groups
                .contains("order")
        }

        #[test]
        fn arrows_walk_the_visible_tree_and_enter_opens() {
            let mut harness = Harness::new();
            let tab = tree_tab(&mut harness);
            let key = |harness: &mut Harness, key| harness.app.apply(Action::TreeKey { tab, key });
            key(&mut harness, TreeKey::Down);
            assert_eq!(cursor(&harness, tab), Some(group_node()));
            key(&mut harness, TreeKey::Down);
            assert_eq!(cursor(&harness, tab), Some(object_node("users")));
            key(&mut harness, TreeKey::Home);
            assert_eq!(cursor(&harness, tab), Some(group_node()));
            key(&mut harness, TreeKey::Enter); // unfold the group
            assert!(group_open(&harness, tab));
            key(&mut harness, TreeKey::Down);
            assert_eq!(cursor(&harness, tab), Some(object_node("order_items")));
            key(&mut harness, TreeKey::End);
            assert_eq!(cursor(&harness, tab), Some(object_node("users")));
            key(&mut harness, TreeKey::Up);
            key(&mut harness, TreeKey::Enter);
            let workspace = harness.app.workspace(tab).unwrap();
            let object = workspace
                .object_tab(workspace.active_object.unwrap())
                .unwrap();
            assert_eq!(object.object.name, "order_notes");
            assert!(object.pinned);
            assert_eq!(workspace.pane, Pane::Grid);
        }

        #[test]
        fn left_and_right_fold_and_climb() {
            let mut harness = Harness::new();
            let tab = tree_tab(&mut harness);
            harness.app.apply(Action::ToggleGroup {
                tab,
                schema: "main".into(),
                prefix: "order".into(),
            });
            harness.app.apply(Action::SetTreeCursor {
                tab,
                node: object_node("order_notes"),
            });
            let key = |harness: &mut Harness, key| harness.app.apply(Action::TreeKey { tab, key });
            key(&mut harness, TreeKey::Left); // up to the group
            assert_eq!(cursor(&harness, tab), Some(group_node()));
            key(&mut harness, TreeKey::Left); // fold the group
            assert!(!group_open(&harness, tab));
            key(&mut harness, TreeKey::Right); // unfold it
            assert!(group_open(&harness, tab));
            key(&mut harness, TreeKey::Right); // into it
            assert_eq!(cursor(&harness, tab), Some(object_node("order_items")));
        }

        #[test]
        fn a_cursor_on_a_hidden_row_restarts_at_the_top() {
            let mut harness = Harness::new();
            let tab = tree_tab(&mut harness);
            harness.app.apply(Action::SetTreeCursor {
                tab,
                node: object_node("orders"),
            });
            harness.app.workspace_mut(tab).unwrap().tree.filter = "users".into();
            harness.app.apply(Action::TreeKey {
                tab,
                key: TreeKey::Down,
            });
            let cursor = cursor(&harness, tab).unwrap();
            let rows = harness
                .app
                .workspace(tab)
                .unwrap()
                .tree
                .visible_rows(Driver::Sqlite, false);
            assert!(
                rows.iter().any(|row| row.node == cursor),
                "{cursor:?} is visible"
            );
        }

        #[test]
        fn applying_filters_clears_the_old_page_but_keeps_the_columns() {
            let mut harness = Harness::new();
            let (tab, id) = open_users(&mut harness);
            harness.app.apply(Action::ToggleFilterBar(tab));
            harness.app.object_tab_mut(tab, id).unwrap().filter.rows = age_over("old");
            harness.app.apply(Action::ApplyFilters {
                tab,
                object_tab: id,
            });
            let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
            assert!(object.page().is_none(), "no unfiltered rows under a filter");
            let (session, request) = match harness.app.backend.sent.last() {
                Some(Command::FetchRows {
                    session, request, ..
                }) => (*session, *request),
                other => panic!("{other:?}"),
            };
            harness.app.apply(Action::Backend(Event::Rows {
                session,
                request,
                result: Err(Error::query("invalid input syntax for type integer")),
            }));
            let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
            assert!(object.page().is_none());
            assert!(
                !object.filter.columns.is_empty(),
                "the bar still lists columns"
            );
        }

        #[test]
        fn a_count_pending_at_reconnect_is_forgotten() {
            let mut harness = Harness::new();
            let (tab, id) = open_users(&mut harness);
            harness.app.apply(Action::CountRows {
                tab,
                object_tab: id,
            });
            harness.app.apply(Action::Reconnect(tab));
            let (session, request) = match harness
                .app
                .backend
                .sent
                .iter()
                .rev()
                .find(|c| matches!(c, Command::Connect { .. }))
            {
                Some(Command::Connect {
                    session, request, ..
                }) => (*session, *request),
                other => panic!("{other:?}"),
            };
            harness.app.apply(Action::Backend(Event::Connected {
                session,
                request,
                driver: Driver::Sqlite,
                encrypted: false,
            }));
            let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
            assert!(!object.count.is_loading(), "not stuck on Counting");
        }

        #[test]
        fn retry_after_a_failed_filter_uses_the_bar() {
            let mut harness = Harness::new();
            let (tab, id) = open_users(&mut harness);
            harness.app.apply(Action::ToggleFilterBar(tab));
            harness.app.object_tab_mut(tab, id).unwrap().filter.rows = age_over("old");
            harness.app.apply(Action::ApplyFilters {
                tab,
                object_tab: id,
            });
            let (session, request) = match harness.app.backend.sent.last() {
                Some(Command::FetchRows {
                    session, request, ..
                }) => (*session, *request),
                other => panic!("{other:?}"),
            };
            harness.app.apply(Action::Backend(Event::Rows {
                session,
                request,
                result: Err(Error::query("invalid input syntax for type integer")),
            }));
            harness.app.object_tab_mut(tab, id).unwrap().filter.rows[0].value = "30".into();
            harness.app.apply(Action::RetryRows {
                tab,
                object_tab: id,
            });
            match harness.app.backend.sent.last() {
                Some(Command::FetchRows { query, .. }) => assert_eq!(query.filters[0].value, "30"),
                other => panic!("{other:?}"),
            }
        }

        #[test]
        fn retry_with_an_unchanged_filter_keeps_the_page() {
            let mut harness = Harness::new();
            let (tab, id) = open_users(&mut harness);
            harness.app.apply(Action::ToggleFilterBar(tab));
            harness.app.object_tab_mut(tab, id).unwrap().filter.rows = age_over("30");
            harness.app.apply(Action::ApplyFilters {
                tab,
                object_tab: id,
            });
            harness.answer_rows(page(300, true));
            harness.app.apply(Action::NextPage {
                tab,
                object_tab: id,
            });
            let (session, request, offset) = match harness.app.backend.sent.last() {
                Some(Command::FetchRows {
                    session,
                    request,
                    query,
                }) => (*session, *request, query.offset),
                other => panic!("{other:?}"),
            };
            assert_ne!(offset, 0);
            harness.app.apply(Action::Backend(Event::Rows {
                session,
                request,
                result: Err(Error::query("connection reset")),
            }));
            harness.app.apply(Action::RetryRows {
                tab,
                object_tab: id,
            });
            match harness.app.backend.sent.last() {
                Some(Command::FetchRows { query, .. }) => assert_eq!(query.offset, offset),
                other => panic!("{other:?}"),
            }
        }

        #[test]
        fn end_without_a_cursor_goes_to_the_bottom() {
            let mut harness = Harness::new();
            let tab = tree_tab(&mut harness);
            harness.app.apply(Action::TreeKey {
                tab,
                key: TreeKey::End,
            });
            assert_eq!(cursor(&harness, tab), Some(object_node("users")));
        }

        #[test]
        fn adding_a_condition_to_an_empty_bar_uses_the_first_column() {
            let mut harness = Harness::new();
            let (tab, id) = open_users(&mut harness);
            harness.app.apply(Action::ToggleFilterBar(tab));
            harness.app.apply(Action::RemoveFilterRow {
                tab,
                object_tab: id,
                index: 0,
            });
            harness.app.apply(Action::AddFilterRow {
                tab,
                object_tab: id,
            });
            let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
            assert_eq!(
                object.filter.rows[0].column,
                object.page().unwrap().columns[0].name
            );
        }

        #[test]
        fn command_f_from_structure_opens_the_data_view() {
            let mut harness = Harness::new();
            let (tab, id) = open_users(&mut harness);
            harness.app.apply(Action::SetView {
                tab,
                object_tab: id,
                view: crate::model::ObjectView::Structure,
            });
            harness.app.apply(Action::ToggleFilterBar(tab));
            let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
            assert_eq!(object.view, crate::model::ObjectView::Data);
            assert!(object.filter.open);
        }

        #[test]
        fn cancelling_the_ssh_prompt_leaves_a_cancelled_tab() {
            let mut harness = Harness::new();
            let tab = harness.connect_fake();
            let workspace = harness.app.workspace_mut(tab).unwrap();
            workspace.status = SessionStatus::Connecting {
                request: RequestId(999),
            };
            harness.app.dialog = Some(Dialog::Password(Box::new(PasswordPrompt {
                tab,
                kind: SecretKind::SshPassword,
                name: "Prod".into(),
                password: String::new(),
                save: false,
                message: None,
            })));
            harness.app.apply(Action::CancelPassword);
            assert!(matches!(
                harness.app.workspace(tab).unwrap().status,
                SessionStatus::Cancelled
            ));
        }
    }
}
