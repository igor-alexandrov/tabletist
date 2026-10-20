//! The server: the sessions agents have open, and a call's way through
//! the tools into the log.

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::{Map, Value as Json};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::{RwLock, broadcast, watch};
use tokio_util::sync::CancellationToken;

use crate::gateway::Gateway;
use crate::log::{Entry, Log};
use crate::refusal::Refused;
use crate::stop::{Stop, Why};
use crate::tools::{self, Done, Invalid};

/// How an agent is connected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transport {
    /// Through `tabletist mcp`, which the agent starts.
    Stdio,
}

impl Transport {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stdio => "stdio",
        }
    }
}

/// Where a session comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Origin {
    pub transport: Transport,
    /// What the agent is working on, for the person to tell two sessions
    /// apart: the name of the directory it was started in.
    pub project: Option<String>,
}

/// A session as the app shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionInfo {
    pub id: String,
    /// What the client calls itself, once it has said. A label, not an
    /// identity: a client may call itself anything.
    pub agent: Option<String>,
    pub project: Option<String>,
    pub transport: Transport,
    pub connected_at: SystemTime,
    pub calls: u64,
    /// The connections its calls have named, by id, the one it named last
    /// at the end: where it works, and where it works now.
    pub connections: Vec<String>,
    /// When its last call came, once it has made one.
    pub last_call_at: Option<SystemTime>,
}

/// A call, as the app shows it as it ends: what the log keeps of it but
/// its values, with its statement cut to its start.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Call {
    /// When it arrived.
    pub at: SystemTime,
    pub session: String,
    /// What the client calls itself, as the log has it.
    pub agent: String,
    /// The id of the connection it named, where it named one an agent may
    /// see.
    pub connection: Option<String>,
    pub tool: String,
    /// Whether it brought a statement: one that ran, or that was refused.
    pub statement: bool,
    /// Its place in the log ([`crate::Logged::id`]), where the log took it.
    pub logged: Option<i64>,
    /// What it asked: the statement, or the tables, on one line and cut
    /// to [`CALL_CHARS`] characters.
    pub request: Option<String>,
    /// As the log has it: `ok`, `refused`, `error`, `timeout`,
    /// `cancelled` or `stopped`.
    pub result: String,
    /// The refusal's sentence, as the log keeps it.
    pub message: Option<String>,
    /// The rows the agent was sent, or the tables it was described.
    pub rows: Option<u64>,
    pub columns: Option<u64>,
    pub took: Duration,
}

/// The most characters of a call's request [`Server::calls`] tells.
pub const CALL_CHARS: usize = 200;

/// How many calls [`Server::calls`] holds for a receiver that is behind.
const CALLS_HELD: usize = 256;

/// `text` on one line, its runs of white space one space, and cut to
/// [`CALL_CHARS`] characters.
fn one_line(text: &str) -> String {
    let line = text.split_whitespace().collect::<Vec<_>>().join(" ");
    match line.char_indices().nth(CALL_CHARS) {
        Some((end, _)) => format!("{}…", &line[..end]),
        None => line,
    }
}

/// What stands for an agent's name until it has given one.
const UNNAMED: &str = "unknown";

/// The most characters of a name a client gives itself.
const MAX_AGENT_CHARS: usize = 64;

/// How long a session that was stopped is left to send the answers of the
/// calls it was stopped in, before its transport is closed.
const LAST_ANSWERS: Duration = Duration::from_secs(5);

/// The most calls of one session that run at once. One more is refused,
/// so that a client cannot have the server hold calls without end.
pub(crate) const MAX_CALLS_AT_ONCE: usize = 16;

/// One agent's session, while it is open.
#[derive(Debug)]
pub(crate) struct Live {
    id: String,
    origin: Origin,
    agent: OnceLock<String>,
    connected_at: SystemTime,
    calls: AtomicU64,
    /// Ended when the person stops the session or the server, or when the
    /// client is gone. Every call's stop is a child of it.
    pub stop: Stop,
    /// Held shared by each call from its start to its entry in the log.
    in_flight: RwLock<()>,
    /// How many calls are running.
    running: AtomicUsize,
    /// The connections its calls named, and when the last one came.
    used: Mutex<Used>,
    /// The server's, told of each change to the session.
    changed: Arc<watch::Sender<()>>,
}

