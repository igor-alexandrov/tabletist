//! The text of an export and of Copy as: six formats, each written with
//! the options a person last chose for it.
//!
//! The values are the database's, never the grid's display: no digit
//! grouping, no value tags, no display zone. JSON's dates are the one
//! exception: ISO 8601, with an offset where the value has one. A writer
//! takes a row at a time, so an export can stream through it; Markdown
//! alone keeps its rows until the end, to pad its columns.
//!
//! The options are kept in the state directory
//! (`AppDirs::export_options_file`), by format. What the file holds that
//! this version cannot read is kept and written back.

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tabletist_db::{Dialect, ObjectRef, Value, ValueKind};

/// The most rows the clipboard takes; more go to a file.
pub const CLIPBOARD_ROWS: usize = 10_000;
/// The most text the clipboard takes, in bytes: 20 MB.
pub const CLIPBOARD_BYTES: usize = 20 * 1024 * 1024;

/// A format an export writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Format {
    Csv,
    Tsv,
    Json,
    JsonLines,
    SqlInsert,
    Markdown,
}

impl Format {
    pub const ALL: [Format; 6] = [
        Format::Csv,
        Format::Tsv,
        Format::Json,
        Format::JsonLines,
        Format::SqlInsert,
        Format::Markdown,
    ];

    /// Its key in the file of options.
    pub fn key(self) -> &'static str {
        match self {
            Self::Csv => "csv",
            Self::Tsv => "tsv",
            Self::Json => "json",
            Self::JsonLines => "jsonl",
            Self::SqlInsert => "sql",
            Self::Markdown => "md",
        }
    }

    /// Its name, as the menus write it.
    pub fn name(self) -> &'static str {
        match self {
            Self::Csv => "CSV",
            Self::Tsv => "TSV",
            Self::Json => "JSON",
            Self::JsonLines => "JSON Lines",
            Self::SqlInsert => "SQL INSERT",
            Self::Markdown => "Markdown",
        }
    }
}

/// What a NULL is written as.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NullAs {
    Empty,
    /// The option's text: `\N`, `NULL`.
    Text,
}

/// How bytes are written, where no SQL literal is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Binary {
    /// `0x` and two digits a byte.
    Hex,
    Base64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Quoting {
    /// A value that holds the delimiter, a quote or a line break.
    WhenNeeded,
    /// Every value but a NULL written empty, so a NULL and `""` differ.
    Every,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LineEnding {
    Lf,
    Crlf,
}

impl LineEnding {
    fn text(self) -> &'static str {
        match self {
            Self::Lf => "\n",
            Self::Crlf => "\r\n",
        }
    }
}

