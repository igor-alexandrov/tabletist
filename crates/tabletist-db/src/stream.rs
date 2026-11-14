//! An export's rows, streamed from the database in batches: never the whole
//! result in memory at once (see [`crate::Connection::stream_rows`]).

use tokio::sync::mpsc::Sender;

use crate::{ColumnMeta, Error, Result, Value};

/// What a stream sends, in order: the columns once, then the rows in
/// batches, none empty.
#[derive(Debug, Clone, PartialEq)]
pub enum Streamed {
    Columns(Vec<ColumnMeta>),
    Rows(Vec<Vec<Value>>),
}

/// Sends `streamed`, waiting while the receiver still holds what it was
/// sent before. A receiver that is gone stopped the export: the stream
/// ends as cancelled.
pub(crate) async fn send(to: &Sender<Streamed>, streamed: Streamed) -> Result<()> {
    to.send(streamed).await.map_err(|_| Error::Cancelled)
}

/// [`send`] from a blocking job (SQLite's).
pub(crate) fn send_blocking(to: &Sender<Streamed>, streamed: Streamed) -> Result<()> {
    to.blocking_send(streamed).map_err(|_| Error::Cancelled)
}
