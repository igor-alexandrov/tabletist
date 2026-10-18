//! What the tests share: a gateway over SQLite files of the Bookshop, and
//! an agent that calls the server through the SDK's client.
//!
//! This is the one place of the crate that opens a database by where it
//! is. The crate's own sources never do (`tests/secrets.rs` reads them).

// Each test binary uses its own part of this, and
// `allow-unwrap-in-tests` does not cover helpers in an integration-test
// crate.
#![allow(dead_code, clippy::unwrap_used)]

use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::Duration;

use rmcp::model::{
    CallToolRequestParams, CallToolResult, ClientCapabilities, ClientConfig, Implementation,
};
use rmcp::service::RunningService;
use rmcp::{RoleClient, ServiceExt};
use serde_json::Value as Json;
use tabletist_db::agent::{Answer, ColumnName, Hidden, Limits, Read, Tables};
use tabletist_db::{
    Access, ConnectSpec, Connection, Dialect, Error, HostKeys, ObjectInfo, ObjectRef, Secrets,
    StopFlag, Structure,
};
pub use tabletist_mcp::Server;
use tabletist_mcp::{Entry, Failure, Gateway, Listed, Log, Origin, Schemas, Session, Transport};
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader, ReadBuf};
use tokio::sync::OwnedMutexGuard;
use tokio_util::sync::CancellationToken;

/// The limits of the Bookshop's connections, unless a test gives others.
pub const LIMITS: Limits = Limits {
    rows: 500,
    timeout: Duration::from_secs(10),
};

/// A gateway over SQLite files in a directory of its own. A clone is the
/// same gateway.
#[derive(Clone)]
pub struct Shop {
    inner: Arc<Inner>,
}

struct Inner {
    dir: tempfile::TempDir,
    connections: Mutex<Vec<Kept>>,
    /// Whether the database is given the connection's timeout. Without it
    /// only the server's own timer ends a statement.
    database_times_out: AtomicBool,
    /// The reads that were awaited to their end.
    reads_ended: AtomicUsize,
}

#[derive(Clone)]
struct Kept {
    listed: Listed,
    path: PathBuf,
    /// A connection runs one call at a time.
    turn: Arc<tokio::sync::Mutex<()>>,
}

impl Shop {
    /// A gateway with one connection, "Bookshop · dev", over a new file
    /// that holds the Bookshop.
    pub fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bookshop.db");
        tabletist_db::fixtures::write_sqlite_bookshop(&path).unwrap();
        let shop = Self {
            inner: Arc::new(Inner {
                dir,
                connections: Mutex::default(),
                database_times_out: AtomicBool::new(true),
                reads_ended: AtomicUsize::new(0),
            }),
        };
        shop.list("Bookshop · dev", &path, LIMITS);
        shop
    }

    /// Lets agents use the file at `path` under `name`. Its id is the name
    /// with `id:` before it.
    pub fn list(&self, name: &str, path: &Path, limits: Limits) {
        self.unlist(name);
        self.inner.connections.lock().unwrap().push(Kept {
            listed: Listed {
                id: format!("id:{name}"),
                name: name.to_owned(),
                dialect: Dialect::Sqlite,
                limits,
                tables: Tables::All,
                masked: Vec::new(),
            },
            path: path.to_owned(),
            turn: Arc::default(),
        });
    }

    /// Sets what agents may not read of a listed connection.
    pub fn hide(&self, name: &str, tables: Tables) {
        let mut connections = self.inner.connections.lock().unwrap();
        for kept in connections
            .iter_mut()
            .filter(|kept| kept.listed.name == name)
        {
            kept.listed.tables = tables.clone();
        }
    }

    /// Sets the columns of a listed connection whose values agents get as
    /// `•••`, by their entries as the person writes them.
    pub fn mask(&self, name: &str, masked: &[&str]) {
        let masked: Vec<ColumnName> = masked.iter().map(|entry| entry.parse().unwrap()).collect();
        let mut connections = self.inner.connections.lock().unwrap();
        for kept in connections
            .iter_mut()
            .filter(|kept| kept.listed.name == name)
        {
            kept.listed.masked = masked.clone();
        }
    }

    /// Turns a connection's access Off.
    pub fn unlist(&self, name: &str) {
        self.inner
            .connections
            .lock()
            .unwrap()
            .retain(|kept| kept.listed.name != name);
    }

    /// The Bookshop's file.
    pub fn bookshop(&self) -> PathBuf {
        self.path("bookshop.db")
    }

    /// A path in the gateway's directory.
    pub fn path(&self, file: &str) -> PathBuf {
        self.inner.dir.path().join(file)
    }

    /// Runs `sql` on the Bookshop's file, as its owner would.
    pub fn change(&self, sql: &str) {
        rusqlite::Connection::open(self.bookshop())
            .unwrap()
            .execute_batch(sql)
            .unwrap();
    }

    /// Leaves the statement's time to the server's own timer.
    pub fn without_the_databases_timeout(&self) {
        self.inner.database_times_out.store(false, Ordering::SeqCst);
    }

    pub fn reads_ended(&self) -> usize {
        self.inner.reads_ended.load(Ordering::SeqCst)
    }
}

