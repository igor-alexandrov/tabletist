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
    /// The connection the keys act on.
    pub selected: Option<ConnectionId>,
    /// Groups the user folded, by title.
    pub folded: Vec<String>,
    /// Focus the search on the next frame.
    pub focus_search: bool,
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
    /// Select a saved connection in the picker (`None` clears it).
    SelectConnection {
        tab: ConnTabId,
        conn: Option<ConnectionId>,
    },
    /// Move the picker's selection by this many connections.
    MovePickerSelection {
        tab: ConnTabId,
        step: isize,
    },
    /// Fold or unfold a group of connections in the picker.
    FoldConnectionGroup {
        tab: ConnTabId,
        group: String,
    },
    /// Open the connection dialog for a new connection.
    NewConnection,
    EditConnection(ConnectionId),
    DuplicateConnection(ConnectionId),
    DeleteConnection(ConnectionId),
    CloseDialog,
    /// Hide the notice above the window's content.
    DismissNotice,
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
    /// Show this schema's objects in the sidebar (loading them if needed).
    ShowSchema {
        tab: ConnTabId,
        schema: String,
    },
    /// Fold or unfold a group of objects sharing a name prefix.
    ToggleGroup {
        tab: ConnTabId,
        schema: String,
        prefix: String,
    },
    /// Switch the sidebar between prefix groups and one flat list.
    ToggleFlatTree(ConnTabId),
    /// Hide or show the sidebar.
    ToggleSidebar(ConnTabId),
    /// Show timestamps in the grid to the microsecond, or to the second.
    ToggleFullPrecision(ConnTabId),
    /// Drop one applied filter (the raw WHERE counts last) and query again.
    DropFilter {
        tab: ConnTabId,
        object_tab: ObjectTabId,
        index: usize,
    },
    /// Put keyboard focus in the terminal look's WHERE line.
    FocusWhere(ConnTabId),
    /// Put keyboard focus in the picker's search.
    FocusPickerSearch(ConnTabId),
    /// Fold (or unfold) every JSON document in the row panel (`za`).
    FoldDocuments {
        tab: ConnTabId,
        object_tab: ObjectTabId,
    },
    /// Follow the foreign key of the selected cell's column (`gd`).
    FollowSelectedKey {
        tab: ConnTabId,
        object_tab: ObjectTabId,
    },
    /// Drop the active object tab's sort, back to the key order.
    ClearSort {
        tab: ConnTabId,
        object_tab: ObjectTabId,
    },
    /// Open the table a foreign key points at, filtered to the row it names.
    FollowForeignKey {
        tab: ConnTabId,
        object: ObjectRef,
        column: String,
        value: String,
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
    pub environment: crate::connections::Environment,
    pub spec: ConnectSpec,
    pub driver: Driver,
    /// Whether the session runs over TLS, as negotiated. Set when the
    /// session connects; `prefer` may have fallen back to plain text.
    pub encrypted: bool,
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
    /// Objects opened lately, newest first.
    pub recent: Vec<(ObjectRef, ObjectKind)>,
    pub sidebar_hidden: bool,
    /// The grid shows timestamps in full rather than to the second.
    pub full_precision: bool,
    /// Focus the WHERE line on the next frame.
    pub focus_where: bool,
    /// Fold or unfold the row panel's documents on the next frame (`za`).
    pub fold_documents: Option<ObjectTabId>,
}

/// How many objects the sidebar's Recent section keeps.
pub const RECENT: usize = 5;

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
    pub environment: crate::connections::Environment,
    /// Put the keyboard in Name on the next frame (a new connection).
    pub focus_name: bool,
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
    /// The connection as saved, when editing one. Saved secrets belong to
    /// its servers and are never sent to another one.
    pub saved_spec: Option<ConnectSpec>,
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
    /// What a running Test connects to, fixed when it started.
    pub test_spec: Option<ConnectSpec>,
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
            environment: crate::connections::Environment::None,
            focus_name: true,
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
            saved_spec: None,
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
            test_spec: None,
            test_secrets: tabletist_db::Secrets::default(),
            test_waiting: 0,
        }
    }
}

