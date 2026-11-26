//! What a Files connection opens: the files of a folder or of a list, the
//! format of each, and the table (or, for a `.duckdb` file, the schema)
//! each becomes. Reading the folder is the only disk work here; DuckDB
//! reads the files themselves (`duck`).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{Error, Result};

/// Where a Files connection's files come from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    /// Every supported file directly in the folder, read again at each
    /// open: one added later shows then.
    Folder(PathBuf),
    /// These files, in this order.
    Files(Vec<PathBuf>),
}

impl Source {
    /// The folder, or the files' names, as one line.
    pub fn summary(&self) -> String {
        match self {
            Self::Folder(folder) => shown(folder),
            Self::Files(files) => files
                .iter()
                .map(|file| file_name(file))
                .collect::<Vec<_>>()
                .join(", "),
        }
    }

    /// What the source is called in a word: the folder's name, or the
    /// files' names.
    pub fn title(&self) -> String {
        match self {
            Self::Folder(folder) => folder
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| shown(folder)),
            Self::Files(_) => self.summary(),
        }
    }

    /// The folder the files are in: the folder itself, or the first
    /// file's.
    pub fn folder(&self) -> Option<&Path> {
        match self {
            Self::Folder(folder) => Some(folder),
            Self::Files(files) => files.first().and_then(|file| file.parent()),
        }
    }
}

/// How a Files connection reads its CSV and TSV files: the defaults the
/// dialog's Reading CSV section sets, and the overrides for one file.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Reading {
    pub header: Header,
    /// For `.csv` files: a `.tsv` is read with a tab whatever this says.
    pub delimiter: Delimiter,
    pub types: Types,
    pub overrides: Vec<Override>,
}

impl Reading {
    /// Whether it reads as a new connection does, with no override.
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }

    /// The overrides of the file at `path` (see [`Override::is_of`]).
    pub fn overrides_of<'a>(&'a self, path: &'a Path) -> impl Iterator<Item = &'a Change> + 'a {
        self.overrides
            .iter()
            .filter(move |each| each.is_of(path))
            .map(|each| &each.change)
    }

    /// Whether the file at `path` is left out of the connection.
    pub fn skips(&self, path: &Path) -> bool {
        self.overrides_of(path)
            .any(|change| matches!(change, Change::Skip))
    }
}

/// Whether a CSV's first row names its columns.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Header {
    /// As DuckDB's sniffer finds it.
    #[default]
    Detect,
    Yes,
    No,
}

impl Header {
    pub const ALL: [Self; 3] = [Self::Detect, Self::Yes, Self::No];
}

/// What a CSV's values are parted by.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Delimiter {
    /// As DuckDB's sniffer finds it.
    #[default]
    Detect,
    Comma,
    Semicolon,
    Tab,
    Pipe,
}

impl Delimiter {
    pub const ALL: [Self; 5] = [
        Self::Detect,
        Self::Comma,
        Self::Semicolon,
        Self::Tab,
        Self::Pipe,
    ];

    /// The character, where one is chosen.
    pub fn char(self) -> Option<char> {
        match self {
            Self::Detect => None,
            Self::Comma => Some(','),
            Self::Semicolon => Some(';'),
            Self::Tab => Some('\t'),
            Self::Pipe => Some('|'),
        }
    }

    /// The delimiter whose character is `char`.
    pub fn of(char: char) -> Option<Self> {
        Self::ALL.into_iter().find(|each| each.char() == Some(char))
    }
}

/// How a CSV's column types are found.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Types {
    /// From the first rows read ([`SAMPLE_ROWS`]).
    #[default]
    Sample,
    /// From every row: slower, and nothing past the sample surprises it.
    All,
    /// None: every column is text.
    Text,
}

impl Types {
    pub const ALL: [Self; 3] = [Self::Sample, Self::All, Self::Text];
}

/// A file a Test could not read, or its check in full found a row of that
/// it cannot, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure {
    /// The file's name with its extension: `returns.csv`.
    pub file: String,
    pub path: PathBuf,
    pub format: Format,
    pub why: Why,
}

impl Failure {
    /// Whether the fixes that change a column (read it as text, read a
    /// value as empty) can mend it: a CSV's or TSV's value its column's
    /// type cannot take. Any failure can be mended by skipping the file.
    pub fn takes_column_fixes(&self) -> bool {
        matches!(self.why, Why::Value { .. }) && matches!(self.format, Format::Csv | Format::Tsv)
    }
}

