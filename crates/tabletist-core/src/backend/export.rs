//! An export (spec `2026-11-03-export-copy-as-3-table-design.md`): a
//! table's rows, read on a read-only connection of its own and streamed
//! through a format's writer into a hidden temporary file beside the
//! target, renamed into place once whole; or, for the clipboard, kept as
//! text within the clipboard's limits. In the background: the tab and the
//! app stay usable, and a cancel or a failure leaves nothing behind.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tabletist_db::{
    CancelHandle, ColumnMeta, ConnectSpec, Connection, Dialect, Error, HostKeys, ObjectRef,
    RowQuery, Secrets, StopFlag, Streamed, Value,
};

use crate::export::{self, Columns, Copied, Format, Options, Target, Writer};

/// One export, from the sheet that asked for it to its end.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExportId(pub u64);

/// How often a running export says how far it is, at most.
pub const PROGRESS_EVERY: Duration = Duration::from_millis(250);

/// The connection an export opens for itself: the tab's details and the
/// secrets the tab connected with. `Secrets` prints as `Secrets { .. }`.
#[derive(Debug, Clone)]
pub struct Opener {
    pub spec: ConnectSpec,
    pub secrets: Secrets,
    pub host_keys: HostKeys,
}

/// Where the rows come from.
#[derive(Debug, Clone)]
pub enum ExportRows {
    /// Read from the database: every row of the table its filters keep,
    /// in its order (a whole table's query has none of either).
    Query(RowQuery),
    /// The rows the tab has loaded, already in the chosen columns' order
    /// (spec, D7): nothing runs for them.
    Loaded {
        columns: Vec<ColumnMeta>,
        rows: Vec<Vec<Value>>,
    },
}

/// Where the text goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Destination {
    /// A file. `replace` lets the last step, the rename, put it over a
    /// file of that name; otherwise one that is there by then fails the
    /// export as [`ExportFailure::Exists`].
    File { path: PathBuf, replace: bool },
    /// The clipboard, within [`export::CLIPBOARD_ROWS`] and
    /// [`export::CLIPBOARD_BYTES`].
    Clipboard,
}

/// What an export does, whole: what the app sends, and what Try again
/// sends again.
#[derive(Debug, Clone)]
pub struct ExportJob {
    pub id: ExportId,
    pub open: Opener,
    pub rows: ExportRows,
    /// The columns written, in their order. A query reads only these.
    pub columns: Vec<String>,
    /// Which of `columns` are written as `•••` (see [`export::masks`]).
    pub masked: Vec<bool>,
    pub format: Format,
    pub options: Options,
    /// The table the rows are of: SQL INSERT's target, and the `CREATE
    /// TABLE` it starts with where its options ask (read on the export's
    /// connection).
    pub object: ObjectRef,
    pub dialect: Dialect,
    pub destination: Destination,
}

/// Why an export did not finish.
#[derive(Debug, Clone, PartialEq)]
pub enum ExportFailure {
    /// The folder (or the file) may not be written.
    PermissionDenied { folder: PathBuf },
    /// The disk filled up after `written` bytes.
    DiskFull { written: u64 },
    /// The connection went after `rows` rows.
    ConnectionLost { rows: u64 },
    /// The server's own timeout ended the statement after `after`.
    Timeout { after: Duration },
    /// A file of the name was made while the export ran, and replacing it
    /// was not asked.
    Exists { path: PathBuf },
    /// For the clipboard: more rows than it takes.
    TooManyRows,
    /// For the clipboard: more text than it takes.
    TooLarge,
    /// Anything else: its first line.
    Other(String),
}

/// How a running export stands, as it tells the app.
#[derive(Debug, Clone, PartialEq)]
pub enum ExportState {
    /// It has begun (its connection is open, or its rows are at hand),
    /// with the statement it runs as a person reads it: `None` for loaded
    /// rows, or where the statement could not be written out.
    Started { statement: Option<String> },
    /// Rows written so far, and the bytes they took.
    Progress { rows: u64, bytes: u64 },
    /// All written: `copied` holds the text for the clipboard.
    Done {
        rows: u64,
        bytes: u64,
        took: Duration,
        copied: Option<String>,
        statements: usize,
    },
    /// Stopped by the person: nothing was saved.
    Cancelled,
    /// Ended by `ExportFailure`: nothing was saved.
    Failed(ExportFailure),
}

impl ExportState {
    /// Whether the export is over.
    pub fn ended(&self) -> bool {
        matches!(self, Self::Done { .. } | Self::Cancelled | Self::Failed(_))
    }
}

