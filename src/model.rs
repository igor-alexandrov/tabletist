//! Application state types and the actions that change them.

use tabletist_db::{ConnectSpec, Driver, Filter, FilterOp, SshAuth, SshSpec, TlsMode};

use crate::backend::{Event, RequestId, SessionId};
use crate::connections::{ColorTag, ConnectionId, PasswordMode};

/// Identifies a connection tab for its whole life, whatever its position.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ConnTabId(pub u64);

/// One tab in the connection tab bar: one connection, or the picker that
/// chooses one.
#[derive(Debug)]
pub struct ConnTab {
    pub id: ConnTabId,
    pub content: ConnTabContent,
}

#[derive(Debug)]
pub enum ConnTabContent {
    /// Choose a saved connection (or create one).
    Picker(PickerState),
    /// A connection, connected or not.
    Workspace(Box<Workspace>),
}

#[derive(Debug, Default)]
pub struct PickerState {
    /// Text typed into the picker's search field.
    pub search: String,
}

/// Everything that changes application state. Views push these; `App::apply`
/// applies them after the frame is drawn.
#[derive(Debug)]
pub enum Action {
    /// Open a new picker tab and make it active.
    NewConnTab,
    CloseConnTab(ConnTabId),
    ActivateConnTab(ConnTabId),
    /// Cmd/Ctrl+1..9: activate the tab at this position, if there is one.
    ActivateConnTabIndex(usize),
    /// Ctrl+Tab (+1) and Ctrl+Shift+Tab (-1), wrapping around.
    CycleConnTab(isize),
    /// A result from the backend.
    Backend(Event),
    /// Open the connection dialog for a new connection.
    NewConnection,
    EditConnection(ConnectionId),
    DuplicateConnection(ConnectionId),
    DeleteConnection(ConnectionId),
    CloseDialog,
    /// Open the native file dialog for the connection dialog's SQLite path.
    PickSqliteFile,
    /// Open the native file dialog for the SSH key file.
    PickKeyFile,
    /// Fill the connection dialog from its URL field.
    ApplyUrl,
    TestConnection,
    /// Trust the SSH host key the prompt shows, then reconnect.
    TrustHostKey,
    /// Close the host key prompt without trusting anything.
    CancelHostKey,
    /// Trust the key the dialog's Test ran into, then test again.
    TrustTestHostKey,
    /// Save the dialog's connection; `connect` also opens it in the active tab.
    SaveConnection {
        connect: bool,
    },
    /// Open a saved connection in this tab (replacing its picker).
    Connect {
        tab: ConnTabId,
        conn: ConnectionId,
    },
    Reconnect(ConnTabId),
    /// Close the connection and turn the tab back into a picker.
    Disconnect(ConnTabId),
    ToggleSchema {
        tab: ConnTabId,
        schema: String,
    },
    ToggleGroup {
        tab: ConnTabId,
        schema: String,
        kind: ObjectKind,
    },
    RefreshTree(ConnTabId),
    OpenObject {
        tab: ConnTabId,
        object: ObjectRef,
        kind: ObjectKind,
        pin: bool,
    },
    ActivateObjectTab {
        tab: ConnTabId,
        object_tab: ObjectTabId,
    },
    CloseObjectTab {
        tab: ConnTabId,
        object_tab: ObjectTabId,
    },
    PinObjectTab {
        tab: ConnTabId,
        object_tab: ObjectTabId,
    },
    CycleObjectTab {
        tab: ConnTabId,
        step: isize,
    },
    SetView {
        tab: ConnTabId,
        object_tab: ObjectTabId,
        view: ObjectView,
    },
    NextPage {
        tab: ConnTabId,
        object_tab: ObjectTabId,
    },
    PrevPage {
        tab: ConnTabId,
        object_tab: ObjectTabId,
    },
    SortBy {
        tab: ConnTabId,
        object_tab: ObjectTabId,
        column: String,
    },
    SelectCell {
        tab: ConnTabId,
        object_tab: ObjectTabId,
        cell: CellPos,
    },
    /// Arrow keys (±1), Page Up/Down (±page), Home/End (isize::MIN/MAX).
    MoveSelection {
        tab: ConnTabId,
        object_tab: ObjectTabId,
        rows: isize,
        cols: isize,
    },
    ToggleRowPanel(ConnTabId),
    /// Refetch the active object tab (and its structure if loaded).
    Refresh(ConnTabId),
    CancelQuery(ConnTabId),
    /// A key for the sidebar tree.
    TreeKey {
        tab: ConnTabId,
        key: TreeKey,
    },
    /// Put the tree's cursor on a row (a click) and give the tree the arrows.
    SetTreeCursor {
        tab: ConnTabId,
        node: TreeNode,
    },
    /// Show the keyboard shortcuts.
    ShowHelp,
    /// Open quick open for the active connection tab.
    OpenQuickOpen,
    /// Move quick open's selection by this many results.
    QuickOpenMove(isize),
    /// Open (and pin) quick open's selected result.
    QuickOpenPick,
    /// Put keyboard focus back into the open filter bar.
    FocusFilterBar(ConnTabId),
    /// Open or close the active object tab's filter bar.
    ToggleFilterBar(ConnTabId),
    AddFilterRow {
        tab: ConnTabId,
        object_tab: ObjectTabId,
    },
    RemoveFilterRow {
        tab: ConnTabId,
        object_tab: ObjectTabId,
        index: usize,
    },
    /// Query with the filter bar's conditions, from the first page.
    ApplyFilters {
        tab: ConnTabId,
        object_tab: ObjectTabId,
    },
    /// Drop every condition, close the bar, and query everything.
    ClearFilters {
        tab: ConnTabId,
        object_tab: ObjectTabId,
    },
    /// Count the object tab's rows exactly (with its filters).
    CountRows {
        tab: ConnTabId,
        object_tab: ObjectTabId,
    },
    RetryRows {
        tab: ConnTabId,
        object_tab: ObjectTabId,
    },
    RetryStructure {
        tab: ConnTabId,
        object_tab: ObjectTabId,
    },
    /// The connection dialog's driver switch.
    SetDriver(Driver),
    SubmitPassword,
    CancelPassword,
    /// Reconnect this tab to another database on the same server.
    SwitchDatabase {
        tab: ConnTabId,
        database: String,
    },
}