impl Gateway for Shop {
    type Session = ShopSession;

    fn connections(&self) -> Vec<Listed> {
        let connections = self.inner.connections.lock().unwrap();
        connections.iter().map(|kept| kept.listed.clone()).collect()
    }

    async fn open(&self, id: &str, stop: &CancellationToken) -> Result<ShopSession, Failure> {
        let kept = {
            let connections = self.inner.connections.lock().unwrap();
            connections
                .iter()
                .find(|kept| kept.listed.id == id)
                .cloned()
        };
        let kept = kept.ok_or(Failure::Unavailable)?;
        let turn = tokio::select! {
            turn = tokio::time::timeout(kept.listed.limits.timeout, kept.turn.clone().lock_owned()) => {
                turn.map_err(|_| Failure::Busy)?
            }
            () = stop.cancelled() => return Err(Error::Cancelled.into()),
        };
        let connection = Connection::connect_with(
            &ConnectSpec::sqlite(&kept.path),
            &Secrets::default(),
            &HostKeys::default(),
            Access::ReadOnly,
        )
        .await?;
        Ok(ShopSession {
            connection,
            stop: stop.clone(),
            shop: self.clone(),
            _turn: turn,
        })
    }
}

pub struct ShopSession {
    connection: Connection,
    stop: CancellationToken,
    shop: Shop,
    _turn: OwnedMutexGuard<()>,
}

impl Session for ShopSession {
    async fn schemas(&self) -> Result<Schemas, Failure> {
        Ok(Schemas {
            names: self.connection.list_schemas().await?,
            home: Some("main".into()),
        })
    }

    async fn objects(&self, schema: &str) -> Result<Vec<ObjectInfo>, Failure> {
        Ok(self.connection.list_objects(schema).await?)
    }

    async fn describe(&self, object: &ObjectRef) -> Result<Structure, Failure> {
        Ok(self.connection.describe(object).await?)
    }

    async fn hidden(&self, tables: &Tables, masked: &[ColumnName]) -> Result<Hidden, Failure> {
        Ok(self.connection.agent_hidden(tables, masked).await?)
    }

    async fn read(
        &self,
        read: &Read,
        limits: Limits,
        tables: &Tables,
        masked: &[ColumnName],
    ) -> Result<Answer, Failure> {
        if self.stop.is_cancelled() {
            return Err(Error::Cancelled.into());
        }
        let limits = match self.shop.inner.database_times_out.load(Ordering::SeqCst) {
            true => limits,
            false => Limits {
                timeout: Duration::from_secs(3_600),
                ..limits
            },
        };
        let flag = StopFlag::new();
        let run = self
            .connection
            .run_agent_read(read, limits, tables, masked, &flag);
        tokio::pin!(run);
        let answer = tokio::select! {
            answer = &mut run => answer,
            () = self.stop.cancelled() => {
                flag.stop();
                let _ = self.connection.cancel_handle().cancel().await;
                // Awaited to its end, as the gateway promises.
                run.await
            }
        };
        self.shop.inner.reads_ended.fetch_add(1, Ordering::SeqCst);
        Ok(answer?)
    }
}

