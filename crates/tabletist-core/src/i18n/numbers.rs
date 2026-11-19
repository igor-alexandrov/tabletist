//! Numbers and dates as a language writes them: its group and decimal
//! marks, its month and weekday names, and its day's order. A value in the
//! grid that is not grouped, and every date in the grid, stays as the
//! database wrote it (ISO); only the interface's own words follow here.

use std::borrow::Cow;

use super::{Count, Locale, Msgid, fill, msgid, pgettext, pmsgid};

/// The mark between groups of three digits, and the decimal mark.
pub fn marks(locale: Locale) -> (char, char) {
    match locale {
        Locale::English => (',', '.'),
        // CLDR's: a no-break space, so a wrap never splits a number.
        Locale::Russian => ('\u{a0}', ','),
    }
}

/// A count as the interface writes it: `12,480`, «12 480».
pub fn number(locale: Locale, count: impl Count) -> String {
    grouped(&count.as_u64().to_string(), marks(locale).0)
}

/// A run of digits with `mark` between every three, from the end.
fn grouped(digits: &str, mark: char) -> String {
    let mut out = String::with_capacity(digits.len() + digits.len() / 3 * mark.len_utf8());
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            out.push(mark);
        }
        out.push(digit);
    }
    out
}

/// A plain decimal number from the grid with its integer digits in threes,
/// in the language's marks: `1240.50` as `1,240.50`, «1 240,50». Anything
/// else (an exponent, `NaN`, a currency sign) comes back as it is. In
/// English a number with nothing to group does too; in Russian a decimal
/// one still takes its comma, so a grouped column reads alike.
pub fn group_number(locale: Locale, text: &str) -> Cow<'_, str> {
    let (sign, rest) = match text.strip_prefix('-') {
        Some(rest) => ("-", rest),
        None => ("", text),
    };
    let (whole, fraction) = match rest.split_once('.') {
        Some((whole, fraction)) => (whole, Some(fraction)),
        None => (rest, None),
    };
    let digits = |part: &str| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit());
    if !digits(whole) || fraction.is_some_and(|fraction| !digits(fraction)) {
        return Cow::Borrowed(text);
    }
    let (group, point) = marks(locale);
    if whole.len() <= 3 && (fraction.is_none() || point == '.') {
        return Cow::Borrowed(text);
    }
    let mut out = format!("{sign}{}", grouped(whole, group));
    if let Some(fraction) = fraction {
        out.push(point);
        out.push_str(fraction);
    }
    Cow::Owned(out)
}

/// `value` to one decimal place in the language's mark, a trailing zero
/// dropped where `trim`: `1.3`, «1,3», `7`.
fn tenths(locale: Locale, value: f64, trim: bool) -> String {
    // Round half away from zero first: `{:.1}` alone rounds 1.25 to 1.2.
    let text = format!("{:.1}", (value * 10.0).round() / 10.0);
    let text = match trim {
        true => text.trim_end_matches(".0"),
        false => &text,
    };
    text.replace('.', &marks(locale).1.to_string())
}

/// A count shortened to its thousands, millions or billions: `1.2K`,
/// «1,2 тыс.», `3.4M`, `7B`. Under a thousand it is the count itself.
pub fn compact_count(locale: Locale, n: u64) -> String {
    let (value, unit) = match n {
        0..=999 => return n.to_string(),
        // Translators: a count shortened to thousands: "1.2K".
        1_000..=999_999 => (n as f64 / 1e3, pgettext(locale, "thousands", "{n}K")),
        // Translators: a count shortened to millions: "3.4M".
        1_000_000..=999_999_999 => (n as f64 / 1e6, pgettext(locale, "millions", "{n}M")),
        // Translators: a count shortened to billions: "7B".
        _ => (n as f64 / 1e9, pgettext(locale, "billions", "{n}B")),
    };
    fill(&unit, &[("{n}", &tenths(locale, value, true))])
}