/// A connection tab that is (or was) connected.
#[derive(Debug)]
pub struct Workspace {
    pub session: SessionId,
    pub conn_id: ConnectionId,
    /// Copied from the saved connection when the tab opened, so editing or
    /// deleting the saved entry never breaks an open tab.
    pub name: String,
    pub color: ColorTag,
    pub spec: ConnectSpec,
    pub driver: Driver,
    pub status: SessionStatus,
    pub tree: Tree,
    pub objects: Vec<ObjectTab>,
    pub active_object: Option<ObjectTabId>,
    /// Whether the row panel is open.
    pub row_panel: bool,
    /// An object to open as soon as the session connects (demo mode).
    pub pending_open: Option<(ObjectRef, ObjectKind)>,
    pub password_mode: PasswordMode,
    /// The secrets this tab connected with (kept in memory for Reconnect).
    pub secrets: tabletist_db::Secrets,
    /// Databases on the server (PostgreSQL), for the switcher.
    pub databases: Fetch<Vec<String>>,
    /// Save the typed password in the keyring once the server accepts it.
    pub save_password: bool,
    /// Why the next connect must ask for the password (the saved or typed
    /// one was rejected, or a dialog was open when it was needed).
    pub needs_prompt: Option<String>,
    /// How the SSH password or key passphrase is kept.
    pub ssh_mode: PasswordMode,
    /// Save the typed SSH secret in the keyring once the server accepts it.
    pub save_ssh: bool,
    /// Why the next connect must ask for the SSH secret.
    pub needs_ssh_prompt: Option<String>,
    /// Where the arrow keys go.
    pub pane: Pane,
}

#[derive(Debug)]
pub enum SessionStatus {
    Connecting {
        request: RequestId,
    },
    Connected,
    Disconnected(tabletist_db::Error),
    /// The user cancelled a password or SSH secret prompt.
    Cancelled,
}

#[derive(Debug, Default, PartialEq)]
pub enum TestState {
    #[default]
    Idle,
    Running(RequestId),
    Passed,
    Failed(String),
    /// The SSH host's key is not trusted yet.
    Untrusted {
        host: String,
        port: u16,
        fingerprint: String,
    },
}

/// The connection dialog's fields while it is open.
pub struct ConnectionForm {
    /// `Some` when editing a saved connection, `None` for a new one.
    pub editing: Option<ConnectionId>,
    pub name: String,
    pub color: ColorTag,
    pub driver: Driver,
    pub sqlite_path: String,
    pub host: String,
    /// As typed; checked on save.
    pub port: String,
    pub user: String,
    /// Typed in this dialog only. Empty while editing means "keep the saved one".
    pub password: String,
    pub database: String,
    pub tls: TlsMode,
    pub ca_file: String,
    pub password_mode: PasswordMode,
    /// Editing a connection whose password is in the keyring.
    pub has_saved_password: bool,
    /// Text in the "Paste URL" field.
    pub url: String,
    /// A validation or URL error shown under the fields.
    pub message: Option<String>,
    pub test: TestState,
    /// The file dialog request in flight, if any.
    pub pick_request: Option<RequestId>,
    /// Which field the file dialog fills.
    pub pick_target: PickTarget,
    /// Connect through an SSH tunnel.
    pub ssh: bool,
    pub ssh_host: String,
    /// As typed; checked on save.
    pub ssh_port: String,
    pub ssh_user: String,
    pub ssh_auth: SshAuthKind,
    pub ssh_key_file: String,
    /// The SSH password or key passphrase typed in this dialog.
    pub ssh_secret: String,
    pub ssh_secret_mode: PasswordMode,
    /// Editing a connection whose SSH secret is in the keyring.
    pub has_saved_ssh_secret: bool,
    /// The SSH login method that secret belongs to.
    pub saved_ssh_auth: Option<SshAuthKind>,
    /// The secrets a running Test uses, filled as saved ones load.
    pub test_secrets: tabletist_db::Secrets,
    /// Saved secrets the running Test still waits for.
    pub test_waiting: u8,
}