impl ConnectionForm {
    /// Whether the password crosses a network with nothing to stop someone
    /// on the way reading it: a remote host, no SSH tunnel, and TLS that is
    /// off or does not check the server's certificate (PostgreSQL's
    /// `require` with a CA file does, like libpq).
    pub fn password_can_be_intercepted(&self) -> bool {
        let checked_by_ca = self.driver == Driver::Postgres
            && self.tls == TlsMode::Require
            && !self.ca_file.trim().is_empty();
        self.driver != Driver::Sqlite
            && !self.ssh
            && matches!(
                self.tls,
                TlsMode::Disable | TlsMode::Prefer | TlsMode::Require
            )
            && !checked_by_ca
            && !is_local_host(&self.host)
    }

    pub fn from_saved(saved: &crate::connections::SavedConnection) -> Self {
        let spec = &saved.spec;
        Self {
            editing: Some(saved.id.clone()),
            name: saved.name.clone(),
            color: saved.color,
            environment: saved.environment(),
            focus_name: false,
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
            saved_spec: Some(spec.clone()),
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
                // Without the host check, the system roots vouch for any
                // public certificate, so verify-ca needs its own CA.
                if self.tls == TlsMode::VerifyCa && ca_file.is_empty() {
                    return Err(
                        "Verify certificate needs a CA file. Choose Verify certificate \
                                and host to use the system certificates."
                            .into(),
                    );
                }
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

    /// Whether the fields still name the database server and login the
    /// connection was saved with.
    fn same_server(&self) -> bool {
        self.saved_spec.as_ref().is_some_and(|saved| {
            saved.driver == self.driver
                && saved.host == self.host.trim()
                && self.port.trim().parse() == Ok(saved.port)
                && saved.user == self.user.trim()
        })
    }

    /// Whether the fields still name the SSH server and login the
    /// connection was saved with.
    fn same_ssh_server(&self) -> bool {
        self.saved_spec
            .as_ref()
            .and_then(|saved| saved.ssh.as_ref())
            .is_some_and(|saved| {
                saved.host == self.ssh_host.trim()
                    && self.ssh_port.trim().parse() == Ok(saved.port)
                    && saved.user == self.ssh_user.trim()
            })
    }

    /// Whether the keyring's database password may be used: one is saved and
    /// the driver, host, port and user are still the ones it was saved for.
    pub fn password_is_saved(&self) -> bool {
        self.has_saved_password && self.same_server()
    }

    /// Whether a saved database password no longer fits the fields (the
    /// server or user changed): it is dropped on save and asked for instead.
    pub fn password_is_stale(&self) -> bool {
        self.has_saved_password && !self.same_server()
    }

    /// Whether a saved SSH secret belongs to the login method now chosen. A
    /// passphrase saved for a key is never used as a password.
    fn ssh_secret_fits_method(&self) -> bool {
        self.has_saved_ssh_secret && self.ssh && self.saved_ssh_auth == Some(self.ssh_auth)
    }

    /// Whether the keyring holds the SSH secret for the login method now
    /// chosen, on the SSH host, port and user it was saved for.
    pub fn ssh_secret_is_saved(&self) -> bool {
        self.ssh_secret_fits_method() && self.same_ssh_server()
    }

    /// Whether a saved SSH secret no longer fits the SSH host, port or user:
    /// it is dropped on save and asked for instead.
    pub fn ssh_secret_is_stale(&self) -> bool {
        self.ssh_secret_fits_method() && !self.same_ssh_server()
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
                && !self.ssh_secret_fits_method() =>
            {
                PasswordMode::None
            }
            _ => self.ssh_secret_mode,
        };
        Ok(crate::connections::SavedConnection {
            id: self.editing.clone().unwrap_or_else(ConnectionId::new),
            name: name.to_owned(),
            color: self.color,
            environment: Some(self.environment),
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
/// Loopback names and addresses, and Unix socket directories (PostgreSQL
/// reads a host starting with `/` as one). An empty host is not checked yet.
pub fn is_local_host(host: &str) -> bool {
    let host = host.trim();
    let bare = host
        .strip_prefix('[')
        .and_then(|host| host.strip_suffix(']'))
        .unwrap_or(host);
    host.is_empty()
        || host.starts_with('/')
        || host.eq_ignore_ascii_case("localhost")
        || bare
            .parse::<std::net::IpAddr>()
            .is_ok_and(|address| address.is_loopback())
}

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
    /// The schema the sidebar shows, once the user picked one.
    pub schema: Option<String>,
    /// List objects flat rather than in prefix groups.
    pub flat: bool,
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
    /// Its objects were asked for (the sidebar showed it).
    pub expanded: bool,
    pub objects: Fetch<Vec<ObjectInfo>>,
    /// Prefix groups the user unfolded (`book_`).
    pub open_groups: HashSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TreeNode {
    /// "No tables or views" in a loaded schema that has none.
    Empty(String),
    /// Objects of a schema sharing a name prefix: the schema, the prefix
    /// without its underscore (`book`).
    Group(String, String),
    Object(ObjectRef, ObjectKind),
}

/// One visible line of the tree.
#[derive(Debug, Clone, PartialEq)]
pub struct TreeRow {
    pub node: TreeNode,
    pub depth: u8,
    /// What the row shows: a group's prefix (`book_`), an object's name
    /// without its group's prefix.
    pub label: String,
    /// Objects in a group.
    pub count: Option<usize>,
    /// `Some` for rows that fold.
    pub expanded: Option<bool>,
    pub loading: bool,
    pub error: Option<String>,
}

/// Objects sorted into prefix groups: `book_authors` and `book_reviews`
/// share the group `book_`, and `orders` joins `orders_archive` in `orders`.
/// A prefix with one object stays a plain row.
#[derive(Debug, Clone, PartialEq)]
pub enum Entry<'a> {
    Object(&'a ObjectInfo),
    Group {
        /// The shared prefix without its underscore.
        prefix: String,
        /// `book_`, or `orders` when the bare prefix is an object too.
        label: String,
        objects: Vec<&'a ObjectInfo>,
    },
}

/// The part of `name` before its first underscore (all of it if none).
fn prefix_of(name: &str) -> &str {
    match name.find('_') {
        Some(at) if at > 0 => &name[..at],
        _ => name,
    }
}

/// What an object in the group `prefix` is called there: its name without
/// the prefix and underscore, or in full when it is the prefix itself.
pub fn short_name<'a>(name: &'a str, prefix: &str) -> &'a str {
    name.strip_prefix(prefix)
        .and_then(|rest| rest.strip_prefix('_'))
        .filter(|rest| !rest.is_empty())
        .unwrap_or(name)
}

/// `objects` in prefix groups, by name.
pub fn group_objects<'a>(objects: &[&'a ObjectInfo]) -> Vec<Entry<'a>> {
    let mut by_prefix: std::collections::BTreeMap<&str, Vec<&'a ObjectInfo>> =
        std::collections::BTreeMap::new();
    for object in objects {
        by_prefix
            .entry(prefix_of(&object.name))
            .or_default()
            .push(object);
    }
    let mut entries: Vec<(String, Entry<'a>)> = by_prefix
        .into_iter()
        .flat_map(|(prefix, mut members)| {
            members.sort_by(|a, b| a.name.cmp(&b.name));
            if members.len() < 2 {
                return members
                    .into_iter()
                    .map(|object| (object.name.clone(), Entry::Object(object)))
                    .collect::<Vec<_>>();
            }
            let bare = members.iter().any(|object| object.name == prefix);
            let label = if bare {
                prefix.to_owned()
            } else {
                format!("{prefix}_")
            };
            vec![(
                label.clone(),
                Entry::Group {
                    prefix: prefix.to_owned(),
                    label,
                    objects: members,
                },
            )]
        })
        .collect();
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    entries.into_iter().map(|(_, entry)| entry).collect()
}

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
    /// The schemas the sidebar offers, system ones only when asked for.
    pub fn visible_schemas(&self, driver: Driver, show_system: bool) -> Vec<String> {
        self.schemas
            .value
            .iter()
            .flatten()
            .filter(|schema| show_system || !is_system_schema(driver, schema))
            .cloned()
            .collect()
    }