/// How a json or jsonb value is written in JSON.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JsonValues {
    /// As JSON, in the document.
    Nested,
    /// As a string holding its text.
    Strings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Layout {
    /// Two spaces deeper with each level.
    Indented,
    OneLine,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LongValues {
    Full,
    /// At [`MARKDOWN_CUT`] characters, then `…`.
    Cut,
}

/// Where Markdown cuts a long value.
pub const MARKDOWN_CUT: usize = 200;
/// The widest a Markdown column is padded to; a longer value runs past.
const MARKDOWN_PAD: usize = 80;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Csv {
    pub header: bool,
    pub delimiter: char,
    pub quoting: Quoting,
    pub null_as: NullAs,
    pub null_text: String,
    pub bom: bool,
    pub line_ending: LineEnding,
    pub binary: Binary,
}

impl Default for Csv {
    fn default() -> Self {
        Self {
            header: true,
            delimiter: ',',
            quoting: Quoting::WhenNeeded,
            null_as: NullAs::Empty,
            null_text: "\\N".into(),
            bom: false,
            line_ending: LineEnding::Lf,
            binary: Binary::Hex,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tsv {
    pub header: bool,
    pub null_as: NullAs,
    pub null_text: String,
    pub bom: bool,
    pub line_ending: LineEnding,
    pub binary: Binary,
}

impl Default for Tsv {
    fn default() -> Self {
        Self {
            header: true,
            null_as: NullAs::Empty,
            null_text: "\\N".into(),
            bom: false,
            line_ending: LineEnding::Lf,
            binary: Binary::Hex,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Json {
    pub values: JsonValues,
    pub layout: Layout,
    pub binary: Binary,
}

impl Default for Json {
    fn default() -> Self {
        Self {
            values: JsonValues::Nested,
            layout: Layout::Indented,
            binary: Binary::Hex,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JsonLines {
    pub values: JsonValues,
    pub binary: Binary,
}

impl Default for JsonLines {
    fn default() -> Self {
        Self {
            values: JsonValues::Nested,
            binary: Binary::Hex,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sql {
    /// 1, 100 or 1,000.
    pub rows_per_insert: usize,
    /// Start with the table's `CREATE TABLE`, from the DDL view.
    pub create_table: bool,
}

impl Default for Sql {
    fn default() -> Self {
        Self {
            rows_per_insert: 100,
            create_table: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Markdown {
    pub null_as: NullAs,
    pub null_text: String,
    pub long_values: LongValues,
}

impl Default for Markdown {
    fn default() -> Self {
        Self {
            null_as: NullAs::Text,
            null_text: "NULL".into(),
            long_values: LongValues::Full,
        }
    }
}

/// Every format's options, as last chosen.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Options {
    pub csv: Csv,
    pub tsv: Tsv,
    pub json: Json,
    pub jsonl: JsonLines,
    pub sql: Sql,
    pub md: Markdown,
    /// What the file held of each format that this version could not
    /// read (an option it does not know, a value it cannot read), by the
    /// format's key: written back over the options, as the file had it.
    kept: BTreeMap<String, serde_json::Map<String, serde_json::Value>>,
    /// The file's other keys, written back as they were.
    rest: BTreeMap<String, serde_json::Value>,
}

/// What a format's entry held that this version could not read.
type Unread = serde_json::Map<String, serde_json::Value>;

impl Options {
    /// The options in `path`: the defaults where it is missing or does not
    /// read.
    pub fn load(path: &Path) -> Self {
        Self::from_file(crate::util::load_json(path))
    }

    /// The options a file of them holds.
    pub fn from_file(mut file: BTreeMap<String, serde_json::Value>) -> Self {
        let mut kept = BTreeMap::new();
        let csv = take(&mut file, Format::Csv, &mut kept);
        let tsv = take(&mut file, Format::Tsv, &mut kept);
        let json = take(&mut file, Format::Json, &mut kept);
        let jsonl = take(&mut file, Format::JsonLines, &mut kept);
        let sql = take(&mut file, Format::SqlInsert, &mut kept);
        let md = take(&mut file, Format::Markdown, &mut kept);
        Self {
            csv,
            tsv,
            json,
            jsonl,
            sql,
            md,
            kept,
            rest: file,
        }
    }

    /// The file of these options.
    pub fn to_file(&self) -> BTreeMap<String, serde_json::Value> {
        let mut file = self.rest.clone();
        let kept = &self.kept;
        put(&mut file, Format::Csv, &self.csv, kept);
        put(&mut file, Format::Tsv, &self.tsv, kept);
        put(&mut file, Format::Json, &self.json, kept);
        put(&mut file, Format::JsonLines, &self.jsonl, kept);
        put(&mut file, Format::SqlInsert, &self.sql, kept);
        put(&mut file, Format::Markdown, &self.md, kept);
        file
    }

    /// `format`'s options were chosen again: what was kept of them unread
    /// gives way to them, and only an option this version does not know
    /// is still written back.
    pub fn chosen(&mut self, format: Format) {
        let known = match format {
            Format::Csv => keys(&self.csv),
            Format::Tsv => keys(&self.tsv),
            Format::Json => keys(&self.json),
            Format::JsonLines => keys(&self.jsonl),
            Format::SqlInsert => keys(&self.sql),
            Format::Markdown => keys(&self.md),
        };
        if let Some(unread) = self.kept.get_mut(format.key()) {
            unread.retain(|key, _| !known.contains(key));
        }
        self.rest.remove(format.key());
    }
}

/// The names of the options `options` writes.
fn keys<T: Serialize>(options: &T) -> Vec<String> {
    match serde_json::to_value(options) {
        Ok(serde_json::Value::Object(entry)) => entry.keys().cloned().collect(),
        _ => Vec::new(),
    }
}

/// `format`'s options from `file`: each option the entry holds that reads,
/// the default for the others. What does not read is put in `kept`.
fn take<T: Serialize + DeserializeOwned + Default>(
    file: &mut BTreeMap<String, serde_json::Value>,
    format: Format,
    kept: &mut BTreeMap<String, Unread>,
) -> T {
    let Ok(serde_json::Value::Object(mut merged)) = serde_json::to_value(T::default()) else {
        return T::default();
    };
    let entry = match file.remove(format.key()) {
        Some(serde_json::Value::Object(entry)) => entry,
        // An entry this version cannot read at all stays in the file, as
        // its other keys do, until the format's options are chosen again.
        Some(other) => {
            file.insert(format.key().to_owned(), other);
            return T::default();
        }
        None => return T::default(),
    };
    let mut unread = Unread::new();
    for (key, value) in entry {
        let Some(before) = merged.insert(key.clone(), value.clone()) else {
            // An option this version does not know.
            merged.remove(&key);
            unread.insert(key, value);
            continue;
        };
        let reads = serde_json::from_value::<T>(serde_json::Value::Object(merged.clone())).is_ok();
        if !reads {
            merged.insert(key.clone(), before);
            unread.insert(key, value);
        }
    }
    if !unread.is_empty() {
        kept.insert(format.key().to_owned(), unread);
    }
    serde_json::from_value(serde_json::Value::Object(merged)).unwrap_or_default()
}

/// Writes `options` into `file` as `format`'s entry, with what was kept of
/// it over them.
fn put<T: Serialize>(
    file: &mut BTreeMap<String, serde_json::Value>,
    format: Format,
    options: &T,
    kept: &BTreeMap<String, Unread>,
) {
    let mut entry = match serde_json::to_value(options) {
        Ok(serde_json::Value::Object(entry)) => entry,
        _ => Unread::new(),
    };
    if let Some(unread) = kept.get(format.key()) {
        entry.extend(unread.clone());
    }
    // Not over an entry kept as the file had it.
    file.entry(format.key().to_owned())
        .or_insert(serde_json::Value::Object(entry));
}

/// The columns a writer writes, in the order written, and the dialect
/// that writes their names and SQL INSERT's literals.
#[derive(Debug, Clone)]
pub struct Columns<'a> {
    pub names: Vec<&'a str>,
    pub kinds: Vec<ValueKind>,
    pub dialect: Dialect,
}

/// The table SQL INSERT writes into.
#[derive(Debug, Clone, Copy)]
pub enum Target<'a> {
    /// The table the rows are of, schema-qualified, and its `CREATE
    /// TABLE` to start with, where the options ask for it.
    Table {
        object: &'a ObjectRef,
        create: Option<&'a str>,
    },
    /// A name of no table: `result`, for a SQL result's rows.
    Named(&'a str),
}

/// A cell a writer reads: `None` is a new row's cell that nothing is set
/// in, written as a NULL is.
pub trait Cell {
    fn value(&self) -> Option<&Value>;

    /// What SQL INSERT writes in its place, where that is no literal of its
    /// value: a pending DEFAULT, or the time of the save.
    fn sql(&self) -> Option<&str> {
        None
    }
}

impl Cell for Value {
    fn value(&self) -> Option<&Value> {
        Some(self)
    }
}

impl Cell for Option<Value> {
    fn value(&self) -> Option<&Value> {
        self.as_ref()
    }
}

/// A cell of a grid with its pending changes: the value it shows, and for
/// a pending DEFAULT or time of the save what a save would send.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Pending {
    pub value: Option<Value>,
    pub sql: Option<String>,
}

impl Cell for Pending {
    fn value(&self) -> Option<&Value> {
        self.value.as_ref()
    }

    fn sql(&self) -> Option<&str> {
        self.sql.as_deref()
    }
}

/// Writes one format's text, a row at a time: [`Writer::start`],
/// [`Writer::row`] for each row, then [`Writer::finish`].
pub struct Writer<'a> {
    format: Format,
    options: &'a Options,
    columns: Columns<'a>,
    target: Target<'a>,
    rows: usize,
    statements: usize,
    /// Markdown's cells, as written, kept to be padded at the end.
    kept: Vec<Vec<String>>,
    /// How much text those cells hold.
    kept_bytes: usize,
}

impl<'a> Writer<'a> {
    pub fn new(
        format: Format,
        options: &'a Options,
        columns: Columns<'a>,
        target: Target<'a>,
    ) -> Self {
        Self {
            format,
            options,
            columns,
            target,
            rows: 0,
            statements: 0,
            kept: Vec::new(),
            kept_bytes: 0,
        }
    }

    /// How much text the rows kept for the end hold: Markdown's, which
    /// [`Writer::finish`] writes.
    pub fn held(&self) -> usize {
        self.kept_bytes
    }

    /// How many INSERT statements were written.
    pub fn statements(&self) -> usize {
        self.statements
    }

    /// What comes before the rows: a byte-order mark, a header row, an
    /// array's opening, a `CREATE TABLE`.
    pub fn start(&mut self, out: &mut String) {
        match self.format {
            Format::Csv => {
                let csv = &self.options.csv;
                if csv.bom {
                    out.push('\u{feff}');
                }
                if csv.header {
                    let names = self.columns.names.iter();
                    let cells: Vec<String> = names
                        .map(|name| csv_quoted(name, csv, csv.quoting == Quoting::Every))
                        .collect();
                    line(out, &cells, &csv.delimiter.to_string(), csv.line_ending);
                }
            }
            Format::Tsv => {
                let tsv = &self.options.tsv;
                if tsv.bom {
                    out.push('\u{feff}');
                }
                if tsv.header {
                    let names = self.columns.names.iter();
                    let cells: Vec<String> = names.map(|name| tsv_escaped(name)).collect();
                    line(out, &cells, "\t", tsv.line_ending);
                }
            }
            Format::Json => out.push('['),
            Format::SqlInsert => {
                let create = match self.target {
                    Target::Table { create, .. } if self.options.sql.create_table => create,
                    _ => None,
                };
                if let Some(create) = create.map(str::trim_end).filter(|text| !text.is_empty()) {
                    out.push_str(create);
                    if !create.ends_with(';') {
                        out.push(';');
                    }
                    out.push_str("\n\n");
                }
            }
            Format::JsonLines | Format::Markdown => {}
        }
    }

    /// One row, its cells in the columns' order.
    pub fn row<C: Cell>(&mut self, cells: &[C], out: &mut String) {
        let values = || cells.iter().map(Cell::value);
        match self.format {
            Format::Csv => {
                let csv = &self.options.csv;
                let written: Vec<String> = values().map(|value| csv_cell(value, csv)).collect();
                line(out, &written, &csv.delimiter.to_string(), csv.line_ending);
            }
            Format::Tsv => {
                let tsv = &self.options.tsv;
                let written: Vec<String> = values().map(|value| tsv_cell(value, tsv)).collect();
                line(out, &written, "\t", tsv.line_ending);
            }
            Format::Json => {
                let json = &self.options.json;
                if self.rows > 0 {
                    out.push(',');
                }
                let indented = json.layout == Layout::Indented;
                if indented {
                    out.push_str("\n  ");
                }
                let depth = usize::from(indented);
                self.object(values(), json.values, json.binary, indented, depth, out);
            }
            Format::JsonLines => {
                let jsonl = &self.options.jsonl;
                self.object(values(), jsonl.values, jsonl.binary, false, 0, out);
                out.push('\n');
            }
            Format::SqlInsert => self.insert(cells, out),
            Format::Markdown => {
                let md = &self.options.md;
                let written: Vec<String> = values().map(|value| markdown_cell(value, md)).collect();
                // A cell, its padding aside, and the ` | ` after it.
                self.kept_bytes += written.iter().map(|cell| cell.len() + 3).sum::<usize>();
                self.kept.push(written);
            }
        }
        self.rows += 1;
    }

    /// What comes after the rows: an array's end, the last statement's
    /// `;`, a Markdown table whole.
    pub fn finish(&mut self, out: &mut String) {
        match self.format {
            Format::Json => {
                if self.rows > 0 && self.options.json.layout == Layout::Indented {
                    out.push('\n');
                }
                out.push_str("]\n");
            }
            Format::SqlInsert if self.rows > 0 => out.push_str(";\n"),
            Format::Markdown => self.markdown(out),
            _ => {}
        }
    }

    /// A row as a JSON object, keys in the columns' order. Indented, its
    /// members are a line each, `depth` levels in.
    fn object<'v>(
        &self,
        values: impl Iterator<Item = Option<&'v Value>>,
        nested: JsonValues,
        binary: Binary,
        indented: bool,
        depth: usize,
        out: &mut String,
    ) {
        let inner = "  ".repeat(depth + 1);
        out.push('{');
        let mut any = false;
        for (col, value) in values.enumerate() {
            if any {
                out.push(',');
            }
            any = true;
            if indented {
                out.push('\n');
                out.push_str(&inner);
            }
            let name = self.columns.names.get(col).copied().unwrap_or_default();
            out.push_str(&json_string(name));
            out.push(':');
            if indented {
                out.push(' ');
            }
            let kind = self
                .columns
                .kinds
                .get(col)
                .copied()
                .unwrap_or(ValueKind::Other);
            let written = json_value(value, kind, nested, binary, indented);
            match (indented, kind == ValueKind::Json) {
                // A nested document's later lines, as deep as its member.
                (true, true) => out.push_str(&written.replace('\n', &format!("\n{inner}"))),
                _ => out.push_str(&written),
            }
        }
        if indented && any {
            out.push('\n');
            out.push_str(&"  ".repeat(depth));
        }
        out.push('}');
    }

    /// A row of an INSERT: the statement's head on the first of each run
    /// of `rows_per_insert` rows.
    fn insert<C: Cell>(&mut self, cells: &[C], out: &mut String) {
        let dialect = self.columns.dialect;
        let per = self.options.sql.rows_per_insert.max(1);
        let tuple: Vec<String> = cells
            .iter()
            .enumerate()
            .map(|(col, cell)| match cell.sql() {
                Some(sql) => sql.to_owned(),
                None => {
                    let kinds = self.columns.kinds.get(col).copied();
                    let kind = kinds.unwrap_or(ValueKind::Other);
                    dialect.value_literal(cell.value().unwrap_or(&Value::Null), kind)
                }
            })
            .collect();
        let tuple = format!("({})", tuple.join(", "));
        if self.rows.is_multiple_of(per) {
            if self.rows > 0 {
                out.push_str(";\n");
            }
            let table = match self.target {
                Target::Table { object, .. } => {
                    format!(
                        "{}.{}",
                        dialect.ident(&object.schema),
                        dialect.ident(&object.name)
                    )
                }
                Target::Named(name) => dialect.ident(name).into_owned(),
            };
            let names: Vec<_> = self
                .columns
                .names
                .iter()
                .map(|name| dialect.ident(name))
                .collect();
            let _ = write!(out, "INSERT INTO {table} ({}) VALUES", names.join(", "));
            out.push_str(if per == 1 { " " } else { "\n  " });
            self.statements += 1;
        } else {
            out.push_str(",\n  ");
        }
        out.push_str(&tuple);
    }

    /// The Markdown table: each column padded to its widest cell (to
    /// [`MARKDOWN_PAD`] characters), numbers to the right.
    fn markdown(&mut self, out: &mut String) {
        let header: Vec<String> = self
            .columns
            .names
            .iter()
            .map(|name| markdown_escaped(name))
            .collect();
        let count = |text: &str| text.chars().count();
        let widths: Vec<usize> = (0..header.len())
            .map(|col| {
                let cells = self.kept.iter().filter_map(|row| row.get(col));
                let widest = cells
                    .map(|cell| count(cell))
                    .chain([count(&header[col])])
                    .max();
                widest.unwrap_or(0).clamp(1, MARKDOWN_PAD)
            })
            .collect();
        let right = |col: usize| self.columns.kinds.get(col) == Some(&ValueKind::Numeric);
        let row = |out: &mut String, cells: &[String]| {
            out.push('|');
            for (col, width) in widths.iter().enumerate() {
                let cell = cells.get(col).map_or("", String::as_str);
                let pad = " ".repeat(width.saturating_sub(count(cell)));
                if right(col) {
                    let _ = write!(out, " {pad}{cell} |");
                } else {
                    let _ = write!(out, " {cell}{pad} |");
                }
            }
            out.push('\n');
        };
        row(out, &header);
        out.push('|');
        for (col, width) in widths.iter().enumerate() {
            if right(col) {
                let _ = write!(out, "{}:|", "-".repeat(width + 1));
            } else {
                let _ = write!(out, "{}|", "-".repeat(width + 2));
            }
        }
        out.push('\n');
        for cells in std::mem::take(&mut self.kept) {
            row(out, &cells);
        }
    }
}

/// `cells` as one line, ended as the options say.
fn line(out: &mut String, cells: &[String], delimiter: &str, ending: LineEnding) {
    out.push_str(&cells.join(delimiter));
    out.push_str(ending.text());
}

/// A value as plain text: what CSV, TSV and Markdown write of it.
fn plain(value: &Value, binary: Binary) -> Cow<'_, str> {
    match value {
        Value::Null => Cow::Borrowed(""),
        Value::Bool(flag) => Cow::Borrowed(if *flag { "true" } else { "false" }),
        Value::Int(number) => Cow::Owned(number.to_string()),
        Value::Float(number) => Cow::Owned(crate::format::float_text(*number)),
        Value::Text(text) => Cow::Borrowed(text),
        Value::Bytes(bytes) => Cow::Owned(binary_text(bytes, binary)),
    }
}

/// Bytes as `0x` hex, or as Base64.
pub fn binary_text(bytes: &[u8], binary: Binary) -> String {
    match binary {
        Binary::Hex => {
            let mut hex = String::with_capacity(2 + bytes.len() * 2);
            hex.push_str("0x");
            for byte in bytes {
                let _ = write!(hex, "{byte:02x}");
            }
            hex
        }
        Binary::Base64 => base64(bytes),
    }
}

/// Standard Base64, with its padding.
fn base64(bytes: &[u8]) -> String {
    const LETTERS: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut text = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let at = |index: usize| chunk.get(index).copied().map_or(0, u32::from);
        let triple = (at(0) << 16) | (at(1) << 8) | at(2);
        for (index, shift) in [18, 12, 6, 0].into_iter().enumerate() {
            if index <= chunk.len() {
                text.push(char::from(LETTERS[((triple >> shift) & 63) as usize]));
            } else {
                text.push('=');
            }
        }
    }
    text
}

fn csv_cell(value: Option<&Value>, csv: &Csv) -> String {
    match value {
        None | Some(Value::Null) => match csv.null_as {
            NullAs::Empty => String::new(),
            // Never quoted unless it must be: a quoted one reads as text.
            NullAs::Text => csv_quoted(&csv.null_text, csv, false),
        },
        Some(value) => csv_quoted(
            &plain(value, csv.binary),
            csv,
            csv.quoting == Quoting::Every,
        ),
    }
}

/// `text` in quotes where `always` says or it holds the delimiter, a quote
/// or a line break, its quotes doubled.
fn csv_quoted(text: &str, csv: &Csv, always: bool) -> String {
    let needs = text.contains([csv.delimiter, '"', '\n', '\r']);
    if always || needs {
        format!("\"{}\"", text.replace('"', "\"\""))
    } else {
        text.to_owned()
    }
}

fn tsv_cell(value: Option<&Value>, tsv: &Tsv) -> String {
    match value {
        None | Some(Value::Null) => match tsv.null_as {
            NullAs::Empty => String::new(),
            NullAs::Text => tsv.null_text.clone(),
        },
        Some(value) => tsv_escaped(&plain(value, tsv.binary)),
    }
}

/// `text` with a backslash, a tab and a line break written as PostgreSQL's
/// `COPY` writes them: `\\`, `\t`, `\n`, `\r`.
fn tsv_escaped(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '\\' => escaped.push_str("\\\\"),
            '\t' => escaped.push_str("\\t"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            other => escaped.push(other),
        }
    }
    escaped
}

fn markdown_cell(value: Option<&Value>, md: &Markdown) -> String {
    let text = match value {
        None | Some(Value::Null) => match md.null_as {
            NullAs::Empty => return String::new(),
            NullAs::Text => Cow::Borrowed(md.null_text.as_str()),
        },
        Some(value) => plain(value, Binary::Hex),
    };
    match md.long_values {
        LongValues::Cut if text.chars().count() > MARKDOWN_CUT => {
            let cut: String = text.chars().take(MARKDOWN_CUT).collect();
            markdown_escaped(&format!("{cut}…"))
        }
        _ => markdown_escaped(&text),
    }
}

/// `text` on one line of a table: a backslash as `\\`, a pipe as `\|`, a
/// line break as `<br>`.
fn markdown_escaped(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace("\r\n", "<br>")
        .replace(['\n', '\r'], "<br>")
        .replace('|', "\\|")
}

/// `text` as a JSON string.
fn json_string(text: &str) -> String {
    serde_json::to_string(text).unwrap_or_else(|_| "\"\"".to_owned())
}

/// A value as JSON, by its column's kind.
/// A nested document is laid out a member to a line where `indented`, on
/// one line otherwise.
fn json_value(
    value: Option<&Value>,
    kind: ValueKind,
    nested: JsonValues,
    binary: Binary,
    indented: bool,
) -> String {
    let Some(value) = value else {
        return "null".to_owned();
    };
    match value {
        Value::Null => "null".to_owned(),
        Value::Bool(flag) => flag.to_string(),
        Value::Int(number) => number.to_string(),
        Value::Float(number) if number.is_finite() => crate::format::float_text(*number),
        Value::Float(number) => json_string(&crate::format::float_text(*number)),
        Value::Bytes(bytes) => json_string(&binary_text(bytes, binary)),
        Value::Text(text) => match kind {
            ValueKind::Json if nested == JsonValues::Nested && is_json(text) => {
                if indented {
                    crate::json::pretty(text)
                } else {
                    compact(text)
                }
            }
            ValueKind::Numeric if tabletist_db::is_plain_number(text) => text.to_string(),
            ValueKind::Temporal => json_string(&iso_8601(text)),
            _ => json_string(text),
        },
    }
}

/// Whether `text` is one JSON document.
fn is_json(text: &str) -> bool {
    serde_json::from_str::<serde::de::IgnoredAny>(text).is_ok()
}

/// A JSON document on one line: the white space between its pieces gone.
fn compact(text: &str) -> String {
    crate::json::tokens(text)
        .into_iter()
        .filter(|(kind, _)| *kind != crate::json::Kind::Space)
        .map(|(_, range)| &text[range])
        .collect()
}

/// A date and time as ISO 8601: `T` between the two, and an offset as
/// `+HH:MM`. What is not a date and a time (a date, a time, an interval,
/// `infinity`) is left as it is.
pub fn iso_8601(text: &str) -> Cow<'_, str> {
    let bytes = text.as_bytes();
    let digits = |range: std::ops::Range<usize>| {
        bytes
            .get(range)
            .is_some_and(|part| part.iter().all(u8::is_ascii_digit))
    };
    let date_time = bytes.len() >= 19
        && digits(0..4)
        && bytes[4] == b'-'
        && digits(5..7)
        && bytes[7] == b'-'
        && digits(8..10)
        && matches!(bytes[10], b' ' | b'T')
        && digits(11..13)
        && bytes[13] == b':'
        && digits(14..16)
        && bytes[16] == b':'
        && digits(17..19);
    if !date_time {
        return Cow::Borrowed(text);
    }
    let mut at = 19;
    if bytes.get(at) == Some(&b'.') {
        at += 1;
        while bytes.get(at).is_some_and(u8::is_ascii_digit) {
            at += 1;
        }
    }
    let (moment, offset) = text.split_at(at);
    let offset = match offset.as_bytes() {
        [] => String::new(),
        [b'Z'] => "Z".to_owned(),
        // `+05`: an hour alone, as PostgreSQL writes a whole hour.
        [sign @ (b'+' | b'-'), h1, h2] if h1.is_ascii_digit() && h2.is_ascii_digit() => {
            format!(
                "{}{}{}:00",
                char::from(*sign),
                char::from(*h1),
                char::from(*h2)
            )
        }
        // `+05:30`, as it should be.
        [b'+' | b'-', h1, h2, b':', m1, m2]
            if [h1, h2, m1, m2].iter().all(|digit| digit.is_ascii_digit()) =>
        {
            offset.to_owned()
        }
        // `+0530`: the colon put in.
        [sign @ (b'+' | b'-'), h1, h2, m1, m2]
            if [h1, h2, m1, m2].iter().all(|digit| digit.is_ascii_digit()) =>
        {
            let [sign, h1, h2, m1, m2] = [sign, h1, h2, m1, m2].map(|byte| char::from(*byte));
            format!("{sign}{h1}{h2}:{m1}{m2}")
        }
        // Seconds in the offset, a BC year, anything else after the time:
        // left as it was, for no ISO 8601 writes it.
        _ => return Cow::Borrowed(text),
    };
    Cow::Owned(format!("{}T{}{offset}", &moment[..10], &moment[11..]))
}

/// What a copy puts on the clipboard, or why it puts nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Copied {
    Text {
        text: String,
        /// SQL INSERT's statements.
        statements: usize,
    },
    /// More rows than [`CLIPBOARD_ROWS`].
    TooManyRows(usize),
    /// More text than [`CLIPBOARD_BYTES`].
    TooLarge,
}

/// `rows` (`count` of them) in `format`, for the clipboard: no byte-order
/// mark, which a file is for, and no line ending after the last line.
/// Nothing is written past the clipboard's limits.
pub fn clipboard<C: Cell>(
    format: Format,
    options: &Options,
    columns: Columns<'_>,
    target: Target<'_>,
    count: usize,
    rows: impl Iterator<Item = Vec<C>>,
) -> Copied {
    if count > CLIPBOARD_ROWS {
        return Copied::TooManyRows(count);
    }
    let mut options = options.clone();
    options.csv.bom = false;
    options.tsv.bom = false;
    let mut writer = Writer::new(format, &options, columns, target);
    let mut text = String::new();
    writer.start(&mut text);
    for row in rows {
        writer.row(&row, &mut text);
        if text.len() + writer.held() > CLIPBOARD_BYTES {
            return Copied::TooLarge;
        }
    }
    writer.finish(&mut text);
    if text.len() > CLIPBOARD_BYTES {
        return Copied::TooLarge;
    }
    let ending = text
        .strip_suffix("\r\n")
        .or_else(|| text.strip_suffix('\n'));
    if let Some(kept) = ending {
        text.truncate(kept.len());
    }
    Copied::Text {
        statements: writer.statements(),
        text,
    }
}

/// The columns' names on one line, for a `SELECT` list: each as the
/// dialect writes a name, quoted only where it must be.
pub fn column_names(names: &[&str], dialect: Dialect) -> String {
    let names: Vec<_> = names.iter().map(|name| dialect.ident(name)).collect();
    names.join(", ")
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    /// Rows of Bookshop's `reviews`: a NULL, an empty string, a quote, a
    /// comma, a tab, a newline, a pipe, a backslash, unicode, jsonb, bytea
    /// and timestamptz.
    fn reviews() -> (Vec<&'static str>, Vec<ValueKind>, Vec<Vec<Value>>) {
        use ValueKind::{Binary, Json, Numeric, Temporal, Text};
        let names = vec![
            "id",
            "rating",
            "title",
            "body",
            "metadata",
            "cover",
            "created_at",
        ];
        let kinds = vec![Numeric, Numeric, Text, Text, Json, Binary, Temporal];
        let text = |text: &str| Value::Text(text.into());
        let rows = vec![
            vec![
                Value::Int(10),
                Value::Int(4),
                text("The Tide Atlas"),
                text("Maps the coast, one \"harbor\" at a time."),
                text(r#"{"tags": ["sea", "maps"], "helpful": 12}"#),
                Value::Bytes(vec![0x89, 0x50, 0x4e, 0x47].into()),
                text("2026-02-06 08:05:00+00"),
            ],
            vec![
                Value::Int(11),
                Value::Int(5),
                text("Harbor Lights"),
                text("Lovely\tsecond half\nand a pipe | here"),
                Value::Null,
                Value::Null,
                text("2026-02-06 08:06:12.5+00"),
            ],
            vec![
                Value::Int(12),
                Value::Null,
                text(""),
                text("Café Ünïcödé 東京, shelved under C:\\fiction"),
                text(r#"{"a": {"b": [1, 2.50]}, "z": null}"#),
                Value::Bytes(Vec::new().into()),
                text("2026-02-11 12:40:00+05:30"),
            ],
        ];
        (names, kinds, rows)
    }

    fn written(
        format: Format,
        options: &Options,
        dialect: Dialect,
        create: Option<&str>,
    ) -> String {
        let (names, kinds, rows) = reviews();
        let object = ObjectRef::new("public", "reviews");
        let columns = Columns {
            names,
            kinds,
            dialect,
        };
        let target = Target::Table {
            object: &object,
            create,
        };
        let mut writer = Writer::new(format, options, columns, target);
        let mut out = String::new();
        writer.start(&mut out);
        for row in &rows {
            writer.row(row, &mut out);
        }
        writer.finish(&mut out);
        out
    }

    /// The text `name` in `export/golden`, byte for byte.
    fn golden(name: &str) -> String {
        let path = format!("{}/src/export/golden/{name}", env!("CARGO_MANIFEST_DIR"));
        std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{path}: {error}"))
    }

    fn check(name: &str, format: Format, options: &Options) {
        check_in(name, format, options, Dialect::Postgres, None);
    }

    fn check_in(
        name: &str,
        format: Format,
        options: &Options,
        dialect: Dialect,
        create: Option<&str>,
    ) {
        let text = written(format, options, dialect, create);
        assert_eq!(text, golden(name), "{name}");
    }

    fn with(change: impl FnOnce(&mut Options)) -> Options {
        let mut options = Options::default();
        change(&mut options);
        options
    }

    #[test]
    fn csv_is_written_as_each_option_says() {
        check("csv-default.csv", Format::Csv, &Options::default());
        check(
            "csv-no-header.csv",
            Format::Csv,
            &with(|o| o.csv.header = false),
        );
        check(
            "csv-semicolon.csv",
            Format::Csv,
            &with(|o| o.csv.delimiter = ';'),
        );
        check(
            "csv-every-value.csv",
            Format::Csv,
            &with(|o| o.csv.quoting = Quoting::Every),
        );
        check(
            "csv-null-text.csv",
            Format::Csv,
            &with(|o| o.csv.null_as = NullAs::Text),
        );
        check("csv-bom.csv", Format::Csv, &with(|o| o.csv.bom = true));
        check(
            "csv-crlf.csv",
            Format::Csv,
            &with(|o| o.csv.line_ending = LineEnding::Crlf),
        );
        check(
            "csv-base64.csv",
            Format::Csv,
            &with(|o| o.csv.binary = Binary::Base64),
        );
    }

    #[test]
    fn tsv_is_written_as_each_option_says() {
        check("tsv-default.tsv", Format::Tsv, &Options::default());
        check(
            "tsv-no-header.tsv",
            Format::Tsv,
            &with(|o| o.tsv.header = false),
        );
        check(
            "tsv-null-text.tsv",
            Format::Tsv,
            &with(|o| o.tsv.null_as = NullAs::Text),
        );
    }

    #[test]
    fn json_is_written_as_each_option_says() {
        check("json-default.json", Format::Json, &Options::default());
        let strings = with(|o| o.json.values = JsonValues::Strings);
        check("json-strings.json", Format::Json, &strings);
        check(
            "json-one-line.json",
            Format::Json,
            &with(|o| o.json.layout = Layout::OneLine),
        );
        check(
            "jsonl-default.jsonl",
            Format::JsonLines,
            &Options::default(),
        );
    }

    #[test]
    fn sql_insert_is_written_as_each_option_says() {
        check("sql-default.sql", Format::SqlInsert, &Options::default());
        let one = with(|o| o.sql.rows_per_insert = 1);
        check("sql-one-per-insert.sql", Format::SqlInsert, &one);
        let create = "CREATE TABLE public.reviews (\n  id bigint PRIMARY KEY\n);\n";
        let started = with(|o| o.sql.create_table = true);
        check_in(
            "sql-create.sql",
            Format::SqlInsert,
            &started,
            Dialect::Postgres,
            Some(create),
        );
        // Asked for, with no definition to hand: none.
        check_in(
            "sql-default.sql",
            Format::SqlInsert,
            &started,
            Dialect::Postgres,
            None,
        );
        check_in(
            "sql-mysql.sql",
            Format::SqlInsert,
            &Options::default(),
            Dialect::MySql,
            None,
        );
        check_in(
            "sql-sqlite.sql",
            Format::SqlInsert,
            &Options::default(),
            Dialect::Sqlite,
            None,
        );
    }

    #[test]
    fn markdown_is_written_as_each_option_says() {
        check("md-default.md", Format::Markdown, &Options::default());
        check(
            "md-null-empty.md",
            Format::Markdown,
            &with(|o| o.md.null_as = NullAs::Empty),
        );
    }

    #[test]
    fn markdown_cuts_a_long_value_where_asked() {
        let options = with(|o| o.md.long_values = LongValues::Cut);
        let columns = Columns {
            names: vec!["body"],
            kinds: vec![ValueKind::Text],
            dialect: Dialect::Postgres,
        };
        let mut writer = Writer::new(Format::Markdown, &options, columns, Target::Named("result"));
        let mut out = String::new();
        writer.start(&mut out);
        writer.row(
            &[Value::Text("é".repeat(MARKDOWN_CUT + 1).into())],
            &mut out,
        );
        writer.finish(&mut out);
        let cell = out.lines().nth(2).unwrap();
        assert_eq!(cell, format!("| {}… |", "é".repeat(MARKDOWN_CUT)));
    }

    #[test]
    fn a_value_over_256_kib_is_written_whole_in_every_format() {
        let long = "q".repeat(300 * 1024);
        let columns = || Columns {
            names: vec!["body"],
            kinds: vec![ValueKind::Text],
            dialect: Dialect::Postgres,
        };
        let options = Options::default();
        for format in Format::ALL {
            let mut writer = Writer::new(format, &options, columns(), Target::Named("result"));
            let mut out = String::new();
            writer.start(&mut out);
            writer.row(&[Value::Text(long.as_str().into())], &mut out);
            writer.finish(&mut out);
            assert!(out.contains(&long), "{format:?}");
        }
    }

    #[test]
    fn rows_in_two_batches_write_what_one_batch_does() {
        let (names, kinds, rows) = reviews();
        let options = with(|o| o.sql.rows_per_insert = 2);
        for format in Format::ALL {
            let object = ObjectRef::new("public", "reviews");
            let target = Target::Table {
                object: &object,
                create: None,
            };
            let columns = Columns {
                names: names.clone(),
                kinds: kinds.clone(),
                dialect: Dialect::Postgres,
            };
            let mut writer = Writer::new(format, &options, columns, target);
            let (mut first, mut second) = (String::new(), String::new());
            writer.start(&mut first);
            writer.row(&rows[0], &mut first);
            for row in &rows[1..] {
                writer.row(row, &mut second);
            }
            writer.finish(&mut second);
            let whole = written(format, &options, Dialect::Postgres, None);
            assert_eq!(format!("{first}{second}"), whole, "{format:?}");
        }
    }

    #[test]
    fn json_dates_and_times_are_iso_8601() {
        assert_eq!(
            iso_8601("2026-02-06 08:05:00+00"),
            "2026-02-06T08:05:00+00:00"
        );
        assert_eq!(
            iso_8601("2026-02-06 08:05:00.25-08"),
            "2026-02-06T08:05:00.25-08:00"
        );
        assert_eq!(
            iso_8601("2026-02-06 08:05:00+05:30"),
            "2026-02-06T08:05:00+05:30"
        );
        assert_eq!(iso_8601("2026-02-06 08:05:00"), "2026-02-06T08:05:00");
        assert_eq!(
            iso_8601("2026-02-06 08:05:00+0530"),
            "2026-02-06T08:05:00+05:30"
        );
        assert_eq!(iso_8601("2026-02-06 08:05:00Z"), "2026-02-06T08:05:00Z");
        // What ISO 8601 has no way to write is left as the database wrote it.
        for kept in [
            "2026-02-06",
            "08:05:00",
            "1 day 02:00:00",
            "infinity",
            "2026-02-06 08:05:00 BC",
            "0044-03-15 12:00:00+00 BC",
            "1900-01-01 00:00:00+02:30:17",
        ] {
            assert_eq!(iso_8601(kept), kept);
        }
    }

    #[test]
    fn json_numbers_and_documents_keep_every_digit() {
        let numeric = Value::Text("123456789012345678901234567890.50".into());
        let written = json_value(
            Some(&numeric),
            ValueKind::Numeric,
            JsonValues::Nested,
            Binary::Hex,
            true,
        );
        assert_eq!(written, "123456789012345678901234567890.50");
        let money = Value::Text("$1.00".into());
        let written = json_value(
            Some(&money),
            ValueKind::Numeric,
            JsonValues::Nested,
            Binary::Hex,
            true,
        );
        assert_eq!(written, "\"$1.00\"");
        let broken = Value::Text("{nope".into());
        let written = json_value(
            Some(&broken),
            ValueKind::Json,
            JsonValues::Nested,
            Binary::Hex,
            true,
        );
        assert_eq!(written, "\"{nope\"");
        let nan = Value::Float(f64::NAN);
        let written = json_value(
            Some(&nan),
            ValueKind::Numeric,
            JsonValues::Nested,
            Binary::Hex,
            true,
        );
        assert_eq!(written, "\"NaN\"");
    }

    #[test]
    fn a_pending_default_is_written_as_a_save_sends_it() {
        let columns = Columns {
            names: vec!["id", "kind"],
            kinds: vec![ValueKind::Numeric, ValueKind::Text],
            dialect: Dialect::Postgres,
        };
        let row = vec![
            Pending {
                value: Some(Value::Int(2)),
                sql: None,
            },
            Pending {
                value: Some(Value::Text("DEFAULT".into())),
                sql: Some("DEFAULT".into()),
            },
        ];
        let options = Options::default();
        let target = Target::Named("result");
        let copied = clipboard(
            Format::SqlInsert,
            &options,
            columns.clone(),
            target,
            1,
            [row.clone()].into_iter(),
        );
        let Copied::Text { text, .. } = copied else {
            panic!("{copied:?}");
        };
        assert_eq!(
            text,
            "INSERT INTO result (id, kind) VALUES\n  (2, DEFAULT);"
        );
        // Elsewhere it is the word the grid shows.
        let copied = clipboard(Format::Csv, &options, columns, target, 1, [row].into_iter());
        let Copied::Text { text, .. } = copied else {
            panic!("{copied:?}");
        };
        assert_eq!(text, "id,kind\n2,DEFAULT");
    }

    #[test]
    fn markdown_stops_once_past_the_clipboards_limit() {
        let columns = Columns {
            names: vec!["body"],
            kinds: vec![ValueKind::Text],
            dialect: Dialect::Postgres,
        };
        let taken = std::cell::Cell::new(0);
        let rows = (0..100).map(|_| {
            taken.set(taken.get() + 1);
            vec![Value::Text("w".repeat(1 << 20).into())]
        });
        let options = Options::default();
        let copied = clipboard(
            Format::Markdown,
            &options,
            columns,
            Target::Named("result"),
            100,
            rows,
        );
        assert_eq!(copied, Copied::TooLarge);
        assert!(taken.get() <= 21, "{} rows read", taken.get());
    }

    #[test]
    fn the_clipboard_takes_no_byte_order_mark() {
        let columns = Columns {
            names: vec!["id"],
            kinds: vec![ValueKind::Numeric],
            dialect: Dialect::Postgres,
        };
        let options = with(|o| o.csv.bom = true);
        let rows = [vec![Value::Int(1)]].into_iter();
        let copied = clipboard(
            Format::Csv,
            &options,
            columns,
            Target::Named("result"),
            1,
            rows,
        );
        assert_eq!(
            copied,
            Copied::Text {
                text: "id\n1".into(),
                statements: 0
            }
        );
    }

    #[test]
    fn base64_pads_as_the_standard_says() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foob"), "Zm9vYg==");
    }

    #[test]
    fn the_clipboard_takes_no_last_line_ending_and_keeps_its_limits() {
        let columns = || Columns {
            names: vec!["id"],
            kinds: vec![ValueKind::Numeric],
            dialect: Dialect::Postgres,
        };
        let options = with(|o| o.tsv.header = false);
        let rows = (1..=2).map(|id| vec![Value::Int(id)]);
        let copied = clipboard(
            Format::Tsv,
            &options,
            columns(),
            Target::Named("result"),
            2,
            rows,
        );
        assert_eq!(
            copied,
            Copied::Text {
                text: "1\n2".into(),
                statements: 0
            }
        );
        let many = CLIPBOARD_ROWS + 1;
        let rows = (0..many).map(|id| vec![Value::Int(id as i64)]);
        let copied = clipboard(
            Format::Tsv,
            &options,
            columns(),
            Target::Named("result"),
            many,
            rows,
        );
        assert_eq!(copied, Copied::TooManyRows(many));
        let big = Value::Text("w".repeat(CLIPBOARD_BYTES + 1).into());
        let copied = clipboard(
            Format::Tsv,
            &options,
            columns(),
            Target::Named("result"),
            1,
            [vec![big]].into_iter(),
        );
        assert_eq!(copied, Copied::TooLarge);
    }

    #[test]
    fn sql_insert_counts_its_statements() {
        let options = with(|o| o.sql.rows_per_insert = 2);
        let columns = Columns {
            names: vec!["id"],
            kinds: vec![ValueKind::Numeric],
            dialect: Dialect::Postgres,
        };
        let rows = (1..=3).map(|id| vec![Value::Int(id)]);
        let copied = clipboard(
            Format::SqlInsert,
            &options,
            columns,
            Target::Named("result"),
            3,
            rows,
        );
        assert_eq!(
            copied,
            Copied::Text {
                text: "INSERT INTO result (id) VALUES\n  (1),\n  (2);\nINSERT INTO result (id) VALUES\n  (3);".into(),
                statements: 2
            }
        );
    }

    #[test]
    fn column_names_are_written_for_a_select_list() {
        let names = ["rating", "Title", "order"];
        assert_eq!(
            column_names(&names, Dialect::Postgres),
            "rating, \"Title\", \"order\""
        );
        assert_eq!(
            column_names(&names, Dialect::MySql),
            "rating, Title, `order`"
        );
    }

    #[test]
    fn options_are_kept_by_format_and_read_back() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("export_options.json");
        assert_eq!(Options::load(&path), Options::default());
        let mut options = Options::default();
        options.csv.delimiter = ';';
        options.tsv.null_as = NullAs::Text;
        options.json.layout = Layout::OneLine;
        options.jsonl.binary = Binary::Base64;
        options.sql.rows_per_insert = 1;
        options.md.long_values = LongValues::Cut;
        crate::util::save_json(&path, &options.to_file()).unwrap();
        assert_eq!(Options::load(&path), options);
    }

    #[test]
    fn what_this_version_cannot_read_is_written_back() {
        let file: BTreeMap<String, serde_json::Value> = serde_json::from_value(serde_json::json!({
            "csv": {"delimiter": ";", "quoting": "sometimes", "newer": 1},
            "xlsx": {"sheet": "one"},
        }))
        .unwrap();
        let mut options = Options::from_file(file);
        // What reads is taken; what does not, the default.
        assert_eq!(options.csv.delimiter, ';');
        assert_eq!(options.csv.quoting, Quoting::WhenNeeded);
        let written = options.to_file();
        assert_eq!(written["csv"]["quoting"], "sometimes");
        assert_eq!(written["csv"]["newer"], 1);
        assert_eq!(written["xlsx"]["sheet"], "one");
        // An entry that is no set of options at all is kept as it was.
        let file =
            serde_json::from_value(serde_json::json!({"md": "from a newer version"})).unwrap();
        let mut whole = Options::from_file(file);
        assert_eq!(whole.md, Markdown::default());
        assert_eq!(whole.to_file()["md"], "from a newer version");
        whole.chosen(Format::Markdown);
        assert_eq!(whole.to_file()["md"]["null_text"], "NULL");
        // Chosen again, the option this version knows is its own.
        options.csv.quoting = Quoting::Every;
        options.chosen(Format::Csv);
        let written = options.to_file();
        assert_eq!(written["csv"]["quoting"], "every");
        assert_eq!(written["csv"]["newer"], 1);
    }
}