/// What stops a running export: its flag, and the cancel of its
/// connection's statement once one runs.
#[derive(Clone, Default)]
pub struct Stopper {
    stop: StopFlag,
    cancel: Arc<Mutex<Option<CancelHandle>>>,
    /// The person asked: the export ends as cancelled, unless its file is
    /// in place already.
    asked: Arc<std::sync::atomic::AtomicBool>,
}

impl Stopper {
    /// Stops the export, as the person asks: between batches, and the
    /// statement running now.
    pub async fn stop(&self) {
        self.asked.store(true, std::sync::atomic::Ordering::SeqCst);
        self.halt().await;
    }

    /// Whether the person asked it to stop.
    fn asked(&self) -> bool {
        self.asked.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Stops the stream and its statement: for a cancel, or a writer that
    /// failed, whose rows nobody would read.
    async fn halt(&self) {
        self.stop.stop();
        let cancel = self
            .cancel
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        if let Some(cancel) = cancel
            && !self.stop.is_finishing()
            && let Err(error) = cancel.cancel().await
        {
            log::warn!("could not cancel an export's statement: {error}");
        }
    }
}

/// The hidden temporary file an export writes beside `target`:
/// `.reviews.csv.` then six random letters, then `.tabletist-part` (spec,
/// D8). Made new, so nothing that is there is ever followed or reused;
/// private to the user on Unix (0600), and on Windows with the folder's
/// own ACL.
pub fn part_beside(target: &Path) -> std::io::Result<tempfile::NamedTempFile> {
    let name = target
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    tempfile::Builder::new()
        .prefix(&format!(".{name}."))
        .suffix(".tabletist-part")
        .rand_bytes(6)
        .tempfile_in(folder_of(target))
}

/// The folder `path` is in: `.` for a bare name.
fn folder_of(path: &Path) -> &Path {
    match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    }
}

/// Runs `job` to its end, telling `report` each state as it goes, and
/// returns the last. Every path out of it leaves no temporary file.
pub async fn run(job: ExportJob, stopper: Stopper, report: impl Fn(ExportState)) -> ExportState {
    let started = Instant::now();
    let reading = match &job.rows {
        ExportRows::Loaded { .. } if !creates(&job) => None,
        _ => match Connection::connect_with(
            &job.open.spec,
            &job.open.secrets,
            &job.open.host_keys,
            tabletist_db::Access::ReadOnly,
        )
        .await
        {
            Ok(connection) => Some(connection),
            Err(error) => {
                return failed(&report, ExportFailure::Other(first_line(&error)));
            }
        },
    };
    if let Some(connection) = &reading {
        *stopper
            .cancel
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(connection.cancel_handle());
    }
    if stopper.stop.is_stopped() {
        return ended(&report, ExportState::Cancelled, reading).await;
    }
    let create = match (&reading, creates(&job)) {
        (Some(connection), true) => match connection.ddl(&job.object).await {
            Ok(ddl) => ddl.part(tabletist_db::ddl::PartKind::Object).map(|part| {
                part.statements
                    .iter()
                    .map(|statement| statement.trim_end().trim_end_matches(';').to_owned() + ";")
                    .collect::<Vec<_>>()
                    .join("\n\n")
            }),
            Err(error) => {
                let state = state_of_error(error, 0, started, &stopper);
                return ended(&report, state, reading).await;
            }
        },
        _ => None,
    };
    let statement = match (&job.rows, &reading) {
        (ExportRows::Query(query), Some(connection)) => {
            connection.export_statement(query, &job.columns).await.ok()
        }
        _ => None,
    };
    report(ExportState::Started { statement });
    let (to, from) = tokio::sync::mpsc::channel(1);
    let producing = produce(&job, reading.as_ref(), &stopper.stop, to);
    let writing = async {
        let written = write(&job, create.as_deref(), from, &report).await;
        // A writer that failed stops the statement at once: on MySQL the
        // rows it would still send are otherwise read to the end.
        if written.is_err() {
            stopper.halt().await;
        }
        written
    };
    let (produced, written) = tokio::join!(producing, writing);
    let state = match (produced, written) {
        // The writer's own failure wins: it is why the stream stopped.
        (_, Err(failure)) => ExportState::Failed(failure),
        (Err(error), Ok(written)) => state_of_error(error, written.rows, started, &stopper),
        // A cancel that comes after the last batch still keeps the file
        // out, up to its rename; after that the file is the export's.
        (Ok(_), Ok(written)) => match written.finish(&|| stopper.asked()) {
            Ok(done) => ExportState::Done {
                rows: done.rows,
                bytes: done.bytes,
                took: started.elapsed(),
                copied: done.copied,
                statements: done.statements,
            },
            Err(Some(failure)) => ExportState::Failed(failure),
            Err(None) => ExportState::Cancelled,
        },
    };
    let state = match state {
        ExportState::Failed(_) if stopper.asked() => ExportState::Cancelled,
        state => state,
    };
    ended(&report, state, reading).await
}

