//! The MCP server in the app: the connections agents may use, and the
//! gateway through which the server reaches them.
//!
//! The gateway keeps sessions of its own, one for each connection an agent
//! has used: a tab's session runs its own commands one at a time, and an
//! agent's ten-second query must not stall the grid behind it. Every one
//! is read-only at the database, whatever the connection's own box says.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::AtomicU32;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use tabletist_db::agent::{Answer, Limits, Read};
use tabletist_db::{
    Access, CancelHandle, ConnectSpec, Connection, Error, HostKeys, ObjectInfo, ObjectRef, Secrets,
    StopFlag, Structure,
};
use tabletist_mcp::{Failure, Gateway, Listed, Log, Schemas, Session};
use tokio::sync::OwnedMutexGuard;
use tokio_util::sync::CancellationToken;

use super::{CANCEL_AGAIN, Outbox, lock, longer, send_cancel};
use crate::connections::{AgentAccess, ConnectionId, PasswordMode, SavedConnection};
use crate::env::Platform;
use crate::model::SecretKind;
use crate::secrets::{Keyring, password_account, ssh_account};

#[cfg(unix)]
mod listen;

/// A connection agents may use, as the server is given it: what opens it,
/// and what an agent calls it. Its password is not here: the gateway reads
/// it from the keyring when it opens the connection, as a tab does.
#[derive(Debug, Clone, PartialEq)]
pub struct Agentable {
    pub id: ConnectionId,
    /// What an agent calls it: its name and its environment, with a number
    /// after it where another connection would be called the same.
    pub name: String,
    pub spec: ConnectSpec,
    pub password: PasswordMode,
    pub ssh_secret: PasswordMode,
    pub limits: Limits,
}

/// The connections of `saved` that agents may use, in its order. One that
/// is Off is left out: to an agent it does not exist.
///
/// An agent calls a connection by its name and its environment. Where two
/// saved connections would be called the same, Off or not, each is called
/// that with the start of its id after it. So a name means one database for
/// as long as the connections called the same are the same ones: turning
/// one Off, or on, changes no name. When one of them is added or removed
/// the names of the others change with it, the bare name among them, and
/// an agent learns the new ones from `list_connections`.
pub fn agentable(saved: &[SavedConnection]) -> Vec<Agentable> {
    saved
        .iter()
        .zip(agent_names(saved))
        .filter(|(saved, _)| match saved.agents.access {
            AgentAccess::Off => false,
            AgentAccess::Read => true,
        })
        .map(|(saved, name)| Agentable {
            id: saved.id.clone(),
            name,
            spec: saved.spec.clone(),
            password: saved.password,
            ssh_secret: saved.ssh_secret,
            limits: Limits {
                rows: saved.agents.rows,
                timeout: Duration::from_secs(u64::from(saved.agents.timeout_secs)),
            },
        })
        .collect()
}

/// What an agent calls each of `saved`, in its order, Off or not, by the
/// rule [`agentable`] says.
pub fn agent_names(saved: &[SavedConnection]) -> Vec<String> {
    let called = |saved: &SavedConnection| match saved.environment.named(Platform::Native) {
        Some(environment) => format!("{} · {environment}", saved.name),
        None => saved.name.clone(),
    };
    let mut times: HashMap<String, usize> = HashMap::new();
    for saved in saved {
        *times.entry(called(saved)).or_default() += 1;
    }
    let shared = |saved: &SavedConnection| times[&called(saved)] > 1;
    let short: Vec<String> = saved
        .iter()
        .map(|saved| match shared(saved) {
            true => format!("{} ({})", called(saved), short_id(&saved.id, 8)),
            false => called(saved),
        })
        .collect();
    // A short id can make a name another connection already has: those
    // take their whole id.
    let names: Vec<String> = saved
        .iter()
        .enumerate()
        .map(|(index, saved)| {
            let taken = short
                .iter()
                .enumerate()
                .any(|(other, name)| other != index && *name == short[index]);
            match taken && shared(saved) {
                true => format!("{} ({})", called(saved), saved.id.0),
                false => short[index].clone(),
            }
        })
        .collect();
    names
}

/// The first `length` letters and digits of an id.
fn short_id(id: &ConnectionId, length: usize) -> String {
    id.0.chars()
        .filter(char::is_ascii_alphanumeric)
        .take(length)
        .collect()
}

/// What the backend is told of the server.
#[derive(Debug)]
pub enum AgentsCommand {
    /// The connections agents may use, and the SSH host keys the user
    /// trusts. Sent whenever either changes. Kept while the server is off.
    Policy {
        connections: Vec<Agentable>,
        host_keys: HostKeys,
    },
    /// Starts the server, with its log at `log`, which keeps calls for
    /// `log_days` days, listening in `endpoint` for `tabletist mcp`
    /// (`AppDirs::agents_dir`). Without one nothing reaches it, which the
    /// tests of the gateway want. Answered with [`AgentsEvent::Started`] or
    /// [`AgentsEvent::Failed`].
    Start {
        log: PathBuf,
        log_days: u32,
        endpoint: Option<PathBuf>,
    },
    /// The log keeps calls for these days from now on.
    LogDays(u32),
    /// Stops one session, by its id: its calls answer that the server
    /// stopped them, and its agent is let go. Nothing where it is gone.
    StopSession(String),
    /// Reads a page of the log at `log`, whether the server runs or not:
    /// the calls on `connection`, or on every one, from before the call
    /// `before` names. Answered with [`AgentsEvent::LogRead`], `asked`
    /// with it.
    ReadLog {
        log: PathBuf,
        connection: Option<String>,
        before: Option<i64>,
        asked: u64,
    },
    /// Stops it: every session ends, and the connections are closed.
    /// Answered with [`AgentsEvent::Stopped`].
    Stop,
}

/// What the backend tells of the server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentsEvent {
    /// It runs. `kept_aside` is where a log that could not be read was put.
    Started {
        kept_aside: Option<PathBuf>,
    },
    /// It could not start: its log could not be opened, or it could not
    /// listen. Why, for the person.
    Failed(String),
    Stopped,
    /// The sessions that are open, the oldest first: said whenever one
    /// opens or ends, names its agent, or makes a call. None is said after
    /// [`AgentsEvent::Stopped`].
    Sessions(Vec<tabletist_mcp::SessionInfo>),
    /// A call ended, and is in the log.
    Called(tabletist_mcp::Call),
    /// A page of the log, for the [`AgentsCommand::ReadLog`] that `asked`;
    /// or why it could not be read.
    LogRead {
        asked: u64,
        page: Result<tabletist_mcp::Page, String>,
    },
}

/// How many calls a page of the log holds.
pub const LOG_PAGE: u32 = 200;

/// How long a connection an agent used is kept open with nothing to do.
const IDLE: Duration = Duration::from_secs(10 * 60);

/// How often the server's chores are done: closing what is idle, and,
/// every [`RETENTION_EVERY`], forgetting old calls.
const CHORES_EVERY: Duration = Duration::from_secs(60);

/// How often the log forgets what is older than it keeps.
const RETENTION_EVERY: Duration = Duration::from_secs(6 * 60 * 60);

/// How long opening a connection for an agent may take, its read of the
/// keyring and an SSH tunnel's among it.
const CONNECT_TIME: Duration = Duration::from_secs(20);

/// The server in the backend, while it runs.
pub(super) struct Running {
    server: tabletist_mcp::Server<AppGateway>,
    chores: tokio::task::JoinHandle<()>,
    /// Tells the app of the sessions as they change.
    watching: tokio::task::JoinHandle<()>,
    /// Tells the app of each call as it ends.
    telling: tokio::task::JoinHandle<()>,
    #[cfg(unix)]
    listening: Option<listen::Listening>,
}

/// The server's part of the worker: the gateway, whatever the server is
/// doing, and the server while it runs.
pub(super) struct Agents {
    gateway: AppGateway,
    running: Arc<Mutex<Option<Running>>>,
    /// Counted up by each start and stop, so that a start whose log is
    /// still opening when a stop comes runs nothing, and says nothing.
    turn: Arc<Mutex<u64>>,
    /// Held by a start from the opening of its log to its end: two starts
    /// must not open the one file at once.
    starting: Arc<tokio::sync::Mutex<()>>,
    /// The days the log keeps a call, which its chores read each time.
    log_days: Arc<AtomicU32>,
}