/// Why a file could not be read, as DuckDB said it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Why {
    /// A CSV's value that its column's type cannot take: the data row it
    /// is on (1 for the first after a header row), the column, the value,
    /// and the type DuckDB found for the column (`DATE`).
    Value {
        row: u64,
        column: String,
        value: String,
        kind: String,
    },
    /// A CSV's row with more or fewer values than the file's others.
    Columns { row: u64, expected: u64, found: u64 },
    /// An NDJSON file's line that is not JSON (`malformed`), or whose
    /// value is not of the type the lines before it gave its key.
    Json { line: u64, malformed: bool },
    /// Anything else: DuckDB's first line.
    Other(String),
}

/// How many rows of a CSV DuckDB samples for its columns' types, and how
/// many rows of each file a Test reads.
pub const SAMPLE_ROWS: u32 = 20_000;

/// A change to how one file is read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Override {
    /// The file's name with its extension, `returns.csv`, or its path,
    /// `~/Data/returns.csv`, which tells apart two files of one name in a
    /// list of files.
    pub file: String,
    pub change: Change,
}

impl Override {
    /// Whether it changes the file at `path`: a name is of every file of
    /// that name, a path (one with a separator, `~` for the home folder)
    /// of that file alone. `./returns.csv` is the file a list names
    /// `returns.csv`, apart from others of its name.
    pub fn is_of(&self, path: &Path) -> bool {
        let separated = self.file.contains('/') || self.file.contains(std::path::MAIN_SEPARATOR);
        match separated {
            true => undotted(&expanded(Path::new(&self.file))) == undotted(path),
            false => self.file == file_name(path),
        }
    }
}

/// `path` without the `.` it may start with.
fn undotted(path: &Path) -> &Path {
    path.strip_prefix(".").unwrap_or(path)
}

/// What an override changes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Change {
    /// The column is read as text.
    Text { column: String },
    /// These values of the column are read as empty (NULL); the column
    /// keeps the type found for it with them taken as empty. Several of
    /// one column are one; with no values it changes nothing.
    Empty { column: String, values: Vec<String> },
    /// The file is left out of the connection.
    Skip,
}

impl Change {
    /// The column it is about, which matches the file's in any case;
    /// `None` for a skip.
    pub fn column(&self) -> Option<&str> {
        match self {
            Self::Text { column } | Self::Empty { column, .. } => Some(column),
            Self::Skip => None,
        }
    }
}

/// How a file is read, from its extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Format {
    Csv,
    Tsv,
    Parquet,
    /// One JSON array of objects.
    Json,
    /// One JSON object per line (`.ndjson`, `.jsonl`).
    Ndjson,
    /// The first sheet of a workbook.
    Excel,
    /// A DuckDB database: a schema of its own tables.
    DuckDb,
}

impl Format {
    /// Every format, in the order the found line names them.
    pub const ALL: [Self; 7] = [
        Self::Csv,
        Self::Tsv,
        Self::Parquet,
        Self::Json,
        Self::Ndjson,
        Self::Excel,
        Self::DuckDb,
    ];

    /// The format a file of this name is read as, by its extension in any
    /// case; `None` for a file the connection leaves out.
    pub fn of(path: &Path) -> Option<Self> {
        let extension = path.extension()?.to_str()?.to_ascii_lowercase();
        match extension.as_str() {
            "csv" => Some(Self::Csv),
            "tsv" => Some(Self::Tsv),
            "parquet" => Some(Self::Parquet),
            "json" => Some(Self::Json),
            "ndjson" | "jsonl" => Some(Self::Ndjson),
            "xlsx" => Some(Self::Excel),
            "duckdb" => Some(Self::DuckDb),
            _ => None,
        }
    }

    /// As the dialog and the Structure view name it.
    pub fn label(self) -> &'static str {
        match self {
            Self::Csv => "CSV",
            Self::Tsv => "TSV",
            Self::Parquet => "Parquet",
            Self::Json => "JSON",
            Self::Ndjson => "NDJSON",
            Self::Excel => "Excel",
            Self::DuckDb => "DuckDB",
        }
    }

    /// Whether the file becomes a schema of tables rather than a table.
    pub fn is_database(self) -> bool {
        match self {
            Self::DuckDb => true,
            Self::Csv | Self::Tsv | Self::Parquet | Self::Json | Self::Ndjson | Self::Excel => {
                false
            }
        }
    }
}

/// One file of a connection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    pub path: PathBuf,
    pub format: Format,
    /// Its size in bytes, as the folder said it.
    pub size: u64,
    /// The table it becomes, or for a `.duckdb` file the schema.
    pub name: String,
}

/// What a source holds as it is on disk now.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Scan {
    /// The files the connection opens, each with its name.
    pub files: Vec<Found>,
    /// Files of a list that are not there (or are no longer files).
    pub missing: Vec<PathBuf>,
    /// Files that cannot be looked at, with why, when a Test scans past
    /// them: else the first is the scan's error.
    pub unreadable: Vec<(PathBuf, String)>,
}