/// A server over `shop`, with its log in the shop's directory.
pub fn server(shop: &Shop) -> (Server<Shop>, Log) {
    let log = Log::open(&shop.path("agents.sqlite3")).unwrap().log;
    (Server::new(shop.clone(), log.clone(), "0.0.0-test"), log)
}

/// An agent connected to a server.
pub struct Agent {
    pub client: RunningService<RoleClient, ClientConfig>,
    /// Every byte the server has written to the agent.
    pub received: Arc<Mutex<Vec<u8>>>,
    served: tokio::task::JoinHandle<()>,
}

/// What a tool answered.
#[derive(Debug, Clone, PartialEq)]
pub struct Called {
    /// The result's structured content, which a refusal has.
    pub structured: Json,
    pub text: String,
    pub is_error: bool,
    /// The text as JSON, which an answer is.
    read: Json,
}

impl Called {
    /// The refusal's code, of a call that was refused.
    pub fn refused(&self) -> &str {
        assert!(self.is_error, "not refused: {}", self.text);
        assert_eq!(self.structured["message"], self.text);
        self.structured["refused"].as_str().unwrap()
    }

    /// The answer, of a call that was answered: its text, read as JSON.
    /// An answer is sent once, so it has no structured content beside it.
    pub fn answer(&self) -> &Json {
        assert!(!self.is_error, "refused: {}", self.text);
        assert_eq!(self.structured, Json::Null);
        assert!(self.read.is_object(), "{}", self.text);
        &self.read
    }
}

/// What a client says it is.
pub fn client(name: &str, title: Option<&str>) -> ClientConfig {
    let mut info = Implementation::new(name, "1.0.0");
    info.title = title.map(str::to_owned);
    ClientConfig::new(ClientCapabilities::default(), info)
}

impl Agent {
    /// Connects "Claude Code", started in `tabletist-repo`.
    pub async fn connect(server: &Server<Shop>) -> Self {
        Self::connect_as(server, client("claude-code", Some("Claude Code"))).await
    }

    pub async fn connect_as(server: &Server<Shop>, client: ClientConfig) -> Self {
        let (transport, served, received) = serve(server);
        let client = client.serve(transport).await.unwrap();
        Self {
            client,
            received,
            served,
        }
    }

    pub async fn call(&self, tool: &'static str, arguments: Json) -> Called {
        called(
            self.client
                .call_tool(request(tool, arguments))
                .await
                .unwrap(),
        )
    }

    /// Closes the agent's end, and waits for the server to have let the
    /// session go.
    pub async fn close(self) {
        let Self { client, served, .. } = self;
        let _ = client.cancel().await;
        served.await.unwrap();
    }

    /// Waits for the server to have ended the session by itself.
    pub async fn ended(self) {
        self.served.await.unwrap();
    }

    /// What the server has written so far, as text.
    pub fn received(&self) -> String {
        String::from_utf8_lossy(&self.received.lock().unwrap()).into_owned()
    }
}

/// Starts serving a new session, and gives the agent's end of it.
pub fn serve(
    server: &Server<Shop>,
) -> (
    Tap<tokio::io::DuplexStream>,
    tokio::task::JoinHandle<()>,
    Arc<Mutex<Vec<u8>>>,
) {
    serve_with_room(server, 4 * 1024 * 1024)
}

/// The same, over a transport that holds `room` bytes each way before a
/// write to it waits.
pub fn serve_with_room(
    server: &Server<Shop>,
    room: usize,
) -> (
    Tap<tokio::io::DuplexStream>,
    tokio::task::JoinHandle<()>,
    Arc<Mutex<Vec<u8>>>,
) {
    let (ours, theirs) = tokio::io::duplex(room);
    let served = tokio::spawn({
        let server = server.clone();
        async move {
            let origin = Origin {
                transport: Transport::Stdio,
                project: Some("tabletist-repo".into()),
            };
            server.serve(theirs, origin).await;
        }
    });
    let received = Arc::<Mutex<Vec<u8>>>::default();
    let tap = Tap {
        inner: ours,
        read: received.clone(),
    };
    (tap, served, received)
}

