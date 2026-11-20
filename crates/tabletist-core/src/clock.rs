//! The time by the person's clock, in their time zone: when an agent's
//! call came, as the Agents tab says it.

use std::time::SystemTime;

use jiff::Timestamp;
use jiff::civil::Date;
use jiff::tz::TimeZone;

use crate::i18n::{Locale, numbers};

/// "10:43:02": a call of the feed, which came today.
pub fn time(at: SystemTime) -> String {
    format(at, &TimeZone::system(), "%H:%M:%S")
}

/// "10:38": when a session connected.
pub fn hour(at: SystemTime) -> String {
    format(at, &TimeZone::system(), "%H:%M")
}

/// "Oct 13 10:43", «13 окт. 10:43»: a call of the log, which keeps a
/// month and more.
pub fn day(locale: Locale, at: SystemTime) -> String {
    day_at(locale, at, &TimeZone::system(), "%H:%M")
}

/// "Oct 13 10:43:40", «13 окт. 10:43:40»: a call of the log on its own
/// line.
pub fn moment(locale: Locale, at: SystemTime) -> String {
    day_at(locale, at, &TimeZone::system(), "%H:%M:%S")
}

/// `at`'s day of the year in `locale`, and its time as `time` writes it.
fn day_at(locale: Locale, at: SystemTime, zone: &TimeZone, time: &str) -> String {
    let Some(date) = date_in(at, zone) else {
        return String::new();
    };
    numbers::day_and_time(
        locale,
        &numbers::short_day(locale, date.month(), date.day()),
        &format(at, zone, time),
    )
}

/// The day `at` fell on, as the history's list heads its entries: today,
/// yesterday, or its date, by the person's clock as it is `now`. The date
/// is said where it is drawn, in the window's language, by [`Day::words`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Day {
    Today,
    Yesterday,
    /// A day before yesterday; `this_year` when it fell in `now`'s year.
    On {
        date: Date,
        this_year: bool,
    },
    /// A time no clock shows.
    Never,
}

impl Day {
    /// The date of an `On` day in `locale`: "Oct 13", and its year where
    /// that is not this year's: "Oct 13, 2025", «13 окт. 2025 г.». Nothing
    /// for the others, which each caller says in words of its own.
    pub fn words(&self, locale: Locale) -> String {
        match self {
            Self::On { date, this_year } => match this_year {
                true => numbers::short_day(locale, date.month(), date.day()),
                false => numbers::dated_day(locale, date.year(), date.month(), date.day()),
            },
            Self::Today | Self::Yesterday | Self::Never => String::new(),
        }
    }
}

pub fn day_of(at: SystemTime, now: SystemTime) -> Day {
    day_in(at, now, &TimeZone::system())
}

fn day_in(at: SystemTime, now: SystemTime, zone: &TimeZone) -> Day {
    let (Some(day), Some(today)) = (date_in(at, zone), date_in(now, zone)) else {
        return Day::Never;
    };
    if day == today {
        return Day::Today;
    }
    if today.yesterday().is_ok_and(|yesterday| yesterday == day) {
        return Day::Yesterday;
    }
    Day::On {
        date: day,
        this_year: day.year() == today.year(),
    }
}

/// The date `at` falls on in `zone`, where a clock shows it.
fn date_in(at: SystemTime, zone: &TimeZone) -> Option<Date> {
    Timestamp::try_from(at)
        .ok()
        .map(|at| at.to_zoned(zone.clone()).date())
}

/// `at` in `zone`, as `pattern` writes it. Nothing for a time no clock
/// shows.
fn format(at: SystemTime, zone: &TimeZone, pattern: &str) -> String {
    Timestamp::try_from(at)
        .map(|at| at.to_zoned(zone.clone()).strftime(pattern).to_string())
        .unwrap_or_default()
}

/// The time a log's entry keeps, milliseconds of Unix time, as a time.
pub fn of_millis(at_ms: i64) -> SystemTime {
    let millis = u64::try_from(at_ms).unwrap_or_default();
    SystemTime::UNIX_EPOCH + std::time::Duration::from_millis(millis)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_time_is_said_in_the_zone_it_is_read_in() {
        // 2026-10-13 08:43:02 UTC.
        let at = of_millis(1_791_880_982_000);
        let berlin = TimeZone::fixed(jiff::tz::offset(2));
        assert_eq!(format(at, &berlin, "%H:%M:%S"), "10:43:02");
        assert_eq!(format(at, &berlin, "%H:%M"), "10:43");
        let english = Locale::English;
        assert_eq!(day_at(english, at, &berlin, "%H:%M"), "Oct 13 10:43");
        assert_eq!(day_at(english, at, &berlin, "%H:%M:%S"), "Oct 13 10:43:02");
        assert_eq!(day_at(english, at, &TimeZone::UTC, "%H:%M"), "Oct 13 08:43");
        let russian = Locale::Russian;
        assert_eq!(day_at(russian, at, &berlin, "%H:%M:%S"), "13 окт. 10:43:02");
    }

    #[test]
    fn a_day_is_today_yesterday_or_its_date() {
        let berlin = TimeZone::fixed(jiff::tz::offset(2));
        // 2026-10-13 23:30 in Berlin.
        let now = of_millis(1_791_927_000_000);
        let hours = |n: u64| std::time::Duration::from_secs(n * 3600);
        assert_eq!(day_in(now - hours(20), now, &berlin), Day::Today);
        assert_eq!(day_in(now - hours(24), now, &berlin), Day::Yesterday);
        let before = day_in(now - hours(48), now, &berlin);
        assert_eq!(before.words(Locale::English), "Oct 11");
        assert_eq!(before.words(Locale::Russian), "11 окт.");
        let last_year = day_in(now - hours(24 * 300), now, &berlin);
        assert_eq!(last_year.words(Locale::English), "Dec 17, 2025");
        assert_eq!(last_year.words(Locale::Russian), "17 дек. 2025 г.");
        assert_eq!(Day::Today.words(Locale::English), "");
    }
}
