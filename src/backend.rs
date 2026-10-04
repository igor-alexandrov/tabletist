//! The backend: a tokio runtime on its own thread. The UI sends commands and
//! polls events each frame; the backend wakes the UI when an event is ready.

use std::collections::{HashMap, HashSet};
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, mpsc};
use std::time::Duration;

use tabletist_db::{
    CancelHandle, ConnectSpec, Connection, Driver, Error, HostKeys, ObjectInfo, ObjectRef, RowPage,
    RowQuery, ScriptOutcome, Secrets, StopFlag, Structure,
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

/// What the UI asks the backend to do. `Secrets` prints as `Secrets { .. }`,
/// and a script's statements print without their text.
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
    /// Run a SQL editor script (see `Connection::run_script`). A timeout
    /// stops it as a user's Cancel does; the script still runs to its end,
    /// rolling back.
    RunSql {
        session: SessionId,
        request: RequestId,
        statements: Vec<tabletist_db::sql::Statement>,
        /// The most rows kept per statement.
        limit: u32,
        /// How long the script may run before it is stopped; `None` lets
        /// it run until it ends or the user cancels it.
        timeout: Option<Duration>,
    },
    /// The server's name and version, for the SQL editor's footer.
    ServerVersion {
        session: SessionId,
        request: RequestId,
    },
    /// Writes a state file atomically. Saves to one file are written in
    /// order, and a burst of them writes only the newest.
    Save {
        path: PathBuf,
        file: StateFile,
    },
    /// Watches the settings file `path` for changes made outside the app,
    /// behind a symbolic link too. Answered with [`Event::SettingsWatch`];
    /// each change is then an [`Event::SettingsFile`].
    WatchSettings {
        path: PathBuf,
    },
    /// Opens the settings file `path` in the editor, written from `text`
    /// first when it is not there. Answered with
    /// [`Event::SettingsFileOpened`].
    EditSettingsFile {
        path: PathBuf,
        text: String,
    },
    /// Shows the settings file `path` in the file manager, written from
    /// `text` first when it is not there. Answered with
    /// [`Event::SettingsFileOpened`].
    RevealSettingsFile {
        path: PathBuf,
        text: String,
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
    Settings(crate::settings::Settings),
}

impl StateFile {
    fn save(&self, path: &std::path::Path) -> std::io::Result<()> {
        match self {
            Self::Connections(connections) => connections.save(path),
            Self::KnownHosts(keys) => crate::known_hosts::save(path, keys),
            Self::Settings(settings) => settings.save(path),
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
    /// A `RunSql` ended. One stopped before any statement ran, whether it
    /// had started or was still queued, is `Ok` with no results and
    /// `stopped` set.
    SqlRan {
        session: SessionId,
        request: RequestId,
        result: Result<ScriptOutcome, Error>,
        /// Who stopped the run, when a stop ended it.
        cancel: Option<CancelReason>,
    },
    ServerVersion {
        session: SessionId,
        request: RequestId,
        result: Result<String, Error>,
    },
    /// A `Save` was written, or why it was not.
    Saved {
        path: PathBuf,
        result: Result<(), String>,
    },
    /// Whether the settings file is being watched, wherever an edit of it
    /// can be made. Sent again when that changes: a symbolic link turned to
    /// a directory that cannot be watched, or such a directory that can be
    /// watched after all.
    SettingsWatch { live: bool },
    /// The settings file changed on disk: its text. `own` when it is, byte
    /// for byte, what the backend itself wrote there last: the app's own
    /// save coming back, which it must not take for someone else's change.
    SettingsFile { text: String, own: bool },
    /// A program was started on the settings file, or why it was not.
    SettingsFileOpened {
        with: Opened,
        result: Result<(), String>,
    },
}

/// What the settings file was handed to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Opened {
    /// The editor.
    Editor,
    /// The file manager, to show where the file is.
    Folder,
}

/// Who stopped a SQL editor run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CancelReason {
    User,
    /// The run's timeout, which was this long.
    Timeout(Duration),
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
    /// The names and sizes of the values a test asked to save.
    #[cfg(test)]
    pub saves: Vec<(String, usize)>,
    #[cfg(test)]
    watched: Watched,
}

/// What every session of a backend is doing, for a test to look at: one
/// that cancels or closes a session first waits until it runs the request
/// the test means.
#[cfg(test)]
type Watched = Arc<Mutex<HashMap<SessionId, Arc<Mutex<Running>>>>>;

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
        #[cfg(test)]
        let watched = Watched::default();
        #[cfg(test)]
        let worker_watched = Arc::clone(&watched);
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
                #[cfg(test)]
                let worker = Worker {
                    watched: worker_watched,
                    ..worker
                };
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
            #[cfg(test)]
            saves: Vec::new(),
            #[cfg(test)]
            watched,
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
            #[cfg(test)]
            saves: Vec::new(),
            #[cfg(test)]
            watched: Watched::default(),
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

    /// Asks where to save `bytes` (a binary value), suggesting `name`, and
    /// writes them there off the UI thread. A dialog closed without a
    /// choice saves nothing and says nothing; a write that fails is told as
    /// any failed save is.
    pub fn save_bytes(&mut self, name: String, bytes: Vec<u8>) {
        #[cfg(test)]
        self.saves.push((name.clone(), bytes.len()));
        let Some(runtime) = &self.runtime else {
            return;
        };
        let dialog = rfd::AsyncFileDialog::new()
            .set_title("Save value")
            .set_file_name(name)
            .save_file();
        let outbox = self.outbox.clone();
        runtime.spawn(async move {
            let Some(file) = dialog.await else {
                return;
            };
            let path = file.path().to_path_buf();
            let target = path.clone();
            let written = tokio::task::spawn_blocking(move || std::fs::write(target, bytes)).await;
            let result = match written {
                Ok(result) => result.map_err(|error| error.to_string()),
                Err(error) => Err(error.to_string()),
            };
            if let Err(error) = &result {
                log::error!("could not save {}: {error}", path.display());
            }
            outbox.emit(Event::Saved { path, result });
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

    /// The request `session` runs now, and whether it is a script (one
    /// with a stop flag).
    #[cfg(test)]
    fn running(&self, session: SessionId) -> Option<(RequestId, bool)> {
        let watched = lock(&self.watched);
        let running = lock(watched.get(&session)?);
        Some((running.request?, running.stop.is_some()))
    }

    /// Whether cancels are being sent for the script `session` runs.
    #[cfg(test)]
    fn cancelling(&self, session: SessionId) -> bool {
        let watched = lock(&self.watched);
        watched
            .get(&session)
            .is_some_and(|running| lock(running).cancelling)
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
    /// The running SQL editor script's stop flag: a Cancel and a Close set
    /// it too, so the script also stops between statements.
    stop: Option<StopFlag>,
    /// Cancels are being sent for the running script, again and again
    /// (see `keep_cancelling`). Whoever stops the script first sends them:
    /// a second Cancel, a timeout or a Close after it has nothing to add.
    cancelling: bool,
    /// The worker closed the session: it starts no further command, not
    /// even one it has already taken off its queue.
    closed: bool,
    /// The request at which a test has the session's task panic.
    #[cfg(test)]
    panics: Option<RequestId>,
}

impl Running {
    /// Tells the running script, if there is one, to stop: it runs no
    /// further statement, whether or not a cancel reaches the one running.
    fn stop_script(&self) {
        if let Some(stop) = &self.stop {
            stop.stop();
        }
    }

    /// Closes the session: the running script stops, and no command starts
    /// after this.
    fn close(&mut self) {
        self.closed = true;
        self.stop_script();
    }

    /// Whether a script is running and a cancel can still reach one of its
    /// statements. Not once it has begun its cleanup: there a cancel would
    /// only interrupt the rollback.
    fn script_takes_a_cancel(&self) -> bool {
        self.stop.as_ref().is_some_and(|stop| !stop.is_finishing())
    }

    /// Whether the caller is to keep cancelling the running script: when a
    /// cancel can still reach it and nobody sends them yet. The caller
    /// that hears yes sends them.
    fn start_cancelling(&mut self) -> bool {
        let first = !self.cancelling && self.script_takes_a_cancel();
        self.cancelling |= first;
        first
    }
}

/// Marks a session's task as over when it ends, however it ends, so that
/// nothing goes on cancelling a script for it.
///
/// A task that ends before its loop does (it panicked, or was dropped) has
/// a connection nobody runs any more. The command it was running and the
/// ones still queued are answered as lost and the session is reported
/// disconnected, so the tab offers a reconnect and no request waits for an
/// answer that never comes.
struct SessionEnd {
    session: SessionId,
    running: Arc<Mutex<Running>>,
    commands: tokio_mpsc::UnboundedReceiver<Command>,
    outbox: Outbox,
    /// The command taken off the queue, until it has answered.
    command: Option<Command>,
    /// The task reached the end of its loop: it has answered all it will.
    finished: bool,
}

impl SessionEnd {
    fn new(
        session: SessionId,
        running: Arc<Mutex<Running>>,
        commands: tokio_mpsc::UnboundedReceiver<Command>,
        outbox: Outbox,
    ) -> Self {
        Self {
            session,
            running,
            commands,
            outbox,
            command: None,
            finished: false,
        }
    }
}

impl Drop for SessionEnd {
    fn drop(&mut self) {
        {
            let mut running = lock(&self.running);
            running.request = None;
            running.stop = None;
            running.cancelling = false;
            running.closed = true;
        }
        // The worker let go of the session (a Close, or the backend is
        // shutting down): nobody waits for it, as when it ends in order.
        if self.finished || self.commands.is_closed() {
            return;
        }
        log::error!("session {} stopped unexpectedly", self.session.0);
        let error = Error::ConnectionLost("the session stopped unexpectedly".into());
        if let Some(command) = self.command.take() {
            fail(&self.outbox, command, error.clone());
        }
        fail_queued(&mut self.commands, &self.outbox, &error);
        self.outbox.emit(Event::Disconnected {
            session: self.session,
            error,
        });
    }
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
    settings_written: Written,
}

/// The text the backend wrote to the settings file last, if it wrote one.
/// Locked while it is written and while the reader reads the file and
/// compares: a write cannot land between the reader's read and its
/// comparison and make the app's own older text look like someone else's.
type Written = Arc<Mutex<Option<String>>>;

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
            let result = match &file {
                // Known to the settings file's reader for the app's own.
                StateFile::Settings(settings) => {
                    let mut written = lock(&self.settings_written);
                    let result = file.save(path);
                    // After a write that failed nothing on the disk is
                    // known to be the app's: the text there may be an
                    // older write of its own, but the app has moved on
                    // from it, and someone putting it back must be heard.
                    *written = result.is_ok().then(|| settings.to_toml());
                    result
                }
                _ => file.save(path),
            }
            .map_err(|error| error.to_string());
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

/// How long the settings file must have been left alone before it is read:
/// a save is several changes (a temporary file, a rename, a truncate and a
/// write), and a read between two of them finds the file half-written.
const SETTLE: Duration = Duration::from_millis(100);

/// The watch on the settings file, held by the task that reads it. The
/// directory of the file is always watched. The directory and not the
/// file: an editor saves by renaming another file over it, and a watch on
/// the file would stay with the one that was replaced. When the file is a
/// symbolic link, as a dotfiles manager makes it, the directory of the file
/// it leads to is watched too: an edit made there raises no event beside
/// the link. So is the directory of each link on the way, when one link
/// leads to another: any of them can be turned.
struct SettingsWatcher {
    watcher: notify::RecommendedWatcher,
    /// The settings file, as the app names it.
    path: PathBuf,
    /// The names a change of the settings comes under: the file's own and,
    /// behind a link, the name of the file it leads to and of every link
    /// on the way there. The watcher's callback reads them.
    names: Arc<Mutex<Vec<OsString>>>,
    /// The directories watched for a link's sake, while there are any.
    elsewhere: Vec<PathBuf>,
    /// Every directory watched, as the watcher names it in an event. The
    /// callback reads them, to tell when one is moved or removed.
    watched: Arc<Mutex<Vec<PathBuf>>>,
    /// Set by the callback when a watched directory was moved or removed:
    /// a watch stays with the directory it was put on, not with its path,
    /// so one made anew at that path is not watched until it is asked for
    /// again ([`Self::renew`]).
    gone: Arc<AtomicBool>,
}

/// What lies behind a symbolic link at the settings file: the names of what
/// the links lead through, and the directories those are in.
type Behind = (Vec<OsString>, Vec<PathBuf>);

/// How many times in a row the links may be found turned since they were
/// last looked at before the watcher gives up for now.
const LOOKS: usize = 3;

impl SettingsWatcher {
    /// Looks where the settings file is now and watches there. A link can
    /// be made, turned elsewhere or replaced by a file while the app runs.
    /// An error, with what to log, when a link leads where nothing can be
    /// watched: an edit made there would not be seen.
    fn follow(&mut self) -> Result<(), String> {
        self.follow_with(behind_link)
    }

    /// [`Self::follow`], with `look` for the look at the links: the tests
    /// turn a link between that look and the watch that comes of it.
    fn follow_with(
        &mut self,
        mut look: impl FnMut(&Path) -> std::io::Result<Behind>,
    ) -> Result<(), String> {
        self.renew()?;
        // A link in a directory that is not watched yet can be turned
        // between the look at it and the watch on its directory, and no
        // event would say so. So once the watches are there the links are
        // looked at again, until they are as they were when watched.
        let mut watched = self.watch_behind(look(&self.path));
        for _ in 0..LOOKS {
            let again = self.watch_behind(look(&self.path));
            if again == watched {
                return watched.map(|_| ());
            }
            watched = again;
        }
        Err(format!(
            "the link at {} keeps being turned",
            self.path.display()
        ))
    }

    /// Watches every directory anew once one of them was moved or removed
    /// ([`Self::gone`]): the settings file's own here, and the ones behind
    /// a link when [`Self::watch_behind`] finds none kept. An error when
    /// the file's own directory is not there to be watched, and then the
    /// next look tries again.
    fn renew(&mut self) -> Result<(), String> {
        use notify::Watcher as _;
        if !self.gone.swap(false, Ordering::SeqCst) {
            return Ok(());
        }
        // Best effort: the watcher may have let go of a directory that was
        // removed by itself.
        for directory in self.elsewhere.drain(..) {
            let _ = self.watcher.unwatch(&directory);
        }
        let beside = crate::util::directory_of(&self.path);
        let _ = self.watcher.unwatch(beside);
        let renewed = self
            .watcher
            .watch(beside, notify::RecursiveMode::NonRecursive);
        self.tell_watched();
        renewed.map_err(|error| {
            self.gone.store(true, Ordering::SeqCst);
            format!("could not watch {}: {error}", beside.display())
        })
    }

    /// Tells the callback which directories are watched now.
    fn tell_watched(&self) {
        let beside = crate::util::directory_of(&self.path);
        let named = |directory: &Path| {
            std::path::absolute(directory).unwrap_or_else(|_| directory.to_path_buf())
        };
        let elsewhere = self.elsewhere.iter().map(|directory| named(directory));
        *lock(&self.watched) = std::iter::once(named(beside)).chain(elsewhere).collect();
    }

    /// Watches where `behind` says the links lead, and no longer where
    /// they led before. Gives back what it was given, or why that cannot
    /// be watched.
    fn watch_behind(&mut self, behind: std::io::Result<Behind>) -> Result<Behind, String> {
        use notify::Watcher as _;
        let behind = behind.map_err(|error| {
            format!(
                "could not follow the link at {}: {error}",
                self.path.display()
            )
        });
        let (names, directories) = behind.clone().unwrap_or_default();
        let own = self.path.file_name().map(OsStr::to_owned);
        *lock(&self.names) = own.into_iter().chain(names).collect();
        // Let go of where the links no longer lead. Best effort: a
        // directory that is gone is not watched any more as it is.
        let watcher = &mut self.watcher;
        self.elsewhere.retain(|watched| {
            let kept = directories.contains(watched);
            if !kept {
                let _ = watcher.unwatch(watched);
            }
            kept
        });
        let mut failed = None;
        for directory in directories {
            if self.elsewhere.contains(&directory) {
                continue;
            }
            match watcher.watch(&directory, notify::RecursiveMode::NonRecursive) {
                Ok(()) => self.elsewhere.push(directory),
                Err(error) => {
                    failed = Some(format!("could not watch {}: {error}", directory.display()));
                    break;
                }
            }
        }
        self.tell_watched();
        match failed {
            Some(why) => Err(why),
            None => behind,
        }
    }
}

/// Where the settings file is when `path` is a symbolic link: the names of
/// what the links lead through (each further link, and the file at the
/// end), and the directories those are in, other than the link's own.
/// Nothing for a path that is no link.
fn behind_link(path: &Path) -> std::io::Result<Behind> {
    let chain = crate::util::link_chain(path)?;
    let (mut names, mut directories) = (Vec::new(), Vec::new());
    let Some(behind) = chain.get(1..).filter(|behind| !behind.is_empty()) else {
        return Ok((names, directories));
    };
    // As the system has them: one directory reached by two paths is told
    // for one, and is not watched twice.
    let beside = std::fs::canonicalize(crate::util::directory_of(path))?;
    for file in behind {
        // A link to a directory's `..` names nothing that could be read.
        let Some(name) = file.file_name() else {
            continue;
        };
        names.push(name.to_owned());
        let directory = std::fs::canonicalize(crate::util::directory_of(file))?;
        if directory != beside && !directories.contains(&directory) {
            directories.push(directory);
        }
    }
    Ok((names, directories))
}

/// The task that watches and reads the settings file. Dropped, it stops
/// the task, and the watcher the task holds goes with it.
struct SettingsReader(tokio::task::AbortHandle);

impl Drop for SettingsReader {
    fn drop(&mut self) {
        self.0.abort();
    }
}

/// Watches the settings file `path` and starts the task that reads it
/// whenever it changes, and once at the start. Says too whether every
/// place an edit can be made is watched (see [`SettingsWatcher::follow`]).
fn watch_settings(
    path: PathBuf,
    outbox: Outbox,
    written: Written,
) -> notify::Result<(SettingsReader, bool)> {
    use notify::Watcher as _;
    let (changed, changes) = tokio_mpsc::unbounded_channel();
    // The file as it is now: it was loaded before there was a watch, and a
    // save made in between raised no event. The app drops a text it holds
    // already.
    let _ = changed.send(());
    let names = Arc::new(Mutex::new(Vec::new()));
    let known = Arc::clone(&names);
    let watched = Arc::new(Mutex::new(Vec::new()));
    let directories = Arc::clone(&watched);
    let gone = Arc::new(AtomicBool::new(false));
    let left = Arc::clone(&gone);
    let mut watcher =
        notify::recommended_watcher(move |event: notify::Result<notify::Event>| match event {
            Ok(event) => {
                // A directory made anew where this one was is watched once
                // the reader has looked: it is woken for that too.
                let moved = leaves(&event, &lock(&directories));
                if moved {
                    left.store(true, Ordering::SeqCst);
                }
                if moved || concerns(&event, &lock(&known)) {
                    let _ = changed.send(());
                }
            }
            Err(error) => log::warn!("watching the settings file: {error}"),
        })?;
    watcher.watch(
        crate::util::directory_of(&path),
        notify::RecursiveMode::NonRecursive,
    )?;
    let mut watch = SettingsWatcher {
        watcher,
        path: path.clone(),
        names,
        elsewhere: Vec::new(),
        watched,
        gone,
    };
    let live = match watch.follow() {
        Ok(()) => true,
        Err(why) => {
            log::warn!("{why}");
            false
        }
    };
    // The reader holds the watcher from here on, to turn it with the link.
    let follow = move || tokio::task::block_in_place(|| watch.follow());
    let read = move |path| read_file(path, Arc::clone(&written));
    let reader = tokio::spawn(read_settings(path, live, changes, outbox, follow, read));
    Ok((SettingsReader(reader.abort_handle()), live))
}

/// The settings file as the reader found it.
struct Found {
    bytes: Vec<u8>,
    /// It is what the backend wrote there last.
    own: bool,
}

/// Reads `path` on the blocking pool: a disk can be slow, and the runtime's
/// threads serve every session. `written` is held across the read and the
/// comparison, so that no write of the backend's lands between the two.
async fn read_file(path: PathBuf, written: Written) -> std::io::Result<Found> {
    let read = move || {
        let written = lock(&written);
        let bytes = std::fs::read(path)?;
        let own = written
            .as_deref()
            .is_some_and(|text| text.as_bytes() == bytes.as_slice());
        Ok(Found { bytes, own })
    };
    match tokio::task::spawn_blocking(read).await {
        Ok(found) => found,
        Err(error) => Err(std::io::Error::other(error)),
    }
}

/// Whether `event` says a file named as one of `names` may have changed.
/// Not when it was only looked at: Linux reports every open, and the reader
/// below opens the file, so answering those would have it read again for
/// ever. The file is known by its name: the paths come as the system has
/// them, which is not always as the directory was given.
fn concerns(event: &notify::Event, names: &[OsString]) -> bool {
    !event.kind.is_access()
        && event
            .paths
            .iter()
            .filter_map(|path| path.file_name())
            .any(|changed| names.iter().any(|name| same_file(changed, name)))
}

/// Whether `event` says that one of the watched `directories` is no longer
/// where it was: moved, or removed. The watch on it did not stay with its
/// path, and what is at that path now, or comes to be, is not watched.
fn leaves(event: &notify::Event, directories: &[PathBuf]) -> bool {
    use notify::EventKind;
    use notify::event::ModifyKind;
    let leaving = matches!(
        event.kind,
        EventKind::Remove(_) | EventKind::Modify(ModifyKind::Name(_))
    );
    leaving && event.paths.iter().any(|path| directories.contains(path))
}

/// Whether two file names are one file's. macOS and Windows find a file
/// whatever the case of its name, so one kept as `Settings.toml` is the
/// one the app reads as `settings.toml`, and its changes come under the
/// spelling it is kept in. That goes for every letter, not for those of
/// ASCII alone: a link may spell the file it leads to `Ä.toml` where it is
/// kept as `ä.toml`. Beyond ASCII the systems' own rules cannot be told
/// from here (Windows takes `Σ.toml` and `ς.toml` for one name, which no
/// lower case does, and macOS a letter with its accent however the two are
/// written), so there two names that both go beyond it are taken for one.
/// A name taken for the file's by mistake costs a read.
fn same_file(one: &std::ffi::OsStr, other: &std::ffi::OsStr) -> bool {
    if one == other {
        return true;
    }
    if !cfg!(any(windows, target_os = "macos")) {
        return false;
    }
    // Through the upper case: `ς` and `σ` share one, and no lower one.
    let folded = |name: &std::ffi::OsStr| name.to_string_lossy().to_uppercase().to_lowercase();
    folded(one) == folded(other) || !(one.is_ascii() || other.is_ascii())
}

/// Waits until the changes have been quiet for `SETTLE`: a save is several
/// of them, and each starts the wait again. False when the watch has ended.
async fn settled(changes: &mut tokio_mpsc::UnboundedReceiver<()>) -> bool {
    loop {
        match tokio::time::timeout(SETTLE, changes.recv()).await {
            Ok(Some(())) => {}
            Ok(None) => return false,
            Err(_) => return true,
        }
    }
}

/// How many times the settings file is read for one change before the
/// reader leaves it until the next: a file that cannot be read at all (its
/// permissions, say) is not tried for ever.
const READ_TRIES: u32 = 6;

/// The longest wait between two of those reads. The first wait is `SETTLE`
/// and each after it twice as long: an editor that still holds the file
/// lets go within moments, and one that does not is asked less and less.
const READ_AGAIN_MAX: Duration = Duration::from_secs(1);

/// How often a watcher that is not live looks again for the place its link
/// leads to: nothing that is watched says when that place appears.
const LOOK_AGAIN: Duration = Duration::from_secs(2);

/// Sends the settings file's text each time it changes, once the changes
/// have settled, and says so when it can no longer see every change, or
/// can again ([`Event::SettingsWatch`]). A read that fails is tried again,
/// `READ_TRIES` times in all, and one the file changed under is not sent.
/// Runs until it is stopped ([`SettingsReader`]) or nothing is left that
/// could wake it. `follow` looks where the file is now
/// ([`SettingsWatcher::follow`]) and `read` reads it: the tests have reads
/// that fail, and ones that take their time.
async fn read_settings<W, R, F>(
    path: PathBuf,
    mut live: bool,
    mut changes: tokio_mpsc::UnboundedReceiver<()>,
    outbox: Outbox,
    mut follow: W,
    read: R,
) where
    W: FnMut() -> Result<(), String>,
    R: Fn(PathBuf) -> F,
    F: Future<Output = std::io::Result<Found>>,
{
    // What was sent last: the same text is not news, whatever woke us.
    let mut sent: Option<String> = None;
    let mut look_again = tokio::time::Instant::now() + LOOK_AGAIN;
    loop {
        // A link that leads where nothing can be watched (a directory that
        // is not there yet, a volume not mounted) is looked at again from
        // time to time: no event comes when the place appears.
        let woken = if live {
            Ok(changes.recv().await)
        } else {
            tokio::time::timeout_at(look_again, changes.recv()).await
        };
        let mut changed = match woken {
            Ok(Some(())) => true,
            Ok(None) => return,
            Err(_) => false,
        };
        let found = 'settle: loop {
            if changed && !settled(&mut changes).await {
                return;
            }
            // Before the file is read, so that no edit falls between the
            // read and the watch on a place the file has just moved to.
            let follows = follow();
            look_again = tokio::time::Instant::now() + LOOK_AGAIN;
            if follows.is_ok() != live {
                // Logged when it changes, not at every look.
                if let Err(why) = &follows {
                    log::warn!("{why}");
                }
                live = follows.is_ok();
                outbox.emit(Event::SettingsWatch { live });
            }
            // Looked again, and the place is still not there: no change
            // was reported, so there is nothing new to read.
            if !changed && !live {
                break 'settle None;
            }
            let mut tries = 1;
            let mut wait = SETTLE;
            loop {
                match read(path.clone()).await {
                    Ok(found) => match changes.try_recv() {
                        // Changed while it was read (an editor truncating
                        // it, say): what was read may be half of that save,
                        // and sending it would have the app apply it until
                        // the next read. It is left to settle and read
                        // again.
                        Ok(()) => {
                            changed = true;
                            continue 'settle;
                        }
                        // The end of the watch is not a change: this text
                        // is whole, and the task ends at its next wait.
                        Err(_) => break 'settle Some(found),
                    },
                    // Deleted: the settings in memory stay, and the next
                    // change made in the app writes the file again. No
                    // wait brings it back.
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                        break 'settle None;
                    }
                    Err(error) if tries == READ_TRIES => {
                        log::warn!("could not read {}: {error}", path.display());
                        break 'settle None;
                    }
                    Err(_) => tries += 1,
                }
                // An editor may still hold the file (a sharing violation on
                // Windows), and its letting go is not a change: nothing
                // would wake us. So the change stays pending, and the file
                // is tried again once the editor has had the time.
                match tokio::time::timeout(wait, changes.recv()).await {
                    // Changed meanwhile: a new save, to be left to settle
                    // and read from the first try.
                    Ok(Some(())) => {
                        changed = true;
                        continue 'settle;
                    }
                    Ok(None) => return,
                    Err(_) => wait = (wait * 2).min(READ_AGAIN_MAX),
                }
            }
        };
        let Some(found) = found else {
            continue;
        };
        match String::from_utf8(found.bytes) {
            Ok(text) if sent.as_deref() == Some(text.as_str()) => {}
            Ok(text) => {
                sent = Some(text.clone());
                outbox.emit(Event::SettingsFile {
                    text,
                    own: found.own,
                });
            }
            // Half-written, or not a settings file at all: the settings in
            // memory stay, and nothing on disk is touched.
            Err(_) => log::warn!("{} is not text; it is not read", path.display()),
        }
    }
}

/// Opens the settings file `path` with `start` (the editor or the file
/// manager), writing `text` to it first
/// when it is not there: the editor is given a file, and the UI thread
/// never looked at the disk to know. That write is the backend's own, as a
/// save's is: `written` is held from the look for the file to the end of
/// the write and then holds `text`. Written past the lock, it could land
/// after a save the app made meanwhile, and the reader would hand the
/// older text to the app as someone else's change.
fn open_in_editor(
    path: &std::path::Path,
    text: &str,
    written: &Written,
    start: impl FnOnce(&std::path::Path) -> std::io::Result<()>,
) -> Result<(), String> {
    open_where_missing(path, text, written, |path| !path.exists(), start)
}

/// [`open_in_editor`], with `missing` for the look at the disk: the tests
/// have someone else make the file between that look and the write.
fn open_where_missing(
    path: &std::path::Path,
    text: &str,
    written: &Written,
    missing: impl FnOnce(&std::path::Path) -> bool,
    start: impl FnOnce(&std::path::Path) -> std::io::Result<()>,
) -> Result<(), String> {
    {
        let mut written = lock(written);
        // The lock keeps the backend's own writes out, not anyone else's:
        // an editor can make the file after the look for it. So the file
        // is made only where none is, in one step, and one that appeared
        // meanwhile is opened as it is: theirs, and no write of the
        // backend's.
        if missing(path) {
            match crate::util::create_atomic(path, text.as_bytes()) {
                Ok(true) => *written = Some(text.to_owned()),
                Ok(false) => {}
                // As after a save: a write that failed leaves nothing on
                // the disk that is known to be the app's.
                Err(error) => {
                    *written = None;
                    return Err(error.to_string());
                }
            }
        }
    }
    start(path).map_err(|error| error.to_string())
}

/// The program that opens a text file for editing on this system, and what
/// it is given. `on_path` says whether a command of that name can be run.
fn editor_command(
    path: &std::path::Path,
    on_path: impl Fn(&str) -> bool,
) -> (String, Vec<std::ffi::OsString>) {
    let file = path.as_os_str().to_owned();
    #[cfg(target_os = "macos")]
    {
        let _ = on_path;
        ("open".into(), vec!["-t".into(), file])
    }
    #[cfg(windows)]
    {
        let _ = on_path;
        ("explorer".into(), vec![file])
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        // Omarchy has a launcher for the editor it is set up with, which
        // gives a terminal editor a terminal of its own; any other desktop
        // knows what edits a text file.
        let program = if on_path("omarchy-launch-editor") {
            "omarchy-launch-editor"
        } else {
            "xdg-open"
        };
        (program.into(), vec![file])
    }
}

/// Whether a command named `name` can be run from one of `dirs`, which are
/// `PATH`'s outside the tests.
fn on_path(name: &str, dirs: impl IntoIterator<Item = PathBuf>) -> bool {
    dirs.into_iter().any(|dir| can_run(&dir.join(name)))
}

/// Whether `file` is a file the system would start. A file of a command's
/// name that nobody may run would be chosen, fail to start, and keep the
/// next way to open the file from being tried.
fn can_run(file: &std::path::Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        // Any of the three bits: whose it is, the system says at the start.
        std::fs::metadata(file)
            .is_ok_and(|file| file.is_file() && file.permissions().mode() & 0o111 != 0)
    }
    #[cfg(not(unix))]
    {
        file.is_file()
    }
}

/// The program that shows where `path` is kept on this system, and what
/// it is given.
fn reveal_command(path: &std::path::Path) -> (String, Vec<std::ffi::OsString>) {
    #[cfg(target_os = "macos")]
    {
        (
            "open".into(),
            vec!["-R".into(), path.as_os_str().to_owned()],
        )
    }
    #[cfg(windows)]
    {
        // One argument: explorer reads the path after the comma.
        let mut select = std::ffi::OsString::from("/select,");
        select.push(path);
        ("explorer".into(), vec![select])
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        // No two file managers select a file the same way; every desktop
        // opens a directory.
        let directory = crate::util::directory_of(path).as_os_str().to_owned();
        ("xdg-open".into(), vec![directory])
    }
}

/// Starts `program` and lets it go: it is the user's window from here. A
/// thread of its own waits for it, so it leaves no zombie, and says in the
/// log when it ended with a failure: a launcher with nothing to open the
/// file with has no other way to be heard.
fn start(program: String, args: Vec<std::ffi::OsString>) -> std::io::Result<()> {
    use std::process::Stdio;
    let mut child = std::process::Command::new(&program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    std::thread::spawn(move || match child.wait() {
        Ok(status) if status.success() => {}
        // Windows' explorer ends with a failure whatever it did: its
        // status says nothing, and a warning at every press would.
        Ok(_) if cfg!(windows) => {}
        Ok(status) => log::warn!("{program} ended with {status}"),
        Err(error) => log::warn!("could not wait for {program}: {error}"),
    });
    Ok(())
}

/// Starts the editor on `path`.
fn start_editor(path: &std::path::Path) -> std::io::Result<()> {
    let paths = std::env::var_os("PATH");
    let found = |name: &str| {
        paths
            .as_deref()
            .is_some_and(|paths| on_path(name, std::env::split_paths(paths)))
    };
    let (program, args) = editor_command(path, found);
    start(program, args)
}

/// Starts the file manager where `path` is.
fn start_reveal(path: &std::path::Path) -> std::io::Result<()> {
    let (program, args) = reveal_command(path);
    start(program, args)
}

/// Hands the settings file to a program: `open_in_editor` with `start`,
/// then the answer, logged when it is a failure and sent to the app.
fn open_settings_file(
    path: &std::path::Path,
    text: &str,
    written: &Written,
    with: Opened,
    start: impl FnOnce(&std::path::Path) -> std::io::Result<()>,
    outbox: &Outbox,
) {
    let result = open_in_editor(path, text, written, start);
    if let Err(error) = &result {
        match with {
            Opened::Editor => {
                log::warn!("could not open {} in the editor: {error}", path.display());
            }
            Opened::Folder => {
                log::warn!(
                    "could not show {} in the file manager: {error}",
                    path.display()
                );
            }
        }
    }
    outbox.emit(Event::SettingsFileOpened { with, result });
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
    /// Watches and reads the settings file. Dropping it ends both.
    settings_watch: Option<SettingsReader>,
    #[cfg(test)]
    watched: Watched,
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
            settings_watch: None,
            #[cfg(test)]
            watched: Watched::default(),
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
                #[cfg(test)]
                lock(&self.watched).insert(done.session, Arc::clone(&handle.running));
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
                    // A cancel stops one statement at most, and none when
                    // it arrives between two: a script must not run the
                    // rest of its statements for a closed session. Nor may
                    // the session start a command it has already taken off
                    // its queue, which dropping the handle cannot prevent.
                    let (script, again) = {
                        let mut running = lock(&handle.running);
                        running.close();
                        // Again and again: one cancel can reach the server
                        // before its statement, and the script may have no
                        // timeout to send another. Not beside the cancels
                        // the user or the timeout already keeps sending.
                        let again = running
                            .start_cancelling()
                            .then_some(running.request)
                            .flatten();
                        (running.stop.is_some(), again)
                    };
                    let cancel = handle.cancel.clone();
                    let running = Arc::clone(&handle.running);
                    // Stop first: the cancel can end the running query on
                    // another thread at once, and the session must already
                    // see the stop then, not start a queued command.
                    drop(handle);
                    if let Some(request) = again {
                        tokio::spawn(keep_cancelling(cancel, running, request));
                    } else if !script {
                        tokio::spawn(send_cancel(&cancel));
                    }
                } else if self.connecting.contains(&session) {
                    self.closed_early.insert(session);
                }
            }
            Command::Cancel { session, request } => {
                if let Some(handle) = self.sessions.get(&session) {
                    let cancel = handle.cancel.clone();
                    cancel_request(&handle.running, request, move || send_cancel(&cancel));
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
            Command::WatchSettings { path } => {
                let written = Arc::clone(&self.saves.settings_written);
                let live = match watch_settings(path, self.outbox.clone(), written) {
                    Ok((reader, live)) => {
                        self.settings_watch = Some(reader);
                        live
                    }
                    Err(error) => {
                        log::warn!("could not watch the settings file: {error}");
                        false
                    }
                };
                self.outbox.emit(Event::SettingsWatch { live });
            }
            Command::EditSettingsFile { path, text } => {
                let outbox = self.outbox.clone();
                let written = Arc::clone(&self.saves.settings_written);
                tokio::task::spawn_blocking(move || {
                    open_settings_file(
                        &path,
                        &text,
                        &written,
                        Opened::Editor,
                        start_editor,
                        &outbox,
                    );
                });
            }
            Command::RevealSettingsFile { path, text } => {
                let outbox = self.outbox.clone();
                let written = Arc::clone(&self.saves.settings_written);
                tokio::task::spawn_blocking(move || {
                    open_settings_file(
                        &path,
                        &text,
                        &written,
                        Opened::Folder,
                        start_reveal,
                        &outbox,
                    );
                });
            }
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
        | Command::ListDatabases { session, .. }
        | Command::RunSql { session, .. }
        | Command::ServerVersion { session, .. } => *session,
        Command::Test { .. }
        | Command::LoadSecret { .. }
        | Command::StoreSecret { .. }
        | Command::Save { .. }
        | Command::WatchSettings { .. }
        | Command::EditSettingsFile { .. }
        | Command::RevealSettingsFile { .. }
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
        | Command::ListDatabases { request, .. }
        | Command::RunSql { request, .. }
        | Command::ServerVersion { request, .. } => Some(*request),
        Command::Connect { .. }
        | Command::Test { .. }
        | Command::Close { .. }
        | Command::Cancel { .. }
        | Command::LoadSecret { .. }
        | Command::StoreSecret { .. }
        | Command::Save { .. }
        | Command::WatchSettings { .. }
        | Command::EditSettingsFile { .. }
        | Command::RevealSettingsFile { .. }
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
        Command::RunSql {
            session, request, ..
        } => Event::SqlRan {
            session,
            request,
            result: Err(error),
            cancel: None,
        },
        Command::ServerVersion { session, request } => Event::ServerVersion {
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
        | Command::WatchSettings { .. }
        | Command::EditSettingsFile { .. }
        | Command::RevealSettingsFile { .. }
        | Command::Flush { .. } => return,
    };
    outbox.emit(event);
}

/// Answers a command the user cancelled while it was still queued: it
/// never ran. A script answers as one stopped before its first statement
/// does.
fn skip(outbox: &Outbox, command: Command) {
    match command {
        Command::RunSql {
            session, request, ..
        } => outbox.emit(Event::SqlRan {
            session,
            request,
            result: Ok(ScriptOutcome {
                results: Vec::new(),
                stopped: true,
            }),
            cancel: Some(CancelReason::User),
        }),
        command => fail(outbox, command, Error::Cancelled),
    }
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
    commands: tokio_mpsc::UnboundedReceiver<Command>,
    mut stop: tokio::sync::oneshot::Receiver<()>,
    running: Arc<Mutex<Running>>,
    outbox: Outbox,
) {
    let mut end = SessionEnd::new(session, Arc::clone(&running), commands, outbox.clone());
    loop {
        let command = tokio::select! {
            biased;
            _ = &mut stop => break,
            command = end.commands.recv() => match command {
                Some(command) => command,
                None => break,
            },
        };
        // Kept by `end` until it has answered, so that it is answered even
        // if this task ends first.
        let command = &*end.command.insert(command);
        // A cancel meant for the previous command may still be on its way:
        // let it land first, so it cannot stop this one.
        let cancels = std::mem::take(&mut lock(&running).cancels);
        for cancel in cancels {
            let _ = cancel.await;
        }
        let request = request_of(command);
        // A script's stop flag; any other command leaves it unused.
        let script_stop = StopFlag::new();
        let skipped = {
            let mut running = lock(&running);
            // Closed while this command waited above: checked together
            // with registering it, so a Close either finds it running (and
            // stops it) or keeps it from starting.
            if running.closed {
                break;
            }
            let skipped = request.is_some_and(|request| running.skip.remove(&request));
            // Requests run in the order they were made, so an id at or
            // before this one will never come again.
            if let Some(request) = request {
                running.skip.retain(|id| id.0 > request.0);
            }
            if !skipped {
                running.request = request;
                // In the same critical section, so a Cancel that arrives
                // right after the run starts always finds the flag.
                running.stop =
                    matches!(command, Command::RunSql { .. }).then(|| script_stop.clone());
            }
            skipped
        };
        if skipped {
            if let Some(command) = end.command.take() {
                skip(&outbox, command);
            }
            continue;
        }
        #[cfg(test)]
        if request.is_some() && lock(&running).panics == request {
            panic!("a test has the session panic at {request:?}");
        }
        let lost = match command {
            Command::ListSchemas { session, request } => {
                let result = connection.list_schemas().await;
                let lost = lost_error(&result);
                outbox.emit(Event::Schemas {
                    session: *session,
                    request: *request,
                    result,
                });
                lost
            }
            Command::ListObjects {
                session,
                request,
                schema,
            } => {
                let result = connection.list_objects(schema).await;
                let lost = lost_error(&result);
                outbox.emit(Event::Objects {
                    session: *session,
                    request: *request,
                    schema: schema.clone(),
                    result,
                });
                lost
            }
            Command::Describe {
                session,
                request,
                object,
            } => {
                let result = connection.describe(object).await;
                let lost = lost_error(&result);
                outbox.emit(Event::Structure {
                    session: *session,
                    request: *request,
                    result,
                });
                lost
            }
            Command::FetchRows {
                session,
                request,
                query,
            } => {
                let result = connection.fetch_rows(query).await;
                let lost = lost_error(&result);
                outbox.emit(Event::Rows {
                    session: *session,
                    request: *request,
                    result,
                });
                lost
            }
            Command::CountRows {
                session,
                request,
                query,
            } => {
                let result = connection.count_rows(query).await;
                let lost = lost_error(&result);
                outbox.emit(Event::Count {
                    session: *session,
                    request: *request,
                    result,
                });
                lost
            }
            Command::ListDatabases { session, request } => {
                let result = connection.list_databases().await;
                let lost = lost_error(&result);
                outbox.emit(Event::Databases {
                    session: *session,
                    request: *request,
                    result,
                });
                lost
            }
            Command::RunSql {
                session,
                request,
                statements,
                limit,
                timeout,
            } => {
                let timer = timeout.map(|after| {
                    Timer::start(
                        *request,
                        after,
                        script_stop.clone(),
                        connection.cancel_handle(),
                        Arc::clone(&running),
                    )
                });
                // Awaited to its end whatever stops it: the script rolls
                // back and leaves the session as it found it.
                let result = connection
                    .run_script(statements, *limit, &script_stop)
                    .await;
                let timed_out = match timer {
                    Some(timer) => timer.end().await,
                    None => None,
                };
                let cancel = cancel_reason(&script_stop, timed_out, &result);
                let lost = lost_error(&result);
                outbox.emit(Event::SqlRan {
                    session: *session,
                    request: *request,
                    result,
                    cancel,
                });
                lost
            }
            Command::ServerVersion { session, request } => {
                let result = connection.server_version().await;
                let lost = lost_error(&result);
                outbox.emit(Event::ServerVersion {
                    session: *session,
                    request: *request,
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
            | Command::WatchSettings { .. }
            | Command::EditSettingsFile { .. }
            | Command::RevealSettingsFile { .. }
            | Command::Flush { .. } => None,
        };
        end.command = None;
        {
            let mut running = lock(&running);
            running.request = None;
            running.stop = None;
            running.cancelling = false;
        }
        if let Some(error) = lost {
            fail_queued(&mut end.commands, &outbox, &error);
            outbox.emit(Event::Disconnected { session, error });
            break;
        }
    }
    end.finished = true;
    let _ = connection.close().await;
}

fn lost_error<T>(result: &Result<T, Error>) -> Option<Error> {
    match result {
        Err(error) if error.is_connection_lost() => Some(error.clone()),
        _ => None,
    }
}

/// How long to wait after the first cancel before sending another. Each
/// wait after that is twice as long, up to `CANCEL_AGAIN_MAX`.
const CANCEL_AGAIN: Duration = Duration::from_millis(500);
const CANCEL_AGAIN_MAX: Duration = Duration::from_secs(5);

/// One cancel of a session, whatever comes of it.
fn send_cancel(cancel: &CancelHandle) -> impl Future<Output = ()> + Send + 'static + use<> {
    let cancel = cancel.clone();
    async move {
        let _ = cancel.cancel().await;
    }
}

/// The user's cancel of `request`. For what the session runs now: a script
/// is stopped and cancelled until it is finishing (see `keep_sending`); one
/// that is being cancelled already takes one more cancel at once. Any other
/// command takes one cancel. A request that is not running is skipped when
/// its turn comes.
fn cancel_request<C>(
    running: &Arc<Mutex<Running>>,
    request: RequestId,
    send: impl Fn() -> C + Send + 'static,
) where
    C: Future<Output = ()> + Send + 'static,
{
    let mut state = lock(running);
    if state.request != Some(request) {
        // Queued, or already answered: the session drops the id once it
        // passes it.
        state.skip.insert(request);
        return;
    }
    // Before a cancel is sent: a script it ends at once must already see
    // who stopped it.
    state.stop_script();
    if state.start_cancelling() {
        // More than one: a cancel that reaches the server in the gap
        // between two of the script's queries is lost, and without a
        // timeout the statement would run until the user cancels again.
        tokio::spawn(keep_sending(send, Arc::clone(running), request));
    } else if state.stop.is_none() || state.script_takes_a_cancel() {
        // Anything but a script takes one cancel while it runs. So does a
        // script whose cancels already go out, at once: the next of those
        // may be seconds away, and the user asked now. No second row of
        // them starts.
        state.cancels.retain(|task| !task.is_finished());
        state.cancels.push(tokio::spawn(send()));
    }
}

/// Sends the session's cancel until the script `request` runs reaches its
/// cleanup or is gone. A cancel can reach the server before the statement
/// it is meant for, so one may not be enough; once the cleanup has begun,
/// another would only interrupt the rollback.
///
/// The next cancel waits for the one before it to finish, and a little
/// longer each time: on MySQL every cancel is a new authenticated
/// connection, and failing ones in quick succession can get this host
/// blocked (`max_connect_errors`).
async fn keep_cancelling(cancel: CancelHandle, running: Arc<Mutex<Running>>, request: RequestId) {
    keep_sending(move || send_cancel(&cancel), running, request).await;
}

/// `keep_cancelling` with any future as the cancel, so that a test can
/// hold one back.
async fn keep_sending<C>(
    send: impl Fn() -> C + Send,
    running: Arc<Mutex<Running>>,
    request: RequestId,
) where
    C: Future<Output = ()> + Send + 'static,
{
    let mut pause = CANCEL_AGAIN;
    loop {
        let (done, finished) = tokio::sync::oneshot::channel::<()>();
        {
            // Checked and sent under the lock the session takes to say its
            // script is over, so no cancel is sent after that, and none
            // to what the session runs next. Kept with the other cancels:
            // the session waits for all of them before its next command.
            let mut running = lock(&running);
            if running.request != Some(request) || !running.script_takes_a_cancel() {
                return;
            }
            let cancel = send();
            running.cancels.retain(|task| !task.is_finished());
            running.cancels.push(tokio::spawn(async move {
                cancel.await;
                let _ = done.send(());
            }));
        }
        // Over when the cancel is, whether it was sent or failed.
        let _ = finished.await;
        tokio::time::sleep(pause).await;
        pause = longer(pause);
    }
}

/// The wait that follows a wait of `pause` between two cancels.
fn longer(pause: Duration) -> Duration {
    (pause * 2).min(CANCEL_AGAIN_MAX)
}

/// A running script's timeout. The script is never dropped: once the time
/// is up the timer stops it through its flag and the session's cancel, and
/// the session goes on waiting for the script to roll back and return.
struct Timer {
    after: Duration,
    task: tokio::task::JoinHandle<()>,
    /// Whether the time was up and it was the timer that stopped the
    /// script.
    fired: Arc<AtomicBool>,
}

impl Timer {
    /// Stops the script `request` runs once `after` has passed.
    fn start(
        request: RequestId,
        after: Duration,
        stop: StopFlag,
        cancel: CancelHandle,
        running: Arc<Mutex<Running>>,
    ) -> Self {
        let fired = Arc::new(AtomicBool::new(false));
        let task = tokio::spawn({
            let fired = Arc::clone(&fired);
            async move {
                tokio::time::sleep(after).await;
                // A script the user stopped just before this did not time
                // out, and the user's cancels are still being sent.
                if stop.stop() {
                    fired.store(true, Ordering::SeqCst);
                }
                let first = lock(&running).start_cancelling();
                if first {
                    keep_cancelling(cancel, running, request).await;
                }
            }
        });
        Self { after, task, fired }
    }

    /// Stops the timer and gives the timeout if it stopped the script.
    /// Waits until the timer's task is gone, so every cancel the timer sent
    /// is in `Running.cancels` by now and it sends none after the script
    /// has returned. That is the timer's cancels only: the ones a Cancel or
    /// a Close keeps sending come from a task of their own, which sends
    /// none once the session no longer runs their request.
    async fn end(mut self) -> Option<Duration> {
        self.task.abort();
        let _ = (&mut self.task).await;
        self.fired.load(Ordering::SeqCst).then_some(self.after)
    }
}

impl Drop for Timer {
    /// A timer must not outlive its script, however the script ends.
    fn drop(&mut self) {
        self.task.abort();
    }
}

/// Who stopped a script, when a stop ended it: `timed_out` holds the
/// timeout when its timer was what stopped it, and anything else that set
/// `stop` is the user. A stop that came after the script had ended by
/// itself is no reason, and neither is a cancel nobody here asked for (the
/// server's own `statement_timeout`, say).
fn cancel_reason(
    stop: &StopFlag,
    timed_out: Option<Duration>,
    result: &Result<ScriptOutcome, Error>,
) -> Option<CancelReason> {
    let cancelled = match result {
        Ok(outcome) => outcome.was_cancelled(),
        Err(error) => *error == Error::Cancelled,
    };
    (stop.is_stopped() && cancelled)
        .then(|| timed_out.map_or(CancelReason::User, CancelReason::Timeout))
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

    fn statements(text: &str) -> Vec<tabletist_db::sql::Statement> {
        tabletist_db::sql::statements(tabletist_db::Dialect::Sqlite, text)
    }

    fn connected_sqlite() -> (tempfile::TempDir, Backend, SessionId) {
        let (dir, spec) = fixture();
        let (backend, session) = connected(spec);
        (dir, backend, session)
    }

    /// Waits until the session runs `request` (a script, or not): a Cancel
    /// for a request still queued skips it, and a Close ends the session
    /// before a queued request runs or answers.
    fn wait_until_running(backend: &Backend, session: SessionId, request: RequestId, script: bool) {
        let deadline = std::time::Instant::now() + WAIT;
        while backend.running(session) != Some((request, script)) {
            assert!(
                std::time::Instant::now() < deadline,
                "the session must start {request:?}"
            );
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    /// The next event, which must be a script's answer.
    fn sql_ran(
        backend: &mut Backend,
    ) -> (
        RequestId,
        Result<ScriptOutcome, Error>,
        Option<CancelReason>,
    ) {
        match backend.wait(WAIT) {
            Some(Event::SqlRan {
                request,
                result,
                cancel,
                ..
            }) => (request, result, cancel),
            other => panic!("expected SqlRan, got {other:?}"),
        }
    }

    /// A script stopped before any of its statements ran.
    fn never_ran() -> ScriptOutcome {
        ScriptOutcome {
            results: Vec::new(),
            stopped: true,
        }
    }

    /// Never ends by itself: only a stop or a cancel ends it.
    const ENDLESS: &str =
        "WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n) SELECT count(*) FROM n";

    #[test]
    fn a_script_runs_and_answers_with_its_outcome() {
        let (_dir, mut backend, session) = connected_sqlite();
        backend.send(Command::RunSql {
            session,
            request: RequestId(2),
            statements: statements("SELECT 1; SELECT 2"),
            limit: 10,
            timeout: None,
        });
        let (request, result, cancel) = sql_ran(&mut backend);
        assert_eq!(request, RequestId(2));
        assert_eq!(cancel, None);
        assert_eq!(result.unwrap().results.len(), 2);
    }

    #[test]
    fn a_script_that_ends_before_its_timeout_is_not_cancelled() {
        let (_dir, mut backend, session) = connected_sqlite();
        backend.send(Command::RunSql {
            session,
            request: RequestId(2),
            statements: statements("SELECT 1"),
            limit: 10,
            timeout: Some(Duration::from_secs(3600)),
        });
        let (_, result, cancel) = sql_ran(&mut backend);
        assert_eq!(cancel, None);
        assert!(!result.unwrap().was_cancelled());
        // The session does not wait for the timeout to take what is next.
        backend.send(Command::ListSchemas {
            session,
            request: RequestId(3),
        });
        match backend.wait(WAIT) {
            Some(Event::Schemas { result: Ok(_), .. }) => {}
            other => panic!("expected the schemas, got {other:?}"),
        }
    }

    #[test]
    fn a_timeout_cancels_the_script_and_keeps_earlier_results() {
        let (_dir, mut backend, session) = connected_sqlite();
        let after = Duration::from_millis(300);
        backend.send(Command::RunSql {
            session,
            request: RequestId(2),
            statements: statements(&format!("SELECT 1; {ENDLESS}")),
            limit: 10,
            timeout: Some(after),
        });
        let (_, result, cancel) = sql_ran(&mut backend);
        assert_eq!(cancel, Some(CancelReason::Timeout(after)));
        let outcome = result.unwrap();
        assert!(
            matches!(
                outcome.results[0].outcome,
                tabletist_db::StatementOutcome::Rows { .. }
            ),
            "{outcome:?}"
        );
        assert_eq!(
            outcome.results[1].outcome,
            tabletist_db::StatementOutcome::Cancelled
        );
        // The session keeps working.
        backend.send(Command::RunSql {
            session,
            request: RequestId(3),
            statements: statements("SELECT count(*) FROM big"),
            limit: 10,
            timeout: None,
        });
        let (request, result, cancel) = sql_ran(&mut backend);
        assert_eq!(request, RequestId(3));
        assert_eq!(cancel, None);
        assert!(!result.unwrap().was_cancelled());
    }

    #[test]
    fn a_user_cancel_stops_a_script() {
        let (_dir, mut backend, session) = connected_sqlite();
        backend.send(Command::RunSql {
            session,
            request: RequestId(2),
            statements: statements(&format!("SELECT 1; {ENDLESS}")),
            limit: 10,
            timeout: Some(Duration::from_secs(3600)),
        });
        wait_until_running(&backend, session, RequestId(2), true);
        backend.send(Command::Cancel {
            session,
            request: RequestId(2),
        });
        let (request, result, cancel) = sql_ran(&mut backend);
        assert_eq!(request, RequestId(2));
        assert_eq!(cancel, Some(CancelReason::User));
        let outcome = result.unwrap();
        assert!(outcome.was_cancelled());
        // The cancel lands in the endless statement, and the rows of the
        // one before it are kept. The test does not wait for that one to
        // end, so the cancel can also land before it: then nothing ran.
        use tabletist_db::StatementOutcome::{Cancelled, Rows};
        let ran: Vec<_> = outcome.results.iter().map(|ran| &ran.outcome).collect();
        assert!(
            matches!(ran.as_slice(), [Rows { .. }, Cancelled] | [Cancelled] | []),
            "{outcome:?}"
        );
        backend.send(Command::ListSchemas {
            session,
            request: RequestId(3),
        });
        match backend.wait(WAIT) {
            Some(Event::Schemas { result: Ok(_), .. }) => {}
            other => panic!("expected the schemas, got {other:?}"),
        }
    }

    #[test]
    fn a_cancelled_script_leaves_the_next_one_to_be_cancelled() {
        let (_dir, mut backend, session) = connected_sqlite();
        for request in [RequestId(2), RequestId(3)] {
            backend.send(Command::RunSql {
                session,
                request,
                statements: statements(ENDLESS),
                limit: 10,
                timeout: None,
            });
            wait_until_running(&backend, session, request, true);
            // The cancels sent for the script before it ended with that
            // script. Were they still counted as going out, this one's
            // Cancel would start none, and on PostgreSQL or MySQL nothing
            // would stop its statement.
            assert!(!backend.cancelling(session), "{request:?}");
            backend.send(Command::Cancel { session, request });
            let (answered, result, cancel) = sql_ran(&mut backend);
            assert_eq!(answered, request);
            assert_eq!(cancel, Some(CancelReason::User));
            assert!(result.unwrap().was_cancelled());
        }
    }

    #[test]
    fn a_script_cancelled_while_queued_never_runs() {
        let (_dir, mut backend, session) = connected_sqlite();
        backend.send(Command::RunSql {
            session,
            request: RequestId(2),
            statements: statements(ENDLESS),
            limit: 10,
            timeout: None,
        });
        backend.send(Command::RunSql {
            session,
            request: RequestId(3),
            statements: statements("SELECT 1"),
            limit: 10,
            timeout: None,
        });
        // Superseded while the first still runs (or waits its turn).
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
        assert!(
            matches!(
                events.as_slice(),
                [
                    // Stopped while it ran, or before it began.
                    Event::SqlRan {
                        request: RequestId(2),
                        result: Ok(_),
                        cancel: Some(CancelReason::User),
                        ..
                    },
                    // Never run: `SELECT 1` would have answered with a row.
                    Event::SqlRan {
                        request: RequestId(3),
                        result: Ok(skipped),
                        cancel: Some(CancelReason::User),
                        ..
                    },
                ] if *skipped == never_ran()
            ),
            "{events:?}"
        );
    }

    #[test]
    fn a_script_cancelled_while_queued_answers_as_one_stopped_before_it_began() {
        let (events, received) = mpsc::channel();
        let outbox = Outbox {
            events,
            waker: Waker::default(),
        };
        let session = SessionId(1);
        skip(
            &outbox,
            Command::RunSql {
                session,
                request: RequestId(2),
                statements: statements("SELECT 1"),
                limit: 10,
                timeout: None,
            },
        );
        skip(
            &outbox,
            Command::ListSchemas {
                session,
                request: RequestId(3),
            },
        );
        // A script that fails for another reason was stopped by nobody.
        fail(
            &outbox,
            Command::RunSql {
                session,
                request: RequestId(4),
                statements: statements("SELECT 1"),
                limit: 10,
                timeout: None,
            },
            lost(),
        );
        let answered: Vec<Event> = received.try_iter().collect();
        assert!(
            matches!(
                answered.as_slice(),
                [
                    Event::SqlRan {
                        request: RequestId(2),
                        result: Ok(skipped),
                        cancel: Some(CancelReason::User),
                        ..
                    },
                    Event::Schemas {
                        request: RequestId(3),
                        result: Err(Error::Cancelled),
                        ..
                    },
                    Event::SqlRan {
                        request: RequestId(4),
                        result: Err(Error::ConnectionLost(_)),
                        cancel: None,
                        ..
                    },
                ] if *skipped == never_ran()
            ),
            "{answered:?}"
        );
        // The same answer as a script stopped right after it started.
        let stopped = StopFlag::new();
        stopped.stop();
        assert!(never_ran().was_cancelled());
        assert_eq!(
            cancel_reason(&stopped, None, &Ok(never_ran())),
            Some(CancelReason::User)
        );
    }

    #[test]
    fn closing_a_session_stops_its_script() {
        let (_dir, mut backend, session) = connected_sqlite();
        backend.send(Command::RunSql {
            session,
            request: RequestId(2),
            statements: statements(&format!("SELECT 1; {ENDLESS}")),
            limit: 10,
            timeout: None,
        });
        wait_until_running(&backend, session, RequestId(2), true);
        backend.send(Command::Close { session });
        // A reason is reported only when the stop flag was set: the cancel
        // alone would end this run with none.
        let (request, result, cancel) = sql_ran(&mut backend);
        assert_eq!(request, RequestId(2));
        assert_eq!(cancel, Some(CancelReason::User));
        assert!(result.unwrap().was_cancelled());
        backend.send(Command::ListSchemas {
            session,
            request: RequestId(3),
        });
        match backend.wait(WAIT) {
            Some(Event::Schemas {
                result: Err(error), ..
            }) => assert!(error.is_connection_lost()),
            other => panic!("expected a lost-connection error, got {other:?}"),
        }
    }

    /// A runtime like the backend's, for tests that drive a worker, a
    /// session or a timer by hand.
    fn runtime() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .unwrap()
    }

    /// One thread, so a task runs only while the test waits: what a timer
    /// or a closed session's cancels have done between two steps of a test
    /// is then the same on every run.
    fn one_thread() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
    }

    async fn sqlite_connection() -> (tempfile::TempDir, Connection) {
        let (dir, spec) = fixture();
        let connection = Connection::connect(&spec, &Secrets::default())
            .await
            .unwrap();
        (dir, connection)
    }

    fn quiet_outbox() -> (Outbox, mpsc::Receiver<Event>) {
        let (events, received) = mpsc::channel();
        let outbox = Outbox {
            events,
            waker: Waker::default(),
        };
        (outbox, received)
    }

    /// The worker's side of a session on `connection` in the state
    /// `running`, and the session's side of its two channels.
    fn session_handle(
        connection: &Connection,
        running: Running,
    ) -> (
        SessionHandle,
        tokio_mpsc::UnboundedReceiver<Command>,
        tokio::sync::oneshot::Receiver<()>,
    ) {
        let (queue, commands) = tokio_mpsc::unbounded_channel();
        let (stop, stopped) = tokio::sync::oneshot::channel();
        let handle = SessionHandle {
            queue,
            cancel: connection.cancel_handle(),
            running: Arc::new(Mutex::new(running)),
            _stop: stop,
        };
        (handle, commands, stopped)
    }

    /// The state of a session running request 2, a script with this flag.
    fn running_script(stop: &StopFlag) -> Running {
        Running {
            request: Some(RequestId(2)),
            stop: Some(stop.clone()),
            ..Running::default()
        }
    }

    /// Waits until cancels were sent for what `running` runs, and takes
    /// them away.
    async fn cancels_sent(running: &Mutex<Running>) {
        let deadline = std::time::Instant::now() + WAIT;
        loop {
            let sent = std::mem::take(&mut lock(running).cancels);
            if !sent.is_empty() {
                return;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "a cancel must be sent"
            );
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    /// Longer than the wait after the second cancel of a row: a third one
    /// would have been sent by the end of it.
    const THIRD_CANCEL: Duration = Duration::from_millis(1500);

    #[test]
    fn a_session_closed_before_its_script_is_registered_never_runs_it() {
        let (outbox, received) = quiet_outbox();
        let ended = runtime().block_on(async {
            let (_dir, connection) = sqlite_connection().await;
            let session = SessionId(1);
            let (handle, commands, stopped) = session_handle(&connection, Running::default());
            // A cancel for the command before is still on its way, so the
            // session takes the script off its queue and waits: it has not
            // said yet that the script runs.
            let (land, landed) = tokio::sync::oneshot::channel::<()>();
            lock(&handle.running).cancels.push(tokio::spawn(async move {
                let _ = landed.await;
            }));
            handle
                .queue
                .send(Command::RunSql {
                    session,
                    request: RequestId(2),
                    statements: statements(&format!("SELECT 1; {ENDLESS}")),
                    limit: 10,
                    timeout: None,
                })
                .unwrap();
            let mut task = tokio::spawn(run_session(
                session,
                connection,
                commands,
                stopped,
                Arc::clone(&handle.running),
                outbox.clone(),
            ));
            tokio::time::sleep(Duration::from_millis(100)).await;
            let (mut worker, _ready) = Worker::new(outbox, Keyring::memory());
            worker.sessions.insert(session, handle);
            worker.handle(Command::Close { session });
            // Only now does the session get to the script.
            land.send(()).unwrap();
            let ended = tokio::time::timeout(WAIT, &mut task).await.is_ok();
            // A script left running would keep the runtime from ending.
            task.abort();
            ended
        });
        assert!(ended, "the session must end after Close");
        let answered: Vec<Event> = received.try_iter().collect();
        assert!(answered.is_empty(), "the script must not run: {answered:?}");
    }

    #[test]
    fn closing_a_session_keeps_cancelling_its_script() {
        let (outbox, _received) = quiet_outbox();
        one_thread().block_on(async {
            let (_dir, connection) = sqlite_connection().await;
            let session = SessionId(1);
            let stop = StopFlag::new();
            let (handle, _commands, _stopped) = session_handle(&connection, running_script(&stop));
            let running = Arc::clone(&handle.running);
            let (mut worker, _ready) = Worker::new(outbox, Keyring::memory());
            worker.sessions.insert(session, handle);
            worker.handle(Command::Close { session });
            assert!(stop.is_stopped());
            assert!(lock(&running).closed);
            // The first cancel, and another: the first may have reached
            // the server between two statements.
            cancels_sent(&running).await;
            cancels_sent(&running).await;
            // The session says its script is over, as `run_session` does.
            lock(&running).stop = None;
            tokio::time::sleep(THIRD_CANCEL).await;
            assert!(lock(&running).cancels.is_empty());
        });
    }

    #[test]
    fn closing_a_session_adds_no_cancels_to_the_ones_already_sent() {
        let (outbox, _received) = quiet_outbox();
        one_thread().block_on(async {
            let (_dir, connection) = sqlite_connection().await;
            let session = SessionId(1);
            let stop = StopFlag::new();
            let mut script = running_script(&stop);
            // The user cancelled the script: cancels go out for it. (No
            // task sends them here, so any cancel below is the Close's.)
            assert!(script.start_cancelling());
            let (handle, _commands, _stopped) = session_handle(&connection, script);
            let running = Arc::clone(&handle.running);
            let (mut worker, _ready) = Worker::new(outbox, Keyring::memory());
            worker.sessions.insert(session, handle);
            worker.handle(Command::Close { session });
            // The script stops and the session starts nothing more.
            assert!(stop.is_stopped());
            assert!(lock(&running).closed);
            // No second row of cancels beside the first, and not one more.
            tokio::time::sleep(THIRD_CANCEL).await;
            assert!(lock(&running).cancels.is_empty());
        });
    }

    #[test]
    fn a_users_cancel_keeps_cancelling_a_script() {
        let (outbox, _received) = quiet_outbox();
        one_thread().block_on(async {
            let (_dir, connection) = sqlite_connection().await;
            let session = SessionId(1);
            let stop = StopFlag::new();
            let (handle, _commands, _stopped) = session_handle(&connection, running_script(&stop));
            let running = Arc::clone(&handle.running);
            let (mut worker, _ready) = Worker::new(outbox, Keyring::memory());
            worker.sessions.insert(session, handle);
            worker.handle(Command::Cancel {
                session,
                request: RequestId(2),
            });
            assert!(stop.is_stopped());
            // The first cancel, and another: the first may have reached
            // the server between two statements, and a script without a
            // timeout has nobody else to send the next.
            cancels_sent(&running).await;
            cancels_sent(&running).await;
            // The driver begins its cleanup: a cancel would interrupt it.
            stop.finish();
            tokio::time::sleep(THIRD_CANCEL).await;
            assert!(lock(&running).cancels.is_empty());
        });
    }

    #[test]
    fn a_cancel_reaches_a_running_script_but_not_its_cleanup() {
        let (outbox, _received) = quiet_outbox();
        runtime().block_on(async {
            let (_dir, connection) = sqlite_connection().await;
            let session = SessionId(1);
            let (mut worker, _ready) = Worker::new(outbox, Keyring::memory());
            for finishing in [false, true] {
                let stop = StopFlag::new();
                if finishing {
                    stop.finish();
                }
                let (handle, _commands, _stopped) =
                    session_handle(&connection, running_script(&stop));
                let running = Arc::clone(&handle.running);
                worker.sessions.insert(session, handle);
                worker.handle(Command::Cancel {
                    session,
                    request: RequestId(2),
                });
                // Stopped either way: that is what says the user did it.
                assert!(stop.is_stopped(), "finishing: {finishing}");
                // Cancels are sent to a script that still runs a statement,
                // and none to one that is cleaning up.
                assert_eq!(lock(&running).cancelling, !finishing);
                if finishing {
                    assert!(lock(&running).cancels.is_empty());
                } else {
                    cancels_sent(&running).await;
                    stop.finish();
                }
                assert!(lock(&running).skip.is_empty());
            }
            // Anything but a script takes its cancel as it always did.
            let other = Running {
                request: Some(RequestId(2)),
                ..Running::default()
            };
            let (handle, _commands, _stopped) = session_handle(&connection, other);
            let running = Arc::clone(&handle.running);
            worker.sessions.insert(session, handle);
            worker.handle(Command::Cancel {
                session,
                request: RequestId(2),
            });
            assert_eq!(lock(&running).cancels.len(), 1);
            assert!(!lock(&running).cancelling);
            // A cancel for a request that is not running cancels nothing.
            worker.handle(Command::Cancel {
                session,
                request: RequestId(3),
            });
            assert_eq!(lock(&running).cancels.len(), 1);
            assert!(lock(&running).skip.contains(&RequestId(3)));
        });
    }

    /// A clock that stands still while a task runs and jumps to the next
    /// wait when all of them wait: each wait is exactly as long as asked.
    fn paused() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .start_paused(true)
            .build()
            .unwrap()
    }

    /// A cancel that is over at once and notes when it was sent, so that
    /// only the waits separate one from the next.
    fn noting(
        sent: &Arc<Mutex<Vec<tokio::time::Instant>>>,
    ) -> impl Fn() -> std::future::Ready<()> + Send + 'static + use<> {
        let sent = Arc::clone(sent);
        move || {
            lock(&sent).push(tokio::time::Instant::now());
            std::future::ready(())
        }
    }

    /// When each of `sent` was, in milliseconds after `start`.
    fn sent_at(sent: &Mutex<Vec<tokio::time::Instant>>, start: tokio::time::Instant) -> Vec<u128> {
        let sent = lock(sent);
        sent.iter().map(|at| (*at - start).as_millis()).collect()
    }

    #[test]
    fn a_users_cancel_is_sent_until_the_script_is_finishing() {
        paused().block_on(async {
            let stop = StopFlag::new();
            let running = Arc::new(Mutex::new(running_script(&stop)));
            let sent = Arc::new(Mutex::new(Vec::new()));
            let start = tokio::time::Instant::now();
            cancel_request(&running, RequestId(2), noting(&sent));
            assert!(stop.is_stopped());
            // The first may reach the server between two of the script's
            // queries, where it is lost: more follow, further apart.
            tokio::time::sleep(Duration::from_millis(1_200)).await;
            assert_eq!(sent_at(&sent, start), [0, 500]);
            // Cancel pressed again: one more goes out at once, not when
            // the wait is over (seconds, later on). The cancels already
            // going out keep their pace, and no second row of them starts
            // beside the first.
            cancel_request(&running, RequestId(2), noting(&sent));
            tokio::time::sleep(Duration::from_millis(2_800)).await;
            assert_eq!(sent_at(&sent, start), [0, 500, 1_200, 1_500, 3_500]);
            // The driver begins its cleanup: a cancel would interrupt it,
            // whether it is the next of the row or one the user asks for.
            stop.finish();
            cancel_request(&running, RequestId(2), noting(&sent));
            tokio::time::sleep(Duration::from_secs(60)).await;
            assert_eq!(sent_at(&sent, start).len(), 5);
            // Each was kept for the session to wait for before its next
            // command: the last is still there.
            assert_eq!(lock(&running).cancels.len(), 1);
            assert!(lock(&running).skip.is_empty());
        });
    }

    #[test]
    fn a_users_cancel_of_anything_but_a_script_is_sent_once() {
        paused().block_on(async {
            let other = Running {
                request: Some(RequestId(2)),
                ..Running::default()
            };
            let running = Arc::new(Mutex::new(other));
            let sent = Arc::new(Mutex::new(Vec::new()));
            let start = tokio::time::Instant::now();
            cancel_request(&running, RequestId(2), noting(&sent));
            assert_eq!(lock(&running).cancels.len(), 1);
            assert!(!lock(&running).cancelling);
            tokio::time::sleep(Duration::from_secs(60)).await;
            assert_eq!(sent_at(&sent, start), [0]);
            // A request that is not running takes none: it is skipped.
            cancel_request(&running, RequestId(3), noting(&sent));
            tokio::time::sleep(Duration::from_secs(60)).await;
            assert_eq!(sent_at(&sent, start), [0]);
            assert!(lock(&running).skip.contains(&RequestId(3)));
        });
    }

    #[test]
    fn a_users_cancels_end_with_their_script() {
        paused().block_on(async {
            let stop = StopFlag::new();
            let running = Arc::new(Mutex::new(running_script(&stop)));
            let sent = Arc::new(Mutex::new(Vec::new()));
            let start = tokio::time::Instant::now();
            cancel_request(&running, RequestId(2), noting(&sent));
            tokio::time::sleep(Duration::from_millis(100)).await;
            assert_eq!(sent_at(&sent, start), [0]);
            // The script ends and the session starts the next one, as
            // `run_session` does, while the next cancel is still waiting.
            let next = StopFlag::new();
            {
                let mut running = lock(&running);
                running.cancelling = false;
                running.request = Some(RequestId(3));
                running.stop = Some(next.clone());
            }
            tokio::time::sleep(Duration::from_secs(60)).await;
            assert_eq!(sent_at(&sent, start), [0], "none for the next script");
            assert!(!next.is_stopped());
            assert!(!lock(&running).cancelling);
            // A Cancel for the next script sends that one's own.
            let again = tokio::time::Instant::now();
            lock(&sent).clear();
            cancel_request(&running, RequestId(3), noting(&sent));
            assert!(next.is_stopped());
            tokio::time::sleep(Duration::from_millis(1_200)).await;
            assert_eq!(sent_at(&sent, again), [0, 500]);
            next.finish();
        });
    }

    #[test]
    fn only_the_first_to_stop_a_script_keeps_cancelling_it() {
        let stop = StopFlag::new();
        let mut script = running_script(&stop);
        assert!(script.start_cancelling());
        assert!(!script.start_cancelling());
        // Nobody does for a script that is cleaning up, or without one.
        let stop = StopFlag::new();
        stop.finish();
        assert!(!running_script(&stop).start_cancelling());
        assert!(!Running::default().start_cancelling());
    }

    #[test]
    fn a_timer_cancels_until_the_script_is_finishing() {
        one_thread().block_on(async {
            let (_dir, connection) = sqlite_connection().await;
            let stop = StopFlag::new();
            let running = Arc::new(Mutex::new(running_script(&stop)));
            let after = Duration::from_millis(50);
            let timer = Timer::start(
                RequestId(2),
                after,
                stop.clone(),
                connection.cancel_handle(),
                Arc::clone(&running),
            );
            assert!(!stop.is_stopped());
            cancels_sent(&running).await;
            assert!(stop.is_stopped());
            // Again: the first may have reached the server too early.
            cancels_sent(&running).await;
            // The driver begins its cleanup: a cancel would interrupt it.
            stop.finish();
            tokio::time::sleep(THIRD_CANCEL).await;
            assert!(lock(&running).cancels.is_empty());
            assert_eq!(timer.end().await, Some(after));
        });
    }

    #[test]
    fn a_timer_ended_early_is_gone_and_did_nothing() {
        one_thread().block_on(async {
            let (_dir, connection) = sqlite_connection().await;
            let stop = StopFlag::new();
            let running = Arc::new(Mutex::new(running_script(&stop)));
            let timer = Timer::start(
                RequestId(2),
                Duration::from_secs(3600),
                stop.clone(),
                connection.cancel_handle(),
                Arc::clone(&running),
            );
            // The timer's task starts, and holds the session's state.
            tokio::time::sleep(Duration::from_millis(20)).await;
            assert_eq!(Arc::strong_count(&running), 2);
            assert_eq!(timer.end().await, None);
            // Its task is gone, not merely told to end: nothing is left
            // that could send a cancel.
            assert_eq!(Arc::strong_count(&running), 1);
            assert!(!stop.is_stopped());
            assert!(lock(&running).cancels.is_empty());
        });
    }

    #[test]
    fn a_timer_does_not_claim_a_script_the_user_stopped() {
        one_thread().block_on(async {
            let (_dir, connection) = sqlite_connection().await;
            let stop = StopFlag::new();
            let running = Arc::new(Mutex::new(running_script(&stop)));
            // The user cancels just before the time is up, and the user's
            // cancels are being sent.
            assert!(stop.stop());
            assert!(lock(&running).start_cancelling());
            let after = Duration::from_millis(50);
            let timer = Timer::start(
                RequestId(2),
                after,
                stop.clone(),
                connection.cancel_handle(),
                Arc::clone(&running),
            );
            // The timer adds none of its own to them.
            tokio::time::sleep(after * 4).await;
            assert!(lock(&running).cancels.is_empty());
            assert_eq!(timer.end().await, None);
            assert_eq!(
                cancel_reason(&stop, None, &Ok(never_ran())),
                Some(CancelReason::User)
            );
        });
    }

    #[test]
    fn the_next_cancel_waits_for_the_one_before_and_then_a_while() {
        use std::sync::atomic::AtomicUsize;
        one_thread().block_on(async {
            let stop = StopFlag::new();
            let running = Arc::new(Mutex::new(running_script(&stop)));
            let sent = Arc::new(AtomicUsize::new(0));
            let answer = Arc::new(tokio::sync::Notify::new());
            // A cancel that takes until the test lets it end, like one to
            // a server that is slow to answer.
            let send = {
                let (sent, answer) = (Arc::clone(&sent), Arc::clone(&answer));
                move || {
                    let (sent, answer) = (Arc::clone(&sent), Arc::clone(&answer));
                    async move {
                        sent.fetch_add(1, Ordering::SeqCst);
                        answer.notified().await;
                    }
                }
            };
            let cancelling = tokio::spawn(keep_sending(send, Arc::clone(&running), RequestId(2)));
            // However long the first takes, no second one is sent beside it.
            tokio::time::sleep(THIRD_CANCEL).await;
            assert_eq!(sent.load(Ordering::SeqCst), 1);
            // Once it is over the next follows, but not at once.
            let answered = std::time::Instant::now();
            answer.notify_one();
            while sent.load(Ordering::SeqCst) < 2 {
                assert!(answered.elapsed() < WAIT, "a second cancel must be sent");
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            assert!(answered.elapsed() >= CANCEL_AGAIN / 2);
            // The script reaches its cleanup: that was the last cancel.
            stop.finish();
            answer.notify_one();
            assert!(tokio::time::timeout(WAIT, cancelling).await.is_ok());
            assert_eq!(sent.load(Ordering::SeqCst), 2);
        });
    }

    #[test]
    fn cancels_are_sent_further_and_further_apart() {
        let mut pause = CANCEL_AGAIN;
        let mut pauses = vec![pause];
        for _ in 0..5 {
            pause = longer(pause);
            pauses.push(pause);
        }
        let millis: Vec<u128> = pauses.iter().map(Duration::as_millis).collect();
        assert_eq!(millis, [500, 1000, 2000, 4000, 5000, 5000]);
    }

    #[test]
    fn the_waits_between_cancels_grow_while_they_are_sent() {
        // A paused clock: the waits pass at once, and each is exactly as
        // long as the loop asked for.
        paused().block_on(async {
            let stop = StopFlag::new();
            let running = Arc::new(Mutex::new(running_script(&stop)));
            let sent = Arc::new(Mutex::new(Vec::new()));
            // A cancel that is over at once, so that only the waits
            // separate one from the next.
            let send = {
                let sent = Arc::clone(&sent);
                move || {
                    lock(&sent).push(tokio::time::Instant::now());
                    async {}
                }
            };
            let cancelling = tokio::spawn(keep_sending(send, Arc::clone(&running), RequestId(2)));
            // Long enough for six cancels, and into the wait after the last.
            tokio::time::sleep(Duration::from_millis(12_600)).await;
            // The script reaches its cleanup: no cancel follows.
            stop.finish();
            assert!(tokio::time::timeout(WAIT, cancelling).await.is_ok());
            let sent = lock(&sent);
            let waits: Vec<u128> = sent
                .windows(2)
                .map(|pair| (pair[1] - pair[0]).as_millis())
                .collect();
            assert_eq!(waits, [500, 1000, 2000, 4000, 5000]);
        });
    }

    #[test]
    fn a_session_that_ends_leaves_nothing_to_cancel() {
        let stop = StopFlag::new();
        let running = Arc::new(Mutex::new(running_script(&stop)));
        assert!(lock(&running).script_takes_a_cancel());
        assert!(lock(&running).start_cancelling());
        let (_queue, commands) = tokio_mpsc::unbounded_channel();
        let (outbox, _received) = quiet_outbox();
        drop(SessionEnd::new(
            SessionId(1),
            Arc::clone(&running),
            commands,
            outbox,
        ));
        let running = lock(&running);
        assert!(!running.script_takes_a_cancel());
        assert!(!running.cancelling);
        assert!(running.closed);
        assert_eq!(running.request, None);
    }

    #[test]
    fn a_session_whose_queue_ends_says_that_it_is_over() {
        let (outbox, received) = quiet_outbox();
        runtime().block_on(async {
            let (_dir, connection) = sqlite_connection().await;
            let (queue, commands) = tokio_mpsc::unbounded_channel();
            // Kept to the end: it is the queue that ends this session.
            let (_stop, stopped) = tokio::sync::oneshot::channel();
            // As if a script were running: what a session that ends under
            // one would leave behind for the cancels to go on with.
            let stop = StopFlag::new();
            let running = Arc::new(Mutex::new(running_script(&stop)));
            let task = tokio::spawn(run_session(
                SessionId(1),
                connection,
                commands,
                stopped,
                Arc::clone(&running),
                outbox,
            ));
            drop(queue);
            tokio::time::timeout(WAIT, task)
                .await
                .expect("the session must end with its queue")
                .unwrap();
            let running = lock(&running);
            assert!(running.closed);
            assert_eq!(running.request, None);
            assert!(running.stop.is_none());
            assert!(!running.script_takes_a_cancel());
        });
        // It ended in order: nothing was lost, so nothing is reported.
        let said: Vec<Event> = received.try_iter().collect();
        assert!(said.is_empty(), "{said:?}");
    }

    /// Starts a session with a count (request 2) and a listing (request 3)
    /// on its queue, in the state `running`.
    async fn session_with_two_requests(
        running: Running,
        count: RowQuery,
        outbox: Outbox,
    ) -> (
        tempfile::TempDir,
        SessionHandle,
        tokio::task::JoinHandle<()>,
    ) {
        let (dir, connection) = sqlite_connection().await;
        let session = SessionId(1);
        let (handle, commands, stopped) = session_handle(&connection, running);
        handle
            .queue
            .send(Command::CountRows {
                session,
                request: RequestId(2),
                query: count,
            })
            .unwrap();
        handle
            .queue
            .send(Command::ListSchemas {
                session,
                request: RequestId(3),
            })
            .unwrap();
        let task = tokio::spawn(run_session(
            session,
            connection,
            commands,
            stopped,
            Arc::clone(&handle.running),
            outbox,
        ));
        (dir, handle, task)
    }

    /// Waits until the session of `handle` runs `request`.
    async fn runs(handle: &SessionHandle, request: RequestId) {
        let deadline = std::time::Instant::now() + WAIT;
        while lock(&handle.running).request != Some(request) {
            assert!(
                std::time::Instant::now() < deadline,
                "the session must start {request:?}"
            );
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    }

    /// What a session that stopped while it ran request 2, with request 3
    /// queued, must have said, and what it must have left behind.
    fn assert_answered_as_lost(handle: &SessionHandle, said: &[Event]) {
        assert!(
            matches!(
                said,
                [
                    Event::Count {
                        request: RequestId(2),
                        result: Err(Error::ConnectionLost(_)),
                        ..
                    },
                    Event::Schemas {
                        request: RequestId(3),
                        result: Err(Error::ConnectionLost(_)),
                        ..
                    },
                    Event::Disconnected {
                        session: SessionId(1),
                        error: Error::ConnectionLost(_),
                    },
                ]
            ),
            "{said:?}"
        );
        // Later commands fail at the sender, so the worker answers them.
        assert!(
            handle
                .queue
                .send(Command::ListSchemas {
                    session: SessionId(1),
                    request: RequestId(4),
                })
                .is_err()
        );
        let running = lock(&handle.running);
        assert!(running.closed);
        assert_eq!(running.request, None);
    }

    #[test]
    fn a_session_that_panics_answers_its_requests_and_is_disconnected() {
        let (outbox, received) = quiet_outbox();
        runtime().block_on(async {
            let panics = Running {
                panics: Some(RequestId(2)),
                ..Running::default()
            };
            let count = RowQuery::new(ObjectRef::new("main", "users"), 10);
            let (_dir, handle, task) = session_with_two_requests(panics, count, outbox).await;
            let ended = tokio::time::timeout(WAIT, task)
                .await
                .expect("the session must end");
            assert!(ended.is_err_and(|error| error.is_panic()));
            let said: Vec<Event> = received.try_iter().collect();
            assert_answered_as_lost(&handle, &said);
        });
    }

    #[test]
    fn a_session_that_is_dropped_answers_its_requests_and_is_disconnected() {
        let (outbox, received) = quiet_outbox();
        runtime().block_on(async {
            let (_dir, handle, task) =
                session_with_two_requests(Running::default(), slow_count(100_000), outbox).await;
            runs(&handle, RequestId(2)).await;
            task.abort();
            let ended = tokio::time::timeout(WAIT, task)
                .await
                .expect("the session must end");
            assert!(ended.is_err_and(|error| error.is_cancelled()));
            let said: Vec<Event> = received.try_iter().collect();
            assert_answered_as_lost(&handle, &said);
        });
    }

    #[test]
    fn a_session_the_worker_let_go_of_is_dropped_in_silence() {
        let (outbox, received) = quiet_outbox();
        runtime().block_on(async {
            let (_dir, handle, task) =
                session_with_two_requests(Running::default(), slow_count(100_000), outbox).await;
            runs(&handle, RequestId(2)).await;
            // A Close, or the backend shutting down: no tab waits for this
            // session any more.
            drop(handle);
            task.abort();
            let ended = tokio::time::timeout(WAIT, task)
                .await
                .expect("the session must end");
            assert!(ended.is_err_and(|error| error.is_cancelled()));
        });
        let said: Vec<Event> = received.try_iter().collect();
        assert!(said.is_empty(), "{said:?}");
    }

    #[test]
    fn a_refused_script_runs_nothing_and_keeps_the_session() {
        let (_dir, mut backend, session) = connected_sqlite();
        backend.send(Command::RunSql {
            session,
            request: RequestId(2),
            statements: statements("SELECT 1;\nCOMMIT"),
            limit: 10,
            timeout: Some(Duration::from_secs(3600)),
        });
        let (_, result, cancel) = sql_ran(&mut backend);
        assert_eq!(cancel, None);
        assert!(
            matches!(result, Err(Error::Refused { line: 2, .. })),
            "{result:?}"
        );
        backend.send(Command::ListSchemas {
            session,
            request: RequestId(3),
        });
        match backend.wait(WAIT) {
            Some(Event::Schemas { result: Ok(_), .. }) => {}
            other => panic!("expected the schemas, got {other:?}"),
        }
    }

    fn cancelled_outcome() -> ScriptOutcome {
        ScriptOutcome {
            results: vec![tabletist_db::StatementResult {
                elapsed: Duration::ZERO,
                outcome: tabletist_db::StatementOutcome::Cancelled,
            }],
            stopped: true,
        }
    }

    #[test]
    fn a_cancel_reason_needs_a_stop_that_ended_the_run() {
        let after = Duration::from_secs(30);
        let stopped = StopFlag::new();
        stopped.stop();
        let cancelled = Ok(cancelled_outcome());
        assert_eq!(
            cancel_reason(&stopped, None, &cancelled),
            Some(CancelReason::User)
        );
        assert_eq!(
            cancel_reason(&stopped, Some(after), &cancelled),
            Some(CancelReason::Timeout(after))
        );
        // Stopped before the first statement: no results, still cancelled.
        assert_eq!(
            cancel_reason(&stopped, None, &Ok(never_ran())),
            Some(CancelReason::User)
        );
        assert_eq!(
            cancel_reason(&stopped, None, &Err(Error::Cancelled)),
            Some(CancelReason::User)
        );
        // The stop came too late: the script had already ended by itself.
        assert_eq!(
            cancel_reason(&stopped, Some(after), &Ok(ScriptOutcome::default())),
            None
        );
        assert_eq!(
            cancel_reason(&stopped, None, &Err(Error::LeftReadOnly)),
            None
        );
        // Cancelled by the server (a statement_timeout, say), not by a stop.
        assert_eq!(cancel_reason(&StopFlag::new(), None, &cancelled), None);
        assert_eq!(
            cancel_reason(&StopFlag::new(), None, &Err(Error::Cancelled)),
            None
        );
    }

    #[test]
    fn a_cancel_is_not_sent_to_a_script_that_is_cleaning_up() {
        // Not a script: there is none to keep cancelling.
        let other = Running::default();
        other.stop_script();
        assert!(!other.script_takes_a_cancel());

        let stop = StopFlag::new();
        let script = running_script(&stop);
        assert!(script.script_takes_a_cancel());
        assert!(!stop.is_stopped());
        script.stop_script();
        assert!(stop.is_stopped());
        assert!(script.script_takes_a_cancel());

        // The driver has begun its cleanup: the flag is still set (it says
        // who stopped the run), but no cancel goes out.
        let stop = StopFlag::new();
        let finishing = running_script(&stop);
        stop.finish();
        finishing.stop_script();
        assert!(stop.is_stopped());
        assert!(!finishing.script_takes_a_cancel());
    }

    #[test]
    fn a_script_that_loses_the_session_counts_as_lost() {
        let left: Result<ScriptOutcome, Error> = Err(Error::LeftReadOnly);
        assert_eq!(lost_error(&left), Some(Error::LeftReadOnly));
        let cleanup: Result<ScriptOutcome, Error> = Err(Error::ConnectionLost(
            "could not end the read-only transaction".into(),
        ));
        assert!(lost_error(&cleanup).is_some());
        // The session stays usable after these.
        for kept in [
            Error::Cancelled,
            Error::Unsupported("too old"),
            Error::Refused {
                line: 1,
                what: "COMMIT".into(),
            },
        ] {
            assert_eq!(lost_error(&Err::<ScriptOutcome, _>(kept)), None);
        }
        assert_eq!(lost_error(&Ok(cancelled_outcome())), None);
    }

    #[test]
    fn a_lost_connection_answers_queued_scripts() {
        let (sender, mut commands) = tokio_mpsc::unbounded_channel();
        let (outbox, received) = quiet_outbox();
        let session = SessionId(1);
        sender
            .send(Command::RunSql {
                session,
                request: RequestId(2),
                statements: statements("SELECT 1"),
                limit: 10,
                timeout: Some(Duration::from_secs(30)),
            })
            .unwrap();
        sender
            .send(Command::ServerVersion {
                session,
                request: RequestId(3),
            })
            .unwrap();
        fail_queued(&mut commands, &outbox, &Error::LeftReadOnly);
        let answered: Vec<Event> = received.try_iter().collect();
        assert!(
            matches!(
                answered.as_slice(),
                [
                    Event::SqlRan {
                        request: RequestId(2),
                        result: Err(Error::LeftReadOnly),
                        cancel: None,
                        ..
                    },
                    Event::ServerVersion {
                        request: RequestId(3),
                        result: Err(Error::LeftReadOnly),
                        ..
                    },
                ]
            ),
            "{answered:?}"
        );
    }

    #[test]
    fn scripts_and_version_requests_belong_to_their_session() {
        let script = Command::RunSql {
            session: SessionId(7),
            request: RequestId(2),
            statements: statements("SELECT 1"),
            limit: 10,
            timeout: None,
        };
        let version = Command::ServerVersion {
            session: SessionId(7),
            request: RequestId(3),
        };
        assert_eq!(session_of(&script), SessionId(7));
        assert_eq!(request_of(&script), Some(RequestId(2)));
        assert_eq!(session_of(&version), SessionId(7));
        assert_eq!(request_of(&version), Some(RequestId(3)));
    }

    #[test]
    fn the_server_version_is_asked_for() {
        let (_dir, mut backend, session) = connected_sqlite();
        backend.send(Command::ServerVersion {
            session,
            request: RequestId(2),
        });
        match backend.wait(WAIT) {
            Some(Event::ServerVersion {
                request: RequestId(2),
                result: Ok(version),
                ..
            }) => assert!(version.starts_with("SQLite "), "{version}"),
            other => panic!("expected the server's version, got {other:?}"),
        }
    }

    #[test]
    fn a_closed_session_fails_a_script() {
        let (_dir, mut backend, session) = connected_sqlite();
        backend.send(Command::Close { session });
        backend.send(Command::RunSql {
            session,
            request: RequestId(2),
            statements: statements("SELECT 1"),
            limit: 10,
            timeout: None,
        });
        let (_, result, cancel) = sql_ran(&mut backend);
        assert_eq!(cancel, None);
        assert!(result.unwrap_err().is_connection_lost());
    }

    #[test]
    fn settings_are_saved_as_a_state_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config").join("settings.toml");
        let settings = crate::settings::Settings {
            page_size: 50,
            ..Default::default()
        };
        let mut backend = Backend::start_with(Waker::default(), Keyring::memory());
        backend.send(Command::Save {
            path: path.clone(),
            file: StateFile::Settings(settings.clone()),
        });
        match backend.wait(WAIT) {
            Some(Event::Saved { result: Ok(()), .. }) => {}
            other => panic!("expected the save to be written, got {other:?}"),
        }
        let text = std::fs::read_to_string(&path).unwrap();
        assert_eq!(text, settings.to_toml());
        assert_eq!(
            crate::settings::Settings::from_toml(&text).settings,
            settings
        );
    }

    #[test]
    fn a_state_file_saved_through_a_symbolic_link_keeps_the_link() {
        if !crate::util::can_symlink() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let (config, dotfiles) = config_and_dotfiles(dir.path());
        let mut backend = Backend::start_with(Waker::default(), Keyring::memory());
        let files = [
            (
                "settings.toml",
                StateFile::Settings(crate::settings::Settings::default()),
            ),
            (
                "connections.json",
                StateFile::Connections(SavedConnections::default()),
            ),
            (
                "known_hosts.json",
                StateFile::KnownHosts(HostKeys::default()),
            ),
        ];
        for (name, file) in files {
            // As a dotfiles manager leaves it: the file in its repository
            // and a link in the config directory.
            let (path, target) = (config.join(name), dotfiles.join(name));
            std::fs::write(&target, "from the repository").unwrap();
            crate::util::symlink_file(&target, &path);
            backend.send(Command::Save {
                path: path.clone(),
                file,
            });
            match backend.wait(WAIT) {
                Some(Event::Saved { result: Ok(()), .. }) => {}
                other => panic!("expected {name} to be written, got {other:?}"),
            }
            assert_eq!(std::fs::read_link(&path).unwrap(), target, "{name}");
            assert!(!target.symlink_metadata().unwrap().file_type().is_symlink());
            assert_ne!(
                std::fs::read_to_string(&target).unwrap(),
                "from the repository",
                "{name} is written where the link leads"
            );
        }
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
        wait_until_running(&backend, session, RequestId(2), false);
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
    fn script_commands_do_not_print_their_sql() {
        let command = Command::RunSql {
            session: SessionId(1),
            request: RequestId(2),
            statements: statements("SELECT 1;\nSELECT 'hunter2' FROM payroll"),
            limit: 10,
            timeout: None,
        };
        let printed = format!("{command:?}");
        for typed in ["hunter2", "payroll", "SELECT"] {
            assert!(!printed.contains(typed), "{printed}");
        }
        // What may be logged is still there: the request and the count.
        assert!(printed.contains("RequestId(2)"), "{printed}");
        assert_eq!(printed.matches("Statement {").count(), 2, "{printed}");
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

    /// Starts a backend watching `path`, and says whether it could.
    fn watching(path: &std::path::Path) -> (Backend, bool) {
        let mut backend = Backend::start_with(Waker::default(), Keyring::memory());
        backend.send(Command::WatchSettings {
            path: path.to_path_buf(),
        });
        match backend.wait(WAIT) {
            Some(Event::SettingsWatch { live }) => (backend, live),
            other => panic!("expected to hear whether the file is watched, got {other:?}"),
        }
    }

    /// The texts of the settings file the backend sends until `expected`
    /// comes, that one included; empty when it never does.
    fn texts_until(backend: &mut Backend, expected: &str) -> Vec<String> {
        let deadline = std::time::Instant::now() + WAIT;
        let mut texts = Vec::new();
        while std::time::Instant::now() < deadline {
            if let Some(Event::SettingsFile { text, .. }) = backend.wait(Duration::from_millis(200))
            {
                let done = text == expected;
                texts.push(text);
                if done {
                    return texts;
                }
            }
        }
        Vec::new()
    }

    /// The settings file's texts the backend sends until `expected` comes,
    /// that one included, each with whether it is the backend's own write;
    /// empty when it never does.
    fn files_until(backend: &mut Backend, expected: &str) -> Vec<(String, bool)> {
        let deadline = std::time::Instant::now() + WAIT;
        let mut files = Vec::new();
        while std::time::Instant::now() < deadline {
            if let Some(Event::SettingsFile { text, own }) =
                backend.wait(Duration::from_millis(200))
            {
                let done = text == expected;
                files.push((text, own));
                if done {
                    return files;
                }
            }
        }
        Vec::new()
    }

    #[test]
    fn the_settings_file_is_read_when_it_changes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.toml");
        let (mut backend, live) = watching(&path);
        assert!(live);
        std::fs::write(&path, "[data]\npage_size = 500\n").unwrap();
        assert!(!texts_until(&mut backend, "[data]\npage_size = 500\n").is_empty());
        // An editor that saves by renaming another file over it.
        crate::util::write_atomic(&path, b"[data]\npage_size = 100\n").unwrap();
        assert!(!texts_until(&mut backend, "[data]\npage_size = 100\n").is_empty());
        // And one that writes in place. The first write above was that
        // too, but so soon after the watch began that the read it starts
        // with may have been the one to find it.
        std::fs::write(&path, "[data]\npage_size = 300\n").unwrap();
        assert!(!texts_until(&mut backend, "[data]\npage_size = 300\n").is_empty());
    }

    #[test]
    fn the_settings_file_is_read_when_the_watch_starts() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.toml");
        // Saved after the app loaded its settings and before it watched
        // the file: no change follows to say so.
        std::fs::write(&path, PAGE_500).unwrap();
        let (mut backend, live) = watching(&path);
        assert!(live);
        assert_eq!(texts_until(&mut backend, PAGE_500), [PAGE_500]);
        // Once: the watch starting is not a change of the file.
        assert!(backend.wait(Duration::from_millis(600)).is_none());
    }

    #[test]
    fn the_settings_file_settles_from_its_last_change() {
        paused().block_on(async {
            let (changed, mut changes) = tokio_mpsc::unbounded_channel();
            let start = tokio::time::Instant::now();
            // A save in three steps, 80 ms apart: truncated at 80, written
            // at 160. Read 100 ms after the first, the file would be empty.
            let editor = changed.clone();
            tokio::spawn(async move {
                for _ in 0..3 {
                    let _ = editor.send(());
                    tokio::time::sleep(Duration::from_millis(80)).await;
                }
            });
            assert!(settled(&mut changes).await);
            assert_eq!(start.elapsed(), Duration::from_millis(260));
            // The watcher is gone: there is nothing left to read for.
            drop(changed);
            assert!(!settled(&mut changes).await);
        });
    }

    /// The reader of a settings file whose read number `n`, from 0, answers
    /// at once with `answer(n)`: the channel that wakes it, what it sends,
    /// and when each read was.
    fn reader(
        answer: impl Fn(usize) -> std::io::Result<Found> + Send + 'static,
    ) -> (
        tokio_mpsc::UnboundedSender<()>,
        mpsc::Receiver<Event>,
        Arc<Mutex<Vec<tokio::time::Instant>>>,
    ) {
        slow_reader(move |read| std::future::ready(answer(read)))
    }

    /// As `reader`, with reads that may take their time: read number `n` is
    /// the future `read(n)`, and the file can change while it runs. The
    /// times are those the reads began at.
    fn slow_reader<F>(
        read: impl Fn(usize) -> F + Send + 'static,
    ) -> (
        tokio_mpsc::UnboundedSender<()>,
        mpsc::Receiver<Event>,
        Arc<Mutex<Vec<tokio::time::Instant>>>,
    )
    where
        F: Future<Output = std::io::Result<Found>> + Send + 'static,
    {
        let (outbox, events) = quiet_outbox();
        let (changed, changes) = tokio_mpsc::unbounded_channel();
        let reads = Arc::new(Mutex::new(Vec::new()));
        let noted = Arc::clone(&reads);
        let read = move |_: PathBuf| {
            let mut reads = lock(&noted);
            reads.push(tokio::time::Instant::now());
            read(reads.len() - 1)
        };
        // A plain file that is watched: there is no link to follow.
        let follow = || Ok(());
        let path = PathBuf::from("settings.toml");
        tokio::spawn(read_settings(path, true, changes, outbox, follow, read));
        (changed, events, reads)
    }

    /// What a read answers while an editor holds the file.
    fn held() -> std::io::Error {
        std::io::ErrorKind::PermissionDenied.into()
    }

    /// What a read answers when it finds `text`, put there by someone else.
    fn found(text: &str) -> std::io::Result<Found> {
        Ok(Found {
            bytes: text.into(),
            own: false,
        })
    }

    /// The texts of the settings file sent so far.
    fn texts(events: &mpsc::Receiver<Event>) -> Vec<String> {
        let text = |event| match event {
            Event::SettingsFile { text, .. } => Some(text),
            _ => None,
        };
        events.try_iter().filter_map(text).collect()
    }

    const PAGE_500: &str = "[data]\npage_size = 500\n";

    #[test]
    fn a_settings_file_that_cannot_be_read_is_tried_again() {
        paused().block_on(async {
            // An editor holds the file through two reads, and lets go
            // without another change to say so.
            let (changed, events, reads) = reader(|read| match read {
                0 | 1 => Err(held()),
                _ => found(PAGE_500),
            });
            let start = tokio::time::Instant::now();
            changed.send(()).unwrap();
            tokio::time::sleep(Duration::from_secs(60)).await;
            assert_eq!(texts(&events), [PAGE_500]);
            // Once the change has settled, then after a wait that doubles.
            assert_eq!(sent_at(&reads, start), [100, 200, 400]);
        });
    }

    #[test]
    fn a_settings_file_that_stays_unreadable_is_left_until_it_changes() {
        paused().block_on(async {
            let (changed, events, reads) = reader(|read| match read {
                0..6 => Err(held()),
                _ => found(PAGE_500),
            });
            let start = tokio::time::Instant::now();
            changed.send(()).unwrap();
            tokio::time::sleep(Duration::from_secs(60)).await;
            // Six reads, the waits between them doubling up to a second,
            // and no more: the reader does not ask for ever.
            assert_eq!(sent_at(&reads, start), [100, 200, 400, 800, 1_600, 2_600]);
            assert!(texts(&events).is_empty());
            // The next change is read as any other.
            changed.send(()).unwrap();
            tokio::time::sleep(Duration::from_secs(60)).await;
            assert_eq!(sent_at(&reads, start).len(), 7);
            assert_eq!(texts(&events), [PAGE_500]);
        });
    }

    #[test]
    fn a_settings_file_that_is_gone_is_read_once() {
        paused().block_on(async {
            let (changed, events, reads) = reader(|_| Err(std::io::ErrorKind::NotFound.into()));
            let start = tokio::time::Instant::now();
            changed.send(()).unwrap();
            tokio::time::sleep(Duration::from_secs(60)).await;
            // Waiting does not bring a deleted file back.
            assert_eq!(sent_at(&reads, start), [100]);
            assert!(texts(&events).is_empty());
        });
    }

    #[test]
    fn a_change_while_a_read_waits_starts_over() {
        paused().block_on(async {
            let (changed, events, reads) = reader(|read| match read {
                0..4 => Err(held()),
                _ => found(PAGE_500),
            });
            let start = tokio::time::Instant::now();
            changed.send(()).unwrap();
            // The third read has failed, and the fourth is due at 800.
            tokio::time::sleep(Duration::from_millis(500)).await;
            changed.send(()).unwrap();
            tokio::time::sleep(Duration::from_secs(60)).await;
            // The file changed again: it is left to settle, and the waits
            // begin again from the shortest.
            assert_eq!(sent_at(&reads, start), [100, 200, 400, 600, 700]);
            assert_eq!(texts(&events), [PAGE_500]);
        });
    }

    /// A read that takes 50 ms to answer with `bytes`, another's.
    async fn after_50_ms(bytes: Vec<u8>) -> std::io::Result<Found> {
        tokio::time::sleep(Duration::from_millis(50)).await;
        Ok(Found { bytes, own: false })
    }

    #[test]
    fn a_read_overtaken_by_a_change_is_not_sent() {
        paused().block_on(async {
            // An editor truncates the file and writes it again while the
            // first read runs: that read finds the file empty.
            let (changed, events, reads) = slow_reader(|read| {
                after_50_ms(match read {
                    0 => Vec::new(),
                    _ => PAGE_500.into(),
                })
            });
            let start = tokio::time::Instant::now();
            changed.send(()).unwrap();
            // The first read began at 100 and answers at 150.
            tokio::time::sleep(Duration::from_millis(120)).await;
            changed.send(()).unwrap();
            tokio::time::sleep(Duration::from_secs(60)).await;
            // What it found is dropped when it answers, and the file is
            // read again once it has been left alone since.
            assert_eq!(sent_at(&reads, start), [100, 250]);
            assert_eq!(texts(&events), [PAGE_500]);
        });
    }

    #[test]
    fn a_read_that_outlasts_the_watch_is_sent() {
        paused().block_on(async {
            let (changed, events, reads) = slow_reader(|_| after_50_ms(PAGE_500.into()));
            let start = tokio::time::Instant::now();
            changed.send(()).unwrap();
            tokio::time::sleep(Duration::from_millis(120)).await;
            // The end of the watch is not a change of the file.
            drop(changed);
            tokio::time::sleep(Duration::from_secs(60)).await;
            assert_eq!(sent_at(&reads, start), [100]);
            assert_eq!(texts(&events), [PAGE_500]);
        });
    }

    #[test]
    fn a_read_that_waits_ends_with_the_watch() {
        paused().block_on(async {
            let (changed, events, reads) = reader(|_| Err(held()));
            let start = tokio::time::Instant::now();
            changed.send(()).unwrap();
            tokio::time::sleep(Duration::from_millis(150)).await;
            drop(changed);
            tokio::time::sleep(Duration::from_secs(60)).await;
            assert_eq!(sent_at(&reads, start), [100]);
            assert!(texts(&events).is_empty());
        });
    }

    #[test]
    fn only_a_change_of_the_settings_file_wakes_the_reader() {
        use notify::event::{AccessKind, CreateKind, ModifyKind, RenameMode};
        use notify::{Event, EventKind};
        let own = [OsString::from("settings.toml")];
        let name = &own[..];
        let at = |kind: EventKind, path: &str| Event::new(kind).add_path(path.into());
        let data = EventKind::Modify(ModifyKind::Any);
        assert!(concerns(&at(data, "/config/settings.toml"), name));
        assert!(concerns(
            &at(EventKind::Create(CreateKind::File), "/config/settings.toml"),
            name
        ));
        // A rename carries both names: the temporary file's and ours.
        let renamed = Event::new(EventKind::Modify(ModifyKind::Name(RenameMode::Both)))
            .add_path("/config/.tabletist-x.tmp".into())
            .add_path("/private/config/settings.toml".into());
        assert!(concerns(&renamed, name));
        // Another file of the directory, and a look at ours: Linux reports
        // every open, and the reader's own read is one.
        assert!(!concerns(&at(data, "/config/connections.json"), name));
        assert!(!concerns(
            &at(EventKind::Access(AccessKind::Any), "/config/settings.toml"),
            name
        ));
        assert!(!concerns(&at(data, "/config/settings.toml"), &[]));
        // Behind a symbolic link the file has a second name, elsewhere.
        let linked = [own[0].clone(), OsString::from("tabletist.toml")];
        assert!(concerns(&at(data, "/dotfiles/tabletist.toml"), &linked));
        assert!(concerns(&at(data, "/config/settings.toml"), &linked));
        assert!(!concerns(&at(data, "/dotfiles/tabletist.toml"), name));
        assert!(!concerns(&at(data, "/dotfiles/README.md"), &linked));
    }

    #[test]
    fn a_settings_file_kept_in_another_case_is_the_same_where_case_does_not_count() {
        use notify::event::{AccessKind, ModifyKind};
        use notify::{Event, EventKind};
        let name = std::ffi::OsStr::new("settings.toml");
        let names = [name.to_owned()];
        let at = |kind: EventKind, path: &str| Event::new(kind).add_path(path.into());
        let data = EventKind::Modify(ModifyKind::Any);
        // macOS and Windows open `Settings.toml` under the lower-case name
        // the app asks for, and report its changes as it is spelled.
        let folds = cfg!(any(windows, target_os = "macos"));
        assert_eq!(concerns(&at(data, "/config/Settings.toml"), &names), folds);
        assert_eq!(concerns(&at(data, "/config/SETTINGS.TOML"), &names), folds);
        // Another file is another file in any case, and a look is a look.
        assert!(!concerns(&at(data, "/config/Settings.json"), &names));
        assert!(!concerns(
            &at(EventKind::Access(AccessKind::Any), "/config/Settings.toml"),
            &names
        ));
        assert!(same_file(name, std::ffi::OsStr::new("settings.toml")));
    }

    #[test]
    fn a_name_in_another_case_is_the_same_beyond_ascii_too() {
        use notify::event::ModifyKind;
        use notify::{Event, EventKind};
        let folds = cfg!(any(windows, target_os = "macos"));
        let name = OsString::from;
        assert_eq!(same_file(&name("Ä.toml"), &name("ä.toml")), folds);
        assert_eq!(
            same_file(&name("НАСТРОЙКИ.toml"), &name("настройки.toml")),
            folds
        );
        assert!(same_file(&name("ä.toml"), &name("ä.toml")));
        assert!(!same_file(&name("ä.toml"), &name("a.toml")));
        // A link spells the file it leads to in one case, and the file is
        // kept, and its changes reported, in another.
        let linked = [name("settings.toml"), name("Ä.toml")];
        let change = Event::new(EventKind::Modify(ModifyKind::Any));
        let change = change.add_path("/dotfiles/ä.toml".into());
        assert_eq!(concerns(&change, &linked), folds);
    }

    #[test]
    fn names_beyond_ascii_that_cannot_be_told_apart_are_taken_for_one() {
        use notify::event::ModifyKind;
        use notify::{Event, EventKind};
        let folds = cfg!(any(windows, target_os = "macos"));
        let name = OsString::from;
        // Windows compares names by their upper case, which the two small
        // sigmas share. Their lower cases differ.
        assert_eq!(same_file(&name("Σ.toml"), &name("ς.toml")), folds);
        assert_eq!(same_file(&name("σ.toml"), &name("ς.toml")), folds);
        let linked = [name("settings.toml"), name("Σ.toml")];
        let change = Event::new(EventKind::Modify(ModifyKind::Any));
        let change = change.add_path("/dotfiles/ς.toml".into());
        assert_eq!(concerns(&change, &linked), folds);
        // macOS takes a letter with its accent for the same however the
        // two are written: as one character, or as two.
        assert_eq!(
            same_file(&name("\u{e4}.toml"), &name("a\u{308}.toml")),
            folds
        );
        // A name within ASCII is no name beyond it, and the app's own file
        // is not read for every such name in its directory.
        assert!(!same_file(&name("settings.toml"), &name("настройки.toml")));
        assert!(!same_file(&name("a.toml"), &name("\u{e4}.toml")));
    }

    #[test]
    fn a_watched_directory_that_is_moved_or_removed_is_told() {
        use notify::event::{AccessKind, ModifyKind, RemoveKind, RenameMode};
        use notify::{Event, EventKind};
        let watched = [PathBuf::from("/config"), PathBuf::from("/dotfiles")];
        let at = |kind: EventKind, path: &str| Event::new(kind).add_path(path.into());
        let moved = EventKind::Modify(ModifyKind::Name(RenameMode::From));
        let removed = EventKind::Remove(RemoveKind::Folder);
        assert!(leaves(&at(moved, "/dotfiles"), &watched));
        assert!(leaves(&at(removed, "/dotfiles"), &watched));
        assert!(leaves(&at(removed, "/config"), &watched));
        // What happens in it is not the directory leaving, nor is a look
        // at it or a change of its own times.
        assert!(!leaves(&at(removed, "/dotfiles/settings.toml"), &watched));
        assert!(!leaves(&at(moved, "/dotfiles/settings.toml"), &watched));
        let looked = EventKind::Access(AccessKind::Any);
        assert!(!leaves(&at(looked, "/dotfiles"), &watched));
        let touched = EventKind::Modify(ModifyKind::Any);
        assert!(!leaves(&at(touched, "/dotfiles"), &watched));
        // Nor is another directory's.
        assert!(!leaves(&at(removed, "/elsewhere"), &watched));
    }

    #[test]
    fn a_settings_file_that_is_not_text_sends_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.toml");
        let (mut backend, _) = watching(&path);
        // Each write is all or nothing: one that truncates and then writes
        // could be read in between, as an empty text.
        crate::util::write_atomic(&path, &[0xff, 0xfe, 0x00]).unwrap();
        std::thread::sleep(Duration::from_millis(400));
        crate::util::write_atomic(&path, b"[data]\npage_size = 500\n").unwrap();
        // The first text to come is the readable one.
        assert_eq!(
            texts_until(&mut backend, "[data]\npage_size = 500\n"),
            vec!["[data]\npage_size = 500\n".to_owned()]
        );
    }

    #[test]
    fn reading_the_file_does_not_send_it_again() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.toml");
        let (mut backend, _) = watching(&path);
        std::fs::write(&path, "[data]\npage_size = 500\n").unwrap();
        assert!(!texts_until(&mut backend, "[data]\npage_size = 500\n").is_empty());
        // The reader's own look at the file does not wake it: nothing more
        // is sent.
        assert!(backend.wait(Duration::from_millis(600)).is_none());
        // And a save of the same text wakes it, but is not news. Renamed
        // into place, so that no read finds the file empty on the way.
        crate::util::write_atomic(&path, b"[data]\npage_size = 500\n").unwrap();
        assert!(backend.wait(Duration::from_millis(600)).is_none());
    }

    #[test]
    fn a_write_of_the_backends_own_is_sent_as_its_own() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.toml");
        let (mut backend, live) = watching(&path);
        assert!(live);
        // Another's save: the app is told it is not its own.
        crate::util::write_atomic(&path, b"[data]\npage_size = 500\n").unwrap();
        assert_eq!(
            files_until(&mut backend, "[data]\npage_size = 500\n"),
            vec![("[data]\npage_size = 500\n".to_owned(), false)]
        );
        // The app's save, through the backend.
        let settings = crate::settings::Settings {
            page_size: 100,
            ..Default::default()
        };
        backend.send(Command::Save {
            path: path.clone(),
            file: StateFile::Settings(settings.clone()),
        });
        assert_eq!(
            files_until(&mut backend, &settings.to_toml()),
            vec![(settings.to_toml(), true)]
        );
        // The text before it, put back by someone else, is theirs.
        crate::util::write_atomic(&path, b"[data]\npage_size = 500\n").unwrap();
        assert_eq!(
            files_until(&mut backend, "[data]\npage_size = 500\n"),
            vec![("[data]\npage_size = 500\n".to_owned(), false)]
        );
        // So is a text the backend did write, once it has written another:
        // only its last write is its own.
        let newer = crate::settings::Settings {
            page_size: 50,
            ..Default::default()
        };
        backend.send(Command::Save {
            path: path.clone(),
            file: StateFile::Settings(newer.clone()),
        });
        assert_eq!(
            files_until(&mut backend, &newer.to_toml()),
            vec![(newer.to_toml(), true)]
        );
        crate::util::write_atomic(&path, settings.to_toml().as_bytes()).unwrap();
        assert_eq!(
            files_until(&mut backend, &settings.to_toml()),
            vec![(settings.to_toml(), false)]
        );
    }

    #[test]
    fn a_save_that_failed_leaves_no_write_of_the_backends_own() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.toml");
        let (mut backend, live) = watching(&path);
        assert!(live);
        let first = crate::settings::Settings {
            page_size: 100,
            ..Default::default()
        };
        backend.send(Command::Save {
            path: path.clone(),
            file: StateFile::Settings(first.clone()),
        });
        assert_eq!(
            files_until(&mut backend, &first.to_toml()),
            vec![(first.to_toml(), true)]
        );
        // A folder where the file was: no system renames a file over one,
        // so the next save fails.
        std::fs::remove_file(&path).unwrap();
        std::fs::create_dir(&path).unwrap();
        let second = crate::settings::Settings {
            page_size: 50,
            ..Default::default()
        };
        backend.send(Command::Save {
            path: path.clone(),
            file: StateFile::Settings(second),
        });
        let deadline = std::time::Instant::now() + WAIT;
        loop {
            assert!(
                std::time::Instant::now() < deadline,
                "the save over a folder did not fail"
            );
            let event = backend.wait(Duration::from_millis(200));
            if let Some(Event::Saved { result: Err(_), .. }) = event {
                break;
            }
        }
        // Someone else writes the file again. The reader does not send the
        // text it sent last a second time, so another comes first.
        std::fs::remove_dir(&path).unwrap();
        crate::util::write_atomic(&path, PAGE_500.as_bytes()).unwrap();
        assert_eq!(
            files_until(&mut backend, PAGE_500),
            vec![(PAGE_500.to_owned(), false)]
        );
        // The first text is the last the backend wrote that reached the
        // disk. The app has moved on from it: put back, it is theirs.
        crate::util::write_atomic(&path, first.to_toml().as_bytes()).unwrap();
        assert_eq!(
            files_until(&mut backend, &first.to_toml()),
            vec![(first.to_toml(), false)]
        );
    }

    #[test]
    fn a_settings_file_that_is_deleted_sends_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.toml");
        let (mut backend, _) = watching(&path);
        std::fs::write(&path, "[data]\npage_size = 500\n").unwrap();
        assert!(!texts_until(&mut backend, "[data]\npage_size = 500\n").is_empty());
        std::fs::remove_file(&path).unwrap();
        assert!(backend.wait(Duration::from_millis(600)).is_none());
    }

    /// A config directory and a dotfiles directory under `root`.
    fn config_and_dotfiles(root: &std::path::Path) -> (PathBuf, PathBuf) {
        let (config, dotfiles) = (root.join("config"), root.join("dotfiles"));
        std::fs::create_dir_all(&config).unwrap();
        std::fs::create_dir_all(&dotfiles).unwrap();
        (config, dotfiles)
    }

    #[test]
    fn an_edit_behind_a_symbolic_link_is_read() {
        if !crate::util::can_symlink() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let (config, dotfiles) = config_and_dotfiles(dir.path());
        // As a dotfiles manager leaves it: the file in its repository,
        // under a name of its own, and a link in the config directory.
        let (path, target) = (
            config.join("settings.toml"),
            dotfiles.join("tabletist.toml"),
        );
        std::fs::write(&target, "[data]\npage_size = 100\n").unwrap();
        crate::util::symlink_file(&target, &path);
        let (mut backend, live) = watching(&path);
        assert!(live);
        // An edit of the file in the repository, in place.
        std::fs::write(&target, "[data]\npage_size = 500\n").unwrap();
        assert!(!texts_until(&mut backend, "[data]\npage_size = 500\n").is_empty());
        // An editor that saves there by renaming another file over it.
        crate::util::write_atomic(&target, b"[data]\npage_size = 1000\n").unwrap();
        assert!(!texts_until(&mut backend, "[data]\npage_size = 1000\n").is_empty());
        // The app's own save goes through the link, which stays.
        crate::util::write_atomic(&path, b"[data]\npage_size = 100\n").unwrap();
        assert!(!texts_until(&mut backend, "[data]\npage_size = 100\n").is_empty());
        assert_eq!(std::fs::read_link(&path).unwrap(), target);
    }

    #[test]
    fn a_link_made_while_the_file_is_watched_is_followed() {
        if !crate::util::can_symlink() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let (config, dotfiles) = config_and_dotfiles(dir.path());
        let (path, target) = (config.join("settings.toml"), dotfiles.join("settings.toml"));
        std::fs::write(&path, "[data]\npage_size = 100\n").unwrap();
        let (mut backend, live) = watching(&path);
        assert!(live);
        // The file moves to the repository and a link takes its place.
        std::fs::write(&target, "[data]\npage_size = 500\n").unwrap();
        std::fs::remove_file(&path).unwrap();
        crate::util::symlink_file(&target, &path);
        assert!(!texts_until(&mut backend, "[data]\npage_size = 500\n").is_empty());
        // From then on an edit made there is seen.
        std::fs::write(&target, "[data]\npage_size = 1000\n").unwrap();
        assert!(!texts_until(&mut backend, "[data]\npage_size = 1000\n").is_empty());
    }

    #[test]
    fn a_link_to_where_nothing_can_be_watched_is_not_live() {
        if !crate::util::can_symlink() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let (config, dotfiles) = config_and_dotfiles(dir.path());
        let path = config.join("settings.toml");
        // The directory the link leads to is not there.
        crate::util::symlink_file(dir.path().join("away").join("settings.toml"), &path);
        let (mut backend, live) = watching(&path);
        assert!(!live, "an edit made there would not be seen");
        // The link is turned to a directory that is: said, and followed.
        let target = dotfiles.join("settings.toml");
        std::fs::write(&target, "[data]\npage_size = 500\n").unwrap();
        std::fs::remove_file(&path).unwrap();
        crate::util::symlink_file(&target, &path);
        assert!(matches!(
            backend.wait(WAIT),
            Some(Event::SettingsWatch { live: true })
        ));
        assert!(!texts_until(&mut backend, "[data]\npage_size = 500\n").is_empty());
        std::fs::write(&target, "[data]\npage_size = 1000\n").unwrap();
        assert!(!texts_until(&mut backend, "[data]\npage_size = 1000\n").is_empty());
    }

    /// Puts a link to `target` at `path`, in place of what is there.
    fn link_anew(target: &std::path::Path, path: &std::path::Path) {
        let _ = std::fs::remove_file(path);
        crate::util::symlink_file(target, path);
    }

    #[test]
    fn the_watcher_watches_where_the_link_leads_each_time_it_turns() {
        if !crate::util::can_symlink() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let (config, dotfiles) = config_and_dotfiles(dir.path());
        let other = dir.path().join("other");
        std::fs::create_dir(&other).unwrap();
        let path = config.join("settings.toml");
        let mut watch = SettingsWatcher {
            watcher: notify::recommended_watcher(|_| {}).unwrap(),
            path: path.clone(),
            names: Arc::default(),
            elsewhere: Vec::new(),
            watched: Arc::default(),
            gone: Arc::default(),
        };
        let names = |watch: &SettingsWatcher| -> Vec<String> {
            let names = lock(&watch.names);
            let names = names.iter();
            names
                .map(|name| name.to_string_lossy().into_owned())
                .collect()
        };
        // As the system has it, which is how the watcher keeps it.
        let real = |directory: &std::path::Path| std::fs::canonicalize(directory).unwrap();
        let nowhere: Vec<PathBuf> = Vec::new();

        // No file yet, then a plain one: only the config directory.
        assert_eq!(watch.follow(), Ok(()));
        std::fs::write(&path, "").unwrap();
        assert_eq!(watch.follow(), Ok(()));
        assert_eq!(
            (names(&watch), &watch.elsewhere),
            (vec!["settings.toml".into()], &nowhere)
        );

        // A link to another directory, under another name there.
        link_anew(&dotfiles.join("tabletist.toml"), &path);
        assert_eq!(watch.follow(), Ok(()));
        assert_eq!(names(&watch), ["settings.toml", "tabletist.toml"]);
        assert_eq!(watch.elsewhere, [real(&dotfiles)]);

        // Turned to a third directory: that one, and no longer the second.
        link_anew(&other.join("settings.toml"), &path);
        assert_eq!(watch.follow(), Ok(()));
        assert_eq!(names(&watch), ["settings.toml", "settings.toml"]);
        assert_eq!(watch.elsewhere, [real(&other)]);

        // Through a second link, which can be turned too: the names of
        // both, and the directories of both.
        let current = dotfiles.join("current");
        link_anew(&other.join("settings.toml"), &current);
        link_anew(&current, &path);
        assert_eq!(watch.follow(), Ok(()));
        assert_eq!(names(&watch), ["settings.toml", "current", "settings.toml"]);
        assert_eq!(watch.elsewhere, [real(&other), real(&dotfiles)]);

        // Turned to a directory that is not there: said, and nothing kept.
        link_anew(&dir.path().join("away").join("settings.toml"), &path);
        assert!(watch.follow().is_err());
        assert_eq!(
            (names(&watch), &watch.elsewhere),
            (vec!["settings.toml".into()], &nowhere)
        );

        // A link within the config directory: a second name, no second
        // directory, also when it is reached by another path.
        link_anew(std::path::Path::new("real.toml"), &path);
        assert_eq!(watch.follow(), Ok(()));
        assert_eq!(names(&watch), ["settings.toml", "real.toml"]);
        assert_eq!(watch.elsewhere, nowhere);
        link_anew(&dotfiles.join("..").join("config").join("real.toml"), &path);
        assert_eq!(watch.follow(), Ok(()));
        assert_eq!(watch.elsewhere, nowhere);

        // And a plain file again.
        std::fs::remove_file(&path).unwrap();
        std::fs::write(&path, "").unwrap();
        assert_eq!(watch.follow(), Ok(()));
        assert_eq!(
            (names(&watch), &watch.elsewhere),
            (vec!["settings.toml".into()], &nowhere)
        );
    }

    #[test]
    fn a_link_turned_before_its_directory_is_watched_is_not_missed() {
        if !crate::util::can_symlink() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let (config, dotfiles) = config_and_dotfiles(dir.path());
        let (first, second) = (dir.path().join("first"), dir.path().join("second"));
        std::fs::create_dir(&first).unwrap();
        std::fs::create_dir(&second).unwrap();
        // config/settings.toml -> dotfiles/current -> first/settings.toml
        let (path, current) = (config.join("settings.toml"), dotfiles.join("current"));
        crate::util::symlink_file(first.join("settings.toml"), &current);
        crate::util::symlink_file(&current, &path);
        let mut watch = SettingsWatcher {
            watcher: notify::recommended_watcher(|_| {}).unwrap(),
            path,
            names: Arc::default(),
            elsewhere: Vec::new(),
            watched: Arc::default(),
            gone: Arc::default(),
        };
        // The second link is turned right after the first look at it, when
        // its directory is not watched yet: no event will ever say so.
        let mut looks = 0;
        let followed = watch.follow_with(|path| {
            let behind = behind_link(path);
            looks += 1;
            if looks == 1 {
                link_anew(&second.join("settings.toml"), &current);
            }
            behind
        });
        assert_eq!(followed, Ok(()));
        // What is watched is where the links lead now, not where the
        // first look found them leading.
        let real = |directory: &std::path::Path| std::fs::canonicalize(directory).unwrap();
        assert_eq!(watch.elsewhere, [real(&dotfiles), real(&second)]);
        assert_eq!(looks, 3, "looked at again until nothing had turned");

        // A link that is turned at every look is not called followed.
        let mut turns = 0;
        let followed = watch.follow_with(|path| {
            let behind = behind_link(path);
            turns += 1;
            let to = if turns % 2 == 0 { &second } else { &first };
            link_anew(&to.join("settings.toml"), &current);
            behind
        });
        assert!(followed.is_err(), "{followed:?}");
    }

    #[test]
    fn a_link_turned_in_the_middle_of_a_chain_is_followed() {
        if !crate::util::can_symlink() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let (config, dotfiles) = config_and_dotfiles(dir.path());
        let (first, second) = (dir.path().join("first"), dir.path().join("second"));
        std::fs::create_dir(&first).unwrap();
        std::fs::create_dir(&second).unwrap();
        std::fs::write(first.join("settings.toml"), "[data]\npage_size = 100\n").unwrap();
        std::fs::write(second.join("settings.toml"), "[data]\npage_size = 500\n").unwrap();
        // config/settings.toml -> dotfiles/current -> first/settings.toml
        let (path, current) = (config.join("settings.toml"), dotfiles.join("current"));
        crate::util::symlink_file(first.join("settings.toml"), &current);
        crate::util::symlink_file(&current, &path);
        let (mut backend, live) = watching(&path);
        assert!(live);
        // The second link is turned: nothing changes beside the first, nor
        // beside the file it led to.
        link_anew(&second.join("settings.toml"), &current);
        assert!(!texts_until(&mut backend, "[data]\npage_size = 500\n").is_empty());
        // And an edit made where it leads now is seen.
        std::fs::write(second.join("settings.toml"), "[data]\npage_size = 1000\n").unwrap();
        assert!(!texts_until(&mut backend, "[data]\npage_size = 1000\n").is_empty());
    }

    #[test]
    fn a_link_turned_while_the_file_is_watched_is_followed() {
        if !crate::util::can_symlink() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let (config, dotfiles) = config_and_dotfiles(dir.path());
        let other = dir.path().join("other");
        std::fs::create_dir(&other).unwrap();
        let path = config.join("settings.toml");
        let (first, second) = (dotfiles.join("settings.toml"), other.join("settings.toml"));
        std::fs::write(&first, "[data]\npage_size = 100\n").unwrap();
        std::fs::write(&second, "[data]\npage_size = 500\n").unwrap();
        crate::util::symlink_file(&first, &path);
        let (mut backend, live) = watching(&path);
        assert!(live);

        // Turned to a file in another directory: read, and watched there.
        link_anew(&second, &path);
        assert!(!texts_until(&mut backend, "[data]\npage_size = 500\n").is_empty());
        std::fs::write(&second, "[data]\npage_size = 1000\n").unwrap();
        assert!(!texts_until(&mut backend, "[data]\npage_size = 1000\n").is_empty());

        // Turned to a directory that is not there: no longer live.
        link_anew(&dir.path().join("away").join("settings.toml"), &path);
        let deadline = std::time::Instant::now() + WAIT;
        loop {
            match backend.wait(Duration::from_millis(200)) {
                Some(Event::SettingsWatch { live }) => break assert!(!live),
                // The second file's text once more, when its last write
                // was read in two halves.
                Some(Event::SettingsFile { .. }) | None => {}
                other => panic!("expected the watch to be lost, got {other:?}"),
            }
            assert!(std::time::Instant::now() < deadline, "never said");
        }

        // A plain file takes the link's place: live again, and read.
        std::fs::remove_file(&path).unwrap();
        crate::util::write_atomic(&path, b"[data]\npage_size = 50\n").unwrap();
        assert!(matches!(
            backend.wait(WAIT),
            Some(Event::SettingsWatch { live: true })
        ));
        assert!(!texts_until(&mut backend, "[data]\npage_size = 50\n").is_empty());
        std::fs::write(&path, "[data]\npage_size = 100\n").unwrap();
        assert!(!texts_until(&mut backend, "[data]\npage_size = 100\n").is_empty());
    }

    #[test]
    fn a_place_that_appears_behind_a_link_is_found() {
        if !crate::util::can_symlink() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let (config, _) = config_and_dotfiles(dir.path());
        let path = config.join("settings.toml");
        // The directory the link leads to is not there yet (a volume that
        // is not mounted).
        let away = dir.path().join("away");
        let target = away.join("settings.toml");
        crate::util::symlink_file(&target, &path);
        let (mut backend, live) = watching(&path);
        assert!(!live);
        // It appears, and nothing changes beside the link: no event says so.
        std::fs::create_dir(&away).unwrap();
        crate::util::write_atomic(&target, b"[data]\npage_size = 500\n").unwrap();
        assert!(matches!(
            backend.wait(WAIT),
            Some(Event::SettingsWatch { live: true })
        ));
        assert!(!texts_until(&mut backend, "[data]\npage_size = 500\n").is_empty());
        // From then on an edit made there is seen.
        std::fs::write(&target, "[data]\npage_size = 1000\n").unwrap();
        assert!(!texts_until(&mut backend, "[data]\npage_size = 1000\n").is_empty());
    }

    /// A link at the config directory's `settings.toml` to the dotfiles
    /// directory's, which reads 100 rows a page, and a backend watching it.
    #[cfg(target_os = "linux")]
    fn linked_to_dotfiles(root: &std::path::Path) -> (Backend, PathBuf, PathBuf) {
        let (config, dotfiles) = config_and_dotfiles(root);
        let (path, target) = (config.join("settings.toml"), dotfiles.join("settings.toml"));
        std::fs::write(&target, "[data]\npage_size = 100\n").unwrap();
        crate::util::symlink_file(&target, &path);
        let (backend, live) = watching(&path);
        assert!(live);
        (backend, dotfiles, target)
    }

    // The two below are for Linux, whose watches stay with the directory
    // they were put on and say when it leaves.
    #[cfg(target_os = "linux")]
    #[test]
    fn a_dotfiles_directory_set_aside_and_made_anew_is_watched_again() {
        let dir = tempfile::tempdir().unwrap();
        let (mut backend, dotfiles, target) = linked_to_dotfiles(dir.path());
        // The repository is set aside and another takes its place. The link
        // is as it was, and nothing changes beside it.
        std::fs::rename(&dotfiles, dir.path().join("dotfiles.old")).unwrap();
        std::fs::create_dir(&dotfiles).unwrap();
        std::fs::write(&target, "[data]\npage_size = 500\n").unwrap();
        assert!(!texts_until(&mut backend, "[data]\npage_size = 500\n").is_empty());
        // And an edit made in the new one is seen.
        std::fs::write(&target, "[data]\npage_size = 1000\n").unwrap();
        assert!(!texts_until(&mut backend, "[data]\npage_size = 1000\n").is_empty());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn a_dotfiles_directory_removed_and_made_anew_is_watched_again() {
        let dir = tempfile::tempdir().unwrap();
        let (mut backend, dotfiles, target) = linked_to_dotfiles(dir.path());
        std::fs::remove_dir_all(&dotfiles).unwrap();
        std::fs::create_dir(&dotfiles).unwrap();
        std::fs::write(&target, "[data]\npage_size = 500\n").unwrap();
        assert!(!texts_until(&mut backend, "[data]\npage_size = 500\n").is_empty());
        // The watch that was on the directory went with it: an edit made
        // in the new one is seen all the same.
        std::fs::write(&target, "[data]\npage_size = 1000\n").unwrap();
        assert!(!texts_until(&mut backend, "[data]\npage_size = 1000\n").is_empty());
    }

    #[test]
    fn a_watcher_told_a_directory_left_watches_them_all_anew() {
        if !crate::util::can_symlink() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let (config, dotfiles) = config_and_dotfiles(dir.path());
        let path = config.join("settings.toml");
        link_anew(&dotfiles.join("settings.toml"), &path);
        let mut watch = SettingsWatcher {
            watcher: notify::recommended_watcher(|_| {}).unwrap(),
            path: path.clone(),
            names: Arc::default(),
            elsewhere: Vec::new(),
            watched: Arc::default(),
            gone: Arc::default(),
        };
        let real = |directory: &std::path::Path| std::fs::canonicalize(directory).unwrap();
        assert_eq!(watch.follow(), Ok(()));
        // The callback is told both directories, to know them in an event.
        assert_eq!(
            *lock(&watch.watched),
            [std::path::absolute(&config).unwrap(), real(&dotfiles)]
        );
        // One left and is there again: all are watched anew, and the
        // watcher is done with what it was told.
        watch.gone.store(true, Ordering::SeqCst);
        assert_eq!(watch.follow(), Ok(()));
        assert!(!watch.gone.load(Ordering::SeqCst));
        assert_eq!(watch.elsewhere, [real(&dotfiles)]);
    }

    #[test]
    fn a_watcher_whose_own_directory_left_tries_again_until_it_is_there() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("config");
        let mut watch = SettingsWatcher {
            watcher: notify::recommended_watcher(|_| {}).unwrap(),
            path: config.join("settings.toml"),
            names: Arc::default(),
            elsewhere: Vec::new(),
            watched: Arc::default(),
            gone: Arc::new(AtomicBool::new(true)),
        };
        // Not there: said, and what the watcher was told stays told.
        assert!(watch.follow().is_err());
        assert!(watch.gone.load(Ordering::SeqCst));
        std::fs::create_dir(&config).unwrap();
        assert_eq!(watch.follow(), Ok(()));
        assert!(!watch.gone.load(Ordering::SeqCst));
    }

    #[test]
    fn a_directory_that_is_not_there_cannot_be_watched() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("missing").join("settings.toml");
        let (_backend, live) = watching(&path);
        assert!(!live);
    }

    #[test]
    fn a_settings_file_that_is_not_there_is_written_before_the_editor_starts() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config").join("settings.toml");
        let written = Written::default();
        let mut seen = None;
        open_in_editor(&path, "[data]\npage_size = 300\n", &written, |opened| {
            seen = Some((
                opened.to_path_buf(),
                std::fs::read_to_string(opened).unwrap(),
            ));
            Ok(())
        })
        .unwrap();
        assert_eq!(
            seen,
            Some((path.clone(), "[data]\npage_size = 300\n".to_owned()))
        );
        // A file that is there is opened as it is, whatever the app holds.
        std::fs::write(&path, "[data]\npage_size = 100\n").unwrap();
        open_in_editor(&path, "[data]\npage_size = 300\n", &written, |_| Ok(())).unwrap();
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "[data]\npage_size = 100\n"
        );
    }

    #[test]
    fn the_file_written_for_the_editor_is_the_backends_own_write() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.toml");
        let written = Written::default();
        open_in_editor(&path, PAGE_500, &written, |_| Ok(())).unwrap();
        assert_eq!(lock(&written).as_deref(), Some(PAGE_500));
        // So the reader knows it: a save of the app's that lands before
        // this write does not leave it looking like someone else's text.
        let found = runtime()
            .block_on(read_file(path.clone(), Arc::clone(&written)))
            .unwrap();
        assert!(found.own);
    }

    #[test]
    fn a_file_that_is_there_is_no_write_of_the_backends() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.toml");
        let theirs = "[data]\npage_size = 100\n";
        std::fs::write(&path, theirs).unwrap();
        // Nothing written yet, or a text of the backend's from before
        // someone else wrote the file: neither is touched.
        for before in [None, Some("[data]\npage_size = 50\n".to_owned())] {
            let written: Written = Arc::new(Mutex::new(before.clone()));
            open_in_editor(&path, PAGE_500, &written, |_| Ok(())).unwrap();
            assert_eq!(*lock(&written), before);
            assert_eq!(std::fs::read_to_string(&path).unwrap(), theirs);
        }
    }

    #[test]
    fn a_file_made_after_the_look_for_it_is_opened_as_it_is() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.toml");
        let theirs = "[data]\npage_size = 100\n";
        // Nothing written yet, or a text of the backend's from before:
        // neither is touched by a file someone else made.
        for before in [None, Some("[data]\npage_size = 50\n".to_owned())] {
            let _ = std::fs::remove_file(&path);
            let written: Written = Arc::new(Mutex::new(before.clone()));
            let mut opened = None;
            open_where_missing(
                &path,
                PAGE_500,
                &written,
                // Not there when it is looked for, and there a moment
                // later: an editor saved it.
                |path| {
                    std::fs::write(path, theirs).unwrap();
                    true
                },
                |path| {
                    opened = Some(std::fs::read_to_string(path).unwrap());
                    Ok(())
                },
            )
            .unwrap();
            assert_eq!(opened.as_deref(), Some(theirs));
            assert_eq!(*lock(&written), before);
            // And nothing is left beside it.
            let files = std::fs::read_dir(dir.path()).unwrap().count();
            assert_eq!(files, 1);
        }
    }

    #[test]
    fn an_editor_that_cannot_start_is_told_with_the_file_still_written() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.toml");
        let error = open_in_editor(&path, "[data]\n", &Written::default(), |_| {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "no editor",
            ))
        })
        .unwrap_err();
        assert!(error.contains("no editor"), "{error}");
        assert!(path.exists());
    }

    #[test]
    fn the_editor_is_the_systems_way_to_open_a_text_file() {
        let path = std::path::Path::new("/config/settings.toml");
        let (program, args) = editor_command(path, |_| false);
        #[cfg(target_os = "macos")]
        assert_eq!((program.as_str(), args.len()), ("open", 2));
        #[cfg(windows)]
        assert_eq!((program.as_str(), args.len()), ("explorer", 1));
        #[cfg(all(unix, not(target_os = "macos")))]
        {
            assert_eq!((program.as_str(), args.len()), ("xdg-open", 1));
            // Omarchy's own launcher, where it is installed.
            let (program, args) = editor_command(path, |name| name == "omarchy-launch-editor");
            assert_eq!(program, "omarchy-launch-editor");
            assert_eq!(args, vec![path.as_os_str().to_owned()]);
        }
        assert_eq!(
            args.last().map(|arg| arg.as_os_str()),
            Some(path.as_os_str())
        );
    }

    #[test]
    fn reveal_shows_the_file_where_it_is_kept() {
        let path = std::path::Path::new("/config/settings.toml");
        let (program, args) = reveal_command(path);
        #[cfg(target_os = "macos")]
        assert_eq!(
            (program.as_str(), args),
            ("open", vec!["-R".into(), path.as_os_str().to_owned()])
        );
        // One argument: explorer takes the path after the comma.
        #[cfg(windows)]
        assert_eq!(
            (program.as_str(), args),
            ("explorer", vec!["/select,/config/settings.toml".into()])
        );
        // No file manager is asked to select a file the same way: the
        // directory is opened.
        #[cfg(all(unix, not(target_os = "macos")))]
        assert_eq!(
            (program.as_str(), args),
            ("xdg-open", vec![std::ffi::OsString::from("/config")])
        );
    }

    #[test]
    fn what_starts_the_program_is_told_with_its_answer() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.toml");
        let (outbox, events) = quiet_outbox();
        // A file that is not there is written first, whichever program it is
        // for, and a program that does not start is told with which it was.
        open_settings_file(
            &path,
            PAGE_500,
            &Written::default(),
            Opened::Folder,
            |shown| {
                assert_eq!(std::fs::read_to_string(shown).unwrap(), PAGE_500);
                Err(std::io::Error::other("no file manager"))
            },
            &outbox,
        );
        match events.try_recv() {
            Ok(Event::SettingsFileOpened {
                with: Opened::Folder,
                result: Err(error),
            }) => assert!(error.contains("no file manager"), "{error}"),
            other => panic!("{other:?}"),
        }
    }

    #[cfg(unix)]
    #[test]
    fn a_command_is_on_the_path_only_where_it_can_be_run() {
        use std::os::unix::fs::PermissionsExt as _;
        let dir = tempfile::tempdir().unwrap();
        let dirs = || [dir.path().join("missing"), dir.path().to_path_buf()];
        assert!(!on_path("launch-editor", dirs()));
        // A file of the name that nobody may run: starting it would fail,
        // and the next way to open the file would never be tried.
        let file = dir.path().join("launch-editor");
        std::fs::write(&file, "#!/bin/sh\n").unwrap();
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(!on_path("launch-editor", dirs()));
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert!(on_path("launch-editor", dirs()));
        // A folder of the name has the bits and is no command.
        std::fs::create_dir(dir.path().join("a-folder")).unwrap();
        assert!(!on_path("a-folder", dirs()));
    }
}