/// A size in bytes as a person reads it: `512 B`, `2.0 KB`, «5,0 МБ».
pub fn file_size(locale: Locale, bytes: usize) -> String {
    const KB: f64 = 1024.0;
    let size = bytes as f64;
    let (value, unit) = if size < KB {
        // Translators: a size in bytes, kilobytes or megabytes.
        (bytes.to_string(), pgettext(locale, "file size", "{n} B"))
    } else if size < KB * KB {
        (
            tenths(locale, size / KB, false),
            pgettext(locale, "file size", "{n} KB"),
        )
    } else {
        (
            tenths(locale, size / (KB * KB), false),
            pgettext(locale, "file size", "{n} MB"),
        )
    };
    fill(&unit, &[("{n}", &value)])
}

/// A size in decimal units as the export toasts say it: "7.8 MB",
/// "312 KB", «7,8 МБ»; under a thousand bytes, `None`, for the caller's
/// plural.
pub fn decimal_size(locale: Locale, bytes: u64) -> Option<String> {
    const KB: f64 = 1_000.0;
    const MB: f64 = 1_000_000.0;
    const GB: f64 = 1_000_000_000.0;
    let size = bytes as f64;
    let (value, unit) = if size >= GB {
        (
            tenths(locale, size / GB, false),
            pgettext(locale, "file size", "{n} GB"),
        )
    } else if size >= MB {
        (
            tenths(locale, size / MB, false),
            pgettext(locale, "file size", "{n} MB"),
        )
    } else if size >= KB {
        (
            format!("{:.0}", size / KB),
            pgettext(locale, "file size", "{n} KB"),
        )
    } else {
        return None;
    };
    Some(fill(&unit, &[("{n}", &value)]))
}

/// [`file_size`] as the terminal writes it: `0B`, `48.2K`, «2,1М».
pub fn terse_size(locale: Locale, bytes: usize) -> String {
    const KB: f64 = 1024.0;
    let size = bytes as f64;
    let (value, unit) = if size < KB {
        (
            bytes.to_string(),
            // Translators: a size as the terminal look writes it: "48.2K".
            pgettext(locale, "short file size", "{n}B"),
        )
    } else if size < KB * KB {
        (
            tenths(locale, size / KB, false),
            pgettext(locale, "short file size", "{n}K"),
        )
    } else {
        (
            tenths(locale, size / (KB * KB), false),
            pgettext(locale, "short file size", "{n}M"),
        )
    };
    fill(&unit, &[("{n}", &value)])
}

/// Milliseconds: "450 ms", "12,480 ms", «450 мс».
pub fn millis(locale: Locale, ms: impl Count) -> String {
    fill(
        // Translators: a time in milliseconds.
        &pgettext(locale, "duration", "{n} ms"),
        &[("{n}", &number(locale, ms))],
    )
}

/// Seconds, to a tenth where `tenth`: "18 s", "1.2 s", «1,2 с».
pub fn seconds(locale: Locale, seconds: f64, tenth: bool) -> String {
    let value = match tenth {
        true => tenths(locale, seconds, false),
        false => format!("{seconds:.0}"),
    };
    // Translators: a time in seconds.
    fill(&pgettext(locale, "duration", "{n} s"), &[("{n}", &value)])
}

/// Whole seconds past a minute: "2 min", "2 min 5 s", «2 мин 5 с».
pub fn minutes(locale: Locale, seconds: u64) -> String {
    let (minutes, rest) = (seconds / 60, seconds % 60);
    let minutes = number(locale, minutes);
    match rest {
        0 => fill(
            // Translators: a time in whole minutes.
            &pgettext(locale, "duration", "{n} min"),
            &[("{n}", &minutes)],
        ),
        rest => fill(
            // Translators: a time in minutes and seconds.
            &pgettext(locale, "duration", "{n} min {s} s"),
            &[("{n}", &minutes), ("{s}", &rest.to_string())],
        ),
    }
}

