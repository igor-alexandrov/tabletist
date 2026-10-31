//! Files connections: DuckDB in the app's process, in memory, with a view
//! for each file and each `.duckdb` file attached as a schema. DuckDB is
//! blocking, so every call runs on tokio's blocking pool, as SQLite's do.
//!
//! The session is fenced twice. DuckDB's own configuration, set once the
//! views are made and locked so no SQL can set it back: it may open the
//! connection's files and no other, which also keeps it from writing a
//! file, installing or loading an extension. And a read-only transaction
//! around every page, count, lookup, script and agent's read.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use duckdb::arrow::array::{Array, AsArray, RecordBatch};
use duckdb::arrow::datatypes::{DataType, Field, Float64Type, Int64Type, UInt64Type};
use duckdb::arrow::util::display::{ArrayFormatter, FormatOptions};

use crate::adapter::Adapter;
use crate::agent::{Catalog, ColumnName, Limits, Param, Read, Refusal, Sources};
use crate::files::{Format, Found, Source};
use crate::{
    CancelHandle, CancelInner, ChangeSet, ColumnInfo, ColumnMeta, Ddl, Dialect, Error, Lookup,
    LookupPage, MAX_LISTED, ObjectInfo, ObjectKind, ObjectRef, Result, RowPage, RowQuery,
    ScriptMode, ScriptOutcome, StatementOutcome, StatementResult, StopFlag, Structure, Value,
    ValueKind, WriteOutcome,
};

/// The schema a connection's files are tables of.
pub(crate) const MAIN: &str = "main";

/// DuckDB's name for the in-memory database the views are in.
const MEMORY: &str = "memory";

/// How many rows Test reads of each file, and how many rows of a CSV the
/// reader samples for its columns' types.
pub(crate) const SAMPLE_ROWS: u32 = 20_000;

/// Where DuckDB keeps the extensions it installs: the app's own folder,
/// set once at start. Unset (in tests), DuckDB's own default.
static EXTENSIONS: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();

/// Sets the folder DuckDB installs extensions into, for every Files
/// connection opened after it. Only the first call counts.
pub fn set_extension_directory(folder: std::path::PathBuf) {
    let _ = EXTENSIONS.set(folder);
}

/// What a Test of a Files connection read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilesRead {
    /// The files read, `.duckdb` ones included.
    pub files: usize,
    /// Their size on disk, in bytes.
    pub bytes: u64,
}

/// What the Structure view says of the file a table is read from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileFacts {
    pub path: std::path::PathBuf,
    pub size: u64,
    pub format: Format,
    /// How the reader is told to read it, as the view's call says it:
    /// `header = true, auto_detect = true, sample_size = 20000`.
    pub options: String,
}

/// An open Files connection.
pub(crate) struct Conn {
    inner: Arc<Mutex<duckdb::Connection>>,
    interrupt: Arc<duckdb::InterruptHandle>,
    /// The call whose work holds the session now (0 for none), so that a
    /// call given up on interrupts its own statement and no other's.
    running: Arc<std::sync::atomic::AtomicU64>,
    /// The number the next call takes.
    calls: std::sync::atomic::AtomicU64,
    files: Arc<Vec<Opened>>,
    version: String,
}

/// A file as the session opened it.
#[derive(Debug, Clone)]
struct Opened {
    found: Found,
    /// The statement that made its view, which the DDL view shows: DuckDB
    /// keeps it rewritten, each option in parentheses.
    made: String,
    /// Rows, where the file says (Parquet's metadata) or a sample of it
    /// lets them be estimated.
    rows: Option<u64>,
}

/// Maps DuckDB's errors onto ours. DuckDB starts its message with its kind
/// (`Binder Error: …`, `Permission Error: …`), which stays.
pub(crate) fn map_error(error: duckdb::Error) -> Error {
    match &error {
        // DuckDB's own kind for a statement it was told to stop, at the
        // start of its words: a word "Interrupted" a message quotes is not.
        duckdb::Error::DuckDBFailure(_, Some(message))
            if message.starts_with("INTERRUPT Error") =>
        {
            Error::Cancelled
        }
        duckdb::Error::DuckDBFailure(_, message) => Error::query(
            message
                .clone()
                .unwrap_or_else(|| error.to_string())
                .trim_end()
                .to_owned(),
        ),
        _ => Error::query(error.to_string()),
    }
}

/// A string literal in DuckDB's SQL: its strings are standard, so a quote
/// is doubled and a backslash is itself.
fn literal(text: &str) -> String {
    Dialect::DuckDb.literal(text)
}

/// A path as DuckDB is given it.
fn path_text(path: &Path) -> Result<String> {
    path.to_str().map(str::to_owned).ok_or_else(|| {
        Error::Connect(format!(
            "Can't open {}: the path is not valid UTF-8",
            path.display()
        ))
    })
}

/// The reader's options for a file of `format`, as its call writes them
/// after the path.
fn reader_options(format: Format) -> &'static str {
    match format {
        Format::Csv => "header = true, auto_detect = true, sample_size = 20000",
        Format::Tsv => "header = true, auto_detect = true, sample_size = 20000, delim = '\\t'",
        Format::Json => "format = 'array'",
        Format::Ndjson => "format = 'newline_delimited'",
        Format::Excel => "header = true",
        Format::Parquet | Format::DuckDb => "",
    }
}

/// The table function that reads a file of `format`.
fn reader(format: Format) -> &'static str {
    match format {
        Format::Csv | Format::Tsv => "read_csv",
        Format::Parquet => "read_parquet",
        Format::Json | Format::Ndjson => "read_json",
        Format::Excel => "read_xlsx",
        // Attached, not read: never asked.
        Format::DuckDb => "",
    }
}

