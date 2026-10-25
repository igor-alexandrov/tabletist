//! Dates and times as an editor reads them: what a column of dates or
//! times keeps (its kind, the digits of its seconds, its range), the text
//! typed into it read as a moment (ISO 8601, a day with a month's name,
//! the numbers of a day in the locale's order, a time of day, words such
//! as `tomorrow`), and the literal a save sends for it. The text is the
//! person's; what is sent is written here, the same way every time.
//!
//! Nothing here draws, and nothing reads a clock: the time it is now and
//! the person's zone come in a [`Clock`].

use jiff::civil::{Date, DateTime, Time, Weekday};
use jiff::tz::{AmbiguousOffset, Offset, TimeZone};
use jiff::{Timestamp, Unit};
use tabletist_db::{Dialect, Value};

/// What a column of dates or times keeps, which decides what its editor
/// reads and shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A day.
    Date,
    /// A day and a time, with no zone: stored as typed.
    Timestamp,
    /// A moment, shown and typed in a zone: PostgreSQL's `timestamptz`,
    /// and MySQL's `TIMESTAMP`, which the server moves to and from the
    /// session's zone.
    Zoned,
    /// A time of day. `offset` where the type keeps one (`timetz`).
    Time { offset: bool },
}

impl Kind {
    /// Whether a value of the kind has a day.
    pub fn has_date(self) -> bool {
        match self {
            Self::Date | Self::Timestamp | Self::Zoned => true,
            Self::Time { .. } => false,
        }
    }

    /// Whether a value of the kind has a time of day.
    pub fn has_time(self) -> bool {
        match self {
            Self::Timestamp | Self::Zoned | Self::Time { .. } => true,
            Self::Date => false,
        }
    }
}

/// The type's name without what is in its parentheses: `timestamp(3)
/// with time zone` is `timestamp with time zone`.
fn unsized_name(type_name: &str) -> String {
    let name = type_name.trim().to_ascii_lowercase();
    let mut plain = String::with_capacity(name.len());
    let mut depth = 0_u32;
    for letter in name.chars() {
        match letter {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            letter if depth == 0 => plain.push(letter),
            _ => {}
        }
    }
    plain.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The kind of the column whose type the catalog names `type_name`, by
/// the engine's own names. `None` for every other type: `interval`,
/// MySQL's `YEAR`, an array of dates (its editor is the array's).
pub fn kind(dialect: Dialect, type_name: &str) -> Option<Kind> {
    if type_name.trim().ends_with("[]") {
        return None;
    }
    let name = unsized_name(type_name);
    match dialect {
        Dialect::Postgres => match name.as_str() {
            "date" => Some(Kind::Date),
            "timestamp" | "timestamp without time zone" => Some(Kind::Timestamp),
            "timestamptz" | "timestamp with time zone" => Some(Kind::Zoned),
            "time" | "time without time zone" => Some(Kind::Time { offset: false }),
            "timetz" | "time with time zone" => Some(Kind::Time { offset: true }),
            _ => None,
        },
        // The first word: `datetime(6)`, `timestamp`, and whatever the
        // catalog writes after them.
        Dialect::MySql => match name.split(' ').next().unwrap_or_default() {
            "date" => Some(Kind::Date),
            "datetime" => Some(Kind::Timestamp),
            "timestamp" => Some(Kind::Zoned),
            "time" => Some(Kind::Time { offset: false }),
            _ => None,
        },
        // SQLite has no such types: the declared name says what the
        // column is meant to keep.
        Dialect::Sqlite => {
            let date = name.contains("date");
            let time = name.contains("time");
            if name.contains("timestamp") || (date && time) {
                Some(Kind::Timestamp)
            } else if date {
                Some(Kind::Date)
            } else if time {
                Some(Kind::Time { offset: false })
            } else {
                None
            }
        }
    }
}

/// The digits of a second's fraction a column keeps: those its type
/// states, else the engine's own. SQLite keeps what it is given.
fn kept_digits(dialect: Dialect, type_name: &str) -> u8 {
    let stated = type_name
        .split_once('(')
        .and_then(|(_, rest)| rest.split(')').next())
        .and_then(|digits| digits.trim().parse::<u8>().ok());
    match (dialect, stated) {
        (Dialect::Postgres | Dialect::MySql, Some(digits)) => digits.min(6),
        (Dialect::Postgres, None) => 6,
        (Dialect::MySql, None) => 0,
        (Dialect::Sqlite, _) => 9,
    }
}

/// How the values a SQLite column already holds are kept, which a new
/// value follows. Other engines keep a type, and are written as theirs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stored {
    /// No value to follow: ISO 8601 text.
    Fresh,
    /// Text, its day and time apart by `separator` (` ` or `T`).
    Text { separator: char },
    /// Whole seconds since 1970 (UTC).
    Seconds,
    /// Milliseconds since 1970 (UTC), as some libraries write them.
    Millis,
    /// Julian day numbers, as SQLite's `julianday()` gives them: days and
    /// their fraction since noon UTC on 24 November 4714 BC. A whole one,
    /// a noon, is kept as an INTEGER.
    Days,
    /// A day's number written as its digits: `20260115`.
    Compact,
    /// Numbers of no form told apart here (or of several): a new value is
    /// ISO 8601 text, which says what it is.
    Unknown,
}

/// The Julian days of 0000-01-01 and 9999-12-31, at midnight UTC.
const JULIAN_DAYS: std::ops::RangeInclusive<f64> = 1_721_059.5..=5_373_484.5;
/// The last second of 9999, in seconds since 1970.
const LAST_SECOND: i64 = 253_402_300_799;
/// The least a value of milliseconds is taken to be (3 March 1973): as
/// seconds it would be a day past the year 5000.
const LEAST_MILLIS: i64 = 100_000_000_000;
/// [`LEAST_MILLIS`], to compare a value's size with: the size of the least
/// `i64` has no `i64` to hold it.
const MILLIS: u64 = LEAST_MILLIS as u64;

/// The day a number such as `20260115` is, years 1000 to 9999.
fn compact_day(number: i64) -> Option<Date> {
    let (year, month, day) = (number / 10_000, number / 100 % 100, number % 100);
    if !(1000..=9999).contains(&year) {
        return None;
    }
    Date::new(year as i16, month as i8, day as i8).ok()
}

impl Stored {
    /// How the values of `values` are kept, by the first that is not
    /// NULL: text, or numbers, whose form all of the page's decide.
    pub fn of<'a>(values: impl IntoIterator<Item = &'a Value>) -> Self {
        let mut values = values
            .into_iter()
            .filter(|value| !value.is_null())
            .peekable();
        match values.peek() {
            // The NULLs were left out.
            None | Some(Value::Null) => return Self::Fresh,
            Some(Value::Text(text)) => {
                let separator = match text.as_bytes().get(10) {
                    Some(b'T') => 'T',
                    _ => ' ',
                };
                return Self::Text { separator };
            }
            Some(Value::Bool(_) | Value::Bytes(_)) => return Self::Fresh,
            Some(Value::Int(_) | Value::Float(_)) => {}
        }
        let (mut whole, mut real) = (Vec::new(), Vec::new());
        for value in values {
            match value {
                Value::Int(number) => whole.push(*number),
                Value::Float(number) => real.push(*number),
                Value::Null | Value::Bool(_) | Value::Text(_) | Value::Bytes(_) => {}
            }
        }
        Self::of_numbers(&whole, &real)
    }

    /// The form whole numbers `whole` and real ones `real` are all of.
    fn of_numbers(whole: &[i64], real: &[f64]) -> Self {
        let julian = |number: f64| JULIAN_DAYS.contains(&number);
        let all = |test: &dyn Fn(i64) -> bool| whole.iter().all(|number| test(*number));
        if !real.is_empty() {
            let days = real.iter().all(|number| julian(*number));
            return if days && all(&|number| julian(number as f64)) {
                Self::Days
            } else {
                Self::Unknown
            };
        }
        if all(&|number| compact_day(number).is_some()) {
            Self::Compact
        } else if all(&|number| julian(number as f64)) {
            // As seconds these are days of early 1970.
            Self::Days
        } else if all(&|number| {
            (MILLIS..=LAST_SECOND as u64 * 1000 + 999).contains(&number.unsigned_abs())
        }) {
            Self::Millis
        } else if all(&|number| number.unsigned_abs() <= LAST_SECOND as u64) {
            Self::Seconds
        } else {
            Self::Unknown
        }
    }
}

/// A column of dates or times, as an editor reads text for it.
#[derive(Debug, Clone, PartialEq)]
pub struct Column {
    pub dialect: Dialect,
    pub kind: Kind,
    /// The digits of a second's fraction it keeps.
    pub digits: u8,
    /// How SQLite's values of it are kept ([`Stored::Fresh`] elsewhere).
    pub stored: Stored,
    /// The zone a [`Kind::Zoned`] value is shown and typed in: the
    /// settings' for PostgreSQL, the session's for MySQL. `None` where a
    /// MySQL session's is not known (not answered yet, or named in a way
    /// the database of zones does not know): the value is read as the
    /// server's clock writes it, and no change of the clocks is found.
    pub zone: Option<TimeZone>,
}

impl Column {
    /// The column of `type_name`, where it keeps dates or times. `values`
    /// are those of it loaded, which a SQLite value follows.
    pub fn of<'a>(
        dialect: Dialect,
        type_name: &str,
        zone: Option<&TimeZone>,
        values: impl IntoIterator<Item = &'a Value>,
    ) -> Option<Self> {
        let kind = kind(dialect, type_name)?;
        let stored = match dialect {
            Dialect::Sqlite => Stored::of(values),
            Dialect::Postgres | Dialect::MySql => Stored::Fresh,
        };
        // A number such as `20260115` is a day's, and no time's.
        let stored = match (stored, kind) {
            (Stored::Compact, Kind::Timestamp | Kind::Zoned | Kind::Time { .. }) => Stored::Unknown,
            (stored, _) => stored,
        };
        Some(Self {
            dialect,
            kind,
            digits: kept_digits(dialect, type_name),
            stored,
            zone: match kind {
                Kind::Zoned => zone.cloned(),
                Kind::Date | Kind::Timestamp | Kind::Time { .. } => None,
            },
        })
    }
}

/// The order a day written in numbers alone is read in (`15/01/2026`), by
/// the region of the system's locale.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Order {
    /// `15/01/2026`.
    #[default]
    DayMonth,
    /// `01/15/2026`.
    MonthDay,
    /// `2026/01/15`.
    YearMonth,
}

impl Order {
    /// The order of the locale for times, as POSIX decides it: `LC_ALL`
    /// where it is set, whatever region it names or none (`C`); else
    /// `LC_TIME`; else the first of the system's `languages` that names a
    /// region.
    pub fn of_locale(all: Option<&str>, time: Option<&str>, languages: &[String]) -> Self {
        let region = |tag: &str| fastframe_i18n::LanguageTag::parse(tag).and_then(|tag| tag.region);
        let (all, time) = (
            all.filter(|all| !all.is_empty()),
            time.filter(|time| !time.is_empty()),
        );
        let found = match (all, time) {
            (Some(all), _) => region(all),
            (None, Some(time)) => region(time),
            (None, None) => languages.iter().find_map(|tag| region(tag)),
        };
        Self::of_region(found.as_deref())
    }

