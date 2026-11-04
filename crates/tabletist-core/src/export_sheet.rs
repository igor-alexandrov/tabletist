//! The Export sheet (macOS) and float (Omarchy) of a table, and the export
//! it starts while that runs: what each holds and the words they say
//! (spec `2026-11-03-export-copy-as-3-table-design.md`). No drawing.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::backend::export::{ExportFailure, ExportJob};
use crate::export::{Format, Options};
use crate::i18n::{Locale, gettext, ngettext};
use crate::model::{ConnTabId, TabId};

/// Which rows an export writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowsChoice {
    /// The rows selected in the grid, as loaded.
    Selected,
    /// The page the grid shows, as loaded.
    Page,
    /// Every row the tab's filter keeps, in its sort.
    Matching,
    /// Every row of the table, in its key's order.
    Whole,
}

impl RowsChoice {
    pub const ALL: [Self; 4] = [Self::Selected, Self::Page, Self::Matching, Self::Whole];

    /// Whether the rows are the tab's own, so nothing runs (spec, D7).
    pub fn loaded(self) -> bool {
        matches!(self, Self::Selected | Self::Page)
    }
}

/// Where an export writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DestinationChoice {
    File,
    Clipboard,
}

/// A column in the sheet's list: its name, its type, whether it is
/// written. The list's order is the order written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnPick {
    pub name: String,
    pub type_name: String,
    pub included: bool,
}

/// A count the sheet shows: exact, or the planner's estimate (≈).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Count {
    pub rows: u64,
    pub exact: bool,
}

/// The sheet, while it is open.
#[derive(Debug, Clone, PartialEq)]
pub struct ExportSheet {
    /// The connection tab, and the table's tab in it.
    pub tab: ConnTabId,
    pub id: TabId,
    /// From `App::next_id`: keys the sheet's state in egui.
    pub asked: u64,
    pub format: Format,
    /// The options as chosen in the sheet: a copy of the remembered ones,
    /// remembered when the export starts (spec, Q11).
    pub options: Options,
    pub rows: RowsChoice,
    /// Columns: Choose (else All columns).
    pub choose: bool,
    /// Every column, in the order written.
    pub columns: Vec<ColumnPick>,
    /// The column list's cursor.
    pub column: usize,
    pub destination: DestinationChoice,
    /// The terminal look's "save to" line.
    pub path: String,
    /// The path line names a file that is there: the next Enter replaces
    /// it (spec, Q6).
    pub exists: bool,
    /// Review SQL shows in place of the choices.
    pub review: bool,
    /// The save panel is up: the sheet waits for its answer.
    pub asking: bool,
}

/// A change made in the sheet.
#[derive(Debug, Clone, PartialEq)]
pub enum ExportChange {
    Format(Format),
    Rows(RowsChoice),
    Choose(bool),
    /// Includes the column at the place, or leaves it out.
    Toggle(usize),
    /// Moves the column at `from` by `by` places.
    Move {
        from: usize,
        by: isize,
    },
    AllColumns,
    NoColumns,
    Cursor(usize),
    Destination(DestinationChoice),
    Options(Box<Options>),
    Path(String),
    Review(bool),
}

impl ExportSheet {
    /// The columns written, in their order.
    pub fn chosen(&self) -> Vec<&ColumnPick> {
        self.columns
            .iter()
            .filter(|column| !self.choose || column.included)
            .collect()
    }

    /// Makes `change`. Says what a screen reader is told, where a column
    /// moved ("title moved to position 3").
    pub fn change(&mut self, change: ExportChange, locale: Locale) -> Option<String> {
        match change {
            ExportChange::Format(format) => self.set_format(format),
            ExportChange::Rows(rows) => self.rows = rows,
            ExportChange::Choose(choose) => self.choose = choose,
            ExportChange::Toggle(at) => {
                if let Some(column) = self.columns.get_mut(at) {
                    column.included = !column.included;
                    self.column = at;
                }
            }
            ExportChange::Move { from, by } => {
                let to = from.saturating_add_signed(by);
                if from < self.columns.len() && to < self.columns.len() && to != from {
                    let column = self.columns.remove(from);
                    let name = column.name.clone();
                    self.columns.insert(to, column);
                    self.column = to;
                    return Some(
                        gettext(locale, "{column} moved to position {n}")
                            .replace("{column}", &name)
                            .replace("{n}", &(to + 1).to_string()),
                    );
                }
            }
            ExportChange::AllColumns => self.columns.iter_mut().for_each(|c| c.included = true),
            ExportChange::NoColumns => self.columns.iter_mut().for_each(|c| c.included = false),
            ExportChange::Cursor(at) => self.column = at.min(self.columns.len().saturating_sub(1)),
            ExportChange::Destination(destination) => self.destination = destination,
            ExportChange::Options(options) => self.options = *options,
            ExportChange::Path(path) => self.set_path(path),
            ExportChange::Review(review) => self.review = review,
        }
        None
    }