/// The statement that makes `file` a table or a schema of the session,
/// its name bare where that is safe: the DDL view shows it as it is.
fn make(file: &Found) -> Result<String> {
    let path = literal(&path_text(&file.path)?);
    let name = Dialect::DuckDb.ident(&file.name);
    Ok(match file.format {
        Format::DuckDb => format!("ATTACH {path} AS {name} (READ_ONLY)"),
        format => {
            let options = reader_options(format);
            let call = match options {
                "" => format!("{}({path})", reader(format)),
                options => format!("{}({path}, {options})", reader(format)),
            };
            format!("CREATE VIEW {name} AS SELECT * FROM {call}")
        }
    })
}

/// Rows of a text file by a sample of its start: its lines in the first
/// 256 KiB, scaled to its size, without a CSV's header. `None` for a
/// format that is not a line a row, or a file that cannot be read.
fn estimate(file: &Found) -> Option<u64> {
    use std::io::Read as _;
    let header = match file.format {
        Format::Csv | Format::Tsv => 1,
        Format::Ndjson => 0,
        Format::Parquet | Format::Json | Format::Excel | Format::DuckDb => return None,
    };
    let mut start = Vec::with_capacity(256 * 1024);
    std::fs::File::open(&file.path)
        .ok()?
        .take(256 * 1024)
        .read_to_end(&mut start)
        .ok()?;
    if start.is_empty() {
        return Some(0);
    }
    let mut lines = start.iter().filter(|byte| **byte == b'\n').count() as u64;
    if start.last() != Some(&b'\n') {
        lines += 1;
    }
    let whole = start.len() as u64 >= file.size;
    let rows = match whole {
        true => lines,
        false => {
            let scaled = (lines as f64) * (file.size as f64) / (start.len() as f64);
            scaled.round() as u64
        }
    };
    Some(rows.saturating_sub(header))
}

/// Opens a session over `files`: views and attaches, then the fence.
fn opened(files: &[Found]) -> Result<(duckdb::Connection, Vec<Opened>, String)> {
    let config = duckdb::Config::default()
        .enable_autoload_extension(false)
        .map_err(map_error)?;
    let connection = duckdb::Connection::open_in_memory_with_flags(config).map_err(map_error)?;
    let run = |sql: &str| connection.execute_batch(sql).map_err(map_error);
    if let Some(folder) = EXTENSIONS.get() {
        run(&format!(
            "SET extension_directory = {}",
            literal(&path_text(folder)?)
        ))?;
    }
    if files.iter().any(|file| file.format == Format::Excel) {
        // Installed once, then found in the folder: offline, a second open
        // still reads Excel files.
        run("INSTALL excel; LOAD excel").map_err(|error| {
            Error::Connect(format!(
                "Can't read Excel files: the Excel reader for DuckDB could not be installed \
                 ({error})"
            ))
        })?;
    }
    let mut opened = Vec::with_capacity(files.len());
    for file in files {
        // A `.duckdb` file is attached read-only, as every session is.
        let statement = make(file)?;
        run(&statement).map_err(|error| {
            Error::Connect(format!(
                "Can't read {}: {}",
                file_label(file),
                plain(&error)
            ))
        })?;
        let rows = match file.format {
            Format::Parquet => connection
                .query_row(
                    &format!(
                        "SELECT sum(num_rows)::BIGINT FROM parquet_file_metadata({})",
                        literal(&path_text(&file.path)?)
                    ),
                    [],
                    |row| row.get::<_, Option<i64>>(0),
                )
                .ok()
                .flatten()
                .and_then(|rows| u64::try_from(rows).ok()),
            Format::Csv
            | Format::Tsv
            | Format::Json
            | Format::Ndjson
            | Format::Excel
            | Format::DuckDb => estimate(file),
        };
        opened.push(Opened {
            found: file.clone(),
            made: statement,
            rows,
        });
    }
    // The fence: these files and no other, then no setting changed again.
    let allowed: Vec<String> = files
        .iter()
        .map(|file| path_text(&file.path).map(|path| literal(&path)))
        .collect::<Result<_>>()?;
    run(&format!("SET allowed_paths = [{}]", allowed.join(", ")))?;
    run("SET enable_external_access = false")?;
    run("SET lock_configuration = true")?;
    let version: String = connection
        .query_row("SELECT version()", [], |row| row.get(0))
        .map_err(map_error)?;
    Ok((
        connection,
        opened,
        format!("DuckDB {}", version.trim_start_matches('v')),
    ))
}

/// Stops call `call`'s work when dropped, unless disarmed first: a dropped
/// future (a closed tab) stops its statement, or the work before it starts.
/// The work of another call, waiting behind it or holding the session, is
/// left alone.
struct InterruptOnDrop {
    interrupt: Arc<duckdb::InterruptHandle>,
    running: Arc<std::sync::atomic::AtomicU64>,
    given_up: Arc<AtomicBool>,
    call: u64,
    armed: bool,
}

impl InterruptOnDrop {
    fn disarm(mut self) {
        self.armed = false;
    }
}

impl Drop for InterruptOnDrop {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        self.given_up.store(true, Ordering::SeqCst);
        if self.running.load(Ordering::SeqCst) == self.call {
            self.interrupt.interrupt();
        }
    }
}