impl Scan {
    pub fn total_size(&self) -> u64 {
        self.files.iter().map(|file| file.size).sum()
    }

    /// The formats found, each once, in [`Format::ALL`]'s order.
    pub fn formats(&self) -> Vec<Format> {
        Format::ALL
            .into_iter()
            .filter(|format| self.files.iter().any(|file| file.format == *format))
            .collect()
    }

    /// The names the files become, in the files' order.
    pub fn names(&self) -> Vec<&str> {
        self.files.iter().map(|file| file.name.as_str()).collect()
    }
}

/// The files `source` holds: a folder's supported files directly in it,
/// by name in any case (hidden files, subfolders and other types left out), or a
/// list's in its order. A folder that cannot be read is an error naming
/// it; a file of a list that is gone is [`Scan::missing`].
pub fn scan(source: &Source) -> Result<Scan> {
    scan_past(source, |_| false, false)
}

/// [`scan`], with the files `skipped` says left out before anything is
/// asked of them: one that is gone, or that cannot be looked at, is no
/// matter. A Test goes `past` a file it cannot look at: it is one of
/// [`Scan::unreadable`].
pub fn scan_past(source: &Source, skipped: impl Fn(&Path) -> bool, past: bool) -> Result<Scan> {
    let mut entries: Vec<(PathBuf, Format, u64)> = Vec::new();
    let mut missing = Vec::new();
    let mut unreadable = Vec::new();
    let mut unlooked = |path: &Path, error: &std::io::Error| match past {
        true => {
            unreadable.push((path.to_owned(), file_why(error)));
            Ok(())
        }
        false => Err(file_error(path, error)),
    };
    match source {
        Source::Folder(folder) => {
            let read = std::fs::read_dir(folder).map_err(|error| folder_error(folder, &error))?;
            for entry in read {
                let entry = entry.map_err(|error| folder_error(folder, &error))?;
                let path = entry.path();
                let hidden = entry
                    .file_name()
                    .to_str()
                    .is_none_or(|n| n.starts_with('.'));
                let Some(format) = Format::of(&path).filter(|_| !hidden) else {
                    continue;
                };
                if skipped(&path) {
                    continue;
                }
                // A link to a file counts as the file; a folder never, nor
                // a broken link. A file that cannot be looked at, or read,
                // is said.
                match std::fs::metadata(&path) {
                    Ok(meta) if meta.is_file() => match readable(&path) {
                        Ok(()) => entries.push((path, format, meta.len())),
                        Err(error) => unlooked(&path, &error)?,
                    },
                    Ok(_) => {}
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(error) => unlooked(&path, &error)?,
                }
            }
            // By name, in any case, as a person reads the folder.
            entries.sort_by_cached_key(|(path, _, _)| by_name(path));
        }
        Source::Files(files) => {
            for path in files.iter().filter(|path| !skipped(path)) {
                let Some(format) = Format::of(path) else {
                    return Err(Error::InvalidSpec(format!(
                        "{} is not a CSV, TSV, Parquet, JSON, NDJSON, Excel or .duckdb file",
                        file_name(path)
                    )));
                };
                match std::fs::metadata(path) {
                    Ok(meta) if meta.is_file() => match readable(path) {
                        Ok(()) => entries.push((path.clone(), format, meta.len())),
                        Err(error) => unlooked(path, &error)?,
                    },
                    Ok(_) => missing.push(path.clone()),
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                        missing.push(path.clone());
                    }
                    Err(error) => unlooked(path, &error)?,
                }
            }
        }
    }
    // A list's in its order, a folder's as its files are.
    if let Source::Folder(_) = source {
        unreadable.sort_by_cached_key(|(path, _)| by_name(path));
    }
    let names = names(
        entries
            .iter()
            .map(|(path, format, _)| (path.as_path(), *format)),
    );
    let files = entries
        .into_iter()
        .zip(names)
        .map(|((path, format, size), name)| Found {
            path,
            format,
            size,
            name,
        })
        .collect();
    Ok(Scan {
        files,
        missing,
        unreadable,
    })
}

/// Where a file comes in a folder: by name, in any case, as a person reads
/// it.
pub fn by_name(path: &Path) -> (Option<String>, PathBuf) {
    let name = path.file_name().map(|n| n.to_string_lossy().to_lowercase());
    (name, path.to_owned())
}

/// Whether `path` may be read, which looking at it does not say: a file's
/// own permissions refuse its contents and not its size, and so do macOS's
/// privacy settings for a file in a folder they guard.
fn readable(path: &Path) -> std::io::Result<()> {
    std::fs::File::open(path).map(drop)
}

