//! The log of calls: one entry for each call a tool answered or refused,
//! in a SQLite file of the person's own. It holds what was asked and what
//! came of it, and never a row's value.
//!
//! The file is opened once and written from the runtime's blocking pool,
//! by one task at a time, which writes together the entries that wait.

use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use rusqlite::{
    Connection, ErrorCode, OpenFlags, OptionalExtension, Row, Transaction, TransactionBehavior,
    params,
};
use tokio::sync::oneshot;

/// The schema's version, in `PRAGMA user_version`.
const VERSION: i64 = 1;

/// How long a write waits for another connection's lock on the file. This
/// process is the log's one writer, bar a second Tabletist that is open at
/// the same time, and an entry is written in an instant.
const LOCK_WAIT: Duration = Duration::from_secs(2);

/// Every column the plan gives the log. Those a write's approval fills
/// (`reason`, `note`, `decided_ms`) and the masked columns are here from
/// the start, so that the file is not changed for them.
const SCHEMA: &str = "
CREATE TABLE calls (
  id            INTEGER PRIMARY KEY,
  at_ms         INTEGER NOT NULL,
  session       TEXT    NOT NULL,
  agent         TEXT    NOT NULL,
  project       TEXT,
  transport     TEXT    NOT NULL,
  connection_id TEXT,
  connection    TEXT,
  tool          TEXT    NOT NULL,
  sql           TEXT,
  params        TEXT,
  reason        TEXT,
  request       TEXT,
  result        TEXT    NOT NULL,
  code          TEXT,
  message       TEXT,
  note          TEXT,
  ran_as        TEXT,
  rows          INTEGER,
  matched       INTEGER,
  columns       INTEGER,
  bytes         INTEGER,
  masked        TEXT,
  took_ms       INTEGER,
  decided_ms    INTEGER
);
CREATE INDEX calls_at         ON calls (at_ms);
CREATE INDEX calls_agent      ON calls (agent, at_ms);
CREATE INDEX calls_connection ON calls (connection_id, at_ms);
CREATE INDEX calls_result     ON calls (result, at_ms);
";

/// One call.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Entry {
    /// When the call arrived, in milliseconds of Unix time.
    pub at_ms: i64,
    pub session: String,
    /// What the client calls itself. A label, not an identity.
    pub agent: String,
    pub project: Option<String>,
    pub transport: String,
    /// `None` for a call that named no connection, or none an agent may
    /// see.
    pub connection_id: Option<String>,
    /// The connection's name as it was then.
    pub connection: Option<String>,
    pub tool: String,
    pub sql: Option<String>,
    /// The statement's values, as JSON: the list as it came, or, where
    /// that is more than 100 kB, `{"cut": its start, "bytes": its length}`.
    pub params: Option<String>,
    /// The tables `describe_schema` was asked for.
    pub request: Option<String>,
    /// `ok`, `refused`, `error`, `timeout`, `cancelled` or `stopped`.
    pub result: String,
    pub code: Option<String>,
    /// The refusal's sentence. For a database's own error, its code and
    /// not its words, which can quote a row.
    pub message: Option<String>,
    pub ran_as: Option<String>,
    /// The rows the agent was sent.
    pub rows: Option<i64>,
    /// The statement's rows, where that is known.
    pub matched: Option<i64>,
    pub columns: Option<i64>,
    /// The answer's size as JSON.
    pub bytes: Option<i64>,
    /// The answer's masked columns, by name, as a JSON list: the call
    /// answered `•••` for each of their values.
    pub masked: Option<String>,
    pub took_ms: Option<i64>,
}

/// Why the log could not be read or written. Its words name the file and
/// SQLite's own error, never a call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogError(String);

impl fmt::Display for LogError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for LogError {}

impl From<rusqlite::Error> for LogError {
    fn from(error: rusqlite::Error) -> Self {
        Self(error.to_string())
    }
}

/// A call as the log has it, with its place there: what the next page is
/// read from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Logged {
    pub id: i64,
    pub entry: Entry,
}

/// A page of the log, the newest first.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Page {
    pub calls: Vec<Logged>,
    /// How many calls the log holds of what was asked: those that pass
    /// the filter.
    pub total: u64,
    /// Whether there are older ones.
    pub more: bool,
    /// The agents that called, on the filter's connection or on every one,
    /// whatever else it asks: what the agent filter offers. Read with a
    /// first page alone; empty with a later one.
    pub agents: Vec<String>,
}

/// Which calls a page of the log holds: those on one connection or on
/// every one, of one agent or any, that ended one way or any, and whose
/// request holds a text or not.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LogFilter {
    /// The connection's id.
    pub connection: Option<String>,
    /// The agent, as it names itself.
    pub agent: Option<String>,
    /// How the call ended, as [`Entry::result`] says it.
    pub result: Option<String>,
    /// Text the request holds: the statement, or the tables
    /// `describe_schema` was asked for. English letters match in either
    /// case.
    pub search: Option<String>,
}

impl LogFilter {
    /// The filter of every call on `connection`, or on every connection.
    pub fn on(connection: Option<String>) -> Self {
        Self {
            connection,
            ..Self::default()
        }
    }

