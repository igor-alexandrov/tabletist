//! The backend: a tokio runtime on its own thread. The UI sends commands and
//! polls events each frame; the backend wakes the UI when an event is ready.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, mpsc};

use tabletist_db::{
    CancelHandle, ConnectSpec, Connection, Driver, Error, HostKeys, ObjectInfo, ObjectRef, RowPage,
    RowQuery, Secrets, Structure,
};
use tokio::sync::mpsc as tokio_mpsc;

use crate::connections::SavedConnections;
use crate::secrets::{Keyring, SecretString};

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
    Connect {
        session: SessionId,
        request: RequestId,
        spec: ConnectSpec,
        secrets: Secrets,
        /// The SSH host keys the user trusts.
        host_keys: HostKeys,
    },
    /// Connect and close again, for the dialog's Test button.
    Test {
        request: RequestId,
        spec: ConnectSpec,
        secrets: Secrets,
        host_keys: HostKeys,
    },
    Close {
        session: SessionId,
    },
    /// Stops `request`: cancelled if it is running, skipped if it is still
    /// queued. One that already finished is left alone, so a late cancel
    /// never stops the command after it.
    Cancel {
        session: SessionId,
        request: RequestId,
    },
    ListSchemas {
        session: SessionId,
        request: RequestId,
    },
    ListObjects {
        session: SessionId,
        request: RequestId,
        schema: String,
    },
    Describe {
        session: SessionId,
        request: RequestId,
        object: ObjectRef,
    },
    FetchRows {
        session: SessionId,
        request: RequestId,
        query: RowQuery,
    },
    CountRows {
        session: SessionId,
        request: RequestId,
        query: RowQuery,
    },
    /// Read a saved password from the keyring.
    LoadSecret {
        request: RequestId,
        account: String,
    },
    /// Save (`Some`) or delete (`None`) a password in the keyring.
    StoreSecret {
        request: RequestId,
        account: String,
        secret: Option<SecretString>,
    },
    ListDatabases {
        session: SessionId,
        request: RequestId,
    },
    /// Writes a state file atomically. Saves to one file are written in
    /// order, and a burst of them writes only the newest.
    Save {
        path: PathBuf,
        file: StateFile,
    },
    /// Signals `done` once every save sent before it is on disk.
    Flush {
        done: mpsc::Sender<()>,
    },
}

/// The contents of a state file the backend saves.
#[derive(Debug, Clone, PartialEq)]
pub enum StateFile {
    Connections(SavedConnections),
    KnownHosts(HostKeys),
}

impl StateFile {
    fn save(&self, path: &std::path::Path) -> std::io::Result<()> {
        match self {
            Self::Connections(connections) => connections.save(path),
            Self::KnownHosts(keys) => crate::known_hosts::save(path, keys),
        }
    }
}

