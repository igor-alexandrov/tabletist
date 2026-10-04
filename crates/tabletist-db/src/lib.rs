//! Access to PostgreSQL, MySQL and SQLite for Tabletist.
//!
//! Nothing in this crate writes to a connected database yet. A session is
//! read-only unless it is opened [`Access::Writable`], and then it is
//! fenced: row fetches and counts run in read-only transactions (on SQLite
//! under `query_only`), and a SQL editor script still cannot write.

mod catalog;
mod check;
mod class;
pub mod complete;
mod dialect;
mod error;
pub mod fixtures;
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

pub use catalog::{
    ColumnInfo, ForeignKeyInfo, IndexInfo, MAX_LISTED, ObjectInfo, ObjectKind, ObjectRef, Structure,
};
pub use class::{ColumnClass, column_class};
pub use dialect::{Dialect, RowUpdate, Sql, escape_like, quote_literal};
pub use error::{Error, Result, SshStage};
pub use query::{Filter, FilterOp, RowPage, RowQuery, Sort, SortDir};
pub use script::{ScriptOutcome, StatementOutcome, StatementResult, StopFlag};
pub use spec::{ConnectSpec, Driver, ParsedUrl, Secrets, SshAuth, SshSpec, TlsMode};
pub use ssh::HostKeys;
pub use value::{ColumnMeta, Value, ValueKind, value_from_pg_text};
pub use write::{CellChange, ChangeSet, Conflict, NewValue, RowChange, WriteOutcome};

/// Whether a session may write. The app has no writing call yet; a
/// writable session is the one a later `write` will be allowed on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Access {
    /// Nothing can write: the session itself is read-only.
    #[default]
    ReadOnly,
    /// The session is read-write. Browsing still reads in read-only
    /// transactions, and a script still cannot write.
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
        let ssh = spec.ssh.as_ref().filter(|_| spec.driver != Driver::Sqlite);
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
            Inner::Sqlite(_) => false,
            Inner::Postgres(conn) => conn.encrypted,
            Inner::MySql(conn) => conn.encrypted,
        }
    }

    pub fn dialect(&self) -> Dialect {
        self.driver().dialect()
    }

    /// Runs `statements` in order in one read-only transaction that is
    /// always rolled back, keeping at most `limit` rows per statement.
    /// Refuses the whole script, running nothing, when a statement could
    /// leave the transaction (see [`sql::refusal`]). `stop` ends the run
    /// between statements (and, on SQLite, inside one); the caller also
    /// fires [`CancelHandle::cancel`] for a statement already running.
    pub async fn run_script(
        &self,
        statements: &[sql::Statement],
        limit: u32,
        stop: &StopFlag,
    ) -> Result<ScriptOutcome> {
        let dialect = self.dialect();
        for statement in statements {
            if let Some(what) = sql::refusal(dialect, &statement.text) {
                return Err(Error::Refused {
                    line: statement.first_line,
                    what,
                });
            }
        }
        if statements.is_empty() {
            return Ok(ScriptOutcome::default());
        }
        let texts: Vec<String> = statements.iter().map(|s| s.text.clone()).collect();
        match &self.inner {
            Inner::Sqlite(conn) => conn.run_script(texts, limit, stop).await,
            Inner::Postgres(conn) => conn.run_script(&texts, limit, stop).await,
            Inner::MySql(conn) => conn.run_script(&texts, limit, stop).await,
        }
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

    /// Databases the session can switch to. Empty when switching does not
    /// apply (SQLite, and MySQL where databases are listed as schemas).
    pub async fn list_databases(&self) -> Result<Vec<String>> {
        match &self.inner {
            Inner::Sqlite(_) => Ok(Vec::new()),
            Inner::Postgres(conn) => conn.list_databases().await,
            Inner::MySql(_) => Ok(Vec::new()),
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

    /// Writes `changes` in one transaction, or nothing: the crate's only
    /// writing call. Each row is found by its key, locked, and compared
    /// with what the page loaded in the columns the save changes; a row
    /// that differs or is gone makes the whole save a conflict.
    ///
    /// On a read-only connection it is refused before the set is even
    /// looked at. The future must be awaited to its end and never dropped:
    /// a save dropped mid-way would leave its transaction open on the
    /// session. The backend awaits every command to its end, and stops a
    /// save through the session's cancel.
    pub async fn write(&self, changes: &ChangeSet) -> Result<WriteOutcome> {
        if self.access == Access::ReadOnly {
            return Err(Error::ReadOnly);
        }
        changes.check()?;
        match &self.inner {
            Inner::Sqlite(_) | Inner::Postgres(_) | Inner::MySql(_) => Err(Error::Unsupported(
                "saving is not built for this database yet",
            )),
        }
    }

    /// A handle that cancels whatever this session is running, from any task.
    pub fn cancel_handle(&self) -> CancelHandle {
        match &self.inner {
            Inner::Sqlite(conn) => CancelHandle(CancelInner::Sqlite(conn.interrupt_handle())),
            Inner::MySql(conn) => CancelHandle(CancelInner::MySql {
                opts: conn.opts.clone(),
                id: conn.id,
                server: conn.server.clone(),
            }),
            Inner::Postgres(conn) => CancelHandle(CancelInner::Postgres {
                token: conn.cancel.clone(),
                tls: conn.tls.clone(),
            }),
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