/// The field a file dialog fills.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PickTarget {
    #[default]
    Sqlite,
    KeyFile,
}

/// How the SSH tunnel logs in, as the dialog offers it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SshAuthKind {
    #[default]
    Password,
    KeyFile,
    Agent,
}

impl SshAuthKind {
    pub const ALL: [Self; 3] = [Self::Password, Self::KeyFile, Self::Agent];

    pub fn label(self) -> &'static str {
        match self {
            Self::Password => "Password",
            Self::KeyFile => "Key file",
            Self::Agent => "Agent",
        }
    }
}

impl Default for ConnectionForm {
    fn default() -> Self {
        Self {
            editing: None,
            name: String::new(),
            color: ColorTag::None,
            driver: Driver::Sqlite,
            sqlite_path: String::new(),
            host: "localhost".into(),
            port: String::new(),
            user: String::new(),
            password: String::new(),
            database: String::new(),
            tls: TlsMode::Prefer,
            ca_file: String::new(),
            password_mode: PasswordMode::None,
            has_saved_password: false,
            url: String::new(),
            message: None,
            test: TestState::Idle,
            pick_request: None,
            pick_target: PickTarget::Sqlite,
            ssh: false,
            ssh_host: String::new(),
            ssh_port: "22".into(),
            ssh_user: String::new(),
            ssh_auth: SshAuthKind::Password,
            ssh_key_file: String::new(),
            ssh_secret: String::new(),
            ssh_secret_mode: PasswordMode::Keyring,
            has_saved_ssh_secret: false,
            saved_ssh_auth: None,
            test_secrets: tabletist_db::Secrets::default(),
            test_waiting: 0,
        }
    }
}

impl ConnectionForm {
    pub fn from_saved(saved: &crate::connections::SavedConnection) -> Self {
        let spec = &saved.spec;
        Self {
            editing: Some(saved.id.clone()),
            name: saved.name.clone(),
            color: saved.color,
            driver: spec.driver,
            sqlite_path: spec
                .sqlite_path
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_default(),
            host: spec.host.clone(),
            port: if spec.port == 0 {
                String::new()
            } else {
                spec.port.to_string()
            },
            user: spec.user.clone(),
            database: spec.database.clone(),
            tls: spec.tls,
            ca_file: spec
                .ca_file
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_default(),
            password_mode: saved.password,
            has_saved_password: saved.password == PasswordMode::Keyring,
            ssh: spec.ssh.is_some(),
            ssh_host: spec
                .ssh
                .as_ref()
                .map(|ssh| ssh.host.clone())
                .unwrap_or_default(),
            ssh_port: spec
                .ssh
                .as_ref()
                .map_or_else(|| "22".into(), |ssh| ssh.port.to_string()),
            ssh_user: spec
                .ssh
                .as_ref()
                .map(|ssh| ssh.user.clone())
                .unwrap_or_default(),
            ssh_auth: match spec.ssh.as_ref().map(|ssh| &ssh.auth) {
                Some(SshAuth::KeyFile { .. }) => SshAuthKind::KeyFile,
                Some(SshAuth::Agent) => SshAuthKind::Agent,
                _ => SshAuthKind::Password,
            },
            ssh_key_file: match spec.ssh.as_ref().map(|ssh| &ssh.auth) {
                Some(SshAuth::KeyFile { path }) => path.display().to_string(),
                _ => String::new(),
            },
            ssh_secret_mode: match saved.ssh_secret {
                PasswordMode::Ask => PasswordMode::Ask,
                _ => PasswordMode::Keyring,
            },
            has_saved_ssh_secret: saved.ssh_secret == PasswordMode::Keyring,
            saved_ssh_auth: spec.ssh.as_ref().map(|ssh| match ssh.auth {
                SshAuth::Password => SshAuthKind::Password,
                SshAuth::KeyFile { .. } => SshAuthKind::KeyFile,
                SshAuth::Agent => SshAuthKind::Agent,
            }),
            ..Self::default()
        }
    }