/// What the backend reports back.
#[derive(Debug)]
pub enum Event {
    Connected {
        session: SessionId,
        request: RequestId,
        driver: Driver,
        /// Whether the session runs over TLS (`prefer` may have fallen back
        /// to plain text).
        encrypted: bool,
    },
    ConnectFailed {
        session: SessionId,
        request: RequestId,
        error: Error,
    },
    Tested {
        request: RequestId,
        result: Result<(), Error>,
    },
    /// The session died; later commands to it fail until a reconnect.
    Disconnected { session: SessionId, error: Error },
    Schemas {
        session: SessionId,
        request: RequestId,
        result: Result<Vec<String>, Error>,
    },
    Objects {
        session: SessionId,
        request: RequestId,
        schema: String,
        result: Result<Vec<ObjectInfo>, Error>,
    },
    Structure {
        session: SessionId,
        request: RequestId,
        result: Result<Structure, Error>,
    },
    Rows {
        session: SessionId,
        request: RequestId,
        result: Result<RowPage, Error>,
    },
    Count {
        session: SessionId,
        request: RequestId,
        result: Result<u64, Error>,
    },
    FilePicked {
        request: RequestId,
        path: Option<PathBuf>,
    },
    /// The Host aliases in ~/.ssh/config, for the connection dialog.
    SshHosts {
        request: RequestId,
        hosts: Vec<tabletist_db::ssh_config::ConfigHost>,
    },
    SecretLoaded {
        request: RequestId,
        result: Result<Option<SecretString>, String>,
    },
    SecretStored {
        request: RequestId,
        result: Result<(), String>,
    },
    Databases {
        session: SessionId,
        request: RequestId,
        result: Result<Vec<String>, Error>,
    },
    /// A `Save` was written, or why it was not.
    Saved {
        path: PathBuf,
        result: Result<(), String>,
    },
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
    /// Starts the backend thread and its runtime with the given keyring: the
    /// OS keyring normally, `Keyring::memory()` in demo mode and tests.
    pub fn start_with(waker: Waker, keyring: Keyring) -> Self {
        let (command_tx, command_rx) = tokio_mpsc::unbounded_channel();
        let (event_tx, event_rx) = mpsc::channel();
        let outbox = Outbox {
            events: event_tx,
            waker,
        };
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
                let (worker, ready) = Worker::new(worker_outbox, keyring);
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
        let Some(commands) = &self.commands else {
            #[cfg(test)]
            self.sent.push(command);
            return;
        };
        if commands.send(command).is_err() {
            log::error!("the backend has stopped");
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

    /// Asks for an SSH private key, starting in ~/.ssh when it exists.
    pub fn pick_key_file(&mut self, request: RequestId) {
        let Some(runtime) = &self.runtime else {
            return;
        };
        let mut dialog = rfd::AsyncFileDialog::new().set_title("Choose an SSH private key");
        if let Some(ssh) = directories::BaseDirs::new()
            .map(|dirs| dirs.home_dir().join(".ssh"))
            .filter(|ssh| ssh.is_dir())
        {
            dialog = dialog.set_directory(ssh);
        }
        let dialog = dialog.pick_file();
        let outbox = self.outbox.clone();
        runtime.spawn(async move {
            let path = dialog.await.map(|file| file.path().to_path_buf());
            outbox.emit(Event::FilePicked { request, path });
        });
    }

    /// Asks for the CA certificate a server's certificate is checked against.
    pub fn pick_ca_file(&mut self, request: RequestId) {
        let Some(runtime) = &self.runtime else {
            return;
        };
        let dialog = rfd::AsyncFileDialog::new()
            .set_title("Choose a CA certificate")
            .add_filter("Certificates", &["pem", "crt", "cer"])
            .add_filter("All files", &["*"])
            .pick_file();
        let outbox = self.outbox.clone();
        runtime.spawn(async move {
            let path = dialog.await.map(|file| file.path().to_path_buf());
            outbox.emit(Event::FilePicked { request, path });
        });
    }

    /// Reads the Host aliases in ~/.ssh/config off the UI thread.
    pub fn list_ssh_hosts(&mut self, request: RequestId) {
        let Some(runtime) = &self.runtime else {
            return;
        };
        let outbox = self.outbox.clone();
        runtime.spawn_blocking(move || {
            let hosts = tabletist_db::ssh_config::hosts();
            outbox.emit(Event::SshHosts { request, hosts });
        });
    }

    /// Waits up to `timeout` for the saves sent so far to reach the disk,
    /// so quitting right after a change keeps it.
    pub fn flush(&mut self, timeout: std::time::Duration) {
        if self.commands.is_none() {
            return;
        }
        let (done, flushed) = mpsc::channel();
        self.send(Command::Flush { done });
        if flushed.recv_timeout(timeout).is_err() {
            log::error!("state files were still being saved at exit");
        }
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
    running: Arc<Mutex<Running>>,
    /// Dropped on Close: the session stops before its next queued command
    /// (a receiver still yields buffered commands after its sender drops).
    _stop: tokio::sync::oneshot::Sender<()>,
}

/// What a session is doing, shared by the worker (which cancels) and the
/// session task (which runs one command at a time).
#[derive(Default)]
struct Running {
    /// The request running now.
    request: Option<RequestId>,
    /// Queued requests to skip when their turn comes.
    skip: HashSet<RequestId>,
    /// Cancels sent for the running request. The session waits for them
    /// before it starts the next command, so none can land on that one.
    cancels: Vec<tokio::task::JoinHandle<()>>,
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// A connect attempt that finished, on its way to the worker. Failures come
/// this way too, so the worker always clears its `connecting` entry.
struct Ready {
    session: SessionId,
    request: RequestId,
    outcome: Result<(Driver, bool, SessionHandle), Error>,
}

/// State files being written. A file is here while its writer runs, with
/// the newest content that writer has not taken yet.
#[derive(Clone, Default)]
struct Saves {
    pending: Arc<Mutex<HashMap<PathBuf, Option<StateFile>>>>,
    /// Woken whenever a writer finishes.
    idle: Arc<tokio::sync::Notify>,
}

impl Saves {
    /// Queues `file` for `path`, starting a writer unless one runs.
    fn save(&self, path: PathBuf, file: StateFile, outbox: &Outbox) {
        use std::collections::hash_map::Entry;
        match lock(&self.pending).entry(path.clone()) {
            // The running writer picks it up after its current write.
            Entry::Occupied(mut next) => *next.get_mut() = Some(file),
            Entry::Vacant(slot) => {
                slot.insert(Some(file));
                let saves = self.clone();
                let outbox = outbox.clone();
                tokio::task::spawn_blocking(move || saves.write(&path, &outbox));
            }
        }
    }

    /// Writes the newest content for `path` until none is left.
    fn write(&self, path: &std::path::Path, outbox: &Outbox) {
        loop {
            let file = {
                let mut pending = lock(&self.pending);
                match pending.get_mut(path).and_then(Option::take) {
                    Some(file) => file,
                    None => {
                        pending.remove(path);
                        break;
                    }
                }
            };
            let result = file.save(path).map_err(|error| error.to_string());
            if let Err(error) = &result {
                log::error!("could not save {}: {error}", path.display());
            }
            outbox.emit(Event::Saved {
                path: path.to_path_buf(),
                result,
            });
        }
        self.idle.notify_waiters();
    }

    /// Resolves once no file is being written.
    async fn flushed(self) {
        loop {
            // Registered before the check, so a writer finishing in between
            // still wakes it.
            let idle = self.idle.notified();
            if lock(&self.pending).is_empty() {
                return;
            }
            idle.await;
        }
    }
}

/// Runs on the backend runtime. Owns every session.
struct Worker {
    outbox: Outbox,
    keyring: Keyring,
    saves: Saves,
    sessions: HashMap<SessionId, SessionHandle>,
    /// Connect tasks hand finished sessions to the worker through this.
    ready: tokio_mpsc::UnboundedSender<Ready>,
    /// Sessions closed before they finished connecting.
    closed_early: std::collections::HashSet<SessionId>,
    connecting: std::collections::HashSet<SessionId>,
}

impl Worker {
    fn new(outbox: Outbox, keyring: Keyring) -> (Self, tokio_mpsc::UnboundedReceiver<Ready>) {
        let (ready, ready_rx) = tokio_mpsc::unbounded_channel();
        let worker = Self {
            outbox,
            keyring,
            saves: Saves::default(),
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
            Ok((driver, encrypted, handle)) => {
                self.sessions.insert(done.session, handle);
                self.outbox.emit(Event::Connected {
                    session: done.session,
                    request: done.request,
                    driver,
                    encrypted,
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
            Command::Connect {
                session,
                request,
                spec,
                secrets,
                host_keys,
            } => {
                self.connecting.insert(session);
                let outbox = self.outbox.clone();
                let ready = self.ready.clone();
                tokio::spawn(async move {
                    let outcome = Connection::connect_with(&spec, &secrets, &host_keys)
                        .await
                        .map(|connection| {
                            let driver = connection.driver();
                            let encrypted = connection.is_encrypted();
                            let cancel = connection.cancel_handle();
                            let (queue, commands) = tokio_mpsc::unbounded_channel();
                            let (stop, stopped) = tokio::sync::oneshot::channel();
                            let running = Arc::new(Mutex::new(Running::default()));
                            tokio::spawn(run_session(
                                session,
                                connection,
                                commands,
                                stopped,
                                Arc::clone(&running),
                                outbox,
                            ));
                            (
                                driver,
                                encrypted,
                                SessionHandle {
                                    queue,
                                    cancel,
                                    running,
                                    _stop: stop,
                                },
                            )
                        });
                    let _ = ready.send(Ready {
                        session,
                        request,
                        outcome,
                    });
                });
            }
            Command::Test {
                request,
                spec,
                secrets,
                host_keys,
            } => {
                let outbox = self.outbox.clone();
                tokio::spawn(async move {
                    let result = match Connection::connect_with(&spec, &secrets, &host_keys).await {
                        Ok(connection) => connection.close().await,
                        Err(error) => Err(error),
                    };
                    outbox.emit(Event::Tested { request, result });
                });
            }
            Command::Close { session } => {
                if let Some(handle) = self.sessions.remove(&session) {
                    let cancel = handle.cancel.clone();
                    // Stop first: the cancel can end the running query on
                    // another thread at once, and the session must already
                    // see the stop then, not start a queued command.
                    drop(handle);
                    tokio::spawn(async move {
                        let _ = cancel.cancel().await;
                    });
                } else if self.connecting.contains(&session) {
                    self.closed_early.insert(session);
                }
            }
            Command::Cancel { session, request } => {
                if let Some(handle) = self.sessions.get(&session) {
                    let mut running = lock(&handle.running);
                    if running.request == Some(request) {
                        let cancel = handle.cancel.clone();
                        running.cancels.retain(|task| !task.is_finished());
                        running.cancels.push(tokio::spawn(async move {
                            let _ = cancel.cancel().await;
                        }));
                    } else {
                        // Queued, or already answered: the session drops
                        // the id once it passes it.
                        running.skip.insert(request);
                    }
                }
            }
            Command::LoadSecret { request, account } => {
                // Queued on the keyring thread now, so it runs after any
                // StoreSecret sent before it.
                let pending = self.keyring.read(&account);
                let outbox = self.outbox.clone();
                tokio::spawn(async move {
                    let result = pending
                        .wait()
                        .await
                        .map(|secret| secret.map(SecretString))
                        .map_err(|error| error.to_string());
                    outbox.emit(Event::SecretLoaded { request, result });
                });
            }
            Command::StoreSecret {
                request,
                account,
                secret,
            } => {
                let pending = match secret {
                    Some(secret) => self.keyring.write(&account, secret),
                    None => self.keyring.delete(&account),
                };
                let outbox = self.outbox.clone();
                tokio::spawn(async move {
                    let result = pending.wait().await.map_err(|error| error.to_string());
                    if let Err(error) = &result {
                        log::warn!("could not update the keyring: {error}");
                    }
                    outbox.emit(Event::SecretStored { request, result });
                });
            }
            Command::Save { path, file } => self.saves.save(path, file, &self.outbox),
            Command::Flush { done } => {
                let saves = self.saves.clone();
                tokio::spawn(async move {
                    saves.flushed().await;
                    let _ = done.send(());
                });
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
        | Command::Cancel { session, .. }
        | Command::ListSchemas { session, .. }
        | Command::ListObjects { session, .. }
        | Command::Describe { session, .. }
        | Command::FetchRows { session, .. }
        | Command::CountRows { session, .. }
        | Command::ListDatabases { session, .. } => *session,
        Command::Test { .. }
        | Command::LoadSecret { .. }
        | Command::StoreSecret { .. }
        | Command::Save { .. }
        | Command::Flush { .. } => SessionId(0),
    }
}

/// The request a queued command answers.
fn request_of(command: &Command) -> Option<RequestId> {
    match command {
        Command::ListSchemas { request, .. }
        | Command::ListObjects { request, .. }
        | Command::Describe { request, .. }
        | Command::FetchRows { request, .. }
        | Command::CountRows { request, .. }
        | Command::ListDatabases { request, .. } => Some(*request),
        Command::Connect { .. }
        | Command::Test { .. }
        | Command::Close { .. }
        | Command::Cancel { .. }
        | Command::LoadSecret { .. }
        | Command::StoreSecret { .. }
        | Command::Save { .. }
        | Command::Flush { .. } => None,
    }
}

/// Reports `error` as the result of `command`.
fn fail(outbox: &Outbox, command: Command, error: Error) {
    let event = match command {
        Command::ListSchemas { session, request } => Event::Schemas {
            session,
            request,
            result: Err(error),
        },
        Command::ListObjects {
            session,
            request,
            schema,
        } => Event::Objects {
            session,
            request,
            schema,
            result: Err(error),
        },
        Command::Describe {
            session, request, ..
        } => Event::Structure {
            session,
            request,
            result: Err(error),
        },
        Command::FetchRows {
            session, request, ..
        } => Event::Rows {
            session,
            request,
            result: Err(error),
        },
        Command::CountRows {
            session, request, ..
        } => Event::Count {
            session,
            request,
            result: Err(error),
        },
        Command::ListDatabases { session, request } => Event::Databases {
            session,
            request,
            result: Err(error),
        },
        Command::Connect { .. }
        | Command::Test { .. }
        | Command::Close { .. }
        | Command::Cancel { .. }
        | Command::LoadSecret { .. }
        | Command::StoreSecret { .. }
        | Command::Save { .. }
        | Command::Flush { .. } => return,
    };
    outbox.emit(event);
}

/// Answers every command still queued for a session whose connection is
/// gone, and closes the queue so later commands fail at the sender.
fn fail_queued(
    commands: &mut tokio_mpsc::UnboundedReceiver<Command>,
    outbox: &Outbox,
    error: &Error,
) {
    commands.close();
    while let Ok(command) = commands.try_recv() {
        fail(outbox, command, error.clone());
    }
}

/// Owns one connection and runs its commands one at a time. Ends (closing the
/// connection) when the worker drops its handle, skipping any commands still
/// queued.
async fn run_session(
    session: SessionId,
    connection: Connection,
    mut commands: tokio_mpsc::UnboundedReceiver<Command>,
    mut stop: tokio::sync::oneshot::Receiver<()>,
    running: Arc<Mutex<Running>>,
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
        // A cancel meant for the previous command may still be on its way:
        // let it land first, so it cannot stop this one.
        let cancels = std::mem::take(&mut lock(&running).cancels);
        for cancel in cancels {
            let _ = cancel.await;
        }
        let request = request_of(&command);
        let skipped = {
            let mut running = lock(&running);
            let skipped = request.is_some_and(|request| running.skip.remove(&request));
            // Requests run in the order they were made, so an id at or
            // before this one will never come again.
            if let Some(request) = request {
                running.skip.retain(|id| id.0 > request.0);
            }
            if !skipped {
                running.request = request;
            }
            skipped
        };
        if skipped {
            fail(&outbox, command, Error::Cancelled);
            continue;
        }
        let lost = match command {
            Command::ListSchemas { session, request } => {
                let result = connection.list_schemas().await;
                let lost = lost_error(&result);
                outbox.emit(Event::Schemas {
                    session,
                    request,
                    result,
                });
                lost
            }
            Command::ListObjects {
                session,
                request,
                schema,
            } => {
                let result = connection.list_objects(&schema).await;
                let lost = lost_error(&result);
                outbox.emit(Event::Objects {
                    session,
                    request,
                    schema,
                    result,
                });
                lost
            }
            Command::Describe {
                session,
                request,
                object,
            } => {
                let result = connection.describe(&object).await;
                let lost = lost_error(&result);
                outbox.emit(Event::Structure {
                    session,
                    request,
                    result,
                });
                lost
            }
            Command::FetchRows {
                session,
                request,
                query,
            } => {
                let result = connection.fetch_rows(&query).await;
                let lost = lost_error(&result);
                outbox.emit(Event::Rows {
                    session,
                    request,
                    result,
                });
                lost
            }
            Command::CountRows {
                session,
                request,
                query,
            } => {
                let result = connection.count_rows(&query).await;
                let lost = lost_error(&result);
                outbox.emit(Event::Count {
                    session,
                    request,
                    result,
                });
                lost
            }
            Command::ListDatabases { session, request } => {
                let result = connection.list_databases().await;
                let lost = lost_error(&result);
                outbox.emit(Event::Databases {
                    session,
                    request,
                    result,
                });
                lost
            }
            Command::Connect { .. }
            | Command::Test { .. }
            | Command::Close { .. }
            | Command::Cancel { .. }
            | Command::LoadSecret { .. }
            | Command::StoreSecret { .. }
            | Command::Save { .. }
            | Command::Flush { .. } => None,
        };
        lock(&running).request = None;
        if let Some(error) = lost {
            fail_queued(&mut commands, &outbox, &error);
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
        let mut backend = Backend::start_with(waker, Keyring::memory());
        let session = SessionId(1);
        backend.send(Command::Connect {
            session,
            request: RequestId(10),
            spec,
            secrets: Secrets::default(),
            host_keys: HostKeys::default(),
        });
        assert!(matches!(
            backend.wait(WAIT),
            Some(Event::Connected {
                session: SessionId(1),
                request: RequestId(10),
                ..
            })
        ));

        backend.send(Command::ListSchemas {
            session,
            request: RequestId(11),
        });
        match backend.wait(WAIT) {
            Some(Event::Schemas {
                request: RequestId(11),
                result: Ok(schemas),
                ..
            }) => {
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
            Some(Event::Rows {
                request: RequestId(12),
                result: Ok(page),
                ..
            }) => {
                assert_eq!(page.rows.len(), 2);
            }
            other => panic!("expected rows, got {other:?}"),
        }
    }

    #[test]
    fn a_failed_connect_reports_its_error() {
        let mut backend = Backend::start_with(Waker::default(), Keyring::memory());
        backend.send(Command::Connect {
            session: SessionId(1),
            request: RequestId(1),
            spec: ConnectSpec::sqlite("/definitely/not/here.db"),
            secrets: Secrets::default(),
            host_keys: HostKeys::default(),
        });
        assert!(matches!(
            backend.wait(WAIT),
            Some(Event::ConnectFailed {
                error: Error::Connect(_),
                ..
            })
        ));
    }

    #[test]
    fn commands_for_an_unknown_session_fail_as_lost() {
        let mut backend = Backend::start_with(Waker::default(), Keyring::memory());
        backend.send(Command::ListSchemas {
            session: SessionId(99),
            request: RequestId(1),
        });
        match backend.wait(WAIT) {
            Some(Event::Schemas {
                result: Err(error), ..
            }) => assert!(error.is_connection_lost()),
            other => panic!("expected a lost-connection error, got {other:?}"),
        }
    }

    #[test]
    fn a_test_connects_and_closes_without_a_session() {
        let (_dir, spec) = fixture();
        let mut backend = Backend::start_with(Waker::default(), Keyring::memory());
        backend.send(Command::Test {
            request: RequestId(5),
            spec,
            secrets: Secrets::default(),
            host_keys: HostKeys::default(),
        });
        assert!(matches!(
            backend.wait(WAIT),
            Some(Event::Tested {
                request: RequestId(5),
                result: Ok(())
            })
        ));
    }

    #[test]
    fn cancel_stops_a_running_query_and_the_session_keeps_working() {
        let (_dir, spec) = fixture();
        let mut backend = Backend::start_with(Waker::default(), Keyring::memory());
        let session = SessionId(1);
        backend.send(Command::Connect {
            session,
            request: RequestId(1),
            spec,
            secrets: Secrets::default(),
            host_keys: HostKeys::default(),
        });
        assert!(matches!(backend.wait(WAIT), Some(Event::Connected { .. })));
        let mut slow = RowQuery::new(ObjectRef::new("main", "big"), 10);
        slow.raw_where = Some("(SELECT count(*) FROM big a, big b) > 0".into());
        backend.send(Command::CountRows {
            session,
            request: RequestId(2),
            query: slow,
        });
        // SQLite ignores an interrupt before the query starts, so keep
        // cancelling until the count answers.
        let deadline = std::time::Instant::now() + WAIT;
        let answer = loop {
            assert!(
                std::time::Instant::now() < deadline,
                "cancel must stop the query"
            );
            backend.send(Command::Cancel {
                session,
                request: RequestId(2),
            });
            if let Some(event) = backend.wait(Duration::from_millis(200)) {
                break event;
            }
        };
        assert!(matches!(
            answer,
            Event::Count {
                request: RequestId(2),
                result: Err(Error::Cancelled),
                ..
            }
        ));
        backend.send(Command::ListSchemas {
            session,
            request: RequestId(3),
        });
        assert!(matches!(
            backend.wait(WAIT),
            Some(Event::Schemas {
                request: RequestId(3),
                result: Ok(_),
                ..
            })
        ));
    }

    fn connected(spec: ConnectSpec) -> (Backend, SessionId) {
        let mut backend = Backend::start_with(Waker::default(), Keyring::memory());
        let session = SessionId(1);
        backend.send(Command::Connect {
            session,
            request: RequestId(1),
            spec,
            secrets: Secrets::default(),
            host_keys: HostKeys::default(),
        });
        assert!(matches!(backend.wait(WAIT), Some(Event::Connected { .. })));
        (backend, session)
    }

    /// Counts `big` against itself `rows` times over: slow enough to cancel.
    fn slow_count(rows: u64) -> RowQuery {
        let mut slow = RowQuery::new(ObjectRef::new("main", "big"), 10);
        slow.raw_where = Some(format!(
            "(SELECT count(*) FROM big a, big b WHERE b.id <= {rows}) > 0"
        ));
        slow
    }

    #[test]
    fn a_cancelled_queued_request_never_runs() {
        let (_dir, spec) = fixture();
        let (mut backend, session) = connected(spec);
        backend.send(Command::CountRows {
            session,
            request: RequestId(2),
            query: slow_count(100_000),
        });
        backend.send(Command::ListSchemas {
            session,
            request: RequestId(3),
        });
        // Superseded while the count still runs.
        backend.send(Command::Cancel {
            session,
            request: RequestId(3),
        });
        let deadline = std::time::Instant::now() + WAIT;
        let mut events = Vec::new();
        while events.len() < 2 {
            assert!(std::time::Instant::now() < deadline, "both must answer");
            backend.send(Command::Cancel {
                session,
                request: RequestId(2),
            });
            events.extend(backend.wait(Duration::from_millis(200)));
        }
        assert!(matches!(
            events.as_slice(),
            [
                Event::Count {
                    request: RequestId(2),
                    result: Err(Error::Cancelled),
                    ..
                },
                // Answered as cancelled, never listed.
                Event::Schemas {
                    request: RequestId(3),
                    result: Err(Error::Cancelled),
                    ..
                },
            ]
        ));
        backend.send(Command::ListSchemas {
            session,
            request: RequestId(4),
        });
        assert!(matches!(
            backend.wait(WAIT),
            Some(Event::Schemas {
                request: RequestId(4),
                result: Ok(_),
                ..
            })
        ));
    }

    #[test]
    fn a_cancel_for_a_finished_request_leaves_the_next_one_alone() {
        let (_dir, spec) = fixture();
        let (mut backend, session) = connected(spec);
        backend.send(Command::ListSchemas {
            session,
            request: RequestId(2),
        });
        assert!(matches!(
            backend.wait(WAIT),
            Some(Event::Schemas { result: Ok(_), .. })
        ));
        backend.send(Command::CountRows {
            session,
            request: RequestId(3),
            query: slow_count(40),
        });
        // Late cancels for the finished listing arrive while the count runs;
        // a cancel for the whole session would stop it.
        let deadline = std::time::Instant::now() + WAIT;
        let answer = loop {
            assert!(
                std::time::Instant::now() < deadline,
                "the count must finish"
            );
            backend.send(Command::Cancel {
                session,
                request: RequestId(2),
            });
            if let Some(event) = backend.wait(Duration::from_millis(20)) {
                break event;
            }
        };
        assert!(
            matches!(
                answer,
                Event::Count {
                    request: RequestId(3),
                    result: Ok(_),
                    ..
                }
            ),
            "{answer:?}"
        );
    }

    fn named(name: &str) -> StateFile {
        let mut connections = SavedConnections::default();
        connections.upsert(crate::connections::SavedConnection {
            id: crate::connections::ConnectionId::new(),
            name: name.into(),
            environment: crate::env::Environment::None,
            read_only: None,
            password: crate::connections::PasswordMode::None,
            ssh_secret: crate::connections::PasswordMode::None,
            spec: ConnectSpec::sqlite("/tmp/a.db"),
        });
        StateFile::Connections(connections)
    }

    #[test]
    fn a_burst_of_saves_leaves_the_newest_on_disk() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config").join("connections.json");
        let mut backend = Backend::start_with(Waker::default(), Keyring::memory());
        for n in 0..50 {
            backend.send(Command::Save {
                path: path.clone(),
                file: named(&n.to_string()),
            });
        }
        backend.flush(WAIT);
        let stored = SavedConnections::load(&path);
        assert_eq!(stored.connections[0].name, "49");
        let mut saved = 0;
        while let Some(Event::Saved { result, .. }) = backend.wait(Duration::from_millis(100)) {
            assert_eq!(result, Ok(()));
            saved += 1;
        }
        assert!((1..=50).contains(&saved), "{saved}");
    }

    #[test]
    fn a_failed_save_is_reported() {
        let dir = tempfile::tempdir().unwrap();
        let blocker = dir.path().join("config");
        std::fs::write(&blocker, "a file, not a folder").unwrap();
        let path = blocker.join("known_hosts.json");
        let mut backend = Backend::start_with(Waker::default(), Keyring::memory());
        backend.send(Command::Save {
            path: path.clone(),
            file: StateFile::KnownHosts(HostKeys::default()),
        });
        match backend.wait(WAIT) {
            Some(Event::Saved { path: at, result }) => {
                assert_eq!(at, path);
                assert!(result.is_err());
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn closing_a_session_makes_later_commands_fail() {
        let (_dir, spec) = fixture();
        let mut backend = Backend::start_with(Waker::default(), Keyring::memory());
        let session = SessionId(1);
        backend.send(Command::Connect {
            session,
            request: RequestId(1),
            spec,
            secrets: Secrets::default(),
            host_keys: HostKeys::default(),
        });
        assert!(matches!(backend.wait(WAIT), Some(Event::Connected { .. })));
        backend.send(Command::Close { session });
        backend.send(Command::ListSchemas {
            session,
            request: RequestId(2),
        });
        match backend.wait(WAIT) {
            Some(Event::Schemas {
                result: Err(error), ..
            }) => assert!(error.is_connection_lost()),
            other => panic!("expected a lost-connection error, got {other:?}"),
        }
    }

    #[test]
    fn closing_a_session_skips_commands_still_queued() {
        let (_dir, spec) = fixture();
        let mut backend = Backend::start_with(Waker::default(), Keyring::memory());
        let session = SessionId(1);
        backend.send(Command::Connect {
            session,
            request: RequestId(1),
            spec,
            secrets: Secrets::default(),
            host_keys: HostKeys::default(),
        });
        assert!(matches!(backend.wait(WAIT), Some(Event::Connected { .. })));
        let mut slow = RowQuery::new(ObjectRef::new("main", "big"), 10);
        slow.raw_where = Some("(SELECT count(*) FROM big a, big b) > 0".into());
        backend.send(Command::CountRows {
            session,
            request: RequestId(2),
            query: slow,
        });
        for request in 3..10 {
            backend.send(Command::ListSchemas {
                session,
                request: RequestId(request),
            });
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
    fn a_lost_connection_answers_every_queued_command() {
        let (sender, mut commands) = tokio_mpsc::unbounded_channel();
        let (events, received) = mpsc::channel();
        let outbox = Outbox {
            events,
            waker: Waker::default(),
        };
        let session = SessionId(1);
        sender
            .send(Command::ListSchemas {
                session,
                request: RequestId(2),
            })
            .unwrap();
        sender
            .send(Command::CountRows {
                session,
                request: RequestId(3),
                query: RowQuery::new(ObjectRef::new("main", "users"), 10),
            })
            .unwrap();
        fail_queued(
            &mut commands,
            &outbox,
            &Error::ConnectionLost("reset".into()),
        );
        let answered: Vec<Event> = received.try_iter().collect();
        assert!(matches!(
            answered.as_slice(),
            [
                Event::Schemas {
                    request: RequestId(2),
                    result: Err(Error::ConnectionLost(_)),
                    ..
                },
                Event::Count {
                    request: RequestId(3),
                    result: Err(Error::ConnectionLost(_)),
                    ..
                },
            ]
        ));
        // Later commands fail at the sender, so the worker answers them too.
        assert!(
            sender
                .send(Command::ListSchemas {
                    session,
                    request: RequestId(4)
                })
                .is_err()
        );
    }

    #[test]
    fn the_recording_backend_keeps_commands_and_returns_injected_events() {
        let mut backend = Backend::recording();
        backend.send(Command::Close {
            session: SessionId(3),
        });
        assert!(matches!(
            backend.sent.as_slice(),
            [Command::Close {
                session: SessionId(3)
            }]
        ));
        backend.inject(Event::Disconnected {
            session: SessionId(3),
            error: Error::Cancelled,
        });
        assert_eq!(backend.poll().len(), 1);
        assert!(backend.poll().is_empty());
    }

    #[test]
    fn commands_do_not_print_secrets() {
        let command = Command::Test {
            request: RequestId(1),
            spec: ConnectSpec::sqlite("/a.db"),
            secrets: Secrets {
                password: Some("hunter2".into()),
                ..Secrets::default()
            },
            host_keys: HostKeys::default(),
        };
        assert!(!format!("{command:?}").contains("hunter2"));
    }

    #[test]
    fn secrets_are_stored_loaded_and_deleted_in_order() {
        use crate::secrets::{Keyring, SecretString};
        let mut backend = Backend::start_with(Waker::default(), Keyring::memory());
        backend.send(Command::StoreSecret {
            request: RequestId(1),
            account: "a".into(),
            secret: Some(SecretString("pw".into())),
        });
        backend.send(Command::LoadSecret {
            request: RequestId(2),
            account: "a".into(),
        });
        backend.send(Command::StoreSecret {
            request: RequestId(3),
            account: "a".into(),
            secret: None,
        });
        backend.send(Command::LoadSecret {
            request: RequestId(4),
            account: "a".into(),
        });
        let mut loaded = std::collections::HashMap::new();
        for _ in 0..4 {
            match backend.wait(WAIT) {
                Some(Event::SecretLoaded { request, result }) => {
                    loaded.insert(request.0, result);
                }
                Some(Event::SecretStored { result, .. }) => assert_eq!(result, Ok(())),
                other => panic!("{other:?}"),
            }
        }
        assert_eq!(loaded[&2], Ok(Some(SecretString("pw".into()))));
        assert_eq!(loaded[&4], Ok(None));
    }

    #[test]
    fn store_commands_do_not_print_secrets() {
        let command = Command::StoreSecret {
            request: RequestId(1),
            account: "a".into(),
            secret: Some(crate::secrets::SecretString("hunter2".into())),
        };
        assert!(!format!("{command:?}").contains("hunter2"));
    }

    #[test]
    fn sqlite_sessions_list_no_databases() {
        let (_dir, spec) = fixture();
        let mut backend = Backend::start_with(Waker::default(), Keyring::memory());
        let session = SessionId(1);
        backend.send(Command::Connect {
            session,
            request: RequestId(1),
            spec,
            secrets: Secrets::default(),
            host_keys: HostKeys::default(),
        });
        assert!(matches!(backend.wait(WAIT), Some(Event::Connected { .. })));
        backend.send(Command::ListDatabases {
            session,
            request: RequestId(2),
        });
        assert!(matches!(
            backend.wait(WAIT),
            Some(Event::Databases { request: RequestId(2), result: Ok(databases), .. }) if databases.is_empty()
        ));
    }

    #[test]
    fn listing_ssh_hosts_answers_the_request() {
        let mut backend = Backend::start_with(Waker::default(), Keyring::memory());
        backend.list_ssh_hosts(RequestId(9));
        match backend.wait(Duration::from_secs(5)) {
            Some(Event::SshHosts { request, .. }) => assert_eq!(request, RequestId(9)),
            other => panic!("{other:?}"),
        }
    }
}
