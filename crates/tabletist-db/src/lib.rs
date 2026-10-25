//! Access to PostgreSQL, MySQL and SQLite for Tabletist.
//!
//! The crate writes to a connected database in two places:
//! [`Connection::write`], which saves edited rows, and
//! [`Connection::run_script`] in [`ScriptMode::Write`]. Both only on a
//! session opened [`Access::Writable`]. Everything else stays fenced there
//! as on a read-only session: row fetches, counts and lookups run in read-only
//! transactions (on SQLite under `query_only`), and a script run in
//! [`ScriptMode::ReadOnly`] cannot write. Neither can
//! [`Connection::run_agent_read`], which is such a run of one statement.

mod adapter;
pub mod agent;
mod catalog;
mod check;
mod class;
pub mod complete;
mod dialect;
mod error;
pub mod fixtures;
mod lookup;
mod mysql;
mod pg;
mod query;
mod reserved;
mod script;
mod spec;
pub mod sql;
mod sqlite;
mod ssh;
pub mod ssh_config;
mod tls;
mod value;
mod write;

use std::fmt;
use std::sync::Arc;

use adapter::Adapter;
pub use catalog::{
    Acts, ColumnInfo, ForeignKeyInfo, IndexInfo, MAX_LISTED, ObjectInfo, ObjectKind, ObjectRef,
    ReferencingKey, Structure,
};
pub use class::{ColumnClass, Temporal, column_class, temporal};
pub use dialect::{
    DeleteParts, Dialect, InsertParts, InsertStatement, RowDeletion, RowUpdate, Sql, UpdateParts,
    escape_like, quote_literal,
};
pub use error::{Error, Named, Result, SshStage};
pub use lookup::{Lookup, LookupPage, LookupPart, LookupRow, Matched};
pub use query::{Filter, FilterOp, RowPage, RowQuery, Sort, SortDir};
pub use script::{
    ScriptEnd, ScriptMode, ScriptOutcome, StatementOutcome, StatementResult, StopFlag,
};
pub use spec::{ConnectSpec, Driver, ParsedUrl, Secrets, SshAuth, SshSpec, TlsMode};
pub use ssh::HostKeys;
pub use value::{ColumnMeta, Value, ValueKind, value_from_pg_text};
pub use write::{
    CellChange, ChangeSet, Conflict, InsertValue, MAX_RUN, NewValue, RowChange, RowDelete,
    RowInsert, WriteOutcome, delete_runs,
};

/// Whether a session may write. [`Connection::write`] and a script run in
/// [`ScriptMode::Write`] are the calls that do, and both are refused on a
/// read-only session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Access {
    /// Nothing can write: the session itself is read-only.
    #[default]
    ReadOnly,
    /// The session is read-write. Browsing still reads in read-only
    /// transactions, and a script writes only when it is run to.
    Writable,
}

/// An open database session, and the SSH tunnel it runs through, if any.
pub struct Connection {
    inner: Inner,
    access: Access,
    /// Declared after `inner`, so the driver closes before its tunnel.
    tunnel: Option<ssh::Tunnel>,
}

enum Inner {
    Sqlite(sqlite::Conn),
    // Boxed: a PostgreSQL client is much larger than a SQLite handle.
    Postgres(Box<pg::Conn>),
    MySql(Box<mysql::Conn>),
}

impl Inner {
    async fn connect(
        spec: &ConnectSpec,
        secrets: &Secrets,
        via: Option<u16>,
        access: Access,
    ) -> Result<Self> {
        match spec.driver {
            Driver::Sqlite => {
                let path = spec
                    .sqlite_path
                    .as_ref()
                    .ok_or_else(|| Error::InvalidSpec("choose a SQLite file".into()))?;
                Ok(Self::Sqlite(sqlite::Conn::open(path, access).await?))
            }
            Driver::Postgres => Ok(Self::Postgres(Box::new(
                pg::Conn::connect(spec, secrets, via, access).await?,
            ))),
            Driver::MySql => Ok(Self::MySql(Box::new(
                mysql::Conn::connect(spec, secrets, via, access).await?,
            ))),
        }
    }
}