/// Whether SQL INSERT starts with the table's `CREATE TABLE`.
fn creates(job: &ExportJob) -> bool {
    job.format == Format::SqlInsert && job.options.sql.create_table
}

fn failed(report: &impl Fn(ExportState), failure: ExportFailure) -> ExportState {
    let state = ExportState::Failed(failure);
    report(state.clone());
    state
}

/// Tells `state`, and closes the connection: a stream left half read
/// ends with it.
async fn ended(
    report: &impl Fn(ExportState),
    state: ExportState,
    connection: Option<Connection>,
) -> ExportState {
    if let Some(connection) = connection {
        match state {
            ExportState::Done { .. } => {
                if let Err(error) = connection.close().await {
                    log::warn!("an export's connection did not close cleanly: {error}");
                }
            }
            // A statement may still be sending rows: it is stopped first,
            // for the rest are never read (a goodbye, or a dropped MySQL
            // session, would read them all).
            _ => {
                if let Err(error) = connection.cancel_handle().cancel().await {
                    log::warn!("could not stop an export's statement: {error}");
                }
                drop(connection);
            }
        }
    }
    report(state.clone());
    state
}

fn state_of_error(error: Error, rows: u64, started: Instant, stopper: &Stopper) -> ExportState {
    if stopper.asked() {
        return ExportState::Cancelled;
    }
    ExportState::Failed(match error {
        Error::Cancelled => return ExportState::Cancelled,
        Error::ConnectionLost(_) => ExportFailure::ConnectionLost { rows },
        Error::Timeout => ExportFailure::Timeout {
            after: started.elapsed(),
        },
        error => ExportFailure::Other(first_line(&error)),
    })
}

fn first_line(error: &impl std::fmt::Display) -> String {
    crate::history::first_line(&error.to_string())
}

/// Sends the rows: streamed from the database, or the loaded ones in
/// batches. Returns the rows sent.
async fn produce(
    job: &ExportJob,
    connection: Option<&Connection>,
    stop: &StopFlag,
    to: tokio::sync::mpsc::Sender<Streamed>,
) -> Result<u64, Error> {
    match (&job.rows, connection) {
        (ExportRows::Query(query), Some(connection)) => {
            connection
                .stream_rows(query, &job.columns, export::BATCH, &to, stop)
                .await
        }
        (ExportRows::Query(_), None) => Err(Error::ConnectionLost("not connected".into())),
        (ExportRows::Loaded { columns, rows }, _) => {
            let gone = |_| Error::Cancelled;
            to.send(Streamed::Columns(columns.clone()))
                .await
                .map_err(gone)?;
            for batch in rows.chunks(export::BATCH) {
                if stop.is_stopped() {
                    return Err(Error::Cancelled);
                }
                to.send(Streamed::Rows(batch.to_vec()))
                    .await
                    .map_err(gone)?;
            }
            Ok(rows.len() as u64)
        }
    }
}

/// Where the text goes while it is written.
enum Sink {
    /// The temporary file, and the bytes written into it.
    File {
        part: tempfile::NamedTempFile,
        target: PathBuf,
        replace: bool,
        bytes: u64,
        /// A test's disk that fills up after this many bytes.
        #[cfg(test)]
        full_after: Option<u64>,
    },
    /// The rows for the clipboard, written out once all are in.
    Clipboard { rows: Vec<Vec<Value>> },
}

/// What a writer did, until it is finished.
struct Written<'a> {
    job: &'a ExportJob,
    create: Option<&'a str>,
    columns: Vec<ColumnMeta>,
    sink: Option<Sink>,
    writer_state: Option<Pending>,
    rows: u64,
}

/// A writer's text not yet in the file: a format holds its tail (and
/// Markdown its rows) until it finishes.
struct Pending {
    statements: usize,
    tail: String,
}

/// What a finished export wrote.
struct Finished {
    rows: u64,
    bytes: u64,
    copied: Option<String>,
    statements: usize,
}

