# Batch 2: Backend + Connections Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Connect to a saved SQLite database in a connection tab. This batch adds the backend runtime thread, sessions, the saved-connections store, the picker list, the connection dialog (SQLite path, Paste URL, Test), and connect/close/reconnect in connection tabs, with a top bar and a disconnected banner.

**Architecture:** Following spotifast: a `Backend` owns a dedicated OS thread running a tokio multi-thread runtime. The UI sends `Command`s over a tokio unbounded channel; the backend sends `Event`s back over a std channel and calls the `Waker` (egui repaint). Each session is a tokio task that owns its `tabletist_db::Connection` and runs its commands one at a time; a session's `CancelHandle` sits beside it so cancel works while a query runs. The app allocates `SessionId`s and `RequestId`s from its id counter; every result carries its `RequestId`, and the reducer drops results that are not the latest for their slot.

**Tech Stack:** tokio 1 (rt-multi-thread, sync, macros, time), rfd 0.17 (file dialog), uuid 1 (connection ids), plus Batch 0 and Batch 1.

**Spec:** `docs/superpowers/specs/2026-09-27-tabletist-design.md` (sections 3.3, 5.1, 5.3, 5.4, 5.12)

## Global Constraints

- Everything from Batch 0's Global Constraints still applies.
- The UI thread never awaits and never blocks on the database or disk beyond the small, synchronous `connections.json` save (the same as settings).
- Views push `Action`s; `App::apply` reduces them. Backend results enter the reducer as `Action::Backend(Event)`.
- Every `Command` that produces a result carries a `RequestId`. A result whose `RequestId` is not the one the app is waiting for in that slot is dropped silently.
- One `Session` per workspace connection tab. Closing the tab sends `Command::Close`, which cancels any running query and drops the connection.
- `connections.json` holds `ConnectSpec`s only, never secrets. Saved atomically via `util::save_json`; loaded via `util::load_json` (damaged file kept as `.bad`).
- SQLite is the only driver this batch connects to; the dialog shows the driver as SQLite (the driver switch arrives with PostgreSQL in batch 4).
- `rfd::AsyncFileDialog` is built on the UI thread and awaited on the backend runtime.
- Run `cargo fmt --all` before every commit; code in this plan is not pre-wrapped to rustfmt's width.
- Never use em dashes. One topic per commit on `main`.

## Review Focus

1. **A result arriving after its tab was closed or superseded** (connect finishes after the user closed the tab; a Test result arrives after the dialog closed): must be ignored, never panic on a missing tab, never resurrect a session. Test: Task 4 `a_connect_result_for_a_closed_tab_is_ignored` and `a_late_session_is_closed_not_adopted`.
2. **Deleting a saved connection that is open in a tab**: the open tab keeps working (it has its own spec copy) and the picker no longer lists it. Test: Task 5 `deleting_a_connection_leaves_open_tabs_alone`.
3. **Saving a connection with an empty name or no SQLite file**: the dialog refuses with an inline message instead of saving an unusable entry. Test: Task 5 `saving_requires_a_name_and_a_file`.
4. **A damaged `connections.json`**: the app starts with no saved connections and keeps the damaged file as `connections.json.bad`. Test: Task 2 `a_damaged_store_is_kept_aside`.
5. **Reconnecting after the database file was deleted**: the tab shows the connect error in its banner and stays usable (can retry or close). Test: Task 4 `a_failed_reconnect_keeps_the_tab_with_its_error`.

---

## File Structure

```
Cargo.toml                    add tokio, rfd, uuid
src/lib.rs                    add backend, connections modules
src/backend.rs                Waker, SessionId, RequestId, Command, Event, Backend, worker
src/connections.rs            ConnectionId, ColorTag, SavedConnection, SavedConnections store
src/model.rs                  + Workspace, SessionStatus, Dialog, ConnectionForm, new Actions
src/app.rs                    + backend, connections, dialogs; reducer for connect/close/reconnect/save
src/entrypoint.rs             create the Backend with a repaint waker
src/ui/mod.rs                 route Workspace tabs, draw dialogs
src/ui/conn_tabs.rs           titles and status dots for workspace tabs
src/ui/picker.rs              searchable saved-connection list with actions
src/ui/connect_dialog.rs      the connection dialog
src/ui/workspace.rs           top bar, disconnected banner, empty body (Batch 3 fills it)
src/ui/keys.rs                + Cmd/Ctrl+N
src/testing.rs                harness gets a recording backend
```

---

### Task 1: The backend runtime, sessions, commands and events

**Files:**
- Create: `src/backend.rs`
- Modify: `Cargo.toml`, `src/lib.rs`

**Interfaces:**
- Consumes: `tabletist_db::{Connection, ConnectSpec, Secrets, Error, ObjectRef, RowQuery, RowPage, ObjectInfo, Structure, CancelHandle, Driver}`.
- Produces:
  - `backend::Waker` (`Clone, Default`): `Waker::new(wake: impl Fn() + Send + Sync + 'static) -> Waker`, `wake(&self)`.
  - `backend::SessionId(pub u64)`, `backend::RequestId(pub u64)` (both `Debug, Clone, Copy, PartialEq, Eq, Hash`).
  - `backend::Command` (`Debug`): `Connect { session: SessionId, request: RequestId, spec: ConnectSpec, secrets: Secrets }`, `Test { request: RequestId, spec: ConnectSpec, secrets: Secrets }`, `Close { session: SessionId }`, `Cancel { session: SessionId }`, `ListSchemas { session, request }`, `ListObjects { session, request, schema: String }`, `Describe { session, request, object: ObjectRef }`, `FetchRows { session, request, query: RowQuery }`, `CountRows { session, request, query: RowQuery }`.
  - `backend::Event` (`Debug`): `Connected { session, request, driver: Driver }`, `ConnectFailed { session, request, error: Error }`, `Tested { request, result: Result<(), Error> }`, `Disconnected { session, error: Error }`, `Schemas { session, request, result: Result<Vec<String>, Error> }`, `Objects { session, request, schema: String, result: Result<Vec<ObjectInfo>, Error> }`, `Structure { session, request, result: Result<Structure, Error> }`, `Rows { session, request, result: Result<RowPage, Error> }`, `Count { session, request, result: Result<u64, Error> }`, `FilePicked { request, path: Option<PathBuf> }`.
  - `backend::Backend`: `Backend::start(waker: Waker) -> Backend`, `Backend::recording() -> Backend` (no thread; records commands for tests), `send(&mut self, command: Command)`, `poll(&mut self) -> Vec<Event>`, `pick_sqlite_file(&mut self, request: RequestId)`; test-only `sent: Vec<Command>` (pub, `#[cfg(test)]`), `inject(&self, event: Event)` (`#[cfg(test)]`), `wait(&mut self, timeout: Duration) -> Option<Event>` (`#[cfg(test)]`).

- [ ] **Step 1: Add dependencies and write failing backend tests**

Add to the app's `[dependencies]` in `Cargo.toml`:

```toml
# The backend runtime: one OS thread running tokio, where every database
# call happens (src/backend.rs).
tokio = { version = "1", features = ["rt-multi-thread", "sync", "macros", "time"] }
# Native file dialogs (portal on Linux), built on the UI thread and awaited
# on the backend runtime.
rfd = "0.17"
# Stable ids for saved connections, so renaming one keeps its identity.
uuid = { version = "1", features = ["v4"] }
```

Create `src/backend.rs` with only tests:

```rust
//! The backend: a tokio runtime on its own thread. The UI sends commands and
//! polls events each frame; the backend wakes the UI when an event is ready.

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tabletist_db::{ConnectSpec, Error, ObjectRef, RowQuery, Secrets};

    const WAIT: Duration = Duration::from_secs(10);

    fn fixture() -> (tempfile::TempDir, ConnectSpec) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fixture.db");
        tabletist_db::fixtures::write_sqlite_demo(&path).unwrap();
        (dir, ConnectSpec::sqlite(path))
    }

    fn woken() -> (Waker, std::sync::Arc<std::sync::atomic::AtomicUsize>) {
        let count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let seen = std::sync::Arc::clone(&count);
        let waker = Waker::new(move || {
            seen.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        });
        (waker, count)
    }

    #[test]
    fn a_session_connects_lists_and_fetches_then_wakes_the_ui() {
        let (_dir, spec) = fixture();
        let (waker, wakes) = woken();
        let mut backend = Backend::start(waker);
        let session = SessionId(1);
        backend.send(Command::Connect {
            session,
            request: RequestId(10),
            spec,
            secrets: Secrets::default(),
        });
        assert!(matches!(
            backend.wait(WAIT),
            Some(Event::Connected { session: SessionId(1), request: RequestId(10), .. })
        ));

        backend.send(Command::ListSchemas { session, request: RequestId(11) });
        match backend.wait(WAIT) {
            Some(Event::Schemas { request: RequestId(11), result: Ok(schemas), .. }) => {
                assert_eq!(schemas, vec!["main".to_owned()]);
            }
            other => panic!("expected schemas, got {other:?}"),
        }
        // `emit` sends before it wakes, so check the wake count only once a
        // later event proves the Connected wake has run.
        assert!(wakes.load(std::sync::atomic::Ordering::SeqCst) >= 1);

        backend.send(Command::FetchRows {
            session,
            request: RequestId(12),
            query: RowQuery::new(ObjectRef::new("main", "users"), 2),
        });
        match backend.wait(WAIT) {
            Some(Event::Rows { request: RequestId(12), result: Ok(page), .. }) => {
                assert_eq!(page.rows.len(), 2);
            }
            other => panic!("expected rows, got {other:?}"),
        }
    }

    #[test]
    fn a_failed_connect_reports_its_error() {
        let mut backend = Backend::start(Waker::default());
        backend.send(Command::Connect {
            session: SessionId(1),
            request: RequestId(1),
            spec: ConnectSpec::sqlite("/definitely/not/here.db"),
            secrets: Secrets::default(),
        });
        assert!(matches!(
            backend.wait(WAIT),
            Some(Event::ConnectFailed { error: Error::Connect(_), .. })
        ));
    }

    #[test]
    fn commands_for_an_unknown_session_fail_as_lost() {
        let mut backend = Backend::start(Waker::default());
        backend.send(Command::ListSchemas { session: SessionId(99), request: RequestId(1) });
        match backend.wait(WAIT) {
            Some(Event::Schemas { result: Err(error), .. }) => assert!(error.is_connection_lost()),
            other => panic!("expected a lost-connection error, got {other:?}"),
        }
    }

    #[test]
    fn a_test_connects_and_closes_without_a_session() {
        let (_dir, spec) = fixture();
        let mut backend = Backend::start(Waker::default());
        backend.send(Command::Test { request: RequestId(5), spec, secrets: Secrets::default() });
        assert!(matches!(
            backend.wait(WAIT),
            Some(Event::Tested { request: RequestId(5), result: Ok(()) })
        ));
    }

    #[test]
    fn cancel_stops_a_running_query_and_the_session_keeps_working() {
        let (_dir, spec) = fixture();
        let mut backend = Backend::start(Waker::default());
        let session = SessionId(1);
        backend.send(Command::Connect { session, request: RequestId(1), spec, secrets: Secrets::default() });
        assert!(matches!(backend.wait(WAIT), Some(Event::Connected { .. })));
        let mut slow = RowQuery::new(ObjectRef::new("main", "big"), 10);
        slow.raw_where = Some("(SELECT count(*) FROM big a, big b) > 0".into());
        backend.send(Command::CountRows { session, request: RequestId(2), query: slow });
        // SQLite ignores an interrupt before the query starts, so keep
        // cancelling until the count answers.
        let deadline = std::time::Instant::now() + WAIT;
        let answer = loop {
            assert!(std::time::Instant::now() < deadline, "cancel must stop the query");
            backend.send(Command::Cancel { session });
            if let Some(event) = backend.wait(Duration::from_millis(200)) {
                break event;
            }
        };
        assert!(matches!(
            answer,
            Event::Count { request: RequestId(2), result: Err(Error::Cancelled), .. }
        ));
        backend.send(Command::ListSchemas { session, request: RequestId(3) });
        assert!(matches!(
            backend.wait(WAIT),
            Some(Event::Schemas { request: RequestId(3), result: Ok(_), .. })
        ));
    }

    #[test]
    fn closing_a_session_makes_later_commands_fail() {
        let (_dir, spec) = fixture();
        let mut backend = Backend::start(Waker::default());
        let session = SessionId(1);
        backend.send(Command::Connect { session, request: RequestId(1), spec, secrets: Secrets::default() });
        assert!(matches!(backend.wait(WAIT), Some(Event::Connected { .. })));
        backend.send(Command::Close { session });
        backend.send(Command::ListSchemas { session, request: RequestId(2) });
        match backend.wait(WAIT) {
            Some(Event::Schemas { result: Err(error), .. }) => assert!(error.is_connection_lost()),
            other => panic!("expected a lost-connection error, got {other:?}"),
        }
    }

    #[test]
    fn closing_a_session_skips_commands_still_queued() {
        let (_dir, spec) = fixture();
        let mut backend = Backend::start(Waker::default());
        let session = SessionId(1);
        backend.send(Command::Connect { session, request: RequestId(1), spec, secrets: Secrets::default() });
        assert!(matches!(backend.wait(WAIT), Some(Event::Connected { .. })));
        let mut slow = RowQuery::new(ObjectRef::new("main", "big"), 10);
        slow.raw_where = Some("(SELECT count(*) FROM big a, big b) > 0".into());
        backend.send(Command::CountRows { session, request: RequestId(2), query: slow });
        for request in 3..10 {
            backend.send(Command::ListSchemas { session, request: RequestId(request) });
        }
        std::thread::sleep(Duration::from_millis(300));
        backend.send(Command::Close { session });
        // The running count ends (cancelled); the queued listings never run.
        let mut schemas = 0;
        while let Some(event) = backend.wait(Duration::from_secs(2)) {
            if matches!(event, Event::Schemas { result: Ok(_), .. }) {
                schemas += 1;
            }
        }
        assert_eq!(schemas, 0);
    }

    #[test]
    fn the_recording_backend_keeps_commands_and_returns_injected_events() {
        let mut backend = Backend::recording();
        backend.send(Command::Close { session: SessionId(3) });
        assert!(matches!(backend.sent.as_slice(), [Command::Close { session: SessionId(3) }]));
        backend.inject(Event::Disconnected { session: SessionId(3), error: Error::Cancelled });
        assert_eq!(backend.poll().len(), 1);
        assert!(backend.poll().is_empty());
    }

    #[test]
    fn commands_do_not_print_secrets() {
        let command = Command::Test {
            request: RequestId(1),
            spec: ConnectSpec::sqlite("/a.db"),
            secrets: Secrets { password: Some("hunter2".into()), ..Secrets::default() },
        };
        assert!(!format!("{command:?}").contains("hunter2"));
    }
}
```

Add `pub mod backend;` to `src/lib.rs`.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --lib backend`
Expected: FAIL to compile: `cannot find type Backend`.

- [ ] **Step 3: Implement the backend**

Put above the tests in `src/backend.rs`:

```rust
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, mpsc};

use tabletist_db::{
    CancelHandle, ConnectSpec, Connection, Driver, Error, ObjectInfo, ObjectRef, RowPage, RowQuery,
    Secrets, Structure,
};
use tokio::sync::mpsc as tokio_mpsc;

/// Wakes the UI from any thread. The default does nothing (tests).
#[derive(Clone)]
pub struct Waker(Arc<dyn Fn() + Send + Sync>);

impl Waker {
    pub fn new(wake: impl Fn() + Send + Sync + 'static) -> Self {
        Self(Arc::new(wake))
    }

    pub fn wake(&self) {
        (self.0)();
    }
}

impl Default for Waker {
    fn default() -> Self {
        Self::new(|| {})
    }
}

/// One database session: one connection tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SessionId(pub u64);

/// Identifies one request so its result can be matched to what asked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RequestId(pub u64);

/// What the UI asks the backend to do. `Secrets` prints as `Secrets { .. }`.
#[derive(Debug)]
pub enum Command {
    Connect { session: SessionId, request: RequestId, spec: ConnectSpec, secrets: Secrets },
    /// Connect and close again, for the dialog's Test button.
    Test { request: RequestId, spec: ConnectSpec, secrets: Secrets },
    Close { session: SessionId },
    Cancel { session: SessionId },
    ListSchemas { session: SessionId, request: RequestId },
    ListObjects { session: SessionId, request: RequestId, schema: String },
    Describe { session: SessionId, request: RequestId, object: ObjectRef },
    FetchRows { session: SessionId, request: RequestId, query: RowQuery },
    CountRows { session: SessionId, request: RequestId, query: RowQuery },
}

/// What the backend reports back.
#[derive(Debug)]
pub enum Event {
    Connected { session: SessionId, request: RequestId, driver: Driver },
    ConnectFailed { session: SessionId, request: RequestId, error: Error },
    Tested { request: RequestId, result: Result<(), Error> },
    /// The session died; later commands to it fail until a reconnect.
    Disconnected { session: SessionId, error: Error },
    Schemas { session: SessionId, request: RequestId, result: Result<Vec<String>, Error> },
    Objects { session: SessionId, request: RequestId, schema: String, result: Result<Vec<ObjectInfo>, Error> },
    Structure { session: SessionId, request: RequestId, result: Result<Structure, Error> },
    Rows { session: SessionId, request: RequestId, result: Result<RowPage, Error> },
    Count { session: SessionId, request: RequestId, result: Result<u64, Error> },
    FilePicked { request: RequestId, path: Option<PathBuf> },
}

/// Sends events to the UI and wakes it.
#[derive(Clone)]
struct Outbox {
    events: mpsc::Sender<Event>,
    waker: Waker,
}

impl Outbox {
    fn emit(&self, event: Event) {
        if self.events.send(event).is_ok() {
            self.waker.wake();
        }
    }
}

pub struct Backend {
    commands: Option<tokio_mpsc::UnboundedSender<Command>>,
    events: mpsc::Receiver<Event>,
    /// Kept so file dialogs can be awaited on the runtime.
    runtime: Option<tokio::runtime::Handle>,
    outbox: Outbox,
    #[cfg(test)]
    pub sent: Vec<Command>,
}

impl Backend {
    /// Starts the backend thread and its runtime.
    pub fn start(waker: Waker) -> Self {
        let (command_tx, command_rx) = tokio_mpsc::unbounded_channel();
        let (event_tx, event_rx) = mpsc::channel();
        let outbox = Outbox { events: event_tx, waker };
        let (handle_tx, handle_rx) = mpsc::channel();
        let worker_outbox = outbox.clone();
        std::thread::Builder::new()
            .name("tabletist-backend".into())
            .spawn(move || {
                let runtime = match tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(2)
                    .enable_all()
                    .thread_name("tabletist-db")
                    .build()
                {
                    Ok(runtime) => runtime,
                    Err(error) => {
                        log::error!("could not start the backend runtime: {error}");
                        return;
                    }
                };
                let _ = handle_tx.send(runtime.handle().clone());
                let (worker, ready) = Worker::new(worker_outbox);
                runtime.block_on(worker.run(command_rx, ready));
            })
            .expect("spawn the backend thread");
        let runtime = handle_rx.recv().ok();
        Self {
            commands: Some(command_tx),
            events: event_rx,
            runtime,
            outbox,
            #[cfg(test)]
            sent: Vec::new(),
        }
    }

    /// A backend with no thread: commands are recorded, events are injected.
    /// For reducer and UI tests.
    pub fn recording() -> Self {
        let (event_tx, event_rx) = mpsc::channel();
        Self {
            commands: None,
            events: event_rx,
            runtime: None,
            outbox: Outbox {
                events: event_tx,
                waker: Waker::default(),
            },
            #[cfg(test)]
            sent: Vec::new(),
        }
    }

    pub fn send(&mut self, command: Command) {
        match &self.commands {
            Some(commands) => {
                if commands.send(command).is_err() {
                    log::error!("the backend has stopped");
                }
            }
            None => {
                #[cfg(test)]
                self.sent.push(command);
            }
        }
    }

    /// Events that arrived since the last poll. Never blocks.
    pub fn poll(&mut self) -> Vec<Event> {
        self.events.try_iter().collect()
    }