    /// What to connect to, or why the fields do not describe a connection.
    pub fn to_spec(&self) -> Result<ConnectSpec, String> {
        match self.driver {
            Driver::Sqlite => {
                let path = self.sqlite_path.trim();
                if path.is_empty() {
                    return Err("Choose a SQLite file.".into());
                }
                Ok(ConnectSpec::sqlite(path))
            }
            Driver::Postgres | Driver::MySql => {
                let host = self.host.trim();
                if host.is_empty() {
                    return Err("Enter a host.".into());
                }
                let port: u16 = self
                    .port
                    .trim()
                    .parse()
                    .map_err(|_| "Enter a port number.".to_owned())?;
                let user = self.user.trim();
                if user.is_empty() {
                    return Err("Enter a user name.".into());
                }
                let ca_file = self.ca_file.trim();
                let ssh = if self.ssh {
                    let host = self.ssh_host.trim();
                    if host.is_empty() {
                        return Err("Enter the SSH host.".into());
                    }
                    let port: u16 = self
                        .ssh_port
                        .trim()
                        .parse()
                        .map_err(|_| "Enter the SSH port number.".to_owned())?;
                    let user = self.ssh_user.trim();
                    if user.is_empty() {
                        return Err("Enter the SSH user.".into());
                    }
                    let auth = match self.ssh_auth {
                        SshAuthKind::Password => SshAuth::Password,
                        SshAuthKind::Agent => SshAuth::Agent,
                        SshAuthKind::KeyFile => {
                            let path = self.ssh_key_file.trim();
                            if path.is_empty() {
                                return Err("Choose a key file for SSH.".into());
                            }
                            SshAuth::KeyFile { path: path.into() }
                        }
                    };
                    Some(SshSpec {
                        host: host.to_owned(),
                        port,
                        user: user.to_owned(),
                        auth,
                    })
                } else {
                    None
                };
                Ok(ConnectSpec {
                    driver: self.driver,
                    host: host.to_owned(),
                    port,
                    user: user.to_owned(),
                    database: self.database.trim().to_owned(),
                    sqlite_path: None,
                    tls: self.tls,
                    ca_file: (!ca_file.is_empty()).then(|| ca_file.into()),
                    ssh,
                })
            }
        }
    }

    /// Whether the keyring holds the SSH secret for the login method now
    /// chosen. A passphrase saved for a key is never used as a password.
    pub fn ssh_secret_is_saved(&self) -> bool {
        self.has_saved_ssh_secret && self.ssh && self.saved_ssh_auth == Some(self.ssh_auth)
    }

    /// The connection the form describes, or why it cannot be saved.
    pub fn to_saved(&self) -> Result<crate::connections::SavedConnection, String> {
        let name = self.name.trim();
        if name.is_empty() {
            return Err("Give the connection a name.".into());
        }
        let spec = self.to_spec()?;
        let password = if spec.driver == Driver::Sqlite {
            PasswordMode::None
        } else if self.password_mode == PasswordMode::Keyring
            && self.password.is_empty()
            && !self.has_saved_password
        {
            // Nothing typed and nothing saved: a server that needs no
            // password (trust or peer authentication).
            PasswordMode::None
        } else {
            self.password_mode
        };
        let ssh_secret = match (&spec.ssh, self.ssh_auth) {
            (None, _) | (Some(_), SshAuthKind::Agent) => PasswordMode::None,
            // Nothing typed and nothing saved: an unencrypted key, or a
            // password the server will ask for when connecting.
            _ if self.ssh_secret_mode == PasswordMode::Keyring
                && self.ssh_secret.is_empty()
                && !self.ssh_secret_is_saved() =>
            {
                PasswordMode::None
            }
            _ => self.ssh_secret_mode,
        };
        Ok(crate::connections::SavedConnection {
            id: self.editing.clone().unwrap_or_else(ConnectionId::new),
            name: name.to_owned(),
            color: self.color,
            password,
            ssh_secret,
            spec,
        })
    }
}

/// Which secret a prompt asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretKind {
    Database,
    SshPassword,
    SshPassphrase,
}

impl SecretKind {
    /// The SSH secret this spec can use, if any.
    pub fn for_ssh(spec: &ConnectSpec) -> Option<Self> {
        match spec.ssh.as_ref()?.auth {
            SshAuth::Password => Some(Self::SshPassword),
            SshAuth::KeyFile { .. } => Some(Self::SshPassphrase),
            SshAuth::Agent => None,
        }
    }

    /// Where this secret goes in `secrets`.
    pub fn slot(self, secrets: &mut tabletist_db::Secrets) -> &mut Option<String> {
        match self {
            Self::Database => &mut secrets.password,
            Self::SshPassword => &mut secrets.ssh_password,
            Self::SshPassphrase => &mut secrets.ssh_passphrase,
        }
    }

    pub fn is_ssh(self) -> bool {
        self != Self::Database
    }
}

/// Asks for a password before connecting.
pub struct PasswordPrompt {
    pub tab: ConnTabId,
    /// The secret asked for.
    pub kind: SecretKind,
    /// The connection's name, for the prompt's title.
    pub name: String,
    pub password: String,
    /// Save the password in the keyring.
    pub save: bool,
    /// Why the prompt appeared (no saved password, keyring locked).
    pub message: Option<String>,
}

