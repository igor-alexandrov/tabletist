//! Access to PostgreSQL, MySQL and SQLite for Tabletist.
//!
//! Nothing in this crate writes to a connected database: every session is
//! opened read-only.

mod catalog;
mod dialect;
mod error;
pub mod fixtures;
mod mysql;
mod pg;
mod query;
mod spec;
// Used by pg.rs from the next task.
mod sqlite;
mod ssh;
#[allow(dead_code)]
mod tls;
mod value;

use std::fmt;
use std::sync::Arc;

pub use catalog::{
    ColumnInfo, ForeignKeyInfo, IndexInfo, ObjectInfo, ObjectKind, ObjectRef, Structure,
};
pub use dialect::{Dialect, Sql, escape_like, quote_literal};
pub use error::{Error, Result, SshStage};
pub use query::{Filter, FilterOp, RowPage, RowQuery, Sort, SortDir};
pub use spec::{ConnectSpec, Driver, Secrets, SshAuth, SshSpec, TlsMode};
pub use ssh::HostKeys;
pub use value::{ColumnMeta, Value, ValueKind, value_from_pg_text};

/// An open, read-only database session. Batches 4 and 5 add PostgreSQL and
/// MySQL variants.
/// A database session, and the SSH tunnel it runs through, if any.
pub struct Connection {
    inner: Inner,
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
    async fn connect(spec: &ConnectSpec, secrets: &Secrets, via: Option<u16>) -> Result<Self> {
        match spec.driver {
            Driver::Sqlite => {
                let path = spec
                    .sqlite_path
                    .as_ref()
                    .ok_or_else(|| Error::InvalidSpec("choose a SQLite file".into()))?;
                Ok(Self::Sqlite(sqlite::Conn::open(path).await?))
            }
            Driver::Postgres => Ok(Self::Postgres(Box::new(
                pg::Conn::connect(spec, secrets, via).await?,
            ))),
            Driver::MySql => Ok(Self::MySql(Box::new(
                mysql::Conn::connect(spec, secrets, via).await?,
            ))),
        }
    }
}

impl Connection {
    pub async fn connect(spec: &ConnectSpec, secrets: &Secrets) -> Result<Self> {
        Self::connect_with(spec, secrets, &HostKeys::default()).await
    }

    /// Connects, through an SSH tunnel when the spec has one; `host_keys`
    /// are the SSH host keys the user trusts.
    pub async fn connect_with(
        spec: &ConnectSpec,
        secrets: &Secrets,
        host_keys: &HostKeys,
    ) -> Result<Self> {
        let ssh = spec.ssh.as_ref().filter(|_| spec.driver != Driver::Sqlite);
        let Some(ssh) = ssh else {
            return Ok(Self {
                inner: Inner::connect(spec, secrets, None).await?,
                tunnel: None,
            });
        };
        let tunnel = ssh::Tunnel::open(ssh, &spec.host, spec.port, secrets, host_keys).await?;
        match Inner::connect(spec, secrets, Some(tunnel.port)).await {
            Ok(inner) => Ok(Self {
                inner,
                tunnel: Some(tunnel),
            }),
            Err(error) => Err(tunnel.forward_error().unwrap_or(error)),
        }
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

    pub fn dialect(&self) -> Dialect {
        match &self.inner {
            Inner::Sqlite(_) => Dialect::Sqlite,
            Inner::Postgres(_) => Dialect::Postgres,
            Inner::MySql(_) => Dialect::MySql,
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

    pub async fn list_schemas(&self) -> Result<Vec<String>> {
        match &self.inner {
            Inner::Sqlite(conn) => conn.list_schemas().await,
            Inner::Postgres(conn) => conn.list_schemas().await,
            Inner::MySql(conn) => conn.list_schemas().await,
        }
    }

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

    /// A handle that cancels whatever this session is running, from any task.
    pub fn cancel_handle(&self) -> CancelHandle {
        match &self.inner {
            Inner::Sqlite(conn) => CancelHandle(CancelInner::Sqlite(conn.interrupt_handle())),
            Inner::MySql(conn) => CancelHandle(CancelInner::MySql {
                opts: conn.opts.clone(),
                id: conn.id,
            }),
            Inner::Postgres(conn) => CancelHandle(CancelInner::Postgres {
                token: conn.cancel.clone(),
                tls: conn.tls.clone(),
            }),
        }
    }

    pub async fn close(self) -> Result<()> {
        let Self { inner, tunnel } = self;
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
    },
}

impl CancelHandle {
    pub async fn cancel(&self) -> Result<()> {
        match &self.0 {
            CancelInner::Sqlite(handle) => {
                handle.interrupt();
                Ok(())
            }
            CancelInner::MySql { opts, id } => {
                use mysql_async::prelude::Queryable;
                let mut conn = mysql_async::Conn::new(opts.clone())
                    .await
                    .map_err(|error| Error::Connect(format!("could not cancel: {error}")))?;
                let result = conn.query_drop(format!("KILL QUERY {id}")).await;
                let _ = conn.disconnect().await;
                result.map_err(|error| Error::Connect(format!("could not cancel: {error}")))
            }
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
