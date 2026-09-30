# Tabletist: design spec

Date: 2026-09-27
Status: implemented. Batches 0 to 8 (through polish and release) are done; see
`docs/superpowers/plans/`.

## 1. Purpose

Tabletist is a small, fast, native desktop database client written in
Rust. It must feel at home on
Omarchy (Arch + Hyprland, Wayland) and also run on macOS and Windows.

The first milestone is **browsing**: connect to a database, explore
its objects, page through a table's rows, inspect one row in a side panel, and
see a table's structure.

### Supported databases

PostgreSQL, MySQL, SQLite. No others in v1.

### Success criteria for v1

1. A user can save a connection (direct, TLS, or through an SSH tunnel), and
   open it in a connection tab. Several connection tabs can be open at once.
2. The sidebar lists schemas and their tables, views, and materialized views.
3. Opening a table shows its rows in a virtualized grid, paged 300 at a time,
   sortable by column (server side), with an estimated total row count.
4. Selecting a row shows every field of that row, in full, in the row panel.
5. The Structure view lists columns, indexes, and foreign keys.
6. No action in the app can modify data in the connected database.
7. The UI thread never blocks on the database, network, or disk; any running
   query can be cancelled.
8. The app follows the Omarchy theme live, and the OS light/dark setting
   elsewhere.

### Out of scope for v1

Editing cells, inserting or deleting rows, a SQL editor, query history,
exporting/importing data, databases other than the three above, functions,
sequences, triggers and other object kinds in the tree, restoring open tabs
across restarts, self-update, single-instance handling.

## 2. Reference projects and what we take from them