impl Written<'_> {
    /// Moves the temporary file into place, or writes the clipboard's text.
    /// `cancelled` is asked just before the rename: `Err(None)` is a
    /// cancel, and the temporary file goes.
    fn finish(mut self, cancelled: &dyn Fn() -> bool) -> Result<Finished, Option<ExportFailure>> {
        let pending = self.writer_state.take();
        match self.sink.take() {
            Some(Sink::File {
                mut part,
                target,
                replace,
                mut bytes,
                #[cfg(test)]
                full_after,
            }) => {
                let pending = pending.unwrap_or(Pending {
                    statements: 0,
                    tail: String::new(),
                });
                #[cfg(not(test))]
                let full_after = None;
                put(&mut part, &pending.tail, &mut bytes, full_after, &target).map_err(Some)?;
                part.as_file()
                    .sync_all()
                    .map_err(|error| Some(io_failure(error, &target, bytes)))?;
                if cancelled() {
                    // Dropped, the temporary file goes.
                    return Err(None);
                }
                let persisted = match replace {
                    true => part.persist(&target).map(|_| ()),
                    false => part.persist_noclobber(&target).map(|_| ()),
                };
                if let Err(error) = persisted {
                    // The temporary file goes with the error that holds it.
                    return Err(Some(match error.error.kind() {
                        std::io::ErrorKind::AlreadyExists => ExportFailure::Exists { path: target },
                        _ => io_failure(error.error, &target, bytes),
                    }));
                }
                #[cfg(unix)]
                if let Ok(folder) = std::fs::File::open(folder_of(&target)) {
                    // Best effort: the file itself is in place already.
                    let _ = folder.sync_all();
                }
                Ok(Finished {
                    rows: self.rows,
                    bytes,
                    copied: None,
                    statements: pending.statements,
                })
            }
            Some(Sink::Clipboard { rows }) => {
                let names: Vec<&str> = self
                    .columns
                    .iter()
                    .map(|column| column.name.as_str())
                    .collect();
                let columns = Columns {
                    names,
                    kinds: self.columns.iter().map(|column| column.kind).collect(),
                    dialect: self.job.dialect,
                };
                let target = Target::Table {
                    object: &self.job.object,
                    create: self.create,
                };
                let count = rows.len();
                match export::clipboard(
                    self.job.format,
                    &self.job.options,
                    columns,
                    target,
                    count,
                    rows.into_iter(),
                ) {
                    Copied::Text { text, statements } => Ok(Finished {
                        rows: self.rows,
                        bytes: text.len() as u64,
                        copied: Some(text),
                        statements,
                    }),
                    Copied::TooManyRows(_) => Err(Some(ExportFailure::TooManyRows)),
                    Copied::TooLarge => Err(Some(ExportFailure::TooLarge)),
                }
            }
            None => Err(Some(ExportFailure::Other("nothing was written".into()))),
        }
    }
}

/// The bytes a value takes as text, at the least: a binary value's hex is
/// twice its length.
fn value_size(value: &Value) -> usize {
    match value {
        Value::Null => 0,
        Value::Bool(_) => 4,
        Value::Int(_) | Value::Float(_) => 8,
        Value::Text(text) => text.len(),
        Value::Bytes(bytes) => 2 * bytes.len(),
    }
}

/// Writes `text` into the temporary file, counting its bytes.
fn put(
    part: &mut tempfile::NamedTempFile,
    text: &str,
    bytes: &mut u64,
    full_after: Option<u64>,
    target: &Path,
) -> Result<(), ExportFailure> {
    if let Some(limit) = full_after
        && *bytes + text.len() as u64 > limit
    {
        let full = std::io::Error::from(std::io::ErrorKind::StorageFull);
        return Err(io_failure(full, target, *bytes));
    }
    part.write_all(text.as_bytes())
        .map_err(|error| io_failure(error, target, *bytes))?;
    *bytes += text.len() as u64;
    Ok(())
}

fn io_failure(error: std::io::Error, target: &Path, written: u64) -> ExportFailure {
    match error.kind() {
        std::io::ErrorKind::PermissionDenied | std::io::ErrorKind::ReadOnlyFilesystem => {
            ExportFailure::PermissionDenied {
                folder: folder_of(target).to_path_buf(),
            }
        }
        std::io::ErrorKind::StorageFull | std::io::ErrorKind::QuotaExceeded => {
            ExportFailure::DiskFull { written }
        }
        _ => ExportFailure::Other(first_line(&error)),
    }
}

