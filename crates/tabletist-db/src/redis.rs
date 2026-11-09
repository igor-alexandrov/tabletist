//! Redis: a standalone server over TCP, with TLS and an SSH tunnel as the
//! SQL servers have them, on the `redis` crate (this module is the only
//! one that names it).
//!
//! Read-only by construction. Every command goes through [`allowed`]
//! before it is written to the socket, and one that is not on [`READS`]
//! is refused there, whatever the connection's access: this version sends
//! no write, and cannot. A guarantee on the server's side needs an ACL
//! user without write permissions.
//!
//! The crate's own handshake is turned off (no `CLIENT SETINFO`, no
//! `AUTH`, no `SELECT` of its own): this module signs in itself, through
//! the same check, so the handshake's words are on the list too.

use std::collections::BTreeSet;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use ::redis::aio::MultiplexedConnection;
use ::redis::{AsyncConnectionConfig, Cmd, Pipeline, RedisConnectionInfo, RedisError, Value};
use tokio::net::TcpStream;

use crate::adapter::Adapter;
use crate::agent::{Catalog, ColumnName, Limits, Read, Sources};
use crate::{
    CancelHandle, ChangeSet, ConnectSpec, Ddl, Error, Lookup, LookupPage, ObjectInfo, ObjectRef,
    RedisTls, Result, RowPage, RowQuery, ScriptMode, ScriptOutcome, Secrets, StopFlag, Structure,
    WriteOutcome,
};

/// Every command Tabletist sends to a Redis server, and nothing else: the
/// handshake's, the reads of the browser, and `CONFIG GET databases`
/// (that one argument only). `MEMORY USAGE`, `OBJECT ENCODING`, `CONFIG
/// GET` and `CLIENT SETNAME` are a command and its subcommand.
pub const READS: &[&str] = &[
    "AUTH",
    "HELLO",
    "PING",
    "INFO",
    "SELECT",
    "DBSIZE",
    "SCAN",
    "HSCAN",
    "SSCAN",
    "TYPE",
    "PTTL",
    "MEMORY USAGE",
    "OBJECT ENCODING",
    "GET",
    "GETRANGE",
    "STRLEN",
    "HGETALL",
    "HLEN",
    "LRANGE",
    "LLEN",
    "SCARD",
    "ZRANGE",
    "ZCARD",
    "XRANGE",
    "XLEN",
    "JSON.GET",
    "JSON.TYPE",
    "CONFIG GET",
    "CLIENT SETNAME",
];

/// The name every connection gives itself, which `CLIENT LIST` shows.
pub const CLIENT_NAME: &str = "tabletist";

/// How long connecting and signing in may take.
pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// How long one command may take once connected.
const RESPONSE_TIMEOUT: Duration = Duration::from_secs(30);

/// The most of a string that is read: a longer one is cut here.
pub const STRING_LIMIT: u64 = 256 * 1024;

/// A hash this long or shorter is read with one `HGETALL`; a longer one
/// with `HSCAN`, a page at a time.
pub const HGETALL_LIMIT: u64 = 1000;

/// The words of [`Error::Connect`] for a server that closed the
/// connection while it was being signed in to: on a plain connection it
/// may want TLS.
pub const CLOSED_IN_HANDSHAKE: &str = "the server closed the connection during the handshake";

/// The words of [`Error::Connect`] for a refused connection.
pub const REFUSED: &str = "connection refused";

/// The entry of [`READS`] that `args` is, or `None` for a command that is
/// not on it: that one is never sent.
pub fn allowed(args: &[&[u8]]) -> Option<&'static str> {
    let word = |at: usize| {
        args.get(at)
            .map(|arg| String::from_utf8_lossy(arg).to_ascii_uppercase())
    };
    let name = word(0)?;
    let two = |sub: &str, len: Option<usize>| {
        let full = format!("{name} {sub}");
        let fits = word(1).as_deref() == Some(sub) && len.is_none_or(|len| args.len() == len);
        fits.then(|| READS.iter().copied().find(|read| *read == full))
            .flatten()
    };
    match name.as_str() {
        "MEMORY" => two("USAGE", None),
        "OBJECT" => two("ENCODING", Some(3)),
        "CLIENT" => two("SETNAME", Some(3)),
        // The number of databases, and no other setting.
        "CONFIG" => two("GET", Some(3)).filter(|_| args[2] == b"databases"),
        _ => READS
            .iter()
            .copied()
            .find(|read| !read.contains(' ') && *read == name),
    }
}

/// The server a connection reached, as `INFO server` names it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Server {
    /// `Redis`, or a server that speaks its protocol: `Valkey`, `KeyDB`,
    /// `Dragonfly`.
    pub name: String,
    pub version: String,
}

impl Server {
    /// Whether the server is another than Redis that speaks its protocol.
    pub fn compatible(&self) -> bool {
        self.name != "Redis"
    }

    /// `Redis 7.4.1`.
    pub fn label(&self) -> String {
        format!("{} {}", self.name, self.version).trim().to_owned()
    }

    /// Reads `INFO server`.
    pub fn of(info: &str) -> Self {
        let field = |name: &str| {
            info.lines().find_map(|line| {
                let (key, value) = line.trim().split_once(':')?;
                (key == name).then(|| value.trim().to_owned())
            })
        };
        let redis = field("redis_version").unwrap_or_default();
        if let Some(version) = field("valkey_version") {
            return Self {
                name: "Valkey".into(),
                version,
            };
        }
        if let Some(version) = field("dragonfly_version") {
            return Self {
                name: "Dragonfly".into(),
                version: version.trim_start_matches("df-v").to_owned(),
            };
        }
        let keydb = field("server_name").is_some_and(|name| name.eq_ignore_ascii_case("keydb"))
            || info.lines().any(|line| line.starts_with("keydb_"));
        Self {
            name: if keydb { "KeyDB" } else { "Redis" }.into(),
            version: redis,
        }
    }
}