/// Where a session has worked, and when it last did.
#[derive(Debug, Default)]
struct Used {
    connections: Vec<String>,
    last_call_at: Option<SystemTime>,
}

impl Live {
    /// Names the agent, by the first name its client gives: its title, or
    /// its name where the title is nothing a person could read.
    pub fn name_agent(&self, title: Option<&str>, name: &str) {
        let label = title
            .map(label)
            .filter(|label| !label.is_empty())
            .unwrap_or_else(|| label(name));
        if !label.is_empty() && self.agent.set(label).is_ok() {
            self.changed.send_replace(());
        }
    }

    /// Waits until no call of the session is in flight, as long as
    /// [`LAST_ANSWERS`].
    pub async fn settled(&self) {
        let _ = tokio::time::timeout(LAST_ANSWERS, self.in_flight.write()).await;
    }

    fn info(&self) -> SessionInfo {
        let used = self.used.lock().unwrap_or_else(PoisonError::into_inner);
        SessionInfo {
            id: self.id.clone(),
            agent: self.agent.get().cloned(),
            project: self.origin.project.clone(),
            transport: self.origin.transport,
            connected_at: self.connected_at,
            calls: self.calls.load(Ordering::Relaxed),
            connections: used.connections.clone(),
            last_call_at: used.last_call_at,
        }
    }

    /// Notes a call that came at `at`, and the connection it named, which
    /// goes to the end of those it has named. A call that ends after a
    /// later one takes back neither the time nor the end: the later one
    /// named where the session works now.
    fn called(&self, at: SystemTime, connection: Option<&str>) {
        let mut used = self.used.lock().unwrap_or_else(PoisonError::into_inner);
        let latest = used.last_call_at.is_none_or(|last| at >= last);
        used.last_call_at = used.last_call_at.max(Some(at));
        if let Some(id) = connection {
            let named = used.connections.iter().any(|named| named == id);
            if latest {
                used.connections.retain(|named| named != id);
                used.connections.push(id.to_owned());
            } else if !named {
                // Where it worked before, which is not where it works now.
                used.connections.insert(0, id.to_owned());
            }
        }
        drop(used);
        self.changed.send_replace(());
    }
}

/// A name a client gives itself, as a label a person reads: nothing that
/// moves the cursor, turns the line around or cannot be seen, and of a fit
/// length.
fn label(name: &str) -> String {
    let unseen = |c: char| {
        c.is_control()
            || matches!(
                c,
                '\u{200B}'..='\u{200F}'
                    | '\u{2028}'..='\u{202E}'
                    | '\u{2060}'..='\u{2069}'
                    | '\u{FEFF}'
            )
    };
    let label: String = name
        .chars()
        .filter(|c| !unseen(*c))
        .take(MAX_AGENT_CHARS)
        .collect();
    label.trim().to_owned()
}

/// What a call answers.
#[derive(Debug)]
pub(crate) enum Reply {
    /// The answer, as JSON.
    Answer(String),
    Refused(Refused),
}

/// A session among the open ones, until this is dropped: when its serving
/// ends, and also when whoever served it gave up half way.
struct Open<G: Gateway> {
    server: Server<G>,
    live: Arc<Live>,
}

impl<G: Gateway> Drop for Open<G> {
    fn drop(&mut self) {
        // Nobody is there to read an answer now.
        self.live.stop.end(Why::Client);
        self.server
            .sessions()
            .retain(|open| !Arc::ptr_eq(open, &self.live));
        self.server.inner.changed.send_replace(());
    }
}

/// One more call running, until this is dropped.
struct Running<'a>(&'a AtomicUsize);

impl<'a> Running<'a> {
    /// Counts a call in, and says how many were running before it.
    fn count(running: &'a AtomicUsize) -> (Self, usize) {
        let before = running.fetch_add(1, Ordering::SeqCst);
        (Self(running), before)
    }
}

impl Drop for Running<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

/// The MCP server. A clone is the same server.
///
/// Nothing in it listens: whoever has a transport an agent is on the other
/// end of hands it to [`Server::serve`].
pub struct Server<G: Gateway> {
    inner: Arc<Inner<G>>,
}

impl<G: Gateway> Clone for Server<G> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