#[derive(Debug)]
pub enum Dialog {
    Connection(Box<ConnectionForm>),
    Password(Box<PasswordPrompt>),
    HostKey(Box<HostKeyPrompt>),
    QuickOpen(Box<QuickOpen>),
    /// The keyboard shortcuts.
    Help,
}

/// Cmd/Ctrl+P: find a loaded table or view by name.
#[derive(Debug)]
pub struct QuickOpen {
    pub tab: ConnTabId,
    pub query: String,
    /// Index into the current matches.
    pub selected: usize,
    /// The selection the list last scrolled to, so it scrolls only when the
    /// selection moves and a wheel scroll is left alone.
    pub scrolled_to: Option<usize>,
    /// The list's scroll offset, in points.
    pub scroll_offset: f32,
}

/// Asks whether to trust an SSH host seen for the first time.
#[derive(Debug)]
pub struct HostKeyPrompt {
    pub tab: ConnTabId,
    pub host: String,
    pub port: u16,
    pub fingerprint: String,
}

/// The file name of `path`, for naming a connection after its file.
pub fn file_name(path: &str) -> String {
    std::path::Path::new(path)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

use std::collections::{HashMap, HashSet};

use tabletist_db::{
    Error, ObjectInfo, ObjectKind, ObjectRef, RowPage, RowQuery, SortDir, Structure,
};

/// Something being fetched: the last good value, the request in flight, and
/// the last error. Only the pending request's result is accepted, which is
/// how stale results are dropped.
#[derive(Debug)]
pub struct Fetch<T> {
    pub value: Option<T>,
    pub pending: Option<RequestId>,
    pub error: Option<Error>,
}

impl<T> Default for Fetch<T> {
    fn default() -> Self {
        Self {
            value: None,
            pending: None,
            error: None,
        }
    }
}

impl<T> Fetch<T> {
    pub fn start(&mut self, request: RequestId) {
        self.pending = Some(request);
        self.error = None;
    }

    /// Applies a result if it answers the pending request. Returns whether it did.
    pub fn finish(&mut self, request: RequestId, result: Result<T, Error>) -> bool {
        if self.pending != Some(request) {
            return false;
        }
        self.pending = None;
        match result {
            Ok(value) => {
                self.value = Some(value);
                self.error = None;
            }
            Err(error) => self.error = Some(error),
        }
        true
    }

    pub fn is_loading(&self) -> bool {
        self.pending.is_some()
    }

    /// Never loaded and not loading.
    pub fn needs_load(&self) -> bool {
        self.value.is_none() && self.pending.is_none() && self.error.is_none()
    }
}

/// The sidebar's state.
#[derive(Debug, Default)]
pub struct Tree {
    pub schemas: Fetch<Vec<String>>,
    pub nodes: HashMap<String, SchemaNode>,
    /// Typed into the sidebar's filter field.
    pub filter: String,
    /// The row the arrow keys move from.
    pub cursor: Option<TreeNode>,
    /// Scroll the cursor row into view on the next frame (it moved by key).
    pub reveal_cursor: bool,
}

/// Where the arrow keys go: the pane the user last worked in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Pane {
    #[default]
    Tree,
    Grid,
}

/// A key pressed while the tree has the arrows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeKey {
    Up,
    Down,
    /// Unfold, or step into an unfolded row.
    Right,
    /// Fold, or climb to the parent.
    Left,
    /// Open an object, fold or unfold anything else.
    Enter,
    Home,
    End,
}