    /// The order of the region `region` (`us`, `de`, `jp`: lower case, as
    /// `fastframe_i18n::LanguageTag` gives it), by CLDR's short date
    /// patterns. Day first where it is not known.
    pub fn of_region(region: Option<&str>) -> Self {
        match region.unwrap_or_default() {
            "us" | "ph" | "fm" | "mh" | "pw" | "as" | "gu" | "mp" | "pr" | "um" | "vi" | "bz" => {
                Self::MonthDay
            }
            "cn" | "jp" | "kr" | "kp" | "tw" | "hu" | "lt" | "mn" | "ir" | "bt" | "se" | "za"
            | "ca" => Self::YearMonth,
            _ => Self::DayMonth,
        }
    }
}

/// What a reading needs of the moment it is read at: the time it is, the
/// zone the person's days are counted in, and how they write a day.
#[derive(Debug, Clone)]
pub struct Clock {
    pub now: Timestamp,
    pub zone: TimeZone,
    pub order: Order,
}

impl Clock {
    /// Today, in `zone`.
    pub fn today_in(&self, zone: &TimeZone) -> Date {
        self.now.to_zoned(zone.clone()).date()
    }

    /// The time it is in `zone`, to the second.
    fn now_in(&self, zone: &TimeZone) -> DateTime {
        let at = self.now.to_zoned(zone.clone()).datetime();
        at.with().subsec_nanosecond(0).build().unwrap_or(at)
    }
}

/// A value read: a day, a time, or both, with the offset a zoned value
/// has, or one of the values that are no day.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Moment {
    At {
        date: Option<Date>,
        time: Option<Time>,
        offset: Option<Offset>,
    },
    /// PostgreSQL's `infinity`: later than every day.
    Infinity,
    /// PostgreSQL's `-infinity`: earlier than every day.
    MinusInfinity,
    /// `24:00:00`: the end of a day, which a PostgreSQL or SQLite time
    /// holds.
    EndOfDay,
    /// A MySQL `TIME` past a day or below zero, which the type keeps as a
    /// span of time: written as it is sent (`-12:00:00`, `100:30:00`).
    Span(String),
}

/// The two moments a time the clocks go back over is, in the zone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fold {
    /// The time, as the zone's clock shows it twice.
    pub at: DateTime,
    /// Where it is, as the zone's name says it ("New York").
    pub place: String,
    /// The first, before the clocks go back (summer time), with the
    /// zone's abbreviation then ("EDT").
    pub earlier: (Offset, String),
    /// The second, after it ("EST").
    pub later: (Offset, String),
}

/// What a text is as a value of its column.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reading {
    /// The text read.
    pub typed: String,
    pub moment: Moment,
    /// The digits of the second's fraction it has, as it will be stored.
    pub digits: u8,
    /// What a save sends.
    pub literal: String,
    /// The day was named by a word or a count from today.
    pub relative: bool,
    /// How many days from today the day is (in the zone it is counted
    /// in): `1` tomorrow, `-1` yesterday. `None` for a value with no day.
    pub days: Option<i64>,
    /// The zone's abbreviation for a zoned value then ("EST").
    pub zone_name: Option<String>,
    /// The time is one the clocks go back over: the value is the moment
    /// of its offset, the first one where none was typed.
    pub fold: Option<Fold>,
}

/// The limits a type sets, named by what holds them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Range {
    /// PostgreSQL's `date`: 4714-11-24 BC to 5874897 AD.
    PgDate,
    /// PostgreSQL's `timestamp` and `timestamptz`: 4714-11-24 BC to 294276
    /// AD.
    PgTimestamp,
    /// MySQL's `DATE`: 1000-01-01 to 9999-12-31.
    MySqlDate,
    /// MySQL's `DATETIME`: 1000-01-01 to 9999-12-31.
    MySqlDatetime,
    /// MySQL's `TIMESTAMP`: 1970-01-01 00:00:01 to 2038-01-19 03:14:07
    /// UTC.
    MySqlTimestamp,
    /// MySQL's `TIME`, a span of time: -838:59:59 to 838:59:59.
    MySqlTime,
    /// PostgreSQL's `timetz` offsets: -15:59:59 to +15:59:59.
    PgOffset,
}

/// Why a text is no value of its column.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// Nothing here reads as a value of the kind.
    NotRead(Kind),
    /// A month that is none: `13`.
    NoMonth(i64),
    /// A day the month does not have: 30 February.
    NoDay { day: i64, month: i8 },
    /// A time of day that is none, as it was typed: `25:00`.
    NoTime(String),
    /// Outside what the type holds.
    Range(Range),
    /// A year of more than four digits, which the app does not read.
    Years,
    /// PostgreSQL counts no year 0: 1 BC is just before 1 AD.
    YearZero,
    /// A time the zone's clocks skip. `offered` is the time the same
    /// moment is once they have gone forward: 03:30 for 02:30.
    Gap {
        at: DateTime,
        place: String,
        offered: DateTime,
    },
    /// MySQL's zero date, `0000-00-00`, which a server may hold and
    /// nothing should be given.
    Zero,
    /// An offset typed for a column that keeps no zone.
    NoZone,
    /// A time typed for a column that keeps a day only.
    DayOnly,
    /// A day typed for a column that keeps a time only.
    TimeOnly,
    /// A day counted from today, a time without its day, or an offset, for
    /// a MySQL `TIMESTAMP` whose session's zone is not known: what today is
    /// there, and what the offset is in its clock, are the server's to
    /// say.
    ZoneUnknown,
}

/// The refusal of a text that reads as nothing, before [`read`] puts the
/// column's kind in.
const UNREAD: Refusal = Refusal::NotRead(Kind::Date);

/// The place a zone is for, as its name says it: `America/New_York` is
/// "New York". The zone's name where it has no place in it ("UTC"), and
/// nothing where it has no name.
pub fn place(zone: &TimeZone) -> String {
    let name = zone.iana_name().unwrap_or_default();
    name.rsplit('/')
        .next()
        .unwrap_or_default()
        .replace('_', " ")
}

/// An offset as PostgreSQL writes one: `+00`, `-05`, `+05:30`,
/// `-04:56:02`.
pub fn written_offset(offset: Offset) -> String {
    let seconds = offset.seconds();
    let sign = if seconds < 0 { '-' } else { '+' };
    let seconds = seconds.unsigned_abs();
    let (hours, minutes, seconds) = (seconds / 3_600, seconds / 60 % 60, seconds % 60);
    match (minutes, seconds) {
        (0, 0) => format!("{sign}{hours:02}"),
        (_, 0) => format!("{sign}{hours:02}:{minutes:02}"),
        _ => format!("{sign}{hours:02}:{minutes:02}:{seconds:02}"),
    }
}

/// `time` as `HH:MM:SS`, with `digits` of its fraction.
fn written_time(time: Time, digits: u8) -> String {
    let mut out = format!(
        "{:02}:{:02}:{:02}",
        time.hour(),
        time.minute(),
        time.second()
    );
    if digits > 0 {
        let nanos = format!("{:09}", time.subsec_nanosecond());
        out.push('.');
        out.push_str(&nanos[..usize::from(digits.min(9))]);
    }
    out
}

/// Whether the engine writes a year before 1 AD with its era (`0044 BC`):
/// PostgreSQL alone. SQLite's year 0 is `0000`, and MySQL holds none.
fn eras(dialect: Dialect) -> bool {
    match dialect {
        Dialect::Postgres => true,
        Dialect::MySql | Dialect::Sqlite => false,
    }
}

/// `date` as `YYYY-MM-DD`, its year as written: before 1 AD, where the
/// engine writes an era, the year counted back (1 BC is `0001`), which
/// `bc` then follows.
fn written_date(date: Date, dialect: Dialect) -> (String, bool) {
    let year = i32::from(date.year());
    let bc = year < 1 && eras(dialect);
    let shown = if bc { 1 - year } else { year };
    (
        format!("{shown:04}-{:02}-{:02}", date.month(), date.day()),
        bc,
    )
}

/// The value `typed` reads as for `column`, at `clock`, or why it reads
/// as none.
pub fn read(typed: &str, column: &Column, clock: &Clock) -> Result<Reading, Refusal> {
    let mut reading = reading(typed, column, clock).map_err(|refusal| match refusal {
        Refusal::NotRead(_) => Refusal::NotRead(column.kind),
        refusal => refusal,
    })?;
    reading.typed = typed.to_owned();
    let today = clock.today_in(column.zone.as_ref().unwrap_or(&clock.zone));
    reading.days = match reading.moment {
        Moment::At {
            date: Some(date), ..
        } => today
            .until((Unit::Day, date))
            .ok()
            .map(|span| i64::from(span.get_days())),
        Moment::At { date: None, .. }
        | Moment::Infinity
        | Moment::MinusInfinity
        | Moment::Span(_)
        | Moment::EndOfDay => None,
    };
    Ok(reading)
}

/// [`read`], before the text and the count of days are put in.
fn reading(typed: &str, column: &Column, clock: &Clock) -> Result<Reading, Refusal> {
    let text = typed.trim().to_ascii_lowercase();
    if let Some(moment) = special(&text, column)? {
        let literal = match &moment {
            Moment::Infinity => "infinity".to_owned(),
            Moment::MinusInfinity => "-infinity".to_owned(),
            Moment::Span(written) => written.clone(),
            Moment::EndOfDay => "24:00:00".to_owned(),
            // SQLite's seconds: the day and time they are, written as the
            // column keeps them.
            Moment::At { date, time, .. } => {
                let parts = Parts {
                    day: date.map(Day::Known),
                    time: time.map(|time| (time, 0)),
                    ..Parts::default()
                };
                return resolve(parts, column, clock);
            }
        };
        return Ok(Reading {
            typed: String::new(),
            moment,
            digits: 0,
            literal,
            relative: false,
            days: None,
            zone_name: None,
            fold: None,
        });
    }
    let parts = scan(&text, column, clock)?;
    resolve(parts, column, clock)
}

/// The words a value that is no day is typed as, and SQLite's seconds.
fn special(text: &str, column: &Column) -> Result<Option<Moment>, Refusal> {
    match column.dialect {
        Dialect::Postgres if column.kind.has_time() && !column.kind.has_date() => {
            Ok(end_of_day(text, column))
        }
        Dialect::Postgres => Ok(match text {
            "infinity" | "+infinity" => Some(Moment::Infinity),
            "-infinity" => Some(Moment::MinusInfinity),
            _ => None,
        }),
        Dialect::MySql => {
            let zero = text.strip_prefix("0000-00-00").is_some_and(|rest| {
                let rest = rest.trim_start_matches([' ', 't']);
                rest.is_empty()
                    || rest
                        .strip_prefix("00:00:00")
                        .is_some_and(|fraction| fraction.trim_start_matches(['.', '0']).is_empty())
            });
            if zero && column.kind.has_date() {
                return Err(Refusal::Zero);
            }
            match column.kind {
                Kind::Time { .. } => span(text, column.digits),
                Kind::Date | Kind::Timestamp | Kind::Zoned => Ok(None),
            }
        }
        Dialect::Sqlite => {
            if let Some(end) = end_of_day(text, column) {
                return Ok(Some(end));
            }
            let Some(moment) = numbered(text, column.stored) else {
                return Ok(None);
            };
            let at = moment?.to_zoned(TimeZone::UTC).datetime();
            Ok(Some(Moment::At {
                date: column.kind.has_date().then(|| at.date()),
                time: column.kind.has_time().then(|| at.time()),
                offset: None,
            }))
        }
    }
}