/// One database of `INFO keyspace`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Keyspace {
    pub database: u32,
    pub keys: u64,
    pub expires: u64,
}

/// Reads `INFO keyspace`: `db0:keys=1204,expires=31,avg_ttl=0`.
pub fn keyspace(info: &str) -> Vec<Keyspace> {
    info.lines()
        .filter_map(|line| {
            let (db, fields) = line.trim().split_once(':')?;
            let database = db.strip_prefix("db")?.parse().ok()?;
            let field = |name: &str| {
                fields.split(',').find_map(|part| {
                    let (key, value) = part.split_once('=')?;
                    (key == name).then(|| value.parse().ok()).flatten()
                })
            };
            Some(Keyspace {
                database,
                keys: field("keys")?,
                expires: field("expires").unwrap_or(0),
            })
        })
        .collect()
}

/// What a Test, and a window that has just connected, learn of the server.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Probe {
    pub server: Server,
    /// The database the connection is on.
    pub database: u32,
    /// `DBSIZE` of that database.
    pub keys: Option<u64>,
    /// Each database that holds keys.
    pub keyspace: Vec<Keyspace>,
    /// How many databases the server has: `CONFIG GET databases` where the
    /// user may ask, else Redis's 16.
    pub databases: u32,
    /// The command the server refused with `NOPERM`, where it refused one
    /// the browser needs (`INFO` or `SCAN`): keys can't be listed.
    pub refused: Option<String>,
}

impl Probe {
    /// The keys `database` holds, as `INFO keyspace` counts them: none
    /// for one it does not list, which is empty.
    pub fn keys_in(&self, database: u32) -> u64 {
        self.keyspace
            .iter()
            .find(|space| space.database == database)
            .map_or(0, |space| space.keys)
    }
}

/// What `TYPE` says a key is.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum KeyType {
    String,
    Hash,
    List,
    Set,
    ZSet,
    Stream,
    /// RedisJSON's `ReJSON-RL`.
    Json,
    /// A module's type this version does not show, by its name.
    Other(String),
    /// The key is gone (`none`).
    Missing,
}

impl KeyType {
    pub fn of(name: &str) -> Self {
        match name {
            "string" => Self::String,
            "hash" => Self::Hash,
            "list" => Self::List,
            "set" => Self::Set,
            "zset" => Self::ZSet,
            "stream" => Self::Stream,
            "ReJSON-RL" => Self::Json,
            "none" => Self::Missing,
            other => Self::Other(other.to_owned()),
        }
    }

    /// The tag the tree and the key's header show.
    pub fn tag(&self) -> &str {
        match self {
            Self::String => "string",
            Self::Hash => "hash",
            Self::List => "list",
            Self::Set => "set",
            Self::ZSet => "zset",
            Self::Stream => "stream",
            Self::Json => "json",
            Self::Other(name) => name,
            Self::Missing => "none",
        }
    }
}

/// What a key's header shows of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyInfo {
    pub kind: KeyType,
    /// Milliseconds left (`PTTL`); `None` without an expiry.
    pub ttl_ms: Option<i64>,
    /// `MEMORY USAGE`, where the user may ask.
    pub memory: Option<u64>,
    /// `OBJECT ENCODING`, where the user may ask.
    pub encoding: Option<String>,
    /// Its fields, items, members, entries or bytes.
    pub length: Option<u64>,
}

/// Where the next page of a value starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cursor {
    /// The first page.
    Start,
    /// An index (`LRANGE`, `ZRANGE`).
    Offset(u64),
    /// A cursor of `HSCAN` or `SSCAN`.
    Scan(u64),
    /// The next stream entry after this ID.
    After(String),
}

/// A field and its value, or a stream entry's: bytes, as the server has
/// them.
pub type Pair = (Vec<u8>, Vec<u8>);

/// A stream entry: its ID, and its fields in order.
pub type Entry = (String, Vec<Pair>);

/// A page of a value, as it is read.
#[derive(Debug, Clone, PartialEq)]
pub enum Page {
    /// A string, or its first [`STRING_LIMIT`] bytes.
    Text { bytes: Vec<u8>, length: u64 },
    /// Fields of a hash.
    Pairs(Vec<Pair>),
    /// Items of a list, or members of a set.
    Items(Vec<Vec<u8>>),
    /// Members of a sorted set, lowest score first, with their scores as
    /// the server writes them.
    Scored(Vec<(Vec<u8>, String)>),
    /// Entries of a stream, oldest first.
    Entries(Vec<Entry>),
    /// A RedisJSON value, as `JSON.GET` gives it.
    Json(String),
    /// A type this version does not show.
    Unsupported(String),
}

/// A page and where the next one starts; `None` after the last.
#[derive(Debug, Clone, PartialEq)]
pub struct Loaded {
    pub page: Page,
    pub next: Option<Cursor>,
    /// The command that read it, for the line under the value.
    pub command: &'static str,
    pub took: Duration,
}