impl Agents {
    pub fn new(keyring: Keyring) -> Self {
        Self {
            gateway: AppGateway::new(keyring),
            running: Arc::default(),
            turn: Arc::default(),
            starting: Arc::default(),
            log_days: Arc::default(),
        }
    }

    pub fn handle(&mut self, command: AgentsCommand, outbox: &Outbox) {
        match command {
            AgentsCommand::Policy {
                connections,
                host_keys,
            } => self.gateway.set_policy(connections, host_keys),
            AgentsCommand::LogDays(days) => {
                self.log_days
                    .store(days, std::sync::atomic::Ordering::SeqCst);
            }
            AgentsCommand::ReadLog {
                log,
                connection,
                before,
                asked,
            } => {
                let outbox = outbox.clone();
                tokio::spawn(async move {
                    let page = read_log(log, connection, before).await;
                    outbox.emit(super::Event::Agents(AgentsEvent::LogRead { asked, page }));
                });
            }
            AgentsCommand::StopSession(id) => {
                if let Some(running) = lock(&self.running).as_ref()
                    && running.server.stop_session(&id)
                {
                    log::info!("a session of an agent was stopped");
                }
            }
            AgentsCommand::Start {
                log,
                log_days,
                endpoint,
            } => {
                self.log_days
                    .store(log_days, std::sync::atomic::Ordering::SeqCst);
                let log_days = Arc::clone(&self.log_days);
                let turn = {
                    let mut turn = lock(&self.turn);
                    *turn += 1;
                    *turn
                };
                let gateway = self.gateway.clone();
                let running = Arc::clone(&self.running);
                let current = Arc::clone(&self.turn);
                let starting = Arc::clone(&self.starting);
                let outbox = outbox.clone();
                tokio::spawn(async move {
                    let _starting = starting.lock().await;
                    let overtaken = || *lock(&current) != turn;
                    if overtaken() {
                        return;
                    }
                    // Before the log: another Tabletist's server has it open,
                    // and one of another version would set it aside.
                    if lock(&running).is_none()
                        && endpoint.as_deref().is_some_and(served_by_another)
                    {
                        log::error!("the agent server did not start: another one serves agents");
                        let said = crate::endpoint::ANOTHER_SERVES.to_owned();
                        outbox.emit(super::Event::Agents(AgentsEvent::Failed(said)));
                        return;
                    }
                    let opened = tokio::task::spawn_blocking(move || Log::open(&log)).await;
                    let failed = match opened {
                        Ok(Ok(opened)) => Ok(opened),
                        Ok(Err(error)) => Err(error.to_string()),
                        // The open panicked on the blocking pool. It is
                        // answered all the same: whoever started waits for
                        // a word.
                        Err(_) => Err("opening it failed".to_owned()),
                    };
                    let opened = match failed {
                        Ok(opened) => opened,
                        Err(error) => {
                            log::error!("the agent server did not start: {error}");
                            // A stop or a later start came meanwhile: theirs
                            // is the last word.
                            if !overtaken() {
                                let said = format!(
                                    "the log of agents' calls could not be opened: {error}"
                                );
                                outbox.emit(super::Event::Agents(AgentsEvent::Failed(said)));
                            }
                            return;
                        }
                    };
                    let server = tabletist_mcp::Server::new(
                        gateway.clone(),
                        opened.log.clone(),
                        env!("CARGO_PKG_VERSION"),
                    );
                    let mut running = lock(&running);
                    // A stop came while the log was opening.
                    if overtaken() {
                        return;
                    }
                    if let Some(before) = running.take() {
                        before.stop();
                    }
                    // Before it listens: no session opens unseen, and no
                    // call ends untold.
                    let changes = server.changes();
                    let calls = server.calls();
                    // Once the server before it has let its socket go.
                    let listening = match listening(endpoint.as_deref(), &server) {
                        Ok(listening) => listening,
                        Err(error) => {
                            log::error!("the agent server did not start: {error}");
                            server.stop();
                            outbox.emit(super::Event::Agents(AgentsEvent::Failed(error)));
                            return;
                        }
                    };
                    #[cfg(not(unix))]
                    let () = listening;
                    let chores = tokio::spawn(chores(opened.log, gateway, log_days));
                    let watching = tokio::spawn(sessions(server.clone(), changes, outbox.clone()));
                    let telling = tokio::spawn(called(calls, outbox.clone()));
                    *running = Some(Running {
                        server,
                        chores,
                        watching,
                        telling,
                        #[cfg(unix)]
                        listening,
                    });
                    log::info!("the agent server started");
                    // Said under the lock a stop takes, so that no Stopped
                    // is said before it.
                    outbox.emit(super::Event::Agents(AgentsEvent::Started {
                        kept_aside: opened.kept_aside,
                    }));
                });
            }
            AgentsCommand::Stop => {
                *lock(&self.turn) += 1;
                let mut running = lock(&self.running);
                if let Some(running) = running.take() {
                    running.stop();
                }
                self.gateway.close_all();
                log::info!("the agent server stopped");
                outbox.emit(super::Event::Agents(AgentsEvent::Stopped));
            }
        }
    }
}

impl Running {
    /// Stops the server, then the endpoint, whose socket and key go with
    /// it.
    fn stop(self) {
        self.server.stop();
        self.chores.abort();
        self.watching.abort();
        self.telling.abort();
        #[cfg(unix)]
        drop(self.listening);
    }
}

/// Listens in `endpoint` for `server`, where there is one.
#[cfg(unix)]
fn listening(
    endpoint: Option<&std::path::Path>,
    server: &tabletist_mcp::Server<AppGateway>,
) -> Result<Option<listen::Listening>, String> {
    endpoint
        .map(|dir| listen::listen(dir, server.clone()))
        .transpose()
}

/// Whether another Tabletist serves agents in `dir`.
#[cfg(unix)]
fn served_by_another(dir: &std::path::Path) -> bool {
    listen::served_by_another(dir)
}

/// Nothing serves agents here yet.
#[cfg(not(unix))]
fn served_by_another(_dir: &std::path::Path) -> bool {
    false
}

/// Nothing listens here yet (`endpoint::AVAILABLE`).
#[cfg(not(unix))]
fn listening(
    endpoint: Option<&std::path::Path>,
    _server: &tabletist_mcp::Server<AppGateway>,
) -> Result<(), String> {
    match endpoint {
        Some(_) => Err("agents cannot reach Tabletist on this system yet".into()),
        None => Ok(()),
    }
}

/// Tells the app of the server's sessions as it starts, and each time they
/// change, until the server stops.
async fn sessions(
    server: tabletist_mcp::Server<AppGateway>,
    mut changes: tokio::sync::watch::Receiver<()>,
    outbox: Outbox,
) {
    loop {
        changes.mark_unchanged();
        let open = server.open_sessions();
        outbox.emit(super::Event::Agents(AgentsEvent::Sessions(open)));
        if changes.changed().await.is_err() {
            return;
        }
    }
}

/// A page of the log at `log`, opened to read: nothing is made or set
/// aside. No log yet is an empty one.
async fn read_log(
    log: PathBuf,
    connection: Option<String>,
    before: Option<i64>,
) -> Result<tabletist_mcp::Page, String> {
    let opened = tokio::task::spawn_blocking(move || Log::open_to_read(&log))
        .await
        .map_err(|_| "opening it failed".to_owned())?
        .map_err(|error| error.to_string())?;
    match opened {
        Some(log) => log
            .page(connection, before, LOG_PAGE)
            .await
            .map_err(|error| error.to_string()),
        None => Ok(tabletist_mcp::Page::default()),
    }
}