/// Why a folder cannot be read, in the dialog's words.
fn folder_error(folder: &Path, error: &std::io::Error) -> Error {
    let why = match error.kind() {
        std::io::ErrorKind::NotFound => "no such folder".to_owned(),
        std::io::ErrorKind::NotADirectory => "not a folder".to_owned(),
        std::io::ErrorKind::PermissionDenied => not_allowed(error, Refused::Folder),
        _ => error.to_string(),
    };
    Error::Connect(format!("Can't open {}: {why}", shown(folder)))
}

/// Why a file of the source cannot be looked at, other than its being
/// gone.
fn file_error(file: &Path, error: &std::io::Error) -> Error {
    Error::Connect(format!("Can't open {}: {}", shown(file), file_why(error)))
}

/// Why a file cannot be looked at, in the dialog's words.
fn file_why(error: &std::io::Error) -> String {
    match error.kind() {
        std::io::ErrorKind::PermissionDenied => not_allowed(error, Refused::File),
        _ => error.to_string(),
    }
}

/// What macOS's privacy settings (Files and Folders, Full Disk Access)
/// did not let Tabletist open: a Files connection's folder, or one of its
/// files.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refused {
    Folder,
    File,
}

/// What a refusal of macOS's privacy settings says after the path.
const FOLDER_REFUSED: &str = "macOS hasn't allowed Tabletist to open this folder";
const FILE_REFUSED: &str = "macOS hasn't allowed Tabletist to open this file";

/// Why `what` may not be read, its error a refusal: macOS's privacy
/// settings where they refused it, its own permissions anywhere else.
fn not_allowed(error: &std::io::Error, what: Refused) -> String {
    not_allowed_on(error, what, cfg!(target_os = "macos"))
}

/// [`not_allowed`] on macOS or elsewhere. macOS's privacy settings refuse
/// with `EPERM`, where a file's own permissions refuse with `EACCES`.
fn not_allowed_on(error: &std::io::Error, what: Refused, macos: bool) -> String {
    const EPERM: i32 = 1;
    match (macos && error.raw_os_error() == Some(EPERM), what) {
        (true, Refused::Folder) => FOLDER_REFUSED.to_owned(),
        (true, Refused::File) => FILE_REFUSED.to_owned(),
        (false, _) => "not allowed to read it".to_owned(),
    }
}

/// Whether `why`, a file's [`Why::Other`], is macOS's privacy settings
/// refusing Tabletist that file: a Test goes past such a file, and says
/// so.
pub fn refused_file_by_macos(why: &str) -> bool {
    why == FILE_REFUSED
}

/// The folder or file a message of [`scan`]'s says macOS's privacy
/// settings did not let Tabletist open, as the message shows it, and which
/// it is. `None` for any other message.
pub fn refused_by_macos(message: &str) -> Option<(&str, Refused)> {
    let rest = message.strip_prefix("Can't open ")?;
    [
        (FOLDER_REFUSED, Refused::Folder),
        (FILE_REFUSED, Refused::File),
    ]
    .into_iter()
    .find_map(|(why, what)| Some((rest.strip_suffix(why)?.strip_suffix(": ")?, what)))
}

/// The name a file's stem becomes: lowercased, with anything outside
/// `[a-z0-9_]` turned into `_`, and `_` before a leading digit.
pub fn table_name(stem: &str) -> String {
    let mut name: String = stem
        .chars()
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'a'..='z' | '0'..='9' | '_' => c,
            _ => '_',
        })
        .collect();
    if name.is_empty() || name.starts_with(|c: char| c.is_ascii_digit()) {
        name.insert(0, '_');
    }
    name
}

/// A saved connection a Files connection attaches, as it is kept: the
/// saved connection's id, and the name SQL calls it by
/// (`bookshop_dev.orders`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attach {
    pub connection: String,
    pub name: String,
}

/// A saved connection to attach as a session opens: the name SQL calls
/// it by, how it is reached, and its secrets, read from the keyring just
/// before. Prints without them.
#[derive(Clone)]
pub struct Attaching {
    pub name: String,
    pub spec: crate::ConnectSpec,
    pub secrets: crate::Secrets,
}

impl std::fmt::Debug for Attaching {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Attaching")
            .field("name", &self.name)
            .field("driver", &self.spec.driver)
            .finish_non_exhaustive()
    }
}

/// The name SQL calls an attached connection by until it is changed: its
/// database's name as it is (a SQLite file's stem), made a plain name as
/// a file's table name is (see [`table_name`]). A PostgreSQL connection
/// that names no database reads its user's.
pub fn attach_name(spec: &crate::ConnectSpec) -> String {
    use crate::Driver;
    let named = match spec.driver {
        Driver::Sqlite => spec
            .sqlite_path
            .as_deref()
            .and_then(Path::file_stem)
            .and_then(|stem| stem.to_str())
            .unwrap_or_default()
            .to_owned(),
        Driver::Postgres if spec.database.is_empty() => spec.user.clone(),
        Driver::Postgres | Driver::MySql | Driver::BigQuery | Driver::Files => {
            spec.database.clone()
        }
    };
    table_name(&named)
}