/// Interrupts the session's statement once `stop` is set or `deadline`
/// passes, from a thread of its own: DuckDB has no handler it calls while
/// a statement runs. Ends with the run.
struct Watch {
    done: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl Watch {
    fn start(
        interrupt: Arc<duckdb::InterruptHandle>,
        stop: StopFlag,
        deadline: Option<Instant>,
    ) -> Self {
        let done = Arc::new(AtomicBool::new(false));
        let ended = Arc::clone(&done);
        let thread = std::thread::spawn(move || {
            while !ended.load(Ordering::SeqCst) {
                if stop.is_stopped() || deadline.is_some_and(|at| Instant::now() >= at) {
                    interrupt.interrupt();
                    return;
                }
                std::thread::sleep(Duration::from_millis(5));
            }
        });
        Self {
            done,
            thread: Some(thread),
        }
    }
}

impl Drop for Watch {
    fn drop(&mut self) {
        self.done.store(true, Ordering::SeqCst);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// Runs `work` in a read-only transaction, which is rolled back after it
/// whatever it did.
fn read_only<T>(
    connection: &duckdb::Connection,
    work: impl FnOnce(&duckdb::Connection) -> Result<T>,
) -> Result<T> {
    connection
        .execute_batch("BEGIN TRANSACTION READ ONLY")
        .map_err(map_error)?;
    let result = work(connection);
    let ended = connection.execute_batch("ROLLBACK").map_err(map_error);
    let value = result?;
    ended?;
    Ok(value)
}

/// A value an agent sent, as DuckDB binds it.
fn bound(param: &Param) -> duckdb::types::Value {
    match param {
        Param::Null => duckdb::types::Value::Null,
        Param::Bool(flag) => duckdb::types::Value::Boolean(*flag),
        Param::Int(number) => duckdb::types::Value::BigInt(*number),
        Param::Float(number) => duckdb::types::Value::Double(*number),
        Param::Text(text) => duckdb::types::Value::Text(text.clone()),
    }
}

/// One of ours, as DuckDB binds it.
fn to_duckdb(value: &Value) -> duckdb::types::Value {
    match value {
        Value::Null => duckdb::types::Value::Null,
        Value::Bool(flag) => duckdb::types::Value::Boolean(*flag),
        Value::Int(number) => duckdb::types::Value::BigInt(*number),
        Value::Float(number) => duckdb::types::Value::Double(*number),
        Value::Text(text) => duckdb::types::Value::Text(text.to_string()),
        Value::Bytes(bytes) => duckdb::types::Value::Blob(bytes.to_vec()),
    }
}

/// DuckDB's name for a column of `field`, and how the grid treats it.
fn column_meta(field: &Field) -> ColumnMeta {
    let json = field
        .metadata()
        .get("ARROW:extension:name")
        .is_some_and(|name| name.ends_with("json"));
    let (type_name, kind) = match field.data_type() {
        _ if json => ("JSON".to_owned(), ValueKind::Json),
        DataType::Boolean => ("BOOLEAN".to_owned(), ValueKind::Bool),
        DataType::Int8 => ("TINYINT".to_owned(), ValueKind::Numeric),
        DataType::Int16 => ("SMALLINT".to_owned(), ValueKind::Numeric),
        DataType::Int32 => ("INTEGER".to_owned(), ValueKind::Numeric),
        DataType::Int64 => ("BIGINT".to_owned(), ValueKind::Numeric),
        DataType::UInt8 => ("UTINYINT".to_owned(), ValueKind::Numeric),
        DataType::UInt16 => ("USMALLINT".to_owned(), ValueKind::Numeric),
        DataType::UInt32 => ("UINTEGER".to_owned(), ValueKind::Numeric),
        DataType::UInt64 => ("UBIGINT".to_owned(), ValueKind::Numeric),
        DataType::Float16 | DataType::Float32 => ("FLOAT".to_owned(), ValueKind::Numeric),
        DataType::Float64 => ("DOUBLE".to_owned(), ValueKind::Numeric),
        DataType::Decimal32(precision, scale)
        | DataType::Decimal64(precision, scale)
        | DataType::Decimal128(precision, scale) => {
            (format!("DECIMAL({precision},{scale})"), ValueKind::Numeric)
        }
        DataType::Decimal256(precision, scale) => {
            (format!("DECIMAL({precision},{scale})"), ValueKind::Numeric)
        }
        DataType::Utf8 | DataType::LargeUtf8 | DataType::Utf8View => {
            ("VARCHAR".to_owned(), ValueKind::Text)
        }
        DataType::Binary
        | DataType::LargeBinary
        | DataType::BinaryView
        | DataType::FixedSizeBinary(_) => ("BLOB".to_owned(), ValueKind::Binary),
        DataType::Date32 | DataType::Date64 => ("DATE".to_owned(), ValueKind::Temporal),
        DataType::Time32(_) | DataType::Time64(_) => ("TIME".to_owned(), ValueKind::Temporal),
        DataType::Timestamp(_, None) => ("TIMESTAMP".to_owned(), ValueKind::Temporal),
        DataType::Timestamp(_, Some(_)) => {
            ("TIMESTAMP WITH TIME ZONE".to_owned(), ValueKind::Temporal)
        }
        DataType::Interval(_) | DataType::Duration(_) => {
            ("INTERVAL".to_owned(), ValueKind::Temporal)
        }
        DataType::List(_)
        | DataType::LargeList(_)
        | DataType::ListView(_)
        | DataType::LargeListView(_)
        | DataType::FixedSizeList(..) => ("LIST".to_owned(), ValueKind::Other),
        DataType::Struct(_) => ("STRUCT".to_owned(), ValueKind::Other),
        DataType::Map(..) => ("MAP".to_owned(), ValueKind::Other),
        DataType::Union(..) => ("UNION".to_owned(), ValueKind::Other),
        DataType::Dictionary(..) => ("ENUM".to_owned(), ValueKind::Text),
        DataType::Null => (String::new(), ValueKind::Other),
        other => (other.to_string(), ValueKind::Other),
    };
    ColumnMeta {
        name: field.name().clone(),
        type_name,
        kind,
    }
}

/// How a value DuckDB has no plain form for is written: dates and times
/// as PostgreSQL writes them, the rest as DuckDB's Arrow export says it.
/// A list, a struct or a map, whose own values are written inside it.
fn nested(data_type: &DataType) -> bool {
    matches!(
        data_type,
        DataType::List(_)
            | DataType::LargeList(_)
            | DataType::FixedSizeList(..)
            | DataType::Struct(_)
            | DataType::Map(..)
    )
}

/// How a value DuckDB has no plain form for is written: dates and times
/// as PostgreSQL writes them, the rest as DuckDB's Arrow export says it;
/// inside a list, a struct or a map, NULL as the word and text quoted. A
/// value that cannot be written (a year past what Arrow keeps) is an
/// error of its own, not text.
fn format_options(data_type: &DataType) -> FormatOptions<'static> {
    FormatOptions::new()
        .with_display_error(false)
        .with_null("NULL")
        .with_quoted_strings(nested(data_type))
        .with_date_format(Some("%Y-%m-%d"))
        .with_datetime_format(Some("%Y-%m-%d %H:%M:%S%.f"))
        .with_timestamp_format(Some("%Y-%m-%d %H:%M:%S%.f"))
        .with_time_format(Some("%H:%M:%S%.f"))
}

/// `data_type` with every moment's zone taken off, where it holds one
/// inside a list, a struct or a map: DuckDB keeps those in UTC, and Arrow
/// cannot read the zone's name without the zone database.
fn zoneless(data_type: &DataType) -> Option<DataType> {
    use duckdb::arrow::datatypes::Fields;
    let field = |field: &Field, inner: DataType| Arc::new(field.clone().with_data_type(inner));
    match data_type {
        DataType::Timestamp(unit, Some(_)) => Some(DataType::Timestamp(*unit, None)),
        DataType::List(inner) => {
            zoneless(inner.data_type()).map(|t| DataType::List(field(inner, t)))
        }
        DataType::LargeList(inner) => {
            zoneless(inner.data_type()).map(|t| DataType::LargeList(field(inner, t)))
        }
        DataType::FixedSizeList(inner, size) => {
            zoneless(inner.data_type()).map(|t| DataType::FixedSizeList(field(inner, t), *size))
        }
        DataType::Map(inner, sorted) => {
            zoneless(inner.data_type()).map(|t| DataType::Map(field(inner, t), *sorted))
        }
        DataType::Struct(fields) => {
            let changed: Vec<Option<DataType>> =
                fields.iter().map(|f| zoneless(f.data_type())).collect();
            changed.iter().any(Option::is_some).then(|| {
                DataType::Struct(Fields::from(
                    fields
                        .iter()
                        .zip(changed)
                        .map(|(f, t)| match t {
                            Some(t) => field(f, t),
                            None => Arc::clone(f),
                        })
                        .collect::<Vec<_>>(),
                ))
            })
        }
        _ => None,
    }
}

/// DuckDB's infinities, where `array` is of dates or moments: `infinity`
/// and `-infinity` for the largest value and its negation, which Arrow
/// would take for a day past what it can write.
fn infinities(array: &dyn Array) -> Option<Vec<Option<&'static str>>> {
    let (wide, max) = match array.data_type() {
        DataType::Timestamp(..) => (
            duckdb::arrow::compute::cast(array, &DataType::Int64).ok()?,
            i64::MAX,
        ),
        DataType::Date32 => {
            let days = duckdb::arrow::compute::cast(array, &DataType::Int32).ok()?;
            (
                duckdb::arrow::compute::cast(&days, &DataType::Int64).ok()?,
                i64::from(i32::MAX),
            )
        }
        _ => return None,
    };
    let values = wide.as_primitive::<Int64Type>();
    Some(
        (0..values.len())
            .map(|at| match values.is_valid(at).then(|| values.value(at)) {
                Some(value) if value == max => Some("infinity"),
                Some(value) if value == -max => Some("-infinity"),
                _ => None,
            })
            .collect(),
    )
}

/// The values of one column of a batch, in its rows' order.
fn column_values(array: &dyn Array) -> Result<Vec<Value>> {
    let rows = array.len();
    let null = |at: usize| array.is_null(at);
    let infinite = infinities(array);
    let text_of = |shown: &dyn Array, suffix: &str| -> Result<Vec<Value>> {
        let options = format_options(shown.data_type());
        let formatter = ArrayFormatter::try_new(shown, &options)
            .map_err(|error| Error::query(error.to_string()))?;
        (0..rows)
            .map(|at| {
                if null(at) {
                    return Ok(Value::Null);
                }
                if let Some(word) = infinite.as_ref().and_then(|words| words[at]) {
                    return Ok(Value::Text(word.into()));
                }
                let text = formatter
                    .value(at)
                    .try_to_string()
                    .map_err(|error| Error::query(error.to_string()))?;
                Ok(Value::Text(format!("{text}{suffix}").into()))
            })
            .collect()
    };
    let cast = |to: &DataType| {
        duckdb::arrow::compute::cast(array, to).map_err(|error| Error::query(error.to_string()))
    };
    Ok(match array.data_type() {
        DataType::Null => vec![Value::Null; rows],
        DataType::Boolean => {
            let values = array.as_boolean();
            (0..rows)
                .map(|at| match null(at) {
                    true => Value::Null,
                    false => Value::Bool(values.value(at)),
                })
                .collect()
        }
        DataType::Int8
        | DataType::Int16
        | DataType::Int32
        | DataType::Int64
        | DataType::UInt8
        | DataType::UInt16
        | DataType::UInt32 => {
            let wide = cast(&DataType::Int64)?;
            let values = wide.as_primitive::<Int64Type>();
            (0..rows)
                .map(|at| match null(at) {
                    true => Value::Null,
                    false => Value::Int(values.value(at)),
                })
                .collect()
        }
        // Past i64 it stays text, so nothing is rounded.
        DataType::UInt64 => {
            let values = array.as_primitive::<UInt64Type>();
            (0..rows)
                .map(|at| match null(at) {
                    true => Value::Null,
                    false => match i64::try_from(values.value(at)) {
                        Ok(number) => Value::Int(number),
                        Err(_) => Value::Text(values.value(at).to_string().into()),
                    },
                })
                .collect()
        }
        DataType::Float16 | DataType::Float32 | DataType::Float64 => {
            let wide = cast(&DataType::Float64)?;
            let values = wide.as_primitive::<Float64Type>();
            (0..rows)
                .map(|at| match null(at) {
                    true => Value::Null,
                    false => Value::Float(values.value(at)),
                })
                .collect()
        }
        DataType::Binary | DataType::LargeBinary | DataType::BinaryView => {
            let wide = cast(&DataType::LargeBinary)?;
            let values = wide.as_binary::<i64>();
            (0..rows)
                .map(|at| match null(at) {
                    true => Value::Null,
                    false => Value::Bytes(values.value(at).into()),
                })
                .collect()
        }
        // A moment with an offset is kept in UTC, and DuckDB is built
        // without the zones' names: written as PostgreSQL writes UTC.
        DataType::Timestamp(unit, Some(_)) => {
            text_of(cast(&DataType::Timestamp(*unit, None))?.as_ref(), "+00")?
        }
        // Moments inside a list, a struct or a map, without their zone.
        other => match zoneless(other) {
            Some(plain) => text_of(cast(&plain)?.as_ref(), "")?,
            None => text_of(array, "")?,
        },
    })
}

/// The rows of `batch`, each a row of cells.
fn batch_rows(batch: &RecordBatch) -> Result<Vec<Vec<Value>>> {
    let columns: Vec<Vec<Value>> = batch
        .columns()
        .iter()
        .map(|column| column_values(column.as_ref()))
        .collect::<Result<_>>()?;
    Ok((0..batch.num_rows())
        .map(|row| columns.iter().map(|column| column[row].clone()).collect())
        .collect())
}

/// Runs one statement and reads at most `limit` rows of it, and whether
/// there were more. A streaming read: DuckDB makes no more rows than are
/// read.
fn rows_of(
    connection: &duckdb::Connection,
    text: &str,
    params: Vec<duckdb::types::Value>,
    limit: usize,
) -> Result<(Vec<ColumnMeta>, Vec<Vec<Value>>, bool)> {
    let mut statement = connection.prepare(text).map_err(map_error)?;
    let stream = statement
        .stream_arrow(duckdb::params_from_iter(params))
        .map_err(map_error)?;
    let columns: Vec<ColumnMeta> = stream
        .get_schema()
        .fields()
        .iter()
        .map(|field| column_meta(field))
        .collect();
    let mut rows = Vec::new();
    for batch in stream {
        let batch = batch.map_err(map_error)?;
        rows.extend(batch_rows(&batch)?);
        // Never read past one extra row, even if a raw WHERE comments out
        // the LIMIT.
        if rows.len() > limit {
            break;
        }
    }
    let more = rows.len() > limit;
    rows.truncate(limit);
    Ok((columns, rows, more))
}

/// Runs `texts` in one read-only transaction, each keeping at most `limit`
/// rows; the first that fails ends the run. `params` are bound to the
/// first statement (an agent's read is one statement). `watch`, which
/// interrupts a statement on a stop or at the deadline, ends before the
/// rollback.
fn script(
    connection: &duckdb::Connection,
    texts: &[String],
    limit: usize,
    stop: &StopFlag,
    params: &[Param],
    watch: Watch,
) -> Result<ScriptOutcome> {
    let mut outcome = ScriptOutcome::default();
    if stop.is_stopped() {
        outcome.stopped = true;
        return Ok(outcome);
    }
    connection
        .execute_batch("BEGIN TRANSACTION READ ONLY")
        .map_err(map_error)?;
    for (index, text) in texts.iter().enumerate() {
        if stop.is_stopped() {
            outcome.stopped = true;
            break;
        }
        let started = Instant::now();
        let sent = match index {
            0 => params.iter().map(bound).collect(),
            _ => Vec::new(),
        };
        let result = rows_of(connection, text, sent, limit);
        let elapsed = started.elapsed();
        let (statement, failed) = match result {
            Ok((columns, rows, truncated)) => (
                StatementOutcome::Rows {
                    columns,
                    rows,
                    truncated,
                },
                false,
            ),
            Err(Error::Cancelled) => (StatementOutcome::Cancelled, true),
            Err(error) => (
                StatementOutcome::Error {
                    error,
                    position: None,
                },
                true,
            ),
        };
        outcome.results.push(StatementResult {
            elapsed,
            outcome: statement,
        });
        if failed {
            break;
        }
    }
    // The watch ends before the rollback, so no interrupt reaches it.
    drop(watch);
    stop.finish();
    // A failed statement leaves DuckDB's transaction aborted; the rollback
    // ends it either way.
    connection
        .execute_batch("ROLLBACK")
        .map_err(|error| crate::script::cleanup_failed(ScriptMode::ReadOnly, &map_error(error)))?;
    Ok(outcome)
}

/// A WHERE typed in the filter bar is one condition: a `;` or a NUL in
/// it is refused before DuckDB reads it.
fn check_raw_where(query: &RowQuery) -> Result<()> {
    let Some(raw) = query.raw_where.as_deref() else {
        return Ok(());
    };
    if raw.contains('\0') {
        return Err(Error::query(
            "The WHERE text holds a NUL character. Remove it.",
        ));
    }
    if crate::sql::tokenize(Dialect::DuckDb, raw)
        .iter()
        .any(|token| token.kind == crate::sql::TokenKind::Semicolon)
    {
        return Err(Error::query(
            "The WHERE text holds a `;`. A filter is one condition, not a statement.",
        ));
    }
    Ok(())
}

/// The catalog and schema DuckDB keeps `object` in: a file's view in the
/// in-memory database's `main`, a `.duckdb` file's table in its own.
fn catalog_of(object: &ObjectRef) -> (&str, &str) {
    match object.schema.as_str() {
        MAIN => (MEMORY, MAIN),
        attached => (attached, MAIN),
    }
}

/// The columns of `object`, as DuckDB's catalog has them.
fn columns(connection: &duckdb::Connection, object: &ObjectRef) -> Result<Vec<ColumnInfo>> {
    let (catalog, schema) = catalog_of(object);
    let mut statement = connection
        .prepare(
            "SELECT column_name, data_type, is_nullable, column_default \
             FROM information_schema.columns \
             WHERE table_catalog = ? AND table_schema = ? AND table_name = ? \
             ORDER BY ordinal_position",
        )
        .map_err(map_error)?;
    let rows = statement
        .query_map([catalog, schema, object.name.as_str()], |row| {
            Ok(ColumnInfo {
                name: row.get(0)?,
                type_name: row.get(1)?,
                nullable: row.get::<_, String>(2)? != "NO",
                default: row.get(3)?,
                ..ColumnInfo::default()
            })
        })
        .map_err(map_error)?;
    rows.collect::<std::result::Result<Vec<_>, _>>()
        .map_err(map_error)
}

/// The columns of `query`'s table that hold bytes.
fn binary_columns(connection: &duckdb::Connection, object: &ObjectRef) -> Result<Vec<String>> {
    Ok(binary_of(&columns(connection, object)?))
}

/// The names of `columns` that hold bytes.
fn binary_of(columns: &[ColumnInfo]) -> Vec<String> {
    columns
        .iter()
        .filter(|column| {
            crate::column_class(Dialect::DuckDb, &column.type_name) == crate::ColumnClass::Binary
        })
        .map(|column| column.name.clone())
        .collect()
}

impl Conn {
    /// Opens `source`'s files, read-only: every one is read far enough to
    /// know its columns, and an error names the first that cannot be.
    pub(crate) async fn open(source: &Source) -> Result<Self> {
        let source = source.clone();
        let (connection, files, version) = tokio::task::spawn_blocking(move || {
            let scan = crate::files::scan(&source)?;
            if let Some(gone) = scan.missing.first() {
                return Err(Error::Connect(format!(
                    "Can't open {}: no such file",
                    crate::files::shown(gone)
                )));
            }
            opened(&scan.files)
        })
        .await
        .map_err(|error| Error::Io(error.to_string()))??;
        let interrupt = connection.interrupt_handle();
        Ok(Self {
            inner: Arc::new(Mutex::new(connection)),
            interrupt,
            running: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            calls: std::sync::atomic::AtomicU64::new(0),
            files: Arc::new(files),
            version,
        })
    }