/// The moment SQLite's number `text` is, where the column keeps its values
/// as numbers. `None` for any other text.
///
/// A number is of the column's form only within the range that form is
/// told by ([`Stored::of`]): `9.30` in a column of Julian days is a time,
/// not the ninth day of 4713 BC, and is read as one.
fn numbered(text: &str, stored: Stored) -> Option<Result<Timestamp, Refusal>> {
    let moment = match stored {
        Stored::Seconds => Timestamp::from_second(text.parse::<i64>().ok()?),
        Stored::Millis => {
            let millis = text.parse::<i64>().ok()?;
            if millis.unsigned_abs() < MILLIS {
                return None;
            }
            Timestamp::from_millisecond(millis)
        }
        Stored::Compact => {
            let day = compact_day(text.parse::<i64>().ok()?)?;
            day.to_zoned(TimeZone::UTC).map(|zoned| zoned.timestamp())
        }
        Stored::Days => {
            let days = text
                .parse::<f64>()
                .ok()
                .filter(|days| JULIAN_DAYS.contains(days))?;
            let millis = ((days - JULIAN_EPOCH) * MILLIS_A_DAY).round();
            Timestamp::from_millisecond(millis as i64)
        }
        Stored::Fresh | Stored::Text { .. } | Stored::Unknown => return None,
    };
    Some(moment.map_err(|_| Refusal::Years))
}

/// The Julian day number of 1970-01-01 00:00 UTC, and a day's
/// milliseconds.
const JULIAN_EPOCH: f64 = 2_440_587.5;
const MILLIS_A_DAY: f64 = 86_400_000.0;

/// `24:00` (with seconds or a fraction of nothing), the end of a day, which
/// a time of PostgreSQL's or SQLite's holds. `None` for any other text.
fn end_of_day(text: &str, column: &Column) -> Option<Moment> {
    let rest = text.strip_prefix("24:00")?;
    let rest = rest.strip_prefix(":00").unwrap_or(rest);
    let rest = match rest.strip_prefix('.') {
        Some(fraction) if fraction.bytes().all(|byte| byte == b'0') => "",
        Some(_) => return None,
        None => rest,
    };
    (rest.is_empty() && !column.kind.has_date()).then_some(Moment::EndOfDay)
}

/// A MySQL `TIME` that is no time of day: below zero, or of a day and
/// more (`-12:00`, `100:30:00`), with `places` of its fraction kept. `None`
/// for any other text: a time of day is read as one.
fn span(text: &str, places: u8) -> Result<Option<Moment>, Refusal> {
    let (negative, rest) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text),
    };
    let (hours, rest) = digits(rest);
    let Some(rest) = rest.strip_prefix(':') else {
        return Ok(None);
    };
    let (minutes, rest) = digits(rest);
    let (seconds, rest) = match rest.strip_prefix(':') {
        Some(rest) => digits(rest),
        None => ("00", rest),
    };
    let (fraction, rest) = match rest.strip_prefix('.') {
        Some(rest) => digits(rest),
        None => ("", rest),
    };
    let formed = (1..=3).contains(&hours.len())
        && minutes.len() == 2
        && seconds.len() == 2
        && rest.is_empty();
    let number = |part: &str| part.parse::<u32>().unwrap_or(u32::MAX);
    let (hour, minute, second) = (number(hours), number(minutes), number(seconds));
    if !formed || !(negative || hour >= 24) {
        return Ok(None);
    }
    if minute > 59 || second > 59 {
        return Err(Refusal::NoTime(text.to_owned()));
    }
    if fraction.len() > 9 {
        return Ok(None);
    }
    // Rounded to the places kept, as the server rounds: half up, carried
    // into the seconds and on.
    let nanos: u64 = format!("{fraction:0<9}").parse().unwrap_or_default();
    let step = 10_u64.pow(9 - u32::from(places.min(9)));
    let whole = (u64::from(hour) * 3_600 + u64::from(minute) * 60 + u64::from(second))
        * 1_000_000_000
        + nanos;
    let whole = (whole + step / 2) / step * step;
    if whole > 838 * 3_600 * 1_000_000_000 + 3_599 * 1_000_000_000 {
        return Err(Refusal::Range(Range::MySqlTime));
    }
    let (seconds, nanos) = (whole / 1_000_000_000, whole % 1_000_000_000);
    let sign = if negative && whole > 0 { "-" } else { "" };
    let mut written = format!(
        "{sign}{:02}:{:02}:{:02}",
        seconds / 3_600,
        seconds / 60 % 60,
        seconds % 60
    );
    if places > 0 && nanos > 0 {
        let nanos = format!("{nanos:09}");
        written.push('.');
        written.push_str(&nanos[..usize::from(places.min(9))]);
    }
    Ok(Some(Moment::Span(written)))
}

/// A day as it was typed, before its era and its month are checked.
#[derive(Debug, Clone, Copy)]
enum Day {
    Numbers {
        year: i64,
        month: i64,
        day: i64,
    },
    /// Named by a word or a count from today.
    Known(Date),
    /// `now`: today, and the time it is.
    Now,
}

/// What a text was made of.
#[derive(Debug, Default)]
struct Parts {
    day: Option<Day>,
    time: Option<(Time, u8)>,
    offset: Option<Offset>,
    bc: bool,
    relative: bool,
}

/// Splits the run of ASCII digits at the start of `text` off it.
fn digits(text: &str) -> (&str, &str) {
    let end = text.bytes().take_while(u8::is_ascii_digit).count();
    text.split_at(end)
}

/// Splits the run of ASCII letters at the start of `text` off it.
fn letters(text: &str) -> (&str, &str) {
    let end = text.bytes().take_while(u8::is_ascii_alphabetic).count();
    text.split_at(end)
}

/// The month `word` begins the English name of, three letters at least.
fn month(word: &str) -> Option<i64> {
    const MONTHS: [&str; 12] = [
        "january",
        "february",
        "march",
        "april",
        "may",
        "june",
        "july",
        "august",
        "september",
        "october",
        "november",
        "december",
    ];
    if word.len() < 3 {
        return None;
    }
    let at = MONTHS.iter().position(|name| name.starts_with(word))?;
    i64::try_from(at).ok().map(|at| at + 1)
}

/// The weekday `word` begins the English name of, three letters at least.
fn weekday(word: &str) -> Option<Weekday> {
    const DAYS: [(&str, Weekday); 7] = [
        ("monday", Weekday::Monday),
        ("tuesday", Weekday::Tuesday),
        ("wednesday", Weekday::Wednesday),
        ("thursday", Weekday::Thursday),
        ("friday", Weekday::Friday),
        ("saturday", Weekday::Saturday),
        ("sunday", Weekday::Sunday),
    ];
    if word.len() < 3 {
        return None;
    }
    DAYS.iter()
        .find(|(name, _)| name.starts_with(word))
        .map(|(_, day)| *day)
}

/// A year after a day and its month: four digits or more, not the hour
/// of a time that follows.
fn year(text: &str) -> Option<(i64, &str)> {
    let text = text.trim_start_matches([' ', ',']);
    let (year, rest) = digits(text);
    if year.len() < 4 || rest.starts_with([':', '.']) {
        return None;
    }
    Some((year.parse().ok()?, rest))
}

/// `2026-01-15`.
fn iso_day(text: &str) -> Option<(Day, &str)> {
    let (year, rest) = digits(text);
    if year.len() < 4 {
        return None;
    }
    let (month, rest) = digits(rest.strip_prefix('-')?);
    let (day, rest) = digits(rest.strip_prefix('-')?);
    if !(1..=2).contains(&month.len()) || !(1..=2).contains(&day.len()) {
        return None;
    }
    let numbers = Day::Numbers {
        year: year.parse().ok()?,
        month: month.parse().ok()?,
        day: day.parse().ok()?,
    };
    Some((numbers, rest))
}

/// `15/01/2026`, `15.01.2026` or `15-01-2026` in the locale's order, or
/// `2026/01/15` with its year first in any.
fn numeric_day(text: &str, order: Order) -> Option<(Day, &str)> {
    let (first, rest) = digits(text);
    let separator = rest
        .chars()
        .next()
        .filter(|c| ['/', '.', '-'].contains(c))?;
    let (second, rest) = digits(&rest[1..]);
    let (third, rest) = digits(rest.strip_prefix(separator)?);
    if first.is_empty() || !(1..=2).contains(&second.len()) || third.is_empty() {
        return None;
    }
    let number = |part: &str| part.parse::<i64>().ok();
    let (year, month, day) = if first.len() >= 4 {
        (first, second, third)
    } else if third.len() >= 4 && first.len() <= 2 {
        match order {
            Order::DayMonth | Order::YearMonth => (third, second, first),
            Order::MonthDay => (third, first, second),
        }
    } else {
        return None;
    };
    let numbers = Day::Numbers {
        year: number(year)?,
        month: number(month)?,
        day: number(day)?,
    };
    Some((numbers, rest))
}

/// `15 Jan 2026`, `Jan 15 2026`, `January 15, 2026`; this year's where no
/// year follows.
fn named_day(text: &str, this_year: i64) -> Option<(Day, &str, bool)> {
    let (number, rest) = digits(text);
    let (day, month_number, rest) = if (1..=2).contains(&number.len()) {
        let (word, rest) = letters(rest.trim_start());
        (number, month(word)?, rest)
    } else {
        let (word, rest) = letters(text);
        let month_number = month(word)?;
        let (day, rest) = digits(rest.trim_start_matches([' ', '.']));
        if !(1..=2).contains(&day.len()) {
            return None;
        }
        let rest = ["st", "nd", "rd", "th"]
            .iter()
            .find_map(|ending| rest.strip_prefix(ending))
            .unwrap_or(rest);
        (day, month_number, rest)
    };
    let rest = rest.strip_prefix('.').unwrap_or(rest);
    let typed = year(rest);
    let named = typed.is_some();
    let (year, rest) = typed.unwrap_or((this_year, rest));
    let numbers = Day::Numbers {
        year,
        month: month_number,
        day: day.parse().ok()?,
    };
    Some((numbers, rest, named))
}

/// `today`, `tomorrow`, `yesterday`, `now`, `next fri`, `fri`, `last mon`,
/// `+7d`, `-2w`, `+1m`, `+1y`.
fn relative_day(text: &str, today: Date) -> Result<Option<(Day, &str)>, Refusal> {
    let (word, rest) = letters(text);
    let known = |date: Result<Date, jiff::Error>| date.map(Day::Known).map_err(|_| Refusal::Years);
    let found = match word {
        "today" => Day::Known(today),
        "tomorrow" => known(today.tomorrow())?,
        "yesterday" => known(today.yesterday())?,
        "now" => Day::Now,
        "next" | "last" => {
            let (name, after) = letters(rest.trim_start());
            let Some(day) = weekday(name) else {
                return Ok(None);
            };
            let nth = if word == "next" { 1 } else { -1 };
            return Ok(Some((known(today.nth_weekday(nth, day))?, after)));
        }
        "" => {
            let sign = match text.as_bytes().first() {
                Some(b'+') => 1,
                Some(b'-') => -1,
                _ => return Ok(None),
            };
            let (count, rest) = digits(&text[1..]);
            let (unit, rest) = letters(rest);
            let Ok(count) = count.parse::<i64>() else {
                return Ok(None);
            };
            let count = sign * count;
            // A count past what a span holds is past every day there is.
            let span = jiff::Span::new();
            let span = match unit {
                "d" | "day" | "days" => span.try_days(count),
                "w" | "week" | "weeks" => span.try_weeks(count),
                "m" | "month" | "months" => span.try_months(count),
                "y" | "year" | "years" => span.try_years(count),
                _ => return Ok(None),
            }
            .map_err(|_| Refusal::Years)?;
            return Ok(Some((known(today.checked_add(span))?, rest)));
        }
        name => match weekday(name) {
            Some(day) => known(today.nth_weekday(1, day))?,
            None => return Ok(None),
        },
    };
    Ok(Some((found, rest)))
}