struct Inner<G> {
    gateway: G,
    log: Log,
    version: String,
    sessions: Mutex<Vec<Arc<Live>>>,
    /// Ended by [`Server::stop`]. Every session's stop is a child of it.
    stop: Stop,
    /// Sent to when a session opens or ends, names its agent, or makes a
    /// call: what [`Server::open_sessions`] says has changed.
    changed: Arc<watch::Sender<()>>,
    /// Each call, as it ends.
    calls: broadcast::Sender<Call>,
}

impl<G: Gateway> Server<G> {
    /// A server over `gateway` that writes its calls to `log`. `version`
    /// is the app's, which a client is told.
    pub fn new(gateway: G, log: Log, version: impl Into<String>) -> Self {
        Self {
            inner: Arc::new(Inner {
                gateway,
                log,
                version: version.into(),
                sessions: Mutex::default(),
                stop: Stop::new(),
                changed: Arc::new(watch::Sender::new(())),
                calls: broadcast::Sender::new(CALLS_HELD),
            }),
        }
    }

    /// Serves one agent on `io` until either side closes it, or the
    /// session or the server is stopped. It returns when every call of the
    /// session has ended and is in the log.
    pub async fn serve<T>(&self, io: T, origin: Origin)
    where
        T: AsyncRead + AsyncWrite + Send + Unpin + 'static,
    {
        let live = Arc::new(Live {
            id: uuid::Uuid::new_v4().to_string(),
            origin,
            agent: OnceLock::new(),
            connected_at: SystemTime::now(),
            calls: AtomicU64::new(0),
            stop: self.inner.stop.child(),
            in_flight: RwLock::new(()),
            running: AtomicUsize::new(0),
            used: Mutex::default(),
            changed: Arc::clone(&self.inner.changed),
        });
        self.sessions().push(live.clone());
        self.inner.changed.send_replace(());
        let open = Open {
            server: self.clone(),
            live: live.clone(),
        };
        crate::protocol::serve(self.clone(), live.clone(), io).await;
        // Nobody is there to read an answer now.
        live.stop.end(Why::Client);
        let _ = live.in_flight.write().await;
        drop(open);
    }

    /// Told when what [`Server::open_sessions`] says has changed: a
    /// session opened or ended, its agent was named, or it made a call.
    pub fn changes(&self) -> watch::Receiver<()> {
        self.inner.changed.subscribe()
    }

    /// Told of each call as it ends, once it is in the log. A receiver
    /// that falls more than a few hundred calls behind loses the oldest.
    pub fn calls(&self) -> broadcast::Receiver<Call> {
        self.inner.calls.subscribe()
    }

    /// The sessions that are open, the oldest first.
    pub fn open_sessions(&self) -> Vec<SessionInfo> {
        self.sessions().iter().map(|live| live.info()).collect()
    }

    /// Stops one session: each of its calls answers `server_stopped`, and
    /// its transport is closed. Says whether there was such a session.
    pub fn stop_session(&self, id: &str) -> bool {
        let live = self.sessions().iter().find(|live| live.id == id).cloned();
        match live {
            Some(live) => {
                live.stop.end(Why::Stopped);
                true
            }
            None => false,
        }
    }

    /// Stops every session, for good: a server that was stopped serves
    /// nobody again.
    pub fn stop(&self) {
        self.inner.stop.end(Why::Stopped);
    }

    pub(crate) fn version(&self) -> &str {
        &self.inner.version
    }