    /// The conditions a call passes, and the values they are bound to, in
    /// order. One for each part asked: a part that may be nothing, bound
    /// as a value, would plan the statement without its index. With a
    /// connection, the agent and the result are written so that no index
    /// of theirs is taken (a `+` before the column): one connection's
    /// calls are read through its own, not every connection's that ended
    /// one way.
    fn conditions(&self) -> (Vec<&'static str>, Vec<String>) {
        let mut conditions = Vec::new();
        let mut values = Vec::new();
        let one = self.connection.is_some();
        if let Some(connection) = &self.connection {
            conditions.push("connection_id = ?");
            values.push(connection.clone());
        }
        if let Some(agent) = &self.agent {
            conditions.push(if one { "+agent = ?" } else { "agent = ?" });
            values.push(agent.clone());
        }
        if let Some(result) = &self.result {
            conditions.push(if one { "+result = ?" } else { "result = ?" });
            values.push(result.clone());
        }
        if let Some(search) = self.search.as_deref().filter(|search| !search.is_empty()) {
            // The text as it is: `%`, `_` and the escape match themselves.
            let escaped = search
                .replace('\\', "\\\\")
                .replace('%', "\\%")
                .replace('_', "\\_");
            let pattern = format!("%{escaped}%");
            conditions.push("(sql LIKE ? ESCAPE '\\' OR request LIKE ? ESCAPE '\\')");
            values.push(pattern.clone());
            values.push(pattern);
        }
        (conditions, values)
    }

    /// `WHERE` and the conditions, or nothing for every call.
    fn clause(conditions: &[&str]) -> String {
        match conditions.is_empty() {
            true => String::new(),
            false => format!(" WHERE {}", conditions.join(" AND ")),
        }
    }
}

/// The log, open. A clone writes to the same file.
#[derive(Debug, Clone)]
pub struct Log {
    connection: Arc<Mutex<Connection>>,
    queue: Arc<Mutex<Queue>>,
}

/// The entries waiting to be written, each with who waits for its place.
#[derive(Debug, Default)]
struct Queue {
    entries: Vec<(Entry, oneshot::Sender<Result<i64, LogError>>)>,
    /// Whether a task of the blocking pool is writing them.
    writing: bool,
}

/// The task that writes what waits in the queue, until nothing does.
struct Writer {
    connection: Arc<Mutex<Connection>>,
    queue: Arc<Mutex<Queue>>,
    /// Whether it left an empty queue to the next entry's task.
    finished: bool,
}

impl Writer {
    fn write(mut self) {
        let connection = self
            .connection
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        loop {
            let waiting = {
                let mut queue = self.queue.lock().unwrap_or_else(PoisonError::into_inner);
                if queue.entries.is_empty() {
                    queue.writing = false;
                    self.finished = true;
                    return;
                }
                std::mem::take(&mut queue.entries)
            };
            let (entries, written): (Vec<_>, Vec<_>) = waiting.into_iter().unzip();
            for (place, written) in insert_all(&connection, &entries).into_iter().zip(written) {
                // Whoever waited may have stopped waiting.
                let _ = written.send(place);
            }
        }
    }
}

impl Drop for Writer {
    /// A task that ended half way, or never ran, leaves the queue to the
    /// next entry's: those that waited are told it failed.
    fn drop(&mut self) {
        if self.finished {
            return;
        }
        let mut queue = self.queue.lock().unwrap_or_else(PoisonError::into_inner);
        queue.writing = false;
        queue.entries.clear();
    }
}

/// Writes `entries` in one transaction, and says the place of each. Where
/// that fails, each is written alone, and says what came of it.
fn insert_all(connection: &Connection, entries: &[Entry]) -> Vec<Result<i64, LogError>> {
    let together = || -> Result<Vec<i64>, rusqlite::Error> {
        let transaction = Transaction::new_unchecked(connection, TransactionBehavior::Immediate)?;
        let places = entries
            .iter()
            .map(|entry| insert(&transaction, entry))
            .collect::<Result<_, _>>()?;
        transaction.commit()?;
        Ok(places)
    };
    if entries.len() > 1
        && let Ok(places) = together()
    {
        return places.into_iter().map(Ok).collect();
    }
    entries
        .iter()
        .map(|entry| Ok(insert(connection, entry)?))
        .collect()
}

/// Writes `entry`, and says its place.
fn insert(connection: &Connection, entry: &Entry) -> Result<i64, rusqlite::Error> {
    connection.execute(
        "INSERT INTO calls (at_ms, session, agent, project, transport, connection_id, \
         connection, tool, sql, params, request, result, code, message, ran_as, rows, \
         matched, columns, bytes, masked, took_ms) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, \
         ?16, ?17, ?18, ?19, ?20, ?21)",
        params![
            entry.at_ms,
            entry.session,
            entry.agent,
            entry.project,
            entry.transport,
            entry.connection_id,
            entry.connection,
            entry.tool,
            entry.sql,
            entry.params,
            entry.request,
            entry.result,
            entry.code,
            entry.message,
            entry.ran_as,
            entry.rows,
            entry.matched,
            entry.columns,
            entry.bytes,
            entry.masked,
            entry.took_ms,
        ],
    )?;
    Ok(connection.last_insert_rowid())
}

/// Why a file was not opened as a log.
enum Unread {
    /// It is no log this version reads: not a database, a damaged one, or
    /// one of another schema. It is kept aside.
    NotALog(String),
    /// It could not be told what the file is: it is locked, or cannot be
    /// reached. It is left where it is. A log that is only busy must not
    /// be taken for a broken one.
    Failed(LogError),
}

impl From<rusqlite::Error> for Unread {
    fn from(error: rusqlite::Error) -> Self {
        let no_database = matches!(
            &error,
            rusqlite::Error::SqliteFailure(failure, _)
                if matches!(failure.code, ErrorCode::NotADatabase | ErrorCode::DatabaseCorrupt)
        );
        match no_database {
            true => Self::NotALog(error.to_string()),
            false => Self::Failed(error.into()),
        }
    }
}

