//! Editing a Files connection's files. A file's table is read into an
//! in-memory copy the first time a page of it is read in a session that
//! may write, each row with its place in the file beside it: what a save
//! finds the row by, since a file has no key. A save checks first that
//! the file is as it was read, then changes the copy, writes the whole
//! file to a hidden one in its folder and renames that over it.
//!
//! A CSV or TSV file is written by Tabletist: every row the save does not
//! change is copied as it was, byte for byte, and a changed or new row is
//! written with the file's delimiter and quote. So its rows are read here
//! as DuckDB reads them, and a file whose rows this reading does not count
//! as DuckDB did is not edited at all. Parquet, JSON and NDJSON are written
//! by DuckDB from the copy, Parquet with the compression it had.

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime};

use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};

use super::{FILE_COPY, Opened, literal, map_error, path_text, reader_path, to_duckdb};
use crate::files::{Format, Header, Reading};
use crate::write::{Conflict, NewValue};
use crate::{
    ChangeSet, Dialect, Error, ObjectRef, Result, RowPage, RowQuery, StopFlag, Unsaved, Value,
    WriteOutcome,
};

/// The in-memory database the copies are kept in, apart from the views.
pub(super) const EDITS: &str = "tabletist_edits";

/// The column of a copy that holds each row's place in the file.
pub(super) const POSITION: &str = super::FILE_POSITION;

/// The formats a save writes back.
pub(super) fn editable(format: Format) -> bool {
    format.rewritten()
}

/// The hidden file a save of `path` writes first, in its folder, which is
/// then renamed over it.
pub(super) fn temp_path(path: &Path) -> PathBuf {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    path.with_file_name(format!(".{name}.tabletist-save"))
}

/// A file as it was when it was read: what a save compares it with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Stamp {
    size: u64,
    modified: Option<SystemTime>,
}

impl Stamp {
    /// None that a file on disk has: a copy whose file a save wrote, but
    /// that the save could not set as it is now, is read again.
    const UNKNOWN: Self = Self {
        size: u64::MAX,
        modified: None,
    };

    fn of(path: &Path) -> std::io::Result<Self> {
        let meta = std::fs::metadata(path)?;
        Ok(Self {
            size: meta.len(),
            modified: meta.modified().ok(),
        })
    }
}

/// A file's table as a session that may write has read it.
#[derive(Debug, Clone)]
pub(super) enum Loaded {
    Copy(Copy),
    /// A file a save could not write back as it is, as it was when it was
    /// read: a CSV or TSV whose rows are not counted here as DuckDB counted
    /// them, one with a column of its own named `rowid`, or a JSON or
    /// NDJSON file whose records do not all have every key. It is not
    /// edited.
    Unmatched(Stamp),
}

impl Loaded {
    /// Whether the file at `path` changed on disk since it was read: the
    /// next page reads it again.
    pub(super) fn stale(&self, path: &Path) -> bool {
        let stamp = match self {
            Self::Copy(copy) => copy.stamp,
            Self::Unmatched(stamp) => *stamp,
        };
        Stamp::of(path).ok() != Some(stamp)
    }
}

/// A file's copy: where it is and what a save writes it back with.
#[derive(Debug, Clone)]
pub(super) struct Copy {
    stamp: Stamp,
    /// A CSV's or a TSV's own way of writing.
    csv: Option<Layout>,
    /// A Parquet file's compression, as `COPY` is told it.
    compression: Option<String>,
    /// The place the next new row takes.
    next: i64,
    /// Which copy it is, of every one made in this run of the app: a
    /// page's places are of one ([`crate::Positions::copy`]).
    pub(super) generation: i64,
}

/// A number no copy made before has: each read of a file, and each save
/// that numbers its rows again, takes the next.
fn next_generation() -> i64 {
    static NEXT: AtomicI64 = AtomicI64::new(1);
    NEXT.fetch_add(1, Ordering::SeqCst)
}

/// What keeps two saves of the file at `path` apart in this app: two
/// sessions over one file would write the same hidden file.
fn save_lock(path: &Path) -> Arc<Mutex<()>> {
    static LOCKS: OnceLock<Mutex<std::collections::HashMap<PathBuf, Arc<Mutex<()>>>>> =
        OnceLock::new();
    let mut locks = LOCKS
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    Arc::clone(locks.entry(path.to_owned()).or_default())
}

impl Copy {
    /// The rows in the copy, for the page's positions.
    pub(super) fn object(name: &str) -> ObjectRef {
        ObjectRef {
            schema: EDITS.to_owned(),
            name: name.to_owned(),
        }
    }
}

/// A page of a file's table, read from its copy, each row with its place:
/// in the file's order, or that after the page's own sort.
pub(super) fn page(
    connection: &duckdb::Connection,
    query: &RowQuery,
    limit: usize,
    generation: i64,
) -> Result<RowPage> {
    let mut from = query.clone();
    from.object = Copy::object(&query.object.name);
    let catalog = super::columns(connection, &from.object, &[])?;
    let binary = super::binary_of(&catalog);
    let sql = Dialect::DuckDb.select_rows(&from, &[POSITION.to_owned()], &binary)?;
    let started = Instant::now();
    let params = sql.params.iter().map(to_duckdb).collect();
    let (mut columns, mut rows, has_more) = super::rows_of(connection, &sql.text, params, limit)?;
    let at = columns
        .iter()
        .position(|column| column.name == POSITION)
        .ok_or_else(|| Error::query("the rows' places were not read"))?;
    columns.remove(at);
    let positions = rows
        .iter_mut()
        .map(|row| match row.remove(at) {
            Value::Int(place) => Ok(place),
            _ => Err(Error::query("a row's place was not read")),
        })
        .collect::<Result<Vec<_>>>()?;
    super::catalog_types(&mut columns, &catalog);
    Ok(RowPage {
        columns,
        rows,
        has_more,
        ordered_by_key: true,
        elapsed: started.elapsed(),
        positions: Some(crate::Positions {
            copy: generation,
            rows: positions,
        }),
    })
}

/// How a CSV or a TSV file writes its rows, as DuckDB's sniffer and the
/// connection's Reading CSV say.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Layout {
    pub(super) delimiter: u8,
    pub(super) quote: Option<u8>,
    pub(super) escape: Option<u8>,
    pub(super) header: bool,
    pub(super) newline: &'static [u8],
    pub(super) columns: usize,
    /// How its dates and timestamps are written, where the sniffer found
    /// a way: `%d/%m/%Y`.
    pub(super) date: Option<String>,
    pub(super) timestamp: Option<String>,
}