/// What comes between a day and its time: `T`, a space, a comma, `at`.
fn separator(text: &str) -> &str {
    if let Some(rest) = text.strip_prefix('t')
        && rest.starts_with(|c: char| c.is_ascii_digit())
    {
        return rest;
    }
    let rest = text.trim_start_matches([' ', ',']);
    rest.strip_prefix("at ").unwrap_or(rest).trim_start()
}

/// `9:30`, `21:30:15.25`, `9.30`, `9am`, `9:30 pm`, with the digits of
/// its fraction. `None` where `text` begins with no time.
fn time_of_day(text: &str) -> Result<Option<(Time, u8, &str)>, Refusal> {
    let (hour, mut rest) = digits(text);
    if !(1..=2).contains(&hour.len()) {
        return Ok(None);
    }
    let (mut minute, mut second, mut nanos, mut fraction) = ("0", "0", 0_i32, 0_u8);
    let mut written = false;
    if let Some(after) = rest.strip_prefix(':') {
        let (part, after) = digits(after);
        if part.len() != 2 {
            return Err(UNREAD);
        }
        (minute, rest, written) = (part, after, true);
        if let Some(after) = rest.strip_prefix(':') {
            let (part, after) = digits(after);
            if part.len() != 2 {
                return Err(UNREAD);
            }
            (second, rest) = (part, after);
            if let Some(after) = rest.strip_prefix('.') {
                let (part, after) = digits(after);
                if !(1..=9).contains(&part.len()) {
                    return Err(UNREAD);
                }
                fraction = part.len() as u8;
                nanos = format!("{part:0<9}").parse().unwrap_or_default();
                rest = after;
            }
        }
    } else if let Some(after) = rest.strip_prefix('.') {
        let (part, after) = digits(after);
        if part.len() == 2 {
            (minute, rest, written) = (part, after, true);
        }
    }
    let after = rest.trim_start();
    let meridiem = ["am", "pm", "a.m.", "p.m."]
        .iter()
        .find_map(|word| Some((word.starts_with('p'), after.strip_prefix(word)?)));
    if !written && meridiem.is_none() {
        return Ok(None);
    }
    if let Some((_, after)) = meridiem {
        rest = after;
    }
    let piece = text[..text.len() - rest.len()].trim();
    let refused = || Refusal::NoTime(piece.to_owned());
    let number = |part: &str| part.parse::<i8>().map_err(|_| refused());
    let mut hour = number(hour)?;
    if let Some((afternoon, _)) = meridiem {
        if !(1..=12).contains(&hour) {
            return Err(refused());
        }
        hour = hour % 12 + if afternoon { 12 } else { 0 };
    }
    let time = Time::new(hour, number(minute)?, number(second)?, nanos).map_err(|_| refused())?;
    Ok(Some((time, fraction, rest)))
}

/// `Z`, `UTC`, `+05`, `-05:00`, `+0530`, `-04:56:02`.
fn offset(text: &str) -> Option<(Offset, &str)> {
    for word in ["z", "utc", "gmt"] {
        if let Some(rest) = text.strip_prefix(word)
            && !rest.starts_with(|c: char| c.is_ascii_alphabetic())
        {
            return Some((Offset::UTC, rest));
        }
    }
    let sign = match text.as_bytes().first()? {
        b'+' => 1,
        b'-' => -1,
        _ => return None,
    };
    let (first, mut rest) = digits(&text[1..]);
    let (hours, mut minutes, mut seconds) = match first.len() {
        2 => (first, "0", "0"),
        4 => (&first[..2], &first[2..], "0"),
        _ => return None,
    };
    if first.len() == 2
        && let Some(after) = rest.strip_prefix(':')
    {
        let (part, after) = digits(after);
        if part.len() != 2 {
            return None;
        }
        (minutes, rest) = (part, after);
        if let Some(after) = rest.strip_prefix(':') {
            let (part, after) = digits(after);
            if part.len() != 2 {
                return None;
            }
            (seconds, rest) = (part, after);
        }
    }
    let number = |part: &str| part.parse::<i32>().ok();
    let (minutes, seconds) = (number(minutes)?, number(seconds)?);
    if minutes > 59 || seconds > 59 {
        return None;
    }
    let total = number(hours)? * 3_600 + minutes * 60 + seconds;
    Some((Offset::from_seconds(sign * total).ok()?, rest))
}

/// The pieces of `text`, lower case and trimmed.
fn scan(text: &str, column: &Column, clock: &Clock) -> Result<Parts, Refusal> {
    let mut parts = Parts::default();
    let mut rest = text;
    // PostgreSQL writes an era after the whole value.
    if eras(column.dialect) && column.kind.has_date() {
        if let Some(before) = rest.strip_suffix(" bc") {
            (parts.bc, rest) = (true, before.trim_end());
        } else if let Some(before) = rest.strip_suffix(" ad") {
            rest = before.trim_end();
        }
    }
    let zone = column.zone.as_ref().unwrap_or(&clock.zone);
    let today = clock.today_in(zone);
    // Whether the day's year was written: an era is of a year typed.
    let mut year_typed = true;
    let day = match relative_day(rest, today)? {
        Some(found) => {
            parts.relative = true;
            Some(found)
        }
        None => iso_day(rest)
            .or_else(|| numeric_day(rest, clock.order))
            .or_else(|| {
                let (day, after, named) = named_day(rest, i64::from(today.year()))?;
                year_typed = named;
                Some((day, after))
            }),
    };
    if let Some((day, after)) = day {
        parts.day = Some(day);
        rest = separator(after);
    }
    let numbers = matches!(parts.day, Some(Day::Numbers { .. }));
    if parts.bc && !(numbers && year_typed) {
        return Err(UNREAD);
    }
    if !matches!(parts.day, Some(Day::Now))
        && let Some((time, fraction, after)) = time_of_day(rest)?
    {
        parts.time = Some((time, fraction));
        rest = after;
    }
    let after = rest.trim_start();
    if let Some((found, after)) = offset(after) {
        parts.offset = Some(found);
        rest = after;
    }
    if !rest.trim().is_empty() || (parts.day.is_none() && parts.time.is_none()) {
        return Err(UNREAD);
    }
    Ok(parts)
}

/// The day the numbers name, in the era typed, as the column's engine
/// counts years.
fn checked_day(
    column: &Column,
    year: i64,
    month: i64,
    day: i64,
    bc: bool,
) -> Result<Date, Refusal> {
    if year > 9999 {
        return Err(Refusal::Years);
    }
    if eras(column.dialect) && year == 0 {
        return Err(Refusal::YearZero);
    }
    let year = if bc { 1 - year } else { year };
    let month = i8::try_from(month)
        .ok()
        .filter(|month| (1..=12).contains(month))
        .ok_or(Refusal::NoMonth(month))?;
    let first = Date::new(i16::try_from(year).map_err(|_| Refusal::Years)?, month, 1)
        .map_err(|_| Refusal::Years)?;
    let day = i8::try_from(day)
        .ok()
        .filter(|day| (1..=first.days_in_month()).contains(day))
        .ok_or(Refusal::NoDay { day, month })?;
    first.with().day(day).build().map_err(|_| UNREAD)
}

/// `at` rounded to `digits` of a second, as the databases round a value
/// they keep fewer digits of.
fn rounded(at: DateTime, digits: u8) -> DateTime {
    if digits >= 9 {
        return at;
    }
    // Half up, carried into the next second (and day): jiff's own
    // rounding of a civil time wraps at midnight.
    let step = 10_i32.pow(9 - u32::from(digits));
    let kept = (at.subsec_nanosecond() + step / 2) / step * step;
    let whole = at.with().subsec_nanosecond(0).build().unwrap_or(at);
    if kept >= 1_000_000_000 {
        whole
            .checked_add(jiff::SignedDuration::from_secs(1))
            .unwrap_or(at)
    } else {
        whole.with().subsec_nanosecond(kept).build().unwrap_or(at)
    }
}

/// The two moments of a time the clocks of `zone` go back over.
fn fold(zone: &TimeZone, at: DateTime) -> Option<Fold> {
    let AmbiguousOffset::Fold { before, after } = zone.to_ambiguous_timestamp(at).offset() else {
        return None;
    };
    let named = |offset: Offset| {
        let name = offset
            .to_timestamp(at)
            .map(|moment| zone.to_offset_info(moment).abbreviation().to_owned())
            .unwrap_or_default();
        (offset, name)
    };
    Some(Fold {
        at,
        place: place(zone),
        earlier: named(before),
        later: named(after),
    })
}

/// The first and the last moment MySQL's `TIMESTAMP` holds.
fn mysql_timestamps() -> (Timestamp, Timestamp) {
    let first = Timestamp::from_second(1).unwrap_or(Timestamp::UNIX_EPOCH);
    let last = Timestamp::new(2_147_483_647, 999_999_000).unwrap_or(Timestamp::MAX);
    (first, last)
}

/// The first day PostgreSQL keeps: 24 November 4714 BC.
fn pg_first_day() -> Date {
    Date::new(-4713, 11, 24).unwrap_or(Date::MIN)
}

/// Whether the column's type holds `at` (the moment `instant` is, for a
/// zoned value).
fn within(column: &Column, at: DateTime, instant: Option<Timestamp>) -> Result<(), Refusal> {
    let date = at.date();
    let thousand = Date::new(1000, 1, 1).unwrap_or(Date::MIN);
    let refused = match (column.dialect, column.kind) {
        (Dialect::Postgres, Kind::Date) => (date < pg_first_day()).then_some(Range::PgDate),
        (Dialect::Postgres, Kind::Timestamp) => {
            (date < pg_first_day()).then_some(Range::PgTimestamp)
        }
        (Dialect::Postgres, Kind::Zoned) => {
            let first = pg_first_day()
                .to_zoned(TimeZone::UTC)
                .map(|zoned| zoned.timestamp())
                .unwrap_or(Timestamp::MIN);
            let instant = instant.unwrap_or_else(|| {
                at.to_zoned(TimeZone::UTC)
                    .map_or(Timestamp::MIN, |zoned| zoned.timestamp())
            });
            (instant < first).then_some(Range::PgTimestamp)
        }
        (Dialect::MySql, Kind::Date) => (date < thousand).then_some(Range::MySqlDate),
        (Dialect::MySql, Kind::Timestamp) => (date < thousand).then_some(Range::MySqlDatetime),
        (Dialect::MySql, Kind::Zoned) => {
            // Where the session's zone is not known, as UTC: the server's
            // own limits are within a day of it.
            let instant = instant.unwrap_or_else(|| {
                at.to_zoned(TimeZone::UTC)
                    .map_or(Timestamp::MIN, |zoned| zoned.timestamp())
            });
            let (first, last) = mysql_timestamps();
            (instant < first || instant > last).then_some(Range::MySqlTimestamp)
        }
        // SQLite's dates run from 0000 to 9999, and the years read do.
        (Dialect::Sqlite, _) | (Dialect::Postgres | Dialect::MySql, Kind::Time { .. }) => None,
    };
    refused.map_or(Ok(()), |range| Err(Refusal::Range(range)))
}

