//! The time by the person's clock, in their time zone: when an agent's
//! call came, as the Agents tab says it.

use std::time::SystemTime;

use jiff::Timestamp;
use jiff::tz::TimeZone;

/// "10:43:02": a call of the feed, which came today.
pub fn time(at: SystemTime) -> String {
    format(at, &TimeZone::system(), "%H:%M:%S")
}

/// "10:38": when a session connected.
pub fn hour(at: SystemTime) -> String {
    format(at, &TimeZone::system(), "%H:%M")
}

/// "Oct 13 10:43": a call of the log, which keeps a month and more.
pub fn day(at: SystemTime) -> String {
    format(at, &TimeZone::system(), "%b %-d %H:%M")
}

/// "Oct 13 10:43:40": a call of the log on its own line.
pub fn moment(at: SystemTime) -> String {
    format(at, &TimeZone::system(), "%b %-d %H:%M:%S")
}

/// The day `at` fell on, as the history's list heads its entries: today,
/// yesterday, or its date ("Oct 13", and its year where that is not this
/// year's), by the person's clock as it is `now`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Day {
    Today,
    Yesterday,
    On(String),
}

pub fn day_of(at: SystemTime, now: SystemTime) -> Day {
    day_in(at, now, &TimeZone::system())
}

fn day_in(at: SystemTime, now: SystemTime, zone: &TimeZone) -> Day {
    let date = |time: SystemTime| {
        Timestamp::try_from(time)
            .ok()
            .map(|time| time.to_zoned(zone.clone()).date())
    };
    let (Some(day), Some(today)) = (date(at), date(now)) else {
        return Day::On(String::new());
    };
    if day == today {
        return Day::Today;
    }
    if today.yesterday().is_ok_and(|yesterday| yesterday == day) {
        return Day::Yesterday;
    }
    let pattern = match day.year() == today.year() {
        true => "%b %-d",
        false => "%b %-d, %Y",
    };
    Day::On(format(at, zone, pattern))
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
        assert_eq!(format(at, &berlin, "%b %-d %H:%M"), "Oct 13 10:43");
        assert_eq!(format(at, &berlin, "%b %-d %H:%M:%S"), "Oct 13 10:43:02");
        assert_eq!(format(at, &TimeZone::UTC, "%b %-d %H:%M"), "Oct 13 08:43");
    }

    #[test]
    fn a_day_is_today_yesterday_or_its_date() {
        let berlin = TimeZone::fixed(jiff::tz::offset(2));
        // 2026-10-13 23:30 in Berlin.
        let now = of_millis(1_791_927_000_000);
        let hours = |n: u64| std::time::Duration::from_secs(n * 3600);
        assert_eq!(day_in(now - hours(20), now, &berlin), Day::Today);
        assert_eq!(day_in(now - hours(24), now, &berlin), Day::Yesterday);
        assert_eq!(
            day_in(now - hours(48), now, &berlin),
            Day::On("Oct 11".into())
        );
        assert_eq!(
            day_in(now - hours(24 * 300), now, &berlin),
            Day::On("Dec 17, 2025".into())
        );
    }
}