/// Reads `file` into its copy, from its own view. `None` where it reads
/// the file but cannot edit it ([`Loaded::Unmatched`]).
pub(super) fn load(
    connection: &duckdb::Connection,
    file: &Opened,
    reading: &Reading,
) -> Result<Loaded> {
    let path = &file.found.path;
    // Before it is read: a change made while it is read is one since.
    let stamp = Stamp::of(path).map_err(|error| Error::Io(error.to_string()))?;
    // A file read whole for its copy that can't be (a bad line past the
    // first page, one too big for memory) is still read as before, a page
    // at a time: it is not edited. A read given up on is no such file.
    match copied(connection, file, reading, stamp) {
        Err(Error::Cancelled) => Err(Error::Cancelled),
        Err(_) => {
            let copy = Dialect::DuckDb.qualified(&Copy::object(&file.found.name));
            let _ = connection.execute_batch(&format!("DROP TABLE IF EXISTS {copy}"));
            Ok(Loaded::Unmatched(stamp))
        }
        loaded => loaded,
    }
}

/// [`load`]'s work, which may fail anywhere: the copy, its places, and
/// what the save writes the file back with.
fn copied(
    connection: &duckdb::Connection,
    file: &Opened,
    reading: &Reading,
    stamp: Stamp,
) -> Result<Loaded> {
    let path = &file.found.path;
    let copy = Dialect::DuckDb.qualified(&Copy::object(&file.found.name));
    let view = Dialect::DuckDb.qualified(&ObjectRef {
        schema: super::MAIN.to_owned(),
        name: file.found.name.clone(),
    });
    let position = Dialect::DuckDb.quote_ident(POSITION);
    connection
        .execute_batch(&format!(
            "CREATE OR REPLACE TABLE {copy} AS SELECT * FROM {view}"
        ))
        .map_err(map_error)?;
    let columns_named = |named: &str| -> Result<i64> {
        connection
            .query_row(
                "SELECT count(*) FROM information_schema.columns \
                 WHERE table_catalog = ? AND table_name = ? AND lower(column_name) = ?",
                [EDITS, file.found.name.as_str(), named],
                |row| row.get(0),
            )
            .map_err(map_error)
    };
    let unmatched = || -> Result<Loaded> {
        connection
            .execute_batch(&format!("DROP TABLE {copy}"))
            .map_err(map_error)?;
        Ok(Loaded::Unmatched(stamp))
    };
    // DuckDB keeps the file's order in the copy's rows: `rowid` is each
    // one's place in the file, and stays its own when a row before it is
    // deleted. A column of the file's own of that name would stand for it.
    if columns_named("rowid")? > 0 || columns_named(POSITION)? > 0 {
        return unmatched();
    }
    connection
        .execute_batch(&format!(
            "ALTER TABLE {copy} ADD COLUMN {position} BIGINT; \
             UPDATE {copy} SET {position} = rowid;"
        ))
        .map_err(map_error)?;
    let rows: i64 = connection
        .query_row(&format!("SELECT count(*) FROM {copy}"), [], |row| {
            row.get(0)
        })
        .map_err(map_error)?;
    let (csv, compression) = match file.found.format {
        Format::Csv | Format::Tsv => {
            let columns: usize = file_columns(connection, &file.found.name)?
                .try_into()
                .unwrap_or(0);
            let layout = layout(connection, path, file.found.format, reading, columns)?;
            let counted = std::fs::File::open(path)
                .and_then(|opened| count(BufReader::new(opened), &layout))
                .map_err(|error| Error::Io(error.to_string()))?;
            if i64::try_from(counted).ok() != Some(rows) {
                return unmatched();
            }
            (Some(layout), None)
        }
        // DuckDB writes every key of every object back, and a key an
        // object did not have would come back as `null`: only a file whose
        // objects at each place have the same keys is edited.
        Format::Json | Format::Ndjson => {
            let format = match file.found.format {
                Format::Json => "array",
                _ => "newline_delimited",
            };
            let mut statement = connection
                .prepare(&format!(
                    "SELECT json::VARCHAR FROM read_json_objects({}, format = '{format}')",
                    literal(&reader_path(path)?)
                ))
                .map_err(map_error)?;
            let mut records = statement.query([]).map_err(map_error)?;
            let mut seen = std::collections::HashMap::new();
            while let Some(record) = records.next().map_err(map_error)? {
                let text: String = record.get(0).map_err(map_error)?;
                let parsed = serde_json::from_str::<serde_json::Value>(&text);
                // A record that is no object comes back as one.
                let kept = parsed.is_ok_and(|value| {
                    value.is_object() && same_keys(&value, &mut String::new(), &mut seen)
                });
                if !kept {
                    drop(records);
                    return unmatched();
                }
            }
            (None, None)
        }
        Format::Parquet => {
            let compression: Option<String> = connection
                .query_row(
                    &format!(
                        "SELECT compression FROM parquet_metadata({}) LIMIT 1",
                        literal(&reader_path(path)?)
                    ),
                    [],
                    |row| row.get(0),
                )
                .ok();
            (None, compression.and_then(|named| compression_of(&named)))
        }
        Format::Excel | Format::DuckDb => (None, None),
    };
    Ok(Loaded::Copy(Copy {
        stamp,
        csv,
        compression,
        next: rows,
        generation: next_generation(),
    }))
}

/// Whether every object in `value` has the keys `seen` has for its place
/// (`."meta"`, `."items"[]`), each place's first keeping its keys there.
fn same_keys(
    value: &serde_json::Value,
    place: &mut String,
    seen: &mut std::collections::HashMap<String, Vec<String>>,
) -> bool {
    match value {
        serde_json::Value::Object(map) => {
            let mut keys: Vec<String> = map.keys().cloned().collect();
            keys.sort();
            match seen.get(place.as_str()) {
                Some(known) if *known != keys => return false,
                Some(_) => {}
                None => {
                    seen.insert(place.clone(), keys);
                }
            }
            map.iter().all(|(key, inner)| {
                let at = place.len();
                // Quoted, so a key with a dot in it is no nested one.
                place.push('.');
                place.push_str(&serde_json::Value::String(key.clone()).to_string());
                let same = same_keys(inner, place, seen);
                place.truncate(at);
                same
            })
        }
        serde_json::Value::Array(items) => items.iter().all(|item| {
            let at = place.len();
            place.push_str("[]");
            let same = same_keys(item, place, seen);
            place.truncate(at);
            same
        }),
        _ => true,
    }
}

/// How many columns the copy of the file whose table is `name` has, its
/// place apart.
fn file_columns(connection: &duckdb::Connection, name: &str) -> Result<i64> {
    connection
        .query_row(
            "SELECT count(*) FROM information_schema.columns \
             WHERE table_catalog = ? AND table_name = ? AND column_name <> ?",
            [EDITS, name, POSITION],
            |row| row.get(0),
        )
        .map_err(map_error)
}

/// A Parquet file's compression as `parquet_metadata` names it, as `COPY`
/// is told it; `None` for one it does not take, which then writes its own.
fn compression_of(named: &str) -> Option<String> {
    let named = named.to_ascii_lowercase();
    [
        "uncompressed",
        "snappy",
        "gzip",
        "zstd",
        "brotli",
        "lz4",
        "lz4_raw",
    ]
    .contains(&named.as_str())
    .then_some(named)
}