/// Tells the app of each call as it ends, until the server stops. One
/// that falls behind misses the oldest: the log has them all.
async fn called(mut calls: tokio::sync::broadcast::Receiver<tabletist_mcp::Call>, outbox: Outbox) {
    use tokio::sync::broadcast::error::RecvError;
    loop {
        match calls.recv().await {
            Ok(call) => outbox.emit(super::Event::Agents(AgentsEvent::Called(call))),
            Err(RecvError::Lagged(_)) => {}
            Err(RecvError::Closed) => return,
        }
    }
}

/// What the server does from time to time while it runs: closes the
/// connections no agent has used for a while, and forgets calls older than
/// the log keeps. The first time as it starts.
async fn chores(log: Log, gateway: AppGateway, log_days: Arc<AtomicU32>) {
    let mut forgot: Option<Instant> = None;
    loop {
        if forgot.is_none_or(|at| at.elapsed() >= RETENTION_EVERY) {
            let days = log_days.load(std::sync::atomic::Ordering::SeqCst).max(1);
            let keep = Duration::from_secs(u64::from(days) * 24 * 60 * 60);
            let before = SystemTime::now()
                .checked_sub(keep)
                .and_then(|before| before.duration_since(UNIX_EPOCH).ok())
                .map_or(0, |before| {
                    i64::try_from(before.as_millis()).unwrap_or(i64::MAX)
                });
            match log.forget_before(before).await {
                Ok(0) => {}
                Ok(gone) => log::info!("the agents' log forgot {gone} calls"),
                Err(error) => log::warn!("the agents' log could not forget old calls: {error}"),
            }
            forgot = Some(Instant::now());
        }
        gateway.close_idle(IDLE);
        tokio::time::sleep(CHORES_EVERY).await;
    }
}

/// The gateway of the app's MCP server: the saved connections an agent
/// may use, and a session of its own for each. A clone is the same gateway.
#[derive(Clone)]
pub struct AppGateway {
    inner: Arc<Inner>,
}

struct Inner {
    keyring: Keyring,
    policy: Mutex<Policy>,
    /// A connection's turn, and its session while it is open. A turn is
    /// never taken out of the map: a call that holds it, or waits for it,
    /// would then be beside a call that took a new one.
    turns: Mutex<HashMap<ConnectionId, Arc<tokio::sync::Mutex<Option<Open>>>>>,
    /// Counted up by [`AppGateway::close_all`]: a session opened before it
    /// is closed when its call lets it go.
    epoch: std::sync::atomic::AtomicU64,
}

impl Inner {
    fn epoch(&self) -> u64 {
        self.epoch.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Whether a session opened for `opened_for` in `epoch` may be kept for
    /// the next call: the connection is still that, and nothing closed
    /// every session since.
    fn keeps(&self, opened_for: &Agentable, epoch: u64) -> bool {
        epoch == self.epoch()
            && lock(&self.policy)
                .connections
                .iter()
                .any(|agentable| agentable == opened_for)
    }
}

#[derive(Default)]
struct Policy {
    connections: Vec<Agentable>,
    host_keys: HostKeys,
}

/// A connection's session, open.
struct Open {
    connection: Connection,
    /// What it was opened as: a connection that is not that any more is
    /// opened again.
    opened_for: Agentable,
    used: Instant,
    /// [`Inner::epoch`] when it was opened.
    epoch: u64,
}

impl AppGateway {
    pub fn new(keyring: Keyring) -> Self {
        Self {
            inner: Arc::new(Inner {
                keyring,
                policy: Mutex::default(),
                turns: Mutex::default(),
                epoch: std::sync::atomic::AtomicU64::new(0),
            }),
        }
    }

    /// The connections agents may use from now on. One that is not among
    /// them any more, or not as it was, is closed now if no call holds it,
    /// else once its call is over.
    pub fn set_policy(&self, connections: Vec<Agentable>, host_keys: HostKeys) {
        *lock(&self.inner.policy) = Policy {
            connections,
            host_keys,
        };
        self.close_free(|open| !self.inner.keeps(&open.opened_for, open.epoch));
    }

    /// Closes every connection no call holds, and the others once their
    /// calls are over.
    pub fn close_all(&self) {
        self.inner
            .epoch
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        self.close_free(|_| true);
    }

    /// Closes each session no call holds that `closes` says to.
    fn close_free(&self, closes: impl Fn(&Open) -> bool) {
        let turns: Vec<_> = lock(&self.inner.turns).values().cloned().collect();
        for turn in turns {
            if let Ok(mut open) = turn.try_lock()
                && open.as_ref().is_some_and(&closes)
            {
                *open = None;
            }
        }
    }

    /// Closes the connections that have had nothing to do for `idle`. One
    /// that is in use is not idle.
    pub fn close_idle(&self, idle: Duration) {
        self.close_free(|open| open.used.elapsed() >= idle);
    }

    fn find(&self, id: &str) -> Option<(Agentable, HostKeys)> {
        let policy = lock(&self.inner.policy);
        policy
            .connections
            .iter()
            .find(|agentable| agentable.id.0 == id)
            .map(|agentable| (agentable.clone(), policy.host_keys.clone()))
    }
}

impl Gateway for AppGateway {
    type Session = AppSession;

    fn connections(&self) -> Vec<Listed> {
        lock(&self.inner.policy)
            .connections
            .iter()
            .map(|agentable| Listed {
                id: agentable.id.0.clone(),
                name: agentable.name.clone(),
                dialect: agentable.spec.driver.dialect(),
                limits: agentable.limits,
            })
            .collect()
    }