/// The connections to one server: commands and reads on one, scans on
/// another, so a key opened while a scan runs never waits behind it. A
/// handle is cheap to clone, and every clone sends through the same check.
#[derive(Clone)]
pub struct Handle {
    main: MultiplexedConnection,
    scans: MultiplexedConnection,
    server: Server,
    encrypted: bool,
    /// The database both connections are on.
    database: Arc<AtomicU32>,
    /// The names of the commands sent, from [`READS`]: bounded by it.
    sent: Arc<Mutex<BTreeSet<&'static str>>>,
}

impl std::fmt::Debug for Handle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Handle")
            .field("server", &self.server)
            .finish_non_exhaustive()
    }
}

/// A Redis server's error, worded as the crate's.
fn error(error: RedisError) -> Error {
    if error.is_timeout() {
        return Error::Timeout;
    }
    if error.is_connection_refusal() {
        return Error::Connect(REFUSED.into());
    }
    if let Some(code) = error.code() {
        let code = code.to_owned();
        let detail = error.detail().unwrap_or_default().to_owned();
        let message = format!("{code} {detail}").trim().to_owned();
        return match code.as_str() {
            "WRONGPASS" | "NOAUTH" => Error::Auth(message),
            _ => Error::Query {
                code: Some(code),
                message,
                detail: None,
                hint: None,
                named: Box::default(),
            },
        };
    }
    if error.is_io_error() || error.is_connection_dropped() {
        return Error::ConnectionLost(error.to_string());
    }
    Error::query(error.to_string())
}

/// The server's error code of `error`, where it is the server's.
pub fn code(error: &Error) -> Option<&str> {
    match error {
        Error::Auth(message) => message.split_whitespace().next(),
        Error::Query { code, .. } => code.as_deref(),
        _ => None,
    }
}

/// A reply that is the server's error, as one.
fn reply(value: Value) -> Result<Value> {
    match value {
        Value::ServerError(server) => Err(error(server.into())),
        value => Ok(value),
    }
}

/// The command `args` makes, or the refusal of one not on [`READS`]:
/// nothing is sent for it.
fn command(args: &[&[u8]]) -> Result<(Cmd, &'static str)> {
    let Some(name) = allowed(args) else {
        let name = args
            .first()
            .map(|arg| String::from_utf8_lossy(arg).to_ascii_uppercase())
            .unwrap_or_default();
        return Err(Error::Refused {
            line: 0,
            what: format!("{name}: Tabletist sends only read commands to Redis"),
            mode: ScriptMode::ReadOnly,
        });
    };
    let mut cmd = ::redis::cmd(&String::from_utf8_lossy(args[0]));
    for arg in &args[1..] {
        cmd.arg(*arg);
    }
    Ok((cmd, name))
}

/// Sends `args` on `conn`, once [`allowed`] has let it through.
async fn send(
    conn: &MultiplexedConnection,
    sent: &Mutex<BTreeSet<&'static str>>,
    args: &[&[u8]],
) -> Result<Value> {
    let (cmd, name) = command(args)?;
    if let Ok(mut sent) = sent.lock() {
        sent.insert(name);
    }
    let mut conn = conn.clone();
    reply(conn.send_packed_command(&cmd).await.map_err(error)?)
}

fn text(value: &Value) -> Option<String> {
    match value {
        Value::BulkString(bytes) => Some(String::from_utf8_lossy(bytes).into_owned()),
        Value::SimpleString(text) => Some(text.clone()),
        Value::VerbatimString { text, .. } => Some(text.clone()),
        Value::Okay => Some("OK".into()),
        Value::Int(number) => Some(number.to_string()),
        Value::Double(number) => Some(number.to_string()),
        _ => None,
    }
}

fn bytes(value: Value) -> Vec<u8> {
    match value {
        Value::BulkString(bytes) => bytes,
        Value::SimpleString(text) => text.into_bytes(),
        Value::VerbatimString { text, .. } => text.into_bytes(),
        other => text(&other).unwrap_or_default().into_bytes(),
    }
}

fn int(value: &Value) -> Option<i64> {
    match value {
        Value::Int(number) => Some(*number),
        other => text(other)?.parse().ok(),
    }
}

/// The items of an array, a set or a map's pairs flattened, as RESP2 and
/// RESP3 give a collection.
fn items(value: Value) -> Vec<Value> {
    match value {
        Value::Array(items) | Value::Set(items) => items,
        Value::Map(pairs) => pairs.into_iter().flat_map(|(k, v)| [k, v]).collect(),
        Value::Nil => Vec::new(),
        other => vec![other],
    }
}

/// Pairs of a flat list, `[a, 1, b, 2]`, or of a RESP3 map.
fn pairs(value: Value) -> Vec<(Value, Value)> {
    match value {
        Value::Map(pairs) => pairs,
        other => {
            let mut items = items(other).into_iter();
            let mut pairs = Vec::new();
            while let (Some(a), Some(b)) = (items.next(), items.next()) {
                pairs.push((a, b));
            }
            pairs
        }
    }
}

/// A plain TCP connection to `host:port`, or the reason there is none.
async fn tcp(host: &str, port: u16) -> Result<TcpStream> {
    match TcpStream::connect((host, port)).await {
        Ok(stream) => {
            let _ = stream.set_nodelay(true);
            Ok(stream)
        }
        Err(error) if error.kind() == std::io::ErrorKind::ConnectionRefused => {
            Err(Error::Connect(REFUSED.into()))
        }
        Err(error) => Err(Error::Connect(error.to_string())),
    }
}

/// The TLS configuration of `tls`.
fn tls_config(tls: &RedisTls) -> Result<rustls::ClientConfig> {
    crate::tls::redis_config(
        tls.ca.as_deref(),
        tls.verify_host,
        tls.cert.as_deref().zip(tls.key.as_deref()),
    )
}