/// How `path` writes its rows: DuckDB's sniffer, then what the connection's
/// Reading CSV sets instead of detecting.
fn layout(
    connection: &duckdb::Connection,
    path: &Path,
    format: Format,
    reading: &Reading,
    columns: usize,
) -> Result<Layout> {
    let sniffed = connection
        .query_row(
            &format!(
                "SELECT Delimiter, Quote, Escape, NewLineDelimiter, HasHeader, DateFormat, \
                 TimestampFormat FROM sniff_csv({})",
                literal(&reader_path(path)?)
            ),
            [],
            |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<bool>>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, Option<String>>(6)?,
                ))
            },
        )
        .map_err(map_error)?;
    let (delimiter, quote, escape, newline, header, date, timestamp) = sniffed;
    // The sniffer writes a character it found none of as `(empty)`.
    let byte = |text: Option<String>| -> Option<u8> {
        let text = text?;
        match text.as_str() {
            "(empty)" | "" => None,
            "\\t" => Some(b'\t'),
            text => (text.len() == 1).then(|| text.as_bytes()[0]),
        }
    };
    // A `.tsv` is read with a tab, whatever Reading CSV sets.
    let delimiter = match (format, reading.delimiter.char()) {
        (Format::Tsv, _) => b'\t',
        (_, Some(set)) => u8::try_from(set).unwrap_or(b','),
        _ => byte(delimiter).unwrap_or(b','),
    };
    let header = match reading.header {
        Header::Yes => true,
        Header::No => false,
        Header::Detect => header.unwrap_or(true),
    };
    Ok(Layout {
        delimiter,
        quote: byte(quote),
        escape: byte(escape),
        header,
        newline: match newline.as_deref() {
            Some("\\r\\n") => b"\r\n",
            _ => b"\n",
        },
        columns,
        date: date.filter(|format| !format.is_empty()),
        timestamp: timestamp.filter(|format| !format.is_empty()),
    })
}

/// Reads a CSV file one record at a time, as DuckDB splits it: a line
/// break inside quotes is part of the record.
struct Records<R> {
    reader: R,
    quote: Option<u8>,
    escape: Option<u8>,
}

impl<R: BufRead> Records<R> {
    /// Puts the next record's bytes in `record` and the line break after
    /// it in `brk` (none at the end of the file). `false` past the last.
    fn next(&mut self, record: &mut Vec<u8>, brk: &mut Vec<u8>) -> std::io::Result<bool> {
        record.clear();
        brk.clear();
        let (mut quoted, mut escaped, mut cr, mut any) = (false, false, false, false);
        loop {
            let buffer = self.reader.fill_buf()?;
            if buffer.is_empty() {
                if cr {
                    brk.push(b'\r');
                }
                return Ok(any);
            }
            any = true;
            let mut used = 0;
            for &byte in buffer {
                if cr {
                    // A `\r` alone ends the record too; the byte after it is
                    // the next record's.
                    match byte {
                        b'\n' => {
                            brk.extend_from_slice(b"\r\n");
                            used += 1;
                        }
                        _ => brk.push(b'\r'),
                    }
                    self.reader.consume(used);
                    return Ok(true);
                }
                used += 1;
                if quoted {
                    record.push(byte);
                    if escaped {
                        escaped = false;
                    } else if self.escape.is_some_and(|escape| escape == byte)
                        && self.escape != self.quote
                    {
                        escaped = true;
                    } else if self.quote == Some(byte) {
                        // A doubled quote closes and opens again.
                        quoted = false;
                    }
                    continue;
                }
                match byte {
                    b'\n' => {
                        brk.push(b'\n');
                        self.reader.consume(used);
                        return Ok(true);
                    }
                    b'\r' => cr = true,
                    byte => {
                        if self.quote == Some(byte) {
                            quoted = true;
                        }
                        record.push(byte);
                    }
                }
            }
            self.reader.consume(used);
        }
    }
}

/// Whether DuckDB reads `record` as no row: an empty line, where a row has
/// more than one column. With one column it is a row whose value is NULL.
fn blank(record: &[u8], layout: &Layout) -> bool {
    record.is_empty() && layout.columns > 1
}

/// Strips a UTF-8 byte-order mark off the start, which is written back as
/// it was.
fn mark<R: BufRead>(reader: &mut R) -> std::io::Result<&'static [u8]> {
    let start = reader.fill_buf()?;
    if start.starts_with(b"\xEF\xBB\xBF") {
        reader.consume(3);
        return Ok(b"\xEF\xBB\xBF");
    }
    Ok(b"")
}

/// The rows of a CSV file, counted as DuckDB counts them.
fn count(mut reader: impl BufRead, layout: &Layout) -> std::io::Result<u64> {
    mark(&mut reader)?;
    let mut records = Records {
        reader,
        quote: layout.quote,
        escape: layout.escape,
    };
    let (mut record, mut brk) = (Vec::new(), Vec::new());
    let (mut header, mut rows) = (layout.header, 0);
    while records.next(&mut record, &mut brk)? {
        if blank(&record, layout) {
            continue;
        }
        if header {
            header = false;
            continue;
        }
        rows += 1;
    }
    Ok(rows)
}

/// A save of a CSV file: by each changed row's place, what becomes of it.
#[derive(Debug, Default)]
struct Plan {
    deleted: std::collections::BTreeSet<u64>,
    /// Each changed row's fields, by their place, with their new text.
    changed: BTreeMap<u64, Vec<(usize, String)>>,
    /// The new rows, every field's text in the columns' order.
    added: Vec<Vec<String>>,
}

/// Writes the CSV file `reader` reads to `to`, as `plan` changes it.
fn rewrite(
    mut reader: impl BufRead,
    to: &mut impl Write,
    layout: &Layout,
    plan: &Plan,
) -> Result<()> {
    let io = |error: std::io::Error| Error::Io(error.to_string());
    to.write_all(mark(&mut reader).map_err(io)?).map_err(io)?;
    let mut records = Records {
        reader,
        quote: layout.quote,
        escape: layout.escape,
    };
    let (mut record, mut brk) = (Vec::new(), Vec::new());
    let (mut header, mut row, mut ended) = (layout.header, 0_u64, true);
    // `ended` says whether the last record written ended its line: a new
    // row after one that did not starts on a line of its own.
    while records.next(&mut record, &mut brk).map_err(io)? {
        if blank(&record, layout) || std::mem::take(&mut header) {
            to.write_all(&record).map_err(io)?;
            to.write_all(&brk).map_err(io)?;
            ended = !brk.is_empty();
            continue;
        }
        let at = row;
        row += 1;
        if plan.deleted.contains(&at) {
            continue;
        }
        ended = !brk.is_empty();
        match plan.changed.get(&at) {
            Some(cells) => {
                let mut fields: Vec<Vec<u8>> = split(&record, layout)
                    .into_iter()
                    .map(|field| record[field].to_vec())
                    .collect();
                for (place, text) in cells {
                    let Some(field) = fields.get_mut(*place) else {
                        return Err(Error::query(format!(
                            "row {} has fewer fields than the file's columns",
                            at + 1
                        )));
                    };
                    *field = written(text, layout);
                }
                to.write_all(&fields.join(&layout.delimiter)).map_err(io)?;
            }
            None => to.write_all(&record).map_err(io)?,
        }
        to.write_all(&brk).map_err(io)?;
    }
    for (index, added) in plan.added.iter().enumerate() {
        if index == 0 && !ended {
            to.write_all(layout.newline).map_err(io)?;
        }
        let fields: Vec<Vec<u8>> = added.iter().map(|text| written(text, layout)).collect();
        to.write_all(&fields.join(&layout.delimiter)).map_err(io)?;
        to.write_all(layout.newline).map_err(io)?;
    }
    Ok(())
}