#[derive(Debug, Default)]
pub struct SchemaNode {
    pub expanded: bool,
    pub objects: Fetch<Vec<ObjectInfo>>,
    /// Groups the user folded (Tables, Views...).
    pub collapsed: HashSet<ObjectKind>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TreeNode {
    Schema(String),
    /// "No tables or views" under a loaded schema that has none.
    Empty(String),
    Group(String, ObjectKind),
    Object(ObjectRef, ObjectKind),
}

/// One visible line of the tree.
#[derive(Debug, Clone, PartialEq)]
pub struct TreeRow {
    pub node: TreeNode,
    pub depth: u8,
    /// Schema or object name; empty for groups (the view names the kind).
    pub label: String,
    /// Objects in a group.
    pub count: Option<usize>,
    /// `Some` for rows that fold.
    pub expanded: Option<bool>,
    pub loading: bool,
    pub error: Option<String>,
}

const KINDS: [ObjectKind; 3] = [
    ObjectKind::Table,
    ObjectKind::View,
    ObjectKind::MaterializedView,
];

/// Schemas the database keeps for itself, hidden unless the user asks.
pub fn is_system_schema(driver: Driver, name: &str) -> bool {
    match driver {
        Driver::Postgres => {
            matches!(name, "pg_catalog" | "information_schema")
                || name.starts_with("pg_toast")
                || name.starts_with("pg_temp_")
        }
        Driver::MySql => matches!(
            name,
            "mysql" | "sys" | "performance_schema" | "information_schema"
        ),
        Driver::Sqlite => false,
    }
}

impl Tree {
    /// The lines to draw. A filter shows every loaded match, unfolded.
    pub fn visible_rows(&self, driver: Driver, show_system: bool) -> Vec<TreeRow> {
        let needle = self.filter.trim().to_lowercase();
        let filtering = !needle.is_empty();
        let mut rows = Vec::new();
        let Some(schemas) = &self.schemas.value else {
            return rows;
        };
        for schema in schemas {
            if !show_system && is_system_schema(driver, schema) {
                continue;
            }
            let node = self.nodes.get(schema);
            let expanded = node.is_some_and(|node| node.expanded);
            let matching: Vec<&ObjectInfo> = node
                .and_then(|node| node.objects.value.as_ref())
                .map(|objects| {
                    objects
                        .iter()
                        .filter(|object| !filtering || object.name.to_lowercase().contains(&needle))
                        .collect()
                })
                .unwrap_or_default();
            if filtering && matching.is_empty() {
                continue;
            }
            let open = expanded || filtering;
            rows.push(TreeRow {
                node: TreeNode::Schema(schema.clone()),
                depth: 0,
                label: schema.clone(),
                count: None,
                expanded: Some(open),
                loading: node.is_some_and(|node| node.objects.is_loading()),
                error: node.and_then(|node| node.objects.error.as_ref().map(ToString::to_string)),
            });
            if !open {
                continue;
            }
            let loaded_empty = node
                .and_then(|node| node.objects.value.as_ref())
                .is_some_and(Vec::is_empty);
            if loaded_empty {
                rows.push(TreeRow {
                    node: TreeNode::Empty(schema.clone()),
                    depth: 1,
                    label: "No tables or views".into(),
                    count: None,
                    expanded: None,
                    loading: false,
                    error: None,
                });
            }
            for kind in KINDS {
                let items: Vec<&&ObjectInfo> = matching
                    .iter()
                    .filter(|object| object.kind == kind)
                    .collect();
                if items.is_empty() {
                    continue;
                }
                let folded = !filtering && node.is_some_and(|node| node.collapsed.contains(&kind));
                rows.push(TreeRow {
                    node: TreeNode::Group(schema.clone(), kind),
                    depth: 1,
                    label: String::new(),
                    count: Some(items.len()),
                    expanded: Some(!folded),
                    loading: false,
                    error: None,
                });
                if folded {
                    continue;
                }
                for object in items {
                    rows.push(TreeRow {
                        node: TreeNode::Object(
                            ObjectRef::new(schema.clone(), object.name.clone()),
                            kind,
                        ),
                        depth: 2,
                        label: object.name.clone(),
                        count: None,
                        expanded: None,
                        loading: false,
                        error: None,
                    });
                }
            }
        }
        rows
    }

    pub fn object_info(&self, object: &ObjectRef) -> Option<&ObjectInfo> {
        self.nodes
            .get(&object.schema)?
            .objects
            .value
            .as_ref()?
            .iter()
            .find(|info| info.name == object.name)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ObjectTabId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ObjectView {
    #[default]
    Data,
    Structure,
}

/// A cell in the current page: row index within the page, column index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellPos {
    pub row: usize,
    pub col: usize,
}

/// One open table or view.
#[derive(Debug)]
pub struct ObjectTab {
    pub id: ObjectTabId,
    pub object: ObjectRef,
    pub kind: ObjectKind,
    /// A preview tab (not pinned) is replaced by the next single click.
    pub pinned: bool,
    pub view: ObjectView,
    pub query: RowQuery,
    pub rows: Fetch<RowPage>,
    pub structure: Fetch<Structure>,
    pub selection: Option<CellPos>,
    pub estimated_rows: Option<u64>,
    /// The exact row count for the current filters, when asked for.
    pub count: Fetch<u64>,
    pub filter: FilterBar,
}

/// One condition in the filter bar.
#[derive(Debug, Clone, PartialEq)]
pub struct FilterRow {
    pub column: String,
    pub op: FilterOp,
    pub value: String,
}

/// An object tab's filter bar: rows combined with AND, plus raw SQL.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FilterBar {
    pub open: bool,
    pub rows: Vec<FilterRow>,
    /// Use `raw_text` as a WHERE clause as well.
    pub raw: bool,
    pub raw_text: String,
    /// Move keyboard focus into the bar on the next frame.
    pub focus: bool,
    /// The columns last seen, for when no page is loaded (a filter is
    /// running or failed).
    pub columns: Vec<String>,
}

impl FilterBar {
    /// The filters and raw WHERE the bar describes. Rows without a column,
    /// or without a value for an operator that needs one, are skipped.
    pub fn to_query(&self) -> (Vec<Filter>, Option<String>) {
        let filters = self
            .rows
            .iter()
            .filter(|row| !row.column.is_empty())
            .filter(|row| {
                matches!(row.op, FilterOp::IsNull | FilterOp::IsNotNull)
                    || !row.value.trim().is_empty()
            })
            .map(|row| Filter {
                column: row.column.clone(),
                op: row.op,
                value: row.value.trim().to_owned(),
            })
            .collect();
        let raw = self.raw_text.trim();
        (
            filters,
            (self.raw && !raw.is_empty()).then(|| raw.to_owned()),
        )
    }
}

impl ObjectTab {
    pub fn new(
        id: ObjectTabId,
        object: ObjectRef,
        kind: ObjectKind,
        pinned: bool,
        page_size: u32,
        estimated_rows: Option<u64>,
    ) -> Self {
        Self {
            id,
            query: RowQuery::new(object.clone(), page_size),
            object,
            kind,
            pinned,
            view: ObjectView::Data,
            rows: Fetch::default(),
            structure: Fetch::default(),
            selection: None,
            estimated_rows,
            count: Fetch::default(),
            filter: FilterBar::default(),
        }
    }