/// One connection, signed in: the crate's handshake is empty (no
/// password, database 0, RESP2, no `CLIENT SETINFO`), and [`sign_in`]
/// does the rest through the check.
async fn link(
    spec: &ConnectSpec,
    secrets: &Secrets,
    via: Option<u16>,
    sent: &Mutex<BTreeSet<&'static str>>,
) -> Result<MultiplexedConnection> {
    let redis = spec.redis_spec();
    let (host, port) = match via {
        Some(port) => ("127.0.0.1", port),
        None => (spec.host.as_str(), spec.port),
    };
    let stream = tcp(host, port).await?;
    let info = RedisConnectionInfo::default().set_skip_set_lib_name();
    let config = AsyncConnectionConfig::new().set_response_timeout(Some(RESPONSE_TIMEOUT));
    let (conn, driver) = if redis.tls.enabled {
        let config_tls = tls_config(&redis.tls)?;
        let name = rustls::pki_types::ServerName::try_from(spec.host.clone())
            .map_err(|error| Error::Tls(format!("{}: {error}", spec.host)))?;
        let tls = tokio_rustls::TlsConnector::from(Arc::new(config_tls))
            .connect(name, stream)
            .await
            .map_err(|error| Error::Tls(error.to_string()))?;
        let (conn, driver) = MultiplexedConnection::new_with_config(&info, tls, config)
            .await
            .map_err(error)?;
        (conn, futures_util::FutureExt::boxed(driver))
    } else {
        let (conn, driver) = MultiplexedConnection::new_with_config(&info, stream, config)
            .await
            .map_err(error)?;
        (conn, futures_util::FutureExt::boxed(driver))
    };
    tokio::spawn(driver);
    match sign_in(&conn, spec, secrets, redis.database, sent).await {
        // A plain connection the server drops, or answers with what is
        // no reply (a TLS alert), as it is signed in to: it wants TLS.
        Err(Error::ConnectionLost(_) | Error::Query { code: None, .. }) if !redis.tls.enabled => {
            Err(Error::Connect(CLOSED_IN_HANDSHAKE.into()))
        }
        Err(Error::ConnectionLost(message)) => Err(Error::Connect(message)),
        Err(error) => Err(error),
        Ok(()) => Ok(conn),
    }
}

/// `HELLO 3` with the user and password where it is known (Redis 6 and
/// later), else `AUTH`; then the connection's name, its database, and a
/// `PING`.
async fn sign_in(
    conn: &MultiplexedConnection,
    spec: &ConnectSpec,
    secrets: &Secrets,
    database: u32,
    sent: &Mutex<BTreeSet<&'static str>>,
) -> Result<()> {
    let password = secrets.password.as_deref().filter(|p| !p.is_empty());
    let user = match spec.user.trim() {
        "" => "default",
        user => user,
    };
    let mut hello: Vec<&[u8]> = vec![b"HELLO", b"3"];
    if let Some(password) = password {
        hello.extend([b"AUTH".as_slice(), user.as_bytes(), password.as_bytes()]);
    }
    match send(conn, sent, &hello).await {
        Ok(_) => {}
        // No HELLO (before Redis 6), or no RESP3: RESP2, with AUTH.
        Err(Error::Query {
            code: Some(code), ..
        }) if code == "ERR" || code == "NOPROTO" => {
            if let Some(password) = password {
                let mut auth: Vec<&[u8]> = vec![b"AUTH"];
                if !spec.user.trim().is_empty() {
                    auth.push(user.as_bytes());
                }
                auth.push(password.as_bytes());
                send(conn, sent, &auth).await?;
            }
        }
        Err(error) => return Err(error),
    }
    // A user whose ACL does not allow it is still a user: its name is
    // only what `CLIENT LIST` shows.
    match send(conn, sent, &[b"CLIENT", b"SETNAME", CLIENT_NAME.as_bytes()]).await {
        Ok(_) | Err(Error::Query { .. }) => {}
        Err(error) => return Err(error),
    }
    if database != 0 {
        send(conn, sent, &[b"SELECT", database.to_string().as_bytes()]).await?;
    }
    send(conn, sent, &[b"PING"]).await?;
    Ok(())
}

/// Connects the two connections of a handle, in at most
/// [`CONNECT_TIMEOUT`]. `via` is the local port of an SSH tunnel.
pub(crate) async fn connect(
    spec: &ConnectSpec,
    secrets: &Secrets,
    via: Option<u16>,
) -> Result<Handle> {
    let sent = Arc::new(Mutex::new(BTreeSet::new()));
    let connecting = async {
        let main = link(spec, secrets, via, &sent).await?;
        let scans = link(spec, secrets, via, &sent).await?;
        let info = send(&main, &sent, &[b"INFO", b"server"]).await;
        let server = match info {
            Ok(info) => Server::of(&text(&info).unwrap_or_default()),
            // NOPERM: a server the user may not ask about is still one.
            Err(Error::Query { .. }) => Server {
                name: "Redis".into(),
                version: String::new(),
            },
            Err(error) => return Err(error),
        };
        Ok(Handle {
            main,
            scans,
            server,
            encrypted: spec.redis_spec().tls.enabled,
            database: Arc::new(AtomicU32::new(spec.redis_spec().database)),
            sent: Arc::clone(&sent),
        })
    };
    tokio::time::timeout(CONNECT_TIMEOUT, connecting)
        .await
        .map_err(|_| Error::Timeout)?
}

impl Handle {
    async fn send(&self, args: &[&[u8]]) -> Result<Value> {
        send(&self.main, &self.sent, args).await
    }