    /// Opens the native file dialog for a SQLite file. The dialog is built
    /// here, on the UI thread (macOS requires it), and awaited on the runtime.
    pub fn pick_sqlite_file(&mut self, request: RequestId) {
        let Some(runtime) = &self.runtime else {
            return;
        };
        let dialog = rfd::AsyncFileDialog::new()
            .set_title("Open SQLite database")
            .add_filter("SQLite", &["db", "sqlite", "sqlite3", "db3"])
            .add_filter("All files", &["*"])
            .pick_file();
        let outbox = self.outbox.clone();
        runtime.spawn(async move {
            let path = dialog.await.map(|file| file.path().to_path_buf());
            outbox.emit(Event::FilePicked { request, path });
        });
    }

    #[cfg(test)]
    pub fn inject(&self, event: Event) {
        self.outbox.emit(event);
    }

    #[cfg(test)]
    pub fn wait(&mut self, timeout: std::time::Duration) -> Option<Event> {
        self.events.recv_timeout(timeout).ok()
    }
}

/// A live session as the worker sees it.
struct SessionHandle {
    queue: tokio_mpsc::UnboundedSender<Command>,
    cancel: CancelHandle,
    /// Dropped on Close: the session stops before its next queued command
    /// (a receiver still yields buffered commands after its sender drops).
    _stop: tokio::sync::oneshot::Sender<()>,
}

/// A connect attempt that finished, on its way to the worker. Failures come
/// this way too, so the worker always clears its `connecting` entry.
struct Ready {
    session: SessionId,
    request: RequestId,
    outcome: Result<(Driver, SessionHandle), Error>,
}

/// Runs on the backend runtime. Owns every session.
struct Worker {
    outbox: Outbox,
    sessions: HashMap<SessionId, SessionHandle>,
    /// Connect tasks hand finished sessions to the worker through this.
    ready: tokio_mpsc::UnboundedSender<Ready>,
    /// Sessions closed before they finished connecting.
    closed_early: std::collections::HashSet<SessionId>,
    connecting: std::collections::HashSet<SessionId>,
}

impl Worker {
    fn new(outbox: Outbox) -> (Self, tokio_mpsc::UnboundedReceiver<Ready>) {
        let (ready, ready_rx) = tokio_mpsc::unbounded_channel();
        let worker = Self {
            outbox,
            sessions: HashMap::new(),
            ready,
            closed_early: Default::default(),
            connecting: Default::default(),
        };
        (worker, ready_rx)
    }

    async fn run(
        mut self,
        mut commands: tokio_mpsc::UnboundedReceiver<Command>,
        mut ready: tokio_mpsc::UnboundedReceiver<Ready>,
    ) {
        loop {
            tokio::select! {
                command = commands.recv() => match command {
                    Some(command) => self.handle(command),
                    None => break,
                },
                Some(done) = ready.recv() => self.adopt(done),
            }
        }
    }

    /// Registers a connected session, then tells the UI. `Connected` is sent
    /// only here, after the session is in the map, so a command the UI sends
    /// right after seeing it always finds the session.
    fn adopt(&mut self, done: Ready) {
        self.connecting.remove(&done.session);
        let closed = self.closed_early.remove(&done.session);
        match done.outcome {
            // Dropping the handle stops the session task, which closes it.
            Ok(_) if closed => {}
            Ok((driver, handle)) => {
                self.sessions.insert(done.session, handle);
                self.outbox.emit(Event::Connected {
                    session: done.session,
                    request: done.request,
                    driver,
                });
            }
            Err(error) => self.outbox.emit(Event::ConnectFailed {
                session: done.session,
                request: done.request,
                error,
            }),
        }
    }

    fn handle(&mut self, command: Command) {
        match command {
            Command::Connect { session, request, spec, secrets } => {
                self.connecting.insert(session);
                let outbox = self.outbox.clone();
                let ready = self.ready.clone();
                tokio::spawn(async move {
                    let outcome = Connection::connect(&spec, &secrets).await.map(|connection| {
                        let driver = connection.driver();
                        let cancel = connection.cancel_handle();
                        let (queue, commands) = tokio_mpsc::unbounded_channel();
                        let (stop, stopped) = tokio::sync::oneshot::channel();
                        tokio::spawn(run_session(session, connection, commands, stopped, outbox));
                        (driver, SessionHandle { queue, cancel, _stop: stop })
                    });
                    let _ = ready.send(Ready { session, request, outcome });
                });
            }
            Command::Test { request, spec, secrets } => {
                let outbox = self.outbox.clone();
                tokio::spawn(async move {
                    let result = match Connection::connect(&spec, &secrets).await {
                        Ok(connection) => connection.close().await,
                        Err(error) => Err(error),
                    };
                    outbox.emit(Event::Tested { request, result });
                });
            }
            Command::Close { session } => {
                if let Some(handle) = self.sessions.remove(&session) {
                    let cancel = handle.cancel.clone();
                    tokio::spawn(async move {
                        let _ = cancel.cancel().await;
                    });
                } else if self.connecting.contains(&session) {
                    self.closed_early.insert(session);
                }
            }
            Command::Cancel { session } => {
                if let Some(handle) = self.sessions.get(&session) {
                    let cancel = handle.cancel.clone();
                    tokio::spawn(async move {
                        let _ = cancel.cancel().await;
                    });
                }
            }
            query => {
                let session = session_of(&query);
                match self.sessions.get(&session) {
                    Some(handle) => {
                        if let Err(error) = handle.queue.send(query) {
                            fail(&self.outbox, error.0, lost());
                        }
                    }
                    None => fail(&self.outbox, query, lost()),
                }
            }
        }
    }
}

fn lost() -> Error {
    Error::ConnectionLost("this connection is closed".into())
}

fn session_of(command: &Command) -> SessionId {
    match command {
        Command::Connect { session, .. }
        | Command::Close { session }
        | Command::Cancel { session }
        | Command::ListSchemas { session, .. }
        | Command::ListObjects { session, .. }
        | Command::Describe { session, .. }
        | Command::FetchRows { session, .. }
        | Command::CountRows { session, .. } => *session,
        Command::Test { .. } => SessionId(0),
    }
}

/// Reports `error` as the result of `command`.
fn fail(outbox: &Outbox, command: Command, error: Error) {
    let event = match command {
        Command::ListSchemas { session, request } => Event::Schemas { session, request, result: Err(error) },
        Command::ListObjects { session, request, schema } => {
            Event::Objects { session, request, schema, result: Err(error) }
        }
        Command::Describe { session, request, .. } => Event::Structure { session, request, result: Err(error) },
        Command::FetchRows { session, request, .. } => Event::Rows { session, request, result: Err(error) },
        Command::CountRows { session, request, .. } => Event::Count { session, request, result: Err(error) },
        Command::Connect { .. } | Command::Test { .. } | Command::Close { .. } | Command::Cancel { .. } => return,
    };
    outbox.emit(event);
}

/// Owns one connection and runs its commands one at a time. Ends (closing the
/// connection) when the worker drops its handle, skipping any commands still
/// queued.
async fn run_session(
    session: SessionId,
    connection: Connection,
    mut commands: tokio_mpsc::UnboundedReceiver<Command>,
    mut stop: tokio::sync::oneshot::Receiver<()>,
    outbox: Outbox,
) {
    loop {
        let command = tokio::select! {
            biased;
            _ = &mut stop => break,
            command = commands.recv() => match command {
                Some(command) => command,
                None => break,
            },
        };
        let lost = match command {
            Command::ListSchemas { session, request } => {
                let result = connection.list_schemas().await;
                let lost = lost_error(&result);
                outbox.emit(Event::Schemas { session, request, result });
                lost
            }
            Command::ListObjects { session, request, schema } => {
                let result = connection.list_objects(&schema).await;
                let lost = lost_error(&result);
                outbox.emit(Event::Objects { session, request, schema, result });
                lost
            }
            Command::Describe { session, request, object } => {
                let result = connection.describe(&object).await;
                let lost = lost_error(&result);
                outbox.emit(Event::Structure { session, request, result });
                lost
            }
            Command::FetchRows { session, request, query } => {
                let result = connection.fetch_rows(&query).await;
                let lost = lost_error(&result);
                outbox.emit(Event::Rows { session, request, result });
                lost
            }
            Command::CountRows { session, request, query } => {
                let result = connection.count_rows(&query).await;
                let lost = lost_error(&result);
                outbox.emit(Event::Count { session, request, result });
                lost
            }
            Command::Connect { .. } | Command::Test { .. } | Command::Close { .. } | Command::Cancel { .. } => None,
        };
        if let Some(error) = lost {
            outbox.emit(Event::Disconnected { session, error });
            break;
        }
    }
    let _ = connection.close().await;
}

fn lost_error<T>(result: &Result<T, Error>) -> Option<Error> {
    match result {
        Err(error) if error.is_connection_lost() => Some(error.clone()),
        _ => None,
    }
}
```

Note on `Close`: removing the handle drops its `_stop` sender, so `run_session` stops before its next queued command and closes the connection; the cancel is sent first so the running query stops promptly. A connect that fails also reports through `Ready`, so `connecting` never keeps a stale entry.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --lib backend`
Expected: PASS (9 tests).

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml Cargo.lock src/lib.rs src/backend.rs
git commit -m "Add the backend runtime with sessions, commands and events"
```

---

### Task 2: The saved-connections store

**Files:**
- Create: `src/connections.rs`
- Modify: `src/lib.rs`

**Interfaces:**
- Consumes: `util::{load_json, save_json}`, `tabletist_db::{ConnectSpec, Driver}`.
- Produces:
  - `connections::ConnectionId(pub String)` (`Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize`), `ConnectionId::new() -> ConnectionId` (uuid v4).
  - `connections::ColorTag` (`Debug, Clone, Copy, PartialEq, Eq, Default=None, Serialize, Deserialize` lowercase): `None, Red, Orange, Yellow, Green, Blue, Purple, Gray`; `ColorTag::ALL: [ColorTag; 8]`, `ColorTag::color(self) -> Option<egui::Color32>`, `ColorTag::label(self) -> &'static str`.
  - `connections::SavedConnection { id: ConnectionId, name: String, color: ColorTag, spec: ConnectSpec }` (`Debug, Clone, PartialEq, Serialize, Deserialize`).
  - `connections::SavedConnections { version: u32, connections: Vec<SavedConnection> }` (`Default`), `SavedConnections::load(path: &Path) -> SavedConnections`, `save(&self, path: &Path) -> std::io::Result<()>`, `get(&self, id: &ConnectionId) -> Option<&SavedConnection>`, `upsert(&mut self, connection: SavedConnection)`, `remove(&mut self, id: &ConnectionId) -> Option<SavedConnection>`, `duplicate(&mut self, id: &ConnectionId) -> Option<ConnectionId>`, `search(&self, text: &str) -> Vec<&SavedConnection>`.