/// Takes the stream and writes it: the file's text batch by batch, the
/// clipboard's rows until they are all in or there are too many.
async fn write<'a>(
    job: &'a ExportJob,
    create: Option<&'a str>,
    mut from: tokio::sync::mpsc::Receiver<Streamed>,
    report: &impl Fn(ExportState),
) -> Result<Written<'a>, ExportFailure> {
    let mut written = Written {
        job,
        create,
        columns: Vec::new(),
        sink: None,
        writer_state: None,
        rows: 0,
    };
    // The columns come first: none means the stream failed before any.
    let Some(Streamed::Columns(columns)) = from.recv().await else {
        return Ok(written);
    };
    // The writer borrows the names it writes: they are kept here.
    let names: Vec<String> = columns.iter().map(|column| column.name.clone()).collect();
    let kinds: Vec<_> = columns.iter().map(|column| column.kind).collect();
    written.columns = columns;
    written.sink = Some(match &job.destination {
        Destination::File { path, replace } => Sink::File {
            part: part_beside(path).map_err(|error| io_failure(error, path, 0))?,
            target: path.clone(),
            replace: *replace,
            bytes: 0,
            #[cfg(test)]
            full_after: test_full_after(path),
        },
        Destination::Clipboard => Sink::Clipboard { rows: Vec::new() },
    });
    let mut text = String::new();
    let mut writer = match written.sink {
        Some(Sink::File { .. }) => {
            let mut writer = Writer::new(
                job.format,
                &job.options,
                Columns {
                    names: names.iter().map(String::as_str).collect(),
                    kinds,
                    dialect: job.dialect,
                },
                Target::Table {
                    object: &job.object,
                    create,
                },
            );
            writer.start(&mut text);
            Some(writer)
        }
        _ => None,
    };
    let masked = |mut row: Vec<Value>| {
        for (value, masked) in row.iter_mut().zip(&job.masked) {
            if *masked && !matches!(value, Value::Null) {
                *value = Value::Text(tabletist_db::agent::MASKED.into());
            }
        }
        row
    };
    let mut said = Instant::now();
    let mut held = 0usize;
    while let Some(Streamed::Rows(rows)) = from.recv().await {
        match (written.sink.as_mut(), writer.as_mut()) {
            (
                Some(Sink::File {
                    part,
                    target,
                    bytes,
                    #[cfg(test)]
                    full_after,
                    ..
                }),
                Some(writer),
            ) => {
                for row in rows {
                    writer.row(&masked(row), &mut text);
                    written.rows += 1;
                }
                #[cfg(test)]
                let full_after = *full_after;
                #[cfg(not(test))]
                let full_after = None;
                put(part, &text, bytes, full_after, target)?;
                text.clear();
                if said.elapsed() >= PROGRESS_EVERY {
                    said = Instant::now();
                    report(ExportState::Progress {
                        rows: written.rows,
                        bytes: *bytes,
                    });
                }
            }
            (Some(Sink::Clipboard { rows: kept }), _) => {
                for row in rows {
                    // What the row's values come to, at the least: past the
                    // clipboard's bytes nothing more is read.
                    held += row.iter().map(value_size).sum::<usize>();
                    if held > export::CLIPBOARD_BYTES {
                        return Err(ExportFailure::TooLarge);
                    }
                    kept.push(masked(row));
                    written.rows += 1;
                    if kept.len() > export::CLIPBOARD_ROWS {
                        // Dropping the stream's receiver stops it.
                        return Err(ExportFailure::TooManyRows);
                    }
                }
            }
            _ => {}
        }
    }
    if let Some(mut writer) = writer {
        writer.finish(&mut text);
        written.writer_state = Some(Pending {
            statements: writer.statements(),
            tail: std::mem::take(&mut text),
        });
    }
    Ok(written)
}