/// The places of `record`'s fields, split at the delimiter outside quotes.
fn split(record: &[u8], layout: &Layout) -> Vec<std::ops::Range<usize>> {
    let (mut fields, mut start, mut quoted, mut escaped) = (Vec::new(), 0, false, false);
    for (at, &byte) in record.iter().enumerate() {
        if quoted {
            if escaped {
                escaped = false;
            } else if layout.escape == Some(byte) && layout.escape != layout.quote {
                escaped = true;
            } else if layout.quote == Some(byte) {
                quoted = false;
            }
        } else if layout.quote == Some(byte) {
            quoted = true;
        } else if byte == layout.delimiter {
            fields.push(start..at);
            start = at + 1;
        }
    }
    fields.push(start..record.len());
    fields
}

/// `text` as a field of the file: quoted where it holds the delimiter, a
/// quote or a line break, or starts or ends with a space.
fn written(text: &str, layout: &Layout) -> Vec<u8> {
    let quote = layout.quote.unwrap_or(b'"');
    let bytes = text.as_bytes();
    let needs = bytes
        .iter()
        .any(|&byte| byte == layout.delimiter || byte == quote || byte == b'\n' || byte == b'\r')
        || text.starts_with(' ')
        || text.ends_with(' ');
    if !needs {
        return bytes.to_vec();
    }
    let escape = layout.escape.unwrap_or(quote);
    let mut out = Vec::with_capacity(bytes.len() + 2);
    out.push(quote);
    for &byte in bytes {
        if byte == quote || (byte == escape && escape != quote) {
            out.push(escape);
        }
        out.push(byte);
    }
    out.push(quote);
    out
}

/// What became of `changes`, to the copy of `file` and the file itself.
/// Nothing is written where the file changed since it was read, or was
/// read again since the page the changes are of, or cannot be written: the
/// copy is as it was, and the save says why ([`Unsaved`]). Once the file is
/// written the save is: a copy that could not be brought up to it after is
/// read again by the next page.
pub(super) fn save(
    connection: &duckdb::Connection,
    file: &Opened,
    copy: &mut Copy,
    changes: &ChangeSet,
    stop: &StopFlag,
) -> Result<WriteOutcome> {
    let started = Instant::now();
    let path = &file.found.path;
    let label = crate::files::file_name(path);
    let unsaved = |why: Unsaved, message: String| {
        let mut error = Error::query(message);
        if let Error::Query { named, .. } = &mut error {
            named.unsaved = Some(why);
        }
        error
    };
    let lock = save_lock(path);
    let _one_at_a_time = lock.lock().unwrap_or_else(PoisonError::into_inner);
    // Rows found by their places in another copy may not be where they
    // were: the file was read again, or a save numbered them again.
    let Some(changes) = own_keys(changes, copy.generation) else {
        return Err(unsaved(
            Unsaved::Reread(label.clone()),
            format!("{label} was read again since these rows were"),
        ));
    };
    if Stamp::of(path).ok() != Some(copy.stamp) {
        return Err(unsaved(
            Unsaved::Changed(label.clone()),
            format!("{label} changed on disk since it was opened"),
        ));
    }
    // A file its owner made read-only is not written over: the rename
    // would replace it all the same where its folder is writable.
    // Its permissions say so, or this user may not write it: opened to
    // write, and not truncated, it says which.
    let protected = std::fs::metadata(path).is_ok_and(|meta| meta.permissions().readonly())
        || std::fs::OpenOptions::new()
            .write(true)
            .open(path)
            .is_err_and(|error| error.kind() == std::io::ErrorKind::PermissionDenied);
    if protected {
        return Err(unsaved(
            Unsaved::Protected(label.clone()),
            format!("{label} is read-only"),
        ));
    }
    connection
        .execute_batch("BEGIN TRANSACTION")
        .map_err(map_error)?;
    let rollback = || {
        let _ = connection.execute_batch("ROLLBACK");
    };
    let (inserted, rows, gone) = match changed(connection, file, copy, &changes, stop) {
        Ok(WriteOutcome::Written {
            inserted,
            rows,
            gone,
            ..
        }) => (inserted, rows, gone),
        other => {
            rollback();
            return other;
        }
    };
    let stamp = match write_file(connection, file, copy, &changes) {
        Ok(stamp) => stamp,
        Err(error) => {
            rollback();
            return Err(match error {
                WriteError::Changed => unsaved(
                    Unsaved::Changed(label.clone()),
                    format!("{label} changed on disk while it was saved"),
                ),
                WriteError::Taken => {
                    let why = format!(
                        "another program replaced {} while it was written",
                        crate::files::file_name(&temp_path(path))
                    );
                    unsaved(
                        Unsaved::Failed(label.clone(), why.clone()),
                        format!("{label} could not be written: {why}"),
                    )
                }
                WriteError::Folder(error) => unsaved(
                    Unsaved::Unwritable(label.clone()),
                    format!("{label}'s folder cannot be written to: {error}"),
                ),
                WriteError::Io(error) => unsaved(
                    Unsaved::Failed(label.clone(), error.to_string()),
                    format!("{label} could not be written: {error}"),
                ),
                WriteError::Db(error) => error,
            });
        }
    };
    // The file is written. Each row's place in it as it is now: rows after
    // a deleted one move up, and new rows are last.
    let position = Dialect::DuckDb.quote_ident(POSITION);
    let table = Dialect::DuckDb.qualified(&Copy::object(&file.found.name));
    let brought = match fails_after_rename(path) {
        true => Err(duckdb::Error::QueryReturnedNoRows),
        false => bring_up(connection, &table, &position),
    };
    match brought {
        Ok(rows) => {
            copy.next = rows;
            copy.stamp = stamp;
        }
        Err(_) => {
            rollback();
            copy.stamp = Stamp::UNKNOWN;
        }
    }
    // The page's places are of the copy as it was.
    copy.generation = next_generation();
    Ok(WriteOutcome::Written {
        inserted,
        rows,
        gone,
        elapsed: started.elapsed(),
    })
}