    /// Forgets the exact count: the rows it counted changed (filters,
    /// refresh), and a count still running for them is ignored.
    pub fn reset_count(&mut self) {
        self.count = Fetch::default();
    }

    pub fn page(&self) -> Option<&RowPage> {
        self.rows.value.as_ref()
    }

    pub fn sort_of(&self, column: &str) -> Option<SortDir> {
        self.query
            .sort
            .iter()
            .find(|sort| sort.column == column)
            .map(|sort| sort.dir)
    }
}

impl Workspace {
    pub fn object_tab(&self, id: ObjectTabId) -> Option<&ObjectTab> {
        self.objects.iter().find(|tab| tab.id == id)
    }

    pub fn object_tab_mut(&mut self, id: ObjectTabId) -> Option<&mut ObjectTab> {
        self.objects.iter_mut().find(|tab| tab.id == id)
    }

    pub fn active_object_tab(&self) -> Option<&ObjectTab> {
        self.object_tab(self.active_object?)
    }
}

// Hand-written so a typed password never reaches logs or panic messages.
impl std::fmt::Debug for ConnectionForm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConnectionForm")
            .field("editing", &self.editing)
            .field("name", &self.name)
            .field("driver", &self.driver)
            .field("host", &self.host)
            .field("port", &self.port)
            .field("user", &self.user)
            .field("database", &self.database)
            .field("tls", &self.tls)
            .field("password_mode", &self.password_mode)
            .field("password", &"..")
            .finish_non_exhaustive()
    }
}