    fn sessions(&self) -> std::sync::MutexGuard<'_, Vec<Arc<Live>>> {
        self.inner
            .sessions
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    /// Runs one call of `live`'s and writes it to the log. `cancelled` is
    /// the client's own cancel of this call.
    pub(crate) async fn call(
        &self,
        live: &Live,
        name: &str,
        arguments: &Map<String, Json>,
        cancelled: &CancellationToken,
    ) -> Result<Reply, Invalid> {
        let arrived = SystemTime::now();
        let started = Instant::now();
        let _in_flight = live.in_flight.read().await;
        let (_running, before) = Running::count(&live.running);
        // The session works where the call names, from the moment it comes:
        // a call that runs for seconds is there the while.
        let named = arguments
            .get("connection")
            .and_then(Json::as_str)
            .and_then(|name| {
                self.inner
                    .gateway
                    .connections()
                    .into_iter()
                    .find(|listed| listed.name == name)
            })
            .map(|listed| listed.id);
        live.called(arrived, named.as_deref());
        let stop = live.stop.child();
        let refuse = |refused: Refused| {
            known(name).map(|()| Done {
                answer: Err(refused),
                note: tools::Note::default(),
            })
        };
        let done = match live.stop.why() {
            // A session that was stopped is given nothing more.
            Some(why) => refuse(Refused::session_ended(why)),
            None if before >= MAX_CALLS_AT_ONCE => refuse(Refused::too_many_calls()),
            None => {
                let work = tools::call(&self.inner.gateway, &stop, name, arguments);
                tokio::pin!(work);
                tokio::select! {
                    done = &mut work => done,
                    () = cancelled.cancelled() => {
                        stop.end(Why::Client);
                        work.await
                    }
                }
            }
        }?;
        live.calls.fetch_add(1, Ordering::Relaxed);
        let Done { answer, note } = done;
        let named = note.connection.as_ref().map(|(id, _)| id.as_str());
        live.called(arrived, named);
        let reply = match answer {
            // A map of JSON values is always written.
            Ok(json) => Reply::Answer(serde_json::to_string(&json).unwrap_or_default()),
            Err(refused) => Reply::Refused(refused),
        };
        let took = note.took.unwrap_or_else(|| started.elapsed());
        let (result, code, message, bytes) = match &reply {
            Reply::Answer(text) => ("ok", None, None, Some(text.len())),
            Reply::Refused(refused) => (
                refused.result(),
                Some(refused.code),
                Some(refused.for_the_log().to_owned()),
                None,
            ),
        };
        log::info!(
            "agent call: {name} {result}{} in {} ms",
            code.map(|code| format!(" ({code})")).unwrap_or_default(),
            took.as_millis()
        );
        let (connection_id, connection) = note.connection.unzip();
        let agent = live.agent.get().map_or(UNNAMED, String::as_str).to_owned();
        let mut call = Call {
            at: arrived,
            session: live.id.clone(),
            agent: agent.clone(),
            connection: connection_id.clone(),
            tool: name.to_owned(),
            statement: note.sql.is_some(),
            logged: None,
            request: note
                .sql
                .as_deref()
                .or(note.request.as_deref())
                .map(one_line),
            result: result.to_owned(),
            message: message.clone(),
            rows: note.rows,
            columns: note.columns,
            took,
        };
        let entry = Entry {
            at_ms: millis(arrived.duration_since(UNIX_EPOCH).unwrap_or_default()),
            session: live.id.clone(),
            agent,
            project: live.origin.project.clone(),
            transport: live.origin.transport.as_str().to_owned(),
            connection_id,
            connection,
            tool: name.to_owned(),
            sql: note.sql,
            params: note.params,
            request: note.request,
            result: result.to_owned(),
            code: code.map(str::to_owned),
            message,
            ran_as: note.ran_as.map(str::to_owned),
            rows: note.rows.map(count),
            matched: note.matched.map(count),
            columns: note.columns.map(count),
            bytes: bytes.map(|bytes| count(bytes as u64)),
            masked: note.masked,
            took_ms: Some(millis(took)),
        };
        match self.inner.log.record(entry).await {
            Ok(id) => call.logged = Some(id),
            // The call was answered all the same: a full disk must not
            // turn every read into an error.
            Err(error) => log::error!("an agent's call is not in the log: {error}"),
        }
        // Nobody may be listening.
        let _ = self.inner.calls.send(call);
        Ok(reply)
    }
}

/// Whether there is a tool of this name.
fn known(name: &str) -> Result<(), Invalid> {
    match tools::all().iter().any(|tool| tool.name == name) {
        true => Ok(()),
        false => Err(Invalid::UnknownTool),
    }
}

fn millis(time: Duration) -> i64 {
    i64::try_from(time.as_millis()).unwrap_or(i64::MAX)
}

fn count(count: u64) -> i64 {
    i64::try_from(count).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_request_is_told_on_one_line_and_cut() {
        assert_eq!(
            super::one_line("SELECT  id,\n\t title\nFROM books "),
            "SELECT id, title FROM books"
        );
        let long = "x".repeat(super::CALL_CHARS + 10);
        let told = super::one_line(&long);
        assert_eq!(told.chars().count(), super::CALL_CHARS + 1);
        assert!(told.ends_with('…'));
        assert_eq!(
            super::one_line(&"é".repeat(super::CALL_CHARS))
                .chars()
                .count(),
            super::CALL_CHARS
        );
    }
}
