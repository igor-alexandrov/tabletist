//! Query history: every statement the app runs on a connection, from the
//! SQL editor, a grid's save, a filter or sort set on a table, and the
//! agents, kept per connection on this computer in a SQLite file of the
//! person's own (`history.sqlite3`, beside the Agents Log).
//!
//! An entry keeps the statement's text, when it ran, how long it took, its
//! rows and how it ended. Never a result's rows. And never what an agent
//! sent: that is the Agents Log's alone (`AGENTS.md`), so an agent's entry
//! keeps the call's place in that log, its tool, its rows, how long it took
//! and how it ended, and links to the call there.
//!
//! The file is written from the backend's runtime, an entry at a time: a
//! statement never waits for it, and an entry the file cannot take is
//! dropped with a line in the app's log.

use std::fmt;
use std::path::{Path, PathBuf};

use rusqlite::{Connection, ErrorCode, OptionalExtension, Row, params};

use crate::session::KeptTab;

/// The schema's version, in `PRAGMA user_version`.
const VERSION: i64 = 1;

/// How long a write waits for another Tabletist's lock on the file.
const LOCK_WAIT: std::time::Duration = std::time::Duration::from_secs(2);

const SCHEMA: &str = "
CREATE TABLE entries (
  id            INTEGER PRIMARY KEY,
  connection_id TEXT    NOT NULL,
  at_ms         INTEGER NOT NULL,
  source        TEXT    NOT NULL,
  detail        TEXT    NOT NULL,
  tool          TEXT,
  database      TEXT    NOT NULL,
  sql           TEXT,
  call_id       INTEGER,
  kind          TEXT    NOT NULL,
  read_only     INTEGER NOT NULL,
  rows          INTEGER,
  took_ms       INTEGER,
  result        TEXT    NOT NULL,
  code          TEXT,
  message       TEXT,
  query         TEXT
);
CREATE INDEX entries_connection ON entries (connection_id, id);
";

/// Where a statement came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Source {
    /// A SQL editor's run: a statement as it was split and run.
    Sql,
    /// A table's Save: the whole transaction.
    Save,
    /// The `SELECT` a table ran for a filter or a sort the person set.
    Filter,
    /// An agent's call, in the Agents Log.
    Agent,
}

impl Source {
    pub const ALL: [Self; 4] = [Self::Sql, Self::Save, Self::Filter, Self::Agent];

    /// Its word in the file and in the terminal look's filter.
    pub fn word(self) -> &'static str {
        match self {
            Self::Sql => "sql",
            Self::Save => "save",
            Self::Filter => "filter",
            Self::Agent => "agent",
        }
    }

    pub fn of(word: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|source| source.word() == word)
    }
}

/// How a statement ended, as the Result column says it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Outcome {
    Ok,
    /// A save that was written.
    Saved,
    Error,
    /// Stopped by the person, by the timeout, or by a session that ended.
    Cancelled,
    /// An agent's write the person let through.
    Approved,
    /// An agent's write the person turned down.
    Denied,
    /// An agent's call the app refused before it reached the database.
    Refused,
}

impl Outcome {
    pub const ALL: [Self; 7] = [
        Self::Ok,
        Self::Saved,
        Self::Error,
        Self::Cancelled,
        Self::Approved,
        Self::Denied,
        Self::Refused,
    ];

    /// Its word in the file and in the terminal look's filter.
    pub fn word(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Saved => "saved",
            Self::Error => "error",
            Self::Cancelled => "cancelled",
            Self::Approved => "approved",
            Self::Denied => "denied",
            Self::Refused => "refused",
        }
    }

    pub fn of(word: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|outcome| outcome.word() == word)
    }

    /// An agent's call's end, as the Agents Log says it.
    pub fn of_call(result: &str) -> Self {
        match result {
            "ok" => Self::Ok,
            "refused" => Self::Refused,
            "timeout" | "cancelled" | "stopped" => Self::Cancelled,
            _ => Self::Error,
        }
    }
}

/// Whether a statement only reads: what History runs again.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Read,
    Write,
}

impl Kind {
    pub fn of(dialect: tabletist_db::Dialect, sql: &str) -> Self {
        match tabletist_db::sql::kind(dialect, sql) {
            tabletist_db::sql::StatementKind::Read => Self::Read,
            tabletist_db::sql::StatementKind::Write => Self::Write,
        }
    }