- [ ] **Step 1: Write the failing tests**

Create `src/connections.rs` with only tests:

```rust
//! Saved connections, stored as `connections.json`. Specs only: passwords
//! live in the OS keyring (batch 4), never in this file.

#[cfg(test)]
mod tests {
    use super::*;
    use tabletist_db::ConnectSpec;

    fn saved(name: &str, file: &str) -> SavedConnection {
        SavedConnection {
            id: ConnectionId::new(),
            name: name.into(),
            color: ColorTag::None,
            spec: ConnectSpec::sqlite(file),
        }
    }

    #[test]
    fn connections_round_trip_through_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("connections.json");
        let mut store = SavedConnections::default();
        store.upsert(SavedConnection { color: ColorTag::Red, ..saved("Prod", "/prod.db") });
        store.save(&path).unwrap();
        let loaded = SavedConnections::load(&path);
        assert_eq!(loaded.connections, store.connections);
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("\"red\""));
    }

    #[test]
    fn a_damaged_store_is_kept_aside() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("connections.json");
        std::fs::write(&path, "[not json").unwrap();
        assert!(SavedConnections::load(&path).connections.is_empty());
        assert!(dir.path().join("connections.json.bad").exists());
    }

    #[test]
    fn upsert_replaces_by_id_and_keeps_order() {
        let mut store = SavedConnections::default();
        let first = saved("A", "/a.db");
        let second = saved("B", "/b.db");
        store.upsert(first.clone());
        store.upsert(second.clone());
        store.upsert(SavedConnection { name: "A2".into(), ..first.clone() });
        let names: Vec<&str> = store.connections.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["A2", "B"]);
    }

    #[test]
    fn remove_and_duplicate_work_by_id() {
        let mut store = SavedConnections::default();
        let original = saved("Local", "/l.db");
        store.upsert(original.clone());
        let copy = store.duplicate(&original.id).unwrap();
        assert_ne!(copy, original.id);
        assert_eq!(store.get(&copy).unwrap().name, "Local copy");
        assert_eq!(store.get(&copy).unwrap().spec, original.spec);
        assert!(store.remove(&original.id).is_some());
        assert!(store.get(&original.id).is_none());
        assert!(store.duplicate(&original.id).is_none());
    }

    #[test]
    fn search_matches_name_and_summary_case_insensitively() {
        let mut store = SavedConnections::default();
        store.upsert(saved("Production", "/srv/prod.db"));
        store.upsert(saved("Staging", "/srv/stage.db"));
        let names = |text: &str| -> Vec<String> {
            store.search(text).iter().map(|c| c.name.clone()).collect()
        };
        assert_eq!(names("PROD"), vec!["Production"]);
        assert_eq!(names("stage.db"), vec!["Staging"]);
        assert_eq!(names("  "), vec!["Production", "Staging"]);
        assert!(names("mysql").is_empty());
    }

    #[test]
    fn every_tag_but_none_has_a_colour() {
        for tag in ColorTag::ALL {
            assert_eq!(tag.color().is_none(), tag == ColorTag::None, "{tag:?}");
        }
    }
}
```

Add `pub mod connections;` to `src/lib.rs`.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --lib connections`
Expected: FAIL to compile: `cannot find type SavedConnections`.

- [ ] **Step 3: Implement the store**

Put above the tests in `src/connections.rs`:

```rust
use std::path::Path;

use egui::Color32;
use serde::{Deserialize, Serialize};
use tabletist_db::ConnectSpec;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConnectionId(pub String);

impl ConnectionId {
    #[allow(clippy::new_without_default)] // a new id is never a "default"
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}

/// A colour to tell connections apart at a glance (red for production).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ColorTag {
    #[default]
    None,
    Red,
    Orange,
    Yellow,
    Green,
    Blue,
    Purple,
    Gray,
}

impl ColorTag {
    pub const ALL: [ColorTag; 8] = [
        Self::None,
        Self::Red,
        Self::Orange,
        Self::Yellow,
        Self::Green,
        Self::Blue,
        Self::Purple,
        Self::Gray,
    ];

