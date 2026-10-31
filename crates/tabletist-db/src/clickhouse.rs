//! ClickHouse over its HTTP interface. Not an engine yet: no `Driver`
//! names it (see
//! `docs/superpowers/specs/2026-10-31-clickhouse-read-only-engine-design.md`,
//! whose phase 2 makes it one).
//!
//! Each statement is one POST (`http`). Rows come back as
//! `TabSeparatedWithNamesAndTypes`, read a line at a time (`read`), so a
//! page stops reading where it is full. Every statement carries its own
//! `query_id`, which is how [`Canceller`] stops it: `KILL QUERY` on another
//! connection, for a dropped connection does not stop a query.

mod http;
mod read;

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Instant;

use tokio::io::{AsyncReadExt, BufReader};

use http::Http;
pub use http::{Endpoint, Login};
use read::read_tsv;

use crate::{Error, Result, RowPage, Value};

/// A session with one server: one statement at a time, as a PostgreSQL or
/// MySQL session runs them.
pub struct Session {
    http: Arc<Http>,
    /// Held while a statement runs, so `running` names the one there is.
    one_at_a_time: tokio::sync::Mutex<()>,
    /// The `query_id` of the statement running now, which [`Canceller`]
    /// kills.
    running: Arc<Mutex<Option<String>>>,
}

/// Stops the statement a [`Session`] runs. Doing so when nothing runs does
/// nothing.
#[derive(Clone)]
pub struct Canceller {
    http: Arc<Http>,
    running: Arc<Mutex<Option<String>>>,
}

impl Session {
    pub async fn connect(endpoint: Endpoint, login: Login) -> Result<Self> {
        Ok(Self {
            http: Arc::new(Http::connect(endpoint, login).await?),
            one_at_a_time: tokio::sync::Mutex::new(()),
            running: Arc::default(),
        })
    }

    /// Whether the session's requests go over TLS. `prefer` falls back to
    /// plain HTTP where the server has no TLS on the port.
    pub fn is_encrypted(&self) -> bool {
        self.http.is_encrypted()
    }

    pub fn canceller(&self) -> Canceller {
        Canceller {
            http: Arc::clone(&self.http),
            running: Arc::clone(&self.running),
        }
    }

    pub async fn server_version(&self) -> Result<String> {
        let page = self.select_page("SELECT version()", 1).await?;
        match page.rows.first().and_then(|row| row.first()) {
            Some(Value::Text(version)) => Ok(version.to_string()),
            _ => Err(Error::query("the server did not say its version")),
        }
    }

    /// Runs `sql`, which must not name a `FORMAT`, under the server's
    /// `readonly = 2` (no DDL, settings allowed), and keeps its first
    /// `limit` rows. That is no fence on its own: `INSERT INTO FUNCTION`,
    /// `BACKUP` and `KILL` still run, so `sql` must be a read.
    pub async fn select_page(&self, sql: &str, limit: u32) -> Result<RowPage> {
        let _turn = self.one_at_a_time.lock().await;
        let started = Instant::now();
        let id = query_id();
        *self.running.lock().unwrap_or_else(PoisonError::into_inner) = Some(id.clone());
        let read = self.read_rows(sql, limit, &id).await;
        *self.running.lock().unwrap_or_else(PoisonError::into_inner) = None;
        let (columns, mut rows) = read?;
        let has_more = rows.len() > limit as usize;
        rows.truncate(limit as usize);
        Ok(RowPage {
            columns,
            rows,
            has_more,
            ordered_by_key: false,
            elapsed: started.elapsed(),
        })
    }

    async fn read_rows(
        &self,
        sql: &str,
        limit: u32,
        id: &str,
    ) -> Result<(Vec<crate::ColumnMeta>, read::Rows)> {
        // One row past the page says whether another page exists. The
        // server stops at about that many too, a block past it at most.
        let wanted = u64::from(limit) + 1;
        let wanted_text = wanted.to_string();
        let answer = self
            .http
            .post(
                sql,
                &[
                    ("query_id", id),
                    ("readonly", "2"),
                    ("default_format", "TabSeparatedWithNamesAndTypes"),
                    ("max_result_rows", &wanted_text),
                    ("result_overflow_mode", "break"),
                ],
            )
            .await?;
        let tag = answer.header("x-clickhouse-exception-tag");
        read_tsv(BufReader::new(answer.body), wanted, tag.as_deref()).await
    }
}

impl Canceller {
    pub async fn cancel(&self) -> Result<()> {
        let Some(id) = self
            .running
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
        else {
            return Ok(());
        };
        // `id` is ours (`query_id`): letters, digits and dashes only.
        let sql = format!("KILL QUERY WHERE query_id = '{id}' ASYNC FORMAT Null");
        let mut answer = self.http.post(&sql, &[]).await?;
        let mut drained = Vec::new();
        answer
            .body
            .read_to_end(&mut drained)
            .await
            .map_err(|error| Error::Connect(format!("could not cancel: {error}")))?;
        Ok(())
    }
}

/// A `query_id` no other statement of this process has.
fn query_id() -> String {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    format!(
        "tabletist-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    )
}