    fn word(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
        }
    }
}

/// One statement the app ran, or one agent's call. Its `Debug` leaves out
/// the statement, the database's words and the filter: what a person types
/// is never logged.
#[derive(Clone, PartialEq)]
pub struct Entry {
    /// When it began, in milliseconds of Unix time.
    pub at_ms: i64,
    pub source: Source,
    /// The SQL editor's tab ("Query 1"), the table's name, or the agent's.
    pub detail: String,
    /// An agent's tool.
    pub tool: Option<String>,
    /// The saved connection's id.
    pub connection: String,
    /// The database it ran on (the file, for SQLite).
    pub database: String,
    /// The statement, its values written in: a save's whole transaction.
    /// Never an agent's.
    pub sql: Option<String>,
    /// An agent's call, by its place in the Agents Log.
    pub call: Option<i64>,
    pub kind: Kind,
    /// It ran in a read-only transaction.
    pub read_only: bool,
    /// The rows it returned, or changed.
    pub rows: Option<u64>,
    pub took_ms: Option<u64>,
    pub outcome: Outcome,
    /// The database's code for an error.
    pub code: Option<String>,
    /// The first line of an error's message.
    pub message: Option<String>,
    /// A filter's table, sort and filter bar, to set them again.
    pub query: Option<KeptTab>,
}

impl fmt::Debug for Entry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Entry")
            .field("at_ms", &self.at_ms)
            .field("source", &self.source)
            .field("call", &self.call)
            .field("kind", &self.kind)
            .field("rows", &self.rows)
            .field("took_ms", &self.took_ms)
            .field("outcome", &self.outcome)
            .field("code", &self.code)
            .finish_non_exhaustive()
    }
}

impl Entry {
    /// A statement that History runs again: a read of the SQL editor's, or
    /// a filter's `SELECT`. Never a write, a save or an agent's call.
    pub fn runs_again(&self) -> bool {
        match self.source {
            Source::Sql => self.kind == Kind::Read && self.sql.is_some(),
            Source::Filter => self.sql.is_some(),
            Source::Save | Source::Agent => false,
        }
    }

    /// What a search looks through, in lower case: the statement, or an
    /// agent's name and tool, which is all an agent's entry has.
    pub fn searched(&self) -> String {
        match self.source {
            Source::Agent => format!(
                "{} {}",
                self.detail.to_lowercase(),
                self.tool.as_deref().unwrap_or_default().to_lowercase()
            ),
            Source::Sql | Source::Save | Source::Filter => {
                self.sql.as_deref().unwrap_or_default().to_lowercase()
            }
        }
    }
}

/// The first line of a database's message, as an entry keeps it.
pub fn first_line(message: &str) -> String {
    message.lines().next().unwrap_or_default().trim().to_owned()
}

/// Which entries a History tab shows.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Filter {
    pub source: Option<Source>,
    pub outcome: Option<Outcome>,
    /// Text the statements shown hold, in any case: none where it is empty.
    pub search: String,
}

impl Filter {
    pub fn is_set(&self) -> bool {
        self.source.is_some() || self.outcome.is_some() || !self.search.trim().is_empty()
    }

    /// Whether `entry` passes, `searched` being [`Entry::searched`].
    pub fn passes(&self, entry: &Entry, searched: &str) -> bool {
        self.source.is_none_or(|source| source == entry.source)
            && self.outcome.is_none_or(|outcome| outcome == entry.outcome)
            && searched.contains(&self.search.trim().to_lowercase())
    }
}

/// The terminal look's filter line, read: plain words search the SQL,
/// `source:` and `result:` choose, and any other `word:` is a token this
/// version does not know, shown and ignored.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Line {
    pub filter: Filter,
    /// The tokens it does not know, as typed.
    pub unknown: Vec<String>,
}