    async fn open(&self, id: &str, stop: &CancellationToken) -> Result<AppSession, Failure> {
        let (agentable, _) = self.find(id).ok_or(Failure::Unavailable)?;
        let turn = Arc::clone(
            lock(&self.inner.turns)
                .entry(agentable.id.clone())
                .or_default(),
        );
        let mut open = tokio::select! {
            biased;
            () = stop.cancelled() => return Err(Error::Cancelled.into()),
            turn = tokio::time::timeout(agentable.limits.timeout, turn.lock_owned()) => {
                turn.map_err(|_| Failure::Busy)?
            }
        };
        // The policy may have changed while the call waited: the
        // connection as it is now, or none.
        let Some((agentable, host_keys)) = self.find(id) else {
            *open = None;
            return Err(Failure::Unavailable);
        };
        // Changed since it was opened, or opened before every session was
        // closed: opened again as it is now.
        if open
            .as_ref()
            .is_some_and(|open| open.opened_for != agentable || open.epoch != self.inner.epoch())
        {
            *open = None;
        }
        if open.is_none() {
            // Read before the connect: a close of every session while it
            // opens makes this one closed when its call lets it go.
            let epoch = self.inner.epoch();
            let connection = tokio::select! {
                biased;
                () = stop.cancelled() => return Err(Error::Cancelled.into()),
                connection = connect(&self.inner.keyring, &agentable, &host_keys) => connection?,
            };
            // Turned Off or changed while it opened: not used as it was.
            if self.find(id).is_none_or(|(now, _)| now != agentable) {
                return Err(Failure::Unavailable);
            }
            *open = Some(Open {
                connection,
                opened_for: agentable.clone(),
                used: Instant::now(),
                epoch,
            });
        }
        let cancel = open
            .as_ref()
            .map(|open| open.connection.cancel_handle())
            .ok_or(Failure::Unavailable)?;
        Ok(AppSession {
            open,
            gateway: Arc::clone(&self.inner),
            database: agentable.spec.database.clone(),
            stop: stop.clone(),
            cancel,
            lost: std::sync::atomic::AtomicBool::new(false),
        })
    }
}

/// Opens a connection for agents: read-only, with its secrets from the
/// keyring. A connection whose password is asked for each time cannot be
/// opened without the person, and neither can one whose keyring is locked
/// or whose SSH host is not trusted yet. Why it failed goes to the app's
/// log, never to the agent: it can name a host or a user.
///
/// All of it within [`CONNECT_TIME`]: the keyring does one job at a time,
/// and a read for a tab may be waiting on the person to unlock it.
async fn connect(
    keyring: &Keyring,
    agentable: &Agentable,
    host_keys: &HostKeys,
) -> Result<Connection, Failure> {
    match tokio::time::timeout(CONNECT_TIME, opened(keyring, agentable, host_keys)).await {
        Ok(opened) => opened,
        Err(_) => Err(unavailable("it took too long to open")),
    }
}

fn unavailable(why: &str) -> Failure {
    log::warn!("an agent's call could not open a connection: {why}");
    Failure::Unavailable
}

async fn opened(
    keyring: &Keyring,
    agentable: &Agentable,
    host_keys: &HostKeys,
) -> Result<Connection, Failure> {
    let mut secrets = Secrets::default();
    let needed = [
        (
            SecretKind::Database,
            agentable.password,
            password_account(&agentable.id),
        ),
        (
            SecretKind::for_ssh(&agentable.spec).unwrap_or(SecretKind::Database),
            match SecretKind::for_ssh(&agentable.spec) {
                Some(_) => agentable.ssh_secret,
                None => PasswordMode::None,
            },
            ssh_account(&agentable.id),
        ),
    ];
    for (kind, mode, account) in needed {
        match mode {
            PasswordMode::None => {}
            PasswordMode::Ask => return Err(unavailable("its password is asked for each time")),
            // Never a prompt to unlock the keyring: one that came up because
            // an agent asked could fish for the person's password.
            PasswordMode::Keyring => match keyring.read_quietly(&account).wait().await {
                Ok(Some(secret)) => *kind.slot(&mut secrets) = Some(secret),
                Ok(None) => return Err(unavailable("the keyring has no password for it")),
                Err(error) => return Err(unavailable(&error.to_string())),
            },
        }
    }
    Connection::connect_with(&agentable.spec, &secrets, host_keys, Access::ReadOnly)
        .await
        .map_err(|error| unavailable(&error.to_string()))
}

/// A connection's session, held by one call.
pub struct AppSession {
    open: OwnedMutexGuard<Option<Open>>,
    gateway: Arc<Inner>,
    database: String,
    stop: CancellationToken,
    cancel: CancelHandle,
    /// Set when the session answered that it is lost: it is closed when the
    /// call lets it go, and the next call opens it again.
    lost: std::sync::atomic::AtomicBool,
}

/// As the call lets the session go: closed when it was lost, when its
/// connection was turned Off or changed meanwhile, or when every session
/// was closed meanwhile. Else kept for the next call.
impl Drop for AppSession {
    fn drop(&mut self) {
        let lost = self.lost.load(std::sync::atomic::Ordering::SeqCst);
        let kept = self
            .open
            .as_ref()
            .is_some_and(|open| self.gateway.keeps(&open.opened_for, open.epoch));
        if lost || !kept {
            *self.open = None;
        } else if let Some(open) = self.open.as_mut() {
            open.used = Instant::now();
        }
    }
}

impl AppSession {
    fn connection(&self) -> Result<&Connection, Failure> {
        self.open
            .as_ref()
            .map(|open| &open.connection)
            .ok_or(Failure::Unavailable)
    }

    /// Awaits `work` on the session to its end. Once the call is stopped
    /// the database is told to stop it, as the backend stops a script: the
    /// session's cancel is sent again and again, each time a little later,
    /// until the work is over or its cleanup has begun. The work is never
    /// dropped half way.
    async fn ended<T>(
        &self,
        flag: Option<&StopFlag>,
        work: impl Future<Output = Result<T, Error>>,
    ) -> Result<T, Failure> {
        let done = self.stopped(flag, work).await;
        if let Err(error) = &done
            && error.is_connection_lost()
        {
            self.lost.store(true, std::sync::atomic::Ordering::SeqCst);
        }
        done.map_err(Failure::from)
    }

    /// The work, and the cancels a stop sends, as [`Self::ended`] says.
    async fn stopped<T>(
        &self,
        flag: Option<&StopFlag>,
        work: impl Future<Output = Result<T, Error>>,
    ) -> Result<T, Error> {
        tokio::pin!(work);
        tokio::select! {
            done = &mut work => return done,
            () = self.stop.cancelled() => {}
        }
        if let Some(flag) = flag {
            flag.stop();
        }
        // Each cancel is a task of its own, awaited before the session is
        // let go: one that is still on its way must not land on the
        // statement of the next call.
        let sent = Mutex::new(Vec::new());
        let cancels = async {
            let mut pause = CANCEL_AGAIN;
            loop {
                if flag.is_some_and(StopFlag::is_finishing) {
                    std::future::pending::<()>().await;
                }
                lock(&sent).push(tokio::spawn(send_cancel(&self.cancel)));
                tokio::time::sleep(pause).await;
                pause = longer(pause);
            }
        };
        let done = tokio::select! {
            done = &mut work => done,
            () = cancels => unreachable!("the cancels go on until the work is over"),
        };
        let sent = std::mem::take(&mut *lock(&sent));
        for cancel in sent {
            let _ = cancel.await;
        }
        done
    }
}

impl Session for AppSession {
    async fn schemas(&self) -> Result<Schemas, Failure> {
        let connection = self.connection()?;
        let names = self.ended(None, connection.list_schemas()).await?;
        let home = crate::model::first_schema(&names, &self.database);
        Ok(Schemas { names, home })
    }

    async fn objects(&self, schema: &str) -> Result<Vec<ObjectInfo>, Failure> {
        let connection = self.connection()?;
        self.ended(None, connection.list_objects(schema)).await
    }

    async fn describe(&self, object: &ObjectRef) -> Result<Structure, Failure> {
        let connection = self.connection()?;
        self.ended(None, connection.describe(object)).await
    }