    /// The format, and the path line's extension with it.
    fn set_format(&mut self, format: Format) {
        self.format = format;
        self.path = with_extension(&self.path, format);
        self.exists = false;
    }

    /// The path line, and the format of a known extension typed in it.
    fn set_path(&mut self, path: String) {
        if let Some(format) = Format::of_path(Path::new(&path)) {
            self.format = format;
        }
        if path != self.path {
            self.exists = false;
        }
        self.path = path;
    }
}

/// `path` ending in `format`'s extension: a known one is replaced, any
/// other kept and the format's added.
pub fn with_extension(path: &str, format: Format) -> String {
    if path.is_empty() {
        return path.to_owned();
    }
    let known = Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
        .and_then(Format::of_extension)
        .is_some();
    let stem = match (known, path.rfind('.')) {
        (true, Some(dot)) => &path[..dot],
        _ => path,
    };
    format!("{stem}.{}", format.extension())
}

/// The rows a sheet starts on (prompt): Selected rows with a selection,
/// else All rows matching the filter.
pub fn first_rows(selected: usize) -> RowsChoice {
    if selected > 0 {
        RowsChoice::Selected
    } else {
        RowsChoice::Matching
    }
}

/// The folder files are suggested in (spec, Q19): `$XDG_DOWNLOAD_DIR`,
/// the system's Downloads folder, or the home folder.
pub fn downloads() -> PathBuf {
    let set = std::env::var_os("XDG_DOWNLOAD_DIR")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute());
    set.or_else(|| {
        directories::UserDirs::new().and_then(|dirs| dirs.download_dir().map(Path::to_path_buf))
    })
    .or_else(|| directories::BaseDirs::new().map(|dirs| dirs.home_dir().to_path_buf()))
    .unwrap_or_else(|| PathBuf::from("."))
}

/// `path` as the person reads it: the home folder as `~`.
pub fn shown_path(path: &Path, home: Option<&Path>) -> String {
    if let Some(home) = home
        && let Ok(rest) = path.strip_prefix(home)
    {
        if rest.as_os_str().is_empty() {
            return "~".to_owned();
        }
        return format!("~{}{}", std::path::MAIN_SEPARATOR, rest.display());
    }
    path.display().to_string()
}

/// The file a typed path names (spec, Q13): `~` is the home folder, and a
/// path that is not absolute is in `downloads`.
pub fn typed_path(typed: &str, downloads: &Path, home: Option<&Path>) -> PathBuf {
    let typed = typed.trim();
    let expanded = match (typed.strip_prefix('~'), home) {
        (Some(""), Some(home)) => home.to_path_buf(),
        (Some(rest), Some(home)) if rest.starts_with(['/', std::path::MAIN_SEPARATOR]) => {
            home.join(&rest[1..])
        }
        _ => PathBuf::from(typed),
    };
    if expanded.is_absolute() {
        expanded
    } else {
        downloads.join(expanded)
    }
}

/// How the running (or last) export stands, as the app keeps it.
#[derive(Debug, Clone, PartialEq)]
pub enum ExportStatus {
    /// The save panel is up.
    Asking,
    Running {
        rows: u64,
        bytes: u64,
    },
    Done {
        rows: u64,
        bytes: u64,
        took: Duration,
        at: Instant,
    },
    Cancelled {
        at: Instant,
    },
    Failed {
        failure: ExportFailure,
        /// The rows written when it failed, and their bytes.
        rows: u64,
        bytes: u64,
        at: Instant,
    },
}

