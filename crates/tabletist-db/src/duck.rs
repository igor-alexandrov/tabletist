//! Files connections: DuckDB in the app's process, in memory, with a view
//! for each file and each `.duckdb` file attached as a schema. DuckDB is
//! blocking, so every call runs on tokio's blocking pool, as SQLite's do.
//!
//! The session is fenced twice. DuckDB's own configuration, set once the
//! views are made and locked so no SQL can set it back: it may open the
//! connection's files and no other, which also keeps it from writing a
//! file, installing or loading an extension. And a read-only transaction
//! around every page, count, lookup, script and agent's read.
//!
//! A session that may write saves edits to the files (`edit`): only a
//! save writes, from Tabletist's own statements, which no script can run.
//! Its fence also lets DuckDB write the hidden file each save of a
//! Parquet, JSON or NDJSON file goes to first, and no other.
//!
//! Saved PostgreSQL, MySQL and SQLite connections it is told to attach are
//! attached read-only before the fence (`attach`), and stay reachable
//! after it: the fence keeps any other from being attached.

mod attach;
mod database;
mod edit;

use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use duckdb::arrow::array::{Array, AsArray, RecordBatch};
use duckdb::arrow::datatypes::{DataType, Field, Float64Type, Int64Type, UInt64Type};
use duckdb::arrow::util::display::{ArrayFormatter, FormatOptions};

use crate::adapter::Adapter;
use crate::agent::{Catalog, ColumnName, Limits, Param, Read, Refusal, Sources};
use crate::files::{
    Attaching, Change, Downloading, Format, Found, Header, Reader, Reading, SAMPLE_ROWS, Source,
    Types,
};
use crate::{
    Access, CancelHandle, CancelInner, ChangeSet, ColumnClass, ColumnInfo, ColumnMeta, Ddl,
    Dialect, Error, Lookup, LookupPage, MAX_LISTED, ObjectInfo, ObjectKind, ObjectRef, Result,
    RowPage, RowQuery, ScriptMode, ScriptOutcome, StatementOutcome, StatementResult, StopFlag,
    Structure, Value, ValueKind, WriteOutcome,
};

/// The schema a connection's files are tables of.
pub(crate) const MAIN: &str = "main";

/// What a save finds a row of a file's table by, as a [`crate::RowChange`]'s
/// key names it: the row's place in the file, from 0, as the page's
/// `positions` say it.
pub const FILE_POSITION: &str = "__tabletist_position";

/// The other part of that key: the copy of the file the place is of, as
/// the page's `positions` say it. No column holds it: a save checks it.
pub const FILE_COPY: &str = "__tabletist_copy";

/// DuckDB's name for the in-memory database the views are in.
const MEMORY: &str = "memory";

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
    /// The connection's files, `.duckdb` ones included, those that failed
    /// too.
    pub files: usize,
    /// Their size on disk, in bytes.
    pub bytes: u64,
    /// The files it could not read, in the connection's order.
    pub failed: Vec<crate::files::Failure>,
    /// The files read whose whole is checked next, in the background: the
    /// CSV, TSV and NDJSON files, by their tables' names.
    pub checks: Vec<String>,
}

/// What the Structure view says of the file a table is read from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileFacts {
    pub path: std::path::PathBuf,
    pub size: u64,
    pub format: Format,
    /// How the reader is told to read it, as the view's call says it, the
    /// connection's Reading CSV and the file's overrides included:
    /// `auto_detect = true, sample_size = 20000`.
    pub options: String,
}

/// Why a table of a Files connection is not edited, though its session
/// may write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unedited {
    /// An Excel file's: Excel files are never written.
    Excel,
    /// An attached connection's: those are attached read-only.
    Attached,
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
    /// The files a Test's session could not open, left out of it. A
    /// session that browses has none: it does not open with one.
    failed: Vec<crate::files::Failure>,
    /// Every file of the connection's, in its order: those it could not
    /// open among them.
    order: Vec<std::path::PathBuf>,
    /// The saved connections it attached, in order.
    attached: Arc<Vec<Attached>>,
    version: String,
    /// Whether a save may write the files.
    access: Access,
    /// How its CSV files are read, which a save writes them back by.
    reading: Reading,
    /// The files' tables read for editing so far, by name: those of a
    /// session that may write whose page was read.
    copies: Arc<Mutex<HashMap<String, edit::Loaded>>>,
    /// Its `.duckdb` files, by their paths: each call holds them for
    /// reading, and a save one alone.
    databases: Arc<Vec<database::Held>>,
}

/// A saved connection the session attached: the name SQL calls it by,
/// which the sidebar lists it as, and the schema of its own whose tables
/// are listed there and that `name.table` reads.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Attached {
    name: String,
    schema: String,
}