impl Connection {
    /// A read-only session, with no SSH host keys trusted.
    pub async fn connect(spec: &ConnectSpec, secrets: &Secrets) -> Result<Self> {
        Self::connect_with(spec, secrets, &HostKeys::default(), Access::ReadOnly).await
    }

    /// Connects, through an SSH tunnel when the spec has one; `host_keys`
    /// are the SSH host keys the user trusts, and `access` says whether
    /// the session may write.
    pub async fn connect_with(
        spec: &ConnectSpec,
        secrets: &Secrets,
        host_keys: &HostKeys,
        access: Access,
    ) -> Result<Self> {
        // A file is opened here, whatever tunnel its spec still names.
        let ssh = match spec.driver {
            Driver::Sqlite => None,
            Driver::Postgres | Driver::MySql => spec.ssh.as_ref(),
        };
        let Some(ssh) = ssh else {
            return Ok(Self {
                inner: Inner::connect(spec, secrets, None, access).await?,
                access,
                tunnel: None,
            });
        };
        let tunnel = ssh::Tunnel::open(ssh, &spec.host, spec.port, secrets, host_keys).await?;
        match Inner::connect(spec, secrets, Some(tunnel.port), access).await {
            Ok(inner) => Ok(Self {
                inner,
                access,
                tunnel: Some(tunnel),
            }),
            Err(error) => Err(tunnel.forward_error().unwrap_or(error)),
        }
    }

    /// The access the session was opened with.
    pub fn access(&self) -> Access {
        self.access
    }

    #[doc(hidden)]
    pub fn tunnel_port(&self) -> Option<u16> {
        self.tunnel.as_ref().map(|tunnel| tunnel.port)
    }

    pub fn driver(&self) -> Driver {
        match &self.inner {
            Inner::Sqlite(_) => Driver::Sqlite,
            Inner::Postgres(_) => Driver::Postgres,
            Inner::MySql(_) => Driver::MySql,
        }
    }

    /// Whether the session's traffic to the database runs over TLS (never
    /// for SQLite). `prefer` falls back to plain text when the server has no
    /// TLS, so this can be false whatever the spec asked for.
    pub fn is_encrypted(&self) -> bool {
        match &self.inner {
            Inner::Sqlite(conn) => conn.is_encrypted(),
            Inner::Postgres(conn) => conn.is_encrypted(),
            Inner::MySql(conn) => conn.is_encrypted(),
        }
    }

    pub fn dialect(&self) -> Dialect {
        self.driver().dialect()
    }

    /// Runs `statements` in order in one transaction, keeping at most
    /// `limit` rows per statement. In [`ScriptMode::ReadOnly`] the
    /// transaction is read-only and always rolled back. In
    /// [`ScriptMode::Write`] it is read-write, committed when every
    /// statement succeeded and rolled back otherwise; the outcome's `end`
    /// says which. A `Write` run on a session opened
    /// [`Access::ReadOnly`] is [`Error::ReadOnly`], with nothing sent.
    ///
    /// Refuses the whole script, running nothing, when a statement could
    /// leave the transaction (see [`sql::refusal`]). `stop` ends the run
    /// between statements (and, on SQLite, inside one); the caller also
    /// fires [`CancelHandle::cancel`] for a statement already running.
    pub async fn run_script(
        &self,
        statements: &[sql::Statement],
        limit: u32,
        mode: ScriptMode,
        stop: &StopFlag,
    ) -> Result<ScriptOutcome> {
        if mode == ScriptMode::Write && self.access == Access::ReadOnly {
            return Err(Error::ReadOnly);
        }
        let dialect = self.dialect();
        for statement in statements {
            if let Some(what) = sql::refusal(dialect, &statement.text) {
                return Err(Error::Refused {
                    line: statement.first_line,
                    what,
                    mode,
                });
            }
        }
        if statements.is_empty() {
            return Ok(ScriptOutcome::default());
        }
        let texts: Vec<String> = statements.iter().map(|s| s.text.clone()).collect();
        match &self.inner {
            Inner::Sqlite(conn) => conn.run_script(texts, limit, mode, stop).await,
            Inner::Postgres(conn) => conn.run_script(texts, limit, mode, stop).await,
            Inner::MySql(conn) => conn.run_script(texts, limit, mode, stop).await,
        }
    }