/// The export the app runs (one at a time, spec D5), or the last one,
/// while its end is still told.
#[derive(Debug, Clone)]
pub struct Exporting {
    /// What runs, whole: Try again sends it again.
    pub job: ExportJob,
    pub tab: ConnTabId,
    /// The table's name: "reviews".
    pub table: String,
    /// The file's name ("reviews.csv"), and its path as the person reads
    /// it; empty for the clipboard.
    pub file: String,
    pub shown_path: String,
    /// "Bookshop · dev".
    pub connection: String,
    pub total: Option<Count>,
    /// The `SELECT` it runs, with its values written in: what Review SQL
    /// showed and the history keeps. `None` for loaded rows (D7).
    pub statement: Option<String>,
    /// The sheet it came from, to open again (Review SQL, Choose folder…).
    pub sheet: Box<ExportSheet>,
    pub status: ExportStatus,
    /// The macOS toast is hidden to the pill.
    pub hidden: bool,
    pub started: Instant,
    /// When a screen reader was last told how far it is.
    pub announced: Option<Instant>,
    /// It came from the terminal look's float: a file that is there
    /// opens the float again, which Enter then replaces.
    pub reopens: bool,
    /// Its history entry as it starts (its connection, its database):
    /// filled in and recorded when it ends, the tab gone or not. `None`
    /// where nothing runs (D7).
    pub history: Option<crate::history::Entry>,
}

/// How long the end of an export is told, unless it failed: then until
/// it is dismissed (prompt, open question 6).
pub const TOLD_FOR: Duration = Duration::from_secs(4);

impl Exporting {
    /// Whether it still runs (or waits for the save panel).
    pub fn running(&self) -> bool {
        matches!(
            self.status,
            ExportStatus::Asking | ExportStatus::Running { .. }
        )
    }

    /// Whether its end has been told long enough to go: a failure stays.
    pub fn gone(&self, now: Instant) -> bool {
        match self.status {
            ExportStatus::Done { at, .. } | ExportStatus::Cancelled { at } => {
                now.duration_since(at) >= TOLD_FOR
            }
            _ => false,
        }
    }

    /// The share written, from 0 to 1, where the total is known.
    pub fn share(&self) -> Option<f32> {
        let ExportStatus::Running { rows, .. } = self.status else {
            return None;
        };
        let total = self.total?.rows;
        (total > 0).then(|| (rows as f32 / total as f32).min(1.0))
    }

    /// "about 20 s left" (spec, Q17): once 2 s have gone and the total is
    /// known.
    pub fn time_left(&self, now: Instant, locale: Locale) -> Option<String> {
        let ExportStatus::Running { rows, .. } = self.status else {
            return None;
        };
        let total = self.total?.rows;
        let gone = now.duration_since(self.started).as_secs_f64();
        if gone < 2.0 || rows == 0 {
            return None;
        }
        let left = total.saturating_sub(rows) as f64 * gone / rows as f64;
        Some(left_words(left, locale))
    }
}

/// "about 20 s left", rounded to 5 s under a minute, to the minute above.
pub fn left_words(seconds: f64, locale: Locale) -> String {
    if seconds < 60.0 {
        let five = ((seconds / 5.0).round() * 5.0).max(5.0) as u64;
        gettext(locale, "about {n} s left").replace("{n}", &five.to_string())
    } else {
        let minutes = (seconds / 60.0).round() as u64;
        gettext(locale, "about {n} min left").replace("{n}", &minutes.to_string())
    }
}

/// A size as the toasts say it: "7.8 MB", "312 KB", "640 bytes".
pub fn size_words(bytes: u64, locale: Locale) -> String {
    const KB: f64 = 1_000.0;
    const MB: f64 = 1_000_000.0;
    const GB: f64 = 1_000_000_000.0;
    let bytes_f = bytes as f64;
    if bytes_f >= GB {
        format!("{:.1} GB", bytes_f / GB)
    } else if bytes_f >= MB {
        format!("{:.1} MB", bytes_f / MB)
    } else if bytes_f >= KB {
        format!("{:.0} KB", bytes_f / KB)
    } else {
        ngettext(
            locale,
            "{n} byte",
            "{n} bytes",
            u32::try_from(bytes).unwrap_or(u32::MAX),
        )
        .replace("{n}", &bytes.to_string())
    }
}

