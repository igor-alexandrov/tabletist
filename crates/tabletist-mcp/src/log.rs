//! The log of calls: one entry for each call a tool answered or refused,
//! in a SQLite file of the person's own. It holds what was asked and what
//! came of it, and never a row's value.
//!
//! The file is opened once and written from the runtime's blocking pool.

use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use rusqlite::{Connection, ErrorCode, params};

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

/// The log, open. A clone writes to the same file.
#[derive(Debug, Clone)]
pub struct Log {
    connection: Arc<Mutex<Connection>>,
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
        })
    }

    /// Adds an entry.
    pub async fn record(&self, entry: Entry) -> Result<(), LogError> {
        self.blocking(move |connection| {
            connection.execute(
                "INSERT INTO calls (at_ms, session, agent, project, transport, connection_id, \
                 connection, tool, sql, params, request, result, code, message, ran_as, rows, \
                 matched, columns, bytes, took_ms) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, \
                 ?16, ?17, ?18, ?19, ?20)",
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
                    entry.took_ms,
                ],
            )?;
            Ok(())
        })
        .await
    }

    /// The last `limit` entries, the newest first.
    pub async fn latest(&self, limit: u32) -> Result<Vec<Entry>, LogError> {
        self.blocking(move |connection| {
            let mut statement = connection.prepare(
                "SELECT at_ms, session, agent, project, transport, connection_id, connection, \
                 tool, sql, params, request, result, code, message, ran_as, rows, matched, \
                 columns, bytes, took_ms FROM calls ORDER BY id DESC LIMIT ?1",
            )?;
            let entries = statement
                .query_map([limit], |row| {
                    Ok(Entry {
                        at_ms: row.get(0)?,
                        session: row.get(1)?,
                        agent: row.get(2)?,
                        project: row.get(3)?,
                        transport: row.get(4)?,
                        connection_id: row.get(5)?,
                        connection: row.get(6)?,
                        tool: row.get(7)?,
                        sql: row.get(8)?,
                        params: row.get(9)?,
                        request: row.get(10)?,
                        result: row.get(11)?,
                        code: row.get(12)?,
                        message: row.get(13)?,
                        ran_as: row.get(14)?,
                        rows: row.get(15)?,
                        matched: row.get(16)?,
                        columns: row.get(17)?,
                        bytes: row.get(18)?,
                        took_ms: row.get(19)?,
                    })
                })?
                .collect::<Result<Vec<Entry>, rusqlite::Error>>()?;
            Ok(entries)
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