pub fn request(tool: &'static str, arguments: Json) -> CallToolRequestParams {
    let Json::Object(arguments) = arguments else {
        panic!("a tool's arguments are an object");
    };
    CallToolRequestParams::new(tool).with_arguments(arguments)
}

pub fn called(result: CallToolResult) -> Called {
    let text = result
        .content
        .iter()
        .filter_map(|block| block.as_text())
        .map(|text| text.text.clone())
        .collect::<Vec<_>>()
        .join("\n");
    Called {
        structured: result.structured_content.unwrap_or(Json::Null),
        read: serde_json::from_str(&text).unwrap_or(Json::Null),
        text,
        is_error: result.is_error.unwrap_or(false),
    }
}

/// An agent that writes the protocol's lines itself, for what the SDK's
/// client would never send.
pub struct Raw {
    pub write: tokio::io::WriteHalf<Tap<tokio::io::DuplexStream>>,
    pub lines: tokio::io::Lines<BufReader<tokio::io::ReadHalf<Tap<tokio::io::DuplexStream>>>>,
    pub served: tokio::task::JoinHandle<()>,
}

impl Raw {
    /// Connects over a transport that holds `room` bytes each way, and
    /// starts a session by the protocol's handshake.
    pub async fn connect(server: &Server<Shop>, room: usize) -> Self {
        let (transport, served, _received) = serve_with_room(server, room);
        let (read, write) = tokio::io::split(transport);
        let mut raw = Self {
            write,
            lines: BufReader::new(read).lines(),
            served,
        };
        raw.send(serde_json::json!({
            "jsonrpc": "2.0",
            "id": 0,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-11-25",
                "capabilities": {},
                "clientInfo": { "name": "raw", "version": "1" },
            },
        }))
        .await;
        assert_eq!(raw.next().await["id"], serde_json::json!(0));
        raw.send(serde_json::json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }))
            .await;
        raw
    }

    pub async fn send(&mut self, message: Json) {
        self.write
            .write_all(format!("{message}\n").as_bytes())
            .await
            .unwrap();
    }

    /// Sends a call of a tool, numbered `id`.
    pub async fn call(&mut self, id: u64, tool: &str, arguments: Json) {
        self.send(serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "tools/call",
            "params": { "name": tool, "arguments": arguments },
        }))
        .await;
    }

    /// The next message the server sent.
    pub async fn next(&mut self) -> Json {
        let line = self.lines.next_line().await.unwrap().unwrap();
        serde_json::from_str(&line).unwrap()
    }
}

/// The entries of the log, the oldest first.
pub async fn entries(log: &Log) -> Vec<Entry> {
    let mut entries = log.latest(1_000).await.unwrap();
    entries.reverse();
    entries
}

/// A statement over `copies` of the Bookshop's order items that answers
/// one row and takes long to: each copy multiplies the work by forty.
pub fn slow(copies: usize) -> String {
    let tables: Vec<String> = (1..=copies)
        .map(|copy| format!("order_items t{copy}"))
        .collect();
    let weights: Vec<String> = (1..=copies)
        .map(|copy| format!("t{copy}.quantity"))
        .collect();
    format!(
        "SELECT count(*) FROM {} WHERE {} < 0",
        tables.join(", "),
        weights.join(" + ")
    )
}

/// A transport that keeps a copy of what is read from it.
pub struct Tap<T> {
    inner: T,
    read: Arc<Mutex<Vec<u8>>>,
}

impl<T: AsyncRead + Unpin> AsyncRead for Tap<T> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let before = buffer.filled().len();
        let polled = Pin::new(&mut self.inner).poll_read(context, buffer);
        self.read
            .lock()
            .unwrap()
            .extend_from_slice(&buffer.filled()[before..]);
        polled
    }
}

impl<T: AsyncWrite + Unpin> AsyncWrite for Tap<T> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        Pin::new(&mut self.inner).poll_write(context, bytes)
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.inner).poll_flush(context)
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.inner).poll_shutdown(context)
    }
}