/// The value `parts` are for `column`, and what is sent for it.
fn resolve(parts: Parts, column: &Column, clock: &Clock) -> Result<Reading, Refusal> {
    let here = column.zone.as_ref().unwrap_or(&clock.zone);
    let now = matches!(parts.day, Some(Day::Now));
    let date = match parts.day {
        Some(Day::Numbers { year, month, day }) => {
            Some(checked_day(column, year, month, day, parts.bc)?)
        }
        Some(Day::Known(date)) => Some(date),
        Some(Day::Now) => Some(clock.today_in(here)),
        None => None,
    };
    let (time, typed_digits) = match (parts.time, now) {
        (Some((time, digits)), _) => (Some(time), digits),
        (None, true) => (Some(clock.now_in(here).time()), 0),
        (None, false) => (None, 0),
    };
    let digits = typed_digits.min(column.digits);
    let mut reading = Reading {
        typed: String::new(),
        moment: Moment::Infinity,
        digits,
        literal: String::new(),
        relative: parts.relative,
        days: None,
        zone_name: None,
        fold: None,
    };
    match column.kind {
        Kind::Date => {
            if parts.time.is_some() {
                return Err(Refusal::DayOnly);
            }
            if parts.offset.is_some() {
                return Err(Refusal::NoZone);
            }
            let date = date.ok_or(UNREAD)?;
            within(column, date.to_datetime(Time::midnight()), None)?;
            reading.moment = Moment::At {
                date: Some(date),
                time: None,
                offset: None,
            };
            reading.literal = match column.stored {
                Stored::Seconds | Stored::Millis | Stored::Days | Stored::Compact => {
                    number_of(date.to_datetime(Time::midnight()), None, column.stored)
                }
                Stored::Fresh | Stored::Text { .. } | Stored::Unknown => {
                    era(written_date(date, column.dialect))
                }
            };
        }
        Kind::Time { offset: keeps } => {
            if date.is_some() && !now {
                return Err(Refusal::TimeOnly);
            }
            if parts.offset.is_some() && !keeps {
                return Err(Refusal::NoZone);
            }
            let postgres = match column.dialect {
                Dialect::Postgres => true,
                Dialect::MySql | Dialect::Sqlite => false,
            };
            if let Some(offset) = parts.offset
                && postgres
                && offset.seconds().abs() > PG_OFFSETS
            {
                return Err(Refusal::Range(Range::PgOffset));
            }
            let time = time.ok_or(UNREAD)?;
            let day = Date::default();
            let at = rounded(day.to_datetime(time), digits);
            let digits = match column.stored {
                Stored::Seconds | Stored::Compact => 0,
                Stored::Millis | Stored::Days => digits.min(3),
                Stored::Fresh | Stored::Text { .. } | Stored::Unknown => digits,
            };
            reading.digits = digits;
            if at.date() != day {
                // Rounded up past the day's last moment: its end, which
                // MySQL keeps as a span of a day.
                reading.moment = match column.dialect {
                    Dialect::MySql => Moment::Span("24:00:00".to_owned()),
                    Dialect::Postgres | Dialect::Sqlite => Moment::EndOfDay,
                };
                reading.literal = "24:00:00".to_owned();
            } else {
                let time = at.time();
                reading.moment = Moment::At {
                    date: None,
                    time: Some(time),
                    offset: parts.offset,
                };
                reading.literal = match column.stored {
                    // Seconds of the day; a Julian day of 1 January 2000,
                    // the day SQLite's `julianday('09:30')` gives a time.
                    Stored::Seconds | Stored::Millis | Stored::Days => {
                        let year = match column.stored {
                            Stored::Days => 2000,
                            _ => 1970,
                        };
                        let epoch = Date::new(year, 1, 1).unwrap_or_default();
                        number_of(epoch.to_datetime(time), None, column.stored)
                    }
                    Stored::Fresh | Stored::Text { .. } | Stored::Compact | Stored::Unknown => {
                        written_time(time, digits)
                    }
                };
            }
            if let Some(offset) = parts.offset {
                reading.literal.push_str(&written_offset(offset));
            }
        }
        Kind::Timestamp => {
            // SQLite keeps the text it is given, an offset too.
            let keeps_any = match column.dialect {
                Dialect::Sqlite => true,
                Dialect::Postgres | Dialect::MySql => false,
            };
            if parts.offset.is_some() && !keeps_any {
                return Err(Refusal::NoZone);
            }
            let date = date.unwrap_or_else(|| clock.today_in(&clock.zone));
            let digits = match column.stored {
                Stored::Seconds | Stored::Compact => 0,
                Stored::Millis | Stored::Days => digits.min(3),
                Stored::Fresh | Stored::Text { .. } | Stored::Unknown => digits,
            };
            reading.digits = digits;
            let at = rounded(date.to_datetime(time.unwrap_or(Time::midnight())), digits);
            within(column, at, None)?;
            reading.moment = Moment::At {
                date: Some(at.date()),
                time: Some(at.time()),
                offset: parts.offset,
            };
            let separator = match column.stored {
                Stored::Text { separator } => separator,
                Stored::Fresh
                | Stored::Seconds
                | Stored::Millis
                | Stored::Days
                | Stored::Compact
                | Stored::Unknown => ' ',
            };
            reading.literal = match column.stored {
                Stored::Seconds | Stored::Millis | Stored::Days => {
                    number_of(at, parts.offset, column.stored)
                }
                Stored::Fresh | Stored::Text { .. } | Stored::Compact | Stored::Unknown => {
                    let (day, bc) = written_date(at.date(), column.dialect);
                    let mut written =
                        format!("{day}{separator}{}", written_time(at.time(), digits));
                    if let Some(offset) = parts.offset {
                        written.push_str(&written_offset(offset));
                    }
                    era((written, bc))
                }
            };
        }
        Kind::Zoned => zoned(&mut reading, column, clock, (date, time, parts.offset))?,
    }
    Ok(reading)
}

/// `written` with ` BC` after it where its year is before 1 AD.
fn era((written, bc): (String, bool)) -> String {
    if bc { format!("{written} BC") } else { written }
}

/// The furthest offset PostgreSQL's `timetz` keeps, in seconds: 15:59:59.
const PG_OFFSETS: i32 = 15 * 3_600 + 59 * 60 + 59;

/// The number SQLite keeps for `at` (read as UTC where no offset was
/// typed): its seconds since 1970, or its Julian day.
fn number_of(at: DateTime, offset: Option<Offset>, stored: Stored) -> String {
    let Ok(moment) = offset.unwrap_or(Offset::UTC).to_timestamp(at) else {
        return String::new();
    };
    match stored {
        Stored::Days => (moment.as_millisecond() as f64 / MILLIS_A_DAY + JULIAN_EPOCH).to_string(),
        Stored::Millis => moment.as_millisecond().to_string(),
        Stored::Compact => {
            let day = moment.to_zoned(TimeZone::UTC).date();
            format!("{:04}{:02}{:02}", day.year(), day.month(), day.day())
        }
        Stored::Seconds | Stored::Fresh | Stored::Text { .. } | Stored::Unknown => {
            moment.as_second().to_string()
        }
    }
}

/// A zoned value: the day and time in the column's zone, the offset the
/// zone has then, and the moment's literal.
fn zoned(
    reading: &mut Reading,
    column: &Column,
    clock: &Clock,
    (date, time, typed): (Option<Date>, Option<Time>, Option<Offset>),
) -> Result<(), Refusal> {
    let Some(zone) = column.zone.as_ref() else {
        // A zone not known: the value as the server's clock writes it. What
        // today is there is the server's to say.
        if typed.is_some() || reading.relative || date.is_none() {
            return Err(Refusal::ZoneUnknown);
        }
        let date = date.unwrap_or_else(|| clock.today_in(&clock.zone));
        let at = rounded(
            date.to_datetime(time.unwrap_or(Time::midnight())),
            reading.digits,
        );
        within(column, at, None)?;
        reading.moment = Moment::At {
            date: Some(at.date()),
            time: Some(at.time()),
            offset: None,
        };
        let (day, bc) = written_date(at.date(), column.dialect);
        reading.literal = era((
            format!("{day} {}", written_time(at.time(), reading.digits)),
            bc,
        ));
        return Ok(());
    };
    let date = date.unwrap_or_else(|| clock.today_in(zone));
    let typed_at = rounded(
        date.to_datetime(time.unwrap_or(Time::midnight())),
        reading.digits,
    );
    let instant = match typed {
        Some(offset) => offset.to_timestamp(typed_at).map_err(|_| Refusal::Years)?,
        None => match zone.to_ambiguous_timestamp(typed_at).offset() {
            AmbiguousOffset::Unambiguous { offset }
            | AmbiguousOffset::Fold { before: offset, .. } => {
                offset.to_timestamp(typed_at).map_err(|_| Refusal::Years)?
            }
            AmbiguousOffset::Gap { before, .. } => {
                let moment = before.to_timestamp(typed_at).map_err(|_| Refusal::Years)?;
                return Err(Refusal::Gap {
                    at: typed_at,
                    place: place(zone),
                    offered: moment.to_zoned(zone.clone()).datetime(),
                });
            }
        },
    };
    let shown = instant.to_zoned(zone.clone());
    let at = shown.datetime();
    if at.year() > 9999 {
        return Err(Refusal::Years);
    }
    within(column, at, Some(instant))?;
    let offset = shown.offset();
    reading.moment = Moment::At {
        date: Some(at.date()),
        time: Some(at.time()),
        offset: Some(offset),
    };
    reading.zone_name = Some(zone.to_offset_info(instant).abbreviation().to_owned());
    // A MySQL literal has no offset to choose one of the two moments by:
    // the server takes the one it takes, and nothing is offered.
    reading.fold = match column.dialect {
        Dialect::Postgres | Dialect::Sqlite => fold(zone, at),
        Dialect::MySql => None,
    };
    let (day, bc) = written_date(at.date(), column.dialect);
    let time = written_time(at.time(), reading.digits);
    reading.literal = match column.dialect {
        // The server reads a MySQL value in the session's zone, which is
        // this one: it is sent as that zone's clock writes it.
        Dialect::MySql => format!("{day} {time}"),
        Dialect::Postgres | Dialect::Sqlite => {
            era((format!("{day} {time}{}", written_offset(offset)), bc))
        }
    };
    Ok(())
}

/// The text a value read is rewritten to where a choice of the editor's
/// changes it (the time offered across a change of the clocks, one of the
/// two a fold has): its day and time as the zone's clock shows them, and
/// the offset where one is chosen.
pub fn rewritten(at: DateTime, offset: Option<Offset>, column: &Column) -> String {
    let (day, bc) = written_date(at.date(), column.dialect);
    let mut time = format!("{:02}:{:02}", at.hour(), at.minute());
    if at.second() != 0 || at.subsec_nanosecond() != 0 {
        time = written_time(at.time(), fraction_digits(at.time()));
    }
    let mut written = format!("{day} {time}");
    if let Some(offset) = offset {
        written.push_str(&written_offset(offset));
    }
    era((written, bc))
}

/// The digits a time's fraction needs, its trailing zeros left off.
fn fraction_digits(time: Time) -> u8 {
    let nanos = time.subsec_nanosecond();
    if nanos == 0 {
        return 0;
    }
    let zeros = (0..9)
        .take_while(|at| nanos % 10_i32.pow(at + 1) == 0)
        .count();
    9 - zeros as u8
}