    /// The schema whose objects the sidebar lists: the one picked, else the
    /// first one opened, else the first one.
    pub fn shown_schema(&self, driver: Driver, show_system: bool) -> Option<String> {
        let schemas = self.visible_schemas(driver, show_system);
        self.schema
            .as_ref()
            .filter(|schema| schemas.contains(schema))
            .cloned()
            .or_else(|| {
                schemas
                    .iter()
                    .find(|schema| self.nodes.get(*schema).is_some_and(|node| node.expanded))
                    .cloned()
            })
            .or_else(|| schemas.first().cloned())
    }

    /// The lines to draw: the shown schema's objects, in prefix groups or
    /// flat. A filter shows every match, unfolded.
    pub fn visible_rows(&self, driver: Driver, show_system: bool) -> Vec<TreeRow> {
        let needle = self.filter.trim().to_lowercase();
        let filtering = !needle.is_empty();
        let mut rows = Vec::new();
        let Some(schema) = self.shown_schema(driver, show_system) else {
            return rows;
        };
        let Some(node) = self.nodes.get(&schema) else {
            return rows;
        };
        let Some(objects) = node.objects.value.as_ref() else {
            return rows;
        };
        if objects.is_empty() {
            rows.push(TreeRow {
                node: TreeNode::Empty(schema.clone()),
                depth: 0,
                label: "No tables or views".into(),
                count: None,
                expanded: None,
                loading: false,
                error: None,
            });
            return rows;
        }
        let matching: Vec<&ObjectInfo> = objects
            .iter()
            .filter(|object| !filtering || object.name.to_lowercase().contains(&needle))
            .collect();
        let object_row = |object: &ObjectInfo, depth: u8, label: &str| TreeRow {
            node: TreeNode::Object(
                ObjectRef::new(schema.clone(), object.name.clone()),
                object.kind,
            ),
            depth,
            label: label.to_owned(),
            count: None,
            expanded: None,
            loading: false,
            error: None,
        };
        if self.flat {
            let mut sorted = matching;
            sorted.sort_by(|a, b| a.name.cmp(&b.name));
            for object in sorted {
                rows.push(object_row(object, 0, &object.name));
            }
            return rows;
        }
        for entry in group_objects(&matching) {
            match entry {
                Entry::Object(object) => rows.push(object_row(object, 0, &object.name)),
                Entry::Group {
                    prefix,
                    label,
                    objects,
                } => {
                    let open = filtering || node.open_groups.contains(&prefix);
                    rows.push(TreeRow {
                        node: TreeNode::Group(schema.clone(), prefix.clone()),
                        depth: 0,
                        label,
                        count: Some(objects.len()),
                        expanded: Some(open),
                        loading: false,
                        error: None,
                    });
                    if open {
                        for object in objects {
                            rows.push(object_row(object, 1, short_name(&object.name, &prefix)));
                        }
                    }
                }
            }
        }
        rows
    }