    /// Sends each of `commands` in one round trip, each checked first: a
    /// refused one refuses them all, and nothing is sent.
    async fn pipeline(&self, commands: &[Vec<&[u8]>]) -> Result<Vec<Result<Value>>> {
        let mut pipe = Pipeline::new();
        let mut names = Vec::new();
        for args in commands {
            let (cmd, name) = command(args)?;
            names.push(name);
            pipe.add_command(cmd);
        }
        if let Ok(mut sent) = self.sent.lock() {
            sent.extend(names);
        }
        if commands.is_empty() {
            return Ok(Vec::new());
        }
        let mut conn = self.main.clone();
        let replies = conn
            .send_packed_commands(&pipe, 0, commands.len())
            .await
            .map_err(error)?;
        Ok(replies.into_iter().map(reply).collect())
    }

    pub fn server(&self) -> &Server {
        &self.server
    }

    pub fn is_encrypted(&self) -> bool {
        self.encrypted
    }

    /// The names of the commands this handle has sent, each once.
    pub fn sent(&self) -> Vec<&'static str> {
        self.sent
            .lock()
            .map(|sent| sent.iter().copied().collect())
            .unwrap_or_default()
    }

    /// Sends a command of the caller's: only one on [`READS`] is sent.
    /// For tests of the check, which see a refused one never reach the
    /// server.
    #[doc(hidden)]
    pub async fn run(&self, args: &[&str]) -> Result<Option<String>> {
        let args: Vec<&[u8]> = args.iter().map(|arg| arg.as_bytes()).collect();
        Ok(text(&self.send(&args).await?))
    }

    /// What a Test and a window that has just connected show: the
    /// server, its databases and their keys. A `NOPERM` on `INFO` or
    /// `SCAN` is said in [`Probe::refused`], not as an error.
    pub async fn probe(&self, database: u32) -> Result<Probe> {
        let mut probe = Probe {
            server: self.server.clone(),
            database,
            databases: 16,
            ..Probe::default()
        };
        let noperm = |error: &Error| code(error) == Some("NOPERM");
        match self.send(&[b"INFO", b"keyspace"]).await {
            Ok(info) => probe.keyspace = keyspace(&text(&info).unwrap_or_default()),
            Err(error) if noperm(&error) => probe.refused = Some("INFO".into()),
            Err(error) => return Err(error),
        }
        match self.send(&[b"SCAN", b"0", b"COUNT", b"1"]).await {
            Ok(_) => {}
            Err(error) if noperm(&error) => {
                probe.refused.get_or_insert_with(|| "SCAN".into());
            }
            Err(error) => return Err(error),
        }
        match self.send(&[b"DBSIZE"]).await {
            Ok(size) => probe.keys = int(&size).and_then(|n| u64::try_from(n).ok()),
            Err(Error::Query { .. }) => {}
            Err(error) => return Err(error),
        }
        // Allowed to few users, and asked only where it can say more.
        if let Ok(reply) = self.send(&[b"CONFIG", b"GET", b"databases"]).await {
            let count = pairs(reply)
                .into_iter()
                .find_map(|(_, value)| int(&value))
                .and_then(|n| u32::try_from(n).ok());
            if let Some(count) = count.filter(|count| *count > 0) {
                probe.databases = count;
            }
        }
        Ok(probe)
    }

    /// Moves both connections to `database`.
    /// Moves both connections to `database`, or neither: the main one
    /// goes back to the database it was on where the scans' cannot follow.
    pub async fn select(&self, database: u32) -> Result<()> {
        let number = database.to_string();
        let was = self.database.load(Ordering::SeqCst).to_string();
        self.send(&[b"SELECT", number.as_bytes()]).await?;
        if let Err(error) = send(&self.scans, &self.sent, &[b"SELECT", number.as_bytes()]).await {
            let _ = self.send(&[b"SELECT", was.as_bytes()]).await;
            return Err(error);
        }
        self.database.store(database, Ordering::SeqCst);
        Ok(())
    }

    /// The keys of the database, at once: `DBSIZE`.
    pub async fn dbsize(&self) -> Result<u64> {
        let size = send(&self.scans, &self.sent, &[b"DBSIZE"]).await?;
        Ok(int(&size).and_then(|n| u64::try_from(n).ok()).unwrap_or(0))
    }

    /// One step of `SCAN`, on the scans' connection: the next cursor (0
    /// when the walk is done) and the names it returned.
    pub async fn scan(
        &self,
        cursor: u64,
        pattern: &str,
        count: u32,
    ) -> Result<(u64, Vec<Vec<u8>>)> {
        let cursor = cursor.to_string();
        let count = count.to_string();
        let mut args: Vec<&[u8]> = vec![b"SCAN", cursor.as_bytes()];
        if !pattern.is_empty() && pattern != "*" {
            args.extend([b"MATCH".as_slice(), pattern.as_bytes()]);
        }
        args.extend([b"COUNT".as_slice(), count.as_bytes()]);
        let reply = send(&self.scans, &self.sent, &args).await?;
        let mut parts = items(reply).into_iter();
        let next = parts
            .next()
            .and_then(|cursor| int(&cursor))
            .and_then(|n| u64::try_from(n).ok())
            .unwrap_or(0);
        let names = parts.next().map(items).unwrap_or_default();
        Ok((next, names.into_iter().map(bytes).collect()))
    }

    /// The types of `keys`, in one round trip.
    pub async fn types(&self, keys: &[Vec<u8>]) -> Result<Vec<KeyType>> {
        let commands: Vec<Vec<&[u8]>> = keys
            .iter()
            .map(|key| vec![b"TYPE".as_slice(), key.as_slice()])
            .collect();
        let replies = self.pipeline(&commands).await?;
        replies
            .into_iter()
            .map(|reply| Ok(KeyType::of(&text(&reply?).unwrap_or_default())))
            .collect()
    }

    /// The header of `key`: its type, expiry, size, encoding and length.
    /// The size and the encoding are left out where the user may not ask.
    pub async fn describe(&self, key: &[u8]) -> Result<KeyInfo> {
        let replies = self
            .pipeline(&[vec![b"TYPE", key], vec![b"PTTL", key]])
            .await?;
        let mut replies = replies.into_iter();
        let kind = KeyType::of(
            &replies
                .next()
                .transpose()?
                .and_then(|reply| text(&reply))
                .unwrap_or_default(),
        );
        let ttl = replies.next().transpose()?.and_then(|reply| int(&reply));
        let memory = self
            .send(&[b"MEMORY", b"USAGE", key])
            .await
            .ok()
            .and_then(|reply| int(&reply))
            .and_then(|n| u64::try_from(n).ok());
        let encoding = self
            .send(&[b"OBJECT", b"ENCODING", key])
            .await
            .ok()
            .and_then(|reply| text(&reply));
        let length_of: Option<&[u8]> = match kind {
            KeyType::String => Some(b"STRLEN"),
            KeyType::Hash => Some(b"HLEN"),
            KeyType::List => Some(b"LLEN"),
            KeyType::Set => Some(b"SCARD"),
            KeyType::ZSet => Some(b"ZCARD"),
            KeyType::Stream => Some(b"XLEN"),
            KeyType::Json | KeyType::Other(_) | KeyType::Missing => None,
        };
        let length = match length_of {
            Some(name) => int(&self.send(&[name, key]).await?).and_then(|n| u64::try_from(n).ok()),
            None => None,
        };
        Ok(KeyInfo {
            kind,
            ttl_ms: ttl.filter(|ttl| *ttl >= 0),
            memory,
            encoding,
            length,
        })
    }

    /// A page of `key`'s value, `count` items from `from`. `length` is
    /// the header's, which chooses `HGETALL` over `HSCAN` for a short
    /// hash, and `GETRANGE` over `GET` for a long string.
    pub async fn read(
        &self,
        key: &[u8],
        kind: &KeyType,
        length: Option<u64>,
        from: &Cursor,
        count: u32,
    ) -> Result<Loaded> {
        let started = Instant::now();
        let count = u64::from(count.max(1));
        let offset = match from {
            Cursor::Offset(offset) => *offset,
            Cursor::Start | Cursor::Scan(_) | Cursor::After(_) => 0,
        };
        let scan_cursor = match from {
            Cursor::Scan(cursor) => *cursor,
            Cursor::Start | Cursor::Offset(_) | Cursor::After(_) => 0,
        };
        let (page, next, command) = match kind {
            KeyType::String => {
                let length = length.unwrap_or(0);
                let (reply, command) = if length > STRING_LIMIT {
                    let end = (STRING_LIMIT - 1).to_string();
                    (
                        self.send(&[b"GETRANGE", key, b"0", end.as_bytes()]).await?,
                        "GETRANGE",
                    )
                } else {
                    (self.send(&[b"GET", key]).await?, "GET")
                };
                let bytes = bytes(reply);
                let length = length.max(bytes.len() as u64);
                (Page::Text { bytes, length }, None, command)
            }
            KeyType::Hash if length.unwrap_or(0) <= HGETALL_LIMIT && *from == Cursor::Start => {
                let reply = self.send(&[b"HGETALL", key]).await?;
                let rows = pairs(reply)
                    .into_iter()
                    .map(|(field, value)| (bytes(field), bytes(value)))
                    .collect();
                (Page::Pairs(rows), None, "HGETALL")
            }
            KeyType::Hash | KeyType::Set => {
                let name: &[u8] = match kind {
                    KeyType::Hash => b"HSCAN",
                    _ => b"SSCAN",
                };
                let cursor = scan_cursor.to_string();
                let count = count.to_string();
                let reply = self
                    .send(&[name, key, cursor.as_bytes(), b"COUNT", count.as_bytes()])
                    .await?;
                let mut parts = items(reply).into_iter();
                let next = parts
                    .next()
                    .and_then(|cursor| int(&cursor))
                    .and_then(|n| u64::try_from(n).ok())
                    .unwrap_or(0);
                let found = parts.next().unwrap_or(Value::Nil);
                let page = match kind {
                    KeyType::Hash => Page::Pairs(
                        pairs(found)
                            .into_iter()
                            .map(|(field, value)| (bytes(field), bytes(value)))
                            .collect(),
                    ),
                    _ => Page::Items(items(found).into_iter().map(bytes).collect()),
                };
                let command = match kind {
                    KeyType::Hash => "HSCAN",
                    _ => "SSCAN",
                };
                (page, (next != 0).then_some(Cursor::Scan(next)), command)
            }
            KeyType::List => {
                let start = offset.to_string();
                let stop = (offset + count - 1).to_string();
                let reply = self
                    .send(&[b"LRANGE", key, start.as_bytes(), stop.as_bytes()])
                    .await?;
                let rows: Vec<Vec<u8>> = items(reply).into_iter().map(bytes).collect();
                let end = offset + rows.len() as u64;
                let more = rows.len() as u64 == count && length.is_none_or(|length| end < length);
                (
                    Page::Items(rows),
                    more.then_some(Cursor::Offset(end)),
                    "LRANGE",
                )
            }
            KeyType::ZSet => {
                let start = offset.to_string();
                let stop = (offset + count - 1).to_string();
                let reply = self
                    .send(&[
                        b"ZRANGE",
                        key,
                        start.as_bytes(),
                        stop.as_bytes(),
                        b"WITHSCORES",
                    ])
                    .await?;
                // RESP3 gives [member, score] pairs; RESP2 a flat list.
                let items = items(reply);
                let nested = items
                    .first()
                    .is_some_and(|item| matches!(item, Value::Array(_)));
                let rows: Vec<(Vec<u8>, String)> = if nested {
                    items
                        .into_iter()
                        .filter_map(|pair| {
                            let mut pair = self::items(pair).into_iter();
                            let member = bytes(pair.next()?);
                            let score = text(&pair.next()?)?;
                            Some((member, score))
                        })
                        .collect()
                } else {
                    pairs(Value::Array(items))
                        .into_iter()
                        .map(|(member, score)| (bytes(member), text(&score).unwrap_or_default()))
                        .collect()
                };
                let end = offset + rows.len() as u64;
                let more = rows.len() as u64 == count && length.is_none_or(|length| end < length);
                (
                    Page::Scored(rows),
                    more.then_some(Cursor::Offset(end)),
                    "ZRANGE",
                )
            }
            KeyType::Stream => {
                let start = match from {
                    Cursor::After(id) => next_id(id),
                    Cursor::Start | Cursor::Offset(_) | Cursor::Scan(_) => "-".into(),
                };
                let count = count.to_string();
                let reply = self
                    .send(&[
                        b"XRANGE",
                        key,
                        start.as_bytes(),
                        b"+",
                        b"COUNT",
                        count.as_bytes(),
                    ])
                    .await?;
                let rows: Vec<Entry> = items(reply)
                    .into_iter()
                    .filter_map(|entry| {
                        let mut entry = items(entry).into_iter();
                        let id = text(&entry.next()?)?;
                        let fields = pairs(entry.next().unwrap_or(Value::Nil))
                            .into_iter()
                            .map(|(field, value)| (bytes(field), bytes(value)))
                            .collect();
                        Some((id, fields))
                    })
                    .collect();
                let full = rows.len().to_string() == count;
                let next = rows
                    .last()
                    .filter(|_| full)
                    .map(|(id, _)| Cursor::After(id.clone()));
                (Page::Entries(rows), next, "XRANGE")
            }
            KeyType::Json => {
                let reply = self.send(&[b"JSON.GET", key]).await?;
                (
                    Page::Json(text(&reply).unwrap_or_default()),
                    None,
                    "JSON.GET",
                )
            }
            KeyType::Other(name) => (Page::Unsupported(name.clone()), None, "TYPE"),
            KeyType::Missing => (Page::Unsupported("none".into()), None, "TYPE"),
        };
        Ok(Loaded {
            page,
            next,
            command,
            took: started.elapsed(),
        })
    }
}