/// Numbers the copy's rows again from 0 in their order, commits, and says
/// how many rows it has.
fn bring_up(connection: &duckdb::Connection, table: &str, position: &str) -> duckdb::Result<i64> {
    connection
        .execute_batch(&format!(
            "UPDATE {table} SET {position} = placed.at FROM \
             (SELECT {position} AS was, row_number() OVER (ORDER BY {position}) - 1 AS at \
             FROM {table}) AS placed WHERE {table}.{position} = placed.was; COMMIT"
        ))
        .and_then(|()| {
            connection.query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
                row.get::<_, i64>(0)
            })
        })
}

/// A test's way to make a save of one file fail after it is written.
#[cfg(test)]
static FAIL_AFTER_RENAME: Mutex<Option<PathBuf>> = Mutex::new(None);

/// A test's way to have another program replace one file's hidden file
/// mid-save.
#[cfg(test)]
static REPLACE_HIDDEN: Mutex<Option<PathBuf>> = Mutex::new(None);

/// Whether the hook's other program could take the hidden file away, or
/// write it.
#[cfg(test)]
static REPLACED: Mutex<Option<bool>> = Mutex::new(None);

/// Whether a test set `hook` for the file at `path`.
#[cfg(test)]
fn hooked(hook: &Mutex<Option<PathBuf>>, path: &Path) -> bool {
    hook.lock()
        .unwrap_or_else(PoisonError::into_inner)
        .as_deref()
        == Some(path)
}

/// Whether this save of `path` fails after its file is written: only where
/// a test says so.
fn fails_after_rename(path: &Path) -> bool {
    #[cfg(test)]
    return hooked(&FAIL_AFTER_RENAME, path);
    #[cfg(not(test))]
    {
        let _ = path;
        false
    }
}

/// `changes` with each row's key as the copy finds it, its place alone.
/// `None` where a key is not a place in the copy `generation`.
fn own_keys(changes: &ChangeSet, generation: i64) -> Option<ChangeSet> {
    let own = |key: &[(String, Value)]| match key {
        [(place, Value::Int(at)), (of, Value::Int(copy))]
            if place == POSITION && of == FILE_COPY && *copy == generation =>
        {
            Some(vec![(POSITION.to_owned(), Value::Int(*at))])
        }
        _ => None,
    };
    let mut own_set = changes.clone();
    for row in &mut own_set.rows {
        row.key = own(&row.key)?;
    }
    for row in &mut own_set.deletes {
        row.key = own(&row.key)?;
    }
    Some(own_set)
}

/// Changes the copy as `changes` say, in the transaction `save` began.
fn changed(
    connection: &duckdb::Connection,
    file: &Opened,
    copy: &Copy,
    changes: &ChangeSet,
    stop: &StopFlag,
) -> Result<WriteOutcome> {
    let object = Copy::object(&file.found.name);
    let dialect = Dialect::DuckDb;
    let stopped = || {
        if stop.is_stopped() {
            return Err(Error::Cancelled);
        }
        Ok(())
    };
    let run = |sql: &crate::Sql| -> Result<usize> {
        let params = duckdb::params_from_iter(sql.params.iter().map(to_duckdb));
        connection.execute(&sql.text, params).map_err(map_error)
    };
    for (insert, row) in changes.inserts.iter().enumerate() {
        stopped()?;
        let mut row = row.clone();
        row.set.push(crate::InsertValue {
            column: POSITION.to_owned(),
            type_name: "BIGINT".to_owned(),
            new: NewValue::Text((copy.next + insert as i64).to_string()),
        });
        let statement = dialect.insert_row(&object, &row);
        let done = statement.and_then(|statement| {
            let params = duckdb::params_from_iter(statement.sql.params.iter().map(to_duckdb));
            let mut prepared = connection.prepare(&statement.sql.text).map_err(map_error)?;
            let mut rows = prepared.query(params).map_err(map_error)?;
            while rows.next().map_err(map_error)?.is_some() {}
            Ok(())
        });
        if let Err(error) = done {
            return Ok(WriteOutcome::FailedInsert { insert, error });
        }
    }
    let mut conflicts = Vec::new();
    for (row, change) in changes.rows.iter().enumerate() {
        stopped()?;
        match dialect
            .update_row(&object, change)
            .and_then(|update| run(&update.sql))
        {
            Ok(0) => conflicts.push(Conflict { row, server: None }),
            Ok(_) => {}
            Err(error) => return Ok(WriteOutcome::Failed { row, error }),
        }
    }
    if !conflicts.is_empty() {
        return Ok(WriteOutcome::Conflicts(conflicts));
    }
    let mut gone = Vec::new();
    for (delete, row) in changes.deletes.iter().enumerate() {
        stopped()?;
        match dialect
            .delete_row(&object, row)
            .and_then(|deletion| run(&deletion.sql))
        {
            Ok(0) => gone.push(delete),
            Ok(_) => {}
            Err(error) => return Ok(WriteOutcome::FailedDelete { delete, error }),
        }
    }
    stopped()?;
    // Each changed row as the copy now holds it, in the page's columns.
    let table = dialect.qualified(&object);
    let position = dialect.quote_ident(POSITION);
    let mut rows = Vec::with_capacity(changes.rows.len());
    for change in &changes.rows {
        let Some(at) = place_of(&change.key) else {
            continue;
        };
        let sql = format!("SELECT * EXCLUDE ({position}) FROM {table} WHERE {position} = {at}");
        let (_, mut found, _) = super::rows_of(connection, &sql, Vec::new(), 1)?;
        rows.extend(found.pop());
    }
    Ok(WriteOutcome::Written {
        inserted: vec![None; changes.inserts.len()],
        rows,
        gone,
        elapsed: std::time::Duration::ZERO,
    })
}

/// The place a row's key names, where it is the copy's own.
fn place_of(key: &[(String, Value)]) -> Option<i64> {
    match key {
        [(column, Value::Int(at))] if column == POSITION => Some(*at),
        _ => None,
    }
}

enum WriteError {
    /// The hidden file could not be made in the folder, or put in the
    /// file's place, for want of permission: the folder is not writable.
    Folder(std::io::Error),
    /// Anything else that could not be read or written.
    Io(std::io::Error),
    /// The file changed on disk while the save wrote its hidden one.
    Changed,
    /// Another program put a file of its own at the hidden file's name.
    Taken,
    Db(Error),
}