    /// Unfolds the group `object` sits in, so the sidebar shows it.
    pub fn reveal(&mut self, object: &ObjectRef) {
        let prefix = prefix_of(&object.name).to_owned();
        if let Some(node) = self.nodes.get_mut(&object.schema) {
            node.open_groups.insert(prefix);
        }
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

    #[test]
    fn only_remote_hosts_without_a_tunnel_or_verification_expose_the_password() {
        for host in [
            "localhost",
            "LOCALHOST",
            " 127.0.0.1 ",
            "127.0.0.2",
            "::1",
            "[::1]",
            "/var/run/postgresql",
            "",
        ] {
            assert!(is_local_host(host), "{host:?}");
        }
        for host in ["db.example.com", "10.0.0.5", "::2", "localhost.example.com"] {
            assert!(!is_local_host(host), "{host:?}");
        }
        let mut form = ConnectionForm {
            driver: Driver::Postgres,
            host: "db.example.com".into(),
            tls: TlsMode::Prefer,
            ..ConnectionForm::default()
        };
        assert!(form.password_can_be_intercepted());
        form.tls = TlsMode::VerifyFull;
        assert!(!form.password_can_be_intercepted());
        form.tls = TlsMode::Require;
        form.ca_file = "/etc/ca.pem".into();
        assert!(!form.password_can_be_intercepted(), "verify-ca, like libpq");
        form.driver = Driver::MySql;
        assert!(form.password_can_be_intercepted(), "MySQL refuses it");
        form.driver = Driver::Postgres;
        form.ca_file.clear();
        assert!(form.password_can_be_intercepted());
        form.ssh = true;
        assert!(!form.password_can_be_intercepted());
        form.ssh = false;
        form.driver = Driver::Sqlite;
        assert!(!form.password_can_be_intercepted());
    }

    fn labels(rows: &[TreeRow]) -> Vec<String> {
        rows.iter()
            .map(|row| match &row.node {
                TreeNode::Group(..) => format!("{}({})", row.label, row.count.unwrap()),
                _ => format!("{}{}", "  ".repeat(usize::from(row.depth)), row.label),
            })
            .collect()
    }

    #[test]
    fn the_sidebar_lists_the_shown_schema_and_hides_system_ones() {
        let tree = tree();
        assert_eq!(
            tree.shown_schema(Driver::Postgres, false).as_deref(),
            Some("public"),
            "the opened one"
        );
        assert_eq!(
            labels(&tree.visible_rows(Driver::Postgres, false)),
            ["active_users", "orders", "users"]
        );
        assert_eq!(
            tree.visible_schemas(Driver::Postgres, false),
            ["public", "billing"]
        );
        assert!(
            tree.visible_schemas(Driver::Postgres, true)
                .contains(&"pg_catalog".to_owned())
        );
    }

    fn grouped() -> Tree {
        let mut tree = Tree::default();
        tree.schemas.value = Some(vec!["public".into()]);
        let mut public = SchemaNode::default();
        public.objects.value = Some(vec![
            info("orders_archive", ObjectKind::Table),
            info("book_reviews", ObjectKind::Table),
            info("books", ObjectKind::Table),
            info("orders", ObjectKind::Table),
            info("book_authors", ObjectKind::View),
        ]);
        tree.nodes.insert("public".into(), public);
        tree
    }

    #[test]
    fn objects_sharing_a_prefix_fold_into_a_group() {
        let mut tree = grouped();
        assert_eq!(
            labels(&tree.visible_rows(Driver::Postgres, false)),
            ["book_(2)", "books", "orders(2)"]
        );
        tree.reveal(&ObjectRef::new("public", "book_reviews"));
        tree.nodes
            .get_mut("public")
            .unwrap()
            .open_groups
            .insert("orders".into());
        assert_eq!(
            labels(&tree.visible_rows(Driver::Postgres, false)),
            [
                "book_(2)",
                "  authors",
                "  reviews",
                "books",
                "orders(2)",
                "  orders",
                "  archive"
            ]
        );
    }

    #[test]
    fn the_flat_list_names_every_object_in_full() {
        let mut tree = grouped();
        tree.flat = true;
        assert_eq!(
            labels(&tree.visible_rows(Driver::Postgres, false)),
            [
                "book_authors",
                "book_reviews",
                "books",
                "orders",
                "orders_archive"
            ]
        );
    }

    #[test]
    fn a_filter_shows_matches_unfolded() {
        let mut tree = grouped();
        tree.filter = "REV".into();
        assert_eq!(
            labels(&tree.visible_rows(Driver::Postgres, false)),
            ["book_reviews"],
            "one match is no group"
        );
        tree.filter = "book_".into();
        assert_eq!(
            labels(&tree.visible_rows(Driver::Postgres, false)),
            ["book_(2)", "  authors", "  reviews"]
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
    fn verify_ca_needs_a_ca_file() {
        let mut form = ConnectionForm {
            ssh: false,
            tls: TlsMode::VerifyCa,
            ..ssh_form()
        };
        let error = form.to_spec().unwrap_err();
        assert!(error.contains("needs a CA file"), "{error}");
        form.ca_file = "/etc/ssl/db-ca.pem".into();
        assert_eq!(
            form.to_spec().unwrap().ca_file,
            Some("/etc/ssl/db-ca.pem".into())
        );
        form.ca_file.clear();
        form.tls = TlsMode::VerifyFull;
        assert_eq!(form.to_spec().unwrap().ca_file, None);
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