/// A time as the toasts say it: "18 s", "2 min 5 s".
pub fn took_words(took: Duration) -> String {
    let seconds = took.as_secs();
    match seconds {
        // "after 60 s", as the boards write a minute.
        0..120 => format!("{seconds} s"),
        _ if seconds.is_multiple_of(60) => format!("{} min", seconds / 60),
        _ => format!("{} min {} s", seconds / 60, seconds % 60),
    }
}

/// A count as the sheet and the toasts write it: "31,400", "≈ 31,400".
pub fn count_words(count: Count) -> String {
    let rows = crate::format::group_digits(count.rows);
    match count.exact {
        true => rows,
        false => format!("≈ {rows}"),
    }
}

/// The rows, of the total where it is known: "18,200 of ≈ 31,400 rows".
pub fn of_total(rows: u64, total: Option<Count>, locale: Locale) -> String {
    let done = crate::format::group_digits(rows);
    match total {
        Some(total) => gettext(locale, "{done} of {total} rows")
            .replace("{done}", &done)
            .replace("{total}", &count_words(total)),
        None => ngettext(
            locale,
            "{n} row",
            "{n} rows",
            u32::try_from(rows).unwrap_or(u32::MAX),
        )
        .replace("{n}", &done),
    }
}

/// The macOS look's sentence for a failure, and its action's name.
pub fn failure_words(
    exporting: &Exporting,
    failure: &ExportFailure,
    written: (u64, u64),
    home: Option<&Path>,
    locale: Locale,
) -> (String, Option<String>) {
    let (rows, bytes) = written;
    let total = exporting.total;
    let try_again = Some(gettext(locale, "Try again").into_owned());
    match failure {
        ExportFailure::PermissionDenied { folder } => (
            gettext(locale, "Can't write to {folder}. Permission denied.")
                .replace("{folder}", &shown_path(folder, home)),
            Some(gettext(locale, "Choose folder…").into_owned()),
        ),
        ExportFailure::DiskFull { written } => (
            gettext(
                locale,
                "The disk is full. {written} written. The partial file was removed.",
            )
            .replace(
                "{written}",
                &written_of((*written).max(bytes), exporting, locale),
            ),
            try_again,
        ),
        ExportFailure::ConnectionLost { rows: lost } => (
            gettext(
                locale,
                "Connection to {connection} was lost after {rows}. The partial file was removed.",
            )
            .replace("{connection}", &exporting.connection)
            .replace("{rows}", &of_total((*lost).max(rows), total, locale)),
            try_again,
        ),
        ExportFailure::Timeout { after } => (
            gettext(
                locale,
                "The server stopped the query after {time} (statement_timeout on {connection}).",
            )
            .replace("{time}", &took_words(*after))
            .replace("{connection}", &exporting.connection),
            Some(gettext(locale, "Review SQL").into_owned()),
        ),
        ExportFailure::Exists { path } => (
            gettext(locale, "{file} already exists. Nothing was replaced.")
                .replace("{file}", &shown_path(path, home)),
            Some(gettext(locale, "Choose folder…").into_owned()),
        ),
        ExportFailure::TooManyRows => (
            gettext(locale, "That's more than 10,000 rows. Export to a file?").into_owned(),
            None,
        ),
        ExportFailure::TooLarge => (
            gettext(locale, "That's over 20 MB. Export to a file?").into_owned(),
            None,
        ),
        ExportFailure::Other(why) => (
            gettext(locale, "Couldn't export {table}: {why}.")
                .replace("{table}", &exporting.table)
                .replace("{why}", why.trim_end_matches('.')),
            try_again,
        ),
    }
}

/// "6.1 of ≈ 7.8 MB": what was written, of the size the rows written so
/// far say the whole would be, where the total is known.
fn written_of(bytes: u64, exporting: &Exporting, locale: Locale) -> String {
    let ExportStatus::Failed { rows, .. } = exporting.status else {
        return size_words(bytes, locale);
    };
    match exporting.total {
        Some(total) if rows > 0 && total.rows > rows => {
            let whole = (bytes as f64 * total.rows as f64 / rows as f64) as u64;
            let shown = size_words(whole, locale);
            let unit = shown.rsplit(' ').next().unwrap_or_default().to_owned();
            let part = size_words(bytes, locale);
            let part = part.strip_suffix(&format!(" {unit}")).unwrap_or(&part);
            format!("{part} of ≈ {shown}")
        }
        _ => size_words(bytes, locale),
    }
}