/// The schema of an attached database whose tables the sidebar lists,
/// and that `name.table` reads: PostgreSQL's `public`, MySQL's database,
/// SQLite's `main`.
pub fn attached_schema(spec: &crate::ConnectSpec) -> String {
    use crate::Driver;
    match spec.driver {
        Driver::Postgres => "public".into(),
        Driver::MySql => spec.database.clone(),
        Driver::Sqlite | Driver::BigQuery | Driver::Files => "main".into(),
    }
}

/// Whether `name` is one an attached connection cannot take: one of
/// DuckDB's own databases, or a schema every database has.
pub fn reserved_name(name: &str) -> bool {
    RESERVED.contains(&name)
}

/// A reader DuckDB installs the first time a connection needs it, from
/// DuckDB's servers, and keeps for every open after: the Excel files',
/// and those of the databases a Files connection attaches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reader {
    Excel,
    Postgres,
    MySql,
    Sqlite,
}

impl Reader {
    /// The reader of a database `driver` names, where DuckDB has one.
    pub fn of(driver: crate::Driver) -> Option<Self> {
        use crate::Driver;
        match driver {
            Driver::Postgres => Some(Self::Postgres),
            Driver::MySql => Some(Self::MySql),
            Driver::Sqlite => Some(Self::Sqlite),
            Driver::BigQuery | Driver::Files => None,
        }
    }

    /// What `INSTALL` and `LOAD` are told.
    pub(crate) fn extension(self) -> &'static str {
        match self {
            Self::Excel => "excel",
            Self::Postgres => "postgres",
            Self::MySql => "mysql",
            Self::Sqlite => "sqlite",
        }
    }

    /// The name `duckdb_extensions()` lists it by, installed or not.
    pub(crate) fn listed(self) -> &'static str {
        match self {
            Self::Excel => "excel",
            Self::Postgres => "postgres_scanner",
            Self::MySql => "mysql_scanner",
            Self::Sqlite => "sqlite_scanner",
        }
    }

    /// What it reads, for a message.
    pub fn label(self) -> &'static str {
        match self {
            Self::Excel => "Excel",
            Self::Postgres => "PostgreSQL",
            Self::MySql => "MySQL",
            Self::Sqlite => "SQLite",
        }
    }
}

/// Told, on the session's thread, each time DuckDB is about to download a
/// reader: what a Test's status says while it waits.
pub type Downloading = std::sync::Arc<dyn Fn(Reader) + Send + Sync>;

/// The names DuckDB keeps for its own databases, which a `.duckdb` file
/// cannot be attached under, and the schemas every catalog has: a file
/// attached as `information_schema` makes `information_schema.columns`
/// ambiguous, and Structure fails for every table.
const RESERVED: [&str; 6] = [
    "information_schema",
    "main",
    "memory",
    "pg_catalog",
    "system",
    "temp",
];

/// `files` named again as tables among themselves: what a scan names once
/// some of its files are left out, as a connection names those it opens.
pub fn renamed(files: Vec<Found>) -> Vec<Found> {
    let names = names(files.iter().map(|file| (file.path.as_path(), file.format)));
    files
        .into_iter()
        .zip(names)
        .map(|(file, name)| Found { name, ..file })
        .collect()
}

/// Each file's name, in order: its stem's [`table_name`], and where two
/// files would share one, each of them that name with `_` and its
/// extension (`orders_csv`, `orders_parquet`). A `.duckdb` file named as
/// one of DuckDB's own databases takes `_duckdb`. Should a name still be
/// taken (the same file name in two folders, or a file named as another's
/// suffixed name), a number follows, until it is one no other file has.
fn names<'a>(files: impl Iterator<Item = (&'a Path, Format)>) -> Vec<String> {
    let files: Vec<(String, String, Format)> = files
        .map(|(path, format)| {
            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            (table_name(stem), extension.to_ascii_lowercase(), format)
        })
        .collect();
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for (name, _, _) in &files {
        *counts.entry(name).or_default() += 1;
    }
    // The names given so far.
    let mut taken: std::collections::HashSet<String> = std::collections::HashSet::new();
    let wanted: Vec<String> = files
        .iter()
        .map(|(name, extension, format)| {
            let name = if counts[name.as_str()] > 1 {
                format!("{name}_{}", table_name(extension).trim_start_matches('_'))
            } else {
                name.clone()
            };
            match format.is_database() && RESERVED.contains(&name.as_str()) {
                true => format!("{name}_duckdb"),
                false => name,
            }
        })
        .collect();
    let mut wanted_count: HashMap<&str, usize> = HashMap::new();
    for name in &wanted {
        *wanted_count.entry(name).or_default() += 1;
    }
    let mut given: HashMap<&str, usize> = HashMap::new();
    wanted
        .iter()
        .map(|name| {
            let seen = given.entry(name).or_default();
            *seen += 1;
            let mut candidate = match *seen {
                1 => name.clone(),
                n => format!("{name}_{n}"),
            };
            let mut n = (*seen).max(1);
            // Neither one given already nor one another file wants as it is.
            while taken.contains(&candidate)
                || (candidate != *name && wanted_count.contains_key(candidate.as_str()))
            {
                n += 1;
                candidate = format!("{name}_{n}");
            }
            taken.insert(candidate.clone());
            candidate
        })
        .collect()
}