/// Writes the copy, changed, to the hidden file beside `file`, then
/// renames it over `file`, and returns the file as it is then. A hidden
/// file of that name already there is no save's of this one: it is left as
/// it is, and nothing is written. The hidden file this save made is gone
/// after, whatever came of it.
fn write_file(
    connection: &duckdb::Connection,
    file: &Opened,
    copy: &Copy,
    changes: &ChangeSet,
) -> std::result::Result<Stamp, WriteError> {
    let path = &file.found.path;
    let temp = temp_path(path);
    // The hidden file is this save's from here: made where none is, in one
    // step, or not at all. One already there is left as it is.
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    // Only this user's until it takes the file's own permissions: the file
    // may be one others can't read.
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    // Windows lets no other program delete or rename it while this save
    // holds it, nor write it where this save writes it itself (a CSV):
    // shared to read, and to write only where DuckDB writes it, never to
    // delete.
    #[cfg(windows)]
    std::os::windows::fs::OpenOptionsExt::share_mode(
        &mut options,
        if copy.csv.is_some() { 0x1 } else { 0x1 | 0x2 },
    );
    // Windows answers "access is denied" where a folder has the name: what
    // is there is what says it is taken.
    let claimed = options.open(&temp).map_err(|error| {
        if std::fs::symlink_metadata(&temp).is_err() {
            return in_folder(error);
        }
        let name = crate::files::file_name(&temp);
        WriteError::Io(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            format!("a file named {name} is in its folder"),
        ))
    })?;
    // Kept open to the end: what is at the hidden file's name is still the
    // one this save made, or another program put its own there.
    let written = match &copy.csv {
        Some(layout) => claimed
            .try_clone()
            .map_err(WriteError::Io)
            .and_then(|target| write_csv(connection, file, layout, changes, target)),
        // DuckDB writes over the file this save made, as it is.
        None => write_copy(connection, file, copy, &temp),
    };
    #[cfg(test)]
    if hooked(&REPLACE_HIDDEN, path) {
        let removed = std::fs::remove_file(&temp).is_ok();
        let wrote = std::fs::write(&temp, "another program's").is_ok();
        *REPLACED.lock().unwrap_or_else(PoisonError::into_inner) = Some(removed || wrote);
    }
    let checked = written.and_then(|()| {
        if !same_file(&claimed, &temp) {
            return Err(WriteError::Taken);
        }
        // The file keeps its own permissions, given to the file this save
        // holds, whatever is at its name.
        if let Ok(meta) = std::fs::metadata(path) {
            claimed
                .set_permissions(meta.permissions())
                .map_err(WriteError::Io)?;
        }
        // Once more, right before it is replaced: another program's change
        // while the hidden file was written is not written over. A rename
        // keeps the hidden file's size and time, which are the file's next.
        if Stamp::of(path).ok() != Some(copy.stamp) {
            return Err(WriteError::Changed);
        }
        Stamp::of(&temp).map_err(WriteError::Io)
    });
    let ours = same_file(&claimed, &temp);
    // On Windows this save's own hold keeps it from renaming the file too.
    // From here to the rename, another program could take the name: std
    // renames no file by its handle.
    drop(claimed);
    let renamed = checked.and_then(|stamp| {
        // Refused for permission, a rename is not the folder's doing alone
        // (a folder with its sticky bit, a file another user owns): the
        // system's words say which.
        std::fs::rename(&temp, path).map_err(WriteError::Io)?;
        settle(path);
        Ok(stamp)
    });
    if renamed.is_err() && ours {
        let _ = std::fs::remove_file(&temp);
    }
    renamed
}

/// Whether the file at `path` is `claimed`, the one this save made. On
/// Windows it is taken to be: while this save holds it no other program
/// can delete or rename it.
fn same_file(claimed: &std::fs::File, path: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        match (claimed.metadata(), std::fs::symlink_metadata(path)) {
            (Ok(ours), Ok(there)) => ours.dev() == there.dev() && ours.ino() == there.ino(),
            _ => false,
        }
    }
    #[cfg(not(unix))]
    {
        let _ = (claimed, path);
        true
    }
}

/// Writes the folder holding `path` to disk, so the rename that put the
/// saved file in place outlasts a crash. The file is saved by then, so a
/// folder that can't be synced (Windows opens none) is no failure.
fn settle(path: &Path) {
    #[cfg(unix)]
    if let Some(folder) = path.parent() {
        let _ = std::fs::File::open(folder).and_then(|folder| folder.sync_all());
    }
    #[cfg(not(unix))]
    let _ = path;
}

/// What DuckDB's `COPY` into the hidden file said, in the system's words.
/// This save made that file already: no failure of it is the folder's.
fn copy_failed(said: &str) -> WriteError {
    WriteError::Io(std::io::Error::other(said.to_owned()))
}

/// `error` from making the hidden file: the folder's want of permission
/// where it is one.
fn in_folder(error: std::io::Error) -> WriteError {
    match error.kind() {
        std::io::ErrorKind::PermissionDenied => WriteError::Folder(error),
        _ => WriteError::Io(error),
    }
}

/// A Parquet, JSON or NDJSON file, written by DuckDB from the copy.
fn write_copy(
    connection: &duckdb::Connection,
    file: &Opened,
    copy: &Copy,
    temp: &Path,
) -> std::result::Result<(), WriteError> {
    let options = match file.found.format {
        Format::Parquet => match &copy.compression {
            Some(compression) => format!("FORMAT parquet, COMPRESSION {compression}"),
            None => "FORMAT parquet".to_owned(),
        },
        Format::Json => "FORMAT json, ARRAY true".to_owned(),
        Format::Ndjson => "FORMAT json".to_owned(),
        Format::Csv | Format::Tsv | Format::Excel | Format::DuckDb => {
            return Err(WriteError::Db(Error::query(
                "this file is not written by DuckDB",
            )));
        }
    };
    let position = Dialect::DuckDb.quote_ident(POSITION);
    let table = Dialect::DuckDb.qualified(&Copy::object(&file.found.name));
    let to = path_text(temp).map_err(WriteError::Db)?;
    connection
        .execute_batch(&format!(
            // Into the hidden file this save made, as it is: DuckDB would
            // write a file beside an existing one first, which the fence
            // refuses.
            "COPY (SELECT * EXCLUDE ({position}) FROM {table} ORDER BY {position}) TO {} \
             ({options}, USE_TMP_FILE false)",
            literal(&to)
        ))
        .map_err(|error| copy_failed(&map_error(error).to_string()))
}

/// A CSV or TSV file, written row for row as [`rewrite`] does: what a save
/// changes is read from the copy, its dates as the file writes them.
fn write_csv(
    connection: &duckdb::Connection,
    file: &Opened,
    layout: &Layout,
    changes: &ChangeSet,
    target: std::fs::File,
) -> std::result::Result<(), WriteError> {
    let plan = plan(connection, file, layout, changes).map_err(WriteError::Db)?;
    let source = std::fs::File::open(&file.found.path).map_err(WriteError::Io)?;
    let mut target = BufWriter::new(target);
    rewrite(BufReader::new(source), &mut target, layout, &plan).map_err(|error| match error {
        Error::Io(message) => WriteError::Io(std::io::Error::other(message)),
        error => WriteError::Db(error),
    })?;
    target.flush().map_err(WriteError::Io)?;
    target
        .into_inner()
        .map_err(|error| WriteError::Io(error.into_error()))?
        .sync_all()
        .map_err(WriteError::Io)
}