    /// Runs `work` on the blocking pool with the session.
    async fn run<T: Send + 'static>(
        &self,
        work: impl FnOnce(&duckdb::Connection) -> Result<T> + Send + 'static,
    ) -> Result<T> {
        let inner = Arc::clone(&self.inner);
        let call = self.calls.fetch_add(1, Ordering::SeqCst) + 1;
        let given_up = Arc::new(AtomicBool::new(false));
        let running = Arc::clone(&self.running);
        let guard = InterruptOnDrop {
            interrupt: Arc::clone(&self.interrupt),
            running: Arc::clone(&self.running),
            given_up: Arc::clone(&given_up),
            call,
            armed: true,
        };
        let result = tokio::task::spawn_blocking(move || {
            let connection = inner
                .lock()
                .map_err(|_| Error::ConnectionLost("the DuckDB session was poisoned".into()))?;
            // Said to be running first, then asked whether it was given up
            // on, as the guard says it gives up first, then asks: one of
            // the two always sees the other. Given up on: nothing runs.
            running.store(call, Ordering::SeqCst);
            if given_up.load(Ordering::SeqCst) {
                running.store(0, Ordering::SeqCst);
                return Err(Error::Cancelled);
            }
            let result = work(&connection);
            running.store(0, Ordering::SeqCst);
            result
        })
        .await;
        guard.disarm();
        result.map_err(|error| {
            if error.is_panic() {
                Error::ConnectionLost("the DuckDB worker panicked".into())
            } else {
                Error::Io(error.to_string())
            }
        })?
    }