/// A file's name with its extension: for a list of them, and as an
/// override names it.
pub fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

/// The home folder a path under it is shown from with `~`, as the app
/// found it at its start ([`set_home`]); asked of the system once where
/// it was not.
static HOME: std::sync::OnceLock<Option<PathBuf>> = std::sync::OnceLock::new();

/// Sets the home folder `~` stands for. Only the first call counts.
pub fn set_home(home: Option<PathBuf>) {
    let _ = HOME.set(home);
}

fn home() -> Option<&'static Path> {
    HOME.get_or_init(std::env::home_dir).as_deref()
}

/// A path as the app shows it: `~` for the home folder.
pub fn shown(path: &Path) -> String {
    if let Some(home) = home()
        && let Ok(rest) = path.strip_prefix(home)
    {
        if rest.as_os_str().is_empty() {
            return "~".to_owned();
        }
        return format!("~{}{}", std::path::MAIN_SEPARATOR, rest.display());
    }
    path.display().to_string()
}

/// A path as it was written: one that starts with `~` is under the home
/// folder.
pub fn expanded(path: &Path) -> PathBuf {
    let Some(text) = path.to_str() else {
        return path.to_owned();
    };
    let rest = match text.strip_prefix('~') {
        Some("") => "",
        Some(rest) => match rest.strip_prefix(['/', std::path::MAIN_SEPARATOR]) {
            Some(rest) => rest,
            None => return path.to_owned(),
        },
        None => return path.to_owned(),
    };
    match home() {
        Some(home) if rest.is_empty() => home.to_owned(),
        Some(home) => home.join(rest),
        None => path.to_owned(),
    }
}