    /// Runs a statement that passed [`agent::check_read`], as a script of
    /// one statement runs in [`ScriptMode::ReadOnly`]: prepared alone, in
    /// a read-only transaction that is rolled back, keeping at most
    /// `limits.rows` rows.
    ///
    /// Two things a script does not have. The values the statement was
    /// checked with are bound to its placeholders by the database's own
    /// protocol, and are never written into its text. And the database
    /// itself ends the statement after `limits.timeout`, which answers
    /// [`Error::Timeout`]; a caller keeps a timer of its own behind it, as
    /// for any script. A wait for a lock is part of that time, on all
    /// three.
    ///
    /// A statement that failed is its error, and one `stop` ended is
    /// [`Error::Cancelled`]. A `read` that was checked for another
    /// dialect than this session's is refused, with nothing sent.
    ///
    /// `sources` are the person's settings and the administrator's rules
    /// that match the connection ([`agent::Sources`]). Where any of them
    /// hides anything, the read is refused
    /// ([`Error::Agent`]) when its text names what is hidden, or when the
    /// database says it would read it: SQLite's authorizer as it prepares
    /// the statement, and the statement's plan on PostgreSQL and MySQL,
    /// asked for in the run's own transaction. Nothing hidden is read
    /// (the MCP plan's B.3).
    ///
    /// Where any of them masks a column, the read is refused when its text
    /// uses one otherwise than selecting it as it is, or reads one through
    /// a view; and a renamed or dropped one refuses every read (B.4, C.3).
    /// Each engine stops what the reading of the text misses: SQLite reads
    /// such a column as `NULL`, and PostgreSQL's plan is refused where it
    /// uses one. A column of the answer that is a masked column's, as the
    /// database says where it comes from, answers [`agent::MASKED`] in
    /// every row, and is marked so ([`agent::Answer::masked`]).
    ///
    /// The future must be awaited to its end, as [`Self::run_script`]'s.
    pub async fn run_agent_read(
        &self,
        read: &agent::Read,
        limits: agent::Limits,
        sources: &agent::Sources,
        stop: &StopFlag,
    ) -> Result<agent::Answer> {
        if read.dialect() != self.dialect() {
            return Err(Error::query(
                "the statement was checked for another kind of database",
            ));
        }
        let (outcome, masked) = match &self.inner {
            Inner::Sqlite(conn) => conn.run_read(read, limits, sources, stop).await,
            Inner::Postgres(conn) => conn.run_read(read, limits, sources, stop).await,
            Inner::MySql(conn) => conn.run_read(read, limits, sources, stop).await,
        }?;
        agent::answer(outcome, stop).map(|answer| agent::masked(answer, masked))
    }

    /// What `sources` hide and mask on this connection's database as it
    /// is now: for what the server tells an agent of the schema, which
    /// leaves the hidden out. The catalog is read only where something is
    /// hidden or masked.
    pub async fn agent_hidden(&self, sources: &agent::Sources) -> Result<agent::Hidden> {
        let catalog = match sources.guards_anything() {
            true => {
                let masked = sources.masked();
                match &self.inner {
                    Inner::Sqlite(conn) => conn.agents_catalog(&masked).await?,
                    Inner::Postgres(conn) => conn.agents_catalog(&masked).await?,
                    Inner::MySql(conn) => conn.agents_catalog(&masked).await?,
                }
            }
            false => agent::Catalog::default(),
        };
        Ok(agent::Hidden::of(self.dialect(), &catalog, sources))
    }

    /// The server's name and version for the footer: `PostgreSQL 17.2`,
    /// `MySQL 8.4.3`, `MariaDB 10.11.6`, `SQLite 3.46.0`.
    pub async fn server_version(&self) -> Result<String> {
        match &self.inner {
            Inner::Sqlite(conn) => conn.server_version().await,
            Inner::Postgres(conn) => conn.server_version().await,
            Inner::MySql(conn) => conn.server_version().await,
        }
    }