    async fn read(&self, read: &Read, limits: Limits) -> Result<Answer, Failure> {
        let connection = self.connection()?;
        let flag = StopFlag::new();
        self.ended(Some(&flag), connection.run_agent_read(read, limits, &flag))
            .await
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use tabletist_db::Dialect;
    use tabletist_db::agent::check_read;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

    use super::*;
    use crate::backend::{Backend, Command, Event, Waker};
    use crate::connections::Agents as AgentSettings;
    use crate::env::Environment;
    use crate::secrets::SecretString;

    const WAIT: Duration = Duration::from_secs(20);

    fn saved(id: &str, name: &str, environment: Environment, spec: ConnectSpec) -> SavedConnection {
        SavedConnection {
            id: ConnectionId(id.into()),
            name: name.into(),
            environment,
            read_only: Some(false),
            password: PasswordMode::None,
            ssh_secret: PasswordMode::None,
            spec,
            agents: AgentSettings {
                access: AgentAccess::Read,
                ..AgentSettings::default()
            },
        }
    }

    /// A new SQLite file of the Bookshop.
    fn bookshop(dir: &Path, file: &str) -> PathBuf {
        let path = dir.join(file);
        tabletist_db::fixtures::write_sqlite_bookshop(&path).unwrap();
        path
    }

    /// A gateway over one connection agents may read.
    fn gateway(connection: SavedConnection) -> AppGateway {
        let gateway = AppGateway::new(Keyring::memory());
        gateway.set_policy(agentable(&[connection]), HostKeys::default());
        gateway
    }

    async fn read(session: &AppSession, dialect: Dialect, sql: &str) -> Result<Answer, Failure> {
        let read = check_read(dialect, sql, &[]).unwrap();
        session
            .read(
                &read,
                Limits {
                    rows: 500,
                    timeout: Duration::from_secs(10),
                },
            )
            .await
    }

    fn the_int(answer: &Answer) -> i64 {
        match answer.rows[0][0] {
            tabletist_db::Value::Int(value) => value,
            ref other => panic!("{other:?}"),
        }
    }

    /// A statement that takes minutes: six copies of the Bookshop's forty
    /// order items, crossed.
    fn slow() -> String {
        let tables: Vec<String> = (1..=6).map(|n| format!("order_items t{n}")).collect();
        let sum: Vec<String> = (1..=6).map(|n| format!("t{n}.quantity")).collect();
        format!(
            "SELECT count(*) FROM {} WHERE {} < 0",
            tables.join(", "),
            sum.join(" + ")
        )
    }

    #[test]
    fn an_agent_calls_a_connection_by_its_name_and_its_environment() {
        let file = || ConnectSpec::sqlite("/tmp/bookshop.db");
        let connections = vec![
            saved("1f0c2a9e-0001", "Bookshop", Environment::Dev, file()),
            saved("7b3d55c1-0002", "Bookshop", Environment::Production, file()),
            saved("c9e81f40-0003", "Scratch", Environment::None, file()),
        ];
        let named: Vec<String> = agentable(&connections)
            .into_iter()
            .map(|agentable| agentable.name)
            .collect();
        assert_eq!(
            named,
            ["Bookshop · dev", "Bookshop · production", "Scratch"]
        );
        let mut few = connections[2].clone();
        few.agents.rows = 100;
        few.agents.timeout_secs = 30;
        assert_eq!(
            agentable(&[few])[0].limits,
            Limits {
                rows: 100,
                timeout: Duration::from_secs(30),
            }
        );
    }

    #[test]
    fn a_name_two_connections_share_never_comes_to_mean_another() {
        let file = || ConnectSpec::sqlite("/tmp/bookshop.db");
        let a = saved("aaaa1111-x", "Shop", Environment::Dev, file());
        let b = saved("bbbb2222-y", "Shop", Environment::Dev, file());
        let c = saved("cccc3333-z", "Shop", Environment::Dev, file());
        let names = |saved: &[SavedConnection]| -> Vec<(String, String)> {
            agentable(saved)
                .into_iter()
                .map(|agentable| (agentable.id.0, agentable.name))
                .collect()
        };
        let pair = |id: &str, name: &str| (id.to_owned(), name.to_owned());
        assert_eq!(
            names(&[a.clone(), b.clone(), c.clone()]),
            [
                pair("aaaa1111-x", "Shop · dev (aaaa1111)"),
                pair("bbbb2222-y", "Shop · dev (bbbb2222)"),
                pair("cccc3333-z", "Shop · dev (cccc3333)"),
            ]
        );
        // One turned Off: the others keep their names.
        let mut off = a.clone();
        off.agents.access = AgentAccess::Off;
        assert_eq!(
            names(&[off, b.clone(), c.clone()]),
            [
                pair("bbbb2222-y", "Shop · dev (bbbb2222)"),
                pair("cccc3333-z", "Shop · dev (cccc3333)"),
            ]
        );
        // One removed: the others keep theirs while two are left.
        assert_eq!(
            names(&[b.clone(), c.clone()]),
            [
                pair("bbbb2222-y", "Shop · dev (bbbb2222)"),
                pair("cccc3333-z", "Shop · dev (cccc3333)"),
            ]
        );
        // Alone again, the last takes the bare name. An agent that learnt
        // "Shop · dev" when it was a's would now read c: what the names
        // keep is no more than the doc of `agentable` says.
        assert_eq!(
            names(std::slice::from_ref(&c)),
            [pair("cccc3333-z", "Shop · dev")]
        );
        // A name that would be another's own takes the whole id.
        let own = saved(
            "dddd4444-w",
            "Shop · dev (bbbb2222)",
            Environment::None,
            file(),
        );
        assert_eq!(
            names(&[b, c, own]),
            [
                pair("bbbb2222-y", "Shop · dev (bbbb2222-y)"),
                pair("cccc3333-z", "Shop · dev (cccc3333)"),
                pair("dddd4444-w", "Shop · dev (bbbb2222)"),
            ]
        );
    }

    #[tokio::test]
    async fn a_call_opens_the_connection_read_only_and_the_next_call_has_it_too() {
        let dir = tempfile::tempdir().unwrap();
        let spec = ConnectSpec::sqlite(bookshop(dir.path(), "bookshop.db"));
        // The connection's own box lets it write: an agent's session never.
        let gateway = gateway(saved("shop", "Bookshop", Environment::Dev, spec));
        assert_eq!(gateway.connections()[0].name, "Bookshop · dev");
        let stop = CancellationToken::new();
        let first = gateway.open("shop", &stop).await.unwrap();
        assert_eq!(first.connection().unwrap().access(), Access::ReadOnly);
        let answer = read(&first, Dialect::Sqlite, "SELECT count(*) FROM books").await;
        assert_eq!(the_int(&answer.unwrap()), 12);
        let schemas = first.schemas().await.unwrap();
        assert_eq!(schemas.home.as_deref(), Some("main"));
        let opened = std::ptr::from_ref(first.connection().unwrap());
        drop(first);
        let again = gateway.open("shop", &stop).await.unwrap();
        assert_eq!(std::ptr::from_ref(again.connection().unwrap()), opened);
    }

    #[tokio::test]
    async fn a_connection_that_needs_a_person_is_not_opened() {
        let dir = tempfile::tempdir().unwrap();
        let server = ConnectSpec {
            driver: tabletist_db::Driver::Postgres,
            host: "db.example.invalid".into(),
            port: 5432,
            user: "app".into(),
            database: "bookshop".into(),
            ..ConnectSpec::default()
        };
        let mut asks = saved("asks", "Asks", Environment::Dev, server.clone());
        asks.password = PasswordMode::Ask;
        let mut kept = saved("kept", "Kept", Environment::Dev, server);
        kept.password = PasswordMode::Keyring;
        let gone = saved(
            "gone",
            "Gone",
            Environment::Dev,
            ConnectSpec::sqlite(dir.path().join("no-such.db")),
        );
        let gateway = AppGateway::new(Keyring::memory());
        gateway.set_policy(agentable(&[asks, kept, gone]), HostKeys::default());
        let stop = CancellationToken::new();
        // Asked for each time; nothing in the keyring; no such file.
        for id in ["asks", "kept", "gone"] {
            let opened = gateway.open(id, &stop).await;
            assert!(matches!(opened, Err(Failure::Unavailable)), "{id}");
        }
        // And one that is not there at all.
        assert!(matches!(
            gateway.open("payroll", &stop).await,
            Err(Failure::Unavailable)
        ));
    }

    /// A keyring that is locked: a read would ask the person to unlock it,
    /// and says that it was made.
    struct Locked(Arc<std::sync::atomic::AtomicBool>);

    impl crate::secrets::SecretStore for Locked {
        fn read(&mut self, _account: &str) -> Result<Option<String>, crate::secrets::SecretError> {
            self.0.store(true, std::sync::atomic::Ordering::SeqCst);
            Err(crate::secrets::SecretError::Locked)
        }

        fn read_quietly(
            &mut self,
            _account: &str,
        ) -> Result<Option<String>, crate::secrets::SecretError> {
            Err(crate::secrets::SecretError::Locked)
        }

        fn write(
            &mut self,
            _account: &str,
            _secret: &str,
        ) -> Result<(), crate::secrets::SecretError> {
            Err(crate::secrets::SecretError::Locked)
        }

        fn delete(&mut self, _account: &str) -> Result<(), crate::secrets::SecretError> {
            Err(crate::secrets::SecretError::Locked)
        }
    }

    #[tokio::test]
    async fn a_locked_keyring_is_not_asked_to_be_unlocked() {
        let mut kept = saved(
            "kept",
            "Kept",
            Environment::Dev,
            ConnectSpec {
                driver: tabletist_db::Driver::Postgres,
                host: "db.example.invalid".into(),
                port: 5432,
                ..ConnectSpec::default()
            },
        );
        kept.password = PasswordMode::Keyring;
        let asked = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let gateway = AppGateway::new(Keyring::start(Locked(Arc::clone(&asked))));
        gateway.set_policy(agentable(&[kept]), HostKeys::default());
        let opened = gateway.open("kept", &CancellationToken::new()).await;
        assert!(matches!(opened, Err(Failure::Unavailable)));
        assert!(
            !asked.load(std::sync::atomic::Ordering::SeqCst),
            "a read that would prompt"
        );
    }

    /// A keyring whose job before an agent's is a read for a tab, waiting
    /// on the person to unlock it: every read waits until `answered` is
    /// dropped.
    struct Waiting(std::sync::mpsc::Receiver<()>);

    impl crate::secrets::SecretStore for Waiting {
        fn read(&mut self, _account: &str) -> Result<Option<String>, crate::secrets::SecretError> {
            let _ = self.0.recv();
            Ok(None)
        }

        fn write(
            &mut self,
            _account: &str,
            _secret: &str,
        ) -> Result<(), crate::secrets::SecretError> {
            Ok(())
        }

        fn delete(&mut self, _account: &str) -> Result<(), crate::secrets::SecretError> {
            Ok(())
        }
    }

    #[tokio::test(start_paused = true)]
    async fn a_keyring_that_does_not_answer_holds_the_connection_no_longer_than_a_connect() {
        let mut kept = saved(
            "kept",
            "Kept",
            Environment::Dev,
            ConnectSpec {
                driver: tabletist_db::Driver::Postgres,
                host: "db.example.invalid".into(),
                port: 5432,
                ..ConnectSpec::default()
            },
        );
        kept.password = PasswordMode::Keyring;
        let (answered, waits) = std::sync::mpsc::channel();
        let gateway = AppGateway::new(Keyring::start(Waiting(waits)));
        gateway.set_policy(agentable(&[kept]), HostKeys::default());
        let started = tokio::time::Instant::now();
        let opened = gateway.open("kept", &CancellationToken::new()).await;
        assert!(matches!(opened, Err(Failure::Unavailable)));
        assert_eq!(started.elapsed(), CONNECT_TIME);
        // And the connection is free for the next call, which waits as long.
        let opened = gateway.open("kept", &CancellationToken::new()).await;
        assert!(matches!(opened, Err(Failure::Unavailable)));
        assert_eq!(started.elapsed(), 2 * CONNECT_TIME);
        drop(answered);
    }

    #[tokio::test]
    async fn a_call_that_waited_while_its_connection_was_turned_off_runs_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let spec = ConnectSpec::sqlite(bookshop(dir.path(), "bookshop.db"));
        let connection = saved("shop", "Bookshop", Environment::Dev, spec);
        let gateway = gateway(connection.clone());
        let stop = CancellationToken::new();
        let holding = gateway.open("shop", &stop).await.unwrap();
        let (waited, ()) = tokio::join!(gateway.open("shop", &stop), async {
            tokio::time::sleep(Duration::from_millis(100)).await;
            let mut off = connection;
            off.agents.access = AgentAccess::Off;
            gateway.set_policy(agentable(&[off]), HostKeys::default());
            drop(holding);
        });
        assert!(matches!(waited, Err(Failure::Unavailable)));
    }