impl Line {
    pub fn read(text: &str) -> Self {
        let mut line = Self::default();
        let mut words = Vec::new();
        for word in text.split_whitespace() {
            let Some((name, value)) = word.split_once(':') else {
                words.push(word);
                continue;
            };
            if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphabetic()) {
                words.push(word);
                continue;
            }
            let known = match name.to_ascii_lowercase().as_str() {
                "source" => Source::of(&value.to_ascii_lowercase())
                    .map(|source| line.filter.source = Some(source)),
                "result" => Outcome::of(&value.to_ascii_lowercase())
                    .map(|outcome| line.filter.outcome = Some(outcome)),
                _ => None,
            };
            if known.is_none() {
                line.unknown.push(word.to_owned());
            }
        }
        line.filter.search = words.join(" ");
        line
    }
}

/// Why the history could not be read or written. Its words name the file
/// and SQLite's own error, never a statement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreError(String);

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<rusqlite::Error> for StoreError {
    fn from(error: rusqlite::Error) -> Self {
        Self(error.to_string())
    }
}

/// The history's file, open. It blocks on the disk: the backend calls it
/// from its blocking pool.
pub struct Store {
    connection: Connection,
}

/// A store that was opened, and the name a file that could not be read
/// was kept under.
pub struct Opened {
    pub store: Store,
    pub kept_aside: Option<PathBuf>,
}

/// Why a file could not be opened as the history.
enum Unread {
    /// It is not one this version reads: kept aside, a new one made.
    NotHistory(String),
    Failed(StoreError),
}

impl From<rusqlite::Error> for Unread {
    fn from(error: rusqlite::Error) -> Self {
        let no_database = matches!(
            &error,
            rusqlite::Error::SqliteFailure(failure, _)
                if matches!(failure.code, ErrorCode::NotADatabase | ErrorCode::DatabaseCorrupt)
        );
        match no_database {
            true => Self::NotHistory(error.to_string()),
            false => Self::Failed(error.into()),
        }
    }
}

impl From<StoreError> for Unread {
    fn from(error: StoreError) -> Self {
        Self::Failed(error)
    }
}

/// An entry's columns, in [`entry_at`]'s order.
const COLUMNS: &str = "connection_id, at_ms, source, detail, tool, database, sql, call_id, \
    kind, read_only, rows, took_ms, result, code, message, query";

impl Store {
    /// Opens the history at `path`, making it, private to the user, when
    /// it is not there. A file that is not a history this version reads is
    /// kept aside as `<name>.bad` and a new one is made.
    pub fn open(path: &Path) -> Result<Opened, StoreError> {
        match Self::open_as_it_is(path) {
            Ok(store) => Ok(Opened {
                store,
                kept_aside: None,
            }),
            Err(Unread::Failed(error)) => Err(error),
            Err(Unread::NotHistory(problem)) => {
                let aside = crate::util::keep_aside(path, &problem)
                    .ok_or_else(|| StoreError(format!("{problem}, and it could not be moved")))?;
                match Self::open_as_it_is(path) {
                    Ok(store) => Ok(Opened {
                        store,
                        kept_aside: Some(aside),
                    }),
                    Err(Unread::Failed(error)) => Err(error),
                    Err(Unread::NotHistory(problem)) => Err(StoreError(problem)),
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
                // Before the first table, so what trimming deletes is given
                // back to the disk.
                connection.execute_batch("PRAGMA auto_vacuum = INCREMENTAL")?;
                connection.execute_batch(&format!(
                    "BEGIN; {SCHEMA} PRAGMA user_version = {VERSION}; COMMIT;"
                ))?;
            }
            (VERSION, _) => {
                connection
                    .prepare(&format!("SELECT id, {COLUMNS} FROM entries LIMIT 1"))
                    .map_err(|error| Unread::NotHistory(error.to_string()))?;
            }
            // A newer Tabletist's: left as it is, for that version.
            (other, _) if other > VERSION => {
                return Err(Unread::Failed(StoreError(format!(
                    "the history is of version {other}, and this Tabletist reads version {VERSION}"
                ))));
            }
            (other, _) => {
                return Err(Unread::NotHistory(format!(
                    "the history is of version {other}, and this Tabletist reads version {VERSION}"
                )));
            }
        }
        Ok(Self { connection })
    }

    /// Adds `entry`, then keeps the newest `keep` of its connection's.
    pub fn append(&self, entry: &Entry, keep: u32) -> Result<(), StoreError> {
        let query = entry
            .query
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|error| StoreError(error.to_string()))?;
        self.connection.execute(
            &format!(
                "INSERT INTO entries ({COLUMNS}) VALUES \
                 (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)"
            ),
            params![
                entry.connection,
                entry.at_ms,
                entry.source.word(),
                entry.detail,
                entry.tool,
                entry.database,
                entry.sql,
                entry.call,
                entry.kind.word(),
                entry.read_only,
                entry.rows.map(count),
                entry.took_ms.map(count),
                entry.outcome.word(),
                entry.code,
                entry.message,
                query,
            ],
        )?;
        self.trim_connection(&entry.connection, keep)?;
        Ok(())
    }