    /// Reads the first rows of every file, as a Test does: never a whole
    /// file.
    pub(crate) async fn read_files(&self) -> Result<FilesRead> {
        let files = Arc::clone(&self.files);
        self.run(move |connection| {
            read_only(connection, |connection| {
                for file in files.iter().filter(|file| !file.found.format.is_database()) {
                    let sql = format!(
                        "SELECT * FROM {} LIMIT {SAMPLE_ROWS}",
                        Dialect::DuckDb.quote_ident(&file.found.name)
                    );
                    rows_of(connection, &sql, Vec::new(), SAMPLE_ROWS as usize).map_err(
                        |error| {
                            Error::Connect(format!(
                                "Can't read {}: {}",
                                file_label(&file.found),
                                plain(&error)
                            ))
                        },
                    )?;
                }
                Ok(())
            })?;
            Ok(FilesRead {
                files: files.len(),
                bytes: files.iter().map(|file| file.found.size).sum(),
            })
        })
        .await
    }

    /// The file `object` is read from, where it is a file's table.
    fn file_of(&self, object: &ObjectRef) -> Option<&Opened> {
        (object.schema == MAIN)
            .then(|| {
                self.files
                    .iter()
                    .find(|file| file.found.name == object.name)
            })
            .flatten()
            .filter(|file| !file.found.format.is_database())
    }
}