    /// The zone the server moves a moment it keeps without an offset to
    /// and from, as the server names it (`UTC`, `+00:00`, `Europe/Berlin`,
    /// or the system's abbreviation, `CET`): MySQL's `TIMESTAMP`. `None`
    /// for an engine that keeps no such moment.
    pub async fn session_zone(&self) -> Result<Option<String>> {
        match &self.inner {
            Inner::Sqlite(conn) => conn.session_zone().await,
            Inner::Postgres(conn) => conn.session_zone().await,
            Inner::MySql(conn) => conn.session_zone().await,
        }
    }

    /// Databases the session can switch to. Empty when switching does not
    /// apply (SQLite, and MySQL where databases are listed as schemas).
    pub async fn list_databases(&self) -> Result<Vec<String>> {
        match &self.inner {
            Inner::Sqlite(conn) => conn.list_databases().await,
            Inner::Postgres(conn) => conn.list_databases().await,
            Inner::MySql(conn) => conn.list_databases().await,
        }
    }

    /// At most `MAX_LISTED` schemas.
    pub async fn list_schemas(&self) -> Result<Vec<String>> {
        match &self.inner {
            Inner::Sqlite(conn) => conn.list_schemas().await,
            Inner::Postgres(conn) => conn.list_schemas().await,
            Inner::MySql(conn) => conn.list_schemas().await,
        }
    }

    /// At most `MAX_LISTED` tables and views, by name.
    pub async fn list_objects(&self, schema: &str) -> Result<Vec<ObjectInfo>> {
        match &self.inner {
            Inner::Sqlite(conn) => conn.list_objects(schema).await,
            Inner::Postgres(conn) => conn.list_objects(schema).await,
            Inner::MySql(conn) => conn.list_objects(schema).await,
        }
    }

    pub async fn describe(&self, object: &ObjectRef) -> Result<Structure> {
        match &self.inner {
            Inner::Sqlite(conn) => conn.describe(object).await,
            Inner::Postgres(conn) => conn.describe(object).await,
            Inner::MySql(conn) => conn.describe(object).await,
        }
    }

    /// One page of up to `query.limit` rows (plus one to learn whether there
    /// are more). Each cell comes back whole, however large, and PostgreSQL's
    /// simple-query protocol buffers the whole page before the first row is
    /// read; the page size (at most 10,000 in the app) is the only bound.
    pub async fn fetch_rows(&self, query: &RowQuery) -> Result<RowPage> {
        match &self.inner {
            Inner::Sqlite(conn) => conn.fetch_rows(query).await,
            Inner::Postgres(conn) => conn.fetch_rows(query).await,
            Inner::MySql(conn) => conn.fetch_rows(query).await,
        }
    }

    pub async fn count_rows(&self, query: &RowQuery) -> Result<u64> {
        match &self.inner {
            Inner::Sqlite(conn) => conn.count_rows(query).await,
            Inner::Postgres(conn) => conn.count_rows(query).await,
            Inner::MySql(conn) => conn.count_rows(query).await,
        }
    }

    /// Rows of the table a foreign key points at: the one a key names, or
    /// the ones a text is found in (see [`Lookup`]). It reads as a page of
    /// rows does and cannot write: in a read-only transaction on
    /// PostgreSQL and MySQL, under `query_only` on SQLite. What was typed
    /// is a bound value or an escaped literal, never SQL of its own.
    pub async fn lookup(&self, lookup: &Lookup) -> Result<LookupPage> {
        match &self.inner {
            Inner::Sqlite(conn) => conn.lookup(lookup).await,
            Inner::Postgres(conn) => conn.lookup(lookup).await,
            Inner::MySql(conn) => conn.lookup(lookup).await,
        }
    }