    #[tokio::test]
    async fn a_second_call_waits_its_turn_and_then_is_busy() {
        let dir = tempfile::tempdir().unwrap();
        let spec = ConnectSpec::sqlite(bookshop(dir.path(), "bookshop.db"));
        let mut connection = saved("shop", "Bookshop", Environment::Dev, spec);
        connection.agents.timeout_secs = 1;
        let gateway = gateway(connection);
        let stop = CancellationToken::new();
        let first = gateway.open("shop", &stop).await.unwrap();
        let started = Instant::now();
        let second = gateway.open("shop", &stop).await;
        assert!(matches!(second, Err(Failure::Busy)));
        assert!(started.elapsed() >= Duration::from_secs(1));
        drop(first);
        assert!(gateway.open("shop", &stop).await.is_ok());
        // A call that is stopped while it waits stops waiting.
        let holding = gateway.open("shop", &stop).await.unwrap();
        let stopped = CancellationToken::new();
        stopped.cancel();
        let waiting = gateway.open("shop", &stopped).await;
        assert!(matches!(waiting, Err(Failure::Database(Error::Cancelled))));
        drop(holding);
    }

    #[test]
    fn a_name_two_connections_came_to_share_changes_for_both() {
        let file = || ConnectSpec::sqlite("/tmp/bookshop.db");
        let x = saved("aaaa1111-x", "Shop", Environment::Dev, file());
        let y = saved("bbbb2222-y", "Shop", Environment::Dev, file());
        let names = |saved: &[SavedConnection]| -> Vec<String> {
            agentable(saved)
                .into_iter()
                .map(|agentable| agentable.name)
                .collect()
        };
        assert_eq!(names(std::slice::from_ref(&x)), ["Shop · dev"]);
        // Another called the same is added: neither is the bare name now.
        assert_eq!(
            names(&[x.clone(), y.clone()]),
            ["Shop · dev (aaaa1111)", "Shop · dev (bbbb2222)"]
        );
        // And the first removed: the bare name is the second's.
        assert_eq!(names(&[y]), ["Shop · dev"]);
    }

    #[tokio::test]
    async fn a_turn_is_kept_while_a_call_holds_it_whatever_the_policy_does() {
        let dir = tempfile::tempdir().unwrap();
        let spec = ConnectSpec::sqlite(bookshop(dir.path(), "bookshop.db"));
        let mut connection = saved("shop", "Bookshop", Environment::Dev, spec);
        connection.agents.timeout_secs = 1;
        let gateway = gateway(connection.clone());
        let stop = CancellationToken::new();
        let holding = gateway.open("shop", &stop).await.unwrap();
        // Off, and Read again, while the call holds the session.
        let mut off = connection.clone();
        off.agents.access = AgentAccess::Off;
        gateway.set_policy(agentable(&[off]), HostKeys::default());
        gateway.set_policy(agentable(&[connection]), HostKeys::default());
        // Every session closed, too.
        gateway.close_all();
        // Still one call at a time.
        assert!(matches!(
            gateway.open("shop", &stop).await,
            Err(Failure::Busy)
        ));
        let turn = Arc::clone(&lock(&gateway.inner.turns)[&ConnectionId("shop".into())]);
        drop(holding);
        // Let go, it is closed: every session was closed while it was held.
        assert!(turn.try_lock().unwrap().is_none());
        let next = gateway.open("shop", &stop).await.unwrap();
        let count = read(&next, Dialect::Sqlite, "SELECT count(*) FROM books").await;
        assert_eq!(the_int(&count.unwrap()), 12);
    }

    #[tokio::test]
    async fn a_connection_taken_away_or_changed_is_not_used_as_it_was() {
        let dir = tempfile::tempdir().unwrap();
        let first = bookshop(dir.path(), "first.db");
        // Another database altogether: the one the app's demo opens.
        let second = dir.path().join("second.db");
        tabletist_db::fixtures::write_sqlite_demo(&second).unwrap();
        let shop = |path: &Path| {
            saved(
                "shop",
                "Bookshop",
                Environment::Dev,
                ConnectSpec::sqlite(path),
            )
        };
        let gateway = gateway(shop(&first));
        let stop = CancellationToken::new();
        let session = gateway.open("shop", &stop).await.unwrap();
        let count = read(&session, Dialect::Sqlite, "SELECT count(*) FROM books").await;
        assert_eq!(the_int(&count.unwrap()), 12);
        drop(session);
        // Pointed at another file: opened again, on that file, which has
        // no books.
        gateway.set_policy(agentable(&[shop(&second)]), HostKeys::default());
        let session = gateway.open("shop", &stop).await.unwrap();
        let count = read(&session, Dialect::Sqlite, "SELECT count(*) FROM books").await;
        assert!(
            matches!(count, Err(Failure::Database(Error::Query { .. }))),
            "{count:?}"
        );
        drop(session);
        // Turned Off: gone.
        let mut off = shop(&second);
        off.agents.access = AgentAccess::Off;
        gateway.set_policy(agentable(&[off]), HostKeys::default());
        assert_eq!(gateway.connections(), []);
        assert!(matches!(
            gateway.open("shop", &stop).await,
            Err(Failure::Unavailable)
        ));
        let turn = Arc::clone(&lock(&gateway.inner.turns)[&ConnectionId("shop".into())]);
        assert!(turn.try_lock().unwrap().is_none(), "closed");
    }