/// A file's name, for a message.
fn file_label(file: &Found) -> String {
    file.path
        .file_name()
        .map_or_else(|| file.name.clone(), |name| name.to_string_lossy().into())
}

/// An error's first line, without what `Error`'s words add: DuckDB follows
/// a message with the statement it was in, which a line of the dialog has
/// no room for.
fn plain(error: &Error) -> String {
    let words = match error {
        Error::Query { message, .. } => message.clone(),
        other => other.to_string(),
    };
    words
        .lines()
        .next()
        .unwrap_or_default()
        .trim_end()
        .to_owned()
}

impl Adapter for Conn {
    /// Never: the files are read here, not over a network.
    fn is_encrypted(&self) -> bool {
        false
    }

    fn cancel_handle(&self) -> CancelHandle {
        CancelHandle(CancelInner::Files(Arc::clone(&self.interrupt)))
    }

    async fn server_version(&self) -> Result<String> {
        Ok(self.version.clone())
    }

    /// DuckDB is built without its time zones: a moment with an offset is
    /// read and shown in UTC.
    async fn session_zone(&self) -> Result<Option<String>> {
        Ok(Some("UTC".into()))
    }

    /// None: a connection's files are one database.
    async fn list_databases(&self) -> Result<Vec<String>> {
        Ok(Vec::new())
    }