impl std::fmt::Debug for PasswordPrompt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PasswordPrompt")
            .field("tab", &self.tab)
            .field("name", &self.name)
            .field("save", &self.save)
            .field("message", &self.message)
            .field("password", &"..")
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(name: &str, kind: ObjectKind) -> ObjectInfo {
        ObjectInfo {
            name: name.into(),
            kind,
            estimated_rows: None,
        }
    }

    fn tree() -> Tree {
        let mut tree = Tree::default();
        tree.schemas.value = Some(vec!["public".into(), "billing".into(), "pg_catalog".into()]);
        let mut public = SchemaNode {
            expanded: true,
            ..SchemaNode::default()
        };
        public.objects.value = Some(vec![
            info("orders", ObjectKind::Table),
            info("users", ObjectKind::Table),
            info("active_users", ObjectKind::View),
        ]);
        tree.nodes.insert("public".into(), public);
        let mut billing = SchemaNode::default();
        billing.objects.value = Some(vec![info("invoices", ObjectKind::Table)]);
        tree.nodes.insert("billing".into(), billing);
        tree
    }

    fn labels(rows: &[TreeRow]) -> Vec<String> {
        rows.iter()
            .map(|row| match &row.node {
                TreeNode::Group(_, kind) => format!("{kind:?}({})", row.count.unwrap()),
                _ => row.label.clone(),
            })
            .collect()
    }

    #[test]
    fn expanded_schemas_show_groups_and_objects_and_system_schemas_hide() {
        let rows = tree().visible_rows(Driver::Postgres, false);
        assert_eq!(
            labels(&rows),
            [
                "public",
                "Table(2)",
                "orders",
                "users",
                "View(1)",
                "active_users",
                "billing"
            ]
        );
        let with_system = tree().visible_rows(Driver::Postgres, true);
        assert!(labels(&with_system).contains(&"pg_catalog".to_owned()));
    }

    #[test]
    fn folded_groups_hide_their_objects() {
        let mut tree = tree();
        tree.nodes
            .get_mut("public")
            .unwrap()
            .collapsed
            .insert(ObjectKind::Table);
        assert_eq!(
            labels(&tree.visible_rows(Driver::Postgres, false)),
            ["public", "Table(2)", "View(1)", "active_users", "billing"]
        );
    }

    #[test]
    fn a_filter_shows_loaded_matches_in_every_schema_unfolded() {
        let mut tree = tree();
        tree.filter = "IN".into();
        assert_eq!(
            labels(&tree.visible_rows(Driver::Postgres, false)),
            ["billing", "Table(1)", "invoices"]
        );
        tree.filter = "nothing matches".into();
        assert!(tree.visible_rows(Driver::Postgres, false).is_empty());
    }

    #[test]
    fn system_schemas_are_per_driver() {
        assert!(is_system_schema(Driver::Postgres, "pg_toast_temp_1"));
        assert!(is_system_schema(Driver::MySql, "performance_schema"));
        assert!(!is_system_schema(Driver::Sqlite, "main"));
        assert!(!is_system_schema(Driver::Postgres, "public"));
    }

    #[test]
    fn fetch_accepts_only_the_pending_request() {
        let mut fetch: Fetch<u32> = Fetch::default();
        assert!(fetch.needs_load());
        fetch.start(RequestId(1));
        fetch.start(RequestId(2));
        assert!(!fetch.finish(RequestId(1), Ok(1)));
        assert!(fetch.is_loading());
        assert!(fetch.finish(RequestId(2), Ok(2)));
        assert_eq!(fetch.value, Some(2));
        fetch.start(RequestId(3));
        assert!(fetch.finish(RequestId(3), Err(Error::Cancelled)));
        assert_eq!(fetch.value, Some(2), "an error keeps the last good value");
        assert_eq!(fetch.error, Some(Error::Cancelled));
    }

    fn ssh_form() -> ConnectionForm {
        ConnectionForm {
            name: "Prod".into(),
            driver: Driver::Postgres,
            host: "db.internal".into(),
            port: "5432".into(),
            user: "me".into(),
            ssh: true,
            ssh_host: "bastion".into(),
            ssh_port: "22".into(),
            ssh_user: "ops".into(),
            ssh_auth: SshAuthKind::KeyFile,
            ssh_key_file: "/home/me/.ssh/id_ed25519".into(),
            ..ConnectionForm::default()
        }
    }

    #[test]
    fn an_ssh_form_becomes_a_spec_and_back() {
        let saved = ssh_form().to_saved().unwrap();
        assert_eq!(
            saved.spec.ssh,
            Some(SshSpec {
                host: "bastion".into(),
                port: 22,
                user: "ops".into(),
                auth: SshAuth::KeyFile {
                    path: "/home/me/.ssh/id_ed25519".into()
                },
            })
        );
        assert_eq!(
            saved.ssh_secret,
            PasswordMode::None,
            "unencrypted key, nothing typed"
        );
        let back = ConnectionForm::from_saved(&saved);
        assert!(back.ssh);
        assert_eq!(back.ssh_key_file, "/home/me/.ssh/id_ed25519");
        assert_eq!(back.ssh_auth, SshAuthKind::KeyFile);
    }

    #[test]
    fn an_ssh_form_needs_host_user_port_and_key_file() {
        type Change = fn(&mut ConnectionForm);
        let changes: [(Change, &str); 4] = [
            (|f| f.ssh_host.clear(), "SSH host"),
            (|f| f.ssh_user.clear(), "SSH user"),
            (|f| f.ssh_port = "twenty-two".into(), "SSH port"),
            (|f| f.ssh_key_file.clear(), "key file"),
        ];
        for (change, message) in changes {
            let mut form = ssh_form();
            change(&mut form);
            let error = form.to_spec().unwrap_err();
            assert!(error.contains(message), "{error}");
        }
    }

    #[test]
    fn unticking_ssh_drops_the_tunnel() {
        let mut form = ssh_form();
        form.ssh = false;
        assert_eq!(form.to_spec().unwrap().ssh, None);
    }

    #[test]
    fn a_filter_bar_becomes_filters_and_a_raw_where() {
        use tabletist_db::{Filter, FilterOp};
        let bar = FilterBar {
            open: true,
            rows: vec![
                FilterRow {
                    column: "age".into(),
                    op: FilterOp::Gt,
                    value: "30".into(),
                },
                // Empty value: skipped, except for the NULL tests.
                FilterRow {
                    column: "name".into(),
                    op: FilterOp::Contains,
                    value: " ".into(),
                },
                FilterRow {
                    column: "email".into(),
                    op: FilterOp::IsNull,
                    value: String::new(),
                },
                // No column: skipped.
                FilterRow {
                    column: String::new(),
                    op: FilterOp::Eq,
                    value: "x".into(),
                },
            ],
            raw: true,
            raw_text: "  id % 2 = 0 ".into(),
            focus: false,
            columns: Vec::new(),
        };
        let (filters, raw) = bar.to_query();
        assert_eq!(
            filters,
            vec![
                Filter {
                    column: "age".into(),
                    op: FilterOp::Gt,
                    value: "30".into()
                },
                Filter {
                    column: "email".into(),
                    op: FilterOp::IsNull,
                    value: String::new()
                },
            ]
        );
        assert_eq!(raw.as_deref(), Some("id % 2 = 0"));
        let off = FilterBar {
            raw: false,
            ..bar.clone()
        };
        assert_eq!(off.to_query().1, None);
    }
}