/// The smallest stream ID after `id` (`1-2` → `1-3`), for the next page
/// of `XRANGE`.
fn next_id(id: &str) -> String {
    match id.split_once('-') {
        Some((ms, seq)) => match seq.parse::<u64>() {
            Ok(seq) if seq < u64::MAX => format!("{ms}-{}", seq + 1),
            _ => match ms.parse::<u64>() {
                Ok(ms) => format!("{}-0", ms + 1),
                Err(_) => id.to_owned(),
            },
        },
        None => format!("{id}-1"),
    }
}

/// A Redis session as the crate holds it. Everything SQL is refused.
pub(crate) struct Conn {
    pub(crate) handle: Handle,
}

const NO_SQL: &str = "Redis has no SQL";

impl Adapter for Conn {
    fn is_encrypted(&self) -> bool {
        self.handle.is_encrypted()
    }

    fn cancel_handle(&self) -> CancelHandle {
        CancelHandle::nothing()
    }

    async fn server_version(&self) -> Result<String> {
        Ok(self.handle.server.label())
    }

    async fn session_zone(&self) -> Result<Option<String>> {
        Ok(None)
    }

    async fn list_databases(&self) -> Result<Vec<String>> {
        Ok(Vec::new())
    }

    async fn list_schemas(&self) -> Result<Vec<String>> {
        Err(Error::Unsupported(NO_SQL))
    }