    /// `main`, which holds the files' tables, then each `.duckdb` file.
    async fn list_schemas(&self) -> Result<Vec<String>> {
        Ok(std::iter::once(MAIN.to_owned())
            .chain(
                self.files
                    .iter()
                    .filter(|file| file.found.format.is_database())
                    .map(|file| file.found.name.clone()),
            )
            .take(MAX_LISTED as usize)
            .collect())
    }

    /// In `main`, a table for each file, in the files' order; in a
    /// `.duckdb` file's schema, its tables and views by name.
    async fn list_objects(&self, schema: &str) -> Result<Vec<ObjectInfo>> {
        if schema == MAIN {
            return Ok(self
                .files
                .iter()
                .filter(|file| !file.found.format.is_database())
                .map(|file| ObjectInfo {
                    name: file.found.name.clone(),
                    kind: ObjectKind::Table,
                    estimated_rows: file.rows,
                })
                .take(MAX_LISTED as usize)
                .collect());
        }
        let catalog = schema.to_owned();
        self.run(move |connection| {
            read_only(connection, |connection| {
                let mut statement = connection
                    .prepare(&format!(
                        "SELECT table_name, table_type FROM information_schema.tables \
                         WHERE table_catalog = ? AND table_schema = 'main' \
                         ORDER BY table_name LIMIT {MAX_LISTED}"
                    ))
                    .map_err(map_error)?;
                let objects = statement
                    .query_map([catalog.as_str()], |row| {
                        let kind: String = row.get(1)?;
                        Ok(ObjectInfo {
                            name: row.get(0)?,
                            kind: match kind.as_str() {
                                "VIEW" => ObjectKind::View,
                                _ => ObjectKind::Table,
                            },
                            estimated_rows: None,
                        })
                    })
                    .map_err(map_error)?
                    .collect::<std::result::Result<Vec<_>, _>>()
                    .map_err(map_error)?;
                Ok(objects)
            })
        })
        .await
    }

    async fn describe(&self, object: &ObjectRef) -> Result<Structure> {
        let file = self.file_of(object).map(|file| FileFacts {
            path: file.found.path.clone(),
            size: file.found.size,
            format: file.found.format,
            options: reader_options(file.found.format).to_owned(),
        });
        let object = object.clone();
        self.run(move |connection| {
            let columns = read_only(connection, |connection| columns(connection, &object))?;
            if columns.is_empty() {
                return Err(Error::query(format!(
                    "no such table or view: {}",
                    object.name
                )));
            }
            Ok(Structure {
                columns,
                file,
                ..Structure::default()
            })
        })
        .await
    }