/// A test's full disk: a target named `full-<bytes>.<ext>` fills up after
/// that many bytes.
#[cfg(test)]
fn test_full_after(path: &Path) -> Option<u64> {
    path.file_stem()?
        .to_str()?
        .strip_prefix("full-")?
        .parse()
        .ok()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::sync::Mutex as StdMutex;

    /// The Bookshop in a new SQLite file, with `copies` rows of
    /// `shelf_copies`, and an export of `books` as CSV into `dir`.
    fn bookshop(copies: u32) -> (tempfile::TempDir, ExportJob) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bookshop.db");
        tabletist_db::fixtures::write_sqlite_bookshop(&path).unwrap();
        if copies > 0 {
            let file = rusqlite::Connection::open(&path).unwrap();
            file.execute_batch(&format!(
                "CREATE TABLE shelf_copies (id INTEGER PRIMARY KEY, book_id INTEGER, title TEXT);
                 WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < {copies})
                 INSERT INTO shelf_copies SELECT i, 1 + i % 12, 'copy ' || i FROM n;"
            ))
            .unwrap();
        }
        let object = ObjectRef::new("main", "books");
        let job = ExportJob {
            id: ExportId(1),
            open: Opener {
                spec: ConnectSpec::sqlite(&path),
                secrets: Secrets::default(),
                host_keys: HostKeys::default(),
            },
            rows: ExportRows::Query(RowQuery::new(object.clone(), 0)),
            columns: vec!["id".into(), "title".into()],
            masked: vec![false, false],
            format: Format::Csv,
            options: Options::default(),
            object,
            dialect: Dialect::Sqlite,
            destination: Destination::File {
                path: dir.path().join("books.csv"),
                replace: false,
            },
        };
        (dir, job)
    }

    /// Runs `job`, keeping each state it told.
    async fn ran(job: ExportJob) -> (ExportState, Vec<ExportState>) {
        let told = StdMutex::new(Vec::new());
        let last = run(job, Stopper::default(), |state| {
            told.lock().unwrap().push(state)
        })
        .await;
        (last, told.into_inner().unwrap())
    }

    /// What is in `dir` besides the database: the files an export left.
    fn left(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|name| name != "bookshop.db")
            .collect();
        names.sort();
        names
    }

    #[tokio::test]
    async fn the_books_are_written_to_the_file_and_nothing_else_is_left() {
        let (dir, job) = bookshop(0);
        let (last, told) = ran(job).await;
        let ExportState::Done {
            rows,
            bytes,
            copied,
            ..
        } = last
        else {
            panic!("{last:?}");
        };
        assert_eq!((rows, copied), (12, None));
        assert!(
            matches!(told.first(), Some(ExportState::Started { statement: Some(sql) })
                if sql.starts_with(r#"SELECT "id", "title" FROM "main"."books""#)),
            "{told:?}"
        );
        let text = std::fs::read_to_string(dir.path().join("books.csv")).unwrap();
        assert_eq!(bytes, text.len() as u64);
        assert!(text.starts_with("id,title\n1,"), "{text}");
        assert!(text.ends_with('\n'), "a file ends with a line ending");
        assert_eq!(text.lines().count(), 13);
        assert_eq!(left(dir.path()), ["books.csv"]);
    }

    /// Test 5 of the spec: the file is the user's alone.
    #[cfg(unix)]
    #[tokio::test]
    async fn the_file_is_readable_by_its_owner_only() {
        use std::os::unix::fs::PermissionsExt;
        let (dir, job) = bookshop(0);
        ran(job).await;
        let mode = std::fs::metadata(dir.path().join("books.csv"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600);
    }

    /// Test 4: a file that is there is replaced only when asked, and the
    /// one that was there is kept otherwise.
    #[tokio::test]
    async fn an_existing_file_is_replaced_only_when_asked() {
        let (dir, mut job) = bookshop(0);
        let target = dir.path().join("books.csv");
        std::fs::write(&target, "kept").unwrap();
        let (last, _) = ran(job.clone()).await;
        assert_eq!(
            last,
            ExportState::Failed(ExportFailure::Exists {
                path: target.clone()
            })
        );
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "kept");
        assert_eq!(left(dir.path()), ["books.csv"]);
        job.destination = Destination::File {
            path: target.clone(),
            replace: true,
        };
        assert!(matches!(ran(job).await.0, ExportState::Done { .. }));
        assert!(
            std::fs::read_to_string(&target)
                .unwrap()
                .starts_with("id,title")
        );
    }

    /// Test 4: a disk that fills up leaves no file and no temporary file.
    #[tokio::test]
    async fn a_full_disk_leaves_nothing() {
        let (dir, mut job) = bookshop(0);
        job.destination = Destination::File {
            path: dir.path().join("full-20.csv"),
            replace: false,
        };
        let (last, _) = ran(job).await;
        assert_eq!(
            last,
            ExportState::Failed(ExportFailure::DiskFull { written: 0 })
        );
        assert!(left(dir.path()).is_empty(), "{:?}", left(dir.path()));
    }

    /// Test 4: a folder that may not be written is told by its name.
    #[cfg(unix)]
    #[tokio::test]
    async fn a_folder_that_may_not_be_written_is_named() {
        use std::os::unix::fs::PermissionsExt;
        let (dir, mut job) = bookshop(0);
        let shut = dir.path().join("shared");
        std::fs::create_dir(&shut).unwrap();
        std::fs::set_permissions(&shut, std::fs::Permissions::from_mode(0o500)).unwrap();
        job.destination = Destination::File {
            path: shut.join("books.csv"),
            replace: false,
        };
        let (last, _) = ran(job).await;
        std::fs::set_permissions(&shut, std::fs::Permissions::from_mode(0o700)).unwrap();
        assert_eq!(
            last,
            ExportState::Failed(ExportFailure::PermissionDenied {
                folder: shut.clone()
            })
        );
        assert_eq!(std::fs::read_dir(&shut).unwrap().count(), 0);
    }

    /// Test 4: a stop while it runs ends it as cancelled, with nothing
    /// left behind.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_cancel_leaves_nothing() {
        let (dir, mut job) = bookshop(60_000);
        job.rows = ExportRows::Query(RowQuery::new(ObjectRef::new("main", "shelf_copies"), 0));
        job.columns = vec!["id".into(), "book_id".into(), "title".into()];
        job.masked = vec![false; 3];
        let stopper = Stopper::default();
        let stopping = stopper.clone();
        let told = StdMutex::new(Vec::new());
        let last = run(job, stopper, |state| {
            if matches!(state, ExportState::Started { .. }) {
                stopping.stop.stop();
            }
            told.lock().unwrap().push(state);
        })
        .await;
        assert_eq!(last, ExportState::Cancelled);
        assert!(left(dir.path()).is_empty(), "{:?}", left(dir.path()));
    }

    /// Test 4: the connection going midway is told with the rows read,
    /// and leaves nothing.
    #[tokio::test]
    async fn a_lost_connection_leaves_nothing() {
        let (dir, job) = bookshop(0);
        let (to, from) = tokio::sync::mpsc::channel(1);
        let producing = async move {
            let columns = vec![ColumnMeta {
                name: "id".into(),
                type_name: "INTEGER".into(),
                kind: tabletist_db::ValueKind::Numeric,
            }];
            to.send(Streamed::Columns(columns)).await.unwrap();
            to.send(Streamed::Rows(vec![vec![Value::Int(1)]; 3]))
                .await
                .unwrap();
            Err::<u64, _>(Error::ConnectionLost("gone".into()))
        };
        let writing = write(&job, None, from, &|_| {});
        let (produced, written) = tokio::join!(producing, writing);
        let written = written.unwrap();
        let state = state_of_error(
            produced.unwrap_err(),
            written.rows,
            Instant::now(),
            &Stopper::default(),
        );
        drop(written);
        assert_eq!(
            state,
            ExportState::Failed(ExportFailure::ConnectionLost { rows: 3 })
        );
        assert!(left(dir.path()).is_empty(), "{:?}", left(dir.path()));
    }

    /// Test 2: 200,000 rows stream through, the progress told on the way.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn two_hundred_thousand_rows_are_written_whole() {
        let (dir, mut job) = bookshop(200_000);
        job.rows = ExportRows::Query(RowQuery::new(ObjectRef::new("main", "shelf_copies"), 0));
        job.columns = vec!["id".into()];
        job.masked = vec![false];
        job.destination = Destination::File {
            path: dir.path().join("shelf_copies.csv"),
            replace: false,
        };
        let (last, _) = ran(job).await;
        assert!(
            matches!(last, ExportState::Done { rows: 200_000, .. }),
            "{last:?}"
        );
        let text = std::fs::read_to_string(dir.path().join("shelf_copies.csv")).unwrap();
        assert_eq!(text.lines().count(), 200_001);
        assert_eq!(text.lines().last(), Some("200000"));
    }

    /// The loaded rows of a tab (spec, D7) are written without a
    /// connection, and to the clipboard within its limit.
    #[tokio::test]
    async fn loaded_rows_go_to_the_clipboard_without_a_connection() {
        let (_dir, mut job) = bookshop(0);
        job.open.spec = ConnectSpec::sqlite("/nowhere/at/all.db");
        job.rows = ExportRows::Loaded {
            columns: vec![ColumnMeta {
                name: "title".into(),
                type_name: "TEXT".into(),
                kind: tabletist_db::ValueKind::Text,
            }],
            rows: vec![
                vec![Value::Text("Small Rooms".into())],
                vec![Value::Text("Salt Roads".into())],
            ],
        };
        job.format = Format::Tsv;
        job.destination = Destination::Clipboard;
        let (last, _) = ran(job.clone()).await;
        let ExportState::Done { copied, rows, .. } = last else {
            panic!("{last:?}");
        };
        assert_eq!(rows, 2);
        assert_eq!(copied.as_deref(), Some("title\nSmall Rooms\nSalt Roads"));

        let ExportRows::Loaded { rows, .. } = &mut job.rows else {
            unreachable!()
        };
        *rows = vec![vec![Value::Text("x".into())]; export::CLIPBOARD_ROWS + 1];
        assert_eq!(
            ran(job).await.0,
            ExportState::Failed(ExportFailure::TooManyRows)
        );
    }

    /// Test 15: a masked column is written as `•••` when the decision
    /// says so, and as it is otherwise; NULL stays NULL.
    #[tokio::test]
    async fn masked_columns_follow_the_decision() {
        let (dir, mut job) = bookshop(0);
        let masked: Vec<tabletist_db::agent::ColumnName> = vec!["books.title".parse().unwrap()];
        for follow in [false, true] {
            job.masked = export::masks(follow, &masked, &job.object, &["id", "title"]);
            job.destination = Destination::Clipboard;
            let ExportState::Done { copied, .. } = ran(job.clone()).await.0 else {
                panic!()
            };
            let copied = copied.unwrap();
            let second = copied.lines().nth(1).unwrap();
            assert_eq!(second.ends_with(",•••"), follow, "{follow}: {second}");
        }
        assert!(!export::follows_masking(), "files get the real values");
        assert!(left(dir.path()).is_empty());
    }

    /// SQL INSERT with Start with CREATE TABLE reads the table's
    /// definition on the export's connection.
    #[tokio::test]
    async fn sql_insert_starts_with_the_create_table_when_asked() {
        let (dir, mut job) = bookshop(0);
        job.format = Format::SqlInsert;
        job.options.sql.create_table = true;
        job.destination = Destination::File {
            path: dir.path().join("books.sql"),
            replace: false,
        };
        assert!(matches!(ran(job).await.0, ExportState::Done { .. }));
        let text = std::fs::read_to_string(dir.path().join("books.sql")).unwrap();
        assert!(text.starts_with("CREATE TABLE books"), "{text}");
        assert!(text.contains("INSERT INTO"), "{text}");
    }

    /// A cancel that comes after the last batch, before the rename, keeps
    /// the file out: what the toast says ("Nothing was saved") holds.
    #[tokio::test]
    async fn a_cancel_after_the_last_batch_keeps_the_file_out() {
        let (dir, job) = bookshop(0);
        let (to, from) = tokio::sync::mpsc::channel(1);
        let producing = async move {
            let columns = vec![ColumnMeta {
                name: "id".into(),
                type_name: "INTEGER".into(),
                kind: tabletist_db::ValueKind::Numeric,
            }];
            to.send(Streamed::Columns(columns)).await.unwrap();
            to.send(Streamed::Rows(vec![vec![Value::Int(1)]; 3]))
                .await
                .unwrap();
        };
        let writing = write(&job, None, from, &|_| {});
        let ((), written) = tokio::join!(producing, writing);
        let ended = written.unwrap().finish(&|| true);
        assert!(matches!(ended, Err(None)), "a cancel");
        assert!(left(dir.path()).is_empty(), "{:?}", left(dir.path()));
    }

    /// Test 10: past 20 MB the clipboard copies nothing, and stops reading
    /// there rather than at 10,000 rows.
    #[tokio::test]
    async fn the_clipboard_stops_at_twenty_megabytes() {
        let (_dir, mut job) = bookshop(0);
        let wide = "x".repeat(1024 * 1024);
        job.rows = ExportRows::Loaded {
            columns: vec![ColumnMeta {
                name: "title".into(),
                type_name: "TEXT".into(),
                kind: tabletist_db::ValueKind::Text,
            }],
            rows: vec![vec![Value::Text(wide.into())]; 25],
        };
        job.columns = vec!["title".into()];
        job.masked = vec![false];
        job.destination = Destination::Clipboard;
        assert_eq!(
            ran(job).await.0,
            ExportState::Failed(ExportFailure::TooLarge)
        );
    }

    #[test]
    fn the_temporary_file_is_hidden_beside_the_target() {
        let dir = tempfile::tempdir().unwrap();
        let part = part_beside(&dir.path().join("reviews.csv")).unwrap();
        let name = part
            .path()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        assert!(name.starts_with(".reviews.csv."), "{name}");
        assert!(name.ends_with(".tabletist-part"), "{name}");
        assert_eq!(
            name.len(),
            ".reviews.csv.".len() + 6 + ".tabletist-part".len()
        );
        assert_eq!(part.path().parent(), Some(dir.path()));
    }
}