    /// Writes `changes` in one transaction, or nothing: the crate's only
    /// writing call. Each row is found by its key, locked, and compared
    /// with what the page loaded in the columns the save changes; a row
    /// that differs or is gone makes the whole save a conflict. Two changes
    /// that read the same row, however each spells its key, are an error:
    /// the database decides which row a key finds.
    ///
    /// The set's new rows are written first, each by one `INSERT`, then
    /// its changed rows, then its rows to delete, each by one `DELETE`.
    /// A new row comes back as the database stored it,
    /// or as `None` where that is not known for sure, which is more often
    /// than on a table with a trigger: [`WriteOutcome::Written`] lists
    /// every case. A caller reads the table again for such a row, and
    /// takes no new row for known because its table has no trigger.
    ///
    /// A row to delete is found by its key and locked with the changed
    /// rows, before anything is written, and no other column of it is
    /// compared. One that is not there is no failure and no conflict:
    /// the rest of the save is written, and [`WriteOutcome::Written`]
    /// says which rows were already gone. One the database refuses to
    /// delete, a row a foreign key holds most often, undoes the save
    /// ([`WriteOutcome::FailedDelete`]).
    ///
    /// On a read-only connection it is refused before the set is even
    /// looked at. The future must be awaited to its end and never dropped:
    /// a save dropped mid-way would leave its transaction open on the
    /// session. The backend awaits every command to its end.
    ///
    /// `stop` ends the save between two of its statements, where a cancel
    /// finds nothing to stop: it is asked before every statement and once
    /// more before `COMMIT`, and a save it ends answers
    /// [`Error::Cancelled`] with nothing written. After that nothing is
    /// asked, and the save is written. The caller also fires
    /// [`CancelHandle::cancel`] for a statement already running.
    pub async fn write(&self, changes: &ChangeSet, stop: &StopFlag) -> Result<WriteOutcome> {
        if self.access == Access::ReadOnly {
            return Err(Error::ReadOnly);
        }
        changes.check()?;
        match &self.inner {
            Inner::Sqlite(conn) => conn.write(changes, stop).await,
            Inner::Postgres(conn) => conn.write(changes, stop).await,
            Inner::MySql(conn) => conn.write(changes, stop).await,
        }
    }

    /// A handle that cancels whatever this session is running, from any task.
    pub fn cancel_handle(&self) -> CancelHandle {
        match &self.inner {
            Inner::Sqlite(conn) => conn.cancel_handle(),
            Inner::Postgres(conn) => conn.cancel_handle(),
            Inner::MySql(conn) => conn.cancel_handle(),
        }
    }

    pub async fn close(self) -> Result<()> {
        let Self { inner, tunnel, .. } = self;
        let closed = match inner {
            Inner::Sqlite(conn) => {
                drop(conn);
                Ok(())
            }
            Inner::Postgres(conn) => {
                drop(conn);
                Ok(())
            }
            Inner::MySql(conn) => {
                drop(conn);
                Ok(())
            }
        };
        // The driver has closed; now its tunnel.
        drop(tunnel);
        closed
    }
}

/// Cancels the query a session is running. Cancelling when nothing runs
/// does nothing.
#[derive(Clone)]
pub struct CancelHandle(CancelInner);

#[derive(Clone)]
enum CancelInner {
    Sqlite(Arc<rusqlite::InterruptHandle>),
    Postgres {
        token: tokio_postgres::CancelToken,
        tls: tokio_postgres_rustls::MakeRustlsConnect,
    },
    MySql {
        opts: mysql_async::Opts,
        id: u32,
        /// The server that thread id belongs to.
        server: String,
    },
}

impl CancelHandle {
    pub async fn cancel(&self) -> Result<()> {
        match &self.0 {
            CancelInner::Sqlite(handle) => {
                handle.interrupt();
                Ok(())
            }
            CancelInner::MySql { opts, id, server } => mysql::cancel(opts, *id, server).await,
            CancelInner::Postgres { token, tls } => token
                .cancel_query(tls.clone())
                .await
                .map_err(|error| Error::Connect(format!("could not cancel: {error}"))),
        }
    }
}

impl fmt::Debug for CancelHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("CancelHandle")
    }
}