/// The page's rows and how many there are: `1–300 of ~1.2M`, `1–300 of
/// 1,234,567`, or `None` when the page is empty.
pub fn range_label(
    locale: Locale,
    offset: u64,
    shown: usize,
    has_more: bool,
    estimate: Option<u64>,
    exact: Option<u64>,
) -> Option<String> {
    if shown == 0 {
        return None;
    }
    let first = offset + 1;
    let last = offset + shown as u64;
    let (words, total) = if let Some(exact) = exact {
        // Translators: the rows a page shows, of all the table's.
        (msgid("{first}–{last} of {total}"), number(locale, exact))
    } else if !has_more {
        (msgid("{first}–{last} of {total}"), number(locale, last))
    } else if let Some(estimate) = estimate.filter(|estimate| *estimate >= last) {
        (
            // Translators: the rows a page shows, of about this many.
            msgid("{first}–{last} of ~{total}"),
            compact_count(locale, estimate),
        )
    } else {
        // Translators: the rows a page shows, the total not known.
        (msgid("{first}–{last}"), String::new())
    };
    Some(fill(
        &words.say(locale),
        &[
            ("{first}", &number(locale, first)),
            ("{last}", &number(locale, last)),
            ("{total}", &total),
        ],
    ))
}

const SHORT_MONTHS: [Msgid; 12] = [
    pmsgid("short month in a date", "Jan"),
    pmsgid("short month in a date", "Feb"),
    pmsgid("short month in a date", "Mar"),
    pmsgid("short month in a date", "Apr"),
    pmsgid("short month in a date", "May"),
    pmsgid("short month in a date", "Jun"),
    pmsgid("short month in a date", "Jul"),
    pmsgid("short month in a date", "Aug"),
    pmsgid("short month in a date", "Sep"),
    pmsgid("short month in a date", "Oct"),
    pmsgid("short month in a date", "Nov"),
    pmsgid("short month in a date", "Dec"),
];

const MONTHS: [Msgid; 12] = [
    pmsgid("month", "January"),
    pmsgid("month", "February"),
    pmsgid("month", "March"),
    pmsgid("month", "April"),
    pmsgid("month", "May"),
    pmsgid("month", "June"),
    pmsgid("month", "July"),
    pmsgid("month", "August"),
    pmsgid("month", "September"),
    pmsgid("month", "October"),
    pmsgid("month", "November"),
    pmsgid("month", "December"),
];

const MONTHS_IN_A_DATE: [Msgid; 12] = [
    pmsgid("month in a date", "January"),
    pmsgid("month in a date", "February"),
    pmsgid("month in a date", "March"),
    pmsgid("month in a date", "April"),
    pmsgid("month in a date", "May"),
    pmsgid("month in a date", "June"),
    pmsgid("month in a date", "July"),
    pmsgid("month in a date", "August"),
    pmsgid("month in a date", "September"),
    pmsgid("month in a date", "October"),
    pmsgid("month in a date", "November"),
    pmsgid("month in a date", "December"),
];

const SHORT_WEEKDAYS: [Msgid; 7] = [
    pmsgid("short weekday", "Mon"),
    pmsgid("short weekday", "Tue"),
    pmsgid("short weekday", "Wed"),
    pmsgid("short weekday", "Thu"),
    pmsgid("short weekday", "Fri"),
    pmsgid("short weekday", "Sat"),
    pmsgid("short weekday", "Sun"),
];

const WEEKDAY_INITIALS: [Msgid; 7] = [
    pmsgid("weekday initials", "Mo"),
    pmsgid("weekday initials", "Tu"),
    pmsgid("weekday initials", "We"),
    pmsgid("weekday initials", "Th"),
    pmsgid("weekday initials", "Fr"),
    pmsgid("weekday initials", "Sa"),
    pmsgid("weekday initials", "Su"),
];