    #[tokio::test]
    async fn a_stop_ends_a_statement_at_the_database_and_the_session_goes_on() {
        let dir = tempfile::tempdir().unwrap();
        let spec = ConnectSpec::sqlite(bookshop(dir.path(), "bookshop.db"));
        let gateway = gateway(saved("shop", "Bookshop", Environment::Dev, spec));
        let stop = CancellationToken::new();
        let session = gateway.open("shop", &stop).await.unwrap();
        let slow = slow();
        let started = Instant::now();
        let (ended, ()) = tokio::join!(read(&session, Dialect::Sqlite, &slow), async {
            tokio::time::sleep(Duration::from_millis(300)).await;
            stop.cancel();
        });
        assert!(
            matches!(ended, Err(Failure::Database(Error::Cancelled))),
            "{ended:?}"
        );
        assert!(started.elapsed() < Duration::from_secs(5));
        drop(session);
        let next = gateway
            .open("shop", &CancellationToken::new())
            .await
            .unwrap();
        let count = read(&next, Dialect::Sqlite, "SELECT count(*) FROM books").await;
        assert_eq!(the_int(&count.unwrap()), 12);
    }

    #[tokio::test]
    async fn a_connection_left_idle_is_closed_and_one_in_use_is_not() {
        let dir = tempfile::tempdir().unwrap();
        let spec = ConnectSpec::sqlite(bookshop(dir.path(), "bookshop.db"));
        let gateway = gateway(saved("shop", "Bookshop", Environment::Dev, spec));
        let stop = CancellationToken::new();
        let session = gateway.open("shop", &stop).await.unwrap();
        let turn = Arc::clone(&lock(&gateway.inner.turns)[&ConnectionId("shop".into())]);
        gateway.close_idle(Duration::ZERO);
        assert!(session.connection().is_ok(), "in use");
        drop(session);
        assert!(turn.try_lock().unwrap().is_some());
        gateway.close_idle(Duration::from_secs(60));
        assert!(turn.try_lock().unwrap().is_some(), "not idle so long yet");
        gateway.close_idle(Duration::ZERO);
        assert!(turn.try_lock().unwrap().is_none());
        // And opened again for the next call.
        assert!(gateway.open("shop", &stop).await.is_ok());
    }

    /// One line of the protocol, sent, and the answer to it, when it has an
    /// id.
    async fn said<R: tokio::io::AsyncBufRead + Unpin, W: tokio::io::AsyncWrite + Unpin>(
        lines: &mut tokio::io::Lines<R>,
        write: &mut W,
        message: serde_json::Value,
    ) -> Option<serde_json::Value> {
        write
            .write_all(format!("{message}\n").as_bytes())
            .await
            .unwrap();
        message.get("id")?;
        let line = lines.next_line().await.unwrap().unwrap();
        Some(serde_json::from_str(&line).unwrap())
    }

