//! Application state and the reducer.

use crate::backend::{Backend, SessionId};
use crate::connections::SavedConnections;
use std::collections::HashMap;

use tabletist_db::{
    Driver, Error, FilterOp, HostKeys, ObjectKind, ObjectRef, Secrets, Sort, SortDir, SshStage,
};

use crate::backend::{CancelReason, Command, Event, Opened, RequestId, StateFile};
use crate::completion::Need;
use crate::connections::{PasswordMode, SavedConnection};
use crate::edit::EditorPlace;
use crate::i18n::Locale;
use crate::model::{Action, ConnTab, ConnTabContent, ConnTabId, PickerState};
use crate::model::{
    Advance, CellPos, Completion, ConnectionForm, Dialog, EditStart, Fetch, FilterBar, FilterRow,
    Held, HostKeyPrompt, LeavePrompt, ObjectTab, ObjectView, Pane, PasswordPrompt, PickTarget,
    QuickOpen, ResultPane, RunMode, SecretKind, SessionStatus, SqlTab, Tab, TabId, TestState,
    TextPrint, Tree, TreeKey, TreeNode, Wanted, Workspace,
};
use crate::paths::AppDirs;
use crate::secrets::{SecretString, password_account, ssh_account};
use crate::settings::{Loaded, Settings, SettingsFile, Source};
use crate::theme::{self, Catalog, Palette};

mod editing;

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

/// The native title bar the connection bar shares: its height, and how far
/// the window's own buttons (the macOS traffic lights) reach from the left.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct TitleBar {
    pub height: f32,
    pub inset: f32,
}

pub struct App {
    pub dirs: AppDirs,
    pub settings: Settings,
    /// The settings file as the app last read or wrote it or, before any
    /// file exists, as it would be written.
    pub settings_file: SettingsFile,
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
    /// The window's own title bar the connection bar shares (macOS),
    /// measured from the window every frame; zero elsewhere.
    pub titlebar: TitleBar,
    /// The OS theme seen last frame, to notice light/dark switches.
    system_theme: Option<egui::Theme>,
    /// Whether the desktop's themes are followed (not in tests or the demo).
    follow_desktop: bool,
    /// The settings named another theme since the last `logic`.
    theme_changed: bool,
    /// The window title last sent, so it is sent only when it changes.
    window_title: String,
    /// SQL editors closed since the last frame: what egui keeps for each
    /// (its undo history holds copies of the script) is dropped once a
    /// frame has the `egui::Context` to drop it from.
    pub closed_editors: Vec<(ConnTabId, TabId)>,
    /// The window is closing: it was asked to, and no tab holds edits any
    /// more, or the user gave them up.
    pub closing: bool,
    next_id: u64,
}

impl App {
    pub fn new(dirs: AppDirs, loaded: Loaded, backend: Backend) -> Self {
        let source = loaded.source;
        let (settings, settings_file) = loaded.into_parts();
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
            settings_file,
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
            follow_desktop: false,
            theme_changed: false,
            window_title: "Tabletist".into(),
            closed_editors: Vec::new(),
            closing: false,
            next_id: 1,
        };
        let tab = app.picker_tab();
        app.tabs.push(tab);
        // An older file is written in this version once, off the UI thread.
        if upgraded {
            app.save_connections();
        }
        // So are settings that were read from the old settings.json.
        if source == Source::Json {
            app.save_settings();
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

    /// The open connections with their tabs, in the order the header shows
    /// them. The picker is not one of them.
    pub fn open_connections(&self) -> impl Iterator<Item = (ConnTabId, &Workspace)> {
        self.tabs.iter().filter_map(|tab| match &tab.content {
            ConnTabContent::Workspace(workspace) => Some((tab.id, &**workspace)),
            ConnTabContent::Picker(_) => None,
        })
    }

    /// The first tab that has the saved connection `conn` open.
    pub fn tab_showing(&self, conn: &crate::connections::ConnectionId) -> Option<ConnTabId> {
        self.open_connections()
            .find(|(_, workspace)| workspace.conn_id == *conn)
            .map(|(tab, _)| tab)
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

    /// Where the picker tab is, when one is open. There is never more than
    /// one: the saved connections are one screen.
    fn picker_index(&self) -> Option<usize> {
        self.tabs
            .iter()
            .position(|tab| matches!(tab.content, ConnTabContent::Picker(_)))
    }

    /// Applies queued actions until none are left (an action may queue more).
    pub fn apply_actions(&mut self) {
        while !self.actions.is_empty() {
            for action in std::mem::take(&mut self.actions) {
                self.apply(action);
            }
        }
        self.format_rows();
        self.make_reviews();
    }

    /// Formats the row each open row panel shows, once per selection or
    /// page (or SQL result), so drawing never reads a whole (possibly huge)
    /// value.
    fn format_rows(&mut self) {
        for tab in &mut self.tabs {
            let ConnTabContent::Workspace(workspace) = &mut tab.content else {
                continue;
            };
            let shown = workspace.row_panel_tab();
            for sql in workspace.sql_tabs_mut() {
                let row = sql.selected_row().filter(|_| shown == Some(sql.id));
                let Some(row) = row else {
                    // Nothing shows it: free the text.
                    sql.fields = None;
                    continue;
                };
                if sql.selected_fields().is_some() {
                    continue;
                }
                let request = sql.run.loaded;
                let values = sql.shown_rows().and_then(|(_, rows, _)| rows.get(row));
                let nothing = std::collections::BTreeMap::new();
                sql.fields = values.map(|values| row_fields(request, row, values, &nothing));
            }
            for object in workspace.object_tabs_mut() {
                let row = object
                    .selection
                    .filter(|_| shown == Some(object.id))
                    .map(|cell| cell.row);
                let Some((row, page)) = row.zip(object.page()) else {
                    // Nothing shows it: free the text.
                    object.fields = None;
                    continue;
                };
                if object.selected_fields().is_some() {
                    continue;
                }
                // What is pending in the row shows in the panel as it does
                // in the grid. The set's every change drops this text, so
                // it is made again from the set as it stands.
                let request = object.rows.loaded;
                object.fields = page
                    .rows
                    .get(row)
                    .map(|values| row_fields(request, row, values, &object.edits.cells));
            }
        }
    }

    pub fn apply(&mut self, action: Action) {
        // What a prompt asks about must not change under it. The keys still
        // run while a dialog is open, and a click can be a frame behind it.
        if matches!(
            self.dialog,
            Some(Dialog::Leave(_) | Dialog::ConfirmWrite(_) | Dialog::Conflict(_))
        ) && editing::dropped_under_a_prompt(&action)
        {
            return;
        }
        // An action that would drop a page with pending changes waits for
        // the user's answer (see `Dialog::Leave`).
        let dropped = self.dropped_by(&action);
        if !dropped.is_empty() {
            self.hold(Held::Action(Box::new(action)), dropped);
            return;
        }
        match action {
            Action::ShowConnections => match self.picker_index() {
                Some(index) => self.active = index,
                None => {
                    let tab = self.picker_tab();
                    self.tabs.push(tab);
                    self.active = self.tabs.len() - 1;
                }
            },
            Action::CloseConnTab(id) => self.close_tab(id),
            Action::ActivateConnTab(id) => {
                if let Some(index) = self.tab_index(id) {
                    self.active = index;
                }
            }
            Action::ActivateConnection(position) => {
                let tab = self.open_connections().nth(position).map(|(tab, _)| tab);
                if let Some(index) = tab.and_then(|tab| self.tab_index(tab)) {
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
                    if self.picker_index().is_some() {
                        // The saved connections have a tab already: this
                        // one closes, and that one shows.
                        self.close_tab(tab);
                        self.apply(Action::ShowConnections);
                    } else {
                        self.backend.send(Command::Close { session });
                        self.close_editors(tab);
                        if let Some(entry) = self.tabs.iter_mut().find(|t| t.id == tab) {
                            entry.content = ConnTabContent::Picker(PickerState::default());
                        }
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
            Action::FoldDocuments { tab, id } => {
                if let Some(workspace) = self.workspace_mut(tab) {
                    workspace.fold_documents = Some(id);
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
            Action::OpenCommand(tab) => {
                // The keys do not run under a dialog, and neither does
                // the prompt they open.
                if self.dialog.is_some() {
                    return;
                }
                if let Some(workspace) = self.workspace_mut(tab) {
                    workspace.command = Some(String::new());
                    workspace.focus_command = true;
                    workspace.command_error = None;
                    workspace.save_refused = false;
                    workspace.review_refused = false;
                }
            }
            Action::CloseCommand(tab) => {
                if let Some(workspace) = self.workspace_mut(tab) {
                    workspace.command = None;
                    workspace.focus_command = false;
                    workspace.command_error = None;
                    workspace.save_refused = false;
                    workspace.review_refused = false;
                }
            }
            Action::RunCommand(tab) => self.run_command(tab),
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
            Action::ActivateTab { tab, id } => {
                if let Some(workspace) = self.workspace_mut(tab)
                    && workspace.tab(id).is_some()
                {
                    workspace.active_tab = Some(id);
                }
            }
            Action::CloseTab { tab, id } => {
                if let Some(workspace) = self.workspace_mut(tab)
                    && let Some(index) = workspace.tabs.iter().position(|t| t.id() == id)
                {
                    let closed = workspace.tabs.remove(index);
                    let session = workspace.session;
                    let editor = closed.as_sql().map(|sql| (tab, sql.id));
                    if workspace.active_tab == Some(id) {
                        workspace.active_tab = workspace
                            .tabs
                            .get(index)
                            .or_else(|| workspace.tabs.last())
                            .map(Tab::id);
                    }
                    // Nothing will show what it was loading (a count or a
                    // script can hold the connection for minutes).
                    self.cancel(session, closed.pending());
                    self.closed_editors.extend(editor);
                }
            }
            Action::PinObjectTab { tab, object_tab } => {
                if let Some(object) = self.object_tab_mut(tab, object_tab) {
                    object.pinned = true;
                }
            }
            Action::CycleTab { tab, step } => {
                if let Some(workspace) = self.workspace_mut(tab)
                    && !workspace.tabs.is_empty()
                {
                    let len = workspace.tabs.len() as isize;
                    let current = workspace
                        .active_tab
                        .and_then(|id| workspace.tabs.iter().position(|t| t.id() == id))
                        .unwrap_or(0) as isize;
                    let next = (current + step).rem_euclid(len) as usize;
                    workspace.active_tab = Some(workspace.tabs[next].id());
                }
            }
            Action::SetView {
                tab,
                object_tab,
                view,
            } => {
                // An editor is the grid's: on another view it would stay
                // open where nothing shows it. Left, as when the keyboard
                // goes elsewhere: what was typed is kept.
                let changes = self
                    .workspace(tab)
                    .and_then(|workspace| workspace.object_tab(object_tab))
                    .is_some_and(|object| object.view != view);
                if changes {
                    self.close_editor(tab, object_tab, true);
                }
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
                // Back by the size of the page about to be fetched, which
                // must end where the one on screen begins. That one may be
                // of another size (the size changed while its session was
                // down): back by its limit, the rows between the two would
                // be skipped. The page fetched has the settings' size, or
                // is only the rows before this one when they are fewer (the
                // size grew on a page near the start): a whole page from
                // the start would show the first rows of this one a second
                // time, and Next from it would not come back here.
                let page_size = self.settings.page_size;
                let step = self.object_tab_mut(tab, object_tab).and_then(|object| {
                    if object.query.offset == 0 {
                        return None;
                    }
                    let step = u32::try_from(object.query.offset)
                        .map_or(page_size, |before| before.min(page_size));
                    object.query.offset -= u64::from(step);
                    object.pinned = true;
                    object.selection = None;
                    object.rows.value = None;
                    Some(step)
                });
                if let Some(step) = step {
                    self.fetch_page(tab, object_tab, step);
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
            Action::SaveValue { tab, id, row, col } => {
                // The value the row panel showed: it may be gone by now (a
                // refresh, another page), and then there is nothing to save.
                let found = self.workspace(tab).and_then(|workspace| {
                    let (table, columns, rows) = match workspace.tab(id)? {
                        Tab::Object(object) => {
                            let page = object.page()?;
                            (
                                object.object.name.as_str(),
                                page.columns.as_slice(),
                                page.rows.as_slice(),
                            )
                        }
                        Tab::Sql(sql) => {
                            let (columns, rows, _) = sql.shown_rows()?;
                            ("query", columns, rows)
                        }
                    };
                    let tabletist_db::Value::Bytes(bytes) = rows.get(row)?.get(col)? else {
                        return None;
                    };
                    let name = crate::ui::format::save_name(table, &columns.get(col)?.name, bytes);
                    Some((name, bytes.to_vec()))
                });
                if let Some((name, bytes)) = found {
                    self.backend.save_bytes("Save value", name, bytes);
                }
            }
            Action::GridKeys(tab) => {
                if let Some(workspace) = self.workspace_mut(tab) {
                    workspace.pane = Pane::Grid;
                }
            }
            Action::SelectCell { tab, id, cell } => {
                // The editor's text is kept, and the note of a locked cell
                // was about the cell the selection leaves.
                self.close_editor(tab, id, true);
                if let Some(object) = self.object_tab_mut(tab, id) {
                    object.edits.why = None;
                    // The field that was to get the keyboard back was the
                    // row's the selection leaves.
                    object.focus_field = None;
                    object.selection = Some(cell);
                    object.pinned = true;
                } else if let Some(sql) = self.sql_tab_mut(tab, id) {
                    // The result may have been replaced since the frame
                    // that drew the cell.
                    let (height, width) = sql.dims();
                    if cell.row < height && cell.col < width {
                        sql.selection = Some(cell);
                    }
                }
                if let Some(workspace) = self.workspace_mut(tab) {
                    workspace.pane = Pane::Grid;
                }
            }
            Action::EditCell {
                tab,
                id,
                cell,
                start,
            } => self.edit_cell(tab, id, cell, start, EditorPlace::Grid),
            Action::EditField { tab, id, cell } => {
                self.edit_cell(tab, id, cell, EditStart::Value, EditorPlace::Panel);
            }
            Action::EditRow { tab, id } => self.edit_row(tab, id),
            Action::FieldFocused { tab, id, col } => {
                // Only the request that was met: another edit may have
                // ended since the frame that drew the field.
                if let Some(object) = self.object_tab_mut(tab, id)
                    && object.focus_field == Some(col)
                {
                    object.focus_field = None;
                }
            }
            Action::EditorTyped { tab, id } => {
                let problem = self.editor_problem(tab, id);
                if let Some(editor) = self.editor_mut(tab, id) {
                    editor.problem = problem;
                    editor.touched = true;
                }
            }
            Action::CommitEdit { tab, id, then } => {
                let field = self.panel_field(tab, id);
                if self.close_editor(tab, id, false) {
                    // In the panel nothing moves: the keyboard goes back
                    // to the field that was edited.
                    if let Some(col) = field {
                        self.back_to_field(tab, id, col);
                        return;
                    }
                    let (rows, cols) = match then {
                        Advance::Stay => (0, 0),
                        Advance::Down => (1, 0),
                        Advance::Right => (0, 1),
                        Advance::Left => (0, -1),
                    };
                    if (rows, cols) != (0, 0) {
                        self.apply(Action::MoveSelection {
                            tab,
                            id,
                            rows,
                            cols,
                        });
                    }
                }
            }
            Action::LeaveEdit {
                tab,
                id,
                cell,
                place,
            } => {
                // Only the editor that was left: another may have opened
                // since, in the frame that took the keyboard from it.
                let left = self
                    .editor_mut(tab, id)
                    .is_some_and(|editor| editor.cell == cell && editor.place == place);
                if left {
                    self.close_editor(tab, id, true);
                }
            }
            Action::CancelEdit { tab, id } => {
                let field = self.panel_field(tab, id);
                if let Some(object) = self.object_tab_mut(tab, id) {
                    object.edits.editor = None;
                }
                if let Some(col) = field {
                    self.back_to_field(tab, id, col);
                }
            }
            Action::EditorBreak { tab, id } => {
                if let Some(editor) = self.editor_mut(tab, id) {
                    // At the end of the text, where the cursor of a field
                    // that just opened is. A break elsewhere is typed in
                    // the large editor, which is the grid's: an edit begun
                    // in the row panel goes on at its cell.
                    editor.text.push('\n');
                    editor.large = true;
                    editor.place = EditorPlace::Grid;
                    editor.focus = true;
                    editor.touched = true;
                }
            }
            Action::SetNull { tab, id } => self.set_null(tab, id),
            Action::RevertCell { tab, id } => {
                // Not under a save: its answer is put into this set.
                if let Some(object) = self.object_tab_mut(tab, id)
                    && let Some(cell) = object.selection
                    && object.edits.editor.is_none()
                    && object.edits.saving.is_none()
                {
                    object.edits.revert((cell.row, cell.col));
                    object.fields = None;
                }
            }
            Action::DiscardEdits { tab, id } => {
                // Not under a save: dropping the set would drop the save
                // with it, and its answer would find no tab to tell.
                if let Some(object) = self.object_tab_mut(tab, id)
                    && object.edits.saving.is_none()
                {
                    object.edits.discard();
                    object.fields = None;
                }
            }
            Action::DismissNote { tab, id } => {
                if let Some(object) = self.object_tab_mut(tab, id) {
                    object.edits.note = None;
                    object.edits.saved = None;
                }
            }
            Action::WriteEdits { tab, id } => self.write_edits(tab, id, None),
            Action::ReviewEdits { tab, id, show } => self.review_edits(tab, id, show),
            Action::LeaveStay => {
                if matches!(self.dialog, Some(Dialog::Leave(_))) {
                    self.dialog = None;
                }
            }
            Action::LeaveDiscard => {
                // The kind is checked before the dialog is taken, as
                // `SubmitPassword` does: another dialog is not closed by it.
                if !matches!(self.dialog, Some(Dialog::Leave(_))) {
                    return;
                }
                let Some(Dialog::Leave(prompt)) = self.dialog.take() else {
                    return;
                };
                let LeavePrompt { held, tabs, .. } = *prompt;
                // A save one of them runs goes with its set: its answer
                // finds no tab saving, and tells none.
                for (tab, id) in tabs {
                    if let Some(object) = self.object_tab_mut(tab, id) {
                        object.edits.discard();
                        object.fields = None;
                    }
                }
                self.perform(held);
            }
            Action::LeaveSave => {
                if !matches!(self.dialog, Some(Dialog::Leave(_))) {
                    return;
                }
                let Some(Dialog::Leave(prompt)) = self.dialog.take() else {
                    return;
                };
                let LeavePrompt {
                    held,
                    tabs,
                    can_save,
                    ..
                } = *prompt;
                // Save is offered for one tab only, and only where its save
                // is not disabled. Anything else keeps the changes and
                // drops what was held.
                if let ([(tab, id)], true) = (tabs.as_slice(), can_save) {
                    self.write_as_answer(*tab, *id, Some(held));
                }
            }
            Action::ConfirmWrite => self.confirm_write(),
            Action::AnswerConflict { at, answer } => self.answer_conflict(at, answer),
            Action::CancelWrite => {
                if matches!(self.dialog, Some(Dialog::ConfirmWrite(_))) {
                    self.dialog = None;
                }
            }
            Action::MoveSelection {
                tab,
                id,
                rows,
                cols,
            } => {
                self.close_editor(tab, id, true);
                if let Some(object) = self.object_tab_mut(tab, id) {
                    object.edits.why = None;
                    object.focus_field = None;
                }
                if let Some(sql) = self.sql_tab_mut(tab, id) {
                    let (height, width) = sql.dims();
                    sql.selection = (height > 0 && width > 0).then(|| match sql.selection {
                        None => CellPos { row: 0, col: 0 },
                        Some(cell) => CellPos {
                            row: step(cell.row, rows, height),
                            col: step(cell.col, cols, width),
                        },
                    });
                } else if let Some(object) = self.object_tab_mut(tab, id) {
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
                // An editor the panel draws goes with it, its text kept:
                // closed, the panel would leave it open where nothing
                // shows it. Of every table's tab: the panel is the
                // workspace's, and an editor waits in a tab behind another.
                let editing: Vec<TabId> = self
                    .workspace(tab)
                    .filter(|workspace| workspace.row_panel)
                    .map(|workspace| {
                        workspace
                            .object_tabs()
                            .filter(|object| {
                                let editor = object.edits.editor.as_ref();
                                editor.is_some_and(|editor| editor.place == EditorPlace::Panel)
                            })
                            .map(|object| object.id)
                            .collect()
                    })
                    .unwrap_or_default();
                for id in editing {
                    self.close_editor(tab, id, true);
                }
                if let Some(workspace) = self.workspace_mut(tab) {
                    // A field that was to get the keyboard back is none of
                    // the panel that opens next.
                    for object in workspace.object_tabs_mut() {
                        object.focus_field = None;
                    }
                    workspace.row_panel = !workspace.row_panel;
                }
            }
            Action::Refresh(tab) => {
                // A SQL editor has nothing to fetch again: its text runs
                // only when asked.
                if self
                    .workspace(tab)
                    .is_some_and(|workspace| workspace.active_sql_tab().is_some())
                {
                    return;
                }
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
                        // Over an error box a refresh is a retry: what the
                        // error replaced is not what comes back if it is
                        // given up.
                        if let Some(object) = self.object_tab_mut(tab, id) {
                            if object.rows.shown_error().is_some() {
                                object.drop_page();
                            }
                            if object.structure.shown_error().is_some() {
                                object.structure.value = None;
                            }
                        }
                        self.fetch_rows(tab, id);
                        if described {
                            self.describe(tab, id);
                        }
                    }
                    None => self.refresh_tree(tab),
                }
            }
            Action::CancelQuery(tab) => {
                // What the tab shows a spinner for: the active tab's loads
                // or run, or the tree's when no tab is open.
                if let Some(workspace) = self.workspace(tab) {
                    let session = workspace.session;
                    let active = workspace.active_tab.and_then(|id| workspace.tab(id));
                    let pending: Vec<RequestId> = match active {
                        Some(active) => active.pending(),
                        None => std::iter::once(workspace.tree.schemas.pending)
                            .chain(workspace.tree.nodes.values().map(|n| n.objects.pending))
                            .chain(std::iter::once(workspace.databases.pending))
                            .flatten()
                            .collect(),
                    };
                    self.cancel(session, pending);
                }
            }
            Action::NewSqlTab(tab) => self.new_sql_tab(tab),
            Action::RunSql { tab, sql_tab, all } => {
                if let Some(sql) = self.sql_tab_mut(tab, sql_tab) {
                    sql.completion = None;
                    sql.completion_wanted = None;
                }
                self.run_sql(tab, sql_tab, all);
            }
            Action::RunSqlAgain { tab, sql_tab } => self.run_sql_again(tab, sql_tab),
            Action::SqlTyped { tab, sql_tab } => {
                if let Some(sql) = self.sql_tab_mut(tab, sql_tab) {
                    // A list asked for by hand in the same frame stays so.
                    if sql.completion_wanted.is_none() {
                        sql.completion_wanted = Some(Wanted::Typed);
                    }
                }
            }
            Action::OpenCompletion { tab, sql_tab } => {
                if let Some(sql) = self.sql_tab_mut(tab, sql_tab) {
                    sql.completion_wanted = Some(Wanted::Manual);
                }
            }
            Action::MoveCompletion { tab, sql_tab, step } => {
                if let Some(list) = self
                    .sql_tab_mut(tab, sql_tab)
                    .and_then(|sql| sql.completion.as_mut())
                {
                    list.move_by(step);
                }
            }
            Action::AcceptCompletion { tab, sql_tab, row } => {
                if let Some(list) = self
                    .sql_tab_mut(tab, sql_tab)
                    .and_then(|sql| sql.completion.as_mut())
                {
                    // The editor's view inserts it on its next draw.
                    list.accept(row);
                }
            }
            Action::CloseCompletion { tab, sql_tab } => {
                if let Some(sql) = self.sql_tab_mut(tab, sql_tab) {
                    sql.completion = None;
                    sql.completion_wanted = None;
                }
            }
            Action::FormatSql { tab, sql_tab } => {
                if let Some(sql) = self.sql_tab_mut(tab, sql_tab) {
                    sql.format = true;
                    // The keys go back to the editor after a click on the
                    // toolbar's button.
                    sql.focus_editor = true;
                }
            }
            Action::SetSqlLimit {
                tab,
                sql_tab,
                limit,
            } => {
                let limit = Settings::valid_sql_limit(limit);
                if let Some(sql) = self.sql_tab_mut(tab, sql_tab) {
                    sql.limit = limit;
                }
                self.change_settings(|settings| settings.sql_limit = limit);
            }
            Action::SetSqlTimeout { tab, sql_tab, secs } => {
                let secs = Settings::valid_sql_timeout(secs);
                if let Some(sql) = self.sql_tab_mut(tab, sql_tab) {
                    sql.timeout = Settings::timeout_of(secs);
                }
                self.change_settings(|settings| settings.sql_timeout_secs = secs);
            }
            Action::SetSqlMode { tab, sql_tab, mode } => {
                self.set_sql_mode(tab, sql_tab, Some(mode))
            }
            Action::ToggleSqlMode { tab, sql_tab } => self.set_sql_mode(tab, sql_tab, None),
            Action::SetResultPane { tab, sql_tab, pane } => {
                if let Some(sql) = self.sql_tab_mut(tab, sql_tab) {
                    sql.pane = pane;
                }
            }
            Action::SetSqlSplit {
                tab,
                sql_tab,
                split,
            } => {
                if let Some(sql) = self.sql_tab_mut(tab, sql_tab) {
                    sql.set_split(split);
                }
            }
            Action::SqlEditorFocused { tab, sql_tab } => {
                if let Some(workspace) = self.workspace_mut(tab)
                    && workspace.sql_tab(sql_tab).is_some()
                {
                    workspace.pane = Pane::Grid;
                }
            }
            Action::CountRows { tab, object_tab } => self.count_rows(tab, object_tab),
            Action::ShowHelp => {
                if self.dialog.is_none() {
                    self.dialog = Some(Dialog::Help);
                }
            }
            Action::ShowAbout => {
                // The shortcuts dialog offers it, and gives way to it.
                if matches!(self.dialog, None | Some(Dialog::Help)) {
                    self.dialog = Some(Dialog::About);
                }
            }
            Action::ShowSettings => {
                // The shortcuts dialog offers it, and gives way to it. Asked
                // for while it is open, it stays as it is.
                if matches!(self.dialog, None | Some(Dialog::Help)) {
                    self.dialog = Some(Dialog::Settings(Box::default()));
                }
            }
            Action::MoveSettingsRow(by) => {
                if let Some(Dialog::Settings(dialog)) = &mut self.dialog {
                    let last = crate::settings::OptionId::ALL.len() - 1;
                    dialog.row = dialog.row.saturating_add_signed(by).min(last);
                }
            }
            Action::SelectSettingsRow(row) => {
                if let Some(Dialog::Settings(dialog)) = &mut self.dialog
                    && row < crate::settings::OptionId::ALL.len()
                {
                    dialog.row = row;
                }
            }
            Action::SetOption(value) => self.change_settings(|settings| value.set(settings)),
            Action::EditSettingsFile => {
                let text = self.offer_settings_text();
                self.backend.send(Command::EditSettingsFile {
                    path: self.dirs.settings_file(),
                    text,
                });
            }
            // As for the editor: the backend writes the text when no file is
            // there, and that write is the app's own, to be known when it
            // comes back.
            Action::RevealSettingsFile => {
                let text = self.offer_settings_text();
                self.backend.send(Command::RevealSettingsFile {
                    path: self.dirs.settings_file(),
                    text,
                });
            }
            // The app's own text, whatever the file holds: a line the app
            // ignores is not a setting to hand on.
            Action::ExportSettings => self.backend.save_bytes(
                "Export settings",
                "tabletist-settings.toml".to_owned(),
                self.settings.to_toml().into_bytes(),
            ),
            Action::ResetSettings => {
                if let Some(Dialog::Settings(dialog)) = &mut self.dialog {
                    dialog.resetting = true;
                }
            }
            Action::ConfirmResetSettings(reset) => {
                // An answer to a question that was asked.
                let asked = match &mut self.dialog {
                    Some(Dialog::Settings(dialog)) => std::mem::take(&mut dialog.resetting),
                    _ => false,
                };
                if asked && reset {
                    // The options the window shows. A key it has no control
                    // for is not the window's to change.
                    self.change_settings(|settings| {
                        for option in crate::settings::OptionId::ALL {
                            option.default_value().set(settings);
                        }
                    });
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
                    // A retry answers an error: no older page is on screen
                    // for a cancelled one to go back to.
                    if let Some(object) = self.object_tab_mut(tab, object_tab) {
                        object.drop_page();
                    }
                    self.fetch_rows(tab, object_tab);
                }
            }
            Action::RetryStructure { tab, object_tab } => {
                // As a retry of the rows: the structure under the error goes.
                if let Some(object) = self.object_tab_mut(tab, object_tab) {
                    object.structure.value = None;
                }
                self.describe(tab, object_tab);
            }
            Action::SetDriver(driver) => {
                if let Some(Dialog::Connection(form)) = &mut self.dialog {
                    // A port left at the old driver's default follows the driver.
                    let old_default = form.driver.default_port().to_string();
                    form.driver = driver;
                    match driver {
                        Driver::Sqlite => {}
                        Driver::Postgres | Driver::MySql => {
                            if form.port.trim().is_empty() || form.port.trim() == old_default {
                                form.port = driver.default_port().to_string();
                            }
                            if form.password_mode == PasswordMode::None {
                                form.password_mode = PasswordMode::Keyring;
                            }
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
                    workspace.catalog_changed();
                    // The other database has other objects, but a SQL
                    // editor's text is the user's work: it stays.
                    workspace.tabs.retain(|open| matches!(open, Tab::Sql(_)));
                    workspace.active_tab = workspace
                        .active_tab
                        .filter(|id| workspace.tab(*id).is_some())
                        .or_else(|| workspace.tabs.first().map(Tab::id));
                }
                // Which also forgets what the editors asked the old
                // session for.
                self.reconnect(tab);
            }
            Action::Backend(event) => self.apply_event(event),
            Action::NewConnection => {
                self.dialog = Some(Dialog::Connection(Box::default()));
                self.list_ssh_hosts();
            }
            Action::EditConnection(id) => {
                if let Some(saved) = self.connections.get(&id) {
                    self.dialog = Some(Dialog::Connection(Box::new(ConnectionForm::from_saved(
                        saved,
                    ))));
                    self.list_ssh_hosts();
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
            Action::CloseDialog => {
                // Any dialog goes. The conflict question leaves its line
                // for the rows it did not get an answer for.
                if let Some(Dialog::Conflict(prompt)) = self.dialog.take() {
                    self.conflict_unanswered(&prompt);
                }
            }
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
            Action::PickCaFile => {
                let request = RequestId(self.next_id());
                if let Some(Dialog::Connection(form)) = &mut self.dialog {
                    form.pick_request = Some(request);
                    form.pick_target = PickTarget::CaFile;
                    self.backend.pick_ca_file(request);
                }
            }
            Action::PickSshHost(alias) => {
                if let Some(Dialog::Connection(form)) = &mut self.dialog {
                    form.pick_ssh_host(&alias);
                }
            }
            Action::ApplyUrl => {
                if let Some(Dialog::Connection(form)) = &mut self.dialog
                    && let Err(message) = apply_url(form)
                {
                    form.message = Some(message);
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
                if !use_typed_url(form) {
                    return;
                }
                let spec = match form.to_spec() {
                    Ok(spec) => spec,
                    Err(message) => {
                        form.message = Some(message);
                        // The message names fields: show them.
                        form.url_mode = false;
                        return;
                    }
                };
                form.message = None;
                form.test = TestState::Running(request);
                // The clock starts when the Test is sent, not while a saved
                // secret is read from the keyring.
                form.test_started = None;
                form.test_took = None;
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
                    form.test_started = Some(std::time::Instant::now());
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

    /// Records the SQL editors of `tab`'s workspace as closed, before the
    /// workspace goes.
    fn close_editors(&mut self, tab: ConnTabId) {
        let editors: Vec<_> = self
            .workspace(tab)
            .into_iter()
            .flat_map(|workspace| workspace.sql_tabs())
            .map(|sql| (tab, sql.id))
            .collect();
        self.closed_editors.extend(editors);
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
        self.close_editors(id);
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
        // Where the option puts it; the grid's link switches it from there.
        let full_precision = self.settings.timestamps == crate::settings::Timestamps::Full;
        let Some(entry) = self.tabs.iter_mut().find(|t| t.id == tab) else {
            return;
        };
        let mut workspace = Workspace::new(session, request, saved, typed);
        workspace.full_precision = full_precision;
        entry.content = ConnTabContent::Workspace(Box::new(workspace));
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
        // The saved connection's box as it stands now: a session's access
        // is fixed when it connects, so a reconnect picks up a change. Only
        // the box is read again. The tab's name, environment and server
        // are its own from when it opened, so the default is its own
        // environment's: a saved entry relabelled since does not make a
        // tab still drawn as production writable. A connection deleted
        // since keeps what the tab has.
        let boxed = self
            .workspace(tab)
            .and_then(|workspace| self.connections.get(&workspace.conn_id))
            .map(|saved| saved.read_only);
        let Some(workspace) = self.workspace_mut(tab) else {
            return;
        };
        let SessionStatus::Connecting { request } = workspace.status else {
            return;
        };
        if let Some(read_only) = boxed {
            workspace.access = crate::connections::access(read_only, workspace.environment);
        }
        workspace.secrets = secrets.clone();
        workspace.connect_started = Some(std::time::Instant::now());
        let (session, spec, access) = (workspace.session, workspace.spec.clone(), workspace.access);
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
            access,
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
            workspace.connect_started = None;
            workspace.forget_session_requests();
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
        workspace.connect_started = None;
        workspace.forget_session_requests();
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
                access,
            } => {
                let adopted = self.tab_for_session(session).and_then(|tab| {
                    let workspace = self.workspace_mut(tab)?;
                    match workspace.status {
                        SessionStatus::Connecting { request: waiting } if waiting == request => {
                            workspace.status = SessionStatus::Connected;
                            workspace.driver = driver;
                            workspace.encrypted = encrypted;
                            // What the session is, whatever was asked.
                            workspace.access = access;
                            workspace.connected_at = Some(crate::util::now_secs());
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
                        stage:
                            SshStage::HostKeyUnknown {
                                host,
                                port,
                                fingerprint,
                            },
                        ..
                    } => Some((host.clone(), *port, fingerprint.clone())),
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
                    let elapsed = form.test_started.take().map(|started| started.elapsed());
                    form.test_took = if result.is_ok() { elapsed } else { None };
                    // The key belongs to the SSH host the test reached, which
                    // the error names (a Host alias's HostName), not whatever
                    // the fields say now.
                    form.test = match result {
                        Ok(()) => TestState::Passed,
                        Err(Error::Ssh {
                            stage:
                                SshStage::HostKeyUnknown {
                                    host,
                                    port,
                                    fingerprint,
                                },
                            ..
                        }) if self.host_keys_error.is_none() => TestState::Untrusted {
                            host,
                            port,
                            fingerprint,
                        },
                        Err(error) => TestState::Failed(error.to_string()),
                    };
                }
            }
            Event::SshHosts { request, hosts } => {
                if let Some(Dialog::Connection(form)) = &mut self.dialog
                    && form.ssh_hosts_request == Some(request)
                {
                    form.ssh_hosts_request = None;
                    form.ssh_hosts = hosts;
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
                        (Some(path), PickTarget::CaFile) => {
                            form.ca_file = path.display().to_string();
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
                workspace.catalog_changed();
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
                    && node.objects.finish(request, result)
                {
                    workspace.catalog_changed();
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
                    .object_tabs_mut()
                    .find(|o| o.rows.pending == Some(request))
                else {
                    return;
                };
                let id = object.id;
                let failed = result.is_err();
                object.rows.finish(request, result);
                // The marks of the last save and the note of a locked cell
                // were about the page this one replaces. Nothing is pending
                // here: a tab that holds edits is never fetched again (the
                // guard at the top of `apply` is what makes that so).
                if failed {
                    // The page on screen is still the one a save found
                    // rows gone from.
                    object.edits.discard();
                } else {
                    object.edits = crate::edit::Edits::default();
                }
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
                // A question about this tab's rows was about their pending
                // cells, which went with the set just now, and names rows
                // by their place in the page: nothing is left to ask.
                if matches!(
                    &self.dialog,
                    Some(Dialog::Conflict(prompt)) if prompt.tab == tab && prompt.id == id
                ) {
                    self.dialog = None;
                }
            }
            Event::Structure {
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
                let waiting = workspace
                    .object_tabs_mut()
                    .find(|o| o.structure.pending == Some(request));
                if let Some(object) = waiting {
                    // A table tab's structure tells the completion list
                    // its columns too.
                    let columns = result.as_ref().ok();
                    let columns = columns.map(|structure| structure.columns.clone());
                    let named = object.object.clone();
                    object.structure.finish(request, result);
                    // The review was made of the structure before this
                    // one, its key and its types: it is made again.
                    object.edits.review = None;
                    if let Some(columns) = columns {
                        let kept = workspace.columns.entry(named).or_default();
                        // One being fetched gets its own answer.
                        if !kept.is_loading() {
                            kept.value = Some(columns);
                            kept.error = None;
                        }
                        workspace.catalog_changed();
                    }
                } else if let Some(kept) = workspace
                    .columns
                    .values_mut()
                    .find(|kept| kept.pending == Some(request))
                {
                    // Asked for by a completion list.
                    kept.finish(request, result.map(|structure| structure.columns));
                    workspace.catalog_changed();
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
                                    form.test_started = Some(std::time::Instant::now());
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
            Event::SettingsWatch { live } => self.settings_file.live = live,
            Event::SettingsFile { text, own } => {
                // What the app holds already: its own write coming back,
                // or a save that changed nothing.
                if text == self.settings_file.text {
                    return;
                }
                // A write of its own that is not what it holds. An older
                // one was read between two of its writes: the newest is
                // still to come, and applying this would undo the change
                // made since. The newest itself landed over a change from
                // outside that was applied in between: the disk has it. So
                // does a text handed over with the file to be opened, which
                // the backend wrote because the file was gone by then.
                let file = &self.settings_file;
                let newest =
                    file.saved.as_deref() == Some(text.as_str()) || file.offered.contains(&text);
                if own && !newest {
                    return;
                }
                let loaded = Settings::from_toml(&text);
                loaded.warn_invalid(&self.dirs.settings_file());
                let live = self.settings_file.live;
                let saved = self.settings_file.saved.take();
                let offered = std::mem::take(&mut self.settings_file.offered);
                let (settings, file) = loaded.into_parts();
                // The file as its writer left it: not written back, so a
                // line that was ignored stays where they can see it.
                self.settings_file = SettingsFile {
                    live,
                    saved,
                    offered,
                    ..file
                };
                self.apply_settings(settings);
            }
            Event::SettingsFileOpened { with, result } => {
                if let Err(error) = result {
                    let path = self.dirs.settings_file();
                    self.notice = Some(match with {
                        Opened::Editor => {
                            format!("Could not open {} in the editor: {error}.", path.display())
                        }
                        Opened::Folder => format!(
                            "Could not show {} in the file manager: {error}.",
                            path.display()
                        ),
                    });
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
                        .object_tabs_mut()
                        .find(|o| o.count.pending == Some(request))
                {
                    object.count.finish(request, result);
                }
            }
            Event::ServerVersion {
                session,
                request,
                result,
            } => {
                if let Some(tab) = self.tab_for_session(session)
                    && let Some(workspace) = self.workspace_mut(tab)
                {
                    workspace.server_version.finish(request, result);
                }
            }
            Event::Written {
                session,
                request,
                result,
            } => self.written(session, request, result),
            Event::SqlRan {
                session,
                request,
                result,
                cancel,
            } => {
                // An answer for a closed tab or a replaced run finds no
                // editor waiting for it.
                let Some(sql) = self
                    .tab_for_session(session)
                    .and_then(|tab| self.workspace_mut(tab))
                    .and_then(|workspace| {
                        workspace
                            .sql_tabs_mut()
                            .find(|sql| sql.run.pending == Some(request))
                    })
                else {
                    return;
                };
                let messages = opens_messages(&result, cancel);
                if sql.finish_run(request, result, cancel) {
                    sql.selection = None;
                    sql.pane = if messages {
                        ResultPane::Messages
                    } else {
                        ResultPane::Results
                    };
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
                    host: host.to_owned(),
                    port,
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
                    host: host.to_owned(),
                    port,
                    fingerprint: fingerprint.to_owned(),
                },
                message: "the host key changed since you trusted it, which can mean someone \
                          is intercepting the connection"
                    .into(),
            }),
            _ => None,
        }
    }

    /// Asks the backend for ~/.ssh/config's Host aliases for the open
    /// connection dialog.
    fn list_ssh_hosts(&mut self) {
        let request = RequestId(self.next_id());
        if let Some(Dialog::Connection(form)) = &mut self.dialog {
            form.ssh_hosts_request = Some(request);
            self.backend.list_ssh_hosts(request);
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

    /// Sends the settings to the backend to be written. Only
    /// [`App::change_settings`] and the start that read the old
    /// settings.json call it.
    fn save_settings(&mut self) {
        self.settings_file.saved = Some(self.settings.to_toml());
        // The save is newer than any text handed over with the file: where
        // one of those was written, this lands over it.
        self.settings_file.offered.clear();
        self.backend.send(Command::Save {
            path: self.dirs.settings_file(),
            file: StateFile::Settings(self.settings.clone()),
        });
    }

    /// The text the app holds of its settings file, to hand over with the
    /// file to be opened. The backend writes it when no file is there: a
    /// write of the app's own, which is noted here to be known when it
    /// comes back.
    fn offer_settings_text(&mut self) -> String {
        let text = self.settings_file.text.clone();
        if !self.settings_file.offered.contains(&text) {
            self.settings_file.offered.push(text.clone());
        }
        text
    }

    /// Asks the backend to watch the settings file, so an edit made outside
    /// the app reaches it (`Event::SettingsFile`).
    fn watch_settings(&mut self) {
        self.backend.send(Command::WatchSettings {
            path: self.dirs.settings_file(),
        });
    }

    /// A change made in the app (a menu, the Settings window): applied, and
    /// written as the canonical text, which is the file from then on. The
    /// lines the reader had ignored are gone with the text they were in.
    /// A change that changes nothing writes nothing.
    pub fn change_settings(&mut self, change: impl FnOnce(&mut Settings)) {
        let mut changed = self.settings.clone();
        change(&mut changed);
        let loaded = Loaded::of(changed, Source::Toml);
        if loaded.settings == self.settings {
            return;
        }
        let live = self.settings_file.live;
        let saved = self.settings_file.saved.take();
        let (settings, file) = loaded.into_parts();
        self.settings_file = SettingsFile {
            live,
            saved,
            ..file
        };
        self.apply_settings(settings);
        self.save_settings();
    }

    /// Replaces the settings and does what the ones that changed ask for.
    /// Every change comes through here, from the app or from the file, and
    /// nothing is written here: saving is [`App::change_settings`]'s.
    pub fn apply_settings(&mut self, new: Settings) {
        let old = std::mem::replace(&mut self.settings, new);
        if old.timestamps != self.settings.timestamps {
            let full = self.settings.timestamps == crate::settings::Timestamps::Full;
            for tab in &mut self.tabs {
                if let ConnTabContent::Workspace(workspace) = &mut tab.content {
                    workspace.full_precision = full;
                }
            }
        }
        if old.page_size != self.settings.page_size {
            self.resize_pages();
        }
        if old.show_system_schemas != self.settings.show_system_schemas {
            // A completion list offers the schemas that are shown, and an
            // open one is worked out again only when its script, its cursor
            // or its catalog changed: its catalog did.
            for tab in &mut self.tabs {
                if let ConnTabContent::Workspace(workspace) = &mut tab.content {
                    workspace.catalog_changed();
                }
            }
        }
        if old.custom_theme != self.settings.custom_theme {
            // Reading a theme needs the window: `logic` has it.
            self.theme_changed = true;
        }
    }

    /// Fetches again, at the settings' page size, every table that shows a
    /// page or waits for one on a session that can answer. `fetch_rows`
    /// drops the page of the old size as it takes the new one. The others,
    /// and a table that holds edits, take the size, and lose their page
    /// the same way, when they next fetch.
    fn resize_pages(&mut self) {
        let size = self.settings.page_size;
        let mut again = Vec::new();
        for tab in &self.tabs {
            let ConnTabContent::Workspace(workspace) = &tab.content else {
                continue;
            };
            if !matches!(workspace.status, SessionStatus::Connected) {
                continue;
            }
            again.extend(
                workspace
                    .object_tabs()
                    .filter(|object| object.query.limit != size)
                    .filter(|object| object.page().is_some() || object.rows.pending.is_some())
                    // A page with pending changes stays: the tab takes the
                    // size at its next fetch.
                    .filter(|object| !object.edits.holds())
                    .map(|object| (tab.id, object.id)),
            );
        }
        for (tab, id) in again {
            self.fetch_rows(tab, id);
        }
    }

    fn save_dialog(&mut self, connect: bool) {
        let Some(Dialog::Connection(form)) = &mut self.dialog else {
            return;
        };
        if !use_typed_url(form) {
            return;
        }
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
                // The message names fields: show them.
                form.url_mode = false;
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
                self.apply(Action::ShowConnections);
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
        if let Some(workspace) = self.workspace(tab) {
            match workspace.driver {
                // The one driver with other databases to switch to.
                Driver::Postgres => {
                    let session = workspace.session;
                    let request = RequestId(self.next_id());
                    if let Some(workspace) = self.workspace_mut(tab) {
                        workspace.databases.start(request);
                    }
                    self.backend
                        .send(Command::ListDatabases { session, request });
                }
                Driver::MySql | Driver::Sqlite => {}
            }
        }
        // The footer of a SQL editor names the server; ask again when the
        // session is new (see `Workspace::forget_session_requests`).
        if let Some(workspace) = self.workspace(tab)
            && workspace.sql_tabs().next().is_some()
            && workspace.server_version.needs_load()
        {
            let session = workspace.session;
            let request = RequestId(self.next_id());
            if let Some(workspace) = self.workspace_mut(tab) {
                workspace.server_version.start(request);
            }
            self.backend
                .send(Command::ServerVersion { session, request });
        }
        self.refresh_tree(tab);
        let Some(workspace) = self.workspace_mut(tab) else {
            return;
        };
        let active = workspace.active_tab;
        let pending = workspace.pending_open.take();
        // A count queued on the old session will never answer, and one the
        // lost connection failed is worth another try.
        for object in workspace.object_tabs_mut() {
            if object.count.is_loading() || lost(&object.count) {
                // The old session is closed, so its count never runs.
                let _ = object.reset_count();
            }
        }
        let stale: Vec<(TabId, bool, bool)> = workspace
            .object_tabs()
            .map(|object| {
                // A tab that holds edits keeps its page and its structure
                // as the edits were made on them.
                if object.edits.holds() {
                    return (object.id, false, false);
                }
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

/// What `workspace` knows, as a completion list reads it. `schemas` are
/// the ones its sidebar shows.
fn catalog_of<'a>(
    workspace: &'a Workspace,
    schemas: &'a [String],
) -> crate::completion::Catalog<'a> {
    crate::completion::Catalog {
        dialect: workspace.driver.dialect(),
        tree: &workspace.tree,
        bare: workspace.bare_schema(),
        schemas,
        columns: &workspace.columns,
    }
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
        // An open completion list says it waits for them.
        workspace.catalog_changed();
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
        // The names of a schema the sidebar does not show unfolded (one
        // only a completion list loaded, or one that was folded) are not
        // loaded again here: they are forgotten, and load again when a
        // list or the sidebar needs them. Its node stays: the prefix
        // groups unfolded in it are the user's. The columns a list
        // fetched go too.
        if let Some(workspace) = self.workspace_mut(tab) {
            for node in workspace.tree.nodes.values_mut() {
                if !node.expanded && !node.objects.is_loading() {
                    node.objects = Fetch::default();
                }
            }
            workspace.columns.clear();
            workspace.catalog_changed();
        }
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

    fn object_tab_mut(&mut self, tab: ConnTabId, id: TabId) -> Option<&mut ObjectTab> {
        self.workspace_mut(tab)?.object_tab_mut(id)
    }

    fn sql_tab_mut(&mut self, tab: ConnTabId, id: TabId) -> Option<&mut SqlTab> {
        self.workspace_mut(tab)?.sql_tab_mut(id)
    }

    /// The active connection tab and the tab its workspace shows, of
    /// either kind.
    pub fn active_workspace_tab(&self) -> Option<(ConnTabId, TabId)> {
        let tab = self.active_tab_id();
        Some((tab, self.workspace(tab)?.active_tab?))
    }

    /// The active connection tab and the SQL editor its workspace shows:
    /// none while it shows an object tab.
    pub fn active_sql(&self) -> Option<(ConnTabId, TabId)> {
        let tab = self.active_tab_id();
        Some((tab, self.workspace(tab)?.active_sql_tab()?.id))
    }

    /// Opens an empty SQL editor after the workspace's other tabs, with the
    /// limit and timeout of the settings, and shows it.
    fn new_sql_tab(&mut self, tab: ConnTabId) {
        let id = TabId(self.next_id());
        let request = RequestId(self.next_id());
        let (limit, timeout) = (self.settings.sql_limit, self.settings.sql_timeout());
        let Some(workspace) = self.workspace_mut(tab) else {
            return;
        };
        workspace.push_sql_tab(id, limit, timeout);
        workspace.active_tab = Some(id);
        workspace.pane = Pane::Grid;
        // The editor's footer names the server. A session still connecting
        // is asked once it has (see `after_connect`).
        if matches!(workspace.status, SessionStatus::Connected)
            && workspace.server_version.needs_load()
        {
            workspace.server_version.start(request);
            let session = workspace.session;
            self.backend
                .send(Command::ServerVersion { session, request });
        }
    }

    /// Asks the backend for the names a completion list needs and the
    /// workspace never loaded. Only on a connected session; a load that
    /// failed is not asked for again until the tree is refreshed, which
    /// forgets the columns too.
    fn send_needs(&mut self, tab: ConnTabId, needs: &[Need]) {
        for need in needs {
            let Some(workspace) = self.workspace(tab) else {
                return;
            };
            if !matches!(workspace.status, SessionStatus::Connected) {
                return;
            }
            match need {
                Need::Objects(schema) => {
                    let node = workspace.tree.nodes.get(schema);
                    if node.is_none_or(|node| node.objects.needs_load()) {
                        self.load_objects(tab, schema);
                    }
                }
                Need::Columns(object) => {
                    let kept = workspace.columns.get(object);
                    if kept.is_none_or(Fetch::needs_load) {
                        self.load_columns(tab, object.clone());
                    }
                }
            }
        }
    }

    /// Asks for `object`'s structure, for its columns in the completion
    /// list. Nothing waits for it but the workspace's `columns`.
    fn load_columns(&mut self, tab: ConnTabId, object: ObjectRef) {
        let request = RequestId(self.next_id());
        let Some(workspace) = self.workspace_mut(tab) else {
            return;
        };
        let kept = workspace.columns.entry(object.clone()).or_default();
        kept.start(request);
        // An open completion list says it waits for them.
        workspace.catalog_changed();
        let session = workspace.session;
        self.backend.send(Command::Describe {
            session,
            request,
            object,
        });
    }

    /// Works out the completion list of the SQL editor on screen: opens
    /// the one that was asked for, recomputes an open one whose script,
    /// cursor or catalog changed, and closes one with nothing left to
    /// offer. The names its site reads and the workspace never loaded are
    /// asked for, and a list with no rows stays open while they are on
    /// their way. Lists of editors that are not on screen are dropped. A
    /// list waiting for the view to insert its row is left as it is.
    /// Returns whether a list changed, so the frame is drawn again.
    ///
    /// Called from `frame_ui`, which has the egui context: the script's
    /// tokens come from the cache the editor's layouter fills, so a script
    /// is tokenized once per change.
    pub fn refresh_completion(&mut self, ctx: &egui::Context) -> bool {
        let active = self.active_sql();
        let mut changed = false;
        for conn in &mut self.tabs {
            let ConnTabContent::Workspace(workspace) = &mut conn.content else {
                continue;
            };
            for sql in workspace.sql_tabs_mut() {
                if active != Some((conn.id, sql.id)) {
                    changed |= sql.completion.take().is_some();
                    sql.completion_wanted = None;
                }
            }
        }
        let Some((tab, id)) = active else {
            return changed;
        };
        let palette = self.palette;
        let Some(workspace) = self.workspace_mut(tab) else {
            return changed;
        };
        let dialect = workspace.driver.dialect();
        let generation = workspace.catalog_generation;
        let Some(sql) = workspace.sql_tab_mut(id) else {
            return changed;
        };
        // The view is about to insert the highlighted row: the rows must
        // not move under it. What was asked for meanwhile waits.
        if sql.completion.as_ref().is_some_and(|list| list.accept) {
            return changed;
        }
        let wanted = sql.completion_wanted.take();
        let was_open = sql.completion.is_some();
        if wanted.is_none() && !was_open {
            return changed;
        }
        let of = (TextPrint::of(&sql.text), sql.cursor, generation);
        if wanted.is_none() && sql.completion.as_ref().is_some_and(|list| list.is_of(of)) {
            return changed;
        }
        let editor = crate::ui::sql_text::editor_id(tab, id);
        let parsed = crate::ui::sql_text::parsed(ctx, editor, dialect, &palette, &sql.text);
        let Some(site) = tabletist_db::complete::site(&parsed.tokens, &sql.text, sql.cursor) else {
            sql.completion = None;
            return changed | was_open;
        };
        // A cursor that is stale or inside a character has no word.
        let Some(typed) = sql.text.get(site.word.start..sql.cursor).map(str::to_owned) else {
            sql.completion = None;
            return changed | was_open;
        };
        // An open list goes on only for the word it was opened on.
        let going = sql
            .completion
            .as_ref()
            .is_some_and(|list| list.is_on(&site, of.0));
        let manual = match wanted {
            Some(Wanted::Manual) => true,
            _ if going => sql.completion.as_ref().is_some_and(|list| list.manual),
            Some(Wanted::Typed) => false,
            None => {
                // The cursor left the word.
                sql.completion = None;
                return true;
            }
        };
        // Opening, by typing: a word of two characters or more, or right
        // after a dot, and not where a new name goes. Told before any row
        // is looked for.
        if !going && !crate::completion::may_open(&site, &typed, manual) {
            sql.completion = None;
            return changed | was_open;
        }
        let (print, cursor) = (of.0, of.1);
        // Asking is part of opening: the names the site reads are asked
        // for before the list is worked out, whatever it holds so far.
        let show_system = self.settings.show_system_schemas;
        let Some(workspace) = self.workspace(tab) else {
            return changed;
        };
        let schemas = workspace
            .tree
            .visible_schemas(workspace.driver, show_system);
        let needs = crate::completion::needs(&site, &catalog_of(workspace, &schemas));
        self.send_needs(tab, &needs);
        let Some(workspace) = self.workspace(tab) else {
            return changed;
        };
        let catalog = catalog_of(workspace, &schemas);
        let listed = crate::completion::list(&site, &typed, manual, &catalog);
        let loading = needs.iter().any(|need| workspace.is_loading(need));
        // The generation after asking: asking changed it.
        let of = (print, cursor, workspace.catalog_generation);
        let Some(sql) = self.sql_tab_mut(tab, id) else {
            return changed;
        };
        // A list with no rows stays open only while names are on their way.
        let empty = listed.candidates.is_empty() && !loading;
        if going {
            match sql.completion.as_mut() {
                Some(list) if !empty => {
                    list.manual = manual;
                    list.relist(of, site, typed, listed, loading);
                }
                _ => sql.completion = None,
            }
            return true;
        }
        // Opening: not with nothing to offer, and by typing not when the
        // only row is what is already typed (its names were still asked
        // for). Asked for by hand, that one row shows.
        let opens = !empty && (manual || !listed.only_repeats(&typed));
        sql.completion = opens.then(|| Completion::new(manual, of, site, typed, listed, loading));
        changed | was_open | sql.completion.is_some()
    }

    /// Sets how an editor's runs end: to `mode`, or to the other one. Only
    /// where an editor can run read-write: everywhere else the badge and
    /// the key do nothing, and a mode the tab was given before stays as it
    /// is for a session that can write again.
    fn set_sql_mode(&mut self, tab: ConnTabId, id: TabId, mode: Option<RunMode>) {
        let Some(workspace) = self.workspace_mut(tab) else {
            return;
        };
        if workspace.sql_writes().is_err() {
            return;
        }
        if let Some(sql) = workspace.sql_tab_mut(id) {
            sql.mode = mode.unwrap_or_else(|| sql.mode.other());
        }
    }

    /// Runs the statement at the editor's cursor, or every statement. An
    /// editor holding no statement (empty, or only comments) runs nothing.
    /// Nor does one whose session is not connected: the backend would
    /// answer that the connection is closed, and a run that fails as a
    /// whole takes the result before it away. Nor one whose read-write run
    /// is still in flight: a new run would cancel it.
    ///
    /// The run is read-write only when the editor's runs are (see
    /// `Workspace::run_mode`) and one of the statements looks like a
    /// write. Every other run is the read-only one.
    fn run_sql(&mut self, tab: ConnTabId, id: TabId, all: bool) {
        let Some(workspace) = self.workspace(tab) else {
            return;
        };
        if !matches!(workspace.status, SessionStatus::Connected) {
            return;
        }
        let dialect = workspace.driver.dialect();
        let Some(sql) = workspace.sql_tab(id) else {
            return;
        };
        if sql.is_writing() {
            return;
        }
        let mut statements = tabletist_db::sql::statements(dialect, &sql.text);
        if !all {
            statements = tabletist_db::sql::statement_at(&statements, sql.cursor)
                .cloned()
                .into_iter()
                .collect();
        }
        if statements.is_empty() {
            return;
        }
        let mode = script_mode(workspace.run_mode(sql), dialect, &statements);
        self.send_run(tab, id, statements, mode);
    }

    /// The card's "Run in a read-write transaction": sends the statements
    /// of the editor's last run again, to write. That run was read-only,
    /// its statements having looked like reads or the tab having been in
    /// Read-only then, and the database refused one of them.
    ///
    /// The editor's mode is looked at again here: a card left on screen
    /// writes nothing for a tab that was switched back since, or whose
    /// session came back read-only. Nor once the text is no longer the one
    /// that ran: what would be sent is not what the editor shows.
    fn run_sql_again(&mut self, tab: ConnTabId, id: TabId) {
        let Some(workspace) = self.workspace(tab) else {
            return;
        };
        if !matches!(workspace.status, SessionStatus::Connected) {
            return;
        }
        let Some(sql) = workspace.sql_tab(id) else {
            return;
        };
        if workspace.run_mode(sql) != RunMode::ReadWrite || !sql.ran_this_text() {
            return;
        }
        let Some(run) = sql
            .last_run()
            .filter(|run| run.mode == tabletist_db::ScriptMode::ReadOnly)
        else {
            return;
        };
        let statements = run.statements.clone();
        if statements.is_empty() {
            return;
        }
        self.send_run(tab, id, statements, tabletist_db::ScriptMode::Write);
    }

    /// Sends `statements` to the editor's session as its run, in a
    /// transaction of `mode`.
    fn send_run(
        &mut self,
        tab: ConnTabId,
        id: TabId,
        statements: Vec<tabletist_db::sql::Statement>,
        mode: tabletist_db::ScriptMode,
    ) {
        let request = RequestId(self.next_id());
        let Some(workspace) = self.workspace_mut(tab) else {
            return;
        };
        let session = workspace.session;
        let Some(sql) = workspace.sql_tab_mut(id) else {
            return;
        };
        let (limit, timeout) = (sql.limit, sql.timeout);
        let superseded = sql.start_run(request, statements.clone(), mode);
        // The session runs one thing at a time: the run still going must
        // stop before this one can start.
        self.cancel(session, superseded);
        self.backend.send(Command::RunSql {
            session,
            request,
            statements,
            limit,
            timeout,
            mode,
        });
    }

    /// The connection tab and object tab the keyboard acts on: none while
    /// the workspace shows a SQL editor.
    pub fn active_object(&self) -> Option<(ConnTabId, TabId)> {
        let tab = self.active_tab_id();
        let workspace = self.workspace(tab)?;
        Some((tab, workspace.active_object_tab()?.id))
    }

    pub fn open_object(&mut self, tab: ConnTabId, object: ObjectRef, kind: ObjectKind, pin: bool) {
        let page_size = self.settings.page_size;
        let new_id = TabId(self.next_id());
        let Some(workspace) = self.workspace_mut(tab) else {
            return;
        };
        workspace.recent.retain(|(seen, _)| *seen != object);
        workspace.recent.insert(0, (object.clone(), kind));
        workspace.recent.truncate(crate::model::RECENT);
        workspace.tree.reveal(&object);
        let existing = workspace
            .object_tabs_mut()
            .find(|o| o.object == object)
            .map(|existing| {
                existing.pinned |= pin;
                existing.id
            });
        if let Some(id) = existing {
            workspace.active_tab = Some(id);
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
            workspace.tabs.iter().position(Tab::is_preview)
        };
        let session = workspace.session;
        let opened = Tab::Object(Box::new(opened));
        let replaced = match preview {
            Some(index) => Some(std::mem::replace(&mut workspace.tabs[index], opened)),
            None => {
                workspace.tabs.push(opened);
                None
            }
        };
        workspace.active_tab = Some(new_id);
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
        self.open_object(tab, object.clone(), kind, true);
        let Some(id) = self
            .workspace(tab)
            .and_then(|workspace| workspace.active_tab)
        else {
            return;
        };
        // The table was open already and holds edits: filtering it drops
        // its page, so the same follow waits for the user's answer, with
        // the tab's filter bar as it was.
        let holds = self
            .workspace(tab)
            .and_then(|workspace| workspace.object_tab(id))
            .is_some_and(|opened| opened.edits.holds());
        if holds {
            let follow = Action::FollowForeignKey {
                tab,
                object,
                column,
                value,
            };
            self.hold(Held::Action(Box::new(follow)), vec![(tab, id)]);
            return;
        }
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
    fn reset_count(&mut self, tab: ConnTabId, id: TabId) {
        let Some(workspace) = self.workspace_mut(tab) else {
            return;
        };
        let session = workspace.session;
        let running = workspace
            .object_tab_mut(id)
            .and_then(ObjectTab::reset_count);
        self.cancel(session, running);
    }

    /// Loads the object tab's rows at the settings' page size, replacing
    /// any load still pending. A page of another size does not stay on
    /// screen meanwhile, whatever the caller kept.
    pub fn fetch_rows(&mut self, tab: ConnTabId, id: TabId) {
        self.fetch_page(tab, id, self.settings.page_size);
    }

    /// Loads a page of `limit` rows, as [`App::fetch_rows`] does at the
    /// settings' size. Only Previous asks for another: the rows before a
    /// page that begins less than a page from the start.
    fn fetch_page(&mut self, tab: ConnTabId, id: TabId, limit: u32) {
        let request = RequestId(self.next_id());
        let Some(workspace) = self.workspace_mut(tab) else {
            return;
        };
        let session = workspace.session;
        let Some(object) = workspace.object_tab_mut(id) else {
            return;
        };
        // Until here the limit was the size of the page on screen. Next
        // has just moved past that page by it. Previous has moved back by
        // the size it asks for here, so that the page fetched ends where
        // that one begins. From here the limit is the size of the page
        // awaited. A page of another size cannot stay under the new limit:
        // were this fetch cancelled, Next would move past it by the wrong
        // size.
        if object.query.limit != limit {
            object.drop_page();
            object.query.limit = limit;
        }
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
    fn apply_filters(&mut self, tab: ConnTabId, id: TabId) {
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

    pub fn count_rows(&mut self, tab: ConnTabId, id: TabId) {
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

    pub fn describe(&mut self, tab: ConnTabId, id: TabId) {
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
    /// What the grid shows: a pending cell gives its new value, as the row
    /// panel's copy does.
    pub fn copy_text(&self, whole_row: bool) -> Option<String> {
        use tabletist_db::{NewValue, Value};
        let (tab, id) = self.active_object()?;
        let object = self.workspace(tab)?.object_tab(id)?;
        let cell = object.selection?;
        let row = object.page()?.rows.get(cell.row)?;
        let shown = |col: usize, loaded: &Value| match object.edits.cells.get(&(cell.row, col)) {
            Some(pending) => match &pending.new {
                NewValue::Text(text) => Value::Text(text.as_str().into()),
                NewValue::Null => Value::Null,
            },
            None => loaded.clone(),
        };
        Some(if whole_row {
            let row: Vec<Value> = row
                .iter()
                .enumerate()
                .map(|(col, loaded)| shown(col, loaded))
                .collect();
            crate::ui::format::tsv_row(&row)
        } else {
            crate::ui::format::plain_text(&shown(cell.col, row.get(cell.col)?))
        })
    }

    /// Called once the window exists. `follow_desktop` is false in demo mode
    /// and tests, which must not scan the user's themes or Omarchy.
    pub fn attach(&mut self, ctx: &egui::Context, follow_desktop: bool) {
        self.follow_desktop = follow_desktop;
        theme::install(ctx, follow_desktop, &self.look);
        if follow_desktop {
            theme::enable_desktop_themes(&mut self.themes);
            let repaint = ctx.clone();
            self.themes.start(
                self.dirs.themes_dir(),
                self.settings.custom_theme.clone(),
                &fastframe_theme::Waker::new(move || repaint.request_repaint()),
            );
            // Not in tests or the demo, which must not watch the user's
            // directories any more than they scan them.
            self.watch_settings();
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

    /// Work that does not draw: theme changes on disk or in the OS, and a
    /// request to close the window.
    pub fn logic(&mut self, ctx: &egui::Context) {
        // The settings named another theme: where the desktop is followed
        // the catalog reads that file first, as it does at the start.
        let renamed = std::mem::take(&mut self.theme_changed);
        if self.themes.needs_reload() || (renamed && self.follow_desktop) {
            let repaint = ctx.clone();
            self.themes.start(
                self.dirs.themes_dir(),
                self.settings.custom_theme.clone(),
                &fastframe_theme::Waker::new(move || repaint.request_repaint()),
            );
        }
        let scanned = self.themes.poll();
        let system = ctx.system_theme();
        if scanned || renamed || system != self.system_theme {
            self.system_theme = system;
            let palette = self.resolve_palette();
            if palette != self.palette {
                self.palette = palette;
                theme::apply(ctx, &palette, &self.look);
            }
        }
        // A hidden window draws no frame: its close request is answered
        // here, or nothing would hold it back.
        self.hold_close(ctx);
    }

    /// Draws one frame, then applies what the frame asked for.
    pub fn frame_ui(&mut self, ui: &mut egui::Ui) {
        self.poll_backend();
        self.apply_actions();
        // Before anything is drawn: a close request that pending changes
        // hold back is asked about in this frame.
        self.hold_close(ui.ctx());
        // Before the shortcuts take their keys: what the user works with.
        crate::ui::focus::begin_frame(ui.ctx());
        // Before the keys and the view, which read the list: it is the one
        // the last frame's actions left, after anything the backend
        // delivered just now.
        self.refresh_completion(ui.ctx());
        // A dialog takes the keyboard: no shortcut acts behind it, and a
        // first key that waited for its second (`cc`) waits no longer.
        if self.dialog.is_none() {
            crate::ui::keys::handle(self, ui.ctx());
        } else {
            crate::ui::keys::forget_pending(ui.ctx());
        }
        crate::ui::show(self, ui);
        // Over everything drawn: the ring of what has the keyboard.
        crate::ui::focus::paint(ui.ctx(), &self.look, &self.palette);
        crate::ui::keys::after_frame(ui.ctx());
        self.apply_actions();
        // A list that opened or changed shows on the next frame.
        if self.refresh_completion(ui.ctx()) {
            ui.ctx().request_repaint();
        }
        let title = self.window_title();
        if title != self.window_title {
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.window_title = title;
        }
        // Asked every frame until the window goes: a window that is
        // closing has nothing else to do.
        if self.closing {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }
        // After the frame drew them for the last time.
        for (tab, id) in self.closed_editors.drain(..) {
            crate::ui::sql_text::forget(ui.ctx(), tab, id);
            crate::ui::sql_results::forget(ui.ctx(), tab, id);
        }
    }
}

/// A URL being typed comes before the parameters: saving or testing fills
/// the form from it first. False when it does not parse (the message says
/// why), so nothing is saved or tested in its place.
fn use_typed_url(form: &mut ConnectionForm) -> bool {
    if form.url_mode
        && !form.url.trim().is_empty()
        && let Err(message) = apply_url(form)
    {
        form.message = Some(message);
        return false;
    }
    true
}

/// Fills the form from its URL field and leaves the URL mode, or says why
/// the URL cannot be used: then the form is as it was, the URL too.
fn apply_url(form: &mut ConnectionForm) -> Result<(), String> {
    let tabletist_db::ParsedUrl {
        spec,
        secrets,
        names_tls,
        names_ca_file,
        ..
    } = tabletist_db::ParsedUrl::parse(&form.url).map_err(|error| error.to_string())?;
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
        }
    }
    // The field is emptied once it has filled the form: its password is in
    // the masked field now, and a URL left behind would fill the form again
    // on the next save, over anything edited since.
    form.url.clear();
    form.message = None;
    form.url_mode = false;
    Ok(())
}

/// The transaction a run of `statements` is sent in, in an editor whose
/// runs have `mode`: a read-write one only in Read-write, and only when a
/// statement looks like a write. So an editor left in Read-write opens no
/// read-write transaction for its reads, and nothing writes from one in
/// Read-only.
fn script_mode(
    mode: RunMode,
    dialect: tabletist_db::Dialect,
    statements: &[tabletist_db::sql::Statement],
) -> tabletist_db::ScriptMode {
    use tabletist_db::sql::{StatementKind, kind};
    let writes = statements
        .iter()
        .any(|statement| kind(dialect, &statement.text) == StatementKind::Write);
    if mode == RunMode::ReadWrite && writes {
        tabletist_db::ScriptMode::Write
    } else {
        tabletist_db::ScriptMode::ReadOnly
    }
}

/// Whether a finished SQL run opens Messages rather than Results. It does
/// when the run failed, as a whole or in a statement; when its timeout
/// stopped it (nobody asked for that, so the reason must show); and when
/// it was cancelled without a statement returning rows, which leaves
/// Results nothing to show. And when a run sent to write ended as neither
/// of the two a user expects, committed or rolled back whole: part of it
/// is written, its commit failed, the database could not undo all of it,
/// or its session had to be closed. A run the user stopped after rows came
/// back stays on Results.
fn opens_messages(
    result: &Result<tabletist_db::ScriptOutcome, Error>,
    cancel: Option<CancelReason>,
) -> bool {
    use tabletist_db::StatementOutcome;
    let Ok(outcome) = result else {
        return true;
    };
    let any = |wanted: fn(&StatementOutcome) -> bool| {
        outcome.results.iter().any(|result| wanted(&result.outcome))
    };
    // A run sent to write that left something to read about its end.
    let end = matches!(
        outcome.end,
        tabletist_db::ScriptEnd::Partly { .. } | tabletist_db::ScriptEnd::CommitFailed { .. }
    ) || outcome.rollback_warning.is_some()
        || outcome.broken.is_some();
    end || any(|outcome| matches!(outcome, StatementOutcome::Error { .. }))
        || matches!(cancel, Some(CancelReason::Timeout(_)))
        || (outcome.was_cancelled()
            && !any(|outcome| matches!(outcome, StatementOutcome::Rows { .. })))
}

/// Moves `index` by `delta` within `0..len` (len > 0), saturating.
fn step(index: usize, delta: isize, len: usize) -> usize {
    let moved = (index as i128 + delta as i128).clamp(0, len as i128 - 1);
    moved as usize
}

/// A row's text for the row panel, formatted once. A cell of the page's
/// row `row` that is pending in `cells` reads as its new value, and keeps
/// what it loaded as beside it, so the panel never disagrees with the
/// grid.
fn row_fields(
    request: Option<RequestId>,
    row: usize,
    values: &[tabletist_db::Value],
    cells: &std::collections::BTreeMap<(usize, usize), crate::edit::Pending>,
) -> crate::model::RowFields {
    use crate::ui::format::{cell_text, field_text};
    use tabletist_db::{NewValue, Value};
    let pending: Vec<Option<crate::model::PendingField>> = values
        .iter()
        .enumerate()
        .map(|(col, loaded)| {
            let new = match &cells.get(&(row, col))?.new {
                NewValue::Text(text) => Value::Text(text.as_str().into()),
                NewValue::Null => Value::Null,
            };
            Some(crate::model::PendingField {
                new,
                was: cell_text(loaded).into_owned(),
            })
        })
        .collect();
    let fields = values
        .iter()
        .zip(&pending)
        .map(|(loaded, pending)| field_text(pending.as_ref().map_or(loaded, |cell| &cell.new)))
        .collect();
    crate::model::RowFields {
        request,
        row,
        fields,
        // A row with nothing pending keeps no list of nothing.
        pending: if pending.iter().any(Option::is_some) {
            pending
        } else {
            Vec::new()
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Action, ConnTabContent};

    fn app() -> (App, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let app = App::new(
            AppDirs::at(dir.path()),
            Settings::default().into(),
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
            Settings::default().into(),
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
            Settings::default().into(),
            Backend::recording(),
        );
        assert_eq!(saves(&app), 0);
    }

    #[test]
    fn settings_read_from_the_old_json_are_written_as_toml_once() {
        let dir = tempfile::tempdir().unwrap();
        let dirs = AppDirs::at(dir.path());
        let path = dirs.settings_file();
        let settings = Settings {
            page_size: 100,
            ..Settings::default()
        };
        // Saves of these settings to the TOML file, and of nothing else.
        let saves = |app: &App| {
            app.backend
                .sent
                .iter()
                .filter(|command| {
                    matches!(
                        command,
                        Command::Save { path: to, file: StateFile::Settings(saved) }
                            if *to == path && *saved == settings
                    )
                })
                .count()
        };
        let app = App::new(
            dirs.clone(),
            Loaded::of(settings.clone(), Source::Json),
            Backend::recording(),
        );
        assert_eq!(app.settings, settings);
        assert_eq!(saves(&app), 1);
        assert!(!path.exists(), "the UI thread writes nothing");
        // A TOML file, or none, is not written at a start.
        for source in [Source::Toml, Source::Defaults] {
            let app = App::new(
                dirs.clone(),
                Loaded::of(settings.clone(), source),
                Backend::recording(),
            );
            assert_eq!(saves(&app), 0, "{source:?}");
        }
    }

    #[test]
    fn the_app_keeps_the_text_and_the_lines_it_started_with() {
        let dir = tempfile::tempdir().unwrap();
        let text = "[data]\npage_size = 100\ngroup_digits = \"yes\"\n";
        let app = App::new(
            AppDirs::at(dir.path()),
            Settings::from_toml(text),
            Backend::recording(),
        );
        assert_eq!(app.settings.page_size, 100);
        assert_eq!(app.settings_file.text, text);
        assert_eq!(app.settings_file.invalid, vec![3]);
        assert_eq!(
            app.settings_file.lines,
            vec![(crate::settings::Key::PageSize, 2)]
        );
        assert!(!app.settings_file.live);
    }

    #[test]
    fn a_new_theme_name_is_resolved_at_the_next_logic_pass() {
        let mut harness = Harness::new();
        // A catalog that holds the theme already: no directory is read.
        let mut nord = Palette::dark();
        nord.accent = egui::Color32::from_rgb(0x88, 0xc0, 0xd0);
        harness.app.themes = Catalog::preview(
            vec![crate::theme::CustomTheme {
                filename: "Nord.json".into(),
                palette: nord,
            }],
            false,
        );
        let settings = Settings {
            custom_theme: Some("Nord.json".into()),
            ..harness.app.settings.clone()
        };
        harness.app.apply_settings(settings);
        assert!(harness.app.theme_changed);
        assert_ne!(harness.app.palette, nord, "not before `logic`");
        harness.app.logic(&harness.ctx.clone());
        assert!(!harness.app.theme_changed);
        assert_eq!(harness.app.palette, nord.with_readable_labels());
        // Tests do not follow the desktop: no scan of the themes directory.
        assert!(!harness.app.themes.loading());
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

    /// Opens another connection: the picker, then a connect in it.
    fn connect_another(app: &mut App) -> ConnTabId {
        app.apply(Action::ShowConnections);
        connect(app).0
    }

    #[test]
    fn showing_the_connections_opens_one_picker_tab_and_returns_to_it() {
        let (mut app, _dir) = app();
        // The app starts on the picker: nothing to open.
        app.apply(Action::ShowConnections);
        assert_eq!(app.tabs.len(), 1);
        let (first, _, _) = connect(&mut app);
        app.apply(Action::ShowConnections);
        assert_eq!(app.tabs.len(), 2);
        assert_eq!(app.active, 1, "a new tab opens at the end and shows");
        assert!(matches!(
            app.active_tab().content,
            ConnTabContent::Picker(_)
        ));
        let picker = app.active_tab_id();
        // From the connection again, the same picker tab shows.
        app.apply(Action::ActivateConnTab(first));
        app.apply(Action::ShowConnections);
        assert_eq!(app.tabs.len(), 2);
        assert_eq!(app.active_tab_id(), picker);
        let unique: std::collections::HashSet<_> = ids(&app).into_iter().collect();
        assert_eq!(unique.len(), 2, "tab ids must be unique");
    }

    #[test]
    fn a_saved_connection_knows_the_tab_that_has_it_open() {
        let (mut app, _dir) = app();
        let (tab, _, _) = connect(&mut app);
        let conn = app.workspace(tab).unwrap().conn_id.clone();
        assert_eq!(app.tab_showing(&conn), Some(tab));
        assert_eq!(app.tab_showing(&ConnectionId::new()), None);
        // Open twice, the first tab is the one that shows.
        app.apply(Action::ShowConnections);
        let second = app.active_tab_id();
        app.apply(Action::Connect {
            tab: second,
            conn: conn.clone(),
        });
        assert_eq!(app.tab_showing(&conn), Some(tab));
    }

    #[test]
    fn disconnecting_shows_the_one_picker() {
        let (mut app, _dir) = app();
        // Alone, the connection's tab becomes the picker.
        let (only, session, _) = connect(&mut app);
        app.apply(Action::Disconnect(only));
        assert_eq!(ids(&app), [only.0]);
        assert!(matches!(
            app.active_tab().content,
            ConnTabContent::Picker(_)
        ));
        assert!(
            matches!(app.backend.sent.last(), Some(Command::Close { session: s }) if *s == session)
        );
        // With the picker open in a tab of its own, the connection's tab
        // closes and the picker shows.
        let (first, session, _) = connect(&mut app);
        let second = connect_another(&mut app);
        app.apply(Action::ShowConnections);
        let picker = app.active_tab_id();
        app.apply(Action::Disconnect(first));
        assert_eq!(ids(&app), [second.0, picker.0]);
        assert_eq!(app.active_tab_id(), picker);
        assert!(
            app.backend
                .sent
                .iter()
                .any(|command| matches!(command, Command::Close { session: s } if *s == session))
        );
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
        let (first, _, _) = connect(&mut app);
        connect_another(&mut app);
        let active = connect_another(&mut app);
        app.apply(Action::CloseConnTab(first));
        assert_eq!(app.active_tab_id(), active);
    }

    #[test]
    fn closing_the_active_tab_activates_its_right_neighbour_or_the_new_last() {
        let (mut app, _dir) = app();
        let (a, _, _) = connect(&mut app);
        let b = connect_another(&mut app);
        let c = connect_another(&mut app);
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
    fn connections_activate_by_position_and_tabs_cycle_with_wrapping() {
        let (mut app, _dir) = app();
        let (first, _, _) = connect(&mut app);
        let second = connect_another(&mut app);
        // The picker, last, is a tab but not a connection.
        app.apply(Action::ShowConnections);
        app.apply(Action::ActivateConnection(0));
        assert_eq!(app.active_tab_id(), first);
        app.apply(Action::ActivateConnection(1));
        assert_eq!(app.active_tab_id(), second);
        app.apply(Action::ActivateConnection(2));
        assert_eq!(app.active_tab_id(), second, "the picker has no number");
        app.apply(Action::ActivateConnection(7));
        assert_eq!(
            app.active_tab_id(),
            second,
            "a position past the end is ignored"
        );
        app.apply(Action::CycleConnTab(1));
        assert_eq!(app.active, 2, "cycling reaches the picker");
        app.apply(Action::CycleConnTab(1));
        assert_eq!(app.active, 0);
        app.apply(Action::CycleConnTab(-1));
        assert_eq!(app.active, 2);
    }

    #[test]
    fn queued_actions_are_drained_in_order() {
        let (mut app, _dir) = app();
        let (first, _, _) = connect(&mut app);
        app.actions.push(Action::ShowConnections);
        app.actions.push(Action::ActivateConnection(0));
        app.apply_actions();
        assert!(app.actions.is_empty());
        assert_eq!(app.tabs.len(), 2);
        assert_eq!(app.active_tab_id(), first);
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
    fn a_session_opens_with_the_access_its_saved_connection_asks_for() {
        use tabletist_db::Access;
        let sent = |app: &App| match app.backend.sent.last() {
            Some(Command::Connect { access, .. }) => *access,
            other => panic!("expected a Connect command, got {other:?}"),
        };
        let (mut app, _dir) = app();
        // Dev, its box never set: writable.
        let (tab, _, _) = connect(&mut app);
        assert_eq!(sent(&app), Access::Writable);
        assert_eq!(app.workspace(tab).unwrap().access, Access::Writable);
        // The box turned on meanwhile changes nothing until the tab
        // connects again.
        let id = app.workspace(tab).unwrap().conn_id.clone();
        let mut saved = app.connections.get(&id).unwrap().clone();
        saved.read_only = Some(true);
        app.connections.upsert(saved);
        assert_eq!(app.workspace(tab).unwrap().access, Access::Writable);
        app.apply(Action::Reconnect(tab));
        assert_eq!(sent(&app), Access::ReadOnly);
        assert_eq!(app.workspace(tab).unwrap().access, Access::ReadOnly);
    }

    #[test]
    fn a_reconnect_reads_the_box_again_and_keeps_the_tabs_own_environment() {
        use tabletist_db::Access;
        let sent = |app: &App| match app.backend.sent.last() {
            Some(Command::Connect { access, spec, .. }) => (*access, spec.clone()),
            other => panic!("expected a Connect command, got {other:?}"),
        };
        let (mut app, _dir) = app();
        // Production, its box never set: read-only by its environment.
        let conn = with_saved(&mut app);
        let mut saved = app.connections.get(&conn).unwrap().clone();
        saved.environment = crate::env::Environment::Production;
        app.connections.upsert(saved.clone());
        let tab = app.active_tab_id();
        app.apply(Action::Connect {
            tab,
            conn: conn.clone(),
        });
        let opened = saved.spec.clone();
        assert_eq!(sent(&app), (Access::ReadOnly, opened.clone()));
        // The saved entry is relabelled and pointed elsewhere. The tab is
        // still production's, on the server it opened: the default is its
        // own environment's, not the entry's new one.
        saved.environment = crate::env::Environment::Dev;
        saved.spec = ConnectSpec::sqlite("/tmp/elsewhere.db");
        app.connections.upsert(saved.clone());
        app.apply(Action::Reconnect(tab));
        assert_eq!(sent(&app), (Access::ReadOnly, opened.clone()));
        assert_eq!(app.workspace(tab).unwrap().access, Access::ReadOnly);
        // The box, once set, is read again.
        saved.read_only = Some(false);
        app.connections.upsert(saved.clone());
        app.apply(Action::Reconnect(tab));
        assert_eq!(sent(&app), (Access::Writable, opened.clone()));
        saved.read_only = Some(true);
        app.connections.upsert(saved);
        app.apply(Action::Reconnect(tab));
        assert_eq!(sent(&app), (Access::ReadOnly, opened.clone()));
        // A connection deleted since keeps what the tab has.
        app.apply(Action::DeleteConnection(conn));
        app.apply(Action::Reconnect(tab));
        assert_eq!(sent(&app), (Access::ReadOnly, opened));
        assert_eq!(app.workspace(tab).unwrap().access, Access::ReadOnly);
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
            access: crate::testing::asked_access(&app),
        }));
        assert!(matches!(
            app.workspace(tab).unwrap().status,
            SessionStatus::Connected
        ));
        assert!(app.workspace(tab).unwrap().connected_at.is_some());
    }

    #[test]
    fn a_connected_tab_is_what_its_session_says_it_was_opened_as() {
        use tabletist_db::Access;
        let (mut app, _dir) = app();
        // Dev, its box never set: the connect asks for a writable session.
        let (tab, session, request) = connect(&mut app);
        assert_eq!(app.workspace(tab).unwrap().access, Access::Writable);
        app.apply(Action::Backend(Event::Connected {
            session,
            request,
            driver: Driver::Sqlite,
            encrypted: false,
            access: Access::ReadOnly,
        }));
        assert_eq!(app.workspace(tab).unwrap().access, Access::ReadOnly);
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
            access: crate::testing::asked_access(&app),
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
            access: crate::testing::asked_access(&app),
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
            access: crate::testing::asked_access(&app),
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
            access: crate::testing::asked_access(&app),
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
    fn sending_the_connect_starts_its_clock() {
        let (mut app, _dir) = app();
        let (tab, _, _) = connect(&mut app);
        let first = app.workspace(tab).unwrap().connect_started;
        let first = first.expect("the connect was sent");
        // The clock a second behind: a reconnect that left it alone would
        // still read that.
        let earlier = std::time::Instant::now().checked_sub(std::time::Duration::from_secs(1));
        if earlier.is_some() {
            app.workspace_mut(tab).unwrap().connect_started = earlier;
        }
        // A reconnect is a new attempt with a clock of its own.
        let sent = app.backend.sent.len();
        app.apply(Action::Reconnect(tab));
        assert!(
            app.backend.sent[sent..]
                .iter()
                .any(|command| matches!(command, Command::Connect { .. })),
            "the reconnect sent a Connect"
        );
        let second = app.workspace(tab).unwrap().connect_started;
        let second = second.expect("the clock was restarted");
        match earlier {
            Some(earlier) => assert!(second > earlier, "the clock runs from the new Connect"),
            // A clock too young to set back: a coarse one can read the same
            // for both.
            None => assert!(second >= first, "the clock runs from the new Connect"),
        }
    }

    #[test]
    fn a_reconnect_waiting_at_the_password_prompt_has_no_clock() {
        let (mut app, _dir) = app();
        let conn = postgres_saved(&mut app, PasswordMode::Ask);
        let tab = app.active_tab_id();
        app.apply(Action::Connect { tab, conn });
        prompt(&mut app).password = "wrong".into();
        app.apply(Action::SubmitPassword);
        assert!(app.workspace(tab).unwrap().connect_started.is_some());
        let (session, request, _) = last_connect(&app);
        app.apply(Action::Backend(Event::ConnectFailed {
            session,
            request,
            error: rejected(),
        }));
        app.apply(Action::CloseDialog);
        let sent = app.backend.sent.len();
        app.apply(Action::Reconnect(tab));
        assert!(matches!(app.dialog, Some(Dialog::Password(_))));
        assert!(
            !app.backend.sent[sent..]
                .iter()
                .any(|command| matches!(command, Command::Connect { .. })),
            "no Connect is sent while the prompt is open"
        );
        let workspace = app.workspace(tab).unwrap();
        assert!(matches!(workspace.status, SessionStatus::Connecting { .. }));
        assert!(
            workspace.connect_started.is_none(),
            "the time of the attempt before is not shown"
        );
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

    fn settings_saves(app: &App) -> Vec<&Settings> {
        app.backend
            .sent
            .iter()
            .filter_map(|command| match command {
                Command::Save {
                    file: StateFile::Settings(settings),
                    ..
                } => Some(settings),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_change_made_in_the_app_is_applied_written_and_kept_as_the_files_text() {
        let dir = tempfile::tempdir().unwrap();
        // A file the user wrote, with a line the reader ignored.
        let text = "[data]\npage_size = 100\ngroup_digits = \"yes\"\n";
        let mut app = App::new(
            AppDirs::at(dir.path()),
            Settings::from_toml(text),
            Backend::recording(),
        );
        app.change_settings(|settings| settings.sql_limit = 100);
        assert_eq!(app.settings.sql_limit, 100);
        assert_eq!(app.settings.page_size, 100, "what the file set stays");
        let saved = settings_saves(&app);
        assert_eq!(saved.len(), 1);
        assert_eq!(*saved[0], app.settings);
        // The file is the canonical text now: nothing in it is invalid.
        assert_eq!(app.settings_file.text, app.settings.to_toml());
        assert!(app.settings_file.invalid.is_empty());
        assert_eq!(
            app.settings_file.lines,
            Settings::from_toml(&app.settings.to_toml()).lines
        );
    }

    #[test]
    fn a_change_that_changes_nothing_is_not_written() {
        let (mut app, _dir) = app();
        let limit = app.settings.sql_limit;
        app.change_settings(|settings| settings.sql_limit = limit);
        assert!(settings_saves(&app).is_empty());
    }

    #[test]
    fn a_change_made_in_the_app_is_brought_into_range() {
        let (mut app, _dir) = app();
        app.change_settings(|settings| settings.page_size = 5);
        assert_eq!(app.settings.page_size, Settings::MIN_PAGE_SIZE);
        assert_eq!(settings_saves(&app).len(), 1);
    }

    #[test]
    fn a_change_that_is_brought_back_to_what_was_set_is_not_written() {
        let (mut app, _dir) = app();
        app.change_settings(|settings| settings.page_size = Settings::MIN_PAGE_SIZE);
        let saved = settings_saves(&app).len();
        // Below the range: brought up to the size already set.
        app.change_settings(|settings| settings.page_size = 5);
        assert_eq!(app.settings.page_size, Settings::MIN_PAGE_SIZE);
        assert_eq!(settings_saves(&app).len(), saved);
    }

    #[test]
    fn a_change_of_the_file_is_applied_and_not_written_back() {
        let (mut app, _dir) = app();
        let text = "[data]\npage_size = 500\ngroup_digits = \"yes\"\ntimestamps = \"full\"\n";
        app.apply(Action::Backend(Event::SettingsFile {
            text: text.into(),
            own: false,
        }));
        assert_eq!(app.settings.page_size, 500);
        assert_eq!(app.settings.timestamps, crate::settings::Timestamps::Full);
        assert!(!app.settings.group_digits);
        // The file as the user wrote it, with the line that was ignored.
        assert_eq!(app.settings_file.text, text);
        assert_eq!(app.settings_file.invalid, vec![3]);
        assert_eq!(
            app.settings_file.lines,
            vec![
                (crate::settings::Key::PageSize, 2),
                (crate::settings::Key::Timestamps, 4)
            ]
        );
        assert!(settings_saves(&app).is_empty(), "the user's file is theirs");
    }

    #[test]
    fn the_apps_own_text_coming_back_is_not_read_again() {
        let (mut app, _dir) = app();
        app.change_settings(|settings| settings.sql_limit = 100);
        let saved = settings_saves(&app).len();
        // Something only a second reading would change.
        app.settings_file.invalid = vec![9];
        let text = app.settings_file.text.clone();
        app.apply(Action::Backend(Event::SettingsFile { text, own: true }));
        assert_eq!(app.settings_file.invalid, vec![9]);
        assert_eq!(app.settings.sql_limit, 100);
        assert_eq!(settings_saves(&app).len(), saved);
    }

    /// The settings file's text coming from the disk.
    fn from_disk(text: &str, own: bool) -> Action {
        Action::Backend(Event::SettingsFile {
            text: text.into(),
            own,
        })
    }

    #[test]
    fn an_older_write_of_the_apps_own_coming_back_late_undoes_nothing() {
        let (mut app, _dir) = app();
        app.change_settings(|settings| settings.sql_limit = 100);
        let first = app.settings_file.text.clone();
        app.change_settings(|settings| settings.page_size = 500);
        let second = app.settings_file.text.clone();
        let saved = settings_saves(&app).len();
        // The disk was read between the two writes.
        app.apply(from_disk(&first, true));
        assert_eq!(app.settings.page_size, 500, "the newer change stays");
        assert_eq!(app.settings_file.text, second);
        // Then the newest comes back.
        app.apply(from_disk(&second, true));
        assert_eq!(app.settings.page_size, 500);
        assert_eq!(settings_saves(&app).len(), saved);
    }

    #[test]
    fn a_text_written_for_the_editor_is_the_apps_newest_write() {
        let (mut app, _dir) = app();
        let ours = app.settings_file.text.clone();
        // The editor is asked for with the text the app holds...
        app.apply(Action::EditSettingsFile);
        // ...an edit from outside is read and applied, and the file is
        // deleted, before the backend gets to it...
        app.apply(from_disk("[data]\npage_size = 500\n", false));
        assert_eq!(app.settings.page_size, 500);
        // ...so the backend writes the text it was given, and that is
        // what the disk and the editor have.
        app.apply(from_disk(&ours, true));
        assert_eq!(app.settings.page_size, Settings::DEFAULT_PAGE_SIZE);
        assert_eq!(app.settings_file.text, ours);
    }

    #[test]
    fn each_text_written_for_an_editor_that_waits_is_the_apps_own() {
        let (mut app, _dir) = app();
        let first = app.settings_file.text.clone();
        // Two requests wait at once, with a change from outside between
        // them: the file is deleted, the first of them writes its text,
        // and the second finds a file and writes nothing.
        app.apply(Action::EditSettingsFile);
        let second = "[data]\npage_size = 500\n";
        app.apply(from_disk(second, false));
        app.apply(Action::EditSettingsFile);
        assert_eq!(app.settings.page_size, 500);
        app.apply(from_disk(&first, true));
        assert_eq!(app.settings.page_size, Settings::DEFAULT_PAGE_SIZE);
        assert_eq!(app.settings_file.text, first);
        // The other way round: the second is the one that wrote.
        app.apply(from_disk(second, true));
        assert_eq!(app.settings.page_size, 500);
        // Asked for again with a text already handed over, nothing is
        // kept twice.
        app.apply(Action::EditSettingsFile);
        app.apply(Action::EditSettingsFile);
        assert_eq!(app.settings_file.offered.len(), 2);
    }

    #[test]
    fn a_save_made_since_the_editor_was_asked_for_is_the_newer_write() {
        let (mut app, _dir) = app();
        let offered = app.settings_file.text.clone();
        app.apply(Action::EditSettingsFile);
        app.change_settings(|settings| settings.page_size = 500);
        let saved = settings_saves(&app).len();
        // The text written for the editor is read before the save lands
        // over it: the save is still to come.
        app.apply(from_disk(&offered, true));
        assert_eq!(app.settings.page_size, 500);
        assert_eq!(settings_saves(&app).len(), saved);
    }

    #[test]
    fn the_same_older_text_from_someone_else_is_a_change() {
        let (mut app, _dir) = app();
        app.change_settings(|settings| settings.sql_limit = 100);
        let first = app.settings_file.text.clone();
        app.change_settings(|settings| settings.page_size = 500);
        // Not the backend's write: someone put that text there.
        app.apply(from_disk(&first, false));
        assert_eq!(app.settings.page_size, Settings::DEFAULT_PAGE_SIZE);
        assert_eq!(app.settings_file.text, first);
    }

    #[test]
    fn the_apps_save_landing_over_a_change_from_outside_is_what_the_disk_has() {
        let (mut app, _dir) = app();
        app.change_settings(|settings| settings.sql_limit = 100);
        let ours = app.settings_file.text.clone();
        // An edit from outside is read before the app's save lands...
        app.apply(from_disk("[data]\npage_size = 500\n", false));
        assert_eq!(app.settings.page_size, 500);
        assert_eq!(app.settings.sql_limit, 1_000);
        let saved = settings_saves(&app).len();
        // ...and then it lands, over the edit.
        app.apply(from_disk(&ours, true));
        assert_eq!(app.settings.sql_limit, 100);
        assert_eq!(app.settings.page_size, Settings::DEFAULT_PAGE_SIZE);
        assert_eq!(app.settings_file.text, ours);
        assert_eq!(settings_saves(&app).len(), saved, "nothing is written back");
    }

    #[test]
    fn a_change_of_the_file_keeps_whether_it_is_watched() {
        let (mut app, _dir) = app();
        app.apply(Action::Backend(Event::SettingsWatch { live: true }));
        assert!(app.settings_file.live);
        app.apply(Action::Backend(Event::SettingsFile {
            text: "[data]\npage_size = 500\n".into(),
            own: false,
        }));
        assert!(app.settings_file.live);
        app.change_settings(|settings| settings.sql_limit = 100);
        assert!(app.settings_file.live);
    }

    fn settings_row(app: &App) -> Option<usize> {
        match &app.dialog {
            Some(Dialog::Settings(dialog)) => Some(dialog.row),
            _ => None,
        }
    }

    #[test]
    fn settings_open_over_nothing_or_the_shortcuts_and_close() {
        let (mut app, _dir) = app();
        app.apply(Action::ShowSettings);
        assert_eq!(settings_row(&app), Some(0));
        // A second ask changes nothing: the cursor stays where it is.
        app.apply(Action::MoveSettingsRow(2));
        app.apply(Action::ShowSettings);
        assert_eq!(settings_row(&app), Some(2));
        app.apply(Action::CloseDialog);
        assert!(app.dialog.is_none());
        // The shortcuts dialog offers it, and gives way to it.
        app.apply(Action::ShowHelp);
        app.apply(Action::ShowSettings);
        assert_eq!(settings_row(&app), Some(0));
        // Another dialog does not.
        app.apply(Action::CloseDialog);
        app.apply(Action::ShowAbout);
        app.apply(Action::ShowSettings);
        assert!(matches!(app.dialog, Some(Dialog::About)));
    }

    #[test]
    fn the_settings_cursor_stays_among_the_options() {
        use crate::settings::OptionId;
        let (mut app, _dir) = app();
        app.apply(Action::ShowSettings);
        app.apply(Action::MoveSettingsRow(-1));
        assert_eq!(settings_row(&app), Some(0));
        app.apply(Action::MoveSettingsRow(1));
        assert_eq!(settings_row(&app), Some(1));
        app.apply(Action::MoveSettingsRow(99));
        assert_eq!(settings_row(&app), Some(OptionId::ALL.len() - 1));
        app.apply(Action::SelectSettingsRow(2));
        assert_eq!(settings_row(&app), Some(2));
        app.apply(Action::SelectSettingsRow(99));
        assert_eq!(
            settings_row(&app),
            Some(2),
            "no such row: left where it was"
        );
        // Without the screen the cursor's actions do nothing.
        app.apply(Action::CloseDialog);
        app.apply(Action::MoveSettingsRow(1));
        assert!(app.dialog.is_none());
    }

    #[test]
    fn setting_an_option_changes_it_and_saves() {
        use crate::settings::{OptionValue, Timestamps};
        let (mut app, _dir) = app();
        app.apply(Action::SetOption(OptionValue::Timestamps(Timestamps::Full)));
        assert_eq!(app.settings.timestamps, Timestamps::Full);
        assert_eq!(settings_saves(&app).len(), 1);
        // The value it already has is not written again.
        app.apply(Action::SetOption(OptionValue::Timestamps(Timestamps::Full)));
        assert_eq!(settings_saves(&app).len(), 1);
        // A page size out of range comes into it, as from the file.
        app.apply(Action::SetOption(OptionValue::PageSize(5)));
        assert_eq!(app.settings.page_size, Settings::MIN_PAGE_SIZE);
    }

    #[test]
    fn the_settings_file_is_watched_through_the_backend() {
        let (mut app, _dir) = app();
        app.watch_settings();
        let path = app.dirs.settings_file();
        assert!(matches!(
            app.backend.sent.last(),
            Some(Command::WatchSettings { path: watched }) if *watched == path
        ));
    }

    #[test]
    fn the_settings_file_is_opened_in_the_editor_through_the_backend() {
        let (mut app, _dir) = app();
        app.apply(Action::EditSettingsFile);
        let path = app.dirs.settings_file();
        let text = app.settings_file.text.clone();
        assert!(matches!(
            app.backend.sent.last(),
            Some(Command::EditSettingsFile { path: sent, text: held })
                if *sent == path && *held == text
        ));
    }

    #[test]
    fn reveal_sends_the_path_and_the_text_the_app_holds() {
        let (mut app, _dir) = app();
        app.apply(Action::RevealSettingsFile);
        match crate::testing::last_sent(&app) {
            Command::RevealSettingsFile { path, text } => {
                assert_eq!(path, &app.dirs.settings_file());
                assert_eq!(text, &app.settings_file.text);
            }
            other => panic!("{other:?}"),
        }
        // Written by the backend if the file is gone: the app's own write.
        assert!(app.settings_file.offered.contains(&app.settings_file.text));
    }

    #[test]
    fn export_offers_the_canonical_text_under_the_settings_name() {
        let (mut app, _dir) = app();
        // A file as someone wrote it: not the text the app would write.
        app.apply(from_disk("[data]\npage_size = 500\n", false));
        assert_eq!(app.settings.page_size, 500);
        let text = app.settings.to_toml();
        assert_ne!(app.settings_file.text.len(), text.len());
        app.apply(Action::ExportSettings);
        assert_eq!(
            app.backend.saves.last(),
            Some(&(
                "Export settings".to_owned(),
                "tabletist-settings.toml".to_owned(),
                text.len()
            ))
        );
    }

    #[test]
    fn reset_asks_first_and_then_resets_only_the_four_options() {
        let (mut app, _dir) = app();
        app.apply(Action::ShowSettings);
        // An option of the window, and a key it does not show.
        app.change_settings(|settings| {
            settings.page_size = 500;
            settings.group_digits = true;
            settings.show_system_schemas = true;
            settings.sql_limit = 50;
        });
        let resetting = |app: &App| match &app.dialog {
            Some(Dialog::Settings(dialog)) => dialog.resetting,
            _ => panic!("the window is not open"),
        };
        app.apply(Action::ResetSettings);
        assert!(resetting(&app));
        assert_eq!(app.settings.page_size, 500, "nothing is reset yet");
        // Cancel leaves everything.
        app.apply(Action::ConfirmResetSettings(false));
        assert!(!resetting(&app));
        assert_eq!(app.settings.page_size, 500);
        // Reset puts the four back and leaves the others.
        app.apply(Action::ResetSettings);
        app.apply(Action::ConfirmResetSettings(true));
        assert!(!resetting(&app));
        let defaults = Settings::default();
        assert_eq!(app.settings.page_size, defaults.page_size);
        assert_eq!(app.settings.group_digits, defaults.group_digits);
        assert!(app.settings.show_system_schemas);
        assert_eq!(app.settings.sql_limit, 50);
        assert_eq!(app.settings_file.text, app.settings.to_toml());
    }

    #[test]
    fn a_reset_nobody_asked_for_resets_nothing() {
        let (mut app, _dir) = app();
        app.apply(Action::ShowSettings);
        app.change_settings(|settings| settings.page_size = 500);
        app.apply(Action::ConfirmResetSettings(true));
        assert_eq!(app.settings.page_size, 500);
    }

    #[test]
    fn a_reveal_that_fails_is_told_in_the_notice() {
        let (mut app, _dir) = app();
        app.apply(Action::Backend(Event::SettingsFileOpened {
            with: crate::backend::Opened::Folder,
            result: Err("no file manager".into()),
        }));
        let notice = app.notice.expect("a notice");
        assert!(notice.contains("Could not show"), "{notice}");
        assert!(notice.contains("no file manager"), "{notice}");
    }

    #[test]
    fn an_editor_that_did_not_start_shows_a_notice() {
        let (mut app, _dir) = app();
        app.apply(Action::Backend(Event::SettingsFileOpened {
            with: crate::backend::Opened::Editor,
            result: Err("no editor".into()),
        }));
        let notice = app.notice.clone().expect("a notice");
        assert!(notice.contains("no editor"), "{notice}");
        app.notice = None;
        app.apply(Action::Backend(Event::SettingsFileOpened {
            with: crate::backend::Opened::Editor,
            result: Ok(()),
        }));
        assert!(app.notice.is_none());
    }

    #[test]
    fn a_start_that_does_not_follow_the_desktop_does_not_watch_the_users_directories() {
        // The harness attaches as the demo does.
        let harness = crate::testing::Harness::new();
        assert!(
            !harness
                .app
                .backend
                .sent
                .iter()
                .any(|command| matches!(command, Command::WatchSettings { .. }))
        );
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
    fn a_url_that_fills_the_form_leaves_the_url_field() {
        let (mut app, _dir) = app();
        app.apply(Action::NewConnection);
        form(&mut app).url_mode = true;
        form(&mut app).url = "not a url".into();
        app.apply(Action::ApplyUrl);
        assert!(form(&mut app).message.is_some());
        assert!(
            form(&mut app).url_mode,
            "a URL that does not parse stays to be fixed"
        );
        assert_eq!(form(&mut app).url, "not a url");
        form(&mut app).url = "postgres://me@db.example.com/app".into();
        app.apply(Action::ApplyUrl);
        assert!(!form(&mut app).url_mode);
        assert_eq!(form(&mut app).host, "db.example.com");
        assert_eq!(
            form(&mut app).url,
            "",
            "a URL that fills the form is used up"
        );
        form(&mut app).url_mode = true;
        form(&mut app).url = "sqlite:///srv/app.db".into();
        app.apply(Action::ApplyUrl);
        assert!(!form(&mut app).url_mode);
        assert_eq!(form(&mut app).sqlite_path, "/srv/app.db");
        assert_eq!(form(&mut app).url, "");
    }

    #[test]
    fn a_url_that_filled_the_form_does_not_undo_a_later_edit() {
        let (mut app, _dir) = app();
        app.apply(Action::NewConnection);
        form(&mut app).name = "Shop".into();
        form(&mut app).url_mode = true;
        form(&mut app).url = "postgres://me@db.example.com/app".into();
        app.apply(Action::ApplyUrl);
        // The host is corrected by hand, and the URL field shown again.
        form(&mut app).host = "replica.example.com".into();
        form(&mut app).url_mode = true;
        app.apply(Action::SaveConnection { connect: false });
        assert!(app.dialog.is_none());
        assert_eq!(
            app.connections.connections[0].spec.host,
            "replica.example.com"
        );
    }

    #[test]
    fn saving_from_the_url_field_saves_the_urls_connection() {
        let (mut app, _dir) = app();
        app.apply(Action::NewConnection);
        form(&mut app).name = "Shop".into();
        form(&mut app).url_mode = true;
        form(&mut app).url = "postgres://me@db.example.com/app".into();
        app.apply(Action::SaveConnection { connect: false });
        assert!(app.dialog.is_none(), "the URL was filled in and saved");
        assert_eq!(app.connections.connections.len(), 1);
        let saved = &app.connections.connections[0];
        assert_eq!(saved.name, "Shop");
        assert_eq!(saved.spec.driver, Driver::Postgres);
        assert_eq!(saved.spec.host, "db.example.com");
    }

    #[test]
    fn a_url_that_does_not_parse_is_not_saved_over() {
        let (mut app, _dir) = app();
        app.apply(Action::NewConnection);
        // The parameters describe a connection; the URL being typed does not.
        form(&mut app).name = "Shop".into();
        form(&mut app).sqlite_path = "/tmp/shop.db".into();
        form(&mut app).url_mode = true;
        form(&mut app).url = "not a url".into();
        app.apply(Action::SaveConnection { connect: false });
        assert!(app.connections.connections.is_empty(), "nothing is saved");
        assert!(form(&mut app).message.is_some());
        assert!(form(&mut app).url_mode, "the URL stays to be fixed");
        let before = app.backend.sent.len();
        app.apply(Action::TestConnection);
        assert!(
            !app.backend.sent[before..]
                .iter()
                .any(|command| matches!(command, Command::Test { .. })),
            "and nothing is tested"
        );
        assert!(form(&mut app).url_mode);
    }

    #[test]
    fn a_test_from_the_url_field_tests_the_urls_server() {
        let (mut app, _dir) = app();
        app.apply(Action::NewConnection);
        form(&mut app).url_mode = true;
        form(&mut app).url = "postgres://me@db.example.com/app".into();
        app.apply(Action::TestConnection);
        match app.backend.sent.last() {
            Some(Command::Test { spec, .. }) => assert_eq!(spec.host, "db.example.com"),
            other => panic!("{other:?}"),
        }
        assert!(!form(&mut app).url_mode);
    }

    #[test]
    fn a_message_about_the_fields_leaves_the_url_field() {
        // Nothing typed anywhere: the message names fields the URL mode
        // does not show.
        let (mut app, _dir) = app();
        app.apply(Action::NewConnection);
        form(&mut app).url_mode = true;
        app.apply(Action::SaveConnection { connect: false });
        assert!(form(&mut app).message.is_some());
        assert!(!form(&mut app).url_mode, "a failed save shows the fields");
        form(&mut app).url_mode = true;
        form(&mut app).message = None;
        app.apply(Action::TestConnection);
        assert!(form(&mut app).message.is_some());
        assert!(!form(&mut app).url_mode, "a failed test shows the fields");
    }

    #[test]
    fn a_test_that_passes_says_how_long_it_took() {
        let (mut app, _dir) = app();
        app.apply(Action::NewConnection);
        form(&mut app).url = "postgres://me@localhost/app".into();
        app.apply(Action::ApplyUrl);
        app.apply(Action::TestConnection);
        let request = match form(&mut app).test {
            TestState::Running(request) => request,
            ref other => panic!("{other:?}"),
        };
        assert!(form(&mut app).test_started.is_some());
        app.apply(Action::Backend(Event::Tested {
            request,
            result: Ok(()),
        }));
        assert_eq!(form(&mut app).test, TestState::Passed);
        assert!(form(&mut app).test_took.is_some());
        // The next Test forgets that time until it finishes itself.
        app.apply(Action::TestConnection);
        assert!(form(&mut app).test_took.is_none());
        // A Test that fails has no time to show.
        let request = match form(&mut app).test {
            TestState::Running(request) => request,
            ref other => panic!("{other:?}"),
        };
        app.apply(Action::Backend(Event::Tested {
            request,
            result: Err(Error::Connect("nope".into())),
        }));
        assert!(matches!(form(&mut app).test, TestState::Failed(_)));
        assert!(form(&mut app).test_took.is_none());
    }

    #[test]
    fn the_test_clock_starts_when_the_test_is_sent_not_while_the_keyring_is_read() {
        let (mut app, _dir) = app();
        let conn = postgres_saved(&mut app, PasswordMode::Keyring);
        app.apply(Action::EditConnection(conn));
        app.apply(Action::TestConnection);
        let Some(Command::LoadSecret { request, .. }) = app.backend.sent.last() else {
            panic!()
        };
        let request = *request;
        assert!(
            form(&mut app).test_started.is_none(),
            "waiting for the keyring is not the Test"
        );
        app.apply(Action::Backend(Event::SecretLoaded {
            request,
            result: Ok(Some(SecretString("pw".into()))),
        }));
        assert!(matches!(
            app.backend.sent.last(),
            Some(Command::Test { .. })
        ));
        assert!(form(&mut app).test_started.is_some());
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
    fn a_picked_ca_certificate_fills_its_field() {
        let (mut app, _dir) = app();
        app.apply(Action::NewConnection);
        app.apply(Action::PickCaFile);
        let request = form(&mut app).pick_request.expect("a pick is in flight");
        assert_eq!(form(&mut app).pick_target, PickTarget::CaFile);
        app.apply(Action::Backend(Event::FilePicked {
            request,
            path: Some("/etc/ssl/ca.pem".into()),
        }));
        assert_eq!(form(&mut app).ca_file, "/etc/ssl/ca.pem");
        assert_eq!(form(&mut app).sqlite_path, "", "only the CA file changes");
    }

    #[test]
    fn a_pick_overtaken_by_another_fills_nothing() {
        let (mut app, _dir) = app();
        app.apply(Action::NewConnection);
        app.apply(Action::PickKeyFile);
        let first = form(&mut app).pick_request.expect("a pick is in flight");
        app.apply(Action::PickCaFile);
        let second = form(&mut app).pick_request.expect("a second pick");
        assert_ne!(first, second);
        app.apply(Action::Backend(Event::FilePicked {
            request: first,
            path: Some("/home/me/.ssh/id_ed25519".into()),
        }));
        assert_eq!(form(&mut app).ssh_key_file, "");
        assert_eq!(form(&mut app).ca_file, "");
        assert_eq!(form(&mut app).pick_request, Some(second));
        app.apply(Action::Backend(Event::FilePicked {
            request: second,
            path: Some("/etc/ssl/ca.pem".into()),
        }));
        assert_eq!(form(&mut app).ca_file, "/etc/ssl/ca.pem");
        assert_eq!(form(&mut app).ssh_key_file, "");
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
    fn refreshing_the_tree_keeps_the_groups_unfolded_in_a_folded_schema() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        harness.app.apply(Action::ToggleGroup {
            tab,
            schema: "main".into(),
            prefix: "order".into(),
        });
        let toggle = || Action::ToggleSchema {
            tab,
            schema: "main".into(),
        };
        // Folded: its tables are not the refresh's to load again.
        harness.app.apply(toggle());
        let before = harness.app.backend.sent.len();
        harness.app.apply(Action::RefreshTree(tab));
        let new = &harness.app.backend.sent[before..];
        assert!(!new.iter().any(|c| matches!(c, Command::ListObjects { .. })));
        let node = &harness.app.workspace(tab).unwrap().tree.nodes["main"];
        assert!(node.open_groups.contains("order"), "the user's stays");
        // Its names are forgotten, and load again once it is unfolded.
        assert!(node.objects.needs_load());
        harness.app.apply(toggle());
        assert!(matches!(
            last_sent(&harness.app),
            Command::ListObjects { schema, .. } if schema == "main"
        ));
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

    use crate::model::{CellPos, ObjectView, TabId};
    use crate::testing::{error_outcome, last_sent, page, rows_outcome, script_outcome};
    use std::time::Duration;
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
            access: crate::testing::asked_access(&harness.app),
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
            access: crate::testing::asked_access(&harness.app),
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
            access: crate::testing::asked_access(&harness.app),
        }));
        assert!(object(&harness, tab, background).rows.is_loading());
    }

    fn users() -> ObjectRef {
        ObjectRef::new("main", "users")
    }

    fn open(harness: &mut Harness, tab: ConnTabId, name: &str, pin: bool) -> TabId {
        harness.app.apply(Action::OpenObject {
            tab,
            object: ObjectRef::new("main", name),
            kind: ObjectKind::Table,
            pin,
        });
        harness.app.workspace(tab).unwrap().active_tab.unwrap()
    }

    fn object(harness: &Harness, tab: ConnTabId, id: TabId) -> &crate::model::ObjectTab {
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
        assert_eq!(workspace.tabs.len(), 1, "the preview was replaced");
        assert_eq!(
            workspace.object_tabs().next().unwrap().object.name,
            "orders"
        );
        let orders = open(&mut harness, tab, "orders", true);
        assert!(object(&harness, tab, orders).pinned);
        open(&mut harness, tab, "users", false);
        assert_eq!(
            harness.app.workspace(tab).unwrap().tabs.len(),
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
        harness.app.apply(Action::CloseTab { tab, id: b });
        assert_eq!(harness.app.workspace(tab).unwrap().active_tab, Some(a));
        harness.app.apply(Action::CloseTab { tab, id: a });
        assert_eq!(harness.app.workspace(tab).unwrap().active_tab, None);
    }

    #[test]
    fn object_tabs_cycle_with_wrapping() {
        let mut harness = Harness::new();
        let tab = connect_tab(&mut harness);
        let a = open(&mut harness, tab, "users", true);
        let b = open(&mut harness, tab, "orders", true);
        harness.app.apply(Action::CycleTab { tab, step: 1 });
        assert_eq!(harness.app.workspace(tab).unwrap().active_tab, Some(a));
        harness.app.apply(Action::CycleTab { tab, step: -1 });
        assert_eq!(harness.app.workspace(tab).unwrap().active_tab, Some(b));
    }

    /// Puts a run in flight on a SQL editor, as running its text does.
    fn start_run(harness: &mut Harness, tab: ConnTabId, id: TabId) -> RequestId {
        let request = RequestId(harness.app.next_id());
        let workspace = harness.app.workspace_mut(tab).unwrap();
        let statements = tabletist_db::sql::statements(workspace.driver.dialect(), "SELECT 1");
        let superseded = workspace.sql_tab_mut(id).unwrap().start_run(
            request,
            statements,
            tabletist_db::ScriptMode::ReadOnly,
        );
        assert_eq!(superseded, None);
        request
    }

    /// Answers the newest Connect, as the backend does once it connected.
    fn answer_connect(harness: &mut Harness) -> SessionId {
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
            access: crate::testing::asked_access(&harness.app),
        }));
        session
    }

    fn version_requests(harness: &Harness, from: usize) -> Vec<(SessionId, RequestId)> {
        harness.app.backend.sent[from..]
            .iter()
            .filter_map(|command| match command {
                Command::ServerVersion { session, request } => Some((*session, *request)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn reconnecting_forgets_a_sql_run_and_asks_for_the_version_again() {
        let mut harness = Harness::new();
        let tab = connect_tab(&mut harness);
        let query = harness.add_sql_tab(tab);
        let running = start_run(&mut harness, tab, query);
        let workspace = harness.app.workspace_mut(tab).unwrap();
        workspace.server_version.value = Some("SQLite 3.46.0".into());
        workspace.sql_tab_mut(query).unwrap().text = "SELECT 1".into();
        let old = workspace.session;
        harness.app.apply(Action::Backend(Event::Disconnected {
            session: old,
            error: Error::ConnectionLost("gone".into()),
        }));
        let sent = harness.app.backend.sent.len();
        harness.app.apply(Action::Reconnect(tab));
        let workspace = harness.app.workspace(tab).unwrap();
        let sql = workspace.sql_tab(query).unwrap();
        assert!(
            !sql.is_running() && sql.in_flight.is_none(),
            "nothing answers for the closed session"
        );
        assert_eq!(sql.text, "SELECT 1", "the editor keeps what was typed");
        assert!(workspace.server_version.needs_load());
        assert!(
            version_requests(&harness, sent).is_empty(),
            "not before the session connects"
        );
        // The answer of the old session, should it still come, is dropped.
        harness.app.apply(Action::Backend(Event::SqlRan {
            session: old,
            request: running,
            result: Ok(Default::default()),
            cancel: None,
        }));
        // So is anything else the old session says, even about the run
        // the new session has in flight.
        let rerun = start_run(&mut harness, tab, query);
        harness.app.apply(Action::Backend(Event::SqlRan {
            session: old,
            request: rerun,
            result: Ok(Default::default()),
            cancel: None,
        }));
        let workspace = harness.app.workspace(tab).unwrap();
        let sql = workspace.sql_tab(query).unwrap();
        assert_eq!(sql.run.pending, Some(rerun), "still running");
        assert!(sql.run.value.is_none());
        let session = answer_connect(&mut harness);
        let asked = version_requests(&harness, sent);
        assert_eq!(asked.len(), 1);
        assert_eq!(asked[0].0, session);
        let workspace = harness.app.workspace(tab).unwrap();
        assert_eq!(workspace.server_version.pending, Some(asked[0].1));
        assert!(workspace.sql_tab(query).unwrap().run.value.is_none());
        harness.app.apply(Action::Backend(Event::ServerVersion {
            session,
            request: asked[0].1,
            result: Ok("SQLite 3.46.1".into()),
        }));
        assert_eq!(
            harness
                .app
                .workspace(tab)
                .unwrap()
                .server_version
                .value
                .as_deref(),
            Some("SQLite 3.46.1")
        );
    }

    #[test]
    fn connecting_without_a_sql_editor_does_not_ask_for_the_version() {
        let mut harness = Harness::new();
        let tab = connect_tab(&mut harness);
        open(&mut harness, tab, "users", true);
        harness.app.apply(Action::Reconnect(tab));
        answer_connect(&mut harness);
        assert!(version_requests(&harness, 0).is_empty());
        assert!(
            harness
                .app
                .workspace(tab)
                .unwrap()
                .server_version
                .needs_load()
        );
    }

    #[test]
    fn a_version_answer_for_another_request_is_ignored() {
        let mut harness = Harness::new();
        let tab = connect_tab(&mut harness);
        harness.add_sql_tab(tab);
        harness.app.apply(Action::Reconnect(tab));
        let session = answer_connect(&mut harness);
        let asked = version_requests(&harness, 0);
        harness.app.apply(Action::Backend(Event::ServerVersion {
            session,
            request: RequestId(asked[0].1.0 + 1_000),
            result: Ok("stale".into()),
        }));
        let workspace = harness.app.workspace(tab).unwrap();
        assert!(workspace.server_version.value.is_none());
        assert!(workspace.server_version.is_loading());
    }

    fn strip(harness: &Harness, tab: ConnTabId) -> Vec<TabId> {
        let workspace = harness.app.workspace(tab).unwrap();
        workspace.tabs.iter().map(Tab::id).collect()
    }

    #[test]
    fn tabs_of_both_kinds_activate_and_cycle() {
        let mut harness = Harness::new();
        let tab = connect_tab(&mut harness);
        let users = open(&mut harness, tab, "users", true);
        let query = harness.add_sql_tab(tab);
        let orders = open(&mut harness, tab, "orders", true);
        assert_eq!(strip(&harness, tab), vec![users, query, orders]);
        let active = |harness: &Harness| harness.app.workspace(tab).unwrap().active_tab;
        harness.app.apply(Action::CycleTab { tab, step: -1 });
        assert_eq!(active(&harness), Some(query));
        harness.app.apply(Action::CycleTab { tab, step: -1 });
        assert_eq!(active(&harness), Some(users));
        harness.app.apply(Action::CycleTab { tab, step: -1 });
        assert_eq!(active(&harness), Some(orders));
        harness.app.apply(Action::ActivateTab { tab, id: query });
        assert_eq!(active(&harness), Some(query));
        // A tab that is not there (closed since the click) changes nothing.
        harness.app.apply(Action::ActivateTab {
            tab,
            id: TabId(u64::MAX),
        });
        assert_eq!(active(&harness), Some(query));
    }

    #[test]
    fn no_object_is_active_while_a_sql_editor_shows() {
        let mut harness = Harness::new();
        let tab = connect_tab(&mut harness);
        let users = open(&mut harness, tab, "users", true);
        assert_eq!(harness.app.active_object(), Some((tab, users)));
        let query = harness.add_sql_tab(tab);
        harness.app.apply(Action::ActivateTab { tab, id: query });
        assert_eq!(harness.app.active_object(), None);
        assert!(
            harness
                .app
                .workspace(tab)
                .unwrap()
                .active_sql_tab()
                .is_some()
        );
    }

    #[test]
    fn closing_a_sql_editor_stops_its_run_and_shows_a_neighbour() {
        let mut harness = Harness::new();
        let tab = connect_tab(&mut harness);
        let users = open(&mut harness, tab, "users", true);
        let query = harness.add_sql_tab(tab);
        let running = start_run(&mut harness, tab, query);
        harness.app.workspace_mut(tab).unwrap().active_tab = Some(query);
        let sent = harness.app.backend.sent.len();
        harness.app.apply(Action::CloseTab { tab, id: query });
        assert_eq!(cancels_since(&harness, sent), vec![running]);
        assert_eq!(strip(&harness, tab), vec![users]);
        assert_eq!(
            harness.app.workspace(tab).unwrap().active_tab,
            Some(users),
            "the last tab left takes over"
        );
    }

    #[test]
    fn a_preview_never_replaces_a_sql_editor() {
        let mut harness = Harness::new();
        let tab = connect_tab(&mut harness);
        let query = harness.add_sql_tab(tab);
        let users = open(&mut harness, tab, "users", false);
        assert_eq!(strip(&harness, tab), vec![query, users]);
        let orders = open(&mut harness, tab, "orders", false);
        assert_eq!(
            strip(&harness, tab),
            vec![query, orders],
            "only the preview object tab gives way"
        );
    }

    /// Opens a SQL editor on the fixture connection.
    fn new_sql(harness: &mut Harness) -> (ConnTabId, TabId) {
        let tab = harness.connect_fake();
        harness.app.apply(Action::NewSqlTab(tab));
        let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
        (tab, id)
    }

    fn sql(harness: &Harness, tab: ConnTabId, id: TabId) -> &SqlTab {
        harness.app.workspace(tab).unwrap().sql_tab(id).unwrap()
    }

    /// Types `text` into the editor, leaving the cursor at byte `cursor`.
    fn type_sql(harness: &mut Harness, tab: ConnTabId, id: TabId, text: &str, cursor: usize) {
        let workspace = harness.app.workspace_mut(tab).unwrap();
        let sql = workspace.sql_tab_mut(id).unwrap();
        sql.text = text.into();
        sql.cursor = cursor;
    }

    fn run(harness: &mut Harness, tab: ConnTabId, id: TabId, all: bool) {
        harness.app.apply(Action::RunSql {
            tab,
            sql_tab: id,
            all,
        });
    }

    fn runs_since(harness: &Harness, from: usize) -> usize {
        harness.app.backend.sent[from..]
            .iter()
            .filter(|command| matches!(command, Command::RunSql { .. }))
            .count()
    }

    #[test]
    fn sql_tabs_are_numbered_per_connection_and_ask_for_the_version() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        open(&mut harness, tab, "users", true);
        harness.app.workspace_mut(tab).unwrap().pane = Pane::Tree;
        harness.app.apply(Action::NewSqlTab(tab));
        assert!(matches!(
            last_sent(&harness.app),
            Command::ServerVersion { .. }
        ));
        let workspace = harness.app.workspace(tab).unwrap();
        let first = workspace.active_tab.unwrap();
        assert_eq!(harness.app.active_sql(), Some((tab, first)));
        assert_eq!(harness.app.active_workspace_tab(), Some((tab, first)));
        assert_eq!(harness.app.active_object(), None);
        assert_eq!(workspace.pane, Pane::Grid, "the tree gives up the keys");
        harness.app.apply(Action::NewSqlTab(tab));
        let workspace = harness.app.workspace(tab).unwrap();
        let second = workspace.active_tab.unwrap();
        assert_ne!(first, second);
        assert_eq!(workspace.tabs.len(), 3, "after the object tab");
        assert_eq!(workspace.sql_tab(first).unwrap().number, 1);
        assert_eq!(workspace.sql_tab(second).unwrap().number, 2);
        assert_eq!(workspace.sql_tab(second).unwrap().limit, 1_000);
        assert_eq!(
            workspace.sql_tab(second).unwrap().timeout,
            Some(Duration::from_secs(30))
        );
        assert!(workspace.sql_tab(second).unwrap().focus_editor);
        assert_eq!(
            version_requests(&harness, 0).len(),
            1,
            "the version is asked for once"
        );
    }

    #[test]
    fn a_new_sql_tab_starts_from_the_settings() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        harness.app.settings.sql_limit = 100;
        harness.app.settings.sql_timeout_secs = None;
        harness.app.apply(Action::NewSqlTab(tab));
        let workspace = harness.app.workspace(tab).unwrap();
        let sql = workspace.active_sql_tab().unwrap();
        assert_eq!((sql.limit, sql.timeout), (100, None));
    }

    #[test]
    fn a_sql_tab_opened_while_connecting_asks_for_the_version_once_connected() {
        let (mut app, _dir) = app();
        let (tab, session, request) = connect(&mut app);
        app.apply(Action::NewSqlTab(tab));
        assert!(app.workspace(tab).unwrap().active_sql_tab().is_some());
        let asked = |app: &App| {
            app.backend
                .sent
                .iter()
                .filter(|command| matches!(command, Command::ServerVersion { .. }))
                .count()
        };
        assert_eq!(asked(&app), 0, "the session cannot answer yet");
        assert!(app.workspace(tab).unwrap().server_version.needs_load());
        app.apply(Action::Backend(Event::Connected {
            session,
            request,
            driver: Driver::Sqlite,
            encrypted: false,
            access: crate::testing::asked_access(&app),
        }));
        assert_eq!(asked(&app), 1);
        // On a picker there is no workspace to open an editor in.
        app.apply(Action::ShowConnections);
        let picker = app.active_tab_id();
        app.apply(Action::NewSqlTab(picker));
        assert!(app.workspace(picker).is_none());
        assert_eq!(app.active_sql(), None);
        assert_eq!(app.active_workspace_tab(), None);
    }

    #[test]
    fn run_sends_the_statement_at_the_cursor_and_run_all_sends_every_one() {
        let mut harness = Harness::new();
        let (tab, id) = new_sql(&mut harness);
        type_sql(&mut harness, tab, id, "SELECT 1;\nSELECT 2;", 12);
        run(&mut harness, tab, id, false);
        let session = harness.app.workspace(tab).unwrap().session;
        assert!(matches!(
            last_sent(&harness.app),
            Command::RunSql { session: sent, statements, limit: 1_000, timeout: Some(timeout), .. }
                if *sent == session
                    && statements.len() == 1
                    && statements[0].text == "SELECT 2"
                    && statements[0].first_line == 2
                    && *timeout == Duration::from_secs(30)
        ));
        let editor = sql(&harness, tab, id);
        assert!(editor.is_running());
        assert_eq!(editor.in_flight.as_ref().unwrap().statements.len(), 1);
        run(&mut harness, tab, id, true);
        assert!(matches!(
            last_sent(&harness.app),
            Command::RunSql { statements, .. } if statements.len() == 2
        ));
        assert_eq!(
            sql(&harness, tab, id)
                .in_flight
                .as_ref()
                .unwrap()
                .statements
                .len(),
            2
        );
    }

    #[test]
    fn running_an_empty_editor_does_nothing() {
        let mut harness = Harness::new();
        let (tab, id) = new_sql(&mut harness);
        let sent = harness.app.backend.sent.len();
        for all in [false, true] {
            type_sql(&mut harness, tab, id, "", 0);
            run(&mut harness, tab, id, all);
            type_sql(&mut harness, tab, id, "  -- nothing\n /* at all */ ;\n", 4);
            run(&mut harness, tab, id, all);
        }
        assert_eq!(harness.app.backend.sent.len(), sent);
        let editor = sql(&harness, tab, id);
        assert!(editor.run.needs_load(), "no run, and no error");
        assert!(!editor.is_running());
        assert_eq!(editor.pane, ResultPane::Results);
    }

    #[test]
    fn running_nothing_leaves_the_run_in_flight_alone() {
        let mut harness = Harness::new();
        let (tab, id) = new_sql(&mut harness);
        type_sql(&mut harness, tab, id, "SELECT 1", 0);
        run(&mut harness, tab, id, false);
        let running = sql(&harness, tab, id).run.pending;
        let sent = harness.app.backend.sent.len();
        type_sql(&mut harness, tab, id, "-- nothing", 0);
        run(&mut harness, tab, id, true);
        assert_eq!(harness.app.backend.sent.len(), sent, "nothing is cancelled");
        assert_eq!(sql(&harness, tab, id).run.pending, running);
    }

    #[test]
    fn run_on_something_that_is_not_a_sql_tab_does_nothing() {
        let mut harness = Harness::new();
        let (tab, id) = new_sql(&mut harness);
        let users = open(&mut harness, tab, "users", true);
        harness.answer_rows(page(5, false));
        type_sql(&mut harness, tab, id, "SELECT 1", 0);
        let sent = harness.app.backend.sent.len();
        run(&mut harness, tab, users, false);
        run(&mut harness, tab, TabId(u64::MAX), true);
        run(&mut harness, ConnTabId(u64::MAX), id, true);
        assert_eq!(harness.app.backend.sent.len(), sent);
        assert!(!sql(&harness, tab, id).is_running());
    }

    #[test]
    fn a_new_run_cancels_the_one_still_running() {
        let mut harness = Harness::new();
        let (tab, id) = new_sql(&mut harness);
        type_sql(&mut harness, tab, id, "SELECT 1", 0);
        run(&mut harness, tab, id, false);
        let first = sql(&harness, tab, id).run.pending.unwrap();
        let sent = harness.app.backend.sent.len();
        run(&mut harness, tab, id, false);
        let second = sql(&harness, tab, id).run.pending.unwrap();
        assert_ne!(first, second);
        assert_eq!(cancels_since(&harness, sent), vec![first]);
        assert!(
            matches!(
                harness.app.backend.sent[sent],
                Command::Cancel { request, .. } if request == first
            ),
            "the cancel goes first: the session runs one thing at a time"
        );
        assert_eq!(runs_since(&harness, sent), 1);
    }

    #[test]
    fn a_late_answer_for_a_replaced_run_is_dropped() {
        let mut harness = Harness::new();
        let (tab, id) = new_sql(&mut harness);
        type_sql(&mut harness, tab, id, "SELECT 1", 0);
        run(&mut harness, tab, id, false);
        let session = harness.app.workspace(tab).unwrap().session;
        let first = sql(&harness, tab, id).run.pending.unwrap();
        run(&mut harness, tab, id, false);
        let workspace = harness.app.workspace_mut(tab).unwrap();
        let editor = workspace.sql_tab_mut(id).unwrap();
        editor.pane = ResultPane::Messages;
        editor.selection = Some(CellPos { row: 0, col: 0 });
        harness.app.apply(Action::Backend(Event::SqlRan {
            session,
            request: first,
            result: Ok(script_outcome(vec![rows_outcome(5)])),
            cancel: Some(CancelReason::User),
        }));
        let editor = sql(&harness, tab, id);
        assert!(editor.is_running(), "the second run is still in flight");
        assert!(editor.run.value.is_none());
        assert_eq!(editor.pane, ResultPane::Messages, "nothing was touched");
        assert_eq!(editor.selection, Some(CellPos { row: 0, col: 0 }));
        harness.answer_sql(Ok(script_outcome(vec![rows_outcome(2)])), None);
        let editor = sql(&harness, tab, id);
        assert!(!editor.is_running());
        assert_eq!(editor.dims(), (2, 3));
        // Nor does an answer from another session, even one naming the
        // run in flight.
        run(&mut harness, tab, id, false);
        let third = sql(&harness, tab, id).run.pending.unwrap();
        harness.app.apply(Action::Backend(Event::SqlRan {
            session: SessionId(u64::MAX),
            request: third,
            result: Ok(script_outcome(vec![rows_outcome(9)])),
            cancel: None,
        }));
        let editor = sql(&harness, tab, id);
        assert_eq!(editor.run.pending, Some(third), "still running");
        assert_eq!(editor.dims(), (2, 3));
    }

    #[test]
    fn a_result_shows_its_rows_and_an_error_opens_messages() {
        let mut harness = Harness::new();
        let (tab, id) = new_sql(&mut harness);
        type_sql(&mut harness, tab, id, "SELECT 1;\nSELECT x", 0);
        run(&mut harness, tab, id, true);
        harness
            .app
            .workspace_mut(tab)
            .unwrap()
            .sql_tab_mut(id)
            .unwrap()
            .selection = Some(CellPos { row: 4, col: 2 });
        harness.answer_sql(
            Ok(script_outcome(vec![
                rows_outcome(5),
                error_outcome("no such column: x", None),
            ])),
            None,
        );
        let editor = sql(&harness, tab, id);
        assert!(!editor.is_running() && editor.in_flight.is_none());
        assert_eq!(editor.pane, ResultPane::Messages);
        assert_eq!(editor.selection, None, "a new result starts unselected");
        assert_eq!(editor.shown().map(|(index, _)| index), Some(0));
        assert_eq!(editor.dims(), (5, 3));
        assert_eq!(editor.error_mark(), Some((2, None)));
        assert_eq!(editor.run.value.as_ref().unwrap().statements.len(), 2);
        // The next run that works shows its rows again.
        type_sql(&mut harness, tab, id, "SELECT 1", 0);
        run(&mut harness, tab, id, false);
        harness.answer_sql(Ok(script_outcome(vec![rows_outcome(3)])), None);
        let editor = sql(&harness, tab, id);
        assert_eq!(editor.pane, ResultPane::Results);
        assert_eq!(editor.dims(), (3, 3));
        assert_eq!(editor.error_mark(), None);
    }

    #[test]
    fn a_refused_script_is_the_runs_error() {
        let mut harness = Harness::new();
        let (tab, id) = new_sql(&mut harness);
        type_sql(&mut harness, tab, id, "SELECT 1", 0);
        run(&mut harness, tab, id, false);
        harness.answer_sql(Ok(script_outcome(vec![rows_outcome(5)])), None);
        type_sql(&mut harness, tab, id, "SELECT 1;\nCOMMIT", 0);
        run(&mut harness, tab, id, true);
        let refused = Error::Refused {
            line: 2,
            what: "COMMIT".into(),
            mode: tabletist_db::ScriptMode::ReadOnly,
        };
        harness.answer_sql(Err(refused.clone()), None);
        let editor = sql(&harness, tab, id);
        assert!(!editor.is_running());
        assert_eq!(editor.run.error, Some(refused));
        assert_eq!(editor.pane, ResultPane::Messages);
        assert_eq!(editor.error_mark(), Some((2, None)));
        // Nothing ran: the rows of the run before are not this run's.
        assert!(editor.last_run().is_none());
        assert_eq!(editor.dims(), (0, 0));
    }

    #[test]
    fn a_result_stays_when_the_session_is_replaced_with_nothing_failed() {
        let mut harness = Harness::new();
        let (tab, id) = new_sql(&mut harness);
        type_sql(&mut harness, tab, id, "SELECT 1", 0);
        run(&mut harness, tab, id, false);
        harness.answer_sql(Ok(script_outcome(vec![rows_outcome(5)])), None);
        // The run in flight goes with its session; the result before it
        // does not, for nothing failed.
        run(&mut harness, tab, id, false);
        harness.app.apply(Action::SwitchDatabase {
            tab,
            database: "other".into(),
        });
        let editor = sql(&harness, tab, id);
        assert!(!editor.is_running() && editor.run.error.is_none());
        assert_eq!(editor.dims(), (5, 3));
        assert_eq!(editor.last_run().map(|run| run.statements.len()), Some(1));
        // Nor does a reconnect drop it.
        let session = answer_connect(&mut harness);
        run(&mut harness, tab, id, false);
        assert!(sql(&harness, tab, id).is_running());
        harness.app.apply(Action::Backend(Event::Disconnected {
            session,
            error: Error::ConnectionLost("gone".into()),
        }));
        harness.app.apply(Action::Reconnect(tab));
        let editor = sql(&harness, tab, id);
        assert!(!editor.is_running() && editor.run.error.is_none());
        assert_eq!(editor.dims(), (5, 3));
    }

    #[test]
    fn a_run_waits_for_a_connected_session() {
        let mut harness = Harness::new();
        let (tab, id) = new_sql(&mut harness);
        type_sql(&mut harness, tab, id, "SELECT 1", 0);
        run(&mut harness, tab, id, false);
        harness.answer_sql(Ok(script_outcome(vec![rows_outcome(5)])), None);
        let kept = |harness: &Harness| {
            let editor = sql(harness, tab, id);
            assert!(!editor.is_running() && editor.run.error.is_none());
            assert_eq!(editor.dims(), (5, 3));
        };
        // Connecting to the other database: the backend would answer that
        // the connection is closed, and that failure drops the result.
        harness.app.apply(Action::SwitchDatabase {
            tab,
            database: "other".into(),
        });
        let status = &harness.app.workspace(tab).unwrap().status;
        assert!(matches!(status, SessionStatus::Connecting { .. }));
        let sent = harness.app.backend.sent.len();
        run(&mut harness, tab, id, false);
        run(&mut harness, tab, id, true);
        assert_eq!(harness.app.backend.sent.len(), sent, "nothing is sent");
        kept(&harness);
        // Nor on a session that is gone.
        let session = answer_connect(&mut harness);
        harness.app.apply(Action::Backend(Event::Disconnected {
            session,
            error: Error::ConnectionLost("gone".into()),
        }));
        let sent = harness.app.backend.sent.len();
        run(&mut harness, tab, id, false);
        assert_eq!(harness.app.backend.sent.len(), sent, "nothing is sent");
        kept(&harness);
        // Connected again, it runs.
        harness.app.apply(Action::Reconnect(tab));
        answer_connect(&mut harness);
        let sent = harness.app.backend.sent.len();
        run(&mut harness, tab, id, false);
        assert_eq!(runs_since(&harness, sent), 1);
        assert!(sql(&harness, tab, id).is_running());
    }

    #[test]
    fn a_run_that_lost_the_session_keeps_the_text_and_no_older_result() {
        let mut harness = Harness::new();
        let (tab, id) = new_sql(&mut harness);
        type_sql(&mut harness, tab, id, "SELECT 1", 0);
        run(&mut harness, tab, id, false);
        harness.answer_sql(Ok(script_outcome(vec![rows_outcome(5)])), None);
        run(&mut harness, tab, id, false);
        harness.answer_sql(Err(Error::LeftReadOnly), None);
        let editor = sql(&harness, tab, id);
        assert!(!editor.is_running());
        assert_eq!(editor.run.error, Some(Error::LeftReadOnly));
        assert_eq!(editor.pane, ResultPane::Messages);
        assert_eq!(editor.error_mark(), None);
        assert_eq!(editor.text, "SELECT 1");
        assert!(editor.last_run().is_none());
        assert_eq!(editor.dims(), (0, 0));
    }

    #[test]
    fn a_run_stopped_before_it_started_is_a_finished_run_without_results() {
        let mut harness = Harness::new();
        let (tab, id) = new_sql(&mut harness);
        type_sql(&mut harness, tab, id, "SELECT 1", 0);
        run(&mut harness, tab, id, false);
        let stopped = crate::testing::stopped_before_it_began();
        harness.answer_sql(Ok(stopped), Some(CancelReason::User));
        let editor = sql(&harness, tab, id);
        assert!(!editor.is_running());
        assert!(editor.run.error.is_none());
        let finished = editor.run.value.as_ref().unwrap();
        assert!(finished.outcome.was_cancelled());
        assert_eq!(finished.cancel, Some(CancelReason::User));
        assert_eq!(finished.statements.len(), 1);
        assert!(editor.shown().is_none());
        assert_eq!(
            editor.pane,
            ResultPane::Messages,
            "Results has nothing to show, Messages says it was cancelled"
        );
    }

    #[test]
    fn a_run_that_was_only_cancelled_opens_messages() {
        let mut harness = Harness::new();
        let (tab, id) = new_sql(&mut harness);
        type_sql(&mut harness, tab, id, "SET x = 1; SELECT slow()", 0);
        // With or without a reason (a statement_timeout the script set
        // itself cancels without one).
        for cancel in [Some(CancelReason::User), None] {
            run(&mut harness, tab, id, true);
            harness.answer_sql(
                Ok(script_outcome(vec![
                    tabletist_db::StatementOutcome::Done {
                        affected: None,
                        warnings: 0,
                    },
                    tabletist_db::StatementOutcome::Cancelled,
                ])),
                cancel,
            );
            let editor = sql(&harness, tab, id);
            assert!(editor.shown().is_none());
            assert_eq!(editor.pane, ResultPane::Messages, "{cancel:?}");
            // A run that works brings Results back.
            run(&mut harness, tab, id, true);
            harness.answer_sql(Ok(script_outcome(vec![rows_outcome(1)])), None);
            assert_eq!(sql(&harness, tab, id).pane, ResultPane::Results);
        }
    }

    #[test]
    fn a_user_cancel_after_rows_stays_on_results() {
        let mut harness = Harness::new();
        let (tab, id) = new_sql(&mut harness);
        type_sql(&mut harness, tab, id, "SELECT 1; SELECT slow()", 0);
        run(&mut harness, tab, id, true);
        harness
            .app
            .workspace_mut(tab)
            .unwrap()
            .sql_tab_mut(id)
            .unwrap()
            .pane = ResultPane::Messages;
        harness.answer_sql(
            Ok(script_outcome(vec![
                rows_outcome(4),
                tabletist_db::StatementOutcome::Cancelled,
            ])),
            Some(CancelReason::User),
        );
        let editor = sql(&harness, tab, id);
        assert_eq!(editor.dims(), (4, 3));
        assert_eq!(
            editor.pane,
            ResultPane::Results,
            "the user stopped it and has rows to read"
        );
    }

    #[test]
    fn a_timed_out_statement_keeps_the_reason_and_the_earlier_rows() {
        let mut harness = Harness::new();
        let (tab, id) = new_sql(&mut harness);
        type_sql(&mut harness, tab, id, "SELECT 1; SELECT slow()", 0);
        run(&mut harness, tab, id, true);
        let timeout = CancelReason::Timeout(Duration::from_secs(30));
        harness.answer_sql(
            Ok(script_outcome(vec![
                rows_outcome(4),
                tabletist_db::StatementOutcome::Cancelled,
            ])),
            Some(timeout),
        );
        let editor = sql(&harness, tab, id);
        assert_eq!(editor.run.value.as_ref().unwrap().cancel, Some(timeout));
        assert_eq!(editor.dims(), (4, 3));
        assert_eq!(editor.error_mark(), None);
        assert_eq!(
            editor.pane,
            ResultPane::Messages,
            "nobody asked for the run to stop, so say why it did"
        );
    }

    #[test]
    fn a_result_for_a_closed_sql_tab_is_ignored() {
        let mut harness = Harness::new();
        let (tab, id) = new_sql(&mut harness);
        type_sql(&mut harness, tab, id, "SELECT 1", 0);
        run(&mut harness, tab, id, false);
        let request = sql(&harness, tab, id).run.pending.unwrap();
        let sent = harness.app.backend.sent.len();
        harness.app.apply(Action::CloseTab { tab, id });
        assert_eq!(cancels_since(&harness, sent), vec![request]);
        harness.answer_sql(Ok(script_outcome(vec![rows_outcome(5)])), None);
        assert!(harness.app.workspace(tab).unwrap().tabs.is_empty());
    }

    #[test]
    fn a_result_lands_in_its_own_sql_tab() {
        let mut harness = Harness::new();
        let (tab, first) = new_sql(&mut harness);
        harness.app.apply(Action::NewSqlTab(tab));
        let second = harness.app.workspace(tab).unwrap().active_tab.unwrap();
        type_sql(&mut harness, tab, first, "SELECT 1", 0);
        run(&mut harness, tab, first, false);
        type_sql(&mut harness, tab, second, "SELECT 2", 0);
        run(&mut harness, tab, second, false);
        // The newest RunSql is the second tab's, though the first is hidden.
        harness.answer_sql(Ok(script_outcome(vec![rows_outcome(2)])), None);
        assert!(sql(&harness, tab, first).is_running());
        assert_eq!(sql(&harness, tab, first).dims(), (0, 0));
        assert_eq!(sql(&harness, tab, second).dims(), (2, 3));
    }

    #[test]
    fn cancel_stops_the_active_sql_run() {
        let mut harness = Harness::new();
        let (tab, id) = new_sql(&mut harness);
        let users = open(&mut harness, tab, "users", true);
        harness.app.apply(Action::ActivateTab { tab, id });
        // Nothing runs in the editor: the object's load behind it is not
        // what the tab shows a spinner for.
        let sent = harness.app.backend.sent.len();
        harness.app.apply(Action::CancelQuery(tab));
        assert_eq!(harness.app.backend.sent.len(), sent);
        type_sql(&mut harness, tab, id, "SELECT 1", 0);
        run(&mut harness, tab, id, false);
        let request = sql(&harness, tab, id).run.pending.unwrap();
        let sent = harness.app.backend.sent.len();
        harness.app.apply(Action::CancelQuery(tab));
        assert_eq!(cancels_since(&harness, sent), vec![request]);
        assert!(
            sql(&harness, tab, id).is_running(),
            "until the backend answers"
        );
        // On an object tab Cancel still stops its loads, not the script.
        harness.app.apply(Action::ActivateTab { tab, id: users });
        let loading: Vec<RequestId> = object(&harness, tab, users).pending().collect();
        assert!(!loading.is_empty());
        let sent = harness.app.backend.sent.len();
        harness.app.apply(Action::CancelQuery(tab));
        assert_eq!(cancels_since(&harness, sent), loading);
    }

    /// A SQL editor on a connection that takes writes.
    fn writable_sql(harness: &mut Harness) -> (ConnTabId, TabId) {
        let tab = harness.connect_fake_as(false);
        harness.app.apply(Action::NewSqlTab(tab));
        let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
        (tab, id)
    }

    /// The mode a run of the editor would have now.
    fn run_mode(harness: &Harness, tab: ConnTabId, id: TabId) -> RunMode {
        let workspace = harness.app.workspace(tab).unwrap();
        workspace.run_mode(workspace.sql_tab(id).unwrap())
    }

    fn set_mode(harness: &mut Harness, tab: ConnTabId, id: TabId, mode: RunMode) {
        harness.app.apply(Action::SetSqlMode {
            tab,
            sql_tab: id,
            mode,
        });
    }

    fn toggle_mode(harness: &mut Harness, tab: ConnTabId, id: TabId) {
        harness
            .app
            .apply(Action::ToggleSqlMode { tab, sql_tab: id });
    }

    #[test]
    fn a_new_editor_reads_only_until_it_is_switched() {
        let mut harness = Harness::new();
        let (tab, id) = writable_sql(&mut harness);
        assert_eq!(sql(&harness, tab, id).mode, RunMode::ReadOnly);
        assert_eq!(run_mode(&harness, tab, id), RunMode::ReadOnly);
        set_mode(&mut harness, tab, id, RunMode::ReadWrite);
        assert_eq!(run_mode(&harness, tab, id), RunMode::ReadWrite);
        // The key switches to the other mode, and back.
        toggle_mode(&mut harness, tab, id);
        assert_eq!(run_mode(&harness, tab, id), RunMode::ReadOnly);
        toggle_mode(&mut harness, tab, id);
        assert_eq!(run_mode(&harness, tab, id), RunMode::ReadWrite);
        // The mode is the tab's own: another editor of the connection
        // starts read-only all the same.
        harness.app.apply(Action::NewSqlTab(tab));
        let other = harness.app.workspace(tab).unwrap().active_tab.unwrap();
        assert_eq!(run_mode(&harness, tab, other), RunMode::ReadOnly);
        assert_eq!(run_mode(&harness, tab, id), RunMode::ReadWrite);
    }

    #[test]
    fn the_badge_and_the_key_do_nothing_where_an_editor_cannot_write() {
        use crate::model::NoWrites;
        // A connection that opens read-only.
        let mut harness = Harness::new();
        let (tab, id) = new_sql(&mut harness);
        let workspace = harness.app.workspace(tab).unwrap();
        assert_eq!(workspace.sql_writes(), Err(NoWrites::ReadOnlyConnection));
        set_mode(&mut harness, tab, id, RunMode::ReadWrite);
        toggle_mode(&mut harness, tab, id);
        assert_eq!(sql(&harness, tab, id).mode, RunMode::ReadOnly);
        // A writable connection to production: a write there is asked about
        // first, and nothing asks about a script yet.
        let mut harness = Harness::new();
        let (tab, id) = writable_sql(&mut harness);
        harness.app.workspace_mut(tab).unwrap().environment = crate::env::Environment::Production;
        let workspace = harness.app.workspace(tab).unwrap();
        assert_eq!(workspace.sql_writes(), Err(NoWrites::Unconfirmed));
        set_mode(&mut harness, tab, id, RunMode::ReadWrite);
        toggle_mode(&mut harness, tab, id);
        assert_eq!(sql(&harness, tab, id).mode, RunMode::ReadOnly);
        // Every other environment writes without a question.
        for environment in crate::env::Environment::ALL {
            let mut harness = Harness::new();
            let (tab, _) = writable_sql(&mut harness);
            harness.app.workspace_mut(tab).unwrap().environment = environment;
            let writes = harness.app.workspace(tab).unwrap().sql_writes();
            assert_eq!(
                writes.is_ok(),
                !environment.confirms_writes(),
                "{environment:?}"
            );
        }
    }

    #[test]
    fn a_session_that_came_back_read_only_runs_read_only_and_keeps_the_tabs_mode() {
        let mut harness = Harness::new();
        let (tab, id) = writable_sql(&mut harness);
        set_mode(&mut harness, tab, id, RunMode::ReadWrite);
        // The box was turned on meanwhile.
        harness.reconnect_fake_as(tab, true);
        assert_eq!(run_mode(&harness, tab, id), RunMode::ReadOnly);
        assert_eq!(sql(&harness, tab, id).mode, RunMode::ReadWrite);
        // Neither the badge nor the key changes what the tab was set to.
        toggle_mode(&mut harness, tab, id);
        set_mode(&mut harness, tab, id, RunMode::ReadOnly);
        assert_eq!(sql(&harness, tab, id).mode, RunMode::ReadWrite);
        // Writable once more, the tab's own mode counts again.
        harness.reconnect_fake_as(tab, false);
        assert_eq!(run_mode(&harness, tab, id), RunMode::ReadWrite);
    }

    /// The transaction the newest run was sent in.
    fn sent_mode(harness: &Harness) -> tabletist_db::ScriptMode {
        match last_sent(&harness.app) {
            Command::RunSql { mode, .. } => *mode,
            other => panic!("expected RunSql, got {other:?}"),
        }
    }

    /// Ends the run in flight as committed, each statement having changed
    /// one row.
    fn commit(harness: &mut Harness, statements: usize) {
        use crate::testing::{done_outcome, write_outcome};
        let done = vec![done_outcome(Some(1)); statements];
        let outcome = write_outcome(done, tabletist_db::ScriptEnd::Committed);
        harness.answer_sql(Ok(outcome), None);
    }

    #[test]
    fn a_run_is_read_write_only_in_a_read_write_editor_and_only_with_a_write() {
        use tabletist_db::ScriptMode::{ReadOnly, Write};
        let mut harness = Harness::new();
        let (tab, id) = writable_sql(&mut harness);
        let script = "SELECT 1;\nUPDATE users SET email = 'x' WHERE id = 1";
        // In Read-only nothing is sent to write, whatever the script holds.
        type_sql(&mut harness, tab, id, script, 0);
        run(&mut harness, tab, id, true);
        assert_eq!(sent_mode(&harness), ReadOnly);
        assert!(!sql(&harness, tab, id).is_writing());
        harness.answer_sql(Ok(crate::testing::script_outcome(Vec::new())), None);
        assert_eq!(sql(&harness, tab, id).last_run().unwrap().mode, ReadOnly);
        // In Read-write the run that holds the write is sent to write.
        set_mode(&mut harness, tab, id, RunMode::ReadWrite);
        run(&mut harness, tab, id, true);
        assert_eq!(sent_mode(&harness), Write);
        assert!(sql(&harness, tab, id).is_writing());
        commit(&mut harness, 2);
        assert!(!sql(&harness, tab, id).is_writing());
        assert_eq!(sql(&harness, tab, id).last_run().unwrap().mode, Write);
        // The statement at the cursor is a read: no read-write transaction
        // is opened for it, though the tab is in Read-write.
        run(&mut harness, tab, id, false);
        assert_eq!(sent_mode(&harness), ReadOnly);
        harness.answer_sql(Ok(crate::testing::script_outcome(Vec::new())), None);
        // With the cursor in the UPDATE, that statement alone is the run.
        type_sql(&mut harness, tab, id, script, script.len());
        run(&mut harness, tab, id, false);
        assert_eq!(sent_mode(&harness), Write);
        assert!(matches!(
            last_sent(&harness.app),
            Command::RunSql { statements, .. } if statements.len() == 1
        ));
    }

    #[test]
    fn a_statement_is_a_write_unless_it_plainly_reads() {
        use tabletist_db::ScriptMode::{ReadOnly, Write};
        let mut harness = Harness::new();
        let (tab, id) = writable_sql(&mut harness);
        set_mode(&mut harness, tab, id, RunMode::ReadWrite);
        for (text, mode) in [
            ("SELECT * FROM users", ReadOnly),
            ("WITH old AS (SELECT 1) SELECT * FROM old", ReadOnly),
            ("EXPLAIN DELETE FROM users", ReadOnly),
            ("PRAGMA table_info(users)", ReadOnly),
            ("-- UPDATE in a note\nSELECT 'DELETE'", ReadOnly),
            ("INSERT INTO users (email) VALUES ('a')", Write),
            ("DELETE FROM users", Write),
            ("CREATE TABLE notes (seen int)", Write),
            (
                "WITH gone AS (DELETE FROM users RETURNING id) SELECT * FROM gone",
                Write,
            ),
            ("EXPLAIN ANALYZE DELETE FROM users", Write),
            // One write among reads is enough.
            ("SELECT 1; DROP TABLE users; SELECT 2", Write),
        ] {
            type_sql(&mut harness, tab, id, text, 0);
            run(&mut harness, tab, id, true);
            assert_eq!(sent_mode(&harness), mode, "{text}");
            harness.answer_sql(Ok(crate::testing::script_outcome(Vec::new())), None);
        }
    }

    #[test]
    fn nothing_is_sent_to_write_where_the_session_cannot() {
        use tabletist_db::ScriptMode::ReadOnly;
        let write = "DELETE FROM users WHERE id = 1";
        // The session came back read-only under a tab in Read-write.
        let mut harness = Harness::new();
        let (tab, id) = writable_sql(&mut harness);
        set_mode(&mut harness, tab, id, RunMode::ReadWrite);
        harness.reconnect_fake_as(tab, true);
        type_sql(&mut harness, tab, id, write, 0);
        run(&mut harness, tab, id, false);
        assert_eq!(sent_mode(&harness), ReadOnly);
        // Production, with a tab that says Read-write however it came to:
        // no read-write run goes there without its confirmation.
        let mut harness = Harness::new();
        let (tab, id) = writable_sql(&mut harness);
        let workspace = harness.app.workspace_mut(tab).unwrap();
        workspace.environment = crate::env::Environment::Production;
        workspace.sql_tab_mut(id).unwrap().mode = RunMode::ReadWrite;
        type_sql(&mut harness, tab, id, write, 0);
        run(&mut harness, tab, id, false);
        assert_eq!(sent_mode(&harness), ReadOnly);
    }

    #[test]
    fn run_does_nothing_while_the_editors_read_write_run_is_in_flight() {
        let mut harness = Harness::new();
        let (tab, id) = writable_sql(&mut harness);
        set_mode(&mut harness, tab, id, RunMode::ReadWrite);
        type_sql(&mut harness, tab, id, "DELETE FROM users WHERE id = 1", 0);
        run(&mut harness, tab, id, false);
        let writing = sql(&harness, tab, id).run.pending.expect("a run in flight");
        let sent = harness.app.backend.sent.len();
        // Neither Run nor Run all replaces it, nor cancels it.
        run(&mut harness, tab, id, false);
        run(&mut harness, tab, id, true);
        assert_eq!(harness.app.backend.sent.len(), sent);
        assert_eq!(sql(&harness, tab, id).run.pending, Some(writing));
        // Cancel still stops it.
        harness.app.apply(Action::CancelQuery(tab));
        assert_eq!(cancels_since(&harness, sent), vec![writing]);
        commit(&mut harness, 1);
        assert!(!sql(&harness, tab, id).is_running());
        // A read-only run is replaced by the next one, as it always was.
        type_sql(&mut harness, tab, id, "SELECT 1", 0);
        run(&mut harness, tab, id, false);
        let reading = sql(&harness, tab, id).run.pending.expect("a run in flight");
        let sent = harness.app.backend.sent.len();
        run(&mut harness, tab, id, false);
        assert_eq!(cancels_since(&harness, sent), vec![reading]);
        assert_eq!(runs_since(&harness, sent), 1);
    }

    #[test]
    fn run_again_sends_the_last_runs_statements_to_write_only_where_it_may() {
        use crate::testing::{refused_write, script_outcome};
        use tabletist_db::ScriptMode::{ReadOnly, Write};
        // A function that writes behind a statement that reads, refused in
        // the read-only run it was taken for.
        let text = "SELECT setval('ids', 9)";
        let refused = |harness: &mut Harness, mode: RunMode| {
            let (tab, id) = writable_sql(harness);
            set_mode(harness, tab, id, mode);
            type_sql(harness, tab, id, text, 0);
            run(harness, tab, id, false);
            assert_eq!(sent_mode(harness), ReadOnly);
            harness.answer_sql(Ok(script_outcome(vec![refused_write()])), None);
            (tab, id)
        };
        let again = |harness: &mut Harness, tab, id| {
            let sent = harness.app.backend.sent.len();
            harness.app.apply(Action::RunSqlAgain { tab, sql_tab: id });
            runs_since(harness, sent)
        };
        // In Read-write, with the text that ran: the same statements, sent
        // to write.
        let mut harness = Harness::new();
        let (tab, id) = refused(&mut harness, RunMode::ReadWrite);
        assert_eq!(again(&mut harness, tab, id), 1);
        assert_eq!(sent_mode(&harness), Write);
        assert!(matches!(
            last_sent(&harness.app),
            Command::RunSql { statements, .. }
                if statements.len() == 1 && statements[0].text == text
        ));
        // While that run is in flight the card's button does nothing more.
        assert_eq!(again(&mut harness, tab, id), 0);
        // Nor after it: a run that was sent to write is not what the card
        // was about.
        commit(&mut harness, 1);
        assert_eq!(again(&mut harness, tab, id), 0);
        // Not once the text is another than the one that ran, and again
        // when it is that text once more.
        let mut harness = Harness::new();
        let (tab, id) = refused(&mut harness, RunMode::ReadWrite);
        type_sql(&mut harness, tab, id, "SELECT setval('ids', 10)", 0);
        assert_eq!(again(&mut harness, tab, id), 0);
        type_sql(&mut harness, tab, id, text, 0);
        assert_eq!(again(&mut harness, tab, id), 1);
        // Not in a tab that is in Read-only, or was switched back since.
        let mut harness = Harness::new();
        let (tab, id) = refused(&mut harness, RunMode::ReadOnly);
        assert_eq!(again(&mut harness, tab, id), 0);
        let mut harness = Harness::new();
        let (tab, id) = refused(&mut harness, RunMode::ReadWrite);
        set_mode(&mut harness, tab, id, RunMode::ReadOnly);
        assert_eq!(again(&mut harness, tab, id), 0);
        // Not once the session came back read-only, whatever the tab says.
        let mut harness = Harness::new();
        let (tab, id) = refused(&mut harness, RunMode::ReadWrite);
        harness.reconnect_fake_as(tab, true);
        assert_eq!(again(&mut harness, tab, id), 0);
        assert_eq!(sql(&harness, tab, id).mode, RunMode::ReadWrite);
    }

    #[test]
    fn messages_open_for_every_end_of_a_read_write_run_that_needs_reading() {
        use crate::testing::{done_outcome, write_outcome};
        use tabletist_db::ScriptEnd;
        let done = || vec![done_outcome(Some(1))];
        let failed = ScriptEnd::CommitFailed {
            error: Error::query("a deferred constraint"),
            committed: 0,
        };
        let mut kept = write_outcome(done(), ScriptEnd::RolledBack);
        kept.rollback_warning = Some("could not be rolled back".into());
        let mut closed = write_outcome(done(), ScriptEnd::Committed);
        closed.broken = Some(Error::ConnectionLost(
            "could not end the transaction".into(),
        ));
        for (outcome, messages) in [
            (write_outcome(done(), ScriptEnd::Committed), false),
            (
                write_outcome(done(), ScriptEnd::Partly { committed: 1 }),
                true,
            ),
            (write_outcome(done(), failed), true),
            (kept, true),
            (closed, true),
        ] {
            assert_eq!(
                opens_messages(&Ok(outcome.clone()), None),
                messages,
                "{outcome:?}"
            );
        }
    }

    #[test]
    fn the_limit_and_timeout_menus_change_the_next_run_and_the_settings() {
        let mut harness = Harness::new();
        let (tab, id) = new_sql(&mut harness);
        harness.app.apply(Action::SetSqlLimit {
            tab,
            sql_tab: id,
            limit: 100,
        });
        assert!(matches!(
            last_sent(&harness.app),
            Command::Save { file: StateFile::Settings(settings), .. }
                if settings.sql_limit == 100
        ));
        let sent = harness.app.backend.sent.len();
        harness.app.apply(Action::SetSqlTimeout {
            tab,
            sql_tab: id,
            secs: None,
        });
        assert!(matches!(
            &harness.app.backend.sent[sent..],
            [Command::Save { file: StateFile::Settings(settings), .. }]
                if settings.sql_limit == 100 && settings.sql_timeout_secs.is_none()
        ));
        type_sql(&mut harness, tab, id, "SELECT 1", 0);
        run(&mut harness, tab, id, false);
        assert!(matches!(
            last_sent(&harness.app),
            Command::RunSql {
                limit: 100,
                timeout: None,
                ..
            }
        ));
        assert_eq!(harness.app.settings.sql_limit, 100);
        assert_eq!(harness.app.settings.sql_timeout_secs, None);
        // The next editor starts from them; the menus of one editor leave
        // the others as they are.
        harness.app.apply(Action::NewSqlTab(tab));
        let second = harness.app.workspace(tab).unwrap().active_tab.unwrap();
        assert_eq!(sql(&harness, tab, second).limit, 100);
        assert_eq!(sql(&harness, tab, second).timeout, None);
        harness.app.apply(Action::SetSqlTimeout {
            tab,
            sql_tab: second,
            secs: Some(60),
        });
        assert_eq!(
            sql(&harness, tab, second).timeout,
            Some(Duration::from_secs(60))
        );
        assert_eq!(sql(&harness, tab, id).timeout, None);
        assert_eq!(harness.app.settings.sql_timeout_secs, Some(60));
    }

    fn saves_since(harness: &Harness, from: usize) -> usize {
        harness.app.backend.sent[from..]
            .iter()
            .filter(|command| matches!(command, Command::Save { .. }))
            .count()
    }

    #[test]
    fn picking_the_limit_or_timeout_already_set_saves_nothing() {
        let mut harness = Harness::new();
        let (tab, first) = new_sql(&mut harness);
        harness.app.apply(Action::NewSqlTab(tab));
        let second = harness.app.workspace(tab).unwrap().active_tab.unwrap();
        let sent = harness.app.backend.sent.len();
        // The second editor's menus change the settings...
        harness.app.apply(Action::SetSqlLimit {
            tab,
            sql_tab: second,
            limit: 100,
        });
        harness.app.apply(Action::SetSqlTimeout {
            tab,
            sql_tab: second,
            secs: Some(60),
        });
        assert_eq!(saves_since(&harness, sent), 2);
        // ...so the same picks in the first only change that editor.
        let sent = harness.app.backend.sent.len();
        harness.app.apply(Action::SetSqlLimit {
            tab,
            sql_tab: first,
            limit: 100,
        });
        harness.app.apply(Action::SetSqlTimeout {
            tab,
            sql_tab: first,
            secs: Some(60),
        });
        assert_eq!(harness.app.backend.sent.len(), sent, "nothing to save");
        assert_eq!(sql(&harness, tab, first).limit, 100);
        assert_eq!(
            sql(&harness, tab, first).timeout,
            Some(Duration::from_secs(60))
        );
        assert_eq!(harness.app.settings.sql_limit, 100);
        assert_eq!(harness.app.settings.sql_timeout_secs, Some(60));
        // A timeout of no seconds is no timeout, which is a change once.
        harness.app.apply(Action::SetSqlTimeout {
            tab,
            sql_tab: first,
            secs: Some(0),
        });
        harness.app.apply(Action::SetSqlTimeout {
            tab,
            sql_tab: first,
            secs: None,
        });
        assert_eq!(saves_since(&harness, sent), 1);
    }

    #[test]
    fn a_limit_or_timeout_that_cannot_work_is_not_taken() {
        let mut harness = Harness::new();
        let (tab, id) = new_sql(&mut harness);
        harness.app.apply(Action::SetSqlLimit {
            tab,
            sql_tab: id,
            limit: 0,
        });
        assert_eq!(sql(&harness, tab, id).limit, 1);
        assert_eq!(harness.app.settings.sql_limit, 1);
        harness.app.apply(Action::SetSqlLimit {
            tab,
            sql_tab: id,
            limit: u32::MAX,
        });
        assert_eq!(sql(&harness, tab, id).limit, Settings::MAX_SQL_LIMIT);
        assert_eq!(harness.app.settings.sql_limit, Settings::MAX_SQL_LIMIT);
        // A timeout of no seconds would cancel every run at once.
        harness.app.apply(Action::SetSqlTimeout {
            tab,
            sql_tab: id,
            secs: Some(0),
        });
        assert_eq!(sql(&harness, tab, id).timeout, None);
        assert_eq!(harness.app.settings.sql_timeout_secs, None);
    }

    #[test]
    fn the_result_pane_switches() {
        let mut harness = Harness::new();
        let (tab, id) = new_sql(&mut harness);
        let sent = harness.app.backend.sent.len();
        harness.app.apply(Action::SetResultPane {
            tab,
            sql_tab: id,
            pane: ResultPane::Messages,
        });
        assert_eq!(sql(&harness, tab, id).pane, ResultPane::Messages);
        harness.app.apply(Action::SetResultPane {
            tab,
            sql_tab: id,
            pane: ResultPane::Results,
        });
        assert_eq!(sql(&harness, tab, id).pane, ResultPane::Results);
        assert_eq!(harness.app.backend.sent.len(), sent);
    }

    #[test]
    fn the_split_is_each_sql_tabs_own_and_starts_at_the_default() {
        let mut harness = Harness::new();
        let (tab, first) = new_sql(&mut harness);
        let sent = harness.app.backend.sent.len();
        harness.app.apply(Action::SetSqlSplit {
            tab,
            sql_tab: first,
            split: 0.7,
        });
        assert_eq!(sql(&harness, tab, first).split, 0.7);
        // A share past the whole height is all of it.
        harness.app.apply(Action::SetSqlSplit {
            tab,
            sql_tab: first,
            split: 3.0,
        });
        assert_eq!(sql(&harness, tab, first).split, 1.0);
        harness.app.apply(Action::NewSqlTab(tab));
        let second = harness.app.workspace(tab).unwrap().active_tab.unwrap();
        assert_eq!(sql(&harness, tab, second).split, SqlTab::DEFAULT_SPLIT);
        assert_eq!(sql(&harness, tab, first).split, 1.0);
        // Nothing is asked of the backend or saved, and a tab that closed
        // since is left alone.
        assert_eq!(harness.app.backend.sent.len(), sent);
        harness.app.apply(Action::CloseTab { tab, id: second });
        harness.app.apply(Action::SetSqlSplit {
            tab,
            sql_tab: second,
            split: 0.2,
        });
        assert_eq!(sql(&harness, tab, first).split, 1.0);
        assert_eq!(harness.app.backend.sent.len(), sent);
    }

    #[test]
    fn format_sql_asks_the_editor_to_format_and_take_the_keys() {
        let mut harness = Harness::new();
        let (tab, id) = new_sql(&mut harness);
        let sent = harness.app.backend.sent.len();
        let workspace = harness.app.workspace_mut(tab).unwrap();
        workspace.sql_tab_mut(id).unwrap().focus_editor = false;
        harness.app.apply(Action::FormatSql { tab, sql_tab: id });
        assert!(sql(&harness, tab, id).format);
        assert!(sql(&harness, tab, id).focus_editor);
        // Nothing is asked of the backend, and a tab that closed since is
        // left alone.
        harness.app.apply(Action::CloseTab { tab, id });
        harness.app.apply(Action::FormatSql { tab, sql_tab: id });
        assert_eq!(harness.app.backend.sent.len(), sent);
    }

    #[test]
    fn arrows_move_in_a_sql_result() {
        let mut harness = Harness::new();
        let (tab, id) = new_sql(&mut harness);
        let step = |harness: &mut Harness, rows, cols| {
            harness.app.apply(Action::MoveSelection {
                tab,
                id,
                rows,
                cols,
            });
            sql(harness, tab, id).selection
        };
        assert_eq!(step(&mut harness, 1, 0), None, "nothing to move in yet");
        type_sql(&mut harness, tab, id, "SELECT 1", 0);
        run(&mut harness, tab, id, false);
        harness.answer_sql(Ok(script_outcome(vec![rows_outcome(3)])), None);
        assert_eq!(
            step(&mut harness, 1, 0),
            Some(CellPos { row: 0, col: 0 }),
            "the first key lands on the first cell"
        );
        assert_eq!(step(&mut harness, 1, 1), Some(CellPos { row: 1, col: 1 }));
        assert_eq!(
            step(&mut harness, isize::MAX, isize::MAX),
            Some(CellPos { row: 2, col: 2 })
        );
        assert_eq!(
            step(&mut harness, isize::MIN, 0),
            Some(CellPos { row: 0, col: 2 })
        );
        // A run without a result set leaves nothing to select.
        run(&mut harness, tab, id, false);
        harness.answer_sql(
            Ok(script_outcome(vec![tabletist_db::StatementOutcome::Done {
                affected: None,
                warnings: 0,
            }])),
            None,
        );
        assert_eq!(step(&mut harness, 1, 0), None);
    }

    #[test]
    fn a_click_selects_a_cell_of_a_sql_result() {
        let mut harness = Harness::new();
        let (tab, id) = new_sql(&mut harness);
        type_sql(&mut harness, tab, id, "SELECT 1", 0);
        run(&mut harness, tab, id, false);
        harness.answer_sql(Ok(script_outcome(vec![rows_outcome(3)])), None);
        harness.app.workspace_mut(tab).unwrap().pane = Pane::Tree;
        let cell = CellPos { row: 2, col: 1 };
        harness.app.apply(Action::SelectCell { tab, id, cell });
        assert_eq!(sql(&harness, tab, id).selection, Some(cell));
        assert_eq!(harness.app.workspace(tab).unwrap().pane, Pane::Grid);
        // A click on a cell the result no longer has (it was replaced
        // after the frame was drawn) selects nothing new.
        for gone in [CellPos { row: 3, col: 0 }, CellPos { row: 0, col: 3 }] {
            harness.app.apply(Action::SelectCell {
                tab,
                id,
                cell: gone,
            });
            assert_eq!(sql(&harness, tab, id).selection, Some(cell));
        }
    }

    #[test]
    fn closed_sql_editors_are_recorded_for_the_frame_to_forget() {
        let mut harness = Harness::new();
        let (tab, first) = new_sql(&mut harness);
        let users = open(&mut harness, tab, "users", true);
        harness.app.apply(Action::NewSqlTab(tab));
        let second = harness.app.workspace(tab).unwrap().active_tab.unwrap();
        harness.app.apply(Action::NewSqlTab(tab));
        let third = harness.app.workspace(tab).unwrap().active_tab.unwrap();
        assert!(harness.app.closed_editors.is_empty());
        // A table tab has no editor to forget.
        harness.app.apply(Action::CloseTab { tab, id: users });
        assert!(harness.app.closed_editors.is_empty());
        harness.app.apply(Action::CloseTab { tab, id: second });
        assert_eq!(harness.app.closed_editors, [(tab, second)]);
        // Closed twice (two clicks in one frame), it is recorded once.
        harness.app.apply(Action::CloseTab { tab, id: second });
        assert_eq!(harness.app.closed_editors, [(tab, second)]);
        // The connection tab closes the editors it still has.
        harness.app.apply(Action::CloseConnTab(tab));
        assert_eq!(
            harness.app.closed_editors,
            [(tab, second), (tab, first), (tab, third)]
        );
        // So does leaving the connection for the picker.
        let mut harness = Harness::new();
        let (tab, id) = new_sql(&mut harness);
        harness.app.apply(Action::Disconnect(tab));
        assert_eq!(harness.app.closed_editors, [(tab, id)]);
    }

    #[test]
    fn a_focused_sql_editor_takes_the_arrows_from_the_tree() {
        let mut harness = Harness::new();
        let (tab, id) = new_sql(&mut harness);
        harness.app.workspace_mut(tab).unwrap().pane = Pane::Tree;
        harness
            .app
            .apply(Action::SqlEditorFocused { tab, sql_tab: id });
        assert_eq!(harness.app.workspace(tab).unwrap().pane, Pane::Grid);
        // An editor closed since the frame that drew it takes nothing.
        harness.app.workspace_mut(tab).unwrap().pane = Pane::Tree;
        harness.app.apply(Action::CloseTab { tab, id });
        harness
            .app
            .apply(Action::SqlEditorFocused { tab, sql_tab: id });
        assert_eq!(harness.app.workspace(tab).unwrap().pane, Pane::Tree);
    }

    #[test]
    fn refresh_does_nothing_on_a_sql_tab() {
        let mut harness = Harness::new();
        let (tab, id) = new_sql(&mut harness);
        let users = open(&mut harness, tab, "users", true);
        harness.answer_rows(page(5, false));
        harness.app.apply(Action::ActivateTab { tab, id });
        let sent = harness.app.backend.sent.len();
        harness.app.apply(Action::Refresh(tab));
        assert_eq!(
            harness.app.backend.sent.len(),
            sent,
            "neither the tree nor the object tab behind it reloads"
        );
        harness.app.apply(Action::ActivateTab { tab, id: users });
        harness.app.apply(Action::Refresh(tab));
        assert!(matches!(last_sent(&harness.app), Command::FetchRows { .. }));
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
        harness.app.apply(Action::CloseTab { tab, id });
        harness.answer_rows(page(5, false));
        assert!(harness.app.workspace(tab).unwrap().tabs.is_empty());
    }

    #[test]
    fn selecting_a_cell_pins_and_selection_is_clamped_to_a_shorter_page() {
        let mut harness = Harness::new();
        let tab = connect_tab(&mut harness);
        let id = open(&mut harness, tab, "users", false);
        harness.answer_rows(page(300, true));
        harness.app.apply(Action::SelectCell {
            tab,
            id,
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
            id,
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
                id,
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
        harness.app.apply(Action::CloseTab { tab, id });
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
            id,
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
            access: crate::testing::asked_access(&app),
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
            access: crate::testing::asked_access(app),
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
            access: crate::testing::asked_access(&app),
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
    fn answering_a_prompt_after_a_failure_forgets_sql_runs_of_the_old_session() {
        let (mut app, _dir) = app();
        let conn = postgres_saved(&mut app, PasswordMode::Ask);
        let tab = app.active_tab_id();
        app.apply(Action::Connect { tab, conn });
        prompt(&mut app).password = "wrong".into();
        app.apply(Action::SubmitPassword);
        let (session, request, _) = last_connect(&app);
        app.apply(Action::Backend(Event::ConnectFailed {
            session,
            request,
            error: rejected(),
        }));
        let id = TabId(app.next_id());
        let run = RequestId(app.next_id());
        let workspace = app.workspace_mut(tab).unwrap();
        let statements = tabletist_db::sql::statements(workspace.driver.dialect(), "SELECT 1");
        let _ = workspace.push_sql_tab(id, 1_000, None).start_run(
            run,
            statements,
            tabletist_db::ScriptMode::ReadOnly,
        );
        workspace.server_version.value = Some("PostgreSQL 17.2".into());
        prompt(&mut app).password = "right".into();
        app.apply(Action::SubmitPassword);
        let workspace = app.workspace(tab).unwrap();
        assert_ne!(workspace.session, session, "a fresh session");
        assert!(!workspace.sql_tab(id).unwrap().is_running());
        assert!(workspace.server_version.needs_load());
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
            access: crate::testing::asked_access(&app),
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
    fn a_password_typed_into_a_connection_saved_without_one_is_stored() {
        let (mut app, _dir) = app();
        let conn = postgres_saved(&mut app, PasswordMode::None);
        app.apply(Action::EditConnection(conn.clone()));
        assert_eq!(form(&mut app).password_mode, PasswordMode::Keyring);
        // Saved again untouched, it still has no password.
        app.apply(Action::SaveConnection { connect: false });
        assert_eq!(
            app.connections.get(&conn).unwrap().password,
            PasswordMode::None
        );
        assert!(sent_secrets(&app).is_empty());
        app.apply(Action::EditConnection(conn.clone()));
        form(&mut app).password = "pw".into();
        app.apply(Action::SaveConnection { connect: false });
        assert_eq!(
            app.connections.get(&conn).unwrap().password,
            PasswordMode::Keyring
        );
        assert!(app.backend.sent.iter().any(|c| matches!(
            c,
            Command::StoreSecret { secret: Some(SecretString(p)), account, .. }
                if p == "pw" && *account == password_account(&conn)
        )));
    }

    #[test]
    fn editing_an_ask_every_time_connection_keeps_asking() {
        let (mut app, _dir) = app();
        let conn = postgres_saved(&mut app, PasswordMode::Ask);
        app.apply(Action::EditConnection(conn.clone()));
        assert_eq!(form(&mut app).password_mode, PasswordMode::Ask);
        form(&mut app).name = "Renamed".into();
        app.apply(Action::SaveConnection { connect: false });
        assert_eq!(
            app.connections.get(&conn).unwrap().password,
            PasswordMode::Ask
        );
        assert!(sent_secrets(&app).is_empty());
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
        assert_eq!(filled.url, "", "the unmasked field keeps nothing");

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
            access: crate::testing::asked_access(app),
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
        assert!(workspace.tree.schemas.value.is_none() && workspace.tabs.is_empty());
        match app.backend.sent.last() {
            Some(Command::Connect { spec, .. }) => assert_eq!(spec.database, "other"),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn switching_database_keeps_sql_editors_and_drops_object_tabs() {
        let mut harness = Harness::new();
        let tab = connect_tab(&mut harness);
        let users = open(&mut harness, tab, "users", true);
        let first = harness.add_sql_tab(tab);
        open(&mut harness, tab, "orders", true);
        let second = harness.add_sql_tab(tab);
        start_run(&mut harness, tab, second);
        let workspace = harness.app.workspace_mut(tab).unwrap();
        workspace.server_version.value = Some("SQLite 3.46.0".into());
        workspace.active_tab = Some(users);
        workspace.sql_tab_mut(second).unwrap().text = "SELECT 1".into();
        assert!(workspace.sql_tab(second).unwrap().is_running());
        harness.app.apply(Action::SwitchDatabase {
            tab,
            database: "other".into(),
        });
        let workspace = harness.app.workspace(tab).unwrap();
        assert_eq!(strip(&harness, tab), vec![first, second]);
        assert_eq!(workspace.object_tabs().count(), 0);
        assert_eq!(
            workspace.active_tab,
            Some(first),
            "the object tab it showed is gone"
        );
        assert!(workspace.server_version.needs_load());
        assert_eq!(workspace.next_query, 3, "numbers are not handed out again");
        let sql = workspace.sql_tab(second).unwrap();
        assert_eq!(sql.text, "SELECT 1");
        // The old session is closed, so the run will never answer.
        assert!(!sql.is_running() && sql.in_flight.is_none());
        assert!(sql.running_for().is_none());
        assert!(matches!(
            last_sent(&harness.app),
            Command::Connect { spec, .. } if spec.database == "other"
        ));
    }

    #[test]
    fn switching_database_stays_on_the_sql_editor_it_showed() {
        let mut harness = Harness::new();
        let tab = connect_tab(&mut harness);
        harness.add_sql_tab(tab);
        open(&mut harness, tab, "users", true);
        let second = harness.add_sql_tab(tab);
        harness.app.workspace_mut(tab).unwrap().active_tab = Some(second);
        harness.app.apply(Action::SwitchDatabase {
            tab,
            database: "other".into(),
        });
        assert_eq!(harness.app.workspace(tab).unwrap().active_tab, Some(second));
    }

    #[test]
    fn switching_database_with_only_object_tabs_shows_none() {
        let mut harness = Harness::new();
        let tab = connect_tab(&mut harness);
        open(&mut harness, tab, "users", true);
        open(&mut harness, tab, "orders", false);
        harness.app.apply(Action::SwitchDatabase {
            tab,
            database: "other".into(),
        });
        let workspace = harness.app.workspace(tab).unwrap();
        assert!(workspace.tabs.is_empty());
        assert_eq!(workspace.active_tab, None);
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
            access: crate::testing::asked_access(&app),
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
                port: Some(22),
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
                    host: "bastion".into(),
                    port: 22,
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
                        host: "bastion".into(),
                        port: 22,
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

        /// A Host alias: the tunnel reached 10.0.0.5:2222 for "bastion".
        fn unknown_key_at_the_resolved_host() -> Error {
            Error::Ssh {
                stage: SshStage::HostKeyUnknown {
                    host: "10.0.0.5".into(),
                    port: 2222,
                    fingerprint: "SHA256:abc".into(),
                },
                message: "10.0.0.5 is not a trusted host yet".into(),
            }
        }

        #[test]
        fn connecting_trusts_the_host_and_port_the_tunnel_reached() {
            let (mut app, _dir) = app();
            let conn = ssh_saved(&mut app);
            let tab = app.active_tab_id();
            app.apply(Action::Connect { tab, conn });
            fail_last_connect(&mut app, unknown_key_at_the_resolved_host());
            match &app.dialog {
                Some(Dialog::HostKey(prompt)) => {
                    assert_eq!((prompt.host.as_str(), prompt.port), ("10.0.0.5", 2222))
                }
                other => panic!("{other:?}"),
            }
            app.apply(Action::TrustHostKey);
            assert_eq!(
                app.host_keys.fingerprint("10.0.0.5", 2222),
                Some("SHA256:abc")
            );
            assert_eq!(app.host_keys.fingerprint("bastion", 22), None);
        }

        #[test]
        fn testing_trusts_the_host_and_port_the_tunnel_reached() {
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
                result: Err(unknown_key_at_the_resolved_host()),
            }));
            match &super::form(&mut app).test {
                TestState::Untrusted { host, port, .. } => {
                    assert_eq!((host.as_str(), *port), ("10.0.0.5", 2222))
                }
                other => panic!("{other:?}"),
            }
            app.apply(Action::TrustTestHostKey);
            assert_eq!(
                app.host_keys.fingerprint("10.0.0.5", 2222),
                Some("SHA256:abc")
            );
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
                access: crate::testing::asked_access(&app),
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

        fn bastion_host() -> tabletist_db::ssh_config::ConfigHost {
            tabletist_db::ssh_config::ConfigHost {
                alias: "bastion".into(),
                config: tabletist_db::ssh_config::HostConfig {
                    identity_agent: Some(tabletist_db::ssh_config::AgentSocket::Environment),
                    ..Default::default()
                },
            }
        }

        #[test]
        fn opening_the_dialog_asks_for_the_config_hosts_and_takes_only_its_answer() {
            let (mut app, _dir) = app();
            app.apply(Action::NewConnection);
            let request = form(&mut app).ssh_hosts_request.expect("asked");
            app.apply(Action::Backend(Event::SshHosts {
                request: RequestId(request.0 + 1000),
                hosts: vec![bastion_host()],
            }));
            assert!(form(&mut app).ssh_hosts.is_empty(), "a stale answer");
            app.apply(Action::Backend(Event::SshHosts {
                request,
                hosts: vec![bastion_host()],
            }));
            assert_eq!(form(&mut app).ssh_hosts, vec![bastion_host()]);
            assert_eq!(form(&mut app).ssh_hosts_request, None);
        }

        #[test]
        fn editing_asks_for_the_config_hosts_too() {
            let (mut app, _dir) = app();
            let id = ssh_saved(&mut app);
            app.apply(Action::EditConnection(id));
            assert!(form(&mut app).ssh_hosts_request.is_some());
        }

        #[test]
        fn picking_a_config_host_fills_the_ssh_fields() {
            let (mut app, _dir) = app();
            postgres_form(&mut app);
            let form = form(&mut app);
            form.ssh_hosts = vec![bastion_host()];
            form.ssh_user = "someone".into();
            app.apply(Action::PickSshHost("bastion".into()));
            let form = super::form(&mut app);
            assert_eq!(form.ssh_host, "bastion");
            assert_eq!(form.ssh_user, "");
            assert_eq!(form.ssh_auth, SshAuthKind::Agent);
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
            let mut app = App::new(dirs, Settings::default().into(), Backend::recording());
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

    /// Count, filters, quick open, tree keys.
    mod power {
        use super::*;
        use crate::model::{FilterRow, Pane, TabId, TreeKey, TreeNode};
        use crate::testing::{Harness, page};
        use tabletist_db::{ObjectKind, ObjectRef};

        pub(super) fn open_users(harness: &mut Harness) -> (ConnTabId, TabId) {
            let tab = harness.connect_fake();
            harness.app.apply(Action::OpenObject {
                tab,
                object: ObjectRef::new("main", "users"),
                kind: ObjectKind::Table,
                pin: true,
            });
            harness.answer_rows(page(3, false));
            let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
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
            let object = workspace.object_tab(workspace.active_tab.unwrap()).unwrap();
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
            let object = workspace.object_tab(workspace.active_tab.unwrap()).unwrap();
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
                access: crate::testing::asked_access(&harness.app),
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
        fn retry_drops_the_page_under_the_error_but_keeps_the_columns() {
            let mut harness = Harness::new();
            let (tab, id) = open_users(&mut harness);
            harness.app.apply(Action::SelectCell {
                tab,
                id,
                cell: CellPos { row: 1, col: 0 },
            });
            harness.app.apply(Action::Refresh(tab));
            let (session, request) = match harness.app.backend.sent.last() {
                Some(Command::FetchRows {
                    session, request, ..
                }) => (*session, *request),
                other => panic!("{other:?}"),
            };
            harness.app.apply(Action::Backend(Event::Rows {
                session,
                request,
                result: Err(Error::query("connection reset")),
            }));
            assert!(object(&harness, tab, id).page().is_some(), "held");
            harness.app.apply(Action::RetryRows {
                tab,
                object_tab: id,
            });
            let object = object(&harness, tab, id);
            assert!(object.page().is_none(), "no page older than the error");
            assert_eq!(object.selection, None, "nor a cell of it");
            assert!(
                !object.filter.columns.is_empty(),
                "the bar still lists columns"
            );
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

    /// Editing a table's values: the pending set, the editor, the save and
    /// the guard that keeps a page with pending changes.
    mod editing {
        use super::*;
        use crate::edit::{Answer, Conflicting, EditorPlace, Lock, Problem, State};
        use crate::model::{Advance, EditStart};
        use tabletist_db::{Conflict, NewValue, Value, WriteOutcome};

        fn at(row: usize, col: usize) -> CellPos {
            CellPos { row, col }
        }

        /// Opens the editor on `cell`, sets its text as a field would, and
        /// commits.
        fn type_into(harness: &mut Harness, tab: ConnTabId, id: TabId, cell: CellPos, text: &str) {
            harness.app.apply(Action::EditCell {
                tab,
                id,
                cell,
                start: EditStart::Value,
            });
            let object = harness
                .app
                .workspace_mut(tab)
                .unwrap()
                .object_tab_mut(id)
                .unwrap();
            object.edits.editor.as_mut().expect("an editor").text = text.to_owned();
            // What a field says when its text changed.
            harness.app.apply(Action::EditorTyped { tab, id });
            harness.app.apply(Action::CommitEdit {
                tab,
                id,
                then: Advance::Stay,
            });
        }

        #[test]
        fn a_committed_edit_is_pending_and_the_loaded_text_takes_it_out_again() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            harness.app.apply(Action::EditCell {
                tab,
                id,
                cell: at(1, 1),
                start: EditStart::Value,
            });
            // The editor starts from the whole loaded value.
            let editor = object(&harness, tab, id).edits.editor.as_ref().unwrap();
            assert_eq!(
                (editor.cell, editor.text.as_str(), editor.large),
                (at(1, 1), "user2@example.com", false)
            );
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            let edits = &object(&harness, tab, id).edits;
            assert!(edits.editor.is_none());
            assert_eq!(
                edits.cells.get(&(1, 1)).map(|cell| &cell.new),
                Some(&NewValue::Text("bob@example.com".into()))
            );
            assert_eq!(edits.counts().changes, 1);
            // The page itself is untouched: nothing has been sent.
            assert_eq!(
                object(&harness, tab, id).page().unwrap().rows[1][1],
                tabletist_db::Value::Text("user2@example.com".into())
            );
            // Opened again, the editor holds the pending text.
            harness.app.apply(Action::EditCell {
                tab,
                id,
                cell: at(1, 1),
                start: EditStart::Value,
            });
            assert_eq!(
                object(&harness, tab, id)
                    .edits
                    .editor
                    .as_ref()
                    .unwrap()
                    .text,
                "bob@example.com"
            );
            // Typing the loaded text back is no change.
            type_into(&mut harness, tab, id, at(1, 1), "user2@example.com");
            assert!(object(&harness, tab, id).edits.cells.is_empty());
        }

        #[test]
        fn a_locked_cell_opens_no_editor_and_says_why() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            harness.app.apply(Action::EditCell {
                tab,
                id,
                cell: at(0, 0),
                start: EditStart::Value,
            });
            let edits = &object(&harness, tab, id).edits;
            assert!(edits.editor.is_none());
            assert_eq!(edits.why, Some((at(0, 0), Lock::KeyColumn)));
            // Moving on forgets the note.
            harness.app.apply(Action::SelectCell {
                tab,
                id,
                cell: at(0, 1),
            });
            assert_eq!(object(&harness, tab, id).edits.why, None);
            // A read-only connection locks every cell.
            harness.app.workspace_mut(tab).unwrap().access = tabletist_db::Access::ReadOnly;
            harness.app.apply(Action::EditCell {
                tab,
                id,
                cell: at(0, 1),
                start: EditStart::Value,
            });
            assert_eq!(
                object(&harness, tab, id).edits.why,
                Some((at(0, 1), Lock::ReadOnly))
            );
        }

        #[test]
        fn a_text_that_fails_its_check_stays_in_the_editor_and_is_kept_when_left() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            // `meta` is JSON, so its editor is the large one.
            type_into(&mut harness, tab, id, at(1, 2), "{oops");
            let editor = object(&harness, tab, id).edits.editor.as_ref().unwrap();
            assert!(editor.large);
            assert!(matches!(editor.problem, Some(Problem::Json { .. })));
            assert!(object(&harness, tab, id).edits.cells.is_empty());
            // Clicking elsewhere never loses the typing: the cell is to fix.
            harness.app.apply(Action::SelectCell {
                tab,
                id,
                cell: at(0, 1),
            });
            let edits = &object(&harness, tab, id).edits;
            assert!(edits.editor.is_none());
            assert!(matches!(
                edits.cells.get(&(1, 2)).map(|cell| &cell.state),
                Some(State::ToFix(Problem::Json { .. }))
            ));
            assert_eq!(edits.counts().to_fix, 1);
        }

        #[test]
        fn cancel_drops_the_edit_and_typed_marks_the_problem_as_it_is_typed() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            harness.app.apply(Action::EditCell {
                tab,
                id,
                cell: at(1, 2),
                start: EditStart::Replace("{".into()),
            });
            harness.app.apply(Action::EditorTyped { tab, id });
            assert!(
                object(&harness, tab, id)
                    .edits
                    .editor
                    .as_ref()
                    .unwrap()
                    .problem
                    .is_some()
            );
            harness.app.apply(Action::CancelEdit { tab, id });
            let edits = &object(&harness, tab, id).edits;
            assert!(edits.editor.is_none() && edits.cells.is_empty());
        }

        #[test]
        fn null_is_set_only_where_the_column_allows_it_and_one_cell_is_reverted() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            // `email` is NOT NULL: the key does nothing.
            harness.app.apply(Action::SelectCell {
                tab,
                id,
                cell: at(0, 1),
            });
            harness.app.apply(Action::SetNull { tab, id });
            assert!(object(&harness, tab, id).edits.cells.is_empty());
            // `meta` of the first row holds a value and may be NULL.
            harness.app.apply(Action::SelectCell {
                tab,
                id,
                cell: at(0, 2),
            });
            harness.app.apply(Action::SetNull { tab, id });
            assert_eq!(
                object(&harness, tab, id)
                    .edits
                    .cells
                    .get(&(0, 2))
                    .map(|cell| &cell.new),
                Some(&NewValue::Null)
            );
            // NULL on a cell that was NULL is no change.
            harness.app.apply(Action::SelectCell {
                tab,
                id,
                cell: at(1, 2),
            });
            harness.app.apply(Action::SetNull { tab, id });
            assert_eq!(object(&harness, tab, id).edits.counts().changes, 1);
            // Reverting the active cell puts the loaded value back.
            harness.app.apply(Action::SelectCell {
                tab,
                id,
                cell: at(0, 2),
            });
            harness.app.apply(Action::RevertCell { tab, id });
            assert!(object(&harness, tab, id).edits.cells.is_empty());
        }

        #[test]
        fn an_editor_that_was_only_opened_changes_nothing() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            // On a NULL cell the editor starts empty, and the empty string is
            // not NULL: only typing makes it a change.
            for then in [Advance::Stay, Advance::Down] {
                harness.app.apply(Action::EditCell {
                    tab,
                    id,
                    cell: at(1, 2),
                    start: EditStart::Value,
                });
                harness.app.apply(Action::CommitEdit { tab, id, then });
                assert!(object(&harness, tab, id).edits.cells.is_empty());
            }
            harness.app.apply(Action::EditCell {
                tab,
                id,
                cell: at(1, 2),
                start: EditStart::Value,
            });
            harness
                .app
                .apply(crate::testing::leave_edit(&harness.app, tab, id));
            assert!(!object(&harness, tab, id).edits.holds());
            // A cell made NULL stays NULL when its editor is opened and left.
            harness.app.apply(Action::SelectCell {
                tab,
                id,
                cell: at(0, 2),
            });
            harness.app.apply(Action::SetNull { tab, id });
            harness.app.apply(Action::EditCell {
                tab,
                id,
                cell: at(0, 2),
                start: EditStart::Value,
            });
            harness.app.apply(Action::CommitEdit {
                tab,
                id,
                then: Advance::Stay,
            });
            assert_eq!(
                object(&harness, tab, id)
                    .edits
                    .cells
                    .get(&(0, 2))
                    .map(|cell| &cell.new),
                Some(&NewValue::Null)
            );
        }

        #[test]
        fn typing_on_a_locked_cell_does_nothing() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            harness.app.apply(Action::EditCell {
                tab,
                id,
                cell: at(0, 0),
                start: EditStart::Typed("7".into()),
            });
            let edits = &object(&harness, tab, id).edits;
            assert!(edits.editor.is_none());
            assert_eq!(edits.why, None);
        }

        #[test]
        fn commit_moves_on_and_discard_empties_the_set() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            harness.app.apply(Action::EditCell {
                tab,
                id,
                cell: at(1, 1),
                start: EditStart::Replace("x@example.com".into()),
            });
            harness.app.apply(Action::CommitEdit {
                tab,
                id,
                then: Advance::Down,
            });
            assert_eq!(object(&harness, tab, id).selection, Some(at(2, 1)));
            type_into(&mut harness, tab, id, at(2, 1), "y@example.com");
            assert_eq!(
                (
                    object(&harness, tab, id).edits.counts().changes,
                    object(&harness, tab, id).edits.counts().rows
                ),
                (2, 2)
            );
            harness.app.apply(Action::DiscardEdits { tab, id });
            assert!(!object(&harness, tab, id).edits.holds());
        }

        #[test]
        fn editing_pins_a_preview_tab() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            harness
                .app
                .workspace_mut(tab)
                .unwrap()
                .object_tab_mut(id)
                .unwrap()
                .pinned = false;
            harness.app.apply(Action::EditCell {
                tab,
                id,
                cell: at(1, 1),
                start: EditStart::Value,
            });
            assert!(object(&harness, tab, id).pinned);
        }

        #[test]
        fn a_cell_made_null_pins_a_preview_tab_too() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            // A selection on a tab that is still a preview: a cell asked
            // for while it was locked is selected, and that pins nothing.
            {
                let object = harness
                    .app
                    .workspace_mut(tab)
                    .unwrap()
                    .object_tab_mut(id)
                    .unwrap();
                object.pinned = false;
                object.selection = Some(at(0, 2));
            }
            harness.app.apply(Action::SetNull { tab, id });
            assert_eq!(object(&harness, tab, id).edits.cells.len(), 1);
            assert!(object(&harness, tab, id).pinned);
            // So the next single click opens beside it, not over it.
            open(&mut harness, tab, "orders", false);
            assert_eq!(object(&harness, tab, id).edits.cells.len(), 1);
        }

        /// The newest `Write` sent, if any since `from`.
        fn write_since(harness: &Harness, from: usize) -> Option<&tabletist_db::ChangeSet> {
            harness.app.backend.sent[from..]
                .iter()
                .rev()
                .find_map(|command| match command {
                    Command::Write { changes, .. } => Some(changes),
                    _ => None,
                })
        }

        fn row(id: i64, email: &str) -> Vec<Value> {
            vec![Value::Int(id), Value::Text(email.into()), Value::Null]
        }

        #[test]
        fn a_save_sends_the_set_and_the_written_rows_replace_the_loaded_ones() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            type_into(&mut harness, tab, id, at(3, 1), "dan@example.com");
            let before = harness.app.backend.sent.len();
            harness.app.apply(Action::WriteEdits { tab, id });
            let changes = write_since(&harness, before).expect("a Write");
            assert_eq!(changes.object, users());
            assert_eq!(changes.rows.len(), 2);
            // While it runs the cells are locked, and the save is the tab's to
            // cancel.
            let saving = object(&harness, tab, id).edits.saving.as_ref().unwrap();
            assert_eq!(saving.rows, [1, 3]);
            assert!(
                object(&harness, tab, id)
                    .pending()
                    .any(|r| r == saving.request)
            );
            harness.app.apply(Action::EditCell {
                tab,
                id,
                cell: at(0, 1),
                start: EditStart::Value,
            });
            assert_eq!(
                object(&harness, tab, id).edits.why,
                Some((at(0, 1), Lock::Saving))
            );
            harness.answer_written(Ok(WriteOutcome::Written {
                rows: vec![row(2, "bob@example.com"), row(4, "dan@example.com")],
                elapsed: std::time::Duration::from_millis(14),
            }));
            let tab_now = object(&harness, tab, id);
            assert!(tab_now.edits.cells.is_empty() && tab_now.edits.saving.is_none());
            let page = tab_now.page().unwrap();
            assert_eq!(page.rows[1][1], Value::Text("bob@example.com".into()));
            assert_eq!(page.rows[3][1], Value::Text("dan@example.com".into()));
            let saved = tab_now.edits.saved.as_ref().unwrap();
            assert_eq!((saved.changes, saved.rows), (2, 2));
            assert_eq!(saved.cells, [at(1, 1), at(3, 1)]);
            // The row panel's text is formatted again.
            assert!(tab_now.fields.is_none());
        }

        #[test]
        fn a_conflict_a_failure_and_a_refusal_keep_the_set_and_say_what_happened() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            type_into(&mut harness, tab, id, at(3, 1), "dan@example.com");
            harness.app.apply(Action::WriteEdits { tab, id });
            harness.answer_written(Ok(WriteOutcome::Conflicts(vec![Conflict {
                row: 1,
                server: None,
            }])));
            let edits = &object(&harness, tab, id).edits;
            assert_eq!(edits.cells.len(), 2);
            // The conflict's row is the set's second, which is the page's
            // row 3. It is asked about, and no line says it too.
            assert_eq!(edits.note, None);
            match &harness.app.dialog {
                Some(Dialog::Conflict(prompt)) => assert_eq!(
                    prompt.rows,
                    [crate::edit::Conflicting {
                        row: 3,
                        server: None
                    }]
                ),
                other => panic!("expected the conflict question, got {other:?}"),
            }
            // Left as it is, the set stays.
            harness.app.apply(Action::AnswerConflict {
                at: 0,
                answer: crate::edit::Answer::KeepMine,
            });
            assert!(harness.app.dialog.is_none());
            assert_eq!(object(&harness, tab, id).edits.cells.len(), 2);
            harness.app.apply(Action::WriteEdits { tab, id });
            harness.answer_written(Ok(WriteOutcome::Failed {
                row: 0,
                error: tabletist_db::Error::query("violates check"),
            }));
            let edits = &object(&harness, tab, id).edits;
            assert!(matches!(
                edits.cells.get(&(1, 1)).map(|cell| &cell.state),
                Some(State::Failed(_))
            ));
            assert!(matches!(
                edits.cells.get(&(3, 1)).map(|cell| &cell.state),
                Some(State::Ready)
            ));
            // A failed cell does not block the next save.
            let before = harness.app.backend.sent.len();
            harness.app.apply(Action::WriteEdits { tab, id });
            assert!(write_since(&harness, before).is_some());
            harness.answer_written(Err(tabletist_db::Error::Cancelled));
            let edits = &object(&harness, tab, id).edits;
            assert_eq!(edits.note, Some(crate::edit::Note::Cancelled));
            assert_eq!(edits.cells.len(), 2);
        }

        #[test]
        fn a_save_is_not_sent_while_a_cell_is_to_fix_the_session_is_down_or_one_runs() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            type_into(&mut harness, tab, id, at(1, 2), "{oops");
            harness
                .app
                .apply(crate::testing::leave_edit(&harness.app, tab, id));
            let before = harness.app.backend.sent.len();
            harness.app.apply(Action::WriteEdits { tab, id });
            assert!(write_since(&harness, before).is_none());
            harness.app.apply(Action::DiscardEdits { tab, id });
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            // Disconnected: the set is kept, nothing is sent.
            let session = harness.app.workspace(tab).unwrap().session;
            harness.app.apply(Action::Backend(Event::Disconnected {
                session,
                error: tabletist_db::Error::ConnectionLost("gone".into()),
            }));
            harness.app.apply(Action::WriteEdits { tab, id });
            assert!(write_since(&harness, before).is_none());
            assert_eq!(object(&harness, tab, id).edits.cells.len(), 1);
        }

        #[test]
        fn the_row_panels_text_follows_the_pending_set() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            // The panel's text is made once the actions are applied.
            harness.app.apply_actions();
            let fields = object(&harness, tab, id).selected_fields();
            let fields = fields.expect("the selected row's text");
            assert_eq!(fields.fields[1].short, "bob@example.com");
            let was = |col: usize| fields.pending[col].as_ref().map(|cell| cell.was.as_str());
            assert_eq!(
                [was(0), was(1), was(2)],
                [None, Some("user2@example.com"), None]
            );
            // What was typed stays out of what a log or a panic prints.
            let printed = format!("{:?}", object(&harness, tab, id));
            assert!(!printed.contains("bob@example.com"), "{printed}");
            // NULL is a value the panel draws as one.
            harness.app.apply(Action::SelectCell {
                tab,
                id,
                cell: at(0, 2),
            });
            harness.app.apply(Action::SetNull { tab, id });
            harness.app.apply_actions();
            let fields = object(&harness, tab, id).selected_fields();
            let pending = fields.and_then(|fields| fields.pending[2].as_ref());
            let pending = pending.expect("the pending NULL");
            assert!(pending.new.is_null());
            assert_eq!(pending.was, r#"{"plan":"pro"}"#);
            // Taken back, the text is the page's again and nothing is kept.
            harness.app.apply(Action::RevertCell { tab, id });
            harness.app.apply_actions();
            let fields = object(&harness, tab, id).selected_fields();
            let fields = fields.expect("the selected row's text");
            assert!(fields.pending.is_empty());
            assert_eq!(fields.fields[2].short, r#"{"plan":"pro"}"#);
        }

        #[test]
        fn a_reconnect_under_a_save_abandons_it_and_keeps_the_set() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            harness.app.apply(Action::WriteEdits { tab, id });
            harness.app.apply(Action::Reconnect(tab));
            let edits = &object(&harness, tab, id).edits;
            assert!(edits.saving.is_none());
            assert_eq!(edits.note, Some(crate::edit::Note::Lost));
            assert_eq!(edits.cells.len(), 1);
        }

        #[test]
        fn a_written_row_of_another_width_fetches_the_page_again() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            harness.app.apply(Action::WriteEdits { tab, id });
            let before = harness.app.backend.sent.len();
            harness.answer_written(Ok(WriteOutcome::Written {
                rows: vec![vec![Value::Int(2)]],
                elapsed: std::time::Duration::ZERO,
            }));
            assert!(!object(&harness, tab, id).edits.holds());
            assert!(matches!(
                harness.app.backend.sent[before..].last(),
                Some(Command::FetchRows { .. })
            ));
        }

        #[test]
        fn a_new_page_forgets_the_last_save() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            harness
                .app
                .workspace_mut(tab)
                .unwrap()
                .object_tab_mut(id)
                .unwrap()
                .rows
                .value
                .as_mut()
                .unwrap()
                .has_more = true;
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            harness.app.apply(Action::WriteEdits { tab, id });
            harness.answer_written(Ok(WriteOutcome::Written {
                rows: vec![row(2, "bob@example.com")],
                elapsed: std::time::Duration::ZERO,
            }));
            // A cell that cannot be edited leaves its note.
            harness.app.apply(Action::EditCell {
                tab,
                id,
                cell: at(0, 0),
                start: EditStart::Value,
            });
            let edits = &object(&harness, tab, id).edits;
            assert!(edits.saved.is_some() && edits.why.is_some());
            // Neither holds the page: the next one is fetched without a question,
            // and both were about the page it replaces.
            harness.app.apply(Action::NextPage {
                tab,
                object_tab: id,
            });
            assert!(harness.app.dialog.is_none());
            harness.answer_rows(page(5, false));
            let edits = &object(&harness, tab, id).edits;
            assert!(edits.saved.is_none() && edits.why.is_none());
        }

        #[test]
        fn discard_and_revert_do_nothing_while_a_save_runs() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            harness.app.apply(Action::WriteEdits { tab, id });
            let request = object(&harness, tab, id)
                .edits
                .saving
                .as_ref()
                .unwrap()
                .request;
            // The pending cell is the active one, as a revert needs it.
            assert_eq!(object(&harness, tab, id).selection, Some(at(1, 1)));
            harness.app.apply(Action::RevertCell { tab, id });
            harness.app.apply(Action::DiscardEdits { tab, id });
            let edits = &object(&harness, tab, id).edits;
            assert_eq!(edits.cells.len(), 1);
            assert_eq!(
                edits.saving.as_ref().map(|saving| saving.request),
                Some(request)
            );
            // So the answer still finds the tab it was sent for.
            harness.answer_written(Ok(WriteOutcome::Written {
                rows: vec![row(2, "bob@example.com")],
                elapsed: std::time::Duration::ZERO,
            }));
            let tab_now = object(&harness, tab, id);
            assert!(tab_now.edits.cells.is_empty() && tab_now.edits.saved.is_some());
            assert_eq!(
                tab_now.page().unwrap().rows[1][1],
                Value::Text("bob@example.com".into())
            );
        }

        /// What makes an action for a tab, for a table of guarded ones.
        type Guarded = Box<dyn Fn(ConnTabId, TabId) -> Action>;

        /// Whether the Leave prompt is up, and whether it offers Save.
        fn leave_prompt(harness: &Harness) -> Option<bool> {
            match &harness.app.dialog {
                Some(Dialog::Leave(prompt)) => Some(prompt.can_save),
                _ => None,
            }
        }

        #[test]
        fn an_action_that_would_drop_the_page_is_held_until_the_user_chooses() {
            let guarded: Vec<(&str, Guarded)> = vec![
                (
                    "next page",
                    Box::new(|tab, object_tab| Action::NextPage { tab, object_tab }),
                ),
                (
                    "previous page",
                    Box::new(|tab, object_tab| Action::PrevPage { tab, object_tab }),
                ),
                (
                    "clear sort",
                    Box::new(|tab, object_tab| Action::ClearSort { tab, object_tab }),
                ),
                (
                    "retry",
                    Box::new(|tab, object_tab| Action::RetryRows { tab, object_tab }),
                ),
                (
                    "sort",
                    Box::new(|tab, object_tab| Action::SortBy {
                        tab,
                        object_tab,
                        column: "email".into(),
                    }),
                ),
                (
                    "filters",
                    Box::new(|tab, object_tab| Action::ApplyFilters { tab, object_tab }),
                ),
                (
                    "clear filters",
                    Box::new(|tab, object_tab| Action::ClearFilters { tab, object_tab }),
                ),
                ("refresh", Box::new(|tab, _| Action::Refresh(tab))),
                (
                    "retry structure",
                    Box::new(|tab, object_tab| Action::RetryStructure { tab, object_tab }),
                ),
                (
                    "close tab",
                    Box::new(|tab, id| Action::CloseTab { tab, id }),
                ),
                ("disconnect", Box::new(|tab, _| Action::Disconnect(tab))),
                (
                    "close connection",
                    Box::new(|tab, _| Action::CloseConnTab(tab)),
                ),
                (
                    "switch database",
                    Box::new(|tab, _| Action::SwitchDatabase {
                        tab,
                        database: "other".into(),
                    }),
                ),
            ];
            for (name, action) in guarded {
                let mut harness = Harness::new();
                let (tab, id) = harness.editable();
                // A page in the middle of a sorted table, so Next, Previous and
                // Clear sort each have something to do.
                {
                    let object = harness
                        .app
                        .workspace_mut(tab)
                        .unwrap()
                        .object_tab_mut(id)
                        .unwrap();
                    object.rows.value.as_mut().unwrap().has_more = true;
                    object.query.offset = 5;
                    object.query.sort = vec![tabletist_db::Sort {
                        column: "email".into(),
                        dir: tabletist_db::SortDir::Asc,
                    }];
                }
                type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
                let before = harness.app.backend.sent.len();
                harness.app.apply(action(tab, id));
                assert_eq!(leave_prompt(&harness), Some(true), "{name}");
                assert_eq!(
                    harness.app.backend.sent.len(),
                    before,
                    "{name}: nothing ran"
                );
                assert_eq!(object(&harness, tab, id).edits.cells.len(), 1, "{name}");
                // Staying drops the held action.
                harness.app.apply(Action::LeaveStay);
                assert!(harness.app.dialog.is_none(), "{name}");
                assert_eq!(harness.app.backend.sent.len(), before, "{name}");
                // Discard runs it.
                harness.app.apply(action(tab, id));
                harness.app.apply(Action::LeaveDiscard);
                assert!(harness.app.dialog.is_none(), "{name}");
                let gone = harness
                    .app
                    .workspace(tab)
                    .and_then(|workspace| workspace.object_tab(id))
                    .is_none_or(|object| !object.edits.holds());
                assert!(gone, "{name}: the set is dropped");
                // It ran: a command went out, or (closing a tab sends none when
                // nothing is pending) the tab is gone.
                let ran = harness.app.backend.sent.len() > before
                    || harness
                        .app
                        .workspace(tab)
                        .and_then(|workspace| workspace.object_tab(id))
                        .is_none();
                assert!(ran, "{name}: the action ran");
            }
        }

        #[test]
        fn an_action_that_would_do_nothing_is_not_held() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            // The last page, the first page, no sort: the arms do nothing, and
            // asking to discard for that would throw the set away for nothing.
            for action in [
                Action::NextPage {
                    tab,
                    object_tab: id,
                },
                Action::PrevPage {
                    tab,
                    object_tab: id,
                },
                Action::ClearSort {
                    tab,
                    object_tab: id,
                },
            ] {
                harness.app.apply(action);
                assert!(harness.app.dialog.is_none());
            }
            assert_eq!(object(&harness, tab, id).edits.cells.len(), 1);
        }

        #[test]
        fn what_does_not_drop_the_page_is_not_held() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            harness.app.apply(Action::SetView {
                tab,
                object_tab: id,
                view: ObjectView::Structure,
            });
            harness.app.apply(Action::OpenObject {
                tab,
                object: tabletist_db::ObjectRef::new("main", "orders"),
                kind: tabletist_db::ObjectKind::Table,
                pin: false,
            });
            harness.app.apply(Action::ActivateTab { tab, id });
            assert!(harness.app.dialog.is_none());
            assert_eq!(object(&harness, tab, id).edits.cells.len(), 1);
        }

        #[test]
        fn save_from_the_prompt_runs_the_held_action_only_when_everything_was_written() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            harness.app.apply(Action::CloseTab { tab, id });
            harness.app.apply(Action::LeaveSave);
            assert!(harness.app.dialog.is_none());
            // A conflict drops the held action: the tab stays, and once
            // the row is answered nothing asks about closing a second time.
            harness.answer_written(Ok(WriteOutcome::Conflicts(vec![Conflict {
                row: 0,
                server: None,
            }])));
            assert!(harness.app.workspace(tab).unwrap().object_tab(id).is_some());
            assert!(matches!(harness.app.dialog, Some(Dialog::Conflict(_))));
            harness.app.apply(Action::AnswerConflict {
                at: 0,
                answer: crate::edit::Answer::KeepMine,
            });
            assert!(harness.app.workspace(tab).unwrap().object_tab(id).is_some());
            assert!(harness.app.dialog.is_none());
            // Written: the tab closes.
            harness.app.apply(Action::CloseTab { tab, id });
            harness.app.apply(Action::LeaveSave);
            harness.answer_written(Ok(WriteOutcome::Written {
                rows: vec![row(2, "bob@example.com")],
                elapsed: std::time::Duration::ZERO,
            }));
            assert!(harness.app.workspace(tab).unwrap().object_tab(id).is_none());
        }

        #[test]
        fn the_prompt_offers_no_save_where_save_is_disabled_and_none_for_several_tabs() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            type_into(&mut harness, tab, id, at(1, 2), "{oops");
            harness
                .app
                .apply(crate::testing::leave_edit(&harness.app, tab, id));
            harness.app.apply(Action::CloseTab { tab, id });
            assert_eq!(leave_prompt(&harness), Some(false));
            harness.app.apply(Action::LeaveStay);
            // Two tabs with pending changes: Discard or Cancel only.
            harness.app.apply(Action::DiscardEdits { tab, id });
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            harness.app.apply(Action::OpenObject {
                tab,
                object: tabletist_db::ObjectRef::new("main", "orders"),
                kind: tabletist_db::ObjectKind::Table,
                pin: true,
            });
            harness.answer_structure(crate::testing::fixture_structure());
            harness.answer_rows(page(5, false));
            let other = harness.app.workspace(tab).unwrap().active_tab.unwrap();
            type_into(&mut harness, tab, other, at(0, 1), "x@example.com");
            harness.app.apply(Action::Disconnect(tab));
            assert_eq!(leave_prompt(&harness), Some(false));
        }

        /// Opens the editor on `cell` and sets its text as a field would:
        /// the editor stays open, typed into.
        fn typing(harness: &mut Harness, tab: ConnTabId, id: TabId, cell: CellPos, text: &str) {
            harness.app.apply(Action::EditCell {
                tab,
                id,
                cell,
                start: EditStart::Value,
            });
            let editor = harness.app.editor_mut(tab, id).expect("an editor");
            editor.text = text.to_owned();
            harness.app.apply(Action::EditorTyped { tab, id });
        }

        /// The fixture's table with two cells pending.
        fn two_pending() -> (Harness, ConnTabId, TabId) {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            type_into(&mut harness, tab, id, at(3, 1), "dan@example.com");
            (harness, tab, id)
        }

        /// How many changes the Leave prompt that is up asks about.
        fn asked_about(harness: &Harness) -> usize {
            match &harness.app.dialog {
                Some(Dialog::Leave(prompt)) => prompt.changes,
                other => panic!("expected the Leave prompt, got {other:?}"),
            }
        }

        /// Asks to close the tab, reads the count the Leave prompt gives,
        /// and answers it with Save: the count, and how many cells the save
        /// then holds as pending.
        fn counted_and_saved(harness: &mut Harness, tab: ConnTabId, id: TabId) -> (usize, usize) {
            harness.app.apply(Action::CloseTab { tab, id });
            let asked = asked_about(harness);
            harness.app.apply(Action::LeaveSave);
            (asked, object(harness, tab, id).edits.cells.len())
        }

        #[test]
        fn the_leave_prompt_counts_the_cell_being_edited() {
            // Two cells pending and an editor typed into on a third: its
            // text is part of what Save writes, and of what is asked about.
            let (mut harness, tab, id) = two_pending();
            typing(&mut harness, tab, id, at(4, 1), "eve@example.com");
            let before = harness.app.backend.sent.len();
            assert_eq!(counted_and_saved(&mut harness, tab, id), (3, 3));
            let sent = write_since(&harness, before).expect("a Write");
            let cells: usize = sent.rows.iter().map(|row| row.set.len()).sum();
            assert_eq!(cells, 3);
            // On one of the pending cells it is that cell, counted already.
            let (mut harness, tab, id) = two_pending();
            typing(&mut harness, tab, id, at(3, 1), "fay@example.com");
            assert_eq!(counted_and_saved(&mut harness, tab, id), (2, 2));
            // Typed into and left with the text the cell loaded, it is no
            // change.
            let (mut harness, tab, id) = two_pending();
            typing(&mut harness, tab, id, at(4, 1), "user5@example.com");
            let editor = object(&harness, tab, id).edits.editor.as_ref();
            assert!(editor.is_some_and(|editor| editor.touched));
            assert_eq!(counted_and_saved(&mut harness, tab, id), (2, 2));
            // Nor is an editor that was opened and not typed into.
            let (mut harness, tab, id) = two_pending();
            harness.app.apply(Action::EditCell {
                tab,
                id,
                cell: at(4, 1),
                start: EditStart::Value,
            });
            assert_eq!(counted_and_saved(&mut harness, tab, id), (2, 2));
            // A pending cell typed back to what it loaded leaves the set
            // when the editor closes: it is one change fewer.
            let (mut harness, tab, id) = two_pending();
            typing(&mut harness, tab, id, at(3, 1), "user4@example.com");
            assert_eq!(counted_and_saved(&mut harness, tab, id), (1, 1));
        }

        #[test]
        fn a_tab_that_holds_edits_never_counts_as_none() {
            // An editor that was only opened, with nothing pending.
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            harness.app.apply(Action::EditCell {
                tab,
                id,
                cell: at(1, 1),
                start: EditStart::Value,
            });
            harness.app.apply(Action::CloseTab { tab, id });
            assert_eq!(asked_about(&harness), 1);
            // One that was typed into is the one change, not a second.
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            typing(&mut harness, tab, id, at(1, 1), "bob@example.com");
            assert_eq!(counted_and_saved(&mut harness, tab, id), (1, 1));
        }

        #[test]
        fn a_text_its_column_refuses_counts_as_the_cell_to_fix_it_becomes() {
            // Left by the prompt's Save, the text stays as a cell to fix:
            // it is one of the tab's changes, though no save is sent.
            let (mut harness, tab, id) = two_pending();
            typing(&mut harness, tab, id, at(1, 2), "{oops");
            assert_eq!(counted_and_saved(&mut harness, tab, id), (3, 3));
            let edits = &object(&harness, tab, id).edits;
            assert!(matches!(edits.cells[&(1, 2)].state, State::ToFix(_)));
            assert_eq!(edits.counts().to_fix, 1);
            assert_eq!(writes(&harness), 0);
        }

        #[test]
        fn the_leave_prompt_sums_what_several_tabs_hold() {
            // Two pending and a third being typed in one tab, one pending
            // and an editor only opened on another cell in the next.
            let (mut harness, tab, id) = two_pending();
            typing(&mut harness, tab, id, at(4, 1), "eve@example.com");
            let orders = open(&mut harness, tab, "orders", true);
            harness.answer_structure(crate::testing::fixture_structure());
            harness.answer_rows(page(5, false));
            type_into(&mut harness, tab, orders, at(0, 1), "x@example.com");
            harness.app.apply(Action::EditCell {
                tab,
                id: orders,
                cell: at(2, 1),
                start: EditStart::Value,
            });
            for id in [id, orders] {
                assert!(object(&harness, tab, id).edits.editor.is_some());
            }
            harness.app.apply(Action::Disconnect(tab));
            assert_eq!(asked_about(&harness), 4);
            // A third whose editor was only opened is one more.
            harness.app.apply(Action::LeaveStay);
            let items = open(&mut harness, tab, "items", true);
            harness.answer_structure(crate::testing::fixture_structure());
            harness.answer_rows(page(5, false));
            harness.app.apply(Action::EditCell {
                tab,
                id: items,
                cell: at(0, 1),
                start: EditStart::Value,
            });
            harness.app.apply(Action::Disconnect(tab));
            assert_eq!(asked_about(&harness), 5);
        }

        #[test]
        fn a_guarded_action_is_ignored_while_the_save_runs_and_refused_under_another_dialog() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            harness.app.apply(Action::ShowHelp);
            harness.app.apply(Action::CloseTab { tab, id });
            assert!(matches!(harness.app.dialog, Some(Dialog::Help)));
            assert!(harness.app.notice.is_some());
            assert!(harness.app.workspace(tab).unwrap().object_tab(id).is_some());
            harness.app.apply(Action::CloseDialog);
            harness.app.apply(Action::WriteEdits { tab, id });
            harness.app.apply(Action::CloseTab { tab, id });
            assert!(harness.app.dialog.is_none());
            assert!(harness.app.workspace(tab).unwrap().object_tab(id).is_some());
        }

        #[test]
        fn a_tab_with_pending_changes_keeps_its_page_through_a_reconnect() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            harness.app.apply(Action::Reconnect(tab));
            let before = harness.app.backend.sent.len();
            answer_connect(&mut harness);
            assert!(
                !harness.app.backend.sent[before..]
                    .iter()
                    .any(|command| matches!(
                        command,
                        Command::FetchRows { .. } | Command::Describe { .. }
                    )),
                "the active tab is not read again"
            );
            assert_eq!(object(&harness, tab, id).edits.cells.len(), 1);
            // And a save works afterwards.
            let before = harness.app.backend.sent.len();
            harness.app.apply(Action::WriteEdits { tab, id });
            assert!(write_since(&harness, before).is_some());
        }

        #[test]
        fn a_save_to_production_is_asked_first_with_its_statements() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            harness.app.workspace_mut(tab).unwrap().environment =
                crate::env::Environment::Production;
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            let before = harness.app.backend.sent.len();
            harness.app.apply(Action::WriteEdits { tab, id });
            assert!(write_since(&harness, before).is_none());
            let Some(Dialog::ConfirmWrite(prompt)) = &harness.app.dialog else {
                panic!("expected the confirmation");
            };
            let lines: Vec<String> = prompt
                .review
                .lines
                .iter()
                .filter_map(crate::review::Line::sql)
                .collect();
            assert_eq!(
                lines,
                [
                    r#"UPDATE "main"."users""#,
                    r#"   SET "email" = 'bob@example.com'"#,
                    r#" WHERE "id" = 2;"#,
                ]
            );
            assert_eq!((prompt.changes, prompt.rows), (1, 1));
            // Cancel sends nothing and keeps the set.
            harness.app.apply(Action::CancelWrite);
            assert!(harness.app.dialog.is_none());
            assert!(write_since(&harness, before).is_none());
            assert_eq!(object(&harness, tab, id).edits.cells.len(), 1);
            harness.app.apply(Action::WriteEdits { tab, id });
            harness.app.apply(Action::ConfirmWrite);
            assert!(harness.app.dialog.is_none());
            assert!(write_since(&harness, before).is_some());
        }

        #[test]
        fn a_new_page_size_leaves_a_tab_with_pending_changes_alone() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            let orders = open(&mut harness, tab, "orders", true);
            harness.answer_rows(page(3, false));
            let before = harness.app.backend.sent.len();
            let settings = Settings {
                page_size: 500,
                ..harness.app.settings.clone()
            };
            harness.app.apply_settings(settings);
            // Only the tab without edits is fetched again.
            let fetched: Vec<&ObjectRef> = harness.app.backend.sent[before..]
                .iter()
                .filter_map(|command| match command {
                    Command::FetchRows { query, .. } => Some(&query.object),
                    _ => None,
                })
                .collect();
            assert_eq!(fetched, [&ObjectRef::new("main", "orders")]);
            assert_eq!(object(&harness, tab, orders).query.limit, 500);
            let users_tab = object(&harness, tab, id);
            assert_eq!(users_tab.edits.cells.len(), 1);
            assert_eq!(users_tab.page().unwrap().rows.len(), 5);
            assert_ne!(users_tab.query.limit, 500);
            // It takes the new size at its next fetch.
            harness.app.apply(Action::DiscardEdits { tab, id });
            harness.app.apply(Action::ActivateTab { tab, id });
            harness.app.apply(Action::Refresh(tab));
            assert_eq!(object(&harness, tab, id).query.limit, 500);
        }

        #[test]
        fn a_reconnect_that_comes_back_read_only_keeps_the_set_and_blocks_save() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            // "Open read-only" was turned on while the tab was open.
            let conn = harness.app.workspace(tab).unwrap().conn_id.clone();
            harness
                .app
                .connections
                .connections
                .iter_mut()
                .find(|saved| saved.id == conn)
                .unwrap()
                .read_only = Some(true);
            harness.app.apply(Action::Reconnect(tab));
            answer_connect(&mut harness);
            assert_eq!(
                harness.app.workspace(tab).unwrap().access,
                tabletist_db::Access::ReadOnly
            );
            assert_eq!(object(&harness, tab, id).edits.cells.len(), 1);
            assert_eq!(
                harness.app.save_blocked(tab, id),
                Some(crate::model::SaveBlock::ReadOnly)
            );
            let before = harness.app.backend.sent.len();
            harness.app.apply(Action::WriteEdits { tab, id });
            assert!(write_since(&harness, before).is_none());
            // Leaving offers no Save either.
            harness.app.apply(Action::CloseTab { tab, id });
            assert_eq!(leave_prompt(&harness), Some(false));
        }

        #[test]
        fn connecting_over_a_workspace_with_pending_changes_is_held_too() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            let before = harness.app.backend.sent.len();
            // A connection that is not saved any more: the arm does
            // nothing, so there is nothing to ask.
            harness.app.apply(Action::Connect {
                tab,
                conn: ConnectionId::new(),
            });
            assert!(harness.app.dialog.is_none());
            // The saved one would replace the workspace and every tab in it.
            let conn = harness.app.workspace(tab).unwrap().conn_id.clone();
            harness.app.apply(Action::Connect {
                tab,
                conn: conn.clone(),
            });
            assert_eq!(leave_prompt(&harness), Some(true));
            assert_eq!(harness.app.backend.sent.len(), before, "nothing ran");
            assert_eq!(object(&harness, tab, id).edits.cells.len(), 1);
            harness.app.apply(Action::LeaveDiscard);
            assert!(harness.app.workspace(tab).unwrap().object_tab(id).is_none());
            assert!(matches!(
                harness.app.backend.sent.last(),
                Some(Command::Connect { .. })
            ));
        }

        #[test]
        fn dropping_one_filter_is_held_too() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            let filter = tabletist_db::Filter {
                column: "email".into(),
                op: FilterOp::Contains,
                value: "example".into(),
            };
            harness
                .app
                .workspace_mut(tab)
                .unwrap()
                .object_tab_mut(id)
                .unwrap()
                .query
                .filters = vec![filter.clone()];
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            let before = harness.app.backend.sent.len();
            harness.app.apply(Action::DropFilter {
                tab,
                object_tab: id,
                index: 0,
            });
            assert_eq!(leave_prompt(&harness), Some(true));
            assert_eq!(harness.app.backend.sent.len(), before);
            assert_eq!(object(&harness, tab, id).query.filters, [filter]);
            assert_eq!(object(&harness, tab, id).edits.cells.len(), 1);
        }

        #[test]
        fn saving_from_the_prompt_with_only_an_untouched_editor_open_just_goes_on() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            harness.app.apply(Action::EditCell {
                tab,
                id,
                cell: at(1, 1),
                start: EditStart::Value,
            });
            let before = harness.app.backend.sent.len();
            // An open editor holds the page, so closing asks.
            harness.app.apply(Action::CloseTab { tab, id });
            assert_eq!(leave_prompt(&harness), Some(true));
            harness.app.apply(Action::LeaveSave);
            // Nothing was typed: there is nothing to save, and the tab closes.
            assert!(write_since(&harness, before).is_none());
            assert!(harness.app.dialog.is_none());
            assert!(harness.app.workspace(tab).unwrap().object_tab(id).is_none());
        }

        #[test]
        fn following_a_key_into_a_tab_with_pending_changes_is_held() {
            let mut harness = Harness::new();
            let (tab, users_tab) = harness.editable();
            let orders = open(&mut harness, tab, "orders", true);
            harness.answer_structure(crate::testing::fixture_structure());
            harness.answer_rows(page(5, false));
            type_into(&mut harness, tab, orders, at(0, 1), "x@example.com");
            harness
                .app
                .apply(Action::ActivateTab { tab, id: users_tab });
            let before = harness.app.backend.sent.len();
            let follow = || Action::FollowForeignKey {
                tab,
                object: ObjectRef::new("main", "orders"),
                column: "id".into(),
                value: "2".into(),
            };
            harness.app.apply(follow());
            assert_eq!(leave_prompt(&harness), Some(true));
            assert_eq!(harness.app.backend.sent.len(), before, "nothing ran");
            let held = object(&harness, tab, orders);
            assert!(held.filter.rows.is_empty() && held.query.filters.is_empty());
            assert_eq!(held.edits.cells.len(), 1);
            assert_eq!(held.page().unwrap().rows.len(), 5);
            // Staying keeps all of it.
            harness.app.apply(Action::LeaveStay);
            let held = object(&harness, tab, orders);
            assert!(held.filter.rows.is_empty() && held.edits.cells.len() == 1);
            // Discard follows the key: the tab is filtered to the row it names.
            harness.app.apply(follow());
            harness.app.apply(Action::LeaveDiscard);
            assert!(harness.app.dialog.is_none());
            let followed = object(&harness, tab, orders);
            assert!(!followed.edits.holds());
            assert_eq!(followed.query.filters.len(), 1);
            assert_eq!(followed.query.filters[0].value, "2");
            assert!(matches!(
                harness.app.backend.sent.last(),
                Some(Command::FetchRows { .. })
            ));
        }

        #[test]
        fn the_prompts_are_printed_without_what_was_typed() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            harness.app.workspace_mut(tab).unwrap().environment =
                crate::env::Environment::Production;
            type_into(&mut harness, tab, id, at(1, 1), "a-secret@example.com");
            harness.app.apply(Action::WriteEdits { tab, id });
            let printed = format!("{:?}", harness.app.dialog);
            assert!(printed.contains("ConfirmWrite"), "{printed}");
            assert!(!printed.contains("secret"), "{printed}");
            // The tab under it is printed without it too.
            let printed = format!("{:?}", object(&harness, tab, id));
            assert!(!printed.contains("secret"), "{printed}");
        }

        /// How many `Write`s were sent.
        fn writes(harness: &Harness) -> usize {
            let sent = harness.app.backend.sent.iter();
            sent.filter(|command| matches!(command, Command::Write { .. }))
                .count()
        }

        /// The terminal's `:` prompt as the workspace holds it: its text
        /// while it is open, and what it last refused.
        fn prompt(harness: &Harness, tab: ConnTabId) -> (Option<String>, Option<String>) {
            let workspace = harness.app.workspace(tab).unwrap();
            (workspace.command.clone(), workspace.command_error.clone())
        }

        /// Opens the prompt, types `text` as its field does, and runs it.
        fn run(harness: &mut Harness, tab: ConnTabId, text: &str) {
            harness.app.apply(Action::OpenCommand(tab));
            let workspace = harness.app.workspace_mut(tab).unwrap();
            *workspace.command.as_mut().expect("the prompt is open") = text.to_owned();
            harness.app.apply(Action::RunCommand(tab));
        }

        #[test]
        fn the_prompt_runs_w_and_e_bang_and_refuses_the_rest() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            let pending = |harness: &Harness| {
                let workspace = harness.app.workspace(tab).unwrap();
                workspace.object_tab(id).unwrap().edits.cells.len()
            };
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            // Anything else is not a command: it is kept to say so, and
            // nothing is sent or dropped.
            for text in ["wq", "W", "e", "w!", "e !", "diff!", "Diff"] {
                run(&mut harness, tab, text);
                assert_eq!(
                    prompt(&harness, tab),
                    (None, Some(text.to_owned())),
                    "{text}"
                );
                assert_eq!(writes(&harness), 0, "{text}");
                assert_eq!(pending(&harness), 1, "{text}");
            }
            // The prompt opens empty, and takes the message away.
            harness.app.apply(Action::OpenCommand(tab));
            assert_eq!(prompt(&harness, tab), (Some(String::new()), None));
            assert!(harness.app.workspace(tab).unwrap().focus_command);
            // Closed, it runs nothing.
            harness.app.workspace_mut(tab).unwrap().command = Some("e!".into());
            harness.app.apply(Action::CloseCommand(tab));
            assert_eq!(prompt(&harness, tab), (None, None));
            harness.app.apply(Action::RunCommand(tab));
            assert_eq!(pending(&harness), 1);
            // An empty line is no command and no mistake.
            run(&mut harness, tab, "  ");
            assert_eq!(prompt(&harness, tab), (None, None));
            // `w` writes, the spaces round it overlooked.
            run(&mut harness, tab, " w ");
            assert_eq!(prompt(&harness, tab), (None, None));
            assert_eq!(writes(&harness), 1);
            harness.answer_written(written("bob@example.com"));
            assert_eq!(pending(&harness), 0);
            // `e!` discards.
            type_into(&mut harness, tab, id, at(3, 1), "dan@example.com");
            run(&mut harness, tab, "e!");
            assert_eq!(pending(&harness), 0);
            assert_eq!(writes(&harness), 1);
            assert_eq!(prompt(&harness, tab), (None, None));
            // What was refused is taken away by closing too.
            run(&mut harness, tab, "nope");
            harness.app.apply(Action::CloseCommand(tab));
            assert_eq!(prompt(&harness, tab), (None, None));
            // With a SQL editor in front there is no table to write.
            type_into(&mut harness, tab, id, at(3, 1), "dan@example.com");
            harness.app.apply(Action::NewSqlTab(tab));
            run(&mut harness, tab, "w");
            run(&mut harness, tab, "e!");
            assert_eq!(writes(&harness), 1);
            assert_eq!(pending(&harness), 1);
            assert_eq!(prompt(&harness, tab), (None, None));
        }

        /// The statements of the tab's Review SQL, as the end of a frame
        /// leaves it: `None` while it is closed.
        fn reviewed(harness: &mut Harness, tab: ConnTabId, id: TabId) -> Option<Vec<String>> {
            // What a frame does once its actions are applied.
            harness.app.apply_actions();
            let review = object(harness, tab, id).edits.review.as_ref()?;
            Some(
                review
                    .lines
                    .iter()
                    .filter_map(crate::review::Line::sql)
                    .collect(),
            )
        }

        fn show_review(harness: &mut Harness, tab: ConnTabId, id: TabId, show: bool) {
            harness.app.apply(Action::ReviewEdits { tab, id, show });
        }

        #[test]
        fn review_sql_is_made_when_it_opens_and_again_when_the_set_changes() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            // Nothing pending: there is nothing to open it on.
            show_review(&mut harness, tab, id, true);
            assert!(!object(&harness, tab, id).edits.reviewing);
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            // Closed, it is not made: no statement is built for nobody.
            assert_eq!(reviewed(&mut harness, tab, id), None);
            show_review(&mut harness, tab, id, true);
            assert!(object(&harness, tab, id).edits.reviewing);
            let one = [
                r#"UPDATE "main"."users""#,
                r#"   SET "email" = 'bob@example.com'"#,
                r#" WHERE "id" = 2;"#,
            ];
            assert_eq!(reviewed(&mut harness, tab, id).unwrap(), one);
            let review = object(&harness, tab, id).edits.review.as_ref().unwrap();
            assert_eq!((review.changes, review.rows), (1, 1));
            // A frame that changes nothing makes nothing again: the review
            // is the one that was made.
            let lines = review.lines.as_ptr();
            harness.app.apply_actions();
            harness.app.apply(Action::SelectCell {
                tab,
                id,
                cell: at(3, 1),
            });
            harness.app.apply_actions();
            let review = object(&harness, tab, id).edits.review.as_ref().unwrap();
            assert_eq!(review.lines.as_ptr(), lines);
            // A second change: stale at once, and made again by the frame.
            type_into(&mut harness, tab, id, at(3, 1), "dan@example.com");
            assert!(object(&harness, tab, id).edits.review.is_none());
            let two = reviewed(&mut harness, tab, id).unwrap();
            assert_eq!(two.len(), 6);
            assert_eq!(two[4], r#"   SET "email" = 'dan@example.com'"#);
            // NULL, and the revert of one cell, change it too.
            harness.app.apply(Action::SelectCell {
                tab,
                id,
                cell: at(0, 2),
            });
            harness.app.apply(Action::SetNull { tab, id });
            let three = reviewed(&mut harness, tab, id).unwrap();
            assert_eq!(three[1], r#"   SET "meta" = NULL"#);
            harness.app.apply(Action::RevertCell { tab, id });
            assert_eq!(reviewed(&mut harness, tab, id).unwrap(), two);
            // A cell to fix blocks its row: the row has no statement.
            type_into(&mut harness, tab, id, at(3, 2), "{oops");
            harness
                .app
                .apply(crate::testing::leave_edit(&harness.app, tab, id));
            assert_eq!(reviewed(&mut harness, tab, id).unwrap(), one);
            let review = object(&harness, tab, id).edits.review.as_ref().unwrap();
            assert_eq!(
                review.lines.last(),
                Some(&crate::review::Line::Blocked {
                    row: "id 4".into(),
                    columns: vec!["meta".into()],
                })
            );
            // Hidden, it is dropped.
            show_review(&mut harness, tab, id, false);
            let edits = &object(&harness, tab, id).edits;
            assert!(!edits.reviewing && edits.review.is_none());
            assert_eq!(reviewed(&mut harness, tab, id), None);
        }

        #[test]
        fn showing_the_review_takes_what_is_being_typed() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            // Only an editor is open, and typed into: shown, the review
            // holds its text, as a save would send it.
            harness.app.apply(Action::EditCell {
                tab,
                id,
                cell: at(1, 1),
                start: EditStart::Replace("bob@example.com".into()),
            });
            show_review(&mut harness, tab, id, true);
            let edits = &object(&harness, tab, id).edits;
            assert!(edits.editor.is_none() && edits.reviewing);
            assert_eq!(
                reviewed(&mut harness, tab, id).unwrap()[1],
                r#"   SET "email" = 'bob@example.com'"#
            );
            // A text its column does not take is kept as a cell to fix,
            // and its row is blocked.
            harness.app.apply(Action::EditCell {
                tab,
                id,
                cell: at(3, 2),
                start: EditStart::Replace("{oops".into()),
            });
            assert!(object(&harness, tab, id).edits.editor.is_some());
            show_review(&mut harness, tab, id, true);
            assert!(object(&harness, tab, id).edits.editor.is_none());
            assert_eq!(object(&harness, tab, id).edits.counts().to_fix, 1);
            assert_eq!(reviewed(&mut harness, tab, id).unwrap().len(), 3);
            // Hiding it leaves an open editor alone.
            harness.app.apply(Action::EditCell {
                tab,
                id,
                cell: at(2, 1),
                start: EditStart::Replace("cy@example.com".into()),
            });
            show_review(&mut harness, tab, id, false);
            let edits = &object(&harness, tab, id).edits;
            assert!(edits.editor.is_some() && !edits.reviewing);
            // An editor that was only opened is no change: nothing is
            // pending, and nothing opens.
            harness.app.apply(Action::DiscardEdits { tab, id });
            harness.app.apply(Action::EditCell {
                tab,
                id,
                cell: at(1, 1),
                start: EditStart::Value,
            });
            show_review(&mut harness, tab, id, true);
            let edits = &object(&harness, tab, id).edits;
            assert!(edits.editor.is_none() && !edits.reviewing);
        }

        #[test]
        fn the_review_closes_with_the_set_and_stays_through_a_save_that_wrote_nothing() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            show_review(&mut harness, tab, id, true);
            let made = reviewed(&mut harness, tab, id).unwrap();
            // While the save runs it shows what was sent.
            harness.app.apply(Action::WriteEdits { tab, id });
            assert!(object(&harness, tab, id).edits.saving.is_some());
            assert_eq!(reviewed(&mut harness, tab, id).unwrap(), made);
            // A statement that failed leaves the set, and the review of it.
            harness.answer_written(Ok(WriteOutcome::Failed {
                row: 0,
                error: tabletist_db::Error::query("no"),
            }));
            assert_eq!(reviewed(&mut harness, tab, id).unwrap(), made);
            assert!(object(&harness, tab, id).edits.reviewing);
            // Written: nothing is pending, and nothing is reviewed.
            harness.app.apply(Action::WriteEdits { tab, id });
            harness.answer_written(written("bob@example.com"));
            let edits = &object(&harness, tab, id).edits;
            assert!(!edits.reviewing && edits.review.is_none());
            // It does not come back by itself with the next change.
            type_into(&mut harness, tab, id, at(3, 1), "dan@example.com");
            assert_eq!(reviewed(&mut harness, tab, id), None);
            // Discarded, and reverted to nothing, it closes as well.
            show_review(&mut harness, tab, id, true);
            harness.app.apply(Action::DiscardEdits { tab, id });
            assert!(!object(&harness, tab, id).edits.reviewing);
            type_into(&mut harness, tab, id, at(3, 1), "dan@example.com");
            show_review(&mut harness, tab, id, true);
            assert!(reviewed(&mut harness, tab, id).is_some());
            harness.app.apply(Action::SelectCell {
                tab,
                id,
                cell: at(3, 1),
            });
            harness.app.apply(Action::RevertCell { tab, id });
            // Open until the frame ends, with nothing to show.
            assert_eq!(reviewed(&mut harness, tab, id), None);
            assert!(!object(&harness, tab, id).edits.reviewing);
        }

        #[test]
        fn each_tab_has_its_own_review_and_keeps_it_while_another_shows() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            show_review(&mut harness, tab, id, true);
            let made = reviewed(&mut harness, tab, id).unwrap();
            let orders = open(&mut harness, tab, "orders", true);
            harness.answer_structure(crate::testing::fixture_structure());
            harness.answer_rows(page(3, false));
            type_into(&mut harness, tab, orders, at(0, 1), "eve@example.com");
            // The other tab's review is its own, and closed.
            assert!(!object(&harness, tab, orders).edits.reviewing);
            harness.app.apply_actions();
            assert!(object(&harness, tab, orders).edits.review.is_none());
            // Back on the first tab it is as it was left.
            harness.app.apply(Action::ActivateTab { tab, id });
            assert_eq!(reviewed(&mut harness, tab, id).unwrap(), made);
            // Without a session the statements are still there to read.
            harness.app.workspace_mut(tab).unwrap().status =
                crate::model::SessionStatus::Disconnected(tabletist_db::Error::ConnectionLost(
                    "reset".into(),
                ));
            type_into(&mut harness, tab, id, at(3, 1), "dan@example.com");
            assert_eq!(reviewed(&mut harness, tab, id).unwrap().len(), 6);
        }

        #[test]
        fn what_is_copied_holds_every_value_whole() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            assert!(harness.app.review_whole(tab, id).is_none());
            let long = "x".repeat(100);
            type_into(&mut harness, tab, id, at(1, 1), &long);
            show_review(&mut harness, tab, id, true);
            // Shown, the value is cut at sixty characters.
            let shown = reviewed(&mut harness, tab, id).unwrap();
            assert_eq!(
                shown[1],
                format!(r#"   SET "email" = '{}…'"#, "x".repeat(57))
            );
            // Whole, it is the statement that runs. Asked for with the
            // review closed too: the keys that copy do not open it.
            for show in [true, false] {
                show_review(&mut harness, tab, id, show);
                let whole = harness.app.review_whole(tab, id).unwrap();
                let lines: Vec<String> = whole
                    .lines
                    .iter()
                    .filter_map(crate::review::Line::sql)
                    .collect();
                assert_eq!(lines[1], format!(r#"   SET "email" = '{long}'"#));
            }
            // Asking for it leaves the tab's own review as it was.
            show_review(&mut harness, tab, id, true);
            assert_eq!(reviewed(&mut harness, tab, id).unwrap(), shown);
        }

        #[test]
        fn a_cell_that_is_as_it_loaded_again_leaves_the_review_stale() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            type_into(&mut harness, tab, id, at(2, 2), "{}");
            type_into(&mut harness, tab, id, at(3, 1), "dan@example.com");
            show_review(&mut harness, tab, id, true);
            assert_eq!(reviewed(&mut harness, tab, id).unwrap().len(), 9);
            // The loaded text typed back is no change any more.
            type_into(&mut harness, tab, id, at(3, 1), "user4@example.com");
            assert!(object(&harness, tab, id).edits.review.is_none());
            assert_eq!(reviewed(&mut harness, tab, id).unwrap().len(), 6);
            // Nor is NULL for a cell that loaded NULL.
            harness.app.apply(Action::SelectCell {
                tab,
                id,
                cell: at(2, 2),
            });
            harness.app.apply(Action::SetNull { tab, id });
            assert!(object(&harness, tab, id).edits.review.is_none());
            assert_eq!(reviewed(&mut harness, tab, id).unwrap().len(), 3);
        }

        #[test]
        fn a_page_that_takes_the_tabs_place_closes_its_review() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            show_review(&mut harness, tab, id, true);
            assert!(reviewed(&mut harness, tab, id).is_some());
            // Another page is asked for: under the question the review
            // stands, and its Discard drops it with the set.
            harness.app.apply(Action::SortBy {
                tab,
                object_tab: id,
                column: "email".into(),
            });
            assert_eq!(leave_prompt(&harness), Some(true));
            assert!(object(&harness, tab, id).edits.review.is_some());
            harness.app.apply(Action::LeaveDiscard);
            let edits = &object(&harness, tab, id).edits;
            assert!(!edits.reviewing && edits.review.is_none());
            harness.answer_rows(page(5, false));
            assert_eq!(reviewed(&mut harness, tab, id), None);
            // Whatever brought a page: a review made of the rows it
            // replaces does not outlive them.
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            show_review(&mut harness, tab, id, true);
            assert!(reviewed(&mut harness, tab, id).is_some());
            harness.app.fetch_rows(tab, id);
            harness.answer_rows(page(5, false));
            let edits = &object(&harness, tab, id).edits;
            assert!(!edits.reviewing && edits.review.is_none());
            assert_eq!(reviewed(&mut harness, tab, id), None);
        }

        #[test]
        fn a_structure_that_arrives_makes_the_review_again() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            show_review(&mut harness, tab, id, true);
            assert_eq!(reviewed(&mut harness, tab, id).unwrap().len(), 3);
            // Nothing describes a table under pending cells: the guard
            // holds what would. Were a structure to arrive all the same,
            // the review is of the one before it: it is forced here, and
            // comes without the key the statements find their rows by.
            harness.app.describe(tab, id);
            let Some(Command::Describe {
                session, request, ..
            }) = harness.app.backend.sent.last()
            else {
                panic!("a describe was sent");
            };
            let keyless = tabletist_db::Structure {
                primary_key: Vec::new(),
                ..crate::testing::fixture_structure()
            };
            harness.app.actions.push(Action::Backend(Event::Structure {
                session: *session,
                request: *request,
                result: Ok(keyless),
            }));
            // In the round of actions that brought it, as a frame runs
            // them: no frame draws statements the structure no longer
            // makes.
            harness.app.apply_actions();
            let edits = &object(&harness, tab, id).edits;
            assert!(edits.reviewing);
            assert_eq!(
                edits.review.as_ref().map(|review| review.lines.as_slice()),
                Some([crate::review::Line::Unsendable].as_slice())
            );
            assert_eq!(edits.cells.len(), 1);
        }

        #[test]
        fn the_prompt_shows_the_review_with_diff_and_says_when_nothing_is_pending() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            let refused = |harness: &Harness| harness.app.workspace(tab).unwrap().review_refused;
            // Nothing pending: nothing opens, and the line says so. It is
            // no mistake of the typing.
            run(&mut harness, tab, "diff");
            assert!(refused(&harness));
            assert_eq!(prompt(&harness, tab), (None, None));
            assert!(!object(&harness, tab, id).edits.reviewing);
            // The prompt takes the message away, opened or closed.
            harness.app.apply(Action::OpenCommand(tab));
            assert!(!refused(&harness));
            run(&mut harness, tab, "diff");
            harness.app.apply(Action::CloseCommand(tab));
            assert!(!refused(&harness));
            // And so does an edit begun without a key: a double-click.
            run(&mut harness, tab, "diff");
            harness.app.apply(Action::EditCell {
                tab,
                id,
                cell: at(2, 1),
                start: EditStart::Value,
            });
            assert!(!refused(&harness));
            harness.app.apply(Action::CancelEdit { tab, id });
            // With something pending it opens, the spaces round it
            // overlooked, and sends nothing.
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            run(&mut harness, tab, " diff ");
            assert!(!refused(&harness));
            assert_eq!(prompt(&harness, tab), (None, None));
            assert!(object(&harness, tab, id).edits.reviewing);
            assert_eq!(reviewed(&mut harness, tab, id).unwrap().len(), 3);
            assert_eq!(writes(&harness), 0);
            // Again, it stays open: only Esc closes it.
            run(&mut harness, tab, "diff");
            assert!(object(&harness, tab, id).edits.reviewing);
            // With a SQL editor in front there is no table to review.
            show_review(&mut harness, tab, id, false);
            harness.app.apply(Action::NewSqlTab(tab));
            run(&mut harness, tab, "diff");
            assert!(!object(&harness, tab, id).edits.reviewing);
            assert!(!refused(&harness));
            assert_eq!(prompt(&harness, tab), (None, None));
        }

        #[test]
        fn the_confirmation_holds_its_review_and_the_terminal_look_opens_the_panel() {
            let lines = |review: &crate::review::Review| -> Vec<String> {
                let lines = review.lines.iter();
                lines.filter_map(crate::review::Line::sql).collect()
            };
            // macOS and Windows: the sheet lists the statements itself, and
            // no drawer opens behind it.
            let mut harness = Harness::new();
            assert!(!harness.app.look.terminal);
            let (tab, id) = production(&mut harness);
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            harness.app.apply(Action::WriteEdits { tab, id });
            let Some(Dialog::ConfirmWrite(prompt)) = &harness.app.dialog else {
                panic!("expected the confirmation");
            };
            assert_eq!(lines(&prompt.review).len(), 3);
            assert!(!object(&harness, tab, id).edits.reviewing);
            // The terminal look: its box points at the panel, so the panel
            // is open under it, with the review the box was made with.
            let mut harness = Harness::new();
            harness.set_look(crate::theme::Look::omarchy());
            let (tab, id) = production(&mut harness);
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            harness.app.apply(Action::WriteEdits { tab, id });
            let Some(Dialog::ConfirmWrite(prompt)) = &harness.app.dialog else {
                panic!("expected the confirmation");
            };
            let edits = &object(&harness, tab, id).edits;
            assert!(edits.reviewing);
            assert_eq!(edits.review.as_ref(), Some(&prompt.review));
            // Under the confirmation the panel is not closed.
            show_review(&mut harness, tab, id, false);
            assert!(object(&harness, tab, id).edits.reviewing);
            // Cancelled, it stays: the statements were just declined, and
            // are still what is pending.
            harness.app.apply(Action::CancelWrite);
            assert!(harness.app.dialog.is_none());
            assert!(object(&harness, tab, id).edits.reviewing);
            assert_eq!(reviewed(&mut harness, tab, id).unwrap().len(), 3);
            // Confirmed and written, it closes with the set.
            harness.app.apply(Action::WriteEdits { tab, id });
            harness.app.apply(Action::ConfirmWrite);
            harness.answer_written(written("bob@example.com"));
            assert!(!object(&harness, tab, id).edits.reviewing);
        }

        #[test]
        fn a_confirmed_save_is_sent_though_a_loaded_value_is_not_a_number() {
            // A table with a float column, on production. The changed
            // cell loaded NaN, which a PostgreSQL float can hold and which
            // is not equal to itself.
            let mut harness = Harness::new();
            let tab = harness.connect_fake_as(false);
            harness.app.apply(Action::OpenObject {
                tab,
                object: tabletist_db::ObjectRef::new("main", "users"),
                kind: ObjectKind::Table,
                pin: true,
            });
            let mut structure = crate::testing::fixture_structure();
            structure.columns.push(tabletist_db::ColumnInfo {
                name: "level".into(),
                type_name: "REAL".into(),
                nullable: true,
                ..tabletist_db::ColumnInfo::default()
            });
            harness.answer_structure(structure);
            let mut page = crate::testing::page(5, false);
            page.columns.push(tabletist_db::ColumnMeta {
                name: "level".into(),
                type_name: "REAL".into(),
                kind: tabletist_db::ValueKind::Numeric,
            });
            for row in &mut page.rows {
                row.push(Value::Float(f64::NAN));
            }
            harness.answer_rows(page);
            let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
            harness.app.workspace_mut(tab).unwrap().environment =
                crate::env::Environment::Production;
            type_into(&mut harness, tab, id, at(1, 3), "2.5");
            harness.app.apply(Action::WriteEdits { tab, id });
            let shown = confirming(&harness).expect("the confirmation").clone();
            assert!(matches!(
                shown.rows[0].set[0].loaded,
                Value::Float(loaded) if loaded.is_nan()
            ));
            assert_eq!(writes(&harness), 0);
            // The set is the one the confirmation shows: it is sent.
            harness.app.apply(Action::ConfirmWrite);
            assert!(harness.app.dialog.is_none());
            assert_eq!(writes(&harness), 1);
            let Some(Command::Write { changes, .. }) = harness.app.backend.sent.last() else {
                panic!("a save was sent");
            };
            assert!(crate::edit::same_changes(changes, &shown));
            // One that changed under the confirmation is still not sent.
            harness.answer_written(Err(tabletist_db::Error::Cancelled));
            harness.app.apply(Action::WriteEdits { tab, id });
            assert!(confirming(&harness).is_some());
            let object = harness.app.object_tab_mut(tab, id).unwrap();
            object.rows.value.as_mut().unwrap().rows[1][3] = Value::Float(0.5);
            harness.app.apply(Action::ConfirmWrite);
            assert!(harness.app.dialog.is_none());
            assert_eq!(writes(&harness), 1);
        }

        #[test]
        fn the_prompt_changes_nothing_under_a_question_about_the_changes() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            // The tab is asked to close, and the question is up.
            harness.app.apply(Action::CloseTab { tab, id });
            assert!(matches!(harness.app.dialog, Some(Dialog::Leave(_))));
            // No prompt opens under a dialog: the keys do not run there.
            harness.app.apply(Action::OpenCommand(tab));
            assert_eq!(prompt(&harness, tab), (None, None));
            // One that was open when the question came runs nothing: what
            // is asked about stays as it is.
            for text in ["e!", "w"] {
                harness.app.workspace_mut(tab).unwrap().command = Some(text.into());
                harness.app.apply(Action::RunCommand(tab));
                assert_eq!(prompt(&harness, tab), (None, None), "{text}");
                let workspace = harness.app.workspace(tab).unwrap();
                let edits = &workspace.object_tab(id).unwrap().edits;
                assert_eq!(edits.cells.len(), 1, "{text}");
                assert!(edits.saving.is_none(), "{text}");
                assert_eq!(writes(&harness), 0, "{text}");
                assert!(matches!(harness.app.dialog, Some(Dialog::Leave(_))));
            }
        }

        /// `Harness::editable` on a connection to production, whose saves
        /// are confirmed first.
        fn production(harness: &mut Harness) -> (ConnTabId, TabId) {
            let (tab, id) = harness.editable();
            harness.app.workspace_mut(tab).unwrap().environment =
                crate::env::Environment::Production;
            (tab, id)
        }

        /// A set's table and rows, to compare two sets by: a set is printed
        /// by its counts alone, and a comparison that fails here should
        /// say which rows it found.
        fn held(
            changes: Option<&tabletist_db::ChangeSet>,
        ) -> Option<(&ObjectRef, &[tabletist_db::RowChange])> {
            changes.map(|changes| (&changes.object, changes.rows.as_slice()))
        }

        /// The set the production confirmation shows.
        fn confirming(harness: &Harness) -> Option<&tabletist_db::ChangeSet> {
            match &harness.app.dialog {
                Some(Dialog::ConfirmWrite(prompt)) => Some(&prompt.changeset),
                _ => None,
            }
        }

        /// Asks for `text` on `cell` as an editor that was opened with it
        /// and left does. Unlike `type_into` it asks only: under a prompt
        /// nothing of it may happen.
        fn change(harness: &mut Harness, tab: ConnTabId, id: TabId, cell: CellPos, text: &str) {
            harness.app.apply(Action::EditCell {
                tab,
                id,
                cell,
                start: EditStart::Replace(text.into()),
            });
            harness
                .app
                .apply(crate::testing::leave_edit(&harness.app, tab, id));
        }

        fn written(email: &str) -> Result<WriteOutcome, tabletist_db::Error> {
            Ok(WriteOutcome::Written {
                rows: vec![row(2, email)],
                elapsed: std::time::Duration::ZERO,
            })
        }

        #[test]
        fn a_change_asked_for_under_the_confirmation_is_not_made() {
            let mut harness = Harness::new();
            let (tab, id) = production(&mut harness);
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            // The active cell holds a value and may be NULL.
            harness.app.apply(Action::SelectCell {
                tab,
                id,
                cell: at(0, 2),
            });
            harness.app.apply(Action::WriteEdits { tab, id });
            let shown = confirming(&harness).expect("the confirmation").clone();
            // The keys still run under a dialog: a second change, a NULL, a
            // move of the selection.
            change(&mut harness, tab, id, at(3, 1), "dan@example.com");
            harness.app.apply(Action::SetNull { tab, id });
            harness.app.apply(Action::SelectCell {
                tab,
                id,
                cell: at(2, 1),
            });
            harness.app.apply(Action::MoveSelection {
                tab,
                id,
                rows: 1,
                cols: 0,
            });
            let under = object(&harness, tab, id);
            assert_eq!(under.selection, Some(at(0, 2)));
            assert_eq!(under.edits.cells.keys().collect::<Vec<_>>(), [&(1, 1)]);
            assert!(under.edits.editor.is_none());
            // The save is the one that was shown, and what it wrote is all
            // that is marked saved.
            harness.app.apply(Action::ConfirmWrite);
            assert_eq!(writes(&harness), 1);
            assert_eq!(held(write_since(&harness, 0)), held(Some(&shown)));
            harness.answer_written(written("bob@example.com"));
            let after = object(&harness, tab, id);
            assert_eq!(after.edits.saved.as_ref().unwrap().cells, [at(1, 1)]);
            assert!(after.edits.cells.is_empty());
            assert_eq!(
                after.page().unwrap().rows[3][1],
                Value::Text("user4@example.com".into())
            );
        }

        #[test]
        fn a_discard_asked_for_under_the_confirmation_discards_nothing() {
            let mut harness = Harness::new();
            let (tab, id) = production(&mut harness);
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            harness.app.apply(Action::WriteEdits { tab, id });
            let shown = confirming(&harness).expect("the confirmation").clone();
            // The pending cell is the active one, as a revert needs it.
            harness.app.apply(Action::RevertCell { tab, id });
            harness.app.apply(Action::DiscardEdits { tab, id });
            // Nor does a second save ask a second time.
            harness.app.apply(Action::WriteEdits { tab, id });
            assert_eq!(object(&harness, tab, id).edits.cells.len(), 1);
            assert_eq!(held(confirming(&harness)), held(Some(&shown)));
            // So what is confirmed is still there to be saved.
            harness.app.apply(Action::ConfirmWrite);
            assert_eq!(writes(&harness), 1);
            assert_eq!(held(write_since(&harness, 0)), held(Some(&shown)));
        }

        #[test]
        fn a_save_asked_for_under_the_leave_prompt_is_not_sent() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            harness.app.apply(Action::CloseTab { tab, id });
            assert_eq!(leave_prompt(&harness), Some(true));
            harness.app.apply(Action::WriteEdits { tab, id });
            assert_eq!(writes(&harness), 0);
            assert_eq!(leave_prompt(&harness), Some(true));
            assert!(object(&harness, tab, id).edits.saving.is_none());
            // So Discard closes a tab that has no save in flight.
            harness.app.apply(Action::LeaveDiscard);
            assert!(harness.app.workspace(tab).unwrap().object_tab(id).is_none());
            assert_eq!(writes(&harness), 0);
        }

        #[test]
        fn no_editor_opens_under_the_confirmation() {
            let mut harness = Harness::new();
            let (tab, id) = production(&mut harness);
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            harness.app.apply(Action::WriteEdits { tab, id });
            for start in [
                EditStart::Value,
                EditStart::Replace("x".into()),
                EditStart::Typed("x".into()),
            ] {
                harness.app.apply(Action::EditCell {
                    tab,
                    id,
                    cell: at(3, 1),
                    start,
                });
                assert!(object(&harness, tab, id).edits.editor.is_none());
            }
            // So none is left to add a cell to a save that is running.
            harness.app.apply(Action::ConfirmWrite);
            harness
                .app
                .apply(crate::testing::leave_edit(&harness.app, tab, id));
            let saving = object(&harness, tab, id);
            assert!(saving.edits.saving.is_some());
            assert_eq!(saving.edits.cells.len(), 1);
            harness.answer_written(written("bob@example.com"));
            assert!(!object(&harness, tab, id).edits.holds());
        }

        #[test]
        fn an_editor_under_the_leave_prompt_stays_as_it_was() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            harness.app.apply(Action::EditCell {
                tab,
                id,
                cell: at(1, 1),
                start: EditStart::Value,
            });
            harness.app.apply(Action::CloseTab { tab, id });
            assert_eq!(leave_prompt(&harness), Some(true));
            harness.app.apply(Action::EditorBreak { tab, id });
            let editor = object(&harness, tab, id).edits.editor.as_ref().unwrap();
            assert_eq!(editor.text, "user2@example.com");
            assert!(!editor.touched && !editor.large);
            // What the field says of its text is heard all the same: it
            // changes nothing but what the editor knows of it.
            harness.app.apply(Action::EditorTyped { tab, id });
            let editor = object(&harness, tab, id).edits.editor.as_ref().unwrap();
            assert_eq!(editor.text, "user2@example.com");
            assert!(editor.touched && !editor.large);
            for closes in [
                Action::CommitEdit {
                    tab,
                    id,
                    then: Advance::Down,
                },
                crate::testing::leave_edit(&harness.app, tab, id),
                Action::CancelEdit { tab, id },
            ] {
                harness.app.apply(closes);
                assert!(object(&harness, tab, id).edits.editor.is_some());
            }
            assert_eq!(object(&harness, tab, id).selection, Some(at(1, 1)));
            // Staying finds it open.
            harness.app.apply(Action::LeaveStay);
            assert!(object(&harness, tab, id).edits.editor.is_some());
        }

        #[test]
        fn the_review_is_neither_shown_nor_hidden_under_a_prompt() {
            // Under the confirmation it does not open.
            let mut harness = Harness::new();
            let (tab, id) = production(&mut harness);
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            harness.app.apply(Action::WriteEdits { tab, id });
            let shown = confirming(&harness).expect("the confirmation").clone();
            show_review(&mut harness, tab, id, true);
            assert_eq!(reviewed(&mut harness, tab, id), None);
            assert!(!object(&harness, tab, id).edits.reviewing);
            harness.app.apply(Action::CancelWrite);
            // Open, it does not close under it.
            show_review(&mut harness, tab, id, true);
            let lines = reviewed(&mut harness, tab, id).expect("the review");
            harness.app.apply(Action::WriteEdits { tab, id });
            show_review(&mut harness, tab, id, false);
            assert!(object(&harness, tab, id).edits.reviewing);
            assert_eq!(reviewed(&mut harness, tab, id), Some(lines.clone()));
            assert_eq!(held(confirming(&harness)), held(Some(&shown)));
            harness.app.apply(Action::CancelWrite);
            // Nor under the question before the changes are dropped.
            harness.app.apply(Action::CloseTab { tab, id });
            assert!(leave_prompt(&harness).is_some());
            show_review(&mut harness, tab, id, false);
            assert_eq!(reviewed(&mut harness, tab, id), Some(lines));
            harness.app.apply(Action::LeaveStay);
            show_review(&mut harness, tab, id, false);
            assert_eq!(reviewed(&mut harness, tab, id), None);
            // Shown, the review would close an open editor as a left edit,
            // and the set the question counted would be another.
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            harness.app.apply(Action::EditCell {
                tab,
                id,
                cell: at(3, 1),
                start: EditStart::Replace("dan@example.com".into()),
            });
            harness.app.apply(Action::CloseTab { tab, id });
            assert!(leave_prompt(&harness).is_some());
            show_review(&mut harness, tab, id, true);
            let edits = &object(&harness, tab, id).edits;
            assert!(edits.editor.is_some() && !edits.reviewing);
            assert_eq!(edits.cells.len(), 1);
            assert_eq!(reviewed(&mut harness, tab, id), None);
            // Staying, it opens as it does anywhere, the editor's text in
            // it.
            harness.app.apply(Action::LeaveStay);
            show_review(&mut harness, tab, id, true);
            assert_eq!(object(&harness, tab, id).edits.cells.len(), 2);
            assert_eq!(reviewed(&mut harness, tab, id).unwrap().len(), 6);
        }

        #[test]
        fn a_first_keystroke_under_a_prompt_that_just_opened_is_saved() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            harness.app.apply(Action::EditCell {
                tab,
                id,
                cell: at(1, 1),
                start: EditStart::Value,
            });
            // One frame's actions, in their order: the key that asks to
            // close the tab, read before anything is drawn, then the field
            // that took a keystroke in that frame.
            harness.app.apply(Action::CloseTab { tab, id });
            assert_eq!(leave_prompt(&harness), Some(true));
            let editor = harness
                .app
                .workspace_mut(tab)
                .unwrap()
                .object_tab_mut(id)
                .unwrap()
                .edits
                .editor
                .as_mut()
                .expect("an editor");
            editor.text = "user2@example.comx".to_owned();
            harness.app.apply(Action::EditorTyped { tab, id });
            // Save takes what was typed, and closes once it is written.
            harness.app.apply(Action::LeaveSave);
            assert_eq!(writes(&harness), 1);
            let Some(Command::Write { changes, .. }) = harness.app.backend.sent.last() else {
                panic!("a save was sent");
            };
            assert_eq!(
                changes.rows[0].set[0].new,
                NewValue::Text("user2@example.comx".into())
            );
            assert!(harness.app.workspace(tab).unwrap().object_tab(id).is_some());
        }

        #[test]
        fn a_prompt_about_pending_changes_is_not_replaced_by_another_dialog() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            let conn = harness.app.workspace(tab).unwrap().conn_id.clone();
            let replacing = || {
                [
                    Action::NewConnection,
                    Action::EditConnection(conn.clone()),
                    Action::CancelPassword,
                    Action::TrustHostKey,
                ]
            };
            harness.app.apply(Action::CloseTab { tab, id });
            for action in replacing() {
                harness.app.apply(action);
                assert_eq!(leave_prompt(&harness), Some(true));
            }
            // The held close is still there to be done.
            harness.app.apply(Action::LeaveDiscard);
            assert!(harness.app.workspace(tab).unwrap().object_tab(id).is_none());
            // The confirmation too.
            let mut harness = Harness::new();
            let (tab, id) = production(&mut harness);
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            harness.app.apply(Action::WriteEdits { tab, id });
            let conn = harness.app.workspace(tab).unwrap().conn_id.clone();
            for action in [
                Action::NewConnection,
                Action::EditConnection(conn),
                Action::CancelPassword,
                Action::TrustHostKey,
            ] {
                harness.app.apply(action);
                assert!(confirming(&harness).is_some());
            }
        }

        #[test]
        fn a_confirmation_sends_nothing_when_the_tab_is_not_as_it_showed_it() {
            // Nothing the user can do changes the tab under the prompt, so
            // each of these is forced: the confirmation checks all the same.
            type Forced = fn(&mut crate::model::ObjectTab);
            let forced: [(&str, Forced); 3] = [
                ("another cell", |object| {
                    object.edits.cells.insert(
                        (3, 1),
                        crate::edit::Pending {
                            new: NewValue::Text("dan@example.com".into()),
                            state: State::Ready,
                        },
                    );
                }),
                ("no cell", |object| object.edits.cells.clear()),
                ("an editor", |object| {
                    object.edits.editor = Some(crate::edit::Editor {
                        cell: at(3, 1),
                        place: crate::edit::EditorPlace::Grid,
                        text: "dan@example.com".into(),
                        large: false,
                        focus: false,
                        touched: true,
                        problem: None,
                    });
                }),
            ];
            for (name, force) in forced {
                let mut harness = Harness::new();
                let (tab, id) = production(&mut harness);
                type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
                // From the Leave prompt, so a close is held for the save.
                harness.app.apply(Action::CloseTab { tab, id });
                harness.app.apply(Action::LeaveSave);
                assert!(confirming(&harness).is_some(), "{name}");
                force(
                    harness
                        .app
                        .workspace_mut(tab)
                        .unwrap()
                        .object_tab_mut(id)
                        .unwrap(),
                );
                harness.app.apply(Action::ConfirmWrite);
                assert!(harness.app.dialog.is_none(), "{name}");
                assert_eq!(writes(&harness), 0, "{name}");
                let kept = harness.app.workspace(tab).unwrap().object_tab(id);
                assert!(
                    kept.is_some_and(|object| object.edits.saving.is_none()),
                    "{name}: the held close is dropped"
                );
            }
        }

        #[test]
        fn no_edit_starts_while_the_structure_is_described_again() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            // A refresh reads the rows and the structure again: the rows
            // are back, the structure is not.
            harness.app.apply(Action::Refresh(tab));
            harness.answer_rows(page(5, false));
            let described = object(&harness, tab, id);
            assert!(described.structure.is_loading() && !described.rows.is_loading());
            harness.app.apply(Action::EditCell {
                tab,
                id,
                cell: at(1, 1),
                start: EditStart::Value,
            });
            let edits = &object(&harness, tab, id).edits;
            assert!(edits.editor.is_none());
            assert_eq!(edits.why, Some((at(1, 1), Lock::Refreshing)));
            // `meta` of the first row holds a value and may be NULL.
            harness.app.apply(Action::SelectCell {
                tab,
                id,
                cell: at(0, 2),
            });
            harness.app.apply(Action::SetNull { tab, id });
            assert!(!object(&harness, tab, id).edits.holds());
            // Once it is known what the table is now, both work.
            harness.answer_structure(crate::testing::fixture_structure());
            harness.app.apply(Action::SetNull { tab, id });
            assert_eq!(object(&harness, tab, id).edits.cells.len(), 1);
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            assert_eq!(object(&harness, tab, id).edits.cells.len(), 2);
        }

        #[test]
        fn retrying_the_structure_waits_for_an_open_editor_too() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            harness.app.apply(Action::EditCell {
                tab,
                id,
                cell: at(1, 1),
                start: EditStart::Replace("bob@example.com".into()),
            });
            let before = harness.app.backend.sent.len();
            harness.app.apply(Action::RetryStructure {
                tab,
                object_tab: id,
            });
            assert_eq!(leave_prompt(&harness), Some(true));
            assert_eq!(harness.app.backend.sent.len(), before, "nothing ran");
            let kept = object(&harness, tab, id);
            assert!(kept.structure.value.is_some() && !kept.structure.is_loading());
            // Staying keeps what was typed, and it can still be saved.
            harness.app.apply(Action::LeaveStay);
            harness.app.apply(Action::WriteEdits { tab, id });
            assert_eq!(writes(&harness), 1);
        }

        #[test]
        fn a_set_no_change_set_can_be_built_from_is_not_offered_to_be_saved() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            // Nothing pending is nothing to block.
            assert_eq!(harness.app.save_blocked(tab, id), None);
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            assert_eq!(harness.app.save_blocked(tab, id), None);
            // No path leaves a pending cell on a table without a key: it is
            // forced.
            harness
                .app
                .workspace_mut(tab)
                .unwrap()
                .object_tab_mut(id)
                .unwrap()
                .structure
                .value
                .as_mut()
                .unwrap()
                .primary_key
                .clear();
            assert_eq!(
                harness.app.save_blocked(tab, id),
                Some(crate::model::SaveBlock::Unsendable)
            );
            harness.app.apply(Action::WriteEdits { tab, id });
            assert_eq!(writes(&harness), 0);
            // Leaving offers no Save that would do nothing.
            harness.app.apply(Action::CloseTab { tab, id });
            assert_eq!(leave_prompt(&harness), Some(false));
            // The structure gone altogether is the same.
            harness.app.apply(Action::LeaveStay);
            harness
                .app
                .workspace_mut(tab)
                .unwrap()
                .object_tab_mut(id)
                .unwrap()
                .structure
                .value = None;
            assert_eq!(
                harness.app.save_blocked(tab, id),
                Some(crate::model::SaveBlock::Unsendable)
            );
        }

        /// The request of the save that runs.
        fn saving(harness: &Harness, tab: ConnTabId, id: TabId) -> Option<RequestId> {
            let edits = &object(harness, tab, id).edits;
            edits.saving.as_ref().map(|saving| saving.request)
        }

        #[test]
        fn a_second_save_is_not_sent_while_the_first_runs() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            harness.app.apply(Action::WriteEdits { tab, id });
            let first = saving(&harness, tab, id).expect("a save");
            assert_eq!(
                harness.app.save_blocked(tab, id),
                Some(crate::model::SaveBlock::Saving)
            );
            harness.app.apply(Action::WriteEdits { tab, id });
            assert_eq!(writes(&harness), 1);
            // The first answer still finds the save it answers.
            assert_eq!(saving(&harness, tab, id), Some(first));
            harness.answer_written(written("bob@example.com"));
            assert!(!object(&harness, tab, id).edits.holds());
        }

        #[test]
        fn a_confirmation_sends_no_second_save_either() {
            let mut harness = Harness::new();
            let (tab, id) = production(&mut harness);
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            harness.app.apply(Action::WriteEdits { tab, id });
            // A save that started behind the confirmation's back: nothing
            // a user does starts one, so the prompt is set aside for it.
            let prompt = harness.app.dialog.take();
            assert!(matches!(prompt, Some(Dialog::ConfirmWrite(_))));
            harness.app.workspace_mut(tab).unwrap().environment = crate::env::Environment::None;
            harness.app.apply(Action::WriteEdits { tab, id });
            let first = saving(&harness, tab, id).expect("a save");
            harness.app.dialog = prompt;
            harness.app.apply(Action::ConfirmWrite);
            assert!(harness.app.dialog.is_none());
            assert_eq!(writes(&harness), 1);
            assert_eq!(saving(&harness, tab, id), Some(first));
        }

        #[test]
        fn no_edit_starts_while_the_page_is_fetched_again() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            harness.app.apply(Action::SelectCell {
                tab,
                id,
                cell: at(0, 2),
            });
            // The structure is back, the rows are not.
            harness.app.apply(Action::Refresh(tab));
            harness.answer_structure(crate::testing::fixture_structure());
            let fetched = object(&harness, tab, id);
            assert!(fetched.rows.is_loading() && !fetched.structure.is_loading());
            harness.app.apply(Action::SetNull { tab, id });
            assert!(!object(&harness, tab, id).edits.holds());
            harness.app.apply(Action::EditCell {
                tab,
                id,
                cell: at(1, 1),
                start: EditStart::Value,
            });
            let edits = &object(&harness, tab, id).edits;
            assert!(edits.editor.is_none() && edits.cells.is_empty());
            assert_eq!(edits.why, Some((at(1, 1), Lock::Refreshing)));
        }

        #[test]
        fn null_is_not_set_under_a_save_or_on_a_read_only_session() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            // `meta` of the first row holds a value and may be NULL.
            harness.app.apply(Action::SelectCell {
                tab,
                id,
                cell: at(0, 2),
            });
            harness.app.apply(Action::WriteEdits { tab, id });
            harness.app.apply(Action::SetNull { tab, id });
            assert_eq!(object(&harness, tab, id).edits.cells.len(), 1);
            harness.answer_written(written("bob@example.com"));
            // The session came back read-only.
            harness.app.workspace_mut(tab).unwrap().access = tabletist_db::Access::ReadOnly;
            harness.app.apply(Action::SetNull { tab, id });
            assert!(object(&harness, tab, id).edits.cells.is_empty());
            harness.app.workspace_mut(tab).unwrap().access = tabletist_db::Access::Writable;
            harness.app.apply(Action::SetNull { tab, id });
            assert_eq!(object(&harness, tab, id).edits.cells.len(), 1);
        }

        #[test]
        fn a_confirmation_answered_after_the_session_went_sends_nothing() {
            let mut harness = Harness::new();
            let (tab, id) = production(&mut harness);
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            harness.app.apply(Action::WriteEdits { tab, id });
            let session = harness.app.workspace(tab).unwrap().session;
            harness.app.apply(Action::Backend(Event::Disconnected {
                session,
                error: tabletist_db::Error::ConnectionLost("gone".into()),
            }));
            assert!(confirming(&harness).is_some());
            harness.app.apply(Action::ConfirmWrite);
            assert!(harness.app.dialog.is_none());
            assert_eq!(writes(&harness), 0);
            let edits = &object(&harness, tab, id).edits;
            assert!(edits.saving.is_none());
            assert_eq!(edits.cells.len(), 1);
            // And says why, where the prompt was: nothing went out, which
            // is not what a save lost in flight says.
            assert_eq!(edits.note, Some(crate::edit::Note::NotSent));
            // Any other reason shows on the Save itself: nothing is added.
            let mut harness = Harness::new();
            let (tab, id) = production(&mut harness);
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            harness.app.apply(Action::WriteEdits { tab, id });
            harness.app.workspace_mut(tab).unwrap().access = tabletist_db::Access::ReadOnly;
            harness.app.apply(Action::ConfirmWrite);
            assert!(harness.app.dialog.is_none());
            assert_eq!(writes(&harness), 0);
            assert_eq!(object(&harness, tab, id).edits.note, None);
        }

        #[test]
        fn fewer_written_rows_than_the_set_has_fetch_the_page_again() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            type_into(&mut harness, tab, id, at(3, 1), "dan@example.com");
            harness.app.apply(Action::WriteEdits { tab, id });
            let before = harness.app.backend.sent.len();
            harness.answer_written(written("bob@example.com"));
            let after = object(&harness, tab, id);
            assert!(!after.edits.holds() && after.edits.saved.is_none());
            // The one row that came is not put anywhere.
            assert_eq!(
                after.page().unwrap().rows[1][1],
                Value::Text("user2@example.com".into())
            );
            assert!(matches!(
                harness.app.backend.sent[before..].last(),
                Some(Command::FetchRows { .. })
            ));
        }

        #[test]
        fn an_answer_to_a_prompt_that_is_not_up_closes_no_other_dialog() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            harness.app.apply(Action::ShowHelp);
            for answer in [
                Action::LeaveDiscard,
                Action::LeaveSave,
                Action::LeaveStay,
                Action::ConfirmWrite,
                Action::CancelWrite,
            ] {
                harness.app.apply(answer);
                assert!(matches!(harness.app.dialog, Some(Dialog::Help)));
            }
            assert_eq!(object(&harness, tab, id).edits.cells.len(), 1);
            assert_eq!(writes(&harness), 0);
        }

        #[test]
        fn a_save_to_production_does_not_replace_another_dialog() {
            let mut harness = Harness::new();
            let (tab, id) = production(&mut harness);
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            harness.app.apply(Action::ShowHelp);
            harness.app.apply(Action::WriteEdits { tab, id });
            assert!(matches!(harness.app.dialog, Some(Dialog::Help)));
            assert_eq!(writes(&harness), 0);
            assert!(object(&harness, tab, id).edits.saving.is_none());
            // With the dialog closed it asks.
            harness.app.apply(Action::CloseDialog);
            harness.app.apply(Action::WriteEdits { tab, id });
            assert!(confirming(&harness).is_some());
        }

        #[test]
        fn save_from_the_leave_prompt_on_production_is_confirmed_and_then_goes_on() {
            let mut harness = Harness::new();
            let (tab, id) = production(&mut harness);
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            let open_tab = |harness: &Harness| {
                let workspace = harness.app.workspace(tab).unwrap();
                workspace.object_tab(id).is_some()
            };
            // Cancelling the confirmation drops the held close: the tab and
            // its set stay, and nothing asks a second time.
            harness.app.apply(Action::CloseTab { tab, id });
            harness.app.apply(Action::LeaveSave);
            assert!(confirming(&harness).is_some());
            assert_eq!(writes(&harness), 0, "nothing is sent before the answer");
            harness.app.apply(Action::CancelWrite);
            assert!(harness.app.dialog.is_none() && open_tab(&harness));
            assert_eq!(object(&harness, tab, id).edits.cells.len(), 1);
            assert_eq!(writes(&harness), 0);
            // Confirmed, the held close waits for the save.
            harness.app.apply(Action::CloseTab { tab, id });
            harness.app.apply(Action::LeaveSave);
            assert_eq!(writes(&harness), 0);
            harness.app.apply(Action::ConfirmWrite);
            assert_eq!(writes(&harness), 1);
            assert!(open_tab(&harness), "the tab closes once it is written");
            harness.answer_written(written("bob@example.com"));
            assert!(!open_tab(&harness));
        }

        #[test]
        fn discard_from_the_leave_prompt_drops_every_tabs_changes_and_goes_on() {
            let two_tabs = |harness: &mut Harness| {
                let (tab, id) = harness.editable();
                type_into(harness, tab, id, at(1, 1), "bob@example.com");
                let orders = open(harness, tab, "orders", true);
                harness.answer_structure(crate::testing::fixture_structure());
                harness.answer_rows(page(5, false));
                type_into(harness, tab, orders, at(0, 1), "x@example.com");
                type_into(harness, tab, orders, at(2, 1), "y@example.com");
                tab
            };
            // Closing the connection.
            let mut harness = Harness::new();
            let tab = two_tabs(&mut harness);
            let session = harness.app.workspace(tab).unwrap().session;
            harness.app.apply(Action::CloseConnTab(tab));
            match &harness.app.dialog {
                Some(Dialog::Leave(prompt)) => {
                    assert_eq!((prompt.tabs.len(), prompt.changes), (2, 3));
                    assert!(!prompt.can_save);
                }
                other => panic!("expected the Leave prompt, got {other:?}"),
            }
            assert!(harness.app.workspace(tab).is_some());
            harness.app.apply(Action::LeaveDiscard);
            assert!(harness.app.dialog.is_none());
            assert!(harness.app.workspace(tab).is_none());
            assert!(matches!(
                harness.app.backend.sent.last(),
                Some(Command::Close { session: closed }) if *closed == session
            ));
            // Disconnecting.
            let mut harness = Harness::new();
            let tab = two_tabs(&mut harness);
            harness.app.apply(Action::Disconnect(tab));
            assert_eq!(leave_prompt(&harness), Some(false));
            harness.app.apply(Action::LeaveDiscard);
            assert!(harness.app.dialog.is_none());
            assert!(harness.app.workspace(tab).is_none());
            assert_eq!(writes(&harness), 0);
        }

        #[test]
        fn cancelling_stops_the_save_that_runs() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            harness.app.apply(Action::WriteEdits { tab, id });
            let request = saving(&harness, tab, id).expect("a save");
            let session = harness.app.workspace(tab).unwrap().session;
            let before = harness.app.backend.sent.len();
            harness.app.apply(Action::CancelQuery(tab));
            let cancelled: Vec<_> = harness.app.backend.sent[before..]
                .iter()
                .filter_map(|command| match command {
                    Command::Cancel { session, request } => Some((*session, *request)),
                    _ => None,
                })
                .collect();
            assert_eq!(cancelled, [(session, request)]);
            // The save runs until the backend says how it ended.
            assert_eq!(saving(&harness, tab, id), Some(request));
            harness.answer_written(Err(tabletist_db::Error::Cancelled));
            let edits = &object(&harness, tab, id).edits;
            assert_eq!(edits.note, Some(crate::edit::Note::Cancelled));
            assert_eq!(edits.cells.len(), 1);
        }

        #[test]
        fn an_answer_naming_a_row_the_save_did_not_send_marks_no_row() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            // The page's first row is changed too: it is not the one to
            // name and mark for want of another.
            type_into(&mut harness, tab, id, at(0, 1), "ada@example.com");
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            let all_ready = |harness: &Harness| {
                let edits = &object(harness, tab, id).edits;
                edits.cells.len() == 2
                    && edits.cells.values().all(|cell| cell.state == State::Ready)
            };
            let error = tabletist_db::Error::query("violates check");
            harness.app.apply(Action::WriteEdits { tab, id });
            harness.answer_written(Ok(WriteOutcome::Failed {
                row: 2,
                error: error.clone(),
            }));
            assert!(all_ready(&harness));
            assert_eq!(
                object(&harness, tab, id).edits.note,
                Some(crate::edit::Note::Refused(error))
            );
            // A conflict has no error of its own to show: what the save
            // came to is not known.
            harness.app.apply(Action::WriteEdits { tab, id });
            harness.answer_written(Ok(WriteOutcome::Conflicts(vec![Conflict {
                row: 2,
                server: None,
            }])));
            assert!(all_ready(&harness));
            assert_eq!(
                object(&harness, tab, id).edits.note,
                Some(crate::edit::Note::Lost)
            );
        }

        #[test]
        fn what_was_held_goes_on_after_a_save_whose_rows_do_not_fit_the_page() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            harness.app.apply(Action::CloseTab { tab, id });
            harness.app.apply(Action::LeaveSave);
            // Everything was written, by a table of another width.
            harness.answer_written(Ok(WriteOutcome::Written {
                rows: vec![vec![Value::Int(2)]],
                elapsed: std::time::Duration::ZERO,
            }));
            assert!(harness.app.dialog.is_none());
            assert!(harness.app.workspace(tab).unwrap().object_tab(id).is_none());
        }

        #[test]
        fn an_edit_is_printed_without_what_was_typed() {
            let (tab, id) = (ConnTabId(1), TabId(2));
            for (start, variant) in [
                (EditStart::Value, "Value"),
                (EditStart::Replace("hunter2".into()), "Replace"),
                (EditStart::Typed("hunter2".into()), "Typed"),
            ] {
                let printed = format!(
                    "{:?}",
                    Action::EditCell {
                        tab,
                        id,
                        cell: at(0, 1),
                        start,
                    }
                );
                assert!(printed.contains(variant), "{printed}");
                assert!(!printed.contains("hunter2"), "{printed}");
            }
        }

        #[test]
        fn a_tab_with_a_pending_cell_is_no_preview_whatever_its_pin_says() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            // Every path that makes a cell pending pins the tab: one that
            // forgot to is forced.
            harness
                .app
                .workspace_mut(tab)
                .unwrap()
                .object_tab_mut(id)
                .unwrap()
                .pinned = false;
            let orders = open(&mut harness, tab, "orders", false);
            assert_ne!(orders, id);
            let workspace = harness.app.workspace(tab).unwrap();
            assert_eq!(workspace.tabs.len(), 2, "opened beside it, not over it");
            assert_eq!(object(&harness, tab, id).edits.cells.len(), 1);
            // Without the cell it is a preview again, and the next one
            // takes its place.
            harness.app.apply(Action::DiscardEdits { tab, id });
            harness.app.apply(Action::CloseTab { tab, id: orders });
            open(&mut harness, tab, "orders", false);
            let workspace = harness.app.workspace(tab).unwrap();
            assert_eq!(workspace.tabs.len(), 1);
            assert!(workspace.object_tab(id).is_none());
        }

        #[test]
        fn a_statement_that_cannot_be_built_fails_its_row_instead_of_a_confirmation() {
            let mut harness = Harness::new();
            let (tab, id) = production(&mut harness);
            type_into(&mut harness, tab, id, at(0, 1), "ada@example.com");
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            harness.app.apply(Action::SelectCell {
                tab,
                id,
                cell: at(1, 2),
            });
            // No check lets through what the builder refuses: it is forced,
            // by a row that held bytes where its change was typed. And an
            // earlier save's mark, for this one to take away as any save
            // does.
            {
                let object = harness
                    .app
                    .workspace_mut(tab)
                    .unwrap()
                    .object_tab_mut(id)
                    .unwrap();
                object.rows.value.as_mut().unwrap().rows[1][1] = Value::Bytes(vec![1, 2].into());
                object.edits.saved = Some(crate::edit::Saved {
                    at: std::time::Instant::now(),
                    cells: vec![at(4, 1)],
                    changes: 1,
                    rows: 1,
                    elapsed: std::time::Duration::ZERO,
                });
            }
            // From the Leave prompt, so a close is held for the save.
            harness.app.apply(Action::CloseTab { tab, id });
            harness.app.apply(Action::LeaveSave);
            assert!(harness.app.dialog.is_none(), "no statement to show");
            assert_eq!(writes(&harness), 0);
            let edits = &object(&harness, tab, id).edits;
            let Some(crate::edit::Note::Failed { row: 1, error }) = &edits.note else {
                panic!("expected the second row to fail, got {:?}", edits.note);
            };
            assert_eq!(
                edits.cells.get(&(1, 1)).map(|cell| &cell.state),
                Some(&State::Failed(error.clone()))
            );
            // The row before it is as it was, and the held close is dropped.
            assert_eq!(
                edits.cells.get(&(0, 1)).map(|cell| &cell.state),
                Some(&State::Ready)
            );
            assert!(edits.saving.is_none() && edits.saved.is_none());
        }

        #[test]
        fn a_row_the_save_would_refuse_opens_no_confirmation_and_fails_its_cells() {
            // Two rows no save of their database sends: PostgreSQL text
            // that holds a NUL, and a SQLite row whose key may not have
            // been read exactly (no editor opens on such a row: it is
            // forced, by a key that changed under a pending cell).
            for driver in [tabletist_db::Driver::Postgres, tabletist_db::Driver::Sqlite] {
                let mut harness = Harness::new();
                let (tab, id) = production(&mut harness);
                harness.app.workspace_mut(tab).unwrap().driver = driver;
                type_into(&mut harness, tab, id, at(0, 1), "ada@example.com");
                let reason = if driver == tabletist_db::Driver::Postgres {
                    type_into(&mut harness, tab, id, at(1, 1), "bob\0@example.com");
                    "PostgreSQL text cannot hold a NUL character"
                } else {
                    type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
                    let object = harness.app.object_tab_mut(tab, id).unwrap();
                    object.rows.value.as_mut().unwrap().rows[1][0] =
                        Value::Text("caf\u{FFFD}".into());
                    "the row's key holds text that may not have been read exactly, so the \
                     save cannot be sure which row it names"
                };
                assert_eq!(object(&harness, tab, id).edits.counts().to_fix, 0);
                // The review says so before a save is asked for: a
                // comment, and no statement.
                show_review(&mut harness, tab, id, true);
                assert_eq!(
                    reviewed(&mut harness, tab, id).unwrap().len(),
                    3,
                    "{driver:?}"
                );
                let review = object(&harness, tab, id).edits.review.as_ref().unwrap();
                assert!(
                    matches!(
                        review.lines.last(),
                        Some(crate::review::Line::Refused { reason: said, .. }) if said == reason
                    ),
                    "{driver:?}: {:?}",
                    review.lines.last()
                );
                // And the save asks nothing: no statement of the row is
                // one to confirm.
                harness.app.apply(Action::WriteEdits { tab, id });
                assert!(harness.app.dialog.is_none(), "{driver:?}");
                assert_eq!(writes(&harness), 0, "{driver:?}");
                let edits = &object(&harness, tab, id).edits;
                let Some(crate::edit::Note::Failed { row: 1, error }) = &edits.note else {
                    panic!(
                        "{driver:?}: expected the second row to fail, got {:?}",
                        edits.note
                    );
                };
                assert_eq!(error.to_string(), reason, "{driver:?}");
                assert_eq!(
                    edits.cells.get(&(1, 1)).map(|cell| &cell.state),
                    Some(&State::Failed(error.clone())),
                    "{driver:?}"
                );
                assert_eq!(
                    edits.cells.get(&(0, 1)).map(|cell| &cell.state),
                    Some(&State::Ready),
                    "{driver:?}"
                );
                assert!(edits.saving.is_none(), "{driver:?}");
            }
        }

        #[test]
        fn copying_takes_the_pending_value_the_cell_shows() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            type_into(&mut harness, tab, id, at(0, 1), "ada@example.com");
            harness.app.apply(Action::SelectCell {
                tab,
                id,
                cell: at(0, 1),
            });
            assert_eq!(
                harness.app.copy_text(false).as_deref(),
                Some("ada@example.com")
            );
            // The row, column by column: what is pending in it, and what
            // was loaded where nothing is.
            assert_eq!(
                harness.app.copy_text(true).as_deref(),
                Some("1\tada@example.com\t{\"plan\":\"pro\"}")
            );
            // A cell set to NULL copies as a loaded NULL does.
            harness.app.apply(Action::SelectCell {
                tab,
                id,
                cell: at(1, 2),
            });
            let null = harness.app.copy_text(false);
            harness.app.apply(Action::SelectCell {
                tab,
                id,
                cell: at(0, 2),
            });
            harness.app.apply(Action::SetNull { tab, id });
            assert_eq!(harness.app.copy_text(false), null);
            assert_eq!(
                harness.app.copy_text(true),
                null.map(|null| format!("1\tada@example.com\t{null}"))
            );
            // Another row's changes are not this row's.
            harness.app.apply(Action::SelectCell {
                tab,
                id,
                cell: at(1, 1),
            });
            assert_eq!(
                harness.app.copy_text(false).as_deref(),
                Some("user2@example.com")
            );
        }

        #[test]
        fn a_gone_row_is_locked_until_the_page_is_loaded_again() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            let gone = |harness: &Harness| -> Vec<usize> {
                let edits = &object(harness, tab, id).edits;
                edits.gone.iter().copied().collect()
            };
            // What a save that found the row `id 2` gone leaves on the tab.
            let mark = |harness: &mut Harness| {
                let workspace = harness.app.workspace_mut(tab).unwrap();
                workspace.object_tab_mut(id).unwrap().edits.gone.insert(1);
            };
            mark(&mut harness);
            // Its cells are not edited, and say why when asked.
            harness.app.apply(Action::EditCell {
                tab,
                id,
                cell: at(1, 1),
                start: EditStart::Value,
            });
            let edits = &object(&harness, tab, id).edits;
            assert!(edits.editor.is_none());
            assert_eq!(edits.why, Some((at(1, 1), Lock::Gone)));
            harness.app.apply(Action::SelectCell {
                tab,
                id,
                cell: at(1, 2),
            });
            harness.app.apply(Action::SetNull { tab, id });
            assert!(object(&harness, tab, id).edits.cells.is_empty());
            // It is still gone after a save of another row that wrote,
            type_into(&mut harness, tab, id, at(3, 1), "dan@example.com");
            harness.app.apply(Action::WriteEdits { tab, id });
            harness.answer_written(Ok(WriteOutcome::Written {
                rows: vec![row(4, "dan@example.com")],
                elapsed: std::time::Duration::ZERO,
            }));
            assert_eq!(gone(&harness), [1]);
            // after a discard of what is pending,
            type_into(&mut harness, tab, id, at(0, 1), "ada@example.com");
            harness.app.apply(Action::DiscardEdits { tab, id });
            assert_eq!(gone(&harness), [1]);
            // and after a discard from the question before leaving, while
            // the page it leaves is still on screen.
            type_into(&mut harness, tab, id, at(0, 1), "ada@example.com");
            harness.app.apply(Action::Refresh(tab));
            harness.app.apply(Action::LeaveDiscard);
            assert_eq!(gone(&harness), [1]);
            // A fetch that fails leaves that page on screen, and the row
            // gone.
            let fetch = |harness: &Harness| {
                let sent = harness.app.backend.sent.iter().rev();
                let mut fetches = sent.filter_map(|command| match command {
                    Command::FetchRows {
                        session, request, ..
                    } => Some((*session, *request)),
                    _ => None,
                });
                fetches.next().expect("a FetchRows")
            };
            let (session, request) = fetch(&harness);
            harness.app.apply(Action::Backend(Event::Rows {
                session,
                request,
                result: Err(tabletist_db::Error::query("no such table")),
            }));
            assert_eq!(gone(&harness), [1]);
            // A gone row is nothing of the user's: it holds no page, and
            // nothing is asked before the page goes.
            assert!(!object(&harness, tab, id).edits.holds());
            harness.app.apply(Action::Refresh(tab));
            assert!(harness.app.dialog.is_none());
            // The page that arrives is of rows that are there.
            harness.answer_rows(page(5, false));
            assert!(gone(&harness).is_empty());
        }

        /// What the conflict question asks about: the place of the row
        /// among the save's conflicts, the page's row, and how many rows
        /// the save found changed.
        fn asking(harness: &Harness) -> Option<(usize, usize, usize)> {
            match &harness.app.dialog {
                Some(Dialog::Conflict(prompt)) => {
                    let row = prompt.rows.get(prompt.at)?.row;
                    Some((prompt.at, row, prompt.rows.len()))
                }
                _ => None,
            }
        }

        /// The columns the question shows, each with the loaded value it
        /// shows for it (`None` for NULL).
        fn shown(harness: &Harness) -> Vec<(String, Option<String>)> {
            let Some(Dialog::Conflict(prompt)) = &harness.app.dialog else {
                panic!("no conflict question: {:?}", harness.app.dialog);
            };
            let loaded = |line: &crate::edit::ShownLine| match &line.loaded {
                crate::edit::Shown::Null => None,
                crate::edit::Shown::Text { text, .. } => Some(text.clone()),
            };
            let lines = prompt.lines.iter();
            lines
                .map(|line| (line.name.clone(), loaded(line)))
                .collect()
        }

        /// Answers the question for the row it shows.
        fn answer(harness: &mut Harness, answer: Answer) {
            let Some((at, ..)) = asking(harness) else {
                panic!("no conflict question: {:?}", harness.app.dialog);
            };
            harness.app.apply(Action::AnswerConflict { at, answer });
        }

        /// The set's row `row` holds `email` on the server now.
        fn changed(row: usize, id: i64, email: &str) -> Conflict {
            Conflict {
                row,
                server: Some(self::row(id, email)),
            }
        }

        fn gone(row: usize) -> Conflict {
            Conflict { row, server: None }
        }

        /// The cells that are pending, by row and column.
        fn pending(harness: &Harness, tab: ConnTabId, id: TabId) -> Vec<(usize, usize)> {
            let edits = &object(harness, tab, id).edits;
            edits.cells.keys().copied().collect()
        }

        fn email(harness: &Harness, tab: ConnTabId, id: TabId, row: usize) -> Value {
            object(harness, tab, id).page().unwrap().rows[row][1].clone()
        }

        fn text(value: &str) -> Value {
            Value::Text(value.into())
        }

        /// The emails of the page's rows 1 and 3 (`id` 2 and 4) pending,
        /// saved, and the save answered with `conflicts`.
        fn conflicts(harness: &mut Harness, conflicts: Vec<Conflict>) -> (ConnTabId, TabId) {
            let (tab, id) = harness.editable();
            type_into(harness, tab, id, at(1, 1), "bob@example.com");
            type_into(harness, tab, id, at(3, 1), "dan@example.com");
            harness.app.apply(Action::WriteEdits { tab, id });
            harness.answer_written(Ok(WriteOutcome::Conflicts(conflicts)));
            (tab, id)
        }

        /// Both rows of `conflicts` found changed on the server.
        fn two_conflicts(harness: &mut Harness) -> (ConnTabId, TabId) {
            let both = vec![
                changed(0, 2, "eve@example.com"),
                changed(1, 4, "fay@example.com"),
            ];
            conflicts(harness, both)
        }

        #[test]
        fn a_conflict_asks_about_its_row_and_nothing_changes_the_set_under_the_question() {
            let mut harness = Harness::new();
            // The set's second row is the page's row 3.
            let (tab, id) = conflicts(&mut harness, vec![changed(1, 4, "fay@example.com")]);
            match &harness.app.dialog {
                Some(Dialog::Conflict(prompt)) => {
                    assert_eq!((prompt.tab, prompt.id, prompt.at), (tab, id, 0));
                    assert_eq!(
                        prompt.rows,
                        [Conflicting {
                            row: 3,
                            server: Some(row(4, "fay@example.com")),
                        }]
                    );
                }
                other => panic!("expected the conflict question, got {other:?}"),
            }
            // It shows the one column changed in that row.
            let line = ("email".to_owned(), Some("user4@example.com".to_owned()));
            assert_eq!(shown(&harness), [line]);
            // Nothing was written and nothing is rebased before an answer.
            let edits = &object(&harness, tab, id).edits;
            assert!(edits.saving.is_none() && edits.note.is_none());
            assert_eq!(pending(&harness, tab, id), [(1, 1), (3, 1)]);
            assert_eq!(email(&harness, tab, id, 3), text("user4@example.com"));
            // What edits, saves, discards or moves is dropped under it.
            let before = harness.app.backend.sent.len();
            harness.app.apply(Action::SelectCell {
                tab,
                id,
                cell: at(1, 1),
            });
            harness.app.apply(Action::RevertCell { tab, id });
            harness.app.apply(Action::DiscardEdits { tab, id });
            harness.app.apply(Action::WriteEdits { tab, id });
            change(&mut harness, tab, id, at(0, 1), "ada@example.com");
            assert_eq!(pending(&harness, tab, id), [(1, 1), (3, 1)]);
            assert!(write_since(&harness, before).is_none());
            // What would drop the page is refused, with the reason.
            harness.app.apply(Action::CloseTab { tab, id });
            assert_eq!(asking(&harness), Some((0, 3, 1)));
            assert!(harness.app.notice.is_some());
            assert!(harness.app.workspace(tab).unwrap().object_tab(id).is_some());
            // The answers of the other prompts are not answers to it.
            for other in [
                Action::LeaveDiscard,
                Action::LeaveSave,
                Action::LeaveStay,
                Action::ConfirmWrite,
                Action::CancelWrite,
            ] {
                harness.app.apply(other);
                assert_eq!(asking(&harness), Some((0, 3, 1)));
            }
            // Nor is its answer one to another dialog.
            answer(&mut harness, Answer::KeepMine);
            harness.app.apply(Action::ShowHelp);
            harness.app.apply(Action::AnswerConflict {
                at: 0,
                answer: Answer::UseServer,
            });
            assert!(matches!(harness.app.dialog, Some(Dialog::Help)));
            assert_eq!(pending(&harness, tab, id), [(1, 1), (3, 1)]);
        }

        #[test]
        fn keep_mine_reloads_the_row_and_keeps_the_cells_that_still_differ() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            type_into(&mut harness, tab, id, at(1, 2), "{}");
            type_into(&mut harness, tab, id, at(3, 1), "dan@example.com");
            harness.app.apply(Action::WriteEdits { tab, id });
            // Someone gave the row the email typed here, and another meta.
            let server = vec![Value::Int(2), text("bob@example.com"), text("[1]")];
            harness.answer_written(Ok(WriteOutcome::Conflicts(vec![Conflict {
                row: 0,
                server: Some(server.clone()),
            }])));
            // The row panel holds the row's text as it was loaded.
            harness
                .app
                .workspace_mut(tab)
                .unwrap()
                .object_tab_mut(id)
                .unwrap()
                .fields = Some(crate::model::RowFields {
                request: None,
                row: 1,
                fields: Vec::new(),
                pending: Vec::new(),
            });
            let before = harness.app.backend.sent.len();
            // Discard is the answer for a row that is gone, not for this one.
            answer(&mut harness, Answer::Discard);
            assert_eq!(asking(&harness), Some((0, 1, 1)));
            assert_eq!(pending(&harness, tab, id), [(1, 1), (1, 2), (3, 1)]);
            answer(&mut harness, Answer::KeepMine);
            assert!(harness.app.dialog.is_none());
            let now = object(&harness, tab, id);
            // The server's row is the loaded one now.
            assert_eq!(now.page().unwrap().rows[1], server);
            // The email is what the server holds: no change any more. The
            // meta still differs, and stays on top.
            assert_eq!(pending(&harness, tab, id), [(1, 2), (3, 1)]);
            assert_eq!(
                now.edits.cells.get(&(1, 2)).map(|cell| &cell.new),
                Some(&NewValue::Text("{}".into()))
            );
            // The row panel's text is formatted again.
            assert!(now.fields.is_none());
            assert!(now.edits.gone.is_empty() && now.edits.note.is_none());
            // Nothing is saved: what is left waits for the user.
            assert!(write_since(&harness, before).is_none());
            assert!(now.edits.saving.is_none());
        }

        #[test]
        fn use_server_values_reloads_the_row_and_drops_its_cells() {
            let mut harness = Harness::new();
            let (tab, id) = conflicts(&mut harness, vec![changed(0, 2, "eve@example.com")]);
            let before = harness.app.backend.sent.len();
            answer(&mut harness, Answer::UseServer);
            assert!(harness.app.dialog.is_none());
            assert_eq!(email(&harness, tab, id, 1), text("eve@example.com"));
            // The other row did not conflict: it stays pending, unsaved.
            assert_eq!(pending(&harness, tab, id), [(3, 1)]);
            assert_eq!(email(&harness, tab, id, 3), text("user4@example.com"));
            assert!(write_since(&harness, before).is_none());
        }

        #[test]
        fn a_row_that_is_gone_can_only_be_discarded_and_is_marked_gone() {
            let mut harness = Harness::new();
            let (tab, id) = conflicts(&mut harness, vec![gone(0)]);
            let before = harness.app.backend.sent.len();
            // There is no row on the server to take or to write over.
            answer(&mut harness, Answer::UseServer);
            answer(&mut harness, Answer::Overwrite);
            assert_eq!(asking(&harness), Some((0, 1, 1)));
            assert_eq!(pending(&harness, tab, id), [(1, 1), (3, 1)]);
            answer(&mut harness, Answer::Discard);
            assert!(harness.app.dialog.is_none());
            // Its cells are dropped; the row that did not conflict stays
            // pending, unsaved.
            assert_eq!(pending(&harness, tab, id), [(3, 1)]);
            assert!(write_since(&harness, before).is_none());
            let edits = &object(&harness, tab, id).edits;
            assert_eq!(edits.gone.iter().copied().collect::<Vec<_>>(), [1]);
            // The row stays on the page as it was loaded, and is locked.
            assert_eq!(email(&harness, tab, id, 1), text("user2@example.com"));
            harness.app.apply(Action::EditCell {
                tab,
                id,
                cell: at(1, 1),
                start: EditStart::Value,
            });
            let edits = &object(&harness, tab, id).edits;
            assert!(edits.editor.is_none());
            assert_eq!(edits.why, Some((at(1, 1), Lock::Gone)));
        }

        #[test]
        fn keeping_a_row_that_is_gone_leaves_its_cells_and_says_so() {
            let mut harness = Harness::new();
            let (tab, id) = conflicts(&mut harness, vec![gone(0)]);
            let before = harness.app.backend.sent.len();
            answer(&mut harness, Answer::KeepMine);
            assert!(harness.app.dialog.is_none());
            // Nothing is dropped and nothing is marked or locked.
            assert_eq!(pending(&harness, tab, id), [(1, 1), (3, 1)]);
            let edits = &object(&harness, tab, id).edits;
            assert!(edits.gone.is_empty());
            assert!(write_since(&harness, before).is_none());
            // The tab says that the row is not there: it is not left
            // looking like any other row with pending changes.
            assert_eq!(
                edits.note,
                Some(crate::edit::Note::Conflict {
                    row: 1,
                    gone: true,
                    others: 0
                })
            );
            // A row that changed and was kept has nothing to say: it is
            // on the page as the server has it.
            let mut harness = Harness::new();
            let (tab, id) = conflicts(&mut harness, vec![changed(0, 2, "eve@example.com")]);
            answer(&mut harness, Answer::KeepMine);
            assert_eq!(object(&harness, tab, id).edits.note, None);
        }

        #[test]
        fn the_next_rows_question_comes_up_when_the_row_before_it_is_answered() {
            let mut harness = Harness::new();
            two_conflicts(&mut harness);
            let came_up = |harness: &Harness| match &harness.app.dialog {
                Some(Dialog::Conflict(prompt)) => prompt.shown,
                other => panic!("no conflict question: {other:?}"),
            };
            // The first row's question has been up for a while.
            let earlier = std::time::Instant::now()
                .checked_sub(2 * crate::edit::ANSWER_AFTER)
                .expect("an earlier instant");
            if let Some(Dialog::Conflict(prompt)) = &mut harness.app.dialog {
                prompt.shown = earlier;
            }
            assert!(crate::edit::answers_taken(came_up(&harness)));
            let answered = std::time::Instant::now();
            answer(&mut harness, Answer::UseServer);
            // The second row's has not: it is a new question on screen, in
            // the place of the one that was answered, and the second click
            // of a double click is no answer to it.
            assert_eq!(asking(&harness), Some((1, 3, 2)));
            assert!(came_up(&harness) >= answered);
        }

        #[test]
        fn several_conflicts_are_asked_one_after_another() {
            let mut harness = Harness::new();
            let (tab, id) = two_conflicts(&mut harness);
            assert_eq!(asking(&harness), Some((0, 1, 2)));
            answer(&mut harness, Answer::UseServer);
            // The first row is settled at once, whatever the second gets.
            assert_eq!(email(&harness, tab, id, 1), text("eve@example.com"));
            assert_eq!(pending(&harness, tab, id), [(3, 1)]);
            assert_eq!(asking(&harness), Some((1, 3, 2)));
            // What it shows is the second row's.
            let line = ("email".to_owned(), Some("user4@example.com".to_owned()));
            assert_eq!(shown(&harness), [line]);
            // An answer given for the row before is not one for this row.
            harness.app.apply(Action::AnswerConflict {
                at: 0,
                answer: Answer::UseServer,
            });
            assert_eq!(asking(&harness), Some((1, 3, 2)));
            assert_eq!(pending(&harness, tab, id), [(3, 1)]);
            // Nor is Esc pressed twice, which keeps whatever row it finds,
            // nor an answer for a row that is not asked about yet.
            for (at, late) in [
                (0, Answer::KeepMine),
                (0, Answer::Discard),
                (2, Answer::KeepMine),
            ] {
                harness
                    .app
                    .apply(Action::AnswerConflict { at, answer: late });
                assert_eq!(asking(&harness), Some((1, 3, 2)), "{at}, {late:?}");
                let edits = &object(&harness, tab, id).edits;
                assert!(
                    edits.note.is_none() && edits.gone.is_empty(),
                    "{at}, {late:?}"
                );
            }
            assert_eq!(pending(&harness, tab, id), [(3, 1)]);
            answer(&mut harness, Answer::KeepMine);
            assert!(harness.app.dialog.is_none());
            assert_eq!(email(&harness, tab, id, 3), text("fay@example.com"));
            assert_eq!(pending(&harness, tab, id), [(3, 1)]);
        }

        #[test]
        fn a_conflict_that_cannot_be_asked_about_is_a_line() {
            // Under another dialog: the user is in it, and it stays.
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            harness.app.apply(Action::WriteEdits { tab, id });
            harness.app.apply(Action::ShowHelp);
            harness.answer_written(Ok(WriteOutcome::Conflicts(vec![changed(
                0,
                2,
                "eve@example.com",
            )])));
            assert!(matches!(harness.app.dialog, Some(Dialog::Help)));
            let line = Some(crate::edit::Note::Conflict {
                row: 1,
                gone: false,
                others: 0,
            });
            assert_eq!(object(&harness, tab, id).edits.note, line);
            assert_eq!(pending(&harness, tab, id), [(1, 1)]);
            assert_eq!(email(&harness, tab, id, 1), text("user2@example.com"));
            // The next save asks.
            harness.app.apply(Action::CloseDialog);
            harness.app.apply(Action::WriteEdits { tab, id });
            harness.answer_written(Ok(WriteOutcome::Conflicts(vec![changed(
                0,
                2,
                "eve@example.com",
            )])));
            assert_eq!(asking(&harness), Some((0, 1, 1)));
            assert_eq!(object(&harness, tab, id).edits.note, None);
            answer(&mut harness, Answer::KeepMine);
            // A row that is not as wide as the page: the table is no longer
            // the one the page was read from, and there is nothing to put
            // the row into.
            harness.app.apply(Action::WriteEdits { tab, id });
            harness.answer_written(Ok(WriteOutcome::Conflicts(vec![Conflict {
                row: 0,
                server: Some(vec![Value::Int(2)]),
            }])));
            assert!(harness.app.dialog.is_none());
            assert_eq!(object(&harness, tab, id).edits.note, line);
            assert_eq!(pending(&harness, tab, id), [(1, 1)]);
        }

        #[test]
        fn a_page_that_arrives_ends_the_question_about_the_one_it_replaces() {
            let mut harness = Harness::new();
            let (tab, id) = two_conflicts(&mut harness);
            // No fetch runs for a tab that holds edits: one is forged.
            let request = RequestId(u64::MAX);
            let session = harness.app.workspace(tab).unwrap().session;
            harness
                .app
                .workspace_mut(tab)
                .unwrap()
                .object_tab_mut(id)
                .unwrap()
                .rows
                .pending = Some(request);
            harness.app.apply(Action::Backend(Event::Rows {
                session,
                request,
                result: Ok(page(5, false)),
            }));
            assert!(harness.app.dialog.is_none());
            assert!(pending(&harness, tab, id).is_empty());
        }

        #[test]
        fn closing_the_question_leaves_the_rows_it_did_not_ask_about_as_they_were() {
            let mut harness = Harness::new();
            let (tab, id) = two_conflicts(&mut harness);
            let before = harness.app.backend.sent.len();
            answer(&mut harness, Answer::Overwrite);
            harness.app.apply(Action::CloseDialog);
            assert!(harness.app.dialog.is_none());
            // The answered row is settled, the other is as it was loaded,
            // and no save runs for an answer that was never the last.
            assert_eq!(email(&harness, tab, id, 1), text("eve@example.com"));
            assert_eq!(email(&harness, tab, id, 3), text("user4@example.com"));
            assert_eq!(pending(&harness, tab, id), [(1, 1), (3, 1)]);
            assert!(write_since(&harness, before).is_none());
            // The tab says which row is still to be asked about, as where
            // the question is not asked at all: the next save asks.
            let line = |row: usize, others: usize| {
                Some(crate::edit::Note::Conflict {
                    row,
                    gone: false,
                    others,
                })
            };
            assert_eq!(object(&harness, tab, id).edits.note, line(3, 0));
            // Closed before any answer: the first row, and one more.
            let mut harness = Harness::new();
            let (tab, id) = two_conflicts(&mut harness);
            harness.app.apply(Action::CloseDialog);
            assert_eq!(object(&harness, tab, id).edits.note, line(1, 1));
            assert_eq!(email(&harness, tab, id, 1), text("user2@example.com"));
            // Any other dialog is closed as before, and leaves no line.
            harness.app.apply(Action::DismissNote { tab, id });
            harness.app.apply(Action::ShowHelp);
            harness.app.apply(Action::CloseDialog);
            assert!(harness.app.dialog.is_none());
            assert_eq!(object(&harness, tab, id).edits.note, None);
        }

        /// What each statement of the tab's review checks before it runs:
        /// its changed columns, each with the value the page loaded.
        fn checked(harness: &mut Harness, tab: ConnTabId, id: TabId) -> Vec<Vec<(String, String)>> {
            // What a frame does once its actions are applied.
            harness.app.apply_actions();
            let review = object(harness, tab, id).edits.review.as_ref();
            let lines = review.map(|review| review.lines.as_slice());
            lines
                .unwrap_or_default()
                .iter()
                .filter_map(|line| match line {
                    crate::review::Line::Check(loaded) => Some(loaded.clone()),
                    _ => None,
                })
                .collect()
        }

        #[test]
        fn an_open_review_is_made_again_from_the_row_an_answer_left() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            type_into(&mut harness, tab, id, at(3, 1), "dan@example.com");
            show_review(&mut harness, tab, id, true);
            let loaded = |email: &str| vec![("email".to_owned(), format!("'{email}'"))];
            let before = [loaded("user2@example.com"), loaded("user4@example.com")];
            assert_eq!(checked(&mut harness, tab, id), before);
            harness.app.apply(Action::WriteEdits { tab, id });
            harness.answer_written(Ok(WriteOutcome::Conflicts(vec![
                changed(0, 2, "eve@example.com"),
                changed(1, 4, "fay@example.com"),
            ])));
            // Under the question the review is not hidden, and it is made
            // of the rows as they were loaded: nothing is rebased yet.
            show_review(&mut harness, tab, id, false);
            assert!(object(&harness, tab, id).edits.reviewing);
            assert_eq!(checked(&mut harness, tab, id), before);
            // Keep mine: the server's row is the loaded one now, and that
            // is what the save would check. Stale at once, and made again
            // by the frame, with the next row's question up.
            answer(&mut harness, Answer::KeepMine);
            assert!(object(&harness, tab, id).edits.review.is_none());
            assert_eq!(
                checked(&mut harness, tab, id),
                [loaded("eve@example.com"), loaded("user4@example.com")]
            );
            assert_eq!(asking(&harness), Some((1, 3, 2)));
            // Use server values: the row's cells left the set, and its
            // statement the review.
            answer(&mut harness, Answer::UseServer);
            assert_eq!(checked(&mut harness, tab, id), [loaded("eve@example.com")]);
            let one = [
                r#"UPDATE "main"."users""#,
                r#"   SET "email" = 'bob@example.com'"#,
                r#" WHERE "id" = 2;"#,
            ];
            assert_eq!(reviewed(&mut harness, tab, id).unwrap(), one);
            // Overwrite: the save that follows checks against the server's
            // row, and the review says so while it runs.
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            show_review(&mut harness, tab, id, true);
            harness.app.apply(Action::WriteEdits { tab, id });
            harness.answer_written(Ok(WriteOutcome::Conflicts(vec![changed(
                0,
                2,
                "eve@example.com",
            )])));
            answer(&mut harness, Answer::Overwrite);
            assert!(object(&harness, tab, id).edits.saving.is_some());
            assert_eq!(checked(&mut harness, tab, id), [loaded("eve@example.com")]);
            // On production the confirmation that follows holds its own
            // review, made of the same row.
            let mut harness = Harness::new();
            let (tab, id) = production(&mut harness);
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            harness.app.apply(Action::WriteEdits { tab, id });
            harness.app.apply(Action::ConfirmWrite);
            harness.answer_written(Ok(WriteOutcome::Conflicts(vec![changed(
                0,
                2,
                "eve@example.com",
            )])));
            answer(&mut harness, Answer::Overwrite);
            let Some(Dialog::ConfirmWrite(prompt)) = &harness.app.dialog else {
                panic!("expected the confirmation, got {:?}", harness.app.dialog);
            };
            let check = crate::review::Line::Check(loaded("eve@example.com"));
            assert!(prompt.review.lines.contains(&check));
            // A row that is gone and discarded, the only one pending: with
            // no statement left the review closes, as with any empty set.
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            show_review(&mut harness, tab, id, true);
            assert_eq!(checked(&mut harness, tab, id).len(), 1);
            harness.app.apply(Action::WriteEdits { tab, id });
            harness.answer_written(Ok(WriteOutcome::Conflicts(vec![gone(0)])));
            answer(&mut harness, Answer::Discard);
            assert!(checked(&mut harness, tab, id).is_empty());
            let edits = &object(&harness, tab, id).edits;
            assert!(!edits.reviewing && edits.review.is_none());
        }

        #[test]
        fn overwrite_saves_again_with_the_servers_row_as_the_loaded_one() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            harness.app.apply(Action::WriteEdits { tab, id });
            harness.answer_written(Ok(WriteOutcome::Conflicts(vec![changed(
                0,
                2,
                "eve@example.com",
            )])));
            let before = harness.app.backend.sent.len();
            answer(&mut harness, Answer::Overwrite);
            assert!(harness.app.dialog.is_none());
            // The save is sent at once, and checks against what the server
            // held when it was asked.
            let changes = write_since(&harness, before).expect("a second Write");
            assert_eq!(changes.rows.len(), 1);
            assert_eq!(changes.rows[0].set[0].loaded, text("eve@example.com"));
            assert_eq!(
                changes.rows[0].set[0].new,
                NewValue::Text("bob@example.com".into())
            );
            assert!(object(&harness, tab, id).edits.saving.is_some());
            harness.answer_written(written("bob@example.com"));
            assert!(!object(&harness, tab, id).edits.holds());
            assert_eq!(email(&harness, tab, id, 1), text("bob@example.com"));
        }

        #[test]
        fn the_save_runs_again_only_when_a_row_was_overwritten_and_none_was_kept() {
            use Answer::{KeepMine, Overwrite, UseServer};
            // The answers to the page's rows 1 and 3 (`id` 2 and 4), the
            // keys of the rows the second save sends (none: no save), and
            // the cells left pending when no save runs.
            type Case = (Answer, Answer, Option<Vec<i64>>, Vec<(usize, usize)>);
            let cases: Vec<Case> = vec![
                (Overwrite, Overwrite, Some(vec![2, 4]), vec![]),
                (Overwrite, UseServer, Some(vec![2]), vec![]),
                (UseServer, Overwrite, Some(vec![4]), vec![]),
                (Overwrite, KeepMine, None, vec![(1, 1), (3, 1)]),
                (KeepMine, Overwrite, None, vec![(1, 1), (3, 1)]),
                (KeepMine, KeepMine, None, vec![(1, 1), (3, 1)]),
                (KeepMine, UseServer, None, vec![(1, 1)]),
                (UseServer, KeepMine, None, vec![(3, 1)]),
                (UseServer, UseServer, None, vec![]),
            ];
            for (first, second, saved, left) in cases {
                let said = format!("{first:?}, then {second:?}");
                let mut harness = Harness::new();
                let (tab, id) = two_conflicts(&mut harness);
                let before = harness.app.backend.sent.len();
                answer(&mut harness, first);
                // Nothing is sent before the last answer.
                assert!(write_since(&harness, before).is_none(), "{said}");
                answer(&mut harness, second);
                assert!(harness.app.dialog.is_none(), "{said}");
                let keys = write_since(&harness, before).map(|changes| {
                    let key = |row: &tabletist_db::RowChange| row.key[0].1.clone();
                    changes.rows.iter().map(key).collect::<Vec<_>>()
                });
                let expected = saved.map(|ids| ids.into_iter().map(Value::Int).collect::<Vec<_>>());
                assert_eq!(keys, expected, "{said}");
                let edits = &object(&harness, tab, id).edits;
                assert_eq!(edits.saving.is_some(), keys.is_some(), "{said}");
                if keys.is_none() {
                    assert_eq!(pending(&harness, tab, id), left, "{said}");
                    // With nothing left, the tab holds its page no longer.
                    assert_eq!(edits.holds(), !left.is_empty(), "{said}");
                }
                // Whatever the answers, both rows are the server's now.
                let server = |row: usize| email(&harness, tab, id, row);
                assert_eq!(server(1), text("eve@example.com"), "{said}");
                assert_eq!(server(3), text("fay@example.com"), "{said}");
            }
        }

        #[test]
        fn a_discarded_row_neither_asks_for_a_save_nor_stops_one() {
            // A gone row beside an overwritten one: the save runs, without it.
            let mut harness = Harness::new();
            let both = vec![gone(0), changed(1, 4, "fay@example.com")];
            let (tab, id) = conflicts(&mut harness, both);
            let before = harness.app.backend.sent.len();
            answer(&mut harness, Answer::Discard);
            assert_eq!(asking(&harness), Some((1, 3, 2)));
            answer(&mut harness, Answer::Overwrite);
            let changes = write_since(&harness, before).expect("a second Write");
            assert_eq!(changes.rows.len(), 1);
            assert_eq!(changes.rows[0].key, [("id".to_owned(), Value::Int(4))]);
            assert!(object(&harness, tab, id).edits.gone.contains(&1));
            // A gone row beside one that did not conflict: no save runs.
            let mut harness = Harness::new();
            let (tab, id) = conflicts(&mut harness, vec![gone(0)]);
            let before = harness.app.backend.sent.len();
            answer(&mut harness, Answer::Discard);
            assert!(write_since(&harness, before).is_none());
            assert_eq!(pending(&harness, tab, id), [(3, 1)]);
        }

        #[test]
        fn a_row_that_is_gone_and_kept_stops_the_save_as_any_kept_row_does() {
            let mut harness = Harness::new();
            let both = vec![gone(0), changed(1, 4, "fay@example.com")];
            let (tab, id) = conflicts(&mut harness, both);
            let before = harness.app.backend.sent.len();
            // Esc on the row that is gone, then Overwrite on the other.
            answer(&mut harness, Answer::KeepMine);
            answer(&mut harness, Answer::Overwrite);
            assert!(harness.app.dialog.is_none());
            // The save would write the row that is not there: it waits.
            assert!(write_since(&harness, before).is_none());
            assert_eq!(pending(&harness, tab, id), [(1, 1), (3, 1)]);
            assert_eq!(email(&harness, tab, id, 3), text("fay@example.com"));
            assert_eq!(
                object(&harness, tab, id).edits.note,
                Some(crate::edit::Note::Conflict {
                    row: 1,
                    gone: true,
                    others: 0
                })
            );
        }

        #[test]
        fn an_overwrite_that_leaves_nothing_pending_saves_nothing() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            type_into(&mut harness, tab, id, at(1, 1), "eve@example.com");
            harness.app.apply(Action::WriteEdits { tab, id });
            // Someone saved the very value typed here.
            harness.answer_written(Ok(WriteOutcome::Conflicts(vec![Conflict {
                row: 0,
                server: Some(vec![Value::Int(2), text("eve@example.com"), text("{}")]),
            }])));
            // (The conflict is real: the save compares the column with what
            // was loaded, not with what is new.)
            let before = harness.app.backend.sent.len();
            answer(&mut harness, Answer::Overwrite);
            assert!(harness.app.dialog.is_none());
            assert!(write_since(&harness, before).is_none());
            let now = object(&harness, tab, id);
            assert!(!now.edits.holds() && now.edits.note.is_none());
            assert_eq!(email(&harness, tab, id, 1), text("eve@example.com"));
            // The same with the session gone: there was nothing to send,
            // so nothing says that nothing was sent.
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            type_into(&mut harness, tab, id, at(1, 1), "eve@example.com");
            harness.app.apply(Action::WriteEdits { tab, id });
            harness.answer_written(Ok(WriteOutcome::Conflicts(vec![changed(
                0,
                2,
                "eve@example.com",
            )])));
            let session = harness.app.workspace(tab).unwrap().session;
            harness.app.apply(Action::Backend(Event::Disconnected {
                session,
                error: tabletist_db::Error::ConnectionLost("gone".into()),
            }));
            answer(&mut harness, Answer::Overwrite);
            let now = object(&harness, tab, id);
            assert!(!now.edits.holds() && now.edits.note.is_none());
        }

        #[test]
        fn a_row_that_changed_once_more_conflicts_once_more() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            harness.app.apply(Action::WriteEdits { tab, id });
            harness.answer_written(Ok(WriteOutcome::Conflicts(vec![changed(
                0,
                2,
                "eve@example.com",
            )])));
            answer(&mut harness, Answer::Overwrite);
            // Changed again between the question and the second save.
            harness.answer_written(Ok(WriteOutcome::Conflicts(vec![changed(
                0,
                2,
                "zoe@example.com",
            )])));
            assert_eq!(asking(&harness), Some((0, 1, 1)));
            // What it shows as loaded is the row of the first answer.
            assert_eq!(email(&harness, tab, id, 1), text("eve@example.com"));
            let before = harness.app.backend.sent.len();
            answer(&mut harness, Answer::KeepMine);
            assert_eq!(email(&harness, tab, id, 1), text("zoe@example.com"));
            assert_eq!(pending(&harness, tab, id), [(1, 1)]);
            assert!(write_since(&harness, before).is_none());
        }

        #[test]
        fn on_production_the_save_after_an_overwrite_is_confirmed_again() {
            let mut harness = Harness::new();
            let (tab, id) = production(&mut harness);
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            harness.app.apply(Action::WriteEdits { tab, id });
            harness.app.apply(Action::ConfirmWrite);
            assert_eq!(writes(&harness), 1);
            harness.answer_written(Ok(WriteOutcome::Conflicts(vec![changed(
                0,
                2,
                "eve@example.com",
            )])));
            answer(&mut harness, Answer::Overwrite);
            // Asked again, with the statements of the save as it is now.
            assert_eq!(writes(&harness), 1);
            let shown = confirming(&harness).expect("the confirmation");
            assert_eq!(shown.rows[0].set[0].loaded, text("eve@example.com"));
            // Cancelled: nothing is sent and the cell stays pending.
            harness.app.apply(Action::CancelWrite);
            assert_eq!(writes(&harness), 1);
            assert_eq!(pending(&harness, tab, id), [(1, 1)]);
            // Confirmed: it is sent.
            harness.app.apply(Action::WriteEdits { tab, id });
            harness.app.apply(Action::ConfirmWrite);
            assert_eq!(writes(&harness), 2);
        }

        #[test]
        fn what_a_save_was_to_be_followed_by_is_dropped_by_its_conflict_for_good() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            harness.app.apply(Action::CloseTab { tab, id });
            harness.app.apply(Action::LeaveSave);
            harness.answer_written(Ok(WriteOutcome::Conflicts(vec![changed(
                0,
                2,
                "eve@example.com",
            )])));
            answer(&mut harness, Answer::Overwrite);
            assert_eq!(writes(&harness), 2);
            harness.answer_written(written("bob@example.com"));
            // Everything is written now, and the tab is still open: the
            // close was dropped with the first save.
            assert!(harness.app.dialog.is_none());
            let now = object(&harness, tab, id);
            assert!(!now.edits.holds());
            assert_eq!(email(&harness, tab, id, 1), text("bob@example.com"));
        }

        #[test]
        fn an_overwrite_after_the_session_went_sends_nothing_and_says_so() {
            let mut harness = Harness::new();
            let (tab, id) = conflicts(&mut harness, vec![changed(0, 2, "eve@example.com")]);
            let session = harness.app.workspace(tab).unwrap().session;
            harness.app.apply(Action::Backend(Event::Disconnected {
                session,
                error: tabletist_db::Error::ConnectionLost("gone".into()),
            }));
            // The question stays, and its answer still settles the row.
            assert_eq!(asking(&harness), Some((0, 1, 1)));
            let before = harness.app.backend.sent.len();
            answer(&mut harness, Answer::Overwrite);
            assert!(harness.app.dialog.is_none());
            assert!(write_since(&harness, before).is_none());
            assert_eq!(email(&harness, tab, id, 1), text("eve@example.com"));
            assert_eq!(pending(&harness, tab, id), [(1, 1), (3, 1)]);
            assert_eq!(
                object(&harness, tab, id).edits.note,
                Some(crate::edit::Note::NotSent)
            );
        }

        /// Whether the confirmation that is up was opened by the answer to
        /// another dialog. `None` when no confirmation is up.
        fn after_an_answer(harness: &Harness) -> Option<bool> {
            match &harness.app.dialog {
                Some(Dialog::ConfirmWrite(prompt)) => Some(prompt.after_answer.is_some()),
                _ => None,
            }
        }

        #[test]
        fn a_confirmation_knows_whether_the_answer_to_another_dialog_opened_it() {
            // Asked for by Save itself, from a key, the bar or the prompt's
            // `:w`: it answers at once, as before.
            let mut harness = Harness::new();
            let (tab, id) = production(&mut harness);
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            harness.app.apply(Action::WriteEdits { tab, id });
            assert_eq!(after_an_answer(&harness), Some(false));
            harness.app.apply(Action::CancelWrite);
            harness.app.workspace_mut(tab).unwrap().command = Some("w".into());
            harness.app.apply(Action::RunCommand(tab));
            assert_eq!(after_an_answer(&harness), Some(false));
            harness.app.apply(Action::CancelWrite);
            // Save in the Leave prompt: the confirmation takes that
            // prompt's place.
            harness.app.apply(Action::CloseTab { tab, id });
            harness.app.apply(Action::LeaveSave);
            assert_eq!(after_an_answer(&harness), Some(true));
            // Overwrite in the conflict question: the same.
            harness.app.apply(Action::ConfirmWrite);
            harness.answer_written(Ok(WriteOutcome::Conflicts(vec![changed(
                0,
                2,
                "eve@example.com",
            )])));
            answer(&mut harness, Answer::Overwrite);
            assert_eq!(after_an_answer(&harness), Some(true));
            // Asked for again by Save, it is an ordinary one again.
            harness.app.apply(Action::CancelWrite);
            harness.app.apply(Action::WriteEdits { tab, id });
            assert_eq!(after_an_answer(&harness), Some(false));
            // Where no confirmation is needed, the answer opens none.
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            harness.app.apply(Action::CloseTab { tab, id });
            harness.app.apply(Action::LeaveSave);
            assert_eq!(after_an_answer(&harness), None);
            assert_eq!(writes(&harness), 1);
        }

        /// The tab's open editor: its cell, where it is drawn and whether
        /// it is the large one.
        fn editor(
            harness: &Harness,
            tab: ConnTabId,
            id: TabId,
        ) -> Option<(CellPos, EditorPlace, bool)> {
            let editor = object(harness, tab, id).edits.editor.as_ref()?;
            Some((editor.cell, editor.place, editor.large))
        }

        /// Sets the open editor's text as its field would.
        fn type_text(harness: &mut Harness, tab: ConnTabId, id: TabId, text: &str) {
            let object = harness
                .app
                .workspace_mut(tab)
                .unwrap()
                .object_tab_mut(id)
                .unwrap();
            object.edits.editor.as_mut().expect("an editor").text = text.to_owned();
            harness.app.apply(Action::EditorTyped { tab, id });
        }

        #[test]
        fn an_edit_asked_for_in_the_row_panel_opens_there_and_makes_the_same_pending_cell() {
            let mut harness = Harness::new();
            harness.set_look(crate::theme::Look::macos());
            let (tab, id) = harness.editable();
            // In the grid, as ever.
            harness.app.apply(Action::EditCell {
                tab,
                id,
                cell: at(1, 1),
                start: EditStart::Value,
            });
            assert_eq!(
                editor(&harness, tab, id),
                Some((at(1, 1), EditorPlace::Grid, false))
            );
            harness.app.apply(Action::CancelEdit { tab, id });
            assert_eq!(object(&harness, tab, id).focus_field, None);
            // In the panel: the same editor, on the same cell, which is
            // the selected one, in a tab that is no preview any more.
            harness.app.apply(Action::EditField {
                tab,
                id,
                cell: at(2, 1),
            });
            assert_eq!(
                editor(&harness, tab, id),
                Some((at(2, 1), EditorPlace::Panel, false))
            );
            let tab_now = object(&harness, tab, id);
            assert_eq!(tab_now.selection, Some(at(2, 1)));
            assert!(tab_now.pinned);
            assert_eq!(
                tab_now.edits.editor.as_ref().unwrap().text,
                "user3@example.com"
            );
            // Committed as a cell's field commits (Enter there moves down):
            // the text is the pending cell an edit in the grid makes, the
            // selection stays on the field, and the keyboard goes back to
            // it.
            type_text(&mut harness, tab, id, "cy@example.com");
            harness.app.apply(Action::CommitEdit {
                tab,
                id,
                then: Advance::Down,
            });
            let tab_now = object(&harness, tab, id);
            assert!(tab_now.edits.editor.is_none());
            assert_eq!(
                tab_now.edits.cells.get(&(2, 1)).map(|cell| &cell.new),
                Some(&NewValue::Text("cy@example.com".into()))
            );
            assert_eq!(tab_now.selection, Some(at(2, 1)));
            assert_eq!(tab_now.focus_field, Some(1));
            // Opened again in the panel it starts from the pending text.
            harness.app.apply(Action::EditField {
                tab,
                id,
                cell: at(2, 1),
            });
            let text = &object(&harness, tab, id)
                .edits
                .editor
                .as_ref()
                .unwrap()
                .text;
            assert_eq!(text, "cy@example.com");
        }

        #[test]
        fn an_edit_that_ends_in_the_panel_gives_the_keyboard_back_to_its_field() {
            for look in crate::theme::Look::ALL {
                let mut harness = Harness::new();
                harness.set_look(look);
                let (tab, id) = harness.editable();
                let field = |harness: &Harness| object(harness, tab, id).focus_field;
                let cell = at(1, 1);
                // Dropped, and committed: on the terminal's look the keys
                // are the grid's again, and nothing is asked of the panel.
                let back = (!look.terminal).then_some(1);
                harness.app.apply(Action::EditField { tab, id, cell });
                harness.app.apply(Action::CancelEdit { tab, id });
                assert_eq!(field(&harness), back, "{}", look.name);
                // It is owed until the panel says that field has it: what
                // the panel says of another column forgets nothing.
                let focused = |col| Action::FieldFocused { tab, id, col };
                harness.app.apply(focused(2));
                assert_eq!(field(&harness), back, "{}", look.name);
                harness.app.apply(focused(1));
                assert_eq!(field(&harness), None, "{}", look.name);
                harness.app.apply(Action::EditField { tab, id, cell });
                harness.app.apply(Action::CancelEdit { tab, id });
                assert_eq!(field(&harness), back, "{}", look.name);
                // The field is the selected row's: once the selection
                // moves, by a click or a key, no field is owed the keyboard.
                harness.app.apply(Action::SelectCell { tab, id, cell });
                assert_eq!(field(&harness), None, "{}", look.name);
                harness.app.apply(Action::EditField { tab, id, cell });
                harness.app.apply(Action::CommitEdit {
                    tab,
                    id,
                    then: Advance::Stay,
                });
                assert_eq!(field(&harness), back, "{}", look.name);
                harness.app.apply(Action::MoveSelection {
                    tab,
                    id,
                    rows: 1,
                    cols: 0,
                });
                assert_eq!(field(&harness), None, "{}", look.name);
                // Nor once the panel closes: opened again, it starts as
                // any panel does.
                harness.app.apply(Action::EditField { tab, id, cell });
                harness.app.apply(Action::CancelEdit { tab, id });
                harness.app.apply(Action::ToggleRowPanel(tab));
                assert_eq!(field(&harness), None, "{}", look.name);
                harness.app.apply(Action::ToggleRowPanel(tab));
                // Left, the keyboard is where the user put it.
                harness.app.apply(Action::EditField { tab, id, cell });
                let left = crate::testing::leave_edit(&harness.app, tab, id);
                harness.app.apply(left);
                assert!(editor(&harness, tab, id).is_none());
                assert_eq!(field(&harness), None, "{}", look.name);
            }
        }

        #[test]
        fn a_tall_value_asked_for_in_the_row_panel_is_edited_at_its_cell() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            // A document: the popover at the cell is its only editor yet.
            harness.app.apply(Action::EditField {
                tab,
                id,
                cell: at(0, 2),
            });
            assert_eq!(
                editor(&harness, tab, id),
                Some((at(0, 2), EditorPlace::Grid, true))
            );
            harness.app.apply(Action::CancelEdit { tab, id });
            assert_eq!(object(&harness, tab, id).focus_field, None);
            // A line break typed into a field of the panel moves the edit
            // there too.
            harness.app.apply(Action::EditField {
                tab,
                id,
                cell: at(1, 1),
            });
            harness.app.apply(Action::EditorBreak { tab, id });
            assert_eq!(
                editor(&harness, tab, id),
                Some((at(1, 1), EditorPlace::Grid, true))
            );
            let text = &object(&harness, tab, id)
                .edits
                .editor
                .as_ref()
                .unwrap()
                .text;
            assert_eq!(text, "user2@example.com\n");
        }

        #[test]
        fn leaving_closes_only_the_editor_that_was_left() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            let leave = |cell, place| Action::LeaveEdit {
                tab,
                id,
                cell,
                place,
            };
            // The grid's field on one cell loses the keyboard to a pencil
            // of the panel: the panel is drawn first, so its edit is asked
            // for before the grid's field says it was left.
            harness.app.apply(Action::EditCell {
                tab,
                id,
                cell: at(1, 1),
                start: EditStart::Value,
            });
            type_text(&mut harness, tab, id, "bob@example.com");
            harness.app.apply(Action::EditField {
                tab,
                id,
                cell: at(2, 1),
            });
            harness.app.apply(leave(at(1, 1), EditorPlace::Grid));
            // The new editor is open, and the old one's text was kept.
            assert_eq!(
                editor(&harness, tab, id),
                Some((at(2, 1), EditorPlace::Panel, false))
            );
            let edits = &object(&harness, tab, id).edits;
            assert_eq!(
                edits.cells.get(&(1, 1)).map(|cell| &cell.new),
                Some(&NewValue::Text("bob@example.com".into()))
            );
            // The same cell in the other place is another editor too.
            harness.app.apply(leave(at(2, 1), EditorPlace::Grid));
            assert!(editor(&harness, tab, id).is_some());
            // Its own leaving closes it.
            harness.app.apply(leave(at(2, 1), EditorPlace::Panel));
            assert!(editor(&harness, tab, id).is_none());
        }

        #[test]
        fn a_refused_edit_remembers_where_it_was_asked_for() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            let why = |harness: &Harness| {
                let edits = &object(harness, tab, id).edits;
                edits.why.map(|(cell, lock)| (cell, lock, edits.why_place))
            };
            harness.app.apply(Action::EditField {
                tab,
                id,
                cell: at(0, 0),
            });
            assert!(editor(&harness, tab, id).is_none());
            assert_eq!(
                why(&harness),
                Some((at(0, 0), Lock::KeyColumn, EditorPlace::Panel))
            );
            harness.app.apply(Action::EditCell {
                tab,
                id,
                cell: at(0, 0),
                start: EditStart::Value,
            });
            assert_eq!(
                why(&harness),
                Some((at(0, 0), Lock::KeyColumn, EditorPlace::Grid))
            );
        }

        #[test]
        fn closing_the_row_panel_closes_the_editor_it_draws_and_keeps_the_text() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            let panel = |harness: &Harness| harness.app.workspace(tab).unwrap().row_panel;
            assert!(panel(&harness));
            // The grid's editor is not the panel's to close.
            harness.app.apply(Action::EditCell {
                tab,
                id,
                cell: at(1, 1),
                start: EditStart::Value,
            });
            harness.app.apply(Action::ToggleRowPanel(tab));
            assert!(!panel(&harness));
            assert!(editor(&harness, tab, id).is_some());
            harness.app.apply(Action::CancelEdit { tab, id });
            // Opening the panel closes nothing.
            harness.app.apply(Action::ToggleRowPanel(tab));
            harness.app.apply(Action::EditField {
                tab,
                id,
                cell: at(1, 1),
            });
            type_text(&mut harness, tab, id, "bob@example.com");
            // Closed from another tab, where the editor waits: the panel
            // is the workspace's.
            let sql = harness.add_sql_tab(tab);
            harness.app.apply(Action::ActivateTab { tab, id: sql });
            assert!(editor(&harness, tab, id).is_some());
            harness.app.apply(Action::ToggleRowPanel(tab));
            assert!(!panel(&harness));
            let edits = &object(&harness, tab, id).edits;
            assert!(edits.editor.is_none());
            assert_eq!(
                edits.cells.get(&(1, 1)).map(|cell| &cell.new),
                Some(&NewValue::Text("bob@example.com".into()))
            );
            // The Structure view shows no panel: switching to it closes
            // the panel's editor the same way, as it closes a cell's.
            harness.app.apply(Action::ActivateTab { tab, id });
            harness.app.apply(Action::ToggleRowPanel(tab));
            harness.app.apply(Action::EditField {
                tab,
                id,
                cell: at(2, 1),
            });
            type_text(&mut harness, tab, id, "cy@example.com");
            harness.app.apply(Action::SetView {
                tab,
                object_tab: id,
                view: crate::model::ObjectView::Structure,
            });
            let edits = &object(&harness, tab, id).edits;
            assert!(edits.editor.is_none());
            assert_eq!(
                edits.cells.get(&(2, 1)).map(|cell| &cell.new),
                Some(&NewValue::Text("cy@example.com".into()))
            );
        }

        #[test]
        fn the_row_panels_actions_are_dropped_under_a_prompt_about_the_changes() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            harness.app.apply(Action::EditField {
                tab,
                id,
                cell: at(2, 1),
            });
            type_text(&mut harness, tab, id, "cy@example.com");
            harness.app.apply(Action::CloseTab { tab, id });
            assert!(leave_prompt(&harness).is_some());
            // The panel stays open, and its editor with it: closing it
            // would put the text into the set the prompt asks about.
            harness.app.apply(Action::ToggleRowPanel(tab));
            assert!(harness.app.workspace(tab).unwrap().row_panel);
            // No other field opens either.
            harness.app.apply(Action::EditField {
                tab,
                id,
                cell: at(3, 1),
            });
            assert_eq!(
                editor(&harness, tab, id),
                Some((at(2, 1), EditorPlace::Panel, false))
            );
            assert_eq!(object(&harness, tab, id).edits.cells.len(), 1);
        }

        #[test]
        fn editing_the_row_takes_the_selected_cells_field_or_the_first_that_can_be_edited() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            let panel = |harness: &Harness| harness.app.workspace(tab).unwrap().row_panel;
            // With no row selected there is nothing to edit.
            harness.app.apply(Action::EditRow { tab, id });
            assert!(editor(&harness, tab, id).is_none());
            // The selected cell's field, in the panel, which is shown.
            harness.app.apply(Action::ToggleRowPanel(tab));
            assert!(!panel(&harness));
            harness.app.apply(Action::SelectCell {
                tab,
                id,
                cell: at(1, 1),
            });
            harness.app.apply(Action::EditRow { tab, id });
            assert!(panel(&harness));
            assert_eq!(
                editor(&harness, tab, id),
                Some((at(1, 1), EditorPlace::Panel, false))
            );
            harness.app.apply(Action::CancelEdit { tab, id });
            // On the key's cell, which is locked: the row's first field
            // that can be edited.
            harness.app.apply(Action::SelectCell {
                tab,
                id,
                cell: at(2, 0),
            });
            harness.app.apply(Action::EditRow { tab, id });
            assert_eq!(
                editor(&harness, tab, id),
                Some((at(2, 1), EditorPlace::Panel, false))
            );
            assert_eq!(object(&harness, tab, id).edits.why, None);
            // An editor open in the grid gives way, its text kept.
            harness.app.apply(Action::EditCell {
                tab,
                id,
                cell: at(3, 1),
                start: EditStart::Value,
            });
            type_text(&mut harness, tab, id, "dan@example.com");
            harness.app.apply(Action::EditRow { tab, id });
            assert_eq!(
                editor(&harness, tab, id),
                Some((at(3, 1), EditorPlace::Panel, false))
            );
            let text = &object(&harness, tab, id)
                .edits
                .editor
                .as_ref()
                .unwrap()
                .text;
            assert_eq!(text, "dan@example.com");
        }

        #[test]
        fn editing_a_row_that_cannot_be_edited_shows_the_panel_and_says_why() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            let why = |harness: &Harness| {
                let edits = &object(harness, tab, id).edits;
                edits.why.map(|(cell, lock)| (cell, lock, edits.why_place))
            };
            harness.app.apply(Action::ToggleRowPanel(tab));
            harness.app.apply(Action::SelectCell {
                tab,
                id,
                cell: at(1, 1),
            });
            // No cell of the row can be edited: the reason is the row's,
            // kept for the selected cell.
            harness.app.workspace_mut(tab).unwrap().access = tabletist_db::Access::ReadOnly;
            harness.app.apply(Action::EditRow { tab, id });
            assert!(harness.app.workspace(tab).unwrap().row_panel);
            assert!(editor(&harness, tab, id).is_none());
            assert_eq!(
                why(&harness),
                Some((at(1, 1), Lock::ReadOnly, EditorPlace::Panel))
            );
            // Each cell is locked for a reason of its own: the selected
            // cell's is the one said.
            harness.app.workspace_mut(tab).unwrap().access = tabletist_db::Access::Writable;
            let mut structure = crate::testing::fixture_structure();
            for column in &mut structure.columns[1..] {
                column.generated = true;
            }
            harness
                .app
                .workspace_mut(tab)
                .unwrap()
                .object_tab_mut(id)
                .unwrap()
                .structure
                .value = Some(structure);
            harness.app.apply(Action::EditRow { tab, id });
            assert!(editor(&harness, tab, id).is_none());
            assert_eq!(
                why(&harness),
                Some((at(1, 1), Lock::Generated, EditorPlace::Panel))
            );
            // In the Structure view there is no panel to edit in.
            harness.app.apply(Action::SetView {
                tab,
                object_tab: id,
                view: crate::model::ObjectView::Structure,
            });
            harness
                .app
                .workspace_mut(tab)
                .unwrap()
                .object_tab_mut(id)
                .unwrap()
                .edits
                .why = None;
            harness.app.apply(Action::EditRow { tab, id });
            assert_eq!(why(&harness), None);
        }

        #[test]
        fn editing_the_row_is_dropped_under_a_prompt_about_the_changes() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            harness.app.apply(Action::CloseTab { tab, id });
            assert!(leave_prompt(&harness).is_some());
            harness.app.apply(Action::EditRow { tab, id });
            assert!(editor(&harness, tab, id).is_none());
        }
    }
}