    /// The newest `keep` entries of `connection`, the newest first. An
    /// entry a newer version wrote, with words this one does not know, is
    /// left out.
    pub fn load(&self, connection: &str, keep: u32) -> Result<Vec<Entry>, StoreError> {
        let mut statement = self.connection.prepare(&format!(
            "SELECT {COLUMNS} FROM entries WHERE connection_id = ?1 ORDER BY id DESC LIMIT ?2"
        ))?;
        let rows = statement.query_map(params![connection, keep], entry_at)?;
        let mut entries = Vec::new();
        for row in rows {
            if let Some(entry) = row? {
                entries.push(entry);
            }
        }
        Ok(entries)
    }

    /// Deletes every entry of `connection`, or of every connection.
    pub fn clear(&self, connection: Option<&str>) -> Result<(), StoreError> {
        match connection {
            Some(connection) => self
                .connection
                .execute("DELETE FROM entries WHERE connection_id = ?1", [connection])?,
            None => self.connection.execute("DELETE FROM entries", [])?,
        };
        self.vacuum()
    }

    /// Keeps the newest `keep` entries of each connection.
    pub fn trim(&self, keep: u32) -> Result<(), StoreError> {
        let connections: Vec<String> = self
            .connection
            .prepare("SELECT DISTINCT connection_id FROM entries")?
            .query_map([], |row| row.get(0))?
            .collect::<Result<_, _>>()?;
        for connection in connections {
            self.trim_connection(&connection, keep)?;
        }
        self.vacuum()
    }

    /// How many entries `connection` has: for tests.
    #[cfg(test)]
    fn count(&self, connection: &str) -> i64 {
        self.connection
            .query_row(
                "SELECT count(*) FROM entries WHERE connection_id = ?1",
                [connection],
                |row| row.get(0),
            )
            .unwrap()
    }

    fn trim_connection(&self, connection: &str, keep: u32) -> Result<(), StoreError> {
        // The oldest that is kept, where there are more than `keep`.
        let last: Option<i64> = self
            .connection
            .query_row(
                "SELECT id FROM entries WHERE connection_id = ?1 \
                 ORDER BY id DESC LIMIT 1 OFFSET ?2",
                params![connection, keep.saturating_sub(1)],
                |row| row.get(0),
            )
            .optional()?;
        match (keep, last) {
            (0, _) => {
                self.connection
                    .execute("DELETE FROM entries WHERE connection_id = ?1", [connection])?;
            }
            (_, Some(last)) => {
                self.connection.execute(
                    "DELETE FROM entries WHERE connection_id = ?1 AND id < ?2",
                    params![connection, last],
                )?;
            }
            (_, None) => {}
        }
        Ok(())
    }

    fn vacuum(&self) -> Result<(), StoreError> {
        // It answers a row for each page it frees: read them all.
        let mut vacuum = self.connection.prepare("PRAGMA incremental_vacuum")?;
        let mut pages = vacuum.query([])?;
        while pages.next()?.is_some() {}
        Ok(())
    }
}

/// The entry at `row`, read in [`COLUMNS`]' order. `None` for one whose
/// words this version does not know.
fn entry_at(row: &Row<'_>) -> rusqlite::Result<Option<Entry>> {
    let source: String = row.get(2)?;
    let kind: String = row.get(8)?;
    let result: String = row.get(12)?;
    let query: Option<String> = row.get(15)?;
    let kind = match kind.as_str() {
        "read" => Some(Kind::Read),
        "write" => Some(Kind::Write),
        _ => None,
    };
    let (Some(source), Some(kind), Some(outcome)) =
        (Source::of(&source), kind, Outcome::of(&result))
    else {
        return Ok(None);
    };
    let unsigned = |value: Option<i64>| value.and_then(|value| u64::try_from(value).ok());
    Ok(Some(Entry {
        connection: row.get(0)?,
        at_ms: row.get(1)?,
        source,
        detail: row.get(3)?,
        tool: row.get(4)?,
        database: row.get(5)?,
        sql: row.get(6)?,
        call: row.get(7)?,
        kind,
        read_only: row.get(9)?,
        rows: unsigned(row.get(10)?),
        took_ms: unsigned(row.get(11)?),
        outcome,
        code: row.get(13)?,
        message: row.get(14)?,
        query: query.and_then(|query| serde_json::from_str(&query).ok()),
    }))
}