    #[tokio::test]
    async fn the_server_reads_the_bookshop_through_the_apps_gateway() {
        use serde_json::json;
        let dir = tempfile::tempdir().unwrap();
        let spec = ConnectSpec::sqlite(bookshop(dir.path(), "bookshop.db"));
        let gateway = gateway(saved("shop", "Bookshop", Environment::Dev, spec));
        let log = tabletist_mcp::Log::open(&dir.path().join("agents.sqlite3"))
            .unwrap()
            .log;
        let server = tabletist_mcp::Server::new(gateway, log.clone(), "test");
        let (ours, theirs) = tokio::io::duplex(1024 * 1024);
        let served = tokio::spawn({
            let server = server.clone();
            async move {
                let origin = tabletist_mcp::Origin {
                    transport: tabletist_mcp::Transport::Stdio,
                    project: None,
                };
                server.serve(theirs, origin).await;
            }
        });
        let (read, mut write) = tokio::io::split(ours);
        let mut lines = BufReader::new(read).lines();
        let started = said(
            &mut lines,
            &mut write,
            json!({
                "jsonrpc": "2.0",
                "id": 0,
                "method": "initialize",
                "params": {
                    "protocolVersion": "2025-11-25",
                    "capabilities": {},
                    "clientInfo": { "name": "claude-code", "version": "1" },
                },
            }),
        )
        .await;
        assert!(started.unwrap()["result"].is_object());
        said(
            &mut lines,
            &mut write,
            json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }),
        )
        .await;
        let call = |id: u64, name: &str, arguments: serde_json::Value| {
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": "tools/call",
                "params": { "name": name, "arguments": arguments },
            })
        };
        let listed = said(
            &mut lines,
            &mut write,
            call(1, "list_connections", json!({})),
        )
        .await;
        let text = listed.unwrap()["result"]["content"][0]["text"].clone();
        assert!(
            text.as_str()
                .unwrap()
                .contains("\"name\":\"Bookshop · dev\""),
            "{text}"
        );
        let arguments =
            json!({ "connection": "Bookshop · dev", "sql": "SELECT count(*) FROM books" });
        let ran = said(&mut lines, &mut write, call(2, "run_query", arguments)).await;
        let text = ran.unwrap()["result"]["content"][0]["text"].clone();
        assert!(text.as_str().unwrap().contains("\"rows\":[[12]]"), "{text}");
        write.shutdown().await.unwrap();
        served.await.unwrap();
        let entries = log.latest(10).await.unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].connection_id.as_deref(), Some("shop"));
    }

    #[test]
    fn the_backend_starts_and_stops_the_server_and_keeps_its_log_to_its_days() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("agents.sqlite3");
        // A call of long ago, and one of now.
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            let log = Log::open(&path).unwrap().log;
            for at_ms in [
                1_000,
                i64::try_from(
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_millis(),
                )
                .unwrap(),
            ] {
                let entry = tabletist_mcp::Entry {
                    at_ms,
                    session: "s".into(),
                    agent: "a".into(),
                    transport: "stdio".into(),
                    tool: "list_connections".into(),
                    result: "ok".into(),
                    ..tabletist_mcp::Entry::default()
                };
                log.record(entry).await.unwrap();
            }
        });
        let mut backend = Backend::start_with(Waker::default(), Keyring::memory());
        backend.send(Command::Agents(AgentsCommand::Start {
            log: path.clone(),
            log_days: 30,
            endpoint: None,
        }));
        assert_eq!(
            match told(&mut backend, WAIT) {
                Some(Event::Agents(event)) => event,
                other => panic!("{other:?}"),
            },
            AgentsEvent::Started { kept_aside: None }
        );
        // Retention runs as the server starts.
        let deadline = Instant::now() + WAIT;
        let left = loop {
            let left = runtime.block_on(async {
                Log::open(&path)
                    .unwrap()
                    .log
                    .latest(10)
                    .await
                    .unwrap()
                    .len()
            });
            if left == 1 || Instant::now() > deadline {
                break left;
            }
            std::thread::sleep(Duration::from_millis(50));
        };
        assert_eq!(left, 1, "the call of long ago is forgotten");
        backend.send(Command::Agents(AgentsCommand::Stop));
        assert!(matches!(
            told(&mut backend, WAIT),
            Some(Event::Agents(AgentsEvent::Stopped))
        ));
        // A log that cannot be made: the server does not start, and says
        // why.
        backend.send(Command::Agents(AgentsCommand::Start {
            log: dir.path().join("no-such-folder").join("agents.sqlite3"),
            log_days: 30,
            endpoint: None,
        }));
        match told(&mut backend, WAIT) {
            Some(Event::Agents(AgentsEvent::Failed(said))) => {
                assert!(
                    said.starts_with("the log of agents' calls could not be opened"),
                    "{said}"
                );
            }
            other => panic!("{other:?}"),
        }
    }

    /// Another Tabletist serves agents: this one's server does not start,
    /// and does not open the log the other has open, which one of another
    /// version would set aside.
    #[cfg(unix)]
    #[test]
    fn a_start_where_another_tabletist_serves_agents_leaves_its_log_alone() {
        let dir = tempfile::tempdir().unwrap();
        let at = dir.path().join("run");
        std::fs::create_dir(&at).unwrap();
        let another = std::fs::File::create(at.join(crate::endpoint::LOCK)).unwrap();
        another.lock().unwrap();
        let path = dir.path().join("agents.sqlite3");
        let mut backend = Backend::start_with(Waker::default(), Keyring::memory());
        backend.send(Command::Agents(AgentsCommand::Start {
            log: path.clone(),
            log_days: 30,
            endpoint: Some(at),
        }));
        match told(&mut backend, WAIT) {
            // Said as it is: the app puts its own words before it.
            Some(Event::Agents(AgentsEvent::Failed(said))) => {
                assert_eq!(said, crate::endpoint::ANOTHER_SERVES);
            }
            other => panic!("{other:?}"),
        }
        assert!(!path.exists());
    }

    #[test]
    fn the_log_is_read_with_the_server_off_and_reading_it_makes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("agents.sqlite3");
        let mut backend = Backend::start_with(Waker::default(), Keyring::memory());
        let read = |backend: &mut Backend, connection: Option<&str>| {
            backend.send(Command::Agents(AgentsCommand::ReadLog {
                log: path.clone(),
                connection: connection.map(str::to_owned),
                before: None,
                asked: 7,
            }));
            match told(backend, WAIT) {
                Some(Event::Agents(AgentsEvent::LogRead { asked: 7, page })) => page.unwrap(),
                other => panic!("{other:?}"),
            }
        };
        // No log yet: an empty page, and no file made.
        assert_eq!(read(&mut backend, None), tabletist_mcp::Page::default());
        assert!(!path.exists());
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            let log = Log::open(&path).unwrap().log;
            for on in ["shop", "ledger"] {
                let entry = tabletist_mcp::Entry {
                    at_ms: 1,
                    session: "s".into(),
                    agent: "Claude Code".into(),
                    transport: "stdio".into(),
                    connection_id: Some(on.into()),
                    tool: "run_query".into(),
                    result: "ok".into(),
                    ..tabletist_mcp::Entry::default()
                };
                log.record(entry).await.unwrap();
            }
        });
        let shop = read(&mut backend, Some("shop"));
        assert_eq!((shop.calls.len(), shop.total), (1, 1));
        assert_eq!(read(&mut backend, None).total, 2);
    }

    #[test]
    fn a_server_says_its_sessions_as_it_starts() {
        let dir = tempfile::tempdir().unwrap();
        let mut backend = Backend::start_with(Waker::default(), Keyring::memory());
        backend.send(Command::Agents(AgentsCommand::Start {
            log: dir.path().join("agents.sqlite3"),
            log_days: 30,
            endpoint: None,
        }));
        // None yet, said without a change to wait for: a session that
        // opened before the app heard of the start is not missed.
        let mut sessions = None;
        while let Some(Event::Agents(event)) = backend.wait(WAIT) {
            if let AgentsEvent::Sessions(open) = event {
                sessions = Some(open);
                break;
            }
        }
        assert_eq!(sessions, Some(Vec::new()));
    }

    #[test]
    fn starts_and_stops_one_after_another_leave_the_last_one_standing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("agents.sqlite3");
        let mut backend = Backend::start_with(Waker::default(), Keyring::memory());
        let start = || {
            Command::Agents(AgentsCommand::Start {
                log: path.clone(),
                log_days: 30,
                endpoint: None,
            })
        };
        backend.send(start());
        backend.send(start());
        backend.send(Command::Agents(AgentsCommand::Stop));
        backend.send(start());
        let mut said = Vec::new();
        while let Some(Event::Agents(event)) = told(&mut backend, Duration::from_secs(2)) {
            said.push(event);
        }
        assert_eq!(
            said,
            [
                AgentsEvent::Stopped,
                AgentsEvent::Started { kept_aside: None }
            ],
            "{said:?}"
        );
        assert!(!dir.path().join("agents.sqlite3.bad").exists());
    }

    /// The backend's next event but the sessions and the calls, which it
    /// says beside what the server does.
    fn told(backend: &mut Backend, wait: Duration) -> Option<Event> {
        loop {
            match backend.wait(wait)? {
                Event::Agents(AgentsEvent::Sessions(_) | AgentsEvent::Called(_)) => {}
                event => return Some(event),
            }
        }
    }

    /// A server's connection from a test URL, its password in the keyring.
    async fn on_a_server(url_variable: &str) -> Option<(AppGateway, Dialect)> {
        let Ok(url) = std::env::var(url_variable) else {
            eprintln!("skipped: {url_variable} is not set");
            return None;
        };
        let (mut spec, secrets) = ConnectSpec::from_url(&url).unwrap();
        spec.tls = tabletist_db::TlsMode::Disable;
        let dialect = spec.driver.dialect();
        let mut connection = saved("server", "Server", Environment::Dev, spec);
        connection.password = PasswordMode::Keyring;
        let keyring = Keyring::memory();
        let password = secrets.password.unwrap_or_default();
        keyring
            .write(&password_account(&connection.id), SecretString(password))
            .wait()
            .await
            .unwrap();
        let gateway = AppGateway::new(keyring);
        gateway.set_policy(agentable(&[connection]), HostKeys::default());
        Some((gateway, dialect))
    }

    async fn the_password_is_the_keyrings_and_a_stop_reaches_the_server(
        url_variable: &str,
        slow: &str,
    ) {
        let Some((gateway, dialect)) = on_a_server(url_variable).await else {
            return;
        };
        let stop = CancellationToken::new();
        let session = gateway.open("server", &stop).await.unwrap();
        let one = read(&session, dialect, "SELECT 1").await.unwrap();
        assert_eq!(one.rows.len(), 1);
        let started = Instant::now();
        let (ended, ()) = tokio::join!(read(&session, dialect, slow), async {
            tokio::time::sleep(Duration::from_millis(500)).await;
            stop.cancel();
        });
        assert!(
            matches!(ended, Err(Failure::Database(Error::Cancelled))),
            "{ended:?}"
        );
        assert!(
            started.elapsed() < Duration::from_secs(8),
            "{:?}",
            started.elapsed()
        );
        drop(session);
        let next = gateway
            .open("server", &CancellationToken::new())
            .await
            .unwrap();
        assert!(read(&next, dialect, "SELECT 1").await.is_ok());
    }

    #[tokio::test]
    async fn on_postgres_a_session_that_was_lost_is_opened_again() {
        let Some((gateway, dialect)) = on_a_server("TABLETIST_TEST_PG_URL").await else {
            return;
        };
        let stop = CancellationToken::new();
        let session = gateway.open("server", &stop).await.unwrap();
        let pid = the_int(
            &read(&session, dialect, "SELECT pg_backend_pid()")
                .await
                .unwrap(),
        );
        drop(session);
        // Ended from another session, as a server restart or an admin
        // would.
        let url = std::env::var("TABLETIST_TEST_PG_URL").unwrap();
        let (mut spec, secrets) = ConnectSpec::from_url(&url).unwrap();
        spec.tls = tabletist_db::TlsMode::Disable;
        let admin = Connection::connect(&spec, &secrets).await.unwrap();
        let text = format!("SELECT pg_terminate_backend({pid})");
        let statements = tabletist_db::sql::statements(dialect, &text);
        admin
            .run_script(
                &statements,
                1,
                tabletist_db::ScriptMode::ReadOnly,
                &StopFlag::new(),
            )
            .await
            .unwrap();
        let session = gateway.open("server", &stop).await.unwrap();
        let lost = read(&session, dialect, "SELECT 1").await;
        assert!(
            matches!(lost, Err(Failure::Database(ref error)) if error.is_connection_lost()),
            "{lost:?}"
        );
        drop(session);
        // The next call has a session again.
        let session = gateway.open("server", &stop).await.unwrap();
        assert!(read(&session, dialect, "SELECT 1").await.is_ok());
    }

    #[tokio::test]
    async fn on_postgres_the_password_is_the_keyrings_and_a_stop_reaches_the_server() {
        the_password_is_the_keyrings_and_a_stop_reaches_the_server(
            "TABLETIST_TEST_PG_URL",
            "SELECT count(*) FROM generate_series(1, 2000000000)",
        )
        .await;
    }

    #[tokio::test]
    async fn on_mysql_the_password_is_the_keyrings_and_a_stop_reaches_the_server() {
        the_password_is_the_keyrings_and_a_stop_reaches_the_server(
            "TABLETIST_TEST_MYSQL_URL",
            "SELECT count(*) FROM information_schema.columns a, information_schema.columns b, \
             information_schema.columns c",
        )
        .await;
    }
}