/// The terminal look's line for a failure (spec, D13), lower case.
/// After a timeout `:export` shows the statement (plan, B13).
pub fn failure_line(
    exporting: &Exporting,
    failure: &ExportFailure,
    written: (u64, u64),
    home: Option<&Path>,
    locale: Locale,
) -> String {
    let (rows, bytes) = written;
    let total = exporting.total;
    let line = match failure {
        ExportFailure::PermissionDenied { .. } => gettext(
            locale,
            "✗ can't write {path}: permission denied · :w another path",
        )
        .replace("{path}", &exporting.shown_path),
        ExportFailure::DiskFull { written } => gettext(
            locale,
            "✗ disk full after {written} · partial file removed · :export to retry",
        )
        .replace(
            "{written}",
            &written_of((*written).max(bytes), exporting, locale),
        ),
        ExportFailure::ConnectionLost { rows: lost } => gettext(
            locale,
            "✗ connection lost after {rows} · partial file removed · :export to retry",
        )
        .replace("{rows}", &of_total((*lost).max(rows), total, locale)),
        ExportFailure::Timeout { after } => gettext(
            locale,
            "✗ server stopped the query after {time} (statement_timeout) · :export review sql",
        )
        .replace("{time}", &took_words(*after)),
        ExportFailure::Exists { path } => {
            gettext(locale, "! {path} exists · :w! to replace, or another name")
                .replace("{path}", &shown_path(path, home))
        }
        ExportFailure::TooManyRows => {
            gettext(locale, "that's more than 10,000 rows · export to a file").into_owned()
        }
        ExportFailure::TooLarge => {
            gettext(locale, "that's over 20 mb · export to a file").into_owned()
        }
        ExportFailure::Other(why) => gettext(
            locale,
            "✗ couldn't export {table}: {why} · :export to retry",
        )
        .replace("{table}", &exporting.table)
        .replace("{why}", why.trim_end_matches('.')),
    };
    line.to_lowercase()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn sheet() -> ExportSheet {
        let column = |name: &str| ColumnPick {
            name: name.into(),
            type_name: "text".into(),
            included: true,
        };
        ExportSheet {
            tab: ConnTabId(1),
            id: TabId(1),
            asked: 1,
            format: Format::Csv,
            options: Options::default(),
            rows: RowsChoice::Matching,
            choose: false,
            columns: vec![column("id"), column("rating"), column("title")],
            column: 0,
            destination: DestinationChoice::File,
            path: "~/Downloads/reviews.csv".into(),
            exists: false,
            review: false,
            asking: false,
        }
    }

    #[test]
    fn the_format_and_the_extension_follow_each_other() {
        let mut sheet = sheet();
        sheet.change(ExportChange::Format(Format::JsonLines), Locale::English);
        assert_eq!(sheet.path, "~/Downloads/reviews.jsonl");
        sheet.change(
            ExportChange::Path("~/Downloads/reviews.md".into()),
            Locale::English,
        );
        assert_eq!(sheet.format, Format::Markdown);
        // An ending no format has is kept, and the format's added.
        assert_eq!(with_extension("notes.txt", Format::Csv), "notes.txt.csv");
        assert_eq!(with_extension("dump.SQL", Format::Tsv), "dump.tsv");
        assert_eq!(Format::of_extension("JSONL"), Some(Format::JsonLines));
        assert_eq!(Format::of_extension("xlsx"), None);
    }

    /// Test 7 of the spec: chosen and reordered columns, in that order,
    /// and the move as a screen reader hears it.
    #[test]
    fn chosen_columns_are_written_in_their_order() {
        let mut sheet = sheet();
        assert_eq!(sheet.chosen().len(), 3, "All columns: every one");
        sheet.change(ExportChange::Choose(true), Locale::English);
        sheet.change(ExportChange::Toggle(1), Locale::English);
        let said = sheet.change(ExportChange::Move { from: 2, by: -2 }, Locale::English);
        assert_eq!(said.as_deref(), Some("title moved to position 1"));
        let names: Vec<&str> = sheet.chosen().iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["title", "id"]);
        assert_eq!(sheet.column, 0);
        assert_eq!(
            sheet.change(ExportChange::Move { from: 0, by: -1 }, Locale::English),
            None,
            "nothing above the first"
        );
        sheet.change(ExportChange::NoColumns, Locale::English);
        assert!(sheet.chosen().is_empty());
        sheet.change(ExportChange::AllColumns, Locale::English);
        assert_eq!(sheet.chosen().len(), 3);
    }

    #[test]
    fn a_typed_path_is_in_the_downloads_folder_unless_it_says_otherwise() {
        // Absolute on every system: Windows' have a drive.
        let home = std::env::temp_dir().join("reader");
        let downloads = home.join("Downloads");
        let elsewhere = std::env::temp_dir().join("reviews.csv");
        assert_eq!(
            typed_path("reviews.csv", &downloads, Some(&home)),
            downloads.join("reviews.csv")
        );
        assert_eq!(
            typed_path("~/exports/reviews.csv", &downloads, Some(&home)),
            home.join("exports").join("reviews.csv")
        );
        assert_eq!(
            typed_path(&elsewhere.display().to_string(), &downloads, Some(&home)),
            elsewhere
        );
        let separator = std::path::MAIN_SEPARATOR;
        assert_eq!(
            shown_path(&downloads.join("reviews.csv"), Some(&home)),
            format!("~{separator}Downloads{separator}reviews.csv")
        );
    }

    /// The disk-full sentence says what was written when it filled up.
    #[test]
    fn a_full_disk_says_what_was_written() {
        let job = crate::backend::export::ExportJob {
            id: crate::backend::export::ExportId(1),
            open: crate::backend::export::Opener {
                spec: tabletist_db::ConnectSpec::sqlite("bookshop.db"),
                secrets: tabletist_db::Secrets::default(),
                host_keys: tabletist_db::HostKeys::default(),
            },
            rows: crate::backend::export::ExportRows::Query(tabletist_db::RowQuery::new(
                tabletist_db::ObjectRef::new("main", "books"),
                0,
            )),
            columns: Vec::new(),
            masked: Vec::new(),
            format: Format::Csv,
            options: Options::default(),
            object: tabletist_db::ObjectRef::new("main", "books"),
            dialect: tabletist_db::Dialect::Sqlite,
            destination: crate::backend::export::Destination::Clipboard,
        };
        let failure = ExportFailure::DiskFull { written: 6_100_000 };
        let exporting = Exporting {
            job,
            tab: ConnTabId(1),
            table: "books".into(),
            file: "books.csv".into(),
            shown_path: "~/Downloads/books.csv".into(),
            connection: "Bookshop · dev".into(),
            total: Some(Count {
                rows: 31_400,
                exact: false,
            }),
            statement: None,
            sheet: Box::new(sheet()),
            status: ExportStatus::Failed {
                failure: failure.clone(),
                rows: 24_560,
                bytes: 0,
                at: Instant::now(),
            },
            hidden: false,
            started: Instant::now(),
            announced: None,
            reopens: false,
            history: None,
        };
        let (words, action) =
            failure_words(&exporting, &failure, (24_560, 0), None, Locale::English);
        assert_eq!(
            words,
            "The disk is full. 6.1 of ≈ 7.8 MB written. The partial file was removed."
        );
        assert_eq!(action.as_deref(), Some("Try again"));
        assert_eq!(
            failure_line(&exporting, &failure, (24_560, 0), None, Locale::English),
            "✗ disk full after 6.1 of ≈ 7.8 mb · partial file removed · :export to retry"
        );
    }

    #[test]
    fn sizes_times_and_counts_read_as_the_boards_write_them() {
        let en = Locale::English;
        assert_eq!(size_words(7_800_000, en), "7.8 MB");
        assert_eq!(size_words(312_400, en), "312 KB");
        assert_eq!(size_words(640, en), "640 bytes");
        assert_eq!(took_words(Duration::from_secs(18)), "18 s");
        assert_eq!(took_words(Duration::from_secs(125)), "2 min 5 s");
        assert_eq!(left_words(19.0, en), "about 20 s left");
        assert_eq!(left_words(130.0, en), "about 2 min left");
        let estimate = Count {
            rows: 31_400,
            exact: false,
        };
        assert_eq!(
            of_total(18_200, Some(estimate), en),
            "18,200 of ≈ 31,400 rows"
        );
        assert_eq!(of_total(4_000, None, en), "4,000 rows");
        assert_eq!(first_rows(12), RowsChoice::Selected);
        assert_eq!(first_rows(0), RowsChoice::Matching);
    }
}