- **[crmne/spotifast](https://github.com/crmne/spotifast)**: the architecture.
  Unidirectional `Action` queue applied after drawing; a backend tokio runtime
  on its own OS thread talking to the UI over channels and waking it with a
  repaint waker; generation tokens to drop stale results; `Loadable<T>`;
  `virtual_rows` virtualized rows; keyboard handling in one `keys.rs`; JSON
  settings with atomic writes in XDG/platform dirs via `directories`; secrets
  in the OS keyring via `keyring-core` on a dedicated thread; demo mode for
  deterministic headless UI tests and screenshots; the conventions in its
  `AGENTS.md`/`CONTRIBUTING.md`.
- **[crmne/fastframe](https://github.com/crmne/fastframe)**: foundation crates
  for egui/eframe 0.36 apps. We use `fastframe-text`, `-fonts`, `-icons`,
  `-theme` (including Omarchy theme following), `-i18n`, `-log`, all pinned to
  one tag (currently `v0.1.7`) and moved together. We do not use `-tray`,
  `-shell`, `-update`. fastframe ships **no widgets**: the tree, grid, tab
  bars, and dialogs are built in Tabletist, on egui.

## 3. Architecture

### 3.1 Stack

- UI: `eframe`/`egui` 0.36, `glow` renderer, `wayland` + `x11` features,
  `accesskit`, `persistence`. The egui and winit forks spotifast patches in
  (`crmne/egui` apps-0.36, `crmne/winit` apps-0.30) are used through
  `[patch.crates-io]`, pinned to the same revisions spotifast uses.
- Async: `tokio` multi-thread runtime on a dedicated `tabletist-backend` thread.
- Drivers: `tokio-postgres`, `mysql_async`, `rusqlite` (bundled SQLite).
- TLS: `rustls` everywhere (`tokio-postgres-rustls`, `mysql_async` rustls
  feature). No OpenSSL.
- SSH: `russh` (pure Rust, async; password, key, and agent auth).
- Errors: `thiserror` in libraries, `anyhow` only at the binary edge.
- Logging: `log` + `fastframe-log`.
- Persistence: `serde` + `serde_json`, `directories`, `keyring-core` with the
  native store per target.
- File picker: `rfd`.

### 3.2 Workspace layout

```
tabletist/
  Cargo.toml                 workspace + the app package
  rust-toolchain.toml        1.98.0
  mise.toml                  rust, mbx
  compose.yaml               postgres 17, mysql 8.4, openssh-server (tests)
  AGENTS.md                  agent rules (adapted from spotifast)
  build.rs                   compiles the gettext catalogs in assets/i18n
  compose/                   MySQL init script, SSH test server Dockerfile
  crates/tabletist-db/       no UI dependencies
    src/lib.rs               Connection (driver + optional SSH tunnel), CancelHandle
    src/spec.rs              ConnectSpec, Driver, TlsMode, SshSpec, Secrets, URL parsing
    src/value.rs             Value, ColumnMeta, ValueKind
    src/catalog.rs           ObjectRef, ObjectInfo, ObjectKind, Structure
    src/query.rs             RowQuery, Filter, Sort, RowPage
    src/dialect.rs           per-dialect SQL builder and identifier quoting
    src/error.rs             Error, SshStage
    src/tls.rs               rustls config per TlsMode (libpq sslmode meanings)
    src/ssh.rs               russh tunnel, host key check
    src/pg.rs  src/mysql.rs  src/sqlite.rs   adapters
    src/fixtures.rs          fixture scripts; writes the SQLite demo database
    fixtures/                postgres.sql, mysql.sql, sqlite.sql
    tests/                   integration tests per driver and SSH; ssh/ test keys
  src/                       the app: lib + bin
    main.rs                  calls entrypoint::run
    lib.rs                   module list
    entrypoint.rs            CLI (clap), demo flags, logging, native window
    app.rs                   App state, apply(Action) reducer
    model.rs                 Action, ConnTab, Workspace, Tree, ObjectTab, Fetch, Dialog
    backend.rs               runtime thread, Command/Event, sessions
    connections.rs           saved connections store (JSON)
    secrets.rs               OS keyring access on a dedicated thread
    known_hosts.rs           SSH host key trust store
    paths.rs                 config and state directories
    settings.rs              settings.json
    theme.rs                 palette, look, typography, icons, egui style
    theme/desktop_font.rs    the desktop's monospace font on Linux (fontconfig)
    i18n.rs                  bundled gettext catalogs
    util.rs                  atomic JSON files, fuzzy matching
    macos.rs                 unified title bar with the tabs (macOS only)
    testing.rs               headless UI test harness (AccessKit tree + events)
    shots.rs                 screenshots for visual review (`shots` feature)
    ui/mod.rs                panel layout, dialogs
    ui/conn_tabs.rs          connection tab bar
    ui/picker.rs             saved-connection picker
    ui/connect_dialog.rs     new or edit connection, SSH section
    ui/password_prompt.rs    asks for a password or passphrase
    ui/host_key_prompt.rs    trust an unknown SSH host key
    ui/workspace.rs          a connected tab: top bar, disconnected banner, body
    ui/sidebar.rs            filter, refresh, tree of schemas and objects
    ui/object_tabs.rs        object tab bar (preview tabs in italics)
    ui/data_view.rs          footer and grid, or the error or empty state
    ui/grid.rs               virtualized data grid
    ui/structure.rs          columns, indexes, foreign keys
    ui/row_panel.rs          every field of the selected row
    ui/filter_bar.rs         filter rows and raw WHERE
    ui/quick_open.rs         Cmd/Ctrl+P
    ui/help.rs               keyboard shortcuts dialog
    ui/format.rs             values as text for grid, row panel, clipboard
    ui/keys.rs               keyboard shortcuts
    ui/widgets.rs            shared widgets (virtual_rows, tabs, modal, fields)
  tests/cli.rs               runs the built binary
  assets/                    app icon, i18n/ (catalogs; English only for now),
                             icons/ (empty: icons come from fastframe-icons)
  packaging/                 arch/ (AUR PKGBUILD templates), linux/ (desktop
                             file, icon), macos/ (bundle, DMG, signing),
                             windows/ (Inno Setup installer, icon)
  contrib/omarchy/           Omarchy theme template for Tabletist's palette
```

App-id: `dev.tabletist.Tabletist`. Binary: `tabletist`.

### 3.3 Data flow

```
 ui/* draw  ──push──▶ app.actions: Vec<Action>
                           │ drained after drawing
                           ▼
                    App::apply(Action) ──▶ mutates state
                           │ may send
                           ▼
                    backend.send(Command { session, request, .. })
                           │ tokio mpsc (unbounded)
                           ▼
        backend thread: Worker owns HashMap<SessionId, SessionHandle>
                           │ queues the command on its session's task
                           ▼
                    std mpsc Event ──▶ waker.wake() (request_repaint)
                           │
 next frame: backend.poll() drains events ──▶ App::apply(Action::Backend(event))
```

Rules:

- Views never mutate `App` directly; they push `Action`s.
- Every command carries its `SessionId` and a fresh `RequestId`; the slot that
  waits for the answer (a `Fetch<T>`) records that id as pending. The reducer
  drops events whose session or tab is gone or that do not answer the slot's
  pending request.
- One session per connection tab: the driver connection plus an optional SSH
  tunnel. Each session runs its commands in order on its own task (queries on
  one connection cannot run concurrently); a separate `CancelHandle` works
  while a query runs. A cancel names its request: it stops that request if
  it is running and skips it if it is still queued, and the session lets a
  cancel land before it starts the next command. A load that replaces a
  pending one (refresh, sort, paging, filters) and closing an object tab
  cancel what nothing waits for any more.
- The UI is idle when nothing happens: repaints only on input, on backend
  events, or on a scheduled spinner tick while something is loading.

## 4. Database layer (`tabletist-db`)

### 4.1 API

```rust
pub struct Connection { inner: Inner, tunnel: Option<ssh::Tunnel> }
enum Inner { Sqlite(sqlite::Conn), Postgres(Box<pg::Conn>), MySql(Box<mysql::Conn>) }

impl Connection {
    pub async fn connect(spec: &ConnectSpec, secrets: &Secrets) -> Result<Connection>;
    pub async fn connect_with(spec: &ConnectSpec, secrets: &Secrets, host_keys: &HostKeys)
        -> Result<Connection>;
    pub fn driver(&self) -> Driver;
    pub fn dialect(&self) -> Dialect;
    pub async fn list_databases(&self) -> Result<Vec<String>>;
    pub async fn list_schemas(&self) -> Result<Vec<String>>;
    pub async fn list_objects(&self, schema: &str) -> Result<Vec<ObjectInfo>>;
    pub async fn describe(&self, obj: &ObjectRef) -> Result<Structure>;
    pub async fn fetch_rows(&self, q: &RowQuery) -> Result<RowPage>;
    pub async fn count_rows(&self, q: &RowQuery) -> Result<u64>;
    pub fn cancel_handle(&self) -> CancelHandle;
    pub async fn close(self) -> Result<()>;
}
```

A closed enum, not a trait object: there are exactly three drivers, and the
enum avoids boxed async trait methods. The public `Connection` wraps it with
the SSH tunnel, declared after the driver so the driver closes first.

### 4.2 Values

```rust
pub enum Value { Null, Bool(bool), Int(i64), Float(f64), Text(Box<str>), Bytes(Box<[u8]>) }
pub struct ColumnMeta { pub name: String, pub type_name: String, pub kind: ValueKind }
pub enum ValueKind { Numeric, Text, Json, Temporal, Binary, Bool, Other }
```

- PostgreSQL uses the simple-query (text) protocol for row fetches, so every
  type, including enums, arrays, ranges, and extension types, arrives as text.
  `bytea` is decoded from `\x..` hex into `Bytes`. `ValueKind` comes from the
  column's type OID.
  The SQL is `prepare`d first (to learn column types and refuse a second
  statement), then run with the simple-query protocol inside a read-only
  transaction.
- MySQL maps `mysql_async::Value`; `ValueKind` comes from the column type.
- SQLite maps `ValueRef`; `ValueKind` comes from the declared type affinity,
  falling back to the storage class of the value.
- Integers wider than `i64` (e.g. MySQL `BIGINT UNSIGNED`, `NUMERIC`) and
  decimals are kept as `Text` with `ValueKind::Numeric`, so nothing is rounded.

### 4.3 Read-only sessions

Every session is opened read-only, so nothing in the app, including a raw
WHERE clause, can modify data:

- PostgreSQL: `SET default_transaction_read_only = on` right after connect.
- MySQL: `SET SESSION TRANSACTION READ ONLY` right after connect.
- SQLite: `SQLITE_OPEN_READ_ONLY`.

### 4.4 Catalog

- `ObjectKind`: `Table`, `View`, `MaterializedView` (PostgreSQL only).
- `ObjectInfo { name, kind, estimated_rows: Option<u64> }`, estimates from
  `pg_class.reltuples`, `information_schema.TABLES.TABLE_ROWS`, or none for
  SQLite.
- `Structure { columns: Vec<ColumnInfo>, primary_key: Vec<String>,
  indexes: Vec<IndexInfo>, foreign_keys: Vec<ForeignKeyInfo> }` where
  `ColumnInfo { name, type_name, nullable, default, comment }`,
  `IndexInfo { name, columns, unique, primary, method }`,
  `ForeignKeyInfo { name: Option<String>, columns, ref_schema, ref_table,
  ref_columns, on_update, on_delete }`.
- Sources: `pg_catalog` (PostgreSQL), `information_schema` (MySQL),
  `PRAGMA table_xinfo`, `index_list`, `index_info`, `foreign_key_list`
  (SQLite).
- Hierarchy per driver:
  - PostgreSQL: one database per session (switching database reconnects the
    session), then schemas, then objects. System schemas (`pg_catalog`,
    `information_schema`, `pg_toast*`) are hidden by default.
  - MySQL: databases are presented as schemas; `mysql`, `sys`,
    `performance_schema`, `information_schema` hidden by default.
  - SQLite: `main` plus attached databases.

### 4.5 Row queries

```rust
pub struct RowQuery {
    pub object: ObjectRef,
    pub filters: Vec<Filter>,          // AND-combined
    pub raw_where: Option<String>,     // escape hatch, appended with AND
    pub sort: Vec<Sort>,               // v1 UI sets at most one
    pub offset: u64,
    pub limit: u32,                    // default 300
}
pub enum FilterOp { Eq, Ne, Lt, Gt, Le, Ge, Contains, StartsWith, IsNull, IsNotNull, In }
pub struct RowPage { pub columns: Vec<ColumnMeta>, pub rows: Vec<Vec<Value>>,
                     pub has_more: bool, pub ordered_by_key: bool, pub elapsed: Duration }
```

- `dialect.rs` builds `SELECT * FROM <quoted> WHERE .. ORDER BY .. LIMIT ..
  OFFSET ..` per dialect. Identifiers are always quoted (`"x"` or `` `x` ``,
  with embedded quotes doubled). Filter values are bound parameters for SQLite and MySQL, and quoted literals (with `'` doubled, under `standard_conforming_strings = on`) for PostgreSQL, whose rows are read through the simple-query protocol.
  `Contains`/`StartsWith` escape `%` and `_`.
- With no user sort, rows are ordered by the primary key when there is one, so
  paging is stable. Without a primary key there is no ORDER BY and the footer
  says the order is unstable.
- `count_rows` runs `SELECT count(*)` with the same filters. It is only run on
  request and is cancellable.

### 4.6 Connecting, TLS, SSH

```rust
pub struct ConnectSpec {
    pub driver: Driver,
    pub host: String, pub port: u16, pub user: String, pub database: String,
    pub sqlite_path: Option<PathBuf>,
    pub tls: TlsMode,                   // Disable, Prefer, Require, VerifyCa, VerifyFull
    pub ca_file: Option<PathBuf>,
    pub ssh: Option<SshSpec>,
}
pub struct SshSpec { pub host: String, pub port: u16, pub user: String, pub auth: SshAuth }
pub enum SshAuth { Password, KeyFile { path: PathBuf }, Agent }
```

- Secrets (database password, SSH password, key passphrase) are passed
  separately in `Secrets`, never stored in `ConnectSpec`, and their `Debug`
  impl prints only `Secrets { .. }`.
- `ConnectSpec::from_url` parses `postgres://`, `postgresql://`, `mysql://`,
  `mariadb://`, `sqlite:` URLs including `sslmode` (and MySQL's `ssl-mode`).
- Connect timeout: 10 s.
- SSH: `ssh.rs` connects with `russh`, authenticates, binds a listener on
  `127.0.0.1:0`, and forwards every accepted connection through a
  `direct-tcpip` channel to the database host. The driver connects to the local
  port. The tunnel is owned by the `Session` and closed with it.
- SSH host keys: the first connection to a host reports the key's fingerprint
  as `Error::Ssh { stage: HostKeyUnknown { fingerprint } }`; the app asks the
  user to trust it, stores it in `known_hosts.json`, and retries. A changed key
  is `HostKeyMismatch { fingerprint }` and is refused, with no override in the
  dialog. The trusted keys reach the tunnel as `HostKeys` through
  `Connection::connect_with`.

### 4.7 Cancellation

- PostgreSQL: `tokio_postgres::CancelToken` (through the SSH tunnel when there
  is one).
- MySQL: a second short-lived connection sends `KILL QUERY <connection id>`.
- SQLite: `rusqlite::InterruptHandle`.

### 4.8 Errors

```rust
pub enum Error {
    Connect(String), Auth(String), Tls(String),
    Ssh { stage: SshStage, message: String },
    Query { code: Option<String>, message: String, detail: Option<String>, hint: Option<String> },
    Cancelled, Timeout, ConnectionLost(String), InvalidSpec(String),
    Unsupported(&'static str), Io(String),
}
```

Messages are shown to the user as-is. Passwords and secrets never appear in
errors or logs.

## 5. The app

### 5.1 State

```rust
App { connections: SavedConnections, tabs: Vec<ConnTab>, active: usize,
      dialog: Option<Dialog>, settings: Settings, backend: Backend, actions: Vec<Action>,
      host_keys: HostKeys, palette: Palette, look: Look, .. }

ConnTab { id: ConnTabId, content: ConnTabContent }
enum ConnTabContent { Picker(PickerState), Workspace(Box<Workspace>) }

Workspace { session: SessionId, conn_id: ConnectionId, name, color, spec: ConnectSpec,
            status: SessionStatus, tree: Tree, objects: Vec<ObjectTab>,
            active_object: Option<ObjectTabId>, row_panel: bool,
            databases: Fetch<Vec<String>>, pane: Pane /* Tree | Grid */, .. }

ObjectTab { id: ObjectTabId, object: ObjectRef, kind: ObjectKind, pinned: bool,
            view: ObjectView /* Data | Structure */, query: RowQuery,
            rows: Fetch<RowPage>, structure: Fetch<Structure>, selection: Option<CellPos>,
            estimated_rows: Option<u64>, count: Fetch<u64>, filter: FilterBar }

enum SessionStatus { Connecting { request }, Connected, Disconnected(Error), Cancelled }
enum Dialog { Connection(..), Password(..), HostKey(..), QuickOpen(..), Help }

// Loadable state is `Fetch<T> { value, pending: Option<RequestId>, error }`:
// only the pending request's result is accepted, which drops stale results.
```

### 5.2 Layout

```
┌ [● prod-db ×] [● staging ×] [○ local.sqlite ×] [+] ───────────────────────────┐  connection tabs
├ top bar: database ▾ │ via SSH host │ TLS verified │ disconnect ───────────────┤
├ sidebar ──────┬ [users] [orders*] [events] ─────────────┬ row panel ─────────┤  object tabs
│ filter  ⟳     │ filter bar: [col ▾][op ▾][value] + x ⏎   │ users #42          │
│ ▾ public      ├─────────────────────────────────────────│ id      int4    42 │
│   ▾ Tables    │ id │ email          │ created_at  │ ... │ email   text  b@.. │
│     users     │▶42 │ b@example.com  │ 2026-01-03  │     │ meta    jsonb      │
│   ▸ Views     │ ...virtualized rows...                  │  { "plan": "pro" } │
│ ▸ billing     ├─────────────────────────────────────────│ [copy] per field   │
│               │ [Data|Structure] 1-300 of ~1.2M < > Count  84 ms  stop     │
└───────────────┴─────────────────────────────────────────┴────────────────────┘
```

### 5.3 Connection tabs

- One tab, one connection (one `Session`). Several tabs can be open at once;
  the same saved connection can be opened in more than one tab.
- A tab shows the connection's color tag, name, and status (connecting,
  connected, disconnected).
- `+` or Cmd/Ctrl+T opens a new tab with the **picker**. The app starts with
  one picker tab.
- Closing a tab cancels its running query, closes the connection and the SSH
  tunnel, and removes the tab. Middle-click closes. The bar scrolls when it
  overflows.
- When a connection drops, the tab keeps its tree and object tabs and shows a
  "Disconnected: <reason>" banner with **Reconnect**.

### 5.4 Picker and connection dialog

- Picker: searchable list of saved connections showing color tag, driver icon,
  and `user@host/db` (or the file name for SQLite). Double-click or Enter
  connects in this tab. New, Edit, Duplicate, Delete.
- Connection dialog: driver switch; name; color tag (none, red, orange, yellow,
  green, blue, purple, gray); host, port, user, password, database, or SQLite
  file picker; TLS mode, with a CA file for the verifying modes; collapsible
  SSH section (host, port, user, auth method, password or key file +
  passphrase);
  "Paste URL" to fill fields; **Test**; **Save**; **Save & Connect**.
- Password storage per secret: "Save in keyring" (default) or "Ask every
  time". Secrets live only in the keyring, keyed by connection id.

### 5.5 Sidebar tree

- Custom tree widget: lazy children with a per-node spinner, keyboard
  access (arrows, Home/End, Enter move and open when the tree was the last
  pane used), filter field narrows by name (case-insensitive
  substring over loaded nodes). A refresh button sits beside the filter.
- Nodes: schema, then groups (Tables, Views, Materialized views) with counts,
  then objects.
- Single click opens a **preview tab** that the next single click replaces
  (shown in italics). Double click, or any interaction inside the tab
  (sorting, paging, selecting), pins it.

### 5.6 Data view and grid

- Grid (`ui/grid.rs`) virtualizes rows with spotifast's `virtual_rows` pattern.
  Sticky header, horizontal scroll, resizable columns with initial widths
  sampled from the first page (capped), click header to cycle sort
  ascending, descending, off (re-queries the server).
- Cells render one line, truncated at 256 characters; NULL is drawn dimmed;
  numeric columns are right-aligned with tabular figures.
- Selection: one cell, which also selects its row. Arrows, Page Up/Down,
  Home/End move it. Cmd/Ctrl+C copies the cell, Shift+Cmd/Ctrl+C copies the
  row as TSV.
- Footer: Data/Structure switch, row range, estimated (`~`) or exact total,
  previous/next page, Count, elapsed time, and a stop button while a query
  runs. Each page fetches one extra row; "Next" is enabled when that row
  exists (`RowPage::has_more`).
- Errors show inline in the tab (code, message, detail, hint) with Retry.

### 5.7 Row panel

- Resizable right panel, toggled with Space (when the grid has focus) or
  Cmd/Ctrl+Shift+R. Follows the grid selection.
- One entry per field: name, type, and the full value as selectable read-only
  text. JSON up to 256 KiB is a highlighted tree in monospace: keys keep
  their order, objects and arrays fold (all open up to 40 lines, else only
  the top level), with Expand all / Collapse all. Larger or invalid JSON is
  plain text. Binary shows its size and a hex
  preview of the first 4 KiB. Text longer than 20 lines or 4,000 characters
  is collapsed with "Show all". NULL shows a NULL badge.
- The reducer formats the selected row's text once, when the selection or
  the page changes; drawing only lays that text out.
- A copy button per field and a filter field for wide tables.

### 5.8 Structure view

Three sections (Columns, Indexes, Foreign keys), each rendered as a plain
read-only table (structure data is small; the data grid is not needed).

### 5.9 Filter bar and quick open

- Filter bar (Cmd/Ctrl+F): rows of column, operator, value; add/remove rows;
  a "Raw WHERE" toggle; Enter applies and resets to the first page.
- Quick open (Cmd/Ctrl+P): fuzzy search over the loaded objects of the active
  connection; Enter opens and pins.

### 5.10 Keyboard

| Shortcut | Action |
|---|---|
| Cmd/Ctrl+T | New connection tab (picker) |
| Cmd/Ctrl+Shift+W | Close connection tab |
| Cmd/Ctrl+1..9, Ctrl+Tab, Ctrl+Shift+Tab | Switch connection tab |
| Cmd/Ctrl+N | New connection |
| Cmd/Ctrl+W | Close object tab |
| Cmd/Ctrl+Shift+[ / ] | Previous / next object tab |
| Cmd/Ctrl+R | Refresh current tab (or tree when it has focus) |
| Cmd/Ctrl+F | Filter bar |
| Cmd/Ctrl+P | Quick open |
| Cmd/Ctrl+Alt+Left / Right | Previous / next page |
| Cmd/Ctrl+. | Cancel running query |
| Space, Cmd/Ctrl+Shift+R | Toggle row panel |
| Cmd/Ctrl+C, Cmd/Ctrl+Shift+C | Copy cell / copy row |
| Arrows, Home/End, Enter | Move in the tree |
| Arrows, Page Up/Down, Home/End | Move in the grid |
| ? | Shortcuts dialog |

All handled in `ui/keys.rs`. Plain keys (arrows, Space, `?`) and copy are
suppressed while a text field has focus; Cmd/Ctrl shortcuts are not.

### 5.11 Platform integration

- Theme: `fastframe-theme` presets, following the Omarchy theme live on
  Omarchy and the OS light/dark setting elsewhere; palette mapped onto
  `egui::Visuals` in `theme.rs`.
- Look: shape and density follow the platform (`theme::Look`: macOS, Omarchy
  on every Linux desktop, standard on Windows), independent of the palette.
  See `2026-09-28-platform-looks-design.md`. A palette file in `themes/`
  chosen in settings overrides the desktop's palette.
- Text: `fastframe-text` follows desktop hinting; `fastframe-fonts` Inter with
  tabular figures, plus a monospace face for JSON and hex. On Linux the
  monospace face is the desktop's when fontconfig resolves one
  (`theme/desktop_font.rs`), and the Omarchy look draws grid and row panel
  data in it.
- macOS: the connection tabs share a unified title bar with the window
  buttons (`macos.rs`).
- Wayland first; app-id `dev.tabletist.Tabletist` and a `.desktop` file so
  Hyprland window rules match.
- eframe persistence restores window geometry and panel widths.

### 5.12 Files

Via `directories::ProjectDirs` (`~/.config/tabletist`, `~/.local/state/tabletist`
on Linux; platform equivalents elsewhere):

- `config/settings.json`: page size, show system schemas, custom palette file.
- `config/connections.json`: saved connections (no secrets).
- `config/known_hosts.json`: trusted SSH host keys.
- `config/themes/`: palette files.
- `state/tabletist.log`, `state/panic.log`: log files (fastframe-log).

Connections and known hosts are saved on the backend runtime, never the UI
thread; a burst of saves to one file writes only the newest, and quitting
waits briefly for pending saves.
All JSON is written atomically (write `*.tmp`, then rename), versioned, and
loaded with `#[serde(default)]` so older files keep working. An unreadable file
falls back to defaults with a warning and is kept aside as `*.bad`.

## 6. Testing

- `tabletist-db` unit tests: dialect SQL builder (quoting, parameters, filters,
  sort, paging), URL parsing, value conversion, error mapping.
- `tabletist-db` integration tests over shared fixtures
  (`crates/tabletist-db/fixtures/{postgres,mysql,sqlite}.sql`) covering enums, arrays, JSON/JSONB,
  bytea/BLOB, NULLs, Unicode, a table without a primary key, a 100k-row table,
  views, materialized views, foreign keys, composite indexes. SQLite runs
  always. PostgreSQL, MySQL, and SSH run when `TABLETIST_TEST_PG_URL`,
  `TABLETIST_TEST_MYSQL_URL`, `TABLETIST_TEST_SSH_URL` are set; `compose.yaml`
  provides them locally and CI provides them as service containers on Linux.
- App reducer tests: apply `Action`s against a recording, offline backend and
  assert state and the commands sent.
- Headless UI tests (`src/testing.rs`, no window): lay out every screen at
  several window sizes; assert on the AccessKit tree (no overlaps, expected
  labels). Demo mode (`--demo`) and tests write the SQLite fixture to a
  throwaway file with `tabletist_db::fixtures::write_sqlite_demo`.
- Screenshots for visual review: `--demo --demo-shot PATH --demo-size WxH`
  saves the running window; `src/shots.rs` (`cargo test --features shots
  --lib shots -- --ignored`, needs a GPU) renders every scene under each
  look, light and dark, to `target/shots/`.
- `tests/cli.rs` runs the built binary.
- Tests never touch the network or the keyring, except the gated integration
  tests and one `#[ignore]` native keyring round trip.

## 7. Conventions

- Edition 2024, Rust 1.98 pinned in `rust-toolchain.toml`; `mise` + `mbx`.
- `unsafe_code = "forbid"`; CI runs `cargo fmt --check`,
  `cargo clippy --all-targets -- -D warnings`, and
  `RUSTDOCFLAGS=-D warnings cargo doc` on Linux, and `cargo test` on Linux,
  macOS, and Windows.
- Views emit Actions; the reducer applies them after drawing.
- The UI thread never waits on the database, network, or disk.
- Platform code sits behind `cfg`; all three targets must keep compiling.
- Every dependency gets a comment in `Cargo.toml` saying why it is there.
- Never log credentials.
- Work on `main`, linear history, one topic per commit, each passing checks.
- Release profile: thin LTO, `codegen-units = 1`, `strip`, `panic = "abort"`.

## 8. Packaging

- Linux first: AUR PKGBUILD, `.desktop` file, icon.
- macOS: universal `.app` in a DMG.
- Windows: installer.
- Release CI builds all three. No self-update in v1.

## 9. Batches

Each batch ends compiling, tested, and demonstrable. Plans live in
`docs/superpowers/plans/`, one per batch. Batches 0 to 3 are planned in detail
up front; batches 4 to 8 are planned just before each starts.

| # | Batch | Delivers | Depends on |
|---|---|---|---|
| 0 | Foundation | Workspace, toolchain, `mise`, CI; `AGENTS.md`; eframe window with fastframe fonts, text, theme (Omarchy), icons, log, i18n; paths and settings; connection tab bar with an empty picker tab; demo-mode skeleton | none |
| 1 | DB core + SQLite | `tabletist-db`: `Value`/`ColumnMeta`, `ConnectSpec` + URL parsing, `Error`, catalog types, dialect SQL builder, SQLite adapter (list, describe, fetch, count, cancel), fixtures and tests | 0 |
| 2 | Backend + connections | Backend runtime thread, sessions, Command/Event, waker, generations; `Action` reducer; saved connections store; picker; connection dialog (SQLite path); connect, close, reconnect in connection tabs | 1 |
| 3 | Browsing UI (MVP) | Sidebar tree, object tabs with preview/pin, grid, paging footer, server-side sort, row panel, Structure view. First usable app, on SQLite | 2 |
| 4 | PostgreSQL + TLS | PostgreSQL adapter, rustls TLS modes, database switcher, integration suite, dialog fields for host/port/user/password/TLS, keyring credentials ("save in keyring" / "ask every time") | 3 |
| 5 | MySQL + TLS | MySQL adapter, `KILL QUERY` cancel, integration suite | 3 (parallel with 4) |
| 6 | SSH tunnel | russh tunnel (password, key, agent), known_hosts trust, tunnel lifecycle, dialog SSH section, container test | 4 |
| 7 | Power browsing | Filter bar and raw WHERE, exact count, cancel everywhere, quick open, copy cell/row, full shortcut map and help dialog | 3 |
| 8 | Polish + release | Error and empty states, accessibility pass, Omarchy/Hyprland check, packaging (AUR, DMG, Windows installer), release CI | all |

## 10. Open risks

- **fastframe/egui fork churn**: fastframe has no API stability yet. Mitigation:
  pin one tag and one egui/winit fork revision; move them together.
- **Grid performance with wide tables**: hundreds of columns times 300 rows.
  Mitigation: virtualize columns as well as rows in `grid.rs` if profiling on
  the 100k-row, 200-column fixture shows frame times over 8 ms.
- **Large values**: a 50 MB text or blob cell is fetched in full. Accepted in
  v1; the grid truncates for display and the row panel collapses. Revisit with
  per-column length limits if it bites.
- **SSH agent on Windows**: `russh` agent support on Windows (Pageant/OpenSSH
  named pipe) needs verification in batch 6; key-file auth is the fallback.