impl From<LogError> for Unread {
    fn from(error: LogError) -> Self {
        Self::Failed(error)
    }
}

/// A log that was opened, and the name a file that could not be read was
/// kept under, for whoever tells the person.
#[derive(Debug)]
pub struct Opened {
    pub log: Log,
    pub kept_aside: Option<PathBuf>,
}

impl Log {
    /// Opens the log at `path`, making it when it is not there, private to
    /// the user. A file that is not a log this version reads is kept
    /// aside as `<name>.bad` and a new one is made: a log that cannot be
    /// read must not keep the server from starting. A file that could not
    /// be looked into (it is locked, or the disk failed) is an error, and
    /// stays where it is.
    ///
    /// It blocks on the disk. A caller on a runtime uses `spawn_blocking`.
    pub fn open(path: &Path) -> Result<Opened, LogError> {
        match Self::open_as_it_is(path) {
            Ok(log) => Ok(Opened {
                log,
                kept_aside: None,
            }),
            Err(Unread::Failed(error)) => Err(error),
            Err(Unread::NotALog(problem)) => {
                let aside = keep_aside(path)
                    .map_err(|error| LogError(format!("{problem}, and {error}")))?;
                match Self::open_as_it_is(path) {
                    Ok(log) => Ok(Opened {
                        log,
                        kept_aside: Some(aside),
                    }),
                    Err(Unread::Failed(error)) => Err(error),
                    Err(Unread::NotALog(problem)) => Err(LogError(problem)),
                }
            }
        }
    }

    fn open_as_it_is(path: &Path) -> Result<Self, Unread> {
        make_private(path)?;
        let connection = Connection::open(path)?;
        connection.busy_timeout(LOCK_WAIT)?;
        let version: i64 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;
        let tables: i64 =
            connection.query_row("SELECT count(*) FROM sqlite_master", [], |row| row.get(0))?;
        match (version, tables) {
            (0, 0) => {
                // Before the first table: it decides how the file is laid
                // out, and without it what retention deletes is never
                // given back to the disk.
                connection.execute_batch("PRAGMA auto_vacuum = INCREMENTAL")?;
                connection.execute_batch(&format!(
                    "BEGIN; {SCHEMA} PRAGMA user_version = {VERSION}; COMMIT;"
                ))?;
            }
            (VERSION, _) => {
                // Reads the table's definition, so a file with the version
                // and without the table is found out here.
                connection
                    .prepare("SELECT id, at_ms, tool, result FROM calls LIMIT 1")
                    .map_err(|error| Unread::NotALog(error.to_string()))?;
            }
            (other, _) => {
                return Err(Unread::NotALog(format!(
                    "the log is of version {other}, and this Tabletist reads version {VERSION}"
                )));
            }
        }
        Ok(Self {
            connection: Arc::new(Mutex::new(connection)),
            queue: Arc::default(),
        })
    }

    /// Writes `entry`, and says its place in the log ([`Logged::id`]).
    ///
    /// Entries that come while another is being written wait for it, and
    /// are then written together, in one transaction: a disk that takes
    /// long over each one takes it once for all that came at once, as the
    /// calls of a session that was stopped do.
    pub async fn record(&self, entry: Entry) -> Result<i64, LogError> {
        let (written, recorded) = oneshot::channel();
        let start = {
            let mut queue = self.queue.lock().unwrap_or_else(PoisonError::into_inner);
            queue.entries.push((entry, written));
            !std::mem::replace(&mut queue.writing, true)
        };
        if start {
            let writer = Writer {
                connection: self.connection.clone(),
                queue: self.queue.clone(),
                finished: false,
            };
            tokio::task::spawn_blocking(move || writer.write());
        }
        recorded
            .await
            .map_err(|_| LogError("the log's task did not end".to_owned()))?
    }

    /// Which of `calls` the log still holds, each named by its place and
    /// the time it arrived (`Entry::at_ms`), with the agent that made it.
    /// The query history keeps a call's place and reads its agent here, as
    /// the log keeps what an agent sent and nothing else does. The time is
    /// asked too: a place is not a call's for good, since SQLite gives the
    /// next call the place after the highest, and once every call is
    /// forgotten (or the file is made again) a place comes back.
    pub async fn holds(&self, calls: Vec<(i64, i64)>) -> Result<Vec<(i64, String)>, LogError> {
        self.blocking(move |connection| {
            let mut statement =
                connection.prepare("SELECT agent FROM calls WHERE id = ?1 AND at_ms = ?2")?;
            let mut held = Vec::new();
            for (id, at_ms) in calls {
                let agent: Option<String> = statement
                    .query_row([id, at_ms], |row| row.get(0))
                    .optional()?;
                if let Some(agent) = agent {
                    held.push((id, agent));
                }
            }
            Ok(held)
        })
        .await
    }

    /// The last `limit` entries, the newest first.
    pub async fn latest(&self, limit: u32) -> Result<Vec<Entry>, LogError> {
        let page = self.page(LogFilter::default(), None, limit).await?;
        Ok(page.calls.into_iter().map(|logged| logged.entry).collect())
    }