/// A file as the session opened it.
#[derive(Debug, Clone)]
struct Opened {
    found: Found,
    /// The statement that made its view, which the DDL view shows: DuckDB
    /// keeps it rewritten, each option in parentheses.
    made: String,
    /// The reader's options in that statement, as the Structure view says
    /// them.
    options: String,
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

/// A path as a reader is given it: the readers take `*`, `?` and `[` as a
/// pattern, so each is written as a class of itself, and the path names
/// that one file. Without it `report[1].csv` reads `report1.csv`.
fn reader_path(path: &Path) -> Result<String> {
    let mut text = String::new();
    for char in path_text(path)?.chars() {
        match char {
            '*' | '?' | '[' => text.extend(['[', char, ']']),
            char => text.push(char),
        }
    }
    Ok(text)
}

/// The reader's options for a file of `format`, as its call writes them
/// after the path: a CSV's and a TSV's from the connection's `reading`,
/// with `text` the columns read as text; the other formats' fixed.
fn reader_options(format: Format, reading: &Reading, text: &[&str]) -> String {
    match format {
        Format::Csv | Format::Tsv => {
            let mut options = Vec::new();
            match reading.header {
                Header::Detect => {}
                Header::Yes => options.push("header = true".to_owned()),
                Header::No => options.push("header = false".to_owned()),
            }
            options.push("auto_detect = true".to_owned());
            options.push(match reading.types {
                Types::Sample => format!("sample_size = {SAMPLE_ROWS}"),
                Types::All => "sample_size = -1".to_owned(),
                Types::Text => "all_varchar = true".to_owned(),
            });
            // A TSV is tab-separated by its name.
            let delimiter = match format {
                Format::Tsv => Some('\t'),
                _ => reading.delimiter.char(),
            };
            match delimiter {
                None => {}
                Some('\t') => options.push("delim = '\\t'".to_owned()),
                Some(char) => options.push(format!("delim = {}", literal(&char.to_string()))),
            }
            if !text.is_empty() {
                let types: Vec<String> = text
                    .iter()
                    .map(|column| format!("{}: 'VARCHAR'", literal(column)))
                    .collect();
                options.push(format!("types = {{{}}}", types.join(", ")));
            }
            options.join(", ")
        }
        Format::Json => "format = 'array'".to_owned(),
        Format::Ndjson => "format = 'newline_delimited'".to_owned(),
        Format::Excel => "header = true".to_owned(),
        Format::Parquet | Format::DuckDb => String::new(),
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
/// its name bare where that is safe (the DDL view shows it as it is), and
/// the reader's options in it. A CSV's or TSV's overrides of a column
/// are kept only where the file has that column, which a `DESCRIBE` of
/// the call without them says, with the type found for each.
fn make(
    connection: &duckdb::Connection,
    file: &Found,
    reading: &Reading,
) -> Result<(String, String)> {
    let name = Dialect::DuckDb.ident(&file.name);
    let format = file.format;
    if format == Format::DuckDb {
        let statement = format!(
            "ATTACH {} AS {name} (READ_ONLY)",
            literal(&path_text(&file.path)?)
        );
        return Ok((statement, String::new()));
    }
    let path = literal(&reader_path(&file.path)?);
    let call = |options: &str| match options {
        "" => format!("{}({path})", reader(format)),
        options => format!("{}({path}, {options})", reader(format)),
    };
    // The file's changes of a column. Values read as empty that are none
    // change nothing.
    let changes: Vec<&Change> = match format {
        Format::Csv | Format::Tsv => reading
            .overrides_of(&file.path)
            .filter(|change| match change {
                Change::Text { .. } => true,
                Change::Empty { values, .. } => !values.is_empty(),
                Change::Skip => false,
            })
            .collect(),
        Format::Parquet | Format::Json | Format::Ndjson | Format::Excel | Format::DuckDb => {
            Vec::new()
        }
    };
    let plain = reader_options(format, reading, &[]);
    if changes.is_empty() {
        let statement = format!("CREATE VIEW {name} AS SELECT * FROM {}", call(&plain));
        return Ok((statement, plain));
    }
    // The file's columns, as the call without the overrides finds them.
    // An override of a column the file does not have changes nothing.
    let found = described(connection, &call(&plain))?;
    // A column as the file names it, matched in any case, as DuckDB
    // matches names.
    let column_of = |column: &str| {
        found
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case(column))
    };
    let same = |a: &str, b: &str| a.eq_ignore_ascii_case(b);
    // Read as text: the text overrides' columns and the columns whose
    // values are read as empty, which are cast back below unless they are
    // read as text too.
    let mut text: Vec<&str> = Vec::new();
    for change in &changes {
        if let Some((column, _)) = change.column().and_then(column_of)
            && !text.contains(&column.as_str())
        {
            text.push(column);
        }
    }
    let mut replaced = Vec::new();
    let mut cast: Vec<&str> = Vec::new();
    for change in &changes {
        let Change::Empty { column, .. } = change else {
            continue;
        };
        let Some((column, _)) = column_of(column) else {
            continue;
        };
        let read_as_text = changes
            .iter()
            .any(|other| matches!(other, Change::Text { column: other } if same(other, column)));
        if read_as_text || cast.contains(&column.as_str()) {
            continue;
        }
        cast.push(column);
        // Every override of the column, its values in one cast.
        let values: Vec<&String> = changes
            .iter()
            .flat_map(|other| match other {
                Change::Empty {
                    column: other,
                    values,
                } if same(other, column) => values.as_slice(),
                Change::Text { .. } | Change::Empty { .. } | Change::Skip => &[],
            })
            .collect();
        // The type the column is found to have, and the format its dates
        // are written in, while these values (and DuckDB's own empty
        // string) are taken as empty: the column's own values alone, as
        // DuckDB's `nullstr` is the whole file's and only this column's
        // type is taken from it.
        let mut empty = vec![literal("")];
        empty.extend(values.iter().map(|value| literal(value)));
        // The columns named as the call without the overrides names them:
        // a value read as empty that is a header's word would have DuckDB
        // name its column anew, and one name could then be another's.
        let names: Vec<String> = found.iter().map(|(name, _)| literal(name)).collect();
        let sniffed = format!(
            "{plain}, nullstr = [{}], names = [{}]",
            empty.join(", "),
            names.join(", ")
        );
        let kind = described(connection, &call(&sniffed))?
            .into_iter()
            .find(|(name, _)| name == column)
            .map_or_else(|| "VARCHAR".to_owned(), |(_, kind)| kind);
        let quoted = Dialect::DuckDb.ident(column).into_owned();
        let value = values.iter().fold(quoted.clone(), |inner, value| {
            format!("NULLIF({inner}, {})", literal(value))
        });
        let back = match kind.as_str() {
            "VARCHAR" => value,
            "DATE" | "TIMESTAMP" => match written(connection, &path, &sniffed, &kind)? {
                // Written as the sniffer found it, `31/01/2026`, which a
                // cast does not read.
                Some(format) => format!("CAST(strptime({value}, {}) AS {kind})", literal(&format)),
                None => format!("CAST({value} AS {kind})"),
            },
            _ => format!("CAST({value} AS {kind})"),
        };
        replaced.push(format!("{back} AS {quoted}"));
    }
    let options = reader_options(format, reading, &text);
    let columns = match replaced.is_empty() {
        true => "*".to_owned(),
        false => format!("* REPLACE ({})", replaced.join(", ")),
    };
    let statement = format!(
        "CREATE VIEW {name} AS SELECT {columns} FROM {}",
        call(&options)
    );
    Ok((statement, options))
}

/// The format the sniffer found a CSV's dates (`DATE`) or timestamps
/// (`TIMESTAMP`) written in, read with `options`: `%d/%m/%Y`. None where
/// they are written as a cast reads them (ISO, year first). A sniff that
/// fails is the file's error: a view cast without the format would fail
/// only once its rows are read.
fn written(
    connection: &duckdb::Connection,
    path: &str,
    options: &str,
    kind: &str,
) -> Result<Option<String>> {
    let field = match kind {
        "DATE" => "DateFormat",
        _ => "TimestampFormat",
    };
    let format = connection
        .query_row(
            &format!("SELECT {field} FROM sniff_csv({path}, {options})"),
            [],
            |row| row.get::<_, Option<String>>(0),
        )
        .map_err(map_error)?;
    // ISO dates, `2026-01-31` and on, a cast reads as it is.
    Ok(format.filter(|format| !format.is_empty() && !format.starts_with("%Y-%m-%d")))
}

/// The columns a reader's call gives, by name, with DuckDB's type of each.
fn described(connection: &duckdb::Connection, call: &str) -> Result<Vec<(String, String)>> {
    let mut statement = connection
        .prepare(&format!("DESCRIBE SELECT * FROM {call}"))
        .map_err(map_error)?;
    let columns = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(map_error)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(map_error)?;
    Ok(columns)
}

/// Rows of a text file by a sample of its start: its lines in the first
/// 256 KiB, scaled to its size, without a CSV's header unless it is read
/// as having none. `None` for a format that is not a line a row, or a file
/// that cannot be read.
fn estimate(file: &Found, header: Header) -> Option<u64> {
    use std::io::Read as _;
    let header = match (file.format, header) {
        (Format::Csv | Format::Tsv, Header::No) => 0,
        (Format::Csv | Format::Tsv, Header::Yes) => 1,
        // Not known: every line is counted rather than one lost.
        (Format::Csv | Format::Tsv, Header::Detect) => 0,
        (Format::Ndjson, _) => 0,
        (Format::Parquet | Format::Json | Format::Excel | Format::DuckDb, _) => return None,
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

/// Whether a CSV's first row names its columns, read with `options`:
/// where DuckDB is left to detect it, its sniffer says from a sample, and
/// the file is then read so (`Detect` where the sniffer cannot say).
fn header_of(
    connection: &duckdb::Connection,
    file: &Found,
    header: Header,
    options: &str,
) -> Header {
    match (file.format, header) {
        (Format::Csv | Format::Tsv, Header::Detect) => {
            let Ok(path) = reader_path(&file.path) else {
                return Header::Detect;
            };
            // From the rows a sample holds: with types from every row, the
            // view's own sniff reads the whole file, and a second would
            // read it again. The view takes the header found here.
            let options =
                options.replace("sample_size = -1", &format!("sample_size = {SAMPLE_ROWS}"));
            let call = match options.as_str() {
                "" => literal(&path),
                options => format!("{}, {options}", literal(&path)),
            };
            match connection.query_row(
                &format!("SELECT HasHeader FROM sniff_csv({call})"),
                [],
                |row| row.get::<_, Option<bool>>(0),
            ) {
                Ok(Some(true)) => Header::Yes,
                Ok(Some(false)) => Header::No,
                Ok(None) | Err(_) => Header::Detect,
            }
        }
        (_, header) => header,
    }
}

/// What [`opened`] made: the session, its files, those it could not open
/// (where it was told to go on past them), the saved connections it
/// attached and DuckDB's version.
type Made = (
    duckdb::Connection,
    Vec<Opened>,
    Vec<crate::files::Failure>,
    Vec<Attached>,
    String,
);

/// Installs and loads `reader` in the session: from the folder it was
/// installed into once, else downloaded from DuckDB's servers, which
/// `downloading` is told of first.
fn install(
    connection: &duckdb::Connection,
    reader: Reader,
    downloading: Option<&Downloading>,
) -> Result<()> {
    // One install at a time in the app: two sessions that both found the
    // reader missing would both download it, and on Windows the second
    // cannot put its copy over the one the first has loaded.
    static INSTALLING: Mutex<()> = Mutex::new(());
    let installed = || -> Result<bool> {
        connection
            .query_row(
                "SELECT coalesce(bool_or(installed), false) FROM duckdb_extensions() \
                 WHERE extension_name = ?",
                [reader.listed()],
                |row| row.get(0),
            )
            .map_err(map_error)
    };
    let extension = reader.extension();
    {
        let _one = INSTALLING
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if !installed()? {
            if let Some(downloading) = downloading {
                downloading(reader);
            }
            let made = connection
                .execute_batch(&format!("INSTALL {extension}"))
                .map_err(map_error);
            // Another Tabletist may have installed it meanwhile.
            if let Err(error) = made
                && !installed()?
            {
                return Err(error);
            }
        }
    }
    connection
        .execute_batch(&format!("LOAD {extension}"))
        .map_err(map_error)
}

/// Attaches `attaching`, reached through a tunnel's local port `via`
/// where it has one, as the session's `at`th: the SQLite file it opens,
/// which the fence lets in, where it is one.
fn attached(
    connection: &duckdb::Connection,
    at: usize,
    attaching: &Attaching,
    via: Option<u16>,
    downloading: Option<&Downloading>,
) -> Result<(Attached, Option<String>)> {
    let name = &attaching.name;
    let reader = attach::reader(attaching)?;
    install(connection, reader, downloading).map_err(|error| {
        Error::Connect(format!(
            "Can't attach {name}: the {} reader for DuckDB could not be installed ({})",
            reader.label(),
            plain(&error)
        ))
    })?;
    let made = attach::statements(at, attaching, via)?;
    if let Some(secret) = &made.secret {
        // Never DuckDB's words: a parser error quotes the statement, and
        // this one holds the password.
        connection.execute_batch(secret).map_err(|_| {
            Error::Connect(format!(
                "Can't attach {name}: DuckDB did not take its login"
            ))
        })?;
    }
    let attached = connection.execute_batch(&made.attach).map_err(|error| {
        let error = map_error(error);
        // The reader's own words: should they ever repeat the password,
        // it is not said.
        let mut words = plain(&error);
        if let Some(password) = attaching.secrets.password.as_deref()
            && !password.is_empty()
        {
            words = words.replace(password, "********");
        }
        Error::Connect(format!("Can't attach {name}: {words}"))
    });
    // The login is let go whether it was taken or not: the attached
    // database keeps its own.
    if let Some(forget) = &made.forget {
        connection.execute_batch(forget).map_err(map_error)?;
    }
    attached?;
    Ok((
        Attached {
            name: name.clone(),
            schema: crate::files::attached_schema(&attaching.spec),
        },
        made.file,
    ))
}

/// Opens a session over `files`, read as `reading` says, a file it skips
/// left out: views and attaches, then the saved connections in
/// `attaching`, each with the tunnel port it is reached through, then the
/// fence. A file that cannot be opened stops it, unless `past` says to go
/// on without it (a Test, which says which failed); a saved connection
/// that cannot be attached always does. `downloading` is told of each
/// reader DuckDB downloads first.
fn opened(
    files: &[Found],
    reading: &Reading,
    past: bool,
    attaching: &[(Attaching, Option<u16>)],
    downloading: Option<&Downloading>,
    writable: bool,
) -> Result<Made> {
    let files: Vec<&Found> = files
        .iter()
        .filter(|file| !reading.skips(&file.path))
        .collect();
    // Held while they are attached: a save another session runs is waited
    // for, and the file attached as it left it.
    let opening = database::Opening::of(
        files
            .iter()
            .filter(|file| file.format.is_database())
            .map(|file| file.path.as_path()),
    );
    let _opening = opening.hold();
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
    // Why the Excel reader is not there, where a Test goes on without it.
    let mut no_excel = None;
    if files.iter().any(|file| file.format == Format::Excel) {
        // Installed once, then found in the folder: offline, a second open
        // still reads Excel files.
        if let Err(error) = install(&connection, Reader::Excel, downloading) {
            let why = format!(
                "the Excel reader for DuckDB could not be installed ({})",
                plain(&error)
            );
            match past {
                true => no_excel = Some(why),
                false => return Err(Error::Connect(format!("Can't read Excel files: {why}"))),
            }
        }
    }
    let mut opened = Vec::with_capacity(files.len());
    let mut failures = Vec::new();
    for file in &files {
        if let (Some(why), Format::Excel) = (&no_excel, file.format) {
            failures.push(crate::files::Failure {
                file: file_label(file),
                path: file.path.clone(),
                format: file.format,
                why: crate::files::Why::Other(why.clone()),
            });
            continue;
        }
        let failed = |error: &Error| {
            Error::Connect(format!("Can't read {}: {}", file_label(file), plain(error)))
        };
        // A CSV whose header DuckDB is left to detect: its sniffer says,
        // from a sample. With types from every row the view's own sniff
        // reads the whole file and may find otherwise, so there the view,
        // its columns' own reads and the estimate are told the header
        // found; elsewhere they read the same rows and find the same.
        let header = header_of(
            &connection,
            file,
            reading.header,
            &reader_options(file.format, reading, &[]),
        );
        let pinned;
        let reading = match reading.types {
            Types::All => {
                pinned = Reading {
                    header,
                    ..reading.clone()
                };
                &pinned
            }
            Types::Sample | Types::Text => reading,
        };
        // A `.duckdb` file is attached read-only, in a session that may
        // write too: a save attaches it to write for as long as it runs
        // (`database`).
        let made = make(&connection, file, reading).and_then(|(statement, options)| {
            run(&statement)?;
            Ok((statement, options))
        });
        let (statement, options) = match made {
            Ok(made) => made,
            Err(error) if past => {
                failures.push(failure(file, &error));
                continue;
            }
            Err(error) => return Err(failed(&error)),
        };
        let rows = match file.format {
            Format::Parquet => connection
                .query_row(
                    &format!(
                        "SELECT sum(num_rows)::BIGINT FROM parquet_file_metadata({})",
                        literal(&reader_path(&file.path)?)
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
            | Format::DuckDb => estimate(file, header),
        };
        opened.push(Opened {
            found: (*file).clone(),
            made: statement,
            options,
            rows,
        });
    }
    // The saved connections, read-only, as every session is.
    let mut joined = Vec::with_capacity(attaching.len());
    let mut attached_files = Vec::new();
    for (at, (attaching, via)) in attaching.iter().enumerate() {
        // A `.duckdb` file of the connection's is a schema of that name.
        let taken = opened
            .iter()
            .any(|file| file.found.format.is_database() && file.found.name == attaching.name);
        if taken {
            return Err(Error::Connect(format!(
                "Can't attach {}: a .duckdb file of this connection is a schema of that name. \
                 Name it another way.",
                attaching.name
            )));
        }
        let (each, file) = attached(&connection, at, attaching, *via, downloading)?;
        joined.push(each);
        attached_files.extend(file);
    }
    // The fence: the files that opened and no other, then no setting
    // changed again. One a Test went on without is not read, and its path
    // may be one DuckDB cannot be given. A reader asks by its own spelling
    // of a file, which names only it. An attached SQLite file too.
    let mut allowed = Vec::with_capacity(opened.len() + attached_files.len());
    for file in &opened {
        let file = &file.found;
        let (path, read) = (path_text(&file.path)?, reader_path(&file.path)?);
        if read != path {
            allowed.push(literal(&read));
        }
        allowed.push(literal(&path));
    }
    allowed.extend(attached_files.iter().map(|path| literal(path)));
    // A session that may write keeps its copies apart from the views, and
    // may write the hidden file each save of a file DuckDB writes goes to
    // first: none other.
    if writable {
        run(&format!("ATTACH ':memory:' AS {}", edit::EDITS))?;
        for file in opened
            .iter()
            .filter(|file| edit::editable(file.found.format))
        {
            allowed.push(literal(&path_text(&edit::temp_path(&file.found.path))?));
        }
    }
    run(&format!("SET allowed_paths = [{}]", allowed.join(", ")))?;
    run("SET enable_external_access = false")?;
    run("SET lock_configuration = true")?;
    let version: String = connection
        .query_row("SELECT version()", [], |row| row.get(0))
        .map_err(map_error)?;
    Ok((
        connection,
        opened,
        failures,
        joined,
        format!("DuckDB {}", version.trim_start_matches('v')),
    ))
}

/// Why `file` could not be read, from DuckDB's words.
fn failure(file: &Found, error: &Error) -> crate::files::Failure {
    crate::files::Failure {
        file: file_label(file),
        path: file.path.clone(),
        format: file.format,
        why: why(error),
    }
}

/// Reads DuckDB's message for what its reader could not take. Its CSV
/// errors say the file's line (the header row is line 1 where the file
/// has one) and its settings, `header = true`, after.
fn why(error: &Error) -> crate::files::Why {
    use crate::files::Why;
    let message = match error {
        Error::Query { message, .. } => message.as_str(),
        _ => return Why::Other(plain(error)),
    };
    // The number that follows `before` in the message.
    let number = |before: &str| -> Option<u64> {
        let at = message.find(before)? + before.len();
        let digits: String = message[at..]
            .trim_start()
            .chars()
            .take_while(char::is_ascii_digit)
            .collect();
        digits.parse().ok()
    };
    // DuckDB's line naming the column and the value, on its own: the
    // message also quotes the file's row, which may hold any words. The
    // pieces end at the last of their closing words, so a quote in a
    // column's name or a value stays in it.
    let converting = message
        .lines()
        .find(|line| line.starts_with("Error when converting column \""));
    let value = converting.and_then(|line| {
        let rest = line.strip_prefix("Error when converting column \"")?;
        let split = rest.rfind("\". Could not convert string \"")?;
        let column = &rest[..split];
        let rest = &rest[split + "\". Could not convert string \"".len()..];
        let to = rest.rfind("\" to '")?;
        let kind = &rest[to + "\" to '".len()..];
        // The value as the file writes it, a quote in it doubled: the
        // text the column holds, which a fix compares, has it once.
        Some((
            column.to_owned(),
            rest[..to].replace("\"\"", "\""),
            kind[..kind.find('\'')?].to_owned(),
        ))
    });
    if let Some(line) = number("CSV Error on Line:") {
        let header = message
            .lines()
            .any(|line| line.trim_start().starts_with("header = true"));
        let row = line.saturating_sub(u64::from(header));
        if let Some((column, value, kind)) = value {
            return Why::Value {
                row,
                column,
                value,
                kind,
            };
        }
        let counts = message.lines().find_map(|line| {
            let rest = line.strip_prefix("Expected Number of Columns:")?;
            let (expected, found) = rest.split_once("Found:")?;
            Some((expected.trim().parse().ok()?, found.trim().parse().ok()?))
        });
        if let Some((expected, found)) = counts {
            return Why::Columns {
                row,
                expected,
                found,
            };
        }
    }
    // DuckDB says the line after a malformed one.
    if message.contains("Malformed JSON")
        && let Some(line) = number(" in line ")
    {
        return Why::Json {
            line: line.saturating_sub(1).max(1),
            malformed: true,
        };
    }
    if message.contains("JSON transform error")
        && let Some(line) = number(" in line ")
    {
        return Why::Json {
            line,
            malformed: false,
        };
    }
    Why::Other(plain(error))
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
/// The key `object`'s rows are read in, as a page reads them: a `.duckdb`
/// file's table's primary key (`database` is where the session holds the
/// file), none elsewhere.
fn key_of(
    connection: &duckdb::Connection,
    object: &ObjectRef,
    database: Option<usize>,
) -> Result<Vec<String>> {
    Ok(match database {
        Some(_) => {
            database::table_key(connection, &object.schema, &object.name)?.unwrap_or_default()
        }
        None => Vec::new(),
    })
}

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
/// in-memory database's `main`, a `.duckdb` file's table in its own, an
/// attached connection's in its schema the sidebar lists.
fn catalog_of<'a>(object: &'a ObjectRef, attached: &'a [Attached]) -> (&'a str, &'a str) {
    match object.schema.as_str() {
        MAIN => (MEMORY, MAIN),
        schema => match attached.iter().find(|each| each.name == schema) {
            Some(each) => (&each.name, &each.schema),
            None => (schema, MAIN),
        },
    }
}

/// The columns of `object`, as DuckDB's catalog has them.
fn columns(
    connection: &duckdb::Connection,
    object: &ObjectRef,
    attached: &[Attached],
) -> Result<Vec<ColumnInfo>> {
    let (catalog, schema) = catalog_of(object, attached);
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
fn binary_columns(
    connection: &duckdb::Connection,
    object: &ObjectRef,
    attached: &[Attached],
) -> Result<Vec<(String, ColumnClass)>> {
    Ok(binary_of(&columns(connection, object, attached)?))
}

/// The columns of `columns` that hold bytes, with their class: what the
/// dialect writes a filter on bytes from.
fn binary_of(columns: &[ColumnInfo]) -> Vec<(String, ColumnClass)> {
    columns
        .iter()
        .filter(|column| {
            crate::column_class(Dialect::DuckDb, &column.type_name) == ColumnClass::Binary
        })
        .map(|column| (column.name.clone(), ColumnClass::Binary))
        .collect()
}

/// The catalog's types over the columns DuckDB's Arrow export names, which
/// it does not all keep: a JSON column is text to Arrow.
fn catalog_types(columns: &mut [ColumnMeta], catalog: &[ColumnInfo]) {
    for meta in columns {
        if let Some(info) = catalog.iter().find(|info| info.name == meta.name) {
            meta.type_name.clone_from(&info.type_name);
            if crate::column_class(Dialect::DuckDb, &info.type_name) == crate::ColumnClass::Json {
                meta.kind = ValueKind::Json;
            }
        }
    }
}

/// Sends a stream's rows to `to`: its columns, with `catalog`'s types for
/// a table's, then batches of `batch` as DuckDB's chunks come. A stop ends
/// it as [`Error::Cancelled`] at a full batch or before the last one.
fn send_chunks(
    chunks: duckdb::ArrowStream<'_>,
    catalog: &[ColumnInfo],
    batch: usize,
    to: &tokio::sync::mpsc::Sender<crate::Streamed>,
    stop: &StopFlag,
) -> Result<u64> {
    let mut metas: Vec<ColumnMeta> = chunks
        .get_schema()
        .fields()
        .iter()
        .map(|field| column_meta(field))
        .collect();
    catalog_types(&mut metas, catalog);
    crate::stream::send_blocking(to, crate::Streamed::Columns(metas))?;
    let mut sent = 0u64;
    let mut values: Vec<Vec<Value>> = Vec::with_capacity(batch);
    for chunk in chunks {
        let chunk = chunk.map_err(map_error)?;
        for row in batch_rows(&chunk)? {
            values.push(row);
            if values.len() == batch {
                if stop.is_stopped() {
                    return Err(Error::Cancelled);
                }
                let full = std::mem::replace(&mut values, Vec::with_capacity(batch));
                sent += full.len() as u64;
                crate::stream::send_blocking(to, crate::Streamed::Rows(full))?;
            }
        }
    }
    if stop.is_stopped() {
        return Err(Error::Cancelled);
    }
    if !values.is_empty() {
        sent += values.len() as u64;
        crate::stream::send_blocking(to, crate::Streamed::Rows(values))?;
    }
    stop.finish();
    Ok(sent)
}

impl Conn {
    /// Opens `source`'s files, read-only: every one is read far enough to
    /// know its columns, and an error names the first that cannot be. A
    /// Test goes on `past` them: those it cannot open or look at, and a
    /// listed file that is gone, are its failures. The saved connections
    /// in `attaching` are attached after, each through the tunnel port
    /// beside it where it has one; `downloading` is told of each reader
    /// DuckDB downloads first. A session whose `access` may write saves
    /// edits to the files a save writes back.
    pub(crate) async fn open(
        source: &Source,
        reading: &Reading,
        past: bool,
        attaching: Vec<(Attaching, Option<u16>)>,
        downloading: Option<Downloading>,
        access: Access,
    ) -> Result<Self> {
        let (source, reading) = (source.clone(), reading.clone());
        let kept = reading.clone();
        let writable = access == Access::Writable;
        let made = tokio::task::spawn_blocking(move || {
            // A skipped file is not looked at: one that is gone, or that
            // cannot be looked at, is no matter.
            let scan = crate::files::scan_past(&source, |file| reading.skips(file), past)?;
            // A skipped file that is gone is no matter. Another fails the
            // session, or, for a Test, is one of the files it cannot read.
            let gone: Vec<&std::path::PathBuf> = scan
                .missing
                .iter()
                .filter(|gone| !reading.skips(gone))
                .collect();
            if let (Some(gone), false) = (gone.first(), past) {
                return Err(Error::Connect(format!(
                    "Can't open {}: no such file",
                    crate::files::shown(gone)
                )));
            }
            // A list's own order, a gone file in its place; a folder's as
            // it was read.
            let order = match &source {
                Source::Files(listed) => listed.clone(),
                Source::Folder(_) => {
                    let mut order: Vec<_> = scan
                        .files
                        .iter()
                        .map(|file| file.path.clone())
                        .chain(scan.unreadable.iter().map(|(path, _)| path.clone()))
                        .collect();
                    order.sort_by_cached_key(|path| crate::files::by_name(path));
                    order
                }
            };
            opened(
                &scan.files,
                &reading,
                past,
                &attaching,
                downloading.as_ref(),
                writable,
            )
            .map(|(connection, files, mut failed, attached, version)| {
                failed.extend(gone.iter().filter_map(|gone| {
                    Some(crate::files::Failure {
                        file: crate::files::file_name(gone),
                        path: (*gone).clone(),
                        format: Format::of(gone)?,
                        why: crate::files::Why::Other("no such file".into()),
                    })
                }));
                // Those it could not look at, with why.
                failed.extend(scan.unreadable.iter().filter_map(|(path, why)| {
                    Some(crate::files::Failure {
                        file: crate::files::file_name(path),
                        path: path.clone(),
                        format: Format::of(path)?,
                        why: crate::files::Why::Other(why.clone()),
                    })
                }));
                (connection, files, failed, attached, version, order)
            })
        })
        .await
        .map_err(|error| Error::Io(error.to_string()))??;
        let (connection, files, failed, attached, version, order) = made;
        let interrupt = connection.interrupt_handle();
        let mut databases: Vec<database::Held> = files
            .iter()
            .filter(|file| file.found.format.is_database())
            .map(|file| database::Held::new(&file.found.name, &file.found.path))
            .collect();
        databases.sort_by(|one, other| one.own().cmp(other.own()));
        Ok(Self {
            inner: Arc::new(Mutex::new(connection)),
            interrupt,
            running: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            calls: std::sync::atomic::AtomicU64::new(0),
            files: Arc::new(files),
            failed,
            order,
            attached: Arc::new(attached),
            version,
            access,
            reading: kept,
            copies: Arc::new(Mutex::new(HashMap::new())),
            databases: Arc::new(databases),
        })
    }

    /// The file `object` is read from, where a save may write it back.
    fn editing(&self, object: &ObjectRef) -> Option<Opened> {
        (self.access == Access::Writable)
            .then(|| self.file_of(object))
            .flatten()
            .filter(|file| edit::editable(file.found.format))
            .cloned()
    }

    /// Why `object` is not edited in a session that may write, where its
    /// connection says so.
    fn unedited(&self, object: &ObjectRef) -> Option<Unedited> {
        if self.access != Access::Writable {
            return None;
        }
        if let Some(file) = self.file_of(object) {
            return (file.found.format == Format::Excel).then_some(Unedited::Excel);
        }
        self.attached
            .iter()
            .any(|each| each.name == object.schema)
            .then_some(Unedited::Attached)
    }

    /// Where `object` is a table or a view of one of the session's
    /// `.duckdb` files, which of `databases` it is.
    fn database_of(&self, object: &ObjectRef) -> Option<usize> {
        self.databases
            .iter()
            .position(|held| held.name == object.schema)
    }

    /// Runs `work` on the blocking pool with the session, holding its
    /// `.duckdb` files for reading.
    async fn run<T: Send + 'static>(
        &self,
        work: impl FnOnce(&duckdb::Connection) -> Result<T> + Send + 'static,
    ) -> Result<T> {
        self.run_gated(true, work).await
    }

    /// Runs `work` as [`Conn::run`] does; `reading` says whether it holds
    /// the `.duckdb` files for reading meanwhile. A save holds its own
    /// alone instead, and reads no other.
    async fn run_gated<T: Send + 'static>(
        &self,
        reading: bool,
        work: impl FnOnce(&duckdb::Connection) -> Result<T> + Send + 'static,
    ) -> Result<T> {
        let databases = Arc::clone(&self.databases);
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
            // Held first: a save may keep the call waiting, and a call
            // given up on meanwhile is asked about after.
            let _held = match reading {
                true => database::reading(&connection, &databases),
                false => Vec::new(),
            };
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
    /// file. A file it cannot read is a failure, with those the session
    /// could not open; the others are read. Which files to check in full
    /// next come with it.
    pub(crate) async fn read_files(&self) -> Result<FilesRead> {
        let files = Arc::clone(&self.files);
        let mut failed = self.failed.clone();
        let order = self.order.clone();
        // The files it could not open count, and their size on disk.
        let unopened = (
            self.failed.len(),
            self.failed
                .iter()
                .map(|failure| std::fs::metadata(&failure.path).map_or(0, |meta| meta.len()))
                .sum::<u64>(),
        );
        self.run(move |connection| {
            let mut read = Vec::new();
            // Each in a transaction of its own: one that fails ends its
            // transaction, and the next file is read in a new one.
            for file in files.iter().filter(|file| !file.found.format.is_database()) {
                let sql = format!(
                    "SELECT * FROM {} LIMIT {SAMPLE_ROWS}",
                    Dialect::DuckDb.quote_ident(&file.found.name)
                );
                match read_only(connection, |connection| {
                    rows_of(connection, &sql, Vec::new(), SAMPLE_ROWS as usize)
                }) {
                    Ok(_) => read.push(file),
                    Err(Error::Cancelled) => return Err(Error::Cancelled),
                    Err(error) => failed.push(failure(&file.found, &error)),
                }
            }
            // In the connection's order, those it could not open among them.
            failed.sort_by_key(|failure| {
                order
                    .iter()
                    .position(|path| *path == failure.path)
                    .unwrap_or(usize::MAX)
            });
            let checks = read
                .iter()
                .filter(|file| {
                    matches!(
                        file.found.format,
                        Format::Csv | Format::Tsv | Format::Ndjson
                    )
                })
                .map(|file| file.found.name.clone())
                .collect();
            Ok(FilesRead {
                files: files.len() + unopened.0,
                bytes: files.iter().map(|file| file.found.size).sum::<u64>() + unopened.1,
                failed,
                checks,
            })
        })
        .await
    }

    /// Reads the whole of the file whose table is `name`, every column of
    /// every row, as a Test's check in the background does: `None` when it
    /// reads to the end, or why it stops.
    pub(crate) async fn check(&self, name: &str) -> Result<Option<crate::files::Failure>> {
        let Some(file) = self
            .files
            .iter()
            .find(|file| file.found.name == name)
            .cloned()
        else {
            return Ok(None);
        };
        self.run(move |connection| {
            // Each column counted: a column read is a column converted,
            // where `count(*)` would read none.
            let sql = format!(
                "SELECT count(COLUMNS(*)) FROM {}",
                Dialect::DuckDb.quote_ident(&file.found.name)
            );
            read_only(connection, |connection| {
                match rows_of(connection, &sql, Vec::new(), 1) {
                    Ok(_) => Ok(None),
                    Err(Error::Cancelled) => Err(Error::Cancelled),
                    Err(error) => Ok(Some(failure(&file.found, &error))),
                }
            })
        })
        .await
    }

    /// `error` from reading `object`, naming the file it could not be read
    /// from where `object` is a file's table, DuckDB's words name that
    /// table's own file, and they say a row, a value or a line of it. A
    /// WHERE line may read another file's table, whose failure is not this
    /// table's to mend. Any other error is left as it is.
    fn of_file(&self, object: &ObjectRef, mut error: Error) -> Error {
        let Some(file) = self.file_of(object) else {
            return error;
        };
        let failure = failure(&file.found, &error);
        let Error::Query { message, named, .. } = &mut error else {
            return error;
        };
        let own = [path_text(&file.found.path), reader_path(&file.found.path)];
        let its_own = named_file(message).is_some_and(|named| {
            own.iter()
                .any(|own| own.as_deref().is_ok_and(|own| own == named))
        });
        if its_own && !matches!(failure.why, crate::files::Why::Other(_)) {
            named.file = Some(failure);
        }
        error
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

/// The file DuckDB's words say it could not read, as it was given it:
/// a CSV reader's `file = …` option line, or a JSON reader's `in file "…"`.
fn named_file(message: &str) -> Option<&str> {
    let option = message
        .lines()
        .find_map(|line| line.trim_start().strip_prefix("file = "));
    if let Some(path) = option {
        return Some(path.trim_end());
    }
    let (_, after) = message.split_once("in file \"")?;
    after.split_once('"').map(|(path, _)| path)
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

    /// `main`, which holds the files' tables, then each `.duckdb` file,
    /// then each saved connection attached, by the name SQL calls it by.
    async fn list_schemas(&self) -> Result<Vec<String>> {
        Ok(std::iter::once(MAIN.to_owned())
            .chain(
                self.files
                    .iter()
                    .filter(|file| file.found.format.is_database())
                    .map(|file| file.found.name.clone()),
            )
            .chain(self.attached.iter().map(|each| each.name.clone()))
            .take(MAX_LISTED as usize)
            .collect())
    }

    /// In `main`, a table for each file, in the files' order; in a
    /// `.duckdb` file's schema, its tables and views by name; in an
    /// attached connection's, those of its schema `name.table` reads.
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
        let object = ObjectRef {
            schema: schema.to_owned(),
            name: String::new(),
        };
        let (catalog, schema) = catalog_of(&object, &self.attached);
        let (catalog, schema) = (catalog.to_owned(), schema.to_owned());
        self.run(move |connection| {
            read_only(connection, |connection| {
                let mut statement = connection
                    .prepare(&format!(
                        "SELECT table_name, table_type FROM information_schema.tables \
                         WHERE table_catalog = ? AND table_schema = ? \
                         ORDER BY table_name LIMIT {MAX_LISTED}"
                    ))
                    .map_err(map_error)?;
                let objects = statement
                    .query_map([catalog.as_str(), schema.as_str()], |row| {
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
            options: file.options.clone(),
        });
        let unedited = self.unedited(object);
        let in_database = self.database_of(object).is_some();
        let object = object.clone();
        let attached = Arc::clone(&self.attached);
        self.run(move |connection| {
            let (columns, primary_key) = read_only(connection, |connection| {
                let columns = columns(connection, &object, &attached)?;
                // A `.duckdb` file's table finds its rows by its own key.
                let primary_key = match in_database {
                    true => database::table_key(connection, &object.schema, &object.name)?
                        .unwrap_or_default(),
                    false => Vec::new(),
                };
                Ok((columns, primary_key))
            })?;
            if columns.is_empty() {
                return Err(Error::query(format!(
                    "no such table or view: {}",
                    object.name
                )));
            }
            Ok(Structure {
                columns,
                primary_key,
                file,
                unedited,
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
        let attached = Arc::clone(&self.attached);
        self.run(move |connection| {
            let (catalog, schema) = catalog_of(&object, &attached);
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

    /// A page of a file's table. In a session that may write, the first
    /// page of a file a save writes back reads the file into its copy,
    /// and every page is read from the copy, each row with its place.
    async fn fetch_rows(&self, query: &RowQuery) -> Result<RowPage> {
        check_raw_where(query)?;
        let object = query.object.clone();
        let query = query.clone();
        let limit = query.limit as usize;
        let attached = Arc::clone(&self.attached);
        let editing = self.editing(&object);
        let copies = Arc::clone(&self.copies);
        let reading = self.reading.clone();
        let database = self.database_of(&object);
        let writable = self.access == Access::Writable;
        let databases = Arc::clone(&self.databases);
        self.run(move |connection| {
            // A `.duckdb` file's table: read in its key's order, or, with
            // none, where a save may write it, each row with its `rowid`.
            let keyed = match database {
                Some(_) => read_only(connection, |connection| {
                    database::table_key(connection, &query.object.schema, &query.object.name)
                })?,
                None => None,
            };
            // Not where a column of its own is named `rowid`: that reads
            // it instead, and its page says it has no key.
            if let (Some(at), Some(key), true) = (database, &keyed, writable)
                && key.is_empty()
                && read_only(connection, |connection| {
                    database::by_rowid(connection, &query.object.schema, &query.object.name)
                })?
            {
                return read_only(connection, |connection| {
                    database::page(connection, &query, &attached, &databases[at], limit)
                });
            }
            let key = keyed.unwrap_or_default();
            let copied = match &editing {
                Some(file) => {
                    let mut copies = copies.lock().map_err(|_| {
                        Error::ConnectionLost("the DuckDB session was poisoned".into())
                    })?;
                    // Read again where the file changed on disk since: a page
                    // shows the file as it is.
                    let fresh = copies
                        .get(&file.found.name)
                        .is_some_and(|loaded| !loaded.stale(&file.found.path));
                    if !fresh {
                        copies.remove(&file.found.name);
                        let loaded = edit::load(connection, file, &reading)?;
                        copies.insert(file.found.name.clone(), loaded);
                    }
                    match copies.get(&file.found.name) {
                        Some(edit::Loaded::Copy(copy)) => Some(copy.generation),
                        Some(edit::Loaded::Unmatched(_)) | None => None,
                    }
                }
                None => None,
            };
            read_only(connection, |connection| {
                if let Some(generation) = copied {
                    let from = edit::Copy::object(&query.object.name);
                    return edit::page(connection, &query, &from, limit, generation);
                }
                let catalog = columns(connection, &query.object, &attached)?;
                let binary = binary_of(&catalog);
                // A `.duckdb` file's table in its key's order; another
                // file's rows, with no key, in the file's.
                let sql = Dialect::DuckDb.select_rows(&query, &key, &binary)?;
                let started = Instant::now();
                let params = sql.params.iter().map(to_duckdb).collect();
                let (mut columns, rows, has_more) = rows_of(connection, &sql.text, params, limit)?;
                catalog_types(&mut columns, &catalog);
                Ok(RowPage {
                    columns,
                    rows,
                    has_more,
                    ordered_by_key: !key.is_empty(),
                    elapsed: started.elapsed(),
                    positions: None,
                })
            })
        })
        .await
        .map_err(|error| self.of_file(&object, error))
    }

    async fn export_statement(&self, query: &RowQuery, columns: &[String]) -> Result<String> {
        let query = query.clone();
        let chosen = columns.to_vec();
        let attached = Arc::clone(&self.attached);
        let database = self.database_of(&query.object);
        self.run(move |connection| {
            read_only(connection, |connection| {
                let key = key_of(connection, &query.object, database)?;
                let binary = binary_columns(connection, &query.object, &attached)?;
                Dialect::DuckDb.shown_export(&query, &key, &binary, &chosen)
            })
        })
        .await
    }

    /// An export's rows, read as DuckDB's chunks in one blocking job and
    /// sent on a batch at a time: the job waits while the receiver holds
    /// the batch before. Rows come in the order a page reads them in.
    async fn stream_rows(
        &self,
        query: &RowQuery,
        columns: &[String],
        batch: usize,
        to: &tokio::sync::mpsc::Sender<crate::Streamed>,
        stop: &StopFlag,
    ) -> Result<u64> {
        check_raw_where(query)?;
        let query = query.clone();
        let chosen = columns.to_vec();
        let to = to.clone();
        let stop = stop.clone();
        let attached = Arc::clone(&self.attached);
        let database = self.database_of(&query.object);
        self.run(move |connection| {
            read_only(connection, |connection| {
                let key = key_of(connection, &query.object, database)?;
                let catalog = self::columns(connection, &query.object, &attached)?;
                let binary = binary_of(&catalog);
                let sql = Dialect::DuckDb.export_rows(&query, &key, &binary, &chosen)?;
                let mut statement = connection.prepare(&sql.text).map_err(map_error)?;
                let chunks = statement
                    .stream_arrow(duckdb::params_from_iter(sql.params.iter().map(to_duckdb)))
                    .map_err(map_error)?;
                send_chunks(chunks, &catalog, batch, &to, &stop)
            })
        })
        .await
    }

    /// A SQL result's statement, run again without the limit in a
    /// read-only transaction and sent on as `stream_rows` sends a table's.
    /// A stop interrupts the statement while DuckDB reads.
    async fn stream_statement(
        &self,
        text: &str,
        batch: usize,
        to: &tokio::sync::mpsc::Sender<crate::Streamed>,
        stop: &StopFlag,
    ) -> Result<u64> {
        let text = text.to_owned();
        let to = to.clone();
        let stop = stop.clone();
        let interrupt = Arc::clone(&self.interrupt);
        self.run(move |connection| {
            read_only(connection, |connection| {
                let watch = Watch::start(interrupt, stop.clone(), None);
                let mut statement = connection.prepare(&text).map_err(map_error)?;
                let chunks = statement.stream_arrow([]).map_err(map_error)?;
                let sent = send_chunks(chunks, &[], batch, &to, &stop);
                // As in `script`: the watch ends before the rollback.
                drop(watch);
                sent
            })
        })
        .await
    }

    async fn count_rows(&self, query: &RowQuery) -> Result<u64> {
        check_raw_where(query)?;
        let query = query.clone();
        let attached = Arc::clone(&self.attached);
        self.run(move |connection| {
            read_only(connection, |connection| {
                let binary = binary_columns(connection, &query.object, &attached)?;
                let sql = Dialect::DuckDb.count_rows(&query, &binary)?;
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
        // A file is edited in its table and saved: a script reads, in a
        // session that may write too.
        match mode {
            ScriptMode::ReadOnly => {}
            ScriptMode::Write if self.access == Access::Writable => {}
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

    /// Saves `changes` to the copy of the file their table reads, then to
    /// the file (see `edit::save`). A table no save writes back is refused.
    async fn write(&self, changes: &ChangeSet, stop: &StopFlag) -> Result<WriteOutcome> {
        // A `.duckdb` file's table is saved in place, the file held alone.
        if let (Some(at), Access::Writable) = (self.database_of(&changes.object), self.access) {
            let databases = Arc::clone(&self.databases);
            let (changes, stop) = (changes.clone(), stop.clone());
            return self
                .run_gated(false, move |connection| {
                    database::save(connection, &databases[at], &changes, &stop)
                })
                .await;
        }
        let Some(file) = self.editing(&changes.object) else {
            return Err(Error::ReadOnly);
        };
        let copies = Arc::clone(&self.copies);
        let (changes, stop) = (changes.clone(), stop.clone());
        self.run(move |connection| {
            let mut copies = copies
                .lock()
                .map_err(|_| Error::ConnectionLost("the DuckDB session was poisoned".into()))?;
            // No copy in this session: its rows were read in another, a
            // session before a reconnect, and may not be where they were.
            let Some(edit::Loaded::Copy(copy)) = copies.get_mut(&file.found.name) else {
                let label = file_label(&file.found);
                let mut error =
                    Error::query(format!("{label} was read again since these rows were"));
                if let Error::Query { named, .. } = &mut error {
                    named.unsaved = Some(crate::Unsaved::Reread(label));
                }
                return Err(error);
            };
            edit::save(connection, &file, copy, &changes, &stop)
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NewValue;

    /// What an `INSERT` copied or exported from a Files connection writes
    /// for a boolean, the numbers no digits write and a column's default,
    /// read back by DuckDB itself.
    #[test]
    fn duckdb_reads_the_literals_an_insert_is_written_with() {
        let duck = duckdb::Connection::open_in_memory().unwrap();
        duck.execute_batch(
            "CREATE TABLE shelf (flag BOOLEAN, price DOUBLE, note TEXT DEFAULT 'none')",
        )
        .unwrap();
        let dialect = Dialect::DuckDb;
        let default = NewValue::Default { expression: None };
        let default = dialect.pending_literal(&default, "TEXT").unwrap();
        assert_eq!(default, "DEFAULT");
        let prices = [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 4.5];
        for (at, price) in prices.into_iter().enumerate() {
            let flag = dialect.value_literal(&Value::Bool(at % 2 == 0), ValueKind::Bool);
            let price = dialect.value_literal(&Value::Float(price), ValueKind::Numeric);
            duck.execute_batch(&format!(
                "INSERT INTO shelf VALUES ({flag}, {price}, {default})"
            ))
            .unwrap();
        }
        let mut statement = duck
            .prepare("SELECT flag, price, note FROM shelf ORDER BY rowid")
            .unwrap();
        let read: Vec<(bool, f64, String)> = statement
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .unwrap()
            .collect::<duckdb::Result<_>>()
            .unwrap();
        let flags: Vec<bool> = read.iter().map(|row| row.0).collect();
        assert_eq!(flags, [true, false, true, false]);
        assert!(read[0].1.is_nan());
        assert_eq!(read[1].1, f64::INFINITY);
        assert_eq!(read[2].1, f64::NEG_INFINITY);
        assert_eq!(read[3].1, 4.5);
        assert!(read.iter().all(|row| row.2 == "none"));
    }

    /// The file DuckDB's words name: the CSV reader's option line, and the
    /// JSON reader's quoted path. Words that name none give none.
    #[test]
    fn named_file_reads_the_csv_and_the_json_readers_words() {
        let csv = "Conversion Error: CSV Error on Line: 30002\n\n  file = /data/returns.csv\n  delimiter = , (Auto-Detected)";
        assert_eq!(named_file(csv), Some("/data/returns.csv"));
        let json = "Invalid Input Error: Malformed JSON in file \"/data/events.ndjson\", at byte 8 in line 30002: unexpected character.";
        assert_eq!(named_file(json), Some("/data/events.ndjson"));
        assert_eq!(named_file("Binder Error: column \"nope\" not found"), None);
    }
}