fn count(count: u64) -> i64 {
    i64::try_from(count).unwrap_or(i64::MAX)
}

/// Makes the file at `path`, 0600, where it is not there, and makes it
/// 0600 where it is.
fn make_private(path: &Path) -> Result<(), StoreError> {
    let failed = |what: &str, error: std::io::Error| {
        StoreError(format!("could not {what} {}: {error}", path.display()))
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| failed("make the folder of", error))?;
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) fn read(sql: &str) -> Entry {
        Entry {
            at_ms: 1_792_000_000_000,
            source: Source::Sql,
            detail: "Query 1".into(),
            tool: None,
            connection: "bookshop".into(),
            database: "bookshop_development".into(),
            sql: Some(sql.into()),
            call: None,
            kind: Kind::Read,
            read_only: true,
            rows: Some(6),
            took_ms: Some(38),
            outcome: Outcome::Ok,
            code: None,
            message: None,
            query: None,
        }
    }

    fn agent() -> Entry {
        Entry {
            source: Source::Agent,
            detail: "Claude Code".into(),
            tool: Some("run_query".into()),
            sql: None,
            call: Some(42),
            ..read("")
        }
    }

    fn store(dir: &tempfile::TempDir) -> Store {
        let opened = Store::open(&dir.path().join("history.sqlite3")).unwrap();
        assert_eq!(opened.kept_aside, None);
        opened.store
    }

    #[test]
    fn an_entry_is_read_back_as_it_was_written() {
        let dir = tempfile::tempdir().unwrap();
        let store = store(&dir);
        let mut filter = read("SELECT * FROM book_covers WHERE kind = 'print'");
        filter.source = Source::Filter;
        filter.detail = "book_covers".into();
        filter.query = Some(KeptTab {
            schema: "public".into(),
            name: "book_covers".into(),
            kind: "table".into(),
            sort: vec![("created_at".into(), "asc".into())],
            filter: crate::session::KeptFilter {
                rows: vec![("kind".into(), "eq".into(), "print".into())],
                ..Default::default()
            },
            pinned: true,
            rest: Default::default(),
        });
        let mut failed = read("SELECT id, created FROM book_covers");
        failed.outcome = Outcome::Error;
        failed.code = Some("42703".into());
        failed.message = Some("column \"created\" does not exist".into());
        failed.rows = None;
        for entry in [&filter, &failed, &agent()] {
            store.append(entry, 1_000).unwrap();
        }
        let loaded = store.load("bookshop", 1_000).unwrap();
        assert_eq!(loaded, [agent(), failed, filter], "the newest first");
        assert!(store.load("ledger", 1_000).unwrap().is_empty());
    }

    #[test]
    fn each_write_keeps_the_newest_of_its_connection_alone() {
        let dir = tempfile::tempdir().unwrap();
        let store = store(&dir);
        let mut ledger = read("SELECT 1");
        ledger.connection = "ledger".into();
        store.append(&ledger, 2).unwrap();
        for n in 0..5 {
            store.append(&read(&format!("SELECT {n}")), 3).unwrap();
        }
        let kept: Vec<_> = store
            .load("bookshop", 100)
            .unwrap()
            .into_iter()
            .map(|entry| entry.sql.unwrap())
            .collect();
        assert_eq!(kept, ["SELECT 4", "SELECT 3", "SELECT 2"]);
        assert_eq!(store.count("ledger"), 1);
        store.trim(1).unwrap();
        assert_eq!((store.count("bookshop"), store.count("ledger")), (1, 1));
        store.trim(0).unwrap();
        assert_eq!((store.count("bookshop"), store.count("ledger")), (0, 0));
    }

    #[test]
    fn clear_clears_one_connection_or_every_one() {
        let dir = tempfile::tempdir().unwrap();
        let store = store(&dir);
        let mut ledger = read("SELECT 1");
        ledger.connection = "ledger".into();
        store.append(&ledger, 10).unwrap();
        store.append(&read("SELECT 2"), 10).unwrap();
        store.clear(Some("bookshop")).unwrap();
        assert_eq!((store.count("bookshop"), store.count("ledger")), (0, 1));
        store.clear(None).unwrap();
        assert_eq!(store.count("ledger"), 0);
    }

    #[test]
    fn a_file_that_is_no_history_is_kept_aside() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history.sqlite3");
        std::fs::write(&path, "not a database, and long enough to be read as one").unwrap();
        let opened = Store::open(&path).unwrap();
        assert!(opened.kept_aside.is_some());
        opened.store.append(&read("SELECT 1"), 10).unwrap();
    }

    #[test]
    fn a_newer_versions_file_is_left_as_it_is() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history.sqlite3");
        store(&dir).append(&read("SELECT 1"), 10).unwrap();
        Connection::open(&path)
            .unwrap()
            .execute_batch("PRAGMA user_version = 2")
            .unwrap();
        assert!(Store::open(&path).is_err());
        assert!(!dir.path().join("history.sqlite3.bad").exists());
    }

    #[cfg(unix)]
    #[test]
    fn the_file_is_the_users_alone() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        store(&dir);
        let path = dir.path().join("history.sqlite3");
        let mode = std::fs::metadata(path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }

    #[test]
    fn an_agents_entry_is_searched_by_its_agent_and_tool_alone() {
        let entry = agent();
        let filter = |search: &str| Filter {
            search: search.into(),
            ..Filter::default()
        };
        assert!(filter("claude").passes(&entry, &entry.searched()));
        assert!(filter("RUN_QUERY").passes(&entry, &entry.searched()));
        assert!(!filter("select").passes(&entry, &entry.searched()));
        let sql = read("SELECT name FROM publishers");
        assert!(filter("from PUBLISHERS").passes(&sql, &sql.searched()));
        assert!(!filter("claude").passes(&sql, &sql.searched()));
    }

    #[test]
    fn the_source_the_result_and_the_search_combine() {
        let mut failed = read("SELECT id, created FROM book_covers");
        failed.outcome = Outcome::Error;
        let ok = read("SELECT * FROM book_covers");
        let both = Filter {
            source: Some(Source::Sql),
            outcome: Some(Outcome::Error),
            search: "book_covers".into(),
        };
        assert!(both.passes(&failed, &failed.searched()));
        assert!(!both.passes(&ok, &ok.searched()));
        let agents = Filter {
            source: Some(Source::Agent),
            ..both
        };
        assert!(!agents.passes(&failed, &failed.searched()));
    }

    #[test]
    fn the_terminal_line_reads_its_tokens_and_keeps_those_it_does_not_know() {
        let line = Line::read("source:save  result:error  covers kind:print source:nope");
        assert_eq!(line.filter.source, Some(Source::Save));
        assert_eq!(line.filter.outcome, Some(Outcome::Error));
        assert_eq!(line.filter.search, "covers");
        assert_eq!(line.unknown, ["kind:print", "source:nope"]);
        // A cast is a word of the SQL, not a token.
        assert_eq!(
            Line::read("created_at::date").filter.search,
            "created_at::date"
        );
    }

    #[test]
    fn only_reads_and_filters_run_again() {
        assert!(read("SELECT 1").runs_again());
        let mut write = read("UPDATE book_covers SET kind = 'ebook'");
        write.kind = Kind::Write;
        assert!(!write.runs_again());
        let mut save = read("BEGIN; UPDATE book_covers SET kind = 'ebook'; COMMIT;");
        save.source = Source::Save;
        assert!(!save.runs_again());
        assert!(!agent().runs_again());
    }

    #[test]
    fn an_entry_logs_no_statement() {
        let mut entry = read("SELECT secret FROM vault");
        entry.message = Some("near secret".into());
        let said = format!("{entry:?}");
        assert!(!said.contains("secret"), "{said}");
    }
}