    /// The log at `path`, to read it as it is: nothing is made and nothing
    /// is set aside, so that reading it while the server is off changes
    /// nothing. `None` where there is no log yet. A write that a crash left
    /// half done is rolled back as the file is read, which SQLite does on
    /// any open, and which a connection that may not write cannot do: it
    /// is opened to write, and never writes.
    pub fn open_to_read(path: &Path) -> Result<Option<Self>, LogError> {
        if !path.exists() {
            return Ok(None);
        }
        let connection = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        connection.busy_timeout(LOCK_WAIT)?;
        let version: i64 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;
        let tables: i64 =
            connection.query_row("SELECT count(*) FROM sqlite_master", [], |row| row.get(0))?;
        match (version, tables) {
            // Made, and not written yet.
            (0, 0) => Ok(None),
            (VERSION, _) => Ok(Some(Self {
                connection: Arc::new(Mutex::new(connection)),
                queue: Arc::default(),
            })),
            (other, _) => Err(LogError(format!(
                "the log is of version {other}, and this Tabletist reads version {VERSION}"
            ))),
        }
    }

    /// A page of the log, the newest first: `limit` calls that pass
    /// `filter`, from before the one `before` names. A first page says the
    /// agents of the filter's connection too.
    pub async fn page(
        &self,
        filter: LogFilter,
        before: Option<i64>,
        limit: u32,
    ) -> Result<Page, LogError> {
        self.blocking(move |db| {
            let before = before.unwrap_or(i64::MAX);
            // One more than asked: whether there are more.
            let asked = i64::from(limit) + 1;
            let logged = |row: &Row<'_>| {
                Ok(Logged {
                    id: row.get(0)?,
                    entry: entry_at(row, 1)?,
                })
            };
            let (conditions, values) = filter.conditions();
            let mut calls = {
                let mut bound: Vec<&dyn rusqlite::ToSql> = values
                    .iter()
                    .map(|value| value as &dyn rusqlite::ToSql)
                    .collect();
                bound.extend([&before as &dyn rusqlite::ToSql, &asked]);
                db.prepare(&page_of(&conditions))?
                    .query_map(bound.as_slice(), logged)?
                    .collect::<Result<Vec<Logged>, rusqlite::Error>>()?
            };
            let total: i64 = db.query_row(
                &count_of(&conditions),
                rusqlite::params_from_iter(&values),
                |row| row.get(0),
            )?;
            let agents = match before == i64::MAX {
                true => agents_of(db, filter.connection.as_deref())?,
                false => Vec::new(),
            };
            let more = calls.len() > limit as usize;
            calls.truncate(limit as usize);
            Ok(Page {
                calls,
                total: u64::try_from(total).unwrap_or_default(),
                more,
                agents,
            })
        })
        .await
    }

    /// Deletes every entry of a call that arrived before `at_ms`, gives
    /// the room back to the disk, and says how many there were.
    pub async fn forget_before(&self, at_ms: i64) -> Result<usize, LogError> {
        self.blocking(move |connection| {
            let gone = connection.execute("DELETE FROM calls WHERE at_ms < ?1", [at_ms])?;
            // It answers a row for each page it frees: read them all.
            let mut vacuum = connection.prepare("PRAGMA incremental_vacuum")?;
            let mut pages = vacuum.query([])?;
            while pages.next()?.is_some() {}
            Ok(gone)
        })
        .await
    }

    async fn blocking<T: Send + 'static>(
        &self,
        work: impl FnOnce(&Connection) -> Result<T, LogError> + Send + 'static,
    ) -> Result<T, LogError> {
        let connection = self.connection.clone();
        tokio::task::spawn_blocking(move || {
            let connection = connection.lock().unwrap_or_else(PoisonError::into_inner);
            work(&connection)
        })
        .await
        .map_err(|error| LogError(format!("the log's task did not end: {error}")))?
    }
}

/// The columns of an entry, in [`entry_at`]'s order.
const COLUMNS: &str = "at_ms, session, agent, project, transport, connection_id, connection, \
    tool, sql, params, request, result, code, message, ran_as, rows, matched, columns, bytes, \
    masked, took_ms";

/// A page of the calls that pass `conditions`, from before a call: their
/// values are bound first, then the call's place and the page's length.
fn page_of(conditions: &[&str]) -> String {
    let mut all = conditions.to_vec();
    all.push("id < ?");
    format!(
        "SELECT id, {COLUMNS} FROM calls{} ORDER BY id DESC LIMIT ?",
        LogFilter::clause(&all)
    )
}

/// How many calls pass `conditions`.
fn count_of(conditions: &[&str]) -> String {
    format!(
        "SELECT count(*) FROM calls{}",
        LogFilter::clause(conditions)
    )
}

/// The agents that called on `connection`, or on any, by name.
fn agents_of(db: &Connection, connection: Option<&str>) -> Result<Vec<String>, rusqlite::Error> {
    let (sql, values): (&str, Vec<&str>) = match connection {
        Some(connection) => (
            "SELECT DISTINCT agent FROM calls WHERE connection_id = ? ORDER BY agent",
            vec![connection],
        ),
        None => (
            "SELECT DISTINCT agent FROM calls ORDER BY agent",
            Vec::new(),
        ),
    };
    db.prepare(sql)?
        .query_map(rusqlite::params_from_iter(values), |row| row.get(0))?
        .collect()
}

/// The entry in `row`, from its column `at` on, in [`COLUMNS`]'s order.
fn entry_at(row: &Row<'_>, at: usize) -> Result<Entry, rusqlite::Error> {
    Ok(Entry {
        at_ms: row.get(at)?,
        session: row.get(at + 1)?,
        agent: row.get(at + 2)?,
        project: row.get(at + 3)?,
        transport: row.get(at + 4)?,
        connection_id: row.get(at + 5)?,
        connection: row.get(at + 6)?,
        tool: row.get(at + 7)?,
        sql: row.get(at + 8)?,
        params: row.get(at + 9)?,
        request: row.get(at + 10)?,
        result: row.get(at + 11)?,
        code: row.get(at + 12)?,
        message: row.get(at + 13)?,
        ran_as: row.get(at + 14)?,
        rows: row.get(at + 15)?,
        matched: row.get(at + 16)?,
        columns: row.get(at + 17)?,
        bytes: row.get(at + 18)?,
        masked: row.get(at + 19)?,
        took_ms: row.get(at + 20)?,
    })
}

