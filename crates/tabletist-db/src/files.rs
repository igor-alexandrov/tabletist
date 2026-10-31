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
    let mut entries: Vec<(PathBuf, Format, u64)> = Vec::new();
    let mut missing = Vec::new();
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
                // A link to a file counts as the file; a folder never, nor
                // a broken link. A file that cannot be looked at is said.
                match std::fs::metadata(&path) {
                    Ok(meta) if meta.is_file() => entries.push((path, format, meta.len())),
                    Ok(_) => {}
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(error) => return Err(file_error(&path, &error)),
                }
            }
            // By name, in any case, as a person reads the folder.
            entries.sort_by_cached_key(|(path, _, _)| {
                let name = path.file_name().map(|n| n.to_string_lossy().to_lowercase());
                (name, path.clone())
            });
        }
        Source::Files(files) => {
            for path in files {
                let Some(format) = Format::of(path) else {
                    return Err(Error::InvalidSpec(format!(
                        "{} is not a CSV, TSV, Parquet, JSON, NDJSON, Excel or .duckdb file",
                        file_name(path)
                    )));
                };
                match std::fs::metadata(path) {
                    Ok(meta) if meta.is_file() => entries.push((path.clone(), format, meta.len())),
                    Ok(_) => missing.push(path.clone()),
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                        missing.push(path.clone());
                    }
                    Err(error) => return Err(file_error(path, &error)),
                }
            }
        }
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
    Ok(Scan { files, missing })
}

/// Why a folder cannot be read, in the dialog's words.
fn folder_error(folder: &Path, error: &std::io::Error) -> Error {
    let why = match error.kind() {
        std::io::ErrorKind::NotFound => "no such folder".to_owned(),
        std::io::ErrorKind::NotADirectory => "not a folder".to_owned(),
        std::io::ErrorKind::PermissionDenied => "not allowed to read it".to_owned(),
        _ => error.to_string(),
    };
    Error::Connect(format!("Can't open {}: {why}", shown(folder)))
}

/// Why a file of the source cannot be looked at, other than its being
/// gone.
fn file_error(file: &Path, error: &std::io::Error) -> Error {
    let why = match error.kind() {
        std::io::ErrorKind::PermissionDenied => "not allowed to read it".to_owned(),
        _ => error.to_string(),
    };
    Error::Connect(format!("Can't open {}: {why}", shown(file)))
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

/// The names DuckDB keeps for its own databases, which a `.duckdb` file
/// cannot be attached under.
const RESERVED: [&str; 4] = ["main", "memory", "system", "temp"];

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

/// A file's name, for a list of them.
fn file_name(path: &Path) -> String {
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
            "main.csv",
        ] {
            touch(dir.path(), name, 1);
        }
        let scan = scan(&Source::Folder(dir.path().to_owned())).unwrap();
        assert_eq!(
            scan.names(),
            [
                "main_csv",
                "main_duckdb",
                "memory_duckdb",
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
}