    async fn list_objects(&self, _schema: &str) -> Result<Vec<ObjectInfo>> {
        Err(Error::Unsupported(NO_SQL))
    }

    async fn describe(&self, _object: &ObjectRef) -> Result<Structure> {
        Err(Error::Unsupported(NO_SQL))
    }

    async fn ddl(&self, _object: &ObjectRef) -> Result<Ddl> {
        Err(Error::Unsupported(NO_SQL))
    }

    async fn fetch_rows(&self, _query: &RowQuery) -> Result<RowPage> {
        Err(Error::Unsupported(NO_SQL))
    }

    async fn count_rows(&self, _query: &RowQuery) -> Result<u64> {
        Err(Error::Unsupported(NO_SQL))
    }

    async fn export_statement(&self, _query: &RowQuery, _columns: &[String]) -> Result<String> {
        Err(Error::Unsupported(NO_SQL))
    }

    async fn stream_rows(
        &self,
        _query: &RowQuery,
        _columns: &[String],
        _batch: usize,
        _to: &tokio::sync::mpsc::Sender<crate::Streamed>,
        _stop: &StopFlag,
    ) -> Result<u64> {
        Err(Error::Unsupported(NO_SQL))
    }

    async fn stream_statement(
        &self,
        _text: &str,
        _batch: usize,
        _to: &tokio::sync::mpsc::Sender<crate::Streamed>,
        _stop: &StopFlag,
    ) -> Result<u64> {
        Err(Error::Unsupported(NO_SQL))
    }