    /// The statement DuckDB keeps of the view a file is read through, or
    /// of a `.duckdb` file's table or view.
    async fn ddl(&self, object: &ObjectRef) -> Result<Ddl> {
        use crate::ddl::{Part, PartKind};
        let version = self.version.clone();
        let ddl = move |statement: String| {
            Ddl::new(
                crate::Driver::Files,
                version.clone(),
                None,
                vec![Part {
                    kind: PartKind::Object,
                    statements: vec![statement],
                }],
            )
        };
        if let Some(file) = self.file_of(object) {
            return Ok(ddl(file.made.clone()));
        }
        let object = object.clone();
        self.run(move |connection| {
            let (catalog, schema) = catalog_of(&object);
            let sql: Option<String> = read_only(connection, |connection| {
                connection
                    .query_row(
                        "SELECT sql FROM duckdb_views() \
                         WHERE database_name = ? AND schema_name = ? AND view_name = ? \
                         UNION ALL \
                         SELECT sql FROM duckdb_tables() \
                         WHERE database_name = ? AND schema_name = ? AND table_name = ? \
                         LIMIT 1",
                        [catalog, schema, &object.name, catalog, schema, &object.name],
                        |row| row.get(0),
                    )
                    .map(Some)
                    .or_else(|error| match error {
                        duckdb::Error::QueryReturnedNoRows => Ok(None),
                        error => Err(map_error(error)),
                    })
            })?;
            let Some(sql) = sql else {
                return Err(Error::query(format!(
                    "no such table or view: {}",
                    object.name
                )));
            };
            Ok(ddl(sql.trim_end().trim_end_matches(';').to_owned()))
        })
        .await
    }

    async fn fetch_rows(&self, query: &RowQuery) -> Result<RowPage> {
        check_raw_where(query)?;
        let query = query.clone();
        let limit = query.limit as usize;
        self.run(move |connection| {
            read_only(connection, |connection| {
                let catalog = columns(connection, &query.object)?;
                let binary = binary_of(&catalog);
                // A file has no key: rows come in the file's order.
                let sql = Dialect::DuckDb.select_rows(&query, &[], &binary);
                let started = Instant::now();
                let params = sql.params.iter().map(to_duckdb).collect();
                let (mut columns, rows, has_more) = rows_of(connection, &sql.text, params, limit)?;
                // The catalog's types, which DuckDB's Arrow export does not
                // all keep: a JSON column is text to Arrow.
                for meta in &mut columns {
                    if let Some(info) = catalog.iter().find(|info| info.name == meta.name) {
                        meta.type_name.clone_from(&info.type_name);
                        if crate::column_class(Dialect::DuckDb, &info.type_name)
                            == crate::ColumnClass::Json
                        {
                            meta.kind = ValueKind::Json;
                        }
                    }
                }
                Ok(RowPage {
                    columns,
                    rows,
                    has_more,
                    ordered_by_key: false,
                    elapsed: started.elapsed(),
                })
            })
        })
        .await
    }

    async fn count_rows(&self, query: &RowQuery) -> Result<u64> {
        check_raw_where(query)?;
        let query = query.clone();
        self.run(move |connection| {
            read_only(connection, |connection| {
                let binary = binary_columns(connection, &query.object)?;
                let sql = Dialect::DuckDb.count_rows(&query, &binary);
                let count: i64 = connection
                    .query_row(
                        &sql.text,
                        duckdb::params_from_iter(sql.params.iter().map(to_duckdb)),
                        |row| row.get(0),
                    )
                    .map_err(map_error)?;
                Ok(u64::try_from(count).unwrap_or(0))
            })
        })
        .await
    }

    async fn lookup(&self, lookup: &Lookup) -> Result<LookupPage> {
        let lookup = lookup.clone();
        self.run(move |connection| {
            read_only(connection, |connection| {
                let sql = Dialect::DuckDb.lookup(&lookup);
                let most = lookup.limit as usize + 1;
                let params = sql.params.iter().map(to_duckdb).collect();
                let (_, rows, _) = rows_of(connection, &sql.text, params, most)?;
                crate::lookup::page(rows, &lookup)
            })
        })
        .await
    }

    /// See [`crate::Connection::run_script`]. One blocking job; a thread
    /// beside it interrupts the statement when `stop` is set.
    async fn run_script(
        &self,
        texts: Vec<String>,
        limit: u32,
        mode: ScriptMode,
        stop: &StopFlag,
    ) -> Result<ScriptOutcome> {
        match mode {
            ScriptMode::ReadOnly => {}
            // Editing files is not built: a Files session never writes.
            ScriptMode::Write => return Err(Error::ReadOnly),
        }
        let stop = stop.clone();
        let interrupt = Arc::clone(&self.interrupt);
        self.run(move |connection| {
            let watch = Watch::start(interrupt, stop.clone(), None);
            script(connection, &texts, limit as usize, &stop, &[], watch)
        })
        .await
    }

    /// See [`crate::Connection::run_agent_read`]. A Files connection keeps
    /// no hidden tables and no masked columns: where `sources` ask for
    /// either, every read is refused.
    async fn run_read(
        &self,
        read: &Read,
        limits: Limits,
        sources: &Sources,
        stop: &StopFlag,
    ) -> Result<(ScriptOutcome, Vec<bool>)> {
        if sources.guards_anything() {
            return Err(Error::Agent(Refusal::NotKeptOnFiles));
        }
        let stop = stop.clone();
        let interrupt = Arc::clone(&self.interrupt);
        let texts = vec![read.text().to_owned()];
        let params = read.params().to_vec();
        self.run(move |connection| {
            let deadline = Instant::now().checked_add(limits.timeout);
            let watch = Watch::start(interrupt, stop.clone(), deadline);
            let outcome = script(
                connection,
                &texts,
                limits.rows as usize,
                &stop,
                &params,
                watch,
            )?;
            Ok((outcome, Vec::new()))
        })
        .await
    }

    /// Nothing to read: nothing is hidden or masked here (see
    /// `run_read`).
    async fn agents_catalog(&self, _masked: &[ColumnName]) -> Result<Catalog> {
        Ok(Catalog::default())
    }

    /// Editing files is not built.
    async fn write(&self, _changes: &ChangeSet, _stop: &StopFlag) -> Result<WriteOutcome> {
        Err(Error::ReadOnly)
    }
}