/// What the save writes of each row it changes, deletes or adds. A cell
/// set to text is written as it was typed, but for a date or a timestamp
/// in a file that writes those its own way; NULL and a default are an
/// empty field.
fn plan(
    connection: &duckdb::Connection,
    file: &Opened,
    layout: &Layout,
    changes: &ChangeSet,
) -> Result<Plan> {
    let object = Copy::object(&file.found.name);
    let table = Dialect::DuckDb.qualified(&object);
    let position = Dialect::DuckDb.quote_ident(POSITION);
    let columns = super::columns(connection, &object, &[])?;
    let columns: Vec<_> = columns
        .into_iter()
        .filter(|column| column.name != POSITION)
        .collect();
    // The text the copy holds in a column for a row, in the file's way.
    let text_at = |column: &crate::ColumnInfo, at: i64| -> Result<String> {
        let name = Dialect::DuckDb.quote_ident(&column.name);
        let kind = column.type_name.to_ascii_uppercase();
        let expression = match (&layout.date, &layout.timestamp) {
            (Some(date), _) if kind == "DATE" => format!("strftime({name}, {})", literal(date)),
            (_, Some(stamp)) if kind.starts_with("TIMESTAMP") => {
                format!("strftime({name}, {})", literal(stamp))
            }
            _ => format!("CAST({name} AS VARCHAR)"),
        };
        let text: Option<String> = connection
            .query_row(
                &format!("SELECT {expression} FROM {table} WHERE {position} = {at}"),
                [],
                |row| row.get(0),
            )
            .map_err(map_error)?;
        Ok(text.unwrap_or_default())
    };
    let own_way = |type_name: &str| {
        let kind = type_name.to_ascii_uppercase();
        (kind == "DATE" && layout.date.is_some())
            || (kind.starts_with("TIMESTAMP") && layout.timestamp.is_some())
    };
    let cell = |column: &crate::ColumnInfo, new: &NewValue, at: i64| -> Result<String> {
        match new {
            NewValue::Null | NewValue::Default { .. } => Ok(String::new()),
            NewValue::Text(text) if !own_way(&column.type_name) => Ok(text.clone()),
            NewValue::Text(_) | NewValue::Now => text_at(column, at),
        }
    };
    let mut plan = Plan::default();
    for row in &changes.rows {
        let at = place_of(&row.key).ok_or_else(|| Error::query("a row to save has no place"))?;
        let mut cells = Vec::with_capacity(row.set.len());
        for change in &row.set {
            let place = columns
                .iter()
                .position(|column| column.name == change.column)
                .ok_or_else(|| Error::query(format!("no such column: {}", change.column)))?;
            cells.push((place, cell(&columns[place], &change.new, at)?));
        }
        plan.changed
            .insert(u64::try_from(at).unwrap_or(u64::MAX), cells);
    }
    for row in &changes.deletes {
        let at = place_of(&row.key).ok_or_else(|| Error::query("a row to delete has no place"))?;
        plan.deleted.insert(u64::try_from(at).unwrap_or(u64::MAX));
    }
    // The new rows, last in the copy in their order.
    let first: i64 = connection
        .query_row(
            &format!("SELECT coalesce(max({position}), -1) FROM {table}"),
            [],
            |row| row.get(0),
        )
        .map_err(map_error)?;
    let first = first + 1 - changes.inserts.len() as i64;
    for (index, insert) in changes.inserts.iter().enumerate() {
        let at = first + index as i64;
        let mut fields = Vec::with_capacity(columns.len());
        for column in &columns {
            let text = match insert.set.iter().find(|set| set.column == column.name) {
                Some(set) => cell(column, &set.new, at)?,
                None => String::new(),
            };
            fields.push(text);
        }
        plan.added.push(fields);
    }
    Ok(plan)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn a_copy_refused_for_permission_is_not_the_folder_s_doing() {
        let said = "IO Error: Cannot open file \"x\": Permission denied";
        assert!(matches!(copy_failed(said), WriteError::Io(error) if error.to_string() == said));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn a_hidden_file_another_program_replaced_is_neither_put_in_place_nor_removed() {
        use crate::adapter::Adapter;
        let dir = tempfile::tempdir().unwrap();
        crate::fixtures::write_bookshop_files(dir.path()).unwrap();
        let path = dir.path().join("orders.csv");
        let before = std::fs::read(&path).unwrap();
        let source = crate::files::Source::Folder(dir.path().to_owned());
        let conn = super::super::Conn::open(
            &source,
            &Reading::default(),
            false,
            Vec::new(),
            None,
            crate::Access::Writable,
        )
        .await
        .unwrap();
        let query = RowQuery::new(ObjectRef::new("main", "orders"), 100);
        let page = conn.fetch_rows(&query).await.unwrap();
        let copy = page.positions.as_ref().unwrap().copy;
        let changes = ChangeSet {
            object: query.object.clone(),
            inserts: Vec::new(),
            rows: Vec::new(),
            deletes: vec![crate::RowDelete {
                key: vec![
                    (POSITION.to_owned(), Value::Int(0)),
                    (FILE_COPY.to_owned(), Value::Int(copy)),
                ],
            }],
        };
        *REPLACE_HIDDEN.lock().unwrap() = Some(path.clone());
        let error = conn.write(&changes, &StopFlag::new()).await.unwrap_err();
        *REPLACE_HIDDEN.lock().unwrap() = None;
        let Error::Query { named, .. } = &error else {
            panic!("{error:?}");
        };
        assert!(matches!(&named.unsaved, Some(Unsaved::Failed(file, _)) if file == "orders.csv"));
        assert_eq!(std::fs::read(&path).unwrap(), before);
        let hidden = dir.path().join(".orders.csv.tabletist-save");
        assert_eq!(
            std::fs::read_to_string(&hidden).unwrap(),
            "another program's"
        );
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn on_windows_no_other_program_takes_the_hidden_file_mid_save() {
        use crate::adapter::Adapter;
        let dir = tempfile::tempdir().unwrap();
        crate::fixtures::write_bookshop_files(dir.path()).unwrap();
        let path = dir.path().join("orders.csv");
        let source = crate::files::Source::Folder(dir.path().to_owned());
        let conn = super::super::Conn::open(
            &source,
            &Reading::default(),
            false,
            Vec::new(),
            None,
            crate::Access::Writable,
        )
        .await
        .unwrap();
        let query = RowQuery::new(ObjectRef::new("main", "orders"), 100);
        let page = conn.fetch_rows(&query).await.unwrap();
        let copy = page.positions.as_ref().unwrap().copy;
        let changes = ChangeSet {
            object: query.object.clone(),
            inserts: Vec::new(),
            rows: Vec::new(),
            deletes: vec![crate::RowDelete {
                key: vec![
                    (POSITION.to_owned(), Value::Int(0)),
                    (FILE_COPY.to_owned(), Value::Int(copy)),
                ],
            }],
        };
        *REPLACE_HIDDEN.lock().unwrap() = Some(path.clone());
        let outcome = conn.write(&changes, &StopFlag::new()).await;
        *REPLACE_HIDDEN.lock().unwrap() = None;
        assert_eq!(*REPLACED.lock().unwrap(), Some(false));
        assert!(
            matches!(outcome, Ok(WriteOutcome::Written { .. })),
            "{outcome:?}"
        );
        assert_eq!(std::fs::read_to_string(&path).unwrap().lines().count(), 12);
    }

    #[tokio::test]
    async fn a_save_that_wrote_the_file_says_so_though_its_copy_fails_after() {
        use crate::adapter::Adapter;
        let dir = tempfile::tempdir().unwrap();
        crate::fixtures::write_bookshop_files(dir.path()).unwrap();
        let source = crate::files::Source::Folder(dir.path().to_owned());
        let conn = super::super::Conn::open(
            &source,
            &Reading::default(),
            false,
            Vec::new(),
            None,
            crate::Access::Writable,
        )
        .await
        .unwrap();
        let query = RowQuery::new(ObjectRef::new("main", "orders"), 100);
        let page = conn.fetch_rows(&query).await.unwrap();
        let copy = page.positions.as_ref().unwrap().copy;
        let key = |at: i64| {
            vec![
                (POSITION.to_owned(), Value::Int(at)),
                (FILE_COPY.to_owned(), Value::Int(copy)),
            ]
        };
        let delete = |at| ChangeSet {
            object: query.object.clone(),
            inserts: Vec::new(),
            rows: Vec::new(),
            deletes: vec![crate::RowDelete { key: key(at) }],
        };
        *FAIL_AFTER_RENAME.lock().unwrap() = Some(dir.path().join("orders.csv"));
        let outcome = conn.write(&delete(0), &StopFlag::new()).await;
        *FAIL_AFTER_RENAME.lock().unwrap() = None;
        // The file is written, and the save says so.
        assert!(
            matches!(outcome, Ok(WriteOutcome::Written { .. })),
            "{outcome:?}"
        );
        let text = std::fs::read_to_string(dir.path().join("orders.csv")).unwrap();
        assert_eq!(text.lines().count(), 12);
        // The next page reads it again, a copy of its own.
        let again = conn.fetch_rows(&query).await.unwrap();
        assert_eq!(again.rows.len(), 11);
        assert_ne!(again.positions.unwrap().copy, copy);
        // And the old page's rows are refused.
        let error = conn.write(&delete(1), &StopFlag::new()).await.unwrap_err();
        let Error::Query { named, .. } = &error else {
            panic!("{error:?}");
        };
        assert_eq!(named.unsaved, Some(Unsaved::Reread("orders.csv".into())));
    }

    fn layout(columns: usize) -> Layout {
        Layout {
            delimiter: b',',
            quote: Some(b'"'),
            escape: Some(b'"'),
            header: true,
            newline: b"\n",
            columns,
            date: None,
            timestamp: None,
        }
    }

    fn rewritten(text: &str, layout: &Layout, plan: &Plan) -> String {
        let mut out = Vec::new();
        rewrite(text.as_bytes(), &mut out, layout, plan).unwrap();
        String::from_utf8(out).unwrap()
    }

    #[test]
    fn rows_are_counted_as_duckdb_reads_them() {
        let cases = [
            ("id,title\n1,Dune\n2,Emma\n", 2, 2),
            ("id,title\n1,Dune\n\n2,Emma\n\n", 2, 2),
            ("id,title\n1,\"Du\nne\"\n2,Emma\n", 2, 2),
            ("id,title\r\n1,Dune\r\n2,Emma\r\n", 2, 2),
            ("\u{feff}id,title\n1,Dune\n2,Emma", 2, 2),
            ("\"id\",\"title\"\n\"1\",\"Dune, \"\"the\"\" book\"\n", 2, 1),
            ("title\nDune\n\nEmma\n", 1, 3),
        ];
        for (text, columns, rows) in cases {
            assert_eq!(
                count(text.as_bytes(), &layout(columns)).unwrap(),
                rows,
                "{text:?}"
            );
        }
    }

    #[test]
    fn a_save_keeps_every_untouched_row_as_it_was_written() {
        let text = "\u{feff}id,title,price\r\n1,\"Dune\",12.50\r\n\r\n2,Emma,9.00\r\n3,\"Ulysses, a novel\",15.00\r\n";
        let mut layout = layout(3);
        layout.newline = b"\r\n";
        let mut plan = Plan::default();
        plan.changed.insert(1, vec![(2, "8.5".into())]);
        plan.deleted.insert(0);
        plan.added
            .push(vec!["4".into(), "Beloved, a novel".into(), String::new()]);
        assert_eq!(
            rewritten(text, &layout, &plan),
            "\u{feff}id,title,price\r\n\r\n2,Emma,8.5\r\n3,\"Ulysses, a novel\",15.00\r\n\
             4,\"Beloved, a novel\",\r\n"
        );
    }

    #[test]
    fn a_new_row_starts_on_a_line_of_its_own() {
        let mut plan = Plan::default();
        plan.added.push(vec!["3".into(), "Emma \"E\"".into()]);
        assert_eq!(
            rewritten("id,title\n1,Dune", &layout(2), &plan),
            "id,title\n1,Dune\n3,\"Emma \"\"E\"\"\"\n"
        );
    }

    #[test]
    fn a_new_row_follows_the_last_row_written_not_the_last_read() {
        // The last row, with no line break, is deleted: the row before it
        // ended its line, and the new row goes right after.
        let mut plan = Plan::default();
        plan.deleted.insert(1);
        plan.added.push(vec!["Cara".into()]);
        assert_eq!(
            rewritten("name\nAlice\nBob", &layout(1), &plan),
            "name\nAlice\nCara\n"
        );
        // Kept, it still needs one.
        let mut plan = Plan::default();
        plan.added.push(vec!["Cara".into()]);
        assert_eq!(
            rewritten("name\nAlice\nBob", &layout(1), &plan),
            "name\nAlice\nBob\nCara\n"
        );
    }

    #[test]
    fn a_tsv_keeps_its_tabs_and_a_field_s_quotes() {
        let mut layout = layout(2);
        layout.delimiter = b'\t';
        let mut plan = Plan::default();
        plan.changed.insert(0, vec![(1, "Dune, Messiah".into())]);
        assert_eq!(
            rewritten("id\ttitle\n1\t\"Dune\"\n2\t\"Emma\"\n", &layout, &plan),
            "id\ttitle\n1\tDune, Messiah\n2\t\"Emma\"\n"
        );
    }
}