    async fn lookup(&self, _lookup: &Lookup) -> Result<LookupPage> {
        Err(Error::Unsupported(NO_SQL))
    }

    async fn run_script(
        &self,
        _texts: Vec<String>,
        _limit: u32,
        _mode: ScriptMode,
        _stop: &StopFlag,
    ) -> Result<ScriptOutcome> {
        Err(Error::Unsupported(NO_SQL))
    }

    async fn run_read(
        &self,
        _read: &Read,
        _limits: Limits,
        _sources: &Sources,
        _stop: &StopFlag,
    ) -> Result<(ScriptOutcome, Vec<bool>)> {
        Err(Error::Unsupported(NO_SQL))
    }

    async fn agents_catalog(&self, _masked: &[ColumnName]) -> Result<Catalog> {
        Err(Error::Unsupported(NO_SQL))
    }

    async fn write(&self, _changes: &ChangeSet, _stop: &StopFlag) -> Result<WriteOutcome> {
        Err(Error::Unsupported(
            "Redis values are read-only in this version",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_read_list_lets_reads_through_and_nothing_else() {
        for args in [
            &[b"GET".as_slice(), b"k"][..],
            &[b"get", b"k"],
            &[b"HGETALL", b"k"],
            &[b"MEMORY", b"USAGE", b"k"],
            &[b"memory", b"usage", b"k", b"SAMPLES", b"5"],
            &[b"OBJECT", b"ENCODING", b"k"],
            &[b"CONFIG", b"GET", b"databases"],
            &[b"CLIENT", b"SETNAME", b"tabletist"],
            &[b"JSON.GET", b"k"],
        ] {
            assert!(allowed(args).is_some(), "{args:?} should be allowed");
        }
        for args in [
            &[b"SET".as_slice(), b"k", b"v"][..],
            &[b"DEL", b"k"],
            &[b"FLUSHALL"],
            &[b"KEYS", b"*"],
            &[b"EVAL", b"return 1", b"0"],
            &[b"CONFIG", b"SET", b"databases", b"1"],
            &[b"CONFIG", b"GET", b"requirepass"],
            &[b"CONFIG", b"GET", b"*"],
            &[b"CLIENT", b"KILL", b"ID", b"1"],
            &[b"CLIENT", b"SETNAME"],
            &[b"OBJECT", b"FREQ", b"k"],
            &[b"MEMORY", b"PURGE"],
            &[b"JSON.SET", b"k", b"$", b"1"],
            &[b"GETDEL", b"k"],
            &[b"GETEX", b"k"],
            &[],
        ] {
            assert_eq!(allowed(args), None, "{args:?} should be refused");
            assert!(matches!(command(args), Err(Error::Refused { .. })));
        }
    }

    #[test]
    fn the_list_is_the_documented_one() {
        assert_eq!(READS.len(), 29);
        assert!(!READS.contains(&"KEYS"));
    }

    #[test]
    fn info_server_names_redis_and_compatible_servers() {
        let redis = Server::of("# Server\r\nredis_version:7.4.1\r\nredis_mode:standalone\r\n");
        assert_eq!(redis.label(), "Redis 7.4.1");
        assert!(!redis.compatible());
        let valkey =
            Server::of("redis_version:7.2.4\r\nserver_name:valkey\r\nvalkey_version:8.0.1\r\n");
        assert_eq!(valkey.label(), "Valkey 8.0.1");
        assert!(valkey.compatible());
        let dragonfly = Server::of("redis_version:6.2.11\r\ndragonfly_version:df-v1.21.2\r\n");
        assert_eq!(dragonfly.label(), "Dragonfly 1.21.2");
        let keydb = Server::of("redis_version:6.3.4\r\nserver_name:KeyDB\r\n");
        assert_eq!(keydb.label(), "KeyDB 6.3.4");
    }

    #[test]
    fn info_keyspace_counts_each_database() {
        let spaces = keyspace(
            "# Keyspace\r\ndb0:keys=1204,expires=31,avg_ttl=0\r\ndb2:keys=38,expires=0\r\n",
        );
        assert_eq!(
            spaces,
            vec![
                Keyspace {
                    database: 0,
                    keys: 1204,
                    expires: 31
                },
                Keyspace {
                    database: 2,
                    keys: 38,
                    expires: 0
                },
            ]
        );
        let probe = Probe {
            keyspace: spaces,
            ..Probe::default()
        };
        assert_eq!(probe.keys_in(2), 38);
        assert_eq!(probe.keys_in(1), 0);
    }

    #[test]
    fn stream_pages_start_after_the_last_id() {
        assert_eq!(next_id("1700000000000-0"), "1700000000000-1");
        assert_eq!(next_id("5-18446744073709551615"), "6-0");
    }

    #[test]
    fn errors_keep_the_servers_code() {
        assert_eq!(
            code(&Error::Auth("WRONGPASS invalid".into())),
            Some("WRONGPASS")
        );
        assert_eq!(
            code(&Error::Query {
                code: Some("NOPERM".into()),
                message: "NOPERM".into(),
                detail: None,
                hint: None,
                named: Box::default(),
            }),
            Some("NOPERM")
        );
        assert_eq!(code(&Error::Timeout), None);
    }
}