/// Makes the file when it is not there, and makes it its owner's alone
/// whether it was there or not: SQLite would make it with the permissions
/// any new file gets, and a file that was there has whatever it was given.
fn make_private(path: &Path) -> Result<(), LogError> {
    let failed = |what: &str, error: std::io::Error| {
        LogError(format!("could not {what} {}: {error}", path.display()))
    };
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    match options.open(path) {
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(failed("create", error)),
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
            .map_err(|error| failed("protect", error))?;
    }
    Ok(())
}

/// Renames a log that cannot be read to `<name>.bad`, or to `<name>.bad.1`
/// and so on where earlier ones exist, so that a second failure never
/// replaces the first copy. Gives the name it was kept under.
fn keep_aside(path: &Path) -> Result<PathBuf, LogError> {
    let name = |suffix: String| {
        let mut name = path.as_os_str().to_owned();
        name.push(suffix);
        PathBuf::from(name)
    };
    let aside = (0..1_000)
        .map(|n| match n {
            0 => name(".bad".into()),
            n => name(format!(".bad.{n}")),
        })
        // This also sees a link that leads nowhere, which a rename would
        // replace.
        .find(|candidate| candidate.symlink_metadata().is_err())
        .ok_or_else(|| LogError(format!("every .bad name of {} is taken", path.display())))?;
    std::fs::rename(path, &aside).map_err(|error| {
        LogError(format!(
            "could not keep {} aside as {}: {error}",
            path.display(),
            aside.display()
        ))
    })?;
    Ok(aside)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(at_ms: i64, tool: &str) -> Entry {
        Entry {
            at_ms,
            session: "s1".into(),
            agent: "Claude Code".into(),
            transport: "stdio".into(),
            tool: tool.into(),
            result: "ok".into(),
            ..Entry::default()
        }
    }

    fn open(path: &Path) -> Log {
        let opened = Log::open(path).unwrap();
        assert_eq!(opened.kept_aside, None);
        opened.log
    }

    #[tokio::test]
    async fn the_log_is_read_a_page_at_a_time_for_a_connection_or_for_all() {
        let dir = tempfile::tempdir().unwrap();
        let log = open(&dir.path().join("agents.sqlite3"));
        for n in 0..5 {
            let mut call = entry(n, "run_query");
            call.connection_id = Some(if n % 2 == 0 { "shop" } else { "ledger" }.into());
            log.record(call).await.unwrap();
        }
        let at = |page: &Page| {
            page.calls
                .iter()
                .map(|logged| logged.entry.at_ms)
                .collect::<Vec<_>>()
        };
        let all = LogFilter::default;
        let first = log.page(all(), None, 2).await.unwrap();
        assert_eq!((at(&first), first.total, first.more), (vec![4, 3], 5, true));
        let next = log.page(all(), Some(first.calls[1].id), 2).await.unwrap();
        assert_eq!((at(&next), next.more), (vec![2, 1], true));
        let shop = log
            .page(LogFilter::on(Some("shop".into())), None, 10)
            .await
            .unwrap();
        assert_eq!(
            (at(&shop), shop.total, shop.more),
            (vec![4, 2, 0], 3, false)
        );
    }

    #[tokio::test]
    async fn a_page_holds_the_calls_of_an_agent_a_result_and_a_search() {
        let dir = tempfile::tempdir().unwrap();
        let log = open(&dir.path().join("agents.sqlite3"));
        let calls = [
            ("Claude Code", "ok", Some("SELECT * FROM books"), None),
            (
                "Cursor",
                "refused",
                Some("select name from customers"),
                None,
            ),
            ("Cursor", "ok", None, Some("books, authors")),
            ("Cursor", "ok", Some("SELECT '100%' AS sure"), None),
            ("Claude Code", "error", Some("SELECT x_y FROM orders"), None),
        ];
        for (n, (agent, result, sql, request)) in calls.into_iter().enumerate() {
            let mut call = entry(n as i64, "run_query");
            call.agent = agent.into();
            call.result = result.into();
            call.sql = sql.map(str::to_owned);
            call.request = request.map(str::to_owned);
            call.connection_id = Some(if n == 4 { "ledger" } else { "shop" }.into());
            log.record(call).await.unwrap();
        }
        let at = |filter: LogFilter| {
            let log = log.clone();
            async move {
                let page = log.page(filter, None, 10).await.unwrap();
                let at: Vec<i64> = page.calls.iter().map(|logged| logged.entry.at_ms).collect();
                (at, page.total)
            }
        };
        let search = |text: &str| LogFilter {
            search: Some(text.into()),
            ..LogFilter::default()
        };
        // An agent, a result, and both, with the connection.
        let cursor = LogFilter {
            agent: Some("Cursor".into()),
            ..LogFilter::default()
        };
        assert_eq!(at(cursor.clone()).await, (vec![3, 2, 1], 3));
        let refused = LogFilter {
            result: Some("refused".into()),
            ..cursor.clone()
        };
        assert_eq!(at(refused).await, (vec![1], 1));
        let ledger = LogFilter {
            connection: Some("ledger".into()),
            ..cursor
        };
        assert_eq!(at(ledger).await, (vec![], 0));
        // The search reads the statement and the tables asked for, in any
        // case of English letters; `%` and `_` are themselves.
        assert_eq!(at(search("from")).await, (vec![4, 1, 0], 3));
        assert_eq!(at(search("authors")).await, (vec![2], 1));
        assert_eq!(at(search("100%")).await, (vec![3], 1));
        assert_eq!(at(search("x_y")).await, (vec![4], 1));
        assert_eq!(at(search("x%y")).await, (vec![], 0));
        assert_eq!(at(search("")).await, (vec![4, 3, 2, 1, 0], 5));
        // The agents of the connection, whatever else is asked; a later
        // page says none.
        let page = log
            .page(
                LogFilter {
                    connection: Some("shop".into()),
                    result: Some("error".into()),
                    ..LogFilter::default()
                },
                None,
                10,
            )
            .await
            .unwrap();
        assert_eq!(page.agents, ["Claude Code", "Cursor"]);
        assert_eq!(page.total, 0);
        let page = log.page(LogFilter::default(), None, 1).await.unwrap();
        let later = log
            .page(LogFilter::default(), Some(page.calls[0].id), 1)
            .await
            .unwrap();
        assert!(later.agents.is_empty());
    }

    #[tokio::test]
    async fn a_log_a_crash_left_mid_write_is_read_as_it_was_before() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("agents.sqlite3");
        let log = open(&path);
        for n in 0..2_000 {
            log.record(entry(n, "run_query")).await.unwrap();
        }
        drop(log);
        // A writer half way through, which spills to the file: the file
        // and its journal as a crash leaves them, copied while it holds
        // them.
        let writer = Connection::open(&path).unwrap();
        writer
            .execute_batch("PRAGMA cache_size = 1; BEGIN; UPDATE calls SET tool = 'x';")
            .unwrap();
        let crashed = dir.path().join("crashed.sqlite3");
        std::fs::copy(&path, &crashed).unwrap();
        let journal = |path: &Path| {
            let mut name = path.as_os_str().to_owned();
            name.push("-journal");
            PathBuf::from(name)
        };
        std::fs::copy(journal(&path), journal(&crashed)).unwrap();
        drop(writer);
        let log = Log::open_to_read(&crashed).unwrap().unwrap();
        let page = log.page(LogFilter::default(), None, 1).await.unwrap();
        assert_eq!(page.total, 2_000);
        assert_eq!(page.calls[0].entry.tool, "run_query");
    }

    #[test]
    fn one_connections_page_and_count_are_read_through_its_index() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("agents.sqlite3");
        drop(open(&path));
        let db = Connection::open(&path).unwrap();
        let plan = |sql: &str, values: &[&dyn rusqlite::ToSql]| {
            let mut statement = db.prepare(&format!("EXPLAIN QUERY PLAN {sql}")).unwrap();
            let rows = statement
                .query_map(values, |row| row.get::<_, String>(3))
                .unwrap();
            rows.map(Result::unwrap).collect::<Vec<_>>().join("; ")
        };
        let (one, _) = LogFilter::on(Some("shop".into())).conditions();
        let page = plan(&page_of(&one), params!["shop", i64::MAX, 201]);
        assert!(page.contains("calls_connection"), "{page}");
        let count = plan(&count_of(&one), params!["shop"]);
        assert!(count.contains("calls_connection"), "{count}");
        // With an agent, a result or a search too, alone or together:
        // still the connection's index, for the page and for the count.
        let parts = |agent: bool, result: bool, search: bool| LogFilter {
            connection: Some("shop".into()),
            agent: agent.then(|| "Cursor".into()),
            result: result.then(|| "ok".into()),
            search: search.then(|| "books".into()),
        };
        for (agent, result, search) in [
            (true, false, false),
            (false, true, false),
            (false, false, true),
            (true, true, false),
            (true, true, true),
        ] {
            let (conditions, values) = parts(agent, result, search).conditions();
            let mut bound: Vec<&dyn rusqlite::ToSql> = values
                .iter()
                .map(|value| value as &dyn rusqlite::ToSql)
                .collect();
            let count = plan(&count_of(&conditions), &bound);
            assert!(count.contains("calls_connection"), "{count}");
            bound.extend([&i64::MAX as &dyn rusqlite::ToSql, &201]);
            let page = plan(&page_of(&conditions), &bound);
            assert!(page.contains("calls_connection"), "{page}");
        }
        // Without a connection, a result's own index.
        let (conditions, values) = LogFilter {
            result: Some("ok".into()),
            ..LogFilter::default()
        }
        .conditions();
        let bound: Vec<&dyn rusqlite::ToSql> = values
            .iter()
            .map(|value| value as &dyn rusqlite::ToSql)
            .collect();
        let count = plan(&count_of(&conditions), &bound);
        assert!(count.contains("calls_result"), "{count}");
    }

    #[tokio::test]
    async fn a_log_opened_to_read_is_left_as_it_is() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("agents.sqlite3");
        // None yet, and none made.
        assert!(Log::open_to_read(&path).unwrap().is_none());
        assert!(!path.exists());
        open(&path)
            .record(entry(7, "describe_schema"))
            .await
            .unwrap();
        let log = Log::open_to_read(&path).unwrap().unwrap();
        assert_eq!(
            log.page(LogFilter::default(), None, 10)
                .await
                .unwrap()
                .total,
            1
        );
        // A file that is not a log is said so, and left where it is.
        let other = dir.path().join("other.sqlite3");
        let made = Connection::open(&other).unwrap();
        made.execute_batch("PRAGMA user_version = 9; CREATE TABLE t (x)")
            .unwrap();
        drop(made);
        assert!(Log::open_to_read(&other).is_err());
        assert!(other.exists());
        assert!(!dir.path().join("other.sqlite3.bad").exists());
    }

    #[tokio::test]
    async fn an_entry_is_read_back_as_it_was_written() {
        let dir = tempfile::tempdir().unwrap();
        let log = open(&dir.path().join("agents.sqlite3"));
        let whole = Entry {
            at_ms: 1_791_000_000_000,
            session: "5f0c".into(),
            agent: "Claude Code".into(),
            project: Some("tabletist-repo".into()),
            transport: "stdio".into(),
            connection_id: Some("b7e1".into()),
            connection: Some("Bookshop · dev".into()),
            tool: "run_query".into(),
            sql: Some("SELECT id FROM book_covers WHERE publisher_id = ?".into()),
            params: Some("[9100000000000000004]".into()),
            request: None,
            result: "ok".into(),
            code: None,
            message: None,
            ran_as: Some("read-only transaction".into()),
            rows: Some(3),
            matched: Some(3),
            columns: Some(1),
            bytes: Some(96),
            masked: Some(r#"["email"]"#.into()),
            took_ms: Some(4),
        };
        log.record(whole.clone()).await.unwrap();
        log.record(entry(1_791_000_000_500, "list_connections"))
            .await
            .unwrap();
        let read = log.latest(10).await.unwrap();
        assert_eq!(read.len(), 2);
        assert_eq!(read[0].tool, "list_connections", "the newest first");
        assert_eq!(read[1], whole);
        assert_eq!(log.latest(1).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn a_call_is_told_its_place_and_the_log_says_which_it_still_holds() {
        let dir = tempfile::tempdir().unwrap();
        let log = open(&dir.path().join("agents.sqlite3"));
        let old = log.record(entry(1_000, "run_query")).await.unwrap();
        let new = log.record(entry(9_000, "run_query")).await.unwrap();
        assert!(new > old);
        let page = log.page(LogFilter::default(), None, 10).await.unwrap();
        assert_eq!(page.calls[0].id, new, "the place is the one a page reads");
        // Each with its agent, which the log alone keeps.
        let agent = || "Claude Code".to_owned();
        assert_eq!(
            log.holds(vec![(old, 1_000), (new, 9_000), (new + 7, 9_000)])
                .await
                .unwrap(),
            [(old, agent()), (new, agent())]
        );
        // A place with another call's time is not that call.
        assert!(log.holds(vec![(old, 9_000)]).await.unwrap().is_empty());
        log.forget_before(5_000).await.unwrap();
        assert_eq!(
            log.holds(vec![(old, 1_000), (new, 9_000)]).await.unwrap(),
            [(new, agent())]
        );
    }

    /// Holds the log's connection, as a write that takes long does, until
    /// the sender is dropped.
    fn hold(log: &Log) -> std::sync::mpsc::Sender<()> {
        let (release, released) = std::sync::mpsc::channel::<()>();
        let (held, holding) = std::sync::mpsc::channel();
        let connection = log.connection.clone();
        std::thread::spawn(move || {
            let _connection = connection.lock().unwrap();
            held.send(()).unwrap();
            let _ = released.recv();
        });
        holding.recv().unwrap();
        release
    }

    /// Records `entries` while the connection is held, so that each waits
    /// for the one before, and gives what came of each.
    async fn record_at_once(log: &Log, entries: Vec<Entry>) -> Vec<Result<i64, LogError>> {
        let release = hold(log);
        let count = entries.len();
        let records: Vec<_> = entries
            .into_iter()
            .map(|entry| {
                let log = log.clone();
                tokio::spawn(async move { log.record(entry).await })
            })
            .collect();
        while log.queue.lock().unwrap().entries.len() < count {
            tokio::task::yield_now().await;
        }
        drop(release);
        let mut recorded = Vec::new();
        for record in records {
            recorded.push(record.await.unwrap());
        }
        recorded
    }

    /// How many times the file was written to: SQLite counts each
    /// transaction in the file's header.
    fn changes(path: &Path) -> u32 {
        let header = std::fs::read(path).unwrap();
        u32::from_be_bytes(header[24..28].try_into().unwrap())
    }

    #[tokio::test]
    async fn calls_that_come_at_once_are_written_in_one_transaction() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("agents.sqlite3");
        let log = open(&path);
        let before = changes(&path);
        let entries = (0..16).map(|at| entry(at, "run_query")).collect();
        let places: Vec<i64> = record_at_once(&log, entries)
            .await
            .into_iter()
            .map(Result::unwrap)
            .collect();
        assert_eq!(changes(&path), before + 1);
        // Each is told its own place, in the order they came.
        let mut sorted = places.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted, places);
        let written = log.latest(100).await.unwrap();
        assert_eq!(written.len(), 16);
        assert!(log.queue.lock().unwrap().entries.is_empty());
        // The next is written by a task of its own.
        log.record(entry(16, "run_query")).await.unwrap();
        assert_eq!(changes(&path), before + 2);
    }

    #[tokio::test]
    async fn an_entry_that_cannot_be_written_leaves_those_that_came_with_it() {
        let dir = tempfile::tempdir().unwrap();
        let log = open(&dir.path().join("agents.sqlite3"));
        log.connection
            .lock()
            .unwrap()
            .execute_batch(
                "CREATE TRIGGER refused BEFORE INSERT ON calls WHEN NEW.tool = 'refused' \
                 BEGIN SELECT RAISE(ABORT, 'refused'); END;",
            )
            .unwrap();
        let entries = vec![
            entry(1, "run_query"),
            entry(2, "refused"),
            entry(3, "describe_table"),
        ];
        let recorded = record_at_once(&log, entries).await;
        assert!(recorded[0].is_ok());
        assert!(recorded[1].is_err());
        assert!(recorded[2].is_ok());
        let tools: Vec<String> = log
            .latest(10)
            .await
            .unwrap()
            .into_iter()
            .map(|entry| entry.tool)
            .collect();
        assert_eq!(tools, ["describe_table", "run_query"]);
    }

    #[tokio::test]
    async fn a_place_that_comes_back_is_not_the_call_that_had_it() {
        let dir = tempfile::tempdir().unwrap();
        let log = open(&dir.path().join("agents.sqlite3"));
        let old = log.record(entry(1_000, "run_query")).await.unwrap();
        // Every call forgotten, the next one is given the same place.
        log.forget_before(5_000).await.unwrap();
        let next = log.record(entry(9_000, "run_query")).await.unwrap();
        assert_eq!(next, old, "SQLite gives the place again");
        assert!(log.holds(vec![(old, 1_000)]).await.unwrap().is_empty());
        assert_eq!(log.holds(vec![(next, 9_000)]).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn the_log_is_there_again_when_it_is_opened_again() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("agents.sqlite3");
        open(&path).record(entry(1, "run_query")).await.unwrap();
        let again = open(&path);
        again.record(entry(2, "run_query")).await.unwrap();
        assert_eq!(again.latest(10).await.unwrap().len(), 2);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn the_file_is_the_users_alone() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("agents.sqlite3");
        open(&path).record(entry(1, "run_query")).await.unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn a_file_that_was_there_becomes_the_users_alone() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("agents.sqlite3");
        std::fs::write(&path, "").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        open(&path).record(entry(1, "run_query")).await.unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }

    #[tokio::test]
    async fn a_log_that_is_only_locked_is_left_where_it_is() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("agents.sqlite3");
        open(&path).record(entry(1, "run_query")).await.unwrap();
        let other = rusqlite::Connection::open(&path).unwrap();
        other.execute_batch("BEGIN EXCLUSIVE").unwrap();
        let locked = Log::open(&path);
        assert!(locked.is_err(), "{locked:?}");
        assert!(!dir.path().join("agents.sqlite3.bad").exists());
        other.execute_batch("ROLLBACK").unwrap();
        // And it is the log it was.
        assert_eq!(open(&path).latest(10).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn entries_older_than_the_day_asked_are_forgotten() {
        let dir = tempfile::tempdir().unwrap();
        let log = open(&dir.path().join("agents.sqlite3"));
        for at_ms in [100, 200, 300] {
            log.record(entry(at_ms, "run_query")).await.unwrap();
        }
        assert_eq!(log.forget_before(300).await.unwrap(), 2);
        let left = log.latest(10).await.unwrap();
        assert_eq!(left.len(), 1);
        assert_eq!(left[0].at_ms, 300, "an entry of that very moment stays");
        assert_eq!(log.forget_before(300).await.unwrap(), 0);
    }

    #[tokio::test]
    async fn what_is_forgotten_gives_its_room_back() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("agents.sqlite3");
        let log = open(&path);
        let mut long = entry(1, "run_query");
        long.sql = Some("x".repeat(50_000));
        for _ in 0..20 {
            log.record(long.clone()).await.unwrap();
        }
        let full = std::fs::metadata(&path).unwrap().len();
        assert_eq!(log.forget_before(2).await.unwrap(), 20);
        let emptied = std::fs::metadata(&path).unwrap().len();
        assert!(emptied < full / 4, "{emptied} of {full}");
    }

    #[tokio::test]
    async fn a_file_that_is_no_log_is_kept_aside_and_a_new_one_made() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("agents.sqlite3");
        std::fs::write(&path, "this is not a database, and is longer than a header").unwrap();
        let opened = Log::open(&path).unwrap();
        let aside = dir.path().join("agents.sqlite3.bad");
        assert_eq!(opened.kept_aside.as_deref(), Some(aside.as_path()));
        assert!(
            std::fs::read_to_string(&aside)
                .unwrap()
                .starts_with("this is not")
        );
        opened.log.record(entry(1, "run_query")).await.unwrap();
        drop(opened);

        // A second one does not replace the first.
        std::fs::write(&path, "nor is this one, whatever it is that wrote it").unwrap();
        let opened = Log::open(&path).unwrap();
        assert_eq!(
            opened.kept_aside,
            Some(dir.path().join("agents.sqlite3.bad.1"))
        );
        assert!(
            std::fs::read_to_string(&aside)
                .unwrap()
                .starts_with("this is not")
        );
    }

    #[tokio::test]
    async fn a_log_of_a_later_version_is_kept_aside_and_not_written_to() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("agents.sqlite3");
        open(&path).record(entry(1, "run_query")).await.unwrap();
        rusqlite::Connection::open(&path)
            .unwrap()
            .execute_batch("PRAGMA user_version = 2")
            .unwrap();
        let opened = Log::open(&path).unwrap();
        assert!(opened.kept_aside.is_some());
        assert_eq!(opened.log.latest(10).await.unwrap(), []);
    }

    #[tokio::test]
    async fn a_database_of_something_else_is_kept_aside() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("agents.sqlite3");
        rusqlite::Connection::open(&path)
            .unwrap()
            .execute_batch("CREATE TABLE notes (body TEXT)")
            .unwrap();
        assert!(Log::open(&path).unwrap().kept_aside.is_some());
    }
}