const WEEKDAYS: [Msgid; 7] = [
    pmsgid("weekday", "Monday"),
    pmsgid("weekday", "Tuesday"),
    pmsgid("weekday", "Wednesday"),
    pmsgid("weekday", "Thursday"),
    pmsgid("weekday", "Friday"),
    pmsgid("weekday", "Saturday"),
    pmsgid("weekday", "Sunday"),
];

/// The entry of `table` for `month`, 1 to 12.
fn of_month(table: &[Msgid; 12], month: i8, locale: Locale) -> Cow<'static, str> {
    let at = usize::try_from(month - 1).unwrap_or_default();
    table[at % 12].say(locale)
}

/// The entry of `table` for a weekday, Monday 0.
fn of_weekday(table: &[Msgid; 7], monday_zero: usize, locale: Locale) -> Cow<'static, str> {
    table[monday_zero % 7].say(locale)
}

/// A short month by its number, 1 to 12, as a date says it: "Oct", «окт.».
pub fn short_month(locale: Locale, month: i8) -> Cow<'static, str> {
    of_month(&SHORT_MONTHS, month, locale)
}

/// A month's name alone, as a calendar's heading: "October", «Октябрь».
pub fn month_name(locale: Locale, month: i8) -> Cow<'static, str> {
    of_month(&MONTHS, month, locale)
}

/// A month's whole name as a date says it: "October", «октября».
pub fn month_of_a_date(locale: Locale, month: i8) -> Cow<'static, str> {
    of_month(&MONTHS_IN_A_DATE, month, locale)
}

/// A short weekday, Monday 0: "Thu", «чт».
pub fn weekday(locale: Locale, monday_zero: usize) -> Cow<'static, str> {
    of_weekday(&SHORT_WEEKDAYS, monday_zero, locale)
}

/// A weekday as a calendar's column heads it, Monday 0: "Th", «Чт».
pub fn weekday_initial(locale: Locale, monday_zero: usize) -> Cow<'static, str> {
    of_weekday(&WEEKDAY_INITIALS, monday_zero, locale)
}

/// A weekday's whole name, Monday 0: "Thursday", «четверг».
pub fn weekday_name(locale: Locale, monday_zero: usize) -> Cow<'static, str> {
    of_weekday(&WEEKDAYS, monday_zero, locale)
}

/// A day of this year: "Oct 13", «13 окт.».
pub fn short_day(locale: Locale, month: i8, day: i8) -> String {
    fill(
        // Translators: a day of this year, as History heads its entries.
        &pgettext(locale, "day of this year", "{month} {day}"),
        &[
            ("{month}", &short_month(locale, month)),
            ("{day}", &day.to_string()),
        ],
    )
}

/// A day of another year: "Oct 13, 2025", «13 окт. 2025 г.».
pub fn dated_day(locale: Locale, year: i16, month: i8, day: i8) -> String {
    fill(
        // Translators: a day of another year, as History heads its entries.
        &pgettext(locale, "day of another year", "{month} {day}, {year}"),
        &[
            ("{month}", &short_month(locale, month)),
            ("{day}", &day.to_string()),
            ("{year}", &year.to_string()),
        ],
    )
}