/// `bytes` as the found line says them: `512 B`, `12.4 MB`, `2.5 GB`.
pub fn size_text(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1000.0 && unit + 1 < UNITS.len() {
        value /= 1000.0;
        unit += 1;
    }
    // What would be written as 1000 of a unit is 1.0 of the next.
    if unit > 0 && value >= 999.5 && unit + 1 < UNITS.len() {
        value /= 1000.0;
        unit += 1;
    }
    match unit {
        0 => format!("{bytes} B"),
        _ if value >= 100.0 => format!("{value:.0} {}", UNITS[unit]),
        _ => format!("{value:.1} {}", UNITS[unit]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn touch(dir: &Path, name: &str, bytes: usize) {
        std::fs::write(dir.join(name), vec![b'x'; bytes]).unwrap();
    }

    #[test]
    fn a_folder_holds_its_supported_files_by_name() {
        let dir = tempfile::tempdir().unwrap();
        for (name, bytes) in [
            ("orders.csv", 10),
            ("Customers.PARQUET", 20),
            ("events.ndjson", 5),
            ("log.jsonl", 1),
            ("authors.tsv", 2),
            ("books.json", 3),
            ("stock.xlsx", 4),
            ("bookshop.duckdb", 6),
            (".hidden.csv", 1),
            ("notes.txt", 1),
            ("README", 1),
        ] {
            touch(dir.path(), name, bytes);
        }
        std::fs::create_dir(dir.path().join("archive.csv")).unwrap();
        std::fs::create_dir(dir.path().join("older")).unwrap();
        touch(&dir.path().join("older"), "orders.csv", 1);
        let scan = scan(&Source::Folder(dir.path().to_owned())).unwrap();
        assert_eq!(
            scan.names(),
            [
                "authors",
                "books",
                "bookshop",
                "customers",
                "events",
                "log",
                "orders",
                "stock"
            ]
        );
        assert_eq!(scan.total_size(), 51);
        assert_eq!(
            scan.formats(),
            [
                Format::Csv,
                Format::Tsv,
                Format::Parquet,
                Format::Json,
                Format::Ndjson,
                Format::Excel,
                Format::DuckDb
            ]
        );
        assert!(scan.missing.is_empty());
    }

    #[test]
    fn names_that_clash_take_their_extension() {
        let dir = tempfile::tempdir().unwrap();
        touch(dir.path(), "orders.csv", 1);
        touch(dir.path(), "orders.parquet", 1);
        touch(dir.path(), "Order Lines.csv", 1);
        touch(dir.path(), "2024-returns.csv", 1);
        let scan = scan(&Source::Folder(dir.path().to_owned())).unwrap();
        assert_eq!(
            scan.names(),
            [
                "_2024_returns",
                "order_lines",
                "orders_csv",
                "orders_parquet"
            ]
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_file_that_cannot_be_looked_at_is_said_not_missing() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let shut = dir.path().join("shut");
        std::fs::create_dir(&shut).unwrap();
        touch(&shut, "orders.csv", 1);
        // Read without search: the names are listed, the files not.
        std::fs::set_permissions(&shut, std::fs::Permissions::from_mode(0o600)).unwrap();
        let looked = std::fs::metadata(shut.join("orders.csv"));
        let folder = scan(&Source::Folder(shut.clone()));
        let list = scan(&Source::Files(vec![shut.join("orders.csv")]));
        // A Test goes past it.
        let past = [
            scan_past(&Source::Folder(shut.clone()), |_| false, true),
            scan_past(
                &Source::Files(vec![shut.join("orders.csv")]),
                |_| false,
                true,
            ),
        ];
        std::fs::set_permissions(&shut, std::fs::Permissions::from_mode(0o700)).unwrap();
        if looked.is_ok() {
            // Run as root, which may look at anything.
            return;
        }
        for scanned in [folder, list] {
            match scanned {
                Err(Error::Connect(why)) => {
                    assert!(why.ends_with("orders.csv: not allowed to read it"), "{why}");
                }
                other => panic!("{other:?}"),
            }
        }
        for scanned in past {
            let scanned = scanned.unwrap();
            assert!(scanned.files.is_empty() && scanned.missing.is_empty());
            assert_eq!(
                scanned.unreadable,
                [(shut.join("orders.csv"), "not allowed to read it".to_owned())]
            );
        }
    }

    #[test]
    fn a_number_never_gives_a_name_another_file_has() {
        let dir = tempfile::tempdir().unwrap();
        // Two names that differ by more than case: macOS's file system
        // keeps `orders.csv` and `orders.CSV` as one file.
        touch(dir.path(), "order lines.csv", 1);
        touch(dir.path(), "order-lines.csv", 1);
        touch(dir.path(), "order_lines_csv_2.csv", 1);
        let scan = scan(&Source::Folder(dir.path().to_owned())).unwrap();
        let mut names = scan.names();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), 3, "{:?}", scan.names());
        assert!(scan.names().contains(&"order_lines_csv_2"));
    }

    #[test]
    fn a_duckdb_file_named_as_duckdbs_own_database_takes_a_suffix() {
        let dir = tempfile::tempdir().unwrap();
        for name in [
            "main.duckdb",
            "memory.duckdb",
            "system.duckdb",
            "temp.duckdb",
            "information_schema.duckdb",
            "pg_catalog.duckdb",
            "main.csv",
        ] {
            touch(dir.path(), name, 1);
        }
        let scan = scan(&Source::Folder(dir.path().to_owned())).unwrap();
        assert_eq!(
            scan.names(),
            [
                "information_schema_duckdb",
                "main_csv",
                "main_duckdb",
                "memory_duckdb",
                "pg_catalog_duckdb",
                "system_duckdb",
                "temp_duckdb"
            ]
        );
    }

    #[test]
    fn a_list_keeps_its_order_and_says_what_is_gone() {
        let dir = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        touch(dir.path(), "orders.csv", 3);
        touch(other.path(), "orders.csv", 4);
        touch(dir.path(), "events.ndjson", 2);
        let list = vec![
            dir.path().join("events.ndjson"),
            dir.path().join("orders.csv"),
            other.path().join("orders.csv"),
            dir.path().join("returns.csv"),
        ];
        let scan = scan(&Source::Files(list)).unwrap();
        assert_eq!(scan.names(), ["events", "orders_csv", "orders_csv_2"]);
        assert_eq!(scan.missing, [dir.path().join("returns.csv")]);
        assert_eq!(scan.total_size(), 9);
    }

    #[test]
    fn a_list_refuses_a_file_it_cannot_read() {
        let error = scan(&Source::Files(vec!["/data/notes.txt".into()])).unwrap_err();
        assert!(
            error.to_string().contains("notes.txt is not a CSV"),
            "{error}"
        );
    }

    #[test]
    fn a_folder_that_is_gone_says_so() {
        let dir = tempfile::tempdir().unwrap();
        let gone = dir.path().join("bookshop-exports");
        let error = scan(&Source::Folder(gone)).unwrap_err();
        let Error::Connect(message) = error else {
            panic!("{error:?}");
        };
        assert!(message.starts_with("Can't open "), "{message}");
        assert!(
            message.ends_with("bookshop-exports: no such folder"),
            "{message}"
        );
    }

    /// What a session attaches prints its name and engine, never its
    /// secrets.
    #[test]
    fn an_attaching_prints_without_its_secrets() {
        let attaching = Attaching {
            name: "bookshop_dev".into(),
            spec: crate::ConnectSpec::default(),
            secrets: crate::Secrets {
                password: Some("hunter2".into()),
                ssh_password: Some("hunter3".into()),
                ssh_passphrase: Some("hunter4".into()),
            },
        };
        let printed = format!("{attaching:?}");
        assert!(printed.contains("bookshop_dev"), "{printed}");
        assert!(!printed.contains("hunter"), "{printed}");
    }

    #[test]
    fn table_names_are_plain_lowercase_words() {
        assert_eq!(table_name("Orders"), "orders");
        assert_eq!(table_name("order-lines 2024"), "order_lines_2024");
        assert_eq!(table_name("café"), "caf_");
        assert_eq!(table_name("2024"), "_2024");
        assert_eq!(table_name(""), "_");
    }

    #[test]
    fn sizes_read_as_the_found_line_says_them() {
        assert_eq!(size_text(512), "512 B");
        assert_eq!(size_text(12_400_000), "12.4 MB");
        assert_eq!(size_text(2_480_000_000), "2.5 GB");
        assert_eq!(size_text(1_600_000_000), "1.6 GB");
        assert_eq!(size_text(250_000), "250 KB");
        assert_eq!(size_text(999_950), "1.0 MB");
        assert_eq!(size_text(999), "999 B");
    }

    #[test]
    fn home_paths_are_shown_and_read_with_a_tilde() {
        let Some(home) = std::env::home_dir() else {
            return;
        };
        let path = home.join("Data").join("bookshop-exports");
        let shown = shown(&path);
        assert!(shown.starts_with('~'), "{shown}");
        assert_eq!(expanded(Path::new(&shown)), path);
        assert_eq!(expanded(Path::new("~")), home);
        assert_eq!(expanded(Path::new("/data/x")), Path::new("/data/x"));
        assert_eq!(expanded(Path::new("~bob/x")), Path::new("~bob/x"));
    }

    #[test]
    fn a_source_says_itself_in_a_line() {
        let list = Source::Files(vec!["/d/orders.csv".into(), "/d/customers.parquet".into()]);
        assert_eq!(list.summary(), "orders.csv, customers.parquet");
        assert_eq!(list.folder(), Some(Path::new("/d")));
        assert_eq!(list.title(), "orders.csv, customers.parquet");
        let folder = Source::Folder("/data/bookshop-exports".into());
        assert_eq!(folder.title(), "bookshop-exports");
    }

    /// On macOS a refusal of its privacy settings (`EPERM`) is said as
    /// macOS's, of the folder or of a file; a file's own permissions
    /// (`EACCES`) are not, and elsewhere neither is.
    #[test]
    fn a_refusal_of_macos_privacy_settings_is_said_as_its() {
        let eperm = std::io::Error::from_raw_os_error(1);
        let eacces = std::io::Error::from_raw_os_error(13);
        assert_eq!(
            not_allowed_on(&eperm, Refused::Folder, true),
            "macOS hasn't allowed Tabletist to open this folder"
        );
        assert_eq!(
            not_allowed_on(&eperm, Refused::File, true),
            "macOS hasn't allowed Tabletist to open this file"
        );
        assert_eq!(
            not_allowed_on(&eacces, Refused::Folder, true),
            "not allowed to read it"
        );
        assert_eq!(
            not_allowed_on(&eperm, Refused::Folder, false),
            "not allowed to read it"
        );
    }

    /// The path and what macOS refused, read back from the message; any
    /// other message is not one.
    #[test]
    fn a_refusal_of_macos_is_read_back_from_its_message() {
        assert_eq!(
            refused_by_macos(
                "Can't open ~/Data/bookshop-exports: macOS hasn't allowed Tabletist to open this folder"
            ),
            Some(("~/Data/bookshop-exports", Refused::Folder))
        );
        assert_eq!(
            refused_by_macos(
                "Can't open ~/Documents/orders.csv: macOS hasn't allowed Tabletist to open this file"
            ),
            Some(("~/Documents/orders.csv", Refused::File))
        );
        assert_eq!(
            refused_by_macos("Can't open ~/Data/bookshop-exports: no such folder"),
            None
        );
        assert_eq!(
            refused_by_macos("Can't open ~/Data/bookshop-exports: not allowed to read it"),
            None
        );
    }
}