/// The text an editor opens on for a SQLite value kept as a number
/// (seconds since 1970, or a Julian day): the day, its time, or both, as
/// it is in UTC. `None` for any other.
pub fn opened(value: &Value, column: &Column) -> Option<String> {
    let number = match (value, column.stored) {
        (Value::Int(number), Stored::Seconds | Stored::Millis | Stored::Compact) => {
            number.to_string()
        }
        (Value::Float(days), Stored::Days) => days.to_string(),
        (Value::Int(days), Stored::Days) => days.to_string(),
        _ => return None,
    };
    let at = numbered(&number, column.stored)?
        .ok()?
        .to_zoned(TimeZone::UTC)
        .datetime();
    let (day, _) = written_date(at.date(), column.dialect);
    let time = written_time(at.time(), fraction_digits(at.time()));
    Some(match column.kind {
        Kind::Date => day,
        Kind::Time { .. } => time,
        Kind::Timestamp | Kind::Zoned => format!("{day} {time}"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::civil::date;

    fn new_york() -> TimeZone {
        TimeZone::get("America/New_York").expect("the zone")
    }

    /// Saturday 24 October 2026, 10:00 in New York, as the boards have it.
    fn clock() -> Clock {
        let now = date(2026, 10, 24)
            .at(10, 0, 0, 0)
            .to_zoned(new_york())
            .unwrap()
            .timestamp();
        Clock {
            now,
            zone: new_york(),
            order: Order::DayMonth,
        }
    }

    fn column(dialect: Dialect, type_name: &str) -> Column {
        Column::of(dialect, type_name, Some(&new_york()), []).expect("a column of dates")
    }

    fn literal(dialect: Dialect, type_name: &str, typed: &str) -> String {
        match read(typed, &column(dialect, type_name), &clock()) {
            Ok(reading) => reading.literal,
            Err(refusal) => panic!("{typed:?} refused: {refusal:?}"),
        }
    }

    fn refusal(dialect: Dialect, type_name: &str, typed: &str) -> Refusal {
        read(typed, &column(dialect, type_name), &clock()).expect_err(typed)
    }

    #[test]
    fn each_engine_names_its_kinds() {
        use Dialect::{MySql, Postgres, Sqlite};
        let zoned = Some(Kind::Zoned);
        let stamp = Some(Kind::Timestamp);
        let day = Some(Kind::Date);
        let time = Some(Kind::Time { offset: false });
        for (dialect, name, expected) in [
            (Postgres, "date", day),
            (Postgres, "timestamp without time zone", stamp),
            (Postgres, "timestamp(3) without time zone", stamp),
            (Postgres, "timestamp", stamp),
            (Postgres, "timestamp with time zone", zoned),
            (Postgres, "timestamptz", zoned),
            (Postgres, "time without time zone", time),
            (
                Postgres,
                "time(3) with time zone",
                Some(Kind::Time { offset: true }),
            ),
            (Postgres, "timetz", Some(Kind::Time { offset: true })),
            (Postgres, "interval", None),
            (Postgres, "date[]", None),
            (Postgres, "daterange", None),
            (Postgres, "text", None),
            (MySql, "date", day),
            (MySql, "datetime", stamp),
            (MySql, "datetime(6)", stamp),
            (MySql, "timestamp", zoned),
            (MySql, "timestamp(3)", zoned),
            (MySql, "time", time),
            (MySql, "year", None),
            (MySql, "varchar(20)", None),
            (Sqlite, "DATE", day),
            (Sqlite, "DATETIME", stamp),
            (Sqlite, "TIMESTAMP", stamp),
            (Sqlite, "timestamptz", stamp),
            (Sqlite, "DATE_TIME", stamp),
            (Sqlite, "TIME", time),
            (Sqlite, "TEXT", None),
            (Sqlite, "", None),
        ] {
            assert_eq!(kind(dialect, name), expected, "{dialect:?} {name}");
        }
    }

    #[test]
    fn each_written_form_reads_as_its_day() {
        let pg = Dialect::Postgres;
        for typed in [
            "2026-01-15",
            "2026-1-15",
            "15 Jan 2026",
            "15 january 2026",
            "Jan 15 2026",
            "January 15, 2026",
            "jan 15th 2026",
            "15/01/2026",
            "15.01.2026",
            "2026/01/15",
            " 2026-01-15 ",
        ] {
            assert_eq!(literal(pg, "date", typed), "2026-01-15", "{typed}");
        }
        // A month's name with no year: this year's.
        assert_eq!(literal(pg, "date", "15 jan"), "2026-01-15");
    }

    #[test]
    fn the_locale_orders_a_day_written_in_numbers() {
        let column = column(Dialect::Postgres, "date");
        let in_order = |order: Order| Clock { order, ..clock() };
        let read_in = |order| {
            read("02/03/2026", &column, &in_order(order))
                .unwrap()
                .literal
        };
        assert_eq!(read_in(Order::DayMonth), "2026-03-02");
        assert_eq!(read_in(Order::MonthDay), "2026-02-03");
        assert_eq!(Order::of_region(Some("us")), Order::MonthDay);
        assert_eq!(Order::of_region(Some("ca")), Order::YearMonth);
        assert_eq!(Order::of_region(Some("gb")), Order::DayMonth);
        assert_eq!(Order::of_region(Some("jp")), Order::YearMonth);
        assert_eq!(Order::of_region(None), Order::DayMonth);
        // `LC_ALL` decides where it is set, a region or none; then
        // `LC_TIME`; then the system's languages.
        let languages = ["ja-JP".to_owned(), "en-US".to_owned()];
        let of = |all, time| Order::of_locale(all, time, &languages);
        assert_eq!(of(Some("C"), Some("en_US.UTF-8")), Order::DayMonth);
        assert_eq!(
            of(Some("en_US.UTF-8"), Some("de_DE.UTF-8")),
            Order::MonthDay
        );
        assert_eq!(of(Some(""), Some("en_US.UTF-8")), Order::MonthDay);
        assert_eq!(of(None, Some("C")), Order::DayMonth);
        assert_eq!(of(None, None), Order::YearMonth);
        assert_eq!(Order::of_locale(None, None, &[]), Order::DayMonth);
    }

    #[test]
    fn each_written_form_reads_as_its_time() {
        let pg = Dialect::Postgres;
        for (typed, expected) in [
            ("9:30", "09:30:00"),
            ("9.30", "09:30:00"),
            ("9am", "09:00:00"),
            ("9 am", "09:00:00"),
            ("9:30pm", "21:30:00"),
            ("12am", "00:00:00"),
            ("12pm", "12:00:00"),
            ("21:30", "21:30:00"),
            ("21:30:15", "21:30:15"),
            ("21:30:15.25", "21:30:15.25"),
        ] {
            assert_eq!(literal(pg, "time", typed), expected, "{typed}");
        }
    }

    #[test]
    fn a_day_and_a_time_read_together_in_every_form() {
        let pg = Dialect::Postgres;
        let stamp = "timestamp without time zone";
        for typed in [
            "2026-01-15 09:30",
            "2026-01-15T09:30",
            "2026-01-15t09:30:00",
            "15 Jan 2026 9:30",
            "Jan 15 2026, 9:30am",
            "15/01/2026 9.30",
        ] {
            assert_eq!(literal(pg, stamp, typed), "2026-01-15 09:30:00", "{typed}");
        }
        assert_eq!(
            literal(pg, stamp, "2026-01-15 09:30:12.250"),
            "2026-01-15 09:30:12.250"
        );
        // A day alone is its midnight; a time alone today's.
        assert_eq!(literal(pg, stamp, "2026-01-15"), "2026-01-15 00:00:00");
        assert_eq!(literal(pg, stamp, "9:30"), "2026-10-24 09:30:00");
    }

    #[test]
    fn words_count_from_today_by_the_clock() {
        let pg = Dialect::Postgres;
        // Saturday 24 October 2026.
        for (typed, expected) in [
            ("today", "2026-10-24"),
            ("Tomorrow", "2026-10-25"),
            ("yesterday", "2026-10-23"),
            ("now", "2026-10-24"),
            ("+7d", "2026-10-31"),
            ("-2w", "2026-10-10"),
            ("+1m", "2026-11-24"),
            ("+1y", "2027-10-24"),
            ("next fri", "2026-10-30"),
            ("fri", "2026-10-30"),
            ("next saturday", "2026-10-31"),
            ("last mon", "2026-10-19"),
        ] {
            assert_eq!(literal(pg, "date", typed), expected, "{typed}");
        }
        let stamp = "timestamp without time zone";
        assert_eq!(literal(pg, stamp, "tomorrow 9am"), "2026-10-25 09:00:00");
        assert_eq!(
            literal(pg, stamp, "tomorrow at 9:30"),
            "2026-10-25 09:30:00"
        );
        assert_eq!(literal(pg, stamp, "now"), "2026-10-24 10:00:00");
        assert_eq!(literal(pg, "time", "now"), "10:00:00");
        let reading = read("+7d", &column(pg, "date"), &clock()).unwrap();
        assert!(reading.relative);
        assert_eq!(reading.days, Some(7));
        assert_eq!(reading.typed, "+7d");
        let reading = read("yesterday", &column(pg, "date"), &clock()).unwrap();
        assert_eq!(reading.days, Some(-1));
        let reading = read("2026-10-30", &column(pg, "date"), &clock()).unwrap();
        assert!(!reading.relative);
    }

    #[test]
    fn a_month_moved_past_its_end_is_held_to_its_last_day() {
        let clock = Clock {
            now: date(2026, 1, 31)
                .at(12, 0, 0, 0)
                .to_zoned(new_york())
                .unwrap()
                .timestamp(),
            ..clock()
        };
        let column = column(Dialect::Postgres, "date");
        assert_eq!(read("+1m", &column, &clock).unwrap().literal, "2026-02-28");
    }

    #[test]
    fn a_day_or_a_time_that_is_none_is_refused_with_why() {
        let pg = Dialect::Postgres;
        assert_eq!(
            refusal(pg, "date", "2026-02-30"),
            Refusal::NoDay { day: 30, month: 2 }
        );
        assert_eq!(refusal(pg, "date", "2026-13-01"), Refusal::NoMonth(13));
        assert_eq!(
            refusal(pg, "time", "25:00"),
            Refusal::NoTime("25:00".into())
        );
        assert_eq!(refusal(pg, "time", "9:61"), Refusal::NoTime("9:61".into()));
        assert_eq!(refusal(pg, "time", "13pm"), Refusal::NoTime("13pm".into()));
        assert_eq!(refusal(pg, "date", "soon"), UNREAD);
        assert_eq!(refusal(pg, "date", ""), UNREAD);
        assert_eq!(refusal(pg, "date", "2026-01-15 9:30"), Refusal::DayOnly);
        assert_eq!(refusal(pg, "time", "2026-01-15 9:30"), Refusal::TimeOnly);
        assert_eq!(refusal(pg, "date", "20260-01-15"), Refusal::Years);
        // 2028 has a 29 February; 2026 has none.
        assert_eq!(literal(pg, "date", "2028-02-29"), "2028-02-29");
    }

    #[test]
    fn a_moment_is_sent_in_the_zone_with_its_offset() {
        let pg = Dialect::Postgres;
        let tz = "timestamp with time zone";
        assert_eq!(
            literal(pg, tz, "2026-01-15 09:30"),
            "2026-01-15 09:30:00-05"
        );
        assert_eq!(
            literal(pg, tz, "2026-07-15 09:30"),
            "2026-07-15 09:30:00-04"
        );
        // Typed with another offset: the same moment, in the zone.
        assert_eq!(
            literal(pg, tz, "2026-01-15 09:30+01"),
            "2026-01-15 03:30:00-05"
        );
        assert_eq!(
            literal(pg, tz, "2026-01-15T14:30:00Z"),
            "2026-01-15 09:30:00-05"
        );
        // As the server writes one, with its fraction: what the column
        // keeps.
        assert_eq!(
            literal(pg, tz, "2026-01-12 09:14:03.482915+00"),
            "2026-01-12 04:14:03.482915-05"
        );
        let reading = read("2026-01-15 09:30", &column(pg, tz), &clock()).unwrap();
        assert_eq!(reading.zone_name.as_deref(), Some("EST"));
        assert_eq!(reading.fold, None);
    }

    #[test]
    fn a_time_the_clocks_skip_is_refused_and_the_time_after_offered() {
        let column = column(Dialect::Postgres, "timestamptz");
        let refused = read("2026-03-08 02:30", &column, &clock()).unwrap_err();
        assert_eq!(
            refused,
            Refusal::Gap {
                at: date(2026, 3, 8).at(2, 30, 0, 0),
                place: "New York".into(),
                offered: date(2026, 3, 8).at(3, 30, 0, 0),
            }
        );
        let offered = rewritten(date(2026, 3, 8).at(3, 30, 0, 0), None, &column);
        assert_eq!(offered, "2026-03-08 03:30");
        assert_eq!(
            read(&offered, &column, &clock()).unwrap().literal,
            "2026-03-08 03:30:00-04"
        );
    }

    #[test]
    fn a_time_the_clocks_go_back_over_takes_summer_time_first_or_the_one_chosen() {
        let column = column(Dialect::Postgres, "timestamptz");
        let first = read("2026-11-01 01:30", &column, &clock()).unwrap();
        assert_eq!(first.literal, "2026-11-01 01:30:00-04");
        let fold = first.fold.expect("a fold");
        assert_eq!(fold.earlier.1, "EDT");
        assert_eq!(fold.later.1, "EST");
        assert_eq!(fold.place, "New York");
        let chosen = rewritten(fold.at, Some(fold.later.0), &column);
        assert_eq!(chosen, "2026-11-01 01:30-05");
        let second = read(&chosen, &column, &clock()).unwrap();
        assert_eq!(second.literal, "2026-11-01 01:30:00-05");
        assert!(second.fold.is_some(), "the choice is still offered");
    }

    #[test]
    fn a_timestamp_without_a_zone_is_stored_as_typed_in_any_zone() {
        for zone in [
            new_york(),
            TimeZone::UTC,
            TimeZone::get("Asia/Kolkata").unwrap(),
        ] {
            let clock = Clock {
                zone: zone.clone(),
                ..clock()
            };
            let column = Column::of(Dialect::Postgres, "timestamp", Some(&zone), []).unwrap();
            let reading = read("2026-03-08 02:30", &column, &clock).unwrap();
            assert_eq!(reading.literal, "2026-03-08 02:30:00");
            assert_eq!(reading.fold, None);
        }
        assert_eq!(
            refusal(Dialect::Postgres, "timestamp", "2026-03-08 02:30-05"),
            Refusal::NoZone
        );
        assert_eq!(
            literal(Dialect::MySql, "datetime", "2026-03-08 02:30"),
            "2026-03-08 02:30:00"
        );
    }

    #[test]
    fn each_type_holds_its_own_range() {
        let my = Dialect::MySql;
        assert_eq!(
            refusal(my, "timestamp", "2040-05-01 08:00"),
            Refusal::Range(Range::MySqlTimestamp)
        );
        assert_eq!(
            refusal(my, "timestamp", "1969-12-31 18:00"),
            Refusal::Range(Range::MySqlTimestamp)
        );
        // 2038-01-19 03:14:07 UTC is 22:14:07 the day before in New York.
        assert_eq!(
            literal(my, "timestamp", "2038-01-18 22:14:07"),
            "2038-01-18 22:14:07"
        );
        assert_eq!(
            refusal(my, "timestamp", "2038-01-18 22:14:08"),
            Refusal::Range(Range::MySqlTimestamp)
        );
        assert_eq!(
            refusal(my, "date", "0999-12-31"),
            Refusal::Range(Range::MySqlDate)
        );
        assert_eq!(literal(my, "date", "1000-01-01"), "1000-01-01");
        assert_eq!(literal(my, "date", "9999-12-31"), "9999-12-31");
        assert_eq!(
            refusal(my, "datetime", "0999-12-31 23:59"),
            Refusal::Range(Range::MySqlDatetime)
        );
        assert_eq!(refusal(my, "date", "0000-00-00"), Refusal::Zero);
        assert_eq!(
            refusal(my, "datetime", "0000-00-00 00:00:00"),
            Refusal::Zero
        );
        let pg = Dialect::Postgres;
        assert_eq!(literal(pg, "date", "4714-11-24 BC"), "4714-11-24 BC");
        assert_eq!(
            refusal(pg, "date", "4714-11-23 BC"),
            Refusal::Range(Range::PgDate)
        );
        assert_eq!(refusal(pg, "date", "0000-01-01"), Refusal::YearZero);
        assert_eq!(literal(Dialect::Sqlite, "DATE", "0000-01-01"), "0000-01-01");
    }

    #[test]
    fn a_mysql_timestamp_is_sent_as_the_sessions_clock_writes_it() {
        let my = Dialect::MySql;
        let berlin = TimeZone::get("Europe/Berlin").unwrap();
        let column = Column::of(my, "timestamp", Some(&berlin), []).unwrap();
        // Berlin's clocks go back that night: the literal has no offset to
        // say which 02:30, and none is asked.
        let reading = read("2026-10-25 02:30", &column, &clock()).unwrap();
        assert_eq!(reading.literal, "2026-10-25 02:30:00");
        assert_eq!(reading.fold, None);
        // They go forward in March, over 02:30.
        let refused = read("2026-03-29 02:30", &column, &clock()).unwrap_err();
        assert!(matches!(refused, Refusal::Gap { .. }), "{refused:?}");
        // The limits are UTC's: 03:14:07 UTC is 04:14:07 in Berlin.
        assert!(read("2038-01-19 04:14:07", &column, &clock()).is_ok());
        assert_eq!(
            read("2038-01-19 04:14:08", &column, &clock()).unwrap_err(),
            Refusal::Range(Range::MySqlTimestamp)
        );
        // A zone the app cannot name: the server's clock, no change found.
        let unnamed = Column::of(my, "timestamp", None, []).unwrap();
        let reading = read("2026-03-29 02:30", &unnamed, &clock()).unwrap();
        assert_eq!(reading.literal, "2026-03-29 02:30:00");
    }

    #[test]
    fn infinity_and_days_before_christ_go_back_as_they_came() {
        let pg = Dialect::Postgres;
        for typed in ["infinity", "-infinity"] {
            for type_name in ["date", "timestamp", "timestamptz"] {
                assert_eq!(literal(pg, type_name, typed), typed);
            }
        }
        assert_eq!(literal(pg, "date", "+Infinity"), "infinity");
        assert_eq!(
            refusal(Dialect::MySql, "date", "infinity"),
            Refusal::NotRead(Kind::Date)
        );
        assert_eq!(literal(pg, "date", "0044-03-15 BC"), "0044-03-15 BC");
        assert_eq!(
            literal(pg, "timestamp", "0044-03-15 12:00:00 BC"),
            "0044-03-15 12:00:00 BC"
        );
        // As the server writes a moment before Christ: its offset, then
        // the era. New York kept its own clock then.
        assert_eq!(
            literal(pg, "timestamptz", "0044-03-15 12:00:00+00 BC"),
            "0044-03-15 07:03:58-04:56:02 BC"
        );
        let reading = read("0044-03-15 BC", &column(pg, "date"), &clock()).unwrap();
        assert_eq!(
            reading.moment,
            Moment::At {
                date: Some(date(-43, 3, 15)),
                time: None,
                offset: None
            }
        );
        assert_eq!(
            refusal(Dialect::Sqlite, "DATE", "0044-03-15 BC"),
            Refusal::NotRead(Kind::Date)
        );
    }

    #[test]
    fn sqlite_writes_the_form_its_column_already_keeps() {
        let lite = Dialect::Sqlite;
        let of = |type_name: &str, values: &[Value]| {
            Column::of(lite, type_name, None, values.iter()).unwrap()
        };
        let text = [Value::Null, Value::Text("2026-10-01".into())];
        let seconds = [Value::Int(1_790_000_000)];
        let stamps = [Value::Text("2026-10-01T08:00:00".into())];
        assert_eq!(Stored::of(&[]), Stored::Fresh);
        assert_eq!(Stored::of(&text), Stored::Text { separator: ' ' });
        assert_eq!(Stored::of(&seconds), Stored::Seconds);
        assert_eq!(Stored::of(&stamps), Stored::Text { separator: 'T' });
        let read_as = |column: &Column, typed: &str| read(typed, column, &clock()).unwrap().literal;
        assert_eq!(read_as(&of("DATE", &text), "24 Oct 2026"), "2026-10-24");
        assert_eq!(read_as(&of("DATE", &[]), "24 Oct 2026"), "2026-10-24");
        assert_eq!(read_as(&of("DATE", &seconds), "24 Oct 2026"), "1792800000");
        assert_eq!(
            read_as(&of("DATETIME", &seconds), "2026-10-24 08:00:00.5"),
            "1792828801"
        );
        assert_eq!(
            read_as(&of("DATETIME", &stamps), "2026-10-24 08:00"),
            "2026-10-24T08:00:00"
        );
        assert_eq!(
            read_as(&of("DATETIME", &[]), "2026-10-24 08:00"),
            "2026-10-24 08:00:00"
        );
        // A SQLite column keeps what it is given: an offset too.
        assert_eq!(
            read_as(&of("DATETIME", &[]), "2026-10-24 08:00+02"),
            "2026-10-24 08:00:00+02"
        );
        // Seconds read back, and open as their day.
        let column = of("DATE", &seconds);
        assert_eq!(read_as(&column, "1792800000"), "1792800000");
        assert_eq!(
            opened(&Value::Int(1_792_800_000), &column).as_deref(),
            Some("2026-10-24")
        );
        assert_eq!(opened(&Value::Text("2026-10-24".into()), &column), None);
    }

    #[test]
    fn a_fraction_is_kept_to_the_digits_the_column_keeps() {
        let my = Dialect::MySql;
        assert_eq!(
            literal(my, "datetime", "2026-01-15 09:30:12.6"),
            "2026-01-15 09:30:13"
        );
        assert_eq!(
            literal(my, "datetime(3)", "2026-01-15 09:30:12.123456"),
            "2026-01-15 09:30:12.123"
        );
        assert_eq!(
            literal(Dialect::Postgres, "time(0)", "09:30:59.7"),
            "09:31:00"
        );
        // Rounded up into the next day, and at the last there is not.
        assert_eq!(
            literal(my, "datetime", "2026-01-15 23:59:59.7"),
            "2026-01-16 00:00:00"
        );
        assert_eq!(
            literal(my, "datetime", "9999-12-31 23:59:59.7"),
            "9999-12-31 23:59:59"
        );
    }

    #[test]
    fn a_mysql_time_past_a_day_or_below_zero_is_a_span_of_time() {
        let my = Dialect::MySql;
        assert_eq!(literal(my, "time", "100:30:00"), "100:30:00");
        assert_eq!(literal(my, "time", "-12:00"), "-12:00:00");
        assert_eq!(literal(my, "time", "838:59:59"), "838:59:59");
        assert_eq!(
            refusal(my, "time", "839:00:00"),
            Refusal::Range(Range::MySqlTime)
        );
        assert_eq!(
            refusal(my, "time", "100:61:00"),
            Refusal::NoTime("100:61:00".into())
        );
        // A time of day is read as one; PostgreSQL's time keeps no span.
        assert_eq!(literal(my, "time", "9:30"), "09:30:00");
        assert_eq!(
            refusal(Dialect::Postgres, "time", "100:30:00"),
            Refusal::NotRead(Kind::Time { offset: false })
        );
    }

    #[test]
    fn a_time_with_an_offset_keeps_it_where_the_type_does() {
        let pg = Dialect::Postgres;
        assert_eq!(literal(pg, "timetz", "09:30-05"), "09:30:00-05");
        assert_eq!(literal(pg, "timetz", "09:30"), "09:30:00");
        assert_eq!(refusal(pg, "time", "09:30-05"), Refusal::NoZone);
    }

    #[test]
    fn a_count_past_every_day_is_refused_not_a_crash() {
        let pg = Dialect::Postgres;
        assert_eq!(refusal(pg, "date", "+20000y"), Refusal::Years);
        assert_eq!(refusal(pg, "date", "+99999999d"), Refusal::Years);
        assert_eq!(refusal(pg, "timestamp", "-9999999w 9am"), Refusal::Years);
        assert_eq!(refusal(pg, "date", "+9999999999999999999d"), UNREAD);
    }

    #[test]
    fn an_offset_is_read_only_where_its_minutes_and_its_range_are_ones() {
        let pg = Dialect::Postgres;
        assert_eq!(
            refusal(pg, "timestamptz", "2026-01-15 10:00+05:99"),
            Refusal::NotRead(Kind::Zoned)
        );
        assert_eq!(
            refusal(pg, "timestamptz", "2026-01-15 10:00+0599"),
            Refusal::NotRead(Kind::Zoned)
        );
        assert_eq!(
            refusal(pg, "timetz", "10:00+16"),
            Refusal::Range(Range::PgOffset)
        );
        assert_eq!(
            refusal(pg, "timetz", "10:00+25:00"),
            Refusal::Range(Range::PgOffset)
        );
        assert_eq!(literal(pg, "timetz", "10:00+15:59"), "10:00:00+15:59");
    }

    #[test]
    fn a_time_rounded_past_its_day_is_the_days_end() {
        let pg = Dialect::Postgres;
        assert_eq!(literal(pg, "time(0)", "23:59:59.7"), "24:00:00");
        assert_eq!(literal(pg, "time", "23:59:59.9999999"), "24:00:00");
        assert_eq!(literal(pg, "time", "24:00"), "24:00:00");
        assert_eq!(literal(pg, "time", "24:00:00.000"), "24:00:00");
        assert_eq!(literal(Dialect::Sqlite, "TIME", "24:00:00"), "24:00:00");
        assert_eq!(
            refusal(pg, "time", "24:00:01"),
            Refusal::NoTime("24:00:01".into())
        );
        let my = Dialect::MySql;
        assert_eq!(literal(my, "time", "23:59:59.7"), "24:00:00");
        // A span is rounded as a time of day is.
        assert_eq!(literal(my, "time", "100:30:00.9"), "100:30:01");
        assert_eq!(literal(my, "time", "12:00:00.9"), "12:00:01");
        assert_eq!(literal(my, "time(2)", "-12:00:00.125"), "-12:00:00.13");
        assert_eq!(
            refusal(my, "time", "838:59:59.5"),
            Refusal::Range(Range::MySqlTime)
        );
    }

    #[test]
    fn a_choice_keeps_the_fraction_of_the_time_it_moves() {
        let column = column(Dialect::Postgres, "timestamptz");
        let Err(Refusal::Gap { offered, .. }) = read("2026-03-08 02:30:15.5", &column, &clock())
        else {
            panic!("a gap");
        };
        let text = rewritten(offered, None, &column);
        assert_eq!(text, "2026-03-08 03:30:15.5");
        assert_eq!(
            read(&text, &column, &clock()).unwrap().literal,
            "2026-03-08 03:30:15.5-04"
        );
    }

    #[test]
    fn an_era_goes_only_after_a_year_that_is_written() {
        let pg = Dialect::Postgres;
        for typed in ["today bc", "15 jan bc", "+7d bc"] {
            assert_eq!(refusal(pg, "date", typed), UNREAD, "{typed}");
        }
        assert_eq!(
            refusal(pg, "timestamp", "9:30 bc"),
            Refusal::NotRead(Kind::Timestamp)
        );
        assert_eq!(literal(pg, "date", "15 jan 2026 bc"), "2026-01-15 BC");
    }

    #[test]
    fn a_mysql_session_whose_zone_is_not_known_takes_no_day_counted_from_today() {
        let unnamed = Column::of(Dialect::MySql, "timestamp", None, []).unwrap();
        for typed in ["now", "tomorrow 9am", "9:30", "+1d"] {
            assert_eq!(
                read(typed, &unnamed, &clock()),
                Err(Refusal::ZoneUnknown),
                "{typed}"
            );
        }
        assert!(read("2026-01-15 09:30", &unnamed, &clock()).is_ok());
    }

    #[test]
    fn sqlite_numbers_are_told_apart_by_every_value_of_the_page() {
        let of = |values: &[Value]| Stored::of(values);
        // A whole Julian day (a noon) first, a fraction after it.
        assert_eq!(
            of(&[Value::Int(2_461_337), Value::Float(2_461_337.5)]),
            Stored::Days
        );
        assert_eq!(of(&[Value::Int(2_461_337)]), Stored::Days);
        assert_eq!(of(&[Value::Int(1_792_800_000_000)]), Stored::Millis);
        assert_eq!(
            of(&[Value::Int(20_260_115), Value::Int(20_261_024)]),
            Stored::Compact
        );
        assert_eq!(
            of(&[Value::Int(1_792_800_000), Value::Int(0)]),
            Stored::Seconds
        );
        // Seconds beside milliseconds, a real beside seconds: no one form.
        assert_eq!(
            of(&[Value::Int(1_792_800_000), Value::Int(1_792_800_000_000)]),
            Stored::Unknown
        );
        assert_eq!(
            of(&[Value::Float(2_461_337.5), Value::Int(1_792_800_000)]),
            Stored::Unknown
        );
        assert_eq!(
            of(&[Value::Null, Value::Text("2026-10-24".into()), Value::Int(1)]),
            Stored::Text { separator: ' ' }
        );
        let lite = Dialect::Sqlite;
        let column = |type_name: &str, values: &[Value]| {
            Column::of(lite, type_name, None, values.iter()).unwrap()
        };
        let read_as = |column: &Column, typed: &str| read(typed, column, &clock()).unwrap().literal;
        // A whole Julian day is a noon: 2461337 is noon on 23 October.
        let noon = column("DATE", &[Value::Int(2_461_337)]);
        assert_eq!(
            opened(&Value::Int(2_461_337), &noon).as_deref(),
            Some("2026-10-23")
        );
        assert_eq!(read_as(&noon, "25 Oct 2026"), "2461338.5");
        let millis = column("DATETIME", &[Value::Int(1_792_800_000_000)]);
        assert_eq!(
            opened(&Value::Int(1_792_800_000_000), &millis).as_deref(),
            Some("2026-10-24 00:00:00")
        );
        assert_eq!(read_as(&millis, "2026-10-24 08:00:00.25"), "1792828800250");
        let compact = column("DATE", &[Value::Int(20_260_115)]);
        assert_eq!(
            opened(&Value::Int(20_260_115), &compact).as_deref(),
            Some("2026-01-15")
        );
        assert_eq!(read_as(&compact, "25 Oct 2026"), "20261025");
        // A number such as 20260115 is a day's, no time's: in a timestamp it
        // is of no form known, and a new value is text.
        assert_eq!(
            column("DATETIME", &[Value::Int(20_260_115)]).stored,
            Stored::Unknown
        );
        let unknown = column(
            "DATE",
            &[Value::Int(1_792_800_000), Value::Int(1_792_800_000_000)],
        );
        assert_eq!(read_as(&unknown, "25 Oct 2026"), "2026-10-25");
    }

    #[test]
    fn a_number_outside_its_columns_form_is_read_as_typed_text() {
        let lite = Dialect::Sqlite;
        let column = |type_name: &str, values: &[Value]| {
            Column::of(lite, type_name, None, values.iter()).unwrap()
        };
        let read_as = |column: &Column, typed: &str| read(typed, column, &clock()).unwrap().literal;
        // A time of Julian days, as `julianday('09:30')` keeps one: `9.30`
        // is 09:30, not the Julian day 9.3.
        let times = column("TIME", &[Value::Float(2_451_544.895_833_333)]);
        assert_eq!(times.stored, Stored::Days);
        assert_eq!(read_as(&times, "9.30"), read_as(&times, "09:30"));
        let stamps = column("DATETIME", &[Value::Float(2_461_337.5)]);
        assert_eq!(
            read_as(&stamps, "9.30"),
            read_as(&stamps, "2026-10-24 09:30")
        );
        // Nor is a small number a value of milliseconds.
        let millis = column("DATETIME", &[Value::Int(1_792_800_000_000)]);
        assert_eq!(
            read("2026", &millis, &clock()),
            Err(Refusal::NotRead(Kind::Timestamp))
        );
        // The least whole number there is, typed or held, is refused, and
        // nothing overflows on the way.
        assert_eq!(
            read(&i64::MIN.to_string(), &millis, &clock()),
            Err(Refusal::Years)
        );
        assert_eq!(Stored::of(&[Value::Int(i64::MIN)]), Stored::Unknown);
    }

    #[test]
    fn sqlite_times_and_julian_days_follow_their_columns() {
        let lite = Dialect::Sqlite;
        let of = |type_name: &str, values: &[Value]| {
            Column::of(lite, type_name, None, values.iter()).unwrap()
        };
        // A time kept as seconds: of the day, opened as its time.
        let times = of("TIME", &[Value::Int(3_600)]);
        assert_eq!(
            opened(&Value::Int(3_600), &times).as_deref(),
            Some("01:00:00")
        );
        assert_eq!(read("01:00:00", &times, &clock()).unwrap().literal, "3600");
        assert_eq!(read("2:00", &times, &clock()).unwrap().literal, "7200");
        assert_eq!(read("7200", &times, &clock()).unwrap().literal, "7200");
        // Julian days, as `julianday()` keeps them.
        let days = of("DATE", &[Value::Float(2_461_337.5)]);
        assert_eq!(days.stored, Stored::Days);
        assert_eq!(
            opened(&Value::Float(2_461_337.5), &days).as_deref(),
            Some("2026-10-24")
        );
        assert_eq!(
            read("25 Oct 2026", &days, &clock()).unwrap().literal,
            "2461338.5"
        );
        assert_eq!(
            read("2461337.5", &days, &clock()).unwrap().literal,
            "2461337.5"
        );
        // A Julian day begins at noon: .5 is midnight, .75 six in the
        // morning.
        let stamps = of("DATETIME", &[Value::Float(2_461_337.75)]);
        assert_eq!(
            opened(&Value::Float(2_461_337.75), &stamps).as_deref(),
            Some("2026-10-24 06:00:00")
        );
        assert_eq!(
            read("2026-10-24 06:00", &stamps, &clock()).unwrap().literal,
            "2461337.75"
        );
    }

    #[test]
    fn a_zones_place_is_the_last_part_of_its_name() {
        assert_eq!(place(&new_york()), "New York");
        assert_eq!(place(&TimeZone::UTC), "UTC");
        assert_eq!(
            written_offset(Offset::from_seconds(-17_762).unwrap()),
            "-04:56:02"
        );
        assert_eq!(
            written_offset(Offset::from_seconds(19_800).unwrap()),
            "+05:30"
        );
    }
}