/// A day and the time on it: "Oct 13 10:43", «13 окт. 10:43».
pub fn day_and_time(locale: Locale, day: &str, time: &str) -> String {
    fill(
        // Translators: a day ("Oct 13", "Today") and the time on it.
        &pgettext(locale, "day and time", "{day} {time}"),
        &[("{day}", day), ("{time}", time)],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::Locale::{English, Russian};

    #[test]
    fn a_number_is_grouped_by_its_language() {
        assert_eq!(number(English, 12_480_u64), "12,480");
        assert_eq!(number(Russian, 12_480_u64), "12\u{a0}480");
        assert_eq!(number(Russian, 999_u64), "999");
        assert_eq!(number(English, 1_234_567_usize), "1,234,567");
        assert_eq!(number(English, 0_u64), "0");
    }

    #[test]
    fn a_grid_number_takes_its_language_s_marks() {
        assert_eq!(group_number(English, "-1240.50"), "-1,240.50");
        assert_eq!(group_number(Russian, "-1240.50"), "-1\u{a0}240,50");
        assert_eq!(group_number(English, "1234567"), "1,234,567");
        // English leaves a short one as the database wrote it; Russian
        // gives a decimal its comma, so a grouped column reads alike.
        assert!(matches!(
            group_number(English, "12.5"),
            Cow::Borrowed("12.5")
        ));
        assert_eq!(group_number(Russian, "12.5"), "12,5");
        assert!(matches!(group_number(Russian, "125"), Cow::Borrowed("125")));
        assert_eq!(group_number(Russian, "1e9"), "1e9");
        assert_eq!(group_number(Russian, "NaN"), "NaN");
        assert_eq!(group_number(English, "$1240"), "$1240");
        assert_eq!(group_number(English, "1240."), "1240.");
    }

    #[test]
    fn a_short_count_says_its_unit() {
        assert_eq!(compact_count(English, 999), "999");
        assert_eq!(compact_count(English, 1_000), "1K");
        assert_eq!(compact_count(English, 1_234), "1.2K");
        assert_eq!(compact_count(English, 1_250), "1.3K");
        assert_eq!(compact_count(English, 1_250_000), "1.3M");
        assert_eq!(compact_count(English, 7_000_000_000), "7B");
        assert_eq!(compact_count(Russian, 1_250), "1,3 тыс.");
        assert_eq!(compact_count(Russian, 3_400_000), "3,4 млн");
        assert_eq!(compact_count(Russian, 7_000_000_000), "7 млрд");
        assert_eq!(compact_count(Russian, 999), "999");
    }

    #[test]
    fn a_size_is_said_in_its_language() {
        assert_eq!(file_size(English, 512), "512 B");
        assert_eq!(file_size(English, 2048), "2.0 KB");
        assert_eq!(file_size(English, 5 * 1024 * 1024), "5.0 MB");
        assert_eq!(file_size(Russian, 512), "512 Б");
        assert_eq!(file_size(Russian, 5 * 1024 * 1024 + 104_858), "5,1 МБ");
        assert_eq!(terse_size(English, 0), "0B");
        assert_eq!(terse_size(English, 49_357), "48.2K");
        assert_eq!(terse_size(English, 5 * 1024 * 1024), "5.0M");
        assert_eq!(terse_size(Russian, 49_357), "48,2К");
    }

    #[test]
    fn a_page_s_range_is_said_in_its_language() {
        assert_eq!(range_label(English, 0, 0, false, None, None), None);
        let english = |offset, shown, more, estimate, exact| {
            range_label(English, offset, shown, more, estimate, exact).unwrap()
        };
        assert_eq!(
            english(0, 300, true, Some(1_234_567), None),
            "1–300 of ~1.2M"
        );
        assert_eq!(
            english(0, 300, true, None, Some(1_234_567)),
            "1–300 of 1,234,567"
        );
        assert_eq!(english(0, 3, false, None, None), "1–3 of 3");
        assert_eq!(english(1200, 300, true, None, None), "1,201–1,500");
        let russian = range_label(Russian, 1200, 300, true, None, Some(1_234_567));
        assert_eq!(
            russian.as_deref(),
            Some("1\u{a0}201–1\u{a0}500 из 1\u{a0}234\u{a0}567")
        );
        let russian = range_label(Russian, 0, 300, true, Some(1_234_567), None);
        assert_eq!(russian.as_deref(), Some("1–300 из ~1,2 млн"));
    }

    #[test]
    fn a_day_is_written_in_its_language() {
        assert_eq!(short_day(English, 10, 13), "Oct 13");
        assert_eq!(short_day(Russian, 10, 13), "13 окт.");
        assert_eq!(dated_day(English, 2025, 10, 13), "Oct 13, 2025");
        assert_eq!(dated_day(Russian, 2025, 10, 13), "13 окт. 2025 г.");
        assert_eq!(short_day(Russian, 5, 1), "1 мая");
        assert_eq!(month_name(English, 1), "January");
        assert_eq!(month_name(Russian, 1), "Январь");
        assert_eq!(month_of_a_date(Russian, 1), "января");
        assert_eq!(weekday(English, 3), "Thu");
        assert_eq!(weekday(Russian, 3), "чт");
        assert_eq!(weekday_initial(Russian, 0), "Пн");
        assert_eq!(weekday_name(Russian, 6), "воскресенье");
        assert_eq!(day_and_time(Russian, "13 окт.", "10:43"), "13 окт. 10:43");
    }

    #[test]
    fn a_time_taken_is_said_in_its_language() {
        assert_eq!(millis(English, 12_480), "12,480 ms");
        assert_eq!(millis(Russian, 450), "450 мс");
        assert_eq!(seconds(English, 1.234, true), "1.2 s");
        assert_eq!(seconds(Russian, 1.234, true), "1,2 с");
        assert_eq!(seconds(English, 18.0, false), "18 s");
        assert_eq!(minutes(English, 120), "2 min");
        assert_eq!(minutes(English, 125), "2 min 5 s");
        assert_eq!(minutes(Russian, 125), "2 мин 5 с");
        assert_eq!(decimal_size(English, 7_800_000).as_deref(), Some("7.8 MB"));
        assert_eq!(decimal_size(English, 312_000).as_deref(), Some("312 KB"));
        assert_eq!(decimal_size(Russian, 7_800_000).as_deref(), Some("7,8 МБ"));
        assert_eq!(decimal_size(Russian, 640), None);
    }

    #[test]
    fn an_exact_total_replaces_the_estimate() {
        let label = |estimate, exact| range_label(English, 0, 300, true, estimate, exact);
        assert_eq!(
            label(Some(90_000), Some(123_456)).as_deref(),
            Some("1–300 of 123,456")
        );
        assert_eq!(label(Some(90_000), None).as_deref(), Some("1–300 of ~90K"));
        assert_eq!(label(None, None).as_deref(), Some("1–300"));
        assert_eq!(
            range_label(English, 300, 50, false, Some(10), None).as_deref(),
            Some("301–350 of 350")
        );
    }

    #[test]
    fn a_number_groups_its_integer_digits_in_threes() {
        assert_eq!(group_number(English, "1240.50"), "1,240.50");
        assert_eq!(group_number(English, "-9100000"), "-9,100,000");
        assert_eq!(group_number(English, "1234567.891011"), "1,234,567.891011");
        // More digits than any integer type holds: it is text all the way.
        assert_eq!(
            group_number(English, "123456789012345678901234567890"),
            "123,456,789,012,345,678,901,234,567,890"
        );
    }

    #[test]
    fn what_is_not_a_plain_number_is_left_as_it_is() {
        for text in [
            "999",
            "-999",
            "0.5",
            "1e21",
            "NaN",
            "inf",
            "-inf",
            "$1240.50",
            "1,240",
            "1240.",
            ".5",
            "",
            "-",
            "1.2.3",
            "1240.5e3",
            "1240.5.6",
            "１２３４",
            " 1240",
        ] {
            assert!(
                matches!(group_number(English, text), Cow::Borrowed(same) if same == text),
                "{text:?}"
            );
        }
        // In Russian only a plain decimal takes its comma.
        for text in ["999", "1e21", "NaN", "$1240.50", "1240.", ".5", "1.2.3"] {
            assert!(
                matches!(group_number(Russian, text), Cow::Borrowed(same) if same == text),
                "{text:?}"
            );
        }
        assert_eq!(group_number(Russian, "0.5"), "0,5");
    }
}