    pub fn color(self) -> Option<Color32> {
        match self {
            Self::None => None,
            Self::Red => Some(Color32::from_rgb(0xe5, 0x48, 0x4d)),
            Self::Orange => Some(Color32::from_rgb(0xf0, 0x8c, 0x2e)),
            Self::Yellow => Some(Color32::from_rgb(0xe6, 0xc2, 0x29)),
            Self::Green => Some(Color32::from_rgb(0x3f, 0xb9, 0x50)),
            Self::Blue => Some(Color32::from_rgb(0x3b, 0x82, 0xf6)),
            Self::Purple => Some(Color32::from_rgb(0x9b, 0x5d, 0xe5)),
            Self::Gray => Some(Color32::from_rgb(0x8b, 0x93, 0x9e)),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::None => "No color",
            Self::Red => "Red",
            Self::Orange => "Orange",
            Self::Yellow => "Yellow",
            Self::Green => "Green",
            Self::Blue => "Blue",
            Self::Purple => "Purple",
            Self::Gray => "Gray",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SavedConnection {
    pub id: ConnectionId,
    pub name: String,
    #[serde(default)]
    pub color: ColorTag,
    pub spec: ConnectSpec,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SavedConnections {
    pub version: u32,
    pub connections: Vec<SavedConnection>,
}

impl Default for SavedConnections {
    fn default() -> Self {
        Self {
            version: 1,
            connections: Vec::new(),
        }
    }
}

impl SavedConnections {
    pub fn load(path: &Path) -> Self {
        crate::util::load_json(path)
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        crate::util::save_json(path, self)
    }

    pub fn get(&self, id: &ConnectionId) -> Option<&SavedConnection> {
        self.connections.iter().find(|connection| &connection.id == id)
    }

    /// Replaces the connection with the same id, or appends it.
    pub fn upsert(&mut self, connection: SavedConnection) {
        match self.connections.iter_mut().find(|existing| existing.id == connection.id) {
            Some(existing) => *existing = connection,
            None => self.connections.push(connection),
        }
    }

    pub fn remove(&mut self, id: &ConnectionId) -> Option<SavedConnection> {
        let index = self.connections.iter().position(|connection| &connection.id == id)?;
        Some(self.connections.remove(index))
    }

    /// Copies a connection under a new id, right after the original.
    pub fn duplicate(&mut self, id: &ConnectionId) -> Option<ConnectionId> {
        let index = self.connections.iter().position(|connection| &connection.id == id)?;
        let mut copy = self.connections[index].clone();
        copy.id = ConnectionId::new();
        copy.name = format!("{} copy", copy.name);
        let new_id = copy.id.clone();
        self.connections.insert(index + 1, copy);
        Some(new_id)
    }

    /// Connections whose name or summary contains `text`, ignoring case.
    pub fn search(&self, text: &str) -> Vec<&SavedConnection> {
        let needle = text.trim().to_lowercase();
        self.connections
            .iter()
            .filter(|connection| {
                needle.is_empty()
                    || connection.name.to_lowercase().contains(&needle)
                    || connection.spec.summary().to_lowercase().contains(&needle)
            })
            .collect()
    }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --lib connections`
Expected: PASS (6 tests).

- [ ] **Step 5: Commit**

```bash
git add src/lib.rs src/connections.rs
git commit -m "Add the saved-connections store"
```

---

### Task 3: Model and app wiring for workspaces, dialogs and the backend

**Files:**
- Modify: `src/model.rs`, `src/app.rs`, `src/entrypoint.rs`, `src/testing.rs`, `src/ui/mod.rs`, `src/ui/conn_tabs.rs`

**Interfaces:**
- Consumes: `backend::{Backend, Waker, SessionId, RequestId, Command, Event}`, `connections::{SavedConnections, SavedConnection, ConnectionId, ColorTag}`.
- Produces (model additions):
  - `model::SessionStatus` (`Debug`): `Connecting { request: RequestId }`, `Connected`, `Disconnected(tabletist_db::Error)`.
  - `model::Workspace { session: SessionId, conn_id: ConnectionId, name: String, color: ColorTag, spec: ConnectSpec, status: SessionStatus, driver: Driver }` (Batch 3 adds tree and object tabs).
  - `ConnTabContent::Workspace(Box<Workspace>)`.
  - `model::ConnectionForm { editing: Option<ConnectionId>, name: String, color: ColorTag, sqlite_path: String, url: String, message: Option<String>, test: TestState, pick_request: Option<RequestId> }` (`Default`), `model::TestState` (`Default=Idle`): `Idle`, `Running(RequestId)`, `Passed`, `Failed(String)`.
  - `model::Dialog`: `Connection(Box<ConnectionForm>)`.
  - New `Action` variants: `Backend(Event)`, `NewConnection`, `EditConnection(ConnectionId)`, `DuplicateConnection(ConnectionId)`, `DeleteConnection(ConnectionId)`, `CloseDialog`, `PickSqliteFile`, `ApplyUrl`, `TestConnection`, `SaveConnection { connect: bool }`, `Connect { tab: ConnTabId, conn: ConnectionId }`, `Reconnect(ConnTabId)`, `Disconnect(ConnTabId)`.
  - `App` gains pub fields `backend: Backend`, `connections: SavedConnections`, `dialog: Option<Dialog>`; `App::new(dirs, settings, backend: Backend) -> App` (signature change); `App::poll_backend(&mut self)` (drains events into actions); `App::workspace(&self, tab: ConnTabId) -> Option<&Workspace>`; `App::workspace_mut(&mut self, tab: ConnTabId) -> Option<&mut Workspace>`; `App::tab_for_session(&self, session: SessionId) -> Option<ConnTabId>`.
  - `testing::Harness` uses `Backend::recording()`.

This task changes signatures only and routes the new content; the behaviour and its tests come in Task 4. Existing tests must keep passing.

- [ ] **Step 1: Extend the model**

Add to `src/model.rs`:

```rust
use tabletist_db::{ConnectSpec, Driver};

use crate::backend::{Event, RequestId, SessionId};
use crate::connections::{ColorTag, ConnectionId};

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
}

#[derive(Debug)]
pub enum SessionStatus {
    Connecting { request: RequestId },
    Connected,
    Disconnected(tabletist_db::Error),
}

#[derive(Debug, Default, PartialEq)]
pub enum TestState {
    #[default]
    Idle,
    Running(RequestId),
    Passed,
    Failed(String),
}

/// The connection dialog's fields while it is open.
#[derive(Debug, Default)]
pub struct ConnectionForm {
    /// `Some` when editing a saved connection, `None` for a new one.
    pub editing: Option<ConnectionId>,
    pub name: String,
    pub color: ColorTag,
    pub sqlite_path: String,
    /// Text in the "Paste URL" field.
    pub url: String,
    /// A validation or URL error shown under the fields.
    pub message: Option<String>,
    pub test: TestState,
    /// The file dialog request in flight, if any.
    pub pick_request: Option<RequestId>,
}

#[derive(Debug)]
pub enum Dialog {
    Connection(Box<ConnectionForm>),
}
```

Change `ConnTabContent` to:

```rust
#[derive(Debug)]
pub enum ConnTabContent {
    /// Choose a saved connection (or create one).
    Picker(PickerState),
    /// A connection, connected or not.
    Workspace(Box<Workspace>),
}
```

Remove `Clone` from `Action`'s derive (events are not `Clone`) and add variants:

```rust
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
    /// Fill the connection dialog from its URL field.
    ApplyUrl,
    TestConnection,
    /// Save the dialog's connection; `connect` also opens it in the active tab.
    SaveConnection { connect: bool },
    /// Open a saved connection in this tab (replacing its picker).
    Connect { tab: ConnTabId, conn: ConnectionId },
    Reconnect(ConnTabId),
    /// Close the connection and turn the tab back into a picker.
    Disconnect(ConnTabId),
```

- [ ] **Step 2: Wire the backend and store into App**

In `src/app.rs`, add fields and change `new`:

```rust
use crate::backend::{Backend, SessionId};
use crate::connections::SavedConnections;
use crate::model::{Dialog, Workspace};
```

```rust
    pub backend: Backend,
    pub connections: SavedConnections,
    pub dialog: Option<Dialog>,
```

```rust
    pub fn new(dirs: AppDirs, settings: Settings, backend: Backend) -> Self {
        let connections = SavedConnections::load(&dirs.connections_file());
        let mut app = Self {
            dirs,
            settings,
            locale: Locale::default(),
            palette: Palette::dark(),
            themes: Catalog::default(),
            tabs: Vec::new(),
            active: 0,
            actions: Vec::new(),
            backend,
            connections,
            dialog: None,
            system_theme: None,
            next_id: 1,
        };
        let tab = app.picker_tab();
        app.tabs.push(tab);
        app
    }

    pub fn workspace(&self, tab: ConnTabId) -> Option<&Workspace> {
        match &self.tabs.iter().find(|t| t.id == tab)?.content {
            ConnTabContent::Workspace(workspace) => Some(workspace),
            ConnTabContent::Picker(_) => None,
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
```

In `frame_ui`, poll first:

```rust
    pub fn frame_ui(&mut self, ui: &mut egui::Ui) {
        self.poll_backend();
        self.apply_actions();
        crate::ui::keys::handle(self, ui.ctx());
        crate::ui::show(self, ui);
        self.apply_actions();
    }
```

In `apply`, add a temporary catch-all arm so the new variants compile until Task 4 implements them:

```rust
            other => log::debug!("not handled yet: {other:?}"),
```

Update the test helper in `app.rs` to `App::new(AppDirs::at(dir.path()), Settings::default(), Backend::recording())`.

In `src/testing.rs`, create the app with `Backend::recording()` the same way.

In `src/entrypoint.rs`, build the backend inside the eframe creator so its waker repaints the window:

```rust
        Box::new(move |cc| {
            let repaint = cc.egui_ctx.clone();
            let backend = crate::backend::Backend::start(crate::backend::Waker::new(move || {
                repaint.request_repaint()
            }));
            let mut app = App::new(dirs, settings, backend);
            app.attach(&cc.egui_ctx, !demo);
            if demo {
                demo_setup(&mut app);
            }
            Ok(Box::new(Window { app, shot, demo }))
        }),
```

- [ ] **Step 3: Route workspace tabs in the UI**

In `src/ui/mod.rs`, add `pub mod workspace;` and change the central panel body:

```rust
        .show(ui, |ui| {
            let tab = app.active_tab_id();
            if matches!(app.active_tab().content, ConnTabContent::Picker(_)) {
                picker::show(app, ui);
            } else {
                workspace::show(app, ui, tab);
            }
        });
```

Create `src/ui/workspace.rs` with a minimal body for now (Task 6 completes it):

```rust
//! A connected tab: top bar, banner, and (batch 3) sidebar and object tabs.

use crate::app::App;
use crate::model::ConnTabId;

pub fn show(_app: &mut App, _ui: &mut egui::Ui, _tab: ConnTabId) {}
```

In `src/ui/conn_tabs.rs`, extend `tab_title`:

```rust
pub fn tab_title(app: &App, tab: &ConnTab) -> String {
    match &tab.content {
        ConnTabContent::Picker(_) => gettext(app.locale, "New tab").into_owned(),
        ConnTabContent::Workspace(workspace) => workspace.name.clone(),
    }
}
```

- [ ] **Step 4: Run all tests**

Run: `cargo test --locked --all-targets`
Expected: PASS (every earlier test; no behaviour changed).

- [ ] **Step 5: Commit**

```bash
git add src
git commit -m "Wire the backend, saved connections and workspace tabs into the app"
```

---

### Task 4: Connect, close, disconnect and reconnect in the reducer

**Files:**
- Modify: `src/app.rs`

**Interfaces:**
- Consumes: Task 3's model and `App` fields; `Backend::recording().sent` in tests.
- Produces: reducer behaviour for `Connect`, `Reconnect`, `Disconnect`, `CloseConnTab` (now also closes the session), and `Backend(Connected | ConnectFailed | Disconnected)`. `App::connect_tab(&mut self, tab: ConnTabId, saved: SavedConnection)` (private helper is fine).

- [ ] **Step 1: Write the failing reducer tests**

Add to the `tests` module in `src/app.rs`:

```rust
    use crate::backend::{Command, Event, RequestId, SessionId};
    use crate::connections::{ColorTag, ConnectionId, SavedConnection};
    use crate::model::SessionStatus;
    use tabletist_db::{ConnectSpec, Driver, Error};

    fn with_saved(app: &mut App) -> ConnectionId {
        let saved = SavedConnection {
            id: ConnectionId::new(),
            name: "Local".into(),
            color: ColorTag::Green,
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
            Some(Command::Connect { session, request, .. }) => (tab, *session, *request),
            other => panic!("expected a Connect command, got {other:?}"),
        }
    }

    #[test]
    fn connecting_turns_the_picker_into_a_connecting_workspace() {
        let (mut app, _dir) = app();
        let (tab, _, request) = connect(&mut app);
        let workspace = app.workspace(tab).unwrap();
        assert_eq!(workspace.name, "Local");
        assert!(matches!(workspace.status, SessionStatus::Connecting { request: r } if r == request));
    }

    #[test]
    fn a_connected_event_marks_the_tab_connected() {
        let (mut app, _dir) = app();
        let (tab, session, request) = connect(&mut app);
        app.apply(Action::Backend(Event::Connected { session, request, driver: Driver::Sqlite }));
        assert!(matches!(app.workspace(tab).unwrap().status, SessionStatus::Connected));
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
        assert!(matches!(app.backend.sent.last(), Some(Command::Close { session: s }) if *s == session));
        app.apply(Action::Backend(Event::Connected { session, request, driver: Driver::Sqlite }));
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
        }));
        assert!(matches!(app.workspace(tab).unwrap().status, SessionStatus::Connecting { .. }));
        assert!(matches!(
            app.backend.sent.get(before),
            Some(Command::Close { session }) if *session == first_session
        ));
    }

    #[test]
    fn a_disconnect_event_shows_the_banner_state() {
        let (mut app, _dir) = app();
        let (tab, session, request) = connect(&mut app);
        app.apply(Action::Backend(Event::Connected { session, request, driver: Driver::Sqlite }));
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
        app.apply(Action::Backend(Event::Connected { session, request, driver: Driver::Sqlite }));
        app.apply(Action::Reconnect(tab));
        let new_session = app.workspace(tab).unwrap().session;
        assert_ne!(new_session, session);
        assert!(app.backend.sent.iter().any(|c| matches!(c, Command::Close { session: s } if *s == session)));
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
            Some(Command::Connect { session, request, .. }) => (*session, *request),
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
        assert!(matches!(app.tabs[app.active].content, ConnTabContent::Picker(_)));
        assert!(matches!(app.backend.sent.last(), Some(Command::Close { session: s }) if *s == session));
    }

    #[test]
    fn connecting_an_unknown_saved_connection_does_nothing() {
        let (mut app, _dir) = app();
        let tab = app.active_tab_id();
        app.apply(Action::Connect { tab, conn: ConnectionId("missing".into()) });
        assert!(app.workspace(tab).is_none());
        assert!(app.backend.sent.is_empty());
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --lib app::tests`
Expected: FAIL (the catch-all arm ignores the new actions).

- [ ] **Step 3: Implement the reducer**

In `App::apply`, replace the catch-all with these arms (the dialog arms come in Task 5; keep a catch-all for them until then):

```rust
            Action::Connect { tab, conn } => {
                if let Some(saved) = self.connections.get(&conn).cloned() {
                    self.connect_tab(tab, saved);
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
            Action::Backend(event) => self.apply_event(event),
            other => log::debug!("not handled yet: {other:?}"),
```

Change `close_tab` to close the session first:

```rust
    fn close_tab(&mut self, id: ConnTabId) {
        let Some(index) = self.tab_index(id) else {
            return;
        };
        if let ConnTabContent::Workspace(workspace) = &self.tabs[index].content {
            self.backend.send(Command::Close { session: workspace.session });
        }
        self.tabs.remove(index);
        // ...the rest as before
    }
```

Add the helpers:

```rust
    fn connect_tab(&mut self, tab: ConnTabId, saved: SavedConnection) {
        let session = SessionId(self.next_id());
        let request = RequestId(self.next_id());
        let Some(entry) = self.tabs.iter_mut().find(|t| t.id == tab) else {
            return;
        };
        entry.content = ConnTabContent::Workspace(Box::new(Workspace {
            session,
            conn_id: saved.id,
            name: saved.name,
            color: saved.color,
            driver: saved.spec.driver,
            spec: saved.spec.clone(),
            status: SessionStatus::Connecting { request },
        }));
        self.backend.send(Command::Connect {
            session,
            request,
            spec: saved.spec,
            secrets: Secrets::default(),
        });
    }

    fn reconnect(&mut self, tab: ConnTabId) {
        let session = SessionId(self.next_id());
        let request = RequestId(self.next_id());
        let Some(workspace) = self.workspace_mut(tab) else {
            return;
        };
        let old = std::mem::replace(&mut workspace.session, session);
        workspace.status = SessionStatus::Connecting { request };
        let spec = workspace.spec.clone();
        self.backend.send(Command::Close { session: old });
        self.backend.send(Command::Connect {
            session,
            request,
            spec,
            secrets: Secrets::default(),
        });
    }

    fn apply_event(&mut self, event: Event) {
        match event {
            Event::Connected { session, request, driver } => {
                let adopted = self.tab_for_session(session).and_then(|tab| {
                    let workspace = self.workspace_mut(tab)?;
                    match workspace.status {
                        SessionStatus::Connecting { request: waiting } if waiting == request => {
                            workspace.status = SessionStatus::Connected;
                            workspace.driver = driver;
                            Some(tab)
                        }
                        _ => None,
                    }
                });
                if adopted.is_none() {
                    // Nobody is waiting for this session any more.
                    self.backend.send(Command::Close { session });
                }
            }
            Event::ConnectFailed { session, request, error } => {
                if let Some(tab) = self.tab_for_session(session)
                    && let Some(workspace) = self.workspace_mut(tab)
                    && matches!(workspace.status, SessionStatus::Connecting { request: r } if r == request)
                {
                    workspace.status = SessionStatus::Disconnected(error);
                }
            }
            Event::Disconnected { session, error } => {
                if let Some(tab) = self.tab_for_session(session)
                    && let Some(workspace) = self.workspace_mut(tab)
                {
                    workspace.status = SessionStatus::Disconnected(error);
                }
            }
            other => log::debug!("unhandled event {other:?}"),
        }
    }
```

Add the imports this needs at the top of `app.rs`:

```rust
use tabletist_db::Secrets;

use crate::backend::{Command, Event, RequestId};
use crate::connections::SavedConnection;
use crate::model::SessionStatus;
```

The `Close` sent in `Event::Connected` for an unknown session is what `a_late_session_is_closed_not_adopted` checks; it also covers a tab closed mid-connect (the worker ignores a `Close` for a session it already dropped).

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --lib app::tests`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/app.rs
git commit -m "Connect, close, disconnect and reconnect connection tabs"
```

---

### Task 5: The connection dialog and saved-connection actions

**Files:**
- Modify: `src/app.rs`
- Create: `src/ui/connect_dialog.rs`
- Modify: `src/ui/mod.rs` (draw the dialog), `src/ui/keys.rs` (Cmd/Ctrl+N)

**Interfaces:**
- Consumes: `ConnectionForm`, `TestState`, `Dialog`, `SavedConnections`, `Backend::pick_sqlite_file`, `ConnectSpec::from_url`.
- Produces: reducer behaviour for `NewConnection`, `EditConnection`, `DuplicateConnection`, `DeleteConnection`, `CloseDialog`, `PickSqliteFile`, `ApplyUrl`, `TestConnection`, `SaveConnection`, and `Backend(Tested | FilePicked)`; `ConnectionForm::from_saved(saved: &SavedConnection) -> ConnectionForm`; `ConnectionForm::to_saved(&self) -> Result<SavedConnection, String>`; `ui::connect_dialog::show(app: &mut App, ctx: &egui::Context)`.

- [ ] **Step 1: Write the failing tests**

Add to the `tests` module in `src/app.rs`:

```rust
    use crate::model::{ConnectionForm, Dialog, TestState};

    fn form(app: &mut App) -> &mut ConnectionForm {
        match app.dialog.as_mut() {
            Some(Dialog::Connection(form)) => form,
            None => panic!("the dialog is not open"),
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

    #[test]
    fn saved_connections_are_written_to_disk() {
        let (mut app, dir) = app();
        app.apply(Action::NewConnection);
        form(&mut app).name = "Disk".into();
        form(&mut app).sqlite_path = "/tmp/disk.db".into();
        app.apply(Action::SaveConnection { connect: false });
        let stored = crate::connections::SavedConnections::load(&AppDirs::at(dir.path()).connections_file());
        assert_eq!(stored.connections[0].name, "Disk");
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
        form(&mut app).url = "postgres://h/db".into();
        app.apply(Action::ApplyUrl);
        assert!(form(&mut app).message.as_deref().unwrap().contains("PostgreSQL"));
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
        app.apply(Action::Backend(Event::Tested { request: RequestId(request.0 + 1000), result: Ok(()) }));
        assert!(matches!(form(&mut app).test, TestState::Running(_)));
        app.apply(Action::Backend(Event::Tested { request, result: Err(Error::Connect("nope".into())) }));
        assert!(matches!(&form(&mut app).test, TestState::Failed(message) if message.contains("nope")));
        app.apply(Action::CloseDialog);
        app.apply(Action::Backend(Event::Tested { request, result: Ok(()) }));
        assert!(app.dialog.is_none());
    }

    #[test]
    fn a_picked_file_fills_the_path_and_a_cancelled_pick_changes_nothing() {
        let (mut app, _dir) = app();
        app.apply(Action::NewConnection);
        form(&mut app).pick_request = Some(RequestId(77));
        app.apply(Action::Backend(Event::FilePicked { request: RequestId(77), path: None }));
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
        let stored = crate::connections::SavedConnections::load(&AppDirs::at(dir.path()).connections_file());
        assert!(stored.get(&conn).is_none());
    }

    #[test]
    fn duplicating_adds_a_copy_right_after() {
        let (mut app, _dir) = app();
        let id = with_saved(&mut app);
        app.apply(Action::DuplicateConnection(id));
        let names: Vec<&str> = app.connections.connections.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["Local", "Local copy"]);
    }
```

Add a UI test to `src/ui/mod.rs` tests:

```rust
    #[test]
    fn ctrl_n_opens_the_connection_dialog_and_escape_closes_it() {
        let mut harness = Harness::new();
        harness.press(Key::N, Modifiers::COMMAND);
        assert!(harness.app.dialog.is_some());
        assert!(harness.has("Save & Connect"));
        harness.press(Key::Escape, Modifiers::NONE);
        assert!(harness.app.dialog.is_none());
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --lib`
Expected: FAIL (dialog actions not handled; `connect_dialog` missing).

- [ ] **Step 3: Implement the form helpers and the reducer arms**

Add to `src/model.rs`:

```rust
impl ConnectionForm {
    pub fn from_saved(saved: &crate::connections::SavedConnection) -> Self {
        Self {
            editing: Some(saved.id.clone()),
            name: saved.name.clone(),
            color: saved.color,
            sqlite_path: saved
                .spec
                .sqlite_path
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_default(),
            ..Self::default()
        }
    }

    /// The connection the form describes, or why it cannot be saved.
    pub fn to_saved(&self) -> Result<crate::connections::SavedConnection, String> {
        let name = self.name.trim();
        if name.is_empty() {
            return Err("Give the connection a name.".into());
        }
        let path = self.sqlite_path.trim();
        if path.is_empty() {
            return Err("Choose a SQLite file.".into());
        }
        Ok(crate::connections::SavedConnection {
            id: self.editing.clone().unwrap_or_else(ConnectionId::new),
            name: name.to_owned(),
            color: self.color,
            spec: ConnectSpec::sqlite(path),
        })
    }
}

/// The file name of `path`, for naming a connection after its file.
pub fn file_name(path: &str) -> String {
    std::path::Path::new(path)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}
```

Replace the catch-all in `App::apply` with:

```rust
            Action::NewConnection => {
                self.dialog = Some(Dialog::Connection(Box::default()));
            }
            Action::EditConnection(id) => {
                if let Some(saved) = self.connections.get(&id) {
                    self.dialog = Some(Dialog::Connection(Box::new(ConnectionForm::from_saved(saved))));
                }
            }
            Action::DuplicateConnection(id) => {
                if self.connections.duplicate(&id).is_some() {
                    self.save_connections();
                }
            }
            Action::DeleteConnection(id) => {
                if self.connections.remove(&id).is_some() {
                    self.save_connections();
                }
            }
            Action::CloseDialog => self.dialog = None,
            Action::PickSqliteFile => {
                let request = RequestId(self.next_id());
                if let Some(Dialog::Connection(form)) = &mut self.dialog {
                    form.pick_request = Some(request);
                    self.backend.pick_sqlite_file(request);
                }
            }
            Action::ApplyUrl => {
                if let Some(Dialog::Connection(form)) = &mut self.dialog {
                    apply_url(form);
                }
            }
            Action::TestConnection => {
                // Testing needs only the file, not a name.
                let request = RequestId(self.next_id());
                if let Some(Dialog::Connection(form)) = &mut self.dialog {
                    let path = form.sqlite_path.trim();
                    if path.is_empty() {
                        form.message = Some("Choose a SQLite file.".into());
                    } else {
                        let spec = tabletist_db::ConnectSpec::sqlite(path);
                        form.message = None;
                        form.test = TestState::Running(request);
                        self.backend.send(Command::Test { request, spec, secrets: Secrets::default() });
                    }
                }
            }
            Action::SaveConnection { connect } => self.save_dialog(connect),
```

and add to `apply_event`:

```rust
            Event::Tested { request, result } => {
                if let Some(Dialog::Connection(form)) = &mut self.dialog
                    && form.test == TestState::Running(request)
                {
                    form.test = match result {
                        Ok(()) => TestState::Passed,
                        Err(error) => TestState::Failed(error.to_string()),
                    };
                }
            }
            Event::FilePicked { request, path } => {
                if let Some(Dialog::Connection(form)) = &mut self.dialog
                    && form.pick_request == Some(request)
                {
                    form.pick_request = None;
                    if let Some(path) = path {
                        form.sqlite_path = path.display().to_string();
                        if form.name.trim().is_empty() {
                            form.name = crate::model::file_name(&form.sqlite_path);
                        }
                    }
                }
            }
```

Add the helpers to `impl App` and a free function:

```rust
    fn save_connections(&self) {
        if let Err(error) = self.connections.save(&self.dirs.connections_file()) {
            log::error!("could not save connections: {error}");
        }
    }

    fn save_dialog(&mut self, connect: bool) {
        let Some(Dialog::Connection(form)) = &mut self.dialog else {
            return;
        };
        let saved = match form.to_saved() {
            Ok(saved) => saved,
            Err(message) => {
                form.message = Some(message);
                return;
            }
        };
        self.dialog = None;
        self.connections.upsert(saved.clone());
        self.save_connections();
        if connect {
            if !matches!(self.active_tab().content, ConnTabContent::Picker(_)) {
                self.apply(Action::NewConnTab);
            }
            let tab = self.active_tab_id();
            self.connect_tab(tab, saved);
        }
    }
```

```rust
/// Fills the form from its URL field. Only SQLite URLs can be used until
/// PostgreSQL and MySQL arrive.
fn apply_url(form: &mut ConnectionForm) {
    match tabletist_db::ConnectSpec::from_url(&form.url) {
        Ok((spec, _)) if spec.driver == tabletist_db::Driver::Sqlite => {
            let path = spec
                .sqlite_path
                .map(|path| path.display().to_string())
                .unwrap_or_default();
            if form.name.trim().is_empty() {
                form.name = crate::model::file_name(&path);
            }
            form.sqlite_path = path;
            form.message = None;
        }
        Ok((spec, _)) => {
            form.message = Some(format!(
                "{} connections arrive in a later version.",
                spec.driver.label()
            ));
        }
        Err(error) => form.message = Some(error.to_string()),
    }
}
```

Merge the `crate::model` imports in `app.rs` into one line, `use crate::model::{ConnectionForm, Dialog, SessionStatus, TestState, Workspace};` (plus the names Batch 0 already imports); a second `use` of `Dialog` does not compile.

- [ ] **Step 4: Implement the dialog view and the shortcut**

Create `src/ui/connect_dialog.rs`:

```rust
//! The connection dialog: new or edit, SQLite only for now.

use egui::{Align2, RichText, TextEdit};

use crate::app::App;
use crate::connections::ColorTag;
use crate::i18n::gettext;
use crate::model::{Action, Dialog, TestState};
use crate::theme;

pub fn show(app: &mut App, ctx: &egui::Context) {
    let locale = app.locale;
    let palette = app.palette;
    let Some(Dialog::Connection(form)) = &mut app.dialog else {
        return;
    };
    let mut actions = Vec::new();
    let title = if form.editing.is_some() {
        gettext(locale, "Edit connection")
    } else {
        gettext(locale, "New connection")
    };
    let mut open = true;
    egui::Window::new(title.as_ref())
        .collapsible(false)
        .resizable(false)
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
        .fixed_size([460.0, 0.0])
        .open(&mut open)
        .show(ctx, |ui| {
            egui::Grid::new("connection-form")
                .num_columns(2)
                .spacing([12.0, 8.0])
                .show(ui, |ui| {
                    ui.label(gettext(locale, "Driver"));
                    ui.label(RichText::new("SQLite").color(palette.secondary));
                    ui.end_row();

                    ui.label(gettext(locale, "Name"));
                    ui.add(TextEdit::singleline(&mut form.name).desired_width(f32::INFINITY));
                    ui.end_row();

                    ui.label(gettext(locale, "Color"));
                    egui::ComboBox::from_id_salt("color-tag")
                        .selected_text(form.color.label())
                        .show_ui(ui, |ui| {
                            for tag in ColorTag::ALL {
                                ui.selectable_value(&mut form.color, tag, tag.label());
                            }
                        });
                    ui.end_row();

                    ui.label(gettext(locale, "File"));
                    ui.horizontal(|ui| {
                        ui.add(TextEdit::singleline(&mut form.sqlite_path).desired_width(300.0));
                        if ui.button(gettext(locale, "Choose…")).clicked() {
                            actions.push(Action::PickSqliteFile);
                        }
                    });
                    ui.end_row();

                    ui.label(gettext(locale, "Paste URL"));
                    ui.horizontal(|ui| {
                        let response = ui.add(
                            TextEdit::singleline(&mut form.url)
                                .hint_text("sqlite:///path/to/file.db")
                                .desired_width(300.0),
                        );
                        let entered = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                        if ui.button(gettext(locale, "Fill")).clicked() || entered {
                            actions.push(Action::ApplyUrl);
                        }
                    });
                    ui.end_row();
                });

            if let Some(message) = &form.message {
                ui.add_space(4.0);
                ui.label(RichText::new(message).color(palette.danger));
            }
            match &form.test {
                TestState::Idle => {}
                TestState::Running(_) => {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(gettext(locale, "Testing…"));
                    });
                }
                TestState::Passed => {
                    ui.label(RichText::new(gettext(locale, "Connection works.")).color(palette.accent));
                }
                TestState::Failed(message) => {
                    ui.label(RichText::new(message).color(palette.danger));
                }
            }

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button(gettext(locale, "Test")).clicked() {
                    actions.push(Action::TestConnection);
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let connect = egui::Button::new(
                        RichText::new(gettext(locale, "Save & Connect"))
                            .font(theme::medium(13.5))
                            .color(palette.on_accent),
                    )
                    .fill(palette.accent);
                    if ui.add(connect).clicked() {
                        actions.push(Action::SaveConnection { connect: true });
                    }
                    if ui.button(gettext(locale, "Save")).clicked() {
                        actions.push(Action::SaveConnection { connect: false });
                    }
                    if ui.button(gettext(locale, "Cancel")).clicked() {
                        actions.push(Action::CloseDialog);
                    }
                });
            });
        });
    if !open || ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
        actions.push(Action::CloseDialog);
    }
    app.actions.extend(actions);
}
```

In `src/ui/mod.rs`, add `pub mod connect_dialog;` and at the end of `show`:

```rust
    connect_dialog::show(app, &ui.ctx().clone());
```

In `src/ui/keys.rs`, add inside the closure (after the Cmd+Shift shortcuts):

```rust
        key(Modifiers::COMMAND, Key::N, Action::NewConnection);
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test --lib`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src
git commit -m "Add the connection dialog and saved-connection actions"
```

---

### Task 6: The picker list, the workspace top bar, and the disconnected banner

**Files:**
- Modify: `src/ui/picker.rs`, `src/ui/workspace.rs`, `src/ui/conn_tabs.rs`

**Interfaces:**
- Consumes: `App`, `SavedConnections::search`, `ColorTag::color`, `SessionStatus`, `Workspace`.
- Produces: `ui::workspace::TOP_BAR_HEIGHT: f32 = 36.0`; `ui::workspace::show(app, ui, tab)` drawing the top bar and banner (Batch 3 adds the body); picker list rows labelled with the connection name (accessible `Button` role) that connect on double-click or Enter, with Edit, Duplicate, Delete in a context menu and as row buttons.

- [ ] **Step 1: Write the failing UI tests**

Add to the tests in `src/ui/mod.rs`:

```rust
    fn add_saved(harness: &mut Harness, name: &str) -> crate::connections::ConnectionId {
        let saved = crate::connections::SavedConnection {
            id: crate::connections::ConnectionId::new(),
            name: name.into(),
            color: crate::connections::ColorTag::Red,
            spec: tabletist_db::ConnectSpec::sqlite(format!("/tmp/{name}.db")),
        };
        let id = saved.id.clone();
        harness.app.connections.upsert(saved);
        id
    }

    #[test]
    fn the_picker_lists_saved_connections_and_filters_them() {
        let mut harness = Harness::new();
        add_saved(&mut harness, "Production");
        add_saved(&mut harness, "Staging");
        assert!(harness.has("Production"));
        assert!(harness.has("Staging"));
        assert!(!harness.has("No saved connections yet"));
        if let crate::model::ConnTabContent::Picker(picker) = &mut harness.app.tabs[0].content {
            picker.search = "stag".into();
        }
        assert!(!harness.has("Production"));
        assert!(harness.has("Staging"));
    }

    #[test]
    fn the_connect_button_on_a_row_connects_in_this_tab() {
        let mut harness = Harness::new();
        add_saved(&mut harness, "Production");
        harness.click("Connect to Production");
        let tab = harness.app.active_tab_id();
        assert_eq!(harness.app.workspace(tab).unwrap().name, "Production");
        assert!(harness.has("Production"), "the tab is now titled after the connection");
    }

    #[test]
    fn a_disconnected_workspace_offers_reconnect() {
        let mut harness = Harness::new();
        add_saved(&mut harness, "Production");
        harness.click("Connect to Production");
        let tab = harness.app.active_tab_id();
        let session = harness.app.workspace(tab).unwrap().session;
        harness.app.apply(crate::model::Action::Backend(crate::backend::Event::Disconnected {
            session,
            error: tabletist_db::Error::ConnectionLost("server went away".into()),
        }));
        assert!(harness.has("Reconnect"));
        harness.click("Reconnect");
        assert!(matches!(
            harness.app.workspace(tab).unwrap().status,
            crate::model::SessionStatus::Connecting { .. }
        ));
    }

    #[test]
    fn the_top_bar_disconnect_button_returns_to_the_picker() {
        let mut harness = Harness::new();
        add_saved(&mut harness, "Production");
        harness.click("Connect to Production");
        harness.click("Disconnect");
        assert!(matches!(
            harness.app.active_tab().content,
            crate::model::ConnTabContent::Picker(_)
        ));
    }

    #[test]
    fn the_picker_has_a_new_connection_button() {
        let mut harness = Harness::new();
        harness.click("New connection");
        assert!(harness.app.dialog.is_some());
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --lib ui::tests`
Expected: FAIL (no rows, no top bar).

- [ ] **Step 3: Implement the picker list**

Replace `src/ui/picker.rs`:

```rust
//! The body of a picker tab: search, saved connections, New.

use egui::{Align, CornerRadius, Layout, RichText, Sense, TextEdit, vec2};

use crate::app::App;
use crate::connections::ConnectionId;
use crate::i18n::gettext;
use crate::model::{Action, ConnTabContent};
use crate::theme::{self, Icon};
use crate::ui::widgets::icon_button;

const ROW_HEIGHT: f32 = 44.0;
const LIST_WIDTH: f32 = 520.0;

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let locale = app.locale;
    let palette = app.palette;
    let tab = app.active_tab_id();
    let index = app.active;
    ui.vertical_centered(|ui| {
        ui.add_space(48.0);
        ui.set_max_width(LIST_WIDTH);
        ui.label(
            RichText::new(gettext(locale, "Connections"))
                .font(theme::semibold(20.0))
                .color(palette.text),
        );
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            if let ConnTabContent::Picker(picker) = &mut app.tabs[index].content {
                ui.add(
                    TextEdit::singleline(&mut picker.search)
                        .hint_text(gettext(locale, "Search connections"))
                        .desired_width(LIST_WIDTH - 150.0),
                );
            }
            if ui.button(gettext(locale, "New connection")).clicked() {
                app.actions.push(Action::NewConnection);
            }
        });
        ui.add_space(12.0);

        let search = match &app.tabs[index].content {
            ConnTabContent::Picker(picker) => picker.search.clone(),
            ConnTabContent::Workspace(_) => String::new(),
        };
        let rows: Vec<(ConnectionId, String, String, Option<egui::Color32>)> = app
            .connections
            .search(&search)
            .into_iter()
            .map(|saved| (saved.id.clone(), saved.name.clone(), saved.spec.summary(), saved.color.color()))
            .collect();
        if app.connections.connections.is_empty() {
            ui.label(RichText::new(gettext(locale, "No saved connections yet")).color(palette.secondary));
            return;
        }
        if rows.is_empty() {
            ui.label(RichText::new(gettext(locale, "No connections match")).color(palette.secondary));
            return;
        }
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (id, name, summary, color) in rows {
                let (rect, response) = ui.allocate_exact_size(vec2(LIST_WIDTH, ROW_HEIGHT), Sense::click());
                let connect = format!("{} {name}", gettext(locale, "Connect to"));
                response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &name));
                if response.hovered() {
                    ui.painter().rect_filled(rect, CornerRadius::same(theme::RADIUS), palette.surface_hover);
                }
                // Double-click, or Enter/Space/screen-reader activation (egui
                // reports those as a click not made by the pointer).
                let activated = response.double_clicked()
                    || (response.clicked() && !response.clicked_by(egui::PointerButton::Primary));
                if activated {
                    app.actions.push(Action::Connect { tab, conn: id.clone() });
                }
                response.context_menu(|ui| {
                    if ui.button(gettext(locale, "Connect")).clicked() {
                        app.actions.push(Action::Connect { tab, conn: id.clone() });
                    }
                    if ui.button(gettext(locale, "Edit…")).clicked() {
                        app.actions.push(Action::EditConnection(id.clone()));
                    }
                    if ui.button(gettext(locale, "Duplicate")).clicked() {
                        app.actions.push(Action::DuplicateConnection(id.clone()));
                    }
                    if ui.button(gettext(locale, "Delete")).clicked() {
                        app.actions.push(Action::DeleteConnection(id.clone()));
                    }
                });
                let mut row = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(rect.shrink2(vec2(10.0, 0.0)))
                        .layout(Layout::left_to_right(Align::Center)),
                );
                let dot = color.unwrap_or(palette.dim);
                let (dot_rect, _) = row.allocate_exact_size(vec2(10.0, 10.0), Sense::hover());
                row.painter().circle_filled(dot_rect.center(), 4.0, dot);
                row.vertical(|ui| {
                    ui.label(RichText::new(&name).font(theme::medium(13.5)).color(palette.text));
                    ui.label(RichText::new(&summary).font(theme::regular(12.0)).color(palette.secondary));
                });
                row.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let delete = format!("{} {name}", gettext(locale, "Delete"));
                    if icon_button(ui, Icon::Trash2, &delete, &palette).clicked() {
                        app.actions.push(Action::DeleteConnection(id.clone()));
                    }
                    let edit = format!("{} {name}", gettext(locale, "Edit"));
                    if icon_button(ui, Icon::Pencil, &edit, &palette).clicked() {
                        app.actions.push(Action::EditConnection(id.clone()));
                    }
                    if icon_button(ui, Icon::ChevronRight, &connect, &palette).clicked() {
                        app.actions.push(Action::Connect { tab, conn: id.clone() });
                    }
                });
            }
        });
    });
}
```

Row buttons (connect, edit, delete) are drawn after the row's own click area, so they sit on top and receive their clicks.

- [ ] **Step 4: Implement the top bar and banner**

Replace `src/ui/workspace.rs`:

```rust
//! A connection tab: top bar, disconnected banner, and (batch 3) the body.

use egui::{Frame, Margin, RichText};

use crate::app::App;
use crate::i18n::gettext;
use crate::model::{Action, ConnTabId, SessionStatus};
use crate::theme::{self, Icon};
use crate::ui::widgets::icon_button;

pub const TOP_BAR_HEIGHT: f32 = 36.0;

pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
    top_bar(app, ui, tab);
    banner(app, ui, tab);
}

fn top_bar(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
    let locale = app.locale;
    let palette = app.palette;
    let Some(workspace) = app.workspace(tab) else {
        return;
    };
    let name = workspace.name.clone();
    let summary = workspace.spec.summary();
    let color = workspace.color.color().unwrap_or(palette.dim);
    let driver = workspace.driver.label();
    let mut actions = Vec::new();
    egui::Panel::top(egui::Id::new(("workspace-top", tab.0)))
        .exact_size(TOP_BAR_HEIGHT)
        .resizable(false)
        .frame(Frame::new().fill(palette.panel).inner_margin(Margin::symmetric(10, 4)))
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                let (dot, _) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
                ui.painter().circle_filled(dot.center(), 4.0, color);
                ui.label(RichText::new(&name).font(theme::medium(13.5)).color(palette.text));
                ui.label(RichText::new(format!("{driver} · {summary}")).color(palette.secondary));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if icon_button(ui, Icon::LogOut, &gettext(locale, "Disconnect"), &palette).clicked() {
                        actions.push(Action::Disconnect(tab));
                    }
                });
            });
        });
    app.actions.extend(actions);
}

fn banner(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
    let locale = app.locale;
    let palette = app.palette;
    let Some(workspace) = app.workspace(tab) else {
        return;
    };
    let message = match &workspace.status {
        SessionStatus::Connecting { .. } => {
            ui.centered_and_justified(|ui| {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(gettext(locale, "Connecting…"));
                });
            });
            return;
        }
        SessionStatus::Connected => return,
        SessionStatus::Disconnected(error) => error.to_string(),
    };
    let mut reconnect = false;
    Frame::new()
        .fill(palette.danger.gamma_multiply(0.15))
        .inner_margin(Margin::symmetric(12, 8))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!("{}: {message}", gettext(locale, "Disconnected")))
                        .color(palette.text),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    reconnect = ui.button(gettext(locale, "Reconnect")).clicked();
                });
            });
        });
    if reconnect {
        app.actions.push(Action::Reconnect(tab));
    }
}
```

In `src/ui/conn_tabs.rs`, draw the connection's colour dot and status before the title for workspace tabs. In `tab()`, before the label, compute:

```rust
    let status_color = match &app.tabs[index].content {
        ConnTabContent::Picker(_) => None,
        ConnTabContent::Workspace(workspace) => Some(match workspace.status {
            crate::model::SessionStatus::Connected => workspace.color.color().unwrap_or(palette.accent),
            crate::model::SessionStatus::Connecting { .. } => palette.warning,
            crate::model::SessionStatus::Disconnected(_) => palette.danger,
        }),
    };
```

and inside `label_ui`, before adding the label:

```rust
    if let Some(color) = status_color {
        let (dot, _) = label_ui.allocate_exact_size(vec2(8.0, 8.0), Sense::hover());
        label_ui.painter().circle_filled(dot.center(), 3.5, color);
    }
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test --lib`
Expected: PASS. If `has("Production")` fails after connecting because two nodes share the label (tab + top bar), it still passes (`has` is "any"); if `click("Disconnect")` is ambiguous, confirm only the top bar uses that label.

- [ ] **Step 6: Try it**

Run: `cargo run`
Expected: "New connection" opens the dialog; choose any `.db` file (for example, make one with `sqlite3 /tmp/try.db 'create table t(x)'`), Save & Connect; the tab shows the connection's name with a coloured dot and the top bar; Ctrl+T opens another picker tab; the saved connection appears in it; Disconnect returns to the picker; `~/.config/tabletist/connections.json` holds the spec and no password.

- [ ] **Step 7: Full checks and commit**

Run: `cargo fmt --all --check && cargo clippy --locked --all-targets -- -D warnings && cargo test --locked --all-targets`
Expected: all pass.

```bash
git add src
git commit -m "Add the picker list, workspace top bar and disconnected banner"
```

---

## Done when

- Saved SQLite connections can be created, edited, duplicated, deleted, tested, and connected in any connection tab.
- Several connection tabs can be connected at once; closing one closes only its session.
- A dropped or failed connection shows a banner with Reconnect.
- All checks pass.
